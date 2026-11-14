// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Keyboard shortcuts. `command` is Ctrl on Windows/Linux and Cmd on macOS.
//! Note that egui turns Ctrl/Cmd+C/X/V into `Copy`/`Cut`/`Paste` events.

use crate::app::Command;
use eframe::egui::{Event, Key, Modifiers};

/// Human-readable list shown in the UI.
pub const HELP: &[(&str, &str)] = &[
    ("Ctrl/Cmd+B", "Split"),
    ("Ctrl/Cmd+X", "Cut"),
    ("Ctrl/Cmd+C", "Copy"),
    ("Ctrl/Cmd+V", "Paste"),
    ("Ctrl/Cmd+Z", "Undo"),
    ("Ctrl/Cmd+Shift+Z, Ctrl/Cmd+Y", "Redo"),
    ("Space", "Play/Pause"),
];

/// The command bound to an input event, if any.
pub fn command_for(event: &Event) -> Option<Command> {
    match event {
        Event::Copy => Some(Command::Copy),
        Event::Cut => Some(Command::Cut),
        Event::Paste(_) => Some(Command::Paste),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => key_command(*key, *modifiers),
        _ => None,
    }
}

fn key_command(key: Key, m: Modifiers) -> Option<Command> {
    if m.alt {
        return None;
    }
    match (m.command, m.shift, key) {
        (false, false, Key::Space) => Some(Command::PlayPause),
        (true, false, Key::B) => Some(Command::Split),
        (true, false, Key::X) => Some(Command::Cut),
        (true, false, Key::C) => Some(Command::Copy),
        (true, false, Key::V) => Some(Command::Paste),
        (true, false, Key::Z) => Some(Command::Undo),
        (true, true, Key::Z) | (true, false, Key::Y) => Some(Command::Redo),
        _ => None,
    }
}

/// Commands from this frame's input (none while a text field has focus).
pub fn collect(ctx: &eframe::egui::Context) -> Vec<Command> {
    if ctx.wants_keyboard_input() {
        return Vec::new();
    }
    ctx.input(|i| i.events.iter().filter_map(command_for).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(k: Key, m: Modifiers) -> Event {
        Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: m,
        }
    }

    // covers: REQ-025
    #[test]
    fn shortcuts_map_to_commands() {
        let c = Modifiers::COMMAND;
        assert_eq!(command_for(&key(Key::B, c)), Some(Command::Split));
        assert_eq!(command_for(&key(Key::X, c)), Some(Command::Cut));
        assert_eq!(command_for(&key(Key::C, c)), Some(Command::Copy));
        assert_eq!(command_for(&key(Key::V, c)), Some(Command::Paste));
        assert_eq!(command_for(&key(Key::Z, c)), Some(Command::Undo));
        assert_eq!(
            command_for(&key(Key::Z, c | Modifiers::SHIFT)),
            Some(Command::Redo)
        );
        assert_eq!(command_for(&key(Key::Y, c)), Some(Command::Redo));
        assert_eq!(
            command_for(&key(Key::Space, Modifiers::NONE)),
            Some(Command::PlayPause)
        );
        assert_eq!(command_for(&Event::Copy), Some(Command::Copy));
        assert_eq!(command_for(&Event::Cut), Some(Command::Cut));
        assert_eq!(
            command_for(&Event::Paste(String::new())),
            Some(Command::Paste)
        );
        assert_eq!(command_for(&key(Key::B, Modifiers::NONE)), None);
        assert_eq!(command_for(&key(Key::B, c | Modifiers::ALT)), None);
        assert_eq!(command_for(&Event::Text("b".into())), None);
        let released = Event::Key {
            key: Key::B,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: c,
        };
        assert_eq!(command_for(&released), None);
        assert_eq!(HELP.len(), 7);
    }
}
