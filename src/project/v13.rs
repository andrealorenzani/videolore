// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The conversion of a project of format 13 or older to format 14 (2.3.0, ADR-241, REQ-330,
//! REQ-332), made in memory when the file is opened, once, with no undo step and no change
//! mark (as the format 9 conversion, `v9`): the crop of a clip is dropped (Crop was removed,
//! REQ-283), and each picture item whose start lies inside a clip becomes an overlap of that
//! clip with the same picture, place and times (REQ-330). Volume keys and speeds need no
//! change: they are the clip's Volume and Change speed effects as stored.

use super::Project;
use crate::effect_order::Row;
use crate::overlap::{Kind, Overlap};
use crate::timeline::ClipId;
use std::collections::BTreeMap;

/// What the conversion changed, for the one status line of REQ-332.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Conversion {
    /// Clips that lost a crop or got an overlap.
    pub clips: usize,
    /// Crops dropped.
    pub crops: usize,
    /// Picture items that became overlaps.
    pub pictures: usize,
}

impl Conversion {
    /// The one opening status line (REQ-332); the picture count is part of the same line.
    pub fn message(&self) -> String {
        let mut s = format!(
            "Opening: {} clip(s) changed by the conversion ({} crop(s) dropped",
            self.clips, self.crops
        );
        if self.pictures > 0 {
            s.push_str(&format!(", {} picture(s) made overlaps", self.pictures));
        }
        s.push(')');
        s
    }
}

/// Converts `p` in place; `None` when nothing changed (no status line).
pub(super) fn convert(p: &mut Project) -> Option<Conversion> {
    let mut changed: Vec<ClipId> = Vec::new();
    let mut crops = 0;
    for c in &mut p.timeline.clips {
        if !c.crop.is_none() {
            crops += 1;
            changed.push(c.id);
        }
        c.crop = Default::default();
    }
    // the picture items that lie under a clip's start become overlaps of that clip
    let mut made: BTreeMap<u64, u64> = BTreeMap::new();
    let mut gone: Vec<u64> = Vec::new();
    let mut items = std::mem::take(&mut p.texts.items);
    items.retain(|t| {
        let Some(pic) = &t.picture else { return true };
        let Some((i, clip_start)) = p.timeline.locate(t.start) else {
            return true;
        };
        let from = t.start - clip_start;
        let clip = p.timeline.clips[i];
        let clip_len = clip.len();
        let end = (from + t.ttl).min(clip_len);
        if end <= from || !pic.is_valid() {
            return true;
        }
        let id = p.timeline.new_overlap_id();
        let o = Overlap {
            id,
            path: pic.path.clone(),
            kind: Kind::Still,
            from,
            len: end - from,
            place: t.place,
            width: t.width.clamp(1, 1000),
            height: t.height.clamp(1, 1000),
            reshaped: false,
            src_w: pic.width,
            src_h: pic.height,
        };
        let mut list = p.timeline.overlaps_of(clip.id).to_vec();
        list.push(o);
        p.timeline.put_overlaps(clip.id, list);
        made.insert(t.id.0, id);
        gone.push(t.id.0);
        if !changed.contains(&clip.id) {
            changed.push(clip.id);
        }
        false
    });
    p.texts.items = items;
    let pictures = made.len();
    // the rows follow: `picture:<id>` of the clip that took the item becomes `overlap:<new>`;
    // the rows of the other clips the item showed on go (nothing of it is left there)
    let entries = std::mem::take(&mut p.timeline.clip_order);
    for mut e in entries {
        let own: Vec<u64> = p
            .timeline
            .overlaps_of(e.clip)
            .iter()
            .map(|o| o.id)
            .collect();
        e.rows = e
            .rows
            .into_iter()
            .filter_map(|r| match r {
                Row::Picture(i) if gone.contains(&i) => made
                    .get(&i)
                    .filter(|n| own.contains(n))
                    .map(|n| Row::Overlap(*n)),
                Row::Crop => None,
                r => Some(r),
            })
            .collect();
        p.timeline.set_order(e.clip, e.rows);
    }
    if changed.is_empty() {
        return None;
    }
    Some(Conversion {
        clips: changed.len(),
        crops,
        pictures,
    })
}
