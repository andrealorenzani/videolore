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
    /// The clip's Volume: a row of the sound group (2.2.0, REQ-320), see [`crate::order_chain`].
    Volume,
    /// The clip's Clean audio: a row of the sound group (2.2.0).
    Clean,
    /// The clip's Crop: a row of the picture group (2.2.0, REQ-320).
    Crop,
    /// Change aspect (the clip's frame: the fit and the background), a row of the picture group.
    Frame,
    /// The clip's Scale, a row of the picture group.
    Scale,
    /// The clip's Move, a row of the picture group.
    Move,
    /// The clip's Fade IN/OUT (every fade the clip has), a row of the picture group.
    Fade,
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
            Row::Volume => "volume".into(),
            Row::Clean => "clean".into(),
            Row::Crop => "crop".into(),
            Row::Frame => "frame".into(),
            Row::Scale => "scale".into(),
            Row::Move => "move".into(),
            Row::Fade => "fade".into(),
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
            "volume" => Row::Volume,
            "clean" => Row::Clean,
            "crop" => Row::Crop,
            "frame" => Row::Frame,
            "scale" => Row::Scale,
            "move" => Row::Move,
            "fade" => Row::Fade,
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
    /// The clip has a Crop (2.2.0).
    pub crop: bool,
    /// The clip has a Change aspect, its own or the project's.
    pub frame: bool,
    pub scale: bool,
    pub moved: bool,
    /// The clip has a fade (at the start, at the end or with a neighbour).
    pub fade: bool,
}

impl Present {
    /// Whether the clip has no movable row of the 2.1.0 kind (Apply LUT, texts, pictures, lines).
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
            Row::Crop => self.crop,
            Row::Frame => self.frame,
            Row::Scale => self.scale,
            Row::Move => self.moved,
            Row::Fade => self.fade,
            // the sound rows are ordered apart from the picture rows
            Row::Volume | Row::Clean | Row::Unknown => false,
        }
    }
}

impl Row {
    /// True for the rows of the sound group (Volume, Clean audio).
    pub fn is_sound(self) -> bool {
        matches!(self, Row::Volume | Row::Clean)
    }

    /// True for the picture rows that act on the picture itself (Crop, Change aspect, Scale,
    /// Move, Fade): they have a place in the default sequence, see [`FIXED`].
    pub fn is_fixed(self) -> bool {
        FIXED.contains(&self)
    }

    /// True for a row drawn on the frame (a text, a picture item, the subtitle lines).
    pub fn is_drawn(self) -> bool {
        matches!(self, Row::Text(_) | Row::Picture(_) | Row::Subtitles)
    }
}

/// The picture rows that act on the picture, top first, in the order the picture has been built
/// since 1.5.0: the fade is applied last, then the frame (the background under the picture), the
/// picture is moved, scaled, and cropped first (Apply LUT sits between Scale and Crop, see
/// [`legacy`]).
pub const FIXED: [Row; 5] = [Row::Fade, Row::Frame, Row::Move, Row::Scale, Row::Crop];

/// The place of a fixed row in [`FIXED`] (0 = the top).
fn rank(r: Row) -> Option<usize> {
    FIXED.iter().position(|x| *x == r)
}

/// The default order of the picture group, top first: the flat texts (the later item on top),
/// then the subtitle lines, then the picture items (the later on top) as the order of 2.0.4 and
/// before, then the picture rows in [`FIXED`] order with the Apply LUT between Scale and Crop (it
/// is in the clip's own chain, under the frame, Scale and Move and over the Crop). A clip with
/// this order is drawn exactly as in the older versions.
pub fn legacy(p: &Present) -> Vec<Row> {
    let mut v: Vec<Row> = p.texts.iter().rev().map(|i| Row::Text(*i)).collect();
    if p.subtitles {
        v.push(Row::Subtitles);
    }
    v.extend(p.pictures.iter().rev().map(|i| Row::Picture(*i)));
    for r in FIXED {
        if r == Row::Crop && p.lut {
            v.push(Row::Lut);
        }
        if p.has(r) {
            v.push(r);
        }
    }
    if p.lut && !v.contains(&Row::Lut) {
        v.push(Row::Lut);
    }
    v
}

/// The order of a clip: `stored` (what the user chose) restricted to the rows the clip has;
/// the Apply LUT, texts, pictures and lines it has that are not stored (new ones) go on top, in
/// the legacy order, and a Crop, Change aspect, Scale, Move or Fade that is not stored takes its
/// place in the default sequence (a new effect never changes how the others are drawn). With
/// nothing stored it is the legacy order.
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
        .filter(|r| !r.is_fixed() && !kept.contains(r))
        .collect();
    out.extend(kept);
    for r in FIXED {
        if !p.has(r) || out.contains(&r) {
            continue;
        }
        let k = rank(r);
        // above the first fixed row that the default sequence puts lower; else at the bottom
        match out.iter().position(|x| rank(*x) > k) {
            Some(i) => out.insert(i, r),
            None => out.push(r),
        }
    }
    out
}

