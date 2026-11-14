// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Turns the timeline into ffmpeg work: the export [`Plan`] (stream copy or one re-encode
//! pass with a single `filter_complex`) and the argv for export and preview.
//!
//! Every clip becomes its own input (`-ss <in> -t <len> -i file:<path>`), referenced by
//! index in the graph. Per clip: trim -> setpts -> scale (keep aspect) -> pad (letterbox)
//! -> setsar -> fps -> format -> fades; audio: atrim -> aresample -> aformat -> apad ->
//! afades (or anullsrc when the source has no audio); then a single concat.

use crate::ffmpeg::{FfVersion, argv, file_arg, filter_graph_args};
use crate::format::{self, EncodeSettings};
use crate::media::MediaItem;
use crate::project::Project;
use crate::time::{Fps, Micros};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// Height of the preview video.
pub const PREVIEW_HEIGHT: u32 = 360;

/// Sample rate used inside the graphs.
pub const SAMPLE_RATE: u32 = 48_000;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RenderError {
    #[error("the timeline is empty")]
    Empty,
    #[error("source '{0}' is missing")]
    Missing(String),
    #[error("a clip refers to an unknown source")]
    UnknownMedia,
}

/// One clip as seen by the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
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
pub fn segments(project: &Project, from: Micros) -> Result<Vec<Segment>, RenderError> {
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
                path: m.path.clone(),
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

/// Which streams a graph produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Streams {
    pub video: bool,
    pub audio: bool,
}

fn video_chain(i: usize, s: &Segment, g: &Geometry, pix_fmt: &str) -> String {
    let (w, h) = (g.width, g.height);
    let mut c = format!(
        "[{i}:v:0]trim=duration={d},setpts=PTS-STARTPTS+{o}/TB,\
         scale={w}:{h}:force_original_aspect_ratio=decrease,\
         pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={f},format={pix_fmt}",
        d = s.rendered().ff(),
        o = s.skip.ff(),
        f = g.fps.ff(),
    );
    if s.fade_in > Micros::ZERO {
        c.push_str(&format!(",fade=t=in:st=0:d={}", s.fade_in.ff()));
    }
    if s.fade_out > Micros::ZERO {
        c.push_str(&format!(
            ",fade=t=out:st={}:d={}",
            (s.len - s.fade_out).ff(),
            s.fade_out.ff()
        ));
    }
    c.push_str(&format!(",setpts=PTS-STARTPTS[v{i}]"));
    c
}

fn audio_chain(i: usize, s: &Segment) -> String {
    let d = s.rendered().ff();
    let fmt = "aformat=sample_fmts=fltp:channel_layouts=stereo";
    if !s.has_audio {
        return format!(
            "anullsrc=r={SAMPLE_RATE}:cl=stereo,atrim=duration={d},{fmt},asetpts=PTS-STARTPTS[a{i}]"
        );
    }
    let mut c = format!(
        "[{i}:a:0]atrim=duration={d},asetpts=PTS-STARTPTS,aresample={SAMPLE_RATE},{fmt},\
         apad=whole_dur={d},atrim=duration={d},asetpts=PTS-STARTPTS+{o}/TB",
        o = s.skip.ff()
    );
    if s.fade_in > Micros::ZERO {
        c.push_str(&format!(",afade=t=in:st=0:d={}", s.fade_in.ff()));
    }
    if s.fade_out > Micros::ZERO {
        c.push_str(&format!(
            ",afade=t=out:st={}:d={}",
            (s.len - s.fade_out).ff(),
            s.fade_out.ff()
        ));
    }
    c.push_str(&format!(",asetpts=PTS-STARTPTS[a{i}]"));
    c
}

/// The single filter graph for `segs`; outputs are labelled `[outv]` and `[outa]`.
pub fn filter_graph(segs: &[Segment], g: &Geometry, streams: Streams, pix_fmt: &str) -> String {
    let mut parts = Vec::new();
    let mut labels = String::new();
    for (i, s) in segs.iter().enumerate() {
        if streams.video {
            parts.push(video_chain(i, s, g, pix_fmt));
            labels.push_str(&format!("[v{i}]"));
        }
        if streams.audio {
            parts.push(audio_chain(i, s));
            labels.push_str(&format!("[a{i}]"));
        }
    }
    let mut outs = String::new();
    if streams.video {
        outs.push_str("[outv]");
    }
    if streams.audio {
        outs.push_str("[outa]");
    }
    parts.push(format!(
        "{labels}concat=n={}:v={}:a={}{outs}",
        segs.len(),
        u8::from(streams.video),
        u8::from(streams.audio)
    ));
    parts.join(";")
}

