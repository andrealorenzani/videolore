// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The timeline panel: ruler, video track (clips, fades, subtitle marks) and, below it,
//! the music track. Both share the panel; its size in the window does not change.

use super::zoom;
use crate::app::{AppState, Command};
use crate::time::Micros;
use crate::timeline::Timeline;
use eframe::egui::{
    self, Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

const CLIP_COLORS: [Color32; 4] = [
    Color32::from_rgb(70, 110, 170),
    Color32::from_rgb(90, 140, 90),
    Color32::from_rgb(150, 100, 60),
    Color32::from_rgb(120, 80, 150),
];
const MUSIC_COLOR: Color32 = Color32::from_rgb(40, 130, 130);
/// Share of the track area used by the video track (the rest is music).
pub const VIDEO_SHARE: f32 = 0.62;

/// Length shown on the timeline: the video, or the music when it goes further.
pub fn content_duration(app: &AppState) -> Micros {
    app.project.timeline.duration().max(app.project.music.end())
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

/// Splits the area under the ruler into the video and music tracks.
pub fn track_rects(area: Rect) -> (Rect, Rect) {
    let split = area.min.y + area.height() * VIDEO_SHARE;
    (
        Rect::from_min_max(area.min, pos2(area.max.x, split - 2.0)),
        Rect::from_min_max(pos2(area.min.x, split + 2.0), area.max),
    )
}

pub fn timeline_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let fps = app.fps();
    let width = ui.available_width();
    let total = content_duration(app);
    let range = zoom::range(total.as_secs_f64(), width - 20.0, fps.as_f64());
    let z = zoom::clamp(app.zoom, range);
    if app.zoom != z {
        cmds.push(Command::SetZoom(z));
    }
    let px = |t: Micros| (t.as_secs_f64() * z as f64) as f32;
    let height = ui.available_height() - 16.0;
    let content_w = px(total) + 40.0;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        let (rect, bg) =
            ui.allocate_exact_size(vec2(content_w.max(width - 10.0), height), Sense::click());
        let x0 = rect.min.x;
        let t_at = |x: f32| Micros::from_secs_f64(((x - x0) / z).max(0.0) as f64);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(30));
        // ruler: a tick every second (or less often when zoomed out)
        let step = (60.0 / z).ceil().max(1.0) as i64;
        let ruler_h = 14.0;
        let mut s = 0;
        while (s as f32) * z <= content_w {
            let x = x0 + s as f32 * z;
            painter.line_segment(
                [pos2(x, rect.min.y), pos2(x, rect.min.y + 5.0)],
                Stroke::new(1.0_f32, Color32::GRAY),
            );
            painter.text(
                pos2(x + 2.0, rect.min.y),
                Align2::LEFT_TOP,
                format!("{s}s"),
                FontId::monospace(9.0),
                Color32::GRAY,
            );
            s += step;
        }
        if bg.clicked()
            && let Some(p) = bg.interact_pointer_pos()
        {
            cmds.push(Command::SelectClip(None));
            cmds.push(Command::SelectMusic(None));
            cmds.push(Command::Seek(t_at(p.x)));
        }
        let tracks = Rect::from_min_max(pos2(rect.min.x, rect.min.y + ruler_h), rect.max);
        let (video, music) = track_rects(tracks);
        painter.rect_filled(music, 0.0, Color32::from_gray(38));
        painter.text(
            pos2(rect.min.x + 2.0, music.min.y + 1.0),
            Align2::LEFT_TOP,
            "♪",
            FontId::proportional(10.0),
            Color32::GRAY,
        );
        let mut start = Micros::ZERO;
        for (i, c) in tl.clips.iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(x0 + px(start), video.min.y),
                vec2(px(c.len()), video.height()),
            );
            let resp = ui.interact(
                r,
                ui.scope_id().with(("clip", c.id.0)),
                Sense::click_and_drag(),
            );
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
            let name = app
                .project
                .media(c.media)
                .map(|m| m.name.as_str())
                .unwrap_or("?");
            painter.text(
                r.min + vec2(4.0, 4.0),
                Align2::LEFT_TOP,
                name,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
            if resp.clicked()
                && let Some(p) = resp.interact_pointer_pos()
            {
                cmds.push(Command::SelectClip(Some(c.id)));
                cmds.push(Command::Seek(t_at(p.x)));
            }
            if resp.drag_stopped()
                && let Some(p) = resp.interact_pointer_pos()
            {
                let to = drop_index(tl, t_at(p.x));
                cmds.push(Command::SelectClip(Some(c.id)));
                cmds.push(Command::MoveClip { clip: c.id, to });
            }
            let (fin, fout) = tl.fades_of(i);
            let fade_color = Color32::from_black_alpha(160);
            if fin > Micros::ZERO {
                painter.add(egui::Shape::convex_polygon(
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
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(r.max.x - px(fout), r.min.y),
                        r.right_top(),
                        r.right_bottom(),
                    ],
                    fade_color,
                    Stroke::NONE,
                ));
            }
            start = start + c.len();
        }
        // subtitle marks along the bottom of the video track
        for l in app.project.subtitles.project(tl) {
            let r = Rect::from_min_max(
                pos2(x0 + px(l.start), video.max.y - 4.0),
                pos2(x0 + px(l.end), video.max.y - 1.0),
            );
            let color = if app.selected_line == Some(l.line) {
                Color32::YELLOW
            } else {
                Color32::from_rgb(230, 200, 90)
            };
            painter.rect_filled(r, 0.0, color);
        }
        // boundary handles: click to toggle the fade
        let mut at = Micros::ZERO;
        for b in 0..tl.clips.len().saturating_sub(1) {
            at = at + tl.clips[b].len();
            let center = pos2(x0 + px(at), video.max.y - 10.0);
            let hr = Rect::from_center_size(center, vec2(12.0, 12.0));
            let resp = ui
                .interact(hr, ui.scope_id().with(("fade", b)), Sense::click())
                .on_hover_text("Toggle fade");
            let on = tl.has_fade(b);
            painter.circle(
                center,
                5.0,
                if on {
                    Color32::BLACK
                } else {
                    Color32::from_gray(200)
                },
                Stroke::new(1.0_f32, Color32::WHITE),
            );
            if resp.clicked() {
                cmds.push(Command::ToggleFade(b));
            }
        }
        // music track: clips at their own times; drag to move
        for m in &app.project.music.clips {
            let r = Rect::from_min_size(
                pos2(x0 + px(m.start), music.min.y),
                vec2(px(m.len()).max(2.0), music.height()),
            );
            let resp = ui.interact(
                r,
                ui.scope_id().with(("music", m.id.0)),
                Sense::click_and_drag(),
            );
            painter.rect_filled(r.shrink(1.0), 3.0, MUSIC_COLOR);
            if app.selected_music == Some(m.id) {
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(2.0_f32, Color32::YELLOW),
                    StrokeKind::Inside,
                );
            }
            let name = app
                .project
                .media(m.media)
                .map(|m| m.name.as_str())
                .unwrap_or("?");
            painter.text(
                r.min + vec2(4.0, 2.0),
                Align2::LEFT_TOP,
                name,
                FontId::proportional(10.0),
                Color32::WHITE,
            );
            if resp.clicked()
                && let Some(p) = resp.interact_pointer_pos()
            {
                cmds.push(Command::SelectMusic(Some(m.id)));
                cmds.push(Command::Seek(t_at(p.x)));
            }
            // the pointer's start is gone when the button is released: keep the last delta
            let drag_id = resp.id.with("dx");
            if let Some(d) = resp.total_drag_delta() {
                ui.data_mut(|data| data.insert_temp(drag_id, d.x));
            }
            if resp.drag_stopped() {
                let dx: f32 = ui.data_mut(|data| data.remove_temp(drag_id)).unwrap_or(0.0);
                let delta = Micros::from_secs_f64((dx / z) as f64);
                cmds.push(Command::SelectMusic(Some(m.id)));
                cmds.push(Command::MoveMusic {
                    clip: m.id,
                    start: (m.start + delta).max(Micros::ZERO),
                });
            }
        }
        let xp = x0 + px(app.playhead);
        painter.line_segment(
            [pos2(xp, rect.min.y), pos2(xp, rect.max.y)],
            Stroke::new(2.0_f32, Color32::RED),
        );
    });
}
