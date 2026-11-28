// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the pure parts of the recorder for the three systems (ADR-268), and of the
//! parts that need only a file, a stream or a connection on this machine.

use crate::app_tests::{Scripted, fail, ok};
use crate::audio::{self, Feeder, PulseSound, Sound};
use crate::devices::{self, PulseDevice, microphone_problem, parse_devices, parse_pulse_sources};
use crate::plan::{
    self, AudioFormat, AudioInput, Capture, Os, PulseSource, Recording, VideoFormat,
};
use crate::preview::{self, Feed};
use crate::process;
#[cfg(unix)]
use crate::process::{Spawner, SystemSpawner};
use crate::screens::{self, Reported, Screen};
use crate::session::{
    self, MacVersion, MicrophoneAccess, MicrophonePermission, Platform, microphone_gate,
};
use crate::settings::{self, RecorderSettings};
use std::ffi::OsString;
use std::io::{Cursor, Read};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SCREEN: Capture = Capture {
    index: 1,
    x: -1920,
    y: 40,
    width: 1920,
    height: 1080,
};

fn text(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

fn has(args: &[String], run: &[&str]) -> bool {
    args.windows(run.len()).any(|w| w == run)
}

fn record(os: Os, video: VideoFormat, audio: AudioFormat, overwrite: bool) -> Vec<String> {
    let sources = match os {
        Os::Linux => [
            AudioInput::Pulse(PulseSource::Computer),
            AudioInput::Pulse(PulseSource::Microphone),
        ],
        _ => [4001, 4002].map(|port| AudioInput::Tcp {
            port,
            rate: 44_100,
            channels: 2,
        }),
    };
    text(&plan::record_args(&Recording {
        os,
        screen: SCREEN,
        display: ":0",
        video,
        audio,
        sources: &sources,
        output: Path::new("/videos/take one.mp4"),
        overwrite,
    }))
}

// covers: REQ-341, REQ-344
#[test]
fn each_system_captures_the_chosen_screen_with_its_own_ffmpeg_input() {
    let w = record(Os::Windows, VideoFormat::Hd1080, AudioFormat::Aac, false);
    assert!(has(&w, &["-f", "gdigrab", "-framerate", "30"]));
    assert!(has(&w, &["-offset_x", "-1920", "-offset_y", "40"]));
    assert!(has(&w, &["-video_size", "1920x1080", "-i", "desktop"]));
    let m = record(Os::MacOs, VideoFormat::Hd1080, AudioFormat::Aac, false);
    assert!(has(&m, &["-f", "avfoundation", "-capture_cursor", "1"]));
    assert!(has(
        &m,
        &["-framerate", "30", "-i", "Capture screen 1:none"]
    ));
    let l = record(Os::Linux, VideoFormat::Hd1080, AudioFormat::Aac, false);
    assert!(has(&l, &["-f", "x11grab", "-framerate", "30"]));
    assert!(has(&l, &["-video_size", "1920x1080", "-i", ":0+-1920,40"]));
    assert_eq!(Os::current() == Os::Linux, cfg!(target_os = "linux"));
}

// covers: REQ-343, REQ-347
#[test]
fn the_four_video_formats_set_size_and_rate_of_the_file_and_the_preview_stays_apart() {
    let table: Vec<_> = VideoFormat::ALL
        .map(|f| (f.size(), f.fps(), f.label(), f.key()))
        .into();
    assert_eq!(
        table,
        [
            ((1280, 720), 30, "1280x720, 30 fps", "720p30"),
            ((1920, 1080), 30, "1920x1080, 30 fps", "1080p30"),
            ((1920, 1080), 60, "1920x1080, 60 fps", "1080p60"),
            (
                (3840, 2160),
                30,
                "4K UHD 2160p (3840x2160), 30 fps",
                "2160p30"
            ),
        ]
    );
    assert_eq!(VideoFormat::default(), VideoFormat::Hd1080);
    for f in VideoFormat::ALL {
        assert_eq!(VideoFormat::from_key(f.key()), Some(f));
        let a = record(Os::Linux, f, AudioFormat::Aac, false);
        let graph = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        let (w, h) = f.size();
        assert!(graph.contains(&format!(
            "scale={w}:{h}:force_original_aspect_ratio=decrease"
        )));
        assert!(graph.contains(&format!("pad={w}:{h}:")));
        // the file's rate is the format's; the preview's is its own
        assert!(graph.contains(&format!("setsar=1,fps={},format=yuv420p[v]", f.fps())));
        assert!(graph.contains("[pv]fps=5,scale=640:360,format=rgb24[p]"));
        assert!(has(&a, &["-framerate", &f.fps().to_string()]));
        assert!(has(&a, &["-g", &f.fps().to_string()]));
        // the preview goes to the standard output, never into the file
        let file = a.iter().position(|x| x.starts_with("file:")).unwrap();
        assert_eq!(&a[file + 1..file + 3], ["-map", "[p]"]);
        assert_eq!(a.last().unwrap(), "pipe:1");
    }
    assert_eq!(VideoFormat::from_key("480p"), None);
    // before Start the preview alone: 5 pictures a second, 640 wide, no input control
    let p = text(&plan::preview_args(Os::Windows, &SCREEN, ""));
    assert!(has(&p, &["-framerate", "5"]) && p.contains(&"-nostdin".to_string()));
    assert!(has(&p, &["-vf", "fps=5,scale=640:360,format=rgb24"]));
    assert!(!p.iter().any(|x| x.starts_with("file:")));
    assert_eq!(plan::preview_size(1080, 1920), (640, 1138));
    assert_eq!(plan::preview_size(2560, 1600), (640, 400));
    assert_eq!(plan::preview_size(0, 0), (640, 2));
    assert_eq!(plan::preview_size(1, 4000).1, 2160);
}

// covers: REQ-348, REQ-347
#[test]
fn the_sound_format_decides_the_container_and_no_pair_is_refused() {
    let table: Vec<_> = AudioFormat::ALL
        .map(|a| (a.label(), a.encoder(), a.extension()))
        .into();
    assert_eq!(
        table,
        [
            ("AAC", "aac", "mp4"),
            ("MP3", "libmp3lame", "mp4"),
            ("WAV", "pcm_s16le", "mkv"),
        ]
    );
    assert_eq!(AudioFormat::default(), AudioFormat::Aac);
    for os in [Os::Linux, Os::Windows, Os::MacOs] {
        for video in VideoFormat::ALL {
            for audio in AudioFormat::ALL {
                assert_eq!(AudioFormat::from_key(audio.key()), Some(audio));
                let a = record(os, video, audio, false);
                assert!(has(&a, &["-c:a", audio.encoder()]));
                assert_eq!(a.contains(&"-b:a".to_string()), audio != AudioFormat::Wav);
                if audio == AudioFormat::Wav {
                    assert!(has(&a, &["-cluster_time_limit", "1000", "-f", "matroska"]));
                    assert!(!a.contains(&"-movflags".to_string()));
                } else {
                    let flags = "frag_keyframe+empty_moov+default_base_moof";
                    assert!(has(&a, &["-movflags", flags, "-f", "mp4"]));
                }
                assert!(has(&a, &["-c:v", "libx264", "-preset", "veryfast"]));
            }
        }
    }
}

// covers: REQ-357
#[test]
fn both_sources_are_mixed_into_the_one_sound_track() {
    for os in [Os::Linux, Os::Windows, Os::MacOs] {
        let a = record(os, VideoFormat::Hd720, AudioFormat::Aac, false);
        let graph = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        assert!(graph.contains("[0:a]aresample=48000:async=1:first_pts=0"));
        assert!(graph.contains("[1:a]aresample=48000:async=1:first_pts=0"));
        assert!(graph.contains("[a0][a1]amix=inputs=2:duration=longest:normalize=0"));
        // the screen is opened last, so that the sound runs when the picture starts
        assert!(graph.starts_with("[2:v]split=2"));
        let inputs: Vec<&String> = a
            .windows(2)
            .filter(|w| w[0] == "-i")
            .map(|w| &w[1])
            .collect();
        assert_eq!(inputs.len(), 3);
        assert!(
            inputs[2].starts_with(":0+") || inputs[2] == "desktop" || inputs[2].ends_with(":none")
        );
        // only the sources that share the screen's clock are aligned by it
        let aligned = a.windows(2).filter(|w| w == &["-isync", "2"]).count();
        assert_eq!(aligned, if os == Os::Linux { 2 } else { 0 });
        // one picture and one sound go into the file
        assert_eq!(a.iter().filter(|x| *x == "-map").count(), 3);
        assert!(has(&a, &["-map", "[v]", "-map", "[a]"]));
        assert!(!a.contains(&"-nostdin".to_string()), "Stop writes to it");
    }
    let l = record(Os::Linux, VideoFormat::Hd720, AudioFormat::Aac, false);
    assert!(has(&l, &["-f", "pulse", "-i", "@DEFAULT_MONITOR@"]));
    assert!(has(&l, &["-f", "pulse", "-i", "@DEFAULT_SOURCE@"]));
    let w = record(Os::Windows, VideoFormat::Hd720, AudioFormat::Aac, false);
    for port in ["4001", "4002"] {
        let url = format!("tcp://127.0.0.1:{port}");
        assert!(has(
            &w,
            &["-f", "s16le", "-ar", "44100", "-ac", "2", "-i", &url]
        ));
    }
}

// covers: REQ-346
#[test]
fn ffmpeg_replaces_a_file_only_after_replace_was_chosen() {
    let no = record(Os::Linux, VideoFormat::Hd720, AudioFormat::Aac, false);
    assert!(has(&no, &["-n", "file:/videos/take one.mp4"]));
    assert!(!no.contains(&"-y".to_string()));
    let yes = record(Os::Linux, VideoFormat::Hd720, AudioFormat::Aac, true);
    assert!(has(&yes, &["-y", "file:/videos/take one.mp4"]));
}

// covers: REQ-345, REQ-348
#[test]
fn file_names_carry_the_start_time_and_the_extension_of_the_sound_format() {
    let stamp = "2026-11-26-101500";
    let mp4 = "screen-recording-2026-11-26-101500.mp4";
    assert_eq!(plan::default_name(stamp, AudioFormat::Aac), mp4);
    assert_eq!(plan::default_name(stamp, AudioFormat::Mp3), mp4);
    assert!(plan::default_name(stamp, AudioFormat::Wav).ends_with("101500.mkv"));
    for (typed, audio, want) in [
        ("talk", AudioFormat::Aac, "talk.mp4"),
        (" talk.MKV ", AudioFormat::Aac, "talk.mp4"),
        ("talk.mp4", AudioFormat::Wav, "talk.mkv"),
        ("v1.2", AudioFormat::Mp3, "v1.2.mp4"),
    ] {
        assert_eq!(plan::file_name(typed, audio).unwrap(), want, "{typed}");
    }
    for bad in ["", "  ", ".mp4", "..", "a/b", "a\\b", "a\nb"] {
        assert!(plan::file_name(bad, AudioFormat::Aac).is_err(), "{bad:?}");
    }
    assert!(plan::file_name(&"x".repeat(197), AudioFormat::Aac).is_err());
    // the stamp of this machine has the written shape
    let now = crate::local_stamp();
    assert_eq!(now.len(), 17, "{now}");
    assert!(now.chars().all(|c| c.is_ascii_digit() || c == '-'));
    // the system's video folder, else home
    let (v, h) = (PathBuf::from("/h/Videos"), PathBuf::from("/h"));
    assert_eq!(settings::folder_of(Some(v.clone()), Some(h.clone())), v);
    assert_eq!(settings::folder_of(None, Some(h.clone())), h);
    assert_eq!(settings::folder_of(None, None), PathBuf::from("."));
    assert!(!settings::default_folder().as_os_str().is_empty());
}

fn env_of<'a>(vars: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
    |k| {
        vars.iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| OsString::from(v))
    }
}

