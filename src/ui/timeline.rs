// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Timeline section: header (zoom slider, Fit), then ruler, marker lane (fades,
//! subtitle lines, "Add fade" handles), video track and music track. The body scrolls both
//! ways when the tracks do not fit.

use super::markers::{self, HIT_TOLERANCE, MarkerKind};
use super::{UiCache, ruler, scroll_body, zoom};
use crate::app::{AppState, Axis, Command, Edge, EffectKind, Instance, Popup, Target};
use crate::time::Micros;
use crate::timeline::Timeline;
use eframe::egui::{
    self, Align2, Button, Color32, FontId, Id, Painter, Pos2, Rect, ScrollArea, Sense, Shape,
    Stroke, StrokeKind, Ui, pos2, text::LayoutJob, vec2,
};

const CLIP_COLORS: [Color32; 4] = [
    Color32::from_rgb(70, 110, 170),
    Color32::from_rgb(90, 140, 90),
    Color32::from_rgb(150, 100, 60),
    Color32::from_rgb(120, 80, 150),
];
const TEXT_COLOR: Color32 = Color32::from_rgb(170, 90, 140);
/// Height of the Text lane and the width of the grab zones at the ends of a bar.
pub const TEXT_H: f32 = 18.0;
pub const BAR_GRAB: f32 = 6.0;
const MUSIC_COLOR: Color32 = Color32::from_rgb(40, 130, 130);
const LINE_COLOR: Color32 = Color32::from_rgb(230, 200, 90);
/// Share of the track area used by the video track (the rest is music).
pub const VIDEO_SHARE: f32 = 0.62;
pub const RULER_H: f32 = 14.0;
pub const LANE_H: f32 = 16.0;
pub const VIDEO_MIN_H: f32 = 40.0;
pub const MUSIC_MIN_H: f32 = 24.0;
/// Space between the video and music tracks.
const TRACK_GAP: f32 = 4.0;

/// Length shown on the timeline: the video, or the music when it goes further.
pub fn content_duration(app: &AppState) -> Micros {
    app.project.timeline.duration().max(app.project.music.end())
}

/// Zoom range for a Timeline section `section_w` pixels wide.
pub fn zoom_range(app: &AppState, section_w: f32) -> (f32, f32) {
    zoom::range(
        content_duration(app).as_secs_f64(),
        section_w - 40.0,
        app.fps().as_f64(),
    )
}

/// Index a dragged clip should move to when dropped at time `t`.
pub fn drop_index(tl: &Timeline, t: Micros) -> usize {
    match tl.locate(t) {
        Some((i, start)) => {
            let half = Micros(tl.clips[i].len().0 / 2);
            if t - start < half { i } else { i + 1 }.min(tl.clips.len() - 1)
        }
        None => tl.clips.len().saturating_sub(1),
    }
}

/// Splits the area under the marker lane into the video and music tracks.
pub fn track_rects(area: Rect) -> (Rect, Rect) {
    let video_h = (area.height() * VIDEO_SHARE).max(VIDEO_MIN_H);
    let split = area.min.y + video_h;
    (
        Rect::from_min_max(area.min, pos2(area.max.x, split - TRACK_GAP / 2.0)),
        Rect::from_min_max(pos2(area.min.x, split + TRACK_GAP / 2.0), area.max),
    )
}

/// Splits the area left under the video track into the Text lane (between video and
/// music) and the music track.
pub fn text_lane(below_video: Rect) -> (Rect, Rect) {
    let top = below_video.min.y;
    (
        Rect::from_min_max(
            pos2(below_video.min.x, top),
            pos2(below_video.max.x, top + TEXT_H),
        ),
        Rect::from_min_max(
            pos2(below_video.min.x, top + TEXT_H + TRACK_GAP),
            below_video.max,
        ),
    )
}

/// Height of the timeline content: the visible height, or the sum of the minimum heights
/// when that is more (then it scrolls).
pub fn content_height(visible: f32) -> f32 {
    visible.max(RULER_H + LANE_H + VIDEO_MIN_H + TEXT_H + MUSIC_MIN_H + 2.0 * TRACK_GAP)
}

/// What a drag on the bar of a text item changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarDrag {
    /// The whole bar: the start moves, the time to live stays.
    Move,
    /// The left end: the start moves, the end stays.
    Left,
    /// The right end: the time to live changes.
    Right,
}

