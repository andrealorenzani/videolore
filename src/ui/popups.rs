// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Floating windows that do not block the editor (REQ-076, REQ-077, REQ-078): Settings,
//! the effect popups, Clip details, the settings of one fade or line, the keyboard
//! shortcuts, and the chooser of overlapping markers. Each window has a fixed id (its
//! position is kept for the session), a close button, closes with Esc, stays inside the
//! window and scrolls when its content does not fit.

use super::strings::tr;
use super::{UiCache, button, effects, keymap, scroll_body, settings};
use crate::app::{AppState, Command, Edge, Instance, Popup};
use crate::time::{Micros, timecode};
use eframe::egui::{self, Align2, Button, Context, Id, Order, ScrollArea, Ui};

/// Default width of a popup (fits the smallest window).
const WIDTH: f32 = 340.0;

/// Draws every open popup and the marker chooser.
pub fn windows(ctx: &Context, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let open = app.popups.open.clone();
    if !app.popups.is_open(Popup::Text) {
        cache.text_draft = None;
        cache.text_edit = None;
        cache.max_text = None;
    }
    if !app.popups.is_open(Popup::Splash) {
        cache.splash_rects = None;
    }
    for (n, p) in open.into_iter().enumerate() {
        window(ctx, app, cache, cmds, p, n);
    }
    chooser(ctx, app, cache, cmds);
}

/// The icon in the title of a popup (REQ-142).
pub fn popup_icon(app: &AppState, p: Popup) -> &'static str {
    use super::icons as i;
    match p {
        Popup::Settings => i::GEAR,
        Popup::Fades => i::CIRCLE_HALF,
        Popup::Subtitles => i::SUBTITLES,
        Popup::Volume => i::SPEAKER_HIGH,
        Popup::CleanAudio => i::WAVEFORM,
        Popup::ClipDetails => i::SLIDERS,
        Popup::Shortcuts => i::KEYBOARD,
        Popup::Text => i::TEXT_T,
        Popup::Splash => i::INFO,
        Popup::Instance => match app.popups.instance {
            Some(Instance::Fade(..) | Instance::FadeStart | Instance::FadeEnd) => i::CIRCLE_HALF,
            Some(Instance::Line(..)) => i::SUBTITLES,
            Some(Instance::ClipVolume(_)) => i::SPEAKER_HIGH,
            Some(Instance::CleanAudio(_)) => i::WAVEFORM,
            Some(Instance::Pov(..)) => i::VIDEO_CAMERA,
            Some(Instance::Aspect) => i::FRAME_CORNERS,
            Some(Instance::Scale(_)) => i::ARROWS_OUT,
            Some(Instance::Move(_)) => i::ARROWS_OUT_CARDINAL,
            Some(Instance::Lut(_)) => i::PALETTE,
            None => i::LIST,
        },
    }
}

