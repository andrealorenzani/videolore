// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Preview proxies: lower-resolution copies of sources that are big for this machine.
//!
//! * The rule ([`needs_proxy`]) is deterministic: more than 1920x1080 pixels, or more than
//!   1280x720 on a machine with fewer than 8 logical cores or less than 8 GiB of memory.
//! * Proxies live in the user's cache folder as `<key>.mkv`, where the key hashes the
//!   source path, size and modification time; they are reused across sessions and
//!   recreated when missing. Clearing deletes only files with that name pattern.
//! * Generation runs in the background, one at a time, with the lowest priority; a
//!   preview or export that needs the ffmpeg slot preempts it (it restarts later).
//! * Proxies are only used by the preview, never by the export.

use crate::ffmpeg::{FfVersion, ProgressEvent, Runner, argv, file_arg, parse_progress_line};
use crate::jobs::Limited;
use crate::machine::{GIB, MachineInfo};
use crate::media::{MediaFormat, MediaId};
use crate::time::{Fps, Micros};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

/// Bump when the proxy encoding changes, so old cache entries are not reused.
pub const PARAMS_VERSION: u32 = 1;
/// Proxies fit inside this size.
pub const MAX_WIDTH: u32 = 960;
pub const MAX_HEIGHT: u32 = 540;

/// The proxy rule.
pub fn needs_proxy(width: u32, height: u32, machine: MachineInfo) -> bool {
    let px = u64::from(width) * u64::from(height);
    let weak = machine.cores < 8 || machine.ram_bytes < 8 * GIB;
    px > 1920 * 1080 || (px > 1280 * 720 && weak)
}

/// Whether a probed format should get a proxy (audio-only files never do).
pub fn format_needs_proxy(f: &MediaFormat, machine: MachineInfo) -> bool {
    f.has_video() && needs_proxy(f.width, f.height, machine)
}

/// Largest even size with the same aspect ratio that fits in `max_w` x `max_h`.
pub fn fit_even(width: u32, height: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let (w, h) = (u64::from(width.max(1)), u64::from(height.max(1)));
    let (mw, mh) = (u64::from(max_w), u64::from(max_h));
    let (fw, fh) = if w * mh > h * mw {
        (mw, h * mw / w)
    } else {
        (w * mh / h, mh)
    };
    let (fw, fh) = if w <= mw && h <= mh { (w, h) } else { (fw, fh) };
    (((fw as u32) & !1).max(2), ((fh as u32) & !1).max(2))
}

/// Cache key of a source: first 32 hex digits of SHA-256(path, size, mtime, version).
pub fn cache_key(path: &Path, size: u64, mtime_ns: u128) -> String {
    let mut h = Sha256::new();
    h.update(path.as_os_str().as_encoded_bytes());
    h.update([0]);
    h.update(size.to_le_bytes());
    h.update(mtime_ns.to_le_bytes());
    h.update(PARAMS_VERSION.to_le_bytes());
    h.finalize()
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Cache key of an existing file.
pub fn key_of(path: &Path) -> io::Result<String> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(cache_key(path, meta.len(), mtime))
}

/// True for file names the cache owns: `<32 hex>.mkv` or `<32 hex>.mkv.partial`.
pub fn is_cache_file(name: &str) -> bool {
    let stem = name
        .strip_suffix(".mkv.partial")
        .or_else(|| name.strip_suffix(".mkv"));
    stem.is_some_and(|s| {
        s.len() == 32
            && s.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    })
}

/// The proxy cache folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyCache {
    pub dir: PathBuf,
}

impl ProxyCache {
    pub fn new(dir: PathBuf) -> ProxyCache {
        ProxyCache { dir }
    }

    /// `<user cache>/proxies`.
    pub fn default_dir() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "videolore").map(|d| d.cache_dir().join("proxies"))
    }

    pub fn path_for(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.mkv"))
    }

    pub fn partial_for(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.mkv.partial"))
    }

    /// The existing proxy of `source`, if any.
    pub fn lookup(&self, source: &Path) -> Option<PathBuf> {
        let p = self.path_for(&key_of(source).ok()?);
        p.is_file().then_some(p)
    }

    fn entries(&self) -> Vec<(String, PathBuf, u64)> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        rd.filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let meta = e.metadata().ok()?;
                (meta.is_file() && is_cache_file(&name)).then(|| (name, e.path(), meta.len()))
            })
            .collect()
    }

    /// Bytes used by the cache.
    pub fn size(&self) -> u64 {
        self.entries().iter().map(|e| e.2).sum()
    }

    /// Deletes cache files except those whose key is in `busy` (being generated).
    /// Returns the bytes freed.
    pub fn clear(&self, busy: &[String]) -> u64 {
        let mut freed = 0;
        for (name, path, len) in self.entries() {
            if busy.iter().any(|k| name.starts_with(k.as_str())) {
                continue;
            }
            if std::fs::remove_file(&path).is_ok() {
                freed += len;
            }
        }
        freed
    }
}

