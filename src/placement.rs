// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The placement rules of subtitle lines and text items (REQ-168 to REQ-176): the box of a
//! line or a text on the picture, which boxes overlap, which are partly outside, and the
//! least move that puts a box inside. Everything is pure and works in output-frame pixels
//! (the ASS `PlayRes` space), with text measured by [`crate::text`] and [`crate::measure`],
//! so the editor and the commands that change the project agree with the export.

use crate::ass;
use crate::base::pos::{POS_MAX, Pos};
use crate::project::Project;
use crate::selection::ObjRef;
use crate::subtitles::{Preset, ShownLine};
use crate::text::{self, TextItem};
use crate::time::Micros;

/// How close (as a share of the picture's width or height) a dragged box's centre must be to
/// a guide to snap to it (REQ-198).
pub const SNAP_TOLERANCE: f64 = 0.01;
/// The vertical and horizontal guides besides the centre lines: the thirds.
pub const THIRDS: [f64; 2] = [0.33, 0.67];
/// Action-safe and title-safe areas as shares of the picture (REQ-199).
pub const ACTION_SAFE: f64 = 0.90;
pub const TITLE_SAFE: f64 = 0.80;

/// The guide lines a snapped box sits on, in frame pixels (`x`: a vertical line).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Guides {
    pub x: Option<f64>,
    pub y: Option<f64>,
}

/// Snaps the centre `c` of a dragged box to the centre lines and thirds of a `frame` picture
/// and to the centres of `others`, each axis on its own, within [`SNAP_TOLERANCE`] of the
/// picture size (REQ-198). Returns the centre to use and the guides it is on.
pub fn snap(c: (f64, f64), frame: (u32, u32), others: &[(f64, f64)]) -> ((f64, f64), Guides) {
    let axis = |v: f64, size: u32, centres: &mut dyn Iterator<Item = f64>| -> Option<f64> {
        let size = f64::from(size);
        let tol = SNAP_TOLERANCE * size;
        let fixed = [0.5, THIRDS[0], THIRDS[1]].map(|f| f * size);
        fixed
            .into_iter()
            .chain(centres)
            .filter(|g| (g - v).abs() <= tol)
            .min_by(|a, b| (a - v).abs().total_cmp(&(b - v).abs()))
    };
    let gx = axis(c.0, frame.0, &mut others.iter().map(|o| o.0));
    let gy = axis(c.1, frame.1, &mut others.iter().map(|o| o.1));
    (
        (gx.unwrap_or(c.0), gy.unwrap_or(c.1)),
        Guides { x: gx, y: gy },
    )
}

/// The action-safe and title-safe rectangles of a `frame` picture: `(left, top, right,
/// bottom)` in frame pixels (REQ-199).
pub fn safe_area(frame: (u32, u32), share: f64) -> (f64, f64, f64, f64) {
    let (w, h) = (f64::from(frame.0), f64::from(frame.1));
    (
        (1.0 - share) / 2.0 * w,
        (1.0 - share) / 2.0 * h,
        (1.0 + share) / 2.0 * w,
        (1.0 + share) / 2.0 * h,
    )
}

/// A box on the picture, by its centre and size (frame pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub cx: f64,
    pub cy: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn left(&self) -> f64 {
        self.cx - self.w / 2.0
    }
    pub fn right(&self) -> f64 {
        self.cx + self.w / 2.0
    }
    pub fn top(&self) -> f64 {
        self.cy - self.h / 2.0
    }
    pub fn bottom(&self) -> f64 {
        self.cy + self.h / 2.0
    }

    /// True when the two boxes share some area (touching edges do not count).
    pub fn overlaps(&self, o: &Rect) -> bool {
        (self.cx - o.cx).abs() < (self.w + o.w) / 2.0 - EPS
            && (self.cy - o.cy).abs() < (self.h + o.h) / 2.0 - EPS
    }

    /// True when some of the box is outside the picture (REQ-175).
    pub fn partly_outside(&self, frame: (u32, u32)) -> bool {
        self.left() < -EPS
            || self.top() < -EPS
            || self.right() > f64::from(frame.0) + EPS
            || self.bottom() > f64::from(frame.1) + EPS
    }

    /// The box moved by (`dx`, `dy`).
    pub fn moved(&self, dx: f64, dy: f64) -> Rect {
        Rect {
            cx: self.cx + dx,
            cy: self.cy + dy,
            ..*self
        }
    }

    /// The stored place of the centre.
    pub fn pos(&self, frame: (u32, u32)) -> Pos {
        let f = |v: f64, max: u32| {
            if max == 0 {
                return POS_MAX / 2;
            }
            (v / f64::from(max) * f64::from(POS_MAX))
                .round()
                .clamp(0.0, f64::from(POS_MAX)) as u16
        };
        Pos {
            x: f(self.cx, frame.0),
            y: f(self.cy, frame.1),
        }
    }
}

