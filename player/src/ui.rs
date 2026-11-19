// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The window of videolore player (egui). Panels never change the state directly: they
//! push commands, which run at the end of the frame. Every button and menu item has an
//! icon beside its text (REQ-142).

use crate::app::{Command, PlayerState, SKIP};
use crate::fullscreen::HINT_TEXT;
use crate::subs::Source;
use eframe::egui::{
    self, Align2, Button, Color32, ColorImage, Context, CursorIcon, Event, Id, Key, Modifiers,
    Order, Rect, Sense, TextureHandle, TextureOptions, Ui, pos2, vec2,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use videolore::time::{Micros, timecode};
use videolore::ui::icons;
use videolore::ui::placement;

/// The eframe application.
pub struct PlayerApp {
    pub state: PlayerState,
    pub cache: PlayerCache,
}

impl eframe::App for PlayerApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        frame_ui(ui, &mut self.state, &mut self.cache, Instant::now());
    }
}

/// UI-only state.
#[derive(Default)]
pub struct PlayerCache {
    texture: Option<TextureHandle>,
    serial: u64,
    title: String,
    /// Full screen as last sent to the window.
    fullscreen_sent: bool,
    /// A line is being dragged.
    line_drag: Option<placement::Drag>,
    /// The picture is being dragged to turn a 360 video.
    camera_drag: bool,
    /// Where the picture was drawn this frame (screen pixels).
    pub picture: Option<Rect>,
    /// The About text (built when the dialog opens).
    about: Option<String>,
}

/// The icon of an item of the player.
pub mod icon {
    use videolore::ui::icons as i;
    pub const OPEN: &str = i::FOLDER_OPEN;
    pub const QUIT: &str = i::SIGN_OUT;
    pub const FULLSCREEN: &str = i::CORNERS_OUT;
    pub const SUBTITLES: &str = i::SUBTITLES;
    pub const NO_SUBTITLES: &str = i::X;
    pub const OPEN_SRT: &str = i::FILE_VIDEO;
    pub const ABOUT: &str = i::INFO;
    pub const PLAY: &str = i::PLAY;
    pub const PAUSE: &str = i::PAUSE;
    pub const VOLUME: &str = i::SPEAKER_HIGH;
    pub const MUTED: &str = i::SPEAKER_SLASH;
    pub const SKIP_BACK: &str = i::SKIP_BACK;
    pub const OK: &str = i::CHECK;
}

fn label(icon: &str, text: &str) -> String {
    icons::label(icon, text)
}

