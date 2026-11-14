// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The egui user interface. Panels never mutate the state directly: they push
//! [`Command`]s, which are run by [`AppState::dispatch`] at the end of the frame.

pub mod effects;
pub mod keymap;
pub mod layout;
pub mod media;
pub mod modals;
pub mod settings;
pub mod timeline;
pub mod zoom;

use crate::app::{AppState, Command};
use crate::music::Mix;
use crate::subtitles::LineId;
use crate::time::{Micros, timecode};
use eframe::egui::{
    self, Button, Color32, ColorImage, Context, Rect, Sense, TextureHandle, TextureOptions, Ui,
    UiBuilder, Vec2, pos2, vec2,
};
use std::time::{Duration, Instant};

/// UI-only state that is not part of the application state.
#[derive(Default)]
pub struct UiCache {
    texture: Option<TextureHandle>,
    serial: u64,
    title: String,
    /// Volumes while a slider is dragged (committed as one undo step on release).
    pub mix_drag: Option<Mix>,
    /// Text of the subtitle line being edited.
    pub line_edit: Option<(LineId, String)>,
    /// The About text (built when the dialog opens).
    pub about: Option<String>,
}

impl UiCache {
    pub fn has_texture(&self) -> bool {
        self.texture.is_some()
    }
}

/// The eframe application.
pub struct VideoloreApp {
    pub state: AppState,
    pub cache: UiCache,
}

impl eframe::App for VideoloreApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        frame_ui(ui, &mut self.state, &mut self.cache, Instant::now());
    }
}

/// True while a dialog covers the window (shortcuts are off).
pub fn modal_open(app: &AppState) -> bool {
    (app.ffmpeg.is_err() && !app.ffmpeg_checking) || app.notice.is_some() || app.modal.is_some()
}

/// Draws one frame into the root `ui` and runs the commands it produced.
pub fn frame_ui(root: &mut Ui, app: &mut AppState, cache: &mut UiCache, now: Instant) {
    let ctx = root.ctx().clone();
    let ctx = &ctx;
    app.tick(now);
    update_texture(ctx, app, cache);
    let title = app.title();
    if title != cache.title {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        cache.title = title;
    }

    let mut cmds = if modal_open(app) {
        Vec::new()
    } else {
        keymap::collect(ctx)
    };
    // The window's close button: ask first when there are unsaved changes.
    if ctx.input(|i| i.viewport().close_requested()) && !app.close_allowed {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        cmds.push(Command::RequestClose);
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(root, |ui| {
            let r = layout::panels(ui.max_rect());
            panel(ui, r.media, |ui| media::media_panel(ui, app, &mut cmds));
            panel(ui, r.preview, |ui| preview_panel(ui, app, cache, &mut cmds));
            panel(ui, r.timeline, |ui| {
                timeline::timeline_panel(ui, app, &mut cmds)
            });
            panel(ui, r.effects, |ui| {
                effects::effects_panel(ui, app, cache, &mut cmds)
            });
            let width = r.timeline.width();
            panel(ui, r.command_bar, |ui| {
                command_bar(ui, app, width, &mut cmds)
            });
        });
    settings::settings_window(ctx, app, &mut cmds);
    settings::banner(ctx, app, &mut cmds);
    modals::modals(ctx, app, cache, &mut cmds);

    for c in cmds {
        if app.is_enabled(&c) {
            let _ = app.dispatch(c, now);
        }
    }
    if app.quit_requested {
        app.quit_requested = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let busy = app.busy();
    ctx.request_repaint_after(Duration::from_millis(if busy { 10 } else { 100 }));
}

fn update_texture(ctx: &Context, app: &AppState, cache: &mut UiCache) {
    let Some(f) = &app.frame else {
        cache.texture = None;
        return;
    };
    if cache.serial == app.frame_serial && cache.texture.is_some() {
        return;
    }
    let size = [f.width as usize, f.height as usize];
    if f.rgba.len() != size[0] * size[1] * 4 {
        return;
    }
    let img = ColorImage::from_rgba_unmultiplied(size, &f.rgba);
    match &mut cache.texture {
        Some(t) => t.set(img, TextureOptions::LINEAR),
        None => cache.texture = Some(ctx.load_texture("preview", img, TextureOptions::LINEAR)),
    }
    cache.serial = app.frame_serial;
}

fn panel(ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Ui)) {
    ui.scope_builder(UiBuilder::new().max_rect(rect.shrink(2.0)), |ui| {
        ui.set_clip_rect(rect);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            add(ui);
        });
    });
}

/// A button that is greyed out when the command cannot run.
pub fn button(ui: &mut Ui, app: &AppState, label: &str, cmd: Command, cmds: &mut Vec<Command>) {
    if ui
        .add_enabled(app.is_enabled(&cmd), Button::new(label))
        .clicked()
    {
        cmds.push(cmd);
    }
}

