// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The playlist (REQ-254, 355, 356): the videos to play one after the other. It exists only while
//! the player is open and is never written anywhere.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use videolore::time::Micros;

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
    /// How long each video is, as far as it was looked at (`None`: it cannot be told).
    lengths: HashMap<PathBuf, Option<Micros>>,
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

    /// Moves the video at `i` one place earlier; the one playing stays the one playing.
    /// False when there is no earlier place.
    pub fn move_up(&mut self, i: usize) -> bool {
        match i.checked_sub(1) {
            Some(to) if i < self.items.len() => self.swap(i, to),
            _ => false,
        }
    }

    /// Moves the video at `i` one place later (as [`Self::move_up`]).
    pub fn move_down(&mut self, i: usize) -> bool {
        match i.checked_add(1) {
            Some(to) if to < self.items.len() => self.swap(i, to),
            _ => false,
        }
    }

    fn swap(&mut self, a: usize, b: usize) -> bool {
        self.items.swap(a, b);
        self.current = self.current.map(|c| match c {
            c if c == a => b,
            c if c == b => a,
            c => c,
        });
        true
    }

    /// Takes the video at `i` out of the list and returns it. When it was the current one
    /// the one that took its place becomes current (none after the last).
    pub fn remove(&mut self, i: usize) -> Option<PathBuf> {
        if i >= self.items.len() {
            return None;
        }
        let gone = self.items.remove(i);
        self.current = match self.current {
            Some(c) if c > i => Some(c - 1),
            Some(c) if c == i => (i < self.items.len()).then_some(i),
            other => other,
        };
        Some(gone)
    }

    /// Notes how long the video `p` is.
    pub fn set_length(&mut self, p: PathBuf, length: Option<Micros>) {
        self.lengths.insert(p, length);
    }

    /// The first video of a list of two or more that was not looked at yet.
    pub fn unknown(&self) -> Option<&PathBuf> {
        self.items
            .iter()
            .filter(|_| self.items.len() > 1)
            .find(|p| !self.lengths.contains_key(*p))
    }

    /// The length of the whole list (REQ-375): `None` for a list of one video at most,
    /// `Some(None)` while a length is not known.
    pub fn total(&self) -> Option<Option<Micros>> {
        (self.items.len() > 1).then(|| {
            self.items.iter().try_fold(Micros::ZERO, |sum, p| {
                self.lengths.get(p).copied().flatten().map(|l| sum + l)
            })
        })
    }

    /// Replaces the list with the videos of a folder, none playing yet.
    pub fn set_folder(&mut self, dir: &Path) -> usize {
        self.items = list_folder(dir);
        self.current = None;
        self.items.len()
    }

    /// Replaces the list with these videos in the order given (once each, at most
    /// [`MAX_ENTRIES`]), none playing yet (REQ-355). Returns how many it holds.
    pub fn replace(&mut self, paths: Vec<PathBuf>) -> usize {
        self.clear();
        let mut seen = std::collections::HashSet::new();
        self.items = paths
            .into_iter()
            .filter(|p| seen.insert(p.clone()))
            .take(MAX_ENTRIES)
            .collect();
        self.items.len()
    }

    /// Where the playing video is in the list, from 1, and how long the list is; `None`
    /// when the list has one video at most or the playing one is not in it (REQ-356).
    pub fn place(&self) -> Option<(usize, usize)> {
        let len = self.items.len();
        self.current.filter(|_| len > 1).map(|i| (i + 1, len))
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

#[cfg(test)]
mod order_tests {
    use super::*;

    fn list(n: &[&str]) -> Playlist {
        let mut l = Playlist::default();
        for n in n {
            l.add(format!("/{n}.mp4").into());
        }
        l
    }

    fn names(l: &Playlist) -> String {
        l.items
            .iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    // covers: REQ-374
    #[test]
    fn moving_and_removing_keep_the_current_video() {
        let mut l = list(&["a", "b", "c"]);
        l.play(Path::new("/b.mp4"));
        assert!(!l.move_up(0) && !l.move_down(2) && !l.move_up(9) && !l.move_down(9));
        assert!(l.move_up(2));
        assert_eq!((names(&l).as_str(), l.current), ("acb", Some(2)));
        assert!(l.move_up(2));
        assert_eq!((names(&l).as_str(), l.current), ("abc", Some(1)));
        assert!(l.move_down(0));
        assert_eq!((names(&l).as_str(), l.current), ("bac", Some(0)));
        assert!(l.remove(2).is_some());
        assert_eq!(l.current, Some(0));
        assert_eq!(l.remove(0), Some("/b.mp4".into()));
        assert_eq!(l.current, Some(0), "the one that took its place");
        let mut l = list(&["a", "b", "c"]);
        l.play(Path::new("/c.mp4"));
        l.remove(0);
        assert_eq!(l.current, Some(1));
        l.remove(1);
        assert_eq!(l.current, None, "it was the last");
        assert_eq!(l.remove(7), None);
    }

    // covers: REQ-375
    #[test]
    fn the_total_needs_two_videos_and_every_length() {
        let minute = Micros(60_000_000);
        let mut l = list(&["a"]);
        assert_eq!((l.total(), l.unknown()), (None, None));
        l.add("/b.mp4".into());
        assert_eq!(l.total(), Some(None));
        assert_eq!(l.unknown(), Some(&PathBuf::from("/a.mp4")));
        l.set_length("/a.mp4".into(), Some(minute));
        assert_eq!(l.total(), Some(None));
        l.set_length("/b.mp4".into(), Some(minute + minute));
        assert_eq!(l.total(), Some(Some(Micros(180_000_000))));
        assert_eq!(l.unknown(), None);
        l.set_length("/b.mp4".into(), None);
        assert_eq!((l.total(), l.unknown()), (Some(None), None));
        l.remove(1);
        assert_eq!(l.total(), None);
    }
}
