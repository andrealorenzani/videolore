// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Hardware export orchestration with a simulated ffmpeg: every way the hardware can fail
//! ends with a correct file made by the normal method, silently; cancel stops every phase.

use super::tests::job;
use super::*;
use crate::ffmpeg::fake::{FakeRunner, ok, process};
use crate::hwenc::Vendor;
use std::sync::Mutex;

/// What the simulated hardware does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hw {
    /// Writes a good file.
    Works,
    /// Ends with an error message.
    ExitsWithError,
    /// Ends badly with no message (a crash).
    Crashes,
    /// Never makes progress.
    Hangs,
    /// Ends well, but the file has green frames.
    GreenFrames,
    /// Ends well, but frames are missing.
    DropsFrames,
    /// Constant quality: the first run (quantiser 23) grows far too big, the next is fine.
    BigAtFirst,
    /// Constant quality: every run makes a file far too big.
    AlwaysBig,
}

/// Which phase sets the cancel flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CancelAt {
    Never,
    HardwareRun,
    Check,
    Redo,
}

/// A simulated ffmpeg/ffprobe for the whole hardware export.
struct Sim {
    hw: Hw,
    cancel_at: CancelAt,
    cancel: Arc<AtomicBool>,
    /// The redo fails too.
    software_fails: bool,
    /// Each command, tagged with what it was.
    log: Mutex<Vec<&'static str>>,
    /// The quantiser of each hardware run ("" for bitrate control).
    qps: Mutex<Vec<String>>,
}

const FRAMES: u64 = 50;

fn sim(hw: Hw) -> Sim {
    Sim {
        hw,
        cancel_at: CancelAt::Never,
        cancel: Arc::new(AtomicBool::new(false)),
        software_fails: false,
        log: Mutex::new(Vec::new()),
        qps: Mutex::new(Vec::new()),
    }
}

fn out_file(a: &[String]) -> Option<String> {
    a.last()?.strip_prefix("file:").map(str::to_string)
}

fn done(stdout: &str, success: bool, stderr: &str) -> io::Result<Box<dyn Process>> {
    Ok(process(
        CmdOutput {
            success,
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.into(),
        },
        false,
    ))
}

const PROGRESS: &str =
    "out_time_us=1000000\nprogress=continue\nout_time_us=2000000\nprogress=end\n";

impl Sim {
    fn note(&self, what: &'static str) {
        self.log.lock().unwrap().push(what);
    }

    fn cancel_if(&self, at: CancelAt) {
        if self.cancel_at == at {
            self.cancel.store(true, Ordering::SeqCst);
        }
    }

    fn metadata(&self) -> CmdOutput {
        let packets = if self.hw == Hw::DropsFrames {
            FRAMES - 7
        } else {
            FRAMES
        };
        let json = serde_json::json!({
            "streams": [
                {"codec_type": "video", "codec_name": "h264", "profile": "High",
                 "pix_fmt": "yuv420p", "width": 1280, "height": 720,
                 "r_frame_rate": "30/1", "nb_read_packets": packets.to_string(),
                 "duration": "2.000000"},
                {"codec_type": "audio", "codec_name": "aac", "sample_rate": "48000",
                 "channels": 2}
            ]
        });
        CmdOutput {
            success: true,
            stdout: serde_json::to_vec(&json).unwrap(),
            stderr: String::new(),
        }
    }
}

use crate::ffmpeg::CmdOutput;
use std::io;

impl Runner for Sim {
    fn run(&self, program: &Path, args: &[std::ffi::OsString]) -> io::Result<CmdOutput> {
        self.run_within(program, args, Duration::from_secs(1))
    }

    fn run_within(
        &self,
        _: &Path,
        args: &[std::ffi::OsString],
        _: Duration,
    ) -> io::Result<CmdOutput> {
        let a: Vec<String> = args.iter().map(|x| x.to_string_lossy().into()).collect();
        if a.contains(&"-count_packets".to_string()) {
            self.note("metadata");
            return Ok(self.metadata());
        }
        // keyframes of the sources: none at the cut points, so the export re-encodes
        Ok(CmdOutput {
            success: true,
            stdout: b"0.0\n".to_vec(),
            stderr: String::new(),
        })
    }

