// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Video hardware for the export ("Export: Faster").
//!
//! * [`candidates`] lists, in a fixed preference order, the hardware methods the ffmpeg
//!   in use was built with (NVIDIA NVENC, Intel Quick Sync, AMD AMF, Linux VA-API, Apple
//!   VideoToolbox), separately for H.264 and HEVC.
//! * [`encode_args`] gives the arguments of a method: they replace the software video
//!   arguments of the export (same bitrate target, GOP 250, profile High/Main, 8-bit
//!   4:2:0), so the file looks like the Best quality one.
//! * [`probe`] tries the candidates with a tiny synthetic encode (at most 1 s each, one
//!   retry of at most 4 s for a try that ran out of time, 10 s in total) and keeps the
//!   first one that works for each format.
//!
//! Everything is pure except [`probe`] (runs ffmpeg through a [`Runner`]) and
//! [`render_nodes`] (lists `/dev/dri`). The VA-API device path only ever comes from
//! enumerated `renderD<number>` character devices; there is never a shell.

use crate::ffmpeg::{Runner, argv};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The formats that may use video hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    H264,
    Hevc,
    Av1,
    Vp9,
}

impl Codec {
    pub const ALL: [Codec; 4] = [Codec::H264, Codec::Hevc, Codec::Av1, Codec::Vp9];

    /// ffmpeg/ffprobe codec name.
    pub fn name(self) -> &'static str {
        match self {
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
            Codec::Av1 => "av1",
            Codec::Vp9 => "vp9",
        }
    }

    /// The codec of an ffprobe codec name, when it may use hardware.
    pub fn from_name(name: &str) -> Option<Codec> {
        Codec::ALL.into_iter().find(|c| c.name() == name)
    }

    /// The codec the normal method encodes with `encoder`, when it may use hardware.
    pub fn of_software_encoder(encoder: &str) -> Option<Codec> {
        match encoder {
            "libx264" => Some(Codec::H264),
            "libx265" => Some(Codec::Hevc),
            "libsvtav1" | "libaom-av1" => Some(Codec::Av1),
            "libvpx-vp9" => Some(Codec::Vp9),
            _ => None,
        }
    }

    /// The software encoders that make this format, best first (the Export presets use the
    /// first one the ffmpeg in use has).
    pub fn software_encoders(self) -> &'static [&'static str] {
        match self {
            Codec::H264 => &["libx264"],
            Codec::Hevc => &["libx265"],
            Codec::Av1 => &["libsvtav1", "libaom-av1"],
            Codec::Vp9 => &["libvpx-vp9"],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::Hevc => "HEVC",
            Codec::Av1 => "AV1",
            Codec::Vp9 => "VP9",
        }
    }

    /// The profile the normal method produces (x264 "veryfast" High, x265 Main); AV1 and
    /// VP9 have no profile argument (8-bit 4:2:0 is their main profile).
    pub fn profile(self) -> Option<&'static str> {
        match self {
            Codec::H264 => Some("high"),
            Codec::Hevc => Some("main"),
            Codec::Av1 | Codec::Vp9 => None,
        }
    }

    /// The profile name ffprobe reports for [`Codec::profile`].
    pub fn probed_profile(self) -> Option<&'static str> {
        match self {
            Codec::H264 => Some("High"),
            Codec::Hevc => Some("Main"),
            Codec::Av1 | Codec::Vp9 => None,
        }
    }
}

/// A hardware method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Vendor {
    #[serde(rename = "nvenc")]
    Nvenc,
    #[serde(rename = "qsv")]
    Qsv,
    #[serde(rename = "amf")]
    Amf,
    #[serde(rename = "vaapi")]
    Vaapi,
    #[serde(rename = "videotoolbox")]
    VideoToolbox,
}

impl Vendor {
    pub const ALL: [Vendor; 5] = [
        Vendor::Nvenc,
        Vendor::Qsv,
        Vendor::Amf,
        Vendor::Vaapi,
        Vendor::VideoToolbox,
    ];

    /// Suffix of the ffmpeg encoder names (`h264_<suffix>`).
    pub fn suffix(self) -> &'static str {
        match self {
            Vendor::Nvenc => "nvenc",
            Vendor::Qsv => "qsv",
            Vendor::Amf => "amf",
            Vendor::Vaapi => "vaapi",
            Vendor::VideoToolbox => "videotoolbox",
        }
    }

    /// Name shown in the Advanced section.
    pub fn label(self) -> &'static str {
        match self {
            Vendor::Nvenc => "NVIDIA (NVENC)",
            Vendor::Qsv => "Intel Quick Sync",
            Vendor::Amf => "AMD (AMF)",
            Vendor::Vaapi => "VA-API",
            Vendor::VideoToolbox => "Apple VideoToolbox",
        }
    }

    /// Short maker name for problem notes.
    pub fn maker(self) -> &'static str {
        match self {
            Vendor::Nvenc => "NVIDIA",
            Vendor::Qsv => "Intel",
            Vendor::Amf => "AMD",
            Vendor::Vaapi => "VA-API",
            Vendor::VideoToolbox => "Apple",
        }
    }

    /// Bitrate target relative to the normal method, in percent. Methods expected to give
    /// a little less quality at the same bitrate get a little more (tuned by QG-05).
    pub fn bitrate_percent(self) -> u64 {
        match self {
            Vendor::Nvenc | Vendor::Qsv => 100,
            Vendor::Vaapi => 110,
            Vendor::Amf | Vendor::VideoToolbox => 120,
        }
    }

    /// Options only this method understands (removed by test doubles that simulate it).
    pub fn options(self) -> &'static [&'static str] {
        match self {
            Vendor::Nvenc => &["-preset", "-tune", "-rc", "-spatial_aq"],
            Vendor::Qsv => &["-preset", "-global_quality"],
            Vendor::Amf => &["-usage", "-quality", "-rc"],
            Vendor::Vaapi => &["-rc_mode", "-qp"],
            Vendor::VideoToolbox => &["-allow_sw", "-realtime"],
        }
    }
}

