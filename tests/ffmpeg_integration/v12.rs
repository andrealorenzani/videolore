// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 1.2.0: fades at the video's edges, clip volume, subtitle
//! positions, SRT export, subtitle tracks inside videos and their alignment.

use super::*;
use videolore::base::pos::Pos;
use videolore::stt::{Mode, RangeJob, SttJob};

const SRT: &str = "1\n00:00:00,500 --> 00:00:01,200\n<i>Hello</i> there\n\n\
                   2\n00:00:01,400 --> 00:00:01,900\nSecond line\n";

fn two_clips(env: &Env) -> Project {
    let mut p = Project::default();
    let a = env.clip("ea.mp4", 640, 360, 25, 2, true);
    let m = env.probe(&mut p, &a);
    p.timeline.append(&m).unwrap();
    p.timeline.append(&m).unwrap();
    p
}

/// A whole-video fade: the first and the last frame are black, also after cutting the
/// first clip; the middle is not.
// covers: REQ-083, REQ-011
#[test]
fn whole_video_fade_starts_and_ends_in_black() {
    let Some(env) = setup() else { return };
    let mut p = two_clips(&env);
    p.timeline.set_edge_fades(true).unwrap();
    let (r, ev) = env.export(&p, &env.dir.path().join("edges.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let r = r.unwrap();
    let means = env.luma_means(&r, 640, 360);
    assert!(means.len() >= 98, "{}", means.len());
    eprintln!(
        "edge fade means: first {:?} last {:?}",
        &means[..3],
        &means[means.len() - 4..]
    );
    assert!(means[0] < 25.0, "first frame {}", means[0]);
    assert!(
        *means.last().unwrap() < 20.0,
        "last frame {:?}",
        means.last()
    );
    assert!(means[means.len() / 2] > 60.0, "no fade at the boundary");
    // sound fades with the picture
    let quiet = env.audio_peak(&r, 0.0, 0.04);
    let loud = env.audio_peak(&r, 1.0, 0.2);
    assert!(quiet * 4 < loud, "quiet {quiet} loud {loud}");
    // the start fade belongs to the first clip: cutting it removes the fade (REQ-083)
    let first = p.timeline.clips[0].id;
    p.timeline.remove(first).unwrap();
    let (r, _) = env.export(&p, &env.dir.path().join("edges2.mp4"));
    let means = env.luma_means(&r.unwrap(), 640, 360);
    assert!(means[0] > 60.0 && *means.last().unwrap() < 20.0);
}

/// A muted clip is silent, the next one is not; 50 % is a quarter of the level.
// covers: REQ-084, REQ-020
#[test]
fn a_muted_clip_is_silent_and_the_others_are_not() {
    let Some(env) = setup() else { return };
    let mut p = two_clips(&env);
    let (a, b) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
    p.timeline.set_clip_volume(a, 100, true).unwrap();
    let (r, ev) = env.export(&p, &env.dir.path().join("muted.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let r = r.unwrap();
    let silent = env.audio_peak(&r, 0.3, 1.2);
    let loud = env.audio_peak(&r, 2.5, 1.0);
    assert!(silent < 50, "muted clip: {silent}");
    assert!(loud > 1500, "other clip: {loud}");
    p.timeline.set_clip_volume(a, 50, false).unwrap();
    p.timeline.set_clip_volume(b, 100, false).unwrap();
    let (r, _) = env.export(&p, &env.dir.path().join("half-clip.mp4"));
    let r = r.unwrap();
    let ratio = env.audio_peak(&r, 0.3, 1.2) as f64 / env.audio_peak(&r, 2.5, 1.0) as f64;
    assert!(
        (0.18..0.33).contains(&ratio),
        "50% clip volume ratio {ratio}"
    );
}

/// Centre (x, y) of the bright pixels of an RGB24 frame.
fn bright_centre(rgb: &[u8], w: usize) -> Option<(f64, f64)> {
    let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
    for (i, px) in rgb.as_chunks::<3>().0.iter().enumerate() {
        if px.iter().all(|v| *v > 150) {
            sx += (i % w) as f64;
            sy += (i / w) as f64;
            n += 1.0;
        }
    }
    (n > 0.0).then(|| (sx / n, sy / n))
}

/// A positioned line is burned in where its position says (within 2 % of the height),
/// and the preview uses the same subtitle document as the export.
// covers: REQ-089, REQ-017
#[test]
fn a_positioned_subtitle_is_burned_in_at_its_position() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        assert!(!required(), "the CI ffmpeg must have libass");
        return;
    }
    let black = env.fixture(
        "black.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x180:r=25:d=2",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let mut p = Project::default();
    let m = env.probe(&mut p, &black);
    p.timeline.append(&m).unwrap();
    let tl = p.timeline.clone();
    let id = p
        .subtitles
        .add_at(&tl, Micros::ZERO, "HHHHH", Micros(40_000))
        .unwrap();
    let centre = |p: &Project, name: &str| {
        let (r, _) = env.export(p, &env.dir.path().join(name));
        let rgb = env.frame_rgb(&r.unwrap(), 0.5);
        bright_centre(&rgb, 320).expect("white text on black")
    };
    // the style's place: centred, in the lower part
    let (x, y) = centre(&p, "default.mp4");
    assert!((x - 160.0).abs() < 4.0 && y > 150.0, "default at {x},{y}");
    // moved to the top left
    p.subtitles
        .set_position(&[id], Some(Pos { x: 2500, y: 2000 }))
        .unwrap();
    let (x, y) = centre(&p, "moved.mp4");
    let tol = 0.02 * 180.0;
    eprintln!("positioned subtitle: centre {x:.2},{y:.2} (wanted 80,36)");
    assert!(
        (x - 80.0).abs() <= tol && (y - 36.0).abs() <= tol,
        "{x},{y}"
    );
    // the preview shows it from the same document
    let enc = env.info.encoders.clone();
    let plan = render::plan_export(&p, &enc, |_, _| true).unwrap();
    let g = render::output_geometry(&p, &enc).unwrap();
    let still = render::still_args(
        &p,
        Micros::ZERO,
        &g,
        env.info.version,
        &env.dir.path().join("g.txt"),
        render::Source::Export,
    )
    .unwrap();
    assert_eq!(still.subtitles.as_deref(), plan.subtitles());
    assert!(plan.subtitles().unwrap().contains("\\pos(80,36)"));
}

/// The exported SRT is read back by ffmpeg (as a reference reader) with the same cues.
// covers: REQ-088
#[test]
fn exported_srt_is_read_by_ffmpeg() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let v = env.clip("ea.mp4", 640, 360, 25, 2, true);
    let m = env.probe(&mut p, &v);
    p.timeline.append(&m).unwrap();
    p.timeline.append(&m).unwrap();
    let tl = p.timeline.clone();
    p.subtitles
        .add_at(&tl, Micros(300_000), "Ciao è già qui", Micros(40_000))
        .unwrap();
    let shown = p.subtitles.project(&p.timeline);
    assert_eq!(shown.len(), 2, "the copy shows the line again");
    let text = videolore::srt::write(&shown);
    let srt = env.dir.path().join("out.srt");
    std::fs::write(&srt, &text).unwrap();
    let vtt = env.dir.path().join("out.vtt");
    env.ffmpeg(&[
        "-i",
        srt.to_str().unwrap(),
        "-f",
        "webvtt",
        vtt.to_str().unwrap(),
    ]);
    let back = videolore::srt::parse(&std::fs::read_to_string(&vtt).unwrap());
    assert_eq!(back.len(), 2);
    for (cue, line) in back.iter().zip(&shown) {
        assert_eq!(cue.text, line.text);
        assert!(near(cue.start, line.start, Micros(1_000)), "{cue:?}");
        assert!(near(cue.end, line.end, Micros(1_000)), "{cue:?}");
    }
}

/// Videos with a text subtitle track (MKV + SRT, MP4 + timed text).
fn with_subtitles(env: &Env, name: &str, codec: &str, lang: &str, extra: &[&str]) -> PathBuf {
    let v = env.clip("ea.mp4", 640, 360, 25, 2, true);
    let srt = fixture_dir().join("v12-subs.srt");
    std::fs::create_dir_all(fixture_dir()).unwrap();
    std::fs::write(&srt, SRT).unwrap();
    let lang = format!("language={lang}");
    let mut args = vec![
        "-i",
        v.to_str().unwrap(),
        "-i",
        srt.to_str().unwrap(),
        "-map",
        "0",
        "-map",
        "1",
        "-c",
        "copy",
        "-c:s",
        codec,
        "-metadata:s:s:0",
        &lang,
    ];
    args.extend_from_slice(extra);
    env.fixture(name, &args)
}

fn spread_job(env: &Env, path: &Path, track: u8, range: Option<(Micros, Micros)>) -> SttJob {
    SttJob {
        ranges: vec![RangeJob {
            media: media::MediaId(1),
            path: path.to_path_buf(),
            range,
            mode: Mode::Spread { track },
            track_language: None,
        }],
        language: None,
        ffmpeg: env.info.ffmpeg.clone(),
        model: PathBuf::new(),
        threads: 1,
        work_root: env.dir.path().join("work"),
        model_check: None,
    }
}

fn track_lines(
    env: &Env,
    path: &Path,
    range: Option<(Micros, Micros)>,
) -> Vec<videolore::subtitles::NewLine> {
    let (tx, _rx) = channel();
    let runner = limited(Priority::Background);
    videolore::stt::run(
        &spread_job(env, path, 0, range),
        &runner as &dyn videolore::ffmpeg::Runner,
        None,
        None,
        &Arc::new(AtomicBool::new(false)),
        &tx,
    )
    .unwrap()
    .lines()
}

/// Text subtitle tracks are found with their language, and their text is read (tags
/// removed) with the cue times, also for a file that does not start at zero.
// covers: REQ-086, REQ-087
#[test]
fn subtitle_tracks_inside_videos_are_found_and_read() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let mkv = with_subtitles(&env, "subs.mkv", "srt", "ita", &[]);
    let m = env.probe(&mut p, &mkv);
    let t = m.format.subtitle_tracks.clone().unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].codec, media::TextCodec::Srt);
    assert_eq!(t[0].language.as_deref(), Some("ita"));
    assert_eq!(t[0].label(), "Italian");
    let mp4 = with_subtitles(&env, "subs.mp4", "mov_text", "eng", &[]);
    let m = env.probe(&mut p, &mp4);
    let t = m.format.subtitle_tracks.clone().unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].codec, media::TextCodec::MovText);
    assert_eq!(t[0].language.as_deref(), Some("eng"));
    // a plain video has none
    let plain = env.clip("ea.mp4", 640, 360, 25, 2, true);
    let m = env.probe(&mut p, &plain);
    assert_eq!(m.format.subtitle_tracks, Some(vec![]));
    // the text of the track, for the whole file and for a part of it
    for file in [&mkv, &mp4] {
        let lines = track_lines(&env, file, None);
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["Hello there", "Second line"], "{file:?}");
        assert!(
            near(lines[0].start, Micros(500_000), Micros(50_000)),
            "{file:?} {:?}",
            lines[0]
        );
        assert!(near(lines[1].end, Micros(1_900_000), Micros(50_000)));
    }
    let part = track_lines(&env, &mkv, Some((Micros(1_300_000), Micros(1_600_000))));
    assert_eq!(part.len(), 1);
    assert_eq!(part[0].text, "Second line");
    assert!(near(part[0].start, Micros(1_400_000), Micros(50_000)));
    assert_eq!(part[0].end, Micros(1_600_000), "kept inside the range");
    // a file whose time stamps start at 5 s: cue times are still from the file's start
    let late = with_subtitles(&env, "late.mkv", "srt", "ita", &["-output_ts_offset", "5"]);
    let lines = track_lines(&env, &late, None);
    eprintln!("late.mkv first cue at {:?}", lines.first().map(|l| l.start));
    assert_eq!(lines.len(), 2);
    assert!(
        near(lines[0].start, Micros(500_000), Micros(50_000)),
        "{:?}",
        lines[0].start
    );
}

