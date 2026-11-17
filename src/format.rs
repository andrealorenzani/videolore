// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Output format decisions: equality of source formats, choice of the lowest-quality format,
//! and mapping codecs to encoders (with a safe H.264/AAC/MP4 fallback).

use crate::media::MediaFormat;
use crate::time::Fps;
use std::collections::BTreeSet;

/// True when all formats share container, codecs, size and frame rate.
pub fn all_identical(formats: &[&MediaFormat]) -> bool {
    match formats.split_first() {
        None => true,
        Some((first, rest)) => rest.iter().all(|f| f.identity() == first.identity()),
    }
}

/// The format of the output: the lowest by pixel count, then frame rate, then bitrate,
/// then the earliest in timeline order. `formats` must be in timeline order.
pub fn choose_output(formats: &[&MediaFormat]) -> Option<MediaFormat> {
    formats
        .iter()
        .min_by_key(|f| {
            (
                f.pixels(),
                f.fps.map(Fps::cmp_value).unwrap_or(u128::MAX),
                f.bit_rate.unwrap_or(u64::MAX),
            )
        })
        .map(|f| (*f).clone())
}

/// Everything ffmpeg needs to encode the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeSettings {
    pub muxer: String,
    pub extension: String,
    pub vencoder: String,
    pub vextra: Vec<String>,
    pub aencoder: String,
    pub video_bitrate: Option<u64>,
    pub audio_bitrate: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
    pub faststart: bool,
}

fn video_encoder(codec: &str, encoders: &BTreeSet<String>) -> Option<(String, Vec<String>)> {
    let candidates: &[(&str, &[&str])] = match codec {
        "h264" => &[("libx264", &["-preset", "veryfast"])],
        "hevc" => &[("libx265", &["-preset", "veryfast"])],
        "vp9" => &[("libvpx-vp9", &["-deadline", "good", "-cpu-used", "5"])],
        "av1" => &[
            ("libsvtav1", &["-preset", "10"]),
            ("libaom-av1", &["-cpu-used", "8"]),
        ],
        "mpeg4" => &[("mpeg4", &[])],
        _ => &[],
    };
    candidates
        .iter()
        .find(|(e, _)| encoders.contains(*e))
        .map(|(e, x)| (e.to_string(), x.iter().map(|s| s.to_string()).collect()))
}

fn audio_encoder(codec: Option<&str>, muxer: &str, encoders: &BTreeSet<String>) -> Option<String> {
    let preferred = match codec {
        Some("aac") => "aac",
        Some("opus") => "libopus",
        Some("mp3") => "libmp3lame",
        _ if muxer == "webm" => "libopus",
        _ => "aac",
    };
    if encoders.contains(preferred) {
        Some(preferred.into())
    } else if muxer != "webm" && encoders.contains("aac") {
        Some("aac".into())
    } else {
        None
    }
}

fn muxer_for(container: &str) -> (&'static str, &'static str) {
    match container {
        "mov" => ("mov", "mov"),
        "matroska" => ("matroska", "mkv"),
        "webm" => ("webm", "webm"),
        "avi" => ("avi", "avi"),
        _ => ("mp4", "mp4"),
    }
}

fn even(n: u32) -> u32 {
    (n & !1).max(2)
}

/// The H.264 + AAC MP4 fallback.
pub fn fallback(format: &MediaFormat) -> EncodeSettings {
    EncodeSettings {
        muxer: "mp4".into(),
        extension: "mp4".into(),
        vencoder: "libx264".into(),
        vextra: vec!["-preset".into(), "veryfast".into()],
        aencoder: "aac".into(),
        video_bitrate: format.bit_rate,
        audio_bitrate: format.audio_bit_rate,
        width: even(format.width),
        height: even(format.height),
        fps: format.fps.unwrap_or(Fps { num: 30, den: 1 }),
        faststart: true,
    }
}

/// Encoder settings that reproduce `format` as closely as the installed encoders allow.
pub fn encode_settings(format: &MediaFormat, encoders: &BTreeSet<String>) -> EncodeSettings {
    let (muxer, ext) = muxer_for(&format.container);
    let video = format
        .vcodec
        .as_deref()
        .and_then(|c| video_encoder(c, encoders));
    let audio = audio_encoder(format.acodec.as_deref(), muxer, encoders);
    let webm_ok = |v: &str| v.starts_with("libvpx") || v.contains("av1");
    match (video, audio) {
        (Some((v, x)), Some(a)) if muxer != "webm" || webm_ok(&v) => EncodeSettings {
            muxer: muxer.into(),
            extension: ext.into(),
            vencoder: v,
            vextra: x,
            aencoder: a,
            faststart: matches!(muxer, "mp4" | "mov"),
            ..fallback(format)
        },
        _ => fallback(format),
    }
}

