// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The single video track: a contiguous list of clips (no gaps, no overlaps) plus the set of
//! boundaries that carry a fade. Every edit is a method returning `Result<_, EditError>`.

use crate::base::pos::Pos;
use crate::frame::{self, FrameAspect, Scale, Stack, Track};
use crate::keyframes::{self, Key, KeyError};
use crate::media::{MediaId, MediaItem};
use crate::overlap::Overlap;
use crate::time::Micros;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The usual length of a fade (REQ-274).
pub const MAX_FADE: Micros = Micros(500_000);
/// Shortest fade a user can choose, and the longest (a clip also limits it to half its own
/// length).
pub const MIN_FADE_LEN: Micros = Micros(100_000);
pub const MAX_FADE_LEN: Micros = Micros(10_000_000);

/// How much of each side of a picture was cut away, in whole percents. Crop was removed in
/// 2.3.0 (REQ-283): the type remains only to read a project of format 13 and count the crops
/// it drops (REQ-332); nothing writes or draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Crop {
    pub left: u8,
    pub top: u8,
    pub right: u8,
    pub bottom: u8,
}

impl Crop {
    pub fn is_none(&self) -> bool {
        *self == Crop::default()
    }
}

/// Which clips of a fade at a boundary go through black (REQ-275).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FadeSide {
    /// The clip before fades out and the one after fades in (the default).
    #[default]
    Both,
    /// Only the clip before fades out.
    OutOnly,
    /// Only the clip after fades in.
    InOnly,
}

impl FadeSide {
    pub const ALL: [FadeSide; 3] = [FadeSide::Both, FadeSide::OutOnly, FadeSide::InOnly];
}

fn is_default_len(l: &Micros) -> bool {
    *l == Micros::ZERO
}

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
    /// Background noise of the clip's own sound is reduced (1.4.0, REQ-114; absent = off).
    #[serde(default)]
    pub clean_audio: bool,
    /// How much of the clean sound is used, 0–100 % (REQ-278; absent = 100). 0 bypasses the
    /// filter.
    #[serde(default = "full_volume", skip_serializing_if = "is_full")]
    pub clean_strength: u8,
    /// Speech is made even in level ("Normalise speech", REQ-279; absent = off).
    #[serde(default, skip_serializing_if = "is_false")]
    pub speech_enhance: bool,
    /// Format 13 only: read to be dropped by the loader (REQ-332); never written.
    #[serde(default, skip_serializing)]
    pub crop: Crop,
    /// A fade in from black at this clip's start, used when the clip is the first one
    /// (1.6.1, REQ-083: it belongs to the clip; absent = none).
    #[serde(default, skip_serializing_if = "is_false")]
    pub fade_in: bool,
    /// A fade out to black at this clip's end, used when the clip is the last one.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fade_out: bool,
    /// Playing speed in thousandths: 1000 = normal, 500..2000 (REQ-281; absent = 1000).
    #[serde(default = "normal_speed", skip_serializing_if = "is_normal_speed")]
    pub speed: u16,
    /// The audio track the clip plays, its ordinal among the file's audio streams (3.1.0,
    /// REQ-340; absent = 0, the first).
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub audio_track: u8,
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

/// Normal playing speed, in thousandths.
pub const NORMAL_SPEED: u16 = 1000;
/// Slowest speed of a clip (50 %).
pub const MIN_SPEED: u16 = 500;
/// Fastest speed of a clip (200 %).
pub const MAX_SPEED: u16 = 2000;

fn normal_speed() -> u16 {
    NORMAL_SPEED
}

fn is_normal_speed(v: &u16) -> bool {
    *v == NORMAL_SPEED
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

fn is_full(v: &u8) -> bool {
    *v == FULL_VOLUME
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
            clean_audio: false,
            clean_strength: FULL_VOLUME,
            crop: Crop::default(),
            speech_enhance: false,
            fade_in: false,
            fade_out: false,
            speed: NORMAL_SPEED,
            audio_track: 0,
        }
    }

    /// True when the clip's sound is changed (level below 100 % or muted).
    pub fn has_volume(&self) -> bool {
        self.volume != FULL_VOLUME || self.muted
    }

    /// Length on the timeline: the source length divided by the speed (REQ-281).
    pub fn len(&self) -> Micros {
        self.to_timeline(self.src_len())
    }

    /// Length of the source part the clip plays.
    pub fn src_len(&self) -> Micros {
        self.src_out - self.src_in
    }

    fn speed_or_normal(&self) -> i128 {
        i128::from(self.speed.clamp(MIN_SPEED, MAX_SPEED))
    }

    /// A length of source time as time on the timeline.
    pub fn to_timeline(&self, src: Micros) -> Micros {
        if self.speed == NORMAL_SPEED {
            return src;
        }
        Micros((i128::from(src.0) * 1000 / self.speed_or_normal()) as i64)
    }

    /// A length of timeline time as source time.
    pub fn to_src(&self, tl: Micros) -> Micros {
        if self.speed == NORMAL_SPEED {
            return tl;
        }
        Micros((i128::from(tl.0) * self.speed_or_normal() / 1000) as i64)
    }

    /// Keys (in source time) moved in time so that their distance from the clip's start is
    /// timeline time: the picture effects follow the speed (REQ-281).
    pub fn keys_to_timeline<T: Copy>(&self, keys: &[Key<T>]) -> Vec<Key<T>> {
        keys.iter()
            .map(|k| Key {
                t: self.src_in + self.to_timeline(k.t - self.src_in),
                ..*k
            })
            .collect()
    }

    /// The source instant played `offset` after the clip's start on the timeline.
    pub fn src_at(&self, offset: Micros) -> Micros {
        self.src_in + self.to_src(offset)
    }

    pub fn is_empty(&self) -> bool {
        self.len() <= Micros::ZERO
    }

    /// Length of the fade applied at either end of this clip with the usual length.
    pub fn fade_len(&self) -> Micros {
        self.fade_len_of(MAX_FADE)
    }

    /// Length of the fade at either end of this clip when the chosen length is `chosen`: it
    /// never goes over half the clip (REQ-274).
    pub fn fade_len_of(&self, chosen: Micros) -> Micros {
        chosen.min(Micros(self.len().0 / 2))
    }
}

/// The edge of a clip that is dragged to trim it (REQ-191).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipEdge {
    Start,
    End,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum EditError {
    #[error("no such clip")]
    NoSuchClip,
    #[error("no such overlap")]
    NoSuchOverlap,
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
    #[error("a volume key is from 0 to 100 % inside the clip")]
    VolumeKey,
    #[error("a speed is from 50 % to 200 %")]
    SpeedRange,
    #[error("a fade lasts from 0.1 s to 10 s")]
    FadeLength,
    #[error("that boundary already has a fade")]
    BoundaryTaken,
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
    #[error("{0}")]
    Key(KeyError),
    #[error("the picture has no such effect")]
    NoSuchEffect,
}

/// Merge refused: the clips play different audio tracks (REQ-340).
pub const DIFFERENT_AUDIO_TRACKS: &str = "The clips play different audio tracks";

/// Why two clips cannot be merged although they are parts of the same file.
pub const DIFFERENT_VOLUMES: &str = "The clips have different volumes";

/// Why two clips cannot be merged: one has Clean audio and the other has not.
pub const DIFFERENT_CLEAN_AUDIO: &str = "The clips have different clean audio settings";

/// Why two clips cannot be merged: their Scale or Move keyframes differ (1.5.0).
pub const DIFFERENT_FRAMES: &str = "The clips have different frame settings";

/// Why two clips cannot be merged: their Apply LUT files differ (1.6.0).
pub const DIFFERENT_SPEEDS: &str = "The clips have different speeds";
pub const DIFFERENT_LUTS: &str = "The clips have different colour effects";

/// What a Scale, Move or Apply LUT is applied to (REQ-130). `Whole` is not a state of its
/// own (since 1.6.1, and in 2.2.0 it has no popup either): it only means "a copy on every
/// clip" when an effect is applied (`add_scale`, `add_move`, `set_lut`, REQ-147, REQ-148).
/// Keys are read and edited on one clip (`Clip`); `Whole` reads as no keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameTarget {
    Whole,
    Clip(ClipId),
}

/// The Scale and Move keyframes of one clip, in SOURCE time of the clip's media (ADR-094):
/// a trim or split never deletes or shifts them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipFrames {
    pub clip: ClipId,
    #[serde(default)]
    pub scale: Vec<Key<Scale>>,
    #[serde(default)]
    pub moved: Vec<Key<Pos>>,
}

/// The Scale or the Move keyframes of a clip, copied in the clip's own time (REQ-204).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeysClip {
    Scale(Vec<Key<Scale>>),
    Move(Vec<Key<Pos>>),
}

/// The Apply LUT file of one clip (1.6.0, ADR-111): the path of a `.cube` file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipLut {
    pub clip: ClipId,
    pub path: String,
    /// How much of the LUT is used, 0–100 % (REQ-289; absent = 100).
    #[serde(default = "full_volume", skip_serializing_if = "is_full")]
    pub strength: u8,
}

/// The order of the movable effects of one clip (REQ-304, format 12): the rows top first (the
/// top row is drawn last). Only a clip whose order differs from the older programs' is stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipOrder {
    pub clip: ClipId,
    pub rows: Vec<crate::effect_order::Row>,
}

/// Reads the stored orders without refusing the file for a damaged entry: it becomes an
/// entry of the clip 0 (which never exists) holding an unknown row, and the project check
/// drops it with a note (REQ-311).
fn lenient_orders<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<ClipOrder>, D::Error> {
    let raw: Vec<serde_json::Value> = Vec::deserialize(d)?;
    Ok(raw
        .into_iter()
        .map(|v| {
            serde_json::from_value(v).unwrap_or(ClipOrder {
                clip: ClipId(0),
                rows: vec![crate::effect_order::Row::Unknown],
            })
        })
        .collect())
}

/// The Change aspect of one clip (REQ-286): the clip is made in its own frame, which is then
/// placed in the project's frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipAspect {
    pub clip: ClipId,
    pub aspect: FrameAspect,
}

