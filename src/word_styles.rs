// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The subtitle styles that follow the spoken words (2.1.0, ADR-213, REQ-309, REQ-310):
//! pure plans that turn the words of a line into ASS events (a time range and the text with
//! its tags). Nothing here writes a file; `ass` writes the dialogues. Imports the time type
//! only.
//!
//! libass facts the plans rely on (measured, spike S0c with ffmpeg 9.0.2): `\ko` switches a
//! letter to the primary colour at its start and draws no outline before that; with
//! `\2a&HFF&` the letter is also transparent before it; `\alpha&HFF&` hides a run entirely
//! and keeps its place in the line.

use crate::time::Micros;

/// One word of a line: raw text and its time range on the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timed {
    pub start: Micros,
    pub end: Micros,
    pub text: String,
}

/// One event of a plan: a time range and its text with the tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub start: Micros,
    pub end: Micros,
    pub text: String,
}

/// A colour as an ASS tag value, `&HBBGGRR&`.
pub fn bgr(c: [u8; 3]) -> String {
    format!("&H{:02X}{:02X}{:02X}&", c[2], c[1], c[0])
}

/// Where each word is shown: from its start (the first word: the line's start) to the next
/// word's start (the last: the line's end), at least 10 ms.
fn spans(words: &[Timed], line_start: Micros, line_end: Micros) -> Vec<(Micros, Micros)> {
    (0..words.len())
        .map(|i| {
            let from = if i == 0 {
                line_start
            } else {
                words[i].start.max(line_start)
            };
            let to = words.get(i + 1).map_or(line_end, |n| n.start.max(from));
            (from, to.max(from + Micros(10_000)))
        })
        .collect()
}

/// Typewriter (REQ-310): one event per word. The words before it stay in `not_yet`, the
/// word itself appears letter by letter in `highlighted` (the letters spread evenly over its
/// time; a word under 0.1 s appears at once) and the words after it are hidden but keep
/// their place, so the line does not move. `tag` is the line's own tag block (position,
/// size) or empty; `esc` escapes text for ASS.
pub fn typewriter(
    words: &[Timed],
    line: (Micros, Micros),
    tag: &str,
    colours: ([u8; 3], [u8; 3]),
    shift: Micros,
    esc: &dyn Fn(&str) -> String,
) -> Vec<Event> {
    let (highlighted, not_yet) = colours;
    let at = spans(words, line.0, line.1);
    let mut out = Vec::with_capacity(words.len());
    for (i, (from, to)) in at.iter().enumerate() {
        let mut t = String::from(tag);
        for (j, w) in words.iter().enumerate() {
            let space = if j + 1 < words.len() { " " } else { "" };
            if j < i {
                t.push_str(&format!("{{\\1c{}}}{}{space}", bgr(not_yet), esc(&w.text)));
            } else if j == i {
                t.push_str(&format!("{{\\1c{}}}", bgr(highlighted)));
                let letters: Vec<char> = w.text.chars().collect();
                let cs = ((*to - *from).0.max(0) / 10_000) as usize;
                if cs < 10 || letters.is_empty() {
                    t.push_str(&esc(&w.text));
                } else {
                    t.push_str("{\\2a&HFF&}");
                    let n = letters.len();
                    // a render that starts inside the word has the letters that are already
                    // typed at once (the event starts at `shift`, not at the word's start)
                    let elapsed = ((shift - *from).0.max(0) / 10_000) as usize;
                    let start = |k: usize| (cs * k / n).saturating_sub(elapsed);
                    for (k, c) in letters.iter().enumerate() {
                        let d = if k + 1 < n {
                            start(k + 1) - start(k)
                        } else {
                            cs.saturating_sub(elapsed).saturating_sub(start(k))
                        };
                        t.push_str(&format!("{{\\ko{d}}}{}", esc(&c.to_string())));
                    }
                }
                t.push_str(space);
            } else {
                t.push_str(&format!("{{\\alpha&HFF&}}{}{space}", esc(&w.text)));
            }
        }
        out.push(Event {
            start: *from,
            end: *to,
            text: t,
        });
    }
    out
}

