// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Export dialog (REQ-118 to REQ-122) and its overwrite question. The window only
//! shows the choice model of [`crate::export::choice`] and sends the changed choice back
//! as a command; the rules (names, greyed fields, reasons) are there, pure and tested.

use super::strings::tr;
use crate::app::{AppState, Command};
use crate::export::choice::{self, ExportChoice, Mode, Sphere};
use crate::format::{AudioChoice, VIDEO_PRESETS, available_presets};
use eframe::egui::{self, Context, Ui};

pub const TITLE: &str = "Export";
pub const FILE_LABEL: &str = "Export file";
pub const FOLDER_BUTTON: &str = "...";
pub const MODE_LABEL: &str = "Mode";
pub const VIDEO_LABEL: &str = "Video format";
pub const AUDIO_LABEL: &str = "Audio format";
pub const SAME_AS_SOURCE: &str = "Same as source";
pub const RESET: &str = "Reset";
pub const EXPORT_360: &str = "360 video";
pub const EXPORT_FLAT: &str = "Flat camera view";
pub const EXPORT_BUTTON: &str = "Export";
pub const CANCEL: &str = "Cancel";
pub const REPLACE_TITLE: &str = "Replace the file?";
pub const REPLACE: &str = "Replace";

/// Shown before Export when the frame is larger than 4096 px on its longer side (REQ-221).
pub const OVER_CAP_WARNING: &str =
    "This frame is larger than 4096 px: it is scaled down and the export takes longer.";

/// [`OVER_CAP_WARNING`] when it applies to the video `choice` makes (not for Only audio or
/// Only subtitles, nor for a 360 video, which keeps its own size).
pub fn over_cap_warning(app: &AppState, choice: &ExportChoice) -> Option<&'static str> {
    let ctx = app.export_context();
    let sphere = ctx.all_360 && choice.sphere == Sphere::Video360;
    (choice.mode == Mode::Full && !sphere && app.frame_over_cap(choice.video))
        .then_some(OVER_CAP_WARNING)
}

/// The label of the video format dropdown for `choice`.
pub fn video_text(choice: &ExportChoice) -> String {
    match choice.video.and_then(|i| VIDEO_PRESETS.get(i)) {
        Some(p) => p.label(),
        None => SAME_AS_SOURCE.into(),
    }
}

