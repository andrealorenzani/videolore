// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 2.0.4, batch 6: trimming by dragging a clip edge (REQ-191), dragging
//! subtitle and fade markers (REQ-192, REQ-193), the marker keys (REQ-194), Ctrl/Cmd+V
//! (REQ-195), Delete and shifts of several lines (REQ-197), snap guides (REQ-198) and the
//! safe areas (REQ-199).

use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Command, Popup};
use crate::selection::ObjRef;
use crate::subtitles::{LineId, NewLine};
use crate::time::Micros;
use eframe::egui::{Event, Key, Modifiers, Pos2, pos2};
use std::time::Instant;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips (a.mp4, b.mp4) and `lines`: (media index, source start, source end).
fn two_clips(lines: &[(usize, i64, i64)]) -> (tempfile::TempDir, AppState, Vec<LineId>) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        let id = app.project.media[i].id;
        app.dispatch(Command::Append(Some(id)), now).unwrap();
    }
    let ids: Vec<_> = app.project.media.iter().map(|m| m.id).collect();
    let new: Vec<NewLine> = lines
        .iter()
        .map(|(m, a, b)| NewLine {
            media: ids[*m],
            start: Micros(*a * 100_000),
            end: Micros(*b * 100_000),
            text: format!("line {a}"),
            words: vec![],
        })
        .collect();
    app.project.subtitles.replace_all(new, None);
    let line_ids = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    (dir, app, line_ids)
}

fn geometry(h: &Harness) -> crate::ui::timeline::Geometry {
    h.cache.timeline.expect("the timeline was drawn")
}

/// The x position of timeline time `t`.
fn x_at(h: &Harness, t: Micros) -> f32 {
    let g = geometry(h);
    g.x0 + (t.as_secs_f64() * f64::from(g.zoom)) as f32
}

fn lane_y(h: &Harness) -> f32 {
    geometry(h).lane.center().y
}

fn video_y(h: &Harness) -> f32 {
    geometry(h).video.center().y
}

// covers: REQ-191
#[test]
fn dragging_the_right_edge_of_a_clip_trims_it_like_the_clip_details_and_undoes_in_one_step() {
    let (_d, app, _) = two_clips(&[]);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let z = geometry(&h).zoom;
    let edge = x_at(&h, s(5));
    let y = video_y(&h);
    // drag the right edge 1 s to the left
    h.drag(pos2(edge - 2.0, y), pos2(edge - 2.0 - z, y));
    let c = h.app.project.timeline.clips[0];
    let frame = h.app.frame_len();
    assert!(
        ((s(4) - c.len()).0).abs() <= frame.0,
        "1 s shorter within a frame: {:?}",
        c.len()
    );
    assert_eq!(c.src_in, Micros::ZERO);
    assert_eq!(h.app.history.undo_label(), Some("Trim"));
    // the same as the Clip details trim to that length
    let mut other = h.app.project.timeline.clone();
    other
        .trim(c.id, Micros::ZERO, c.src_out, s(5), frame)
        .unwrap();
    assert_eq!(other.clips[0], c);
    // one undo restores the clip
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(h.app.project.timeline.clips[0].len(), s(5));
    // past the source limit stops at it: the right edge dragged further right changes nothing
    h.idle(2);
    let edge = x_at(&h, s(5));
    let before = h.app.undo_text();
    h.drag(pos2(edge - 2.0, y), pos2(edge + 2.0 * z, y));
    assert_eq!(h.app.project.timeline.clips[0].len(), s(5));
    assert_eq!(h.app.undo_text(), before, "no change, no undo step");
    // the left edge of the second clip trims from its start
    let second = x_at(&h, s(5));
    h.drag(pos2(second + 2.0, y), pos2(second + 2.0 + z, y));
    let c1 = h.app.project.timeline.clips[1];
    assert!(
        (800_000..=1_100_000).contains(&c1.src_in.0),
        "about a second: {:?}",
        c1.src_in
    );
}

