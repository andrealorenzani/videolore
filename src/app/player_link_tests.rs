// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The editor's side of the link with videolore player (REQ-249, REQ-250).

use super::tests::{FakeDialogs, files, settle, state};
use super::*;

fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

// covers: REQ-249
#[test]
fn open_in_player_needs_a_selected_video_and_explains_a_missing_player() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    assert!(!app.is_enabled(&Command::OpenInPlayer), "nothing selected");
    assert!(app.disabled_reason(&Command::OpenInPlayer).is_some());
    assert!(app.dispatch(Command::OpenInPlayer, now).is_err());
    app.dispatch(Command::Import(files(dir.path())), now)
        .unwrap();
    settle(&mut app);
    let before = app.project.doc();
    let folder = listing(dir.path());
    app.dispatch(Command::SelectMedia(Some(app.project.media[0].id)), now)
        .unwrap();
    assert!(app.is_enabled(&Command::OpenInPlayer));
    // the test program has no sibling player: the message says so, nothing else happens
    let e = app.dispatch(Command::OpenInPlayer, now).unwrap_err();
    assert!(
        e.contains("videolore player is not installed next to this program"),
        "{e}"
    );
    assert_eq!(app.project.doc(), before, "the project is unchanged");
    assert_eq!(listing(dir.path()), folder);
}

// covers: REQ-250
#[test]
fn a_video_given_to_the_editor_starts_an_unsaved_project_and_writes_nothing_next_to_it() {
    let dir = tempfile::tempdir().unwrap();
    let video = files(dir.path())[0].clone();
    let folder = listing(dir.path());
    let mut app = state(FakeDialogs::default());
    app.dispatch(Command::OpenProject(video.clone()), Instant::now())
        .unwrap();
    settle(&mut app);
    assert_eq!(app.project.media.len(), 1);
    assert_eq!(app.project.media[0].path, video);
    assert!(app.project_path.is_none(), "an unsaved project");
    assert_eq!(
        listing(dir.path()),
        folder,
        "nothing is written next to the video"
    );
    // a project file still opens as a project (no media is imported for it)
    assert!(crate::project::is_project_path(Path::new("/x/a.VLPROJ")));
    assert!(!crate::project::is_project_path(&video));
}
