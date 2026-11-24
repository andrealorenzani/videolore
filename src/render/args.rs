// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Argument lists: export, preview, still and check renders, and the frame cache key.

use super::*;

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
            graph,
            settings: s,
            track,
            ..
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
            // the separate subtitle track is one more input, read from the working folder
            if let Some(t) = track {
                a.extend(argv(["-i", t.file]));
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
            if let Some(t) = track {
                // not switched on by default (`-disposition:s:0 0`), named with the language
                a.extend(argv([
                    "-map".to_string(),
                    format!("{}:0", graph.inputs.len()),
                    "-c:s".to_string(),
                    t.codec.to_string(),
                    "-metadata:s:s:0".to_string(),
                    format!("language={}", t.language),
                    "-disposition:s:0".to_string(),
                    "0".to_string(),
                ]));
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

/// The geometry of the export before a Change aspect frame is applied.
pub fn base_geometry(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
) -> Result<Geometry, RenderError> {
    let media = timeline_media(project)?;
    let owned: Vec<MediaFormat> = media.iter().map(|m| effective_format(m)).collect();
    let formats: Vec<&MediaFormat> = owned.iter().collect();
    let chosen = format::choose_output(&formats).ok_or(RenderError::Empty)?;
    Ok(Geometry::of(&format::encode_settings(&chosen, encoders)))
}

/// The geometry the export would use (and the preview scales down from).
pub fn output_geometry(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
) -> Result<Geometry, RenderError> {
    let mut g = base_geometry(project, encoders)?;
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
    /// LUT tables for the working folder.
    pub lut_files: Vec<std::sync::Arc<crate::lut::Lut>>,
}

pub(super) fn video_render(
    project: &Project,
    segs: &[Segment],
    from: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
) -> Result<PreviewRender, String> {
    video_render_bg(
        project,
        segs,
        from,
        export,
        version,
        script,
        Background::Normal,
        None,
    )
}

/// [`video_render`] with the way the blurred background is made; `raw` is the file of the
/// cached background for [`Background::Cached`].
#[allow(clippy::too_many_arguments)]
fn video_render_bg(
    project: &Project,
    segs: &[Segment],
    from: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
    bg: Background,
    raw: Option<&Path>,
) -> Result<PreviewRender, String> {
    let g = export.preview();
    let src = Source::Preview { proxies: false };
    // a clip whose effects were put in another order is drawn by the overlay stage; the
    // background alone has none of it
    let mut segs = segs.to_vec();
    overlay::attach(
        project,
        &mut segs,
        &g,
        from,
        src,
        true,
        bg != Background::Only,
    )
    .map_err(|e| e.to_string())?;
    let subtitles = subtitles_doc(project, export, from);
    // the background alone has no pictures on it either
    let pics = if bg == Background::Only {
        Vec::new()
    } else {
        picture_overlays(project, &g, from, Source::Preview { proxies: false })
            .map_err(|e| e.to_string())?
    };
    let graph = build_graph_pics(
        &segs,
        None,
        &GraphSpec {
            video: true,
            audio: false,
            pix_fmt: "rgba",
            geometry: g,
            // the background alone has no texts or subtitles on it
            subtitles: subtitles.is_some() && bg != Background::Only,
            mix: project.mix,
            camera_updates: 0,
        },
        bg,
        &pics,
    );
    let mut a = head();
    a.extend(input_args(&graph.inputs));
    if let (Background::Cached(_), Some(raw)) = (bg, raw) {
        // one picture of the frame size, as it came out of the Only run
        a.extend(argv([
            "-protocol_whitelist".to_string(),
            "file".to_string(),
            "-f".to_string(),
            "rawvideo".to_string(),
            "-pixel_format".to_string(),
            "rgba".to_string(),
            "-video_size".to_string(),
            format!("{}x{}", g.width, g.height),
            "-framerate".to_string(),
            g.fps.ff(),
            "-i".to_string(),
        ]));
        a.push(file_arg(raw));
    }
    a.extend(filter_graph_args(&graph.graph, version, script).map_err(|e| e.to_string())?);
    a.extend(argv(["-map", "[outv]"]));
    Ok(PreviewRender {
        args: a,
        subtitles,
        command_files: graph.command_files,
        lut_files: graph.lut_files,
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
    still_args_bg(
        project,
        at,
        export,
        version,
        script,
        src,
        Background::Normal,
        None,
    )
}

/// [`still_args`] for the preview's cache of the blurred background (REQ-223):
/// [`Background::Only`] makes the background alone; [`Background::Cached`] reads it from
/// `raw` and draws the picture.
#[allow(clippy::too_many_arguments)]
pub fn still_args_bg(
    project: &Project,
    at: Micros,
    export: &Geometry,
    version: FfVersion,
    script: &Path,
    src: Source,
    bg: Background,
    raw: Option<&Path>,
) -> Result<PreviewRender, String> {
    let mut segs = segments(project, at, src).map_err(|e| e.to_string())?;
    segs.truncate(1);
    let mut r = video_render_bg(project, &segs, at, export, version, script, bg, raw)?;
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

/// What the output check renders of the normal method: the video graph of `project` at
/// `geometry` from the timeline position `from` for about `len` (REQ-206), without audio
/// or music, plus the burned-in subtitles it reads from its working folder.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckGraph {
    /// Output `[outv]`.
    pub graph: Graph,
    /// The ASS document the graph reads as `subs.ass` from the working folder.
    pub subtitles: Option<String>,
}

/// The normal method's whole video graph of the export of `project` at `geometry`, without
/// audio or music (the output check renders it next to a hardware file). Output `[outv]`;
/// burned-in subtitles read `subs.ass` from the working folder, as in the export.
pub fn check_graph(project: &Project, geometry: &Geometry) -> Result<Graph, RenderError> {
    let mut segs = segments(project, Micros::ZERO, Source::Export)?;
    overlay::attach(
        project,
        &mut segs,
        geometry,
        Micros::ZERO,
        Source::Export,
        true,
        true,
    )?;
    let subtitles = subtitles_doc(project, geometry, Micros::ZERO).is_some();
    let pics = picture_overlays(project, geometry, Micros::ZERO, Source::Export)?;
    Ok(build_graph_with(
        &segs,
        None,
        &GraphSpec {
            video: true,
            audio: false,
            pix_fmt: "yuv420p",
            geometry: *geometry,
            subtitles,
            mix: project.mix,
            camera_updates: crate::camera::EXPORT_UPDATES_PER_SECOND,
        },
        &pics,
    ))
}

/// The normal method's video graph for the short output check (see [`CheckGraph`]): only the
/// clips that begin before `from + len` are read, so the cost is that of the range and not
/// of the video. `burn_lines` false: the subtitle lines go to a separate track and are not
/// in the hardware file either.
pub fn check_range_graph(
    project: &Project,
    geometry: &Geometry,
    from: Micros,
    len: Micros,
    burn_lines: bool,
) -> Result<CheckGraph, RenderError> {
    let mut segs = segments(project, from, Source::Export)?;
    let mut shown = Micros::ZERO;
    let keep = segs
        .iter()
        .position(|s| {
            let over = shown >= len;
            shown = shown + s.rendered();
            over
        })
        .unwrap_or(segs.len());
    segs.truncate(keep.max(1));
    overlay::attach(
        project,
        &mut segs,
        geometry,
        from,
        Source::Export,
        burn_lines,
        true,
    )?;
    let subtitles = subtitles_doc_with(project, geometry, from, burn_lines);
    let pics = picture_overlays(project, geometry, from, Source::Export)?;
    let graph = build_graph_with(
        &segs,
        None,
        &GraphSpec {
            video: true,
            audio: false,
            pix_fmt: "yuv420p",
            geometry: *geometry,
            subtitles: subtitles.is_some(),
            mix: project.mix,
            camera_updates: crate::camera::EXPORT_UPDATES_PER_SECOND,
        },
        &pics,
    );
    Ok(CheckGraph { graph, subtitles })
}

/// How many frames the normal method's video graph makes for `project` at `fps`: every
/// clip gives its length in whole frames (the graph's `fps` filter rounds each clip).
pub fn expected_frames(project: &Project, fps: Fps) -> u64 {
    project
        .timeline
        .clips
        .iter()
        .map(|c| {
            let n = c.len().0.max(0) as i128 * fps.num as i128;
            let d = 1_000_000i128 * fps.den as i128;
            ((n + d / 2) / d) as u64
        })
        .sum()
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
            camera_updates: 0,
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

/// The ambisonic sound a preview from `from` turns by the camera first (REQ-273): the
/// pre-pass of each clip with such sound, before the audio process starts.
pub fn preview_spatial(
    project: &Project,
    from: Micros,
    src: Source,
) -> Result<Vec<crate::spatial::SpatialJob>, String> {
    let segs = segments(project, from, src).map_err(|e| e.to_string())?;
    Ok(segs
        .into_iter()
        .filter(|s| s.has_audio)
        .filter_map(|s| s.spatial)
        .collect())
}

/// A subtitle on screen as part of a [`FrameKey`]: text, highlighted word, own position
/// and own size.
pub type ShownSubtitle = (
    String,
    Option<usize>,
    Option<crate::base::pos::Pos>,
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
    pub line_anchors: Vec<Option<crate::base::pos::Anchor360>>,
    /// Change aspect and the picture's scale and place at this position, in millionths
    /// (1.5.0); `None` with no frame effect.
    pub placement: Option<FramePlacement>,
    /// The Change aspect is the clip's own (REQ-286): its frame is smaller than the output,
    /// so the kept background cannot be used (REQ-335).
    pub own_aspect: bool,
    /// Fingerprint of the clip's Apply LUT table (1.6.0): a changed file re-renders the
    /// picture once.
    pub luts: Option<String>,
    /// The order of the clip's movable effects when the user changed it (REQ-304), top first.
    pub order: Vec<crate::effect_order::Row>,
    /// The overlaps of the clip on screen, with the frame index since their start for an
    /// animation (REQ-325).
    pub overlaps: Vec<(crate::overlap::Overlap, i64)>,
    /// The audio track the clip plays (REQ-340): a change restarts what is shown and heard.
    pub audio_track: u8,
}

impl FrameKey {
    /// The name of the file that holds the blurred background of this picture, when the
    /// background can be kept (REQ-223): a Change aspect frame with a blurred fill, on a flat
    /// clip, outside a fade. The name tells what the background depends on and nothing else:
    /// the source frame, the size of the frame, the shape and fit, the Apply LUT. Scale, Move,
    /// texts and subtitles change the picture only.
    pub fn background_name(&self) -> Option<String> {
        use sha2::{Digest, Sha256};
        let (Some(aspect), _) = self.placement.as_ref()? else {
            return None;
        };
        if !matches!(aspect.fill, frame::Fill::Blur)
            || self.camera.is_some()
            || self.fade.is_some()
            || self.own_aspect
        {
            return None;
        }
        // a clip built step by step has its background made from its own steps (REQ-320)
        if crate::effect_order::needs_steps(&self.order) {
            return None;
        }
        let what = format!(
            "{:?}|{}|{:?}|{:?}|{:?}",
            self.path, self.frame, self.size, aspect, self.luts
        );
        let digest = Sha256::digest(what.as_bytes());
        let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        Some(format!("bg-{hex}.rgba"))
    }
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
            let w = preset.follows_words().then(|| l.word_at(at)).flatten();
            (l.text, w, l.position, l.size)
        })
        .collect();
    let g = export.preview();
    let camera = m.format.is_360().then(|| {
        camera::at(project.camera.path_of(Some(c.id.0), m.id), c.src_at(offset))
            .unwrap_or(Camera::FRONT)
    });
    let stack = tl.frame_stack(i);
    let aspect = tl.own_aspect(c.id).or(tl.frame_aspect);
    let placement = (aspect.is_some() || !stack.is_empty()).then(|| {
        let p = stack.at(offset);
        let q = |v: f64| (v * 1_000_000.0).round() as i64;
        (aspect, (q(p.factor), q(p.ox), q(p.oy)))
    });
    let hash = |t| {
        lut_for(tl, t, src)
            .ok()
            .flatten()
            .map(|l| l.hash().to_string())
    };
    let luts = hash(crate::timeline::FrameTarget::Clip(c.id));
    let order = project
        .changed_orders()
        .into_iter()
        .find(|(r, _)| r.clip == c.id)
        .map(|(_, o)| o)
        .unwrap_or_default();
    let overlaps = tl
        .overlaps_of(c.id)
        .iter()
        .filter(|o| offset >= o.from && offset < o.end())
        .map(|o| {
            let n = if o.kind == crate::overlap::Kind::Motion {
                export.fps.frame_index(offset - o.from)
            } else {
                0
            };
            (o.clone(), n)
        })
        .collect();
    Some(FrameKey {
        own_aspect: tl.own_aspect(c.id).is_some(),
        audio_track: c.audio_track,
        luts,
        order,
        overlaps,
        placement,
        path: src.path_of(m),
        frame: fps.frame_index(c.src_at(offset)),
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
