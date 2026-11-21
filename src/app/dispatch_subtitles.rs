// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the subtitle creation and line commands: the arms of [`AppState::dispatch`] for this area. A command
//! of another area is handed back unchanged ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_subtitles(
        &mut self,
        cmd: Command,
        now: Instant,
        label: &'static str,
    ) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::GenerateSubtitles => {
                    self.show_popup_for(Popup::Subtitles, Target::WholeVideo);
                    self.create_subtitles(None)
                }
                Command::PickImportSubtitles(clip) => match self.dialogs.pick_subtitles_open() {
                    Some(path) => self.open_import(path, clip),
                    None => Ok(()),
                },
                Command::OpenImportSubtitles { path, clip } => self.open_import(path, clip),
                Command::SetImportRetime(on) => {
                    if let Some(d) = &mut self.pending_import {
                        d.retime = on;
                    }
                    Ok(())
                }
                Command::ConfirmImportSubtitles => self.confirm_import(now),
                Command::ConfirmRegenerate => {
                    self.modal = None;
                    if self.stt.is_some() || self.model_install.is_some() {
                        return Err(subs_create::BEING_CREATED.into());
                    }
                    let mut r = self.pending_create.take().unwrap_or_default();
                    r.confirmed = true;
                    self.pending_create = Some(r);
                    self.continue_create()
                }
                Command::ModelConsent(choice) => {
                    let level = match self.modal.take() {
                        Some(Modal::ModelConsent(l)) => l,
                        other => {
                            self.modal = other;
                            return Ok(());
                        }
                    };
                    let store = self
                        .model_store
                        .clone()
                        .ok_or("no folder for speech recognition files")?;
                    let (level, source) = match choice {
                        ConsentChoice::Cancel => {
                            self.pending_create = None;
                            return Ok(());
                        }
                        ConsentChoice::UseWithoutMatching => {
                            if !self.consent_offers_spread() {
                                self.modal = Some(Modal::ModelConsent(level));
                                return Ok(());
                            }
                            // no download: the video's subtitles with words spread evenly
                            if let Some(r) = &mut self.pending_create {
                                r.spread = true;
                            }
                            return self.continue_create();
                        }
                        ConsentChoice::UseFastInstead => {
                            if !store.is_installed(SubtitleQuality::Fast) {
                                return Err("the Fast speech file is not installed".into());
                            }
                            // this generation only: the setting does not change
                            if let Some(r) = &mut self.pending_create {
                                r.level = Some(SubtitleQuality::Fast);
                            }
                            return self.continue_create();
                        }
                        ConsentChoice::Download => (level, ModelSource::Download),
                        ConsentChoice::DownloadFastInstead => {
                            (SubtitleQuality::Fast, ModelSource::Download)
                        }
                        ConsentChoice::LocalFile => match self.dialogs.pick_model_file() {
                            Some(p) => (level, ModelSource::LocalFile(p)),
                            None => return Ok(()),
                        },
                    };
                    let cancel = Arc::new(AtomicBool::new(false));
                    let rx = stt::install_model(
                        store,
                        level,
                        source,
                        self.fetcher.clone(),
                        Arc::clone(&cancel),
                    );
                    self.model_install = Some((rx, cancel));
                    self.model_progress = Some(0.0);
                    self.status = "Installing the speech recognition file…".into();
                    Ok(())
                }
                Command::CancelSubtitles => {
                    self.pending_create = None;
                    if let Some(h) = &self.stt {
                        h.cancel();
                    }
                    if let Some((_, c)) = &self.model_install {
                        c.store(true, Ordering::SeqCst);
                    }
                    Ok(())
                }
                Command::AddSubtitle => {
                    let (at, min) = (self.playhead, self.frame_len());
                    let mut added = None;
                    // on a 360 clip the line is fixed to the point the camera looks at
                    let anchor = self.text_anchor_at_playhead(false);
                    self.edit(now, label, |e| {
                        let id = e
                            .subtitles
                            .add_at(&e.timeline, at, "New subtitle", min)
                            .map_err(err)?;
                        if let Some(l) = e.subtitles.lines.iter_mut().find(|l| l.id == id) {
                            l.anchor = anchor;
                        }
                        added = Some(id);
                        Ok(())
                    })?;
                    if let Some(id) = added {
                        self.selection.select_only(ObjRef::Line(id));
                    }
                    Ok(())
                }
                Command::DeleteSubtitle(id) => {
                    self.edit(now, label, |e| e.subtitles.delete(id).map_err(err))?;
                    Ok(())
                }
                Command::SetSubtitleText(id, text) => {
                    let cur = self
                        .project
                        .subtitles
                        .get(id)
                        .ok_or("no such subtitle line")?;
                    if cur.text == crate::subtitles::clean_text(&text) {
                        return Ok(());
                    }
                    // the box keeps its centre and stays inside the picture (REQ-168, REQ-174)
                    let place = self.place_after_text_edit(id, &text);
                    self.edit(now, label, |e| {
                        e.subtitles.set_text(id, &text).map_err(err)?;
                        match place {
                            Some(p) => e.subtitles.set_position(&[id], Some(p)).map_err(err),
                            None => Ok(()),
                        }
                    })
                }
                Command::RetimeSubtitle {
                    line,
                    clip,
                    start,
                    end,
                } => {
                    let min = self.frame_len();
                    self.edit(now, label, |e| {
                        e.subtitles
                            .retime(line, &e.timeline, clip, start, end, min)
                            .map_err(err)
                    })
                }
                Command::DeleteSubtitles(ids) => {
                    self.edit(now, label, |e| e.subtitles.delete_many(&ids).map_err(err))?;
                    self.prune_selection();
                    Ok(())
                }
                Command::ShiftLines { ids, delta } => {
                    let limits: std::collections::HashMap<MediaId, Micros> = self
                        .project
                        .media
                        .iter()
                        .map(|m| (m.id, m.duration))
                        .collect();
                    let limit = |m: MediaId| limits.get(&m).copied().unwrap_or(Micros::ZERO);
                    // nothing can move (a line at 0 s shifted earlier): no undo step
                    let mut probe = self.project.subtitles.clone();
                    if probe.shift(&ids, delta, limit).map_err(err)? == Micros::ZERO {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.subtitles
                            .shift(&ids, delta, limit)
                            .map(|_| ())
                            .map_err(err)
                    })?;
                    Ok(())
                }
                Command::SelectLine(id) => {
                    self.selection.clear();
                    if let Some(id) = id {
                        self.selection.select_only(ObjRef::Line(id));
                    }
                    Ok(())
                }
                Command::SetPreset(p) => {
                    if p == self.project.subtitles.preset {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.subtitles.preset = p;
                        Ok(())
                    })
                }
                Command::SetSubtitleStyle(st) => {
                    let st = st.clamped();
                    if st == self.project.subtitles.style {
                        return Ok(());
                    }
                    self.edit(now, label, |e| {
                        e.subtitles.style = st;
                        Ok(())
                    })
                }
                Command::SetLanguage(lang) => {
                    if let Some(l) = &lang
                        && !(2..=3).contains(&l.len())
                    {
                        return Err(format!("'{l}' is not a language code"));
                    }
                    if let Some(l) = &lang
                        && !l.chars().all(|c| c.is_ascii_lowercase())
                    {
                        return Err(format!("'{l}' is not a language code"));
                    }
                    self.edit(now, label, |e| {
                        e.subtitles.language_overridden = lang.is_some();
                        e.subtitles.language = lang;
                        Ok(())
                    })
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
