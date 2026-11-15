// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The egui user interface. Panels never mutate the state directly: they push
//! [`Command`]s, which are run by [`AppState::dispatch`] at the end of the frame.
//!
//! The window, top to bottom: menu bar ([`menu`]), toolbar ([`toolbar`]), the content area
//! with the four resizable sections ([`layout`]) and the status bar ([`status`]).
//! Floating windows (Settings, effect popups, clip details, shortcuts) are in [`popups`].

pub mod actions;
pub mod effects;
pub mod keymap;
pub mod layout;
pub mod markers;
pub mod media;
pub mod menu;
pub mod modals;
pub mod placement;
pub mod popups;
pub mod ruler;
pub mod settings;
pub mod status;
pub mod timeline;
pub mod toolbar;
pub mod zoom;

use crate::app::{AppState, Command};
use crate::music::Mix;
use crate::subtitles::LineId;
use crate::time::{Micros, timecode};
use eframe::egui::{
    self, Button, Color32, ColorImage, Context, CursorIcon, Id, Pos2, Rect, ScrollArea, Sense,
    Stroke, TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, pos2, vec2,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// A scroll area drawn this frame: its visible part and the whole content (for the
/// "everything reachable" checks).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Container {
    pub viewport: Rect,
    pub content: Rect,
}

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
    /// Section sizes while a border is dragged (saved once, on release).
    pub layout_drag: Option<layout::Fractions>,
    /// Section sizes when the drag started.
    layout_drag_start: Option<layout::Fractions>,
    /// Where the sections were drawn this frame.
    pub sections: Option<layout::SectionRects>,
    /// Scroll areas drawn this frame.
    pub containers: Vec<Container>,
    /// Timeline geometry of this frame.
    pub timeline: Option<timeline::Geometry>,
    /// Where the chooser of overlapping markers opens.
    pub chooser_pos: Option<Pos2>,
    /// The marker a right-click menu is about.
    pub context_marker: Option<markers::Marker>,
    /// The timeline time a right-click menu is about.
    pub context_time: Option<Micros>,
    /// The track picked in the subtitle source dialog.
    pub source_choice: usize,
    /// A clip's volume while its slider is dragged (one undo step on release).
    pub clip_volume_drag: Option<(crate::timeline::ClipId, u8, bool)>,
    /// The subtitle being dragged in the preview: its lines, the pointer offset from the
    /// box centre, the box size (frame pixels).
    pub subtitle_drag: Option<placement::Drag>,
    /// Where a Shift+drag on the timeline started (time) and whether it adds.
    pub range_drag: Option<(Micros, bool)>,
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
    register_subtitle_font(ctx);
    app.tick(now);
    // Ctrl/Cmd +, - and 0 zoom the timeline, not the whole interface
    if ctx.options(|o| o.zoom_with_keyboard) {
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
    }
    update_texture(ctx, app, cache);
    let title = app.title();
    if title != cache.title {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        cache.title = title;
    }
    cache.containers.clear();

    let mut cmds = Vec::new();
    let dropped: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .collect()
    });
    if !dropped.is_empty() {
        cmds.push(Command::DropFiles(dropped));
    }
    if !modal_open(app) {
        cmds.extend(keymap::collect(ctx, app));
    }
    // The window's close button: ask first when there are unsaved changes.
    if ctx.input(|i| i.viewport().close_requested()) && !app.close_allowed {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        cmds.push(Command::RequestClose);
    }
    egui::Panel::top("videolore-menubar")
        .resizable(false)
        .show(root, |ui| menu::menu_bar(ui, app, cache, &mut cmds));
    egui::Panel::top("videolore-toolbar")
        .resizable(false)
        .show(root, |ui| toolbar::toolbar(ui, app, &mut cmds));
    egui::Panel::bottom("videolore-statusbar")
        .resizable(false)
        .show(root, |ui| status::status_bar(ui, app, &mut cmds));
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(root, |ui| content(ui, app, cache, &mut cmds));
    popups::windows(ctx, app, cache, &mut cmds);
    settings::banner(ctx, app, &mut cmds);
    modals::modals(ctx, app, cache, &mut cmds);
    drop_hint(ctx);

    for c in cmds {
        if app.is_enabled(&c) {
            let _ = app.dispatch(c, now);
        }
    }
    if app.quit_requested {
        app.quit_requested = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let busy = app.busy() || cache.layout_drag.is_some();
    ctx.request_repaint_after(Duration::from_millis(if busy { 10 } else { 100 }));
}

