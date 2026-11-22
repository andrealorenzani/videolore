// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The effects of the Effects section and what they are applied to. Every fact of an effect
//! that is not behaviour (its name, its undo name, its popup, whether it is a frame effect)
//! is one row of [`SPECS`]; a new effect is one variant, one row, one icon
//! (`ui::effects::effect_icon`) and one arm of `AppState::apply_effect`.

use super::Popup;
use crate::timeline::ClipId;

/// An effect of the Effects section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    Fade,
    Subtitles,
    Volume,
    /// Reduces the background noise of the video's sound (REQ-114).
    CleanAudio,
    /// The shape of the whole video's frame and what fills the empty space (REQ-124).
    ChangeAspect,
    /// Zooms the picture, with keyframes (REQ-127, REQ-129).
    Scale,
    /// Moves the picture, with keyframes (REQ-128, REQ-129).
    Move,
    /// Changes the colours of the picture with a LUT file (1.6.0, REQ-135).
    ApplyLut,
    /// The speed of one clip or of all of them, not keyframed (2.3.0, REQ-281).
    ChangeSpeed,
}

/// The fixed facts of one effect.
#[derive(Debug, Clone, Copy)]
pub struct EffectSpec {
    pub kind: EffectKind,
    /// The name in the Effects section and the menus.
    pub name: &'static str,
    /// The name of the undo step that applying it makes.
    pub undo: &'static str,
    /// The popup it opens (`Popup::Instance`: the popup of the applied instance).
    pub popup: Popup,
    /// An effect of the picture frame (1.5.0).
    pub frame: bool,
}

const fn spec(
    kind: EffectKind,
    name: &'static str,
    undo: &'static str,
    popup: Popup,
    frame: bool,
) -> EffectSpec {
    EffectSpec {
        kind,
        name,
        undo,
        popup,
        frame,
    }
}

/// One row per effect, in the order of the variants and of the Effects section.
pub const SPECS: [EffectSpec; 9] = {
    use EffectKind as E;
    [
        spec(E::Fade, "Fade IN/OUT", "Fade IN/OUT", Popup::Fades, false),
        spec(
            E::Subtitles,
            "Subtitles",
            "Create subtitles",
            Popup::Subtitles,
            false,
        ),
        spec(E::Volume, "Volume", "Volume", Popup::Volume, false),
        spec(
            E::CleanAudio,
            "Clean audio",
            "Clean audio",
            Popup::CleanAudio,
            false,
        ),
        spec(
            E::ChangeAspect,
            "Change aspect",
            "Change aspect",
            Popup::Instance,
            true,
        ),
        spec(E::Scale, "Scale", "Scale", Popup::Instance, true),
        spec(E::Move, "Move", "Move", Popup::Instance, true),
        spec(E::ApplyLut, "Apply LUT", "Apply LUT", Popup::Instance, true),
        spec(
            E::ChangeSpeed,
            "Change speed",
            "Change speed",
            Popup::Instance,
            false,
        ),
    ]
};

impl EffectKind {
    /// Every effect, in the order of the Effects section.
    pub const ALL: [EffectKind; 9] = {
        let mut all = [EffectKind::Fade; 9];
        let mut i = 0;
        while i < SPECS.len() {
            all[i] = SPECS[i].kind;
            i += 1;
        }
        all
    };

    /// The row of the effect.
    pub fn spec(self) -> &'static EffectSpec {
        &SPECS[self as usize]
    }

    /// True for the effects of the picture frame (1.5.0).
    pub fn is_frame(self) -> bool {
        self.spec().frame
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    /// The name of the undo step that applying the effect makes.
    pub fn undo_name(self) -> &'static str {
        self.spec().undo
    }

    /// The popup of the effect.
    pub fn popup(self) -> Popup {
        self.spec().popup
    }
}

/// What an effect is applied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// One video clip.
    Clip(ClipId),
    /// The selected clip (or the one under the playhead).
    Selected,
    /// The whole video: one clip from the start of the timeline to its end.
    WholeVideo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_at_the_index_of_its_effect() {
        for (i, s) in SPECS.iter().enumerate() {
            assert_eq!(s.kind as usize, i, "{:?}", s.kind);
            assert_eq!(EffectKind::ALL[i], s.kind);
            assert_eq!(s.kind.spec().name, s.name);
        }
    }

    #[test]
    fn frame_effects_open_the_instance_popup() {
        for e in EffectKind::ALL.into_iter().filter(|e| e.is_frame()) {
            assert_eq!(e.popup(), Popup::Instance, "{e:?}");
        }
        assert_eq!(EffectKind::Subtitles.undo_name(), "Create subtitles");
        assert_eq!(EffectKind::Fade.popup(), Popup::Fades);
    }
}
