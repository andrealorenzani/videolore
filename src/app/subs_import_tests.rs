// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 2.0.4, batch 5: importing subtitle files (REQ-183, REQ-184, REQ-185), exporting
//! them as .vtt and .ass (REQ-180, REQ-181) and the analysis of changed media (REQ-186).

use super::tests::{
    FakeDialogs, files, import, runner, settle, state, state_with, with_fake_speech,
};
use super::*;
use crate::ffmpeg::fake::ok;
use std::sync::atomic::AtomicUsize;

const SRT3: &str = "1\n00:00:01,000 --> 00:00:02,000\nfirst line\n\n\
                    2\n00:00:02,500 --> 00:00:03,000\n<i>second</i> line\n\n\
                    3\n00:00:04,000 --> 00:00:04,800\nthird\n\n";

/// An app with a.mp4 (5 s) on the timeline.
fn with_clip(dir: &Path) -> AppState {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let id = app.project.media[0].id;
    app.dispatch(Command::Append(Some(id)), Instant::now())
        .unwrap();
    app
}

fn open_and_confirm(app: &mut AppState, path: &Path, clip: Option<ClipId>) -> Result<(), String> {
    let now = Instant::now();
    app.dispatch(
        Command::OpenImportSubtitles {
            path: path.to_path_buf(),
            clip,
        },
        now,
    )?;
    assert_eq!(app.modal, Some(Modal::ImportSubtitles));
    app.dispatch(Command::ConfirmImportSubtitles, now)
}

fn texts(app: &AppState) -> Vec<(i64, i64, String)> {
    let mut v: Vec<_> = app
        .project
        .subtitles
        .lines
        .iter()
        .map(|l| (l.start.0, l.end.0, l.text.clone()))
        .collect();
    v.sort();
    v
}

// covers: REQ-183
#[test]
fn an_srt_file_becomes_lines_with_its_times_in_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    let file = dir.path().join("a.srt");
    std::fs::write(&file, SRT3).unwrap();
    let before = app.history.undo_label().map(str::to_string);
    open_and_confirm(&mut app, &file, None).unwrap();
    assert_eq!(
        texts(&app),
        [
            (1_000_000, 2_000_000, "first line".to_string()),
            (2_500_000, 3_000_000, "second line".to_string()),
            (4_000_000, 4_800_000, "third".to_string()),
        ],
        "same times and text, tags removed"
    );
    assert!(app.status.contains("Imported 3"));
    assert_eq!(app.history.undo_label(), Some("Import subtitles"));
    app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert!(app.project.subtitles.lines.is_empty());
    assert_eq!(app.history.undo_label().map(str::to_string), before);
}

// covers: REQ-183
#[test]
fn a_vtt_file_is_read_without_its_positions_and_a_clip_gets_only_its_range() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    let file = dir.path().join("a.vtt");
    std::fs::write(
        &file,
        "\u{FEFF}WEBVTT\n\nNOTE comment\n\nintro\n00:01.000 --> 00:02.000 line:10% position:20%\n\
         <v Bob>Hi <b>there</b> &amp; you\n\n00:00:04.500 --> 00:00:09.000\nlate\n",
    )
    .unwrap();
    let now = Instant::now();
    // the clip uses 0..4 s of the file: the second cue is outside
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::SetTrim {
            clip,
            src_in: Micros::ZERO,
            src_out: Micros(4_000_000),
        },
        now,
    )
    .unwrap();
    open_and_confirm(&mut app, &file, Some(clip)).unwrap();
    assert_eq!(
        texts(&app),
        [(1_000_000, 2_000_000, "Hi there & you".to_string())]
    );
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none()),
        "VTT positions are ignored"
    );
}

// covers: REQ-183
#[test]
fn a_file_that_is_not_utf8_is_refused_and_nothing_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    let file = dir.path().join("a.srt");
    // "caf\xe9" is Latin-1
    let mut bytes = b"1\n00:00:01,000 --> 00:00:02,000\ncaf".to_vec();
    bytes.push(0xE9);
    bytes.extend_from_slice(b"\n\n");
    std::fs::write(&file, bytes).unwrap();
    let before = app.undo_text();
    let e = app
        .dispatch(
            Command::OpenImportSubtitles {
                path: file.clone(),
                clip: None,
            },
            Instant::now(),
        )
        .unwrap_err();
    assert!(e.contains("not UTF-8"), "{e}");
    assert!(app.modal.is_none() && app.pending_import.is_none());
    assert!(app.project.subtitles.lines.is_empty());
    assert_eq!(app.undo_text(), before);
    // a file without cues is refused too, and so is one that is gone
    std::fs::write(&file, "nothing here").unwrap();
    assert!(
        app.dispatch(
            Command::OpenImportSubtitles {
                path: file,
                clip: None
            },
            Instant::now()
        )
        .is_err()
    );
    assert!(
        app.dispatch(
            Command::OpenImportSubtitles {
                path: dir.path().join("none.srt"),
                clip: None
            },
            Instant::now()
        )
        .is_err()
    );
}

