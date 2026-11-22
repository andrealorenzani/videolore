// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Overlap media in the render (2.3.0, ADR-232, ADR-233, ADR-243): the argument lists of a still,
//! an animation and a reshaped overlay; the paths of the default and of a changed order; the
//! half-size preview of large files; a missing or too large file; the same arguments for a
//! project without overlaps (REQ-325, REQ-327, REQ-328, REQ-331).

use super::tests::{enc, project, spec, strs};
use super::*;
use crate::base::pos::Pos;
use crate::effect_order::Row;
use crate::overlap::{Kind, Overlap};

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

/// A file with the header of a picture of `w` x `h` and the overlap of it on clip 1: from 2 s
/// for 3 s, in the middle, in a 30% x 30% box.
fn with_overlap(
    dir: &Path,
    name: &str,
    kind: Kind,
    (w, h): (u32, u32),
    reshaped: bool,
) -> (Project, u64) {
    let mut p = project(&[(8, 640, 360, 25)]);
    let path = dir.join(name);
    let head = if name.ends_with(".gif") {
        crate::picture::tests::gif_header(w as u16, h as u16)
    } else if name.ends_with(".png") {
        crate::picture::tests::header(w, h)
    } else {
        b"a video file".to_vec()
    };
    std::fs::write(&path, head).unwrap();
    let clip = p.timeline.clips[0].id;
    let id = p.timeline.new_overlap_id();
    p.timeline
        .add_overlap(
            clip,
            Overlap {
                id,
                path: path.to_string_lossy().into_owned(),
                kind,
                from: s(2),
                len: s(3),
                place: Pos { x: 5000, y: 5000 },
                width: 300,
                height: 300,
                reshaped,
                src_w: w,
                src_h: h,
            },
        )
        .unwrap();
    (p, id)
}

fn graph_of(p: &Project) -> (String, Vec<String>) {
    let Plan::Reencode { graph, .. } = plan_export(p, &enc(), |_, _| true).unwrap() else {
        panic!("an overlap is burned in: a re-encode")
    };
    let args = strs(&input_args(&graph.inputs));
    (graph.graph, args)
}

// covers: REQ-325, REQ-331
#[test]
fn a_png_overlap_is_a_looped_still_over_the_video_for_its_time_and_makes_a_re_encode() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = with_overlap(dir.path(), "logo.png", Kind::Still, (400, 200), false);
    // identical clips would be copied: the overlap forces a re-encode (REQ-023)
    let (g, args) = graph_of(&p);
    // a 2:1 picture in a 192 x 108 box: 192 x 96, centred
    assert!(g.contains("[1:v:0]format=rgba,scale=192:96[pi0]"), "{g}");
    assert!(
        g.contains("overlay=x=224:y=132:enable='between(t,2.000000,5.000000)':shortest=1"),
        "{g}"
    );
    assert!(args.windows(2).any(|w| w == ["-loop", "1"]), "{args:?}");
    assert!(
        !g.contains("eof_action") && !g.contains("[1:v:0]format=rgba,setpts"),
        "{g}"
    );
}

// covers: REQ-328
#[test]
fn a_gif_or_video_overlap_plays_once_from_its_start_then_lets_the_picture_through() {
    let dir = tempfile::tempdir().unwrap();
    for (name, size) in [("anim.gif", (64, 64)), ("anim.mov", (1280, 720))] {
        let (p, _) = with_overlap(dir.path(), name, Kind::Motion, size, false);
        let (g, args) = graph_of(&p);
        // read once from its start (no loop), shifted to the overlap's start, without sound
        assert!(
            !args.windows(2).any(|w| w == ["-loop", "1"]),
            "{name}: {args:?}"
        );
        assert!(g.contains("setpts=PTS-STARTPTS+2.000000/TB"), "{name}: {g}");
        assert!(
            g.contains("enable='between(t,2.000000,5.000000)':eof_action=pass"),
            "{name}: {g}"
        );
        assert!(
            !g.contains("shortest=1[") && !g.contains(":shortest=1,format"),
            "{g}"
        );
        // only its video stream is read: no sound of the file enters the graph
        assert!(!g.contains("[1:a"), "{name}: {g}");
    }
}

