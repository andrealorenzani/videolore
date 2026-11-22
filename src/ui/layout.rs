// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The four resizable sections of the content area (pure geometry, no egui state).
//!
//! The layout is kept as shares ([`Fractions`]), so resizing the window keeps the
//! proportions. Minimum sizes are applied only when drawing: the stored shares never
//! change because the window got small, so it grows back to the same layout.

use eframe::egui::{Rect, pos2};

pub use crate::settings::SavedLayout as Fractions;

/// Space between two sections; the draggable borders live in it.
pub const GAP: f32 = 6.0;

/// Minimum section sizes in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mins {
    pub media: f32,
    pub effects: f32,
    pub preview: f32,
    pub timeline: f32,
    /// Height of each row.
    pub row: f32,
}

/// Media and Effects 160 px wide, Preview and Timeline 240 px, each row 100 px high.
pub const MINS: Mins = Mins {
    media: 160.0,
    effects: 160.0,
    preview: 240.0,
    timeline: 240.0,
    row: 100.0,
};

/// A draggable border between two sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Splitter {
    MediaPreview,
    TimelineEffects,
    /// Between the top row (Media, Preview) and the bottom row (Timeline, Effects).
    Rows,
}

impl Splitter {
    pub const ALL: [Splitter; 3] = [
        Splitter::MediaPreview,
        Splitter::TimelineEffects,
        Splitter::Rows,
    ];

    pub fn tooltip(self) -> &'static str {
        match self {
            Splitter::MediaPreview => "Drag to resize Media and Preview",
            Splitter::TimelineEffects => "Drag to resize Timeline and Effects",
            Splitter::Rows => "Drag to resize the top and bottom rows",
        }
    }

    /// True for the vertical borders (dragged left and right).
    pub fn is_vertical(self) -> bool {
        self != Splitter::Rows
    }
}

/// Where each section and border is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionRects {
    pub media: Rect,
    pub preview: Rect,
    pub timeline: Rect,
    pub effects: Rect,
    pub splitters: [(Splitter, Rect); 3],
}

/// Length of the first of two neighbours sharing `total` pixels (gap included), given
/// the wished share: clamped so both keep their minimum; when both minimums do not fit
/// the space is split in proportion to them. Whole pixels, never negative.
fn first_len(total: f32, share: f32, min_a: f32, min_b: f32) -> f32 {
    let avail = (total - GAP).max(0.0);
    let share = if share.is_finite() {
        share.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let len = if min_a + min_b <= avail {
        (avail * share).clamp(min_a, avail - min_b)
    } else if min_a + min_b > 0.0 {
        avail * min_a / (min_a + min_b)
    } else {
        avail * share
    };
    len.round().clamp(0.0, avail)
}

/// The sections of `area` for the shares `f` (minimums applied).
pub fn sections(area: Rect, f: Fractions, m: Mins) -> SectionRects {
    let top_h = first_len(area.height(), f.top, m.row, m.row);
    let media_w = first_len(area.width(), f.media, m.media, m.preview);
    let tl_w = first_len(area.width(), f.timeline, m.timeline, m.effects);
    let y_top = (area.min.y + top_h).round();
    let y_bottom = (y_top + GAP).min(area.max.y);
    let x_media = (area.min.x + media_w).round();
    let x_tl = (area.min.x + tl_w).round();
    let (x0, x1, y0, y1) = (area.min.x, area.max.x, area.min.y, area.max.y);
    let gap_end = |x: f32| (x + GAP).min(x1);
    SectionRects {
        media: Rect::from_min_max(pos2(x0, y0), pos2(x_media, y_top)),
        preview: Rect::from_min_max(pos2(gap_end(x_media), y0), pos2(x1, y_top)),
        timeline: Rect::from_min_max(pos2(x0, y_bottom), pos2(x_tl, y1)),
        effects: Rect::from_min_max(pos2(gap_end(x_tl), y_bottom), pos2(x1, y1)),
        splitters: [
            (
                Splitter::MediaPreview,
                Rect::from_min_max(pos2(x_media, y0), pos2(gap_end(x_media), y_top)),
            ),
            (
                Splitter::TimelineEffects,
                Rect::from_min_max(pos2(x_tl, y_bottom), pos2(gap_end(x_tl), y1)),
            ),
            (
                Splitter::Rows,
                Rect::from_min_max(pos2(x0, y_top), pos2(x1, y_bottom)),
            ),
        ],
    }
}

/// How a side section (Media or Effects) is shown (REQ-237).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Shown,
    /// Only the title header, in a narrow column.
    Collapsed,
    Hidden,
}