/// Tolerance of the comparisons, in pixels (a box on the edge is inside).
const EPS: f64 = 0.5;

/// The text of a line that is on screen at `at`: the whole line, or for the word-highlight
/// style the group of words shown at that moment.
pub fn visible_text(l: &ShownLine, preset: Preset, at: Micros) -> String {
    if preset != Preset::WordHighlight {
        return l.text.clone();
    }
    let words = ass::words_of(l);
    let i = l.word_at(at).unwrap_or(0);
    words
        .chunks(ass::HIGHLIGHT_CHUNK)
        .nth(i / ass::HIGHLIGHT_CHUNK)
        .map(|c| {
            c.iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// The size of a subtitle line's box: its text wrapped like the renderer does (inside the
/// margins), at the line's font size, plus the padding. The font is the bundled subtitle
/// font.
pub fn line_size(preset: Preset, text: &str, size: Option<u16>, frame: (u32, u32)) -> (f64, f64) {
    let (w, h) = frame;
    let font = match size {
        Some(s) => ass::sized_font(w, h, s),
        None => ass::font_size(w, h),
    };
    let wrap = (f64::from(w) - 2.0 * f64::from(ass::margin_lr(w))).max(1.0);
    let l = text::layout_at(text, "", f64::from(font), wrap);
    let pad = ass::box_padding(preset, font);
    (l.width + 2.0 * pad, l.height() + 2.0 * pad)
}

/// The box of a line of text `text` on the picture: centred on its own place, or at the
/// style's place (bottom-anchored, so it grows upwards).
pub fn line_rect(
    preset: Preset,
    text: &str,
    position: Option<Pos>,
    size: Option<u16>,
    frame: (u32, u32),
) -> Rect {
    let (bw, bh) = line_size(preset, text, size, frame);
    let (cx, cy) = match position {
        Some(p) => {
            let (fx, fy) = p.fraction();
            (fx * f64::from(frame.0), fy * f64::from(frame.1))
        }
        None => ass::default_centre(preset, frame.0, frame.1, bh),
    };
    Rect {
        cx,
        cy,
        w: bw,
        h: bh,
    }
}

/// The box a text item is laid out in: its width and height at its place (REQ-116).
pub fn text_rect(item: &TextItem, frame: (u32, u32)) -> Rect {
    let (w, h) = text::box_px(item, frame.0, frame.1);
    let (fx, fy) = item.place.fraction();
    Rect {
        cx: fx * f64::from(frame.0),
        cy: fy * f64::from(frame.1),
        w,
        h,
    }
}

/// The least move (`dx`, `dy`) that puts the whole box inside the picture; a box wider or
/// taller than the picture is centred on that axis (REQ-168, REQ-175).
pub fn least_inside(r: &Rect, frame: (u32, u32)) -> (f64, f64) {
    let axis = |lo: f64, hi: f64, size: f64, max: f64| {
        if size >= max {
            max / 2.0 - (lo + hi) / 2.0
        } else if lo < 0.0 {
            -lo
        } else if hi > max {
            max - hi
        } else {
            0.0
        }
    };
    (
        axis(r.left(), r.right(), r.w, f64::from(frame.0)),
        axis(r.top(), r.bottom(), r.h, f64::from(frame.1)),
    )
}

/// The place a line gets when its text changes to `new_text` (REQ-168, REQ-174): the centre
/// of its box stays where it was, the box then grows or shrinks around it, and a box that
/// grew and is now partly outside is moved back by the least move. `None`: the place stays
/// as it is (no own place is needed).
pub fn place_after_text_edit(
    project: &Project,
    id: crate::subtitles::LineId,
    new_text: &str,
    frame: (u32, u32),
) -> Option<Pos> {
    let subs = &project.subtitles;
    let l = subs.get(id)?;
    let new_text = crate::subtitles::clean_text(new_text);
    let old = line_rect(subs.preset, &l.text, l.position, l.size, frame);
    let new = line_rect(subs.preset, &new_text, l.position, l.size, frame);
    let mut r = Rect {
        cx: old.cx,
        cy: old.cy,
        w: new.w,
        h: new.h,
    };
    let enlarged = new.w > old.w + EPS || new.h > old.h + EPS;
    if enlarged && r.partly_outside(frame) {
        let (dx, dy) = least_inside(&r, frame);
        r = r.moved(dx, dy);
    }
    let centre_moved = (r.cx - old.cx).abs() > EPS || (r.cy - old.cy).abs() > EPS;
    let height_changed = (new.h - old.h).abs() > EPS;
    // an own place already keeps the centre; the style's place keeps the bottom, so it
    // needs an own place only when the height changes
    let needed = match l.position {
        Some(_) => centre_moved,
        None => centre_moved || height_changed,
    };
    needed.then(|| r.pos(frame))
}

/// The boxes of the lines and texts shown at `at` (not those fixed to the sphere, whose
/// place depends on the camera).
pub fn rects_at(project: &Project, at: Micros, frame: (u32, u32)) -> Vec<(ObjRef, Rect)> {
    let preset = project.subtitles.preset;
    let mut v: Vec<(ObjRef, Rect)> = project
        .subtitles
        .shown_at(&project.timeline, at)
        .into_iter()
        .filter(|l| l.anchor.is_none() && !l.text.trim().is_empty())
        .map(|l| {
            let t = visible_text(&l, preset, at);
            (
                ObjRef::Line(l.line),
                line_rect(preset, &t, l.position, l.size, frame),
            )
        })
        .collect();
    v.extend(
        project
            .texts
            .shown_at(at)
            .into_iter()
            .filter(|t| t.anchor.is_none())
            .map(|t| (ObjRef::Text(t.id), text_rect(t, frame))),
    );
    v
}

/// The objects at `at` that are partly outside the picture (REQ-175, REQ-176).
pub fn partly_outside_at(project: &Project, at: Micros, frame: (u32, u32)) -> Vec<ObjRef> {
    rects_at(project, at, frame)
        .into_iter()
        .filter(|(_, r)| r.partly_outside(frame))
        .map(|(o, _)| o)
        .collect()
}

/// The lines shown at the same time whose boxes overlap (REQ-169): every line that is in
/// such a pair. Derived from the project, never stored.
pub fn overlapping_lines(
    project: &Project,
    frame: (u32, u32),
) -> std::collections::BTreeSet<crate::subtitles::LineId> {
    let preset = project.subtitles.preset;
    let shown: Vec<ShownLine> = project
        .subtitles
        .project(&project.timeline)
        .into_iter()
        .filter(|l| l.anchor.is_none() && !l.text.trim().is_empty())
        .collect();
    let mut out = std::collections::BTreeSet::new();
    for (i, a) in shown.iter().enumerate() {
        for b in &shown[i + 1..] {
            // sorted by start: nothing later can overlap in time
            if b.start >= a.end {
                break;
            }
            let at = a.start.max(b.start);
            let ra = line_rect(
                preset,
                &visible_text(a, preset, at),
                a.position,
                a.size,
                frame,
            );
            let rb = line_rect(
                preset,
                &visible_text(b, preset, at),
                b.position,
                b.size,
                frame,
            );
            if ra.overlaps(&rb) {
                out.insert(a.line);
                out.insert(b.line);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: (u32, u32) = (1920, 1080);

    fn rect(cx: f64, cy: f64, w: f64, h: f64) -> Rect {
        Rect { cx, cy, w, h }
    }

    // covers: REQ-168
    #[test]
    fn the_least_move_puts_the_box_inside_and_centres_one_that_is_too_big() {
        // 10% of the width outside on the right: move left by that much
        let r = rect(1900.0, 540.0, 400.0, 100.0);
        assert!(r.partly_outside(FRAME));
        let (dx, dy) = least_inside(&r, FRAME);
        assert_eq!((dx, dy), (-180.0, 0.0));
        assert!(!r.moved(dx, dy).partly_outside(FRAME));
        assert_eq!(r.moved(dx, dy).right(), 1920.0);
        // outside on the top and the left
        let (dx, dy) = least_inside(&rect(100.0, 20.0, 400.0, 100.0), FRAME);
        assert_eq!((dx, dy), (100.0, 30.0));
        // inside: no move
        assert_eq!(
            least_inside(&rect(960.0, 540.0, 400.0, 100.0), FRAME),
            (0.0, 0.0)
        );
        // wider than the picture: centred on that axis; taller: centred vertically
        let (dx, dy) = least_inside(&rect(100.0, 540.0, 3000.0, 100.0), FRAME);
        assert_eq!((dx, dy), (860.0, 0.0));
        let (dx, dy) = least_inside(&rect(960.0, 900.0, 100.0, 2000.0), FRAME);
        assert_eq!((dx, dy), (0.0, -360.0));
        assert_eq!(
            rect(960.0, 540.0, 100.0, 100.0).pos(FRAME),
            Pos { x: 5000, y: 5000 }
        );
        assert_eq!(rect(0.0, 0.0, 1.0, 1.0).pos((0, 0)).x, POS_MAX / 2);
    }

    // covers: REQ-169
    #[test]
    fn boxes_that_share_area_overlap_and_boxes_that_touch_do_not() {
        let a = rect(100.0, 100.0, 100.0, 50.0);
        assert!(a.overlaps(&rect(150.0, 110.0, 100.0, 50.0)));
        assert!(!a.overlaps(&rect(200.0, 100.0, 100.0, 50.0)), "touching");
        assert!(!a.overlaps(&rect(100.0, 300.0, 100.0, 50.0)));
    }

    // covers: REQ-175
    #[test]
    fn a_box_on_the_edge_is_inside_and_a_box_past_it_is_partly_outside() {
        assert!(!rect(960.0, 540.0, 1920.0, 1080.0).partly_outside(FRAME));
        assert!(rect(960.0, 540.0, 1930.0, 1080.0).partly_outside(FRAME));
        assert!(rect(0.0, 540.0, 100.0, 100.0).partly_outside(FRAME));
    }

    // covers: REQ-174
    #[test]
    fn a_line_at_the_styles_place_grows_upwards_and_one_with_a_place_around_its_centre() {
        let one = line_rect(Preset::Classic, "Hello", None, None, FRAME);
        let two = line_rect(
            Preset::Classic,
            &"Hello world ".repeat(12),
            None,
            None,
            FRAME,
        );
        assert!(two.h > one.h, "{} {}", two.h, one.h);
        // bottom-anchored: the bottom stays, the centre moves
        assert!((two.bottom() - one.bottom()).abs() < 1e-6);
        assert!(two.cy < one.cy);
        // with its own place the centre is the place
        let p = Some(Pos { x: 5000, y: 8000 });
        let a = line_rect(Preset::Classic, "Hello", p, None, FRAME);
        let b = line_rect(Preset::Classic, &"Hello world ".repeat(12), p, None, FRAME);
        assert_eq!((a.cx, a.cy), (b.cx, b.cy));
        assert_eq!((a.cx, a.cy), (960.0, 864.0));
        // a bigger size makes a bigger box
        let big = line_rect(Preset::Classic, "Hello", p, Some(2000), FRAME);
        assert!(big.w > a.w && big.h > a.h);
    }

    // covers: REQ-176
    #[test]
    fn a_text_box_keeps_its_place_as_a_fraction_of_the_frame_whatever_its_size() {
        let mut t = TextItem::new(text::TextId(1), "x".into(), Micros::ZERO);
        t.place = Pos { x: 9000, y: 5000 };
        t.width = 400;
        let small = text_rect(&t, (1920, 1080));
        let big = text_rect(&t, (3840, 2160));
        assert_eq!(small.cx / 1920.0, big.cx / 3840.0);
        assert!(small.partly_outside((1920, 1080)) == big.partly_outside((3840, 2160)));
        assert!(
            small.partly_outside((1920, 1080)),
            "centre at 90%, 40% wide"
        );
    }
}
