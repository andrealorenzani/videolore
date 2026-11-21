// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The picture frame (1.5.0, REQ-124..131): Change aspect, Scale and Move. Pure maths and
//! text generation, no I/O.
//!
//! * [`layout`] gives the frame size of a Change aspect and the size of the picture in it.
//! * [`Stack`] holds the Scale and Move keys that act on a clip (the whole video's and the
//!   clip's) and turns them into ffmpeg expressions ([`Stack::scale_factor`],
//!   [`Stack::offset_x`]) or into a value at a time ([`Stack::at`]).
//!
//! Expressions are built only from numbers written by this module, never from user text.

use crate::base::pos::{POS_MAX, Pos};
use crate::keyframes::{Key, Lerp, lerp_i64};
use crate::time::Micros;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Write as _;

/// Longest side of a frame (decided 2026-11-19).
pub const MAX_SIDE: u32 = 4096;
/// Scale limits, in permille of the picture's normal size.
pub const MIN_SCALE: u16 = 100;
pub const MAX_SCALE: u16 = 4000;
pub const NORMAL_SCALE: u16 = 1000;
/// Largest centre offset a Scale key can carry (1/10000 of the frame).
pub const MAX_OFFSET: i16 = 20_000;
/// The blurred background is made on a copy this many times smaller.
pub const BLUR_DIVISOR: u32 = 8;
/// Box blur radius on the small copy.
pub const BLUR_RADIUS: u32 = 3;
/// Smallest side of the small copy (even), so both radii stay valid.
pub const BLUR_MIN_SIDE: u32 = 8;

/// The shape of the frame: one of the three presets or a custom ratio from 1:4 to 4:1
/// (REQ-285). Written in the project as "w:h".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    Wide4x3,
    Wide16x9,
    Tall9x16,
    /// Width and height of the ratio, whole numbers; see [`Shape::custom`].
    Custom(u8, u8),
}

/// Why a custom ratio is refused.
pub const CUSTOM_RATIO_NOTE: &str = "A ratio goes from 1:4 to 4:1, in whole numbers.";

impl Shape {
    /// The presets (the Custom choice is made by typing the two numbers).
    pub const ALL: [Shape; 3] = [Shape::Wide4x3, Shape::Wide16x9, Shape::Tall9x16];

    /// A custom shape `w:h`: both from 1 to 100 and the ratio from 1:4 to 4:1. A preset's
    /// ratio gives the preset itself.
    pub fn custom(w: u32, h: u32) -> Option<Shape> {
        if w == 0 || h == 0 || w > 100 || h > 100 || w > 4 * h || h > 4 * w {
            return None;
        }
        let s = Shape::Custom(w as u8, h as u8);
        Some(
            Shape::ALL
                .into_iter()
                .find(|p| p.ratio().0 * h == p.ratio().1 * w && p.ratio() == (w, h))
                .unwrap_or(s),
        )
    }

    pub fn name(self) -> String {
        let (w, h) = self.ratio();
        format!("{w}:{h}")
    }

    /// Width and height of the ratio.
    pub fn ratio(self) -> (u32, u32) {
        match self {
            Shape::Wide4x3 => (4, 3),
            Shape::Wide16x9 => (16, 9),
            Shape::Tall9x16 => (9, 16),
            Shape::Custom(w, h) => (u32::from(w), u32::from(h)),
        }
    }

    /// Reads "w:h".
    pub fn parse(s: &str) -> Option<Shape> {
        let (w, h) = s.split_once(':')?;
        Shape::custom(w.trim().parse().ok()?, h.trim().parse().ok()?)
    }
}

impl Serialize for Shape {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for Shape {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Shape, D::Error> {
        let s = String::deserialize(d)?;
        Shape::parse(&s).ok_or_else(|| serde::de::Error::custom("a shape is w:h from 1:4 to 4:1"))
    }
}

/// Which side of the video the frame keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    Width,
    Height,
}

impl Fit {
    pub const ALL: [Fit; 2] = [Fit::Width, Fit::Height];

