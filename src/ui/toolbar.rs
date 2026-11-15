// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The toolbar (REQ-070): the 12 most common commands in one row. When the window is too
//! narrow, the buttons that do not fit go into a "»" menu at the right end.

use super::actions::ActionId;
use super::{action_tooltip, menu};
use crate::app::{AppState, Command};
use eframe::egui::{Button, TextStyle, TextWrapMode, Ui, WidgetText};

/// The toolbar's actions, left to right.
pub const TOOLBAR: [ActionId; 12] = [
    ActionId::Import,
    ActionId::AddToTimeline,
    ActionId::AddMusic,
    ActionId::Split,
    ActionId::Cut,
    ActionId::Copy,
    ActionId::Paste,
    ActionId::Delete,
    ActionId::Undo,
    ActionId::Redo,
    ActionId::PlayPause,
    ActionId::Export,
];

/// Label of a toolbar button (shorter than the menu's).
pub fn label(app: &AppState, id: ActionId) -> String {
    match id {
        ActionId::Import => "Import…".into(),
        ActionId::AddToTimeline => "Add to timeline".into(),
        ActionId::AddMusic => "Add music".into(),
        ActionId::Split => "Split".into(),
        ActionId::Undo => "Undo".into(),
        ActionId::Redo => "Redo".into(),
        ActionId::PlayPause if app.preview.is_playing() => "Pause".into(),
        ActionId::PlayPause => "Play".into(),
        _ => id.label(app),
    }
}

/// How many buttons (of widths `widths`, in order) fit in `avail` pixels, keeping room for
/// the "»" button (`overflow_w`) when not all of them fit.
pub fn fit(widths: &[f32], avail: f32, overflow_w: f32) -> usize {
    if widths.iter().sum::<f32>() <= avail {
        return widths.len();
    }
    let room = avail - overflow_w;
    let mut used = 0.0;
    for (i, w) in widths.iter().enumerate() {
        used += w;
        if used > room {
            return i;
        }
    }
    widths.len()
}

fn button_width(ui: &Ui, text: &str) -> f32 {
    let galley = WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Button,
    );
    galley.size().x + 2.0 * ui.spacing().button_padding.x + ui.spacing().item_spacing.x
}

fn tool_button(ui: &mut Ui, app: &AppState, id: ActionId, cmds: &mut Vec<Command>) {
    let cmd = id.command(app);
    let reason = app.disabled_reason(&cmd);
    let r = ui
        .add_enabled(reason.is_none(), Button::new(label(app, id)))
        .on_hover_text(action_tooltip(ui.ctx(), app, id))
        .on_disabled_hover_text(reason.unwrap_or_default());
    if r.clicked() {
        cmds.push(cmd);
    }
}

pub fn toolbar(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let widths: Vec<f32> = TOOLBAR
        .iter()
        .map(|id| button_width(ui, &label(app, *id)))
        .collect();
    let n = fit(&widths, ui.available_width(), button_width(ui, "»"));
    ui.horizontal(|ui| {
        for id in &TOOLBAR[..n] {
            tool_button(ui, app, *id, cmds);
        }
        if n < TOOLBAR.len() {
            ui.menu_button("»", |ui| {
                for id in &TOOLBAR[n..] {
                    menu::action_item(ui, app, *id, cmds);
                }
            })
            .response
            .on_hover_text("More commands");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-070
    #[test]
    fn buttons_that_do_not_fit_go_to_the_overflow_menu() {
        let w = [50.0; 12];
        assert_eq!(fit(&w, 600.0, 20.0), 12);
        assert_eq!(fit(&w, 599.0, 20.0), 11);
        assert_eq!(fit(&w, 300.0, 20.0), 5);
        assert_eq!(fit(&w, 10.0, 20.0), 0);
        assert_eq!(fit(&[], 10.0, 20.0), 0);
    }
}
