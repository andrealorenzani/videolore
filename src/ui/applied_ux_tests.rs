// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of the Applied effects widget (REQ-302, REQ-303, REQ-304), through egui's
//! accessibility tree.

use super::ux_tests::{Harness, reachable};
use super::*;
use crate::app::Instance;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::effect_order::Row;
use crate::text::{TextId, TextItem};
use crate::timeline::FrameTarget;
use eframe::egui::{Event, Key, Modifiers};

fn now() -> Instant {
    Instant::now()
}

/// Two clips; the first has a LUT, then a text (1 s to 4 s) added after it.
fn setup() -> (tempfile::TempDir, AppState, crate::timeline::ClipId, u64) {
    let dir = tempfile::tempdir().unwrap();
    let lut = dir.path().join("warm.cube");
    std::fs::write(&lut, crate::lut::tests::identity(2)).unwrap();
    let mut app = state(FakeDialogs {
        lut: Some(lut.clone()),
        ..Default::default()
    });
    import(&mut app, files(dir.path()));
    for i in [0, 1] {
        let m = app.project.media[i].id;
        app.dispatch(Command::Append(Some(m)), now()).unwrap();
    }
    let c = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Clip(c),
            path: lut,
        },
        now(),
    )
    .unwrap();
    app.dispatch(Command::Seek(Micros(1_000_000)), now())
        .unwrap();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), "Hello".into(), Micros::ZERO)),
        now(),
    )
    .unwrap();
    let t = app.selection.primary_text().unwrap().0;
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    (dir, app, c, t)
}

/// The button of the row or the Move button `label`; rows are announced as "Applied effect: ...".
fn button(h: &Harness, label: &str) -> super::ux_tests::Node {
    let label = &if label.starts_with("Move ") {
        label.to_string()
    } else {
        format!("Applied effect: {label}")
    };
    let preview = h.cache.sections.map(|s| s.preview);
    h.nodes
        .iter()
        .filter(|n| n.role == eframe::egui::accesskit::Role::Button && n.label.starts_with(label))
        .find(|n| preview.is_none_or(|p| p.contains(n.rect.center())))
        .cloned()
        .unwrap_or_else(|| panic!("no button \"{label}\" in {:#?}", h.labels()))
}

fn y_of(h: &Harness, label: &str) -> f32 {
    button(h, label).rect.center().y
}

fn click_button(h: &mut Harness, label: &str) {
    let p = button(h, label).rect.center();
    h.click(p);
}

// covers: REQ-302
#[test]
fn the_widget_shows_the_text_above_the_lut_and_a_click_opens_the_popup() {
    let (_d, app, c, _t) = setup();
    let mut h = Harness::new(app, 1800.0, 1600.0);
    assert!(h.has("Applied effects"));
    // the text was added after the LUT: its row is above the LUT row
    let lut = "Apply LUT: warm.cube";
    assert!(y_of(&h, "Text: Hello") < y_of(&h, lut), "{:#?}", h.labels());
    // the picture shrank: the widget is about 15% of the section
    let section = h.cache.sections.unwrap().preview;
    let title = h.find("Applied effects").unwrap();
    let below = section.max.y - title.rect.min.y;
    assert!(
        below >= 0.1 * section.height() && below <= 0.25 * section.height(),
        "{below} of {}",
        section.height()
    );
    // a click on the LUT row opens the Apply LUT popup
    click_button(&mut h, lut);
    assert_eq!(
        h.app.popups.instance,
        Some(Instance::Lut(FrameTarget::Clip(c)))
    );
    assert!(h.has("Choose file…"), "{:#?}", h.labels());
}

// covers: REQ-302
#[test]
fn a_clip_without_effects_shows_none_and_no_clip_says_to_select_one() {
    let (_d, app, _c, _t) = setup();
    let mut h = Harness::new(app, 1800.0, 1600.0);
    // the second clip has only the text that overlaps it? the text is 1-4 s: clip 1 is 5 s
    let second = h.app.project.timeline.clips[1].id;
    h.app
        .dispatch(Command::SelectClip(Some(second)), now())
        .unwrap();
    h.idle(2);
    assert!(h.has("Applied effects"));
    assert!(
        !h.nodes
            .iter()
            .any(|n| n.label.starts_with("Applied effect: Apply LUT")),
        "{:#?}",
        h.labels()
    );
    assert!(!h.has("Select a clip to see its effects"));
    // without a timeline the widget asks for a clip
    let empty = state(FakeDialogs::default());
    let h = Harness::new(empty, 1800.0, 1600.0);
    assert!(h.has("Select a clip to see its effects"));
}

