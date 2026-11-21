// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 2.0.4 (REQ-234 to REQ-247): sections, workspaces, toolbar,
//! shortcuts, popup places, text size and the accessibility tree.

use super::ux_tests::{Harness, close_menus, find_in, loaded, open_menu, open_submenu, reachable};
use super::*;
use crate::app::Popup;
use crate::settings::interface::{Section, TextSize, Theme};
use crate::settings::{Interface, Settings};
use eframe::egui::{Event, Key, Modifiers};

fn restart(h: &Harness) -> Harness {
    // what the next run reads from the settings file
    let json = serde_json::to_string(&h.app.settings).unwrap();
    let settings: Settings = serde_json::from_str(&json).unwrap();
    let (_d, mut app) = loaded();
    app.settings = settings;
    Harness::new(app, 1280.0, 800.0)
}

fn run(h: &mut Harness, cmd: Command) {
    h.app.dispatch(cmd, Instant::now()).unwrap();
    h.idle(3);
}

// covers: REQ-237, REQ-238
#[test]
fn sections_hide_collapse_dock_and_reset_and_restart_keeps_them() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let base = h.cache.sections.unwrap();
    // hide Media from the header menu: the Preview takes the row
    let more = h
        .nodes
        .iter()
        .filter(|n| n.label == "⋯")
        .min_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x))
        .cloned()
        .unwrap();
    h.click(more.rect.center());
    let hide = h
        .find("Hide this section")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()));
    h.click(hide.rect.center());
    assert!(h.app.settings.interface.is_hidden(Section::Media));
    h.idle(3);
    let r = h.cache.sections.unwrap();
    assert!(r.preview.width() > base.preview.width() + base.media.width() - 1.0);
    // Preview and Timeline cannot be hidden
    run(&mut h, Command::SetSectionHidden(Section::Preview, true));
    assert!(!h.app.settings.interface.is_hidden(Section::Preview));
    // collapse Effects: only a narrow column with its header
    run(&mut h, Command::SetSectionCollapsed(Section::Effects, true));
    let r = h.cache.sections.unwrap();
    assert_eq!(r.effects.width(), layout::COLLAPSED_W);
    assert!(h.has("Effects"), "the header stays");
    // dock Media on the right (shown again) and Effects on the left
    run(&mut h, Command::SetSectionHidden(Section::Media, false));
    run(&mut h, Command::SetSectionFlipped(Section::Media, true));
    run(&mut h, Command::SetSectionFlipped(Section::Effects, true));
    let r = h.cache.sections.unwrap();
    assert!(r.media.min.x > r.preview.min.x && r.effects.min.x < r.timeline.min.x);
    // the minimum sizes of REQ-066 hold in a small window
    h.size = vec2(800.0, 500.0);
    h.idle(3);
    let r = h.cache.sections.unwrap();
    assert!(r.media.width() >= layout::MINS.media - 1.0);
    h.size = vec2(1280.0, 800.0);
    h.idle(3);
    // a restart keeps all of it
    let h2 = restart(&h);
    let i = &h2.app.settings.interface;
    assert!(i.is_collapsed(Section::Effects) && i.is_flipped(Section::Media));
    assert_eq!(
        h2.cache.sections.unwrap().effects.width(),
        layout::COLLAPSED_W
    );
    // Reset layout shows everything in its place
    let mut h = h2;
    run(&mut h, Command::ResetLayout);
    assert_eq!(h.app.settings.interface, Interface::default());
    assert_eq!(h.cache.sections.unwrap(), base);
}

// covers: REQ-239
#[test]
fn workspaces_are_saved_switched_and_deleted_from_the_view_menu() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    run(&mut h, Command::SetSectionHidden(Section::Media, true));
    run(&mut h, Command::SaveWorkspace("Review".into()));
    run(&mut h, Command::ResetLayout);
    run(&mut h, Command::SaveWorkspace("Edit".into()));
    // the menu lists them; clicking one switches to it
    let menu = open_menu(&mut h, "View");
    let sub = open_submenu(&mut h, &menu, "Workspaces");
    let review = find_in(&sub, "Review").expect("listed").clone();
    h.click(review.rect.center());
    assert!(h.app.settings.interface.is_hidden(Section::Media));
    close_menus(&mut h);
    let h2 = restart(&h);
    assert_eq!(h2.app.settings.interface.workspaces.len(), 2);
    run(&mut h, Command::DeleteWorkspace("Review".into()));
    assert_eq!(h.app.settings.interface.workspaces.len(), 1);
    assert!(
        h.app
            .dispatch(Command::ApplyWorkspace("none".into()), Instant::now())
            .is_err()
    );
}

