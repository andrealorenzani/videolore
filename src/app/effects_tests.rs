// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 1.2.0: effects on a clip or the whole video, clip volume, edge fades,
//! subtitles for one clip, the video's own subtitles, line selection and position, SRT.

use super::tests::{
    FakeDialogs, files, import, runner, settle, state, state_with, with_fake_speech,
};
use super::*;
use crate::ffmpeg::fake::{FakeRunner, ok};
use crate::media::{SubtitleTrack, TextCodec};
use crate::subtitles::NewLine;
use std::time::Duration;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Three clips: a.mp4, b.mp4 trimmed to [1 s, 4 s), a.mp4 again (5 + 3 + 5 s).
pub(super) fn three(app: &mut AppState, dir: &Path) {
    let f = files(dir);
    import(app, f);
    let now = Instant::now();
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    for m in [a, b, a] {
        app.dispatch(Command::Append(Some(m)), now).unwrap();
    }
    let c = app.project.timeline.clips[1].id;
    app.dispatch(
        Command::SetTrim {
            clip: c,
            src_in: s(1),
            src_out: s(4),
        },
        now,
    )
    .unwrap();
}

pub(super) fn ids(app: &AppState) -> Vec<ClipId> {
    app.project.timeline.clips.iter().map(|c| c.id).collect()
}

/// The speech file installed, so creation starts at once.
pub(super) fn speech_ready(app: &mut AppState, dir: &Path) {
    let body = with_fake_speech(app, dir);
    let local = dir.join("local-model.bin");
    std::fs::write(&local, crate::stt::tests::level_body(&body, 0)).unwrap();
    let store = app.model_store.clone().unwrap();
    store.install_local(SubtitleQuality::Fast, &local).unwrap();
}

/// Lines in media a (0–1 s) and b (0.2–0.8 s and 1.5–2.5 s), not edited.
fn seed_lines(app: &mut AppState) {
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    let line = |media, start, end, text: &str| NewLine {
        media,
        start,
        end,
        text: text.into(),
        words: vec![],
    };
    app.project.subtitles.replace_all(
        vec![
            line(a, Micros(0), s(1), "in a"),
            line(b, Micros(200_000), Micros(800_000), "b before the clip"),
            line(b, Micros(1_500_000), Micros(2_500_000), "b inside"),
        ],
        None,
    );
}

pub(super) fn wait_create(app: &mut AppState) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while (app.stt.is_some() || app.pending_create.is_some() && app.modal.is_none())
        && Instant::now() < deadline
    {
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(2));
    }
    settle(app);
}

// covers: REQ-147, REQ-148, REQ-082
#[test]
fn a_whole_video_effect_is_one_copy_per_clip_one_undo_step_and_stays_with_its_clips() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let now = Instant::now();
    let c = ids(&app);
    let before = app.project.edit();
    // Scale on the whole video: each of the three clips gets its own copy, one undo step
    let undo_len = app.history.undo_len();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo_len + 1);
    for id in &c {
        assert_eq!(
            app.project
                .timeline
                .scale_keys(FrameTarget::Clip(*id))
                .len(),
            1
        );
    }
    // a second double-click gives no second copy and no undo step
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo_len + 1);
    // the volume of the whole video is the volume of each clip, one step
    let mut mix = app.volume_mix();
    mix.video_volume = 50;
    app.dispatch(Command::SetMix(mix), now).unwrap();
    assert!(app.project.timeline.clips.iter().all(|k| k.volume == 50));
    assert_eq!(app.project.mix.video_volume, 100);
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.timeline.clips.iter().all(|k| k.volume == 100));
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.edit(), before);
    app.dispatch(Command::Redo, now).unwrap();
    // deleting clip 2 removes its copy; a clip appended later has none
    app.dispatch(Command::SelectClip(Some(c[1])), now).unwrap();
    app.dispatch(Command::Delete, now).unwrap();
    assert!(app.project.timeline.frames_of(c[1]).is_none());
    assert!(app.project.timeline.frames_of(c[0]).is_some());
    let a = app.project.media[0].id;
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    let last = *ids(&app).last().unwrap();
    assert!(app.project.timeline.frames_of(last).is_none());
}

