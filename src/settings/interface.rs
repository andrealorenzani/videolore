// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The interface settings of the program (2.0.4, REQ-234 to REQ-244, ADR-195): window
//! geometry, remembered popup positions, hidden and collapsed sections, the dock side,
//! workspaces, the toolbar list, shortcuts, theme and text size. Everything is read
//! leniently from an untrusted file: a value out of range or not understood is dropped
//! alone, the rest stays (REQ-057).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Most workspaces (REQ-239).
pub const MAX_WORKSPACES: usize = 10;
/// Longest workspace name, in characters.
pub const MAX_WORKSPACE_NAME: usize = 40;
/// Most remembered popup positions.
pub const MAX_POPUPS: usize = 32;
/// Most toolbar entries and shortcut entries.
pub const MAX_LIST: usize = 64;
/// Longest key or action name kept, in bytes.
pub const MAX_NAME: usize = 48;
/// Largest remembered coordinate (points).
pub const MAX_COORD: f32 = 32768.0;

/// The colours of the interface (REQ-243).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Theme {
    /// Follows the system.
    #[default]
    System,
    Light,
    Dark,
}

/// The size of the interface text (REQ-244).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextSize {
    #[default]
    Normal,
    /// 125 %.
    Large,
    /// 150 %.
    Larger,
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Normal, TextSize::Large, TextSize::Larger];

    /// The factor of the text styles.
    pub fn scale(self) -> f32 {
        match self {
            TextSize::Normal => 1.0,
            TextSize::Large => 1.25,
            TextSize::Larger => 1.5,
        }
    }
}

/// The sections of the window that can be hidden or collapsed (REQ-237).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Section {
    Media,
    Effects,
    Preview,
    Timeline,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Media,
        Section::Effects,
        Section::Preview,
        Section::Timeline,
    ];

    /// The Preview and the Timeline cannot be hidden (they are the work area).
    pub fn can_hide(self) -> bool {
        matches!(self, Section::Media | Section::Effects)
    }
}

/// The window the last time (REQ-234). Sizes in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub width: f32,
    pub height: f32,
    /// Position of the top-left corner; `None` = let the system choose.
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub maximised: bool,
}

/// Smallest window the program remembers (the minimum of REQ-066).
pub const MIN_WINDOW: (f32, f32) = (640.0, 400.0);

impl Window {
    /// Sizes and place that make sense (finite, in range).
    pub fn is_valid(&self) -> bool {
        let ok = |v: f32, lo: f32| v.is_finite() && (lo..=MAX_COORD).contains(&v);
        ok(self.width, MIN_WINDOW.0)
            && ok(self.height, MIN_WINDOW.1)
            && self.x.is_none_or(|v| v.is_finite() && v.abs() <= MAX_COORD)
            && self.y.is_none_or(|v| v.is_finite() && v.abs() <= MAX_COORD)
    }

    /// The saved place when part of the window would be seen on a screen of `monitor`
    /// points (a window may sit on another screen: only places far outside are refused),
    /// else `None` (the system chooses) (REQ-234).
    pub fn visible_position(&self, monitor: (f32, f32)) -> Option<(f32, f32)> {
        let (x, y) = (self.x?, self.y?);
        let seen = x + self.width > 64.0
            && y + self.height > 64.0
            && x < monitor.0 * 3.0 - 64.0
            && y < monitor.1 * 3.0 - 64.0
            && y > -self.height + 32.0;
        seen.then_some((x, y))
    }
}

/// One saved arrangement of the sections (REQ-239).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub name: String,
    #[serde(default)]
    pub hidden: Vec<Section>,
    #[serde(default)]
    pub collapsed: Vec<Section>,
    /// Media and Effects docked on the other side (REQ-238).
    #[serde(default)]
    pub flipped: Vec<Section>,
}

/// What a workspace name may be: 1 to 40 characters, no control characters.
pub fn valid_workspace_name(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty() && n.chars().count() <= MAX_WORKSPACE_NAME && !n.chars().any(char::is_control)
}

/// The interface settings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Interface {
    pub window: Option<Window>,
    /// Popup (and Settings) positions by popup name, top-left in points (REQ-235).
    pub popups: BTreeMap<String, (f32, f32)>,
    pub hidden: Vec<Section>,
    pub collapsed: Vec<Section>,
    /// The sections docked on the other side of the Preview or Timeline (REQ-238): Media
    /// on the right, Effects on the left.
    pub flipped: Vec<Section>,
    pub workspaces: Vec<Workspace>,
    /// The toolbar's actions by name (REQ-240); `None` = the default list.
    pub toolbar: Option<Vec<String>>,
    /// Icons and text (REQ-241); icons only by default.
    pub toolbar_labels: bool,
    /// Rebound shortcuts: action name to shortcut text (REQ-242).
    pub shortcuts: BTreeMap<String, String>,
    pub theme: Theme,
    pub text_size: TextSize,
}

fn name_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_NAME && !s.chars().any(char::is_control)
}

