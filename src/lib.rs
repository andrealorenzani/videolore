// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! videolore: a non-destructive video editor built on ffmpeg.
//!
//! All the logic lives in this library so that it can be tested; `main.rs` only starts the
//! window through [`run`].

pub mod app;
pub mod dialogs;
pub mod export;
pub mod ffmpeg;
pub mod format;
pub mod history;
pub mod media;
pub mod preview;
pub mod project;
pub mod render;
pub mod time;
pub mod timeline;
pub mod ui;

use std::sync::Arc;

/// Builds the application state with the real ffmpeg, dialogs, audio device and settings.
pub fn real_app() -> ui::VideoloreApp {
    let state = app::AppState::new(
        Arc::new(ffmpeg::SystemRunner),
        Box::new(dialogs::NativeDialogs),
        preview::default_sink(),
        app::Settings::default_path(),
    );
    ui::VideoloreApp {
        state,
        cache: ui::UiCache::default(),
    }
}

/// Window options: 1280x800 by default, never smaller than 800x500.
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("videolore")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_options_and_real_app_build() {
        let o = native_options();
        assert!(o.viewport.min_inner_size.is_some());
        let app = real_app();
        assert!(!app.cache.has_texture());
        assert!(app.state.project.media.is_empty());
    }
}
