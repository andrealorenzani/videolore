// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Segments of the timeline: what each clip contributes to a render, the 360 pieces and the effective format.

use super::*;

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
    /// The timeline clip this part is made from.
    pub clip: crate::timeline::ClipId,
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
    /// Playing speed in thousandths (1000 = normal, REQ-281): `src_in`, `lead` and the
    /// instants read from the file are source time, `len`, `skip` and the output are timeline time.
    pub speed: u16,
    /// Linear gain of the clip's own sound (1.0 = unchanged).
    pub gain: f64,
    /// Clean audio is on for this clip (REQ-114): applied before the gain and the fades.
    pub clean: bool,
    /// How much of the clean sound is used, 0 to 100 (REQ-278).
    pub clean_strength: u8,
    /// Normalise speech (REQ-279).
    pub speech: bool,
    /// Volume keys in percent in the clip's own time (REQ-280).
    pub volume_keys: Vec<crate::keyframes::Key<u8>>,
    /// Part of the picture cut away before the fit (REQ-283).
    pub crop: crate::timeline::Crop,
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
    /// The frame rate of the source picture (REQ-155): the frame that holds the first
    /// rendered instant starts before it and must not be dropped.
    pub grid: Option<crate::time::Fps>,
    /// First-order ambisonic sound turned by the camera (REQ-273): the pre-pass to run and
    /// whose file replaces the clip's own sound. `None`: the sound is read from the clip.
    pub spatial: Option<crate::spatial::SpatialJob>,
    /// The ordinal of the picture-based subtitle track drawn over this clip (REQ-253, the
    /// player): the file's own subtitle stream is overlaid on the picture. Not on a 360 clip.
    pub picture_sub: Option<u8>,
    /// What the overlay stage draws over this clip, bottom to top (REQ-304); empty for a clip
    /// with the order of the older versions, which keeps its LUT in its own chain and has its
    /// texts, lines and pictures drawn after the concat. See [`overlay`].
    pub overlay: Vec<overlay::OverlayStep>,
}

