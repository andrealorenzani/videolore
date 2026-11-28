// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The 3.2.0 aids of the timeline (ruler marks, pointer line, picture strips) and the
//! import of a project as media, in the window.

use super::ux_tests::{Harness, loaded};
use super::*;
use crate::app::imports_tests::runner;
use crate::app::tests::{FakeDialogs, state_with};
use crate::app::{Modal, Popup};
use crate::nested::tests::write_project;
use eframe::egui::Event;
use eframe::epaint::Shape;

/// The window with two 5 s clips, at `zoom` points per second.
fn window(zoom: f32) -> (tempfile::TempDir, Harness) {
    let (dir, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.app
        .dispatch(Command::SetZoom(zoom), Instant::now())
        .unwrap();
    h.idle(3);
    (dir, h)
}

/// Every shape of one frame.
fn shapes(h: &mut Harness) -> Vec<Shape> {
    fn flat(s: Shape, out: &mut Vec<Shape>) {
        match s {
            Shape::Vec(v) => v.into_iter().for_each(|s| flat(s, out)),
            s => out.push(s),
        }
    }
    let raw = eframe::egui::RawInput {
        screen_rect: Some(h.screen()),
        ..Default::default()
    };
    let now = Instant::now();
    let (app, cache) = (&mut h.app, &mut h.cache);
    let mut out = h.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
    let mut all = Vec::new();
    for c in std::mem::take(&mut out.shapes) {
        flat(c.shape, &mut all);
    }
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    all
}

// covers: REQ-367
#[test]
fn the_ruler_draws_marks_one_per_frame_when_zoomed_in_under_the_labels() {
    // the highest zoom: 12 points per frame at 30 frames a second
    // and zoomed out (to the whole video) one per second
    for (zoom, per_mark) in [(360.0f32, 1.0f32 / 30.0), (0.0, 1.0)] {
        let (_d, mut h) = window(zoom);
        let spacing = h.app.zoom * per_mark;
        let painted = shapes(&mut h);
        let g = h.cache.timeline.unwrap();
        let top = g.lane.min.y - timeline::RULER_H;
        let marks_top = g.lane.min.y - timeline::MARK_H;
        let mut xs: Vec<f32> = painted
            .iter()
            .filter_map(|s| match s {
                Shape::LineSegment { points, .. }
                    if (points[0].y - marks_top).abs() < 0.1
                        && (points[1].y - g.lane.min.y).abs() < 0.1 =>
                {
                    Some(points[0].x)
                }
                _ => None,
            })
            .collect();
        xs.sort_by(f32::total_cmp);
        assert!(xs.len() > 10, "{}", xs.len());
        for w in xs.windows(2) {
            assert!((w[1] - w[0] - spacing).abs() < 0.1, "{spacing}: {w:?}");
        }
        // the labels stand above the marks
        let labels: Vec<Rect> = painted
            .iter()
            .filter_map(|s| match s {
                Shape::Text(t) if (t.pos.y - top).abs() < 0.1 => {
                    Some(t.galley.rect.translate(t.pos.to_vec2()))
                }
                _ => None,
            })
            .collect();
        assert!(!labels.is_empty());
        assert!(
            labels.iter().all(|r| r.max.y <= marks_top + 0.01),
            "{labels:?} {marks_top}"
        );
    }
}

// covers: REQ-368
#[test]
fn a_line_follows_the_pointer_over_the_timeline_where_a_click_acts() {
    let (_d, mut h) = window(150.0);
    let zoom = h.app.zoom;
    assert_eq!(h.cache.pointer_line, None);
    let g = h.cache.timeline.unwrap();
    // over a clip and over the ruler
    for y in [g.video.center().y, g.lane.min.y - 5.0] {
        let p = pos2(g.x0 + 150.0, y);
        h.frame(vec![Event::PointerMoved(p)]);
        h.idle(1);
        assert_eq!(h.cache.pointer_line, Some(p.x));
    }
    let painted = shapes(&mut h);
    let x = g.x0 + 150.0;
    assert!(painted.iter().any(|s| matches!(s,
        Shape::LineSegment { points, stroke } if points[0].x == x && points[1].x == x
            && stroke.width == 1.0 && points[1].y > g.music.min.y)));
    // a click at the same point sets the time under the line, within one frame
    h.click(pos2(x, g.video.center().y));
    let under = f64::from(150.0 / zoom);
    let off = (h.app.playhead.as_secs_f64() - under).abs();
    assert!(off <= 1.0 / 30.0, "{off}");
    // outside the timeline there is no line
    h.frame(vec![Event::PointerMoved(pos2(600.0, 100.0))]);
    h.idle(1);
    assert_eq!(h.cache.pointer_line, None);
    h.frame(vec![Event::PointerGone]);
    h.idle(1);
    assert_eq!(h.cache.pointer_line, None);
}

fn thumb(k: &crate::strips::Key) -> crate::strips::Thumb {
    crate::strips::Thumb {
        width: k.width,
        height: k.height,
        rgba: vec![200; (k.width * k.height * 4) as usize],
    }
}

// covers: REQ-369
#[test]
fn each_clip_shows_snapshots_in_its_lower_half_from_its_start() {
    let (_d, mut h) = window(150.0);
    // as if the snapshots were made already
    let keys: Vec<_> = h.cache.strips.iter().map(|s| s.2).collect();
    assert!(!keys.is_empty());
    for k in &keys {
        h.app.strips.insert(*k, thumb(k));
    }
    h.idle(2);
    let g = h.cache.timeline.unwrap();
    let clips = h.app.project.timeline.clips.clone();
    for (i, c) in clips.iter().enumerate() {
        let start = g.x_of(h.app.project.timeline.start_of(i));
        let end = g.x_of(h.app.project.timeline.start_of(i + 1));
        let here: Vec<Rect> = h
            .cache
            .strips
            .iter()
            .filter(|s| s.0 == c.id)
            .map(|s| s.1)
            .collect();
        assert!(!here.is_empty());
        assert!(
            (here[0].min.x - start).abs() < 0.01,
            "from the clip's start"
        );
        for r in &here {
            assert!(r.min.y >= g.video.center().y - 0.01 && r.max.y <= g.video.max.y);
            assert!(r.max.x <= end + 0.01, "only what fits");
        }
        for w in here.windows(2) {
            assert!(w[1].min.x - w[0].max.x >= crate::strips::GAP - 0.01);
        }
    }
    assert!(!h.cache.strip_textures.is_empty());
    let of_first = |h: &Harness| {
        let id = h.app.project.timeline.clips[0].id;
        h.cache.strips.iter().filter(|s| s.0 == id).count()
    };
    h.app.selected_media = Some(h.app.project.media[0].id);
    let before = of_first(&h);
    // a longer video zoomed out to the whole of it: fewer in the same clip
    for _ in 0..20 {
        h.app
            .dispatch(Command::Append(None), Instant::now())
            .unwrap();
    }
    h.app
        .dispatch(Command::SetZoom(0.0), Instant::now())
        .unwrap();
    h.idle(3);
    assert!(of_first(&h) < before, "{before} {}", of_first(&h));
    // a file that is missing has none
    h.app.project.media[0].missing = true;
    h.idle(2);
    let first = h.app.project.timeline.clips[0].id;
    assert!(h.cache.strips.iter().all(|s| s.0 != first));
}

/// Turns the wheel down at `at`.
fn scroll(h: &mut Harness, at: Pos2) {
    h.frame(vec![
        Event::PointerMoved(at),
        Event::MouseWheel {
            unit: eframe::egui::MouseWheelUnit::Point,
            delta: eframe::egui::vec2(0.0, -2000.0),
            phase: eframe::egui::TouchPhase::Move,
            source: eframe::egui::MouseWheelSource::Wheel,
            modifiers: eframe::egui::Modifiers::NONE,
        },
    ]);
    h.idle(10);
}

// covers: REQ-370
#[test]
fn the_settings_switch_removes_the_snapshots_and_brings_them_back() {
    let (_d, mut h) = window(150.0);
    assert!(!h.app.strips.wanted().is_empty());
    h.app
        .dispatch(Command::OpenSettings, Instant::now())
        .unwrap();
    h.idle(2);
    let panel = h
        .nodes
        .iter()
        .find(|n| format!("{:?}", n.role) == "Window" && n.label == "Settings")
        .expect("the Settings window")
        .rect;
    scroll(&mut h, panel.center());
    h.click_label("Appearance and start-up");
    h.idle(2);
    scroll(&mut h, panel.center());
    h.click_label("Show picture strips on the timeline");
    h.idle(2);
    assert!(!h.app.settings.show_picture_strips);
    assert!(h.cache.strips.is_empty() && h.cache.strip_textures.is_empty());
    assert!(h.app.strips.wanted().is_empty());
    h.click_label("Show picture strips on the timeline");
    h.idle(2);
    assert!(h.app.settings.show_picture_strips);
    assert!(!h.cache.strips.is_empty());
    assert!(h.app.popups.is_open(Popup::Settings));
}

// covers: REQ-372, REQ-371
#[test]
fn the_file_menu_imports_a_project_after_a_question() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_project(dir.path(), "wedding", &[], true);
    let dialogs = FakeDialogs {
        open: Some(source.clone()),
        ..Default::default()
    };
    let mut app = state_with(dialogs, runner(false));
    app.work_root = dir.path().join("work");
    let mut h = Harness::new(app, 1280.0, 800.0);
    for ok in [false, true] {
        h.click_label("File");
        h.idle(2);
        h.click_label("Import project as media…");
        h.idle(2);
        assert_eq!(h.app.modal, Some(Modal::ImportProject(source.clone())));
        assert!(h.has(crate::app::imports::IMPORT_PROJECT_QUESTION));
        h.click_label(if ok { "OK" } else { "Cancel" });
        assert_eq!(h.app.modal, None);
        if !ok {
            assert!(h.app.project.media.is_empty());
        }
    }
    assert_eq!(h.app.project.media.len(), 1);
    assert_eq!(h.app.project.media[0].name, "wedding.vlproj");
}