fn window(
    ctx: &Context,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    p: Popup,
    n: usize,
) {
    if p == Popup::Splash {
        super::splash_ui::show(ctx, app, cache, cmds);
        return;
    }
    let screen = ctx.content_rect();
    let mut open = true;
    let width = if p == Popup::Settings { 460.0 } else { WIDTH };
    let track = cache.sections.map(|s| s.timeline).unwrap_or(screen);
    let width = super::place::fit_width(screen, track, width);
    let width = width.min(screen.width() - 24.0).max(120.0);
    // where it opens: where the user left it (REQ-235), else clear of the tracks (REQ-236)
    let name = format!("{p:?}");
    let remembered = app
        .settings
        .interface
        .popups
        .get(&name)
        .and_then(|xy| super::place::saved(*xy, egui::vec2(width, 120.0), screen));
    let place = super::place::default_place(screen, track, width, n);
    let max_h = if remembered.is_some() {
        (screen.height() - 90.0).max(80.0)
    } else {
        place.max_height
    };
    let start = remembered.or(place.pos);
    let mut window =
        egui::Window::new(super::icons::label(popup_icon(app, p), &app.popup_title(p)))
            .id(keymap::popup_id(p))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .constrain(true);
    window = match start {
        Some(pos) => window.pivot(Align2::LEFT_TOP).default_pos(pos),
        None => window
            .pivot(Align2::CENTER_CENTER)
            .default_pos(screen.center() + egui::vec2(24.0, 24.0) * (n % 5) as f32),
    };
    let shown = window
        .default_width(width)
        .max_width(width)
        .show(ctx, |ui| {
            let area = ScrollArea::vertical()
                .id_salt(("popup-scroll", p))
                .max_height(max_h)
                .auto_shrink([false, true]);
            scroll_body(ui, cache, area, |ui, cache| match p {
                Popup::Settings => settings::settings_body(ui, app, cmds),
                Popup::Fades => effects::fades_body(ui, app, cmds),
                Popup::Subtitles => effects::subtitles_body(ui, app, cache, cmds),
                Popup::Volume => effects::volume_body(ui, app, cache, cmds),
                Popup::CleanAudio => effects::clean_audio_body(ui, app, cmds),
                Popup::ClipDetails => clip_details_body(ui, app, cache, cmds),
                Popup::Instance => instance_body(ui, app, cache, cmds),
                Popup::Shortcuts => shortcuts_body(ui, app, cmds),
                Popup::Text => super::text_popup::text_body(ui, app, cache, cmds),
                Popup::Splash => {}
            });
        });
    // a popup the user moved is remembered where it was left (REQ-235): its place when
    // it was last at rest is compared with its place when the pointer is released
    if let Some(r) = shown {
        let min = r.response.rect.min;
        let key = Id::unique(("popup-rest", p));
        let rest: Option<egui::Pos2> = ctx.data(|d| d.get_temp(key));
        match rest {
            None => {
                ctx.data_mut(|d| d.insert_temp(key, min));
            }
            Some(at) if ctx.input(|i| i.pointer.any_released()) => {
                if open && (min - at).abs().max_elem() > 1.0 {
                    cmds.push(Command::PopupMoved(name, (min.x, min.y)));
                    ctx.data_mut(|d| d.insert_temp(key, min));
                }
            }
            Some(_) => {}
        }
    }
    if !open {
        cmds.push(Command::ClosePopup(p));
    }
}

fn target_name(app: &AppState, t: crate::timeline::FrameTarget) -> String {
    match t {
        crate::timeline::FrameTarget::Whole => "the whole video".into(),
        crate::timeline::FrameTarget::Clip(c) => {
            app.clip_name(c).unwrap_or_else(|| "a clip".into())
        }
    }
}

/// Text of a fade or line in the chooser.
pub fn instance_label(app: &AppState, i: Instance) -> String {
    match i {
        Instance::Fade(l, r) => match app.boundary_of(l, r) {
            Some(b) => format!("Fade {}", effects::fade_text(app, b)),
            None => "Fade".into(),
        },
        Instance::Line(id, clip) => app
            .project
            .subtitles
            .project(&app.project.timeline)
            .into_iter()
            .find(|x| x.line == id && x.clip == clip)
            .map(|x| format!("Subtitle {}  {}", timecode(x.start), x.text))
            .unwrap_or_else(|| "Subtitle".into()),
        Instance::FadeStart => "Fade from black at the start".into(),
        Instance::FadeEnd => "Fade to black at the end".into(),
        Instance::ClipVolume(c) => format!(
            "Volume of {}",
            app.clip_name(c).unwrap_or_else(|| "a clip".into())
        ),
        Instance::CleanAudio(c) => format!(
            "Clean audio of {}",
            app.clip_name(c).unwrap_or_else(|| "a clip".into())
        ),
        Instance::Aspect => "Change aspect".into(),
        Instance::Scale(t) => format!("Scale of {}", target_name(app, t)),
        Instance::Move(t) => format!("Move of {}", target_name(app, t)),
        Instance::Lut(t) => format!("Apply LUT of {}", target_name(app, t)),
        Instance::Pov(m, t) => {
            let at = app.pov_position(m, t).map(timecode).unwrap_or_default();
            format!("360° point of view {at}")
        }
    }
}

/// Mouse gestures listed in the shortcuts window (they are not keyboard shortcuts).
pub const GESTURES: [(&str, &str); 3] = [
    (
        "Ctrl/Cmd+click",
        "Add a subtitle line to the selection, or take it out",
    ),
    (
        "Shift+drag",
        "Select the subtitle lines starting in a part of the timeline",
    ),
    (
        "Shift+Ctrl/Cmd+drag",
        "Add the lines starting in a part of the timeline to the selection",
    ),
];

