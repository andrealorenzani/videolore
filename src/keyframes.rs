// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Keyframes of Scale and Move (1.5.0, REQ-129): pure maths, no I/O.
//!
//! A path is a list of keys sorted by time with unique times. Before the first key and after
//! the last the value holds; between two neighbours it changes in a straight line in time
//! (no easing). At most [`MAX_KEYS`] keys per path.

use crate::time::Micros;
use serde::{Deserialize, Serialize};

/// Most keys one path can have.
pub const MAX_KEYS: usize = 200;

/// A value at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key<T> {
    pub t: Micros,
    pub v: T,
}

/// Why a key edit was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone, Copy)]
pub enum KeyError {
    #[error("there is already a keyframe at this time")]
    SameTime,
    #[error("a path holds at most 200 keyframes")]
    TooMany,
    #[error("the last keyframe cannot be removed")]
    LastKey,
    #[error("no keyframe there")]
    NoSuchKey,
}

/// A value that can be blended along a straight line.
pub trait Lerp: Copy {
    /// The value `num/den` of the way from `a` to `b` (`0 <= num <= den`, `den > 0`).
    fn lerp(a: Self, b: Self, num: i64, den: i64) -> Self;
}

/// Blends two integers exactly (rounding to nearest).
pub fn lerp_i64(a: i64, b: i64, num: i64, den: i64) -> i64 {
    let den = den.max(1);
    let d = i128::from(b - a) * i128::from(num);
    let half = i128::from(den) / 2;
    let q = if d >= 0 {
        (d + half) / i128::from(den)
    } else {
        (d - half) / i128::from(den)
    };
    a + q as i64
}

/// The value of `keys` at `t`; `None` when there are no keys.
pub fn at<T: Lerp>(keys: &[Key<T>], t: Micros) -> Option<T> {
    let first = keys.first()?;
    if t <= first.t {
        return Some(first.v);
    }
    for w in keys.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t < b.t {
            return Some(T::lerp(a.v, b.v, (t - a.t).0, (b.t - a.t).0));
        }
    }
    keys.last().map(|k| k.v)
}

/// Adds a key; refuses a second key at the same time and a 201st key.
pub fn add<T>(keys: &mut Vec<Key<T>>, t: Micros, v: T) -> Result<(), KeyError> {
    if keys.iter().any(|k| k.t == t) {
        return Err(KeyError::SameTime);
    }
    if keys.len() >= MAX_KEYS {
        return Err(KeyError::TooMany);
    }
    let i = keys.partition_point(|k| k.t < t);
    keys.insert(i, Key { t, v });
    Ok(())
}

/// Sets the value of the key at `t`, creating it when there is none.
pub fn set<T: Copy>(keys: &mut Vec<Key<T>>, t: Micros, v: T) -> Result<(), KeyError> {
    match keys.iter_mut().find(|k| k.t == t) {
        Some(k) => {
            k.v = v;
            Ok(())
        }
        None => add(keys, t, v),
    }
}

/// Removes the key at `t`; the last remaining key stays.
pub fn remove<T>(keys: &mut Vec<Key<T>>, t: Micros) -> Result<(), KeyError> {
    let i = keys
        .iter()
        .position(|k| k.t == t)
        .ok_or(KeyError::NoSuchKey)?;
    if keys.len() == 1 {
        return Err(KeyError::LastKey);
    }
    keys.remove(i);
    Ok(())
}

/// The key nearest to `t` within `tolerance`, if any.
pub fn nearest<T>(keys: &[Key<T>], t: Micros, tolerance: Micros) -> Option<Micros> {
    keys.iter()
        .map(|k| ((k.t - t).0.abs(), k.t))
        .filter(|(d, _)| *d <= tolerance.0)
        .min_by_key(|(d, _)| *d)
        .map(|(_, t)| t)
}

