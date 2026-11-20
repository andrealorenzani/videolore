// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The handles of Scale and Move on the preview picture (REQ-127, REQ-128): a box around
//! the picture with a square handle at each corner, and a round target at its centre. The
//! geometry is pure (frame pixels), so the interaction can be tested without a window.

use super::boxes::{Corner, Side, partly_outside, side_at, visible_part};
use crate::base::pos::Pos;
use crate::frame::{MAX_OFFSET, MAX_SCALE, MIN_SCALE, Scale};
use crate::timeline::FrameTarget;
use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

/// What a pointer press on the picture grabbed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Corner(Corner),
    /// A visible side of the box (REQ-145): the part of the box on the picture.
    Edge(Side),
    Target,
}

/// A drag of a handle in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grab {
    pub target: FrameTarget,
    pub hit: Hit,
    /// The picture's rectangle when the drag started (frame pixels).
    pub rect: Rect,
    /// The Scale value when the drag started.
    pub scale: Scale,
    /// The Move value when the drag started.
    pub pos: Pos,
    /// Pointer minus the picture's centre at the start (frame pixels).
    pub grab: Vec2,
    pub frame: (u32, u32),
}

/// The picture's rectangle in frame pixels from fractions of the frame.
pub fn to_rect(f: crate::frame::FractionRect, frame: (u32, u32)) -> Rect {
    let (w, h) = (f64::from(frame.0), f64::from(frame.1));
    Rect::from_min_max(
        pos2((f.0 * w) as f32, (f.1 * h) as f32),
        pos2((f.2 * w) as f32, (f.3 * h) as f32),
    )
}

/// The handle under `p`: a corner of the Scale box that is on the picture first, then a
/// visible side of the box, then the Move target. `tol` is in frame pixels.
pub fn hit(
    rect: Rect,
    frame: (u32, u32),
    scale: bool,
    moved: bool,
    p: Pos2,
    tol: f32,
) -> Option<Hit> {
    if scale {
        let on_picture = |c: Pos2| !partly_outside(Rect::from_min_max(c, c), frame);
        if let Some(c) = Corner::ALL
            .into_iter()
            .map(|c| (c, c.of(rect)))
            .filter(|(_, at)| on_picture(*at) && at.distance(p) <= tol)
            .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
            .map(|(c, _)| c)
        {
            return Some(Hit::Corner(c));
        }
        if let Some(s) = side_at(rect, frame, p, tol) {
            return Some(Hit::Edge(s));
        }
    }
    (moved && rect.center().distance(p) <= tol * 1.5).then_some(Hit::Target)
}

/// The Scale value after dragging `corner` to `p`: the opposite corner stays where it is,
/// width and height change together (the shape never changes) and the factor stays within
/// the limits. The centre offset keeps the opposite corner fixed.
pub fn scale_to(g: &Grab, corner: Corner, p: Pos2) -> Scale {
    let o = corner.opposite().of(g.rect);
    let (w0, h0) = (g.rect.width().max(1.0), g.rect.height().max(1.0));
    let (sx, sy) = (
        if corner.of(g.rect).x >= o.x {
            1.0
        } else {
            -1.0
        },
        if corner.of(g.rect).y >= o.y {
            1.0
        } else {
            -1.0
        },
    );
    // how far the pointer is from the fixed corner, along each side, and never backwards
    let f = ((p.x - o.x) * sx / w0).max((p.y - o.y) * sy / h0).max(0.0);
    let old = f32::from(g.scale.permille);
    let permille = (old * f)
        .round()
        .clamp(f32::from(MIN_SCALE), f32::from(MAX_SCALE));
    let eff = permille / old;
    let (w, h) = (w0 * eff, h0 * eff);
    let centre = pos2(o.x + sx * w / 2.0, o.y + sy * h / 2.0);
    finish(g, permille as u16, centre)
}

/// The Scale value with the picture's centre at `centre` (frame pixels).
fn finish(g: &Grab, permille: u16, centre: Pos2) -> Scale {
    let moved = centre - g.rect.center();
    let off = |d: f32, size: u32, cur: i16| {
        (i32::from(cur) + (d / size.max(1) as f32 * 10_000.0).round() as i32)
            .clamp(-i32::from(MAX_OFFSET), i32::from(MAX_OFFSET)) as i16
    };
    Scale {
        permille,
        dx: off(moved.x, g.frame.0, g.scale.dx),
        dy: off(moved.y, g.frame.1, g.scale.dy),
    }
}

