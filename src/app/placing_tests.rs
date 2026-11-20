// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 2.0.3: where boxes end up (REQ-168 to REQ-176).

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::placement::{self, line_rect};
use crate::subtitles::NewLine;
use crate::text::{TextId, TextItem};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips and two subtitle lines of 4 s ("one", "two") at the style's place.
fn with_lines() -> (tempfile::TempDir, AppState, LineId, LineId) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        let id = app.project.media[i].id;
        app.dispatch(Command::Append(Some(id)), now).unwrap();
    }
    let m = app.project.media[0].id;
    let line = |text: &str| NewLine {
        media: m,
        start: Micros(0),
        end: s(4),
        text: text.into(),
        words: vec![],
    };
    app.project
        .subtitles
        .replace_all(vec![line("one"), line("two")], None);
    let ids: Vec<LineId> = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    (dir, app, ids[0], ids[1])
}

fn frame(app: &AppState) -> (u32, u32) {
    app.output_size().expect("a frame size")
}

fn rect_of(app: &AppState, id: LineId) -> placement::Rect {
    let l = app.project.subtitles.get(id).unwrap();
    line_rect(
        app.project.subtitles.preset,
        &l.text,
        l.position,
        l.size,
        frame(app),
    )
}

/// Wraps onto two lines of the picture.
const TWO_LINES: &str = "subtitle words that go on and on and on and on and on and on and on and on and on and on and on";

const LONG: &str = "a much longer line of subtitle text that is wide";

// covers: REQ-168
#[test]
fn a_longer_text_moves_a_box_back_inside_by_the_least_move_in_one_undo_step() {
    let (_d, mut app, l1, _) = with_lines();
    let now = Instant::now();
    let f = frame(&app);
    // a line whose right edge is just past the picture
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l1],
            pos: Some(Pos { x: 9900, y: 5000 }),
        },
        now,
    )
    .unwrap();
    assert!(rect_of(&app, l1).partly_outside(f));
    let before = app.project.subtitles.get(l1).unwrap().clone();
    let undo = app.history.undo_len();
    app.dispatch(Command::SetSubtitleText(l1, LONG.into()), now)
        .unwrap();
    let r = rect_of(&app, l1);
    assert!(
        (r.right() - f64::from(f.0)).abs() <= 2.0,
        "right edge {} on the picture edge {}",
        r.right(),
        f.0
    );
    assert!(r.left() > 0.0, "and still inside on the left");
    assert_eq!(
        app.history.undo_len(),
        undo + 1,
        "the move is part of the edit"
    );
    // a shorter text leaves the place where it is
    app.dispatch(Command::SetSubtitleText(l1, "o".into()), now)
        .unwrap();
    let kept = app.project.subtitles.get(l1).unwrap().position;
    app.dispatch(Command::SetSubtitleText(l1, "ok".into()), now)
        .unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap().position, kept);
    // one undo restores the old text and place
    app.dispatch(Command::Undo, now).unwrap();
    app.dispatch(Command::Undo, now).unwrap();
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap(), &before);
}

// covers: REQ-174
#[test]
fn a_second_line_keeps_the_centre_of_the_box() {
    let (_d, mut app, l1, _) = with_lines();
    let now = Instant::now();
    let f = frame(&app);
    // own place at 50% across and 80% down
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l1],
            pos: Some(Pos { x: 5000, y: 8000 }),
        },
        now,
    )
    .unwrap();
    let a = rect_of(&app, l1);
    app.dispatch(Command::SetSubtitleText(l1, TWO_LINES.into()), now)
        .unwrap();
    let b = rect_of(&app, l1);
    assert!(b.h > a.h, "the box grew");
    assert!((b.cx - a.cx).abs() <= 0.005 * f64::from(f.1));
    assert!((b.cy - a.cy).abs() <= 0.005 * f64::from(f.1));
    // at the style's place the box grows upwards under the renderer's own rule: the centre
    // is kept by storing the place
    let (_d2, mut app2, m1, _) = with_lines();
    let old = rect_of(&app2, m1);
    app2.dispatch(Command::SetSubtitleText(m1, TWO_LINES.into()), now)
        .unwrap();
    let new = rect_of(&app2, m1);
    assert!(app2.project.subtitles.get(m1).unwrap().position.is_some());
    assert!((new.cx - old.cx).abs() <= 0.005 * f64::from(f.1));
    assert!((new.cy - old.cy).abs() <= 0.005 * f64::from(f.1));
    // a longer line of the same height keeps the style's place
    let (_d3, mut app3, n1, _) = with_lines();
    app3.dispatch(Command::SetSubtitleText(n1, "one two".into()), now)
        .unwrap();
    assert!(app3.project.subtitles.get(n1).unwrap().position.is_none());
}

// covers: REQ-169
#[test]
fn lines_shown_together_whose_boxes_overlap_are_both_warned_and_the_warning_goes_away() {
    let (_d, mut app, l1, l2) = with_lines();
    let now = Instant::now();
    // two lines at the style's place at the same time
    let both: std::collections::BTreeSet<LineId> = [l1, l2].into();
    assert_eq!(app.overlapping_lines(), both);
    // one dragged away clears it for both
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l2],
            pos: Some(Pos { x: 5000, y: 1500 }),
        },
        now,
    )
    .unwrap();
    assert!(app.overlapping_lines().is_empty());
    // the same place at different times gives no warning
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l2],
            pos: None,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.overlapping_lines(), both);
    let m = app.project.media[0].id;
    app.project.subtitles.replace_all(
        vec![
            NewLine {
                media: m,
                start: Micros(0),
                end: s(2),
                text: "one".into(),
                words: vec![],
            },
            NewLine {
                media: m,
                start: s(2),
                end: s(4),
                text: "two".into(),
                words: vec![],
            },
        ],
        None,
    );
    assert!(app.overlapping_lines().is_empty());
}