/// How a method controls the size of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateControl {
    /// About the normal method's bitrate (the first choice of every method).
    #[default]
    Bitrate,
    /// A constant quantiser ([`CQ_START_QP`], corrected by [`cq_retry_qp`]): only for
    /// VA-API and Quick Sync drivers that refuse bitrate control.
    ConstantQuality,
}

/// One method to try: a vendor, for VA-API the render device, and the rate control.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Candidate {
    pub vendor: Vendor,
    pub device: Option<PathBuf>,
    #[serde(default)]
    pub rc: RateControl,
}

impl Candidate {
    pub fn new(vendor: Vendor) -> Candidate {
        Candidate {
            vendor,
            device: None,
            rc: RateControl::Bitrate,
        }
    }

    /// The same method with constant quality, when its driver may refuse bitrate control
    /// (VA-API, and Quick Sync, REQ-215).
    pub fn constant_quality(&self) -> Option<Candidate> {
        (matches!(self.vendor, Vendor::Vaapi | Vendor::Qsv) && self.rc == RateControl::Bitrate)
            .then(|| Candidate {
                rc: RateControl::ConstantQuality,
                ..self.clone()
            })
    }

    /// The ffmpeg encoder name for `codec`.
    pub fn encoder(&self, codec: Codec) -> String {
        format!("{}_{}", codec.name(), self.vendor.suffix())
    }

    /// Name shown in the Advanced section.
    pub fn label(&self) -> String {
        let mut extra: Vec<String> = Vec::new();
        if let Some(d) = self.device.as_deref().and_then(Path::file_name) {
            extra.push(d.to_string_lossy().into_owned());
        }
        if self.rc == RateControl::ConstantQuality {
            extra.push("constant quality".into());
        }
        if extra.is_empty() {
            self.vendor.label().to_string()
        } else {
            format!("{} ({})", self.vendor.label(), extra.join(", "))
        }
    }
}

/// The kind of graphics a method runs on (REQ-207).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graphics {
    /// Part of the processor (Intel Quick Sync, Apple silicon, a VA-API Intel device).
    Integrated,
    Dedicated,
}

/// The kind of graphics of a PCI vendor id read from the system (`0x8086` is Intel: its
/// VA-API devices are the processor's graphics; other makers' devices count as dedicated).
pub fn graphics_of_vendor_id(id: &str) -> Graphics {
    if id.trim().eq_ignore_ascii_case("0x8086") {
        Graphics::Integrated
    } else {
        Graphics::Dedicated
    }
}

impl Candidate {
    /// What kind of graphics runs this method. Quick Sync and VideoToolbox are the
    /// processor's; NVIDIA and AMD methods are taken as dedicated cards (an AMD processor
    /// graphics is not told apart); a VA-API device is told by its maker (`read` gives the
    /// text of a system file, `/sys/class/drm/<node>/device/vendor`).
    pub fn graphics(&self, read: impl Fn(&Path) -> Option<String>) -> Graphics {
        match self.vendor {
            Vendor::Qsv | Vendor::VideoToolbox => Graphics::Integrated,
            Vendor::Nvenc | Vendor::Amf => Graphics::Dedicated,
            Vendor::Vaapi => self
                .device
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|n| read(&Path::new("/sys/class/drm").join(n).join("device/vendor")))
                .map(|id| graphics_of_vendor_id(&id))
                .unwrap_or(Graphics::Dedicated),
        }
    }

    /// [`Candidate::graphics`] on this computer.
    pub fn graphics_here(&self) -> Graphics {
        self.graphics(|p| std::fs::read_to_string(p).ok())
    }
}

/// The operating system families with a preference order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
    Other,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        }
    }
}

/// What the probe needs to know about the computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    /// VA-API render devices (Linux).
    pub render_nodes: Vec<PathBuf>,
    /// The system video library (libva, Linux): `None` when not found, else whether it has
    /// the functions of libva 2.21+ ([`LIBVA_NEW_SYMBOL`]).
    pub system_libva_new: Option<bool>,
    /// The version of that library as `(major, minor)` when its file name tells.
    pub libva_version: Option<(u32, u32)>,
}

