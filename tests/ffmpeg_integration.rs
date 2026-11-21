// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! End-to-end tests with the real ffmpeg. Fixtures are generated at test time (lavfi
//! testsrc2 + sine) in a temporary folder: no binary files are kept in git.
//! Without ffmpeg the tests are skipped, unless VIDEOLORE_REQUIRE_FFMPEG=1 (then they fail).

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use videolore::export::{self, ExportEvent, ExportJob};
use videolore::ffmpeg::{self, FfmpegInfo, SystemRunner};
use videolore::jobs::{Limited, MAX_PROCESSES, Priority, Scheduler};
use videolore::media::{self, MediaItem};
use videolore::preview::{NullSink, Preview, PreviewConfig};
use videolore::project::{self, Project};
use videolore::render::{self, Plan};
use videolore::time::Micros;

#[path = "ffmpeg_integration/hw.rs"]
mod hw;
#[path = "ffmpeg_integration/v12.rs"]
mod v12;
#[path = "ffmpeg_integration/v126.rs"]
mod v126;
#[path = "ffmpeg_integration/v13.rs"]
mod v13;
#[path = "ffmpeg_integration/v14.rs"]
mod v14;
#[path = "ffmpeg_integration/v15.rs"]
mod v15;
#[path = "ffmpeg_integration/v16.rs"]
mod v16;
#[path = "ffmpeg_integration/v20.rs"]
mod v20;
#[path = "ffmpeg_integration/v204.rs"]
mod v204;

struct Env {
    info: FfmpegInfo,
    dir: tempfile::TempDir,
}

fn required() -> bool {
    std::env::var("VIDEOLORE_REQUIRE_FFMPEG").as_deref() == Ok("1")
}

/// A runner limited like the app's (3 processes) at `priority`.
fn limited(priority: Priority) -> Limited {
    Scheduler::new(MAX_PROCESSES).runner(priority, Arc::new(SystemRunner))
}

/// The `version` of the `[linux-x86_64]` section of the lock file of the bundled ffmpeg.
fn pinned_version(lock: &str) -> Option<String> {
    let mut in_section = false;
    for line in lock.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = line == "[linux-x86_64]";
        } else if in_section && let Some(v) = line.strip_prefix("version") {
            return Some(v.trim_start().strip_prefix('=')?.trim().to_string());
        }
    }
    None
}

