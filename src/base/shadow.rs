// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The shadow of a text or a subtitle (2.1.0): a leaf module shared by `text` and `subtitles`.

use serde::{Deserialize, Serialize};

/// Largest shadow distance, in pixels at a picture height of 1080.
pub const SHADOW_MAX: u8 = 20;

/// A shadow behind letters or a box (2.1.0, REQ-306, REQ-307): `px` pixels to the right and
/// down at a picture height of 1080 (scaled with the height like the text size), in `colour`.
/// 0 is no shadow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Shadow {
    #[serde(default)]
    pub px: u8,
    #[serde(default)]
    pub colour: [u8; 3],
}

impl Shadow {
    pub fn is_none(&self) -> bool {
        self.px == 0
    }

    /// The same shadow with a distance in `0..=SHADOW_MAX`.
    pub fn clamped(self) -> Shadow {
        Shadow {
            px: self.px.min(SHADOW_MAX),
            ..self
        }
    }
}
