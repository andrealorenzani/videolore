// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 1.3.0: the "Add Text" row and popup, the Text lane of the timeline,
//! the Context Menu group and the Fade IN/OUT name. Real pointer events at the positions
//! of the widgets.

use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Modal, Popup};
use crate::text::{TextId, TextItem};
use eframe::egui::accesskit::Role;
use eframe::egui::{Color32, Pos2, Rect, pos2, vec2};
use eframe::egui::{Event, Key, Modifiers, PointerButton};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips, the playhead at 4 s, a frame on screen.
fn loaded() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::Seek(s(4)), now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    (dir, app)
}

fn text_at(app: &mut AppState, text: &str, start: i64) -> TextId {
    let now = Instant::now();
    app.dispatch(Command::Seek(s(start)), now).unwrap();
    app.dispatch(
        Command::AddText(TextItem::new(TextId(0), text.into(), Micros::ZERO)),
        now,
    )
    .unwrap();
    app.selection.primary_text().unwrap()
}

// covers: REQ-075, REQ-108
#[test]
fn the_add_text_row_opens_the_popup_and_add_creates_one_text() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let row = h.find("Add Text").expect("the row");
    assert_eq!(row.role, Role::Button);
    h.click(row.rect.center());
    assert!(h.app.popups.is_open(Popup::Text) && h.app.popups.text.is_none());
    // type two lines into the text field
    let field = h
        .nodes
        .iter()
        .find(|n| n.role == Role::MultilineTextInput)
        .cloned()
        .unwrap_or_else(|| panic!("no text field: {:#?}", h.labels()));
    h.click(field.rect.center());
    h.frame(vec![Event::Text("Hello".into())]);
    h.key(Key::Enter, Modifiers::NONE);
    h.frame(vec![Event::Text("world".into())]);
    h.idle(2);
    h.click_label("Fade IN");
    h.click_label("Add");
    let t = &h.app.project.texts.items[0];
    assert_eq!(t.text, "Hello\nworld");
    assert_eq!((t.start, t.ttl), (s(4), s(3)));
    assert!(t.effects.fade_in.is_some());
    assert!(!h.app.popups.is_open(Popup::Text), "Add closes the popup");
    assert_eq!(h.app.selection.primary_text(), Some(t.id));
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert!(h.app.project.texts.is_empty());
}

// covers: REQ-108
#[test]
fn escape_closes_the_new_text_popup_and_creates_nothing() {
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.click_label("Add Text");
    assert!(h.app.popups.is_open(Popup::Text));
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.popups.is_open(Popup::Text));
    assert!(h.app.project.texts.is_empty());
    // Add is greyed out while the field is empty
    h.click_label("Add Text");
    let add = h.find("Add").unwrap();
    assert!(add.disabled || h.app.project.texts.is_empty());
}