/// "Drop to import" over the window while files are dragged over it.
fn drop_hint(ctx: &Context) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::unique("videolore-drop"),
    ));
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(160));
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        "Drop to import (a project is opened)",
        egui::FontId::proportional(22.0),
        Color32::WHITE,
    );
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

/// The four sections and the borders between them.
fn content(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let area = ui.max_rect();
    let f = cache
        .layout_drag
        .unwrap_or(app.settings.layout.unwrap_or_default());
    let r = layout::sections(area, f, layout::MINS);
    cache.sections = Some(r);
    let zoom_range = timeline::zoom_range(app, r.timeline.width());
    section(
        ui,
        cache,
        cmds,
        r.media,
        "Media",
        true,
        |_, _| {},
        |ui, _, cmds| media::media_panel(ui, app, cmds),
    );
    section(
        ui,
        cache,
        cmds,
        r.preview,
        "Preview",
        false,
        |_, _| {},
        |ui, cache, cmds| preview_panel(ui, app, cache, cmds),
    );
    section(
        ui,
        cache,
        cmds,
        r.timeline,
        "Timeline",
        false,
        |ui, cmds| timeline::header(ui, app, zoom_range, cmds),
        |ui, cache, cmds| timeline::timeline_panel(ui, app, cache, zoom_range, cmds),
    );
    section(
        ui,
        cache,
        cmds,
        r.effects,
        "Effects",
        true,
        |_, _| {},
        |ui, _, cmds| effects::effects_panel(ui, app, cmds),
    );
    for (s, rect) in r.splitters {
        splitter(ui, cache, cmds, area, f, s, rect);
    }
}

/// A draggable border: resize cursor, live update while dragging, one save on release.
fn splitter(
    ui: &mut Ui,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    area: Rect,
    f: layout::Fractions,
    s: layout::Splitter,
    rect: Rect,
) {
    let resp = ui
        .interact(rect, Id::unique(("videolore-splitter", s)), Sense::drag())
        .on_hover_text(s.tooltip());
    let active = resp.hovered() || resp.dragged();
    if active {
        ui.ctx().set_cursor_icon(if s.is_vertical() {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        });
    }
    let color = if active {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke.color
    };
    let line = if s.is_vertical() {
        [rect.center_top(), rect.center_bottom()]
    } else {
        [rect.left_center(), rect.right_center()]
    };
    ui.painter()
        .line_segment(line, Stroke::new(if active { 2.0 } else { 1.0 }, color));
    // measured from where the drag started (per-frame deltas can include the move that
    // came with the press)
    if resp.drag_started() {
        cache.layout_drag_start = Some(f);
    }
    if resp.dragged()
        && let (Some(start), Some(d)) = (cache.layout_drag_start, resp.total_drag_delta())
    {
        let delta = if s.is_vertical() { d.x } else { d.y };
        cache.layout_drag = Some(layout::drag(start, s, delta, area, layout::MINS));
    }
    if resp.drag_stopped() {
        cache.layout_drag_start = None;
        if let Some(f) = cache.layout_drag.take() {
            cmds.push(Command::SetLayout(f));
        }
    }
}

/// One section: a framed title header (with optional extra controls) and its body, in a
/// scroll area when `scroll` (the timeline and preview handle it themselves).
#[allow(clippy::too_many_arguments)]
fn section(
    ui: &mut Ui,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    rect: Rect,
    title: &str,
    scroll: bool,
    header: impl FnOnce(&mut Ui, &mut Vec<Command>),
    body: impl FnOnce(&mut Ui, &mut UiCache, &mut Vec<Command>),
) {
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).id_salt(("section", title)),
        |ui| {
            ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            egui::Frame::group(ui.style())
                .inner_margin(4.0)
                .outer_margin(0.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.horizontal(|ui| {
                        ui.strong(title);
                        header(ui, cmds);
                    });
                    ui.separator();
                    if scroll {
                        let area = ScrollArea::both()
                            .id_salt(("section-scroll", title))
                            .auto_shrink(false);
                        scroll_body(ui, cache, area, |ui, cache| body(ui, cache, cmds));
                    } else {
                        body(ui, cache, cmds);
                    }
                });
        },
    );
}