/// Codec of a video preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetCodec {
    H264,
    Hevc,
}

/// One entry of the video format list of the Export dialog (REQ-120): a size, a frame
/// rate, a codec and a container, fixed in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPreset {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub codec: PresetCodec,
    /// `true`: `.mov`, else `.mp4`.
    pub mov: bool,
}

const fn preset(width: u32, height: u32, fps: u32, codec: PresetCodec, mov: bool) -> VideoPreset {
    VideoPreset {
        width,
        height,
        fps,
        codec,
        mov,
    }
}

/// The video presets (ADR-083). Always re-encoded; H.264 and HEVC, so the hardware export
/// applies.
pub const VIDEO_PRESETS: [VideoPreset; 12] = [
    preset(1920, 1080, 25, PresetCodec::H264, false),
    preset(1920, 1080, 30, PresetCodec::H264, false),
    preset(1920, 1080, 60, PresetCodec::H264, false),
    preset(1920, 1080, 25, PresetCodec::H264, true),
    preset(1920, 1080, 25, PresetCodec::Hevc, false),
    preset(1920, 1080, 30, PresetCodec::Hevc, false),
    preset(1280, 720, 25, PresetCodec::H264, false),
    preset(1280, 720, 30, PresetCodec::H264, false),
    preset(1280, 720, 25, PresetCodec::Hevc, false),
    preset(854, 480, 25, PresetCodec::H264, false),
    preset(3840, 2160, 25, PresetCodec::Hevc, false),
    preset(3840, 2160, 30, PresetCodec::Hevc, true),
];

impl VideoPreset {
    /// The encoder this preset needs.
    pub fn encoder(&self) -> &'static str {
        match self.codec {
            PresetCodec::H264 => "libx264",
            PresetCodec::Hevc => "libx265",
        }
    }

    /// The file extension, without the dot.
    pub fn extension(&self) -> &'static str {
        if self.mov { "mov" } else { "mp4" }
    }

    /// How the dropdown shows it: `1920x1080 25fps, libx264, .mp4`.
    pub fn label(&self) -> String {
        format!(
            "{}x{} {}fps, {}, .{}",
            self.width,
            self.height,
            self.fps,
            self.encoder(),
            self.extension()
        )
    }

    /// The format the settings are made from.
    fn format(&self) -> MediaFormat {
        MediaFormat {
            container: if self.mov { "mov" } else { "mp4" }.into(),
            vcodec: Some(
                match self.codec {
                    PresetCodec::H264 => "h264",
                    PresetCodec::Hevc => "hevc",
                }
                .into(),
            ),
            width: self.width,
            height: self.height,
            fps: Fps::new(self.fps, 1),
            ..Default::default()
        }
    }
}

/// The presets the installed encoders can make, as indexes of [`VIDEO_PRESETS`].
pub fn available_presets(encoders: &BTreeSet<String>) -> Vec<usize> {
    VIDEO_PRESETS
        .iter()
        .enumerate()
        .filter(|(_, p)| encoders.contains(p.encoder()))
        .map(|(i, _)| i)
        .collect()
}

/// The encoder settings of preset `index`; `None` when the index is unknown or the encoder
/// is missing.
pub fn preset_settings(index: usize, encoders: &BTreeSet<String>) -> Option<EncodeSettings> {
    let p = VIDEO_PRESETS.get(index)?;
    encoders.contains(p.encoder()).then(|| {
        let mut s = encode_settings(&p.format(), encoders);
        s.video_bitrate = None;
        s.audio_bitrate = None;
        s
    })
}

/// The audio format of an export (REQ-121).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioChoice {
    /// What the automatic choice makes today.
    #[default]
    Same,
    Aac,
    Mp3,
}

impl AudioChoice {
    pub fn label(self) -> &'static str {
        match self {
            AudioChoice::Same => "Same as source",
            AudioChoice::Aac => "aac",
            AudioChoice::Mp3 => "mp3",
        }
    }

    /// The encoder, `None` for [`AudioChoice::Same`].
    pub fn encoder(self) -> Option<&'static str> {
        match self {
            AudioChoice::Same => None,
            AudioChoice::Aac => Some("aac"),
            AudioChoice::Mp3 => Some("libmp3lame"),
        }
    }

    /// True when the installed encoders can make it.
    pub fn available(self, encoders: &BTreeSet<String>) -> bool {
        self.encoder().is_none_or(|e| encoders.contains(e))
    }
}

