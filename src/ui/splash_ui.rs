// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The splash window (1.6.0, REQ-133, REQ-134): the release image with "Version X.Y" over
//! it and, beside it, a panel with the summary of the version and a Close button. It is an
//! egui window above the editor (not an operating-system window), so the editor stays
//! usable; Esc, Close and the window's own close control all close it.

use super::{UiCache, icons};
use crate::app::{AppState, Command, Popup};
use crate::splash;
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
}

/// The image as textures, with its text.
pub struct Loaded {
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
fn load(ctx: &Context) -> Loaded {
    match splash::decode(splash::IMAGE) {
        Ok(d) => {
            let c = splash::text_colour(splash::mean_colour(
                &d.frames[0].rgba,
                d.size,
                splash::TEXT_RECT,
            ));
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
                        ctx.load_texture(format!("splash-{i}"), img, TextureOptions::LINEAR),
                        Duration::from_millis(u64::from(f.delay_ms)),
                    )
                })
                .collect();
            Loaded {
                frames,
                size: d.size,
                text: Color32::from_rgb(c[0], c[1], c[2]),
            }
        }
        Err(_) => Loaded {
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

/// Draws the splash window when [`Popup::Splash`] is open.
pub fn show(ctx: &Context, _app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    if cache.splash.is_none() {
        cache.splash = Some(load(ctx));
    }
    let Some(loaded) = cache.splash.as_ref() else {
        return;
    };
    let screen = ctx.content_rect();
    // room for the title bar and the window's margins
    let avail = (screen.width() - 40.0, screen.height() - 70.0);
    let lay = splash::layout(loaded.size, avail);
    let mut open = true;
    let mut drawn = None;
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
                let t = splash::TEXT_RECT;
                let text_rect = Rect::from_min_max(
                    rect.min + vec2(t.0 * s, t.1 * s),
                    rect.min + vec2(t.2 * s, t.3 * s),
                );
                let text = splash::current_version_text();
                if !loaded.frames.is_empty() {
                    let size = fit_version(ctx, &text, s);
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
                summary_panel(ui, inner, cmds);
                drawn = Some(SplashRects {
                    image: rect,
                    text: text_rect,
                    panel,
                });
            });
        });
    cache.splash_rects = drawn;
    if !open {
        cmds.push(Command::ClosePopup(Popup::Splash));
    }
}

/// The font size at which `text` fills its rectangle best on a picture scaled by `s`: the
/// fit is made in image pixels (REQ-134: inside 334 x 84), then scaled.
pub fn fit_version(ctx: &Context, text: &str, s: f32) -> f32 {
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
    let t = splash::TEXT_RECT;
    splash::fit_text(measure, (t.2 - t.0, t.3 - t.1), 200.0) * s
}

/// The summary (in the top of `inner`) and the Close button (at its bottom right).
fn summary_panel(ui: &mut egui::Ui, inner: Rect, cmds: &mut Vec<Command>) {
    const BUTTON: egui::Vec2 = egui::vec2(96.0, 28.0);
    let top = Rect::from_min_max(inner.min, pos2(inner.max.x, inner.max.y - BUTTON.y - 8.0));
    // no scroll area: the summary is a few lines (what does not fit is clipped)
    let mut text = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(top)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    text.set_clip_rect(top.intersect(ui.clip_rect()));
    let lines = splash::summary_lines();
    let mut it = lines.iter();
    if let Some(h) = it.next() {
        text.heading(*h);
        text.add_space(8.0);
    }
    for l in it {
        text.add(egui::Label::new(format!("{} {l}", icons::CARET_RIGHT)).wrap());
        text.add_space(6.0);
    }
    let button = Rect::from_min_size(inner.max - BUTTON, BUTTON);
    if ui
        .put(button, egui::Button::new(icons::label(icons::X, "Close")))
        .on_hover_text("Close the splash screen (Esc)")
        .clicked()
    {
        cmds.push(Command::ClosePopup(Popup::Splash));
    }
}