/// ffmpeg arguments producing a proxy: H.264 fitting 960x540, same frame rate and
/// timestamps, short GOP for quick seeking, AAC stereo.
pub fn proxy_args(
    source: &Path,
    out: &Path,
    format: &MediaFormat,
    version: FfVersion,
) -> Vec<OsString> {
    let (w, h) = fit_even(format.width, format.height, MAX_WIDTH, MAX_HEIGHT);
    let fps = format.fps.unwrap_or(Fps { num: 30, den: 1 });
    let gop = ((fps.as_f64() / 2.0).round() as u32).max(1);
    let mut a = argv([
        "-hide_banner",
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-protocol_whitelist",
        "file,pipe",
        "-i",
    ]);
    a.push(file_arg(source));
    a.extend(argv(["-map", "0:v:0", "-map", "0:a:0?"]));
    a.extend(argv([
        "-vf".to_string(),
        format!("scale={w}:{h},setsar=1"),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "23".into(),
        "-g".into(),
        gop.to_string(),
        "-pix_fmt".into(),
        "yuv420p".into(),
    ]));
    if version.at_least(5, 1) {
        a.extend(argv(["-fps_mode", "passthrough"]));
    } else {
        a.extend(argv(["-vsync", "passthrough"]));
    }
    a.extend(argv([
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        "-ac",
        "2",
        "-f",
        "matroska",
        "-progress",
        "pipe:1",
        "-nostats",
    ]));
    a.push(file_arg(out));
    a
}

/// One proxy to make.
#[derive(Debug, Clone)]
pub struct ProxyTask {
    pub media: MediaId,
    pub source: PathBuf,
    pub key: String,
    pub format: MediaFormat,
    pub duration: Micros,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProxyEvent {
    Progress(MediaId, u8),
    Ready(MediaId, PathBuf),
    Failed(MediaId, String),
}

/// Everything a proxy worker needs.
#[derive(Clone)]
pub struct ProxyWorker {
    pub runner: Limited,
    pub ffmpeg: PathBuf,
    pub version: FfVersion,
    pub cache: ProxyCache,
}

impl ProxyWorker {
    /// Generates the proxy, restarting when preempted. Returns `None` when cancelled.
    pub fn generate(&self, task: &ProxyTask, events: &Sender<ProxyEvent>) -> Option<ProxyEvent> {
        if let Err(e) = std::fs::create_dir_all(&self.cache.dir) {
            return Some(ProxyEvent::Failed(task.media, e.to_string()));
        }
        let out = self.cache.path_for(&task.key);
        if out.is_file() {
            return Some(ProxyEvent::Ready(task.media, out));
        }
        let partial = self.cache.partial_for(&task.key);
        loop {
            let preempt = Arc::new(AtomicBool::new(false));
            let r = self
                .runner
                .preemptible(Arc::clone(&preempt))
                .cancellable(Arc::clone(&task.cancel));
            let result = self.attempt(&r, task, &partial, &preempt, events);
            match result {
                Attempt::Done => {
                    return Some(match std::fs::rename(&partial, &out) {
                        Ok(()) => ProxyEvent::Ready(task.media, out),
                        Err(e) => ProxyEvent::Failed(task.media, e.to_string()),
                    });
                }
                Attempt::Preempted => {
                    let _ = std::fs::remove_file(&partial);
                    continue;
                }
                Attempt::Cancelled => {
                    let _ = std::fs::remove_file(&partial);
                    return None;
                }
                Attempt::Failed(e) => {
                    let _ = std::fs::remove_file(&partial);
                    return Some(ProxyEvent::Failed(task.media, e));
                }
            }
        }
    }

