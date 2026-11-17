// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The single video track: a contiguous list of clips (no gaps, no overlaps) plus the set of
//! boundaries that carry a fade. Every edit is a method returning `Result<_, EditError>`.

use crate::media::{MediaId, MediaItem};
use crate::time::Micros;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Longest fade at a boundary.
pub const MAX_FADE: Micros = Micros(500_000);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct ClipId(pub u64);

/// Full volume of a clip's own sound (percent).
pub const FULL_VOLUME: u8 = 100;

fn full_volume() -> u8 {
    FULL_VOLUME
}

/// A slice `[src_in, src_out)` of a media item placed on the track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub media: MediaId,
    pub src_in: Micros,
    pub src_out: Micros,
    /// Level of the clip's own sound, 0–100 % (1.2.0; absent = 100).
    #[serde(default = "full_volume")]
    pub volume: u8,
    /// The clip's own sound is muted (1.2.0; absent = not muted).
    #[serde(default)]
    pub muted: bool,
}

impl Clip {
    /// A clip at full volume, not muted.
    pub fn new(id: ClipId, media: MediaId, src_in: Micros, src_out: Micros) -> Clip {
        Clip {
            id,
            media,
            src_in,
            src_out,
            volume: FULL_VOLUME,
            muted: false,
        }
    }

    /// True when the clip's sound is changed (level below 100 % or muted).
    pub fn has_volume(&self) -> bool {
        self.volume != FULL_VOLUME || self.muted
    }

    pub fn len(&self) -> Micros {
        self.src_out - self.src_in
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= Micros::ZERO
    }

    /// Length of the fade applied at either end of this clip.
    pub fn fade_len(&self) -> Micros {
        MAX_FADE.min(Micros(self.len().0 / 2))
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum EditError {
    #[error("no such clip")]
    NoSuchClip,
    #[error("the playhead is not inside a clip")]
    NotInsideClip,
    #[error("'{0}' has no video: audio files go on the music track")]
    NoVideo(String),
    #[error("'{0}' is missing")]
    MissingMedia(String),
    #[error("these clips cannot be merged: they must be adjacent parts of the same file")]
    NotMergeable,
    #[error("invalid range")]
    InvalidRange,
    #[error("no boundary there")]
    NoSuchBoundary,
    #[error("the clipboard is empty")]
    EmptyClipboard,
    #[error("no clip selected")]
    NothingSelected,
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error("the timeline is empty")]
    EmptyTimeline,
}

/// Why two clips cannot be merged although they are parts of the same file.
pub const DIFFERENT_VOLUMES: &str = "The clips have different volumes";

/// The video track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Timeline {
    pub clips: Vec<Clip>,
    /// Boundaries with a fade, as (left clip, right clip). Only adjacent pairs are kept.
    pub fades: BTreeSet<(ClipId, ClipId)>,
    pub next_clip_id: u64,
    /// Fade from black at the very start of the video (1.2.0). Attached to the video's
    /// start, not to a clip: it stays there whatever is edited later.
    #[serde(default)]
    pub start_fade: bool,
    /// Fade to black at the very end of the video (1.2.0).
    #[serde(default)]
    pub end_fade: bool,
}

impl Timeline {
    fn new_id(&mut self) -> ClipId {
        self.next_clip_id += 1;
        ClipId(self.next_clip_id)
    }

    pub fn duration(&self) -> Micros {
        self.clips.iter().fold(Micros::ZERO, |a, c| a + c.len())
    }

    pub fn index_of(&self, id: ClipId) -> Option<usize> {
        self.clips.iter().position(|c| c.id == id)
    }

