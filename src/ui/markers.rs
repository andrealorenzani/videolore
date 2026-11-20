// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Markers of the timeline (pure): one per fade (a diamond at its clip boundary, at the
//! very start or at the very end), one per subtitle line (a flag at its start with a bar
//! to its end), one per clip whose sound level is changed, and an "Add fade" handle on
//! every boundary without a fade. They are computed from the project each frame, so they
//! follow every edit (ripple, cuts) and the zoom.

use crate::app::Instance;
use crate::project::Project;
use crate::time::{Micros, timecode};

/// How close (in pixels) the pointer must be to a marker to hit it.
pub const HIT_TOLERANCE: f32 = 4.0;
/// Pixels between the volume marker and the Clean audio marker of a clip.
pub const CLEAN_OFFSET: f32 = 12.0;
/// Pixels from a clip's start to its Scale marker, and to its Move marker.
pub const SCALE_OFFSET: f32 = 24.0;
pub const MOVE_OFFSET: f32 = 36.0;
/// Pixels from a clip's start to its Apply LUT marker.
pub const LUT_OFFSET: f32 = 48.0;
/// Characters of a line's text shown in its tooltip.
const TOOLTIP_TEXT: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Fade,
    /// Fade from black at the start or to black at the end of the video.
    EdgeFade,
    Line,
    /// A clip whose own sound is not at 100 % (or is muted).
    ClipVolume,
    /// A clip with Clean audio (REQ-114).
    CleanAudio,
    /// A clip with Scale (1.5.0).
    Scale,
    /// A clip with Move (1.5.0).
    Move,
    /// A clip with Apply LUT (1.6.0); the whole video's has no marker.
    Lut,
    /// A boundary without a fade: clicking adds one.
    AddFade,
    /// A point of view of a 360 clip.
    Pov,
}

/// One marker, with its x position for the current zoom.
#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    pub kind: MarkerKind,
    /// The fade (also for [`MarkerKind::AddFade`]: the pair of clips) or the line.
    pub instance: Instance,
    pub x: f32,
    /// End of a line's bar (= `x` for fades).
    pub x_end: f32,
    pub tooltip: String,
}

/// The icon drawn on a marker (REQ-142).
pub fn icon_of(kind: MarkerKind) -> &'static str {
    use super::icons as i;
    match kind {
        MarkerKind::Fade | MarkerKind::EdgeFade => i::CIRCLE_HALF,
        MarkerKind::Line => i::SUBTITLES,
        MarkerKind::ClipVolume => i::SPEAKER_HIGH,
        MarkerKind::CleanAudio => i::WAVEFORM,
        MarkerKind::Scale => i::ARROWS_OUT,
        MarkerKind::Move => i::ARROWS_OUT_CARDINAL,
        MarkerKind::Lut => i::PALETTE,
        MarkerKind::AddFade => i::PLUS,
        MarkerKind::Pov => i::VIDEO_CAMERA,
    }
}

/// Where the icon sits relative to the marker's x and the lane's middle (pixels).
pub fn icon_offset(kind: MarkerKind) -> (f32, f32) {
    match kind {
        MarkerKind::ClipVolume
        | MarkerKind::CleanAudio
        | MarkerKind::Scale
        | MarkerKind::Move
        | MarkerKind::Lut => (5.0, 0.0),
        MarkerKind::Line => (10.0, -4.0),
        _ => (0.0, 0.0),
    }
}

fn short(text: &str) -> String {
    if text.chars().count() <= TOOLTIP_TEXT {
        text.to_string()
    } else {
        let mut s: String = text.chars().take(TOOLTIP_TEXT).collect();
        s.push('…');
        s
    }
}

