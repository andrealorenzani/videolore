// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Where a popup opens (REQ-235, REQ-236): the place the user left it at, if it is on the
//! screen, else a default place that keeps clear of the timeline's track area when the
//! window is at least 1280x800. Pure geometry.

use crate::settings::interface::Window;
use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

/// Smallest window (points) for which a popup keeps clear of the track area.
pub const BIG_WINDOW: (f32, f32) = (1280.0, 800.0);
/// Space kept around a popup.
pub const MARGIN: f32 = 8.0;
/// Space kept above the popup (menu bar and toolbar).
pub const TOP: f32 = 96.0;

/// Where a popup opens and how tall it may be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    /// Top-left corner, or `None` to centre it in the window (small windows).
    pub pos: Option<Pos2>,
    pub max_height: f32,
}

/// The place saved by the user, when part of the popup would be seen in `screen`.
pub fn saved(pos: (f32, f32), size: Vec2, screen: Rect) -> Option<Pos2> {
    let p = pos2(pos.0, pos.1);
    let seen = p.x + size.x > screen.min.x + 40.0
        && p.x < screen.max.x - 40.0
        && p.y >= screen.min.y
        && p.y < screen.max.y - 40.0;
    (p.x.is_finite() && p.y.is_finite() && seen).then_some(p)
}

/// The width a popup of `width` gets by default: in a big window a wide popup is made as
/// narrow as 340 points so that it fits the column right of the track area.
pub fn fit_width(screen: Rect, track: Rect, width: f32) -> f32 {
    if screen.width() < BIG_WINDOW.0 || screen.height() < BIG_WINDOW.1 {
        return width;
    }
    width.min((screen.max.x - track.max.x - MARGIN * 2.0).max(340.0))
}

/// The default place of the `n`th open popup of width `width` in a window `screen` whose
/// timeline section is `track` (the track area). In a window of at least 1280x800 the popup
/// is in the column right of the track area if it is narrow enough, else above it (its
/// height limited so that it ends before the timeline); in a smaller window it is
/// centred, as before.
pub fn default_place(screen: Rect, track: Rect, width: f32, n: usize) -> Place {
    let centred = Place {
        pos: None,
        max_height: (screen.height() - 90.0).max(80.0),
    };
    if screen.width() < BIG_WINDOW.0 || screen.height() < BIG_WINDOW.1 {
        return centred;
    }
    let step = 24.0 * (n % 5) as f32;
    let right_room = screen.max.x - track.max.x - MARGIN * 2.0;
    if width <= right_room {
        let x = screen.max.x - width - MARGIN - step;
        return Place {
            pos: Some(pos2(x.max(track.max.x + MARGIN), screen.min.y + TOP + step)),
            max_height: (screen.height() - TOP - 90.0 - step).max(80.0),
        };
    }
    // above the timeline: the popup ends before it starts
    let y = screen.min.y + TOP + step;
    Place {
        pos: Some(pos2(
            (screen.center().x - width / 2.0 + step).clamp(screen.min.x, screen.max.x - width),
            y,
        )),
        max_height: (track.min.y - y - MARGIN).max(80.0),
    }
}

/// The window to remember after this frame: its size and place from the viewport, or, while
/// maximised, the size and place it had before (REQ-234). `None` when nothing changed or
/// the viewport tells nothing.
pub fn window_to_save(
    saved: Option<Window>,
    inner: Option<Rect>,
    outer: Option<Rect>,
    maximised: bool,
) -> Option<Window> {
    let inner = inner?;
    let new = if maximised {
        Window {
            maximised: true,
            ..saved.unwrap_or(Window {
                width: inner.width().round(),
                height: inner.height().round(),
                x: None,
                y: None,
                maximised: true,
            })
        }
    } else {
        let at = outer.map_or(inner.min, |o| o.min);
        Window {
            width: inner.width().round(),
            height: inner.height().round(),
            x: Some(at.x.round()),
            y: Some(at.y.round()),
            maximised: false,
        }
    };
    (new.is_valid() && saved != Some(new)).then_some(new)
}

