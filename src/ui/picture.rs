// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Everything you can do on the preview picture: the boxes around movable objects with
//! their corner handles (REQ-096/097), dragging a box to move it (REQ-089, REQ-092),
//! looking around a 360° clip by dragging and zooming with the wheel (REQ-101), and the
//! right-click menu. The geometry is in [`super::boxes`] and [`super::placement`].

use super::boxes::{self, BoxItem, Corner, HANDLE_TOLERANCE, Hit, ObjectId};
use super::frame_handles::{self, Grab};
use super::{UiCache, actions::ActionId, menu, placement, subtitle_font};
use crate::app::{AppState, Command, FrameDrag, FrameFx, FrameValue, ResizeDrag};
use crate::subtitles::{LineId, SIZE_NORMAL};
use eframe::egui::{
    self, Color32, CursorIcon, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

/// A corner drag in progress: what was grabbed and how the box looked at the start.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxResize {
    pub line: LineId,
    pub ids: Vec<LineId>,
    /// What was grabbed: a corner or (box partly outside the picture) a visible side.
    pub grip: Hit,
    pub rect: Rect,
    pub size: u16,
    pub frame: (u32, u32),
}

/// A text box being moved or resized in the preview (REQ-111).
#[derive(Debug, Clone, PartialEq)]
pub struct TextBoxDrag {
    /// Every text that moves with the grabbed one.
    pub ids: Vec<crate::text::TextId>,
    /// True when the right-edge handle is dragged (the width changes), else the whole box
    /// moves.
    pub edge: bool,
    /// The box at the start (frame pixels).
    pub rect: Rect,
    /// Pointer minus box centre at the start.
    pub grab: egui::Vec2,
    pub frame: (u32, u32),
    /// The camera when the box belongs to a text fixed to the sphere: moving it then
    /// changes the point of the sphere, not the flat place.
    pub cam: Option<crate::camera::Camera>,
}

/// Side of the square corner handles (screen pixels).
const HANDLE_SIDE: f32 = 7.0;