    fn spawn_in(
        &self,
        _: &Path,
        args: &[std::ffi::OsString],
        cwd: Option<&Path>,
    ) -> io::Result<Box<dyn Process>> {
        let a: Vec<String> = args.iter().map(|x| x.to_string_lossy().into()).collect();
        let line = a.join(" ");
        if line.contains("_nvenc")
            || line.contains("_qsv")
            || line.contains("_amf")
            || line.contains("_vaapi")
            || line.contains("_videotoolbox")
        {
            self.note("hardware");
            self.cancel_if(CancelAt::HardwareRun);
            let out = out_file(&a).unwrap();
            assert!(out.ends_with(".hw.videolore-partial"), "{out}");
            std::fs::write(&out, b"hardware video").unwrap();
            self.qps.lock().unwrap().push(
                a.iter()
                    .position(|x| x == "-qp")
                    .map(|i| a[i + 1].clone())
                    .unwrap_or_default(),
            );
            let big = "total_size=99000000\nout_time_us=3000000\nprogress=continue\n\
                       out_time_us=3500000\nprogress=continue\nout_time_us=4000000\nprogress=end\n";
            if self.hw == Hw::AlwaysBig {
                std::fs::File::options()
                    .write(true)
                    .open(&out)
                    .unwrap()
                    .set_len(4_000_000)
                    .unwrap();
            }
            return match self.hw {
                Hw::Works | Hw::GreenFrames | Hw::DropsFrames => done(PROGRESS, true, ""),
                Hw::BigAtFirst if line.contains("-qp 23") => done(big, true, ""),
                Hw::BigAtFirst => done(PROGRESS, true, ""),
                Hw::AlwaysBig => done(big, true, ""),
                Hw::ExitsWithError => done("out_time_us=1000\n", false, "Error while encoding"),
                Hw::Crashes => done("", false, ""),
                Hw::Hangs => Ok(process(CmdOutput::default(), true)),
            };
        }
        if line.contains("-xerror") {
            self.note("decode");
            self.cancel_if(CancelAt::Check);
            let packets = if self.hw == Hw::DropsFrames {
                FRAMES - 7
            } else {
                FRAMES
            };
            return done(&format!("frame={packets}\nprogress=end\n"), true, "");
        }
        if line.contains("psnr=stats_file") {
            self.note("compare");
            let y = if self.hw == Hw::GreenFrames {
                "psnr_y:30.0 psnr_u:8.0 psnr_v:7.0"
            } else {
                "psnr_y:41.0 psnr_u:40.0 psnr_v:39.5"
            };
            let log: String = (1..=9).map(|n| format!("n:{n} {y}\n")).collect();
            std::fs::write(cwd.unwrap().join(outcheck::PSNR_LOG), log).unwrap();
            return done(
                &format!("frame={FRAMES}\nout_time_us=1966667\nprogress=end\n"),
                true,
                "",
            );
        }
        // the normal method
        self.note("software");
        self.cancel_if(CancelAt::Redo);
        let out = out_file(&a).unwrap();
        assert!(!out.contains(".hw."), "{out}");
        if self.software_fails {
            return done("", false, "Conversion failed!");
        }
        std::fs::write(&out, b"software video").unwrap();
        done(PROGRESS, true, "")
    }
}

/// A job that re-encodes to H.264 (a fade), may use `vendor`, with a short stall limit.
fn hw_job(dir: &Path, vendor: Option<Vendor>) -> ExportJob {
    let mut j = job(dir);
    j.project
        .timeline
        .append(&j.project.media[0].clone())
        .unwrap();
    j.project.timeline.toggle_fade(0).unwrap();
    j.project.media[0].format.bit_rate = Some(2_000_000);
    j.hw = vendor.map(|v| HwOffer {
        h264: Some(Candidate {
            vendor: v,
            device: (v == Vendor::Vaapi).then(|| "/dev/dri/renderD128".into()),
            rc: Default::default(),
        }),
        hevc: None,
    });
    j.stall_timeout = Duration::from_millis(80);
    j
}

