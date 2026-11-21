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

/// A shortcut the user chose for an action, replacing the default ones (REQ-242).
pub type Override = (ActionId, KeyboardShortcut);

/// Writes a shortcut as text for the settings: "Cmd+Shift+B".
pub fn shortcut_name(s: &KeyboardShortcut) -> String {
    let mut out = String::new();
    for (on, name) in [
        (s.modifiers.command, "Cmd"),
        (s.modifiers.shift, "Shift"),
        (s.modifiers.alt, "Alt"),
    ] {
        if on {
            out += name;
            out.push('+');
        }
    }
    out + s.logical_key.name()
}

/// Reads the text written by [`shortcut_name`]; anything else is `None`.
pub fn parse_shortcut(text: &str) -> Option<KeyboardShortcut> {
    let mut parts: Vec<&str> = text.split('+').collect();
    let key = Key::from_name(parts.pop()?)?;
    let mut m = Modifiers::NONE;
    for p in parts {
        match p {
            "Cmd" => m.command = true,
            "Shift" => m.shift = true,
            "Alt" => m.alt = true,
            _ => return None,
        }
    }
    // Ctrl and Cmd are one modifier: `command` is set, `ctrl` is left alone
    Some(KeyboardShortcut::new(m, key))
}

/// The overrides the settings describe: names that are not actions and texts that are
/// not shortcuts are ignored.
pub fn overrides(shortcuts: &std::collections::BTreeMap<String, String>) -> Vec<Override> {
    shortcuts
        .iter()
        .filter_map(|(k, v)| Some((ActionId::from_name(k)?, parse_shortcut(v)?)))
        .collect()
}

fn store_id() -> Id {
    Id::unique("videolore-overrides")
}

/// Makes the overrides of the settings the ones in use (called every frame).
pub fn install(ctx: &Context, shortcuts: &std::collections::BTreeMap<String, String>) {
    let list = overrides(shortcuts);
    let same = ctx.data(|d| d.get_temp::<Vec<Override>>(store_id()).as_ref() == Some(&list));
    if !same {
        ctx.data_mut(|d| d.insert_temp(store_id(), list));
    }
}

/// The overrides in use.
pub fn current(ctx: &Context) -> Vec<Override> {
    ctx.data(|d| d.get_temp::<Vec<Override>>(store_id()))
        .unwrap_or_default()
}

/// The shortcuts of `id` now: the user's one, or the table's.
pub fn shortcuts_of(over: &[Override], id: ActionId) -> Vec<KeyboardShortcut> {
    let mine: Vec<KeyboardShortcut> = over.iter().filter(|o| o.0 == id).map(|o| o.1).collect();
    if mine.is_empty() {
        action(id).shortcuts.to_vec()
    } else {
        mine
    }
}

/// The action that already uses `sc` (not `except`), if any.
pub fn used_by(over: &[Override], sc: &KeyboardShortcut, except: ActionId) -> Option<ActionId> {
    ACTIONS.iter().map(|a| a.id).find(|id| {
        *id != except
            && shortcuts_of(over, *id)
                .iter()
                .any(|s| s.logical_key == sc.logical_key && s.modifiers == sc.modifiers)
    })
}

/// The action bound to an input event, if any.
pub fn action_for(event: &Event) -> Option<ActionId> {
    action_for_in(event, &[])
}

/// The action bound to an input event with the user's shortcuts in use.
pub fn action_for_in(event: &Event, over: &[Override]) -> Option<ActionId> {
    match event {
        Event::Copy => Some(ActionId::Copy),
        Event::Cut => Some(ActionId::Cut),
        // text, an image or the key itself: whatever the system clipboard holds, the clip
        // of the last cut or copy is pasted (REQ-195)
        Event::Paste(_) | Event::PasteImage(_) => Some(ActionId::Paste),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => ACTIONS
            .iter()
            .find(|a| {
                shortcuts_of(over, a.id)
                    .iter()
                    .any(|s| matches(s, a.id, *key, *modifiers))
            })
            .map(|a| a.id),
        _ => None,
    }
}