/// The small list shown when several markers are under the pointer.
fn chooser(ctx: &Context, app: &AppState, cache: &UiCache, cmds: &mut Vec<Command>) {
    let Some(list) = &app.popups.chooser else {
        return;
    };
    let pos = cache
        .chooser_pos
        .unwrap_or_else(|| ctx.content_rect().center());
    let r = egui::Area::new(Id::unique("videolore-chooser"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(320.0);
                ui.weak("Several markers here:");
                for i in list {
                    let text = instance_label(app, *i);
                    if ui
                        .add(Button::new(text.as_str()).truncate())
                        .on_hover_text(text)
                        .clicked()
                    {
                        cmds.push(Command::OpenInstance(*i));
                    }
                }
            });
        });
    if r.response.clicked_elsewhere() {
        cmds.push(Command::ChooseInstance(Vec::new()));
    }
}

/// The keyboard shortcuts window: exactly the bound shortcuts, each with a button to
/// change it by pressing the new keys (REQ-242), and Reset shortcuts.
pub fn shortcuts_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let over = keymap::current(ui.ctx());
    if let Some((name, conflict)) = &app.rebind {
        ui.horizontal(|ui| {
            ui.strong(format!("{} {}", tr!(PressTheNewKeysFor), name));
            if ui.button(tr!(Cancel)).clicked() {
                cmds.push(Command::CancelRebind);
            }
        });
        if let Some(by) = conflict {
            ui.colored_label(
                egui::Color32::from_rgb(230, 160, 40),
                format!("{} {by}. {}", tr!(ThoseKeysAreUsedBy), tr!(PressOtherKeys)),
            );
        }
    }
    egui::Grid::new("shortcuts-grid")
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            for (id, s, label) in keymap::bindings_in(&over) {
                ui.monospace(ui.ctx().format_shortcut(&s));
                ui.label(label);
                if let Some(id) = id {
                    let r = ui.small_button(tr!(Change));
                    let r = r.on_hover_text(tr!(PressNewKeysToChange));
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::Button,
                            true,
                            format!("Change {label}"),
                        )
                    });
                    if r.clicked() {
                        cmds.push(Command::StartRebind(id.name()));
                    }
                }
                ui.end_row();
            }
            for (gesture, label) in GESTURES {
                ui.monospace(gesture);
                ui.label(label);
                ui.end_row();
            }
        });
    if ui.button(tr!(ResetShortcuts)).clicked() {
        cmds.push(Command::ResetShortcuts);
    }
}

/// Settings of the fade or subtitle line the user clicked.
pub fn instance_body(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    match app.popups.instance {
        None => {
            ui.label(tr!(ClickAMarkerOnThe));
        }
        Some(Instance::Fade(l, r)) => fade_instance(ui, app, l, r, cmds),
        Some(Instance::Line(id, clip)) => line_instance(ui, app, cache, id, clip, cmds),
        Some(Instance::FadeStart) => effects::edge_fade_body(ui, app, Edge::Start, cmds),
        Some(Instance::FadeEnd) => effects::edge_fade_body(ui, app, Edge::End, cmds),
        Some(Instance::ClipVolume(c)) => effects::clip_volume_body(ui, app, cache, c, cmds),
        Some(Instance::CleanAudio(c)) => effects::clean_audio_clip_body(ui, app, c, cmds),
        Some(Instance::Pov(m, t)) => pov_instance(ui, app, cache, m, t, cmds),
        Some(Instance::Aspect) => super::frame_popups::aspect_body(ui, app, cmds),
        Some(Instance::Scale(t)) => {
            super::frame_popups::key_body(ui, app, cache, t, crate::app::FrameFx::Scale, cmds)
        }
        Some(Instance::Move(t)) => {
            super::frame_popups::key_body(ui, app, cache, t, crate::app::FrameFx::Move, cmds)
        }
        Some(Instance::Lut(t)) => super::lut_popup::lut_body(ui, app, t, cmds),
    }
}

