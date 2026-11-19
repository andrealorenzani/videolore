// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Keyboard shortcuts, read from the action table ([`super::actions::ACTIONS`]).
//! `COMMAND` is Ctrl on Windows/Linux and Cmd on macOS. egui turns Ctrl/Cmd+C/X/V into
//! `Copy`/`Cut`/`Paste` events, which are bound to the same actions.
//!
//! Matched events are taken out of the input before any widget sees them (Space does not
//! also press a focused button). Nothing fires while a text field is being typed in, or
//! while a blocking dialog is open (the caller checks that).

use super::actions::{ACTIONS, ActionId, CLOSE_POPUP, CLOSE_POPUP_LABEL, action};
use crate::app::{AppState, Command, Popup};
use eframe::egui::{self, Context, Event, Id, Key, KeyboardShortcut, Modifiers, Order};

fn matches(sc: &KeyboardShortcut, id: ActionId, key: Key, m: Modifiers) -> bool {
    if m.alt || m.command != sc.modifiers.command {
        return false;
    }
    // Ctrl/Cmd + "=" or "+": with or without Shift, whatever the keyboard layout reports
    if id == ActionId::ZoomIn {
        return matches!(key, Key::Equals | Key::Plus);
    }
    m.shift == sc.modifiers.shift && key == sc.logical_key
}

/// The action bound to an input event, if any.
pub fn action_for(event: &Event) -> Option<ActionId> {
    match event {
        Event::Copy => Some(ActionId::Copy),
        Event::Cut => Some(ActionId::Cut),
        Event::Paste(_) => Some(ActionId::Paste),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => ACTIONS
            .iter()
            .find(|a| {
                a.shortcuts
                    .iter()
                    .any(|s| matches(s, a.id, *key, *modifiers))
            })
            .map(|a| a.id),
        _ => None,
    }
}

fn is_escape(e: &Event) -> bool {
    matches!(
        e,
        Event::Key {
            key: Key::Escape,
            pressed: true,
            ..
        }
    )
}

/// Every binding with what it does, in table order (the shortcuts window lists exactly
/// these).
pub fn bindings() -> Vec<(KeyboardShortcut, &'static str)> {
    let mut v: Vec<(KeyboardShortcut, &'static str)> = ACTIONS
        .iter()
        .flat_map(|a| a.shortcuts.iter().map(move |s| (*s, a.label)))
        .collect();
    v.push((CLOSE_POPUP, CLOSE_POPUP_LABEL));
    v
}

/// The id of the window of popup `p`.
pub fn popup_id(p: Popup) -> Id {
    Id::unique(("videolore-popup", p))
}

/// The popup Esc closes: the frontmost of the open ones (the most recently opened when
/// that cannot be told).
pub fn topmost_popup(ctx: &Context, app: &AppState) -> Option<Popup> {
    // the splash is drawn above every other window
    if app.popups.is_open(Popup::Splash) {
        return Some(Popup::Splash);
    }
    let top = ctx.memory(|m| m.areas().top_layer_id(Order::Middle));
    top.and_then(|l| {
        app.popups
            .open
            .iter()
            .copied()
            .find(|p| popup_id(*p) == l.id)
    })
    .or_else(|| app.popups.open.last().copied())
}

/// Commands from this frame's input; matched events are consumed.
pub fn collect(ctx: &Context, app: &AppState) -> Vec<Command> {
    if ctx.text_edit_focused() {
        return Vec::new();
    }
    // an open menu, combo box or context menu closes itself on Esc
    let egui_popup = egui::Popup::is_any_open(ctx);
    let mut out = Vec::new();
    let mut escape = false;
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            if let Some(id) = action_for(e) {
                out.push(id.command(app));
                return false;
            }
            if is_escape(e) && !egui_popup {
                escape = true;
                return false;
            }
            true
        })
    });
    if escape {
        if app.popups.chooser.is_some() {
            out.push(Command::ChooseInstance(Vec::new()));
        } else if let Some(p) = topmost_popup(ctx, app) {
            out.push(Command::ClosePopup(p));
        } else if !app.selection.is_empty() {
            // with no popup open, Esc clears the selected subtitle lines
            out.push(Command::ClearSelection);
        }
    }
    out
}

