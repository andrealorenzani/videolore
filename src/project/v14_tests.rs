// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Project format 14 (2.3.0): overlaps stored in their clips, no crop, the conversion of format
//! 13 (picture items, crops), the damaged-overlap rules and the refusal by older programs
//! (REQ-029, REQ-057, REQ-325, REQ-330, REQ-332).

use super::*;
use crate::base::pos::Pos;
use crate::effect_order::Row;
use crate::overlap::{Kind, Overlap};
use crate::text::TextItem;
use crate::timeline::Crop;

fn json_of(p: &Project) -> String {
    to_json(p, std::path::Path::new("/tmp/x.vlproj")).unwrap()
}

/// The project, the notes (the missing fake sources are not noted) and the conversion.
fn reopened(json: &str) -> (Project, Vec<String>, Option<Conversion>) {
    let (p, mut notes, _) = from_json_noted(json, std::path::Path::new("/tmp")).unwrap();
    notes.retain(|n| !n.ends_with(".mp4"));
    let c = p.converted;
    (p, notes, c)
}

fn s(v: i64) -> Micros {
    Micros(v * 1_000_000)
}

/// Two clips of 4 s and 6 s.
fn two_clips() -> Project {
    let mut p = Project::default();
    for secs in [4, 6] {
        let id = p.new_media_id();
        let m = crate::timeline::tests::media(id.0, secs, true);
        p.add_media(m.clone());
        p.timeline.append(&m).unwrap();
    }
    p
}

fn overlap(id: u64, from: i64, len: i64) -> Overlap {
    Overlap {
        id,
        path: "/tmp/logo.png".into(),
        kind: Kind::Still,
        from: s(from),
        len: s(len),
        place: Pos { x: 2500, y: 7500 },
        width: 300,
        height: 200,
        reshaped: true,
        src_w: 400,
        src_h: 300,
    }
}

// covers: REQ-325, REQ-029
#[test]
fn an_overlap_is_written_in_format_14_with_its_clip_and_read_back() {
    let mut p = two_clips();
    let c = p.timeline.clips[1].id;
    let id = p.timeline.new_overlap_id();
    p.timeline.add_overlap(c, overlap(id, 1, 3)).unwrap();
    let json = json_of(&p);
    assert!(json.contains("\"schema_version\": 14"), "{json}");
    assert!(json.contains("clip_overlaps"));
    let (q, notes, conv) = reopened(&json);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(conv, None, "nothing is converted in a 14 file");
    assert_eq!(q.timeline.overlaps_of(c), p.timeline.overlaps_of(c));
    assert!(q.timeline.next_overlap_id > id);
    // a project without overlaps writes none of it
    let plain = json_of(&two_clips());
    assert!(!plain.contains("clip_overlaps") && !plain.contains("next_overlap_id"));
    // the crop is never written
    assert!(!plain.contains("\"crop\""));
}

// covers: REQ-029
#[test]
fn format_14_is_the_newest_and_a_15_is_refused_untouched() {
    assert_eq!(SCHEMA_VERSION, 14);
    let json = json_of(&two_clips()).replace("\"schema_version\": 14", "\"schema_version\": 15");
    assert_eq!(
        from_json(&json, std::path::Path::new("/tmp")).unwrap_err(),
        ProjectError::TooNew(15)
    );
}

// covers: REQ-057
#[test]
fn damaged_overlaps_are_dropped_and_counted_in_one_line() {
    let mut p = two_clips();
    let c = p.timeline.clips[0].id;
    p.timeline.put_overlaps(
        c,
        vec![overlap(1, 0, 2), overlap(2, 1, 2), overlap(3, 2, 1)],
    );
    let json = json_of(&p);
    let bad = |field: &str, value: serde_json::Value| {
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        v["timeline"]["clip_overlaps"][0]["items"][1][field] = value;
        v.to_string()
    };
    // an unknown kind, a length of 0, an overlap past the clip's end, a repeated id, a box
    // above 100%, a picture above 16 megapixels
    for damaged in [
        bad("kind", "sparkle".into()),
        bad("len", 0.into()),
        bad("from", 9_000_000.into()),
        bad("id", 1.into()),
        bad("width", 1500.into()),
        bad("src_w", 40_000.into()),
    ] {
        let (q, notes, _) = reopened(&damaged);
        assert_eq!(q.timeline.overlaps_of(c).len(), 2, "{damaged}");
        assert!(
            notes
                .iter()
                .any(|n| n == "1 damaged overlap(s) were removed"),
            "{notes:?}"
        );
    }
    // an entry of a clip that is not there goes too
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["timeline"]["clip_overlaps"][0]["clip"] = 77.into();
    let (q, notes, _) = reopened(&v.to_string());
    assert!(q.timeline.overlaps_of(c).is_empty());
    assert!(notes.iter().any(|n| n.contains("3 damaged")), "{notes:?}");
}