/// The popup of a point of view: its time and its view as numbers, and Remove (REQ-102).
/// The numbers are committed as one undo step when an edit ends.
fn pov_instance(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    media: crate::media::MediaId,
    t: Micros,
    cmds: &mut Vec<Command>,
) {
    let Some(p) = app.path_at(media, t).iter().find(|p| p.t == t).copied() else {
        ui.label(tr!(ThisPointOfViewNo));
        return;
    };
    let name = app
        .project
        .media(media)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    ui.strong(tr!(N360PointOfView));
    ui.add(egui::Label::new(format!("Source: {name}")).truncate());
    ui.weak("The camera moves from one point of view to the next, in a straight line unless you choose to ease in and out.");
    // values being edited (kept between frames until the edit ends)
    let mut v = match &cache.pov_edit {
        Some((m, old, v)) if *m == media && *old == t => *v,
        _ => [
            t.as_secs_f64(),
            p.cam.yaw_deg(),
            p.cam.pitch_deg(),
            p.cam.fov_deg(),
            p.cam.roll_deg(),
        ],
    };
    let mut changed = false;
    let mut done = false;
    egui::Grid::new("pov-grid").num_columns(2).show(ui, |ui| {
        let max = app
            .project
            .media(media)
            .map_or(0.0, |m| m.duration.as_secs_f64());
        for (label, i, range, suffix) in [
            ("Time (source)", 0, 0.0..=max, " s"),
            ("Direction (yaw)", 1, -180.0..=180.0, "°"),
            ("Up/down (pitch)", 2, -90.0..=90.0, "°"),
            ("Zoom (field of view)", 3, 30.0..=110.0, "°"),
            ("Roll (tilt)", 4, -180.0..=180.0, "°"),
        ] {
            ui.label(label);
            let r = ui.add(
                egui::DragValue::new(&mut v[i])
                    .range(range)
                    .speed(if i == 0 { 0.01 } else { 0.5 })
                    .max_decimals(if i == 0 { 3 } else { 2 })
                    .suffix(suffix),
            );
            changed |= r.changed();
            done |= r.drag_stopped() || r.lost_focus();
            ui.end_row();
        }
    });
    // how the camera moves on to the next point of view (REQ-271)
    let mut eased = p.ease == crate::camera::Ease::EaseInOut;
    let ease_changed = ui.checkbox(&mut eased, tr!(EaseInAndOut)).changed();
    if changed {
        cache.pov_edit = Some((media, t, v));
    }
    if ease_changed || done || (changed && !ui.input(|i| i.pointer.any_down())) {
        cache.pov_edit = None;
        let pov = crate::camera::Pov {
            t: Micros::from_secs_f64(v[0]),
            cam: crate::camera::Camera::new(
                (v[1] * 100.0).round() as i64,
                (v[2] * 100.0).round() as i64,
                (v[3] * 100.0).round() as i64,
            )
            .with_roll((v[4] * 100.0).round() as i64),
            ease: if eased {
                crate::camera::Ease::EaseInOut
            } else {
                crate::camera::Ease::Straight
            },
        };
        if pov != p {
            cmds.push(Command::EditPov {
                media,
                old_t: t,
                pov,
            });
        }
    }
    button(ui, app, "Remove", Command::RemovePov { media, t }, cmds);
}

fn fade_instance(
    ui: &mut Ui,
    app: &AppState,
    l: crate::timeline::ClipId,
    r: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let Some(b) = app.boundary_of(l, r).filter(|b| tl.has_fade(*b)) else {
        ui.label(tr!(ThisFadeNoLongerExists));
        return;
    };
    ui.strong(tr!(FadeINOUT));
    let name = |i: usize| {
        app.project
            .media(tl.clips[i].media)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };
    ui.add(egui::Label::new(format!("Between \"{}\" and \"{}\"", name(b), name(b + 1))).truncate());
    ui.label(format!("At {}", timecode(tl.start_of(b + 1))));
    ui.weak("Picture and sound fade together. The length is set in the Fades window.");
    // which side goes through black (REQ-275)
    ui.horizontal(|ui| {
        for (side, text) in [
            (crate::timeline::FadeSide::Both, tr!(FadeBothSides)),
            (crate::timeline::FadeSide::OutOnly, tr!(FadeOutOnly)),
            (crate::timeline::FadeSide::InOnly, tr!(FadeInOnly)),
        ] {
            if ui.radio(tl.fade_side(b) == side, text).clicked() {
                cmds.push(Command::SetFadeSide {
                    left: l,
                    right: r,
                    side,
                });
            }
        }
    });
    button(
        ui,
        app,
        "Remove",
        Command::SetFade {
            left: l,
            right: r,
            on: false,
        },
        cmds,
    );
}

