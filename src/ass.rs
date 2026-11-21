// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Writes subtitles as an ASS file for ffmpeg's `subtitles` filter (libass), used both by
//! the preview and the export so they look the same.
//!
//! The coordinate space (`PlayResX/PlayResY`) is the export size; libass scales it to the
//! actual frame. Text is escaped so that a subtitle can never inject ASS override tags:
//! only this module emits `{\k..}` tags.

use crate::base::pos::{POS_MAX, Pos};
use crate::base::shadow::Shadow;
use crate::subtitles::{Preset, ShownLine, StyleSettings, Word};
use crate::time::Micros;

/// File name of the subtitles inside the ffmpeg working folder.
pub const ASS_FILE: &str = "subs.ass";
/// Folder (inside the ffmpeg working folder) holding the bundled font.
pub const FONTS_DIR: &str = "fonts";
/// File name of the bundled font.
pub const FONT_FILE: &str = "NotoSans-Bold.ttf";
/// Noto Sans Bold (SIL Open Font License 1.1): Latin, Greek and Cyrillic.
pub const FONT: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");
/// Noto Sans Arabic Bold: the bundled face for Arabic (REQ-187).
pub const FONT_ARABIC: &[u8] = include_bytes!("../assets/fonts/NotoSansArabic-Bold.ttf");
/// Noto Sans Devanagari Bold: the bundled face for Hindi (REQ-187).
pub const FONT_DEVANAGARI: &[u8] = include_bytes!("../assets/fonts/NotoSansDevanagari-Bold.ttf");
/// The family names of the bundled script faces (what `\fn` writes).
pub const FAMILY_ARABIC: &str = "Noto Sans Arabic";
pub const FAMILY_DEVANAGARI: &str = "Noto Sans Devanagari";

/// Every bundled face: the file name written into the fonts folder, and its bytes.
pub const BUNDLED_FACES: [(&str, &[u8]); 3] = [
    (FONT_FILE, FONT),
    ("NotoSansArabic-Bold.ttf", FONT_ARABIC),
    ("NotoSansDevanagari-Bold.ttf", FONT_DEVANAGARI),
];
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

