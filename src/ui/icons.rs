// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The icons of the interface (1.6.0, REQ-142, REQ-143): Phosphor Icons, Regular weight
//! (MIT, `assets/icons/`), bundled as a font and used as extra characters of the normal
//! font, so an icon can be written inside any text: `format!("{icon} {label}")`.
//!
//! The code points below are generated from the font's own table; the test
//! `every_icon_is_in_the_font` checks each one against the font file.

/// Phosphor Icons Regular (MIT).
pub const FONT: &[u8] = include_bytes!("../../assets/icons/Phosphor-Regular.ttf");

pub const FILE_PLUS: &str = "\u{e236}";
pub const FOLDER_OPEN: &str = "\u{e256}";
pub const FLOPPY_DISK: &str = "\u{e248}";
pub const FLOPPY_DISK_BACK: &str = "\u{eaf4}";
pub const DOWNLOAD_SIMPLE: &str = "\u{e20c}";
pub const EXPORT: &str = "\u{eaf0}";
pub const SUBTITLES: &str = "\u{e1a8}";
pub const SIGN_OUT: &str = "\u{e42a}";
pub const ARROW_U_UP_LEFT: &str = "\u{e08a}";
pub const ARROW_U_UP_RIGHT: &str = "\u{e08c}";
pub const SCISSORS: &str = "\u{eae0}";
pub const COPY: &str = "\u{e1ca}";
pub const CLIPBOARD: &str = "\u{e196}";
pub const TRASH: &str = "\u{e4a6}";
pub const SPLIT_HORIZONTAL: &str = "\u{e872}";
pub const ARROWS_MERGE: &str = "\u{ed3e}";
pub const ARROW_LEFT: &str = "\u{e058}";
pub const ARROW_RIGHT: &str = "\u{e06c}";
pub const FILM_STRIP: &str = "\u{e792}";
pub const MUSIC_NOTE: &str = "\u{e33c}";
pub const SLIDERS: &str = "\u{e432}";
pub const GEAR: &str = "\u{e270}";
pub const PLAY: &str = "\u{e3d0}";
pub const PAUSE: &str = "\u{e39e}";
pub const SKIP_BACK: &str = "\u{e5a4}";
pub const SKIP_FORWARD: &str = "\u{e5a6}";
pub const CARET_LEFT: &str = "\u{e138}";
pub const CARET_RIGHT: &str = "\u{e13a}";
pub const CARET_UP: &str = "\u{e13c}";
pub const CARET_DOWN: &str = "\u{e136}";
pub const CIRCLE_HALF: &str = "\u{e18c}";
pub const SPEAKER_HIGH: &str = "\u{e44a}";
pub const SPEAKER_SLASH: &str = "\u{e45a}";
pub const WAVEFORM: &str = "\u{e802}";
pub const FRAME_CORNERS: &str = "\u{e626}";
pub const ARROWS_OUT: &str = "\u{e0a2}";
pub const ARROWS_OUT_CARDINAL: &str = "\u{e0a4}";
pub const PALETTE: &str = "\u{e6c8}";
pub const IMAGE: &str = "\u{e2ca}";
pub const TEXT_T: &str = "\u{e48a}";
pub const MAGIC_WAND: &str = "\u{e6b6}";
pub const CHAT_TEXT: &str = "\u{e17a}";
pub const ALIGN_CENTER_HORIZONTAL: &str = "\u{e50a}";
pub const ALIGN_CENTER_VERTICAL: &str = "\u{e50c}";
pub const ARROW_COUNTER_CLOCKWISE: &str = "\u{e038}";
pub const CROSSHAIR: &str = "\u{e1d6}";
pub const BOUNDING_BOX: &str = "\u{e6ce}";
pub const MAGNIFYING_GLASS_PLUS: &str = "\u{e310}";
pub const MAGNIFYING_GLASS_MINUS: &str = "\u{e30e}";
pub const CORNERS_OUT: &str = "\u{e1d0}";
pub const SIDEBAR: &str = "\u{eab6}";
pub const KEYBOARD: &str = "\u{e2d8}";
pub const INFO: &str = "\u{e2ce}";
pub const X: &str = "\u{e4f6}";
pub const CHECK: &str = "\u{e182}";
pub const WARNING: &str = "\u{e4e0}";
pub const QUESTION: &str = "\u{e3e8}";
pub const LIST: &str = "\u{e2f0}";
pub const STOP: &str = "\u{e46c}";
pub const UPLOAD_SIMPLE: &str = "\u{e4c0}";
pub const EYE: &str = "\u{e220}";
pub const GLOBE: &str = "\u{e288}";
pub const CUBE: &str = "\u{e1da}";
pub const DIAMOND: &str = "\u{e1ec}";
pub const FLAG_BANNER: &str = "\u{e622}";
pub const VIDEO_CAMERA: &str = "\u{e4da}";
pub const CLOCK: &str = "\u{e19a}";
pub const MONITOR_PLAY: &str = "\u{e58c}";
pub const HARD_DRIVES: &str = "\u{e2a0}";
pub const FOLDER_SIMPLE: &str = "\u{e25a}";
pub const BROOM: &str = "\u{ec54}";
pub const CHECK_CIRCLE: &str = "\u{e184}";
pub const WARNING_CIRCLE: &str = "\u{e4e2}";
pub const X_CIRCLE: &str = "\u{e4f8}";
pub const ARROWS_CLOCKWISE: &str = "\u{e094}";
pub const LIGHTNING: &str = "\u{e2de}";
pub const CPU: &str = "\u{e610}";
pub const HEART: &str = "\u{e2a8}";
pub const FILE_VIDEO: &str = "\u{ea22}";
pub const PENCIL_SIMPLE: &str = "\u{e3b4}";
pub const TRANSLATE: &str = "\u{e4a2}";
pub const PLUS: &str = "\u{e3d4}";

