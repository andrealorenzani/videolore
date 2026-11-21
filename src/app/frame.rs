// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Change aspect, Scale and Move (1.5.0, REQ-124..130): the commands, the popups' state and
//! the values the preview handles start from. The maths are in [`crate::frame`] and
//! [`crate::keyframes`]; every edit here is one undo step.

use super::*;
use crate::frame::FrameAspect;
use crate::keyframes;

/// Why Change aspect cannot go on a clip (REQ-130).
pub const ASPECT_WHOLE_ONLY: &str = "Change aspect applies to the whole video";
/// Reason for the keyframe buttons when no keyframe is selected.
pub const SELECT_A_KEY: &str = "Select a keyframe first";

impl AppState {
    /// Applies Change aspect, Scale or Move and opens its popup (REQ-124, REQ-130). A
    /// second Scale or Move on one clip opens the existing popup.
    pub(super) fn apply_frame_effect(
        &mut self,
        effect: EffectKind,
        clip: Option<ClipId>,
        now: Instant,
    ) -> Result<(), String> {
        let target = clip.map_or(FrameTarget::Whole, FrameTarget::Clip);
        let tl = &self.project.timeline;
        match effect {
            EffectKind::ChangeAspect => {
                if clip.is_some() {
                    return Err(ASPECT_WHOLE_ONLY.into());
                }
                if tl.frame_aspect.is_none() {
                    self.edit(now, "Change aspect", |e| {
                        e.timeline
                            .set_frame_aspect(Some(FrameAspect::default()))
                            .map_err(err)
                    })?;
                }
                self.show_instance(Instance::Aspect);
            }
            EffectKind::Scale => {
                if tl.scale_keys(target).is_empty() {
                    self.edit(now, "Scale", |e| {
                        e.timeline.add_scale(target).map(|_| ()).map_err(err)
                    })?;
                }
                self.show_instance(Instance::Scale(target));
            }
            EffectKind::Move => {
                if tl.move_keys(target).is_empty() {
                    self.edit(now, "Move", |e| {
                        e.timeline.add_move(target).map(|_| ()).map_err(err)
                    })?;
                }
                self.show_instance(Instance::Move(target));
            }
            _ => {}
        }
        Ok(())
    }

    /// Opens the popup of a picture effect without applying it (the effect's row).
    pub(super) fn open_frame_effect(&mut self, effect: EffectKind) {
        match effect {
            EffectKind::ChangeAspect => self.show_instance(Instance::Aspect),
            EffectKind::Scale => self.show_instance(Instance::Scale(FrameTarget::Whole)),
            EffectKind::Move => self.show_instance(Instance::Move(FrameTarget::Whole)),
            EffectKind::ApplyLut => self.show_instance(Instance::Lut(FrameTarget::Whole)),
            _ => {}
        }
    }

    pub(super) fn show_instance(&mut self, i: Instance) {
        self.popups.chooser = None;
        self.popups.instance = Some(i);
        self.popups.key = None;
        self.popups.show(Popup::Instance);
        if let Instance::Scale(FrameTarget::Clip(c))
        | Instance::Move(FrameTarget::Clip(c))
        | Instance::Lut(FrameTarget::Clip(c)) = i
        {
            self.selected_clip = Some(c);
        }
    }

    pub(super) fn set_aspect(
        &mut self,
        a: Option<FrameAspect>,
        now: Instant,
    ) -> Result<(), String> {
        if self.project.timeline.frame_aspect == a {
            return Ok(());
        }
        self.edit(
            now,
            if a.is_some() {
                "Change aspect"
            } else {
                "Remove effect"
            },
            |e| e.timeline.set_frame_aspect(a).map_err(err),
        )?;
        if a.is_none() && self.popups.instance == Some(Instance::Aspect) {
            self.popups.close(Popup::Instance);
        }
        Ok(())
    }

    /// The time of a key of `target` at the playhead.
    fn key_at_playhead(&self, target: FrameTarget) -> Result<Micros, String> {
        // the dashed handles of the whole-video copy work at the same time into their clip
        // as the playhead is into its own (REQ-202)
        if let FrameTarget::Clip(c) = target
            && (self.whole_handle_clip(FrameFx::Scale) == Some(c)
                || self.whole_handle_clip(FrameFx::Move) == Some(c))
            && let Some(t) = self.copy_key_time(c)
        {
            return Ok(t);
        }
        self.project
            .timeline
            .key_time(target, self.playhead)
            .ok_or_else(|| EditError::NoSuchClip.to_string())
    }

