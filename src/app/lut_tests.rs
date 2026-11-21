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

// covers: REQ-289
#[test]
fn the_lut_strength_is_kept_with_the_file_and_reaches_the_render() {
    let dir = tempfile::tempdir().unwrap();
    let lut = dir.path().join("warm.cube");
    std::fs::write(
        &lut,
        "LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n0 0 0\n",
    )
    .unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    let t = FrameTarget::Whole;
    app.dispatch(
        Command::SetLut {
            target: t,
            path: lut,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.timeline.lut_strength_of(t), 100);
    let undo = app.history.undo_len();
    app.dispatch(
        Command::SetLutStrength {
            target: t,
            percent: 50,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(app.project.timeline.lut_strength_of(t), 50);
    let segs = render::segments(&app.project, Micros::ZERO, render::Source::Export).unwrap();
    let full = crate::lut::cached(app.project.timeline.lut_of(t).unwrap()).unwrap();
    assert_ne!(segs[0].lut_clip.as_ref().unwrap().hash(), full.hash());
    // changing the file keeps the strength; a target without a LUT refuses
    assert_eq!(
        serde_json::to_string(&app.project.timeline)
            .unwrap()
            .matches("\"strength\":50")
            .count(),
        1
    );
    app.dispatch(Command::RemoveLut(t), now).unwrap();
    assert!(
        app.dispatch(
            Command::SetLutStrength {
                target: t,
                percent: 10
            },
            now
        )
        .is_err()
    );
}

// covers: REQ-292
#[test]
fn a_missing_lut_at_export_offers_find_export_without_and_cancel() {
    let (dir, mut app, lut) = three();
    let whole = FrameTarget::Whole;
    app.dispatch(
        Command::SetLut {
            target: whole,
            path: lut.clone(),
        },
        now(),
    )
    .unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let choice = crate::export::choice::ExportChoice {
        folder: out.clone(),
        name: "movie".into(),
        ..crate::export::choice::defaults(None, None, Some(&out))
    };
    // the file is there: no question
    assert!(app.missing_luts().is_empty());
    // the file is gone: the question comes, nothing is written
    let kept = std::fs::read(&lut).unwrap();
    std::fs::remove_file(&lut).unwrap();
    app.start_export_choice(choice.clone(), now()).unwrap();
    assert!(matches!(app.modal, Some(Modal::MissingLut(_))));
    assert!(!app.is_exporting() && std::fs::read_dir(&out).unwrap().count() == 0);
    // Cancel: nothing is written and the project keeps its LUT
    app.dispatch(Command::CloseModal, now()).unwrap();
    assert!(app.modal.is_none() && !app.is_exporting());
    assert!(app.project.timeline.any_lut());
    // Export without the LUT: starts, with a warning; the project keeps its LUT file name
    app.start_export_choice(choice.clone(), now()).unwrap();
    app.dispatch(Command::MissingLutExportWithout, now())
        .unwrap();
    assert!(app.is_exporting());
    assert!(app.status.contains("without the LUT"), "{}", app.status);
    assert!(
        app.project.timeline.any_lut(),
        "only this export leaves it out"
    );
    // Find: a chosen file replaces the missing one in the project, and the export starts
    app.dispatch(Command::CancelExport, now()).unwrap();
    let end = Instant::now() + std::time::Duration::from_secs(10);
    while app.is_exporting() && Instant::now() < end {
        app.tick(Instant::now());
    }
    assert!(!app.is_exporting());
    let found = dir.path().join("found.cube");
    std::fs::write(&found, kept).unwrap();
    app.dialogs = Box::new(FakeDialogs {
        lut: Some(found.clone()),
        ..Default::default()
    });
    app.start_export_choice(choice, now()).unwrap();
    app.dispatch(Command::MissingLutFind, now()).unwrap();
    assert!(app.missing_luts().is_empty());
    assert_eq!(
        app.project
            .timeline
            .lut_of(FrameTarget::Clip(clip(&app, 0))),
        Some(text(&found))
    );
    assert!(app.is_exporting());
}

// covers: REQ-293
#[test]
fn the_project_keeps_a_copy_of_its_lut_so_it_works_when_the_file_is_gone() {
    let (dir, mut app, lut) = three();
    let whole = FrameTarget::Whole;
    let plain = dir.path().join("plain.vlproj");
    crate::project::save(&app.project, &plain).unwrap();
    app.dispatch(
        Command::SetLut {
            target: whole,
            path: lut.clone(),
        },
        now(),
    )
    .unwrap();
    let with = dir.path().join("with.vlproj");
    crate::project::save(&app.project, &with).unwrap();
    // the file with the LUT is bigger, and keeps the path as well
    let (a, b) = (
        std::fs::metadata(&plain).unwrap().len(),
        std::fs::metadata(&with).unwrap().len(),
    );
    assert!(b > a, "{b} > {a}");
    let text_of = std::fs::read_to_string(&with).unwrap();
    assert!(text_of.contains("lut_copies") && text_of.contains("warm.cube"));
    // the cube is deleted: the project opens, names nothing missing and the table is there
    std::fs::remove_file(&lut).unwrap();
    let (loaded, _) = crate::project::load(&with).unwrap();
    let path = loaded
        .timeline
        .lut_of(FrameTarget::Clip(loaded.timeline.clips[0].id))
        .unwrap()
        .to_string();
    assert_eq!(path, text(&lut));
    assert!(crate::lut::has_embedded(&path));
    assert_eq!(crate::lut::cached(&path).unwrap().size(), 2);
    let mut other = state(FakeDialogs::default());
    other.project = loaded;
    assert!(other.missing_luts().is_empty());
    let segs = render::segments(&other.project, Micros::ZERO, render::Source::Export).unwrap();
    assert!(segs[0].lut_clip.is_some(), "the export uses the copy");
    // a copy over the limit is not read: only the path is kept
    let mut v: serde_json::Value = serde_json::from_str(&text_of).unwrap();
    v["lut_copies"][0]["path"] = "/gone/huge.cube".into();
    v["lut_copies"][0]["cube"] = format!("LUT_3D_SIZE 2\n{}", "0 0 0\n".repeat(4_000_000)).into();
    let huge = dir.path().join("huge.vlproj");
    std::fs::write(&huge, v.to_string()).unwrap();
    let _ = crate::project::load(&huge);
    assert!(!crate::lut::has_embedded("/gone/huge.cube"));
}
