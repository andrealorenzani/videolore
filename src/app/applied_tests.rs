// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Applied effects of a clip (REQ-302, REQ-303, REQ-304): the rows, the whole-video marks
//! and moving a row.

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::effect_order::Row;
use crate::text::{TextId, TextItem};

fn now() -> Instant {
    Instant::now()
}

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Three clips (5 s, 5 s, 5 s) and a valid LUT file the dialog returns.
fn three() -> (tempfile::TempDir, AppState, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let lut = dir.path().join("warm.cube");
    std::fs::write(&lut, crate::lut::tests::identity(2)).unwrap();
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

fn names(a: &Applied) -> Vec<&str> {
    a.rows.iter().map(|r| r.name.as_str()).collect()
}

fn add_text(app: &mut AppState, text: &str, at: i64) -> TextId {
    app.dispatch(Command::Seek(s(at)), now()).unwrap();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), text.into(), Micros::ZERO)),
        now(),
    )
    .unwrap();
    app.selection.primary_text().unwrap()
}

fn set_lut(app: &mut AppState, lut: &Path, c: ClipId) {
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Clip(c),
            path: lut.to_path_buf(),
        },
        now(),
    )
    .unwrap();
}

// covers: REQ-302
#[test]
fn the_widget_lists_the_movable_rows_top_first_then_the_fixed_rows() {
    let (_d, mut app, lut) = three();
    assert!(AppState::applied(&app).is_some_and(|a| a.rows.is_empty()));
    let c = clip(&app, 0);
    set_lut(&mut app, &lut, c);
    let t = add_text(&mut app, "Hello", 1);
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    app.dispatch(
        Command::SetSpeed {
            clip: c,
            speed: 2000,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetClipVolume {
            clip: c,
            volume: 50,
            muted: false,
        },
        now(),
    )
    .unwrap();
    let a = app.applied().unwrap();
    assert_eq!(a.clip, c);
    // the text, added after the LUT, is above it; the fixed rows follow in the build order
    assert_eq!(
        names(&a),
        [
            "Text: Hello",
            "Apply LUT: warm.cube",
            "Volume",
            "Change speed: 200%"
        ]
    );
    // the picture group is Text and Apply LUT (REQ-320: every effect moves); Volume is a row
    // of the sound group; only Change speed is fixed (REQ-281)
    assert_eq!(a.movable(), 2);
    assert_eq!(a.rows[0].row, Some(Row::Text(t.0)));
    assert_eq!(a.rows[1].row, Some(Row::Lut));
    assert_eq!(a.rows[2].row, Some(Row::Volume));
    assert_eq!(a.rows[3].row, None);
    // the text row shows its time range
    assert_eq!(a.rows[0].range, Some((s(1), s(4))));
    // a click opens the popup of the effect (and selects the clip)
    assert_eq!(a.rows[0].open, vec![Command::OpenText(t)]);
    assert_eq!(
        a.rows[1].open,
        vec![
            Command::SelectClip(Some(c)),
            Command::OpenInstance(Instance::Lut(FrameTarget::Clip(c)))
        ]
    );
    assert_eq!(
        a.rows[3].open,
        vec![
            Command::SelectClip(Some(c)),
            Command::OpenInstance(Instance::Speed(FrameTarget::Clip(c)))
        ]
    );
    // with no clip selected, the clip at the playhead is shown (the second clip has nothing)
    app.dispatch(Command::SelectClip(None), now()).unwrap();
    app.dispatch(Command::Seek(s(6)), now()).unwrap();
    // clip 1 is 2.5 s long at double speed: 6 s is in the second clip
    assert_eq!(app.applied().unwrap().clip, clip(&app, 1));
    // the text, 1 s to 4 s, still shows over the start of the second clip
    assert_eq!(names(&app.applied().unwrap()), ["Text: Hello"]);
    // no clips: nothing to show
    let empty = state(FakeDialogs::default());
    assert!(empty.applied().is_none());
}

// covers: REQ-302
#[test]
fn long_texts_are_shortened_and_pictures_are_named_by_their_file() {
    let (dir, mut app, _) = three();
    add_text(&mut app, "A very long title that does not fit in a row", 1);
    let png = dir.path().join("logo.png");
    std::fs::write(&png, crate::picture::tests::header(10, 10)).unwrap();
    app.dispatch(Command::Seek(s(1)), now()).unwrap();
    app.dispatch(Command::AddPicture(Some(png)), now()).unwrap();
    let a = app.applied().unwrap();
    // the picture was added last: it is on top of the texts
    assert_eq!(
        names(&a),
        ["Picture: logo.png", "Text: A very long title that d…"]
    );
    assert!(matches!(a.rows[0].row, Some(Row::Picture(_))));
    assert_eq!(app.undo_text(), "Undo Add picture");
}

// covers: REQ-303, REQ-147
#[test]
fn a_whole_video_effect_is_an_ordinary_row_on_each_clip_with_no_mark() {
    let (_d, mut app, lut) = three();
    let ids: Vec<ClipId> = (0..3).map(|i| clip(&app, i)).collect();
    // double-clicks of Scale, Move, Apply LUT and Clean audio give each clip its copy
    for effect in [EffectKind::Scale, EffectKind::Move] {
        app.dispatch(
            Command::ApplyEffect {
                effect,
                target: Target::WholeVideo,
            },
            now(),
        )
        .unwrap();
    }
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Whole,
            path: lut,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetCleanAudio {
            target: Target::WholeVideo,
            on: true,
        },
        now(),
    )
    .unwrap();
    for c in &ids {
        app.dispatch(
            Command::SetClipVolume {
                clip: *c,
                volume: 50,
                muted: false,
            },
            now(),
        )
        .unwrap();
    }
    let rows = |app: &mut AppState, c: ClipId| {
        app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
        app.applied().unwrap().rows
    };
    let before: Vec<_> = ids.iter().map(|c| rows(&mut app, *c)).collect();
    for r in &before {
        let n: Vec<&str> = r.iter().map(|r| r.name.as_str()).collect();
        for want in ["Scale", "Move", "Clean audio", "Volume"] {
            assert!(n.contains(&want), "{want} in {n:?}");
        }
        assert!(n.iter().any(|x| x.starts_with("Apply LUT: ")), "{n:?}");
        assert!(
            n.iter().all(|x| !x.to_lowercase().contains("whole video")),
            "no whole-video mark in {n:?}"
        );
    }
    // a change of clip 2's Volume leaves the rows of clips 1 and 3 as they were
    app.dispatch(
        Command::SetClipVolume {
            clip: ids[1],
            volume: 20,
            muted: false,
        },
        now(),
    )
    .unwrap();
    assert_eq!(rows(&mut app, ids[0]), before[0]);
    assert_eq!(rows(&mut app, ids[2]), before[2]);
    // the project's Change aspect and the edge fades are plain rows too
    app.dispatch(
        Command::SetAspect(crate::frame::FrameAspect {
            shape: crate::frame::Shape::Tall9x16,
            fit: crate::frame::Fit::Width,
            fill: crate::frame::Fill::Blur,
            blur: Default::default(),
        }),
        now(),
    )
    .unwrap();
    for edge in [Edge::Start, Edge::End] {
        app.dispatch(Command::SetEdgeFade { edge, on: true }, now())
            .unwrap();
    }
    let name = |app: &mut AppState, c: ClipId, n: &str| rows(app, c).iter().any(|r| r.name == n);
    assert!(name(&mut app, ids[0], "Fade IN") && !name(&mut app, ids[1], "Fade IN"));
    assert!(name(&mut app, ids[2], "Fade OUT") && !name(&mut app, ids[1], "Fade OUT"));
    assert!(ids.iter().all(|c| name(&mut app, *c, "Change aspect")));
}

