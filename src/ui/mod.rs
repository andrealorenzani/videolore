// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The egui user interface. Panels never mutate the state directly: they push
//! [`Command`]s, which are run by [`AppState::dispatch`] at the end of the frame.

pub mod keymap;
pub mod layout;
pub mod zoom;

use crate::app::{AppState, Command};
use crate::time::{Micros, timecode};
use eframe::egui::{
    self, Align2, Button, Color32, ColorImage, Context, FontId, ProgressBar, Rect, Sense, Stroke,
    StrokeKind, TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, pos2, vec2,
};
use std::time::{Duration, Instant};

/// UI-only state that is not part of the application state.
#[derive(Default)]
pub struct UiCache {
    texture: Option<TextureHandle>,
    serial: u64,
    title: String,
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
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        frame_ui(ctx, &mut self.state, &mut self.cache, Instant::now());
    }
}

/// Draws one frame and runs the commands it produced.
pub fn frame_ui(ctx: &Context, app: &mut AppState, cache: &mut UiCache, now: Instant) {
    app.tick(now);
    update_texture(ctx, app, cache);
    let title = app.title();
    if title != cache.title {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        cache.title = title;
    }

    let modal_open = app.ffmpeg.is_err() || app.notice.is_some();
    let mut cmds = if modal_open {
        Vec::new()
    } else {
        keymap::collect(ctx)
    };
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            let r = layout::panels(ui.max_rect());
            panel(ui, r.media, |ui| media_panel(ui, app, &mut cmds));
            panel(ui, r.preview, |ui| preview_panel(ui, app, cache, &mut cmds));
            panel(ui, r.timeline, |ui| timeline_panel(ui, app, &mut cmds));
            panel(ui, r.effects, |ui| effects_panel(ui, app, &mut cmds));
            let width = r.timeline.width();
            panel(ui, r.command_bar, |ui| {
                command_bar(ui, app, width, &mut cmds)
            });
        });
    modals(ctx, app, &mut cmds);

    for c in cmds {
        if app.is_enabled(&c) {
            let _ = app.dispatch(c, now);
        }
    }
    let busy = app.preview.is_playing() || app.is_exporting();
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

fn button(ui: &mut Ui, app: &AppState, label: &str, cmd: Command, cmds: &mut Vec<Command>) {
    if ui
        .add_enabled(app.is_enabled(&cmd), Button::new(label))
        .clicked()
    {
        cmds.push(cmd);
    }
}

fn media_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    ui.horizontal(|ui| {
        ui.strong("Media");
        button(ui, app, "Import…", Command::PickImport, cmds);
        button(ui, app, "Add to timeline", Command::Append(None), cmds);
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        for m in &app.project.media {
            let note = if m.missing {
                " (missing)"
            } else if !m.has_video() {
                " (audio only)"
            } else {
                ""
            };
            let label = format!(
                "{}{note}\n{}  {}",
                m.name,
                timecode(m.duration),
                m.format.describe()
            );
            let r = ui.selectable_label(app.selected_media == Some(m.id), label);
            if r.clicked() {
                cmds.push(Command::SelectMedia(Some(m.id)));
            }
            if r.double_clicked() {
                cmds.push(Command::Append(Some(m.id)));
            }
            r.context_menu(|ui| {
                let cmd = Command::Append(Some(m.id));
                if ui
                    .add_enabled(app.is_enabled(&cmd), Button::new("Add to timeline"))
                    .clicked()
                {
                    cmds.push(cmd);
                    ui.close();
                }
            });
        }
    });
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

const CLIP_COLORS: [Color32; 4] = [
    Color32::from_rgb(70, 110, 170),
    Color32::from_rgb(90, 140, 90),
    Color32::from_rgb(150, 100, 60),
    Color32::from_rgb(120, 80, 150),
];

