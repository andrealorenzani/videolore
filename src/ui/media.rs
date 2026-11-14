// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The media list: imported files with their analysis and proxy status.

use super::button;
use crate::app::{AppState, Command};
use crate::media::{MediaItem, MediaStatus, ProxyState};
use crate::time::timecode;
use eframe::egui::{self, Button, Color32, Ui};

/// The line under a file name: what the file is, or why it cannot be used.
pub fn status_line(m: &MediaItem) -> String {
    let base = match &m.status {
        MediaStatus::Analysing => "analysing…".to_string(),
        MediaStatus::Rejected(reason) => format!("cannot be used: {reason}"),
        MediaStatus::Ready if m.missing => "missing".to_string(),
        MediaStatus::Ready => {
            let kind = if m.has_video() { "" } else { "audio only, " };
            format!("{kind}{}  {}", timecode(m.duration), m.format.describe())
        }
    };
    let proxy = match &m.proxy {
        ProxyState::None => String::new(),
        ProxyState::Queued => "  · preview copy queued".into(),
        ProxyState::Generating(p) => format!("  · preview copy {p}%"),
        ProxyState::Ready(_) => "  · preview copy ready".into(),
        ProxyState::Failed(_) => "  · preview copy failed (using the original)".into(),
    };
    base + &proxy
}

pub fn media_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    ui.horizontal(|ui| {
        ui.strong("Media");
        button(ui, app, "Import…", Command::PickImport, cmds);
        button(ui, app, "Add to timeline", Command::Append(None), cmds);
        button(ui, app, "Add music", Command::PlaceMusic(None), cmds);
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        for m in &app.project.media {
            let line = status_line(m);
            let label = format!("{}\n{line}", m.name);
            let r = if matches!(m.status, MediaStatus::Rejected(_)) || m.missing {
                ui.add(egui::Label::new(
                    egui::RichText::new(label).color(Color32::LIGHT_RED),
                ))
                .interact(egui::Sense::click())
            } else {
                ui.selectable_label(app.selected_media == Some(m.id), label)
            };
            if r.clicked() {
                cmds.push(Command::SelectMedia(Some(m.id)));
            }
            if r.double_clicked() {
                cmds.push(if m.has_video() {
                    Command::Append(Some(m.id))
                } else {
                    Command::PlaceMusic(Some(m.id))
                });
            }
            r.context_menu(|ui| {
                for (label, cmd) in [
                    ("Add to timeline", Command::Append(Some(m.id))),
                    ("Add to music track", Command::PlaceMusic(Some(m.id))),
                    ("Remove from list", Command::RemoveMedia(m.id)),
                ] {
                    if ui
                        .add_enabled(app.is_enabled(&cmd), Button::new(label))
                        .clicked()
                    {
                        cmds.push(cmd);
                        ui.close();
                    }
                }
            });
        }
    });
}
