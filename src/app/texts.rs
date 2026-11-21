// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Text items on the timeline (REQ-108, REQ-110, REQ-111): adding, editing, moving in time,
//! deleting, moving and resizing their box in the preview. Every change is one undo step.

use super::*;
use crate::base::pos::POS_MAX;
use crate::camera::Camera;
use crate::text::{self, TextId, TextItem};

/// Disabled reason of adding a text outside the video.
pub const PLAYHEAD_IN_VIDEO: &str = "Move the playhead inside the video first";
/// Message of a text without any characters.
pub const TYPE_SOME_TEXT: &str = "Type some text first";
/// Disabled reason of Re-anchor without a selection fixed to the sphere (REQ-259).
pub const SELECT_ANCHORED: &str = "Select a subtitle line or text fixed to the 360 picture first";
/// Disabled reason of deleting a text.
pub const SELECT_A_TEXT: &str = "Select a text first";

fn terr(e: text::TextError) -> String {
    e.to_string()
}

impl AppState {
    /// The video length, as the limit of the time to live of a text.
    fn video_len(&self) -> Micros {
        self.project.timeline.duration()
    }

    /// A time to live inside `MIN_TTL..=video length`.
    fn clamp_ttl(&self, ttl: Micros) -> Micros {
        ttl.clamp(text::MIN_TTL, self.video_len().max(text::MIN_TTL))
    }

    pub(super) fn add_text(&mut self, draft: TextItem, now: Instant) -> Result<(), String> {
        let len = self.video_len();
        if self.project.timeline.clips.is_empty() || self.playhead >= len {
            return Err(PLAYHEAD_IN_VIDEO.into());
        }
        let text = text::clean_text(&draft.text).ok_or(TYPE_SOME_TEXT)?;
        let start = self.playhead;
        let anchor = self.text_anchor_at_playhead(true);
        let mut added = None;
        let ttl = self.clamp_ttl(draft.ttl);
        self.edit(now, "Add text", |e| {
            let id = e.texts.new_id();
            let mut item = TextItem::new(id, text, start);
            item.ttl = ttl;
            item.effects = draft.effects;
            item.colour = draft.colour;
            item.font = draft.font;
            item.readability = draft.readability;
            item.anchor = anchor;
            added = Some(e.texts.add(item).map_err(terr)?);
            Ok(())
        })?;
        if let Some(id) = added {
            self.selection.select_only(ObjRef::Text(id));
        }
        Ok(())
    }

    /// Adds a PNG picture at the playhead for 3 s, with the box of its proportions in the
    /// middle of the picture (REQ-287).
    pub(super) fn add_picture(
        &mut self,
        path: Option<std::path::PathBuf>,
        now: Instant,
    ) -> Result<(), String> {
        let len = self.video_len();
        if self.project.timeline.clips.is_empty() || self.playhead >= len {
            return Err(PLAYHEAD_IN_VIDEO.into());
        }
        let Some(path) = path.or_else(|| self.dialogs.pick_picture()) else {
            return Ok(());
        };
        let picture = crate::picture::read(&path).map_err(|e| e.to_string())?;
        let frame = self.output_size().unwrap_or((1920, 1080));
        let (start, ttl) = (self.playhead, self.clamp_ttl(text::DEFAULT_TTL));
        let mut added = None;
        self.edit(now, "Add picture", |e| {
            let id = e.texts.new_id();
            let mut item = TextItem::new_picture(id, picture, start, frame);
            item.ttl = ttl;
            added = Some(e.texts.add(item).map_err(terr)?);
            Ok(())
        })?;
        if let Some(id) = added {
            self.selection.select_only(ObjRef::Text(id));
        }
        Ok(())
    }