    pub fn get(&self, id: ClipId) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == id)
    }

    /// Timeline start of clip `index`.
    pub fn start_of(&self, index: usize) -> Micros {
        self.clips[..index.min(self.clips.len())]
            .iter()
            .fold(Micros::ZERO, |a, c| a + c.len())
    }

    /// Clip containing `t` (`start <= t < end`) and its start.
    pub fn locate(&self, t: Micros) -> Option<(usize, Micros)> {
        let mut start = Micros::ZERO;
        for (i, c) in self.clips.iter().enumerate() {
            let end = start + c.len();
            if t >= start && t < end {
                return Some((i, start));
            }
            start = end;
        }
        None
    }

    /// Whether boundary `b` (between clips `b` and `b + 1`) has a fade.
    pub fn has_fade(&self, b: usize) -> bool {
        match (self.clips.get(b), self.clips.get(b + 1)) {
            (Some(a), Some(n)) => self.fades.contains(&(a.id, n.id)),
            _ => false,
        }
    }

    /// Fade-in and fade-out lengths for clip `index` (zero when absent). The first clip
    /// fades in with the start fade, the last one fades out with the end fade.
    pub fn fades_of(&self, index: usize) -> (Micros, Micros) {
        let Some(c) = self.clips.get(index) else {
            return (Micros::ZERO, Micros::ZERO);
        };
        let last = index + 1 == self.clips.len();
        let fin = if (index > 0 && self.has_fade(index - 1)) || (index == 0 && self.start_fade) {
            c.fade_len()
        } else {
            Micros::ZERO
        };
        let fout = if self.has_fade(index) || (last && self.end_fade) {
            c.fade_len()
        } else {
            Micros::ZERO
        };
        (fin, fout)
    }

    /// Drops fades whose clips are no longer adjacent.
    fn prune_fades(&mut self) {
        let pairs: BTreeSet<(ClipId, ClipId)> =
            self.clips.windows(2).map(|w| (w[0].id, w[1].id)).collect();
        self.fades.retain(|p| pairs.contains(p));
    }

    fn rename_fade_left(&mut self, old: ClipId, new: ClipId) {
        let moved: Vec<_> = self.fades.iter().filter(|p| p.0 == old).copied().collect();
        for p in moved {
            self.fades.remove(&p);
            self.fades.insert((new, p.1));
        }
    }

    /// Appends a whole media item at the end of the track.
    pub fn append(&mut self, media: &MediaItem) -> Result<ClipId, EditError> {
        if media.missing {
            return Err(EditError::MissingMedia(media.name.clone()));
        }
        if !media.has_video() {
            return Err(EditError::NoVideo(media.name.clone()));
        }
        if media.duration <= Micros::ZERO {
            return Err(EditError::InvalidRange);
        }
        let id = self.new_id();
        self.clips
            .push(Clip::new(id, media.id, Micros::ZERO, media.duration));
        Ok(id)
    }

    /// Splits the clip under `at` into two adjacent clips. Both halves must be at least
    /// `min_len` long. Returns the id of the right half.
    pub fn split(&mut self, at: Micros, min_len: Micros) -> Result<ClipId, EditError> {
        let (i, start) = self.locate(at).ok_or(EditError::NotInsideClip)?;
        let c = self.clips[i];
        let offset = at - start;
        if offset < min_len.max(Micros(1)) || c.len() - offset < min_len.max(Micros(1)) {
            return Err(EditError::NotInsideClip);
        }
        let id = self.new_id();
        let cut = c.src_in + offset;
        self.clips[i].src_out = cut;
        // the right half keeps the clip's sound settings
        self.clips.insert(
            i + 1,
            Clip {
                id,
                src_in: cut,
                ..c
            },
        );
        self.rename_fade_left(c.id, id);
        self.prune_fades();
        Ok(id)
    }

    /// Removes a clip (cut / delete); following clips ripple left.
    pub fn remove(&mut self, id: ClipId) -> Result<Clip, EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        let c = self.clips.remove(i);
        self.prune_fades();
        Ok(c)
    }

    /// Inserts a copy of `template` (fresh id) at `at`; a clip under `at` is split first.
    /// Everything after the insertion point ripples right.
    pub fn insert_at(
        &mut self,
        template: &Clip,
        at: Micros,
        min_len: Micros,
    ) -> Result<ClipId, EditError> {
        if template.is_empty() {
            return Err(EditError::InvalidRange);
        }
        let index = match self.locate(at) {
            None => self.clips.len(),
            Some((i, start)) if at == start => i,
            Some((i, start)) => {
                let c = self.clips[i];
                let offset = at - start;
                if offset < min_len || c.len() - offset < min_len {
                    // Too close to an edge to split: insert at the nearest boundary.
                    if offset.0 * 2 < c.len().0 { i } else { i + 1 }
                } else {
                    self.split(at, min_len)?;
                    i + 1
                }
            }
        };
        let id = self.new_id();
        self.clips.insert(index, Clip { id, ..*template });
        self.prune_fades();
        Ok(id)
    }

    /// True when clip `id` and the next one come from the same file, are contiguous in it
    /// and have the same sound settings.
    pub fn can_merge(&self, id: ClipId) -> bool {
        self.contiguous_with_next(id) && self.merge_block_reason(id).is_none()
    }

    fn contiguous_with_next(&self, id: ClipId) -> bool {
        match self.index_of(id) {
            Some(i) if i + 1 < self.clips.len() => {
                let (a, b) = (&self.clips[i], &self.clips[i + 1]);
                a.media == b.media && a.src_out == b.src_in
            }
            _ => false,
        }
    }

    /// Why two parts of the same file next to each other cannot be merged (their volumes
    /// differ); `None` otherwise.
    pub fn merge_block_reason(&self, id: ClipId) -> Option<&'static str> {
        if !self.contiguous_with_next(id) {
            return None;
        }
        let i = self.index_of(id)?;
        let (a, b) = (&self.clips[i], &self.clips[i + 1]);
        ((a.volume, a.muted) != (b.volume, b.muted)).then_some(DIFFERENT_VOLUMES)
    }

    /// Merges clip `id` with the next one (see [`Self::can_merge`]).
    pub fn merge(&mut self, id: ClipId) -> Result<(), EditError> {
        if !self.can_merge(id) {
            return Err(EditError::NotMergeable);
        }
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        let b = self.clips.remove(i + 1);
        self.clips[i].src_out = b.src_out;
        self.fades.remove(&(id, b.id));
        self.rename_fade_left(b.id, id);
        self.prune_fades();
        Ok(())
    }

    /// Sets the source in/out points, clamped to `[0, media_duration]`.
    pub fn trim(
        &mut self,
        id: ClipId,
        src_in: Micros,
        src_out: Micros,
        media_duration: Micros,
        min_len: Micros,
    ) -> Result<(), EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        let new_in = src_in.clamp(Micros::ZERO, media_duration);
        let new_out = src_out.clamp(Micros::ZERO, media_duration);
        if new_out - new_in < min_len.max(Micros(1)) {
            return Err(EditError::InvalidRange);
        }
        self.clips[i].src_in = new_in;
        self.clips[i].src_out = new_out;
        Ok(())
    }

    /// Moves a clip to position `index` (clamped); the others ripple.
    pub fn move_to(&mut self, id: ClipId, index: usize) -> Result<(), EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        let c = self.clips.remove(i);
        let index = index.min(self.clips.len());
        self.clips.insert(index, c);
        self.prune_fades();
        Ok(())
    }

    /// Adds or removes the fade at boundary `b`. Returns whether the fade is now present.
    pub fn toggle_fade(&mut self, b: usize) -> Result<bool, EditError> {
        let (a, n) = match (self.clips.get(b), self.clips.get(b + 1)) {
            (Some(a), Some(n)) => (a.id, n.id),
            _ => return Err(EditError::NoSuchBoundary),
        };
        if self.fades.remove(&(a, n)) {
            Ok(false)
        } else {
            self.fades.insert((a, n));
            Ok(true)
        }
    }

    /// Applies (`on`) or removes "Fade IN/OUT" on a clip: the fades at both of its
    /// edges (the boundaries with its neighbours, the start fade when it is the first clip,
    /// the end fade when it is the last).
    pub fn set_clip_fades(&mut self, id: ClipId, on: bool) -> Result<(), EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        let mut set = |pair: (ClipId, ClipId)| {
            if on {
                self.fades.insert(pair);
            } else {
                self.fades.remove(&pair);
            }
        };
        if i > 0 {
            set((self.clips[i - 1].id, id));
        }
        if i + 1 < self.clips.len() {
            set((id, self.clips[i + 1].id));
        }
        if i == 0 {
            self.start_fade = on;
        }
        if i + 1 == self.clips.len() {
            self.end_fade = on;
        }
        Ok(())
    }

    /// True when every fade of [`Self::set_clip_fades`] is there for clip `id`.
    pub fn clip_has_fades(&self, id: ClipId) -> bool {
        let Some(i) = self.index_of(id) else {
            return false;
        };
        let left = if i == 0 {
            self.start_fade
        } else {
            self.has_fade(i - 1)
        };
        let right = if i + 1 == self.clips.len() {
            self.end_fade
        } else {
            self.has_fade(i)
        };
        left && right
    }

    /// Fade from black at the very start and to black at the very end of the video.
    pub fn set_edge_fades(&mut self, on: bool) -> Result<(), EditError> {
        if self.clips.is_empty() {
            return Err(EditError::EmptyTimeline);
        }
        self.start_fade = on;
        self.end_fade = on;
        Ok(())
    }

    /// True when any fade (boundary, start or end) is set.
    pub fn has_any_fade(&self) -> bool {
        !self.fades.is_empty() || self.start_fade || self.end_fade
    }

    /// Removes every fade (boundaries, start and end).
    pub fn clear_fades(&mut self) {
        self.fades.clear();
        self.start_fade = false;
        self.end_fade = false;
    }

    /// Sets a clip's sound level (0–100 %, clamped) and mute.
    pub fn set_clip_volume(
        &mut self,
        id: ClipId,
        volume: u8,
        muted: bool,
    ) -> Result<(), EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        self.clips[i].volume = volume.min(FULL_VOLUME);
        self.clips[i].muted = muted;
        Ok(())
    }

    /// Boundary closest to `t` (index and time), if any.
    pub fn nearest_boundary(&self, t: Micros) -> Option<(usize, Micros)> {
        let mut start = Micros::ZERO;
        let mut best: Option<(usize, Micros)> = None;
        // Every clip but the last ends on a boundary.
        let inner = self.clips.len().saturating_sub(1);
        for (i, clip) in self.clips[..inner].iter().enumerate() {
            start = start + clip.len();
            let better = best.is_none_or(|(_, bt)| (start - t).0.abs() < (bt - t).0.abs());
            if better {
                best = Some((i, start));
            }
        }
        best
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::media::{MediaFormat, MediaItem};
    use std::path::PathBuf;

    pub fn media(id: u64, secs: i64, video: bool) -> MediaItem {
        MediaItem {
            id: MediaId(id),
            path: PathBuf::from(format!("/m/{id}.mp4")),
            name: format!("{id}.mp4"),
            duration: Micros(secs * 1_000_000),
            format: MediaFormat {
                container: "mp4".into(),
                vcodec: video.then(|| "h264".into()),
                acodec: Some("aac".into()),
                width: 1280,
                height: 720,
                fps: crate::time::Fps::new(30, 1),
                ..Default::default()
            },
            missing: false,
            status: Default::default(),
            proxy: Default::default(),
        }
    }

    const F: Micros = Micros(33_333);

    fn s(n: i64) -> Micros {
        Micros(n * 1_000_000)
    }

    fn track() -> Timeline {
        let mut t = Timeline::default();
        t.append(&media(1, 4, true)).unwrap();
        t.append(&media(2, 6, true)).unwrap();
        t
    }

    // covers: REQ-005
    #[test]
    fn append_places_clips_end_to_end() {
        let t = track();
        assert_eq!(t.clips.len(), 2);
        assert_eq!(t.duration(), s(10));
        assert_eq!(t.start_of(1), s(4));
        assert_eq!(t.locate(s(4)), Some((1, s(4))));
        assert_eq!(t.locate(s(10)), None);
        // contiguous: each clip starts where the previous ends
        let mut end = Micros::ZERO;
        for (i, c) in t.clips.iter().enumerate() {
            assert_eq!(t.start_of(i), end);
            end = end + c.len();
        }
    }

    // covers: REQ-002
    #[test]
    fn audio_only_cannot_be_placed() {
        let mut t = Timeline::default();
        let e = t.append(&media(3, 2, false)).unwrap_err();
        assert_eq!(e, EditError::NoVideo("3.mp4".into()));
        let mut m = media(4, 2, true);
        m.missing = true;
        assert!(matches!(t.append(&m), Err(EditError::MissingMedia(_))));
        let mut z = media(5, 0, true);
        z.duration = Micros::ZERO;
        assert_eq!(t.append(&z), Err(EditError::InvalidRange));
    }

    // covers: REQ-006
    #[test]
    fn split_keeps_total_length() {
        let mut t = track();
        let right = t.split(s(5), F).unwrap();
        assert_eq!(t.clips.len(), 3);
        assert_eq!(t.duration(), s(10));
        assert_eq!(t.clips[1].src_out, s(1));
        assert_eq!(t.clips[2].id, right);
        assert_eq!(t.clips[2].src_in, s(1));
        assert_eq!(t.split(s(4), F), Err(EditError::NotInsideClip));
        assert_eq!(t.split(s(99), F), Err(EditError::NotInsideClip));
        assert_eq!(t.split(Micros(10), F), Err(EditError::NotInsideClip));
    }

    // covers: REQ-007
    #[test]
    fn cut_and_paste_ripple() {
        let mut t = track();
        let first = t.clips[0].id;
        let c = t.remove(first).unwrap();
        assert_eq!(t.duration(), s(6));
        assert_eq!(t.remove(first), Err(EditError::NoSuchClip));
        // paste at the end
        let id = t.insert_at(&c, s(6), F).unwrap();
        assert_eq!(t.clips.last().unwrap().id, id);
        // paste at a boundary (start)
        t.insert_at(&c, Micros::ZERO, F).unwrap();
        assert_eq!(t.clips[0].media, MediaId(1));
        assert_eq!(t.duration(), s(14));
        // paste in the middle of a clip splits it and ripples right
        let before = t.duration();
        let id = t.insert_at(&c, s(6), F).unwrap();
        assert_eq!(t.duration(), before + c.len());
        let i = t.index_of(id).unwrap();
        assert_eq!(t.start_of(i), s(6));
        // too close to an edge: snaps to the nearest boundary
        let n = t.clips.len();
        t.insert_at(&c, Micros(10), F).unwrap();
        assert_eq!(t.clips.len(), n + 1);
        assert_eq!(t.clips[0].media, MediaId(1));
        let end_of_first = t.start_of(1);
        t.insert_at(&c, end_of_first - Micros(10), F).unwrap();
        let empty = Clip::new(ClipId(0), MediaId(1), s(1), s(1));
        assert_eq!(t.insert_at(&empty, s(0), F), Err(EditError::InvalidRange));
    }

    // covers: REQ-008
    #[test]
    fn merge_only_contiguous_same_source() {
        let mut t = track();
        let a = t.clips[0].id;
        assert!(!t.can_merge(a));
        assert_eq!(t.merge(a), Err(EditError::NotMergeable));
        let right = t.split(s(2), F).unwrap();
        assert!(t.can_merge(a));
        assert!(!t.can_merge(right)); // next clip is another file
        t.toggle_fade(0).unwrap();
        t.toggle_fade(1).unwrap();
        t.merge(a).unwrap();
        assert_eq!(t.clips.len(), 2);
        assert_eq!(t.clips[0].src_out, s(4));
        assert!(t.has_fade(0)); // fade after the merged clip survives
        assert!(!t.can_merge(ClipId(999)));
        // not contiguous in source: trim makes a gap
        let r = t.split(s(2), F).unwrap();
        t.trim(r, s(3), s(4), s(4), F).unwrap();
        assert!(!t.can_merge(a));
    }

    // covers: REQ-009
    #[test]
    fn trim_is_clamped_to_source() {
        let mut t = track();
        let a = t.clips[0].id;
        t.trim(a, Micros(-5), s(99), s(4), F).unwrap();
        assert_eq!((t.clips[0].src_in, t.clips[0].src_out), (s(0), s(4)));
        t.trim(a, s(1), s(3), s(4), F).unwrap();
        assert_eq!(t.clips[0].len(), s(2));
        assert_eq!(t.duration(), s(8));
        assert_eq!(t.trim(a, s(3), s(3), s(4), F), Err(EditError::InvalidRange));
        assert_eq!(
            t.trim(ClipId(77), s(0), s(1), s(4), F),
            Err(EditError::NoSuchClip)
        );
    }

    // covers: REQ-010
    #[test]
    fn move_reorders_with_ripple() {
        let mut t = track();
        let a = t.clips[0].id;
        t.toggle_fade(0).unwrap();
        t.move_to(a, 5).unwrap();
        assert_eq!(t.clips[1].id, a);
        assert_eq!(t.start_of(1), s(6));
        assert!(t.fades.is_empty()); // the boundary no longer exists
        assert_eq!(t.move_to(ClipId(42), 0), Err(EditError::NoSuchClip));
    }

    // covers: REQ-011
    #[test]
    fn fade_toggle_and_lengths() {
        let mut t = track();
        assert!(t.toggle_fade(0).unwrap());
        assert!(t.has_fade(0));
        assert_eq!(t.fades_of(0), (Micros::ZERO, MAX_FADE));
        assert_eq!(t.fades_of(1), (MAX_FADE, Micros::ZERO));
        assert_eq!(t.fades_of(9), (Micros::ZERO, Micros::ZERO));
        assert_eq!(t.duration(), s(10)); // clips are not shortened
        assert!(!t.toggle_fade(0).unwrap());
        assert!(!t.has_fade(0));
        assert_eq!(t.toggle_fade(1), Err(EditError::NoSuchBoundary));
        let short = Clip::new(ClipId(1), MediaId(1), s(0), Micros(600_000));
        assert_eq!(short.fade_len(), Micros(300_000));
        // split keeps the fade on the right half's boundary
        t.toggle_fade(0).unwrap();
        let right = t.split(s(2), F).unwrap();
        assert!(t.fades.contains(&(right, t.clips[2].id)));
    }

    #[test]
    fn nearest_boundary() {
        let t = track();
        assert_eq!(t.nearest_boundary(s(1)), Some((0, s(4))));
        let mut t3 = track();
        t3.append(&media(3, 2, true)).unwrap();
        assert_eq!(t3.nearest_boundary(s(9)), Some((1, s(10))));
        assert_eq!(Timeline::default().nearest_boundary(s(1)), None);
    }

    fn three() -> Timeline {
        let mut t = track();
        t.append(&media(3, 5, true)).unwrap();
        t
    }

    // covers: REQ-083, REQ-011
    #[test]
    fn fade_on_a_clip_sets_both_of_its_edges() {
        // (a) the middle clip: its two boundaries, nothing at the start or end
        let mut t = three();
        let ids: Vec<ClipId> = t.clips.iter().map(|c| c.id).collect();
        t.set_clip_fades(ids[1], true).unwrap();
        assert!(t.has_fade(0) && t.has_fade(1));
        assert!(!t.start_fade && !t.end_fade);
        assert!(t.clip_has_fades(ids[1]));
        assert!(!t.clip_has_fades(ids[0]));
        // removing it from the clip removes both edges
        t.set_clip_fades(ids[1], false).unwrap();
        assert!(!t.has_any_fade());
        // (b) the first clip: fade from black at the start + its boundary
        t.set_clip_fades(ids[0], true).unwrap();
        assert!(t.start_fade && t.has_fade(0) && !t.has_fade(1) && !t.end_fade);
        assert_eq!(t.fades_of(0), (MAX_FADE, MAX_FADE));
        // the last clip: its boundary + fade to black at the end
        t.set_clip_fades(ids[2], true).unwrap();
        assert!(t.end_fade && t.has_fade(1));
        assert_eq!(t.fades_of(2), (MAX_FADE, MAX_FADE));
        assert_eq!(
            t.set_clip_fades(ClipId(99), true),
            Err(EditError::NoSuchClip)
        );
        assert!(!t.clip_has_fades(ClipId(99)));
        t.clear_fades();
        assert!(!t.has_any_fade());
    }

    // covers: REQ-083, REQ-011
    #[test]
    fn whole_video_fades_stay_at_the_video_edges() {
        let mut t = three();
        assert_eq!(
            Timeline::default().set_edge_fades(true),
            Err(EditError::EmptyTimeline)
        );
        t.set_edge_fades(true).unwrap();
        assert!(t.fades.is_empty(), "no boundary fades");
        assert_eq!(t.fades_of(0), (MAX_FADE, Micros::ZERO));
        assert_eq!(t.fades_of(2), (Micros::ZERO, MAX_FADE));
        // cutting the first clip: the new first clip fades in from black
        let first = t.clips[0].id;
        t.remove(first).unwrap();
        assert_eq!(t.fades_of(0).0, MAX_FADE);
        // moving the first clip to the end: the start fade stays at the start
        let first = t.clips[0].id;
        t.move_to(first, 9).unwrap();
        assert_eq!(t.fades_of(0).0, MAX_FADE);
        assert_eq!(t.fades_of(1), (Micros::ZERO, MAX_FADE));
        // a clip shorter than 1 s fades over half its length
        let mut short = Timeline::default();
        short.append(&media(1, 4, true)).unwrap();
        let id = short.clips[0].id;
        short.trim(id, s(0), Micros(600_000), s(4), F).unwrap();
        short.set_edge_fades(true).unwrap();
        assert_eq!(short.fades_of(0), (Micros(300_000), Micros(300_000)));
        t.set_edge_fades(false).unwrap();
        assert!(!t.start_fade && !t.end_fade);
    }

    // covers: REQ-084, REQ-020, REQ-008
    #[test]
    fn clip_volume_follows_split_copy_and_reorder_and_blocks_merge() {
        let mut t = track();
        let a = t.clips[0].id;
        assert!(!t.clips[0].has_volume());
        t.set_clip_volume(a, 250, false).unwrap();
        assert_eq!(t.clips[0].volume, 100, "never above 100%");
        t.set_clip_volume(a, 50, true).unwrap();
        assert!(t.clips[0].has_volume());
        assert_eq!(
            t.set_clip_volume(ClipId(77), 1, false),
            Err(EditError::NoSuchClip)
        );
        // (c) split keeps the level on both halves; the halves can be merged back
        let right = t.split(s(2), F).unwrap();
        assert_eq!((t.clips[0].volume, t.clips[0].muted), (50, true));
        let r = *t.get(right).unwrap();
        assert_eq!((r.volume, r.muted), (50, true));
        assert!(t.can_merge(a));
        // a copy keeps it, so does a move
        let id = t.insert_at(&r, t.duration(), F).unwrap();
        assert_eq!(t.get(id).unwrap().volume, 50);
        t.move_to(id, 0).unwrap();
        assert_eq!(t.clips[0].volume, 50);
        // different volumes: no merge, with the reason
        t.set_clip_volume(right, 80, true).unwrap();
        assert!(!t.can_merge(a));
        assert_eq!(t.merge_block_reason(a), Some(DIFFERENT_VOLUMES));
        assert_eq!(t.merge(a), Err(EditError::NotMergeable));
        assert_eq!(
            t.merge_block_reason(right),
            None,
            "not contiguous: other reason"
        );
        t.set_clip_volume(right, 50, true).unwrap();
        t.merge(a).unwrap();
    }

    // covers: REQ-094
    #[test]
    fn clips_and_timelines_of_1_1_read_with_defaults() {
        let c: Clip = serde_json::from_str(r#"{"id":1,"media":2,"src_in":0,"src_out":5}"#).unwrap();
        assert_eq!((c.volume, c.muted), (100, false));
        let t: Timeline =
            serde_json::from_str(r#"{"clips":[],"fades":[],"next_clip_id":0}"#).unwrap();
        assert!(!t.start_fade && !t.end_fade);
    }
}
