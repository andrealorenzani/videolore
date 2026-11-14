// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Settings window (opened from the command bar) and the message banner.
//!
//! The main section uses only plain words (see [`crate::settings::main_texts`]); technical
//! details (ffmpeg, speech engine files, processor, cores) are in the collapsed
//! "Advanced" section.

use crate::app::{AppState, BannerAction, Command, HwProbe};
use crate::export::ExportMethod;
use crate::hwenc::Codec;
use crate::settings::{
    self, ADVANCED_TITLE, CLEAR_PREVIEW_COPIES, CLOSE, CUSTOM_STATUS, EFFORT_TITLE, EXPORT_TITLE,
    Effort, ExportChoice, NO_HARDWARE_NOTE, PREVIEW_COPIES, PREVIEW_TITLE, PreviewQuality,
    RECOMMENDED_STATUS, REMOVE_SPEECH_FILES, SETTINGS_TITLE, SLOWER_NOTE, SPEECH_FILES,
    SUBTITLE_TITLE, SubtitleQuality, USE_RECOMMENDED,
};
use eframe::egui::{self, Align2, Color32, Context, RichText, Ui};

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

/// One choice: a radio button with its plain explanation; greyed with `reason` when it
/// cannot be used here.
fn choice(
    ui: &mut Ui,
    group: &str,
    selected: bool,
    label: &str,
    explanation: &str,
    reason: Option<&str>,
) -> bool {
    let clicked = ui
        .add_enabled(reason.is_none(), egui::RadioButton::new(selected, label))
        .clicked();
    ui.indent((group, label), |ui| {
        ui.label(RichText::new(explanation).small());
        if let Some(r) = reason {
            ui.label(RichText::new(r).small().italics());
        }
    });
    clicked
}

fn main_section(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let fp = app.fingerprint();
    ui.horizontal(|ui| {
        ui.label(if app.is_recommended() {
            RECOMMENDED_STATUS
        } else {
            CUSTOM_STATUS
        });
        if ui.button(USE_RECOMMENDED).clicked() {
            cmds.push(Command::UseRecommended);
        }
    });
    for line in &app.settings_summary {
        ui.label(RichText::new(line).italics());
    }
    ui.separator();
    ui.strong(PREVIEW_TITLE);
    for q in PreviewQuality::ALL {
        let on = app.settings.preview_quality == q;
        if choice(ui, "preview", on, q.label(), q.explanation(), None) && !on {
            cmds.push(Command::SetPreviewQuality(q));
        }
    }
    ui.separator();
    ui.strong(SUBTITLE_TITLE);
    let available = settings::availability(&fp);
    for q in SubtitleQuality::ALL {
        let on = app.settings.subtitle_quality == q;
        let reason = available[q.index()];
        if choice(ui, "subtitles", on, q.label(), q.explanation(), reason) && !on {
            cmds.push(Command::SetSubtitleQuality(q));
        }
    }
    if !fp.speech_supported {
        ui.label(SLOWER_NOTE);
    }
    ui.separator();
    ui.strong(EFFORT_TITLE);
    for e in Effort::ALL {
        let on = app.settings.effort == e;
        if choice(ui, "effort", on, e.label(), e.explanation(), None) && !on {
            cmds.push(Command::SetEffort(e));
        }
    }
    ui.separator();
    ui.strong(EXPORT_TITLE);
    for e in ExportChoice::ALL {
        let on = app.settings.export == e;
        let note = (e == ExportChoice::Faster && no_hardware(app)).then_some(NO_HARDWARE_NOTE);
        if choice(ui, "export", on, e.label(), e.explanation(), None) && !on {
            cmds.push(Command::SetExport(e));
        }
        if let Some(n) = note {
            ui.indent(("export-note", e.label()), |ui| {
                ui.label(RichText::new(n).small());
            });
        }
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(format!("{PREVIEW_COPIES}: {}", mb(app.proxy_cache_size)));
        super::button(
            ui,
            app,
            CLEAR_PREVIEW_COPIES,
            Command::ClearProxyCache,
            cmds,
        );
    });
    ui.horizontal(|ui| {
        ui.label(format!("{SPEECH_FILES}: {}", mb(app.speech_files_size)));
        super::button(
            ui,
            app,
            REMOVE_SPEECH_FILES,
            Command::RemoveSpeechFiles,
            cmds,
        );
    });
}

/// The hardware check finished and found nothing usable for either format.
pub fn no_hardware(app: &AppState) -> bool {
    matches!(&app.hw.probe, HwProbe::Done(s) if s.none_found())
}

/// "Turned off after a problem" line of the Advanced section.
pub const HARDWARE_OFF_LINE: &str = "Video hardware turned off after a problem";
/// Button of the Advanced section.
pub const TRY_HARDWARE_AGAIN: &str = "Try video hardware again";

/// The video hardware lines of the Advanced section.
pub fn hardware_lines(app: &AppState) -> Vec<String> {
    let mut out = Vec::new();
    for codec in Codec::ALL {
        let state = match &app.hw.probe {
            HwProbe::NotChecked => "not checked (only checked when Export is Faster)".to_string(),
            HwProbe::Checking(_) => "checking…".to_string(),
            HwProbe::Done(s) => match s.get(codec) {
                Some(c) if app.settings.hardware_is_off(codec, c.vendor) => {
                    format!("{} — turned off after a problem", c.label())
                }
                Some(c) => format!("{} — works", c.label()),
                None => "none found".to_string(),
            },
        };
        out.push(format!("Video hardware for {}: {state}", codec.label()));
    }
    if let HwProbe::Done(s) = &app.hw.probe {
        for (codec, vendor, reason) in &s.notes {
            out.push(format!(
                "{} ({}): {}",
                vendor.maker(),
                codec.label(),
                reason.text()
            ));
        }
    }
    if let Some(m) = &app.hw.last_method {
        out.push(format!("Last export: {}", method_text(m)));
    }
    if !app.settings.hardware_off.is_empty() {
        out.push(HARDWARE_OFF_LINE.to_string());
    }
    out
}