/// In CI the tests must run on the pinned build (QG-07, QG-08): its version text (first line
/// of `ffmpeg -version`) names the pinned version.
fn assert_pinned(info: &FfmpegInfo) {
    static CHECKED: std::sync::Once = std::sync::Once::new();
    CHECKED.call_once(|| {
        let want = pinned_version(include_str!("../packaging/ffmpeg.lock"))
            .expect("packaging/ffmpeg.lock has a linux-x86_64 version");
        let out = Command::new(&info.ffmpeg)
            .args(["-hide_banner", "-version"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let first = text.lines().next().unwrap_or_default();
        assert!(
            first.contains(&want),
            "the tests must run on the pinned ffmpeg {want}, found: {first}"
        );
    });
}

// covers: QG-08
#[test]
fn the_pinned_version_is_read_from_the_lock_file() {
    let lock =
        "[windows-x86_64]\nversion = 1\n[linux-x86_64]\nprovider = x\nversion = 9.0.2-57-gabc\n";
    assert_eq!(pinned_version(lock).as_deref(), Some("9.0.2-57-gabc"));
    assert_eq!(pinned_version("[linux-x86_64]\n"), None);
    assert!(pinned_version(include_str!("../packaging/ffmpeg.lock")).is_some());
}

fn setup() -> Option<Env> {
    let found = ffmpeg::locate_both(None, None)
        .map_err(|e| e.to_string())
        .and_then(|l| ffmpeg::check_located(&SystemRunner, &l).map_err(|e| e.to_string()));
    match found {
        Ok(info) => {
            if required() {
                assert_pinned(&info);
            }
            Some(Env {
                info,
                dir: tempfile::tempdir().unwrap(),
            })
        }
        Err(e) => {
            if required() {
                panic!("ffmpeg is required but unusable: {e}");
            }
            eprintln!("skipping: ffmpeg unavailable ({e})");
            None
        }
    }
}

/// Where generated fixtures are kept between runs (inside the build folder).
fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixtures")
}

impl Env {
    /// A test clip (`audio` adds a 440 Hz sine). Clips are generated once per set of
    /// parameters and ffmpeg version and reused by every test and later runs (tests only
    /// read them). `name` keeps the test readable; it is not part of the key.
    fn clip(&self, name: &str, w: u32, h: u32, fps: u32, secs: u32, audio: bool) -> PathBuf {
        let key = format!(
            "{w}x{h}@{fps}x{secs}s audio={audio} {:?} {:?} {}",
            self.info.version,
            self.info.ffmpeg,
            self.info.encoders.contains("libx264")
        );
        let digest: String = Sha256::digest(key.as_bytes())[..12]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let dir = fixture_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
        let cached = dir.join(format!("{digest}.{ext}"));
        if cached.is_file() {
            return cached;
        }
        // written under a unique name, then renamed: safe with tests running in parallel
        let tmp = dir.join(format!(
            "{digest}.{}.{:?}.partial.{ext}",
            std::process::id(),
            std::thread::current().id()
        ));
        self.generate(&tmp, w, h, fps, secs, audio);
        std::fs::rename(&tmp, &cached).unwrap();
        cached
    }

    fn generate(&self, out: &Path, w: u32, h: u32, fps: u32, secs: u32, audio: bool) {
        let name = out.display();
        let venc = if self.info.encoders.contains("libx264") {
            "libx264"
        } else {
            "mpeg4"
        };
        let mut c = Command::new(&self.info.ffmpeg);
        c.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
        ])
        .arg(format!("testsrc2=size={w}x{h}:rate={fps}:duration={secs}"));
        if audio {
            c.args(["-f", "lavfi", "-i"]).arg(format!(
                "sine=frequency=440:sample_rate=48000:duration={secs}"
            ));
            c.args(["-c:a", "aac"]);
        }
        c.args(["-c:v", venc, "-pix_fmt", "yuv420p", "-g"])
            .arg(fps.to_string());
        if venc == "libx264" {
            c.args(["-preset", "ultrafast"]);
        }
        c.arg(out);
        assert!(c.status().unwrap().success(), "fixture {name}");
    }

    /// A fixture made by ffmpeg with `args` (the output file is appended), cached like
    /// [`Env::clip`] under a key made of `name`, the arguments and the ffmpeg build.
    fn fixture(&self, name: &str, args: &[&str]) -> PathBuf {
        let key = format!(
            "{name} {args:?} {:?} {:?}",
            self.info.version, self.info.ffmpeg
        );
        let digest: String = Sha256::digest(key.as_bytes())[..12]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let dir = fixture_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");
        let cached = dir.join(format!("{digest}.{ext}"));
        if !cached.is_file() {
            let tmp = dir.join(format!(
                "{digest}.{}.{:?}.partial.{ext}",
                std::process::id(),
                std::thread::current().id()
            ));
            let mut all = args.to_vec();
            let out = tmp.to_string_lossy().into_owned();
            all.push(&out);
            self.ffmpeg(&all);
            std::fs::rename(&tmp, &cached).unwrap();
        }
        cached
    }

    /// Runs ffmpeg with `args` (fixtures), asserting success.
    fn ffmpeg(&self, args: &[&str]) {
        let ok = Command::new(&self.info.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "ffmpeg {args:?}");
    }

    /// `codec_tag_string` of the first video stream.
    fn video_tag(&self, file: &Path) -> String {
        let o = Command::new(&self.info.ffprobe)
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_tag_string",
                "-of",
                "csv=p=0",
            ])
            .arg(file)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    /// Luma (Y plane) mean of every frame of a `w`x`h` video.
    fn luma_means(&self, file: &Path, w: usize, h: usize) -> Vec<f64> {
        let o = Command::new(&self.info.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-i"])
            .arg(file)
            .args(["-f", "rawvideo", "-pix_fmt", "yuv420p", "pipe:1"])
            .output()
            .unwrap();
        assert!(o.status.success());
        let frame = w * h * 3 / 2;
        o.stdout
            .chunks_exact(frame)
            .map(|f| mean(&f[..w * h]))
            .collect()
    }

    fn audio_only(&self, name: &str) -> PathBuf {
        let out = self.dir.path().join(name);
        let ok = Command::new(&self.info.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg("sine=frequency=440:sample_rate=48000:duration=2")
            .args(["-c:a", "aac"])
            .arg(&out)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        out
    }

    fn probe(&self, p: &mut Project, path: &Path) -> MediaItem {
        let id = p.new_media_id();
        let m = media::probe(&SystemRunner, &self.info.ffprobe, path, id).unwrap();
        p.add_media(m.clone());
        m
    }

    fn job(&self, p: &Project, out: &Path) -> ExportJob {
        ExportJob {
            project: p.clone(),
            output: out.to_path_buf(),
            ffmpeg: self.info.ffmpeg.clone(),
            ffprobe: self.info.ffprobe.clone(),
            version: self.info.version,
            encoders: self.info.encoders.clone(),
            can_burn_subtitles: self.info.has_subtitles(),
            // a "Light" limit: proves real ffmpeg accepts the thread options, and keeps
            // the tests running in parallel from fighting over the cores
            threads: Some(2),
            work_root: self.dir.path().join("work"),
            hw: None,
            stall_timeout: export::STALL_TIMEOUT,
            check_timeout: export::CHECK_TIMEOUT,
            mode: Default::default(),
            format: Default::default(),
            audio_only: false,
        }
    }

    fn export(
        &self,
        p: &Project,
        out: &Path,
    ) -> (Result<PathBuf, export::ExportError>, Vec<ExportEvent>) {
        let (tx, rx) = channel();
        let r = export::run(
            &self.job(p, out),
            &SystemRunner,
            &AtomicBool::new(false),
            &tx,
        );
        (r.map(|(path, _)| path), rx.try_iter().collect())
    }

    /// One video frame at `t` as RGB24.
    fn frame_rgb(&self, file: &Path, t: f64) -> Vec<u8> {
        let o = Command::new(&self.info.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-ss"])
            .arg(format!("{t}"))
            .arg("-i")
            .arg(file)
            .args([
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(o.status.success());
        o.stdout
    }

    /// Peak absolute sample (mono s16) between `from` and `from + len`.
    fn audio_peak(&self, file: &Path, from: f64, len: f64) -> i32 {
        let o = Command::new(&self.info.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-ss"])
            .arg(format!("{from}"))
            .arg("-t")
            .arg(format!("{len}"))
            .arg("-i")
            .arg(file)
            .args(["-ac", "1", "-f", "s16le", "pipe:1"])
            .output()
            .unwrap();
        let (samples, _) = o.stdout.as_chunks::<2>();
        samples
            .iter()
            .map(|&s| i32::from(i16::from_le_bytes(s)).abs())
            .max()
            .unwrap_or(0)
    }
}

fn sha256(p: &Path) -> Vec<u8> {
    Sha256::digest(std::fs::read(p).unwrap()).to_vec()
}

fn mean(px: &[u8]) -> f64 {
    px.iter().map(|&v| v as f64).sum::<f64>() / px.len().max(1) as f64
}

fn near(a: Micros, b: Micros, tol: Micros) -> bool {
    (a.0 - b.0).abs() <= tol.0
}

// covers: REQ-002
#[test]
fn import_reads_duration_and_rejects_garbage() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let v = env.clip("v.mp4", 640, 360, 25, 2, true);
    let m = env.probe(&mut p, &v);
    assert!(near(m.duration, Micros(2_000_000), Micros(50_000)));
    assert!(m.format.vcodec.is_some());
    assert_eq!((m.format.width, m.format.height), (640, 360));
    let a = env.audio_only("a.m4a");
    let am = env.probe(&mut p, &a);
    assert!(!am.has_video());
    assert!(p.timeline.append(&am).is_err());
    let txt = env.dir.path().join("notes.txt");
    std::fs::write(&txt, "not a video").unwrap();
    let e = media::probe(&SystemRunner, &env.info.ffprobe, &txt, p.new_media_id()).unwrap_err();
    assert!(e.to_string().contains("notes.txt"));
}

// covers: REQ-023
#[test]
fn identical_sources_export_by_stream_copy() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let a = env.clip("a.mp4", 1280, 720, 30, 2, true);
    let b = env.clip("b.mp4", 1280, 720, 30, 3, true);
    let ma = env.probe(&mut p, &a);
    let mb = env.probe(&mut p, &b);
    p.timeline.append(&ma).unwrap();
    p.timeline.append(&mb).unwrap();
    let out = env.dir.path().join("copy.mp4");
    let (r, ev) = env.export(&p, &out);
    let r = r.unwrap();
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: true });
    let mut q = Project::default();
    let mo = env.probe(&mut q, &r);
    assert_eq!(mo.format.vcodec.as_deref(), Some("h264"));
    assert_eq!((mo.format.width, mo.format.height), (1280, 720));
    assert!(
        near(mo.duration, Micros(5_000_000), Micros(100_000)),
        "{:?}",
        mo.duration
    );
}

// covers: REQ-022
#[test]
fn mixed_formats_export_at_lowest_with_letterbox() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let hd = env.clip("hd.mp4", 1280, 720, 30, 2, true);
    let sd = env.clip("sd.mp4", 480, 360, 25, 2, false);
    let m1 = env.probe(&mut p, &hd);
    let m2 = env.probe(&mut p, &sd);
    p.timeline.append(&m1).unwrap();
    p.timeline.append(&m2).unwrap();
    let out = env.dir.path().join("mixed.mp4");
    let (r, ev) = env.export(&p, &out);
    let r = r.unwrap();
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let mut q = Project::default();
    let mo = env.probe(&mut q, &r);
    assert_eq!((mo.format.width, mo.format.height), (480, 360));
    assert_eq!(mo.format.fps.unwrap().as_f64(), 25.0);
    assert!(mo.format.has_audio());
    // 16:9 into 4:3: black bars above and below, picture in the middle (not stretched)
    let rgb = env.frame_rgb(&r, 0.5);
    assert_eq!(rgb.len(), 480 * 360 * 3);
    let row = 480 * 3;
    let top = &rgb[..row * 30];
    let middle = &rgb[row * 150..row * 210];
    assert!(mean(top) < 20.0, "top bar mean {}", mean(top));
    assert!(mean(middle) > 40.0, "picture mean {}", mean(middle));
}