/// Markers of `project` at zoom `z` (pixels per second), time 0 at `x0`. Only markers
/// whose extent touches `visible` (x range) are returned.
pub fn layout(project: &Project, z: f32, x0: f32, visible: (f32, f32)) -> Vec<Marker> {
    let tl = &project.timeline;
    let x_of = |t: Micros| x0 + (t.as_secs_f64() * f64::from(z)) as f32;
    let name = |i: usize| {
        project
            .media(tl.clips[i].media)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "?".into())
    };
    let mut out = Vec::new();
    let mut at = Micros::ZERO;
    for b in 0..tl.clips.len().saturating_sub(1) {
        at = at + tl.clips[b].len();
        let (l, r) = (tl.clips[b].id, tl.clips[b + 1].id);
        let x = x_of(at);
        let (kind, tooltip) = if tl.has_fade(b) {
            (
                MarkerKind::Fade,
                format!(
                    "Fade IN/OUT at {} — between \"{}\" and \"{}\". Click for its settings.",
                    timecode(at),
                    name(b),
                    name(b + 1)
                ),
            )
        } else {
            (
                MarkerKind::AddFade,
                format!(
                    "Add fade at {} — between \"{}\" and \"{}\"",
                    timecode(at),
                    name(b),
                    name(b + 1)
                ),
            )
        };
        out.push(Marker {
            kind,
            instance: Instance::Fade(l, r),
            x,
            x_end: x,
            tooltip,
        });
    }
    if !tl.clips.is_empty() {
        let end = tl.duration();
        if tl.start_fade() {
            out.push(Marker {
                kind: MarkerKind::EdgeFade,
                instance: Instance::FadeStart,
                x: x_of(Micros::ZERO),
                x_end: x_of(Micros::ZERO),
                tooltip: "Fade from black at the start of the video. Click for its settings."
                    .into(),
            });
        }
        if tl.end_fade() {
            out.push(Marker {
                kind: MarkerKind::EdgeFade,
                instance: Instance::FadeEnd,
                x: x_of(end),
                x_end: x_of(end),
                tooltip: format!(
                    "Fade to black at the end of the video ({}). Click for its settings.",
                    timecode(end)
                ),
            });
        }
    }
    let mut start = Micros::ZERO;
    for (i, c) in tl.clips.iter().enumerate() {
        if c.has_volume() {
            let level = if c.muted {
                "muted".to_string()
            } else {
                format!("{}%", c.volume)
            };
            out.push(Marker {
                kind: MarkerKind::ClipVolume,
                instance: Instance::ClipVolume(c.id),
                x: x_of(start),
                x_end: x_of(start),
                tooltip: format!(
                    "Volume of \"{}\": {level}. Click for its settings.",
                    name(i)
                ),
            });
        }
        if c.clean_audio {
            // beside the volume marker, so both stay clickable
            let x = x_of(start) + CLEAN_OFFSET;
            out.push(Marker {
                kind: MarkerKind::CleanAudio,
                instance: Instance::CleanAudio(c.id),
                x,
                x_end: x,
                tooltip: format!("Clean audio on \"{}\". Click for its settings.", name(i)),
            });
        }
        let keys = |n: usize, what: &str| {
            format!(
                "{what} on \"{}\": {n} keyframe{}. Click for its settings.",
                name(i),
                if n == 1 { "" } else { "s" }
            )
        };
        let (scale, moved) = (
            tl.scale_keys(crate::timeline::FrameTarget::Clip(c.id))
                .len(),
            tl.move_keys(crate::timeline::FrameTarget::Clip(c.id)).len(),
        );
        if scale > 0 {
            let x = x_of(start) + SCALE_OFFSET;
            out.push(Marker {
                kind: MarkerKind::Scale,
                instance: Instance::Scale(crate::timeline::FrameTarget::Clip(c.id)),
                x,
                x_end: x,
                tooltip: keys(scale, "Scale"),
            });
        }
        if moved > 0 {
            let x = x_of(start) + MOVE_OFFSET;
            out.push(Marker {
                kind: MarkerKind::Move,
                instance: Instance::Move(crate::timeline::FrameTarget::Clip(c.id)),
                x,
                x_end: x,
                tooltip: keys(moved, "Move"),
            });
        }
        if let Some(path) = tl.lut_of(crate::timeline::FrameTarget::Clip(c.id)) {
            let x = x_of(start) + LUT_OFFSET;
            out.push(Marker {
                kind: MarkerKind::Lut,
                instance: Instance::Lut(crate::timeline::FrameTarget::Clip(c.id)),
                x,
                x_end: x,
                tooltip: format!(
                    "Apply LUT on \"{}\": {}. Click for its settings.",
                    name(i),
                    crate::lut::display_name(path)
                ),
            });
        }
        start = start + c.len();
    }
    for l in project.subtitles.project(tl) {
        out.push(Marker {
            kind: MarkerKind::Line,
            instance: Instance::Line(l.line, l.clip),
            x: x_of(l.start),
            x_end: x_of(l.end),
            tooltip: format!(
                "Subtitle at {}–{}: \"{}\". Click for its settings.",
                timecode(l.start),
                timecode(l.end),
                short(&l.text)
            ),
        });
    }
    // points of view inside the kept part of each 360 clip, at the clip's position
    let mut start = Micros::ZERO;
    for c in &tl.clips {
        let sphere = project.media(c.media).is_some_and(|m| m.format.is_360());
        if sphere {
            for p in project.camera.path(c.media) {
                if p.t >= c.src_in && p.t < c.src_out {
                    let at = start + (p.t - c.src_in);
                    out.push(Marker {
                        kind: MarkerKind::Pov,
                        instance: Instance::Pov(c.media, p.t),
                        x: x_of(at),
                        x_end: x_of(at),
                        tooltip: format!(
                            "POV at {}: yaw {:.1}°, pitch {:.1}°, zoom {:.0}°. Click for its settings.",
                            timecode(at),
                            p.cam.yaw_deg(),
                            p.cam.pitch_deg(),
                            p.cam.fov_deg()
                        ),
                    });
                }
            }
        }
        start = start + c.len();
    }
    let (from, to) = visible;
    out.retain(|m| m.x_end + HIT_TOLERANCE >= from && m.x - HIT_TOLERANCE <= to);
    out
}