    pub fn name(self) -> &'static str {
        match self {
            Fit::Width => "Fit on width",
            Fit::Height => "Fit on height",
        }
    }
}

/// A colour, written `#RRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const BLACK: Rgb = Rgb(0, 0, 0);

    /// Strict `#RRGGBB` (hexadecimal digits only).
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.strip_prefix('#')?;
        if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let n = u32::from_str_radix(h, 16).ok()?;
        Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }

    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// The colour as ffmpeg writes it.
    pub fn ff(self) -> String {
        format!("0x{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Rgb, D::Error> {
        let s = String::deserialize(d)?;
        Rgb::parse(&s).ok_or_else(|| serde::de::Error::custom("a colour is #RRGGBB"))
    }
}

/// What fills the empty space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fill {
    Colour(Rgb),
    Blur,
}

impl Fill {
    pub fn name(self) -> &'static str {
        match self {
            Fill::Colour(_) => "Fit empty with colour",
            Fill::Blur => "Fit empty with blurred video",
        }
    }
}

/// How blurred the background of a Change aspect frame is (REQ-284).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlurLevel {
    Low,
    /// The blur of every earlier version.
    #[default]
    Medium,
    High,
}

impl BlurLevel {
    pub const ALL: [BlurLevel; 3] = [BlurLevel::Low, BlurLevel::Medium, BlurLevel::High];

    pub fn name(self) -> &'static str {
        match self {
            BlurLevel::Low => "Low",
            BlurLevel::Medium => "Medium",
            BlurLevel::High => "High",
        }
    }

    /// The box blur radius on the small copy.
    pub fn radius(self) -> u32 {
        match self {
            BlurLevel::Low => 1,
            BlurLevel::Medium => BLUR_RADIUS,
            BlurLevel::High => 2 * BLUR_RADIUS,
        }
    }
}

fn is_medium(b: &BlurLevel) -> bool {
    *b == BlurLevel::Medium
}

/// The Change aspect effect of the whole video (REQ-124).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameAspect {
    pub shape: Shape,
    pub fit: Fit,
    pub fill: Fill,
    /// The blur of a blurred fill; not written when Medium.
    #[serde(default, skip_serializing_if = "is_medium")]
    pub blur: BlurLevel,
}

impl Default for FrameAspect {
    /// A new effect: 16:9, Fit on width, black.
    fn default() -> FrameAspect {
        FrameAspect {
            shape: Shape::Wide16x9,
            fit: Fit::Width,
            fill: Fill::Colour(Rgb::BLACK),
            blur: BlurLevel::Medium,
        }
    }
}

/// Reads a Change aspect leniently: an unknown shape, fit or fill reads as "no effect"
/// (REQ-132: a damaged file never crashes).
pub fn lenient_aspect<'de, D: Deserializer<'de>>(d: D) -> Result<Option<FrameAspect>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(serde_json::from_value(v).unwrap_or(None))
}

fn even(x: f64) -> u32 {
    (((x / 2.0).round() as u32) * 2).max(2)
}

/// Scales both sides so that the longer one is `max` at most (same shape, even sides).
fn capped(size: (u32, u32), max: u32) -> (u32, u32) {
    let long = size.0.max(size.1);
    if long <= max {
        return size;
    }
    let k = f64::from(max) / f64::from(long);
    (even(f64::from(size.0) * k), even(f64::from(size.1) * k))
}

/// The frame of `aspect` over a video whose size without the effect is `base`; with no
/// aspect the frame is the base (REQ-125). Sides are even and at most [`MAX_SIDE`].
pub fn frame_size(base: (u32, u32), aspect: Option<&FrameAspect>) -> (u32, u32) {
    match aspect {
        None => base,
        Some(a) => capped(wanted_frame(base, a), MAX_SIDE),
    }
}

/// True when the frame of `aspect` over a video of size `base` has a longer side above
/// [`MAX_SIDE`] before the cap: it is scaled down and the export takes longer (REQ-221).
pub fn over_cap(base: (u32, u32), aspect: Option<&FrameAspect>) -> bool {
    aspect.is_some_and(|a| {
        let f = wanted_frame(base, a);
        f.0.max(f.1) > MAX_SIDE
    })
}

