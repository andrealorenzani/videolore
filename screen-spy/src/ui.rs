// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The window (REQ-342): Preview, Screen, Output and one button. Panels never change the
//! state directly: they push commands, which run at the end of the frame.

use crate::app::{Command, Phase, RecorderState};
use crate::plan::{AudioFormat, VideoFormat};
use crate::screens;
use eframe::egui::{
    self, Button, Color32, ColorImage, Context, Id, Key, Modifiers, RichText, TextureHandle,
    TextureOptions, Ui, pos2, vec2,
};
use std::time::{Duration, Instant};
use videolore::ui::icons;

/// The window's size at the first start, and the smallest.
pub const DEFAULT_SIZE: [f32; 2] = [800.0, 680.0];
pub const MIN_SIZE: [f32; 2] = [640.0, 560.0];
/// The note under the sound formats.
pub const WAV_NOTE: &str = "WAV sound is saved in an MKV file";

/// The eframe application.
pub struct RecorderApp {
    pub state: RecorderState,
    pub cache: RecorderCache,
}

impl eframe::App for RecorderApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        frame_ui(ui, &mut self.state, &mut self.cache, Instant::now());
    }
}

/// UI-only state.
#[derive(Default)]
pub struct RecorderCache {
    texture: Option<TextureHandle>,
    serial: u64,
    /// The About text (built when the dialog opens).
    about: Option<String>,
}

fn label(icon: &str, text: &str) -> String {
    icons::label(icon, text)
}

/// "01:05" or "1:00:05".
pub fn clock(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Draws one frame and runs the commands it produced.
pub fn frame_ui(root: &mut Ui, app: &mut RecorderState, cache: &mut RecorderCache, now: Instant) {
    let ctx = root.ctx().clone();
    let ctx = &ctx;
    icons::install(ctx);
    app.tick(now);
    update_texture(ctx, app, cache);
    let mut cmds = Vec::new();
    // the window's close control asks first while a recording runs (REQ-351)
    if ctx.input(|i| i.viewport().close_requested()) && !app.quit {
        if app.recording() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        cmds.push(Command::Close);
    }
    let modal = app.message.is_some() || app.replace.is_some() || app.ask_close;
    if !modal && ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Q)) {
        cmds.push(Command::Close);
    }

    egui::Panel::top("spy-menubar")
        .resizable(false)
        .show(root, |ui| menu_bar(ui, &mut cmds));
    egui::Panel::bottom("spy-controls")
        .resizable(false)
        .show(root, |ui| controls(ui, app, &mut cmds, now));
    egui::CentralPanel::default().show(root, |ui| preview(ui, app, cache));
    popups(ctx, app, cache, &mut cmds);

    for c in cmds {
        app.dispatch(c, now);
    }
    if app.quit {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let wait = match app.phase() {
        Phase::Idle => 500,
        _ => 100,
    };
    ctx.request_repaint_after(Duration::from_millis(wait));
}

fn menu_bar(ui: &mut Ui, cmds: &mut Vec<Command>) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui
                .button(label(icons::INFO, "About videolore screen spy"))
                .clicked()
            {
                cmds.push(Command::ShowAbout);
                ui.close();
            }
            ui.separator();
            if ui.button(label(icons::SIGN_OUT, "Quit")).clicked() {
                cmds.push(Command::Close);
                ui.close();
            }
        });
    });
}

fn update_texture(ctx: &Context, app: &RecorderState, cache: &mut RecorderCache) {
    let Some(f) = &app.frame else {
        cache.texture = None;
        return;
    };
    if cache.serial == app.frame_serial && cache.texture.is_some() {
        return;
    }
    let size = [f.width as usize, f.height as usize];
    if f.rgb.len() != size[0] * size[1] * 3 {
        return;
    }
    let img = ColorImage::from_rgb(size, &f.rgb);
    match &mut cache.texture {
        Some(t) => t.set(img, TextureOptions::LINEAR),
        None => cache.texture = Some(ctx.load_texture("preview", img, TextureOptions::LINEAR)),
    }
    cache.serial = app.frame_serial;
}

