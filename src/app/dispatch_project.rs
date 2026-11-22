// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the project, media, timeline and music commands: the arms of [`AppState::dispatch`] for this area. A command
//! of another area is handed back unchanged ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_project(
        &mut self,
        cmd: Command,
        now: Instant,
        label: &'static str,
    ) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::NewProject => self.guarded(PendingAction::New, now),
                Command::PickOpenProject => self.guarded(PendingAction::Open(None), now),
                // a video given to the editor (by "Open in editor" of the player) is added to
                // the project, which is a new unsaved one at a start (REQ-250)
                Command::OpenProject(path) if !project::is_project_path(&path) => {
                    self.dispatch_inner(Command::Import(vec![path]), now)
                }
                Command::OpenProject(path) => self.guarded(PendingAction::Open(Some(path)), now),
                Command::OpenInPlayer => {
                    let from_clip = self.target_clip().and_then(|c| {
                        self.project
                            .timeline
                            .clips
                            .iter()
                            .find(|x| x.id == c)
                            .map(|x| x.media)
                    });
                    let path = self
                        .selected_media
                        .or(from_clip)
                        .and_then(|id| self.project.media(id))
                        .map(|m| m.path.clone())
                        .ok_or("Select a video in Media first.")?;
                    crate::siblings::launch(crate::siblings::PLAYER, &path)
                }
                Command::SaveProject => self.save_interactive().map(|_| ()),
                Command::PickSaveProjectAs => match self.dialogs.pick_project_save() {
                    Some(p) => self.save_as(p),
                    None => Ok(()),
                },
                Command::SaveProjectAs(p) => self.save_as(p),
                Command::RequestClose => self.guarded(PendingAction::Close, now),
                Command::Quit => self.guarded(PendingAction::Quit, now),
                Command::ResolveUnsaved(choice) => {
                    let Some(Modal::Unsaved(action)) = self.modal.take() else {
                        return Ok(());
                    };
                    match choice {
                        UnsavedChoice::Cancel => Ok(()),
                        UnsavedChoice::Discard => {
                            self.clear_autosave(now);
                            self.continue_action(action, now)
                        }
                        UnsavedChoice::Save => {
                            if self.save_interactive()? {
                                self.continue_action(action, now)
                            } else {
                                Ok(())
                            }
                        }
                    }
                }
                Command::RecoverAutosave => self.recover_autosave(now),
                Command::DiscardAutosave => {
                    self.discard_autosave();
                    Ok(())
                }
                Command::CancelRecovery => {
                    self.cancel_recovery();
                    Ok(())
                }
                Command::PickImport => {
                    let files = self.dialogs.pick_media();
                    if files.is_empty() {
                        return Ok(());
                    }
                    self.dispatch_inner(Command::Import(files), now)
                }
                Command::Import(files) => self.import(files),
                Command::RemoveMedia(id) => self.remove_media(id),
                Command::SelectMedia(id) => {
                    self.selected_media = id;
                    Ok(())
                }
                Command::Append(id) => {
                    let m = self
                        .usable(id)
                        .cloned()
                        .ok_or("select a file that is ready in the media list first")?;
                    self.edit(now, label, |e| {
                        e.timeline.append(&m).map(|_| ()).map_err(err)
                    })
                }
                Command::SelectClip(id) => {
                    self.selected_clip = id;
                    Ok(())
                }
                Command::Split => {
                    let min = self.frame_len();
                    let at = self.playhead;
                    self.edit(now, label, |e| {
                        let from = e.timeline.locate(at).map(|(i, _)| e.timeline.clips[i].id);
                        let id = e.timeline.split(at, min).map_err(err)?;
                        // both halves keep the clip's own camera path (REQ-268)
                        if let Some(from) = from {
                            e.camera.copy_clip(from.0, id.0);
                        }
                        Ok(())
                    })
                }
                Command::Copy => {
                    let id = self
                        .target_clip()
                        .ok_or(EditError::NothingSelected.to_string())?;
                    self.clipboard = self.project.timeline.get(id).copied();
                    self.clipboard_frames = self.project.timeline.frames_of(id).cloned();
                    self.clipboard_lut = self
                        .project
                        .timeline
                        .lut_of(FrameTarget::Clip(id))
                        .map(str::to_string);
                    self.clipboard_camera = self.project.camera.own_path(id.0).cloned();
                    self.status = "Clip copied".into();
                    self.clipboard_text = Some(crate::app::CLIPBOARD_MARKER.into());
                    Ok(())
                }
                Command::Cut | Command::Delete => {
                    let id = self
                        .target_clip()
                        .ok_or(EditError::NothingSelected.to_string())?;
                    let clip = self.project.timeline.get(id).copied();
                    let frames = self.project.timeline.frames_of(id).cloned();
                    let lut = self
                        .project
                        .timeline
                        .lut_of(FrameTarget::Clip(id))
                        .map(str::to_string);
                    let camera = self.project.camera.own_path(id.0).cloned();
                    self.edit(now, label, |e| {
                        e.timeline.remove(id).map(|_| ()).map_err(err)
                    })?;
                    if cmd == Command::Cut {
                        self.clipboard_text = Some(crate::app::CLIPBOARD_MARKER.into());
                        self.clipboard = clip;
                        self.clipboard_frames = frames;
                        self.clipboard_lut = lut;
                        self.clipboard_camera = camera;
                    }
                    self.selected_clip = None;
                    Ok(())
                }
                Command::Paste => {
                    let clip = self
                        .clipboard
                        .ok_or(EditError::EmptyClipboard.to_string())?;
                    if self.project.media(clip.media).is_none() {
                        return Err("the copied clip's source is no longer in the project".into());
                    }
                    let min = self.frame_len();
                    let at = self.playhead;
                    let mut new_id = None;
                    let frames = self.clipboard_frames.clone();
                    let lut = self.clipboard_lut.clone();
                    let camera = self.clipboard_camera.clone();
                    self.edit(now, label, |e| {
                        let id = e.timeline.insert_at(&clip, at, min).map_err(err)?;
                        if let Some(own) = camera {
                            e.camera.put_own(id.0, own);
                        }
                        if let Some(p) = lut {
                            e.timeline
                                .set_lut(FrameTarget::Clip(id), Some(p))
                                .map_err(err)?;
                        }
                        if let Some(f) = frames {
                            e.timeline
                                .put_frames(crate::timeline::ClipFrames { clip: id, ..f });
                        }
                        new_id = Some(id);
                        Ok(())
                    })?;
                    self.selected_clip = new_id;
                    Ok(())
                }
                Command::Merge => {
                    let id = self
                        .target_clip()
                        .ok_or(EditError::NothingSelected.to_string())?;
                    self.edit(now, label, |e| e.timeline.merge(id).map_err(err))
                }
                Command::SetTrim {
                    clip,
                    src_in,
                    src_out,
                } => {
                    let c = self
                        .project
                        .timeline
                        .get(clip)
                        .ok_or(EditError::NoSuchClip.to_string())?;
                    let dur = self.duration_of(c.media)?;
                    let min = self.frame_len();
                    self.edit(now, label, |e| {
                        e.timeline
                            .trim(clip, src_in, src_out, dur, min)
                            .map_err(err)
                    })
                }
                Command::TrimEdge { clip, edge, delta } => {
                    let c = *self
                        .project
                        .timeline
                        .get(clip)
                        .ok_or(EditError::NoSuchClip.to_string())?;
                    let dur = self.duration_of(c.media)?;
                    let frame = self.frame_len();
                    let (src_in, src_out) = self
                        .project
                        .timeline
                        .trimmed_by_edge(clip, edge, delta, dur, frame)
                        .ok_or(EditError::NoSuchClip.to_string())?;
                    if (src_in, src_out) == (c.src_in, c.src_out) {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.timeline
                            .trim(clip, src_in, src_out, dur, frame)
                            .map_err(err)
                    })
                }
                Command::MoveFade { from, to } => {
                    self.edit(now, label, |e| e.timeline.move_fade(from, to).map_err(err))
                }
                Command::StepMarker { forward } => self.step_marker(forward, now),
                Command::MoveClip { clip, to } => {
                    self.edit(now, label, |e| e.timeline.move_to(clip, to).map_err(err))
                }
                Command::MoveLeft | Command::MoveRight => {
                    let id = self
                        .target_clip()
                        .ok_or(EditError::NothingSelected.to_string())?;
                    let i = self
                        .project
                        .timeline
                        .index_of(id)
                        .ok_or(EditError::NoSuchClip.to_string())?;
                    let to = if cmd == Command::MoveLeft {
                        i.checked_sub(1).ok_or("already the first clip")?
                    } else {
                        i + 1
                    };
                    self.selected_clip = Some(id);
                    self.edit(now, label, |e| e.timeline.move_to(id, to).map_err(err))
                }
                Command::ToggleFade(b) => {
                    let label = fade_label(self.project.timeline.has_fade(b));
                    self.edit(now, label, |e| {
                        e.timeline.toggle_fade(b).map(|_| ()).map_err(err)
                    })
                }
                Command::ToggleFadeAtPlayhead => {
                    let (b, _) = self
                        .project
                        .timeline
                        .nearest_boundary(self.playhead)
                        .ok_or(EditError::NoSuchBoundary.to_string())?;
                    let label = fade_label(self.project.timeline.has_fade(b));
                    self.edit(now, label, |e| {
                        e.timeline.toggle_fade(b).map(|_| ()).map_err(err)
                    })
                }
                Command::PlaceMusic(id) => {
                    let m = self
                        .usable(id)
                        .cloned()
                        .ok_or("select an audio file that is ready in the media list first")?;
                    let (at, min) = (self.playhead, self.frame_len());
                    let mut placed = None;
                    self.edit(now, label, |e| {
                        placed = Some(e.music.place(&m, at, min).map_err(err)?);
                        Ok(())
                    })?;
                    self.selected_music = placed;
                    Ok(())
                }
                Command::SelectMusic(id) => {
                    self.selected_music = id;
                    Ok(())
                }
                Command::SetMusicEffects {
                    clip,
                    volume,
                    muted,
                    fade_in,
                    fade_out,
                    clean_audio,
                } => self.edit(now, label, |e| {
                    e.music
                        .set_effects(clip, volume, muted, fade_in, fade_out, clean_audio)
                        .map_err(err)
                }),
                Command::MoveMusic { clip, start } => {
                    self.edit(now, label, |e| e.music.move_to(clip, start).map_err(err))
                }
                Command::TrimMusic {
                    clip,
                    src_in,
                    src_out,
                } => {
                    let c = self.project.music.get(clip).ok_or("no such music clip")?;
                    let dur = self.duration_of(c.media)?;
                    let min = self.frame_len();
                    self.edit(now, label, |e| {
                        e.music.trim(clip, src_in, src_out, dur, min).map_err(err)
                    })
                }
                Command::SplitMusic => {
                    let (at, min) = (self.playhead, self.frame_len());
                    self.edit(now, label, |e| {
                        e.music.split(at, min).map(|_| ()).map_err(err)
                    })
                }
                Command::DeleteMusic => {
                    let id = self.selected_music.ok_or("select a music clip first")?;
                    self.edit(now, label, |e| e.music.remove(id).map(|_| ()).map_err(err))?;
                    self.selected_music = None;
                    Ok(())
                }
                Command::SetMix(mut mix) => {
                    mix.video_volume = mix.video_volume.min(100);
                    mix.music_volume = mix.music_volume.min(100);
                    let shown = self.volume_mix();
                    if mix == shown {
                        return Ok(());
                    }
                    // the video sound is the level of each clip (REQ-147); the project keeps
                    // only the music level
                    let video = (mix.video_volume, mix.video_muted)
                        != (shown.video_volume, shown.video_muted);
                    self.edit(now, label, |e| {
                        if e.timeline.clips.is_empty() {
                            e.mix = mix;
                            return Ok(());
                        }
                        e.mix.music_volume = mix.music_volume;
                        e.mix.music_muted = mix.music_muted;
                        if video {
                            let ids: Vec<ClipId> = e.timeline.clips.iter().map(|c| c.id).collect();
                            for id in ids {
                                e.timeline
                                    .set_clip_volume(id, mix.video_volume, mix.video_muted)
                                    .map_err(err)?;
                            }
                        }
                        Ok(())
                    })
                }
                Command::DropFiles(paths) => {
                    if self.modal.is_some() || self.notice.is_some() {
                        self.status = "Close the open dialog before dropping files".into();
                        return Ok(());
                    }
                    match drop_plan(paths) {
                        DropPlan::Nothing => Ok(()),
                        DropPlan::Import(files) => self.import(files),
                        DropPlan::Open(p, ignored) => {
                            let r = self.guarded(PendingAction::Open(Some(p)), now);
                            if ignored > 0 && r.is_ok() {
                                self.status = format!(
                                    "Only the first dropped project is opened; {ignored} other \
                                     file(s) ignored"
                                );
                            }
                            r
                        }
                    }
                }
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
