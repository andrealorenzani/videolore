// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The single `filter_complex` graph of a render.

use super::overlay::OverlayStep;
use super::*;

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
    /// Most camera updates a second for a 360 clip (REQ-266): 0 = every frame (the
    /// preview); the export uses [`camera::EXPORT_UPDATES_PER_SECOND`].
    pub camera_updates: u32,
}

/// Inputs and filter graph; outputs are labelled `[outv]` and `[outa]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub inputs: Vec<Input>,
    pub graph: String,
    /// Files the graph reads from the working folder: `(name, content)`, the camera
    /// commands of the 360 clips (written by [`write_command_files`]).
    pub command_files: Vec<(String, String)>,
    /// The LUT tables the graph reads, each as `lut-<hash>.cube` in the working folder
    /// (written once per content by [`write_command_files`], ADR-206).
    pub lut_files: Vec<std::sync::Arc<crate::lut::Lut>>,
    /// The ambisonic sound turned by the camera that the graph reads as inputs (REQ-273): run
    /// [`crate::spatial::prepare`] for each into the working folder before ffmpeg starts.
    pub spatial: Vec<crate::spatial::SpatialJob>,
}

/// Writes the command files of a graph into the working folder `dir`.
pub fn write_command_files(
    dir: &Path,
    files: &[(String, String)],
    luts: &[std::sync::Arc<crate::lut::Lut>],
) -> io::Result<()> {
    for l in luts {
        crate::lut::store::write_copy(dir, l)?;
    }
    for (name, content) in files {
        std::fs::write(dir.join(name), content)?;
        if name.ends_with(".ass") {
            crate::fonts::copy_used(dir, content)?;
        }
    }
    Ok(())
}

/// Labels for the users of each input stream (`split`/`asplit` when shared).
pub(super) fn fan_out(
    parts: &mut Vec<String>,
    input: usize,
    kind: char,
    track: u8,
    users: usize,
) -> Vec<String> {
    let stream = format!("[{input}:{kind}:{track}]");
    if users <= 1 {
        return vec![stream];
    }
    let tag = if track == 0 {
        String::new()
    } else {
        format!("t{track}")
    };
    let labels: Vec<String> = (0..users)
        .map(|j| format!("[{kind}{input}{tag}s{j}]"))
        .collect();
    let filter = if kind == 'a' { "asplit" } else { "split" };
    parts.push(format!("{stream}{filter}={users}{}", labels.concat()));
    labels
}