/// How an export was made, in words.
pub fn method_text(m: &ExportMethod) -> String {
    match m {
        ExportMethod::StreamCopy => "copied without re-encoding".into(),
        ExportMethod::Software => "normal method".into(),
        ExportMethod::Hardware(v) => format!("video hardware ({})", v.label()),
        ExportMethod::SoftwareAfterProblem {
            codec,
            vendor,
            problem,
        } => format!(
            "normal method, after a video hardware problem ({} {}: {})",
            vendor.label(),
            codec.label(),
            problem.text()
        ),
    }
}

/// Lines of the Advanced section (technical details), also used by tests.
pub fn advanced_lines(app: &AppState) -> Vec<String> {
    let m = &app.machine;
    let mut out = Vec::new();
    match &app.ffmpeg {
        Ok(i) => out.push(format!(
            "ffmpeg: {} ({}, {})",
            i.ffmpeg.display(),
            i.version,
            i.source
        )),
        Err(e) => out.push(format!("ffmpeg: {e}")),
    }
    if let Some(p) = &app.settings.ffmpeg {
        out.push(format!("Chosen ffmpeg: {}", p.display()));
    }
    out.push(format!(
        "Speech engine: whisper.cpp ({})",
        if crate::stt::available() {
            "included"
        } else {
            "not in this build"
        }
    ));
    if let Some(store) = &app.model_store {
        for q in SubtitleQuality::ALL {
            let spec = store.spec(q);
            out.push(format!(
                "{}: {} ({} MB, {}){}",
                q.label(),
                spec.file,
                spec.size / 1_000_000,
                crate::stt::MODEL_LICENCE,
                if store.is_installed(q) {
                    " — downloaded"
                } else {
                    ""
                }
            ));
        }
        out.push(format!("Speech files folder: {}", store.dir.display()));
    }
    let missing = m.cpu.missing();
    out.push(if m.speech_supported() {
        format!("Processor: {} {}, supported", m.os, m.arch)
    } else {
        format!(
            "Processor: {} {}, not supported by the speech engine (missing {})",
            m.os,
            m.arch,
            missing.join(", ")
        )
    });
    out.push(format!(
        "Computer: {} logical / {} physical cores, {} GiB memory",
        m.logical,
        m.physical,
        m.ram_gib()
    ));
    let b = &app.budget;
    out.push(format!(
        "Limits now: {} ffmpeg processes, export threads {}, speech threads {}",
        b.max_processes,
        b.threads
            .map(|t| t.to_string())
            .unwrap_or_else(|| "automatic".into()),
        b.stt_threads
    ));
    out.extend(hardware_lines(app));
    out
}

fn advanced_section(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    egui::CollapsingHeader::new(ADVANCED_TITLE)
        .default_open(false)
        .show(ui, |ui| {
            for l in advanced_lines(app) {
                ui.label(l);
            }
            if ui.button(TRY_HARDWARE_AGAIN).clicked() {
                cmds.push(Command::TryHardwareAgain);
            }
            ui.horizontal(|ui| {
                super::button(ui, app, "Choose ffmpeg…", Command::PickFfmpeg, cmds);
                if app.settings.ffmpeg.is_some() {
                    super::button(
                        ui,
                        app,
                        "Use the included ffmpeg",
                        Command::ResetFfmpeg,
                        cmds,
                    );
                }
            });
        });
}

/// The Settings window, when open.
pub fn settings_window(ctx: &Context, app: &AppState, cmds: &mut Vec<Command>) {
    if !app.settings_open {
        return;
    }
    let mut open = true;
    egui::Window::new(SETTINGS_TITLE)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let h = (ctx.content_rect().height() * 0.75).max(200.0);
            egui::ScrollArea::vertical()
                .id_salt("settings-scroll")
                .max_height(h)
                .show(ui, |ui| {
                    main_section(ui, app, cmds);
                    ui.separator();
                    advanced_section(ui, app, cmds);
                    ui.separator();
                    if ui.button(CLOSE).clicked() {
                        cmds.push(Command::CloseSettings);
                    }
                });
        });
    if !open {
        cmds.push(Command::CloseSettings);
    }
}

/// The non-blocking message at the top of the window.
pub fn banner(ctx: &Context, app: &AppState, cmds: &mut Vec<Command>) {
    let Some(b) = &app.banner else { return };
    egui::Area::new(egui::Id::unique("videolore-banner"))
        .anchor(Align2::CENTER_TOP, [0.0, 8.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgb(40, 60, 90))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&b.text).color(Color32::WHITE));
                        if b.action == Some(BannerAction::UseRecommended)
                            && ui.button(USE_RECOMMENDED).clicked()
                        {
                            cmds.push(Command::BannerAction(BannerAction::UseRecommended));
                        }
                        if ui.button("✕").clicked() {
                            cmds.push(Command::DismissBanner);
                        }
                    });
                });
        });
}
