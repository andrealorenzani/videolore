// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Speech regions for the timing of reused subtitles (2.2.0, REQ-177, ADR-220). Pure: it works
//! on the 16 kHz mono samples the recogniser already has and on the words of one cue.
//!
//! 1. The level of every 10 ms in dB ([`envelope_db`]); the noise floor is the 10th percentile
//!    of the clip's frames ([`noise_floor`], nothing tuned).
//! 2. A frame is speech when it is at least `t_db` above the floor; gaps shorter than `gap` are
//!    bridged and regions shorter than [`MIN_REGION`] dropped ([`regions`]).
//! 3. The words of a cue are placed over the regions of its window by a small dynamic
//!    programme ([`place`]): a region takes up to [`MAX_PER_REGION`] words and costs the
//!    difference between its length and the length predicted for those words (`characters /
//!    rate`, at least 0.1 s a word); a word left out costs `lambda` seconds.
//!
//! The four parameters are chosen from the frozen [`grid`] of 81 settings, by [`select`], on the
//! reference clip only and **before** any second clip is seen; they are never tuned again.

use crate::time::Micros;

/// Length of a frame of the envelope.
pub const FRAME: Micros = Micros(10_000);
/// Regions shorter than this are not speech.
pub const MIN_REGION: Micros = Micros(100_000);
/// The most words one region takes.
pub const MAX_PER_REGION: usize = 6;
/// Shortest predicted length of a word, in seconds.
const MIN_WORD_SECS: f64 = 0.1;
/// A setting below this share of matched words within 0.3 s on the reference clip is not
/// eligible.
pub const MIN_ELIGIBLE: f64 = 0.75;

/// One setting of the four parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// dB above the noise floor for a speech frame.
    pub t_db: f32,
    /// Gaps shorter than this are inside a region.
    pub gap: Micros,
    /// Characters per second of speech.
    pub rate: f64,
    /// Cost of a word left out, in seconds.
    pub lambda: f64,
}

/// The setting the program uses for reused subtitles, chosen by [`select`] on the reference clip
/// and then frozen (never tuned again), or `None` while no setting is eligible: the old timing
/// (the recognised times) is then used. Measured on the reference clip with the Fast model
/// (2026-11-22, `v12::speech_region_grid_on_the_reference_clip`): the words of the 81 settings are
/// placed over the speech regions of each cue, and at most 10 of the 23 matched words (43%) are
/// within 0.3 s of where they are said (the recognised times give 7 of 23); the best score is
/// under [`MIN_ELIGIBLE`], so nothing is chosen and the strict test stays ignored. The reason is
/// in the clip: its speech has no pauses the envelope can see (one region for each cue, four in
/// all), so the words of a region can only be shared by length, and the words of this clip are
/// not said at a pace that follows their length.
pub const FROZEN: Option<Params> = None;

/// The frozen grid, declared before any second clip is seen: T in {9, 12, 15} dB, G in {0.12,
/// 0.20, 0.30} s, R in {8, 10, 12} characters per second, lambda in {0.5, 1, 2} s; 81 settings
/// in the order T, G, R, lambda ascending (the first of equal scores wins).
pub fn grid() -> Vec<Params> {
    let mut v = Vec::with_capacity(81);
    for t_db in [9.0, 12.0, 15.0] {
        for gap in [120_000, 200_000, 300_000] {
            for rate in [8.0, 10.0, 12.0] {
                for lambda in [0.5, 1.0, 2.0] {
                    v.push(Params {
                        t_db,
                        gap: Micros(gap),
                        rate,
                        lambda,
                    });
                }
            }
        }
    }
    v
}

/// The level of each 10 ms frame of `samples` (mono, `rate` Hz) in dB (RMS, full scale 1.0);
/// silence is -120 dB.
pub fn envelope_db(samples: &[f32], rate: u32) -> Vec<f32> {
    let n = (rate as usize / 100).max(1);
    samples
        .chunks(n)
        .map(|c| {
            let ms = c.iter().map(|s| f64::from(*s) * f64::from(*s)).sum::<f64>() / c.len() as f64;
            (10.0 * ms.max(1e-12).log10()) as f32
        })
        .collect()
}

/// The noise floor: the 10th percentile of the frames' levels.
pub fn noise_floor(db: &[f32]) -> f32 {
    if db.is_empty() {
        return -120.0;
    }
    let mut v = db.to_vec();
    v.sort_by(f32::total_cmp);
    v[(v.len() - 1) / 10]
}

