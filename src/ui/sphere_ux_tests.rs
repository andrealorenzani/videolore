// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 1.2.6: boxes and corner handles on the preview, the effect command
//! group, the 360° look-around, Set POV, POV markers and the "Treat as 360° video" switch.
//! Real pointer events at the positions of the widgets (AccessKit tree and shapes).

use super::ux_tests::{Harness, Node};
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Instance, Popup};
use crate::base::pos::Pos;
use eframe::egui::accesskit::Role;
use eframe::egui::{Event, Key, Modifiers, PointerButton, RawInput, Shape};

/// A 360 clip then a normal clip, a frame on screen, the playhead at 1 s.
fn sphere_app() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    app.project.media[0].format.spherical = Some(true);
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::Seek(Micros(1_000_000)), now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    (dir, app)
}

/// One subtitle line, centred on the picture, and its box drawn.
fn with_line(app: &mut AppState) -> crate::subtitles::LineId {
    let now = Instant::now();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let id = app.selection.primary_line().unwrap();
    // these tests are about a line fixed to the picture, not to the sphere
    app.project
        .subtitles
        .lines
        .iter_mut()
        .for_each(|l| l.anchor = None);
    app.dispatch(Command::SetSubtitleText(id, "HHHH".into()), now)
        .unwrap();
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![id],
            pos: Some(Pos { x: 5000, y: 5000 }),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ClearSelection, now).unwrap();
    id
}

/// The corner handles drawn in the last frame (7 px white squares).
fn handles(h: &mut Harness) -> Vec<Rect> {
    let raw = RawInput {
        screen_rect: Some(h.screen()),
        ..Default::default()
    };
    let now = Instant::now();
    let (app, cache) = (&mut h.app, &mut h.cache);
    let mut out = h.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
    let found: Vec<Rect> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Rect(r)
                if (r.rect.width() - 7.0).abs() < 0.01
                    && (r.rect.height() - 7.0).abs() < 0.01
                    && r.fill == Color32::WHITE =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    found
}

/// A wheel turn of `y` points (positive: away from the user, zooms in).
fn wheel(y: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, y),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

fn picture(h: &Harness) -> Rect {
    h.find("Drag to look around")
        .or_else(|| h.find("Drag a subtitle to move it"))
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect
}

// covers: REQ-096, REQ-097
#[test]
fn boxes_are_drawn_with_handles_only_when_the_option_is_on() {
    let (_d, mut app) = sphere_app();
    with_line(&mut app);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let on = handles(&mut h);
    assert_eq!(on.len(), 4, "four corner handles on the line's box");
    h.app.settings.show_boxes = false;
    h.idle(2);
    assert!(handles(&mut h).is_empty(), "no handle without boxes");
    h.app.settings.show_boxes = true;
    h.idle(2);
    assert_eq!(handles(&mut h).len(), 4);
}

// covers: REQ-097, REQ-098
#[test]
fn dragging_a_corner_enlarges_the_text_and_undo_restores_it_in_one_step() {
    let (_d, mut app) = sphere_app();
    let id = with_line(&mut app);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let hs = handles(&mut h);
    let br = hs
        .iter()
        .max_by(|a, b| (a.center().x + a.center().y).total_cmp(&(b.center().x + b.center().y)))
        .unwrap()
        .center();
    let n = h.app.history.undo_len();
    let pic = picture(&h);
    // drag the bottom-right corner away from the centre along the diagonal
    let centre = pic.center();
    let out = br + (br - centre) * 0.6;
    h.drag(br, out);
    let l = h.app.project.subtitles.get(id).unwrap();
    let size = l.size.expect("the line got a size");
    assert!((1250..=4000).contains(&size), "{size}");
    assert_eq!(h.app.history.undo_len(), n + 1, "one drag is one undo step");
    assert!(h.app.resize_drag.is_none());
    // the box is still inside the picture (centre moved with the opposite corner fixed)
    let p = l.position.unwrap();
    assert!(p.x > 5000 && p.y > 5000, "{p:?}");
    h.key(Key::Z, Modifiers::COMMAND);
    let l = h.app.project.subtitles.get(id).unwrap();
    assert_eq!((l.size, l.position), (None, Some(Pos { x: 5000, y: 5000 })));
    // a drag of the same corner far outside stops at the limits and the picture edge
    let handles = handles(&mut h);
    let br = handles
        .iter()
        .max_by(|a, b| (a.center().x + a.center().y).total_cmp(&(b.center().x + b.center().y)))
        .unwrap()
        .center();
    h.drag(br, br + vec2(2000.0, 1500.0));
    let l = h.app.project.subtitles.get(id).unwrap();
    assert!(l.size.unwrap() <= 4000);
    let p = l.position.unwrap();
    assert!(p.x <= 10_000 && p.y <= 10_000);
}

