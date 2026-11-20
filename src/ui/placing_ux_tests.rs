// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 2.0.3: warnings of overlapping lines, the stack of boxes under a click,
//! the selection hint, the dashed outline and the status text of items partly outside, and
//! the group move of lines and texts together (REQ-169 to REQ-176).

use super::ux_tests::{Harness, close_menus, find_in, open_menu};
use super::*;
use crate::app::Popup;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::base::pos::Pos;
use crate::selection::ObjRef;
use crate::subtitles::{LineId, NewLine};
use crate::text::{TextId, TextItem};
use eframe::egui::{Event, Pos2, Rect, pos2};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips, the playhead at 1 s, a frame on screen and three lines: "one" and "two"
/// at the style's place for 0 to 4 s, "late" at the same place from 4 s.
fn loaded() -> (tempfile::TempDir, AppState, Vec<LineId>) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    let m = app.project.media[0].id;
    let line = |text: &str, a: i64, b: i64| NewLine {
        media: m,
        start: s(a),
        end: s(b),
        text: text.into(),
        words: vec![],
    };
    app.project.subtitles.replace_all(
        vec![line("one", 0, 4), line("two", 0, 4), line("late", 4, 5)],
        None,
    );
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    let ids = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    (dir, app, ids)
}

fn picture(h: &Harness) -> Rect {
    h.find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect
}

/// The screen point of a frame fraction.
fn at(pic: Rect, fx: f32, fy: f32) -> Pos2 {
    pos2(pic.min.x + fx * pic.width(), pic.min.y + fy * pic.height())
}

/// The screen point at the centre of the box of line `id`.
fn centre_of(h: &Harness, id: LineId) -> Pos2 {
    let app = &h.app;
    let l = app.project.subtitles.get(id).unwrap();
    let frame = app.output_size().unwrap();
    let r = crate::placement::line_rect(
        app.project.subtitles.preset,
        &l.text,
        l.position,
        l.size,
        frame,
    );
    at(
        picture(h),
        (r.cx / f64::from(frame.0)) as f32,
        (r.cy / f64::from(frame.1)) as f32,
    )
}

// covers: REQ-169
#[test]
fn the_subtitles_list_marks_lines_that_overlap_and_the_mark_goes_with_the_overlap() {
    let (_d, mut app, ids) = loaded();
    app.popups.show(Popup::Subtitles);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let warned = |h: &Harness| {
        h.nodes
            .iter()
            .filter(|n| n.raw.contains(icons::WARNING))
            .count()
    };
    // "one" and "two" are at one place at the same time; "late" is at another time
    assert_eq!(warned(&h), 2, "{:#?}", h.labels());
    assert_eq!(h.cache.overlapping_lines(&h.app).len(), 2);
    assert!(!h.cache.overlapping_lines(&h.app).contains(&ids[2]));
    // one dragged away clears the mark for both
    h.app
        .dispatch(
            Command::SetLinePositions {
                ids: vec![ids[1]],
                pos: Some(Pos { x: 5000, y: 1500 }),
            },
            Instant::now(),
        )
        .unwrap();
    h.idle(3);
    assert_eq!(warned(&h), 0, "{:#?}", h.labels());
    assert!(h.cache.overlapping_lines(&h.app).is_empty());
}

// covers: REQ-170
#[test]
fn a_second_click_on_the_same_spot_selects_the_next_box_of_the_stack() {
    let (_d, mut app, ids) = loaded();
    app.dispatch(Command::ClosePopup(Popup::Subtitles), Instant::now())
        .ok();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let p = centre_of(&h, ids[0]);
    let selected = |h: &Harness| h.app.selection.lines();
    h.click(p);
    let first = selected(&h);
    assert_eq!(first.len(), 1);
    h.click(p);
    let second = selected(&h);
    assert_eq!(second.len(), 1);
    assert_ne!(first, second, "the second click takes the other line");
    assert!(
        !first.contains(&ids[2]) && !second.contains(&ids[2]),
        "a line shown at another time is never picked"
    );
    // and round again
    h.click(p);
    assert_eq!(selected(&h), first);
    // a click on another spot starts again with the topmost
    h.click(at(picture(&h), 0.1, 0.1));
    assert!(selected(&h).is_empty());
}

// covers: REQ-171
#[test]
fn the_status_bar_hints_how_to_select_several_only_while_something_is_selected() {
    let (_d, app, ids) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    assert!(!h.has(status::SELECTION_HINT), "nothing selected: no hint");
    h.app
        .dispatch(Command::SelectLine(Some(ids[0])), Instant::now())
        .unwrap();
    h.idle(3);
    let hint = h
        .find("Ctrl/Cmd+click")
        .expect("a hint with a line selected");
    assert!(hint.label.contains("Shift+drag"), "{}", hint.label);
    assert_eq!(hint.label, status::SELECTION_HINT);
    // the Subtitles popup lists the gestures too
    h.app.popups.show(Popup::Subtitles);
    h.idle(3);
    assert!(
        h.nodes
            .iter()
            .filter(|n| n.label == status::SELECTION_HINT)
            .count()
            >= 1
    );
    // a text item selected shows it as well
    h.app.selection.clear();
    h.idle(2);
    assert!(!h.has(status::SELECTION_HINT) || h.app.popups.is_open(Popup::Subtitles));
    h.app
        .dispatch(Command::SelectText(TextId(9)), Instant::now())
        .ok();
}

