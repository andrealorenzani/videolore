// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of "Export: Faster" in the app: when the hardware check runs, what the export
//! may use, and the memory of hardware problems.

use super::tests::{FakeDialogs, files, import, info, read_settings, runner};
use super::*;
use crate::ffmpeg::CmdOutput;
use crate::ffmpeg::fake::{FakeRunner, fail, ok};
use crate::hwenc::{Candidate, Os, Vendor};
use crate::preview::NullSink;
use std::sync::Mutex;

/// Linux without render devices: NVIDIA, then Intel Quick Sync.
fn linux() -> Platform {
    Platform {
        os: Os::Linux,
        render_nodes: Vec::new(),
        system_libva_new: None,
    }
}

/// The encoders `working` pass the hardware check; a hardware export ends with
/// `hw_export` (a normal export always works).
struct World {
    working: Arc<Mutex<Vec<String>>>,
    hw_export: Option<CmdOutput>,
}

fn fake(world: World) -> Arc<FakeRunner> {
    let base = runner();
    Arc::new(FakeRunner::default().on(move |prog, a| {
        if a.iter().any(|x| x.starts_with("testsrc2")) {
            let i = a.iter().position(|x| x == "-c:v")?;
            return if world.working.lock().unwrap().contains(&a[i + 1]) {
                ok("")
            } else {
                fail("No capable devices found")
            };
        }
        if a.last()?.ends_with(".hw.videolore-partial") && a.iter().any(|x| x == "-c:v") {
            let out = a.last()?.strip_prefix("file:")?.to_string();
            std::fs::write(out, b"hw").ok()?;
            return world.hw_export.clone();
        }
        base.responders[0](prog, a)
    }))
}

fn app_in(dir: &Path, world: World) -> (AppState, Arc<FakeRunner>) {
    let f = fake(world);
    let mut i = info();
    i.encoders.extend(
        ["h264_nvenc", "hevc_nvenc", "h264_qsv", "hevc_qsv"]
            .iter()
            .map(|s| s.to_string()),
    );
    let mut app = AppState::with_ffmpeg(
        Ok(i),
        f.clone(),
        Box::new(FakeDialogs::default()),
        Box::new(NullSink),
    );
    app.platform = linux;
    app.settings_path = Some(dir.join("settings.json"));
    app.work_root = dir.join("work");
    (app, f)
}

fn working(list: &[&str]) -> Arc<Mutex<Vec<String>>> {
    Arc::new(Mutex::new(list.iter().map(|s| s.to_string()).collect()))
}

fn probes(f: &FakeRunner) -> usize {
    f.calls
        .lock()
        .unwrap()
        .iter()
        .filter(|a| a.iter().any(|x| x.starts_with("testsrc2")))
        .count()
}

fn hardware_runs(f: &FakeRunner) -> usize {
    f.calls
        .lock()
        .unwrap()
        .iter()
        .filter(|a| {
            a.last()
                .is_some_and(|l| l.ends_with(".hw.videolore-partial"))
                && a.iter().any(|x| x == "-c:v")
        })
        .count()
}

/// Ticks until the hardware check is over.
fn checked(app: &mut AppState) -> HwStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.tick(Instant::now());
        if let HwProbe::Done(s) = &app.hw.probe {
            return s.clone();
        }
        assert!(Instant::now() < deadline, "hardware check did not end");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Two clips with a fade (a re-encode to H.264).
fn timeline(app: &mut AppState, dir: &Path) {
    import(app, files(dir));
    let now = Instant::now();
    for i in 0..2 {
        let id = app.project.media[i].id;
        app.dispatch(Command::Append(Some(id)), now).unwrap();
    }
    app.dispatch(Command::ToggleFade(0), now).unwrap();
}

fn export(app: &mut AppState, out: &Path) {
    let now = Instant::now();
    app.dispatch(Command::Export(out.to_path_buf()), now)
        .unwrap();
    app.wait_export(now);
}

/// Best quality (the default): no hardware check, the hardware is never used.
// covers: REQ-060, REQ-061
#[test]
fn best_quality_never_checks_or_uses_the_hardware() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, f) = app_in(
        dir.path(),
        World {
            working: working(&["h264_nvenc"]),
            hw_export: ok("progress=end\n"),
        },
    );
    assert_eq!(app.settings.export, ExportChoice::BestQuality);
    timeline(&mut app, dir.path());
    // a new ffmpeg (start-up check) with Best quality: still nothing
    app.set_ffmpeg(Ok(info()));
    app.tick(Instant::now());
    assert_eq!(app.hw.probe, HwProbe::NotChecked);
    assert!(app.hw_offer().is_none());
    export(&mut app, &dir.path().join("best.mp4"));
    assert_eq!(probes(&f), 0);
    assert_eq!(hardware_runs(&f), 0);
    assert_eq!(app.hw.last_method, Some(ExportMethod::Software));
    let lines = crate::ui::settings::hardware_lines(&app);
    assert!(lines[0].contains("not checked (only checked when Export is Faster)"));
    assert!(lines.iter().any(|l| l == "Last export: normal method"));
}