/// The dialog. `choice` is what the app holds now; every change is sent back.
pub fn export_dialog(
    ctx: &Context,
    app: &AppState,
    choice: &ExportChoice,
    cmds: &mut Vec<Command>,
) {
    let ctx_info = app.export_context();
    let mut c = choice.clone();
    let modal = egui::Modal::new(egui::Id::unique("videolore-export-dialog")).show(ctx, |ui| {
        ui.set_max_width(520.0);
        ui.heading(super::icons::label(super::icons::EXPORT, TITLE));
        egui::Grid::new("export-grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label(FILE_LABEL);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut c.name)
                            .desired_width(180.0)
                            .hint_text("file name"),
                    );
                    let mut folder = c.folder.to_string_lossy().into_owned();
                    if ui
                        .add(egui::TextEdit::singleline(&mut folder).desired_width(220.0))
                        .changed()
                    {
                        c.folder = folder.into();
                    }
                    if ui
                        .button(FOLDER_BUTTON)
                        .on_hover_text(tr!(ChooseTheFolder))
                        .clicked()
                    {
                        cmds.push(Command::PickExportFolder);
                    }
                });
                ui.end_row();

                ui.label(MODE_LABEL);
                egui::ComboBox::from_id_salt("export-mode")
                    .selected_text(c.mode.label())
                    .show_ui(ui, |ui| {
                        for m in Mode::ALL {
                            ui.selectable_value(&mut c.mode, m, m.label());
                        }
                    });
                ui.end_row();

                if choice::shows_sphere_choice(&c, &ctx_info) {
                    ui.label(tr!(N360Video));
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut c.sphere, Sphere::Video360, EXPORT_360);
                        ui.selectable_value(&mut c.sphere, Sphere::Flat, EXPORT_FLAT);
                    });
                    ui.end_row();
                    if let Some(note) = choice::sphere_frame_note(&c, &ctx_info) {
                        ui.add_space(0.0);
                        ui.add(egui::Label::new(egui::RichText::new(note).small()).wrap());
                        ui.end_row();
                    }
                }

                ui.label(VIDEO_LABEL);
                let reason = choice::video_field_reason(&c, &ctx_info);
                ui.add_enabled_ui(reason.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("export-video")
                            .selected_text(video_text(&c))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut c.video, None, SAME_AS_SOURCE);
                                for i in available_presets(&ctx_info.encoders) {
                                    ui.selectable_value(
                                        &mut c.video,
                                        Some(i),
                                        VIDEO_PRESETS[i].label(),
                                    )
                                    .on_hover_text(VIDEO_PRESETS[i].tooltip(&ctx_info.encoders));
                                }
                            });
                        if ui
                            .button(RESET)
                            .on_hover_text(tr!(BackToSameAsSource))
                            .clicked()
                        {
                            c.video = None;
                        }
                    });
                })
                .response
                .on_disabled_hover_text(reason.unwrap_or_default());
                ui.end_row();

                if let Some(w) = over_cap_warning(app, &c) {
                    ui.add_space(ui.spacing().interact_size.y);
                    ui.label(egui::RichText::new(w).color(egui::Color32::from_rgb(230, 160, 40)));
                    ui.end_row();
                }

                ui.label(AUDIO_LABEL);
                let reason = choice::audio_field_reason(&c);
                ui.add_enabled_ui(reason.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("export-audio")
                            .selected_text(audio_text(c.audio))
                            .show_ui(ui, |ui| {
                                for a in [AudioChoice::Same, AudioChoice::Aac, AudioChoice::Mp3] {
                                    if a.available(&ctx_info.encoders) {
                                        ui.selectable_value(&mut c.audio, a, audio_text(a));
                                    }
                                }
                            });
                        if ui
                            .button(RESET)
                            .on_hover_text(tr!(BackToSameAsSource))
                            .clicked()
                        {
                            c.audio = AudioChoice::Same;
                        }
                    });
                })
                .response
                .on_disabled_hover_text(reason.unwrap_or_default());
                ui.end_row();

                if choice::shows_subtitle_choice(&c, &ctx_info) {
                    ui.label(SUBTITLES_LABEL);
                    let fixed = ctx_info.subtitles_as_track;
                    ui.add_enabled_ui(!fixed, |ui| {
                        egui::ComboBox::from_id_salt("export-subtitles")
                            .selected_text(choice::subtitle_mode(&c, &ctx_info).label())
                            .show_ui(ui, |ui| {
                                for m in crate::format::SubtitleMode::ALL {
                                    ui.selectable_value(&mut c.subtitles, m, m.label());
                                }
                            })
                            .response
                            .on_disabled_hover_text(choice::AS_TRACK_REASON);
                    });
                    ui.end_row();
                    if fixed {
                        ui.add_space(0.0);
                        ui.weak(choice::AS_TRACK_REASON);
                        ui.end_row();
                    }
                }
            });
        if choice::shows_subtitle_choice(&c, &ctx_info)
            && let Some(note) = super::effects::subtitle_script_note(app)
        {
            ui.label(super::icons::label(super::icons::WARNING, &note));
        }
        let target = choice::target_path(&c, &ctx_info);
        ui.add(
            egui::Label::new(egui::RichText::new(target.display().to_string()).weak()).truncate(),
        );
        let why = choice::blocked(&c, &ctx_info);
        if let Some(w) = &why {
            ui.colored_label(ui.visuals().warn_fg_color, w);
        }
        ui.horizontal(|ui| {
            let go = ui.add_enabled(why.is_none(), egui::Button::new(EXPORT_BUTTON));
            if go.clicked() {
                cmds.push(Command::StartExport(Box::new(c.clone())));
            }
            if ui.button(CANCEL).clicked() {
                cmds.push(Command::CloseModal);
            }
        });
    });
    if c != *choice {
        cmds.push(Command::SetExportChoice(Box::new(c)));
    }
    if modal.should_close() {
        cmds.push(Command::CloseModal);
    }
}

/// The label of the subtitle row of the Export dialog (REQ-182).
pub const SUBTITLES_LABEL: &str = "Subtitles";

fn audio_text(a: AudioChoice) -> &'static str {
    a.label()
}

/// The question asked before a file is replaced.
pub fn overwrite_dialog(
    ctx: &Context,
    app: &AppState,
    choice: &ExportChoice,
    cmds: &mut Vec<Command>,
) {
    let target = choice::target_path(choice, &app.export_context());
    let modal =
        egui::Modal::new(egui::Id::unique("videolore-export-replace")).show(ctx, |ui: &mut Ui| {
            ui.heading(super::icons::label(super::icons::WARNING, REPLACE_TITLE));
            ui.label(format!("{} already exists.", target.display()));
            ui.horizontal(|ui| {
                if ui.button(REPLACE).clicked() {
                    cmds.push(Command::ConfirmOverwrite);
                }
                if ui.button(CANCEL).clicked() {
                    cmds.push(Command::CloseModal);
                }
            });
        });
    if modal.should_close() {
        cmds.push(Command::CloseModal);
    }
}
