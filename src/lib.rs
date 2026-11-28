// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! videolore: a non-destructive video editor built on ffmpeg.
//!
//! All the logic lives in this library so that it can be tested; `main.rs` only starts the
//! window through [`real_app`] and [`native_options`].
//!
//! `unsafe` code is denied everywhere except in the two modules that must call C or
//! Objective-C: the speech engine's cancel callback (`stt::whisper`) and the macOS
//! "open document" handler (`desktop::macos`). Each block there carries a SAFETY comment.
#![deny(unsafe_code)]

// lets the test font helper shared with the integration tests name the crate the same way
#[cfg(test)]
extern crate self as videolore;

pub mod align;
pub mod app;
pub mod ass;
pub mod autosave;
pub mod base;
pub mod camera;
pub mod desktop;
pub mod dialogs;
pub mod effect_order;
pub mod export;
pub mod ffmpeg;
pub mod fonts;
pub mod format;
pub mod frame;
pub mod history;
pub mod hwdec;
pub mod hwenc;
pub mod instance;
pub mod jobs;
pub mod keyframes;
pub mod lut;
pub mod machine;
pub mod measure;
pub mod media;
pub mod mp4tracks;
pub mod music;
pub mod nested;
pub mod notices;
pub mod order_chain;
pub mod outcheck;
pub mod overlap;
pub mod packaging;
pub mod picture;
pub mod placement;
pub mod preview;
pub mod project;
pub mod proxy;
pub mod render;
pub mod scratch;
pub mod selection;
pub mod settings;
pub mod siblings;
pub mod spatial;
pub mod speech_regions;
pub mod spherical;
pub mod splash;
pub mod srt;
pub mod strips;
pub mod stt;
pub mod subtitles;
pub mod text;
pub mod time;
pub mod timeline;
pub mod word_layout;
pub mod word_styles;

pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;

/// The window icon (256x256 RGBA, generated from `assets/icon/videolore.svg`).
const ICON_RGBA: &[u8] = include_bytes!("../assets/icon/icon-256.rgba");

/// Builds the application state with the real ffmpeg, dialogs, audio device and settings.
/// `project` is the file given on the command line (double-click on a `.vlproj`).
pub fn real_app(project: Option<PathBuf>) -> ui::VideoloreApp {
    let state = app::AppState::new(
        Arc::new(ffmpeg::SystemRunner),
        Box::new(dialogs::NativeDialogs),
        preview::default_sink(),
        settings::Settings::default_path(),
    );
    let mut state = state;
    match project {
        // a start that opens a file shows no splash (REQ-133)
        Some(p) => desktop::push_open_request(p),
        None => state.start_splash(),
    }
    ui::VideoloreApp {
        state,
        cache: ui::UiCache::default(),
    }
}

/// The window icon of both programs (they share the artwork, REQ-138).
pub fn app_icon() -> eframe::egui::IconData {
    eframe::egui::IconData {
        rgba: ICON_RGBA.to_vec(),
        width: 256,
        height: 256,
    }
}

/// Window options: 1280x800 by default, never smaller than 800x500, with the app icon.
/// On macOS the default menu is disabled so that Cmd+Q goes through the unsaved-changes
/// prompt.
pub fn native_options() -> eframe::NativeOptions {
    native_options_for(saved_window())
}

/// The window the last run left (REQ-234): read leniently from the settings file.
pub fn saved_window() -> Option<settings::interface::Window> {
    let path = settings::Settings::default_path()?;
    let settings::Loaded::Bytes(bytes) = settings::load_file(&path) else {
        return None;
    };
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    settings::Interface::from_value(v.get("interface")?).window
}

/// The window options for the saved window `w`.
pub fn native_options_for(w: Option<settings::interface::Window>) -> eframe::NativeOptions {
    let icon = app_icon();
    let (size, pos, maximised) = ui::place::viewport_of(w);
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("videolore")
            .with_app_id("videolore")
            .with_icon(Arc::new(icon))
            .with_inner_size(size)
            .with_maximized(maximised)
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    if let Some(p) = pos {
        options.viewport = options.viewport.with_position(p);
    }
    #[cfg(target_os = "macos")]
    {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            builder.with_default_menu(false);
        }));
    }
    options
}

/// Start-up hooks that must run before the window: the Linux menu entry (AppImage and
/// tar.gz) and the macOS handler for double-clicked projects.
pub fn prepare_desktop() {
    desktop::integrate_at_startup();
    #[cfg(target_os = "macos")]
    desktop::macos::install();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_options_and_real_app_build() {
        let o = native_options();
        assert!(o.viewport.min_inner_size.is_some());
        assert!(o.viewport.icon.is_some());
        assert_eq!(ICON_RGBA.len(), 256 * 256 * 4);
        let app = real_app(Some("/nowhere/p.vlproj".into()));
        assert!(!app.cache.has_texture());
        assert!(app.state.project.media.is_empty());
        assert!(desktop::take_open_requests().contains(&PathBuf::from("/nowhere/p.vlproj")));
        prepare_desktop();
    }
}
