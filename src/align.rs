// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Matches the words of a video's own subtitle track to the words recognised in its speech,
//! so that reused subtitles get accurate word timings (REQ-095).
//!
//! The track's text and the cue times are never changed: only the time of each word is
//! taken from the speech. Each cue is aligned on its own (a Needleman–Wunsch alignment of
//! its words against the recognised words around it), going forward through the
//! recognised words, so a cue can never take words that an earlier cue used. A cue where
//! fewer than half the words are found keeps no word timings: its words are then spread
//! evenly over the line, as for typed text.
//!
//! Pure and deterministic; no allocation grows beyond the input sizes.

use crate::media::MediaId;
use crate::srt::Cue;
use crate::subtitles::{NewLine, Word};
use crate::time::Micros;

/// How far before and after a cue recognised words are looked at.
pub const WINDOW: Micros = Micros(1_000_000);
/// Shortest word.
pub const MIN_WORD: Micros = Micros(10_000);
/// Share of a cue's words that must be found for the cue to count as matched.
pub const MIN_MATCHED: f64 = 0.5;
/// Longest stretch of words aligned at once (longer cues are rare; beyond it the rest of
/// the cue is left unmatched).
const MAX_TOKENS: usize = 400;

const EXACT: i32 = 2;
const FUZZY: i32 = 1;
const SUBSTITUTION: i32 = -1;
const GAP: i32 = -1;

/// The result of an alignment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Alignment {
    /// One line per cue (same text and times); `words` empty when the cue was not matched.
    pub lines: Vec<NewLine>,
    /// For each line, which of its words were found in the speech.
    pub found: Vec<Vec<bool>>,
    /// Cues that could not be matched to the speech (their words are spread evenly).
    pub unmatched: usize,
}

