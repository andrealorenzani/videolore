// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! What ffmpeg is asked to do, as argument lists, for the three systems (ADR-256, 257,
//! 261, 262). Everything here is pure: the system, the screen and the sound sources are
//! values, so the commands of all three systems are checked on any machine.
//!
//! Every argument is a number, a name from a closed list, the screen's geometry, the
//! checked X11 display name, or the file the user chose.

use crate::devices::avfoundation_screen;
use std::ffi::OsString;
use std::path::Path;
use videolore::ffmpeg::file_arg;

/// The preview's frame rate (REQ-343).
pub const PREVIEW_FPS: u32 = 5;
/// The preview's width in pixels.
pub const PREVIEW_WIDTH: u32 = 640;
/// The sound of the file.
pub const SAMPLE_RATE: u32 = 48_000;
/// Every recording's name starts like this (REQ-345).
pub const NAME_PREFIX: &str = "screen-recording";
/// The longest file name accepted, in bytes (with the extension).
pub const MAX_NAME: usize = 200;

/// The system the recorder runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
    MacOs,
}

impl Os {
    /// The system this program was built for.
    pub fn current() -> Os {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// The four video formats (REQ-347).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoFormat {
    Hd720,
    #[default]
    Hd1080,
    Hd1080Fast,
    Uhd,
}

impl VideoFormat {
    pub const ALL: [VideoFormat; 4] = [Self::Hd720, Self::Hd1080, Self::Hd1080Fast, Self::Uhd];

    /// Width and height of the picture.
    pub fn size(self) -> (u32, u32) {
        match self {
            Self::Hd720 => (1280, 720),
            Self::Hd1080 | Self::Hd1080Fast => (1920, 1080),
            Self::Uhd => (3840, 2160),
        }
    }

    /// Frames per second.
    pub fn fps(self) -> u32 {
        match self {
            Self::Hd1080Fast => 60,
            _ => 30,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hd720 => "1280x720, 30 fps",
            Self::Hd1080 => "1920x1080, 30 fps",
            Self::Hd1080Fast => "1920x1080, 60 fps",
            Self::Uhd => "4K UHD 2160p (3840x2160), 30 fps",
        }
    }

    /// The name in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Hd720 => "720p30",
            Self::Hd1080 => "1080p30",
            Self::Hd1080Fast => "1080p60",
            Self::Uhd => "2160p30",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }
}

/// The three sound formats (REQ-348).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioFormat {
    #[default]
    Aac,
    Mp3,
    Wav,
}

impl AudioFormat {
    pub const ALL: [AudioFormat; 3] = [Self::Aac, Self::Mp3, Self::Wav];

    pub fn label(self) -> &'static str {
        match self {
            Self::Aac => "AAC",
            Self::Mp3 => "MP3",
            Self::Wav => "WAV",
        }
    }

    /// The name in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// The extension of the video file: WAV sound cannot go into MP4, so it goes into
    /// MKV (REQ-348).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Aac | Self::Mp3 => "mp4",
            Self::Wav => "mkv",
        }
    }

    /// The ffmpeg encoder.
    pub fn encoder(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Mp3 => "libmp3lame",
            Self::Wav => "pcm_s16le",
        }
    }
}

/// The screen to capture, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capture {
    /// Its position in the system's list of screens, from 0.
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// One of the two sound sources, as ffmpeg reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioInput {
    /// A PulseAudio source (Linux), by one of the two fixed names.
    Pulse(PulseSource),
    /// Raw 16 bit samples sent by this program over a connection on this machine.
    Tcp { port: u16, rate: u32, channels: u16 },
}

/// The noise reduction of the recording (REQ-377, ADR-283): ffmpeg's FFT denoiser, the
/// values of the editor's Clean audio, checked on a synthetic fixture (`tests/ffmpeg.rs`).
pub const NOISE_REDUCTION: &str = "afftdn=nr=25:nf=-20:tn=0:rf=-60";
/// How much later the sound leaves [`NOISE_REDUCTION`] than it came in, in samples at
/// [`SAMPLE_RATE`]: two hops of 12.5 ms, 25 ms (measured by the same test).
pub const NOISE_DELAY_SAMPLES: u32 = SAMPLE_RATE / 40;

/// The filters of the mixed track: the noise reduction, its delay cut off so that the
/// sound stays with the picture, then the limiter.
pub fn mix_filters() -> String {
    format!(
        "{NOISE_REDUCTION},atrim=start_sample={NOISE_DELAY_SAMPLES},asetpts=PTS-STARTPTS,\
         alimiter=level=false"
    )
}

