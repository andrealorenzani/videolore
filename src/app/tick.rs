// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The once-per-frame update of [`AppState`]: collects the results of the background work.

use super::*;

impl AppState {
    /// Per-frame housekeeping: background results, preview frames, playhead, export.
    pub fn tick(&mut self, now: Instant) {
        self.tick_camera(now);
        self.tick_lut_load(now);
        self.tick_lut_prep(now);
        self.tick_autosave(now);
        // a window that changed is saved once it stays so for a moment (REQ-234)
        if let Some(t) = self.window_dirty
            && now.duration_since(t) >= Duration::from_millis(500)
        {
            self.window_dirty = None;
            let _ = self.save_settings();
        }
        if !self.popups.is_open(Popup::Splash)
            && let Some(b) = self.banner_after_splash.take()
        {
            self.banner = Some(b);
        }
        if let Some(b) = &mut self.banner
            && let Some(life) = b.lifetime
        {
            let until = *b.until.get_or_insert(now + life);
            if now >= until {
                self.banner = None;
            }
        }
        if let Some(rx) = &self.ffmpeg_check
            && let Ok(r) = rx.try_recv()
        {
            self.set_ffmpeg(r);
        }
        if let Some((generation, rx, _)) = &self.hw_probe
            && let Ok((g, status, decoders)) = rx.try_recv()
        {
            let mut again = false;
            // a repeat keeps the first answer in use (and on screen) until it ends
            let repeat = self.hw_rechecked && matches!(self.hw.probe, HwProbe::Done(_));
            if g == *generation && (self.hw.probe == HwProbe::Checking(g) || repeat) {
                again = status.inconclusive() && !self.hw_rechecked;
                self.hw_slow.extend(status.slow.iter().copied());
                self.hw.decoders = decoders;
                self.hw.probe = HwProbe::Done(status);
                self.apply_hw_decoders();
            }
            self.hw_probe = None;
            if again {
                self.hw_rechecked = true;
                self.launch_hw_probe();
            }
        }
        self.pause_or_resume_hw_probe();
        let requests = (self.open_requests)();
        if !requests.is_empty() {
            // a file opened by the system (a double-click) makes the splash pointless
            self.popups.close(Popup::Splash);
        }
        for p in requests {
            let _ = self.dispatch(Command::OpenProject(p), now);
        }
        self.tick_probes();
        self.tick_tracks();
        self.tick_proxies(now);
        self.tick_stt(now);
        self.tick_model();
        if let Some(f) = self.preview.poll(now) {
            self.frame = Some(f);
            self.frame_serial += 1;
        }
        if self.preview.take_hw_fallback() && self.preview.is_playing() {
            // the hardware decoder failed: go on from the playhead with the normal decoder;
            // the picture on screen stays (REQ-144) and nothing is said (REQ-214)
            self.hw_decoders.clear();
            let at = self.preview.position(now);
            let _ = self.preview.play(&self.project, at, now);
        }
        if self.preview.is_playing() {
            self.playhead = self.preview.position(now);
            if let Some(end) = self.stop_at
                && self.playhead >= end
            {
                // "Play this line" reached the end of the line
                self.stop_at = None;
                self.preview.pause(now);
                self.seek(end, now);
                self.refresh_still(now);
            }
        } else if self.preview.position(now) != self.playhead
            && self.preview.position(now) == self.project.timeline.duration()
        {
            // playback reached the end
            self.playhead = self.preview.position(now);
        }
        let events: Vec<ExportEvent> = self
            .export
            .as_ref()
            .map(|h| h.rx.try_iter().collect())
            .unwrap_or_default();
        for e in events {
            match e {
                ExportEvent::Started { stream_copy } => {
                    self.export_status.stream_copy = Some(stream_copy)
                }
                ExportEvent::Progress(p) => self.export_status.progress = p,
                ExportEvent::Done { path, method } => {
                    // a constant-quality file that came out too big is about the video, not a
                    // fault of the hardware: nothing is remembered
                    if let ExportMethod::SoftwareAfterProblem {
                        codec,
                        vendor,
                        problem,
                    } = &method
                        && !matches!(
                            problem,
                            export::Problem::CheckFailed(crate::outcheck::CheckFailure::TooLarge(
                                _
                            ))
                        )
                    {
                        self.settings
                            .turn_hardware_off(*codec, *vendor, env!("CARGO_PKG_VERSION"));
                        let _ = self.save_settings();
                    }
                    self.hw.last_method = Some(method);
                    self.finish_export(format!("Exported to {}", path.display()));
                }
                ExportEvent::Failed(m) => {
                    self.notice = Some(format!("Export failed: {m}"));
                    self.finish_export("Export failed".into());
                }
                ExportEvent::Cancelled => self.finish_export("Export cancelled".into()),
            }
        }
    }

