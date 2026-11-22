// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The order of the movable effects in the project (REQ-304, REQ-311).

use super::*;
use crate::effect_order::Row;
use crate::text::{TextId, TextItem};
use crate::timeline::tests::media;

/// Two clips of 10 s and three texts: A and B over clip 1 only, C over both.
fn three_texts() -> (Project, Vec<crate::timeline::ClipId>) {
    let mut p = Project::default();
    let id = p.new_media_id();
    let m = media(id.0, 10, true);
    p.add_media(m.clone());
    p.timeline.append(&m).unwrap();
    p.timeline.append(&m).unwrap();
    for (name, start) in [("A", 1), ("B", 2), ("C", 8)] {
        let mut t = TextItem::new(TextId(0), name.into(), Micros(start * 1_000_000));
        t.id = TextId(p.texts.next_id + 1);
        p.texts.add(t).unwrap();
    }
    let ids = p.timeline.clips.iter().map(|c| c.id).collect();
    (p, ids)
}

fn reopened(p: &Project) -> (Project, Vec<String>) {
    let json = to_json(p, Path::new("/tmp/x.vlproj")).unwrap();
    from_json(&json, Path::new("/tmp")).unwrap()
}

// covers: REQ-302
#[test]
fn a_clip_has_the_rows_of_the_items_that_overlap_it() {
    let (p, ids) = three_texts();
    let rows = p.clip_rows();
    assert_eq!(rows.len(), 2);
    // A (1-4 s), B (2-5 s) and C (8-11 s) overlap clip 1; only C overlaps clip 2 (10-20 s)
    assert_eq!(rows[0].present.texts, vec![1, 2, 3]);
    assert_eq!(rows[1].present.texts, vec![3]);
    assert_eq!(
        (rows[1].start, rows[1].end),
        (Micros(10_000_000), Micros(20_000_000))
    );
    assert_eq!(p.rows_of(ids[0]).unwrap().clip, ids[0]);
    assert!(p.rows_of(crate::timeline::ClipId(99)).is_none());
    assert_eq!(p.order_of(crate::timeline::ClipId(99)), vec![]);
    // the older order: the later item is on top
    assert_eq!(
        p.order_of(ids[0]),
        vec![Row::Text(3), Row::Text(2), Row::Text(1)]
    );
    assert!(p.changed_orders().is_empty());
}

// covers: REQ-304
// covers: REQ-311
#[test]
fn the_order_belongs_to_the_clip_and_is_saved_only_when_it_changed() {
    let (mut p, ids) = three_texts();
    // text 1 above the others on clip 1
    p.set_order(ids[0], vec![Row::Text(1), Row::Text(3), Row::Text(2)]);
    assert_eq!(p.timeline.clip_order.len(), 1);
    assert_eq!(
        p.order_of(ids[1]),
        vec![Row::Text(3)],
        "clip 2 keeps its order"
    );
    let changed = p.changed_orders();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0.clip, ids[0]);
    let (q, notes) = reopened(&p);
    assert!(notes.iter().all(|n| !n.contains("repaired")), "{notes:?}");
    assert_eq!(q.timeline.clip_order, p.timeline.clip_order);
    let json = to_json(&p, Path::new("/tmp/x.vlproj")).unwrap();
    assert!(json.contains("text:1"), "{json}");
    // the older order is not stored
    p.set_order(ids[0], vec![Row::Text(3), Row::Text(2), Row::Text(1)]);
    assert!(p.timeline.clip_order.is_empty());
    let json = to_json(&p, Path::new("/tmp/x.vlproj")).unwrap();
    assert!(!json.contains("clip_order"), "{json}");
}

// covers: REQ-304
#[test]
fn a_new_row_goes_on_top_of_the_clips_it_shows_on() {
    let (mut p, ids) = three_texts();
    p.set_order(ids[0], vec![Row::Text(1), Row::Text(3), Row::Text(2)]);
    p.put_on_top(Row::Text(2), None);
    assert_eq!(
        p.order_of(ids[0]),
        vec![Row::Text(2), Row::Text(1), Row::Text(3)]
    );
    // on the other clip the row is not there: nothing is stored for it
    assert!(p.timeline.order_of(ids[1]).is_empty());
    // a row that no clip has changes nothing
    let before = p.timeline.clone();
    p.put_on_top(Row::Text(77), None);
    p.put_on_top(Row::Unknown, None);
    p.put_on_top(Row::Text(2), Some(&[]));
    assert_eq!(p.timeline, before);
}

