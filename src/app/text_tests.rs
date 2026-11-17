// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 1.3.0: text items (add, edit, move in time, delete, box in the preview),
//! selection of text items with lines, and the Context Menu rules.

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::text::{SlideDir, TextId, TextItem};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// A project with two clips (5 s each) and the playhead at 4 s.
fn app_with_clips(dir: &Path) -> AppState {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let now = Instant::now();
    for i in [0, 1] {
        let id = app.project.media[i].id;
        app.dispatch(Command::Append(Some(id)), now).unwrap();
    }
    app.dispatch(Command::Seek(s(4)), now).unwrap();
    app
}

fn draft(text: &str) -> TextItem {
    TextItem::new(TextId(0), text.into(), Micros::ZERO)
}

fn add(app: &mut AppState, text: &str) -> TextId {
    app.dispatch(Command::AddText(draft(text)), Instant::now())
        .unwrap();
    app.selection.primary_text().unwrap()
}

// covers: REQ-108
#[test]
fn adding_a_text_at_the_playhead_is_one_undo_step_and_selects_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let mut d = draft("Hello\nthere");
    d.effects.fade_in = Some(Micros(500_000));
    d.colour = [10, 20, 30];
    app.dispatch(Command::AddText(d), now).unwrap();
    let t = &app.project.texts.items[0];
    assert_eq!(t.text, "Hello\nthere");
    assert_eq!((t.start, t.ttl), (s(4), s(3)));
    assert_eq!(t.colour, [10, 20, 30]);
    assert_eq!(t.place, crate::text::DEFAULT_PLACE);
    assert!(t.effects.fade_in.is_some());
    assert_eq!(app.selection.primary_text(), Some(t.id));
    assert!(app.dirty);
    assert_eq!(app.undo_text(), "Undo Add text");
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.texts.is_empty());
    assert!(app.selection.is_empty(), "the selection forgets the text");
    app.dispatch(Command::Redo, now).unwrap();
    assert_eq!(app.project.texts.items.len(), 1);
}

// covers: REQ-108
#[test]
fn an_empty_text_or_a_playhead_outside_the_video_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let e = app
        .dispatch(Command::AddText(draft("  \n ")), now)
        .unwrap_err();
    assert_eq!(e, texts::TYPE_SOME_TEXT);
    assert!(app.project.texts.is_empty());
    assert!(!app.history.can_undo() || app.undo_text() != "Undo Add text");
    app.dispatch(Command::Seek(s(10)), now).unwrap();
    assert!(!app.is_enabled(&Command::OpenNewText));
    assert_eq!(
        app.disabled_reason(&Command::OpenNewText),
        Some(texts::PLAYHEAD_IN_VIDEO)
    );
    let e = app.dispatch(Command::AddText(draft("x")), now).unwrap_err();
    assert_eq!(e, texts::PLAYHEAD_IN_VIDEO);
    // no clips at all
    let mut empty = state(FakeDialogs::default());
    assert!(!empty.is_enabled(&Command::AddText(draft("x"))));
    assert!(empty.dispatch(Command::AddText(draft("x")), now).is_err());
}

// covers: REQ-108, REQ-110
#[test]
fn the_popup_state_follows_opening_adding_and_closing() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    app.dispatch(Command::OpenNewText, now).unwrap();
    assert!(app.popups.is_open(Popup::Text) && app.popups.text.is_none());
    assert_eq!(app.popup_title(Popup::Text), "Text");
    let id = add(&mut app, "Hi");
    app.dispatch(Command::Seek(Micros::ZERO), now).unwrap();
    app.dispatch(Command::OpenText(id), now).unwrap();
    assert_eq!(app.popups.text, Some(id));
    assert_eq!(app.playhead, s(4), "the playhead goes to its start");
    assert!(app.dispatch(Command::OpenText(TextId(99)), now).is_err());
    app.dispatch(Command::ClosePopup(Popup::Text), now).unwrap();
    assert_eq!(app.popups.text, None);
    // opening a new project closes it
    app.dispatch(Command::OpenText(id), now).unwrap();
    app.dispatch(Command::DeleteTexts(vec![id]), now).unwrap();
    assert!(!app.popups.is_open(Popup::Text) || app.popups.text.is_none());
}

