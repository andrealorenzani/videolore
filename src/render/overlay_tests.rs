// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The overlay stage (REQ-304, REQ-305, REQ-311): what a clip whose effects were put in
//! another order draws, and that a clip with the old order keeps its old arguments (QG-21).

use super::tests::{enc, project};
use super::*;
use crate::effect_order::Row;
use crate::text::TextItem;
use crate::timeline::FrameTarget;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

fn geometry() -> Geometry {
    Geometry {
        width: 640,
        height: 360,
        fps: Fps::new(25, 1).unwrap(),
    }
}

fn add_text(p: &mut Project, text: &str, start: Micros, ttl: Micros) -> u64 {
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, text.into(), start);
    t.ttl = ttl;
    p.texts.add(t).unwrap();
    id.0
}

/// One 8 s clip with a LUT and a text (2 s to 5 s).
fn lut_and_text(dir: &Path) -> (Project, u64) {
    let mut p = project(&[(8, 640, 360, 25)]);
    let file = dir.join("a.cube");
    std::fs::write(&file, crate::lut::tests::identity(2)).unwrap();
    p.timeline
        .set_lut(
            FrameTarget::Whole,
            Some(file.to_string_lossy().into_owned()),
        )
        .unwrap();
    let t = add_text(&mut p, "Hello", s(2), s(3));
    (p, t)
}

fn reencode(p: &Project) -> (Graph, Option<String>) {
    match plan_export(p, &enc(), |_, _| true).unwrap() {
        Plan::Reencode {
            graph, subtitles, ..
        } => (*graph, subtitles),
        Plan::StreamCopy { .. } => panic!("expected a re-encode"),
    }
}

// covers: REQ-311
// covers: REQ-305
#[test]
fn a_clip_with_the_old_order_keeps_its_lut_in_the_chain_and_its_text_after_the_concat() {
    let dir = tempfile::tempdir().unwrap();
    let (mut p, _) = lut_and_text(dir.path());
    let (old, doc) = reencode(&p);
    assert!(doc.is_some_and(|d| d.contains("Hello")));
    let g = &old.graph;
    assert!(g.contains("decrease,format=gbrp,lut3d=file="), "{g}");
    assert!(g.contains("[cv]subtitles=f=subs.ass"), "{g}");
    assert!(!g.contains("ov1-"), "{g}");
    assert!(old.command_files.is_empty());
    // the legacy order stored by hand is not an overlay either
    let c = p.timeline.clips[0].id;
    p.timeline.clip_order = vec![crate::timeline::ClipOrder {
        clip: c,
        rows: vec![Row::Text(1), Row::Lut],
    }];
    assert!(p.changed_orders().is_empty());
    let (same, _) = reencode(&p);
    assert_eq!(same, old, "the arguments of the old order do not change");
}

// covers: REQ-304
#[test]
fn a_lut_above_a_text_grades_the_text_and_a_text_above_it_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let (mut p, t) = lut_and_text(dir.path());
    let c = p.timeline.clips[0].id;
    // top first: the LUT above the text
    p.set_order(c, vec![Row::Lut, Row::Text(t)]);
    let (graph, doc) = reencode(&p);
    assert!(
        doc.is_none(),
        "the text is drawn in the clip, not after the concat"
    );
    let g = &graph.graph;
    assert!(
        !g.contains("decrease,format=gbrp"),
        "no LUT in the clip chain: {g}"
    );
    let text = g.find("subtitles=f=ov1-t1.ass").expect(g);
    let lut = g.find("lut3d=file=").expect(g);
    assert!(text < lut, "the text is drawn first, then graded: {g}");
    assert!(lut < g.find("[v0]").unwrap());
    let (name, ass) = &graph.command_files[0];
    assert_eq!(name, "ov1-t1.ass");
    // the clip's own time: 2 s to 5 s of the video
    assert!(ass.contains("0:00:02.00,0:00:05.00"), "{ass}");
    assert_eq!(graph.lut_files.len(), 1);
    // the same text above the LUT: the old order, nothing in the clip
    p.set_order(c, vec![Row::Text(t), Row::Lut]);
    assert!(p.timeline.clip_order.is_empty());
}

