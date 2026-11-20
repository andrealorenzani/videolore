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
    #[error("This ffmpeg cannot make the chosen format")]
    NoEncoder,
    #[error(
        "A 360 video needs every clip to be a 360° video. Choose the flat camera view instead."
    )]
    MixedSphere,
}

/// What Export makes of a project with 360° clips (REQ-103).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportMode {
    /// The usual choice: a flat video (the camera view of 360° clips).
    #[default]
    Automatic,
    /// A 360° video: the whole sphere, in equirectangular form.
    Sphere360,
    /// The flat camera view of 1.2.6 (the same as `Automatic`).
    FlatCamera,
}

/// Who reads the files: the export always uses the originals; the preview may use proxies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Export,
    Preview { proxies: bool },
}

impl Source {
    /// The file to read for `m`.
    pub fn path_of(self, m: &MediaItem) -> PathBuf {
        match (self, m.proxy_path()) {
            (Source::Preview { proxies: true }, Some(p)) => p.to_path_buf(),
            _ => m.path.clone(),
        }
    }
}

/// One clip as seen by the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub media: MediaId,
    pub path: PathBuf,
    pub has_audio: bool,
    pub src_in: Micros,
    /// Full clip length (fades are positioned against it).
    pub len: Micros,
    /// How much of the start of the clip to skip (preview starting mid-clip).
    pub skip: Micros,
    pub fade_in: Micros,
    pub fade_out: Micros,
    /// Linear gain of the clip's own sound (1.0 = unchanged).
    pub gain: f64,
    /// Clean audio is on for this clip (REQ-114): applied before the gain and the fades.
    pub clean: bool,
    /// The fade-out is the one at the very end of the video: its last frame is black.
    pub to_black: bool,
    /// A 360 source (1.2.6): the points of view of its camera path (source time). Empty:
    /// the camera stays at the front. `None`: a normal clip.
    pub sphere: Option<Vec<Pov>>,
    /// The ASS file of the texts fixed to the sphere that are on this 360 clip, drawn on
    /// the equirectangular picture before the camera view is cut out (1.3.0).
    pub anchored: Option<String>,
    /// Change aspect, Scale and Move that act on this clip (1.5.0); `None`: the clip is
    /// drawn as before (fitted to the output picture with black bars).
    pub frame: Option<SegFrame>,
    /// Apply LUT of this clip (1.6.0, REQ-135; a whole-video LUT is one copy per clip since
    /// 1.6.1). A LUT that cannot be read is ignored in a preview and stops an export.
    pub lut_clip: Option<std::sync::Arc<crate::lut::Lut>>,
}

/// The LUT of `target` for a render: `None` without one; a file that cannot be used is an
/// error for an export (REQ-136: nothing is written) and ignored for a preview.
fn lut_for(
    tl: &crate::timeline::Timeline,
    target: crate::timeline::FrameTarget,
    src: Source,
) -> Result<Option<std::sync::Arc<crate::lut::Lut>>, RenderError> {
    let Some(path) = tl.lut_of(target) else {
        return Ok(None);
    };
    match crate::lut::cached(path) {
        Ok(l) => Ok(Some(l)),
        Err(_) if matches!(src, Source::Preview { .. }) => Ok(None),
        Err(e) => Err(RenderError::Lut(
            crate::lut::display_name(path),
            e.to_string(),
        )),
    }
}

/// The picture effects of one clip (1.5.0, REQ-131).
#[derive(Debug, Clone, PartialEq)]
pub struct SegFrame {
    pub aspect: Option<frame::FrameAspect>,
    pub stack: frame::Stack,
}

impl Segment {
    /// Length actually rendered.
    pub fn rendered(&self) -> Micros {
        self.len - self.skip
    }
}

/// Clips from timeline position `from` onwards, with fades resolved.
pub fn segments(project: &Project, from: Micros, src: Source) -> Result<Vec<Segment>, RenderError> {
    let tl = &project.timeline;
    if tl.clips.is_empty() {
        return Err(RenderError::Empty);
    }
    let mut out = Vec::new();
    let mut start = Micros::ZERO;
    for (i, c) in tl.clips.iter().enumerate() {
        let end = start + c.len();
        if end > from {
            let m = project.media(c.media).ok_or(RenderError::UnknownMedia)?;
            if m.missing {
                return Err(RenderError::Missing(m.name.clone()));
            }
            if matches!(src, Source::Preview { .. })
                && m.format.is_360()
                && crate::proxy::sphere_too_large(m.format.width, m.format.height)
            {
                return Err(RenderError::SphereTooLarge);
            }
            let (fade_in, fade_out) = tl.fades_of(i);
            let lut_clip = lut_for(tl, crate::timeline::FrameTarget::Clip(c.id), src)?;
            out.push(Segment {
                lut_clip,
                media: m.id,
                path: src.path_of(m),
                has_audio: m.format.has_audio(),
                src_in: c.src_in,
                len: c.len(),
                skip: (from - start).max(Micros::ZERO),
                fade_in,
                fade_out,
                gain: crate::music::clip_gain(c),
                clean: c.clean_audio,
                to_black: tl.end_fade() && i + 1 == tl.clips.len() && !tl.has_fade(i),
                sphere: m
                    .format
                    .is_360()
                    .then(|| project.camera.path(m.id).to_vec()),
                anchored: m
                    .format
                    .is_360()
                    .then(|| anchored_doc(project, m, start, end, from))
                    .flatten(),
                frame: {
                    let stack = tl.frame_stack(i);
                    (tl.frame_aspect.is_some() || !stack.is_empty()).then_some(SegFrame {
                        aspect: tl.frame_aspect,
                        stack,
                    })
                },
            });
        }
        start = end;
    }
    if out.is_empty() {
        return Err(RenderError::Empty);
    }
    Ok(out)
}

/// The ASS file of the texts fixed to the sphere for the 360 clip of `m` that is on the
/// timeline from `start` to `end` (rendered from `from` on): in the pixels of the source
/// picture, local time 0 = the first rendered picture of the clip.
fn anchored_doc(
    project: &Project,
    m: &MediaItem,
    start: Micros,
    end: Micros,
    from: Micros,
) -> Option<String> {
    let (w, h) = (m.format.width.max(2), m.format.height.max(1));
    let win = [(start, end)];
    let end_all = project.timeline.duration();
    let mut events: Vec<crate::text::AssEvent> = Vec::new();
    for (t, size) in anchored_line_items(project, m, start, end) {
        let px = t.anchor.map(|a| crate::text::line_equirect_px(&a, h, size));
        events.extend(crate::text::equirect_events_px(
            &t,
            w,
            h,
            px,
            end_all,
            Some(&win),
        ));
    }
    for t in &project.texts.items {
        events.extend(crate::text::equirect_events(t, w, h, end_all, Some(&win)));
    }
    let shift = start.max(from);
    events
        .iter()
        .any(|e| e.end > shift)
        .then(|| ass::document_with_texts(&[], project.subtitles.preset, w, h, shift, &events))
}

/// True when the clip a line is shown in is a 360 clip.
fn line_on_sphere(project: &Project, l: &crate::subtitles::ShownLine) -> bool {
    project
        .timeline
        .get(l.clip)
        .and_then(|c| project.media(c.media))
        .is_some_and(|m| m.format.is_360())
}

/// The anchored subtitle lines shown on the 360 clip of `m` between `start` and `end`, as
/// texts (white) with the line's size (thousandths) so that they are drawn like texts fixed
/// to the sphere.
fn anchored_line_items(
    project: &Project,
    m: &MediaItem,
    start: Micros,
    end: Micros,
) -> Vec<(crate::text::TextItem, u16)> {
    project
        .subtitles
        .project(&project.timeline)
        .into_iter()
        .filter(|l| l.anchor.is_some() && l.start >= start && l.start < end)
        .filter(|l| {
            project
                .timeline
                .get(l.clip)
                .is_some_and(|c| c.media == m.id)
        })
        .map(|l| {
            let mut t = crate::text::TextItem::new(
                crate::text::TextId(0),
                crate::text::clean_text(&l.text).unwrap_or_else(|| " ".into()),
                l.start,
            );
            t.ttl = (l.end - l.start).max(crate::text::MIN_TTL);
            t.anchor = l.anchor;
            (t, l.size.unwrap_or(crate::subtitles::SIZE_NORMAL))
        })
        .collect()
}

/// The timeline times of the flat (not 360) clips when the video has 360 clips; `None`
/// when it has none (texts fixed to the sphere are then shown at their flat place).
fn flat_windows(project: &Project) -> Option<Vec<(Micros, Micros)>> {
    let tl = &project.timeline;
    let is_360 =
        |c: &crate::timeline::Clip| project.media(c.media).is_some_and(|m| m.format.is_360());
    if !tl.clips.iter().any(is_360) {
        return None;
    }
    let mut v = Vec::new();
    let mut start = Micros::ZERO;
    for c in &tl.clips {
        let end = start + c.len();
        if !is_360(c) {
            v.push((start, end));
        }
        start = end;
    }
    Some(v)
}

/// Media items of the timeline clips, in timeline order.
pub fn timeline_media(project: &Project) -> Result<Vec<&MediaItem>, RenderError> {
    project
        .timeline
        .clips
        .iter()
        .map(|c| project.media(c.media).ok_or(RenderError::UnknownMedia))
        .collect()
}

/// Size a 360 clip counts as when the output format is chosen (REQ-022): the camera view
/// is a flat picture of this size at the source's frame rate.
pub const SPHERE_FORMAT: (u32, u32) = (1920, 1080);

/// The format of `m` as the output choice sees it: a 360 source counts as a flat
/// 1920x1080 picture.
pub fn effective_format(m: &MediaItem) -> MediaFormat {
    let mut f = m.format.clone();
    if f.is_360() {
        // the bit rate of a big sphere would be wasted on a flat 1080p picture
        let from = (f.width as u128 * f.height as u128).max(1);
        let to = SPHERE_FORMAT.0 as u128 * SPHERE_FORMAT.1 as u128;
        f.bit_rate = f
            .bit_rate
            .map(|b| ((b as u128 * to / from) as u64).clamp(1_500_000, b.max(1_500_000)));
        (f.width, f.height) = SPHERE_FORMAT;
    }
    f
}

/// Output picture geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
}

impl Geometry {
    pub fn of(s: &EncodeSettings) -> Geometry {
        Geometry {
            width: s.width,
            height: s.height,
            fps: s.fps,
        }
    }

    /// Same aspect ratio, at most [`PREVIEW_HEIGHT`] high, even dimensions.
    pub fn preview(self) -> Geometry {
        if self.height <= PREVIEW_HEIGHT {
            return self;
        }
        let w = (self.width as u64 * PREVIEW_HEIGHT as u64 / self.height.max(1) as u64) as u32;
        Geometry {
            width: (w & !1).max(2),
            height: PREVIEW_HEIGHT,
            fps: self.fps,
        }
    }
}

/// One ffmpeg input: `[start, start + len)` of `path` (plus one second of margin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub path: PathBuf,
    pub start: Micros,
    pub len: Micros,
}

/// Groups source spans `(path, start, end)` into inputs. Returns the inputs and, for each
/// span, the index of its input. A span joins the previous input when it reads the same
/// file at or after the previous span's end (and not too far after it).
pub fn group_inputs(spans: &[(PathBuf, Micros, Micros)]) -> (Vec<Input>, Vec<usize>) {
    let mut inputs: Vec<Input> = Vec::new();
    let mut index = Vec::with_capacity(spans.len());
    let mut last_end = Micros::ZERO;
    for (path, start, end) in spans {
        let joins = inputs.last().is_some_and(|i| {
            &i.path == path && *start >= last_end && *start - last_end <= MAX_RUN_GAP
        });
        if joins {
            let i = inputs.last_mut().expect("checked above");
            i.len = *end - i.start;
        } else {
            inputs.push(Input {
                path: path.clone(),
                start: *start,
                len: *end - *start,
            });
        }
        index.push(inputs.len() - 1);
        last_end = *end;
    }
    (inputs, index)
}

/// `-ss <start> -t <len + 1 s> -i file:<path>` for each input.
pub fn input_args(inputs: &[Input]) -> Vec<OsString> {
    let mut a = Vec::new();
    for i in inputs {
        a.extend(argv([
            "-protocol_whitelist".to_string(),
            "file,pipe".to_string(),
            "-ss".to_string(),
            i.start.ff(),
            "-t".to_string(),
            (i.len + Micros::SECOND).ff(),
            "-i".to_string(),
        ]));
        a.push(file_arg(&i.path));
    }
    a
}

/// A piece of the music bed.
#[derive(Debug, Clone, PartialEq)]
pub enum BedPart {
    /// Silence.
    Gap(Micros),
    /// `len` of `path` from source time `src`.
    Clip {
        path: PathBuf,
        src: Micros,
        len: Micros,
    },
}

/// The music under the video from `from` to the video's end.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicBed {
    pub parts: Vec<BedPart>,
    pub len: Micros,
    /// Fade-out `(start, duration)` relative to the bed, when music runs past the video.
    pub fade: Option<(Micros, Micros)>,
}

/// Builds the music bed for the timeline from `from`; `None` when no music plays there.
pub fn music_bed(project: &Project, from: Micros) -> Result<Option<MusicBed>, RenderError> {
    let end = project.timeline.duration();
    if from >= end {
        return Ok(None);
    }
    let mut parts = Vec::new();
    let mut cursor = from;
    let mut past_end = false;
    for c in &project.music.clips {
        if c.end() <= from || c.start >= end {
            continue;
        }
        let m = project.media(c.media).ok_or(RenderError::UnknownMedia)?;
        if m.missing {
            return Err(RenderError::Missing(m.name.clone()));
        }
        let s = c.start.max(from);
        let e = c.end().min(end);
        if s > cursor {
            parts.push(BedPart::Gap(s - cursor));
        }
        parts.push(BedPart::Clip {
            path: m.path.clone(),
            src: c.src_in + (s - c.start),
            len: e - s,
        });
        cursor = e;
        past_end |= c.end() > end;
    }
    if parts.is_empty() {
        return Ok(None);
    }
    if cursor < end {
        parts.push(BedPart::Gap(end - cursor));
    }
    let len = end - from;
    let fade = past_end.then(|| {
        let d = END_FADE.min(len);
        (len - d, d)
    });
    Ok(Some(MusicBed { parts, len, fade }))
}

/// What a graph must produce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphSpec {
    pub video: bool,
    pub audio: bool,
    pub pix_fmt: &'static str,
    pub geometry: Geometry,
    /// Burn in `subs.ass` from the working folder.
    pub subtitles: bool,
    pub mix: Mix,
}

/// Inputs and filter graph; outputs are labelled `[outv]` and `[outa]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub inputs: Vec<Input>,
    pub graph: String,
    /// Files the graph reads from the working folder: `(name, content)`, the camera
    /// commands of the 360 clips (written by [`write_command_files`]).
    pub command_files: Vec<(String, String)>,
}

/// Writes the command files of a graph into the working folder `dir`.
pub fn write_command_files(dir: &Path, files: &[(String, String)]) -> io::Result<()> {
    for (name, content) in files {
        std::fs::write(dir.join(name), content)?;
    }
    Ok(())
}