    /// The Scale shown for `target` at the playhead.
    pub fn scale_at(&self, target: FrameTarget) -> Option<Scale> {
        let t = self.key_at_playhead(target).ok()?;
        keyframes::at(self.project.timeline.scale_keys(target), t)
    }

    /// The Move shown for `target` at the playhead.
    pub fn move_at(&self, target: FrameTarget) -> Option<Pos> {
        let t = self.key_at_playhead(target).ok()?;
        keyframes::at(self.project.timeline.move_keys(target), t)
    }

    /// The effect whose handles the preview shows at the playhead: the copy of the clip
    /// under it (a whole-video Scale or Move is one copy per clip, REQ-148).
    pub fn frame_target_at_playhead(&self, effect: FrameFx) -> Option<FrameTarget> {
        let tl = &self.project.timeline;
        let (i, _) = tl.locate(self.playhead)?;
        let clip = FrameTarget::Clip(tl.clips[i].id);
        let has = match effect {
            FrameFx::Scale => !tl.scale_keys(clip).is_empty(),
            FrameFx::Move => !tl.move_keys(clip).is_empty(),
        };
        has.then_some(clip)
    }

    /// The copy the whole-video popup of `effect` shows: the selected row while that clip
    /// still has the effect, else the first clip that has it (REQ-200).
    pub fn selected_copy(&self, effect: FrameFx) -> Option<ClipId> {
        let tl = &self.project.timeline;
        let has = |id: ClipId| match effect {
            FrameFx::Scale => !tl.scale_keys(FrameTarget::Clip(id)).is_empty(),
            FrameFx::Move => !tl.move_keys(FrameTarget::Clip(id)).is_empty(),
        };
        self.popups
            .copy
            .filter(|c| has(*c))
            .or_else(|| tl.clips.iter().map(|c| c.id).find(|c| has(*c)))
    }

    /// Selects a row of the whole-video popup: its keyframes are the ones shown and
    /// edited, and the playhead goes to the start of that clip (REQ-200).
    pub(super) fn select_copy(&mut self, clip: ClipId, now: Instant) -> Result<(), String> {
        let tl = &self.project.timeline;
        let i = tl.index_of(clip).ok_or(EditError::NoSuchClip.to_string())?;
        let start = tl.start_of(i);
        self.popups.copy = Some(clip);
        self.popups.key = None;
        self.seek(start, now);
        Ok(())
    }

    /// "Copy keyframes": the clip's keys of `effect`, in the clip's own time (REQ-204).
    pub(super) fn copy_keys(&mut self, clip: ClipId, effect: FrameFx) -> Result<(), String> {
        let tl = &self.project.timeline;
        let keys = match effect {
            FrameFx::Scale => tl.copy_scale_keys(clip),
            FrameFx::Move => tl.copy_move_keys(clip),
        }
        .ok_or(EditError::NoSuchEffect.to_string())?;
        self.clipboard_keys = Some(keys);
        self.status = "Keyframes copied".into();
        Ok(())
    }

    /// "Paste keyframes to" `clip`: its keys of the copied effect become the copied ones, at
    /// the same clip times; its other effects stay (REQ-204). `effect` must be the copied one.
    pub(super) fn paste_keys(
        &mut self,
        clip: ClipId,
        effect: FrameFx,
        now: Instant,
    ) -> Result<(), String> {
        use crate::timeline::KeysClip;
        let keys = self
            .clipboard_keys
            .clone()
            .filter(|k| {
                matches!(
                    (k, effect),
                    (KeysClip::Scale(_), FrameFx::Scale) | (KeysClip::Move(_), FrameFx::Move)
                )
            })
            .ok_or("copy keyframes of that effect first")?;
        self.edit(now, "Paste keyframes", |e| {
            e.timeline.paste_keys(clip, &keys).map_err(err)
        })?;
        self.popups.key = None;
        Ok(())
    }

    /// Moves a keyframe in time (typed time, Left and Right: REQ-203).
    pub(super) fn move_key(
        &mut self,
        clip: ClipId,
        effect: FrameFx,
        from: Micros,
        to: Micros,
        now: Instant,
    ) -> Result<(), String> {
        if from == to {
            return Ok(());
        }
        let mut moved = from;
        self.edit(now, "Move keyframe", |e| {
            moved = match effect {
                FrameFx::Scale => e.timeline.move_scale_key(clip, from, to),
                FrameFx::Move => e.timeline.move_move_key(clip, from, to),
            }
            .map_err(err)?;
            Ok(())
        })?;
        self.popups.key = Some(moved);
        Ok(())
    }

