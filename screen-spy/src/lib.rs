// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! videolore screen spy: records one screen, with the computer sound and the microphone
//! mixed into one sound track, to a video file (REQ-341 to 352, 357, 358). It has a live
//! preview and does nothing else: no editing, no project file and no network code.
//!
//! The picture comes from ffmpeg's own screen inputs; at most one ffmpeg process captures
//! at a time. All logic lives here so that it can be tested; `main.rs` only starts the
//! window.
//!
//! `unsafe` code is denied everywhere except in [`macos`], which calls Objective-C and
//! Core Audio (permissions, the tap of the computer's sound); each block there carries a
//! SAFETY comment.
#![deny(unsafe_code)]

pub mod about;
pub mod app;
pub mod audio;
pub mod devices;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub mod macos;
pub mod plan;
pub mod preview;
pub mod process;
pub mod screens;
pub mod session;
pub mod settings;
pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use videolore::ffmpeg::{self, SystemRunner};

/// The native save dialog.
#[derive(Debug, Default)]
pub struct NativeDialogs;

impl app::Dialogs for NativeDialogs {
    fn save_file(
        &mut self,
        folder: &std::path::Path,
        name: &str,
        extension: &str,
    ) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter("Video", &[extension]);
        if folder.is_dir() {
            d = d.set_directory(folder);
        }
        d.save_file()
    }
}

/// The current local date and time as it is written in a default file name.
pub fn local_stamp() -> String {
    jiff::ZonedDateTime::now()
        .strftime("%Y-%m-%d-%H%M%S")
        .to_string()
}

/// The ffmpeg of this installation: the one next to the program before any other
/// (the editor's own order, ADR-013).
fn find_ffmpeg() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
    ffmpeg::locate(
        "ffmpeg",
        None,
        std::env::var_os("VIDEOLORE_FFMPEG").as_deref(),
        exe_dir.as_deref(),
        std::env::var_os("PATH").as_deref(),
    )
    .map(|(p, _)| p)
}

/// The recorder of this machine: its screens, sound, ffmpeg and settings.
pub fn real_app() -> ui::RecorderApp {
    let runner: Arc<dyn ffmpeg::Runner> = Arc::new(SystemRunner);
    let os = plan::Os::current();
    let platform = session::Platform::detect(os, |k| std::env::var_os(k), mac_version());
    let settings_path = settings::RecorderSettings::default_path();
    let remembered = settings_path
        .as_deref()
        .map(settings::RecorderSettings::load)
        .unwrap_or_default();
    let deps = app::Deps {
        spawner: Box::new(process::SystemSpawner),
        runner: Arc::clone(&runner),
        screens: Box::new(screens::SystemScreens(os)),
        sound: audio::system_sound(runner),
        permission: system_permission(),
        dialogs: Box::new(NativeDialogs),
        stamp: Box::new(local_stamp),
    };
    let setup = app::Setup {
        platform,
        ffmpeg: find_ffmpeg(),
        settings: remembered,
        settings_path,
        default_folder: settings::default_folder(),
    };
    ui::RecorderApp {
        state: app::RecorderState::new(deps, setup),
        cache: ui::RecorderCache::default(),
    }
}

#[cfg(target_os = "macos")]
fn mac_version() -> Option<session::MacVersion> {
    Some(macos::version())
}

#[cfg(not(target_os = "macos"))]
fn mac_version() -> Option<session::MacVersion> {
    None
}

#[cfg(target_os = "macos")]
fn system_permission() -> Box<dyn app::Permission> {
    Box::new(macos::ScreenPermission)
}

#[cfg(not(target_os = "macos"))]
fn system_permission() -> Box<dyn app::Permission> {
    Box::new(app::NoPermissionNeeded)
}

/// Window options: 800x680 by default, never smaller than 640x560, with the program icon
/// (the same artwork as the editor's, ADR-270).
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("videolore screen spy")
            .with_app_id("videolore-screen-spy")
            .with_icon(Arc::new(videolore::app_icon()))
            .with_inner_size(ui::DEFAULT_SIZE)
            .with_min_inner_size(ui::MIN_SIZE),
        ..Default::default()
    }
}

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod ui_tests;