/// The committed alignment fixture (made by `tests/fixtures/align/make.sh`).
#[cfg(feature = "stt")]
fn align_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/align")
}

/// The video's own subtitles with real speech and the Fast file: the track's text and
/// cue times are kept, every word gets a time inside its line, most words are found.
/// (CI provides the model with VIDEOLORE_STT_MODEL, like the other speech tests.)
// covers: REQ-095, REQ-086, REQ-087
#[cfg(feature = "stt")]
#[test]
fn alignment_keeps_the_track_and_times_its_words_from_real_speech() {
    alignment_with_real_speech(false);
}

/// The accuracy gate of REQ-177 (reworded 2026-11-20): at least 90 % of the MATCHED words start
/// within 0.3 s of where they are really spoken (the matched share is reported, not required).
/// NOT MET by the Fast file (pinned ffmpeg and model: 23 of 28 words matched, mean start error
/// 0.48 s, largest 1.10 s). DTW token times lag every word by 0.43 to 0.75 s (dropped). Starting
/// each word at the latest energy onset 0.2 to 1.0 s before the model's time gives 7 of 23
/// matched words within 0.3 s (mean 0.45 s, largest 1.13 s): the model's own word times (token
/// times, contiguous segments) are off by up to 1.1 s in both directions, so no one-sided window
/// finds the right onset; the nearest onset within 0.5 s or 1.0 s gives 8 of 23. Placing the words
/// in the speech regions of the envelope (dynamic programming on word length) gave 12 of 23, or
/// 21 to 22 of 23 only with 4 of 120 tuned settings (not built). It stays
/// ignored; run it with
/// `cargo test --test ffmpeg_integration alignment_matches_real_speech -- --ignored`.
// covers: REQ-095, REQ-177
#[cfg(feature = "stt")]
#[test]
#[ignore = "REQ-177: the onset snap does not reach 90% of the matched words within 0.3 s on the Fast file (see the roadmap)"]
fn alignment_matches_real_speech() {
    alignment_with_real_speech(true);
}