/// The volume keyframes of one clip, in percent, in the clip's own time (REQ-280).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipVolume {
    pub clip: ClipId,
    pub keys: Vec<Key<u8>>,
}

/// The overlap media of one clip (2.3.0, REQ-325): offsets are counted from the clip's start,
/// so the overlaps follow the clip through a ripple, a trim, a split and a delete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipOverlaps {
    pub clip: ClipId,
    #[serde(deserialize_with = "crate::overlap::lenient")]
    pub items: Vec<Overlap>,
}

/// The video track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Timeline {
    /// The overlap media of the clips that have some (REQ-325), sorted by clip id; format 14.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_overlaps: Vec<ClipOverlaps>,
    /// The next id of an overlap (unique in the project; 0 and 1 both start at 1).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_overlap_id: u64,
    /// Change aspect of single clips (REQ-286), sorted by clip id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_aspects: Vec<ClipAspect>,
    /// Volume keyframes of the clips that have them (REQ-280).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_volume: Vec<ClipVolume>,
    pub clips: Vec<Clip>,
    /// Boundaries with a fade, as (left clip, right clip). Only adjacent pairs are kept.
    pub fades: BTreeSet<(ClipId, ClipId)>,
    /// How long the fades are when the user chose it (REQ-274); 0 = the usual 0.5 s. Read it
    /// with [`Timeline::fade_len`].
    #[serde(default, skip_serializing_if = "is_default_len")]
    pub fade_length: Micros,
    /// Fades that go through black on one side only (REQ-275), by boundary.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fade_sides: std::collections::BTreeMap<(ClipId, ClipId), FadeSide>,
    pub next_clip_id: u64,
    /// Change aspect of the whole video (1.5.0, REQ-124); `None` = no effect.
    #[serde(
        default,
        deserialize_with = "frame::lenient_aspect",
        skip_serializing_if = "Option::is_none"
    )]
    pub frame_aspect: Option<FrameAspect>,
    /// Scale and Move keyframes of the clips, sorted by clip id; empty entries are removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_frames: Vec<ClipFrames>,
    /// Apply LUT of the clips, sorted by clip id; empty entries are removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_luts: Vec<ClipLut>,
    /// The order of the movable effects of the clips whose order was changed, sorted by clip
    /// id (REQ-304, format 12); a clip not listed has the order of the older versions.
    #[serde(
        default,
        deserialize_with = "lenient_orders",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub clip_order: Vec<ClipOrder>,
}

impl Timeline {
    /// The stored order of a clip, top first (empty: the older order).
    pub fn order_of(&self, id: ClipId) -> &[crate::effect_order::Row] {
        self.clip_order
            .iter()
            .find(|o| o.clip == id)
            .map_or(&[], |o| &o.rows)
    }

    /// Stores the order of a clip (an empty list removes the entry).
    pub fn set_order(&mut self, id: ClipId, rows: Vec<crate::effect_order::Row>) {
        self.clip_order.retain(|o| o.clip != id);
        if !rows.is_empty() && self.get(id).is_some() {
            self.clip_order.push(ClipOrder { clip: id, rows });
            self.clip_order.sort_by_key(|o| o.clip);
        }
    }

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
    /// fades in when its own fade-in is set, the last one fades out when its fade-out is.
    pub fn fades_of(&self, index: usize) -> (Micros, Micros) {
        let Some(c) = self.clips.get(index) else {
            return (Micros::ZERO, Micros::ZERO);
        };
        let last = index + 1 == self.clips.len();
        let len = c.fade_len_of(self.fade_len());
        let side_of = |b: usize| self.fade_side(b);
        let fin =
            if (index > 0 && self.has_fade(index - 1) && side_of(index - 1) != FadeSide::OutOnly)
                || (index == 0 && c.fade_in)
            {
                len
            } else {
                Micros::ZERO
            };
        let fout = if (self.has_fade(index) && side_of(index) != FadeSide::InOnly)
            || (last && c.fade_out)
        {
            len
        } else {
            Micros::ZERO
        };
        (fin, fout)
    }

    /// The length of the fades: the chosen one, or the usual 0.5 s.
    pub fn fade_len(&self) -> Micros {
        if self.fade_length == Micros::ZERO {
            MAX_FADE
        } else {
            self.fade_length
        }
    }

    /// The side of the fade at boundary `b` (Both when it has not been chosen).
    pub fn fade_side(&self, b: usize) -> FadeSide {
        match (self.clips.get(b), self.clips.get(b + 1)) {
            (Some(l), Some(r)) => self
                .fade_sides
                .get(&(l.id, r.id))
                .copied()
                .unwrap_or_default(),
            _ => FadeSide::Both,
        }
    }

    /// Chooses the length of every fade, from 0.1 s to 10 s (REQ-274). A clip still limits
    /// it to half its own length.
    pub fn set_fade_length(&mut self, len: Micros) -> Result<(), EditError> {
        if len < MIN_FADE_LEN || len > MAX_FADE_LEN {
            return Err(EditError::FadeLength);
        }
        self.fade_length = len;
        Ok(())
    }

    /// Chooses the side of the fade at boundary `b` (REQ-275).
    pub fn set_fade_side(&mut self, b: usize, side: FadeSide) -> Result<(), EditError> {
        let (Some(l), Some(r)) = (self.clips.get(b), self.clips.get(b + 1)) else {
            return Err(EditError::NoSuchBoundary);
        };
        let key = (l.id, r.id);
        if !self.fades.contains(&key) {
            return Err(EditError::NoSuchBoundary);
        }
        if side == FadeSide::Both {
            self.fade_sides.remove(&key);
        } else {
            self.fade_sides.insert(key, side);
        }
        Ok(())
    }

    /// Drops fades whose clips are no longer adjacent.
    fn prune_fades(&mut self) {
        let pairs: BTreeSet<(ClipId, ClipId)> =
            self.clips.windows(2).map(|w| (w[0].id, w[1].id)).collect();
        self.fades.retain(|p| pairs.contains(p));
        let kept = self.fades.clone();
        self.fade_sides.retain(|p, _| kept.contains(p));
    }

    fn rename_fade_left(&mut self, old: ClipId, new: ClipId) {
        let moved: Vec<_> = self.fades.iter().filter(|p| p.0 == old).copied().collect();
        for p in moved {
            self.fades.remove(&p);
            self.fades.insert((new, p.1));
            if let Some(side) = self.fade_sides.remove(&p) {
                self.fade_sides.insert((new, p.1), side);
            }
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
        let cut = c.src_at(offset);
        self.clips[i].src_out = cut;
        // the right half keeps the clip's sound settings
        self.clips.insert(
            i + 1,
            Clip {
                id,
                src_in: cut,
                // the start fade stays with the left half, the end fade with the right one
                fade_in: false,
                ..c
            },
        );
        self.clips[i].fade_out = false;
        self.rename_fade_left(c.id, id);
        self.prune_fades();
        // both halves keep the whole path (ADR-101)
        if let Some(f) = self.frames_of(c.id).cloned() {
            self.put_frames(ClipFrames { clip: id, ..f });
        }
        // and the colour effect
        if let Some(p) = self.lut_of(FrameTarget::Clip(c.id)).map(str::to_string) {
            self.put_lut(id, Some(p));
        }
        // and the order of its effects
        let rows = self.order_of(c.id).to_vec();
        self.set_order(id, rows);
        // and each overlap is cut to the part of the clip it lies in (REQ-329); one that
        // spans the cut is in both parts, the right one under a new id
        let all = self.overlaps_of(c.id).to_vec();
        if !all.is_empty() {
            let left: Vec<Overlap> = all
                .iter()
                .filter_map(|o| o.kept_part(Micros::ZERO, offset))
                .collect();
            let mut right = Vec::new();
            for o in &all {
                if let Some(mut part) = o.kept_part(offset, c.len()) {
                    if left.iter().any(|l| l.id == o.id) {
                        part.id = self.new_overlap_id();
                    }
                    right.push(part);
                }
            }
            self.put_overlaps(c.id, left);
            self.put_overlaps(id, right);
        }
        Ok(id)
    }

    /// Removes a clip (cut / delete); following clips ripple left.
    pub fn remove(&mut self, id: ClipId) -> Result<Clip, EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        let c = self.clips.remove(i);
        self.prune_fades();
        self.clip_frames.retain(|f| f.clip != id);
        self.clip_luts.retain(|l| l.clip != id);
        self.clip_order.retain(|o| o.clip != id);
        // the overlaps go with their clip (REQ-329)
        self.clip_overlaps.retain(|o| o.clip != id);
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
        self.clips.insert(
            index,
            Clip {
                id,
                fade_in: false,
                fade_out: false,
                ..*template
            },
        );
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
        if a.speed != b.speed {
            Some(DIFFERENT_SPEEDS)
        } else if (a.volume, a.muted) != (b.volume, b.muted) {
            Some(DIFFERENT_VOLUMES)
        } else if a.audio_track != b.audio_track {
            Some(DIFFERENT_AUDIO_TRACKS)
        } else if a.clean_audio != b.clean_audio
            || a.clean_strength != b.clean_strength
            || a.speech_enhance != b.speech_enhance
        {
            Some(DIFFERENT_CLEAN_AUDIO)
        } else if self.keys_of(a.id) != self.keys_of(b.id) {
            Some(DIFFERENT_FRAMES)
        } else if self.lut_of(FrameTarget::Clip(a.id)) != self.lut_of(FrameTarget::Clip(b.id)) {
            Some(DIFFERENT_LUTS)
        } else {
            None
        }
    }

