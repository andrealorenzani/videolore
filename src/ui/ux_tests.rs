// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of the 1.1 window, through egui's accessibility tree (AccessKit): every
//! widget with its role, label, bounds and enabled state, as a screen reader would see it.
//! Clicks and drags are real pointer events at the widgets' positions.

use super::*;
use crate::app::tests::{FakeDialogs, files, import, settle, state};
use crate::app::{Instance, Modal, PendingAction, Popup};
use eframe::egui::accesskit::{Action, Role};
use eframe::egui::{Event, Key, Modifiers, PointerButton, RawInput};

/// One widget of the accessibility tree.
#[derive(Debug, Clone)]
pub struct Node {
    pub role: Role,
    pub label: String,
    pub rect: Rect,
    pub disabled: bool,
    pub interactive: bool,
    /// The label as drawn, with its icon.
    pub raw: String,
}

/// A label without the icon (a private-use character and a space) that starts it.
pub fn without_icon(label: &str) -> String {
    let mut it = label.chars();
    match (it.next(), it.next()) {
        (Some(c), Some(' ')) if ('\u{e000}'..='\u{f8ff}').contains(&c) => it.collect(),
        _ => label.to_string(),
    }
}

/// A window driven frame by frame.
pub struct Harness {
    pub ctx: Context,
    pub app: AppState,
    pub cache: UiCache,
    pub size: Vec2,
    pub nodes: Vec<Node>,
    /// How many shapes the last frame painted.
    pub shapes: usize,
}

impl Harness {
    pub fn new(app: AppState, w: f32, h: f32) -> Harness {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut h = Harness {
            ctx,
            app,
            cache: UiCache::default(),
            size: vec2(w, h),
            nodes: Vec::new(),
            shapes: 0,
        };
        h.idle(3);
        h
    }

    pub fn screen(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, self.size)
    }

    /// Runs one frame with `events`; keeps its accessibility tree in `nodes`.
    pub fn frame_with(&mut self, raw: RawInput) {
        let now = Instant::now();
        let (app, cache) = (&mut self.app, &mut self.cache);
        let mut out = self.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
        self.nodes = out
            .platform_output
            .accesskit_update
            .take()
            .map(|u| {
                u.nodes
                    .into_iter()
                    .filter_map(|(_, n)| {
                        let b = n.bounds()?;
                        Some(Node {
                            role: n.role(),
                            raw: n.label().or(n.value()).unwrap_or_default().to_string(),
                            label: without_icon(n.label().or(n.value()).unwrap_or_default()),
                            rect: Rect::from_min_max(
                                pos2(b.x0 as f32, b.y0 as f32),
                                pos2(b.x1 as f32, b.y1 as f32),
                            ),
                            disabled: n.is_disabled(),
                            interactive: n.supports_action(Action::Click)
                                || n.supports_action(Action::Focus),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.shapes = out.shapes.len();
        out.textures_delta.clear();
        out.drop_without_applying_deltas();
    }

    pub fn frame(&mut self, events: Vec<Event>) {
        let raw = RawInput {
            screen_rect: Some(self.screen()),
            events,
            ..Default::default()
        };
        self.frame_with(raw);
    }

    pub fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(Vec::new());
        }
    }

    /// The first widget whose label starts with `label`.
    /// The widget labelled `label` (exact labels and interactive widgets first, then
    /// labels starting with it).
    pub fn find(&self, label: &str) -> Option<Node> {
        best(&self.nodes, label).cloned()
    }

    pub fn has(&self, label: &str) -> bool {
        self.find(label).is_some()
    }

    fn button(&mut self, p: Pos2, button: PointerButton) {
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(p), ev(true)]);
        self.frame(vec![ev(false)]);
        self.idle(2);
    }

    pub fn click(&mut self, p: Pos2) {
        self.button(p, PointerButton::Primary);
    }

    pub fn right_click(&mut self, p: Pos2) {
        self.button(p, PointerButton::Secondary);
    }

    pub fn double_click(&mut self, p: Pos2) {
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(p), ev(true)]);
        self.frame(vec![ev(false)]);
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
        self.idle(2);
    }

    /// Clicks the widget labelled `label`; panics when there is none.
    pub fn click_label(&mut self, label: &str) {
        let n = self
            .find(label)
            .unwrap_or_else(|| panic!("no widget \"{label}\" in {:#?}", self.labels()));
        self.click(n.rect.center());
    }

    pub fn drag(&mut self, from: Pos2, to: Pos2) {
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![ev(from, true)]);
        for i in 1..=5 {
            let p = from + (to - from) * (i as f32 / 5.0);
            self.frame(vec![Event::PointerMoved(p)]);
        }
        self.frame(vec![ev(to, false)]);
        self.idle(2);
    }

    /// Presses at `from` and moves to `to` without releasing (a drag in progress).
    pub fn press_and_move(&mut self, from: Pos2, to: Pos2) {
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![ev(from, true)]);
        for i in 1..=5 {
            let p = from + (to - from) * (i as f32 / 5.0);
            self.frame(vec![Event::PointerMoved(p)]);
        }
    }

    /// Releases the primary button at `at`.
    pub fn release(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.idle(2);
    }

    pub fn key(&mut self, k: Key, m: Modifiers) {
        self.frame(vec![Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: m,
        }]);
        self.idle(1);
    }

    pub fn labels(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|n| !n.label.is_empty())
            .map(|n| format!("{:?} {} {:?}", n.role, n.label, n.rect))
            .collect()
    }
}

/// Two 5 s clips with a fade between them.
pub(super) fn loaded() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    (dir, app)
}

/// Widgets of `after` that were not in `before` (what a click opened).
pub(super) fn opened(before: &[Node], after: &[Node]) -> Vec<Node> {
    after
        .iter()
        .filter(|n| {
            !before
                .iter()
                .any(|b| b.label == n.label && b.rect == n.rect && b.role == n.role)
        })
        .cloned()
        .collect()
}

fn best<'a>(nodes: &'a [Node], label: &str) -> Option<&'a Node> {
    nodes
        .iter()
        .filter(|n| n.label.starts_with(label) && n.rect.width() > 0.0)
        .min_by_key(|n| (n.label != label, !n.interactive, n.role == Role::TextRun))
}

pub(super) fn find_in<'a>(nodes: &'a [Node], label: &str) -> Option<&'a Node> {
    best(nodes, label)
}

