// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of the icons (1.6.0, REQ-142): every control on the lists has its icon.
//! The icon is a character of the label, so it greys out together with the text.

use super::ux_tests::{Harness, Node};
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Instance, Modal, Popup};
use crate::ui::actions::ActionId;
use crate::ui::menu::{Item, menus};
use eframe::egui::{Key, Modifiers};

fn setup() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    (dir, app)
}

fn has_icon(n: &Node, icon: &str) -> bool {
    n.raw.starts_with(icon) && n.raw[icon.len()..].starts_with(' ')
}

// covers: REQ-142
#[test]
fn every_menu_item_and_toolbar_button_shows_its_icon() {
    let (_d, app) = setup();
    let mut h = Harness::new(app, 1280.0, 800.0);
    // the toolbar
    let all = crate::ui::toolbar::actions(&h.app);
    for id in &all {
        let text = crate::ui::toolbar::text(&h.app, *id);
        let n = h
            .nodes
            .iter()
            .find(|n| n.raw == text)
            .unwrap_or_else(|| panic!("toolbar {id:?} {text:?}: {:#?}", h.labels()));
        assert!(has_icon(n, crate::ui::toolbar::icon(&h.app, *id)), "{id:?}");
    }
    // every menu, top level and the two submenus of Effects
    for (title, items) in menus() {
        h.click_label(title);
        for item in &items {
            match item {
                Item::Action(id) => {
                    let label = id.label(&h.app);
                    let n = h
                        .nodes
                        .iter()
                        .find(|n| n.label.starts_with(&label) && n.raw != n.label)
                        .unwrap_or_else(|| panic!("{title} > {label}: {:#?}", h.labels()));
                    assert!(has_icon(n, id.icon()), "{title} > {label}: {:?}", n.raw);
                }
                Item::Submenu(name, ids) => {
                    h.click_label(name);
                    for id in *ids {
                        let label = id.label(&h.app);
                        let n = h
                            .nodes
                            .iter()
                            .find(|n| n.label.starts_with(&label) && n.raw != n.label)
                            .unwrap_or_else(|| panic!("{name} > {label}"));
                        assert!(has_icon(n, id.icon()));
                    }
                }
                Item::OpenRecent | Item::Separator => {}
            }
        }
        h.key(Key::Escape, Modifiers::NONE);
        h.key(Key::Escape, Modifiers::NONE);
    }
}

// covers: REQ-142
#[test]
fn a_greyed_out_command_keeps_its_icon_and_is_greyed_with_it() {
    let (_d, app) = setup();
    let h = Harness::new(app, 1280.0, 800.0);
    // Redo has nothing to redo: the button is disabled, its icon is part of its text
    let text = crate::ui::toolbar::text(&h.app, ActionId::Redo);
    let n = h.nodes.iter().find(|n| n.raw == text).unwrap();
    assert!(n.disabled && has_icon(n, ActionId::Redo.icon()));
}

// covers: REQ-142
#[test]
fn every_popup_and_dialog_title_has_an_icon() {
    let (_d, mut app) = setup();
    let line = {
        app.dispatch(Command::Seek(Micros(1_000_000)), Instant::now())
            .unwrap();
        app.dispatch(Command::AddSubtitle, Instant::now()).unwrap();
        app.project.subtitles.project(&app.project.timeline)[0].clone()
    };
    let mut h = Harness::new(app, 1280.0, 800.0);
    for p in Popup::ALL {
        h.app.popups.open = vec![p];
        h.app.popups.instance = Some(Instance::Line(line.line, line.clip));
        h.idle(3);
        let icon = crate::ui::popups::popup_icon(&h.app, p);
        let title = format!("{icon} {}", h.app.popup_title(p));
        assert!(
            h.nodes.iter().any(|n| n.raw == title),
            "{p:?}: {title:?} in {:#?}",
            h.labels()
        );
    }
    // the Instance popup has an icon for every kind
    for i in [
        Instance::FadeStart,
        Instance::Aspect,
        Instance::Scale(crate::timeline::FrameTarget::Whole),
        Instance::Move(crate::timeline::FrameTarget::Whole),
        Instance::Lut(crate::timeline::FrameTarget::Whole),
    ] {
        h.app.popups.open = vec![Popup::Instance];
        h.app.popups.instance = Some(i);
        assert!(
            crate::ui::popups::popup_icon(&h.app, Popup::Instance)
                .chars()
                .count()
                == 1
        );
    }
    // dialogs: the notice and the unsaved-changes question
    h.app.popups.open.clear();
    h.app.notice = Some("hello".into());
    h.idle(3);
    assert!(
        h.nodes
            .iter()
            .any(|n| n.label == "Notice" && has_icon(n, icons::INFO))
    );
    h.app.notice = None;
    h.app.modal = Some(Modal::Unsaved(crate::app::PendingAction::New));
    h.idle(3);
    assert!(
        h.nodes
            .iter()
            .any(|n| n.label == "Unsaved changes" && has_icon(n, icons::FLOPPY_DISK)),
        "{:#?}",
        h.labels()
    );
}

// covers: REQ-142
#[test]
fn every_marker_type_and_effect_row_has_an_icon() {
    use crate::ui::markers::{MarkerKind, icon_of};
    for k in [
        MarkerKind::Fade,
        MarkerKind::EdgeFade,
        MarkerKind::Line,
        MarkerKind::ClipVolume,
        MarkerKind::CleanAudio,
        MarkerKind::Scale,
        MarkerKind::Move,
        MarkerKind::Lut,
        MarkerKind::AddFade,
        MarkerKind::Pov,
    ] {
        assert!(icons::ALL.contains(&icon_of(k)), "{k:?}");
    }
    for e in crate::app::EffectKind::ALL {
        assert!(
            icons::ALL.contains(&crate::ui::effects::effect_icon(e)),
            "{e:?}"
        );
    }
    // every action has an icon from the font (the match is exhaustive: a new action
    // without one does not compile)
    for a in crate::ui::actions::ACTIONS {
        assert!(icons::ALL.contains(&a.id.icon()), "{:?}", a.id);
    }
}