impl Platform {
    /// This computer.
    pub fn detect() -> Platform {
        let os = Os::current();
        let render_nodes = if os == Os::Linux {
            render_nodes(Path::new("/dev/dri"))
        } else {
            Vec::new()
        };
        let libva = if os == Os::Linux {
            find_system_libva()
        } else {
            None
        };
        let system_libva_new = libva.as_deref().map(|l| file_mentions(l, LIBVA_NEW_SYMBOL));
        let libva_version = libva.as_deref().and_then(libva_version_of);
        Platform {
            os,
            render_nodes,
            system_libva_new,
            libva_version,
        }
    }

    /// No hardware methods at all.
    pub fn none() -> Platform {
        Platform {
            os: Os::Other,
            render_nodes: Vec::new(),
            system_libva_new: None,
            libva_version: None,
        }
    }
}

/// The library version libva 2.`minor` in a file name: `libva.so.2.2000.0` is 2.20 (the
/// middle number is the minor version times 100).
pub fn libva_version_from_name(name: &str) -> Option<(u32, u32)> {
    let rest = name.strip_prefix("libva.so.")?;
    let mut parts = rest.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let current: u32 = parts.next()?.parse().ok()?;
    (major > 0 && current >= 100).then_some((major, current / 100))
}

/// The version of the libva at `path` (the link is followed: the real file's name has it).
fn libva_version_of(path: &Path) -> Option<(u32, u32)> {
    let real = std::fs::canonicalize(path).ok()?;
    libva_version_from_name(real.file_name()?.to_str()?)
}

/// What Advanced says when the video library is too old (REQ-217): both versions, in plain
/// words.
pub fn libva_message(installed: Option<(u32, u32)>) -> String {
    let have = match installed {
        Some((a, b)) => format!("{a}.{b}"),
        None => "older than 2.21".to_string(),
    };
    format!(
        "The video library of this computer (libva {have}) is older than the one the included \
         ffmpeg needs (libva 2.21 or newer): the normal method is used for Faster."
    )
}

/// A function added in libva 2.21. ffmpeg builds that load libva at run time (the
/// bundled Linux build) abort when the system library lacks it.
pub const LIBVA_NEW_SYMBOL: &[u8] = b"vaMapBuffer2";

/// Folders searched for the system `libva.so.2` (after `LD_LIBRARY_PATH`).
const LIB_DIRS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu",
    "/lib/x86_64-linux-gnu",
    "/usr/lib64",
    "/lib64",
    "/usr/lib",
    "/lib",
    "/usr/local/lib",
];

/// The system `libva.so.2`, if any (only looked at, never loaded).
pub fn find_system_libva() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("LD_LIBRARY_PATH")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    dirs.extend(LIB_DIRS.iter().map(PathBuf::from));
    dirs.into_iter()
        .map(|d| d.join("libva.so.2"))
        .find(|p| p.is_file())
}

/// Files larger than this are not searched.
const MAX_SCAN: u64 = 1 << 30;

/// Whether the file at `path` contains `needle` (read as plain bytes, in pieces; never
/// loaded as a library). Unreadable files do not.
pub fn file_mentions(path: &Path, needle: &[u8]) -> bool {
    let Ok(f) = std::fs::File::open(path) else {
        return false;
    };
    let mut r = f.take(MAX_SCAN);
    let mut buf = vec![0u8; 1 << 20];
    let keep = needle.len().saturating_sub(1);
    let mut have = 0usize;
    loop {
        let n = match r.read(&mut buf[have..]) {
            Ok(0) | Err(_) => return false,
            Ok(n) => n,
        };
        let end = have + n;
        if buf[..end].windows(needle.len()).any(|w| w == needle) {
            return true;
        }
        let tail = end.saturating_sub(keep);
        buf.copy_within(tail..end, 0);
        have = end - tail;
    }
}

/// VA-API and Quick Sync must be skipped (pure): the ffmpeg in use needs the functions of
/// libva 2.21+ (`ffmpeg_needs_new`) and the system library is missing or older.
pub fn libva_too_old(ffmpeg_needs_new: bool, system_libva_new: Option<bool>) -> bool {
    ffmpeg_needs_new && system_libva_new != Some(true)
}

/// Most render devices tried.
pub const MAX_RENDER_NODES: usize = 4;

/// The number of a `renderD<number>` name.
fn render_number(name: &str) -> Option<u32> {
    let digits = name.strip_prefix("renderD")?;
    if digits.is_empty() || digits.len() > 6 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// From `(file name, is a character device)` entries, the render devices in number order
/// (at most [`MAX_RENDER_NODES`]).
pub fn select_render_nodes(entries: &[(String, bool)]) -> Vec<String> {
    let mut nodes: Vec<(u32, &String)> = entries
        .iter()
        .filter(|(_, chr)| *chr)
        .filter_map(|(n, _)| render_number(n).map(|k| (k, n)))
        .collect();
    nodes.sort();
    nodes.dedup();
    nodes
        .into_iter()
        .take(MAX_RENDER_NODES)
        .map(|(_, n)| n.clone())
        .collect()
}

/// The VA-API render devices in `dir` (normally `/dev/dri`).
pub fn render_nodes(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let entries: Vec<(String, bool)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let ft = std::fs::metadata(e.path()).ok()?.file_type();
            Some((name, is_char_device(&ft)))
        })
        .collect();
    select_render_nodes(&entries)
        .into_iter()
        .map(|n| dir.join(n))
        .collect()
}