// covers: REQ-311
#[test]
fn removing_merging_and_splitting_clips_keeps_the_orders_right() {
    let (mut p, ids) = three_texts();
    p.set_order(ids[0], vec![Row::Text(1), Row::Text(3), Row::Text(2)]);
    let right = p.timeline.split(Micros(5_000_000), Micros(1)).unwrap();
    assert_eq!(
        p.timeline.order_of(right),
        p.timeline.order_of(ids[0]),
        "both halves keep the order"
    );
    p.timeline.merge(ids[0]).unwrap();
    assert!(p.timeline.order_of(right).is_empty());
    p.timeline.remove(ids[0]).unwrap();
    assert!(p.timeline.clip_order.is_empty());
}

// covers: REQ-311
// covers: REQ-057
#[test]
fn a_damaged_order_is_repaired_with_a_note_and_never_stops_the_file() {
    let (mut p, ids) = three_texts();
    p.set_order(ids[0], vec![Row::Text(1), Row::Text(3), Row::Text(2)]);
    let json = to_json(&p, Path::new("/tmp/x.vlproj")).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let id0 = ids[0].0;
    let damaged = [
        // an unknown row name, a repeated row, a text that does not exist
        serde_json::json!({"clip": id0, "rows": ["text:1", "mystery", "text:1", "text:99", 7]}),
        // a clip that is not there
        serde_json::json!({"clip": 4242, "rows": ["lut"]}),
        // not an entry at all
        serde_json::json!("nonsense"),
        serde_json::json!({"clip": "x"}),
    ];
    for d in damaged {
        let mut w = v.clone();
        w["timeline"]["clip_order"] = serde_json::json!([d]);
        let (q, notes) = from_json(&w.to_string(), Path::new("/tmp")).unwrap();
        assert!(
            notes.iter().any(|n| n.contains("damaged effect order")),
            "{notes:?}"
        );
        // what could be kept is kept: texts 1 only, which is a changed order for clip 1?
        assert!(q.timeline.clip_order.len() <= 1);
        for o in &q.timeline.clip_order {
            assert!(!o.rows.contains(&Row::Unknown));
        }
    }
    // a good file has no note about it
    v["timeline"]["clip_order"] = serde_json::json!([]);
    let (_, notes) = from_json(&v.to_string(), Path::new("/tmp")).unwrap();
    assert!(notes.iter().all(|n| !n.contains("effect order")));
    // two entries for one clip: the first is kept
    v["timeline"]["clip_order"] = serde_json::json!([
        {"clip": id0, "rows": ["text:1", "text:3", "text:2"]},
        {"clip": id0, "rows": ["text:2", "text:1", "text:3"]},
    ]);
    let (q, notes) = from_json(&v.to_string(), Path::new("/tmp")).unwrap();
    assert!(notes.iter().any(|n| n.contains("damaged effect order")));
    assert_eq!(q.timeline.order_of(ids[0])[0], Row::Text(1));
}

// covers: REQ-311
#[test]
fn a_project_without_orders_opens_and_saves_as_before() {
    let (p, _) = three_texts();
    let (q, notes) = reopened(&p);
    assert!(q.timeline.clip_order.is_empty());
    assert!(notes.iter().all(|n| !n.contains("effect order")));
    let old = to_json(&p, Path::new("/tmp/x.vlproj"))
        .unwrap()
        .replace("\"schema_version\": 13", "\"schema_version\": 11");
    let (r, _) = from_json(&old, Path::new("/tmp")).unwrap();
    assert!(r.timeline.clip_order.is_empty());
    assert_eq!(r.changed_orders(), vec![]);
}

/// Two clips of 10 s; the first has a Crop, a Scale, a Move, a fade at its start, a Change
/// aspect and a text; the second only the Change aspect of the project.
fn every_picture_row() -> (Project, Vec<crate::timeline::ClipId>) {
    use crate::timeline::{Crop, FrameTarget};
    let (mut p, ids) = three_texts();
    let c = FrameTarget::Clip(ids[0]);
    p.timeline
        .set_crop(
            ids[0],
            Crop {
                left: 10,
                ..Default::default()
            },
        )
        .unwrap();
    p.timeline.add_scale(c).unwrap();
    p.timeline.add_move(c).unwrap();
    p.timeline.set_edge_fades(true).unwrap();
    p.timeline
        .set_frame_aspect(Some(crate::frame::FrameAspect::default()))
        .unwrap();
    (p, ids)
}