/// Opens the menu `title`; returns its widgets.
pub(super) fn open_menu(h: &mut Harness, title: &str) -> Vec<Node> {
    let before = h.nodes.clone();
    let t = h
        .nodes
        .iter()
        .find(|n| n.label == title && n.role == Role::Button && n.rect.max.y <= 26.0)
        .cloned()
        .unwrap_or_else(|| panic!("no menu {title}"));
    h.click(t.rect.center());
    opened(&before, &h.nodes)
}

/// Opens the submenu `name` of the menu whose widgets are `menu`; returns its widgets.
pub(super) fn open_submenu(h: &mut Harness, menu: &[Node], name: &str) -> Vec<Node> {
    let sub = find_in(menu, name)
        .unwrap_or_else(|| panic!("no submenu {name}"))
        .clone();
    let before = h.nodes.clone();
    h.frame(vec![Event::PointerMoved(sub.rect.center())]);
    h.click(sub.rect.center());
    opened(&before, &h.nodes)
}

pub(super) fn close_menus(h: &mut Harness) {
    h.key(Key::Escape, Modifiers::NONE);
    h.idle(1);
}

// covers: REQ-069, REQ-025
#[test]
fn menus_list_every_item_with_shortcut_and_availability() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let titles: Vec<&str> = menu::menus().iter().map(|m| m.0).collect();
    assert_eq!(
        titles,
        ["File", "Edit", "Playback", "Effects", "View", "Help"]
    );
    let mut listed = Vec::new();
    for (title, items) in menu::menus() {
        let mut nodes = open_menu(&mut h, title);
        for item in items {
            match item {
                menu::Item::Action(id) => {
                    listed.push(id);
                    let label = id.label(&h.app);
                    let n = find_in(&nodes, &label)
                        .unwrap_or_else(|| panic!("{title} > {label} missing: {nodes:#?}"));
                    let enabled = h.app.is_enabled(&id.command(&h.app));
                    assert_eq!(n.disabled, !enabled, "{title} > {label}");
                    if let Some(s) = keymap::shortcut_text(&h.ctx, id) {
                        assert!(n.label.ends_with(&s), "{title} > {label}: {}", n.label);
                    }
                }
                menu::Item::OpenRecent => {
                    assert!(find_in(&nodes, "Open recent").is_some());
                }
                menu::Item::Sections => assert!(find_in(&nodes, "Sections").is_some()),
                menu::Item::Workspaces => assert!(find_in(&nodes, "Workspaces").is_some()),
                menu::Item::ThemeMenu => assert!(find_in(&nodes, "Theme").is_some()),
                menu::Item::ToolbarMenu => {
                    assert!(find_in(&nodes, "Customise toolbar").is_some());
                }
                menu::Item::Submenu(name, ids) => {
                    // "Apply to selected clip ▸": its items, with their availability
                    let sub = find_in(&nodes, name)
                        .unwrap_or_else(|| panic!("{title} > {name} missing"))
                        .clone();
                    let before = h.nodes.clone();
                    h.frame(vec![Event::PointerMoved(sub.rect.center())]);
                    h.click(sub.rect.center());
                    let inner = opened(&before, &h.nodes);
                    for id in ids {
                        listed.push(*id);
                        let label = id.label(&h.app);
                        let n = find_in(&inner, &label)
                            .unwrap_or_else(|| panic!("{name} > {label} missing: {inner:#?}"));
                        let enabled = h.app.is_enabled(&id.command(&h.app));
                        assert_eq!(n.disabled, !enabled, "{name} > {label}");
                    }
                    close_menus(&mut h);
                    if !h.app.popups.open.is_empty() || egui::Popup::is_any_open(&h.ctx) {
                        close_menus(&mut h);
                    }
                    nodes = open_menu(&mut h, title);
                }
                menu::Item::Separator => {}
            }
        }
        close_menus(&mut h);
    }
    // every action of the table is in a menu
    for a in actions::ACTIONS {
        assert!(listed.contains(&a.id), "{} is in no menu", a.label);
    }
    // the menu items name the undo step and run their command
    let nodes = open_menu(&mut h, "Edit");
    let history = open_submenu(&mut h, &nodes, "History");
    assert!(find_in(&history, "Undo Add fade").is_some());
    close_menus(&mut h);
    close_menus(&mut h);
    let nodes = open_menu(&mut h, "Edit");
    let clip = open_submenu(&mut h, &nodes, "Clip");
    let n = find_in(&clip, "Clip details…").unwrap().clone();
    h.click(n.rect.center());
    assert!(h.app.popups.is_open(Popup::ClipDetails));
    let nodes = open_menu(&mut h, "Help");
    let n = find_in(&nodes, "Keyboard shortcuts").unwrap().clone();
    h.click(n.rect.center());
    assert!(h.app.popups.is_open(Popup::Shortcuts));
    let nodes = open_menu(&mut h, "Effects");
    let n = find_in(&nodes, "Fades…").unwrap().clone();
    h.click(n.rect.center());
    assert!(h.app.popups.is_open(Popup::Fades));
    // Open recent: the list (empty) and Clear list
    let dir = tempfile::tempdir().unwrap();
    let proj = dir.path().join("recent.vlproj");
    h.app
        .dispatch(Command::SaveProjectAs(proj.clone()), Instant::now())
        .unwrap();
    let before = h.nodes.clone();
    h.click_label("File");
    let recent = h.find("Open recent").unwrap();
    h.frame(vec![Event::PointerMoved(recent.rect.center())]);
    h.click(recent.rect.center());
    let nodes = opened(&before, &h.nodes);
    let item = find_in(&nodes, "recent.vlproj").unwrap_or_else(|| panic!("{nodes:#?}"));
    assert!(!item.disabled);
    assert!(find_in(&nodes, "Clear list").is_some());
    close_menus(&mut h);
    close_menus(&mut h);
}

// covers: REQ-069, REQ-068
#[test]
fn a_menu_taller_than_the_window_scrolls() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 800.0, 220.0);
    let nodes = open_menu(&mut h, "View");
    let last = find_in(&nodes, "Customise toolbar").unwrap();
    assert!(last.rect.max.y > h.size.y, "the menu is taller: {last:?}");
    let bad: Vec<_> = reachable(&h, false)
        .into_iter()
        .filter(|n| nodes.iter().any(|m| m.rect == n.rect))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

