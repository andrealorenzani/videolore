// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Timeline zoom: pixels per second, from "fit everything" to frame level.

/// Pixels per frame at the maximum zoom.
pub const PX_PER_FRAME_MAX: f32 = 12.0;

/// `(fit, max)` zoom in px/s for a timeline of `duration_s` seconds shown `width` px wide.
pub fn range(duration_s: f64, width: f32, fps: f64) -> (f32, f32) {
    let duration = duration_s.max(1.0) as f32;
    let fit = (width.max(10.0) / duration).max(0.001);
    let max = (PX_PER_FRAME_MAX * fps as f32).max(fit);
    (fit, max)
}

/// Clamps `z` into the range; 0 (unset) means "fit".
pub fn clamp(z: f32, (fit, max): (f32, f32)) -> f32 {
    if z <= 0.0 || !z.is_finite() {
        fit
    } else {
        z.clamp(fit, max)
    }
}

/// Logarithmic slider position (0..1) of zoom `z`.
pub fn to_slider(z: f32, (fit, max): (f32, f32)) -> f32 {
    if max <= fit {
        return 0.0;
    }
    ((clamp(z, (fit, max)) / fit).ln() / (max / fit).ln()).clamp(0.0, 1.0)
}

/// Zoom for slider position `s` (0 = fit, 1 = frame level).
pub fn from_slider(s: f32, (fit, max): (f32, f32)) -> f32 {
    fit * (max / fit).powf(s.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-012
    #[test]
    fn logarithmic_from_fit_to_frames() {
        let r = range(60.0, 900.0, 30.0);
        assert_eq!(r, (15.0, 360.0));
        assert_eq!(from_slider(0.0, r), 15.0);
        assert!((from_slider(1.0, r) - 360.0).abs() < 1e-3);
        // logarithmic: the midpoint is the geometric mean
        assert!((from_slider(0.5, r) - (15.0f32 * 360.0).sqrt()).abs() < 1e-2);
        assert!((to_slider(from_slider(0.3, r), r) - 0.3).abs() < 1e-4);
        assert_eq!(clamp(0.0, r), 15.0);
        assert_eq!(clamp(1e9, r), 360.0);
        assert_eq!(clamp(f32::NAN, r), 15.0);
        // at max zoom one frame is 12 px wide
        assert!((r.1 / 30.0 - PX_PER_FRAME_MAX).abs() < 1e-6);
        let tiny = range(0.0, 0.0, 1.0);
        assert_eq!(to_slider(5.0, (tiny.0, tiny.0)), 0.0);
    }
}