// covers: REQ-097, REQ-096
#[test]
fn without_boxes_the_corner_does_not_resize_but_the_text_still_moves() {
    let (_d, mut app) = sphere_app();
    let id = with_line(&mut app);
    app.settings.show_boxes = false;
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let grab = pic.center();
    h.drag(grab, grab + vec2(40.0, 10.0));
    let l = h.app.project.subtitles.get(id).unwrap();
    assert_eq!(l.size, None, "no resizing without boxes");
    assert_ne!(l.position, Some(Pos { x: 5000, y: 5000 }), "moved");
}

// covers: REQ-097, REQ-092
#[test]
fn a_resize_gives_every_selected_line_the_size() {
    let (_d, mut app) = sphere_app();
    let a = with_line(&mut app);
    let now = Instant::now();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let b = app.selection.primary_line().unwrap();
    app.dispatch(Command::SetSubtitleText(b, "x".into()), now)
        .unwrap();
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![b],
            pos: Some(Pos { x: 8500, y: 1500 }),
        },
        now,
    )
    .unwrap();
    app.selection = [a, b].into_iter().collect();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let hs = handles(&mut h);
    // the biggest box is a's: its bottom-right handle is the one nearest the centre line
    let pic = picture(&h);
    let corner = hs
        .iter()
        .filter(|r| (r.center().x - pic.center().x).abs() < pic.width() * 0.2)
        .max_by(|p, q| (p.center().x + p.center().y).total_cmp(&(q.center().x + q.center().y)))
        .unwrap()
        .center();
    h.drag(corner, corner + vec2(40.0, 25.0));
    let subs = &h.app.project.subtitles;
    let (sa, sb) = (subs.get(a).unwrap().size, subs.get(b).unwrap().size);
    assert!(sa.is_some() && sa == sb, "{sa:?} {sb:?}");
    assert_eq!(
        subs.get(b).unwrap().position,
        Some(Pos { x: 8500, y: 1500 }),
        "the other line keeps its place"
    );
}

// covers: REQ-101
#[test]
fn dragging_a_360_picture_turns_the_camera_by_the_pixels_dragged() {
    let (_d, app) = sphere_app();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let m = h.app.project.media[0].id;
    let from = pic.center();
    h.drag(from, from + vec2(100.0, 0.0));
    let yaw = h.app.live[&m].cam.yaw_deg();
    // 100 pixels at 90 degrees over the picture width, the picture follows the pointer
    let want = -100.0 * 90.0 / f64::from(pic.width());
    assert!((yaw - want).abs() < 0.1, "yaw {yaw} want {want}");
    // vertical: up/down, never past the pole
    h.drag(from, from + vec2(0.0, 5000.0));
    assert_eq!(h.app.live[&m].cam.pitch_cd, 9000);
    // the project is untouched and there is nothing to undo
    assert!(h.app.project.camera.is_empty());
    // wheel: zoom stays in 30..110
    for _ in 0..30 {
        h.frame(vec![Event::PointerMoved(from), wheel(400.0)]);
    }
    h.idle(10);
    let fov = h.app.live[&m].cam.fov_cd;
    assert!((3000..9000).contains(&fov), "{fov}");
    for _ in 0..40 {
        h.frame(vec![wheel(-600.0)]);
    }
    h.idle(10);
    assert!(h.app.live[&m].cam.fov_cd <= 11000);
    // pinch zoom
    let before = h.app.live[&m].cam.fov_cd;
    h.frame(vec![Event::Zoom(1.5)]);
    h.idle(2);
    assert!(h.app.live[&m].cam.fov_cd < before);
}

