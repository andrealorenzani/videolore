// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Where a text sits and how big it is drawn: the box, the best fit of the text in it, the
//! shaped lines (REQ-116, REQ-159, REQ-160).

use super::*;

/// The box of a text on a flat `width` x `height` picture, in pixels: its width (REQ-116)
/// and height (REQ-159).
pub fn box_px(item: &TextItem, width: u32, height: u32) -> (f64, f64) {
    (
        f64::from(width) * f64::from(item.width) / 1000.0,
        f64::from(height) * f64::from(item.height) / 1000.0,
    )
}

/// The rows `text` takes at font height `px` when it wraps at `wrap` pixels: characters
/// are taken as 0.6 of the font height wide.
fn legacy_rows_at(text: &str, px: f64, wrap: f64) -> f64 {
    text.split('\n')
        .map(|l| {
            (l.chars().count() as f64 * px * 0.6 / wrap.max(1.0))
                .ceil()
                .max(1.0)
        })
        .sum()
}

/// The 2.0.2 **estimate** of the best fit: the largest font height in pixels at which
/// `text`, wrapped at `box_w` by 0.6-em characters, fits inside `box_h` (rows of 1.2 times
/// the font height). Kept only for the sizes of old projects (ADR-088, REQ-167).
pub fn legacy_fit_in(text: &str, box_w: f64, box_h: f64, min_px: f64) -> f64 {
    let fits = |px: f64| legacy_rows_at(text, px, box_w) * px * 1.2 <= box_h;
    let hi = box_h / 1.2;
    if hi <= min_px || fits(hi) {
        return hi.max(min_px);
    }
    if !fits(min_px) {
        return min_px;
    }
    let (mut lo, mut hi) = (min_px, hi);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The size the 2.0.2 estimate gave a flat text on a `width` x `height` picture.
pub fn legacy_fit_px(item: &TextItem, width: u32, height: u32) -> u32 {
    let (bw, bh) = box_px(item, width, height);
    let min = f64::from(height) * f64::from(MIN_FIT) / 1000.0;
    (legacy_fit_in(&item.text, bw, bh, min).floor() as u32).max(6)
}

/// A text measured with its font: the size and the lines it is drawn with (REQ-156).
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    /// Font size (the height of a line) in pixels.
    pub px: f64,
    /// The lines as they are drawn (already wrapped: the renderer does not wrap).
    pub lines: Vec<String>,
    /// Width of the widest line, in pixels.
    pub width: f64,
    /// False when the text runs past its box even at the smallest size (REQ-159).
    pub fits: bool,
}

impl TextLayout {
    /// Height of the lines, in pixels.
    pub fn height(&self) -> f64 {
        self.lines.len() as f64 * self.px
    }
}

/// Texts shaped so far, by font file and text (shaping is the costly part of a layout).
type ShapeCache = std::collections::HashMap<(String, String), Option<std::sync::Arc<Shaped>>>;
const SHAPE_CACHE_MAX: usize = 512;

fn shape_cache() -> &'static std::sync::Mutex<ShapeCache> {
    static C: std::sync::OnceLock<std::sync::Mutex<ShapeCache>> = std::sync::OnceLock::new();
    C.get_or_init(Default::default)
}

/// `text` shaped with the face that draws the font family `font`: the installed face, or
/// the bundled one when the family is not installed or cannot be read (REQ-163, REQ-164).
pub(super) fn shaped_for(text: &str, font: &str) -> Option<std::sync::Arc<Shaped>> {
    let installed = fonts::resolve(font);
    let try_face = |face: &fonts::Face, key: String| {
        let k = (key, text.to_string());
        if let Ok(c) = shape_cache().lock()
            && let Some(hit) = c.get(&k)
        {
            return hit.clone();
        }
        let v = Shaped::new(&face.bytes, face.index, text).map(std::sync::Arc::new);
        if let Ok(mut c) = shape_cache().lock() {
            if c.len() >= SHAPE_CACHE_MAX {
                c.clear();
            }
            c.insert(k, v.clone());
        }
        v
    };
    if let Some(f) = &installed {
        let key = format!(
            "{}#{}",
            f.file
                .as_deref()
                .map_or(String::new(), |p| p.display().to_string()),
            f.index
        );
        if let Some(s) = try_face(f, key) {
            return Some(s);
        }
    }
    try_face(&fonts::bundled(), "bundled".into())
}

