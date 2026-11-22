// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Change aspect, Scale and Move (1.5.0, REQ-124..130): the commands, the popups' state and
//! the values the preview handles start from. The maths are in [`crate::frame`] and
//! [`crate::keyframes`]; every edit here is one undo step.

use super::*;
use crate::frame::FrameAspect;
use crate::keyframes;

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
                // dropped on a clip: that clip gets its own aspect (REQ-288, REQ-286)
                if let Some(id) = clip {
                    if tl.own_aspect(id).is_none() {
                        self.edit(now, effect.undo_name(), |e| {
                            e.timeline
                                .set_clip_aspect(id, Some(FrameAspect::default()))
                                .map_err(err)
                        })?;
                    }
                    self.status = "This clip has its own aspect: change it in Clip details".into();
                    return Ok(());
                }
                if tl.frame_aspect.is_none() {
                    self.edit(now, effect.undo_name(), |e| {
                        e.timeline
                            .set_frame_aspect(Some(FrameAspect::default()))
                            .map_err(err)
                    })?;
                }
                self.show_instance(Instance::Aspect);
            }
            // a whole-video Scale or Move is a copy on each clip that lacks one (REQ-148);
            // the popup that opens is the one of the clip under the playhead
            EffectKind::Scale | EffectKind::Move => {
                let scale = effect == EffectKind::Scale;
                let lacks = |id: ClipId| {
                    let t = FrameTarget::Clip(id);
                    if scale {
                        tl.scale_keys(t).is_empty()
                    } else {
                        tl.move_keys(t).is_empty()
                    }
                };
                let needed = match clip {
                    Some(id) => lacks(id),
                    None => tl.clips.iter().any(|c| lacks(c.id)),
                };
                if needed {
                    self.edit(now, effect.undo_name(), |e| {
                        let added = if scale {
                            e.timeline.add_scale(target)
                        } else {
                            e.timeline.add_move(target)
                        };
                        added.map(|_| ()).map_err(err)
                    })?;
                }
                let shown = clip.or_else(|| self.clip_at_playhead_or_first());
                if let Some(id) = shown {
                    let t = FrameTarget::Clip(id);
                    self.show_instance(if scale {
                        Instance::Scale(t)
                    } else {
                        Instance::Move(t)
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// The clip under the playhead, else the first clip.
    pub(super) fn clip_at_playhead_or_first(&self) -> Option<ClipId> {
        let tl = &self.project.timeline;
        tl.locate(self.playhead)
            .map(|(i, _)| tl.clips[i].id)
            .or_else(|| tl.clips.first().map(|c| c.id))
    }

    /// Opens the popup of a picture effect without applying it (the effect's row): Change
    /// aspect's, or the popup of the selected clip's own Scale, Move or Apply LUT (there is
    /// no whole-video popup, REQ-200).
    pub(super) fn open_frame_effect(&mut self, effect: EffectKind) {
        if effect == EffectKind::ChangeAspect {
            self.show_instance(Instance::Aspect);
            return;
        }
        let Some(id) = self.target_clip() else { return };
        let t = FrameTarget::Clip(id);
        let tl = &self.project.timeline;
        match effect {
            EffectKind::Scale if !tl.scale_keys(t).is_empty() => {
                self.show_instance(Instance::Scale(t));
            }
            EffectKind::Move if !tl.move_keys(t).is_empty() => {
                self.show_instance(Instance::Move(t));
            }
            EffectKind::ApplyLut => self.show_instance(Instance::Lut(t)),
            EffectKind::Scale | EffectKind::Move => {
                self.status = format!(
                    "This clip has no {}: drag it onto the clip, or double-click it to give every clip one",
                    effect.name()
                );
            }
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
        | Instance::Lut(FrameTarget::Clip(c))
        | Instance::Speed(FrameTarget::Clip(c)) = i
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
