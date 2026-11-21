// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! When a text is shown: fades, blink phases and the visibility at a time (REQ-109).

use super::*;

/// Effective fade lengths: each is limited to half of the time to live (REQ-109 d).
pub fn fades(item: &TextItem) -> (Micros, Micros) {
    let half = Micros(item.ttl.0 / 2);
    (
        item.effects.fade_in.map_or(Micros::ZERO, |f| f.min(half)),
        item.effects.fade_out.map_or(Micros::ZERO, |f| f.min(half)),
    )
}

/// The shown phases of an item, relative to its start: one for a text that does not blink.
pub fn phases(item: &TextItem) -> Vec<(Micros, Micros)> {
    let Some(i) = item.effects.blink else {
        return vec![(Micros::ZERO, item.ttl)];
    };
    let i = i.0.max(1);
    let mut v = Vec::new();
    let mut s = 0i64;
    while s < item.ttl.0 && v.len() <= MAX_EVENTS_ITEM {
        v.push((Micros(s), Micros((s + i).min(item.ttl.0))));
        s += 2 * i;
    }
    v
}

/// How many ASS events an item needs (computed without building them).
pub fn event_count(item: &TextItem) -> usize {
    match item.effects.blink {
        None => 1,
        Some(i) => {
            let i = i.0.max(1);
            usize::try_from(
                item.ttl.0.max(0) / (2 * i) + i64::from(item.ttl.0.max(0) % (2 * i) != 0),
            )
            .unwrap_or(usize::MAX)
        }
    }
}

/// The fades alone: 0 at the start rising to 1 over the fade in, falling to 0 over the
/// fade out at the end of the time to live.
pub(super) fn envelope(item: &TextItem, t: Micros) -> f64 {
    let (fin, fout) = fades(item);
    let mut a = 1.0;
    if fin > Micros::ZERO && t < fin {
        a = (t.0 as f64 / fin.0 as f64).max(0.0);
    }
    if fout > Micros::ZERO && t > item.ttl - fout {
        a = a.min(((item.ttl - t).0 as f64 / fout.0 as f64).max(0.0));
    }
    a
}

/// Visibility (0..=1) of the text at `t` after its start (the formula the ASS events
/// follow; used by tests and by the preview boxes).
pub fn alpha_at(item: &TextItem, t: Micros) -> f64 {
    if t < Micros::ZERO || t >= item.ttl {
        return 0.0;
    }
    if let Some(i) = item.effects.blink
        && (t.0 / i.0.max(1)) % 2 == 1
    {
        return 0.0;
    }
    envelope(item, t)
}