    pub(super) fn edit_text(&mut self, mut item: TextItem, now: Instant) -> Result<(), String> {
        let cur = self.project.texts.get(item.id).ok_or("no such text")?;
        item.text = text::clean_text(&item.text).ok_or(TYPE_SOME_TEXT)?;
        item.ttl = self.clamp_ttl(item.ttl);
        if *cur == item {
            return Ok(());
        }
        self.edit(now, "Edit text", |e| e.texts.replace(item).map_err(terr))
    }

    pub(super) fn move_text(
        &mut self,
        id: TextId,
        start: Micros,
        ttl: Micros,
        now: Instant,
    ) -> Result<(), String> {
        let mut item = self.project.texts.get(id).ok_or("no such text")?.clone();
        item.start = start.clamp(Micros::ZERO, self.video_len());
        item.ttl = self.clamp_ttl(ttl);
        if self.project.texts.get(id) == Some(&item) {
            return Ok(());
        }
        self.edit(now, "Move text", |e| e.texts.replace(item).map_err(terr))
    }

    pub(super) fn delete_texts(&mut self, ids: Vec<TextId>, now: Instant) -> Result<(), String> {
        if ids.is_empty() || ids.iter().any(|i| self.project.texts.get(*i).is_none()) {
            return Err(SELECT_A_TEXT.into());
        }
        self.edit(now, "Delete text", |e| {
            for id in &ids {
                e.texts.remove(*id).map_err(terr)?;
            }
            Ok(())
        })?;
        for id in ids {
            if self.popups.text == Some(id) {
                self.popups.close(Popup::Text);
            }
        }
        Ok(())
    }

    /// A box dragged in the preview: the place of every item in `ids` (a text fixed to the
    /// sphere keeps its flat place for flat clips). The centre may be anywhere on the
    /// picture (REQ-117).
    pub(super) fn set_text_boxes(
        &mut self,
        ids: Vec<TextId>,
        place: Pos,
        now: Instant,
    ) -> Result<(), String> {
        if ids.is_empty() || !place.is_valid() {
            return Err(SELECT_A_TEXT.into());
        }
        self.edit(now, "Move text", |e| {
            for id in &ids {
                let mut item = e.texts.get(*id).ok_or("no such text")?.clone();
                if item.anchor.is_none() {
                    item.place = place;
                }
                e.texts.replace(item).map_err(terr)?;
            }
            Ok(())
        })
    }

    /// An edge of a box dragged in the preview (REQ-116, REQ-159, REQ-161): the width (right
    /// or left edge) or the height (bottom edge) of every item in `ids`, within 10% to 100%
    /// of the picture width and 5% to 100% of its height. The opposite edge stays where it
    /// is, so the centre moves by half of the change and the box on screen is the box that
    /// was dragged. One undo step.
    pub(super) fn resize_text_box(
        &mut self,
        ids: Vec<TextId>,
        edge: TextEdge,
        value: u16,
        now: Instant,
    ) -> Result<(), String> {
        if ids.is_empty() || ids.iter().any(|i| self.project.texts.get(*i).is_none()) {
            return Err(SELECT_A_TEXT.into());
        }
        let resized = |t: &TextItem| {
            let mut t = t.clone();
            // one thousandth of the picture is ten place units; the centre moves by half
            let shift = |old: u16, new: u16| (i32::from(new) - i32::from(old)) * 5;
            let moved = |v: u16, d: i32| (i32::from(v) + d).clamp(0, i32::from(POS_MAX)) as u16;
            match edge {
                TextEdge::Right | TextEdge::Left => {
                    let w = value.clamp(text::WIDTH_MIN, text::WIDTH_MAX);
                    let d = shift(t.width, w);
                    t.place.x = moved(t.place.x, if edge == TextEdge::Right { d } else { -d });
                    t.width = w;
                }
                TextEdge::Bottom => {
                    let h = value.clamp(text::HEIGHT_MIN, text::HEIGHT_MAX);
                    t.place.y = moved(t.place.y, shift(t.height, h));
                    t.height = h;
                }
            }
            t
        };
        if ids
            .iter()
            .all(|i| self.project.texts.get(*i).is_some_and(|t| resized(t) == *t))
        {
            return Ok(());
        }
        self.edit(now, "Resize text", |e| {
            for id in &ids {
                let item = resized(e.texts.get(*id).ok_or("no such text")?);
                e.texts.replace(item).map_err(terr)?;
            }
            Ok(())
        })
    }