/// The frame of `a` over `base`, not yet capped.
fn wanted_frame(base: (u32, u32), a: &FrameAspect) -> (u32, u32) {
    let (rw, rh) = a.shape.ratio();
    match a.fit {
        Fit::Width => (
            base.0,
            even(f64::from(base.0) * f64::from(rh) / f64::from(rw)),
        ),
        Fit::Height => (
            even(f64::from(base.1) * f64::from(rw) / f64::from(rh)),
            base.1,
        ),
    }
}

/// The same frame for a preview at most `height` high.
pub fn preview_size(frame: (u32, u32), height: u32) -> (u32, u32) {
    if frame.1 <= height {
        return frame;
    }
    let k = f64::from(height) / f64::from(frame.1);
    (even(f64::from(frame.0) * k), height & !1)
}

/// The size of the picture of a `src` video in a frame, before Scale: the video fitted to
/// the frame's width (Fit on width) or height (Fit on height), or inside the frame when
/// there is no Change aspect. Mirrors the ffmpeg filter of [`fit_filter`].
pub fn picture_size(
    frame: (u32, u32),
    aspect: Option<&FrameAspect>,
    src: (u32, u32),
) -> (u32, u32) {
    let (sw, sh) = (f64::from(src.0.max(1)), f64::from(src.1.max(1)));
    let (fw, fh) = (f64::from(frame.0), f64::from(frame.1));
    let (w, h) = match aspect.map(|a| a.fit) {
        Some(Fit::Width) => (fw, fw * sh / sw),
        Some(Fit::Height) => (fh * sw / sh, fh),
        None => {
            let k = (fw / sw).min(fh / sh);
            (sw * k, sh * k)
        }
    };
    (even(w), even(h))
}

/// The size a 360 video's camera view is rendered at, before Scale: it fills the frame,
/// or is a 16:9 picture on the kept side of a Change aspect.
pub fn camera_size(frame: (u32, u32), aspect: Option<&FrameAspect>) -> (u32, u32) {
    match aspect.map(|a| a.fit) {
        Some(Fit::Width) => (frame.0, even(f64::from(frame.0) * 9.0 / 16.0)),
        Some(Fit::Height) => (even(f64::from(frame.1) * 16.0 / 9.0), frame.1),
        None => frame,
    }
}

/// The ffmpeg filter (with a leading comma) that fits a flat picture for the frame effect
/// (no bars: the background is drawn separately).
pub fn fit_filter(frame: (u32, u32), aspect: Option<&FrameAspect>) -> String {
    match aspect.map(|a| a.fit) {
        Some(Fit::Width) => format!(",scale={}:-2", frame.0),
        Some(Fit::Height) => format!(",scale=-2:{}", frame.1),
        None => format!(
            ",scale={}:{}:force_original_aspect_ratio=decrease",
            frame.0, frame.1
        ),
    }
}

/// How much Scale a picture has: permille of its normal size, plus the offset of its centre
/// that the corner handles leave (1/10000 of the frame; the opposite corner stays fixed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scale {
    pub permille: u16,
    #[serde(default)]
    pub dx: i16,
    #[serde(default)]
    pub dy: i16,
}

impl Scale {
    pub const NORMAL: Scale = Scale {
        permille: NORMAL_SCALE,
        dx: 0,
        dy: 0,
    };

    /// Brings every field inside its limits; true when something changed.
    pub fn clamp(&mut self) -> bool {
        let old = *self;
        self.permille = self.permille.clamp(MIN_SCALE, MAX_SCALE);
        self.dx = self.dx.clamp(-MAX_OFFSET, MAX_OFFSET);
        self.dy = self.dy.clamp(-MAX_OFFSET, MAX_OFFSET);
        *self != old
    }

    pub fn factor(self) -> f64 {
        f64::from(self.permille) / 1000.0
    }
}

impl Lerp for u8 {
    fn lerp(a: u8, b: u8, num: i64, den: i64) -> u8 {
        lerp_i64(i64::from(a), i64::from(b), num, den).clamp(0, 100) as u8
    }
}

