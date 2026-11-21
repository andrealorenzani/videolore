// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The export plan (stream copy or one re-encode pass), the concat list and the audio-only export.

use super::*;

/// Common leading options.
pub(super) fn head() -> Vec<OsString> {
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
    /// One re-encode pass. `subtitles` is the ASS document to burn in, if any; `track` is
    /// the subtitle stream added next to the picture (REQ-182).
    Reencode {
        graph: Box<Graph>,
        settings: EncodeSettings,
        subtitles: Option<String>,
        track: Option<Box<SubtitleTrack>>,
    },
}

/// The subtitle lines as a separate, switchable stream (REQ-182): the file ffmpeg reads
/// (SRT, or WebVTT for WebM) and the language it is named with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub file: &'static str,
    pub content: String,
    /// ISO 639-2 code of the project language (`und` when it is not known).
    pub language: String,
    /// The stream codec for the container (`mov_text`, `srt` or `webvtt`).
    pub codec: &'static str,
}

/// File name of the separate track in the working folder.
pub const TRACK_SRT: &str = "subs-track.srt";
pub const TRACK_VTT: &str = "subs-track.vtt";

impl SubtitleTrack {
    /// The track for `lines` in a container written by `muxer`; `None` for a container
    /// that has no text-subtitle stream here (and when there are no lines).
    pub fn new(
        lines: &[crate::subtitles::ShownLine],
        language: Option<&str>,
        muxer: &str,
    ) -> Option<SubtitleTrack> {
        let (file, content, codec) = match muxer {
            "mp4" | "mov" => (TRACK_SRT, crate::srt::write(lines), "mov_text"),
            "matroska" => (TRACK_SRT, crate::srt::write(lines), "srt"),
            "webm" => (TRACK_VTT, crate::srt::write_vtt(lines), "webvtt"),
            _ => return None,
        };
        if lines.iter().all(|l| l.text.trim().is_empty()) {
            return None;
        }
        let language = language
            .and_then(crate::base::language::iso639_2)
            .unwrap_or("und")
            .to_string();
        Some(SubtitleTrack {
            file,
            content,
            language,
            codec,
        })
    }
}

impl Plan {
    /// The ASS document the export needs in its working folder.
    pub fn subtitles(&self) -> Option<&str> {
        match self {
            Plan::Reencode { subtitles, .. } => subtitles.as_deref(),
            Plan::StreamCopy { .. } => None,
        }
    }

    /// The separate subtitle track of the export, if any.
    pub fn track(&self) -> Option<&SubtitleTrack> {
        match self {
            Plan::Reencode { track, .. } => track.as_deref(),
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

    /// The ambisonic sound the export turns by the camera first (REQ-273).
    pub fn spatial(&self) -> &[crate::spatial::SpatialJob] {
        match self {
            Plan::Reencode { graph, .. } => &graph.spatial,
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
            seg.src_to().ff()
        ));
    }
    s
}

pub(super) fn hevc_in_mp4(codec: Option<&str>, muxer: &str) -> bool {
    codec == Some("hevc") && matches!(muxer, "mp4" | "mov")
}

/// The ASS document for the subtitles visible from `from` (export geometry), if any.
pub fn subtitles_doc(project: &Project, g: &Geometry, from: Micros) -> Option<String> {
    subtitles_doc_with(project, g, from, true)
}

/// [`subtitles_doc`]; with `burn_lines` false the lines are left out (they go to a separate
/// track, REQ-182) except those fixed to the sphere of a 360 clip, which cannot be a track.
pub fn subtitles_doc_with(
    project: &Project,
    g: &Geometry,
    from: Micros,
    burn_lines: bool,
) -> Option<String> {
    let mut lines = project.subtitles.project(&project.timeline);
    // a line fixed to the sphere on a 360 clip is burned on the equirectangular picture
    lines.retain(|l| !(l.anchor.is_some() && line_on_sphere(project, l)));
    if !burn_lines {
        lines.clear();
    }
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

/// The picture items (REQ-287) shown from the timeline position `from` on, placed on a
/// picture of `g`, with their times counted from `from`. A file that is gone stops an export
/// and is left out of a preview.
pub fn picture_overlays(
    project: &Project,
    g: &Geometry,
    from: Micros,
    src: Source,
) -> Result<Vec<PicOverlay>, RenderError> {
    let video_end = project.timeline.duration();
    let mut out = Vec::new();
    for t in &project.texts.items {
        let Some(pic) = &t.picture else { continue };
        let end = t.end().min(video_end);
        if end <= from.max(t.start) {
            continue;
        }
        let path = PathBuf::from(&pic.path);
        if !path.is_file() {
            if matches!(src, Source::Preview { .. }) {
                continue;
            }
            return Err(RenderError::Picture(pic.name()));
        }
        let (x, y, w, h) = pic.rect(t.place, t.width, t.height, (g.width, g.height));
        out.push(PicOverlay {
            path,
            x,
            y,
            w,
            h,
            start: (t.start - from).max(Micros::ZERO),
            end: end - from,
        });
    }
    Ok(out)
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
            // a 360 video keeps its sound as recorded: the viewer's player turns it (REQ-273)
            s.spatial = None;
            s.frame = None;
            s.crop = Default::default();
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
    let all_lines = project.subtitles.project(&project.timeline);
    let track = (choice.subtitles == format::SubtitleMode::Separate)
        .then(|| {
            let track_lines: Vec<_> = all_lines
                .iter()
                .filter(|l| !(l.anchor.is_some() && line_on_sphere(project, l)))
                .cloned()
                .collect();
            SubtitleTrack::new(
                &track_lines,
                project.subtitles.language.as_deref(),
                &settings.muxer,
            )
        })
        .flatten()
        .map(Box::new);
    let subtitles = subtitles_doc_with(project, &geometry, Micros::ZERO, track.is_none());
    let bed = music_bed(project, Micros::ZERO)?;
    // a 360 video keeps the whole sphere: a flat picture does not belong on it
    let pics = if sphere {
        Vec::new()
    } else {
        picture_overlays(project, &geometry, Micros::ZERO, Source::Export)?
    };
    let no_effects = !project.timeline.has_any_fade()
        && (sphere || !project.timeline.has_frame_effect())
        && preset.is_none()
        && choice.audio == format::AudioChoice::Same
        && !project.timeline.any_clean_audio()
        && (sphere || !project.timeline.any_crop())
        && !project.timeline.any_lut()
        && project.timeline.clips.iter().all(|c| !c.has_volume())
        && !project.timeline.any_volume_keys()
        && !project.timeline.any_speed()
        && pics.is_empty()
        && bed.is_none()
        && subtitles.is_none()
        && track.is_none()
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
    let graph = build_graph_with(
        &segs,
        bed.as_ref(),
        &GraphSpec {
            video: true,
            audio: true,
            pix_fmt: "yuv420p",
            geometry,
            subtitles: subtitles.is_some(),
            mix: project.mix,
            camera_updates: crate::camera::EXPORT_UPDATES_PER_SECOND,
        },
        &pics,
    );
    Ok(Plan::Reencode {
        graph: Box::new(graph),
        settings,
        subtitles,
        track,
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
            camera_updates: 0,
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
