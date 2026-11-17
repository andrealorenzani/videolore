// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Boxes on the preview picture (REQ-096..099): which handle is under the pointer, the
//! cursor to show, resizing by a corner and aligning. Pure geometry, no egui state: the
//! rectangles are plain numbers in one coordinate space (frame pixels for resizing, screen
//! pixels for hit-testing).
//!
//! A box is the outline of one movable object. Today the only objects are subtitle lines;
//! the layer takes a list of [`BoxItem`]s, so another kind of object needs no new
//! interaction code.

use crate::subtitles::{LineId, POS_MAX, Pos, SIZE_MAX, SIZE_MIN, SIZE_NORMAL};
use eframe::egui::{CursorIcon, Pos2, Rect};

/// Pixels around a corner (and so around the box edge) that count as the handle.
pub const HANDLE_TOLERANCE: f32 = 8.0;

/// What a box belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectId {
    Line(LineId),
    Text(crate::text::TextId),
}

/// One movable object on the picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxItem {
    pub id: ObjectId,
    /// The box (frame pixels).
    pub rect: Rect,
    /// The object's size now, in thousandths of its normal size.
    pub size: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    /// The corner of `r` this handle sits on.
    pub fn of(self, r: Rect) -> Pos2 {
        match self {
            Corner::TopLeft => r.left_top(),
            Corner::TopRight => r.right_top(),
            Corner::BottomLeft => r.left_bottom(),
            Corner::BottomRight => r.right_bottom(),
        }
    }

    /// The corner on the other side of the box (the one that stays put when resizing).
    pub fn opposite(self) -> Corner {
        match self {
            Corner::TopLeft => Corner::BottomRight,
            Corner::TopRight => Corner::BottomLeft,
            Corner::BottomLeft => Corner::TopRight,
            Corner::BottomRight => Corner::TopLeft,
        }
    }
}

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Corner(Corner),
    Body,
}

/// The handle under `p` for a box `r` (same coordinates), a corner within `tol` first,
/// else the body.
pub fn handle_at(r: Rect, p: Pos2, tol: f32) -> Option<Hit> {
    let tol = tol.max(0.0);
    let corner = Corner::ALL
        .into_iter()
        .map(|c| (c, c.of(r).distance(p)))
        .filter(|(_, d)| *d <= tol)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| Hit::Corner(c));
    corner.or_else(|| r.contains(p).then_some(Hit::Body))
}

/// The topmost box (last in the list) and what is hit in it. Corner handles of any box
/// win over the body of a box above.
pub fn hit_test(boxes: &[BoxItem], p: Pos2, tol: f32) -> Option<(BoxItem, Hit)> {
    let corner = boxes
        .iter()
        .rev()
        .find_map(|b| match handle_at(b.rect, p, tol) {
            Some(h @ Hit::Corner(_)) => Some((*b, h)),
            _ => None,
        });
    corner.or_else(|| {
        boxes
            .iter()
            .rev()
            .find(|b| b.rect.contains(p))
            .map(|b| (*b, Hit::Body))
    })
}

/// The cursor for what is under the pointer (`dragging`: the grab is closed).
pub fn cursor(hit: Hit, dragging: bool) -> CursorIcon {
    match hit {
        Hit::Corner(Corner::TopLeft | Corner::BottomRight) => CursorIcon::ResizeNwSe,
        Hit::Corner(Corner::TopRight | Corner::BottomLeft) => CursorIcon::ResizeNeSw,
        Hit::Body if dragging => CursorIcon::Grabbing,
        Hit::Body => CursorIcon::Grab,
    }
}

/// Result of dragging a corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resized {
    /// The new box (frame pixels), the opposite corner where it was.
    pub rect: Rect,
    /// New size over old size.
    pub factor: f32,
    /// The new size, thousandths of the normal size, within the limits.
    pub size: u16,
}

