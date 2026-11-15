// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The status bar (REQ-073): the latest message on the left, background work with
//! progress (and Cancel where it can be stopped) in the middle, the output format on the
//! right.

use crate::app::{AppState, Command};
use eframe::egui::{Align, Button, Label, Layout, ProgressBar, Ui};

/// The message on the left (the ffmpeg check while it runs).
pub fn message(app: &AppState) -> &str {
    if app.ffmpeg_checking {
        crate::app::CHECKING_FFMPEG
    } else {
        app.status.as_str()
    }
}

/// The text on the right.
pub fn output_text(app: &AppState) -> String {
    match app.output_description() {
        Some(d) => format!("Output: {d}"),
        None => "Output: add clips to see the format".into(),
    }
}

pub fn status_bar(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let jobs = app.background_jobs();
    let output = output_text(app);
    ui.horizontal(|ui| {
        let w = ui.available_width();
        let msg = message(app);
        ui.scope(|ui| {
            ui.set_max_width((w * 0.3).max(60.0));
            ui.add(Label::new(msg).truncate()).on_hover_text(msg);
        });
        for (i, j) in jobs.iter().enumerate() {
            ui.separator();
            ui.add(Label::new(&j.label).truncate());
            if let Some(p) = j.progress {
                ui.add(ProgressBar::new(p).desired_width(80.0).show_percentage());
            }
            if let Some(c) = &j.cancel
                && ui
                    .push_id(i, |ui| ui.add(Button::new("Cancel")))
                    .inner
                    .on_hover_text(format!("Stop: {}", j.label))
                    .clicked()
            {
                cmds.push(c.clone());
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(Label::new(&output).truncate())
                .on_hover_text(&output);
        });
    });
}
