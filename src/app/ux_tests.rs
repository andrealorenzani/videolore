// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the 1.1 commands behind the window: undo names, disabled reasons, effect and
//! marker popups, "Play this line", recent projects, drag and drop, the status bar jobs
//! and the saved layout.

use super::tests::{FakeDialogs, files, import, read_settings, runner, settle, state, state_with};
use super::*;
use crate::ffmpeg::fake::FakeRunner;
use crate::preview::NullSink;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips on the timeline (sources in `dir`).
fn two_clips(dir: &Path) -> AppState {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app
}

fn ids(app: &AppState) -> Vec<ClipId> {
    app.project.timeline.clips.iter().map(|c| c.id).collect()
}

// covers: REQ-074
#[test]
fn undo_and_redo_name_the_action() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = two_clips(dir.path());
    let now = Instant::now();
    assert_eq!(app.undo_text(), "Undo Add to timeline");
    assert_eq!(app.redo_text(), "Redo");
    app.dispatch(Command::Seek(s(2)), now).unwrap();
    app.dispatch(Command::Split, now).unwrap();
    assert_eq!(app.undo_text(), "Undo Split");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.redo_text(), "Redo Split");
    assert_eq!(app.undo_text(), "Undo Add to timeline");
    app.dispatch(Command::Redo, now).unwrap();
    assert_eq!(app.undo_text(), "Undo Split");
    let c = ids(&app);
    let (l, r) = (c[1], c[2]);
    app.dispatch(
        Command::SetFade {
            left: l,
            right: r,
            on: true,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Add fade");
    app.dispatch(
        Command::SetFade {
            left: l,
            right: r,
            on: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Remove fade");
    app.dispatch(Command::ToggleFadeAtPlayhead, now).unwrap();
    assert_eq!(app.undo_text(), "Undo Add fade");
    app.dispatch(Command::ToggleFadeAtPlayhead, now).unwrap();
    assert_eq!(app.undo_text(), "Undo Remove fade");
    app.dispatch(Command::AddSubtitle, now).unwrap();
    assert_eq!(app.undo_text(), "Undo Add subtitle line");
    let id = app.selected_line.unwrap();
    app.dispatch(Command::SetSubtitleText(id, "Hi".into()), now)
        .unwrap();
    assert_eq!(app.undo_text(), "Undo Edit subtitle text");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.redo_text(), "Redo Edit subtitle text");
    // every editing command has a name
    for c in [
        Command::Cut,
        Command::Delete,
        Command::Paste,
        Command::Merge,
        Command::MoveLeft,
        Command::MoveRight,
        Command::RemoveAllFades,
        Command::PlaceMusic(None),
        Command::SplitMusic,
        Command::DeleteMusic,
        Command::SetMix(Mix::default()),
        Command::SetPreset(Preset::Boxed),
        Command::SetLanguage(None),
        Command::DeleteSubtitle(id),
    ] {
        assert_ne!(c.undo_name(), "Edit", "{c:?}");
    }
}

// covers: REQ-072
#[test]
fn greyed_out_commands_say_why() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let reason = |app: &AppState, c: Command| app.disabled_reason(&c).unwrap_or("");
    assert_eq!(reason(&app, Command::Undo), "Nothing to undo");
    assert_eq!(reason(&app, Command::Redo), "Nothing to redo");
    assert_eq!(
        reason(&app, Command::Split),
        "Move the playhead over a clip first"
    );
    assert_eq!(reason(&app, Command::Merge), "Select a clip first");
    assert_eq!(reason(&app, Command::Paste), "Copy or cut a clip first");
    assert_eq!(
        reason(&app, Command::PickExport),
        "Add clips to the timeline first"
    );
    assert!(reason(&app, Command::PlaceMusic(None)).contains("audio file"));
    assert!(reason(&app, Command::Append(None)).contains("video"));
    let gen_reason = reason(&app, Command::GenerateSubtitles);
    if crate::stt::available() {
        assert_eq!(gen_reason, "Add clips to the timeline first");
    } else {
        assert!(gen_reason.contains("not available"));
    }
    // enabled commands give no reason
    assert_eq!(app.disabled_reason(&Command::PickImport), None);
    // with clips: Merge needs two parts of one file; an export already runs
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    app.dispatch(Command::Append(Some(app.project.media[1].id)), now)
        .unwrap();
    app.dispatch(Command::SelectClip(Some(ids(&app)[0])), now)
        .unwrap();
    assert!(reason(&app, Command::Merge).contains("same clip"));
    assert_eq!(reason(&app, Command::MoveLeft), "Already the first clip");
    app.export_status.running = true;
    assert_eq!(
        reason(&app, Command::PickExport),
        "An export is already running"
    );
    app.export_status.running = false;
    app.ffmpeg = Err("missing".into());
    assert_eq!(reason(&app, Command::PickImport), "ffmpeg is not ready");
    assert_eq!(reason(&app, Command::ClearRecent), "The list is empty");
    assert_eq!(reason(&app, Command::RemoveAllFades), "There are no fades");
}

// covers: REQ-076, REQ-077
#[test]
fn fades_are_opened_removed_and_restored_from_their_popups() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = two_clips(dir.path());
    let now = Instant::now();
    // four clips, three boundaries, three fades
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    let c = ids(&app);
    for b in 0..3 {
        app.dispatch(
            Command::SetFade {
                left: c[b],
                right: c[b + 1],
                on: true,
            },
            now,
        )
        .unwrap();
    }
    assert_eq!(app.project.timeline.fades.len(), 3);
    // adding a fade twice changes nothing; clips that are not neighbours have no boundary
    let undo = app.history.undo_len();
    app.dispatch(
        Command::SetFade {
            left: c[0],
            right: c[1],
            on: true,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo);
    let far = Command::SetFade {
        left: c[0],
        right: c[2],
        on: true,
    };
    assert!(!app.is_enabled(&far));
    assert!(app.dispatch(far, now).is_err());
    // "Remove all fades" is one undo step
    app.dispatch(Command::RemoveAllFades, now).unwrap();
    assert!(app.project.timeline.fades.is_empty());
    assert_eq!(app.undo_text(), "Undo Remove all fades");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.timeline.fades.len(), 3);
    app.dispatch(Command::RemoveAllFades, now).unwrap();
    app.dispatch(Command::Undo, now).unwrap();
    // nothing to remove: no undo step
    let mut empty = two_clips(dir.path());
    let n = empty.history.undo_len();
    empty.dispatch(Command::RemoveAllFades, now).unwrap();
    assert_eq!(empty.history.undo_len(), n);
    // a fade's popup: playhead to the fade, instance shown
    let fade = Instance::Fade(c[1], c[2]);
    app.dispatch(Command::OpenInstance(fade), now).unwrap();
    assert_eq!(app.playhead, s(10));
    assert_eq!(app.popups.instance, Some(fade));
    assert!(app.popups.is_open(Popup::Instance));
    // Remove: the fade (and its marker) is gone; undo brings it back
    app.dispatch(
        Command::SetFade {
            left: c[1],
            right: c[2],
            on: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.instance_start(fade), None);
    assert!(!app.is_enabled(&Command::OpenInstance(fade)));
    assert!(app.dispatch(Command::OpenInstance(fade), now).is_err());
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.instance_start(fade), Some(s(10)));
    // the chooser of overlapping markers
    app.dispatch(Command::ChooseInstance(vec![fade, fade]), now)
        .unwrap();
    assert_eq!(app.popups.chooser.as_ref().map(Vec::len), Some(2));
    app.dispatch(Command::OpenInstance(fade), now).unwrap();
    assert_eq!(app.popups.chooser, None, "choosing closes it");
    app.dispatch(Command::ChooseInstance(vec![fade]), now)
        .unwrap();
    app.dispatch(Command::ChooseInstance(Vec::new()), now)
        .unwrap();
    assert_eq!(app.popups.chooser, None);
    app.dispatch(Command::ClosePopup(Popup::Instance), now)
        .unwrap();
    assert_eq!(app.popups.instance, None);
}

