// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The menu bar (REQ-069): File, Edit, Playback, Effects, View, Help. Every item comes from
//! the action table, shows its shortcut and is greyed out exactly when its command cannot
//! run (with the reason as tooltip). A menu taller than the window scrolls.

use super::actions::ActionId;
use super::{UiCache, keymap, scroll_body};
use crate::app::{AppState, Command};
use eframe::egui::{self, Button, ScrollArea, Ui};

/// One entry of a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Action(ActionId),
    /// File > Open recent ▸
    OpenRecent,
    /// A submenu of actions ("Apply to selected clip ▸").
    Submenu(&'static str, &'static [ActionId]),
    Separator,
}

/// Effects > Apply to selected clip ▸
pub const APPLY_TO_CLIP: &[ActionId] = &[
    ActionId::ApplyClipFade,
    ActionId::ApplyClipSubtitles,
    ActionId::ApplyClipVolume,
];
/// Effects > Apply to whole video ▸
pub const APPLY_TO_VIDEO: &[ActionId] = &[
    ActionId::ApplyWholeFade,
    ActionId::ApplyWholeSubtitles,
    ActionId::ApplyWholeVolume,
];

/// The menus and their items, in order.
pub fn menus() -> Vec<(&'static str, Vec<Item>)> {
    use ActionId::*;
    use Item::{Action as A, OpenRecent, Separator as S, Submenu};
    vec![
        (
            "File",
            vec![
                A(New),
                A(Open),
                OpenRecent,
                S,
                A(Save),
                A(SaveAs),
                S,
                A(Import),
                A(Export),
                A(ExportSubtitles),
                S,
                A(Quit),
            ],
        ),
        (
            "Edit",
            vec![
                A(Undo),
                A(Redo),
                S,
                A(Cut),
                A(Copy),
                A(Paste),
                A(Delete),
                S,
                A(Split),
                A(Merge),
                A(MoveLeft),
                A(MoveRight),
                S,
                A(AddToTimeline),
                A(AddMusic),
                A(ClipDetails),
                S,
                A(Settings),
            ],
        ),
        (
            "Playback",
            vec![
                A(PlayPause),
                A(GoStart),
                A(GoEnd),
                A(PrevFrame),
                A(NextFrame),
            ],
        ),
        (
            "Effects",
            vec![
                A(Fades),
                A(Subtitles),
                A(Volume),
                S,
                Submenu("Apply to selected clip", APPLY_TO_CLIP),
                Submenu("Apply to whole video", APPLY_TO_VIDEO),
                S,
                A(ToggleFadeAtPlayhead),
                A(GenerateSubtitles),
                A(AddSubtitleLine),
                A(ResetSubtitlePosition),
            ],
        ),
        (
            "View",
            vec![A(ZoomIn), A(ZoomOut), A(ZoomFit), S, A(ResetLayout)],
        ),
        ("Help", vec![A(Shortcuts), A(About)]),
    ]
}

/// One action as a menu item.
pub fn action_item(ui: &mut Ui, app: &AppState, id: ActionId, cmds: &mut Vec<Command>) {
    let cmd = id.command(app);
    let reason = app.disabled_reason(&cmd);
    let mut b = Button::new(id.label(app));
    if let Some(s) = keymap::shortcut_text(ui.ctx(), id) {
        b = b.shortcut_text(s);
    }
    let r = ui
        .add_enabled(reason.is_none(), b)
        .on_hover_text(id.tooltip(app))
        .on_disabled_hover_text(reason.unwrap_or_default());
    if r.clicked() {
        cmds.push(cmd);
        ui.close();
    }
}

/// The name shown for a recent project: its file name, safe to display.
pub fn recent_label(p: &std::path::Path) -> String {
    crate::media::display_name(p)
}

fn recent_menu(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    if app.settings.recent.is_empty() {
        ui.add_enabled(false, Button::new("No recent projects"));
    }
    for p in &app.settings.recent {
        let full = crate::media::sanitize_for_display(&p.to_string_lossy());
        if ui
            .add(Button::new(recent_label(p)).truncate())
            .on_hover_text(full)
            .clicked()
        {
            cmds.push(Command::OpenRecent(p.clone()));
            ui.close();
        }
    }
    ui.separator();
    let clear = Command::ClearRecent;
    if ui
        .add_enabled(app.is_enabled(&clear), Button::new("Clear list"))
        .clicked()
    {
        cmds.push(clear);
        ui.close();
    }
}

pub fn menu_bar(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    egui::MenuBar::new().ui(ui, |ui| {
        for (title, items) in menus() {
            ui.menu_button(title, |ui| {
                // a menu taller than the window scrolls
                let max_h = (ui.ctx().content_rect().height() - 40.0).max(80.0);
                let area = ScrollArea::vertical()
                    .id_salt(("menu", title))
                    .max_height(max_h);
                scroll_body(ui, cache, area, |ui, _| {
                    for item in items {
                        match item {
                            Item::Action(id) => action_item(ui, app, id, cmds),
                            Item::OpenRecent => {
                                ui.menu_button("Open recent", |ui| recent_menu(ui, app, cmds));
                            }
                            Item::Submenu(name, ids) => {
                                ui.menu_button(name, |ui| {
                                    for id in ids {
                                        action_item(ui, app, *id, cmds);
                                    }
                                });
                            }
                            Item::Separator => {
                                ui.separator();
                            }
                        }
                    }
                });
            });
        }
    });
}
