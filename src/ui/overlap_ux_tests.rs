// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 2.3.0: the Add media button (REQ-324), the Change aspect of a clip with its
//! Fit and Empty space (REQ-326), the start screen choices (REQ-295), the overlap lane, the
//! overlap box in the preview and the removal warning (REQ-325, REQ-327, REQ-329, REQ-331).

use super::overlap_lane::{self, LANE_HEIGHT};
use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Modal, Popup};
use crate::frame::{Fill, Fit, Shape};
use eframe::egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2};
use std::path::{Path, PathBuf};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips, the playhead at 1 s and a picture on screen.
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

fn logo(dir: &Path) -> PathBuf {
    let p = dir.join("logo.png");
    std::fs::write(&p, crate::picture::tests::header(400, 200)).unwrap();
    p
}

fn add(app: &mut AppState, file: &Path, at: Micros) {
    app.dispatch(Command::Seek(at), Instant::now()).unwrap();
    app.dispatch(
        Command::AddOverlap(Some(file.to_path_buf())),
        Instant::now(),
    )
    .unwrap();
}

// covers: REQ-324
#[test]
fn the_add_media_button_is_the_last_item_of_the_media_section_and_runs_the_import_command() {
    let dir = tempfile::tempdir().unwrap();
    let extra = dir.path().join("c.mp4");
    std::fs::write(&extra, b"c.mp4").unwrap();
    let mut app = state(FakeDialogs {
        media: vec![extra],
        ..Default::default()
    });
    import(&mut app, files(dir.path()));
    let mut h = Harness::new(app, 1280.0, 800.0);
    let button = h.find(media::ADD_MEDIA).expect("the button");
    assert!(button.interactive && !button.disabled);
    // the media section: nothing of its list is below the button
    let section = h.cache.sections.unwrap().media;
    let names: Vec<_> = h
        .nodes
        .iter()
        .filter(|n| section.contains_rect(n.rect) && n.rect.min.y > section.min.y + 30.0)
        .collect();
    assert!(
        names
            .iter()
            .all(|n| n.rect.max.y <= button.rect.max.y + 1.0),
        "{names:#?}"
    );
    // the same action as File > Import media…: the same command, the same dialog
    assert_eq!(
        crate::ui::actions::ActionId::Import.command(&h.app),
        Command::PickImport
    );
    let before = h.app.project.media.len();
    h.click(button.rect.center());
    crate::app::tests::settle(&mut h.app);
    assert_eq!(
        h.app.project.media.len(),
        before + 1,
        "the file the dialog chose is imported"
    );
}

// covers: REQ-326
#[test]
fn a_clip_has_its_own_fit_and_empty_space_like_the_whole_video_and_a_new_shape_keeps_them() {
    let (_d, mut app) = loaded();
    let c = app.project.timeline.clips[0].id;
    app.dispatch(Command::SelectClip(Some(c)), Instant::now())
        .unwrap();
    app.dispatch(Command::OpenPopup(Popup::ClipDetails), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 1000.0);
    assert!(h.has("Aspect of this clip"), "{:#?}", h.labels());
    // no own aspect: no Fit and no Empty space yet
    assert!(!h.has("Fit on width") && !h.has("Empty space"));
    let n = h.app.history.undo_len();
    // the first shape: the default aspect (Fit on width, black empty space)
    h.click_label("9:16");
    let a = h.app.project.timeline.own_aspect(c).unwrap();
    assert_eq!((a.shape, a.fit), (Shape::Tall9x16, Fit::Width));
    assert!(matches!(a.fill, Fill::Colour(_)));
    assert_eq!(h.app.history.undo_len(), n + 1);
    h.idle(2);
    for l in ["Aspect", "Fit", "Empty space", "Colour"] {
        assert!(h.has(l), "{l} in {:#?}", h.labels());
    }
    // the Fit
    h.click_label("Fit on width");
    h.idle(2);
    h.click_label("Fit on height");
    assert_eq!(
        h.app.project.timeline.own_aspect(c).unwrap().fit,
        Fit::Height
    );
    // the Empty space: blurred, with its level
    h.click_label("Fit empty with colour");
    h.idle(2);
    h.click_label("Fit empty with blurred video");
    h.idle(2);
    let a = h.app.project.timeline.own_aspect(c).unwrap();
    assert_eq!(a.fill, Fill::Blur);
    assert!(h.has("Blur"), "{:#?}", h.labels());
    // a later shape keeps the Fit and the Empty space
    h.click_label("4:3");
    let a = h.app.project.timeline.own_aspect(c).unwrap();
    assert_eq!(
        (a.shape, a.fit, a.fill),
        (Shape::Wide4x3, Fit::Height, Fill::Blur)
    );
    // one step per choice; Off takes the aspect away
    assert_eq!(h.app.history.undo_len(), n + 4);
    h.click_label("Off");
    assert!(h.app.project.timeline.own_aspect(c).is_none());
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(
        h.app.project.timeline.own_aspect(c).unwrap().shape,
        Shape::Wide4x3
    );
    // the other clip has none
    let other = h.app.project.timeline.clips[1].id;
    assert!(h.app.project.timeline.own_aspect(other).is_none());
}

