// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Markers of the timeline (pure): one per fade (a diamond at its clip boundary), one per
//! subtitle line (a flag at its start with a bar to its end), and an "Add fade" handle on
//! every boundary without a fade. They are computed from the project each frame, so they
//! follow every edit (ripple, cuts) and the zoom.

use crate::app::Instance;
use crate::project::Project;
use crate::time::{Micros, timecode};

/// How close (in pixels) the pointer must be to a marker to hit it.
pub const HIT_TOLERANCE: f32 = 4.0;
/// Characters of a line's text shown in its tooltip.
const TOOLTIP_TEXT: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Fade,
    Line,
    /// A boundary without a fade: clicking adds one.
    AddFade,
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
                    "Fade through black at {} — between \"{}\" and \"{}\". Click for its settings.",
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
    let (from, to) = visible;
    out.retain(|m| m.x_end + HIT_TOLERANCE >= from && m.x - HIT_TOLERANCE <= to);
    out
}

/// The markers under pointer position `x` (within `tol` pixels of their position).
pub fn hits(markers: &[Marker], x: f32, tol: f32) -> Vec<&Marker> {
    markers.iter().filter(|m| (m.x - x).abs() <= tol).collect()
}

/// What clicking a marker does: open the fade or line, or add the fade.
pub fn click_command(m: &Marker) -> crate::app::Command {
    use crate::app::Command;
    match (m.kind, m.instance) {
        (MarkerKind::AddFade, Instance::Fade(left, right)) => Command::SetFade {
            left,
            right,
            on: true,
        },
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
            assert!(fade.tooltip.contains("Fade through black"));
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
}
