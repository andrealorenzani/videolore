// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Picture items (REQ-287): a PNG image with its transparency shown for a time on the
//! picture, like a text item. This module knows the image file (its size, read from the
//! PNG header without decoding it) and where the image goes inside the box of its item.

use crate::base::pos::{POS_MAX, Pos};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

/// Largest side of a picture, in pixels.
pub const MAX_SIDE: u32 = 16_384;

/// Longest file path stored for a picture.
pub const MAX_PATH: usize = 4096;

/// The image of a picture item: the file and its size in pixels.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Picture {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PictureError {
    #[error("cannot read \"{0}\"")]
    Unreadable(String),
    #[error("\"{0}\" is not a PNG image")]
    NotPng(String),
    #[error("the picture \"{0}\" is larger than 16384 pixels on a side")]
    TooLarge(String),
    #[error("The picture is larger than 16 megapixels ({0} x {1}) and was not added.")]
    TooManyPixels(u32, u32),
}

/// Most pixels (width times height) a picture item may have: 16 megapixels (REQ-316). The
/// only limit: no count, no file size, no copy in the project.
pub const MAX_PICTURE_PIXELS: u64 = 16_000_000;

/// True when `path` is a PNG that can be drawn now: a file whose header gives a size within
/// the limits. A picture file replaced by a larger one is not usable, like a missing one
/// (REQ-316, REQ-287).
pub fn usable(path: &Path) -> bool {
    read(path).is_ok()
}

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// The size of a PNG from its first 24 bytes: the signature and the header chunk.
pub fn parse_png_size(head: &[u8]) -> Option<(u32, u32)> {
    if head.len() < 24 || head[..8] != SIGNATURE || head[12..16] != *b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([head[i], head[i + 1], head[i + 2], head[i + 3]]);
    let (w, h) = (be(16), be(20));
    (w > 0 && h > 0).then_some((w, h))
}

/// Reads the picture at `path`: it must be a PNG of a reasonable size.
pub fn read(path: &Path) -> Result<Picture, PictureError> {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into(),
    );
    let mut head = Vec::with_capacity(24);
    std::fs::File::open(path)
        .and_then(|f| f.take(24).read_to_end(&mut head))
        .map_err(|_| PictureError::Unreadable(name.clone()))?;
    let (width, height) =
        parse_png_size(&head).ok_or_else(|| PictureError::NotPng(name.clone()))?;
    if u64::from(width) * u64::from(height) > MAX_PICTURE_PIXELS {
        return Err(PictureError::TooManyPixels(width, height));
    }
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(PictureError::TooLarge(name));
    }
    let path = path.to_string_lossy().into_owned();
    if path.len() > MAX_PATH {
        return Err(PictureError::Unreadable(name));
    }
    Ok(Picture {
        path,
        width,
        height,
    })
}

impl Picture {
    /// The file name, as shown on the timeline.
    pub fn name(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .map_or_else(|| self.path.clone(), |n| n.to_string_lossy().into())
    }

    /// Checks a value read from a project file.
    pub fn is_valid(&self) -> bool {
        !self.path.is_empty()
            && self.path.len() <= MAX_PATH
            && !self.path.chars().any(char::is_control)
            && (1..=MAX_SIDE).contains(&self.width)
            && (1..=MAX_SIDE).contains(&self.height)
    }

    /// The box (thousandths of the frame) a new picture gets: a third of the frame's width
    /// (or less when the picture is tall), with the picture's own proportions.
    pub fn default_box(&self, frame: (u32, u32)) -> (u16, u16) {
        let (fw, fh) = (f64::from(frame.0.max(1)), f64::from(frame.1.max(1)));
        let ratio = f64::from(self.width) / f64::from(self.height);
        // the box's height (thousandths of the frame) for its width
        let (mut w, mut h) = (300.0, 300.0 * fw / (fh * ratio));
        if h > 500.0 {
            (w, h) = (500.0 * fh * ratio / fw, 500.0);
        }
        // the caller keeps them inside the limits of a text box
        (
            w.round().clamp(1.0, 1000.0) as u16,
            h.round().clamp(1.0, 1000.0) as u16,
        )
    }

