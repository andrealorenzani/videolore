// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dialogs shown over the window: ffmpeg missing, notices, unsaved changes, speech-file
//! consent, regenerate and remove confirmations, and About.

use super::UiCache;
use crate::app::{
    AppState, Command, ConsentChoice, Modal, PendingAction, SourceChoice, UnsavedChoice,
};
use crate::settings::SubtitleQuality;
use crate::stt::{self, ModelSpec};
use eframe::egui::{self, Context};

/// The question asked before `action` when there are unsaved changes.
pub fn unsaved_question(app: &AppState, action: &PendingAction) -> String {
    let name = app
        .project_path
        .as_deref()
        .map(crate::media::display_name)
        .unwrap_or_else(|| "Untitled".into());
    let what = match action {
        PendingAction::New => "starting a new project",
        PendingAction::Open(_) => "opening another project",
        PendingAction::Close | PendingAction::Quit => "quitting",
    };
    format!("Save the changes to \"{name}\" before {what}?")
}

/// Text of the consent dialog for the speech recognition file of `level`.
pub fn consent_text(level: SubtitleQuality, spec: &ModelSpec) -> String {
    format!(
        "Subtitle quality \"{}\" needs a speech recognition file that is downloaded once.\n\n\
         File: {}\nSize: {} MB\nLicence: {}\nSource: {}\n\n\
         Downloads like this one are the only times videolore uses the network. After it, \
         subtitles are generated on this computer, offline. You can also use a copy of the \
         same file that you already have.",
        level.label(),
        spec.file,
        spec.size / 1_000_000,
        stt::MODEL_LICENCE,
        stt::MODEL_SOURCE
    )
}

/// The buttons of the consent dialog: Fast alternatives only when they make sense.
pub fn consent_choices(app: &AppState, level: SubtitleQuality) -> Vec<(String, ConsentChoice)> {
    let mut v = vec![("Download".to_string(), ConsentChoice::Download)];
    if level != SubtitleQuality::Fast
        && let Some(store) = &app.model_store
    {
        if store.is_installed(SubtitleQuality::Fast) {
            v.push((
                "Use Fast instead (no download)".into(),
                ConsentChoice::UseFastInstead,
            ));
        } else if SubtitleQuality::ALL.iter().all(|l| !store.is_installed(*l)) {
            let mb = store.spec(SubtitleQuality::Fast).size / 1_000_000;
            v.push((
                format!("Download the Fast one instead ({mb} MB)"),
                ConsentChoice::DownloadFastInstead,
            ));
        }
    }
    v.push((
        "Choose a file on this computer…".into(),
        ConsentChoice::LocalFile,
    ));
    if app.consent_offers_spread() {
        v.push((
            "Use without matching words".into(),
            ConsentChoice::UseWithoutMatching,
        ));
    }
    v.push(("Cancel".into(), ConsentChoice::Cancel));
    v
}

/// The question of the subtitle source dialog (REQ-087).
pub const SOURCE_QUESTION: &str =
    "This video already contains subtitles. Use their text, or recognise the speech?";

/// The "use" button of the subtitle source dialog: on computers that cannot match words
/// to the speech it says what will happen.
pub fn source_use_label(can_match: bool) -> &'static str {
    if can_match {
        "Use the video's subtitles"
    } else {
        "Use the video's subtitles (this computer cannot match words to the speech; they \
         will be spread evenly)"
    }
}