/// The two PulseAudio sources (ADR-259).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseSource {
    /// What the computer plays: the monitor of the default output.
    Computer,
    /// The default input.
    Microphone,
}

impl PulseSource {
    pub fn device(self) -> &'static str {
        match self {
            Self::Computer => "@DEFAULT_MONITOR@",
            Self::Microphone => "@DEFAULT_SOURCE@",
        }
    }
}

fn push(args: &mut Vec<OsString>, items: &[&str]) {
    args.extend(items.iter().map(OsString::from));
}

/// The input of the screen, at `fps` frames per second. `display` is the X11 display name
/// (used on Linux only).
pub fn screen_input(os: Os, screen: &Capture, fps: u32, display: &str) -> Vec<OsString> {
    let fps = fps.to_string();
    let size = format!("{}x{}", screen.width, screen.height);
    let mut a = Vec::new();
    push(&mut a, &["-thread_queue_size", "512"]);
    match os {
        Os::Windows => {
            let (x, y) = (screen.x.to_string(), screen.y.to_string());
            push(&mut a, &["-f", "gdigrab", "-framerate", &fps]);
            push(&mut a, &["-offset_x", &x, "-offset_y", &y]);
            push(&mut a, &["-video_size", &size, "-i", "desktop"]);
        }
        Os::MacOs => {
            let input = format!("{}:none", avfoundation_screen(screen.index));
            push(&mut a, &["-f", "avfoundation", "-capture_cursor", "1"]);
            push(&mut a, &["-framerate", &fps, "-i", &input]);
        }
        Os::Linux => {
            let input = format!("{display}+{},{}", screen.x, screen.y);
            push(&mut a, &["-f", "x11grab", "-framerate", &fps]);
            push(&mut a, &["-video_size", &size, "-i", &input]);
        }
    }
    a
}

/// The input of one sound source.
pub fn sound_input(source: &AudioInput) -> Vec<OsString> {
    let mut a = Vec::new();
    push(&mut a, &["-thread_queue_size", "1024"]);
    match source {
        AudioInput::Pulse(p) => push(&mut a, &["-f", "pulse", "-i", p.device()]),
        AudioInput::Tcp {
            port,
            rate,
            channels,
        } => {
            let (rate, channels) = (rate.to_string(), channels.to_string());
            let url = format!("tcp://127.0.0.1:{port}");
            push(&mut a, &["-protocol_whitelist", "tcp"]);
            push(&mut a, &["-analyzeduration", "0", "-probesize", "32"]);
            push(&mut a, &["-f", "s16le", "-ar", &rate, "-ac", &channels]);
            push(&mut a, &["-i", &url]);
        }
    }
    a
}

/// The size of the preview picture of a screen: [`PREVIEW_WIDTH`] wide, the screen's
/// shape, an even height.
pub fn preview_size(width: u32, height: u32) -> (u32, u32) {
    let w = u64::from(width.max(1));
    let h = (u64::from(PREVIEW_WIDTH) * u64::from(height) + w) / (2 * w) * 2;
    (PREVIEW_WIDTH, h.clamp(2, 2160) as u32)
}

fn preview_filter(screen: &Capture) -> String {
    let (w, h) = preview_size(screen.width, screen.height);
    format!("fps={PREVIEW_FPS},scale={w}:{h},format=rgb24")
}

fn preview_output(args: &mut Vec<OsString>) {
    push(args, &["-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"]);
}

/// The ffmpeg run that only shows the screen (before and between recordings).
pub fn preview_args(os: Os, screen: &Capture, display: &str) -> Vec<OsString> {
    let mut a = Vec::new();
    push(&mut a, &["-hide_banner", "-loglevel", "error", "-nostdin"]);
    a.extend(screen_input(os, screen, PREVIEW_FPS, display));
    let filter = preview_filter(screen);
    push(&mut a, &["-an", "-vf", &filter]);
    preview_output(&mut a);
    a
}

/// One recording.
#[derive(Debug, Clone, Copy)]
pub struct Recording<'a> {
    pub os: Os,
    pub screen: Capture,
    pub display: &'a str,
    pub video: VideoFormat,
    pub audio: AudioFormat,
    /// The sources mixed into the one sound track (REQ-357).
    pub sources: &'a [AudioInput],
    pub output: &'a Path,
    /// True only after the user chose Replace (REQ-346).
    pub overwrite: bool,
}

