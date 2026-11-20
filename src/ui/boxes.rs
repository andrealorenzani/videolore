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

use crate::base::pos::Pos;
use crate::subtitles::{LineId, SIZE_MAX, SIZE_MIN, SIZE_NORMAL};
use eframe::egui::{CursorIcon, Pos2, Rect, Vec2, pos2, vec2};

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
    /// The box changes its width only, by a handle on its right edge (a text item,
    /// REQ-116); other boxes are resized by their corners.
    pub width_only: bool,
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

/// One side of a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Bottom, Side::Left, Side::Right];

    /// The side on the other end of the box (the one that stays put when resizing).
    pub fn opposite(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// The unit vector pointing out of the box through this side.
    pub fn outward(self) -> Vec2 {
        match self {
            Side::Top => vec2(0.0, -1.0),
            Side::Bottom => vec2(0.0, 1.0),
            Side::Left => vec2(-1.0, 0.0),
            Side::Right => vec2(1.0, 0.0),
        }
    }

    /// The middle of this side of `r`.
    pub fn mid(self, r: Rect) -> Pos2 {
        match self {
            Side::Top => r.center_top(),
            Side::Bottom => r.center_bottom(),
            Side::Left => r.left_center(),
            Side::Right => r.right_center(),
        }
    }

    /// The length of `r` across this side's normal (its width for left and right).
    pub fn extent(self, r: Rect) -> f32 {
        match self {
            Side::Top | Side::Bottom => r.height(),
            Side::Left | Side::Right => r.width(),
        }
    }

    /// Distance from `p` to this side of `r` seen as a segment (`None` when `p` is not
    /// within `tol` of it along the side).
    fn distance(self, r: Rect, p: Pos2, tol: f32) -> Option<f32> {
        let (along, across) = match self {
            Side::Top | Side::Bottom => (p.x, (p.y - self.mid(r).y).abs()),
            Side::Left | Side::Right => (p.y, (p.x - self.mid(r).x).abs()),
        };
        let (lo, hi) = match self {
            Side::Top | Side::Bottom => (r.left(), r.right()),
            Side::Left | Side::Right => (r.top(), r.bottom()),
        };
        (along >= lo - tol && along <= hi + tol && across <= tol).then_some(across)
    }
}

/// The picture's rectangle in frame pixels.
fn frame_rect(frame: (u32, u32)) -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(frame.0 as f32, frame.1 as f32))
}

/// The part of `rect` that lies on the picture (`None` when none of it does).
pub fn visible_part(rect: Rect, frame: (u32, u32)) -> Option<Rect> {
    let v = rect.intersect(frame_rect(frame));
    (v.width() > 0.0 && v.height() > 0.0).then_some(v)
}

/// True when `rect` reaches beyond the picture.
pub fn partly_outside(rect: Rect, frame: (u32, u32)) -> bool {
    !frame_rect(frame).contains_rect(rect)
}

