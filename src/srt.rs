// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! SubRip (`.srt`): writing the timeline's subtitles (File > Export subtitles) and reading
//! the text subtitle track of a video (converted to SRT by ffmpeg).
//!
//! The reader takes untrusted input: it never panics, caps the number of cues and the text
//! of each one, skips damaged blocks and refuses impossible times.

use crate::subtitles::{MAX_TEXT, ShownLine, clean_text};
use crate::time::Micros;

/// Most cues read from one track.
pub const MAX_CUES: usize = 100_000;
/// Largest hour value accepted in a time stamp.
const MAX_HOURS: i64 = 99_999;

/// One subtitle of a track, in the track's (source) time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    pub start: Micros,
    pub end: Micros,
    /// Plain text: tags removed, on one line, cleaned like typed text.
    pub text: String,
}

/// `HH:MM:SS,mmm` (milliseconds rounded down; hours may exceed 99).
pub fn srt_time(t: Micros) -> String {
    let ms = t.0.max(0) / 1000;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

/// The SRT file of `lines` (timeline times): numbered from 1 in time order, one text line
/// per cue, LF line ends, UTF-8 without BOM. Lines without text are skipped.
pub fn write(lines: &[ShownLine]) -> String {
    let mut sorted: Vec<&ShownLine> = lines.iter().filter(|l| !l.text.trim().is_empty()).collect();
    sorted.sort_by_key(|l| (l.start, l.line));
    let mut out = String::new();
    for (i, l) in sorted.iter().enumerate() {
        let start = Micros(l.start.0.max(0) / 1000 * 1000);
        let end = Micros(l.end.0 / 1000 * 1000).max(start + Micros(1000));
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_time(start),
            srt_time(end),
            clean_text(&l.text)
        ));
    }
    out
}

