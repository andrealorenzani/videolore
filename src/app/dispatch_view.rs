// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the popups, fades, layout and recent-project commands: the arms of [`AppState::dispatch`] for this area. A command
//! of another area is handed back unchanged ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_view(
        &mut self,
        cmd: Command,
        now: Instant,
        label: &'static str,
    ) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::CloseModal => {
                    if let Some(
                        Modal::Export(c) | Modal::ConfirmOverwrite(c) | Modal::MissingLut(c),
                    ) = self.modal.take()
                    {
                        self.export_memory = Some(*c);
                    }
                    self.modal = None;
                    self.pending_create = None;
                    self.pending_import = None;
                    Ok(())
                }
                Command::RemoveDesktopIntegration => {
                    crate::desktop::remove_integration().map_err(|e| e.to_string())?;
                    self.desktop_integrated = false;
                    self.status = "Removed from the application menu".into();
                    Ok(())
                }
                Command::DismissNotice => {
                    self.notice = None;
                    Ok(())
                }
                Command::OpenPopup(p) => {
                    if p == Popup::Settings {
                        self.settings_summary.clear();
                        self.refresh_speech_files_size();
                    }
                    if p == Popup::Subtitles {
                        self.refresh_import_offers();
                    }
                    self.popups.show(p);
                    Ok(())
                }
                Command::ClosePopup(p) => {
                    self.popups.close(p);
                    Ok(())
                }
                Command::OpenInstance(i) => {
                    let t = self
                        .instance_start(i)
                        .ok_or("that fade or subtitle line no longer exists")?;
                    if let Instance::Line(id, _) = i {
                        self.selection.select_only(ObjRef::Line(id));
                    }
                    self.popups.instance = Some(i);
                    self.popups.chooser = None;
                    self.popups.show(Popup::Instance);
                    self.seek(t, now);
                    Ok(())
                }
                Command::ChooseInstance(list) => {
                    self.popups.chooser = (!list.is_empty()).then_some(list);
                    Ok(())
                }
                Command::SetFade { left, right, on } => {
                    let b = self
                        .boundary_of(left, right)
                        .ok_or(EditError::NoSuchBoundary.to_string())?;
                    if self.project.timeline.has_fade(b) == on {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.timeline.toggle_fade(b).map(|_| ()).map_err(err)
                    })
                }
                Command::MissingLutFind => {
                    let Some(Modal::MissingLut(choice)) = self.modal.clone() else {
                        return Ok(());
                    };
                    // one file replaces every LUT that is not there: the project's paths change
                    let Some(path) = self.dialogs.pick_lut() else {
                        return Ok(());
                    };
                    let text = path
                        .to_str()
                        .filter(|t| crate::lut::path_ok(t))
                        .ok_or("The LUT file name cannot be used")?
                        .to_string();
                    crate::lut::cached(&text)
                        .map_err(|e| format!("{}: {e}", crate::lut::display_name(&text)))?;
                    self.edit(now, "Find LUT", |e| {
                        for l in &mut e.timeline.clip_luts {
                            if !Path::new(&l.path).exists() {
                                l.path = text.clone();
                            }
                        }
                        Ok(())
                    })?;
                    self.modal = None;
                    self.start_export_choice(*choice, now)
                }
                Command::MissingLutExportWithout => {
                    let Some(Modal::MissingLut(choice)) = self.modal.take() else {
                        return Ok(());
                    };
                    self.export_without_missing_luts = true;
                    self.start_export_choice(*choice, now)
                }
                Command::SetLutStrength { target, percent } => {
                    if self.project.timeline.lut_strength_of(target) == percent {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.timeline.set_lut_strength(target, percent).map_err(err)
                    })
                }
                Command::SetVolumeKey { clip, at, percent } => self.edit(now, label, |e| {
                    e.timeline.set_volume_key(clip, at, percent).map_err(err)
                }),
                Command::RemoveVolumeKey { clip, at } => self.edit(now, label, |e| {
                    e.timeline.remove_volume_key(clip, at).map_err(err)
                }),
                Command::SetCrop { clip, crop } => {
                    if self
                        .project
                        .timeline
                        .get(clip)
                        .is_some_and(|c| c.crop == crop)
                    {
                        return Ok(());
                    }
                    self.edit(now, label, |e| e.timeline.set_crop(clip, crop).map_err(err))
                }
                Command::SetFadeLength(len) => {
                    if self.project.timeline.fade_len() == len {
                        return Ok(());
                    }
                    self.edit(now, label, |e| e.timeline.set_fade_length(len).map_err(err))
                }
                Command::SetFadeSide { left, right, side } => {
                    let b = self
                        .boundary_of(left, right)
                        .ok_or(EditError::NoSuchBoundary.to_string())?;
                    if self.project.timeline.fade_side(b) == side {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.timeline.set_fade_side(b, side).map_err(err)
                    })
                }
                Command::RemoveAllFades => {
                    if !self.project.timeline.has_any_fade() {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.timeline.clear_fades();
                        Ok(())
                    })
                }
                Command::PlayLine(id) => {
                    let line = self
                        .project
                        .subtitles
                        .project(&self.project.timeline)
                        .into_iter()
                        .find(|l| l.line == id)
                        .ok_or("no such subtitle line")?;
                    if self.preview.is_playing() {
                        self.preview.pause(now);
                    }
                    self.selection.select_only(ObjRef::Line(id));
                    self.seek(line.start, now);
                    let shown = self.shown_project().into_owned();
                    self.preview.play(&shown, self.playhead, now)?;
                    self.stop_at = Some(line.end);
                    Ok(())
                }
                Command::ZoomBy(f) => {
                    if f.is_finite() && f > 0.0 {
                        self.zoom = (self.zoom * f).max(0.0);
                    }
                    Ok(())
                }
                Command::ZoomFit => {
                    self.zoom = 0.0;
                    Ok(())
                }
                Command::SetLayout(l) => {
                    if !l.is_valid() {
                        return Err("those section sizes are not valid".into());
                    }
                    self.settings.layout = Some(l);
                    self.save_settings()
                }
                Command::ResetLayout => {
                    self.settings.layout = None;
                    self.settings.interface.reset_layout();
                    self.save_settings()
                }
                Command::OpenRecent(p) => {
                    if !p.is_file() {
                        self.settings.recent.retain(|x| *x != p);
                        self.save_settings()?;
                        self.notice = Some(format!(
                            "The project \"{}\" could not be found ({}). It was removed from the \
                         recent projects.",
                            media::display_name(&p),
                            media::sanitize_for_display(&p.to_string_lossy())
                        ));
                        return Ok(());
                    }
                    self.guarded(PendingAction::Open(Some(p)), now)
                }
                Command::ClearRecent => {
                    self.settings.recent.clear();
                    self.save_settings()
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
