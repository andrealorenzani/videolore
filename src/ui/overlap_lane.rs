// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The shared lane of the overlaps on the timeline (2.3.0, REQ-325, ADR-242): one lane for the
//! whole timeline, a bar for each overlap with its two ends dragged; overlaps that overlap in
//! time are stacked in sub-rows, at most three are visible and the lane scrolls for more. The
//! layout is pure ([`bars`], [`visible_rows`]); the drawing and the drags follow the Text lane.

use super::UiCache;
use super::timeline::BAR_GRAB;
use crate::app::{AppState, Command};
use crate::overlap::{VISIBLE_ROWS, sub_rows};
use crate::time::Micros;
use crate::timeline::ClipId;
use eframe::egui::{
    self, Align2, Color32, FontId, Id, Painter, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

/// Height of one sub-row of the lane.
pub const ROW_H: f32 = 12.0;
/// Height of the lane with its three visible sub-rows (and a pixel of margin on each side).
pub const LANE_HEIGHT: f32 = ROW_H * VISIBLE_ROWS as f32 + 2.0;

/// The height of the lane for `rows` sub-rows: none with no overlap, one to three rows high,
/// never more than [`LANE_HEIGHT`] (the cap of ADR-242).
pub fn lane_height(rows: usize) -> f32 {
    if rows == 0 {
        0.0
    } else {
        ROW_H * rows.min(VISIBLE_ROWS) as f32 + 2.0
    }
}

const OVERLAP_COLOR: Color32 = Color32::from_rgb(90, 120, 190);

/// One bar of the lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bar {
    pub id: u64,
    pub clip: ClipId,
    /// Timeline start and end.
    pub start: Micros,
    pub end: Micros,
    /// Offset from the clip's start and the clip's length (to clamp a drag).
    pub from: Micros,
    pub clip_len: Micros,
    pub name: String,
    /// The sub-row of the bar.
    pub row: usize,
}

/// Every overlap of the project as a bar with its sub-row (ADR-242).
pub fn bars(app: &AppState) -> Vec<Bar> {
    let tl = &app.project.timeline;
    let mut out = Vec::new();
    let mut at = Micros::ZERO;
    for c in &tl.clips {
        for o in tl.overlaps_of(c.id) {
            out.push(Bar {
                id: o.id,
                clip: c.id,
                start: at + o.from,
                end: at + o.end(),
                from: o.from,
                clip_len: c.len(),
                name: o.name(),
                row: 0,
            });
        }
        at = at + c.len();
    }
    let rows = sub_rows(&out.iter().map(|b| (b.start, b.end)).collect::<Vec<_>>());
    for (b, r) in out.iter_mut().zip(rows) {
        b.row = r;
    }
    out
}

/// The sub-rows shown when there are `total` of them and the lane is scrolled to `scroll`: the
/// first one and how many (at most [`VISIBLE_ROWS`]); a scroll past the end is brought back.
pub fn visible_rows(total: usize, scroll: usize) -> (usize, usize) {
    let count = total.min(VISIBLE_ROWS);
    let first = scroll.min(total.saturating_sub(VISIBLE_ROWS));
    (first, count)
}

/// How many sub-rows the bars need.
pub fn row_count(bars: &[Bar]) -> usize {
    bars.iter().map(|b| b.row + 1).max().unwrap_or(0)
}

/// Draws the lane and turns its drags into commands: the body of a bar moves the overlap, an
/// end changes its start or its length, a click selects it (REQ-325). Every drag is one undo
/// step and is kept inside the clip.
pub fn overlap_track(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    painter: &Painter,
    (lane, visible): (Rect, Rect),
    (x0, z): (f32, f32),
    cmds: &mut Vec<Command>,
) {
    let px = |t: Micros| (t.as_secs_f64() * f64::from(z)) as f32;
    let all = bars(app);
    let total = row_count(&all);
    let (first, count) = visible_rows(total, cache.overlap_scroll);
    cache.overlap_scroll = first;
    // the wheel over the lane moves the shown sub-rows when there are more than three
    if total > VISIBLE_ROWS && ui.rect_contains_pointer(lane) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy != 0.0 {
            let step = if dy > 0.0 { -1 } else { 1 };
            cache.overlap_scroll = visible_rows(total, (first as i64 + step).max(0) as usize).0;
        }
    }
    if total > VISIBLE_ROWS {
        painter.text(
            pos2(lane.max.x.min(visible.max.x) - 4.0, lane.min.y + 1.0),
            Align2::RIGHT_TOP,
            format!("{}-{} of {total}", first + 1, first + count),
            FontId::proportional(9.0),
            Color32::GRAY,
        );
    }
    for b in &all {
        if b.row < first || b.row >= first + count {
            continue;
        }
        let y = lane.min.y + 1.0 + (b.row - first) as f32 * ROW_H;
        let r = Rect::from_min_size(
            pos2(x0 + px(b.start), y),
            vec2(px(b.end - b.start).max(4.0), ROW_H - 1.0),
        );
        let selected = app.selected_overlap == Some(b.id);
        let resp = ui
            .interact(
                r,
                Id::unique(("videolore-overlap", b.id)),
                Sense::click_and_drag(),
            )
            .on_hover_text(format!(
                "{}\nClick to select, drag to move, drag an end to change the time it stays",
                b.name
            ));
        let grab = BAR_GRAB.min(r.width() / 3.0);
        let left = ui.interact(
            Rect::from_min_size(r.min, vec2(grab, r.height())),
            Id::unique(("videolore-overlap-left", b.id)),
            Sense::drag(),
        );
        let right = ui.interact(
            Rect::from_min_max(pos2(r.max.x - grab, r.min.y), r.max),
            Id::unique(("videolore-overlap-right", b.id)),
            Sense::drag(),
        );
        if left.hovered() || right.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if r.intersects(visible) {
            painter.rect_filled(r, 2.0, OVERLAP_COLOR);
            if selected {
                painter.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(2.0_f32, Color32::YELLOW),
                    StrokeKind::Inside,
                );
            }
            painter.text(
                r.min + vec2(4.0, 0.0),
                Align2::LEFT_TOP,
                &b.name,
                FontId::proportional(9.0),
                Color32::WHITE,
            );
        }
        if resp.clicked() {
            cmds.push(Command::SelectClip(Some(b.clip)));
            cmds.push(Command::SelectOverlap(Some(b.id)));
        }
        for (mode, r) in [(0, &resp), (1, &left), (2, &right)] {
            // the pointer's start is gone when the button is released: keep the last delta
            let drag_id = r.id.with("dx");
            if let Some(d) = r.total_drag_delta() {
                ui.data_mut(|data| data.insert_temp(drag_id, d.x));
            }
            if r.drag_stopped() {
                let dx: f32 = ui.data_mut(|data| data.remove_temp(drag_id)).unwrap_or(0.0);
                if dx != 0.0 {
                    let delta = Micros::from_secs_f64(f64::from(dx / z));
                    let (from, len) = dragged(b, mode, delta);
                    cmds.push(Command::SetOverlapTimes {
                        id: b.id,
                        from,
                        len,
                    });
                }
            }
        }
    }
}