    /// The readability (none, outline or background box) of every item in `ids`
    /// (REQ-158). One undo step.
    pub(super) fn set_readability(
        &mut self,
        ids: Vec<TextId>,
        readability: text::Readability,
        now: Instant,
    ) -> Result<(), String> {
        if ids.is_empty() || ids.iter().any(|i| self.project.texts.get(*i).is_none()) {
            return Err(SELECT_A_TEXT.into());
        }
        if ids.iter().all(|i| {
            self.project
                .texts
                .get(*i)
                .is_some_and(|t| t.readability == readability)
        }) {
            return Ok(());
        }
        self.edit(now, "Text readability", |e| {
            for id in &ids {
                let mut item = e.texts.get(*id).ok_or("no such text")?.clone();
                item.readability = readability;
                e.texts.replace(item).map_err(terr)?;
            }
            Ok(())
        })
    }

    /// Like [`Self::resize_text_box`] for texts fixed to the sphere: their width is an
    /// angle (10 to 360 degrees).
    pub(super) fn resize_text_anchor_width(
        &mut self,
        ids: Vec<TextId>,
        width_cd: u32,
        now: Instant,
    ) -> Result<(), String> {
        if ids.is_empty()
            || ids.iter().any(|i| {
                self.project
                    .texts
                    .get(*i)
                    .is_none_or(|t| t.anchor.is_none())
            })
        {
            return Err("select a text fixed to the 360 picture first".into());
        }
        let width_cd = width_cd.clamp(1000, 36_000);
        self.edit(now, "Resize text", |e| {
            for id in &ids {
                let mut item = e.texts.get(*id).ok_or("no such text")?.clone();
                if let Some(a) = item.anchor.as_mut() {
                    a.width_cd = width_cd;
                }
                e.texts.replace(item).map_err(terr)?;
            }
            Ok(())
        })
    }

    /// Moves texts fixed to the sphere to the point (`yaw_cd`, `pitch_cd`).
    pub(super) fn move_text_anchor(
        &mut self,
        ids: Vec<TextId>,
        yaw_cd: i32,
        pitch_cd: i32,
        now: Instant,
    ) -> Result<(), String> {
        if yaw_cd.abs() > 18_000 || pitch_cd.abs() > 9_000 {
            return Err("that point is not on the sphere".into());
        }
        if ids.is_empty()
            || ids.iter().any(|i| {
                self.project
                    .texts
                    .get(*i)
                    .is_none_or(|t| t.anchor.is_none())
            })
        {
            return Err("select a text fixed to the 360 picture first".into());
        }
        self.edit(now, "Move text", |e| {
            for id in &ids {
                let mut item = e.texts.get(*id).ok_or("no such text")?.clone();
                if let Some(a) = item.anchor.as_mut() {
                    a.yaw_cd = yaw_cd;
                    a.pitch_cd = pitch_cd;
                }
                e.texts.replace(item).map_err(terr)?;
            }
            Ok(())
        })
    }

