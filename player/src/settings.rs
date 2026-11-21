// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The player's own settings file (REQ-252, ADR-184): the volume, the sound being off,
//! the last folder and full screen, and the version that last showed the start picture.
//! It is program data in the user's configuration folder, never next to a video. The file
//! is untrusted: it is read with a size limit and every value is checked; anything that is
//! not understood is the default.

use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The largest settings file read, in bytes.
pub const MAX_BYTES: u64 = 16 * 1024;
/// The longest remembered folder, in bytes.
pub const MAX_FOLDER: usize = 1024;

/// What the player remembers between runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayerSettings {
    /// 0 to 100.
    pub volume: u8,
    pub muted: bool,
    /// The folder of the last video opened (the dialogs start there).
    pub last_folder: Option<PathBuf>,
    pub fullscreen: bool,
    /// The version that last showed the start picture.
    pub last_splash_version: Option<String>,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        PlayerSettings {
            volume: 100,
            muted: false,
            last_folder: None,
            fullscreen: false,
            last_splash_version: None,
        }
    }
}

impl PlayerSettings {
    /// Reads the settings from a JSON value, value by value.
    pub fn from_value(v: &Value) -> PlayerSettings {
        let mut s = PlayerSettings::default();
        let Some(o) = v.as_object() else {
            return s;
        };
        if let Some(n) = o.get("volume").and_then(Value::as_u64) {
            s.volume = n.min(100) as u8;
        }
        s.muted = o.get("muted").and_then(Value::as_bool).unwrap_or(false);
        s.fullscreen = o
            .get("fullscreen")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        s.last_folder = o
            .get("last_folder")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .filter(|p| valid_folder(p));
        s.last_splash_version = o
            .get("last_splash_version")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty() && v.len() <= 32 && !v.chars().any(char::is_control))
            .map(str::to_string);
        s
    }

    /// Where the file is: `player.json` in the user's configuration folder.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "videolore-player")
            .map(|d| d.config_dir().join("player.json"))
    }

    /// Reads the file; a missing, too big or damaged file gives the defaults.
    pub fn load(path: &Path) -> PlayerSettings {
        videolore::base::io::read_limited(path, MAX_BYTES)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| PlayerSettings::from_value(&v))
            .unwrap_or_default()
    }

    /// Writes the file (creating its folder), atomically.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        videolore::base::io::write_atomic(path, json.as_bytes()).map_err(|e| e.to_string())
    }
}

/// A folder that may be remembered: UTF-8, absolute, short, without control characters.
fn valid_folder(p: &Path) -> bool {
    p.is_absolute()
        && p.to_str()
            .is_some_and(|s| s.len() <= MAX_FOLDER && !s.chars().any(char::is_control))
}

/// Whether `p` can be remembered as the last folder.
pub fn remember_folder(p: &Path) -> bool {
    valid_folder(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // covers: REQ-252
    #[test]
    fn the_settings_are_kept_between_runs_and_a_damaged_file_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/player.json");
        let s = PlayerSettings {
            volume: 35,
            muted: true,
            last_folder: Some(dir.path().to_path_buf()),
            fullscreen: true,
            last_splash_version: Some("2.0.4".into()),
        };
        s.save(&path).unwrap();
        assert_eq!(PlayerSettings::load(&path), s);
        // nothing else is written in the folder of the settings
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
        // missing, not JSON, too big: the defaults
        assert_eq!(
            PlayerSettings::load(&dir.path().join("none")),
            PlayerSettings::default()
        );
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(PlayerSettings::load(&path), PlayerSettings::default());
        std::fs::write(&path, vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert_eq!(PlayerSettings::load(&path), PlayerSettings::default());
    }

    // covers: REQ-252
    #[test]
    fn every_value_is_checked_one_by_one() {
        let v = json!({"volume": 900, "muted": "yes", "last_folder": "relative/dir",
            "fullscreen": true, "last_splash_version": "bad\u{7}"});
        let s = PlayerSettings::from_value(&v);
        assert_eq!((s.volume, s.muted, s.fullscreen), (100, false, true));
        assert_eq!((s.last_folder, s.last_splash_version), (None, None));
        let long = format!("/{}", "x".repeat(MAX_FOLDER));
        assert!(
            PlayerSettings::from_value(&json!({ "last_folder": long }))
                .last_folder
                .is_none()
        );
        assert_eq!(
            PlayerSettings::from_value(&json!([1, 2])),
            PlayerSettings::default()
        );
        assert!(remember_folder(Path::new("/tmp")) && !remember_folder(Path::new("tmp")));
    }
}
