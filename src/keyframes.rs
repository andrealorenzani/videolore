// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Keyframes of Scale and Move (1.5.0, REQ-129): pure maths, no I/O.
//!
//! A path is a list of keys sorted by time with unique times. Before the first key and after
//! the last the value holds; between two neighbours it changes in a straight line in time
//! or with an easing ([`Ease`]). At most [`MAX_KEYS`] keys per path.

use crate::time::Micros;
use serde::{Deserialize, Serialize};

/// Most keys one path can have.
pub const MAX_KEYS: usize = 200;

/// How a value moves from a key to the next one (REQ-271, REQ-282). One rule for the
/// camera's points of view and the keys of Scale, Move and Volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Ease {
    /// At a steady pace (the default).
    #[default]
    Straight,
    /// Slowly at both ends, faster in the middle (smoothstep); both ends are exact.
    EaseInOut,
}

pub fn is_straight(e: &Ease) -> bool {
    *e == Ease::Straight
}

impl Ease {
    /// The part of the way done at `frac` (0..=1) of the time.
    pub fn apply(self, frac: f64) -> f64 {
        let f = frac.clamp(0.0, 1.0);
        match self {
            Ease::Straight => f,
            Ease::EaseInOut => f * f * (3.0 - 2.0 * f),
        }
    }
}

/// A value at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key<T> {
    pub t: Micros,
    pub v: T,
    /// How the value moves on to the next key (REQ-282); not written when straight.
    #[serde(default, skip_serializing_if = "is_straight")]
    pub ease: Ease,
}

impl<T> Key<T> {
    /// A key with a straight move to the next one.
    pub fn new(t: Micros, v: T) -> Key<T> {
        Key {
            t,
            v,
            ease: Ease::Straight,
        }
    }
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
            return Some(blend(a.v, b.v, a.ease, (t - a.t).0, (b.t - a.t).0));
        }
    }
    keys.last().map(|k| k.v)
}

/// The value `num/den` of the way from `a` to `b` with the easing of the segment.
fn blend<T: Lerp>(a: T, b: T, ease: Ease, num: i64, den: i64) -> T {
    if ease == Ease::Straight || num <= 0 || num >= den {
        return T::lerp(a, b, num, den);
    }
    let e = ease.apply(num as f64 / den as f64);
    T::lerp(a, b, (e * den as f64).round() as i64, den)
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
    keys.insert(i, Key::new(t, v));
    Ok(())
}

/// Sets the easing of the segment that starts at the key at `t` (REQ-282).
pub fn set_ease<T>(keys: &mut [Key<T>], t: Micros, ease: Ease) -> Result<(), KeyError> {
    let k = keys
        .iter_mut()
        .find(|k| k.t == t)
        .ok_or(KeyError::NoSuchKey)?;
    k.ease = ease;
    Ok(())
}