// covers: REQ-240, REQ-241
#[test]
fn the_toolbar_is_customised_shows_icons_or_text_and_resets() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let on_toolbar = |h: &Harness, name: &str| {
        h.nodes
            .iter()
            .any(|n| n.raw == name && n.rect.min.y > 22.0 && n.rect.max.y < 60.0)
    };
    assert!(on_toolbar(&h, "Paste"));
    // View > Customise toolbar: untick Paste
    let menu = open_menu(&mut h, "View");
    let sub = open_submenu(&mut h, &menu, "Customise toolbar");
    let paste = find_in(&sub, "Paste").expect("listed").clone();
    h.click(paste.rect.center());
    close_menus(&mut h);
    close_menus(&mut h);
    h.idle(3);
    assert!(!on_toolbar(&h, "Paste"), "removed from the toolbar");
    assert!(on_toolbar(&h, "Split"), "the others stay");
    // it still works from the Edit menu
    let menu = open_menu(&mut h, "Edit");
    let clip = open_submenu(&mut h, &menu, "Clipboard");
    assert!(find_in(&clip, "Paste").is_some());
    close_menus(&mut h);
    close_menus(&mut h);
    // a restart keeps the choice
    let h2 = restart(&h);
    assert!(!toolbar::fixed(&h2.app).contains(&actions::ActionId::Paste));
    // icons only by default, icons and text on request; the tooltip is the same
    let id = actions::ActionId::Split;
    let icon_only = toolbar::text(&h.app, id);
    let tip = action_tooltip(&h.ctx, &h.app, id);
    run(&mut h, Command::SetToolbarLabels(true));
    assert_eq!(icon_only, toolbar::icon(&h.app, id));
    assert!(toolbar::text(&h.app, id).ends_with("Split"));
    assert_eq!(action_tooltip(&h.ctx, &h.app, id), tip);
    // Reset toolbar brings Paste back
    run(&mut h, Command::ResetToolbar);
    assert!(toolbar::fixed(&h.app).contains(&actions::ActionId::Paste));
    assert!(on_toolbar(&h, "Paste"));
}

fn press(h: &mut Harness, key: Key, m: Modifiers) {
    h.frame(vec![Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    }]);
    h.idle(2);
}

// covers: REQ-242
#[test]
fn a_shortcut_is_rebound_a_conflict_is_reported_and_reset_restores_it() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let split = actions::ActionId::Split;
    let default = keymap::shortcut_text(&h.ctx, split).unwrap();
    // Help > Keyboard shortcuts: Change next to Split
    run(&mut h, Command::OpenPopup(Popup::Shortcuts));
    let change = h
        .nodes
        .iter()
        .find(|n| n.label == "Change Split at playhead")
        .cloned()
        .expect("a Change button");
    h.click(change.rect.center());
    assert_eq!(h.app.rebind.as_ref().map(|r| r.0.as_str()), Some("Split"));
    // a key that is used: reported, the change waits
    press(&mut h, Key::O, Modifiers::COMMAND);
    let (_, conflict) = h.app.rebind.clone().expect("still waiting");
    assert_eq!(conflict.as_deref(), Some("Open…"));
    assert!(h.has("Those keys are already used by Open…"));
    assert!(h.app.settings.interface.shortcuts.is_empty());
    // free keys: the binding changes and works from the keyboard and shows in the menu
    press(&mut h, Key::B, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.app.rebind.is_none());
    assert_eq!(h.app.settings.interface.shortcuts["Split"], "Cmd+Shift+B");
    run(&mut h, Command::ClosePopup(Popup::Shortcuts));
    let now_text = keymap::shortcut_text(&h.ctx, split).unwrap();
    assert_ne!(now_text, default);
    let clips = h.app.project.timeline.clips.len();
    h.app
        .dispatch(
            Command::Seek(crate::time::Micros(1_000_000)),
            Instant::now(),
        )
        .unwrap();
    press(&mut h, Key::B, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(
        h.app.project.timeline.clips.len(),
        clips + 1,
        "the new keys split"
    );
    let menu = open_menu(&mut h, "Edit");
    let clip = open_submenu(&mut h, &menu, "Clip");
    let item = find_in(&clip, "Split").unwrap();
    assert!(item.label.ends_with(&now_text), "{}", item.label);
    close_menus(&mut h);
    close_menus(&mut h);
    // the old keys no longer split, and the binding survives a restart
    assert!(
        keymap::action_for_in(
            &Event::Key {
                key: Key::B,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            },
            &keymap::current(&h.ctx)
        )
        .is_none()
    );
    let h2 = restart(&h);
    assert_eq!(
        keymap::overrides(&h2.app.settings.interface.shortcuts).len(),
        1
    );
    // Reset shortcuts restores them; Esc gives a change up
    run(&mut h, Command::ResetShortcuts);
    assert_eq!(keymap::shortcut_text(&h.ctx, split).unwrap(), default);
    run(&mut h, Command::StartRebind("Split".into()));
    press(&mut h, Key::Escape, Modifiers::NONE);
    assert!(h.app.rebind.is_none());
}

