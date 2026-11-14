// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Imported media: [`MediaItem`], [`MediaFormat`] and a defensive ffprobe JSON parser.

use crate::ffmpeg::{Runner, argv, file_arg};
use crate::time::{Fps, Micros};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Identifier of a media item inside a project.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct MediaId(pub u64);

/// Technical description of a media file (what export decisions are based on).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MediaFormat {
    /// Normalised container: `mp4`, `mov`, `matroska`, `webm`, or ffprobe's first name.
    pub container: String,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub width: u32,
    pub height: u32,
    pub fps: Option<Fps>,
    /// Video bitrate in bit/s (stream value, or the container value as a fallback).
    pub bit_rate: Option<u64>,
    pub audio_bit_rate: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

impl MediaFormat {
    pub fn has_video(&self) -> bool {
        self.vcodec.is_some()
    }

    pub fn has_audio(&self) -> bool {
        self.acodec.is_some()
    }

    /// The fields that must match for two files to count as "the same format".
    pub fn identity(&self) -> (&str, Option<&str>, Option<&str>, u32, u32, Option<Fps>) {
        (
            &self.container,
            self.vcodec.as_deref(),
            self.acodec.as_deref(),
            self.width,
            self.height,
            self.fps,
        )
    }

    pub fn pixels(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Short human description, e.g. `1280x720 30 fps h264/aac mp4`.
    pub fn describe(&self) -> String {
        let mut s = String::new();
        if self.has_video() {
            s.push_str(&format!("{}x{}", self.width, self.height));
            if let Some(f) = self.fps {
                s.push_str(&format!(" {f} fps"));
            }
            s.push(' ');
        }
        s.push_str(self.vcodec.as_deref().unwrap_or("-"));
        s.push('/');
        s.push_str(self.acodec.as_deref().unwrap_or("-"));
        s.push(' ');
        s.push_str(&self.container);
        s
    }
}

/// Where an imported file is in its analysis.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MediaStatus {
    /// ffprobe is running (or queued) on it.
    Analysing,
    /// Probed; duration and format are known.
    #[default]
    Ready,
    /// Not a usable video or audio file; the reason is shown to the user.
    Rejected(String),
}

/// The low-resolution preview copy of a big source (never used for export).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyState {
    /// Not needed, disabled, or not started.
    #[default]
    None,
    /// Waiting for a free ffmpeg slot.
    Queued,
    /// Being generated; fraction done.
    Generating(u8),
    /// Ready at this path (in the proxy cache).
    Ready(PathBuf),
    /// Generation failed; the original is used.
    Failed(String),
}

/// What a file can be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Has a video stream: goes on the video track.
    Video,
    /// Audio only: goes on the music track.
    Audio,
}

/// A file imported into the project. Never modified by the editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: MediaId,
    /// Absolute path of the source file.
    pub path: PathBuf,
    pub name: String,
    pub duration: Micros,
    pub format: MediaFormat,
    /// Set when a reopened project cannot find the file.
    #[serde(skip)]
    pub missing: bool,
    #[serde(skip)]
    pub status: MediaStatus,
    #[serde(skip)]
    pub proxy: ProxyState,
}