// covers: REQ-109
#[test]
fn editing_effects_is_one_undo_step_each_and_times_are_limited() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let id = add(&mut app, "Hi");
    let mut t = app.project.texts.get(id).unwrap().clone();
    t.effects.slide = Some((SlideDir::Right, Micros(700_000)));
    t.effects.blink = Some(Micros(500_000));
    app.dispatch(Command::EditText(t.clone()), now).unwrap();
    assert_eq!(app.undo_text(), "Undo Edit text");
    assert_eq!(app.project.texts.get(id).unwrap().effects, t.effects);
    // the same value again is not a step
    let n = app.history.can_undo();
    app.dispatch(Command::EditText(t.clone()), now).unwrap();
    assert!(n);
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.texts.get(id).unwrap().effects.slide.is_none());
    // an effect time of 0.05 s is refused
    t.effects.fade_in = Some(Micros(50_000));
    assert!(app.dispatch(Command::EditText(t.clone()), now).is_err());
    // text with nothing in it too, and unknown ids
    t.effects.fade_in = None;
    t.text = "".into();
    assert!(app.dispatch(Command::EditText(t.clone()), now).is_err());
    t.id = TextId(500);
    t.text = "x".into();
    assert!(app.dispatch(Command::EditText(t), now).is_err());
}

// covers: REQ-110
#[test]
fn moving_and_resizing_the_bar_changes_start_and_ttl_without_touching_the_video() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let id = add(&mut app, "Hi");
    let clips = app.project.timeline.clone();
    // right end dragged from 3 s to 6 s: it is 6 s long, but the video is 10 s: from 4 s it
    // is visible until the end of the video; the time to live is limited to the video length
    app.dispatch(
        Command::MoveText {
            id,
            start: s(4),
            ttl: s(6),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.texts.get(id).unwrap().ttl, s(6));
    assert_eq!(app.undo_text(), "Undo Move text");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.texts.get(id).unwrap().ttl, s(3));
    // move in time; below the minimum ttl and past the video length is limited
    app.dispatch(
        Command::MoveText {
            id,
            start: s(1),
            ttl: Micros(10),
        },
        now,
    )
    .unwrap();
    let t = app.project.texts.get(id).unwrap();
    assert_eq!((t.start, t.ttl), (s(1), crate::text::MIN_TTL));
    app.dispatch(
        Command::MoveText {
            id,
            start: s(-3),
            ttl: s(99),
        },
        now,
    )
    .unwrap();
    let t = app.project.texts.get(id).unwrap();
    assert_eq!((t.start, t.ttl), (Micros::ZERO, s(10)));
    // no ripple: editing the video does not move the text
    app.dispatch(
        Command::MoveText {
            id,
            start: s(6),
            ttl: s(3),
        },
        now,
    )
    .unwrap();
    app.selected_clip = Some(app.project.timeline.clips[0].id);
    app.dispatch(Command::Delete, now).unwrap();
    let t = app.project.texts.get(id).unwrap();
    assert_eq!((t.start, t.ttl), (s(6), s(3)));
    assert_ne!(clips, app.project.timeline);
    // the same values again: no step
    app.dispatch(
        Command::MoveText {
            id,
            start: s(6),
            ttl: s(3),
        },
        now,
    )
    .unwrap();
    assert!(
        app.dispatch(
            Command::MoveText {
                id: TextId(9),
                start: s(1),
                ttl: s(1)
            },
            now
        )
        .is_err()
    );
}

// covers: REQ-110
#[test]
fn deleting_texts_is_one_undo_step_and_the_delete_key_follows_the_selected_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let a = add(&mut app, "a");
    let b = add(&mut app, "b");
    assert!(app.dispatch(Command::DeleteTexts(vec![]), now).is_err());
    assert!(
        app.dispatch(Command::DeleteTexts(vec![TextId(77)]), now)
            .is_err()
    );
    assert!(!app.is_enabled(&Command::DeleteTexts(vec![TextId(77)])));
    assert_eq!(
        app.disabled_reason(&Command::DeleteTexts(vec![])),
        Some(texts::SELECT_A_TEXT)
    );
    app.dispatch(Command::DeleteTexts(vec![a, b]), now).unwrap();
    assert!(app.project.texts.is_empty());
    assert!(app.selection.is_empty());
    assert_eq!(app.undo_text(), "Undo Delete text");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.texts.items.len(), 2);
}

