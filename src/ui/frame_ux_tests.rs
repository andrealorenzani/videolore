// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 1.5.0: the three picture effects in the Effects list and menus, their
//! popups, the keyframe timeline, the handles on the preview and their priority.

use super::frame_popups::{KeyBar, MIN_BAR_WIDTH, key_range};
use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{EffectKind, Instance, Popup, Target};
use crate::base::pos::Pos;
use crate::frame::Shape;
use crate::text::{TextId, TextItem};
use crate::timeline::FrameTarget;
use eframe::egui::{Key, Modifiers, Pos2, Rect, pos2};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips, the playhead at 1 s.
fn loaded() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    (dir, app)
}

fn picture(h: &Harness) -> Rect {
    h.find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect
}

fn at(pic: Rect, fx: f32, fy: f32) -> Pos2 {
    pos2(pic.min.x + fx * pic.width(), pic.min.y + fy * pic.height())
}

fn apply(app: &mut AppState, effect: EffectKind, target: Target) {
    app.dispatch(Command::ApplyEffect { effect, target }, Instant::now())
        .unwrap();
}

// covers: REQ-130
// covers: REQ-129
#[test]
fn the_scale_row_opens_a_popup_that_offers_apply_then_the_keyframe_buttons() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.click_label("Scale");
    assert_eq!(
        h.app.popups.instance,
        Some(Instance::Scale(FrameTarget::Whole))
    );
    assert!(h.has("Scale is not applied here."), "{:#?}", h.labels());
    h.click_label("Apply");
    assert_eq!(
        h.app.project.timeline.scale_keys(FrameTarget::Whole).len(),
        1
    );
    h.idle(2);
    for l in ["Add keyframe", "Remove keyframe", "Remove effect"] {
        assert!(h.has(l), "{l} in {:#?}", h.labels());
    }
    // the popup stays open while the playhead moves
    h.app.dispatch(Command::Seek(s(3)), Instant::now()).unwrap();
    h.idle(2);
    assert!(h.app.popups.is_open(Popup::Instance));
    // Add keyframe at the playhead, then Remove keyframe (selected by the add)
    let n = h.app.history.undo_len();
    h.click_label("Add keyframe");
    assert_eq!(
        h.app.project.timeline.scale_keys(FrameTarget::Whole).len(),
        2
    );
    assert_eq!(h.app.history.undo_len(), n + 1);
    h.click_label("Remove keyframe");
    assert_eq!(
        h.app.project.timeline.scale_keys(FrameTarget::Whole).len(),
        1
    );
    // Remove effect
    h.click_label("Remove effect");
    assert!(
        h.app
            .project
            .timeline
            .scale_keys(FrameTarget::Whole)
            .is_empty()
    );
    assert!(!h.app.popups.is_open(Popup::Instance));
}

// covers: REQ-124
#[test]
fn the_change_aspect_popup_has_three_dropdowns_and_a_colour_and_a_remove_button() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.click_label("Change aspect");
    assert!(h.has("Apply"), "{:#?}", h.labels());
    h.click_label("Apply");
    h.idle(2);
    for l in ["Aspect", "Fit", "Empty space", "Colour", "Remove effect"] {
        assert!(h.has(l), "{l} in {:#?}", h.labels());
    }
    // the shape dropdown lists exactly 4:3, 16:9 and 9:16
    h.click_label("16:9");
    h.idle(2);
    for o in ["4:3", "9:16"] {
        assert!(h.has(o), "{o} in {:#?}", h.labels());
    }
    h.click_label("9:16");
    assert_eq!(
        h.app.project.timeline.frame_aspect.unwrap().shape,
        Shape::Tall9x16
    );
    assert_eq!(h.app.undo_text(), "Undo Change aspect");
    // the fit dropdown
    h.click_label("Fit on width");
    h.idle(2);
    assert!(h.has("Fit on height"), "{:#?}", h.labels());
    h.click_label("Fit on height");
    // the fill dropdown
    h.click_label("Fit empty with colour");
    h.idle(2);
    assert!(h.has("Fit empty with blurred video"), "{:#?}", h.labels());
    h.click_label("Fit empty with blurred video");
    assert_eq!(
        h.app.project.timeline.frame_aspect.unwrap().fill,
        crate::frame::Fill::Blur
    );
    h.idle(2);
    assert!(!h.has("Colour"));
    h.click_label("Remove effect");
    assert!(h.app.project.timeline.frame_aspect.is_none());
}

