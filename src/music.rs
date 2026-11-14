// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The music track (below the video track) and the audio mix.
//!
//! Music clips are placed at absolute timeline times, are kept sorted and never overlap.
//! Unlike video clips they do not ripple: editing the video never moves the music.
//! Only audio files (no video stream) can be placed here.

use crate::media::{MediaId, MediaItem};
use crate::time::Micros;
use serde::{Deserialize, Serialize};

/// Length of the fade-out applied where the video ends while music is still playing.
pub const END_FADE: Micros = Micros(500_000);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct MusicClipId(pub u64);

/// The slice `[src_in, src_out)` of an audio file, playing from timeline time `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicClip {
    pub id: MusicClipId,
    pub media: MediaId,
    pub src_in: Micros,
    pub src_out: Micros,
    pub start: Micros,
}

impl MusicClip {
    pub fn len(&self) -> Micros {
        self.src_out - self.src_in
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= Micros::ZERO
    }

    pub fn end(&self) -> Micros {
        self.start + self.len()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum MusicError {
    #[error("only audio files can go on the music track ('{0}' has video)")]
    NotAudio(String),
    #[error("'{0}' is missing")]
    Missing(String),
    #[error("music clips cannot overlap")]
    Overlap,
    #[error("no such music clip")]
    NoSuchClip,
    #[error("invalid range")]
    InvalidRange,
    #[error("the playhead is not inside a music clip")]
    NotInsideClip,
}

/// The single music track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MusicTrack {
    /// Sorted by `start`, never overlapping.
    pub clips: Vec<MusicClip>,
    pub next_id: u64,
}

impl MusicTrack {
    fn new_id(&mut self) -> MusicClipId {
        self.next_id += 1;
        MusicClipId(self.next_id)
    }

    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    pub fn get(&self, id: MusicClipId) -> Option<&MusicClip> {
        self.clips.iter().find(|c| c.id == id)
    }

    fn index_of(&self, id: MusicClipId) -> Result<usize, MusicError> {
        self.clips
            .iter()
            .position(|c| c.id == id)
            .ok_or(MusicError::NoSuchClip)
    }

    /// The clip playing at `t`.
    pub fn at(&self, t: Micros) -> Option<&MusicClip> {
        self.clips.iter().find(|c| c.start <= t && t < c.end())
    }

    /// End of the last clip.
    pub fn end(&self) -> Micros {
        self.clips
            .iter()
            .map(MusicClip::end)
            .max()
            .unwrap_or(Micros::ZERO)
    }

    fn overlaps(&self, start: Micros, end: Micros, except: Option<MusicClipId>) -> bool {
        self.clips
            .iter()
            .filter(|c| Some(c.id) != except)
            .any(|c| start < c.end() && c.start < end)
    }

    fn sort(&mut self) {
        self.clips.sort_by_key(|c| (c.start, c.id));
    }

    /// Places `media` at `at`. When the next clip is closer than the file's length, the new
    /// clip is shortened to fit the gap (at least `min_len`).
    pub fn place(
        &mut self,
        media: &MediaItem,
        at: Micros,
        min_len: Micros,
    ) -> Result<MusicClipId, MusicError> {
        if media.missing {
            return Err(MusicError::Missing(media.name.clone()));
        }
        if media.has_video() || !media.format.has_audio() {
            return Err(MusicError::NotAudio(media.name.clone()));
        }
        let at = at.max(Micros::ZERO);
        if self.at(at).is_some() {
            return Err(MusicError::Overlap);
        }
        let room = self
            .clips
            .iter()
            .filter(|c| c.start > at)
            .map(|c| c.start - at)
            .min()
            .unwrap_or(Micros(i64::MAX));
        let len = media.duration.min(room);
        if len < min_len.max(Micros(1)) {
            return Err(MusicError::Overlap);
        }
        let id = self.new_id();
        self.clips.push(MusicClip {
            id,
            media: media.id,
            src_in: Micros::ZERO,
            src_out: len,
            start: at,
        });
        self.sort();
        Ok(id)
    }

