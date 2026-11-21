// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the undo, playback, zoom, export and ffmpeg commands: the arms of [`AppState::dispatch`] for this area. A command
//! of another area is handed back unchanged ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_playback(&mut self, cmd: Command, now: Instant) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::Undo => {
                    let prev = self
                        .history
                        .undo(self.project.edit())
                        .ok_or(EditError::NothingToUndo.to_string())?;
                    self.replace_edit(prev, now);
                    Ok(())
                }
                Command::Redo => {
                    let next = self
                        .history
                        .redo(self.project.edit())
                        .ok_or(EditError::NothingToRedo.to_string())?;
                    self.replace_edit(next, now);
                    Ok(())
                }
                Command::PlayPause => {
                    self.stop_at = None;
                    if self.preview.is_playing() {
                        self.preview.pause(now);
                        self.playhead = self.preview.position(now);
                        self.refresh_still(now);
                        Ok(())
                    } else {
                        self.clear_overrides();
                        let shown = self.shown_project().into_owned();
                        self.preview.play(&shown, self.playhead, now)
                    }
                }
                Command::Seek(t) => {
                    self.seek(t, now);
                    Ok(())
                }
                Command::StepFrames(n) => {
                    let f = self.fps();
                    let t = f.frame_time(f.frame_index(self.playhead) + n);
                    self.seek(t, now);
                    Ok(())
                }
                Command::SetZoom(z) => {
                    self.zoom = z.max(0.0);
                    Ok(())
                }
                Command::PickExport => {
                    self.open_export_dialog();
                    Ok(())
                }
                Command::SetExportChoice(c) => {
                    if matches!(self.modal, Some(Modal::Export(_))) {
                        self.modal = Some(Modal::Export(c));
                    }
                    Ok(())
                }
                Command::PickExportFolder => {
                    if let Some(Modal::Export(c)) = &self.modal {
                        let mut c = c.clone();
                        if let Some(f) = self.dialogs.pick_folder(&c.folder) {
                            c.folder = f;
                            self.modal = Some(Modal::Export(c));
                        }
                    }
                    Ok(())
                }
                Command::StartExport(c) => self.start_export_choice(*c, now),
                Command::ConfirmOverwrite => match self.modal.take() {
                    Some(Modal::ConfirmOverwrite(c)) => self.run_export_choice(*c, now),
                    other => {
                        self.modal = other;
                        Ok(())
                    }
                },
                Command::Export(path) => self.start_export(path, render::ExportMode::Automatic),
                Command::ExportAs(path, mode) => self.start_export(path, mode),
                Command::CancelExport => {
                    if let Some(h) = &self.export {
                        h.cancel();
                    }
                    Ok(())
                }
                Command::PickFfmpeg => match self.dialogs.pick_ffmpeg() {
                    Some(p) => self.dispatch_inner(Command::SetFfmpeg(p), now),
                    None => Ok(()),
                },
                Command::SetFfmpeg(p) => {
                    if !p.is_absolute() {
                        return Err("choose ffmpeg with its full path".into());
                    }
                    let probe = p.with_file_name(ffmpeg::exe_name("ffprobe"));
                    self.settings.ffmpeg = Some(p);
                    self.settings.ffprobe = probe.is_file().then_some(probe);
                    if let Some(sp) = &self.settings_path {
                        settings::save(&self.settings, sp)?;
                    }
                    self.dispatch_inner(Command::RetryFfmpeg, now)
                }
                Command::RetryFfmpeg => {
                    self.start_ffmpeg_check();
                    Ok(())
                }
                Command::ShowAbout => {
                    self.modal = Some(Modal::About);
                    Ok(())
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
