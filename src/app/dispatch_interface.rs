// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the interface settings: window, popup places, sections, workspaces,
//! toolbar, shortcuts, theme and text size (REQ-234 to REQ-244). Everything is a program
//! setting, never part of a project, and is saved at once except the window, which is
//! saved when it has stayed the same for a moment.

use super::*;
use crate::settings::interface::{MAX_LIST, MAX_NAME, MAX_POPUPS};

impl AppState {
    pub(super) fn dispatch_interface(&mut self, cmd: Command, now: Instant) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            let ui = &mut self.settings.interface;
            match cmd {
                Command::SetTheme(t) => ui.theme = t,
                Command::SetTextSize(t) => ui.text_size = t,
                Command::SetToolbarLabels(on) => ui.toolbar_labels = on,
                Command::SetSectionHidden(s, on) => ui.set_hidden(s, on),
                Command::SetSectionCollapsed(s, on) => ui.set_collapsed(s, on),
                Command::SetSectionFlipped(s, on) => ui.set_flipped(s, on),
                Command::SaveWorkspace(name) => ui.save_workspace(&name)?,
                Command::ApplyWorkspace(name) => {
                    if !ui.apply_workspace(&name) {
                        return Err(format!("There is no workspace called {name}."));
                    }
                }
                Command::DeleteWorkspace(name) => ui.delete_workspace(&name),
                Command::SetToolbarAction(name, on) => {
                    if name.is_empty() || name.len() > MAX_NAME {
                        return Err("not a toolbar command".into());
                    }
                    let list = ui.toolbar.get_or_insert_with(Vec::new);
                    list.retain(|x| *x != name);
                    if on && list.len() < MAX_LIST {
                        list.push(name);
                    }
                }
                Command::ResetToolbar => ui.toolbar = None,
                Command::SetMarked360(id, on) => {
                    let m = self
                        .project
                        .media
                        .iter_mut()
                        .find(|m| m.id == id)
                        .ok_or("That video is no longer in the project")?;
                    if !m.format.can_be_marked() {
                        return Err(
                            "Only a video twice as wide as it is high can be marked as 360°".into(),
                        );
                    }
                    m.format.marked_360 = on;
                    self.dirty = true;
                    self.refresh_still(now);
                    return Ok(());
                }
                Command::SetFavouriteLut(path, on) => {
                    let list = &mut self.settings.favourite_luts;
                    list.retain(|p| *p != path);
                    if on {
                        if !settings::valid_recent(&path) {
                            return Err("That file name cannot be remembered".into());
                        }
                        list.insert(0, path);
                        list.truncate(settings::MAX_FAVOURITES);
                    }
                }
                Command::StartRebind(name) => {
                    self.rebind = Some((name, None));
                    return Ok(());
                }
                Command::RebindConflict(by) => {
                    if let Some((_, c)) = &mut self.rebind {
                        *c = Some(by);
                    }
                    return Ok(());
                }
                Command::CancelRebind => {
                    self.rebind = None;
                    return Ok(());
                }
                Command::Rebind(name, text) => {
                    self.rebind = None;
                    if name.is_empty() || name.len() > MAX_NAME || text.len() > MAX_NAME {
                        return Err("not a shortcut".into());
                    }
                    ui.shortcuts.retain(|k, _| *k != name);
                    ui.shortcuts.insert(name, text);
                }
                Command::ResetShortcuts => {
                    self.rebind = None;
                    ui.shortcuts.clear();
                }
                Command::WindowChanged(w) => {
                    if w.is_valid() && ui.window != Some(w) {
                        ui.window = Some(w);
                        self.window_dirty = Some(now);
                    }
                    return Ok(());
                }
                Command::PopupMoved(name, xy) => {
                    let ok = xy.0.is_finite() && xy.1.is_finite();
                    if !ok || name.len() > MAX_NAME {
                        return Ok(());
                    }
                    if ui.popups.len() >= MAX_POPUPS && !ui.popups.contains_key(&name) {
                        return Ok(());
                    }
                    if ui.popups.get(&name) == Some(&xy) {
                        return Ok(());
                    }
                    ui.popups.insert(name, xy);
                    self.window_dirty = Some(now);
                    return Ok(());
                }
                other => {
                    pass = Some(other);
                    return Ok(());
                }
            }
            self.save_settings()
        };
        let r = run();
        match pass {
            Some(c) => Routed::Pass(c),
            None => Routed::Done(r),
        }
    }
}