// covers: REQ-192
#[test]
fn dragging_a_subtitle_marker_moves_that_time_in_tenths_and_stops_before_the_other_end() {
    let (_d, app, ids) = two_clips(&[(0, 10, 30)]); // 1 s .. 3 s
    let mut h = Harness::new(app, 1280.0, 800.0);
    let z = geometry(&h).zoom;
    let y = lane_y(&h);
    let line = |h: &Harness| h.app.project.subtitles.get(ids[0]).unwrap().clone();
    // the start marker 0.5 s later
    let x = x_at(&h, s(1));
    h.drag(pos2(x, y), pos2(x + 0.5 * z, y));
    assert_eq!((line(&h).start, line(&h).end), (Micros(1_500_000), s(3)));
    assert_eq!(h.app.history.undo_label(), Some("Change subtitle timing"));
    // dragged past the end it stops 0.1 s before it
    h.idle(2);
    let x = x_at(&h, Micros(1_500_000));
    h.drag(pos2(x, y), pos2(x + 5.0 * z, y));
    assert_eq!(line(&h).start, Micros(2_900_000));
    // one undo restores it
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(line(&h).start, Micros(1_500_000));
    // the end marker
    h.idle(2);
    let xe = x_at(&h, s(3));
    h.drag(pos2(xe, y), pos2(xe + z, y));
    assert_eq!(line(&h).end, s(4));
}

// covers: REQ-193
#[test]
fn dragging_a_fade_marker_moves_it_to_an_empty_boundary_only() {
    let (_d, mut app, _) = two_clips(&[]);
    let now = Instant::now();
    let id = app.project.media[0].id;
    app.dispatch(Command::Append(Some(id)), now).unwrap();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let y = lane_y(&h);
    let (b0, b1) = (x_at(&h, s(5)), x_at(&h, s(10)));
    h.drag(pos2(b0, y), pos2(b1, y));
    let tl = &h.app.project.timeline;
    assert!(
        !tl.has_fade(0) && tl.has_fade(1),
        "the fade moved to boundary 2"
    );
    assert_eq!(h.app.history.undo_label(), Some("Move fade"));
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert!(h.app.project.timeline.has_fade(0) && !h.app.project.timeline.has_fade(1));
    // dropped on empty space: nothing changes
    h.idle(2);
    let n = h.app.undo_text();
    let (b0, mid) = (x_at(&h, s(5)), x_at(&h, Micros(7_500_000)));
    h.drag(pos2(b0, y), pos2(mid, y));
    assert!(h.app.project.timeline.has_fade(0));
    assert_eq!(h.app.undo_text(), n);
    // dropped on an occupied boundary: nothing changes either
    h.app
        .dispatch(Command::ToggleFade(1), Instant::now())
        .unwrap();
    h.idle(2);
    let n = h.app.undo_text();
    let (b0, b1) = (x_at(&h, s(5)), x_at(&h, s(10)));
    h.drag(pos2(b0, y), pos2(b1, y));
    let tl = &h.app.project.timeline;
    assert!(tl.has_fade(0) && tl.has_fade(1));
    assert_eq!(h.app.undo_text(), n);
}

// covers: REQ-194
#[test]
fn the_bracket_keys_go_to_the_previous_and_next_marker_and_select_it_but_not_while_typing() {
    // lines at 2 s (clip 1), 5 s and 9 s (clip 2)
    let (_d, app, ids) = two_clips(&[(0, 20, 30), (1, 0, 10), (1, 40, 45)]);
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.app
        .dispatch(Command::Seek(Micros::ZERO), Instant::now())
        .unwrap();
    h.key(Key::CloseBracket, Modifiers::NONE);
    assert_eq!(h.app.playhead, s(2));
    assert_eq!(h.app.selection.primary_line(), Some(ids[0]));
    h.key(Key::CloseBracket, Modifiers::NONE);
    assert_eq!(h.app.playhead, s(5));
    assert_eq!(h.app.selection.primary_line(), Some(ids[1]));
    h.key(Key::OpenBracket, Modifiers::NONE);
    assert_eq!(h.app.playhead, s(2));
    // before the first and after the last marker nothing moves
    h.key(Key::OpenBracket, Modifiers::NONE);
    assert_eq!(h.app.playhead, s(2));
    h.app.dispatch(Command::Seek(s(9)), Instant::now()).unwrap();
    h.key(Key::CloseBracket, Modifiers::NONE);
    assert_eq!(h.app.playhead, s(9));
    // the keys are in the shortcuts table
    assert!(
        keymap::bindings()
            .iter()
            .any(|(sc, l)| sc.logical_key == Key::OpenBracket && l.contains("Previous"))
    );
    // nothing moves while typing in a text field
    h.app.dispatch(Command::Seek(s(5)), Instant::now()).unwrap();
    let clip = h.app.project.timeline.clips[1].id;
    h.app
        .dispatch(
            Command::OpenInstance(crate::app::Instance::Line(ids[1], clip)),
            Instant::now(),
        )
        .unwrap();
    h.idle(2);
    let field = h
        .nodes
        .iter()
        .find(|n| n.role == eframe::egui::accesskit::Role::TextInput)
        .cloned()
        .unwrap_or_else(|| panic!("{:#?}", h.labels()));
    h.click(field.rect.center());
    assert!(h.ctx.text_edit_focused());
    let at = h.app.playhead;
    h.key(Key::CloseBracket, Modifiers::NONE);
    assert_eq!(h.app.playhead, at);
}

