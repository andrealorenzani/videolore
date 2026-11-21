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
        libva_version: None,
    }
}

/// The encoders `working` pass the hardware check; a hardware export ends with
/// `hw_export` (a normal export always works).
struct World {
    working: Arc<Mutex<Vec<String>>>,
    hw_export: Option<CmdOutput>,
}

fn fake(world: World) -> Arc<FakeRunner> {
    Arc::new(fake_runner(world))
}

fn fake_runner(world: World) -> FakeRunner {
    let base = runner();
    FakeRunner::default().on(move |prog, a| {
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
    })
}

fn app_in(dir: &Path, world: World) -> (AppState, Arc<FakeRunner>) {
    app_with(dir, fake(world))
}

fn app_with(dir: &Path, f: Arc<FakeRunner>) -> (AppState, Arc<FakeRunner>) {
    let mut i = info();
    i.encoders.extend(
        [
            "h264_nvenc",
            "hevc_nvenc",
            "h264_qsv",
            "hevc_qsv",
            "h264_vaapi",
        ]
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
        // the clips of the decoder probe are not encoder probes
        .filter(|a| !a.iter().any(|x| x == "matroska"))
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
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        app.tick(Instant::now());
        if let HwProbe::Done(s) = &app.hw.probe {
            return s.clone();
        }
        assert!(Instant::now() < deadline, "hardware check did not end");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Ticks until the hardware check and its possible repeat are over.
fn settled(app: &mut AppState) -> HwStatus {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        app.tick(Instant::now());
        if let HwProbe::Done(s) = &app.hw.probe
            && app.hw_probe.is_none()
        {
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
    assert_eq!(
        st.get(Codec::H264).cloned(),
        Some(Candidate::new(Vendor::Qsv))
    );
    assert_eq!(
        st.get(Codec::Hevc).cloned(),
        Some(Candidate::new(Vendor::Nvenc))
    );
    // NVIDIA and Intel for H.264, NVIDIA and Intel for HEVC (every method is tried: the
    // list of working devices is complete, REQ-216)
    assert_eq!(probes(&f), 4);
    // the check used the ffmpeg in use
    let offer = app.hw_offer().unwrap();
    assert_eq!(
        offer.get(Codec::H264).cloned(),
        Some(Candidate::new(Vendor::Qsv))
    );
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
    assert_eq!(probes(&f), 4);
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
        app.hw_offer().and_then(|o| o.get(Codec::H264).cloned()),
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
    assert_eq!(
        st.get(Codec::H264).cloned(),
        Some(Candidate::new(Vendor::Nvenc))
    );
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

/// A check whose answer is inconclusive because a candidate did not answer in time (a
/// busy computer) is repeated once by itself; a time-out is never remembered as a
/// hardware problem.
// covers: REQ-061, REQ-064, REQ-211
#[test]
fn a_check_without_an_answer_in_time_is_repeated_once() {
    for (slow, found, runs) in [(2, Some(Candidate::new(Vendor::Nvenc)), 9), (4, None, 10)] {
        let dir = tempfile::tempdir().unwrap();
        let hangs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let h2 = Arc::clone(&hangs);
        let f = fake_runner(World {
            working: working(&["h264_nvenc"]),
            hw_export: ok("progress=end\n"),
        })
        .hanging_if(move |a| {
            a.iter().any(|x| x == "testsrc2=size=640x360:rate=30")
                && a.iter().any(|x| x == "h264_nvenc")
                && h2.fetch_add(1, Ordering::SeqCst) < slow
        });
        let (mut app, f) = app_with(dir.path(), Arc::new(f));
        // short limits, but long enough that an answering fake is never late on a busy
        // test machine (30 ms was flaky when the whole suite ran in parallel)
        app.probe_limits = ProbeLimits {
            per: Duration::from_millis(200),
            retry: Duration::from_millis(400),
            total: Duration::from_secs(5),
        };
        app.settings.export = ExportChoice::Faster;
        app.start_hw_probe();
        // the first answer is inconclusive: it is in use while the check is repeated
        let first = checked(&mut app);
        assert!(first.inconclusive() && first.get(Codec::H264).is_none());
        assert!(app.hw_probe.is_some(), "repeating");
        assert_eq!(app.hw.probe, HwProbe::Done(first));
        // REQ-211: Advanced says so during the repeat
        assert!(app.hw_repeating());
        assert!(
            crate::ui::settings::advanced_lines(&app)
                .iter()
                .any(|l| l == crate::ui::settings::REPEATING),
        );
        let st = settled(&mut app);
        // and the result replaces the text
        assert!(!app.hw_repeating());
        assert!(
            !crate::ui::settings::advanced_lines(&app)
                .iter()
                .any(|l| l == crate::ui::settings::REPEATING),
        );
        assert_eq!(st.get(Codec::H264).cloned(), found, "slow {slow}");
        // first check: NVIDIA twice (retry) + Intel for H.264, NVIDIA + Intel for HEVC;
        // the repeat: NVIDIA (1 or 2 tries) + Intel for H.264, the same 2 for HEVC
        assert_eq!(probes(&f), runs, "slow {slow}");
        assert_eq!(st.inconclusive(), found.is_none());
        // never more than one repeat
        for _ in 0..20 {
            app.tick(Instant::now());
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(app.hw.probe, HwProbe::Done(st));
        assert_eq!(probes(&f), runs);
        assert!(app.settings.hardware_off.is_empty());
        // the user can ask again: a new request may be repeated once more
        app.dispatch(Command::TryHardwareAgain, Instant::now())
            .unwrap();
        assert!(matches!(app.hw.probe, HwProbe::Checking(_)));
        let again = settled(&mut app);
        assert!(
            again.get(Codec::H264).is_some(),
            "the computer is quiet now"
        );
        assert!(app.settings.hardware_off.is_empty());
    }
}

/// The check does not run while an export or a subtitle job runs: it waits and starts when
/// the job ends; one check at a time.
// covers: REQ-208
#[test]
fn the_check_waits_for_an_export_or_a_subtitle_job() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, f) = app_in(
        dir.path(),
        World {
            working: working(&["h264_nvenc"]),
            hw_export: ok("progress=end\n"),
        },
    );
    // an export is running when the check is asked for: no check process starts
    app.export_status.running = true;
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    for _ in 0..20 {
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(probes(&f), 0);
    assert!(
        matches!(app.hw.probe, HwProbe::Checking(_)),
        "still waiting"
    );
    // the export ends: the check starts and finishes
    app.export_status.running = false;
    let st = checked(&mut app);
    assert!(st.get(Codec::H264).is_some());
    let after = probes(&f);
    assert!(after > 0);
    // a job that starts while a check runs stops it: it is begun again after the job
    app.export_status.running = true;
    app.dispatch(Command::TryHardwareAgain, Instant::now())
        .unwrap();
    assert!(matches!(app.hw.probe, HwProbe::Checking(_)));
    app.tick(Instant::now());
    assert_eq!(probes(&f), after, "no check process during the job");
    assert!(app.hw_probe.is_none() && app.hw_paused);
    app.export_status.running = false;
    let st = checked(&mut app);
    assert!(st.get(Codec::H264).is_some());
    assert!(probes(&f) > after, "the check ran after the job");
    assert!(!app.hw_paused);
}

/// A candidate that ran out of time twice is skipped for the rest of the session; a restart
/// tries it again.
// covers: REQ-209
#[test]
fn a_candidate_that_ran_out_of_time_twice_is_skipped_until_the_restart() {
    let dir = tempfile::tempdir().unwrap();
    let nvenc_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let n2 = Arc::clone(&nvenc_calls);
    let hanging = || {
        let n = Arc::clone(&n2);
        fake_runner(World {
            working: working(&["h264_qsv"]),
            hw_export: ok("progress=end\n"),
        })
        .hanging_if(move |a| {
            let h = a.iter().any(|x| x == "h264_nvenc");
            if h {
                n.fetch_add(1, Ordering::SeqCst);
            }
            h
        })
    };
    let (mut app, _) = app_with(dir.path(), Arc::new(hanging()));
    app.probe_limits = ProbeLimits {
        // long enough that the Intel fake, which answers, is never late on a busy machine
        per: Duration::from_millis(200),
        retry: Duration::from_millis(400),
        total: Duration::from_secs(10),
    };
    app.settings.export = ExportChoice::Faster;
    app.start_hw_probe();
    // first check: NVIDIA is tried twice (its retry), then Intel is used; the repeat of an
    // inconclusive check tries NVIDIA again
    let first = settled(&mut app);
    assert_eq!(first.get(Codec::H264).map(|c| c.vendor), Some(Vendor::Qsv));
    let after_first = nvenc_calls.load(Ordering::SeqCst);
    assert!(after_first >= 2);
    // the next check of the session does not wait for NVIDIA: it is skipped at once
    app.start_hw_probe();
    let second = settled(&mut app);
    assert_eq!(
        nvenc_calls.load(Ordering::SeqCst),
        after_first,
        "not started again"
    );
    assert!(second.notes.contains(&(
        Codec::H264,
        Vendor::Nvenc,
        crate::hwenc::Reason::SkippedSlow
    )));
    assert_eq!(second.get(Codec::H264).map(|c| c.vendor), Some(Vendor::Qsv));
    let lines = crate::ui::settings::hardware_lines(&app);
    assert!(
        lines.iter().any(|l| l.contains("skipped: too slow twice")),
        "{lines:?}"
    );
    // a restart (a new session) tries it again
    let (mut fresh, _) = app_with(dir.path(), Arc::new(hanging()));
    fresh.probe_limits = app.probe_limits;
    fresh.settings.export = ExportChoice::Faster;
    fresh.start_hw_probe();
    checked(&mut fresh);
    assert!(
        nvenc_calls.load(Ordering::SeqCst) > after_first,
        "tried again"
    );
    // "Try video hardware again" is a new chance too
    app.dispatch(Command::TryHardwareAgain, Instant::now())
        .unwrap();
    let n = nvenc_calls.load(Ordering::SeqCst);
    settled(&mut app);
    assert!(nvenc_calls.load(Ordering::SeqCst) > n);
}

fn two_cards() -> Platform {
    Platform {
        os: Os::Linux,
        render_nodes: vec!["/dev/dri/renderD128".into(), "/dev/dri/renderD129".into()],
        system_libva_new: Some(true),
        libva_version: Some((2, 21)),
    }
}

/// Settings > Advanced lists the working devices; the choice is used by the next export, is
/// a program setting, and Automatic restores the order.
// covers: REQ-216
#[test]
fn the_chosen_graphics_card_is_used_and_saved() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = app_in(
        dir.path(),
        World {
            working: working(&["h264_nvenc", "h264_vaapi"]),
            hw_export: ok("progress=end\n"),
        },
    );
    app.platform = two_cards;
    timeline(&mut app, dir.path());
    let doc_before = app.project.doc();
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    checked(&mut app);
    let names = crate::ui::settings::graphics_card_choices(&app);
    assert_eq!(
        names,
        [
            "NVIDIA (NVENC)",
            "VA-API (renderD128)",
            "VA-API (renderD129)"
        ]
    );
    // Automatic: the preference order
    assert!(app.settings.graphics_card.is_none());
    assert_eq!(
        app.hw_offer().unwrap().get(Codec::H264).map(|c| c.vendor),
        Some(Vendor::Nvenc)
    );
    // the second device
    app.dispatch(
        Command::SetGraphicsCard(Some("VA-API (renderD129)".into())),
        Instant::now(),
    )
    .unwrap();
    let c = app.hw_offer().unwrap().get(Codec::H264).cloned().unwrap();
    assert_eq!(c.device, Some("/dev/dri/renderD129".into()));
    // a program setting: saved, not in the project
    let saved = read_settings(&dir.path().join("settings.json"));
    assert_eq!(saved.graphics_card.as_deref(), Some("VA-API (renderD129)"));
    assert_eq!(app.project.doc(), doc_before);
    let project_text = crate::project::to_json(&app.project, &dir.path().join("p.vlproj")).unwrap();
    assert!(!project_text.contains("renderD129"));
    // it survives a restart
    let again = crate::settings::Settings::default();
    assert!(again.graphics_card.is_none());
    let reloaded = read_settings(&dir.path().join("settings.json"));
    assert_eq!(reloaded.graphics_card, app.settings.graphics_card);
    // a name that is not a working device is Automatic
    app.dispatch(
        Command::SetGraphicsCard(Some("renderD1; rm -rf /".into())),
        Instant::now(),
    )
    .unwrap();
    assert!(app.settings.graphics_card.is_none());
    // Automatic restores the order
    app.dispatch(
        Command::SetGraphicsCard(Some("VA-API (renderD128)".into())),
        Instant::now(),
    )
    .unwrap();
    assert_eq!(
        app.hw_offer()
            .unwrap()
            .get(Codec::H264)
            .and_then(|c| c.device.clone()),
        Some("/dev/dri/renderD128".into())
    );
    app.dispatch(Command::SetGraphicsCard(None), Instant::now())
        .unwrap();
    assert_eq!(
        app.hw_offer().unwrap().get(Codec::H264).map(|c| c.vendor),
        Some(Vendor::Nvenc)
    );
    assert!(
        read_settings(&dir.path().join("settings.json"))
            .graphics_card
            .is_none()
    );
    // a device that went away (a stale setting) gives the normal method, never another one
    app.settings.graphics_card = Some("VA-API (renderD777)".into());
    assert!(app.hw_offer().is_none());
}

/// The Faster explanation names integrated graphics (REQ-207); dedicated graphics do not.
// covers: REQ-207
#[test]
fn faster_says_when_the_graphics_are_part_of_the_processor() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = app_in(
        dir.path(),
        World {
            working: working(&["h264_qsv"]),
            hw_export: ok("progress=end\n"),
        },
    );
    assert_eq!(crate::ui::settings::integrated_note(&app), None);
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    checked(&mut app);
    assert_eq!(
        crate::ui::settings::integrated_note(&app),
        Some("On this computer Faster may not be quicker than Best quality.")
    );
    // dedicated graphics: not said
    let (mut app, _) = app_in(
        dir.path(),
        World {
            working: working(&["h264_nvenc"]),
            hw_export: ok("progress=end\n"),
        },
    );
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    checked(&mut app);
    assert_eq!(crate::ui::settings::integrated_note(&app), None);
}

/// An old system video library: Advanced says which version is installed and which is
/// needed, in plain words.
// covers: REQ-217
#[test]
fn advanced_names_both_libva_versions() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = app_in(
        dir.path(),
        World {
            working: working(&[]),
            hw_export: ok("progress=end\n"),
        },
    );
    app.hw.probe = HwProbe::Done(HwStatus {
        libva_note: Some(crate::hwenc::libva_message(Some((2, 20)))),
        ..Default::default()
    });
    let lines = crate::ui::settings::hardware_lines(&app);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("libva 2.20") && l.contains("libva 2.21 or newer")),
        "{lines:?}"
    );
    assert!(app.hw_offer().is_none(), "the normal method is used");
}

