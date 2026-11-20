// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! videolore player: a small program that plays one video file, with the 360 view,
//! subtitles from a file or from the video, and full screen. It reuses the editor's
//! engine (`videolore`, without speech-to-text) and has no network code.
//!
//! All logic lives here so that it can be tested; `main.rs` only starts the window.
#![deny(unsafe_code)]

pub mod app;
pub mod fullscreen;
pub mod subs;
pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use videolore::ffmpeg::{self, SystemRunner};

/// Native file dialogs.
#[derive(Debug, Default)]
pub struct NativeDialogs;

const VIDEO_EXT: &[&str] = &[
    "mp4", "mov", "m4v", "mkv", "webm", "avi", "mts", "m2ts", "ts",
];

impl app::Dialogs for NativeDialogs {
    fn pick_video(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Video", VIDEO_EXT)
            .add_filter("All files", &["*"])
            .pick_file()
    }

    fn pick_srt(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Subtitles", &["srt"])
            .pick_file()
    }
}

/// The player with the real ffmpeg, dialogs and audio device. `file` is the video given on
/// the command line, opened at once.
pub fn real_app(file: Option<PathBuf>) -> ui::PlayerApp {
    let runner = Arc::new(SystemRunner);
    let ffmpeg = ffmpeg::locate_both(None, None)
        .map_err(|e| e.to_string())
        .and_then(|l| ffmpeg::check_located(&*runner, &l).map_err(|e| e.to_string()));
    let now = Instant::now();
    let mut state = app::PlayerState::new(
        ffmpeg,
        runner,
        Box::new(NativeDialogs),
        videolore::preview::default_sink(),
        videolore::machine::Machine::detect(),
        app::Folders {
            cache_dir: videolore::proxy::ProxyCache::default_dir(),
            work_root: videolore::scratch::default_root(),
        },
        now,
    );
    match file {
        Some(p) => state.dispatch(app::Command::Open(p), now),
        // a start without a video shows the start screen (REQ-150)
        None => state.show_splash(),
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
