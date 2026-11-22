// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Overlap media (2.3.0, REQ-325, REQ-327 to REQ-331): a picture, a GIF or a video with
//! transparency shown over one video clip for a time. Pure: the model of one overlap and the
//! rules of the edits of its clip (cut to a kept part, speed rescale, the ones an edit loses).
//! Imports only the time type, the picture position and the picture module's numbers.
//!
//! An overlap belongs to its clip (it is stored in `Timeline.clip_overlaps`, next to the other
//! per-clip data): its start is an offset from the clip's start on the timeline, so a ripple
//! moves it with the clip and a trim, a split or a delete is a rule of the clip's edit.

use crate::base::pos::Pos;
use crate::time::Micros;
use serde::{Deserialize, Serialize};

/// The shortest overlap, 0.1 s (assumption of the plan for lane drags).
pub const MIN_LEN: Micros = Micros(100_000);
/// The length of a new overlap: 3 s, cut to its clip (REQ-325).
pub const DEFAULT_LEN: Micros = Micros(3_000_000);
/// Longest file path stored for an overlap.
pub const MAX_PATH: usize = crate::picture::MAX_PATH;
/// Most overlaps on one clip that a project file may hold.
pub const MAX_PER_CLIP: usize = 256;
/// The side above which an overlay file is previewed at half size (ADR-243): 1920 x 1080.
pub const PREVIEW_LIMIT: (u32, u32) = (1920, 1080);

/// What an overlap shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A PNG: a still picture shown for the whole overlap.
    Still,
    /// A GIF or a video: plays once from the overlap's start, then is transparent (REQ-328).
    Motion,
}