/// Draws one frame and runs the commands it produced.
pub fn frame_ui(root: &mut Ui, app: &mut PlayerState, cache: &mut PlayerCache, now: Instant) {
    let ctx = root.ctx().clone();
    let ctx = &ctx;
    videolore::ui::register_subtitle_font(ctx);
    icons::install(ctx);
    app.tick(now);
    update_texture(ctx, app, cache);
    let title = app.title();
    if title != cache.title {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        cache.title = title;
    }
    if app.fullscreen.on != cache.fullscreen_sent {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.fullscreen.on));
        cache.fullscreen_sent = app.fullscreen.on;
    }
    let mut cmds = Vec::new();
    let moved = ctx.input(|i| {
        i.pointer.delta() != egui::Vec2::ZERO
            || i.events
                .iter()
                .any(|e| matches!(e, Event::PointerMoved(_) | Event::PointerButton { .. }))
    });
    if moved {
        app.fullscreen.pointer_moved(now);
    }
    let dropped: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .collect()
    });
    if let Some(p) = dropped.into_iter().next() {
        cmds.push(Command::Open(p));
    }
    let modal = app.notice.is_some() || app.show_about;
    cmds.extend(keys(ctx, app, modal));

    if !app.fullscreen.on {
        egui::Panel::top("player-menubar")
            .resizable(false)
            .show(root, |ui| menu_bar(ui, app, &mut cmds));
        egui::Panel::bottom("player-controls")
            .resizable(false)
            .show(root, |ui| {
                controls(ui, app, &mut cmds);
                if !app.status.is_empty() {
                    ui.weak(&app.status);
                }
            });
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(root, |ui| picture(ui, app, cache, &mut cmds, now));
    popups(ctx, app, cache, &mut cmds);

    for c in cmds {
        app.dispatch(c, now);
    }
    if app.quit {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    ctx.request_repaint_after(Duration::from_millis(if app.busy() { 10 } else { 100 }));
}

/// Keyboard shortcuts of this frame. Esc closes a dialog, then the full-screen popup,
/// then leaves full screen.
fn keys(ctx: &Context, _app: &PlayerState, modal: bool) -> Vec<Command> {
    let mut out = Vec::new();
    ctx.input_mut(|i| {
        let mut take = |m: Modifiers, k: Key| i.consume_key(m, k);
        if take(Modifiers::NONE, Key::Escape) {
            out.push(Command::Escape);
        }
        if modal {
            return;
        }
        if take(Modifiers::NONE, Key::Space) {
            out.push(Command::PlayPause);
        }
        if take(Modifiers::NONE, Key::F11) || take(Modifiers::NONE, Key::F) {
            out.push(Command::ToggleFullScreen);
        }
        if take(Modifiers::COMMAND, Key::O) {
            out.push(Command::PickOpen);
        }
        if take(Modifiers::COMMAND, Key::Q) {
            out.push(Command::Quit);
        }
        if take(Modifiers::NONE, Key::M) {
            out.push(Command::ToggleMute);
        }
        if take(Modifiers::NONE, Key::ArrowLeft) {
            out.push(Command::Skip(Micros(-SKIP.0)));
        }
        if take(Modifiers::NONE, Key::ArrowRight) {
            out.push(Command::Skip(SKIP));
        }
    });
    out
}

fn menu_bar(ui: &mut Ui, app: &PlayerState, cmds: &mut Vec<Command>) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui
                .add(
                    Button::new(label(icon::OPEN, "Open…")).shortcut_text(
                        ui.ctx().format_shortcut(&egui::KeyboardShortcut::new(
                            Modifiers::COMMAND,
                            Key::O,
                        )),
                    ),
                )
                .clicked()
            {
                cmds.push(Command::PickOpen);
                ui.close();
            }
            ui.separator();
            if ui.button(label(icon::QUIT, "Quit")).clicked() {
                cmds.push(Command::Quit);
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            let r = ui
                .add_enabled(
                    true,
                    Button::new(label(icon::FULLSCREEN, "Full screen")).shortcut_text("F11"),
                )
                .on_hover_text("Show only the picture and its controls");
            if r.clicked() {
                cmds.push(Command::ToggleFullScreen);
                ui.close();
            }
        });
        ui.menu_button("Subtitles", |ui| subtitles_menu(ui, app, cmds));
        ui.menu_button("Help", |ui| {
            if ui
                .button(label(icon::ABOUT, "About videolore player"))
                .clicked()
            {
                cmds.push(Command::ShowAbout);
                ui.close();
            }
        });
    });
}

fn subtitles_menu(ui: &mut Ui, app: &PlayerState, cmds: &mut Vec<Command>) {
    let video = app.has_video();
    let none = Button::selectable(
        app.subs.selected.is_none(),
        label(icon::NO_SUBTITLES, "None"),
    );
    if ui.add_enabled(video, none).clicked() {
        cmds.push(Command::ChooseSubtitles(None));
        ui.close();
    }
    for (i, s) in app.subs.sources.iter().enumerate() {
        let name = match s {
            Source::Embedded(_) => s.name(),
            Source::File(p) => format!("{} (file)", videolore::media::display_name(p)),
        };
        let b = Button::selectable(app.subs.selected == Some(i), label(icon::SUBTITLES, &name));
        if ui.add(b).clicked() {
            cmds.push(Command::ChooseSubtitles(Some(i)));
            ui.close();
        }
    }
    ui.separator();
    let open = ui
        .add_enabled(video, Button::new(label(icon::OPEN_SRT, "Open .srt…")))
        .on_hover_text("Choose a subtitle file for this video")
        .on_disabled_hover_text("Open a video first");
    if open.clicked() {
        cmds.push(Command::PickSrt);
        ui.close();
    }
}

