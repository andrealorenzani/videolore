// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Overlap media on the timeline (2.3.0, REQ-325, REQ-327, REQ-329, REQ-331): adding an overlap
//! over the clip under the playhead, moving its box and its ends, removing it, the warning
//! before an edit removes overlaps, and the Change speed effect that sets the speed of one clip
//! or of all of them (REQ-281). Every change is one undo step.

use super::*;
use crate::effect_order::Row;
use crate::overlap::{self as model, Kind, Overlap};

/// The error that [`AppState::edit`] gives when an edit would remove overlaps: `dispatch` turns
/// it into the warning of REQ-329 (it is never shown as an error).
pub const REMOVAL_ASK: &str = "this edit removes overlaps";
/// Message of an overlap added with no clip under the playhead (REQ-325): nothing is added.
pub const NO_CLIP_UNDER_PLAYHEAD: &str = "Move the playhead over a video clip first";

/// What the warning of REQ-329 says: how many overlaps the edit would remove.
pub fn removal_text(n: usize) -> String {
    format!(
        "This edit will remove {n} overlap{}. OK does the edit and removes {}; Cancel changes nothing.",
        if n == 1 { "" } else { "s" },
        if n == 1 { "it" } else { "them" }
    )
}

fn oerr(e: crate::picture::PictureError) -> String {
    e.to_string()
}

impl AppState {
    /// Adds an overlap of `path` (the dialog when `None`) over the clip under the playhead:
    /// 3 s from the playhead, cut to the clip, in a box of the file's own ratio in the middle
    /// of the picture (REQ-325). With no clip there nothing is added and the status line says
    /// so; a broken file or one above 16 megapixels is refused with its message.
    pub(super) fn add_overlap(
        &mut self,
        path: Option<std::path::PathBuf>,
        now: Instant,
    ) -> Result<(), String> {
        let tl = &self.project.timeline;
        let (i, start) = tl.locate(self.playhead).ok_or(NO_CLIP_UNDER_PLAYHEAD)?;
        let clip = tl.clips[i];
        let Some(path) = path.or_else(|| self.dialogs.pick_picture()) else {
            return Ok(());
        };
        let runner = Arc::clone(&self.runner);
        let ffprobe = self.ffmpeg.as_ref().ok().map(|f| f.ffprobe.clone());
        let src = crate::picture::read_overlay(&path, |p| {
            let m = media::probe(&*runner, ffprobe.as_ref()?, p, MediaId(0)).ok()?;
            (m.format.has_video() && m.format.width > 0 && m.format.height > 0)
                .then_some((m.format.width, m.format.height))
        })
        .map_err(oerr)?;
        let frame = self.output_size().unwrap_or((1920, 1080));
        let pic = crate::picture::Picture {
            path: String::new(),
            width: src.width,
            height: src.height,
        };
        let (width, height) = pic.default_box(frame);
        let from = self.playhead - start;
        let len = model::DEFAULT_LEN.min(clip.len() - from);
        let mut added = None;
        self.edit(now, "Add overlap", |e| {
            let id = e.timeline.new_overlap_id();
            let o = Overlap {
                id,
                path: path.to_string_lossy().into_owned(),
                kind: if src.still { Kind::Still } else { Kind::Motion },
                from,
                len,
                place: Pos { x: 5000, y: 5000 },
                width,
                height,
                reshaped: false,
                src_w: src.width,
                src_h: src.height,
            };
            e.timeline.add_overlap(clip.id, o).map_err(err)?;
            added = Some(id);
            Ok(())
        })?;
        if let Some(id) = added {
            // a new overlap goes on top of the clip's other effects (REQ-304, REQ-331)
            self.project.put_on_top(Row::Overlap(id), None);
            self.selected_overlap = Some(id);
            self.selected_clip = Some(clip.id);
        }
        Ok(())
    }

    /// Moves or resizes the box of an overlap (REQ-327); the box is kept inside 1..=1000
    /// thousandths and its centre inside the picture.
    pub(super) fn set_overlap_box(
        &mut self,
        id: u64,
        place: Pos,
        width: u16,
        height: u16,
        reshaped: bool,
        now: Instant,
    ) -> Result<(), String> {
        let (_, cur) = self
            .project
            .timeline
            .find_overlap(id)
            .ok_or(EditError::NoSuchOverlap.to_string())?;
        let place = Pos {
            x: place.x.min(crate::base::pos::POS_MAX),
            y: place.y.min(crate::base::pos::POS_MAX),
        };
        let (width, height) = (width.clamp(1, 1000), height.clamp(1, 1000));
        let reshaped = reshaped || cur.reshaped;
        if (cur.place, cur.width, cur.height, cur.reshaped) == (place, width, height, reshaped) {
            return Ok(());
        }
        let label = if (cur.width, cur.height) == (width, height) {
            "Move overlap"
        } else {
            "Resize overlap"
        };
        self.edit(now, label, |e| {
            e.timeline
                .change_overlap(id, |o| {
                    o.place = place;
                    o.width = width;
                    o.height = height;
                    o.reshaped = reshaped;
                })
                .map_err(err)
        })
    }

