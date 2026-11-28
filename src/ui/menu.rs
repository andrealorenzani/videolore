// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The menu bar (REQ-069): File, Edit, Playback, Effects, View, Help. Every item comes from
//! the action table, shows its shortcut and is greyed out exactly when its command cannot
//! run (with the reason as tooltip). A menu taller than the window scrolls.

use super::actions::ActionId;
use super::icons;
use super::strings::tr;
use super::{UiCache, keymap, scroll_body};
use crate::app::{AppState, Command};
use eframe::egui::{self, Button, ScrollArea, Ui};

/// One entry of a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Action(ActionId),
    /// File > Open recent ▸
    OpenRecent,
    /// View > Sections ▸, Workspaces ▸, Theme ▸, Customise toolbar ▸ (see [`super::view_menu`]).
    Sections,
    Workspaces,
    ThemeMenu,
    ToolbarMenu,
    /// A submenu of actions ("Apply to selected clip ▸").
    Submenu(&'static str, &'static [ActionId]),
    Separator,
}

/// Effects > Apply to selected clip ▸
pub const APPLY_TO_CLIP: &[ActionId] = &[
    ActionId::ApplyClipFade,
    ActionId::ApplyClipSubtitles,
    ActionId::ApplyClipVolume,
    ActionId::ApplyClipCleanAudio,
    ActionId::ApplyClipScale,
    ActionId::ApplyClipMove,
    ActionId::ApplyClipLut,
];
/// Effects > Apply to whole video ▸
pub const APPLY_TO_VIDEO: &[ActionId] = &[
    ActionId::ApplyWholeFade,
    ActionId::ApplyWholeSubtitles,
    ActionId::ApplyWholeVolume,
    ActionId::ApplyWholeCleanAudio,
    ActionId::ApplyWholeChangeAspect,
    ActionId::ApplyWholeScale,
    ActionId::ApplyWholeMove,
    ActionId::ApplyWholeLut,
];

/// Edit > Clipboard ▸
pub const CLIPBOARD: &[ActionId] = &[
    ActionId::Cut,
    ActionId::Copy,
    ActionId::Paste,
    ActionId::Delete,
];
/// Edit > History ▸
pub const HISTORY: &[ActionId] = &[ActionId::Undo, ActionId::Redo];
/// Edit > Clip ▸
pub const CLIP: &[ActionId] = &[
    ActionId::Split,
    ActionId::Merge,
    ActionId::MoveLeft,
    ActionId::MoveRight,
    ActionId::AddToTimeline,
    ActionId::AddMusic,
    ActionId::ClipDetails,
    ActionId::ToggleFadeAtPlayhead,
];
/// Edit > Subtitles ▸
pub const SUBTITLES: &[ActionId] = &[
    ActionId::GenerateSubtitles,
    ActionId::AddSubtitleLine,
    ActionId::ShiftLinesEarlier,
    ActionId::ShiftLinesLater,
    ActionId::AlignHorizontal,
    ActionId::AlignVertical,
    ActionId::MoveInside,
    ActionId::ReAnchor,
    ActionId::ResetObjects,
];

/// The menus and their items, in order.
pub fn menus() -> Vec<(&'static str, Vec<Item>)> {
    use ActionId::*;
    use Item::{
        Action as A, OpenRecent, Sections, Separator as S, Submenu, ThemeMenu, ToolbarMenu,
        Workspaces,
    };
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
                A(ImportProject),
                A(Export),
                A(ImportSubtitles),
                A(ExportSubtitles),
                A(OpenInPlayer),
                S,
                A(Quit),
            ],
        ),
        (
            "Edit",
            vec![
                Submenu("Clipboard", CLIPBOARD),
                Submenu("History", HISTORY),
                Submenu("Clip", CLIP),
                Submenu("Subtitles", SUBTITLES),
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
                A(PrevMarker),
                A(NextMarker),
                S,
                A(SetPov),
            ],
        ),
        (
            "Effects",
            vec![
                A(Fades),
                A(Subtitles),
                A(Volume),
                A(CleanAudio),
                A(ChangeAspect),
                A(Scale),
                A(Move),
                A(ApplyLut),
                A(AddText),
                A(AddOverlap),
                S,
                Submenu("Apply to selected clip", APPLY_TO_CLIP),
                Submenu("Apply to whole video", APPLY_TO_VIDEO),
            ],
        ),
        (
            "View",
            vec![
                A(ZoomIn),
                A(ZoomOut),
                A(ZoomFit),
                S,
                A(ToggleBoxes),
                A(ToggleSnap),
                A(ToggleSafeAreas),
                S,
                Sections,
                Workspaces,
                A(ResetLayout),
                S,
                ThemeMenu,
                ToolbarMenu,
            ],
        ),
        ("Help", vec![A(Shortcuts), A(About)]),
    ]
}