/// The result of an export, its events and what ran.
type Outcome = (
    Result<(PathBuf, ExportMethod), ExportError>,
    Vec<ExportEvent>,
    Vec<&'static str>,
);

/// Runs the export; returns the result, the events and what ran.
fn export(j: &ExportJob, s: &Sim) -> Outcome {
    let (tx, rx) = channel();
    let r = run(j, s, &s.cancel, &tx);
    (r, rx.try_iter().collect(), s.log.lock().unwrap().clone())
}

/// No partial file of either kind is left.
fn clean(dir: &Path) {
    for e in std::fs::read_dir(dir).unwrap() {
        let n = e.unwrap().file_name().to_string_lossy().into_owned();
        assert!(!n.ends_with("videolore-partial"), "left behind: {n}");
    }
}

fn progress(ev: &[ExportEvent]) -> Vec<f32> {
    ev.iter()
        .filter_map(|e| match e {
            ExportEvent::Progress(f) => Some(*f),
            _ => None,
        })
        .collect()
}

// covers: REQ-055
#[test]
fn working_hardware_makes_the_file_after_the_check() {
    for v in Vendor::ALL {
        let dir = tempfile::tempdir().unwrap();
        let s = sim(Hw::Works);
        let (r, ev, log) = export(&hw_job(dir.path(), Some(v)), &s);
        let (path, method) = r.unwrap();
        assert_eq!(method, ExportMethod::Hardware(v));
        assert_eq!(std::fs::read(&path).unwrap(), b"hardware video");
        assert_eq!(log, ["hardware", "metadata", "decode", "compare"]);
        let p = progress(&ev);
        assert!(p.windows(2).all(|w| w[0] <= w[1]), "{p:?}");
        assert!(p.iter().any(|f| *f >= 0.85) && p.iter().all(|f| (0.0..=1.0).contains(f)));
        clean(dir.path());
    }
}

