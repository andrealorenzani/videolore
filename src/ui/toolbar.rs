// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The toolbar (REQ-070): the 12 most common commands in one row, then (separated) the
//! group of effect commands (REQ-098/099). When the window is too narrow, the buttons that
//! do not fit go into a "»" menu at the right end, the effect group first and as a whole.

use super::actions::ActionId;
use super::{action_tooltip, menu};
use crate::app::{AppState, Command};
use eframe::egui::{Button, TextStyle, TextWrapMode, Ui, WidgetText};

/// The toolbar's actions, left to right, before the Context Menu group.
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

/// Index of the first button of the Context Menu group (drawn after a separator).
pub const EFFECT_GROUP: usize = TOOLBAR.len();

/// The Context Menu (REQ-106): the commands that act on what is selected or under the
/// playhead. Empty (so the group is not drawn at all) when there is nothing to act on:
/// Align and Reset with a subtitle line selected, Set POV and Reset with the playhead
/// over a 360 clip. A selected text item shows Align and Reset like a line (1.4.0).
pub fn context_group(app: &AppState) -> Vec<ActionId> {
    context_group_of(
        !app.selection.is_empty(),
        app.sphere_at_playhead().is_some(),
    )
}

/// [`context_group`] from its two inputs.
pub fn context_group_of(line_selected: bool, on_360: bool) -> Vec<ActionId> {
    let mut v = Vec::new();
    if line_selected {
        v.extend([ActionId::AlignHorizontal, ActionId::AlignVertical]);
    }
    if on_360 {
        v.push(ActionId::SetPov);
    }
    if line_selected || on_360 {
        v.push(ActionId::ResetObjects);
    }
    v
}

/// Every button of the toolbar now: the fixed ones, then the Context Menu group.
pub fn actions(app: &AppState) -> Vec<ActionId> {
    let mut v = TOOLBAR.to_vec();
    v.extend(context_group(app));
    v
}

/// Label of a toolbar button (shorter than the menu's).
pub fn label(app: &AppState, id: ActionId) -> String {
    match id {
        ActionId::Import => "Import…".into(),
        ActionId::AddToTimeline => "Add to timeline".into(),
        ActionId::AddMusic => "Add music".into(),
        ActionId::Split => "Split".into(),
        ActionId::Undo => "Undo".into(),
        ActionId::Redo => "Redo".into(),
        ActionId::AlignHorizontal => "Align horizontally".into(),
        ActionId::AlignVertical => "Align vertically".into(),
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

/// The Context Menu group moves to the "»" menu as a whole: part of it is never left in
/// the row.
pub fn visible(fitting: usize, total: usize) -> usize {
    if fitting > EFFECT_GROUP && fitting < total {
        EFFECT_GROUP
    } else {
        fitting
    }
}

pub fn toolbar(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let all = actions(app);
    let widths: Vec<f32> = all
        .iter()
        .map(|id| button_width(ui, &label(app, *id)))
        .collect();
    let n = visible(
        fit(&widths, ui.available_width(), button_width(ui, "»")),
        all.len(),
    );
    ui.horizontal(|ui| {
        for (i, id) in all[..n].iter().enumerate() {
            if i == EFFECT_GROUP {
                ui.separator();
            }
            tool_button(ui, app, *id, cmds);
        }
        if n < all.len() {
            ui.menu_button("»", |ui| {
                for id in &all[n..] {
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
        // the effect group is kept together
        assert_eq!(visible(15, 15), 15);
        assert_eq!(visible(14, 15), EFFECT_GROUP);
        assert_eq!(visible(13, 15), EFFECT_GROUP);
        assert_eq!(visible(12, 15), 12);
        assert_eq!(visible(5, 15), 5);
        assert_eq!(visible(12, 12), 12);
    }

    // covers: REQ-106
    #[test]
    fn the_context_group_depends_on_the_selection_and_the_playhead() {
        use ActionId::*;
        assert!(context_group_of(false, false).is_empty());
        assert_eq!(
            context_group_of(true, false),
            [AlignHorizontal, AlignVertical, ResetObjects]
        );
        assert_eq!(context_group_of(false, true), [SetPov, ResetObjects]);
        assert_eq!(
            context_group_of(true, true),
            [AlignHorizontal, AlignVertical, SetPov, ResetObjects]
        );
    }
}