#[cfg(unix)]
fn is_char_device(ft: &std::fs::FileType) -> bool {
    use std::os::unix::fs::FileTypeExt;
    ft.is_char_device()
}

#[cfg(not(unix))]
fn is_char_device(_: &std::fs::FileType) -> bool {
    false
}

/// The hardware methods this computer may have, best first (before the encoders of the
/// ffmpeg in use are looked at): the order of REQ-061.
pub fn methods(platform: &Platform) -> Vec<Candidate> {
    match platform.os {
        Os::Windows => vec![
            Candidate::new(Vendor::Nvenc),
            Candidate::new(Vendor::Qsv),
            Candidate::new(Vendor::Amf),
        ],
        Os::Linux => {
            let mut v = vec![Candidate::new(Vendor::Nvenc), Candidate::new(Vendor::Qsv)];
            v.extend(platform.render_nodes.iter().map(|d| Candidate {
                vendor: Vendor::Vaapi,
                device: Some(d.clone()),
                rc: RateControl::Bitrate,
            }));
            v
        }
        Os::MacOs => vec![Candidate::new(Vendor::VideoToolbox)],
        Os::Other => Vec::new(),
    }
}

/// The methods to try for `codec`, best first. Methods whose encoder is missing from
/// `encoders` (the ffmpeg in use was built without them) are skipped.
pub fn candidates(
    platform: &Platform,
    codec: Codec,
    encoders: &BTreeSet<String>,
) -> Vec<Candidate> {
    let mut out = methods(platform);
    out.retain(|c| encoders.contains(&c.encoder(codec)));
    out
}

/// What a hardware encode aims at: the normal method's bitrate and, for constant quality,
/// the quantiser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub bps: u64,
    /// Quantiser of [`RateControl::ConstantQuality`].
    pub qp: u32,
}

impl Target {
    /// `bps` with the first constant-quality quantiser ([`CQ_START_QP`]).
    pub fn new(bps: u64) -> Target {
        Target {
            bps,
            qp: CQ_START_QP,
        }
    }
}

/// First quantiser of constant quality. Measured on real videos with an Intel VA-API
/// driver (QG-05): at 23 the picture is never more than 3 VMAF points below the normal
/// method; files of detailed or low-bitrate sources can be too large, which
/// [`cq_retry_qp`] corrects.
pub const CQ_START_QP: u32 = 23;
/// Highest quantiser used.
pub const CQ_MAX_QP: u32 = 40;
/// A constant-quality run whose output grows faster than this many times the normal
/// method's bitrate is restarted once with a coarser quantiser.
pub const CQ_RETRY_RATIO: f64 = 1.4;

/// Size the second run aims at, in times the normal method's (below the 1.5× limit, but
/// not lower than needed: every quantiser step costs picture quality).
pub const CQ_RETRY_AIM: f64 = 1.3;

/// The quantiser for a second run when `qp` made `ratio` times the normal method's
/// bitrate: about 6 quantiser steps halve the size; aims at [`CQ_RETRY_AIM`].
pub fn cq_retry_qp(qp: u32, ratio: f64) -> u32 {
    if !ratio.is_finite() || ratio <= CQ_RETRY_AIM {
        return qp;
    }
    let up = (6.0 * (ratio / CQ_RETRY_AIM).log2()).ceil() as u32;
    (qp + up.max(1)).min(CQ_MAX_QP)
}

/// What a hardware export adds to and replaces in the export argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HwEncode {
    pub codec: Codec,
    pub vendor: Vendor,
    /// Global options (before the inputs): device setup.
    pub global: Vec<OsString>,
    /// Appended to the filter graph (after `[outv]`), e.g. the upload to the device.
    pub graph_tail: String,
    /// The video label to map (`[outv]` unless the tail renames it).
    pub out_label: String,
    /// Replaces the software `-c:v … -pix_fmt yuv420p` arguments.
    pub vcodec: Vec<OsString>,
}

/// The bitrate a hardware export of a preset aims at (the normal method uses constant
/// quality there): 8 Mb/s for H.264 at 1920x1080 and 25 fps, in proportion to the pixels
/// per second, less for the newer formats (HEVC 0.6, VP9 0.65, AV1 0.5 of it), between
/// 0.5 and 80 Mb/s.
pub fn preset_bitrate(codec: Codec, width: u32, height: u32, fps: crate::time::Fps) -> u64 {
    let pixels = width as f64 * height as f64 * fps.as_f64();
    let base = 8_000_000.0 * pixels / (1920.0 * 1080.0 * 25.0);
    let factor = match codec {
        Codec::H264 => 1.0,
        Codec::Hevc => 0.6,
        Codec::Vp9 => 0.65,
        Codec::Av1 => 0.5,
    };
    (base * factor).clamp(500_000.0, 80_000_000.0) as u64
}