// covers: REQ-127
#[test]
fn a_corner_handle_scales_around_the_opposite_corner_in_one_undo_step() {
    let (_d, mut app) = loaded();
    apply(&mut app, EffectKind::Scale, Target::WholeVideo);
    // the keyframe at the start is the selected one
    app.popups.key = Some(Micros::ZERO);
    let mut h = Harness::new(app, 1280.0, 1000.0);
    let pic = picture(&h);
    let n = h.app.history.undo_len();
    // the bottom right corner to the middle of the picture: half size
    h.drag(at(pic, 1.0, 1.0), at(pic, 0.5, 0.5));
    let keys = h.app.project.timeline.scale_keys(FrameTarget::Whole);
    assert_eq!(keys.len(), 1);
    assert!(
        (keys[0].v.permille as i32 - 500).abs() <= 10,
        "{:?}",
        keys[0].v
    );
    assert_eq!(h.app.history.undo_len(), n + 1);
    // the top left corner stayed: the picture's rectangle starts at (0, 0) of the frame
    let (r, _) = h.app.picture_rect().unwrap();
    assert!(r.0.abs() < 0.02 && r.1.abs() < 0.02, "{r:?}");
    assert!((r.2 - 0.5).abs() < 0.02, "{r:?}");
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.app.project.timeline.scale_keys(FrameTarget::Whole)[0]
            .v
            .permille,
        1000
    );
}

// covers: REQ-128
#[test]
fn the_target_moves_the_picture_by_the_dragged_fraction_and_stops_at_the_edge() {
    let (_d, mut app) = loaded();
    apply(&mut app, EffectKind::Move, Target::WholeVideo);
    app.popups.key = Some(Micros::ZERO);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let n = h.app.history.undo_len();
    h.drag(at(pic, 0.5, 0.5), at(pic, 0.6, 0.5));
    let p = h.app.project.timeline.move_keys(FrameTarget::Whole)[0].v;
    assert!((i32::from(p.x) - 6000).abs() <= 100, "{p:?}");
    assert!((i32::from(p.y) - 5000).abs() <= 100, "{p:?}");
    assert_eq!(h.app.history.undo_len(), n + 1);
    // past the edge: stops at the edge of the frame
    let c = at(pic, 0.6, 0.5);
    h.drag(c, c + eframe::egui::vec2(5000.0, 0.0));
    assert_eq!(
        h.app.project.timeline.move_keys(FrameTarget::Whole)[0].v.x,
        10_000
    );
}

// covers: REQ-128
// covers: REQ-127
#[test]
fn text_boxes_come_before_the_handles_and_the_handles_before_the_picture() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), "HHHH".into(), Micros::ZERO)),
        now,
    )
    .unwrap();
    let id = app.selection.primary_text().unwrap();
    // the text sits exactly on the Move target at the picture's centre
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![id],
            place: Pos { x: 5000, y: 5000 },
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ClosePopup(Popup::Text), now).unwrap();
    apply(&mut app, EffectKind::Move, Target::WholeVideo);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    h.drag(at(pic, 0.5, 0.5), at(pic, 0.3, 0.3));
    assert_eq!(
        h.app.project.texts.get(id).map(|t| t.place.x < 4000),
        Some(true),
        "the text moved"
    );
    assert_eq!(
        h.app.project.timeline.move_keys(FrameTarget::Whole)[0].v,
        crate::frame::CENTRE,
        "the picture stayed"
    );
}

