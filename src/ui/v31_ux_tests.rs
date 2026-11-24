// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 3.1.0: Overlap media as its own row of the Effects list (REQ-336), the
//! Audio track choice of Volume and track (REQ-340) and of the Subtitles popup (REQ-339).

use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Instance, Popup};
use crate::media::AudioTrack;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

fn loaded(dialogs: FakeDialogs) -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(dialogs);
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    (dir, app)
}

fn two_tracks(app: &mut AppState, media: usize) {
    app.project.media[media].format.audio_tracks = Some(
        (0..2)
            .map(|i| AudioTrack {
                ordinal: i,
                language: (i == 1).then(|| "eng".to_string()),
                title: None,
                channels: 2,
            })
            .collect(),
    );
}

// covers: REQ-336
#[test]
fn overlap_media_is_a_row_of_its_own_like_the_effect_rows() {
    let dir = tempfile::tempdir().unwrap();
    let logo = dir.path().join("logo.png");
    std::fs::write(&logo, crate::picture::tests::header(400, 200)).unwrap();
    let (_d, app) = loaded(FakeDialogs {
        picture: Some(logo),
        ..Default::default()
    });
    let mut h = Harness::new(app, 1280.0, 800.0);
    let (text, overlap) = (
        h.find(effects::ADD_TEXT_ROW).expect("Add Text"),
        h.find(effects::OVERLAP_MEDIA_ROW).expect("Overlap media"),
    );
    // two rows, one under the other, as wide as an effect row
    assert!(
        overlap.rect.min.y >= text.rect.max.y - 0.5,
        "{text:?} {overlap:?}"
    );
    assert!((overlap.rect.min.x - text.rect.min.x).abs() < 1.0);
    assert!((overlap.rect.width() - text.rect.width()).abs() < 1.0);
    let fade = h.find("Fade IN/OUT").expect("an effect row");
    assert!(
        (overlap.rect.width() - fade.rect.width()).abs() < 1.0,
        "same width as an effect"
    );
    assert!((overlap.rect.min.x - fade.rect.min.x).abs() < 1.0);
    // the other rows are still there
    for name in ["Subtitles", "Volume and track"] {
        assert!(h.has(name), "{name} in {:?}", h.labels());
    }
    // a click adds the overlap as before
    h.click(overlap.rect.center());
    let c0 = h.app.project.timeline.clips[0].id;
    assert_eq!(h.app.project.timeline.overlaps_of(c0).len(), 1);
}

// covers: REQ-340
#[test]
fn the_volume_popup_lists_the_tracks_of_the_file_and_choosing_one_is_an_undo_step() {
    let (_d, mut app) = loaded(FakeDialogs::default());
    two_tracks(&mut app, 0);
    let c = app.project.timeline.clips[0].id;
    app.selected_clip = Some(c);
    app.popups.instance = Some(Instance::ClipVolume(c));
    app.popups.show(Popup::Instance);
    let mut h = Harness::new(app, 1280.0, 800.0);
    assert!(h.has(effects::AUDIO_TRACK), "{:?}", h.labels());
    // the first track is chosen by default; opening the list shows both
    h.click_label("Track 1 (stereo)");
    assert!(h.has("English (stereo)"), "{:?}", h.labels());
    h.click_label("English (stereo)");
    assert_eq!(h.app.project.timeline.clips[0].audio_track, 1);
    assert_eq!(h.app.history.undo_label(), Some("Audio track"));
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(h.app.project.timeline.clips[0].audio_track, 0);
}

// covers: REQ-340
#[test]
fn a_file_with_one_audio_track_shows_one_choice() {
    let (_d, mut app) = loaded(FakeDialogs::default());
    let c = app.project.timeline.clips[0].id;
    app.selected_clip = Some(c);
    app.popups.instance = Some(Instance::ClipVolume(c));
    app.popups.show(Popup::Instance);
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.click_label("Track 1 (stereo)");
    // the list shows the one track (the closed combo box shows it too) and no other
    let shown: Vec<String> = h
        .nodes
        .iter()
        .filter(|n| n.label.starts_with("Track "))
        .map(|n| n.label.clone())
        .collect();
    assert!(!shown.is_empty(), "{:#?}", h.labels());
    assert!(shown.iter().all(|l| l == "Track 1 (stereo)"), "{shown:?}");
    assert!(!h.has("English (stereo)"));
}

// covers: REQ-339, REQ-340
#[test]
fn the_subtitles_popup_lists_the_same_tracks_and_remembers_the_choice_for_the_next_creation() {
    let (_d, mut app) = loaded(FakeDialogs::default());
    two_tracks(&mut app, 0);
    let c = app.project.timeline.clips[0].id;
    app.selected_clip = Some(c);
    app.popups.show(Popup::Subtitles);
    let mut h = Harness::new(app, 1280.0, 800.0);
    assert!(h.has(effects::AUDIO_TRACK), "{:?}", h.labels());
    h.click_label("Track 1 (stereo)");
    assert!(h.has("English (stereo)"), "{:?}", h.labels());
    let before = h.app.history.undo_len();
    h.click_label("English (stereo)");
    assert_eq!(h.app.popups.subtitle_audio, 1);
    // it is not an edit
    assert_eq!(h.app.history.undo_len(), before);
}