// covers: REQ-076, REQ-077
#[test]
fn a_subtitle_line_is_edited_from_its_popup_and_undone() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = two_clips(dir.path());
    let now = Instant::now();
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let id = app.selected_line.unwrap();
    let clip = ids(&app)[0];
    app.dispatch(Command::SelectLine(None), now).unwrap();
    app.dispatch(Command::Seek(s(7)), now).unwrap();
    let line = Instance::Line(id, clip);
    app.dispatch(Command::OpenInstance(line), now).unwrap();
    assert_eq!(app.selected_line, Some(id));
    assert_eq!(app.playhead, s(1));
    assert_eq!(app.instance_start(line), Some(s(1)));
    app.dispatch(Command::SetSubtitleText(id, "Edited".into()), now)
        .unwrap();
    assert_eq!(app.project.subtitles.get(id).unwrap().text, "Edited");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.subtitles.get(id).unwrap().text, "New subtitle");
    // the style: one undo step, undo restores it
    let before = app.project.subtitles.preset;
    app.dispatch(Command::SetPreset(Preset::WordHighlight), now)
        .unwrap();
    assert_eq!(app.project.subtitles.preset, Preset::WordHighlight);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.subtitles.preset, before);
    // a line of another clip is not "in" this clip
    assert_eq!(app.instance_start(Instance::Line(id, ids(&app)[1])), None);
    // cutting the clip removes the line from the timeline
    app.dispatch(Command::SelectClip(Some(clip)), now).unwrap();
    app.dispatch(Command::Cut, now).unwrap();
    assert_eq!(app.instance_start(line), None);
    assert!(!app.is_enabled(&Command::PlayLine(id)));
}