/// The LUT of `target` for a render: `None` without one; a file that cannot be used is an
/// error for an export (REQ-136: nothing is written) and ignored for a preview.
pub(super) fn lut_for(
    tl: &crate::timeline::Timeline,
    target: crate::timeline::FrameTarget,
    src: Source,
) -> Result<Option<std::sync::Arc<crate::lut::Lut>>, RenderError> {
    let Some(path) = tl.lut_of(target) else {
        return Ok(None);
    };
    match crate::lut::cached(path) {
        Ok(l) => {
            let strength = tl.lut_strength_of(target);
            Ok(Some(if strength >= 100 {
                l
            } else {
                std::sync::Arc::new(l.with_strength(strength))
            }))
        }
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
    /// The aspect is this clip's own (REQ-286): made in its own frame, then placed in the
    /// project's frame.
    pub own: bool,
    pub stack: frame::Stack,
}

impl Segment {
    /// A length of timeline time as source time.
    pub fn to_src(&self, t: Micros) -> Micros {
        if self.speed == crate::timeline::NORMAL_SPEED {
            return t;
        }
        Micros((i128::from(t.0) * i128::from(self.speed) / 1000) as i64)
    }

    /// A length of source time as timeline time.
    pub fn to_timeline(&self, t: Micros) -> Micros {
        if self.speed == crate::timeline::NORMAL_SPEED {
            return t;
        }
        Micros((i128::from(t.0) * 1000 / i128::from(self.speed.max(1))) as i64)
    }

    /// The source instant of the first rendered instant.
    pub fn src_from(&self) -> Micros {
        self.src_in + self.to_src(self.skip)
    }

    /// The source instant at the end of the clip.
    pub fn src_to(&self) -> Micros {
        self.src_in + self.to_src(self.len)
    }

    /// The source time the rendered part spans.
    pub fn rendered_src(&self) -> Micros {
        self.to_src(self.rendered())
    }

    /// Length actually rendered.
    pub fn rendered(&self) -> Micros {
        self.len - self.skip
    }

    /// How long before the first rendered instant the source frame that holds it starts
    /// (REQ-155): zero for a clip drawn from its start, for a 360 clip and for a source
    /// without a known frame rate. The picture is read from that frame, so a paused still
    /// in the last frame of a clip, or playback started inside a frame, has a picture.
    pub fn lead(&self) -> Micros {
        self.to_timeline(self.lead_src())
    }

    /// [`Self::lead`] in source time.
    pub fn lead_src(&self) -> Micros {
        let (Some(f), None) = (self.grid, &self.sphere) else {
            return Micros::ZERO;
        };
        if self.skip <= Micros::ZERO {
            return Micros::ZERO;
        }
        let t = self.src_from().0 as i128;
        let per = 1_000_000i128 * f.den as i128;
        let n = t * f.num as i128 / per;
        let first = (n * per + f.num as i128 / 2) / f.num as i128;
        Micros((t - first).max(0) as i64)
    }

    /// How much earlier than the first rendered instant the picture is read: the lead plus
    /// a millisecond so that rounding cannot drop the frame again.
    pub(super) fn read_back(&self) -> Micros {
        match self.lead_src() {
            Micros(0) => Micros::ZERO,
            l => l + Micros(1_000),
        }
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
                clip: c.id,
                overlay: Vec::new(),
                lut_clip,
                media: m.id,
                path: src.path_of(m),
                has_audio: m.format.has_audio(),
                src_in: c.src_in,
                len: c.len(),
                skip: (from - start).max(Micros::ZERO),
                fade_in,
                fade_out,
                speed: c.speed,
                gain: crate::music::clip_gain(c),
                clean: c.clean_audio,
                clean_strength: c.clean_strength,
                speech: c.speech_enhance,
                volume_keys: tl.volume_keys(c.id).to_vec(),
                crop: c.crop,
                to_black: tl.end_fade() && i + 1 == tl.clips.len() && !tl.has_fade(i),
                sphere: m
                    .format
                    .is_360()
                    .then(|| start_path(m, project.camera.path_of(Some(c.id.0), m.id))),
                anchored: m
                    .format
                    .is_360()
                    .then(|| anchored_doc(project, m, start, end, from))
                    .flatten(),
                grid: m.format.fps,
                spatial: spatial_job(m, c, project.camera.path_of(Some(c.id.0), m.id), src),
                picture_sub: project
                    .picture_sub
                    .filter(|(id, _)| *id == m.id && !m.format.is_360())
                    .map(|(_, ordinal)| ordinal),
                frame: {
                    let stack = tl.frame_stack(i);
                    let own = tl.own_aspect(c.id);
                    (tl.frame_aspect.is_some() || own.is_some() || !stack.is_empty()).then_some(
                        SegFrame {
                            aspect: own.or(tl.frame_aspect),
                            own: own.is_some(),
                            stack,
                        },
                    )
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
pub(super) fn anchored_doc(
    project: &Project,
    m: &MediaItem,
    start: Micros,
    end: Micros,
    from: Micros,
) -> Option<String> {
    let (w, h) = (m.format.width.max(2), m.format.height.max(1));
    let win = [(start, end)];
    let end_all = project.timeline.duration();
    let mut events: Vec<crate::ass::AssEvent> = Vec::new();
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
    events.iter().any(|e| e.end > shift).then(|| {
        ass::document_styled(
            &[],
            project.subtitles.preset,
            &project.subtitles.style,
            w,
            h,
            shift,
            &events,
        )
    })
}

/// True when the clip a line is shown in is a 360 clip.
pub(super) fn line_on_sphere(project: &Project, l: &crate::subtitles::ShownLine) -> bool {
    project
        .timeline
        .get(l.clip)
        .and_then(|c| project.media(c.media))
        .is_some_and(|m| m.format.is_360())
}

/// The anchored subtitle lines shown on the 360 clip of `m` between `start` and `end`, as
/// texts (white) with the line's size (thousandths) so that they are drawn like texts fixed
/// to the sphere.
pub(super) fn anchored_line_items(
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
        .flat_map(|l| {
            let size = l.size.unwrap_or(crate::subtitles::SIZE_NORMAL);
            let preset = project.subtitles.preset;
            let make = |text: &str, from: Micros, to: Micros| {
                let mut t = crate::text::TextItem::new(
                    crate::text::TextId(0),
                    crate::text::clean_text(text).unwrap_or_else(|| " ".into()),
                    from,
                );
                t.ttl = (to - from).max(crate::text::MIN_TTL);
                t.anchor = l.anchor;
                // the style of the project's preset (REQ-257)
                t.readability = if preset == crate::subtitles::Preset::Boxed {
                    crate::text::Readability::Box
                } else {
                    crate::text::Readability::Outline
                };
                t
            };
            if preset != crate::subtitles::Preset::WordHighlight {
                return vec![(make(&l.text, l.start, l.end), size)];
            }
            // a few words at a time, the one being spoken turns yellow
            crate::ass::highlight_chunks(&l)
                .into_iter()
                .map(|(from, to, text, cs)| {
                    let mut t = make(&text, from, to);
                    t.karaoke = cs;
                    (t, size)
                })
                .collect()
        })
        .collect()
}

/// The timeline times of the flat (not 360) clips when the video has 360 clips; `None`
/// when it has none (texts fixed to the sphere are then shown at their flat place).
pub(super) fn flat_windows(project: &Project) -> Option<Vec<(Micros, Micros)>> {
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

/// The sound of a 360 clip with ambisonic sound, turned by its camera (REQ-273). A preview
/// leaves a very long clip as recorded: it waits for the pre-pass.
fn spatial_job(
    m: &crate::media::MediaItem,
    c: &crate::timeline::Clip,
    path: &[Pov],
    src: Source,
) -> Option<crate::spatial::SpatialJob> {
    if !crate::spatial::is_ambisonic(&m.format) {
        return None;
    }
    let len = c.src_len();
    if matches!(src, Source::Preview { .. }) && len > crate::spatial::PREVIEW_MAX {
        return None;
    }
    Some(crate::spatial::SpatialJob {
        source: m.path.clone(),
        src_in: c.src_in,
        len,
        path: start_path(m, path),
    })
}

/// The camera path of a 360 source with the file's own initial view as the first point
/// when the user has not chosen one at the start (REQ-269).
fn start_path(m: &crate::media::MediaItem, path: &[Pov]) -> Vec<Pov> {
    let mut v = path.to_vec();
    if m.format.pose.is_some() && v.first().is_none_or(|p| p.t > Micros::ZERO) {
        v.insert(
            0,
            Pov::new(Micros::ZERO, crate::camera::start_camera(m.format.pose)),
        );
    }
    v
}