// covers: REQ-242
#[test]
fn shortcuts_are_written_and_read_back() {
    use eframe::egui::KeyboardShortcut;
    for s in [
        KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::B),
        KeyboardShortcut::new(Modifiers::ALT, Key::F5),
        KeyboardShortcut::new(Modifiers::NONE, Key::Space),
    ] {
        assert_eq!(keymap::parse_shortcut(&keymap::shortcut_name(&s)), Some(s));
    }
    for bad in ["", "Ctrl+", "Hyper+B", "Cmd+Nokey", "+"] {
        assert_eq!(keymap::parse_shortcut(bad), None, "{bad:?}");
    }
}

// covers: REQ-235
#[test]
fn a_popup_opens_where_it_was_left_and_an_off_screen_place_falls_back() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    run(&mut h, Command::PopupMoved("Fades".into(), (300.0, 200.0)));
    run(&mut h, Command::OpenPopup(Popup::Fades));
    let title = h
        .nodes
        .iter()
        .find(|n| n.label.starts_with("Fade"))
        .cloned()
        .unwrap_or_else(|| panic!("{:#?}", h.labels()));
    assert!(
        title.rect.min.x >= 299.0 && title.rect.min.x < 700.0,
        "{:?}",
        title.rect
    );
    let h2 = restart(&h);
    assert_eq!(h2.app.settings.interface.popups["Fades"], (300.0, 200.0));
    // off-screen: the default place
    run(&mut h, Command::ClosePopup(Popup::Fades));
    run(&mut h, Command::PopupMoved("Fades".into(), (9000.0, 200.0)));
    run(&mut h, Command::OpenPopup(Popup::Fades));
    let title = h
        .nodes
        .iter()
        .find(|n| n.label.starts_with("Fade"))
        .cloned()
        .unwrap_or_else(|| panic!("{:#?}", h.labels()));
    assert!(h.screen().contains_rect(title.rect), "{:?}", title.rect);
}

// covers: REQ-243
#[test]
fn the_theme_changes_every_section_and_popup() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    run(&mut h, Command::OpenPopup(Popup::Fades));
    run(&mut h, Command::SetTheme(Theme::Dark));
    assert!(h.ctx.global_style().visuals.dark_mode);
    let dark = h.shapes;
    run(&mut h, Command::SetTheme(Theme::Light));
    assert!(!h.ctx.global_style().visuals.dark_mode);
    assert!(dark > 0 && h.shapes > 0);
    // View > Theme lists the three choices
    let menu = open_menu(&mut h, "View");
    let sub = open_submenu(&mut h, &menu, "Theme");
    for t in ["System", "Light", "Dark"] {
        assert!(find_in(&sub, t).is_some(), "{t}");
    }
}

// covers: REQ-244
#[test]
fn larger_text_keeps_every_control_reachable_at_800_by_500() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 800.0, 500.0);
    let height = |h: &Harness| h.find("Media").unwrap().rect.height();
    let normal = height(&h);
    run(&mut h, Command::SetTextSize(TextSize::Larger));
    let larger = height(&h);
    assert!(larger / normal > 1.3, "{larger} / {normal}");
    let bad = reachable(&h, true);
    assert!(bad.is_empty(), "{bad:#?}");
    run(&mut h, Command::OpenPopup(Popup::Settings));
    let bad = reachable(&h, false);
    assert!(bad.is_empty(), "{bad:#?}");
}

// covers: REQ-246
#[test]
fn every_toolbar_command_and_popup_button_has_an_accessible_name() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    for id in toolbar::actions(&h.app) {
        let name = toolbar::label(&h.app, id);
        let n = h
            .nodes
            .iter()
            .find(|n| n.raw == name && n.role == eframe::egui::accesskit::Role::Button);
        assert!(
            n.is_some(),
            "toolbar {id:?} has no accessible name {name:?}"
        );
    }
    for p in [
        Popup::Settings,
        Popup::Fades,
        Popup::Shortcuts,
        Popup::ClipDetails,
    ] {
        run(&mut h, Command::OpenPopup(p));
        let unnamed: Vec<_> = h
            .nodes
            .iter()
            .filter(|n| n.role == eframe::egui::accesskit::Role::Button && n.interactive)
            .filter(|n| n.label.trim().is_empty())
            .collect();
        assert!(unnamed.is_empty(), "{p:?}: {unnamed:#?}");
        run(&mut h, Command::ClosePopup(p));
    }
}