// covers: REQ-358
#[test]
fn a_native_wayland_session_is_refused_and_x11_and_xwayland_without_it_record() {
    let linux = |vars: &[(&str, &str)]| Platform::detect(Os::Linux, env_of(vars), None);
    let x11 = linux(&[("DISPLAY", ":0"), ("XDG_SESSION_TYPE", "x11")]);
    assert_eq!((x11.display.as_str(), &x11.blocked), (":0", &None));
    assert!(x11.can_preview());
    // an X display with an empty Wayland name is X11
    let empty = linux(&[("DISPLAY", ":1.0"), ("WAYLAND_DISPLAY", "")]);
    assert_eq!(empty.blocked, None);
    for vars in [
        &[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0")][..],
        &[("DISPLAY", ":0"), ("XDG_SESSION_TYPE", "Wayland")][..],
    ] {
        let p = linux(vars);
        let why = p.blocked.clone().unwrap();
        assert!(why.contains("needs an X11 session") && why.contains("comes later"));
        assert!(!p.can_preview());
    }
    for vars in [&[][..], &[("DISPLAY", "$(rm)")][..]] {
        assert_eq!(
            linux(vars).blocked.as_deref(),
            Some(session::NO_DISPLAY_MESSAGE)
        );
    }
    for good in [":0", ":0.0", "localhost:10.0", "/tmp/launch/org.x:0"] {
        assert!(session::valid_display(good), "{good}");
    }
    for bad in ["", "0", ":", ":a", ":0.", ":0.x", "-i:0", "a b:0", ":0+1,2"] {
        assert!(!session::valid_display(bad), "{bad:?}");
    }
    // Windows asks nothing of the environment
    assert_eq!(
        Platform::detect(Os::Windows, env_of(&[]), None).blocked,
        None
    );
}

// covers: REQ-357, REQ-341
#[test]
fn macos_below_14_6_is_told_which_version_it_needs() {
    let mac = |v| Platform::detect(Os::MacOs, env_of(&[]), v);
    for (major, minor) in [(14, 6), (15, 0), (26, 1)] {
        assert_eq!(mac(Some(MacVersion { major, minor })).blocked, None);
    }
    for (major, minor) in [(11, 0), (14, 5), (13, 7)] {
        let p = mac(Some(MacVersion { major, minor }));
        let why = p.blocked.clone().unwrap();
        assert!(why.contains("macOS 14.6 or later"), "{why}");
        assert!(why.contains(&format!("macOS {major}.{minor}")), "{why}");
        // the program still opens and shows the screen
        assert!(p.can_preview());
    }
    assert!(mac(None).blocked.unwrap().contains("14.6"));
    assert!(session::SCREEN_NOT_ALLOWED.contains("Screen Recording"));
    assert!(session::MICROPHONE_NOT_ALLOWED.contains("microphone"));
}

// covers: REQ-344
#[test]
fn screens_are_numbered_with_their_size_in_pixels() {
    let r = Reported {
        id: 7,
        x: 1280,
        y: 0,
        width: 1097,
        height: 617,
        scale: 1.75,
    };
    let linux = screens::physical(Os::Linux, r);
    // scaled back to pixels, never beyond the real 1920x1080
    assert_eq!((linux.x, linux.width, linux.height), (2240, 1920, 1080));
    let win = screens::physical(Os::Windows, r);
    assert_eq!((win.x, win.width, win.height), (1280, 1097, 617));
    let mac = screens::physical(
        Os::MacOs,
        Reported {
            width: 1440,
            height: 900,
            scale: 2.0,
            ..r
        },
    );
    assert_eq!((mac.width, mac.height), (2880, 1800));
    for scale in [0.0, f32::NAN, -1.0] {
        assert_eq!(
            screens::physical(Os::Linux, Reported { scale, ..r }).width,
            1097
        );
    }
    let s = Screen {
        id: 1,
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    assert_eq!(screens::label(1, &s), "Screen 2, 1920x1080");
    assert_eq!(s.capture(3).index, 3);
}

const DEVICES_6: &str = "Devices:
 D. = Demuxing supported
 .E = Muxing supported
 --
 DE alsa            ALSA audio output
  E caca            caca (color ASCII art) output device
 DE pulse           Pulse audio output
 D  x11grab         X11 screen capture, using XCB
 D  video4linux2,v4l2 Video4Linux2 device grab
";
const DEVICES_9: &str = "Formats:
 D.. = Demuxing supported
 .E. = Muxing supported
 ..d = Is a device
 ---
 D d gdigrab         GDI API Windows frame grabber
 D d avfoundation    AVFoundation input device
";

// covers: REQ-350
#[test]
fn a_missing_capture_device_or_encoder_is_named() {
    let six = parse_devices(DEVICES_6);
    let names: Vec<&str> = six.iter().map(String::as_str).collect();
    assert_eq!(names, ["alsa", "pulse", "v4l2", "video4linux2", "x11grab"]);
    let nine = parse_devices(DEVICES_9);
    assert!(nine.contains("gdigrab") && nine.contains("avfoundation"));
    assert_eq!(nine.len(), 2);
    let encoders = ["libx264", "aac"].map(String::from).into();
    assert!(devices::missing(Os::Linux, AudioFormat::Aac, &six, &encoders).is_empty());
    assert_eq!(
        devices::missing(Os::Linux, AudioFormat::Mp3, &nine, &Default::default()),
        [
            "the capture device x11grab",
            "the capture device pulse",
            "the encoder libx264",
            "the encoder libmp3lame",
        ]
    );
    assert!(devices::missing(Os::Windows, AudioFormat::Aac, &nine, &encoders).is_empty());
    assert!(devices::missing(Os::MacOs, AudioFormat::Aac, &nine, &encoders).is_empty());
}

const SOURCES: &str = "Auto-detected sources for pulse:
  out.monitor [Monitor of Null Output] (none)
* alsa_input.usb [USB Microphone] (none)
";

// covers: REQ-357
#[test]
fn on_linux_each_source_must_open_and_the_missing_one_is_named() {
    let device = |name: &str, default| PulseDevice {
        name: name.into(),
        default,
    };
    assert_eq!(
        parse_pulse_sources(SOURCES),
        [device("out.monitor", false), device("alsa_input.usb", true)]
    );
    assert_eq!(microphone_problem(&parse_pulse_sources(SOURCES)), None);
    assert_eq!(
        microphone_problem(&[]),
        None,
        "an unreadable list tells nothing"
    );
    assert_eq!(
        microphone_problem(&[device("out.monitor", true)]),
        Some("no microphone was found")
    );
    // a monitor as the default input would record the computer's sound twice, even with
    // a microphone connected
    let monitor_first = [device("out.monitor", true), device("mic", false)];
    let why = microphone_problem(&monitor_first).unwrap();
    assert!(
        why.contains("twice") && why.contains("default input"),
        "{why}"
    );
    assert_eq!(
        microphone_problem(&[device("a.monitor", false), device("mic", false)]),
        None
    );
    let sound = |list: &'static str, broken: &'static str| {
        let runner = Scripted::new(move |a| {
            if a.contains(&"-sources".to_string()) {
                return ok(list);
            }
            if a.contains(&broken.to_string()) {
                return fail("Input/output error\n\n");
            }
            ok("")
        });
        PulseSound {
            runner: Arc::new(runner),
        }
        .open(Path::new("ffmpeg"), &AtomicBool::new(false))
    };
    let both = sound(SOURCES, "none").ok().unwrap();
    assert_eq!(
        both.inputs,
        [
            AudioInput::Pulse(PulseSource::Computer),
            AudioInput::Pulse(PulseSource::Microphone),
        ]
    );
    let e = sound(SOURCES, "@DEFAULT_SOURCE@").err().unwrap();
    assert!(e.starts_with("The microphone is not available: Input/output error"));
    let e = sound(SOURCES, "@DEFAULT_MONITOR@").err().unwrap();
    assert!(e.starts_with("Computer sound is not available"), "{e}");
    let e = sound(
        "Auto-detected sources for pulse:\n  a.monitor [m] (none)\n",
        "none",
    )
    .err()
    .unwrap();
    assert!(e.contains("no microphone was found") && e.contains("Nothing was recorded"));
    let e = sound(
        "Auto-detected sources for pulse:\n* a.monitor [m] (none)\n  mic [M] (none)\n",
        "none",
    )
    .err()
    .unwrap();
    assert!(
        e.starts_with("The microphone is not available") && e.contains("twice"),
        "{e}"
    );
    // a cancelled start opens no more sources
    let silent = Arc::new(Scripted::new(|_| ok("")));
    let cancelled = PulseSound {
        runner: silent.clone(),
    }
    .open(Path::new("ffmpeg"), &AtomicBool::new(true));
    assert_eq!(cancelled.err().as_deref(), Some(audio::CANCELLED));
    assert_eq!(silent.calls.load(Ordering::SeqCst), 1, "only the list");
    assert_eq!(
        audio::source_error(PulseSource::Microphone, ""),
        "The microphone is not available. Nothing was recorded."
    );
    // a program that cannot be started names the source too
    let broken = PulseSound {
        runner: Arc::new(videolore::ffmpeg::SystemRunner),
    };
    let e = broken
        .open(Path::new("/no/such/ffmpeg"), &AtomicBool::new(false))
        .err()
        .unwrap();
    assert!(e.starts_with("Computer sound is not available"), "{e}");
    let _ = audio::system_sound(Arc::new(videolore::ffmpeg::SystemRunner));
}

// covers: REQ-357
#[test]
fn samples_become_16_bit_stereo_and_silence_keeps_the_clock() {
    let mut out = Vec::new();
    let pairs = |b: &[u8]| -> Vec<i16> {
        b.chunks(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    };
    audio::to_stereo_s16le(&[0.0f32, 1.0, -1.0, 2.0, f32::NAN], 1, &mut out);
    assert_eq!(
        pairs(&out),
        [0, 0, 32767, 32767, -32767, -32767, 32767, 32767, 0, 0]
    );
    out.clear();
    audio::to_stereo_s16le(&[1i16, 2, 3, 4, 5], 2, &mut out);
    assert_eq!(pairs(&out), [1, 2, 3, 4], "a half frame is left out");
    out.clear();
    audio::to_stereo_s16le(&[0u16, 32768, 65535, 32768], 2, &mut out);
    assert_eq!(pairs(&out), [-32768, 0, 32767, 0]);
    out.clear();
    // six channels: front pair plus the centre
    audio::to_stereo_s16le(&[65536i32, 131072, 65_536_000, 0, 0, 0], 6, &mut out);
    assert_eq!(pairs(&out), [708, 709]);
    out.clear();
    audio::to_stereo_s16le(&[9i16], 0, &mut out);
    assert_eq!(pairs(&out), [9, 9]);
    let ms = Duration::from_millis;
    assert_eq!(audio::silence_due(ms(100), 48_000, 0), 0);
    assert_eq!(audio::silence_due(ms(200), 48_000, 0), 0);
    assert_eq!(audio::silence_due(ms(1000), 48_000, 0), 48_000 - 4_800);
    assert_eq!(audio::silence_due(ms(1000), 48_000, 47_000), 0);
    assert_eq!(audio::silence_due(ms(1000), 48_000, 96_000), 0, "ahead");
}

// covers: REQ-357
#[test]
fn the_feeder_serves_one_connection_of_this_machine() {
    let peer = |a: &str| a.parse::<SocketAddr>().unwrap();
    assert!(audio::allowed_peer(&peer("127.0.0.1:5000")));
    for other in ["10.0.0.7:5000", "127.0.0.2:5000", "[::1]:5000"] {
        assert!(!audio::allowed_peer(&peer(other)), "{other}");
    }
    let (tx, rx) = sync_channel(4);
    // captured before ffmpeg connected: not part of the recording
    tx.send(vec![9u8; 4]).unwrap();
    let feeder = Feeder::start(rx, 48_000).unwrap();
    assert_eq!(
        feeder.input(48_000),
        AudioInput::Tcp {
            port: feeder.port,
            rate: 48_000,
            channels: 2
        }
    );
    let mut a = TcpStream::connect(("127.0.0.1", feeder.port)).unwrap();
    a.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    // silence arrives although the source sends nothing
    let mut buf = [1u8; 400];
    a.read_exact(&mut buf).unwrap();
    assert_eq!(buf, [0; 400]);
    tx.send(vec![7u8; 4000]).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        a.read_exact(&mut buf).unwrap();
        if buf.contains(&7) {
            break;
        }
        assert!(Instant::now() < end);
    }
    // the listener is gone: no second connection
    let second = TcpStream::connect(("127.0.0.1", feeder.port));
    assert!(second.is_err());
    drop(a);
    drop(feeder);
    // a feeder nobody connects to ends when dropped
    let (_tx, rx) = sync_channel(1);
    drop(Feeder::start(rx, 48_000).unwrap());
}

/// Gives its bytes a few at a time.
struct Dribble(Cursor<Vec<u8>>);

impl Read for Dribble {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(5);
        self.0.read(&mut buf[..n])
    }
}

// covers: REQ-343
#[test]
fn only_the_newest_preview_picture_is_kept() {
    let feed = Feed::default();
    // three pictures of 2x2 and the start of a fourth
    let mut bytes: Vec<u8> = (1..=3).flat_map(|n| [n; 12]).collect();
    bytes.extend([4; 5]);
    feed.read(Dribble(Cursor::new(bytes)), 2, 2);
    assert_eq!(feed.count(), 3, "all are read, so ffmpeg never waits");
    let f = feed.take().unwrap();
    assert_eq!((f.width, f.height, f.rgb), (2, 2, vec![3; 12]));
    assert_eq!(feed.take(), None);
    feed.read(Cursor::new(vec![0u8; 10]), 0, 2);
    assert_eq!(feed.count(), 3);
    let live = preview::start(Box::new(Cursor::new(vec![5u8; 12])), 2, 2);
    let end = Instant::now() + Duration::from_secs(5);
    while live.count() == 0 {
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
    assert_eq!(live.take().unwrap().rgb, [5; 12]);
}

// covers: REQ-350
#[test]
fn the_cause_of_a_failed_capture_is_said_in_plain_words() {
    let cause = process::failure_cause;
    assert_eq!(
        cause("av_interleaved_write_frame(): No space left on device\n"),
        "The disk is full"
    );
    for gone in [
        "[x11grab] Cannot get the image data event_error: response_type:0",
        "[gdigrab] Failed to capture image (error 5)",
        "Capture area 1920x1080 at position 0.0 outside the screen size",
    ] {
        assert!(cause(gone).contains("screen could not be captured"));
    }
    assert_eq!(cause("file: Permission denied"), "Permission denied");
    assert_eq!(
        cause("one\nlast words\n\n"),
        "The capture failed: last words"
    );
    assert_eq!(cause(""), "The capture failed");
    // only the end of a long error output is kept
    let mut long = vec![b'a'; 20_000];
    long.extend_from_slice(b"the end");
    let kept = process::tail_of(Cursor::new(long));
    assert!(kept.len() == 8192 && kept.ends_with("the end"));
}

// covers: REQ-349, REQ-350
#[cfg(unix)]
#[test]
fn a_real_process_is_asked_to_quit_reports_its_end_and_can_be_killed() {
    // `cat` stands for a process that reads its input: it echoes the quit request
    let mut spawner = SystemSpawner;
    let mut child = spawner.spawn(Path::new("cat"), &[]).unwrap();
    let mut out = child.take_stdout().unwrap();
    assert!(child.take_stdout().is_none());
    assert_eq!(child.try_wait(), None);
    child.quit();
    let mut two = [0u8; 2];
    out.read_exact(&mut two).unwrap();
    assert_eq!(&two, b"q\n");
    child.kill();
    let exit = child.try_wait().unwrap();
    assert!(!exit.success);
    assert_eq!(child.try_wait(), Some(exit));
    child.quit();
    child.kill();
    // an ended one reports success and its errors
    let mut child = spawner
        .spawn(Path::new("cat"), &["/no/such/file".into()])
        .unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    let exit = loop {
        if let Some(e) = child.try_wait() {
            break e;
        }
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(!exit.success && exit.stderr.contains("/no/such/file"));
    assert!(spawner.spawn(Path::new("/no/such/program"), &[]).is_err());
}

// covers: REQ-352
#[test]
fn the_settings_are_kept_in_their_own_file_and_a_damaged_one_is_set_aside() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config/screen-spy.json");
    assert_eq!(RecorderSettings::load(&path), RecorderSettings::default());
    let s = RecorderSettings {
        folder: Some(dir.path().to_path_buf()),
        video: VideoFormat::Uhd,
        audio: AudioFormat::Wav,
    };
    s.save(&path).unwrap();
    assert_eq!(RecorderSettings::load(&path), s);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("\"version\": 1")
    );
    // the default folder is not written down
    RecorderSettings::default().save(&path).unwrap();
    assert_eq!(RecorderSettings::load(&path), RecorderSettings::default());
    let bad = dir.path().join("config/screen-spy.json.bad");
    for damaged in [
        "{ not json".to_string(),
        "[1]".into(),
        "{}".into(),
        r#"{"version": 2}"#.into(),
        r#"{"version": "1"}"#.into(),
        " ".repeat(settings::MAX_BYTES as usize + 1),
    ] {
        std::fs::write(&path, &damaged).unwrap();
        assert_eq!(RecorderSettings::load(&path), RecorderSettings::default());
        assert!(!path.exists(), "set aside");
        assert_eq!(std::fs::read_to_string(&bad).unwrap(), damaged);
    }
    // every value is checked by itself
    let long = format!("/{}", "x".repeat(settings::MAX_FOLDER));
    for folder in ["relative", "/a\u{7}b", long.as_str()] {
        let json = serde_json::json!({"version": 1, "folder": folder, "video": "8k", "audio": 3});
        let s = RecorderSettings::parse(json.to_string().as_bytes()).unwrap();
        assert_eq!(s, RecorderSettings::default(), "{folder}");
    }
    // a folder in the file's place is left alone
    let folder = dir.path().join("config");
    assert_eq!(RecorderSettings::load(&folder), RecorderSettings::default());
    assert!(folder.is_dir());
    assert!(RecorderSettings::default().save(&folder).is_err());
    if let Some(p) = RecorderSettings::default_path() {
        assert!(p.ends_with("screen-spy.json"));
    }
}

// covers: REQ-342
#[test]
fn the_about_text_names_only_the_recorders_components() {
    let names: Vec<&str> = crate::about::bundled().iter().map(|c| c.name).collect();
    assert_eq!(
        names,
        ["FFmpeg", "x264", "x265", "libass", "Phosphor Icons"]
    );
    let text = crate::about::text();
    assert!(text.starts_with("videolore screen spy "));
    for gone in ["whisper", "ggml", "Noto Sans", "splash", "videolore player"] {
        assert!(!text.contains(gone), "{gone}");
    }
    for kept in [
        "FFmpeg",
        "Phosphor Icons",
        "rust-crates-screen-spy.txt",
        "display-info",
    ] {
        assert!(text.contains(kept), "{kept}");
    }
    assert!(!crate::about::RUST_CRATES.contains("ureq "));
}

// covers: REQ-341
#[test]
fn the_recorder_is_a_program_of_its_own_with_installers_and_no_network_code() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("name = \"videolore-screen-spy\""));
    assert!(manifest.contains("videolore = { path = \"..\", default-features = false }"));
    for net in ["ureq", "whisper", "rustls", "videolore-player"] {
        assert!(!manifest.contains(net), "{net}");
    }
    let package = include_str!("../../scripts/package.sh");
    for line in [
        "pname=videolore-screen-spy",
        "pident=io.github.andrealorenzani.videolore-screen-spy",
        "rust-crates-screen-spy.txt",
        "third-party-screen-spy.txt",
        "NSMicrophoneUsageDescription",
        "NSAudioCaptureUsageDescription",
        "minimum-system-version = \"11.0\"",
    ] {
        let found = package.contains(line)
            || include_str!("../../packaging/macos/screen-spy-info.plist").contains(line);
        assert!(found, "{line}");
    }
    let build = include_str!("../../.github/workflows/build.yml");
    assert!(build.contains("scripts/package.sh ${{ matrix.target }} screen-spy"));
    assert!(build.contains("for name in videolore videolore-player videolore-screen-spy"));
    assert!(build.contains("cargo tree --locked -p \"$program\""));
    let options = crate::native_options();
    assert_eq!(
        options.viewport.title.as_deref(),
        Some("videolore screen spy")
    );
}

