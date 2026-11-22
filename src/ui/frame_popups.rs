// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The popups of Change aspect, Scale and Move (REQ-124, REQ-127..129). egui stays thin:
//! the timeline of keyframes is laid out by the pure [`KeyBar`].

use super::strings::tr;
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
pub const KEY_TOLERANCE: f32 = 8.0;
/// Across (screen pixels) of a keyframe marker, at least (REQ-203).
pub const KEY_MARKER: f32 = 12.0;

/// Where the window records that the keyframe bar has the keyboard focus, so that the
/// arrow keys move the keyframe and not the playhead (REQ-203).
pub fn bar_focus_id() -> egui::Id {
    egui::Id::unique("videolore-keybar-focus")
}

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

/// The range of key times a popup shows: the kept part of a clip (source time), and the
/// timeline time of the range's start (to place the playhead). None for `Whole`.
pub fn key_range(app: &AppState, t: FrameTarget) -> Option<(Micros, Micros, Micros)> {
    let tl = &app.project.timeline;
    match t {
        FrameTarget::Whole => None,
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
        ui.label(tr!(ChangeAspectIsNotApplied));
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
    aspect_fields(ui, &mut a, "aspect");
    if a != cur {
        cmds.push(Command::SetAspect(a));
    }
    ui.weak("The frame is the video's size with this shape; the picture is never stretched.");
    button(ui, app, "Remove effect", Command::RemoveAspect, cmds);
}

/// The fields of a Change aspect, the same for the whole video's popup and for a clip's own
/// aspect (REQ-124, REQ-126, REQ-326): the shape, a custom ratio, the Fit (on width or on
/// height), the Empty space (a colour with its chooser, or blurred with its level). `salt`
/// keeps the widgets of two places apart.
pub fn aspect_fields(ui: &mut Ui, a: &mut crate::frame::FrameAspect, salt: &str) {
    egui::Grid::new(format!("{salt}-grid"))
        .num_columns(2)
        .show(ui, |ui| {
            ui.label(tr!(Aspect));
            egui::ComboBox::from_id_salt(format!("{salt}-shape"))
                .selected_text(a.shape.name())
                .show_ui(ui, |ui| {
                    for s in Shape::ALL {
                        ui.selectable_value(&mut a.shape, s, s.name());
                    }
                });
            ui.end_row();
            ui.label(tr!(CustomRatio));
            custom_ratio(ui, a, salt);
            ui.end_row();
            ui.label(tr!(Fit));
            egui::ComboBox::from_id_salt(format!("{salt}-fit"))
                .selected_text(a.fit.name())
                .show_ui(ui, |ui| {
                    for f in Fit::ALL {
                        ui.selectable_value(&mut a.fit, f, f.name());
                    }
                });
            ui.end_row();
            ui.label(tr!(EmptySpace));
            let is_colour = matches!(a.fill, Fill::Colour(_));
            egui::ComboBox::from_id_salt(format!("{salt}-fill"))
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
            if !is_colour {
                ui.label(tr!(Blur));
                egui::ComboBox::from_id_salt(format!("{salt}-blur"))
                    .selected_text(a.blur.name())
                    .show_ui(ui, |ui| {
                        for b in crate::frame::BlurLevel::ALL {
                            ui.selectable_value(&mut a.blur, b, b.name());
                        }
                    });
                ui.end_row();
            }
            if let Fill::Colour(c) = a.fill {
                ui.label(tr!(Colour));
                let mut rgb = [c.0, c.1, c.2];
                if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
                    a.fill = Fill::Colour(Rgb(rgb[0], rgb[1], rgb[2]));
                }
                ui.end_row();
            }
        });
}

/// The two numbers of a custom ratio and the button that uses them (REQ-285): a ratio from
/// 1:4 to 4:1 is used, anything else is explained and nothing changes.
fn custom_ratio(ui: &mut Ui, a: &mut crate::frame::FrameAspect, salt: &str) {
    // one state for each place that shows it (the whole video's popup and a clip's row)
    let id = egui::Id::unique(("custom-ratio", salt));
    let (mut w, mut h): (u32, u32) = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| {
        let (w, h) = a.shape.ratio();
        (w, h)
    });
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut w).range(1..=100));
        ui.monospace(":");
        ui.add(egui::DragValue::new(&mut h).range(1..=100));
        match crate::frame::Shape::custom(w, h) {
            Some(shape) => {
                if ui.button(tr!(UseThisRatio)).clicked() {
                    a.shape = shape;
                }
            }
            None => {
                ui.colored_label(ui.visuals().warn_fg_color, crate::frame::CUSTOM_RATIO_NOTE);
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(id, (w, h)));
}