    /// Moves a clip so that it starts at `start` (never before 0, never over another clip).
    pub fn move_to(&mut self, id: MusicClipId, start: Micros) -> Result<(), MusicError> {
        let i = self.index_of(id)?;
        let start = start.max(Micros::ZERO);
        let end = start + self.clips[i].len();
        if self.overlaps(start, end, Some(id)) {
            return Err(MusicError::Overlap);
        }
        self.clips[i].start = start;
        self.sort();
        Ok(())
    }

    /// Changes the source range. Trimming the head keeps the remaining audio where it was
    /// on the timeline (the clip's left edge moves).
    pub fn trim(
        &mut self,
        id: MusicClipId,
        src_in: Micros,
        src_out: Micros,
        media_duration: Micros,
        min_len: Micros,
    ) -> Result<(), MusicError> {
        let i = self.index_of(id)?;
        let c = self.clips[i];
        let new_in = src_in.clamp(Micros::ZERO, media_duration);
        let new_out = src_out.clamp(Micros::ZERO, media_duration);
        if new_out - new_in < min_len.max(Micros(1)) {
            return Err(MusicError::InvalidRange);
        }
        let start = c.start + (new_in - c.src_in);
        if start < Micros::ZERO {
            return Err(MusicError::InvalidRange);
        }
        if self.overlaps(start, start + (new_out - new_in), Some(id)) {
            return Err(MusicError::Overlap);
        }
        let c = &mut self.clips[i];
        c.src_in = new_in;
        c.src_out = new_out;
        c.start = start;
        self.sort();
        Ok(())
    }

    /// Splits the clip playing at `at`. Returns the id of the right half.
    pub fn split(&mut self, at: Micros, min_len: Micros) -> Result<MusicClipId, MusicError> {
        let i = self
            .clips
            .iter()
            .position(|c| c.start <= at && at < c.end())
            .ok_or(MusicError::NotInsideClip)?;
        let c = self.clips[i];
        let offset = at - c.start;
        let min = min_len.max(Micros(1));
        if offset < min || c.len() - offset < min {
            return Err(MusicError::NotInsideClip);
        }
        let id = self.new_id();
        self.clips[i].src_out = c.src_in + offset;
        self.clips.push(MusicClip {
            id,
            media: c.media,
            src_in: c.src_in + offset,
            src_out: c.src_out,
            start: at,
        });
        self.sort();
        Ok(id)
    }

    pub fn remove(&mut self, id: MusicClipId) -> Result<MusicClip, MusicError> {
        let i = self.index_of(id)?;
        Ok(self.clips.remove(i))
    }
}

/// Volumes (0–100 %) and mutes of the two audio sources. Saved in the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mix {
    pub video_volume: u8,
    pub video_muted: bool,
    pub music_volume: u8,
    pub music_muted: bool,
}

impl Default for Mix {
    fn default() -> Self {
        Mix {
            video_volume: 100,
            video_muted: false,
            music_volume: 100,
            music_muted: false,
        }
    }
}

/// Linear gain for a volume slider: (p/100)² (50 % is about -12 dB); muted is silence.
pub fn gain(percent: u8, muted: bool) -> f64 {
    if muted {
        return 0.0;
    }
    let p = f64::from(percent.min(100)) / 100.0;
    p * p
}

impl Mix {
    pub fn video_gain(&self) -> f64 {
        gain(self.video_volume, self.video_muted)
    }

    pub fn music_gain(&self) -> f64 {
        gain(self.music_volume, self.music_muted)
    }