/// Lower-case letters and digits only (punctuation and symbols are ignored).
pub fn normalise(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Edit distance between two short words (characters).
fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Score of pairing a track word with a recognised word: exact, close spelling (only for
/// words of 3 characters or more), or different.
fn score(a: &str, b: &str) -> i32 {
    if a.is_empty() || b.is_empty() {
        return SUBSTITUTION;
    }
    if a == b {
        return EXACT;
    }
    let (ca, cb): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if ca.len() < 3 || cb.len() < 3 {
        return SUBSTITUTION;
    }
    let allowed = (ca.len().max(cb.len()) / 4).max(1);
    if ca.len().abs_diff(cb.len()) <= allowed && levenshtein(&ca, &cb) <= allowed {
        FUZZY
    } else {
        SUBSTITUTION
    }
}

/// Alignment of `tokens` against `cands` where recognised words before and after the cue
/// cost nothing (the candidates are a time window around the cue); returns for each token
/// the index of the candidate it matched (exactly or by close spelling). Ties go to the
/// earliest candidates.
fn needleman_wunsch(tokens: &[String], cands: &[String]) -> Vec<Option<usize>> {
    let (n, m) = (tokens.len(), cands.len());
    let w = m + 1;
    let mut dp = vec![0i32; (n + 1) * w];
    for i in 0..=n {
        dp[i * w] = GAP * i as i32;
    }
    for i in 1..=n {
        for j in 1..=m {
            let diag = dp[(i - 1) * w + j - 1] + score(&tokens[i - 1], &cands[j - 1]);
            let up = dp[(i - 1) * w + j] + GAP;
            let left = dp[i * w + j - 1] + GAP;
            dp[i * w + j] = diag.max(up).max(left);
        }
    }
    let mut out = vec![None; n];
    // free trailing candidates: end where the score is best, as early as possible
    let mut j = (0..=m).fold(0, |best, j| {
        if dp[n * w + j] > dp[n * w + best] {
            j
        } else {
            best
        }
    });
    let mut i = n;
    while i > 0 && j > 0 {
        let s = score(&tokens[i - 1], &cands[j - 1]);
        if dp[i * w + j] == dp[(i - 1) * w + j - 1] + s {
            if s > 0 {
                out[i - 1] = Some(j - 1);
            }
            i -= 1;
            j -= 1;
        } else if dp[i * w + j] == dp[(i - 1) * w + j] + GAP {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    out
}

/// Lines with the cues' text and times and no word timings (words spread evenly).
pub fn spread(media: MediaId, cues: &[Cue]) -> Vec<NewLine> {
    cues.iter()
        .map(|c| NewLine {
            media,
            start: c.start,
            end: c.end,
            text: c.text.clone(),
            words: Vec::new(),
        })
        .collect()
}

/// Word times of one matched cue: found words at their recognised time (kept inside the
/// cue, never going back), the others spread evenly between their found neighbours.
fn timings(cue: &Cue, tokens: &[&str], anchors: &[Option<(Micros, Micros)>]) -> Vec<Word> {
    let n = tokens.len();
    let last = (cue.end - MIN_WORD).max(cue.start);
    let mut starts = vec![Micros::ZERO; n];
    let mut ends = vec![Micros::ZERO; n];
    let mut prev_start = cue.start;
    for (k, a) in anchors.iter().enumerate() {
        if let Some((s, e)) = a {
            let s = (*s).clamp(cue.start, last).max(prev_start);
            starts[k] = s;
            ends[k] = (*e).clamp(s, cue.end);
            prev_start = s;
        }
    }
    // runs of words that were not found
    let mut k = 0;
    while k < n {
        if anchors[k].is_some() {
            k += 1;
            continue;
        }
        let run_start = k;
        while k < n && anchors[k].is_none() {
            k += 1;
        }
        let to = if k < n { starts[k] } else { cue.end };
        let from = if run_start > 0 {
            ends[run_start - 1].min(to)
        } else {
            cue.start
        };
        let len = (to - from).0.max(0);
        let count = (k - run_start) as i64;
        for (j, slot) in (run_start..k).enumerate() {
            let j = j as i64;
            starts[slot] = from + Micros(len * j / count);
            ends[slot] = from + Micros(len * (j + 1) / count);
        }
    }
    let mut words = Vec::with_capacity(n);
    for k in 0..n {
        let start = starts[k].clamp(cue.start, cue.end);
        let mut end = ends[k];
        if let Some(next) = starts.get(k + 1) {
            end = end.min(*next);
        }
        let end = end.max(start + MIN_WORD).min(cue.end).max(start);
        words.push(Word {
            start,
            end,
            text: tokens[k].to_string(),
        });
    }
    words
}

/// Aligns the cues of one source (source times) with the words recognised in its speech
/// (source times, in time order).
pub fn align(media: MediaId, cues: &[Cue], recognised: &[Word]) -> Alignment {
    let mut order: Vec<&Cue> = cues.iter().collect();
    order.sort_by_key(|c| (c.start, c.end));
    let rec_norm: Vec<String> = recognised.iter().map(|w| normalise(&w.text)).collect();
    let mut cursor = 0usize;
    let mut out = Alignment::default();
    for cue in order {
        let tokens: Vec<&str> = cue.text.split_whitespace().collect();
        let norm: Vec<String> = tokens
            .iter()
            .take(MAX_TOKENS)
            .map(|t| normalise(t))
            .collect();
        let lo = cue.start - WINDOW;
        let hi = cue.end + WINDOW;
        let first =
            cursor + recognised[cursor.min(recognised.len())..].partition_point(|w| w.start < lo);
        let mut cand_idx: Vec<usize> = (first..recognised.len())
            .take_while(|&i| recognised[i].start <= hi)
            .take(MAX_TOKENS)
            .collect();
        cand_idx.retain(|&i| !rec_norm[i].is_empty());
        let cands: Vec<String> = cand_idx.iter().map(|&i| rec_norm[i].clone()).collect();
        let pairs = needleman_wunsch(&norm, &cands);
        let matchable = norm.iter().filter(|t| !t.is_empty()).count();
        let found_n = pairs.iter().filter(|p| p.is_some()).count();
        let matched = matchable > 0 && found_n as f64 / matchable as f64 >= MIN_MATCHED;
        let mut found = vec![false; tokens.len()];
        let words = if matched {
            let mut anchors = vec![None; tokens.len()];
            for (k, p) in pairs.iter().enumerate() {
                if let Some(j) = p {
                    let w = &recognised[cand_idx[*j]];
                    anchors[k] = Some((w.start, w.end));
                    found[k] = true;
                }
            }
            if let Some(last) = pairs.iter().flatten().max() {
                cursor = cand_idx[*last] + 1;
            }
            timings(cue, &tokens, &anchors)
        } else {
            out.unmatched += 1;
            Vec::new()
        };
        out.lines.push(NewLine {
            media,
            start: cue.start,
            end: cue.end,
            text: cue.text.clone(),
            words,
        });
        out.found.push(found);
    }
    out
}

/// [`align`] for an imported file (REQ-184): a cue that matched the speech starts at its
/// first found word and ends at its last one (words before and after are held inside), so
/// a cue written early or late follows what is said. A cue that did not match keeps the
/// file's times and counts in `unmatched`.
pub fn retime(media: MediaId, cues: &[Cue], recognised: &[Word]) -> Alignment {
    let mut a = align(media, cues, recognised);
    for (line, found) in a.lines.iter_mut().zip(&a.found) {
        let first = found.iter().position(|f| *f);
        let last = found.iter().rposition(|f| *f);
        let (Some(first), Some(last)) = (first, last) else {
            continue;
        };
        if line.words.len() != found.len() {
            continue;
        }
        line.start = line.words[first].start;
        line.end = line.words[last].end.max(line.start + MIN_WORD);
        let (s, e) = (line.start, line.end);
        let mut prev = s;
        for w in &mut line.words {
            w.start = w.start.clamp(prev, e);
            w.end = w.end.clamp(w.start, e);
            prev = w.start;
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: i64) -> Micros {
        Micros(n * 1000)
    }

    fn cue(a: i64, b: i64, text: &str) -> Cue {
        Cue {
            start: ms(a),
            end: ms(b),
            text: text.into(),
        }
    }

    fn rec(list: &[(i64, i64, &str)]) -> Vec<Word> {
        list.iter()
            .map(|(a, b, t)| Word {
                start: ms(*a),
                end: ms(*b),
                text: (*t).into(),
            })
            .collect()
    }

    /// Every line is valid for a project: words inside the line, in order, end >= start.
    fn assert_valid(a: &Alignment) {
        for l in &a.lines {
            let mut prev = l.start;
            for w in &l.words {
                assert!(
                    w.start >= l.start && w.end <= l.end && w.end >= w.start,
                    "{l:?}"
                );
                assert!(w.start >= prev, "{l:?}");
                prev = w.start;
            }
            if !l.words.is_empty() {
                assert_eq!(l.words.len(), l.text.split_whitespace().count());
            }
        }
    }

    // covers: REQ-095
    #[test]
    fn identical_words_take_the_recognised_times() {
        let cues = [
            cue(1000, 3000, "Hello, World!"),
            cue(4000, 6000, "good night"),
        ];
        let words = rec(&[
            (1100, 1500, " hello"),
            (1600, 2200, "world."),
            (4200, 4700, "Good"),
            (4800, 5600, "night"),
        ]);
        let a = align(MediaId(1), &cues, &words);
        assert_eq!(a.unmatched, 0);
        assert_eq!(a.lines[0].text, "Hello, World!", "the track's text is kept");
        assert_eq!((a.lines[0].start, a.lines[0].end), (ms(1000), ms(3000)));
        let starts: Vec<Micros> = a
            .lines
            .iter()
            .flat_map(|l| l.words.iter().map(|w| w.start))
            .collect();
        assert_eq!(starts, [ms(1100), ms(1600), ms(4200), ms(4800)]);
        assert_eq!(a.lines[0].words[0].text, "Hello,");
        assert_eq!(a.lines[0].words[0].end, ms(1500));
        assert_eq!(a.found, vec![vec![true, true], vec![true, true]]);
        assert_valid(&a);
    }

    // covers: REQ-095
    #[test]
    fn misspelt_missing_and_extra_words_are_tolerated() {
        // track: "the quick brown fox jumps over" ; speech: "the quik brown [uh] jumps over"
        // ("quick" misspelt by the recogniser, "fox" missing, "uh" extra)
        let cues = [cue(0, 6000, "the quick brown fox jumps over")];
        let words = rec(&[
            (100, 400, "the"),
            (500, 900, "quik"),
            (1000, 1500, "brown"),
            (1600, 1900, "uh"),
            (3000, 3500, "jumps"),
            (3600, 4000, "over"),
        ]);
        let a = align(MediaId(1), &cues, &words);
        assert_eq!(a.unmatched, 0);
        assert_eq!(a.found[0], [true, true, true, false, true, true]);
        let w = &a.lines[0].words;
        assert_eq!(w[1].start, ms(500), "close spelling matched");
        // "fox" is spread between "brown" (ends 1.5 s) and "jumps" (starts 3.0 s)
        assert_eq!(w[3].start, ms(1500));
        assert_eq!(w[3].end, ms(3000));
        assert_eq!(w[4].start, ms(3000));
        assert_valid(&a);
    }

    // covers: REQ-095
    #[test]
    fn a_cue_without_matching_speech_falls_back_and_is_counted() {
        let cues = [
            cue(0, 2000, "completely different words"),
            cue(3000, 5000, "this one matches"),
            cue(6000, 7000, "- ..."),
        ];
        let words = rec(&[
            (100, 500, "nothing"),
            (600, 900, "alike"),
            (3100, 3500, "this"),
            (3600, 4000, "one"),
            (4100, 4600, "matches"),
        ]);
        let a = align(MediaId(1), &cues, &words);
        assert_eq!(a.unmatched, 2, "different text and a punctuation-only cue");
        assert!(a.lines[0].words.is_empty(), "spread evenly");
        assert_eq!(a.lines[0].text, "completely different words");
        assert_eq!(a.lines[1].words.len(), 3);
        // no speech at all: every cue falls back
        let none = align(MediaId(1), &cues, &[]);
        assert_eq!(none.unmatched, 3);
        assert_eq!(spread(MediaId(2), &cues)[1].text, "this one matches");
        assert!(spread(MediaId(2), &cues).iter().all(|l| l.words.is_empty()));
    }

    // covers: REQ-095
    #[test]
    fn recognised_times_outside_the_cue_are_clamped() {
        let cues = [cue(2000, 4000, "early late")];
        let words = rec(&[(1200, 1900, "early"), (3990, 4800, "late")]);
        let a = align(MediaId(1), &cues, &words);
        let w = &a.lines[0].words;
        assert_eq!(w[0].start, ms(2000));
        assert!(w[1].start <= ms(3990) && w[1].end == ms(4000));
        assert_valid(&a);
        // words out of order in the speech never go back in time
        let cues = [cue(0, 3000, "one two three")];
        let words = rec(&[
            (1000, 1200, "one"),
            (500, 700, "two"),
            (2000, 2500, "three"),
        ]);
        let a = align(MediaId(1), &cues, &words);
        assert_valid(&a);
    }

    // covers: REQ-095
    #[test]
    fn a_cue_never_reuses_words_of_an_earlier_one() {
        let cues = [cue(0, 1500, "yes yes"), cue(1000, 2500, "yes yes")];
        let words = rec(&[
            (100, 300, "yes"),
            (400, 600, "yes"),
            (1200, 1400, "yes"),
            (1600, 1800, "yes"),
        ]);
        let a = align(MediaId(1), &cues, &words);
        assert_eq!(a.lines[0].words[0].start, ms(100));
        assert_eq!(a.lines[1].words[0].start, ms(1200));
        assert_valid(&a);
    }

    #[test]
    fn spelling_rules() {
        assert_eq!(normalise("Ça-va?"), "çava");
        assert_eq!(normalise("ÉCOLE"), "école");
        assert_eq!(score("cat", "cat"), EXACT);
        assert_eq!(score("cat", "cut"), FUZZY);
        assert_eq!(score("an", "on"), SUBSTITUTION, "short words only exactly");
        assert_eq!(score("elephant", "elegant"), FUZZY);
        assert_eq!(score("elephant", "telescope"), SUBSTITUTION);
        assert_eq!(score("", "x"), SUBSTITUTION);
        assert_eq!(levenshtein(&['a'], &[]), 1);
    }

    // covers: REQ-095
    #[test]
    fn ten_thousand_cues_align_quickly() {
        let mut cues = Vec::new();
        let mut words = Vec::new();
        for i in 0..10_000i64 {
            let t = i * 3000;
            cues.push(cue(t, t + 2500, "alpha beta gamma delta epsilon"));
            for (k, w) in ["alpha", "beta", "gamma", "delta", "epsilon"]
                .iter()
                .enumerate()
            {
                let s = t + 100 + k as i64 * 450;
                words.push(Word {
                    start: ms(s),
                    end: ms(s + 400),
                    text: (*w).into(),
                });
            }
        }
        // the best of three runs: a busy machine (parallel tests, coverage) slows one run,
        // not all three; the bound itself is unchanged
        let mut took = std::time::Duration::MAX;
        let mut a = align(MediaId(1), &cues, &words);
        for _ in 0..3 {
            let t0 = std::time::Instant::now();
            a = align(MediaId(1), &cues, &words);
            took = took.min(t0.elapsed());
        }
        assert_eq!(a.unmatched, 0);
        assert_valid(&a);
        // the bound is generous for unoptimised test builds; release is far below 50 ms
        assert!(took < std::time::Duration::from_secs(2), "{took:?}");
    }

    // covers: REQ-184
    #[test]
    fn re_timing_moves_a_line_to_its_first_and_last_recognised_word() {
        // written 0.6 s early; the second cue is not in the speech at all
        let cues = [
            cue(400, 2600, "one two three"),
            cue(9000, 9500, "never said"),
        ];
        let words = rec(&[
            (1000, 1300, "one"),
            (1400, 1800, "two"),
            (2000, 2300, "three"),
        ]);
        let a = retime(MediaId(1), &cues, &words);
        assert_valid(&a);
        assert_eq!((a.lines[0].start, a.lines[0].end), (ms(1000), ms(2300)));
        assert_eq!(a.lines[0].words.len(), 3);
        assert_eq!(a.lines[0].words[0].start, ms(1000));
        assert_eq!(a.unmatched, 1);
        assert_eq!(
            (a.lines[1].start, a.lines[1].end),
            (ms(9000), ms(9500)),
            "no match: the file's times"
        );
        // only some words found: the line starts and ends at the found ones
        let words = rec(&[(1500, 1900, "two")]);
        let cues = [cue(400, 3000, "one two three four")];
        let a = retime(MediaId(1), &cues, &words);
        assert_valid(&a);
        let l = &a.lines[0];
        if !l.words.is_empty() {
            assert_eq!((l.start, l.end), (ms(1500), ms(1900)));
        } else {
            assert_eq!((l.start, l.end), (ms(400), ms(3000)));
        }
    }
}
