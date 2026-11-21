// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the export and preview speed items of 2.0.4 (REQ-220 to REQ-224): the names in
//! the Export dialog, the warning above 4096 px, and the queue behind playback and export.

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::frame::{Fill, Fit, FrameAspect, Shape};
use crate::jobs::Scheduler;
use crate::jobs::tests::CountingRunner;
use crate::ui::export_dialog;

fn tall(fit: Fit) -> Option<FrameAspect> {
    Some(FrameAspect {
        shape: Shape::Tall9x16,
        fit,
        fill: Fill::Blur,
        blur: Default::default(),
    })
}

/// A project with the 4K clip of the test runner (3840x2160) and the 720p one.
fn app_with_clips(dir: &Path) -> AppState {
    let mut app = state(FakeDialogs::default());
    let big = dir.join("big.mov");
    std::fs::write(&big, b"big").unwrap();
    let mut list = files(dir);
    list.insert(0, big);
    import(&mut app, list);
    app
}

fn append(app: &mut AppState, name: &str) {
    let id = app
        .project
        .media
        .iter()
        .find(|m| m.name == name)
        .map(|m| m.id)
        .unwrap();
    app.dispatch(Command::Append(Some(id)), Instant::now())
        .unwrap();
}

// covers: REQ-220
#[test]
fn the_dialog_shows_plain_codec_names_and_the_encoder_in_the_tooltip() {
    let app = state(FakeDialogs::default());
    let mut choice = export::choice::defaults(None, None, None);
    choice.video = Some(0);
    assert_eq!(
        export_dialog::video_text(&choice),
        "1920x1080 25fps, H.264, .mp4"
    );
    choice.video = Some(4);
    assert_eq!(
        export_dialog::video_text(&choice),
        "1920x1080 25fps, HEVC, .mp4"
    );
    let enc = app.encoders();
    for i in crate::format::available_presets(&enc) {
        let p = crate::format::VIDEO_PRESETS[i];
        assert!(!p.label().contains("libx"), "{}", p.label());
        assert!(
            p.tooltip(&enc).starts_with("Encoder: libx"),
            "{}",
            p.label()
        );
    }
}

// covers: REQ-221
#[test]
fn a_frame_larger_than_4096_px_gets_a_warning_before_export() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    append(&mut app, "big.mov");
    let choice = export::choice::defaults(None, None, None);
    // no Change aspect: nothing to warn about
    assert_eq!(export_dialog::over_cap_warning(&app, &choice), None);
    // a 4K source in 9:16 (width fitted): 3840 x 6828
    app.project
        .timeline
        .set_frame_aspect(tall(Fit::Width))
        .unwrap();
    assert_eq!(
        export_dialog::over_cap_warning(&app, &choice),
        Some("This frame is larger than 4096 px: it is scaled down and the export takes longer.")
    );
    // the export still happens, at the capped size
    let out = render::output_geometry(&app.project, &app.encoders()).unwrap();
    assert_eq!(out.width.max(out.height), crate::frame::MAX_SIDE);
    assert_eq!((out.width, out.height), (2304, 4096));
    // only audio and only subtitles have no picture to warn about
    for mode in [export::choice::Mode::AudioOnly, export::choice::Mode::Srt] {
        let mut c = choice.clone();
        c.mode = mode;
        assert_eq!(export_dialog::over_cap_warning(&app, &c), None);
    }
    // the size of a chosen preset is the base: 1920x1080 in 9:16 is 1920x3413, not over
    let mut c = choice.clone();
    c.video = Some(0);
    assert_eq!(export_dialog::over_cap_warning(&app, &c), None);
    // 3840x2160 preset in 9:16: over
    c.video = Some(10);
    assert!(export_dialog::over_cap_warning(&app, &c).is_some());
    // height fitted: 2160 x 3840 stays under the cap
    app.project
        .timeline
        .set_frame_aspect(tall(Fit::Height))
        .unwrap();
    assert_eq!(export_dialog::over_cap_warning(&app, &choice), None);
    // a project without clips has no frame
    let empty = state(FakeDialogs::default());
    assert!(!empty.frame_over_cap(None));
    // the sizes themselves
    let aspect = tall(Fit::Width);
    assert!(!crate::frame::over_cap((1920, 1080), aspect.as_ref()));
    assert!(!crate::frame::over_cap((1600, 1200), aspect.as_ref()));
    assert!(crate::frame::over_cap((3840, 2160), aspect.as_ref()));
    assert!(!crate::frame::over_cap((3840, 2160), None));
    assert_eq!(
        crate::frame::frame_size((1600, 1200), aspect.as_ref()),
        (1600, 2844)
    );
}

// covers: REQ-224
#[test]
fn imports_wait_behind_playback_and_export() {
    // two slots, ten imports queued, each analysis taking a while
    let sched = Scheduler::new(2);
    let runner: Arc<dyn Runner> = Arc::new(CountingRunner::new(Duration::from_millis(60)));
    let pool = probe_pool(&sched, &runner);
    let ffprobe = PathBuf::from("ffprobe");
    let dir = tempfile::tempdir().unwrap();
    for i in 0..10 {
        let path = dir.path().join(format!("{i}.mp4"));
        std::fs::write(&path, b"x").unwrap();
        pool.submit(ProbeJob {
            id: MediaId(i),
            path,
            ffprobe: ffprobe.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
        });
    }
    std::thread::sleep(Duration::from_millis(20));
    assert!(sched.in_use() > 0, "the imports are running");
    // playback asks for its process: it is served at the next free slot, before every
    // import still waiting (the first frame within 200 ms)
    let t = Instant::now();
    let permit = sched.acquire(Priority::Preview, 1, None, None).unwrap();
    let waited = t.elapsed();
    assert!(waited < Duration::from_millis(200), "{waited:?}");
    // an export started meanwhile begins before the imports end
    let t = Instant::now();
    let export = sched.acquire(Priority::Export, 1, None, None).unwrap();
    assert!(t.elapsed() < Duration::from_millis(200));
    assert!(sched.waiting() > 0, "imports are still queued behind them");
    // the imports are the lowest level: even the hardware check (probe level), asked after
    // them, is served before the imports still waiting
    drop((permit, export));
    let check = sched.acquire(Priority::Probe, 1, None, None).unwrap();
    assert!(sched.waiting() > 0, "imports still wait");
    drop(check);
    // the real order is the one the app uses: analysis is the lowest level
    let app = state(FakeDialogs::default());
    assert_eq!(
        app.limited(Priority::Background).priority(),
        Priority::Background
    );
    assert!(Priority::Preview < Priority::Export && Priority::Export < Priority::Probe);
    assert!(Priority::Probe < Priority::Background);
}
