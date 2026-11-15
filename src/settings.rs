// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Program settings (never stored in projects): the four plain-language choices, the
//! ffmpeg override and the machine they were recommended for, plus the window layout and
//! the recent projects (1.1, optional fields: 1.0 files load unchanged).
//!
//! Everything here is pure except [`load_file`] and [`save`]:
//! * [`startup`] decides, from the settings file's bytes and the detected machine, which
//!   settings to use, whether to tell the user and whether to write the file.
//! * [`recommend`] picks the recommended choices for a machine (deterministic).
//! * [`budget`] turns "How hard videolore works" into process and thread counts.
//! * [`availability`] says which subtitle levels this machine can run, and why not.
//! * [`summary`] describes what "Use recommended settings" changed.
//! * [`main_texts`] lists every text of the main Settings section (checked by a test for
//!   technical words).

use crate::hwenc::{Codec, Vendor};
use crate::machine::Fingerprint;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Current settings file format.
pub const VERSION: u32 = 2;
/// Settings files larger than this are not read.
pub const MAX_SETTINGS_BYTES: u64 = 64 * 1024;
/// Highest number of ffmpeg/ffprobe processes any level allows.
pub const MAX_PROCESS_CAP: u8 = 6;
/// The process cap of Light and Balanced (playback needs 2, plus 1 for an export).
pub const BASE_PROCESS_CAP: u8 = 3;

/// Preview quality: which picture the preview decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PreviewQuality {
    /// Small preview copies of big videos when this computer needs them.
    #[default]
    Automatic,
    /// Small preview copies of every video bigger than 1280x720.
    Smoother,
    /// Always the original files.
    Original,
}

/// Subtitle quality: which speech recognition file is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleQuality {
    Fast,
    #[default]
    Balanced,
    MostAccurate,
}

/// How hard videolore works: how much of the computer background work may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    Light,
    #[default]
    Balanced,
    Maximum,
}

/// Export: whether the computer's video hardware may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExportChoice {
    /// The normal method (the recommendation on every computer).
    #[default]
    BestQuality,
    /// The video hardware when it works, else the normal method.
    Faster,
}

impl ExportChoice {
    pub const ALL: [ExportChoice; 2] = [ExportChoice::Faster, ExportChoice::BestQuality];

    pub fn label(self) -> &'static str {
        match self {
            ExportChoice::Faster => "Faster",
            ExportChoice::BestQuality => "Best quality",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            ExportChoice::Faster => {
                "Uses your computer's video hardware when it can. Often quicker, but not on every computer; files may be a little larger."
            }
            ExportChoice::BestQuality => {
                "Uses the normal method. Slower, best picture for the file size."
            }
        }
    }
}

/// A hardware method turned off after a problem (REQ-064), until the computer or the
/// program version changes or the user asks to try again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareOff {
    pub format: Codec,
    pub method: Vendor,
    /// The videolore version that had the problem.
    pub version: String,
}

/// Most remembered problems.
pub const MAX_HARDWARE_OFF: usize = 8;

/// Reads `hardware_off` leniently: entries that are not understood are dropped one by
/// one (the rest of the file stays valid).
fn lenient_hardware_off<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<HardwareOff>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    let list = v.as_array().cloned().unwrap_or_default();
    let mut out: Vec<HardwareOff> = Vec::new();
    for item in list {
        if let Ok(h) = serde_json::from_value::<HardwareOff>(item)
            && !h.version.is_empty()
            && h.version.len() <= 32
            && !out.contains(&h)
            && out.len() < MAX_HARDWARE_OFF
        {
            out.push(h);
        }
    }
    Ok(out)
}

/// Section sizes of the window, as shares (REQ-066, REQ-067): `top` = top row of the
/// content height, `media` = Media of the top row's width, `timeline` = Timeline of the
/// bottom row's width. Only values strictly between 0 and 1 are valid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SavedLayout {
    pub top: f32,
    pub media: f32,
    pub timeline: f32,
}