    fn attempt(
        &self,
        runner: &dyn Runner,
        task: &ProxyTask,
        partial: &Path,
        preempt: &AtomicBool,
        events: &Sender<ProxyEvent>,
    ) -> Attempt {
        let args = proxy_args(&task.source, partial, &task.format, self.version);
        let mut child = match runner.spawn(&self.ffmpeg, &args) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => return Attempt::Cancelled,
            Err(e) => return Attempt::Failed(e.to_string()),
        };
        let total = task.duration.0.max(1) as f64;
        if let Some(out) = child.take_stdout() {
            for line in BufReader::new(out).lines() {
                let stop = if task.cancel.load(Ordering::SeqCst) {
                    Some(Attempt::Cancelled)
                } else if preempt.load(Ordering::SeqCst) {
                    Some(Attempt::Preempted)
                } else {
                    None
                };
                if let Some(a) = stop {
                    child.kill();
                    return a;
                }
                let Ok(line) = line else { break };
                if let Some(ProgressEvent::Time(t)) = parse_progress_line(&line) {
                    let pct = ((t.0 as f64 / total) * 100.0).clamp(0.0, 99.0) as u8;
                    let _ = events.send(ProxyEvent::Progress(task.media, pct));
                }
            }
        }
        match child.wait() {
            _ if task.cancel.load(Ordering::SeqCst) => Attempt::Cancelled,
            _ if preempt.load(Ordering::SeqCst) => Attempt::Preempted,
            Ok(o) if o.success => Attempt::Done,
            Ok(o) => Attempt::Failed(o.stderr.trim().to_string()),
            Err(e) => Attempt::Failed(e.to_string()),
        }
    }
}

enum Attempt {
    Done,
    Preempted,
    Cancelled,
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg::fake::{FakeRunner, fail, ok};
    use crate::jobs::{Priority, Scheduler};
    use std::sync::mpsc::channel;

    fn machine(cores: usize, gib: u64) -> MachineInfo {
        MachineInfo {
            cores,
            ram_bytes: gib * GIB,
        }
    }

    // covers: REQ-038
    #[test]
    fn rule_is_deterministic_on_resolution_cores_and_memory() {
        let strong = machine(8, 16);
        let few_cores = machine(4, 16);
        let little_ram = machine(16, 4);
        let table = [
            ((3840, 2160), strong, true),
            ((2560, 1440), strong, true),
            ((1920, 1080), strong, false),
            ((1920, 1080), few_cores, true),
            ((1920, 1080), little_ram, true),
            ((1280, 720), few_cores, false),
            ((1080, 1920), strong, false),
            ((2160, 3840), little_ram, true),
        ];
        for ((w, h), m, want) in table {
            assert_eq!(needs_proxy(w, h, m), want, "{w}x{h} {m:?}");
            assert_eq!(needs_proxy(w, h, m), needs_proxy(w, h, m));
        }
        let audio = MediaFormat::default();
        assert!(!format_needs_proxy(&audio, few_cores));
        assert_eq!(fit_even(3840, 2160, 960, 540), (960, 540));
        assert_eq!(fit_even(1080, 1920, 960, 540), (302, 540));
        assert_eq!(fit_even(641, 361, 960, 540), (640, 360));
        assert_eq!(fit_even(4000, 1000, 960, 540), (960, 240));
    }

    // covers: REQ-038
    #[test]
    fn cache_keys_reuse_and_clearing_only_our_files() {
        let k1 = cache_key(Path::new("/a.mp4"), 10, 5);
        assert_eq!(k1.len(), 32);
        assert_eq!(k1, cache_key(Path::new("/a.mp4"), 10, 5));
        assert_ne!(k1, cache_key(Path::new("/a.mp4"), 11, 5));
        assert_ne!(k1, cache_key(Path::new("/a.mp4"), 10, 6));
        assert_ne!(k1, cache_key(Path::new("/b.mp4"), 10, 5));
        assert!(is_cache_file(&format!("{k1}.mkv")));
        assert!(is_cache_file(&format!("{k1}.mkv.partial")));
        assert!(!is_cache_file("notes.mkv"));
        assert!(!is_cache_file(&format!("{}.mkv", k1.to_uppercase())));
        assert!(!is_cache_file(&format!("{k1}.mp4")));

        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.mp4");
        std::fs::write(&src, b"video").unwrap();
        let cache = ProxyCache::new(dir.path().join("proxies"));
        assert_eq!(cache.size(), 0);
        assert!(cache.lookup(&src).is_none());
        std::fs::create_dir_all(&cache.dir).unwrap();
        let key = key_of(&src).unwrap();
        std::fs::write(cache.path_for(&key), b"12345").unwrap();
        assert_eq!(cache.lookup(&src), Some(cache.path_for(&key)));
        let other = cache_key(Path::new("/other"), 1, 1);
        std::fs::write(cache.partial_for(&other), b"123").unwrap();
        std::fs::write(cache.dir.join("keep-me.txt"), b"user file").unwrap();
        assert_eq!(cache.size(), 8);
        // the proxy being generated is kept
        assert_eq!(cache.clear(std::slice::from_ref(&other)), 5);
        assert_eq!(cache.size(), 3);
        assert_eq!(cache.clear(&[]), 3);
        assert!(cache.dir.join("keep-me.txt").exists());
        assert!(ProxyCache::default_dir().is_some() || cfg!(not(unix)));
        assert!(key_of(&dir.path().join("nope")).is_err());
    }

