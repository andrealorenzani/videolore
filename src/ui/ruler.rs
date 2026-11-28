// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The labels of the timeline ruler (REQ-093). The format follows the time at the right
//! edge of the visible part: seconds up to 90 s ("45s"), minutes and seconds up to 90
//! minutes ("01:40", "75:30"), then hours ("01:05:00"). Ticks sit on round steps chosen so
//! that the widest label always fits between two ticks. Under the labels, small marks follow
//! the zoom: one per frame, second, minute, 10 minutes or hour (REQ-367, ADR-276). Pure:
//! tested without a window.

pub use crate::base::steps::{STEPS, step};

/// How the ruler writes times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Seconds,
    MinSec,
    HourMinSec,
}

/// Longest visible time written in seconds.
pub const SECONDS_UP_TO: f64 = 90.0;
/// Longest visible time written as minutes and seconds.
pub const MINUTES_UP_TO: f64 = 90.0 * 60.0;

/// Space added to the widest label so that two labels never touch.
pub const LABEL_GAP: f32 = 8.0;

/// The format for a visible range ending at `t_right` seconds.
pub fn mode(t_right: f64) -> Mode {
    if t_right <= SECONDS_UP_TO {
        Mode::Seconds
    } else if t_right <= MINUTES_UP_TO {
        Mode::MinSec
    } else {
        Mode::HourMinSec
    }
}

/// The label of whole second `t`.
pub fn label(t: u64, mode: Mode) -> String {
    match mode {
        Mode::Seconds => format!("{t}s"),
        // minutes are not wrapped into hours (up to 90:00)
        Mode::MinSec => format!("{:02}:{:02}", t / 60, t % 60),
        Mode::HourMinSec => format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60),
    }
}

/// Ticks of the visible range `[t0, t1]` (seconds) at `zoom` pixels per second, with
/// `char_w` the width of one label character: whole multiples of the first step whose
/// spacing fits the widest label. Returns the times and the format.
pub fn ticks(t0: f64, t1: f64, zoom: f32, char_w: f32) -> (Vec<u64>, Mode) {
    let t0 = if t0.is_finite() { t0.max(0.0) } else { 0.0 };
    let t1 = if t1.is_finite() { t1.max(t0) } else { t0 };
    let m = mode(t1);
    if !(zoom.is_finite() && zoom > 0.0) {
        return (Vec::new(), m);
    }
    let widest = label(t1.ceil() as u64, m).chars().count() as f32 * char_w + LABEL_GAP;
    let mut n = 0;
    while (step(n) as f32) * zoom < widest && n < 64 {
        n += 1;
    }
    let s = step(n).max(1);
    let first = (t0 / s as f64).ceil() as u64;
    let last = (t1 / s as f64).floor() as u64;
    // bounded by the visible width / label width: never more than a few hundred
    let out = (first..=last).take(10_000).map(|k| k * s).collect();
    (out, m)
}

/// Smallest space between two marks, in pixels.
pub const MARK_GAP: f32 = 6.0;
/// Most marks of one frame of the window.
const MAX_MARKS: usize = 10_000;

/// What one mark of the ruler stands for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkStep {
    Frame,
    Second,
    Minute,
    TenMinutes,
    /// One hour, or that many hours when even hours would be too close.
    Hours(u64),
}

impl MarkStep {
    /// The step in seconds at `fps` frames per second.
    pub fn seconds(self, fps: f64) -> f64 {
        match self {
            MarkStep::Frame => 1.0 / fps,
            MarkStep::Second => 1.0,
            MarkStep::Minute => 60.0,
            MarkStep::TenMinutes => 600.0,
            MarkStep::Hours(n) => 3600.0 * n as f64,
        }
    }
}

/// The step of the marks at `zoom` pixels per second: the smallest one whose marks are at
/// least [`MARK_GAP`] apart.
pub fn mark_step(zoom: f32, fps: f64) -> MarkStep {
    let fps = if fps.is_finite() && fps > 0.0 {
        fps
    } else {
        30.0
    };
    let fits = |s: MarkStep| s.seconds(fps) * f64::from(zoom) >= f64::from(MARK_GAP);
    let fixed = [
        MarkStep::Frame,
        MarkStep::Second,
        MarkStep::Minute,
        MarkStep::TenMinutes,
    ];
    if let Some(s) = fixed.into_iter().find(|s| fits(*s)) {
        return s;
    }
    let mut hours = 1u64;
    while !fits(MarkStep::Hours(hours)) && hours < 1 << 32 {
        hours *= 2;
    }
    MarkStep::Hours(hours)
}

