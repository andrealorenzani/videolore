// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The order of the movable effects of a clip (REQ-304, REQ-311): which row is drawn over which.
//! Pure: rows are named by their kind and a number, the lists are top first (the top row is
//! drawn last, on top of the others). Nothing here knows the project; it only orders.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One movable row of the Applied effects list of a clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Row {
    /// Apply LUT.
    Lut,
    /// A flat text item (its id).
    Text(u64),
    /// A picture item (its id).
    Picture(u64),
    /// The subtitle lines of the clip, as one row.
    Subtitles,
    /// A row read from a damaged file: never made by the program, removed when a project is
    /// opened (with a note).
    Unknown,
}

impl Row {
    /// The name written in the project file: `lut`, `subtitles`, `text:<id>`, `picture:<id>`.
    pub fn name(self) -> String {
        match self {
            Row::Lut => "lut".into(),
            Row::Subtitles => "subtitles".into(),
            Row::Text(i) => format!("text:{i}"),
            Row::Picture(i) => format!("picture:{i}"),
            Row::Unknown => "unknown".into(),
        }
    }

    /// The row of a name written by [`Row::name`]; anything else is [`Row::Unknown`].
    pub fn parse(s: &str) -> Row {
        match s {
            "lut" => Row::Lut,
            "subtitles" => Row::Subtitles,
            _ => {
                let id = |rest: &str| rest.parse::<u64>().ok();
                if let Some(i) = s.strip_prefix("text:").and_then(id) {
                    Row::Text(i)
                } else if let Some(i) = s.strip_prefix("picture:").and_then(id) {
                    Row::Picture(i)
                } else {
                    Row::Unknown
                }
            }
        }
    }
}

impl Serialize for Row {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for Row {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Row, D::Error> {
        // a value that is not a text is a damaged row, not an error that refuses the file
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Name(String),
            Other(serde::de::IgnoredAny),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::Name(s) => Row::parse(&s),
            Raw::Other(_) => Row::Unknown,
        })
    }
}

/// The movable rows a clip has now, each kind in the order of the project's items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Present {
    pub lut: bool,
    /// Flat text items, in item order.
    pub texts: Vec<u64>,
    /// Picture items, in item order.
    pub pictures: Vec<u64>,
    pub subtitles: bool,
}

impl Present {
    /// Whether the clip has no movable row.
    pub fn is_empty(&self) -> bool {
        !self.lut && !self.subtitles && self.texts.is_empty() && self.pictures.is_empty()
    }

    /// Whether `r` is one of the rows.
    pub fn has(&self, r: Row) -> bool {
        match r {
            Row::Lut => self.lut,
            Row::Subtitles => self.subtitles,
            Row::Text(i) => self.texts.contains(&i),
            Row::Picture(i) => self.pictures.contains(&i),
            Row::Unknown => false,
        }
    }
}

/// The order projects saved before 2.1.0 have, top first: the flat texts (the later item on
/// top), then the subtitle lines, then the picture items (the later on top), the Apply LUT at
/// the bottom (it is in the clip's own chain, under everything drawn after the clips).
pub fn legacy(p: &Present) -> Vec<Row> {
    let mut v: Vec<Row> = p.texts.iter().rev().map(|i| Row::Text(*i)).collect();
    if p.subtitles {
        v.push(Row::Subtitles);
    }
    v.extend(p.pictures.iter().rev().map(|i| Row::Picture(*i)));
    if p.lut {
        v.push(Row::Lut);
    }
    v
}

/// The order of a clip: `stored` (what the user chose) restricted to the rows the clip has,
/// and the rows it has that are not stored (new ones) on top, in the legacy order. With nothing
/// stored it is the legacy order.
pub fn resolve(stored: &[Row], p: &Present) -> Vec<Row> {
    if stored.is_empty() {
        return legacy(p);
    }
    let mut kept: Vec<Row> = Vec::new();
    for r in stored {
        if p.has(*r) && !kept.contains(r) {
            kept.push(*r);
        }
    }
    let mut out: Vec<Row> = legacy(p)
        .into_iter()
        .filter(|r| !kept.contains(r))
        .collect();
    out.extend(kept);
    out
}

/// What to store for `order`: nothing when it is the legacy order (such a clip is drawn
/// exactly as in the older versions).
pub fn to_store(order: Vec<Row>, p: &Present) -> Vec<Row> {
    if order == legacy(p) {
        Vec::new()
    } else {
        order
    }
}

/// Whether the order of a clip differs from the legacy one, so that it is drawn by the
/// overlay stage.
pub fn is_changed(stored: &[Row], p: &Present) -> bool {
    !stored.is_empty() && resolve(stored, p) != legacy(p)
}

/// The order after moving `row` one place up (`up`) or down; `None` when it is not in the list
/// or already at that end.
pub fn moved(order: &[Row], row: Row, up: bool) -> Option<Vec<Row>> {
    let i = order.iter().position(|r| *r == row)?;
    let j = if up {
        i.checked_sub(1)?
    } else {
        (i + 1 < order.len()).then_some(i + 1)?
    };
    let mut v = order.to_vec();
    v.swap(i, j);
    Some(v)
}

