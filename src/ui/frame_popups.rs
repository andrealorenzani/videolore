// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The popups of Change aspect, Scale and Move (REQ-124, REQ-127..129). egui stays thin:
//! the timeline of keyframes is laid out by the pure [`KeyBar`].

use super::{UiCache, button};
use crate::app::{AppState, Command, EffectKind, FrameDrag, FrameFx, FrameValue, Target};
use crate::base::pos::{POS_MAX, Pos};
use crate::frame::{Fill, Fit, MAX_SCALE, MIN_SCALE, Rgb, Scale, Shape};
use crate::time::Micros;
use crate::timeline::FrameTarget;
use eframe::egui::{self, Color32, Sense, Stroke, Ui, pos2, vec2};

/// Narrowest timeline of keyframes (screen pixels).
pub const MIN_BAR_WIDTH: f32 = 240.0;
/// How close a click must be to a keyframe to select it (screen pixels).
pub const KEY_TOLERANCE: f32 = 6.0;

/// The timeline of keyframes of one popup: a time range laid over a width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyBar {
    pub from: Micros,
    pub to: Micros,
    pub width: f32,
}

impl KeyBar {
    /// `width` is as wide as the popup allows, never narrower than [`MIN_BAR_WIDTH`].
    pub fn new(from: Micros, to: Micros, available: f32) -> KeyBar {
        KeyBar {
            from,
            to: to.max(from + Micros(1)),
            width: available.max(MIN_BAR_WIDTH),
        }
    }

    /// The x of a time (`None` outside the range: hidden keyframes are not drawn).
    pub fn x_of(&self, t: Micros) -> Option<f32> {
        (t >= self.from && t <= self.to).then(|| {
            ((t - self.from).0 as f64 / (self.to - self.from).0 as f64) as f32 * self.width
        })
    }

    /// The time at x (clamped to the range).
    pub fn time_at(&self, x: f32) -> Micros {
        let f = (f64::from(x) / f64::from(self.width)).clamp(0.0, 1.0);
        self.from + Micros(((self.to - self.from).0 as f64 * f).round() as i64)
    }

    /// The key within [`KEY_TOLERANCE`] of x, the nearest one.
    pub fn key_at(&self, x: f32, keys: &[Micros]) -> Option<Micros> {
        keys.iter()
            .filter_map(|k| self.x_of(*k).map(|kx| ((kx - x).abs(), *k)))
            .filter(|(d, _)| *d <= KEY_TOLERANCE)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, k)| k)
    }
}

/// The range of key times a popup shows: the kept part of a clip (source time) or, for the
/// whole video, the first clip's keys over the longest clip; and the timeline time of the range's start (to place the playhead).
pub fn key_range(app: &AppState, t: FrameTarget) -> Option<(Micros, Micros, Micros)> {
    let tl = &app.project.timeline;
    match t {
        // the first clip with keys, on a timeline as long as the longest clip (REQ-148)
        FrameTarget::Whole => {
            let from = tl.frame_start(FrameTarget::Whole);
            let longest = tl.clips.iter().map(|c| c.len()).max()?;
            Some((from, from + longest, Micros::ZERO))
        }
        FrameTarget::Clip(id) => {
            let i = tl.index_of(id)?;
            let c = &tl.clips[i];
            Some((c.src_in, c.src_out, tl.start_of(i)))
        }
    }
}

/// The body of the Change aspect popup.
pub fn aspect_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let Some(cur) = app.project.timeline.frame_aspect else {
        ui.label("Change aspect is not applied.");
        button(
            ui,
            app,
            "Apply",
            Command::ApplyEffect {
                effect: EffectKind::ChangeAspect,
                target: Target::WholeVideo,
            },
            cmds,
        );
        return;
    };
    let mut a = cur;
    egui::Grid::new("aspect-grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Aspect");
            egui::ComboBox::from_id_salt("aspect-shape")
                .selected_text(a.shape.name())
                .show_ui(ui, |ui| {
                    for s in Shape::ALL {
                        ui.selectable_value(&mut a.shape, s, s.name());
                    }
                });
            ui.end_row();
            ui.label("Fit");
            egui::ComboBox::from_id_salt("aspect-fit")
                .selected_text(a.fit.name())
                .show_ui(ui, |ui| {
                    for f in Fit::ALL {
                        ui.selectable_value(&mut a.fit, f, f.name());
                    }
                });
            ui.end_row();
            ui.label("Empty space");
            let is_colour = matches!(a.fill, Fill::Colour(_));
            egui::ComboBox::from_id_salt("aspect-fill")
                .selected_text(a.fill.name())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(is_colour, Fill::Colour(Rgb::BLACK).name())
                        .clicked()
                        && !is_colour
                    {
                        a.fill = Fill::Colour(Rgb::BLACK);
                    }
                    if ui.selectable_label(!is_colour, Fill::Blur.name()).clicked() {
                        a.fill = Fill::Blur;
                    }
                });
            ui.end_row();
            if let Fill::Colour(c) = a.fill {
                ui.label("Colour");
                let mut rgb = [c.0, c.1, c.2];
                if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                    a.fill = Fill::Colour(Rgb(rgb[0], rgb[1], rgb[2]));
                }
                ui.end_row();
            }
        });
    if a != cur {
        cmds.push(Command::SetAspect(a));
    }
    ui.weak("The frame is the video's size with this shape; the picture is never stretched.");
    button(ui, app, "Remove effect", Command::RemoveAspect, cmds);
}