// covers: REQ-011
#[test]
fn fade_keeps_total_length_and_goes_through_black() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let a = env.clip("fa.mp4", 640, 360, 25, 2, true);
    let b = env.clip("fb.mp4", 640, 360, 25, 2, true);
    let ma = env.probe(&mut p, &a);
    let mb = env.probe(&mut p, &b);
    p.timeline.append(&ma).unwrap();
    p.timeline.append(&mb).unwrap();
    p.timeline.toggle_fade(0).unwrap();
    let out = env.dir.path().join("fade.mp4");
    let (r, _) = env.export(&p, &out);
    let r = r.unwrap();
    let mut q = Project::default();
    let mo = env.probe(&mut q, &r);
    let total = ma.duration + mb.duration;
    assert!(
        near(mo.duration, total, Micros(80_000)),
        "{:?} vs {:?}",
        mo.duration,
        total
    );
    // video: darkest at the boundary, normal in the middle of a clip
    let at_boundary = mean(&env.frame_rgb(&r, 1.98));
    let in_clip = mean(&env.frame_rgb(&r, 1.0));
    assert!(at_boundary < 25.0, "boundary mean {at_boundary}");
    assert!(in_clip > 60.0);
    // audio: silent around the boundary, loud in the middle
    let quiet = env.audio_peak(&r, 1.97, 0.06);
    let loud = env.audio_peak(&r, 0.9, 0.2);
    assert!(quiet * 5 < loud, "quiet {quiet} loud {loud}");
}