/// The playback controls: play/pause, time, seek bar, mute and volume.
fn controls(ui: &mut Ui, app: &PlayerState, cmds: &mut Vec<Command>) {
    let has = app.has_video();
    ui.horizontal(|ui| {
        let (icon, text) = if app.is_playing() {
            (icon::PAUSE, "Pause")
        } else {
            (icon::PLAY, "Play")
        };
        if ui
            .add_enabled(has, Button::new(label(icon, text)))
            .on_hover_text("Play or pause (Space)")
            .clicked()
        {
            cmds.push(Command::PlayPause);
        }
        if ui
            .add_enabled(has, Button::new(label(icon::SKIP_BACK, "Start")))
            .on_hover_text("Back to the start")
            .clicked()
        {
            cmds.push(Command::Seek(Micros::ZERO));
        }
        ui.monospace(format!(
            "{} / {}",
            timecode(app.playhead),
            timecode(app.duration())
        ));
        let (vol_icon, vol_tip) = if app.muted() {
            (icon::MUTED, "Sound is off: click to turn it on (M)")
        } else {
            (icon::VOLUME, "Click to turn the sound off (M)")
        };
        // the right end: volume, then mute
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut v = f32::from(app.volume());
            if ui
                .add_enabled(
                    has,
                    egui::Slider::new(&mut v, 0.0..=100.0)
                        .show_value(false)
                        .step_by(1.0),
                )
                .on_hover_text("Volume")
                .changed()
            {
                cmds.push(Command::SetVolume(v.round() as u8));
            }
            if ui
                .add_enabled(has, Button::new(label(vol_icon, "Sound")))
                .on_hover_text(vol_tip)
                .clicked()
            {
                cmds.push(Command::ToggleMute);
            }
        });
    });
    let dur = app.duration().as_secs_f64().max(0.001);
    let mut secs = app.playhead.as_secs_f64();
    ui.spacing_mut().slider_width = (ui.available_width() - 20.0).max(40.0);
    let r = ui
        .add_enabled(
            has,
            egui::Slider::new(&mut secs, 0.0..=dur).show_value(false),
        )
        .on_hover_text("Seek: drag to move");
    if r.changed() {
        cmds.push(Command::Seek(Micros::from_secs_f64(secs)));
    }
}

fn update_texture(ctx: &Context, app: &PlayerState, cache: &mut PlayerCache) {
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
        None => cache.texture = Some(ctx.load_texture("player", img, TextureOptions::LINEAR)),
    }
    cache.serial = app.frame_serial;
}

/// The picture, its drags and (in full screen) the floating controls.
fn picture(
    ui: &mut Ui,
    app: &PlayerState,
    cache: &mut PlayerCache,
    cmds: &mut Vec<Command>,
    now: Instant,
) {
    let area = ui.max_rect();
    ui.painter().rect_filled(area, 0.0, Color32::BLACK);
    cache.picture = None;
    if let Some(t) = &cache.texture {
        let r = videolore::ui::fit_rect(area, t.size_vec2());
        ui.painter().image(
            t.id(),
            r,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        cache.picture = Some(r);
        interact(ui, app, cache, cmds, r);
    } else if !app.has_video() {
        let text = if app.opening() {
            &app.status
        } else {
            "Open a video with File > Open…, or drop a file here"
        };
        ui.painter().text(
            area.center(),
            Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(16.0),
            Color32::GRAY,
        );
    }
    if let Some(e) = &app.preview.error {
        ui.painter().text(
            area.left_top() + vec2(6.0, 6.0),
            Align2::LEFT_TOP,
            e,
            egui::FontId::proportional(13.0),
            Color32::LIGHT_RED,
        );
    }
    if app.fullscreen.on && app.fullscreen.controls_visible(now) {
        egui::Area::new(Id::unique("player-fs-controls"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -8.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width((area.width() - 80.0).clamp(300.0, 900.0));
                    controls(ui, app, cmds);
                });
            });
    }
}