/// A format 13 project: two clips, a crop on the first, a picture item at 5 s (on clip 2,
/// which starts at 4 s), a picture item past the video, volume keys and a speed.
fn v13_json() -> String {
    let mut p = two_clips();
    let (a, b) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
    p.timeline.set_speed(b, 1500).unwrap();
    p.timeline.set_volume_key(a, Micros::ZERO, 0).unwrap();
    p.timeline.set_volume_key(a, s(3), 100).unwrap();
    let pic = crate::picture::Picture {
        path: "/tmp/logo.png".into(),
        width: 400,
        height: 300,
    };
    let mut item = TextItem::new_picture(p.texts.new_id(), pic.clone(), s(5), (1920, 1080));
    item.ttl = s(2);
    let id = p.texts.add(item).unwrap();
    let mut far = TextItem::new_picture(p.texts.new_id(), pic, s(30), (1920, 1080));
    far.ttl = s(2);
    p.texts.add(far).unwrap();
    // the row of the picture on clip 2, and a crop row on clip 1
    p.timeline.set_order(b, vec![Row::Picture(id.0)]);
    p.timeline.set_order(a, vec![Row::Unknown]);
    // as 2.2.0 wrote it: format 13, the crop of clip 1 (10% left), a crop row
    let mut v: serde_json::Value = serde_json::from_str(&json_of(&p)).unwrap();
    v["schema_version"] = 13.into();
    v["timeline"]["clips"][0]["crop"] =
        serde_json::json!({"left": 10, "top": 0, "right": 0, "bottom": 0});
    for e in v["timeline"]["clip_order"].as_array_mut().unwrap() {
        if e["clip"] == a.0 {
            e["rows"] = serde_json::json!(["crop"]);
        }
    }
    v.to_string()
}

// covers: REQ-330, REQ-332
#[test]
fn a_format_13_project_is_converted_in_memory_with_one_status_line() {
    let json = v13_json();
    assert!(
        json.contains("\"crop\":{\"bottom\":0,\"left\":10"),
        "{json}"
    );
    let (p, notes, conv) = reopened(&json);
    let (a, b) = (p.timeline.clips[0], p.timeline.clips[1]);
    // the crop is dropped; the speed and the volume keys are kept as stored
    assert!(a.crop.is_none() && a.crop == Crop::default());
    assert_eq!(b.speed, 1500);
    assert_eq!(p.timeline.volume_keys(a.id).len(), 2);
    // the picture item under clip 2 became its overlap, the same picture, place and time
    let o = &p.timeline.overlaps_of(b.id)[0];
    assert_eq!(o.path, "/tmp/logo.png");
    assert_eq!((o.from, o.len), (s(1), s(2)));
    assert_eq!(o.kind, Kind::Still);
    assert_eq!((o.src_w, o.src_h), (400, 300));
    assert!(!o.reshaped);
    // the item past the video stays a picture item; the other became the overlap
    assert_eq!(p.texts.items.len(), 1);
    assert!(p.texts.items[0].picture.is_some());
    assert_eq!(p.texts.items[0].start, s(30));
    // the row follows; the crop row is dropped without a note
    assert_eq!(p.timeline.order_of(b.id), &[Row::Overlap(o.id)]);
    assert!(p.timeline.order_of(a.id).is_empty());
    // one status line, once
    let c = conv.expect("something changed");
    assert_eq!((c.clips, c.crops, c.pictures), (2, 1, 1));
    assert_eq!(
        c.message(),
        "Opening: 2 clip(s) changed by the conversion (1 crop(s) dropped, 1 picture(s) made overlaps)"
    );
    assert!(notes.is_empty(), "{notes:?}");
    // saved again it is format 14 and converts no more
    let again = json_of(&p);
    assert!(again.contains("\"schema_version\": 14") && !again.contains("\"crop\""));
    let (_, _, conv) = reopened(&again);
    assert_eq!(conv, None);
}

// covers: REQ-332, REQ-330
#[test]
fn a_project_with_nothing_to_convert_opens_unchanged_and_with_no_line() {
    let json = json_of(&two_clips()).replace("\"schema_version\": 14", "\"schema_version\": 13");
    let (p, notes, conv) = reopened(&json);
    assert_eq!(conv, None);
    assert!(notes.is_empty());
    assert!(!p.timeline.any_overlap());
    assert_eq!(
        Conversion {
            clips: 1,
            crops: 1,
            pictures: 0
        }
        .message(),
        "Opening: 1 clip(s) changed by the conversion (1 crop(s) dropped)"
    );
}