/// [`escape`] plus the font of each Arabic and Devanagari run: the subtitle font has no
/// glyphs for them and libass does not look for another font of the fonts folder by itself,
/// so a run is written with its bundled face (`{\fnNoto Sans Arabic}`) and the base font
/// comes back after it. Other characters (CJK included) are left to the renderer's own
/// fallback to the installed fonts. Text without those scripts is returned as it was.
pub fn escape_scripts(text: &str) -> String {
    use crate::base::font::{DEFAULT_FONT, Script, script_of};
    let plain = escape(text);
    if !plain
        .chars()
        .any(|c| script_of(c).is_some_and(Script::is_bundled))
    {
        return plain;
    }
    let family = |s: Script| match s {
        Script::Arabic => FAMILY_ARABIC,
        _ => FAMILY_DEVANAGARI,
    };
    let mut out = String::with_capacity(plain.len() + 48);
    let mut current: Option<Script> = None;
    for c in plain.chars() {
        let next = match script_of(c) {
            Some(s) if s.is_bundled() => Some(s),
            // spaces, digits and punctuation stay in the run they are in; letters of any
            // other script go back to the base font
            None if !c.is_alphabetic() => current,
            _ => None,
        };
        if next != current {
            let font = next.map_or(DEFAULT_FONT, family);
            out.push_str(&format!("{{\\fn{font}}}"));
            current = next;
        }
        out.push(c);
    }
    if current.is_some() {
        out.push_str(&format!("{{\\fn{DEFAULT_FONT}}}"));
    }
    out
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
        Preset::WordHighlight | Preset::EnlargeWord | Preset::Typewriter => (h * 0.18).round(),
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

/// `&H00BBGGRR` of an opaque colour.
/// `px` pixels at a picture height of 1080, scaled to `height` (at least one).
pub fn scaled_px(px: u8, height: u32) -> u32 {
    ((f64::from(px) * f64::from(height) / 1080.0).round() as u32).max(1)
}

fn abgr(c: [u8; 3]) -> String {
    format!("&H00{:02X}{:02X}{:02X}", c[2], c[1], c[0])
}

/// The shadow distance in pixels on a picture `height` pixels high (REQ-306): the distance
/// at 1080 pixels, scaled with the height.
pub fn shadow_px(px: u8, height: u32) -> f64 {
    (f64::from(px) * f64::from(height) / 1080.0 * 100.0).round() / 100.0
}

fn style(preset: Preset, st: &StyleSettings, width: u32, height: u32) -> String {
    let size = font_size(width, height);
    let outline = outline(size);
    let h = f64::from(height);
    let margin_lr = margin_lr(width);
    // Colours are &HAABBGGRR (alpha 00 = opaque).
    let (primary, secondary, outline_c, mut back, border, margin_v) = match preset {
        Preset::Classic => (
            "&H00FFFFFF".to_string(),
            "&H00FFFFFF".to_string(),
            "&H00000000".to_string(),
            "&H00000000".to_string(),
            1,
            (h * 0.06).round(),
        ),
        Preset::Boxed => (
            "&H00FFFFFF".to_string(),
            "&H00FFFFFF".to_string(),
            "&H80000000".to_string(),
            "&H80000000".to_string(),
            3,
            (h * 0.25).round(),
        ),
        Preset::WordHighlight | Preset::Typewriter => (
            abgr(st.highlighted),
            abgr(st.not_yet),
            "&H00000000".to_string(),
            "&H00000000".to_string(),
            1,
            (h * 0.18).round(),
        ),
        Preset::EnlargeWord => (
            abgr(st.not_yet),
            abgr(st.not_yet),
            "&H00000000".to_string(),
            "&H00000000".to_string(),
            1,
            (h * 0.18).round(),
        ),
    };
    // the shadow is the style's Shadow and its BackColour (REQ-307); libass draws the box of
    // BorderStyle 3 from the outline colour, so the box keeps its own colour (spike S0d)
    let shadow = if st.shadow.px > 0 {
        back = abgr(st.shadow.colour);
        shadow_px(st.shadow.px, height)
    } else {
        0.0
    };
    format!(
        "Style: Default,Noto Sans,{size},{primary},{secondary},{outline_c},{back},-1,0,0,0,\
         100,100,0,0,{border},{outline},{shadow},2,{margin_lr},{margin_lr},{margin_v},1"
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

/// The chunks of a line for the word highlight (REQ-257): each is a few words with its
/// start and end, the words joined by spaces and the time of each word in hundredths of a
/// second (a word lasts until the next one starts).
pub fn highlight_chunks(l: &ShownLine) -> Vec<(Micros, Micros, String, Vec<u32>)> {
    let words = words_of(l);
    let chunks: Vec<&[Word]> = words.chunks(HIGHLIGHT_CHUNK).collect();
    let mut out = Vec::new();
    for (ci, chunk) in chunks.iter().enumerate() {
        let start = chunk[0].start.max(l.start);
        let end = chunks
            .get(ci + 1)
            .map(|n| n[0].start)
            .unwrap_or(l.end)
            .max(start + Micros(10_000));
        let mut cursor = start;
        let mut cs = Vec::new();
        for wi in 0..chunk.len() {
            let until = chunk.get(wi + 1).map(|n| n.start).unwrap_or(end);
            cs.push(((until - cursor).0.max(0) / 10_000).max(1) as u32);
            cursor = until;
        }
        let text = chunk
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        out.push((start, end, text, cs));
    }
    out
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
    document_with_texts(lines, preset, width, height, shift, &[])
}

/// [`document_with_texts`] with the default style settings (2.0.4 output).
pub fn document_with_texts(
    lines: &[ShownLine],
    preset: Preset,
    width: u32,
    height: u32,
    shift: Micros,
    texts: &[AssEvent],
) -> String {
    document_styled(
        lines,
        preset,
        &StyleSettings::default(),
        width,
        height,
        shift,
        texts,
    )
}

/// The `.ass` file of File > Export subtitles (REQ-181): every line with the project's
/// style, its own place (`\an5\pos`) and size (`\fs`), on a frame of `width` x `height`
/// (the project's), at timeline times.
pub fn export_file(lines: &[ShownLine], preset: Preset, width: u32, height: u32) -> String {
    export_file_styled(lines, preset, &StyleSettings::default(), width, height)
}

/// [`export_file`] with the project's style settings (shadow and colours in the styles).
pub fn export_file_styled(
    lines: &[ShownLine],
    preset: Preset,
    st: &StyleSettings,
    width: u32,
    height: u32,
) -> String {
    let mut sorted: Vec<ShownLine> = lines
        .iter()
        .filter(|l| !l.text.trim().is_empty())
        .cloned()
        .collect();
    sorted.sort_by_key(|l| (l.start, l.line));
    document_styled(&sorted, preset, st, width, height, Micros::ZERO, &[])
}

/// Text height at 100%: 5% of the picture height (assumption of REQ-111).
pub const BASE_HEIGHT: f64 = 0.05;

/// One ASS dialogue of a text: times on the timeline, tag block (without braces) and the
/// escaped text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssEvent {
    pub start: Micros,
    pub end: Micros,
    pub tags: String,
    pub text: String,
    /// Left and right margin of the event (sets the wrap width), when it has its own.
    pub margins: Option<(u32, u32)>,
    /// Drawn with the background-box style (REQ-158).
    pub boxed: bool,
    /// The shadow of the letters or of the box (REQ-306).
    pub shadow: Shadow,
}

impl AssEvent {
    /// The name of the style the event is written in: `Text` or `TextBox`, and with a
    /// shadow `Text_<px>_<RRGGBB>` or `TextBox_<px>_<RRGGBB>` (one style per combination).
    pub fn style_name(&self) -> String {
        style_name(self.boxed, self.shadow)
    }
}

fn style_name(boxed: bool, shadow: Shadow) -> String {
    let base = if boxed { TEXT_BOX_STYLE } else { TEXT_STYLE };
    if shadow.px == 0 {
        base.to_string()
    } else {
        let c = shadow.colour;
        format!("{base}_{}_{:02X}{:02X}{:02X}", shadow.px, c[0], c[1], c[2])
    }
}

/// Font size in pixels of an item on a picture `height` pixels high.
pub fn font_px(height: u32, size: u16) -> u32 {
    ((f64::from(height) * BASE_HEIGHT * f64::from(size) / f64::from(crate::subtitles::SIZE_NORMAL))
        .round() as u32)
        .max(6)
}

/// Styles of the text items (REQ-111, REQ-158): white, centred, no wrap of the renderer's
/// own; the outline or box width, the font and the size come from the tags of each event.
/// `Text` has a black outline; `TextBox` has a box behind the lines, black and 60% opaque
/// (libass draws the box in the outline colour), with the box width as its padding.
fn text_styles(width: u32, height: u32, texts: &[AssEvent]) -> String {
    let size = font_px(height, crate::subtitles::SIZE_NORMAL);
    let ml = margin_lr(width);
    let row = |name: &str, outline_c: &str, border: u8, shadow: Shadow| {
        // the shadow is the style's Shadow and BackColour (REQ-307); the box keeps its colour
        // in the outline colour (libass draws it from there: spike S0d)
        let (back, dist) = if shadow.px > 0 {
            (abgr(shadow.colour), shadow_px(shadow.px, height))
        } else {
            ("&HFF000000".to_string(), 0.0)
        };
        format!(
            "Style: {name},Noto Sans,{size},&H00FFFFFF,&H00FFFFFF,{outline_c},{back},0,0,0,0,\
             100,100,0,0,{border},{},{dist},5,{ml},{ml},0,1\n",
            outline(size)
        )
    };
    let mut s = row(TEXT_STYLE, "&H00000000", 1, Shadow::default());
    if texts.iter().any(|e| e.boxed) {
        s.push_str(&row(TEXT_BOX_STYLE, "&H66000000", 3, Shadow::default()));
    }
    let mut seen: Vec<String> = Vec::new();
    for e in texts.iter().filter(|e| e.shadow.px > 0) {
        let name = e.style_name();
        if seen.contains(&name) {
            continue;
        }
        let (outline_c, border) = if e.boxed {
            ("&H66000000", 3)
        } else {
            ("&H00000000", 1)
        };
        s.push_str(&row(&name, outline_c, border, e.shadow));
        seen.push(name);
    }
    s
}

/// Style names of the text items.
pub const TEXT_STYLE: &str = "Text";
pub const TEXT_BOX_STYLE: &str = "TextBox";

/// [`document`] plus the events of text items (timeline times, like the lines), with the
/// project's style settings (shadow, word colours, Enlarge word size: 2.1.0).
pub fn document_styled(
    lines: &[ShownLine],
    preset: Preset,
    st: &StyleSettings,
    width: u32,
    height: u32,
    shift: Micros,
    texts: &[AssEvent],
) -> String {
    let mut s = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {width}\nPlayResY: {height}\n\
         WrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\n\
         Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, \
         BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, \
         BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n{}\n{}\n\
         [Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        style(preset, st, width, height),
        if texts.is_empty() {
            String::new()
        } else {
            text_styles(width, height, texts)
        }
    );
    let at = |t: Micros| (t - shift).max(Micros::ZERO);
    for l in lines.iter().filter(|l| l.end > shift) {
        // only this module writes tags: the position comes from validated numbers
        let tag = line_tag(l, width, height);
        if !preset.follows_words() {
            let text = format!("{tag}{}", escape_scripts(&l.text));
            s.push_str(&dialogue(at(l.start), at(l.end), &text));
            continue;
        }
        if preset != Preset::WordHighlight {
            let words: Vec<crate::word_styles::Timed> = words_of(l)
                .into_iter()
                .map(|w| crate::word_styles::Timed {
                    start: w.start,
                    end: w.end,
                    text: w.text,
                })
                .collect();
            let span = (l.start, l.end);
            let events = if preset == Preset::Typewriter {
                crate::word_styles::typewriter(
                    &words,
                    span,
                    &tag,
                    (st.highlighted, st.not_yet),
                    shift,
                    &escape_scripts,
                )
            } else {
                let base = l.size.map_or_else(
                    || font_size(width, height),
                    |z| sized_font(width, height, z),
                );
                let big = base + scaled_px(st.enlarge, height);
                crate::word_styles::enlarge(&words, span, &tag, big, &escape_scripts)
            };
            for e in events.iter().filter(|e| e.end > shift) {
                s.push_str(&dialogue(at(e.start), at(e.end), &e.text));
            }
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
                text.push_str(&format!("{{\\k{cs}}}{} ", escape_scripts(&w.text)));
                cursor = until;
            }
            s.push_str(&dialogue(at(start), at(end), text.trim_end()));
        }
    }
    for e in texts.iter().filter(|e| e.end > shift) {
        let (ml, mr) = e.margins.unwrap_or((0, 0));
        s.push_str(&format!(
            "Dialogue: 1,{},{},{},,{ml},{mr},0,,{{{}}}{}\n",
            ass_time(at(e.start)),
            ass_time(at(e.end)),
            e.style_name(),
            e.tags,
            e.text
        ));
    }
    s
}

#[cfg(test)]
mod style_tests;

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
            anchor: None,
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
        assert_eq!(docs.len(), 5);
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

    /// A small ASS reader that does not share code with the writer: the `Format:` line of
    /// the Events section gives the columns, every `Dialogue:` line is split into them.
    fn reference_events(doc: &str) -> Vec<std::collections::BTreeMap<String, String>> {
        let events = doc.split("[Events]\n").nth(1).expect("an Events section");
        let mut lines = events.lines();
        let cols: Vec<String> = lines
            .next()
            .and_then(|l| l.strip_prefix("Format: "))
            .expect("a Format line")
            .split(", ")
            .map(str::to_string)
            .collect();
        lines
            .filter_map(|l| l.strip_prefix("Dialogue: "))
            .map(|l| {
                cols.iter()
                    .cloned()
                    .zip(l.splitn(cols.len(), ','))
                    .map(|(k, v)| (k, v.to_string()))
                    .collect()
            })
            .collect()
    }

    // covers: REQ-181
    #[test]
    fn the_ass_file_has_every_line_with_its_own_place_and_size() {
        let (w, h) = (1920, 1080);
        let mut a = line(1000, 2000, "at the corner", vec![]);
        a.position = Some(Pos { x: 1000, y: 1000 }); // 10% across and 10% down
        let mut b = line(3000, 4000, "big", vec![]);
        b.size = Some(1500);
        b.line = LineId(2);
        let c = line(5000, 6000, "plain", vec![]);
        let doc = export_file(&[c.clone(), b.clone(), a.clone()], Preset::Boxed, w, h);
        let events = reference_events(&doc);
        assert_eq!(events.len(), 3, "a reference reader reads all the lines");
        assert!(doc.contains("PlayResX: 1920\nPlayResY: 1080"));
        // in time order, with the project's style
        assert!(events[0]["Text"].ends_with("at the corner"));
        assert!(events.iter().all(|e| e["Style"] == "Default"));
        assert!(doc.contains("Style: Default,Noto Sans,59,"));
        // the place: x = 192, y = 108, within 1% of the frame
        let tag = &events[0]["Text"];
        let nums: Vec<f64> = tag
            .split("\\pos(")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap()
            .split(',')
            .map(|n| n.parse().unwrap())
            .collect();
        assert!(
            (nums[0] - 0.10 * f64::from(w)).abs() <= 0.01 * f64::from(w),
            "{tag}"
        );
        assert!(
            (nums[1] - 0.10 * f64::from(h)).abs() <= 0.01 * f64::from(h),
            "{tag}"
        );
        assert!(tag.starts_with("{\\an5"));
        // 150% of the style size (59 px)
        let fs: f64 = events[1]["Text"]
            .split("\\fs")
            .nth(1)
            .unwrap()
            .split('\\')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(fs, (59.0_f64 * 1.5).round());
        // empty lines are not written
        let e = line(7000, 8000, "  ", vec![]);
        assert_eq!(
            reference_events(&export_file(&[e], Preset::Classic, w, h)).len(),
            0
        );
    }

    // covers: REQ-187
    #[test]
    fn arabic_and_devanagari_runs_get_their_bundled_font_and_the_rest_is_untouched() {
        assert_eq!(escape_scripts("plain {text}"), escape("plain {text}"));
        assert_eq!(
            escape_scripts("日本語 한국어"),
            escape("日本語 한국어"),
            "CJK: the renderer's fallback"
        );
        let t = escape_scripts("hi مرحبا 12 नमस्ते end");
        assert_eq!(
            t,
            "hi {\\fnNoto Sans Arabic}مرحبا 12 {\\fnNoto Sans Devanagari}नमस्ते {\\fnNoto Sans}end"
        );
        // the text ends inside a run: the base font comes back
        assert_eq!(
            escape_scripts("مر"),
            "{\\fnNoto Sans Arabic}مر{\\fnNoto Sans}"
        );
        // lines use it
        let doc = document(
            &[line(0, 1000, "سلام", vec![])],
            Preset::Classic,
            640,
            360,
            Micros::ZERO,
        );
        assert!(doc.contains("{\\fnNoto Sans Arabic}سلام"));
        assert_eq!(BUNDLED_FACES.len(), 3);
        assert!(
            BUNDLED_FACES
                .iter()
                .all(|(n, b)| n.ends_with(".ttf") && b.len() > 100_000)
        );
    }

    // covers: REQ-187, REQ-031
    #[test]
    fn the_bundled_faces_are_the_ones_recorded_with_their_licence_and_fit_the_size_limit() {
        use sha2::{Digest, Sha256};
        let source = include_str!("../assets/fonts/SOURCE.txt");
        let notices = include_str!("../packaging/notices/third-party.txt");
        for (name, bytes) in BUNDLED_FACES {
            let hash: String = Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert!(source.contains(&hash), "{name}: SHA-256 is in SOURCE.txt");
            assert!(source.contains(name), "{name} is in SOURCE.txt");
        }
        assert!(source.contains("SIL Open Font License 1.1"));
        for n in ["Noto Sans Arabic Bold", "Noto Sans Devanagari Bold"] {
            assert!(notices.contains(n), "{n} is in the third-party notices");
        }
        // the scripts added in 2.0.4 add at most 25 MB (the installer check prints the size)
        let added = FONT_ARABIC.len() + FONT_DEVANAGARI.len();
        assert!(added < 25 * 1024 * 1024, "{added} bytes");
        assert!(include_str!("../scripts/package.sh").contains("the added fonts exceed 25 MB"));
        // no CJK face is bundled: Chinese, Japanese and Korean use the installed fonts
        assert!(source.contains("No CJK font is bundled"));
    }
}