/// The Scale value after dragging the visible `side` of the box to `p` (REQ-145): the
/// middle of the opposite visible side stays where it is, width and height change together
/// (the ratio is kept at every step) and the factor stays within the limits. The visible
/// part is the whole box when it is on the picture, so this is the box edge itself then.
pub fn scale_to_edge(g: &Grab, side: Side, p: Pos2) -> Scale {
    let vis = visible_part(g.rect, g.frame).unwrap_or(g.rect);
    let o = side.opposite().mid(vis);
    let len = side.extent(vis).max(1.0);
    let f = if p.x.is_finite() && p.y.is_finite() {
        ((p - o).dot(side.outward()) / len).max(0.0)
    } else {
        1.0
    };
    let old = f32::from(g.scale.permille).max(1.0);
    let permille = (old * f)
        .round()
        .clamp(f32::from(MIN_SCALE), f32::from(MAX_SCALE));
    let eff = permille / old;
    finish(g, permille as u16, o + (g.rect.center() - o) * eff)
}

/// The Move value after dragging the target to `p`: the picture's centre follows the
/// pointer (less where it was grabbed) and stays on the frame.
pub fn move_to(g: &Grab, p: Pos2) -> Pos {
    let shift = (p - g.grab) - g.rect.center();
    let (fw, fh) = (f64::from(g.frame.0.max(1)), f64::from(g.frame.1.max(1)));
    let (x0, y0) = g.pos.fraction();
    Pos::from_fraction(x0 + f64::from(shift.x) / fw, y0 + f64::from(shift.y) / fh)
}

