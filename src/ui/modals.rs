// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dialogs shown over the window: ffmpeg missing, notices, unsaved changes, speech-model
//! consent, regenerate confirmation and About.

use super::UiCache;
use crate::app::{AppState, Command, ConsentChoice, Modal, PendingAction, UnsavedChoice};
use crate::stt;
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

/// Text of the speech-model consent dialog.
pub fn consent_text() -> String {
    format!(
        "Generating subtitles needs a speech model that is downloaded once.\n\n\
         File: {}\nSize: {} MB\nLicence: {}\nSource: {}\n\n\
         This download is the only time videolore uses the network. After it, subtitles \
         are generated on this computer, offline. You can also use a copy of the same file \
         that you already have.",
        stt::MODEL_FILE,
        stt::MODEL_SIZE / 1_000_000,
        stt::MODEL_LICENCE,
        stt::MODEL_SOURCE
    )
}

pub fn modals(ctx: &Context, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    if app.modal != Some(Modal::About) {
        cache.about = None;
    }
    if let Err(e) = &app.ffmpeg
        && !app.ffmpeg_checking
    {
        egui::Modal::new(egui::Id::unique("ffmpeg-missing")).show(ctx, |ui| {
            ui.heading("ffmpeg is required");
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
                ui.heading("Unsaved changes");
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
        Some(Modal::ModelConsent) => {
            egui::Modal::new(egui::Id::unique("consent")).show(ctx, |ui| {
                ui.heading("Download the speech model?");
                ui.label(consent_text());
                ui.horizontal(|ui| {
                    for (label, choice) in [
                        ("Download", ConsentChoice::Download),
                        ("Use a file I have…", ConsentChoice::LocalFile),
                        ("Cancel", ConsentChoice::Cancel),
                    ] {
                        if ui.button(label).clicked() {
                            cmds.push(Command::ModelConsent(choice));
                        }
                    }
                });
            });
        }
        Some(Modal::ConfirmRegenerate) => {
            egui::Modal::new(egui::Id::unique("regenerate")).show(ctx, |ui| {
                ui.heading("Replace the subtitles?");
                ui.label(
                    "You changed some subtitle lines by hand. Generating again replaces all \
                     lines, including your changes (you can undo it).",
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
        Some(Modal::About) => {
            let text = cache
                .about
                .get_or_insert_with(|| crate::notices::about_text(app.ffmpeg.as_ref().ok()));
            egui::Modal::new(egui::Id::unique("about")).show(ctx, |ui| {
                ui.heading("About videolore");
                let h = (ctx.content_rect().height() * 0.6).max(100.0);
                egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
                    ui.monospace(text.as_str());
                });
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
        }
    }
}