    pub(super) fn tick_probes(&mut self) {
        let results = self.probes.results();
        if results.is_empty() {
            return;
        }
        let mut rejected = Vec::new();
        for (id, r) in results {
            if self.probe_cancel.remove(&id).is_none() {
                continue; // removed meanwhile
            }
            let Some(slot) = self.project.media.iter_mut().find(|m| m.id == id) else {
                continue;
            };
            match r {
                Ok(item) => {
                    *slot = item;
                    if self.selected_media.is_none() {
                        self.selected_media = Some(id);
                    }
                    self.maybe_proxy(id);
                }
                Err(e) => {
                    slot.status = MediaStatus::Rejected(e.clone());
                    rejected.push(e);
                }
            }
        }
        self.refresh_import_offers();
        if self.probe_cancel.is_empty() {
            self.status = if rejected.is_empty() {
                "Import finished".into()
            } else {
                format!("Some files could not be imported: {}", rejected.join("; "))
            };
        }
        self.refresh_dirty();
    }

    /// Keeps the preview copies within the chosen limit (REQ-297); the copies of the open
    /// project stay.
    pub fn enforce_cache_limit(&mut self) {
        let Some(cache) = &self.proxy_cache else {
            return;
        };
        let protected: Vec<String> = self
            .project
            .media
            .iter()
            .filter_map(|m| proxy::key_of_kind(&m.path, m.format.is_360()).ok())
            .collect();
        cache.enforce_limit(self.settings.cache_limit, &protected);
    }

    pub(super) fn tick_proxies(&mut self, now: Instant) {
        let events: Vec<ProxyEvent> = self.proxy_events.1.try_iter().collect();
        let mut changed = false;
        for ev in events {
            let (id, state) = match ev {
                ProxyEvent::Progress(id, p) => (id, ProxyState::Generating(p)),
                ProxyEvent::Ready(id, p) => (id, ProxyState::Ready(p)),
                ProxyEvent::Failed(id, e) => (id, ProxyState::Failed(e)),
            };
            if !self.proxy_jobs.contains_key(&id) {
                continue; // cancelled
            }
            if !matches!(state, ProxyState::Generating(_)) {
                self.proxy_jobs.remove(&id);
                changed = true;
            }
            if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id) {
                m.proxy = state;
            }
        }
        if changed {
            self.enforce_cache_limit();
            self.proxy_cache_size = self.proxy_cache.as_ref().map(ProxyCache::size).unwrap_or(0);
            self.refresh_still(now);
        }
    }

    pub(super) fn tick_stt(&mut self, now: Instant) {
        let Some(h) = &self.stt else { return };
        let events: Vec<SttEvent> = h.rx.try_iter().collect();
        for ev in events {
            match ev {
                SttEvent::Progress(p) => self.stt_progress = Some(p),
                SttEvent::Done(r) => {
                    self.stt = None;
                    self.stt_progress = None;
                    self.finish_create(r, now);
                }
                SttEvent::Failed(m) => {
                    self.stt = None;
                    self.stt_progress = None;
                    self.creating_for = None;
                    self.notice = Some(format!("Subtitles could not be generated: {m}"));
                    self.status = "Subtitle generation failed".into();
                }
                SttEvent::Cancelled => {
                    self.stt = None;
                    self.stt_progress = None;
                    self.creating_for = None;
                    self.status = "Subtitle generation cancelled".into();
                }
            }
        }
    }

    pub(super) fn tick_model(&mut self) {
        let Some((rx, _)) = &self.model_install else {
            return;
        };
        let events: Vec<ModelEvent> = rx.try_iter().collect();
        for ev in events {
            match ev {
                ModelEvent::Progress(p) => self.model_progress = Some(p),
                ModelEvent::Installed(level, _) => {
                    self.model_install = None;
                    self.model_progress = None;
                    self.refresh_speech_files_size();
                    self.status = "Speech recognition file installed".into();
                    // the creation that asked for the file goes on (the whole video when
                    // nothing was waiting)
                    let mut r = self.pending_create.take().unwrap_or(CreateRequest {
                        confirmed: true,
                        ..Default::default()
                    });
                    r.level = Some(level);
                    self.pending_create = Some(r);
                    if let Err(e) = self.continue_create() {
                        self.status = e;
                    }
                }
                ModelEvent::Failed(m) => {
                    self.model_install = None;
                    self.model_progress = None;
                    self.pending_create = None;
                    self.notice = Some(format!(
                        "The speech recognition file could not be installed: {m}"
                    ));
                }
            }
        }
    }

    pub(super) fn finish_export(&mut self, status: String) {
        self.export = None;
        self.export_status.running = false;
        self.status = status;
    }

    /// Window title: project name, a dot when there are unsaved changes.
    pub fn title(&self) -> String {
        let name = self
            .project_path
            .as_deref()
            .map(media::display_name)
            .unwrap_or_else(|| "Untitled".into());
        format!("{name}{} — videolore", if self.dirty { " •" } else { "" })
    }

    /// True while background work may change what is shown (repaint soon).
    pub fn busy(&self) -> bool {
        self.preview.busy()
            || self.is_exporting()
            || self.is_generating()
            || self.ffmpeg_checking
            || self.hw_probe.is_some()
            || self.hw_paused
            || self.track_check.is_some()
            || !self.probe_cancel.is_empty()
            || !self.proxy_jobs.is_empty()
    }

    /// Waits for a running export (tests and shutdown).
    pub fn wait_export(&mut self, now: Instant) {
        if let Some(h) = self.export.take() {
            let events = h.join();
            let (tx, rx) = channel();
            for e in events {
                let _ = tx.send(e);
            }
            self.export = Some(ExportHandle::from_events(rx));
            self.tick(now);
        }
    }
}