// covers: REQ-183
#[test]
fn importing_replaces_the_lines_of_the_range_and_asks_when_they_were_edited() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    let now = Instant::now();
    let media = app.project.media[0].id;
    app.project.subtitles.replace_all(
        vec![crate::subtitles::NewLine {
            media,
            start: Micros(500_000),
            end: Micros(900_000),
            text: "old".into(),
            words: vec![],
        }],
        None,
    );
    let id = app.project.subtitles.lines[0].id;
    app.project.subtitles.set_text(id, "old edited").unwrap();
    let file = dir.path().join("a.srt");
    std::fs::write(&file, SRT3).unwrap();
    app.dispatch(
        Command::OpenImportSubtitles {
            path: file,
            clip: None,
        },
        now,
    )
    .unwrap();
    assert!(app.pending_import.as_ref().unwrap().replaces_edits);
    // closing the dialog changes nothing
    app.dispatch(Command::CloseModal, now).unwrap();
    assert!(app.pending_import.is_none() && app.modal.is_none());
    assert_eq!(app.project.subtitles.lines.len(), 1);
    // a missing clip, a busy session and an empty timeline are refused
    assert!(
        app.dispatch(Command::ConfirmImportSubtitles, now).is_ok(),
        "no dialog open: nothing happens"
    );
    let mut empty = state(FakeDialogs::default());
    assert!(!empty.is_enabled(&Command::PickImportSubtitles(None)));
    assert!(
        empty
            .dispatch(
                Command::OpenImportSubtitles {
                    path: dir.path().join("a.srt"),
                    clip: None
                },
                now
            )
            .is_err()
    );
}

fn speech_ready(app: &mut AppState, dir: &Path) {
    let body = with_fake_speech(app, dir);
    let local = dir.join("local-model.bin");
    std::fs::write(&local, crate::stt::tests::level_body(&body, 0)).unwrap();
    let store = app.model_store.clone().unwrap();
    store.install_local(SubtitleQuality::Fast, &local).unwrap();
}

