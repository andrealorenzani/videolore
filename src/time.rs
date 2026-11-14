// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Time arithmetic: microsecond timestamps, frame rates, frame rounding, timecodes.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// A point or span of time in microseconds. Integer to keep edits exact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Micros(pub i64);

impl Micros {
    pub const ZERO: Micros = Micros(0);
    pub const SECOND: Micros = Micros(1_000_000);

    pub fn from_secs_f64(secs: f64) -> Micros {
        if !secs.is_finite() {
            return Micros::ZERO;
        }
        Micros((secs * 1_000_000.0).round() as i64)
    }

    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    pub fn min(self, other: Micros) -> Micros {
        Micros(self.0.min(other.0))
    }

    pub fn max(self, other: Micros) -> Micros {
        Micros(self.0.max(other.0))
    }

    pub fn clamp(self, lo: Micros, hi: Micros) -> Micros {
        if hi < lo {
            return lo;
        }
        Micros(self.0.clamp(lo.0, hi.0))
    }

    /// Seconds with microsecond precision, formatted for ffmpeg arguments (e.g. `1.500000`).
    pub fn ff(self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.unsigned_abs();
        format!("{sign}{}.{:06}", a / 1_000_000, a % 1_000_000)
    }
}

impl Add for Micros {
    type Output = Micros;
    fn add(self, rhs: Micros) -> Micros {
        Micros(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Micros {
    type Output = Micros;
    fn sub(self, rhs: Micros) -> Micros {
        Micros(self.0.saturating_sub(rhs.0))
    }
}

/// Frame rate as a rational number (e.g. 30000/1001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fps {
    pub num: u32,
    pub den: u32,
}

impl Fps {
    pub fn new(num: u32, den: u32) -> Option<Fps> {
        (num > 0 && den > 0).then_some(Fps { num, den })
    }

    /// Parses ffprobe rates like `30000/1001` or `25`.
    pub fn parse(s: &str) -> Option<Fps> {
        let s = s.trim();
        match s.split_once('/') {
            Some((n, d)) => Fps::new(n.trim().parse().ok()?, d.trim().parse().ok()?),
            None => Fps::new(s.parse().ok()?, 1),
        }
    }

    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Duration of one frame (rounded to the nearest microsecond).
    pub fn frame_duration(self) -> Micros {
        Micros(((1_000_000u64 * self.den as u64 + self.num as u64 / 2) / self.num as u64) as i64)
    }

    /// Index of the frame closest to `t`.
    pub fn frame_index(self, t: Micros) -> i64 {
        let n = t.0 as i128 * self.num as i128;
        let d = 1_000_000i128 * self.den as i128;
        ((n + d / 2).div_euclid(d)) as i64
    }

    /// Start time of frame `index`.
    pub fn frame_time(self, index: i64) -> Micros {
        let n = index as i128 * 1_000_000 * self.den as i128;
        Micros((n / self.num as i128) as i64)
    }

    /// Rounds `t` to the nearest frame boundary.
    pub fn snap(self, t: Micros) -> Micros {
        self.frame_time(self.frame_index(t))
    }

    /// Ordering key (frames per second) used when comparing formats.
    pub fn cmp_value(self) -> u128 {
        // Compare num1/den1 vs num2/den2 without floats: scale by a common big factor.
        (self.num as u128 * 1_000_000_000) / self.den as u128
    }

    /// `num/den` as accepted by ffmpeg.
    pub fn ff(self) -> String {
        format!("{}/{}", self.num, self.den)
    }
}

impl fmt::Display for Fps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{:.3}", self.as_f64())
        }
    }
}

/// `HH:MM:SS.mmm` timecode (negative values are clamped to zero).
pub fn timecode(t: Micros) -> String {
    let t = t.0.max(0);
    let ms = t / 1000;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

/// Parses `HH:MM:SS.mmm`, `MM:SS.mmm` or plain seconds. Returns `None` on bad input.
pub fn parse_timecode(s: &str) -> Option<Micros> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut total = 0.0f64;
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() > 3 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        let v: f64 = p.parse().ok()?;
        if !v.is_finite() || v < 0.0 {
            return None;
        }
        if i + 1 < parts.len() && v.fract() != 0.0 {
            return None;
        }
        total = total * 60.0 + v;
    }
    Some(Micros::from_secs_f64(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micros_arith_and_format() {
        assert_eq!(Micros(5) + Micros(7), Micros(12));
        assert_eq!(Micros(5) - Micros(7), Micros(-2));
        assert_eq!(Micros(1_500_000).ff(), "1.500000");
        assert_eq!(Micros(-250).ff(), "-0.000250");
        assert_eq!(Micros::from_secs_f64(1.25), Micros(1_250_000));
        assert_eq!(Micros::from_secs_f64(f64::NAN), Micros::ZERO);
        assert_eq!(Micros(3).clamp(Micros(5), Micros(9)), Micros(5));
        assert_eq!(Micros(3).clamp(Micros(9), Micros(5)), Micros(9));
        assert_eq!(Micros(3).max(Micros(4)), Micros(4));
        assert_eq!(Micros(3).min(Micros(4)), Micros(3));
        assert!((Micros(2_500_000).as_secs_f64() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn fps_parse_and_frames() {
        let f = Fps::parse("30000/1001").unwrap();
        assert_eq!(f.frame_duration(), Micros(33_367));
        assert_eq!(Fps::parse("25").unwrap(), Fps { num: 25, den: 1 });
        assert!(Fps::parse("0/0").is_none());
        assert!(Fps::parse("x").is_none());
        let f25 = Fps::new(25, 1).unwrap();
        assert_eq!(f25.frame_index(Micros(41_000)), 1);
        assert_eq!(f25.frame_time(3), Micros(120_000));
        assert_eq!(f25.snap(Micros(130_000)), Micros(120_000));
        assert!(f25.cmp_value() < Fps::new(30, 1).unwrap().cmp_value());
        assert_eq!(f25.ff(), "25/1");
        assert_eq!(f25.to_string(), "25");
        assert_eq!(f.to_string(), "29.970");
    }

    #[test]
    fn timecodes() {
        assert_eq!(timecode(Micros(3_723_456_000)), "01:02:03.456");
        assert_eq!(timecode(Micros(-5)), "00:00:00.000");
        assert_eq!(parse_timecode("01:02:03.5"), Some(Micros(3_723_500_000)));
        assert_eq!(parse_timecode("2.25"), Some(Micros(2_250_000)));
        assert_eq!(parse_timecode("1:30"), Some(Micros(90_000_000)));
        assert_eq!(parse_timecode(""), None);
        assert_eq!(parse_timecode("1:2:3:4"), None);
        assert_eq!(parse_timecode("a"), None);
        assert_eq!(parse_timecode("-1"), None);
        assert_eq!(parse_timecode("1.5:00"), None);
    }
}
