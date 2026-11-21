// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 2.0.4, batch 6: autosave (REQ-189), recovery (REQ-190) and the undo history when
//! a media file is removed (REQ-196).

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::autosave as store;
use std::time::Duration;

/// An app with a saved project (a.mp4 and b.mp4 on the timeline) and an autosave folder.
fn saved_app(dir: &Path) -> (AppState, PathBuf, PathBuf) {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let now = Instant::now();
    for i in 0..2 {
        let id = app.project.media[i].id;
        app.dispatch(Command::Append(Some(id)), now).unwrap();
    }
    let proj = dir.join("p.vlproj");
    app.dispatch(Command::SaveProjectAs(proj.clone()), now)
        .unwrap();
    let cache = dir.join("autosave");
    app.autosave_dir = Some(cache.clone());
    (app, proj, cache)
}

fn checksum(p: &Path) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(std::fs::read(p).unwrap()).to_vec()
}

fn two_minutes() -> Duration {
    store::INTERVAL
}

// covers: REQ-189
#[test]
fn an_edited_project_is_copied_to_the_cache_after_two_minutes_and_the_copy_goes_on_save() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, proj, cache) = saved_app(dir.path());
    let before = checksum(&proj);
    let t0 = Instant::now();
    // nothing unsaved, nothing written
    app.tick(t0 + two_minutes() * 2);
    app.finish_autosave();
    assert!(!cache.exists() || store::find(&cache).is_empty());
    // an edit, and two minutes later the cache holds a copy with the edit
    app.dispatch(Command::Split, t0).unwrap_or(());
    app.dispatch(Command::ToggleFade(0), t0).unwrap();
    assert!(app.dirty);
    app.tick(t0 + Duration::from_secs(10));
    assert!(app.autosave_run.is_none(), "not yet");
    let started = Instant::now();
    app.tick(t0 + two_minutes() + Duration::from_secs(10));
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the frame does not wait for the disk"
    );
    app.finish_autosave();
    let copies = store::find(&cache);
    assert_eq!(copies.len(), 1);
    let (copy, _) = store::load(&copies[0]).unwrap();
    assert!(copy.timeline.has_fade(0), "the copy has the edit");
    assert_eq!(checksum(&proj), before, "the project file is untouched");
    // at most one copy per interval
    app.tick(t0 + two_minutes() + Duration::from_secs(20));
    assert!(app.autosave_run.is_none());
    // after Save the copy is gone
    app.dispatch(Command::SaveProject, t0).unwrap();
    assert!(store::find(&cache).is_empty());
    assert!(
        std::fs::read_dir(&cache).map(|r| r.count()).unwrap_or(0) == 0,
        "both files of the copy are removed"
    );
}

// covers: REQ-189
#[test]
fn an_untitled_project_gets_a_session_copy_and_discarding_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let cache = dir.path().join("cache");
    app.autosave_dir = Some(cache.clone());
    let t0 = Instant::now();
    let id = app.project.media[0].id;
    app.dispatch(Command::Append(Some(id)), t0).unwrap();
    app.tick(t0 + two_minutes());
    app.finish_autosave();
    let found = store::find(&cache);
    assert_eq!(found.len(), 1);
    assert!(found[0].original.is_none());
    // New project, Discard: the copy of the open project is deleted
    app.dispatch(Command::NewProject, t0).unwrap();
    assert!(app.modal.is_some());
    app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Discard), t0)
        .unwrap();
    assert!(store::find(&cache).is_empty());
}