// covers: REQ-130
#[test]
fn a_clip_with_scale_or_move_has_markers_that_open_the_popup() {
    let (_d, mut app) = loaded();
    let c1 = app.project.timeline.clips[1].id;
    apply(&mut app, EffectKind::Scale, Target::Clip(c1));
    apply(&mut app, EffectKind::Move, Target::Clip(c1));
    app.popups.close(Popup::Instance);
    let ms = markers::layout(&app.project, 50.0, 0.0, (0.0, 5000.0));
    let scale = ms
        .iter()
        .find(|m| m.kind == markers::MarkerKind::Scale)
        .unwrap();
    let moved = ms
        .iter()
        .find(|m| m.kind == markers::MarkerKind::Move)
        .unwrap();
    assert!(
        scale.tooltip.starts_with("Scale on \"b.mp4\": 1 keyframe."),
        "{}",
        scale.tooltip
    );
    assert!(moved.tooltip.contains("Move") && moved.x > scale.x);
    assert!((scale.x - (250.0 + markers::SCALE_OFFSET)).abs() < 0.01);
    app.dispatch(Command::OpenInstance(scale.instance), Instant::now())
        .unwrap();
    assert_eq!(
        app.popups.instance,
        Some(Instance::Scale(FrameTarget::Clip(c1)))
    );
    assert_eq!(app.popup_title(Popup::Instance), "Scale — b.mp4");
    // a clip without them has none
    let none = markers::layout(
        &state(FakeDialogs::default()).project,
        50.0,
        0.0,
        (0.0, 5000.0),
    );
    assert!(none.is_empty());
}

// covers: REQ-129
#[test]
fn the_key_timeline_fills_the_popup_width_at_any_clip_length() {
    for (from, to) in [(s(0), s(1)), (s(0), s(3600)), (s(2), s(2) + Micros(40_000))] {
        for width in [100.0, 240.0, 600.0] {
            let bar = KeyBar::new(from, to, width);
            assert_eq!(bar.width, width.max(MIN_BAR_WIDTH));
            assert_eq!(bar.x_of(from), Some(0.0));
            assert!((bar.x_of(to).unwrap() - bar.width).abs() < 0.01);
            assert_eq!(bar.time_at(0.0), from);
            assert_eq!(bar.time_at(bar.width * 2.0), to);
            // a point in the middle round-trips
            let mid = from + Micros((to - from).0 / 2);
            let x = bar.x_of(mid).unwrap();
            assert!((bar.time_at(x) - mid).0.abs() <= 1 + (to - from).0 / bar.width as i64);
        }
    }
    let bar = KeyBar::new(s(2), s(6), 400.0);
    assert_eq!(
        bar.x_of(s(1)),
        None,
        "a keyframe outside the kept part is hidden"
    );
    assert_eq!(bar.x_of(s(7)), None);
    let keys = [s(2), s(4)];
    assert_eq!(bar.key_at(201.0, &keys), Some(s(4)));
    assert_eq!(bar.key_at(100.0, &keys), None);
    // a range of no length still has a width
    assert!(KeyBar::new(s(1), s(1), 300.0).x_of(s(1)).is_some());
}

// covers: REQ-129
#[test]
fn the_range_of_a_clip_popup_is_its_kept_part() {
    let (_d, app) = loaded();
    let c1 = app.project.timeline.clips[1].id;
    assert_eq!(
        // the whole-video popup: the first clip's keys on a timeline as long as the longest clip
        key_range(&app, FrameTarget::Whole),
        Some((s(0), s(5), s(0)))
    );
    assert_eq!(
        key_range(&app, FrameTarget::Clip(c1)),
        Some((s(0), s(5), s(5)))
    );
    assert_eq!(
        key_range(&app, FrameTarget::Clip(crate::timeline::ClipId(99))),
        None
    );
}
