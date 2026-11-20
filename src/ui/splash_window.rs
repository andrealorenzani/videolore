// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The splash window (1.6.0, REQ-133, REQ-134; shared with the player in 1.6.2, REQ-150):
//! the picture of a [`splash::Program`] with "Version X.Y" over it and, beside it, a panel
//! with the summary of the version and a Close button. It is an egui window above the
//! program (not an operating-system window), so the program stays usable. The summary text
//! is sized to fit its panel: nothing is clipped and there is no scroll. The caller turns
//! [`SplashOutcome::Close`] (Close button or the window's own close control) into its own
//! command; Esc is the caller's.

use super::icons;
use crate::splash::{self, Program};
use eframe::egui::{
    self, Align2, Color32, Context, FontId, Id, Order, Rect, TextureHandle, TextureOptions, pos2,
    vec2,
};
use std::time::Duration;

/// Where the parts of the splash were drawn this frame (screen pixels, for the tests).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplashRects {
    pub image: Rect,
    pub text: Rect,
    pub panel: Rect,
    /// The room of the summary in the panel, and the part of it the text uses.
    pub summary_room: Rect,
    pub summary_used: Rect,
}

/// What the user did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplashOutcome {
    Open,
    Close,
}

/// The image as textures, with its text.
pub struct Loaded {
    /// The summary text size fitted for a picture scale (kept until the scale changes).
    summary_fit: Option<(f32, f32)>,
    frames: Vec<(TextureHandle, Duration)>,
    size: (u32, u32),
    /// Colour of "Version X.Y" over this picture.
    text: Color32,
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded").field("size", &self.size).finish()
    }
}

/// Decodes the image and makes its textures (once, when the splash first opens).
fn load(ctx: &Context, p: &Program) -> Loaded {
    match splash::decode(p.image) {
        Ok(d) => {
            let c =
                splash::text_colour(splash::mean_colour(&d.frames[0].rgba, d.size, p.text_rect));
            let frames = d
                .frames
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [d.size.0 as usize, d.size.1 as usize],
                        &f.rgba,
                    );
                    (
                        ctx.load_texture(
                            format!("splash-{}-{i}", p.name),
                            img,
                            TextureOptions::LINEAR,
                        ),
                        Duration::from_millis(u64::from(f.delay_ms)),
                    )
                })
                .collect();
            Loaded {
                summary_fit: None,
                frames,
                size: d.size,
                text: Color32::from_rgb(c[0], c[1], c[2]),
            }
        }
        Err(_) => Loaded {
            summary_fit: None,
            frames: Vec::new(),
            // no picture: only the summary panel is shown
            size: (0, 506),
            text: Color32::WHITE,
        },
    }
}

/// The frame to show `elapsed` after the splash opened (all frames loop).
pub fn frame_index(delays: &[Duration], elapsed: Duration) -> usize {
    let total: Duration = delays.iter().sum();
    if delays.len() <= 1 || total.is_zero() {
        return 0;
    }
    let mut t = Duration::from_nanos((elapsed.as_nanos() % total.as_nanos()) as u64);
    for (i, d) in delays.iter().enumerate() {
        if t < *d {
            return i;
        }
        t -= *d;
    }
    0
}

/// Draws the splash window of `program`; `slot` keeps its textures between frames (decoded
/// on the first call). Returns what the user did and where the parts were drawn.
pub fn show(
    ctx: &Context,
    slot: &mut Option<Loaded>,
    program: &Program,
) -> (SplashOutcome, Option<SplashRects>) {
    let loaded = slot.get_or_insert_with(|| load(ctx, program));
    let screen = ctx.content_rect();
    // room for the title bar and the window's margins
    let avail = (screen.width() - 40.0, screen.height() - 70.0);
    let lay = splash::layout(loaded.size, avail);
    let mut open = true;
    let mut close = false;
    let mut drawn = None;
    let loaded = &mut *loaded;
    egui::Window::new(icons::label(icons::INFO, "videolore"))
        .id(Id::unique("videolore-splash"))
        .order(Order::Foreground)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.horizontal_top(|ui| {
                // the image, with the version over its lower rectangle
                let image_w = if loaded.frames.is_empty() {
                    0.0
                } else {
                    lay.image.0
                };
                let (rect, _) =
                    ui.allocate_exact_size(vec2(image_w, lay.image.1), egui::Sense::hover());
                if !loaded.frames.is_empty() {
                    let delays: Vec<Duration> = loaded.frames.iter().map(|f| f.1).collect();
                    let elapsed = Duration::from_secs_f64(ctx.input(|i| i.time));
                    let k = frame_index(&delays, elapsed);
                    ui.painter().image(
                        loaded.frames[k].0.id(),
                        rect,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    if delays.len() > 1 {
                        ctx.request_repaint_after(delays[k]);
                    }
                }
                let s = lay.scale;
                let t = program.text_rect;
                let text_rect = Rect::from_min_max(
                    rect.min + vec2(t.0 * s, t.1 * s),
                    rect.min + vec2(t.2 * s, t.3 * s),
                );
                let text = splash::current_version_text();
                if !loaded.frames.is_empty() {
                    let size = fit_version(ctx, &text, s, program);
                    ui.painter().text(
                        text_rect.center(),
                        Align2::CENTER_CENTER,
                        &text,
                        FontId::proportional(size),
                        loaded.text,
                    );
                }
                // the panel: exactly as wide as the layout says and as high as the image
                let (panel, _) =
                    ui.allocate_exact_size(vec2(lay.popup.0, lay.popup.1), egui::Sense::hover());
                let inner = panel.shrink(14.0 * s);
                let (used, room, clicked) =
                    summary_panel(ui, inner, s, program, &mut loaded.summary_fit);
                close |= clicked;
                drawn = Some(SplashRects {
                    image: rect,
                    text: text_rect,
                    panel,
                    summary_room: room,
                    summary_used: used,
                });
            });
        });
    let outcome = if !open || close {
        SplashOutcome::Close
    } else {
        SplashOutcome::Open
    };
    (outcome, drawn)
}