/// The marks of the visible range `[t0, t1]` (seconds) and their step.
pub fn marks(t0: f64, t1: f64, zoom: f32, fps: f64) -> (Vec<f64>, MarkStep) {
    let step = mark_step(zoom, fps);
    if !(zoom.is_finite() && zoom > 0.0 && t0.is_finite() && t1.is_finite()) {
        return (Vec::new(), step);
    }
    let s = step.seconds(if fps.is_finite() && fps > 0.0 {
        fps
    } else {
        30.0
    });
    let first = (t0.max(0.0) / s).ceil() as u64;
    let last = (t1.max(0.0) / s).floor() as u64;
    let out = (first..=last)
        .take(MAX_MARKS)
        .map(|k| k as f64 * s)
        .collect();
    (out, step)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAR_W: f32 = 7.0;

    // covers: REQ-093
    #[test]
    fn labels_follow_the_right_edge_of_the_visible_range() {
        // (a) 0–60 s: seconds
        let (t, m) = ticks(0.0, 60.0, 15.0, CHAR_W);
        assert_eq!(m, Mode::Seconds);
        assert_eq!(label(t[1], m), format!("{}s", t[1]));
        assert_eq!(label(45, Mode::Seconds), "45s");
        assert_eq!(mode(90.0), Mode::Seconds);
        // (b) 0–5 min, or scrolled to 100–110 s: minutes and seconds
        assert_eq!(ticks(0.0, 300.0, 3.0, CHAR_W).1, Mode::MinSec);
        let (t, m) = ticks(100.0, 110.0, 80.0, CHAR_W);
        assert_eq!(m, Mode::MinSec);
        assert_eq!(label(t[0], m), "01:40");
        assert_eq!(
            label(4530, Mode::MinSec),
            "75:30",
            "minutes are not wrapped"
        );
        assert_eq!(mode(5400.0), Mode::MinSec);
        // (c) 0–2 h: hours
        let (_, m) = ticks(0.0, 7200.0, 0.2, CHAR_W);
        assert_eq!(m, Mode::HourMinSec);
        assert_eq!(label(3900, m), "01:05:00");
    }

    // covers: REQ-093
    #[test]
    fn labels_never_overlap_and_only_visible_ticks_are_made() {
        for (t0, t1, zoom) in [
            (0.0, 60.0, 400.0),
            (0.0, 600.0, 2.0),
            (3000.0, 9000.0, 0.05),
            (0.0, 360_000.0, 0.0005),
            (10.0, 12.0, 4000.0),
        ] {
            let (t, m) = ticks(t0, t1, zoom, CHAR_W);
            assert!(!t.is_empty(), "{t0}-{t1}");
            assert!(t.iter().all(|x| (*x as f64) >= t0 && (*x as f64) <= t1));
            for w in t.windows(2) {
                let gap = (w[1] - w[0]) as f32 * zoom;
                let need = label(w[1], m).chars().count() as f32 * CHAR_W;
                assert!(gap >= need, "{t0}-{t1} @ {zoom}: {gap} < {need}");
            }
        }
        // steps are round values
        assert_eq!(step(0), 1);
        assert_eq!(step(12), 3600);
        assert_eq!(step(13), 7200);
        assert_eq!(step(14), 18_000);
        assert_eq!(step(16), 72_000);
        assert!(ticks(0.0, 10.0, 0.0, CHAR_W).0.is_empty());
        assert!(ticks(f64::NAN, f64::INFINITY, 10.0, CHAR_W).0.len() <= 1);
    }

    // covers: REQ-367
    #[test]
    fn marks_follow_the_zoom_from_frames_to_hours() {
        // three zoom levels: frame, second, minute
        for (zoom, step, spacing) in [
            (360.0f32, MarkStep::Frame, 1.0 / 30.0),
            (40.0, MarkStep::Second, 1.0),
            (0.5, MarkStep::Minute, 60.0),
        ] {
            let (m, s) = marks(100.0, 400.0, zoom, 30.0);
            assert_eq!(s, step, "{zoom}");
            for w in m.windows(2) {
                assert!((w[1] - w[0] - spacing).abs() < 1e-6, "{zoom}");
            }
            assert!(m.iter().all(|t| (100.0..=400.0).contains(t)));
        }
        assert_eq!(mark_step(0.05, 30.0), MarkStep::TenMinutes);
        assert_eq!(mark_step(0.002, 30.0), MarkStep::Hours(1));
        // even hours too close: whole multiples of an hour
        assert_eq!(mark_step(0.000_5, 30.0), MarkStep::Hours(4));
        assert!(marks(0.0, 10.0, 0.0, 30.0).0.is_empty());
        assert!(marks(f64::NAN, 10.0, 10.0, 30.0).0.is_empty());
    }

    // covers: REQ-367
    #[test]
    fn marks_and_labels_never_overlap_from_the_smallest_to_the_largest_zoom() {
        for fps in [24.0, 25.0, 30.0, 60.0] {
            for duration in [1.0, 90.0, 3600.0, 36_000.0, 360_000.0] {
                let (min, max) = crate::ui::zoom::range(duration, 760.0, fps);
                for zoom in [min, (min * max).sqrt(), max] {
                    let width = 760.0 / f64::from(zoom);
                    let (m, _) = marks(0.0, width, zoom, fps);
                    for w in m.windows(2) {
                        let gap = (w[1] - w[0]) as f32 * zoom;
                        assert!(gap >= MARK_GAP - 1e-3, "{fps} {duration} @ {zoom}: {gap}");
                    }
                    let (t, mode) = ticks(0.0, width, zoom, CHAR_W);
                    for w in t.windows(2) {
                        let gap = (w[1] - w[0]) as f32 * zoom;
                        assert!(gap >= label(w[1], mode).chars().count() as f32 * CHAR_W);
                    }
                }
            }
        }
    }
}