impl SavedLayout {
    /// The REQ-024 layout: top row 2/3 (Media 30% | Preview 70%), bottom row 1/3
    /// (Timeline 70% | Effects 30%).
    pub const DEFAULT: SavedLayout = SavedLayout {
        top: 2.0 / 3.0,
        media: 0.30,
        timeline: 0.70,
    };

    /// Every share is a finite number strictly between 0 and 1.
    pub fn is_valid(&self) -> bool {
        [self.top, self.media, self.timeline]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v < 1.0)
    }
}

impl Default for SavedLayout {
    fn default() -> Self {
        SavedLayout::DEFAULT
    }
}

/// Most recent projects remembered (REQ-080).
pub const MAX_RECENT: usize = 10;
/// Longest remembered project path, in bytes.
pub const MAX_RECENT_PATH: usize = 1024;

/// Reads `layout` leniently: anything not understood or out of range is the default
/// layout (`None`); the rest of the file stays valid.
fn lenient_layout<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<SavedLayout>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(serde_json::from_value::<SavedLayout>(v)
        .ok()
        .filter(SavedLayout::is_valid))
}

/// A path that may be remembered: UTF-8 (anything else could not be saved), absolute, at
/// most [`MAX_RECENT_PATH`] bytes, without control characters.
pub fn valid_recent(p: &Path) -> bool {
    p.to_str().is_some_and(|s| {
        s.len() <= MAX_RECENT_PATH && !s.chars().any(char::is_control) && p.is_absolute()
    })
}

/// Reads `recent` leniently: entries that are not valid paths are dropped one by one,
/// duplicates removed, at most [`MAX_RECENT`] kept.
fn lenient_recent<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<PathBuf>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    let mut out: Vec<PathBuf> = Vec::new();
    for item in v.as_array().map(Vec::as_slice).unwrap_or_default() {
        if out.len() == MAX_RECENT {
            break;
        }
        if let Some(s) = item.as_str() {
            let p = PathBuf::from(s);
            if valid_recent(&p) && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

/// Puts `path` first in the recent projects (removing an older entry of it, keeping at
/// most [`MAX_RECENT`]). Returns whether the list changed. Invalid paths are ignored.
pub fn push_recent(list: &mut Vec<PathBuf>, path: &Path) -> bool {
    if !valid_recent(path) || list.first().is_some_and(|p| p == path) {
        return false;
    }
    list.retain(|p| p != path);
    list.insert(0, path.to_path_buf());
    list.truncate(MAX_RECENT);
    true
}

impl PreviewQuality {
    pub const ALL: [PreviewQuality; 3] = [
        PreviewQuality::Automatic,
        PreviewQuality::Smoother,
        PreviewQuality::Original,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PreviewQuality::Automatic => "Automatic",
            PreviewQuality::Smoother => "Smoother",
            PreviewQuality::Original => "Original",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            PreviewQuality::Automatic => {
                "Uses lighter copies of big videos only when this computer needs them."
            }
            PreviewQuality::Smoother => {
                "Plays every big video from a lighter copy: smoother, a little less sharp."
            }
            PreviewQuality::Original => {
                "Always plays your original files: sharpest, may stutter on big videos."
            }
        }
    }
}

impl SubtitleQuality {
    pub const ALL: [SubtitleQuality; 3] = [
        SubtitleQuality::Fast,
        SubtitleQuality::Balanced,
        SubtitleQuality::MostAccurate,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SubtitleQuality::Fast => "Fast",
            SubtitleQuality::Balanced => "Balanced",
            SubtitleQuality::MostAccurate => "Most accurate",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            SubtitleQuality::Fast => "Quickest, works everywhere; may miss some words.",
            SubtitleQuality::Balanced => "Fewer mistakes; takes longer. Downloads 190 MB once.",
            SubtitleQuality::MostAccurate => {
                "Best recognition; slowest. Downloads 574 MB once and needs plenty of memory."
            }
        }
    }

    /// Position in [`SubtitleQuality::ALL`].
    pub fn index(self) -> usize {
        match self {
            SubtitleQuality::Fast => 0,
            SubtitleQuality::Balanced => 1,
            SubtitleQuality::MostAccurate => 2,
        }
    }
}