/// The easing of the key at `t` (straight when there is none).
pub fn ease_at<T>(keys: &[Key<T>], t: Micros) -> Ease {
    keys.iter()
        .find(|k| k.t == t)
        .map(|k| k.ease)
        .unwrap_or_default()
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

/// The keys with their times counted from `origin` (the clip's own time).
pub fn relative<T: Copy>(keys: &[Key<T>], origin: Micros) -> Vec<Key<T>> {
    keys.iter()
        .map(|k| Key {
            t: k.t - origin,
            v: k.v,
            ease: k.ease,
        })
        .collect()
}

/// Keys in clip time put on a clip whose source starts at `origin` and lasts `len`: a key
/// after the end of the clip is left out, the first key always stays (REQ-204).
pub fn place<T: Copy>(rel: &[Key<T>], origin: Micros, len: Micros) -> Vec<Key<T>> {
    let mut out: Vec<Key<T>> = rel
        .iter()
        .filter(|k| k.t >= Micros::ZERO && k.t <= len)
        .map(|k| Key {
            t: origin + k.t,
            v: k.v,
            ease: k.ease,
        })
        .collect();
    if out.is_empty()
        && let Some(first) = rel.first()
    {
        out.push(Key::new(origin, first.v));
    }
    out.truncate(MAX_KEYS);
    out
}

/// Moves the key at `from` to the time `to`, kept inside `[lo, hi]`; another key at that
/// time refuses it (REQ-203).
pub fn retime<T: Copy>(
    keys: &mut Vec<Key<T>>,
    from: Micros,
    to: Micros,
    lo: Micros,
    hi: Micros,
) -> Result<Micros, KeyError> {
    let i = keys
        .iter()
        .position(|k| k.t == from)
        .ok_or(KeyError::NoSuchKey)?;
    let to = to.clamp(lo, hi.max(lo));
    if to != from && keys.iter().any(|k| k.t == to) {
        return Err(KeyError::SameTime);
    }
    let mut k = keys.remove(i);
    k.t = to;
    let at = keys.partition_point(|x| x.t < to);
    keys.insert(at, k);
    Ok(to)
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    struct N(i64);
    impl Lerp for N {
        fn lerp(a: N, b: N, num: i64, den: i64) -> N {
            N(lerp_i64(a.0, b.0, num, den))
        }
    }
    fn k(t: i64, v: i64) -> Key<N> {
        Key::new(Micros(t), N(v))
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

    // covers: REQ-204
    #[test]
    fn keys_are_copied_in_clip_time_and_placed_on_another_clip() {
        let keys = vec![k(2_000_000, 1), k(3_000_000, 2), k(9_000_000, 3)];
        let rel = relative(&keys, Micros(2_000_000));
        assert_eq!(rel, vec![k(0, 1), k(1_000_000, 2), k(7_000_000, 3)]);
        // on a clip that starts at 10 s in its file and lasts 4 s: the last key does not fit
        let placed = place(&rel, Micros(10_000_000), Micros(4_000_000));
        assert_eq!(placed, vec![k(10_000_000, 1), k(11_000_000, 2)]);
        // the first key always stays
        assert_eq!(
            place(&[k(5_000_000, 7)], Micros(1_000_000), Micros(1_000_000)),
            vec![k(1_000_000, 7)]
        );
        assert!(place::<N>(&[], Micros::ZERO, Micros(1)).is_empty());
    }

    // covers: REQ-203
    #[test]
    fn a_key_moves_in_time_inside_its_range_and_never_onto_another() {
        let mut keys = vec![k(0, 1), k(2_000_000, 2), k(4_000_000, 3)];
        let (lo, hi) = (Micros::ZERO, Micros(5_000_000));
        assert_eq!(
            retime(&mut keys, Micros(2_000_000), Micros(3_000_000), lo, hi),
            Ok(Micros(3_000_000))
        );
        assert_eq!(keys[1], k(3_000_000, 2));
        // past the last key: still sorted
        assert_eq!(
            retime(&mut keys, Micros(3_000_000), Micros(4_500_000), lo, hi),
            Ok(Micros(4_500_000))
        );
        assert_eq!(keys.iter().map(|x| x.v.0).collect::<Vec<_>>(), [1, 3, 2]);
        // clamped to the range
        assert_eq!(
            retime(&mut keys, Micros(4_500_000), Micros(99_000_000), lo, hi),
            Ok(hi)
        );
        // onto another key: refused
        assert_eq!(
            retime(&mut keys, Micros(4_000_000), Micros(0), lo, hi),
            Err(KeyError::SameTime)
        );
        assert_eq!(
            retime(&mut keys, Micros(7), Micros(1), lo, hi),
            Err(KeyError::NoSuchKey)
        );
        // to its own time: nothing changes
        assert_eq!(
            retime(&mut keys, Micros(0), Micros(0), lo, hi),
            Ok(Micros(0))
        );
    }

    // covers: REQ-282
    #[test]
    fn an_eased_segment_is_behind_the_straight_one_at_a_quarter_and_exact_at_the_ends() {
        let mut keys = vec![k(0, 0), k(4_000_000, 1000)];
        let value = |keys: &[Key<N>], t: i64| at(keys, Micros(t)).unwrap().0;
        assert_eq!(value(&keys, 1_000_000), 250, "straight");
        keys[0].ease = Ease::EaseInOut;
        assert_eq!(value(&keys, 0), 0);
        assert_eq!(value(&keys, 4_000_000), 1000);
        assert_eq!(value(&keys, 2_000_000), 500, "the middle is the same");
        assert_eq!(
            value(&keys, 1_000_000),
            156,
            "a quarter: smoothstep, behind 250"
        );
        // the easing is of the segment that starts at the key: the next segment is straight
        keys.push(k(8_000_000, 0));
        assert_eq!(value(&keys, 6_000_000), 500);
        // not written when straight, kept when written, and kept by relative and place
        let plain = serde_json::to_string(&k(1, 2)).unwrap();
        assert!(!plain.contains("ease"), "{plain}");
        let back: Key<N> = serde_json::from_str(&serde_json::to_string(&keys[0]).unwrap()).unwrap();
        assert_eq!(back.ease, Ease::EaseInOut);
        let rel = relative(&keys, Micros(0));
        assert_eq!(rel[0].ease, Ease::EaseInOut);
        assert_eq!(
            place(&rel, Micros(0), Micros(9_000_000))[0].ease,
            Ease::EaseInOut
        );
    }
}