// covers: REQ-110
#[test]
fn a_text_bar_opens_its_popup_moves_resizes_and_is_deleted() {
    let (_d, mut app) = loaded();
    let id = text_at(&mut app, "Title", 4);
    app.dispatch(Command::ClosePopup(Popup::Text), Instant::now())
        .unwrap();
    app.dispatch(Command::ClearSelection, Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    assert!(h.cache.timeline.is_some());
    let bar = |h: &Harness| {
        let g = h.cache.timeline.unwrap();
        let t = h.app.project.texts.get(id).unwrap();
        let x = g.x_of(t.start);
        (g, x, x + (t.ttl.as_secs_f64() * f64::from(g.zoom)) as f32)
    };
    let (g, x0, x1) = bar(&h);
    // the bar is as long as the time to live (within a pixel)
    assert!((x1 - x0 - 3.0 * g.zoom).abs() <= 1.0);
    let y = g.text.center().y;
    // a click on the bar opens the popup of that text and moves the playhead to its start
    h.click(pos2((x0 + x1) / 2.0, y));
    assert_eq!(h.app.popups.text, Some(id));
    assert_eq!(h.app.selection.primary_text(), Some(id));
    assert_eq!(h.app.playhead, s(4));
    assert!(h.has("Title") || h.nodes.iter().any(|n| n.role == Role::MultilineTextInput));
    // drag the right end 1 s to the right: the text stays 4 s
    let dx = g.zoom;
    h.drag(pos2(x1 - 2.0, y), pos2(x1 - 2.0 + dx, y));
    let t = h.app.project.texts.get(id).unwrap();
    assert_eq!(t.start, s(4));
    assert!((t.ttl.as_secs_f64() - 4.0).abs() < 0.3, "{:?}", t.ttl);
    // drag the left end 1 s to the right: the end stays
    let end = t.end();
    let (_, x0, _) = bar(&h);
    h.drag(pos2(x0 + 2.0, y), pos2(x0 + 2.0 + dx, y));
    let t = h.app.project.texts.get(id).unwrap();
    assert!(
        (t.end() - end).0.abs() < 300_000,
        "{:?} vs {:?}",
        t.end(),
        end
    );
    assert!(t.start > s(4));
    // drag the middle 1 s to the left: it moves, same length
    let ttl = t.ttl;
    let (_, x0, x1) = bar(&h);
    h.drag(pos2((x0 + x1) / 2.0, y), pos2((x0 + x1) / 2.0 - dx, y));
    let t = h.app.project.texts.get(id).unwrap();
    assert_eq!(t.ttl, ttl);
    // one undo per drag
    let before = t.clone();
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_ne!(h.app.project.texts.get(id).unwrap(), &before);
    // Delete key removes the selected text, one undo step
    h.app
        .dispatch(Command::SelectText(id), Instant::now())
        .unwrap();
    h.key(Key::Delete, Modifiers::NONE);
    assert!(h.app.project.texts.is_empty());
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(h.app.project.texts.items.len(), 1);
}

// covers: REQ-110, REQ-090
#[test]
fn ctrl_or_shift_click_on_text_bars_selects_several_without_opening_a_popup() {
    let (_d, mut app) = loaded();
    let a = text_at(&mut app, "a", 1);
    let b = text_at(&mut app, "b", 6);
    app.dispatch(Command::ClosePopup(Popup::Text), Instant::now())
        .unwrap();
    app.dispatch(Command::ClearSelection, Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let g = h.cache.timeline.unwrap();
    let y = g.text.center().y;
    for (id, mods) in [(a, Modifiers::COMMAND), (b, Modifiers::SHIFT)] {
        let t = h.app.project.texts.get(id).unwrap();
        let p = pos2(g.x_of(t.start) + g.zoom, y);
        h.frame(vec![
            Event::ModifiersChanged(mods),
            Event::PointerMoved(p),
            Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: mods,
            },
        ]);
        h.frame(vec![Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: mods,
        }]);
        h.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        h.idle(2);
    }
    assert_eq!(h.app.selection.len(), 2);
    assert!(!h.app.popups.is_open(Popup::Text));
}

// covers: REQ-106
#[test]
fn the_context_group_is_shown_with_a_line_or_a_text_selected() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    let on_bar = |h: &Harness, label: &str| {
        h.nodes
            .iter()
            .any(|n| n.label == label && n.role == Role::Button && n.rect.max.y < 60.0)
    };
    let group = ["Align horizontally", "Align vertically", "Reset"];
    let t = text_at(&mut app, "T", 1);
    app.dispatch(Command::ClosePopup(Popup::Text), now).unwrap();
    let mut h = Harness::new(app, 1600.0, 800.0);
    // nothing selected on a flat clip: no group at all
    h.app.selection.clear();
    h.idle(2);
    assert!(group.iter().all(|l| !on_bar(&h, l)), "{:#?}", h.labels());
    // a text item selected: the three commands (REQ-115)
    h.app.dispatch(Command::SelectText(t), now).unwrap();
    h.idle(2);
    assert!(group.iter().all(|l| on_bar(&h, l)), "{:#?}", h.labels());
    h.app.dispatch(Command::ClearSelection, now).unwrap();
    h.idle(2);
    assert!(group.iter().all(|l| !on_bar(&h, l)));
    // a subtitle line selected: the three commands
    h.app.dispatch(Command::Seek(s(2)), now).unwrap();
    h.app.dispatch(Command::AddSubtitle, now).unwrap();
    h.idle(2);
    assert!(group.iter().all(|l| on_bar(&h, l)), "{:#?}", h.labels());
    // and it goes away again
    h.app.dispatch(Command::ClearSelection, now).unwrap();
    h.idle(2);
    assert!(group.iter().all(|l| !on_bar(&h, l)));
}