    /// The selected keyframe of the open popup, when it still exists.
    pub fn selected_key(&self, target: FrameTarget, effect: FrameFx) -> Option<Micros> {
        let k = self.popups.key?;
        let tl = &self.project.timeline;
        let found = match effect {
            FrameFx::Scale => tl.scale_keys(target).iter().any(|x| x.t == k),
            FrameFx::Move => tl.move_keys(target).iter().any(|x| x.t == k),
        };
        found.then_some(k)
    }

    pub(super) fn add_key(
        &mut self,
        target: FrameTarget,
        effect: FrameFx,
        now: Instant,
    ) -> Result<(), String> {
        let at = self.key_at_playhead(target)?;
        let tl = &self.project.timeline;
        match effect {
            FrameFx::Scale => {
                let v = keyframes::at(tl.scale_keys(target), at).ok_or("no such effect")?;
                self.edit(now, "Add keyframe", |e| {
                    e.timeline.add_scale_key(target, at, v).map_err(err)
                })?;
            }
            FrameFx::Move => {
                let v = keyframes::at(tl.move_keys(target), at).ok_or("no such effect")?;
                self.edit(now, "Add keyframe", |e| {
                    e.timeline.add_move_key(target, at, v).map_err(err)
                })?;
            }
        }
        self.popups.key = Some(at);
        Ok(())
    }

    pub(super) fn remove_key(
        &mut self,
        target: FrameTarget,
        effect: FrameFx,
        now: Instant,
    ) -> Result<(), String> {
        let at = self
            .selected_key(target, effect)
            .ok_or_else(|| SELECT_A_KEY.to_string())?;
        self.edit(now, "Remove keyframe", |e| match effect {
            FrameFx::Scale => e.timeline.remove_scale_key(target, at).map_err(err),
            FrameFx::Move => e.timeline.remove_move_key(target, at).map_err(err),
        })?;
        self.popups.key = None;
        Ok(())
    }

    /// Chooses the easing of the segment after the selected keyframe (REQ-282).
    pub(super) fn set_key_ease(
        &mut self,
        target: FrameTarget,
        effect: FrameFx,
        ease: crate::keyframes::Ease,
        now: Instant,
    ) -> Result<(), String> {
        let at = self
            .selected_key(target, effect)
            .ok_or_else(|| SELECT_A_KEY.to_string())?;
        let scale = effect == FrameFx::Scale;
        if self.project.timeline.key_ease(target, scale, at) == ease {
            return Ok(());
        }
        self.edit(now, "Keyframe easing", |e| {
            e.timeline
                .set_key_ease(target, scale, at, ease)
                .map_err(err)
        })
    }

    pub(super) fn remove_frame_effect(
        &mut self,
        target: FrameTarget,
        effect: FrameFx,
        now: Instant,
    ) -> Result<(), String> {
        self.edit(now, "Remove effect", |e| match effect {
            FrameFx::Scale => e.timeline.remove_scale(target).map_err(err),
            FrameFx::Move => e.timeline.remove_move(target).map_err(err),
        })?;
        self.popups.close(Popup::Instance);
        self.frame_drag = None;
        Ok(())
    }

    /// Where a Scale or Move value goes: the selected keyframe, else the playhead.
    fn value_time(&self, d: &FrameDrag) -> Result<Micros, String> {
        let fx = match d.value {
            FrameValue::Scale(_) => FrameFx::Scale,
            FrameValue::Move(_) => FrameFx::Move,
        };
        match self.selected_key(d.target, fx) {
            Some(k) => Ok(k),
            None => self.key_at_playhead(d.target),
        }
    }

    pub(super) fn drag_picture(&mut self, d: Option<FrameDrag>, now: Instant) {
        self.frame_drag = d;
        self.refresh_still(now);
    }

    pub(super) fn set_picture(&mut self, d: FrameDrag, now: Instant) -> Result<(), String> {
        self.frame_drag = None;
        let at = self.value_time(&d)?;
        let label = if matches!(d.value, FrameValue::Scale(_)) {
            "Scale"
        } else {
            "Move"
        };
        self.edit(now, label, |e| match d.value {
            FrameValue::Scale(v) => e.timeline.set_scale_key(d.target, at, v).map_err(err),
            FrameValue::Move(v) => e.timeline.set_move_key(d.target, at, v).map_err(err),
        })?;
        // a key made by the change is the selected one
        let fx = if matches!(d.value, FrameValue::Scale(_)) {
            FrameFx::Scale
        } else {
            FrameFx::Move
        };
        if self.popups.key.is_none()
            && self
                .popups
                .instance
                .is_some_and(|i| i.frame_fx() == Some(fx))
        {
            self.popups.key = Some(at);
        }
        Ok(())
    }