fn sections(v: Option<&Value>) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for item in v.and_then(Value::as_array).into_iter().flatten() {
        if let Ok(s) = serde_json::from_value::<Section>(item.clone())
            && !out.contains(&s)
        {
            out.push(s);
        }
    }
    out
}

fn pair(v: &Value) -> Option<(f32, f32)> {
    let a = v.as_array()?;
    let (x, y) = (a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32);
    (a.len() == 2 && x.is_finite() && y.is_finite() && x.abs() <= MAX_COORD && y.abs() <= MAX_COORD)
        .then_some((x, y))
}

impl Interface {
    /// Reads the settings from a JSON value; everything wrong is dropped one by one.
    pub fn from_value(v: &Value) -> Interface {
        let mut me = Interface::default();
        let Some(o) = v.as_object() else {
            return me;
        };
        me.window = o
            .get("window")
            .and_then(|w| serde_json::from_value::<Window>(w.clone()).ok())
            .filter(Window::is_valid);
        for (k, p) in o
            .get("popups")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            if name_ok(k)
                && me.popups.len() < MAX_POPUPS
                && let Some(xy) = pair(p)
            {
                me.popups.insert(k.clone(), xy);
            }
        }
        me.hidden = sections(o.get("hidden"))
            .into_iter()
            .filter(|s| s.can_hide())
            .collect();
        me.collapsed = sections(o.get("collapsed"));
        me.flipped = sections(o.get("flipped"))
            .into_iter()
            .filter(|s| s.can_hide())
            .collect();
        for w in o
            .get("workspaces")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Ok(mut w) = serde_json::from_value::<Workspace>(w.clone())
                && valid_workspace_name(&w.name)
                && me.workspaces.len() < MAX_WORKSPACES
                && !me.workspaces.iter().any(|x| x.name == w.name)
            {
                w.name = w.name.trim().to_string();
                w.hidden.retain(|s| s.can_hide());
                w.flipped.retain(|s| s.can_hide());
                me.workspaces.push(w);
            }
        }
        me.toolbar = o.get("toolbar").and_then(Value::as_array).map(|a| {
            let mut out: Vec<String> = Vec::new();
            for s in a.iter().filter_map(Value::as_str) {
                if name_ok(s) && out.len() < MAX_LIST && !out.iter().any(|x| x == s) {
                    out.push(s.to_string());
                }
            }
            out
        });
        me.toolbar_labels = o
            .get("toolbar_labels")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        for (k, s) in o
            .get("shortcuts")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            if let Some(s) = s.as_str()
                && name_ok(k)
                && name_ok(s)
                && me.shortcuts.len() < MAX_LIST
            {
                me.shortcuts.insert(k.clone(), s.to_string());
            }
        }
        me.theme = o
            .get("theme")
            .and_then(|d| serde_json::from_value(d.clone()).ok())
            .unwrap_or_default();
        me.text_size = o
            .get("text_size")
            .and_then(|d| serde_json::from_value(d.clone()).ok())
            .unwrap_or_default();
        me
    }

    pub fn is_hidden(&self, s: Section) -> bool {
        self.hidden.contains(&s)
    }

    pub fn is_collapsed(&self, s: Section) -> bool {
        self.collapsed.contains(&s)
    }

    /// Hides or shows a section (only Media and Effects can be hidden).
    pub fn set_hidden(&mut self, s: Section, hidden: bool) {
        self.hidden.retain(|x| *x != s);
        if hidden && s.can_hide() {
            self.hidden.push(s);
        }
    }

    pub fn is_flipped(&self, s: Section) -> bool {
        self.flipped.contains(&s)
    }

    /// Docks Media or Effects on the other side (or back).
    pub fn set_flipped(&mut self, s: Section, flipped: bool) {
        self.flipped.retain(|x| *x != s);
        if flipped && s.can_hide() {
            self.flipped.push(s);
        }
    }

    pub fn set_collapsed(&mut self, s: Section, collapsed: bool) {
        self.collapsed.retain(|x| *x != s);
        if collapsed {
            self.collapsed.push(s);
        }
    }

    /// Reset layout (REQ-237, REQ-238): every section shown and open, in the default places.
    pub fn reset_layout(&mut self) {
        self.hidden.clear();
        self.collapsed.clear();
        self.flipped.clear();
    }

    /// Saves the current arrangement under `name`, replacing one of the same name; refuses
    /// a bad name and an eleventh workspace.
    pub fn save_workspace(&mut self, name: &str) -> Result<(), String> {
        if !valid_workspace_name(name) {
            return Err("A workspace name has 1 to 40 characters.".into());
        }
        let name = name.trim().to_string();
        let ws = Workspace {
            name: name.clone(),
            hidden: self.hidden.clone(),
            collapsed: self.collapsed.clone(),
            flipped: self.flipped.clone(),
        };
        if let Some(slot) = self.workspaces.iter_mut().find(|w| w.name == name) {
            *slot = ws;
        } else if self.workspaces.len() >= MAX_WORKSPACES {
            return Err("There can be 10 workspaces at most.".into());
        } else {
            self.workspaces.push(ws);
        }
        Ok(())
    }

    /// Applies the workspace called `name`; false when there is none.
    pub fn apply_workspace(&mut self, name: &str) -> bool {
        let Some(w) = self.workspaces.iter().find(|w| w.name == name).cloned() else {
            return false;
        };
        self.hidden = w.hidden;
        self.collapsed = w.collapsed;
        self.flipped = w.flipped;
        true
    }

    pub fn delete_workspace(&mut self, name: &str) {
        self.workspaces.retain(|w| w.name != name);
    }
}

