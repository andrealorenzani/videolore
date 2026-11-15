// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Floating windows that do not block the editor (REQ-076, REQ-077, REQ-078): Settings,
//! the effect popups, Clip details, the settings of one fade or line, the keyboard
//! shortcuts, and the chooser of overlapping markers. Each window has a fixed id (its
//! position is kept for the session), a close button, closes with Esc, stays inside the
//! window and scrolls when its content does not fit.

use super::{UiCache, button, effects, keymap, scroll_body, settings};
use crate::app::{AppState, Command, Instance, Popup};
use crate::time::{Micros, timecode};
use eframe::egui::{self, Align2, Button, Context, Id, Order, ScrollArea, Ui};

/// Default width of a popup (fits the smallest window).
const WIDTH: f32 = 340.0;

/// Draws every open popup and the marker chooser.
pub fn windows(ctx: &Context, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let open = app.popups.open.clone();
    for (n, p) in open.into_iter().enumerate() {
        window(ctx, app, cache, cmds, p, n);
    }
    chooser(ctx, app, cache, cmds);
}

fn window(
    ctx: &Context,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    p: Popup,
    n: usize,
) {
    let screen = ctx.content_rect();
    let mut open = true;
    let width = if p == Popup::Settings { 460.0 } else { WIDTH };
    let width = width.min(screen.width() - 24.0).max(120.0);
    let max_h = (screen.height() - 90.0).max(80.0);
    let offset = egui::vec2(24.0, 24.0) * (n % 5) as f32;
    egui::Window::new(p.title())
        .id(keymap::popup_id(p))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .constrain(true)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(screen.center() + offset)
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
                Popup::ClipDetails => clip_details_body(ui, app, cmds),
                Popup::Instance => instance_body(ui, app, cache, cmds),
                Popup::Shortcuts => shortcuts_body(ui),
            });
        });
    if !open {
        cmds.push(Command::ClosePopup(p));
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
    }
}

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

/// The keyboard shortcuts window: exactly the bound shortcuts.
pub fn shortcuts_body(ui: &mut Ui) {
    egui::Grid::new("shortcuts-grid")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (s, label) in keymap::bindings() {
                ui.monospace(ui.ctx().format_shortcut(&s));
                ui.label(label);
                ui.end_row();
            }
        });
}

/// Settings of the fade or subtitle line the user clicked.
pub fn instance_body(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    match app.popups.instance {
        None => {
            ui.label("Click a marker on the timeline.");
        }
        Some(Instance::Fade(l, r)) => fade_instance(ui, app, l, r, cmds),
        Some(Instance::Line(id, clip)) => line_instance(ui, app, cache, id, clip, cmds),
    }
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
        ui.label("This fade no longer exists.");
        return;
    };
    ui.strong("Fade through black");
    let name = |i: usize| {
        app.project
            .media(tl.clips[i].media)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };
    ui.add(egui::Label::new(format!("Between \"{}\" and \"{}\"", name(b), name(b + 1))).truncate());
    ui.label(format!("At {}", timecode(tl.start_of(b + 1))));
    ui.weak("Fixed length; picture and sound fade together.");
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
        ui.label("This subtitle line no longer exists.");
        return;
    };
    ui.strong("Subtitle line");
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
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    if r.lost_focus() {
        cmds.push(Command::SetSubtitleText(id, text.clone()));
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
        ui.label("Start");
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
        ui.label("End");
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
}

/// Clip details: the selected video clip and the selected music clip (it follows the
/// selection).
pub fn clip_details_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let video = app.selected_clip.and_then(|id| tl.index_of(id));
    let music = app
        .selected_music
        .filter(|id| app.project.music.get(*id).is_some());
    if video.is_none() && music.is_none() {
        ui.label("Select a clip on the timeline (double-click opens this window).");
    }
    if let Some(i) = video {
        clip_settings(ui, app, i, cmds);
    }
    if music.is_some() {
        if video.is_some() {
            ui.separator();
        }
        music_clip_settings(ui, app, cmds);
    }
}

fn clip_settings(ui: &mut Ui, app: &AppState, i: usize, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let c = tl.clips[i];
    let start = tl.start_of(i);
    let name = app
        .project
        .media(c.media)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    ui.strong("Video clip");
    ui.add(egui::Label::new(format!("Source: {name}")).truncate())
        .on_hover_text(&name);
    ui.label(format!(
        "In {}  Out {}  Length {}",
        timecode(c.src_in),
        timecode(c.src_out),
        timecode(c.len())
    ));
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
        ui.label("Trim in:");
        button(ui, app, "-1f", trim(c.src_in - frame, c.src_out), cmds);
        button(ui, app, "+1f", trim(c.src_in + frame, c.src_out), cmds);
        if ui
            .add_enabled(inside, Button::new("at playhead"))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(c.src_in + offset, c.src_out));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Trim out:");
        button(ui, app, "-1f", trim(c.src_in, c.src_out - frame), cmds);
        button(ui, app, "+1f", trim(c.src_in, c.src_out + frame), cmds);
        if ui
            .add_enabled(inside, Button::new("at playhead"))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(c.src_in, c.src_in + offset));
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
    if tl.can_merge(c.id) {
        button(ui, app, "Merge with next", Command::Merge, cmds);
    } else {
        ui.add_enabled(false, Button::new("Merge with next"))
            .on_disabled_hover_text(
                "Only two parts of the same clip, next to each other, can be merged",
            );
    }
}

fn music_clip_settings(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let Some(m) = app.selected_music.and_then(|id| app.project.music.get(id)) else {
        return;
    };
    ui.strong("Music clip");
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
        ui.label("Start:");
        button(ui, app, "-1f", trim(m.src_in - frame, m.src_out), cmds);
        button(ui, app, "+1f", trim(m.src_in + frame, m.src_out), cmds);
        if ui
            .add_enabled(inside, Button::new("at playhead"))
            .on_disabled_hover_text(at_playhead)
            .clicked()
        {
            cmds.push(trim(m.src_in + offset, m.src_out));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("End:");
        button(ui, app, "-1f", trim(m.src_in, m.src_out - frame), cmds);
        button(ui, app, "+1f", trim(m.src_in, m.src_out + frame), cmds);
        if ui
            .add_enabled(inside, Button::new("at playhead"))
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
}