/// One overlap of a clip.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Overlap {
    /// Unique in the project (`Timeline.next_overlap_id`).
    pub id: u64,
    /// The file (UTF-8, as the picture items; the file is not copied).
    pub path: String,
    pub kind: Kind,
    /// Offset of the start from the clip's start, in timeline time (at least 0).
    pub from: Micros,
    /// Length (above 0); `from + len` is at most the clip's length.
    pub len: Micros,
    /// Centre of the box (the same convention as a text item).
    pub place: Pos,
    /// The box, in thousandths of the frame (as a text item).
    pub width: u16,
    pub height: u16,
    /// The shape was changed with Ctrl (Command): a video is cropped to the box and a GIF or
    /// a picture is stretched to it (REQ-327).
    #[serde(default, skip_serializing_if = "is_false")]
    pub reshaped: bool,
    /// The file's picture size in pixels (read when it was added).
    pub src_w: u32,
    pub src_h: u32,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Overlap {
    /// The time after the last instant, from the clip's start.
    pub fn end(&self) -> Micros {
        self.from + self.len
    }

    /// The file name, as shown on the timeline.
    pub fn name(&self) -> String {
        std::path::Path::new(&self.path)
            .file_name()
            .map_or_else(|| self.path.clone(), |n| n.to_string_lossy().into())
    }

    /// True for a GIF (a reshaped GIF is stretched, a reshaped video is cropped).
    pub fn is_gif(&self) -> bool {
        std::path::Path::new(&self.path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gif"))
    }

    /// Checks a value read from a project file for a clip `clip_len` long (REQ-057).
    pub fn is_valid(&self, clip_len: Micros) -> bool {
        self.id > 0
            && !self.path.is_empty()
            && self.path.len() <= MAX_PATH
            && !self.path.chars().any(char::is_control)
            && self.len > Micros::ZERO
            && self.from >= Micros::ZERO
            && self.from < clip_len
            && self.end() <= clip_len
            && self.place.is_valid()
            && (1..=1000).contains(&self.width)
            && (1..=1000).contains(&self.height)
            && (1..=crate::picture::MAX_SIDE).contains(&self.src_w)
            && (1..=crate::picture::MAX_SIDE).contains(&self.src_h)
            && u64::from(self.src_w) * u64::from(self.src_h) <= crate::picture::MAX_PICTURE_PIXELS
    }

    /// The overlap moved inside a clip `clip_len` long: it starts at 0 or later and ends at
    /// the clip's end or before, and is at least [`MIN_LEN`] long when the clip allows it.
    pub fn clamped(mut self, clip_len: Micros) -> Overlap {
        let min = MIN_LEN.min(clip_len);
        self.len = self.len.max(min).min(clip_len);
        self.from = self.from.clamp(Micros::ZERO, clip_len - self.len);
        self
    }

    /// The part of the overlap that lies in `[keep_from, keep_to)` (offsets of the clip), as an
    /// overlap whose offsets count from `keep_from`; `None` when nothing of it is in there.
    pub fn kept_part(&self, keep_from: Micros, keep_to: Micros) -> Option<Overlap> {
        let a = self.from.max(keep_from);
        let b = self.end().min(keep_to);
        (b > a).then(|| Overlap {
            from: a - keep_from,
            len: b - a,
            ..self.clone()
        })
    }

    /// The times rescaled for a clip whose speed changed from `old` to `new` (thousandths):
    /// the overlap keeps its part of the clip (REQ-329).
    pub fn rescaled(&self, old: u16, new: u16) -> Overlap {
        let k =
            |t: Micros| Micros((i128::from(t.0) * i128::from(old) / i128::from(new.max(1))) as i64);
        Overlap {
            from: k(self.from),
            len: k(self.len).max(Micros(1)),
            ..self.clone()
        }
    }

    /// Where the picture is drawn on a `frame`-sized picture, `(x, y, w, h)` in pixels. The
    /// media keeps its own ratio inside the box, centred (REQ-327); a reshaped overlap fills
    /// the whole box.
    pub fn rect(&self, frame: (u32, u32)) -> (i32, i32, u32, u32) {
        let pic = crate::picture::Picture {
            path: String::new(),
            width: self.src_w.max(1),
            height: self.src_h.max(1),
        };
        if !self.reshaped {
            return pic.rect(self.place, self.width, self.height, frame);
        }
        let (fw, fh) = (f64::from(frame.0), f64::from(frame.1));
        let even = |v: f64| (((v.round() as i64).max(2) as u32) & !1).max(2);
        let (w, h) = (
            even(fw * f64::from(self.width) / 1000.0),
            even(fh * f64::from(self.height) / 1000.0),
        );
        let (fx, fy) = self.place.fraction();
        (
            (fx * fw - f64::from(w) / 2.0).round() as i32,
            (fy * fh - f64::from(h) / 2.0).round() as i32,
            w,
            h,
        )
    }

    /// True when the file's picture is larger than 1920 x 1080 (previewed at half size).
    pub fn is_large(&self) -> bool {
        self.src_w > PREVIEW_LIMIT.0 || self.src_h > PREVIEW_LIMIT.1
    }
}

/// Reads the overlaps without refusing the file for a damaged one: it becomes an overlap with
/// id 0, which the project check drops and counts (REQ-057).
pub fn lenient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Overlap>, D::Error> {
    let raw: Vec<serde_json::Value> = Vec::deserialize(d)?;
    Ok(raw
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap_or_else(|_| Overlap::damaged()))
        .collect())
}

impl Overlap {
    pub(crate) fn damaged() -> Overlap {
        Overlap {
            id: 0,
            path: String::new(),
            kind: Kind::Still,
            from: Micros::ZERO,
            len: Micros::ZERO,
            place: Pos { x: 0, y: 0 },
            width: 0,
            height: 0,
            reshaped: false,
            src_w: 0,
            src_h: 0,
        }
    }
}

/// The ids in `before` that are not in `after`: the overlaps an edit removed (REQ-329).
pub fn lost(before: &[u64], after: &[u64]) -> Vec<u64> {
    before
        .iter()
        .copied()
        .filter(|id| !after.contains(id))
        .collect()
}

/// The start of the box when its edge is dragged on the lane: `from` and `len` moved by the
/// two ends, clamped to the clip (REQ-325).
pub fn drag_ends(o: &Overlap, from: Micros, end: Micros, clip_len: Micros) -> Overlap {
    let mut n = o.clone();
    let from = from.clamp(Micros::ZERO, clip_len);
    let end = end.clamp(Micros::ZERO, clip_len);
    n.from = from.min(end);
    n.len = (end - from).max(Micros::ZERO).max(MIN_LEN.min(clip_len));
    n.clamped(clip_len)
}

