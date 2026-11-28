// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! A project imported as media, from the command to the rendered item and back after a
//! reopen, with a scripted ffmpeg.

use super::tests::{FakeDialogs, state_with};
use super::*;
use crate::ffmpeg::fake::{FakeRunner, fail, ok};
use crate::nested::tests::write_project;
use std::time::Duration;

/// The rendered video, as ffprobe describes it.
const RENDERED: &str = r#"{"streams":[
    {"codec_type":"video","codec_name":"h264","width":1280,"height":720,
     "avg_frame_rate":"30/1","r_frame_rate":"30/1"},
    {"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}],
    "format":{"format_name":"matroska,webm","duration":"100.0"}}"#;

/// ffmpeg writes its output file (unless `broken`); ffprobe reads what was written.
pub(crate) fn runner(broken: bool) -> FakeRunner {
    FakeRunner::default().on(move |prog, a| {
        let last = a.last()?.clone();
        let file = last.strip_prefix("file:").unwrap_or(&last);
        if prog.ends_with("ffprobe") {
            if a.iter().any(|x| x == "nokey") {
                return ok("0.0\n");
            }
            return match std::fs::metadata(file) {
                Ok(_) => ok(RENDERED),
                Err(_) => fail("no such file"),
            };
        }
        if broken {
            return fail("encoder broke");
        }
        if last.starts_with("file:") {
            std::fs::write(file, b"video").ok()?;
        }
        ok("out_time_us=2500000\nprogress=end\n")
    })
}

fn app_in(dir: &Path, dialogs: FakeDialogs, runner: FakeRunner) -> AppState {
    let mut app = state_with(dialogs, runner);
    app.work_root = dir.join("work");
    app
}

fn wait_imports(app: &mut AppState) {
    let end = Instant::now() + Duration::from_secs(20);
    while app.imports.busy() {
        app.tick(Instant::now());
        assert!(Instant::now() < end, "the import did not end");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn run(app: &mut AppState, c: Command) -> Result<(), String> {
    app.dispatch(c, Instant::now())
}

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// A source project with three clips of 4 s.
fn three_clips(dir: &Path) -> PathBuf {
    let path = write_project(dir, "wedding", &[], true);
    let (mut p, _) = project::load(&path).unwrap();
    let m = p.media[0].clone();
    p.timeline.append(&m).unwrap();
    p.timeline.append(&m).unwrap();
    project::save(&p, &path).unwrap();
    path
}

// covers: REQ-372, REQ-371
#[test]
fn every_import_asks_first_and_cancel_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let source = three_clips(dir.path());
    let dialogs = FakeDialogs {
        open: Some(source.clone()),
        ..Default::default()
    };
    let mut app = app_in(dir.path(), dialogs, runner(false));
    assert!(app.is_enabled(&Command::PickImportProject));
    assert!(!app.is_enabled(&Command::ConfirmImportProject));
    run(&mut app, Command::PickImportProject).unwrap();
    assert_eq!(app.modal, Some(Modal::ImportProject(source.clone())));
    assert_eq!(
        imports::IMPORT_PROJECT_QUESTION,
        "If someone changes the project, this project may look different when it is reloaded."
    );
    run(&mut app, Command::CloseModal).unwrap();
    assert!(app.project.media.is_empty() && !app.dirty && !app.imports.busy());
    // asked again, and OK adds the item
    run(&mut app, Command::ImportProject(source.clone())).unwrap();
    assert!(matches!(app.modal, Some(Modal::ImportProject(_))));
    run(&mut app, Command::ConfirmImportProject).unwrap();
    assert_eq!(app.modal, None);
    assert_eq!(app.project.media.len(), 1);
    let id = app.project.media[0].id;
    // the window is free while the video is made
    assert!(app.busy() && app.imports.pending(id));
    assert!(app.imports.line(id).is_some());
    assert!(app.is_enabled(&Command::CancelImport));
    assert!(
        app.background_jobs()
            .iter()
            .any(|j| j.label.contains("imported project") && j.cancel.is_some())
    );
    wait_imports(&mut app);
    // one media item, of the total length of the source timeline
    let m = &app.project.media[0];
    assert_eq!((m.name.as_str(), m.duration), ("wedding.vlproj", s(12)));
    assert_eq!(m.from.as_deref(), Some(source.as_path()));
    assert!(m.is_ready() && !m.missing && m.has_video());
    assert!(m.path.starts_with(dir.path().join("work/imports")));
    assert!(app.dirty);
    assert_eq!(app.status, "Imported wedding.vlproj");
    assert_eq!(app.imports.line(id), None);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let folder = std::fs::metadata(m.path.parent().unwrap()).unwrap();
        assert_eq!(folder.permissions().mode() & 0o777, 0o700);
    }
}