// covers: REQ-304
#[test]
fn a_text_overlapping_two_clips_has_the_order_of_each_clip() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = project(&[(4, 640, 360, 25), (4, 640, 360, 25)]);
    let file = dir.path().join("a.cube");
    std::fs::write(&file, crate::lut::tests::identity(2)).unwrap();
    let ids: Vec<_> = p.timeline.clips.iter().map(|c| c.id).collect();
    p.timeline
        .set_lut(
            FrameTarget::Clip(ids[1]),
            Some(file.to_string_lossy().into_owned()),
        )
        .unwrap();
    // 2 s to 6 s: the second half of clip 1 and the first half of clip 2
    let t = add_text(&mut p, "Both", s(2), s(4));
    p.set_order(ids[1], vec![Row::Lut, Row::Text(t)]);
    let (graph, doc) = reencode(&p);
    // clip 1 keeps the old way: the text over 2 s to 4 s is in the document after the concat
    let doc = doc.expect("the part over clip 1");
    assert!(doc.contains("0:00:02.00,0:00:04.00"), "{doc}");
    assert!(!doc.contains("0:00:06.00"), "{doc}");
    // clip 2 draws its part in its own time: 0 s to 2 s
    let (name, ass) = &graph.command_files[0];
    assert_eq!(name, "ov2-t1.ass");
    assert!(ass.contains("0:00:00.00,0:00:02.00"), "{ass}");
    let g = &graph.graph;
    assert!(g.contains("[1:v:0]") || g.contains("[v1]"));
    assert_eq!(g.matches("subtitles=f=").count(), 2, "{g}");
}

// covers: REQ-305
#[test]
fn overlapping_texts_are_drawn_in_the_order_of_the_rows() {
    let mut p = project(&[(6, 640, 360, 25)]);
    let a = add_text(&mut p, "A", s(1), s(4));
    let b = add_text(&mut p, "B", s(1), s(4));
    let c = p.timeline.clips[0].id;
    // the older order: B (the later item) over A
    assert_eq!(p.order_of(c), vec![Row::Text(b), Row::Text(a)]);
    let (_, doc) = reencode(&p);
    let doc = doc.unwrap();
    assert!(doc.find("}A").unwrap() < doc.find("}B").unwrap());
    // A above B: each text in its own step, B first (below)
    p.set_order(c, vec![Row::Text(a), Row::Text(b)]);
    let (graph, doc) = reencode(&p);
    assert!(doc.is_none());
    let names: Vec<&str> = graph.command_files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(
        names,
        ["ov1-t2.ass", "ov1-t1.ass"],
        "B is drawn first, A over it"
    );
    let g = &graph.graph;
    assert!(g.find("ov1-t2.ass").unwrap() < g.find("ov1-t1.ass").unwrap());
    // moving it back is the old order again
    p.set_order(c, vec![Row::Text(b), Row::Text(a)]);
    assert!(p.timeline.clip_order.is_empty());
}

// covers: REQ-304
#[test]
fn a_picture_item_is_overlaid_in_the_clip_chain_and_the_rest_of_the_video_keeps_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("logo.png");
    std::fs::write(&png, crate::picture::tests::header(200, 100)).unwrap();
    let mut p = project(&[(4, 640, 360, 25), (4, 640, 360, 25)]);
    let pic = crate::picture::read(&png).unwrap();
    let id = p.texts.new_id();
    let mut item = TextItem::new_picture(id, pic, s(2), (640, 360));
    item.ttl = s(4);
    p.texts.add(item).unwrap();
    let t = add_text(&mut p, "Hi", s(5), s(1));
    let ids: Vec<_> = p.timeline.clips.iter().map(|c| c.id).collect();
    // on clip 2 the text is below the picture
    p.set_order(ids[1], vec![Row::Picture(id.0), Row::Text(t)]);
    assert_eq!(p.changed_orders().len(), 1);
    let (graph, _) = reencode(&p);
    // the global overlay shows the picture over clip 1 only (2 s to 4 s)
    let g = &graph.graph;
    assert!(g.contains("enable='between(t,2.000000,4.000000)'"), "{g}");
    // two picture inputs: the global one and the one of clip 2
    assert_eq!(graph.inputs.iter().filter(|i| i.still).count(), 2);
    assert!(
        g.contains("[o1x1]") && g.contains("[q1x1]null[v1]") || g.contains("[q1x1]"),
        "{g}"
    );
    assert!(
        g.contains("enable='between(t,0.000000,2.000000)'"),
        "clip 2 shows it from its start to 2 s: {g}"
    );
    // a project without a changed order keeps one plain `between`
    p.timeline.clip_order.clear();
    let (plain, _) = reencode(&p);
    assert!(
        plain
            .graph
            .contains("enable='between(t,2.000000,6.000000)'")
    );
    assert_eq!(plain.inputs.iter().filter(|i| i.still).count(), 1);
}

