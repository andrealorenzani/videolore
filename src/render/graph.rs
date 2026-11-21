// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The single `filter_complex` graph of a render.

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
}

/// Writes the command files of a graph into the working folder `dir`.
pub fn write_command_files(dir: &Path, files: &[(String, String)]) -> io::Result<()> {
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
    users: usize,
) -> Vec<String> {
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
    i: usize,
    pix_fmt: &str,
    files: &mut Vec<(String, String)>,
) -> String {
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
    let o = s.skip.ff();
    let blur = matches!(f.aspect.map(|a| a.fill), Some(frame::Fill::Blur));
    // stage B: the background
    if blur {
        let (sw, sh) = (
            (w / frame::BLUR_DIVISOR).max(frame::BLUR_MIN_SIDE) & !1,
            (h / frame::BLUR_DIVISOR).max(frame::BLUR_MIN_SIDE) & !1,
        );
        let r = blur_radius(sw, sh, f.aspect.map(|a| a.blur).unwrap_or_default());
        let blurred = format!(
            "[pb{i}]scale={sw}:{sh}:force_original_aspect_ratio=increase,crop={sw}:{sh},\
             boxblur={r}:2,scale={w}:{h},setsar=1[b{i}]"
        );
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
    build_graph_bg(segs, bed, spec, Background::Normal)
}

/// [`build_graph`] with the way the blurred background is made (REQ-223).
pub fn build_graph_bg(
    segs: &[Segment],
    bed: Option<&MusicBed>,
    spec: &GraphSpec,
    bg: Background,
) -> Graph {
    let spans: Vec<(PathBuf, Micros, Micros)> = segs
        .iter()
        .map(|s| {
            (
                s.path.clone(),
                s.src_in + s.skip - s.read_back(),
                s.src_in + s.len,
            )
        })
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
        // the picture starts at the source frame that holds the first instant (REQ-155)
        let back = s.read_back();
        let (voff, vd) = (
            (s.src_in + s.skip - back - inputs[k].start).ff(),
            (s.rendered() + back).ff(),
        );
        // that frame keeps its own time, as late as the first instant at the earliest
        let shift = format!(
            "PTS-STARTPTS+{}/TB",
            (s.skip - s.lead()).max(Micros::ZERO).ff()
        );
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
                let cmds = camera::command_lines_capped(
                    &format!("v360@{id}"),
                    path,
                    from,
                    s.src_in + s.len,
                    g.fps,
                    (cw, ch),
                    spec.camera_updates,
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
                let crop = crop_filter(&s.crop);
                match fr {
                    // the picture fitted to the frame, with no bars: the frame effect
                    // draws the background and places the picture
                    Some(f) => format!(
                        "{src}trim=start={voff}:duration={vd},setpts={shift}{crop}{fit},\
                         setsar=1,fps={fps}{lut}{anchored}",
                        fit = frame::fit_filter((w, h), f.aspect.as_ref()),
                        fps = g.fps.ff(),
                    ),
                    None => format!(
                        "{src}trim=start={voff}:duration={vd},setpts={shift}{crop},\
                         scale={w}:{h}:force_original_aspect_ratio=decrease{lut},\
                         pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps={f}{anchored},\
                         format={pf}",
                        f = g.fps.ff(),
                        pf = spec.pix_fmt,
                    ),
                }
            };
            if let Some(f) = fr {
                c = compose_frame(&mut parts, i, s, f, spec, c, bg);
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
            c.push_str(&format!(",setpts=PTS-STARTPTS{cut}[v{i}]"));
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
            if !s.volume_keys.is_empty() && s.gain > 0.0 {
                // `t` on this chain is the clip's own time (the chain starts at `skip`
                // and its clock is moved by `skip`) (REQ-280)
                let expr = frame::volume_expression(&s.volume_keys, Micros::ZERO);
                c.push_str(&format!(",volume='{expr}':eval=frame"));
            } else if s.gain != 1.0 {
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
    }
}

/// The filter that cuts away the part of the picture chosen in Crop (REQ-283), written for
/// the sides in percent, in even pixels: `,crop=w:h:x:y`; nothing when there is no crop.
pub(super) fn crop_filter(c: &crate::timeline::Crop) -> String {
    if c.is_none() {
        return String::new();
    }
    let (l, t, r, b) = (c.left, c.top, c.right, c.bottom);
    format!(
        ",crop=w='trunc(iw*{}/200)*2':h='trunc(ih*{}/200)*2':x='trunc(iw*{l}/200)*2':y='trunc(ih*{t}/200)*2'",
        100 - u32::from(l) - u32::from(r),
        100 - u32::from(t) - u32::from(b)
    )
}
