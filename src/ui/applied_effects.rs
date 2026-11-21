// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Applied effects widget (REQ-302, REQ-303, REQ-304): a strip under the preview that lists
//! the effects of the selected clip, or of the clip at the playhead. The movable rows come
//! first, the one drawn on top at the top; they move by drag or with Move up and Move down (the
//! keyboard: Alt+Up and Alt+Down on the focused or hovered row). The fixed rows follow, in the order the
//! picture is built. An effect applied to the whole video carries a "whole video" mark.

use super::strings::tr;
use super::{UiCache, scroll_body};
use crate::app::{AppState, AppliedRow, Command, RowMove};
use crate::time::timecode;
use eframe::egui::{self, Button, Key, Modifiers, ScrollArea, Sense, Ui, vec2};

/// The share of the preview section the widget takes.
pub const SHARE: f32 = 0.15;

/// The least height of the widget: its title and one row.
pub const MIN_HEIGHT: f32 = 58.0;

/// The height of the widget in a preview section `available` high.
pub fn height(available: f32) -> f32 {
    (available * SHARE).max(MIN_HEIGHT)
}

/// The label of a row's Move up / Move down button for the screen reader and the tooltip.
fn move_label(up: bool, name: &str) -> String {
    format!("{}: {name}", if up { tr!(MoveUp) } else { tr!(MoveDown) })
}

/// The widget, `height` high.
pub fn show(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    height: f32,
) {
    // the widget takes exactly `height`: its list scrolls inside what is left under the title
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.separator();
        ui.strong(tr!(AppliedEffects));
        let Some(applied) = app.applied() else {
            ui.weak(tr!(SelectAClipToSeeItsEffects));
            return;
        };
        let movable = applied.movable();
        let area = ScrollArea::vertical()
            .id_salt("applied-effects")
            .min_scrolled_height(0.0)
            .max_height(ui.available_height())
            .auto_shrink(false);
        scroll_body(ui, cache, area, |ui, _| {
            for (i, row) in applied.rows.iter().enumerate() {
                one_row(ui, app, applied.clip, (i, movable), row, cmds);
            }
        });
    });
}

fn one_row(
    ui: &mut Ui,
    app: &AppState,
    clip: crate::timeline::ClipId,
    (i, movable): (usize, usize),
    row: &AppliedRow,
    cmds: &mut Vec<Command>,
) {
    ui.horizontal(|ui| {
        let mut text = row.name.clone();
        if let Some((a, b)) = row.range {
            text.push_str(&format!("  {}-{}", timecode(a), timecode(b)));
        }
        // a row that can be moved is dragged by its name
        let sense = if row.row.is_some() {
            Sense::click_and_drag()
        } else {
            Sense::click()
        };
        let r = ui
            .add(Button::new(text).sense(sense))
            .on_hover_text(tr!(AppliedRowHint));
        // the screen reader says "Applied effect: Volume", not the name of the Effects list
        let spoken = format!("{} {}", tr!(AppliedEffectLabel), row.name);
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, spoken.clone()));
        if r.clicked() {
            cmds.extend(row.open.iter().cloned());
        }
        if row.whole {
            ui.weak(tr!(WholeVideoMark));
        }
        let Some(kind) = row.row else { return };
        if r.drag_stopped()
            && let Some(p) = r.interact_pointer_pos()
        {
            // the row ends where the pointer is, counted in rows of this height
            let step = (r.rect.height() + ui.spacing().item_spacing.y).max(1.0);
            let delta = ((p.y - r.rect.center().y) / step).round() as i64;
            let to = (i as i64 + delta).clamp(0, movable as i64 - 1) as usize;
            if to != i {
                cmds.push(Command::MoveRow {
                    clip,
                    row: kind,
                    mv: RowMove::To(to),
                });
            }
        }
        let ctx = ui.ctx().clone();
        let active = r.has_focus() || r.hovered();
        let alt = |k: Key| active && ctx.input_mut(|inp| inp.consume_key(Modifiers::ALT, k));
        let mut moves = Vec::new();
        for (up, glyph) in [(true, "\u{25B2}"), (false, "\u{25BC}")] {
            let mv = if up { RowMove::Up } else { RowMove::Down };
            let cmd = Command::MoveRow {
                clip,
                row: kind,
                mv,
            };
            let reason = app.disabled_reason(&cmd);
            let label = move_label(up, &row.name);
            let b = ui
                .add_enabled(reason.is_none(), Button::new(glyph).small())
                .on_hover_text(if up { tr!(MoveUp) } else { tr!(MoveDown) })
                .on_disabled_hover_text(reason.unwrap_or_default());
            b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
            if b.clicked() || alt(if up { Key::ArrowUp } else { Key::ArrowDown }) {
                moves.push(cmd);
            }
        }
        // a keyboard move of a row that cannot move that way is ignored
        cmds.extend(moves.into_iter().filter(|c| app.is_enabled(c)));
    });
}