// covers: REQ-304
#[test]
fn the_lines_of_a_clip_are_one_row_drawn_in_the_clip() {
    let mut p = project(&[(6, 640, 360, 25)]);
    let tl = p.timeline.clone();
    p.subtitles.add_at(&tl, s(1), "spoken words", s(2)).unwrap();
    let t = add_text(&mut p, "Title", s(1), s(3));
    let c = p.timeline.clips[0].id;
    // the older order: text over lines
    assert_eq!(p.order_of(c), vec![Row::Text(t), Row::Subtitles]);
    p.set_order(c, vec![Row::Subtitles, Row::Text(t)]);
    let (graph, doc) = reencode(&p);
    assert!(doc.is_none());
    let names: Vec<&str> = graph.command_files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(names, ["ov1-t1.ass", "ov1-s.ass"]);
    assert!(graph.command_files[1].1.contains("spoken words"));
    // the lines go to a separate track: the clip draws no lines
    let mut segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    overlay::attach(
        &p,
        &mut segs,
        &geometry(),
        Micros::ZERO,
        Source::Export,
        false,
        true,
    )
    .unwrap();
    assert_eq!(segs[0].overlay.len(), 1);
}

// covers: REQ-304
#[test]
fn a_preview_from_the_middle_counts_the_times_of_the_clip_from_there() {
    let dir = tempfile::tempdir().unwrap();
    let (mut p, t) = lut_and_text(dir.path());
    let c = p.timeline.clips[0].id;
    p.set_order(c, vec![Row::Lut, Row::Text(t)]);
    let g = geometry().preview();
    let sc = Path::new("/tmp/g.txt");
    let r = preview_video_args(&p, s(3), &g, FfVersion::Dev, sc, Source::Export).unwrap();
    // the text shows 2 s to 5 s: from 3 s on it shows for the first 2 s
    let (name, doc) = &r.command_files[0];
    assert_eq!(name, "ov1-t1.ass");
    assert!(doc.contains("0:00:00.00,0:00:02.00"), "{doc}");
    assert_eq!(r.lut_files.len(), 1);
    assert!(r.subtitles.is_none());
    // after the end of the text nothing is drawn for it
    let r = preview_video_args(&p, s(6), &g, FfVersion::Dev, sc, Source::Export).unwrap();
    assert!(r.command_files.is_empty());
    // the still frame and the output check use the same clip steps
    let still = still_args(&p, s(3), &g, FfVersion::Dev, sc, Source::Export).unwrap();
    assert_eq!(still.command_files[0].0, "ov1-t1.ass");
    let ex = geometry();
    let check = check_range_graph(&p, &ex, Micros::ZERO, s(8), true).unwrap();
    assert!(check.graph.graph.contains("ov1-t1.ass"));
    assert!(check.subtitles.is_none());
    let whole = check_graph(&p, &ex).unwrap();
    assert!(whole.graph.contains("lut3d") && whole.graph.contains("ov1-t1.ass"));
    // the background alone has no LUT and no texts
    let bg = still_args_bg(
        &p,
        s(3),
        &g,
        FfVersion::Dev,
        sc,
        Source::Export,
        Background::Only,
        None,
    )
    .unwrap();
    assert!(bg.command_files.is_empty() && bg.lut_files.is_empty());
}

// covers: REQ-304
#[test]
fn a_360_video_export_ignores_the_order_and_keeps_everything_in_one_document() {
    let mut p = project(&[(4, 3840, 1920, 25)]);
    p.media[0].format.spherical = Some(true);
    let a = add_text(&mut p, "A", s(1), s(2));
    let b = add_text(&mut p, "B", s(1), s(2));
    let c = p.timeline.clips[0].id;
    p.set_order(c, vec![Row::Text(a), Row::Text(b)]);
    assert_eq!(p.changed_orders().len(), 1);
    let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
    let doc = plan.subtitles().expect("both texts in the one document");
    assert!(doc.contains("}A") && doc.contains("}B"));
    assert!(plan.command_files().is_empty());
}

