// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Everything you can do on the preview picture: the boxes around movable objects with
//! their corner handles (REQ-096/097), dragging a box to move it (REQ-089, REQ-092),
//! looking around a 360° clip by dragging and zooming with the wheel (REQ-101), and the
//! right-click menu. The geometry is in [`super::boxes`] and [`super::placement`].

use super::boxes::{self, BoxItem, Corner, HANDLE_TOLERANCE, Hit, ObjectId};
use super::{UiCache, actions::ActionId, menu, placement, subtitle_font};
use crate::app::{AppState, Command, ResizeDrag};
use crate::subtitles::{LineId, SIZE_NORMAL};
use eframe::egui::{
    self, Color32, CursorIcon, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

/// A corner drag in progress: what was grabbed and how the box looked at the start.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxResize {
    pub line: LineId,
    pub ids: Vec<LineId>,
    pub corner: Corner,
    pub rect: Rect,
    pub size: u16,
    pub frame: (u32, u32),
}

/// Side of the square corner handles (screen pixels).
const HANDLE_SIDE: f32 = 7.0;

/// What is under a frame point: with boxes shown the corner handles first, else only the
/// text (moving by dragging the text stays possible without boxes).
pub fn hit(show_boxes: bool, items: &[BoxItem], p: Pos2, tol: f32) -> Option<(BoxItem, Hit)> {
    if show_boxes {
        return boxes::hit_test(items, p, tol);
    }
    items
        .iter()
        .rev()
        .find(|b| b.rect.contains(p))
        .map(|b| (*b, Hit::Body))
}

/// The boxes of the lines on screen, with their sizes (frame pixels).
pub fn items(app: &AppState, ui: &Ui, frame: (u32, u32)) -> Vec<BoxItem> {
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();
    let mut measure = |t: &str, size: f32, wrap: f32| {
        painter
            .layout(
                t.to_string(),
                subtitle_font(&ctx, size),
                Color32::WHITE,
                wrap,
            )
            .size()
    };
    let shown = app.shown_project();
    placement::boxes(&shown, app.playhead, frame, &mut measure)
        .into_iter()
        .map(|(id, rect)| BoxItem {
            id: ObjectId::Line(id),
            rect,
            size: shown
                .subtitles
                .get(id)
                .and_then(|l| l.size)
                .unwrap_or(SIZE_NORMAL),
        })
        .collect()
}

/// Draws the boxes (and the handles) and runs every pointer interaction on the picture,
/// drawn in `picture`.
pub fn interact(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    picture: Rect,
) {
    let Some(frame) = app.output_size() else {
        return;
    };
    let sphere = app.sphere_at_playhead().is_some();
    let show = app.settings.show_boxes;
    let resp = ui
        .interact(
            picture,
            Id::unique("videolore-picture"),
            Sense::click_and_drag(),
        )
        .on_hover_text(if sphere {
            "Drag to look around, wheel to zoom; drag a subtitle to move it"
        } else {
            "Drag a subtitle to move it, a corner of its box to resize it"
        });
    let ctx = ui.ctx().clone();
    let scale = (picture.width() / frame.0.max(1) as f32).max(f32::EPSILON);
    let tol = HANDLE_TOLERANCE / scale;
    let to_frame = |p: Pos2| placement::to_frame(p, picture, frame);
    let items = items(app, ui, frame);

    // outlines and handles
    for b in &items {
        let ObjectId::Line(id) = b.id;
        let selected = app.selection.contains(&id);
        let s = placement::to_screen(b.rect, picture, frame);
        if show {
            let color = if selected {
                ui.visuals().selection.stroke.color
            } else {
                Color32::from_white_alpha(170)
            };
            ui.painter().rect_stroke(
                s,
                2.0,
                Stroke::new(if selected { 2.0 } else { 1.0 }, color),
                StrokeKind::Outside,
            );
            for h in boxes::handle_rects(s, HANDLE_SIDE) {
                ui.painter().rect_filled(h, 1.0, Color32::WHITE);
                ui.painter().rect_stroke(
                    h,
                    1.0,
                    Stroke::new(1.0, Color32::BLACK),
                    StrokeKind::Outside,
                );
            }
        } else if selected {
            ui.painter().rect_stroke(
                s,
                2.0,
                Stroke::new(1.0, ui.visuals().selection.stroke.color),
                StrokeKind::Outside,
            );
        }
    }

    let dragging = cache.subtitle_drag.is_some() || cache.box_resize.is_some() || cache.camera_drag;
    if !dragging && let Some(p) = resp.hover_pos() {
        match hit(show, &items, to_frame(p), tol) {
            Some((_, h)) => ctx.set_cursor_icon(boxes::cursor(h, false)),
            None if sphere => ctx.set_cursor_icon(CursorIcon::Grab),
            None => {}
        }
    }

    // a click inside a box selects its object (Ctrl/Cmd+click adds or removes it)
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
        && let Some((b, _)) = hit(show, &items, to_frame(p), tol)
    {
        let ObjectId::Line(id) = b.id;
        cmds.push(if ui.input(|i| i.modifiers.command) {
            Command::ToggleLine(id)
        } else {
            Command::SelectLine(Some(id))
        });
    }

    if resp.drag_started() {
        let origin = ui
            .input(|i| i.pointer.press_origin())
            .or(resp.interact_pointer_pos());
        cache.subtitle_drag = None;
        cache.box_resize = None;
        cache.camera_drag = false;
        if let Some(o) = origin {
            let f = to_frame(o);
            match hit(show, &items, f, tol) {
                Some((b, Hit::Corner(corner))) => {
                    let ObjectId::Line(line) = b.id;
                    cache.box_resize = Some(BoxResize {
                        line,
                        ids: app.drag_group(line),
                        corner,
                        rect: b.rect,
                        size: b.size,
                        frame,
                    });
                }
                Some((b, Hit::Body)) => {
                    let ObjectId::Line(line) = b.id;
                    cache.subtitle_drag = Some(placement::Drag {
                        line,
                        ids: app.drag_group(line),
                        grab: f - b.rect.center(),
                        size: b.rect.size(),
                        frame,
                    });
                }
                None => cache.camera_drag = sphere,
            }
        }
    }

    if cache.camera_drag {
        let d = resp.drag_delta();
        if d != egui::Vec2::ZERO {
            cmds.push(Command::DragCamera {
                dx: d.x,
                dy: d.y,
                view_w: picture.width(),
            });
        }
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        if resp.drag_stopped() {
            cache.camera_drag = false;
        }
    }
    if sphere && resp.hovered() {
        // wheel and pinch zoom (the preview is not a scroll area)
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            cmds.push(Command::ZoomCamera(scroll));
        }
        let pinch = ui.input(|i| i.zoom_delta());
        if pinch != 1.0 && pinch > 0.0 {
            cmds.push(Command::ZoomCamera(pinch.ln() / 0.0015));
        }
    }
    if sphere && let Some(c) = app.camera_at_playhead() {
        ui.painter().text(
            picture.left_bottom() + vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            format!(
                "360°  yaw {:.0}°  pitch {:.0}°  zoom {:.0}°",
                c.yaw_deg(),
                c.pitch_deg(),
                c.fov_deg()
            ),
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );
    }

    resize_drag(ui, app, cache, cmds, &resp, picture, frame);
    move_drag(ui, app, cache, cmds, &resp, picture, frame);

    resp.context_menu(|ui| {
        for id in [
            ActionId::AlignHorizontal,
            ActionId::AlignVertical,
            ActionId::ResetObjects,
            ActionId::SetPov,
        ] {
            menu::action_item(ui, app, id, cmds);
        }
    });
}