/// Each way the hardware can fail, on each vendor path: a finished file made by the
/// normal method, no error, nothing left behind.
// covers: REQ-055
#[test]
fn every_hardware_failure_is_redone_with_the_normal_method() {
    let cases = [
        (Hw::ExitsWithError, Problem::ExitedWithError),
        (Hw::Crashes, Problem::ExitedWithError),
        (Hw::Hangs, Problem::Stalled),
        (
            Hw::GreenFrames,
            Problem::CheckFailed(CheckFailure::Picture {
                index: 0,
                psnr: 7.0,
            }),
        ),
        (
            Hw::DropsFrames,
            Problem::CheckFailed(CheckFailure::FrameCount {
                expected: FRAMES,
                got: FRAMES - 7,
            }),
        ),
    ];
    for v in Vendor::ALL {
        for (hw, problem) in &cases {
            let dir = tempfile::tempdir().unwrap();
            let s = sim(*hw);
            let t = Instant::now();
            let (r, ev, log) = export(&hw_job(dir.path(), Some(v)), &s);
            let (path, method) = r.unwrap_or_else(|e| panic!("{v:?} {hw:?}: {e}"));
            assert_eq!(
                method,
                ExportMethod::SoftwareAfterProblem {
                    codec: Codec::H264,
                    vendor: v,
                    problem: problem.clone()
                },
                "{v:?} {hw:?}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"software video");
            assert_eq!(log.first(), Some(&"hardware"));
            assert_eq!(log.last(), Some(&"software"));
            assert!(!ev.iter().any(|e| matches!(e, ExportEvent::Failed(_))));
            // the bar goes back at most once (to 0) for the redo and stays within 0..1
            let p = progress(&ev);
            assert!(
                p.windows(2).filter(|w| w[1] < w[0]).count() <= 1,
                "{v:?} {hw:?} {p:?}"
            );
            if matches!(hw, Hw::GreenFrames | Hw::DropsFrames) {
                assert_eq!(p.windows(2).filter(|w| w[1] < w[0]).count(), 1, "{p:?}");
            }
            assert!(p.iter().all(|f| (0.0..=1.0).contains(f)));
            clean(dir.path());
            if *hw == Hw::Hangs {
                assert!(
                    t.elapsed() < Duration::from_secs(5),
                    "stall detected quickly"
                );
            }
        }
    }
}

/// No hardware for the format (or Best quality): the normal method, hardware never run.
// covers: REQ-055
#[test]
fn without_usable_hardware_the_normal_method_runs() {
    let dir = tempfile::tempdir().unwrap();
    // Best quality: no offer at all
    let s = sim(Hw::Works);
    let (r, _, log) = export(&hw_job(dir.path(), None), &s);
    assert_eq!(r.unwrap().1, ExportMethod::Software);
    assert_eq!(log, ["software"]);
    // an offer for HEVC only does not help an H.264 export
    let mut j = hw_job(dir.path(), Some(Vendor::Nvenc));
    j.hw = Some(HwOffer {
        h264: None,
        hevc: Some(Candidate::new(Vendor::Nvenc)),
    });
    let s = sim(Hw::Works);
    assert_eq!(export(&j, &s).0.unwrap().1, ExportMethod::Software);
    assert_eq!(*s.log.lock().unwrap(), ["software"]);
    // an unknown source bitrate uses the normal method
    let mut j = hw_job(dir.path(), Some(Vendor::Nvenc));
    j.project.media[0].format.bit_rate = None;
    let s = sim(Hw::Works);
    assert_eq!(export(&j, &s).0.unwrap().1, ExportMethod::Software);
    // stream copy never uses the hardware
    let mut j = job(dir.path());
    j.hw = hw_job(dir.path(), Some(Vendor::Nvenc)).hw;
    let s = sim(Hw::Works);
    let r = FakeRunner::default().on(|_, a| {
        let out = a.last()?.strip_prefix("file:")?.to_string();
        std::fs::write(out, b"copy").ok()?;
        ok("progress=end\n")
    });
    let (tx, _rx) = channel();
    let (_, m) = run(&j, &r, &AtomicBool::new(false), &tx).unwrap();
    assert_eq!(m, ExportMethod::StreamCopy);
    assert!(s.log.lock().unwrap().is_empty());
    clean(dir.path());
}

/// Cancel during the hardware run, its check or the redo: cancelled, no redo started
/// after a cancel, no partial file.
// covers: REQ-055, REQ-021
#[test]
fn cancel_stops_every_phase() {
    for (at, hw) in [
        (CancelAt::HardwareRun, Hw::Hangs),
        (CancelAt::HardwareRun, Hw::Works),
        (CancelAt::Check, Hw::Works),
        (CancelAt::Redo, Hw::ExitsWithError),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut s = sim(hw);
        s.cancel_at = at;
        let (r, ev, log) = export(&hw_job(dir.path(), Some(Vendor::Qsv)), &s);
        assert_eq!(r, Err(ExportError::Cancelled), "{at:?} {hw:?}");
        if at != CancelAt::Redo {
            assert!(
                !log.contains(&"software"),
                "{at:?}: no redo after a cancel: {log:?}"
            );
        }
        assert!(!ev.iter().any(|e| matches!(e, ExportEvent::Done { .. })));
        clean(dir.path());
        assert!(!dir.path().join("out.mp4").exists());
    }
}

/// When the redo fails too, the export fails exactly as before (the cause was not the
/// hardware).
// covers: REQ-055
#[test]
fn a_failing_redo_is_reported_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = sim(Hw::Crashes);
    s.software_fails = true;
    let (r, _, log) = export(&hw_job(dir.path(), Some(Vendor::Amf)), &s);
    assert!(matches!(r, Err(ExportError::Ffmpeg(m)) if m.contains("Conversion failed")));
    assert_eq!(log, ["hardware", "software"]);
    clean(dir.path());
    // through the worker thread: a Failed event
    let ev = start(
        hw_job(dir.path(), Some(Vendor::Amf)),
        Arc::new({
            let mut s = sim(Hw::Crashes);
            s.software_fails = true;
            s
        }),
    )
    .join();
    assert!(matches!(ev.last(), Some(ExportEvent::Failed(_))));
    assert_eq!(
        hw_partial_path(Path::new("/x/a.mp4")),
        PathBuf::from("/x/.a.mp4.hw.videolore-partial")
    );
    for p in [
        Problem::ExitedWithError,
        Problem::Stalled,
        Problem::CheckFailed(CheckFailure::Decode),
    ] {
        assert!(!p.text().is_empty());
    }
}

