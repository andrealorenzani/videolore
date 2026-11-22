// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The overlay stage (REQ-304, REQ-305): a clip whose Applied effects order was changed by the
//! user is drawn row by row, bottom to top, after the fades and the frame composition: the
//! LUT (the whole frame is graded), each text, the picture items and the subtitle lines, each as
//! one filter step in the clip's own time. A clip with the order of the older versions has no
//! step: its LUT stays in the clip chain and its texts, lines and pictures are drawn after the
//! concat, so the arguments of an old project do not change.

use super::steps::Step;
use super::*;
use crate::effect_order::Row;
use crate::text::Window;
use crate::timeline::ClipId;
use std::sync::Arc;

/// One thing drawn over a clip.
#[derive(Debug, Clone, PartialEq)]
pub enum OverlayStep {
    /// Apply LUT on the whole frame.
    Lut(Arc<crate::lut::Lut>),
    /// An ASS document (one text item, or the subtitle lines) burned in: the file name in the
    /// working folder and its content, in the clip's own time.
    Ass { name: String, doc: String },
    /// A picture item over the frame, in the clip's own time.
    Pic(PicOverlay),
}

/// The clips drawn by the overlay stage, with their rows top first.
pub fn changed_clips(project: &Project) -> Vec<(crate::project::ClipRows, Vec<Row>)> {
    project.changed_orders()
}

/// The parts of the timeline that the overlay stage does not draw (the clips with the
/// order of the older versions, neighbours joined); `None` when no clip is changed, so the
/// whole timeline is drawn the old way.
pub fn legacy_windows(project: &Project) -> Option<Vec<Window>> {
    let changed = changed_clips(project);
    if changed.is_empty() {
        return None;
    }
    let mut out: Vec<Window> = Vec::new();
    for r in project.clip_rows() {
        if changed.iter().any(|(c, _)| c.clip == r.clip) {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.1 == r.start => last.1 = r.end,
            _ => out.push((r.start, r.end)),
        }
    }
    Some(out)
}

/// The ids of the clips drawn by the overlay stage.
pub fn changed_ids(project: &Project) -> Vec<ClipId> {
    changed_clips(project)
        .into_iter()
        .map(|(c, _)| c.clip)
        .collect()
}

/// Gives the clips of `segs` whose order changed their overlay steps, for a picture of
/// geometry `g` rendered from the timeline position `from`. The LUT of such a clip is taken
/// out of its own chain (it is a step). With `steps` false (the background alone) only the
/// LUT is taken out. A picture file that is gone stops an export and is left out of a preview;
/// `burn_lines` false leaves the subtitle lines to their separate track.
pub fn attach(
    project: &Project,
    segs: &mut [Segment],
    g: &Geometry,
    from: Micros,
    src: Source,
    burn_lines: bool,
    steps: bool,
) -> Result<(), RenderError> {
    for (rows, order) in changed_clips(project) {
        let Some(seg) = segs.iter_mut().find(|s| s.clip == rows.clip) else {
            continue;
        };
        let lut = seg.lut_clip.take();
        if !steps {
            continue;
        }
        let shift = rows.start.max(from);
        // the documents of a clip built step by step are in the clip's own time (its chain
        // keeps the clock of the clip until its last filter); those of the overlay stage start
        // at the first rendered instant, after the chain has restarted its clock
        let by_steps = crate::effect_order::needs_steps(&order);
        let doc_shift = if by_steps { rows.start } else { shift };
        let win = [(rows.start, rows.end)];
        let end_all = project.timeline.duration();
        let mut out = Vec::new();
        let mut seq: Vec<Step> = Vec::new();
        let mut lines: Option<Vec<crate::subtitles::ShownLine>> = None;
        for row in order.iter().rev() {
            let before = out.len();
            match *row {
                Row::Crop if by_steps => seq.push(Step::Crop),
                Row::Frame if by_steps => seq.push(Step::Frame),
                Row::Scale if by_steps => seq.push(Step::Scale),
                Row::Move if by_steps => seq.push(Step::Move),
                Row::Fade if by_steps => seq.push(Step::Fade),
                Row::Lut => out.extend(lut.clone().map(OverlayStep::Lut)),
                Row::Text(id) => {
                    let Some(t) = project.texts.items.iter().find(|t| t.id.0 == id) else {
                        continue;
                    };
                    let events =
                        crate::text::item_events(t, g.width, g.height, end_all, Some(&win));
                    if events.iter().any(|e| e.end > shift) {
                        let doc = ass::document_styled(
                            &[],
                            project.subtitles.preset,
                            &project.subtitles.style,
                            g.width,
                            g.height,
                            doc_shift,
                            &events,
                        );
                        out.push(OverlayStep::Ass {
                            name: format!("ov{}-t{id}.ass", rows.clip.0),
                            doc,
                        });
                    }
                }
                Row::Picture(id) => {
                    let Some(t) = project.texts.items.iter().find(|t| t.id.0 == id) else {
                        continue;
                    };
                    let Some(pic) = &t.picture else { continue };
                    let (a, b) = (t.start.max(shift), t.end().min(rows.end).min(end_all));
                    if b <= a {
                        continue;
                    }
                    let path = PathBuf::from(&pic.path);
                    if !crate::picture::usable(&path) {
                        if matches!(src, Source::Preview { .. }) {
                            continue;
                        }
                        return Err(RenderError::Picture(pic.name()));
                    }
                    let (x, y, w, h) = pic.rect(t.place, t.width, t.height, (g.width, g.height));
                    out.push(OverlayStep::Pic(PicOverlay {
                        path,
                        x,
                        y,
                        w,
                        h,
                        start: a - doc_shift,
                        end: b - doc_shift,
                        pieces: Vec::new(),
                    }));
                }
                Row::Subtitles if burn_lines => {
                    let all = lines.get_or_insert_with(|| {
                        project
                            .subtitles
                            .project(&project.timeline)
                            .into_iter()
                            .filter(|l| l.clip == rows.clip)
                            .filter(|l| !(rows.sphere && l.anchor.is_some()))
                            .collect()
                    });
                    if all.iter().any(|l| l.end > shift) {
                        let doc = ass::document_styled(
                            all,
                            project.subtitles.preset,
                            &project.subtitles.style,
                            g.width,
                            g.height,
                            doc_shift,
                            &[],
                        );
                        out.push(OverlayStep::Ass {
                            name: format!("ov{}-s.ass", rows.clip.0),
                            doc,
                        });
                    }
                }
                Row::Subtitles
                | Row::Volume
                | Row::Clean
                | Row::Unknown
                | Row::Crop
                | Row::Frame
                | Row::Scale
                | Row::Move
                | Row::Fade => {}
            }
            // what a row drew is the step of its place (a row that drew nothing has none)
            if by_steps && out.len() > before {
                seq.push(Step::Over(before));
            }
        }
        seg.overlay = out;
        if by_steps {
            seg.steps = seq;
        }
    }
    Ok(())
}
