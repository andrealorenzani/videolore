// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Turns the project into ffmpeg work: the export [`Plan`] (stream copy or one re-encode
//! pass with a single `filter_complex`) and the argv for export and preview.
//!
//! Inputs are grouped in *runs*: consecutive clips taken from the same file going forward
//! in time share one input (`-ss <run start> -t <run length> -i file:<path>`), split with
//! `split`/`asplit` and trimmed per clip. A new input is opened only when the order goes
//! back in time, so 200 cuts from one long file use one decoder and memory stays bounded.
//!
//! Per clip: trim -> setpts -> scale (keep aspect) -> pad (letterbox) -> setsar -> fps ->
//! format -> fades; audio: atrim -> aresample -> aformat -> apad -> afades (or silence when
//! the file has no audio); then one concat. After the concat: burned-in subtitles
//! (`subtitles` filter on an ASS file in the working folder) and the audio mix (video sound
//! and music bed with their volumes, `amix` + `alimiter`).

use crate::ass;
use crate::camera::{self, Camera, Pov};
use crate::ffmpeg::{FfVersion, argv, file_arg, filter_graph_args};
use crate::format::{self, EncodeSettings};
use crate::frame;
use crate::hwenc::{Codec, HwEncode};
use crate::media::{MediaFormat, MediaId, MediaItem};
use crate::music::{END_FADE, Mix};
use crate::project::Project;
use crate::subtitles::Preset;
use crate::time::{Fps, Micros};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// Height of the preview video.
pub const PREVIEW_HEIGHT: u32 = 360;

/// Sample rate used inside the graphs.
pub const SAMPLE_RATE: u32 = 48_000;

/// Clips of the same file further apart than this get their own input (seeking is
/// cheaper than decoding the gap).
pub const MAX_RUN_GAP: Micros = Micros(30_000_000);

/// The Clean audio filter (REQ-114, ADR-080): FFT noise reduction of 25 dB with a fixed
/// noise floor of -20 dB and a residual floor of -60 dB. The noise tracking of the filter
/// (`tn=1`) is not used: its result changed by 4 dB with the very start of the stream, the
/// fixed floor is the same every time. One place for the constants, tuned on the fixture of
/// the integration test (noise-only parts 9 dB lower, speech within 0.2 dB).
/// Normalise speech (REQ-279): quiet words are raised and loud ones kept, with the bundled
/// ffmpeg's `speechnorm` (the permissive branch of requirement 279: no model file is needed).
pub const NORMALISE_SPEECH: &str = "speechnorm=e=6.25:r=0.0001:l=1";
pub const CLEAN_AUDIO: &str = "afftdn=nr=25:nf=-20:tn=0:rf=-60";

const AFORMAT: &str = "aformat=sample_fmts=fltp:channel_layouts=stereo";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RenderError {
    #[error("the timeline is empty")]
    Empty,
    #[error("source '{0}' is missing")]
    Missing(String),
    #[error("a clip refers to an unknown source")]
    UnknownMedia,
    #[error("This 360° video is too large to show here. It is still exported.")]
    SphereTooLarge,
    #[error("The colour LUT file '{0}' cannot be used: {1}")]
    Lut(String, String),
    #[error("The picture file '{0}' is missing")]
    Picture(String),
    #[error("This ffmpeg cannot make the chosen format")]
    NoEncoder,
    #[error(
        "A 360 video needs every clip to be a 360° video. Choose the flat camera view instead."
    )]
    MixedSphere,
}

mod args;
mod graph;
mod inputs;
pub mod overlay;
mod plan;
mod segments;
pub mod steps;

pub use self::args::*;
pub use self::graph::*;
pub use self::inputs::*;
pub use self::plan::*;
pub use self::segments::*;

#[cfg(test)]
pub(crate) mod golden_tests;
#[cfg(test)]
mod overlap_tests;
#[cfg(test)]
mod overlay_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod v31_tests;