// covers: REQ-111, REQ-117
#[test]
fn a_box_dragged_in_the_preview_sets_the_place_for_the_group() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let a = add(&mut app, "a");
    let b = add(&mut app, "b");
    let p = Pos { x: 1000, y: 900 };
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![a, b],
            place: p,
        },
        now,
    )
    .unwrap();
    for id in [a, b] {
        assert_eq!(app.project.texts.get(id).unwrap().place, p);
    }
    assert_eq!(app.undo_text(), "Undo Move text");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(
        app.project.texts.get(a).unwrap().place,
        crate::text::DEFAULT_PLACE
    );
    // the centre may be on the very edge of the picture
    let edge = Pos { x: 10_000, y: 0 };
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![a],
            place: edge,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.texts.get(a).unwrap().place, edge);
    for bad in [
        (vec![], p),
        (vec![a], Pos { x: 20_000, y: 0 }),
        (vec![TextId(40)], p),
    ] {
        assert!(
            app.dispatch(
                Command::SetTextBoxes {
                    ids: bad.0,
                    place: bad.1
                },
                now
            )
            .is_err()
        );
    }
}

// covers: REQ-116
#[test]
fn the_width_of_a_box_is_one_undo_step_and_stays_between_10_and_100_percent() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let a = add(&mut app, "a");
    let b = add(&mut app, "b");
    assert_eq!(
        app.project.texts.get(a).unwrap().width,
        crate::text::DEFAULT_WIDTH
    );
    app.dispatch(
        Command::ResizeTextWidth {
            ids: vec![a, b],
            width: 500,
        },
        now,
    )
    .unwrap();
    for id in [a, b] {
        assert_eq!(app.project.texts.get(id).unwrap().width, 500);
    }
    assert_eq!(app.undo_text(), "Undo Resize text");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(
        app.project.texts.get(a).unwrap().width,
        crate::text::DEFAULT_WIDTH
    );
    // out of range values stop at the limits
    app.dispatch(
        Command::ResizeTextWidth {
            ids: vec![a],
            width: 5,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.texts.get(a).unwrap().width, 100);
    app.dispatch(
        Command::ResizeTextWidth {
            ids: vec![a],
            width: 9000,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.texts.get(a).unwrap().width, 1000);
    // the same width again is no change
    let undo = app.history.undo_len();
    app.dispatch(
        Command::ResizeTextWidth {
            ids: vec![a],
            width: 1000,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo);
    for ids in [vec![], vec![TextId(40)]] {
        assert!(
            app.dispatch(Command::ResizeTextWidth { ids, width: 500 }, now)
                .is_err()
        );
    }
    // a text that is not fixed to the sphere has no angle width
    assert!(
        app.dispatch(
            Command::ResizeTextAnchorWidth {
                ids: vec![a],
                width_cd: 5000
            },
            now
        )
        .is_err()
    );
}

// covers: REQ-115, REQ-106
#[test]
fn align_and_reset_work_on_text_items_and_lines_together() {
    use crate::subtitles::NewLine;
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let a = add(&mut app, "a");
    let low_left = Pos { x: 1000, y: 9000 };
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![a],
            place: low_left,
        },
        now,
    )
    .unwrap();
    // nothing selected: the commands are not available
    app.dispatch(Command::ClearSelection, now).unwrap();
    assert!(!app.is_enabled(&Command::AlignSelected(Axis::Horizontal)));
    assert!(!app.is_enabled(&Command::ResetObjects));
    app.dispatch(Command::SelectText(a), now).unwrap();
    assert!(app.is_enabled(&Command::AlignSelected(Axis::Horizontal)));
    assert!(app.is_enabled(&Command::ResetObjects));
    // align horizontally: x centred, y unchanged, width unchanged
    app.dispatch(Command::AlignSelected(Axis::Horizontal), now)
        .unwrap();
    let t = app.project.texts.get(a).unwrap();
    assert_eq!((t.place.x, t.place.y), (5000, 9000));
    assert_eq!(app.undo_text(), "Undo Align horizontally");
    app.dispatch(Command::AlignSelected(Axis::Vertical), now)
        .unwrap();
    let t = app.project.texts.get(a).unwrap();
    assert_eq!((t.place.x, t.place.y), (5000, 5000));
    // aligned again: nothing to do, no undo step
    let undo = app.history.undo_len();
    app.dispatch(Command::AlignSelected(Axis::Vertical), now)
        .unwrap();
    assert_eq!(app.history.undo_len(), undo);
    // reset: the default place and the default width, one undo step
    app.dispatch(
        Command::ResizeTextWidth {
            ids: vec![a],
            width: 300,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ResetObjects, now).unwrap();
    let t = app.project.texts.get(a).unwrap();
    assert_eq!(
        (t.place, t.width),
        (crate::text::DEFAULT_PLACE, crate::text::DEFAULT_WIDTH)
    );
    assert_eq!(app.undo_text(), "Undo Reset");
    assert!(!app.is_enabled(&Command::ResetObjects), "already original");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.texts.get(a).unwrap().width, 300);
    // a text and a subtitle line selected together: both get the command
    let m = app.project.media[0].id;
    app.project.subtitles.replace_all(
        vec![NewLine {
            media: m,
            start: Micros(0),
            end: Micros(1_000_000),
            text: "line".into(),
            words: vec![],
        }],
        None,
    );
    let line = app.project.subtitles.lines[0].id;
    app.dispatch(Command::SelectLine(Some(line)), now).unwrap();
    app.dispatch(Command::ToggleText(a), now).unwrap();
    app.dispatch(Command::AlignSelected(Axis::Vertical), now)
        .unwrap();
    assert_eq!(app.project.texts.get(a).unwrap().place.y, 5000);
    assert_eq!(
        app.project.subtitles.lines[0].position.map(|p| p.y),
        Some(5000)
    );
    app.dispatch(Command::ResetObjects, now).unwrap();
    assert_eq!(app.project.subtitles.lines[0].position, None);
    assert_eq!(
        app.project.texts.get(a).unwrap().place,
        crate::text::DEFAULT_PLACE
    );
}