/// Width of a collapsed side section.
pub const COLLAPSED_W: f32 = 120.0;

/// What the user chose for the side sections (REQ-237, REQ-238).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Arrangement {
    pub media: Mode,
    pub effects: Mode,
    /// Media docked right of the Preview.
    pub media_right: bool,
    /// Effects docked left of the Timeline.
    pub effects_left: bool,
}

impl Arrangement {
    /// The arrangement the interface settings describe.
    pub fn of(i: &crate::settings::Interface) -> Arrangement {
        use crate::settings::interface::Section;
        let mode = |s| {
            if i.is_hidden(s) {
                Mode::Hidden
            } else if i.is_collapsed(s) {
                Mode::Collapsed
            } else {
                Mode::Shown
            }
        };
        Arrangement {
            media: mode(Section::Media),
            effects: mode(Section::Effects),
            media_right: i.is_flipped(Section::Media),
            effects_left: i.is_flipped(Section::Effects),
        }
    }
}

fn mirror(r: Rect, x0: f32, x1: f32) -> Rect {
    Rect::from_min_max(
        pos2(x0 + x1 - r.max.x, r.min.y),
        pos2(x0 + x1 - r.min.x, r.max.y),
    )
}

/// One row with a side section and a main one, the side one at the left: (side, main,
/// border).
fn row(x0: f32, x1: f32, y: (f32, f32), side_w: f32, mode: Mode) -> (Rect, Rect, Rect) {
    let rect = |a: f32, b: f32| Rect::from_min_max(pos2(a, y.0), pos2(b, y.1));
    match mode {
        Mode::Hidden => (rect(x0, x0), rect(x0, x1), rect(x0, x0)),
        Mode::Collapsed => {
            let w = COLLAPSED_W.min((x1 - x0 - GAP).max(0.0));
            (
                rect(x0, x0 + w),
                rect(x0 + w + GAP, x1),
                rect(x0 + w, x0 + w),
            )
        }
        Mode::Shown => (
            rect(x0, x0 + side_w),
            rect((x0 + side_w + GAP).min(x1), x1),
            rect(x0 + side_w, (x0 + side_w + GAP).min(x1)),
        ),
    }
}

/// The sections of `area` for the shares `f`, with Media and Effects shown, collapsed,
/// hidden or docked on the other side. A hidden or collapsed section has no border to
/// drag (its border rectangle is empty). The default arrangement equals [`sections`].
pub fn arrange(area: Rect, f: Fractions, m: Mins, a: Arrangement) -> SectionRects {
    let base = sections(area, f, m);
    let (x0, x1) = (area.min.x, area.max.x);
    let top = (base.media.min.y, base.media.max.y);
    let bottom = (base.timeline.min.y, base.timeline.max.y);
    let (media, preview, mp) = row(x0, x1, top, base.media.width(), a.media);
    let (effects, timeline, te) = row(x0, x1, bottom, base.effects.width(), a.effects);
    // the default has Effects at the right: mirrored unless the user docked it left
    let (media, preview, mp) = if a.media_right {
        (
            mirror(media, x0, x1),
            mirror(preview, x0, x1),
            mirror(mp, x0, x1),
        )
    } else {
        (media, preview, mp)
    };
    let (effects, timeline, te) = if a.effects_left {
        (effects, timeline, te)
    } else {
        (
            mirror(effects, x0, x1),
            mirror(timeline, x0, x1),
            mirror(te, x0, x1),
        )
    };
    let mut r = base;
    r.media = media;
    r.preview = preview;
    r.timeline = timeline;
    r.effects = effects;
    r.splitters[0].1 = mp;
    r.splitters[1].1 = te;
    r
}

