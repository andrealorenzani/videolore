// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Take screenshot (3.2.0, REQ-376, ADR-282): the picture as it is on the screen, cut out
//! of a capture of the window and saved as a PNG file the user chooses. Pure except
//! [`save`].

use eframe::egui::{ColorImage, Rect};
use image::ImageEncoder;
use std::path::{Path, PathBuf};
use videolore::time::{Micros, timecode};

/// A picture: `width * height` RGBA pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The part of the capture `window` that shows the picture drawn at `picture` (points, with
/// `pixels_per_point` pixels in each); `None` when none of it is in the capture.
pub fn crop(window: &ColorImage, picture: Rect, pixels_per_point: f32) -> Option<Shot> {
    let [w, h] = window.size;
    if window.pixels.len() != w.checked_mul(h)? {
        return None;
    }
    // a float that is not a number or is negative becomes 0
    let edge = |v: f32, max: usize| ((v * pixels_per_point).round() as usize).min(max);
    let (x0, x1) = (edge(picture.min.x, w), edge(picture.max.x, w));
    let (y0, y1) = (edge(picture.min.y, h), edge(picture.max.y, h));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut rgba = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for y in y0..y1 {
        for p in &window.pixels[y * w + x0..y * w + x1] {
            // the window is opaque: its colours are kept as they are
            rgba.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
        }
    }
    Some(Shot {
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
        rgba,
    })
}

/// The proposed name: the video's name and the time of the frame, with dots (a colon is
/// not allowed in a file name on Windows), e.g. `holiday 00.01.30.500.png`.
pub fn file_name(video: &Path, at: Micros) -> String {
    let stem = video
        .file_stem()
        .map(|s| videolore::media::sanitize_for_display(&s.to_string_lossy()))
        .unwrap_or_else(|| "screenshot".into());
    // room for the time and the extension in a 255-byte name
    let stem: String = stem.chars().take(60).collect();
    format!("{stem} {}.png", timecode(at).replace(':', "."))
}

/// The folder the save dialog opens in: Pictures, else the home folder.
pub fn pictures_folder() -> Option<PathBuf> {
    let dirs = directories::UserDirs::new()?;
    Some(
        dirs.picture_dir()
            .unwrap_or_else(|| dirs.home_dir())
            .to_path_buf(),
    )
}

/// The path written for the chosen one: always a `.png` file, so a video can never be
/// replaced.
pub fn png_path(chosen: &Path) -> PathBuf {
    let is_png = chosen
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    if is_png {
        return chosen.to_path_buf();
    }
    let mut name = chosen.as_os_str().to_os_string();
    name.push(".png");
    PathBuf::from(name)
}

/// The PNG bytes of `shot`.
pub fn encode(shot: &Shot) -> Result<Vec<u8>, String> {
    let expected = (shot.width as usize)
        .checked_mul(shot.height as usize)
        .and_then(|n| n.checked_mul(4));
    if shot.width == 0 || shot.height == 0 || expected != Some(shot.rgba.len()) {
        return Err("the picture is damaged".into());
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            &shot.rgba,
            shot.width,
            shot.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Writes `shot` to the PNG file at `path` (all or nothing).
pub fn save(shot: &Shot, path: &Path) -> Result<(), String> {
    let bytes = encode(shot)?;
    videolore::base::io::write_atomic(path, &bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Color32, pos2};

    /// A 6 x 4 capture whose pixels tell where they are.
    fn window() -> ColorImage {
        ColorImage::new(
            [6, 4],
            (0..24)
                .map(|i| Color32::from_rgb((i % 6) as u8, (i / 6) as u8, 9))
                .collect(),
        )
    }

    // covers: REQ-376
    #[test]
    fn the_picture_is_cut_out_of_the_capture_at_its_place() {
        // two pixels a point: points (0.5, 0.5)..(2, 1.5) are pixels (1, 1)..(4, 3)
        let r = Rect::from_min_max(pos2(0.5, 0.5), pos2(2.0, 1.5));
        let s = crop(&window(), r, 2.0).unwrap();
        assert_eq!((s.width, s.height), (3, 2));
        assert_eq!(&s.rgba[..4], [1, 1, 9, 255]);
        assert_eq!(&s.rgba[s.rgba.len() - 4..], [3, 2, 9, 255]);
        // cut at the edges of the capture
        let all = Rect::from_min_max(pos2(-10.0, -10.0), pos2(100.0, 100.0));
        let s = crop(&window(), all, 1.0).unwrap();
        assert_eq!((s.width, s.height), (6, 4));
        let outside = Rect::from_min_max(pos2(50.0, 0.0), pos2(60.0, 2.0));
        assert_eq!(crop(&window(), outside, 1.0), None);
        assert_eq!(crop(&window(), all, f32::NAN), None);
        let mut damaged = window();
        damaged.pixels.pop();
        assert_eq!(crop(&damaged, all, 1.0), None);
    }

    // covers: REQ-376
    #[test]
    fn the_name_holds_the_video_and_the_time_and_is_always_a_png() {
        assert_eq!(
            file_name(Path::new("/v/holiday.mp4"), Micros(3_723_004_000)),
            "holiday 01.02.03.004.png"
        );
        let long = format!("/v/{}.mp4", "é".repeat(120));
        assert!(file_name(Path::new(&long), Micros::ZERO).len() < 255);
        assert!(file_name(Path::new("/"), Micros::ZERO).starts_with("screenshot "));
        assert_eq!(png_path(Path::new("/p/a.PNG")), Path::new("/p/a.PNG"));
        assert_eq!(png_path(Path::new("/p/a.mp4")), Path::new("/p/a.mp4.png"));
        assert_eq!(png_path(Path::new("/p/a")), Path::new("/p/a.png"));
    }

    // covers: REQ-376
    #[test]
    fn a_saved_picture_reads_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let shot = crop(
            &window(),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(6.0, 4.0)),
            1.0,
        )
        .unwrap();
        let path = dir.path().join("s.png");
        save(&shot, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let back = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        assert_eq!((back.width(), back.height()), (6, 4));
        assert_eq!(back.into_raw(), shot.rgba);
        let bad = Shot {
            width: 3,
            height: 3,
            rgba: vec![0; 4],
        };
        assert!(save(&bad, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "left as it was");
    }
}