// covers: REQ-327
#[test]
fn a_changed_shape_crops_a_video_to_the_box_and_stretches_a_gif_or_a_picture() {
    let dir = tempfile::tempdir().unwrap();
    // the box is 192 x 108 on this 640 x 360 picture
    let (p, _) = with_overlap(dir.path(), "clip.mov", Kind::Motion, (640, 360), true);
    let (g, _) = graph_of(&p);
    assert!(
        g.contains("scale=192:108:force_original_aspect_ratio=increase,crop=192:108"),
        "{g}"
    );
    let (p, _) = with_overlap(dir.path(), "anim.gif", Kind::Motion, (640, 360), true);
    let (g, _) = graph_of(&p);
    assert!(g.contains(",scale=192:108[pi0]"), "{g}");
    assert!(!g.contains("force_original_aspect_ratio=increase"), "{g}");
    let (p, _) = with_overlap(dir.path(), "logo.png", Kind::Still, (400, 200), true);
    let (g, _) = graph_of(&p);
    assert!(g.contains("scale=192:108[pi0]"), "{g}");
    // not changed: the media keeps its ratio inside the box (a 4:3 picture: 144 x 108)
    let (p, _) = with_overlap(dir.path(), "four.png", Kind::Still, (400, 300), false);
    let (g, _) = graph_of(&p);
    assert!(g.contains("scale=144:108[pi0]"), "{g}");
}

// covers: REQ-331
#[test]
fn a_file_above_1920_by_1080_is_halved_first_in_the_preview_and_full_size_in_the_export() {
    let dir = tempfile::tempdir().unwrap();
    let preview = Source::Preview { proxies: false };
    let draw =
        |p: &Project, src: Source| picture_overlays(p, &geometry(), Micros::ZERO, src).unwrap();
    for (name, size, kind) in [
        ("big.png", (2000, 1200), Kind::Still),
        ("big.gif", (2000, 1200), Kind::Motion),
    ] {
        let (p, _) = with_overlap(dir.path(), name, kind, size, false);
        let pics = draw(&p, preview);
        let g = build_graph_with(
            &segments(&p, Micros::ZERO, Source::Export).unwrap(),
            None,
            &spec(true, false),
            &pics,
        )
        .graph;
        assert!(
            g.contains("format=rgba,scale=iw/2:ih/2"),
            "{name} preview: {g}"
        );
        let pics = draw(&p, Source::Export);
        let g = build_graph_with(
            &segments(&p, Micros::ZERO, Source::Export).unwrap(),
            None,
            &spec(true, false),
            &pics,
        )
        .graph;
        assert!(!g.contains("iw/2"), "{name} export: {g}");
    }
    // exactly 1920 x 1080 is drawn the same in both
    let (p, _) = with_overlap(dir.path(), "hd.png", Kind::Still, (1920, 1080), false);
    assert_eq!(draw(&p, preview), draw(&p, Source::Export));
}

// covers: REQ-331, REQ-316
#[test]
fn a_missing_or_too_large_overlay_file_stops_the_export_and_is_left_out_of_the_preview() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = with_overlap(dir.path(), "logo.png", Kind::Still, (400, 200), false);
    let path = std::path::PathBuf::from(&p.timeline.clip_overlaps[0].items[0].path);
    let preview = Source::Preview { proxies: false };
    // replaced by a picture above 16 megapixels: a missing file
    std::fs::write(&path, crate::picture::tests::header(4001, 4000)).unwrap();
    assert_eq!(
        plan_export(&p, &enc(), |_, _| true).unwrap_err(),
        RenderError::Picture("logo.png".into())
    );
    assert!(
        picture_overlays(&p, &geometry(), Micros::ZERO, preview)
            .unwrap()
            .is_empty()
    );
    std::fs::remove_file(&path).unwrap();
    assert!(plan_export(&p, &enc(), |_, _| true).is_err());
    assert!(
        picture_overlays(&p, &geometry(), Micros::ZERO, preview)
            .unwrap()
            .is_empty()
    );
}

