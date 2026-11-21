// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Subtitle sources of the player (REQ-140): the text tracks embedded in the video and the
//! `.srt` files opened in this session. Lines are held in the in-memory project (so the
//! preview draws them like the editor's); their places are kept here for the session only.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use videolore::base::pos::Pos;
use videolore::media::{MediaId, PictureTrack, SubtitleTrack, display_name};
use videolore::srt::Cue;
use videolore::subtitles::NewLine;

/// Largest `.srt` file read (the same limit as an extracted track).
pub const MAX_SRT_BYTES: u64 = videolore::stt::MAX_TRACK_BYTES;

/// Where a subtitle list comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// A text track of the video file.
    Embedded(SubtitleTrack),
    /// A `.srt` file opened with Subtitles > Open .srt…
    File(PathBuf),
    /// A picture-based track of the video file (DVD, Blu-ray PGS, DVB; REQ-253): drawn on the
    /// picture by the shared graph; it has no text lines to move.
    Picture(PictureTrack),
}

impl Source {
    /// The name in the Subtitles menu: the track's title or language, or the file name.
    pub fn name(&self) -> String {
        match self {
            Source::Embedded(t) => t.label(),
            Source::File(p) => display_name(p),
            Source::Picture(t) => t.label(),
        }
    }

    /// The name in the Subtitles menu: picture tracks are marked "picture", files "file".
    pub fn menu_name(&self) -> String {
        match self {
            Source::Embedded(_) => self.name(),
            Source::File(_) => format!("{} (file)", self.name()),
            Source::Picture(_) => format!("{} (picture)", self.name()),
        }
    }
}

/// The subtitle state of the open video.
#[derive(Debug, Clone, Default)]
pub struct Subs {
    pub sources: Vec<Source>,
    /// The chosen one; `None` = no subtitles.
    pub selected: Option<usize>,
    /// Where the user dragged lines: (source, line number) to a place on the picture. Kept
    /// while the player is open, never written anywhere.
    placed: HashMap<(usize, usize), Pos>,
    /// Cues already read, by source.
    cues: HashMap<usize, Vec<Cue>>,
}

impl Subs {
    /// The sources of a freshly opened video: its text tracks.
    pub fn for_tracks(tracks: &[SubtitleTrack]) -> Subs {
        Subs::for_media(tracks, &[])
    }

    /// The sources of a freshly opened video: its text tracks, then its picture tracks.
    pub fn for_media(tracks: &[SubtitleTrack], pictures: &[PictureTrack]) -> Subs {
        let text = tracks.iter().cloned().map(Source::Embedded);
        let pics = pictures.iter().cloned().map(Source::Picture);
        Subs {
            sources: text.chain(pics).collect(),
            ..Subs::default()
        }
    }

    /// Adds a `.srt` file (once per path) and returns its index.
    pub fn add_file(&mut self, path: &Path) -> usize {
        let src = Source::File(path.to_path_buf());
        match self.sources.iter().position(|s| *s == src) {
            Some(i) => i,
            None => {
                self.sources.push(src);
                self.sources.len() - 1
            }
        }
    }

    pub fn cues(&self, source: usize) -> Option<&Vec<Cue>> {
        self.cues.get(&source)
    }

    pub fn set_cues(&mut self, source: usize, cues: Vec<Cue>) {
        self.cues.insert(source, cues);
    }

    /// Remembers where line `line` (its number among the lines of the chosen source) is.
    /// The places of the dragged lines, taken out (kept for the session by the player).
    pub fn take_placed(&mut self) -> HashMap<(usize, usize), Pos> {
        std::mem::take(&mut self.placed)
    }

    /// Puts back places taken out earlier.
    pub fn set_placed(&mut self, placed: HashMap<(usize, usize), Pos>) {
        self.placed = placed;
    }

    pub fn place(&mut self, line: usize, pos: Pos) {
        if let Some(s) = self.selected {
            self.placed.insert((s, line), pos);
        }
    }

    /// The remembered place of line `line` of the chosen source.
    pub fn placed(&self, line: usize) -> Option<Pos> {
        self.placed.get(&(self.selected?, line)).copied()
    }
}

