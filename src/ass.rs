// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Writes subtitles as an ASS file for ffmpeg's `subtitles` filter (libass), used both by
//! the preview and the export so they look the same.
//!
//! The coordinate space (`PlayResX/PlayResY`) is the export size; libass scales it to the
//! actual frame. Text is escaped so that a subtitle can never inject ASS override tags:
//! only this module emits `{\k..}` tags.

use crate::subtitles::{POS_MAX, Pos, Preset, ShownLine, Word};
use crate::time::Micros;

/// File name of the subtitles inside the ffmpeg working folder.
pub const ASS_FILE: &str = "subs.ass";
/// Folder (inside the ffmpeg working folder) holding the bundled font.
pub const FONTS_DIR: &str = "fonts";
/// File name of the bundled font.
pub const FONT_FILE: &str = "NotoSans-Bold.ttf";
/// Noto Sans Bold (SIL Open Font License 1.1): Latin, Greek and Cyrillic.
pub const FONT: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");
/// Words shown at once by the word-highlight preset.
pub const HIGHLIGHT_CHUNK: usize = 4;

/// Font size in pixels for a `width` x `height` frame: readable both in 9:16 and 16:9.
pub fn font_size(width: u32, height: u32) -> u32 {
    let by_h = 0.055 * f64::from(height);
    let by_w = 0.09 * f64::from(width);
    (by_h.min(by_w).round() as u32).max(8)
}

/// Escapes user text: braces and backslashes would start ASS override tags.
pub fn escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '{' => '(',
            '}' => ')',
            '\\' => '\u{FF3C}',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect()
}

/// `H:MM:SS.cc` (centiseconds, rounded down).
pub fn ass_time(t: Micros) -> String {
    let cs = t.0.max(0) / 10_000;
    format!(
        "{}:{:02}:{:02}.{:02}",
        cs / 360_000,
        (cs / 6_000) % 60,
        (cs / 100) % 60,
        cs % 100
    )
}

/// Outline width (pixels) of the text at font size `size`; with the box style it is the
/// padding of the box around the text.
pub fn outline(size: u32) -> f64 {
    (f64::from(size) * 0.08).round().max(1.0)
}

/// Left and right margin (pixels): the widest a line may be is `width - 2 * margin`.
pub fn margin_lr(width: u32) -> u32 {
    (f64::from(width) * 0.06).round() as u32
}

/// Distance (pixels) of the bottom of the text from the bottom of the picture.
pub fn margin_v(preset: Preset, height: u32) -> f64 {
    let h = f64::from(height);
    match preset {
        Preset::Classic => (h * 0.06).round(),
        Preset::Boxed => (h * 0.25).round(),
        Preset::WordHighlight => (h * 0.18).round(),
    }
}

/// Extra space (pixels) around the text that belongs to its box on screen (outline or
/// background box), on each side.
pub fn box_padding(_preset: Preset, size: u32) -> f64 {
    outline(size)
}

/// Centre (pixels) of a line's text box of height `box_h` at the style's place.
pub fn default_centre(preset: Preset, width: u32, height: u32, box_h: f64) -> (f64, f64) {
    (
        f64::from(width) / 2.0,
        f64::from(height) - margin_v(preset, height) - box_h / 2.0,
    )
}

/// The `{\an5\pos(x,y)}` tag of a line's own position (picture of `width` x `height`).
pub fn pos_tag(p: Pos, width: u32, height: u32) -> String {
    let c = |v: u16, size: u32| {
        (f64::from(v.min(POS_MAX)) * f64::from(size) / f64::from(POS_MAX)).round() as u32
    };
    format!("{{\\an5\\pos({},{})}}", c(p.x, width), c(p.y, height))
}

/// Font size in pixels of a line sized `size` thousandths of the preset's (SIZE_NORMAL =
/// the preset's own size).
pub fn sized_font(width: u32, height: u32, size: u16) -> u32 {
    let base = f64::from(font_size(width, height));
    ((base * f64::from(size) / 1000.0).round() as u32).max(1)
}