// covers: REQ-082, REQ-083, REQ-011
#[test]
fn fade_on_a_clip_or_the_whole_video_is_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let now = Instant::now();
    let c = ids(&app);
    let before = app.project.edit();
    // (a) dropping on clip 2 = right-click > Effects > Fade IN/OUT on clip 2
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Fade,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    let tl = &app.project.timeline;
    assert!(tl.has_fade(0) && tl.has_fade(1) && !tl.start_fade() && !tl.end_fade());
    assert!(app.popups.is_open(Popup::Fades));
    assert_eq!(app.popups.target(Popup::Fades), Some(Target::Clip(c[1])));
    assert_eq!(app.popup_title(Popup::Fades), "Fade IN/OUT — b.mp4");
    assert_eq!(app.undo_text(), "Undo Fade IN/OUT");
    // applying it again changes nothing (no undo step)
    let n = app.history.undo_len();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Fade,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.edit(), before);
    // (b) double-click: the whole video
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Fade,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    assert!(app.project.timeline.start_fade() && app.project.timeline.end_fade());
    assert!(app.project.timeline.fades.is_empty());
    // only Change aspect says "whole video" (REQ-076, 2.2.0)
    assert_eq!(app.popup_title(Popup::Fades), "Fade IN/OUT");
    // each edge fade has its own popup and is removed alone
    app.dispatch(Command::OpenInstance(Instance::FadeEnd), now)
        .unwrap();
    assert_eq!(app.popup_title(Popup::Instance), "Fade to black");
    assert!(app.playhead > s(12));
    app.dispatch(
        Command::SetEdgeFade {
            edge: Edge::End,
            on: false,
        },
        now,
    )
    .unwrap();
    assert!(app.project.timeline.start_fade() && !app.project.timeline.end_fade());
    assert!(app.instance_start(Instance::FadeEnd).is_none());
    assert_eq!(app.instance_start(Instance::FadeStart), Some(Micros::ZERO));
    // (c) cutting the first clip removes its start fade with it (REQ-083)
    app.dispatch(Command::SelectClip(Some(c[0])), now).unwrap();
    app.dispatch(Command::Cut, now).unwrap();
    assert_eq!(app.project.timeline.fades_of(0).0, Micros::ZERO);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.timeline.fades_of(0).0, Micros(500_000));
    // Remove all fades removes the edge fades too, in one step
    assert!(app.is_enabled(&Command::RemoveAllFades));
    app.dispatch(Command::RemoveAllFades, now).unwrap();
    assert!(!app.project.timeline.has_any_fade());
    // the clip menu target without a clip is greyed with its reason
    let mut empty = state(FakeDialogs::default());
    let cmd = Command::ApplyEffect {
        effect: EffectKind::Fade,
        target: Target::Selected,
    };
    assert_eq!(empty.disabled_reason(&cmd), Some(effects::SELECT_A_CLIP));
    assert!(empty.dispatch(cmd, now).is_err());
    assert!(
        empty
            .dispatch(
                Command::SetEdgeFade {
                    edge: Edge::Start,
                    on: true
                },
                now
            )
            .is_err()
    );
}