/// Every icon above (for the font test).
pub const ALL: &[&str] = &[
    FILE_PLUS,
    FOLDER_OPEN,
    FLOPPY_DISK,
    FLOPPY_DISK_BACK,
    DOWNLOAD_SIMPLE,
    EXPORT,
    SUBTITLES,
    SIGN_OUT,
    ARROW_U_UP_LEFT,
    ARROW_U_UP_RIGHT,
    SCISSORS,
    COPY,
    CLIPBOARD,
    TRASH,
    SPLIT_HORIZONTAL,
    ARROWS_MERGE,
    ARROW_LEFT,
    ARROW_RIGHT,
    FILM_STRIP,
    MUSIC_NOTE,
    SLIDERS,
    GEAR,
    PLAY,
    PAUSE,
    SKIP_BACK,
    SKIP_FORWARD,
    CARET_LEFT,
    CARET_RIGHT,
    CARET_UP,
    CARET_DOWN,
    CIRCLE_HALF,
    SPEAKER_HIGH,
    SPEAKER_SLASH,
    WAVEFORM,
    FRAME_CORNERS,
    ARROWS_OUT,
    ARROWS_OUT_CARDINAL,
    PALETTE,
    TEXT_T,
    IMAGE,
    MAGIC_WAND,
    CHAT_TEXT,
    ALIGN_CENTER_HORIZONTAL,
    ALIGN_CENTER_VERTICAL,
    ARROW_COUNTER_CLOCKWISE,
    CROSSHAIR,
    BOUNDING_BOX,
    MAGNIFYING_GLASS_PLUS,
    MAGNIFYING_GLASS_MINUS,
    CORNERS_OUT,
    SIDEBAR,
    KEYBOARD,
    INFO,
    X,
    CHECK,
    WARNING,
    QUESTION,
    LIST,
    STOP,
    UPLOAD_SIMPLE,
    EYE,
    GLOBE,
    CUBE,
    DIAMOND,
    FLAG_BANNER,
    VIDEO_CAMERA,
    CLOCK,
    MONITOR_PLAY,
    HARD_DRIVES,
    FOLDER_SIMPLE,
    BROOM,
    CHECK_CIRCLE,
    WARNING_CIRCLE,
    X_CIRCLE,
    ARROWS_CLOCKWISE,
    LIGHTNING,
    CPU,
    HEART,
    FILE_VIDEO,
    PENCIL_SIMPLE,
    TRANSLATE,
    PLUS,
];