impl Effort {
    pub const ALL: [Effort; 3] = [Effort::Light, Effort::Balanced, Effort::Maximum];

    pub fn label(self) -> &'static str {
        match self {
            Effort::Light => "Light",
            Effort::Balanced => "Balanced",
            Effort::Maximum => "Maximum speed",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            Effort::Light => "Leaves most of the computer free for other programs; slower.",
            Effort::Balanced => "Fast while keeping the computer responsive.",
            Effort::Maximum => "Finishes soonest; other programs may feel slower meanwhile.",
        }
    }
}

/// The four plain-language choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choices {
    pub preview: PreviewQuality,
    pub subtitles: SubtitleQuality,
    pub effort: Effort,
    pub export: ExportChoice,
}

/// The settings file (format 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub preview_quality: PreviewQuality,
    pub subtitle_quality: SubtitleQuality,
    pub effort: Effort,
    /// ffmpeg chosen by the user (absolute path), instead of the bundled or system one.
    #[serde(default)]
    pub ffmpeg: Option<PathBuf>,
    #[serde(default)]
    pub ffprobe: Option<PathBuf>,
    /// The machine the choices were made on.
    #[serde(default)]
    pub machine: Option<Fingerprint>,
    /// Export: Faster / Best quality (missing in files of 0.3.x: Best quality).
    #[serde(default)]
    pub export: ExportChoice,
    /// Hardware methods turned off after a problem on this computer.
    #[serde(default, deserialize_with = "lenient_hardware_off")]
    pub hardware_off: Vec<HardwareOff>,
    /// Section sizes (1.1); `None` = the default layout.
    #[serde(
        default,
        deserialize_with = "lenient_layout",
        skip_serializing_if = "Option::is_none"
    )]
    pub layout: Option<SavedLayout>,
    /// Recently opened or saved projects, most recent first (1.1).
    #[serde(default, deserialize_with = "lenient_recent")]
    pub recent: Vec<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings::with(Choices::default(), None)
    }
}

impl Settings {
    /// Settings with `choices` made on machine `fp`, no ffmpeg override.
    pub fn with(choices: Choices, fp: Option<Fingerprint>) -> Settings {
        Settings {
            version: VERSION,
            preview_quality: choices.preview,
            subtitle_quality: choices.subtitles,
            effort: choices.effort,
            ffmpeg: None,
            ffprobe: None,
            machine: fp,
            export: choices.export,
            hardware_off: Vec::new(),
            layout: None,
            recent: Vec::new(),
        }
    }

    pub fn choices(&self) -> Choices {
        Choices {
            preview: self.preview_quality,
            subtitles: self.subtitle_quality,
            effort: self.effort,
            export: self.export,
        }
    }

    pub fn set_choices(&mut self, c: Choices) {
        self.preview_quality = c.preview;
        self.subtitle_quality = c.subtitles;
        self.effort = c.effort;
        self.export = c.export;
    }

    /// The method is turned off for `codec` after a problem.
    pub fn hardware_is_off(&self, codec: Codec, method: Vendor) -> bool {
        self.hardware_off
            .iter()
            .any(|h| h.format == codec && h.method == method)
    }

    /// Remembers a problem of `method` with `codec` (program version `version`).
    pub fn turn_hardware_off(&mut self, codec: Codec, method: Vendor, version: &str) {
        if self.hardware_is_off(codec, method) {
            return;
        }
        if self.hardware_off.len() >= MAX_HARDWARE_OFF {
            self.hardware_off.remove(0);
        }
        self.hardware_off.push(HardwareOff {
            format: codec,
            method,
            version: version.to_string(),
        });
    }