/// The offset and the length of a bar dragged by `delta` (`0` the body, `1` the start, `2` the
/// end), kept inside the clip and at least the shortest overlap long (REQ-325).
pub fn dragged(b: &Bar, mode: u8, delta: Micros) -> (Micros, Micros) {
    let len = b.end - b.start;
    let (from, end) = match mode {
        0 => (b.from + delta, b.from + delta + len),
        1 => (b.from + delta, b.from + len),
        _ => (b.from, b.from + len + delta),
    };
    let o = crate::overlap::drag_ends(
        &crate::overlap::Overlap {
            id: b.id,
            path: String::new(),
            kind: crate::overlap::Kind::Still,
            from: b.from,
            len,
            place: crate::base::pos::Pos { x: 0, y: 0 },
            width: 1,
            height: 1,
            reshaped: false,
            src_w: 1,
            src_h: 1,
        },
        from,
        end,
        b.clip_len,
    );
    if mode == 0 {
        // moving the body keeps the length: the start is clamped, the end follows it
        let from = from.clamp(Micros::ZERO, (b.clip_len - len).max(Micros::ZERO));
        return (from, len);
    }
    (o.from, o.len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(from: i64, len: i64, clip_len: i64) -> Bar {
        let s = |v: i64| Micros(v * 1_000_000);
        Bar {
            id: 1,
            clip: ClipId(1),
            start: s(from),
            end: s(from + len),
            from: s(from),
            clip_len: s(clip_len),
            name: "a".into(),
            row: 0,
        }
    }

    // covers: REQ-325
    #[test]
    fn at_most_three_sub_rows_are_visible_and_the_scroll_reaches_the_others() {
        assert_eq!(visible_rows(0, 0), (0, 0));
        assert_eq!(visible_rows(2, 5), (0, 2));
        assert_eq!(visible_rows(3, 0), (0, 3));
        assert_eq!(visible_rows(5, 0), (0, 3));
        assert_eq!(visible_rows(5, 1), (1, 3));
        // past the end: the last three
        assert_eq!(visible_rows(5, 9), (2, 3));
        assert_eq!(LANE_HEIGHT, ROW_H * 3.0 + 2.0);
        // no overlap: no lane; the lane grows to three rows and stops there
        assert_eq!(lane_height(0), 0.0);
        assert_eq!(lane_height(1), ROW_H + 2.0);
        assert_eq!(lane_height(3), LANE_HEIGHT);
        assert_eq!(lane_height(40), LANE_HEIGHT);
    }

    // covers: REQ-325
    #[test]
    fn dragging_a_bar_moves_it_or_its_ends_inside_the_clip() {
        let s = |v: i64| Micros(v * 1_000_000);
        let b = bar(2, 3, 8);
        // the body keeps the length and stops at the clip's end and at its start
        assert_eq!(dragged(&b, 0, s(1)), (s(3), s(3)));
        assert_eq!(dragged(&b, 0, s(20)), (s(5), s(3)));
        assert_eq!(dragged(&b, 0, s(-20)), (Micros::ZERO, s(3)));
        // the end to 5 s keeps the start; past the clip it stops at the clip's end
        assert_eq!(dragged(&b, 2, s(0)), (s(2), s(3)));
        assert_eq!(dragged(&b, 2, s(20)), (s(2), s(6)));
        // the start dragged before the clip's start stops at the start
        assert_eq!(dragged(&b, 1, s(-20)), (Micros::ZERO, s(5)));
        // the ends crossing leave the shortest overlap
        assert_eq!(dragged(&b, 1, s(20)).1, crate::overlap::MIN_LEN);
    }
}
