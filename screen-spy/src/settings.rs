// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The recorder's settings file (REQ-352, ADR-263): the last folder, the video format
//! and the sound format, version 1. It lives in the user's configuration folder, never
//! next to a recording. The file is untrusted: a size limit, every value checked, and a
//! damaged file is set aside as `.bad`.

use crate::plan::{AudioFormat, VideoFormat};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// The largest settings file read, in bytes.
pub const MAX_BYTES: u64 = 64 * 1024;
/// The longest remembered folder, in bytes.
pub const MAX_FOLDER: usize = 4096;
/// The version of the file.
pub const VERSION: u64 = 1;

/// What the recorder remembers between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecorderSettings {
    /// The folder of the recordings; `None` = the system's video folder.
    pub folder: Option<PathBuf>,
    pub video: VideoFormat,
    pub audio: AudioFormat,
}

/// A folder that may be remembered: UTF-8, absolute, short, without control characters.
pub fn valid_folder(p: &Path) -> bool {
    p.is_absolute()
        && p.to_str()
            .is_some_and(|s| s.len() <= MAX_FOLDER && !s.chars().any(char::is_control))
}

impl RecorderSettings {
    /// The settings of a file's content; `None` when it is not a version 1 settings file.
    /// A value that is not understood is the default.
    pub fn parse(bytes: &[u8]) -> Option<RecorderSettings> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let o = v.as_object()?;
        if o.get("version")?.as_u64()? != VERSION {
            return None;
        }
        let text = |k: &str| o.get(k).and_then(Value::as_str);
        Some(RecorderSettings {
            folder: text("folder")
                .map(PathBuf::from)
                .filter(|p| valid_folder(p)),
            video: text("video")
                .and_then(VideoFormat::from_key)
                .unwrap_or_default(),
            audio: text("audio")
                .and_then(AudioFormat::from_key)
                .unwrap_or_default(),
        })
    }

    /// Where the file is: `screen-spy.json` in the user's configuration folder.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "videolore-screen-spy")
            .map(|d| d.config_dir().join("screen-spy.json"))
    }

    /// Reads the file; a missing, too big or damaged file gives the defaults, and what
    /// could not be understood is renamed to `.bad`.
    pub fn load(path: &Path) -> RecorderSettings {
        use std::io::ErrorKind;
        let parsed = match videolore::base::io::read_limited(path, MAX_BYTES) {
            Ok(bytes) => RecorderSettings::parse(&bytes),
            // too big; anything else (missing, not a file) is left where it is
            Err(e) if e.kind() == ErrorKind::InvalidData => None,
            Err(_) => return RecorderSettings::default(),
        };
        parsed.unwrap_or_else(|| {
            let mut bad = path.as_os_str().to_owned();
            bad.push(".bad");
            let _ = std::fs::rename(path, bad);
            RecorderSettings::default()
        })
    }

    /// Writes the file (creating its folder), atomically.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let json = json!({
            "version": VERSION,
            "folder": self.folder.as_deref().and_then(Path::to_str),
            "video": self.video.key(),
            "audio": self.audio.key(),
        });
        videolore::base::io::write_atomic(path, format!("{json:#}\n").as_bytes())
            .map_err(|e| e.to_string())
    }
}

/// The folder offered for recordings: the system's video folder (Movies on macOS, Videos
/// elsewhere), else the home folder (REQ-345).
pub fn folder_of(video: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    video.or(home).unwrap_or_else(|| PathBuf::from("."))
}

/// [`folder_of`] on this machine.
pub fn default_folder() -> PathBuf {
    let dirs = directories::UserDirs::new();
    folder_of(
        dirs.as_ref()
            .and_then(|d| d.video_dir().map(Path::to_path_buf)),
        dirs.map(|d| d.home_dir().to_path_buf()),
    )
}