/// The override tag block of a line: its position and size (`\an5\pos(x,y)` and/or
/// `\fs..\bord..`). Empty for a line with neither, which keeps the 1.2 output unchanged.
/// A sized line without a position keeps the style's bottom anchor.
pub fn line_tag(l: &ShownLine, width: u32, height: u32) -> String {
    let pos = l.position.map(|p| pos_tag(p, width, height));
    let Some(size) = l.size else {
        return pos.unwrap_or_default();
    };
    let fs = sized_font(width, height, size);
    let sized = format!("\\fs{fs}\\bord{}", outline(fs));
    match pos {
        Some(p) => format!("{}{sized}}}", &p[..p.len() - 1]),
        None => format!("{{{sized}}}"),
    }
}

fn style(preset: Preset, width: u32, height: u32) -> String {
    let size = font_size(width, height);
    let outline = outline(size);
    let h = f64::from(height);
    let margin_lr = margin_lr(width);
    // Colours are &HAABBGGRR (alpha 00 = opaque).
    let (primary, secondary, outline_c, back, border, margin_v) = match preset {
        Preset::Classic => (
            "&H00FFFFFF",
            "&H00FFFFFF",
            "&H00000000",
            "&H00000000",
            1,
            (h * 0.06).round(),
        ),
        Preset::Boxed => (
            "&H00FFFFFF",
            "&H00FFFFFF",
            "&H80000000",
            "&H80000000",
            3,
            (h * 0.25).round(),
        ),
        Preset::WordHighlight => (
            "&H0000FFFF",
            "&H00FFFFFF",
            "&H00000000",
            "&H00000000",
            1,
            (h * 0.18).round(),
        ),
    };
    format!(
        "Style: Default,Noto Sans,{size},{primary},{secondary},{outline_c},{back},-1,0,0,0,\
         100,100,0,0,{border},{outline},0,2,{margin_lr},{margin_lr},{margin_v},1"
    )
}

/// Words of a line: the recognised ones, or the text split evenly over the line.
pub fn words_of(l: &ShownLine) -> Vec<Word> {
    if !l.words.is_empty() {
        return l.words.clone();
    }
    let parts: Vec<&str> = l.text.split_whitespace().collect();
    let n = parts.len() as i64;
    let len = (l.end - l.start).0;
    parts
        .iter()
        .enumerate()
        .map(|(i, w)| Word {
            start: l.start + Micros(len * i as i64 / n.max(1)),
            end: l.start + Micros(len * (i as i64 + 1) / n.max(1)),
            text: (*w).to_string(),
        })
        .collect()
}

fn dialogue(start: Micros, end: Micros, text: &str) -> String {
    format!(
        "Dialogue: 0,{},{},Default,,0,0,0,,{text}\n",
        ass_time(start),
        ass_time(end)
    )
}