// covers: REQ-082, REQ-084, REQ-020, REQ-078, REQ-008
#[test]
fn volume_on_a_clip_opens_its_popup_and_records_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let now = Instant::now();
    let c = ids(&app);
    let n = app.history.undo_len();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Volume,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    // a new Volume effect starts with one keyframe at the level of the slider (REQ-280): one
    // undo step; applying it again changes nothing more
    assert_eq!(app.history.undo_len(), n + 1);
    assert_eq!(app.project.timeline.volume_keys(c[1]).len(), 1);
    assert_eq!(app.project.timeline.volume_keys(c[1])[0].v, 100);
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Volume,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    assert_eq!(app.popups.instance, Some(Instance::ClipVolume(c[1])));
    assert_eq!(app.popup_title(Popup::Instance), "Clip volume — b.mp4");
    app.dispatch(
        Command::SetClipVolume {
            clip: c[1],
            volume: 50,
            muted: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.timeline.clips[1].volume, 50);
    assert_eq!(app.undo_text(), "Undo Clip volume");
    // the same value again: no step
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetClipVolume {
            clip: c[1],
            volume: 50,
            muted: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.timeline.clips[1].volume, 100);
    // the whole video: the Volume popup
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Volume,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    assert!(app.popups.is_open(Popup::Volume));
    assert_eq!(app.popup_title(Popup::Volume), "Volume and track");
    // the volume marker opens the clip's popup
    app.dispatch(
        Command::SetClipVolume {
            clip: c[0],
            volume: 10,
            muted: true,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::OpenInstance(Instance::ClipVolume(c[0])), now)
        .unwrap();
    assert_eq!(app.playhead, Micros::ZERO);
    // Merge needs equal volumes
    app.dispatch(Command::SelectClip(Some(c[0])), now).unwrap();
    app.dispatch(Command::Seek(s(2)), now).unwrap();
    app.dispatch(Command::Split, now).unwrap();
    let left = app.project.timeline.clips[0].id;
    let right = app.project.timeline.clips[1].id;
    assert_eq!(app.project.timeline.clips[1].volume, 10, "split keeps it");
    app.dispatch(
        Command::SetClipVolume {
            clip: right,
            volume: 90,
            muted: true,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SelectClip(Some(left)), now).unwrap();
    assert_eq!(
        app.disabled_reason(&Command::Merge),
        Some(crate::timeline::DIFFERENT_VOLUMES)
    );
    assert!(
        app.dispatch(
            Command::SetClipVolume {
                clip: ClipId(999),
                volume: 1,
                muted: false
            },
            now
        )
        .is_err()
    );
}

// covers: REQ-085, REQ-082, REQ-044
#[test]
fn subtitles_for_one_clip_cover_only_its_range() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    if !stt::available() {
        return;
    }
    speech_ready(&mut app, dir.path());
    let seen: crate::stt::tests::Seen = Default::default();
    let seen2 = Arc::clone(&seen);
    app.transcriber = Arc::new(move |_, _| {
        Ok(Box::new(crate::stt::tests::FakeTranscriber {
            seen: Arc::clone(&seen2),
            cancel_after: None,
        }) as Box<dyn stt::Transcriber>)
    });
    seed_lines(&mut app);
    let now = Instant::now();
    let c = ids(&app);
    let b = app.project.media[1].id;
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert!(app.popups.is_open(Popup::Subtitles));
    assert!(app.is_generating());
    // one creation at a time
    let again = Command::ApplyEffect {
        effect: EffectKind::Subtitles,
        target: Target::WholeVideo,
    };
    assert_eq!(
        app.disabled_reason(&again),
        Some(subs_create::BEING_CREATED)
    );
    wait_create(&mut app);
    // (c) only the clip's audio range went to recognition
    let s0 = seen.lock().unwrap().clone();
    assert_eq!(s0.len(), 1);
    assert_eq!(s0[0].1, s(1));
    let texts: Vec<(MediaId, &str)> = app
        .project
        .subtitles
        .lines
        .iter()
        .map(|l| (l.media, l.text.as_str()))
        .collect();
    // (b) the lines of clips 1 and 3 and of b outside the clip are unchanged; (a) the new
    // lines are inside [1 s, 4 s) of b
    assert!(texts.contains(&(app.project.media[0].id, "in a")));
    assert!(texts.contains(&(b, "b before the clip")));
    assert!(!texts.contains(&(b, "b inside")));
    let new: Vec<&crate::subtitles::Line> = app
        .project
        .subtitles
        .lines
        .iter()
        .filter(|l| l.text.starts_with('w'))
        .collect();
    assert_eq!(new.len(), 3);
    assert!(
        new.iter()
            .all(|l| l.media == b && l.start >= s(1) && l.end <= s(4))
    );
    assert_eq!(app.undo_text(), "Undo Create subtitles");
    app.dispatch(Command::Undo, now).unwrap();
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .any(|l| l.text == "b inside")
    );
    // REQ-044: a hand-edited line in the clip's range asks first; outside it does not
    let inside = app
        .project
        .subtitles
        .lines
        .iter()
        .find(|l| l.text == "b inside")
        .unwrap()
        .id;
    app.dispatch(Command::SetSubtitleText(inside, "mine".into()), now)
        .unwrap();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(c[0]),
        },
        now,
    )
    .unwrap();
    assert!(app.modal.is_none(), "clip 1 has no edited line");
    wait_create(&mut app);
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.modal, Some(Modal::ConfirmRegenerate));
    app.dispatch(Command::CloseModal, now).unwrap();
    assert!(app.pending_create.is_none());
    assert!(app.project.subtitles.lines.iter().any(|l| l.text == "mine"));
}

/// A runner whose subtitle track extraction writes `srt`.
pub(super) fn runner_with_track(srt: &'static str) -> FakeRunner {
    let mut r = runner();
    r.responders.insert(
        0,
        Box::new(move |_, a: &[String]| {
            if !a.iter().any(|x| x.starts_with("0:s:")) {
                return None;
            }
            let out = a.last()?.strip_prefix("file:")?.to_string();
            std::fs::write(out, srt).ok()?;
            ok("")
        }),
    );
    r
}

pub(super) const TRACK: &str = "1\n00:00:01,200 --> 00:00:01,900\n{\\an8}<i>w0</i>\n\n\
                     2\n00:00:02,000 --> 00:00:03,500\nw1\nw2\n\n\
                     3\n00:00:04,500 --> 00:00:05,000\nafter the clip\n";

pub(super) fn with_tracks(app: &mut AppState, m: usize, tracks: &[(&str, &str)]) {
    app.project.media[m].format.subtitle_tracks = Some(
        tracks
            .iter()
            .enumerate()
            .map(|(i, (l, t))| SubtitleTrack {
                ordinal: i as u8,
                codec: TextCodec::Srt,
                language: Some(l.to_string()),
                title: (!t.is_empty()).then(|| t.to_string()),
            })
            .collect(),
    );
}