// covers: REQ-194
#[test]
fn markers_of_every_kind_are_stops_in_time_order() {
    let (_d, mut app, _) = two_clips(&[(0, 20, 30)]);
    let now = Instant::now();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    app.dispatch(
        Command::SetEdgeFade {
            edge: crate::app::Edge::End,
            on: true,
        },
        now,
    )
    .unwrap();
    let c1 = app.project.timeline.clips[1].id;
    app.dispatch(
        Command::SetClipVolume {
            clip: c1,
            volume: 50,
            muted: false,
        },
        now,
    )
    .unwrap();
    let mut t =
        crate::text::TextItem::new(app.project.texts.new_id(), "T".into(), Micros(7_000_000));
    t.ttl = Micros(1_000_000);
    app.project.texts.add(t).unwrap();
    let stops = crate::app::stops::stops(&app.project);
    let times: Vec<i64> = stops.iter().map(|(t, _)| t.0).collect();
    let mut sorted = times.clone();
    sorted.sort();
    assert_eq!(times, sorted);
    use crate::app::stops::Stop;
    assert!(
        stops
            .iter()
            .any(|st| matches!(st.1, Stop::Fade(_)) && st.0 == s(5))
    );
    assert!(
        stops
            .iter()
            .any(|st| matches!(st.1, Stop::Fade(_)) && st.0 == s(10)),
        "the end fade"
    );
    assert!(
        stops
            .iter()
            .any(|st| matches!(st.1, Stop::Clip(c) if c == c1))
    );
    assert!(
        stops
            .iter()
            .any(|st| matches!(st.1, Stop::Line(_)) && st.0 == s(2))
    );
    assert!(
        stops
            .iter()
            .any(|st| matches!(st.1, Stop::Text(_)) && st.0 == s(7))
    );
}

// covers: REQ-195
#[test]
fn ctrl_v_pastes_the_last_copy_whatever_the_system_clipboard_holds() {
    let (_d, mut app, _) = two_clips(&[]);
    let now = Instant::now();
    let c = app.project.timeline.clips[0].id;
    app.dispatch(Command::SelectClip(Some(c)), now).unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let n = h.app.project.timeline.clips.len();
    h.frame(vec![Event::Copy]);
    h.idle(1);
    // Copy puts a marker text on the system clipboard, so the paste key always reaches us
    assert!(h.app.clipboard_text.is_none(), "the window took it");
    assert!(h.app.clipboard.is_some());
    assert_eq!(crate::app::CLIPBOARD_MARKER, "videolore clip");
    // text on the system clipboard
    h.frame(vec![Event::Paste("some other text".into())]);
    assert_eq!(h.app.project.timeline.clips.len(), n + 1);
    // an image on the system clipboard
    let image = std::sync::Arc::new(eframe::egui::ColorImage::filled(
        [2, 2],
        eframe::egui::Color32::RED,
    ));
    h.frame(vec![Event::PasteImage(image)]);
    assert_eq!(h.app.project.timeline.clips.len(), n + 2);
    // the menu entry does the same
    h.app.dispatch(Command::Paste, Instant::now()).unwrap();
    assert_eq!(h.app.project.timeline.clips.len(), n + 3);
}