// covers: REQ-070, REQ-072
#[test]
fn every_toolbar_command_can_be_triggered_at_800_pixels() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    app.dispatch(Command::Seek(Micros(2_000_000)), now).unwrap();
    app.dispatch(Command::Split, now).unwrap();
    let mut h = Harness::new(app, 800.0, 500.0);
    let visible = visible_toolbar(&h);
    for id in &visible {
        let n = find_in(&h.nodes, &toolbar::label(&h.app, *id)).unwrap();
        assert!(h.screen().contains_rect(n.rect), "{id:?}");
    }
    if visible.len() == 12 {
        assert!(!h.has("»"), "no overflow menu when everything fits");
        // narrower than any real window: the rest goes into the "»" menu
        h.size = vec2(260.0, 500.0);
        h.idle(2);
        let v = visible_toolbar(&h);
        return overflow(h, v);
    }
    overflow(h, visible);
}

/// Toolbar actions shown as buttons.
fn visible_toolbar(h: &Harness) -> Vec<actions::ActionId> {
    let toolbar_y = |n: &Node| n.rect.min.y > 22.0 && n.rect.max.y < 60.0;
    toolbar::actions(&h.app)
        .iter()
        .filter(|id| {
            let l = toolbar::label(&h.app, **id);
            h.nodes
                .iter()
                .any(|n| n.label == l && n.role == Role::Button && toolbar_y(n))
        })
        .copied()
        .collect()
}

/// The toolbar commands that do not fit are in the "»" menu and run from there.
fn overflow(mut h: Harness, visible: Vec<actions::ActionId>) {
    assert!(visible.len() < 12, "{visible:?}");
    let more = h
        .nodes
        .iter()
        .find(|n| n.label == "»")
        .cloned()
        .expect("overflow menu");
    assert!(h.screen().contains_rect(more.rect));
    let before = h.nodes.clone();
    h.click(more.rect.center());
    let menu_nodes = opened(&before, &h.nodes);
    for id in toolbar::actions(&h.app) {
        if visible.contains(&id) {
            continue;
        }
        let n = find_in(&menu_nodes, &id.label(&h.app))
            .unwrap_or_else(|| panic!("{id:?} unreachable: {menu_nodes:#?}"));
        assert!(h.screen().contains_rect(n.rect));
        assert_eq!(n.disabled, !h.app.is_enabled(&id.command(&h.app)));
    }
    // a command from the overflow menu (or the toolbar) runs
    let n = h.app.project.timeline.clips.len();
    let undo = h.app.undo_text();
    match find_in(&menu_nodes, &undo) {
        Some(item) => {
            let p = item.rect.center();
            h.click(p);
        }
        None => {
            close_menus(&mut h);
            h.click_label("Undo");
        }
    }
    assert_eq!(h.app.project.timeline.clips.len(), n - 1);
    // greyed buttons say why (Paste: nothing copied)
    assert_eq!(
        h.app.disabled_reason(&Command::Paste),
        Some("Copy or cut a clip first")
    );
    let tip = action_tooltip(&h.ctx, &h.app, actions::ActionId::Split);
    assert!(
        tip.starts_with("Cut the clip in two") && tip.contains('B'),
        "{tip}"
    );
}

// covers: REQ-066, REQ-067, REQ-072
#[test]
fn dragging_a_border_resizes_live_and_saves_once_on_release() {
    let (dir, mut app) = loaded();
    let settings_file = dir.path().join("settings.json");
    app.settings_path = Some(settings_file.clone());
    let mut h = Harness::new(app, 1280.0, 800.0);
    let r0 = h.cache.sections.unwrap();
    let (s, rect) = r0.splitters[0];
    assert_eq!(s, layout::Splitter::MediaPreview);
    assert!(h.has(s.tooltip()), "the border has a tooltip");
    let from = rect.center();
    let ev = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(vec![Event::PointerMoved(from), ev(from, true)]);
    for i in 1..=5 {
        h.frame(vec![Event::PointerMoved(from + vec2(20.0 * i as f32, 0.0))]);
        assert!(h.cache.layout_drag.is_some(), "live while dragging");
        assert!(!settings_file.exists(), "nothing saved while dragging");
    }
    let live = h.cache.sections.unwrap();
    assert!((live.media.width() - r0.media.width() - 80.0).abs() <= 1.0);
    h.frame(vec![ev(from + vec2(100.0, 0.0), false)]);
    h.idle(2);
    assert!(h.cache.layout_drag.is_none());
    let saved = crate::app::tests::read_settings(&settings_file);
    assert_eq!(saved.layout, h.app.settings.layout);
    let r1 = h.cache.sections.unwrap();
    assert!((r1.media.width() - r0.media.width() - 100.0).abs() <= 1.0);
    assert!((r1.preview.width() - r0.preview.width() + 100.0).abs() <= 1.0);
    assert_eq!(r1.timeline, r0.timeline);
    assert_eq!(r1.effects, r0.effects);
    // the rows border, upwards
    let rows = r1.splitters[2].1.center();
    h.drag(rows, rows - vec2(0.0, 100.0));
    let r2 = h.cache.sections.unwrap();
    assert!((r2.media.height() - r1.media.height() + 100.0).abs() <= 1.0);
    assert!((r2.timeline.height() - r1.timeline.height() - 100.0).abs() <= 1.0);
    // View > Reset layout
    let nodes = open_menu(&mut h, "View");
    let n = find_in(&nodes, "Reset layout").unwrap().clone();
    h.click(n.rect.center());
    assert_eq!(h.app.settings.layout, None);
    assert_eq!(h.cache.sections.unwrap(), r0);
}

fn lane_click(h: &mut Harness, t: Micros) {
    let g = h.cache.timeline.unwrap();
    h.click(pos2(g.x_of(t), g.lane.center().y));
}