    /// Where the image is drawn on a `frame`-sized picture for an item whose box is at
    /// `place` with `width` and `height` in thousandths: the largest size that fits the box
    /// with the image's proportions, centred on it. `(x, y, w, h)` in pixels; `x` and `y`
    /// may be negative (the part outside the picture is not drawn).
    pub fn rect(
        &self,
        place: Pos,
        width: u16,
        height: u16,
        frame: (u32, u32),
    ) -> (i32, i32, u32, u32) {
        let (fw, fh) = (f64::from(frame.0), f64::from(frame.1));
        let (bw, bh) = (
            fw * f64::from(width) / 1000.0,
            fh * f64::from(height) / 1000.0,
        );
        let k = (bw / f64::from(self.width)).min(bh / f64::from(self.height));
        let even = |v: f64| (((v.round() as i64).max(2) as u32) & !1).max(2);
        let (w, h) = (
            even(f64::from(self.width) * k),
            even(f64::from(self.height) * k),
        );
        let cx = fw * f64::from(place.x.min(POS_MAX)) / f64::from(POS_MAX);
        let cy = fh * f64::from(place.y.min(POS_MAX)) / f64::from(POS_MAX);
        (
            (cx - f64::from(w) / 2.0).round() as i32,
            (cy - f64::from(h) / 2.0).round() as i32,
            w,
            h,
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The first bytes of a PNG of `w` x `h` (a header only: enough to be read).
    pub fn header(w: u32, h: u32) -> Vec<u8> {
        let mut b = SIGNATURE.to_vec();
        b.extend([0, 0, 0, 13]);
        b.extend(*b"IHDR");
        b.extend(w.to_be_bytes());
        b.extend(h.to_be_bytes());
        b
    }

    // covers: REQ-287
    #[test]
    fn the_size_comes_from_the_png_header_and_other_files_are_refused() {
        assert_eq!(parse_png_size(&header(640, 480)), Some((640, 480)));
        assert_eq!(parse_png_size(&header(0, 480)), None);
        assert_eq!(parse_png_size(b"GIF89a..................."), None);
        assert_eq!(parse_png_size(&[]), None);
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("logo.png");
        std::fs::write(&png, header(200, 100)).unwrap();
        let p = read(&png).unwrap();
        assert_eq!(
            (p.width, p.height, p.name().as_str()),
            (200, 100, "logo.png")
        );
        assert!(p.is_valid());
        let bad = dir.path().join("x.png");
        std::fs::write(&bad, b"not an image at all, really").unwrap();
        assert_eq!(read(&bad), Err(PictureError::NotPng("x.png".into())));
        assert!(matches!(
            read(&dir.path().join("none.png")),
            Err(PictureError::Unreadable(_))
        ));
        std::fs::write(&bad, header(20_000, 10)).unwrap();
        assert!(matches!(read(&bad), Err(PictureError::TooLarge(_))));
        let mut q = p.clone();
        q.width = 0;
        assert!(!q.is_valid());
        q.width = 5;
        q.path = "a\nb".into();
        assert!(!q.is_valid());
    }

    // covers: REQ-287
    // covers: REQ-316
    #[test]
    fn a_picture_of_sixteen_megapixels_is_added_and_a_larger_one_is_refused_by_name_and_size() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("big.png");
        std::fs::write(&png, header(4000, 4000)).unwrap();
        assert_eq!(read(&png).map(|p| (p.width, p.height)), Ok((4000, 4000)));
        assert!(usable(&png));
        std::fs::write(&png, header(4001, 4000)).unwrap();
        let e = read(&png).unwrap_err();
        assert_eq!(e, PictureError::TooManyPixels(4001, 4000));
        assert_eq!(
            e.to_string(),
            "The picture is larger than 16 megapixels (4001 x 4000) and was not added."
        );
        // a file replaced by a larger one, or gone, cannot be drawn (REQ-287)
        assert!(!usable(&png));
        assert!(!usable(&dir.path().join("none.png")));
        // a long thin picture within the pixel limit but past the side limit keeps its message
        std::fs::write(&png, header(20_000, 10)).unwrap();
        assert!(matches!(read(&png), Err(PictureError::TooLarge(_))));
    }

    // covers: REQ-287
    #[test]
    fn the_image_fits_its_box_keeps_its_proportions_and_is_centred_on_the_place() {
        let p = Picture {
            path: "/a.png".into(),
            width: 200,
            height: 100,
        };
        let frame = (1000, 500);
        // the default box has the picture's proportions
        let (bw, bh) = p.default_box(frame);
        let (x, y, w, h) = p.rect(Pos { x: 5000, y: 5000 }, bw, bh, frame);
        assert_eq!((w, h), (300, 150));
        assert_eq!((x + w as i32 / 2, y + h as i32 / 2), (500, 250));
        // a box that is wider than the image leaves it as high as the box
        let (_, _, w, h) = p.rect(Pos { x: 5000, y: 5000 }, 1000, 200, frame);
        assert_eq!((w, h), (200, 100));
        // a place at the edge puts part of it outside the picture
        let (x, _, w, _) = p.rect(Pos { x: 0, y: 5000 }, bw, bh, frame);
        assert_eq!(x, -(w as i32) / 2);
        // a tall picture gets a box no higher than half the frame
        let tall = Picture {
            path: "/t.png".into(),
            width: 100,
            height: 1000,
        };
        let (_, bh) = tall.default_box(frame);
        assert!(bh <= 500);
    }
}