/// The graph: the screen (the input after the sources) split into the file's picture and
/// the preview, and the sources mixed into one track, its noise reduced, that starts with
/// the picture.
pub fn filter_graph(screen: &Capture, video: VideoFormat, sources: usize) -> String {
    let (w, h) = video.size();
    let mut g = format!(
        "[{sources}:v]split=2[rec][pv];[rec]scale={w}:{h}:force_original_aspect_ratio=decrease,\
         pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v];\
         [pv]{preview}[p]",
        fps = video.fps(),
        preview = preview_filter(screen),
    );
    for i in 0..sources {
        g.push_str(&format!(
            ";[{i}:a]aresample={SAMPLE_RATE}:async=1:first_pts=0,\
             aformat=channel_layouts=stereo[a{i}]"
        ));
    }
    g.push(';');
    for i in 0..sources {
        g.push_str(&format!("[a{i}]"));
    }
    g.push_str(&format!(
        "amix=inputs={sources}:duration=longest:normalize=0,{}[a]",
        mix_filters()
    ));
    g
}

/// The ffmpeg run that records (and shows the preview). It is the only one that reads its
/// standard input: `q` ends the file (ADR-261).
pub fn record_args(r: &Recording) -> Vec<OsString> {
    let mut a = Vec::new();
    push(&mut a, &["-hide_banner", "-loglevel", "error"]);
    // The sources are opened first and the screen last (ffmpeg opens its inputs one after
    // the other and counts each from its own start): the file begins with the picture,
    // with the sound already running. The PulseAudio inputs and x11grab stamp the same
    // clock, so those are also shifted by their real distance to the screen's start.
    let screen_index = r.sources.len().to_string();
    for s in r.sources {
        if matches!(s, AudioInput::Pulse(_)) {
            push(&mut a, &["-isync", &screen_index]);
        }
        a.extend(sound_input(s));
    }
    a.extend(screen_input(r.os, &r.screen, r.video.fps(), r.display));
    let graph = filter_graph(&r.screen, r.video, r.sources.len());
    push(&mut a, &["-filter_complex", &graph]);
    push(&mut a, &["-map", "[v]", "-map", "[a]"]);
    // a key frame every second: a file cut short loses at most about the last second
    let gop = r.video.fps().to_string();
    push(&mut a, &["-c:v", "libx264", "-preset", "veryfast"]);
    push(&mut a, &["-crf", "23", "-g", &gop]);
    push(
        &mut a,
        &["-c:a", r.audio.encoder(), "-ar", "48000", "-ac", "2"],
    );
    if r.audio != AudioFormat::Wav {
        push(&mut a, &["-b:a", "160k"]);
    }
    match r.audio.extension() {
        "mp4" => push(
            &mut a,
            &[
                "-movflags",
                "frag_keyframe+empty_moov+default_base_moof",
                "-f",
                "mp4",
            ],
        ),
        _ => push(&mut a, &["-cluster_time_limit", "1000", "-f", "matroska"]),
    }
    push(&mut a, &["-flush_packets", "1"]);
    push(&mut a, &[if r.overwrite { "-y" } else { "-n" }]);
    a.push(file_arg(r.output));
    push(&mut a, &["-map", "[p]"]);
    preview_output(&mut a);
    a
}

/// The name offered for a recording started at `stamp` (REQ-345).
pub fn default_name(stamp: &str, audio: AudioFormat) -> String {
    format!("{NAME_PREFIX}-{stamp}.{}", audio.extension())
}

/// `name` without a `.mp4` or `.mkv` ending.
fn stem(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((s, e)) if e.eq_ignore_ascii_case("mp4") || e.eq_ignore_ascii_case("mkv") => s,
        _ => name,
    }
}

/// The name with the extension the sound format needs.
pub fn with_extension(name: &str, audio: AudioFormat) -> String {
    format!("{}.{}", stem(name), audio.extension())
}

/// The file name to record to, from what is typed in the box: the extension follows the
/// sound format; a name that is empty, too long or names a folder is refused.
pub fn file_name(typed: &str, audio: AudioFormat) -> Result<String, String> {
    let typed = typed.trim();
    let base = stem(typed);
    if base.is_empty() || base.chars().all(|c| c == '.') {
        return Err("Type a file name".into());
    }
    if typed
        .chars()
        .any(|c| c == '/' || c == '\\' || c.is_control())
    {
        return Err("The file name cannot hold a folder: choose the folder with Choose…".into());
    }
    let name = with_extension(typed, audio);
    if name.len() > MAX_NAME {
        return Err("The file name is too long".into());
    }
    Ok(name)
}