fn line_instance(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    id: crate::subtitles::LineId,
    clip: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    let shown = app
        .project
        .subtitles
        .project(&app.project.timeline)
        .into_iter()
        .find(|x| x.line == id && x.clip == clip);
    let Some(l) = shown else {
        ui.label(tr!(ThisSubtitleLineNoLonger));
        return;
    };
    ui.strong(tr!(SubtitleLine));
    ui.label(format!("{} – {}", timecode(l.start), timecode(l.end)));
    let text = match &mut cache.line_edit {
        Some((x, t)) if *x == id => t,
        _ => {
            let full = app
                .project
                .subtitles
                .get(id)
                .map(|x| x.text.clone())
                .unwrap_or_default();
            cache.line_edit = Some((id, full));
            &mut cache.line_edit.as_mut().expect("just set").1
        }
    };
    super::scripts::install_for(
        ui.ctx(),
        &super::scripts::scripts_of_lines(std::iter::once(text.as_str())),
    );
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    if r.lost_focus() {
        cmds.push(Command::SetSubtitleText(id, text.clone()));
    }
    if let Some(note) = crate::fonts::missing_note(text) {
        ui.label(super::icons::label(super::icons::WARNING, &note));
    }
    if !r.has_focus() && !r.lost_focus() {
        // not being edited: show the stored text (it may change with undo)
        cache.line_edit = None;
    }
    let retime = |start: Micros, end: Micros| Command::RetimeSubtitle {
        line: id,
        clip,
        start,
        end,
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(Start));
        button(
            ui,
            app,
            "−0.1 s",
            retime(l.start - effects::NUDGE, l.end),
            cmds,
        );
        button(
            ui,
            app,
            "+0.1 s",
            retime(l.start + effects::NUDGE, l.end),
            cmds,
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(End));
        button(
            ui,
            app,
            "−0.1 s",
            retime(l.start, l.end - effects::NUDGE),
            cmds,
        );
        button(
            ui,
            app,
            "+0.1 s",
            retime(l.start, l.end + effects::NUDGE),
            cmds,
        );
    });
    ui.horizontal_wrapped(|ui| {
        button(ui, app, "Play this line", Command::PlayLine(id), cmds);
        button(ui, app, "Delete", Command::DeleteSubtitle(id), cmds);
    });
    let own = l.position.is_some();
    // with several lines selected, the reset applies to all of them
    let ids = if app.selection.contains(&id) && app.selection.len() > 1 {
        app.selection.iter().copied().collect()
    } else {
        vec![id]
    };
    let label = if ids.len() > 1 {
        "Reset position of the selected lines"
    } else {
        "Reset position"
    };
    let r = ui
        .add_enabled(own || ids.len() > 1, Button::new(label))
        .on_hover_text(tr!(BackToThePlaceOf))
        .on_disabled_hover_text(tr!(TheLineIsAtThe));
    if r.clicked() {
        cmds.push(Command::SetLinePositions { ids, pos: None });
    }
    ui.weak("Drag the subtitle in the preview to move it.");
}

/// Clip details: the selected video clip and the selected music clip (it follows the
/// selection).
pub fn clip_details_body(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let video = app.selected_clip.and_then(|id| tl.index_of(id));
    let music = app
        .selected_music
        .filter(|id| app.project.music.get(*id).is_some());
    if video.is_none() && music.is_none() {
        ui.label(tr!(SelectAClipOnThe));
    }
    if let Some(i) = video {
        clip_settings(ui, app, cache, i, cmds);
    }
    if music.is_some() {
        if video.is_some() {
            ui.separator();
        }
        music_clip_settings(ui, app, cmds);
    }
}