/// GOP length of every method (the x264/x265 default; ffmpeg's own default of 12 makes
/// some hardware files much larger).
pub const GOP: u32 = 250;

/// The arguments of `cand` encoding `codec` at about `target_bps` (the normal method's
/// bitrate): target `T` scaled by the method's factor, cap `T×5/4`, buffer `2T`.
pub fn encode_args(cand: &Candidate, codec: Codec, target: Target) -> HwEncode {
    let v = cand.vendor;
    let t = (target.bps.max(1)).saturating_mul(v.bitrate_percent()) / 100;
    let m = t.saturating_mul(5) / 4;
    let buf = t.saturating_mul(2);
    let mut vcodec: Vec<String> = vec!["-c:v".into(), cand.encoder(codec)];
    let profile: Vec<String> = codec
        .profile()
        .map(|p| vec!["-profile:v".to_string(), p.to_string()])
        .unwrap_or_default();
    let gop = ["-g".to_string(), GOP.to_string()];
    if cand.rc == RateControl::ConstantQuality {
        match v {
            Vendor::Vaapi => {
                vcodec.extend(["-rc_mode".to_string(), "CQP".to_string()]);
                vcodec.extend(["-qp".to_string(), target.qp.to_string()]);
                vcodec.extend(gop);
                vcodec.extend(profile);
            }
            // Quick Sync's intelligent constant quality: a quality value and no bitrate
            _ => {
                vcodec.extend(["-preset".to_string(), "medium".to_string()]);
                vcodec.extend(["-global_quality".to_string(), target.qp.to_string()]);
                vcodec.extend(gop);
                vcodec.extend(profile);
                vcodec.extend(["-pix_fmt".to_string(), "nv12".to_string()]);
            }
        }
        return vaapi_encode(cand, codec, vcodec);
    }
    let extra: &[&str] = match v {
        Vendor::Nvenc => &[
            "-preset",
            "p5",
            "-tune",
            "hq",
            "-rc",
            "vbr",
            "-spatial_aq",
            "1",
        ],
        Vendor::Qsv => &["-preset", "medium"],
        Vendor::Amf => &[
            "-usage",
            "transcoding",
            "-quality",
            "quality",
            "-rc",
            "vbr_peak",
        ],
        Vendor::Vaapi => &["-rc_mode", "VBR"],
        Vendor::VideoToolbox => &["-allow_sw", "0", "-realtime", "0"],
    };
    vcodec.extend(extra.iter().map(|s| s.to_string()));
    vcodec.extend([
        "-b:v".to_string(),
        t.to_string(),
        "-maxrate".to_string(),
        m.to_string(),
        "-bufsize".to_string(),
        buf.to_string(),
    ]);
    vcodec.extend(gop);
    vcodec.extend(profile);
    match v {
        Vendor::Nvenc | Vendor::VideoToolbox => {
            vcodec.extend(["-pix_fmt".into(), "yuv420p".into()])
        }
        Vendor::Qsv | Vendor::Amf => vcodec.extend(["-pix_fmt".into(), "nv12".into()]),
        Vendor::Vaapi => {}
    }
    vaapi_encode(cand, codec, vcodec)
}

/// Adds the device setup (VA-API) to the video arguments `vcodec`.
fn vaapi_encode(cand: &Candidate, codec: Codec, vcodec: Vec<String>) -> HwEncode {
    let v = cand.vendor;
    let (global, graph_tail, out_label) = match (v, &cand.device) {
        (Vendor::Vaapi, Some(dev)) => {
            let mut init = OsString::from("vaapi=va:");
            init.push(dev.as_os_str());
            (
                vec![
                    OsString::from("-init_hw_device"),
                    init,
                    OsString::from("-filter_hw_device"),
                    OsString::from("va"),
                ],
                ";[outv]format=nv12,hwupload[outvhw]".to_string(),
                "[outvhw]".to_string(),
            )
        }
        (Vendor::Vaapi, None) => (
            argv(["-init_hw_device", "vaapi=va", "-filter_hw_device", "va"]),
            ";[outv]format=nv12,hwupload[outvhw]".to_string(),
            "[outvhw]".to_string(),
        ),
        _ => (Vec::new(), String::new(), "[outv]".to_string()),
    };
    HwEncode {
        codec,
        vendor: v,
        global,
        graph_tail,
        out_label,
        vcodec: argv(vcodec),
    }
}

/// Why a method did not pass the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// NVIDIA graphics driver older than the ffmpeg build needs.
    DriverTooOld,
    /// The system video library (libva) is older than this ffmpeg needs.
    LibvaTooOld,
    /// No such device or driver on this computer.
    NotAvailable,
    /// Busy (output or processor use) but not finished in time, even on its second try.
    TimedOut,
    /// No output and no processor use when its time ran out (REQ-210): skipped at once.
    Hanging,
    /// Ran out of time twice earlier in this session (REQ-209): not tried again.
    SkippedSlow,
    Failed,
}

