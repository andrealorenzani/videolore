// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The box of an overlap in the preview (2.3.0, REQ-327): a drag moves it (its centre stays on
//! the frame), a drag of an edge or a corner resizes it with the opposite side or corner
//! fixed; without a modifier the shape never changes (the media keeps its own ratio); with Ctrl
//! (Command on macOS) held at the start of an edge or corner drag the shape may change: a video
//! is cropped to the box and a GIF or a picture is stretched to it. One drag is one undo step:
//! the outline follows the pointer and the command is sent when the button is released. The
//! geometry is pure ([`hit`], [`resized`], [`moved`], [`to_box`]).

use super::placement;
use crate::app::{AppState, Command};
use crate::base::pos::{POS_MAX, Pos};
use eframe::egui::{self, Color32, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2};

/// The part of the box a drag takes hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Body,
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// Distance (screen pixels) within which an edge or a corner is taken.
pub const GRAB: f32 = 7.0;
/// The smallest side of a box, in frame pixels.
pub const MIN_SIDE: f64 = 16.0;

/// The part of the screen rectangle `r` at `p`: a corner, an edge, the body, or nothing.
pub fn hit(r: Rect, p: Pos2, tol: f32) -> Option<Handle> {
    if !r.expand(tol).contains(p) {
        return None;
    }
    let near = |a: f32, b: f32| (a - b).abs() <= tol;
    let (w, e) = (near(p.x, r.min.x), near(p.x, r.max.x));
    let (n, s) = (near(p.y, r.min.y), near(p.y, r.max.y));
    Some(match (w, e, n, s) {
        (true, _, true, _) => Handle::NorthWest,
        (_, true, true, _) => Handle::NorthEast,
        (true, _, _, true) => Handle::SouthWest,
        (_, true, _, true) => Handle::SouthEast,
        (true, ..) => Handle::West,
        (_, true, ..) => Handle::East,
        (_, _, true, _) => Handle::North,
        (.., true) => Handle::South,
        _ => Handle::Body,
    })
}

/// A box in frame pixels, as `(x0, y0, x1, y1)`.
pub type Box4 = (f64, f64, f64, f64);

/// The box after the handle `h` is dragged by `(dx, dy)` frame pixels. The side or corner
/// opposite the handle stays. With `free` (Ctrl or Command) the sides move as dragged; without
/// it the ratio `ratio` (width / height) is kept: an edge drives its size and the other one
/// follows about the middle, a corner drives the width and the height follows from the opposite
/// corner. A box is never smaller than [`MIN_SIDE`].
pub fn resized(b: Box4, h: Handle, dx: f64, dy: f64, ratio: f64, free: bool) -> Box4 {
    let (mut x0, mut y0, mut x1, mut y1) = b;
    match h {
        Handle::Body => return moved(b, dx, dy),
        Handle::East | Handle::NorthEast | Handle::SouthEast => x1 += dx,
        Handle::West | Handle::NorthWest | Handle::SouthWest => x0 += dx,
        Handle::North | Handle::South => {}
    }
    match h {
        Handle::South | Handle::SouthEast | Handle::SouthWest => y1 += dy,
        Handle::North | Handle::NorthEast | Handle::NorthWest => y0 += dy,
        Handle::East | Handle::West | Handle::Body => {}
    }
    // never smaller than the minimum, the dragged side stops
    if x1 - x0 < MIN_SIDE {
        match h {
            Handle::West | Handle::NorthWest | Handle::SouthWest => x0 = x1 - MIN_SIDE,
            _ => x1 = x0 + MIN_SIDE,
        }
    }
    if y1 - y0 < MIN_SIDE {
        match h {
            Handle::North | Handle::NorthEast | Handle::NorthWest => y0 = y1 - MIN_SIDE,
            _ => y1 = y0 + MIN_SIDE,
        }
    }
    if free || ratio <= 0.0 || !ratio.is_finite() {
        return (x0, y0, x1, y1);
    }
    let (w, hgt) = (x1 - x0, y1 - y0);
    let (cx, cy) = ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
    match h {
        // an edge drives one size, the other follows about the middle of the box
        Handle::East | Handle::West => {
            let nh = w / ratio;
            (x0, cy - nh / 2.0, x1, cy + nh / 2.0)
        }
        Handle::North | Handle::South => {
            let nw = hgt * ratio;
            (cx - nw / 2.0, y0, cx + nw / 2.0, y1)
        }
        // a corner drives the width; the height follows from the opposite corner
        Handle::NorthEast | Handle::SouthEast | Handle::NorthWest | Handle::SouthWest => {
            let nh = w / ratio;
            match h {
                Handle::NorthEast | Handle::NorthWest => (x0, b.3 - nh, x1, b.3),
                _ => (x0, b.1, x1, b.1 + nh),
            }
        }
        Handle::Body => (x0, y0, x1, y1),
    }
}