// covers: REQ-313
#[test]
fn the_rows_are_built_once_and_again_only_after_a_change_of_the_clip_or_its_effects() {
    let (_d, mut app, lut) = three();
    let (a, b) = (clip(&app, 0), clip(&app, 1));
    set_lut(&mut app, &lut, a);
    app.dispatch(Command::SelectClip(Some(a)), now()).unwrap();
    let first = app.applied().unwrap();
    let n = app.applied_builds();
    // sixty frames with no change: no build
    for _ in 0..60 {
        assert_eq!(app.applied().unwrap(), first);
    }
    assert_eq!(app.applied_builds(), n);
    // one change (an effect added): one build
    app.dispatch(
        Command::SetClipVolume {
            clip: a,
            volume: 40,
            muted: false,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.applied().unwrap().rows.len(), first.rows.len() + 1);
    assert_eq!(app.applied_builds(), n + 1);
    // a move, an undo and another selected clip each count once
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.applied().unwrap(), first);
    assert_eq!(app.applied_builds(), n + 2);
    app.dispatch(Command::SelectClip(Some(b)), now()).unwrap();
    assert!(app.applied().unwrap().rows.is_empty());
    assert_eq!(app.applied_builds(), n + 3);
    for _ in 0..60 {
        app.applied();
    }
    assert_eq!(app.applied_builds(), n + 3);
    // with no clip selected the clip under the playhead counts: moving the playhead into
    // another clip is a change, moving inside the same clip is not
    app.dispatch(Command::SelectClip(None), now()).unwrap();
    app.dispatch(Command::Seek(s(1)), now()).unwrap();
    app.applied();
    let m = app.applied_builds();
    app.dispatch(Command::Seek(s(2)), now()).unwrap();
    app.applied();
    assert_eq!(app.applied_builds(), m);
    app.dispatch(Command::Seek(s(7)), now()).unwrap();
    app.applied();
    assert_eq!(app.applied_builds(), m + 1);
}