fn preview(ui: &mut Ui, app: &RecorderState, cache: &RecorderCache) {
    ui.strong("Preview");
    let area = ui.available_rect_before_wrap();
    ui.painter().rect_filled(area, 4.0, Color32::BLACK);
    if let Some(t) = &cache.texture {
        let r = videolore::ui::fit_rect(area, t.size_vec2());
        ui.painter().image(
            t.id(),
            r,
            egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    let text = app
        .preview_error
        .clone()
        .unwrap_or_else(|| match app.phase() {
            Phase::Idle => "No preview".to_string(),
            _ => "Waiting for the picture…".to_string(),
        });
    ui.put(
        area.shrink(12.0),
        egui::Label::new(RichText::new(text).color(Color32::GRAY)).wrap(),
    );
}

/// The Screen and Output parts and the button.
fn controls(ui: &mut Ui, app: &RecorderState, cmds: &mut Vec<Command>, now: Instant) {
    let free = !app.locked();
    ui.add_space(4.0);
    if let Some(why) = &app.platform.blocked {
        ui.colored_label(ui.visuals().warn_fg_color, label(icons::WARNING, why));
        ui.add_space(4.0);
    }
    ui.add_enabled_ui(free, |ui| {
        ui.strong("Screen");
        let current = app
            .chosen_index()
            .map(|i| screens::label(i, &app.screens[i]))
            .unwrap_or_else(|| "No screen".to_string());
        egui::ComboBox::from_id_salt("spy-screen")
            .selected_text(current)
            .width(260.0)
            .show_ui(ui, |ui| {
                for (i, s) in app.screens.iter().enumerate() {
                    let chosen = app.chosen == Some(s.id);
                    if ui.selectable_label(chosen, screens::label(i, s)).clicked() {
                        cmds.push(Command::SelectScreen(s.id));
                    }
                }
            });
        ui.add_space(6.0);
        ui.strong("Output");
        egui::Grid::new("spy-output")
            .num_columns(2)
            .spacing(vec2(8.0, 6.0))
            .show(ui, |ui| output(ui, app, cmds));
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let icon = match app.phase() {
            Phase::Idle | Phase::Previewing => icons::PLAY,
            Phase::Starting => icons::X,
            Phase::Recording | Phase::Stopping => icons::STOP,
        };
        let button = Button::new(RichText::new(label(icon, app.button_label())).size(18.0))
            .min_size(vec2(200.0, 34.0));
        let enabled = app.phase() != Phase::Stopping && !app.cancelling();
        if ui.add_enabled(enabled, button).clicked() {
            cmds.push(Command::ToggleRecording);
        }
        if app.phase() == Phase::Starting {
            ui.spinner();
        }
        if let Some(d) = app.recording_for(now) {
            ui.colored_label(Color32::LIGHT_RED, format!("Recording {}", clock(d)));
        }
    });
    if !app.status.is_empty() {
        ui.add(egui::Label::new(RichText::new(&app.status).weak()).truncate());
    }
    ui.add_space(4.0);
}

fn output(ui: &mut Ui, app: &RecorderState, cmds: &mut Vec<Command>) {
    let title = ui.label("File name");
    ui.horizontal(|ui| {
        let mut name = app.name.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(360.0))
            .labelled_by(title.id)
            .changed()
        {
            cmds.push(Command::SetName(name));
        }
        if ui
            .button(label(icons::FOLDER_OPEN, "Choose…"))
            .on_hover_text("Choose the folder and the name of the file")
            .clicked()
        {
            cmds.push(Command::PickFile);
        }
    });
    ui.end_row();
    ui.label("Folder");
    ui.add(egui::Label::new(app.folder.display().to_string()).truncate());
    ui.end_row();
    ui.label("Video format");
    egui::ComboBox::from_id_salt("spy-video")
        .selected_text(app.video.label())
        .width(260.0)
        .show_ui(ui, |ui| {
            for f in VideoFormat::ALL {
                if ui.selectable_label(app.video == f, f.label()).clicked() {
                    cmds.push(Command::SetVideo(f));
                }
            }
        });
    ui.end_row();
    ui.label("Audio format");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("spy-audio")
            .selected_text(app.audio.label())
            .width(90.0)
            .show_ui(ui, |ui| {
                for f in AudioFormat::ALL {
                    if ui.selectable_label(app.audio == f, f.label()).clicked() {
                        cmds.push(Command::SetAudio(f));
                    }
                }
            });
        ui.weak(WAV_NOTE);
    });
    ui.end_row();
}

/// A question or message in the middle of the window, with its buttons.
fn ask(
    ctx: &Context,
    id: &str,
    title: &str,
    text: &str,
    buttons: [Option<(&str, &str, Command)>; 2],
    cmds: &mut Vec<Command>,
) {
    egui::Modal::new(Id::unique(id)).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.heading(title);
        ui.label(text);
        ui.horizontal(|ui| {
            for (icon, text, command) in buttons.into_iter().flatten() {
                if ui.button(label(icon, text)).clicked() {
                    cmds.push(command);
                }
            }
        });
    });
}

fn popups(ctx: &Context, app: &RecorderState, cache: &mut RecorderCache, cmds: &mut Vec<Command>) {
    if let Some(path) = &app.replace {
        ask(
            ctx,
            "spy-replace",
            "Replace the file?",
            &format!("{} already exists.", path.display()),
            [
                Some((icons::CHECK, "Replace", Command::Replace)),
                Some((icons::X, "Cancel", Command::CancelReplace)),
            ],
            cmds,
        );
    }
    if app.ask_close {
        ask(
            ctx,
            "spy-close",
            "Stop the recording and close?",
            "The file is finished before the program closes.",
            [
                Some((icons::STOP, "Stop and close", Command::StopAndClose)),
                Some((icons::X, "Cancel", Command::CancelClose)),
            ],
            cmds,
        );
    }
    if let Some(m) = &app.message {
        ask(
            ctx,
            "spy-message",
            "videolore screen spy",
            m,
            [Some((icons::CHECK, "OK", Command::DismissMessage)), None],
            cmds,
        );
    }
    if !app.show_about {
        cache.about = None;
        return;
    }
    let text = cache.about.get_or_insert_with(crate::about::text);
    let mut open = true;
    egui::Window::new(label(icons::INFO, "About videolore screen spy"))
        .open(&mut open)
        .default_width(560.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(text.as_str()).monospace()).wrap());
                });
        });
    if !open {
        cmds.push(Command::CloseAbout);
    }
}