/// The body of the Scale or Move popup of `target`.
pub fn key_body(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    target: FrameTarget,
    fx: FrameFx,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let keys: Vec<Micros> = match fx {
        FrameFx::Scale => tl.scale_keys(target).iter().map(|k| k.t).collect(),
        FrameFx::Move => tl.move_keys(target).iter().map(|k| k.t).collect(),
    };
    let effect = match fx {
        FrameFx::Scale => EffectKind::Scale,
        FrameFx::Move => EffectKind::Move,
    };
    let Some((from, to, start)) = key_range(app, target) else {
        ui.label("This clip no longer exists.");
        return;
    };
    if keys.is_empty() {
        ui.label(format!("{} is not applied here.", effect.name()));
        button(
            ui,
            app,
            "Apply",
            Command::ApplyEffect {
                effect,
                target: match target {
                    FrameTarget::Whole => Target::WholeVideo,
                    FrameTarget::Clip(c) => Target::Clip(c),
                },
            },
            cmds,
        );
        return;
    }
    // the timeline of keyframes, as wide as the popup
    let bar = KeyBar::new(from, to, ui.available_width());
    let (rect, resp) = ui.allocate_exact_size(vec2(bar.width, 28.0), Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    let selected = app.selected_key(target, fx);
    let at_playhead = tl.key_time(target, app.playhead);
    if let Some(x) = at_playhead.and_then(|t| bar.x_of(t)) {
        painter.line_segment(
            [
                pos2(rect.left() + x, rect.top()),
                pos2(rect.left() + x, rect.bottom()),
            ],
            Stroke::new(1.0, Color32::LIGHT_RED),
        );
    }
    for k in &keys {
        if let Some(x) = bar.x_of(*k) {
            let on = selected == Some(*k);
            let c = pos2(rect.left() + x, rect.center().y);
            painter.circle(
                c,
                if on { 6.0 } else { 4.5 },
                if on { Color32::YELLOW } else { Color32::WHITE },
                Stroke::new(1.0, Color32::BLACK),
            );
        }
    }
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        let x = p.x - rect.left();
        match bar.key_at(x, &keys) {
            Some(k) => cmds.push(Command::SelectKey(Some(k))),
            None => {
                cmds.push(Command::SelectKey(None));
                cmds.push(Command::Seek(start + (bar.time_at(x) - from)));
            }
        }
    }
    resp.on_hover_text("Click a keyframe to select it; click elsewhere to move the playhead");
    ui.weak(format!(
        "{} keyframe{}. The value changes evenly between keyframes and stays the same before the first and after the last.",
        keys.len(),
        if keys.len() == 1 { "" } else { "s" }
    ));
    ui.horizontal_wrapped(|ui| {
        button(
            ui,
            app,
            "Add keyframe",
            Command::AddKey { target, effect: fx },
            cmds,
        );
        button(
            ui,
            app,
            "Remove keyframe",
            Command::RemoveKey { target, effect: fx },
            cmds,
        );
    });
    values(ui, app, cache, target, fx, selected, cmds);
    button(
        ui,
        app,
        "Remove effect",
        Command::RemoveFrameEffect { target, effect: fx },
        cmds,
    );
}

/// The value fields: Scale in percent, Move as the picture centre in percent of the frame.
fn values(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    target: FrameTarget,
    fx: FrameFx,
    selected: Option<Micros>,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let at = selected.or_else(|| tl.key_time(target, app.playhead));
    let Some(at) = at else { return };
    let (mut v, scale0, pos0) = match fx {
        FrameFx::Scale => {
            let s = crate::keyframes::at(tl.scale_keys(target), at).unwrap_or(Scale::NORMAL);
            ([f64::from(s.permille) / 10.0, 0.0], s, crate::frame::CENTRE)
        }
        FrameFx::Move => {
            let p = crate::keyframes::at(tl.move_keys(target), at).unwrap_or(crate::frame::CENTRE);
            (
                [
                    f64::from(p.x) * 100.0 / f64::from(POS_MAX),
                    f64::from(p.y) * 100.0 / f64::from(POS_MAX),
                ],
                Scale::NORMAL,
                p,
            )
        }
    };
    if let Some(e) = cache.frame_edit {
        v = e;
    }
    let mut changed = false;
    let mut done = false;
    ui.horizontal(|ui| match fx {
        FrameFx::Scale => {
            ui.label("Scale");
            let r = ui.add(
                egui::DragValue::new(&mut v[0])
                    .range(f64::from(MIN_SCALE) / 10.0..=f64::from(MAX_SCALE) / 10.0)
                    .speed(0.5)
                    .max_decimals(1)
                    .suffix(" %"),
            );
            changed |= r.changed();
            done |= r.drag_stopped() || r.lost_focus();
        }
        FrameFx::Move => {
            for (label, i) in [("Centre x", 0), ("y", 1)] {
                ui.label(label);
                let r = ui.add(
                    egui::DragValue::new(&mut v[i])
                        .range(0.0..=100.0)
                        .speed(0.2)
                        .max_decimals(1)
                        .suffix(" %"),
                );
                changed |= r.changed();
                done |= r.drag_stopped() || r.lost_focus();
            }
        }
    });
    if changed {
        cache.frame_edit = Some(v);
    }
    if done || (changed && !ui.input(|i| i.pointer.any_down())) {
        cache.frame_edit = None;
        let value = match fx {
            FrameFx::Scale => FrameValue::Scale(Scale {
                permille: (v[0] * 10.0).round() as u16,
                ..scale0
            }),
            FrameFx::Move => FrameValue::Move(Pos::from_fraction(v[0] / 100.0, v[1] / 100.0)),
        };
        let same = match value {
            FrameValue::Scale(s) => s.permille == scale0.permille,
            FrameValue::Move(p) => p == pos0,
        };
        if !same {
            cmds.push(Command::SetPicture(FrameDrag { target, value }));
        }
    }
}