impl Reason {
    pub fn text(self) -> &'static str {
        match self {
            Reason::DriverTooOld => "graphics driver too old (needs 610 or newer)",
            Reason::LibvaTooOld => "needs a newer system video library (libva 2.21 or newer)",
            Reason::NotAvailable => "not available on this computer",
            Reason::TimedOut => "did not answer in time",
            Reason::Hanging => "did not answer at all, skipped",
            Reason::SkippedSlow => "skipped: too slow twice",
            Reason::Failed => "did not work",
        }
    }
}

/// Reads why a probe failed from ffmpeg's error output.
pub fn reason_of(stderr: &str) -> Reason {
    let s = stderr.to_ascii_lowercase();
    if s.contains("driver does not support the required nvenc api version")
        || s.contains("minimum required nvidia driver")
    {
        return Reason::DriverTooOld;
    }
    const MISSING: &[&str] = &[
        "cannot load",
        "no capable devices found",
        "device creation failed",
        "failed to initialise vaapi",
        "failed to create",
        "no device available",
        "unsupported device",
        "not available",
    ];
    if MISSING.iter().any(|m| s.contains(m)) {
        Reason::NotAvailable
    } else {
        Reason::Failed
    }
}

/// The probe result: the working methods of each format (best first), why others did not
/// work, and which candidates were too slow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HwStatus {
    pub working: BTreeMap<Codec, Vec<Candidate>>,
    pub notes: Vec<(Codec, Vendor, Reason)>,
    /// Candidates that ran out of time twice (REQ-209): the app skips them for the rest of
    /// the session.
    pub slow: Vec<(Codec, Vendor)>,
    /// The words of REQ-217 when the system video library is too old.
    pub libva_note: Option<String>,
}

impl HwStatus {
    /// The method [`GraphicsChoice::Automatic`] uses for `codec`: the best one.
    pub fn get(&self, codec: Codec) -> Option<&Candidate> {
        self.working.get(&codec).and_then(|v| v.first())
    }

    /// The method for `codec` on the device the user chose (`None`: Automatic). A chosen
    /// device that is not in the working list (it went away, or it cannot make this format)
    /// gives nothing, so the normal method is used (REQ-216).
    pub fn chosen(&self, codec: Codec, device: Option<&str>) -> Option<&Candidate> {
        match device {
            None => self.get(codec),
            Some(name) => self.working.get(&codec)?.iter().find(|c| c.label() == name),
        }
    }

    /// The working devices by name, in the order of the check, each once (REQ-216).
    pub fn devices(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for list in self.working.values() {
            for c in list {
                let l = c.label();
                if !out.contains(&l) {
                    out.push(l);
                }
            }
        }
        out
    }

    /// No format has working hardware.
    pub fn none_found(&self) -> bool {
        self.working.values().all(Vec::is_empty)
    }

    /// Some format has no working hardware but one of its candidates did not answer in
    /// time (even after its retry): the answer may be different on a less busy computer.
    pub fn inconclusive(&self) -> bool {
        self.notes
            .iter()
            .any(|(codec, _, r)| *r == Reason::TimedOut && self.get(*codec).is_none())
    }
}

/// What an export may use (the probe result minus the methods turned off after a problem,
/// on the device the user chose).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HwOffer {
    pub found: BTreeMap<Codec, Candidate>,
}

impl HwOffer {
    /// An offer of one method for one format.
    pub fn one(codec: Codec, cand: Candidate) -> HwOffer {
        HwOffer {
            found: BTreeMap::from([(codec, cand)]),
        }
    }

    /// What Automatic offers: the best working method of each format.
    pub fn automatic(status: &HwStatus) -> HwOffer {
        HwOffer {
            found: Codec::ALL
                .into_iter()
                .filter_map(|c| Some((c, status.get(c)?.clone())))
                .collect(),
        }
    }

    pub fn get(&self, codec: Codec) -> Option<&Candidate> {
        self.found.get(&codec)
    }

    pub fn is_empty(&self) -> bool {
        self.found.is_empty()
    }
}

/// Time limits of the probe.
///
/// A try that runs out of time is not taken as "this hardware does not work": on a busy
/// computer a working encoder can take longer to start, so a candidate that was busy
/// (output or processor use) is tried once more, immediately (the preference order stays
/// the same), with the longer `retry` limit; one that did nothing at all is skipped (REQ-210).
/// `total` is the hard bound of the whole check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeLimits {
    /// First try of a candidate, from the start of its process (waiting for a slot does
    /// not count).
    pub per: Duration,
    /// The one extra try of a candidate whose first try ran out of time.
    pub retry: Duration,
    /// All tries together; untried candidates then count as not answering in time.
    pub total: Duration,
}

