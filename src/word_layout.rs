// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Where the words of a subtitle line are drawn when one of them is enlarged (3.1.0, REQ-334,
//! ADR-248). Pure numbers in, numbers out: the measured words of a line (from `measure`) and
//! the size of the enlarged word give each word's left edge, the bottom of its line box and
//! its font size. The enlarged word keeps its baseline and grows upwards; it grows by the same
//! amount on both sides, so its centre stays; the words before it on its line move left and
//! those after it move right, each by half of the extra width, so the line stays centred and
//! no two words overlap. Imports nothing from the other modules.

/// One word at its normal size, in pixels: its line, the distance of its left edge from the
/// left edge of its line, and its width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub line: usize,
    pub left: f64,
    pub width: f64,
}

/// The measured line: every word's slot, the width of each line, the centre of the whole text
/// block, the font size (also the height of a line) and the descent (baseline to the bottom of
/// a line, in line heights).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub slots: Vec<Slot>,
    pub line_widths: Vec<f64>,
    pub centre: (f64, f64),
    pub px: f64,
    pub descent: f64,
}

/// Where one word is drawn: its left edge, the bottom of its line box (for a bottom-left
/// anchor) and its font size, all in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub left: f64,
    pub bottom: f64,
    pub size: f64,
}

impl Block {
    /// The word `k` at its normal size.
    fn normal(&self, k: usize) -> Spot {
        let s = self.slots[k];
        let top = self.centre.1 - self.line_widths.len() as f64 * self.px / 2.0;
        Spot {
            left: self.centre.0 - self.line_widths[s.line] / 2.0 + s.left,
            bottom: top + (s.line + 1) as f64 * self.px,
            size: self.px,
        }
    }

    /// Every word's place while word `big_index` is drawn at `big_px` (the other words keep
    /// their size). `None` for `big_index` out of range gives the line at normal size.
    pub fn with_enlarged(&self, big_index: usize, big_px: f64) -> Vec<Spot> {
        let mut out: Vec<Spot> = (0..self.slots.len()).map(|k| self.normal(k)).collect();
        let Some(word) = self.slots.get(big_index) else {
            return out;
        };
        let extra = word.width * (big_px / self.px - 1.0);
        for (k, spot) in out.iter_mut().enumerate() {
            if self.slots[k].line != word.line {
                continue;
            }
            match k.cmp(&big_index) {
                std::cmp::Ordering::Less => spot.left -= extra / 2.0,
                std::cmp::Ordering::Greater => spot.left += extra / 2.0,
                std::cmp::Ordering::Equal => {
                    // grows on both sides, upwards from the same baseline
                    spot.left -= extra / 2.0;
                    spot.bottom += self.descent * (big_px - self.px);
                    spot.size = big_px;
                }
            }
        }
        out
    }

    /// The width of word `k` when drawn at `size`.
    pub fn width_at(&self, k: usize, size: f64) -> f64 {
        self.slots[k].width * size / self.px
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "aaa bbb ccc" on one line at 100 px: words 200 wide, spaces 30.
    fn block() -> Block {
        Block {
            slots: (0..3)
                .map(|i| Slot {
                    line: 0,
                    left: f64::from(i) * 230.0,
                    width: 200.0,
                })
                .collect(),
            line_widths: vec![660.0],
            centre: (960.0, 900.0),
            px: 100.0,
            descent: 0.2,
        }
    }

    // covers: REQ-334
    #[test]
    fn the_base_stays_and_the_word_grows_upwards_from_it() {
        let b = block();
        let normal = b.with_enlarged(9, 140.0);
        let big = b.with_enlarged(1, 140.0);
        // the baseline is the bottom of the box minus the descent at the word's own size
        let base = |s: &Spot| s.bottom - 0.2 * s.size;
        assert!((base(&big[1]) - base(&normal[1])).abs() < 1e-9);
        assert_eq!(big[1].size, 140.0);
        // upwards only: the top of the word is higher
        assert!(big[1].bottom - big[1].size < normal[1].bottom - normal[1].size);
        // the other words are drawn at their own size and baseline
        assert_eq!((big[0].size, big[0].bottom), (100.0, normal[0].bottom));
    }

    // covers: REQ-334
    #[test]
    fn the_centre_stays_and_the_neighbours_move_by_half_the_extra_width() {
        let b = block();
        let normal = b.with_enlarged(9, 140.0);
        let big = b.with_enlarged(1, 140.0);
        let extra = 200.0 * 0.4;
        let centre = |s: &Spot, w: f64| s.left + w / 2.0;
        assert!((centre(&big[1], b.width_at(1, 140.0)) - centre(&normal[1], 200.0)).abs() < 1e-9);
        assert!((normal[0].left - big[0].left - extra / 2.0).abs() < 1e-9);
        assert!((big[2].left - normal[2].left - extra / 2.0).abs() < 1e-9);
        // the line is still centred: the left of the first and the right of the last move
        // by the same amount in opposite directions
        let right = big[2].left + 200.0;
        assert!(((960.0 - big[0].left) - (right - 960.0)).abs() < 1e-9);
    }

    // covers: REQ-334
    #[test]
    fn at_forty_pixels_no_two_words_overlap() {
        let mut b = block();
        b.px = 54.0;
        // 40 px at a picture height of 1080 is the largest growth
        for i in 0..3 {
            let spots = b.with_enlarged(i, 54.0 + 40.0);
            for k in 0..2 {
                let right = spots[k].left + b.width_at(k, spots[k].size);
                assert!(right <= spots[k + 1].left + 1e-9, "word {k} overlaps");
            }
        }
    }

    // covers: REQ-334
    #[test]
    fn words_on_other_lines_do_not_move() {
        let b = Block {
            slots: vec![
                Slot {
                    line: 0,
                    left: 0.0,
                    width: 100.0,
                },
                Slot {
                    line: 1,
                    left: 0.0,
                    width: 100.0,
                },
            ],
            line_widths: vec![100.0, 100.0],
            centre: (500.0, 500.0),
            px: 50.0,
            descent: 0.25,
        };
        let normal = b.with_enlarged(9, 70.0);
        let big = b.with_enlarged(0, 70.0);
        assert_eq!(big[1], normal[1]);
        assert!(big[0].left < normal[0].left);
    }
}