    /// Default location: the platform config folder.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "videolore")
            .map(|d| d.config_dir().join("settings.json"))
    }

    /// The four choices are what [`recommend`] gives for `fp`.
    pub fn is_recommended(&self, fp: &Fingerprint) -> bool {
        self.choices() == recommend(fp)
    }
}

/// Settings of videolore 0.1.x / 0.2.x (no `version` field).
#[derive(Debug, Deserialize)]
struct Legacy {
    #[serde(default)]
    ffmpeg: Option<PathBuf>,
    #[serde(default)]
    ffprobe: Option<PathBuf>,
    #[serde(default = "yes")]
    proxies_enabled: bool,
}

fn yes() -> bool {
    true
}

/// GiB thresholds (memory as reported, rounded to the nearest GiB).
const MOST_ACCURATE_MIN_RAM_GIB: u64 = 7;
const BALANCED_MIN_RAM_GIB: u64 = 4;
const RECOMMEND_MOST_ACCURATE_RAM_GIB: u64 = 15;
const RECOMMEND_BALANCED_RAM_GIB: u64 = 7;
const MAXIMUM_MIN_RAM_GIB: u64 = 7;

/// Why a subtitle level cannot be used on this machine.
pub const TOO_OLD_REASON: &str = "This computer's processor is too old for automatic subtitles.";
pub const MEMORY_REASON: &str = "Needs more memory than this computer has.";

/// For each subtitle level (in [`SubtitleQuality::ALL`] order), `None` when it can run on
/// `fp`, else the plain reason it is greyed out. Unknown memory (0) greys nothing out.
pub fn availability(fp: &Fingerprint) -> [Option<&'static str>; 3] {
    if !fp.speech_supported {
        return [Some(TOO_OLD_REASON); 3];
    }
    let ram = fp.ram_gib;
    let short = |min: u64| (ram != 0 && ram < min).then_some(MEMORY_REASON);
    [
        None,
        short(BALANCED_MIN_RAM_GIB),
        short(MOST_ACCURATE_MIN_RAM_GIB),
    ]
}

/// The recommended choices for a machine (pure and deterministic).
pub fn recommend(fp: &Fingerprint) -> Choices {
    let l = fp.logical;
    let ram = fp.ram_gib;
    let subtitles = if !fp.speech_supported {
        SubtitleQuality::Fast
    } else if l >= 12 && ram >= RECOMMEND_MOST_ACCURATE_RAM_GIB {
        SubtitleQuality::MostAccurate
    } else if l >= 4 && ram >= RECOMMEND_BALANCED_RAM_GIB {
        SubtitleQuality::Balanced
    } else {
        SubtitleQuality::Fast
    };
    let available = availability(fp);
    let subtitles = if available[subtitles.index()].is_none() {
        subtitles
    } else {
        SubtitleQuality::Fast
    };
    Choices {
        preview: PreviewQuality::Automatic,
        subtitles,
        effort: Effort::Balanced,
        // Best quality everywhere: Faster is the user's opt-in (REQ-060).
        export: ExportChoice::BestQuality,
    }
}

/// What a level of "How hard videolore works" allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// ffmpeg/ffprobe processes running at once.
    pub max_processes: u8,
    /// Threads of each export and preview-copy ffmpeg (`None`: ffmpeg decides).
    pub threads: Option<u32>,
    /// Speech recognition threads.
    pub stt_threads: usize,
}

