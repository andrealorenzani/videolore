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

/// The words drawn while each word is spoken: `(left, bottom, size, text)` of every event,
/// `n` per spoken word.
fn drawn(doc: &str, n: usize) -> Vec<Vec<(f64, f64, i64, String)>> {
    let all: Vec<(f64, f64, i64, String)> = doc
        .lines()
        .filter(|l| l.starts_with("Dialogue"))
        .map(|l| {
            let tags = l.split("\\pos(").nth(1).unwrap();
            let (x, rest) = tags.split_once(',').unwrap();
            let (y, rest) = rest.split_once(')').unwrap();
            let size = rest.split("\\fs").nth(1).unwrap();
            let (size, text) = size.split_once('}').unwrap();
            (
                x.parse().unwrap(),
                y.parse().unwrap(),
                size.parse().unwrap(),
                text.to_string(),
            )
        })
        .collect();
    all.chunks(n).map(<[_]>::to_vec).collect()
}

// covers: REQ-309
#[test]
fn enlarge_word_writes_one_event_per_word_for_each_spoken_word_with_the_larger_size() {
    let l = [spoken()];
    let st = StyleSettings {
        enlarge: 20,
        ..StyleSettings::default()
    };
    let d = document_styled(&l, Preset::EnlargeWord, &st, 1920, 1080, Micros::ZERO, &[]);
    let events: Vec<&str> = d.lines().filter(|l| l.starts_with("Dialogue")).collect();
    assert_eq!(events.len(), 9);
    let base = i64::from(font_size(1920, 1080));
    let groups = drawn(&d, 3);
    for (i, g) in groups.iter().enumerate() {
        for (j, w) in g.iter().enumerate() {
            let want = if i == j { base + 20 } else { base };
            assert_eq!(w.2, want, "word {j} while word {i} is spoken");
        }
    }
    // the group of the second word is shown from 1 s to 2 s
    assert!(events[3].contains("0:00:01.00,0:00:02.00"));
    // at 720 the growth is scaled with the height: 20 px -> 13
    let d = document_styled(&l, Preset::EnlargeWord, &st, 1280, 720, Micros::ZERO, &[]);
    assert!(d.contains(&format!("\\fs{}", font_size(1280, 720) + 13)));
}

// covers: REQ-334, REQ-309
#[test]
fn the_enlarged_word_keeps_its_base_and_its_centre_and_the_neighbours_make_room() {
    let l = spoken();
    let (h, w) = (1080.0, 1920.0);
    let px = f64::from(font_size(1920, 1080));
    let timed: Vec<crate::word_styles::Timed> = words_of(&l)
        .into_iter()
        .map(|x| crate::word_styles::Timed {
            start: x.start,
            end: x.end,
            text: x.text,
        })
        .collect();
    // the line at its normal size: the reference of every check
    let block = word_block(
        &l,
        &timed,
        Preset::EnlargeWord,
        font_size(1920, 1080),
        1920,
        1080,
    )
    .expect("the words can be measured");
    let normal = block.with_enlarged(99, px);
    let widths: Vec<f64> = block.slots.iter().map(|s| s.width).collect();
    // the largest growth
    let st = StyleSettings {
        enlarge: 40,
        ..StyleSettings::default()
    };
    let d = document_styled(
        &[l],
        Preset::EnlargeWord,
        &st,
        1920,
        1080,
        Micros::ZERO,
        &[],
    );
    let g = drawn(&d, 3);
    let big = g[1][1].2 as f64;
    assert!(big > px + 30.0, "{big}");
    let extra = widths[1] * (big / px - 1.0);
    let descent = block.descent;
    // the base: the bottom of the letters is where it is at normal size (2% of the height)
    let baseline = |bottom: f64, size: f64| bottom - descent * size;
    for i in 0..3 {
        let spoken = &g[i][i];
        let at_normal = baseline(normal[i].bottom, px);
        assert!(
            (baseline(spoken.1, spoken.2 as f64) - at_normal).abs() < 0.02 * h,
            "word {i}"
        );
    }
    // its centre is where the word's centre is at normal size (0.5% of the width)
    let centre = |left: f64, width: f64| left + width / 2.0;
    let was = centre(normal[1].left, widths[1]);
    let now = centre(g[1][1].0, widths[1] * big / px);
    assert!((was - now).abs() < 0.005 * w, "{was} {now}");
    // the word before moves left and the word after right, by half the extra width (1%)
    let before = normal[0].left - g[1][0].0;
    let after = g[1][2].0 - normal[2].left;
    assert!((before - extra / 2.0).abs() < 0.01 * w, "{before} {extra}");
    assert!((after - extra / 2.0).abs() < 0.01 * w, "{after} {extra}");
    // at the largest growth no two words overlap, whichever word is spoken
    for (i, row) in g.iter().enumerate() {
        for j in 0..2 {
            let right = row[j].0 + widths[j] * row[j].2 as f64 / px;
            assert!(
                right <= row[j + 1].0 + 0.5,
                "words {j} and {} at {i}",
                j + 1
            );
        }
    }
    // the line keeps its centre
    let mid = |row: &[(f64, f64, i64, String)]| {
        (row[0].0 + row[2].0 + widths[2] * row[2].2 as f64 / px) / 2.0
    };
    let mid_normal = (normal[0].left + normal[2].left + widths[2]) / 2.0;
    for row in &g {
        assert!(
            (mid(row) - mid_normal).abs() < 1.0,
            "{} {mid_normal}",
            mid(row)
        );
    }
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
    assert!(d.contains("\\bord"), "a sized line keeps its outline");
    let g = drawn(&d, 3);
    assert!((g[0][0].1 - 270.0).abs() < 0.1 * h, "{:?}", g[0][0]);
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
