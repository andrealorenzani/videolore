// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Applied effects of a clip (REQ-302 to REQ-304): the rows the widget lists (a whole-video
//! effect is a copy on each clip and has no mark, REQ-303), and moving a movable row (one undo
//! step).

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
    /// What a click does: select the clip and open the popup of the effect.
    pub open: Vec<Command>,
}

/// The effects of one clip, for the widget.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub clip: ClipId,
    /// The picture group, top first (every row moves inside it), then the rows fixed to the
    /// sphere, the sound group and Speed.
    pub rows: Vec<AppliedRow>,
    /// How many rows the picture group has (they come first).
    pub picture: usize,
}

impl Applied {
    /// How many rows of the picture group there are (they come first): a drag ends among them.
    pub fn movable(&self) -> usize {
        self.picture
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
        ("Volume", RowMove::Up) => "Move Volume up",
        ("Volume", RowMove::Down) => "Move Volume down",
        ("Clean audio", RowMove::Up) => "Move Clean audio up",
        ("Clean audio", RowMove::Down) => "Move Clean audio down",
        ("overlap", RowMove::Up) => "Move overlap up",
        ("overlap", RowMove::Down) => "Move overlap down",
        ("Change aspect", RowMove::Up) => "Move Change aspect up",
        ("Change aspect", RowMove::Down) => "Move Change aspect down",
        ("Scale", RowMove::Up) => "Move Scale up",
        ("Scale", RowMove::Down) => "Move Scale down",
        ("Move effect", RowMove::Up) => "Move the Move effect up",
        ("Move effect", RowMove::Down) => "Move the Move effect down",
        ("Fade", RowMove::Up) => "Move Fade up",
        ("Fade", RowMove::Down) => "Move Fade down",
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

/// What the rows of the widget were built from: the clip and the state of the project (REQ-313).
pub(super) type AppliedKey = (ClipId, u64, usize, usize, usize);

impl AppState {
    /// The effects of the selected clip, or of the clip at the playhead (REQ-302); `None`
    /// with no clip there. The rows are kept and built again only when the clip, its order
    /// or its effects change: every edit, undo and redo count (REQ-313).
    pub fn applied(&self) -> Option<Applied> {
        let id = self.target_clip()?;
        let key: AppliedKey = (
            id,
            self.rows_serial,
            self.project.timeline.clips.len(),
            self.project.texts.items.len(),
            self.project.subtitles.lines.len(),
        );
        let mut cache = self.applied_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, rows)) = cache.as_ref()
            && *k == key
        {
            return rows.clone();
        }
        self.applied_builds
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rows = self.build_applied(id);
        *cache = Some((key, rows.clone()));
        rows
    }