/// A microphone permission that answers as told.
struct Asked {
    status: MicrophonePermission,
    /// The answer given to the question; `None` leaves it open.
    answer: Option<bool>,
    questions: AtomicUsize,
    open: Mutex<Vec<Sender<bool>>>,
}

impl Asked {
    fn new(status: MicrophonePermission, answer: Option<bool>) -> Asked {
        Asked {
            status,
            answer,
            questions: AtomicUsize::new(0),
            open: Mutex::default(),
        }
    }
}

impl MicrophoneAccess for Asked {
    fn status(&self) -> MicrophonePermission {
        self.status
    }

    fn ask(&self, answer: Sender<bool>) {
        self.questions.fetch_add(1, Ordering::SeqCst);
        match self.answer {
            Some(a) => answer.send(a).unwrap(),
            None => self.open.lock().unwrap().push(answer),
        }
    }
}

// covers: REQ-357
#[test]
fn a_microphone_permission_not_asked_yet_is_answered_before_anything_is_recorded() {
    use MicrophonePermission as P;
    for (status, want) in [
        (0, P::NotAsked),
        (1, P::Denied),
        (2, P::Denied),
        (3, P::Allowed),
    ] {
        assert_eq!(P::of_status(status), want);
    }
    assert_eq!(P::of_status(9), P::Denied);
    let go = AtomicBool::new(false);
    let long = Duration::from_secs(10);
    let gate = |a: &Asked| microphone_gate(a, long, &go);
    // known already: nothing is asked
    let allowed = Asked::new(P::Allowed, None);
    assert_eq!(gate(&allowed), Ok(()));
    let denied = Asked::new(P::Denied, None);
    assert_eq!(gate(&denied), Err(session::MICROPHONE_NOT_ALLOWED.into()));
    assert_eq!(allowed.questions.load(Ordering::SeqCst), 0);
    assert_eq!(denied.questions.load(Ordering::SeqCst), 0);
    // not asked: the answer decides
    let yes = Asked::new(P::NotAsked, Some(true));
    assert_eq!(gate(&yes), Ok(()));
    assert_eq!(yes.questions.load(Ordering::SeqCst), 1);
    let no = Asked::new(P::NotAsked, Some(false));
    assert_eq!(gate(&no), Err(session::MICROPHONE_NOT_ALLOWED.into()));
    // no answer in time, a cancelled start, a question that is dropped: never a pass
    let silent = Err(session::MICROPHONE_NO_ANSWER.to_string());
    let open = Asked::new(P::NotAsked, None);
    let before = Instant::now();
    assert_eq!(
        microphone_gate(&open, Duration::from_millis(120), &go),
        silent
    );
    assert!(before.elapsed() >= Duration::from_millis(120));
    assert_eq!(microphone_gate(&open, long, &AtomicBool::new(true)), silent);
    struct Dropped;
    impl MicrophoneAccess for Dropped {
        fn status(&self) -> MicrophonePermission {
            MicrophonePermission::NotAsked
        }
        fn ask(&self, _: Sender<bool>) {}
    }
    assert_eq!(microphone_gate(&Dropped, long, &go), silent);
}

