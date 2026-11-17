// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 1.4.0: Clean audio, the Export dialog's formats (presets, audio
//! formats, only audio). Fixtures are generated here: nothing binary is kept in git.

use super::*;
use videolore::format::{AudioChoice, FormatChoice};

/// A 7 s clip: black picture; wind-like pink noise all the time, speech-like sound (a
/// voiced tone with a syllable rhythm) from 2 s to 5 s.
fn noisy_speech(env: &Env) -> PathBuf {
    env.fixture(
        "noisy-speech.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x180:r=10:d=7",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc='0.3*(sin(2*PI*150*t)+0.6*sin(2*PI*300*t)+0.4*sin(2*PI*450*t)\
             +0.3*sin(2*PI*1200*t))*(0.6+0.4*sin(2*PI*4*t))*between(t,2,5)':s=48000:d=7",
            "-f",
            "lavfi",
            "-i",
            "anoisesrc=color=pink:amplitude=0.08:d=7:r=48000:seed=7",
            "-filter_complex",
            "[1][2]amix=inputs=2:normalize=0[a]",
            "-map",
            "0:v",
            "-map",
            "[a]",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ],
    )
}

/// Mean volume (dB) of `len` seconds from `from`.
fn level(env: &Env, file: &Path, from: f64, len: f64) -> f64 {
    let o = Command::new(&env.info.ffmpeg)
        .args(["-hide_banner", "-nostats", "-ss"])
        .arg(format!("{from}"))
        .arg("-t")
        .arg(format!("{len}"))
        .arg("-i")
        .arg(file)
        .args(["-vn", "-af", "volumedetect", "-f", "null", "-"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stderr)
        .lines()
        .find_map(|l| l.split("mean_volume: ").nth(1))
        .and_then(|v| v.trim_end_matches(" dB").trim().parse().ok())
        .unwrap_or(-91.0)
}

fn project_with(env: &Env, file: &Path) -> Project {
    let mut p = Project::default();
    let m = env.probe(&mut p, file);
    p.timeline.append(&m).unwrap();
    p
}

/// Streams of a file: (codec_type, codec_name, width, height, avg_frame_rate).
fn streams(env: &Env, file: &Path) -> Vec<(String, String, u64, u64, String)> {
    let o = Command::new(&env.info.ffprobe)
        .args(["-v", "error", "-show_streams", "-of", "json"])
        .arg(file)
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    v["streams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["codec_type"].as_str().unwrap_or("").to_string(),
                s["codec_name"].as_str().unwrap_or("").to_string(),
                s["width"].as_u64().unwrap_or(0),
                s["height"].as_u64().unwrap_or(0),
                s["avg_frame_rate"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

fn duration(env: &Env, file: &Path) -> f64 {
    let o = Command::new(&env.info.ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout).trim().parse().unwrap()
}

// covers: REQ-114
#[test]
fn clean_audio_lowers_the_noise_and_keeps_the_speech() {
    let Some(env) = setup() else { return };
    let file = noisy_speech(&env);
    let mut p = project_with(&env, &file);
    // the encoder gets a bit rate that does not hide the difference (the source's is 64k)
    p.media[0].format.audio_bit_rate = Some(192_000);
    let copy = env.dir.path().join("copy.mp4");
    env.export(&p, &copy).0.unwrap();
    // (b) without Clean audio the sound is the source's
    let src_noise = level(&env, &file, 0.3, 1.5);
    let src_speech = level(&env, &file, 2.5, 2.0);
    assert!((level(&env, &copy, 0.3, 1.5) - src_noise).abs() < 1.0);
    assert!((level(&env, &copy, 2.5, 2.0) - src_speech).abs() < 1.0);
    // the same encoding pass without Clean audio is the baseline of the comparison (a
    // mono source becomes stereo there, which lowers the measured level by itself)
    let plain = env.dir.path().join("plain.mp4");
    let mut job = env.job(&p, &plain);
    job.format.audio = AudioChoice::Aac;
    let (tx, _rx) = channel();
    export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    let base_speech = level(&env, &plain, 2.5, 2.0);
    // (a) with it: noise-only parts at least 6 dB lower, speech within 3 dB
    let id = p.timeline.clips[0].id;
    p.timeline.set_clean_audio(&[id], true).unwrap();
    let clean = env.dir.path().join("clean.mp4");
    let (r, events) = env.export(&p, &clean);
    r.unwrap();
    // (d) it is a re-encode
    assert!(events.contains(&ExportEvent::Started { stream_copy: false }));
    for (from, len) in [(0.3, 1.5), (5.5, 1.3)] {
        let before = level(&env, &plain, from, len);
        let after = level(&env, &clean, from, len);
        assert!(
            after <= before - 6.0,
            "noise at {from}s: {before} -> {after} dB"
        );
    }
    let speech = level(&env, &clean, 2.5, 2.0);
    assert!(
        (speech - base_speech).abs() <= 3.0,
        "speech: {base_speech} -> {speech} dB"
    );
}

// covers: REQ-119, REQ-114
#[test]
fn only_audio_has_the_length_of_the_timeline_and_the_music_at_its_volume() {
    let Some(env) = setup() else { return };
    // a silent 10 s picture with a 3 s music clip placed at 2 s
    let video = env.clip("silent.mp4", 320, 180, 10, 10, false);
    let song = env.fixture(
        "song-880.wav",
        &["-f", "lavfi", "-i", "sine=f=880:d=6", "-c:a", "pcm_s16le"],
    );
    let mut p = project_with(&env, &video);
    let m = env.probe(&mut p, &song);
    p.music.place(&m, Micros(2_000_000), Micros(1)).unwrap();
    let out = env.dir.path().join("sound.m4a");
    let mut job = env.job(&p, &out);
    job.audio_only = true;
    let (tx, _rx) = channel();
    let (path, _) = export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    assert_eq!(path, out);
    // an audio file of the timeline's length, no picture
    let s = streams(&env, &out);
    assert!(s.iter().all(|x| x.0 == "audio"), "{s:?}");
    assert!((duration(&env, &out) - 10.0).abs() < 0.2);
    // the music is there at its place and silence elsewhere
    assert!(level(&env, &out, 3.0, 2.0) > -40.0, "music");
    assert!(level(&env, &out, 0.2, 1.5) < -60.0, "silence before");
    assert!(level(&env, &out, 8.5, 1.0) < -60.0, "silence after");
    // at half volume the music is about 12 dB lower (the gain is squared)
    let loud = level(&env, &out, 3.0, 2.0);
    p.mix.music_volume = 50;
    let half = env.dir.path().join("half.m4a");
    let mut job = env.job(&p, &half);
    job.audio_only = true;
    export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    let quiet = level(&env, &half, 3.0, 2.0);
    assert!((loud - quiet - 12.0).abs() < 2.0, "{loud} -> {quiet}");
    // a project with a text and subtitles still makes only sound
    assert!(job.project.texts.is_empty());
}

// covers: REQ-121
#[test]
fn the_chosen_audio_format_is_made() {
    let Some(env) = setup() else { return };
    let file = env.clip("tone.mp4", 320, 180, 10, 3, true);
    let p = project_with(&env, &file);
    for (choice, name, codec) in [
        (AudioChoice::Aac, "a.m4a", "aac"),
        (AudioChoice::Mp3, "a.mp3", "mp3"),
    ] {
        if !choice.available(&env.info.encoders) {
            continue;
        }
        let out = env.dir.path().join(name);
        let mut job = env.job(&p, &out);
        job.audio_only = true;
        job.format = FormatChoice {
            video: None,
            audio: choice,
        };
        let (tx, _rx) = channel();
        export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
        let s = streams(&env, &out);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].1, codec);
        assert!((duration(&env, &out) - 3.0).abs() < 0.2);
    }
    // a video with the mp3 sound: the audio is re-encoded, the picture is the source's
    if AudioChoice::Mp3.available(&env.info.encoders) {
        let out = env.dir.path().join("v.mp4");
        let mut job = env.job(&p, &out);
        job.format.audio = AudioChoice::Mp3;
        let (tx, _rx) = channel();
        export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
        let s = streams(&env, &out);
        assert!(s.iter().any(|x| x.0 == "audio" && x.1 == "mp3"), "{s:?}");
    }
}

// covers: REQ-120
#[test]
fn a_video_preset_gives_its_size_frame_rate_and_codec() {
    let Some(env) = setup() else { return };
    let file = env.clip("big.mp4", 640, 360, 30, 3, true);
    let p = project_with(&env, &file);
    // 1280x720 25 fps libx264 .mp4 (preset 6)
    let out = env.dir.path().join("p.mp4");
    let mut job = env.job(&p, &out);
    job.format.video = Some(6);
    let (tx, rx) = channel();
    export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    assert!(
        rx.try_iter()
            .any(|e| e == ExportEvent::Started { stream_copy: false })
    );
    let s = streams(&env, &out);
    let v = s.iter().find(|x| x.0 == "video").unwrap();
    assert_eq!(
        (v.1.as_str(), v.2, v.3, v.4.as_str()),
        ("h264", 1280, 720, "25/1")
    );
    // HEVC in MP4 carries hvc1
    if env.info.encoders.contains("libx265") {
        let out = env.dir.path().join("h.mp4");
        let mut job = env.job(&p, &out);
        job.format.video = Some(8);
        export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
        assert_eq!(env.video_tag(&out), "hvc1");
    }
    // "same as source" with identical clips and no effect is not re-encoded
    let same = env.dir.path().join("same.mp4");
    let (tx, rx) = channel();
    export::run(
        &env.job(&p, &same),
        &SystemRunner,
        &AtomicBool::new(false),
        &tx,
    )
    .unwrap();
    assert!(
        rx.try_iter()
            .any(|e| e == ExportEvent::Started { stream_copy: true })
    );
}