/// A fake whose `-hwaccel` decodes work for `vaapi` (and the preview run with it ends badly
/// when `breaks`), after the normal one of `world`.
fn decoding_world(breaks: bool) -> FakeRunner {
    let base = fake_runner(World {
        working: working(&["h264_vaapi"]),
        hw_export: ok("progress=end\n"),
    });
    FakeRunner::default().on(move |prog, a| {
        if a.iter().any(|x| x == "matroska") {
            return ok("");
        }
        if let Some(i) = a.iter().position(|x| x == "-hwaccel") {
            if a.iter().any(|x| x == "rawvideo") {
                // the preview run
                return if breaks {
                    fail("decoder error")
                } else {
                    ok("")
                };
            }
            // the decoder probe
            return (a[i + 1] == "vaapi").then(|| CmdOutput {
                success: true,
                stdout: b"frame=15\nprogress=end\n".to_vec(),
                stderr: String::new(),
            });
        }
        base.responders[0](prog, a)
    })
}

fn one_card() -> Platform {
    Platform {
        os: Os::Linux,
        render_nodes: vec!["/dev/dri/renderD128".into()],
        system_libva_new: Some(true),
        libva_version: Some((2, 21)),
    }
}

fn preview_runs(f: &FakeRunner) -> Vec<String> {
    f.calls
        .lock()
        .unwrap()
        .iter()
        .filter(|a| a.iter().any(|x| x == "rawvideo") && a.iter().any(|x| x == "-i"))
        .map(|a| a.join(" "))
        .collect()
}