// covers: REQ-087, REQ-095, REQ-015, REQ-016
#[test]
fn the_video_s_own_subtitles_can_be_used_or_recognised_or_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state_with(FakeDialogs::default(), runner_with_track(TRACK));
    three(&mut app, dir.path());
    if !stt::available() {
        return;
    }
    speech_ready(&mut app, dir.path());
    with_tracks(&mut app, 1, &[("ita", "Italiano"), ("eng", "")]);
    let now = Instant::now();
    let c = ids(&app);
    let apply = Command::ApplyEffect {
        effect: EffectKind::Subtitles,
        target: Target::Clip(c[1]),
    };
    // (d) Cancel: nothing changes, no undo step
    let before = app.project.edit();
    let n = app.history.undo_len();
    app.dispatch(apply.clone(), now).unwrap();
    assert_eq!(
        app.modal,
        Some(Modal::SubtitleSource {
            labels: vec!["Italian — Italiano".into(), "English".into()],
            can_match: true
        })
    );
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Cancel), now)
        .unwrap();
    assert!(app.modal.is_none() && app.pending_create.is_none() && !app.is_generating());
    assert_eq!((app.project.edit(), app.history.undo_len()), (before, n));
    // (a, b) Use: exactly the cues' text in the clip's range, tags removed, lines joined
    app.dispatch(apply.clone(), now).unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    assert!(app.stt_matching);
    assert!(
        app.background_jobs()
            .iter()
            .any(|j| j.label == "Matching subtitles to the speech")
    );
    wait_create(&mut app);
    let texts: Vec<&str> = app
        .project
        .subtitles
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(texts, ["w0", "w1 w2"]);
    let l = &app.project.subtitles.lines[1];
    assert_eq!(
        (l.start, l.end),
        (s(2), Micros(3_500_000)),
        "cue timing kept"
    );
    assert!(!l.edited);
    // the fake recogniser heard w0, w1, w2 at 1, 2, 3 s: the words got those times
    assert_eq!(l.words.len(), 2);
    assert_eq!(l.words[0].start, s(2));
    // the track's language becomes the project's
    assert_eq!(app.project.subtitles.language.as_deref(), Some("it"));
    // (c) Recognise: the speech, as in 1.1
    app.dispatch(apply.clone(), now).unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Recognise), now)
        .unwrap();
    assert!(!app.stt_matching);
    wait_create(&mut app);
    let texts: Vec<&str> = app
        .project
        .subtitles
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(texts, ["w0", "w1", "w2"]);
    // the answer is only taken while the dialog is open
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    assert!(!app.is_generating());
    // the label picks the same track in other files, else the same language, else the
    // first one
    let tracks = app.project.media[1].format.subtitle_tracks.clone().unwrap();
    assert_eq!(pick_track(&tracks, "English").unwrap().ordinal, 1);
    assert_eq!(pick_track(&tracks, "Italian — other").unwrap().ordinal, 0);
    assert_eq!(pick_track(&tracks, "Klingon").unwrap().ordinal, 0);
    assert!(pick_track(&[], "English").is_none());
}

// covers: REQ-095, REQ-054, REQ-087
#[test]
fn without_matching_the_track_text_is_kept_with_words_spread() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state_with(FakeDialogs::default(), runner_with_track(TRACK));
    three(&mut app, dir.path());
    with_tracks(&mut app, 1, &[("eng", "")]);
    let fetches = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let loads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let l2 = Arc::clone(&loads);
    app.transcriber = Arc::new(move |_, _| {
        l2.fetch_add(1, Ordering::SeqCst);
        Err(stt::SttError::NotAvailable)
    });
    // (e) a processor the speech engine cannot use: no download, no recognition
    let mut old = Machine::new(
        8,
        4,
        16 * GIB,
        "linux",
        "x86_64",
        crate::machine::CpuFeatures::ALL,
    );
    old.cpu.avx2 = false;
    app.machine = old;
    assert!(!app.machine.speech_supported());
    let now = Instant::now();
    let c = ids(&app);
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    let Some(Modal::SubtitleSource { can_match, .. }) = app.modal.clone() else {
        panic!("{:?}", app.modal)
    };
    assert!(!can_match);
    assert!(crate::ui::modals::source_use_label(false).contains("cannot match words"));
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    wait_create(&mut app);
    assert_eq!(app.notice, None);
    let texts: Vec<&str> = app
        .project
        .subtitles
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(texts, ["w0", "w1 w2"]);
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.words.is_empty())
    );
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    assert_eq!(fetches.load(Ordering::SeqCst), 0);
    // Recognise on that processor: the old-processor notice, nothing else
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Recognise), now)
        .unwrap();
    assert_eq!(app.notice.as_deref(), Some(TOO_OLD_FOR_SUBTITLES));
}