/// The sub-row of each bar of the shared lane: bars that overlap in time go to different
/// sub-rows, greedy by start time (ADR-242). `bars` are `(start, end)` of the timeline; the
/// result has one row index per bar, in the order of `bars`.
pub fn sub_rows(bars: &[(Micros, Micros)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..bars.len()).collect();
    order.sort_by_key(|i| (bars[*i].0, bars[*i].1));
    let mut rows_end: Vec<Micros> = Vec::new();
    let mut out = vec![0; bars.len()];
    for i in order {
        let (a, b) = bars[i];
        let row = rows_end.iter().position(|e| *e <= a);
        let row = match row {
            Some(r) => {
                rows_end[r] = b;
                r
            }
            None => {
                rows_end.push(b);
                rows_end.len() - 1
            }
        };
        out[i] = row;
    }
    out
}

/// Sub-rows of the lane that are visible at once (ADR-242).
pub const VISIBLE_ROWS: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    fn ov(id: u64, from: i64, len: i64) -> Overlap {
        Overlap {
            id,
            path: "/tmp/logo.png".into(),
            kind: Kind::Still,
            from: Micros(from * 1_000_000),
            len: Micros(len * 1_000_000),
            place: Pos { x: 5000, y: 5000 },
            width: 300,
            height: 300,
            reshaped: false,
            src_w: 400,
            src_h: 200,
        }
    }

    fn s(v: i64) -> Micros {
        Micros(v * 1_000_000)
    }

    // covers: REQ-325
    #[test]
    fn an_overlap_is_valid_only_inside_its_clip_and_with_sane_numbers() {
        let o = ov(1, 2, 3);
        assert!(o.is_valid(s(8)));
        assert!(!o.is_valid(s(4)), "ends after the clip");
        assert!(!ov(0, 2, 3).is_valid(s(8)), "id 0 is a damaged entry");
        assert!(!ov(1, 2, 0).is_valid(s(8)), "no length");
        let mut bad = o.clone();
        bad.width = 1001;
        assert!(!bad.is_valid(s(8)));
        let mut big = o.clone();
        big.src_w = 4001;
        big.src_h = 4000;
        assert!(!big.is_valid(s(8)), "above 16 megapixels");
        big.src_h = 3999;
        assert!(big.is_valid(s(8)), "just under 16 megapixels");
        let mut ctl = o;
        ctl.path = "a\u{7}b".into();
        assert!(!ctl.is_valid(s(8)));
    }

    // covers: REQ-325
    #[test]
    fn dragging_the_ends_keeps_the_overlap_inside_its_clip() {
        let o = ov(1, 2, 3);
        // the end to 5 s
        let n = drag_ends(&o, s(2), s(5), s(8));
        assert_eq!((n.from, n.len), (s(2), s(3)));
        // the start before the clip's start stops at the start
        let n = drag_ends(&o, Micros(-500_000), s(5), s(8));
        assert_eq!((n.from, n.end()), (Micros::ZERO, s(5)));
        // the end past the clip stops at the clip's end
        let n = drag_ends(&o, s(2), s(20), s(8));
        assert_eq!(n.end(), s(8));
        // the ends crossing leave the shortest overlap
        let n = drag_ends(&o, s(6), s(3), s(8));
        assert_eq!(n.len, MIN_LEN);
        assert!(n.end() <= s(8));
        // a clip shorter than the minimum
        let n = o.clamped(Micros(50_000));
        assert_eq!((n.from, n.len), (Micros::ZERO, Micros(50_000)));
    }

    // covers: REQ-329
    #[test]
    fn an_overlap_is_cut_to_the_part_of_the_clip_that_is_kept() {
        // 2 s to 6 s on an 8 s clip, split at 4 s
        let o = ov(1, 2, 4);
        let left = o.kept_part(Micros::ZERO, s(4)).unwrap();
        assert_eq!((left.from, left.len), (s(2), s(2)));
        let right = o.kept_part(s(4), s(8)).unwrap();
        assert_eq!((right.from, right.len), (Micros::ZERO, s(2)));
        // a trim of the first 3 s: the start moves with it
        let trimmed = o.kept_part(s(3), s(8)).unwrap();
        assert_eq!((trimmed.from, trimmed.len), (Micros::ZERO, s(3)));
        // wholly outside: nothing is left
        assert!(o.kept_part(s(6), s(8)).is_none());
        assert!(o.kept_part(Micros::ZERO, s(2)).is_none());
        assert_eq!(lost(&[1, 2, 3], &[2]), vec![1, 3]);
        assert!(lost(&[1], &[1, 9]).is_empty());
    }

    // covers: REQ-329
    #[test]
    fn a_change_of_speed_rescales_the_times_so_the_overlap_keeps_its_part() {
        let o = ov(1, 2, 4);
        // twice as fast: the clip is half as long, so are the offsets
        let fast = o.rescaled(1000, 2000);
        assert_eq!((fast.from, fast.len), (s(1), s(2)));
        let slow = o.rescaled(1000, 500);
        assert_eq!((slow.from, slow.len), (s(4), s(8)));
        assert_eq!(o.rescaled(1000, 1000), o);
    }

    // covers: REQ-327
    #[test]
    fn the_picture_keeps_its_ratio_in_the_box_unless_the_shape_was_changed() {
        let frame = (1920, 1080);
        let o = ov(1, 0, 1);
        // a 2:1 picture in a 576 x 324 box: 576 wide, 288 high, centred
        let (x, y, w, h) = o.rect(frame);
        assert_eq!((w, h), (576, 288));
        assert_eq!((x + w as i32 / 2, y + h as i32 / 2), (960, 540));
        // reshaped: the whole box
        let mut r = o;
        r.reshaped = true;
        let (x, y, w, h) = r.rect(frame);
        assert_eq!((w, h), (576, 324));
        assert_eq!((x + w as i32 / 2, y + h as i32 / 2), (960, 540));
        assert!(!r.is_gif());
        r.path = "/tmp/a.GIF".into();
        assert!(r.is_gif());
        assert_eq!(r.name(), "a.GIF");
    }

    // covers: REQ-331
    #[test]
    fn a_file_above_1920_by_1080_is_large() {
        let mut o = ov(1, 0, 1);
        assert!(!o.is_large());
        o.src_w = 1920;
        o.src_h = 1080;
        assert!(!o.is_large(), "exactly 1920 x 1080 is drawn the same");
        o.src_h = 1081;
        assert!(o.is_large());
        o.src_h = 100;
        o.src_w = 1921;
        assert!(o.is_large());
    }

    // covers: REQ-325
    #[test]
    fn overlapping_bars_go_to_sub_rows_by_start_time() {
        let bars = [
            (s(0), s(4)),
            (s(2), s(6)),
            (s(4), s(8)),
            (s(3), s(5)),
            (s(9), s(10)),
        ];
        let rows = sub_rows(&bars);
        // bar 0 and bar 1 overlap, bar 2 starts when bar 0 ends: it reuses its row
        assert_eq!(rows, vec![0, 1, 0, 2, 0]);
        assert!(sub_rows(&[]).is_empty());
        // three overlapping ones need three sub-rows, a fourth is reached by the scroll
        let four = [(s(0), s(9)); 4];
        assert_eq!(sub_rows(&four).iter().max(), Some(&3));
        const { assert!(VISIBLE_ROWS < 4) };
    }

    // covers: REQ-057
    #[test]
    fn a_damaged_entry_is_read_as_an_overlap_with_id_zero() {
        let json = r#"[{"id":1,"path":"/a.png","kind":"still","from":0,"len":1000000,
            "place":{"x":1,"y":2},"width":10,"height":10,"src_w":4,"src_h":4},
            {"id":2,"kind":"sparkle"}]"#;
        let items = lenient(&mut serde_json::Deserializer::from_str(json)).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, Kind::Still);
        assert_eq!(items[1].id, 0);
        assert!(!items[1].is_valid(s(8)));
        let back = serde_json::to_string(&items[0]).unwrap();
        assert!(!back.contains("reshaped"));
    }
}