/// Resizes box `r0` (size `size0`) by dragging `corner` to the pointer `p`, keeping the
/// proportions and the opposite corner fixed. The factor is the pointer's projection on
/// the box diagonal, limited so that the size stays between 25% and 400% and the box stays
/// inside the picture (`frame` pixels).
pub fn resize(r0: Rect, size0: u16, corner: Corner, p: Pos2, frame: (u32, u32)) -> Resized {
    let size0 = size0.clamp(SIZE_MIN, SIZE_MAX);
    let fixed = corner.opposite().of(r0);
    let d = corner.of(r0) - fixed;
    let dd = d.x * d.x + d.y * d.y;
    let raw = if dd > 0.0 && p.x.is_finite() && p.y.is_finite() {
        ((p - fixed).x * d.x + (p - fixed).y * d.y) / dd
    } else {
        1.0
    };
    let lo = f32::from(SIZE_MIN) / f32::from(size0);
    let mut hi = f32::from(SIZE_MAX) / f32::from(size0);
    // the dragged corner must stay inside the picture
    let (w, h) = (frame.0 as f32, frame.1 as f32);
    for (f, dv, max) in [(fixed.x, d.x, w), (fixed.y, d.y, h)] {
        if dv > 0.0 {
            hi = hi.min(((max - f) / dv).max(0.0));
        } else if dv < 0.0 {
            hi = hi.min((f / -dv).max(0.0));
        }
    }
    let factor = raw.clamp(lo.min(hi), hi);
    let moved = fixed + d * factor;
    let rect = Rect::from_two_pos(fixed, moved);
    let size = (f32::from(size0) * factor)
        .round()
        .clamp(f32::from(SIZE_MIN), f32::from(SIZE_MAX)) as u16;
    Resized { rect, factor, size }
}

/// The stored position of a box (its centre) in frame pixels.
pub fn centre_pos(r: Rect, frame: (u32, u32)) -> Pos {
    super::placement::to_pos(r.center(), frame)
}

/// Align horizontally: the centre on the picture's vertical middle line, the other
/// coordinate kept.
pub fn align_h(p: Pos) -> Pos {
    Pos {
        x: POS_MAX / 2,
        ..p
    }
}

/// Align vertically: the centre on the picture's horizontal middle line.
pub fn align_v(p: Pos) -> Pos {
    Pos {
        y: POS_MAX / 2,
        ..p
    }
}

/// The size of an object that has none of its own.
pub const fn normal_size() -> u16 {
    SIZE_NORMAL
}