/// Notes the thread it is dropped on.
struct Stream(Sender<std::thread::ThreadId>);

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.0.send(std::thread::current().id());
    }
}

// covers: REQ-357, REQ-349
#[test]
fn sound_streams_stay_on_the_thread_that_opened_them_until_the_session_ends() {
    let (dropped, on) = std::sync::mpsc::channel();
    let (opened, at) = std::sync::mpsc::channel();
    let inputs = vec![AudioInput::Pulse(PulseSource::Computer)];
    let given = inputs.clone();
    let session = audio::hold(move || {
        opened.send(std::thread::current().id()).unwrap();
        // a value that cannot change threads, as a sound stream
        Ok((given, (std::rc::Rc::new(()), Stream(dropped))))
    })
    .unwrap();
    assert_eq!(session.inputs, inputs);
    let thread = at.recv().unwrap();
    assert_ne!(thread, std::thread::current().id());
    assert!(on.try_recv().is_err(), "open while the session lives");
    drop(session);
    // closed when the drop returns, on the same thread
    assert_eq!(on.try_recv().unwrap(), thread);
    let e = audio::hold::<()>(|| Err("no device".into())).err().unwrap();
    assert_eq!(e, "no device");
    let e = audio::hold::<()>(|| panic!("broken driver")).err().unwrap();
    assert_eq!(e, "The sound could not be opened");
}

// covers: REQ-377
#[test]
fn the_mixed_track_has_its_noise_reduced_before_the_limiter_on_every_system() {
    for os in [Os::Linux, Os::Windows, Os::MacOs] {
        let a = record(os, VideoFormat::Hd1080, AudioFormat::Aac, false);
        let graph = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        // after the mix, then its 25 ms delay cut off, then the limiter
        assert!(
            graph.ends_with(
                "normalize=0,afftdn=nr=25:nf=-20:tn=0:rf=-60,atrim=start_sample=1200,\
                 asetpts=PTS-STARTPTS,alimiter=level=false[a]"
            ),
            "{graph}"
        );
        // the picture is not touched
        let picture = graph.split(";[0:a]").next().unwrap();
        assert!(!picture.contains("afftdn") && !picture.contains("setpts"));
    }
}
