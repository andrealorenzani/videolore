// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The media list: imported files with their analysis and proxy status.

use super::button;
use crate::app::{AppState, Command};
use crate::media::{MediaItem, MediaStatus, ProxyState};
use crate::time::timecode;
use eframe::egui::{Button, Color32, Label, RichText, Ui};

/// The line under a file name: what the file is, or why it cannot be used.
pub fn status_line(m: &MediaItem) -> String {
    let base = match &m.status {
        MediaStatus::Analysing => "analysing…".to_string(),
        MediaStatus::Rejected(reason) => format!("cannot be used: {reason}"),
        MediaStatus::Ready if m.missing => "missing".to_string(),
        MediaStatus::Ready => {
            let kind = if !m.has_video() {
                "audio only, "
            } else if m.format.is_360() {
                "360°, "
            } else {
                ""
            };
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

/// "Contains subtitles (Italian, English)" when the file has text subtitle tracks.
pub fn subtitles_note(m: &MediaItem) -> Option<String> {
    let tracks = m
        .format
        .subtitle_tracks
        .as_deref()
        .filter(|t| !t.is_empty())?;
    let mut names: Vec<String> = Vec::new();
    for t in tracks {
        let n = t
            .language
            .as_deref()
            .filter(|l| *l != "und")
            .map(|l| {
                crate::base::language::language_name(l)
                    .map(str::to_string)
                    .unwrap_or_else(|| l.to_string())
            })
            .unwrap_or_else(|| "unknown language".into());
        if !names.contains(&n) {
            names.push(n);
        }
    }
    Some(format!("Contains subtitles ({})", names.join(", ")))
}

/// Shown when the media list is empty.
pub const EMPTY_HINT: &str = "Import media… or drop files here";

/// The list (the section header is drawn by the caller). Names longer than the section
/// are shortened with "…" and shown in full in the tooltip.
pub fn media_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    if app.project.media.is_empty() {
        ui.weak(EMPTY_HINT);
        add_media_button(ui, app, cmds);
        return;
    }
    let w = ui.available_width();
    for m in &app.project.media {
        let line = status_line(m);
        let bad = matches!(m.status, MediaStatus::Rejected(_)) || m.missing;
        let name = if bad {
            RichText::new(&m.name).color(Color32::LIGHT_RED)
        } else {
            RichText::new(&m.name)
        };
        let selected = app.selected_media == Some(m.id);
        let r = ui
            .scope(|ui| {
                ui.set_max_width(w);
                ui.add(Button::selectable(selected, name).truncate())
            })
            .inner;
        ui.add(Label::new(RichText::new(&line).small()).truncate());
        let subs = subtitles_note(m)
            .map(|n| format!("\n{n}"))
            .unwrap_or_default();
        let r = r.on_hover_text(format!(
            "{}\n{line}{subs}\nDouble-click to add it; right-click for more",
            m.name
        ));
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
    add_media_button(ui, app, cmds);
}

/// The label of the button at the end of the Media section (REQ-324).
pub const ADD_MEDIA: &str = "Add media";

/// The **Add media** button at the end of the Media section (REQ-324): the same action as
/// File > Import media…, so the same dialog and the same command (ADR-239).
fn add_media_button(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    button(ui, app, ADD_MEDIA, Command::PickImport, cmds);
}