// covers: REQ-304
#[test]
fn the_windows_of_the_old_clips_join_and_leave_the_changed_ones_out() {
    let mut p = project(&[(2, 640, 360, 25), (2, 640, 360, 25), (2, 640, 360, 25)]);
    assert!(overlay::legacy_windows(&p).is_none());
    let t = add_text(&mut p, "X", s(0), s(6));
    let u = add_text(&mut p, "Y", s(0), s(6));
    let ids: Vec<_> = p.timeline.clips.iter().map(|c| c.id).collect();
    p.set_order(ids[1], vec![Row::Text(t), Row::Text(u)]);
    assert_eq!(overlay::changed_ids(&p), vec![ids[1]]);
    assert_eq!(
        overlay::legacy_windows(&p).unwrap(),
        vec![(s(0), s(2)), (s(4), s(6))]
    );
    // the picture of an item over the three clips is shown over the two old ones
    let png = tempfile::tempdir().unwrap();
    let f = png.path().join("p.png");
    std::fs::write(&f, crate::picture::tests::header(10, 10)).unwrap();
    let pic = crate::picture::read(&f).unwrap();
    let id = p.texts.new_id();
    let mut item = TextItem::new_picture(id, pic, s(0), (640, 360));
    item.ttl = s(6);
    p.texts.add(item).unwrap();
    // the picture is also a row of the middle clip: it is drawn there in the clip
    let o = picture_overlays(&p, &geometry(), Micros::ZERO, Source::Export).unwrap();
    assert_eq!(o[0].pieces, vec![(s(0), s(2)), (s(4), s(6))]);
}

/// A clip with a Scale and a Move (both kept at the default keys), an 8 s clip at 640 x 360.
fn scaled_and_moved() -> (Project, crate::timeline::ClipId) {
    let mut p = project(&[(8, 640, 360, 25)]);
    let c = p.timeline.clips[0].id;
    p.timeline.add_scale(FrameTarget::Clip(c)).unwrap();
    p.timeline.add_move(FrameTarget::Clip(c)).unwrap();
    (p, c)
}

// covers: REQ-320, REQ-321
#[test]
fn a_clip_with_the_default_order_keeps_its_arguments_and_another_order_is_built_by_steps() {
    let (mut p, c) = scaled_and_moved();
    let (old, _) = reencode(&p);
    // the default order is drawn by the frame effect of the older versions: no layer, no step
    assert!(!old.graph.contains("0x00000000"), "{}", old.graph);
    assert!(old.graph.contains("[pm0]"), "{}", old.graph);
    // Scale above Move: the chain is built row by row on a transparent layer
    p.set_order(c, vec![Row::Scale, Row::Move]);
    assert!(crate::effect_order::needs_steps(&p.changed_orders()[0].1));
    let (new, doc) = reencode(&p);
    let g = &new.graph;
    assert!(doc.is_none() && new.command_files.is_empty());
    assert!(g.contains("color=c=0x00000000"), "{g}");
    // Move first (the offset only), then Scale; the chain ends on the clock of the clip
    let mv = g.find("eval=frame:format=auto:shortest=1").expect(g);
    let sc = g.find("scale=w='max(2,trunc(iw*").expect(g);
    assert!(mv < sc, "the Move is built before the Scale: {g}");
    assert!(g.contains("setpts=PTS-STARTPTS[v0]"), "{g}");
    // back to the default order: the same arguments as before
    p.set_order(c, vec![Row::Move, Row::Scale]);
    let (same, _) = reencode(&p);
    assert_eq!(same, old);
}

// covers: REQ-320
#[test]
fn a_text_below_change_aspect_is_drawn_by_a_matte_and_above_it_on_the_frame() {
    let mut p = project(&[(8, 640, 360, 25)]);
    let c = p.timeline.clips[0].id;
    p.timeline
        .set_frame_aspect(Some(crate::frame::FrameAspect::default()))
        .unwrap();
    p.timeline.add_scale(FrameTarget::Clip(c)).unwrap();
    let t = add_text(&mut p, "Hello", s(2), s(3));
    // the text above the frame: drawn on the composed frame
    p.set_order(c, vec![Row::Text(t), Row::Scale, Row::Frame]);
    let (g, _) = reencode(&p);
    assert!(g.graph.contains("subtitles=f=ov1-t1.ass"), "{}", g.graph);
    assert!(!g.graph.contains("alphamerge"), "{}", g.graph);
    // the frame above the text: the text is drawn on the layer, apart from the picture, on black
    // and on white, and laid over the layer with its own transparency
    p.set_order(c, vec![Row::Frame, Row::Scale, Row::Text(t)]);
    let (g, _) = reencode(&p);
    for want in [
        "color=c=black",
        "color=c=white",
        "blend=all_expr='255-(A-B)'",
        "alphamerge",
    ] {
        assert!(g.graph.contains(want), "{want} in {}", g.graph);
    }
    // the document is written in the clip's own time (the text shows 2 s to 5 s)
    assert!(g.command_files[0].1.contains("0:00:02.00,0:00:05.00"));
}
