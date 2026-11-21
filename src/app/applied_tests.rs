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
        Command::SetCrop {
            clip: c,
            crop: crate::timeline::Crop {
                left: 10,
                ..Default::default()
            },
        },
        now(),
    )
    .unwrap();
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
            "Crop",
            "Volume",
            "Speed"
        ]
    );
    assert_eq!(a.movable(), 2);
    assert_eq!(a.rows[0].row, Some(Row::Text(t.0)));
    assert_eq!(a.rows[1].row, Some(Row::Lut));
    assert!(a.rows[2..].iter().all(|r| r.row.is_none()));
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
        a.rows[2].open,
        vec![
            Command::SelectClip(Some(c)),
            Command::OpenPopup(Popup::ClipDetails)
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

// covers: REQ-303
#[test]
fn a_whole_video_effect_is_marked_on_the_clips_it_affects() {
    let (_d, mut app, _) = three();
    let ids: Vec<ClipId> = (0..3).map(|i| clip(&app, i)).collect();
    let volume = |app: &mut AppState, c: ClipId, v: u8| {
        app.dispatch(
            Command::SetClipVolume {
                clip: c,
                volume: v,
                muted: false,
            },
            now(),
        )
        .unwrap();
    };
    let marks = |app: &mut AppState, name: &str| -> Vec<Option<bool>> {
        ids.iter()
            .map(|c| {
                app.dispatch(Command::SelectClip(Some(*c)), now()).unwrap();
                app.applied()
                    .unwrap()
                    .rows
                    .iter()
                    .find(|r| r.name == name)
                    .map(|r| r.whole)
            })
            .collect()
    };
    // the same level on every clip: marked on each
    for c in &ids {
        volume(&mut app, *c, 50);
    }
    assert_eq!(marks(&mut app, "Volume"), [Some(true); 3]);
    // one clip differs: the effect is no longer the same on every clip, no mark
    volume(&mut app, ids[1], 20);
    assert_eq!(marks(&mut app, "Volume"), [Some(false); 3]);
    // a Change aspect of the whole video is shown on every clip, with the mark
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
    assert_eq!(marks(&mut app, "Change aspect"), [Some(true); 3]);
    // the fade in the whole video starts: only the first clip has it; the fade out only the last
    for edge in [Edge::Start, Edge::End] {
        app.dispatch(Command::SetEdgeFade { edge, on: true }, now())
            .unwrap();
    }
    assert_eq!(marks(&mut app, "Fade IN"), [Some(true), None, None]);
    assert_eq!(marks(&mut app, "Fade OUT"), [None, None, Some(true)]);
    // a fade between two clips is on both, without the mark
    app.dispatch(Command::ToggleFade(0), now()).unwrap();
    assert_eq!(
        marks(&mut app, "Fade IN/OUT with the next clip"),
        [Some(false), None, None]
    );
    assert_eq!(
        marks(&mut app, "Fade IN/OUT with the previous clip"),
        [None, Some(false), None]
    );
}

// covers: REQ-303
#[test]
fn whole_video_scale_move_lut_and_clean_audio_copies_are_marked() {
    let (_d, mut app, lut) = three();
    for i in 0..3 {
        let c = clip(&app, i);
        set_lut(&mut app, &lut, c);
        app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
        for kind in [EffectKind::Scale, EffectKind::Move] {
            // applying the effect gives the clip its first key
            app.dispatch(
                Command::ApplyEffect {
                    effect: kind,
                    target: Target::Clip(c),
                },
                now(),
            )
            .unwrap();
        }
        app.dispatch(
            Command::SetCleanAudio {
                target: Target::Clip(c),
                on: true,
            },
            now(),
        )
        .unwrap();
    }
    for i in 0..3 {
        app.dispatch(Command::SelectClip(Some(clip(&app, i))), now())
            .unwrap();
        let a = app.applied().unwrap();
        for name in ["Scale", "Move", "Clean audio"] {
            let r = a.rows.iter().find(|r| r.name == name).expect(name);
            assert!(r.whole && r.row.is_none(), "{name}");
        }
        let l = a.rows.iter().find(|r| r.row == Some(Row::Lut)).unwrap();
        assert!(l.whole);
    }
    // a timeline of one clip has no "whole video" apart from the clip
    let (_d2, mut one, lut) = three();
    for _ in 0..2 {
        one.dispatch(Command::Delete, now()).unwrap();
    }
    assert_eq!(one.project.timeline.clips.len(), 1);
    let c = clip(&one, 0);
    set_lut(&mut one, &lut, c);
    one.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    let a = one.applied().unwrap();
    assert!(!a.rows[0].whole);
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