fn clip_settings(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    i: usize,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let c = tl.clips[i];
    let start = tl.start_of(i);
    let name = app
        .project
        .media(c.media)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    ui.strong(tr!(VideoClip));
    ui.add(egui::Label::new(format!("Source: {name}")).truncate())
        .on_hover_text(&name);
    // a plain 2:1 video can be marked as 360° video (REQ-263)
    if let Some(m) = app.project.media(c.media)
        && m.format.can_be_marked()
        && m.format.spherical != Some(true)
    {
        let mut on = m.format.marked_360;
        if ui
            .checkbox(&mut on, tr!(N360Video3))
            .on_hover_text(tr!(ShowThisVideoThroughTheCamera))
            .changed()
        {
            cmds.push(Command::SetMarked360(c.media, on));
        }
    }
    // volume keyframes (REQ-280): the level at the playhead, and the keys of the clip
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(VolumeKeys));
        let at = (app.playhead - start).clamp(Micros::ZERO, c.len());
        let now_level =
            f32::from(crate::keyframes::at(tl.volume_keys(c.id), at).unwrap_or(c.volume));
        let mut v = now_level;
        let r = ui.add(
            egui::Slider::new(&mut v, 0.0..=100.0)
                .step_by(1.0)
                .suffix(" %"),
        );
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && v.round() != now_level {
            cmds.push(Command::SetVolumeKey {
                clip: c.id,
                at,
                percent: v.round() as u8,
            });
        }
        for k in tl.volume_keys(c.id) {
            if ui
                .small_button(format!("{} {}% ✕", timecode(k.t), k.v))
                .on_hover_text(tr!(RemoveThisKeyframe))
                .clicked()
            {
                cmds.push(Command::RemoveVolumeKey {
                    clip: c.id,
                    at: k.t,
                });
            }
        }
    });
    // the clip's own Change aspect (REQ-286)
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(ClipAspect));
        let own = tl.own_aspect(c.id);
        if ui.selectable_label(own.is_none(), tr!(AspectOff)).clicked() && own.is_some() {
            cmds.push(Command::SetClipAspect {
                clip: c.id,
                aspect: None,
            });
        }
        for shape in crate::frame::Shape::ALL {
            let on = own.is_some_and(|a| a.shape == shape);
            if ui.selectable_label(on, shape.name()).clicked() && !on {
                let aspect = crate::frame::FrameAspect {
                    shape,
                    fit: crate::frame::Fit::Height,
                    ..own.unwrap_or_default()
                };
                cmds.push(Command::SetClipAspect {
                    clip: c.id,
                    aspect: Some(aspect),
                });
            }
        }
    });
    // part of the picture cut away (REQ-283)
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(CropPercent));
        let mut crop = c.crop;
        let mut changed = false;
        for (name, v) in [
            (tr!(CropLeft), &mut crop.left),
            (tr!(CropTop), &mut crop.top),
            (tr!(CropRight), &mut crop.right),
            (tr!(CropBottom), &mut crop.bottom),
        ] {
            let r = ui.add(
                egui::DragValue::new(v)
                    .range(0..=crate::timeline::MAX_CROP)
                    .prefix(format!("{name} ")),
            );
            changed |= r.drag_stopped() || (r.changed() && !r.dragged());
        }
        if changed && crop != c.crop {
            cmds.push(Command::SetCrop { clip: c.id, crop });
        }
    });
    // playing speed (REQ-281)
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(SpeedPercent));
        let mut pct = u32::from(c.speed) / 10;
        let r = ui.add(egui::DragValue::new(&mut pct).range(50..=200));
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && pct * 10 != u32::from(c.speed) {
            cmds.push(Command::SetSpeed {
                clip: c.id,
                speed: (pct * 10) as u16,
            });
        }
    });
    ui.label(format!(
        "In {}  Out {}  Length {}",
        timecode(c.src_in),
        timecode(c.src_out),
        timecode(c.len())
    ));
    if app
        .project
        .media(c.media)
        .is_some_and(|m| m.format.is_360())
    {
        // information only: a clip is 360 when its file says so (REQ-100)
        ui.label(tr!(N360Video2)).on_hover_text(
            "This file carries 360° video information, so it is shown through a camera \
             you can turn.",
        );
    }
    let frame = app.fps().frame_duration();
    let inside = app.playhead > start && app.playhead < start + c.len();
    let offset = app.playhead - start;
    let trim = |src_in: Micros, src_out: Micros| Command::SetTrim {
        clip: c.id,
        src_in,
        src_out,
    };
    let at_playhead = "Move the playhead inside this clip first";
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(TrimIn));
        button(ui, app, "-1f", trim(c.src_in - frame, c.src_out), cmds);
        button(ui, app, "+1f", trim(c.src_in + frame, c.src_out), cmds);
        if ui
            .add_enabled(inside, Button::new(tr!(AtPlayhead)))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(c.src_at(offset), c.src_out));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(TrimOut));
        button(ui, app, "-1f", trim(c.src_in, c.src_out - frame), cmds);
        button(ui, app, "+1f", trim(c.src_in, c.src_out + frame), cmds);
        if ui
            .add_enabled(inside, Button::new(tr!(AtPlayhead)))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(c.src_in, c.src_at(offset)));
        }
    });
    ui.horizontal_wrapped(|ui| {
        if i > 0 {
            let mut on = tl.has_fade(i - 1);
            if ui.checkbox(&mut on, "Fade in").changed() {
                cmds.push(Command::SetFade {
                    left: tl.clips[i - 1].id,
                    right: c.id,
                    on,
                });
            }
        }
        if i + 1 < tl.clips.len() {
            let mut on = tl.has_fade(i);
            if ui.checkbox(&mut on, "Fade out").changed() {
                cmds.push(Command::SetFade {
                    left: c.id,
                    right: tl.clips[i + 1].id,
                    on,
                });
            }
        }
    });
    effects::clip_volume_controls(ui, app, cache, c.id, cmds);
    if tl.can_merge(c.id) {
        button(ui, app, "Merge with next", Command::Merge, cmds);
    } else {
        let why = tl
            .merge_block_reason(c.id)
            .unwrap_or("Only two parts of the same clip, next to each other, can be merged");
        ui.add_enabled(false, Button::new(tr!(MergeWithNext)))
            .on_disabled_hover_text(why);
    }
}