/// The ASS document for `lines` (timeline times), with every time moved back by `shift`
/// (the timeline time where the rendered video starts).
pub fn document(
    lines: &[ShownLine],
    preset: Preset,
    width: u32,
    height: u32,
    shift: Micros,
) -> String {
    let mut s = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {width}\nPlayResY: {height}\n\
         WrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\n\
         Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, \
         BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, \
         BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n{}\n\n\
         [Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        style(preset, width, height)
    );
    let at = |t: Micros| (t - shift).max(Micros::ZERO);
    for l in lines.iter().filter(|l| l.end > shift) {
        // only this module writes tags: the position comes from validated numbers
        let tag = line_tag(l, width, height);
        if preset != Preset::WordHighlight {
            let text = format!("{tag}{}", escape(&l.text));
            s.push_str(&dialogue(at(l.start), at(l.end), &text));
            continue;
        }
        let words = words_of(l);
        let chunks: Vec<&[Word]> = words.chunks(HIGHLIGHT_CHUNK).collect();
        for (ci, chunk) in chunks.iter().enumerate() {
            let start = chunk[0].start.max(l.start);
            let end = chunks
                .get(ci + 1)
                .map(|n| n[0].start)
                .unwrap_or(l.end)
                .max(start + Micros(10_000));
            if end <= shift {
                continue;
            }
            let mut text = tag.clone();
            let mut cursor = start;
            for (wi, w) in chunk.iter().enumerate() {
                // each word lasts until the next one starts (gaps go to the previous word)
                let until = chunk.get(wi + 1).map(|n| n.start).unwrap_or(end);
                let cs = ((until - cursor).0.max(0) / 10_000).max(1);
                text.push_str(&format!("{{\\k{cs}}}{} ", escape(&w.text)));
                cursor = until;
            }
            s.push_str(&dialogue(at(start), at(end), text.trim_end()));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subtitles::LineId;
    use crate::subtitles::tests::word;
    use crate::timeline::ClipId;

    fn line(start: i64, end: i64, text: &str, words: Vec<Word>) -> ShownLine {
        ShownLine {
            line: LineId(1),
            clip: ClipId(1),
            start: Micros(start * 1000),
            end: Micros(end * 1000),
            text: text.into(),
            words,
            position: None,
            size: None,
        }
    }

    // covers: REQ-017
    #[test]
    fn three_presets_and_size_for_vertical_and_horizontal() {
        // 9:16: limited by the width; 16:9: limited by the height
        assert_eq!(font_size(1080, 1920), 97);
        assert_eq!(font_size(1920, 1080), 59);
        assert!(font_size(10, 10) >= 8);
        let lines = [line(1000, 3000, "hello world", vec![])];
        let docs: Vec<String> = Preset::ALL
            .iter()
            .map(|p| document(&lines, *p, 1080, 1920, Micros::ZERO))
            .collect();
        assert_eq!(docs.len(), 3);
        assert!(
            docs.iter()
                .all(|d| d.contains("PlayResX: 1080\nPlayResY: 1920"))
        );
        assert!(docs.iter().all(|d| d.contains("Noto Sans,97,")));
        assert!(docs[0].contains(",1,8,0,2,"), "classic: outline");
        assert!(docs[1].contains(",3,8,0,2,"), "box: opaque box style");
        assert!(docs[0].contains("Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,hello world"));
        // word highlight: karaoke tags, yellow primary colour
        assert!(docs[2].contains("&H0000FFFF"));
        assert!(docs[2].contains("{\\k100}hello {\\k100}world"));
        let h = document(&lines, Preset::Classic, 1920, 1080, Micros::ZERO);
        assert!(h.contains("Noto Sans,59,"));
    }

    // covers: REQ-017
    #[test]
    fn word_highlight_uses_word_timings_in_chunks() {
        let words = vec![
            word(0, 400, "one"),
            word(500, 900, "two"),
            word(900, 1200, "three"),
            word(1200, 1500, "four"),
            word(1600, 2000, "five"),
        ];
        let d = document(
            &[line(0, 2000, "one two three four five", words)],
            Preset::WordHighlight,
            1920,
            1080,
            Micros::ZERO,
        );
        assert!(d.contains(
            "0:00:00.00,0:00:01.60,Default,,0,0,0,,{\\k50}one {\\k40}two {\\k30}three {\\k40}four\n"
        ));
        assert!(d.contains("0:00:01.60,0:00:02.00,Default,,0,0,0,,{\\k40}five\n"));
    }

    #[test]
    fn escaping_times_and_shift() {
        assert_eq!(escape("a{\\b}c\n"), "a(\u{FF3C}b)c ");
        assert_eq!(ass_time(Micros(3_723_456_789)), "1:02:03.45");
        assert_eq!(ass_time(Micros(-5)), "0:00:00.00");
        let lines = [
            line(1000, 2000, "gone", vec![]),
            line(2500, 4000, "{\\an8}evil", vec![]),
        ];
        let d = document(&lines, Preset::Boxed, 640, 360, Micros(3_000_000));
        assert!(!d.contains("gone"));
        assert!(d.contains("0:00:00.00,0:00:01.00,Default,,0,0,0,,(\u{FF3C}an8)evil"));
        assert!(!d.contains("{\\an8}"));
        let d = document(&lines, Preset::WordHighlight, 640, 360, Micros(3_000_000));
        assert!(!d.contains("gone"));
        assert!(d.contains("evil"));
        assert!(FONT.len() > 100_000);
    }

    // covers: REQ-089, REQ-017
    #[test]
    fn own_positions_become_position_tags() {
        let mut l = line(1000, 3000, "hello {world}", vec![]);
        let plain = document(&[l.clone()], Preset::Classic, 1280, 720, Micros::ZERO);
        assert!(!plain.contains("\\pos"), "no position: exactly as before");
        l.position = Some(Pos { x: 2500, y: 2000 });
        let d = document(&[l.clone()], Preset::Classic, 1280, 720, Micros::ZERO);
        assert!(d.contains(",,{\\an5\\pos(320,144)}hello (world)"), "{d}");
        // word highlight: every group of words carries the tag
        let words: Vec<Word> = (0..6).map(|i| word(i * 300, i * 300 + 250, "w")).collect();
        let mut wl = line(0, 1800, "w w w w w w", words);
        wl.position = Some(Pos { x: 10_000, y: 0 });
        let d = document(&[wl], Preset::WordHighlight, 1000, 500, Micros::ZERO);
        assert_eq!(d.matches("{\\an5\\pos(1000,0)}{\\k").count(), 2, "{d}");
        assert_eq!(
            pos_tag(Pos { x: 60_000, y: 5000 }, 100, 100),
            "{\\an5\\pos(100,50)}"
        );
        // the style's place: centred, above the bottom margin
        let (x, y) = default_centre(Preset::Classic, 1280, 720, 40.0);
        assert_eq!(
            (x, y),
            (640.0, 720.0 - margin_v(Preset::Classic, 720) - 20.0)
        );
        assert!(box_padding(Preset::Boxed, 40) >= 1.0);
        assert_eq!(margin_lr(1000), 60);
    }

    // covers: REQ-097
    #[test]
    fn sizes_become_font_size_tags_and_none_changes_nothing() {
        let mut l = line(1000, 3000, "big", vec![]);
        l.position = Some(Pos { x: 5000, y: 5000 });
        let base = document(&[l.clone()], Preset::Classic, 1280, 720, Micros::ZERO);
        // 720p: base size 40 px; 200% = 80 px, outline scaled with it
        l.size = Some(2000);
        let d = document(&[l.clone()], Preset::Classic, 1280, 720, Micros::ZERO);
        assert!(d.contains("{\\an5\\pos(640,360)\\fs80\\bord6}big"), "{d}");
        assert_ne!(d, base);
        // size alone keeps the style's anchor
        l.position = None;
        let d = document(&[l.clone()], Preset::Classic, 1280, 720, Micros::ZERO);
        assert!(d.contains(",,{\\fs80\\bord6}big"), "{d}");
        // word highlight: every group carries it
        let words: Vec<Word> = (0..6).map(|i| word(i * 300, i * 300 + 250, "w")).collect();
        let mut wl = line(0, 1800, "w w w w w w", words);
        wl.size = Some(500);
        let d = document(&[wl], Preset::WordHighlight, 1000, 500, Micros::ZERO);
        assert_eq!(d.matches("{\\fs14\\bord1}{\\k").count(), 2, "{d}");
        assert_eq!(sized_font(1280, 720, 1000), font_size(1280, 720));
        assert_eq!(sized_font(10, 10, 250), 2);
    }
}