/// The documented mapping from "How hard videolore works" to resources.
///
/// | Resource | Light | Balanced | Maximum |
/// |---|---|---|---|
/// | processes | 3 | 3 | clamp(L/2, 3, 6); 3 below 7 GiB |
/// | export/preview-copy threads | max(1, L/2) | automatic | automatic |
/// | speech threads | clamp(min(P, max(1, L/2)), 1, 8) | clamp(min(P, L-1), 1, 8), at least clamp(L/2, 1, 4) | clamp(P, 1, 16) |
pub fn budget(effort: Effort, fp: &Fingerprint) -> Budget {
    let l = fp.logical.max(1);
    let p = fp.physical.clamp(1, l);
    let half = (l / 2).max(1);
    match effort {
        Effort::Light => Budget {
            max_processes: BASE_PROCESS_CAP,
            threads: Some(half),
            stt_threads: p.min(half).clamp(1, 8) as usize,
        },
        Effort::Balanced => {
            // never fewer speech threads than 0.2.2 used: clamp(L/2, 1, 4)
            let old = (l / 2).clamp(1, 4);
            let t = p.min((l - 1).max(1)).clamp(1, 8).max(old).min(l);
            Budget {
                max_processes: BASE_PROCESS_CAP,
                threads: None,
                stt_threads: t as usize,
            }
        }
        Effort::Maximum => {
            let cap = if fp.ram_gib != 0 && fp.ram_gib < MAXIMUM_MIN_RAM_GIB {
                BASE_PROCESS_CAP
            } else {
                (l / 2).clamp(u32::from(BASE_PROCESS_CAP), u32::from(MAX_PROCESS_CAP)) as u8
            };
            Budget {
                max_processes: cap,
                threads: None,
                stt_threads: p.clamp(1, 16) as usize,
            }
        }
    }
}

/// Plain description of what changed between `old` and `new`.
pub fn summary(old: Choices, new: Choices) -> Vec<String> {
    let mut out = Vec::new();
    if old.preview != new.preview {
        out.push(format!(
            "Preview quality: {} → {}",
            old.preview.label(),
            new.preview.label()
        ));
    }
    if old.subtitles != new.subtitles {
        out.push(format!(
            "Subtitle quality: {} → {}",
            old.subtitles.label(),
            new.subtitles.label()
        ));
    }
    if old.effort != new.effort {
        out.push(format!(
            "How hard videolore works: {} → {}",
            old.effort.label(),
            new.effort.label()
        ));
    }
    if old.export != new.export {
        out.push(format!(
            "{EXPORT_TITLE}: {} → {}",
            old.export.label(),
            new.export.label()
        ));
    }
    if out.is_empty() {
        out.push(ALREADY_RECOMMENDED.to_string());
    }
    out
}

pub const SETTINGS_TITLE: &str = "Settings";
pub const PREVIEW_TITLE: &str = "Preview quality";
pub const SUBTITLE_TITLE: &str = "Subtitle quality";
pub const EFFORT_TITLE: &str = "How hard videolore works";
pub const EXPORT_TITLE: &str = "Export";
/// Added to Faster's explanation when the check found no usable hardware.
pub const NO_HARDWARE_NOTE: &str = "This computer has no video hardware videolore can use, so \
                                    both choices give the same result.";
pub const RECOMMENDED_STATUS: &str = "Using recommended settings for this computer";
pub const CUSTOM_STATUS: &str = "Custom settings";
pub const USE_RECOMMENDED: &str = "Use recommended settings";
pub const ALREADY_RECOMMENDED: &str = "Already using the recommended settings";
pub const PREVIEW_COPIES: &str = "Preview copies";
pub const CLEAR_PREVIEW_COPIES: &str = "Clear preview copies";
pub const SPEECH_FILES: &str = "Speech recognition files";
pub const REMOVE_SPEECH_FILES: &str = "Remove downloaded speech files";
pub const SLOWER_NOTE: &str = "Subtitles cannot be generated automatically on this computer; \
                               you can still add subtitle lines by hand.";
pub const ADVANCED_TITLE: &str = "Advanced";
pub const CLOSE: &str = "Close";

/// Notices shown at start-up.
pub const SETUP_NOTICE: &str =
    "videolore has been set up for this computer. You can change this in Settings.";
pub const MACHINE_CHANGED_NOTICE: &str = "This computer is different from the one videolore was \
     set up on. Your own settings were kept.";
pub const MALFORMED_NOTICE: &str = "Your settings could not be read, so videolore now uses the \
     recommended settings for this computer.";

