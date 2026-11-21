// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Applied effects of a clip (REQ-302 to REQ-304): the rows the widget lists, whether each
//! is an effect of the whole video, and moving a movable row (one undo step).

use super::*;
use crate::effect_order::{self, Row};

/// How a movable row is moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowMove {
    Up,
    Down,
    /// Dragged to this place of the list (0 = the top).
    To(usize),
}

/// One row of the Applied effects widget.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedRow {
    pub name: String,
    /// The time range of a text or picture item.
    pub range: Option<(Micros, Micros)>,
    /// The movable row it stands for; `None` for a fixed row.
    pub row: Option<Row>,
    /// The effect was applied to the whole video (REQ-303).
    pub whole: bool,
    /// What a click does: select the clip and open the popup of the effect.
    pub open: Vec<Command>,
}

/// The effects of one clip, for the widget.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub clip: ClipId,
    /// The movable rows, top first, then the fixed rows in the order the picture is built.
    pub rows: Vec<AppliedRow>,
}

impl Applied {
    /// How many movable rows there are (they come first).
    pub fn movable(&self) -> usize {
        self.rows.iter().take_while(|r| r.row.is_some()).count()
    }
}

/// The names of the undo steps of a move.
pub(super) fn move_label(row: Row, mv: RowMove) -> &'static str {
    use effect_order::noun;
    match (noun(row), mv) {
        ("LUT", RowMove::Up) => "Move LUT up",
        ("LUT", RowMove::Down) => "Move LUT down",
        ("text", RowMove::Up) => "Move text up",
        ("text", RowMove::Down) => "Move text down",
        ("picture", RowMove::Up) => "Move picture up",
        ("picture", RowMove::Down) => "Move picture down",
        ("subtitles", RowMove::Up) => "Move subtitles up",
        ("subtitles", RowMove::Down) => "Move subtitles down",
        _ => "Move effect",
    }
}

/// A line of a text shortened for a row.
fn short(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    let mut out: String = line.chars().take(24).collect();
    if line.chars().count() > 24 {
        out.push('…');
    }
    out
}