// covers: REQ-001
#[test]
fn sources_are_never_modified() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let a = env.clip("s1.mp4", 640, 360, 25, 2, true);
    let b = env.clip("s2.mp4", 1280, 720, 30, 2, false);
    let before: Vec<_> = [&a, &b].iter().map(|f| sha256(f)).collect();
    let ma = env.probe(&mut p, &a);
    let mb = env.probe(&mut p, &b);
    p.timeline.append(&ma).unwrap();
    p.timeline.append(&mb).unwrap();
    p.timeline.split(Micros(1_000_000), Micros(40_000)).unwrap();
    p.timeline.toggle_fade(0).unwrap();
    project::save(&p, &env.dir.path().join("p.vlproj")).unwrap();
    env.export(&p, &env.dir.path().join("o1.mp4")).0.unwrap();
    let mut copy = Project::default();
    let mc = env.probe(&mut copy, &a);
    copy.timeline.append(&mc).unwrap();
    env.export(&copy, &env.dir.path().join("o2.mp4")).0.unwrap();
    // exporting onto a source is refused
    assert!(env.export(&p, &a).0.is_err());
    let after: Vec<_> = [&a, &b].iter().map(|f| sha256(f)).collect();
    assert_eq!(before, after);
    assert!(a.exists() && b.exists());
}

// covers: REQ-021
#[test]
fn cancel_mid_export_leaves_nothing_behind() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let a = env.clip("long.mp4", 1280, 720, 30, 4, true);
    let m = env.probe(&mut p, &a);
    for _ in 0..15 {
        p.timeline.append(&m).unwrap();
    }
    p.timeline.toggle_fade(0).unwrap(); // force a (slow) re-encode
    let out = env.dir.path().join("cancelled.mp4");
    let h = export::start(env.job(&p, &out), Arc::new(limited(Priority::Export)));
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last = None;
    while Instant::now() < deadline {
        match h.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ExportEvent::Progress(_)) => {
                h.cancel();
            }
            Ok(
                e @ (ExportEvent::Cancelled | ExportEvent::Done { .. } | ExportEvent::Failed(_)),
            ) => {
                last = Some(e);
                break;
            }
            _ => {}
        }
    }
    assert_eq!(last, Some(ExportEvent::Cancelled));
    let _ = h.join();
    assert!(!out.exists());
    let partial =
        export::partial_path(&env.dir.path().canonicalize().unwrap().join("cancelled.mp4"));
    assert!(!partial.exists());
}

