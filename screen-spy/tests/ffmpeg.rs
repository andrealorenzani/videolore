// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The recorder with the real ffmpeg.
//!
//! Without a display or a sound system: the recorder's own recording command, the screen
//! input replaced by a generated picture, the two sources sent by the real feeders. These
//! are skipped without ffmpeg, unless VIDEOLORE_REQUIRE_FFMPEG=1 (then they fail).
//!
//! The real capture (`x11grab` and PulseAudio) runs only with
//! VIDEOLORE_REQUIRE_CAPTURE=1: CI starts an X server (Xvfb) and a PulseAudio server
//! with a null sink for it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::time::{Duration, Instant};
use videolore::ffmpeg::{self, FfmpegInfo, Runner, SystemRunner, argv};
use videolore_screen_spy::app::RecorderState;
use videolore_screen_spy::app::{Command, Deps, Dialogs, NoPermissionNeeded, Phase, Setup};
use videolore_screen_spy::audio::{Feeder, PulseSound};
use videolore_screen_spy::plan::{self, AudioFormat, Capture, Os, Recording, VideoFormat};
use videolore_screen_spy::preview;
use videolore_screen_spy::process::{Child, Spawner, SystemSpawner};
use videolore_screen_spy::screens::SystemScreens;
use videolore_screen_spy::session::Platform;
use videolore_screen_spy::settings::RecorderSettings;

fn setup() -> Option<FfmpegInfo> {
    let found = ffmpeg::locate_both(None, None)
        .map_err(|e| e.to_string())
        .and_then(|l| ffmpeg::check_located(&SystemRunner, &l).map_err(|e| e.to_string()));
    match found {
        Ok(info) => Some(info),
        Err(e) => {
            if std::env::var("VIDEOLORE_REQUIRE_FFMPEG").as_deref() == Ok("1") {
                panic!("ffmpeg is required but unusable: {e}");
            }
            eprintln!("skipping: ffmpeg unavailable ({e})");
            None
        }
    }
}

/// Size, frame rate, duration in seconds, sound codec and the number of sound tracks.
#[derive(Debug)]
struct Probed {
    size: (u64, u64),
    rate: String,
    seconds: f64,
    audio: Vec<String>,
}