/// Makes damaged keys valid: drops keys with a time outside `[0, limit]`, sorts, keeps the
/// first of equal times and at most [`MAX_KEYS`]. Returns how many keys were dropped.
pub fn sanitise<T>(keys: &mut Vec<Key<T>>, limit: Micros) -> usize {
    let before = keys.len();
    keys.retain(|k| k.t >= Micros::ZERO && k.t <= limit);
    keys.sort_by_key(|k| k.t); // stable: the first of equal times stays first
    keys.dedup_by_key(|k| k.t);
    keys.truncate(MAX_KEYS);
    before - keys.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct N(i64);
    impl Lerp for N {
        fn lerp(a: N, b: N, num: i64, den: i64) -> N {
            N(lerp_i64(a.0, b.0, num, den))
        }
    }
    fn k(t: i64, v: i64) -> Key<N> {
        Key {
            t: Micros(t),
            v: N(v),
        }
    }

    // covers: REQ-129
    #[test]
    fn holds_before_first_and_after_last_and_is_linear_between() {
        let keys = vec![k(0, 100), k(4_000_000, 200)];
        assert_eq!(at(&keys, Micros(-5)), Some(N(100)));
        assert_eq!(at(&keys, Micros(2_000_000)), Some(N(150)));
        assert_eq!(at(&keys, Micros(4_000_000)), Some(N(200)));
        assert_eq!(at(&keys, Micros(9_000_000)), Some(N(200)));
        assert_eq!(at::<N>(&[], Micros(1)), None);
    }

    // covers: REQ-129
    #[test]
    fn one_key_is_constant() {
        let keys = vec![k(1_000_000, 77)];
        for t in [-1, 0, 1_000_000, 50_000_000] {
            assert_eq!(at(&keys, Micros(t)), Some(N(77)));
        }
    }

    // covers: REQ-129
    #[test]
    fn adding_a_key_on_the_path_changes_nothing() {
        let mut keys = vec![k(0, 100), k(4_000_000, 200)];
        let v = at(&keys, Micros(1_000_000)).unwrap();
        add(&mut keys, Micros(1_000_000), v).unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(at(&keys, Micros(500_000)), Some(N(113)));
        assert_eq!(at(&keys, Micros(2_000_000)), Some(N(150)));
    }

    // covers: REQ-129
    #[test]
    fn same_time_limit_and_last_key_rules() {
        let mut keys = vec![k(0, 1)];
        assert_eq!(add(&mut keys, Micros(0), N(2)), Err(KeyError::SameTime));
        assert_eq!(remove(&mut keys, Micros(0)), Err(KeyError::LastKey));
        assert_eq!(remove(&mut keys, Micros(5)), Err(KeyError::NoSuchKey));
        for i in 1..MAX_KEYS as i64 {
            add(&mut keys, Micros(i), N(i)).unwrap();
        }
        assert_eq!(keys.len(), MAX_KEYS);
        assert_eq!(add(&mut keys, Micros(999), N(0)), Err(KeyError::TooMany));
        set(&mut keys, Micros(3), N(42)).unwrap();
        assert_eq!(at(&keys, Micros(3)), Some(N(42)));
        remove(&mut keys, Micros(0)).unwrap();
        assert_eq!(at(&keys, Micros(0)), Some(N(1)));
    }

    #[test]
    fn keys_stay_sorted_and_nearest_finds_within_tolerance() {
        let mut keys = Vec::new();
        for t in [30, 10, 20] {
            add(&mut keys, Micros(t), N(t)).unwrap();
        }
        let times: Vec<i64> = keys.iter().map(|k| k.t.0).collect();
        assert_eq!(times, [10, 20, 30]);
        assert_eq!(nearest(&keys, Micros(22), Micros(5)), Some(Micros(20)));
        assert_eq!(nearest(&keys, Micros(25), Micros(2)), None);
    }

    // covers: REQ-132
    #[test]
    fn sanitise_drops_sorts_dedups_and_truncates() {
        let mut keys = vec![k(30, 3), k(-1, 0), k(10, 1), k(10, 9), k(99, 5)];
        let dropped = sanitise(&mut keys, Micros(50));
        assert_eq!(dropped, 3);
        assert_eq!(keys, vec![k(10, 1), k(30, 3)]);
        let mut many: Vec<Key<N>> = (0..300).map(|i| k(i, i)).collect();
        sanitise(&mut many, Micros(1000));
        assert_eq!(many.len(), MAX_KEYS);
    }

    #[test]
    fn lerp_rounds_both_ways() {
        assert_eq!(lerp_i64(0, 10, 1, 3), 3);
        assert_eq!(lerp_i64(10, 0, 1, 3), 7);
        assert_eq!(lerp_i64(5, 5, 7, 9), 5);
    }
}
