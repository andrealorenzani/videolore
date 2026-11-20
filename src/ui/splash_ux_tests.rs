// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of the splash screen (1.6.0, REQ-133, REQ-134), through egui's
//! accessibility tree and the rectangles the splash records.

use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, state};
use crate::app::{Banner, Popup};
use crate::settings::StartNotice;
use eframe::egui::{Event, Key, Modifiers};

fn with_splash() -> Harness {
    let mut app = state(FakeDialogs::default());
    app.show_splash();
    Harness::new(app, 1280.0, 800.0)
}

// covers: REQ-133
#[test]
fn the_splash_opens_on_top_and_each_way_of_closing_it_removes_it() {
    let mut h = with_splash();
    assert!(h.app.popups.is_open(Popup::Splash));
    assert!(h.cache.splash_rects.is_some(), "drawn");
    assert!(h.has("Close"));
    // the Close button
    h.click_label("Close");
    assert!(!h.app.popups.is_open(Popup::Splash));
    h.idle(2);
    assert!(h.cache.splash_rects.is_none(), "not drawn any more");
    // Esc
    h.app.show_splash();
    h.idle(2);
    assert!(h.app.popups.is_open(Popup::Splash));
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.popups.is_open(Popup::Splash));
    // the window's own close control (its title bar button)
    h.app.show_splash();
    h.idle(2);
    let x = h.find("Close window").expect("the window's close control");
    h.click(x.rect.center());
    assert!(!h.app.popups.is_open(Popup::Splash));
    // closing never asks about unsaved changes, and the next start shows it again
    assert!(h.app.modal.is_none());
    let again = with_splash();
    assert!(again.app.popups.is_open(Popup::Splash));
}

// covers: REQ-133
#[test]
fn escape_closes_the_splash_before_another_popup_and_the_editor_stays_usable() {
    let mut h = with_splash();
    h.app.popups.show(Popup::Shortcuts);
    h.idle(2);
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.popups.is_open(Popup::Splash));
    assert!(
        h.app.popups.is_open(Popup::Shortcuts),
        "the other popup stays"
    );
    // the main window works while the splash is open: a menu click goes through
    h.app.show_splash();
    h.idle(2);
    let before = h.app.popups.is_open(Popup::Settings);
    h.app.popups.close(Popup::Shortcuts);
    h.idle(1);
    let settings = h.nodes.iter().find(|n| n.label == "File").cloned();
    assert!(settings.is_some() && !before);
    h.click(settings.unwrap().rect.center());
    assert!(h.has("Quit"), "{:#?}", h.labels());
}

// covers: REQ-133
#[test]
fn the_first_start_banner_waits_for_the_splash() {
    let mut app = state(FakeDialogs::default());
    app.banner = Some(Banner::from_notice(StartNotice::Setup));
    app.show_splash();
    assert!(app.banner.is_none() && app.banner_after_splash.is_some());
    app.tick(Instant::now());
    assert!(app.banner.is_none(), "never on top of the splash");
    app.popups.close(Popup::Splash);
    app.tick(Instant::now());
    assert!(app.banner.is_some() && app.banner_after_splash.is_none());
    // no banner due: nothing is held
    let mut plain = state(FakeDialogs::default());
    plain.show_splash();
    assert!(plain.banner_after_splash.is_none());
}

// covers: REQ-133
#[test]
fn a_start_with_a_path_shows_no_splash_and_a_start_without_one_shows_it() {
    let with = crate::real_app(Some("/nowhere/p.vlproj".into()));
    assert!(!with.state.popups.is_open(Popup::Splash));
    let without = crate::real_app(None);
    assert!(without.state.popups.is_open(Popup::Splash));
    crate::desktop::take_open_requests();
    // a file opened by the system while the splash is open closes it
    let mut app = state(FakeDialogs::default());
    app.open_requests = || vec!["/nowhere/q.vlproj".into()];
    app.show_splash();
    app.tick(Instant::now());
    assert!(!app.popups.is_open(Popup::Splash));
}