// covers: REQ-320, REQ-321
#[test]
fn the_sound_rows_move_inside_their_group_in_one_named_undo_step_and_speed_stays_fixed() {
    let (_d, mut app, lut) = three();
    let (c, other) = (clip(&app, 0), clip(&app, 1));
    set_lut(&mut app, &lut, c);
    for k in [c, other] {
        app.dispatch(
            Command::SetClipVolume {
                clip: k,
                volume: 50,
                muted: false,
            },
            now(),
        )
        .unwrap();
        app.dispatch(
            Command::SetCleanAudio {
                target: Target::Clip(k),
                on: true,
            },
            now(),
        )
        .unwrap();
    }
    app.dispatch(
        Command::SetSpeed {
            clip: c,
            speed: 2000,
        },
        now(),
    )
    .unwrap();
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    let names = |app: &mut AppState, k: ClipId| -> Vec<String> {
        app.dispatch(Command::SelectClip(Some(k)), now()).unwrap();
        app.applied()
            .unwrap()
            .rows
            .into_iter()
            .map(|r| r.name)
            .collect()
    };
    // the default is the sound of 1.4.0: Clean audio, then Volume; Speed is the last, fixed row
    assert_eq!(
        names(&mut app, c),
        [
            "Apply LUT: warm.cube",
            "Clean audio",
            "Volume",
            "Change speed: 200%"
        ]
    );
    let speed = app
        .applied()
        .unwrap()
        .rows
        .into_iter()
        .find(|r| r.name.starts_with("Change speed"))
        .unwrap();
    assert!(speed.row.is_none(), "Change speed cannot be moved");
    // Volume up: above Clean audio, one named step
    let up = Command::MoveRow {
        clip: c,
        row: Row::Volume,
        mv: RowMove::Up,
    };
    assert!(app.is_enabled(&up));
    app.dispatch(up.clone(), now()).unwrap();
    assert_eq!(app.undo_text(), "Undo Move Volume up");
    assert_eq!(
        names(&mut app, c),
        [
            "Apply LUT: warm.cube",
            "Volume",
            "Clean audio",
            "Change speed: 200%"
        ]
    );
    assert!(app.project.sound_swapped(c));
    // the other clip keeps its order (a copy is moved on its own clip)
    assert!(!app.project.sound_swapped(other));
    // it is the top row of its group now: greyed out, and no move across to the picture group
    assert!(!app.is_enabled(&up));
    assert_eq!(
        app.disabled_reason(&up),
        Some("It is already the top row of its group")
    );
    assert!(
        !app.is_enabled(&Command::MoveRow {
            clip: c,
            row: Row::Volume,
            mv: RowMove::To(0),
        }),
        "a drag cannot put a sound row among the picture rows"
    );
    // the picture rows were not touched: no overlay stage for a sound swap
    assert!(app.project.changed_orders().is_empty());
    // one undo restores the order and stores nothing
    app.dispatch(Command::Undo, now()).unwrap();
    assert!(!app.project.sound_swapped(c));
    assert!(app.project.timeline.clip_order.is_empty());
}