/// The best fit of `text` in font `font` for a box of `box_w` x `box_h` pixels, the size
/// between `min_px` and `max_px` (REQ-156, REQ-157). Pure in its arguments (the font's own
/// tables are the only other input): the preview and the export call it with the same
/// geometry, so they agree.
pub fn layout_in(
    text: &str,
    font: &str,
    box_w: f64,
    box_h: f64,
    min_px: f64,
    max_px: f64,
) -> TextLayout {
    match shaped_for(text, font) {
        Some(s) => {
            let f = s.fit(box_w, box_h, min_px, max_px);
            TextLayout {
                px: f.px,
                width: f.width,
                lines: f.lines,
                fits: f.fits,
            }
        }
        // no readable font at all: one row per line at the smallest size
        None => TextLayout {
            px: min_px,
            width: box_w,
            lines: text.split('\n').map(str::to_string).collect(),
            fits: false,
        },
    }
}

/// The lines of `text` wrapped at `box_w` pixels when drawn at the fixed size `px` (the
/// subtitle lines fixed to the sphere have their own size).
pub fn layout_at(text: &str, font: &str, px: f64, box_w: f64) -> TextLayout {
    match shaped_for(text, font) {
        Some(s) => {
            let f = s.at_size(box_w, px);
            TextLayout {
                px,
                width: f.width,
                lines: f.lines,
                fits: f.fits,
            }
        }
        None => TextLayout {
            px,
            width: box_w,
            lines: text.split('\n').map(str::to_string).collect(),
            fits: false,
        },
    }
}

/// The measured layout of a flat text on a `width` x `height` picture: the best fit of its
/// box, between 2% and 10% of the picture height.
pub fn layout(item: &TextItem, width: u32, height: u32) -> TextLayout {
    let (bw, bh) = box_px(item, width, height);
    let h = f64::from(height);
    let min = (h * f64::from(MIN_FIT) / 1000.0).max(6.0);
    let max = h * f64::from(MAX_FIT) / 1000.0;
    layout_in(&item.text, &item.font, bw, bh, min, max.max(min))
}

/// Font size in pixels of a flat text on a `width` x `height` picture: its best fit.
pub fn fit_px(item: &TextItem, width: u32, height: u32) -> f64 {
    layout(item, width, height).px
}

/// The box width that makes a text made in 1.3.0 (with the size `old_size`, thousandths
/// of the default) look as before (ADR-088): the narrowest width whose best fit is at
/// least the old font height, else the widest. Measured on a 1920x1080 picture.
pub fn legacy_width(text: &str, old_size: u16) -> u16 {
    let want = font_px(1080, old_size);
    let mut item = TextItem::new(TextId(0), text.to_string(), Micros::ZERO);
    let (mut lo, mut hi) = (WIDTH_MIN, WIDTH_MAX);
    item.width = hi;
    if legacy_fit_px(&item, 1920, 1080) < want {
        return WIDTH_MAX;
    }
    while lo < hi {
        let mid = (lo + hi) / 2;
        item.width = mid;
        if legacy_fit_px(&item, 1920, 1080) >= want {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    hi
}

/// The box height (hundredths of a degree) of a text fixed to the sphere made in 1.3.0
/// with the angles `size_cd` (height of a line at 100%), `wrap_cd` and the size `old_size`:
/// the rows it took (ADR-088), clamped to the range of an anchor.
pub fn legacy_anchor_height(text: &str, size_cd: u32, wrap_cd: u32, old_size: u16) -> u32 {
    let (w, h) = (3840.0, 1920.0);
    let px =
        f64::from(size_cd) / 100.0 * (h / 180.0) * f64::from(old_size) / f64::from(SIZE_NORMAL);
    let wrap = (f64::from(wrap_cd) / 100.0 * w / 360.0).clamp(8.0, w);
    let rows = legacy_rows_at(text, px.max(4.0), wrap);
    let cd = rows * f64::from(size_cd) * f64::from(old_size) / f64::from(SIZE_NORMAL) * 1.2;
    (cd.round() as u32).clamp(1, 18_000)
}

/// Size (pixels) of the drawn text of an item with what readability adds around it: used
/// where only the size of the box is needed (the slide distance of the export, the preview
/// boxes).
pub fn estimate_box(item: &TextItem, width: u32, height: u32) -> (f64, f64) {
    let l = layout(item, width, height);
    let pad = item.readability.border(l.px);
    (l.width + 2.0 * pad, l.height() + 2.0 * pad)
}
