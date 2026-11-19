// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The splash screen (1.6.0, REQ-133, REQ-134): pure parts. The image and the summary of
//! the version are compiled in, so a release without them does not build. The window is
//! drawn by `ui::splash_ui`.
//!
//! * [`layout`] sizes the image and the summary panel (scaled together to fit the screen);
//! * [`fit_text`] finds the largest size at which "Version X.Y" fits its rectangle;
//! * [`text_colour`] picks a readable colour for it over the image;
//! * [`decode`] reads the GIF (every frame is kept; the image shipped today has one).

use std::io::Cursor;

/// The release image of this version (`assets/splash/<version>.gif`).
pub const IMAGE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/splash/",
    env!("CARGO_PKG_VERSION"),
    ".gif"
));

/// The summary of the latest change of this version (`assets/splash/<version>.txt`): the
/// first line is the heading, the others one line each.
pub const SUMMARY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/splash/",
    env!("CARGO_PKG_VERSION"),
    ".txt"
));

/// Width of the summary panel, in image pixels.
pub const POPUP_WIDTH: f32 = 400.0;
/// The rectangle of the version text in the image: x from 74 to 408, y from 381 to 465.
pub const TEXT_RECT: (f32, f32, f32, f32) = (74.0, 381.0, 408.0, 465.0);
/// Space kept free on every side of the text inside its rectangle, so it touches no edge.
pub const TEXT_MARGIN: f32 = 6.0;
/// Frames kept at most, and the longest side accepted (a damaged or hostile file).
pub const MAX_FRAMES: usize = 64;
pub const MAX_SIDE: u32 = 4096;

/// "Version X.Y" from `X.Y.Z` (the major and minor numbers).
pub fn version_text(version: &str) -> String {
    let mut parts = version.split('.');
    match (parts.next(), parts.next()) {
        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => format!("Version {a}.{b}"),
        _ => format!("Version {version}"),
    }
}

/// "Version X.Y" of this build.
pub fn current_version_text() -> String {
    version_text(env!("CARGO_PKG_VERSION"))
}

/// Sizes of the two parts of the splash on a screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// 1.0 unless the two parts do not fit the screen.
    pub scale: f32,
    pub image: (f32, f32),
    /// The summary panel: [`POPUP_WIDTH`] wide and as high as the image, both scaled.
    pub popup: (f32, f32),
}

impl Layout {
    /// Both parts side by side.
    pub fn total(&self) -> (f32, f32) {
        (self.image.0 + self.popup.0, self.image.1)
    }
}

/// The layout of an image of `img` pixels on an area of `avail` pixels: both parts are
/// scaled together, keeping proportions, when they do not fit (never enlarged).
pub fn layout(img: (u32, u32), avail: (f32, f32)) -> Layout {
    let (w, h) = (img.0.max(1) as f32, img.1.max(1) as f32);
    let fit = (avail.0 / (w + POPUP_WIDTH)).min(avail.1 / h);
    let scale = if fit.is_finite() {
        fit.clamp(0.05, 1.0)
    } else {
        1.0
    };
    Layout {
        scale,
        image: (w * scale, h * scale),
        popup: (POPUP_WIDTH * scale, h * scale),
    }
}

