// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Which commands can run now and why one cannot (greyed-out controls).

use super::*;

impl AppState {
    /// Whether a command can run now (used to disable buttons).
    pub fn is_enabled(&self, cmd: &Command) -> bool {
        let tl = &self.project.timeline;
        let has_ff = self.ffmpeg.is_ok();
        match cmd {
            Command::Undo => self.history.can_undo(),
            Command::Redo => self.history.can_redo(),
            Command::Split => tl.locate(self.playhead).is_some(),
            Command::Cut | Command::Copy | Command::Delete => self.target_clip().is_some(),
            Command::Paste => self.clipboard.is_some(),
            Command::Merge => self.target_clip().is_some_and(|c| tl.can_merge(c)),
            Command::ApplyEffect { effect, target } => {
                let clip_ok = match target {
                    Target::Selected => self.target_clip().is_some(),
                    Target::Clip(c) => tl.get(*c).is_some(),
                    Target::WholeVideo => !tl.clips.is_empty(),
                };
                let aspect_ok = true;
                clip_ok
                    && aspect_ok
                    && (*effect != EffectKind::Subtitles || (has_ff && !self.is_generating()))
            }
            Command::OpenFrameEffect(_) | Command::SetAspect(_) | Command::RemoveAspect => {
                !tl.clips.is_empty()
            }
            Command::PickLut(t) | Command::SetLut { target: t, .. } => match t {
                FrameTarget::Whole => !tl.clips.is_empty(),
                FrameTarget::Clip(c) => tl.get(*c).is_some(),
            },
            Command::RemoveLut(t) => tl.lut_of(*t).is_some(),
            Command::MoveRow { clip, row, mv } => self.row_move_reason(*clip, *row, *mv).is_none(),
            Command::AddKey { target, .. } | Command::RemoveFrameEffect { target, .. } => {
                match target {
                    FrameTarget::Whole => !tl.clips.is_empty(),
                    FrameTarget::Clip(c) => tl.get(*c).is_some(),
                }
            }
            Command::RemoveKey { target, effect } | Command::SetKeyEase { target, effect, .. } => {
                self.selected_key(*target, *effect).is_some()
            }
            Command::SetMusicEffects { clip, .. } => self.project.music.get(*clip).is_some(),
            Command::SetClipVolume { clip, .. }
            | Command::SetClipAudioTrack { clip, .. }
            | Command::SetVolumeKey { clip, .. }
            | Command::SetVolumeKeyEase { clip, .. }
            | Command::RemoveVolumeKey { clip, .. }
            | Command::SetSpeed { clip, .. } => tl.get(*clip).is_some(),
            Command::SetCleanAudio { target, .. } | Command::SetCleanStrength { target, .. } => {
                match target {
                    Target::Selected => self.target_clip().is_some(),
                    Target::Clip(c) => tl.get(*c).is_some(),
                    Target::WholeVideo => !tl.clips.is_empty(),
                }
            }
            Command::SetEdgeFade { .. } => !tl.clips.is_empty(),
            Command::ResetPosition => self.selection_is_changed(),
            Command::ResetObjects => self.selection_is_changed() || self.reset_applies_to_camera(),
            Command::MoveInside => {
                let (lines, texts) = self.inside_plan();
                !lines.is_empty() || !texts.is_empty()
            }
            Command::ReAnchor => self.re_anchor_plan().is_some(),
            Command::MoveSelection { .. } => true,
            Command::AlignSelected(axis) => {
                let (lines, texts) = self.align_plan(*axis);
                !lines.is_empty() || !texts.is_empty()
            }
            Command::SetPov
            | Command::DragCamera { .. }
            | Command::RollCamera { .. }
            | Command::TurnCamera { .. }
            | Command::ZoomCamera(_) => self.sphere_at_playhead().is_some(),
            Command::PickExportSrt | Command::ExportSrt(_) => self.has_shown_subtitles(),
            Command::DeleteSubtitles(ids) | Command::ShiftLines { ids, .. } => !ids.is_empty(),
            Command::TrimEdge { clip, .. } => tl.get(*clip).is_some(),
            Command::MoveFade { from, .. } => tl.fades.contains(from),
            Command::StepMarker { .. } => !tl.clips.is_empty(),
            Command::PasteKeys { clip, .. } => {
                self.clipboard_keys.is_some() && tl.get(*clip).is_some()
            }
            Command::CopyKeys { clip, .. } | Command::MoveKey { clip, .. } => {
                tl.get(*clip).is_some()
            }
            Command::MoveLeft => self
                .target_clip()
                .and_then(|c| tl.index_of(c))
                .is_some_and(|i| i > 0),
            Command::MoveRight => self
                .target_clip()
                .and_then(|c| tl.index_of(c))
                .is_some_and(|i| i + 1 < tl.clips.len()),
            Command::ToggleFadeAtPlayhead => tl.clips.len() > 1,
            Command::Append(id) => self.usable(*id).is_some_and(MediaItem::has_video),
            Command::PlaceMusic(id) => self
                .usable(*id)
                .is_some_and(|m| !m.has_video() && m.format.has_audio()),
            Command::RemoveMedia(id) => {
                self.project.media(*id).is_some() && !self.media_in_use(*id)
            }
            Command::SplitMusic => self.project.music.at(self.playhead).is_some(),
            Command::DeleteMusic => self
                .selected_music
                .is_some_and(|id| self.project.music.get(id).is_some()),
            Command::PlayPause => has_ff && !tl.clips.is_empty(),
            Command::PickExport | Command::Export(_) | Command::ExportAs(..) => {
                has_ff && !tl.clips.is_empty() && !self.is_exporting()
            }
            Command::CancelExport => self.is_exporting(),
            Command::PickImport | Command::Import(_) => has_ff,
            Command::OpenInPlayer => self.selected_media.is_some() || self.target_clip().is_some(),
            Command::GenerateSubtitles => {
                has_ff && stt::available() && !tl.clips.is_empty() && !self.is_generating()
            }
            Command::CancelSubtitles => self.is_generating(),
            Command::PickImportSubtitles(_) | Command::OpenImportSubtitles { .. } => {
                !tl.clips.is_empty() && !self.is_generating()
            }
            Command::AddSubtitle => tl.locate(self.playhead).is_some(),
            Command::AddText(_) | Command::AddPicture(_) | Command::OpenNewText => {
                !tl.clips.is_empty() && self.playhead < tl.duration()
            }
            Command::AddOverlap(_) => tl.locate(self.playhead).is_some(),
            Command::SetOverlapBox { id, .. }
            | Command::SetOverlapTimes { id, .. }
            | Command::RemoveOverlap(id) => tl.find_overlap(*id).is_some(),
            Command::ConfirmRemoval | Command::CancelRemoval => self.pending_removal.is_some(),
            Command::SetSpeedAll { .. } => !tl.clips.is_empty(),
            Command::RemoveVolume { clip } => !tl.volume_keys(*clip).is_empty(),
            Command::DeleteTexts(ids) => {
                !ids.is_empty() && ids.iter().all(|i| self.project.texts.get(*i).is_some())
            }
            Command::ClearProxyCache => self.proxy_cache.is_some(),
            Command::RemoveSpeechFiles | Command::ConfirmRemoveSpeechFiles => {
                self.model_store.is_some() && !self.is_generating()
            }
            Command::SetSubtitleQuality(q) => {
                settings::availability(&self.fingerprint())[q.index()].is_none()
            }
            Command::RemoveDesktopIntegration => self.desktop_integrated,
            Command::OpenInstance(i) => self.instance_start(*i).is_some(),
            Command::SetFade { left, right, .. } => self.boundary_of(*left, *right).is_some(),
            Command::RemoveAllFades => tl.has_any_fade(),
            Command::PlayLine(id) => {
                has_ff
                    && self
                        .project
                        .subtitles
                        .project(tl)
                        .iter()
                        .any(|l| l.line == *id)
            }
            Command::ClearRecent => !self.settings.recent.is_empty(),
            _ => true,
        }
    }

