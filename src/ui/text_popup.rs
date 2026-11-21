// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The popup of a new text and of an existing text item (REQ-108, REQ-109, REQ-110).
//! A new text is filled in and created by "Add"; an existing one changes as you edit it,
//! one undo step per change.

use super::UiCache;
use super::strings::tr;
use crate::app::{AppState, Command, Popup};
use crate::text::{
    EFFECT_DEFAULT, EFFECT_MAX, EFFECT_MIN, MIN_TTL, Readability, SlideDir, TextId, TextItem,
};
use crate::time::Micros;
use eframe::egui::{self, Ui};

/// The note under the font choice when its font is not installed (REQ-164).
pub const FONT_MISSING: &str = "Font not installed: the bundled font is used";

/// The label of the "Add" button.
pub const ADD: &str = "Add";

fn secs(m: Micros) -> f64 {
    m.as_secs_f64()
}

/// A time setting in seconds; true when the user finished changing it.
fn time_value(ui: &mut Ui, v: &mut Micros, min: Micros, max: Micros) -> bool {
    let mut s = secs(*v);
    let r = ui.add(
        egui::DragValue::new(&mut s)
            .range(secs(min)..=secs(max))
            .speed(0.05)
            .max_decimals(2)
            .suffix(" s"),
    );
    *v = Micros::from_secs_f64(s).clamp(min, max);
    r.drag_stopped() || r.lost_focus() || (r.changed() && !r.dragged() && !r.has_focus())
}

/// One effect row: a checkbox and its setting. Returns true when something was committed.
fn toggle_time(ui: &mut Ui, label: &str, v: &mut Option<Micros>) -> bool {
    let mut on = v.is_some();
    let mut done = false;
    ui.horizontal(|ui| {
        if ui.checkbox(&mut on, label).changed() {
            *v = on.then_some(EFFECT_DEFAULT);
            done = true;
        }
        if let Some(t) = v {
            ui.label(tr!(For));
            done |= time_value(ui, t, EFFECT_MIN, EFFECT_MAX);
        }
    });
    done
}

fn slide_row(ui: &mut Ui, v: &mut Option<(SlideDir, Micros)>) -> bool {
    let mut on = v.is_some();
    let mut done = false;
    ui.horizontal(|ui| {
        if ui.checkbox(&mut on, "Slide in").changed() {
            *v = on.then_some((SlideDir::Left, EFFECT_DEFAULT));
            done = true;
        }
        if let Some((dir, t)) = v {
            egui::ComboBox::from_id_salt("text-slide-dir")
                .selected_text(dir.name())
                .show_ui(ui, |ui| {
                    for d in SlideDir::ALL {
                        done |= ui.selectable_value(dir, d, d.name()).changed();
                    }
                });
            ui.label(tr!(In));
            done |= time_value(ui, t, EFFECT_MIN, EFFECT_MAX);
        }
    });
    done
}