/// The speech regions of an envelope: `(start, end)` in clip time.
pub fn regions(db: &[f32], p: &Params) -> Vec<(Micros, Micros)> {
    let floor = noise_floor(db) + p.t_db;
    let gap = (p.gap.0 / FRAME.0).max(0) as usize;
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (i, d) in db.iter().enumerate() {
        if *d >= floor {
            match runs.last_mut() {
                Some(r) if i - r.1 <= gap => r.1 = i + 1,
                _ => runs.push((i, i + 1)),
            }
        }
    }
    runs.into_iter()
        .map(|(a, b)| (Micros(a as i64 * FRAME.0), Micros(b as i64 * FRAME.0)))
        .filter(|(a, b)| *b - *a >= MIN_REGION)
        .collect()
}

/// The length predicted for a word, in seconds.
fn predicted(word: &str, rate: f64) -> f64 {
    (word.chars().count() as f64 / rate).max(MIN_WORD_SECS)
}

/// Places `words` (in order) over `regions` (in time order): the time of each word, `None` for a
/// word left out. Inside a region the words share its length in proportion to their predicted
/// lengths.
pub fn place(
    words: &[&str],
    regions: &[(Micros, Micros)],
    p: &Params,
) -> Vec<Option<(Micros, Micros)>> {
    let (n, m) = (words.len(), regions.len());
    let pred: Vec<f64> = words.iter().map(|w| predicted(w, p.rate)).collect();
    // cost[j][i]: the best cost with the first j regions used for the first i words
    let inf = f64::INFINITY;
    let mut cost = vec![vec![inf; n + 1]; m + 1];
    let mut back = vec![vec![(0usize, 0usize); n + 1]; m + 1];
    cost[0][0] = 0.0;
    for i in 1..=n {
        cost[0][i] = cost[0][i - 1] + p.lambda;
        back[0][i] = (0, i - 1);
    }
    for j in 1..=m {
        let len = (regions[j - 1].1 - regions[j - 1].0).as_secs_f64();
        for i in 0..=n {
            // the region takes no word
            if cost[j - 1][i] < cost[j][i] {
                cost[j][i] = cost[j - 1][i];
                back[j][i] = (j - 1, i);
            }
            // the word i is left out
            if i > 0 && cost[j][i - 1] + p.lambda < cost[j][i] {
                cost[j][i] = cost[j][i - 1] + p.lambda;
                back[j][i] = (j, i - 1);
            }
            // the region takes the words i-k..i
            for k in 1..=MAX_PER_REGION.min(i) {
                let sum: f64 = pred[i - k..i].iter().sum();
                let c = cost[j - 1][i - k] + (len - sum).abs();
                if c < cost[j][i] {
                    cost[j][i] = c;
                    back[j][i] = (j - 1, i - k);
                }
            }
        }
    }
    let mut out = vec![None; n];
    let (mut j, mut i) = (m, n);
    while (j, i) != (0, 0) {
        let (pj, pi) = back[j][i];
        if pj + 1 == j && pi < i {
            // region j-1 took the words pi..i
            let (a, b) = regions[j - 1];
            let total: f64 = pred[pi..i].iter().sum();
            let mut at = a;
            for w in pi..i {
                let share = Micros(((b - a).0 as f64 * pred[w] / total) as i64);
                out[w] = Some((at, (at + share).min(b)));
                at = at + share;
            }
        }
        if (pj, pi) == (j, i) {
            break;
        }
        (j, i) = (pj, pi);
    }
    out
}