// covers: REQ-295
#[test]
fn the_settings_window_offers_two_start_screen_choices_and_never_is_not_one_of_them() {
    let (_d, mut app) = loaded();
    app.dispatch(Command::OpenPopup(Popup::Settings), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 1000.0);
    // the section that holds it is collapsed, lower in the window: scroll to it and open it
    let window = h
        .nodes
        .iter()
        .find(|n| format!("{:?}", n.role) == "Window" && n.label == "Settings")
        .expect("the Settings window")
        .rect;
    let scroll = |h: &mut Harness, dy: f32| {
        h.frame(vec![
            Event::PointerMoved(window.center()),
            Event::MouseWheel {
                unit: eframe::egui::MouseWheelUnit::Point,
                delta: eframe::egui::vec2(0.0, dy),
                phase: eframe::egui::TouchPhase::Move,
                source: eframe::egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            },
        ]);
        h.idle(2);
    };
    scroll(&mut h, -600.0);
    h.click_label("Appearance and start-up");
    h.idle(2);
    scroll(&mut h, -600.0);
    assert!(h.has("At every start"), "{:#?}", h.labels());
    assert!(h.has("Once for each version"));
    assert!(!h.has("Never"), "no Never choice: {:#?}", h.labels());
    h.click_label("Once for each version");
    assert_eq!(
        h.app.settings.show_splash,
        crate::settings::SplashOption::OncePerVersion
    );
}

/// The point of a bar of the lane: the middle of the overlap's first part.
fn bar_point(h: &Harness, id: u64, row: usize) -> Pos2 {
    let g = h.cache.timeline.expect("the timeline was drawn");
    let b = overlap_lane::bars(&h.app)
        .into_iter()
        .find(|b| b.id == id)
        .expect("the bar");
    let first = overlap_lane::visible_rows(
        overlap_lane::row_count(&overlap_lane::bars(&h.app)),
        h.cache.overlap_scroll,
    )
    .0;
    pos2(
        g.x_of(b.start) + 12.0,
        g.overlaps.min.y + 1.0 + (row - first) as f32 * overlap_lane::ROW_H + 5.0,
    )
}

// covers: REQ-325
#[test]
fn overlaps_are_bars_of_one_shared_lane_that_shows_three_rows_and_scrolls_for_more() {
    let (d, mut app) = loaded();
    let file = logo(d.path());
    // four overlaps that overlap in time: four sub-rows
    for _ in 0..4 {
        add(&mut app, &file, s(1));
    }
    let ids = app.project.timeline.overlap_ids();
    assert_eq!(ids.len(), 4);
    let mut h = Harness::new(app, 1280.0, 1000.0);
    h.idle(2);
    let g = h.cache.timeline.unwrap();
    // the lane is the height of three rows, whatever the number of overlaps
    assert_eq!(g.overlaps.height(), LANE_HEIGHT);
    let bars = overlap_lane::bars(&h.app);
    assert_eq!(overlap_lane::row_count(&bars), 4);
    // the first overlap is on the first row: a click selects it (and its clip)
    h.click(bar_point(&h, ids[0], 0));
    assert_eq!(h.app.selected_overlap, Some(ids[0]));
    assert_eq!(
        h.app.selected_clip,
        Some(h.app.project.timeline.clips[0].id)
    );
    // the fourth is on the fourth row: it is not drawn until the lane scrolls to it
    let fourth = bars.iter().find(|b| b.row == 3).unwrap().id;
    h.cache.overlap_scroll = 1;
    h.idle(2);
    h.click(bar_point(&h, fourth, 3));
    assert_eq!(h.app.selected_overlap, Some(fourth));
    // a scroll past the end is brought back
    h.cache.overlap_scroll = 9;
    h.idle(2);
    assert_eq!(h.cache.overlap_scroll, 1);
    // without overlaps there is no lane
    for id in ids {
        h.app
            .dispatch(Command::RemoveOverlap(id), Instant::now())
            .unwrap();
    }
    h.idle(2);
    assert_eq!(h.cache.timeline.unwrap().overlaps.height(), 0.0);
}