// covers: REQ-325, REQ-328
#[test]
fn a_render_that_starts_inside_an_animation_reads_it_from_where_it_has_got_to() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = with_overlap(dir.path(), "anim.gif", Kind::Motion, (64, 64), false);
    let at = |from: Micros| picture_overlays(&p, &geometry(), from, Source::Export).unwrap();
    // before the overlap: nothing played yet
    let o = &at(Micros::ZERO)[0];
    assert_eq!((o.start, o.look.skip), (s(2), Micros::ZERO));
    // 0.5 s into the overlap: the file is read from 0.5 s and the overlay is there at once
    let o = &at(Micros(2_500_000))[0];
    assert_eq!(
        (o.start, o.end, o.look.skip),
        (Micros::ZERO, Micros(2_500_000), Micros(500_000))
    );
    let input = o.input(s(5));
    assert_eq!(input.start, Micros(500_000));
    assert!(!input.still);
    // after the end of the overlap: nothing
    assert!(at(s(5)).is_empty());
}

// covers: REQ-331, REQ-320
#[test]
fn an_overlap_is_a_step_of_a_clip_whose_order_was_changed_and_drawn_over_its_own_time() {
    let dir = tempfile::tempdir().unwrap();
    let (mut p, id) = with_overlap(dir.path(), "logo.png", Kind::Still, (400, 200), false);
    let clip = p.timeline.clips[0].id;
    // the overlap goes below the Scale (a built order)
    let t = crate::timeline::FrameTarget::Clip(clip);
    p.timeline.add_scale(t).unwrap();
    p.set_order(clip, vec![Row::Scale, Row::Overlap(id)]);
    assert!(crate::effect_order::needs_steps(&p.order_of(clip)));
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("re-encode")
    };
    let g = &graph.graph;
    assert!(g.contains("scale=192:96[pi"), "{g}");
    // in the clip's own time: 2 s to 5 s, and no overlay after the concat
    assert!(
        g.contains("enable='between(t,2.000000,5.000000)':shortest=1"),
        "{g}"
    );
    assert_eq!(g.matches("scale=192:96").count(), 1, "{g}");
    // the overlay stage (a moved LUT or text) also draws it: the order with only drawn rows
    let (mut p, id) = with_overlap(dir.path(), "logo2.png", Kind::Still, (400, 200), false);
    let clip = p.timeline.clips[0].id;
    let tid = p.texts.new_id();
    let mut text = crate::text::TextItem::new(tid, "T".into(), s(1));
    text.ttl = s(3);
    p.texts.add(text).unwrap();
    // the overlap above the text is not the default order (the text is above it)
    p.set_order(clip, vec![Row::Overlap(id), Row::Text(tid.0)]);
    assert_eq!(p.changed_orders().len(), 1);
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("re-encode")
    };
    assert!(graph.graph.contains("scale=192:96"), "{}", graph.graph);
}

// covers: REQ-325
#[test]
fn a_project_without_overlaps_has_the_arguments_it_had_and_the_frame_key_tells_overlaps_apart() {
    let dir = tempfile::tempdir().unwrap();
    let plain = project(&[(8, 640, 360, 25)]);
    let (p, _) = with_overlap(dir.path(), "logo.png", Kind::Still, (400, 200), false);
    let export = Geometry {
        width: 640,
        height: 360,
        fps: Fps::new(25, 1).unwrap(),
    };
    // nothing to draw for the plain project
    assert!(
        picture_overlays(&plain, &geometry(), Micros::ZERO, Source::Export)
            .unwrap()
            .is_empty()
    );
    // the preview picture depends on the overlap while it shows, not before
    let before = frame_key(&p, s(1), Source::Export, &export).unwrap();
    let during = frame_key(&p, s(3), Source::Export, &export).unwrap();
    assert!(before.overlaps.is_empty() && during.overlaps.len() == 1);
    assert_ne!(before, during);
    let mut moved = p.clone();
    let id = moved.timeline.clip_overlaps[0].items[0].id;
    moved
        .timeline
        .change_overlap(id, |o| o.place = Pos { x: 100, y: 100 })
        .unwrap();
    assert_ne!(
        during,
        frame_key(&moved, s(3), Source::Export, &export).unwrap()
    );
}
