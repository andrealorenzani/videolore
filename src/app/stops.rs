// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The keys [ and ] (REQ-194): the playhead goes to the previous or the next timeline
//! marker (fades, subtitle lines, text items, clip markers) and selects it.

use super::*;
use crate::text::TextId;

/// What a marker selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// A fade: the clip that starts at its boundary (the first clip for the video's start
    /// fade, the last one for its end fade).
    Fade(ClipId),
    Line(LineId),
    Text(TextId),
    /// A clip with its own volume, Clean audio, Scale, Move or Apply LUT.
    Clip(ClipId),
}

/// Every marker of the timeline with its time, in time order.
pub fn stops(project: &Project) -> Vec<(Micros, Stop)> {
    let tl = &project.timeline;
    let mut out: Vec<(Micros, Stop)> = Vec::new();
    for (i, c) in tl.clips.iter().enumerate() {
        let at = tl.start_of(i);
        if i > 0 && tl.has_fade(i - 1) {
            out.push((at, Stop::Fade(c.id)));
        }
        let own = c.has_volume()
            || c.clean_audio
            || tl.frames_of(c.id).is_some()
            || tl.lut_of(FrameTarget::Clip(c.id)).is_some();
        if own {
            out.push((at, Stop::Clip(c.id)));
        }
    }
    if let (Some(first), Some(last)) = (tl.clips.first(), tl.clips.last()) {
        if tl.start_fade() {
            out.push((Micros::ZERO, Stop::Fade(first.id)));
        }
        if tl.end_fade() {
            out.push((tl.duration(), Stop::Fade(last.id)));
        }
    }
    out.extend(
        project
            .subtitles
            .project(tl)
            .iter()
            .map(|l| (l.start, Stop::Line(l.line))),
    );
    out.extend(
        project
            .texts
            .items
            .iter()
            .map(|t| (t.start, Stop::Text(t.id))),
    );
    out.sort_by_key(|(t, _)| *t);
    out
}

impl AppState {
    /// Moves the playhead to the previous or the next marker and selects it (REQ-194).
    pub(super) fn step_marker(&mut self, forward: bool, now: Instant) -> Result<(), String> {
        let fps = self.fps();
        let here = self.playhead;
        let all = stops(&self.project);
        // markers sit where a seek puts the playhead, so a marker is never found twice
        let key = |t: Micros| fps.snap(t.clamp(Micros::ZERO, self.project.timeline.duration()));
        let found = if forward {
            all.iter().find(|(t, _)| key(*t) > here)
        } else {
            all.iter().rev().find(|(t, _)| key(*t) < here)
        };
        let Some((t, stop)) = found.copied() else {
            return Ok(());
        };
        self.selected_clip = None;
        self.selection.clear();
        match stop {
            Stop::Line(id) => self.selection.select_only(ObjRef::Line(id)),
            Stop::Text(id) => self.selection.select_only(ObjRef::Text(id)),
            Stop::Fade(id) | Stop::Clip(id) => self.selected_clip = Some(id),
        }
        self.seek(t, now);
        Ok(())
    }
}