fn probe(info: &FfmpegInfo, file: &Path) -> Probed {
    let mut args = argv([
        "-v",
        "error",
        "-show_streams",
        "-show_format",
        "-count_packets",
        "-of",
        "json",
    ]);
    args.push(file.into());
    let o = SystemRunner.run(&info.ffprobe, &args).unwrap();
    assert!(o.success, "{}: {}", file.display(), o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let streams = v["streams"].as_array().unwrap();
    let video = streams.iter().find(|s| s["codec_type"] == "video").unwrap();
    assert_eq!(video["codec_name"], "h264");
    let rate = video["r_frame_rate"].as_str().unwrap().to_string();
    let frames: f64 = video["nb_read_packets"].as_str().unwrap().parse().unwrap();
    let fps: f64 = rate.split('/').next().unwrap().parse().unwrap();
    Probed {
        size: (
            video["width"].as_u64().unwrap(),
            video["height"].as_u64().unwrap(),
        ),
        rate,
        // a file cut short has no duration of its own: count its pictures
        seconds: frames / fps,
        audio: streams
            .iter()
            .filter(|s| s["codec_type"] == "audio")
            .map(|s| s["codec_name"].as_str().unwrap().to_string())
            .collect(),
    }
}

/// The whole file decodes without an error.
fn plays(info: &FfmpegInfo, file: &Path) {
    let mut args = argv(["-v", "error", "-xerror", "-i"]);
    args.push(file.into());
    args.extend(argv(["-f", "null", "-"]));
    let o = SystemRunner.run(&info.ffmpeg, &args).unwrap();
    assert!(o.success, "{}: {}", file.display(), o.stderr);
}

/// Sends a tone to `tx` in real time until the receiver is gone.
fn tone(tx: SyncSender<Vec<u8>>) {
    std::thread::spawn(move || {
        let block: Vec<u8> = (0..480i32)
            .flat_map(|i| {
                let s = ((i % 48 - 24) * 400) as i16;
                [s.to_le_bytes(), s.to_le_bytes()].concat()
            })
            .collect();
        while tx.send(block.clone()).is_ok() {
            std::thread::sleep(Duration::from_millis(10));
        }
    });
}

struct Take {
    child: Box<dyn Child>,
    preview: Arc<preview::Feed>,
    _feeders: [Feeder; 2],
    _quiet: SyncSender<Vec<u8>>,
}

/// Starts the recorder's own command, as on Windows and macOS, with a generated picture
/// in the screen's place: one source plays a tone, the other sends nothing at all.
fn start(info: &FfmpegInfo, audio: AudioFormat, file: &Path) -> Take {
    let screen = Capture {
        index: 0,
        x: 0,
        y: 0,
        width: 640,
        height: 480,
    };
    let (loud, loud_rx) = sync_channel(64);
    let (quiet, quiet_rx) = sync_channel(1);
    let feeders = [
        Feeder::start(loud_rx, 48_000).unwrap(),
        Feeder::start(quiet_rx, 44_100).unwrap(),
    ];
    tone(loud);
    let sources = [feeders[0].input(48_000), feeders[1].input(44_100)];
    let mut args = plan::record_args(&Recording {
        os: Os::Windows,
        screen,
        display: "",
        video: VideoFormat::Hd720,
        audio,
        sources: &sources,
        output: file,
        overwrite: false,
    });
    let from = args.iter().position(|a| a == "gdigrab").unwrap() - 1;
    let to = args.iter().position(|a| a == "desktop").unwrap();
    args.splice(
        from..=to,
        argv(["-re", "-f", "lavfi", "-i", "testsrc2=s=640x480:r=30"]),
    );
    let mut child = SystemSpawner.spawn(&info.ffmpeg, &args).unwrap();
    let preview = preview::start(child.take_stdout().unwrap(), 640, 480);
    Take {
        child,
        preview,
        _feeders: feeders,
        _quiet: quiet,
    }
}

/// Waits until the preview has `pictures`, failing when ffmpeg ends first.
fn wait_pictures(take: &mut Take, pictures: u64) {
    let end = Instant::now() + Duration::from_secs(30);
    while take.preview.count() < pictures {
        if let Some(exit) = take.child.try_wait() {
            panic!("ffmpeg ended: {}", exit.stderr);
        }
        assert!(Instant::now() < end, "only {}", take.preview.count());
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn stop(take: &mut Take) {
    take.child.quit();
    let end = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(exit) = take.child.try_wait() {
            assert!(exit.success, "{}", exit.stderr);
            return;
        }
        assert!(Instant::now() < end, "ffmpeg did not stop");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn recorded(audio: AudioFormat, codec: &str) {
    let Some(info) = setup() else { return };
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(plan::default_name("take", audio));
    let begin = Instant::now();
    let mut take = start(&info, audio, &file);
    // 5 pictures a second: the eleventh comes after 2 seconds
    wait_pictures(&mut take, 11);
    let elapsed = begin.elapsed();
    assert!(elapsed >= Duration::from_millis(1500), "{elapsed:?}");
    stop(&mut take);
    let p = probe(&info, &file);
    assert_eq!((p.size, p.rate.as_str()), ((1280, 720), "30/1"));
    assert_eq!(p.audio, [codec], "one sound track");
    assert!((1.8..8.0).contains(&p.seconds), "{p:?}");
    plays(&info, &file);
}

// covers: REQ-347, REQ-348, REQ-349, REQ-357
#[test]
fn an_aac_recording_is_an_mp4_with_one_mixed_track() {
    recorded(AudioFormat::Aac, "aac");
}

// covers: REQ-348
#[test]
fn an_mp3_recording_is_an_mp4() {
    recorded(AudioFormat::Mp3, "mp3");
}

// covers: REQ-348, REQ-347
#[test]
fn a_wav_recording_is_an_mkv() {
    recorded(AudioFormat::Wav, "pcm_s16le");
}

// covers: REQ-350
#[test]
fn a_recording_that_is_killed_still_plays_what_was_written() {
    let Some(info) = setup() else { return };
    let dir = tempfile::tempdir().unwrap();
    std::thread::scope(|s| {
        for audio in [AudioFormat::Aac, AudioFormat::Wav] {
            let file: PathBuf = dir.path().join(plan::default_name("cut", audio));
            let info = &info;
            s.spawn(move || {
                let mut take = start(info, audio, &file);
                wait_pictures(&mut take, 21);
                take.child.kill();
                assert!(!take.child.try_wait().unwrap().success);
                let p = probe(info, &file);
                assert!(p.seconds >= 1.0, "{p:?}");
                assert_eq!(p.audio.len(), 1);
            });
        }
    });
}

/// The save dialog is never opened here.
struct NoDialogs;

impl Dialogs for NoDialogs {
    fn save_file(&mut self, _: &Path, _: &str, _: &str) -> Option<PathBuf> {
        None
    }
}

fn wait_for(app: &mut RecorderState, what: &str, done: impl Fn(&RecorderState) -> bool) {
    let end = Instant::now() + Duration::from_secs(30);
    while !done(app) {
        app.tick(Instant::now());
        assert_eq!(app.message, None, "while waiting for {what}");
        assert_eq!(app.preview_error, None, "while waiting for {what}");
        assert!(Instant::now() < end, "no {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

// covers: REQ-341, REQ-343, REQ-344, REQ-347, REQ-348, REQ-349, REQ-350, REQ-357, REQ-358
#[test]
fn the_recorder_records_a_real_x11_screen_with_pulseaudio_sound() {
    if std::env::var("VIDEOLORE_REQUIRE_CAPTURE").as_deref() != Ok("1") {
        eprintln!("skipping: set VIDEOLORE_REQUIRE_CAPTURE=1 with an X and a sound server");
        return;
    }
    let info = setup().expect("ffmpeg");
    let dir = tempfile::tempdir().unwrap();
    let platform = Platform::detect(Os::Linux, |k| std::env::var_os(k), None);
    assert_eq!(platform.blocked, None);
    let runner: Arc<dyn Runner> = Arc::new(SystemRunner);
    let deps = Deps {
        spawner: Box::new(SystemSpawner),
        runner: Arc::clone(&runner),
        screens: Box::new(SystemScreens(Os::Linux)),
        sound: Arc::new(PulseSound { runner }),
        permission: Box::new(NoPermissionNeeded),
        dialogs: Box::new(NoDialogs),
        stamp: Box::new(|| "capture".into()),
    };
    let mut app = RecorderState::new(
        deps,
        Setup {
            platform,
            ffmpeg: Some(info.ffmpeg.clone()),
            settings: RecorderSettings::default(),
            settings_path: None,
            default_folder: dir.path().to_path_buf(),
        },
    );
    app.tick(Instant::now());
    assert!(!app.screens.is_empty(), "no screen listed");
    wait_for(&mut app, "preview", |a| a.frame.is_some());
    // the preview's rate: about 5 pictures a second
    let (first, since) = (app.frame_serial, Instant::now());
    wait_for(&mut app, "5 pictures", |a| a.frame_serial >= first + 5);
    assert!(since.elapsed() >= Duration::from_millis(700));
    assert!(since.elapsed() < Duration::from_millis(2500));
    for (audio, codec, name) in [
        (AudioFormat::Aac, "aac", "one"),
        (AudioFormat::Wav, "pcm_s16le", "two"),
    ] {
        app.dispatch(Command::SetAudio(audio), Instant::now());
        app.dispatch(Command::SetName(name.into()), Instant::now());
        app.dispatch(Command::ToggleRecording, Instant::now());
        // the checks run on their own thread: the window is not held
        assert_eq!(app.phase(), Phase::Starting, "{:?}", app.message);
        wait_for(&mut app, "the start", |a| a.phase() != Phase::Starting);
        assert_eq!(app.phase(), Phase::Recording, "{:?}", app.message);
        let first = app.frame_serial;
        wait_for(&mut app, "2 s of recording", |a| {
            a.frame_serial >= first + 11
        });
        let stopping = Instant::now();
        app.dispatch(Command::ToggleRecording, Instant::now());
        wait_for(&mut app, "the end", |a| !a.locked());
        // ffmpeg closed the file itself: no message, far from the 15 s limit
        assert_eq!(app.message, None);
        let took = stopping.elapsed();
        eprintln!("{codec}: stopped in {took:?}");
        assert!(took < Duration::from_secs(10), "{took:?}");
        let file = dir.path().join(format!("{name}.{}", audio.extension()));
        let p = probe(&info, &file);
        assert_eq!((p.size, p.rate.as_str()), ((1920, 1080), "30/1"));
        assert_eq!(p.audio, [codec]);
        assert!((1.5..8.0).contains(&p.seconds), "{p:?}");
        plays(&info, &file);
    }
    // killed during the recording: the file keeps what was written
    let args: Vec<OsString> = {
        use videolore_screen_spy::screens::ScreenSource;
        let screen = SystemScreens(Os::Linux).list()[0].capture(0);
        plan::record_args(&Recording {
            os: Os::Linux,
            screen,
            display: &app.platform.display,
            video: VideoFormat::Hd1080,
            audio: AudioFormat::Aac,
            sources: &[
                plan::AudioInput::Pulse(plan::PulseSource::Computer),
                plan::AudioInput::Pulse(plan::PulseSource::Microphone),
            ],
            output: &dir.path().join("killed.mp4"),
            overwrite: false,
        })
    };
    app.dispatch(Command::Close, Instant::now());
    let mut child = SystemSpawner.spawn(&info.ffmpeg, &args).unwrap();
    let feed = preview::start(child.take_stdout().unwrap(), 640, 360);
    let end = Instant::now() + Duration::from_secs(30);
    while feed.count() < 16 {
        assert!(child.try_wait().is_none() && Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill();
    assert!(probe(&info, &dir.path().join("killed.mp4")).seconds >= 1.0);
}

/// Seconds of the noise fixture, and its sample rate.
const FIXTURE_SECONDS: u32 = 9;
const RATE: usize = plan::SAMPLE_RATE as usize;

/// The mono samples of the lavfi source `source` after the filters `filters`.
fn filtered(info: &FfmpegInfo, source: &str, filters: &str) -> Vec<f32> {
    let graph = format!("{source},aformat=sample_fmts=fltp:channel_layouts=stereo,{filters}[o]");
    let args = argv([
        "-v",
        "error",
        "-nostdin",
        "-filter_complex",
        &graph,
        "-map",
        "[o]",
        "-ac",
        "1",
        "-ar",
        "48000",
        "-f",
        "f32le",
        "pipe:1",
    ]);
    let o = SystemRunner.run(&info.ffmpeg, &args).unwrap();
    assert!(o.success, "{}", o.stderr);
    let (samples, _) = o.stdout.as_chunks::<4>();
    samples.iter().map(|&b| f32::from_le_bytes(b)).collect()
}

/// The level in dB of the samples whose place in their 1.5 s period is in `part`, after the
/// first 3 s.
fn level(samples: &[f32], part: std::ops::Range<f64>) -> f64 {
    let v: Vec<f64> = samples
        .iter()
        .enumerate()
        .skip(3 * RATE)
        .filter(|(i, _)| part.contains(&((*i as f64 / RATE as f64) % 1.5)))
        .map(|(_, s)| f64::from(*s) * f64::from(*s))
        .collect();
    10.0 * (v.iter().sum::<f64>() / v.len() as f64 + 1e-20).log10()
}

// covers: REQ-377
#[test]
fn steady_noise_is_reduced_and_speech_keeps_its_level() {
    let Some(info) = setup() else { return };
    // speech-like bursts (harmonics of a voice, a syllable rhythm) in the last 0.6 s of each
    // 1.5 s, over steady noise with a fixed seed
    let speech = "0.18*(sin(2*PI*180*t)+0.6*sin(2*PI*360*t)+0.4*sin(2*PI*720*t)\
                  +0.25*sin(2*PI*1440*t))*gte(mod(t\\,1.5)\\,0.9)*(0.6+0.4*sin(2*PI*4*t))";
    for (colour, amplitude) in [("white", 0.005), ("white", 0.05), ("pink", 0.05)] {
        let source = format!(
            "aevalsrc={speech}:s={RATE}:d={FIXTURE_SECONDS}[s];\
             anoisesrc=r={RATE}:d={FIXTURE_SECONDS}:a={amplitude}:c={colour}:seed=7[n];\
             [s][n]amix=inputs=2:normalize=0"
        );
        let plain = filtered(&info, &source, "alimiter=level=false");
        let clean = filtered(&info, &source, &plan::mix_filters());
        let quiet = level(&plain, 0.15..0.8) - level(&clean, 0.15..0.8);
        let voice = level(&clean, 0.95..1.45) - level(&plain, 0.95..1.45);
        eprintln!("{colour} {amplitude}: noise -{quiet:.1} dB, speech {voice:+.2} dB");
        assert!(quiet >= 10.0, "{colour} {amplitude}: {quiet}");
        assert!(voice.abs() <= 3.0, "{colour} {amplitude}: {voice}");
    }
}

// covers: REQ-377
#[test]
fn the_noise_reduction_does_not_move_the_sound_against_the_picture() {
    let Some(info) = setup() else { return };
    // one click at 1 s in silence
    let source = "aevalsrc=if(between(n\\,48000\\,48479)\\,0.8*sin(2*PI*1000*n/48000)\\,0)\
                  :s=48000:d=2";
    let start = |s: &[f32]| {
        let top = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(top > 0.3, "the click is heard: {top}");
        s.iter().position(|x| x.abs() > top / 2.0).unwrap()
    };
    let plain = start(&filtered(&info, source, "alimiter=level=false"));
    let clean = start(&filtered(&info, source, &plan::mix_filters()));
    let shift = clean.abs_diff(plain);
    eprintln!("click at {plain} without the reduction, {clean} with it");
    // far inside one picture at 60 pictures a second (800 samples)
    assert!(shift <= 48, "{shift} samples");
}
