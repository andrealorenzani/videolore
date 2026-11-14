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

pub mod app;
pub mod ass;
pub mod desktop;
pub mod dialogs;
pub mod export;
pub mod ffmpeg;
pub mod format;
pub mod history;
pub mod hwenc;
pub mod jobs;
pub mod machine;
pub mod media;
pub mod music;
pub mod notices;
pub mod outcheck;
pub mod packaging;
pub mod preview;
pub mod project;
pub mod proxy;
pub mod render;
pub mod scratch;
pub mod settings;
pub mod stt;
pub mod subtitles;
pub mod time;
pub mod timeline;

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
    if let Some(p) = project {
        desktop::push_open_request(p);
    }
    ui::VideoloreApp {
        state,
        cache: ui::UiCache::default(),
    }
}

/// Window options: 1280x800 by default, never smaller than 800x500, with the app icon.
/// On macOS the default menu is disabled so that Cmd+Q goes through the unsaved-changes
/// prompt.
pub fn native_options() -> eframe::NativeOptions {
    let icon = eframe::egui::IconData {
        rgba: ICON_RGBA.to_vec(),
        width: 256,
        height: 256,
    };
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("videolore")
            .with_app_id("videolore")
            .with_icon(Arc::new(icon))
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
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