// covers: REQ-320, REQ-322
#[test]
fn every_picture_row_is_in_the_clips_order_in_the_build_order_of_the_older_versions() {
    let (p, ids) = every_picture_row();
    let r = p.rows_of(ids[0]).unwrap().present;
    assert!(r.crop && r.scale && r.moved && r.fade && r.frame);
    // the default: texts (the later on top), then Fade, Change aspect, Move, Scale, Crop
    assert_eq!(
        p.order_of(ids[0]),
        vec![
            Row::Text(3),
            Row::Text(2),
            Row::Text(1),
            Row::Fade,
            Row::Frame,
            Row::Move,
            Row::Scale,
            Row::Crop
        ]
    );
    // the second clip has the project's Change aspect and the fade at the end of the video
    assert_eq!(
        p.order_of(ids[1]),
        vec![Row::Text(3), Row::Fade, Row::Frame]
    );
    assert!(p.changed_orders().is_empty());
}

// covers: REQ-320, REQ-321, REQ-322
#[test]
fn a_moved_picture_row_is_stored_with_its_clip_and_comes_back_and_a_new_row_takes_its_default_place()
 {
    let (mut p, ids) = every_picture_row();
    // Scale above Move: stored in full, for the first clip only
    let mut o = p.order_of(ids[0]);
    o.retain(|r| *r != Row::Scale);
    let at = o.iter().position(|r| *r == Row::Move).unwrap();
    o.insert(at, Row::Scale);
    p.set_order(ids[0], o.clone());
    assert_eq!(p.order_of(ids[0]), o);
    assert!(p.timeline.order_of(ids[1]).is_empty());
    assert_eq!(p.changed_orders().len(), 1);
    assert!(crate::effect_order::needs_steps(&p.changed_orders()[0].1));
    let (q, notes) = reopened(&p);
    assert!(
        notes.iter().all(|n| !n.contains("effect order")),
        "{notes:?}"
    );
    assert_eq!(q.order_of(ids[0]), o);
    assert!(q.timeline.order_of(ids[1]).is_empty());
    // a row that is added later (here a Crop on the second clip) takes its place in the default
    // sequence, not the top
    p.timeline
        .set_crop(
            ids[1],
            crate::timeline::Crop {
                left: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        p.order_of(ids[1]),
        vec![Row::Text(3), Row::Fade, Row::Frame, Row::Crop]
    );
    assert!(p.timeline.order_of(ids[1]).is_empty());
    // the sound rows of a clip survive a change of its picture rows
    p.set_sound_order(ids[0], &[Row::Volume, Row::Clean]);
    p.set_order(ids[0], p.order_of(ids[0]));
    assert!(p.timeline.order_of(ids[0]).contains(&Row::Volume));
}

// covers: REQ-322
#[test]
fn a_2_1_0_order_opens_with_its_rows_above_the_default_sequence_of_the_new_ones() {
    let (p, ids) = every_picture_row();
    let json = to_json(&p, Path::new("/tmp/x.vlproj")).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    // what 2.1.0 wrote: a clip order with movable rows only (format 12)
    v["schema_version"] = serde_json::json!(12);
    v["timeline"]["clip_order"] =
        serde_json::json!([{"clip": ids[0].0, "rows": ["text:1", "text:2", "text:3"]}]);
    let (q, notes) = from_json(&v.to_string(), Path::new("/tmp")).unwrap();
    assert!(
        notes.iter().all(|n| !n.contains("effect order")),
        "{notes:?}"
    );
    assert_eq!(
        q.order_of(ids[0]),
        vec![
            Row::Text(1),
            Row::Text(2),
            Row::Text(3),
            Row::Fade,
            Row::Frame,
            Row::Move,
            Row::Scale,
            Row::Crop
        ]
    );
    // it is the form of the 2.1.0 overlay stage: no step builder
    assert!(!crate::effect_order::needs_steps(&q.changed_orders()[0].1));
}