/// Reads the `interface` field of the settings file.
pub fn lenient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Interface, D::Error> {
    Ok(Interface::from_value(&Value::deserialize(d)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // covers: REQ-234
    #[test]
    fn the_window_is_kept_and_an_off_screen_place_is_refused() {
        let w = Window {
            width: 1400.0,
            height: 900.0,
            x: Some(100.0),
            y: Some(50.0),
            maximised: true,
        };
        let i = Interface {
            window: Some(w),
            ..Default::default()
        };
        let v = serde_json::to_value(&i).unwrap();
        assert_eq!(Interface::from_value(&v).window, Some(w));
        assert_eq!(w.visible_position((1920.0, 1080.0)), Some((100.0, 50.0)));
        let far = Window {
            x: Some(30000.0),
            ..w
        };
        assert_eq!(far.visible_position((1920.0, 1080.0)), None);
        let bad = json!({"window": {"width": 1e30, "height": 5.0, "x": null, "y": null, "maximised": false}});
        assert_eq!(Interface::from_value(&bad).window, None);
    }

    // covers: REQ-235
    #[test]
    fn popup_positions_are_checked_and_capped() {
        let v = json!({"popups": {"Export": [10.0, 20.0], "Bad": [1e12, 0.0], "Short": [1.0],
            "": [0.0, 0.0]}});
        let i = Interface::from_value(&v);
        assert_eq!(i.popups.len(), 1);
        assert_eq!(i.popups["Export"], (10.0, 20.0));
    }

    // covers: REQ-237, REQ-238
    #[test]
    fn sections_hide_collapse_dock_and_reset() {
        let mut i = Interface::default();
        i.set_hidden(Section::Media, true);
        i.set_hidden(Section::Preview, true);
        i.set_collapsed(Section::Effects, true);
        i.set_flipped(Section::Media, true);
        assert!(i.is_hidden(Section::Media) && !i.is_hidden(Section::Preview));
        let back = Interface::from_value(&serde_json::to_value(&i).unwrap());
        assert_eq!(back, i);
        i.reset_layout();
        assert_eq!(i, Interface::default());
    }

    // covers: REQ-239
    #[test]
    fn workspaces_are_saved_applied_deleted_and_limited() {
        let mut i = Interface::default();
        i.set_hidden(Section::Media, true);
        i.save_workspace("Editing").unwrap();
        i.reset_layout();
        i.set_flipped(Section::Effects, true);
        i.save_workspace("Subtitles").unwrap();
        assert!(i.apply_workspace("Editing"));
        assert!(i.is_hidden(Section::Media) && !i.is_flipped(Section::Effects));
        assert!(!i.apply_workspace("none"));
        i.delete_workspace("Editing");
        assert_eq!(i.workspaces.len(), 1);
        assert!(i.save_workspace("").is_err());
        assert!(i.save_workspace(&"x".repeat(41)).is_err());
        for n in 0..9 {
            i.save_workspace(&format!("w{n}")).unwrap();
        }
        assert!(i.save_workspace("eleven").is_err());
        // a damaged entry is dropped and the others stay
        let v = json!({"workspaces": [{"name": "ok"}, {"name": 5}, {"name": "bad\u{7}"}, "x"]});
        let back = Interface::from_value(&v);
        assert_eq!(back.workspaces.len(), 1);
        assert_eq!(back.workspaces[0].name, "ok");
    }

    // covers: REQ-240, REQ-241, REQ-242, REQ-243, REQ-244
    #[test]
    fn toolbar_shortcuts_theme_and_text_size_are_read_leniently() {
        let v = json!({"toolbar": ["Split", "Split", "Cut", 5, "bad\u{1}"],
            "toolbar_labels": true,
            "shortcuts": {"Split": "Ctrl+K", "Cut": 3},
            "theme": "Dark", "text_size": "Larger"});
        let i = Interface::from_value(&v);
        assert_eq!(
            i.toolbar,
            Some(vec!["Split".to_string(), "Cut".to_string()])
        );
        assert!(i.toolbar_labels);
        assert_eq!(i.shortcuts.len(), 1);
        assert_eq!((i.theme, i.text_size), (Theme::Dark, TextSize::Larger));
        assert_eq!(TextSize::Larger.scale(), 1.5);
        let j = Interface::from_value(&json!({"theme": "Purple", "text_size": 3}));
        assert_eq!((j.theme, j.text_size), (Theme::System, TextSize::Normal));
        assert_eq!(Interface::from_value(&json!(7)), Interface::default());
    }
}