// covers: REQ-090, REQ-091, REQ-092
#[test]
fn texts_and_lines_share_one_selection() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    let a = add(&mut app, "a");
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let line = app.selection.primary_line().unwrap();
    assert_eq!(app.selection.len(), 1, "adding a line selects only it");
    // a click with Ctrl adds the text; a plain click on it leaves one object
    app.dispatch(Command::ToggleText(a), now).unwrap();
    assert_eq!(app.selection.len(), 2);
    assert!(app.dispatch(Command::ToggleText(TextId(7)), now).is_err());
    app.dispatch(Command::SelectText(a), now).unwrap();
    assert_eq!(app.selection.texts(), [a]);
    assert!(app.selection.lines().is_empty());
    assert!(app.dispatch(Command::SelectText(TextId(7)), now).is_err());
    // a range from 0 to 5 s picks the line (1 s) and the text (4 s) and goes to the first
    app.dispatch(
        Command::SelectRange {
            from: s(0),
            to: s(5),
            add: false,
        },
        now,
    )
    .unwrap();
    assert_eq!((app.selection.len(), app.playhead), (2, s(1)));
    assert!(app.selection.contains(&line) && app.selection.contains_text(a));
    // the Delete key deletes the selected text; Esc... clear
    app.dispatch(Command::SelectText(a), now).unwrap();
    app.dispatch(Command::ClearSelection, now).unwrap();
    assert!(app.selection.is_empty());
    // lines moved as a group do not take the texts along (and the other way round)
    app.dispatch(
        Command::SelectRange {
            from: s(0),
            to: s(5),
            add: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.drag_group(line), [line]);
}

