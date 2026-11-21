// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 2.0.4, batch 5: subtitle scripts in the list and the line popup
//! (REQ-188), the import button and offer (REQ-183, REQ-185) and the export dialog's
//! Subtitles row (REQ-182).

use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Command, Modal, Popup};
use crate::base::font::Script;
use crate::subtitles::NewLine;
use eframe::egui::FontId;
use std::collections::BTreeSet;
use std::time::Instant;

/// One clip of 5 s with a subtitle line in each script and one in Latin letters.
fn with_lines(dir: &std::path::Path) -> AppState {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let media = app.project.media[0].id;
    app.dispatch(Command::Append(Some(media)), Instant::now())
        .unwrap();
    let line = |i: i64, text: &str| NewLine {
        media,
        start: crate::time::Micros(i * 500_000),
        end: crate::time::Micros(i * 500_000 + 400_000),
        text: text.into(),
        words: vec![],
    };
    app.project.subtitles.replace_all(
        vec![
            line(0, "hello"),
            line(1, "مرحبا بالعالم"),
            line(2, "नमस्ते दुनिया"),
            line(3, "漢字 かな 한글"),
        ],
        None,
    );
    app
}

fn has_glyph(h: &Harness, c: char) -> bool {
    h.ctx
        .fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), c))
}

// covers: REQ-188
#[test]
fn the_subtitle_list_shows_arabic_and_hindi_with_the_bundled_faces_and_notes_missing_cjk() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_lines(dir.path());
    app.popups.show(Popup::Subtitles);
    let mut h = Harness::new(app, 1280.0, 900.0);
    // the faces are added once the list shows lines of those scripts
    for c in ['م', 'न'] {
        assert!(has_glyph(&h, c), "U+{:04X} has a glyph", c as u32);
    }
    for t in ["مرحبا بالعالم", "नमस्ते दुनिया"] {
        assert!(h.nodes.iter().any(|n| n.label.contains(t)), "{t} is listed");
    }
    // the note: no installed font covers the CJK scripts (a computer without one)
    crate::fonts::with_coverage(Some(BTreeSet::new()), || {
        h.idle(3);
        assert!(
            h.nodes.iter().any(|n| n
                .label
                .contains("No installed font shows Chinese, Japanese, Korean text")),
            "{:#?}",
            h.labels()
        );
    });
    // and none when an installed font covers them
    crate::fonts::with_coverage(
        Some([Script::Han, Script::Kana, Script::Hangul].into()),
        || {
            h.idle(3);
            assert!(
                !h.nodes
                    .iter()
                    .any(|n| n.label.contains("No installed font"))
            );
        },
    );
}

// covers: REQ-188
#[test]
fn a_context_without_the_faces_has_no_glyph_for_them_and_install_adds_them_once() {
    let ctx = eframe::egui::Context::default();
    let mut out = ctx.run_ui(Default::default(), |_| {});
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    let has = |c| ctx.fonts_mut(|f| f.has_glyph(&FontId::proportional(14.0), c));
    assert!(!has('م') && !has('न'));
    let found = scripts::scripts_of_lines(["a", "مرحبا", "नमस्ते", "مرحبا"]);
    assert_eq!(found, [Script::Arabic, Script::Devanagari]);
    scripts::install_for(&ctx, &found);
    scripts::install_for(&ctx, &found);
    let mut out = ctx.run_ui(Default::default(), |_| {});
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    assert!(has('م') && has('न'));
    assert!(scripts::is_installed(&ctx, Script::Arabic));
    assert!(!scripts::is_installed(&ctx, Script::Han));
    assert!(scripts::scripts_of_lines(["plain"]).is_empty());
    // a script no installed font covers adds nothing (and never fails)
    scripts::install_for(&ctx, &[Script::Han]);
}

// covers: REQ-188
#[test]
fn the_line_popup_shows_the_note_for_a_script_without_a_font() {
    let dir = tempfile::tempdir().unwrap();
    let app = with_lines(dir.path());
    let id = app.project.subtitles.lines[3].id;
    let clip = app.project.timeline.clips[0].id;
    let mut h = Harness::new(app, 1280.0, 900.0);
    crate::fonts::with_coverage(Some(BTreeSet::new()), || {
        h.app
            .dispatch(
                Command::OpenInstance(crate::app::Instance::Line(id, clip)),
                Instant::now(),
            )
            .unwrap();
        h.idle(4);
        assert!(
            h.nodes
                .iter()
                .any(|n| n.label.contains("No installed font shows")),
            "{:#?}",
            h.labels()
        );
    });
}

// covers: REQ-183, REQ-185
#[test]
fn the_subtitles_popup_has_an_import_button_and_offers_the_file_beside_the_video() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_lines(dir.path());
    std::fs::write(
        dir.path().join("a.srt"),
        "1\n00:00:01,000 --> 00:00:02,000\nfrom the file\n",
    )
    .unwrap();
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(Command::SelectClip(Some(clip)), Instant::now())
        .unwrap();
    app.dispatch(Command::OpenPopup(Popup::Subtitles), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 900.0);
    assert!(h.has("Import subtitles…"));
    assert!(h.has("Import a.srt"), "{:#?}", h.labels());
    assert_eq!(
        h.app.project.subtitles.lines.len(),
        4,
        "nothing is imported without a click"
    );
    h.click_label("Import a.srt");
    h.idle(3);
    assert_eq!(h.app.modal, Some(Modal::ImportSubtitles));
    assert!(h.has("Import subtitles"));
    assert!(h.has("Import") && h.has("Cancel"));
    h.click_label("Import");
    h.idle(3);
    assert!(h.app.modal.is_none());
    assert!(
        h.app
            .project
            .subtitles
            .lines
            .iter()
            .any(|l| l.text == "from the file")
    );
}

// covers: REQ-184
#[test]
fn the_import_dialog_offers_the_re_time_checkbox_only_where_speech_can_be_recognised() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_lines(dir.path());
    let file = dir.path().join("x.srt");
    std::fs::write(&file, "1\n00:00:01,000 --> 00:00:02,000\nx\n").unwrap();
    app.dispatch(
        Command::OpenImportSubtitles {
            path: file,
            clip: None,
        },
        Instant::now(),
    )
    .unwrap();
    assert!(
        app.pending_import.as_ref().unwrap().replaces_edits || !app.project.subtitles.has_edits()
    );
    let can = app.pending_import.as_ref().unwrap().can_retime;
    let mut h = Harness::new(app, 1280.0, 900.0);
    assert_eq!(h.has(modals::RETIME_LABEL), can);
    if can {
        h.click_label(modals::RETIME_LABEL);
        h.idle(2);
        assert!(
            h.app.pending_import.as_ref().unwrap().retime,
            "off by default, on after the click"
        );
    }
}

// covers: REQ-182
#[test]
fn the_export_dialog_has_a_subtitles_row_in_full_video_mode_burned_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_lines(dir.path());
    app.dispatch(Command::PickExport, Instant::now()).unwrap();
    let Some(Modal::Export(c)) = &app.modal else {
        panic!("the dialog is open")
    };
    assert_eq!(c.subtitles, crate::format::SubtitleMode::Burned);
    let mut h = Harness::new(app, 1280.0, 900.0);
    assert!(h.has(export_dialog::SUBTITLES_LABEL));
    assert!(h.has(crate::format::SubtitleMode::Burned.label()));
    // the script note shows in the dialog too
    crate::fonts::with_coverage(Some(BTreeSet::new()), || {
        h.idle(3);
        assert!(
            h.nodes
                .iter()
                .any(|n| n.label.contains("No installed font shows"))
        );
    });
}