/// Choosing Faster saves the choice, makes the settings custom and checks the hardware
/// in the background; the next export uses it.
// covers: REQ-060, REQ-061, REQ-055
#[test]
fn faster_checks_the_hardware_and_the_next_export_uses_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, f) = app_in(
        dir.path(),
        World {
            working: working(&["h264_qsv", "hevc_nvenc"]),
            hw_export: ok("progress=end\n"),
        },
    );
    timeline(&mut app, dir.path());
    let doc_before = app.project.doc();
    let dirty_before = app.dirty;
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    assert!(!app.is_recommended());
    assert!(matches!(app.hw.probe, HwProbe::Checking(_)));
    // an export started before the check ends would use the normal method
    assert!(app.hw_offer().is_none());
    assert_eq!(
        read_settings(&dir.path().join("settings.json")).export,
        ExportChoice::Faster
    );
    // a program setting, never part of the project
    assert_eq!(app.project.doc(), doc_before);
    assert_eq!(app.dirty, dirty_before);
    let st = checked(&mut app);
    assert_eq!(st.h264, Some(Candidate::new(Vendor::Qsv)));
    assert_eq!(st.hevc, Some(Candidate::new(Vendor::Nvenc)));
    // NVIDIA and Intel for H.264 (stops at Intel), NVIDIA for HEVC
    assert_eq!(probes(&f), 3);
    // the check used the ffmpeg in use
    let offer = app.hw_offer().unwrap();
    assert_eq!(offer.h264, Some(Candidate::new(Vendor::Qsv)));
    // the hardware export runs (its output check fails with this fake: redo), and the
    // memory remembers the problem
    export(&mut app, &dir.path().join("fast.mp4"));
    assert_eq!(hardware_runs(&f), 1);
    let lines = crate::ui::settings::hardware_lines(&app);
    assert!(
        lines.contains(
            &"Video hardware for H.264: Intel Quick Sync — turned off after a problem".to_string()
        ),
        "{lines:?}"
    );
    assert!(lines.contains(&"Video hardware for HEVC: NVIDIA (NVENC) — works".to_string()));
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("NVIDIA (H.264): not available"))
    );
    // switching back and forth does not check again
    app.dispatch(
        Command::SetExport(ExportChoice::BestQuality),
        Instant::now(),
    )
    .unwrap();
    assert!(app.hw_offer().is_none());
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    assert_eq!(probes(&f), 3);
}

/// A hardware problem turns the method off for that format until one of the resets.
// covers: REQ-064, REQ-055
#[test]
fn a_hardware_problem_is_remembered_until_a_reset() {
    let dir = tempfile::tempdir().unwrap();
    let w = working(&["h264_nvenc"]);
    let (mut app, f) = app_in(
        dir.path(),
        World {
            working: Arc::clone(&w),
            // the hardware run crashes
            hw_export: fail(""),
        },
    );
    timeline(&mut app, dir.path());
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    checked(&mut app);
    export(&mut app, &dir.path().join("one.mp4"));
    assert!(
        dir.path().join("one.mp4").is_file(),
        "the user still gets a file"
    );
    assert!(app.notice.is_none(), "no error shown");
    assert!(matches!(
        app.hw.last_method,
        Some(ExportMethod::SoftwareAfterProblem {
            codec: Codec::H264,
            vendor: Vendor::Nvenc,
            problem: export::Problem::ExitedWithError
        })
    ));
    assert!(app.settings.hardware_is_off(Codec::H264, Vendor::Nvenc));
    let saved = read_settings(&dir.path().join("settings.json"));
    assert_eq!(saved.hardware_off[0].version, env!("CARGO_PKG_VERSION"));
    let lines = crate::ui::settings::hardware_lines(&app);
    assert!(lines.contains(&crate::ui::settings::HARDWARE_OFF_LINE.to_string()));
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Last export: normal method, after a video hardware problem"))
    );
    // the next export goes straight to the normal method
    assert!(app.hw_offer().is_none());
    export(&mut app, &dir.path().join("two.mp4"));
    assert_eq!(hardware_runs(&f), 1);
    // reset 1: "Try video hardware again" forgets and checks again
    let before = probes(&f);
    app.dispatch(Command::TryHardwareAgain, Instant::now())
        .unwrap();
    assert!(app.settings.hardware_off.is_empty());
    checked(&mut app);
    assert!(probes(&f) > before);
    assert!(app.hw_offer().is_some());
    assert!(
        read_settings(&dir.path().join("settings.json"))
            .hardware_off
            .is_empty()
    );
    // a new failure, then reset 2: "Use recommended settings" (back to Best quality)
    export(&mut app, &dir.path().join("three.mp4"));
    assert!(app.settings.hardware_is_off(Codec::H264, Vendor::Nvenc));
    let before = probes(&f);
    app.dispatch(Command::UseRecommended, Instant::now())
        .unwrap();
    assert!(app.settings.hardware_off.is_empty());
    assert_eq!(app.settings.export, ExportChoice::BestQuality);
    checked(&mut app);
    assert!(probes(&f) > before, "the check runs again");
    // reset 3: a different method now wins the check (new hardware): tried again
    app.settings.export = ExportChoice::Faster;
    app.settings
        .turn_hardware_off(Codec::H264, Vendor::Nvenc, env!("CARGO_PKG_VERSION"));
    assert!(app.hw_offer().is_none());
    *w.lock().unwrap() = vec!["h264_qsv".to_string()];
    app.dispatch(Command::TryHardwareAgain, Instant::now())
        .unwrap();
    app.settings
        .turn_hardware_off(Codec::H264, Vendor::Nvenc, env!("CARGO_PKG_VERSION"));
    checked(&mut app);
    assert_eq!(
        app.hw_offer().and_then(|o| o.h264),
        Some(Candidate::new(Vendor::Qsv))
    );
    // resets 4 and 5 (new version, new computer) are decided when the settings load:
    // see settings::tests::hardware_memory_resets_with_a_new_version_or_computer
}