/// The largest size, at most `max`, at which `measure(size)` (width, height) fits inside
/// `rect` shrunk by [`TEXT_MARGIN`] on every side. At least 1.
pub fn fit_text(measure: impl Fn(f32) -> (f32, f32), rect: (f32, f32), max: f32) -> f32 {
    let room = (
        (rect.0 - 2.0 * TEXT_MARGIN).max(1.0),
        (rect.1 - 2.0 * TEXT_MARGIN).max(1.0),
    );
    let fits = |s: f32| {
        let (w, h) = measure(s);
        w <= room.0 && h <= room.1
    };
    let (mut lo, mut hi) = (1.0_f32, max.max(1.0));
    if fits(hi) {
        return hi;
    }
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Mean colour (red, green, blue) of the pixels of `rect` in an RGBA picture of `size`.
pub fn mean_colour(rgba: &[u8], size: (u32, u32), rect: (f32, f32, f32, f32)) -> [u8; 3] {
    let (w, h) = (size.0 as usize, size.1 as usize);
    let x0 = (rect.0.max(0.0) as usize).min(w);
    let x1 = (rect.2.max(0.0) as usize).min(w);
    let y0 = (rect.1.max(0.0) as usize).min(h);
    let y1 = (rect.3.max(0.0) as usize).min(h);
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * w + x) * 4;
            if let Some(px) = rgba.get(i..i + 3) {
                for c in 0..3 {
                    sum[c] += u64::from(px[c]);
                }
                n += 1;
            }
        }
    }
    if n == 0 {
        return [0, 0, 0];
    }
    [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
}

/// White over a dark area, near-black over a light one (by luminance).
pub fn text_colour(mean: [u8; 3]) -> [u8; 3] {
    let lum =
        0.2126 * f32::from(mean[0]) + 0.7152 * f32::from(mean[1]) + 0.0722 * f32::from(mean[2]);
    if lum > 140.0 {
        [20, 20, 20]
    } else {
        [255, 255, 255]
    }
}

/// One picture of the image.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub rgba: Vec<u8>,
    /// How long it shows, milliseconds (at least 20).
    pub delay_ms: u32,
}

/// The decoded image.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub size: (u32, u32),
    pub frames: Vec<Frame>,
}

/// Decodes a GIF: every frame (up to [`MAX_FRAMES`]) as RGBA. A file that is not a GIF, is
/// empty or has frames of other sizes is an error.
pub fn decode(bytes: &[u8]) -> Result<Decoded, String> {
    use image::AnimationDecoder;
    let dec = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut frames = Vec::new();
    let mut size = (0, 0);
    for f in dec.into_frames().take(MAX_FRAMES) {
        let f = f.map_err(|e| e.to_string())?;
        let (num, den) = f.delay().numer_denom_ms();
        let delay_ms = (num / den.max(1)).max(20);
        let buf = f.into_buffer();
        let s = (buf.width(), buf.height());
        if s.0 == 0 || s.1 == 0 || s.0 > MAX_SIDE || s.1 > MAX_SIDE {
            return Err("the image has an unusable size".into());
        }
        if size == (0, 0) {
            size = s;
        } else if size != s {
            return Err("the frames have different sizes".into());
        }
        frames.push(Frame {
            rgba: buf.into_raw(),
            delay_ms,
        });
    }
    if frames.is_empty() {
        return Err("the image has no pictures".into());
    }
    Ok(Decoded { size, frames })
}