/// What is under a frame point: with boxes shown the corner handles first, else only the
/// text (moving by dragging the text stays possible without boxes).
pub fn hit(
    show_boxes: bool,
    items: &[BoxItem],
    p: Pos2,
    tol: f32,
    frame: (u32, u32),
) -> Option<(BoxItem, Hit)> {
    if show_boxes {
        return boxes::hit_test(items, p, tol, frame);
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
    let cam = app.camera_at_playhead();
    let mut line_boxes = placement::boxes(&shown, app.playhead, frame, &mut measure);
    if let Some(c) = cam {
        // lines fixed to the sphere are where the camera sees them
        for (id, r) in placement::anchored_line_boxes(&shown, app.playhead, frame, c, &mut measure)
        {
            line_boxes.retain(|(l, _)| *l != id);
            if let Some(r) = r {
                line_boxes.push((id, r));
            }
        }
    }
    let mut v: Vec<BoxItem> = line_boxes
        .into_iter()
        .map(|(id, rect)| BoxItem {
            id: ObjectId::Line(id),
            rect,
            size: shown
                .subtitles
                .get(id)
                .and_then(|l| l.size)
                .unwrap_or(SIZE_NORMAL),
            width_only: false,
        })
        .collect();
    v.extend(
        placement::text_boxes(
            &shown,
            app.playhead,
            frame,
            app.camera_at_playhead(),
            &mut measure,
        )
        .into_iter()
        .map(|(id, rect)| BoxItem {
            id: ObjectId::Text(id),
            rect,
            size: SIZE_NORMAL,
            width_only: true,
        }),
    );
    v
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
        let selected = match b.id {
            ObjectId::Line(id) => app.selection.contains(&id),
            ObjectId::Text(id) => app.selection.contains_text(id),
        };
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
            // the handles sit on the visible part of the box (REQ-145, REQ-146)
            let handles = boxes::handle_points(b.rect, frame, b.width_only)
                .into_iter()
                .map(|at| {
                    let at = placement::to_screen(Rect::from_min_max(at, at), picture, frame);
                    Rect::from_center_size(at.min, vec2(HANDLE_SIDE, HANDLE_SIDE))
                });
            for h in handles {
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

    draw_frame_handles(ui, app, picture, frame);

    let dragging = cache.subtitle_drag.is_some()
        || cache.frame_handle.is_some()
        || cache.box_resize.is_some()
        || cache.text_box_drag.is_some()
        || cache.camera_drag;
    if !dragging && let Some(p) = resp.hover_pos() {
        match hit(show, &items, to_frame(p), tol, frame) {
            Some((_, h)) => ctx.set_cursor_icon(boxes::cursor(h, false)),
            None if sphere => ctx.set_cursor_icon(CursorIcon::Grab),
            None => {}
        }
    }

    // a click inside a box selects its object (Ctrl/Cmd/Shift+click adds or removes it);
    // a click on the empty picture clears the selection (REQ-090)
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        match hit(show, &items, to_frame(p), tol, frame) {
            Some((b, _)) => {
                let toggle = ui.input(|i| i.modifiers.command || i.modifiers.shift);
                cmds.push(match (b.id, toggle) {
                    (ObjectId::Line(id), true) => Command::ToggleLine(id),
                    (ObjectId::Line(id), false) => Command::SelectLine(Some(id)),
                    (ObjectId::Text(id), true) => Command::ToggleText(id),
                    (ObjectId::Text(id), false) => Command::SelectText(id),
                });
            }
            None => cmds.push(Command::ClearSelection),
        }
    }

    if resp.drag_started() {
        let origin = ui
            .input(|i| i.pointer.press_origin())
            .or(resp.interact_pointer_pos());
        cache.subtitle_drag = None;
        cache.box_resize = None;
        cache.text_box_drag = None;
        cache.frame_handle = None;
        cache.camera_drag = false;
        if let Some(o) = origin {
            let f = to_frame(o);
            match hit(show, &items, f, tol, frame) {
                Some((b, h)) => match b.id {
                    ObjectId::Text(id) => {
                        cache.text_box_drag = Some(TextBoxDrag {
                            ids: app
                                .selection
                                .drag_targets(crate::selection::ObjRef::Text(id))
                                .into_iter()
                                .filter_map(|o| match o {
                                    crate::selection::ObjRef::Text(t) => Some(t),
                                    crate::selection::ObjRef::Line(_) => None,
                                })
                                .collect(),
                            edge: h == Hit::Edge,
                            rect: b.rect,
                            grab: f - b.rect.center(),
                            frame,
                            cam: app
                                .project
                                .texts
                                .get(id)
                                .filter(|t| t.anchor.is_some())
                                .and_then(|_| app.camera_at_playhead()),
                        });
                    }
                    ObjectId::Line(line) => match h {
                        Hit::Edge => {}
                        Hit::Corner(_) | Hit::Side(_) => {
                            cache.box_resize = Some(BoxResize {
                                line,
                                ids: app.drag_group(line),
                                grip: h,
                                rect: b.rect,
                                size: b.size,
                                frame,
                            });
                        }
                        Hit::Body => {
                            cache.subtitle_drag = Some(placement::Drag {
                                line,
                                ids: app.drag_group(line),
                                grab: f - b.rect.center(),
                                size: b.rect.size(),
                                frame,
                                whole_box: sphere,
                            });
                        }
                    },
                },
                // the handles of Scale and Move come after the subtitle and text boxes
                None => match grab_at(app, f, tol, frame) {
                    Some(g) => cache.frame_handle = Some(g),
                    None => cache.camera_drag = sphere,
                },
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

    frame_drag(ui, app, cache, cmds, &resp, picture);
    text_box_drag(ui, cache, cmds, &resp, picture);
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
    let at = placement::to_frame(p, picture, frame);
    let out = match d.grip {
        Hit::Side(side) => boxes::resize_side(d.rect, d.size, side, at, frame),
        Hit::Corner(corner) => boxes::resize(d.rect, d.size, corner, at, frame),
        Hit::Edge | Hit::Body => boxes::resize(d.rect, d.size, Corner::BottomRight, at, frame),
    };
    let s = placement::to_screen(out.rect, picture, frame);
    ui.painter().rect_stroke(
        s,
        2.0,
        Stroke::new(2.0, Color32::YELLOW),
        StrokeKind::Outside,
    );
    ui.ctx().set_cursor_icon(boxes::cursor(d.grip, true));
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
        let cam = app.camera_at_playhead().filter(|_| {
            app.project
                .subtitles
                .get(d.line)
                .is_some_and(|l| l.anchor.is_some())
        });
        if resp.drag_stopped() {
            cache.subtitle_drag = None;
            match cam {
                // a line fixed to the sphere goes to the point of the sphere under its box
                Some(cam) => {
                    let (yaw, pitch) = crate::camera::unproject(
                        cam,
                        frame.0,
                        frame.1,
                        f64::from(c.x),
                        f64::from(c.y),
                    );
                    cmds.push(Command::MoveLineAnchor {
                        ids: d.ids,
                        yaw_cd: (yaw * 100.0).round() as i32,
                        pitch_cd: (pitch * 100.0).round() as i32,
                    });
                }
                None => cmds.push(Command::SetLinePositions {
                    ids: d.ids,
                    pos: Some(pos),
                }),
            }
        } else if app.drag_pos.as_ref() != Some(&(d.ids.clone(), pos)) {
            cmds.push(Command::DragLines(Some((d.ids, pos))));
        }
    } else if resp.drag_stopped() {
        cache.subtitle_drag = None;
        cmds.push(Command::DragLines(None));
    }
}

/// A text box being moved or resized: an outline follows the pointer, one undo step on
/// release (REQ-111).
fn text_box_drag(
    ui: &Ui,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    resp: &egui::Response,
    picture: Rect,
) {
    let Some(d) = cache.text_box_drag.clone() else {
        return;
    };
    let Some(p) = resp.interact_pointer_pos().or(resp.hover_pos()) else {
        if resp.drag_stopped() {
            cache.text_box_drag = None;
        }
        return;
    };
    let pointer = placement::to_frame(p, picture, d.frame);
    let (rect, width) = if d.edge {
        let (r, w) = boxes::resize_width(d.rect, pointer, d.frame);
        (r, Some(w))
    } else {
        let c = if d.cam.is_some() {
            placement::clamp_centre(pointer - d.grab, d.rect.size(), d.frame)
        } else {
            placement::clamp_to_picture(pointer - d.grab, d.frame)
        };
        (Rect::from_center_size(c, d.rect.size()), None)
    };
    ui.painter().rect_stroke(
        placement::to_screen(rect, picture, d.frame),
        2.0,
        Stroke::new(2.0, Color32::YELLOW),
        StrokeKind::Outside,
    );
    ui.ctx().set_cursor_icon(if d.edge {
        CursorIcon::ResizeHorizontal
    } else {
        CursorIcon::Grabbing
    });
    if resp.drag_stopped() {
        cache.text_box_drag = None;
        match (d.cam, width) {
            (Some(cam), Some(_)) => {
                // a text fixed to the sphere: the width is an angle
                let focal = (f64::from(d.frame.0) / 2.0) / (cam.fov_deg().to_radians() / 2.0).tan();
                let per_deg = focal * std::f64::consts::PI / 180.0;
                cmds.push(Command::ResizeTextAnchorWidth {
                    ids: d.ids,
                    width_cd: (f64::from(rect.width()) / per_deg * 100.0).round() as u32,
                });
            }
            (None, Some(width)) => cmds.push(Command::ResizeTextWidth { ids: d.ids, width }),
            // a text fixed to the sphere goes to the point of the sphere under the box
            (Some(cam), None) => {
                let c = rect.center();
                let (yaw, pitch) = crate::camera::unproject(
                    cam,
                    d.frame.0,
                    d.frame.1,
                    f64::from(c.x),
                    f64::from(c.y),
                );
                cmds.push(Command::MoveTextAnchor {
                    ids: d.ids,
                    yaw_cd: (yaw * 100.0).round() as i32,
                    pitch_cd: (pitch * 100.0).round() as i32,
                });
            }
            (None, None) => cmds.push(Command::SetTextBoxes {
                ids: d.ids,
                place: boxes::centre_pos(rect, d.frame),
            }),
        }
    }
}

/// The box, corner handles and target of Scale and Move at the playhead (REQ-127, 128).
fn draw_frame_handles(ui: &Ui, app: &AppState, picture: Rect, frame: (u32, u32)) {
    let (scale, moved) = (
        app.frame_target_at_playhead(FrameFx::Scale).is_some(),
        app.frame_target_at_playhead(FrameFx::Move).is_some(),
    );
    if !scale && !moved {
        return;
    }
    let Some((fr, _)) = app.picture_rect() else {
        return;
    };
    let s = placement::to_screen(frame_handles::to_rect(fr, frame), picture, frame);
    let painter = ui.painter_at(picture.expand(HANDLE_SIDE));
    if scale {
        painter.rect_stroke(
            s,
            0.0,
            Stroke::new(1.5, Color32::LIGHT_BLUE),
            StrokeKind::Outside,
        );
        let rect = frame_handles::to_rect(fr, frame);
        let squares = boxes::handle_points(rect, frame, false)
            .into_iter()
            .map(|at| {
                let at = placement::to_screen(Rect::from_min_max(at, at), picture, frame);
                Rect::from_center_size(at.min, vec2(HANDLE_SIDE, HANDLE_SIDE))
            });
        for h in squares {
            painter.rect_filled(h, 0.0, Color32::LIGHT_BLUE);
            painter.rect_stroke(
                h,
                0.0,
                Stroke::new(1.0, Color32::BLACK),
                StrokeKind::Outside,
            );
        }
    }
    if moved {
        painter.circle(
            s.center(),
            7.0,
            Color32::TRANSPARENT,
            Stroke::new(2.0, Color32::LIGHT_RED),
        );
        painter.circle_filled(s.center(), 2.0, Color32::LIGHT_RED);
    }
}

/// The handle of Scale or Move under frame point `f`, as a drag to start.
fn grab_at(app: &AppState, f: Pos2, tol: f32, frame: (u32, u32)) -> Option<Grab> {
    let (fr, _) = app.picture_rect()?;
    let scale_t = app.frame_target_at_playhead(FrameFx::Scale);
    let move_t = app.frame_target_at_playhead(FrameFx::Move);
    let rect = frame_handles::to_rect(fr, frame);
    let hit = frame_handles::hit(rect, frame, scale_t.is_some(), move_t.is_some(), f, tol)?;
    let target = match hit {
        frame_handles::Hit::Corner(_) | frame_handles::Hit::Edge(_) => scale_t?,
        frame_handles::Hit::Target => move_t?,
    };
    Some(Grab {
        target,
        hit,
        rect,
        scale: app.scale_at(target).unwrap_or(crate::frame::Scale::NORMAL),
        pos: app.move_at(target).unwrap_or(crate::frame::CENTRE),
        grab: f - rect.center(),
        frame,
    })
}

/// A Scale corner or the Move target being dragged: the picture follows the pointer in the
/// preview, one undo step on release.
fn frame_drag(
    ui: &Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    resp: &egui::Response,
    picture: Rect,
) {
    let Some(g) = cache.frame_handle else {
        return;
    };
    let Some(p) = resp.interact_pointer_pos().or(resp.hover_pos()) else {
        if resp.drag_stopped() {
            cache.frame_handle = None;
            cmds.push(Command::DragPicture(None));
        }
        return;
    };
    let p = placement::to_frame(p, picture, g.frame);
    let (value, rect) = match g.hit {
        frame_handles::Hit::Corner(c) => {
            let s = frame_handles::scale_to(&g, c, p);
            (FrameValue::Scale(s), frame_handles::scaled_rect(&g, s))
        }
        frame_handles::Hit::Edge(side) => {
            let s = frame_handles::scale_to_edge(&g, side, p);
            (FrameValue::Scale(s), frame_handles::scaled_rect(&g, s))
        }
        frame_handles::Hit::Target => {
            let m = frame_handles::move_to(&g, p);
            let (fx, fy) = m.fraction();
            let (ox, oy) = g.pos.fraction();
            let shift = egui::vec2(
                ((fx - ox) * f64::from(g.frame.0)) as f32,
                ((fy - oy) * f64::from(g.frame.1)) as f32,
            );
            (FrameValue::Move(m), g.rect.translate(shift))
        }
    };
    ui.painter().rect_stroke(
        placement::to_screen(rect, picture, g.frame),
        0.0,
        Stroke::new(2.0, Color32::YELLOW),
        StrokeKind::Outside,
    );
    ui.ctx().set_cursor_icon(match g.hit {
        frame_handles::Hit::Corner(c) => boxes::cursor(Hit::Corner(c), true),
        frame_handles::Hit::Edge(s) => boxes::side_cursor(s),
        frame_handles::Hit::Target => CursorIcon::Grabbing,
    });
    let drag = FrameDrag {
        target: g.target,
        value,
    };
    if resp.drag_stopped() {
        cache.frame_handle = None;
        cmds.push(Command::SetPicture(drag));
    } else if app.frame_drag != Some(drag) {
        cmds.push(Command::DragPicture(Some(drag)));
    }
}