// covers: REQ-077, REQ-011, REQ-072
#[test]
fn clicking_markers_opens_their_settings() {
    let (_d, app) = loaded();
    let c: Vec<_> = app.project.timeline.clips.iter().map(|c| c.id).collect();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let five = Micros(5_000_000);
    lane_click(&mut h, five);
    let fade = Instance::Fade(c[0], c[1]);
    assert_eq!(h.app.popups.instance, Some(fade));
    assert!(h.app.popups.is_open(Popup::Instance));
    assert_eq!(h.app.playhead, five);
    assert_eq!(
        h.app.project.timeline.fades.len(),
        1,
        "a click never removes"
    );
    h.idle(1);
    // Remove: the fade marker becomes an "Add fade" handle; undo brings it back
    h.click_label("Remove");
    assert!(h.app.project.timeline.fades.is_empty());
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.project.timeline.fades.len(), 1);
    h.app
        .dispatch(
            Command::SetFade {
                left: c[0],
                right: c[1],
                on: false,
            },
            Instant::now(),
        )
        .unwrap();
    h.idle(1);
    lane_click(&mut h, five);
    assert_eq!(
        h.app.project.timeline.fades.len(),
        1,
        "the + handle adds it"
    );
    assert_eq!(h.app.undo_text(), "Undo Add fade");
    // a subtitle line starting on the fade: both are listed by the chooser
    h.app.dispatch(Command::Seek(five), Instant::now()).unwrap();
    h.app
        .dispatch(Command::AddSubtitle, Instant::now())
        .unwrap();
    let line = h.app.selection.primary_line().unwrap();
    h.idle(1);
    lane_click(&mut h, five);
    let choices = h.app.popups.chooser.clone().expect("chooser");
    assert_eq!(choices.len(), 2);
    assert!(choices.contains(&fade));
    assert!(choices.contains(&Instance::Line(line, c[1])));
    h.idle(1);
    let entry = super::popups::instance_label(&h.app, Instance::Line(line, c[1]));
    h.click_label(&entry);
    assert_eq!(h.app.popups.instance, Some(Instance::Line(line, c[1])));
    assert_eq!(h.app.popups.chooser, None);
    // the line's popup shows its text; editing it is one undo step
    h.idle(1);
    assert!(
        h.nodes.iter().any(|n| n.label == "New subtitle"),
        "{:#?}",
        h.labels()
    );
    // a click on an empty part of the lane moves the playhead
    lane_click(&mut h, Micros(8_000_000));
    assert_eq!(h.app.playhead, Micros(8_000_000));
}

// covers: REQ-078, REQ-009, REQ-019
#[test]
fn double_clicking_a_clip_opens_its_details() {
    let (_d, mut app) = loaded();
    app.dispatch(
        Command::PlaceMusic(Some(app.project.media[2].id)),
        Instant::now(),
    )
    .unwrap();
    app.dispatch(Command::SelectMusic(None), Instant::now())
        .unwrap();
    let first = app.project.timeline.clips[0];
    let mut h = Harness::new(app, 1280.0, 800.0);
    let g = h.cache.timeline.unwrap();
    h.double_click(pos2(g.x_of(Micros(1_000_000)), g.video.center().y));
    assert!(h.app.popups.is_open(Popup::ClipDetails));
    assert_eq!(h.app.selected_clip, Some(first.id));
    h.idle(1);
    assert!(h.has("Source: a.mp4"), "{:#?}", h.labels());
    // the trim buttons do what the REQ-009 commands do: "+1f" on the start
    let plus = h
        .nodes
        .iter()
        .filter(|n| n.label == "+1f")
        .min_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y))
        .cloned()
        .unwrap();
    h.click(plus.rect.center());
    let frame = h.app.fps().frame_duration();
    assert_eq!(h.app.project.timeline.clips[0].src_in, frame);
    // music clip: its own section
    let m = h.app.project.music.clips[0].id;
    let g = h.cache.timeline.unwrap();
    h.double_click(pos2(g.x_of(Micros(2_000_000)), g.music.center().y));
    assert_eq!(h.app.selected_music, Some(m));
    h.idle(1);
    assert!(h.has("Move to playhead"));
    h.click_label("Move to playhead");
    assert_eq!(h.app.project.music.get(m).unwrap().start, h.app.playhead);
}

// covers: REQ-079
#[test]
fn right_click_menus_on_clips_markers_and_the_empty_timeline() {
    let (_d, mut app) = loaded();
    app.dispatch(
        Command::PlaceMusic(Some(app.project.media[2].id)),
        Instant::now(),
    )
    .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let g = h.cache.timeline.unwrap();
    let on_clip = pos2(g.x_of(Micros(1_000_000)), g.video.center().y);
    let before = h.nodes.clone();
    h.right_click(on_clip);
    let menu_nodes = opened(&before, &h.nodes);
    for (label, enabled) in [
        ("Split here", true),
        ("Cut", true),
        ("Copy", true),
        ("Paste after", false),
        ("Delete", true),
        ("Merge with next", false),
        ("Remove fade after", true),
        ("Clip details…", true),
    ] {
        let n = find_in(&menu_nodes, label).unwrap_or_else(|| panic!("{label}: {menu_nodes:#?}"));
        assert_eq!(n.disabled, !enabled, "{label}");
    }
    let copy = find_in(&menu_nodes, "Copy").unwrap().rect.center();
    h.click(copy);
    assert!(h.app.clipboard.is_some());
    // now Paste after is available
    let before = h.nodes.clone();
    h.right_click(pos2(g.x_of(Micros(7_000_000)), g.video.center().y));
    let menu_nodes = opened(&before, &h.nodes);
    assert!(!find_in(&menu_nodes, "Paste after").unwrap().disabled);
    assert!(find_in(&menu_nodes, "Remove fade before").is_some());
    let paste = find_in(&menu_nodes, "Paste after").unwrap().rect.center();
    h.click(paste);
    assert_eq!(h.app.project.timeline.clips.len(), 3);
    // a marker
    let before = h.nodes.clone();
    h.right_click(pos2(g.x_of(Micros(5_000_000)), g.lane.center().y));
    let menu_nodes = opened(&before, &h.nodes);
    assert!(
        find_in(&menu_nodes, "Open settings").is_some(),
        "{menu_nodes:#?}"
    );
    let remove = find_in(&menu_nodes, "Remove").unwrap().rect.center();
    h.click(remove);
    assert!(h.app.project.timeline.fades.is_empty());
    // the empty timeline after the clips
    let g = h.cache.timeline.unwrap();
    let before = h.nodes.clone();
    h.right_click(pos2(g.x_of(Micros(15_000_000)) + 30.0, g.video.center().y));
    let menu_nodes = opened(&before, &h.nodes);
    assert!(!find_in(&menu_nodes, "Paste").unwrap().disabled);
    assert!(
        find_in(&menu_nodes, "Add subtitle line here")
            .unwrap()
            .disabled
    );
    close_menus(&mut h);
    // a music clip
    let before = h.nodes.clone();
    h.right_click(pos2(g.x_of(Micros(1_000_000)), g.music.center().y));
    let menu_nodes = opened(&before, &h.nodes);
    for label in ["Split", "Delete", "Move to playhead", "Clip details…"] {
        assert!(find_in(&menu_nodes, label).is_some(), "{label}");
    }
    let del = find_in(&menu_nodes, "Delete").unwrap().rect.center();
    h.click(del);
    assert!(h.app.project.music.clips.is_empty());
}