/// Largest rect with the aspect ratio of `size` that fits in `area`, centred.
pub fn fit_rect(area: Rect, size: Vec2) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return Rect::from_center_size(area.center(), Vec2::ZERO);
    }
    let s = (area.width() / size.x).min(area.height() / size.y);
    Rect::from_center_size(area.center(), size * s)
}

fn preview_panel(ui: &mut Ui, app: &AppState, cache: &UiCache, cmds: &mut Vec<Command>) {
    let controls_h = 2.0 * ui.spacing().interact_size.y + 8.0;
    let avail = ui.available_size();
    let (area, _) = ui.allocate_exact_size(
        vec2(avail.x, (avail.y - controls_h).max(10.0)),
        Sense::hover(),
    );
    ui.painter().rect_filled(area, 0.0, Color32::BLACK);
    if let Some(t) = &cache.texture {
        let r = fit_rect(area, t.size_vec2());
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(t.id(), r, uv, Color32::WHITE);
    }
    if let Some(e) = &app.preview.error {
        ui.colored_label(Color32::LIGHT_RED, e);
    }
    let dur = app.project.timeline.duration();
    ui.horizontal(|ui| {
        button(ui, app, "|<", Command::Seek(Micros::ZERO), cmds);
        button(ui, app, "<", Command::StepFrames(-1), cmds);
        let label = if app.preview.is_playing() {
            "Pause"
        } else {
            "Play"
        };
        button(ui, app, label, Command::PlayPause, cmds);
        button(ui, app, ">", Command::StepFrames(1), cmds);
        ui.monospace(format!("{} / {}", timecode(app.playhead), timecode(dur)));
    });
    let mut secs = app.playhead.as_secs_f64();
    let max = dur.as_secs_f64().max(0.001);
    ui.spacing_mut().slider_width = ui.available_width() - 20.0;
    let r = ui.add_enabled(
        dur > Micros::ZERO,
        egui::Slider::new(&mut secs, 0.0..=max).show_value(false),
    );
    if r.changed() {
        cmds.push(Command::Seek(Micros::from_secs_f64(secs)));
    }
}

fn command_bar(ui: &mut Ui, app: &AppState, timeline_width: f32, cmds: &mut Vec<Command>) {
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (label, cmd) in [
                ("New", Command::NewProject),
                ("Open…", Command::PickOpenProject),
                ("Save", Command::SaveProject),
                ("Save as…", Command::PickSaveProjectAs),
                ("Import…", Command::PickImport),
                ("Add", Command::Append(None)),
                ("Add music", Command::PlaceMusic(None)),
            ] {
                button(ui, app, label, cmd, cmds);
            }
            ui.separator();
            for (label, cmd) in [
                ("Split", Command::Split),
                ("Cut", Command::Cut),
                ("Copy", Command::Copy),
                ("Paste", Command::Paste),
                ("Delete", Command::Delete),
                ("Merge", Command::Merge),
                ("Fade", Command::ToggleFadeAtPlayhead),
                ("Move left", Command::MoveLeft),
                ("Move right", Command::MoveRight),
            ] {
                button(ui, app, label, cmd, cmds);
            }
            ui.separator();
            button(ui, app, "Undo", Command::Undo, cmds);
            button(ui, app, "Redo", Command::Redo, cmds);
            ui.separator();
            let play = if app.preview.is_playing() {
                "Pause"
            } else {
                "Play"
            };
            button(ui, app, play, Command::PlayPause, cmds);
            if app.is_exporting() {
                button(ui, app, "Cancel export", Command::CancelExport, cmds);
            } else {
                button(ui, app, "Export…", Command::PickExport, cmds);
            }
            button(
                ui,
                app,
                "Generate subtitles",
                Command::GenerateSubtitles,
                cmds,
            );
            button(ui, app, "Settings", Command::OpenSettings, cmds);
            button(ui, app, "About", Command::ShowAbout, cmds);
            ui.separator();
            ui.label("Zoom");
            let range = zoom::range(
                timeline::content_duration(app).as_secs_f64(),
                timeline_width - 30.0,
                app.fps().as_f64(),
            );
            let mut s = zoom::to_slider(app.zoom, range);
            if ui
                .add(egui::Slider::new(&mut s, 0.0..=1.0).show_value(false))
                .changed()
            {
                cmds.push(Command::SetZoom(zoom::from_slider(s, range)));
            }
            let status = if app.ffmpeg_checking {
                crate::app::CHECKING_FFMPEG
            } else {
                app.status.as_str()
            };
            if !status.is_empty() {
                ui.separator();
                ui.label(status);
            }
        });
    });
}

#[cfg(test)]
mod tests;
