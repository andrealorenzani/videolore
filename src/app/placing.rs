// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Where boxes end up (REQ-168 to REQ-176): the box of a subtitle line keeps its centre and
//! stays inside the picture when its text changes, a mixed selection moves as one group,
//! **Move inside the picture**, and the warnings that are derived from the project (lines
//! on top of each other, items partly outside). The rules themselves are in
//! [`crate::placement`]; this module applies them to the project in one undo step.

use super::*;
use crate::base::pos::POS_MAX;
use crate::placement;
use crate::selection::ObjRef;
use crate::subtitles::LineId;
use crate::text::TextId;

/// What **Move inside the picture** does: the lines and texts that move, with their new place.
pub(super) type InsidePlan = (Vec<(LineId, Pos)>, Vec<(TextId, Pos)>);

/// Disabled reason of **Move inside the picture** with every selected item inside.
pub const ALL_INSIDE: &str = "Everything selected is already inside the picture";

impl AppState {
    /// The place a line gets when its text changes to `new_text` (REQ-168, REQ-174), see
    /// [`placement::place_after_text_edit`].
    pub(super) fn place_after_text_edit(&self, id: LineId, new_text: &str) -> Option<Pos> {
        placement::place_after_text_edit(&self.project, id, new_text, self.output_size()?)
    }

    /// Moves lines and texts together to `place` (REQ-172): each item ends at the dragged
    /// item's final place. One undo step.
    pub(super) fn move_selection(
        &mut self,
        lines: Vec<LineId>,
        texts: Vec<TextId>,
        place: Pos,
        now: Instant,
    ) -> Result<(), String> {
        if (lines.is_empty() && texts.is_empty())
            || lines
                .iter()
                .any(|i| self.project.subtitles.get(*i).is_none())
            || texts.iter().any(|i| self.project.texts.get(*i).is_none())
        {
            return Err(sphere::SELECT_ON_PICTURE.into());
        }
        if !place.is_valid() {
            return Err(format!("a position must be between 0 and {POS_MAX}"));
        }
        let lines_same = lines.iter().all(|i| {
            self.project
                .subtitles
                .get(*i)
                .is_some_and(|l| l.position == Some(place))
        });
        let texts_same = texts.iter().all(|i| {
            self.project
                .texts
                .get(*i)
                .is_some_and(|t| t.anchor.is_some() || t.place == place)
        });
        self.drag_pos = None;
        if lines_same && texts_same {
            self.refresh_still(now);
            return Ok(());
        }
        self.edit(now, "Move", |e| {
            if !lines.is_empty() {
                e.subtitles
                    .set_position(&lines, Some(place))
                    .map_err(|e| e.to_string())?;
            }
            for t in e.texts.items.iter_mut().filter(|t| texts.contains(&t.id)) {
                if t.anchor.is_none() {
                    t.place = place;
                }
            }
            Ok(())
        })
    }

    /// The selected items that are partly outside the picture, with where each goes to be
    /// wholly inside (REQ-175): the least move; a box larger than the picture is centred.
    pub(super) fn inside_plan(&self) -> InsidePlan {
        let Some(frame) = self.output_size() else {
            return (Vec::new(), Vec::new());
        };
        let subs = &self.project.subtitles;
        let lines = self
            .selected_lines()
            .into_iter()
            .filter_map(|id| {
                let l = subs.get(id).filter(|l| l.anchor.is_none())?;
                let r = placement::line_rect(subs.preset, &l.text, l.position, l.size, frame);
                r.partly_outside(frame).then(|| {
                    let (dx, dy) = placement::least_inside(&r, frame);
                    (id, r.moved(dx, dy).pos(frame))
                })
            })
            .collect();
        let texts = self
            .selected_texts()
            .into_iter()
            .filter_map(|id| {
                let t = self.project.texts.get(id).filter(|t| t.anchor.is_none())?;
                let r = placement::text_rect(t, frame);
                r.partly_outside(frame).then(|| {
                    let (dx, dy) = placement::least_inside(&r, frame);
                    (id, r.moved(dx, dy).pos(frame))
                })
            })
            .collect();
        (lines, texts)
    }

    /// **Move inside the picture** for the selection (REQ-175). One undo step.
    pub(super) fn move_inside(&mut self, now: Instant) -> Result<(), String> {
        if self.selected_lines().is_empty() && self.selected_texts().is_empty() {
            return Err(sphere::SELECT_ON_PICTURE.into());
        }
        let (lines, texts) = self.inside_plan();
        if lines.is_empty() && texts.is_empty() {
            return Err(ALL_INSIDE.into());
        }
        self.edit(now, "Move inside the picture", |e| {
            for (id, pos) in &lines {
                e.subtitles
                    .set_position(&[*id], Some(*pos))
                    .map_err(|e| e.to_string())?;
            }
            for (id, place) in &texts {
                if let Some(t) = e.texts.items.iter_mut().find(|t| t.id == *id) {
                    t.place = *place;
                }
            }
            Ok(())
        })
    }

    /// The items shown at the playhead that are partly outside the picture (REQ-175,
    /// REQ-176).
    pub fn partly_outside(&self) -> Vec<ObjRef> {
        match self.output_size() {
            Some(f) => placement::partly_outside_at(&self.project, self.playhead, f),
            None => Vec::new(),
        }
    }

    /// The subtitle lines that are shown at the same time as another and whose boxes
    /// overlap (REQ-169). Derived from the project, so it is right after every edit.
    pub fn overlapping_lines(&self) -> std::collections::BTreeSet<LineId> {
        match self.output_size() {
            Some(f) => placement::overlapping_lines(&self.project, f),
            None => Default::default(),
        }
    }
}