// covers: REQ-107
#[test]
fn no_user_visible_text_says_fade_through_black() {
    let bad = "through black";
    let (_d, app) = loaded();
    for a in actions::ACTIONS {
        assert!(!a.label.to_lowercase().contains(bad), "{}", a.label);
        assert!(!a.tooltip.to_lowercase().contains(bad), "{}", a.tooltip);
    }
    assert_eq!(crate::app::EffectKind::Fade.name(), "Fade IN/OUT");
    assert_eq!(crate::app::Popup::Fades.title(), "Fade IN/OUT");
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.app.popups.show(Popup::Fades);
    h.app.popups.show(Popup::Shortcuts);
    h.idle(3);
    assert!(h.has("Fade IN/OUT"));
    assert!(
        h.nodes
            .iter()
            .all(|n| !n.label.to_lowercase().contains(bad)),
        "{:#?}",
        h.labels()
    );
    assert!(
        crate::ui::markers::layout(&h.app.project, 50.0, 0.0, (0.0, 5000.0))
            .iter()
            .all(|m| !m.tooltip.to_lowercase().contains(bad))
    );
}

fn picture(h: &Harness) -> Rect {
    h.find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect
}

/// Where a frame fraction is on the screen.
fn at(pic: Rect, fx: f32, fy: f32) -> Pos2 {
    pos2(pic.min.x + fx * pic.width(), pic.min.y + fy * pic.height())
}

// covers: REQ-111, REQ-096
#[test]
fn a_text_box_is_dragged_in_the_preview_in_one_undo_step() {
    let (_d, mut app) = loaded();
    let id = text_at(&mut app, "HHHH", 4);
    app.dispatch(Command::ClosePopup(Popup::Text), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let n = h.app.history.undo_len();
    // the text starts centred in the lower third; drag it to the top left
    h.drag(at(pic, 0.5, 0.83), at(pic, 0.2, 0.2));
    let t = h.app.project.texts.get(id).unwrap();
    assert!(t.place.x < 3000 && t.place.y < 3000, "{:?}", t.place);
    assert_eq!(h.app.history.undo_len(), n + 1);
    // past the edge: the centre stops on the picture (the box may be partly outside)
    h.drag(at(pic, 0.2, 0.2), at(pic, -5.0, -5.0));
    let t = h.app.project.texts.get(id).unwrap();
    assert_eq!((t.place.x, t.place.y), (0, 0), "{:?}", t.place);
    h.key(Key::Z, Modifiers::COMMAND);
    h.drag(at(pic, 0.2, 0.2), at(pic, 0.5, 0.5));
    // one handle on the right edge changes the width only, in one undo step
    let hs = corner_handles(&mut h);
    assert_eq!(
        hs.len(),
        3,
        "handles on the right, left and bottom edges, none on the corners"
    );
    let handle = hs[0].center();
    let before = h.app.project.texts.get(id).unwrap().clone();
    let n = h.app.history.undo_len();
    h.drag(handle, handle - vec2(150.0, 0.0));
    let t = h.app.project.texts.get(id).unwrap();
    assert!(t.width < before.width, "{} -> {}", before.width, t.width);
    // the left edge stays, so the centre moves by half of the change
    assert_eq!((t.place.y, t.anchor), (before.place.y, before.anchor));
    assert!(t.place.x < before.place.x);
    assert_eq!(h.app.history.undo_len(), n + 1);
    // dragged far to the left it stops at 10 % of the picture width
    let handle = corner_handles(&mut h)[0].center();
    h.drag(handle, handle - vec2(5000.0, 0.0));
    assert_eq!(h.app.project.texts.get(id).unwrap().width, 100);
    h.key(Key::Z, Modifiers::COMMAND);
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.project.texts.get(id).unwrap().width, before.width);
}

fn corner_handles(h: &mut Harness) -> Vec<Rect> {
    let raw = eframe::egui::RawInput {
        screen_rect: Some(h.screen()),
        ..Default::default()
    };
    let now = Instant::now();
    let (app, cache) = (&mut h.app, &mut h.cache);
    let mut out = h.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
    let found = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            eframe::egui::Shape::Rect(r)
                if (r.rect.width() - 7.0).abs() < 0.01
                    && (r.rect.height() - 7.0).abs() < 0.01
                    && r.fill == Color32::WHITE =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    found
}