/// A hardware attempt that cannot even be prepared stops the export with the error.
#[test]
fn hardware_preparation_errors_stop_the_export() {
    let dir = tempfile::tempdir().unwrap();
    let mut j = hw_job(dir.path(), Some(Vendor::Nvenc));
    // the hardware partial name is taken by a folder: it cannot be created
    let out = dir.path().canonicalize().unwrap().join("out.mp4");
    std::fs::create_dir(hw_partial_path(&out)).unwrap();
    std::fs::write(hw_partial_path(&out).join("x"), b"").unwrap();
    j.output = out;
    let s = sim(Hw::Works);
    let (r, _, log) = export(&j, &s);
    assert!(matches!(r, Err(ExportError::Io(_))), "{r:?}");
    assert!(log.is_empty());
}

/// A VA-API driver that only does constant quality: a run growing much bigger than the
/// normal method is restarted once with a coarser quantiser; a file still too big is redone
/// with the normal method.
// covers: REQ-055, REQ-063
#[test]
fn constant_quality_keeps_the_file_size_in_check() {
    let cq = |j: &mut ExportJob| {
        j.hw = Some(HwOffer {
            h264: Some(Candidate {
                vendor: Vendor::Vaapi,
                device: Some("/dev/dri/renderD128".into()),
                rc: hwenc::RateControl::ConstantQuality,
            }),
            hevc: None,
        });
    };
    // a normal video: one run at the first quantiser
    let dir = tempfile::tempdir().unwrap();
    let mut j = hw_job(dir.path(), Some(Vendor::Vaapi));
    cq(&mut j);
    let s = sim(Hw::Works);
    let (r, _, _) = export(&j, &s);
    assert_eq!(r.unwrap().1, ExportMethod::Hardware(Vendor::Vaapi));
    assert_eq!(*s.qps.lock().unwrap(), ["23"]);
    // too big at first: a second run with a coarser quantiser, which passes
    let s = sim(Hw::BigAtFirst);
    let (r, ev, log) = export(&j, &s);
    assert_eq!(r.unwrap().1, ExportMethod::Hardware(Vendor::Vaapi));
    let qps = s.qps.lock().unwrap().clone();
    assert_eq!(qps.len(), 2);
    assert!(qps[1].parse::<u32>().unwrap() > 23, "{qps:?}");
    assert_eq!(log.iter().filter(|x| **x == "hardware").count(), 2);
    assert!(progress(&ev).iter().all(|f| (0.0..=1.0).contains(f)));
    clean(dir.path());
    // always too big: the normal method makes the file
    let s = sim(Hw::AlwaysBig);
    let (r, _, log) = export(&j, &s);
    match r.unwrap().1 {
        ExportMethod::SoftwareAfterProblem {
            problem: Problem::CheckFailed(CheckFailure::TooLarge(x)),
            ..
        } => assert!(x > 1.5),
        other => panic!("{other:?}"),
    }
    assert_eq!(log.last(), Some(&"software"));
    clean(dir.path());
}
