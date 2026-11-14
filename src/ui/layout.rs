// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The fixed five-panel layout. Proportions are kept on every resize.

use eframe::egui::{Rect, pos2};

/// Top row height (media list + preview).
pub const TOP: f32 = 0.60;
/// Middle row height (timeline + effects/settings).
pub const MIDDLE: f32 = 0.30;
/// Media list width inside the top row.
pub const MEDIA_WIDTH: f32 = 0.30;
/// Timeline width inside the middle row.
pub const TIMELINE_WIDTH: f32 = 0.70;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelRects {
    pub media: Rect,
    pub preview: Rect,
    pub timeline: Rect,
    pub effects: Rect,
    pub command_bar: Rect,
}

/// Splits `r`: top 60% (media 30% | preview 70%), middle 30% (timeline 70% | effects 30%),
/// bottom 10% command bar.
pub fn panels(r: Rect) -> PanelRects {
    let (x0, x1, w, h) = (r.min.x, r.max.x, r.width(), r.height());
    let y_top = r.min.y + h * TOP;
    let y_mid = r.min.y + h * (TOP + MIDDLE);
    let x_media = x0 + w * MEDIA_WIDTH;
    let x_tl = x0 + w * TIMELINE_WIDTH;
    PanelRects {
        media: Rect::from_min_max(r.min, pos2(x_media, y_top)),
        preview: Rect::from_min_max(pos2(x_media, r.min.y), pos2(x1, y_top)),
        timeline: Rect::from_min_max(pos2(x0, y_top), pos2(x_tl, y_mid)),
        effects: Rect::from_min_max(pos2(x_tl, y_top), pos2(x1, y_mid)),
        command_bar: Rect::from_min_max(pos2(x0, y_mid), r.max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    // covers: REQ-024
    #[test]
    fn proportions_hold_at_any_size() {
        for (w, h) in [(1280.0, 800.0), (640.0, 480.0), (2560.0, 1440.0)] {
            let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(w, h));
            let p = panels(r);
            let near = |a: f32, b: f32| (a - b).abs() < 0.01;
            assert!(near(p.media.height(), h * 0.6));
            assert!(near(p.media.width(), w * 0.3));
            assert!(near(p.preview.width(), w * 0.7));
            assert!(near(p.timeline.height(), h * 0.3));
            assert!(near(p.timeline.width(), w * 0.7));
            assert!(near(p.effects.width(), w * 0.3));
            assert!(near(p.command_bar.height(), h * 0.1));
            assert!(near(p.command_bar.width(), w));
            // no overlaps, full coverage
            let area = p.media.area()
                + p.preview.area()
                + p.timeline.area()
                + p.effects.area()
                + p.command_bar.area();
            assert!((area - w * h).abs() < 1.0);
            assert!(!p.media.intersects(p.preview.shrink(0.5)));
            assert_eq!(p.command_bar.max, r.max);
        }
    }
}
