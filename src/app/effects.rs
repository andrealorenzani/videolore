// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Applying effects to a clip or to the whole video (REQ-082..084), the clip volume, the
//! fades at the video's edges, the selection of subtitle lines and their position on
//! screen (REQ-089..092), and the SRT export (REQ-088).

use super::*;
use crate::subtitles::POS_MAX;

/// Disabled reason of the SRT export.
pub const NO_SUBTITLES: &str = "No subtitles on the timeline";
/// Disabled reason of clip effects without a clip.
pub const SELECT_A_CLIP: &str = "Select a clip first";
/// Disabled reason of "Reset subtitle position".
pub const SELECT_A_LINE: &str = "Select a subtitle line first";

impl AppState {
    /// The clip an effect goes to (`None`: the whole video).
    fn effect_clip(&self, target: Target) -> Result<Option<ClipId>, String> {
        match target {
            Target::WholeVideo => {
                if self.project.timeline.clips.is_empty() {
                    return Err("add clips to the timeline first".into());
                }
                Ok(None)
            }
            Target::Clip(id) => self
                .project
                .timeline
                .get(id)
                .map(|c| Some(c.id))
                .ok_or_else(|| EditError::NoSuchClip.to_string()),
            Target::Selected => self
                .target_clip()
                .map(Some)
                .ok_or_else(|| SELECT_A_CLIP.to_string()),
        }
    }

    /// Applies an effect: one undo step (Volume: when a value changes) and its popup opens.
    pub(super) fn apply_effect(
        &mut self,
        effect: EffectKind,
        target: Target,
        now: Instant,
    ) -> Result<(), String> {
        let clip = self.effect_clip(target)?;
        let shown = clip.map(Target::Clip).unwrap_or(Target::WholeVideo);
        match effect {
            EffectKind::Fade => {
                let tl = &self.project.timeline;
                let already = match clip {
                    Some(id) => tl.clip_has_fades(id),
                    None => tl.start_fade && tl.end_fade,
                };
                if !already {
                    self.edit(now, effect_label(effect), |e| match clip {
                        Some(id) => e.timeline.set_clip_fades(id, true).map_err(err),
                        None => e.timeline.set_edge_fades(true).map_err(err),
                    })?;
                }
                self.show_effect(Popup::Fades, shown);
                Ok(())
            }
            EffectKind::Volume => {
                match clip {
                    Some(id) => {
                        self.selected_clip = Some(id);
                        self.popups.instance = Some(Instance::ClipVolume(id));
                        self.show_effect(Popup::Instance, shown);
                    }
                    None => self.show_effect(Popup::Volume, shown),
                }
                Ok(())
            }
            EffectKind::Subtitles => {
                self.show_effect(Popup::Subtitles, shown);
                self.create_subtitles(clip)
            }
        }
    }

    fn show_effect(&mut self, p: Popup, t: Target) {
        self.popups.chooser = None;
        self.popups.show(p);
        self.popups.set_target(p, t);
    }

    /// Title of a popup: the effect, and what it was applied to ("— whole video",
    /// "— <clip name>").
    pub fn popup_title(&self, p: Popup) -> String {
        let base = match (p, self.popups.instance) {
            (Popup::Instance, Some(Instance::ClipVolume(_))) => "Clip volume",
            (Popup::Instance, Some(Instance::FadeStart)) => "Fade from black",
            (Popup::Instance, Some(Instance::FadeEnd)) => "Fade to black",
            (Popup::Instance, Some(Instance::Pov(..))) => "Point of view",
            _ => p.title(),
        };
        match self.popups.target(p) {
            Some(Target::WholeVideo) => format!("{base} — whole video"),
            Some(Target::Clip(id)) => match self.clip_name(id) {
                Some(n) => format!("{base} — {n}"),
                None => base.to_string(),
            },
            _ => base.to_string(),
        }
    }

    /// The file name of a clip's source.
    pub fn clip_name(&self, id: ClipId) -> Option<String> {
        let c = self.project.timeline.get(id)?;
        self.project.media(c.media).map(|m| m.name.clone())
    }

    pub(super) fn set_clip_volume(
        &mut self,
        clip: ClipId,
        volume: u8,
        muted: bool,
        now: Instant,
    ) -> Result<(), String> {
        let c = self
            .project
            .timeline
            .get(clip)
            .ok_or(EditError::NoSuchClip.to_string())?;
        let volume = volume.min(crate::timeline::FULL_VOLUME);
        if (c.volume, c.muted) == (volume, muted) {
            return Ok(());
        }
        self.edit(now, "Clip volume", |e| {
            e.timeline.set_clip_volume(clip, volume, muted).map_err(err)
        })
    }

    pub(super) fn set_edge_fade(
        &mut self,
        edge: Edge,
        on: bool,
        now: Instant,
    ) -> Result<(), String> {
        let tl = &self.project.timeline;
        if tl.clips.is_empty() {
            return Err(EditError::EmptyTimeline.to_string());
        }
        let current = match edge {
            Edge::Start => tl.start_fade,
            Edge::End => tl.end_fade,
        };
        if current == on {
            return Ok(());
        }
        self.edit(now, fade_label(!on), |e| {
            match edge {
                Edge::Start => e.timeline.start_fade = on,
                Edge::End => e.timeline.end_fade = on,
            }
            Ok(())
        })
    }

    /// Lines shown on the timeline (ids), in timeline order.
    fn shown_ids(&self) -> Vec<(LineId, Micros)> {
        self.project
            .subtitles
            .project(&self.project.timeline)
            .into_iter()
            .map(|l| (l.line, l.start))
            .collect()
    }