// covers: REQ-095, REQ-052, REQ-014
#[test]
fn a_refused_download_can_still_use_the_track_and_cancel_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state_with(FakeDialogs::default(), runner_with_track(TRACK));
    three(&mut app, dir.path());
    if !stt::available() {
        return;
    }
    with_fake_speech(&mut app, dir.path());
    with_tracks(&mut app, 1, &[("eng", "")]);
    let now = Instant::now();
    let c = ids(&app);
    let apply = Command::ApplyEffect {
        effect: EffectKind::Subtitles,
        target: Target::Clip(c[1]),
    };
    // (iii) the consent dialog offers "Use without matching words"
    app.dispatch(apply.clone(), now).unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    assert_eq!(app.modal, Some(Modal::ModelConsent(SubtitleQuality::Fast)));
    assert!(app.consent_offers_spread());
    let labels: Vec<ConsentChoice> =
        crate::ui::modals::consent_choices(&app, SubtitleQuality::Fast)
            .into_iter()
            .map(|c| c.1)
            .collect();
    assert!(labels.contains(&ConsentChoice::UseWithoutMatching));
    // Cancel: nothing changes, no undo step
    let (before, n) = (app.project.edit(), app.history.undo_len());
    app.dispatch(Command::ModelConsent(ConsentChoice::Cancel), now)
        .unwrap();
    assert!(app.pending_create.is_none());
    assert_eq!(
        (app.project.edit(), app.history.undo_len()),
        (before.clone(), n)
    );
    // (f) cancel during the download: unchanged
    app.dispatch(apply.clone(), now).unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    app.fetcher = Some(Arc::new(SlowFetcher));
    app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
        .unwrap();
    assert!(app.is_generating());
    app.dispatch(Command::CancelSubtitles, now).unwrap();
    settle(&mut app);
    app.notice = None;
    assert!(!app.is_generating() && app.pending_create.is_none());
    assert_eq!((app.project.edit(), app.history.undo_len()), (before, n));
    // Use without matching words: the track's text, no download
    app.dispatch(apply.clone(), now).unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    app.dispatch(
        Command::ModelConsent(ConsentChoice::UseWithoutMatching),
        now,
    )
    .unwrap();
    wait_create(&mut app);
    assert_eq!(app.project.subtitles.lines.len(), 2);
    assert!(
        !app.model_store
            .as_ref()
            .unwrap()
            .is_installed(SubtitleQuality::Fast)
    );
    // (f) cancel during matching: unchanged
    speech_ready(&mut app, dir.path());
    app.transcriber = Arc::new(|_, _| {
        Ok(Box::new(crate::stt::tests::FakeTranscriber {
            cancel_after: Some(1),
            ..Default::default()
        }) as Box<dyn stt::Transcriber>)
    });
    let (before, n) = (app.project.edit(), app.history.undo_len());
    app.dispatch(apply, now).unwrap();
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
        .unwrap();
    wait_create(&mut app);
    assert_eq!(app.status, "Subtitle generation cancelled");
    assert_eq!((app.project.edit(), app.history.undo_len()), (before, n));
}

/// A download that waits for its cancel flag.
struct SlowFetcher;

impl stt::ModelFetcher for SlowFetcher {
    fn fetch(
        &self,
        _url: &str,
        _max: u64,
        _out: &mut dyn std::io::Write,
        _progress: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), stt::SttError> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !cancel.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        Err(stt::SttError::Cancelled)
    }
}

