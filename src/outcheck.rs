// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The output check of a hardware export (REQ-063): the file must be what the normal method
//! would have made, or the export is redone with the normal method.
//!
//! All steps use software tools on the finished hardware file:
//! 1. metadata ([`check_metadata`], ffprobe JSON): codec, profile, 8-bit 4:2:0, size,
//!    frame rate, square pixels, `hvc1` tag, audio, packet count and duration;
//! 2. a full, strict decode ([`decode_args`]): no error, every packet gives a frame;
//! 3. one pass ([`compare_args`]) that renders the normal method's video graph (no
//!    encoding) next to the hardware file: the graph's frame count and duration must match
//!    the file's, and 3 samples (10%, 50%, 90%, each with its neighbours) are compared frame by frame
//!    with ffmpeg's `psnr` filter: each sample needs [`MIN_PSNR`] dB on Y, U and V.
//!
//! Frames are matched by their position in the stream: the hardware encoder received
//! exactly the frames of the graph, in order, so no timing model is needed (clip lengths
//! that are not whole frames, variable frame rate sources and many cuts stay exact).
//! This module builds the commands and judges their output; running them is the export's
//! job ([`crate::export`]).

use crate::ffmpeg::{FfVersion, argv, file_arg};
use crate::hwenc::Codec;
use crate::time::Fps;
use serde_json::Value;
use std::ffi::OsString;
use std::path::Path;

/// Lowest PSNR (dB) each plane of a sample may have. Good encodes at the source bitrate
/// score about 33–45 dB; green, garbage, wrong-size or swapped-colour frames score below
/// 15–20 dB.
pub const MIN_PSNR: f64 = 25.0;

/// The ffmpeg filter the check needs (without it the hardware is not used).
pub const CHECK_FILTER: &str = "psnr";

/// Where the samples are taken, as fractions of the frame count (percent).
pub const SAMPLE_PERCENT: [u64; 3] = [10, 50, 90];

/// Why a hardware file was refused.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CheckFailure {
    #[error("the file does not decode cleanly")]
    Decode,
    #[error("{got} frames instead of {expected}")]
    FrameCount { expected: u64, got: u64 },
    #[error("wrong {0}")]
    Metadata(&'static str),
    #[error("sample {index} looks different ({psnr:.1} dB)")]
    Picture { index: usize, psnr: f64 },
    /// Constant quality made a file much bigger than the normal method would.
    #[error("the file is {0:.1} times the usual size")]
    TooLarge(f64),
}

/// The planned audio stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedAudio {
    /// ffprobe codec name (`aac`, `opus`, `mp3`, …).
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
}

/// What the export must contain.
#[derive(Debug, Clone, PartialEq)]
pub struct Expected {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
    /// HEVC in MP4/MOV must be tagged `hvc1`.
    pub hvc1: bool,
    pub audio: Option<ExpectedAudio>,
}

/// The ffprobe codec name an ffmpeg audio encoder produces.
pub fn audio_codec_of(encoder: &str) -> String {
    match encoder {
        "libopus" => "opus".into(),
        "libmp3lame" => "mp3".into(),
        "libvorbis" => "vorbis".into(),
        other => other.into(),
    }
}