impl MediaItem {
    /// A list entry for a file that is still being analysed.
    pub fn analysing(id: MediaId, path: &Path) -> MediaItem {
        MediaItem {
            id,
            path: std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()),
            name: display_name(path),
            duration: Micros::ZERO,
            format: MediaFormat::default(),
            missing: false,
            status: MediaStatus::Analysing,
            proxy: ProxyState::None,
        }
    }

    pub fn has_video(&self) -> bool {
        self.format.has_video()
    }

    pub fn is_ready(&self) -> bool {
        self.status == MediaStatus::Ready
    }

    /// Video track or music track.
    pub fn kind(&self) -> MediaKind {
        if self.has_video() {
            MediaKind::Video
        } else {
            MediaKind::Audio
        }
    }

    /// The proxy file, when ready.
    pub fn proxy_path(&self) -> Option<&Path> {
        match &self.proxy {
            ProxyState::Ready(p) => Some(p),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MediaError {
    #[error("'{0}' does not exist or is not a file")]
    NotAFile(String),
    #[error("'{0}' is not a video or audio file that ffmpeg can read")]
    Unreadable(String),
    #[error("'{0}' has no video or audio stream")]
    NoStreams(String),
    #[error("'{0}' has no usable duration")]
    NoDuration(String),
    #[error("could not run ffprobe: {0}")]
    Ffprobe(String),
}

fn num_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn num_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Normalises ffprobe's `format_name` (e.g. `mov,mp4,m4a,3gp,3g2,mj2`) using the extension.
pub fn normalize_container(format_name: &str, path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let names: Vec<&str> = format_name.split(',').map(str::trim).collect();
    if names.contains(&"mp4") || names.contains(&"mov") {
        return if ext == "mov" { "mov" } else { "mp4" }.into();
    }
    if names.contains(&"matroska") || names.contains(&"webm") {
        return if ext == "webm" { "webm" } else { "matroska" }.into();
    }
    names.first().copied().unwrap_or("").to_string()
}

/// Parses `ffprobe -print_format json -show_format -show_streams` output.
/// Returns the duration and the format. Ignores cover-art pictures.
pub fn parse_ffprobe(json: &str, path: &Path) -> Result<(Micros, MediaFormat), MediaError> {
    let name = display_name(path);
    let v: Value = serde_json::from_str(json).map_err(|_| MediaError::Unreadable(name.clone()))?;
    let format = v
        .get("format")
        .ok_or(MediaError::Unreadable(name.clone()))?;
    let streams = v
        .get("streams")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let is_pic = |s: &Value| {
        s.get("disposition")
            .and_then(|d| d.get("attached_pic"))
            .and_then(num_u64)
            == Some(1)
    };
    let video = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("video") && !is_pic(s));
    let audio = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("audio"));
    if video.is_none() && audio.is_none() {
        return Err(MediaError::NoStreams(name));
    }
    let codec = |s: Option<&Value>| {
        s.and_then(|s| s.get("codec_name"))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    let mut f = MediaFormat {
        container: normalize_container(
            format
                .get("format_name")
                .and_then(Value::as_str)
                .unwrap_or(""),
            path,
        ),
        vcodec: codec(video),
        acodec: codec(audio),
        ..MediaFormat::default()
    };
    if let Some(s) = video {
        f.width = s.get("width").and_then(num_u64).unwrap_or(0) as u32;
        f.height = s.get("height").and_then(num_u64).unwrap_or(0) as u32;
        if f.width == 0 || f.height == 0 {
            return Err(MediaError::Unreadable(name));
        }
        f.fps = ["avg_frame_rate", "r_frame_rate"]
            .iter()
            .filter_map(|k| s.get(*k).and_then(Value::as_str).and_then(Fps::parse))
            .find(|r| r.as_f64() > 0.0 && r.as_f64() <= 1000.0);
        f.bit_rate = s
            .get("bit_rate")
            .and_then(num_u64)
            .or_else(|| format.get("bit_rate").and_then(num_u64));
    }
    if let Some(s) = audio {
        f.audio_bit_rate = s.get("bit_rate").and_then(num_u64);
        f.sample_rate = s.get("sample_rate").and_then(num_u64).map(|n| n as u32);
        f.channels = s.get("channels").and_then(num_u64).map(|n| n as u32);
    }
    let duration = format
        .get("duration")
        .and_then(num_f64)
        .or_else(|| {
            video
                .or(audio)
                .and_then(|s| s.get("duration"))
                .and_then(num_f64)
        })
        .filter(|d| d.is_finite() && *d > 0.0)
        .ok_or(MediaError::NoDuration(name))?;
    if f.has_video() && f.fps.is_none() {
        f.fps = Fps::new(30, 1);
    }
    Ok((Micros::from_secs_f64(duration), f))
}

/// The file name shown to users.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Runs ffprobe on `path` and builds a [`MediaItem`].
pub fn probe(
    runner: &dyn Runner,
    ffprobe: &Path,
    path: &Path,
    id: MediaId,
) -> Result<MediaItem, MediaError> {
    let name = display_name(path);
    if !path.is_file() {
        return Err(MediaError::NotAFile(name));
    }
    let abs = std::path::absolute(path).map_err(|_| MediaError::NotAFile(name.clone()))?;
    let mut args = argv([
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
    ]);
    args.push(file_arg(&abs));
    let out = runner
        .run(ffprobe, &args)
        .map_err(|e| MediaError::Ffprobe(e.to_string()))?;
    if !out.success {
        return Err(MediaError::Unreadable(name));
    }
    let (duration, format) = parse_ffprobe(&String::from_utf8_lossy(&out.stdout), &abs)?;
    Ok(MediaItem {
        id,
        path: abs,
        name,
        duration,
        format,
        missing: false,
        status: MediaStatus::Ready,
        proxy: ProxyState::None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ffmpeg::fake::{FakeRunner, fail, ok};

    pub const MP4_JSON: &str = r#"{
      "streams": [
        {"codec_type": "video", "codec_name": "h264", "width": 1280, "height": 720,
         "avg_frame_rate": "30/1", "r_frame_rate": "30/1", "bit_rate": "2000000"},
        {"codec_type": "audio", "codec_name": "aac", "sample_rate": "48000", "channels": 2,
         "bit_rate": "128000"}
      ],
      "format": {"format_name": "mov,mp4,m4a,3gp,3g2,mj2", "duration": "5.000000", "bit_rate": "2200000"}
    }"#;

    #[test]
    fn parses_video_with_audio() {
        let (d, f) = parse_ffprobe(MP4_JSON, Path::new("/a/clip.mp4")).unwrap();
        assert_eq!(d, Micros(5_000_000));
        assert_eq!(f.container, "mp4");
        assert_eq!(f.vcodec.as_deref(), Some("h264"));
        assert_eq!(f.acodec.as_deref(), Some("aac"));
        assert_eq!((f.width, f.height), (1280, 720));
        assert_eq!(f.fps, Fps::new(30, 1));
        assert_eq!(f.bit_rate, Some(2_000_000));
        assert_eq!(f.sample_rate, Some(48_000));
        assert_eq!(f.channels, Some(2));
        assert_eq!(f.pixels(), 921_600);
        assert_eq!(f.describe(), "1280x720 30 fps h264/aac mp4");
    }

    // covers: REQ-002
    #[test]
    fn audio_only_and_cover_art() {
        let j = r#"{"streams":[{"codec_type":"video","codec_name":"mjpeg","width":10,"height":10,
            "disposition":{"attached_pic":1}},
            {"codec_type":"audio","codec_name":"mp3","duration":"3.5"}],
            "format":{"format_name":"mp3"}}"#;
        let (d, f) = parse_ffprobe(j, Path::new("song.mp3")).unwrap();
        assert_eq!(d, Micros(3_500_000));
        assert!(!f.has_video());
        assert!(f.has_audio());
        assert_eq!(f.container, "mp3");
        assert_eq!(f.describe(), "-/mp3 mp3");
    }

    // covers: REQ-002
    #[test]
    fn rejects_garbage_defensively() {
        let p = Path::new("x.txt");
        assert_eq!(
            parse_ffprobe("not json", p),
            Err(MediaError::Unreadable("x.txt".into()))
        );
        assert_eq!(
            parse_ffprobe("{}", p),
            Err(MediaError::Unreadable("x.txt".into()))
        );
        assert_eq!(
            parse_ffprobe(r#"{"format":{},"streams":[]}"#, p),
            Err(MediaError::NoStreams("x.txt".into()))
        );
        assert_eq!(
            parse_ffprobe(
                r#"{"format":{"duration":"N/A"},"streams":[{"codec_type":"audio","codec_name":"aac"}]}"#,
                p
            ),
            Err(MediaError::NoDuration("x.txt".into()))
        );
        assert_eq!(
            parse_ffprobe(
                r#"{"format":{"duration":"1"},"streams":[{"codec_type":"video","codec_name":"h264"}]}"#,
                p
            ),
            Err(MediaError::Unreadable("x.txt".into()))
        );
    }

    #[test]
    fn fps_fallback_and_containers() {
        let j = r#"{"format":{"duration":2,"format_name":"matroska,webm","bit_rate":900},
            "streams":[{"codec_type":"video","codec_name":"vp9","width":640,"height":360,
            "avg_frame_rate":"0/0","r_frame_rate":"0/0"}]}"#;
        let (_, f) = parse_ffprobe(j, Path::new("a.webm")).unwrap();
        assert_eq!(f.fps, Fps::new(30, 1));
        assert_eq!(f.container, "webm");
        assert_eq!(f.bit_rate, Some(900));
        assert_eq!(
            normalize_container("matroska,webm", Path::new("a.mkv")),
            "matroska"
        );
        assert_eq!(normalize_container("mov,mp4", Path::new("a.MOV")), "mov");
        assert_eq!(normalize_container("avi", Path::new("a.avi")), "avi");
        assert_eq!(display_name(Path::new("/")), "/");
    }

    // covers: REQ-002
    #[test]
    fn probe_rejects_missing_and_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let r = FakeRunner::default().on(|_, _| fail("Invalid data"));
        let missing = dir.path().join("nope.mp4");
        assert_eq!(
            probe(&r, Path::new("ffprobe"), &missing, MediaId(1)),
            Err(MediaError::NotAFile("nope.mp4".into()))
        );
        let txt = dir.path().join("notes.txt");
        std::fs::write(&txt, "hello").unwrap();
        let e = probe(&r, Path::new("ffprobe"), &txt, MediaId(1)).unwrap_err();
        assert_eq!(e, MediaError::Unreadable("notes.txt".into()));
        assert!(e.to_string().contains("notes.txt"));
        let none = FakeRunner::default();
        assert!(matches!(
            probe(&none, Path::new("ffprobe"), &txt, MediaId(1)),
            Err(MediaError::Ffprobe(_))
        ));
        let good = FakeRunner::default().on(|_, _| ok(MP4_JSON));
        let m = probe(&good, Path::new("ffprobe"), &txt, MediaId(7)).unwrap();
        assert_eq!(m.id, MediaId(7));
        assert_eq!(m.name, "notes.txt");
        assert!(m.path.is_absolute());
        assert!(m.has_video());
        assert_eq!(m.kind(), MediaKind::Video);
        assert!(m.is_ready());
        assert!(m.proxy_path().is_none());
        let a = MediaItem::analysing(MediaId(3), Path::new("x/clip.mov"));
        assert_eq!(a.status, MediaStatus::Analysing);
        assert_eq!(a.name, "clip.mov");
        assert!(a.path.is_absolute());
        assert_eq!(a.kind(), MediaKind::Audio);
    }
}