pub fn modals(ctx: &Context, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    if app.modal != Some(Modal::About) {
        cache.about = None;
    }
    if let Err(e) = &app.ffmpeg
        && !app.ffmpeg_checking
    {
        egui::Modal::new(egui::Id::unique("ffmpeg-missing")).show(ctx, |ui| {
            ui.heading(super::icons::label(
                super::icons::WARNING,
                "ffmpeg is required",
            ));
            ui.label(e);
            ui.label(crate::ffmpeg::install_hint());
            ui.label("You can also set VIDEOLORE_FFMPEG / VIDEOLORE_FFPROBE.");
            ui.horizontal(|ui| {
                if ui.button("Choose ffmpeg…").clicked() {
                    cmds.push(Command::PickFfmpeg);
                }
                if ui.button("Retry").clicked() {
                    cmds.push(Command::RetryFfmpeg);
                }
                if ui.button("Quit").clicked() {
                    cmds.push(Command::Quit);
                }
            });
        });
        return;
    }
    if let Some(n) = &app.notice {
        egui::Modal::new(egui::Id::unique("notice")).show(ctx, |ui| {
            ui.heading(super::icons::label(super::icons::INFO, "Notice"));
            ui.label(n);
            if ui.button("OK").clicked() {
                cmds.push(Command::DismissNotice);
            }
        });
        return;
    }
    match &app.modal {
        None => {}
        Some(Modal::Unsaved(action)) => {
            egui::Modal::new(egui::Id::unique("unsaved")).show(ctx, |ui| {
                ui.heading(super::icons::label(
                    super::icons::FLOPPY_DISK,
                    "Unsaved changes",
                ));
                ui.label(unsaved_question(app, action));
                ui.horizontal(|ui| {
                    for (label, choice) in [
                        ("Save", UnsavedChoice::Save),
                        ("Don't save", UnsavedChoice::Discard),
                        ("Cancel", UnsavedChoice::Cancel),
                    ] {
                        if ui.button(label).clicked() {
                            cmds.push(Command::ResolveUnsaved(choice));
                        }
                    }
                });
            });
        }
        Some(Modal::ModelConsent(level)) => {
            let Some(store) = &app.model_store else {
                return;
            };
            let text = consent_text(*level, store.spec(*level));
            let choices = consent_choices(app, *level);
            egui::Modal::new(egui::Id::unique("consent")).show(ctx, |ui| {
                ui.heading(super::icons::label(
                    super::icons::DOWNLOAD_SIMPLE,
                    "Download a speech recognition file?",
                ));
                ui.label(text);
                ui.horizontal_wrapped(|ui| {
                    for (label, choice) in choices {
                        if ui.button(label).clicked() {
                            cmds.push(Command::ModelConsent(choice));
                        }
                    }
                });
            });
        }
        Some(Modal::ConfirmRemoveSpeechFiles) => {
            egui::Modal::new(egui::Id::unique("remove-speech")).show(ctx, |ui| {
                ui.heading(super::icons::label(
                    super::icons::TRASH,
                    "Remove the downloaded speech files?",
                ));
                ui.label(
                    "Your projects and their subtitles are kept. Generating subtitles again \
                     will ask to download the files again.",
                );
                ui.horizontal(|ui| {
                    if ui.button("Remove").clicked() {
                        cmds.push(Command::ConfirmRemoveSpeechFiles);
                    }
                    if ui.button("Cancel").clicked() {
                        cmds.push(Command::CloseModal);
                    }
                });
            });
        }
        Some(Modal::SubtitleSource { labels, can_match }) => {
            if cache.source_choice >= labels.len() {
                cache.source_choice = 0;
            }
            let mut container = None;
            egui::Modal::new(egui::Id::unique("subtitle-source")).show(ctx, |ui| {
                ui.set_max_width(ctx.content_rect().width().min(520.0) - 40.0);
                ui.heading(super::icons::label(
                    super::icons::SUBTITLES,
                    "Subtitles found",
                ));
                ui.label(SOURCE_QUESTION);
                if labels.len() > 1 {
                    ui.label("Subtitles to use:");
                    // many tracks: the list scrolls, the buttons stay in view
                    let h = (ctx.content_rect().height() * 0.3).max(60.0);
                    let out = egui::ScrollArea::vertical()
                        .id_salt("subtitle-tracks")
                        .max_height(h)
                        .show(ui, |ui| {
                            for (i, l) in labels.iter().enumerate() {
                                if ui
                                    .add(
                                        egui::Button::selectable(cache.source_choice == i, l)
                                            .truncate(),
                                    )
                                    .on_hover_text(l)
                                    .clicked()
                                {
                                    cache.source_choice = i;
                                }
                            }
                        });
                    container =
                        Some((out.inner_rect, out.state.clamped_offset(), out.content_size));
                } else if let Some(l) = labels.first() {
                    ui.add(egui::Label::new(format!("Subtitles: {l}")).truncate());
                }
                ui.weak("Either way, you can correct the text afterwards.");
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add(egui::Button::new(source_use_label(*can_match)).wrap())
                        .clicked()
                    {
                        cmds.push(Command::ChooseSubtitleSource(SourceChoice::Use(
                            cache.source_choice,
                        )));
                    }
                    if ui.button("Recognise speech").clicked() {
                        cmds.push(Command::ChooseSubtitleSource(SourceChoice::Recognise));
                    }
                    if ui.button("Cancel").clicked() {
                        cmds.push(Command::ChooseSubtitleSource(SourceChoice::Cancel));
                    }
                });
            });
            if let Some((viewport, offset, size)) = container {
                super::record(cache, viewport, offset, size);
            }
        }
        Some(Modal::ConfirmRegenerate) => {
            egui::Modal::new(egui::Id::unique("regenerate")).show(ctx, |ui| {
                ui.heading(super::icons::label(
                    super::icons::WARNING,
                    "Replace the subtitles?",
                ));
                ui.label(
                    "You changed some subtitle lines by hand. Creating subtitles again \
                     replaces the lines it covers, including your changes (you can undo it).",
                );
                ui.horizontal(|ui| {
                    if ui.button("Replace").clicked() {
                        cmds.push(Command::ConfirmRegenerate);
                    }
                    if ui.button("Cancel").clicked() {
                        cmds.push(Command::CloseModal);
                    }
                });
            });
        }
        Some(Modal::Export(c)) => super::export_dialog::export_dialog(ctx, app, c, cmds),
        Some(Modal::ConfirmOverwrite(c)) => {
            super::export_dialog::overwrite_dialog(ctx, app, c, cmds)
        }
        Some(Modal::About) => {
            let text = cache
                .about
                .get_or_insert_with(|| crate::notices::about_text(app.ffmpeg.as_ref().ok()))
                .clone();
            let mut container = None;
            egui::Modal::new(egui::Id::unique("about")).show(ctx, |ui| {
                ui.heading(super::icons::label(super::icons::INFO, "About videolore"));
                let h = (ctx.content_rect().height() * 0.6).max(100.0);
                let area = egui::ScrollArea::vertical().max_height(h);
                let out = area.show(ui, |ui| {
                    ui.monospace(text.as_str());
                });
                container = Some((out.inner_rect, out.state.clamped_offset(), out.content_size));
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        cmds.push(Command::CloseModal);
                    }
                    if app.desktop_integrated && ui.button("Remove from application menu").clicked()
                    {
                        cmds.push(Command::RemoveDesktopIntegration);
                    }
                });
            });
            if let Some((viewport, offset, size)) = container {
                super::record(cache, viewport, offset, size);
            }
        }
    }
}