// covers: REQ-071, REQ-076
#[test]
fn keyboard_shortcuts_run_and_esc_closes_the_topmost_popup() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    app.dispatch(Command::OpenPopup(Popup::Fades), now).unwrap();
    app.dispatch(Command::OpenPopup(Popup::Volume), now)
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.app.popups.open, vec![Popup::Fades]);
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.app.popups.open.is_empty());
    // egui's own keyboard zoom is off: Ctrl/Cmd + = zooms the timeline
    let z = h.app.zoom;
    h.key(Key::Equals, Modifiers::COMMAND);
    assert!(h.app.zoom > z);
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    assert!(!h.ctx.options(|o| o.zoom_with_keyboard));
    h.key(Key::Minus, Modifiers::COMMAND);
    h.key(Key::Num0, Modifiers::COMMAND);
    h.idle(1);
    assert!((h.app.zoom - z).abs() < 1e-3, "fit again");
    // frame steps, start and end
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.app.playhead, h.app.fps().frame_duration());
    h.key(Key::End, Modifiers::NONE);
    assert_eq!(h.app.playhead, h.app.project.timeline.duration());
    h.key(Key::Home, Modifiers::NONE);
    assert_eq!(h.app.playhead, Micros::ZERO);
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.app.playhead, Micros::ZERO);
    // Delete removes the clip under the playhead
    h.key(Key::Delete, Modifiers::NONE);
    assert_eq!(h.app.project.timeline.clips.len(), 1);
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.project.timeline.clips.len(), 2);
    // F1: the shortcuts window
    h.key(Key::F1, Modifiers::NONE);
    assert!(h.app.popups.is_open(Popup::Shortcuts));
    // nothing fires while typing in a text field
    h.app
        .dispatch(Command::Seek(Micros(1_000_000)), Instant::now())
        .unwrap();
    h.app
        .dispatch(Command::AddSubtitle, Instant::now())
        .unwrap();
    let line = h.app.selection.primary_line().unwrap();
    let clip = h.app.project.timeline.clips[0].id;
    h.app
        .dispatch(
            Command::OpenInstance(Instance::Line(line, clip)),
            Instant::now(),
        )
        .unwrap();
    h.idle(2);
    let field = h
        .nodes
        .iter()
        .find(|n| n.role == Role::TextInput)
        .cloned()
        .unwrap_or_else(|| panic!("{:#?}", h.labels()));
    h.click(field.rect.center());
    assert!(h.ctx.text_edit_focused());
    let n = h.app.project.timeline.clips.len();
    h.key(Key::Delete, Modifiers::NONE);
    h.key(Key::B, Modifiers::COMMAND);
    h.key(Key::Space, Modifiers::NONE);
    assert_eq!(h.app.project.timeline.clips.len(), n);
    assert!(!h.app.preview.is_playing());
    // a blocking dialog also stops them
    h.ctx.memory_mut(|m| m.stop_text_input());
    h.app.modal = Some(Modal::About);
    h.key(Key::B, Modifiers::COMMAND);
    assert_eq!(h.app.project.timeline.clips.len(), n);
}

// covers: REQ-071, REQ-090, REQ-091
#[test]
fn the_shortcuts_window_lists_exactly_the_bindings() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 900.0);
    let before = h.nodes.clone();
    h.key(Key::F1, Modifiers::NONE);
    h.idle(2);
    let mut rows: Vec<Node> = opened(&before, &h.nodes)
        .into_iter()
        .filter(|n| n.role == Role::Label)
        .collect();
    rows.sort_by(|a, b| {
        (a.rect.min.y.round(), a.rect.min.x)
            .partial_cmp(&(b.rect.min.y.round(), b.rect.min.x))
            .unwrap()
    });
    let shown: Vec<String> = rows.iter().map(|n| n.label.clone()).collect();
    let mut want: Vec<String> = keymap::bindings()
        .into_iter()
        .flat_map(|(s, l)| [h.ctx.format_shortcut(&s), l.to_string()])
        .collect();
    // then the mouse gestures (Ctrl/Cmd+click, Shift+drag)
    want.extend(
        popups::GESTURES
            .iter()
            .flat_map(|(g, l)| [g.to_string(), l.to_string()]),
    );
    let start = shown
        .iter()
        .position(|l| *l == want[0])
        .unwrap_or_else(|| panic!("{shown:#?}"));
    assert_eq!(shown[start..start + want.len()], want[..]);
    assert_eq!(
        shown.len(),
        start + want.len(),
        "nothing else listed: {shown:#?}"
    );
}

/// A file dropped on the window.
#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl eframe::egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

// covers: REQ-081, REQ-028, REQ-233
#[test]
fn dropping_a_project_asks_about_unsaved_changes_then_opens_it() {
    let (dir, mut app) = loaded();
    let proj = dir.path().join("dropped.vlproj");
    let mut other = state(FakeDialogs::default());
    other
        .dispatch(Command::SaveProjectAs(proj.clone()), Instant::now())
        .unwrap();
    app.dirty = true;
    let mut h = Harness::new(app, 1280.0, 800.0);
    // files dragged over the window: a hint is drawn
    h.frame_with(RawInput {
        screen_rect: Some(h.screen()),
        hovered_files: vec![eframe::egui::HoveredFile {
            path: Some(proj.clone()),
            mime: String::new(),
        }],
        ..Default::default()
    });
    h.frame_with(RawInput {
        screen_rect: Some(h.screen()),
        dropped_files: vec![std::sync::Arc::new(Dropped(proj.clone()))],
        ..Default::default()
    });
    assert_eq!(
        h.app.modal,
        Some(Modal::Unsaved(PendingAction::Open(Some(proj.clone()))))
    );
    h.idle(2);
    h.click_label("Don't save");
    assert_eq!(h.app.project_path, Some(proj));
    // an empty media list invites to import or drop
    h.idle(1);
    assert!(h.has(media::EMPTY_HINT), "{:#?}", h.labels());
    assert!(h.has(media::ADD_MEDIA));
}

