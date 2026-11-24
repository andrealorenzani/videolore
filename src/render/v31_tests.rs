// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! 3.1.0 in the render: the audio track of a clip (REQ-340), the blurred fill of a clip row
//! (REQ-335) and the subtitle style "As subtitle track" (REQ-337).

use super::tests::{enc, project, strs};
use super::*;
use crate::format::{FormatChoice, SubtitleMode};
use crate::media::AudioTrack;
use crate::subtitles::Preset;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

fn reencode(p: &Project) -> (Graph, Plan) {
    let plan = plan_export(p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode { graph, .. } = &plan else {
        panic!("a re-encode")
    };
    (*graph.clone(), plan)
}

fn tracks(n: usize) -> Vec<AudioTrack> {
    (0..n)
        .map(|i| AudioTrack {
            ordinal: i as u8,
            language: None,
            title: None,
            channels: 2,
        })
        .collect()
}

fn args(plan: &Plan) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let paths = ExportPaths {
        output: &dir.path().join("o.mp4"),
        concat_list: &dir.path().join("l.txt"),
        graph_script: &dir.path().join("g.txt"),
    };
    strs(&export_args(plan, &paths, FfVersion::Release(6, 1, 0), None, None).unwrap())
}

// covers: REQ-340
#[test]
fn track_zero_reads_the_first_stream_as_always_and_another_track_reads_its_own() {
    let mut p = project(&[(4, 1280, 720, 30)]);
    p.media[0].format.audio_tracks = Some(tracks(3));
    // the first track: a graph that only exists because of a fade, with `[0:a:0]`
    p.timeline.toggle_fade(0).ok();
    p.timeline.set_edge_fades(true).unwrap();
    let (g, _) = reencode(&p);
    assert!(g.graph.contains("[0:a:0]"), "{}", g.graph);
    assert!(!g.graph.contains("[0:a:1]") && !g.graph.contains("[0:a:2]"));
    // the third track
    let id = p.timeline.clips[0].id;
    p.timeline.set_audio_track(id, 2).unwrap();
    let (g, _) = reencode(&p);
    assert!(g.graph.contains("[0:a:2]"), "{}", g.graph);
    assert!(!g.graph.contains("[0:a:0]"));
}

// covers: REQ-340
#[test]
fn two_parts_of_one_file_with_different_tracks_read_two_streams() {
    let mut p = project(&[(6, 1280, 720, 30)]);
    p.media[0].format.audio_tracks = Some(tracks(2));
    let a = p.timeline.clips[0].id;
    p.timeline.split(s(3), Micros(1)).unwrap();
    let b = p.timeline.clips[1].id;
    p.timeline.set_audio_track(b, 1).unwrap();
    assert_ne!(a, b);
    let (g, _) = reencode(&p);
    assert!(
        g.graph.contains("[0:a:0]") && g.graph.contains("[0:a:1]"),
        "{}",
        g.graph
    );
    // each part has its own audio chain
    assert_eq!(g.graph.matches("atrim=start").count(), 2);
}

// covers: REQ-340
#[test]
fn a_clip_with_another_track_is_never_stream_copied() {
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    p.media[0].format.audio_tracks = Some(tracks(2));
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    let id = p.timeline.clips[0].id;
    p.timeline.set_audio_track(id, 1).unwrap();
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::Reencode { .. }
    ));
}

// covers: REQ-340
#[test]
fn the_track_is_in_the_picture_key_and_the_proxy_keeps_every_audio_stream() {
    let mut p = project(&[(4, 1280, 720, 30)]);
    p.media[0].format.audio_tracks = Some(tracks(2));
    let eg = output_geometry(&p, &enc()).unwrap();
    let key = |p: &Project| frame_key(p, s(1), Source::Export, &eg).unwrap();
    let before = key(&p);
    let id = p.timeline.clips[0].id;
    p.timeline.set_audio_track(id, 1).unwrap();
    assert_ne!(before, key(&p));
}

// covers: REQ-335
#[test]
fn a_clips_own_blurred_aspect_has_the_blur_chain_and_a_colour_one_does_not() {
    use crate::frame::{Fill, Fit, FrameAspect, Rgb, Shape};
    let aspect = |fill| FrameAspect {
        shape: Shape::Tall9x16,
        fit: Fit::Width,
        fill,
        blur: Default::default(),
    };
    let mut p = project(&[(4, 1600, 1200, 30), (4, 1600, 1200, 30)]);
    let id = p.timeline.clips[0].id;
    p.timeline
        .set_clip_aspect(id, Some(aspect(Fill::Blur)))
        .unwrap();
    let (g, _) = reencode(&p);
    assert!(g.graph.contains("boxblur"), "{}", g.graph);
    // the picture is fitted to the width of the frame and the blur is made from it
    assert!(g.graph.contains("[pb0]scale="), "{}", g.graph);
    p.timeline
        .set_clip_aspect(id, Some(aspect(Fill::Colour(Rgb(0, 255, 0)))))
        .unwrap();
    let (g, _) = reencode(&p);
    assert!(
        !g.graph.contains("boxblur") && g.graph.contains("0x00FF00"),
        "{}",
        g.graph
    );
}