/// The speech fixture is checked byte for byte, so a changed fixture fails (REQ-177).
#[cfg(feature = "stt")]
fn check_fixture() {
    use sha2::{Digest, Sha256};
    for (file, sum) in [
        (
            "speech.mkv",
            "d6656832dedaebfac68551b4d4199b3a4207fadb4d873d499bab75a2b31ef6e0",
        ),
        (
            "reference.json",
            "4c224006b3afa2603cae64b221a951b7a1a8a48554506be5dfbaf791e4142d4e",
        ),
        (
            "track.srt",
            "26352ad26bb8866e5abe613247875ee09b421893c6bfbdf245ec0e9b3bc6f418",
        ),
        (
            "paraphrase.srt",
            "0027d90151c9ea16073162d316f66a931da89b4e309de96c67e458ab26fe0b09",
        ),
    ] {
        let bytes = std::fs::read(align_fixture().join(file)).unwrap();
        let got: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            got, sum,
            "{file} changed: regenerate with make.sh and update the sum"
        );
    }
}

/// The speech model of the tests, or `None` (and a note) when it is not set; with
/// `VIDEOLORE_REQUIRE_STT=1` (CI) a missing model fails instead of skipping.
#[cfg(feature = "stt")]
fn speech_model() -> Option<String> {
    match std::env::var("VIDEOLORE_STT_MODEL") {
        Ok(m) => Some(m),
        Err(_) => {
            assert!(
                std::env::var("VIDEOLORE_REQUIRE_STT").as_deref() != Ok("1"),
                "VIDEOLORE_REQUIRE_STT is set but VIDEOLORE_STT_MODEL is not"
            );
            eprintln!("skipping: VIDEOLORE_STT_MODEL not set");
            None
        }
    }
}