// covers: REQ-101, REQ-089
#[test]
fn a_subtitle_box_under_the_pointer_is_dragged_instead_of_the_camera() {
    let (_d, mut app) = sphere_app();
    let id = with_line(&mut app);
    let m = app.project.media[0].id;
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    h.drag(pic.center(), pic.center() + vec2(30.0, 0.0));
    assert!(h.app.project.subtitles.get(id).unwrap().position.unwrap().x > 5000);
    assert!(!h.app.live.contains_key(&m), "the camera did not turn");
}

// covers: REQ-102
#[test]
fn set_pov_button_stores_the_view_and_the_marker_opens_its_popup() {
    let (_d, app) = sphere_app();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let m = h.app.project.media[0].id;
    let pic = picture(&h);
    h.drag(pic.center(), pic.center() + vec2(60.0, 0.0));
    let n = h.app.history.undo_len();
    h.click_label("Set POV");
    assert_eq!(
        h.app
            .project
            .camera
            .path_of(Some(h.app.project.timeline.clips[0].id.0), m)
            .len(),
        1
    );
    assert_eq!(h.app.history.undo_len(), n + 1);
    let pov = h
        .app
        .project
        .camera
        .path_of(Some(h.app.project.timeline.clips[0].id.0), m)[0];
    assert_eq!(pov.t, Micros(1_000_000));
    assert!(pov.cam.yaw_cd < 0);
    // its marker on the timeline (tooltip with the angles)
    let all = markers::layout(&h.app.project, 50.0, 0.0, (f32::MIN, f32::MAX));
    let mk = all
        .iter()
        .find(|k| k.kind == markers::MarkerKind::Pov)
        .expect("a POV marker");
    assert!(
        mk.tooltip.contains("yaw") && mk.tooltip.contains("zoom"),
        "{}",
        mk.tooltip
    );
    // click opens a popup with the numbers and Remove
    h.app
        .dispatch(Command::OpenInstance(mk.instance), Instant::now())
        .unwrap();
    h.idle(3);
    assert!(h.app.popups.is_open(Popup::Instance));
    assert!(h.has("Time (source)") && h.has("Direction (yaw)") && h.has("Zoom (field of view)"));
    assert_eq!(h.app.popups.instance, Some(Instance::Pov(m, pov.t)));
    h.click_label("Remove");
    assert!(h.app.project.camera.is_empty());
    assert_eq!(h.app.undo_text(), "Undo Remove POV");
    h.idle(3);
    assert!(h.has("This point of view no longer exists."));
}

// covers: REQ-102
#[test]
fn pov_markers_follow_the_clip_and_hide_outside_the_kept_part() {
    let (_d, mut app) = sphere_app();
    let now = Instant::now();
    let m = app.project.media[0].id;
    let pov = |t: i64| crate::camera::Pov {
        t: Micros(t * 1_000_000),
        cam: crate::camera::Camera::FRONT,
        ease: Default::default(),
    };
    app.project.camera.set(m, pov(1)).unwrap();
    app.project.camera.set(m, pov(4)).unwrap();
    let all = |app: &AppState| {
        markers::layout(&app.project, 10.0, 0.0, (f32::MIN, f32::MAX))
            .into_iter()
            .filter(|k| k.kind == markers::MarkerKind::Pov)
            .collect::<Vec<_>>()
    };
    let xs: Vec<f32> = all(&app).iter().map(|k| k.x).collect();
    assert_eq!(xs, vec![10.0, 40.0]);
    // trim the start: the first POV is hidden (not lost), the second moves left
    let c = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::SetTrim {
            clip: c,
            src_in: Micros(2_000_000),
            src_out: Micros(5_000_000),
        },
        now,
    )
    .unwrap();
    let xs: Vec<f32> = all(&app).iter().map(|k| k.x).collect();
    assert_eq!(xs, vec![20.0]);
    assert_eq!(
        app.project
            .camera
            .path_of(Some(app.project.timeline.clips[0].id.0), m)
            .len(),
        2
    );
    assert_eq!(
        app.instance_start(Instance::Pov(m, Micros(1_000_000))),
        None
    );
    assert_eq!(
        app.instance_start(Instance::Pov(m, Micros(4_000_000))),
        Some(Micros(2_000_000))
    );
    // a POV of a normal clip is not shown
    app.project.media[0].format.spherical = Some(false);
    assert!(all(&app).is_empty());
}