// covers: REQ-077, REQ-076
#[test]
fn play_this_line_stops_at_its_end_and_popups_do_not_stop_playback() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state_with(FakeDialogs::default(), runner().holding_streams());
    let t0 = Instant::now();
    import(&mut app, vec![f[0].clone()]);
    app.dispatch(Command::Append(None), t0).unwrap();
    app.dispatch(Command::Seek(s(1)), t0).unwrap();
    app.dispatch(Command::AddSubtitle, t0).unwrap(); // 1 s to 3 s
    let id = app.selected_line.unwrap();
    app.dispatch(Command::Seek(s(4)), t0).unwrap();
    app.dispatch(Command::PlayLine(id), t0).unwrap();
    assert!(app.preview.is_playing());
    assert_eq!(app.playhead, s(1));
    // a popup opened meanwhile does not stop it
    app.dispatch(Command::OpenPopup(Popup::Subtitles), t0)
        .unwrap();
    app.tick(t0 + Duration::from_millis(1000));
    assert!(app.preview.is_playing());
    assert_eq!(app.playhead, s(2));
    app.tick(t0 + Duration::from_millis(2100));
    assert!(!app.preview.is_playing(), "stopped at the end of the line");
    assert_eq!(app.playhead, s(3));
    // Play/Pause afterwards plays normally (no stop at the old line end)
    let t1 = t0 + Duration::from_millis(3000);
    app.dispatch(Command::PlayLine(id), t1).unwrap();
    app.dispatch(Command::PlayPause, t1).unwrap();
    app.dispatch(Command::PlayPause, t1).unwrap();
    app.tick(t1 + Duration::from_millis(2500));
    assert!(app.preview.is_playing());
    app.dispatch(Command::PlayPause, t1 + Duration::from_millis(2500))
        .unwrap();
    assert!(app.dispatch(Command::PlayLine(LineId(999)), t1).is_err());
}

// covers: REQ-080, REQ-028
#[test]
fn recent_projects_are_remembered_checked_and_cleared() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = two_clips(dir.path());
    let settings_file = dir.path().join("settings.json");
    app.settings_path = Some(settings_file.clone());
    let now = Instant::now();
    let p = |i: usize| dir.path().join(format!("p{i}.vlproj"));
    // 12 projects saved and opened: the 10 most recent, most recent first
    for i in 0..12 {
        app.dispatch(Command::SaveProjectAs(p(i)), now).unwrap();
    }
    app.dispatch(Command::OpenProject(p(3)), now).unwrap();
    let mut want: Vec<PathBuf> = vec![p(3)];
    want.extend((2..12).rev().map(p).filter(|x| *x != p(3)));
    want.truncate(10);
    assert_eq!(app.settings.recent, want);
    assert_eq!(read_settings(&settings_file).recent, want, "saved");
    // a file that is gone: message, removed from the list, nothing else changes
    std::fs::remove_file(p(11)).unwrap();
    app.dispatch(Command::OpenRecent(p(11)), now).unwrap();
    assert!(
        app.notice
            .as_deref()
            .unwrap()
            .contains("could not be found")
    );
    assert!(!app.settings.recent.contains(&p(11)));
    assert!(!read_settings(&settings_file).recent.contains(&p(11)));
    assert_eq!(app.project_path, Some(p(3)));
    app.notice = None;
    // unsaved changes: the usual question first
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    app.dispatch(Command::OpenRecent(p(5)), now).unwrap();
    assert_eq!(
        app.modal,
        Some(Modal::Unsaved(PendingAction::Open(Some(p(5)))))
    );
    app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Discard), now)
        .unwrap();
    assert_eq!(app.project_path, Some(p(5)));
    assert_eq!(app.settings.recent[0], p(5));
    // the list never goes in a project file
    let text = std::fs::read_to_string(p(5)).unwrap();
    assert!(!text.contains("recent") && !text.contains("p3.vlproj"));
    // Clear list
    assert!(app.is_enabled(&Command::ClearRecent));
    app.dispatch(Command::ClearRecent, now).unwrap();
    assert!(app.settings.recent.is_empty());
    assert!(read_settings(&settings_file).recent.is_empty());
    assert!(!app.is_enabled(&Command::ClearRecent));
}

