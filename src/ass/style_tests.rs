// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The 2.1.0 styles in the ASS output: shadows, word colours, Enlarge word, Typewriter.

use super::*;
use crate::subtitles::LineId;
use crate::subtitles::tests::word;
use crate::timeline::ClipId;

fn spoken() -> ShownLine {
    ShownLine {
        line: LineId(1),
        clip: ClipId(1),
        start: Micros::ZERO,
        end: Micros(3_000_000),
        text: "one two three".into(),
        words: vec![
            word(0, 1000, "one"),
            word(1000, 2000, "two"),
            word(2000, 3000, "three"),
        ],
        position: None,
        size: None,
        anchor: None,
    }
}

fn style_line(doc: &str) -> &str {
    doc.lines()
        .find(|l| l.starts_with("Style: Default"))
        .unwrap()
}

fn fields(line: &str) -> Vec<&str> {
    line["Style: ".len()..].split(',').collect()
}

fn settings(shadow: u8, colour: [u8; 3]) -> StyleSettings {
    StyleSettings {
        shadow: Shadow { px: shadow, colour },
        ..StyleSettings::default()
    }
}

// covers: REQ-307
#[test]
fn the_style_has_the_shadow_distance_and_colour_and_zero_keeps_the_2_0_4_style() {
    let l = [spoken()];
    let plain = document(&l, Preset::Classic, 1920, 1080, Micros::ZERO);
    let same = document_styled(
        &l,
        Preset::Classic,
        &StyleSettings::default(),
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    assert_eq!(plain, same);
    assert!(style_line(&plain).contains(",&H00000000,&H00000000,-1,"));
    // 6 px at 1080 on a 1080 picture: Shadow 6, BackColour the shadow colour (BGR)
    let d = document_styled(
        &l,
        Preset::Classic,
        &settings(6, [0x10, 0x20, 0x30]),
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    let f = fields(style_line(&d));
    assert_eq!(f[6], "&H00302010", "BackColour");
    assert_eq!(f[17], "6", "Shadow");
    // scaled with the picture height (720 -> two thirds)
    let d = document_styled(
        &l,
        Preset::Classic,
        &settings(6, [0, 0, 0]),
        1280,
        720,
        Micros::ZERO,
        &[],
    );
    assert_eq!(fields(style_line(&d))[17], "4");
    // the box of the Box style stays in the outline colour, its shadow is the back colour
    let d = document_styled(
        &l,
        Preset::Boxed,
        &settings(8, [255, 0, 0]),
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    let f = fields(style_line(&d));
    assert_eq!((f[5], f[6], f[17]), ("&H80000000", "&H000000FF", "8"));
    assert_eq!(f[15], "3", "BorderStyle");
}

// covers: REQ-306
#[test]
fn a_text_with_a_shadow_has_its_own_style_and_without_one_the_old_style() {
    let ev = |boxed, px: u8| AssEvent {
        start: Micros::ZERO,
        end: Micros(1_000_000),
        tags: String::new(),
        text: "t".into(),
        margins: None,
        boxed,
        shadow: Shadow {
            px,
            colour: [255, 0, 0],
        },
    };
    let none = document_with_texts(
        &[],
        Preset::Classic,
        1920,
        1080,
        Micros::ZERO,
        &[ev(false, 0)],
    );
    assert!(none.contains("Dialogue: 1,0:00:00.00,0:00:01.00,Text,"));
    assert!(!none.contains("Text_"));
    let d = document_with_texts(
        &[],
        Preset::Classic,
        1920,
        1080,
        Micros::ZERO,
        &[ev(false, 8), ev(true, 8)],
    );
    assert!(d.contains("Style: Text_8_FF0000,Noto Sans,"));
    assert!(d.contains("Style: TextBox_8_FF0000,Noto Sans,"));
    let st = d
        .lines()
        .find(|l| l.starts_with("Style: TextBox_8_FF0000"))
        .unwrap();
    let f = fields(st);
    assert_eq!(
        (f[5], f[6], f[15], f[17]),
        ("&H66000000", "&H000000FF", "3", "8")
    );
    assert!(d.contains(",TextBox_8_FF0000,,"));
}

// covers: REQ-308
#[test]
fn the_word_colours_are_the_primary_and_secondary_colours_of_the_style() {
    let l = [spoken()];
    let default = document(&l, Preset::WordHighlight, 1920, 1080, Micros::ZERO);
    assert!(style_line(&default).contains(",&H0000FFFF,&H00FFFFFF,"));
    let st = StyleSettings {
        highlighted: [255, 0, 0],
        not_yet: [0, 0, 255],
        ..StyleSettings::default()
    };
    let d = document_styled(
        &l,
        Preset::WordHighlight,
        &st,
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    assert!(
        style_line(&d).contains(",&H000000FF,&H00FF0000,"),
        "{}",
        style_line(&d)
    );
}

// covers: REQ-309
#[test]
fn enlarge_word_writes_two_events_per_word_with_the_larger_size() {
    let l = [spoken()];
    let st = StyleSettings {
        enlarge: 20,
        ..StyleSettings::default()
    };
    let d = document_styled(&l, Preset::EnlargeWord, &st, 1920, 1080, Micros::ZERO, &[]);
    let events: Vec<&str> = d.lines().filter(|l| l.starts_with("Dialogue")).collect();
    assert_eq!(events.len(), 6);
    let base = font_size(1920, 1080);
    assert!(
        events[1].contains(&format!("\\fs{}", base + 20)),
        "{}",
        events[1]
    );
    assert!(events[1].contains("0:00:00.00,0:00:01.00"));
    // at 720 the growth is scaled with the height: 20 px -> 13
    let d = document_styled(&l, Preset::EnlargeWord, &st, 1280, 720, Micros::ZERO, &[]);
    assert!(d.contains(&format!("\\fs{}", font_size(1280, 720) + 13)));
    // the other words keep their size: only the second event sets one
    assert!(!events[0].contains("\\fs"));
}

// covers: REQ-333, REQ-309
#[test]
fn the_enlarged_word_is_drawn_at_its_measured_centre_inside_two_percent_of_the_height() {
    let l = [spoken()];
    let st = StyleSettings::default();
    let d = document_styled(&l, Preset::EnlargeWord, &st, 1920, 1080, Micros::ZERO, &[]);
    let events: Vec<&str> = d.lines().filter(|l| l.starts_with("Dialogue")).collect();
    // the big event is the word alone at an own position; the base keeps the whole line
    let big = events[1];
    assert!(big.contains("\\an5\\pos("), "{big}");
    assert!(big.ends_with("}one"), "{big}");
    assert!(!events[0].contains("\\pos"), "{}", events[0]);
    // the centre of every word is on the line's own height (single line): the style's place
    let h = 1080.0;
    let px = f64::from(font_size(1920, 1080));
    let expect_y = h - margin_v(Preset::EnlargeWord, 1080) - px / 2.0;
    for e in events.iter().skip(1).step_by(2) {
        let at = e.split("\\pos(").nth(1).unwrap();
        let (x, rest) = at.split_once(',').unwrap();
        let y: f64 = rest.split(')').next().unwrap().parse().unwrap();
        assert!((y - expect_y).abs() < 0.02 * h, "{e}");
        let x: f64 = x.parse().unwrap();
        assert!(x > 0.0 && x < 1920.0);
    }
    // words go left to right
    let xs: Vec<f64> = events
        .iter()
        .skip(1)
        .step_by(2)
        .map(|e| {
            let at = e.split("\\pos(").nth(1).unwrap();
            at.split_once(',').unwrap().0.parse().unwrap()
        })
        .collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
    // a line with its own place and size is centred on that place
    let mut own = spoken();
    own.position = Some(crate::base::pos::Pos { x: 5000, y: 2500 });
    own.size = Some(1500);
    let d = document_styled(
        &[own],
        Preset::EnlargeWord,
        &st,
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    let big = d
        .lines()
        .filter(|l| l.starts_with("Dialogue"))
        .nth(1)
        .unwrap();
    assert!(big.contains("\\bord"), "{big}");
    let y: f64 = big
        .split("\\pos(")
        .nth(1)
        .unwrap()
        .split_once(',')
        .unwrap()
        .1
        .split(')')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((y - 270.0).abs() < 0.02 * h, "{big}");
}

// covers: REQ-310
#[test]
fn typewriter_writes_one_event_per_word_with_the_letters_on_their_times() {
    let l = [spoken()];
    let d = document(&l, Preset::Typewriter, 1920, 1080, Micros::ZERO);
    let events: Vec<&str> = d.lines().filter(|l| l.starts_with("Dialogue")).collect();
    assert_eq!(events.len(), 3);
    // "three": 5 letters spread over 1 s: 20 cs each
    assert_eq!(events[2].matches("\\ko20}").count(), 5, "{}", events[2]);
    // shifted: the time at which the video starts is taken off
    let late = document(&l, Preset::Typewriter, 1920, 1080, Micros(1_500_000));
    assert_eq!(
        late.lines().filter(|l| l.starts_with("Dialogue")).count(),
        2
    );
    // a shadow and the project colours reach it too
    let d = document_styled(
        &l,
        Preset::Typewriter,
        &settings(4, [1, 2, 3]),
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    assert!(style_line(&d).contains(",&H0000FFFF,&H00FFFFFF,"));
}
