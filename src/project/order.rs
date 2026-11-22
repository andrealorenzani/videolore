// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Applied effects of a clip as the project has them (REQ-302, REQ-304, REQ-311): the rows
//! it has now, their order, and the clips drawn by the overlay stage.

use super::Project;
use crate::effect_order::{self, Present, Row};
use crate::time::Micros;
use crate::timeline::{ClipId, FrameTarget};

/// What a clip has that can be moved or is fixed to the sphere, and where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipRows {
    pub clip: ClipId,
    /// Timeline start and end of the clip.
    pub start: Micros,
    pub end: Micros,
    pub present: Present,
    /// A 360 clip: its texts and lines fixed to the sphere are fixed rows, not movable.
    pub sphere: bool,
    /// Text items fixed to the sphere of this 360 clip (fixed rows).
    pub sphere_texts: Vec<u64>,
    /// The clip has subtitle lines fixed to the sphere (a fixed row).
    pub sphere_lines: bool,
}

impl Project {
    /// The rows of every clip, in clip order.
    pub fn clip_rows(&self) -> Vec<ClipRows> {
        let tl = &self.timeline;
        let lines = self.subtitles.project(tl);
        let mut start = Micros::ZERO;
        let mut out = Vec::with_capacity(tl.clips.len());
        for (index, c) in tl.clips.iter().enumerate() {
            let end = start + c.len();
            let sphere = self.media(c.media).is_some_and(|m| m.format.is_360());
            // the fades at the ends of the video belong to the first and last clip (REQ-083);
            // a fade between two clips is shown on both
            let fade = (index == 0 && c.fade_in)
                || (index > 0 && tl.has_fade(index - 1))
                || tl.has_fade(index)
                || (index + 1 == tl.clips.len() && c.fade_out);
            let mut present = Present {
                lut: tl.lut_of(FrameTarget::Clip(c.id)).is_some(),
                overlaps: tl.overlaps_of(c.id).iter().map(|o| o.id).collect(),
                frame: tl.frame_aspect.is_some() || tl.own_aspect(c.id).is_some(),
                scale: !tl.scale_keys(FrameTarget::Clip(c.id)).is_empty(),
                moved: !tl.move_keys(FrameTarget::Clip(c.id)).is_empty(),
                fade,
                ..Default::default()
            };
            let mut sphere_texts = Vec::new();
            for t in &self.texts.items {
                if t.start >= end || t.end() <= start {
                    continue;
                }
                if t.picture.is_some() {
                    present.pictures.push(t.id.0);
                } else if sphere && t.anchor.is_some() {
                    sphere_texts.push(t.id.0);
                } else {
                    present.texts.push(t.id.0);
                }
            }
            let mut sphere_lines = false;
            for l in lines.iter().filter(|l| l.clip == c.id) {
                if sphere && l.anchor.is_some() {
                    sphere_lines = true;
                } else {
                    present.subtitles = true;
                }
            }
            out.push(ClipRows {
                clip: c.id,
                start,
                end,
                present,
                sphere,
                sphere_texts,
                sphere_lines,
            });
            start = end;
        }
        out
    }

    /// The rows of one clip.
    pub fn rows_of(&self, clip: ClipId) -> Option<ClipRows> {
        self.clip_rows().into_iter().find(|r| r.clip == clip)
    }

    /// The order of the movable rows of a clip, top first (the stored order, or the order of
    /// the older versions).
    pub fn order_of(&self, clip: ClipId) -> Vec<Row> {
        self.rows_of(clip).map_or_else(Vec::new, |r| {
            effect_order::resolve(self.timeline.order_of(clip), &r.present)
        })
    }

    /// What the sound group of a clip has: its Volume (a level, a mute or keyframes) and its
    /// Clean audio.
    fn sound_has(&self, clip: ClipId) -> (bool, bool) {
        let tl = &self.timeline;
        tl.get(clip).map_or((false, false), |c| {
            (
                c.has_volume() || !tl.volume_keys(clip).is_empty(),
                c.clean_audio,
            )
        })
    }

    /// The order of the sound group of a clip, top (applied first) to bottom (REQ-320).
    pub fn sound_order(&self, clip: ClipId) -> Vec<Row> {
        let (v, c) = self.sound_has(clip);
        crate::order_chain::resolve_sound(self.timeline.order_of(clip), v, c)
    }