// covers: REQ-081
#[test]
fn dropped_files_follow_the_drop_rules() {
    let p = |s: &str| PathBuf::from(s);
    assert_eq!(drop_plan(Vec::new()), DropPlan::Nothing);
    assert_eq!(
        drop_plan(vec![p("/a.mp4"), p("/b.mov")]),
        DropPlan::Import(vec![p("/a.mp4"), p("/b.mov")])
    );
    assert_eq!(
        drop_plan(vec![
            p("/a.mp4"),
            p("/x.vlproj"),
            p("/y.VLPROJ"),
            p("/n.txt")
        ]),
        DropPlan::Open(p("/x.vlproj"), 3)
    );
    assert_eq!(
        drop_plan(vec![p("/y.VLPROJ")]),
        DropPlan::Open(p("/y.VLPROJ"), 0)
    );
    // 3 videos and 1 file that is not media: 3 imported, 1 rejected
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    let mut dropped: Vec<PathBuf> = ["a.mp4", "b.mp4", "c.mp4", "notes.txt"]
        .iter()
        .map(|n| dir.path().join(n))
        .collect();
    for f in &dropped {
        std::fs::write(f, b"x").unwrap();
    }
    app.dispatch(Command::DropFiles(dropped.clone()), now)
        .unwrap();
    settle(&mut app);
    let ready = app.project.media.iter().filter(|m| m.is_ready()).count();
    let rejected = app
        .project
        .media
        .iter()
        .filter(|m| matches!(m.status, MediaStatus::Rejected(_)))
        .count();
    assert_eq!((ready, rejected), (3, 1));
    assert!(
        app.status.contains("could not be imported"),
        "{}",
        app.status
    );
    // a project dropped with unsaved changes: the usual question
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    let proj = dir.path().join("other.vlproj");
    let mut clean = state(FakeDialogs::default());
    clean
        .dispatch(Command::SaveProjectAs(proj.clone()), now)
        .unwrap();
    dropped.insert(0, proj.clone());
    app.dispatch(Command::DropFiles(dropped.clone()), now)
        .unwrap();
    assert_eq!(
        app.modal,
        Some(Modal::Unsaved(PendingAction::Open(Some(proj.clone()))))
    );
    // while a dialog is open, drops are ignored
    let n = app.project.media.len();
    app.dispatch(Command::DropFiles(vec![dir.path().join("a.mp4")]), now)
        .unwrap();
    assert_eq!(app.project.media.len(), n);
    assert!(app.status.contains("Close the open dialog"));
    app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Discard), now)
        .unwrap();
    assert_eq!(app.project_path, Some(proj.clone()));
    // several projects: the first one wins, the rest is ignored with a message
    let second = dir.path().join("second.vlproj");
    clean
        .dispatch(Command::SaveProjectAs(second.clone()), now)
        .unwrap();
    app.dispatch(
        Command::DropFiles(vec![second.clone(), proj.clone(), dir.path().join("a.mp4")]),
        now,
    )
    .unwrap();
    assert_eq!(app.project_path, Some(second));
    assert!(
        app.status.contains("2 other file(s) ignored"),
        "{}",
        app.status
    );
    app.dispatch(Command::DropFiles(Vec::new()), now).unwrap();
}