// covers: REQ-086, REQ-087
#[test]
fn media_of_older_projects_are_checked_for_subtitles_without_becoming_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let probe = r#"{"streams":[
        {"codec_type":"video","codec_name":"h264","width":1280,"height":720,"avg_frame_rate":"30/1"},
        {"codec_type":"audio","codec_name":"aac"},
        {"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"ita"}}],
        "format":{"format_name":"matroska","duration":"5.0"}}"#;
    let mut r = runner_with_track(TRACK);
    r.responders.insert(
        0,
        Box::new(move |p: &Path, a: &[String]| {
            (p.ends_with("ffprobe") && a.last()?.ends_with("a.mp4")).then(|| ok(probe))?
        }),
    );
    let mut app = state_with(FakeDialogs::default(), r);
    // the 1.1 fixture: its media were analysed before 1.2.0
    let p = crate::render::golden_tests::open_fixture(
        dir.path(),
        crate::render::golden_tests::V2_FIXTURE,
    );
    let file = dir.path().join("fixture.vlproj");
    assert!(p.media.iter().all(|m| m.format.subtitle_tracks.is_none()));
    let now = Instant::now();
    app.dispatch(Command::OpenProject(file), now).unwrap();
    assert!(!app.tracks_checking.is_empty());
    assert!(app.busy());
    // a creation asked meanwhile waits for the check
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Subtitles,
            target: Target::Clip(app.project.timeline.clips[0].id),
        },
        now,
    )
    .unwrap();
    assert!(app.pending_create.is_some() || app.modal.is_some());
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.track_check.is_some() && Instant::now() < deadline {
        app.tick(Instant::now());
    }
    let a = app
        .project
        .media
        .iter()
        .find(|m| m.name == "a.mp4")
        .unwrap();
    assert_eq!(a.format.subtitle_tracks.as_ref().unwrap().len(), 1);
    assert_eq!(
        crate::ui::media::subtitles_note(a).as_deref(),
        Some("Contains subtitles (Italian)")
    );
    let b = app
        .project
        .media
        .iter()
        .find(|m| m.name == "b.mp4")
        .unwrap();
    assert_eq!(
        b.format.subtitle_tracks,
        Some(vec![]),
        "unreadable: no track"
    );
    assert!(crate::ui::media::subtitles_note(b).is_none());
    assert!(!app.dirty, "the check does not make the project unsaved");
    // then the waiting creation asks about the video's subtitles
    assert!(matches!(app.modal, Some(Modal::SubtitleSource { .. })));
    app.dispatch(Command::ChooseSubtitleSource(SourceChoice::Cancel), now)
        .unwrap();
}

// covers: REQ-090, REQ-091, REQ-092, REQ-089
#[test]
fn lines_are_selected_and_moved_together() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let a = app.project.media[0].id;
    let line = |start: i64, end: i64, text: &str| NewLine {
        media: a,
        start: Micros(start * 1000),
        end: Micros(end * 1000),
        text: text.into(),
        words: vec![],
    };
    app.project.subtitles.replace_all(
        vec![line(2000, 2500, "two"), line(3000, 4500, "three to six")],
        None,
    );
    let now = Instant::now();
    let shown = app.project.subtitles.project(&app.project.timeline);
    // timeline starts: 2 s, 3 s (clip 1) and 10 s, 11 s (clip 3, a copy of a at 8 s)
    let starts: Vec<Micros> = shown.iter().map(|l| l.start).collect();
    assert_eq!(starts, [s(2), s(3), s(10), s(11)]);
    let ids: Vec<LineId> = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    // REQ-091: Shift+drag 2.2–9.5 s selects the line starting inside (3 s); the 2–2.5 s
    // line only ends inside and is not selected
    app.dispatch(
        Command::SelectRange {
            from: Micros(9_500_000),
            to: Micros(2_200_000),
            add: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.selection.iter().copied().collect::<Vec<_>>(), [ids[1]]);
    assert_eq!(
        app.playhead,
        s(3),
        "the playhead goes to the first selected line"
    );
    app.dispatch(
        Command::SelectRange {
            from: s(1),
            to: Micros(2_100_000),
            add: true,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.selection.len(), 2);
    // REQ-090: Ctrl/Cmd+click toggles, a plain click selects only that line
    app.dispatch(Command::ClearSelection, now).unwrap();
    app.dispatch(Command::ToggleLine(ids[0]), now).unwrap();
    app.dispatch(Command::ToggleLine(ids[1]), now).unwrap();
    assert_eq!(app.selection.len(), 2);
    app.dispatch(Command::ToggleLine(ids[1]), now).unwrap();
    assert_eq!(app.selection.len(), 1);
    app.dispatch(Command::ToggleLine(ids[1]), now).unwrap();
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(Command::OpenInstance(Instance::Line(ids[0], clip)), now)
        .unwrap();
    assert_eq!(app.selection.iter().copied().collect::<Vec<_>>(), [ids[0]]);
    assert!(app.dispatch(Command::ToggleLine(LineId(99)), now).is_err());
    // REQ-092: a group drag gives every selected line the dragged line's position
    app.dispatch(Command::ToggleLine(ids[1]), now).unwrap();
    assert_eq!(app.drag_group(ids[0]), ids);
    assert_eq!(app.drag_group(LineId(77)), [LineId(77)]);
    let p = Pos { x: 5000, y: 2000 };
    app.dispatch(Command::DragLines(Some((ids.clone(), p))), now)
        .unwrap();
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none())
    );
    assert!(
        app.shown_project()
            .subtitles
            .lines
            .iter()
            .all(|l| l.position == Some(p)),
        "shown at once, not saved"
    );
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetLinePositions {
            ids: app.drag_group(ids[0]),
            pos: Some(p),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    assert!(app.drag_pos.is_none());
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position == Some(p))
    );
    assert_eq!(app.undo_text(), "Undo Move subtitle");
    app.dispatch(Command::Undo, now).unwrap();
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none())
    );
    app.dispatch(Command::Redo, now).unwrap();
    // REQ-089: Reset position on a selection resets all of them
    assert!(app.is_enabled(&Command::ResetPosition));
    app.dispatch(Command::ResetPosition, now).unwrap();
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none())
    );
    assert!(
        app.dispatch(
            Command::SetLinePositions {
                ids: vec![ids[0]],
                pos: Some(Pos { x: 10_001, y: 0 })
            },
            now
        )
        .is_err()
    );
    assert!(
        app.dispatch(
            Command::SetLinePositions {
                ids: vec![],
                pos: None
            },
            now
        )
        .is_err()
    );
    // deleted lines leave the selection
    app.dispatch(Command::DeleteSubtitle(ids[1]), now).unwrap();
    assert!(!app.selection.contains(&ids[1]));
    app.dispatch(Command::ClearSelection, now).unwrap();
    app.dispatch(Command::SelectLine(None), now).unwrap();
    assert_eq!(
        app.disabled_reason(&Command::ResetPosition),
        Some(effects::SELECT_A_LINE)
    );
}