impl Lerp for Scale {
    fn lerp(a: Scale, b: Scale, num: i64, den: i64) -> Scale {
        let l = |x: i64, y: i64| lerp_i64(x, y, num, den);
        Scale {
            permille: l(a.permille.into(), b.permille.into()) as u16,
            dx: l(a.dx.into(), b.dx.into()) as i16,
            dy: l(a.dy.into(), b.dy.into()) as i16,
        }
    }
}

impl Lerp for Pos {
    fn lerp(a: Pos, b: Pos, num: i64, den: i64) -> Pos {
        Pos {
            x: lerp_i64(a.x.into(), b.x.into(), num, den) as u16,
            y: lerp_i64(a.y.into(), b.y.into(), num, den) as u16,
        }
    }
}

/// The centre of the frame.
pub const CENTRE: Pos = Pos {
    x: POS_MAX / 2,
    y: POS_MAX / 2,
};

/// Keys of one effect together with the time shift between the key domain and the clip's
/// local time (`local = key time - offset`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track<T> {
    pub keys: Vec<Key<T>>,
    pub offset: Micros,
}

/// The Scale and Move keys acting on one clip: the whole video's and the clip's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stack {
    pub scale: Vec<Track<Scale>>,
    pub moved: Vec<Track<Pos>>,
}

/// A rectangle `(x0, y0, x1, y1)` in fractions of the frame.
pub type FractionRect = (f64, f64, f64, f64);

/// The picture's state at one time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Product of every Scale (1.0 = normal).
    pub factor: f64,
    /// Offset of the centre from the frame's centre, as fractions of the frame.
    pub ox: f64,
    pub oy: f64,
}

impl Placement {
    /// The picture's rectangle `(x0, y0, x1, y1)` as fractions of the frame.
    pub fn rect(self, frame: (u32, u32), picture: (u32, u32)) -> FractionRect {
        let w = f64::from(picture.0) * self.factor / f64::from(frame.0.max(1));
        let h = f64::from(picture.1) * self.factor / f64::from(frame.1.max(1));
        let (cx, cy) = (0.5 + self.ox, 0.5 + self.oy);
        (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0)
    }
}

fn frac(v: i16) -> f64 {
    f64::from(v) / f64::from(POS_MAX)
}

impl Stack {
    /// True when no key acts on the clip.
    pub fn is_empty(&self) -> bool {
        self.scale.is_empty() && self.moved.is_empty()
    }

    /// The picture's state at local time `t` (time since the clip's start in the
    /// timeline, as the renderer sees it).
    pub fn at(&self, local: Micros) -> Placement {
        let mut p = Placement {
            factor: 1.0,
            ox: 0.0,
            oy: 0.0,
        };
        for tr in &self.scale {
            if let Some(s) = crate::keyframes::at(&tr.keys, local + tr.offset) {
                p.factor *= s.factor();
                p.ox += frac(s.dx);
                p.oy += frac(s.dy);
            }
        }
        for tr in &self.moved {
            if let Some(m) = crate::keyframes::at(&tr.keys, local + tr.offset) {
                p.ox += f64::from(m.x) / f64::from(POS_MAX) - 0.5;
                p.oy += f64::from(m.y) / f64::from(POS_MAX) - 0.5;
            }
        }
        p
    }

    /// ffmpeg expression of the picture's scale factor (a product).
    pub fn scale_factor(&self) -> String {
        let terms: Vec<String> = self
            .scale
            .iter()
            .map(|tr| piecewise(tr, |s| s.factor()))
            .collect();
        if terms.is_empty() {
            return "1".into();
        }
        terms.join("*")
    }

    /// ffmpeg expression of the centre's offset along x (fraction of the frame).
    pub fn offset_x(&self) -> String {
        self.offset(
            |s| frac(s.dx),
            |p| f64::from(p.x) / f64::from(POS_MAX) - 0.5,
        )
    }

    /// ffmpeg expression of the centre's offset along y.
    pub fn offset_y(&self) -> String {
        self.offset(
            |s| frac(s.dy),
            |p| f64::from(p.y) / f64::from(POS_MAX) - 0.5,
        )
    }