// covers: REQ-075, REQ-076
#[test]
fn the_effects_section_lists_the_effects_and_opens_their_popups() {
    let (_d, mut app) = loaded();
    app.dispatch(Command::Seek(Micros(1_000_000)), Instant::now())
        .unwrap();
    app.dispatch(Command::AddSubtitle, Instant::now()).unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let rows: Vec<_> = effects::rows().into_iter().collect();
    let names: Vec<&str> = rows.iter().map(|r| r.1).collect();
    // (REQ-130: the three picture effects follow Clean audio)
    assert_eq!(
        names,
        [
            "Fade IN/OUT",
            "Subtitles",
            "Volume and track",
            "Clean audio",
            "Change aspect",
            "Scale",
            "Move",
            "Apply LUT",
            "Change speed"
        ]
    );
    for (effect, name) in rows.into_iter().take(4) {
        // the row shows only the name (and its icon, REQ-142), with the tooltip explaining
        // how to apply it
        let n = h.find(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(n.label, name);
        assert!(
            n.raw.ends_with(&format!(" {name}"))
                && n.raw.chars().count() == name.chars().count() + 2
        );
        h.click(n.rect.center());
        assert!(h.app.popups.is_open(effect.popup()), "{name}");
    }
    h.idle(2);
    // each popup's content is there
    assert!(h.has("Add fade at the boundary nearest the playhead"));
    assert!(h.has("Remove all fades"));
    assert!(h.has("Add line at playhead"));
    assert!(h.has("00:00:01.000  New subtitle"), "{:#?}", h.labels());
    assert!(h.has("Apply to whole video"), "{:#?}", h.labels());
    // the fade list opens that fade (Volume, Clean audio and Subtitles closed: they cover it)
    h.key(Key::Escape, Modifiers::NONE);
    h.key(Key::Escape, Modifiers::NONE);
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.app.popups.open, vec![Popup::Fades]);
    h.idle(1);
    h.click_label("00:00:05.000  a.mp4 → b.mp4");
    assert!(matches!(h.app.popups.instance, Some(Instance::Fade(..))));
    // Remove all fades: one undo step (the fade's own popup is closed first: it is on top)
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.app.popups.open, vec![Popup::Fades]);
    h.click_label("Remove all fades");
    assert!(h.app.project.timeline.fades.is_empty());
    assert_eq!(h.app.undo_text(), "Undo Remove all fades");
}

/// Widgets that cannot be seen nor scrolled to: outside the window, or cut off by their
/// section without a scroll area to reach them.
pub(super) fn reachable(h: &Harness, check_sections: bool) -> Vec<Node> {
    let screen = h.screen().expand(1.0);
    let cs = &h.cache.containers;
    fn via(r: Rect, screen: Rect, cs: &[Container], depth: u8) -> bool {
        cs.iter().any(|c| {
            c.content.expand(1.0).contains_rect(r)
                && (screen.contains_rect(c.viewport)
                    || (depth > 0 && via(c.viewport, screen, cs, depth - 1)))
        })
    }
    let sections = h
        .cache
        .sections
        .map(|s| [s.media, s.preview, s.timeline, s.effects]);
    h.nodes
        .iter()
        .filter(|n| n.interactive && n.rect.width() > 0.0)
        .filter(|n| {
            if via(n.rect, screen, cs, 3) {
                return false;
            }
            if !screen.contains_rect(n.rect) {
                return true;
            }
            check_sections
                && sections.is_some_and(|s| {
                    s.iter().any(|r| {
                        r.contains(n.rect.center()) && !r.expand(1.0).contains_rect(n.rect)
                    })
                })
        })
        .cloned()
        .collect()
}

/// 50 media items (one with a 200-character name, on the timeline), 200 subtitle lines.
fn crowded() -> (tempfile::TempDir, AppState) {
    let (d, mut app) = loaded();
    let now = Instant::now();
    let base = app.project.media[0].clone();
    for i in 0..47 {
        let mut m = base.clone();
        m.id = app.project.new_media_id();
        m.name = if i == 0 {
            format!("{}.mp4", "a-very-long-file-name-".repeat(9))
        } else {
            format!("clip {i}.mp4")
        };
        app.project.add_media(m);
    }
    assert_eq!(app.project.media.len(), 50);
    let long = app.project.media[3].id;
    app.dispatch(Command::Append(Some(long)), now).unwrap();
    let lines = (0..200)
        .map(|i| crate::subtitles::NewLine {
            media: base.id,
            start: Micros(i * 20_000),
            end: Micros(i * 20_000 + 15_000),
            text: format!("line {i} with some words in it"),
            words: Vec::new(),
        })
        .collect();
    app.project.subtitles.replace_all(lines, Some("en".into()));
    assert_eq!(
        app.project.subtitles.project(&app.project.timeline).len(),
        200
    );
    (d, app)
}