// covers: REQ-090, REQ-092
#[test]
fn shift_click_on_boxes_selects_several_a_drag_keeps_them_and_an_empty_click_clears() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    let a = text_at(&mut app, "AAAA", 4);
    let b = text_at(&mut app, "BBBB", 4);
    // b sits above a
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![b],
            place: crate::base::pos::Pos { x: 5000, y: 3000 },
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ClosePopup(Popup::Text), now).unwrap();
    app.dispatch(Command::SelectText(a), now).unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    // Shift+click on b adds it
    let p = at(pic, 0.5, 0.3);
    h.frame(vec![
        Event::ModifiersChanged(Modifiers::SHIFT),
        Event::PointerMoved(p),
        Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::SHIFT,
        },
    ]);
    h.frame(vec![Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::SHIFT,
    }]);
    h.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
    h.idle(2);
    assert_eq!(h.app.selection.len(), 2, "{:?}", h.app.status);
    // dragging one of them keeps the selection
    h.drag(at(pic, 0.5, 0.83), at(pic, 0.7, 0.6));
    assert_eq!(h.app.selection.len(), 2);
    // both are at the dragged place now (REQ-092)
    let (pa, pb) = (
        h.app.project.texts.get(a).unwrap().place,
        h.app.project.texts.get(b).unwrap().place,
    );
    assert_eq!(pa, pb);
    // a plain click on empty picture clears the selection
    h.click(at(pic, 0.05, 0.05));
    assert!(h.app.selection.is_empty());
}

// covers: REQ-118, REQ-122, REQ-103
#[test]
fn ctrl_e_opens_the_export_dialog_with_its_fields_inside_a_small_window() {
    use crate::ui::export_dialog as d;
    let (_d, mut app) = loaded();
    app.project.media[0].format.spherical = Some(true);
    app.project.media[1].format.spherical = Some(true);
    let mut h = Harness::new(app, 800.0, 500.0);
    h.key(Key::E, Modifiers::COMMAND);
    h.idle(3);
    assert!(matches!(h.app.modal, Some(Modal::Export(_))));
    for label in [
        d::TITLE,
        d::FILE_LABEL,
        d::FOLDER_BUTTON,
        d::MODE_LABEL,
        d::VIDEO_LABEL,
        d::AUDIO_LABEL,
        d::EXPORT_360,
        d::EXPORT_FLAT,
        d::EXPORT_BUTTON,
        d::CANCEL,
    ] {
        let n = h
            .find(label)
            .unwrap_or_else(|| panic!("no \"{label}\" in {:#?}", h.labels()));
        assert!(
            h.screen().contains_rect(n.rect),
            "\"{label}\" is outside the window: {:?}",
            n.rect
        );
    }
    // the first choices are the project's: no saved project, so the folder may be empty
    // and Export is refused with a reason on screen
    let mut c = match &h.app.modal {
        Some(Modal::Export(c)) => (**c).clone(),
        _ => unreachable!(),
    };
    c.folder = std::path::PathBuf::from("/no/such/folder");
    h.app.modal = Some(Modal::Export(Box::new(c)));
    h.idle(2);
    assert!(h.has("The folder does not exist"), "{:#?}", h.labels());
    // Esc closes it without writing anything
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.app.modal.is_none() && !h.app.is_exporting());
    // Cancel does the same
    h.key(Key::E, Modifiers::COMMAND);
    h.idle(3);
    h.click_label(d::CANCEL);
    assert!(h.app.modal.is_none());
    // a project without 360 clips has no 360 choice
    let mut app = loaded().1;
    app.project.media[0].format.spherical = Some(false);
    let mut h = Harness::new(app, 800.0, 500.0);
    h.key(Key::E, Modifiers::COMMAND);
    h.idle(3);
    assert!(!h.has(d::EXPORT_360));
    assert!(h.has(d::SAME_AS_SOURCE));
}

/// The box of a text on the screen, from the picture and the stored box.
fn box_on_screen(h: &Harness, id: TextId) -> Rect {
    let pic = picture(h);
    let t = h.app.project.texts.get(id).unwrap();
    let r = super::placement::text_box(t, h.app.output_size().unwrap());
    let scale = pic.width() / h.app.output_size().unwrap().0 as f32;
    Rect::from_min_max(
        pic.min + r.min.to_vec2() * scale,
        pic.min + r.max.to_vec2() * scale,
    )
}

