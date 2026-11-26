// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The screens of the computer (REQ-344, ADR-258): the numbering and the conversion to
//! physical pixels are pure; the list itself comes from `display-info`.

use crate::plan::{Capture, Os};

/// One connected screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// The system's identifier: the choice follows it when the list changes.
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Screen {
    /// The capture of this screen, found at `index` of the list.
    pub fn capture(&self, index: usize) -> Capture {
        Capture {
            index,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

/// "Screen 2, 1920x1080" for the screen at `index` (from 0) of the list.
pub fn label(index: usize, s: &Screen) -> String {
    format!("Screen {}, {}x{}", index + 1, s.width, s.height)
}

/// A screen as `display-info` reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reported {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

/// The screen in physical pixels: `display-info` divides by the scale factor on Linux
/// and reports points on macOS; Windows reports pixels already. Rounding never goes past
/// the real size, so that a capture stays inside the screen.
pub fn physical(os: Os, r: Reported) -> Screen {
    let scale = match os {
        Os::Windows => 1.0,
        _ if r.scale.is_finite() && r.scale > 0.0 => r.scale,
        _ => 1.0,
    };
    let at = |v: f32| (v * scale).round();
    Screen {
        id: r.id,
        x: at(r.x as f32) as i32,
        y: at(r.y as f32) as i32,
        width: at(r.width as f32) as u32,
        height: at(r.height as f32) as u32,
    }
}

/// Where the list of screens comes from (faked in tests).
pub trait ScreenSource {
    /// The connected screens in the system's order; empty when they cannot be read.
    fn list(&mut self) -> Vec<Screen>;
}

/// The screens of this machine.
#[derive(Debug, Clone, Copy)]
pub struct SystemScreens(pub Os);

impl ScreenSource for SystemScreens {
    fn list(&mut self) -> Vec<Screen> {
        display_info::DisplayInfo::all()
            .unwrap_or_default()
            .into_iter()
            .map(|d| {
                physical(
                    self.0,
                    Reported {
                        id: d.id,
                        x: d.x,
                        y: d.y,
                        width: d.width,
                        height: d.height,
                        scale: d.scale_factor,
                    },
                )
            })
            .filter(|s| s.width > 0 && s.height > 0)
            .collect()
    }
}