    /// Why a command cannot run now (shown on greyed-out controls); `None` when it can.
    pub fn disabled_reason(&self, cmd: &Command) -> Option<&'static str> {
        if self.is_enabled(cmd) {
            return None;
        }
        let tl = &self.project.timeline;
        let no_ffmpeg = self.ffmpeg.is_err();
        Some(match cmd {
            Command::Undo => "Nothing to undo",
            Command::Redo => "Nothing to redo",
            Command::MoveRow { clip, row, mv } => self
                .row_move_reason(*clip, *row, *mv)
                .unwrap_or("Not available right now"),
            Command::OpenInPlayer => "Select a video in Media or a clip first",
            Command::Split | Command::AddSubtitle => "Move the playhead over a clip first",
            Command::AddText(_) | Command::AddPicture(_) | Command::OpenNewText => {
                texts::PLAYHEAD_IN_VIDEO
            }
            Command::AddOverlap(_) => overlap::NO_CLIP_UNDER_PLAYHEAD,
            Command::DeleteTexts(_) => texts::SELECT_A_TEXT,
            Command::Cut | Command::Copy | Command::Delete => "Select a clip first",
            Command::Merge if self.target_clip().is_none() => "Select a clip first",
            Command::Merge
                if self
                    .target_clip()
                    .and_then(|c| tl.merge_block_reason(c))
                    .is_some() =>
            {
                self.target_clip()
                    .and_then(|c| tl.merge_block_reason(c))
                    .unwrap_or(crate::timeline::DIFFERENT_VOLUMES)
            }
            Command::Merge => "Only two parts of the same clip, next to each other, can be merged",
            Command::MoveLeft | Command::MoveRight if self.target_clip().is_none() => {
                "Select a clip first"
            }
            Command::MoveLeft => "Already the first clip",
            Command::MoveRight => "Already the last clip",
            Command::Paste => "Copy or cut a clip first",
            Command::ToggleFadeAtPlayhead => "Fades need at least two clips",
            Command::RemoveAllFades => "There are no fades",
            Command::Append(_) => "Select a video that is ready in the media list first",
            Command::PlaceMusic(_) => "Select an audio file that is ready in the media list first",
            Command::SplitMusic => "Move the playhead over a music clip first",
            Command::DeleteMusic => "Select a music clip first",
            Command::CancelExport => "No export is running",
            Command::CancelSubtitles => "Subtitles are not being generated",
            Command::ClearRecent => "The list is empty",
            Command::PickExportSrt | Command::ExportSrt(_) => effects::NO_SUBTITLES,
            Command::ResetPosition if self.selected_lines().is_empty() => effects::SELECT_A_LINE,
            Command::ResetPosition => sphere::ALREADY_ORIGINAL,
            Command::ResetObjects
                if self.selected_lines().is_empty() && self.selected_texts().is_empty() =>
            {
                if self.sphere_at_playhead().is_some() {
                    sphere::CAMERA_AT_FRONT
                } else {
                    sphere::SELECT_ON_PICTURE
                }
            }
            Command::ResetObjects => sphere::ALREADY_ORIGINAL,
            Command::AlignSelected(_)
                if self.selected_lines().is_empty() && self.selected_texts().is_empty() =>
            {
                sphere::SELECT_ON_PICTURE
            }
            Command::AlignSelected(_) => sphere::ALREADY_CENTRED,
            Command::MoveInside
                if self.selected_lines().is_empty() && self.selected_texts().is_empty() =>
            {
                sphere::SELECT_ON_PICTURE
            }
            Command::MoveInside => placing::ALL_INSIDE,
            Command::ReAnchor if self.sphere_at_playhead().is_none() => sphere::NOT_A_360,
            Command::ReAnchor => texts::SELECT_ANCHORED,
            Command::SetPov
            | Command::DragCamera { .. }
            | Command::RollCamera { .. }
            | Command::TurnCamera { .. }
            | Command::ZoomCamera(_) => sphere::NOT_A_360,
            Command::RemoveKey { .. } | Command::SetKeyEase { .. } => frame::SELECT_A_KEY,
            Command::RemoveLut(_) => "There is no LUT to remove",
            Command::PickLut(_) | Command::SetLut { .. } => "Add a clip to the timeline first",
            Command::ApplyEffect {
                target: Target::Selected,
                ..
            }
            | Command::SetCleanAudio {
                target: Target::Selected,
                ..
            } if self.target_clip().is_none() => effects::SELECT_A_CLIP,
            Command::ApplyEffect {
                effect: EffectKind::Subtitles,
                ..
            }
            | Command::GenerateSubtitles
            | Command::PickImportSubtitles(_)
            | Command::OpenImportSubtitles { .. }
                if self.is_generating() =>
            {
                subs_create::BEING_CREATED
            }
            _ if no_ffmpeg => "ffmpeg is not ready",
            Command::PickExport | Command::Export(_) | Command::ExportAs(..)
                if self.is_exporting() =>
            {
                "An export is already running"
            }
            Command::GenerateSubtitles if !stt::available() => {
                "Subtitle generation is not available in this build"
            }
            Command::PickExport
            | Command::Export(_)
            | Command::ExportAs(..)
            | Command::GenerateSubtitles
            | Command::PickImportSubtitles(_)
            | Command::OpenImportSubtitles { .. }
            | Command::ApplyEffect { .. }
            | Command::SetEdgeFade { .. }
            | Command::PlayPause
            | Command::PlayLine(_)
                if tl.clips.is_empty() =>
            {
                "Add clips to the timeline first"
            }
            _ => "Not available right now",
        })
    }
}