/// The window options for the saved window (REQ-234): its size, its place when it is not
/// obviously outside every screen, and whether it was maximised.
pub fn viewport_of(w: Option<Window>) -> (Vec2, Option<Pos2>, bool) {
    match w.filter(Window::is_valid) {
        Some(w) => (
            vec2(w.width.max(800.0), w.height.max(500.0)),
            // the screens are not known before the window exists: only absurd places are left out
            w.x.zip(w.y)
                .filter(|(x, y)| x.abs() <= 16384.0 && y.abs() <= 16384.0)
                .map(|(x, y)| pos2(x, y)),
            w.maximised,
        ),
        None => (vec2(1280.0, 800.0), None, false),
    }
}

/// The rectangle a popup of height `height` (at most the place's limit) takes.
pub fn rect_of(place: Place, screen: Rect, width: f32, height: f32) -> Rect {
    let h = height.min(place.max_height);
    match place.pos {
        Some(p) => Rect::from_min_size(p, vec2(width, h)),
        None => Rect::from_center_size(screen.center(), vec2(width, h)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::{self, Arrangement, Fractions, MINS};

    // covers: REQ-236
    #[test]
    fn default_popups_keep_clear_of_the_track_area_in_big_windows() {
        for (w, h) in [(1280.0, 800.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
            // the window below the menu bar, toolbar and status bar
            let area = Rect::from_min_max(pos2(0.0, 64.0), pos2(w, h - 24.0));
            let sections = layout::arrange(area, Fractions::DEFAULT, MINS, Arrangement::default());
            let screen = Rect::from_min_size(Pos2::ZERO, vec2(w, h));
            for width in [340.0, 460.0] {
                let width = fit_width(screen, sections.timeline, width);
                for n in 0..5 {
                    let place = default_place(screen, sections.timeline, width, n);
                    // the tallest a popup gets: as tall as it may be
                    let r = rect_of(place, screen, width, 5000.0);
                    assert!(
                        !r.intersects(sections.timeline),
                        "{w}x{h} {width} {n}: {r:?}"
                    );
                    assert!(screen.contains_rect(r), "{w}x{h} {width} {n}: {r:?}");
                }
            }
        }
        // a smaller window keeps the centred place
        let small = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0));
        assert_eq!(default_place(small, small, 340.0, 0).pos, None);
    }

    // covers: REQ-234
    #[test]
    fn the_window_is_saved_and_restored_and_an_off_screen_place_falls_back() {
        let inner = Rect::from_min_size(pos2(12.0, 34.0), vec2(1500.0, 900.0));
        let w = window_to_save(None, Some(inner), None, false).unwrap();
        assert_eq!(
            (w.width, w.height, w.x, w.y),
            (1500.0, 900.0, Some(12.0), Some(34.0))
        );
        assert_eq!(
            window_to_save(Some(w), Some(inner), None, false),
            None,
            "nothing changed"
        );
        // maximised: the size it had before is kept
        let max = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
        let m = window_to_save(Some(w), Some(max), None, true).unwrap();
        assert!(m.maximised && m.width == 1500.0);
        assert_eq!(window_to_save(Some(w), None, None, false), None);
        // restart: the options carry what was saved, through the settings file
        let i = crate::settings::Interface {
            window: Some(m),
            ..Default::default()
        };
        let back = crate::settings::Interface::from_value(&serde_json::to_value(&i).unwrap());
        let (size, pos, maximised) = viewport_of(back.window);
        assert_eq!(
            (size, pos, maximised),
            (vec2(1500.0, 900.0), Some(pos2(12.0, 34.0)), true)
        );
        assert_eq!(viewport_of(None).0, vec2(1280.0, 800.0));
        // a place outside the screens opens the window at the system's place
        let far = Window {
            x: Some(30000.0),
            ..m
        };
        assert!(far.visible_position((1920.0, 1080.0)).is_none());
        assert_eq!(
            viewport_of(Some(Window {
                x: Some(16000.0),
                y: Some(50.0),
                ..w
            }))
            .1,
            Some(pos2(16000.0, 50.0))
        );
    }

    // covers: REQ-235
    #[test]
    fn a_saved_place_is_used_only_when_it_is_on_the_screen() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
        let size = vec2(340.0, 300.0);
        assert_eq!(saved((100.0, 80.0), size, screen), Some(pos2(100.0, 80.0)));
        assert_eq!(saved((5000.0, 80.0), size, screen), None);
        assert_eq!(saved((100.0, -400.0), size, screen), None);
        assert_eq!(saved((-1000.0, 80.0), size, screen), None);
        assert_eq!(saved((f32::NAN, 80.0), size, screen), None);
    }
}