impl Default for ProbeLimits {
    fn default() -> Self {
        ProbeLimits {
            per: Duration::from_secs(1),
            retry: Duration::from_secs(4),
            total: Duration::from_secs(10),
        }
    }
}

/// The probe encode of `cand`: 8 synthetic 640x360 frames with exactly the arguments of a
/// real export, written nowhere. Its progress goes to stdout, so that output shows that the
/// try is alive (REQ-210).
pub fn probe_args(cand: &Candidate, codec: Codec) -> Vec<OsString> {
    let hw = encode_args(cand, codec, Target::new(2_000_000));
    let mut a = argv(["-hide_banner", "-nostdin", "-v", "error"]);
    a.extend(hw.global);
    a.extend(argv([
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=640x360:rate=30",
        "-frames:v",
        "8",
        "-filter_complex",
    ]));
    a.push(format!("[0:v]format=yuv420p[outv]{}", hw.graph_tail).into());
    a.extend(argv(["-map", hw.out_label.as_str()]));
    a.extend(hw.vcodec);
    a.extend(argv(["-f", "null", "-", "-progress", "pipe:1", "-nostats"]));
    a
}

/// Whether a try that ran out of time was still doing something (REQ-210).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// It wrote output or used processor time.
    Busy,
    /// Neither.
    Idle,
    /// No output, and the processor use cannot be read.
    Unknown,
}

/// Processor time (ms) a try must use to count as busy.
const BUSY_CPU_MS: u64 = 20;

/// The activity of a try from its output and its processor time at the start and now.
pub fn classify(output: bool, cpu: Option<(u64, u64)>) -> Activity {
    if output {
        return Activity::Busy;
    }
    match cpu {
        Some((a, b)) if b.saturating_sub(a) >= BUSY_CPU_MS => Activity::Busy,
        Some(_) => Activity::Idle,
        None => Activity::Unknown,
    }
}

/// Outcome of one timed run.
enum Run {
    Ok,
    Failed(String),
    TimedOut(Activity),
}

/// How often a try is looked at.
const POLL: Duration = Duration::from_millis(10);