/// The rectangle drawn while a Scale drag is in progress.
pub fn scaled_rect(g: &Grab, s: Scale) -> Rect {
    let f = f32::from(s.permille) / f32::from(g.scale.permille).max(1.0);
    let (w, h) = (g.rect.width() * f, g.rect.height() * f);
    let shift = vec2(
        f32::from(s.dx - g.scale.dx) / 10_000.0 * g.frame.0 as f32,
        f32::from(s.dy - g.scale.dy) / 10_000.0 * g.frame.1 as f32,
    );
    Rect::from_center_size(g.rect.center() + shift, vec2(w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grab(rect: Rect, scale: Scale) -> Grab {
        Grab {
            target: FrameTarget::Whole,
            hit: Hit::Corner(Corner::BottomRight),
            rect,
            scale,
            pos: crate::frame::CENTRE,
            grab: Vec2::ZERO,
            frame: (1000, 500),
        }
    }

    const F: (u32, u32) = (1000, 500);

    fn rect() -> Rect {
        Rect::from_min_max(pos2(300.0, 100.0), pos2(700.0, 300.0))
    }

    // covers: REQ-127
    #[test]
    fn dragging_a_corner_keeps_the_opposite_corner_and_the_shape() {
        let g = grab(rect(), Scale::NORMAL);
        // the bottom right corner dragged outwards to twice the size
        let s = scale_to(&g, Corner::BottomRight, pos2(1100.0, 500.0));
        assert_eq!(s.permille, 2000);
        let r = scaled_rect(&g, s);
        assert!(
            (r.min.x - 300.0).abs() < 1.0 && (r.min.y - 100.0).abs() < 1.0,
            "{r:?}"
        );
        assert!((r.width() / r.height() - 2.0).abs() < 1e-3);
        // the top left corner, the opposite (bottom right) stays
        let s = scale_to(&g, Corner::TopLeft, pos2(500.0, 200.0));
        assert_eq!(s.permille, 500);
        let r = scaled_rect(&g, s);
        assert!(
            (r.max.x - 700.0).abs() < 1.0 && (r.max.y - 300.0).abs() < 1.0,
            "{r:?}"
        );
    }

    // covers: REQ-127
    #[test]
    fn the_limits_are_10_and_400_percent() {
        let g = grab(rect(), Scale::NORMAL);
        assert_eq!(
            scale_to(&g, Corner::BottomRight, pos2(310.0, 101.0)).permille,
            100
        );
        assert_eq!(
            scale_to(&g, Corner::BottomRight, pos2(9000.0, 9000.0)).permille,
            4000
        );
        // dragging past the fixed corner never inverts the picture
        assert_eq!(
            scale_to(&g, Corner::BottomRight, pos2(0.0, 0.0)).permille,
            100
        );
        // from a scaled value the limits still hold
        let g = grab(
            rect(),
            Scale {
                permille: 3000,
                dx: 0,
                dy: 0,
            },
        );
        assert_eq!(
            scale_to(&g, Corner::BottomRight, pos2(9000.0, 9000.0)).permille,
            4000
        );
    }

    // covers: REQ-128
    #[test]
    fn the_target_moves_the_centre_by_the_fraction_of_the_frame() {
        let g = grab(rect(), Scale::NORMAL);
        // 10% of the 1000 px wide frame to the right
        let p = move_to(&g, g.rect.center() + vec2(100.0, 0.0));
        assert_eq!(p, Pos { x: 6000, y: 5000 });
        // never past the frame's edge
        let p = move_to(&g, pos2(5000.0, -5000.0));
        assert_eq!(p, Pos { x: 10000, y: 0 });
        // where it was grabbed does not jump
        let mut g2 = g;
        g2.grab = vec2(20.0, 0.0);
        assert_eq!(
            move_to(&g2, g.rect.center() + vec2(20.0, 0.0)),
            crate::frame::CENTRE
        );
    }

    // covers: REQ-145
    #[test]
    fn the_visible_edges_of_a_box_cut_by_the_frame_are_handles() {
        // 16:9 picture (320x180 here) in a 9:16 frame (180x320) with Fit on height: the box
        // is 569 px wide on a 180 px frame, so the sides are cut
        let frame = (180, 320);
        let rect = Rect::from_center_size(pos2(90.0, 160.0), vec2(569.0, 320.0));
        // no corner of the box is on the picture; the visible sides are the handles
        assert_eq!(
            hit(rect, frame, true, false, pos2(1.0, 160.0), 6.0),
            Some(Hit::Edge(Side::Left))
        );
        assert_eq!(
            hit(rect, frame, true, false, pos2(179.0, 160.0), 6.0),
            Some(Hit::Edge(Side::Right))
        );
        assert_eq!(hit(rect, frame, true, false, pos2(90.0, 160.0), 6.0), None);
        // dragging the visible left side inwards by 60 px: visible width 180 -> 120 = 2/3
        let g = Grab {
            frame,
            ..grab(rect, Scale::NORMAL)
        };
        let s = scale_to_edge(&g, Side::Left, pos2(60.0, 160.0));
        assert_eq!(s.permille, 667);
        // the ratio is kept at every step
        let r = scaled_rect(&g, s);
        assert!((r.width() / r.height() - 569.0 / 320.0).abs() < 1e-2);
        // the picture point on the opposite (right, visible) side stays where it is
        let at = |r: Rect| (180.0 - r.left()) / r.width();
        assert!((at(r) - at(rect)).abs() < 1e-2);
    }

    // covers: REQ-145
    #[test]
    fn a_scale_of_300_percent_comes_back_in_one_drag_by_the_visible_edge() {
        // the box is 3x the 1000x500 frame, centred: its edges are all outside
        let rect = Rect::from_center_size(pos2(500.0, 250.0), vec2(3000.0, 1500.0));
        let scale = Scale {
            permille: 3000,
            dx: 0,
            dy: 0,
        };
        let g = Grab {
            frame: F,
            ..grab(rect, scale)
        };
        assert_eq!(
            hit(rect, F, true, false, pos2(2.0, 250.0), 8.0),
            Some(Hit::Edge(Side::Left))
        );
        // pointer 1/3 of the frame width from the right edge: factor 1/3 -> 100%
        let s = scale_to_edge(&g, Side::Left, pos2(2000.0 / 3.0, 250.0));
        assert_eq!(s.permille, 1000);
        // the opposite visible side did not move; the centre stays on the same line
        assert_eq!(s.dy, 0);
        // and the limits hold
        assert_eq!(
            scale_to_edge(&g, Side::Left, pos2(999.0, 250.0)).permille,
            100
        );
        assert_eq!(
            scale_to_edge(&g, Side::Left, pos2(f32::NAN, 0.0)).permille,
            3000
        );
        // a Scale box on the picture: the box edge itself, dragged outwards
        let g = grab(rect_inside(), Scale::NORMAL);
        let s = scale_to_edge(&g, Side::Right, pos2(900.0, 200.0));
        assert_eq!(s.permille, 1500);
        let r = scaled_rect(&g, s);
        assert!((r.left() - 300.0).abs() < 1.0, "{r:?}");
    }

    fn rect_inside() -> Rect {
        Rect::from_min_max(pos2(300.0, 100.0), pos2(700.0, 300.0))
    }

    #[test]
    fn corners_come_before_the_target() {
        let r = rect();
        assert_eq!(
            hit(r, F, true, true, pos2(702.0, 301.0), 8.0),
            Some(Hit::Corner(Corner::BottomRight))
        );
        assert_eq!(hit(r, F, true, true, r.center(), 8.0), Some(Hit::Target));
        assert_eq!(hit(r, F, false, true, pos2(702.0, 301.0), 8.0), None);
        assert_eq!(hit(r, F, true, false, r.center(), 8.0), None);
        assert_eq!(hit(r, F, true, true, pos2(0.0, 0.0), 8.0), None);
        let f = to_rect((0.25, 0.5, 0.75, 1.0), (200, 100));
        assert_eq!((f.min, f.max), (pos2(50.0, 50.0), pos2(150.0, 100.0)));
    }
}
