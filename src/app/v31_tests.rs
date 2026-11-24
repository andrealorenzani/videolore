// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! 3.1.0 in the application: the audio track of a clip (Volume and track, REQ-340) and the
//! audio track of the Subtitles effect (REQ-339).

use super::effects_tests::{
    TRACK, ids, runner_with_track, speech_ready, three, wait_create, with_tracks,
};
use super::tests::{FakeDialogs, runner, state, state_with};
use super::*;
use crate::media::AudioTrack;
use std::sync::{Arc, Mutex};

fn tracks(n: u8) -> Vec<AudioTrack> {
    (0..n)
        .map(|i| AudioTrack {
            ordinal: i,
            language: (i == 1).then(|| "eng".to_string()),
            title: None,
            channels: 2,
        })
        .collect()
}

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

// covers: REQ-340
#[test]
fn the_audio_tracks_of_a_file_are_listed_one_for_one_track_and_two_for_two() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    // the fake probe finds one sound
    assert_eq!(app.audio_track_labels(a).len(), 1);
    app.project.media[1].format.audio_tracks = Some(tracks(2));
    let labels = app.audio_track_labels(b);
    assert_eq!(
        labels,
        [
            (0, "Track 1 (stereo)".to_string()),
            (1, "English (stereo)".to_string())
        ]
    );
    // a file not analysed yet shows its one sound; a file without sound has none
    app.project.media[1].format.audio_tracks = None;
    assert_eq!(app.audio_track_labels(b), [(0, "Track 1".to_string())]);
    app.project.media[1].format.audio_tracks = Some(vec![]);
    assert!(app.audio_track_labels(b).is_empty());
    assert!(
        app.audio_track_labels(crate::media::MediaId(999))
            .is_empty()
    );
}

// covers: REQ-340
#[test]
fn choosing_the_track_is_one_undo_step_and_a_missing_track_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    app.project.media[1].format.audio_tracks = Some(tracks(2));
    let c = ids(&app);
    let now = Instant::now();
    // the clip without the effect uses the first track
    assert_eq!(app.project.timeline.clips[1].audio_track, 0);
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetClipAudioTrack {
            clip: c[1],
            track: 0,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n, "the same track is no edit");
    app.dispatch(
        Command::SetClipAudioTrack {
            clip: c[1],
            track: 1,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.timeline.clips[1].audio_track, 1);
    assert_eq!(app.project.timeline.clips[0].audio_track, 0);
    assert_eq!(app.history.undo_len(), n + 1);
    // a track the file does not have, and the first clip's file has only one
    assert!(
        app.dispatch(
            Command::SetClipAudioTrack {
                clip: c[1],
                track: 2
            },
            now
        )
        .is_err()
    );
    assert!(
        app.dispatch(
            Command::SetClipAudioTrack {
                clip: c[0],
                track: 1
            },
            now
        )
        .is_err()
    );
    assert_eq!(app.history.undo_label(), Some("Audio track"));
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.timeline.clips[1].audio_track, 0);
}

// covers: REQ-340
#[test]
fn merge_needs_the_same_track_and_the_popup_asks_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    app.project.media[0].format.audio_tracks = Some(tracks(2));
    let now = Instant::now();
    let a = app.project.media[0].id;
    // two contiguous parts of the first file
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    let tl = &mut app.project.timeline;
    let last = tl.clips.len() - 1;
    let id = tl.clips[last].id;
    tl.split(tl.start_of(last) + s(2), Micros(1)).unwrap();
    let right = tl.clips[last + 1].id;
    assert!(tl.can_merge(id));
    tl.set_audio_track(right, 1).unwrap();
    assert!(!tl.can_merge(id));
    assert_eq!(
        tl.merge_block_reason(id),
        Some(crate::timeline::DIFFERENT_AUDIO_TRACKS)
    );
    tl.set_audio_track(right, 0).unwrap();
    assert!(tl.can_merge(id));
    // the effect is named "Volume and track" in the list, the popup and the applied rows
    assert_eq!(
        EffectKind::Volume.spec().name,
        "Volume and track",
        "the Effects list"
    );
    assert_eq!(app.popup_title(Popup::Volume), "Volume and track");
}

/// A runner that remembers the arguments of every call.
fn recording(
    inner: crate::ffmpeg::fake::FakeRunner,
) -> (
    crate::ffmpeg::fake::FakeRunner,
    Arc<Mutex<Vec<Vec<String>>>>,
) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let mut r = inner;
    r.responders.insert(
        0,
        Box::new(move |_, a: &[String]| {
            log.lock().unwrap().push(a.to_vec());
            None
        }),
    );
    (r, seen)
}

fn extraction_tracks(seen: &Arc<Mutex<Vec<Vec<String>>>>) -> Vec<String> {
    seen.lock()
        .unwrap()
        .iter()
        .filter(|a| a.iter().any(|x| x == "f32le"))
        .filter_map(|a| a.windows(2).find(|w| w[0] == "-map").map(|w| w[1].clone()))
        .collect()
}

// covers: REQ-339
#[test]
fn the_subtitles_effect_takes_the_speech_from_the_chosen_audio_track() {
    let dir = tempfile::tempdir().unwrap();
    let (r, seen) = recording(runner());
    let mut app = state_with(FakeDialogs::default(), r);
    three(&mut app, dir.path());
    if !stt::available() {
        return;
    }
    speech_ready(&mut app, dir.path());
    app.project.media[1].format.audio_tracks = Some(tracks(3));
    let now = Instant::now();
    let c = ids(&app);
    // no choice: the first track, as before
    let apply = Command::ApplyEffect {
        effect: EffectKind::Subtitles,
        target: Target::Clip(c[1]),
    };
    app.dispatch(apply.clone(), now).unwrap();
    wait_create(&mut app);
    assert_eq!(extraction_tracks(&seen), ["0:a:0"]);
    // the third track: with no subtitle track in the file, Recognise uses it
    app.dispatch(Command::SetSubtitleAudio(2), now).unwrap();
    app.dispatch(apply.clone(), now).unwrap();
    wait_create(&mut app);
    assert_eq!(extraction_tracks(&seen), ["0:a:0", "0:a:2"]);
    // nothing about it is stored in the project
    assert!(
        !crate::project::to_json(&app.project, Path::new("/tmp/x.vlproj"))
            .unwrap()
            .contains("subtitle_audio")
    );
    // a track the file does not have falls back to the first one
    app.dispatch(Command::SetSubtitleAudio(9), now).unwrap();
    app.dispatch(apply, now).unwrap();
    wait_create(&mut app);
    assert_eq!(extraction_tracks(&seen).last().unwrap(), "0:a:0");
}

// covers: REQ-339
#[test]
fn with_a_subtitle_track_the_lines_keep_its_text_and_the_matching_uses_the_chosen_track() {
    let dir = tempfile::tempdir().unwrap();
    let (r, seen) = recording(runner_with_track(TRACK));
    let mut app = state_with(FakeDialogs::default(), r);
    three(&mut app, dir.path());
    if !stt::available() {
        return;
    }
    speech_ready(&mut app, dir.path());
    with_tracks(&mut app, 1, &[("eng", "English")]);
    app.project.media[1].format.audio_tracks = Some(tracks(2));
    let now = Instant::now();
    let c = ids(&app);
    app.dispatch(Command::SetSubtitleAudio(1), now).unwrap();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    wait_create(&mut app);
    let texts: Vec<&str> = app
        .project
        .subtitles
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(texts, ["w0", "w1 w2"], "the text of the track is kept");
    assert_eq!(
        extraction_tracks(&seen),
        ["0:a:1"],
        "the words come from track 2"
    );
}