/// Start and time to live of a bar dragged by `delta` (REQ-110): never before 0 and never
/// shorter than the minimum time to live.
pub fn dragged(start: Micros, ttl: Micros, mode: BarDrag, delta: Micros) -> (Micros, Micros) {
    let min = crate::text::MIN_TTL;
    match mode {
        BarDrag::Move => ((start + delta).max(Micros::ZERO), ttl),
        BarDrag::Left => {
            let end = start + ttl;
            let s = (start + delta).clamp(Micros::ZERO, (end - min).max(Micros::ZERO));
            (s, end - s)
        }
        BarDrag::Right => (start, (ttl + delta).max(min)),
    }
}

/// Where the timeline was drawn this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// x of time 0.
    pub x0: f32,
    /// Pixels per second.
    pub zoom: f32,
    pub lane: Rect,
    pub video: Rect,
    pub text: Rect,
    pub music: Rect,
}

impl Geometry {
    pub fn x_of(&self, t: Micros) -> f32 {
        self.x0 + (t.as_secs_f64() * f64::from(self.zoom)) as f32
    }
}

/// The zoom slider and "Fit" in the section header.
pub fn header(ui: &mut Ui, app: &AppState, range: (f32, f32), cmds: &mut Vec<Command>) {
    let mut s = zoom::to_slider(app.zoom, range);
    ui.spacing_mut().slider_width = (ui.available_width() - 60.0).clamp(30.0, 160.0);
    if ui
        .add(egui::Slider::new(&mut s, 0.0..=1.0).show_value(false))
        .on_hover_text("Zoom: from the whole project to single frames")
        .changed()
    {
        cmds.push(Command::SetZoom(zoom::from_slider(s, range)));
    }
    let tip = super::action_tooltip(ui.ctx(), app, super::actions::ActionId::ZoomFit);
    if ui.button("Fit").on_hover_text(tip).clicked() {
        cmds.push(Command::ZoomFit);
    }
}

/// A name painted on one row, shortened with "…" to `width`.
fn paint_name(painter: &Painter, at: Pos2, name: &str, width: f32, size: f32) {
    let mut job =
        LayoutJob::simple_singleline(name.to_string(), FontId::proportional(size), Color32::WHITE);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(0.0));
    let galley = painter.layout_job(job);
    painter.galley(at, galley, Color32::WHITE);
}

fn media_name(app: &AppState, id: crate::media::MediaId) -> String {
    app.project
        .media(id)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "?".into())
}

/// A menu entry greyed out when `cmd` cannot run; `before` runs first (selection).
fn entry(ui: &mut Ui, enabled: bool, label: &str, cmds: &mut Vec<Command>, run: Vec<Command>) {
    if ui.add_enabled(enabled, Button::new(label)).clicked() {
        cmds.extend(run);
        ui.close();
    }
}

pub fn timeline_panel(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    range: (f32, f32),
    cmds: &mut Vec<Command>,
) {
    let total = content_duration(app);
    let z = zoom::clamp(app.zoom, range);
    if app.zoom != z {
        cmds.push(Command::SetZoom(z));
    }
    let avail = ui.available_size();
    let content_w = (total.as_secs_f64() * f64::from(z)) as f32 + 40.0;
    let size = vec2(content_w.max(avail.x - 2.0), content_height(avail.y - 2.0));
    let area = ScrollArea::both()
        .id_salt("timeline-scroll")
        .auto_shrink(false);
    scroll_body(ui, cache, area, |ui, cache| {
        tracks(ui, app, cache, z, size, cmds);
    });
}

