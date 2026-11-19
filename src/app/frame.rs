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

    /// The effect whose handles the preview shows at the playhead: the clip's own, else the
    /// whole video's.
    pub fn frame_target_at_playhead(&self, effect: FrameFx) -> Option<FrameTarget> {
        let tl = &self.project.timeline;
        let has = |t: FrameTarget| match effect {
            FrameFx::Scale => !tl.scale_keys(t).is_empty(),
            FrameFx::Move => !tl.move_keys(t).is_empty(),
        };
        let (i, _) = tl.locate(self.playhead)?;
        let clip = FrameTarget::Clip(tl.clips[i].id);
        [clip, FrameTarget::Whole].into_iter().find(|t| has(*t))
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
        let tl = &self.project.timeline;
        let (i, start) = tl.locate(self.playhead)?;
        let st = tl.frame_stack(i);
        if st.is_empty() && tl.frame_aspect.is_none() {
            return None;
        }
        let frame = self.output_size()?;
        let c = &tl.clips[i];
        let m = self.project.media(c.media)?;
        let aspect = tl.frame_aspect.as_ref();
        let picture = if m.format.is_360() {
            crate::frame::camera_size(frame, aspect)
        } else {
            crate::frame::picture_size(frame, aspect, (m.format.width, m.format.height))
        };
        let p = st.at(self.playhead - start);
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