// covers: REQ-335
#[test]
fn the_kept_background_is_not_used_for_a_clips_own_frame() {
    use crate::frame::{Fill, Fit, FrameAspect, Shape};
    let aspect = FrameAspect {
        shape: Shape::Tall9x16,
        fit: Fit::Width,
        fill: Fill::Blur,
        blur: Default::default(),
    };
    let mut p = project(&[(4, 1600, 1200, 30)]);
    let eg = output_geometry(&p, &enc()).unwrap();
    let id = p.timeline.clips[0].id;
    p.timeline.set_frame_aspect(Some(aspect)).unwrap();
    let key = frame_key(&p, s(1), Source::Export, &eg).unwrap();
    assert!(key.background_name().is_some(), "the whole video's is kept");
    p.timeline.set_frame_aspect(None).unwrap();
    p.timeline.set_clip_aspect(id, Some(aspect)).unwrap();
    let key = frame_key(&p, s(1), Source::Export, &eg).unwrap();
    assert!(key.own_aspect && key.background_name().is_none());
}

fn with_line(preset: Preset) -> Project {
    let mut p = project(&[(4, 1280, 720, 30)]);
    let tl = p.timeline.clone();
    for (t, text) in [(0, "one"), (1, "two"), (2, "three")] {
        p.subtitles.add_at(&tl, s(t), text, Micros(1)).unwrap();
    }
    p.subtitles.language = Some("it".into());
    p.subtitles.preset = preset;
    p
}

fn plan_of(p: &Project, mode: SubtitleMode, muxer_mkv: bool) -> Plan {
    let mut q = p.clone();
    if muxer_mkv {
        q.media[0].format.container = "matroska".into();
    }
    plan_export_choice(
        &q,
        &enc(),
        ExportMode::Automatic,
        &FormatChoice {
            subtitles: mode,
            ..Default::default()
        },
        |_, _| true,
    )
    .unwrap()
}

// covers: REQ-337
#[test]
fn as_subtitle_track_writes_the_lines_as_a_track_and_draws_nothing() {
    let p = with_line(Preset::AsTrack);
    // whatever the dialog says, the lines are a separate track
    for mode in [SubtitleMode::Burned, SubtitleMode::Separate] {
        let plan = plan_of(&p, mode, false);
        let a = args(&plan);
        assert!(plan.subtitles().is_none(), "no burned lines");
        assert!(!a.iter().any(|x| x.contains("subtitles=f=subs.ass")));
        let t = plan.track().unwrap();
        assert_eq!((t.codec, t.language.as_str()), ("mov_text", "ita"));
        assert_eq!(t.content.matches("-->").count(), 3);
        assert!(a.windows(2).any(|w| w == ["-c:s", "mov_text"]));
        assert!(a.windows(2).any(|w| w == ["-disposition:s:0", "0"]));
    }
    // an MKV file takes the same lines as SRT
    let plan = plan_of(&p, SubtitleMode::Burned, true);
    assert_eq!(plan.track().unwrap().codec, "srt");
    assert_eq!(plan.track().unwrap().content.matches("-->").count(), 3);
}

// covers: REQ-337
#[test]
fn the_other_styles_are_burned_as_before() {
    for preset in [Preset::Classic, Preset::Boxed, Preset::WordHighlight] {
        let p = with_line(preset);
        let plan = plan_of(&p, SubtitleMode::Burned, false);
        assert!(plan.track().is_none());
        assert!(plan.subtitles().unwrap().contains("one"));
        assert!(
            args(&plan)
                .iter()
                .any(|x| x.contains("subtitles=f=subs.ass"))
        );
    }
    // a texts-only project with the style burns the texts, not the lines
    use crate::text::TextItem;
    let mut p = with_line(Preset::AsTrack);
    let id = p.texts.new_id();
    p.texts
        .add(TextItem::new(id, "Title".into(), s(2)))
        .unwrap();
    let plan = plan_of(&p, SubtitleMode::Burned, false);
    let doc = plan.subtitles().unwrap();
    assert!(doc.contains("Style: Text,") && !doc.contains("Dialogue: 0"));
    assert!(plan.track().is_some());
}

// covers: REQ-337
#[test]
fn a_360_export_writes_the_track_the_same_way() {
    let mut p = with_line(Preset::AsTrack);
    p.media[0].format.spherical = Some(true);
    p.media[0].format.width = 3840;
    p.media[0].format.height = 1920;
    let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
    let t = plan.track().expect("the track of a 360 export");
    assert_eq!(t.content.matches("-->").count(), 3);
    assert!(plan.subtitles().is_none());
}

// covers: REQ-337
#[test]
fn the_export_dialog_fixes_the_subtitle_choice_for_the_style() {
    use crate::export::choice::{
        AS_TRACK_REASON, Context, ExportChoice, Mode, Request, defaults, request, subtitle_mode,
    };
    let ctx = |as_track| Context {
        has_clips: true,
        has_subtitles: true,
        subtitles_as_track: as_track,
        encoders: enc(),
        auto_extension: "mp4".into(),
        ..Default::default()
    };
    let choice: ExportChoice = defaults(None, None, None);
    assert_eq!(choice.subtitles, SubtitleMode::Burned);
    assert_eq!(subtitle_mode(&choice, &ctx(false)), SubtitleMode::Burned);
    assert_eq!(subtitle_mode(&choice, &ctx(true)), SubtitleMode::Separate);
    assert!(AS_TRACK_REASON.contains("As subtitle track"));
    let Request::Video { format, .. } = request(&choice, &ctx(true)) else {
        panic!("a video")
    };
    assert_eq!(format.subtitles, SubtitleMode::Separate);
    assert_eq!(choice.mode, Mode::Full);
}