    fn offset(&self, s: impl Fn(&Scale) -> f64 + Copy, m: impl Fn(&Pos) -> f64 + Copy) -> String {
        let mut terms: Vec<String> = self.scale.iter().map(|tr| piecewise(tr, s)).collect();
        terms.extend(self.moved.iter().map(|tr| piecewise(tr, m)));
        if terms.is_empty() {
            return "0".into();
        }
        terms.join("+")
    }
}

/// A number as ffmpeg reads it: fixed decimals, negative ones in parentheses.
fn num(v: f64) -> String {
    let v = if v.is_finite() { v } else { 0.0 };
    let mut s = String::new();
    let _ = write!(s, "{:.6}", v.abs());
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() || s == "0" {
        return "0".into();
    }
    if v < 0.0 { format!("(-{s})") } else { s }
}

/// A seconds value (key time relative to the clip), as ffmpeg reads it.
fn secs(t: Micros) -> String {
    num(t.0 as f64 / 1_000_000.0)
}

/// The expression of volume keys (percent, REQ-280) in the variable `t` of a clip whose sound
/// starts at `start` on the chain's clock: the level is interpolated, the gain is its square
/// (REQ-280, D12).
pub fn volume_expression(keys: &[Key<u8>], start: Micros) -> String {
    let tr = Track {
        keys: keys.to_vec(),
        offset: Micros::ZERO - start,
    };
    format!("pow({}/100,2)", piecewise(&tr, |v| f64::from(*v)))
}