/// With Faster the preview decodes with the hardware decoder the check found; with Best
/// quality it never does.
// covers: REQ-214
#[test]
fn the_preview_decodes_with_the_hardware_only_with_faster() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, f) = app_with(dir.path(), Arc::new(decoding_world(false)));
    app.platform = one_card;
    timeline(&mut app, dir.path());
    let now = Instant::now();
    // Best quality: no hardware decoding, whatever the computer has
    app.dispatch(Command::PlayPause, now).unwrap();
    assert!(preview_runs(&f).iter().all(|l| !l.contains("-hwaccel")));
    app.dispatch(Command::PlayPause, now).unwrap();
    assert!(!app.preview.uses_hw_decoding());
    // Faster: the check finds the decoder (and the encoder), the next playback uses it
    app.dispatch(Command::SetExport(ExportChoice::Faster), now)
        .unwrap();
    checked(&mut app);
    assert_eq!(app.hw_decoders[&Codec::H264].method(), "vaapi");
    assert!(app.preview.uses_hw_decoding());
    let before = preview_runs(&f).len();
    app.dispatch(Command::PlayPause, now).unwrap();
    let runs = preview_runs(&f);
    assert!(
        runs[before..]
            .iter()
            .all(|l| l.contains("-hwaccel vaapi -hwaccel_device /dev/dri/renderD128 -i")),
        "{runs:?}"
    );
    app.dispatch(Command::PlayPause, now).unwrap();
    // back to Best quality: no hardware decoding again
    app.dispatch(Command::SetExport(ExportChoice::BestQuality), now)
        .unwrap();
    assert!(app.hw_decoders.is_empty() && !app.preview.uses_hw_decoding());
    let before = preview_runs(&f).len();
    app.dispatch(Command::PlayPause, now).unwrap();
    assert!(
        preview_runs(&f)[before..]
            .iter()
            .all(|l| !l.contains("-hwaccel"))
    );
}