/// The format choices of the Export dialog that change the encoding (REQ-120, REQ-121).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormatChoice {
    /// A preset of [`VIDEO_PRESETS`]; `None`: same as source.
    pub video: Option<usize>,
    pub audio: AudioChoice,
}

impl FormatChoice {
    /// True when nothing is chosen: the automatic format, stream copy where possible.
    pub fn is_automatic(&self) -> bool {
        self.video.is_none() && self.audio == AudioChoice::Same
    }
}

/// The settings of an audio-only file (REQ-119, REQ-121): AAC in `.m4a` (also the choice
/// "Same as source") or MP3; `None` when the encoder is missing.
pub fn audio_only_settings(
    choice: AudioChoice,
    encoders: &BTreeSet<String>,
) -> Option<EncodeSettings> {
    let (encoder, muxer, ext) = match choice {
        AudioChoice::Mp3 => ("libmp3lame", "mp3", "mp3"),
        AudioChoice::Aac | AudioChoice::Same => ("aac", "mp4", "m4a"),
    };
    encoders.contains(encoder).then(|| EncodeSettings {
        muxer: muxer.into(),
        extension: ext.into(),
        vencoder: String::new(),
        vextra: Vec::new(),
        aencoder: encoder.into(),
        video_bitrate: None,
        audio_bitrate: Some(192_000),
        width: 2,
        height: 2,
        fps: Fps { num: 1, den: 1 },
        faststart: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(w: u32, h: u32, fps: u32, br: Option<u64>) -> MediaFormat {
        MediaFormat {
            container: "mp4".into(),
            vcodec: Some("h264".into()),
            acodec: Some("aac".into()),
            width: w,
            height: h,
            fps: Fps::new(fps, 1),
            bit_rate: br,
            ..Default::default()
        }
    }

    fn enc(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    // covers: REQ-022
    #[test]
    fn identical_formats_are_kept() {
        let a = fmt(1280, 720, 30, Some(1));
        let b = fmt(1280, 720, 30, Some(999)); // bitrate is not part of identity
        assert!(all_identical(&[&a, &b]));
        assert!(all_identical(&[]));
        let c = fmt(640, 360, 30, Some(1));
        assert!(!all_identical(&[&a, &c]));
        let s = encode_settings(&a, &enc(&["libx264", "aac"]));
        assert_eq!(s.muxer, "mp4");
        assert_eq!(s.vencoder, "libx264");
        assert_eq!(s.vextra, vec!["-preset", "veryfast"]);
        assert!(s.faststart);
    }

    // covers: REQ-022
    #[test]
    fn lowest_quality_wins_with_tiebreaks() {
        let hd = fmt(1280, 720, 30, Some(5));
        let sd = fmt(640, 360, 30, Some(5));
        let sd25 = fmt(640, 360, 25, Some(9));
        let sd25_low = fmt(640, 360, 25, Some(1));
        assert_eq!(choose_output(&[&hd, &sd]).unwrap(), sd);
        assert_eq!(choose_output(&[&hd, &sd, &sd25]).unwrap(), sd25);
        assert_eq!(choose_output(&[&sd25, &sd25_low]).unwrap(), sd25_low);
        let mut first = sd25_low.clone();
        first.container = "mov".into();
        assert_eq!(choose_output(&[&first, &sd25_low]).unwrap(), first);
        assert_eq!(choose_output(&[]), None);
    }

    // covers: REQ-120
    #[test]
    fn presets_read_like_the_example_and_make_their_settings() {
        let all = enc(&["libx264", "libx265", "aac", "libmp3lame"]);
        assert_eq!(VIDEO_PRESETS.len(), 12);
        let p = VIDEO_PRESETS[6];
        assert_eq!(p.label(), "1280x720 25fps, libx264, .mp4");
        let s = preset_settings(6, &all).unwrap();
        assert_eq!(
            (
                s.width,
                s.height,
                s.fps,
                s.vencoder.as_str(),
                s.muxer.as_str()
            ),
            (1280, 720, Fps { num: 25, den: 1 }, "libx264", "mp4")
        );
        assert_eq!(s.video_bitrate, None);
        let mov = preset_settings(3, &all).unwrap();
        assert_eq!((mov.muxer.as_str(), mov.extension.as_str()), ("mov", "mov"));
        let hevc = preset_settings(11, &all).unwrap();
        assert_eq!(hevc.vencoder, "libx265");
        assert_eq!((hevc.width, hevc.height), (3840, 2160));
        // every preset has a distinct label and an encoder
        let labels: BTreeSet<String> = VIDEO_PRESETS.iter().map(VideoPreset::label).collect();
        assert_eq!(labels.len(), VIDEO_PRESETS.len());
        // presets whose encoder is missing are not offered
        assert_eq!(available_presets(&all).len(), 12);
        let no265 = enc(&["libx264", "aac"]);
        assert_eq!(available_presets(&no265).len(), 7);
        assert_eq!(preset_settings(11, &no265), None);
        assert_eq!(preset_settings(99, &all), None);
    }

    // covers: REQ-121
    #[test]
    fn audio_choices_need_their_encoder() {
        let all = enc(&["aac", "libmp3lame"]);
        let no_mp3 = enc(&["aac"]);
        assert!(AudioChoice::Same.available(&no_mp3));
        assert!(AudioChoice::Aac.available(&no_mp3));
        assert!(AudioChoice::Mp3.available(&all) && !AudioChoice::Mp3.available(&no_mp3));
        assert_eq!(AudioChoice::Same.label(), "Same as source");
        assert!(FormatChoice::default().is_automatic());
        assert!(
            !FormatChoice {
                audio: AudioChoice::Aac,
                ..Default::default()
            }
            .is_automatic()
        );
        let m = audio_only_settings(AudioChoice::Mp3, &all).unwrap();
        assert_eq!((m.muxer.as_str(), m.extension.as_str()), ("mp3", "mp3"));
        let a = audio_only_settings(AudioChoice::Same, &all).unwrap();
        assert_eq!((a.aencoder.as_str(), a.extension.as_str()), ("aac", "m4a"));
        assert_eq!(audio_only_settings(AudioChoice::Mp3, &no_mp3), None);
    }

    #[test]
    fn codec_mapping_and_fallbacks() {
        let all = enc(&[
            "libx264",
            "libx265",
            "libvpx-vp9",
            "libaom-av1",
            "mpeg4",
            "aac",
            "libopus",
            "libmp3lame",
        ]);
        let mut f = fmt(1279, 721, 30, Some(10));
        f.vcodec = Some("hevc".into());
        f.container = "mov".into();
        let s = encode_settings(&f, &all);
        assert_eq!((s.muxer.as_str(), s.vencoder.as_str()), ("mov", "libx265"));
        assert_eq!((s.width, s.height), (1278, 720));
        f.vcodec = Some("vp9".into());
        f.acodec = Some("opus".into());
        f.container = "webm".into();
        let s = encode_settings(&f, &all);
        assert_eq!(
            (s.vencoder.as_str(), s.aencoder.as_str()),
            ("libvpx-vp9", "libopus")
        );
        assert!(!s.faststart);
        f.vcodec = Some("av1".into());
        f.container = "matroska".into();
        f.acodec = Some("mp3".into());
        let s = encode_settings(&f, &all);
        assert_eq!(s.vencoder, "libaom-av1");
        assert_eq!(s.aencoder, "libmp3lame");
        assert_eq!(s.extension, "mkv");
        f.vcodec = Some("mpeg4".into());
        f.container = "avi".into();
        f.acodec = None;
        let s = encode_settings(&f, &all);
        assert_eq!((s.muxer.as_str(), s.aencoder.as_str()), ("avi", "aac"));
        // unknown codec -> H.264/AAC MP4
        f.vcodec = Some("prores".into());
        assert_eq!(encode_settings(&f, &all), fallback(&f));
        // encoder not installed -> fallback
        f.vcodec = Some("hevc".into());
        assert_eq!(encode_settings(&f, &enc(&["aac"])).vencoder, "libx264");
        // h264 in webm is impossible -> fallback to mp4
        f.vcodec = Some("h264".into());
        f.container = "webm".into();
        assert_eq!(encode_settings(&f, &all).muxer, "mp4");
        // webm without libopus -> fallback
        f.vcodec = Some("vp9".into());
        assert_eq!(
            encode_settings(&f, &enc(&["libvpx-vp9", "aac"])).muxer,
            "mp4"
        );
        // mp3 encoder missing -> aac
        f.container = "mp4".into();
        f.acodec = Some("mp3".into());
        assert_eq!(
            encode_settings(&f, &enc(&["libvpx-vp9", "aac"])).aencoder,
            "aac"
        );
        f.container = "flv".into();
        assert_eq!(encode_settings(&f, &all).muxer, "mp4");
        let mut nofps = fmt(1, 1, 30, None);
        nofps.fps = None;
        let fb = fallback(&nofps);
        assert_eq!(fb.fps, Fps { num: 30, den: 1 });
        assert_eq!((fb.width, fb.height), (2, 2));
    }
}