impl AppState {
    /// The effects of the selected clip, or of the clip at the playhead (REQ-302); `None`
    /// with no clip there.
    pub fn applied(&self) -> Option<Applied> {
        let id = self.target_clip()?;
        let tl = &self.project.timeline;
        let i = tl.index_of(id)?;
        let c = tl.clips[i];
        let rows = self.project.rows_of(id)?;
        let order = effect_order::resolve(tl.order_of(id), &rows.present);
        let many = tl.clips.len() > 1;
        let all = |same: &dyn Fn(&crate::timeline::Clip) -> bool| many && tl.clips.iter().all(same);
        let open_instance =
            |i: Instance| vec![Command::SelectClip(Some(id)), Command::OpenInstance(i)];
        let open_popup = |p: Popup| vec![Command::SelectClip(Some(id)), Command::OpenPopup(p)];
        let mut out = Vec::new();
        // the movable rows, top first
        for row in order {
            let (name, range, open, whole) = match row {
                Row::Lut => {
                    let path = tl.lut_of(FrameTarget::Clip(id));
                    let name = path.map_or_else(
                        || "Apply LUT".to_string(),
                        |p| format!("Apply LUT: {}", crate::lut::display_name(p)),
                    );
                    let same = all(&|k| {
                        tl.lut_of(FrameTarget::Clip(k.id)) == path
                            && tl.lut_strength_of(FrameTarget::Clip(k.id))
                                == tl.lut_strength_of(FrameTarget::Clip(id))
                    });
                    (
                        name,
                        None,
                        open_instance(Instance::Lut(FrameTarget::Clip(id))),
                        path.is_some() && same,
                    )
                }
                Row::Text(t) | Row::Picture(t) => {
                    let Some(item) = self.project.texts.get(crate::text::TextId(t)) else {
                        continue;
                    };
                    let name = match &item.picture {
                        Some(p) => format!("Picture: {}", short(&p.name())),
                        None => format!("Text: {}", short(&item.text)),
                    };
                    (
                        name,
                        Some((item.start, item.end())),
                        vec![Command::OpenText(item.id)],
                        false,
                    )
                }
                Row::Subtitles => (
                    "Subtitles".to_string(),
                    None,
                    open_popup(Popup::Subtitles),
                    false,
                ),
                Row::Unknown => continue,
            };
            out.push(AppliedRow {
                name,
                range,
                row: Some(row),
                whole,
                open,
            });
        }
        let mut fixed = |name: &str, open: Vec<Command>, whole: bool| {
            out.push(AppliedRow {
                name: name.to_string(),
                range: None,
                row: None,
                whole,
                open,
            });
        };
        // on a 360 clip the texts and lines fixed to the sphere are drawn first and stay put
        for t in &rows.sphere_texts {
            let name = self
                .project
                .texts
                .get(crate::text::TextId(*t))
                .map_or_else(String::new, |x| {
                    format!("Text fixed to the sphere: {}", short(&x.text))
                });
            fixed(
                &name,
                vec![Command::OpenText(crate::text::TextId(*t))],
                false,
            );
        }
        if rows.sphere_lines {
            fixed(
                "Subtitles fixed to the sphere",
                open_popup(Popup::Subtitles),
                false,
            );
        }
        if !c.crop.is_none() {
            fixed("Crop", open_popup(Popup::ClipDetails), false);
        }
        for (name, keys) in [
            ("Scale", !tl.scale_keys(FrameTarget::Clip(id)).is_empty()),
            ("Move", !tl.move_keys(FrameTarget::Clip(id)).is_empty()),
        ] {
            if keys {
                let (instance, same) = if name == "Scale" {
                    let mine = tl.scale_keys(FrameTarget::Clip(id));
                    (
                        Instance::Scale(FrameTarget::Clip(id)),
                        all(&|k| tl.scale_keys(FrameTarget::Clip(k.id)) == mine),
                    )
                } else {
                    let mine = tl.move_keys(FrameTarget::Clip(id));
                    (
                        Instance::Move(FrameTarget::Clip(id)),
                        all(&|k| tl.move_keys(FrameTarget::Clip(k.id)) == mine),
                    )
                };
                fixed(name, open_instance(instance), same);
            }
        }
        if tl.own_aspect(id).is_some() {
            fixed("Change aspect", open_popup(Popup::ClipDetails), false);
        }
        if tl.frame_aspect.is_some() {
            fixed("Change aspect", open_instance(Instance::Aspect), true);
        }
        // the fade at the start and at the end of the video belong to the first and last
        // clip (REQ-083); the fades between two clips are shown on both
        if c.fade_in && i == 0 {
            fixed("Fade IN", open_instance(Instance::FadeStart), true);
        }
        if i > 0 && tl.has_fade(i - 1) {
            let prev = tl.clips[i - 1].id;
            fixed(
                "Fade IN/OUT with the previous clip",
                open_instance(Instance::Fade(prev, id)),
                false,
            );
        }
        if tl.has_fade(i) {
            let next = tl.clips[i + 1].id;
            fixed(
                "Fade IN/OUT with the next clip",
                open_instance(Instance::Fade(id, next)),
                false,
            );
        }
        if c.fade_out && i + 1 == tl.clips.len() {
            fixed("Fade OUT", open_instance(Instance::FadeEnd), true);
        }
        if c.has_volume() || !tl.volume_keys(id).is_empty() {
            let same = all(&|k| {
                (k.volume, k.muted) == (c.volume, c.muted)
                    && tl.volume_keys(k.id) == tl.volume_keys(id)
            });
            fixed("Volume", open_instance(Instance::ClipVolume(id)), same);
        }
        if c.clean_audio {
            let same = all(&|k| k.clean_audio && k.clean_strength == c.clean_strength);
            fixed("Clean audio", open_instance(Instance::CleanAudio(id)), same);
        }
        if c.speed != crate::timeline::NORMAL_SPEED {
            let same = all(&|k| k.speed == c.speed);
            fixed("Speed", open_popup(Popup::ClipDetails), same);
        }
        Some(Applied {
            clip: id,
            rows: out,
        })
    }

    /// Moves a movable row of `clip` one place up or down, or to a place of the list: one
    /// undo step named after the move (REQ-304).
    pub(super) fn move_row(
        &mut self,
        clip: ClipId,
        row: Row,
        mv: RowMove,
        now: Instant,
    ) -> Result<(), String> {
        let rows = self
            .project
            .rows_of(clip)
            .ok_or("That clip no longer exists")?;
        let order = effect_order::resolve(self.project.timeline.order_of(clip), &rows.present);
        let moved = match mv {
            RowMove::Up => effect_order::moved(&order, row, true),
            RowMove::Down => effect_order::moved(&order, row, false),
            RowMove::To(to) => effect_order::dragged(&order, row, to),
        };
        let Some(moved) = moved else {
            return if order.contains(&row) {
                Ok(())
            } else {
                Err("That effect is no longer on the clip".into())
            };
        };
        let store = effect_order::to_store(moved, &rows.present);
        self.edit(now, move_label(row, mv), |e| {
            e.timeline.set_order(clip, store);
            Ok(())
        })
    }

    /// Whether a movable row can move that way now, and why not.
    pub(super) fn row_move_reason(
        &self,
        clip: ClipId,
        row: Row,
        mv: RowMove,
    ) -> Option<&'static str> {
        let Some(rows) = self.project.rows_of(clip) else {
            return Some("That clip no longer exists");
        };
        let order = effect_order::resolve(self.project.timeline.order_of(clip), &rows.present);
        let Some(i) = order.iter().position(|r| *r == row) else {
            return Some("That effect is no longer on the clip");
        };
        match mv {
            RowMove::Up if i == 0 => Some("It is already the top row"),
            RowMove::Down if i + 1 == order.len() => Some("It is already the bottom row"),
            _ => None,
        }
    }
}