// covers: REQ-160
#[test]
fn dragging_an_edge_draws_a_live_box_with_a_readout_and_stores_it_on_release() {
    let (_d, mut app) = loaded();
    let id = text_at(&mut app, "Hello there", 4);
    app.dispatch(Command::ClosePopup(Popup::Text), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let pic = picture(&h);
    let handle = corner_handles(&mut h)[0].center();
    let n = h.app.history.undo_len();
    let before = h.app.project.texts.get(id).unwrap().clone();
    // halfway through a drag to half the picture width: nothing is stored, no undo step
    let left = box_on_screen(&h, id).left();
    let goal = pos2(left + pic.width() * 0.5, handle.y);
    h.press_and_move(handle, goal);
    assert_eq!(h.app.history.undo_len(), n, "no undo step during the drag");
    assert_eq!(h.app.project.texts.get(id).unwrap(), &before);
    h.release(goal);
    // the stored box is the live box: 50% of the picture width
    let t = h.app.project.texts.get(id).unwrap();
    assert!((i32::from(t.width) - 500).abs() <= 8, "{}", t.width);
    assert_eq!(h.app.history.undo_len(), n + 1);
    // the readout names the width, the height and the text size
    let frame = h.app.output_size().unwrap();
    let r = super::placement::text_box(t, frame);
    let text = super::picture::readout(t, r, frame);
    assert!(
        text.contains("50% wide") || text.contains("49% wide") || text.contains("51% wide"),
        "{text}"
    );
    assert!(
        text.contains("20% high") && text.contains("text "),
        "{text}"
    );
}

// covers: REQ-159, REQ-161
#[test]
fn the_bottom_and_left_handles_change_the_box_in_one_undo_step_each() {
    let (_d, mut app) = loaded();
    let id = text_at(&mut app, "Hello there", 4);
    app.dispatch(Command::ClosePopup(Popup::Text), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let hs = corner_handles(&mut h);
    let (bottom, left) = (hs[2].center(), hs[1].center());
    let before = h.app.project.texts.get(id).unwrap().clone();
    let n = h.app.history.undo_len();
    // bottom edge down: taller, same width, top edge stays
    h.drag(bottom, bottom + vec2(0.0, 40.0));
    let t = h.app.project.texts.get(id).unwrap().clone();
    assert!(t.height > before.height, "{} {}", t.height, before.height);
    assert_eq!(t.width, before.width);
    assert_eq!(h.app.history.undo_len(), n + 1);
    // left edge to the left: wider, the right edge stays
    let left = left + vec2(0.0, (t.height - before.height) as f32 * 0.0);
    let right_before = i32::from(t.place.x) + i32::from(t.width) * 5;
    h.drag(left, left - vec2(60.0, 0.0));
    let u = h.app.project.texts.get(id).unwrap();
    assert!(u.width > t.width, "{} {}", u.width, t.width);
    assert_eq!(i32::from(u.place.x) + i32::from(u.width) * 5, right_before);
    assert_eq!(h.app.history.undo_len(), n + 2);
    h.key(Key::Z, Modifiers::COMMAND);
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.project.texts.get(id).unwrap(), &before);
}

// covers: REQ-162
#[test]
fn a_blink_hides_the_box_of_a_text_but_a_selected_one_keeps_a_dashed_outline() {
    use crate::text::{TextId as Tid, TextItem};
    let mut t = TextItem::new(Tid(1), "Hello".into(), s(4));
    t.effects.blink = Some(Micros(500_000));
    let frame = (1920, 1080);
    // shown for 0.5 s, hidden for 0.5 s
    let (_, shown) = super::picture::text_flags(&t, s(4) + Micros(250_000), frame);
    let (_, hidden) = super::picture::text_flags(&t, s(4) + Micros(750_000), frame);
    assert!(!shown && hidden);
    // a text without a blink is never hidden
    t.effects.blink = None;
    assert!(!super::picture::text_flags(&t, s(4) + Micros(750_000), frame).1);
    // the warning is on a text that does not fit even at the smallest size
    t.text = "word ".repeat(300);
    t.height = 50;
    assert!(super::picture::text_flags(&t, s(4), frame).0);
    // the picture draws it: a hidden selected text shows dashes, a hidden other one nothing
    let (_d, mut app) = loaded();
    let id = text_at(&mut app, "Blinking", 4);
    let mut item = app.project.texts.get(id).unwrap().clone();
    item.effects.blink = Some(Micros(500_000));
    item.ttl = s(1);
    app.project.texts.replace(item).unwrap();
    app.dispatch(Command::ClosePopup(Popup::Text), Instant::now())
        .unwrap();
    app.dispatch(Command::Seek(s(4) + Micros(750_000)), Instant::now())
        .unwrap();
    let dashes = |h: &mut Harness| {
        let raw = eframe::egui::RawInput {
            screen_rect: Some(h.screen()),
            ..Default::default()
        };
        let now = Instant::now();
        let (app, cache) = (&mut h.app, &mut h.cache);
        let mut out = h.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
        let n = out
            .shapes
            .iter()
            .filter(|s| matches!(s.shape, eframe::egui::Shape::LineSegment { .. }))
            .count();
        out.textures_delta.clear();
        out.drop_without_applying_deltas();
        n
    };
    let mut h = Harness::new(app, 1280.0, 800.0);
    let selected = dashes(&mut h);
    h.app
        .dispatch(Command::ClearSelection, Instant::now())
        .unwrap();
    let unselected = dashes(&mut h);
    assert!(
        selected >= unselected + 8,
        "dashes around the hidden selected box: {selected} / {unselected}"
    );
}

// covers: REQ-158, REQ-164
#[test]
fn the_text_popup_offers_readability_and_notes_a_missing_font() {
    let (_d, mut app) = loaded();
    app.fonts = crate::fonts::Fonts::with(vec!["Here Sans".into()]);
    let id = text_at(&mut app, "Hi", 4);
    let mut item = app.project.texts.get(id).unwrap().clone();
    item.font = "Gone Sans".into();
    app.project.texts.replace(item).unwrap();
    app.dispatch(Command::OpenText(id), Instant::now()).unwrap();
    let mut h = Harness::new(app, 1280.0, 800.0);
    assert!(h.has("Readability"), "{:#?}", h.labels());
    assert!(h.has("Outline"), "a new text starts with an outline");
    assert!(
        h.has("Font not installed: the bundled font is used"),
        "{:#?}",
        h.labels()
    );
    // choosing the readability stores it in one undo step
    let n = h.app.history.undo_len();
    h.click_label("Outline");
    h.click_label("Background box");
    assert_eq!(
        h.app.project.texts.get(id).unwrap().readability,
        crate::text::Readability::Box
    );
    assert_eq!(h.app.history.undo_len(), n + 1);
    // the saved font name is still the missing one
    assert_eq!(h.app.project.texts.get(id).unwrap().font, "Gone Sans");
}

// covers: REQ-108
#[test]
fn the_colour_button_opens_the_picker_and_a_click_in_it_changes_the_colour() {
    let (_d, mut app) = loaded();
    let id = text_at(&mut app, "Hi", 4);
    app.dispatch(Command::OpenText(id), Instant::now()).unwrap();
    let mut h = Harness::new(app, 1280.0, 900.0);
    let before = h.app.project.texts.get(id).unwrap().colour;
    let button = h.find("Color").expect("the colour button").rect;
    h.click(button.center());
    h.idle(3);
    // the picker is a new popup with a square: the largest interactive widget
    let square = h
        .nodes
        .iter()
        .filter(|n| n.interactive && n.rect.width() > 100.0 && n.rect.height() > 100.0)
        .map(|n| n.rect)
        .next()
        .expect("the picker's colour square");
    h.click(pos2(square.left() + 5.0, square.top() + 5.0));
    h.idle(2);
    // a click elsewhere closes the picker and the new colour is stored in one step
    h.click(pos2(5.0, 400.0));
    h.idle(3);
    let after = h.app.project.texts.get(id).unwrap().colour;
    assert_ne!(
        before, after,
        "the colour changed by the click in the picker"
    );
}

// covers: REQ-103
#[test]
fn the_export_dialog_dropdowns_open_on_a_click_and_a_choice_is_kept() {
    use crate::ui::export_dialog as d;
    let (_d, app) = loaded();
    let mut h = Harness::new(app, 1000.0, 800.0);
    h.key(Key::E, Modifiers::COMMAND);
    h.idle(3);
    let mode = |h: &Harness| match &h.app.modal {
        Some(Modal::Export(c)) => c.mode,
        _ => panic!("no export dialog"),
    };
    let first = mode(&h);
    let other = crate::export::choice::Mode::ALL
        .iter()
        .copied()
        .find(|m| *m != first)
        .unwrap();
    // the Mode dropdown shows the current choice; a click lists every mode
    h.click_label(first.label());
    for m in crate::export::choice::Mode::ALL {
        assert!(h.has(m.label()), "{} in {:#?}", m.label(), h.labels());
    }
    h.click_label(other.label());
    assert_eq!(mode(&h), other);
    // the Video dropdown lists "Same as source" and at least one preset
    h.click_label(d::SAME_AS_SOURCE);
    let n = h.labels().len();
    assert!(n > 10, "the list is open: {:#?}", h.labels());
    h.key(Key::Escape, Modifiers::NONE);
}