    /// Moves the two ends of an overlap, kept inside its clip (REQ-325).
    pub(super) fn set_overlap_times(
        &mut self,
        id: u64,
        from: Micros,
        len: Micros,
        now: Instant,
    ) -> Result<(), String> {
        let (clip, cur) = self
            .project
            .timeline
            .find_overlap(id)
            .ok_or(EditError::NoSuchOverlap.to_string())?;
        let clip_len = self
            .project
            .timeline
            .get(clip)
            .ok_or(EditError::NoSuchClip.to_string())?
            .len();
        let new = model::drag_ends(cur, from, from + len, clip_len);
        if (new.from, new.len) == (cur.from, cur.len) {
            return Ok(());
        }
        self.edit(now, "Change overlap time", |e| {
            e.timeline
                .change_overlap(id, |o| {
                    o.from = new.from;
                    o.len = new.len;
                })
                .map_err(err)
        })
    }

    pub(super) fn remove_overlap(&mut self, id: u64, now: Instant) -> Result<(), String> {
        self.project
            .timeline
            .find_overlap(id)
            .ok_or(EditError::NoSuchOverlap.to_string())?;
        // removing an overlap is what the user asked for: no warning (REQ-329 is about the
        // overlaps an edit of their clip would remove)
        let was = std::mem::replace(&mut self.removal_confirmed, true);
        let done = self.edit(now, "Remove overlap", |e| {
            e.timeline.remove_overlap(id).map_err(err)
        });
        self.removal_confirmed = was;
        done?;
        if self.selected_overlap == Some(id) {
            self.selected_overlap = None;
        }
        Ok(())
    }

    /// OK of the warning of REQ-329: the waiting edit is done again with its removals.
    pub(super) fn confirm_removal(&mut self, now: Instant) -> Result<(), String> {
        self.modal = None;
        let Some(cmd) = self.pending_removal.take() else {
            return Ok(());
        };
        self.removal_confirmed = true;
        let r = self.dispatch(cmd, now);
        self.removal_confirmed = false;
        // overlaps that went with their clip are no longer selected
        if let Some(id) = self.selected_overlap
            && self.project.timeline.find_overlap(id).is_none()
        {
            self.selected_overlap = None;
        }
        r
    }

    /// Cancel of the warning of REQ-329: nothing changes.
    pub(super) fn cancel_removal(&mut self) -> Result<(), String> {
        self.modal = None;
        self.pending_removal = None;
        Ok(())
    }

    /// Change speed on every clip (REQ-281, REQ-147): one copy of the effect on each clip, one
    /// undo step; nothing happens when every clip already has the speed.
    pub(super) fn set_speed_all(&mut self, speed: u16, now: Instant) -> Result<(), String> {
        let tl = &self.project.timeline;
        if tl.clips.is_empty() {
            return Err(EditError::NoSuchClip.to_string());
        }
        if tl.clips.iter().all(|c| c.speed == speed) {
            return Ok(());
        }
        let ids: Vec<ClipId> = tl.clips.iter().map(|c| c.id).collect();
        self.edit(now, "Change speed", |e| {
            for id in ids {
                e.timeline.set_speed(id, speed).map_err(err)?;
            }
            Ok(())
        })
    }

    /// Takes the Volume effect off a clip: the keyframes go (REQ-280).
    pub(super) fn remove_volume(&mut self, clip: ClipId, now: Instant) -> Result<(), String> {
        self.project
            .timeline
            .get(clip)
            .ok_or(EditError::NoSuchClip.to_string())?;
        if self.project.timeline.volume_keys(clip).is_empty() {
            return Ok(());
        }
        self.edit(now, "Remove Volume", |e| {
            e.timeline.clear_volume_keys(clip).map_err(err)
        })
    }

    /// Gives a clip the Volume effect: one keyframe at the level of its slider (REQ-280).
    pub(super) fn add_volume_effect(&mut self, clip: ClipId, now: Instant) -> Result<(), String> {
        let c = *self
            .project
            .timeline
            .get(clip)
            .ok_or(EditError::NoSuchClip.to_string())?;
        if !self.project.timeline.volume_keys(clip).is_empty() {
            return Ok(());
        }
        let level = if c.muted { 0 } else { c.volume };
        self.edit(now, "Volume", |e| {
            e.timeline
                .set_volume_key(clip, Micros::ZERO, level)
                .map_err(err)
        })
    }
}