/// Reads a `.srt` file: a regular file of at most [`MAX_SRT_BYTES`], decoded leniently
/// (invalid UTF-8 becomes replacement characters), parsed by the editor's reader.
pub fn read_srt(path: &Path) -> Result<Vec<Cue>, String> {
    let name = display_name(path);
    let bytes =
        videolore::base::io::read_limited(path, MAX_SRT_BYTES).map_err(|e| match e.kind() {
            std::io::ErrorKind::InvalidData => format!("{name} is larger than 16 MiB"),
            std::io::ErrorKind::InvalidInput => format!("{name} is not a regular file"),
            _ => format!("{name} could not be read"),
        })?;
    let cues = videolore::srt::parse(&String::from_utf8_lossy(&bytes));
    if cues.is_empty() {
        return Err(format!("{name} has no subtitles"));
    }
    Ok(cues)
}

/// The cues as lines of media `media`, in source time (the clip is the whole file).
pub fn to_lines(media: MediaId, cues: &[Cue]) -> Vec<NewLine> {
    cues.iter()
        .map(|c| NewLine {
            media,
            start: c.start,
            end: c.end,
            text: c.text.clone(),
            words: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use videolore::media::TextCodec;

    fn track(ordinal: u8, lang: Option<&str>, title: Option<&str>) -> SubtitleTrack {
        SubtitleTrack {
            ordinal,
            codec: TextCodec::Srt,
            language: lang.map(str::to_string),
            title: title.map(str::to_string),
        }
    }

    // covers: REQ-140
    #[test]
    fn sources_are_named_from_the_file_and_files_are_added_once() {
        let tracks = [
            track(0, Some("eng"), None),
            track(1, Some("ita"), Some("Forced")),
            track(2, None, None),
        ];
        let mut s = Subs::for_tracks(&tracks);
        let names: Vec<String> = s.sources.iter().map(Source::name).collect();
        assert_eq!(names, ["English", "Italian — Forced", "Track 3"]);
        let i = s.add_file(Path::new("/x/film.srt"));
        assert_eq!(i, 3);
        assert_eq!(s.sources[i].name(), "film.srt");
        assert_eq!(s.add_file(Path::new("/x/film.srt")), 3, "once per path");
        assert_eq!(s.sources.len(), 4);
        assert!(s.selected.is_none());
    }

    // covers: REQ-140
    #[test]
    fn positions_are_kept_per_source_and_line_for_the_session() {
        let mut s = Subs::for_tracks(&[track(0, None, None), track(1, None, None)]);
        s.place(0, Pos { x: 1, y: 2 }); // nothing chosen: ignored
        assert_eq!(s.placed(0), None);
        s.selected = Some(0);
        s.place(0, Pos { x: 100, y: 200 });
        assert_eq!(s.placed(0), Some(Pos { x: 100, y: 200 }));
        s.selected = Some(1);
        assert_eq!(s.placed(0), None);
        s.selected = Some(0);
        assert_eq!(s.placed(0), Some(Pos { x: 100, y: 200 }));
        s.set_cues(0, vec![]);
        assert!(s.cues(0).is_some() && s.cues(1).is_none());
    }

    // covers: REQ-140
    #[test]
    fn an_srt_file_is_read_with_limits() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.srt");
        std::fs::write(&p, "1\n00:00:01,000 --> 00:00:02,500\n<i>Hello</i>\n\n").unwrap();
        let cues = read_srt(&p).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Hello");
        let lines = to_lines(MediaId(1), &cues);
        assert_eq!((lines[0].start.0, lines[0].end.0), (1_000_000, 2_500_000));
        // bad UTF-8 is read leniently; an empty or damaged file, a folder and a missing
        // file are refused with a message
        std::fs::write(&p, b"1\n00:00:01,000 --> 00:00:02,000\nCaf\xe9\n\n").unwrap();
        assert!(read_srt(&p).unwrap()[0].text.starts_with("Caf"));
        std::fs::write(&p, "nothing here").unwrap();
        assert!(read_srt(&p).unwrap_err().contains("no subtitles"));
        assert!(
            read_srt(d.path())
                .unwrap_err()
                .contains("not a regular file")
        );
        assert!(
            read_srt(&d.path().join("none.srt"))
                .unwrap_err()
                .contains("could not be read")
        );
        let big = d.path().join("big.srt");
        std::fs::File::create(&big)
            .unwrap()
            .set_len(MAX_SRT_BYTES + 1)
            .unwrap();
        assert!(read_srt(&big).unwrap_err().contains("larger"));
    }
}