// covers: REQ-304
#[test]
fn move_up_and_move_down_buttons_change_the_order_and_one_undo_restores_it() {
    let (_d, app, c, t) = setup();
    let mut h = Harness::new(app, 1800.0, 1600.0);
    let lut = "Apply LUT: warm.cube";
    // the top row cannot go up: its button is disabled
    let up_text = button(&h, "Move up: Text: Hello");
    assert!(up_text.disabled);
    // the LUT moves up: it is then drawn over the text
    click_button(&mut h, &format!("Move up: {lut}"));
    assert_eq!(h.app.project.order_of(c), vec![Row::Lut, Row::Text(t)]);
    assert!(y_of(&h, lut) < y_of(&h, "Text: Hello"));
    assert_eq!(h.app.undo_text(), "Undo Move LUT up");
    // and down again
    click_button(&mut h, &format!("Move down: {lut}"));
    assert_eq!(h.app.project.order_of(c), vec![Row::Text(t), Row::Lut]);
    assert_eq!(h.app.undo_text(), "Undo Move LUT down");
    h.app.dispatch(Command::Undo, now()).unwrap();
    h.idle(2);
    assert_eq!(h.app.project.order_of(c), vec![Row::Lut, Row::Text(t)]);
}

// covers: REQ-304
#[test]
fn a_row_can_be_dragged_and_moved_from_the_keyboard() {
    let (_d, app, c, t) = setup();
    let mut h = Harness::new(app, 1800.0, 1600.0);
    let lut = "Apply LUT: warm.cube";
    // drag the LUT row over the text row
    let from = button(&h, lut).rect.center();
    let to = button(&h, "Text: Hello").rect.center();
    h.drag(from, to);
    assert_eq!(h.app.project.order_of(c), vec![Row::Lut, Row::Text(t)]);
    assert_eq!(h.app.undo_text(), "Undo Move effect");
    // the keyboard: the text row is below now; focus it and press Alt+Up
    let pos = button(&h, "Text: Hello").rect.center();
    h.frame(vec![Event::PointerMoved(pos)]);
    h.idle(1);
    h.frame(vec![Event::Key {
        key: Key::ArrowUp,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::ALT,
    }]);
    h.idle(2);
    assert_eq!(h.app.project.order_of(c), vec![Row::Text(t), Row::Lut]);
    assert_eq!(h.app.undo_text(), "Undo Move text up");
}

// covers: REQ-302
// covers: REQ-303
#[test]
fn fixed_rows_follow_the_movable_ones_and_the_whole_video_mark_shows() {
    let (_d, mut app, c, _t) = setup();
    for clip in app
        .project
        .timeline
        .clips
        .iter()
        .map(|c| c.id)
        .collect::<Vec<_>>()
    {
        app.dispatch(
            Command::SetClipVolume {
                clip,
                volume: 40,
                muted: false,
            },
            now(),
        )
        .unwrap();
    }
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    let mut h = Harness::new(app, 1800.0, 1600.0);
    assert!(y_of(&h, "Apply LUT: warm.cube") < y_of(&h, "Volume"));
    // the mark sits next to the row
    assert!(h.has("whole video"), "{:#?}", h.labels());
    // the fixed row has no Move buttons
    assert!(!h.has("Move up: Volume"));
    click_button(&mut h, "Volume");
    assert_eq!(h.app.popups.instance, Some(Instance::ClipVolume(c)));
}

// covers: REQ-302
// covers: REQ-068
#[test]
fn at_the_smallest_window_every_row_can_be_reached() {
    let (_d, mut app, c, _t) = setup();
    for i in 0..6 {
        app.dispatch(Command::Seek(Micros(1_000_000)), now())
            .unwrap();
        app.dispatch(
            Command::AddText(TextItem::new(TextId(0), format!("Extra {i}"), Micros::ZERO)),
            now(),
        )
        .unwrap();
    }
    app.dispatch(Command::SelectClip(Some(c)), now()).unwrap();
    let mut h = Harness::new(app, 800.0, 500.0);
    h.idle(3);
    let lost = reachable(&h, true);
    assert!(lost.is_empty(), "{lost:#?}");
}