/// The body of the Scale or Move popup of one clip: its own copy of the effect, edited and
/// removed on that clip only (REQ-148, REQ-200).
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
    let (Some((from, to, start)), FrameTarget::Clip(own)) = (key_range(app, target), target) else {
        ui.label(tr!(ThisClipNoLongerExists));
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
                target: Target::Clip(own),
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
                if on {
                    KEY_MARKER * 0.6
                } else {
                    KEY_MARKER / 2.0
                },
                if on { Color32::YELLOW } else { Color32::WHITE },
                Stroke::new(1.0, Color32::BLACK),
            );
        }
    }
    if resp.clicked()
        && let Some(p) = resp.interact_pointer_pos()
    {
        resp.request_focus();
        let x = p.x - rect.left();
        match bar.key_at(x, &keys) {
            Some(k) => cmds.push(Command::SelectKey(Some(k))),
            None => {
                cmds.push(Command::SelectKey(None));
                cmds.push(Command::Seek(start + (bar.time_at(x) - from)));
            }
        }
    }
    // with the focus in the popup, Left and Right move the selected keyframe by one frame
    let focused = resp.has_focus();
    if focused {
        // the arrow keys are this widget's: egui does not move the focus with them
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                resp.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            );
        });
    }
    let pass = ui.ctx().cumulative_pass_nr();
    ui.ctx()
        .data_mut(|d| d.insert_temp(bar_focus_id(), (pass, focused)));
    if focused && let Some(k) = selected {
        let frame = app.frame_len();
        for (key, dir) in [(egui::Key::ArrowLeft, -1), (egui::Key::ArrowRight, 1)] {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                cmds.push(Command::MoveKey {
                    clip: own,
                    effect: fx,
                    from: k,
                    to: k + Micros(frame.0 * dir),
                });
            }
        }
    }
    resp.on_hover_text(tr!(ClickAKeyframeToSelect));
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
        // how the value moves on to the next keyframe (REQ-282)
        if let Some(at) = selected {
            let eased = app
                .project
                .timeline
                .key_ease(target, fx == FrameFx::Scale, at)
                == crate::keyframes::Ease::EaseInOut;
            let mut on = eased;
            if ui.checkbox(&mut on, tr!(EaseInAndOut2)).changed() {
                cmds.push(Command::SetKeyEase {
                    target,
                    effect: fx,
                    ease: if on {
                        crate::keyframes::Ease::EaseInOut
                    } else {
                        crate::keyframes::Ease::Straight
                    },
                });
            }
        }
    });
    key_time_field(ui, app, own, fx, selected, from, to, cmds);
    copy_paste(ui, app, own, fx, cmds);
    values(ui, app, cache, target, fx, selected, cmds);
    button(
        ui,
        app,
        "Remove effect",
        Command::RemoveFrameEffect { target, effect: fx },
        cmds,
    );
}

/// The time of the selected keyframe, typed in seconds of the clip (REQ-203).
#[allow(clippy::too_many_arguments)]
fn key_time_field(
    ui: &mut Ui,
    app: &AppState,
    clip: crate::timeline::ClipId,
    fx: FrameFx,
    selected: Option<Micros>,
    from: Micros,
    to: Micros,
    cmds: &mut Vec<Command>,
) {
    let Some(k) = selected else { return };
    let mut secs = (k - from).as_secs_f64();
    let max = (to - from).as_secs_f64();
    ui.horizontal(|ui| {
        ui.label(tr!(KeyframeTime));
        let r = ui.add(
            egui::DragValue::new(&mut secs)
                .range(0.0..=max)
                .speed(0.02)
                .max_decimals(3)
                .suffix(" s"),
        );
        if r.drag_stopped() || r.lost_focus() {
            let to = from + Micros::from_secs_f64(secs);
            if to != k {
                cmds.push(Command::MoveKey {
                    clip,
                    effect: fx,
                    from: k,
                    to,
                });
            }
        }
    });
    let _ = app;
}

/// "Copy keyframes" and the list "Paste keyframes to" the other clips (REQ-204).
fn copy_paste(
    ui: &mut Ui,
    app: &AppState,
    clip: crate::timeline::ClipId,
    fx: FrameFx,
    cmds: &mut Vec<Command>,
) {
    ui.horizontal_wrapped(|ui| {
        button(
            ui,
            app,
            "Copy keyframes",
            Command::CopyKeys { clip, effect: fx },
            cmds,
        );
        let tl = &app.project.timeline;
        ui.add_enabled_ui(app.clipboard_keys.is_some(), |ui| {
            ui.menu_button(tr!(PasteKeyframesTo), |ui| {
                for (i, c) in tl.clips.iter().enumerate().filter(|(_, c)| c.id != clip) {
                    let name = app
                        .project
                        .media(c.media)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "?".into());
                    if ui.button(format!("{}. {name}", i + 1)).clicked() {
                        cmds.push(Command::PasteKeys {
                            clip: c.id,
                            effect: fx,
                        });
                        ui.close();
                    }
                }
            });
        });
    });
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
            ui.label(tr!(Scale));
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