/// The fade filters of a segment. `early` moves the fade-out that much earlier (for the
/// picture of the fade at the very end of the video, so that the last frame is black).
pub(super) fn fades(c: &mut String, filter: &str, s: &Segment, early: Micros) {
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
pub(super) fn lut_chain(
    s: &Segment,
    pix_fmt: &str,
    luts: &mut Vec<std::sync::Arc<crate::lut::Lut>>,
) -> String {
    let mut c = String::new();
    if let Some(l) = &s.lut_clip {
        let name = crate::lut::store::copy_name(l);
        c.push_str(&format!(",lut3d=file={name}:interp=tetrahedral"));
        if !luts.iter().any(|x| x.hash() == l.hash()) {
            luts.push(l.clone());
        }
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
/// Radius of the blur on the small copy. `boxblur` uses it for the luma and, unless told
/// otherwise, the chroma planes; ffmpeg wants it under half of the smaller side of a plane,
/// and the chroma planes are half the size (yuv420).
pub(super) fn blur_radius(sw: u32, sh: u32, level: frame::BlurLevel) -> u32 {
    level.radius().min((sw.min(sh) / 4).saturating_sub(1))
}

/// How the blurred background of a Change aspect frame is made (REQ-223).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Background {
    /// Computed from the picture, as always.
    #[default]
    Normal,
    /// The graph makes only the blurred background (for the preview's cache).
    Only,
    /// The background is one raw picture of the frame size (made by an [`Background::Only`]
    /// run) read as the input after the files; the graph draws only the picture over it.
    /// The number is filled in by the graph builder.
    Cached(usize),
}

/// The chain that makes the blurred background of clip `i` from the picture `[pb{i}]` (the
/// fitted picture): a small copy, blurred, scaled back to the frame, ending in `[b{i}]`.
pub(super) fn blur_chain(i: usize, (w, h): (u32, u32), level: frame::BlurLevel) -> String {
    let (sw, sh) = (
        (w / frame::BLUR_DIVISOR).max(frame::BLUR_MIN_SIDE) & !1,
        (h / frame::BLUR_DIVISOR).max(frame::BLUR_MIN_SIDE) & !1,
    );
    let r = blur_radius(sw, sh, level);
    format!(
        "[pb{i}]scale={sw}:{sh}:force_original_aspect_ratio=increase,crop={sw}:{sh},\
         boxblur={r}:2,scale={w}:{h},setsar=1[b{i}]"
    )
}

/// The chain of a plain colour background of clip `i`, ending in `[b{i}]`, on the clock of
/// the clip (it starts at the clip's first rendered instant).
pub(super) fn colour_chain(
    i: usize,
    s: &Segment,
    (w, h): (u32, u32),
    fps: Fps,
    rgb: frame::Rgb,
) -> String {
    format!(
        "color=c={c}:s={w}x{h}:r={fps}:d={d},setpts=PTS-STARTPTS+{o}/TB[b{i}]",
        c = rgb.ff(),
        fps = fps.ff(),
        d = s.rendered().ff(),
        o = s.skip.ff(),
    )
}

pub(super) fn compose_frame(
    parts: &mut Vec<String>,
    i: usize,
    s: &Segment,
    f: &SegFrame,
    spec: &GraphSpec,
    picture: String,
    bg: Background,
) -> String {
    let g = &spec.geometry;
    let (w, h) = (g.width, g.height);
    let blur = matches!(f.aspect.map(|a| a.fill), Some(frame::Fill::Blur));
    // stage B: the background
    if blur {
        let blurred = blur_chain(i, (w, h), f.aspect.map(|a| a.blur).unwrap_or_default());
        match bg {
            Background::Normal => {
                parts.push(format!("{picture},split=2[pa{i}][pb{i}]"));
                parts.push(blurred);
            }
            Background::Only => {
                parts.push(format!("{picture}[pb{i}]"));
                parts.push(blurred);
                return format!("[b{i}]format={}", spec.pix_fmt);
            }
            Background::Cached(n) => {
                parts.push(format!("{picture}[pa{i}]"));
                // the same instant as the picture's first frame
                let at = (s.skip - s.lead()).max(Micros::ZERO).ff();
                parts.push(format!(
                    "[{n}:v:0]format=rgba,setsar=1,setpts=PTS-STARTPTS+{at}/TB,fps={}[b{i}]",
                    g.fps.ff()
                ));
            }
        }
    } else {
        let rgb = match f.aspect.map(|a| a.fill) {
            Some(frame::Fill::Colour(c)) => c,
            _ => frame::Rgb::BLACK,
        };
        parts.push(format!("{picture}[pa{i}]"));
        parts.push(colour_chain(i, s, (w, h), g.fps, rgb));
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

/// The `setpts` expression that restarts a clip's time and, with a speed, scales it (REQ-281).
fn rate_pts(s: &Segment) -> String {
    if s.speed == crate::timeline::NORMAL_SPEED {
        "PTS-STARTPTS".into()
    } else {
        format!("(PTS-STARTPTS)*1000/{}", s.speed)
    }
}

/// The `atempo` filter of a clip with a speed (one filter covers 0.5 to 2.0): the pitch is
/// kept (REQ-281).
fn tempo_filter(s: &Segment) -> String {
    if s.speed == crate::timeline::NORMAL_SPEED {
        String::new()
    } else {
        format!(",atempo={:.6}", f64::from(s.speed) / 1000.0)
    }
}

/// A picture item drawn over the video (REQ-287): the PNG file, its place and size in the
/// pixels of the picture, and when it shows, counted from the start of this render.
#[derive(Debug, Clone, PartialEq)]
pub struct PicOverlay {
    pub path: PathBuf,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub start: Micros,
    pub end: Micros,
    /// The parts of `start..end` the picture shows when some of it is drawn by the overlay
    /// stage instead (REQ-304); empty: the whole of it.
    pub pieces: Vec<(Micros, Micros)>,
    /// How an overlap (2.3.0) is drawn; a picture item or a PNG overlap is the default: a
    /// looped still fitted to its box.
    pub look: Look,
}

/// What differs between a picture item and the files of an overlap (2.3.0, REQ-327, REQ-328,
/// ADR-232, ADR-243).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Look {
    /// A GIF or a video: read from its start without a loop, so it plays once and the overlay
    /// lets the picture through after it (REQ-328).
    pub motion: bool,
    /// How much of the animation had played when this render starts (a preview from the
    /// middle of an overlap): the input is read from there.
    pub skip: Micros,
    /// A reshaped video fills its box: it is scaled up and the overflow is cut evenly (REQ-327).
    pub cover: bool,
    /// The file is larger than 1920 x 1080 and this is a preview: it is halved first (ADR-243).
    pub half: bool,
}

impl PicOverlay {
    /// The input that reads this overlay.
    pub(super) fn input(&self, video_end: Micros) -> Input {
        Input {
            path: self.path.clone(),
            start: if self.look.motion {
                self.look.skip
            } else {
                Micros::ZERO
            },
            len: video_end,
            still: !self.look.motion,
        }
    }

    /// The filters that turn input `n` into the picture to lay over the video, ending in
    /// `out` (a label with its brackets). Numbers only (ADR-099).
    pub(super) fn prepared(&self, n: usize, out: &str) -> String {
        let mut f = format!("[{n}:v:0]format=rgba");
        if self.look.half {
            f.push_str(",scale=iw/2:ih/2");
        }
        if self.look.motion {
            f.push_str(&format!(
                ",setpts=PTS-STARTPTS+{}/TB",
                (self.start.max(Micros::ZERO)).ff()
            ));
        }
        if self.look.cover {
            f.push_str(&format!(
                ",scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}",
                w = self.w,
                h = self.h
            ));
        } else {
            f.push_str(&format!(",scale={}:{}", self.w, self.h));
        }
        f.push_str(out);
        f
    }

    /// The options of the `overlay` filter after the picture's own place: it shows while
    /// `enable` is true; a still ends with the video, an animation lets the picture through
    /// when it has ended.
    pub(super) fn laid(&self) -> String {
        format!(
            "x={}:y={}:enable='{}':{}",
            self.x,
            self.y,
            self.enable(),
            if self.look.motion {
                "eof_action=pass"
            } else {
                "shortest=1"
            }
        )
    }
}

impl PicOverlay {
    /// The `enable` expression: the picture shows while it is true.
    pub(super) fn enable(&self) -> String {
        let between = |a: Micros, b: Micros| format!("between(t,{},{})", a.ff(), b.ff());
        if self.pieces.is_empty() {
            between(self.start, self.end)
        } else {
            self.pieces
                .iter()
                .map(|(a, b)| between(*a, *b))
                .collect::<Vec<_>>()
                .join("+")
        }
    }
}

/// The single filter graph for `segs` (and the music `bed`).
pub fn build_graph(segs: &[Segment], bed: Option<&MusicBed>, spec: &GraphSpec) -> Graph {
    build_graph_bg(segs, bed, spec, Background::Normal)
}

/// [`build_graph`] with picture items over the video (REQ-287).
pub fn build_graph_with(
    segs: &[Segment],
    bed: Option<&MusicBed>,
    spec: &GraphSpec,
    pics: &[PicOverlay],
) -> Graph {
    build_graph_pics(segs, bed, spec, Background::Normal, pics)
}

/// [`build_graph`] with the way the blurred background is made (REQ-223).
pub fn build_graph_bg(
    segs: &[Segment],
    bed: Option<&MusicBed>,
    spec: &GraphSpec,
    bg: Background,
) -> Graph {
    build_graph_pics(segs, bed, spec, bg, &[])
}

/// [`build_graph_bg`] with picture items over the video (REQ-287).
pub fn build_graph_pics(
    segs: &[Segment],
    bed: Option<&MusicBed>,
    spec: &GraphSpec,
    bg: Background,
    pics: &[PicOverlay],
) -> Graph {
    let pics = if spec.video { pics } else { &[] };
    let spans: Vec<(PathBuf, Micros, Micros)> = segs
        .iter()
        .map(|s| (s.path.clone(), s.src_from() - s.read_back(), s.src_to()))
        .collect();
    let (mut inputs, idx) = group_inputs(&spans);
    let bed = bed.filter(|_| spec.audio);
    let music: Vec<(PathBuf, Micros, Micros)> = bed
        .map(|b| {
            b.parts
                .iter()
                .filter_map(|p| match p {
                    BedPart::Clip { path, src, len, .. } => Some((path.clone(), *src, *src + *len)),
                    BedPart::Gap(_) => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let (music_inputs, music_idx) = group_inputs(&music);
    let offset = inputs.len();
    inputs.extend(music_inputs);
    // the pictures follow the music; the raw background is after them all
    let pic_base = inputs.len();
    let video_end = segs.iter().fold(Micros::ZERO, |a, s| a + s.rendered());
    inputs.extend(pics.iter().map(|p| p.input(video_end)));
    // the picture items of the overlay stage, one input each, after the others' pictures
    let mut seg_pics: Vec<Vec<usize>> = vec![Vec::new(); segs.len()];
    for (i, s) in segs.iter().enumerate() {
        for step in &s.overlay {
            if let (true, OverlayStep::Pic(p)) = (spec.video, step) {
                seg_pics[i].push(inputs.len());
                inputs.push(p.input(s.rendered() + Micros(1_000_000)));
            }
        }
    }
    // the ambisonic sound turned by the camera (REQ-273): one input per clip, after the
    // pictures; the clip's own sound is not read
    let spatial_on = |s: &Segment| spec.audio && s.has_audio && s.spatial.is_some();
    let mut spatial_jobs = Vec::new();
    let mut spatial_input: Vec<Option<usize>> = vec![None; segs.len()];
    for (i, s) in segs.iter().enumerate() {
        if let (true, Some(job)) = (spatial_on(s), &s.spatial) {
            spatial_input[i] = Some(inputs.len());
            inputs.push(Input {
                path: PathBuf::from(job.wav_name()),
                start: Micros::ZERO,
                len: job.len,
                still: false,
            });
            spatial_jobs.push(job.clone());
        }
    }
    // the raw background is the input after all the files
    let bg = match bg {
        Background::Cached(_) => Background::Cached(inputs.len()),
        other => other,
    };

    let mut parts = Vec::new();
    let count = |k: usize, f: &dyn Fn(&Segment) -> bool| {
        idx.iter()
            .zip(segs)
            .filter(|(i, s)| **i == k && f(s))
            .count()
    };
    let mut vlabels: Vec<Vec<String>> = Vec::new();
    let mut alabels: Vec<std::collections::BTreeMap<u8, Vec<String>>> = Vec::new();
    for k in 0..offset {
        vlabels.push(if spec.video {
            fan_out(&mut parts, k, 'v', 0, count(k, &|_| true))
        } else {
            Vec::new()
        });
        // one stream per audio track the clips of this input play (REQ-340)
        let mut by_track: std::collections::BTreeMap<u8, Vec<String>> = Default::default();
        if spec.audio {
            let tracks: std::collections::BTreeSet<u8> = idx
                .iter()
                .zip(segs)
                .filter(|(i, s)| **i == k && s.has_audio && !spatial_on(s))
                .map(|(_, s)| s.audio_track)
                .collect();
            for t in tracks {
                let users = count(k, &|s| s.has_audio && !spatial_on(s) && s.audio_track == t);
                by_track.insert(t, fan_out(&mut parts, k, 'a', t, users));
            }
        }
        alabels.push(by_track);
    }
    // the picture-based subtitle stream of an input, one label for each clip that draws it
    // (REQ-253); the stream is the one the clips choose (they share the project's choice)
    let picture_on = |s: &Segment| spec.video && s.picture_sub.is_some();
    let slabels: Vec<Vec<String>> = (0..offset)
        .map(|k| {
            let users: Vec<&Segment> = idx
                .iter()
                .zip(segs)
                .filter(|(i, s)| **i == k && picture_on(s))
                .map(|(_, s)| s)
                .collect();
            match users.first().and_then(|s| s.picture_sub) {
                Some(n) if users.len() == 1 => vec![format!("[{k}:s:{n}]")],
                Some(n) => {
                    let labels: Vec<String> =
                        (0..users.len()).map(|j| format!("[s{k}s{j}]")).collect();
                    parts.push(format!(
                        "[{k}:s:{n}]split={}{}",
                        users.len(),
                        labels.concat()
                    ));
                    labels
                }
                None => Vec::new(),
            }
        })
        .collect();
    let (mut vused, mut sused) = (vec![0usize; offset], vec![0usize; offset]);
    let mut aused: std::collections::BTreeMap<(usize, u8), usize> = Default::default();
    let g = &spec.geometry;
    let mut command_files: Vec<(String, String)> = Vec::new();
    let mut lut_files: Vec<std::sync::Arc<crate::lut::Lut>> = Vec::new();
    let mut concat_in = String::new();
    for (i, s) in segs.iter().enumerate() {
        let k = idx[i];
        let off = (s.src_from() - inputs[k].start).ff();
        let d = s.rendered().ff();
        // the source time the clip's part spans (equal to `d` at normal speed)
        let ds = s.rendered_src().ff();
        let rate = rate_pts(s);
        // the picture starts at the source frame that holds the first instant (REQ-155)
        let back = s.read_back();
        let (voff, vd) = (
            (s.src_from() - back - inputs[k].start).ff(),
            (s.rendered_src() + back).ff(),
        );
        // that frame keeps its own time, as late as the first instant at the earliest
        let shift = format!("{rate}+{}/TB", (s.skip - s.lead()).max(Micros::ZERO).ff());
        if spec.video {
            let mut src = vlabels[k][vused[k]].clone();
            vused[k] += 1;
            if picture_on(s) {
                // the file's own picture subtitles drawn on the picture, in the pixels of the
                // video: the picture and its subtitles are then fitted together (REQ-253)
                let sub = &slabels[k][sused[k]];
                sused[k] += 1;
                parts.push(format!(
                    "{src}{sub}overlay=eof_action=pass:format=auto[ps{i}]"
                ));
                src = format!("[ps{i}]");
            }
            let src = &src;
            let fr = s.frame.as_ref();
            // a clip with its own Change aspect is made in its own frame (a size of the
            // project's, with the clip's shape), then placed in the project's frame (REQ-286)
            let outer = (g.width, g.height);
            let own_spec;
            let (spec, g) = match fr.filter(|f| f.own).and_then(|f| f.aspect) {
                Some(a) => {
                    let (fw, fh) = frame::frame_size(outer, Some(&a));
                    own_spec = GraphSpec {
                        geometry: Geometry {
                            width: fw,
                            height: fh,
                            ..*g
                        },
                        ..*spec
                    };
                    (&own_spec, &own_spec.geometry)
                }
                None => (spec, g),
            };
            let (w, h) = (g.width, g.height);
            // the camera view fills the output picture, or the kept side of a Change aspect
            let (cw, ch) = match fr {
                Some(f) => frame::camera_size((w, h), f.aspect.as_ref()),
                None => (w, h),
            };
            let lut = lut_chain(s, spec.pix_fmt, &mut lut_files);
            let by_steps = !s.steps.is_empty();
            let mut c = if let Some(path) = &s.sphere {
                // the camera view fills the output picture: no scale, no bars
                let from = s.src_from();
                let cam = camera::at(path, from).unwrap_or(Camera::FRONT);
                let id = format!("c{i}");
                let cmds = camera::command_lines_speed(
                    &format!("v360@{id}"),
                    path,
                    from,
                    s.src_to(),
                    g.fps,
                    (cw, ch),
                    spec.camera_updates,
                    s.speed,
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
                    "{src}trim=start={off}:duration={ds},setpts={rate},fps={f}{lut_before},{anchored}{send}{v360},\
                     setsar=1{lut_after},setpts=PTS+{o}/TB{tail}",
                    v360 = camera::filter(&id, cam, cw, ch),
                    o = s.skip.ff(),
                    f = g.fps.ff(),
                )
            } else if by_steps {
                // the rows of the clip are built one by one (REQ-320): the source, trimmed and on
                // the clip's clock
                format!("{src}trim=start={voff}:duration={vd},setpts={shift}")
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
                        "{src}trim=start={voff}:duration={vd},setpts={shift}{fit},\
                         setsar=1,fps={fps}{lut}{anchored}",
                        fit = frame::fit_filter((w, h), f.aspect.as_ref()),
                        fps = g.fps.ff(),
                    ),
                    None => format!(
                        "{src}trim=start={voff}:duration={vd},setpts={shift},\
                         scale={w}:{h}:force_original_aspect_ratio=decrease{lut},\
                         pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={f}{anchored},\
                         format={pf}",
                        f = g.fps.ff(),
                        pf = spec.pix_fmt,
                    ),
                }
            };
            let c = if by_steps {
                let ctx = steps::Ctx {
                    i,
                    s,
                    spec,
                    outer,
                    pics: &seg_pics[i],
                    sphere: s.sphere.is_some(),
                };
                steps::chain(&mut parts, &ctx, c, (&mut lut_files, &mut command_files))
            } else {
                if let Some(f) = fr {
                    c = compose_frame(&mut parts, i, s, f, spec, c, bg);
                    if f.own && (outer.0, outer.1) != (w, h) {
                        c.push_str(&format!(
                            ",scale={ow}:{oh}:force_original_aspect_ratio=decrease,\
                             pad={ow}:{oh}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,format={pf}",
                            ow = outer.0,
                            oh = outer.1,
                            pf = spec.pix_fmt
                        ));
                    }
                }
                // two frames: sources often start a few milliseconds late, which costs one
                let early = if s.to_black {
                    Micros(g.fps.frame_duration().0 * 2)
                } else {
                    Micros::ZERO
                };
                fades(&mut c, "fade", s, early);
                // the picture read from the frame before the first instant ends with the clip
                let cut = if back > Micros::ZERO {
                    format!(",trim=duration={d}")
                } else {
                    String::new()
                };
                c.push_str(&format!(",setpts=PTS-STARTPTS{cut}"));
                overlay_chain(
                    &mut parts,
                    (c, i),
                    s,
                    spec.pix_fmt,
                    &seg_pics[i],
                    (&mut lut_files, &mut command_files),
                )
            };
            parts.push(format!("{c}[v{i}]"));
            concat_in.push_str(&format!("[v{i}]"));
        }
        if spec.audio {
            let mut c = if s.has_audio {
                // the turned sound is a file of the clip's own part: it starts at the clip's
                // first instant
                let (src, off) = match spatial_input[i] {
                    Some(n) => (format!("[{n}:a:0]"), s.to_src(s.skip).ff()),
                    None => {
                        let n = aused.entry((k, s.audio_track)).or_default();
                        *n += 1;
                        (alabels[k][&s.audio_track][*n - 1].clone(), off.clone())
                    }
                };
                format!(
                    "{src}atrim=start={off}:duration={ds},asetpts=PTS-STARTPTS,\
                     aresample={SAMPLE_RATE},{AFORMAT}{tempo},apad=whole_dur={d},atrim=duration={d},\
                     asetpts=PTS-STARTPTS+{o}/TB",
                    o = s.skip.ff(),
                    tempo = tempo_filter(s)
                )
            } else {
                format!(
                    "anullsrc=r={SAMPLE_RATE}:cl=stereo,atrim=duration={d},{AFORMAT},\
                     asetpts=PTS-STARTPTS+{o}/TB",
                    o = s.skip.ff()
                )
            };
            // the clip's Volume (REQ-280); `t` on this chain is the clip's own time (the chain
            // starts at `skip` and its clock is moved by `skip`)
            let volume = if !s.volume_keys.is_empty() && s.gain > 0.0 {
                let expr = frame::volume_expression(&s.volume_keys, Micros::ZERO);
                format!(",volume='{expr}':eval=frame")
            } else if s.gain != 1.0 {
                format!(",volume={:.6}", s.gain)
            } else {
                String::new()
            };
            // Volume and Clean audio are in the order of the clip's sound group (REQ-320)
            if s.volume_first {
                c.push_str(&volume);
            }
            if s.clean && s.has_audio && s.clean_strength >= 100 {
                c.push(',');
                c.push_str(CLEAN_AUDIO);
            } else if s.clean && s.has_audio && s.clean_strength > 0 {
                // a part of the clean sound mixed with the sound as it was (REQ-278)
                let w = f64::from(s.clean_strength) / 100.0;
                parts.push(format!("{c},asplit=2[cd{i}][cw{i}]"));
                parts.push(format!("[cw{i}]{CLEAN_AUDIO}[cc{i}]"));
                c = format!(
                    "[cd{i}][cc{i}]amix=inputs=2:weights='{:.4} {w:.4}':normalize=0:duration=first",
                    1.0 - w
                );
            }
            if s.speech && s.has_audio {
                c.push(',');
                c.push_str(NORMALISE_SPEECH);
            }
            if !s.volume_first {
                c.push_str(&volume);
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
        let last = if spec.subtitles { "[cv]" } else { "[outv]" };
        outs.push_str(if pics.is_empty() { last } else { "[pv0]" });
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
    for (j, p) in pics.iter().enumerate() {
        // the picture at its size, over the video while it shows
        let last = j + 1 == pics.len();
        let next = if !last {
            format!("[pv{}]", j + 1)
        } else if spec.subtitles {
            "[cv]".to_string()
        } else {
            "[outv]".to_string()
        };
        parts.push(format!(
            "{prep};[pv{j}][pi{j}]overlay={laid},format={pf}{next}",
            prep = p.prepared(pic_base + j, &format!("[pi{j}]")),
            laid = p.laid(),
            pf = spec.pix_fmt,
        ));
    }
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
            mlabels.push(fan_out(&mut parts, offset + k, 'a', 0, users[j]));
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
                BedPart::Clip {
                    src,
                    len,
                    gain,
                    fade_in,
                    fade_out,
                    clean,
                    ..
                } => {
                    let k = music_idx[clip_no];
                    clip_no += 1;
                    let slot = first_of.iter().position(|x| *x == k).unwrap_or(0);
                    let label = &mlabels[slot][mused[slot]];
                    mused[slot] += 1;
                    let off = (*src - inputs[offset + k].start).ff();
                    let d = len.ff();
                    // the clip's own clean sound, level and fades (REQ-276, REQ-277)
                    let mut extra = String::new();
                    if *clean {
                        extra.push(',');
                        extra.push_str(CLEAN_AUDIO);
                    }
                    if *gain != 1.0 {
                        extra.push_str(&format!(",volume={gain:.6}"));
                    }
                    if *fade_in > Micros::ZERO {
                        extra.push_str(&format!(",afade=t=in:st=0:d={}", fade_in.ff()));
                    }
                    if *fade_out > Micros::ZERO {
                        extra.push_str(&format!(
                            ",afade=t=out:st={}:d={}",
                            (*len - *fade_out).ff(),
                            fade_out.ff()
                        ));
                    }
                    format!(
                        "{label}atrim=start={off}:duration={d},asetpts=PTS-STARTPTS,\
                         aresample={SAMPLE_RATE},{AFORMAT},apad=whole_dur={d},atrim=duration={d},\
                         asetpts=PTS-STARTPTS{extra}[m{j}]"
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
        lut_files,
        spatial: spatial_jobs,
    }
}

/// Appends the overlay steps of a clip (REQ-304) to its chain `c` (of clip `i`): the LUT and
/// the texts and lines are filters of the chain, a picture closes it and overlays the picture
/// file. Returns the chain with its last filter (without the output label: a chain that ends
/// with a label gets a `null` filter from the caller's label).
fn overlay_chain(
    parts: &mut Vec<String>,
    (mut c, i): (String, usize),
    s: &Segment,
    pix_fmt: &str,
    pics: &[usize],
    (luts, files): (
        &mut Vec<std::sync::Arc<crate::lut::Lut>>,
        &mut Vec<(String, String)>,
    ),
) -> String {
    let mut pic = pics.iter();
    for (n, step) in s.overlay.iter().enumerate() {
        let f = match step {
            OverlayStep::Lut(l) => {
                if !luts.iter().any(|x| x.hash() == l.hash()) {
                    luts.push(l.clone());
                }
                format!(
                    ",format=gbrp,lut3d=file={}:interp=tetrahedral,format={pix_fmt}",
                    crate::lut::store::copy_name(l)
                )
            }
            OverlayStep::Ass { name, doc } => {
                files.push((name.clone(), doc.clone()));
                format!(",subtitles=f={name}:fontsdir={}", ass::FONTS_DIR)
            }
            OverlayStep::Pic(p) => {
                let Some(input) = pic.next() else { continue };
                parts.push(format!("{c}[o{i}x{n}]"));
                parts.push(format!(
                    "{prep};[o{i}x{n}][pi{i}x{n}]overlay={laid},format={pix_fmt}[q{i}x{n}]",
                    prep = p.prepared(*input, &format!("[pi{i}x{n}]")),
                    laid = p.laid(),
                ));
                c = format!("[q{i}x{n}]");
                continue;
            }
        };
        if c.ends_with(']') {
            c.push_str(&f[1..]);
        } else {
            c.push_str(&f);
        }
    }
    if c.ends_with(']') {
        c.push_str("null");
    }
    c
}