// covers: REQ-373
#[test]
fn the_imported_item_is_edited_like_any_video_and_cannot_be_the_project_itself() {
    let dir = tempfile::tempdir().unwrap();
    let source = three_clips(dir.path());
    let mut app = app_in(dir.path(), FakeDialogs::default(), runner(false));
    run(&mut app, Command::ImportProject(source.clone())).unwrap();
    run(&mut app, Command::ConfirmImportProject).unwrap();
    let id = app.project.media[0].id;
    wait_imports(&mut app);
    run(&mut app, Command::Append(Some(id))).unwrap();
    run(&mut app, Command::Seek(s(5))).unwrap();
    run(&mut app, Command::Split).unwrap();
    assert_eq!(app.project.timeline.clips.len(), 2);
    run(&mut app, Command::ToggleFade(0)).unwrap();
    let clip = app.project.timeline.clips[1].id;
    run(&mut app, Command::SetSpeed { clip, speed: 2000 }).unwrap();
    run(&mut app, Command::MoveClip { clip, to: 0 }).unwrap();
    // the export reads the rendered video with those edits
    let plan =
        render::plan_export(&app.project, &super::tests::info().encoders, |_, _| true).unwrap();
    let render::Plan::Reencode { graph, .. } = plan else {
        panic!("edited clips are encoded again");
    };
    assert_eq!(graph.inputs.len(), 2);
    // saved, the project cannot import itself, directly or through another
    let me = dir.path().join("me.vlproj");
    run(&mut app, Command::SaveProjectAs(me.clone())).unwrap();
    let e = run(&mut app, Command::ImportProject(me.clone())).unwrap_err();
    assert!(e.contains("cannot import itself"), "{e}");
    assert_eq!(app.notice, Some(e));
    assert_eq!(app.modal, None);
    let through = write_project(dir.path(), "through", &[&me], true);
    app.notice = None;
    let e = run(&mut app, Command::ImportProject(through)).unwrap_err();
    assert!(e.contains("cannot import itself"), "{e}");
    assert_eq!(app.project.media.len(), 1);
}

// covers: REQ-371
#[test]
fn reopening_the_project_follows_the_source_as_it_is_now() {
    let dir = tempfile::tempdir().unwrap();
    let source = three_clips(dir.path());
    let me = dir.path().join("me.vlproj");
    let mut app = app_in(dir.path(), FakeDialogs::default(), runner(false));
    run(&mut app, Command::ImportProject(source.clone())).unwrap();
    run(&mut app, Command::ConfirmImportProject).unwrap();
    let id = app.project.media[0].id;
    wait_imports(&mut app);
    run(&mut app, Command::Append(Some(id))).unwrap();
    run(&mut app, Command::SaveProjectAs(me.clone())).unwrap();
    let first = app.project.media[0].path.clone();
    // unchanged: checked and left as it is, with nothing to save
    run(&mut app, Command::OpenProject(me.clone())).unwrap();
    assert!(app.imports.pending(id));
    wait_imports(&mut app);
    assert_eq!(app.project.media[0].path, first);
    assert!(!app.dirty, "nothing changed");
    // the source project gets a fourth clip
    let (mut p, _) = project::load(&source).unwrap();
    let m = p.media[0].clone();
    p.timeline.append(&m).unwrap();
    project::save(&p, &source).unwrap();
    run(&mut app, Command::OpenProject(me.clone())).unwrap();
    // the export waits for the new video
    let out = dir.path().join("out.mp4");
    let e = run(&mut app, Command::Export(out)).unwrap_err();
    assert!(e.contains("still being rendered"), "{e}");
    wait_imports(&mut app);
    let item = &app.project.media[0];
    assert_ne!(item.path, first);
    assert_eq!(item.duration, s(16));
    assert_eq!(
        app.project.timeline.duration(),
        s(16),
        "the whole item follows"
    );
    assert!(app.dirty);
    assert!(app.status.contains("rendered again"), "{}", app.status);
    // the rendered video is gone (another computer): made again, no "missing" notice
    run(&mut app, Command::SaveProject).unwrap();
    std::fs::remove_dir_all(dir.path().join("work/imports")).unwrap();
    run(&mut app, Command::OpenProject(me.clone())).unwrap();
    assert!(app.project.media[0].missing);
    assert_eq!(app.notice, None);
    wait_imports(&mut app);
    assert!(!app.project.media[0].missing && app.project.media[0].path.is_file());
    // the source project is gone: the item is refused with its name
    std::fs::remove_file(&source).unwrap();
    run(&mut app, Command::OpenProject(me)).unwrap();
    wait_imports(&mut app);
    assert!(app.project.media[0].missing);
    assert!(app.notice.as_deref().unwrap().contains("wedding.vlproj"));
    assert_eq!(
        app.project.media.len(),
        1,
        "the item and its clips are kept"
    );
    assert_eq!(app.project.timeline.clips.len(), 1);
}

