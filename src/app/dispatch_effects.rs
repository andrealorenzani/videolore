// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the effect commands (apply, frame effects and their keys, LUT, volume, speed,
//! clean audio, fades at the edges, overlaps, the Applied effects order): the arms of
//! [`AppState::dispatch`] for this area. A command of another area is handed back unchanged
//! ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_effects(
        &mut self,
        cmd: Command,
        now: Instant,
        _label: &'static str,
    ) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::ApplyEffect { effect, target } => self.apply_effect(effect, target, now),
                Command::OpenFrameEffect(effect) => {
                    self.open_frame_effect(effect);
                    Ok(())
                }
                Command::MoveRow { clip, row, mv } => self.move_row(clip, row, mv, now),
                Command::PickLut(target) => self.pick_lut(target, now),
                Command::SetLut { target, path } => self.set_lut_file(target, &path, now),
                Command::RemoveLut(target) => self.remove_lut(target, now),
                Command::SetAspect(a) => self.set_aspect(Some(a), now),
                Command::RemoveAspect => self.set_aspect(None, now),
                Command::AddKey { target, effect } => self.add_key(target, effect, now),
                Command::RemoveKey { target, effect } => self.remove_key(target, effect, now),
                Command::SetKeyEase {
                    target,
                    effect,
                    ease,
                } => self.set_key_ease(target, effect, ease, now),
                Command::RemoveFrameEffect { target, effect } => {
                    self.remove_frame_effect(target, effect, now)
                }
                Command::SelectKey(k) => {
                    self.popups.key = k;
                    Ok(())
                }
                Command::DragPicture(d) => {
                    self.drag_picture(d, now);
                    Ok(())
                }
                Command::SetPicture(d) => self.set_picture(d, now),
                Command::SetClipVolume {
                    clip,
                    volume,
                    muted,
                } => self.set_clip_volume(clip, volume, muted, now),
                Command::SetSpeedAll { speed } => self.set_speed_all(speed, now),
                Command::RemoveVolume { clip } => self.remove_volume(clip, now),
                Command::SetVolumeKeyEase { clip, at, ease } => {
                    self.edit(now, "Volume keyframe ease", |e| {
                        e.timeline.set_volume_key_ease(clip, at, ease).map_err(err)
                    })
                }
                Command::AddOverlap(path) => self.add_overlap(path, now),
                Command::SetOverlapBox {
                    id,
                    place,
                    width,
                    height,
                    reshaped,
                } => self.set_overlap_box(id, place, width, height, reshaped, now),
                Command::SetOverlapTimes { id, from, len } => {
                    self.set_overlap_times(id, from, len, now)
                }
                Command::RemoveOverlap(id) => self.remove_overlap(id, now),
                Command::SelectOverlap(id) => {
                    self.selected_overlap = id;
                    Ok(())
                }
                Command::ConfirmRemoval => self.confirm_removal(now),
                Command::CancelRemoval => self.cancel_removal(),
                Command::SetCleanAudio { target, on } => self.set_clean_audio(target, on, now),
                Command::SetSpeechEnhance { target, on } => {
                    self.set_speech_enhance(target, on, now)
                }
                Command::SetCleanStrength { target, percent } => {
                    self.set_clean_strength(target, percent, now)
                }
                Command::SetEdgeFade { edge, on } => self.set_edge_fade(edge, on, now),
                Command::CopyKeys { clip, effect } => self.copy_keys(clip, effect),
                Command::PasteKeys { clip, effect } => self.paste_keys(clip, effect, now),
                Command::MoveKey {
                    clip,
                    effect,
                    from,
                    to,
                } => self.move_key(clip, effect, from, to, now),
                other => {
                    pass = Some(other);
                    Ok(())
                }
            }
        };
        let r = run();
        match pass {
            Some(c) => Routed::Pass(c),
            None => Routed::Done(r),
        }
    }
}