/// The fields common to a new text and an existing one. True when a change is ready to be
/// stored.
fn fields(ui: &mut Ui, app: &AppState, item: &mut TextItem, buffer: &mut String) -> bool {
    let mut done = false;
    ui.label(tr!(TextEnterStartsANew));
    let r = ui.add(
        egui::TextEdit::multiline(buffer)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    item.text = buffer.clone();
    done |= r.lost_focus();
    ui.separator();
    ui.strong(tr!(Effects));
    done |= toggle_time(ui, "Fade IN", &mut item.effects.fade_in);
    done |= toggle_time(ui, "Fade OUT", &mut item.effects.fade_out);
    done |= toggle_time(ui, "Blink every", &mut item.effects.blink);
    done |= slide_row(ui, &mut item.effects.slide);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(tr!(TimeToLive));
        let max = app.project.timeline.duration().max(MIN_TTL);
        done |= time_value(ui, &mut item.ttl, MIN_TTL, max);
    });
    ui.horizontal(|ui| {
        ui.label(tr!(Colour));
        let before = item.colour;
        let r = egui::color_picker::color_edit_button_srgb(ui, &mut item.colour);
        if item.colour != before && !ui.input(|i| i.pointer.any_down()) {
            done = true;
        }
        done |= r.clicked_elsewhere() && item.colour != before;
    });
    ui.horizontal(|ui| {
        ui.label(tr!(Font));
        let shown = if item.font.is_empty() {
            crate::text::DEFAULT_FONT.to_string()
        } else {
            item.font.clone()
        };
        egui::ComboBox::from_id_salt("text-font")
            .selected_text(shown)
            .width(220.0)
            .show_ui(ui, |ui| match app.fonts.families() {
                None => {
                    ui.label(tr!(LoadingFonts));
                }
                Some(list) => {
                    for f in list {
                        let name = if f == crate::text::DEFAULT_FONT {
                            String::new()
                        } else {
                            f.clone()
                        };
                        if ui.selectable_label(item.font == name, &f).clicked() {
                            item.font = name;
                            done = true;
                        }
                    }
                }
            });
    });
    if !app.fonts.is_installed(&item.font) {
        ui.weak(FONT_MISSING);
    }
    ui.horizontal(|ui| {
        ui.label(tr!(Readability));
        egui::ComboBox::from_id_salt("text-readability")
            .selected_text(item.readability.name())
            .show_ui(ui, |ui| {
                for r in Readability::ALL {
                    done |= ui
                        .selectable_value(&mut item.readability, r, r.name())
                        .changed();
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label(tr!(ShadowLabel));
        let r = ui.add(
            egui::DragValue::new(&mut item.shadow.px)
                .range(0..=crate::base::shadow::SHADOW_MAX)
                .suffix(" px"),
        );
        done |= r.drag_stopped() || (r.changed() && !r.dragged());
        let before = item.shadow.colour;
        let c = egui::color_picker::color_edit_button_srgb(ui, &mut item.shadow.colour);
        done |= item.shadow.colour != before && !ui.input(|i| i.pointer.any_down())
            || c.clicked_elsewhere() && item.shadow.colour != before;
    });
    done
}

/// Body of [`Popup::Text`].
pub fn text_body(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    match app.popups.text {
        None => new_text(ui, app, cache, cmds),
        Some(id) => existing(ui, app, cache, id, cmds),
    }
}

fn new_text(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let (draft, buffer) = cache.text_draft.get_or_insert_with(|| {
        (
            TextItem::new(TextId(0), String::new(), Micros::ZERO),
            String::new(),
        )
    });
    ui.strong(tr!(NewText));
    ui.weak(format!(
        "It starts at the playhead ({}).",
        crate::time::timecode(app.playhead)
    ));
    fields(ui, app, draft, buffer);
    ui.separator();
    ui.horizontal(|ui| {
        let cmd = Command::AddText(draft.clone());
        let reason = app.disabled_reason(&cmd);
        let r = ui.add_enabled(reason.is_none(), egui::Button::new(ADD));
        let r = match reason {
            Some(why) => r.on_disabled_hover_text(why),
            None => r,
        };
        if r.clicked() {
            cmds.push(cmd);
            cmds.push(Command::ClosePopup(Popup::Text));
        }
        if ui.button(tr!(Close)).clicked() {
            cmds.push(Command::ClosePopup(Popup::Text));
        }
    });
}

fn existing(ui: &mut Ui, app: &AppState, cache: &mut UiCache, id: TextId, cmds: &mut Vec<Command>) {
    let Some(stored) = app.project.texts.get(id) else {
        ui.label(tr!(ThisTextNoLongerExists));
        return;
    };
    if let Some(pic) = &stored.picture {
        picture_body(ui, app, stored, &pic.name(), cmds);
        return;
    }
    ui.strong(tr!(TextWord));
    ui.label(format!(
        "{} – {}",
        crate::time::timecode(stored.start),
        crate::time::timecode(stored.end())
    ));
    let mut item = stored.clone();
    let mut buffer = match &cache.text_edit {
        Some((x, t)) if *x == id => t.clone(),
        _ => stored.text.clone(),
    };
    let done = fields(ui, app, &mut item, &mut buffer);
    // the text is kept while it is being typed; the stored one shows otherwise
    let typing = ui.memory(|m| m.focused().is_some());
    cache.text_edit = typing.then(|| (id, buffer.clone()));
    if done {
        cmds.push(Command::EditText(item));
    }
    ui.separator();
    let del = Command::DeleteTexts(vec![id]);
    if ui.button(tr!(Delete)).clicked() {
        cmds.push(del);
    }
}

/// The popup of a picture item (REQ-287): its file, when it shows, and Delete. Its place and
/// size are set on the picture in the preview, as for a text.
fn picture_body(
    ui: &mut Ui,
    app: &AppState,
    stored: &TextItem,
    name: &str,
    cmds: &mut Vec<Command>,
) {
    ui.strong(format!("Picture {name}"));
    ui.label(format!(
        "{} – {}",
        crate::time::timecode(stored.start),
        crate::time::timecode(stored.end())
    ));
    let mut item = stored.clone();
    ui.horizontal(|ui| {
        ui.label(tr!(TimeToLive));
        let max = app.project.timeline.duration().max(MIN_TTL);
        if time_value(ui, &mut item.ttl, MIN_TTL, max) {
            cmds.push(Command::EditText(item.clone()));
        }
    });
    ui.separator();
    if ui.button(tr!(Delete)).clicked() {
        cmds.push(Command::DeleteTexts(vec![stored.id]));
    }
}
