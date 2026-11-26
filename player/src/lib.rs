// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! videolore player: a small program that plays video files, one after the other, with the 360 view,
//! subtitles from a file or from the video, and full screen. It reuses the editor's
//! engine (`videolore`, without speech-to-text) and has no network code.
//!
//! All logic lives here so that it can be tested; `main.rs` only starts the window.
#![deny(unsafe_code)]

pub mod app;
pub mod fullscreen;
pub mod playlist;
pub mod settings;
pub mod subs;
pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use videolore::ffmpeg::{self, SystemRunner};

/// Native file dialogs.
#[derive(Debug, Default)]
pub struct NativeDialogs {
    /// The folder the next dialog starts in.
    start: Option<PathBuf>,
}

const VIDEO_EXT: &[&str] = playlist::VIDEO_EXT;

impl NativeDialogs {
    fn dialog(&self) -> rfd::FileDialog {
        match &self.start {
            Some(d) if d.is_dir() => rfd::FileDialog::new().set_directory(d),
            _ => rfd::FileDialog::new(),
        }
    }
}

impl app::Dialogs for NativeDialogs {
    fn pick_videos(&mut self) -> Vec<PathBuf> {
        self.dialog()
            .add_filter("Video", VIDEO_EXT)
            .add_filter("All files", &["*"])
            .pick_files()
            .unwrap_or_default()
    }

    fn pick_srt(&mut self) -> Option<PathBuf> {
        self.dialog().add_filter("Subtitles", &["srt"]).pick_file()
    }

    fn pick_folder(&mut self) -> Option<PathBuf> {
        self.dialog().pick_folder()
    }

    fn start_in(&mut self, folder: Option<PathBuf>) {
        self.start = folder;
    }
}

/// When the start picture shows, as chosen in the editor's settings (the player follows it;
/// the file is only read).
fn editor_splash_option() -> videolore::settings::SplashOption {
    videolore::settings::Settings::default_path()
        .and_then(|p| match videolore::settings::load_file(&p) {
            videolore::settings::Loaded::Bytes(b) => {
                serde_json::from_slice::<videolore::settings::Settings>(&b).ok()
            }
            _ => None,
        })
        .map(|s| s.show_splash)
        .unwrap_or_default()
}

/// The player with the real ffmpeg, dialogs and audio device. `file` is the video given on
/// the command line, opened at once.
pub fn real_app(file: Option<PathBuf>) -> ui::PlayerApp {
    let runner = Arc::new(SystemRunner);
    let ffmpeg = ffmpeg::locate_both(None, None)
        .map_err(|e| e.to_string())
        .and_then(|l| ffmpeg::check_located(&*runner, &l).map_err(|e| e.to_string()));
    let now = Instant::now();
    let settings_path = settings::PlayerSettings::default_path();
    let remembered = settings_path
        .as_deref()
        .map(settings::PlayerSettings::load)
        .unwrap_or_default();
    let mut state = app::PlayerState::new(
        ffmpeg,
        runner,
        Box::new(NativeDialogs::default()),
        videolore::preview::default_sink(),
        videolore::machine::Machine::detect(),
        app::Folders {
            cache_dir: videolore::proxy::ProxyCache::default_dir(),
            work_root: videolore::scratch::default_root(),
        },
        now,
    )
    .with_settings(remembered, settings_path, now);
    match file {
        Some(p) => state.dispatch(app::Command::Open(p), now),
        // a start without a video shows the start screen (REQ-150) as the editor's
        // choice says (REQ-295)
        None => state.start_splash(editor_splash_option()),
    }
    ui::PlayerApp {
        state,
        cache: ui::PlayerCache::default(),
    }
}

/// Window options: 960x600 by default, never smaller than 800x500, with the program icon
/// (the same artwork as the editor's).
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("videolore player")
            .with_app_id("videolore-player")
            .with_icon(Arc::new(videolore::app_icon()))
            .with_inner_size([960.0, 600.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    }
}

/// The video named on the command line: the first argument (macOS adds `-psn_…` when
/// launched from the Finder; it is ignored).
pub fn file_arg<I: IntoIterator<Item = std::ffi::OsString>>(args: I) -> Option<PathBuf> {
    videolore::desktop::project_arg(args)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ui_tests;