/// Dragging on the picture: a subtitle line moves; on a 360 video anything else turns the
/// camera (the same gesture, limits and code as the editor's); the wheel zooms.
fn interact(
    ui: &mut Ui,
    app: &PlayerState,
    cache: &mut PlayerCache,
    cmds: &mut Vec<Command>,
    pic: Rect,
) {
    let Some(frame) = app.output_size() else {
        return;
    };
    let resp = ui.interact(pic, Id::unique("player-picture"), Sense::click_and_drag());
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();
    let boxes = line_boxes(&ctx, app);
    let to_frame = |p| placement::to_frame(p, pic, frame);
    if resp.drag_started() {
        let origin = ui
            .input(|i| i.pointer.press_origin())
            .or(resp.interact_pointer_pos());
        cache.line_drag = None;
        cache.camera_drag = false;
        if let Some(o) = origin {
            let f = to_frame(o);
            match placement::hit(&boxes, f) {
                Some((id, r)) => {
                    cache.line_drag = Some(placement::Drag {
                        line: id,
                        ids: vec![id],
                        grab: f - r.center(),
                        size: r.size(),
                        frame,
                        // a 360 view keeps the whole box on the picture
                        whole_box: app.is_360(),
                    });
                }
                None => cache.camera_drag = app.is_360(),
            }
        }
    }
    if let Some(d) = cache.line_drag.clone() {
        if let Some(p) = resp.interact_pointer_pos() {
            let pos = placement::drag_to(&d, to_frame(p));
            if resp.drag_stopped() {
                cmds.push(Command::DragLines(None));
                cmds.push(Command::PlaceLines {
                    ids: d.ids.clone(),
                    pos,
                });
            } else {
                cmds.push(Command::DragLines(Some((d.ids.clone(), pos))));
            }
        }
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        if resp.drag_stopped() {
            cache.line_drag = None;
        }
    } else if cache.camera_drag {
        let d = resp.drag_delta();
        if d != egui::Vec2::ZERO {
            cmds.push(Command::DragCamera {
                dx: d.x,
                dy: d.y,
                view_w: pic.width(),
            });
        }
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        if resp.drag_stopped() {
            cache.camera_drag = false;
        }
    } else if resp.hovered() {
        match resp
            .hover_pos()
            .map(|p| placement::hit(&boxes, to_frame(p)))
        {
            Some(Some(_)) => ctx.set_cursor_icon(CursorIcon::Grab),
            _ if app.is_360() => ctx.set_cursor_icon(CursorIcon::Grab),
            _ => {}
        }
    }
    if app.is_360() && resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            cmds.push(Command::ZoomCamera(scroll));
        }
    }
    if resp.clicked() {
        cmds.push(Command::PlayPause);
    }
    if resp.double_clicked() {
        cmds.push(Command::ToggleFullScreen);
    }
    // the lines that can be dragged show a thin outline while the pointer is over the picture
    if resp.hovered() || cache.line_drag.is_some() {
        for (_, r) in &boxes {
            painter.rect_stroke(
                placement::to_screen(*r, pic, frame),
                2.0,
                egui::Stroke::new(1.0, Color32::from_white_alpha(120)),
                egui::StrokeKind::Outside,
            );
        }
    }
}

/// The boxes of the lines on screen now (frame pixels), measured with the subtitle font.
pub fn line_boxes(ctx: &Context, app: &PlayerState) -> Vec<(videolore::subtitles::LineId, Rect)> {
    let painter = ctx.layer_painter(egui::LayerId::new(Order::Background, Id::NULL));
    let mut measure = |t: &str, size: f32, wrap: f32| {
        painter
            .layout(
                t.to_string(),
                videolore::ui::subtitle_font(ctx, size),
                Color32::WHITE,
                wrap,
            )
            .size()
    };
    app.line_boxes(&mut measure)
}

/// The full-screen hint, the notice and About.
fn popups(ctx: &Context, app: &PlayerState, cache: &mut PlayerCache, cmds: &mut Vec<Command>) {
    if app.fullscreen.hint_visible() {
        egui::Window::new("player-fs-hint")
            .title_bar(false)
            .order(Order::Foreground)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_TOP, vec2(0.0, 40.0))
            .show(ctx, |ui| {
                ui.label(label(icon::FULLSCREEN, HINT_TEXT));
                if ui.button(label(icon::OK, "OK")).clicked() {
                    cmds.push(Command::DismissFullScreenHint);
                }
            });
    }
    if let Some(n) = &app.notice {
        egui::Modal::new(Id::unique("player-notice")).show(ctx, |ui| {
            ui.heading(label(icons::INFO, "videolore player"));
            ui.label(n);
            if ui.button(label(icon::OK, "OK")).clicked() {
                cmds.push(Command::DismissNotice);
            }
        });
    }
    if app.show_about {
        let text = cache
            .about
            .get_or_insert_with(|| videolore::notices::about_text_player(app.ffmpeg.as_ref().ok()));
        let mut open = true;
        egui::Window::new(label(icon::ABOUT, "About videolore player"))
            .open(&mut open)
            .default_width(560.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(text.as_str()).monospace()).wrap(),
                        );
                    });
            });
        if !open {
            cmds.push(Command::CloseAbout);
        }
    } else {
        cache.about = None;
    }
}