/// Every fixed text of the main Settings section (not "Advanced").
pub fn main_texts() -> Vec<&'static str> {
    let mut t = vec![
        SETTINGS_TITLE,
        PREVIEW_TITLE,
        SUBTITLE_TITLE,
        EFFORT_TITLE,
        EXPORT_TITLE,
        NO_HARDWARE_NOTE,
        RECOMMENDED_STATUS,
        CUSTOM_STATUS,
        USE_RECOMMENDED,
        ALREADY_RECOMMENDED,
        PREVIEW_COPIES,
        CLEAR_PREVIEW_COPIES,
        SPEECH_FILES,
        REMOVE_SPEECH_FILES,
        TOO_OLD_REASON,
        MEMORY_REASON,
        SLOWER_NOTE,
        ADVANCED_TITLE,
        CLOSE,
    ];
    for p in PreviewQuality::ALL {
        t.extend([p.label(), p.explanation()]);
    }
    for s in SubtitleQuality::ALL {
        t.extend([s.label(), s.explanation()]);
    }
    for e in Effort::ALL {
        t.extend([e.label(), e.explanation()]);
    }
    for e in ExportChoice::ALL {
        t.extend([e.label(), e.explanation()]);
    }
    t
}

/// What was found where the settings file should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    /// No settings file: first start.
    Missing,
    /// The file's bytes (at most [`MAX_SETTINGS_BYTES`]).
    Bytes(Vec<u8>),
    /// Something unusable: not a regular file, too big, unreadable.
    Unusable,
}

/// Reads the settings file safely: only regular files, at most [`MAX_SETTINGS_BYTES`].
pub fn load_file(path: &Path) -> Loaded {
    match crate::project::read_limited(path, MAX_SETTINGS_BYTES) {
        Ok(bytes) => Loaded::Bytes(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Loaded::Missing,
        Err(_) => Loaded::Unusable,
    }
}

/// Why the start-up message is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartNotice {
    /// First start (or upgrade, or recommended settings recomputed on a new machine).
    Setup,
    /// A different machine, custom settings kept: offer "Use recommended settings".
    MachineChanged,
    /// The file could not be read: recommended settings are used.
    Malformed,
}

impl StartNotice {
    pub fn text(self) -> &'static str {
        match self {
            StartNotice::Setup => SETUP_NOTICE,
            StartNotice::MachineChanged => MACHINE_CHANGED_NOTICE,
            StartNotice::Malformed => MALFORMED_NOTICE,
        }
    }
}

/// The start-up decision.
#[derive(Debug, Clone, PartialEq)]
pub struct Startup {
    pub settings: Settings,
    pub notice: Option<StartNotice>,
    /// Save `settings` now.
    pub write: bool,
    /// Keep a copy of the unreadable file as `settings.json.bad` first.
    pub backup: bool,
}

fn valid_fingerprint(fp: &Fingerprint) -> bool {
    (1..=4096).contains(&fp.logical)
        && (1..=4096).contains(&fp.physical)
        && fp.ram_gib <= 1 << 20
        && fp.os.len() <= 32
        && fp.arch.len() <= 32
}

/// An override is kept only when it is an absolute path.
fn valid_override(p: &Option<PathBuf>) -> bool {
    p.as_ref().is_none_or(|p| p.is_absolute())
}

/// Decides the settings to use at start-up (pure). `fp` is the detected machine.
pub fn startup(loaded: &Loaded, fp: &Fingerprint) -> Startup {
    startup_at(loaded, fp, env!("CARGO_PKG_VERSION"))
}