/// The lines of the summary: heading and items (empty lines dropped).
pub fn summary_lines() -> Vec<&'static str> {
    SUMMARY
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-134
    #[test]
    fn the_version_text_has_the_major_and_minor_numbers() {
        assert_eq!(version_text("1.6.0"), "Version 1.6");
        assert_eq!(version_text("12.0.3"), "Version 12.0");
        assert_eq!(version_text("2"), "Version 2");
        assert_eq!(version_text(""), "Version ");
        assert_eq!(
            current_version_text(),
            version_text(env!("CARGO_PKG_VERSION"))
        );
    }

    // covers: REQ-134
    #[test]
    fn the_shipped_image_decodes_at_its_own_size_and_the_summary_exists() {
        let d = decode(IMAGE).unwrap();
        assert_eq!(d.size, (486, 506));
        assert_eq!(d.frames.len(), 1, "a still image today");
        assert_eq!(d.frames[0].rgba.len(), 486 * 506 * 4);
        // the rectangle of the text is inside the image
        assert!(TEXT_RECT.2 <= 486.0 && TEXT_RECT.3 <= 506.0);
        assert_eq!(
            (
                (TEXT_RECT.2 - TEXT_RECT.0) as u32,
                (TEXT_RECT.3 - TEXT_RECT.1) as u32
            ),
            (334, 84)
        );
        // the summary written for this version: a heading and at most 6 lines
        let lines = summary_lines();
        assert!(lines.len() >= 2 && lines.len() <= 6, "{lines:?}");
        assert!(SUMMARY.contains("Apply LUT"));
        // the colour rule gives a readable colour over the real picture
        let m = mean_colour(&d.frames[0].rgba, d.size, TEXT_RECT);
        let c = text_colour(m);
        assert!(c == [255, 255, 255] || c == [20, 20, 20]);
    }

    // covers: REQ-134
    #[test]
    fn the_layout_is_400_wide_and_as_high_as_the_image_and_scales_together_when_small() {
        let l = layout((486, 506), (1600.0, 900.0));
        assert_eq!(l.scale, 1.0);
        assert_eq!(l.image, (486.0, 506.0));
        assert_eq!(l.popup, (400.0, 506.0));
        assert_eq!(l.total(), (886.0, 506.0));
        // a small screen: both parts scaled by the same factor, proportions kept
        let s = layout((486, 506), (800.0, 400.0));
        assert!(s.scale < 0.8);
        assert!((s.popup.0 - 400.0 * s.scale).abs() < 0.01);
        assert_eq!(s.popup.1, s.image.1);
        assert!(s.total().0 <= 800.0 + 0.01 && s.total().1 <= 400.0 + 0.01);
        assert!((s.image.0 / s.image.1 - 486.0 / 506.0).abs() < 0.001);
        // never enlarged, never zero
        assert_eq!(layout((100, 100), (5000.0, 5000.0)).scale, 1.0);
        assert!(layout((486, 506), (1.0, 1.0)).scale > 0.0);
        assert!(layout((0, 0), (500.0, 500.0)).image.0 > 0.0);
    }

    // covers: REQ-134
    #[test]
    fn the_text_is_the_largest_size_that_fits_inside_its_rectangle() {
        // a fake font: each character is 0.6 em wide and 1.2 em high
        let measure = |s: f32| (0.6 * s * 11.0, 1.2 * s);
        let rect = (334.0, 84.0);
        let size = fit_text(measure, rect, 200.0);
        let (w, h) = measure(size);
        assert!(
            w <= rect.0 - 2.0 * TEXT_MARGIN && h <= rect.1 - 2.0 * TEXT_MARGIN,
            "{w} {h}"
        );
        // the next size up would not fit
        let (w2, h2) = measure(size + 0.5);
        assert!(w2 > rect.0 - 2.0 * TEXT_MARGIN || h2 > rect.1 - 2.0 * TEXT_MARGIN);
        // a short text is limited by the maximum, an impossible one by 1
        assert_eq!(fit_text(|s| (s, s), rect, 20.0), 20.0);
        assert_eq!(fit_text(|_| (1e6, 1e6), rect, 20.0), 1.0);
    }

    // covers: REQ-134
    #[test]
    fn colours_and_damaged_images() {
        assert_eq!(text_colour([0, 0, 0]), [255, 255, 255]);
        assert_eq!(text_colour([250, 250, 250]), [20, 20, 20]);
        let px = [200, 100, 0, 255, 100, 50, 0, 255];
        assert_eq!(mean_colour(&px, (2, 1), (0.0, 0.0, 2.0, 1.0)), [150, 75, 0]);
        assert_eq!(mean_colour(&px, (2, 1), (5.0, 5.0, 9.0, 9.0)), [0, 0, 0]);
        assert_eq!(mean_colour(&[], (2, 1), (0.0, 0.0, 2.0, 1.0)), [0, 0, 0]);
        assert!(decode(b"not a gif").is_err());
        assert!(decode(&[]).is_err());
        assert!(decode(&IMAGE[..IMAGE.len() / 2]).is_err() || decode(&IMAGE[..40]).is_err());
    }
}