/// Enlarge word (REQ-309): two events per word. The base event shows the whole line with
/// the word hidden; the second shows only the word, `fs` pixels in size, with the other
/// words hidden at their normal size. The line is centred, so the word grows around its own
/// centre and no other word moves.
///
/// With `anchors` (REQ-333) the enlarged event is the word alone, drawn at its own measured
/// centre (`\an5\pos`), so it grows around the place it has at normal size and no other word
/// moves; `bord` is the outline tag the big word keeps (empty or `\bord..`). Without them
/// (the text cannot be measured) the word grows inside the line.
pub fn enlarge(
    words: &[Timed],
    line: (Micros, Micros),
    tag: &str,
    fs: u32,
    esc: &dyn Fn(&str) -> String,
    anchors: Option<(&[(f64, f64)], &str)>,
) -> Vec<Event> {
    let at = spans(words, line.0, line.1);
    let mut out = Vec::with_capacity(words.len() * 2);
    for (i, (from, to)) in at.iter().enumerate() {
        let mut base = String::from(tag);
        let mut big = String::from(tag);
        if let Some((centres, bord)) = anchors.filter(|(c, _)| c.len() == words.len()) {
            for (j, w) in words.iter().enumerate() {
                let space = if j + 1 < words.len() { " " } else { "" };
                let text = esc(&w.text);
                if j == i {
                    base.push_str(&format!("{{\\alpha&HFF&}}{text}{{\\alpha&H00&}}{space}"));
                } else {
                    base.push_str(&format!("{text}{space}"));
                }
            }
            let (x, y) = centres[i];
            let big = format!(
                "{{\\an5\\pos({},{}){bord}\\fs{fs}}}{}",
                x.round() as i64,
                y.round() as i64,
                esc(&words[i].text)
            );
            for text in [base, big] {
                out.push(Event {
                    start: *from,
                    end: *to,
                    text,
                });
            }
            continue;
        }
        for (j, w) in words.iter().enumerate() {
            let space = if j + 1 < words.len() { " " } else { "" };
            let text = esc(&w.text);
            if j == i {
                base.push_str(&format!("{{\\alpha&HFF&}}{text}{{\\alpha&H00&}}{space}"));
                big.push_str(&format!("{{\\fs{fs}}}{text}{{\\r}}{space}"));
            } else {
                base.push_str(&format!("{text}{space}"));
                big.push_str(&format!("{{\\alpha&HFF&}}{text}{{\\alpha&H00&}}{space}"));
            }
        }
        for text in [base, big] {
            out.push(Event {
                start: *from,
                end: *to,
                text,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Vec<Timed> {
        ["one", "typed", "end"]
            .iter()
            .enumerate()
            .map(|(i, t)| Timed {
                start: Micros(i as i64 * 1_000_000),
                end: Micros((i as i64 + 1) * 1_000_000),
                text: (*t).to_string(),
            })
            .collect()
    }

    fn esc(s: &str) -> String {
        s.to_string()
    }

    // covers: REQ-310
    #[test]
    fn typewriter_makes_one_event_per_word_with_letters_spread_over_the_word() {
        let ev = typewriter(
            &words(),
            (Micros::ZERO, Micros(3_000_000)),
            "",
            ([255, 0, 0], [0, 0, 255]),
            Micros::ZERO,
            &esc,
        );
        assert_eq!(ev.len(), 3);
        // the second word: the first word spoken (blue), 5 letters of 0.2 s, the last hidden
        let e = &ev[1];
        assert_eq!((e.start, e.end), (Micros(1_000_000), Micros(2_000_000)));
        assert!(
            e.text
                .starts_with("{\\1c&HFF0000&}one {\\1c&H0000FF&}{\\2a&HFF&}")
        );
        assert_eq!(e.text.matches("\\ko20}").count(), 5, "{}", e.text);
        assert!(e.text.ends_with("{\\alpha&HFF&}end"), "{}", e.text);
        // the first word has no spoken words before it and the last none hidden after it
        assert!(!ev[0].text.contains("&H0000FF&}one"));
        assert!(!ev[2].text.contains("alpha"));
    }

    // covers: REQ-310
    #[test]
    fn a_word_under_a_tenth_of_a_second_appears_at_once() {
        let w = vec![Timed {
            start: Micros::ZERO,
            end: Micros(50_000),
            text: "hi".into(),
        }];
        let ev = typewriter(
            &w,
            (Micros::ZERO, Micros(50_000)),
            "",
            ([1, 2, 3], [4, 5, 6]),
            Micros::ZERO,
            &esc,
        );
        assert!(!ev[0].text.contains("\\ko"));
        assert!(ev[0].text.ends_with("hi"));
    }

    // covers: REQ-309
    #[test]
    fn enlarge_hides_the_word_in_the_base_and_alone_enlarges_it_in_the_second_event() {
        let ev = enlarge(
            &words(),
            (Micros::ZERO, Micros(3_000_000)),
            "{\\an5}",
            52,
            &esc,
            None,
        );
        assert_eq!(ev.len(), 6);
        assert_eq!(
            ev[2].text,
            "{\\an5}one {\\alpha&HFF&}typed{\\alpha&H00&} end"
        );
        assert_eq!(
            ev[3].text,
            "{\\an5}{\\alpha&HFF&}one{\\alpha&H00&} {\\fs52}typed{\\r} {\\alpha&HFF&}end{\\alpha&H00&}"
        );
        assert_eq!(ev[2].start, ev[3].start);
    }

    // covers: REQ-333
    #[test]
    fn enlarge_with_anchors_draws_the_word_alone_at_its_centre_and_leaves_the_base() {
        let centres = [(100.0, 50.4), (200.0, 50.4), (300.4, 50.6)];
        let ev = enlarge(
            &words(),
            (Micros::ZERO, Micros(3_000_000)),
            "{\\an5\\pos(960,900)}",
            52,
            &esc,
            Some((&centres, "\\bord3")),
        );
        assert_eq!(ev.len(), 6);
        // the base event is the same as without anchors; the big word stands alone
        assert_eq!(
            ev[2].text,
            "{\\an5\\pos(960,900)}one {\\alpha&HFF&}typed{\\alpha&H00&} end"
        );
        assert_eq!(ev[3].text, "{\\an5\\pos(200,50)\\bord3\\fs52}typed");
        assert_eq!(ev[5].text, "{\\an5\\pos(300,51)\\bord3\\fs52}end");
        assert_eq!((ev[2].start, ev[2].end), (ev[3].start, ev[3].end));
        // a wrong count of anchors is ignored: the in-line form
        let few = enlarge(
            &words(),
            (Micros::ZERO, Micros(3_000_000)),
            "",
            52,
            &esc,
            Some((&centres[..2], "")),
        );
        assert!(few[3].text.contains("{\\fs52}typed{\\r}"));
    }

    // covers: REQ-310
    #[test]
    fn a_render_that_starts_inside_a_word_has_the_typed_letters_at_once() {
        // "typed" lasts 1 s from 1 s: 0.5 s in, 3 of the 5 letters (at 0, 0.2, 0.4 s) are typed
        let ev = typewriter(
            &words(),
            (Micros::ZERO, Micros(3_000_000)),
            "",
            ([255, 0, 0], [0, 0, 255]),
            Micros(1_500_000),
            &esc,
        );
        let t = &ev[1].text;
        // t, y and p (at 0, 0.2 and 0.4 s) are shown at once, e comes 0.1 s in, d 0.3 s in
        assert!(
            t.contains("{\\ko0}t{\\ko0}y{\\ko10}p{\\ko20}e{\\ko20}d"),
            "{t}"
        );
    }

    #[test]
    fn colours_are_written_blue_green_red() {
        assert_eq!(bgr([1, 2, 3]), "&H030201&");
    }
}