    /// Merges clip `id` with the next one (see [`Self::can_merge`]).
    pub fn merge(&mut self, id: ClipId) -> Result<(), EditError> {
        if !self.can_merge(id) {
            return Err(EditError::NotMergeable);
        }
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        // the overlaps of the second part move to the end of the first (REQ-329)
        let shift = self.clips[i].len();
        let mut items = self.overlaps_of(id).to_vec();
        items.extend(
            self.overlaps_of(self.clips[i + 1].id)
                .iter()
                .map(|o| Overlap {
                    from: o.from + shift,
                    ..o.clone()
                }),
        );
        let b = self.clips.remove(i + 1);
        self.clips[i].src_out = b.src_out;
        self.put_overlaps(id, items);
        self.clip_overlaps.retain(|o| o.clip != b.id);
        self.clips[i].fade_out = b.fade_out;
        self.fades.remove(&(id, b.id));
        self.rename_fade_left(b.id, id);
        self.prune_fades();
        self.clip_frames.retain(|f| f.clip != b.id);
        self.clip_luts.retain(|l| l.clip != b.id);
        self.clip_order.retain(|o| o.clip != b.id);
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
        if self.clips[i].to_timeline(new_out - new_in) < min_len.max(Micros(1)) {
            return Err(EditError::InvalidRange);
        }
        // the overlaps follow the kept part of the clip (REQ-329): the start of the clip moved
        // by the trimmed time, an overlap with nothing left in the new clip is removed
        let old = self.clips[i];
        let moved = old.to_timeline(new_in - old.src_in);
        self.clips[i].src_in = new_in;
        self.clips[i].src_out = new_out;
        if !self.overlaps_of(id).is_empty() {
            let len = self.clips[i].len();
            self.cut_overlaps(id, moved, moved + len);
        }
        Ok(())
    }

    /// The source range clip `id` gets when its `edge` is dragged by `delta` of timeline time
    /// (REQ-191): in whole frames of `frame`, kept inside `[0, media_duration]` and at least
    /// one frame long. `None` when there is no such clip.
    pub fn trimmed_by_edge(
        &self,
        id: ClipId,
        edge: ClipEdge,
        delta: Micros,
        media_duration: Micros,
        frame: Micros,
    ) -> Option<(Micros, Micros)> {
        let c = self.get(id)?;
        let frame = frame.max(Micros(1));
        // whole frames, to the nearest one
        let steps = (delta.0 as f64 / frame.0 as f64).round() as i64;
        let moved = c.to_src(Micros(steps.saturating_mul(frame.0)));
        Some(match edge {
            ClipEdge::Start => {
                let hi = (c.src_out - frame).max(Micros::ZERO);
                ((c.src_in + moved).clamp(Micros::ZERO, hi), c.src_out)
            }
            ClipEdge::End => {
                let lo = (c.src_in + frame).min(media_duration);
                (c.src_in, (c.src_out + moved).clamp(lo, media_duration))
            }
        })
    }