/// A cancelled export never turns the hardware off.
// covers: REQ-064
#[test]
fn a_cancelled_export_is_not_a_hardware_problem() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _f) = app_in(
        dir.path(),
        World {
            working: working(&["h264_nvenc"]),
            hw_export: ok("progress=end\n"),
        },
    );
    let (tx, rx) = channel();
    tx.send(ExportEvent::Cancelled).unwrap();
    app.export = Some(ExportHandle::from_events(rx));
    app.export_status.running = true;
    app.tick(Instant::now());
    assert!(app.settings.hardware_off.is_empty());
    assert!(app.hw.last_method.is_none());
    // a constant-quality file too big for this video is not a hardware fault either
    let (tx, rx) = channel();
    let method = ExportMethod::SoftwareAfterProblem {
        codec: Codec::H264,
        vendor: Vendor::Vaapi,
        problem: export::Problem::CheckFailed(crate::outcheck::CheckFailure::TooLarge(2.0)),
    };
    tx.send(ExportEvent::Done {
        path: dir.path().join("x.mp4"),
        method: method.clone(),
    })
    .unwrap();
    app.export = Some(ExportHandle::from_events(rx));
    app.export_status.running = true;
    app.tick(Instant::now());
    assert!(app.settings.hardware_off.is_empty());
    assert_eq!(app.hw.last_method, Some(method));
}

/// Results of an older check (the ffmpeg changed meanwhile) are ignored; a new ffmpeg
/// with Faster selected is checked again.
// covers: REQ-061
#[test]
fn stale_checks_are_ignored_and_a_new_ffmpeg_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, f) = app_in(
        dir.path(),
        World {
            working: working(&["h264_nvenc"]),
            hw_export: ok("progress=end\n"),
        },
    );
    app.settings.export = ExportChoice::Faster;
    app.start_hw_probe();
    let HwProbe::Checking(first) = app.hw.probe else {
        panic!()
    };
    // the user picks another ffmpeg: a new check replaces the old one
    let mut other = info();
    other.ffmpeg = "/opt/mine/ffmpeg".into();
    other.encoders.insert("h264_nvenc".into());
    app.set_ffmpeg(Ok(other));
    let HwProbe::Checking(second) = app.hw.probe else {
        panic!()
    };
    assert!(second > first);
    let st = checked(&mut app);
    assert_eq!(st.h264, Some(Candidate::new(Vendor::Nvenc)));
    assert!(probes(&f) >= 1);
    // an ffmpeg without the filter of the output check never gets the hardware
    app.settings.export = ExportChoice::Faster;
    assert!(app.hw_offer().is_some());
    if let Ok(i) = &mut app.ffmpeg {
        i.filters.remove(crate::outcheck::CHECK_FILTER);
    }
    assert!(app.hw_offer().is_none());
    // a broken ffmpeg cannot be checked
    app.set_ffmpeg(Err("no ffmpeg".into()));
    app.start_hw_probe();
    assert_eq!(app.hw.probe, HwProbe::NotChecked);
    // the "no video hardware" note shows only when the check found nothing
    app.hw.probe = HwProbe::Done(HwStatus::default());
    assert!(crate::ui::settings::no_hardware(&app));
    let lines = crate::ui::settings::hardware_lines(&app);
    assert!(lines[0].ends_with("none found"));
    app.hw.probe = HwProbe::Checking(9);
    assert!(!crate::ui::settings::no_hardware(&app));
    assert!(crate::ui::settings::hardware_lines(&app)[1].ends_with("checking…"));
}