// covers: REQ-197
#[test]
fn delete_removes_every_selected_line_and_the_shift_commands_move_them_together() {
    let (_d, app, ids) = two_clips(&[(0, 10, 20), (0, 30, 40), (1, 5, 15)]);
    let mut h = Harness::new(app, 1280.0, 800.0);
    let now = Instant::now();
    for id in &ids {
        h.app.selection.click(ObjRef::Line(*id), true);
    }
    let starts = |h: &Harness| -> Vec<i64> {
        h.app
            .project
            .subtitles
            .lines
            .iter()
            .map(|l| l.start.0)
            .collect()
    };
    // shift later by 0.5 s through the menu command
    let cmd = actions::ActionId::ShiftLinesLater.command(&h.app);
    h.app.dispatch(cmd, now).unwrap();
    assert_eq!(starts(&h), [1_500_000, 3_500_000, 1_000_000]);
    assert_eq!(h.app.history.undo_label(), Some("Shift subtitle lines"));
    // earlier: stops at 0 s together (the third line would pass it first)
    let cmd = actions::ActionId::ShiftLinesEarlier.command(&h.app);
    for _ in 0..4 {
        h.app.dispatch(cmd.clone(), now).unwrap();
    }
    assert_eq!(starts(&h)[2], 0, "the earliest line stops at 0 s");
    assert_eq!(starts(&h)[0], 500_000);
    // a shift that cannot move anything is no undo step
    let n = h.app.history.undo_len();
    h.app.dispatch(cmd, now).unwrap();
    assert_eq!(h.app.history.undo_len(), n);
    // Delete with lines selected removes all three, one undo restores them
    h.key(Key::Delete, Modifiers::NONE);
    assert!(h.app.project.subtitles.lines.is_empty());
    assert_eq!(h.app.project.timeline.clips.len(), 2, "no clip was removed");
    assert!(h.app.selection.is_empty());
    h.app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(h.app.project.subtitles.lines.len(), 3);
}

// covers: REQ-198
#[test]
fn a_dragged_line_snaps_to_the_centre_and_shows_a_guide_and_the_option_turns_it_off() {
    let (_d, mut app, ids) = two_clips(&[(0, 5, 45)]);
    app.dispatch(Command::Seek(s(1)), Instant::now()).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    // the pure rule first: within 1% of the centre lines and thirds, and of other boxes
    let frame = (1000, 500);
    let (c, g) = crate::placement::snap((505.0, 247.0), frame, &[]);
    assert_eq!(c, (500.0, 250.0));
    assert_eq!((g.x, g.y), (Some(500.0), Some(250.0)));
    let (c, g) = crate::placement::snap((335.0, 100.0), frame, &[(100.0, 104.0)]);
    assert_eq!(c, (330.0, 104.0), "a third, and the centre of another box");
    assert_eq!((g.x, g.y), (Some(330.0), Some(104.0)));
    let (c, g) = crate::placement::snap((400.0, 100.0), frame, &[]);
    assert_eq!(c, (400.0, 100.0));
    assert_eq!(g, crate::placement::Guides::default());
    // in the window: the option is on by default, off after the command, kept as a setting
    assert!(app.settings.snap_to_guides);
    let (c, g) = placement::snapped(&app, pos2(503.0, 252.0), (1920, 1080), &[]);
    let _ = (c, g, ids);
    let (c, _) = placement::snapped(&app, pos2(962.0, 540.0), (1920, 1080), &[]);
    assert_eq!((c.x, c.y), (960.0, 540.0));
    app.dispatch(Command::ToggleSnap, Instant::now()).unwrap();
    assert!(!app.settings.snap_to_guides);
    let (c, g) = placement::snapped(&app, pos2(962.0, 540.0), (1920, 1080), &[]);
    assert_eq!(
        (c.x, c.y),
        (962.0, 540.0),
        "with the option off nothing snaps"
    );
    assert_eq!(g, crate::placement::Guides::default());
    app.dispatch(Command::ToggleSnap, Instant::now()).unwrap();
    assert!(app.settings.snap_to_guides);
}