fn tracks(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    z: f32,
    size: egui::Vec2,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let (rect, bg) = ui.allocate_exact_size(size, Sense::click());
    let x0 = rect.min.x;
    let px = |t: Micros| (t.as_secs_f64() * f64::from(z)) as f32;
    let t_at = |x: f32| Micros::from_secs_f64(f64::from(((x - x0) / z).max(0.0)));
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_gray(30));
    let visible = ui.clip_rect();
    // ruler: labels follow the time at the right edge of the visible part (REQ-093)
    let ruler_font = FontId::monospace(9.0);
    let char_w = ui
        .painter()
        .layout_no_wrap("0".into(), ruler_font.clone(), Color32::GRAY)
        .size()
        .x;
    let t0 = f64::from(((visible.min.x - x0) / z).max(0.0));
    let t1 = f64::from(((visible.max.x - x0) / z).max(0.0)).min(f64::from(rect.width() / z));
    let (ticks, mode) = ruler::ticks(t0, t1, z, char_w);
    for s in ticks {
        let x = x0 + s as f32 * z;
        painter.line_segment(
            [pos2(x, rect.min.y), pos2(x, rect.min.y + 5.0)],
            Stroke::new(1.0_f32, Color32::GRAY),
        );
        painter.text(
            pos2(x + 2.0, rect.min.y),
            Align2::LEFT_TOP,
            ruler::label(s, mode),
            ruler_font.clone(),
            Color32::GRAY,
        );
    }
    let lane = Rect::from_min_size(
        pos2(rect.min.x, rect.min.y + RULER_H),
        vec2(rect.width(), LANE_H),
    );
    let tracks_area = Rect::from_min_max(pos2(rect.min.x, lane.max.y + 2.0), rect.max);
    let (video, below) = track_rects(tracks_area);
    let (text, music) = text_lane(below);
    cache.timeline = Some(Geometry {
        x0,
        zoom: z,
        lane,
        video,
        text,
        music,
    });
    painter.rect_filled(lane, 0.0, Color32::from_gray(24));
    painter.rect_filled(text, 0.0, Color32::from_gray(34));
    painter.text(
        pos2(rect.min.x + 2.0, text.min.y + 1.0),
        Align2::LEFT_TOP,
        "T",
        FontId::proportional(10.0),
        Color32::GRAY,
    );
    painter.rect_filled(music, 0.0, Color32::from_gray(38));
    painter.text(
        pos2(rect.min.x + 2.0, music.min.y + 1.0),
        Align2::LEFT_TOP,
        "♪",
        FontId::proportional(10.0),
        Color32::GRAY,
    );
    // empty timeline: click to move the playhead; right-click for Paste / Add line here
    if bg.clicked()
        && let Some(p) = bg.interact_pointer_pos()
    {
        cmds.push(Command::SelectClip(None));
        cmds.push(Command::SelectMusic(None));
        cmds.push(Command::ClearSelection);
        cmds.push(Command::Seek(t_at(p.x)));
    }
    // an effect dragged from the Effects list: only video clips accept it
    let effect_payload = egui::DragAndDrop::has_payload_of_type::<EffectKind>(ui.ctx());
    let mut over_clip = false;
    if bg.secondary_clicked()
        && let Some(p) = bg.interact_pointer_pos()
    {
        cache.context_time = Some(t_at(p.x));
    }
    let here = cache.context_time.unwrap_or(app.playhead);
    bg.context_menu(|ui| {
        entry(
            ui,
            app.clipboard.is_some(),
            "Paste",
            cmds,
            vec![Command::Seek(here), Command::Paste],
        );
        entry(
            ui,
            tl.locate(here).is_some(),
            "Add subtitle line here",
            cmds,
            vec![Command::Seek(here), Command::AddSubtitle],
        );
    });

    let mut start = Micros::ZERO;
    for (i, c) in tl.clips.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(x0 + px(start), video.min.y),
            vec2(px(c.len()), video.height()),
        );
        start = start + c.len();
        let name = media_name(app, c.media);
        let resp = ui
            .interact(
                r,
                Id::unique(("videolore-clip", c.id.0)),
                Sense::click_and_drag(),
            )
            .on_hover_text(format!(
                "{name}\nDouble-click for details, right-click for more"
            ));
        if resp
            .dnd_hover_payload::<EffectKind>()
            .is_some_and(|e| *e != EffectKind::ChangeAspect)
        {
            over_clip = true;
            ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
            painter.rect_stroke(
                r,
                3.0,
                Stroke::new(3.0_f32, Color32::LIGHT_GREEN),
                StrokeKind::Inside,
            );
        }
        // Change aspect is for the whole video: dropped on a clip it does nothing (REQ-130)
        if let Some(effect) = resp.dnd_release_payload::<EffectKind>()
            && *effect != EffectKind::ChangeAspect
        {
            cmds.push(Command::SelectClip(Some(c.id)));
            cmds.push(Command::ApplyEffect {
                effect: *effect,
                target: Target::Clip(c.id),
            });
        }
        if !r.intersects(visible) {
            continue;
        }
        let selected = app.selected_clip == Some(c.id);
        let color = CLIP_COLORS[(c.media.0 as usize) % CLIP_COLORS.len()];
        painter.rect_filled(r.shrink(1.0), 3.0, color);
        if selected {
            painter.rect_stroke(
                r,
                3.0,
                Stroke::new(2.0_f32, Color32::YELLOW),
                StrokeKind::Inside,
            );
        }
        paint_name(
            &painter,
            r.min + vec2(4.0, 4.0),
            &name,
            r.width() - 8.0,
            11.0,
        );
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            cmds.push(Command::SelectClip(Some(c.id)));
            cmds.push(Command::Seek(t_at(p.x)));
        }
        if resp.double_clicked() {
            cmds.push(Command::SelectClip(Some(c.id)));
            cmds.push(Command::OpenPopup(Popup::ClipDetails));
        }
        if resp.drag_stopped()
            && let Some(p) = resp.interact_pointer_pos()
        {
            let to = drop_index(tl, t_at(p.x));
            cmds.push(Command::SelectClip(Some(c.id)));
            cmds.push(Command::MoveClip { clip: c.id, to });
        }
        if resp.secondary_clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            cache.context_time = Some(t_at(p.x));
        }
        let at = cache.context_time.unwrap_or(app.playhead);
        resp.context_menu(|ui| clip_menu(ui, app, i, at, cmds));
        let (fin, fout) = tl.fades_of(i);
        let fade_color = Color32::from_black_alpha(160);
        if fin > Micros::ZERO {
            painter.add(Shape::convex_polygon(
                vec![
                    r.left_top(),
                    pos2(r.min.x + px(fin), r.min.y),
                    r.left_bottom(),
                ],
                fade_color,
                Stroke::NONE,
            ));
        }
        if fout > Micros::ZERO {
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(r.max.x - px(fout), r.min.y),
                    r.right_top(),
                    r.right_bottom(),
                ],
                fade_color,
                Stroke::NONE,
            ));
        }
    }
    if effect_payload && !over_clip && ui.rect_contains_pointer(rect) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::NoDrop);
    }
    marker_lane(ui, app, cache, &painter, lane, x0, z, visible, cmds);
    text_track(ui, app, &painter, text, x0, z, visible, cmds);
    music_track(ui, app, cache, &painter, music, x0, z, visible, cmds);
    range_select(ui, cache, &painter, rect, x0, z, cmds);
    let xp = x0 + px(app.playhead);
    painter.line_segment(
        [pos2(xp, rect.min.y), pos2(xp, rect.max.y)],
        Stroke::new(2.0_f32, Color32::RED),
    );
}