/// Removes ASS override blocks (`{...}`) and HTML-like tags (`<i>`, `</font>`), turns the
/// ASS line breaks `\N`, `\n` and hard spaces `\h` into spaces, joins the lines and cleans
/// the result like typed text.
pub fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{'
            && let Some(close) = chars[i..].iter().position(|x| *x == '}')
        {
            i += close + 1;
            continue;
        }
        if c == '<'
            && chars.get(i + 1).is_some_and(|n| {
                n.is_ascii_alphabetic()
                    || (*n == '/' && chars.get(i + 2).is_some_and(char::is_ascii_alphabetic))
            })
            && let Some(close) = chars[i..].iter().position(|x| *x == '>')
        {
            i += close + 1;
            continue;
        }
        if c == '\\' && matches!(chars.get(i + 1), Some('N' | 'n' | 'h')) {
            out.push(' ');
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    clean_text(&out)
}

/// Parses `HH:MM:SS,mmm` (also `.` before the milliseconds, and `MM:SS,mmm`).
fn parse_time(s: &str) -> Option<Micros> {
    let s = s.trim();
    let (hms, ms) = s.split_once([',', '.'])?;
    if ms.is_empty() || ms.len() > 3 || !ms.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // "5" after the comma means 500 ms
    let ms: i64 = ms.parse::<i64>().ok()? * 10_i64.pow(3 - ms.len() as u32);
    let parts: Vec<&str> = hms.split(':').collect();
    let num = |p: &str| -> Option<i64> {
        if p.is_empty() || p.len() > 6 || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        p.parse().ok()
    };
    let (h, m, sec) = match parts.as_slice() {
        [h, m, s] => (num(h)?, num(m)?, num(s)?),
        [m, s] => (0, num(m)?, num(s)?),
        _ => return None,
    };
    if h > MAX_HOURS || m >= 60 || sec >= 60 {
        return None;
    }
    let total_ms = h
        .checked_mul(3_600_000)?
        .checked_add(m * 60_000)?
        .checked_add(sec * 1000)?
        .checked_add(ms)?;
    Some(Micros(total_ms.checked_mul(1000)?))
}

/// Reads the `start --> end` line of a cue.
fn parse_timing(line: &str) -> Option<(Micros, Micros)> {
    let (a, b) = line.split_once("-->")?;
    // anything after the end time (positions of some writers) is ignored
    let b = b.split_whitespace().next()?;
    Some((parse_time(a)?, parse_time(b)?))
}

/// Parses an SRT document leniently: CRLF or LF, an optional BOM, missing or wrong cue
/// numbers, damaged blocks (skipped). Cues whose end is not after their start are dropped;
/// at most [`MAX_CUES`] are kept, each at most [`MAX_TEXT`] characters.
pub fn parse(text: &str) -> Vec<Cue> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut cues = Vec::new();
    let mut current: Option<(Micros, Micros, String)> = None;
    let flush = |cur: &mut Option<(Micros, Micros, String)>, cues: &mut Vec<Cue>| {
        if let Some((start, end, body)) = cur.take() {
            let text = strip_tags(&body);
            if end > start && !text.is_empty() {
                cues.push(Cue {
                    start,
                    end,
                    text: text.chars().take(MAX_TEXT).collect(),
                });
            }
        }
    };
    for raw in text.lines() {
        if cues.len() >= MAX_CUES {
            break;
        }
        let line = raw.trim_end_matches('\r');
        if let Some((s, e)) = parse_timing(line) {
            flush(&mut current, &mut cues);
            current = Some((s, e, String::new()));
            continue;
        }
        if line.trim().is_empty() {
            flush(&mut current, &mut cues);
            continue;
        }
        if let Some((_, _, body)) = &mut current {
            // the text is capped while it is read: a huge cue cannot use much memory
            if body.len() < MAX_TEXT * 8 {
                if !body.is_empty() {
                    body.push(' ');
                }
                body.push_str(line);
            }
        }
        // anything else (cue numbers, junk before the first timing) is ignored
    }
    if cues.len() < MAX_CUES {
        flush(&mut current, &mut cues);
    }
    cues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subtitles::LineId;
    use crate::timeline::ClipId;

    fn shown(id: u64, a: i64, b: i64, text: &str) -> ShownLine {
        ShownLine {
            line: LineId(id),
            clip: ClipId(1),
            start: Micros(a),
            end: Micros(b),
            text: text.into(),
            words: vec![],
            position: None,
            size: None,
        }
    }

    // covers: REQ-088
    #[test]
    fn writes_numbered_cues_in_time_order_without_bom() {
        let lines = [
            shown(2, 3_723_456_789, 3_725_000_000, "later"),
            shown(1, 1_500, 2_000_999, "first  line"),
            shown(3, 5_000_000, 6_000_000, "   "),
            shown(4, 1_500, 2_000_000, "same start, higher id"),
        ];
        let out = write(&lines);
        assert_eq!(
            out,
            "1\n00:00:00,001 --> 00:00:02,000\nfirst line\n\n\
             2\n00:00:00,001 --> 00:00:02,000\nsame start, higher id\n\n\
             3\n01:02:03,456 --> 01:02:05,000\nlater\n\n"
        );
        assert!(!out.starts_with('\u{FEFF}'));
        assert!(!out.contains('\r'));
        // a cue never ends before it starts, even when rounding would make it empty
        let tiny = write(&[shown(1, 10_100, 10_200, "x")]);
        assert!(tiny.contains("00:00:00,010 --> 00:00:00,011"));
        assert_eq!(write(&[]), "");
        assert_eq!(srt_time(Micros(-5)), "00:00:00,000");
        assert_eq!(srt_time(Micros(360_000_000_000_000)), "100000:00:00,000");
    }

    // covers: REQ-088
    #[test]
    fn written_files_read_back_identically() {
        let lines = [
            shown(1, 0, 1_234_000, "hello"),
            shown(2, 61_000_000, 62_500_000, "àèì 漢字 {braces} <b>"),
        ];
        let cues = parse(&write(&lines));
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start, Micros(0));
        assert_eq!(cues[0].end, Micros(1_234_000));
        assert_eq!(cues[1].start, Micros(61_000_000));
        assert_eq!(cues[0].text, "hello");
    }

    // covers: REQ-087, REQ-057
    #[test]
    fn tags_are_removed_and_lines_joined() {
        assert_eq!(strip_tags("{\\an8}<i>Hello</i>\\Nworld"), "Hello world");
        assert_eq!(strip_tags("<font color=\"red\">a</font> b\\hc"), "a b c");
        assert_eq!(strip_tags("1 < 2 and 3 > 2"), "1 < 2 and 3 > 2");
        assert_eq!(strip_tags("unclosed {tag"), "unclosed {tag");
        assert_eq!(strip_tags("a\u{0}b\tc\u{7}"), "a b c");
        assert_eq!(strip_tags("x\\"), "x\\");
    }

    // covers: REQ-087, REQ-057
    #[test]
    fn hostile_and_damaged_tracks_never_break_the_reader() {
        let doc = "\u{FEFF}1\r\n00:00:01,000 --> 00:00:02,500\r\n<i>Hi</i>\r\nthere\r\n\r\n\
                   garbage line\n\n\
                   3\n00:00:03.5 --> 00:00:04.000 X1:10 X2:20\n{\\an8}Top\n\n\
                   4\n00:00:05,000 --> 00:00:04,000\nbackwards\n\n\
                   5\n00:61:00,000 --> 00:62:00,000\nbad minutes\n\n\
                   6\n99999999999:00:00,000 --> 99999999999:00:01,000\noverflow\n\n\
                   7\n00:00:06,000 --> 00:00:07,000\n\n\
                   00:00:08,000 --> 00:00:09,000\nno number\n\
                   00:10,000 --> 00:11,000\nshort form";
        let cues = parse(doc);
        let texts: Vec<&str> = cues.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["Hi there", "Top", "no number", "short form"]);
        assert_eq!(cues[0].start, Micros(1_000_000));
        assert_eq!(cues[1].start, Micros(3_500_000));
        assert_eq!(cues[3].end, Micros(11_000_000));
        for bad in [
            "",
            "-->",
            "1\n-->\n",
            "a --> b",
            "00:00:01,0000 --> 00:00:02,000\nx",
            "00:00:01, --> 00:00:02,000\nx",
            "00:00:01,-1 --> 00:00:02,000\nx",
            "1:2:3:4,000 --> 00:00:02,000\nx",
        ] {
            assert!(parse(bad).is_empty(), "{bad:?}");
        }
        // caps: number of cues and text length
        let mut many = String::new();
        for i in 0..(MAX_CUES + 50) {
            many.push_str(&format!("{}\n00:00:01,000 --> 00:00:02,000\nx\n\n", i + 1));
        }
        assert_eq!(parse(&many).len(), MAX_CUES);
        let long = format!(
            "1\n00:00:01,000 --> 00:00:02,000\n{}\n",
            "word ".repeat(100_000)
        );
        let c = parse(&long);
        assert!(c[0].text.chars().count() <= MAX_TEXT);
    }
}
