// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The popup of Apply LUT (1.6.0, REQ-135): the chosen file's name (its full path in a
//! tooltip), **Choose file…** and **Remove effect**.

use super::icons;
use crate::app::{AppState, Command};
use crate::timeline::FrameTarget;
use eframe::egui::{self, Ui};

/// What the popup says when no file is chosen.
pub const NO_FILE: &str = "No LUT file chosen";

/// The text of the file line: the file name, or [`NO_FILE`].
pub fn file_text(app: &AppState, target: FrameTarget) -> String {
    match app.project.timeline.lut_of(target) {
        Some(p) => crate::lut::display_name(p),
        None => NO_FILE.to_string(),
    }
}

/// The body of the Apply LUT popup of `target`.
pub fn lut_body(ui: &mut Ui, app: &AppState, target: FrameTarget, cmds: &mut Vec<Command>) {
    if let FrameTarget::Clip(c) = target
        && app.project.timeline.get(c).is_none()
    {
        ui.label("This clip no longer exists.");
        return;
    }
    if let FrameTarget::Clip(c) = target
        && let Some(name) = app.clip_name(c)
    {
        ui.add(egui::Label::new(format!("Clip: {name}")).truncate());
    }
    ui.weak("Changes the colours of the picture with a .cube LUT file.");
    let path = app.project.timeline.lut_of(target);
    let mut line = ui.add(egui::Label::new(file_text(app, target)).truncate());
    if let Some(p) = path {
        line = line.on_hover_text(crate::media::sanitize_for_display(p));
        if !std::path::Path::new(p).exists() {
            ui.colored_label(ui.visuals().warn_fg_color, "The file is missing");
        }
    }
    let _ = line;
    ui.horizontal(|ui| {
        super::button(
            ui,
            app,
            &icons::label(icons::FOLDER_OPEN, "Choose file…"),
            Command::PickLut(target),
            cmds,
        );
        super::button(
            ui,
            app,
            &icons::label(icons::TRASH, "Remove effect"),
            Command::RemoveLut(target),
            cmds,
        );
    });
}
