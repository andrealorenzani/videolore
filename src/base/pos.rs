// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Positions on the picture and anchors on the 360 sphere.

use serde::{Deserialize, Serialize};

/// Largest coordinate of a [`Pos`] (the right or bottom edge of the picture).
pub const POS_MAX: u16 = 10_000;

/// Where a line is on screen: the centre of its text box, in 1/10000 of the output
/// picture's width (`x`) and height (`y`). Survives a change of output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: u16,
    pub y: u16,
}

impl Pos {
    /// A position from fractions of the picture (clamped to it).
    pub fn from_fraction(x: f64, y: f64) -> Pos {
        let f = |v: f64| {
            if v.is_finite() {
                (v.clamp(0.0, 1.0) * f64::from(POS_MAX)).round() as u16
            } else {
                POS_MAX / 2
            }
        };
        Pos { x: f(x), y: f(y) }
    }

    /// The position as fractions of the picture.
    pub fn fraction(self) -> (f64, f64) {
        (
            f64::from(self.x.min(POS_MAX)) / f64::from(POS_MAX),
            f64::from(self.y.min(POS_MAX)) / f64::from(POS_MAX),
        )
    }

    /// True when both coordinates are inside the picture.
    pub fn is_valid(self) -> bool {
        self.x <= POS_MAX && self.y <= POS_MAX
    }
}

/// A point of the 360° world a text or subtitle line is fixed to (REQ-112), in
/// hundredths of a degree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Anchor360 {
    pub yaw_cd: i32,
    pub pitch_cd: i32,
    /// Height as an angle. For a text item: the height of its box. For a subtitle line:
    /// the height of its text at 100% (the line's own size is a factor of it). Files of
    /// 1.3.0 and before call it `size_cd`.
    #[serde(alias = "size_cd")]
    pub height_cd: u32,
    /// Width the text wraps at (the width of the box), as an angle. Files of 1.3.0 and
    /// before call it `wrap_cd`.
    #[serde(alias = "wrap_cd")]
    pub width_cd: u32,
}

impl Anchor360 {
    pub fn is_valid(&self) -> bool {
        self.yaw_cd.abs() <= 18_000
            && self.pitch_cd.abs() <= 9_000
            && (1..=18_000).contains(&self.height_cd)
            && (1..=36_000).contains(&self.width_cd)
    }
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