/// The markers under pointer position `x` (within `tol` pixels of their position).
pub fn hits(markers: &[Marker], x: f32, tol: f32) -> Vec<&Marker> {
    markers.iter().filter(|m| (m.x - x).abs() <= tol).collect()
}

/// What clicking a marker does: open the fade, line or clip volume, or add the fade.
pub fn click_command(m: &Marker) -> crate::app::Command {
    click_command_with(m, false)
}

/// [`click_command`] with Ctrl/Cmd held (`toggle`): a line goes in or out of the
/// selection instead.
pub fn click_command_with(m: &Marker, toggle: bool) -> crate::app::Command {
    use crate::app::Command;
    match (m.kind, m.instance) {
        (MarkerKind::AddFade, Instance::Fade(left, right)) => Command::SetFade {
            left,
            right,
            on: true,
        },
        (MarkerKind::Line, Instance::Line(id, _)) if toggle => Command::ToggleLine(id),
        (_, i) => Command::OpenInstance(i),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Command;
    use crate::subtitles::tests::setup;

    fn project() -> Project {
        // one 10 s clip split at 5 s: a line in each half
        let (mut timeline, subtitles) = setup();
        timeline.split(Micros(5_000_000), Micros(33_333)).unwrap();
        Project {
            timeline,
            subtitles,
            ..Default::default()
        }
    }

    const ALL: (f32, f32) = (f32::NEG_INFINITY, f32::INFINITY);

    // covers: REQ-077
    #[test]
    fn markers_sit_at_their_instance_at_every_zoom() {
        let mut p = project();
        assert!(p.timeline.clips.len() >= 2, "{:?}", p.timeline);
        p.timeline.toggle_fade(0).unwrap();
        let shown = p.subtitles.project(&p.timeline);
        assert!(!shown.is_empty());
        let fade_at = p.timeline.start_of(1);
        // minimum (fit 10 s in 300 px), middle and maximum (12 px per frame at 30 fps)
        for z in [30.0f32, 95.0, 360.0] {
            let x0 = 17.0;
            let m = layout(&p, z, x0, ALL);
            let fade = m.iter().find(|m| m.kind == MarkerKind::Fade).unwrap();
            let want = x0 + fade_at.as_secs_f64() as f32 * z;
            assert!((fade.x - want).abs() <= 1.0, "zoom {z}");
            for l in &shown {
                let mk = m
                    .iter()
                    .find(|m| m.instance == Instance::Line(l.line, l.clip))
                    .unwrap();
                assert!((mk.x - (x0 + l.start.as_secs_f64() as f32 * z)).abs() <= 1.0);
                assert!((mk.x_end - (x0 + l.end.as_secs_f64() as f32 * z)).abs() <= 1.0);
                assert!(mk.tooltip.contains("Subtitle") && mk.tooltip.contains(&short(&l.text)));
            }
            assert!(fade.tooltip.contains("Fade IN/OUT"));
        }
    }

    // covers: REQ-077
    #[test]
    fn add_fade_handles_only_where_there_is_no_fade() {
        let mut p = project();
        let n = p.timeline.clips.len() - 1;
        let m = layout(&p, 50.0, 0.0, ALL);
        assert_eq!(
            m.iter().filter(|m| m.kind == MarkerKind::AddFade).count(),
            n
        );
        assert!(m.iter().all(|m| m.kind != MarkerKind::Fade));
        p.timeline.toggle_fade(0).unwrap();
        let m = layout(&p, 50.0, 0.0, ALL);
        assert_eq!(
            m.iter().filter(|m| m.kind == MarkerKind::AddFade).count(),
            n - 1
        );
        let fade = m.iter().find(|m| m.kind == MarkerKind::Fade).unwrap();
        let add = m.iter().find(|m| m.kind == MarkerKind::AddFade);
        assert!(matches!(click_command(fade), Command::OpenInstance(_)));
        if let Some(add) = add {
            assert!(matches!(
                click_command(add),
                Command::SetFade { on: true, .. }
            ));
            assert!(add.tooltip.starts_with("Add fade"));
        }
    }

    // covers: REQ-077
    #[test]
    fn cutting_a_clip_removes_its_line_markers() {
        let mut p = project();
        let shown = p.subtitles.project(&p.timeline);
        let clip = shown[0].clip;
        let before = layout(&p, 50.0, 0.0, ALL);
        assert!(
            before
                .iter()
                .any(|m| matches!(m.instance, Instance::Line(_, c) if c == clip))
        );
        p.timeline.remove(clip).unwrap();
        let after = layout(&p, 50.0, 0.0, ALL);
        assert!(
            !after
                .iter()
                .any(|m| matches!(m.instance, Instance::Line(_, c) if c == clip))
        );
    }

    // covers: REQ-077
    #[test]
    fn overlapping_markers_are_all_hit_and_far_ones_culled() {
        let mut p = project();
        p.timeline.toggle_fade(0).unwrap();
        // zoomed far out every marker is within a few pixels
        let m = layout(&p, 0.01, 0.0, ALL);
        let hit = hits(&m, 0.0, HIT_TOLERANCE);
        assert_eq!(hit.len(), m.len());
        assert!(hit.len() >= 2);
        // nothing far from every marker
        assert!(hits(&m, 1000.0, HIT_TOLERANCE).is_empty());
        // culling to the visible range
        let wide = layout(&p, 100.0, 0.0, ALL);
        let visible = layout(&p, 100.0, 0.0, (0.0, 50.0));
        assert!(visible.len() < wide.len());
        assert!(visible.iter().all(|m| m.x - HIT_TOLERANCE <= 50.0));
        assert_eq!(short(&"a".repeat(50)).chars().count(), TOOLTIP_TEXT + 1);
    }

    // covers: REQ-114, REQ-077
    #[test]
    fn a_clip_with_clean_audio_has_a_marker_beside_its_volume_marker() {
        let mut p = project();
        let first = p.timeline.clips[0].id;
        assert!(
            layout(&p, 10.0, 0.0, ALL)
                .iter()
                .all(|m| m.kind != MarkerKind::CleanAudio)
        );
        p.timeline.set_clean_audio(&[first], true).unwrap();
        p.timeline.set_clip_volume(first, 40, false).unwrap();
        let m = layout(&p, 10.0, 0.0, ALL);
        let c = m
            .iter()
            .find(|m| m.kind == MarkerKind::CleanAudio)
            .expect("a marker at the start of the clip");
        assert_eq!(c.instance, Instance::CleanAudio(first));
        let v = m.iter().find(|m| m.kind == MarkerKind::ClipVolume).unwrap();
        assert!(
            (c.x - v.x - CLEAN_OFFSET).abs() < 0.01,
            "both can be clicked"
        );
        assert!(c.tooltip.contains("Clean audio"));
    }

    // covers: REQ-077, REQ-083, REQ-084, REQ-090
    #[test]
    fn edge_fades_clip_volumes_and_line_toggles_have_markers() {
        let mut p = project();
        p.timeline.set_edge_fades(true).unwrap();
        let first = p.timeline.clips[0].id;
        p.timeline.set_clip_volume(first, 40, false).unwrap();
        let end_x = p.timeline.duration().as_secs_f64() as f32 * 10.0;
        let m = layout(&p, 10.0, 0.0, ALL);
        let edges: Vec<&Marker> = m
            .iter()
            .filter(|m| m.kind == MarkerKind::EdgeFade)
            .collect();
        assert_eq!(edges.len(), 2);
        assert_eq!((edges[0].instance, edges[0].x), (Instance::FadeStart, 0.0));
        assert_eq!(edges[1].instance, Instance::FadeEnd);
        assert!((edges[1].x - end_x).abs() < 0.5);
        let vol = m.iter().find(|m| m.kind == MarkerKind::ClipVolume).unwrap();
        assert_eq!(vol.instance, Instance::ClipVolume(first));
        assert!(vol.tooltip.contains("40%"));
        assert_eq!(
            click_command(vol),
            Command::OpenInstance(Instance::ClipVolume(first))
        );
        // a clip at 100 % has no marker; muted shows "muted"
        p.timeline.set_clip_volume(first, 100, true).unwrap();
        let m = layout(&p, 10.0, 0.0, ALL);
        assert!(m.iter().any(|m| m.tooltip.contains("muted")));
        p.timeline.set_clip_volume(first, 100, false).unwrap();
        let m = layout(&p, 10.0, 0.0, ALL);
        assert!(m.iter().all(|m| m.kind != MarkerKind::ClipVolume));
        // Ctrl/Cmd+click on a line toggles it; a plain click opens it
        let line = m.iter().find(|m| m.kind == MarkerKind::Line).unwrap();
        let Instance::Line(id, _) = line.instance else {
            panic!()
        };
        assert_eq!(click_command_with(line, true), Command::ToggleLine(id));
        assert_eq!(
            click_command_with(line, false),
            Command::OpenInstance(line.instance)
        );
    }
}