// covers: REQ-113
#[test]
fn texts_are_saved_and_make_the_project_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let file = dir.path().join("p.vlproj");
    let now = Instant::now();
    app.project_path = Some(file.clone());
    app.dispatch(Command::SaveProject, now).unwrap();
    assert!(!app.dirty);
    add(&mut app, "kept");
    assert!(app.dirty);
    app.dispatch(Command::SaveProject, now).unwrap();
    assert!(!app.dirty);
    let (p, _) = crate::project::load(&file).unwrap();
    assert_eq!(p.texts.items[0].text, "kept");
    // the new id of a later text does not reuse one
    let mut q = p.clone();
    let id = q.texts.new_id();
    assert!(id.0 > p.texts.items[0].id.0);
}

/// The choices of the open Export dialog.
fn export_choice(app: &AppState) -> crate::export::choice::ExportChoice {
    match &app.modal {
        Some(Modal::Export(c)) => (**c).clone(),
        other => panic!("no Export dialog: {other:?}"),
    }
}

// covers: REQ-118, REQ-119, REQ-122, REQ-103
#[test]
fn the_export_dialog_starts_the_export_it_describes() {
    use crate::export::choice::{Mode, Sphere};
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    // an empty timeline cannot be exported
    assert!(!app.is_enabled(&Command::PickExport));
    import(&mut app, files(dir.path()));
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    // Export… opens the dialog and starts nothing
    app.dispatch(Command::PickExport, now).unwrap();
    assert!(!app.is_exporting());
    let mut c = export_choice(&app);
    assert_eq!((c.mode, c.video), (Mode::Full, None));
    c.folder = dir.path().to_path_buf();
    c.name = "movie".into();
    app.dispatch(Command::SetExportChoice(Box::new(c.clone())), now)
        .unwrap();
    // a missing folder, an empty name: Export is refused with the reason
    let mut bad = c.clone();
    bad.folder = dir.path().join("missing");
    let e = app
        .dispatch(Command::StartExport(Box::new(bad)), now)
        .unwrap_err();
    assert!(e.contains("folder"), "{e}");
    let mut bad = c.clone();
    bad.name = String::new();
    assert!(
        app.dispatch(Command::StartExport(Box::new(bad)), now)
            .is_err()
    );
    // the source files are never replaced
    let mut bad = c.clone();
    bad.name = "a".into();
    let e = app
        .dispatch(Command::StartExport(Box::new(bad)), now)
        .unwrap_err();
    assert!(e.contains("source"), "{e}");
    // Export: the file appears with the extension of the format
    app.dispatch(Command::StartExport(Box::new(c.clone())), now)
        .unwrap();
    assert!(app.is_exporting() && app.modal.is_none());
    app.wait_export(now);
    assert!(dir.path().join("movie.mp4").is_file());
    // a second opening in the same session shows the last choices
    app.dispatch(Command::PickExport, now).unwrap();
    assert_eq!(export_choice(&app), c);
    // the file exists: asks first; Cancel writes nothing
    app.dispatch(Command::StartExport(Box::new(c.clone())), now)
        .unwrap();
    assert!(matches!(app.modal, Some(Modal::ConfirmOverwrite(_))));
    assert!(!app.is_exporting());
    app.dispatch(Command::CloseModal, now).unwrap();
    assert!(app.modal.is_none() && !app.is_exporting());
    // Replace exports
    app.dispatch(Command::PickExport, now).unwrap();
    app.dispatch(Command::StartExport(Box::new(c.clone())), now)
        .unwrap();
    app.dispatch(Command::ConfirmOverwrite, now).unwrap();
    assert!(app.is_exporting());
    app.wait_export(now);
    // "..." changes the folder
    let other = dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    app.dialogs = Box::new(FakeDialogs {
        folder: Some(other.clone()),
        ..Default::default()
    });
    app.dispatch(Command::PickExport, now).unwrap();
    app.dispatch(Command::PickExportFolder, now).unwrap();
    assert_eq!(export_choice(&app).folder, other);
    app.dispatch(Command::CloseModal, now).unwrap();
    // Only audio writes an audio file, Only subtitles needs subtitles
    app.dispatch(Command::PickExport, now).unwrap();
    let mut audio = export_choice(&app);
    audio.mode = Mode::AudioOnly;
    audio.name = "sound".into();
    app.dispatch(Command::StartExport(Box::new(audio.clone())), now)
        .unwrap();
    app.wait_export(now);
    assert!(other.join("sound.m4a").is_file());
    app.dispatch(Command::PickExport, now).unwrap();
    let mut srt = export_choice(&app);
    srt.mode = Mode::Srt;
    let e = app
        .dispatch(Command::StartExport(Box::new(srt)), now)
        .unwrap_err();
    assert!(e.contains("No subtitles"), "{e}");
    app.dispatch(Command::CloseModal, now).unwrap();
    // a 360 project: the choice shows, "Flat camera view" starts a flat export
    app.project.media[0].format.spherical = Some(true);
    let ctx = app.export_context();
    assert!(ctx.all_360);
    app.dispatch(Command::PickExport, now).unwrap();
    let mut flat = export_choice(&app);
    flat.mode = Mode::Full;
    flat.sphere = Sphere::Flat;
    flat.name = "flat".into();
    app.dispatch(Command::StartExport(Box::new(flat)), now)
        .unwrap();
    app.wait_export(now);
    assert!(other.join("flat.mp4").is_file());
    // the dialog's mixed project (a flat clip too) is not all 360: exported flat, no choice
    app.dispatch(Command::Append(Some(b)), now).unwrap();
    assert!(!app.export_context().all_360);
    app.dispatch(Command::PickExport, now).unwrap();
    let mut mixed = export_choice(&app);
    mixed.name = "mixed".into();
    mixed.sphere = Sphere::Video360;
    app.dispatch(Command::StartExport(Box::new(mixed)), now)
        .unwrap();
    app.wait_export(now);
    assert!(other.join("mixed.mp4").is_file());
    // the lower level still refuses a 360 video of a mixed project
    let e = app
        .dispatch(
            Command::ExportAs(dir.path().join("x.mp4"), render::ExportMode::Sphere360),
            now,
        )
        .unwrap_err();
    assert!(e.contains("every clip"), "{e}");
}