fn timeline_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let fps = app.fps();
    let width = ui.available_width();
    let range = zoom::range(tl.duration().as_secs_f64(), width - 20.0, fps.as_f64());
    let z = zoom::clamp(app.zoom, range);
    if app.zoom != z {
        cmds.push(Command::SetZoom(z));
    }
    let px = |t: Micros| (t.as_secs_f64() * z as f64) as f32;
    let height = ui.available_height() - 16.0;
    let content_w = px(tl.duration()) + 40.0;
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
                Stroke::new(1.0, Color32::GRAY),
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
            cmds.push(Command::Seek(t_at(p.x)));
        }
        let clip_top = rect.min.y + ruler_h;
        let clip_h = (rect.height() - ruler_h - 4.0).max(10.0);
        let mut start = Micros::ZERO;
        for (i, c) in tl.clips.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x0 + px(start), clip_top), vec2(px(c.len()), clip_h));
            let resp = ui.interact(r, ui.id().with(("clip", c.id.0)), Sense::click_and_drag());
            let selected = app.selected_clip == Some(c.id);
            let color = CLIP_COLORS[(c.media.0 as usize) % CLIP_COLORS.len()];
            painter.rect_filled(r.shrink(1.0), 3.0, color);
            if selected {
                painter.rect_stroke(
                    r,
                    3.0,
                    Stroke::new(2.0, Color32::YELLOW),
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
        // boundary handles: click to toggle the fade
        let mut at = Micros::ZERO;
        for b in 0..tl.clips.len().saturating_sub(1) {
            at = at + tl.clips[b].len();
            let center = pos2(x0 + px(at), clip_top + clip_h - 8.0);
            let hr = Rect::from_center_size(center, vec2(12.0, 12.0));
            let resp = ui
                .interact(hr, ui.id().with(("fade", b)), Sense::click())
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
                Stroke::new(1.0, Color32::WHITE),
            );
            if resp.clicked() {
                cmds.push(Command::ToggleFade(b));
            }
        }
        let xp = x0 + px(app.playhead);
        painter.line_segment(
            [pos2(xp, rect.min.y), pos2(xp, rect.max.y)],
            Stroke::new(2.0, Color32::RED),
        );
    });
}

/// Index a dragged clip should move to when dropped at time `t`.
pub fn drop_index(tl: &crate::timeline::Timeline, t: Micros) -> usize {
    match tl.locate(t) {
        Some((i, start)) => {
            let half = Micros(tl.clips[i].len().0 / 2);
            if t - start < half { i } else { i + 1 }.min(tl.clips.len() - 1)
        }
        None => tl.clips.len().saturating_sub(1),
    }
}

fn effects_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.strong("Clip");
        let tl = &app.project.timeline;
        match app.selected_clip.and_then(|id| tl.index_of(id)) {
            None => {
                ui.label("Select a clip on the timeline.");
            }
            Some(i) => clip_settings(ui, app, i, cmds),
        }
        ui.separator();
        ui.strong("Export");
        match app.output_description() {
            Some(d) => ui.label(format!("Output: {d}")),
            None => ui.label("Add clips to see the output format."),
        };
        if app.is_exporting() {
            let s = &app.export_status;
            let mode = match s.stream_copy {
                Some(true) => "fast (no re-encoding)",
                Some(false) => "re-encoding",
                None => "preparing",
            };
            ui.label(format!("Exporting: {mode}"));
            ui.add(ProgressBar::new(s.progress).show_percentage());
            button(ui, app, "Cancel export", Command::CancelExport, cmds);
        } else {
            button(ui, app, "Export…", Command::PickExport, cmds);
        }
        ui.separator();
        ui.strong("ffmpeg");
        match &app.ffmpeg {
            Ok(i) => ui.label(format!("{} ({})", i.ffmpeg.display(), i.version)),
            Err(e) => ui.colored_label(Color32::LIGHT_RED, e),
        };
        button(ui, app, "Choose ffmpeg…", Command::PickFfmpeg, cmds);
        ui.separator();
        ui.strong("Shortcuts");
        for (k, v) in keymap::HELP {
            ui.label(format!("{k}: {v}"));
        }
    });
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
    ui.label(format!("Source: {name}"));
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
    ui.horizontal(|ui| {
        ui.label("Trim in:");
        button(ui, app, "-1f", trim(c.src_in - frame, c.src_out), cmds);
        button(ui, app, "+1f", trim(c.src_in + frame, c.src_out), cmds);
        if ui.add_enabled(inside, Button::new("at playhead")).clicked() {
            cmds.push(trim(c.src_in + offset, c.src_out));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Trim out:");
        button(ui, app, "-1f", trim(c.src_in, c.src_out - frame), cmds);
        button(ui, app, "+1f", trim(c.src_in, c.src_out + frame), cmds);
        if ui.add_enabled(inside, Button::new("at playhead")).clicked() {
            cmds.push(trim(c.src_in, c.src_in + offset));
        }
    });
    ui.horizontal(|ui| {
        if i > 0 {
            let mut on = tl.has_fade(i - 1);
            if ui.checkbox(&mut on, "Fade in").changed() {
                cmds.push(Command::ToggleFade(i - 1));
            }
        }
        if i + 1 < tl.clips.len() {
            let mut on = tl.has_fade(i);
            if ui.checkbox(&mut on, "Fade out").changed() {
                cmds.push(Command::ToggleFade(i));
            }
        }
    });
    button(ui, app, "Merge with next", Command::Merge, cmds);
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
            ui.separator();
            ui.label("Zoom");
            let range = zoom::range(
                app.project.timeline.duration().as_secs_f64(),
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
            if !app.status.is_empty() {
                ui.separator();
                ui.label(&app.status);
            }
        });
    });
}