    /// True when the video's own sound is left untouched.
    pub fn video_neutral(&self) -> bool {
        self.video_volume == 100 && !self.video_muted
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::timeline::tests::media;

    pub fn song(id: u64, secs: i64) -> MediaItem {
        let mut m = media(id, secs, false);
        m.name = format!("{id}.mp3");
        m.path = format!("/m/{id}.mp3").into();
        m.format.container = "mp3".into();
        m.format.acodec = Some("mp3".into());
        m.format.width = 0;
        m.format.height = 0;
        m.format.fps = None;
        m
    }

    fn s(n: i64) -> Micros {
        Micros(n * 1_000_000)
    }

    const F: Micros = Micros(33_333);

    // covers: REQ-019, REQ-002
    #[test]
    fn place_move_trim_split_delete_without_overlaps() {
        let mut t = MusicTrack::default();
        let a = t.place(&song(1, 10), s(2), F).unwrap();
        assert_eq!(t.get(a).unwrap().end(), s(12));
        // a video file cannot go on the music track
        assert_eq!(
            t.place(&media(9, 5, true), s(20), F),
            Err(MusicError::NotAudio("9.mp4".into()))
        );
        let mut gone = song(8, 5);
        gone.missing = true;
        assert!(matches!(
            t.place(&gone, s(20), F),
            Err(MusicError::Missing(_))
        ));
        // inside an existing clip: refused; before it: shortened to fit the gap
        assert_eq!(t.place(&song(2, 4), s(5), F), Err(MusicError::Overlap));
        let b = t.place(&song(2, 4), Micros::ZERO, F).unwrap();
        assert_eq!(t.get(b).unwrap().len(), s(2));
        assert_eq!(t.clips[0].id, b);
        // move: overlap refused, free spot accepted, never negative
        assert_eq!(t.move_to(b, s(3)), Err(MusicError::Overlap));
        t.move_to(b, s(20)).unwrap();
        assert_eq!(t.clips[1].id, b);
        t.move_to(a, s(-5)).unwrap();
        assert_eq!(t.get(a).unwrap().start, Micros::ZERO);
        // trim the head: the rest stays in place on the timeline
        t.trim(a, s(1), s(10), s(10), F).unwrap();
        let c = *t.get(a).unwrap();
        assert_eq!((c.start, c.src_in, c.end()), (s(1), s(1), s(10)));
        assert_eq!(t.trim(a, s(0), s(10), s(10), F), Ok(()));
        assert_eq!(
            t.trim(a, s(5), s(5), s(10), F),
            Err(MusicError::InvalidRange)
        );
        t.move_to(b, s(11)).unwrap();
        assert_eq!(t.trim(a, s(0), s(12), s(20), F), Err(MusicError::Overlap));
        // a head trim that would start before 0 is refused
        t.move_to(a, Micros::ZERO).unwrap();
        t.trim(a, s(2), s(10), s(10), F).unwrap();
        t.move_to(a, Micros::ZERO).unwrap();
        assert_eq!(
            t.trim(a, s(0), s(10), s(10), F),
            Err(MusicError::InvalidRange)
        );
        // split keeps the sound where it was
        let before = t.end();
        let right = t.split(s(4), F).unwrap();
        assert_eq!(t.clips.len(), 3);
        assert_eq!(t.get(right).unwrap().start, s(4));
        assert_eq!(t.end(), before);
        assert_eq!(t.split(s(4), F), Err(MusicError::NotInsideClip));
        assert_eq!(t.split(s(99), F), Err(MusicError::NotInsideClip));
        // delete
        t.remove(right).unwrap();
        assert_eq!(t.remove(right), Err(MusicError::NoSuchClip));
        assert_eq!(t.move_to(right, s(1)), Err(MusicError::NoSuchClip));
        assert!(!t.is_empty());
        assert!(t.at(s(1)).is_some());
        // no two clips overlap
        for w in t.clips.windows(2) {
            assert!(w[0].end() <= w[1].start);
        }
        assert!(MusicTrack::default().end() == Micros::ZERO);
        let tiny = MusicClip {
            id: MusicClipId(1),
            media: MediaId(1),
            src_in: s(1),
            src_out: s(1),
            start: s(0),
        };
        assert!(tiny.is_empty());
    }

    // covers: REQ-020
    #[test]
    fn volume_curve_and_mute() {
        assert_eq!(gain(100, false), 1.0);
        assert_eq!(gain(50, false), 0.25);
        assert_eq!(gain(0, false), 0.0);
        assert_eq!(gain(80, true), 0.0);
        assert_eq!(gain(200, false), 1.0);
        let m = Mix::default();
        assert!(m.video_neutral());
        assert_eq!((m.video_gain(), m.music_gain()), (1.0, 1.0));
        let m = Mix {
            video_volume: 50,
            music_muted: true,
            ..Mix::default()
        };
        assert!(!m.video_neutral());
        assert_eq!(m.music_gain(), 0.0);
    }
}