/// The box moved by `(dx, dy)`, its centre kept on the frame `frame` (REQ-117).
pub fn moved(b: Box4, dx: f64, dy: f64) -> Box4 {
    (b.0 + dx, b.1 + dy, b.2 + dx, b.3 + dy)
}

/// The stored place and box (thousandths) of a box in frame pixels: the centre is kept on the
/// frame and the sides inside 1..=1000.
pub fn to_box(b: Box4, frame: (u32, u32)) -> (Pos, u16, u16) {
    let (fw, fh) = (f64::from(frame.0.max(1)), f64::from(frame.1.max(1)));
    let fx = (((b.0 + b.2) / 2.0) / fw).clamp(0.0, 1.0);
    let fy = (((b.1 + b.3) / 2.0) / fh).clamp(0.0, 1.0);
    let thousandths = |v: f64| (v.round() as i64).clamp(1, 1000) as u16;
    (
        Pos {
            x: (fx * f64::from(POS_MAX)).round() as u16,
            y: (fy * f64::from(POS_MAX)).round() as u16,
        },
        thousandths((b.2 - b.0) / fw * 1000.0),
        thousandths((b.3 - b.1) / fh * 1000.0),
    )
}

/// Draws the outlines of the overlaps on screen at the playhead and runs their drags.
pub fn interact(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>, picture: Rect) {
    let Some(frame) = app.output_size() else {
        return;
    };
    let tl = &app.project.timeline;
    let Some((i, start)) = tl.locate(app.playhead) else {
        return;
    };
    let offset = app.playhead - start;
    let clip = tl.clips[i];
    for o in tl.overlaps_of(clip.id) {
        if offset < o.from || offset >= o.end() {
            continue;
        }
        let selected = app.selected_overlap == Some(o.id);
        let (x, y, w, h) = o.rect(frame);
        let media: Box4 = (
            f64::from(x),
            f64::from(y),
            f64::from(x) + f64::from(w),
            f64::from(y) + f64::from(h),
        );
        let ratio = f64::from(o.src_w) / f64::from(o.src_h.max(1));
        let grab_id = Id::unique(("videolore-overlap-grab", o.id));
        let live = ui.data(|d| d.get_temp::<(Box4, bool)>(grab_id));
        let shown = live.map_or(media, |(b, _)| b);
        let r = placement::to_screen(
            Rect::from_min_max(
                pos2(shown.0 as f32, shown.1 as f32),
                pos2(shown.2 as f32, shown.3 as f32),
            ),
            picture,
            frame,
        );
        let color = if selected {
            Color32::YELLOW
        } else {
            Color32::from_rgb(120, 170, 255)
        };
        ui.painter_at(picture).rect_stroke(
            r,
            0.0,
            Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, color),
            StrokeKind::Inside,
        );
        // an unselected overlap is taken by a click; the selected one by drags too
        let sense = if selected {
            Sense::click_and_drag()
        } else {
            Sense::click()
        };
        let resp = ui
            .interact(
                r.expand(GRAB),
                Id::unique(("videolore-overlap-box", o.id)),
                sense,
            )
            .on_hover_text(format!(
                "{}\nDrag to move; drag an edge to resize (Ctrl or Command: change its shape)",
                o.name()
            ));
        if resp.clicked() {
            cmds.push(Command::SelectClip(Some(clip.id)));
            cmds.push(Command::SelectOverlap(Some(o.id)));
        }
        if !selected {
            continue;
        }
        let scale = (picture.width() / frame.0.max(1) as f32).max(f32::EPSILON);
        // the part taken is the one under the pointer where the button went down (the drag
        // starts a few pixels later)
        if resp.drag_started()
            && let Some(p) = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| resp.interact_pointer_pos())
        {
            let handle = hit(r, p, GRAB).unwrap_or(Handle::Body);
            let free = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            ui.data_mut(|d| {
                d.insert_temp(grab_id.with("grab"), (handle as u8, free));
            });
        }
        let held: Option<(u8, bool)> = ui.data(|d| d.get_temp(grab_id.with("grab")));
        if let (Some((hb, free)), Some(delta)) = (held, resp.total_drag_delta()) {
            let handle = handle_of(hb);
            let (dx, dy) = (f64::from(delta.x / scale), f64::from(delta.y / scale));
            let b = resized(media, handle, dx, dy, ratio, free);
            ui.data_mut(|d| d.insert_temp(grab_id, (b, free)));
        }
        if resp.drag_stopped() {
            let done: Option<(Box4, bool)> = ui.data_mut(|d| d.remove_temp(grab_id));
            ui.data_mut(|d| d.remove_temp::<(u8, bool)>(grab_id.with("grab")));
            if let Some((b, free)) = done {
                let (place, width, height) = to_box(b, frame);
                cmds.push(Command::SetOverlapBox {
                    id: o.id,
                    place,
                    width,
                    height,
                    reshaped: free && changed_shape(media, b),
                });
            }
        }
        if resp.hovered() || live.is_some() {
            let icon = match ui
                .input(|i| i.pointer.hover_pos())
                .and_then(|p| hit(r, p, GRAB))
            {
                Some(Handle::North | Handle::South) => egui::CursorIcon::ResizeVertical,
                Some(Handle::East | Handle::West) => egui::CursorIcon::ResizeHorizontal,
                Some(Handle::NorthEast | Handle::SouthWest) => egui::CursorIcon::ResizeNeSw,
                Some(Handle::NorthWest | Handle::SouthEast) => egui::CursorIcon::ResizeNwSe,
                _ => egui::CursorIcon::Grab,
            };
            ui.ctx().set_cursor_icon(icon);
        }
    }
}

