// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Theme and text size of the interface (REQ-243, REQ-244, ADR-196). The colours are the
//! two [`visuals`] below; `System` lets the system decide which of them is used. The text
//! size scales the egui text styles, never the zoom factor, so the timeline zoom is
//! untouched.

use crate::settings::interface::{Interface, TextSize, Theme};
use eframe::egui::{self, Color32, Context, Id, Visuals};

/// The colours of the dark or the light look. Body text is an explicit colour with a
/// contrast of at least 4.5 to 1 on every background it is drawn on ([`pairs`]).
pub fn visuals(dark: bool) -> Visuals {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    if dark {
        v.override_text_color = Some(Color32::from_gray(232));
        v.panel_fill = Color32::from_gray(27);
        v.window_fill = Color32::from_gray(32);
        v.extreme_bg_color = Color32::from_gray(10);
    } else {
        v.override_text_color = Some(Color32::from_gray(20));
        v.panel_fill = Color32::from_gray(246);
        v.window_fill = Color32::from_gray(252);
        v.extreme_bg_color = Color32::WHITE;
    }
    v
}

fn channel(c: u8) -> f64 {
    let s = f64::from(c) / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(c: Color32) -> f64 {
    0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
}

/// The WCAG contrast ratio of two colours (1 to 21).
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// The text colour with each background it is drawn on, for the contrast test.
pub fn pairs(v: &Visuals) -> Vec<(Color32, Color32)> {
    let text = v.override_text_color.unwrap_or(v.text_color());
    [v.panel_fill, v.window_fill, v.extreme_bg_color]
        .into_iter()
        .map(|bg| (text, bg))
        .collect()
}

fn key() -> Id {
    Id::unique("videolore-look")
}

/// Applies the theme and the text size when they differ from what is applied.
pub fn apply(ctx: &Context, i: &Interface) {
    let want = (i.theme, i.text_size);
    if ctx.data(|d| d.get_temp::<(Theme, TextSize)>(key())) == Some(want) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(key(), want));
    ctx.set_theme(match i.theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
    let scale = i.text_size.scale();
    for (theme, dark) in [(egui::Theme::Dark, true), (egui::Theme::Light, false)] {
        ctx.set_visuals_of(theme, visuals(dark));
        ctx.style_mut_of(theme, |s| {
            for (style, font) in egui::Style::default().text_styles {
                s.text_styles
                    .insert(style, egui::FontId::new(font.size * scale, font.family));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-243
    #[test]
    fn both_themes_reach_the_contrast_of_the_text_and_its_backgrounds() {
        for dark in [true, false] {
            let v = visuals(dark);
            assert_eq!(v.dark_mode, dark);
            for (fg, bg) in pairs(&v) {
                assert!(contrast(fg, bg) >= 4.5, "{dark}: {fg:?} on {bg:?}");
            }
        }
        assert!(contrast(Color32::BLACK, Color32::WHITE) > 20.9);
    }

    fn frame(ctx: &Context, i: &Interface, system: Option<egui::Theme>) -> (bool, f32) {
        let raw = egui::RawInput {
            system_theme: system,
            ..Default::default()
        };
        let mut height = 0.0;
        let mut dark = false;
        let out = ctx.run_ui(raw, |ui| {
            apply(ui.ctx(), i);
        });
        out.drop_without_applying_deltas();
        let raw = egui::RawInput {
            system_theme: system,
            ..Default::default()
        };
        let out = ctx.run_ui(raw, |ui| {
            dark = ui.visuals().dark_mode;
            let g = egui::WidgetText::from("Label").into_galley(
                ui,
                None,
                f32::INFINITY,
                egui::TextStyle::Body,
            );
            height = g.size().y;
        });
        out.drop_without_applying_deltas();
        (dark, height)
    }

    // covers: REQ-243
    #[test]
    fn each_choice_sets_the_colours_and_system_follows_the_system() {
        let ctx = Context::default();
        let mut i = Interface {
            theme: Theme::Dark,
            ..Default::default()
        };
        assert!(frame(&ctx, &i, None).0);
        i.theme = Theme::Light;
        assert!(!frame(&ctx, &i, None).0);
        i.theme = Theme::System;
        assert!(frame(&ctx, &i, Some(egui::Theme::Dark)).0);
        let ctx2 = Context::default();
        assert!(!frame(&ctx2, &i, Some(egui::Theme::Light)).0);
    }

    // covers: REQ-244
    #[test]
    fn larger_text_makes_labels_one_and_a_half_times_as_high() {
        let ctx = Context::default();
        let mut i = Interface::default();
        let normal = frame(&ctx, &i, None).1;
        i.text_size = TextSize::Larger;
        let larger = frame(&ctx, &i, None).1;
        i.text_size = TextSize::Large;
        let large = frame(&ctx, &i, None).1;
        assert!((larger / normal - 1.5).abs() < 0.08, "{larger} / {normal}");
        assert!((large / normal - 1.25).abs() < 0.08, "{large} / {normal}");
    }
}