/// Shows `area` and records it in [`UiCache::containers`].
pub fn scroll_body<R>(
    ui: &mut Ui,
    cache: &mut UiCache,
    area: ScrollArea,
    add: impl FnOnce(&mut Ui, &mut UiCache) -> R,
) -> R {
    let out = area.show(ui, |ui| add(ui, cache));
    record(
        cache,
        out.inner_rect,
        out.state.clamped_offset(),
        out.content_size,
    );
    out.inner
}

/// Records a scroll area drawn this frame.
pub fn record(cache: &mut UiCache, viewport: Rect, offset: Vec2, content_size: Vec2) {
    cache.containers.push(Container {
        viewport,
        content: Rect::from_min_size(viewport.min - offset, content_size),
    });
}

/// A button that is greyed out (with the reason in its tooltip) when the command cannot
/// run.
pub fn button(ui: &mut Ui, app: &AppState, label: &str, cmd: Command, cmds: &mut Vec<Command>) {
    let reason = app.disabled_reason(&cmd);
    let mut r = ui.add_enabled(reason.is_none(), Button::new(label));
    if let Some(why) = reason {
        r = r.on_disabled_hover_text(why);
    }
    if r.clicked() {
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

/// Height of the preview's two rows of controls.
fn controls_height(ui: &Ui) -> f32 {
    2.0 * ui.spacing().interact_size.y + 3.0 * ui.spacing().item_spacing.y
}

/// The preview: picture, play controls and seek bar. The controls always get their room
/// first; the picture shrinks (to nothing when the section is very small).
fn preview_panel(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let controls_h = controls_height(ui);
    let picture_h = (ui.available_height() - controls_h).max(0.0);
    let area = ScrollArea::vertical()
        .id_salt("preview-scroll")
        .auto_shrink(false);
    scroll_body(ui, cache, area, |ui, cache| {
        let w = ui.available_width();
        let (area, _) = ui.allocate_exact_size(vec2(w, picture_h), Sense::hover());
        ui.painter().rect_filled(area, 0.0, Color32::BLACK);
        if let Some(t) = &cache.texture {
            let r = fit_rect(area, t.size_vec2());
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            ui.painter().image(t.id(), r, uv, Color32::WHITE);
            subtitle_drag(ui, app, cache, cmds, r);
        }
        if let Some(e) = &app.preview.error {
            ui.painter().text(
                area.left_top() + vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                e,
                egui::FontId::proportional(12.0),
                Color32::LIGHT_RED,
            );
        }
        let dur = app.project.timeline.duration();
        ui.horizontal(|ui| {
            use actions::ActionId;
            for (label, id) in [
                ("|<", ActionId::GoStart),
                ("<", ActionId::PrevFrame),
                (
                    if app.preview.is_playing() {
                        "Pause"
                    } else {
                        "Play"
                    },
                    ActionId::PlayPause,
                ),
                (">", ActionId::NextFrame),
                (">|", ActionId::GoEnd),
            ] {
                let tip = action_tooltip(ui.ctx(), app, id);
                let cmd = id.command(app);
                let reason = app.disabled_reason(&cmd);
                let r = ui
                    .add_enabled(reason.is_none(), Button::new(label))
                    .on_hover_text(tip)
                    .on_disabled_hover_text(reason.unwrap_or_default());
                if r.clicked() {
                    cmds.push(cmd);
                }
            }
            ui.monospace(format!("{} / {}", timecode(app.playhead), timecode(dur)));
        });
        let mut secs = app.playhead.as_secs_f64();
        let max = dur.as_secs_f64().max(0.001);
        ui.spacing_mut().slider_width = (ui.available_width() - 20.0).max(40.0);
        let r = ui
            .add_enabled(
                dur > Micros::ZERO,
                egui::Slider::new(&mut secs, 0.0..=max).show_value(false),
            )
            .on_hover_text("Seek: drag to move the playhead");
        if r.changed() {
            cmds.push(Command::Seek(Micros::from_secs_f64(secs)));
        }
    });
}

/// The font family the subtitles are measured with (the bundled subtitle font).
pub const SUBTITLE_FAMILY: &str = "videolore-subtitle";

/// Makes the bundled subtitle font available for measuring (once per context).
pub fn register_subtitle_font(ctx: &Context) {
    ctx.add_font(egui::epaint::text::FontInsert::new(
        SUBTITLE_FAMILY,
        egui::FontData::from_static(crate::ass::FONT),
        vec![egui::epaint::text::InsertFontFamily {
            family: egui::FontFamily::Name(SUBTITLE_FAMILY.into()),
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));
}

/// The subtitle font at `size`, or the normal one while it is not loaded yet.
fn subtitle_font(ctx: &Context, size: f32) -> egui::FontId {
    let family = egui::FontFamily::Name(SUBTITLE_FAMILY.into());
    let loaded = ctx.fonts(|f| f.definitions().families.contains_key(&family));
    egui::FontId::new(
        size.max(1.0),
        if loaded {
            family
        } else {
            egui::FontFamily::Proportional
        },
    )
}

/// Dragging a subtitle on the preview picture moves its line (or the selected lines,
/// when it is one of them): an outline follows the pointer at once, the picture is
/// rendered again, and one undo step is recorded on release.
fn subtitle_drag(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    cmds: &mut Vec<Command>,
    picture: Rect,
) {
    let Some(frame) = app.output_size() else {
        return;
    };
    let resp = ui
        .interact(
            picture,
            Id::unique("videolore-subtitle-drag"),
            Sense::drag(),
        )
        .on_hover_text("Drag a subtitle to move it");
    let ctx = ui.ctx().clone();
    let shown = app.shown_project();
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
    let boxes = placement::boxes(&shown, app.playhead, frame, &mut measure);
    let to_frame = |p: Pos2| placement::to_frame(p, picture, frame);
    if let Some(p) = resp.hover_pos()
        && placement::hit(&boxes, to_frame(p)).is_some()
    {
        ctx.set_cursor_icon(CursorIcon::Grab);
    }
    // selected lines are outlined
    for (id, r) in &boxes {
        if app.selection.contains(id) {
            let s = placement::to_screen(*r, picture, frame);
            ui.painter().rect_stroke(
                s,
                2.0,
                Stroke::new(1.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Outside,
            );
        }
    }
    if resp.drag_started() {
        let origin = ui
            .input(|i| i.pointer.press_origin())
            .or(resp.interact_pointer_pos());
        cache.subtitle_drag = origin.and_then(|o| {
            let f = to_frame(o);
            placement::hit(&boxes, f).map(|(line, r)| placement::Drag {
                line,
                ids: app.drag_group(line),
                grab: f - r.center(),
                size: r.size(),
                frame,
            })
        });
    }
    let Some(d) = cache.subtitle_drag.clone() else {
        return;
    };
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let pos = pointer
        .map(|p| placement::drag_to(&d, to_frame(p)))
        .or(app.drag_pos.as_ref().map(|(_, p)| *p));
    if let Some(pos) = pos {
        let (fx, fy) = pos.fraction();
        let c = pos2(fx as f32 * frame.0 as f32, fy as f32 * frame.1 as f32);
        let ghost = placement::to_screen(Rect::from_center_size(c, d.size), picture, frame);
        ui.painter().rect_stroke(
            ghost,
            2.0,
            Stroke::new(2.0, Color32::YELLOW),
            egui::StrokeKind::Outside,
        );
        ctx.set_cursor_icon(CursorIcon::Grabbing);
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

/// Tooltip of an action: what it does and its shortcut ("Save the project (Ctrl+S)").
pub fn action_tooltip(ctx: &Context, app: &AppState, id: actions::ActionId) -> String {
    let tip = id.tooltip(app);
    match keymap::shortcut_text(ctx, id) {
        Some(s) => format!("{tip} ({s})"),
        None => tip,
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod ux_tests;