// covers: REQ-118
#[test]
fn the_dialog_of_a_saved_project_starts_in_its_folder_with_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    app.project_path = Some(dir.path().join("my trip.vlproj"));
    app.dispatch(Command::PickExport, now).unwrap();
    let c = export_choice(&app);
    assert_eq!(
        (c.name.as_str(), c.folder.as_path()),
        ("my trip", dir.path())
    );
    // Esc / Cancel keeps what was chosen for the next time
    let mut edited = c.clone();
    edited.name = "other".into();
    app.dispatch(Command::SetExportChoice(Box::new(edited)), now)
        .unwrap();
    app.dispatch(Command::CloseModal, now).unwrap();
    app.dispatch(Command::PickExport, now).unwrap();
    assert_eq!(export_choice(&app).name, "other");
    // a choice sent while no dialog is open changes nothing
    app.dispatch(Command::CloseModal, now).unwrap();
    app.dispatch(Command::SetExportChoice(Box::new(c)), now)
        .unwrap();
    assert!(app.modal.is_none());
    // answers without a question are ignored
    app.dispatch(Command::ConfirmOverwrite, now).unwrap();
    app.dispatch(Command::PickExportFolder, now).unwrap();
}

// covers: REQ-112
#[test]
fn a_text_added_on_a_360_clip_is_fixed_to_the_point_the_camera_looks_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    app.project.media[0].format.spherical = Some(true);
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    // the camera looks at yaw 30, pitch 10 (a live view)
    let m = app.project.media[0].id;
    app.dispatch(
        Command::DragCamera {
            dx: 0.0,
            dy: 0.0,
            view_w: 100.0,
        },
        now,
    )
    .ok();
    let id = add(&mut app, "Pinned");
    let t = app.project.texts.get(id).unwrap();
    let a = t.anchor.expect("anchored on a 360 clip");
    assert!(a.is_valid());
    assert_eq!(
        (a.yaw_cd, a.pitch_cd),
        (0, 0),
        "the front view when no POV is set"
    );
    // moving it to another point is one undo step; a flat text cannot be moved this way
    app.dispatch(
        Command::MoveTextAnchor {
            ids: vec![id],
            yaw_cd: 3000,
            pitch_cd: 1000,
        },
        now,
    )
    .unwrap();
    let a = app.project.texts.get(id).unwrap().anchor.unwrap();
    assert_eq!((a.yaw_cd, a.pitch_cd), (3000, 1000));
    assert_eq!(app.undo_text(), "Undo Move text");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.texts.get(id).unwrap().anchor.unwrap().yaw_cd, 0);
    for (ids, yaw, pitch) in [
        (vec![id], 18_001, 0),
        (vec![id], 0, 9_001),
        (vec![], 0, 0),
        (vec![TextId(99)], 0, 0),
    ] {
        assert!(
            app.dispatch(
                Command::MoveTextAnchor {
                    ids,
                    yaw_cd: yaw,
                    pitch_cd: pitch
                },
                now
            )
            .is_err()
        );
    }
    // the box of a resize keeps the flat place of an anchored text
    let before = app.project.texts.get(id).unwrap().place;
    app.dispatch(
        Command::SetTextBoxes {
            ids: vec![id],
            place: Pos { x: 1, y: 1 },
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.texts.get(id).unwrap().place, before);
    // its width is an angle
    app.dispatch(
        Command::ResizeTextAnchorWidth {
            ids: vec![id],
            width_cd: 5, // below 10 degrees
        },
        now,
    )
    .unwrap();
    assert_eq!(
        app.project.texts.get(id).unwrap().anchor.unwrap().width_cd,
        1000
    );
    // on a flat clip the text is not anchored
    app.dispatch(Command::Seek(s(7)), now).unwrap();
    let flat = add(&mut app, "Flat");
    assert!(app.project.texts.get(flat).unwrap().anchor.is_none());
    assert!(
        app.dispatch(
            Command::MoveTextAnchor {
                ids: vec![flat],
                yaw_cd: 0,
                pitch_cd: 0
            },
            now
        )
        .is_err()
    );
    let _ = m;
}

