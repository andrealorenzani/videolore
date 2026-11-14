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
//! * [`probe`] tries the candidates with a tiny synthetic encode (at most 1 s each, 3 s in
//!   total) and keeps the first one that works for each format.
//!
//! Everything is pure except [`probe`] (runs ffmpeg through a [`Runner`]) and
//! [`render_nodes`] (lists `/dev/dri`). The VA-API device path only ever comes from
//! enumerated `renderD<number>` character devices; there is never a shell.

use crate::ffmpeg::{Runner, argv};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The two formats that may use video hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    H264,
    Hevc,
}

impl Codec {
    pub const ALL: [Codec; 2] = [Codec::H264, Codec::Hevc];

    /// ffmpeg/ffprobe codec name.
    pub fn name(self) -> &'static str {
        match self {
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
        }
    }

    /// The codec of an ffprobe codec name, when it may use hardware.
    pub fn from_name(name: &str) -> Option<Codec> {
        match name {
            "h264" => Some(Codec::H264),
            "hevc" => Some(Codec::Hevc),
            _ => None,
        }
    }

    /// The codec the normal method encodes with `encoder`, when it may use hardware.
    pub fn of_software_encoder(encoder: &str) -> Option<Codec> {
        match encoder {
            "libx264" => Some(Codec::H264),
            "libx265" => Some(Codec::Hevc),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::Hevc => "HEVC",
        }
    }

    /// The profile the normal method produces (x264 "veryfast" High, x265 Main).
    pub fn profile(self) -> &'static str {
        match self {
            Codec::H264 => "high",
            Codec::Hevc => "main",
        }
    }

    /// The profile name ffprobe reports for [`Codec::profile`].
    pub fn probed_profile(self) -> &'static str {
        match self {
            Codec::H264 => "High",
            Codec::Hevc => "Main",
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
            Vendor::Qsv => &["-preset"],
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
    /// VA-API drivers that refuse bitrate control.
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

    /// The same method with constant quality, when its driver may refuse bitrate control.
    pub fn constant_quality(&self) -> Option<Candidate> {
        (self.vendor == Vendor::Vaapi && self.rc == RateControl::Bitrate).then(|| Candidate {
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
        let system_libva_new = if os == Os::Linux {
            find_system_libva().map(|l| file_mentions(&l, LIBVA_NEW_SYMBOL))
        } else {
            None
        };
        Platform {
            os,
            render_nodes,
            system_libva_new,
        }
    }

    /// No hardware methods at all.
    pub fn none() -> Platform {
        Platform {
            os: Os::Other,
            render_nodes: Vec::new(),
            system_libva_new: None,
        }
    }
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

/// The methods to try for `codec`, best first. Methods whose encoder is missing from
/// `encoders` (the ffmpeg in use was built without them) are skipped.
pub fn candidates(
    platform: &Platform,
    codec: Codec,
    encoders: &BTreeSet<String>,
) -> Vec<Candidate> {
    let mut out = match platform.os {
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
    };
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
    if v == Vendor::Vaapi && cand.rc == RateControl::ConstantQuality {
        vcodec.extend([
            "-rc_mode".to_string(),
            "CQP".to_string(),
            "-qp".to_string(),
            target.qp.to_string(),
            "-g".to_string(),
            GOP.to_string(),
            "-profile:v".to_string(),
            codec.profile().to_string(),
        ]);
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
        "-g".to_string(),
        GOP.to_string(),
        "-profile:v".to_string(),
        codec.profile().to_string(),
    ]);
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
    TimedOut,
    Failed,
}

impl Reason {
    pub fn text(self) -> &'static str {
        match self {
            Reason::DriverTooOld => "graphics driver too old (needs 610 or newer)",
            Reason::LibvaTooOld => "needs a newer system video library (libva 2.21 or newer)",
            Reason::NotAvailable => "not available on this computer",
            Reason::TimedOut => "did not answer in time",
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

/// The probe result: the working method of each format, and why others did not work.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HwStatus {
    pub h264: Option<Candidate>,
    pub hevc: Option<Candidate>,
    pub notes: Vec<(Codec, Vendor, Reason)>,
}

impl HwStatus {
    pub fn get(&self, codec: Codec) -> Option<&Candidate> {
        match codec {
            Codec::H264 => self.h264.as_ref(),
            Codec::Hevc => self.hevc.as_ref(),
        }
    }

    fn set(&mut self, codec: Codec, c: Option<Candidate>) {
        match codec {
            Codec::H264 => self.h264 = c,
            Codec::Hevc => self.hevc = c,
        }
    }

    /// No format has working hardware.
    pub fn none_found(&self) -> bool {
        self.h264.is_none() && self.hevc.is_none()
    }
}

/// What an export may use (the probe result minus the methods turned off after a problem).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HwOffer {
    pub h264: Option<Candidate>,
    pub hevc: Option<Candidate>,
}

impl HwOffer {
    pub fn get(&self, codec: Codec) -> Option<&Candidate> {
        match codec {
            Codec::H264 => self.h264.as_ref(),
            Codec::Hevc => self.hevc.as_ref(),
        }
    }
}

/// Time limits of the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeLimits {
    /// Per candidate, from the start of its process (waiting for a slot does not count).
    pub per: Duration,
    /// All candidates together; untried ones then count as not working.
    pub total: Duration,
}

impl Default for ProbeLimits {
    fn default() -> Self {
        ProbeLimits {
            per: Duration::from_secs(1),
            total: Duration::from_secs(3),
        }
    }
}

/// The probe encode of `cand`: 8 synthetic 640x360 frames with exactly the arguments of a
/// real export, written nowhere.
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
    a.extend(argv(["-f", "null", "-"]));
    a
}

