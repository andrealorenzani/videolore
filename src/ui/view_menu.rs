// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The submenus of View that change the look of the window: Sections, Workspaces, Theme
//! and Customise toolbar (REQ-237, REQ-239, REQ-240, REQ-241, REQ-243).

use super::actions::{ACTIONS, ActionId};
use super::strings::tr;
use crate::app::{AppState, Command};
use crate::settings::interface::{Section, Theme};
use eframe::egui::{self, Id, ScrollArea, Ui};

/// View > Sections ▸: show or hide Media and Effects.
pub fn sections(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    ui.menu_button(tr!(Sections), |ui| {
        for s in [Section::Media, Section::Effects] {
            let mut shown = !app.settings.interface.is_hidden(s);
            let name = format!("{s:?}");
            if ui.checkbox(&mut shown, name).changed() {
                cmds.push(Command::SetSectionHidden(s, !shown));
            }
        }
    });
}

/// View > Workspaces ▸: switch to, delete or save a workspace.
pub fn workspaces(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    ui.menu_button(tr!(Workspaces), |ui| {
        for w in &app.settings.interface.workspaces {
            ui.horizontal(|ui| {
                if ui.button(&w.name).clicked() {
                    cmds.push(Command::ApplyWorkspace(w.name.clone()));
                    ui.close();
                }
                if ui
                    .small_button(tr!(CloseMark))
                    .on_hover_text(tr!(DeleteThisWorkspace))
                    .clicked()
                {
                    cmds.push(Command::DeleteWorkspace(w.name.clone()));
                }
            });
        }
        ui.separator();
        let id = Id::unique("videolore-workspace-name");
        let mut name: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        ui.add(egui::TextEdit::singleline(&mut name).hint_text(tr!(WorkspaceName)));
        if ui.button(tr!(SaveTheCurrentLayout)).clicked() {
            cmds.push(Command::SaveWorkspace(name.clone()));
            name.clear();
        }
        ui.data_mut(|d| d.insert_temp(id, name));
    });
}

/// View > Theme ▸: System, Light, Dark.
pub fn theme(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    ui.menu_button(tr!(Theme), |ui| {
        for (t, text) in [
            (Theme::System, tr!(ThemeSystem)),
            (Theme::Light, tr!(ThemeLight)),
            (Theme::Dark, tr!(ThemeDark)),
        ] {
            if ui.radio(app.settings.interface.theme == t, text).clicked() {
                cmds.push(Command::SetTheme(t));
                ui.close();
            }
        }
    });
}

/// View > Customise toolbar ▸: which commands the toolbar shows, icons or icons and text,
/// and Reset toolbar.
pub fn toolbar(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    ui.menu_button(tr!(CustomiseToolbar), |ui| {
        let mut labels = app.settings.interface.toolbar_labels;
        if ui.checkbox(&mut labels, tr!(IconsAndText)).changed() {
            cmds.push(Command::SetToolbarLabels(labels));
        }
        ui.separator();
        let shown: Vec<ActionId> = super::toolbar::fixed(app);
        ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            // the commands on the toolbar first, then the others
            let mut list: Vec<&super::actions::Action> =
                ACTIONS.iter().filter(|a| in_a_menu(a.id)).collect();
            list.sort_by_key(|a| shown.iter().position(|s| *s == a.id).unwrap_or(usize::MAX));
            for a in list {
                let mut on = shown.contains(&a.id);
                if ui.checkbox(&mut on, a.label).changed() {
                    // the first change writes the whole default list, then the change
                    if app.settings.interface.toolbar.is_none() {
                        for id in &shown {
                            cmds.push(Command::SetToolbarAction(id.name(), true));
                        }
                    }
                    cmds.push(Command::SetToolbarAction(a.id.name(), on));
                }
            }
        });
        ui.separator();
        if ui.button(tr!(ResetToolbar)).clicked() {
            cmds.push(Command::ResetToolbar);
            ui.close();
        }
    });
}

/// A command of the menus (everything in the table but the context-only ones).
fn in_a_menu(id: ActionId) -> bool {
    super::menu::menus().iter().any(|(_, items)| {
        items.iter().any(|i| match i {
            super::menu::Item::Action(a) => *a == id,
            super::menu::Item::Submenu(_, ids) => ids.contains(&id),
            _ => false,
        })
    })
}