/// `ffprobe` arguments printing streams (with counted packets) and format as JSON.
pub fn metadata_args(file: &Path) -> Vec<OsString> {
    let mut a = argv([
        "-v",
        "error",
        "-protocol_whitelist",
        "file",
        "-count_packets",
        "-show_streams",
        "-show_format",
        "-of",
        "json",
    ]);
    a.push(file_arg(file));
    a
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn number(v: &Value, key: &str) -> Option<f64> {
    match v.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn same_rate(text: Option<&str>, fps: Fps) -> bool {
    text.and_then(Fps::parse)
        .is_some_and(|f| f.num as u64 * fps.den as u64 == fps.num as u64 * f.den as u64)
}

fn close_rate(text: Option<&str>, fps: Fps) -> bool {
    text.and_then(Fps::parse)
        .is_some_and(|f| (f.as_f64() - fps.as_f64()).abs() <= fps.as_f64() * 0.01)
}

/// Checks the ffprobe JSON of the hardware file; returns its video packet count and duration
/// (both are checked against the normal method's graph after [`compare_args`]).
pub fn check_metadata(json: &[u8], exp: &Expected) -> Result<Measured, CheckFailure> {
    let v: Value = serde_json::from_slice(json).map_err(|_| CheckFailure::Decode)?;
    let streams = v
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(CheckFailure::Decode)?;
    let of_type = |t: &str| streams.iter().find(|s| text(s, "codec_type") == Some(t));
    let video = of_type("video").ok_or(CheckFailure::Metadata("video stream"))?;
    if text(video, "codec_name") != Some(exp.codec.name()) {
        return Err(CheckFailure::Metadata("codec"));
    }
    if text(video, "profile") != Some(exp.codec.probed_profile()) {
        return Err(CheckFailure::Metadata("profile"));
    }
    if text(video, "pix_fmt") != Some("yuv420p") {
        return Err(CheckFailure::Metadata("picture format"));
    }
    if number(video, "width") != Some(exp.width as f64)
        || number(video, "height") != Some(exp.height as f64)
    {
        return Err(CheckFailure::Metadata("size"));
    }
    if !same_rate(text(video, "r_frame_rate"), exp.fps)
        && !close_rate(text(video, "avg_frame_rate"), exp.fps)
    {
        return Err(CheckFailure::Metadata("frame rate"));
    }
    if !matches!(
        text(video, "sample_aspect_ratio"),
        None | Some("1:1") | Some("0:1") | Some("N/A")
    ) {
        return Err(CheckFailure::Metadata("pixel shape"));
    }
    if exp.hvc1 && text(video, "codec_tag_string") != Some("hvc1") {
        return Err(CheckFailure::Metadata("hvc1 tag"));
    }
    if let Some(a) = &exp.audio {
        let s = of_type("audio").ok_or(CheckFailure::Metadata("audio stream"))?;
        if text(s, "codec_name") != Some(a.codec.as_str())
            || number(s, "sample_rate") != Some(a.sample_rate as f64)
            || number(s, "channels") != Some(a.channels as f64)
        {
            return Err(CheckFailure::Metadata("audio"));
        }
    }
    let packets = number(video, "nb_read_packets").ok_or(CheckFailure::Decode)?;
    if packets < 1.0 {
        return Err(CheckFailure::FrameCount {
            expected: 1,
            got: 0,
        });
    }
    let duration = number(video, "duration")
        .or_else(|| v.get("format").and_then(|f| number(f, "duration")))
        .filter(|d| d.is_finite());
    Ok(Measured {
        packets: packets as u64,
        duration,
    })
}

/// What the metadata says about the hardware file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measured {
    pub packets: u64,
    /// Video duration in seconds (the container's when the stream has none).
    pub duration: Option<f64>,
}

/// Largest file size allowed to constant quality, in times the normal method's (QG-05 f).
pub const MAX_SIZE_RATIO: f64 = 1.5;

/// A constant-quality file of `bytes` for `duration_us` stays within [`MAX_SIZE_RATIO`] of
/// what `bps` (the normal method's video and audio bitrate) gives; 64 KiB are allowed for the
/// container.
pub fn check_size(bytes: u64, bps: u64, duration_us: i64) -> Result<(), CheckFailure> {
    let usual = bps as f64 * (duration_us.max(0) as f64 / 1e6) / 8.0;
    let ratio = bytes.saturating_sub(64 * 1024) as f64 / usual.max(1.0);
    if ratio > MAX_SIZE_RATIO {
        return Err(CheckFailure::TooLarge(ratio));
    }
    Ok(())
}

/// The video lasts as long as the normal method's graph made it (`rendered_us`, the
/// graph's output time), within 2 frames.
pub fn check_duration(m: &Measured, rendered_us: i64, fps: Fps) -> Result<(), CheckFailure> {
    let Some(d) = m.duration else { return Ok(()) };
    let want = rendered_us as f64 / 1e6;
    if (d - want).abs() > 2.0 / fps.as_f64() + 0.001 {
        return Err(CheckFailure::Metadata("duration"));
    }
    Ok(())
}

/// The sample frame indices of a `frames`-long video (10%, 50%, 90%).
pub fn sample_indices(frames: u64) -> [u64; 3] {
    let last = frames.saturating_sub(1);
    SAMPLE_PERCENT.map(|p| (frames * p / 100).min(last))
}

/// Each sample with its neighbours (±1, inside the video).
pub fn neighbours(frames: u64, samples: &[u64; 3]) -> [Vec<u64>; 3] {
    let last = frames.saturating_sub(1);
    samples.map(|k| {
        let mut v = vec![k.saturating_sub(1), k, (k + 1).min(last)];
        v.dedup();
        v
    })
}

/// The distinct frame indices to extract, in order.
pub fn extracted(groups: &[Vec<u64>; 3]) -> Vec<u64> {
    let mut all: Vec<u64> = groups.iter().flatten().copied().collect();
    all.sort_unstable();
    all.dedup();
    all
}

/// The full, strict decode of `file` (stops at the first error). Progress goes to stdout.
pub fn decode_args(file: &Path, threads: Option<u32>) -> Vec<OsString> {
    let mut a = argv(["-hide_banner", "-nostdin", "-v", "error", "-xerror"]);
    if let Some(t) = threads {
        a.extend(argv(["-threads".to_string(), t.to_string()]));
    }
    a.extend(argv(["-protocol_whitelist", "file", "-i"]));
    a.push(file_arg(file));
    a.extend(argv(["-map", "0:v:0", "-f", "null", "-"]));
    a.extend(argv(["-progress", "pipe:1", "-nostats"]));
    a
}

/// The last `frame=` value of `-progress` output.
pub fn last_frame(lines: &[String]) -> Option<u64> {
    lines
        .iter()
        .rev()
        .find_map(|l| l.trim().strip_prefix("frame=")?.trim().parse().ok())
}

/// File (in the working folder) where the `psnr` filter writes one line per sample.
pub const PSNR_LOG: &str = "psnr.log";

/// The comparison pass: the normal method's video graph (`graph`, inputs `inputs`, output
/// `[outv]`) and the hardware file as the next input. The first output counts every frame
/// of the graph; the second compares the frames `indices` of both sides (scaled down alike
/// above [`crate::render::CHECK_MAX_PIXELS`]) and logs their PSNR to [`PSNR_LOG`]. Runs in
/// the working folder (subtitles and the log are relative to it).
#[allow(clippy::too_many_arguments)]
pub fn compare_args(
    inputs: Vec<OsString>,
    input_count: usize,
    graph: &str,
    file: &Path,
    indices: &[u64],
    width: u32,
    height: u32,
    version: FfVersion,
    threads: Option<u32>,
    script: &Path,
) -> std::io::Result<Vec<OsString>> {
    let mut a = argv(["-hide_banner", "-nostdin", "-v", "error"]);
    if let Some(t) = threads {
        a.extend(argv(["-filter_complex_threads".to_string(), t.to_string()]));
    }
    a.extend(inputs);
    if let Some(t) = threads {
        a.extend(argv(["-threads".to_string(), t.to_string()]));
    }
    a.extend(argv(["-protocol_whitelist", "file", "-i"]));
    a.push(file_arg(file));
    let select: Vec<String> = indices.iter().map(|k| format!("eq(n,{k})")).collect();
    let pick = format!(
        "select='{}',settb=1/1000000,setpts=N,format=yuv420p{}",
        select.join("+"),
        crate::render::check_scale(width, height)
    );
    let full = format!(
        "{graph};[outv]split[all][r];[r]{pick}[rs];[{input_count}:v:0]{pick}[hs];\
         [hs][rs]psnr=stats_file={PSNR_LOG}[cmp]"
    );
    a.extend(crate::ffmpeg::filter_graph_args(&full, version, script)?);
    a.extend(argv([
        "-map", "[all]", "-f", "null", "-", "-map", "[cmp]", "-f", "null", "-",
    ]));
    if let Some(t) = threads {
        a.extend(argv(["-threads".to_string(), t.to_string()]));
    }
    a.extend(argv(["-progress", "pipe:1", "-nostats"]));
    Ok(a)
}

/// The PSNR of Y, U and V of each line of the `psnr` filter's log (`inf` counts as 100).
pub fn parse_psnr_log(text: &str) -> Vec<[f64; 3]> {
    let value = |line: &str, key: &str| -> Option<f64> {
        let v = line
            .split_whitespace()
            .find_map(|w| w.strip_prefix(key)?.strip_prefix(':'))?;
        if v.starts_with("inf") {
            return Some(100.0);
        }
        v.parse().ok()
    };
    text.lines()
        .filter_map(|l| {
            Some([
                value(l, "psnr_y")?,
                value(l, "psnr_u")?,
                value(l, "psnr_v")?,
            ])
        })
        .collect()
}

/// Judges the samples: `scores` are the PSNR lines of the compared frames `indices` (in
/// order). Each sample passes when one of its neighbours reaches [`MIN_PSNR`] on every
/// plane; a frame without a score (the file is short) scores 0.
pub fn judge(
    indices: &[u64],
    scores: &[[f64; 3]],
    groups: &[Vec<u64>; 3],
) -> Result<(), CheckFailure> {
    for (index, group) in groups.iter().enumerate() {
        let best = group
            .iter()
            .map(|k| {
                indices
                    .iter()
                    .position(|i| i == k)
                    .and_then(|pos| scores.get(pos))
                    .map(|p| p.iter().copied().fold(f64::INFINITY, f64::min))
                    .unwrap_or(0.0)
            })
            .fold(0.0, f64::max);
        if best < MIN_PSNR {
            return Err(CheckFailure::Picture { index, psnr: best });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