// covers: REQ-068
#[test]
fn everything_is_reachable_at_the_smallest_window() {
    let (_d, app) = crowded();
    let mut h = Harness::new(app, 800.0, 500.0);
    let tiny = layout::Fractions {
        top: 0.01,
        media: 0.01,
        timeline: 0.01,
        applied: 0.05,
    };
    let huge = layout::Fractions {
        top: 0.99,
        media: 0.99,
        timeline: 0.99,
        applied: 0.5,
    };
    for f in [tiny, huge, layout::Fractions::DEFAULT] {
        h.app.settings.layout = Some(f);
        h.idle(3);
        let bad = reachable(&h, true);
        assert!(bad.is_empty(), "{f:?}: {bad:#?}");
        let s = h.cache.sections.unwrap();
        let mins = [
            s.media.width() == layout::MINS.media,
            s.preview.width() == layout::MINS.preview,
            s.timeline.width() == layout::MINS.timeline,
            s.effects.width() == layout::MINS.effects,
            s.media.height() == layout::MINS.row,
            s.timeline.height() == layout::MINS.row,
        ];
        assert!(f == layout::Fractions::DEFAULT || mins.iter().filter(|m| **m).count() >= 3);
    }
    // the long name is shortened, the full name is in the tooltip
    let long = h
        .nodes
        .iter()
        .find(|n| n.label.starts_with("a-very-long") && n.role == Role::Button)
        .cloned()
        .unwrap();
    assert!(long.rect.width() <= h.cache.sections.unwrap().media.width());
    // every popup and window in turn
    h.app.selected_clip = Some(h.app.project.timeline.clips[2].id);
    let line = h.app.project.subtitles.project(&h.app.project.timeline)[0].clone();
    h.app.popups.instance = Some(Instance::Line(line.line, line.clip));
    for p in Popup::ALL {
        h.app.popups.open = vec![p];
        h.idle(3);
        let bad = reachable(&h, false);
        assert!(bad.is_empty(), "{p:?}: {bad:#?}");
    }
    // the popups of 1.2: a clip's volume, the fades at the video's edges
    h.app.project.timeline.set_edge_fades(true).unwrap();
    let clip = h.app.project.timeline.clips[0].id;
    for i in [
        Instance::ClipVolume(clip),
        Instance::FadeStart,
        Instance::FadeEnd,
    ] {
        h.app.popups.instance = Some(i);
        h.app.popups.open = vec![Popup::Instance];
        h.idle(3);
        let bad = reachable(&h, false);
        assert!(bad.is_empty(), "{i:?}: {bad:#?}");
    }
    h.app.popups.open.clear();
    for m in [
        Modal::About,
        Modal::Unsaved(PendingAction::Quit),
        Modal::ConfirmRegenerate,
        Modal::ConfirmRemoveSpeechFiles,
        Modal::SubtitleSource {
            labels: (0..16)
                .map(|i| format!("Italian — track {i} {}", "x".repeat(60)))
                .collect(),
            can_match: false,
        },
    ] {
        h.app.modal = Some(m.clone());
        h.idle(3);
        let bad = reachable(&h, false);
        assert!(bad.is_empty(), "{m:?}: {bad:#?}");
    }
    h.app.modal = None;
    // every menu
    for (title, _) in menu::menus() {
        h.idle(1);
        open_menu(&mut h, title);
        let bad = reachable(&h, false);
        assert!(bad.is_empty(), "{title}: {bad:#?}");
        close_menus(&mut h);
    }
    // scroll areas show content that does not fit (media list, subtitle lines)
    assert!(
        h.cache
            .containers
            .iter()
            .any(|c| c.content.height() > c.viewport.height() + 1.0)
    );
    settle(&mut h.app);
}

/// Three clips (a, b, a) and a music clip.
fn three_clips() -> (tempfile::TempDir, AppState) {
    let (d, mut app) = loaded();
    let now = Instant::now();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    app.dispatch(Command::PlaceMusic(Some(app.project.media[2].id)), now)
        .unwrap();
    (d, app)
}

fn clip_nodes(h: &Harness) -> Vec<Node> {
    let mut v: Vec<Node> = h
        .nodes
        .iter()
        .filter(|n| {
            n.label
                .contains("Double-click for details, right-click for more")
        })
        .cloned()
        .collect();
    v.sort_by(|a, b| a.rect.min.x.partial_cmp(&b.rect.min.x).unwrap());
    v
}

// covers: REQ-082, REQ-075, REQ-079, REQ-083
#[test]
fn an_effect_dragged_onto_a_clip_applies_to_that_clip_only() {
    let (_d, app) = three_clips();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let clips = clip_nodes(&h);
    assert_eq!(clips.len(), 3);
    let ids: Vec<_> = h.app.project.timeline.clips.iter().map(|c| c.id).collect();
    let before = h.app.project.edit();
    // (c) dropped on the music track or on empty space: nothing changes
    let row = h.find("Fade IN/OUT").unwrap();
    let music = h
        .nodes
        .iter()
        .find(|n| n.label.starts_with("song.mp3\nDrag to move"))
        .cloned()
        .unwrap();
    h.drag(row.rect.center(), music.rect.center());
    assert_eq!(h.app.project.edit(), before);
    let media = h.cache.sections.unwrap().media;
    h.drag(row.rect.center(), media.center());
    assert_eq!(h.app.project.edit(), before);
    assert!(h.app.history.undo_len() >= 1);
    // (a) dropped on clip 2: fades at both of its edges, one undo step
    let n = h.app.history.undo_len();
    h.drag(row.rect.center(), clips[1].rect.center());
    let tl = &h.app.project.timeline;
    assert!(tl.has_fade(0) && tl.has_fade(1) && !tl.start_fade() && !tl.end_fade());
    assert_eq!(h.app.history.undo_len(), n + 1);
    assert_eq!(
        tl.clips.iter().map(|c| c.id).collect::<Vec<_>>(),
        ids,
        "no clip moved"
    );
    let dropped = h.app.project.edit();
    assert!(h.has("Fade IN/OUT — b.mp4"), "{:#?}", h.labels());
    h.key(Key::Escape, Modifiers::NONE);
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.project.edit(), before);
    // the same from the clip's menu: Effects > Fade IN/OUT
    let clips = clip_nodes(&h);
    let nodes_before = h.nodes.clone();
    h.right_click(clips[1].rect.center());
    let menu = opened(&nodes_before, &h.nodes);
    let effects = find_in(&menu, "Effects").expect("Effects submenu").clone();
    let nodes_before = h.nodes.clone();
    h.frame(vec![Event::PointerMoved(effects.rect.center())]);
    h.click(effects.rect.center());
    let sub = opened(&nodes_before, &h.nodes);
    for name in ["Fade IN/OUT", "Subtitles", "Volume"] {
        assert!(find_in(&sub, name).is_some(), "{name}: {sub:#?}");
    }
    let item = find_in(&sub, "Fade IN/OUT").unwrap().clone();
    h.click(item.rect.center());
    assert_eq!(h.app.project.edit(), dropped);
    // (b) a double-click applies it to the whole video
    h.key(Key::Escape, Modifiers::NONE);
    h.key(Key::Z, Modifiers::COMMAND);
    let row = h.find("Fade IN/OUT").unwrap();
    h.double_click(row.rect.center());
    let tl = &h.app.project.timeline;
    assert!(
        tl.start_fade() && tl.end_fade() && tl.fades.is_empty(),
        "{tl:?} {:?} {:?} {:#?}",
        h.app.status,
        h.app.popups,
        h.labels()
    );
}