// covers: REQ-320, REQ-321
#[test]
fn every_effect_of_the_picture_is_a_row_that_moves_in_one_named_undo_step_and_a_copy_on_one_clip_stays_put()
 {
    let (_d, mut app, lut) = three();
    let ids: Vec<ClipId> = (0..3).map(|i| clip(&app, i)).collect();
    for effect in [EffectKind::Scale, EffectKind::Move] {
        app.dispatch(
            Command::ApplyEffect {
                effect,
                target: Target::WholeVideo,
            },
            now(),
        )
        .unwrap();
    }
    set_lut(&mut app, &lut, ids[0]);
    app.dispatch(
        Command::SetAspect(crate::frame::FrameAspect::default()),
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetEdgeFade {
            edge: Edge::Start,
            on: true,
        },
        now(),
    )
    .unwrap();
    let rows = |app: &mut AppState, c: ClipId| {
        app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
        let a = app.applied().unwrap();
        assert_eq!(a.picture, a.rows.iter().filter(|r| r.row.is_some()).count());
        a.rows.into_iter().map(|r| r.name).collect::<Vec<_>>()
    };
    // the default is the build of the older versions: the fade is applied last, then the frame,
    // the Move, the Scale and the LUT; each row is movable
    let default = [
        "Fade IN",
        "Change aspect",
        "Move",
        "Scale",
        "Apply LUT: warm.cube",
    ];
    assert_eq!(rows(&mut app, ids[0]), default);
    assert_eq!(rows(&mut app, ids[1]), ["Change aspect", "Move", "Scale"]);
    // the Change aspect row opens the project's Change aspect, the fade its popup
    app.dispatch(Command::SelectClip(Some(ids[0])), now())
        .unwrap();
    let a = app.applied().unwrap();
    assert!(a.rows.iter().all(|r| r.row.is_some()));
    assert_eq!(
        a.rows[1].open,
        vec![
            Command::SelectClip(Some(ids[0])),
            Command::OpenInstance(Instance::Aspect)
        ]
    );
    assert_eq!(
        a.rows[0].open,
        vec![
            Command::SelectClip(Some(ids[0])),
            Command::OpenInstance(Instance::FadeStart)
        ]
    );
    // Scale up: above Move; one step with its name
    let up = Command::MoveRow {
        clip: ids[0],
        row: Row::Scale,
        mv: RowMove::Up,
    };
    assert!(app.is_enabled(&up));
    app.dispatch(up, now()).unwrap();
    assert_eq!(app.undo_text(), "Undo Move Scale up");
    assert_eq!(
        rows(&mut app, ids[0]),
        [
            "Fade IN",
            "Change aspect",
            "Scale",
            "Move",
            "Apply LUT: warm.cube"
        ]
    );
    // the copy on the second clip did not move
    assert_eq!(rows(&mut app, ids[1]), ["Change aspect", "Move", "Scale"]);
    // each kind has its own undo name
    for (row, name) in [
        (Row::Crop, None),
        (Row::Frame, Some("Undo Move Change aspect down")),
        (Row::Move, Some("Undo Move the Move effect down")),
        (Row::Fade, Some("Undo Move Fade down")),
        (Row::Lut, Some("Undo Move LUT up")),
    ] {
        let mv = if row == Row::Lut {
            RowMove::Up
        } else {
            RowMove::Down
        };
        app.dispatch(Command::SelectClip(Some(ids[0])), now())
            .unwrap();
        let cmd = Command::MoveRow {
            clip: ids[0],
            row,
            mv,
        };
        if let Some(name) = name {
            assert!(app.is_enabled(&cmd), "{row:?}");
            app.dispatch(cmd, now()).unwrap();
            assert_eq!(app.undo_text(), name);
        } else {
            // the clip has no Crop: no such row
            assert!(!app.is_enabled(&cmd));
        }
    }
    // a drag to the top of the list, and undo after undo restores the default order
    while app.undo_text().starts_with("Undo Move") {
        app.dispatch(Command::Undo, now()).unwrap();
    }
    assert!(app.project.timeline.clip_order.is_empty());
    assert_eq!(rows(&mut app, ids[0]), default);
    app.dispatch(
        Command::MoveRow {
            clip: ids[0],
            row: Row::Fade,
            mv: RowMove::To(4),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Move effect");
    assert_eq!(rows(&mut app, ids[0])[4], "Fade IN");
}

// covers: REQ-321
#[test]
fn moving_a_picture_row_keeps_the_order_of_the_sound_rows() {
    let (_d, mut app, _) = three();
    let c = clip(&app, 0);
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c),
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Move,
            target: Target::Clip(c),
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetClipVolume {
            clip: c,
            volume: 50,
            muted: false,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetCleanAudio {
            target: Target::Clip(c),
            on: true,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::MoveRow {
            clip: c,
            row: Row::Volume,
            mv: RowMove::Up,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::MoveRow {
            clip: c,
            row: Row::Scale,
            mv: RowMove::Up,
        },
        now(),
    )
    .unwrap();
    // the Volume above Clean audio is still stored
    assert!(app.project.sound_swapped(c));
    app.dispatch(Command::Undo, now()).unwrap();
    assert!(app.project.sound_swapped(c));
}

// covers: REQ-304
#[test]
fn moving_a_row_is_one_named_undo_step_and_one_undo_restores_the_order() {
    let (_d, mut app, lut) = three();
    let c = clip(&app, 0);
    set_lut(&mut app, &lut, c);
    let t = add_text(&mut app, "Hello", 1);
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    let rows = |app: &AppState| {
        app.applied()
            .unwrap()
            .rows
            .iter()
            .filter_map(|r| r.row)
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(&app), [Row::Text(t.0), Row::Lut]);
    // the LUT up: above the text
    let up = Command::MoveRow {
        clip: c,
        row: Row::Lut,
        mv: RowMove::Up,
    };
    assert!(app.is_enabled(&up));
    app.dispatch(up.clone(), now()).unwrap();
    assert_eq!(rows(&app), [Row::Lut, Row::Text(t.0)]);
    assert_eq!(app.undo_text(), "Undo Move LUT up");
    assert!(app.dirty);
    // it cannot go higher: greyed out with its reason
    assert!(!app.is_enabled(&up));
    assert_eq!(app.disabled_reason(&up), Some("It is already the top row"));
    // and moving it is a no-op that makes no step
    let n = app.history.undo_len();
    app.dispatch(up, now()).unwrap();
    assert_eq!(app.history.undo_len(), n);
    // the order is saved with the clip
    assert_eq!(app.project.timeline.order_of(c), [Row::Lut, Row::Text(t.0)]);
    // one undo restores the order of before
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(rows(&app), [Row::Text(t.0), Row::Lut]);
    assert!(app.project.timeline.clip_order.is_empty());
    app.dispatch(Command::Redo, now()).unwrap();
    assert_eq!(rows(&app), [Row::Lut, Row::Text(t.0)]);
    // down is the same step the other way, named after the text
    app.dispatch(
        Command::MoveRow {
            clip: c,
            row: Row::Text(t.0),
            mv: RowMove::Up,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Move text up");
    app.dispatch(
        Command::MoveRow {
            clip: c,
            row: Row::Text(t.0),
            mv: RowMove::Down,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Move text down");
    // a drag to a place of the list
    app.dispatch(
        Command::MoveRow {
            clip: c,
            row: Row::Lut,
            mv: RowMove::To(1),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.undo_text(), "Undo Move effect");
}

// covers: REQ-302
// covers: REQ-304
#[test]
fn on_a_360_clip_the_texts_and_lines_fixed_to_the_sphere_are_fixed_rows() {
    let (_d, mut app, _) = three();
    app.project.media[0].format.spherical = Some(true);
    let c = clip(&app, 0);
    // a flat text, a text fixed to the sphere, and a line fixed to the sphere
    let flat = add_text(&mut app, "Flat", 1);
    // (a text added over a 360 clip is fixed to the sphere: this one is made flat)
    app.project.texts.items[0].anchor = None;
    let anchor = crate::base::pos::Anchor360 {
        yaw_cd: 0,
        pitch_cd: 0,
        height_cd: 1000,
        width_cd: 3000,
    };
    let mut fixed = TextItem::new(TextId(0), "Fixed".into(), s(1));
    fixed.anchor = Some(anchor);
    fixed.id = app.project.texts.new_id();
    app.project.texts.add(fixed).unwrap();
    let tl = app.project.timeline.clone();
    app.project
        .subtitles
        .add_at(&tl, s(1), "line", Micros(500_000))
        .unwrap();
    app.project.subtitles.lines[0].anchor = Some(anchor);
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    let a = app.applied().unwrap();
    // only the flat text is movable; the sphere rows are fixed, drawn first
    assert_eq!(
        names(&a),
        [
            "Text: Flat",
            "Text fixed to the sphere: Fixed",
            "Subtitles fixed to the sphere"
        ]
    );
    assert_eq!(a.movable(), 1);
    assert_eq!(a.rows[0].row, Some(Row::Text(flat.0)));
    assert!(a.rows[1..].iter().all(|r| r.row.is_none()));
    // on a flat clip the same anchored text is an ordinary movable row
    app.project.media[0].format.spherical = Some(false);
    let rows = app.project.rows_of(c).unwrap();
    assert!(rows.sphere_texts.is_empty() && !rows.sphere_lines);
    assert_eq!(rows.present.texts.len(), 2);
}

// covers: REQ-304
#[test]
fn the_labels_of_the_moves_name_the_row_and_the_direction() {
    use super::applied::move_label;
    assert_eq!(move_label(Row::Picture(1), RowMove::Up), "Move picture up");
    assert_eq!(
        move_label(Row::Picture(1), RowMove::Down),
        "Move picture down"
    );
    assert_eq!(move_label(Row::Subtitles, RowMove::Up), "Move subtitles up");
    assert_eq!(
        move_label(Row::Subtitles, RowMove::Down),
        "Move subtitles down"
    );
    assert_eq!(move_label(Row::Lut, RowMove::To(0)), "Move effect");
    assert_eq!(move_label(Row::Unknown, RowMove::Up), "Move effect");
}

// covers: REQ-304
#[test]
fn a_row_that_is_gone_or_a_clip_that_is_gone_is_refused_with_a_reason() {
    let (_d, mut app, _) = three();
    let c = clip(&app, 0);
    let gone = Command::MoveRow {
        clip: c,
        row: Row::Text(99),
        mv: RowMove::Up,
    };
    assert!(!app.is_enabled(&gone));
    assert_eq!(
        app.disabled_reason(&gone),
        Some("That effect is no longer on the clip")
    );
    assert!(app.dispatch(gone, now()).is_err());
    let none = Command::MoveRow {
        clip: ClipId(999),
        row: Row::Lut,
        mv: RowMove::Down,
    };
    assert_eq!(
        app.disabled_reason(&none),
        Some("That clip no longer exists")
    );
    assert!(app.dispatch(none, now()).is_err());
    // the lowest row cannot go lower
    let t = add_text(&mut app, "x", 1);
    let down = Command::MoveRow {
        clip: c,
        row: Row::Text(t.0),
        mv: RowMove::Down,
    };
    assert_eq!(
        app.disabled_reason(&down),
        Some("It is already the bottom row")
    );
    assert!(app.dispatch(down, now()).is_ok());
}

// covers: REQ-304
#[test]
fn a_new_lut_or_picture_goes_on_top_of_the_clips_other_effects() {
    let (dir, mut app, lut) = three();
    let c = clip(&app, 0);
    let t = add_text(&mut app, "Hello", 1);
    // a LUT applied after the text is above it: the clip has a stored order
    set_lut(&mut app, &lut, c);
    assert_eq!(app.project.order_of(c), vec![Row::Lut, Row::Text(t.0)]);
    assert_eq!(app.project.timeline.clip_order.len(), 1);
    // choosing another file for the same clip does not move the row again
    app.dispatch(
        Command::MoveRow {
            clip: c,
            row: Row::Lut,
            mv: RowMove::Down,
        },
        now(),
    )
    .unwrap();
    let other = dir.path().join("other.cube");
    std::fs::write(&other, crate::lut::tests::identity(3)).unwrap();
    set_lut(&mut app, &other, c);
    assert_eq!(app.project.order_of(c), vec![Row::Text(t.0), Row::Lut]);
    // a picture goes above the text
    let png = dir.path().join("logo.png");
    std::fs::write(&png, crate::picture::tests::header(10, 10)).unwrap();
    app.dispatch(Command::Seek(s(1)), now()).unwrap();
    app.dispatch(Command::AddPicture(Some(png)), now()).unwrap();
    let pic = app.project.texts.items.last().unwrap().id.0;
    assert_eq!(
        app.project.order_of(c),
        vec![Row::Picture(pic), Row::Text(t.0), Row::Lut]
    );
    // one undo takes the picture and its place away
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.project.order_of(c), vec![Row::Text(t.0), Row::Lut]);
    assert!(app.project.timeline.clip_order.is_empty());
}
