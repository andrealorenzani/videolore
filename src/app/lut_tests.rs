// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 1.6.0: Apply LUT (REQ-135, REQ-136, REQ-137).

use super::tests::{FakeDialogs, files, import, state};
use super::*;

fn now() -> Instant {
    Instant::now()
}

fn cube(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, crate::lut::tests::identity(2)).unwrap();
    p
}

/// Three clips a, b, a, and a valid LUT file the dialog returns.
fn three() -> (tempfile::TempDir, AppState, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let lut = cube(dir.path(), "warm.cube");
    let mut app = state(FakeDialogs {
        lut: Some(lut.clone()),
        ..Default::default()
    });
    import(&mut app, files(dir.path()));
    for i in [0, 1, 0] {
        let m = app.project.media[i].id;
        app.dispatch(Command::Append(Some(m)), now()).unwrap();
    }
    (dir, app, lut)
}

fn clip(app: &AppState, i: usize) -> ClipId {
    app.project.timeline.clips[i].id
}

fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}

// covers: REQ-135
#[test]
fn choosing_changing_and_removing_a_lut_are_one_undo_step_each() {
    let (dir, mut app, lut) = three();
    let c = FrameTarget::Clip(clip(&app, 1));
    // applying opens the popup; nothing is set until a file is chosen
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::ApplyLut,
            target: Target::Clip(clip(&app, 1)),
        },
        now(),
    )
    .unwrap();
    assert!(!app.project.timeline.any_lut());
    assert_eq!(app.popups.instance, Some(Instance::Lut(c)));
    assert_eq!(app.selected_clip, Some(clip(&app, 1)));
    assert_eq!(
        app.instance_start(Instance::Lut(c)),
        Some(Micros(5_000_000))
    );
    app.dispatch(Command::PickLut(c), now()).unwrap();
    assert_eq!(app.project.timeline.lut_of(c), Some(text(&lut)));
    assert_eq!(app.undo_text(), "Undo Apply LUT");
    // the same file again changes nothing
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetLut {
            target: c,
            path: lut.clone(),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    // another file: "Change LUT"
    let other = cube(dir.path(), "cold.cube");
    app.dispatch(
        Command::SetLut {
            target: c,
            path: other.clone(),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Change LUT");
    assert_eq!(app.project.timeline.lut_of(c), Some(text(&other)));
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.project.timeline.lut_of(c), Some(text(&lut)));
    // removal: one step; the other clips were never touched
    app.dispatch(Command::RemoveLut(c), now()).unwrap();
    assert_eq!(app.undo_text(), "Undo Remove LUT");
    assert!(!app.project.timeline.any_lut());
    assert!(!app.is_enabled(&Command::RemoveLut(c)));
    assert_eq!(
        app.disabled_reason(&Command::RemoveLut(c)),
        Some("There is no LUT to remove")
    );
    // the whole video's LUT, from the Effects row (no target yet): shown for the whole video
    app.dispatch(Command::OpenFrameEffect(EffectKind::ApplyLut), now())
        .unwrap();
    assert_eq!(app.popups.instance, Some(Instance::Lut(FrameTarget::Whole)));
    assert_eq!(app.popup_title(Popup::Instance), "Apply LUT — whole video");
    app.dispatch(Command::PickLut(FrameTarget::Whole), now())
        .unwrap();
    assert_eq!(
        app.project.timeline.lut_of(FrameTarget::Whole),
        Some(text(&lut))
    );
    // a cancelled dialog changes nothing
    let mut none = state(FakeDialogs::default());
    import(&mut none, files(dir.path()));
    let m = none.project.media[0].id;
    none.dispatch(Command::Append(Some(m)), now()).unwrap();
    none.dispatch(Command::PickLut(FrameTarget::Whole), now())
        .unwrap();
    assert!(!none.project.timeline.any_lut());
}

// covers: REQ-136
// covers: REQ-057
#[test]
fn a_file_that_is_not_a_valid_lut_is_refused_with_a_message_and_nothing_changes() {
    let (dir, mut app, _) = three();
    let c = FrameTarget::Clip(clip(&app, 0));
    let bad = dir.path().join("bad.cube");
    std::fs::write(&bad, "this is not a lut").unwrap();
    let before = app.project.edit();
    let e = app
        .dispatch(
            Command::SetLut {
                target: c,
                path: bad,
            },
            now(),
        )
        .unwrap_err();
    assert!(e.contains("bad.cube") && e.contains("not a valid"), "{e}");
    assert_eq!(app.project.edit(), before);
    // a missing file and a folder too
    for p in [dir.path().join("gone.cube"), dir.path().to_path_buf()] {
        assert!(
            app.dispatch(Command::SetLut { target: c, path: p }, now())
                .is_err()
        );
    }
    assert_eq!(app.project.edit(), before);
    // an unknown clip is refused by the command check
    let ghost = FrameTarget::Clip(ClipId(999));
    assert!(!app.is_enabled(&Command::PickLut(ghost)));
    assert!(app.disabled_reason(&Command::PickLut(ghost)).is_some());
}

// covers: REQ-135
#[test]
fn a_lut_goes_with_the_clip_on_split_copy_paste_and_blocks_a_merge_of_different_ones() {
    let (dir, mut app, lut) = three();
    let a = clip(&app, 0);
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Clip(a),
            path: lut.clone(),
        },
        now(),
    )
    .unwrap();
    // copy and paste carry it
    app.dispatch(Command::SelectClip(Some(a)), now()).unwrap();
    app.dispatch(Command::Copy, now()).unwrap();
    app.dispatch(Command::Seek(app.project.timeline.duration()), now())
        .unwrap();
    app.dispatch(Command::Paste, now()).unwrap();
    let pasted = *app.project.timeline.clips.last().unwrap();
    assert_eq!(
        app.project.timeline.lut_of(FrameTarget::Clip(pasted.id)),
        Some(text(&lut))
    );
    // cut and paste too; the cut clip's entry goes with it
    app.dispatch(Command::SelectClip(Some(pasted.id)), now())
        .unwrap();
    app.dispatch(Command::Cut, now()).unwrap();
    assert_eq!(app.project.timeline.clip_luts.len(), 1);
    app.dispatch(Command::Paste, now()).unwrap();
    assert_eq!(app.project.timeline.clip_luts.len(), 2);
    // a split gives both halves the LUT; different files block the merge with the reason
    app.dispatch(Command::SelectClip(Some(a)), now()).unwrap();
    app.dispatch(Command::Seek(Micros(2_000_000)), now())
        .unwrap();
    app.dispatch(Command::Split, now()).unwrap();
    let right = clip(&app, 1);
    assert_eq!(
        app.project.timeline.lut_of(FrameTarget::Clip(right)),
        Some(text(&lut))
    );
    let other = cube(dir.path(), "cold.cube");
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Clip(right),
            path: other,
        },
        now(),
    )
    .unwrap();
    app.dispatch(Command::SelectClip(Some(a)), now()).unwrap();
    assert!(!app.is_enabled(&Command::Merge));
    assert_eq!(
        app.disabled_reason(&Command::Merge),
        Some(crate::timeline::DIFFERENT_LUTS)
    );
}

// covers: REQ-136
// covers: REQ-137
#[test]
fn opening_a_project_with_a_missing_lut_still_opens_and_names_the_file() {
    let (dir, mut app, lut) = three();
    let c = FrameTarget::Clip(clip(&app, 1));
    app.dispatch(
        Command::SetLut {
            target: c,
            path: lut.clone(),
        },
        now(),
    )
    .unwrap();
    assert!(app.missing_luts().is_empty());
    let file = dir.path().join("p.vlproj");
    crate::project::save(&app.project, &file).unwrap();
    std::fs::remove_file(&lut).unwrap();
    assert_eq!(app.missing_luts(), ["warm.cube"]);
    let mut other = state(FakeDialogs::default());
    other.dispatch(Command::OpenRecent(file), now()).unwrap();
    assert_eq!(other.project.timeline.lut_of(c), Some(text(&lut)));
    let notice = other.notice.clone().unwrap();
    assert!(notice.contains("warm.cube"), "{notice}");
}