/// Right-click menu of video clip `i`; `at` is where the user clicked.
fn clip_menu(ui: &mut Ui, app: &AppState, i: usize, at: Micros, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let Some(c) = tl.clips.get(i) else { return };
    let select = Command::SelectClip(Some(c.id));
    let end = tl.start_of(i + 1);
    entry(
        ui,
        true,
        "Split here",
        cmds,
        vec![Command::Seek(at), Command::Split],
    );
    entry(ui, true, "Cut", cmds, vec![select.clone(), Command::Cut]);
    entry(ui, true, "Copy", cmds, vec![select.clone(), Command::Copy]);
    entry(
        ui,
        app.clipboard.is_some(),
        "Paste after",
        cmds,
        vec![Command::Seek(end), Command::Paste],
    );
    entry(
        ui,
        true,
        "Delete",
        cmds,
        vec![select.clone(), Command::Delete],
    );
    entry(
        ui,
        tl.can_merge(c.id),
        "Merge with next",
        cmds,
        vec![select.clone(), Command::Merge],
    );
    ui.separator();
    if i > 0 {
        let on = tl.has_fade(i - 1);
        let label = if on {
            "Remove fade before"
        } else {
            "Add fade before"
        };
        let cmd = Command::SetFade {
            left: tl.clips[i - 1].id,
            right: c.id,
            on: !on,
        };
        entry(ui, true, label, cmds, vec![cmd]);
    }
    if i + 1 < tl.clips.len() {
        let on = tl.has_fade(i);
        let label = if on {
            "Remove fade after"
        } else {
            "Add fade after"
        };
        let cmd = Command::SetFade {
            left: c.id,
            right: tl.clips[i + 1].id,
            on: !on,
        };
        entry(ui, true, label, cmds, vec![cmd]);
    }
    ui.menu_button("Effects", |ui| {
        for effect in EffectKind::ALL
            .into_iter()
            .filter(|e| *e != EffectKind::ChangeAspect)
        {
            entry(
                ui,
                true,
                effect.name(),
                cmds,
                vec![
                    select.clone(),
                    Command::ApplyEffect {
                        effect,
                        target: Target::Clip(c.id),
                    },
                ],
            );
        }
    });
    ui.separator();
    entry(
        ui,
        true,
        "Clip details…",
        cmds,
        vec![select, Command::OpenPopup(Popup::ClipDetails)],
    );
}