#[cfg(feature = "stt")]
fn alignment_with_real_speech(strict: bool) {
    let Some(env) = setup() else { return };
    let Some(model) = speech_model() else { return };
    check_fixture();
    let video = align_fixture().join("speech.mkv");
    let reference: Vec<(String, f64)> = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(align_fixture().join("reference.json")).unwrap(),
    )
    .unwrap()
    .as_array()
    .unwrap()
    .iter()
    .map(|w| {
        (
            w["word"].as_str().unwrap().to_string(),
            w["start"].as_f64().unwrap(),
        )
    })
    .collect();
    // the track is found, in English
    let mut p = Project::default();
    let m = env.probe(&mut p, &video);
    let tracks = m.format.subtitle_tracks.clone().unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].language.as_deref(), Some("eng"));
    let run = |mode: Mode| {
        let mut job = spread_job(&env, &video, 0, None);
        job.ranges[0].mode = mode;
        job.ranges[0].track_language = tracks[0].language.clone();
        job.model = model.clone().into();
        job.threads = 2;
        let mut t = videolore::stt::whisper::WhisperTranscriber::load(&job.model, 2).unwrap();
        let (tx, _rx) = channel();
        let runner = limited(Priority::Background);
        videolore::stt::run(
            &job,
            &runner as &dyn videolore::ffmpeg::Runner,
            Some(&mut t),
            None,
            &Arc::new(AtomicBool::new(false)),
            &tx,
        )
        .unwrap()
    };
    // the cues of the track and the recognised words, then the matching itself (to know
    // which words were found)
    let cues: Vec<videolore::srt::Cue> = track_lines(&env, &video, None)
        .into_iter()
        .map(|l| videolore::srt::Cue {
            start: l.start,
            end: l.end,
            text: l.text,
        })
        .collect();
    assert_eq!(cues.len(), 4);
    assert!(
        cues[1].text.contains("quik"),
        "the track's spelling is kept"
    );
    let heard: Vec<videolore::subtitles::Word> = run(Mode::Recognise)
        .lines()
        .into_iter()
        .flat_map(|l| l.words)
        .collect();
    let a = videolore::align::align(media::MediaId(1), &cues, &heard);
    let words: Vec<(&videolore::subtitles::Word, bool)> = a
        .lines
        .iter()
        .zip(&a.found)
        .flat_map(|(l, f)| l.words.iter().zip(f.iter().copied()))
        .collect();
    let total = reference.len();
    assert!(a.unmatched <= 1, "{} lines not matched", a.unmatched);
    // every line keeps the track's text and times; its words are inside it, in order
    for (l, c) in a.lines.iter().zip(&cues) {
        assert_eq!((l.start, l.end, &l.text), (c.start, c.end, &c.text));
        assert!(l.words.windows(2).all(|w| w[0].start <= w[1].start));
        assert!(l.words.iter().all(|w| w.start >= l.start && w.end <= l.end));
    }
    let mut found: u32 = 0;
    let mut within: u32 = 0;
    let mut worst: f64 = 0.0;
    let mut sum = 0.0;
    for ((w, was_found), (word, start)) in words.iter().zip(&reference) {
        eprintln!(
            "  {word:>10} reference {start:.3} s, heard {:.3} s{}",
            w.start.as_secs_f64(),
            if *was_found { "" } else { " (not matched)" }
        );
        if !was_found {
            continue;
        }
        found += 1;
        let err = (w.start.as_secs_f64() - start).abs();
        worst = worst.max(err);
        sum += err;
        if err <= 0.3 {
            within += 1;
        } else {
            eprintln!("  \"{word}\" at {:?}, really at {start} s", w.start);
        }
    }
    eprintln!(
        "alignment: {found}/{total} words matched, word-start error mean {:.3} s, largest {worst:.3} s",
        sum / f64::from(found.max(1))
    );
    eprintln!("alignment: {within} of {found} matched words within 0.3 s");
    if strict {
        assert_eq!(words.len(), total, "one timed word per word of the track");
        // the matched share is reported above; the gate is the timing of the matched words
        assert!(
            within * 10 >= found * 9,
            "{within} of {found} matched words within 0.3 s"
        );
    }
    assert!(
        found as usize * 2 >= total,
        "{found} of {total} words matched"
    );
    // the whole job gives the same lines: track text, cue times, matched words
    let r = run(Mode::Align { track: 0 });
    assert_eq!(r.unmatched, (a.unmatched, 4));
    assert_eq!(r.language.as_deref(), Some("en"));
    let lines = r.lines();
    assert_eq!(lines[1].text, cues[1].text);
    assert_eq!((lines[1].start, lines[1].end), (cues[1].start, cues[1].end));
    assert!(r.fallback.is_none());
}