// covers: REQ-134
#[test]
fn the_popup_is_400_wide_and_as_high_as_the_image_and_the_version_fits_its_rectangle() {
    let h = with_splash();
    let r = h.cache.splash_rects.unwrap();
    assert!((r.panel.width() - 400.0).abs() <= 1.0, "{:?}", r.panel);
    assert!(
        (r.panel.height() - r.image.height()).abs() <= 1.0,
        "{:?} {:?}",
        r.panel,
        r.image
    );
    assert!((r.image.width() - 486.0).abs() <= 1.0 && (r.image.height() - 506.0).abs() <= 1.0);
    // the text rectangle is 334 x 84 inside the image, at x 74, y 381
    assert!((r.text.width() - 334.0).abs() <= 1.0 && (r.text.height() - 84.0).abs() <= 1.0);
    assert!((r.text.min.x - r.image.min.x - 74.0).abs() <= 1.0);
    assert!((r.text.min.y - r.image.min.y - 381.0).abs() <= 1.0);
    // "Version 1.6" at its fitted size is entirely inside the rectangle
    let ctx = &h.ctx;
    let size = fit_version_for_test(ctx, 1.0);
    let g = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            crate::splash::current_version_text(),
            eframe::egui::FontId::proportional(size),
            eframe::egui::Color32::WHITE,
        )
    });
    assert!(g.size().x <= r.text.width() - 2.0 * crate::splash::TEXT_MARGIN + 0.5);
    assert!(g.size().y <= r.text.height() - 2.0 * crate::splash::TEXT_MARGIN + 0.5);
    assert!(size > 20.0, "a large size, not 1: {size}");
    // the summary written for this version is in the panel
    assert!(
        h.has(crate::splash::summary_lines(&crate::splash::EDITOR)[0]),
        "{:#?}",
        h.labels()
    );
}

// covers: REQ-134
#[test]
fn the_summary_fits_its_panel_without_clipping_or_scrolling() {
    for (w, hgt) in [(1280.0, 800.0), (800.0, 500.0)] {
        let mut app = state(FakeDialogs::default());
        app.show_splash();
        let mut h = Harness::new(app, w, hgt);
        h.idle(2);
        let r = h.cache.splash_rects.unwrap();
        assert!(
            r.summary_used.height() <= r.summary_room.height() + 0.5,
            "{r:?}"
        );
        assert!(
            r.summary_used.width() <= r.summary_room.width() + 0.5,
            "{r:?}"
        );
        // every line of the summary is on screen
        for l in crate::splash::summary_lines(&crate::splash::EDITOR) {
            assert!(h.labels().iter().any(|x| x.contains(&l[..15])), "{l}");
        }
    }
}

fn fit_version_for_test(ctx: &eframe::egui::Context, s: f32) -> f32 {
    crate::ui::splash_window::fit_version(
        ctx,
        &crate::splash::current_version_text(),
        s,
        &crate::splash::EDITOR,
    )
}

// covers: REQ-134
#[test]
fn on_a_small_window_both_parts_are_scaled_together() {
    let mut h = Harness::new(
        {
            let mut a = state(FakeDialogs::default());
            a.show_splash();
            a
        },
        800.0,
        500.0,
    );
    h.idle(2);
    let r = h.cache.splash_rects.unwrap();
    assert!(r.image.height() < 506.0 && r.image.height() <= 500.0);
    assert!((r.panel.width() / r.image.width() - 400.0 / 486.0).abs() < 0.01);
    assert!((r.panel.height() - r.image.height()).abs() <= 1.0);
    // Close is still there and the Esc key still works
    h.frame(vec![Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle(2);
    assert!(!h.app.popups.is_open(Popup::Splash));
}

#[test]
fn frames_of_an_animated_image_loop_and_a_single_frame_stays() {
    use crate::ui::splash_window::frame_index;
    use std::time::Duration as D;
    let d = [D::from_millis(100), D::from_millis(300)];
    assert_eq!(frame_index(&d, D::from_millis(0)), 0);
    assert_eq!(frame_index(&d, D::from_millis(150)), 1);
    assert_eq!(frame_index(&d, D::from_millis(399)), 1);
    assert_eq!(frame_index(&d, D::from_millis(400)), 0);
    assert_eq!(frame_index(&d[..1], D::from_secs(9)), 0);
    assert_eq!(frame_index(&[], D::from_secs(9)), 0);
}