/// Shift+drag over the timeline selects the subtitle lines starting in the dragged time
/// range (Shift+Ctrl/Cmd adds them to the selection). Only active with Shift held, so it
/// never takes clicks or drags from the clips.
#[allow(clippy::too_many_arguments)]
fn range_select(
    ui: &mut Ui,
    cache: &mut UiCache,
    painter: &Painter,
    rect: Rect,
    x0: f32,
    z: f32,
    cmds: &mut Vec<Command>,
) {
    let shift = ui.input(|i| i.modifiers.shift);
    if !shift && cache.range_drag.is_none() {
        return;
    }
    let t_at = |x: f32| Micros::from_secs_f64(f64::from(((x - x0) / z).max(0.0)));
    let resp = ui
        .interact(rect, Id::unique("videolore-range-select"), Sense::drag())
        .on_hover_text("Shift+drag: select the subtitle lines starting here");
    if resp.drag_started()
        && let Some(o) = ui.input(|i| i.pointer.press_origin())
    {
        let add = ui.input(|i| i.modifiers.command);
        cache.range_drag = Some((t_at(o.x), add));
    }
    let Some((from, add)) = cache.range_drag else {
        return;
    };
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    if let Some(p) = pointer {
        let xa = x0 + (from.as_secs_f64() * f64::from(z)) as f32;
        let band = Rect::from_x_y_ranges(xa.min(p.x)..=xa.max(p.x), rect.y_range());
        painter.rect_filled(
            band,
            0.0,
            Color32::from_rgba_unmultiplied(120, 160, 255, 40),
        );
    }
    if resp.drag_stopped() || !ui.input(|i| i.pointer.any_down()) {
        cache.range_drag = None;
        if let Some(p) = pointer {
            cmds.push(Command::SelectRange {
                from,
                to: t_at(p.x),
                add,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn marker_lane(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    painter: &Painter,
    lane: Rect,
    x0: f32,
    z: f32,
    visible: Rect,
    cmds: &mut Vec<Command>,
) {
    let all = markers::layout(&app.project, z, x0, (visible.min.x, visible.max.x));
    let mid = lane.center().y;
    for m in &all {
        let chosen = app.popups.instance == Some(m.instance)
            || matches!(m.instance, Instance::Line(id, _)
                if app.selection.contains(&id));
        let outline = Stroke::new(
            if chosen { 2.0 } else { 1.0 },
            if chosen {
                Color32::YELLOW
            } else {
                Color32::WHITE
            },
        );
        match m.kind {
            MarkerKind::ClipVolume => {
                let c = pos2(m.x + 5.0, mid);
                painter.circle(c, 4.0, Color32::from_rgb(60, 140, 200), outline);
            }
            MarkerKind::CleanAudio => {
                let c = pos2(m.x + 5.0, mid);
                painter.rect(
                    Rect::from_center_size(c, vec2(8.0, 8.0)),
                    1.0,
                    Color32::from_rgb(70, 170, 110),
                    outline,
                    StrokeKind::Inside,
                );
            }
            MarkerKind::Scale => {
                let c = pos2(m.x + 5.0, mid);
                painter.rect(
                    Rect::from_center_size(c, vec2(9.0, 7.0)),
                    0.0,
                    Color32::from_rgb(200, 140, 60),
                    outline,
                    StrokeKind::Inside,
                );
            }
            MarkerKind::Move => {
                let c = pos2(m.x + 5.0, mid);
                painter.circle(c, 4.0, Color32::from_rgb(170, 90, 190), outline);
            }
            MarkerKind::Lut => {
                let c = pos2(m.x + 5.0, mid);
                painter.rect(
                    Rect::from_center_size(c, vec2(10.0, 10.0)),
                    3.0,
                    Color32::from_rgb(190, 80, 110),
                    outline,
                    StrokeKind::Inside,
                );
            }
            MarkerKind::Fade | MarkerKind::EdgeFade => {
                let d = 6.0;
                painter.add(Shape::convex_polygon(
                    vec![
                        pos2(m.x, mid - d),
                        pos2(m.x + d, mid),
                        pos2(m.x, mid + d),
                        pos2(m.x - d, mid),
                    ],
                    Color32::BLACK,
                    outline,
                ));
            }
            MarkerKind::Line => {
                painter.line_segment(
                    [pos2(m.x, mid + 4.0), pos2(m.x_end, mid + 4.0)],
                    Stroke::new(2.0, LINE_COLOR),
                );
                painter.add(Shape::convex_polygon(
                    vec![
                        pos2(m.x, mid - 6.0),
                        pos2(m.x + 7.0, mid - 3.0),
                        pos2(m.x, mid),
                    ],
                    LINE_COLOR,
                    outline,
                ));
                painter.line_segment([pos2(m.x, mid - 6.0), pos2(m.x, mid + 6.0)], outline);
            }
            MarkerKind::Pov => {
                // a camera icon-ish square on its point
                let d = 5.0;
                painter.add(Shape::convex_polygon(
                    vec![
                        pos2(m.x - d, mid - d),
                        pos2(m.x + d, mid - d),
                        pos2(m.x + d, mid + d),
                        pos2(m.x - d, mid + d),
                    ],
                    Color32::from_rgb(120, 70, 170),
                    outline,
                ));
            }
            MarkerKind::AddFade => {
                let c = pos2(m.x, mid);
                painter.circle(
                    c,
                    5.0,
                    Color32::from_gray(70),
                    Stroke::new(1.0, Color32::GRAY),
                );
            }
        }
    }
    // every marker type has its icon on its shape (REQ-142)
    for m in &all {
        let (dx, dy) = markers::icon_offset(m.kind);
        painter.text(
            pos2(m.x + dx, mid + dy),
            Align2::CENTER_CENTER,
            markers::icon_of(m.kind),
            FontId::proportional(9.0),
            Color32::WHITE,
        );
    }
    let resp = ui.interact(lane, Id::unique("videolore-marker-lane"), Sense::click());
    let under = |x: f32| markers::hits(&all, x, HIT_TOLERANCE);
    let hover = resp.hover_pos().map(|p| under(p.x)).unwrap_or_default();
    let resp = if hover.is_empty() {
        resp.on_hover_text(
            "Markers: ◆ fade, ⚑ subtitle line (Ctrl/Cmd or Shift+click: select several), ● clip \
             volume, ■ 360° point of view, + add a fade. Click one for its settings.",
        )
    } else {
        let text: Vec<&str> = hover.iter().map(|m| m.tooltip.as_str()).collect();
        resp.on_hover_text_at_pointer(text.join("\n"))
    };
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let hit = under(p.x);
        let toggle = ui.input(|i| i.modifiers.command || i.modifiers.shift);
        match hit.as_slice() {
            [] => {
                cmds.push(Command::ClearSelection);
                cmds.push(Command::Seek(Micros::from_secs_f64(f64::from(
                    ((p.x - x0) / z).max(0.0),
                ))));
            }
            [one] => cmds.push(markers::click_command_with(one, toggle)),
            many => {
                cache.chooser_pos = Some(p);
                cmds.push(Command::ChooseInstance(
                    many.iter().map(|m| m.instance).collect(),
                ));
            }
        }
    }
    if resp.secondary_clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        cache.context_marker = under(p.x).first().map(|m| (*m).clone());
    }
    let marker = cache.context_marker.clone();
    resp.context_menu(|ui| {
        let Some(m) = &marker else {
            ui.weak("No marker here");
            return;
        };
        match (m.kind, m.instance) {
            (MarkerKind::AddFade, Instance::Fade(left, right)) => {
                let cmd = Command::SetFade {
                    left,
                    right,
                    on: true,
                };
                entry(ui, app.is_enabled(&cmd), "Add fade", cmds, vec![cmd]);
            }
            (_, i) => {
                let open = Command::OpenInstance(i);
                entry(ui, app.is_enabled(&open), "Open settings", cmds, vec![open]);
                let (label, cmd) = match i {
                    Instance::Fade(left, right) => (
                        "Remove",
                        Command::SetFade {
                            left,
                            right,
                            on: false,
                        },
                    ),
                    Instance::Line(id, _) => ("Delete", Command::DeleteSubtitle(id)),
                    Instance::FadeStart => (
                        "Remove",
                        Command::SetEdgeFade {
                            edge: Edge::Start,
                            on: false,
                        },
                    ),
                    Instance::FadeEnd => (
                        "Remove",
                        Command::SetEdgeFade {
                            edge: Edge::End,
                            on: false,
                        },
                    ),
                    Instance::ClipVolume(clip) => (
                        "Reset volume",
                        Command::SetClipVolume {
                            clip,
                            volume: 100,
                            muted: false,
                        },
                    ),
                    Instance::CleanAudio(clip) => (
                        "Remove",
                        Command::SetCleanAudio {
                            target: crate::app::Target::Clip(clip),
                            on: false,
                        },
                    ),
                    Instance::Pov(media, t) => ("Remove", Command::RemovePov { media, t }),
                    Instance::Aspect => ("Remove effect", Command::RemoveAspect),
                    Instance::Scale(target) => (
                        "Remove effect",
                        Command::RemoveFrameEffect {
                            target,
                            effect: crate::app::FrameFx::Scale,
                        },
                    ),
                    Instance::Move(target) => (
                        "Remove effect",
                        Command::RemoveFrameEffect {
                            target,
                            effect: crate::app::FrameFx::Move,
                        },
                    ),
                    Instance::Lut(target) => ("Remove effect", Command::RemoveLut(target)),
                };
                entry(ui, app.is_enabled(&cmd), label, cmds, vec![cmd]);
                if let Instance::Line(id, _) = i {
                    let reset = Command::SetLinePositions {
                        ids: vec![id],
                        pos: None,
                    };
                    entry(ui, true, "Reset position and size", cmds, vec![reset]);
                    // the effect commands act on this line
                    for (label, axis) in [
                        ("Align horizontally", Axis::Horizontal),
                        ("Align vertically", Axis::Vertical),
                    ] {
                        let pick = Command::OpenInstance(i);
                        entry(
                            ui,
                            true,
                            label,
                            cmds,
                            vec![pick, Command::AlignSelected(axis)],
                        );
                    }
                }
            }
        }
    });
}

/// Music clips at their own times: click to select, drag to move, double-click for
/// details, right-click for more.
#[allow(clippy::too_many_arguments)]
fn music_track(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    painter: &Painter,
    music: Rect,
    x0: f32,
    z: f32,
    visible: Rect,
    cmds: &mut Vec<Command>,
) {
    let px = |t: Micros| (t.as_secs_f64() * f64::from(z)) as f32;
    let t_at = |x: f32| Micros::from_secs_f64(f64::from(((x - x0) / z).max(0.0)));
    for m in &app.project.music.clips {
        let r = Rect::from_min_size(
            pos2(x0 + px(m.start), music.min.y),
            vec2(px(m.len()).max(2.0), music.height()),
        );
        let name = media_name(app, m.media);
        let resp = ui
            .interact(
                r,
                Id::unique(("videolore-music", m.id.0)),
                Sense::click_and_drag(),
            )
            .on_hover_text(format!(
                "{name}\nDrag to move, double-click for details, right-click for more"
            ));
        if r.intersects(visible) {
            painter.rect_filled(r.shrink(1.0), 3.0, MUSIC_COLOR);
            if app.selected_music == Some(m.id) {
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(2.0_f32, Color32::YELLOW),
                    StrokeKind::Inside,
                );
            }
            paint_name(
                painter,
                r.min + vec2(4.0, 2.0),
                &name,
                r.width() - 8.0,
                10.0,
            );
        }
        if resp.clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            cmds.push(Command::SelectMusic(Some(m.id)));
            cmds.push(Command::Seek(t_at(p.x)));
        }
        if resp.double_clicked() {
            cmds.push(Command::SelectMusic(Some(m.id)));
            cmds.push(Command::OpenPopup(Popup::ClipDetails));
        }
        // the pointer's start is gone when the button is released: keep the last delta
        let drag_id = resp.id.with("dx");
        if let Some(d) = resp.total_drag_delta() {
            ui.data_mut(|data| data.insert_temp(drag_id, d.x));
        }
        if resp.drag_stopped() {
            let dx: f32 = ui.data_mut(|data| data.remove_temp(drag_id)).unwrap_or(0.0);
            let delta = Micros::from_secs_f64(f64::from(dx / z));
            cmds.push(Command::SelectMusic(Some(m.id)));
            cmds.push(Command::MoveMusic {
                clip: m.id,
                start: (m.start + delta).max(Micros::ZERO),
            });
        }
        if resp.secondary_clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            cache.context_time = Some(t_at(p.x));
        }
        let at = cache.context_time.unwrap_or(app.playhead);
        let select = Command::SelectMusic(Some(m.id));
        resp.context_menu(|ui| {
            entry(
                ui,
                at > m.start && at < m.end(),
                "Split",
                cmds,
                vec![Command::Seek(at), Command::SplitMusic],
            );
            entry(
                ui,
                true,
                "Delete",
                cmds,
                vec![select.clone(), Command::DeleteMusic],
            );
            entry(
                ui,
                true,
                "Move to playhead",
                cmds,
                vec![
                    select.clone(),
                    Command::MoveMusic {
                        clip: m.id,
                        start: app.playhead,
                    },
                ],
            );
            ui.separator();
            entry(
                ui,
                true,
                "Clip details…",
                cmds,
                vec![select.clone(), Command::OpenPopup(Popup::ClipDetails)],
            );
        });
    }
}