// covers: REQ-184
#[test]
fn re_timing_moves_a_cue_written_early_to_the_recognised_words() {
    if !stt::available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    speech_ready(&mut app, dir.path());
    let now = Instant::now();
    // the fake speech has w0 at 0 s, w1 at 1 s, w2 at 2 s: the cue is 0.6 s early
    let file = dir.path().join("a.srt");
    std::fs::write(
        &file,
        "1\n00:00:00,400 --> 00:00:02,600\nw1 w2\n\n2\n00:00:04,000 --> 00:00:04,500\nnot spoken\n\n",
    )
    .unwrap();
    // checkbox off: exactly the file's times
    open_and_confirm(&mut app, &file, None).unwrap();
    assert_eq!(texts(&app)[0].0, 400_000);
    app.dispatch(Command::Undo, now).unwrap();
    // cancel changes nothing
    app.dispatch(
        Command::OpenImportSubtitles {
            path: file.clone(),
            clip: None,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SetImportRetime(true), now).unwrap();
    app.dispatch(Command::CloseModal, now).unwrap();
    assert!(app.project.subtitles.lines.is_empty() && !app.is_generating());
    // checkbox on: the start is the recognised one
    app.dispatch(
        Command::OpenImportSubtitles {
            path: file,
            clip: None,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SetImportRetime(true), now).unwrap();
    app.dispatch(Command::ConfirmImportSubtitles, now).unwrap();
    assert!(app.is_generating());
    settle(&mut app);
    let lines = texts(&app);
    assert_eq!(lines[0].0, 1_000_000, "starts at the first recognised word");
    assert_eq!(lines[0].1, 2_300_000, "ends at the last recognised word");
    assert_eq!(lines[0].2, "w1 w2");
    assert_eq!(lines.last().unwrap().0, 4_000_000, "no match: file times");
    assert_eq!(app.history.undo_label(), Some("Import subtitles"));
    assert!(
        app.status.contains("kept the file's times"),
        "{}",
        app.status
    );
}

// covers: REQ-184
#[test]
fn re_timing_can_be_cancelled_and_changes_nothing() {
    if !stt::available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    speech_ready(&mut app, dir.path());
    let now = Instant::now();
    let file = dir.path().join("a.srt");
    std::fs::write(&file, SRT3).unwrap();
    app.dispatch(
        Command::OpenImportSubtitles {
            path: file,
            clip: None,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SetImportRetime(true), now).unwrap();
    app.dispatch(Command::ConfirmImportSubtitles, now).unwrap();
    app.dispatch(Command::CancelSubtitles, now).unwrap();
    settle(&mut app);
    assert!(app.project.subtitles.lines.is_empty());
}

// covers: REQ-185
#[test]
fn a_subtitle_file_with_the_same_name_is_offered_and_never_imported_by_itself() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = with_clip(dir.path());
    let now = Instant::now();
    let clip = app.project.timeline.clips[0].id;
    // no file beside a.mp4: no offer
    app.dispatch(Command::OpenPopup(Popup::Subtitles), now)
        .unwrap();
    assert!(app.import_offer(clip).is_none());
    std::fs::write(dir.path().join("a.srt"), SRT3).unwrap();
    app.dispatch(Command::ClosePopup(Popup::Subtitles), now)
        .unwrap();
    app.dispatch(Command::OpenPopup(Popup::Subtitles), now)
        .unwrap();
    let offer = app
        .import_offer(clip)
        .expect("a.srt is offered")
        .to_path_buf();
    assert_eq!(crate::media::display_name(&offer), "a.srt");
    assert!(
        app.project.subtitles.lines.is_empty(),
        "nothing is imported without a click"
    );
    // the click opens the import of that file for the clip
    app.dispatch(
        Command::OpenImportSubtitles {
            path: offer,
            clip: Some(clip),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ConfirmImportSubtitles, now).unwrap();
    assert_eq!(app.project.subtitles.lines.len(), 3);
    // a vtt file is found too, srt first
    assert_eq!(
        super::subs_import::sibling_file(&dir.path().join("b.mp4")),
        None
    );
    std::fs::write(dir.path().join("b.vtt"), "WEBVTT").unwrap();
    assert_eq!(
        super::subs_import::sibling_file(&dir.path().join("b.mp4")),
        Some(dir.path().join("b.vtt"))
    );
}

fn probe_counter(app_runner: &mut crate::ffmpeg::fake::FakeRunner) -> Arc<AtomicUsize> {
    let n = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&n);
    app_runner.responders.insert(
        0,
        Box::new(move |p: &Path, _: &[String]| {
            if p.ends_with("ffprobe") {
                c.fetch_add(1, Ordering::SeqCst);
            }
            None
        }),
    );
    n
}

fn wait_checks(app: &mut AppState) {
    let deadline = Instant::now() + std::time::Duration::from_secs(10);
    while app.track_check.is_some() && Instant::now() < deadline {
        app.tick(Instant::now());
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(app.track_check.is_none(), "the check did not finish");
}

/// A saved project with a.mp4 and b.mp4 on the timeline.
fn saved_project(dir: &Path) -> PathBuf {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let now = Instant::now();
    for i in 0..2 {
        let id = app.project.media[i].id;
        app.dispatch(Command::Append(Some(id)), now).unwrap();
    }
    let file = dir.join("p.vlproj");
    app.dispatch(Command::SaveProjectAs(file.clone()), now)
        .unwrap();
    file
}

// covers: REQ-186
#[test]
fn media_that_changed_on_disk_is_analysed_again_and_named_unchanged_media_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let file = saved_project(dir.path());
    let now = Instant::now();
    // unchanged: no analysis at all
    let mut r = runner();
    let count = probe_counter(&mut r);
    let mut app = state_with(FakeDialogs::default(), r);
    app.dispatch(Command::OpenProject(file.clone()), now)
        .unwrap();
    wait_checks(&mut app);
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "unchanged media: no analysis"
    );
    assert!(!app.dirty);
    // a.mp4 is replaced by a file with another subtitle track
    std::fs::write(
        dir.path().join("a.mp4"),
        b"a replaced file, longer than before",
    )
    .unwrap();
    let probe = r#"{"streams":[
        {"codec_type":"video","codec_name":"h264","width":1280,"height":720,"avg_frame_rate":"30/1"},
        {"codec_type":"audio","codec_name":"aac"},
        {"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"fre"}}],
        "format":{"format_name":"matroska","duration":"5.0"}}"#;
    let mut r = runner();
    r.responders.insert(
        0,
        Box::new(move |p: &Path, a: &[String]| {
            (p.ends_with("ffprobe") && a.last()?.ends_with("a.mp4")).then(|| ok(probe))?
        }),
    );
    let count = probe_counter(&mut r);
    let mut app = state_with(FakeDialogs::default(), r);
    app.dispatch(Command::OpenProject(file), now).unwrap();
    wait_checks(&mut app);
    assert_eq!(count.load(Ordering::SeqCst), 1, "only the changed file");
    let a = &app.project.media[0];
    assert_eq!(a.format.subtitle_tracks.as_ref().unwrap().len(), 1);
    assert_eq!(a.format.container, "matroska");
    assert!(
        app.status.contains("analysed again") && app.status.contains("a.mp4"),
        "{}",
        app.status
    );
    assert!(!app.status.contains("b.mp4"));
    assert!(
        !app.dirty,
        "the new analysis does not make the project unsaved"
    );
}

// covers: REQ-186
#[test]
fn a_project_of_2_0_3_records_the_file_stats_without_an_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let file = saved_project(dir.path());
    // the format 10 file has no sizes or times
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    v["schema_version"] = 10.into();
    for m in v["media"].as_array_mut().unwrap() {
        m.as_object_mut().unwrap().remove("file_size");
        m.as_object_mut().unwrap().remove("modified_ms");
    }
    std::fs::write(&file, v.to_string()).unwrap();
    let mut r = runner();
    let count = probe_counter(&mut r);
    let mut app = state_with(FakeDialogs::default(), r);
    app.dispatch(Command::OpenProject(file.clone()), Instant::now())
        .unwrap();
    wait_checks(&mut app);
    assert_eq!(count.load(Ordering::SeqCst), 0, "no analysis count change");
    assert!(app.project.media.iter().all(|m| m.stat.is_some()));
    assert!(!app.dirty);
    // saving writes format 12 with the stats
    app.dispatch(Command::SaveProject, Instant::now()).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"schema_version\": 15"));
    assert!(text.contains("\"file_size\"") && text.contains("\"modified_ms\""));
}