// covers: REQ-088
#[test]
fn subtitles_are_exported_as_srt_with_timeline_times() {
    let dir = tempfile::tempdir().unwrap();
    let srt = dir.path().join("subs");
    let dialogs = FakeDialogs {
        srt: Some(srt.clone()),
        ..Default::default()
    };
    let name = Arc::clone(&dialogs.srt_name);
    let mut app = state(dialogs);
    three(&mut app, dir.path());
    let now = Instant::now();
    // (d) nothing to export
    assert_eq!(
        app.disabled_reason(&Command::PickExportSrt),
        Some(effects::NO_SUBTITLES)
    );
    assert!(app.dispatch(Command::ExportSrt(srt.clone()), now).is_err());
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    app.project.subtitles.replace_all(
        vec![
            NewLine {
                media: a,
                start: s(1),
                end: s(2),
                text: "in a".into(),
                words: vec![],
            },
            NewLine {
                media: b,
                start: Micros(500_000),
                end: Micros(1_500_000),
                text: "b, partly trimmed".into(),
                words: vec![],
            },
            NewLine {
                media: b,
                start: Micros(4_500_000),
                end: s(5),
                text: "b, trimmed away".into(),
                words: vec![],
            },
        ],
        None,
    );
    app.project_path = Some(dir.path().join("Holiday.vlproj"));
    app.dispatch(Command::PickExportSrt, now).unwrap();
    assert_eq!(*name.lock().unwrap(), "Holiday.srt");
    let text = std::fs::read_to_string(srt.with_extension("srt")).unwrap();
    // (a) the line of a is shown twice (clip 1 and its copy, clip 3); (b) the trimmed
    // line has the cut times: b starts at 1 s in the source = 5 s on the timeline
    assert_eq!(
        text,
        "1\n00:00:01,000 --> 00:00:02,000\nin a\n\n\
         2\n00:00:05,000 --> 00:00:05,500\nb, partly trimmed\n\n\
         3\n00:00:09,000 --> 00:00:10,000\nin a\n\n"
    );
    // a write that fails leaves no file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let ro = dir.path().join("ro");
        std::fs::create_dir(&ro).unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o500)).unwrap();
        let target = ro.join("x.srt");
        let r = app.dispatch(Command::ExportSrt(target.clone()), now);
        let left: Vec<_> = std::fs::read_dir(&ro).unwrap().collect();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o700)).unwrap();
        if r.is_err() {
            assert!(left.is_empty(), "no partial file");
        }
    }
    // cancelled dialog: nothing
    let mut other = state(FakeDialogs::default());
    other.project = app.project.clone();
    other.dispatch(Command::PickExportSrt, now).unwrap();
    assert_eq!(other.srt_name(), "Untitled.srt");
}