/// The Text lane: one bar per text item, from its start for its time to live (REQ-110).
#[allow(clippy::too_many_arguments)]
fn text_track(
    ui: &mut Ui,
    app: &AppState,
    painter: &Painter,
    lane: Rect,
    x0: f32,
    z: f32,
    visible: Rect,
    cmds: &mut Vec<Command>,
) {
    let px = |t: Micros| (t.as_secs_f64() * f64::from(z)) as f32;
    let video_end = app.project.timeline.duration();
    for t in &app.project.texts.items {
        let r = Rect::from_min_size(
            pos2(x0 + px(t.start), lane.min.y),
            vec2(px(t.ttl), lane.height()),
        );
        let first = t.text.lines().next().unwrap_or("").to_string();
        let selected = app.selection.contains_text(t.id);
        let resp = ui
            .interact(
                r,
                Id::unique(("videolore-text", t.id.0)),
                Sense::click_and_drag(),
            )
            .on_hover_text(format!(
                "{first}\nClick for its settings, drag to move, drag an end to change the time \
                 it stays, right-click to delete"
            ));
        let grab = BAR_GRAB.min(r.width() / 3.0);
        let left = ui.interact(
            Rect::from_min_size(r.min, vec2(grab, r.height())),
            Id::unique(("videolore-text-left", t.id.0)),
            Sense::drag(),
        );
        let right = ui.interact(
            Rect::from_min_max(pos2(r.max.x - grab, r.min.y), r.max),
            Id::unique(("videolore-text-right", t.id.0)),
            Sense::drag(),
        );
        if left.hovered() || right.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if r.intersects(visible) {
            painter.rect_filled(r.shrink(1.0), 3.0, TEXT_COLOR);
            let cut = x0 + px(video_end);
            if cut < r.max.x {
                // the part after the end of the video is not shown
                let past = Rect::from_min_max(pos2(cut.max(r.min.x), r.min.y), r.max);
                painter.rect_filled(past.shrink(1.0), 3.0, Color32::from_black_alpha(120));
            }
            if selected {
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(2.0_f32, Color32::YELLOW),
                    StrokeKind::Inside,
                );
            }
            paint_name(
                painter,
                r.min + vec2(4.0, 2.0),
                &first,
                r.width() - 8.0,
                10.0,
            );
        }
        if resp.clicked() {
            if ui.input(|i| i.modifiers.command || i.modifiers.shift) {
                cmds.push(Command::ToggleText(t.id));
            } else {
                cmds.push(Command::OpenText(t.id));
            }
        }
        for (mode, r) in [
            (BarDrag::Move, &resp),
            (BarDrag::Left, &left),
            (BarDrag::Right, &right),
        ] {
            // the pointer's start is gone when the button is released: keep the last delta
            let drag_id = r.id.with("dx");
            if let Some(d) = r.total_drag_delta() {
                ui.data_mut(|data| data.insert_temp(drag_id, d.x));
            }
            if r.drag_stopped() {
                let dx: f32 = ui.data_mut(|data| data.remove_temp(drag_id)).unwrap_or(0.0);
                if dx != 0.0 {
                    let delta = Micros::from_secs_f64(f64::from(dx / z));
                    let (start, ttl) = dragged(t.start, t.ttl, mode, delta);
                    cmds.push(Command::MoveText {
                        id: t.id,
                        start,
                        ttl,
                    });
                }
            }
        }
        let del = Command::DeleteTexts(vec![t.id]);
        resp.context_menu(|ui| {
            entry(ui, true, "Settings…", cmds, vec![Command::OpenText(t.id)]);
            entry(ui, app.is_enabled(&del), "Delete", cmds, vec![del.clone()]);
        });
    }
}
