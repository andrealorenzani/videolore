// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Measuring text with the glyphs of a real font (REQ-156, REQ-157, REQ-159, REQ-163): the
//! widths of the words come from the font's own tables (shaped with kerning and ligatures,
//! as the renderer does), the text is wrapped greedily at the box width, and the **best fit**
//! is the largest size at which the wrapped lines fit the box height.
//!
//! Everything here is pure: bytes in, numbers out; a font that cannot be read gives `None`
//! (font files are untrusted input) and the caller uses the bundled font instead.
//!
//! Sizes follow the renderer's rule (measured against libass, tests `v21` of the ffmpeg
//! suite): the ASS font size is the height of one line, the Windows ascender plus descender
//! of the font, so a glyph is `size / (ascender + descender)` em tall and a line is exactly
//! `size` high.

use harfrust::{ShaperData, UnicodeBuffer};
use skrifa::FontRef;
use skrifa::raw::TableProvider;

/// Largest text size, in thousandths of the picture height (REQ-157: 10%).
pub const MAX_SIZE: u32 = 100;
/// Smallest text size, in thousandths of the picture height (REQ-116: 2%).
pub const MIN_SIZE: u32 = 20;
/// The size is searched in steps of this many pixels.
const STEP: f64 = 0.1;
/// Most characters shaped for one text (the model limits are far below).
const MAX_CHARS: usize = 400_000;

/// A word: its width and the width up to each of its characters, in line heights.
#[derive(Debug, Clone, PartialEq)]
struct Word {
    text: String,
    width: f32,
    /// `cuts[i]` = width of the first `i + 1` characters.
    cuts: Vec<f32>,
}

/// A text shaped with one font, ready to be wrapped at any size.
#[derive(Debug, Clone, PartialEq)]
pub struct Shaped {
    /// Width of a space, in line heights.
    space: f32,
    paragraphs: Vec<Vec<Word>>,
}

/// The wrapped text at one size.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// Font size (the height of a line) in pixels.
    pub px: f64,
    /// The lines, as they are drawn.
    pub lines: Vec<String>,
    /// Width of the widest line, in pixels.
    pub width: f64,
    /// False when the text runs past the box even at the smallest size (REQ-159).
    pub fits: bool,
}

/// One word placed in its line (pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WordBox {
    pub line: usize,
    pub left: f64,
    pub width: f64,
}

/// The words of a text placed in its wrapped lines (see [`Shaped::place_words`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub boxes: Vec<WordBox>,
    pub line_widths: Vec<f64>,
}

impl Fit {
    /// Height of the lines, in pixels.
    pub fn height(&self) -> f64 {
        self.lines.len() as f64 * self.px
    }
}

/// The height of one line of the font, in font units, as the renderer takes it: libass
/// mimics GDI and uses the OS/2 table's `usWinAscent + usWinDescent` when there is one, else
/// the hhea ascender minus descender. The renderer's font size is this height.
fn line_units(font: &FontRef) -> Option<f32> {
    if let Ok(o) = font.os2() {
        let sum = u32::from(o.us_win_ascent()) + u32::from(o.us_win_descent());
        if sum > 0 {
            return Some(sum as f32);
        }
    }
    let h = font.hhea().ok()?;
    let sum = i32::from(h.ascender().to_i16()) - i32::from(h.descender().to_i16());
    (sum > 0).then_some(sum as f32)
}