// covers: REQ-172
#[test]
fn a_line_and_a_text_move_as_one_group_in_one_undo_step() {
    let (_d, mut app, l1, _) = with_lines();
    let now = Instant::now();
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), "text".into(), Micros::ZERO)),
        now,
    )
    .unwrap();
    let t = app.selection.primary_text().unwrap();
    let place = Pos { x: 5000, y: 2000 };
    let undo = app.history.undo_len();
    app.dispatch(
        Command::MoveSelection {
            lines: vec![l1],
            texts: vec![t],
            place,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap().position, Some(place));
    assert_eq!(app.project.texts.get(t).unwrap().place, place);
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(app.undo_text(), "Undo Move");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap().position, None);
    assert_eq!(
        app.project.texts.get(t).unwrap().place,
        crate::text::DEFAULT_PLACE
    );
    // the same place again is no change; nothing to move is refused
    app.dispatch(Command::Redo, now).unwrap();
    let n = app.history.undo_len();
    app.dispatch(
        Command::MoveSelection {
            lines: vec![l1],
            texts: vec![t],
            place,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    assert!(
        app.dispatch(
            Command::MoveSelection {
                lines: vec![],
                texts: vec![],
                place
            },
            now
        )
        .is_err()
    );
    assert!(
        app.dispatch(
            Command::MoveSelection {
                lines: vec![l1],
                texts: vec![],
                place: Pos { x: 20_000, y: 0 }
            },
            now
        )
        .is_err()
    );
}

// covers: REQ-175
#[test]
fn items_partly_outside_are_counted_and_move_inside_puts_them_in_one_undo_step() {
    let (_d, mut app, l1, l2) = with_lines();
    let now = Instant::now();
    assert!(app.partly_outside().is_empty());
    // a line 30% outside the picture: its centre at the right edge
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l1],
            pos: Some(Pos { x: 10_000, y: 5000 }),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.partly_outside(), [crate::selection::ObjRef::Line(l1)]);
    // nothing selected: greyed out
    let c = Command::MoveInside;
    assert!(!app.is_enabled(&c));
    assert_eq!(app.disabled_reason(&c), Some(sphere::SELECT_ON_PICTURE));
    // an item that is inside: greyed out with the reason
    app.dispatch(Command::SelectLine(Some(l2)), now).unwrap();
    assert!(!app.is_enabled(&c));
    assert_eq!(app.disabled_reason(&c), Some(placing::ALL_INSIDE));
    assert!(app.dispatch(c.clone(), now).is_err());
    // the outside one is moved by the least move
    app.dispatch(Command::SelectLine(Some(l1)), now).unwrap();
    assert!(app.is_enabled(&c));
    let undo = app.history.undo_len();
    app.dispatch(c.clone(), now).unwrap();
    assert!(app.partly_outside().is_empty());
    assert!(!rect_of(&app, l1).partly_outside(frame(&app)));
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(app.undo_text(), "Undo Move inside the picture");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(
        app.project.subtitles.get(l1).unwrap().position,
        Some(Pos { x: 10_000, y: 5000 })
    );
    // a text box too is moved inside, and one larger than the picture is centred
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), "text".into(), Micros::ZERO)),
        now,
    )
    .unwrap();
    let t = app.selection.primary_text().unwrap();
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![t],
            place: Pos { x: 10_000, y: 5000 },
        },
        now,
    )
    .unwrap();
    assert!(
        app.partly_outside()
            .contains(&crate::selection::ObjRef::Text(t))
    );
    app.dispatch(Command::SelectText(t), now).unwrap();
    app.dispatch(c, now).unwrap();
    let item = app.project.texts.get(t).unwrap();
    assert!(!placement::text_rect(item, frame(&app)).partly_outside(frame(&app)));
}

// covers: REQ-176
#[test]
fn scale_never_moves_an_item_and_the_warnings_still_apply() {
    let (_d, mut app, ..) = with_lines();
    let now = Instant::now();
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), "text".into(), Micros::ZERO)),
        now,
    )
    .unwrap();
    let t = app.selection.primary_text().unwrap();
    // at the right edge of the picture
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![t],
            place: Pos { x: 9000, y: 5000 },
        },
        now,
    )
    .unwrap();
    let before = app.project.texts.get(t).unwrap().place;
    assert!(
        app.partly_outside()
            .contains(&crate::selection::ObjRef::Text(t))
    );
    // Scale of 150% on the clip
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(clip),
        },
        now,
    )
    .unwrap();
    let target = crate::timeline::FrameTarget::Clip(clip);
    app.dispatch(
        Command::SetPicture(FrameDrag {
            target,
            value: FrameValue::Scale(crate::frame::Scale {
                permille: 1500,
                dx: 0,
                dy: 0,
            }),
        }),
        now,
    )
    .unwrap();
    // the item keeps its place in the frame and is still reported
    assert_eq!(app.project.texts.get(t).unwrap().place, before);
    assert!(
        app.partly_outside()
            .contains(&crate::selection::ObjRef::Text(t))
    );
}
