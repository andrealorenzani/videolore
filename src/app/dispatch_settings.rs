// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the settings, speech files and ffmpeg-location commands: the arms of [`AppState::dispatch`] for this area. A command
//! of another area is handed back unchanged ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_settings(&mut self, cmd: Command, now: Instant) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::OpenSettings => {
                    self.dispatch_inner(Command::OpenPopup(Popup::Settings), now)
                }
                Command::CloseSettings => {
                    self.dispatch_inner(Command::ClosePopup(Popup::Settings), now)
                }
                Command::SetPreviewQuality(q) => {
                    self.settings.preview_quality = q;
                    self.settings_summary.clear();
                    self.apply_preview_quality(now);
                    self.save_settings()
                }
                Command::SetSubtitleQuality(q) => {
                    let fp = self.fingerprint();
                    if let Some(reason) = settings::availability(&fp)[q.index()] {
                        return Err(reason.to_string());
                    }
                    // only the choice changes: existing subtitles are never regenerated
                    self.settings.subtitle_quality = q;
                    self.settings_summary.clear();
                    self.save_settings()
                }
                Command::SetEffort(e) => {
                    self.settings.effort = e;
                    self.settings_summary.clear();
                    self.apply_budget();
                    self.save_settings()
                }
                Command::SetExport(e) => {
                    self.settings.export = e;
                    self.settings_summary.clear();
                    if e == ExportChoice::Faster && self.hw.probe == HwProbe::NotChecked {
                        self.start_hw_probe();
                    }
                    // the preview decodes with the hardware only with Faster (REQ-214)
                    self.apply_hw_decoders();
                    // a running export keeps its method; the next one uses the new choice
                    self.save_settings()
                }
                Command::SetGraphicsCard(name) => {
                    // only a device the check found can be chosen; anything else is Automatic
                    let known = matches!(&self.hw.probe, HwProbe::Done(st)
                        if name.as_ref().is_some_and(|n| st.devices().contains(n)));
                    self.settings.graphics_card = name.filter(|_| known);
                    self.apply_hw_decoders();
                    self.save_settings()
                }
                Command::TryHardwareAgain => {
                    self.settings.hardware_off.clear();
                    self.hw_slow.clear();
                    self.start_hw_probe();
                    self.save_settings()
                }
                Command::UseRecommended | Command::BannerAction(BannerAction::UseRecommended) => {
                    self.use_recommended(now)
                }
                Command::RemoveSpeechFiles => {
                    self.modal = Some(Modal::ConfirmRemoveSpeechFiles);
                    Ok(())
                }
                Command::ConfirmRemoveSpeechFiles => {
                    self.modal = None;
                    let store = self
                        .model_store
                        .clone()
                        .ok_or("no folder for speech recognition files")?;
                    let freed = store.remove_downloaded();
                    self.refresh_speech_files_size();
                    self.status = format!(
                        "Removed the speech recognition files ({} MB freed)",
                        freed / 1_000_000
                    );
                    Ok(())
                }
                Command::ResetFfmpeg => {
                    self.settings.ffmpeg = None;
                    self.settings.ffprobe = None;
                    self.save_settings()?;
                    self.dispatch_inner(Command::RetryFfmpeg, now)
                }
                Command::DismissBanner => {
                    self.banner = None;
                    Ok(())
                }
                Command::ClearProxyCache => {
                    let cache = self.proxy_cache.clone().ok_or("no proxy cache")?;
                    let busy: Vec<String> =
                        self.proxy_jobs.values().map(|(k, _)| k.clone()).collect();
                    let freed = cache.clear(&busy);
                    for m in &mut self.project.media {
                        if let ProxyState::Ready(p) = &m.proxy
                            && !p.is_file()
                        {
                            m.proxy = ProxyState::None;
                        }
                    }
                    // the copies of the open project are made again in the background (REQ-296)
                    let ids: Vec<MediaId> = self.project.media.iter().map(|m| m.id).collect();
                    for id in ids {
                        self.maybe_proxy(id);
                    }
                    self.proxy_cache_size = cache.size();
                    self.status = format!("Freed {} MB of proxies", freed / 1_000_000);
                    self.refresh_still(now);
                    Ok(())
                }
                Command::SetCacheLimit(bytes) => {
                    let (lo, hi) = settings::CACHE_LIMIT_RANGE;
                    self.settings.cache_limit = bytes.clamp(lo, hi);
                    self.enforce_cache_limit();
                    self.proxy_cache_size = self.proxy_cache.as_ref().map_or(0, ProxyCache::size);
                    self.save_settings()
                }
                Command::SetSplashOption(o) => {
                    self.settings.show_splash = o;
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