/// [`startup`] for program version `version`: hardware problems remembered by another
/// version are forgotten.
pub fn startup_at(loaded: &Loaded, fp: &Fingerprint, version: &str) -> Startup {
    let fresh = |notice| Startup {
        settings: Settings::with(recommend(fp), Some(fp.clone())),
        notice: Some(notice),
        write: true,
        backup: false,
    };
    let bytes = match loaded {
        Loaded::Missing => return fresh(StartNotice::Setup),
        Loaded::Unusable => return fresh(StartNotice::Malformed),
        Loaded::Bytes(b) => b,
    };
    let malformed = || Startup {
        backup: true,
        ..fresh(StartNotice::Malformed)
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return malformed();
    };
    if !value.is_object() {
        return malformed();
    }
    match value.get("version") {
        None => legacy(value, fp).unwrap_or_else(malformed),
        Some(v) => match v.as_u64() {
            Some(n) if n > u64::from(VERSION) => Startup {
                // made by a newer videolore: use recommended values, leave the file alone
                settings: Settings::with(recommend(fp), Some(fp.clone())),
                notice: None,
                write: false,
                backup: false,
            },
            Some(n) if n == u64::from(VERSION) => {
                current(value, fp, version).unwrap_or_else(malformed)
            }
            _ => malformed(),
        },
    }
}

/// Upgrade from 0.1.x / 0.2.x: first start, but proxies off become Original and the ffmpeg
/// override is kept.
fn legacy(value: serde_json::Value, fp: &Fingerprint) -> Option<Startup> {
    let old: Legacy = serde_json::from_value(value).ok()?;
    let mut choices = recommend(fp);
    if !old.proxies_enabled {
        choices.preview = PreviewQuality::Original;
    }
    let mut settings = Settings::with(choices, Some(fp.clone()));
    let mut notice = StartNotice::Setup;
    if valid_override(&old.ffmpeg) && valid_override(&old.ffprobe) {
        settings.ffmpeg = old.ffmpeg;
        settings.ffprobe = old.ffprobe;
    } else {
        notice = StartNotice::Malformed;
    }
    Some(Startup {
        settings,
        notice: Some(notice),
        write: true,
        backup: false,
    })
}

fn current(value: serde_json::Value, fp: &Fingerprint, version: &str) -> Option<Startup> {
    let mut s: Settings = serde_json::from_value(value).ok()?;
    if s.machine.as_ref().is_some_and(|m| !valid_fingerprint(m)) {
        return None;
    }
    let mut notice = None;
    let mut write = false;
    let remembered = s.hardware_off.len();
    s.hardware_off.retain(|h| h.version == version);
    if s.hardware_off.len() != remembered {
        write = true;
    }
    if !valid_override(&s.ffmpeg) || !valid_override(&s.ffprobe) {
        s.ffmpeg = None;
        s.ffprobe = None;
        notice = Some(StartNotice::Malformed);
        write = true;
    }
    if s.machine.as_ref() != Some(fp) {
        let was_recommended = s
            .machine
            .as_ref()
            .is_some_and(|old| s.choices() == recommend(old));
        if was_recommended {
            s.set_choices(recommend(fp));
            notice = notice.or(Some(StartNotice::Setup));
        } else {
            notice = notice.or(Some(StartNotice::MachineChanged));
        }
        // a different computer: its video hardware is tried again
        s.hardware_off.clear();
        s.machine = Some(fp.clone());
        write = true;
    }
    Some(Startup {
        settings: s,
        notice,
        write,
        backup: false,
    })
}

/// Saves the settings atomically (creating the folder).
pub fn save(settings: &Settings, path: &Path) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    crate::project::write_atomic(path, json.as_bytes()).map_err(|e| e.to_string())
}

/// Where an unreadable settings file is copied before being replaced.
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bad");
    path.with_file_name(name)
}

/// Copies the unreadable bytes to [`backup_path`] (overwriting an older copy).
pub fn backup(path: &Path, loaded: &Loaded) {
    if let Loaded::Bytes(b) = loaded {
        let _ = crate::project::write_atomic(&backup_path(path), b);
    }
}

/// Reads at most `limit` bytes from `r`; errors when there is more.
pub fn read_capped(mut r: impl Read, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    r.by_ref().take(limit + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file too large",
        ));
    }
    Ok(buf)
}

#[cfg(test)]
mod tests;