    /// How many times the rows were built (for the test of REQ-313).
    pub fn applied_builds(&self) -> u32 {
        self.applied_builds
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    fn build_applied(&self, id: ClipId) -> Option<Applied> {
        let tl = &self.project.timeline;
        let i = tl.index_of(id)?;
        let c = tl.clips[i];
        let rows = self.project.rows_of(id)?;
        let order = effect_order::resolve(tl.order_of(id), &rows.present);
        let open_instance =
            |i: Instance| vec![Command::SelectClip(Some(id)), Command::OpenInstance(i)];
        let open_popup = |p: Popup| vec![Command::SelectClip(Some(id)), Command::OpenPopup(p)];
        let mut out = Vec::new();
        // the fades of the clip: at the start and at the end of the video (REQ-083), and with
        // the previous and the next clip; they are one row, "Fade IN/OUT" when there are several
        let mut fades: Vec<(&str, Vec<Command>)> = Vec::new();
        if c.fade_in && i == 0 {
            fades.push(("Fade IN", open_instance(Instance::FadeStart)));
        }
        if i > 0 && tl.has_fade(i - 1) {
            let prev = tl.clips[i - 1].id;
            fades.push((
                "Fade IN/OUT with the previous clip",
                open_instance(Instance::Fade(prev, id)),
            ));
        }
        if tl.has_fade(i) {
            let next = tl.clips[i + 1].id;
            fades.push((
                "Fade IN/OUT with the next clip",
                open_instance(Instance::Fade(id, next)),
            ));
        }
        if c.fade_out && i + 1 == tl.clips.len() {
            fades.push(("Fade OUT", open_instance(Instance::FadeEnd)));
        }
        // the picture group, top first (REQ-320): every row can move inside it
        for row in order {
            let (name, range, open) = match row {
                Row::Frame => {
                    let open = if tl.own_aspect(id).is_some() {
                        open_popup(Popup::ClipDetails)
                    } else {
                        open_instance(Instance::Aspect)
                    };
                    ("Change aspect".to_string(), None, open)
                }
                Row::Scale => (
                    "Scale".to_string(),
                    None,
                    open_instance(Instance::Scale(FrameTarget::Clip(id))),
                ),
                Row::Move => (
                    "Move".to_string(),
                    None,
                    open_instance(Instance::Move(FrameTarget::Clip(id))),
                ),
                Row::Fade => {
                    let Some((first, open)) = fades.first() else {
                        continue;
                    };
                    let name = if fades.len() == 1 {
                        (*first).to_string()
                    } else {
                        "Fade IN/OUT".to_string()
                    };
                    (name, None, open.clone())
                }
                Row::Lut => {
                    let path = tl.lut_of(FrameTarget::Clip(id));
                    let name = path.map_or_else(
                        || "Apply LUT".to_string(),
                        |p| format!("Apply LUT: {}", crate::lut::display_name(p)),
                    );
                    (
                        name,
                        None,
                        open_instance(Instance::Lut(FrameTarget::Clip(id))),
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
                    )
                }
                Row::Overlap(oid) => {
                    let Some((_, o)) = tl.find_overlap(oid) else {
                        continue;
                    };
                    let start = rows.start + o.from;
                    (
                        format!("Overlap: {}", short(&o.name())),
                        Some((start, start + o.len)),
                        vec![
                            Command::SelectClip(Some(id)),
                            Command::SelectOverlap(Some(oid)),
                        ],
                    )
                }
                Row::Subtitles => ("Subtitles".to_string(), None, open_popup(Popup::Subtitles)),
                Row::Volume | Row::Clean | Row::Unknown | Row::Crop => continue,
            };
            out.push(AppliedRow {
                name,
                range,
                row: Some(row),
                open,
            });
        }
        let picture = out.len();
        let mut fixed = |name: &str, open: Vec<Command>| {
            out.push(AppliedRow {
                name: name.to_string(),
                range: None,
                row: None,
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
            fixed(&name, vec![Command::OpenText(crate::text::TextId(*t))]);
        }
        if rows.sphere_lines {
            fixed(
                "Subtitles fixed to the sphere",
                open_popup(Popup::Subtitles),
            );
        }
        // the sound group, in the order of the sound build (top = applied first, REQ-320);
        // each of its rows moves inside the group only
        let sound: Vec<(Row, &str, Instance)> = self
            .project
            .sound_order(id)
            .into_iter()
            .map(|r| match r {
                Row::Volume => (r, "Volume and track", Instance::ClipVolume(id)),
                _ => (r, "Clean audio", Instance::CleanAudio(id)),
            })
            .collect();
        drop(fixed);
        for (row, name, instance) in sound {
            out.push(AppliedRow {
                name: name.to_string(),
                range: None,
                row: Some(row),
                open: open_instance(instance),
            });
        }
        if c.speed != crate::timeline::NORMAL_SPEED {
            // the fixed row applied first (REQ-320): the clip's Change speed (REQ-281)
            out.push(AppliedRow {
                name: format!("Change speed: {}%", c.speed / 10),
                range: None,
                row: None,
                open: open_instance(Instance::Speed(FrameTarget::Clip(id))),
            });
        }
        Some(Applied {
            clip: id,
            rows: out,
            picture,
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
        if row.is_sound() {
            // the sound group moves inside itself only (REQ-320, REQ-321)
            let order = self.project.sound_order(clip);
            if !order.contains(&row) {
                return Err("That effect is no longer on the clip".into());
            }
            let moved = match mv {
                RowMove::Up => crate::order_chain::moved_sound(&order, row, true),
                RowMove::Down => crate::order_chain::moved_sound(&order, row, false),
                RowMove::To(_) => None,
            };
            let Some(moved) = moved else { return Ok(()) };
            let rows = crate::order_chain::with_sound(self.project.timeline.order_of(clip), &moved);
            return self.edit(now, move_label(row, mv), |e| {
                e.timeline.set_order(clip, rows);
                Ok(())
            });
        }
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
        // the sound group keeps its stored order (a move of a picture row never touches it)
        let store = effect_order::to_store(moved, &rows.present);
        let store = crate::order_chain::with_picture(self.project.timeline.order_of(clip), store);
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
        if row.is_sound() {
            let order = self.project.sound_order(clip);
            let Some(i) = order.iter().position(|r| *r == row) else {
                return Some("That effect is no longer on the clip");
            };
            return match mv {
                RowMove::Up if i == 0 => Some("It is already the top row of its group"),
                RowMove::Down if i + 1 == order.len() => {
                    Some("It is already the bottom row of its group")
                }
                RowMove::To(_) => Some("A sound row moves with the buttons"),
                _ => None,
            };
        }
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