/// Common leading options.
fn head() -> Vec<OsString> {
    argv(["-hide_banner", "-nostdin", "-loglevel", "error", "-y"])
}

/// One input per segment, seeking with `-ss` before `-i`.
fn segment_inputs(segs: &[Segment]) -> Vec<OsString> {
    let mut a = Vec::new();
    for s in segs {
        a.extend(argv([
            "-protocol_whitelist".to_string(),
            "file,pipe".to_string(),
            "-ss".to_string(),
            (s.src_in + s.skip).ff(),
            "-t".to_string(),
            (s.rendered() + Micros::SECOND).ff(),
            "-i".to_string(),
        ]));
        a.push(file_arg(&s.path));
    }
    a
}

/// How the export is done.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Concat demuxer with `-c copy`; `concat_list` is the list file content.
    StreamCopy {
        concat_list: String,
        muxer: String,
        faststart: bool,
    },
    /// One re-encode pass.
    Reencode {
        filter_graph: String,
        settings: EncodeSettings,
    },
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

/// Decides between stream copy and re-encode. `is_keyframe(media, t)` is only consulted
/// when the formats are identical and there are no fades.
pub fn plan_export(
    project: &Project,
    encoders: &std::collections::BTreeSet<String>,
    mut is_keyframe: impl FnMut(&MediaItem, Micros) -> bool,
) -> Result<Plan, RenderError> {
    let segs = segments(project, Micros::ZERO)?;
    let media = timeline_media(project)?;
    let formats: Vec<_> = media.iter().map(|m| &m.format).collect();
    let chosen = format::choose_output(&formats).ok_or(RenderError::Empty)?;
    let no_fades = project.timeline.fades.is_empty();
    if format::all_identical(&formats) && no_fades {
        let keyframes_ok = project
            .timeline
            .clips
            .iter()
            .zip(&media)
            .all(|(c, m)| c.src_in == Micros::ZERO || is_keyframe(m, c.src_in));
        if keyframes_ok {
            let settings = format::encode_settings(&chosen, encoders);
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
            });
        }
    }
    let settings = format::encode_settings(&chosen, encoders);
    let graph = filter_graph(
        &segs,
        &Geometry::of(&settings),
        Streams {
            video: true,
            audio: true,
        },
        "yuv420p",
    );
    Ok(Plan::Reencode {
        filter_graph: graph,
        settings,
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

/// Full ffmpeg argv for an export. Writes the concat list / graph script when needed.
pub fn export_args(
    plan: &Plan,
    project: &Project,
    paths: &ExportPaths<'_>,
    version: FfVersion,
) -> io::Result<Vec<OsString>> {
    let mut a = head();
    match plan {
        Plan::StreamCopy {
            concat_list,
            muxer,
            faststart,
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
            if *faststart {
                a.extend(argv(["-movflags", "+faststart"]));
            }
            a.extend(argv(["-f", muxer.as_str()]));
        }
        Plan::Reencode {
            filter_graph,
            settings: s,
        } => {
            let segs = segments(project, Micros::ZERO).map_err(io::Error::other)?;
            a.extend(segment_inputs(&segs));
            a.extend(filter_graph_args(
                filter_graph,
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
                let br = br.to_string();
                let buf = (s.video_bitrate.unwrap_or(0) * 2).to_string();
                a.extend(argv(["-b:v", &br, "-maxrate", &br, "-bufsize", &buf]));
            }
            a.extend(argv(["-pix_fmt", "yuv420p", "-c:a", &s.aencoder]));
            if let Some(br) = s.audio_bitrate {
                a.extend(argv(["-b:a".to_string(), br.to_string()]));
            }
            a.extend(argv(["-ar".to_string(), SAMPLE_RATE.to_string()]));
            if s.faststart {
                a.extend(argv(["-movflags", "+faststart"]));
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

/// argv producing raw RGBA frames of `g` on stdout, from timeline position `from`.
pub fn preview_video_args(
    project: &Project,
    from: Micros,
    g: &Geometry,
    version: FfVersion,
    script: &Path,
) -> Result<Vec<OsString>, String> {
    let segs = segments(project, from).map_err(|e| e.to_string())?;
    let graph = filter_graph(
        &segs,
        g,
        Streams {
            video: true,
            audio: false,
        },
        "rgba",
    );
    let mut a = head();
    a.extend(segment_inputs(&segs));
    a.extend(filter_graph_args(&graph, version, script).map_err(|e| e.to_string())?);
    a.extend(argv([
        "-map", "[outv]", "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1",
    ]));
    Ok(a)
}

/// argv producing interleaved s16le PCM (`rate` Hz, `channels`) on stdout from `from`.
pub fn preview_audio_args(
    project: &Project,
    from: Micros,
    rate: u32,
    channels: u16,
    version: FfVersion,
    script: &Path,
) -> Result<Vec<OsString>, String> {
    let segs = segments(project, from).map_err(|e| e.to_string())?;
    let g = Geometry {
        width: 2,
        height: 2,
        fps: Fps { num: 1, den: 1 },
    };
    let graph = filter_graph(
        &segs,
        &g,
        Streams {
            video: false,
            audio: true,
        },
        "rgba",
    );
    let mut a = head();
    a.extend(segment_inputs(&segs));
    a.extend(filter_graph_args(&graph, version, script).map_err(|e| e.to_string())?);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;
    use crate::timeline::tests::media;

    fn project(fmts: &[(u64, u32, u32, u32)]) -> Project {
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

    fn enc() -> std::collections::BTreeSet<String> {
        ["libx264", "aac"].iter().map(|s| s.to_string()).collect()
    }

    fn strs(a: &[OsString]) -> Vec<String> {
        a.iter().map(|s| s.to_string_lossy().into_owned()).collect()
    }

    // covers: REQ-023
    #[test]
    fn identical_without_fades_uses_stream_copy() {
        let p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        let plan = plan_export(&p, &enc(), |_, _| false).unwrap();
        let Plan::StreamCopy {
            concat_list,
            muxer,
            faststart,
        } = &plan
        else {
            panic!("expected stream copy")
        };
        assert_eq!(muxer, "mp4");
        assert!(faststart);
        assert!(concat_list.contains("file 'file:/m/1.mp4'\ninpoint 0.000000\noutpoint 2.000000"));
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("o.mp4");
        let list = dir.path().join("l.txt");
        let script = dir.path().join("g.txt");
        let paths = ExportPaths {
            output: &out,
            concat_list: &list,
            graph_script: &script,
        };
        let a = strs(&export_args(&plan, &p, &paths, FfVersion::Release(6, 1, 0)).unwrap());
        assert!(a.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(a.windows(2).any(|w| w == ["-safe", "0"]));
        assert!(a.last().unwrap().starts_with("file:"));
        assert_eq!(std::fs::read_to_string(&list).unwrap(), *concat_list);
    }

    // covers: REQ-023
    #[test]
    fn non_keyframe_start_or_fade_forces_reencode() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        let id = p.timeline.clips[1].id;
        p.timeline
            .trim(
                id,
                Micros(500_000),
                Micros(3_000_000),
                Micros(3_000_000),
                Micros(1),
            )
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

    // covers: REQ-022
    #[test]
    fn mixed_formats_scale_and_pad_to_lowest() {
        let p = project(&[(2, 1280, 720, 30), (2, 640, 360, 25)]);
        let Plan::Reencode {
            filter_graph,
            settings,
        } = plan_export(&p, &enc(), |_, _| true).unwrap()
        else {
            panic!("expected re-encode")
        };
        assert_eq!((settings.width, settings.height), (640, 360));
        assert_eq!(settings.fps, Fps::new(25, 1).unwrap());
        assert!(filter_graph.contains("scale=640:360:force_original_aspect_ratio=decrease"));
        assert!(filter_graph.contains("pad=640:360:(ow-iw)/2:(oh-ih)/2:color=black"));
        assert!(filter_graph.contains("fps=25/1"));
        assert!(!filter_graph.contains("stretch"));
        assert!(filter_graph.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"));
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("o.mp4");
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let paths = ExportPaths {
            output: &out,
            concat_list: &out,
            graph_script: &out,
        };
        let a = strs(&export_args(&plan, &p, &paths, FfVersion::Release(6, 1, 0)).unwrap());
        assert!(a.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(a.windows(2).any(|w| w == ["-b:v", "1000000"]));
        assert!(a.windows(2).any(|w| w == ["-movflags", "+faststart"]));
        assert_eq!(a.iter().filter(|x| *x == "-i").count(), 2);
    }

    // covers: REQ-011
    #[test]
    fn fades_in_graph_video_and_audio() {
        let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25)]);
        p.media[1].format.acodec = None;
        p.timeline.toggle_fade(0).unwrap();
        let segs = segments(&p, Micros::ZERO).unwrap();
        assert_eq!(segs[0].fade_out, Micros(500_000));
        assert_eq!(segs[1].fade_in, Micros(500_000));
        let g = filter_graph(
            &segs,
            &Geometry {
                width: 640,
                height: 360,
                fps: Fps::new(25, 1).unwrap(),
            },
            Streams {
                video: true,
                audio: true,
            },
            "yuv420p",
        );
        assert!(g.contains("fade=t=out:st=1.500000:d=0.500000"));
        assert!(g.contains("afade=t=out:st=1.500000:d=0.500000"));
        assert!(g.contains("fade=t=in:st=0:d=0.500000"));
        assert!(g.contains("anullsrc=r=48000:cl=stereo,atrim=duration=3.000000"));
        // the total length is the sum of the clips: fades do not shorten anything
        let total: Micros = segs.iter().fold(Micros::ZERO, |a, s| a + s.rendered());
        assert_eq!(total, p.timeline.duration());
    }

    // covers: REQ-013
    #[test]
    fn preview_starts_mid_timeline_at_360p() {
        let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
        p.timeline.toggle_fade(0).unwrap();
        let segs = segments(&p, Micros(2_500_000)).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].skip, Micros(500_000));
        assert_eq!(segs[0].rendered(), Micros(2_500_000));
        let g = output_geometry(&p, &enc()).unwrap().preview();
        assert_eq!((g.width, g.height), (640, 360));
        let dir = tempfile::tempdir().unwrap();
        let s = dir.path().join("g");
        let a = strs(&preview_video_args(&p, Micros(2_500_000), &g, FfVersion::Dev, &s).unwrap());
        assert!(a.windows(2).any(|w| w == ["-ss", "0.500000"]));
        assert!(a.windows(2).any(|w| w == ["-pix_fmt", "rgba"]));
        let fc = a.iter().position(|x| x == "-filter_complex").unwrap();
        assert!(a[fc + 1].contains("setpts=PTS-STARTPTS+0.500000/TB"));
        assert!(a[fc + 1].contains("fade=t=in:st=0:d=0.500000"));
        let a = strs(&preview_audio_args(&p, Micros::ZERO, 44_100, 2, FfVersion::Dev, &s).unwrap());
        assert!(a.windows(2).any(|w| w == ["-ar", "44100"]));
        assert!(a.windows(2).any(|w| w == ["-f", "s16le"]));
        assert!(preview_video_args(&p, Micros(99_000_000), &g, FfVersion::Dev, &s).is_err());
        assert!(preview_audio_args(&p, Micros(99_000_000), 1, 1, FfVersion::Dev, &s).is_err());
        let small = Geometry {
            width: 320,
            height: 240,
            fps: Fps::new(25, 1).unwrap(),
        };
        assert_eq!(small.preview(), small);
    }

    #[test]
    fn errors_and_quoting() {
        let p = Project::default();
        assert_eq!(segments(&p, Micros::ZERO), Err(RenderError::Empty));
        assert_eq!(
            plan_export(&p, &enc(), |_, _| true),
            Err(RenderError::Empty)
        );
        let mut p = project(&[(2, 640, 360, 25)]);
        p.media[0].missing = true;
        assert_eq!(
            segments(&p, Micros::ZERO),
            Err(RenderError::Missing("1.mp4".into()))
        );
        p.media.clear();
        assert_eq!(segments(&p, Micros::ZERO), Err(RenderError::UnknownMedia));
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