    /// The project with the value being dragged applied (for the preview).
    pub(super) fn with_frame_drag(&self, q: &mut Project) {
        let Some(d) = self.frame_drag else { return };
        let Ok(at) = self.value_time(&d) else { return };
        let _ = match d.value {
            FrameValue::Scale(v) => q.timeline.set_scale_key(d.target, at, v),
            FrameValue::Move(v) => q.timeline.set_move_key(d.target, at, v),
        };
    }

    /// The picture's rectangle at the playhead as fractions of the frame, and the frame.
    pub fn picture_rect(&self) -> Option<(crate::frame::FractionRect, (u32, u32))> {
        let (i, start) = self.project.timeline.locate(self.playhead)?;
        self.picture_rect_of(i, self.playhead - start)
    }

    /// The rectangle of the picture of clip `index`, `local` into the clip, and the frame.
    fn picture_rect_of(
        &self,
        index: usize,
        local: Micros,
    ) -> Option<(crate::frame::FractionRect, (u32, u32))> {
        let tl = &self.project.timeline;
        let st = tl.frame_stack(index);
        if st.is_empty() && tl.frame_aspect.is_none() {
            return None;
        }
        let frame = self.output_size()?;
        let c = tl.clips.get(index)?;
        let m = self.project.media(c.media)?;
        let aspect = tl.frame_aspect.as_ref();
        let picture = if m.format.is_360() {
            crate::frame::camera_size(frame, aspect)
        } else {
            crate::frame::picture_size(frame, aspect, (m.format.width, m.format.height))
        };
        let p = st.at(local.clamp(Micros::ZERO, c.len()));
        Some((p.rect(frame, picture), frame))
    }

    /// The second set of picture handles (REQ-202): the copy that the whole-video Scale or
    /// Move popup has selected, when it belongs to another clip than the one under the
    /// playhead. The clip's own handles are solid, these are dashed; a drag changes only
    /// the copy its handle belongs to.
    pub fn whole_handle_clip(&self, effect: FrameFx) -> Option<ClipId> {
        let open = self.popups.is_open(Popup::Instance)
            && match self.popups.instance {
                Some(Instance::Scale(FrameTarget::Whole)) => effect == FrameFx::Scale,
                Some(Instance::Move(FrameTarget::Whole)) => effect == FrameFx::Move,
                _ => false,
            };
        if !open {
            return None;
        }
        let copy = self.selected_copy(effect)?;
        let tl = &self.project.timeline;
        let (i, _) = tl.locate(self.playhead)?;
        (tl.clips[i].id != copy).then_some(copy)
    }

    /// The rectangle of the copy of clip `copy` shown with the dashed handles: the same time
    /// into the clip as the playhead is into the clip under it.
    pub fn picture_rect_of_copy(
        &self,
        copy: ClipId,
    ) -> Option<(crate::frame::FractionRect, (u32, u32))> {
        let tl = &self.project.timeline;
        let (_, start) = tl.locate(self.playhead)?;
        self.picture_rect_of(tl.index_of(copy)?, self.playhead - start)
    }

    /// The Scale shown for clip `copy` at the same time into the clip as the playhead.
    pub fn scale_of_copy(&self, copy: ClipId) -> Option<Scale> {
        let t = self.copy_key_time(copy)?;
        keyframes::at(self.project.timeline.scale_keys(FrameTarget::Clip(copy)), t)
    }

    /// The Move shown for clip `copy` at the same time into the clip as the playhead.
    pub fn move_of_copy(&self, copy: ClipId) -> Option<Pos> {
        let t = self.copy_key_time(copy)?;
        keyframes::at(self.project.timeline.move_keys(FrameTarget::Clip(copy)), t)
    }

    fn copy_key_time(&self, copy: ClipId) -> Option<Micros> {
        let tl = &self.project.timeline;
        let (_, start) = tl.locate(self.playhead)?;
        let c = tl.get(copy)?;
        Some(c.src_in + (self.playhead - start).clamp(Micros::ZERO, c.len()))
    }
}

impl Instance {
    /// Scale or Move when this instance is the popup of one of them.
    pub fn frame_fx(self) -> Option<FrameFx> {
        match self {
            Instance::Scale(_) => Some(FrameFx::Scale),
            Instance::Move(_) => Some(FrameFx::Move),
            _ => None,
        }
    }
}
