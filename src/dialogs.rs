// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Native file dialogs (rfd). Thin glue: the logic using the chosen paths is in `app`.

use crate::app::Dialogs;
use std::path::PathBuf;

const MEDIA_EXT: &[&str] = &[
    "mp4", "mov", "m4v", "mkv", "webm", "avi", "mts", "m2ts", "mp3", "m4a", "aac", "wav", "flac",
    "ogg", "opus",
];

/// Dialogs shown with the platform's file chooser.
#[derive(Debug, Default)]
pub struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn pick_media(&mut self) -> Vec<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Video and audio", MEDIA_EXT)
            .add_filter("All files", &["*"])
            .pick_files()
            .unwrap_or_default()
    }

    fn pick_project_open(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("videolore project", &[crate::project::EXTENSION])
            .pick_file()
    }

    fn pick_project_save(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("videolore project", &[crate::project::EXTENSION])
            .set_file_name("project.vlproj")
            .save_file()
    }

    fn pick_export(&mut self, extension: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Video", &[extension])
            .set_file_name(format!("export.{extension}"))
            .save_file()
    }

    fn pick_ffmpeg(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }

    fn pick_folder(&mut self, start: &std::path::Path) -> Option<PathBuf> {
        rfd::FileDialog::new().set_directory(start).pick_folder()
    }

    fn pick_model_file(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Speech model", &["bin"])
            .pick_file()
    }

    fn pick_lut(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("LUT file", &["cube", "3dl", "csp"])
            .add_filter("All files", &["*"])
            .pick_file()
    }

    fn pick_subtitles_save(&mut self, default_name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("SubRip (.srt)", &["srt"])
            .add_filter("WebVTT (.vtt)", &["vtt"])
            .add_filter("Advanced SubStation Alpha (.ass)", &["ass"])
            .set_file_name(default_name)
            .save_file()
    }

    fn pick_subtitles_open(&mut self) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Subtitles", &["srt", "vtt"])
            .pick_file()
    }
}