/// Chooses the setting on the reference clip: `score` gives the share of matched words within
/// 0.3 s for a setting (0 to 1). The highest wins, ties go to the first in the [`grid`] order;
/// a best below [`MIN_ELIGIBLE`] gives no setting.
pub fn select(mut score: impl FnMut(&Params) -> f64) -> Option<Params> {
    let mut best: Option<(f64, Params)> = None;
    for p in grid() {
        let s = score(&p);
        if s >= MIN_ELIGIBLE && best.is_none_or(|(b, _)| s > b) {
            best = Some((s, p));
        }
    }
    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 16 kHz samples: `(start_ms, end_ms)` bursts of a loud tone over a faint noise.
    fn speech(total_ms: usize, bursts: &[(usize, usize)]) -> Vec<f32> {
        let mut v = vec![0.0f32; total_ms * 16];
        for (i, s) in v.iter_mut().enumerate() {
            *s = 0.001 * if i % 2 == 0 { 1.0 } else { -1.0 };
        }
        for (a, b) in bursts {
            for (i, s) in v[a * 16..b * 16].iter_mut().enumerate() {
                *s = 0.3 * ((i as f32) * 0.2).sin();
            }
        }
        v
    }

    fn params() -> Params {
        Params {
            t_db: 12.0,
            gap: Micros(200_000),
            rate: 10.0,
            lambda: 1.0,
        }
    }

    // covers: REQ-177
    #[test]
    fn the_grid_has_81_settings_in_a_fixed_order() {
        let g = grid();
        assert_eq!(g.len(), 81);
        assert_eq!(
            g[0],
            Params {
                t_db: 9.0,
                gap: Micros(120_000),
                rate: 8.0,
                lambda: 0.5
            }
        );
        assert_eq!(g[1].lambda, 1.0);
        assert_eq!(g[3].rate, 10.0);
        assert_eq!(
            g[80],
            Params {
                t_db: 15.0,
                gap: Micros(300_000),
                rate: 12.0,
                lambda: 2.0
            }
        );
    }

    // covers: REQ-177
    #[test]
    fn regions_follow_the_bursts_bridge_short_gaps_and_drop_clicks() {
        // two words 150 ms apart (bridged), a pause of 600 ms, a 50 ms click, a last word
        let s = speech(3000, &[(200, 600), (750, 1100), (1700, 1750), (2000, 2500)]);
        let db = envelope_db(&s, 16_000);
        assert_eq!(db.len(), 300);
        let r = regions(&db, &params());
        assert_eq!(r.len(), 2, "{r:?}");
        let near = |a: Micros, ms: i64| (a.0 - ms * 1000).abs() <= 30_000;
        assert!(near(r[0].0, 200) && near(r[0].1, 1100), "{r:?}");
        assert!(near(r[1].0, 2000) && near(r[1].1, 2500), "{r:?}");
        // silence has no region and an empty clip no floor
        assert!(regions(&envelope_db(&speech(500, &[]), 16_000), &params()).is_empty());
        assert_eq!(noise_floor(&[]), -120.0);
    }

    // covers: REQ-177
    #[test]
    fn words_are_placed_over_the_regions_by_length_and_a_word_can_be_left_out() {
        let r = [
            (Micros(200_000), Micros(600_000)),
            (Micros(1_000_000), Micros(1_800_000)),
        ];
        // "ab" is 0.2 s, "cdefgh" 0.6 s, "ij" 0.2 s, "klmnopqr" 0.8 s at 10 characters a second
        let t = place(&["abcd", "efgh", "ijklmnop", "qr"], &r, &params());
        assert_eq!(t.len(), 4);
        // the first region (0.4 s) takes two short words, the second (0.8 s) the long one
        let (a, b) = (t[0].unwrap(), t[1].unwrap());
        assert!(
            a.0 == Micros(200_000) && b.1 == Micros(600_000) && a.1 == b.0,
            "{t:?}"
        );
        assert!(t[2].unwrap().0 >= Micros(1_000_000));
        // more words than the regions can take: the extra ones are left out
        let few = place(
            &["a", "b", "c", "d", "e", "f", "g", "h"],
            &r[..1],
            &Params {
                lambda: 0.05,
                ..params()
            },
        );
        assert!(few.iter().any(Option::is_none), "{few:?}");
        assert!(few.iter().flatten().all(|w| w.0 >= r[0].0 && w.1 <= r[0].1));
        // no region: nothing is placed
        assert!(place(&["a"], &[], &params()).iter().all(Option::is_none));
        assert!(place(&[], &r, &params()).is_empty());
    }

    // covers: REQ-177
    #[test]
    fn the_setting_is_the_first_best_and_must_reach_75_percent_on_the_reference_clip() {
        // every setting 0.7: none is eligible
        assert!(select(|_| 0.7).is_none());
        // all equal and eligible: the first of the grid
        assert_eq!(select(|_| 0.8), Some(grid()[0]));
        // the best one wins; a tie goes to the first
        let want = grid()[40];
        let got = select(|p| {
            if *p == want || *p == grid()[60] {
                0.9
            } else {
                0.8
            }
        });
        assert_eq!(got, Some(want));
    }
}
