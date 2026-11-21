// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Hardware decoding for the preview and the preview copies (REQ-214, ADR-178).
//!
//! With Export: Faster the preview decodes the sources with the video hardware that the check
//! found working, so that big videos play more smoothly:
//! * [`probe`] decodes a short clip, made on the spot by the bundled software encoder of each
//!   format, with each method of this computer and keeps the methods that give every frame;
//! * [`Decoder::input_args`] are the arguments put before the source of a preview run
//!   ([`apply`]);
//! * a decoder that fails (at the start or in the middle of a playback) is dropped by the
//!   preview, which goes on at the playhead with the normal decoder.
//!
//! Everything runs through a [`Runner`], in the background check; the device of a VA-API
//! decoder only ever comes from [`crate::hwenc::render_nodes`].

use crate::ffmpeg::{Runner, argv, file_arg};
use crate::hwenc::{Candidate, Codec, Os, Platform, ProbeLimits, Vendor, libva_too_old};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Frames of the probe clip.
pub const SAMPLE_FRAMES: u64 = 15;

/// A hardware decoder: the method of a vendor and, for VA-API, its render device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Decoder {
    pub vendor: Vendor,
    pub device: Option<PathBuf>,
}

impl Decoder {
    /// The ffmpeg `-hwaccel` name of the method.
    pub fn method(&self) -> &'static str {
        match self.vendor {
            Vendor::Nvenc => "cuda",
            Vendor::Qsv => "qsv",
            // the Windows decoder every maker's graphics has
            Vendor::Amf => "d3d11va",
            Vendor::Vaapi => "vaapi",
            Vendor::VideoToolbox => "videotoolbox",
        }
    }

    /// The name of the device, the same text as [`Candidate::label`] (the choice of REQ-216
    /// applies to the decoder of that device).
    pub fn label(&self) -> String {
        Candidate {
            vendor: self.vendor,
            device: self.device.clone(),
            rc: Default::default(),
        }
        .label()
    }

    /// The input options that make ffmpeg decode with this method; the frames come back to
    /// system memory, so every filter of the preview works as before.
    pub fn input_args(&self) -> Vec<OsString> {
        let mut a = argv(["-hwaccel", self.method()]);
        if let Some(d) = &self.device {
            a.push("-hwaccel_device".into());
            a.push(d.as_os_str().to_os_string());
        }
        a
    }
}

/// The decoders this computer may have, best first (the order of the encoders).
pub fn candidates(platform: &Platform) -> Vec<Decoder> {
    crate::hwenc::methods(platform)
        .into_iter()
        .map(|c| Decoder {
            vendor: c.vendor,
            device: c.device,
        })
        .collect()
}

/// The ffmpeg arguments making the probe clip of `codec` with `encoder` into `out`
/// (matroska holds every format).
pub fn sample_args(encoder: &str, out: &Path) -> Vec<OsString> {
    let mut a = argv([
        "-hide_banner",
        "-nostdin",
        "-v",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=320x180:rate=30",
        "-frames:v",
    ]);
    a.push(SAMPLE_FRAMES.to_string().into());
    a.extend(argv(["-pix_fmt", "yuv420p", "-c:v", encoder]));
    let fast: &[&str] = match encoder {
        "libx264" => &["-preset", "ultrafast"],
        "libx265" => &["-preset", "ultrafast", "-x265-params", "log-level=error"],
        "libsvtav1" => &["-preset", "12"],
        "libaom-av1" => &["-cpu-used", "8"],
        "libvpx-vp9" => &["-deadline", "realtime", "-cpu-used", "8"],
        _ => &[],
    };
    a.extend(argv(fast));
    a.extend(argv(["-f", "matroska"]));
    a.push(file_arg(out));
    a
}

/// The decode of the probe clip with `dec`; its progress goes to stdout.
pub fn decode_args(dec: &Decoder, sample: &Path) -> Vec<OsString> {
    let mut a = argv(["-hide_banner", "-nostdin", "-v", "warning"]);
    a.extend(dec.input_args());
    a.extend(argv(["-protocol_whitelist", "file", "-i"]));
    a.push(file_arg(sample));
    a.extend(argv(["-f", "null", "-", "-progress", "pipe:1", "-nostats"]));
    a
}

/// Words of ffmpeg's messages that tell it did not use the hardware (it then decodes with
/// the processor and still succeeds).
const FELL_BACK: &[&str] = &[
    "hwaccel",
    "failed setup",
    "device creation failed",
    "failed to create",
    "cannot load",
    "no device",
];

