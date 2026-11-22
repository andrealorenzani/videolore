// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The text size comes from the text box and a maximum (2.2.0, REQ-318, REQ-319, REQ-323).

use super::*;

fn item(text: &str, width: u16, height: u16) -> TextItem {
    let mut t = TextItem::new(TextId(1), text.into(), Micros::ZERO);
    t.width = width;
    t.height = height;
    t
}

/// The texts of a 2.0.4 project: (text, box width, box height) and the font size in pixels
/// of the 2.0.4 build on a 1920 x 1080 picture, measured with the bundled font.
const GOLDEN_2_0_4: &[(&str, u16, u16, f64)] = &[
    ("Hi", 800, 200, 108.0),
    ("A short title", 800, 200, 108.0),
    ("Two\nlines of text", 800, 200, 108.0),
    (
        "A much longer text that has to wrap over several lines of the box",
        300,
        120,
        52.3,
    ),
    ("WWWWWWWW", 150, 60, 56.5),
    ("iiiiiiii 123", 200, 40, 43.2),
    ("Two\nlines", 300, 80, 43.1),
    ("Mixed Case 123", 500, 300, 108.0),
];

// covers: REQ-319
#[test]
fn texts_of_a_2_0_4_project_keep_their_sizes() {
    for (text, w, h, want) in GOLDEN_2_0_4 {
        let t = item(text, *w, *h);
        assert_eq!(t.max_size, None, "a 2.0.4 text has no maximum");
        let px = layout(&t, 1920, 1080).px;
        assert!((px - want).abs() < 0.01, "{text}: {px}");
    }
}

// covers: REQ-323
#[test]
fn a_maximum_replaces_the_default_cap_and_the_box_still_governs_the_size() {
    // "Hi" in a box that fits 15%: the default cap is 10%, a maximum of 20% lets it grow
    let mut t = item("Hi", 800, 200);
    assert!((layout(&t, 1920, 1080).px - 108.0).abs() < 0.01);
    t.max_size = Some(200);
    let big = layout(&t, 1920, 1080).px;
    assert!(big > 108.0 && big <= 216.0, "{big}");
    // a box that fits only 15%: a maximum of 20% does not raise it above the box's fit
    let mut small = item("Hi", 800, 150);
    small.max_size = Some(200);
    let fit = layout(&small, 1920, 1080).px;
    assert!(fit <= 150.0 * 1080.0 / 1000.0, "{fit}");
    // a maximum below the fit lowers it
    t.max_size = Some(50);
    assert!((layout(&t, 1920, 1080).px - 54.0).abs() < 0.01);
    // the maximum is not below the minimum size of the fit
    t.max_size = Some(20);
    assert!((layout(&t, 1920, 1080).px - 21.6).abs() < 0.01);
}

// covers: REQ-318
#[test]
fn the_block_is_centred_in_the_box_and_a_larger_box_gives_a_longer_slide() {
    let start_x = |w: u16| {
        let mut t = item("Hi", w, 200);
        t.effects.slide = Some((SlideDir::Left, Micros(500_000)));
        t.ttl = Micros(3_000_000);
        let ev = events(&t, 1920, 1080, box_px(&t, 1920, 1080), Micros(10_000_000));
        let tags = &ev[0].tags;
        let m = tags.split("\\move(").nth(1).expect(tags);
        let x: f64 = m.split(',').next().unwrap().parse().unwrap();
        (960.0 - x, tags.contains("\\an5"))
    };
    // the slide starts one box width away from the box's centre: a box twice as wide gives
    // a longer slide, and the text is anchored at its centre (an5)
    let ((short, c1), (long, c2)) = (start_x(400), start_x(800));
    assert!(c1 && c2);
    assert!(long > short, "{short} {long}");
}

// covers: REQ-318
#[test]
fn the_text_event_is_anchored_at_the_centre_of_its_box() {
    let t = item("Hi", 800, 200);
    let ev = events(&t, 1920, 1080, box_px(&t, 1920, 1080), Micros(10_000_000));
    let tags = &ev[0].tags;
    assert!(tags.contains("\\an5"), "{tags}");
    let (cx, cy) = (
        1920.0 * f64::from(t.place.x) / 10_000.0,
        1080.0 * f64::from(t.place.y) / 10_000.0,
    );
    assert!(tags.contains(&format!("\\pos({cx:.0},{cy:.0})")), "{tags}");
}
