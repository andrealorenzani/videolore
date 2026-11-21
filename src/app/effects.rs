// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Applying effects to a clip or to the whole video (REQ-082..084), the clip volume, the
//! fades at the video's edges, the selection of subtitle lines and their position on
//! screen (REQ-089..092), and the SRT export (REQ-088).

use super::*;
use crate::base::pos::POS_MAX;

/// Disabled reason of the SRT export.
pub const NO_SUBTITLES: &str = "No subtitles on the timeline";
/// Disabled reason of clip effects without a clip.
pub const SELECT_A_CLIP: &str = "Select a clip first";
/// Disabled reason of "Reset subtitle position".
pub const SELECT_A_LINE: &str = "Select a subtitle line first";

impl AppState {
    /// The clip an effect goes to (`None`: the whole video).
    pub(super) fn effect_clip(&self, target: Target) -> Result<Option<ClipId>, String> {
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
                    None => tl.start_fade() && tl.end_fade(),
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
            EffectKind::ChangeAspect | EffectKind::Scale | EffectKind::Move => {
                self.apply_frame_effect(effect, clip, now)
            }
            EffectKind::ApplyLut => {
                // nothing is set until a file is chosen: the popup asks for it
                self.show_instance(Instance::Lut(
                    clip.map_or(FrameTarget::Whole, FrameTarget::Clip),
                ));
                Ok(())
            }
            EffectKind::CleanAudio => {
                self.set_clean_audio(shown, true, now)?;
                match clip {
                    Some(id) => {
                        self.selected_clip = Some(id);
                        self.popups.instance = Some(Instance::CleanAudio(id));
                        self.show_effect(Popup::Instance, shown);
                    }
                    None => self.show_effect(Popup::CleanAudio, shown),
                }
                Ok(())
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
            (Popup::Instance, Some(Instance::CleanAudio(_))) => "Clean audio",
            (Popup::Instance, Some(Instance::FadeStart)) => "Fade from black",
            (Popup::Instance, Some(Instance::FadeEnd)) => "Fade to black",
            (Popup::Instance, Some(Instance::Pov(..))) => "Point of view",
            (Popup::Instance, Some(Instance::Aspect)) => "Change aspect",
            (Popup::Instance, Some(Instance::Scale(t))) => return self.frame_title("Scale", t),
            (Popup::Instance, Some(Instance::Move(t))) => return self.frame_title("Move", t),
            (Popup::Instance, Some(Instance::Lut(t))) => return self.frame_title("Apply LUT", t),
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

    fn frame_title(&self, base: &str, t: FrameTarget) -> String {
        match t {
            FrameTarget::Whole => format!("{base} — whole video"),
            FrameTarget::Clip(id) => match self.clip_name(id) {
                Some(n) => format!("{base} — {n}"),
                None => base.to_string(),
            },
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

    /// Clean audio on or off for a clip or every clip (REQ-114): one undo step, nothing
    /// when every clip concerned already has that setting.
    pub(super) fn set_clean_audio(
        &mut self,
        target: Target,
        on: bool,
        now: Instant,
    ) -> Result<(), String> {
        let ids: Vec<ClipId> = match self.effect_clip(target)? {
            Some(id) => vec![id],
            None => self.project.timeline.clips.iter().map(|c| c.id).collect(),
        };
        let tl = &self.project.timeline;
        if ids
            .iter()
            .all(|i| tl.get(*i).is_some_and(|c| c.clean_audio == on))
        {
            return Ok(());
        }
        let label = if on {
            "Clean audio"
        } else {
            "Remove clean audio"
        };
        self.edit(now, label, |e| {
            e.timeline.set_clean_audio(&ids, on).map_err(err)
        })
    }

    /// Normalise speech on the clip or the whole video (REQ-279): one undo step.
    pub(super) fn set_speech_enhance(
        &mut self,
        target: Target,
        on: bool,
        now: Instant,
    ) -> Result<(), String> {
        let ids: Vec<ClipId> = match self.effect_clip(target)? {
            Some(id) => vec![id],
            None => self.project.timeline.clips.iter().map(|c| c.id).collect(),
        };
        let tl = &self.project.timeline;
        if ids
            .iter()
            .all(|i| tl.get(*i).is_some_and(|c| c.speech_enhance == on))
        {
            return Ok(());
        }
        let label = if on {
            "Normalise speech"
        } else {
            "Remove normalise speech"
        };
        self.edit(now, label, |e| {
            e.timeline.set_speech_enhance(&ids, on).map_err(err)
        })
    }

    /// How strong Clean audio is on the clip or the whole video (REQ-278): one undo step.
    pub(super) fn set_clean_strength(
        &mut self,
        target: Target,
        percent: u8,
        now: Instant,
    ) -> Result<(), String> {
        if percent > 100 {
            return Err("The strength is from 0 to 100 %".into());
        }
        let ids: Vec<ClipId> = match self.effect_clip(target)? {
            Some(id) => vec![id],
            None => self.project.timeline.clips.iter().map(|c| c.id).collect(),
        };
        let tl = &self.project.timeline;
        if ids
            .iter()
            .all(|i| tl.get(*i).is_some_and(|c| c.clean_strength == percent))
        {
            return Ok(());
        }
        self.edit(now, "Clean audio strength", |e| {
            e.timeline.set_clean_strength(&ids, percent).map_err(err)
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
            Edge::Start => tl.start_fade(),
            Edge::End => tl.end_fade(),
        };
        if current == on {
            return Ok(());
        }
        self.edit(now, fade_label(!on), |e| {
            match edge {
                Edge::Start => e.timeline.set_start_fade(on),
                Edge::End => e.timeline.set_end_fade(on),
            }
            .map_err(err)
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
        self.selection.click(ObjRef::Line(id), true);
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
        let mut picked: Vec<(ObjRef, Micros)> = self
            .shown_ids()
            .into_iter()
            .filter(|(_, start)| *start >= a && *start <= b)
            .map(|(id, start)| (ObjRef::Line(id), start))
            .collect();
        picked.extend(
            self.project
                .texts
                .items
                .iter()
                .filter(|t| t.start >= a && t.start <= b)
                .map(|t| (ObjRef::Text(t.id), t.start)),
        );
        let first = picked.iter().map(|(_, start)| *start).min();
        let objs: Vec<ObjRef> = picked.iter().map(|(o, _)| *o).collect();
        self.selection.pick(&objs, add);
        if let Some(start) = first {
            self.seek(start, now);
        }
        Ok(())
    }

    /// The lines "Reset position" and a group drag act on.
    pub fn selected_lines(&self) -> Vec<LineId> {
        self.selection.lines()
    }

    /// The text items the Context Menu commands act on.
    pub fn selected_texts(&self) -> Vec<crate::text::TextId> {
        self.selection.texts()
    }

    /// The lines a drag of `id` moves: the selection when `id` is in it, else `id` alone.
    pub fn drag_group(&self, id: LineId) -> Vec<LineId> {
        self.selection
            .drag_targets(ObjRef::Line(id))
            .into_iter()
            .filter_map(|o| match o {
                ObjRef::Line(l) => Some(l),
                ObjRef::Text(_) => None,
            })
            .collect()
    }

    /// The subtitle lines a drag of the text `id` moves along (REQ-172): the lines of the
    /// selection when `id` is in it with others.
    pub fn drag_group_lines_of_text(&self, id: crate::text::TextId) -> Vec<LineId> {
        self.selection
            .drag_targets(ObjRef::Text(id))
            .into_iter()
            .filter_map(|o| match o {
                ObjRef::Line(l) => Some(l),
                ObjRef::Text(_) => None,
            })
            .collect()
    }

    /// The text items a drag of the line `id` moves along (REQ-172).
    pub fn drag_group_texts_of_line(&self, id: LineId) -> Vec<crate::text::TextId> {
        self.selection
            .drag_targets(ObjRef::Line(id))
            .into_iter()
            .filter_map(|o| match o {
                ObjRef::Text(t) => Some(t),
                ObjRef::Line(_) => None,
            })
            .collect()
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
        if self.drag_pos.is_none()
            && self.resize_drag.is_none()
            && self.frame_drag.is_none()
            && overlay.is_empty()
        {
            return std::borrow::Cow::Borrowed(&self.project);
        }
        let mut q = self.project.clone();
        if let Some((ids, p)) = &self.drag_pos {
            let _ = q.subtitles.set_position(ids, Some(*p));
        }
        self.with_frame_drag(&mut q);
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
                    ease: Default::default(),
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

    /// Writes the timeline's subtitles as `.srt`, `.vtt` or `.ass` by the file's extension
    /// (atomically: no partial file is left; REQ-180, REQ-181).
    pub(super) fn export_srt(&mut self, mut path: PathBuf) -> Result<(), String> {
        use crate::srt::Format;
        if !self.has_shown_subtitles() {
            return Err(NO_SUBTITLES.into());
        }
        if path.extension().is_none() {
            path.set_extension("srt");
        }
        let lines = self.project.subtitles.project(&self.project.timeline);
        let text = match Format::of_path(&path) {
            Format::Srt => crate::srt::write(&lines),
            Format::Vtt => crate::srt::write_vtt(&lines),
            Format::Ass => {
                let (w, h) = self.output_size().unwrap_or((1920, 1080));
                crate::ass::export_file_styled(
                    &lines,
                    self.project.subtitles.preset,
                    &self.project.subtitles.style,
                    w,
                    h,
                )
            }
        };
        project::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())?;
        self.status = format!("Subtitles saved to {}", path.display());
        Ok(())
    }

    /// Removes lines that no longer exist from the selection.
    pub(super) fn prune_selection(&mut self) {
        let subs = &self.project.subtitles;
        let texts = &self.project.texts;
        self.selection
            .prune(|id| subs.get(id).is_some(), |id| texts.get(id).is_some());
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
        EffectKind::Fade => "Fade IN/OUT",
        EffectKind::Subtitles => "Create subtitles",
        EffectKind::Volume => "Volume",
        EffectKind::CleanAudio => "Clean audio",
        EffectKind::ChangeAspect => "Change aspect",
        EffectKind::Scale => "Scale",
        EffectKind::Move => "Move",
        EffectKind::ApplyLut => "Apply LUT",
    }
}
