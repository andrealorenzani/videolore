// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The groups of the order of a clip's effects (2.2.0, REQ-320 to REQ-322, ADR-223, ADR-229).
//! Pure: it knows the row names of [`crate::effect_order`] and nothing else (no render, app or
//! ui). A clip's stored order is one list; its picture rows are ordered by
//! [`crate::effect_order`] (Apply LUT, texts, pictures, subtitles) and its **sound group**
//! (Volume and Clean audio) here. A row moves only inside its group.
//!
//! The sound list is read top first as the order of the sound build: the top row is applied
//! first ("Volume above Clean audio raises the level before the noise reduction"). The default
//! is the order the sound has had since 1.4.0: Clean audio, then Volume.

use crate::effect_order::Row;

/// The default order of the sound group, top (applied first) to bottom.
pub const DEFAULT_SOUND: [Row; 2] = [Row::Clean, Row::Volume];

/// The two groups a row can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Picture,
    Sound,
}

/// The group of a row; `None` for a damaged row.
pub fn group(r: Row) -> Option<Group> {
    match r {
        Row::Unknown => None,
        r if r.is_sound() => Some(Group::Sound),
        _ => Some(Group::Picture),
    }
}

/// The sound rows of a stored order, in their stored order, each once.
pub fn sound_rows(stored: &[Row]) -> Vec<Row> {
    let mut v: Vec<Row> = Vec::new();
    for r in stored.iter().filter(|r| r.is_sound()) {
        if !v.contains(r) {
            v.push(*r);
        }
    }
    v
}

/// The picture rows of a stored order (everything that is not a sound row).
pub fn picture_rows(stored: &[Row]) -> Vec<Row> {
    stored.iter().copied().filter(|r| !r.is_sound()).collect()
}

/// The order of the sound group of a clip: the stored order of the rows it has, else the
/// default order. A row the clip has that is not stored takes its default place.
pub fn resolve_sound(stored: &[Row], has_volume: bool, has_clean: bool) -> Vec<Row> {
    let has = |r: Row| match r {
        Row::Volume => has_volume,
        Row::Clean => has_clean,
        _ => false,
    };
    let kept: Vec<Row> = sound_rows(stored).into_iter().filter(|r| has(*r)).collect();
    if kept.len() == DEFAULT_SOUND.iter().filter(|r| has(**r)).count() {
        kept
    } else {
        DEFAULT_SOUND.into_iter().filter(|r| has(*r)).collect()
    }
}

/// Whether the clip's sound is built in another order than the default (Volume first).
pub fn sound_swapped(stored: &[Row], has_volume: bool, has_clean: bool) -> bool {
    has_volume
        && has_clean
        && resolve_sound(stored, has_volume, has_clean) != DEFAULT_SOUND.to_vec()
}

/// The stored order with its sound group replaced by `sound`: nothing is written for the
/// default order (a clip with the default order stores nothing, as in 2.1.0).
pub fn with_sound(stored: &[Row], sound: &[Row]) -> Vec<Row> {
    let mut v = picture_rows(stored);
    if sound != DEFAULT_SOUND.as_slice() {
        v.extend_from_slice(sound);
    }
    v
}

/// The stored order with its picture rows replaced by `picture` (the sound rows are kept).
pub fn with_picture(stored: &[Row], picture: Vec<Row>) -> Vec<Row> {
    let mut v = picture;
    v.extend(sound_rows(stored));
    v
}

/// The sound order after moving `row` one place up (`up`) or down inside the group.
pub fn moved_sound(order: &[Row], row: Row, up: bool) -> Option<Vec<Row>> {
    crate::effect_order::moved(order, row, up)
}

/// Expands the order a 2.1.0 project stored (format 12): it has no sound rows, so the sound
/// group has its default order and the picture rows are unchanged: the look of the project
/// does not change (REQ-322).
pub fn expand_2_1_0(stored: &[Row]) -> Vec<Row> {
    picture_rows(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-320
    #[test]
    fn rows_are_in_one_of_two_groups_and_the_default_sound_order_is_clean_then_volume() {
        assert_eq!(group(Row::Volume), Some(Group::Sound));
        assert_eq!(group(Row::Clean), Some(Group::Sound));
        assert_eq!(group(Row::Lut), Some(Group::Picture));
        assert_eq!(group(Row::Text(3)), Some(Group::Picture));
        assert_eq!(group(Row::Unknown), None);
        assert_eq!(
            resolve_sound(&[], true, true),
            vec![Row::Clean, Row::Volume]
        );
        assert!(!sound_swapped(&[], true, true));
        // one row has no order: nothing to swap
        assert_eq!(
            resolve_sound(&[Row::Volume], true, false),
            vec![Row::Volume]
        );
        assert!(!sound_swapped(&[Row::Volume, Row::Clean], true, false));
    }

    // covers: REQ-320, REQ-321
    #[test]
    fn a_row_moves_only_inside_its_group_and_the_default_order_stores_nothing() {
        let swapped = moved_sound(&DEFAULT_SOUND, Row::Volume, true).unwrap();
        assert_eq!(swapped, vec![Row::Volume, Row::Clean]);
        assert!(
            moved_sound(&DEFAULT_SOUND, Row::Clean, true).is_none(),
            "top row"
        );
        assert!(
            moved_sound(&DEFAULT_SOUND, Row::Volume, false).is_none(),
            "bottom row"
        );
        // a picture row is not in the sound list: it cannot move there
        assert!(moved_sound(&DEFAULT_SOUND, Row::Lut, true).is_none());
        let stored = with_sound(&[Row::Lut, Row::Text(1)], &swapped);
        assert_eq!(
            stored,
            vec![Row::Lut, Row::Text(1), Row::Volume, Row::Clean]
        );
        assert!(sound_swapped(&stored, true, true));
        assert_eq!(picture_rows(&stored), vec![Row::Lut, Row::Text(1)]);
        // back to the default: the sound rows are not written
        assert_eq!(
            with_sound(&stored, &DEFAULT_SOUND),
            vec![Row::Lut, Row::Text(1)]
        );
        assert!(with_sound(&[], &DEFAULT_SOUND).is_empty());
        // changing the picture rows keeps the sound rows
        assert_eq!(
            with_picture(&stored, vec![Row::Text(1)]),
            vec![Row::Text(1), Row::Volume, Row::Clean]
        );
    }

    // covers: REQ-322
    #[test]
    fn a_2_1_0_order_has_the_default_sound_order_and_its_picture_rows_unchanged() {
        let stored = vec![Row::Subtitles, Row::Text(2), Row::Lut];
        assert_eq!(expand_2_1_0(&stored), stored);
        assert!(!sound_swapped(&expand_2_1_0(&stored), true, true));
        // a damaged repeated or half-written sound list falls back to the default
        assert_eq!(
            resolve_sound(&[Row::Volume, Row::Volume], true, true),
            vec![Row::Clean, Row::Volume]
        );
        assert_eq!(sound_rows(&[Row::Volume, Row::Volume]), vec![Row::Volume]);
    }
}
