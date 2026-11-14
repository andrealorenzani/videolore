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
use crate::ffmpeg::{FfVersion, argv, file_arg, filter_graph_args};
use crate::format::{self, EncodeSettings};
use crate::media::{MediaId, MediaItem};
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

const AFORMAT: &str = "aformat=sample_fmts=fltp:channel_layouts=stereo";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RenderError {
    #[error("the timeline is empty")]
    Empty,
    #[error("source '{0}' is missing")]
    Missing(String),
    #[error("a clip refers to an unknown source")]
    UnknownMedia,
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
            let (fade_in, fade_out) = tl.fades_of(i);
            out.push(Segment {
                media: m.id,
                path: src.path_of(m),
                has_audio: m.format.has_audio(),
                src_in: c.src_in,
                len: c.len(),
                skip: (from - start).max(Micros::ZERO),
                fade_in,
                fade_out,
            });
        }
        start = end;
    }
    if out.is_empty() {
        return Err(RenderError::Empty);
    }
    Ok(out)
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

fn fades(c: &mut String, filter: &str, s: &Segment) {
    if s.fade_in > Micros::ZERO {
        c.push_str(&format!(",{filter}=t=in:st=0:d={}", s.fade_in.ff()));
    }
    if s.fade_out > Micros::ZERO {
        c.push_str(&format!(
            ",{filter}=t=out:st={}:d={}",
            (s.len - s.fade_out).ff(),
            s.fade_out.ff()
        ));
    }
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
    let mut concat_in = String::new();
    for (i, s) in segs.iter().enumerate() {
        let k = idx[i];
        let off = (s.src_in + s.skip - inputs[k].start).ff();
        let d = s.rendered().ff();
        if spec.video {
            let src = &vlabels[k][vused[k]];
            vused[k] += 1;
            let (w, h) = (g.width, g.height);
            let mut c = format!(
                "{src}trim=start={off}:duration={d},setpts=PTS-STARTPTS+{o}/TB,\
                 scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={f},format={pf}",
                o = s.skip.ff(),
                f = g.fps.ff(),
                pf = spec.pix_fmt,
            );
            fades(&mut c, "fade", s);
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
            fades(&mut c, "afade", s);
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
        graph: Graph,
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
    let lines = project.subtitles.project(&project.timeline);
    if !lines.iter().any(|l| l.end > from) {
        return None;
    }
    Some(ass::document(
        &lines,
        project.subtitles.preset,
        g.width,
        g.height,
        from,
    ))
}

/// Decides between stream copy and re-encode. `is_keyframe(media, t)` is only consulted
/// when the formats are identical and there are no effects (fades, music, subtitles,
/// video volume change). Always reads the original files (never proxies).
pub fn plan_export(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
    mut is_keyframe: impl FnMut(&MediaItem, Micros) -> bool,
) -> Result<Plan, RenderError> {
    let segs = segments(project, Micros::ZERO, Source::Export)?;
    let media = timeline_media(project)?;
    let formats: Vec<_> = media.iter().map(|m| &m.format).collect();
    let chosen = format::choose_output(&formats).ok_or(RenderError::Empty)?;
    let settings = format::encode_settings(&chosen, encoders);
    let geometry = Geometry::of(&settings);
    let subtitles = subtitles_doc(project, &geometry, Micros::ZERO);
    let bed = music_bed(project, Micros::ZERO)?;
    let no_effects = project.timeline.fades.is_empty()
        && bed.is_none()
        && subtitles.is_none()
        && project.mix.video_neutral();
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
        graph,
        settings,
        subtitles,
    })
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
pub fn export_args(
    plan: &Plan,
    paths: &ExportPaths<'_>,
    version: FfVersion,
    threads: Option<u32>,
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
            match threads {
                Some(t) => {
                    a.extend(argv(["-filter_complex_threads".to_string(), t.to_string()]));
                    a.extend(threaded_inputs(input_args(&graph.inputs), t));
                }
                None => a.extend(input_args(&graph.inputs)),
            }
            a.extend(filter_graph_args(
                &graph.graph,
                version,
                paths.graph_script,
            )?);
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
            if s.vencoder == "libx265" && matches!(s.muxer.as_str(), "mp4" | "mov") {
                a.extend(argv(["-tag:v", "hvc1"]));
            }
            a.extend(argv(["-pix_fmt", "yuv420p", "-c:a", &s.aencoder]));
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
    let formats: Vec<_> = media.iter().map(|m| &m.format).collect();
    let chosen = format::choose_output(&formats).ok_or(RenderError::Empty)?;
    Ok(Geometry::of(&format::encode_settings(&chosen, encoders)))
}

/// A preview render: the argv and the ASS document to put in the working folder.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewRender {
    pub args: Vec<OsString>,
    pub subtitles: Option<String>,
}

fn video_render(
    project: &Project,
    segs: &[Segment],
    from: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
) -> Result<(Vec<OsString>, Option<String>), String> {
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
    Ok((a, subtitles))
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
    let (mut args, subtitles) = video_render(project, &segs, from, export, version, script)?;
    args.extend(argv(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"]));
    Ok(PreviewRender { args, subtitles })
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
    let (mut args, subtitles) = video_render(project, &segs, at, export, version, script)?;
    args.extend(argv([
        "-frames:v",
        "1",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "pipe:1",
    ]));
    Ok(PreviewRender { args, subtitles })
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
    /// Subtitles on screen and the highlighted word.
    pub subtitles: Vec<(String, Option<usize>)>,
    pub preset: Preset,
}

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
            (l.text, w)
        })
        .collect();
    let g = export.preview();
    Some(FrameKey {
        path: src.path_of(m),
        frame: fps.frame_index(c.src_in + offset),
        fade: in_fade.then_some((fin, fout, offset, c.len())),
        size: (g.width, g.height),
        subtitles,
        preset,
    })
}

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
        strs(&export_args(plan, &paths, FfVersion::Release(6, 1, 0), threads).unwrap())
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
        assert_eq!(ks.subtitles, vec![("a b".to_string(), None)]);
        p.subtitles.preset = Preset::WordHighlight;
        let kw = frame_key(&p, Micros(500_000), src, &eg).unwrap();
        assert_eq!(kw.subtitles, vec![("a b".to_string(), Some(0))]);
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
}