// covers: REQ-325, REQ-329
#[test]
fn dragging_an_end_of_a_bar_changes_the_time_and_deleting_the_clip_shows_the_one_popup() {
    let (d, mut app) = loaded();
    let file = logo(d.path());
    add(&mut app, &file, s(1));
    add(&mut app, &file, s(3));
    let ids = app.project.timeline.overlap_ids();
    let clip = app.project.timeline.clips[0].id;
    let mut h = Harness::new(app, 1280.0, 1000.0);
    h.idle(2);
    let g = h.cache.timeline.unwrap();
    // the end of the first overlap (1 s to 4 s) dragged to 4.5 s
    let b = overlap_lane::bars(&h.app)
        .into_iter()
        .find(|b| b.id == ids[0])
        .unwrap();
    let y = g.overlaps.min.y + 1.0 + overlap_lane::ROW_H * b.row as f32 + 5.0;
    let from = pos2(g.x_of(b.end) - 2.0, y);
    let to = pos2(g.x_of(b.end + Micros(500_000)) - 2.0, y);
    let n = h.app.history.undo_len();
    h.drag(from, to);
    let o = h.app.project.timeline.overlaps_of(clip)[0].clone();
    assert!((o.end().as_secs_f64() - 4.5).abs() < 0.1, "{:?}", o.end());
    assert_eq!(h.app.history.undo_len(), n + 1);
    // deleting the clip: one popup for both, the text names the count, Cancel keeps all
    h.app
        .dispatch(Command::SelectClip(Some(clip)), Instant::now())
        .unwrap();
    h.app.dispatch(Command::Delete, Instant::now()).unwrap();
    assert_eq!(h.app.modal, Some(Modal::RemoveOverlaps(2)));
    h.idle(2);
    assert!(
        h.has(&crate::app::overlap::removal_text(2)),
        "{:#?}",
        h.labels()
    );
    h.click_label("Cancel");
    assert_eq!(h.app.modal, None);
    assert_eq!(h.app.project.timeline.clips.len(), 2);
    assert_eq!(h.app.project.timeline.overlap_ids().len(), 2);
    // OK does the edit
    h.app.dispatch(Command::Delete, Instant::now()).unwrap();
    h.idle(2);
    h.click_label("OK");
    assert_eq!(h.app.project.timeline.clips.len(), 1);
    assert!(!h.app.project.timeline.any_overlap());
}

fn drag_with(h: &mut Harness, from: Pos2, to: Pos2, mods: Modifiers) {
    let ev = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: mods,
    };
    let run = |h: &mut Harness, events| {
        h.frame_with(RawInput {
            screen_rect: Some(h.screen()),
            events,
            ..Default::default()
        });
    };
    // the modifier is held for the whole drag
    run(
        h,
        vec![Event::ModifiersChanged(mods), Event::PointerMoved(from)],
    );
    run(h, vec![ev(from, true)]);
    for i in 1..=5 {
        let p = from + (to - from) * (i as f32 / 5.0);
        run(h, vec![Event::PointerMoved(p)]);
    }
    run(h, vec![ev(to, false)]);
    run(h, vec![Event::ModifiersChanged(Modifiers::NONE)]);
    h.idle(2);
}

/// The screen rectangle of the selected overlap's picture in the preview.
fn overlap_screen_rect(h: &Harness, pic: Rect) -> Rect {
    let frame = h.app.output_size().unwrap();
    let (_, o) = h
        .app
        .project
        .timeline
        .find_overlap(h.app.selected_overlap.unwrap())
        .unwrap();
    let (x, y, w, hh) = o.rect(frame);
    placement::to_screen(
        Rect::from_min_size(
            pos2(x as f32, y as f32),
            eframe::egui::vec2(w as f32, hh as f32),
        ),
        pic,
        frame,
    )
}

fn preview_rect(h: &Harness) -> Rect {
    h.find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect
}