// covers: REQ-114
#[test]
fn clean_audio_is_applied_to_a_clip_or_the_whole_video_in_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let now = Instant::now();
    let c = ids(&app);
    let on = |app: &AppState| -> Vec<bool> {
        app.project
            .timeline
            .clips
            .iter()
            .map(|c| c.clean_audio)
            .collect()
    };
    assert_eq!(on(&app), [false, false, false]);
    // dropped on a clip: that clip, and its popup with Remove opens
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::CleanAudio,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert_eq!(on(&app), [false, true, false]);
    assert_eq!(app.popups.instance, Some(Instance::CleanAudio(c[1])));
    assert!(app.popups.is_open(Popup::Instance));
    assert_eq!(app.popup_title(Popup::Instance), "Clean audio — b.mp4");
    assert_eq!(app.undo_text(), "Undo Clean audio");
    // applying it again changes nothing and adds no undo step
    let n = app.history.undo_len();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::CleanAudio,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    // whole video: every clip, one step, the popup of the whole video
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::CleanAudio,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    assert_eq!(on(&app), [true, true, true]);
    assert!(app.popups.is_open(Popup::CleanAudio));
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(on(&app), [false, true, false], "one undo");
    // Remove (the popup's button): one undo step, nothing to do when it is off
    app.dispatch(
        Command::SetCleanAudio {
            target: Target::Clip(c[1]),
            on: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(on(&app), [false, false, false]);
    assert_eq!(app.undo_text(), "Undo Remove clean audio");
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetCleanAudio {
            target: Target::WholeVideo,
            on: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    // the selected clip
    app.dispatch(Command::SelectClip(Some(c[2])), now).unwrap();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::CleanAudio,
            target: Target::Selected,
        },
        now,
    )
    .unwrap();
    assert_eq!(on(&app), [false, false, true]);
    // it is saved with the project and a re-encode is needed
    let file = dir.path().join("p.vlproj");
    crate::project::save(&app.project, &file).unwrap();
    assert_eq!(
        crate::project::load(&file).unwrap().0.timeline,
        app.project.timeline
    );
    // availability: no clip, no effect
    let empty = state(FakeDialogs::default());
    let cmd = Command::SetCleanAudio {
        target: Target::WholeVideo,
        on: true,
    };
    assert!(!empty.is_enabled(&cmd));
    assert!(app.is_enabled(&cmd));
    assert!(!empty.is_enabled(&Command::SetCleanAudio {
        target: Target::Selected,
        on: true
    }));
    assert!(
        empty
            .disabled_reason(&Command::SetCleanAudio {
                target: Target::Selected,
                on: true
            })
            .is_some()
    );
}

// covers: REQ-274, REQ-275
#[test]
fn the_fade_length_and_side_are_one_undo_step_each_and_reach_the_render() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let now = Instant::now();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    let (l, r) = (
        app.project.timeline.clips[0].id,
        app.project.timeline.clips[1].id,
    );
    let seg_fades = |app: &AppState| {
        let segs = render::segments(&app.project, Micros::ZERO, render::Source::Export).unwrap();
        (segs[0].fade_out, segs[1].fade_in)
    };
    assert_eq!(seg_fades(&app), (Micros(500_000), Micros(500_000)));
    let undo = app.history.undo_len();
    app.dispatch(Command::SetFadeLength(Micros(1_500_000)), now)
        .unwrap();
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(seg_fades(&app), (Micros(1_500_000), Micros(1_500_000)));
    assert!(
        app.dispatch(Command::SetFadeLength(Micros(5)), now)
            .is_err()
    );
    app.dispatch(
        Command::SetFadeSide {
            left: l,
            right: r,
            side: crate::timeline::FadeSide::OutOnly,
        },
        now,
    )
    .unwrap();
    assert_eq!(seg_fades(&app), (Micros(1_500_000), Micros::ZERO));
    assert_eq!(app.history.undo_len(), undo + 2);
    app.dispatch(Command::Undo, now).unwrap();
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(seg_fades(&app), (Micros(500_000), Micros(500_000)));
}

// covers: REQ-276, REQ-277
#[test]
fn a_music_clips_sound_settings_are_one_undo_step_and_fades_stay_inside_the_clip() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let now = Instant::now();
    app.dispatch(Command::PlaceMusic(Some(app.project.media[2].id)), now)
        .unwrap();
    let id = app.project.music.clips[0].id;
    let undo = app.history.undo_len();
    app.dispatch(
        Command::SetMusicEffects {
            clip: id,
            volume: 50,
            muted: false,
            fade_in: Micros(1_000_000),
            fade_out: Micros(900_000_000),
            clean_audio: true,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo + 1);
    let c = app.project.music.get(id).unwrap();
    assert_eq!((c.volume, c.clean_audio), (50, true));
    assert_eq!(
        c.fade_out,
        Micros(c.len().0 / 2),
        "a fade is at most half the clip"
    );
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.music.get(id).unwrap().volume, 100);
    assert!(
        app.dispatch(
            Command::SetMusicEffects {
                clip: crate::music::MusicClipId(999),
                volume: 1,
                muted: false,
                fade_in: Micros::ZERO,
                fade_out: Micros::ZERO,
                clean_audio: false
            },
            now
        )
        .is_err()
    );
}