    pub(super) fn toggle_line(&mut self, id: LineId) -> Result<(), String> {
        if self.project.subtitles.get(id).is_none() {
            return Err("no such subtitle line".into());
        }
        if !self.selection.remove(&id) {
            self.selection.insert(id);
            self.selected_line = Some(id);
        } else if self.selected_line == Some(id) {
            self.selected_line = self.selection.iter().next().copied();
        }
        Ok(())
    }

    pub(super) fn select_range(
        &mut self,
        from: Micros,
        to: Micros,
        add: bool,
        now: Instant,
    ) -> Result<(), String> {
        let (a, b) = (from.min(to), from.max(to));
        let picked: Vec<(LineId, Micros)> = self
            .shown_ids()
            .into_iter()
            .filter(|(_, start)| *start >= a && *start <= b)
            .collect();
        if !add {
            self.selection.clear();
        }
        self.selection.extend(picked.iter().map(|(id, _)| *id));
        if let Some((id, start)) = picked.first() {
            self.selected_line = Some(*id);
            self.seek(*start, now);
        }
        Ok(())
    }

    /// The lines "Reset position" and a group drag act on.
    pub fn selected_lines(&self) -> Vec<LineId> {
        if !self.selection.is_empty() {
            return self.selection.iter().copied().collect();
        }
        self.selected_line.into_iter().collect()
    }

    /// The lines a drag of `id` moves: the selection when `id` is in it, else `id` alone.
    pub fn drag_group(&self, id: LineId) -> Vec<LineId> {
        if self.selection.contains(&id) && self.selection.len() > 1 {
            self.selection.iter().copied().collect()
        } else {
            vec![id]
        }
    }

    pub(super) fn set_line_positions(
        &mut self,
        ids: Vec<LineId>,
        pos: Option<Pos>,
        now: Instant,
    ) -> Result<(), String> {
        self.drag_pos = None;
        if ids.is_empty() {
            return Err(SELECT_A_LINE.into());
        }
        if pos.is_none() {
            // back to the style's place also gives the size back
            return self.reset_lines(ids, now);
        }
        if let Some(p) = pos
            && !p.is_valid()
        {
            return Err(format!("a position must be between 0 and {POS_MAX}"));
        }
        let subs = &self.project.subtitles;
        if ids
            .iter()
            .all(|id| subs.get(*id).is_some_and(|l| l.position == pos))
        {
            self.refresh_still(now);
            return Ok(());
        }
        self.edit(now, "Move subtitle", |e| {
            e.subtitles.set_position(&ids, pos).map_err(err)
        })
    }

    /// Shows lines at a position while they are dragged (nothing is saved).
    pub(super) fn drag_lines(&mut self, drag: Option<(Vec<LineId>, Pos)>, now: Instant) {
        self.drag_pos = drag.map(|(ids, p)| {
            (
                ids,
                Pos {
                    x: p.x.min(POS_MAX),
                    y: p.y.min(POS_MAX),
                },
            )
        });
        self.refresh_still(now);
    }

    /// The project as shown in the preview: with the lines being dragged at their new
    /// place.
    pub fn shown_project(&self) -> std::borrow::Cow<'_, Project> {
        let overlay = self.live_overlay();
        if self.drag_pos.is_none() && self.resize_drag.is_none() && overlay.is_empty() {
            return std::borrow::Cow::Borrowed(&self.project);
        }
        let mut q = self.project.clone();
        if let Some((ids, p)) = &self.drag_pos {
            let _ = q.subtitles.set_position(ids, Some(*p));
        }
        if let Some(d) = &self.resize_drag {
            let _ = q.subtitles.set_boxes(&d.ids, None, Some(Some(d.size)));
            let _ = q.subtitles.set_boxes(&[d.line], Some(Some(d.pos)), None);
        }
        // a camera you dragged is shown as one fixed view
        for (media, cam) in overlay {
            q.camera.drop_media(media);
            let _ = q.camera.set(
                media,
                crate::camera::Pov {
                    t: Micros::ZERO,
                    cam,
                },
            );
        }
        std::borrow::Cow::Owned(q)
    }

    /// True when some line with text is on the timeline (the SRT export has content).
    pub fn has_shown_subtitles(&self) -> bool {
        self.project
            .subtitles
            .project(&self.project.timeline)
            .iter()
            .any(|l| !l.text.trim().is_empty())
    }

    /// The name proposed for the SRT file: `<project name>.srt`.
    pub fn srt_name(&self) -> String {
        let stem = self
            .project_path
            .as_deref()
            .and_then(|p| p.file_stem())
            .map(|s| media::sanitize_for_display(&s.to_string_lossy()))
            .unwrap_or_else(|| "Untitled".into());
        format!("{stem}.srt")
    }

    /// Writes the timeline's subtitles as SRT (atomically: no partial file is left).
    pub(super) fn export_srt(&mut self, mut path: PathBuf) -> Result<(), String> {
        if !self.has_shown_subtitles() {
            return Err(NO_SUBTITLES.into());
        }
        if path.extension().is_none() {
            path.set_extension("srt");
        }
        let text = crate::srt::write(&self.project.subtitles.project(&self.project.timeline));
        project::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())?;
        self.status = format!("Subtitles saved to {}", path.display());
        Ok(())
    }

    /// Removes lines that no longer exist from the selection.
    pub(super) fn prune_selection(&mut self) {
        let subs = &self.project.subtitles;
        self.selection.retain(|id| subs.get(*id).is_some());
        if let Some((ids, _)) = &self.drag_pos
            && ids.iter().any(|id| subs.get(*id).is_none())
        {
            self.drag_pos = None;
        }
    }
}

/// Undo name of applying an effect.
fn effect_label(effect: EffectKind) -> &'static str {
    match effect {
        EffectKind::Fade => "Fade through black",
        EffectKind::Subtitles => "Create subtitles",
        EffectKind::Volume => "Volume",
    }
}