// covers: REQ-371
#[test]
fn refusals_failures_and_cancelling_add_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let source = three_clips(dir.path());
    let mut app = app_in(dir.path(), FakeDialogs::default(), runner(true));
    // a cancelled file dialog
    run(&mut app, Command::PickImportProject).unwrap();
    assert_eq!(app.modal, None);
    let gone = dir.path().join("gone.vlproj");
    let e = run(&mut app, Command::ImportProject(gone)).unwrap_err();
    assert!(e.contains("gone.vlproj"), "{e}");
    assert_eq!(app.modal, None);
    // confirming with no question open does nothing
    app.notice = None;
    app.confirm_import_project().unwrap();
    assert!(app.project.media.is_empty());
    // the rendering fails: the item goes and the reason is told
    run(&mut app, Command::ImportProject(source.clone())).unwrap();
    run(&mut app, Command::ConfirmImportProject).unwrap();
    wait_imports(&mut app);
    assert!(app.project.media.is_empty());
    assert!(
        app.notice
            .as_deref()
            .unwrap()
            .contains("could not be rendered")
    );
    // cancel, with two waiting
    app.notice = None;
    for _ in 0..2 {
        run(&mut app, Command::ImportProject(source.clone())).unwrap();
        run(&mut app, Command::ConfirmImportProject).unwrap();
    }
    run(&mut app, Command::CancelImport).unwrap();
    assert!(app.project.media.is_empty() && !app.imports.busy());
    // removing the item stops its rendering
    run(&mut app, Command::ImportProject(source)).unwrap();
    run(&mut app, Command::ConfirmImportProject).unwrap();
    let id = app.project.media[0].id;
    run(&mut app, Command::RemoveMedia(id)).unwrap();
    assert!(app.project.media.is_empty() && !app.imports.busy());
}

#[test]
fn an_import_waits_for_ffmpeg_to_be_ready() {
    let dir = tempfile::tempdir().unwrap();
    let source = three_clips(dir.path());
    let mut app = app_in(dir.path(), FakeDialogs::default(), runner(false));
    let info = std::mem::replace(&mut app.ffmpeg, Err(CHECKING_FFMPEG.into()));
    assert!(!app.is_enabled(&Command::PickImportProject));
    assert!(run(&mut app, Command::ImportProject(source.clone())).is_err());
    // a project opened before the check ended is rebuilt after it
    let mut p = Project::default();
    let mut item = crate::timeline::tests::media(1, 12, true);
    item.from = Some(source);
    p.add_media(item);
    app.load_project(p, None, Instant::now());
    app.tick(Instant::now());
    assert_eq!(
        app.imports.line(MediaId(1)).as_deref(),
        Some("waiting to be rendered")
    );
    app.ffmpeg = info;
    wait_imports(&mut app);
    assert_eq!(app.project.media[0].duration, s(12));
}

// covers: REQ-370
#[test]
fn the_strip_setting_is_on_by_default_stops_the_snapshots_and_is_remembered() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut app = app_in(
        dir.path(),
        FakeDialogs::default(),
        crate::strips::tests::pictures(usize::MAX),
    );
    app.settings_path = Some(path.clone());
    assert!(app.settings.show_picture_strips);
    let want = crate::strips::Want {
        key: crate::strips::Key {
            media: MediaId(1),
            at: Micros::ZERO,
            width: 36,
            height: 20,
        },
        step: s(1),
        path: "/m/1.mp4".into(),
    };
    run(&mut app, Command::SetPictureStrips(false)).unwrap();
    app.strips.want(want.clone());
    app.tick(Instant::now());
    assert_eq!(app.strips.started, 0, "no snapshot with the switch off");
    // a restart keeps the choice
    let (again, _) = start_settings(Some(&path), &test_machine());
    assert!(!again.show_picture_strips);
    run(&mut app, Command::SetPictureStrips(true)).unwrap();
    app.strips.want(want.clone());
    let end = Instant::now() + Duration::from_secs(10);
    while app.strips.get(&want.key).is_none() {
        app.tick(Instant::now());
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(2));
    }
    // a file without the key, or with a wrong one, has the strips on
    for text in ["null", "\"no\""] {
        let json = std::fs::read_to_string(&path).unwrap().replace(
            "\"show_picture_strips\": true",
            &format!("\"show_picture_strips\": {text}"),
        );
        std::fs::write(&path, json).unwrap();
        assert!(
            start_settings(Some(&path), &test_machine())
                .0
                .show_picture_strips
        );
    }
}