// covers: REQ-013
#[test]
fn preview_delivers_rgba_frames_of_expected_size() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let a = env.clip("pv.mp4", 1280, 720, 30, 2, true);
    let m = env.probe(&mut p, &a);
    p.timeline.append(&m).unwrap();
    p.timeline.append(&m).unwrap();
    p.timeline.toggle_fade(0).unwrap();
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    pv.seek(&p, Micros(2_500_000), Instant::now()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    let frame = loop {
        if let Some(f) = pv.poll(Instant::now()) {
            break f;
        }
        assert!(Instant::now() < deadline, "no preview frame");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!((frame.width, frame.height), (640, 360));
    assert_eq!(frame.rgba.len(), 640 * 360 * 4);
    assert_eq!(frame.pts, Micros(2_500_000));
    // playing produces a sequence of frames
    pv.play(&p, Micros::ZERO, Instant::now()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut got = 0;
    while got < 3 && Instant::now() < deadline {
        if pv.poll(Instant::now()).is_some() {
            got += 1;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(got, 3);
    pv.pause(Instant::now());
}

/// Two HEVC clips in MP4: stream copy and re-encode both carry the Apple `hvc1` tag.
// covers: REQ-036
#[test]
fn hevc_in_mp4_is_tagged_hvc1() {
    let Some(env) = setup() else { return };
    if !env.info.encoders.contains("libx265") {
        assert!(!required(), "the CI ffmpeg must have libx265");
        eprintln!("skipping: no libx265");
        return;
    }
    let mut p = Project::default();
    for name in ["h1.mp4", "h2.mp4"] {
        let out = env.fixture(
            name,
            &[
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x240:rate=25:duration=1",
                "-c:v",
                "libx265",
                "-preset",
                "ultrafast",
                "-x265-params",
                "log-level=error",
                "-pix_fmt",
                "yuv420p",
                "-tag:v",
                "hev1",
            ],
        );
        let m = env.probe(&mut p, &out);
        p.timeline.append(&m).unwrap();
    }
    let (r, ev) = env.export(&p, &env.dir.path().join("copy.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: true });
    assert_eq!(env.video_tag(&r.unwrap()), "hvc1");
    p.timeline.toggle_fade(0).unwrap();
    let (r, ev) = env.export(&p, &env.dir.path().join("reencoded.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    assert_eq!(env.video_tag(&r.unwrap()), "hvc1");
}

/// 200 short pieces cut in order from one source: one input in the graph, every output
/// frame is the expected source frame.
// covers: REQ-037
#[test]
fn two_hundred_segments_of_one_source_are_frame_accurate() {
    let Some(env) = setup() else { return };
    if !env.info.encoders.contains("libx264") {
        assert!(!required());
        return;
    }
    // each source frame has its own flat grey level: Y = (37 * frame) mod 200 + 20
    let src = env.fixture(
        "grey.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:r=25:d=25,format=yuv420p,geq=lum='mod(N*37,200)+20':cb=128:cr=128",
            "-c:v",
            "libx264",
            "-qp",
            "0",
            "-preset",
            "ultrafast",
            "-g",
            "250",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let mut p = Project::default();
    let m = env.probe(&mut p, &src);
    let frame = Micros(40_000);
    for i in 0..200i64 {
        let id = p.timeline.append(&m).unwrap();
        let start = Micros(frame.0 * 3 * i);
        p.timeline
            .trim(id, start, start + Micros(frame.0 * 2), m.duration, frame)
            .unwrap();
    }
    assert_eq!(p.timeline.clips.len(), 200);
    let plan = render::plan_export(&p, &env.info.encoders, |_, _| false).unwrap();
    match &plan {
        Plan::Reencode { graph, .. } => assert_eq!(graph.inputs.len(), 1),
        other => panic!("expected a re-encode, got {other:?}"),
    }
    let (r, _) = env.export(&p, &env.dir.path().join("cuts.mp4"));
    let means = env.luma_means(&r.unwrap(), 64, 64);
    assert_eq!(means.len(), 400);
    for (k, got) in means.iter().enumerate() {
        let src_frame = 3 * (k / 2) + k % 2;
        let want = ((37 * src_frame) % 200 + 20) as f64;
        assert!(
            (got - want).abs() < 9.0,
            "output frame {k}: luma {got:.1}, expected {want} (source frame {src_frame})"
        );
    }
}

/// Music mixed under a silent video: nothing before the music clip, sound during it,
/// faded out at the video end; half volume is about a quarter of the level.
// covers: REQ-020, REQ-043, REQ-019
#[test]
fn music_is_mixed_cut_and_faded_at_the_video_end() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let v = env.clip("mute.mp4", 320, 240, 25, 3, false);
    let song = env.fixture(
        "song.m4a",
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000:duration=5",
            "-c:a",
            "aac",
        ],
    );
    let mv = env.probe(&mut p, &v);
    let ms = env.probe(&mut p, &song);
    p.timeline.append(&mv).unwrap();
    p.music
        .place(&ms, Micros(1_000_000), Micros(40_000))
        .unwrap();
    let (r, ev) = env.export(&p, &env.dir.path().join("music.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let r = r.unwrap();
    let mut q = Project::default();
    let out = env.probe(&mut q, &r);
    assert!(out.format.has_audio());
    // music past the video end is cut there
    assert!(
        near(out.duration, Micros(3_000_000), Micros(120_000)),
        "{:?}",
        out.duration
    );
    let before = env.audio_peak(&r, 0.2, 0.6);
    let during = env.audio_peak(&r, 1.5, 0.5);
    let at_end = env.audio_peak(&r, 2.88, 0.08);
    assert!(before < 50, "no music before the clip: {before}");
    // lavfi sine is 1/8 of full scale (about 4096)
    assert!(during > 1500, "music plays: {during}");
    assert!(
        at_end * 2 < during,
        "faded out at the end: {at_end} vs {during}"
    );
    // music at 50%: gain (0.5)^2 = 0.25
    p.mix.music_volume = 50;
    let (half, _) = env.export(&p, &env.dir.path().join("half.mp4"));
    let ratio = env.audio_peak(&half.unwrap(), 1.5, 0.5) as f64 / during as f64;
    assert!((0.18..0.33).contains(&ratio), "50% volume ratio {ratio}");
}

/// Subtitles are burned into the exported picture.
// covers: REQ-017
#[test]
fn subtitles_are_burned_in() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        assert!(!required(), "the CI ffmpeg must have libass");
        return;
    }
    let mut p = Project::default();
    let v = env.clip("talk.mp4", 640, 360, 25, 2, true);
    let m = env.probe(&mut p, &v);
    p.timeline.append(&m).unwrap();
    // reference: same re-encode without subtitles (a fade forces the re-encode)
    p.timeline.append(&m).unwrap();
    p.timeline.toggle_fade(0).unwrap();
    let (plain, _) = env.export(&p, &env.dir.path().join("plain.mp4"));
    p.subtitles
        .add_at(&p.timeline, Micros::ZERO, "HELLO SUBTITLES", Micros(40_000))
        .unwrap();
    let (burned, _) = env.export(&p, &env.dir.path().join("burned.mp4"));
    let a = env.frame_rgb(&plain.unwrap(), 0.5);
    let b = env.frame_rgb(&burned.unwrap(), 0.5);
    assert_eq!(a.len(), b.len());
    let diff: u64 = a
        .iter()
        .zip(&b)
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    assert!(
        diff > (a.len() as u64) / 50,
        "the text changes the picture ({diff})"
    );
}

/// A real proxy from a 1440p source: small, same length, written to the cache.
// covers: REQ-038
#[test]
fn proxy_is_generated_from_a_big_source() {
    let Some(env) = setup() else { return };
    if !env.info.encoders.contains("libx264") {
        assert!(!required());
        return;
    }
    let mut p = Project::default();
    let big = env.clip("big.mp4", 2560, 1440, 25, 1, true);
    let m = env.probe(&mut p, &big);
    let cache = videolore::proxy::ProxyCache::new(env.dir.path().join("proxies"));
    let worker = videolore::proxy::ProxyWorker {
        runner: limited(Priority::Background),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        cache: cache.clone(),
        threads: Some(2),
        work_root: Some(env.dir.path().join("work")),
    };
    let task = videolore::proxy::ProxyTask {
        media: m.id,
        source: m.path.clone(),
        key: videolore::proxy::key_of(&m.path).unwrap(),
        format: m.format.clone(),
        duration: m.duration,
        cancel: Arc::new(AtomicBool::new(false)),
        hwaccel: Vec::new(),
    };
    let (tx, _rx) = channel();
    let ev = worker.generate(&task, &tx).unwrap();
    let videolore::proxy::ProxyEvent::Ready(_, path) = ev else {
        panic!("proxy failed: {ev:?}");
    };
    assert_eq!(cache.lookup(&m.path), Some(path.clone()));
    let mut q = Project::default();
    let pm = env.probe(&mut q, &path);
    assert_eq!((pm.format.width, pm.format.height), (960, 540));
    assert!(near(pm.duration, m.duration, Micros(100_000)));
    assert!(cache.size() > 0);
}

/// Environment variable telling `memory_peak_of_one_preview` which file to preview.
const MEMORY_PROBE: &str = "VIDEOLORE_MEMORY_PROBE";
/// Prefix of the line where `memory_peak_of_one_preview` reports its peak.
const PEAK_LINE: &str = "VIDEOLORE_PEAK=";

/// Memory does not grow with the length of the source: preview of a long file costs the
/// same as a short one. Each preview runs in its own process (this test binary, running only
/// `memory_peak_of_one_preview`): the resident memory of this process also counts every
/// other test running in parallel (the speech model alone adds ~170 MB), which made the
/// comparison depend on test scheduling.
// covers: REQ-039
#[test]
fn memory_does_not_grow_with_source_length() {
    let Some(env) = setup() else { return };
    // small pictures: the test is about length, not size (25 fps: 20 frames in 0.8 s)
    let short = env.clip("short.mp4", 160, 90, 25, 2, true);
    let long = env.clip("long.mp4", 160, 90, 25, 120, true);
    // both measured at the same time, each in its own process
    let (small, big) = std::thread::scope(|s| {
        let a = s.spawn(|| peak_in_own_process(&short));
        let b = s.spawn(|| peak_in_own_process(&long));
        (a.join().unwrap(), b.join().unwrap())
    });
    assert!(
        big <= small + 200 * 1024 * 1024,
        "peak memory {big} with the long source vs {small}"
    );
}

/// Runs `memory_peak_of_one_preview` on `file` in a fresh process and returns its peak.
fn peak_in_own_process(file: &Path) -> u64 {
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "memory_peak_of_one_preview",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(MEMORY_PROBE, file)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "memory probe failed: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
        .lines()
        .find_map(|l| l.split_once(PEAK_LINE)?.1.trim().parse().ok())
        .unwrap_or_else(|| panic!("memory probe printed no peak: {stdout}"))
}

/// Helper of `memory_does_not_grow_with_source_length` (does nothing on its own): plays the
/// file named by `VIDEOLORE_MEMORY_PROBE` for 20 frames and prints the peak resident memory.
#[test]
#[ignore = "run in its own process by memory_does_not_grow_with_source_length"]
fn memory_peak_of_one_preview() {
    let Some(file) = std::env::var_os(MEMORY_PROBE) else {
        return;
    };
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let m = env.probe(&mut p, Path::new(&file));
    p.timeline.append(&m).unwrap();
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    pv.play(&p, Micros::ZERO, Instant::now()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut got = 0;
    let mut peak = 0;
    while got < 20 && Instant::now() < deadline {
        if pv.poll(Instant::now()).is_some() {
            got += 1;
        }
        peak = peak.max(videolore::machine::own_rss());
        std::thread::sleep(Duration::from_millis(10));
    }
    pv.pause(Instant::now());
    assert!(got > 0, "no preview frame");
    println!("{PEAK_LINE}{peak}");
}

/// Leaves many freed small heap blocks full of 0xFF, as in a long-running busy process, so
/// code that reads memory it never wrote sees non-zero garbage instead of fresh zeroes.
#[cfg(feature = "stt")]
fn dirty_heap() {
    for size in [8usize, 16, 24, 32, 48, 64] {
        let blocks: Vec<Vec<u8>> = (0..4096).map(|_| vec![0xFF; size]).collect();
        drop(std::hint::black_box(blocks));
    }
}

/// Real speech recognition with the pinned model (CI provides it with
/// VIDEOLORE_STT_MODEL; espeak-ng makes the speech).
// covers: REQ-014, REQ-015
#[cfg(feature = "stt")]
#[test]
fn real_speech_is_transcribed() {
    let Some(env) = setup() else { return };
    let Ok(model) = std::env::var("VIDEOLORE_STT_MODEL") else {
        eprintln!("skipping: VIDEOLORE_STT_MODEL not set");
        return;
    };
    let wav = env.dir.path().join("speech.wav");
    let spoke = Command::new("espeak-ng")
        .args(["-s", "140", "-w"])
        .arg(&wav)
        .arg("hello world, this is videolore")
        .status();
    if !spoke.is_ok_and(|s| s.success()) {
        assert!(!required(), "espeak-ng is needed for the speech test");
        return;
    }
    let job = videolore::stt::SttJob {
        ranges: vec![videolore::stt::RangeJob {
            media: videolore::media::MediaId(1),
            path: wav,
            range: None,
            mode: videolore::stt::Mode::Recognise,
            track_language: None,
        }],
        language: None,
        ffmpeg: env.info.ffmpeg.clone(),
        model: model.into(),
        threads: 2,
        work_root: env.dir.path().join("work"),
        model_check: None,
    };
    let mut t = videolore::stt::whisper::WhisperTranscriber::load(&job.model, 2).unwrap();
    let (tx, _rx) = channel();
    let runner = limited(Priority::Background);
    let r = videolore::stt::run(
        &job,
        &runner as &dyn videolore::ffmpeg::Runner,
        Some(&mut t),
        None,
        &Arc::new(AtomicBool::new(false)),
        &tx,
    )
    .unwrap();
    let (lang, lines) = (r.language.clone(), r.lines());
    assert_eq!(lang.as_deref(), Some("en"));
    let text: String = lines.iter().map(|l| l.text.to_lowercase() + " ").collect();
    assert!(text.contains("hello"), "{text}");
    let words: Vec<_> = lines.iter().flat_map(|l| &l.words).collect();
    assert!(words.windows(2).all(|w| w[0].start <= w[1].start));
}

/// Regression: whisper-rs 0.16's `set_abort_callback_safe` read its "cancel flag" from heap
/// memory next to a box, so with garbage there every encode came back aborted (whisper.cpp
/// code -6). It passed on a quiet machine and failed on the CI runner; with `dirty_heap()`
/// right before the call it fails every time. The cancel flag must still stop the work.
#[cfg(feature = "stt")]
#[test]
fn abort_check_of_whisper_ignores_heap_garbage() {
    use videolore::stt::{SttError, Transcriber};
    let Ok(model) = std::env::var("VIDEOLORE_STT_MODEL") else {
        eprintln!("skipping: VIDEOLORE_STT_MODEL not set");
        return;
    };
    // 3 s of a quiet tone: no speech is needed, only a full encode + decode pass
    let pcm: Vec<f32> = (0..16_000 * 3)
        .map(|i| (i as f32 * 0.05).sin() * 0.1)
        .collect();
    // Three transcriptions, each right after dirtying the heap of its own thread: two in a
    // row on one transcriber (its decoding state is reused) and one, at the same time, on a
    // second transcriber in another thread (each thread has its own heap arena).
    let run = |calls: usize| {
        let mut t =
            videolore::stt::whisper::WhisperTranscriber::load(Path::new(&model), 2).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        for _ in 0..calls {
            dirty_heap();
            let r = t.transcribe(&pcm, Micros::ZERO, Some("en"), &cancel);
            assert!(r.is_ok(), "{r:?}");
        }
        (t, cancel)
    };
    let (mut t, cancel) = std::thread::scope(|s| {
        let other = s.spawn(|| run(1));
        let main = run(2);
        other.join().unwrap();
        main
    });
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    let r = t.transcribe(&pcm, Micros::ZERO, Some("en"), &cancel);
    assert_eq!(r.unwrap_err(), SttError::Cancelled);
}

/// Words of `text`, lower-case, without punctuation.
#[cfg(feature = "stt")]
fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word error rate: word-level edit distance divided by the reference length.
#[cfg(feature = "stt")]
fn wer(reference: &[String], heard: &[String]) -> f64 {
    let mut prev: Vec<usize> = (0..=heard.len()).collect();
    for (i, r) in reference.iter().enumerate() {
        let mut cur = vec![i + 1; heard.len() + 1];
        for (j, h) in heard.iter().enumerate() {
            let sub = prev[j] + usize::from(r != h);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[heard.len()] as f64 / reference.len().max(1) as f64
}

/// Higher Subtitle quality levels are at least as accurate as Fast on a reference clip.
/// Needs the three speech files in `VIDEOLORE_STT_MODELS_DIR` (CI caches them) and
/// espeak-ng.
// covers: REQ-052
#[cfg(feature = "stt")]
#[test]
fn higher_levels_are_at_least_as_accurate() {
    let Some(env) = setup() else { return };
    let Ok(models) = std::env::var("VIDEOLORE_STT_MODELS_DIR") else {
        eprintln!("skipping: VIDEOLORE_STT_MODELS_DIR not set");
        return;
    };
    let sentence = "the quick brown fox jumps over the lazy dog near the river bank";
    let wav = env.dir.path().join("reference.wav");
    let spoke = Command::new("espeak-ng")
        .args(["-s", "140", "-w"])
        .arg(&wav)
        .arg(sentence)
        .status();
    if !spoke.is_ok_and(|s| s.success()) {
        assert!(!required(), "espeak-ng is needed for the accuracy test");
        return;
    }
    let reference = words(sentence);
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get().min(8));
    let mut rates = Vec::new();
    for spec in videolore::stt::builtin_models() {
        let model = Path::new(&models).join(&spec.file);
        assert!(model.is_file(), "missing {}", model.display());
        let job = videolore::stt::SttJob {
            ranges: vec![videolore::stt::RangeJob {
                media: videolore::media::MediaId(1),
                path: wav.clone(),
                range: None,
                mode: videolore::stt::Mode::Recognise,
                track_language: None,
            }],
            language: Some("en".into()),
            ffmpeg: env.info.ffmpeg.clone(),
            model: model.clone(),
            threads,
            work_root: env.dir.path().join("work"),
            model_check: None,
        };
        let mut t = videolore::stt::whisper::WhisperTranscriber::load(&model, threads).unwrap();
        let (tx, _rx) = channel();
        let runner = limited(Priority::Background);
        let lines = videolore::stt::run(
            &job,
            &runner as &dyn videolore::ffmpeg::Runner,
            Some(&mut t),
            None,
            &Arc::new(AtomicBool::new(false)),
            &tx,
        )
        .unwrap()
        .lines();
        let heard: String = lines.iter().map(|l| l.text.clone() + " ").collect();
        let rate = wer(&reference, &words(&heard));
        eprintln!("{:?}: WER {rate:.2} \"{heard}\"", spec.level);
        rates.push(rate);
    }
    assert!(rates[1] <= rates[0], "Balanced {rates:?}");
    assert!(rates[2] <= rates[0], "Most accurate {rates:?}");
}

#[cfg(feature = "stt")]
#[test]
fn word_error_rate_is_computed_on_words() {
    let r = words("The quick, brown fox!");
    assert_eq!(r, vec!["the", "quick", "brown", "fox"]);
    assert_eq!(wer(&r, &r), 0.0);
    assert_eq!(wer(&r, &words("the quick fox")), 0.25);
    assert_eq!(wer(&r, &words("a quick brown fox jumps")), 0.5);
}
#[path = "ffmpeg_integration/v21.rs"]
mod v21;
