// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The round time steps of the timeline: used by the ruler labels and the picture strips.

/// Steps in seconds, smallest first (after the last one: 2, 5, 10, … hours).
pub const STEPS: [u64; 13] = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600];

/// The `n`-th step (seconds): the table, then 2, 5 and 10 times larger each decade of hours.
pub fn step(n: usize) -> u64 {
    if let Some(s) = STEPS.get(n) {
        return *s;
    }
    let k = n - STEPS.len();
    let decade = 10u64.saturating_pow((k / 3) as u32);
    3600u64
        .saturating_mul(decade)
        .saturating_mul([2, 5, 10][k % 3])
}