    /// Moves the fade between clips `from` to the boundary `to` (REQ-193): the fade leaves
    /// its old boundary and a boundary keeps at most one fade. A boundary that has one, a
    /// pair that is not adjacent and a fade that is not there change nothing.
    pub fn move_fade(
        &mut self,
        from: (ClipId, ClipId),
        to: (ClipId, ClipId),
    ) -> Result<(), EditError> {
        if !self.fades.contains(&from) {
            return Err(EditError::NoSuchBoundary);
        }
        let adjacent = self.clips.windows(2).any(|w| (w[0].id, w[1].id) == to);
        if !adjacent {
            return Err(EditError::NoSuchBoundary);
        }
        if self.fades.contains(&to) {
            return Err(EditError::BoundaryTaken);
        }
        self.fades.remove(&from);
        self.fades.insert(to);
        if let Some(side) = self.fade_sides.remove(&from) {
            self.fade_sides.insert(to, side);
        }
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
            self.fade_sides.remove(&(a, n));
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
                self.fade_sides.remove(&pair);
            }
        };
        if i > 0 {
            set((self.clips[i - 1].id, id));
        }
        if i + 1 < self.clips.len() {
            set((id, self.clips[i + 1].id));
        }
        if i == 0 {
            self.clips[i].fade_in = on;
        }
        if i + 1 == self.clips.len() {
            self.clips[i].fade_out = on;
        }
        Ok(())
    }

    /// True when every fade of [`Self::set_clip_fades`] is there for clip `id`.
    pub fn clip_has_fades(&self, id: ClipId) -> bool {
        let Some(i) = self.index_of(id) else {
            return false;
        };
        let left = if i == 0 {
            self.clips[i].fade_in
        } else {
            self.has_fade(i - 1)
        };
        let right = if i + 1 == self.clips.len() {
            self.clips[i].fade_out
        } else {
            self.has_fade(i)
        };
        left && right
    }

    /// The fade from black at the very start of the video: the first clip's own (REQ-083).
    pub fn start_fade(&self) -> bool {
        self.clips.first().is_some_and(|c| c.fade_in)
    }

    /// The fade to black at the very end of the video: the last clip's own.
    pub fn end_fade(&self) -> bool {
        self.clips.last().is_some_and(|c| c.fade_out)
    }

    /// Sets the fade from black of the first clip (its own fade-in).
    pub fn set_start_fade(&mut self, on: bool) -> Result<(), EditError> {
        self.clips
            .first_mut()
            .map(|c| c.fade_in = on)
            .ok_or(EditError::EmptyTimeline)
    }

    /// Sets the fade to black of the last clip (its own fade-out).
    pub fn set_end_fade(&mut self, on: bool) -> Result<(), EditError> {
        self.clips
            .last_mut()
            .map(|c| c.fade_out = on)
            .ok_or(EditError::EmptyTimeline)
    }

    /// Sets or removes the fade in of the first clip and the fade out of the last clip
    /// (the whole-video Fade IN/OUT, REQ-147).
    pub fn set_edge_fades(&mut self, on: bool) -> Result<(), EditError> {
        if self.clips.is_empty() {
            return Err(EditError::EmptyTimeline);
        }
        if let Some(c) = self.clips.first_mut() {
            c.fade_in = on;
        }
        if let Some(c) = self.clips.last_mut() {
            c.fade_out = on;
        }
        Ok(())
    }

    /// True when any fade (boundary, start or end) is set.
    pub fn has_any_fade(&self) -> bool {
        !self.fades.is_empty() || self.start_fade() || self.end_fade()
    }

    /// Removes every fade (boundaries, start and end flags of every clip).
    pub fn clear_fades(&mut self) {
        self.fades.clear();
        self.fade_sides.clear();
        for c in &mut self.clips {
            c.fade_in = false;
            c.fade_out = false;
        }
    }

    // ---- Apply LUT (1.6.0) ----

    /// The LUT file path of a clip; for the whole video, the first clip's (REQ-147).
    pub fn lut_of(&self, t: FrameTarget) -> Option<&str> {
        let id = match t {
            FrameTarget::Whole => self.clips.first()?.id,
            FrameTarget::Clip(id) => id,
        };
        self.clip_luts
            .iter()
            .find(|l| l.clip == id)
            .map(|l| l.path.as_str())
    }

    /// Sets (`Some`) or removes (`None`) the LUT of a clip, or of every clip for the whole
    /// video; an unknown clip is refused.
    pub fn set_lut(&mut self, t: FrameTarget, path: Option<String>) -> Result<(), EditError> {
        match t {
            FrameTarget::Clip(id) => {
                self.get(id).ok_or(EditError::NoSuchClip)?;
                self.put_lut(id, path);
            }
            FrameTarget::Whole => {
                if self.clips.is_empty() {
                    return Err(EditError::EmptyTimeline);
                }
                let ids: Vec<ClipId> = self.clips.iter().map(|c| c.id).collect();
                for id in ids {
                    self.put_lut(id, path.clone());
                }
            }
        }
        Ok(())
    }

    fn put_lut(&mut self, id: ClipId, path: Option<String>) {
        let path = path.filter(|p| !p.is_empty());
        let strength = self
            .clip_luts
            .iter()
            .find(|l| l.clip == id)
            .map_or(FULL_VOLUME, |l| l.strength);
        self.clip_luts.retain(|l| l.clip != id);
        if let Some(path) = path {
            self.clip_luts.push(ClipLut {
                clip: id,
                path,
                strength,
            });
            self.clip_luts.sort_by_key(|l| l.clip);
        }
    }

    /// The strength of the LUT of `t` (the first clip's for the whole video), 100 when none.
    pub fn lut_strength_of(&self, t: FrameTarget) -> u8 {
        let id = match t {
            FrameTarget::Clip(id) => Some(id),
            FrameTarget::Whole => self.clips.first().map(|c| c.id),
        };
        id.and_then(|id| self.clip_luts.iter().find(|l| l.clip == id))
            .map_or(FULL_VOLUME, |l| l.strength)
    }

    /// Chooses how much of the LUT is used, 0 to 100 % (REQ-289); the target must have one.
    pub fn set_lut_strength(&mut self, t: FrameTarget, percent: u8) -> Result<(), EditError> {
        let ids: Vec<ClipId> = match t {
            FrameTarget::Clip(id) => {
                self.get(id).ok_or(EditError::NoSuchClip)?;
                vec![id]
            }
            FrameTarget::Whole => self.clips.iter().map(|c| c.id).collect(),
        };
        let mut any = false;
        for l in self.clip_luts.iter_mut().filter(|l| ids.contains(&l.clip)) {
            l.strength = percent.min(100);
            any = true;
        }
        if any {
            Ok(())
        } else {
            Err(EditError::NoSuchClip)
        }
    }

    /// True when any Apply LUT exists (the export is then a re-encode, REQ-136).
    pub fn any_lut(&self) -> bool {
        !self.clip_luts.is_empty()
    }

    // ---- Change aspect, Scale and Move (1.5.0) ----

    /// The keyframes of a clip, if it has any.
    pub fn frames_of(&self, id: ClipId) -> Option<&ClipFrames> {
        self.clip_frames.iter().find(|f| f.clip == id)
    }

    /// The (scale, move) keys of a clip, empty when it has none.
    fn keys_of(&self, id: ClipId) -> (&[Key<Scale>], &[Key<Pos>]) {
        match self.frames_of(id) {
            Some(f) => (&f.scale, &f.moved),
            None => (&[], &[]),
        }
    }

    /// Stores the keyframes of a clip (replacing its entry; an empty entry is dropped).
    pub fn put_frames(&mut self, f: ClipFrames) {
        self.clip_frames.retain(|x| x.clip != f.clip);
        if !f.scale.is_empty() || !f.moved.is_empty() {
            self.clip_frames.push(f);
            self.clip_frames.sort_by_key(|x| x.clip);
        }
    }

    /// The Scale keys of a clip (source time of its media); none for `Whole`.
    pub fn scale_keys(&self, t: FrameTarget) -> &[Key<Scale>] {
        match t {
            FrameTarget::Whole => &[],
            FrameTarget::Clip(id) => self.keys_of(id).0,
        }
    }

    pub fn move_keys(&self, t: FrameTarget) -> &[Key<Pos>] {
        match t {
            FrameTarget::Whole => &[],
            FrameTarget::Clip(id) => self.keys_of(id).1,
        }
    }

    fn check_target(&self, t: FrameTarget) -> Result<(), EditError> {
        match t {
            FrameTarget::Whole if self.clips.is_empty() => Err(EditError::EmptyTimeline),
            FrameTarget::Clip(id) if self.get(id).is_none() => Err(EditError::NoSuchClip),
            _ => Ok(()),
        }
    }

    /// Runs `f` on the scale keys of `t` (an entry of a clip is made when missing and
    /// dropped again when it ends up empty).
    fn with_scale<R>(
        &mut self,
        t: FrameTarget,
        f: impl FnOnce(&mut Vec<Key<Scale>>) -> R,
    ) -> Result<R, EditError> {
        self.check_target(t)?;
        let FrameTarget::Clip(id) = t else {
            return Err(EditError::NoSuchClip);
        };
        let mut e = self.frames_of(id).cloned().unwrap_or(ClipFrames {
            clip: id,
            scale: vec![],
            moved: vec![],
        });
        let r = f(&mut e.scale);
        self.put_frames(e);
        Ok(r)
    }

    fn with_move<R>(
        &mut self,
        t: FrameTarget,
        f: impl FnOnce(&mut Vec<Key<Pos>>) -> R,
    ) -> Result<R, EditError> {
        self.check_target(t)?;
        let FrameTarget::Clip(id) = t else {
            return Err(EditError::NoSuchClip);
        };
        let mut e = self.frames_of(id).cloned().unwrap_or(ClipFrames {
            clip: id,
            scale: vec![],
            moved: vec![],
        });
        let r = f(&mut e.moved);
        self.put_frames(e);
        Ok(r)
    }

    /// Where a new Scale or Move of a clip starts: its first source instant.
    pub fn frame_start(&self, t: FrameTarget) -> Micros {
        match t {
            FrameTarget::Whole => Micros::ZERO,
            FrameTarget::Clip(id) => self.get(id).map_or(Micros::ZERO, |c| c.src_in),
        }
    }

    /// The time of a key of a clip at timeline position `at`: the clip's source time; none
    /// for `Whole`.
    pub fn key_time(&self, t: FrameTarget, at: Micros) -> Option<Micros> {
        match t {
            FrameTarget::Whole => None,
            FrameTarget::Clip(id) => {
                let i = self.index_of(id)?;
                let c = &self.clips[i];
                Some(c.src_at((at - self.start_of(i)).clamp(Micros::ZERO, c.len())))
            }
        }
    }

    /// Adds Scale (100 %, one keyframe at the start) to a clip, or to every clip that has
    /// none for the whole video (one copy per clip, REQ-148); false when nothing was added.
    pub fn add_scale(&mut self, t: FrameTarget) -> Result<bool, EditError> {
        self.add_normal(
            t,
            |c| c.scale.is_empty(),
            |c, at| {
                c.scale.push(Key {
                    t: at,
                    v: Scale::NORMAL,
                    ease: Default::default(),
                })
            },
        )
    }

    /// Adds Move (centred, one keyframe at the start); false when it is there.
    pub fn add_move(&mut self, t: FrameTarget) -> Result<bool, EditError> {
        self.add_normal(
            t,
            |c| c.moved.is_empty(),
            |c, at| {
                c.moved.push(Key {
                    t: at,
                    v: frame::CENTRE,
                    ease: Default::default(),
                })
            },
        )
    }

    fn add_normal(
        &mut self,
        t: FrameTarget,
        lacks: impl Fn(&ClipFrames) -> bool,
        push: impl Fn(&mut ClipFrames, Micros),
    ) -> Result<bool, EditError> {
        self.check_target(t)?;
        let ids: Vec<(ClipId, Micros)> = match t {
            FrameTarget::Whole => self.clips.iter().map(|c| (c.id, c.src_in)).collect(),
            FrameTarget::Clip(id) => vec![(id, self.frame_start(t))],
        };
        let mut any = false;
        for (id, at) in ids {
            let mut e = self.frames_of(id).cloned().unwrap_or(ClipFrames {
                clip: id,
                scale: vec![],
                moved: vec![],
            });
            if lacks(&e) {
                push(&mut e, at);
                self.put_frames(e);
                any = true;
            }
        }
        Ok(any)
    }

    /// Removes the Scale of one clip (its copy, REQ-148); `Whole` has none.
    pub fn remove_scale(&mut self, t: FrameTarget) -> Result<(), EditError> {
        if self.scale_keys(t).is_empty() {
            return Err(EditError::NoSuchEffect);
        }
        self.remove_kind(t, true)
    }

    pub fn remove_move(&mut self, t: FrameTarget) -> Result<(), EditError> {
        if self.move_keys(t).is_empty() {
            return Err(EditError::NoSuchEffect);
        }
        self.remove_kind(t, false)
    }

    fn remove_kind(&mut self, t: FrameTarget, scale: bool) -> Result<(), EditError> {
        self.check_target(t)?;
        let FrameTarget::Clip(id) = t else {
            return Err(EditError::NoSuchClip);
        };
        if let Some(mut e) = self.frames_of(id).cloned() {
            if scale {
                e.scale.clear();
            } else {
                e.moved.clear();
            }
            self.put_frames(e);
        }
        Ok(())
    }

    /// Sets the Scale key at `at` (adds it when there is none); limits are applied.
    pub fn set_scale_key(
        &mut self,
        t: FrameTarget,
        at: Micros,
        mut v: Scale,
    ) -> Result<(), EditError> {
        v.clamp();
        if self.scale_keys(t).is_empty() {
            return Err(EditError::NoSuchEffect);
        }
        self.with_scale(t, |k| keyframes::set(k, at, v))?
            .map_err(EditError::Key)
    }

    pub fn set_move_key(&mut self, t: FrameTarget, at: Micros, v: Pos) -> Result<(), EditError> {
        let v = Pos {
            x: v.x.min(crate::base::pos::POS_MAX),
            y: v.y.min(crate::base::pos::POS_MAX),
        };
        if self.move_keys(t).is_empty() {
            return Err(EditError::NoSuchEffect);
        }
        self.with_move(t, |k| keyframes::set(k, at, v))?
            .map_err(EditError::Key)
    }

    /// Adds a Scale key (never replaces one).
    pub fn add_scale_key(
        &mut self,
        t: FrameTarget,
        at: Micros,
        mut v: Scale,
    ) -> Result<(), EditError> {
        v.clamp();
        if self.scale_keys(t).is_empty() {
            return Err(EditError::NoSuchEffect);
        }
        self.with_scale(t, |k| keyframes::add(k, at, v))?
            .map_err(EditError::Key)
    }

    pub fn add_move_key(&mut self, t: FrameTarget, at: Micros, v: Pos) -> Result<(), EditError> {
        if self.move_keys(t).is_empty() {
            return Err(EditError::NoSuchEffect);
        }
        self.with_move(t, |k| keyframes::add(k, at, v))?
            .map_err(EditError::Key)
    }

    /// Removes the Scale key at `at`; the last key stays.
    pub fn remove_scale_key(&mut self, t: FrameTarget, at: Micros) -> Result<(), EditError> {
        self.with_scale(t, |k| keyframes::remove(k, at))?
            .map_err(EditError::Key)
    }

    /// Chooses how the Scale or Move value moves on from the key at `at` (REQ-282).
    pub fn set_key_ease(
        &mut self,
        t: FrameTarget,
        scale: bool,
        at: Micros,
        ease: keyframes::Ease,
    ) -> Result<(), EditError> {
        let done = if scale {
            self.with_scale(t, |k| keyframes::set_ease(k, at, ease))?
        } else {
            self.with_move(t, |k| keyframes::set_ease(k, at, ease))?
        };
        done.map_err(EditError::Key)
    }

    /// The easing of the Scale or Move key at `at`.
    pub fn key_ease(&self, t: FrameTarget, scale: bool, at: Micros) -> keyframes::Ease {
        if scale {
            keyframes::ease_at(self.scale_keys(t), at)
        } else {
            keyframes::ease_at(self.move_keys(t), at)
        }
    }

    pub fn remove_move_key(&mut self, t: FrameTarget, at: Micros) -> Result<(), EditError> {
        self.with_move(t, |k| keyframes::remove(k, at))?
            .map_err(EditError::Key)
    }

    /// The Scale keys of a clip in the clip's own time (REQ-204); `None` without any.
    pub fn copy_scale_keys(&self, id: ClipId) -> Option<KeysClip> {
        let c = self.get(id)?;
        let keys = self.keys_of(id).0;
        (!keys.is_empty()).then(|| KeysClip::Scale(keyframes::relative(keys, c.src_in)))
    }

    /// The Move keys of a clip in the clip's own time (REQ-204).
    pub fn copy_move_keys(&self, id: ClipId) -> Option<KeysClip> {
        let c = self.get(id)?;
        let keys = self.keys_of(id).1;
        (!keys.is_empty()).then(|| KeysClip::Move(keyframes::relative(keys, c.src_in)))
    }

    /// Puts copied keys on clip `id` at the same clip times, replacing the keys of that
    /// effect only (REQ-204); the clip's other effects are not touched.
    pub fn paste_keys(&mut self, id: ClipId, keys: &KeysClip) -> Result<(), EditError> {
        let c = *self.get(id).ok_or(EditError::NoSuchClip)?;
        let mut e = self.frames_of(id).cloned().unwrap_or(ClipFrames {
            clip: id,
            scale: vec![],
            moved: vec![],
        });
        match keys {
            KeysClip::Scale(k) => e.scale = keyframes::place(k, c.src_in, c.src_len()),
            KeysClip::Move(k) => e.moved = keyframes::place(k, c.src_in, c.src_len()),
        }
        self.put_frames(e);
        Ok(())
    }

    /// Moves the Scale key of a clip from `from` to `to` (source time, inside the clip).
    pub fn move_scale_key(
        &mut self,
        id: ClipId,
        from: Micros,
        to: Micros,
    ) -> Result<Micros, EditError> {
        let c = *self.get(id).ok_or(EditError::NoSuchClip)?;
        self.with_scale(FrameTarget::Clip(id), |k| {
            keyframes::retime(k, from, to, c.src_in, c.src_out)
        })?
        .map_err(EditError::Key)
    }

    /// Moves the Move key of a clip from `from` to `to` (source time, inside the clip).
    pub fn move_move_key(
        &mut self,
        id: ClipId,
        from: Micros,
        to: Micros,
    ) -> Result<Micros, EditError> {
        let c = *self.get(id).ok_or(EditError::NoSuchClip)?;
        self.with_move(FrameTarget::Clip(id), |k| {
            keyframes::retime(k, from, to, c.src_in, c.src_out)
        })?
        .map_err(EditError::Key)
    }

    /// Sets or removes Change aspect.
    pub fn set_frame_aspect(&mut self, a: Option<FrameAspect>) -> Result<(), EditError> {
        if self.clips.is_empty() {
            return Err(EditError::EmptyTimeline);
        }
        self.frame_aspect = a;
        Ok(())
    }

    /// True when Change aspect, Scale or Move changes the picture (REQ-131: such a project
    /// is always re-encoded).
    pub fn has_frame_effect(&self) -> bool {
        self.frame_aspect.is_some()
            || !self.clip_aspects.is_empty()
            || self.clips.iter().any(|c| {
                let (s, m) = self.keys_of(c.id);
                !s.is_empty() || !m.is_empty()
            })
    }

    /// The clip's own Change aspect (REQ-286), if it has one.
    pub fn own_aspect(&self, id: ClipId) -> Option<FrameAspect> {
        self.clip_aspects
            .iter()
            .find(|a| a.clip == id)
            .map(|a| a.aspect)
    }

    /// Gives a clip its own Change aspect, or takes it away (`None`).
    pub fn set_clip_aspect(
        &mut self,
        id: ClipId,
        aspect: Option<FrameAspect>,
    ) -> Result<(), EditError> {
        self.get(id).ok_or(EditError::NoSuchClip)?;
        self.clip_aspects.retain(|a| a.clip != id);
        if let Some(aspect) = aspect {
            self.clip_aspects.push(ClipAspect { clip: id, aspect });
            self.clip_aspects.sort_by_key(|a| a.clip);
        }
        Ok(())
    }

    /// The Scale and Move keys that act on the clip at `index`: its own copy only, since a
    /// clip has at most one Scale and one Move (REQ-130, REQ-131).
    pub fn frame_stack(&self, index: usize) -> Stack {
        let mut st = Stack::default();
        let Some(c) = self.clips.get(index) else {
            return st;
        };
        let (s, m) = self.keys_of(c.id);
        if !s.is_empty() {
            st.scale.push(Track {
                keys: c.keys_to_timeline(s),
                offset: c.src_in,
            });
        }
        if !m.is_empty() {
            st.moved.push(Track {
                keys: c.keys_to_timeline(m),
                offset: c.src_in,
            });
        }
        st
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

    /// Chooses the audio track a clip plays (REQ-340).
    pub fn set_audio_track(&mut self, id: ClipId, track: u8) -> Result<(), EditError> {
        let i = self.index_of(id).ok_or(EditError::NoSuchClip)?;
        self.clips[i].audio_track = track;
        Ok(())
    }

    /// Switches Clean audio on or off for the clips `ids` (REQ-114).
    pub fn set_clean_audio(&mut self, ids: &[ClipId], on: bool) -> Result<(), EditError> {
        if ids.is_empty() {
            return Err(EditError::NothingSelected);
        }
        if ids.iter().any(|i| self.get(*i).is_none()) {
            return Err(EditError::NoSuchClip);
        }
        for c in self.clips.iter_mut().filter(|c| ids.contains(&c.id)) {
            c.clean_audio = on;
        }
        Ok(())
    }

    /// Chooses how strong Clean audio is on the clips `ids`, 0 to 100 % (REQ-278).
    pub fn set_clean_strength(&mut self, ids: &[ClipId], percent: u8) -> Result<(), EditError> {
        if ids.is_empty() {
            return Err(EditError::NothingSelected);
        }
        if ids.iter().any(|i| self.get(*i).is_none()) {
            return Err(EditError::NoSuchClip);
        }
        for c in self.clips.iter_mut().filter(|c| ids.contains(&c.id)) {
            c.clean_strength = percent.min(100);
        }
        Ok(())
    }

    /// Switches Normalise speech on or off for the clips `ids` (REQ-279).
    pub fn set_speech_enhance(&mut self, ids: &[ClipId], on: bool) -> Result<(), EditError> {
        if ids.is_empty() {
            return Err(EditError::NothingSelected);
        }
        if ids.iter().any(|i| self.get(*i).is_none()) {
            return Err(EditError::NoSuchClip);
        }
        for c in self.clips.iter_mut().filter(|c| ids.contains(&c.id)) {
            c.speech_enhance = on;
        }
        Ok(())
    }

    /// Adds or changes the volume key at clip time `at` (REQ-280): percent 0 to 100.
    pub fn set_volume_key(&mut self, id: ClipId, at: Micros, percent: u8) -> Result<(), EditError> {
        let c = self
            .clips
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(EditError::NoSuchClip)?;
        if percent > 100 {
            return Err(EditError::VolumeKey);
        }
        let len = c.len();
        if at < Micros::ZERO || at > len {
            return Err(EditError::VolumeKey);
        }
        let entry = match self.clip_volume.iter().position(|v| v.clip == id) {
            Some(i) => i,
            None => {
                self.clip_volume.push(ClipVolume {
                    clip: id,
                    keys: Vec::new(),
                });
                self.clip_volume.len() - 1
            }
        };
        keyframes::set(&mut self.clip_volume[entry].keys, at, percent).map_err(EditError::Key)
    }

    /// The volume keys of a clip (empty when it has none).
    pub fn volume_keys(&self, id: ClipId) -> &[Key<u8>] {
        self.clip_volume
            .iter()
            .find(|v| v.clip == id)
            .map_or(&[], |v| v.keys.as_slice())
    }

    pub fn any_volume_keys(&self) -> bool {
        self.clip_volume.iter().any(|v| !v.keys.is_empty())
    }

    /// Removes the volume key at clip time `at`.
    pub fn remove_volume_key(&mut self, id: ClipId, at: Micros) -> Result<(), EditError> {
        self.get(id).ok_or(EditError::NoSuchClip)?;
        for v in &mut self.clip_volume {
            if v.clip == id {
                v.keys.retain(|k| k.t != at);
            }
        }
        self.clip_volume.retain(|v| !v.keys.is_empty());
        Ok(())
    }

    /// Sets how the volume moves on from the key at clip time `at` (REQ-282).
    pub fn set_volume_key_ease(
        &mut self,
        id: ClipId,
        at: Micros,
        ease: keyframes::Ease,
    ) -> Result<(), EditError> {
        self.get(id).ok_or(EditError::NoSuchClip)?;
        let v = self
            .clip_volume
            .iter_mut()
            .find(|v| v.clip == id)
            .ok_or(EditError::NoSuchClip)?;
        keyframes::set_ease(&mut v.keys, at, ease).map_err(EditError::Key)
    }

    /// Takes every volume key off a clip: the Volume effect is removed (REQ-280).
    pub fn clear_volume_keys(&mut self, id: ClipId) -> Result<(), EditError> {
        self.get(id).ok_or(EditError::NoSuchClip)?;
        self.clip_volume.retain(|v| v.clip != id);
        Ok(())
    }

    /// Sets the speed of a clip, in thousandths from 500 to 2000 (REQ-281); the clip's length
    /// on the timeline changes and what follows ripples. The times of its overlaps are
    /// rescaled with the speed, so each keeps its part of the clip (REQ-329).
    pub fn set_speed(&mut self, id: ClipId, speed: u16) -> Result<(), EditError> {
        if !(MIN_SPEED..=MAX_SPEED).contains(&speed) {
            return Err(EditError::SpeedRange);
        }
        let c = self
            .clips
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(EditError::NoSuchClip)?;
        let old = std::mem::replace(&mut c.speed, speed);
        if old != speed {
            for o in &mut self.clip_overlaps {
                if o.clip == id {
                    for x in &mut o.items {
                        *x = x.rescaled(old, speed);
                    }
                }
            }
        }
        Ok(())
    }

    /// True when any clip plays at another speed than normal.
    pub fn any_speed(&self) -> bool {
        self.clips.iter().any(|c| c.speed != NORMAL_SPEED)
    }

    /// The overlaps of clip `id`, in the order they were added (REQ-325).
    pub fn overlaps_of(&self, id: ClipId) -> &[Overlap] {
        self.clip_overlaps
            .iter()
            .find(|o| o.clip == id)
            .map_or(&[], |o| &o.items)
    }

    /// The ids of every overlap of the project.
    pub fn overlap_ids(&self) -> Vec<u64> {
        self.clip_overlaps
            .iter()
            .flat_map(|o| o.items.iter().map(|x| x.id))
            .collect()
    }

    /// True when any clip has an overlap.
    pub fn any_overlap(&self) -> bool {
        self.clip_overlaps.iter().any(|o| !o.items.is_empty())
    }

    /// The clip that holds overlap `id` and the overlap.
    pub fn find_overlap(&self, id: u64) -> Option<(ClipId, &Overlap)> {
        self.clip_overlaps
            .iter()
            .find_map(|o| o.items.iter().find(|x| x.id == id).map(|x| (o.clip, x)))
    }

    /// Reserves a new overlap id (unique in the project).
    pub fn new_overlap_id(&mut self) -> u64 {
        self.next_overlap_id = self.next_overlap_id.max(1);
        let id = self.next_overlap_id;
        self.next_overlap_id += 1;
        id
    }

    /// Replaces the overlaps of a clip (an empty list removes the entry); entries stay sorted
    /// by clip id.
    pub fn put_overlaps(&mut self, id: ClipId, items: Vec<Overlap>) {
        self.clip_overlaps.retain(|o| o.clip != id);
        if !items.is_empty() {
            self.clip_overlaps.push(ClipOverlaps { clip: id, items });
            self.clip_overlaps.sort_by_key(|o| o.clip);
        }
    }

    /// Adds an overlap to clip `id`, kept inside the clip; its id comes from
    /// [`Self::new_overlap_id`].
    pub fn add_overlap(&mut self, id: ClipId, o: Overlap) -> Result<(), EditError> {
        let len = self.get(id).ok_or(EditError::NoSuchClip)?.len();
        let mut items = self.overlaps_of(id).to_vec();
        items.push(o.clamped(len));
        self.put_overlaps(id, items);
        Ok(())
    }

    /// Changes overlap `id` with `f` (the result is kept inside its clip).
    pub fn change_overlap(
        &mut self,
        id: u64,
        f: impl FnOnce(&mut Overlap),
    ) -> Result<(), EditError> {
        let (clip, _) = self.find_overlap(id).ok_or(EditError::NoSuchOverlap)?;
        let len = self.get(clip).ok_or(EditError::NoSuchClip)?.len();
        let mut items = self.overlaps_of(clip).to_vec();
        if let Some(x) = items.iter_mut().find(|x| x.id == id) {
            f(x);
            *x = x.clone().clamped(len);
        }
        self.put_overlaps(clip, items);
        Ok(())
    }

    /// Removes overlap `id`.
    pub fn remove_overlap(&mut self, id: u64) -> Result<(), EditError> {
        let (clip, _) = self.find_overlap(id).ok_or(EditError::NoSuchOverlap)?;
        let mut items = self.overlaps_of(clip).to_vec();
        items.retain(|x| x.id != id);
        self.put_overlaps(clip, items);
        Ok(())
    }

    /// The overlaps of a clip after its time window changed to `[keep_from, keep_to)` of its old
    /// offsets, counted from `keep_from` (a trim, REQ-329); the ones with no part left are gone.
    fn cut_overlaps(&mut self, id: ClipId, keep_from: Micros, keep_to: Micros) {
        let items: Vec<Overlap> = self
            .overlaps_of(id)
            .iter()
            .filter_map(|o| o.kept_part(keep_from, keep_to))
            .collect();
        self.put_overlaps(id, items);
    }

    /// True when any clip has Clean audio.
    pub fn any_clean_audio(&self) -> bool {
        self.clips.iter().any(|c| c.clean_audio || c.speech_enhance)
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
            stat: None,
            missing: false,
            status: Default::default(),
            proxy: Default::default(),
            from: None,
        }
    }

    const F: Micros = Micros(33_333);

    fn s(n: i64) -> Micros {
        Micros(n * 1_000_000)
    }

    fn neg(n: i64) -> Micros {
        Micros(-n * 1_000_000)
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

    // covers: REQ-191
    #[test]
    fn dragging_an_edge_trims_in_whole_frames_inside_the_source() {
        let mut t = Timeline::default();
        t.append(&media(1, 4, true)).unwrap();
        let a = t.clips[0].id;
        let at = |t: &Timeline, e, d| t.trimmed_by_edge(a, e, d, s(4), F);
        // the right edge 1 s to the left: 1 s shorter, within one frame
        let (i, o) = at(&t, ClipEdge::End, neg(1)).unwrap();
        assert_eq!(i, s(0));
        assert!(((s(4) - o) - s(1)).0.abs() <= F.0, "{o:?}");
        assert_eq!(((s(4) - o).0 % F.0), 0, "whole frames");
        // the left edge 0.5 s to the right
        let (i, o) = at(&t, ClipEdge::Start, Micros(500_000)).unwrap();
        assert!((i - Micros(500_000)).0.abs() <= F.0 && o == s(4));
        // limits: the source, and one frame at least
        assert_eq!(at(&t, ClipEdge::End, s(9)).unwrap().1, s(4));
        assert_eq!(at(&t, ClipEdge::Start, neg(9)).unwrap().0, s(0));
        assert_eq!(at(&t, ClipEdge::End, neg(99)).unwrap(), (s(0), F));
        assert_eq!(at(&t, ClipEdge::Start, s(99)).unwrap(), (s(4) - F, s(4)));
        assert!(
            t.trimmed_by_edge(ClipId(9), ClipEdge::End, s(1), s(4), F)
                .is_none()
        );
        // the same trim as the clip details make
        let (i, o) = at(&t, ClipEdge::End, neg(1)).unwrap();
        let mut u = t.clone();
        u.trim(a, i, o, s(4), F).unwrap();
        assert_eq!(u.duration(), o);
        // a clip already trimmed moves from its own edges
        t.trim(a, s(1), s(3), s(4), F).unwrap();
        assert_eq!(at(&t, ClipEdge::End, s(5)).unwrap(), (s(1), s(4)));
        assert_eq!(at(&t, ClipEdge::Start, neg(5)).unwrap(), (s(0), s(3)));
    }

    // covers: REQ-193
    #[test]
    fn a_fade_moves_to_an_empty_boundary_only() {
        let mut t = Timeline::default();
        for i in 1..=4 {
            t.append(&media(i, 2, true)).unwrap();
        }
        let ids: Vec<ClipId> = t.clips.iter().map(|c| c.id).collect();
        let (b0, b1, b2) = ((ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[3]));
        t.toggle_fade(0).unwrap();
        t.toggle_fade(2).unwrap();
        // boundary 1 -> 2 (indexes 0 and 1)
        t.move_fade(b0, b1).unwrap();
        assert!(!t.has_fade(0) && t.has_fade(1) && t.has_fade(2));
        // a boundary keeps at most one fade
        assert_eq!(t.move_fade(b1, b2), Err(EditError::BoundaryTaken));
        // no fade to move, no such boundary: nothing changes
        assert_eq!(t.move_fade(b0, b1), Err(EditError::NoSuchBoundary));
        assert_eq!(
            t.move_fade(b1, (ids[0], ids[3])),
            Err(EditError::NoSuchBoundary)
        );
        assert!(!t.has_fade(0) && t.has_fade(1) && t.has_fade(2));
        assert!(EditError::BoundaryTaken.to_string().contains("already"));
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
        assert!(!t.start_fade() && !t.end_fade());
        assert!(t.clip_has_fades(ids[1]));
        assert!(!t.clip_has_fades(ids[0]));
        // removing it from the clip removes both edges
        t.set_clip_fades(ids[1], false).unwrap();
        assert!(!t.has_any_fade());
        // (b) the first clip: fade from black at the start + its boundary
        t.set_clip_fades(ids[0], true).unwrap();
        assert!(t.start_fade() && t.has_fade(0) && !t.has_fade(1) && !t.end_fade());
        assert_eq!(t.fades_of(0), (MAX_FADE, MAX_FADE));
        // the last clip: its boundary + fade to black at the end
        t.set_clip_fades(ids[2], true).unwrap();
        assert!(t.end_fade() && t.has_fade(1));
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
        // deleting the first clip removes its start fade (REQ-083): it belongs to the clip
        let first = t.clips[0].id;
        t.remove(first).unwrap();
        assert_eq!(t.fades_of(0).0, Micros::ZERO);
        assert_eq!(t.fades_of(1), (Micros::ZERO, MAX_FADE));
        // moving the first clip to the end: its start fade moves with it (a boundary fade
        // is not there, so nothing shows at the start)
        let mut u = three();
        u.set_edge_fades(true).unwrap();
        let first = u.clips[0].id;
        u.move_to(first, 9).unwrap();
        assert_eq!(u.fades_of(0).0, Micros::ZERO);
        assert_eq!(u.fades_of(2), (Micros::ZERO, Micros::ZERO));
        assert!(u.clips[2].fade_in && !u.start_fade());
        // a clip shorter than 1 s fades over half its length
        let mut short = Timeline::default();
        short.append(&media(1, 4, true)).unwrap();
        let id = short.clips[0].id;
        short.trim(id, s(0), Micros(600_000), s(4), F).unwrap();
        short.set_edge_fades(true).unwrap();
        assert_eq!(short.fades_of(0), (Micros(300_000), Micros(300_000)));
        t.set_edge_fades(false).unwrap();
        assert!(!t.start_fade() && !t.end_fade());
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

    // covers: REQ-135
    #[test]
    fn lut_rules_set_remove_split_merge_and_remove_clip() {
        let mut t = track();
        let a = t.clips[0].id;
        assert!(!t.any_lut());
        assert_eq!(
            t.set_lut(FrameTarget::Clip(ClipId(99)), Some("x.cube".into())),
            Err(EditError::NoSuchClip)
        );
        t.set_lut(FrameTarget::Clip(a), Some("/l/a.cube".into()))
            .unwrap();
        t.set_lut(FrameTarget::Whole, Some("/l/w.cube".into()))
            .unwrap();
        assert!(t.any_lut());
        // the whole-video LUT is a copy on each clip and replaces the clip's own (REQ-147)
        assert_eq!(t.lut_of(FrameTarget::Clip(a)), Some("/l/w.cube"));
        assert_eq!(t.lut_of(FrameTarget::Whole), Some("/l/w.cube"));
        t.set_lut(FrameTarget::Clip(a), Some("/l/a.cube".into()))
            .unwrap();
        // split: both halves have it, and can merge back
        let right = t.split(s(2), F).unwrap();
        assert_eq!(t.lut_of(FrameTarget::Clip(right)), Some("/l/a.cube"));
        assert!(t.can_merge(a));
        // different files block the merge, with the reason
        t.set_lut(FrameTarget::Clip(right), Some("/l/b.cube".into()))
            .unwrap();
        assert_eq!(t.merge_block_reason(a), Some(DIFFERENT_LUTS));
        t.set_lut(FrameTarget::Clip(right), None).unwrap();
        assert_eq!(t.merge_block_reason(a), Some(DIFFERENT_LUTS));
        t.set_lut(FrameTarget::Whole, None).unwrap();
        t.merge(a).unwrap();
        assert!(t.clip_luts.is_empty(), "empty entries are removed");
        // removing a clip drops its entry; an empty path removes
        t.set_lut(FrameTarget::Clip(a), Some("/l/a.cube".into()))
            .unwrap();
        t.set_lut(FrameTarget::Clip(a), Some(String::new()))
            .unwrap();
        assert!(t.clip_luts.is_empty());
        t.set_lut(FrameTarget::Clip(a), Some("/l/a.cube".into()))
            .unwrap();
        t.remove(a).unwrap();
        assert!(t.clip_luts.is_empty());
        t.set_lut(FrameTarget::Whole, None).unwrap();
        assert!(!t.any_lut());
    }

    // covers: REQ-114
    #[test]
    fn clean_audio_follows_split_copy_and_move_and_blocks_merge() {
        let mut t = track();
        let a = t.clips[0].id;
        assert!(!t.any_clean_audio());
        assert_eq!(
            t.set_clean_audio(&[], true),
            Err(EditError::NothingSelected)
        );
        assert_eq!(
            t.set_clean_audio(&[ClipId(77)], true),
            Err(EditError::NoSuchClip)
        );
        t.set_clean_audio(&[a], true).unwrap();
        assert!(t.any_clean_audio());
        let right = t.split(s(2), F).unwrap();
        assert!(t.clips[0].clean_audio && t.get(right).unwrap().clean_audio);
        assert!(t.can_merge(a));
        let r = *t.get(right).unwrap();
        let id = t.insert_at(&r, t.duration(), F).unwrap();
        assert!(t.get(id).unwrap().clean_audio);
        t.move_to(id, 0).unwrap();
        assert!(t.clips[0].clean_audio);
        t.set_clean_audio(&[right], false).unwrap();
        assert!(!t.can_merge(a));
        assert_eq!(t.merge_block_reason(a), Some(DIFFERENT_CLEAN_AUDIO));
        assert_eq!(t.merge(a), Err(EditError::NotMergeable));
        t.set_clean_audio(&[right], true).unwrap();
        t.merge(a).unwrap();
        // old files have it off
        let c: Clip = serde_json::from_str(r#"{"id":1,"media":2,"src_in":0,"src_out":5}"#).unwrap();
        assert!(!c.clean_audio);
    }

    // covers: REQ-094
    #[test]
    fn clips_and_timelines_of_1_1_read_with_defaults() {
        let c: Clip = serde_json::from_str(r#"{"id":1,"media":2,"src_in":0,"src_out":5}"#).unwrap();
        assert_eq!((c.volume, c.muted), (100, false));
        let t: Timeline =
            serde_json::from_str(r#"{"clips":[],"fades":[],"next_clip_id":0}"#).unwrap();
        assert!(!t.start_fade() && !t.end_fade());
    }

    fn one() -> (Timeline, ClipId) {
        let mut t = Timeline::default();
        let id = t.append(&media(1, 8, true)).unwrap();
        (t, id)
    }

    fn sc(p: u16) -> Scale {
        Scale {
            permille: p,
            dx: 0,
            dy: 0,
        }
    }

    // covers: REQ-129
    // covers: REQ-130
    #[test]
    fn scale_and_move_start_with_one_key_and_a_second_add_does_nothing() {
        let (mut t, id) = one();
        let c = FrameTarget::Clip(id);
        assert!(t.add_scale(c).unwrap());
        assert!(!t.add_scale(c).unwrap());
        assert!(t.add_move(FrameTarget::Whole).unwrap());
        assert_eq!(
            t.scale_keys(c),
            &[Key {
                t: Micros::ZERO,
                v: Scale::NORMAL,
                ease: Default::default(),
            }]
        );
        // the copy is the clip's own (2.2.0): the whole video has no keys to read
        assert_eq!(t.move_keys(c).len(), 1);
        assert!(t.move_keys(FrameTarget::Whole).is_empty());
        assert!(t.has_frame_effect());
        t.remove_scale(c).unwrap();
        assert_eq!(t.remove_scale(c), Err(EditError::NoSuchEffect));
        assert_eq!(
            t.remove_move(FrameTarget::Whole),
            Err(EditError::NoSuchEffect)
        );
        t.remove_move(c).unwrap();
        assert!(t.clip_frames.is_empty(), "empty entries are removed");
        assert_eq!(
            t.add_scale(FrameTarget::Clip(ClipId(99))),
            Err(EditError::NoSuchClip)
        );
        assert_eq!(
            Timeline::default().add_move(FrameTarget::Whole),
            Err(EditError::EmptyTimeline)
        );
    }

    // covers: REQ-147, REQ-148
    #[test]
    fn a_whole_video_scale_is_a_copy_on_each_clip_each_edited_alone() {
        let mut t = track(); // clips of 4 s and 6 s
        let (a, b) = (t.clips[0].id, t.clips[1].id);
        let (ca, cb) = (FrameTarget::Clip(a), FrameTarget::Clip(b));
        let w = FrameTarget::Whole;
        assert!(t.add_scale(w).unwrap());
        assert!(!t.add_scale(w).unwrap(), "no second copy");
        assert_eq!(t.scale_keys(ca).len(), 1);
        assert_eq!(t.scale_keys(cb).len(), 1);
        // there is no whole-video state to read or edit any more (REQ-200)
        assert!(t.scale_keys(w).is_empty());
        assert_eq!(
            t.add_scale_key(w, s(2), sc(1500)),
            Err(EditError::NoSuchEffect)
        );
        // a key on clip 2 changes clip 2 only
        t.add_scale_key(cb, s(5), sc(2000)).unwrap();
        t.add_scale_key(cb, s(2), sc(1500)).unwrap();
        t.set_scale_key(cb, s(2), sc(700)).unwrap();
        assert_eq!(t.scale_keys(ca).len(), 1);
        assert_eq!(t.scale_keys(cb).len(), 3);
        assert_eq!(t.scale_keys(cb)[1].v.permille, 700);
        // removing clip 2's copy leaves clip 1's
        t.remove_scale(cb).unwrap();
        assert_eq!(t.scale_keys(ca).len(), 1);
        assert!(t.scale_keys(cb).is_empty());
        // deleting a clip takes its copy; a clip added later has none (REQ-147)
        t.remove(a).unwrap();
        assert!(t.frames_of(a).is_none());
        let c = t.append(&media(9, 3, true)).unwrap();
        assert!(t.frames_of(c).is_none());
        assert!(t.clip_frames.is_empty());
    }

    // covers: REQ-147
    #[test]
    fn a_whole_video_move_and_lut_are_one_copy_per_clip_and_fades_stay_with_clips() {
        let mut t = track();
        let (a, b) = (t.clips[0].id, t.clips[1].id);
        let w = FrameTarget::Whole;
        // a clip that has a Move keeps its own and gets no second copy
        t.add_move(FrameTarget::Clip(b)).unwrap();
        t.set_move_key(FrameTarget::Clip(b), t.clips[1].src_in, Pos { x: 1, y: 2 })
            .unwrap();
        assert!(t.add_move(w).unwrap());
        assert_eq!(t.move_keys(FrameTarget::Clip(b))[0].v, Pos { x: 1, y: 2 });
        assert_eq!(t.move_keys(FrameTarget::Clip(a))[0].v, frame::CENTRE);
        // the Apply LUT: one copy per clip, later clips have none
        t.set_lut(w, Some("/l/a.cube".into())).unwrap();
        let c = t.append(&media(9, 3, true)).unwrap();
        assert_eq!(t.lut_of(FrameTarget::Clip(a)), Some("/l/a.cube"));
        assert_eq!(t.lut_of(FrameTarget::Clip(c)), None);
        // the whole-video fade sits on the first and the last clip
        t.set_edge_fades(true).unwrap();
        assert!(t.clips[0].fade_in && t.clips[2].fade_out && !t.clips[1].fade_in);
        assert!(!t.clips[0].fade_out && !t.clips[2].fade_in);
        // a split keeps the start fade on the left half and the end fade on the right one
        let right = t.split(s(1), F).unwrap();
        assert!(t.get(a).unwrap().fade_in && !t.get(a).unwrap().fade_out);
        assert!(!t.get(right).unwrap().fade_in);
    }

    // covers: REQ-129
    #[test]
    fn keys_are_added_set_removed_and_clamped() {
        let (mut t, id) = one();
        let c = FrameTarget::Clip(id);
        assert_eq!(
            t.set_scale_key(c, s(1), sc(500)),
            Err(EditError::NoSuchEffect)
        );
        t.add_scale(c).unwrap();
        t.add_scale_key(c, s(4), sc(2000)).unwrap();
        assert_eq!(
            t.add_scale_key(c, s(4), sc(1)),
            Err(EditError::Key(KeyError::SameTime))
        );
        t.set_scale_key(c, s(4), sc(9000)).unwrap();
        assert_eq!(t.scale_keys(c)[1].v.permille, 4000);
        t.set_scale_key(c, s(2), sc(1)).unwrap();
        assert_eq!(t.scale_keys(c)[1].v.permille, 100);
        t.remove_scale_key(c, s(2)).unwrap();
        t.remove_scale_key(c, s(4)).unwrap();
        assert_eq!(
            t.remove_scale_key(c, Micros::ZERO),
            Err(EditError::Key(KeyError::LastKey))
        );
        t.add_move(c).unwrap();
        t.add_move_key(c, s(1), Pos { x: 60000, y: 1 }).unwrap();
        t.set_move_key(c, s(1), Pos { x: 65000, y: 2 }).unwrap();
        assert_eq!(t.move_keys(c)[1].v, Pos { x: 10000, y: 2 });
        t.remove_move_key(c, s(1)).unwrap();
        t.remove_move(c).unwrap();
        assert_eq!(
            t.add_move_key(c, s(1), frame::CENTRE),
            Err(EditError::NoSuchEffect)
        );
        assert_eq!(t.key_time(c, s(3)), Some(s(3)));
        assert_eq!(t.key_time(FrameTarget::Whole, s(3)), None);
        assert_eq!(t.key_time(FrameTarget::Clip(ClipId(9)), s(3)), None);
    }

    // covers: REQ-129
    #[test]
    fn split_gives_both_halves_the_path_and_trim_keeps_keys() {
        let (mut t, id) = one();
        let c = FrameTarget::Clip(id);
        t.add_scale(c).unwrap();
        t.add_scale_key(c, s(8), sc(2000)).unwrap();
        let right = t.split(s(4), F).unwrap();
        assert_eq!(t.scale_keys(c), t.scale_keys(FrameTarget::Clip(right)));
        // the value at the new edge is the one the path had there
        let st = t.frame_stack(1);
        let p = st.at(Micros::ZERO);
        assert!((p.factor - 1.5).abs() < 1e-9);
        t.trim(right, s(6), s(8), s(8), F).unwrap();
        assert_eq!(t.scale_keys(FrameTarget::Clip(right)).len(), 2);
        t.remove(right).unwrap();
        assert!(t.frames_of(right).is_none());
    }

    // covers: REQ-129
    #[test]
    fn merge_needs_equal_keys() {
        let (mut t, id) = one();
        let right = t.split(s(4), F).unwrap();
        assert!(t.can_merge(id));
        t.add_scale(FrameTarget::Clip(right)).unwrap();
        assert!(!t.can_merge(id));
        assert_eq!(t.merge_block_reason(id), Some(DIFFERENT_FRAMES));
        t.add_scale(FrameTarget::Clip(id)).unwrap();
        assert!(!t.can_merge(id));
        t.add_scale_key(FrameTarget::Clip(id), s(4), Scale::NORMAL)
            .unwrap();
        t.add_scale_key(FrameTarget::Clip(right), Micros::ZERO, Scale::NORMAL)
            .unwrap();
        t.merge(id).unwrap();
        assert!(t.frames_of(right).is_none());
        assert_eq!(t.scale_keys(FrameTarget::Clip(id)).len(), 2);
    }

    // covers: REQ-124
    // covers: REQ-131
    #[test]
    fn change_aspect_is_one_effect_of_the_whole_video() {
        let (mut t, _) = one();
        assert!(!t.has_frame_effect());
        t.set_frame_aspect(Some(FrameAspect::default())).unwrap();
        assert!(t.has_frame_effect());
        t.set_frame_aspect(None).unwrap();
        assert!(!t.has_frame_effect());
        assert_eq!(
            Timeline::default().set_frame_aspect(None),
            Err(EditError::EmptyTimeline)
        );
    }

    // covers: REQ-131
    #[test]
    fn the_stack_of_a_clip_has_the_whole_video_first() {
        let (mut t, id) = one();
        let right = t.split(s(4), F).unwrap();
        t.add_scale(FrameTarget::Whole).unwrap();
        t.add_move(FrameTarget::Clip(right)).unwrap();
        let st = t.frame_stack(1);
        assert_eq!(st.scale.len(), 1);
        assert_eq!(st.scale[0].offset, s(4));
        assert_eq!(st.moved[0].offset, s(4));
        assert!(t.frame_stack(0).moved.is_empty());
        assert!(t.frame_stack(7).is_empty());
        let _ = id;
    }

    // covers: REQ-132
    #[test]
    fn frame_fields_round_trip_and_old_timelines_have_none() {
        let (mut t, id) = one();
        t.add_scale(FrameTarget::Clip(id)).unwrap();
        t.add_move(FrameTarget::Whole).unwrap();
        t.set_frame_aspect(Some(FrameAspect::default())).unwrap();
        let j = serde_json::to_string(&t).unwrap();
        assert_eq!(serde_json::from_str::<Timeline>(&j).unwrap(), t);
        let old = Timeline::default();
        let j = serde_json::to_string(&old).unwrap();
        assert!(!j.contains("frame_aspect") && !j.contains("clip_frames"));
    }

    // covers: REQ-274
    #[test]
    fn the_fade_length_is_chosen_within_limits_and_a_short_clip_caps_it() {
        let mut t = three();
        t.toggle_fade(0).unwrap();
        // the usual length until chosen, and not written then
        assert_eq!(t.fades_of(0).1, Micros(500_000));
        assert!(!serde_json::to_string(&t).unwrap().contains("fade_length"));
        t.set_fade_length(Micros(1_500_000)).unwrap();
        assert_eq!(
            (t.fades_of(0).1, t.fades_of(1).0),
            (Micros(1_500_000), Micros(1_500_000))
        );
        assert!(serde_json::to_string(&t).unwrap().contains("fade_length"));
        let back: Timeline = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(back.fade_length, Micros(1_500_000));
        // a clip of 2 s allows half of its length
        let mut short = Timeline::default();
        short.append(&media(1, 2, true)).unwrap();
        short.append(&media(2, 5, true)).unwrap();
        short.toggle_fade(0).unwrap();
        short.set_fade_length(Micros(5_000_000)).unwrap();
        assert_eq!(short.fades_of(0).1, Micros(1_000_000));
        // limits: 0.1 s to 10 s
        assert_eq!(
            t.set_fade_length(Micros(99_999)),
            Err(EditError::FadeLength)
        );
        assert_eq!(
            t.set_fade_length(Micros(10_000_001)),
            Err(EditError::FadeLength)
        );
        assert!(t.set_fade_length(Micros(100_000)).is_ok());
    }

    // covers: REQ-275
    #[test]
    fn a_fade_can_go_through_black_on_one_side_only() {
        let mut t = three();
        t.toggle_fade(0).unwrap();
        let none = Micros::ZERO;
        let usual = Micros(500_000);
        assert_eq!((t.fades_of(0).1, t.fades_of(1).0), (usual, usual));
        t.set_fade_side(0, FadeSide::OutOnly).unwrap();
        assert_eq!(
            (t.fades_of(0).1, t.fades_of(1).0),
            (usual, none),
            "only the clip before"
        );
        t.set_fade_side(0, FadeSide::InOnly).unwrap();
        assert_eq!(
            (t.fades_of(0).1, t.fades_of(1).0),
            (none, usual),
            "only the clip after"
        );
        // the choice goes with the boundary when the left clip is split, and goes when the fade does
        t.split(s(2), Micros(100_000)).unwrap();
        assert_eq!(t.fade_side(1), FadeSide::InOnly);
        t.toggle_fade(1).unwrap();
        assert!(t.fade_sides.is_empty());
        assert_eq!(
            t.set_fade_side(1, FadeSide::OutOnly),
            Err(EditError::NoSuchBoundary)
        );
        t.toggle_fade(1).unwrap();
        assert_eq!(
            t.fade_side(1),
            FadeSide::Both,
            "a new fade starts with both"
        );
    }

    // covers: REQ-281
    #[test]
    fn a_clip_at_200_percent_lasts_half_and_the_timeline_ripples() {
        let mut t = track();
        let (a, b) = (t.clips[0].id, t.clips[1].id);
        assert_eq!(t.set_speed(a, 100), Err(EditError::SpeedRange));
        assert_eq!(t.set_speed(a, 2001), Err(EditError::SpeedRange));
        assert_eq!(t.set_speed(ClipId(99), 1000), Err(EditError::NoSuchClip));
        assert!(!t.any_speed());
        t.set_speed(a, 2000).unwrap();
        assert!(t.any_speed());
        assert_eq!(t.clips[0].len(), s(2), "4 s at 200 % occupy 2 s");
        assert_eq!(t.clips[0].src_len(), s(4));
        assert_eq!(t.start_of(1), s(2), "the next clip ripples");
        assert_eq!(t.duration(), s(8));
        t.set_speed(a, 500).unwrap();
        assert_eq!(t.clips[0].len(), s(8), "50 % doubles the length");
        assert_eq!(t.clips[0].src_at(s(2)), s(1));
        assert_eq!(t.clips[0].to_src(s(2)), s(1));
        // a split cuts the source where the timeline position is
        t.set_speed(a, 2000).unwrap();
        let right = t.split(s(1), Micros(100_000)).unwrap();
        assert_eq!(t.clips[0].src_out, s(2));
        assert_eq!(t.get(right).unwrap().src_in, s(2));
        assert_eq!(
            t.get(right).unwrap().speed,
            2000,
            "the halves keep the speed"
        );
        // merge needs the same speed
        assert_eq!(t.merge_block_reason(a), None);
        assert!(t.can_merge(a));
        t.set_speed(right, 1000).unwrap();
        assert_eq!(t.merge_block_reason(a), Some(DIFFERENT_SPEEDS));
        assert!(!t.can_merge(a));
        let _ = b;
    }

    // covers: REQ-281
    #[test]
    fn the_speed_is_saved_only_when_it_is_not_normal_and_keys_follow_it() {
        let mut t = track();
        let a = t.clips[0].id;
        let normal = serde_json::to_string(&t.clips[0]).unwrap();
        assert!(!normal.contains("speed"));
        t.set_speed(a, 1500).unwrap();
        let json = serde_json::to_string(&t.clips[0]).unwrap();
        assert!(json.contains("\"speed\":1500"));
        let back: Clip = serde_json::from_str(&json).unwrap();
        assert_eq!(back.speed, 1500);
        // keys stay in source time; the picture effects see them on the timeline's clock
        t.add_scale(FrameTarget::Clip(a)).unwrap();
        t.add_scale_key(FrameTarget::Clip(a), s(3), Scale::NORMAL)
            .unwrap();
        let st = t.frame_stack(0);
        let last = st.scale[0].keys.last().unwrap().t;
        let first = st.scale[0].keys.first().unwrap().t;
        assert_eq!(first, Micros::ZERO);
        assert_eq!(last, s(2), "3 s of source at 150 % is 2 s of timeline");
        // trimming by an edge drag moves the source by the timeline distance times the speed
        let (i, _) = t
            .trimmed_by_edge(a, ClipEdge::Start, s(1), s(4), Micros(100_000))
            .unwrap();
        assert_eq!(i, Micros(1_500_000));
    }
}