    /// True when the clip's Volume is applied before its Clean audio (not the default order).
    pub fn sound_swapped(&self, clip: ClipId) -> bool {
        let (v, c) = self.sound_has(clip);
        crate::order_chain::sound_swapped(self.timeline.order_of(clip), v, c)
    }

    /// Stores the sound group order of `clip`, keeping its picture rows (nothing is written
    /// for the default order).
    pub fn set_sound_order(&mut self, clip: ClipId, order: &[Row]) {
        let rows = crate::order_chain::with_sound(self.timeline.order_of(clip), order);
        self.timeline.set_order(clip, rows);
    }

    /// The clips whose order differs from the older one, with their rows top first: they are
    /// drawn by the overlay stage of the render.
    pub fn changed_orders(&self) -> Vec<(ClipRows, Vec<Row>)> {
        if self.timeline.clip_order.is_empty() {
            return Vec::new();
        }
        self.clip_rows()
            .into_iter()
            .filter_map(|r| {
                let stored = self.timeline.order_of(r.clip);
                effect_order::is_changed(stored, &r.present)
                    .then(|| (effect_order::resolve(stored, &r.present), r))
                    .map(|(o, r)| (r, o))
            })
            .collect()
    }

    /// Stores `order` for `clip` (nothing when it is the older order).
    pub fn set_order(&mut self, clip: ClipId, order: Vec<Row>) {
        let Some(r) = self.rows_of(clip) else { return };
        let rows = effect_order::to_store(order, &r.present);
        let rows = crate::order_chain::with_picture(self.timeline.order_of(clip), rows);
        self.timeline.set_order(clip, rows);
    }

    /// A new row goes on top of the clips it shows on (REQ-304): where the clip has other
    /// rows the order is stored with the new one first, unless that is the older order.
    /// `only` limits it to those clips (a LUT is new on the clips that had none).
    pub fn put_on_top(&mut self, row: Row, only: Option<&[ClipId]>) {
        for r in self.clip_rows() {
            if only.is_some_and(|o| !o.contains(&r.clip)) {
                continue;
            }
            // on top of the other movable rows only: a LUT on a clip that has no text, picture
            // or lines stays in the clip's own chain, under the frame, as before (2.2.0: the
            // rows that act on the picture are not part of "on top")
            let p = &r.present;
            let movable = usize::from(p.lut)
                + usize::from(p.subtitles)
                + p.texts.len()
                + p.pictures.len()
                + p.overlaps.len();
            if movable > 1 && p.has(row) {
                let mut order: Vec<Row> = self.order_of(r.clip);
                order.retain(|x| *x != row);
                order.insert(0, row);
                let rows = effect_order::to_store(order, &r.present);
                let rows = crate::order_chain::with_picture(self.timeline.order_of(r.clip), rows);
                self.timeline.set_order(r.clip, rows);
            }
        }
    }

    /// Takes damaged order entries out after a load (REQ-311): an entry of a clip that is not
    /// there, an unknown or repeated row, a text or picture that is not in the project. Each
    /// kind adds one line to `notes`.
    pub(super) fn repair_orders(&mut self, notes: &mut Vec<String>) {
        let ids: std::collections::BTreeSet<u64> =
            self.texts.items.iter().map(|t| t.id.0).collect();
        let mut damaged = false;
        let entries = std::mem::take(&mut self.timeline.clip_order);
        let mut seen = std::collections::BTreeSet::new();
        for e in entries {
            if self.timeline.get(e.clip).is_none() || !seen.insert(e.clip) {
                damaged = true;
                continue;
            }
            let mut rows: Vec<Row> = Vec::new();
            for r in &e.rows {
                let ok = match r {
                    Row::Unknown => false,
                    Row::Text(i) | Row::Picture(i) => ids.contains(i),
                    Row::Overlap(i) => self.timeline.overlaps_of(e.clip).iter().any(|o| o.id == *i),
                    // a crop row of format 13 went with the crop (no note)
                    Row::Crop => {
                        continue;
                    }
                    Row::Lut
                    | Row::Subtitles
                    | Row::Volume
                    | Row::Clean
                    | Row::Frame
                    | Row::Scale
                    | Row::Move
                    | Row::Fade => true,
                };
                if ok && !rows.contains(r) {
                    rows.push(*r);
                } else {
                    damaged = true;
                }
            }
            self.timeline.set_order(e.clip, rows);
        }
        if damaged {
            notes.push("A damaged effect order was repaired".to_string());
        }
    }
}