impl Shaped {
    /// Shapes `text` (lines separated by `\n`) with the face `index` of the font file
    /// `bytes`. `None` when the font cannot be read or the text is too long.
    pub fn new(bytes: &[u8], index: u32, text: &str) -> Option<Shaped> {
        if text.chars().count() > MAX_CHARS {
            return None;
        }
        let font = FontRef::from_index(bytes, index).ok()?;
        let line = line_units(&font)?;
        let data = ShaperData::new(&font);
        let shaper = data.shaper(&font).build();
        let measure = |s: &str| -> (f32, Vec<f32>) {
            let mut buf = UnicodeBuffer::new();
            buf.push_str(s);
            buf.guess_segment_properties();
            let out = shaper.shape(buf, harfrust::ShapeOptions::new());
            let scale = 1.0 / line;
            let infos = out.glyph_infos();
            let pos = out.glyph_positions();
            let mut cuts = Vec::with_capacity(s.chars().count());
            let mut total = 0.0f32;
            // width before each byte offset: advances of the glyphs of earlier clusters
            let mut at = 0usize;
            for (i, c) in s.char_indices() {
                let end = i + c.len_utf8();
                while at < infos.len() && (infos[at].cluster as usize) < end {
                    total += pos[at].x_advance as f32 * scale;
                    at += 1;
                }
                cuts.push(total);
            }
            while at < infos.len() {
                total += pos[at].x_advance as f32 * scale;
                at += 1;
            }
            if let Some(last) = cuts.last_mut() {
                *last = total;
            }
            (total, cuts)
        };
        let space = measure(" ").0;
        let paragraphs = text
            .split('\n')
            .map(|p| {
                p.split(' ')
                    .filter(|w| !w.is_empty())
                    .map(|w| {
                        let (width, cuts) = measure(w);
                        Word {
                            text: w.to_string(),
                            width,
                            cuts,
                        }
                    })
                    .collect()
            })
            .collect();
        Some(Shaped { space, paragraphs })
    }

    /// The lines of the text wrapped at `max` line heights, each with its width: greedy at
    /// the spaces, a word wider than the box is cut by characters, `\n` is a hard break
    /// (REQ-156).
    fn wrap(&self, max: f32) -> Vec<(String, f32)> {
        let max = max.max(f32::EPSILON);
        let mut lines = Vec::new();
        for words in &self.paragraphs {
            let (mut line, mut w) = (String::new(), 0.0f32);
            for word in words {
                if !line.is_empty() && w + self.space + word.width <= max {
                    line.push(' ');
                    line.push_str(&word.text);
                    w += self.space + word.width;
                    continue;
                }
                if !line.is_empty() {
                    lines.push((std::mem::take(&mut line), w));
                }
                if word.width <= max {
                    line = word.text.clone();
                    w = word.width;
                    continue;
                }
                // cut a word wider than the box into pieces that fit
                let chars: Vec<char> = word.text.chars().collect();
                let (mut from, mut base) = (0usize, 0.0f32);
                for i in 1..chars.len() {
                    if word.cuts[i] - base > max {
                        lines.push((chars[from..i].iter().collect(), word.cuts[i - 1] - base));
                        from = i;
                        base = word.cuts[i - 1];
                    }
                }
                line = chars[from..].iter().collect();
                w = word.cuts[chars.len() - 1] - base;
            }
            lines.push((line, w));
        }
        lines
    }

    /// Where each word of a one-paragraph text sits when the text is wrapped greedily at
    /// `box_w` pixels at the size `px` (REQ-333): its line, the distance of its left edge from
    /// the line's left edge and its width, in pixels, plus the width of every line. `None`
    /// for a text of several paragraphs or a word wider than the box (the caller then keeps
    /// the old layout).
    pub fn place_words(&self, box_w: f64, px: f64) -> Option<Placed> {
        if self.paragraphs.len() != 1 || px <= 0.0 {
            return None;
        }
        let max = (box_w / px) as f32;
        let mut boxes = Vec::new();
        let mut lines: Vec<f32> = Vec::new();
        let (mut line, mut x) = (0usize, 0.0f32);
        for (i, word) in self.paragraphs[0].iter().enumerate() {
            if word.width > max {
                return None;
            }
            if i > 0 && x + self.space + word.width > max {
                lines.push(x);
                line += 1;
                x = 0.0;
            } else if i > 0 {
                x += self.space;
            }
            boxes.push(WordBox {
                line,
                left: f64::from(x) * px,
                width: f64::from(word.width) * px,
            });
            x += word.width;
        }
        lines.push(x);
        Some(Placed {
            boxes,
            line_widths: lines.into_iter().map(|w| f64::from(w) * px).collect(),
        })
    }