/// The corner handle squares of a box, `side` pixels wide, centred on the corners.
pub fn handle_rects(r: Rect, side: f32) -> [Rect; 4] {
    Corner::ALL.map(|c| Rect::from_center_size(c.of(r), eframe::egui::vec2(side, side)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    fn rect_at(centre: Pos2, w: f32, h: f32) -> Rect {
        Rect::from_center_size(centre, vec2(w, h))
    }

    fn rect() -> Rect {
        Rect::from_min_size(pos2(100.0, 100.0), vec2(200.0, 100.0))
    }

    // covers: REQ-097, REQ-096
    #[test]
    fn corners_win_over_the_body_and_the_cursor_follows() {
        let r = rect();
        assert_eq!(
            handle_at(r, pos2(102.0, 103.0), 8.0),
            Some(Hit::Corner(Corner::TopLeft))
        );
        assert_eq!(
            handle_at(r, pos2(298.0, 198.0), 8.0),
            Some(Hit::Corner(Corner::BottomRight))
        );
        // just outside the box but near a corner: still the handle
        assert_eq!(
            handle_at(r, pos2(305.0, 95.0), 8.0),
            Some(Hit::Corner(Corner::TopRight))
        );
        assert_eq!(handle_at(r, pos2(200.0, 150.0), 8.0), Some(Hit::Body));
        assert_eq!(handle_at(r, pos2(50.0, 50.0), 8.0), None);
        assert_eq!(handle_at(r, pos2(f32::NAN, 1.0), 8.0), None);
        assert_eq!(
            cursor(Hit::Corner(Corner::TopLeft), false),
            CursorIcon::ResizeNwSe
        );
        assert_eq!(
            cursor(Hit::Corner(Corner::BottomRight), false),
            CursorIcon::ResizeNwSe
        );
        assert_eq!(
            cursor(Hit::Corner(Corner::TopRight), true),
            CursorIcon::ResizeNeSw
        );
        assert_eq!(
            cursor(Hit::Corner(Corner::BottomLeft), false),
            CursorIcon::ResizeNeSw
        );
        assert_eq!(cursor(Hit::Body, false), CursorIcon::Grab);
        assert_eq!(cursor(Hit::Body, true), CursorIcon::Grabbing);
        // overlapping boxes: the one on top, and a corner of a lower box beats the body
        let a = BoxItem {
            id: ObjectId::Line(LineId(1)),
            rect: r,
            size: 1000,
        };
        let b = BoxItem {
            id: ObjectId::Line(LineId(2)),
            rect: r.translate(vec2(50.0, 0.0)),
            size: 1000,
        };
        let (hit, h) = hit_test(&[a, b], pos2(200.0, 150.0), 8.0).unwrap();
        assert_eq!((hit.id, h), (b.id, Hit::Body));
        let (hit, h) = hit_test(&[a, b], pos2(300.0, 200.0), 8.0).unwrap();
        assert_eq!((hit.id, h), (a.id, Hit::Corner(Corner::BottomRight)));
        assert!(hit_test(&[a, b], pos2(0.0, 0.0), 8.0).is_none());
        assert!(hit_test(&[], pos2(0.0, 0.0), 8.0).is_none());
        assert_eq!(normal_size(), 1000);
        assert_eq!(handle_rects(r, 6.0)[0].center(), r.left_top());
        assert_eq!(rect_at(pos2(10.0, 10.0), 4.0, 2.0).size(), vec2(4.0, 2.0));
    }

    // covers: REQ-097
    #[test]
    fn dragging_a_corner_scales_with_the_opposite_corner_fixed() {
        let r = rect();
        // drag the bottom-right corner along the diagonal to twice the size
        let p = pos2(100.0 + 400.0, 100.0 + 200.0);
        let out = resize(r, 1000, Corner::BottomRight, p, (2000, 1000));
        assert!((out.factor - 2.0).abs() < 1e-5);
        assert_eq!(out.size, 2000);
        assert_eq!(out.rect.min, r.min, "the opposite corner stays");
        assert!((out.rect.width() - 400.0).abs() < 1e-3);
        assert!((out.rect.height() - 200.0).abs() < 1e-3);
        // every corner keeps the proportions and its opposite corner
        for c in Corner::ALL {
            let fixed = c.opposite().of(r);
            let target = fixed + (c.of(r) - fixed) * 1.5;
            let out = resize(r, 1000, c, target, (2000, 1000));
            assert!((out.rect.width() / out.rect.height() - 2.0).abs() < 1e-4);
            assert!(c.opposite().of(out.rect).distance(fixed) < 1e-3, "{c:?}");
            assert_eq!(out.size, 1500);
        }
        // the pointer off the diagonal counts by its projection on it
        let out = resize(
            r,
            1000,
            Corner::BottomRight,
            pos2(300.0, 300.0),
            (2000, 1000),
        );
        let proj = ((200.0 * 200.0 + 200.0 * 100.0) / (200.0 * 200.0 + 100.0 * 100.0)) as f32;
        assert!((out.factor - proj).abs() < 1e-4);
    }

    // covers: REQ-097
    #[test]
    fn the_size_stays_between_25_and_400_percent_and_the_box_in_the_picture() {
        let r = rect();
        let frame = (10_000, 10_000);
        let tiny = resize(r, 1000, Corner::BottomRight, pos2(100.0, 100.0), frame);
        assert_eq!(tiny.size, 250);
        assert!((tiny.factor - 0.25).abs() < 1e-5);
        let huge = resize(r, 1000, Corner::BottomRight, pos2(9000.0, 9000.0), frame);
        assert_eq!(huge.size, 4000);
        // from an already big text the room is smaller
        let big = resize(r, 3000, Corner::BottomRight, pos2(9000.0, 9000.0), frame);
        assert_eq!(big.size, 4000);
        assert!((big.factor - 4.0 / 3.0).abs() < 1e-4);
        // the picture edge stops the growth before the size limit
        let out = resize(r, 1000, Corner::BottomRight, pos2(900.0, 900.0), (500, 400));
        assert!(out.rect.max.x <= 500.0 + 1e-3 && out.rect.max.y <= 400.0 + 1e-3);
        assert_eq!(out.rect.max.x, 500.0);
        // top-left corner towards the top edge
        let out = resize(r, 1000, Corner::TopLeft, pos2(-500.0, -500.0), (500, 400));
        assert!(out.rect.min.x >= -1e-3 && out.rect.min.y >= -1e-3);
        // hostile pointer values leave the size alone
        let out = resize(
            r,
            1000,
            Corner::BottomRight,
            pos2(f32::NAN, 0.0),
            (500, 400),
        );
        assert_eq!(out.size, 1000);
        let flat = Rect::from_min_size(pos2(1.0, 1.0), vec2(0.0, 0.0));
        let out = resize(flat, 0, Corner::BottomRight, pos2(5.0, 5.0), (10, 10));
        assert!((SIZE_MIN..=SIZE_MAX).contains(&out.size));
    }

    // covers: REQ-098
    #[test]
    fn align_moves_one_coordinate_to_the_middle() {
        let p = Pos { x: 1200, y: 8000 };
        assert_eq!(align_h(p), Pos { x: 5000, y: 8000 });
        assert_eq!(align_v(p), Pos { x: 1200, y: 5000 });
        assert_eq!(align_h(align_v(p)), Pos { x: 5000, y: 5000 });
        let r = rect_at(pos2(250.0, 150.0), 100.0, 50.0);
        assert_eq!(centre_pos(r, (1000, 300)), Pos { x: 2500, y: 5000 });
    }
}