/// The order after dragging `row` to `to` (an index in the list).
pub fn dragged(order: &[Row], row: Row, to: usize) -> Option<Vec<Row>> {
    let i = order.iter().position(|r| *r == row)?;
    let mut v = order.to_vec();
    let r = v.remove(i);
    v.insert(to.min(v.len()), r);
    (v != order).then_some(v)
}

/// A short name for the undo label ("LUT", "text", "picture", "subtitles").
pub fn noun(r: Row) -> &'static str {
    match r {
        Row::Lut => "LUT",
        Row::Text(_) => "text",
        Row::Picture(_) => "picture",
        Row::Subtitles => "subtitles",
        Row::Unknown => "row",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Present {
        Present {
            lut: true,
            texts: vec![1, 2],
            pictures: vec![5],
            subtitles: true,
        }
    }

    #[test]
    fn legacy_order_is_texts_lines_pictures_lut_from_the_top() {
        // covers: REQ-305
        assert_eq!(
            legacy(&p()),
            vec![
                Row::Text(2),
                Row::Text(1),
                Row::Subtitles,
                Row::Picture(5),
                Row::Lut
            ]
        );
        assert!(legacy(&Present::default()).is_empty());
        assert!(Present::default().is_empty() && !p().is_empty());
    }

    #[test]
    fn nothing_stored_is_the_legacy_order_and_it_is_never_changed() {
        // covers: REQ-311
        assert_eq!(resolve(&[], &p()), legacy(&p()));
        assert!(!is_changed(&[], &p()));
        assert!(!is_changed(&legacy(&p()), &p()));
        assert!(to_store(legacy(&p()), &p()).is_empty());
    }

    #[test]
    fn a_stored_order_wins_new_rows_go_on_top_and_gone_rows_are_dropped() {
        // covers: REQ-304
        let stored = [Row::Lut, Row::Text(1), Row::Text(9)];
        let present = Present {
            lut: true,
            texts: vec![1, 3],
            ..Default::default()
        };
        assert_eq!(
            resolve(&stored, &present),
            vec![Row::Text(3), Row::Lut, Row::Text(1)]
        );
        assert!(is_changed(&stored, &present));
        // duplicates and unknown rows are ignored
        let odd = [Row::Unknown, Row::Lut, Row::Lut];
        assert_eq!(
            resolve(&odd, &present),
            vec![Row::Text(3), Row::Text(1), Row::Lut]
        );
    }

    #[test]
    fn moving_swaps_one_place_and_stops_at_the_ends() {
        // covers: REQ-304
        let o = vec![Row::Text(1), Row::Lut, Row::Subtitles];
        assert_eq!(
            moved(&o, Row::Lut, true),
            Some(vec![Row::Lut, Row::Text(1), Row::Subtitles])
        );
        assert_eq!(
            moved(&o, Row::Lut, false),
            Some(vec![Row::Text(1), Row::Subtitles, Row::Lut])
        );
        assert_eq!(moved(&o, Row::Text(1), true), None);
        assert_eq!(moved(&o, Row::Subtitles, false), None);
        assert_eq!(moved(&o, Row::Text(7), true), None);
        assert_eq!(
            dragged(&o, Row::Subtitles, 0),
            Some(vec![Row::Subtitles, Row::Text(1), Row::Lut])
        );
        assert_eq!(dragged(&o, Row::Text(1), 0), None);
        assert_eq!(dragged(&o, Row::Text(7), 0), None);
        assert_eq!(noun(Row::Lut), "LUT");
        assert_eq!(noun(Row::Text(1)), "text");
        assert_eq!(noun(Row::Picture(1)), "picture");
        assert_eq!(noun(Row::Subtitles), "subtitles");
        assert_eq!(noun(Row::Unknown), "row");
    }

    #[test]
    fn rows_are_written_as_names_and_damaged_names_are_unknown() {
        // covers: REQ-311
        for r in [Row::Lut, Row::Subtitles, Row::Text(12), Row::Picture(3)] {
            assert_eq!(Row::parse(&r.name()), r);
            let j = serde_json::to_string(&r).unwrap();
            assert_eq!(serde_json::from_str::<Row>(&j).unwrap(), r);
        }
        for bad in ["", "text:", "text:x", "picture:-1", "mystery", "Lut"] {
            assert_eq!(Row::parse(bad), Row::Unknown, "{bad}");
        }
        assert_eq!(Row::Unknown.name(), "unknown");
        assert_eq!(serde_json::from_str::<Row>("42").unwrap(), Row::Unknown);
        assert_eq!(serde_json::from_str::<Row>("[1,2]").unwrap(), Row::Unknown);
        assert_eq!(serde_json::from_str::<Row>("null").unwrap(), Row::Unknown);
    }
}