// covers: REQ-089, REQ-092
#[test]
fn a_subtitle_dragged_in_the_preview_moves_and_stays_inside_the_picture() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    app.dispatch(Command::Seek(Micros(1_000_000)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let id = app.selection.primary_line().unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let other = app.selection.primary_line().unwrap();
    // the second line sits elsewhere (top right) so only the first is grabbed
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![other],
            pos: Some(crate::base::pos::Pos { x: 9000, y: 1000 }),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SetSubtitleText(other, "x".into()), now)
        .unwrap();
    app.dispatch(Command::ClearSelection, now).unwrap();
    let (w, hgt) = app.output_size().unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    assert_eq!((w, hgt), (1280, 720));
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = h
        .find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect;
    // the line is at the style's place: centred, just above the bottom margin
    let grab = pos2(pic.center().x, pic.min.y + pic.height() * 0.91);
    let n = h.app.history.undo_len();
    // (b) dragged far past the top-left corner: its centre stops on the corner (REQ-117)
    h.drag(grab, pic.min - vec2(300.0, 300.0));
    let p = h.app.project.subtitles.get(id).unwrap().position;
    let p = p.unwrap_or_else(|| panic!("the line did not move: {:?}", h.app.status));
    assert_eq!((p.x, p.y), (0, 0), "{p:?}");
    assert_eq!(h.app.history.undo_len(), n + 1, "one undo step per drag");
    assert!(h.app.drag_pos.is_none());
    // (e) the other line did not move
    assert_eq!(
        h.app.project.subtitles.get(other).unwrap().position,
        Some(crate::base::pos::Pos { x: 9000, y: 1000 })
    );
    // (c) undo puts it back at the style's place
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.project.subtitles.get(id).unwrap().position, None);
    // a drag that starts outside any subtitle does nothing
    h.drag(pic.center(), pic.min);
    assert_eq!(h.app.project.subtitles.get(id).unwrap().position, None);
    // REQ-092: with both lines selected, dragging one moves both to the same place
    h.app.selection = [id, other].into_iter().collect();
    h.drag(grab, pic.center());
    let a = h.app.project.subtitles.get(id).unwrap().position;
    assert!(a.is_some());
    assert_eq!(h.app.project.subtitles.get(other).unwrap().position, a);
    settle(&mut h.app);
}

// covers: REQ-090, REQ-091, REQ-077
#[test]
fn lines_are_selected_with_ctrl_click_and_shift_drag_on_the_timeline() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    for t in [2, 6, 8] {
        app.dispatch(Command::Seek(Micros(t * 1_000_000)), now)
            .unwrap();
        app.dispatch(Command::AddSubtitle, now).unwrap();
    }
    app.dispatch(Command::ClearSelection, now).unwrap();
    app.dispatch(Command::SelectLine(None), now).unwrap();
    let ids: Vec<_> = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let g = h.cache.timeline.unwrap();
    let at = |s: f64, y: f32| pos2(g.x_of(Micros::from_secs_f64(s)), y);
    let lane_y = g.lane.center().y;
    let ctrl_click = |h: &mut Harness, p: Pos2| {
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::COMMAND,
        };
        h.frame(vec![
            Event::ModifiersChanged(Modifiers::COMMAND),
            Event::PointerMoved(p),
            ev(true),
        ]);
        h.frame(vec![ev(false)]);
        h.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        h.idle(1);
    };
    // Ctrl/Cmd+click three markers: 3 selected; one again: 2; a plain click: 1
    for t in [2.0, 6.0, 8.0] {
        ctrl_click(&mut h, at(t, lane_y));
    }
    assert_eq!(h.app.selection.len(), 3, "{:?}", h.app.status);
    ctrl_click(&mut h, at(6.0, lane_y));
    assert_eq!(
        h.app.selection.iter().copied().collect::<Vec<_>>(),
        [ids[0], ids[2]]
    );
    h.click(at(8.0, lane_y));
    assert_eq!(
        h.app.selection.iter().copied().collect::<Vec<_>>(),
        [ids[2]]
    );
    // Esc closes the popup the click opened, then clears the selection
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.app.selection.len(), 1);
    assert!(h.app.popups.open.is_empty());
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.app.selection.is_empty());
    // Shift+drag over 4–9 s of the tracks: the lines starting at 6 s and 8 s
    let y = g.video.center().y;
    let (from, to) = (at(4.0, y), at(9.0, y));
    let ev = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::SHIFT,
    };
    let order: Vec<_> = h.app.project.timeline.clips.iter().map(|c| c.id).collect();
    h.frame(vec![
        Event::ModifiersChanged(Modifiers::SHIFT),
        Event::PointerMoved(from),
    ]);
    h.frame(vec![ev(from, true)]);
    for i in 1..=5 {
        let p = from + (to - from) * (i as f32 / 5.0);
        h.frame(vec![Event::PointerMoved(p)]);
    }
    h.frame(vec![ev(to, false)]);
    h.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
    h.idle(2);
    assert_eq!(
        h.app.selection.iter().copied().collect::<Vec<_>>(),
        [ids[1], ids[2]]
    );
    assert_eq!(
        h.app.playhead,
        Micros(6_000_000),
        "playhead at the first selected line"
    );
    let now_order: Vec<_> = h.app.project.timeline.clips.iter().map(|c| c.id).collect();
    assert_eq!(now_order, order, "the clips were not dragged");
    // a click on the empty part of the timeline clears the selection
    let empty = pos2(g.x_of(Micros(10_000_000)) + 15.0, g.video.center().y);
    h.click(empty);
    assert!(h.app.selection.is_empty());
    settle(&mut h.app);
}

// covers: REQ-114, REQ-075
#[test]
fn the_clean_audio_popup_explains_the_effect_and_removes_it() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::ApplyEffect {
            effect: crate::app::EffectKind::CleanAudio,
            target: crate::app::Target::Clip(clip),
        },
        now,
    )
    .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.idle(2);
    assert!(h.has("Reduces background noise"), "{:#?}", h.labels());
    h.click_label("Remove");
    assert!(!h.app.project.timeline.clips[0].clean_audio);
    assert_eq!(h.app.undo_text(), "Undo Remove clean audio");
    // the whole-video popup: apply and remove for every clip
    h.app
        .dispatch(Command::OpenPopup(Popup::CleanAudio), now)
        .unwrap();
    h.idle(2);
    assert!(h.has("0 of 2 clips have Clean audio."), "{:#?}", h.labels());
    h.click_label("Apply to whole video");
    assert!(h.app.project.timeline.clips.iter().all(|c| c.clean_audio));
    h.idle(2);
    assert!(h.has("2 of 2 clips have Clean audio."));
    h.click_label("Remove from whole video");
    assert!(!h.app.project.timeline.any_clean_audio());
}