// covers: REQ-112, REQ-103
#[test]
fn a_subtitle_line_added_on_a_360_clip_is_fixed_to_the_sphere_automatic_ones_are_not() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_clips(dir.path());
    let now = Instant::now();
    app.project.media[0].format.spherical = Some(true);
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let id = app.selection.primary_line().unwrap();
    let a = app
        .project
        .subtitles
        .get(id)
        .unwrap()
        .anchor
        .expect("anchored");
    assert!(a.is_valid());
    // moving it: one undo step; only fixed lines can be moved this way
    app.dispatch(
        Command::MoveLineAnchor {
            ids: vec![id],
            yaw_cd: 4000,
            pitch_cd: -500,
        },
        now,
    )
    .unwrap();
    assert_eq!(
        app.project
            .subtitles
            .get(id)
            .unwrap()
            .anchor
            .unwrap()
            .yaw_cd,
        4000
    );
    assert_eq!(app.undo_text(), "Undo Move subtitle");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(
        app.project
            .subtitles
            .get(id)
            .unwrap()
            .anchor
            .unwrap()
            .yaw_cd,
        a.yaw_cd
    );
    for (ids, y, p) in [
        (vec![id], 18_001, 0),
        (vec![id], 0, 9_001),
        (vec![LineId(99)], 0, 0),
    ] {
        assert!(
            app.dispatch(
                Command::MoveLineAnchor {
                    ids,
                    yaw_cd: y,
                    pitch_cd: p
                },
                now
            )
            .is_err()
        );
    }
    // on a flat clip a new line has no anchor
    app.dispatch(Command::Seek(s(7)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let flat = app.selection.primary_line().unwrap();
    assert!(app.project.subtitles.get(flat).unwrap().anchor.is_none());
    assert!(
        app.dispatch(
            Command::MoveLineAnchor {
                ids: vec![flat],
                yaw_cd: 0,
                pitch_cd: 0
            },
            now
        )
        .is_err()
    );
}