// covers: REQ-100
#[test]
fn clip_details_shows_the_360_tag_without_a_switch_and_the_media_list_the_tag() {
    let (_d, mut app) = sphere_app();
    let c = app.project.timeline.clips[0].id;
    app.selected_clip = Some(c);
    app.popups.show(Popup::ClipDetails);
    let h = Harness::new(app, 1280.0, 900.0);
    assert!(
        h.nodes
            .iter()
            .any(|n| n.label.starts_with("360°") && n.role != Role::Button
                || n.label.contains("360°, ")),
        "{:#?}",
        h.labels()
    );
    // information only: no switch to turn it off or on
    assert!(h.has("360° video"), "{:#?}", h.labels());
    assert!(!h.has("Treat as 360° video"), "{:#?}", h.labels());
}

// covers: REQ-098, REQ-099, REQ-070, REQ-072
#[test]
fn the_effect_commands_are_in_the_toolbar_and_the_menus_and_run() {
    let (_d, mut app) = sphere_app();
    let id = with_line(&mut app);
    let now = Instant::now();
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![id],
            pos: Some(Pos { x: 1500, y: 8000 }),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SelectLine(Some(id)), now).unwrap();
    let mut h = Harness::new(app, 1600.0, 900.0);
    for label in ["Align horizontally", "Align vertically", "Reset"] {
        let n = h
            .nodes
            .iter()
            .find(|n| n.label == label && n.role == Role::Button && n.rect.max.y < 60.0)
            .unwrap_or_else(|| panic!("{label} not in the toolbar: {:#?}", h.labels()));
        assert!(!n.disabled, "{label}");
    }
    h.click_label("Align horizontally");
    assert_eq!(
        h.app.project.subtitles.get(id).unwrap().position,
        Some(Pos { x: 5000, y: 8000 })
    );
    assert_eq!(h.app.undo_text(), "Undo Align horizontally");
    // the shortcut
    h.key(
        Key::J,
        Modifiers {
            command: true,
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(
        h.app.project.subtitles.get(id).unwrap().position,
        Some(Pos { x: 5000, y: 5000 })
    );
    h.key(
        Key::R,
        Modifiers {
            command: true,
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(h.app.project.subtitles.get(id).unwrap().position, None);
    // Reset is greyed out now, with the reason as tooltip text on the command
    assert!(!h.app.is_enabled(&Command::ResetObjects));
    // nothing selected: Align is not shown, Set POV stays (the playhead is on a 360 clip)
    h.app.selection.clear();
    h.idle(2);
    assert!(
        h.nodes
            .iter()
            .any(|n| n.label == "Set POV" && n.role == Role::Button && n.rect.max.y < 60.0)
    );
    for label in ["Align horizontally", "Align vertically"] {
        assert!(
            !h.nodes
                .iter()
                .any(|n| n.label == label && n.role == Role::Button && n.rect.max.y < 60.0),
            "{label} still on the bar"
        );
    }
}

// covers: REQ-098, REQ-070
#[test]
fn the_effect_group_is_reachable_through_the_overflow_menu_at_800_pixels() {
    let (_d, mut app) = sphere_app();
    let id = with_line(&mut app);
    app.dispatch(Command::SelectLine(Some(id)), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 800.0, 500.0);
    let before = h.nodes.clone();
    let on_bar = |h: &Harness, label: &str| {
        h.nodes
            .iter()
            .any(|n| n.label == label && n.role == Role::Button && n.rect.max.y < 60.0)
    };
    // the group is never split: all in the row or all in the menu
    let shown: Vec<bool> = ["Align horizontally", "Align vertically", "Reset"]
        .iter()
        .map(|l| on_bar(&h, l))
        .collect();
    assert!(
        shown.iter().all(|s| *s) || shown.iter().all(|s| !*s),
        "{shown:?}"
    );
    if !shown[0] {
        let more = h.find("»").expect("overflow menu");
        h.click(more.rect.center());
        let menu = h
            .nodes
            .iter()
            .filter(|n| {
                !before
                    .iter()
                    .any(|b| b.label == n.label && b.rect == n.rect)
            })
            .cloned()
            .collect::<Vec<Node>>();
        for label in ["Align horizontally", "Align vertically", "Reset"] {
            let n = menu
                .iter()
                .find(|n| n.label.starts_with(label))
                .unwrap_or_else(|| panic!("{label}: {menu:#?}"));
            assert!(h.screen().contains_rect(n.rect));
        }
    }
}

// covers: REQ-096, REQ-069
#[test]
fn view_menu_shows_and_hides_the_boxes() {
    let (_d, mut app) = sphere_app();
    with_line(&mut app);
    app.settings_path = None;
    let mut h = Harness::new(app, 1280.0, 800.0);
    assert!(h.app.settings.show_boxes);
    let view = h
        .nodes
        .iter()
        .find(|n| n.label == "View" && n.role == Role::Button && n.rect.max.y <= 26.0)
        .cloned()
        .unwrap();
    h.click(view.rect.center());
    let item = h.find("Show boxes on the picture").expect("the item");
    h.click(item.rect.center());
    assert!(!h.app.settings.show_boxes);
    assert!(handles(&mut h).is_empty());
}

// covers: REQ-102, REQ-098
#[test]
fn the_preview_right_click_menu_has_the_effect_commands_and_set_pov() {
    let (_d, mut app) = sphere_app();
    with_line(&mut app);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let before = h.nodes.clone();
    h.right_click(pic.min + vec2(20.0, 20.0));
    let menu: Vec<Node> = h
        .nodes
        .iter()
        .filter(|n| {
            !before
                .iter()
                .any(|b| b.label == n.label && b.rect == n.rect)
        })
        .cloned()
        .collect();
    for label in ["Align horizontally", "Align vertically", "Reset", "Set POV"] {
        assert!(
            menu.iter().any(|n| n.label.starts_with(label)),
            "{label}: {menu:#?}"
        );
    }
    let set = menu
        .iter()
        .find(|n| n.label.starts_with("Set POV"))
        .unwrap();
    assert!(!set.disabled);
    let _ = PointerButton::Primary;
}

// covers: REQ-112
#[test]
fn a_line_fixed_to_the_sphere_is_dragged_to_another_point_not_to_a_flat_place() {
    let (_d, mut app) = sphere_app();
    let now = Instant::now();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let id = app.selection.primary_line().unwrap();
    app.dispatch(Command::SetSubtitleText(id, "HHHH".into()), now)
        .unwrap();
    app.dispatch(Command::ClearSelection, now).unwrap();
    let before = app
        .project
        .subtitles
        .get(id)
        .unwrap()
        .anchor
        .expect("fixed to the sphere");
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    // the box is at the centre of the view (the front): drag it to the right
    h.drag(pic.center(), pic.center() + vec2(120.0, 0.0));
    let l = h.app.project.subtitles.get(id).unwrap();
    assert_eq!(l.position, None, "the flat place is untouched");
    assert!(
        l.anchor.unwrap().yaw_cd > before.yaw_cd + 500,
        "{:?}",
        l.anchor
    );
    assert!(!h.app.live.contains_key(&h.app.project.media[0].id));
}

// covers: REQ-272
#[test]
fn the_arrow_keys_turn_the_camera_and_enter_adds_a_point_of_view_only_with_the_picture_focused() {
    let (_d, app) = sphere_app();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let (m, t) = h.app.sphere_at_playhead().unwrap();
    let yaw = |h: &Harness| h.app.effective_camera(m, t).yaw_cd;
    // no focus on the picture: the arrows do not turn the camera
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(yaw(&h), 0);
    // a click on the picture gives it the focus
    let sections = h.cache.sections.unwrap();
    let centre = sections.preview.center();
    h.click(centre);
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(yaw(&h), 500, "5 degrees");
    h.key(Key::ArrowLeft, Modifiers::NONE);
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(yaw(&h), -500);
    h.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.app.effective_camera(m, t).pitch_cd, 500);
    let fov = h.app.effective_camera(m, t).fov_cd;
    h.key(Key::PageUp, Modifiers::NONE);
    assert!(
        h.app.effective_camera(m, t).fov_cd < fov,
        "Page Up zooms in"
    );
    // Enter makes a point of view at the playhead
    assert!(
        h.app
            .project
            .camera
            .path_of(Some(h.app.project.timeline.clips[0].id.0), m)
            .is_empty()
    );
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(
        h.app
            .project
            .camera
            .path_of(Some(h.app.project.timeline.clips[0].id.0), m)
            .len(),
        1
    );
    assert_eq!(
        h.app
            .project
            .camera
            .path_of(Some(h.app.project.timeline.clips[0].id.0), m)[0]
            .t,
        h.app.playhead
    );
}