/// The name of the font in egui.
pub const FONT_NAME: &str = "videolore-icons";

/// Adds the icon font as the last fallback of the normal and the monospace fonts (once per
/// context), so an icon is just a character in any text, and text measuring includes it.
pub fn install(ctx: &eframe::egui::Context) {
    use eframe::egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
    use eframe::egui::{FontData, FontFamily};
    if ctx.fonts(|f| f.definitions().font_data.contains_key(FONT_NAME)) {
        return;
    }
    let family = |family| InsertFontFamily {
        family,
        priority: FontPriority::Lowest,
    };
    ctx.add_font(FontInsert::new(
        FONT_NAME,
        FontData::from_static(FONT),
        vec![
            family(FontFamily::Proportional),
            family(FontFamily::Monospace),
        ],
    ));
}

/// An icon followed by its text, for buttons, menu items and titles.
pub fn label(icon: &str, text: &str) -> String {
    format!("{icon} {text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ctx: &eframe::egui::Context, f: impl FnMut(&mut eframe::egui::Ui)) {
        let mut out = ctx.run_ui(Default::default(), f);
        out.textures_delta.clear();
        out.drop_without_applying_deltas();
    }

    // covers: REQ-142
    #[test]
    fn every_icon_is_in_the_font() {
        use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
        use std::sync::Arc;
        // a context whose only font is the icon font: a glyph found there is in Phosphor
        let ctx = Context::default();
        let mut defs = FontDefinitions::empty();
        defs.font_data
            .insert("icons".into(), Arc::new(FontData::from_static(FONT)));
        defs.families
            .insert(FontFamily::Proportional, vec!["icons".into()]);
        ctx.set_fonts(defs);
        run(&ctx, |_| {});
        let only_icons = FontId::proportional(14.0);
        for icon in ALL {
            let mut it = icon.chars();
            let c = it.next().unwrap();
            assert!(it.next().is_none(), "one character per icon");
            assert!(
                ctx.fonts_mut(|f| f.has_glyph(&only_icons, c)),
                "U+{:04X} is not in the font",
                c as u32
            );
        }
        // and an unused private-use character is not (the check can fail)
        assert!(!ctx.fonts_mut(|f| f.has_glyph(&only_icons, '\u{f8ff}')));
        let mut sorted: Vec<&&str> = ALL.iter().collect();
        sorted.sort();
        sorted.dedup();
        // two controls may share an icon, but the table lists each constant once
        assert!(sorted.len() >= ALL.len() - 2);
    }

    // covers: REQ-143
    #[test]
    fn the_font_is_the_one_recorded_in_source_txt_and_has_its_licence() {
        use sha2::{Digest, Sha256};
        let source = include_str!("../../assets/icons/SOURCE.txt");
        let hash: String = Sha256::digest(FONT)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(
            source.contains(&hash),
            "SOURCE.txt records the font's SHA-256"
        );
        assert!(source.contains("Version:  2.1.2") && source.contains("MIT"));
        let licence = include_str!("../../assets/icons/LICENSE-Phosphor.txt");
        assert!(licence.contains("MIT License") && licence.contains("Phosphor Icons"));
        // one icon font only: no icon of another licence is bundled
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        let fonts: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x == "ttf" || x == "otf")
            })
            .collect();
        assert_eq!(fonts.len(), 1);
    }

    #[test]
    fn install_registers_the_font_once_and_label_joins() {
        let ctx = eframe::egui::Context::default();
        run(&ctx, |ui| install(ui.ctx()));
        run(&ctx, |ui| install(ui.ctx()));
        assert!(ctx.fonts(|f| f.definitions().font_data.contains_key(FONT_NAME)));
        assert_eq!(label(PLAY, "Play"), format!("{PLAY} Play"));
    }
}