/// True when the new box has another ratio than the old one (more than a pixel of difference).
fn changed_shape(a: Box4, b: Box4) -> bool {
    let (wa, ha) = (a.2 - a.0, a.3 - a.1);
    let (wb, hb) = (b.2 - b.0, b.3 - b.1);
    (wa * hb - wb * ha).abs() > wa.max(ha).max(wb).max(hb)
}

fn handle_of(v: u8) -> Handle {
    [
        Handle::Body,
        Handle::North,
        Handle::South,
        Handle::East,
        Handle::West,
        Handle::NorthEast,
        Handle::NorthWest,
        Handle::SouthEast,
        Handle::SouthWest,
    ]
    .get(usize::from(v))
    .copied()
    .unwrap_or(Handle::Body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Box4 = (100.0, 100.0, 300.0, 200.0);

    // covers: REQ-327
    #[test]
    fn an_edge_or_corner_is_taken_within_the_tolerance_and_the_body_inside() {
        let r = Rect::from_min_max(pos2(100.0, 100.0), pos2(300.0, 200.0));
        assert_eq!(hit(r, pos2(200.0, 150.0), 7.0), Some(Handle::Body));
        assert_eq!(hit(r, pos2(102.0, 150.0), 7.0), Some(Handle::West));
        assert_eq!(hit(r, pos2(298.0, 150.0), 7.0), Some(Handle::East));
        assert_eq!(hit(r, pos2(200.0, 98.0), 7.0), Some(Handle::North));
        assert_eq!(hit(r, pos2(200.0, 203.0), 7.0), Some(Handle::South));
        assert_eq!(hit(r, pos2(101.0, 101.0), 7.0), Some(Handle::NorthWest));
        assert_eq!(hit(r, pos2(299.0, 101.0), 7.0), Some(Handle::NorthEast));
        assert_eq!(hit(r, pos2(101.0, 199.0), 7.0), Some(Handle::SouthWest));
        assert_eq!(hit(r, pos2(299.0, 199.0), 7.0), Some(Handle::SouthEast));
        assert_eq!(hit(r, pos2(400.0, 400.0), 7.0), None);
    }

    // covers: REQ-327
    #[test]
    fn without_a_modifier_the_ratio_is_kept_and_the_opposite_side_stays() {
        let ratio = 2.0;
        // the east edge 100 px out: the width 300, the height follows about the middle
        let b = resized(B, Handle::East, 100.0, 7.0, ratio, false);
        assert_eq!((b.0, b.2), (100.0, 400.0));
        assert!((((b.2 - b.0) / (b.3 - b.1)) - ratio).abs() < 1e-9);
        assert!((((b.1 + b.3) / 2.0) - 150.0).abs() < 1e-9);
        // the south-east corner: the north-west corner stays
        let b = resized(B, Handle::SouthEast, 100.0, 3.0, ratio, false);
        assert_eq!((b.0, b.1), (100.0, 100.0));
        assert!((((b.2 - b.0) / (b.3 - b.1)) - ratio).abs() < 1e-9);
        // the north-west corner: the south-east corner stays
        let b = resized(B, Handle::NorthWest, -50.0, 0.0, ratio, false);
        assert_eq!((b.2, b.3), (300.0, 200.0));
        assert!((((b.2 - b.0) / (b.3 - b.1)) - ratio).abs() < 1e-9);
        // never smaller than the minimum
        let b = resized(B, Handle::East, -500.0, 0.0, ratio, true);
        assert_eq!(b.2 - b.0, MIN_SIDE);
        // the body moves
        assert_eq!(
            resized(B, Handle::Body, 10.0, -5.0, ratio, false),
            (110.0, 95.0, 310.0, 195.0)
        );
    }

    // covers: REQ-327
    #[test]
    fn with_ctrl_the_shape_changes_and_the_opposite_side_stays() {
        // a 2:1 box dragged to a square: the east edge to 200 wide, the south edge to 200 high
        let b = resized(B, Handle::East, -0.0, 0.0, 2.0, true);
        assert_eq!(b, B);
        let b = resized(B, Handle::SouthEast, -100.0, 100.0, 2.0, true);
        assert_eq!(b, (100.0, 100.0, 200.0, 300.0));
        assert!(changed_shape(B, b));
        assert!(!changed_shape(B, (100.0, 100.0, 500.0, 300.0)));
        // the west edge keeps the east side
        let b = resized(B, Handle::West, 50.0, 0.0, 2.0, true);
        assert_eq!((b.2, b.1, b.3), (300.0, 100.0, 200.0));
    }

    // covers: REQ-327
    #[test]
    fn the_place_and_box_are_thousandths_with_the_centre_on_the_frame() {
        let (p, w, h) = to_box((660.0, 540.0, 1260.0, 702.0), (1920, 1080));
        assert_eq!((p.x, p.y), (5000, 5750));
        assert_eq!((w, h), (313, 150));
        // a box dragged out of the frame keeps its centre on it
        let (p, w, _) = to_box((-3000.0, 0.0, -1000.0, 100.0), (1920, 1080));
        assert_eq!(p.x, 0);
        assert!((1..=1000).contains(&w));
        // every handle has its number back
        assert_eq!(handle_of(0), Handle::Body);
        assert_eq!(handle_of(200), Handle::Body);
        assert_eq!(handle_of(Handle::SouthWest as u8), Handle::SouthWest);
    }
}
