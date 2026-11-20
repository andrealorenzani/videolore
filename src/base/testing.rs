// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Test helper for wall-clock product gates that must not fail because the machine is busy
//! (QG-09). Only the standard library is used, so the module cycle check holds.

use std::time::Duration;

/// Runs `sample` up to `n` times and returns the shortest time, stopping at the first sample
/// under `limit`. When every sample is over the limit all the times are printed, so a real
/// slowdown (every sample slow) still fails and shows its numbers.
pub fn best_of(n: usize, limit: Duration, mut sample: impl FnMut() -> Duration) -> Duration {
    let mut all = Vec::with_capacity(n);
    for _ in 0..n.max(1) {
        let d = sample();
        all.push(d);
        if d < limit {
            return d;
        }
    }
    let best = all.iter().copied().min().unwrap_or_default();
    eprintln!("best_of: every sample over {limit:?}: {all:?}");
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_sample_under_the_limit_is_enough_and_all_over_fails() {
        let limit = Duration::from_millis(10);
        let mut n = 0;
        let d = best_of(5, limit, || {
            n += 1;
            Duration::from_millis(if n == 3 { 1 } else { 50 })
        });
        assert!(d < limit && n == 3, "stops at the first good sample");
        let mut n = 0;
        let d = best_of(5, limit, || {
            n += 1;
            Duration::from_millis(40 + n)
        });
        assert!(d >= limit && n == 5, "every sample was tried: {d:?}");
    }
}