/// Whether a decode of the probe clip used the hardware and gave every frame.
pub fn decoded_with_hardware(stdout: &str, stderr: &str) -> bool {
    let frames = stdout
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix("frame=")?.trim().parse::<u64>().ok());
    let lower = stderr.to_ascii_lowercase();
    frames == Some(SAMPLE_FRAMES) && !FELL_BACK.iter().any(|w| lower.contains(w))
}

/// The decoders that work, by format, best first.
pub type Working = BTreeMap<Codec, Vec<Decoder>>;

/// Decodes a probe clip of each format (made by the bundled software encoder, when
/// `encoders` has one) with each decoder of `platform` and keeps the ones that give all its
/// frames from the hardware. Each try is limited to `limits.per` (REQ-061 limits), the whole
/// to `limits.total`; VA-API and Quick Sync are left out when `libva_blocked`.
pub fn probe(
    runner: &dyn Runner,
    ffmpeg: &Path,
    encoders: &BTreeSet<String>,
    platform: &Platform,
    libva_blocked: bool,
    limits: ProbeLimits,
    cancel: &AtomicBool,
) -> Working {
    let mut out = Working::new();
    let mut decoders = candidates(platform);
    if libva_blocked {
        decoders.retain(|d| !matches!(d.vendor, Vendor::Vaapi | Vendor::Qsv));
    }
    if decoders.is_empty() {
        return out;
    }
    let Ok(dir) = tempfile::Builder::new()
        .prefix("videolore-hwdec-")
        .tempdir()
    else {
        return out;
    };
    let started = Instant::now();
    for codec in Codec::ALL {
        let Some(encoder) = codec
            .software_encoders()
            .iter()
            .find(|e| encoders.contains(**e))
        else {
            continue;
        };
        if cancel.load(Ordering::SeqCst) || started.elapsed() >= limits.total {
            break;
        }
        let sample = dir.path().join(format!("sample-{}.mkv", codec.name()));
        let made = runner.run_within(ffmpeg, &sample_args(encoder, &sample), limits.retry);
        if !made.is_ok_and(|o| o.success) {
            continue;
        }
        for d in &decoders {
            let left = limits.total.saturating_sub(started.elapsed());
            if cancel.load(Ordering::SeqCst) || left.is_zero() {
                break;
            }
            let r = runner.run_within(
                ffmpeg,
                &decode_args(d, &sample),
                limits.per.min(left).max(Duration::from_millis(1)),
            );
            if let Ok(o) = r
                && o.success
                && decoded_with_hardware(&String::from_utf8_lossy(&o.stdout), &o.stderr)
            {
                out.entry(codec).or_default().push(d.clone());
            }
        }
    }
    out
}

/// [`probe`] for the computer `platform`, leaving out what the old system video library
/// cannot run.
pub fn probe_all(
    runner: &dyn Runner,
    ffmpeg: &Path,
    encoders: &BTreeSet<String>,
    platform: &Platform,
    limits: ProbeLimits,
    cancel: &AtomicBool,
) -> Working {
    let blocked = platform.os == Os::Linux
        && libva_too_old(
            crate::hwenc::ffmpeg_needs_new_libva(ffmpeg),
            platform.system_libva_new,
        );
    probe(runner, ffmpeg, encoders, platform, blocked, limits, cancel)
}

/// The decoder to use for each format: the first working one, or, with a chosen device
/// (`device`, a name of [`Decoder::label`]), the working decoder of that device.
pub fn choose(working: &Working, device: Option<&str>) -> HashMap<Codec, Decoder> {
    working
        .iter()
        .filter_map(|(codec, list)| {
            let d = match device {
                None => list.first(),
                Some(name) => list.iter().find(|d| d.label() == name),
            }?;
            Some((*codec, d.clone()))
        })
        .collect()
}

/// Puts the decoder of each input's format before its `-i` in `args` (a preview run).
/// `codec_of` tells the format of the file named by the argument after `-i` (`file:…`);
/// inputs it does not know (images, 360 sources, other formats) are left alone.
pub fn apply(
    args: Vec<OsString>,
    decoders: &HashMap<Codec, Decoder>,
    codec_of: &dyn Fn(&OsString) -> Option<Codec>,
) -> Vec<OsString> {
    if decoders.is_empty() {
        return args;
    }
    let mut out = Vec::with_capacity(args.len() + 8);
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-i"
            && let Some(file) = args.get(i + 1)
            && let Some(dec) = codec_of(file).and_then(|c| decoders.get(&c))
        {
            out.extend(dec.input_args());
        }
        out.push(args[i].clone());
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests;