/// Labels for the users of each input stream (`split`/`asplit` when shared).
fn fan_out(parts: &mut Vec<String>, input: usize, kind: char, users: usize) -> Vec<String> {
    let stream = format!("[{input}:{kind}:0]");
    if users <= 1 {
        return vec![stream];
    }
    let labels: Vec<String> = (0..users).map(|j| format!("[{kind}{input}s{j}]")).collect();
    let filter = if kind == 'v' { "split" } else { "asplit" };
    parts.push(format!("{stream}{filter}={users}{}", labels.concat()));
    labels
}

/// The fade filters of a segment. `early` moves the fade-out that much earlier (for the
/// picture of the fade at the very end of the video, so that the last frame is black).
fn fades(c: &mut String, filter: &str, s: &Segment, early: Micros) {
    if s.fade_in > Micros::ZERO {
        c.push_str(&format!(",{filter}=t=in:st=0:d={}", s.fade_in.ff()));
    }
    if s.fade_out > Micros::ZERO {
        c.push_str(&format!(
            ",{filter}=t=out:st={}:d={}",
            (s.len - s.fade_out - early).max(Micros::ZERO).ff(),
            s.fade_out.ff()
        ));
    }
}

/// The Apply LUT filters of a segment (1.6.0, ADR-110) with a leading comma, or an empty
/// string without a LUT. The whole video's LUT comes first. Each cube is written under a
/// fixed name into the working folder (`command_files`), so no user text reaches the filter.
/// The picture goes through planar RGB for the table and back to `pix_fmt`.
fn lut_chain(s: &Segment, i: usize, pix_fmt: &str, files: &mut Vec<(String, String)>) -> String {
    let mut c = String::new();
    let mut add = |name: String, l: &crate::lut::Lut| {
        c.push_str(&format!(",lut3d=file={name}:interp=tetrahedral"));
        if !files.iter().any(|f| f.0 == name) {
            files.push((name, l.to_cube()));
        }
    };
    if let Some(l) = &s.lut_clip {
        add(format!("lut-clip-{i}.cube"), l);
    }
    if c.is_empty() {
        return c;
    }
    format!(",format=gbrp{c},format={pix_fmt}")
}

/// The frame effect of a segment (1.5.0, REQ-131 stages A-C of ADR-098): `picture` is the
/// chain that makes the fitted picture (without its output format). Pushes the chains of
/// the picture, its scale and the background, and returns the chain that overlays them
/// (the caller adds fades and the label).
fn compose_frame(
    parts: &mut Vec<String>,
    i: usize,
    s: &Segment,
    f: &SegFrame,
    spec: &GraphSpec,
    picture: String,
) -> String {
    let g = &spec.geometry;
    let (w, h) = (g.width, g.height);
    let o = s.skip.ff();
    let blur = matches!(f.aspect.map(|a| a.fill), Some(frame::Fill::Blur));
    // stage B: the background
    if blur {
        parts.push(format!("{picture},split=2[pa{i}][pb{i}]"));
        let (sw, sh) = (
            (w / frame::BLUR_DIVISOR).max(2) & !1,
            (h / frame::BLUR_DIVISOR).max(2) & !1,
        );
        parts.push(format!(
            "[pb{i}]scale={sw}:{sh}:force_original_aspect_ratio=increase,crop={sw}:{sh},\
             boxblur={r}:2,scale={w}:{h},setsar=1[b{i}]",
            // the radius may not pass half of the smaller chroma plane
            r = frame::BLUR_RADIUS.min((sw.min(sh) / 4).max(1))
        ));
    } else {
        let rgb = match f.aspect.map(|a| a.fill) {
            Some(frame::Fill::Colour(c)) => c,
            _ => frame::Rgb::BLACK,
        };
        parts.push(format!("{picture}[pa{i}]"));
        parts.push(format!(
            "color=c={c}:s={w}x{h}:r={fps}:d={d},setpts=PTS-STARTPTS+{o}/TB[b{i}]",
            c = rgb.ff(),
            fps = g.fps.ff(),
            d = s.rendered().ff(),
        ));
    }
    // stage C: the picture at the size and place of the Scale and Move keys
    let st = &f.stack;
    let src = if st.scale.is_empty() {
        format!("[pa{i}]")
    } else {
        let k = st.scale_factor();
        parts.push(format!(
            "[pa{i}]scale=w='max(2,trunc(iw*({k})/2)*2)':h='max(2,trunc(ih*({k})/2)*2)':\
             eval=frame[pm{i}]"
        ));
        format!("[pm{i}]")
    };
    let place = |main: &str, over: &str, off: String| {
        if off == "0" {
            format!("({main}-{over})/2")
        } else {
            format!("({main}-{over})/2+{main}*({off})")
        }
    };
    format!(
        "[b{i}]{src}overlay=x='{x}':y='{y}':eval=frame:shortest=1,format={pf}",
        x = place("W", "w", st.offset_x()),
        y = place("H", "h", st.offset_y()),
        pf = spec.pix_fmt,
    )
}