/// The side of the visible part of `rect` under `p` (the nearest one within `tol`).
pub fn side_at(rect: Rect, frame: (u32, u32), p: Pos2, tol: f32) -> Option<Side> {
    let vis = visible_part(rect, frame)?;
    let tol = tol.max(0.0);
    Side::ALL
        .into_iter()
        .filter_map(|s| s.distance(vis, p, tol).map(|d| (s, d)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(s, _)| s)
}

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Corner(Corner),
    /// A visible side of a box that reaches beyond the picture: resizes it in proportion
    /// (REQ-146).
    Side(Side),
    /// The handle on the right edge of a width-only box.
    Edge,
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
pub fn hit_test(boxes: &[BoxItem], p: Pos2, tol: f32, frame: (u32, u32)) -> Option<(BoxItem, Hit)> {
    let corner = boxes.iter().rev().find_map(|b| {
        if b.width_only {
            return (edge_handle_at(b.rect, p, tol, frame)).then_some((*b, Hit::Edge));
        }
        match handle_at(b.rect, p, tol) {
            Some(h @ Hit::Corner(_)) => Some((*b, h)),
            _ if partly_outside(b.rect, frame) => {
                side_at(b.rect, frame, p, tol).map(|s| (*b, Hit::Side(s)))
            }
            _ => None,
        }
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
        Hit::Side(s) => side_cursor(s),
        Hit::Edge => CursorIcon::ResizeHorizontal,
        Hit::Body if dragging => CursorIcon::Grabbing,
        Hit::Body => CursorIcon::Grab,
    }
}

/// The cursor of a resize handle on `side`.
pub fn side_cursor(side: Side) -> CursorIcon {
    match side {
        Side::Top | Side::Bottom => CursorIcon::ResizeVertical,
        Side::Left | Side::Right => CursorIcon::ResizeHorizontal,
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

/// Resizes box `r0` (size `size0`) by dragging its visible `side` to the pointer `p`, in
/// proportion: the middle of the opposite visible side stays where it is and the size stays
/// between 25% and 400% (REQ-146). Used when the corners are outside the picture.
pub fn resize_side(r0: Rect, size0: u16, side: Side, p: Pos2, frame: (u32, u32)) -> Resized {
    let size0 = size0.clamp(SIZE_MIN, SIZE_MAX);
    let vis = visible_part(r0, frame).unwrap_or(r0);
    let fixed = side.opposite().mid(vis);
    let len = side.extent(vis).max(1.0);
    let raw = if p.x.is_finite() && p.y.is_finite() {
        (p - fixed).dot(side.outward()) / len
    } else {
        1.0
    };
    let lo = f32::from(SIZE_MIN) / f32::from(size0);
    let hi = f32::from(SIZE_MAX) / f32::from(size0);
    let factor = raw.clamp(lo, hi);
    let centre = fixed + (r0.center() - fixed) * factor;
    let size = (f32::from(size0) * factor)
        .round()
        .clamp(f32::from(SIZE_MIN), f32::from(SIZE_MAX)) as u16;
    Resized {
        rect: Rect::from_center_size(centre, r0.size() * factor),
        factor,
        size,
    }
}

/// The stored position of a box (its centre) in frame pixels.
pub fn centre_pos(r: Rect, frame: (u32, u32)) -> Pos {
    super::placement::to_pos(r.center(), frame)
}

/// The size of an object that has none of its own.
pub const fn normal_size() -> u16 {
    SIZE_NORMAL
}

/// True when `p` is on the right-edge handle of box `r` (the middle of its right edge,
/// within `tol`).
pub fn edge_handle_at(r: Rect, p: Pos2, tol: f32, frame: (u32, u32)) -> bool {
    let tol = tol.max(0.0);
    let Some(v) = visible_part(r, frame) else {
        return false;
    };
    (p.x - v.right()).abs() <= tol && p.y >= v.top() - tol && p.y <= v.bottom() + tol
}

/// The square of the right-edge handle of a width-only box: on its visible part, so a box
/// whose right edge is outside the picture keeps the handle at the picture's edge.
pub fn edge_handle_rect(r: Rect, side: f32, frame: (u32, u32)) -> Rect {
    let at = visible_part(r, frame).unwrap_or(r).right_center();
    Rect::from_center_size(at, eframe::egui::vec2(side, side))
}

/// Where the handle squares of a box go: the corners that are on the picture and, for a
/// box that reaches beyond it, the middle of each visible side; for a width-only box, its
/// right-edge handle (REQ-145, REQ-146).
pub fn handle_points(r: Rect, frame: (u32, u32), width_only: bool) -> Vec<Pos2> {
    if width_only {
        return vec![edge_handle_rect(r, 0.0, frame).center()];
    }
    let fr = frame_rect(frame);
    let mut v: Vec<Pos2> = Corner::ALL
        .into_iter()
        .map(|c| c.of(r))
        .filter(|c| fr.contains(*c))
        .collect();
    if partly_outside(r, frame)
        && let Some(vis) = visible_part(r, frame)
    {
        v.extend(Side::ALL.into_iter().map(|s| s.mid(vis)));
    }
    v
}

/// Resizes the width of a text box `r0` by dragging its right edge to the pointer `p` (frame
/// pixels): the left edge and the height stay, the width stays between 10% and 100% of
/// the picture (REQ-116). Gives the new box and its width in thousandths of the picture.
pub fn resize_width(r0: Rect, p: Pos2, frame: (u32, u32)) -> (Rect, u16) {
    let fw = frame.0.max(1) as f32;
    let raw = if p.x.is_finite() {
        p.x - r0.left()
    } else {
        r0.width()
    };
    let lo = f32::from(crate::text::WIDTH_MIN) * fw / 1000.0;
    let hi = f32::from(crate::text::WIDTH_MAX) * fw / 1000.0;
    let w = raw.clamp(lo, hi);
    let permille = (w / fw * 1000.0).round().clamp(
        f32::from(crate::text::WIDTH_MIN),
        f32::from(crate::text::WIDTH_MAX),
    ) as u16;
    (
        Rect::from_min_size(r0.min, eframe::egui::vec2(w, r0.height())),
        permille,
    )
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
            width_only: false,
        };
        let b = BoxItem {
            id: ObjectId::Line(LineId(2)),
            rect: r.translate(vec2(50.0, 0.0)),
            size: 1000,
            width_only: false,
        };
        let (hit, h) = hit_test(&[a, b], pos2(200.0, 150.0), 8.0, (1000, 500)).unwrap();
        assert_eq!((hit.id, h), (b.id, Hit::Body));
        let (hit, h) = hit_test(&[a, b], pos2(300.0, 200.0), 8.0, (1000, 500)).unwrap();
        assert_eq!((hit.id, h), (a.id, Hit::Corner(Corner::BottomRight)));
        assert!(hit_test(&[a, b], pos2(0.0, 0.0), 8.0, (1000, 500)).is_none());
        assert!(hit_test(&[], pos2(0.0, 0.0), 8.0, (1000, 500)).is_none());
        assert_eq!(normal_size(), 1000);
        assert_eq!(handle_rects(r, 6.0)[0].center(), r.left_top());
        assert_eq!(rect_at(pos2(10.0, 10.0), 4.0, 2.0).size(), vec2(4.0, 2.0));
    }

    const FRAME: (u32, u32) = (1000, 500);

    fn item(rect: Rect, width_only: bool) -> BoxItem {
        BoxItem {
            id: ObjectId::Line(LineId(1)),
            rect,
            size: 1000,
            width_only,
        }
    }

    // covers: REQ-146
    #[test]
    fn the_visible_part_and_sides_of_a_box_partly_outside_the_picture() {
        // a box centred on the right edge of the picture
        let r = Rect::from_center_size(pos2(1000.0, 250.0), vec2(200.0, 100.0));
        let v = visible_part(r, FRAME).unwrap();
        assert_eq!((v.min, v.max), (pos2(900.0, 200.0), pos2(1000.0, 300.0)));
        assert!(partly_outside(r, FRAME));
        assert!(!partly_outside(v, FRAME));
        assert!(visible_part(r.translate(vec2(500.0, 0.0)), FRAME).is_none());
        // the sides are the ones of the visible part, the nearest wins
        assert_eq!(side_at(r, FRAME, pos2(950.0, 201.0), 8.0), Some(Side::Top));
        assert_eq!(
            side_at(r, FRAME, pos2(950.0, 299.0), 8.0),
            Some(Side::Bottom)
        );
        assert_eq!(side_at(r, FRAME, pos2(901.0, 250.0), 8.0), Some(Side::Left));
        assert_eq!(
            side_at(r, FRAME, pos2(999.0, 250.0), 8.0),
            Some(Side::Right)
        );
        assert_eq!(side_at(r, FRAME, pos2(950.0, 250.0), 8.0), None);
        assert_eq!(
            side_at(r.translate(vec2(900.0, 0.0)), FRAME, pos2(0.0, 0.0), 8.0),
            None
        );
        // helpers
        for s in Side::ALL {
            assert_eq!(s.opposite().opposite(), s);
            assert_eq!(s.outward().dot(s.opposite().outward()), -1.0);
        }
        assert_eq!(Side::Top.extent(v), 100.0);
        assert_eq!(Side::Left.extent(v), 100.0);
        assert_eq!(Side::Top.mid(v), pos2(950.0, 200.0));
        assert_eq!(side_cursor(Side::Top), CursorIcon::ResizeVertical);
        assert_eq!(side_cursor(Side::Left), CursorIcon::ResizeHorizontal);
        assert_eq!(
            cursor(Hit::Side(Side::Bottom), false),
            CursorIcon::ResizeVertical
        );
        // hit test: a visible side is a handle for a subtitle box that reaches outside ...
        let b = item(r, false);
        let (_, h) = hit_test(&[b], pos2(950.0, 201.0), 8.0, FRAME).unwrap();
        assert_eq!(h, Hit::Side(Side::Top));
        // ... a visible corner still wins ...
        let (_, h) = hit_test(&[b], pos2(901.0, 201.0), 8.0, FRAME).unwrap();
        assert_eq!(h, Hit::Corner(Corner::TopLeft));
        // ... and a box wholly on the picture keeps its edge for moving
        let inside = item(rect(), false);
        let (_, h) = hit_test(&[inside], pos2(200.0, 101.0), 8.0, FRAME).unwrap();
        assert_eq!(h, Hit::Body);
        // handle squares: visible corners and the middle of each visible side; inside: corners only
        let pts = handle_points(r, FRAME, false);
        assert_eq!(pts.len(), 2 + 4);
        assert!(pts.contains(&pos2(950.0, 200.0)));
        assert_eq!(handle_points(rect(), FRAME, false).len(), 4);
    }

    // covers: REQ-146
    #[test]
    fn dragging_a_visible_side_resizes_in_proportion_with_the_opposite_side_fixed() {
        let r = Rect::from_center_size(pos2(1000.0, 250.0), vec2(200.0, 100.0));
        // top side of the visible part pulled up by 100 px: the visible height 100 -> 200
        let out = resize_side(r, 1000, Side::Top, pos2(950.0, 100.0), FRAME);
        assert_eq!(out.size, 2000);
        assert!((out.factor - 2.0).abs() < 1e-4);
        assert!((out.rect.width() / out.rect.height() - 2.0).abs() < 1e-4);
        // the middle of the visible bottom side did not move
        assert!((out.rect.bottom() - 300.0).abs() < 1e-3);
        assert!((out.rect.center().x - 1000.0 - (1000.0 - 950.0) * 0.0 - 50.0).abs() < 1e-3);
        // limits: 25% .. 400%, whatever the pointer does
        assert_eq!(
            resize_side(r, 1000, Side::Top, pos2(950.0, 299.0), FRAME).size,
            250
        );
        assert_eq!(
            resize_side(r, 1000, Side::Top, pos2(950.0, -9e5), FRAME).size,
            4000
        );
        assert_eq!(
            resize_side(r, 1000, Side::Top, pos2(f32::NAN, 0.0), FRAME).size,
            1000
        );
        // a box on the picture: the side is the box side itself
        let out = resize_side(rect(), 1000, Side::Right, pos2(500.0, 150.0), FRAME);
        assert!((out.rect.left() - 100.0).abs() < 1e-3);
        assert_eq!(out.size, 2000);
    }

    // covers: REQ-146
    #[test]
    fn a_text_box_keeps_its_width_handle_at_the_picture_edge() {
        // the right edge is 100 px past the picture
        let r = Rect::from_min_size(pos2(800.0, 100.0), vec2(300.0, 60.0));
        let t = item(r, true);
        let (_, h) = hit_test(&[t], pos2(998.0, 130.0), 8.0, FRAME).unwrap();
        assert_eq!(h, Hit::Edge);
        assert!(edge_handle_at(r, pos2(1000.0, 130.0), 8.0, FRAME));
        assert!(!edge_handle_at(
            r.translate(vec2(900.0, 0.0)),
            pos2(0.0, 0.0),
            8.0,
            FRAME
        ));
        assert_eq!(
            edge_handle_rect(r, 7.0, FRAME).center(),
            pos2(1000.0, 130.0)
        );
        assert_eq!(handle_points(r, FRAME, true), vec![pos2(1000.0, 130.0)]);
        // dragging it changes the width only
        let (out, w) = resize_width(r, pos2(950.0, 130.0), FRAME);
        assert_eq!((out.min, out.height()), (r.min, r.height()));
        assert_eq!(w, 150);
    }

    // covers: REQ-116
    #[test]
    fn a_text_box_has_one_handle_on_its_right_edge_and_changes_its_width_only() {
        let r = rect();
        let t = BoxItem {
            id: ObjectId::Text(crate::text::TextId(1)),
            rect: r,
            size: 1000,
            width_only: true,
        };
        // the corners do not resize it: they are only the body
        let (_, h) = hit_test(&[t], pos2(102.0, 103.0), 8.0, (1000, 500)).unwrap();
        assert_eq!(h, Hit::Body);
        // the middle of the right edge (and a little above and below) is the handle
        let (_, h) = hit_test(&[t], pos2(299.0, 150.0), 8.0, (1000, 500)).unwrap();
        assert_eq!(h, Hit::Edge);
        let (_, h) = hit_test(&[t], pos2(304.0, 110.0), 8.0, (1000, 500)).unwrap();
        assert_eq!(h, Hit::Edge);
        assert_eq!(cursor(Hit::Edge, false), CursorIcon::ResizeHorizontal);
        assert_eq!(
            edge_handle_rect(r, 7.0, (1000, 500)).center(),
            r.right_center()
        );
        // dragging: the left edge and the height stay; 1000 px wide frame
        let frame = (1000, 500);
        let (out, w) = resize_width(r, pos2(400.0, 999.0), frame);
        assert_eq!((out.min, out.height()), (r.min, r.height()));
        assert_eq!(w, 300);
        // narrower than 10 % stops at 10 %, wider than the picture stops at 100 %
        let (out, w) = resize_width(r, pos2(110.0, 0.0), frame);
        assert_eq!((w, out.width()), (100, 100.0));
        let (_, w) = resize_width(r, pos2(9000.0, 0.0), frame);
        assert_eq!(w, 1000);
        let (_, w) = resize_width(r, pos2(f32::NAN, 0.0), frame);
        assert_eq!(w, 200);
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
        use crate::base::pos::{align_h, align_v};
        let p = Pos { x: 1200, y: 8000 };
        assert_eq!(align_h(p), Pos { x: 5000, y: 8000 });
        assert_eq!(align_v(p), Pos { x: 1200, y: 5000 });
        assert_eq!(align_h(align_v(p)), Pos { x: 5000, y: 5000 });
        let r = rect_at(pos2(250.0, 150.0), 100.0, 50.0);
        assert_eq!(centre_pos(r, (1000, 300)), Pos { x: 2500, y: 5000 });
    }
}