/// Whether `order` is the form of the 2.1.0 overlay stage: the rows that act on the picture
/// are in the default sequence at the bottom and everything above them is a movable row. Such a
/// clip is drawn by the 2.1.0 overlay stage; any other order that is not the default one is
/// built step by step (`render/steps.rs`, ADR-224).
pub fn overlay_form(order: &[Row]) -> bool {
    let first = order.iter().position(|r| r.is_fixed());
    let Some(first) = first else { return true };
    let tail = &order[first..];
    tail.iter().all(|r| r.is_fixed()) && tail.windows(2).all(|w| rank(w[0]) < rank(w[1]))
}

/// Whether a clip with this (not default) order needs the step builder.
pub fn needs_steps(order: &[Row]) -> bool {
    !order.is_empty() && !overlay_form(order)
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
        Row::Volume => "Volume",
        Row::Clean => "Clean audio",
        Row::Crop => "Crop",
        Row::Frame => "Change aspect",
        Row::Scale => "Scale",
        Row::Move => "Move effect",
        Row::Fade => "Fade",
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
            ..Default::default()
        }
    }

    fn all() -> Present {
        Present {
            lut: true,
            texts: vec![1],
            pictures: vec![5],
            subtitles: true,
            crop: true,
            frame: true,
            scale: true,
            moved: true,
            fade: true,
        }
    }

    // covers: REQ-320
    #[test]
    fn the_default_order_is_the_build_of_the_older_versions() {
        use Row::*;
        assert_eq!(
            legacy(&all()),
            vec![
                Text(1),
                Subtitles,
                Picture(5),
                Fade,
                Frame,
                Move,
                Scale,
                Lut,
                Crop
            ]
        );
        // Apply LUT is above Crop and below Scale; with no Crop it is the last row
        let no_crop = Present {
            crop: false,
            ..all()
        };
        assert_eq!(legacy(&no_crop).last(), Some(&Lut));
        let only_crop = Present {
            crop: true,
            ..Default::default()
        };
        assert_eq!(legacy(&only_crop), vec![Crop]);
        // nothing stored: the default, never "changed"
        assert_eq!(resolve(&[], &all()), legacy(&all()));
        assert!(!is_changed(&legacy(&all()), &all()));
        assert!(to_store(legacy(&all()), &all()).is_empty());
    }

    // covers: REQ-320, REQ-321
    #[test]
    fn a_new_picture_row_takes_its_place_in_the_default_sequence_and_a_2_1_0_order_keeps_its_look()
    {
        use Row::*;
        // 2.1.0 stored a LUT above a text: the rows that act on the picture stay below them
        let stored = [Lut, Text(1)];
        let present = Present {
            lut: true,
            texts: vec![1],
            crop: true,
            scale: true,
            fade: true,
            ..Default::default()
        };
        let o = resolve(&stored, &present);
        assert_eq!(o, vec![Lut, Text(1), Fade, Scale, Crop]);
        assert!(overlay_form(&o) && !needs_steps(&o));
        // a Move added later goes between Fade... and Scale, not on top
        let more = Present {
            moved: true,
            ..present.clone()
        };
        assert_eq!(
            resolve(&o, &more),
            vec![Lut, Text(1), Fade, Move, Scale, Crop]
        );
        // a stored Crop above Scale is a built order of its own (not the 2.1.0 overlay form)
        let swapped = vec![Text(1), Crop, Scale];
        assert!(needs_steps(&swapped) && !overlay_form(&swapped));
        assert!(needs_steps(&[Scale, Fade]));
        assert!(!needs_steps(&[]) && !needs_steps(&[Text(1), Lut]));
        // a drawn row below a fixed one is not the overlay form
        assert!(needs_steps(&[Fade, Text(1)]));
        assert!(Text(1).is_drawn() && Subtitles.is_drawn() && !Lut.is_drawn());
        assert!(Crop.is_fixed() && !Lut.is_fixed());
    }

    // covers: REQ-322
    #[test]
    fn the_new_rows_have_names_in_the_file() {
        for r in [Row::Crop, Row::Frame, Row::Scale, Row::Move, Row::Fade] {
            assert_eq!(Row::parse(&r.name()), r);
        }
        assert_eq!(Row::Crop.name(), "crop");
        assert_eq!(noun(Row::Frame), "Change aspect");
        assert_eq!(noun(Row::Fade), "Fade");
        assert_eq!(noun(Row::Scale), "Scale");
        assert_eq!(noun(Row::Crop), "Crop");
        assert_eq!(noun(Row::Move), "Move effect");
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
