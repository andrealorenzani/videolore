// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The popup of Apply LUT (1.6.0, REQ-135): the chosen file's name (its full path in a
//! tooltip), **Choose file…** and **Remove effect**.

use super::icons;
use super::strings::tr;
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
        ui.label(tr!(ThisClipNoLongerExists));
        return;
    }
    if let FrameTarget::Clip(c) = target
        && let Some(name) = app.clip_name(c)
    {
        ui.add(egui::Label::new(format!("Clip: {name}")).truncate());
    }
    ui.weak("Changes the colours of the picture with a LUT file (.cube, .3dl or .csp).");
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
    if path.is_some() {
        let now = app.project.timeline.lut_strength_of(target);
        ui.horizontal(|ui| {
            ui.label(tr!(Strength));
            let mut v = f32::from(now);
            let r = ui.add(
                egui::Slider::new(&mut v, 0.0..=100.0)
                    .step_by(1.0)
                    .suffix(" %"),
            );
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && v.round() as u8 != now {
                cmds.push(Command::SetLutStrength {
                    target,
                    percent: v.round() as u8,
                });
            }
        });
    }
    favourites(ui, app, target, path, cmds);
}

/// The favourite LUT files (REQ-294): one click applies one, a missing file is marked.
fn favourites(
    ui: &mut Ui,
    app: &AppState,
    target: FrameTarget,
    current: Option<&str>,
    cmds: &mut Vec<Command>,
) {
    if let Some(p) = current.map(std::path::PathBuf::from) {
        let fav = app.settings.favourite_luts.contains(&p);
        let (text, tip) = if fav {
            (tr!(RemoveFromFavourites), tr!(ForgetThisLut))
        } else {
            (tr!(AddToFavourites), tr!(KeepThisLut))
        };
        if ui.button(text).on_hover_text(tip).clicked() {
            cmds.push(Command::SetFavouriteLut(p, !fav));
        }
    }
    if app.settings.favourite_luts.is_empty() {
        return;
    }
    ui.separator();
    ui.strong(tr!(Favourites));
    for p in &app.settings.favourite_luts {
        let name = crate::lut::display_name(&p.to_string_lossy());
        let missing = !p.exists();
        ui.horizontal(|ui| {
            let shown = if missing {
                format!("{name} ({})", tr!(TheFileIsMissing))
            } else {
                name
            };
            let r = ui
                .add_enabled(!missing, egui::Button::new(shown))
                .on_hover_text(crate::media::sanitize_for_display(&p.to_string_lossy()));
            if r.clicked() {
                cmds.push(Command::SetLut {
                    target,
                    path: p.clone(),
                });
            }
            if ui
                .small_button(tr!(CloseMark))
                .on_hover_text(tr!(ForgetThisLut))
                .clicked()
            {
                cmds.push(Command::SetFavouriteLut(p.clone(), false));
            }
        });
    }
}