/// One action as a menu item.
pub fn action_item(ui: &mut Ui, app: &AppState, id: ActionId, cmds: &mut Vec<Command>) {
    let cmd = id.command(app);
    let reason = app.disabled_reason(&cmd);
    let mut b = Button::new(icons::label(id.icon(), &id.label(app)));
    if let Some(s) = keymap::shortcut_text(ui.ctx(), id) {
        b = b.shortcut_text(s);
    } else if let Some(on) = match id {
        ActionId::ToggleBoxes => Some(app.settings.show_boxes),
        ActionId::ToggleSnap => Some(app.settings.snap_to_guides),
        ActionId::ToggleSafeAreas => Some(app.settings.show_safe_areas),
        _ => None,
    } {
        b = b.shortcut_text(if on { "On" } else { "Off" });
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
        ui.add_enabled(false, Button::new(tr!(NoRecentProjects)));
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
        .add_enabled(app.is_enabled(&clear), Button::new(tr!(ClearList)))
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
                                ui.menu_button(tr!(OpenRecent), |ui| recent_menu(ui, app, cmds));
                            }
                            Item::Sections => super::view_menu::sections(ui, app, cmds),
                            Item::Workspaces => super::view_menu::workspaces(ui, app, cmds),
                            Item::ThemeMenu => super::view_menu::theme(ui, app, cmds),
                            Item::ToolbarMenu => super::view_menu::toolbar(ui, app, cmds),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn items_of(menu: &str) -> Vec<Item> {
        menus()
            .into_iter()
            .find(|m| m.0 == menu)
            .unwrap_or_else(|| panic!("no {menu} menu"))
            .1
    }

    // covers: REQ-069, REQ-075
    #[test]
    fn the_effects_menu_holds_the_effects_and_the_commands_on_objects_are_in_edit() {
        use ActionId::*;
        let effects = items_of("Effects");
        for id in [
            Fades,
            Subtitles,
            Volume,
            CleanAudio,
            ChangeAspect,
            Scale,
            Move,
            ApplyLut,
            AddText,
            AddOverlap,
        ] {
            assert!(effects.contains(&Item::Action(id)), "{id:?}");
        }
        assert!(effects.contains(&Item::Submenu("Apply to selected clip", APPLY_TO_CLIP)));
        assert!(effects.contains(&Item::Submenu("Apply to whole video", APPLY_TO_VIDEO)));
        // Clean audio can be applied to a clip and to the whole video
        assert!(APPLY_TO_CLIP.contains(&ApplyClipCleanAudio));
        assert!(APPLY_TO_VIDEO.contains(&ApplyWholeCleanAudio));
        // the commands that act on an object are not among the effects any more
        for id in [
            ToggleFadeAtPlayhead,
            GenerateSubtitles,
            AddSubtitleLine,
            AlignHorizontal,
            AlignVertical,
            MoveInside,
            ResetObjects,
        ] {
            assert!(!effects.contains(&Item::Action(id)), "{id:?} in Effects");
            assert!(edit_actions().contains(&id), "{id:?} not in Edit");
        }
        // the Export dialog is opened from File > Export…
        assert!(items_of("File").contains(&Item::Action(Export)));
    }

    /// Every action in the Edit menu, in its submenus too.
    fn edit_actions() -> Vec<ActionId> {
        items_of("Edit")
            .into_iter()
            .flat_map(|i| match i {
                Item::Action(id) => vec![id],
                Item::Submenu(_, ids) => ids.to_vec(),
                _ => Vec::new(),
            })
            .collect()
    }

    // covers: REQ-247
    #[test]
    fn the_edit_menu_is_grouped_in_at_most_six_entries_and_keeps_every_command() {
        use ActionId::*;
        let top = items_of("Edit");
        assert!(top.len() <= 6, "{} entries", top.len());
        for (name, ids) in [
            ("Clipboard", CLIPBOARD),
            ("History", HISTORY),
            ("Clip", CLIP),
            ("Subtitles", SUBTITLES),
        ] {
            assert!(top.contains(&Item::Submenu(name, ids)), "{name}");
        }
        assert!(top.contains(&Item::Action(Settings)));
        for id in [
            Undo,
            Redo,
            Cut,
            Copy,
            Paste,
            Delete,
            Split,
            Merge,
            MoveLeft,
            MoveRight,
            AddToTimeline,
            AddMusic,
            ClipDetails,
            GenerateSubtitles,
            AddSubtitleLine,
        ] {
            assert!(edit_actions().contains(&id), "{id:?}");
        }
        // the shortcuts are the table's, whatever the submenu
        assert_eq!(crate::ui::actions::action(Split).shortcuts.len(), 1);
    }
}