// covers: REQ-327
#[test]
fn a_drag_in_the_preview_moves_the_box_an_edge_keeps_the_ratio_and_ctrl_changes_the_shape() {
    let (d, mut app) = loaded();
    add(&mut app, &logo(d.path()), s(1));
    let id = app.selected_overlap.unwrap();
    let mut h = Harness::new(app, 1280.0, 1000.0);
    h.idle(2);
    let pic = preview_rect(&h);
    let r = overlap_screen_rect(&h, pic);
    let n = h.app.history.undo_len();
    let find = |h: &Harness| h.app.project.timeline.find_overlap(id).unwrap().1.clone();
    let before = find(&h);
    // the body: the centre moves, the size stays; one undo step
    h.drag(r.center(), r.center() + eframe::egui::vec2(80.0, 30.0));
    let moved = find(&h);
    assert!(
        moved.place.x > before.place.x && moved.place.y > before.place.y,
        "{moved:?}"
    );
    assert_eq!(
        (moved.width, moved.height, moved.reshaped),
        (before.width, before.height, false)
    );
    assert_eq!(h.app.history.undo_len(), n + 1);
    // the east edge without a modifier: bigger, the ratio of the file (2:1) is kept
    h.idle(2);
    let r = overlap_screen_rect(&h, pic);
    h.drag(
        pos2(r.max.x - 1.0, r.center().y),
        pos2(r.max.x + 40.0, r.center().y),
    );
    let wide = find(&h);
    assert!(!wide.reshaped);
    let k = h.app.output_size().unwrap();
    let (_, _, w, hh) = wide.rect(k);
    assert!(w > before.rect(k).2, "{wide:?}");
    assert!(
        (f64::from(w) / f64::from(hh) - 2.0).abs() < 0.1,
        "{w} x {hh}"
    );
    assert_eq!(h.app.history.undo_len(), n + 2);
    // one undo restores the box
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(find(&h).width, moved.width);
    // Ctrl on an edge: the shape changes (the south edge down: taller, the width the same)
    h.idle(2);
    let r = overlap_screen_rect(&h, pic);
    drag_with(
        &mut h,
        pos2(r.center().x, r.max.y - 1.0),
        pos2(r.center().x, r.max.y + 40.0),
        Modifiers::CTRL,
    );
    let tall = find(&h);
    assert!(tall.reshaped, "{tall:?}");
    assert!(
        tall.height > moved.height && tall.width == moved.width,
        "{tall:?}"
    );
    // a click on the picture over a box that is not selected selects it
    h.app
        .dispatch(Command::SelectOverlap(None), Instant::now())
        .unwrap();
    h.idle(2);
    let r = overlap_screen_rect_of(&h, pic, id);
    h.click(r.center());
    assert_eq!(h.app.selected_overlap, Some(id));
}

fn overlap_screen_rect_of(h: &Harness, pic: Rect, id: u64) -> Rect {
    let frame = h.app.output_size().unwrap();
    let (_, o) = h.app.project.timeline.find_overlap(id).unwrap();
    let (x, y, w, hh) = o.rect(frame);
    placement::to_screen(
        Rect::from_min_size(
            pos2(x as f32, y as f32),
            eframe::egui::vec2(w as f32, hh as f32),
        ),
        pic,
        frame,
    )
}

// covers: REQ-331, REQ-320, REQ-321
#[test]
fn the_applied_effects_widget_lists_the_overlap_in_the_picture_group_and_it_can_be_moved() {
    let (d, mut app) = loaded();
    let file = logo(d.path());
    add(&mut app, &file, s(1));
    add(&mut app, &file, s(2));
    let clip = app.project.timeline.clips[0].id;
    let ids = app.project.timeline.overlap_ids();
    app.dispatch(Command::SelectClip(Some(clip)), Instant::now())
        .unwrap();
    let rows = app.applied().unwrap();
    // on top by default (the later one first), named after the file, with its time on the timeline
    assert_eq!(rows.rows[0].name, "Overlap: logo.png");
    assert_eq!(
        rows.rows[0].row,
        Some(crate::effect_order::Row::Overlap(ids[1]))
    );
    assert_eq!(rows.rows[0].range, Some((s(2), s(5))));
    assert_eq!(rows.movable(), 2);
    // a click selects it
    assert_eq!(
        rows.rows[1].open,
        vec![
            Command::SelectClip(Some(clip)),
            Command::SelectOverlap(Some(ids[0]))
        ]
    );
    // it moves inside the group: one named undo step
    let mv = Command::MoveRow {
        clip,
        row: crate::effect_order::Row::Overlap(ids[1]),
        mv: crate::app::RowMove::Down,
    };
    assert!(app.is_enabled(&mv));
    app.dispatch(mv, Instant::now()).unwrap();
    assert_eq!(app.undo_text(), "Undo Move overlap down");
    let rows = app.applied().unwrap();
    assert_eq!(
        rows.rows[0].row,
        Some(crate::effect_order::Row::Overlap(ids[0]))
    );
    // a project with the order changed is drawn by the overlay stage or the step builder
    assert_eq!(app.project.changed_orders().len(), 1);
}