/// A decoder that fails during playback: the preview goes on at the playhead with the normal
/// decoder, without a message.
// covers: REQ-214
#[test]
fn a_failing_hardware_decoder_falls_back_without_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, f) = app_with(dir.path(), Arc::new(decoding_world(true)));
    app.platform = one_card;
    timeline(&mut app, dir.path());
    let now = Instant::now();
    app.dispatch(Command::SetExport(ExportChoice::Faster), now)
        .unwrap();
    checked(&mut app);
    // a picture is on screen
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 2,
        height: 2,
        rgba: vec![255; 16],
    });
    app.dispatch(Command::PlayPause, now).unwrap();
    // long under load: the fallback is waited for, not raced
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.preview.hw_fallbacks == 0 {
        app.tick(Instant::now());
        assert!(Instant::now() < deadline, "no fallback");
        std::thread::sleep(Duration::from_millis(2));
    }
    // the second run is the normal decoder, started at the playhead
    for _ in 0..50 {
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(2));
    }
    let runs = preview_runs(&f);
    assert!(runs[0].contains("-hwaccel"));
    assert!(runs.len() >= 2 && !runs[1].contains("-hwaccel"), "{runs:?}");
    assert!(app.preview.error.is_none(), "no message");
    assert!(app.notice.is_none());
    assert!(app.hw_decoders.is_empty());
    // the picture stayed
    assert!(app.frame.is_some());
}