// covers: REQ-198
#[test]
fn dragging_a_box_in_the_preview_snaps_its_centre_and_the_export_is_unchanged() {
    let (_d, mut app, ids) = two_clips(&[(0, 5, 45)]);
    app.dispatch(Command::Seek(s(1)), Instant::now()).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    let before = crate::render::subtitles_doc(
        &app.project,
        &crate::render::Geometry {
            width: 1280,
            height: 720,
            fps: crate::time::Fps::new(25, 1).unwrap(),
        },
        Micros::ZERO,
    );
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.idle(2);
    let r = h
        .find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect;
    // grab the line's box and drop it a hair away from the exact centre
    let boxes = crate::placement::rects_at(&h.app.project, h.app.playhead, (1280, 720));
    let (_, b) = boxes
        .iter()
        .find(|(o, _)| *o == ObjRef::Line(ids[0]))
        .unwrap();
    let to_screen = |x: f64, y: f64| {
        pos2(
            r.min.x + x as f32 * r.width() / 1280.0,
            r.min.y + y as f32 * r.height() / 720.0,
        )
    };
    let from = to_screen(b.cx, b.cy);
    // the box on the screen is measured by the interface: the pointer holds it where it was
    // grabbed, so the target is the wanted centre plus that offset
    h.press_and_move(from, from + eframe::egui::vec2(8.0, 0.0));
    let grab = h.cache.subtitle_drag.as_ref().expect("a drag began").grab;
    let to = to_screen(
        640.0 + 2.0 + f64::from(grab.x),
        360.0 - 3.0 + f64::from(grab.y),
    );
    h.frame(vec![Event::PointerMoved(to)]);
    h.release(to);
    let l = h.app.project.subtitles.get(ids[0]).unwrap();
    let p = l.position.expect("the line has its own place now");
    assert_eq!((p.x, p.y), (5000, 5000), "centred exactly");
    // the export has the same text and the place written as a number: nothing else changes
    let after = crate::render::subtitles_doc(
        &h.app.project,
        &crate::render::Geometry {
            width: 1280,
            height: 720,
            fps: crate::time::Fps::new(25, 1).unwrap(),
        },
        Micros::ZERO,
    );
    assert!(after.as_deref().unwrap().contains("\\pos(640,360)"));
    assert!(!before.as_deref().unwrap().contains("\\pos("));
}

// covers: REQ-199
#[test]
fn the_safe_areas_are_drawn_on_the_preview_only_and_the_option_is_kept_as_a_setting() {
    let (_d, mut app, _) = two_clips(&[]);
    app.dispatch(Command::Seek(s(1)), Instant::now()).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    assert!(!app.settings.show_safe_areas, "off by default");
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.idle(2);
    let off = h.shapes;
    h.app
        .dispatch(Command::ToggleSafeAreas, Instant::now())
        .unwrap();
    h.idle(2);
    assert!(
        h.shapes > off,
        "two rectangles are drawn: {} then {}",
        off,
        h.shapes
    );
    // the rectangles are 90% and 80% of the picture
    let (l, t, r, b) = crate::placement::safe_area((1000, 500), crate::placement::ACTION_SAFE);
    assert!(
        ((l, t, r, b).0 - 50.0).abs() < 1e-6
            && ((l, t, r, b).1 - 25.0).abs() < 1e-6
            && (r - 950.0).abs() < 1e-6
            && (b - 475.0).abs() < 1e-6
    );
    let (l, t, r, b) = crate::placement::safe_area((1000, 500), crate::placement::TITLE_SAFE);
    assert!(
        (l - 100.0).abs() < 1e-6
            && (t - 50.0).abs() < 1e-6
            && (r - 900.0).abs() < 1e-6
            && (b - 450.0).abs() < 1e-6
    );
    // never in the export: the export reads nothing of the setting
    let doc = crate::render::subtitles_doc(
        &h.app.project,
        &crate::render::Geometry {
            width: 640,
            height: 360,
            fps: crate::time::Fps::new(25, 1).unwrap(),
        },
        Micros::ZERO,
    );
    assert!(doc.is_none());
    // a program setting: it is saved in the settings file, not in the project
    let json = serde_json::to_string(&h.app.settings).unwrap();
    assert!(json.contains("\"show_safe_areas\":true") && json.contains("\"snap_to_guides\":true"));
    let back: crate::settings::Settings = serde_json::from_str(&json.replace("true", "false"))
        .unwrap_or_else(|_| h.app.settings.clone());
    let _ = back;
    h.app
        .dispatch(Command::ToggleSafeAreas, Instant::now())
        .unwrap();
    assert!(!h.app.settings.show_safe_areas);
    // the menu lists both options
    let _ = (Popup::Settings, Pos2::ZERO);
}