// covers: REQ-073
#[test]
fn the_status_bar_lists_background_work_with_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = two_clips(dir.path());
    assert!(app.background_jobs().is_empty());
    app.project.media[0].status = MediaStatus::Analysing;
    app.project.media[1].proxy = ProxyState::Generating(40);
    app.project.media[2].proxy = ProxyState::Queued;
    app.model_progress = Some(0.25);
    app.stt_progress = Some(0.5);
    app.export_status = ExportStatus {
        running: true,
        progress: 0.75,
        stream_copy: Some(false),
    };
    let jobs = app.background_jobs();
    let labels: Vec<&str> = jobs.iter().map(|j| j.label.as_str()).collect();
    assert_eq!(
        labels,
        vec![
            "Analysing 1 file(s)",
            "Preview copies: 2",
            "Downloading the speech recognition file",
            "Generating subtitles",
            "Exporting (re-encoding)"
        ]
    );
    assert_eq!(jobs[1].progress, Some(0.2));
    assert_eq!(jobs[4].progress, Some(0.75));
    assert_eq!(jobs[3].cancel, Some(Command::CancelSubtitles));
    assert_eq!(jobs[4].cancel, Some(Command::CancelExport));
    assert_eq!(jobs[0].cancel, None);
    app.export_status.stream_copy = None;
    assert!(app.background_jobs()[4].label.contains("preparing"));
    app.export_status.stream_copy = Some(true);
    assert!(app.background_jobs()[4].label.contains("no re-encoding"));
    app.project.media[0].status = MediaStatus::Ready;
    app.export_status.running = false;
}

// covers: REQ-067
#[test]
fn the_layout_is_a_program_setting_restored_at_start() {
    let dir = tempfile::tempdir().unwrap();
    let settings_file = dir.path().join("settings.json");
    let mut app = two_clips(dir.path());
    app.settings_path = Some(settings_file.clone());
    let now = Instant::now();
    let l = SavedLayout {
        top: 0.55,
        media: 0.2,
        timeline: 0.8,
    };
    app.dispatch(Command::SetLayout(l), now).unwrap();
    assert_eq!(read_settings(&settings_file).layout, Some(l));
    // not valid: refused, nothing saved
    let bad = SavedLayout { top: 1.5, ..l };
    assert!(app.dispatch(Command::SetLayout(bad), now).is_err());
    assert_eq!(read_settings(&settings_file).layout, Some(l));
    // projects never carry or change the layout
    let proj = dir.path().join("p.vlproj");
    app.dispatch(Command::SaveProjectAs(proj.clone()), now)
        .unwrap();
    assert!(!std::fs::read_to_string(&proj).unwrap().contains("layout"));
    app.dispatch(Command::NewProject, now).unwrap();
    app.dispatch(Command::OpenProject(proj), now).unwrap();
    assert_eq!(app.settings.layout, Some(l));
    // the next start restores it
    let again = AppState::new(
        Arc::new(crate::ffmpeg::fake::healthy()),
        Box::new(FakeDialogs::default()),
        Box::new(NullSink),
        Some(settings_file.clone()),
    );
    assert_eq!(again.settings.layout, Some(l));
    // Reset layout: back to the default (nothing saved)
    app.dispatch(Command::ResetLayout, now).unwrap();
    assert_eq!(app.settings.layout, None);
    assert_eq!(read_settings(&settings_file).layout, None);
}

#[test]
fn popups_zoom_and_settings_commands() {
    let mut app = state_with(FakeDialogs::default(), FakeRunner::default());
    let now = Instant::now();
    app.dispatch(Command::OpenPopup(Popup::Fades), now).unwrap();
    app.dispatch(Command::OpenSettings, now).unwrap();
    app.dispatch(Command::OpenPopup(Popup::Fades), now).unwrap();
    assert_eq!(app.popups.open, vec![Popup::Settings, Popup::Fades]);
    app.dispatch(Command::CloseSettings, now).unwrap();
    assert_eq!(app.popups.open, vec![Popup::Fades]);
    assert!(Popup::ALL.iter().all(|p| !p.title().is_empty()));
    app.zoom = 100.0;
    app.dispatch(Command::ZoomBy(ZOOM_STEP), now).unwrap();
    assert!((app.zoom - 141.42).abs() < 0.1);
    app.dispatch(Command::ZoomBy(f32::NAN), now).unwrap();
    app.dispatch(Command::ZoomBy(-1.0), now).unwrap();
    assert!((app.zoom - 141.42).abs() < 0.1);
    app.dispatch(Command::ZoomFit, now).unwrap();
    assert_eq!(app.zoom, 0.0);
}