/// A corner being dragged: the box follows the pointer, one undo step on release.
fn resize_drag(
    ui: &Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    resp: &egui::Response,
    picture: Rect,
    frame: (u32, u32),
) {
    let Some(d) = cache.box_resize.clone() else {
        return;
    };
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let Some(p) = pointer else {
        if resp.drag_stopped() {
            cache.box_resize = None;
            cmds.push(Command::DragResize(None));
        }
        return;
    };
    let out = boxes::resize(
        d.rect,
        d.size,
        d.corner,
        placement::to_frame(p, picture, frame),
        frame,
    );
    let s = placement::to_screen(out.rect, picture, frame);
    ui.painter().rect_stroke(
        s,
        2.0,
        Stroke::new(2.0, Color32::YELLOW),
        StrokeKind::Outside,
    );
    ui.ctx()
        .set_cursor_icon(boxes::cursor(Hit::Corner(d.corner), true));
    let drag = ResizeDrag {
        ids: d.ids.clone(),
        line: d.line,
        pos: boxes::centre_pos(out.rect, frame),
        size: out.size,
    };
    if resp.drag_stopped() {
        cache.box_resize = None;
        cmds.push(Command::ResizeLines {
            ids: drag.ids,
            line: drag.line,
            size: drag.size,
            pos: drag.pos,
        });
    } else if app.resize_drag.as_ref() != Some(&drag) {
        cmds.push(Command::DragResize(Some(drag)));
    }
}

/// A box being dragged: an outline follows the pointer at once, the picture is rendered
/// again, one undo step on release.
fn move_drag(
    ui: &Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    resp: &egui::Response,
    picture: Rect,
    frame: (u32, u32),
) {
    let Some(d) = cache.subtitle_drag.clone() else {
        return;
    };
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let pos = pointer
        .map(|p| placement::drag_to(&d, placement::to_frame(p, picture, frame)))
        .or(app.drag_pos.as_ref().map(|(_, p)| *p));
    if let Some(pos) = pos {
        let (fx, fy) = pos.fraction();
        let c = pos2(fx as f32 * frame.0 as f32, fy as f32 * frame.1 as f32);
        let ghost = placement::to_screen(Rect::from_center_size(c, d.size), picture, frame);
        ui.painter().rect_stroke(
            ghost,
            2.0,
            Stroke::new(2.0, Color32::YELLOW),
            StrokeKind::Outside,
        );
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        if resp.drag_stopped() {
            cache.subtitle_drag = None;
            cmds.push(Command::SetLinePositions {
                ids: d.ids,
                pos: Some(pos),
            });
        } else if app.drag_pos.as_ref() != Some(&(d.ids.clone(), pos)) {
            cmds.push(Command::DragLines(Some((d.ids, pos))));
        }
    } else if resp.drag_stopped() {
        cache.subtitle_drag = None;
        cmds.push(Command::DragLines(None));
    }
}