/// Runs `args` for at most `limit` from the moment the process starts. Returns the
/// outcome and the time it ran.
fn timed_run(
    runner: &dyn Runner,
    ffmpeg: &Path,
    args: &[OsString],
    limit: Duration,
) -> (Run, Duration) {
    let mut p = match runner.spawn(ffmpeg, args) {
        Ok(p) => p,
        Err(e) => return (Run::Failed(e.to_string()), Duration::ZERO),
    };
    let started = Instant::now();
    let cpu0 = p.cpu_ms();
    // true: bytes seen; false: the output ended
    let (tx, rx) = std::sync::mpsc::channel::<bool>();
    if let Some(mut out) = p.take_stdout() {
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut total = 0usize;
            loop {
                match out.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        // the rest is read and dropped (bounded, as before)
                        if total < 64 * 1024 {
                            let _ = tx.send(true);
                        }
                        total += n;
                    }
                }
            }
            let _ = tx.send(false);
        });
    } else {
        let _ = tx.send(false);
    }
    let mut output = false;
    loop {
        let left = limit.saturating_sub(started.elapsed());
        if left.is_zero() {
            let cpu = cpu0.zip(p.cpu_ms());
            let activity = classify(output, cpu);
            p.kill();
            let _ = p.wait();
            return (Run::TimedOut(activity), started.elapsed());
        }
        match rx.recv_timeout(left.min(POLL)) {
            Ok(true) => output = true,
            Ok(false) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    let r = p.wait();
    let ran = started.elapsed();
    match r {
        Ok(o) if o.success => (Run::Ok, ran),
        Ok(o) => (Run::Failed(o.stderr), ran),
        Err(e) => (Run::Failed(e.to_string()), ran),
    }
}

/// Running time used by a probe, against [`ProbeLimits::total`].
struct Budget {
    limits: ProbeLimits,
    used: Duration,
}

/// What came of the tries of one candidate.
struct Attempt {
    result: Result<Candidate, Reason>,
    /// It ran out of time twice, or hung (REQ-209, REQ-210): skip it for the session.
    slow: bool,
}

impl Budget {
    fn left(&self) -> Duration {
        self.limits.total.saturating_sub(self.used)
    }

    /// Runs the probe of `cand`: a first try of at most `per`; if it runs out of time while
    /// busy (and was not cut short by the total budget), one more try of at most `retry`; if
    /// it ran out of time doing nothing it is skipped at once (REQ-210). `None` when
    /// nothing could be started (no time left, or cancelled).
    fn attempt(
        &mut self,
        runner: &dyn Runner,
        ffmpeg: &Path,
        cand: &Candidate,
        codec: Codec,
        cancel: &AtomicBool,
    ) -> Option<Attempt> {
        let args = probe_args(cand, codec);
        let mut outcome = None;
        for (n, limit) in [self.limits.per, self.limits.retry].into_iter().enumerate() {
            let left = self.left();
            if left.is_zero() || cancel.load(Ordering::SeqCst) {
                break;
            }
            let (run, ran) = timed_run(runner, ffmpeg, &args, limit.min(left));
            self.used += ran;
            let (result, again, slow) = match run {
                Run::Ok => (Ok(cand.clone()), false, false),
                Run::Failed(err) => (Err(reason_of(&err)), false, false),
                // doing nothing: no second try
                Run::TimedOut(Activity::Idle) => (Err(Reason::Hanging), false, true),
                // a time-out only because the whole check is out of time is final; a busy
                // candidate is not counted against the session (REQ-210), an unknown one
                // is, after its second time-out (REQ-209)
                Run::TimedOut(a) => (
                    Err(Reason::TimedOut),
                    n == 0 && limit <= left,
                    n == 1 && a == Activity::Unknown,
                ),
            };
            outcome = Some(Attempt { result, slow });
            if !again {
                break;
            }
        }
        outcome
    }
}

/// Tries the candidates of each format in order and keeps every one that works (the first is
/// what Automatic uses, REQ-216). A try that runs out of time while busy is repeated once
/// with a longer limit before the next candidate (see [`ProbeLimits`]); once
/// `limits.total` of running time is used up, the untried candidates count as not answering
/// in time. Candidates in `skip` ran out of time twice earlier in the session and are not
/// tried (REQ-209). Stops early when `cancel` is set.
pub fn probe(
    runner: &dyn Runner,
    ffmpeg: &Path,
    cands: &[(Codec, Vec<Candidate>)],
    limits: ProbeLimits,
    skip: &BTreeSet<(Codec, Vendor)>,
    cancel: &AtomicBool,
) -> HwStatus {
    let mut status = HwStatus::default();
    let mut budget = Budget {
        limits,
        used: Duration::ZERO,
    };
    for (codec, list) in cands {
        for c in list {
            if cancel.load(Ordering::SeqCst) {
                return status;
            }
            if skip.contains(&(*codec, c.vendor)) {
                status.notes.push((*codec, c.vendor, Reason::SkippedSlow));
                continue;
            }
            let Some(mut attempt) = budget.attempt(runner, ffmpeg, c, *codec, cancel) else {
                if cancel.load(Ordering::SeqCst) {
                    return status;
                }
                status.notes.push((*codec, c.vendor, Reason::TimedOut));
                continue;
            };
            // a driver that refuses bitrate control may still encode at constant quality
            if attempt.result == Err(Reason::Failed)
                && let Some(cq) = c.constant_quality()
                && let Some(r) = budget.attempt(runner, ffmpeg, &cq, *codec, cancel)
            {
                attempt = r;
            }
            if attempt.slow && !status.slow.contains(&(*codec, c.vendor)) {
                status.slow.push((*codec, c.vendor));
            }
            match attempt.result {
                Ok(found) => status.working.entry(*codec).or_default().push(found),
                Err(r) => status.notes.push((*codec, c.vendor, r)),
            }
        }
    }
    status
}

/// [`probe`] of every format with the candidates of `platform` and `encoders`.
pub fn probe_all(
    runner: &dyn Runner,
    ffmpeg: &Path,
    encoders: &BTreeSet<String>,
    platform: &Platform,
    limits: ProbeLimits,
    skip: &BTreeSet<(Codec, Vendor)>,
    cancel: &AtomicBool,
) -> HwStatus {
    let blocked = platform.os == Os::Linux
        && libva_too_old(ffmpeg_needs_new_libva(ffmpeg), platform.system_libva_new);
    let mut skipped = Vec::new();
    let cands: Vec<(Codec, Vec<Candidate>)> = Codec::ALL
        .iter()
        .map(|c| {
            let mut list = candidates(platform, *c, encoders);
            if blocked {
                list.retain(|k| {
                    let uses_libva = matches!(k.vendor, Vendor::Vaapi | Vendor::Qsv);
                    if uses_libva && !skipped.contains(&(*c, k.vendor, Reason::LibvaTooOld)) {
                        skipped.push((*c, k.vendor, Reason::LibvaTooOld));
                    }
                    !uses_libva
                });
            }
            (*c, list)
        })
        .collect();
    let mut status = probe(runner, ffmpeg, &cands, limits, skip, cancel);
    if !skipped.is_empty() {
        status.libva_note = Some(libva_message(platform.libva_version));
    }
    status.notes.extend(skipped);
    status
}

/// Whether the ffmpeg at `path` loads libva itself and needs libva 2.21+ (it names
/// [`LIBVA_NEW_SYMBOL`]). The answer is kept per file (path, size, time) so the 170 MB
/// bundled build is read once per session.
pub fn ffmpeg_needs_new_libva(path: &Path) -> bool {
    type Key = (PathBuf, u64, Option<std::time::SystemTime>);
    static SEEN: std::sync::Mutex<Vec<(Key, bool)>> = std::sync::Mutex::new(Vec::new());
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, v)) = seen.iter().find(|(k, _)| *k == key) {
        return *v;
    }
    let v = file_mentions(path, LIBVA_NEW_SYMBOL);
    seen.push((key, v));
    v
}

#[cfg(test)]
mod tests;