fn music_clip_settings(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let Some(m) = app.selected_music.and_then(|id| app.project.music.get(id)) else {
        return;
    };
    ui.strong(tr!(MusicClip));
    let name = app
        .project
        .media(m.media)
        .map(|x| x.name.clone())
        .unwrap_or_default();
    ui.add(
        egui::Label::new(format!(
            "{name}: at {}  length {}",
            timecode(m.start),
            timecode(m.len())
        ))
        .truncate(),
    )
    .on_hover_text(&name);
    let frame = app.fps().frame_duration();
    let inside = app.playhead > m.start && app.playhead < m.end();
    let offset = app.playhead - m.start;
    let trim = |src_in: Micros, src_out: Micros| Command::TrimMusic {
        clip: m.id,
        src_in,
        src_out,
    };
    let at_playhead = "Move the playhead inside this clip first";
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(Start2));
        button(ui, app, "-1f", trim(m.src_in - frame, m.src_out), cmds);
        button(ui, app, "+1f", trim(m.src_in + frame, m.src_out), cmds);
        if ui
            .add_enabled(inside, Button::new(tr!(AtPlayhead)))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(m.src_in + offset, m.src_out));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(End2));
        button(ui, app, "-1f", trim(m.src_in, m.src_out - frame), cmds);
        button(ui, app, "+1f", trim(m.src_in, m.src_out + frame), cmds);
        if ui
            .add_enabled(inside, Button::new(tr!(AtPlayhead)))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(m.src_in, m.src_in + offset));
        }
    });
    ui.horizontal_wrapped(|ui| {
        button(
            ui,
            app,
            "Move to playhead",
            Command::MoveMusic {
                clip: m.id,
                start: app.playhead,
            },
            cmds,
        );
        button(ui, app, "Split", Command::SplitMusic, cmds);
        button(ui, app, "Delete", Command::DeleteMusic, cmds);
    });
    // the clip's own level, mute, fades and Clean audio (REQ-276, REQ-277)
    {
        let (mut volume, mut muted, mut clean) = (f32::from(m.volume), m.muted, m.clean_audio);
        let (mut fin, mut fout) = (m.fade_in.as_secs_f64(), m.fade_out.as_secs_f64());
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(tr!(ClipLevel));
            let r = ui.add(
                egui::Slider::new(&mut volume, 0.0..=100.0)
                    .step_by(1.0)
                    .suffix(" %"),
            );
            changed |= r.drag_stopped() || (r.changed() && !r.dragged());
            changed |= ui.checkbox(&mut muted, tr!(Mute)).changed();
            changed |= ui.checkbox(&mut clean, tr!(CleanAudio)).changed();
        });
        ui.horizontal_wrapped(|ui| {
            for (label, v) in [
                (tr!(FadeInSeconds), &mut fin),
                (tr!(FadeOutSeconds), &mut fout),
            ] {
                ui.label(label);
                let r = ui.add(
                    egui::DragValue::new(v)
                        .range(0.0..=60.0)
                        .speed(0.05)
                        .suffix(" s"),
                );
                changed |= r.drag_stopped() || (r.changed() && !r.dragged());
            }
        });
        if changed {
            cmds.push(Command::SetMusicEffects {
                clip: m.id,
                volume: volume.round() as u8,
                muted,
                fade_in: Micros::from_secs_f64(fin),
                fade_out: Micros::from_secs_f64(fout),
                clean_audio: clean,
            });
        }
    }
}