/// Outcome of one timed run.
enum Run {
    Ok,
    Failed(String),
    TimedOut,
}

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
    let (tx, rx) = std::sync::mpsc::channel();
    if let Some(mut out) = p.take_stdout() {
        std::thread::spawn(move || {
            let mut sink = Vec::new();
            let _ = out.by_ref().take(64 * 1024).read_to_end(&mut sink);
            let _ = tx.send(());
        });
    } else {
        let _ = tx.send(());
    }
    if rx.recv_timeout(limit).is_err() {
        p.kill();
        let _ = p.wait();
        return (Run::TimedOut, started.elapsed());
    }
    let r = p.wait();
    let ran = started.elapsed();
    match r {
        Ok(o) if o.success => (Run::Ok, ran),
        Ok(o) => (Run::Failed(o.stderr), ran),
        Err(e) => (Run::Failed(e.to_string()), ran),
    }
}

/// Tries the candidates of each format in order and keeps the first that works. Each try
/// runs at most `limits.per`; once `limits.total` of running time is used up, the untried
/// candidates count as not working. Stops early when `cancel` is set.
pub fn probe(
    runner: &dyn Runner,
    ffmpeg: &Path,
    cands: &[(Codec, Vec<Candidate>)],
    limits: ProbeLimits,
    cancel: &AtomicBool,
) -> HwStatus {
    let mut status = HwStatus::default();
    let mut used = Duration::ZERO;
    for (codec, list) in cands {
        for c in list {
            if cancel.load(Ordering::SeqCst) {
                return status;
            }
            let left = limits.total.saturating_sub(used);
            if left.is_zero() {
                status.notes.push((*codec, c.vendor, Reason::TimedOut));
                continue;
            }
            let (run, ran) =
                timed_run(runner, ffmpeg, &probe_args(c, *codec), limits.per.min(left));
            used += ran;
            let mut outcome = match run {
                Run::Ok => Ok(c.clone()),
                Run::Failed(err) => Err(reason_of(&err)),
                Run::TimedOut => Err(Reason::TimedOut),
            };
            // a driver that refuses bitrate control may still encode at constant quality
            if outcome == Err(Reason::Failed)
                && let Some(cq) = c.constant_quality()
            {
                let left = limits.total.saturating_sub(used);
                if !left.is_zero() && !cancel.load(Ordering::SeqCst) {
                    let (run, ran) = timed_run(
                        runner,
                        ffmpeg,
                        &probe_args(&cq, *codec),
                        limits.per.min(left),
                    );
                    used += ran;
                    outcome = match run {
                        Run::Ok => Ok(cq),
                        Run::Failed(err) => Err(reason_of(&err)),
                        Run::TimedOut => Err(Reason::TimedOut),
                    };
                }
            }
            match outcome {
                Ok(found) => {
                    status.set(*codec, Some(found));
                    break;
                }
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
    let mut status = probe(runner, ffmpeg, &cands, limits, cancel);
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
