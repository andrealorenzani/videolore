// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 2.0.2 (verify the release): the paused preview at a fade, a format 8 project
//! with its colour tables, and the encoders and filters the bundled build must offer.

use super::*;

fn first_frame(pv: &mut Preview) -> videolore::preview::Frame {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(f) = pv.poll(Instant::now()) {
            return f;
        }
        assert!(pv.error.is_none(), "preview error: {:?}", pv.error);
        assert!(Instant::now() < deadline, "no preview frame");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn paused_at(env: &Env, p: &Project, at: Micros) -> f64 {
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    pv.seek(p, at, Instant::now()).unwrap();
    let f = first_frame(&mut pv);
    assert_eq!(f.pts, at);
    mean(
        &f.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|px| px[..3].to_vec())
            .collect::<Vec<u8>>(),
    )
}

// covers: REQ-144
#[test]
fn a_paused_frame_at_a_fade_is_not_black_and_gets_darker_towards_the_cut() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let a = env.clip("pf-a.mp4", 640, 360, 25, 2, true);
    let ma = env.probe(&mut p, &a);
    p.timeline.append(&ma).unwrap();
    p.timeline.append(&ma).unwrap();
    p.timeline.toggle_fade(0).unwrap();
    let in_clip = paused_at(&env, &p, Micros(1_000_000));
    let in_fade = paused_at(&env, &p, Micros(1_700_000));
    let near_cut = paused_at(&env, &p, Micros(1_920_000));
    assert!(in_clip > 60.0, "picture before the fade: {in_clip}");
    assert!(
        in_fade > 12.0,
        "a paused frame inside the fade is a picture: {in_fade}"
    );
    assert!(
        in_fade < in_clip,
        "dimmer than the clip: {in_fade} {in_clip}"
    );
    // a fade through black still shows its dark picture at its own time
    assert!(
        near_cut < in_fade && near_cut < in_clip / 2.0,
        "{near_cut} {in_fade}"
    );
}

// covers: REQ-137
#[test]
fn a_format_8_project_reads_its_luts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.vlproj");
    std::fs::write(&path, include_str!("../fixtures/project-v8-2.0.2.vlproj")).unwrap();
    let (p, notes) = project::load(&path).unwrap();
    let lut_of = |id: u64| {
        p.timeline
            .clip_luts
            .iter()
            .find(|l| l.clip.0 == id)
            .map(|l| l.path.as_str())
    };
    // the whole-video table goes to every clip; a clip's own table is kept
    assert_eq!(
        lut_of(1),
        Some("/videolore-fixture-does-not-exist/whole.cube")
    );
    assert_eq!(
        lut_of(2),
        Some("/videolore-fixture-does-not-exist/whole.cube")
    );
    assert_eq!(
        lut_of(3),
        Some("/videolore-fixture-does-not-exist/own.cube")
    );
    assert!(notes.iter().any(|n| n.contains("whole.cube")), "{notes:?}");
}

// covers: REQ-121
#[test]
fn the_build_offers_mp3_and_the_frame_lut_and_360_filters() {
    let Some(env) = setup() else { return };
    assert!(
        env.info.encoders.contains("libmp3lame"),
        "encoder libmp3lame is missing"
    );
    for f in [
        "lut3d",
        "v360",
        "scale",
        "pad",
        "overlay",
        "fade",
        "subtitles",
    ] {
        assert!(env.info.filters.contains(f), "filter {f} is missing");
    }
}