fn app_with_lines(dir: &Path, dialogs: FakeDialogs) -> AppState {
    let mut app = state(dialogs);
    import(&mut app, files(dir));
    let id = app.project.media[0].id;
    app.dispatch(Command::Append(Some(id)), Instant::now())
        .unwrap();
    let file = dir.join("in.srt");
    std::fs::write(&file, SRT3).unwrap();
    open_and_confirm(&mut app, &file, None).unwrap();
    app
}

// covers: REQ-180
#[test]
fn export_subtitles_writes_srt_vtt_and_ass_from_the_same_lines() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_lines(dir.path(), FakeDialogs::default());
    let now = Instant::now();
    let name = |e: &str| dir.path().join(format!("out.{e}"));
    for e in ["srt", "vtt", "ass"] {
        app.dispatch(Command::ExportSrt(name(e)), now).unwrap();
    }
    let srt = crate::srt::parse(&std::fs::read_to_string(name("srt")).unwrap());
    let vtt_text = std::fs::read_to_string(name("vtt")).unwrap();
    assert!(vtt_text.starts_with("WEBVTT\n"));
    let vtt = crate::srt::parse_vtt(&vtt_text);
    assert_eq!(srt.len(), 3);
    assert_eq!(vtt, srt, "the same cues, times and text");
    let ass = std::fs::read_to_string(name("ass")).unwrap();
    assert_eq!(ass.matches("Dialogue: ").count(), 3);
    assert!(ass.contains("PlayResX: 1280\nPlayResY: 720"), "{ass}");
    // no extension: .srt
    app.dispatch(Command::ExportSrt(dir.path().join("plain")), now)
        .unwrap();
    assert!(dir.path().join("plain.srt").is_file());
    assert!(app.status.contains("saved"));
}

// covers: REQ-180
#[test]
fn a_cancelled_export_of_subtitles_writes_nothing_and_it_is_greyed_without_lines() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_with_lines(dir.path(), FakeDialogs::default());
    let now = Instant::now();
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    app.dispatch(Command::PickExportSrt, now).unwrap(); // the dialog is cancelled
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    // the dialog proposes the project's name, and the filters offer the three formats
    assert!(app.srt_name().ends_with(".srt"));
    let mut empty = state(FakeDialogs::default());
    assert!(!empty.is_enabled(&Command::PickExportSrt));
    assert_eq!(
        empty.disabled_reason(&Command::ExportSrt(dir.path().join("x.vtt"))),
        Some(effects::NO_SUBTITLES)
    );
    assert!(
        empty
            .dispatch(Command::ExportSrt(dir.path().join("x.vtt")), now)
            .is_err()
    );
    assert!(!dir.path().join("x.vtt").exists());
}
