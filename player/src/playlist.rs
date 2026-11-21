// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The playlist (REQ-254): the videos to play one after the other. It exists only while
//! the player is open and is never written anywhere.

use std::path::{Path, PathBuf};

/// Most entries; a folder with more is cut here.
pub const MAX_ENTRIES: usize = 10_000;

/// The video types the player lists from a folder (the same as its Open dialog).
pub const VIDEO_EXT: &[&str] = &[
    "mp4", "mov", "m4v", "mkv", "webm", "avi", "mts", "m2ts", "ts",
];

/// True for a file name with a video extension.
pub fn is_video(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| VIDEO_EXT.iter().any(|v| v.eq_ignore_ascii_case(e)))
}

/// The videos of a folder in name order (not its subfolders), at most [`MAX_ENTRIES`].
pub fn list_folder(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| is_video(p))
        .collect();
    files.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    files.truncate(MAX_ENTRIES);
    files
}

/// The videos waiting and the one playing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub items: Vec<PathBuf>,
    /// Index of the video playing now.
    pub current: Option<usize>,
}

impl Playlist {
    /// Adds a video at the end (once); false when it is there already or the list is full.
    pub fn add(&mut self, p: PathBuf) -> bool {
        if self.items.len() >= MAX_ENTRIES || self.items.contains(&p) {
            return false;
        }
        self.items.push(p);
        true
    }

    /// Replaces the list with the videos of a folder, none playing yet.
    pub fn set_folder(&mut self, dir: &Path) -> usize {
        self.items = list_folder(dir);
        self.current = None;
        self.items.len()
    }

    /// Marks `p` as the video playing (adding it when it is not in the list).
    pub fn play(&mut self, p: &Path) {
        match self.items.iter().position(|x| x == p) {
            Some(i) => self.current = Some(i),
            None if self.items.is_empty() => {}
            None => self.current = None,
        }
    }

    /// The video after the current one; `None` after the last (the player stops there).
    pub fn next(&self) -> Option<&PathBuf> {
        self.items.get(self.current? + 1)
    }

    /// The video before the current one.
    pub fn previous(&self) -> Option<&PathBuf> {
        self.items.get(self.current?.checked_sub(1)?)
    }

    pub fn clear(&mut self) {
        *self = Playlist::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-254
    #[test]
    fn a_folder_is_listed_in_name_order_with_videos_only() {
        let dir = tempfile::tempdir().unwrap();
        for n in ["b.mp4", "A.mkv", "c.txt", "d.WEBM"] {
            std::fs::write(dir.path().join(n), "x").unwrap();
        }
        std::fs::create_dir(dir.path().join("sub.mp4")).unwrap();
        let names: Vec<String> = list_folder(dir.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["A.mkv", "b.mp4", "d.WEBM"]);
        assert!(list_folder(&dir.path().join("none")).is_empty());
    }

    // covers: REQ-254
    #[test]
    fn next_and_previous_stop_at_the_ends_of_the_list() {
        let mut l = Playlist::default();
        for n in ["/a.mp4", "/b.mp4", "/c.mp4"] {
            assert!(l.add(n.into()));
        }
        assert!(!l.add("/a.mp4".into()), "once");
        assert_eq!(l.next(), None, "nothing plays yet");
        l.play(Path::new("/a.mp4"));
        assert_eq!(l.previous(), None);
        assert_eq!(l.next(), Some(&PathBuf::from("/b.mp4")));
        l.play(Path::new("/c.mp4"));
        assert_eq!(l.next(), None, "stop after the last");
        assert_eq!(l.previous(), Some(&PathBuf::from("/b.mp4")));
        // a video that is not in the list leaves it
        l.play(Path::new("/elsewhere.mp4"));
        assert_eq!(l.current, None);
        l.clear();
        assert!(l.items.is_empty());
        // the list is capped
        let mut big = Playlist::default();
        for i in 0..MAX_ENTRIES + 5 {
            big.add(format!("/v{i}.mp4").into());
        }
        assert_eq!(big.items.len(), MAX_ENTRIES);
    }
}