/// The piecewise linear expression of a track in the variable `t`.
fn piecewise<T: Copy>(tr: &Track<T>, f: impl Fn(&T) -> f64) -> String {
    let pts: Vec<(Micros, f64)> = tr.keys.iter().map(|k| (k.t - tr.offset, f(&k.v))).collect();
    let eases: Vec<crate::keyframes::Ease> = tr.keys.iter().map(|k| k.ease).collect();
    match pts.as_slice() {
        [] => "0".into(),
        [(_, v)] => num(*v),
        _ => {
            // the last value, then the segments from the end backwards
            let mut e = num(pts[pts.len() - 1].1);
            for i in (0..pts.len() - 1).rev() {
                let ((t0, a), (t1, b)) = (pts[i], pts[i + 1]);
                let seg = if a == b {
                    num(a)
                } else {
                    // `max(0,t-t0)` holds the first value before the first key
                    let clamp = if i == 0 {
                        format!("max(0,t-{})", secs(t0))
                    } else {
                        format!("(t-{})", secs(t0))
                    };
                    if eases[i] == crate::keyframes::Ease::EaseInOut {
                        // smoothstep of the part of the segment done (REQ-282)
                        let f = format!("({clamp}/{})", secs(t1 - t0));
                        format!("({a}+({d})*{f}*{f}*(3-2*{f}))", a = num(a), d = num(b - a))
                    } else {
                        format!(
                            "({a}+({d})*{clamp}/{span})",
                            a = num(a),
                            d = num(b - a),
                            span = secs(t1 - t0)
                        )
                    }
                };
                if seg != e {
                    e = format!("if(lt(t,{}),{seg},{e})", secs(t1));
                }
            }
            e
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyframes::Key;

    fn sk(t: i64, p: u16) -> Key<Scale> {
        Key {
            t: Micros(t),
            v: Scale {
                permille: p,
                dx: 0,
                dy: 0,
            },
            ease: Default::default(),
        }
    }

    // covers: REQ-125
    #[test]
    fn the_numbers_of_the_requirement() {
        let a = FrameAspect {
            shape: Shape::Tall9x16,
            ..Default::default()
        };
        let f = frame_size((1600, 1200), Some(&a));
        assert_eq!(f, (1600, 2844));
        let p = picture_size(f, Some(&a), (1600, 1200));
        assert_eq!(p, (1600, 1200));
        assert_eq!((2844 - 1200) / 2, 822);
        let b = FrameAspect {
            fit: Fit::Height,
            ..a
        };
        let f = frame_size((1600, 1200), Some(&b));
        assert!((i64::from(f.0) - 675).abs() <= 1);
        assert_eq!(f.1, 1200);
        let p = picture_size(f, Some(&b), (1600, 1200));
        assert_eq!(p, (1600, 1200));
        let cut = (1600 - i64::from(f.0)) / 2;
        assert!((cut - 462).abs() <= 1);
        assert_eq!(fit_filter(f, Some(&b)), ",scale=-2:1200");
        assert_eq!(fit_filter((640, 360), Some(&a)), ",scale=640:-2");
        assert_eq!(
            fit_filter((640, 360), None),
            ",scale=640:360:force_original_aspect_ratio=decrease"
        );
    }

    // covers: REQ-125
    #[test]
    fn sides_are_even_and_the_same_shape_has_no_empty_space() {
        for shape in Shape::ALL {
            for fit in Fit::ALL {
                let a = FrameAspect {
                    shape,
                    fit,
                    fill: Fill::Blur,
                    blur: BlurLevel::Medium,
                };
                for base in [(1920, 1080), (1280, 720), (1000, 998), (4, 6)] {
                    let f = frame_size(base, Some(&a));
                    assert_eq!(f.0 % 2, 0);
                    assert_eq!(f.1 % 2, 0);
                }
            }
        }
        let f = frame_size((1920, 1080), Some(&FrameAspect::default()));
        assert_eq!(f, (1920, 1080));
        let a43 = FrameAspect {
            shape: Shape::Wide4x3,
            ..Default::default()
        };
        let f = frame_size((1600, 1200), Some(&a43));
        assert_eq!(f, (1600, 1200));
        assert_eq!(picture_size(f, Some(&a43), (1600, 1200)), f);
        assert_eq!(frame_size((640, 480), None), (640, 480));
        // no aspect: the picture sits inside the frame
        assert_eq!(picture_size((640, 360), None, (400, 400)), (360, 360));
        // a 16:9 video in a 4:3 frame has empty space above and below
        assert_eq!(
            picture_size((1600, 1200), Some(&a43), (1920, 1080)),
            (1600, 900)
        );
    }

    // covers: REQ-125
    #[test]
    fn a_frame_over_4096_is_scaled_down_keeping_the_shape() {
        let a = FrameAspect {
            shape: Shape::Tall9x16,
            ..Default::default()
        };
        let f = frame_size((3840, 2160), Some(&a));
        assert_eq!(f.1, 4096);
        assert!((i64::from(f.0) - 2304).abs() <= 2);
        let p = preview_size(f, 360);
        assert_eq!(p.1, 360);
        assert_eq!(preview_size(frame_size((320, 240), Some(&a)), 360).1, 360);
        assert_eq!(preview_size((200, 100), 360), (200, 100));
    }

    #[test]
    fn camera_pictures_fill_the_frame_or_the_kept_side() {
        let a = FrameAspect {
            shape: Shape::Tall9x16,
            ..Default::default()
        };
        assert_eq!(camera_size((640, 360), None), (640, 360));
        assert_eq!(camera_size((1080, 1920), Some(&a)), (1080, 608));
        let h = FrameAspect {
            fit: Fit::Height,
            ..a
        };
        assert_eq!(camera_size((608, 1080), Some(&h)), (1920, 1080));
    }

    #[test]
    fn colour_parser_is_strict() {
        assert_eq!(Rgb::parse("#FF8000"), Some(Rgb(255, 128, 0)));
        assert_eq!(Rgb::parse("#ff8000").unwrap().ff(), "0xFF8000");
        for bad in ["FF8000", "#FF80", "#FF80000", "#GG0000", "#+F0000", ""] {
            assert_eq!(Rgb::parse(bad), None, "{bad}");
        }
        let a = FrameAspect {
            fill: Fill::Colour(Rgb(1, 2, 3)),
            ..Default::default()
        };
        let j = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<FrameAspect>(&j).unwrap(), a);
    }

    // covers: REQ-132
    #[test]
    fn unknown_values_read_as_no_effect() {
        #[derive(Deserialize)]
        struct W {
            #[serde(default, deserialize_with = "lenient_aspect")]
            a: Option<FrameAspect>,
        }
        let ok = r##"{"a":{"shape":"9:16","fit":"height","fill":{"colour":"#000000"}}}"##;
        assert!(serde_json::from_str::<W>(ok).unwrap().a.is_some());
        for bad in [
            r#"{"a":{"shape":"5:1","fit":"height","fill":"blur"}}"#,
            r#"{"a":{"shape":"9:16","fit":"both","fill":"blur"}}"#,
            r#"{"a":{"shape":"9:16","fit":"width","fill":"paint"}}"#,
            r#"{"a":5}"#,
            r#"{"a":null}"#,
            r#"{}"#,
        ] {
            assert!(serde_json::from_str::<W>(bad).unwrap().a.is_none(), "{bad}");
        }
    }

    // covers: REQ-129
    #[test]
    fn stacks_multiply_scales_and_add_moves() {
        let st = Stack {
            scale: vec![
                Track {
                    keys: vec![sk(0, 1000), sk(4_000_000, 2000)],
                    offset: Micros::ZERO,
                },
                Track {
                    keys: vec![sk(0, 500)],
                    offset: Micros::ZERO,
                },
            ],
            moved: vec![Track {
                keys: vec![Key {
                    t: Micros::ZERO,
                    v: Pos { x: 6000, y: 5000 },
                    ease: Default::default(),
                }],
                offset: Micros::ZERO,
            }],
        };
        let p = st.at(Micros(2_000_000));
        assert!((p.factor - 0.75).abs() < 1e-9);
        assert!((p.ox - 0.1).abs() < 1e-9 && p.oy.abs() < 1e-9);
        let r = p.rect((1000, 1000), (1000, 1000));
        assert!((r.2 - r.0 - 0.75).abs() < 1e-9);
        assert!(Stack::default().is_empty());
    }

    // covers: REQ-131
    #[test]
    fn expressions_golden() {
        let one = Stack {
            scale: vec![Track {
                keys: vec![sk(0, 500)],
                offset: Micros::ZERO,
            }],
            moved: vec![],
        };
        assert_eq!(one.scale_factor(), "0.5");
        assert_eq!(one.offset_x(), "0");
        let three = Stack {
            scale: vec![Track {
                keys: vec![
                    sk(1_000_000, 1000),
                    sk(3_000_000, 2000),
                    sk(5_000_000, 2000),
                ],
                offset: Micros(500_000),
            }],
            moved: vec![],
        };
        assert_eq!(
            three.scale_factor(),
            "if(lt(t,2.5),(1+(1)*max(0,t-0.5)/2),2)"
        );
        assert_eq!(Stack::default().scale_factor(), "1");
        // an eased segment is a smoothstep of the part done; the straight ones are unchanged
        let mut eased = three.clone();
        eased.scale[0].keys[0].ease = crate::keyframes::Ease::EaseInOut;
        assert_eq!(
            eased.scale_factor(),
            "if(lt(t,2.5),(1+(1)*(max(0,t-0.5)/2)*(max(0,t-0.5)/2)*(3-2*(max(0,t-0.5)/2))),2)"
        );
        assert_eq!(num(-0.25), "(-0.25)");
        assert_eq!(num(f64::NAN), "0");
    }

    // covers: REQ-131
    #[test]
    fn expressions_use_only_numbers_and_known_words() {
        let mut seed = 12345u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            seed >> 33
        };
        for _ in 0..200 {
            let n = 1 + next() % 6;
            let mut keys = Vec::new();
            for i in 0..n {
                keys.push(Key {
                    t: Micros((i * 1_000_000 + next() % 900_000) as i64),
                    v: Scale {
                        permille: 100 + (next() % 3900) as u16,
                        dx: (next() % 20000) as i16 - 10000,
                        dy: (next() % 20000) as i16 - 10000,
                    },
                    ease: Default::default(),
                });
            }
            let st = Stack {
                scale: vec![Track {
                    keys,
                    offset: Micros((next() % 5_000_000) as i64),
                }],
                moved: vec![],
            };
            for e in [st.scale_factor(), st.offset_x(), st.offset_y()] {
                let words = e.replace("max", "").replace("if", "").replace("lt", "");
                assert!(
                    words.chars().all(|c| "0123456789.,()+-*/t".contains(c)),
                    "{e}"
                );
                assert_eq!(e.matches('(').count(), e.matches(')').count());
            }
        }
    }

    #[test]
    fn scale_clamps_and_blends() {
        let mut s = Scale {
            permille: 5,
            dx: i16::MAX,
            dy: i16::MIN,
        };
        assert!(s.clamp());
        assert_eq!((s.permille, s.dx, s.dy), (100, 20000, -20000));
        assert!(!s.clamp());
        let m = Scale::lerp(
            Scale::NORMAL,
            Scale {
                permille: 2000,
                dx: 100,
                dy: 0,
            },
            1,
            2,
        );
        assert_eq!((m.permille, m.dx), (1500, 50));
        assert_eq!(
            Pos::lerp(CENTRE, Pos { x: 0, y: 10000 }, 1, 2),
            Pos { x: 2500, y: 7500 }
        );
    }

    // covers: REQ-285
    #[test]
    fn a_custom_ratio_from_one_to_four_to_four_to_one_is_a_shape() {
        assert_eq!(Shape::custom(1, 1), Some(Shape::Custom(1, 1)));
        assert_eq!(Shape::custom(1, 4), Some(Shape::Custom(1, 4)));
        assert_eq!(Shape::custom(4, 1), Some(Shape::Custom(4, 1)));
        assert_eq!(
            Shape::custom(16, 9),
            Some(Shape::Wide16x9),
            "a preset stays the preset"
        );
        assert_eq!(Shape::custom(1, 5), None);
        assert_eq!(Shape::custom(5, 1), None);
        assert_eq!(Shape::custom(0, 3), None);
        assert_eq!(Shape::custom(101, 100), None);
        // 1:1 is a square frame on a 1920x1080 video
        let a = FrameAspect {
            shape: Shape::Custom(1, 1),
            ..Default::default()
        };
        let (w, h) = frame_size((1920, 1080), Some(&a));
        assert_eq!(w, h);
        assert_eq!(a.shape.name(), "1:1");
        // written as "w:h" and read back; a bad one is refused when a project is read
        let text = serde_json::to_string(&a).unwrap();
        assert!(text.contains("\"1:1\""), "{text}");
        assert_eq!(serde_json::from_str::<FrameAspect>(&text).unwrap(), a);
        assert!(serde_json::from_str::<Shape>("\"1:5\"").is_err());
        assert!(serde_json::from_str::<Shape>("\"x\"").is_err());
        assert_eq!(
            serde_json::from_str::<Shape>("\"4:3\"").unwrap(),
            Shape::Wide4x3
        );
    }

    // covers: REQ-284
    #[test]
    fn the_blur_levels_have_growing_radius_and_medium_is_not_written() {
        let r: Vec<u32> = BlurLevel::ALL.iter().map(|b| b.radius()).collect();
        assert!(r[0] < r[1] && r[1] < r[2], "{r:?}");
        assert_eq!(BlurLevel::default(), BlurLevel::Medium);
        assert_eq!(
            BlurLevel::Medium.radius(),
            BLUR_RADIUS,
            "the blur of every earlier version"
        );
        let plain = serde_json::to_string(&FrameAspect::default()).unwrap();
        assert!(!plain.contains("blur"), "{plain}");
        let high = FrameAspect {
            blur: BlurLevel::High,
            ..Default::default()
        };
        let back: FrameAspect =
            serde_json::from_str(&serde_json::to_string(&high).unwrap()).unwrap();
        assert_eq!(back.blur, BlurLevel::High);
        // a project written before the levels has Medium
        let old: FrameAspect =
            serde_json::from_str(r##"{"shape":"16:9","fit":"width","fill":{"colour":"#000000"}}"##)
                .unwrap();
        assert_eq!(old.blur, BlurLevel::Medium);
    }
}