fn modals(ctx: &Context, app: &AppState, cmds: &mut Vec<Command>) {
    if let Err(e) = &app.ffmpeg {
        egui::Modal::new(egui::Id::new("ffmpeg-missing")).show(ctx, |ui| {
            ui.heading("ffmpeg is required");
            ui.label(e);
            ui.label(crate::ffmpeg::install_hint());
            ui.label("You can also set VIDEOLORE_FFMPEG / VIDEOLORE_FFPROBE.");
            ui.horizontal(|ui| {
                if ui.button("Choose ffmpeg…").clicked() {
                    cmds.push(Command::PickFfmpeg);
                }
                if ui.button("Retry").clicked() {
                    cmds.push(Command::RetryFfmpeg);
                }
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    } else if let Some(n) = &app.notice {
        egui::Modal::new(egui::Id::new("notice")).show(ctx, |ui| {
            ui.label(n);
            if ui.button("OK").clicked() {
                cmds.push(Command::DismissNotice);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{FakeDialogs, files, state};
    use crate::preview::Frame;
    use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput};

    fn input(w: f32, h: f32, events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))),
            events,
            ..Default::default()
        }
    }

    fn run(ctx: &Context, app: &mut AppState, cache: &mut UiCache, w: f32, h: f32, ev: Vec<Event>) {
        let now = Instant::now();
        let _ = ctx.run(input(w, h, ev), |ctx| frame_ui(ctx, app, cache, now));
    }

    fn loaded() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let mut app = state(FakeDialogs::default());
        let now = Instant::now();
        app.dispatch(Command::Import(f), now).unwrap();
        let a = app.project.media[0].id;
        let b = app.project.media[1].id;
        app.dispatch(Command::Append(Some(a)), now).unwrap();
        app.dispatch(Command::Append(Some(b)), now).unwrap();
        app.dispatch(Command::ToggleFade(0), now).unwrap();
        (dir, app)
    }

    fn key(k: Key, m: Modifiers) -> Event {
        Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: m,
        }
    }

    // covers: REQ-024
    #[test]
    fn renders_all_panels_at_any_size() {
        let (_d, mut app) = loaded();
        let ctx = Context::default();
        let mut cache = UiCache::default();
        app.selected_clip = Some(app.project.timeline.clips[1].id);
        app.selected_media = Some(app.project.media[0].id);
        app.project.media[1].missing = true;
        app.frame = Some(Frame {
            pts: Micros::ZERO,
            width: 4,
            height: 2,
            rgba: vec![255; 32],
        });
        app.frame_serial = 1;
        app.status = "hello".into();
        for (w, h) in [(1280.0, 800.0), (800.0, 500.0), (1920.0, 1080.0)] {
            run(&ctx, &mut app, &mut cache, w, h, vec![]);
        }
        assert!(cache.has_texture());
        app.frame_serial = 2;
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        app.frame.as_mut().unwrap().rgba.clear(); // malformed frame is ignored
        app.frame_serial = 3;
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        app.frame = None;
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        assert!(!cache.has_texture());
        // modals
        app.notice = Some("note".into());
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        app.notice = None;
        app.ffmpeg = Err("ffmpeg was not found".into());
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![key(Key::Space, Modifiers::NONE)],
        );
        assert!(!app.preview.is_playing());
    }

    // covers: REQ-025
    #[test]
    fn shortcuts_dispatch_commands() {
        let (_d, mut app) = loaded();
        let ctx = Context::default();
        let mut cache = UiCache::default();
        app.dispatch(Command::Seek(Micros(2_000_000)), Instant::now())
            .unwrap();
        let n = app.project.timeline.clips.len();
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![key(Key::B, Modifiers::COMMAND)],
        );
        assert_eq!(app.project.timeline.clips.len(), n + 1);
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![key(Key::Z, Modifiers::COMMAND)],
        );
        assert_eq!(app.project.timeline.clips.len(), n);
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![key(Key::Y, Modifiers::COMMAND)],
        );
        assert_eq!(app.project.timeline.clips.len(), n + 1);
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![Event::Copy]);
        assert!(app.clipboard.is_some());
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![Event::Paste(String::new())],
        );
        assert_eq!(app.project.timeline.clips.len(), n + 2);
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![Event::Cut]);
        assert_eq!(app.project.timeline.clips.len(), n + 1);
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![key(Key::Space, Modifiers::NONE)],
        );
        assert!(app.preview.is_playing());
        assert!(app.title().starts_with('*'));
        // the fake decoder produces no frames: playback ends on the next frame
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.preview.is_playing() && Instant::now() < deadline {
            run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        }
        assert!(!app.preview.is_playing());
        // disabled commands are ignored
        app.history.clear();
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![key(Key::Z, Modifiers::COMMAND)],
        );
        assert!(app.status.is_empty() || !app.status.contains("undo"));
    }

    // covers: REQ-012
    #[test]
    fn zoom_starts_at_fit_and_follows_the_slider() {
        let (_d, mut app) = loaded();
        let ctx = Context::default();
        let mut cache = UiCache::default();
        assert_eq!(app.zoom, 0.0);
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        let fit = app.zoom;
        assert!(fit > 0.0);
        // fit: the whole 10 s timeline fits in the panel width
        assert!(fit * 10.0 <= 1280.0 * layout::TIMELINE_WIDTH);
        let frame_level = zoom::PX_PER_FRAME_MAX * app.fps().as_f64() as f32;
        app.dispatch(Command::SetZoom(1e6), Instant::now()).unwrap();
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        assert!((app.zoom - frame_level).abs() < 1e-3);
    }

    #[test]
    fn clicking_the_timeline_selects_and_seeks() {
        let (_d, mut app) = loaded();
        let ctx = Context::default();
        let mut cache = UiCache::default();
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        let r = layout::panels(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0)));
        let p = pos2(r.timeline.min.x + 60.0, r.timeline.center().y);
        let press = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![Event::PointerMoved(p), press(true)],
        );
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![press(false)],
        );
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
        assert_eq!(app.selected_clip, Some(app.project.timeline.clips[0].id));
        assert!(app.playhead > Micros::ZERO);
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![Event::PointerGone],
        );
    }

    #[test]
    fn helpers() {
        let (_d, app) = loaded();
        let tl = &app.project.timeline;
        assert_eq!(drop_index(tl, Micros(1_000_000)), 0);
        assert_eq!(drop_index(tl, Micros(4_000_000)), 1);
        assert_eq!(drop_index(tl, Micros(9_000_000)), 1);
        assert_eq!(drop_index(tl, Micros(99_000_000)), 1);
        let r = fit_rect(
            Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0)),
            vec2(16.0, 9.0),
        );
        assert!((r.width() / r.height() - 16.0 / 9.0).abs() < 1e-3);
        assert_eq!(r.height(), 100.0);
        let z = fit_rect(Rect::from_min_size(Pos2::ZERO, vec2(2.0, 2.0)), Vec2::ZERO);
        assert_eq!(z.area(), 0.0);
    }
}