/// The shares actually shown in `area` (minimums applied).
pub fn effective(f: Fractions, area: Rect, m: Mins) -> Fractions {
    let share = |len: f32, total: f32| {
        let avail = total - GAP;
        if avail > 0.0 { len / avail } else { 0.5 }
    };
    Fractions {
        top: share(first_len(area.height(), f.top, m.row, m.row), area.height()),
        media: share(
            first_len(area.width(), f.media, m.media, m.preview),
            area.width(),
        ),
        timeline: share(
            first_len(area.width(), f.timeline, m.timeline, m.effects),
            area.width(),
        ),
        applied: f.applied,
    }
}

/// The column of the Applied effects is never narrower than this (REQ-317: an icon, a name
/// and a time range fit).
pub const APPLIED_MIN: f32 = 180.0;

/// The least width the picture of the Preview section is given when the column can leave it.
pub const PICTURE_MIN: f32 = 60.0;

/// The width of the Applied effects column in a Preview section `section_w` wide (REQ-317):
/// its share of the width, at least [`APPLIED_MIN`]; it keeps that minimum when the section
/// is narrow (the picture shrinks first, REQ-068) and never takes more than the section.
pub fn applied_width(section_w: f32, share: f32) -> f32 {
    if !section_w.is_finite() || section_w <= 0.0 {
        return 0.0;
    }
    let share = if share.is_finite() { share } else { 0.15 };
    (section_w * share)
        .max(APPLIED_MIN)
        .min((section_w - GAP).max(0.0))
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod applied_tests {
    use super::*;

    // covers: REQ-317
    #[test]
    fn the_column_is_its_share_of_the_section_and_never_under_180_px() {
        // 15% of 1200 is 180: wider sections give the share, narrower the minimum
        assert_eq!(applied_width(1200.0, 0.15), 180.0);
        assert_eq!(applied_width(2000.0, 0.15), 300.0);
        assert_eq!(applied_width(885.0, 0.15), APPLIED_MIN);
        // a section narrower than the minimum plus the picture's minimum: the picture
        // shrinks first, the column keeps its minimum; never more than the section
        assert_eq!(applied_width(300.0, 0.15), APPLIED_MIN);
        assert_eq!(applied_width(100.0, 0.15), 100.0 - GAP);
        assert_eq!(applied_width(0.0, 0.15), 0.0);
        assert_eq!(applied_width(f32::NAN, 0.15), 0.0);
        assert_eq!(applied_width(2000.0, f32::NAN), 300.0);
    }

    // covers: REQ-317
    #[test]
    fn dragging_the_border_left_widens_the_column_inside_its_limits() {
        let f = Fractions::DEFAULT;
        let wider = drag_applied(f, 2000.0, -100.0);
        assert!((wider.applied * 2000.0 - 400.0).abs() < 1.0, "{wider:?}");
        let narrower = drag_applied(wider, 2000.0, 1000.0);
        assert!((narrower.applied * 2000.0 - APPLIED_MIN).abs() < 1.0);
        // the picture keeps its minimum
        let most = drag_applied(f, 2000.0, -5000.0);
        assert!(most.applied * 2000.0 <= 2000.0 - GAP - PICTURE_MIN + 1.0);
        // a section too small to move anything, or a bad delta: unchanged
        assert_eq!(drag_applied(f, 200.0, -50.0), f);
        assert_eq!(drag_applied(f, 2000.0, f32::NAN), f);
        // the other shares are not touched
        assert_eq!(
            (wider.top, wider.media, wider.timeline),
            (f.top, f.media, f.timeline)
        );
    }

    // covers: REQ-317
    #[test]
    fn a_settings_file_without_the_share_has_15_percent_and_a_bad_share_is_refused() {
        let old = r#"{"top":0.5,"media":0.3,"timeline":0.7}"#;
        let l: Fractions = serde_json::from_str(old).unwrap();
        assert_eq!(l.applied, 0.15);
        assert!(l.is_valid());
        for bad in [0.0f32, 0.01, 0.6, f32::NAN] {
            assert!(!Fractions { applied: bad, ..l }.is_valid(), "{bad}");
        }
        let kept: Fractions = serde_json::from_str(
            &serde_json::to_string(&Fractions { applied: 0.22, ..l }).unwrap(),
        )
        .unwrap();
        assert_eq!(kept.applied, 0.22);
    }
}

/// The layout after dragging the border between the picture and the column by `delta` pixels
/// (to the left widens the column) in a section `section_w` wide.
pub fn drag_applied(f: Fractions, section_w: f32, delta: f32) -> Fractions {
    if !delta.is_finite() || section_w <= APPLIED_MIN + PICTURE_MIN + GAP {
        return f;
    }
    let wide = (applied_width(section_w, f.applied) - delta)
        .round()
        .clamp(APPLIED_MIN, section_w - GAP - PICTURE_MIN);
    let mut out = f;
    out.applied = (wide / section_w).clamp(0.05, 0.5);
    out
}

/// Moves one border by `delta` pixels (right or down is positive). Only the two sections
/// next to it change; the border stops at their minimum sizes.
pub fn drag(f: Fractions, s: Splitter, delta: f32, area: Rect, m: Mins) -> Fractions {
    let (total, share, min_a, min_b) = match s {
        Splitter::MediaPreview => (area.width(), f.media, m.media, m.preview),
        Splitter::TimelineEffects => (area.width(), f.timeline, m.timeline, m.effects),
        Splitter::Rows => (area.height(), f.top, m.row, m.row),
    };
    let avail = total - GAP;
    if avail <= 0.0 || min_a + min_b > avail || !delta.is_finite() {
        return f;
    }
    let len = (first_len(total, share, min_a, min_b) + delta)
        .round()
        .clamp(min_a, avail - min_b);
    // a share of exactly 0 or 1 is not valid: keep a hair inside
    let new = (len / avail).clamp(1e-4, 1.0 - 1e-4);
    let mut out = f;
    match s {
        Splitter::MediaPreview => out.media = new,
        Splitter::TimelineEffects => out.timeline = new,
        Splitter::Rows => out.top = new,
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    fn area(w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), vec2(w, h))
    }

    fn covered(r: &SectionRects) -> f32 {
        r.media.area()
            + r.preview.area()
            + r.timeline.area()
            + r.effects.area()
            + r.splitters[0].1.area()
            + r.splitters[1].1.area()
            + r.splitters[2].1.area()
    }

    fn no_overlap(r: &SectionRects) {
        let mut all = vec![r.media, r.preview, r.timeline, r.effects];
        all.extend(r.splitters.iter().map(|s| s.1));
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(
                    a.intersect(*b).area() < 1e-3,
                    "{a:?} overlaps {b:?} in {r:?}"
                );
            }
        }
    }

    // covers: REQ-024
    #[test]
    fn default_proportions_hold_at_any_size() {
        // the content area of 800x500, 1280x800 and 2560x1440 windows (and the sizes
        // themselves)
        for (w, h) in [
            (800.0, 500.0),
            (800.0, 420.0),
            (1280.0, 800.0),
            (1280.0, 720.0),
            (2560.0, 1440.0),
            (2560.0, 1360.0),
        ] {
            let a = area(w, h);
            let r = sections(a, Fractions::DEFAULT, MINS);
            let (aw, ah) = (w - GAP, h - GAP);
            let near = |x: f32, y: f32| (x - y).abs() <= 1.0;
            assert!(near(r.media.height(), ah * 2.0 / 3.0), "{w}x{h}");
            assert!(near(r.timeline.height(), ah / 3.0), "{w}x{h}");
            assert!(near(r.media.width(), aw * 0.3), "{w}x{h}");
            assert!(near(r.preview.width(), aw * 0.7), "{w}x{h}");
            assert!(near(r.timeline.width(), aw * 0.7), "{w}x{h}");
            assert!(near(r.effects.width(), aw * 0.3), "{w}x{h}");
            no_overlap(&r);
            // sections and borders cover the whole content area
            assert!((covered(&r) - w * h).abs() < 1e-2, "{w}x{h}");
            assert_eq!(r.media.min, a.min);
            assert_eq!(r.effects.max, a.max);
            // borders on whole pixels
            assert_eq!(r.media.max.x, r.media.max.x.round());
            assert_eq!(r.media.max.y, r.media.max.y.round());
            assert_eq!(effective(Fractions::DEFAULT, a, MINS).media, {
                r.media.width() / aw
            });
        }
    }

    // covers: REQ-066
    #[test]
    fn dragging_a_border_resizes_only_its_two_neighbours() {
        let a = area(1280.0, 720.0);
        let f = Fractions::DEFAULT;
        let before = sections(a, f, MINS);
        for s in Splitter::ALL {
            for delta in [100.0, -100.0] {
                let g = drag(f, s, delta, a, MINS);
                let after = sections(a, g, MINS);
                match s {
                    Splitter::MediaPreview => {
                        assert_eq!(after.media.width(), before.media.width() + delta);
                        assert_eq!(after.preview.width(), before.preview.width() - delta);
                        assert_eq!(after.timeline, before.timeline);
                        assert_eq!(after.effects, before.effects);
                        assert_eq!(after.media.height(), before.media.height());
                    }
                    Splitter::TimelineEffects => {
                        assert_eq!(after.timeline.width(), before.timeline.width() + delta);
                        assert_eq!(after.effects.width(), before.effects.width() - delta);
                        assert_eq!(after.media, before.media);
                        assert_eq!(after.preview, before.preview);
                    }
                    Splitter::Rows => {
                        assert_eq!(after.media.height(), before.media.height() + delta);
                        assert_eq!(after.timeline.height(), before.timeline.height() - delta);
                        assert_eq!(after.media.width(), before.media.width());
                        assert_eq!(after.effects.width(), before.effects.width());
                    }
                }
                no_overlap(&after);
                assert!((covered(&after) - a.area()).abs() < 1e-2);
                assert!(g.is_valid());
            }
        }
    }

    // covers: REQ-066
    #[test]
    fn borders_stop_at_the_minimum_sizes() {
        let a = area(1280.0, 720.0);
        let f = Fractions::DEFAULT;
        let r = sections(a, drag(f, Splitter::MediaPreview, -5000.0, a, MINS), MINS);
        assert_eq!(r.media.width(), MINS.media);
        let r = sections(a, drag(f, Splitter::MediaPreview, 5000.0, a, MINS), MINS);
        assert_eq!(r.preview.width(), MINS.preview);
        let r = sections(
            a,
            drag(f, Splitter::TimelineEffects, -5000.0, a, MINS),
            MINS,
        );
        assert_eq!(r.timeline.width(), MINS.timeline);
        let r = sections(a, drag(f, Splitter::TimelineEffects, 5000.0, a, MINS), MINS);
        assert_eq!(r.effects.width(), MINS.effects);
        let r = sections(a, drag(f, Splitter::Rows, -5000.0, a, MINS), MINS);
        assert_eq!(r.media.height(), MINS.row);
        let r = sections(a, drag(f, Splitter::Rows, 5000.0, a, MINS), MINS);
        assert_eq!(r.timeline.height(), MINS.row);
        // a nonsense drag changes nothing
        assert_eq!(drag(f, Splitter::Rows, f32::NAN, a, MINS), f);
    }

    // covers: REQ-066, REQ-067
    #[test]
    fn window_resizes_keep_the_proportions_and_minimums_win() {
        let big = area(2560.0, 1360.0);
        let f = drag(
            Fractions::DEFAULT,
            Splitter::MediaPreview,
            -300.0,
            big,
            MINS,
        );
        let f = drag(f, Splitter::Rows, 200.0, big, MINS);
        let share = sections(big, f, MINS).media.width() / (2560.0 - GAP);
        for (w, h) in [(1280.0, 800.0), (1920.0, 1080.0)] {
            let r = sections(area(w, h), f, MINS);
            assert!((r.media.width() - (w - GAP) * share).abs() <= 1.0);
            assert!((r.media.height() - (h - GAP) * f.top).abs() <= 1.0);
        }
        // at the smallest window the minimum wins, and growing back restores the layout
        let small = area(800.0, 420.0);
        let r = sections(small, f, MINS);
        assert_eq!(r.media.width(), MINS.media);
        assert!(effective(f, small, MINS).media > f.media);
        let r = sections(big, f, MINS);
        assert!((r.media.width() - (2560.0 - GAP) * share).abs() <= 1.0);
        // a saved layout that cannot fit: minimums win
        let squeezed = Fractions {
            top: 0.99,
            media: 0.99,
            timeline: 0.01,
            applied: 0.15,
        };
        let r = sections(area(800.0, 420.0), squeezed, MINS);
        assert_eq!(r.preview.width(), MINS.preview);
        assert_eq!(r.timeline.width(), MINS.timeline);
        assert_eq!(r.timeline.height(), MINS.row);
    }

    // covers: REQ-066
    #[test]
    fn impossible_sizes_never_panic_or_go_negative() {
        for (w, h) in [(0.0, 0.0), (3.0, 2.0), (300.0, 150.0), (399.0, 205.0)] {
            for f in [
                Fractions::DEFAULT,
                Fractions {
                    top: f32::NAN,
                    media: f32::INFINITY,
                    timeline: -1.0,
                    applied: f32::NAN,
                },
            ] {
                let a = area(w, h);
                let r = sections(a, f, MINS);
                for x in [r.media, r.preview, r.timeline, r.effects] {
                    assert!(x.width() >= 0.0 && x.height() >= 0.0, "{w}x{h} {x:?}");
                    assert!(a.expand(0.01).contains_rect(x), "{w}x{h} {x:?}");
                }
                for s in Splitter::ALL {
                    let g = drag(f, s, 50.0, a, MINS);
                    let _ = sections(a, g, MINS);
                }
            }
        }
        // minimums that do not fit are shared in proportion to them
        let r = sections(area(306.0, 150.0), Fractions::DEFAULT, MINS);
        assert_eq!(r.media.width(), 120.0);
        assert_eq!(r.preview.width(), 180.0);
        let zero = Mins {
            media: 0.0,
            effects: 0.0,
            preview: 0.0,
            timeline: 0.0,
            row: 0.0,
        };
        let r = sections(area(106.0, 106.0), Fractions::DEFAULT, zero);
        assert_eq!(r.media.width(), 30.0);
        assert!(Splitter::MediaPreview.is_vertical() && !Splitter::Rows.is_vertical());
        assert!(Splitter::ALL.iter().all(|s| s.tooltip().contains("Drag")));
    }

    // covers: REQ-237, REQ-238
    #[test]
    fn side_sections_hide_collapse_and_dock() {
        let a = area(1600.0, 900.0);
        let base = sections(a, Fractions::DEFAULT, MINS);
        assert_eq!(
            arrange(a, Fractions::DEFAULT, MINS, Arrangement::default()),
            base
        );
        // hide Media: the Preview takes the row
        let hid = arrange(
            a,
            Fractions::DEFAULT,
            MINS,
            Arrangement {
                media: Mode::Hidden,
                ..Default::default()
            },
        );
        assert_eq!(hid.preview.width(), a.width());
        assert_eq!(hid.media.width(), 0.0);
        // collapse Effects: a narrow column, the Timeline takes the rest
        let col = arrange(
            a,
            Fractions::DEFAULT,
            MINS,
            Arrangement {
                effects: Mode::Collapsed,
                ..Default::default()
            },
        );
        assert_eq!(col.effects.width(), COLLAPSED_W);
        assert!(col.timeline.width() > base.timeline.width());
        assert_eq!(col.effects.max.x, a.max.x);
        // Media docked right: the same width, at the right edge, Preview left of it
        let flip = arrange(
            a,
            Fractions::DEFAULT,
            MINS,
            Arrangement {
                media_right: true,
                effects_left: true,
                ..Default::default()
            },
        );
        assert_eq!(flip.media.max.x, a.max.x);
        assert_eq!(flip.media.width(), base.media.width());
        assert_eq!(flip.preview.min.x, a.min.x);
        assert_eq!(flip.effects.min.x, a.min.x);
        assert_eq!(flip.timeline.max.x, a.max.x);
        // the minimum sizes still hold in a small window
        let small = arrange(
            area(800.0, 500.0),
            Fractions::DEFAULT,
            MINS,
            Arrangement {
                media_right: true,
                ..Default::default()
            },
        );
        assert!(small.media.width() >= MINS.media - 1.0 && small.preview.width() >= 1.0);
    }
}