    /// The text wrapped at `box_w` pixels at the fixed size `px` (no fitting).
    pub fn at_size(&self, box_w: f64, px: f64) -> Fit {
        let lines = self.wrap((box_w / px) as f32);
        Fit {
            px,
            width: lines.iter().map(|l| f64::from(l.1)).fold(0.0, f64::max) * px,
            lines: lines.into_iter().map(|l| l.0).collect(),
            fits: true,
        }
    }

    /// The **best fit** (REQ-156, REQ-157): the largest size, in pixels, at which the text
    /// wrapped at `box_w` fits inside `box_h`, from `min_px` (a text that does not fit even
    /// there sets `fits` to false) up to `max_px`. Steps of a tenth of a pixel, so the lines
    /// fill 90% or more of the box height unless the limits apply.
    pub fn fit(&self, box_w: f64, box_h: f64, min_px: f64, max_px: f64) -> Fit {
        let at = |px: f64| self.wrap((box_w / px) as f32);
        let fits = |px: f64, n: usize| n as f64 * px <= box_h + 1e-9;
        let build = |px: f64, lines: Vec<(String, f32)>, ok: bool| Fit {
            px,
            width: lines.iter().map(|l| f64::from(l.1)).fold(0.0, f64::max) * px,
            lines: lines.into_iter().map(|l| l.0).collect(),
            fits: ok,
        };
        let hi = max_px.min(box_h).max(min_px);
        let lines = at(hi);
        if fits(hi, lines.len()) {
            return build(hi, lines, true);
        }
        let low = at(min_px);
        if !fits(min_px, low.len()) {
            return build(min_px, low, false);
        }
        let (mut lo, mut up) = (min_px, hi);
        while up - lo > STEP / 4.0 {
            let mid = (lo + up) / 2.0;
            if fits(mid, at(mid).len()) {
                lo = mid;
            } else {
                up = mid;
            }
        }
        let px = ((lo / STEP).floor() * STEP).max(min_px);
        let lines = at(px);
        build(px, lines, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ass::FONT;

    fn shaped(text: &str) -> Shaped {
        Shaped::new(FONT, 0, text).expect("the bundled font reads")
    }

    // covers: REQ-156
    #[test]
    fn a_wider_letter_makes_a_wider_word() {
        let w = shaped("WWWW iiii");
        assert!(w.paragraphs[0][0].width > 2.0 * w.paragraphs[0][1].width);
        assert!(w.space > 0.0);
        // the cuts add up to the word
        let word = &w.paragraphs[0][0];
        assert_eq!(word.cuts.len(), 4);
        assert!((word.cuts[3] - word.width).abs() < 1e-4);
        assert!(word.cuts[0] < word.cuts[1]);
    }

    // covers: REQ-333
    #[test]
    fn words_are_placed_left_to_right_and_wrapped_by_line() {
        let s = shaped("one two three");
        let wide = s.place_words(10_000.0, 50.0).unwrap();
        assert_eq!(wide.boxes.len(), 3);
        assert_eq!(wide.line_widths.len(), 1);
        assert_eq!(wide.boxes[0].left, 0.0);
        assert!(wide.boxes[1].left > wide.boxes[0].width);
        let last = wide.boxes[2];
        assert!((last.left + last.width - wide.line_widths[0]).abs() < 1e-3);
        // a narrow box wraps: every word on its own line, starting at the left edge
        let narrow = s.place_words(wide.boxes[2].width * 1.01, 50.0).unwrap();
        assert_eq!(narrow.line_widths.len(), 3);
        assert_eq!(narrow.boxes[2].line, 2);
        assert_eq!(narrow.boxes[2].left, 0.0);
        // a word wider than the box, or several paragraphs: no placement
        assert!(s.place_words(10.0, 50.0).is_none());
        assert!(shaped("a\nb").place_words(10_000.0, 50.0).is_none());
        assert!(s.place_words(10_000.0, 0.0).is_none());
    }

    // covers: REQ-156
    #[test]
    fn text_wraps_at_the_spaces_and_hard_breaks_stay() {
        let s = shaped("one two three\nfour");
        let text = |v: Vec<(String, f32)>| v.into_iter().map(|l| l.0).collect::<Vec<_>>();
        assert_eq!(text(s.wrap(100.0)), ["one two three", "four"]);
        let narrow = s.wrap(s.paragraphs[0][2].width * 1.01);
        assert_eq!(text(narrow), ["one", "two", "three", "four"]);
        // a word wider than the box is cut by characters, nothing is lost
        let long = shaped("WWWWWWWWWW");
        let lines = text(long.wrap(long.paragraphs[0][0].width / 3.0));
        assert!(lines.len() >= 3, "{lines:?}");
        assert_eq!(lines.concat(), "WWWWWWWWWW");
        // an empty line between two paragraphs is kept
        assert_eq!(text(shaped("a\n\nb").wrap(100.0)), ["a", "", "b"]);
    }

    // covers: REQ-156
    #[test]
    fn ten_texts_fill_ninety_to_a_hundred_percent_of_the_box() {
        let texts = [
            "Hello",
            "A somewhat longer line of text to fit",
            "WWWWWWWW WWWWWWWW",
            "iiiiiiii iiiiiiii iiiiiiii",
            "one\ntwo",
            "one\ntwo\nthree",
            "one\ntwo\nthree\nfour",
            "The quick brown fox jumps over the lazy dog and keeps running far away",
            "Mixed Wide MMM and narrow lll words together in a sentence that wraps",
            "Two lines of moderate\nlength here",
        ];
        let (w, h) = (1536.0, 216.0);
        for t in texts {
            let f = shaped(t).fit(w, h, 21.6, 108.0);
            assert!(f.fits, "{t}");
            let fill = f.height() / h;
            // short texts stop at the cap, the rest fills the box
            assert!(
                (0.9..=1.0 + 1e-9).contains(&fill) || (f.px - 108.0).abs() < 1e-9 && fill <= 1.0,
                "{t:?}: {} px, {} lines, fill {fill}",
                f.px,
                f.lines.len()
            );
            assert!(f.width <= w + 1.0, "{t:?} wider than the box: {}", f.width);
        }
    }

    // covers: REQ-157
    #[test]
    fn a_short_text_stops_at_the_size_cap() {
        let f = shaped("Hi").fit(1536.0, 216.0, 21.6, 108.0);
        assert_eq!(f.px, 108.0);
        assert_eq!(f.lines, ["Hi"]);
    }

    // covers: REQ-159
    #[test]
    fn a_text_that_cannot_fit_gets_the_minimum_and_a_warning() {
        let long = "word ".repeat(300);
        let f = shaped(&long).fit(400.0, 40.0, 21.6, 108.0);
        assert!(!f.fits);
        assert_eq!(f.px, 21.6);
        // a bigger box makes the same text larger
        let g = shaped(&long).fit(1536.0, 600.0, 21.6, 108.0);
        assert!(g.fits && g.px > f.px);
    }

    // covers: REQ-116
    #[test]
    fn a_wider_box_makes_the_text_larger_and_longer_lines_smaller() {
        let t = "alpha beta gamma\ndelta epsilon";
        let a = shaped(t).fit(500.0, 216.0, 21.6, 108.0);
        let b = shaped(t).fit(1000.0, 216.0, 21.6, 108.0);
        assert!(b.px > a.px, "{} {}", a.px, b.px);
        let c = shaped("alpha beta gamma alpha beta gamma\ndelta epsilon delta epsilon")
            .fit(500.0, 216.0, 21.6, 108.0);
        assert!(c.px < a.px);
    }

    // covers: REQ-163
    #[test]
    fn a_file_that_is_not_a_font_gives_no_measure() {
        assert!(Shaped::new(b"not a font", 0, "x").is_none());
        // a font cut anywhere never panics (it measures or it does not)
        for cut in [0, 1, 11, 12, 100, 600, 5000, FONT.len() / 3, FONT.len() - 1] {
            let _ = Shaped::new(&FONT[..cut], 0, "Hello world");
        }
        assert!(Shaped::new(&FONT[..100], 0, "x").is_none());
        assert!(Shaped::new(FONT, 7, "x").is_none());
        assert!(Shaped::new(FONT, 0, &"x".repeat(MAX_CHARS + 1)).is_none());
    }
}