/// The shortcut shown next to an action ("Ctrl+S", "⌘S"), if it has one.
pub fn shortcut_text(ctx: &Context, id: ActionId) -> Option<String> {
    action(id).shortcuts.first().map(|s| ctx.format_shortcut(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn key(k: Key, m: Modifiers) -> Event {
        Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: m,
        }
    }

    // covers: REQ-071, REQ-025, REQ-026, REQ-028
    #[test]
    fn every_binding_maps_to_its_action() {
        use ActionId::*;
        let c = Modifiers::COMMAND;
        let cs = c | Modifiers::SHIFT;
        let n = Modifiers::NONE;
        for (k, m, want) in [
            (Key::N, c, New),
            (Key::O, c, Open),
            (Key::S, c, Save),
            (Key::S, cs, SaveAs),
            (Key::I, c, Import),
            (Key::E, c, Export),
            (Key::Q, c, Quit),
            (Key::Z, c, Undo),
            (Key::Z, cs, Redo),
            (Key::Y, c, Redo),
            (Key::X, c, Cut),
            (Key::C, c, Copy),
            (Key::V, c, Paste),
            (Key::Delete, n, Delete),
            (Key::Backspace, n, Delete),
            (Key::B, c, Split),
            (Key::Comma, c, Settings),
            (Key::Space, n, PlayPause),
            (Key::Home, n, GoStart),
            (Key::End, n, GoEnd),
            (Key::ArrowLeft, n, PrevFrame),
            (Key::ArrowRight, n, NextFrame),
            (Key::Equals, c, ZoomIn),
            (Key::Equals, cs, ZoomIn),
            (Key::Plus, c, ZoomIn),
            (Key::Plus, cs, ZoomIn),
            (Key::Minus, c, ZoomOut),
            (Key::Num0, c, ZoomFit),
            (Key::F1, n, Shortcuts),
        ] {
            assert_eq!(action_for(&key(k, m)), Some(want), "{k:?} {m:?}");
        }
        assert_eq!(action_for(&Event::Copy), Some(Copy));
        assert_eq!(action_for(&Event::Cut), Some(Cut));
        assert_eq!(action_for(&Event::Paste(String::new())), Some(Paste));
        // not bound: wrong modifiers, Alt, text, releases
        for (k, m) in [
            (Key::B, n),
            (Key::B, c | Modifiers::ALT),
            (Key::S, Modifiers::SHIFT),
            (Key::Space, c),
            (Key::Equals, n),
            (Key::Num0, n),
            (Key::Escape, n),
        ] {
            assert_eq!(action_for(&key(k, m)), None, "{k:?} {m:?}");
        }
        assert_eq!(action_for(&Event::Text("b".into())), None);
        let released = Event::Key {
            key: Key::B,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: c,
        };
        assert_eq!(action_for(&released), None);
    }

    // covers: REQ-071
    #[test]
    fn no_shortcut_is_bound_twice_and_the_list_is_the_bound_set() {
        let b = bindings();
        for (i, (s, _)) in b.iter().enumerate() {
            assert!(b[i + 1..].iter().all(|(o, _)| o != s), "{s:?} bound twice");
        }
        // every listed binding fires its action (Esc closes popups)
        for a in ACTIONS {
            for s in a.shortcuts {
                assert_eq!(
                    action_for(&key(s.logical_key, s.modifiers)),
                    Some(a.id),
                    "{}",
                    a.label
                );
            }
        }
        assert_eq!(b.last().unwrap().0, CLOSE_POPUP);
        // the 0.x shortcuts are kept
        let kept = [
            (Key::S, Modifiers::COMMAND),
            (Key::Q, Modifiers::COMMAND),
            (Key::B, Modifiers::COMMAND),
            (Key::Z, Modifiers::COMMAND),
            (Key::Y, Modifiers::COMMAND),
            (Key::Comma, Modifiers::COMMAND),
            (Key::Space, Modifiers::NONE),
        ];
        for (k, m) in kept {
            assert!(
                b.iter()
                    .any(|(s, _)| s.logical_key == k && s.modifiers == m)
            );
        }
        assert_eq!(b.len(), 32);
        let ctx = Context::default();
        assert!(shortcut_text(&ctx, ActionId::Save).unwrap().contains('S'));
        assert_eq!(shortcut_text(&ctx, ActionId::Merge), None);
    }
}