/// The font size at which `text` fills its rectangle best on a picture scaled by `s`: the
/// fit is made in image pixels (REQ-134, REQ-150: inside its rectangle), then scaled.
pub fn fit_version(ctx: &Context, text: &str, s: f32, p: &Program) -> f32 {
    let measure = |size: f32| {
        let g = ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_string(),
                FontId::proportional(size * s),
                Color32::WHITE,
            )
        });
        (g.size().x / s, g.size().y / s)
    };
    let t = p.text_rect;
    splash::fit_text(measure, (t.2 - t.0, t.3 - t.1), 200.0) * s
}

/// Text size of the summary items in image pixels at most, and the least.
pub const SUMMARY_MAX: f32 = 15.0;
pub const SUMMARY_FLOOR: f32 = 7.0;
/// Heading size relative to the items; gaps after the heading and after each item (image px).
const HEADING_RATIO: f32 = 1.35;
const GAP_HEADING: f32 = 8.0;
const GAP_ITEM: f32 = 6.0;
const BUTTON: egui::Vec2 = egui::vec2(96.0, 28.0);

fn item_text(line: &str) -> String {
    format!("{} {line}", icons::CARET_RIGHT)
}

/// The height of the summary at an item size (screen pixels) in `width`, with the gaps
/// scaled by `s`.
pub fn summary_height(ctx: &Context, lines: &[&str], size: f32, width: f32, s: f32) -> f32 {
    ctx.fonts_mut(|f| {
        let mut h = 0.0;
        for (i, l) in lines.iter().enumerate() {
            let (text, px, gap) = if i == 0 {
                (l.to_string(), size * HEADING_RATIO, GAP_HEADING * s)
            } else {
                (item_text(l), size, GAP_ITEM * s)
            };
            h += f
                .layout(text, FontId::proportional(px), Color32::WHITE, width)
                .size()
                .y
                + gap;
        }
        h
    })
}

/// Whether the summary lines fit `room` at the floor size (7 image pixels) on a picture
/// scaled by `s`: the rule that a shipped summary must keep (REQ-154).
pub fn summary_fits_at_floor(ctx: &Context, lines: &[&str], room: Rect, s: f32) -> bool {
    summary_height(ctx, lines, SUMMARY_FLOOR * s, room.width(), s) <= room.height() + 0.5
}

/// The summary (in the top of `inner`, sized to fit it: no scroll, nothing clipped) and the
/// Close button (at its bottom right). Returns the part used, the room, and Close clicked.
fn summary_panel(
    ui: &mut egui::Ui,
    inner: Rect,
    s: f32,
    p: &Program,
    fit: &mut Option<(f32, f32)>,
) -> (Rect, Rect, bool) {
    let top = Rect::from_min_max(inner.min, pos2(inner.max.x, inner.max.y - BUTTON.y - 8.0));
    let lines = splash::summary_lines(p);
    // fitted once per scale, not every frame
    let size = match *fit {
        Some((k, size)) if k == s => size,
        _ => {
            let ctx = ui.ctx().clone();
            let size = splash::fit_summary(
                |sz| summary_height(&ctx, &lines, sz, top.width(), s),
                top.height(),
                SUMMARY_MAX * s,
                SUMMARY_FLOOR * s,
            );
            *fit = Some((s, size));
            size
        }
    };
    let mut text = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(top)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    text.set_clip_rect(top.intersect(ui.clip_rect()));
    for (i, l) in lines.iter().enumerate() {
        if i == 0 {
            text.add(
                egui::Label::new(egui::RichText::new(*l).size(size * HEADING_RATIO).strong())
                    .wrap(),
            );
            text.add_space(GAP_HEADING * s);
        } else {
            text.add(egui::Label::new(egui::RichText::new(item_text(l)).size(size)).wrap());
            text.add_space(GAP_ITEM * s);
        }
    }
    let used = text.min_rect();
    let button = Rect::from_min_size(inner.max - BUTTON, BUTTON);
    let clicked = ui
        .put(button, egui::Button::new(icons::label(icons::X, "Close")))
        .on_hover_text("Close the splash screen (Esc)")
        .clicked();
    (used, top, clicked)
}