/// The single filter graph for `segs` (and the music `bed`).
pub fn build_graph(segs: &[Segment], bed: Option<&MusicBed>, spec: &GraphSpec) -> Graph {
    let spans: Vec<(PathBuf, Micros, Micros)> = segs
        .iter()
        .map(|s| (s.path.clone(), s.src_in + s.skip, s.src_in + s.len))
        .collect();
    let (mut inputs, idx) = group_inputs(&spans);
    let bed = bed.filter(|_| spec.audio);
    let music: Vec<(PathBuf, Micros, Micros)> = bed
        .map(|b| {
            b.parts
                .iter()
                .filter_map(|p| match p {
                    BedPart::Clip { path, src, len } => Some((path.clone(), *src, *src + *len)),
                    BedPart::Gap(_) => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let (music_inputs, music_idx) = group_inputs(&music);
    let offset = inputs.len();
    inputs.extend(music_inputs);

    let mut parts = Vec::new();
    let count = |k: usize, f: &dyn Fn(&Segment) -> bool| {
        idx.iter()
            .zip(segs)
            .filter(|(i, s)| **i == k && f(s))
            .count()
    };
    let mut vlabels: Vec<Vec<String>> = Vec::new();
    let mut alabels: Vec<Vec<String>> = Vec::new();
    for k in 0..offset {
        vlabels.push(if spec.video {
            fan_out(&mut parts, k, 'v', count(k, &|_| true))
        } else {
            Vec::new()
        });
        alabels.push(if spec.audio {
            fan_out(&mut parts, k, 'a', count(k, &|s| s.has_audio))
        } else {
            Vec::new()
        });
    }
    let (mut vused, mut aused) = (vec![0usize; offset], vec![0usize; offset]);
    let g = &spec.geometry;
    let mut command_files: Vec<(String, String)> = Vec::new();
    let mut concat_in = String::new();
    for (i, s) in segs.iter().enumerate() {
        let k = idx[i];
        let off = (s.src_in + s.skip - inputs[k].start).ff();
        let d = s.rendered().ff();
        if spec.video {
            let src = &vlabels[k][vused[k]];
            vused[k] += 1;
            let (w, h) = (g.width, g.height);
            let fr = s.frame.as_ref();
            // the camera view fills the output picture, or the kept side of a Change aspect
            let (cw, ch) = match fr {
                Some(f) => frame::camera_size((w, h), f.aspect.as_ref()),
                None => (w, h),
            };
            let lut = lut_chain(s, i, spec.pix_fmt, &mut command_files);
            let mut c = if let Some(path) = &s.sphere {
                // the camera view fills the output picture: no scale, no bars
                let from = s.src_in + s.skip;
                let cam = camera::at(path, from).unwrap_or(Camera::FRONT);
                let id = format!("c{i}");
                let cmds = camera::command_lines(
                    &format!("v360@{id}"),
                    path,
                    from,
                    s.src_in + s.len,
                    g.fps,
                    cw,
                    ch,
                );
                let send = if cmds.is_empty() {
                    String::new()
                } else {
                    let name = format!("cam{i}.txt");
                    command_files.push((name.clone(), cmds));
                    format!("sendcmd=f={name},")
                };
                // texts fixed to the sphere are drawn on the whole picture first
                let anchored = match &s.anchored {
                    Some(doc) => {
                        let name = format!("anc{i}.ass");
                        command_files.push((name.clone(), doc.clone()));
                        format!("subtitles=f={name}:fontsdir={},", ass::FONTS_DIR)
                    }
                    None => String::new(),
                };
                let tail = if fr.is_some() {
                    String::new()
                } else {
                    format!(",format={}", spec.pix_fmt)
                };
                // the LUT goes before the texts fixed to the sphere (they keep their colours),
                // or after the camera cut when there are none (smaller picture)
                let (lut_before, lut_after) = if s.anchored.is_some() {
                    (lut.as_str(), "")
                } else {
                    ("", lut.as_str())
                };
                format!(
                    "{src}trim=start={off}:duration={d},setpts=PTS-STARTPTS,fps={f}{lut_before},{anchored}{send}{v360},\
                     setsar=1{lut_after},setpts=PTS+{o}/TB{tail}",
                    v360 = camera::filter(&id, cam, cw, ch),
                    o = s.skip.ff(),
                    f = g.fps.ff(),
                )
            } else {
                // a 360 video keeps the equirectangular picture: texts fixed to the sphere are
                // burned on it
                let anchored = match &s.anchored {
                    Some(doc) => {
                        let name = format!("anc{i}.ass");
                        command_files.push((name.clone(), doc.clone()));
                        format!(",subtitles=f={name}:fontsdir={}", ass::FONTS_DIR)
                    }
                    None => String::new(),
                };
                match fr {
                    // the picture fitted to the frame, with no bars: the frame effect
                    // draws the background and places the picture
                    Some(f) => format!(
                        "{src}trim=start={off}:duration={d},setpts=PTS-STARTPTS+{o}/TB{fit},\
                         setsar=1,fps={fps}{lut}{anchored}",
                        fit = frame::fit_filter((w, h), f.aspect.as_ref()),
                        o = s.skip.ff(),
                        fps = g.fps.ff(),
                    ),
                    None => format!(
                        "{src}trim=start={off}:duration={d},setpts=PTS-STARTPTS+{o}/TB,\
                         scale={w}:{h}:force_original_aspect_ratio=decrease{lut},\
                         pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={f}{anchored},\
                         format={pf}",
                        o = s.skip.ff(),
                        f = g.fps.ff(),
                        pf = spec.pix_fmt,
                    ),
                }
            };
            if let Some(f) = fr {
                c = compose_frame(&mut parts, i, s, f, spec, c);
            }
            // two frames: sources often start a few milliseconds late, which costs one
            let early = if s.to_black {
                Micros(g.fps.frame_duration().0 * 2)
            } else {
                Micros::ZERO
            };
            fades(&mut c, "fade", s, early);
            c.push_str(&format!(",setpts=PTS-STARTPTS[v{i}]"));
            parts.push(c);
            concat_in.push_str(&format!("[v{i}]"));
        }
        if spec.audio {
            let mut c = if s.has_audio {
                let src = &alabels[k][aused[k]];
                aused[k] += 1;
                format!(
                    "{src}atrim=start={off}:duration={d},asetpts=PTS-STARTPTS,\
                     aresample={SAMPLE_RATE},{AFORMAT},apad=whole_dur={d},atrim=duration={d},\
                     asetpts=PTS-STARTPTS+{o}/TB",
                    o = s.skip.ff()
                )
            } else {
                format!(
                    "anullsrc=r={SAMPLE_RATE}:cl=stereo,atrim=duration={d},{AFORMAT},\
                     asetpts=PTS-STARTPTS+{o}/TB",
                    o = s.skip.ff()
                )
            };
            if s.clean && s.has_audio {
                c.push(',');
                c.push_str(CLEAN_AUDIO);
            }
            if s.gain != 1.0 {
                c.push_str(&format!(",volume={:.6}", s.gain));
            }
            fades(&mut c, "afade", s, Micros::ZERO);
            c.push_str(&format!(",asetpts=PTS-STARTPTS[a{i}]"));
            parts.push(c);
            concat_in.push_str(&format!("[a{i}]"));
        }
    }
    let needs_mix = spec.audio && (bed.is_some() || !spec.mix.video_neutral());
    let mut outs = String::new();
    if spec.video {
        outs.push_str(if spec.subtitles { "[cv]" } else { "[outv]" });
    }
    if spec.audio {
        outs.push_str(if needs_mix { "[ca]" } else { "[outa]" });
    }
    parts.push(format!(
        "{concat_in}concat=n={}:v={}:a={}{outs}",
        segs.len(),
        u8::from(spec.video),
        u8::from(spec.audio)
    ));
    if spec.video && spec.subtitles {
        parts.push(format!(
            "[cv]subtitles=f={}:fontsdir={}[outv]",
            ass::ASS_FILE,
            ass::FONTS_DIR
        ));
    }
    if let Some(bed) = bed {
        let users: Vec<usize> = (0..music_idx.len())
            .map(|j| music_idx.iter().filter(|k| **k == music_idx[j]).count())
            .collect();
        let mut mlabels: Vec<Vec<String>> = Vec::new();
        let mut first_of: Vec<usize> = Vec::new();
        for (j, k) in music_idx.iter().enumerate() {
            if first_of.contains(k) {
                continue;
            }
            first_of.push(*k);
            mlabels.push(fan_out(&mut parts, offset + k, 'a', users[j]));
        }
        let mut mused = vec![0usize; mlabels.len()];
        let mut bed_in = String::new();
        let mut clip_no = 0;
        for (j, p) in bed.parts.iter().enumerate() {
            let c = match p {
                BedPart::Gap(d) => format!(
                    "anullsrc=r={SAMPLE_RATE}:cl=stereo,atrim=duration={},{AFORMAT},asetpts=PTS-STARTPTS[m{j}]",
                    d.ff()
                ),
                BedPart::Clip { src, len, .. } => {
                    let k = music_idx[clip_no];
                    clip_no += 1;
                    let slot = first_of.iter().position(|x| *x == k).unwrap_or(0);
                    let label = &mlabels[slot][mused[slot]];
                    mused[slot] += 1;
                    let off = (*src - inputs[offset + k].start).ff();
                    let d = len.ff();
                    format!(
                        "{label}atrim=start={off}:duration={d},asetpts=PTS-STARTPTS,\
                         aresample={SAMPLE_RATE},{AFORMAT},apad=whole_dur={d},atrim=duration={d},\
                         asetpts=PTS-STARTPTS[m{j}]"
                    )
                }
            };
            parts.push(c);
            bed_in.push_str(&format!("[m{j}]"));
        }
        let fade = bed
            .fade
            .map(|(st, d)| format!(",afade=t=out:st={}:d={}", st.ff(), d.ff()))
            .unwrap_or_default();
        parts.push(format!(
            "{bed_in}concat=n={}:v=0:a=1{fade}[mb]",
            bed.parts.len()
        ));
        parts.push(format!(
            "[ca]volume={:.6}[va];[mb]volume={:.6}[vm];\
             [va][vm]amix=inputs=2:duration=first:normalize=0,alimiter=limit=0.97:level=0[outa]",
            spec.mix.video_gain(),
            spec.mix.music_gain()
        ));
    } else if needs_mix {
        parts.push(format!("[ca]volume={:.6}[outa]", spec.mix.video_gain()));
    }
    Graph {
        inputs,
        graph: parts.join(";"),
        command_files,
    }
}

/// Common leading options.
fn head() -> Vec<OsString> {
    argv(["-hide_banner", "-nostdin", "-loglevel", "error", "-y"])
}

/// How the export is done.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Concat demuxer with `-c copy`; `concat_list` is the list file content.
    StreamCopy {
        concat_list: String,
        muxer: String,
        faststart: bool,
        /// HEVC in MP4/MOV: tag it `hvc1` for Apple players.
        hevc_tag: bool,
    },
    /// One re-encode pass. `subtitles` is the ASS document to burn in, if any.
    Reencode {
        graph: Box<Graph>,
        settings: EncodeSettings,
        subtitles: Option<String>,
    },
}

impl Plan {
    /// The ASS document the export needs in its working folder.
    pub fn subtitles(&self) -> Option<&str> {
        match self {
            Plan::Reencode { subtitles, .. } => subtitles.as_deref(),
            Plan::StreamCopy { .. } => None,
        }
    }

    /// The camera command files the export needs in its working folder.
    pub fn command_files(&self) -> &[(String, String)] {
        match self {
            Plan::Reencode { graph, .. } => &graph.command_files,
            Plan::StreamCopy { .. } => &[],
        }
    }
}

/// True when the path contains a control character (line breaks, tabs, …).
pub fn has_control_chars(path: &Path) -> bool {
    path.to_string_lossy().chars().any(char::is_control)
}

/// Quotes a path for a concat list (`'` becomes `'\''`).
pub fn concat_quote(path: &Path) -> String {
    let s = format!("file:{}", path.to_string_lossy());
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Concat-demuxer list for `segs`.
pub fn concat_list(segs: &[Segment]) -> String {
    let mut s = String::from("ffconcat version 1.0\n");
    for seg in segs {
        s.push_str(&format!(
            "file {}\ninpoint {}\noutpoint {}\n",
            concat_quote(&seg.path),
            seg.src_in.ff(),
            (seg.src_in + seg.len).ff()
        ));
    }
    s
}

fn hevc_in_mp4(codec: Option<&str>, muxer: &str) -> bool {
    codec == Some("hevc") && matches!(muxer, "mp4" | "mov")
}

/// The ASS document for the subtitles visible from `from` (export geometry), if any.
pub fn subtitles_doc(project: &Project, g: &Geometry, from: Micros) -> Option<String> {
    let mut lines = project.subtitles.project(&project.timeline);
    // a line fixed to the sphere on a 360 clip is burned on the equirectangular picture
    lines.retain(|l| !(l.anchor.is_some() && line_on_sphere(project, l)));
    let windows = flat_windows(project);
    let texts = crate::text::project_events(
        &project.texts,
        g.width,
        g.height,
        project.timeline.duration(),
        windows.as_deref(),
    );
    // texts fixed to the sphere need libass (and the fonts folder) too
    let anchored = windows.is_some()
        && (project.texts.items.iter().any(|t| t.anchor.is_some())
            || project
                .subtitles
                .project(&project.timeline)
                .iter()
                .any(|l| l.anchor.is_some() && line_on_sphere(project, l)));
    if !lines.iter().any(|l| l.end > from) && !texts.iter().any(|e| e.end > from) && !anchored {
        return None;
    }
    Some(ass::document_with_texts(
        &lines,
        project.subtitles.preset,
        g.width,
        g.height,
        from,
        &texts,
    ))
}

/// Decides between stream copy and re-encode. `is_keyframe(media, t)` is only consulted
/// when the formats are identical and there are no effects (fades, music, subtitles,
/// video volume change). Always reads the original files (never proxies).
pub fn plan_export(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
    is_keyframe: impl FnMut(&MediaItem, Micros) -> bool,
) -> Result<Plan, RenderError> {
    plan_export_mode(project, encoders, ExportMode::Automatic, is_keyframe)
}

/// [`plan_export`] for a chosen [`ExportMode`]. A 360 video keeps every clip as the whole
/// sphere (no camera view, the output has the size of the 360 clips) and needs all clips
/// to be 360°.
pub fn plan_export_mode(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
    mode: ExportMode,
    is_keyframe: impl FnMut(&MediaItem, Micros) -> bool,
) -> Result<Plan, RenderError> {
    plan_export_choice(
        project,
        encoders,
        mode,
        &format::FormatChoice::default(),
        is_keyframe,
    )
}

/// [`plan_export_mode`] with the format choices of the Export dialog (REQ-120, REQ-121).
/// A video preset replaces the automatic format (not for a 360 video, which keeps its
/// own size); any chosen format, and any clip with Clean audio, forces a re-encode.
pub fn plan_export_choice(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
    mode: ExportMode,
    choice: &format::FormatChoice,
    mut is_keyframe: impl FnMut(&MediaItem, Micros) -> bool,
) -> Result<Plan, RenderError> {
    let sphere = mode == ExportMode::Sphere360;
    let mut segs = segments(project, Micros::ZERO, Source::Export)?;
    let media = timeline_media(project)?;
    if sphere {
        if media.iter().any(|m| !m.format.is_360()) {
            return Err(RenderError::MixedSphere);
        }
        // no camera: the picture stays equirectangular; no frame effects (REQ-131)
        for s in &mut segs {
            s.sphere = None;
            s.frame = None;
        }
    }
    let owned: Vec<MediaFormat> = media
        .iter()
        .map(|m| {
            if sphere {
                m.format.clone()
            } else {
                effective_format(m)
            }
        })
        .collect();
    let formats: Vec<&MediaFormat> = owned.iter().collect();
    let chosen = format::choose_output(&formats).ok_or(RenderError::Empty)?;
    let mut settings = format::encode_settings(&chosen, encoders);
    let preset = choice.video.filter(|_| !sphere);
    if let Some(i) = preset {
        settings = format::preset_settings(i, encoders).ok_or(RenderError::NoEncoder)?;
    }
    if let Some(enc) = choice.audio.encoder() {
        if !encoders.contains(enc) {
            return Err(RenderError::NoEncoder);
        }
        settings.aencoder = enc.into();
    }
    if !sphere {
        // the frame of Change aspect is the output size (REQ-125)
        (settings.width, settings.height) = frame::frame_size(
            (settings.width, settings.height),
            project.timeline.frame_aspect.as_ref(),
        );
    }
    let geometry = Geometry::of(&settings);
    let subtitles = subtitles_doc(project, &geometry, Micros::ZERO);
    let bed = music_bed(project, Micros::ZERO)?;
    let no_effects = !project.timeline.has_any_fade()
        && (sphere || !project.timeline.has_frame_effect())
        && preset.is_none()
        && choice.audio == format::AudioChoice::Same
        && !project.timeline.any_clean_audio()
        && !project.timeline.any_lut()
        && project.timeline.clips.iter().all(|c| !c.has_volume())
        && bed.is_none()
        && subtitles.is_none()
        && project.mix.video_neutral()
        && (sphere || media.iter().all(|m| !m.format.is_360()));
    // A file name with a line break or other control character could add lines to the
    // concat list: such projects are always re-encoded (inputs are then plain arguments).
    let safe_names = segs.iter().all(|s| !has_control_chars(&s.path));
    if format::all_identical(&formats) && no_effects && safe_names {
        let keyframes_ok = project
            .timeline
            .clips
            .iter()
            .zip(&media)
            .all(|(c, m)| c.src_in == Micros::ZERO || is_keyframe(m, c.src_in));
        if keyframes_ok {
            let (muxer, faststart) = match chosen.container.as_str() {
                "mov" => ("mov", true),
                "matroska" => ("matroska", false),
                "webm" => ("webm", false),
                "mp4" => ("mp4", true),
                _ => (settings.muxer.as_str(), settings.faststart),
            };
            return Ok(Plan::StreamCopy {
                concat_list: concat_list(&segs),
                muxer: muxer.to_string(),
                faststart,
                hevc_tag: hevc_in_mp4(chosen.vcodec.as_deref(), muxer),
            });
        }
    }
    let graph = build_graph(
        &segs,
        bed.as_ref(),
        &GraphSpec {
            video: true,
            audio: true,
            pix_fmt: "yuv420p",
            geometry,
            subtitles: subtitles.is_some(),
            mix: project.mix,
        },
    );
    Ok(Plan::Reencode {
        graph: Box::new(graph),
        settings,
        subtitles,
    })
}

/// An export of the sound only (REQ-119): the audio graph of the whole timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioPlan {
    pub graph: Graph,
    pub settings: EncodeSettings,
}

/// The plan of "Only audio": video sound (with volumes and Clean audio) and music of the
/// whole timeline, in the audio format `audio` (`Same` makes AAC).
pub fn plan_audio_only(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
    audio: format::AudioChoice,
) -> Result<AudioPlan, RenderError> {
    let settings = format::audio_only_settings(audio, encoders).ok_or(RenderError::NoEncoder)?;
    let segs = segments(project, Micros::ZERO, Source::Export)?;
    let bed = music_bed(project, Micros::ZERO)?;
    let graph = build_graph(
        &segs,
        bed.as_ref(),
        &GraphSpec {
            video: false,
            audio: true,
            pix_fmt: "yuv420p",
            geometry: Geometry::of(&settings),
            subtitles: false,
            mix: project.mix,
        },
    );
    Ok(AudioPlan { graph, settings })
}

/// The ffmpeg argv of an audio-only export (no video is decoded or written).
pub fn audio_export_args(
    plan: &AudioPlan,
    paths: &ExportPaths<'_>,
    version: FfVersion,
    threads: Option<u32>,
) -> io::Result<Vec<OsString>> {
    let s = &plan.settings;
    let mut a = head();
    match threads {
        Some(t) => {
            a.extend(argv(["-filter_complex_threads".to_string(), t.to_string()]));
            a.extend(threaded_inputs(input_args(&plan.graph.inputs), t));
        }
        None => a.extend(input_args(&plan.graph.inputs)),
    }
    a.extend(filter_graph_args(
        &plan.graph.graph,
        version,
        paths.graph_script,
    )?);
    a.extend(argv(["-map", "[outa]", "-vn", "-c:a", &s.aencoder]));
    if let Some(br) = s.audio_bitrate {
        a.extend(argv(["-b:a".to_string(), br.to_string()]));
    }
    a.extend(argv(["-ar".to_string(), SAMPLE_RATE.to_string()]));
    if let Some(t) = threads {
        a.extend(argv(["-threads".to_string(), t.to_string()]));
    }
    a.extend(argv(["-f", s.muxer.as_str()]));
    a.extend(argv(["-progress", "pipe:1", "-nostats"]));
    a.push(file_arg(paths.output));
    Ok(a)
}

/// Paths the export argv refers to.
pub struct ExportPaths<'a> {
    pub output: &'a Path,
    /// Where the concat list is written (stream copy).
    pub concat_list: &'a Path,
    /// Where a long filter graph is written.
    pub graph_script: &'a Path,
}

/// Inserts `-threads <t>` before every `-i` of `inputs`.
pub fn threaded_inputs(inputs: Vec<OsString>, t: u32) -> Vec<OsString> {
    let mut out = Vec::with_capacity(inputs.len() * 2);
    for x in inputs {
        if x == "-i" {
            out.extend(argv(["-threads".to_string(), t.to_string()]));
        }
        out.push(x);
    }
    out
}

/// Full ffmpeg argv for an export. Writes the concat list / graph script when needed.
/// `threads` limits ffmpeg's decoding, filtering and encoding threads (re-encode only;
/// `None` lets ffmpeg decide). It never changes what the export looks like.
///
/// With `hw` (a hardware method, re-encode only) the device setup goes before the inputs,
/// the graph gets the method's tail and the method's video arguments replace the software
/// ones; everything else (inputs, graph, audio, container) is the same. With `None` the
/// argv is exactly the one of the normal method.
pub fn export_args(
    plan: &Plan,
    paths: &ExportPaths<'_>,
    version: FfVersion,
    threads: Option<u32>,
    hw: Option<&HwEncode>,
) -> io::Result<Vec<OsString>> {
    let mut a = head();
    match plan {
        Plan::StreamCopy {
            concat_list,
            muxer,
            faststart,
            hevc_tag,
        } => {
            std::fs::write(paths.concat_list, concat_list)?;
            a.extend(argv([
                "-protocol_whitelist",
                "file,pipe",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
            ]));
            a.push(file_arg(paths.concat_list));
            a.extend(argv(["-map", "0:v:0", "-map", "0:a?", "-c", "copy"]));
            if *hevc_tag {
                a.extend(argv(["-tag:v", "hvc1"]));
            }
            if *faststart {
                a.extend(argv(["-movflags", "+faststart"]));
            }
            a.extend(argv(["-f", muxer.as_str()]));
        }
        Plan::Reencode {
            graph, settings: s, ..
        } => {
            if let Some(h) = hw {
                a.extend(h.global.iter().cloned());
            }
            match threads {
                Some(t) => {
                    a.extend(argv(["-filter_complex_threads".to_string(), t.to_string()]));
                    a.extend(threaded_inputs(input_args(&graph.inputs), t));
                }
                None => a.extend(input_args(&graph.inputs)),
            }
            match hw {
                None => a.extend(filter_graph_args(
                    &graph.graph,
                    version,
                    paths.graph_script,
                )?),
                Some(h) => a.extend(filter_graph_args(
                    &format!("{}{}", graph.graph, h.graph_tail),
                    version,
                    paths.graph_script,
                )?),
            }
            let codec = match hw {
                Some(h) => Some(h.codec),
                None => Codec::of_software_encoder(&s.vencoder),
            };
            let hvc1 = codec == Some(Codec::Hevc) && matches!(s.muxer.as_str(), "mp4" | "mov");
            match hw {
                None => {
                    a.extend(argv([
                        "-map",
                        "[outv]",
                        "-map",
                        "[outa]",
                        "-c:v",
                        &s.vencoder,
                    ]));
                    a.extend(argv(&s.vextra));
                    if let Some(br) = s.video_bitrate {
                        let buf = (br * 2).to_string();
                        let br = br.to_string();
                        a.extend(argv(["-b:v", &br, "-maxrate", &br, "-bufsize", &buf]));
                    }
                    if hvc1 {
                        a.extend(argv(["-tag:v", "hvc1"]));
                    }
                    a.extend(argv(["-pix_fmt", "yuv420p"]));
                }
                Some(h) => {
                    a.extend(argv(["-map", h.out_label.as_str(), "-map", "[outa]"]));
                    a.extend(h.vcodec.iter().cloned());
                    if hvc1 {
                        a.extend(argv(["-tag:v", "hvc1"]));
                    }
                }
            }
            a.extend(argv(["-c:a", &s.aencoder]));
            if let Some(br) = s.audio_bitrate {
                a.extend(argv(["-b:a".to_string(), br.to_string()]));
            }
            a.extend(argv(["-ar".to_string(), SAMPLE_RATE.to_string()]));
            if s.faststart {
                a.extend(argv(["-movflags", "+faststart"]));
            }
            if let Some(t) = threads {
                a.extend(argv(["-threads".to_string(), t.to_string()]));
            }
            a.extend(argv(["-f", s.muxer.as_str()]));
        }
    }
    a.extend(argv(["-progress", "pipe:1", "-nostats"]));
    a.push(file_arg(paths.output));
    Ok(a)
}

/// The geometry the export would use (and the preview scales down from).
pub fn output_geometry(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
) -> Result<Geometry, RenderError> {
    let media = timeline_media(project)?;
    let owned: Vec<MediaFormat> = media.iter().map(|m| effective_format(m)).collect();
    let formats: Vec<&MediaFormat> = owned.iter().collect();
    let chosen = format::choose_output(&formats).ok_or(RenderError::Empty)?;
    let mut g = Geometry::of(&format::encode_settings(&chosen, encoders));
    (g.width, g.height) =
        frame::frame_size((g.width, g.height), project.timeline.frame_aspect.as_ref());
    Ok(g)
}

/// A preview render: the argv and the ASS document to put in the working folder.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewRender {
    pub args: Vec<OsString>,
    pub subtitles: Option<String>,
    /// Camera command files of 360 clips, for the working folder.
    pub command_files: Vec<(String, String)>,
}

fn video_render(
    project: &Project,
    segs: &[Segment],
    from: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
) -> Result<PreviewRender, String> {
    let subtitles = subtitles_doc(project, export, from);
    let graph = build_graph(
        segs,
        None,
        &GraphSpec {
            video: true,
            audio: false,
            pix_fmt: "rgba",
            geometry: export.preview(),
            subtitles: subtitles.is_some(),
            mix: project.mix,
        },
    );
    let mut a = head();
    a.extend(input_args(&graph.inputs));
    a.extend(filter_graph_args(&graph.graph, version, script).map_err(|e| e.to_string())?);
    a.extend(argv(["-map", "[outv]"]));
    Ok(PreviewRender {
        args: a,
        subtitles,
        command_files: graph.command_files,
    })
}

/// argv producing raw RGBA frames (preview size of `export`) on stdout from `from`.
pub fn preview_video_args(
    project: &Project,
    from: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
    src: Source,
) -> Result<PreviewRender, String> {
    let segs = segments(project, from, src).map_err(|e| e.to_string())?;
    let mut r = video_render(project, &segs, from, export, version, script)?;
    r.args
        .extend(argv(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"]));
    Ok(r)
}

/// argv producing the single frame shown at `at` (one segment, one input).
pub fn still_args(
    project: &Project,
    at: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
    src: Source,
) -> Result<PreviewRender, String> {
    let mut segs = segments(project, at, src).map_err(|e| e.to_string())?;
    segs.truncate(1);
    let mut r = video_render(project, &segs, at, export, version, script)?;
    r.args.extend(argv([
        "-frames:v",
        "1",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "pipe:1",
    ]));
    Ok(r)
}

/// The normal method's video graph of the export of `project` at `geometry`, without
/// audio or music (the output check renders it next to a hardware file). Output `[outv]`;
/// burned-in subtitles read `subs.ass` from the working folder, as in the export.
pub fn check_graph(project: &Project, geometry: &Geometry) -> Result<Graph, RenderError> {
    let segs = segments(project, Micros::ZERO, Source::Export)?;
    let subtitles = subtitles_doc(project, geometry, Micros::ZERO).is_some();
    Ok(build_graph(
        &segs,
        None,
        &GraphSpec {
            video: true,
            audio: false,
            pix_fmt: "yuv420p",
            geometry: *geometry,
            subtitles,
            mix: project.mix,
        },
    ))
}

/// Largest picture compared by the output check; bigger ones are scaled down (same
/// filter on both sides) to bound memory.
pub const CHECK_MAX_PIXELS: u64 = 1920 * 1080;

/// The size the output check compares pictures at, when it must scale them down.
pub fn check_size(width: u32, height: u32) -> Option<(u32, u32)> {
    let px = width as u64 * height as u64;
    if px <= CHECK_MAX_PIXELS {
        return None;
    }
    let f = (CHECK_MAX_PIXELS as f64 / px as f64).sqrt();
    let even = |n: f64| ((n as u32) & !1).max(2);
    Some((even(width as f64 * f), even(height as f64 * f)))
}

/// The scale filter of [`check_size`] (with a leading comma), or nothing.
pub fn check_scale(width: u32, height: u32) -> String {
    check_size(width, height)
        .map(|(w, h)| format!(",scale={w}:{h}:flags=area"))
        .unwrap_or_default()
}

/// argv producing interleaved s16le PCM (`rate` Hz, `channels`) on stdout from `from`:
/// the video sound and the music, mixed with the project's volumes.
pub fn preview_audio_args(
    project: &Project,
    from: Micros,
    rate: u32,
    channels: u16,
    version: FfVersion,
    script: &Path,
    src: Source,
) -> Result<Vec<OsString>, String> {
    let segs = segments(project, from, src).map_err(|e| e.to_string())?;
    let bed = music_bed(project, from).map_err(|e| e.to_string())?;
    let graph = build_graph(
        &segs,
        bed.as_ref(),
        &GraphSpec {
            video: false,
            audio: true,
            pix_fmt: "rgba",
            geometry: Geometry {
                width: 2,
                height: 2,
                fps: Fps { num: 1, den: 1 },
            },
            subtitles: false,
            mix: project.mix,
        },
    );
    let mut a = head();
    a.extend(input_args(&graph.inputs));
    a.extend(filter_graph_args(&graph.graph, version, script).map_err(|e| e.to_string())?);
    a.extend(argv([
        "-map".to_string(),
        "[outa]".to_string(),
        "-f".to_string(),
        "s16le".to_string(),
        "-ac".to_string(),
        channels.to_string(),
        "-ar".to_string(),
        rate.to_string(),
        "pipe:1".to_string(),
    ]));
    Ok(a)
}

/// A subtitle on screen as part of a [`FrameKey`]: text, highlighted word, own position
/// and own size.
pub type ShownSubtitle = (
    String,
    Option<usize>,
    Option<crate::subtitles::Pos>,
    Option<u16>,
);

/// Everything that decides the picture shown at a timeline position. Two positions with
/// the same key show the same picture, so the still frame is not rendered again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameKey {
    pub path: PathBuf,
    /// Source frame index (at the source frame rate).
    pub frame: i64,
    /// `(fade in, fade out, offset in clip, clip length)` when the frame is inside a fade.
    pub fade: Option<(Micros, Micros, Micros, Micros)>,
    pub size: (u32, u32),
    /// Subtitles on screen, the highlighted word and the line's own position.
    pub subtitles: Vec<ShownSubtitle>,
    pub preset: Preset,
    /// The camera of a 360 clip at this position.
    pub camera: Option<Camera>,
    /// Text items on screen, with the frame index since their start when they have effects.
    pub texts: Vec<(crate::text::TextItem, i64)>,
    /// Where the lines on screen are fixed on the sphere.
    pub line_anchors: Vec<Option<crate::text::Anchor360>>,
    /// Change aspect and the picture's scale and place at this position, in millionths
    /// (1.5.0); `None` with no frame effect.
    pub placement: Option<FramePlacement>,
    /// Fingerprint of the clip's Apply LUT table (1.6.0): a changed file re-renders the
    /// picture once.
    pub luts: Option<String>,
}

/// The frame effect state of a [`FrameKey`].
pub type FramePlacement = (Option<frame::FrameAspect>, (i64, i64, i64));

/// The key of the picture at `at`, or `None` when there is no picture there.
pub fn frame_key(
    project: &Project,
    at: Micros,
    src: Source,
    export: &Geometry,
) -> Option<FrameKey> {
    let tl = &project.timeline;
    let (i, start) = tl.locate(at)?;
    let c = tl.clips[i];
    let m = project.media(c.media)?;
    let offset = at - start;
    let fps = m.format.fps.unwrap_or(export.fps);
    let (fin, fout) = tl.fades_of(i);
    let in_fade = offset < fin || offset >= c.len() - fout;
    let preset = project.subtitles.preset;
    let subtitles = project
        .subtitles
        .shown_at(tl, at)
        .into_iter()
        .map(|l| {
            let w = (preset == Preset::WordHighlight)
                .then(|| l.word_at(at))
                .flatten();
            (l.text, w, l.position, l.size)
        })
        .collect();
    let g = export.preview();
    let camera = m
        .format
        .is_360()
        .then(|| camera::at(project.camera.path(m.id), c.src_in + offset).unwrap_or(Camera::FRONT));
    let stack = tl.frame_stack(i);
    let placement = (tl.frame_aspect.is_some() || !stack.is_empty()).then(|| {
        let p = stack.at(offset);
        let q = |v: f64| (v * 1_000_000.0).round() as i64;
        (tl.frame_aspect, (q(p.factor), q(p.ox), q(p.oy)))
    });
    let hash = |t| {
        lut_for(tl, t, src)
            .ok()
            .flatten()
            .map(|l| l.hash().to_string())
    };
    let luts = hash(crate::timeline::FrameTarget::Clip(c.id));
    Some(FrameKey {
        luts,
        placement,
        path: src.path_of(m),
        frame: fps.frame_index(c.src_in + offset),
        fade: in_fade.then_some((fin, fout, offset, c.len())),
        size: (g.width, g.height),
        subtitles,
        preset,
        camera,
        line_anchors: project
            .subtitles
            .shown_at(tl, at)
            .into_iter()
            .map(|l| l.anchor)
            .collect(),
        texts: project
            .texts
            .shown_at(at)
            .into_iter()
            .map(|t| {
                let n = if t.effects.any() {
                    export.fps.frame_index(at - t.start)
                } else {
                    0
                };
                (t.clone(), n)
            })
            .collect(),
    })
}

#[cfg(test)]
pub(crate) mod golden_tests;
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::media::ProxyState;
    use crate::music::tests::song;
    use crate::project::Project;
    use crate::timeline::tests::media;

    pub fn project(fmts: &[(u64, u32, u32, u32)]) -> Project {
        let mut p = Project::default();
        for &(secs, w, h, fps) in fmts {
            let id = p.new_media_id();
            let mut m = media(id.0, secs as i64, true);
            m.format.width = w;
            m.format.height = h;
            m.format.fps = Fps::new(fps, 1);
            m.format.bit_rate = Some(1_000_000);
            p.add_media(m.clone());
            p.timeline.append(&m).unwrap();
        }
        p
    }

    pub fn enc() -> std::collections::BTreeSet<String> {
        ["libx264", "libx265", "aac"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn strs(a: &[OsString]) -> Vec<String> {
        a.iter().map(|s| s.to_string_lossy().into_owned()).collect()
    }

    fn args(plan: &Plan) -> Vec<String> {
        args_with(plan, None)
    }

    fn args_with(plan: &Plan, threads: Option<u32>) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("o.mp4");
        let list = dir.path().join("l.txt");
        let script = dir.path().join("g.txt");
        let paths = ExportPaths {
            output: &out,
            concat_list: &list,
            graph_script: &script,
        };
        strs(&export_args(plan, &paths, FfVersion::Release(6, 1, 0), threads, None).unwrap())
    }

    fn s(n: i64) -> Micros {
        Micros(n * 1_000_000)
    }

    fn spec(video: bool, audio: bool) -> GraphSpec {
        GraphSpec {
            video,
            audio,
            pix_fmt: "yuv420p",
            geometry: Geometry {
                width: 640,
                height: 360,
                fps: Fps::new(25, 1).unwrap(),
            },
            subtitles: false,
            mix: Mix::default(),
        }
    }

    // covers: REQ-023
    #[test]
    fn identical_without_effects_uses_stream_copy() {
        let p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        let plan = plan_export(&p, &enc(), |_, _| false).unwrap();
        let Plan::StreamCopy {
            concat_list,
            muxer,
            faststart,
            hevc_tag,
        } = &plan
        else {
            panic!("expected stream copy")
        };
        assert_eq!(muxer, "mp4");
        assert!(faststart);
        assert!(!hevc_tag);
        assert!(concat_list.contains("file 'file:/m/1.mp4'\ninpoint 0.000000\noutpoint 2.000000"));
        assert!(plan.subtitles().is_none());
        let a = args(&plan);
        assert!(a.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(a.windows(2).any(|w| w == ["-safe", "0"]));
        assert!(!a.contains(&"-tag:v".to_string()));
        assert!(a.last().unwrap().starts_with("file:"));
    }

    // covers: REQ-023
    #[test]
    fn non_keyframe_start_or_fade_forces_reencode() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        let id = p.timeline.clips[1].id;
        p.timeline
            .trim(id, Micros(500_000), s(3), s(3), Micros(1))
            .unwrap();
        let mut asked = Vec::new();
        let plan = plan_export(&p, &enc(), |m, t| {
            asked.push((m.id, t));
            false
        })
        .unwrap();
        assert!(matches!(plan, Plan::Reencode { .. }));
        assert_eq!(asked.len(), 1);
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        assert!(matches!(plan, Plan::StreamCopy { .. }));
        p.timeline.toggle_fade(0).unwrap();
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        assert!(matches!(plan, Plan::Reencode { .. }));
    }

    // covers: REQ-023, REQ-019, REQ-020
    #[test]
    fn music_volume_or_subtitles_force_reencode() {
        let base = project(&[(4, 1280, 720, 30)]);
        assert!(matches!(
            plan_export(&base, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        // music
        let mut p = base.clone();
        let m = song(9, 10);
        p.add_media(m.clone());
        p.music.place(&m, s(1), Micros(1)).unwrap();
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::Reencode { .. }
        ));
        // music volume alone (no music clips) does not matter; video volume does
        let mut p = base.clone();
        p.mix.music_volume = 10;
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        p.mix.video_muted = true;
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let Plan::Reencode { graph, .. } = &plan else {
            panic!()
        };
        assert!(
            graph
                .graph
                .ends_with("concat=n=1:v=1:a=1[outv][ca];[ca]volume=0.000000[outa]")
        );
        // subtitles
        let mut p = base.clone();
        let tl = p.timeline.clone();
        p.subtitles.add_at(&tl, s(1), "hi", Micros(1)).unwrap();
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let Plan::Reencode {
            graph, subtitles, ..
        } = &plan
        else {
            panic!()
        };
        assert!(
            graph
                .graph
                .contains("[cv]subtitles=f=subs.ass:fontsdir=fonts[outv]")
        );
        let doc = subtitles.as_deref().unwrap();
        assert!(doc.contains("PlayResX: 1280\nPlayResY: 720"));
        assert!(doc.contains(",hi"));
        assert_eq!(plan.subtitles(), Some(doc));
    }

    // covers: REQ-022
    #[test]
    fn mixed_formats_scale_and_pad_to_lowest() {
        let p = project(&[(2, 1280, 720, 30), (2, 640, 360, 25)]);
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let Plan::Reencode {
            graph, settings, ..
        } = &plan
        else {
            panic!("expected re-encode")
        };
        assert_eq!((settings.width, settings.height), (640, 360));
        assert_eq!(settings.fps, Fps::new(25, 1).unwrap());
        let g = &graph.graph;
        assert!(g.contains("scale=640:360:force_original_aspect_ratio=decrease"));
        assert!(g.contains("pad=640:360:(ow-iw)/2:(oh-ih)/2:color=black"));
        assert!(g.contains("fps=25/1"));
        assert!(!g.contains("stretch"));
        assert!(g.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"));
        let a = args(&plan);
        assert!(a.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(a.windows(2).any(|w| w == ["-b:v", "1000000"]));
        assert!(a.windows(2).any(|w| w == ["-bufsize", "2000000"]));
        assert!(a.windows(2).any(|w| w == ["-movflags", "+faststart"]));
        assert_eq!(a.iter().filter(|x| *x == "-i").count(), 2);
    }

    // covers: REQ-011
    #[test]
    fn fades_in_graph_video_and_audio() {
        let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25)]);
        p.media[1].format.acodec = None;
        p.timeline.toggle_fade(0).unwrap();
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        assert_eq!(segs[0].fade_out, Micros(500_000));
        assert_eq!(segs[1].fade_in, Micros(500_000));
        let g = build_graph(&segs, None, &spec(true, true)).graph;
        assert!(g.contains("fade=t=out:st=1.500000:d=0.500000"));
        assert!(g.contains("afade=t=out:st=1.500000:d=0.500000"));
        assert!(g.contains("fade=t=in:st=0:d=0.500000"));
        assert!(g.contains("anullsrc=r=48000:cl=stereo,atrim=duration=3.000000"));
        // the total length is the sum of the clips: fades do not shorten anything
        let total: Micros = segs.iter().fold(Micros::ZERO, |a, s| a + s.rendered());
        assert_eq!(total, p.timeline.duration());
    }

    // covers: REQ-037
    #[test]
    fn many_cuts_from_one_source_share_one_input() {
        let mut p = project(&[(60, 640, 360, 25)]);
        let m = p.media[0].clone();
        p.timeline.clips.clear();
        // 200 cuts in source order: 1 input, 1 split
        for i in 0..200 {
            p.timeline.append(&m).unwrap();
            let id = p.timeline.clips[i].id;
            let a = Micros(i as i64 * 250_000);
            p.timeline
                .trim(id, a, a + Micros(80_000), m.duration, Micros(1))
                .unwrap();
        }
        let plan = plan_export(&p, &enc(), |_, _| false).unwrap();
        let Plan::Reencode { graph, .. } = &plan else {
            panic!()
        };
        assert_eq!(graph.inputs.len(), 1);
        assert_eq!(graph.inputs[0].start, Micros::ZERO);
        assert_eq!(graph.inputs[0].len, Micros(199 * 250_000 + 80_000));
        assert!(graph.graph.starts_with("[0:v:0]split=200[v0s0]"));
        assert!(graph.graph.contains("[0:a:0]asplit=200[a0s0]"));
        assert!(
            graph
                .graph
                .contains("[v0s199]trim=start=49.750000:duration=0.080000")
        );
        let a = args(&plan);
        assert_eq!(a.iter().filter(|x| *x == "-i").count(), 1);
        // the long graph goes through a script file
        assert!(a.iter().any(|x| x == "-filter_complex_script"));
        // going back in time, or a far jump, opens a new input
        let spans = vec![
            (PathBuf::from("/a"), s(0), s(1)),
            (PathBuf::from("/a"), s(2), s(3)),
            (PathBuf::from("/a"), s(1), s(2)),
            (PathBuf::from("/b"), s(5), s(6)),
            (PathBuf::from("/b"), s(60), s(61)),
            (PathBuf::from("/a"), s(3), s(4)),
        ];
        let (inputs, idx) = group_inputs(&spans);
        assert_eq!(idx, vec![0, 0, 1, 2, 3, 4]);
        assert_eq!(inputs[0].len, s(3));
        assert_eq!(inputs.len(), 5);
    }

    // covers: REQ-036
    #[test]
    fn hevc_in_mp4_and_mov_is_tagged_hvc1() {
        for (container, tag) in [("mp4", true), ("mov", true), ("matroska", false)] {
            let mut p = project(&[(2, 1280, 720, 30), (2, 1280, 720, 30)]);
            for m in &mut p.media {
                m.format.vcodec = Some("hevc".into());
                m.format.container = container.into();
            }
            // stream copy
            let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
            assert!(matches!(plan, Plan::StreamCopy { .. }));
            let a = args(&plan);
            assert_eq!(
                a.windows(2).any(|w| w == ["-tag:v", "hvc1"]),
                tag,
                "{container}"
            );
            // re-encode
            p.timeline.toggle_fade(0).unwrap();
            let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
            let a = args(&plan);
            assert!(a.windows(2).any(|w| w == ["-c:v", "libx265"]));
            assert_eq!(
                a.windows(2).any(|w| w == ["-tag:v", "hvc1"]),
                tag,
                "{container}"
            );
        }
    }

    // covers: REQ-020, REQ-043
    #[test]
    fn music_bed_volumes_and_end_fade() {
        let mut p = project(&[(10, 640, 360, 25)]);
        let a = song(5, 4);
        let b = song(6, 30);
        p.add_media(a.clone());
        p.add_media(b.clone());
        p.music.place(&a, s(1), Micros(1)).unwrap();
        p.music.place(&b, s(7), Micros(1)).unwrap();
        p.mix = Mix {
            video_volume: 50,
            video_muted: false,
            music_volume: 80,
            music_muted: false,
        };
        let bed = music_bed(&p, Micros::ZERO).unwrap().unwrap();
        // silence where there is no music, music clips cut at the video end, fade-out there
        assert_eq!(
            bed.parts,
            vec![
                BedPart::Gap(s(1)),
                BedPart::Clip {
                    path: "/m/5.mp3".into(),
                    src: Micros::ZERO,
                    len: s(4)
                },
                BedPart::Gap(s(2)),
                BedPart::Clip {
                    path: "/m/6.mp3".into(),
                    src: Micros::ZERO,
                    len: s(3)
                },
            ]
        );
        assert_eq!(bed.len, s(10));
        assert_eq!(bed.fade, Some((Micros(9_500_000), Micros(500_000))));
        let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
            panic!()
        };
        let g = &graph.graph;
        assert_eq!(graph.inputs.len(), 3);
        assert!(
            g.contains("[m0][m1][m2][m3]concat=n=4:v=0:a=1,afade=t=out:st=9.500000:d=0.500000[mb]")
        );
        assert!(g.contains("[ca]volume=0.250000[va];[mb]volume=0.640000[vm]"));
        assert!(g.contains(
            "amix=inputs=2:duration=first:normalize=0,alimiter=limit=0.97:level=0[outa]"
        ));
        assert!(g.contains("[1:a:0]atrim=start=0.000000:duration=4.000000"));
        // preview from 8 s: the second music clip is read from 1 s in
        let bed = music_bed(&p, s(8)).unwrap().unwrap();
        assert_eq!(
            bed.parts,
            vec![BedPart::Clip {
                path: "/m/6.mp3".into(),
                src: s(1),
                len: s(2)
            }]
        );
        // music that ends before the video: no fade; no music at all: no bed
        let mut q = p.clone();
        let last = q.music.clips[1].id;
        q.music.remove(last).unwrap();
        let bed = music_bed(&q, Micros::ZERO).unwrap().unwrap();
        assert_eq!(bed.fade, None);
        assert_eq!(bed.parts.last(), Some(&BedPart::Gap(s(5))));
        assert_eq!(music_bed(&q, s(6)).unwrap(), None);
        assert_eq!(music_bed(&q, s(99)).unwrap(), None);
        q.media[1].missing = true;
        assert!(matches!(
            music_bed(&q, Micros::ZERO),
            Err(RenderError::Missing(_))
        ));
        q.media.remove(1);
        assert_eq!(music_bed(&q, Micros::ZERO), Err(RenderError::UnknownMedia));
    }

    // covers: REQ-043
    #[test]
    fn video_edits_do_not_move_music() {
        let mut p = project(&[(4, 640, 360, 25), (4, 640, 360, 25)]);
        let m = song(9, 3);
        p.add_media(m.clone());
        p.music.place(&m, s(2), Micros(1)).unwrap();
        let before = p.music.clone();
        let first = p.timeline.clips[0].id;
        p.timeline.remove(first).unwrap();
        p.timeline.split(s(1), Micros(1)).unwrap();
        assert_eq!(p.music, before);
        // the same music clip now runs past the shortened video: cut with a fade there
        let id = p.timeline.clips[0].id;
        p.timeline
            .trim(id, Micros::ZERO, Micros(500_000), s(4), Micros(1))
            .unwrap();
        let bed = music_bed(&p, Micros::ZERO).unwrap().unwrap();
        assert_eq!(bed.len, s(3) + Micros(500_000));
        assert!(bed.fade.is_some());
    }

    // covers: REQ-013
    #[test]
    fn preview_starts_mid_timeline_at_360p() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        p.timeline.toggle_fade(0).unwrap();
        let segs = segments(&p, s(2) + Micros(500_000), Source::Export).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].skip, Micros(500_000));
        assert_eq!(segs[0].rendered(), Micros(2_500_000));
        let eg = output_geometry(&p, &enc()).unwrap();
        let g = eg.preview();
        assert_eq!((g.width, g.height), (640, 360));
        let dir = tempfile::tempdir().unwrap();
        let sc = dir.path().join("g");
        let src = Source::Preview { proxies: true };
        let r = preview_video_args(&p, Micros(2_500_000), &eg, FfVersion::Dev, &sc, src).unwrap();
        assert!(r.subtitles.is_none());
        let a = strs(&r.args);
        assert!(a.windows(2).any(|w| w == ["-ss", "0.500000"]));
        assert!(a.windows(2).any(|w| w == ["-pix_fmt", "rgba"]));
        let fc = a.iter().position(|x| x == "-filter_complex").unwrap();
        assert!(a[fc + 1].contains("setpts=PTS-STARTPTS+0.500000/TB"));
        assert!(a[fc + 1].contains("fade=t=in:st=0:d=0.500000"));
        let a = strs(
            &preview_audio_args(&p, Micros::ZERO, 44_100, 2, FfVersion::Dev, &sc, src).unwrap(),
        );
        assert!(a.windows(2).any(|w| w == ["-ar", "44100"]));
        assert!(a.windows(2).any(|w| w == ["-f", "s16le"]));
        assert!(preview_video_args(&p, s(99), &eg, FfVersion::Dev, &sc, src).is_err());
        assert!(preview_audio_args(&p, s(99), 1, 1, FfVersion::Dev, &sc, src).is_err());
        assert!(still_args(&p, s(99), &eg, FfVersion::Dev, &sc, src).is_err());
        let small = Geometry {
            width: 320,
            height: 240,
            fps: Fps::new(25, 1).unwrap(),
        };
        assert_eq!(small.preview(), small);
        // a music clip that cannot be found stops the audio preview
        let m = song(9, 3);
        p.add_media(MediaItem {
            missing: true,
            ..m.clone()
        });
        p.music.place(&m, Micros::ZERO, Micros(1)).unwrap();
        assert!(preview_audio_args(&p, Micros::ZERO, 1, 1, FfVersion::Dev, &sc, src).is_err());
    }

    // covers: REQ-034, REQ-017
    #[test]
    fn still_frame_uses_one_input_and_shows_subtitles() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        let tl = p.timeline.clone();
        p.subtitles.add_at(&tl, s(3), "hello", Micros(1)).unwrap();
        let eg = output_geometry(&p, &enc()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sc = dir.path().join("g");
        let r = still_args(&p, s(3), &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
        let a = strs(&r.args);
        assert_eq!(a.iter().filter(|x| *x == "-i").count(), 1);
        assert!(a.windows(2).any(|w| w == ["-frames:v", "1"]));
        let fc = a.iter().position(|x| x == "-filter_complex").unwrap();
        assert!(a[fc + 1].contains("subtitles=f=subs.ass"));
        // times in the document are relative to the frame
        let doc = r.subtitles.unwrap();
        assert!(doc.contains("Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,hello"));
        // the preview of the whole timeline from 0 carries the same subtitles
        let r =
            preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
        assert!(r.subtitles.unwrap().contains("0:00:03.00,0:00:05.00"));
    }

    // covers: REQ-111, REQ-023
    #[test]
    fn a_text_item_forces_reencode_and_is_burned_in_with_the_subtitles() {
        use crate::text::{TextId, TextItem};
        let mut p = project(&[(4, 1280, 720, 30)]);
        let id = p.texts.new_id();
        let mut t = TextItem::new(id, "Title".into(), s(2));
        t.place = crate::subtitles::Pos { x: 1000, y: 1000 };
        p.texts.add(t).unwrap();
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let Plan::Reencode { graph, .. } = &plan else {
            panic!("a text must force a re-encode")
        };
        assert!(graph.graph.contains("subtitles=f=subs.ass"));
        let doc = plan.subtitles().unwrap();
        assert!(doc.contains("Style: Text,"));
        // 2 s .. 5 s, placed at 10% / 10% of 1280x720
        assert!(
            doc.contains("Dialogue: 1,0:00:02.00,0:00:04.00,Text,,128,128,0,,{"),
            "{doc}"
        );
        assert!(doc.contains("\\pos(128,72)"));
        // the part of the text after a start in the middle is shifted
        let eg = output_geometry(&p, &enc()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sc = dir.path().join("g");
        let r = still_args(&p, s(3), &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
        assert!(r.subtitles.unwrap().contains("0:00:00.00,0:00:01.00,Text"));
        // not visible at 1 s of the preview... the doc from 4 s has no text left
        let r = still_args(&p, s(1), &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
        assert!(r.subtitles.unwrap().contains("0:00:01.00"));
        // the picture key knows about the text and about its effect frame
        let k1 = frame_key(&p, s(1), Source::Export, &eg).unwrap();
        let k3 = frame_key(&p, s(3), Source::Export, &eg).unwrap();
        assert!(k1.texts.is_empty() && k3.texts.len() == 1);
        let mut q = p.clone();
        let mut edited = q.texts.get(TextId(1)).unwrap().clone();
        edited.effects.fade_in = Some(Micros(500_000));
        q.texts.replace(edited).unwrap();
        let a = frame_key(&q, Micros(3_000_000), Source::Export, &eg).unwrap();
        let b = frame_key(&q, Micros(3_100_000), Source::Export, &eg).unwrap();
        assert_ne!(a, b);
        assert_ne!(a, k3);
        let c = frame_key(&p, Micros(3_100_000), Source::Export, &eg).unwrap();
        assert_eq!(c.texts, k3.texts);
    }

    // covers: REQ-034
    #[test]
    fn frame_key_changes_only_with_the_picture() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        let eg = output_geometry(&p, &enc()).unwrap();
        let src = Source::Preview { proxies: true };
        let k = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_eq!(k.frame, 15);
        assert_eq!(k.size, (640, 360));
        assert!(k.fade.is_none());
        // an edit after the playhead does not change the key
        let mut q = p.clone();
        let id = q.timeline.clips[1].id;
        q.timeline.trim(id, s(1), s(2), s(3), Micros(1)).unwrap();
        assert_eq!(frame_key(&q, Micros(500_000), src, &eg), Some(k.clone()));
        // a fade reaching the playhead does
        p.timeline.toggle_fade(0).unwrap();
        assert_eq!(frame_key(&p, Micros(500_000), src, &eg), Some(k.clone()));
        let kf = frame_key(&p, Micros(1_800_000), src, &eg).unwrap();
        assert!(kf.fade.is_some());
        // subtitles on screen and the highlighted word are part of it
        let tl = p.timeline.clone();
        p.subtitles
            .add_at(&tl, Micros(400_000), "a b", Micros(1))
            .unwrap();
        let ks = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_eq!(ks.subtitles, vec![("a b".to_string(), None, None, None)]);
        p.subtitles.preset = Preset::WordHighlight;
        let kw = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_eq!(kw.subtitles, vec![("a b".to_string(), Some(0), None, None)]);
        // a ready proxy changes the file read by the preview but never the export
        p.media[0].proxy = ProxyState::Ready("/cache/x.mkv".into());
        let kp = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_eq!(kp.path, PathBuf::from("/cache/x.mkv"));
        let off = frame_key(&p, Micros(500_000), Source::Preview { proxies: false }, &eg).unwrap();
        assert_eq!(off.path, PathBuf::from("/m/1.mp4"));
        assert!(frame_key(&p, s(99), src, &eg).is_none());
    }

    // covers: REQ-038
    #[test]
    fn export_plan_is_identical_with_proxies_on_and_off() {
        let mut p = project(&[(2, 3840, 2160, 30), (3, 1280, 720, 30)]);
        p.timeline.toggle_fade(0).unwrap();
        let without = plan_export(&p, &enc(), |_, _| true).unwrap();
        p.media[0].proxy = ProxyState::Ready("/cache/a.mkv".into());
        let with = plan_export(&p, &enc(), |_, _| true).unwrap();
        assert_eq!(with, without);
        assert!(!format!("{with:?}").contains("/cache/"));
        let pv = segments(&p, Micros::ZERO, Source::Preview { proxies: true }).unwrap();
        assert_eq!(pv[0].path, PathBuf::from("/cache/a.mkv"));
    }

    #[test]
    fn errors_and_quoting() {
        let p = Project::default();
        assert_eq!(
            segments(&p, Micros::ZERO, Source::Export),
            Err(RenderError::Empty)
        );
        assert_eq!(
            plan_export(&p, &enc(), |_, _| true),
            Err(RenderError::Empty)
        );
        let mut p = project(&[(2, 640, 360, 25)]);
        p.media[0].missing = true;
        assert_eq!(
            segments(&p, Micros::ZERO, Source::Export),
            Err(RenderError::Missing("1.mp4".into()))
        );
        p.media.clear();
        assert_eq!(
            segments(&p, Micros::ZERO, Source::Export),
            Err(RenderError::UnknownMedia)
        );
        assert_eq!(timeline_media(&p), Err(RenderError::UnknownMedia));
        assert_eq!(
            concat_quote(Path::new("/a/it's.mp4")),
            "'file:/a/it'\\''s.mp4'"
        );
    }

    #[test]
    fn other_containers_stream_copy_muxers() {
        for (c, m, fs) in [
            ("mov", "mov", true),
            ("matroska", "matroska", false),
            ("webm", "webm", false),
            ("avi", "avi", false),
        ] {
            let mut p = project(&[(2, 640, 360, 25)]);
            p.media[0].format.container = c.into();
            let Plan::StreamCopy {
                muxer, faststart, ..
            } = plan_export(&p, &enc(), |_, _| true).unwrap()
            else {
                panic!()
            };
            assert_eq!((muxer.as_str(), faststart), (m, fs), "{c}");
        }
    }

    // covers: REQ-114, REQ-023
    #[test]
    fn clean_audio_is_applied_before_the_volume_and_the_fades_and_forces_a_reencode() {
        let mut p = project(&[(4, 640, 360, 25), (4, 640, 360, 25)]);
        let (a, b) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
        let plain = plan_export(&p, &enc(), |_, _| true).unwrap();
        assert!(matches!(plain, Plan::StreamCopy { .. }));
        // without it the sound chain is unchanged
        let g = build_graph(
            &segments(&p, Micros::ZERO, Source::Export).unwrap(),
            None,
            &spec(true, true),
        )
        .graph;
        assert!(!g.contains("afftdn"));
        // (d) a clip with it forces a re-encode
        p.timeline.set_clean_audio(&[a], true).unwrap();
        p.timeline.set_clip_volume(a, 50, false).unwrap();
        p.timeline.toggle_fade(0).unwrap();
        let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
            panic!("clean audio must re-encode")
        };
        let parts: Vec<&str> = graph.graph.split(';').collect();
        let a0 = parts.iter().find(|x| x.ends_with("[a0]")).unwrap();
        let a1 = parts.iter().find(|x| x.ends_with("[a1]")).unwrap();
        // before the clip's own volume and the fades
        let clean = a0.find(CLEAN_AUDIO).expect(a0);
        assert!(clean < a0.find("volume=").unwrap() && clean < a0.find("afade").unwrap());
        assert!(!a1.contains("afftdn"), "other clips untouched: {a1}");
        // the whole video: every clip
        p.timeline.set_clean_audio(&[a, b], true).unwrap();
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        assert!(segs.iter().all(|s| s.clean));
        assert_eq!(
            build_graph(&segs, None, &spec(true, true))
                .graph
                .matches("afftdn")
                .count(),
            2
        );
        // the preview uses the same chain
        let dir = tempfile::tempdir().unwrap();
        let a = preview_audio_args(
            &p,
            Micros::ZERO,
            44_100,
            2,
            FfVersion::Dev,
            &dir.path().join("g"),
            Source::Export,
        )
        .unwrap();
        assert!(strs(&a).iter().any(|x| x.contains(CLEAN_AUDIO)));
        // the music bed is not touched by it
        let mut m = p.clone();
        let song = song(9, 10);
        m.add_media(song.clone());
        m.music.place(&song, Micros::ZERO, Micros(1)).unwrap();
        let g = build_graph(
            &segments(&m, Micros::ZERO, Source::Export).unwrap(),
            music_bed(&m, Micros::ZERO).unwrap().as_ref(),
            &spec(true, true),
        )
        .graph;
        let bed = g.split(';').find(|x| x.ends_with("[mb]")).unwrap();
        assert!(!bed.contains("afftdn"));
        // a clip without sound has nothing to clean
        let mut silent = p.clone();
        silent.media[0].format.acodec = None;
        silent.media[0].format.channels = None;
        let segs = segments(&silent, Micros::ZERO, Source::Export).unwrap();
        let g = build_graph(&segs, None, &spec(true, true)).graph;
        assert_eq!(g.matches("afftdn").count(), usize::from(segs[1].has_audio));
    }

    // covers: REQ-120, REQ-121, REQ-023
    #[test]
    fn a_chosen_video_or_audio_format_is_always_reencoded_with_its_settings() {
        use crate::format::{AudioChoice, FormatChoice};
        let p = project(&[(4, 1280, 720, 30)]);
        let all: std::collections::BTreeSet<String> = ["libx264", "libx265", "aac", "libmp3lame"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plan = |c: FormatChoice, e: &std::collections::BTreeSet<String>| {
            plan_export_choice(&p, e, ExportMode::Automatic, &c, |_, _| true)
        };
        // "Same as source" for both: stream copy where it is allowed
        assert!(matches!(
            plan(FormatChoice::default(), &all).unwrap(),
            Plan::StreamCopy { .. }
        ));
        // 1280x720 25 fps libx264 .mp4: that size, rate and codec
        let Plan::Reencode {
            graph, settings, ..
        } = plan(
            FormatChoice {
                video: Some(6),
                audio: AudioChoice::Same,
            },
            &all,
        )
        .unwrap()
        else {
            panic!("a preset must re-encode")
        };
        assert_eq!((settings.width, settings.height), (1280, 720));
        assert_eq!(settings.fps, Fps::new(25, 1).unwrap());
        assert_eq!(settings.vencoder, "libx264");
        assert!(
            graph
                .graph
                .contains("scale=1280:720:force_original_aspect_ratio=decrease")
        );
        assert!(graph.graph.contains("fps=25"));
        // HEVC in MP4 carries hvc1 (REQ-036) and the sizes of 4K are kept
        let Plan::Reencode { settings, .. } = plan(
            FormatChoice {
                video: Some(10),
                audio: AudioChoice::Same,
            },
            &all,
        )
        .unwrap() else {
            unreachable!()
        };
        assert_eq!((settings.width, settings.height), (3840, 2160));
        let hevc = Plan::Reencode {
            graph: Box::new(build_graph(&[], None, &spec(true, true))),
            settings,
            subtitles: None,
        };
        assert!(args(&hevc).windows(2).any(|w| w == ["-tag:v", "hvc1"]));
        // an audio format re-encodes even with the same video
        let Plan::Reencode { settings, .. } = plan(
            FormatChoice {
                video: None,
                audio: AudioChoice::Mp3,
            },
            &all,
        )
        .unwrap() else {
            panic!("a chosen audio format must re-encode")
        };
        assert_eq!(settings.aencoder, "libmp3lame");
        // missing encoders are refused, not replaced
        let no_mp3 = enc();
        assert_eq!(
            plan(
                FormatChoice {
                    video: None,
                    audio: AudioChoice::Mp3
                },
                &no_mp3
            ),
            Err(RenderError::NoEncoder)
        );
        let h264_only: std::collections::BTreeSet<String> =
            ["libx264", "aac"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            plan(
                FormatChoice {
                    video: Some(10),
                    audio: AudioChoice::Same
                },
                &h264_only
            ),
            Err(RenderError::NoEncoder)
        );
    }

    // covers: REQ-119, REQ-121, REQ-114
    #[test]
    fn only_audio_makes_the_sound_of_the_whole_timeline_without_any_video() {
        use crate::format::AudioChoice;
        let mut p = project(&[(4, 1280, 720, 30), (3, 1280, 720, 30)]);
        let first = p.timeline.clips[0].id;
        p.timeline.set_clean_audio(&[first], true).unwrap();
        let m = song(9, 10);
        p.add_media(m.clone());
        p.music.place(&m, s(1), Micros(1)).unwrap();
        p.mix.music_volume = 50;
        let all: std::collections::BTreeSet<String> = ["libx264", "aac", "libmp3lame"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plan = plan_audio_only(&p, &all, AudioChoice::Same).unwrap();
        assert_eq!(plan.settings.extension, "m4a");
        // no picture is built: no video filter at all, the clean chain and the music are there
        let g = &plan.graph.graph;
        assert!(
            !g.contains("scale=")
                && !g.contains("[outv]")
                && !g.contains("setsar")
                && !g.contains(":v:0]")
        );
        assert!(g.contains(CLEAN_AUDIO) && g.contains("[mb]") && g.contains("[outa]"));
        assert!(
            g.contains("volume=0.250000"),
            "the music volume applies: {g}"
        );
        let dir = tempfile::tempdir().unwrap();
        let (out, list, script) = (
            dir.path().join("o.m4a"),
            dir.path().join("l"),
            dir.path().join("g"),
        );
        let paths = ExportPaths {
            output: &out,
            concat_list: &list,
            graph_script: &script,
        };
        let a = strs(&audio_export_args(&plan, &paths, FfVersion::Dev, Some(2)).unwrap());
        assert!(a.windows(2).any(|w| w == ["-map", "[outa]"]));
        assert!(a.contains(&"-vn".to_string()) && !a.contains(&"-c:v".to_string()));
        assert!(a.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(a.windows(2).any(|w| w == ["-f", "mp4"]));
        assert!(a.iter().any(|x| x == "-threads"));
        assert!(a.last().unwrap().ends_with("o.m4a"));
        // mp3 only when the encoder is there
        let mp3 = plan_audio_only(&p, &all, AudioChoice::Mp3).unwrap();
        assert_eq!(
            (
                mp3.settings.extension.as_str(),
                mp3.settings.aencoder.as_str()
            ),
            ("mp3", "libmp3lame")
        );
        let a = strs(&audio_export_args(&mp3, &paths, FfVersion::Dev, None).unwrap());
        assert!(a.windows(2).any(|w| w == ["-f", "mp3"]));
        assert_eq!(
            plan_audio_only(&p, &enc(), AudioChoice::Mp3),
            Err(RenderError::NoEncoder)
        );
        assert_eq!(
            plan_audio_only(&Project::default(), &all, AudioChoice::Aac),
            Err(RenderError::Empty)
        );
    }

    // covers: REQ-084, REQ-020, REQ-023
    #[test]
    fn clip_volume_is_a_gain_on_that_clip_only() {
        let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25)]);
        let (a, b) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        // (d) a clip level below 100 % or muted forces a re-encode
        p.timeline.set_clip_volume(a, 50, false).unwrap();
        let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
            panic!("clip volume must re-encode")
        };
        let parts: Vec<&str> = graph.graph.split(';').collect();
        let a0 = parts.iter().find(|x| x.ends_with("[a0]")).unwrap();
        let a1 = parts.iter().find(|x| x.ends_with("[a1]")).unwrap();
        assert!(a0.contains(",volume=0.250000"), "{a0}");
        assert!(!a1.contains("volume="), "other clips untouched: {a1}");
        // (b) muted: silence for that clip only
        p.timeline.set_clip_volume(b, 100, true).unwrap();
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        assert_eq!(segs[1].gain, 0.0);
        let g = build_graph(&segs, None, &spec(true, true)).graph;
        assert!(g.contains(",volume=0.000000"));
        // (a) clip 50 % with the video at 100 % gives the same gain as clip 100 % with
        // the video at 50 %
        let mut x = project(&[(2, 640, 360, 25)]);
        let id = x.timeline.clips[0].id;
        x.timeline.set_clip_volume(id, 50, false).unwrap();
        let clip_gain =
            segments(&x, Micros::ZERO, Source::Export).unwrap()[0].gain * x.mix.video_gain();
        let mut y = project(&[(2, 640, 360, 25)]);
        y.mix.video_volume = 50;
        let video_gain =
            segments(&y, Micros::ZERO, Source::Export).unwrap()[0].gain * y.mix.video_gain();
        assert!((clip_gain - video_gain).abs() < 1e-12);
        // the preview's sound uses the same graph
        let dir = tempfile::tempdir().unwrap();
        let a = strs(
            &preview_audio_args(
                &p,
                Micros::ZERO,
                48_000,
                2,
                FfVersion::Release(6, 1, 0),
                &dir.path().join("g.txt"),
                Source::Preview { proxies: false },
            )
            .unwrap(),
        );
        assert!(a.iter().any(|x| x.contains("volume=0.250000")));
    }

    // covers: REQ-083, REQ-011, REQ-023
    #[test]
    fn start_and_end_fades_are_rendered_at_the_video_edges() {
        let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25), (2, 640, 360, 25)]);
        p.timeline.set_edge_fades(true).unwrap();
        let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
            panic!("edge fades must re-encode")
        };
        let parts: Vec<&str> = graph.graph.split(';').collect();
        let v0 = parts.iter().find(|x| x.ends_with("[v0]")).unwrap();
        let v1 = parts.iter().find(|x| x.ends_with("[v1]")).unwrap();
        let v2 = parts.iter().find(|x| x.ends_with("[v2]")).unwrap();
        assert!(v0.contains("fade=t=in:st=0:d=0.500000") && !v0.contains("t=out"));
        assert!(!v1.contains("fade="), "{v1}");
        // the picture's fade ends two frames early: the very last frame is black
        assert!(v2.contains("fade=t=out:st=1.420000:d=0.500000") && !v2.contains("t=in"));
        let a2 = parts.iter().find(|x| x.ends_with("[a2]")).unwrap();
        assert!(a2.contains("afade=t=out:st=1.500000:d=0.500000"));
        let a0 = parts.iter().find(|x| x.ends_with("[a0]")).unwrap();
        assert!(a0.contains("afade=t=in:st=0"));
        // the preview's first and last frames are inside the fades
        let g = Geometry {
            width: 640,
            height: 360,
            fps: Fps { num: 25, den: 1 },
        };
        let first = frame_key(&p, Micros::ZERO, Source::Export, &g).unwrap();
        assert!(first.fade.is_some());
        let last = frame_key(
            &p,
            p.timeline.duration() - Micros(40_000),
            Source::Export,
            &g,
        );
        assert!(last.unwrap().fade.is_some());
        // the start fade belongs to the first clip: cutting it removes the fade (REQ-083)
        let first_id = p.timeline.clips[0].id;
        p.timeline.remove(first_id).unwrap();
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        assert_eq!(segs[0].fade_in, Micros::ZERO);
    }

    fn sphere_project() -> Project {
        let mut p = project(&[(4, 3840, 1920, 30), (3, 640, 360, 30)]);
        p.media[0].format.spherical = Some(true);
        p.media[0].format.bit_rate = Some(40_000_000);
        p
    }

    // covers: REQ-103
    #[test]
    fn a_360_video_keeps_the_sphere_and_refuses_a_mix_with_flat_clips() {
        use crate::text::{Anchor360, TextId, TextItem};
        let mut p = project(&[(2, 3840, 1920, 30), (2, 3840, 1920, 30)]);
        p.media[0].format.spherical = Some(true);
        p.media[1].format.spherical = Some(true);
        // nothing to burn in, identical clips: the file can be copied as it is
        assert!(matches!(
            plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        // a text fixed to the sphere at yaw 30 forces a re-encode: no camera in the graph,
        // the equirect picture keeps the clips' size, the text is burned on every clip
        let id = p.texts.new_id();
        let mut t = TextItem::new(id, "North".into(), Micros::ZERO);
        t.ttl = Micros(4_000_000);
        t.anchor = Some(Anchor360 {
            yaw_cd: 3000,
            pitch_cd: 1000,
            height_cd: 500,
            width_cd: 6000,
        });
        p.texts.add(t).unwrap();
        let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
        let Plan::Reencode {
            graph, settings, ..
        } = &plan
        else {
            panic!("re-encoded")
        };
        assert_eq!((settings.width, settings.height), (3840, 1920));
        assert!(!graph.graph.contains("v360") && !graph.graph.contains("sendcmd"));
        assert!(
            graph.graph.contains("subtitles=f=anc0.ass:fontsdir=fonts"),
            "{}",
            graph.graph
        );
        assert!(graph.graph.contains("subtitles=f=anc1.ass"));
        let doc = &graph
            .command_files
            .iter()
            .find(|(n, _)| n == "anc0.ass")
            .unwrap()
            .1;
        // yaw 30 / pitch 10 on 3840x1920: x = (30/360 + 0.5) * 3840, y = (0.5 - 10/180) * 1920
        assert!(doc.contains("\\pos(2240,853)"), "{doc}");
        assert!(doc.contains("PlayResX: 3840\nPlayResY: 1920"));
        // the flat file has nothing of it, but exists so that the fonts are written
        assert!(!plan.subtitles().unwrap().contains("Dialogue"));
        // the same project as a flat camera view: v360 per clip, the text burned before it
        let Plan::Reencode { graph, .. } =
            plan_export_mode(&p, &enc(), ExportMode::FlatCamera, |_, _| true).unwrap()
        else {
            panic!()
        };
        assert!(graph.graph.contains("subtitles=f=anc0.ass") && graph.graph.contains("v360@c0"));
        let (a, b) = (
            graph.graph.find("anc0.ass").unwrap(),
            graph.graph.find("v360@c0").unwrap(),
        );
        assert!(a < b, "burned before the camera view is cut out");
        // a flat clip in the mix: refused, not exported
        let mut q = project(&[(2, 3840, 1920, 30), (2, 1280, 720, 30)]);
        q.media[0].format.spherical = Some(true);
        assert_eq!(
            plan_export_mode(&q, &enc(), ExportMode::Sphere360, |_, _| true).unwrap_err(),
            RenderError::MixedSphere
        );
        assert!(
            RenderError::MixedSphere
                .to_string()
                .contains("Flat camera view")
                || RenderError::MixedSphere
                    .to_string()
                    .contains("flat camera view")
        );
        let _ = TextId(0);
    }

    // covers: REQ-112, REQ-103
    #[test]
    fn a_line_fixed_to_the_sphere_is_burned_on_the_equirect_picture_and_not_on_the_flat_one() {
        use crate::text::Anchor360;
        let mut p = sphere_project();
        let tl = p.timeline.clone();
        let a = p
            .subtitles
            .add_at(&tl, Micros(1_000_000), "pinned", Micros(1))
            .unwrap();
        let b = p
            .subtitles
            .add_at(&tl, Micros(5_000_000), "flat clip", Micros(1))
            .unwrap();
        let c = p
            .subtitles
            .add_at(&tl, Micros(2_000_000), "automatic", Micros(1))
            .unwrap();
        let anchor = Anchor360 {
            yaw_cd: 3000,
            pitch_cd: 0,
            height_cd: 500,
            width_cd: 6000,
        };
        for id in [a, b] {
            p.subtitles
                .lines
                .iter_mut()
                .find(|l| l.id == id)
                .unwrap()
                .anchor = Some(anchor);
        }
        let _ = c;
        let eg = Geometry {
            width: 1280,
            height: 720,
            fps: crate::time::Fps::new(30, 1).unwrap(),
        };
        let flat = subtitles_doc(&p, &eg, Micros::ZERO).unwrap();
        assert!(!flat.contains(",pinned"), "{flat}");
        assert!(flat.contains(",automatic") && flat.contains(",flat clip"));
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        let doc = segs[0].anchored.as_ref().unwrap();
        assert!(doc.contains("pinned") && !doc.contains("automatic") && !doc.contains("flat clip"));
        assert!(segs[1].anchored.is_none());
        // 360 video: no camera, the line is burned on the sphere picture
        let plan = plan_export_mode(&p, &enc(), ExportMode::FlatCamera, |_, _| true).unwrap();
        let Plan::Reencode { graph, .. } = plan else {
            panic!()
        };
        assert!(graph.graph.contains("subtitles=f=anc0.ass"));
        // the picture key knows where lines are fixed
        let eg2 = output_geometry(&p, &enc()).unwrap();
        let k = frame_key(&p, Micros(1_500_000), Source::Export, &eg2).unwrap();
        assert_eq!(k.line_anchors, [Some(anchor)]);
    }

    // covers: REQ-112
    #[test]
    fn a_text_fixed_to_the_sphere_is_shown_flat_only_on_flat_clips() {
        use crate::text::{Anchor360, TextItem};
        // a 360 clip (0-4 s) then a flat one (4-7 s); the text lives from 2 s to 6 s
        let mut p = sphere_project();
        let id = p.texts.new_id();
        let mut t = TextItem::new(id, "Pin".into(), Micros(2_000_000));
        t.ttl = Micros(4_000_000);
        t.anchor = Some(Anchor360 {
            yaw_cd: -17900,
            pitch_cd: 0,
            height_cd: 500,
            width_cd: 6000,
        });
        p.texts.add(t).unwrap();
        let eg = Geometry {
            width: 1280,
            height: 720,
            fps: crate::time::Fps::new(30, 1).unwrap(),
        };
        let flat = subtitles_doc(&p, &eg, Micros::ZERO).unwrap();
        // flat part: 4 s .. 6 s only
        assert!(
            flat.contains("Dialogue: 1,0:00:04.00,0:00:06.00,Text"),
            "{flat}"
        );
        assert_eq!(flat.matches("Dialogue: 1").count(), 1);
        // the 360 clip: its own file, 2 s .. 4 s, local time from the clip start; the text
        // is near the left edge (yaw -179): drawn also at the right edge (the seam)
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        let doc = segs[0].anchored.as_ref().unwrap();
        assert!(
            doc.contains("Dialogue: 1,0:00:02.00,0:00:04.00,Text"),
            "{doc}"
        );
        assert_eq!(doc.matches("Dialogue: 1").count(), 2, "{doc}");
        assert!(segs[1].anchored.is_none());
        // a preview from 3 s: local time 0 is 3 s
        let segs = segments(&p, Micros(3_000_000), Source::Export).unwrap();
        assert!(
            segs[0]
                .anchored
                .as_ref()
                .unwrap()
                .contains("Dialogue: 1,0:00:00.00,0:00:01.00,Text")
        );
        // without a 360 clip the text is at its flat place
        let mut q = project(&[(4, 1280, 720, 30)]);
        let id = q.texts.new_id();
        let mut t = TextItem::new(id, "Flat".into(), Micros::ZERO);
        t.anchor = Some(Anchor360 {
            yaw_cd: 0,
            pitch_cd: 0,
            height_cd: 500,
            width_cd: 6000,
        });
        q.texts.add(t).unwrap();
        assert!(
            subtitles_doc(&q, &eg, Micros::ZERO)
                .unwrap()
                .contains("Dialogue: 1")
        );
    }

    // covers: REQ-103, REQ-023, REQ-022
    #[test]
    fn a_360_clip_is_never_stream_copied_and_counts_as_1080p() {
        let mut p = project(&[(2, 3840, 1920, 30), (2, 3840, 1920, 30)]);
        // flat: identical formats copy
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        p.media[0].format.spherical = Some(true);
        let Plan::Reencode {
            graph, settings, ..
        } = plan_export(&p, &enc(), |_, _| true).unwrap()
        else {
            panic!("a 360 clip is re-encoded");
        };
        assert_eq!((settings.width, settings.height), (1920, 1080));
        assert!(
            graph
                .graph
                .contains("v360@c0=input=e:output=flat:reset_rot=1:w=1920:h=1080")
        );
        assert!(
            !graph.graph.split(';').next().unwrap().contains("scale="),
            "no bars for the camera view"
        );
        // the huge bit rate of the sphere is not given to the flat picture
        assert!(settings.video_bitrate.unwrap() < 10_000_000);
        // without the file's spherical information it is flat again
        p.media[0].format.spherical = Some(false);
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        assert_eq!(effective_format(&p.media[0]).width, 3840);
    }

    // covers: REQ-103, REQ-022
    #[test]
    fn a_360_clip_next_to_a_normal_one_follows_the_usual_choice() {
        let p = sphere_project();
        let g = output_geometry(&p, &enc()).unwrap();
        assert_eq!((g.width, g.height), (640, 360));
        let q = {
            let mut q = project(&[(4, 3840, 1920, 25)]);
            q.media[0].format.spherical = Some(true);
            q
        };
        let g = output_geometry(&q, &enc()).unwrap();
        assert_eq!(
            (g.width, g.height, g.fps),
            (1920, 1080, Fps::new(25, 1).unwrap())
        );
    }

    // covers: REQ-102, REQ-103
    #[test]
    fn the_camera_path_becomes_one_command_file_for_each_moving_clip() {
        let mut p = sphere_project();
        let m = p.media[0].id;
        let pov = |t: i64, yaw: i64| crate::camera::Pov {
            t: Micros(t),
            cam: Camera::new(yaw * 100, 0, 9000),
        };
        // no POV: the camera stays at the front, no command file
        let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
            panic!()
        };
        assert!(graph.command_files.is_empty() && !graph.graph.contains("sendcmd"));
        p.camera.set(m, pov(0, 0)).unwrap();
        p.camera.set(m, pov(2_000_000, 90)).unwrap();
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let Plan::Reencode { graph, .. } = &plan else {
            panic!()
        };
        assert_eq!(graph.command_files.len(), 1);
        let (name, text) = &graph.command_files[0];
        assert_eq!(name, "cam0.txt");
        assert_eq!(
            text.lines().count(),
            60,
            "one line per frame while it moves"
        );
        assert!(graph.graph.contains("fps=30/1,sendcmd=f=cam0.txt,v360@c0="));
        assert_eq!(plan.command_files().len(), 1);
        let dir = tempfile::tempdir().unwrap();
        write_command_files(dir.path(), plan.command_files()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("cam0.txt")).unwrap(),
            *text
        );
        // a clip cut at its start starts at the view the path had there
        let id = p.timeline.clips[0].id;
        p.timeline
            .trim(
                id,
                Micros(1_000_000),
                Micros(4_000_000),
                Micros(4_000_000),
                Micros(1),
            )
            .unwrap();
        let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
            panic!()
        };
        assert!(graph.graph.contains("yaw=45.00"), "{}", graph.graph);
        // a normal clip is untouched by it all
        assert!(graph.graph.contains("[1:v:0]trim") || graph.graph.contains("scale=640:360"));
        // preview of a later start: the offset comes after the camera
        let eg = output_geometry(&p, &enc()).unwrap();
        let r = preview_video_args(
            &p,
            Micros(500_000),
            &eg,
            FfVersion::Dev,
            Path::new("/tmp/g.txt"),
            Source::Export,
        )
        .unwrap();
        assert_eq!(r.command_files.len(), 1);
        let a = strs(&r.args).join(" ");
        assert!(
            a.contains("v360@c0") && a.contains("setpts=PTS+0.500000/TB"),
            "{a}"
        );
    }

    // covers: REQ-101, REQ-102
    #[test]
    fn the_picture_key_follows_the_camera() {
        let mut p = sphere_project();
        let m = p.media[0].id;
        let eg = output_geometry(&p, &enc()).unwrap();
        let src = Source::Export;
        let k0 = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_eq!(k0.camera, Some(Camera::FRONT));
        p.camera
            .set(
                m,
                crate::camera::Pov {
                    t: Micros::ZERO,
                    cam: Camera::new(1000, 0, 9000),
                },
            )
            .unwrap();
        let k1 = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_ne!(k0, k1);
        // a normal clip has no camera
        let n = frame_key(&p, Micros(4_500_000), src, &eg).unwrap();
        assert_eq!(n.camera, None);
    }

    // covers: REQ-104
    #[test]
    fn a_sphere_too_large_for_the_preview_is_not_decoded_but_still_exported() {
        let mut p = project(&[(2, 20_000, 10_000, 30)]);
        p.media[0].format.spherical = Some(true);
        let eg = Geometry {
            width: 1920,
            height: 1080,
            fps: Fps::new(30, 1).unwrap(),
        };
        let sc = Path::new("/tmp/g.txt");
        let src = Source::Preview { proxies: true };
        let e = still_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src).unwrap_err();
        assert_eq!(
            e,
            "This 360° video is too large to show here. It is still exported."
        );
        assert!(preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src).is_err());
        assert_eq!(
            segments(&p, Micros::ZERO, src),
            Err(RenderError::SphereTooLarge)
        );
        // the export is planned as usual
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::Reencode { .. }
        ));
        // a normal video of the same size is not subject to this rule
        p.media[0].format.spherical = Some(false);
        assert!(segments(&p, Micros::ZERO, src).is_ok());
    }

    // ---- 1.5.0: Change aspect, Scale and Move ----

    use crate::frame::{Fill, Fit, FrameAspect, Rgb, Scale, Shape};
    use crate::timeline::FrameTarget;

    fn graph_of(p: &Project) -> (Graph, Geometry, bool) {
        let plan = plan_export(p, &enc(), |_, _| true).unwrap();
        let Plan::Reencode {
            graph, settings, ..
        } = plan
        else {
            return (
                Graph {
                    inputs: vec![],
                    graph: String::new(),
                    command_files: vec![],
                },
                Geometry {
                    width: 0,
                    height: 0,
                    fps: Fps::new(1, 1).unwrap(),
                },
                false,
            );
        };
        let g = Geometry::of(&settings);
        (*graph, g, true)
    }

    fn tall(fit: Fit, fill: Fill) -> FrameAspect {
        FrameAspect {
            shape: Shape::Tall9x16,
            fit,
            fill,
        }
    }

    // covers: REQ-131
    // covers: REQ-023
    #[test]
    fn any_frame_effect_makes_a_reencode_even_at_the_defaults() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        let id = p.timeline.clips[0].id;
        p.timeline.add_scale(FrameTarget::Clip(id)).unwrap();
        let (g, geo, re) = graph_of(&p);
        assert!(re, "Scale at 100% is a re-encode");
        assert_eq!((geo.width, geo.height), (1280, 720));
        // only the first clip has the frame path
        assert!(g.graph.contains("overlay="));
        assert_eq!(g.graph.matches("overlay=").count(), 1);
        let mut q = project(&[(2, 1280, 720, 30)]);
        q.timeline.add_move(FrameTarget::Whole).unwrap();
        assert!(graph_of(&q).2);
        let mut r = project(&[(2, 1280, 720, 30)]);
        r.timeline
            .set_frame_aspect(Some(FrameAspect::default()))
            .unwrap();
        assert!(graph_of(&r).2);
    }

    // covers: REQ-125
    // covers: REQ-126
    // covers: REQ-131
    #[test]
    fn the_frame_graph_has_background_picture_overlay_in_order() {
        let mut p = project(&[(4, 1600, 1200, 30)]);
        p.timeline
            .set_frame_aspect(Some(tall(Fit::Width, Fill::Colour(Rgb(0, 255, 0)))))
            .unwrap();
        let (g, geo, _) = graph_of(&p);
        assert_eq!((geo.width, geo.height), (1600, 2844));
        let want = "[0:v:0]trim=start=0.000000:duration=4.000000,\
                    setpts=PTS-STARTPTS+0.000000/TB,scale=1600:-2,setsar=1,fps=30/1[pa0];\
                    color=c=0x00FF00:s=1600x2844:r=30/1:d=4.000000,\
                    setpts=PTS-STARTPTS+0.000000/TB[b0];\
                    [b0][pa0]overlay=x='(W-w)/2':y='(H-h)/2':eval=frame:shortest=1,\
                    format=yuv420p,setpts=PTS-STARTPTS[v0]";
        let video: Vec<&str> = g.graph.split(';').take(3).collect();
        assert_eq!(video.join(";"), want);
        assert!(g.graph.contains("[b0][pa0]overlay=x='(W-w)/2':y='(H-h)/2'"));
        // blur: a small blurred copy is the background
        let mut q = project(&[(4, 1600, 1200, 30)]);
        q.timeline
            .set_frame_aspect(Some(tall(Fit::Height, Fill::Blur)))
            .unwrap();
        let (g, geo, _) = graph_of(&q);
        assert_eq!((geo.width, geo.height), (676, 1200));
        assert!(
            g.graph
                .contains("scale=-2:1200,setsar=1,fps=30/1,split=2[pa0][pb0]")
        );
        assert!(g.graph.contains("[pb0]scale=84:150:force_original_aspect_ratio=increase,crop=84:150,boxblur=3:2,scale=676:1200,setsar=1[b0]"));
        assert!(!g.graph.contains("color=c="));
    }

    // covers: REQ-127
    // covers: REQ-128
    // covers: REQ-131
    #[test]
    fn scale_and_move_expressions_follow_the_keys_in_clip_time() {
        let mut p = project(&[(4, 640, 360, 30), (4, 640, 360, 30)]);
        let second = p.timeline.clips[1].id;
        let c = FrameTarget::Clip(second);
        p.timeline.add_scale(c).unwrap();
        p.timeline
            .add_scale_key(
                c,
                Micros(6_000_000),
                Scale {
                    permille: 2000,
                    dx: 0,
                    dy: 0,
                },
            )
            .unwrap();
        p.timeline.add_move(FrameTarget::Whole).unwrap();
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        // clip 1 starts at timeline 0: the whole video's Move key at 0 s
        assert_eq!(segs[0].frame.as_ref().unwrap().stack.scale.len(), 0);
        let (g, _, _) = graph_of(&p);
        // clip 2: scale keys at source 0 and 6 s, the clip starts at source 0: local 0..6
        assert!(
            g.graph
                .contains("scale=w='max(2,trunc(iw*(if(lt(t,6),(1+(1)*max(0,t-0)/6),2))/2)*2)'"),
            "{}",
            g.graph
        );
        // the picture's centre: the whole video's Move at the centre adds nothing
        assert!(g.graph.contains("overlay=x='(W-w)/2'"));
    }

    // covers: REQ-131
    #[test]
    fn a_360_export_ignores_the_frame_effects_and_the_flat_view_applies_them() {
        let mut p = sphere_project();
        let plain = {
            let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true);
            // clip 2 is flat: a mix is refused, so use two 360 clips
            assert!(plan.is_err());
            let mut q = project(&[(2, 3840, 1920, 30), (2, 3840, 1920, 30)]);
            q.media[0].format.spherical = Some(true);
            q.media[1].format.spherical = Some(true);
            q.texts = Default::default();
            q
        };
        let with = |q: &Project| {
            let mut q = q.clone();
            // a text keeps the plan a re-encode
            let id = q.texts.new_id();
            let mut t = crate::text::TextItem::new(id, "N".into(), Micros::ZERO);
            t.ttl = Micros(4_000_000);
            q.texts.add(t).unwrap();
            q
        };
        let base = with(&plain);
        let mut effects = base.clone();
        effects
            .timeline
            .set_frame_aspect(Some(tall(Fit::Width, Fill::Blur)))
            .unwrap();
        effects.timeline.add_scale(FrameTarget::Whole).unwrap();
        let a = plan_export_mode(&base, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
        let b = plan_export_mode(&effects, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
        assert_eq!(a, b);
        // the flat camera view of the same project has the frame
        let c = plan_export_mode(&effects, &enc(), ExportMode::FlatCamera, |_, _| true).unwrap();
        let Plan::Reencode {
            graph, settings, ..
        } = c
        else {
            panic!("re-encode")
        };
        assert_eq!((settings.width, settings.height), (1920, 3414));
        assert!(graph.graph.contains("v360") && graph.graph.contains("overlay="));
        p.timeline.clips.clear();
    }

    // covers: REQ-131
    #[test]
    fn subtitles_are_drawn_after_the_composition() {
        let mut p = project(&[(4, 640, 360, 30)]);
        p.timeline.add_scale(FrameTarget::Whole).unwrap();
        let g = build_graph(
            &segments(&p, Micros::ZERO, Source::Export).unwrap(),
            None,
            &GraphSpec {
                subtitles: true,
                ..spec(true, false)
            },
        );
        let at = |n: &str| g.graph.find(n).unwrap();
        assert!(at("overlay=") < at("concat=") && at("concat=") < at("subtitles=f=subs.ass"));
    }

    // covers: REQ-131
    #[test]
    fn a_preview_from_the_middle_of_a_clip_shifts_the_background_with_the_picture() {
        let mut p = project(&[(4, 640, 360, 30)]);
        p.timeline.add_move(FrameTarget::Whole).unwrap();
        let segs = segments(&p, s(1), Source::Export).unwrap();
        let g = build_graph(&segs, None, &spec(true, false));
        assert!(g.graph.contains("setpts=PTS-STARTPTS+1.000000/TB"));
        assert_eq!(
            g.graph.matches("setpts=PTS-STARTPTS+1.000000/TB").count(),
            2
        );
    }

    // covers: REQ-034
    // covers: REQ-131
    #[test]
    fn the_frame_key_changes_with_the_picture_state_only() {
        let mut p = project(&[(4, 640, 360, 30)]);
        let eg = Geometry {
            width: 640,
            height: 360,
            fps: Fps::new(30, 1).unwrap(),
        };
        let key = |p: &Project, at: i64| frame_key(p, s(at), Source::Export, &eg).unwrap();
        assert!(key(&p, 1).placement.is_none());
        p.timeline.add_scale(FrameTarget::Whole).unwrap();
        p.timeline
            .add_scale_key(
                FrameTarget::Whole,
                s(4),
                Scale {
                    permille: 2000,
                    dx: 0,
                    dy: 0,
                },
            )
            .unwrap();
        let k1 = key(&p, 1);
        assert_ne!(k1, key(&p, 2));
        // a still position in the same clip with the same state has the same key
        p.timeline
            .remove_scale_key(FrameTarget::Whole, s(4))
            .unwrap();
        assert_eq!(key(&p, 1).placement, key(&p, 2).placement);
        assert_ne!(key(&p, 1), k1);
    }

    // covers: REQ-125
    #[test]
    fn the_preview_has_the_frame_shape() {
        let mut p = project(&[(4, 1600, 1200, 30)]);
        p.timeline
            .set_frame_aspect(Some(tall(Fit::Width, Fill::Blur)))
            .unwrap();
        let g = output_geometry(&p, &enc()).unwrap();
        assert_eq!((g.width, g.height), (1600, 2844));
        assert_eq!(g.preview().height, 360);
        let sc = Path::new("/tmp/g.txt");
        let r = still_args(&p, Micros::ZERO, &g, FfVersion::Dev, sc, Source::Export).unwrap();
        let script = strs(&r.args).join(" ");
        assert!(script.contains("boxblur") || !r.args.is_empty());
    }

    fn lut_file(dir: &Path, name: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, crate::lut::tests::identity(2)).unwrap();
        p.to_string_lossy().into_owned()
    }

    fn lut_graph(p: &Project, src: Source) -> Graph {
        let segs = segments(p, Micros::ZERO, src).unwrap();
        build_graph(&segs, None, &spec(true, false))
    }

    // covers: REQ-135
    // covers: REQ-136
    #[test]
    fn each_clip_has_one_lut_stage_and_never_uses_a_user_name() {
        use crate::timeline::FrameTarget;
        let dir = tempfile::tempdir().unwrap();
        let mut p = project(&[(2, 1280, 720, 30), (2, 1280, 720, 30)]);
        let plain = lut_graph(&p, Source::Export);
        assert!(!plain.graph.contains("lut3d") && plain.command_files.is_empty());
        let a = lut_file(dir.path(), "my warm look; rm.cube");
        let id = p.timeline.clips[1].id;
        // the whole-video LUT is a copy on each clip (REQ-147)
        p.timeline
            .set_lut(FrameTarget::Whole, Some(a.clone()))
            .unwrap();
        let g = lut_graph(&p, Source::Export);
        // before the pad (black bars keep their colour), then back to yuv
        assert!(
            g.graph.contains(
                "decrease,format=gbrp,lut3d=file=lut-clip-0.cube:interp=tetrahedral,\
             format=yuv420p,pad="
            ),
            "{}",
            g.graph
        );
        assert!(g.graph.contains("[0:v:0]") && !g.graph.contains("warm"));
        assert_eq!(g.graph.matches("lut-clip-0.cube").count(), 1);
        assert_eq!(g.graph.matches("lut-clip-1.cube").count(), 1);
        let names: Vec<&str> = g.command_files.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, ["lut-clip-0.cube", "lut-clip-1.cube"]);
        assert!(g.command_files[0].1.starts_with("LUT_3D_SIZE 2\n"));
        // one clip alone
        p.timeline.set_lut(FrameTarget::Whole, None).unwrap();
        p.timeline.set_lut(FrameTarget::Clip(id), Some(a)).unwrap();
        let g = lut_graph(&p, Source::Export);
        assert!(!g.graph.contains("lut-clip-0.cube") && g.graph.contains("lut-clip-1.cube"));
        // a frame effect: the LUT acts on the fitted picture before the overlay
        let b = lut_file(dir.path(), "b.cube");
        p.timeline.set_lut(FrameTarget::Whole, Some(b)).unwrap();
        p.timeline
            .set_frame_aspect(Some(tall(Fit::Width, Fill::Blur)))
            .unwrap();
        let g = lut_graph(&p, Source::Export);
        let lut = g.graph.find("lut3d").unwrap();
        assert!(lut < g.graph.find("overlay").unwrap());
    }

    // covers: REQ-136
    #[test]
    fn a_lut_forces_the_reencode_and_the_export_checks_the_file_first() {
        use crate::timeline::FrameTarget;
        let dir = tempfile::tempdir().unwrap();
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::StreamCopy { .. }
        ));
        let path = lut_file(dir.path(), "a.cube");
        p.timeline
            .set_lut(FrameTarget::Whole, Some(path.clone()))
            .unwrap();
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true).unwrap(),
            Plan::Reencode { .. }
        ));
        // a missing file: the export stops with a message that names it; the preview
        // ignores the LUT
        std::fs::remove_file(&path).unwrap();
        let e = plan_export(&p, &enc(), |_, _| true).unwrap_err();
        assert!(matches!(e, RenderError::Lut(ref n, _) if n == "a.cube"));
        assert!(e.to_string().contains("a.cube"));
        let g = lut_graph(&p, Source::Preview { proxies: false });
        assert!(!g.graph.contains("lut3d"));
        // a broken file is refused at the export too
        std::fs::write(&path, "nonsense").unwrap();
        assert!(matches!(
            plan_export(&p, &enc(), |_, _| true),
            Err(RenderError::Lut(..))
        ));
    }

    // covers: REQ-135
    #[test]
    fn the_lut_goes_before_texts_fixed_to_the_sphere_and_after_the_camera_otherwise() {
        use crate::timeline::FrameTarget;
        let dir = tempfile::tempdir().unwrap();
        let mut p = sphere_project();
        let path = lut_file(dir.path(), "s.cube");
        p.timeline.set_lut(FrameTarget::Whole, Some(path)).unwrap();
        let g = lut_graph(&p, Source::Export);
        assert!(
            g.graph.find("v360").unwrap() < g.graph.find("lut3d").unwrap(),
            "{}",
            g.graph
        );
        // texts fixed to the sphere: drawn after the table, so they keep their colours
        let segs = {
            let mut v = segments(&p, Micros::ZERO, Source::Export).unwrap();
            v[0].anchored = Some("[Script Info]".into());
            v
        };
        let g = build_graph(&segs, None, &spec(true, false));
        assert!(
            g.graph.find("lut3d").unwrap() < g.graph.find("subtitles=f=anc0.ass").unwrap(),
            "{}",
            g.graph
        );
    }

    // covers: REQ-135
    #[test]
    fn a_changed_lut_changes_the_frame_key_once() {
        use crate::timeline::FrameTarget;
        let dir = tempfile::tempdir().unwrap();
        let mut p = project(&[(4, 1280, 720, 30)]);
        let g = output_geometry(&p, &enc()).unwrap();
        let key = |p: &Project| frame_key(p, s(1), Source::Export, &g).unwrap();
        let none = key(&p);
        assert_eq!(none.luts, None);
        let path = lut_file(dir.path(), "k.cube");
        p.timeline
            .set_lut(FrameTarget::Whole, Some(path.clone()))
            .unwrap();
        let with = key(&p);
        assert_ne!(none, with);
        assert!(with.luts.is_some());
        assert_eq!(with, key(&p));
        std::fs::write(&path, crate::lut::tests::identity(3)).unwrap();
        assert_ne!(with, key(&p));
    }
}