// covers: REQ-190
#[test]
fn after_a_crash_the_next_start_offers_to_recover_discard_or_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, proj, cache) = saved_app(dir.path());
    let t0 = Instant::now();
    app.dispatch(Command::ToggleFade(0), t0).unwrap();
    app.tick(t0 + two_minutes());
    app.finish_autosave();
    // the process "dies": a new one starts with the same folder
    let mut next = state(FakeDialogs::default());
    next.autosave_dir = Some(cache.clone());
    next.find_recoveries();
    assert_eq!(next.modal, Some(Modal::Recover));
    assert!(next.recovery_text().unwrap().contains("p.vlproj"));
    // Cancel: nothing opens and the copy stays
    next.dispatch(Command::CancelRecovery, t0).unwrap();
    assert!(next.modal.is_none() && next.project.timeline.clips.is_empty());
    assert_eq!(store::find(&cache).len(), 1);
    // Recover: the edit is back with the unsaved marker, the saved project is the old one
    next.find_recoveries();
    next.dispatch(Command::RecoverAutosave, t0).unwrap();
    assert!(next.modal.is_none());
    assert!(next.project.timeline.has_fade(0), "the edit is restored");
    assert!(next.dirty, "with the unsaved marker");
    assert_eq!(next.project_path.as_deref(), Some(proj.as_path()));
    // saving writes the project and removes the copy
    next.dispatch(Command::SaveProject, t0).unwrap();
    assert!(!next.dirty && store::find(&cache).is_empty());
    assert!(project::load(&proj).unwrap().0.timeline.has_fade(0));
    // Discard deletes the copy
    app.dispatch(
        Command::SetEdgeFade {
            edge: Edge::Start,
            on: true,
        },
        t0,
    )
    .unwrap();
    app.tick(t0 + two_minutes() * 3);
    app.finish_autosave();
    let mut third = state(FakeDialogs::default());
    third.autosave_dir = Some(cache.clone());
    third.find_recoveries();
    assert_eq!(third.modal, Some(Modal::Recover));
    third.dispatch(Command::DiscardAutosave, t0).unwrap();
    assert!(third.modal.is_none() && store::find(&cache).is_empty());
}

// covers: REQ-190
#[test]
fn a_clean_exit_leaves_no_copy_and_so_no_offer() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _proj, cache) = saved_app(dir.path());
    let t0 = Instant::now();
    app.dispatch(Command::ToggleFade(0), t0).unwrap();
    app.tick(t0 + two_minutes());
    app.finish_autosave();
    assert_eq!(store::find(&cache).len(), 1);
    // undo back to the saved state, then quit: a clean exit
    app.dispatch(Command::Undo, t0).unwrap();
    assert!(!app.dirty);
    app.dispatch(Command::Quit, t0).unwrap();
    assert!(app.quit_requested);
    let mut next = state(FakeDialogs::default());
    next.autosave_dir = Some(cache);
    next.find_recoveries();
    assert!(next.modal.is_none(), "nothing to offer after a clean exit");
    // a copy that cannot be read is reported, not opened
    let mut third = state(FakeDialogs::default());
    third.recoveries = vec![store::Copy {
        stem: "x".into(),
        file: dir.path().join("none"),
        original: None,
        modified: std::time::SystemTime::now(),
    }];
    assert!(third.dispatch(Command::RecoverAutosave, t0).is_err());
    assert!(
        third.dispatch(Command::RecoverAutosave, t0).is_ok(),
        "nothing left to recover"
    );
    assert!(third.recovery_text().is_none());
}

// covers: REQ-196
#[test]
fn removing_an_unused_file_keeps_every_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    let a = app.project.media[0].id;
    let b = app.project.media[1].id;
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    for _ in 0..10 {
        app.dispatch(Command::Append(Some(a)), now).unwrap();
    }
    assert_eq!(app.history.undo_len(), 11);
    // b is not on the timeline: removing it keeps the ten steps (and the eleventh)
    app.dispatch(Command::RemoveMedia(b), now).unwrap();
    assert_eq!(app.history.undo_len(), 11);
    for _ in 0..11 {
        app.dispatch(Command::Undo, now).unwrap();
    }
    assert!(app.project.timeline.clips.is_empty());
}

// covers: REQ-196
#[test]
fn only_the_steps_that_refer_to_a_removed_file_are_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    app.dispatch(Command::Append(Some(b)), now).unwrap();
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    // b leaves the timeline again (Delete of the second clip)
    let second = app.project.timeline.clips[1].id;
    app.dispatch(Command::SelectClip(Some(second)), now)
        .unwrap();
    app.dispatch(Command::Delete, now).unwrap();
    assert_eq!(app.history.undo_len(), 4);
    app.dispatch(Command::RemoveMedia(b), now).unwrap();
    // the states that had a clip of b (before "Delete" and before the last append) are gone
    assert!(app.history.undo_len() < 4);
    let mut left = 0;
    while app.history.can_undo() {
        app.dispatch(Command::Undo, now).unwrap();
        left += 1;
        assert!(
            app.project.timeline.clips.iter().all(|c| c.media != b),
            "no undo step brings back a clip of the removed file"
        );
    }
    assert!(left >= 1);
}
