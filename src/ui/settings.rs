// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Settings window (Edit > Settings…, Ctrl/Cmd+,) and the message banner.
//!
//! The main section uses only plain words (see [`crate::settings::main_texts`]); technical
//! details (ffmpeg, speech engine files, processor, cores) are in the collapsed
//! "Advanced" section.

use super::strings::tr;
use crate::app::{AppState, BannerAction, Command, HwProbe};
use crate::export::ExportMethod;
use crate::hwenc::Codec;
use crate::settings::{
    self, ADVANCED_TITLE, CLEAR_PREVIEW_COPIES, CLOSE, CUSTOM_STATUS, EFFORT_TITLE, EXPORT_TITLE,
    Effort, ExportChoice, NO_HARDWARE_NOTE, PREVIEW_COPIES, PREVIEW_TITLE, PreviewQuality,
    RECOMMENDED_STATUS, REMOVE_SPEECH_FILES, SLOWER_NOTE, SPEECH_FILES, SUBTITLE_TITLE,
    SubtitleQuality, USE_RECOMMENDED,
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
        let note = (e == ExportChoice::Faster)
            .then(|| {
                if no_hardware(app) {
                    Some(NO_HARDWARE_NOTE)
                } else {
                    integrated_note(app)
                }
            })
            .flatten();
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

/// Added to the Faster explanation when the hardware the next export would use is the
/// processor's own graphics (REQ-207).
pub const INTEGRATED_NOTE: &str = "On this computer Faster may not be quicker than Best quality.";

/// [`INTEGRATED_NOTE`] when the check found integrated graphics for a format (the device
/// the export would use: Automatic, or the one chosen); never with dedicated graphics.
pub fn integrated_note(app: &AppState) -> Option<&'static str> {
    let HwProbe::Done(s) = &app.hw.probe else {
        return None;
    };
    let device = app.settings.graphics_card.as_deref();
    Codec::ALL
        .into_iter()
        .filter_map(|c| s.chosen(c, device))
        .any(|c| c.graphics_here() == crate::hwenc::Graphics::Integrated)
        .then_some(INTEGRATED_NOTE)
}

/// While the automatic repeat of an inconclusive check runs (REQ-211).
pub const REPEATING: &str = "check inconclusive, repeating";

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
    if app.hw_repeating() {
        out.push(REPEATING.to_string());
    }
    if let HwProbe::Done(s) = &app.hw.probe {
        if let Some(n) = &s.libva_note {
            out.push(n.clone());
        }
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

/// Label of the Automatic choice of the graphics card (REQ-216).
pub const AUTOMATIC_CARD: &str = "Automatic";
/// Title of the list of video hardware devices.
pub const GRAPHICS_CARD_TITLE: &str = "Video hardware to use";

/// The working video hardware by device name, Automatic first (REQ-216): what the list
/// shows, and what [`Command::SetGraphicsCard`] accepts.
pub fn graphics_card_choices(app: &AppState) -> Vec<String> {
    match &app.hw.probe {
        HwProbe::Done(s) => s.devices(),
        _ => Vec::new(),
    }
}

/// The list of devices: Automatic (the preference order) and each working device.
fn graphics_card_choice(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let devices = graphics_card_choices(app);
    if devices.is_empty() {
        return;
    }
    ui.label(GRAPHICS_CARD_TITLE);
    let current = app.settings.graphics_card.as_deref();
    if ui
        .selectable_label(current.is_none(), AUTOMATIC_CARD)
        .clicked()
        && current.is_some()
    {
        cmds.push(Command::SetGraphicsCard(None));
    }
    for d in devices {
        if ui
            .selectable_label(current == Some(d.as_str()), &d)
            .clicked()
            && current != Some(d.as_str())
        {
            cmds.push(Command::SetGraphicsCard(Some(d)));
        }
    }
}

fn advanced_section(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    egui::CollapsingHeader::new(ADVANCED_TITLE)
        .default_open(false)
        .show(ui, |ui| {
            for l in advanced_lines(app) {
                ui.label(l);
            }
            graphics_card_choice(ui, app, cmds);
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

/// The preview copies' size limits offered, in bytes (REQ-297).
pub const CACHE_LIMITS: [u64; 5] = [
    1_000_000_000,
    2_000_000_000,
    4_000_000_000,
    8_000_000_000,
    16_000_000_000,
];

/// Text size, start-up picture and the limit of the preview copies, in plain words
/// (REQ-244, REQ-295, REQ-297).
fn look_section(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    use crate::settings::SplashOption;
    use crate::settings::interface::TextSize;
    ui.strong(tr!(TextSize));
    ui.horizontal(|ui| {
        for (t, text) in [
            (TextSize::Normal, tr!(TextSizeNormal)),
            (TextSize::Large, tr!(TextSizeLarge)),
            (TextSize::Larger, tr!(TextSizeLarger)),
        ] {
            if ui
                .radio(app.settings.interface.text_size == t, text)
                .clicked()
            {
                cmds.push(Command::SetTextSize(t));
            }
        }
    });
    ui.strong(tr!(StartPicture));
    ui.horizontal_wrapped(|ui| {
        for (o, text) in [
            (SplashOption::Always, tr!(AtEveryStart)),
            (SplashOption::OncePerVersion, tr!(OnceForEachVersion)),
        ] {
            if ui.radio(app.settings.show_splash == o, text).clicked() {
                cmds.push(Command::SetSplashOption(o));
            }
        }
    });
    let mut strips = app.settings.show_picture_strips;
    if ui.checkbox(&mut strips, tr!(ShowPictureStrips)).changed() {
        cmds.push(Command::SetPictureStrips(strips));
    }
    ui.horizontal(|ui| {
        ui.label(tr!(PreviewCopiesUpTo));
        for limit in CACHE_LIMITS {
            let text = format!("{} GB", limit / 1_000_000_000);
            if ui.radio(app.settings.cache_limit == limit, text).clicked() {
                cmds.push(Command::SetCacheLimit(limit));
            }
        }
    });
}

/// The content of the Settings window (a popup, see [`super::popups`]).
pub fn settings_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    main_section(ui, app, cmds);
    ui.separator();
    egui::CollapsingHeader::new(tr!(AppearanceAndStart))
        .default_open(false)
        .show(ui, |ui| look_section(ui, app, cmds));
    advanced_section(ui, app, cmds);
    ui.separator();
    if ui.button(CLOSE).clicked() {
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
                        if ui.button(tr!(CloseMark)).clicked() {
                            cmds.push(Command::DismissBanner);
                        }
                    });
                });
        });
}