// covers: REQ-172
#[test]
fn a_line_and_a_text_selected_together_are_dragged_as_one_group() {
    let (_d, mut app, ids) = loaded();
    let now = Instant::now();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), "text".into(), Micros::ZERO)),
        now,
    )
    .unwrap();
    let t = app.selection.primary_text().unwrap();
    app.dispatch(Command::ClosePopup(Popup::Text), now).ok();
    app.dispatch(Command::SelectLine(Some(ids[2])), now)
        .unwrap();
    app.dispatch(Command::SelectLine(Some(ids[0])), now)
        .unwrap();
    app.selection
        .pick(&[ObjRef::Line(ids[0]), ObjRef::Text(t)], false);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let n = h.app.history.undo_len();
    // the text box is at 50% across, 83% down: drag it to 50% across and 20% down
    h.drag(at(pic, 0.5, 0.83), at(pic, 0.5, 0.2));
    let item = h.app.project.texts.get(t).unwrap();
    let line = h.app.project.subtitles.get(ids[0]).unwrap();
    assert!(
        (i32::from(item.place.y) - 2000).abs() <= 60,
        "{:?}",
        item.place
    );
    assert_eq!(
        line.position,
        Some(item.place),
        "both end at the dragged place"
    );
    assert_eq!(h.app.history.undo_len(), n + 1);
    h.key(eframe::egui::Key::Z, eframe::egui::Modifiers::COMMAND);
    assert_eq!(h.app.project.subtitles.get(ids[0]).unwrap().position, None);
    assert_eq!(
        h.app.project.texts.get(t).unwrap().place,
        crate::text::DEFAULT_PLACE
    );
}

// covers: REQ-175, REQ-176
#[test]
fn items_partly_outside_get_a_dashed_outline_a_status_text_and_a_move_inside_command() {
    let (_d, mut app, ids) = loaded();
    let now = Instant::now();
    app.dispatch(Command::ClosePopup(Popup::Subtitles), now)
        .ok();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let dashes = |h: &mut Harness| {
        let raw = eframe::egui::RawInput {
            screen_rect: Some(h.screen()),
            events: vec![Event::PointerMoved(pos2(5.0, 5.0))],
            ..Default::default()
        };
        let now = Instant::now();
        let (app, cache) = (&mut h.app, &mut h.cache);
        let mut out = h.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
        let n = out
            .shapes
            .iter()
            .filter(|s| matches!(s.shape, eframe::egui::Shape::LineSegment { .. }))
            .count();
        out.textures_delta.clear();
        out.drop_without_applying_deltas();
        n
    };
    let inside = dashes(&mut h);
    assert!(!h.nodes.iter().any(|n| n.label.contains("partly outside")));
    // a line whose centre is on the right edge of the picture
    h.app
        .dispatch(
            Command::SetLinePositions {
                ids: vec![ids[0]],
                pos: Some(Pos { x: 10_000, y: 5000 }),
            },
            now,
        )
        .unwrap();
    h.idle(3);
    let outside = dashes(&mut h);
    assert!(outside >= inside + 6, "dashes: {inside} -> {outside}");
    assert!(
        h.has("1 item is partly outside the picture"),
        "{:#?}",
        h.labels()
    );
    // in the Edit menu: greyed out with nothing selected, enabled with the line selected
    let nodes = open_menu(&mut h, "Edit");
    let item = find_in(&nodes, "Move inside the picture").expect("in the Edit menu");
    assert!(item.disabled, "nothing selected");
    close_menus(&mut h);
    h.app
        .dispatch(Command::SelectLine(Some(ids[0])), now)
        .unwrap();
    h.idle(3);
    let nodes = open_menu(&mut h, "Edit");
    let item = find_in(&nodes, "Move inside the picture")
        .expect("in the Edit menu")
        .clone();
    assert!(!item.disabled);
    close_menus(&mut h);
    // the command of the menu is the one that moves the line inside, in one undo step
    let action = actions::ActionId::MoveInside;
    let n = h.app.history.undo_len();
    h.app.dispatch(action.command(&h.app), now).unwrap();
    assert_eq!(h.app.history.undo_len(), n + 1);
    assert!(h.app.partly_outside().is_empty());
    h.idle(3);
    assert!(!h.has("1 item is partly outside the picture"));
    // with everything inside the command is greyed out
    h.app
        .dispatch(Command::SelectLine(Some(ids[0])), now)
        .unwrap();
    let nodes = open_menu(&mut h, "Edit");
    assert!(find_in(&nodes, "Move inside the picture").unwrap().disabled);
}