    fn task(dir: &Path) -> ProxyTask {
        let src = dir.join("big.mp4");
        std::fs::write(&src, b"video").unwrap();
        ProxyTask {
            media: MediaId(1),
            key: key_of(&src).unwrap(),
            source: src,
            format: MediaFormat {
                vcodec: Some("h264".into()),
                width: 3840,
                height: 2160,
                fps: Fps::new(30, 1),
                ..Default::default()
            },
            duration: Micros(2_000_000),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    fn worker(dir: &Path, fake: FakeRunner, version: FfVersion) -> ProxyWorker {
        let sched = Scheduler::new(3);
        ProxyWorker {
            runner: sched.runner(Priority::Background, Arc::new(fake)),
            ffmpeg: "ffmpeg".into(),
            version,
            cache: ProxyCache::new(dir.join("cache")),
        }
    }

    // covers: REQ-038
    #[test]
    fn generation_reports_progress_and_reuses_the_result() {
        let dir = tempfile::tempdir().unwrap();
        let t = task(dir.path());
        let fake = FakeRunner::default().on(|_, a| {
            assert!(a.iter().any(|x| x == "scale=960:540,setsar=1"));
            assert!(a.windows(2).any(|w| w == ["-g", "15"]));
            assert!(a.windows(2).any(|w| w == ["-fps_mode", "passthrough"]));
            let out = a.last()?.strip_prefix("file:")?.to_string();
            assert!(out.ends_with(".mkv.partial"));
            std::fs::write(out, b"proxy").ok()?;
            ok("out_time_us=1000000\nprogress=end\n")
        });
        let w = worker(dir.path(), fake, FfVersion::Release(6, 1, 0));
        let (tx, rx) = channel();
        let ev = w.generate(&t, &tx).unwrap();
        let path = w.cache.path_for(&t.key);
        assert_eq!(ev, ProxyEvent::Ready(MediaId(1), path.clone()));
        assert_eq!(std::fs::read(&path).unwrap(), b"proxy");
        assert_eq!(
            rx.try_iter().collect::<Vec<_>>(),
            vec![ProxyEvent::Progress(MediaId(1), 50)]
        );
        assert_eq!(w.cache.lookup(&t.source), Some(path.clone()));
        // second time: reused without running ffmpeg
        let none = worker(dir.path(), FakeRunner::default(), FfVersion::Dev);
        assert_eq!(
            none.generate(&t, &tx).unwrap(),
            ProxyEvent::Ready(MediaId(1), path)
        );
    }

    #[test]
    fn generation_failure_cancel_and_old_ffmpeg() {
        let dir = tempfile::tempdir().unwrap();
        let t = task(dir.path());
        let (tx, _rx) = channel();
        let failing = FakeRunner::default().on(|_, a| {
            assert!(a.windows(2).any(|w| w == ["-vsync", "passthrough"]));
            fail("no encoder")
        });
        let w = worker(dir.path(), failing, FfVersion::Release(4, 4, 0));
        assert_eq!(
            w.generate(&t, &tx),
            Some(ProxyEvent::Failed(MediaId(1), "no encoder".into()))
        );
        assert!(!w.cache.partial_for(&t.key).exists());
        t.cancel.store(true, Ordering::SeqCst);
        let w = worker(
            dir.path(),
            FakeRunner::default().on(|_, _| ok("")),
            FfVersion::Dev,
        );
        // cancelled while waiting for a slot (all slots taken) or while running
        let hold = w
            .runner
            .scheduler()
            .acquire(Priority::Preview, 3, None, None);
        assert_eq!(w.generate(&t, &tx), None);
        drop(hold);
        assert_eq!(w.generate(&t, &tx), None);
        let missing = worker(dir.path(), FakeRunner::default(), FfVersion::Dev);
        let t2 = task(dir.path());
        assert!(matches!(
            missing.generate(&t2, &tx),
            Some(ProxyEvent::Failed(..))
        ));
        let mut bad = worker(dir.path(), FakeRunner::default(), FfVersion::Dev);
        let file = dir.path().join("file");
        std::fs::write(&file, b"x").unwrap();
        bad.cache = ProxyCache::new(file.join("sub"));
        assert!(matches!(
            bad.generate(&t2, &tx),
            Some(ProxyEvent::Failed(..))
        ));
    }

    /// First process prints a progress line every 5 ms until killed (a long encode);
    /// later ones write the output and finish at once.
    struct Ticking {
        attempts: Arc<std::sync::atomic::AtomicUsize>,
    }

    struct TickProcess {
        killed: Arc<AtomicBool>,
        endless: bool,
    }

    struct TickReader {
        killed: Arc<AtomicBool>,
        sent: bool,
        endless: bool,
    }

    impl io::Read for TickReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.endless {
                if self.killed.load(Ordering::SeqCst) {
                    return Ok(0);
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
                let line = b"out_time_us=100000\n";
                buf[..line.len()].copy_from_slice(line);
                return Ok(line.len());
            }
            if self.sent {
                return Ok(0);
            }
            self.sent = true;
            let line = b"progress=end\n";
            buf[..line.len()].copy_from_slice(line);
            Ok(line.len())
        }
    }

    impl crate::ffmpeg::Process for TickProcess {
        fn take_stdout(&mut self) -> Option<Box<dyn io::Read + Send>> {
            Some(Box::new(TickReader {
                killed: Arc::clone(&self.killed),
                sent: false,
                endless: self.endless,
            }))
        }
        fn kill(&mut self) {
            self.killed.store(true, Ordering::SeqCst);
        }
        fn wait(&mut self) -> io::Result<crate::ffmpeg::CmdOutput> {
            Ok(crate::ffmpeg::CmdOutput {
                success: !self.killed.load(Ordering::SeqCst),
                ..Default::default()
            })
        }
    }

    impl Runner for Ticking {
        fn run(&self, _: &Path, _: &[OsString]) -> io::Result<crate::ffmpeg::CmdOutput> {
            Err(io::Error::other("unused"))
        }
        fn spawn_in(
            &self,
            _: &Path,
            args: &[OsString],
            _: Option<&Path>,
        ) -> io::Result<Box<dyn crate::ffmpeg::Process>> {
            let i = self.attempts.fetch_add(1, Ordering::SeqCst);
            let out = args.last().unwrap().to_string_lossy().into_owned();
            std::fs::write(out.strip_prefix("file:").unwrap(), b"part")?;
            Ok(Box::new(TickProcess {
                killed: Arc::new(AtomicBool::new(false)),
                endless: i == 0,
            }))
        }
    }

    // covers: REQ-038, REQ-033
    #[test]
    fn preempted_generation_restarts_from_zero() {
        let dir = tempfile::tempdir().unwrap();
        let t = task(dir.path());
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sched = Scheduler::new(3);
        let w = ProxyWorker {
            runner: sched.runner(
                Priority::Background,
                Arc::new(Ticking {
                    attempts: Arc::clone(&attempts),
                }),
            ),
            ffmpeg: "ffmpeg".into(),
            version: FfVersion::Dev,
            cache: ProxyCache::new(dir.path().join("cache")),
        };
        let (tx, rx) = channel();
        let w2 = w.clone();
        let t2 = t.clone();
        let h = std::thread::spawn(move || w2.generate(&t2, &tx));
        // wait until the proxy is running, then a preview needs all 3 slots
        assert!(rx.recv_timeout(std::time::Duration::from_secs(5)).is_ok());
        let preview = sched.acquire(Priority::Preview, 3, None, None).unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        drop(preview);
        let ev = h.join().unwrap().unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 2, "restarted");
        assert_eq!(ev, ProxyEvent::Ready(MediaId(1), w.cache.path_for(&t.key)));
    }
}
