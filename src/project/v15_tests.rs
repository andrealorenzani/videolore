// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Project format 15 (3.1.0): the audio track of a clip, the audio tracks of a media file and
//! the subtitle style "As subtitle track"; the refusal by older programs (REQ-029, REQ-340,
//! REQ-337).

use super::*;
use crate::media::AudioTrack;
use crate::subtitles::Preset;

fn json_of(p: &Project) -> String {
    to_json(p, std::path::Path::new("/tmp/x.vlproj")).unwrap()
}

fn reopened(json: &str) -> (Project, Vec<String>) {
    let (p, mut notes, _) = from_json_noted(json, std::path::Path::new("/tmp")).unwrap();
    notes.retain(|n| !n.ends_with(".mp4"));
    (p, notes)
}

fn track(i: u8) -> AudioTrack {
    AudioTrack {
        ordinal: i,
        language: Some("eng".into()),
        title: Some(format!("Track {i}")),
        channels: 2,
    }
}

/// One clip of 4 s from a file with two audio tracks.
fn two_tracks() -> Project {
    let mut p = Project::default();
    let id = p.new_media_id();
    let mut m = crate::timeline::tests::media(id.0, 4, true);
    m.format.audio_tracks = Some(vec![track(0), track(1)]);
    p.add_media(m.clone());
    p.timeline.append(&m).unwrap();
    p
}

// covers: REQ-340, REQ-029
#[test]
fn format_15_is_written_and_read_with_the_tracks() {
    let mut p = two_tracks();
    let c = p.timeline.clips[0].id;
    p.timeline.set_audio_track(c, 1).unwrap();
    p.subtitles.preset = Preset::AsTrack;
    let json = json_of(&p);
    assert!(json.contains("\"schema_version\": 15"), "{json}");
    assert!(json.contains("\"audio_track\": 1") && json.contains("as_track"));
    let (q, notes) = reopened(&json);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(q.timeline.clips[0].audio_track, 1);
    assert_eq!(q.subtitles.preset, Preset::AsTrack);
    assert_eq!(q.media[0].format.audio_tracks.as_ref().unwrap().len(), 2);
    // a 16 is refused untouched
    let newer = json.replace("\"schema_version\": 15", "\"schema_version\": 16");
    assert_eq!(
        from_json(&newer, std::path::Path::new("/tmp")).unwrap_err(),
        ProjectError::TooNew(16)
    );
}

// covers: REQ-340
#[test]
fn a_project_without_the_new_settings_writes_none_of_them_and_a_14_file_opens_with_defaults() {
    let mut p = Project::default();
    let id = p.new_media_id();
    let m = crate::timeline::tests::media(id.0, 4, true);
    p.add_media(m.clone());
    p.timeline.append(&m).unwrap();
    let json = json_of(&p);
    // an old project looks the same as before, apart from its number
    assert!(!json.contains("audio_track"), "{json}");
    assert!(!json.contains("as_track"));
    let old = json.replace("\"schema_version\": 15", "\"schema_version\": 14");
    let (q, notes) = reopened(&old);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(q.timeline.clips[0].audio_track, 0);
    assert_eq!(q.media[0].format.audio_tracks, None, "not analysed yet");
    // saving it again writes 15
    assert!(json_of(&q).contains("\"schema_version\": 15"));
}

// covers: REQ-340
#[test]
fn a_track_the_file_does_not_have_goes_back_to_the_first_with_one_line() {
    let p = two_tracks();
    let json = json_of(&p);
    let set = |track: u64, clips: usize| {
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        v["timeline"]["clips"][0]["audio_track"] = track.into();
        let (q, notes) = reopened(&v.to_string());
        assert_eq!(q.timeline.clips.len(), clips);
        (q.timeline.clips[0].audio_track, notes)
    };
    assert_eq!(set(1, 1), (1, vec![]));
    let (t, notes) = set(2, 1);
    assert_eq!(t, 0);
    assert_eq!(notes, ["1 clip track choices reset"]);
    // a file that was never analysed only has the first track
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["media"][0]["format"]
        .as_object_mut()
        .unwrap()
        .remove("audio_tracks");
    v["timeline"]["clips"][0]["audio_track"] = 1.into();
    let (q, notes) = reopened(&v.to_string());
    assert_eq!(q.timeline.clips[0].audio_track, 0);
    assert_eq!(notes.len(), 1);
    // a number that is not a byte is a damaged file, refused as any other
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["timeline"]["clips"][0]["audio_track"] = 300.into();
    assert!(from_json(&v.to_string(), std::path::Path::new("/tmp")).is_err());
}

// covers: REQ-340
#[test]
fn the_audio_tracks_of_a_media_file_are_made_safe_when_a_project_opens() {
    let p = two_tracks();
    let mut v: serde_json::Value = serde_json::from_str(&json_of(&p)).unwrap();
    let tracks = &mut v["media"][0]["format"]["audio_tracks"];
    tracks[0]["title"] = "x\u{7}y".into();
    tracks[0]["language"] = "bad code!".into();
    tracks[1]["channels"] = 100_000.into();
    let (q, _) = reopened(&v.to_string());
    let t = q.media[0].format.audio_tracks.as_ref().unwrap();
    assert_eq!(t[0].title.as_deref(), Some("x\u{FFFD}y"));
    assert_eq!(t[0].language, None);
    assert_eq!(t[1].channels, 64);
    // too many tracks are cut
    let one = v["media"][0]["format"]["audio_tracks"][0].clone();
    v["media"][0]["format"]["audio_tracks"] = serde_json::Value::Array(vec![one; 1000]);
    let (q, _) = reopened(&v.to_string());
    assert_eq!(
        q.media[0].format.audio_tracks.as_ref().unwrap().len(),
        crate::media::MAX_AUDIO_TRACKS
    );
}

// covers: REQ-340
#[test]
fn a_later_check_of_a_file_records_its_audio_tracks_without_making_the_project_unsaved() {
    let mut p = Project::default();
    let id = p.new_media_id();
    let m = crate::timeline::tests::media(id.0, 4, true);
    p.add_media(m);
    let mut saved = p.doc();
    let before = p.doc();
    saved.set_audio_tracks(id, vec![track(0)]);
    assert_ne!(saved, before);
    // the project found the same tracks: it equals what was saved, so it is not unsaved
    p.media[0].format.audio_tracks = Some(vec![track(0)]);
    assert_eq!(p.doc(), saved);
}