fn is_plain_arrow(e: &Event) -> bool {
    matches!(
        e,
        Event::Key {
            key: Key::ArrowLeft | Key::ArrowRight,
            pressed: true,
            modifiers,
            ..
        } if !modifiers.any()
    )
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

/// Every binding in use with its action (the shortcuts window lists these).
pub fn bindings_in(over: &[Override]) -> Vec<(Option<ActionId>, KeyboardShortcut, &'static str)> {
    let mut v = Vec::new();
    for a in ACTIONS {
        for s in shortcuts_of(over, a.id) {
            v.push((Some(a.id), s, a.label));
        }
    }
    v.push((None, CLOSE_POPUP, CLOSE_POPUP_LABEL));
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
    let over = current(ctx);
    if let Some((name, _)) = &app.rebind {
        return capture(ctx, name, &over);
    }
    if ctx.text_edit_focused() {
        return Vec::new();
    }
    // an open menu, combo box or context menu closes itself on Esc
    let egui_popup = egui::Popup::is_any_open(ctx);
    let mut out = Vec::new();
    let mut escape = false;
    // the picture of a 360 clip has the focus: its keys are the camera's (REQ-272)
    let camera_keys = ctx
        .data(|d| d.get_temp::<bool>(super::picture::keyboard_camera_id()))
        .unwrap_or(false);
    // the keyframe bar of a popup has the focus: Left and Right are its own (REQ-203)
    // (recorded by the popup in the last frames: a closed popup leaves nothing behind)
    let now = ctx.cumulative_pass_nr();
    let keybar = ctx
        .data(|d| d.get_temp::<(u64, bool)>(super::frame_popups::bar_focus_id()))
        .is_some_and(|(pass, focused)| focused && pass + 2 >= now);
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            if (keybar || camera_keys) && is_plain_arrow(e) {
                return true;
            }
            if let Some(id) = action_for_in(e, &over) {
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

/// Waiting for the new keys of the action `name` (REQ-242): the first key pressed is the
/// new shortcut, unless another action uses it (then it is reported and the change
/// waits). Esc gives up.
fn capture(ctx: &Context, name: &str, over: &[Override]) -> Vec<Command> {
    let mut out = Vec::new();
    let Some(id) = ActionId::from_name(name) else {
        return vec![Command::CancelRebind];
    };
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = e
            else {
                return true;
            };
            if !out.is_empty() {
                return false;
            }
            if *key == Key::Escape {
                out.push(Command::CancelRebind);
                return false;
            }
            // a modifier alone is not a shortcut
            let mut m = Modifiers::NONE;
            m.command = modifiers.command;
            m.shift = modifiers.shift;
            m.alt = modifiers.alt;
            let sc = KeyboardShortcut::new(m, *key);
            out.push(match used_by(over, &sc, id) {
                Some(other) => Command::RebindConflict(action(other).label.to_string()),
                None => Command::Rebind(name.to_string(), shortcut_name(&sc)),
            });
            false
        })
    });
    out
}

/// The shortcut shown next to an action ("Ctrl+S", "⌘S"), if it has one.
pub fn shortcut_text(ctx: &Context, id: ActionId) -> Option<String> {
    shortcuts_of(&current(ctx), id)
        .first()
        .map(|s| ctx.format_shortcut(s))
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
            (Key::OpenBracket, n, PrevMarker),
            (Key::CloseBracket, n, NextMarker),
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
        assert_eq!(b.len(), 34, "[ and ] were added in 2.0.4");
        let ctx = Context::default();
        assert!(shortcut_text(&ctx, ActionId::Save).unwrap().contains('S'));
        assert_eq!(shortcut_text(&ctx, ActionId::Merge), None);
    }
}
