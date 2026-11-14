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
use videolore::media::{self, MediaItem};
use videolore::preview::{NullSink, Preview, PreviewConfig};
use videolore::project::{self, Project};
use videolore::time::Micros;

struct Env {
    info: FfmpegInfo,
    dir: tempfile::TempDir,
}

fn setup() -> Option<Env> {
    let found = ffmpeg::locate_both(None, None)
        .map_err(|e| e.to_string())
        .and_then(|(f, p)| ffmpeg::check(&SystemRunner, &f, &p).map_err(|e| e.to_string()));
    match found {
        Ok(info) => Some(Env {
            info,
            dir: tempfile::tempdir().unwrap(),
        }),
        Err(e) => {
            if std::env::var("VIDEOLORE_REQUIRE_FFMPEG").as_deref() == Ok("1") {
                panic!("ffmpeg is required but unusable: {e}");
            }
            eprintln!("skipping: ffmpeg unavailable ({e})");
            None
        }
    }
}

impl Env {
    /// Generates a test clip. `audio` adds a 440 Hz sine.
    fn clip(&self, name: &str, w: u32, h: u32, fps: u32, secs: u32, audio: bool) -> PathBuf {
        let out = self.dir.path().join(name);
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
            .arg(fps.to_string())
            .arg(&out);
        assert!(c.status().unwrap().success(), "fixture {name}");
        out
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
        (r, rx.try_iter().collect())
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
    let h = export::start(env.job(&p, &out), Arc::new(SystemRunner));
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last = None;
    while Instant::now() < deadline {
        match h.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ExportEvent::Progress(_)) => {
                h.cancel();
            }
            Ok(e @ (ExportEvent::Cancelled | ExportEvent::Done(_) | ExportEvent::Failed(_))) => {
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
        runner: Arc::new(SystemRunner),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    let now = Instant::now();
    pv.seek(&p, Micros(2_500_000), now).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let frame = loop {
        if let Some(f) = pv.poll(now) {
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
    let deadline = Instant::now() + Duration::from_secs(30);
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