    /// Moves subtitle lines fixed to the sphere to the point (`yaw_cd`, `pitch_cd`); lines
    /// that are not fixed to it are left alone.
    pub(super) fn move_line_anchor(
        &mut self,
        ids: Vec<crate::subtitles::LineId>,
        yaw_cd: i32,
        pitch_cd: i32,
        now: Instant,
    ) -> Result<(), String> {
        if yaw_cd.abs() > 18_000 || pitch_cd.abs() > 9_000 {
            return Err("that point is not on the sphere".into());
        }
        let subs = &self.project.subtitles;
        let any = ids
            .iter()
            .any(|i| subs.get(*i).is_some_and(|l| l.anchor.is_some()));
        if !any {
            return Err("select a subtitle line fixed to the 360 picture first".into());
        }
        self.edit(now, "Move subtitle", |e| {
            for l in e.subtitles.lines.iter_mut().filter(|l| ids.contains(&l.id)) {
                if let Some(a) = l.anchor.as_mut() {
                    a.yaw_cd = yaw_cd;
                    a.pitch_cd = pitch_cd;
                }
            }
            Ok(())
        })
    }

    /// What Re-anchor moves: the camera at the playhead, the selected lines and texts that
    /// are fixed to the sphere. `None` when the playhead is not on a 360 clip or nothing
    /// selected is fixed to it.
    pub(super) fn re_anchor_plan(
        &self,
    ) -> Option<(Camera, Vec<crate::subtitles::LineId>, Vec<TextId>)> {
        let cam = self.camera_at_playhead()?;
        let lines: Vec<_> = self
            .selection
            .lines()
            .into_iter()
            .filter(|i| {
                self.project
                    .subtitles
                    .get(*i)
                    .is_some_and(|l| l.anchor.is_some())
            })
            .collect();
        let texts: Vec<_> = self
            .selection
            .texts()
            .into_iter()
            .filter(|i| {
                self.project
                    .texts
                    .get(*i)
                    .is_some_and(|t| t.anchor.is_some())
            })
            .collect();
        (!lines.is_empty() || !texts.is_empty()).then_some((cam, lines, texts))
    }

    /// Re-anchor (REQ-259): the selected items fixed to the sphere move to the centre of the
    /// current view, keeping their size; one undo step.
    pub(super) fn re_anchor(&mut self, now: Instant) -> Result<(), String> {
        let (cam, lines, texts) = self.re_anchor_plan().ok_or_else(|| {
            self.disabled_reason(&Command::ReAnchor)
                .unwrap_or("")
                .to_string()
        })?;
        self.edit(now, "Re-anchor", |e| {
            for l in e
                .subtitles
                .lines
                .iter_mut()
                .filter(|l| lines.contains(&l.id))
            {
                if let Some(a) = l.anchor.as_mut() {
                    (a.yaw_cd, a.pitch_cd) = (cam.yaw_cd, cam.pitch_cd);
                }
            }
            for t in e.texts.items.iter_mut().filter(|t| texts.contains(&t.id)) {
                if let Some(a) = t.anchor.as_mut() {
                    (a.yaw_cd, a.pitch_cd) = (cam.yaw_cd, cam.pitch_cd);
                }
            }
            Ok(())
        })
    }

    /// Opens the popup of a text: selects it and moves the playhead to its start.
    pub(super) fn open_text(&mut self, id: TextId, now: Instant) -> Result<(), String> {
        let start = self.project.texts.get(id).ok_or("no such text")?.start;
        self.selection.select_only(ObjRef::Text(id));
        self.popups.text = Some(id);
        self.popups.show(Popup::Text);
        self.seek(start, now);
        Ok(())
    }

    pub(super) fn toggle_text(&mut self, id: TextId) -> Result<(), String> {
        if self.project.texts.get(id).is_none() {
            return Err("no such text".into());
        }
        self.selection.click(ObjRef::Text(id), true);
        Ok(())
    }

    /// Where a text (`text`) or subtitle line added now is fixed on the sphere when the
    /// playhead is on a 360 clip: the point the camera looks at.
    pub fn text_anchor_at_playhead(&self, text: bool) -> Option<crate::base::pos::Anchor360> {
        let cam = self.camera_at_playhead()?;
        let (w, h) = self.output_size()?;
        Some(if text {
            crate::camera::text_anchor_at(cam, w, h)
        } else {
            crate::camera::anchor_at(cam, w, h)
        })
    }
}
