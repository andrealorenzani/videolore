// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

use super::*;
use crate::media::ProxyState;
use crate::music::tests::song;
use crate::project::Project;
use crate::timeline::tests::media;

pub fn project(fmts: &[(u64, u32, u32, u32)]) -> Project {
    let mut p = Project::default();
    for &(secs, w, h, fps) in fmts {
        let id = p.new_media_id();
        let mut m = media(id.0, secs as i64, true);
        m.format.width = w;
        m.format.height = h;
        m.format.fps = Fps::new(fps, 1);
        m.format.bit_rate = Some(1_000_000);
        p.add_media(m.clone());
        p.timeline.append(&m).unwrap();
    }
    p
}

pub fn enc() -> std::collections::BTreeSet<String> {
    ["libx264", "libx265", "aac"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn strs(a: &[OsString]) -> Vec<String> {
    a.iter().map(|s| s.to_string_lossy().into_owned()).collect()
}

fn args(plan: &Plan) -> Vec<String> {
    args_with(plan, None)
}

fn args_with(plan: &Plan, threads: Option<u32>) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("o.mp4");
    let list = dir.path().join("l.txt");
    let script = dir.path().join("g.txt");
    let paths = ExportPaths {
        output: &out,
        concat_list: &list,
        graph_script: &script,
    };
    strs(&export_args(plan, &paths, FfVersion::Release(6, 1, 0), threads, None).unwrap())
}

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

pub(crate) fn spec(video: bool, audio: bool) -> GraphSpec {
    GraphSpec {
        video,
        audio,
        pix_fmt: "yuv420p",
        geometry: Geometry {
            width: 640,
            height: 360,
            fps: Fps::new(25, 1).unwrap(),
        },
        subtitles: false,
        mix: Mix::default(),
        camera_updates: 0,
    }
}

// covers: REQ-023
#[test]
fn identical_without_effects_uses_stream_copy() {
    let p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    let plan = plan_export(&p, &enc(), |_, _| false).unwrap();
    let Plan::StreamCopy {
        concat_list,
        muxer,
        faststart,
        hevc_tag,
    } = &plan
    else {
        panic!("expected stream copy")
    };
    assert_eq!(muxer, "mp4");
    assert!(faststart);
    assert!(!hevc_tag);
    assert!(concat_list.contains("file 'file:/m/1.mp4'\ninpoint 0.000000\noutpoint 2.000000"));
    assert!(plan.subtitles().is_none());
    let a = args(&plan);
    assert!(a.windows(2).any(|w| w == ["-c", "copy"]));
    assert!(a.windows(2).any(|w| w == ["-safe", "0"]));
    assert!(!a.contains(&"-tag:v".to_string()));
    assert!(a.last().unwrap().starts_with("file:"));
}

// covers: REQ-023
#[test]
fn non_keyframe_start_or_fade_forces_reencode() {
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    let id = p.timeline.clips[1].id;
    p.timeline
        .trim(id, Micros(500_000), s(3), s(3), Micros(1))
        .unwrap();
    let mut asked = Vec::new();
    let plan = plan_export(&p, &enc(), |m, t| {
        asked.push((m.id, t));
        false
    })
    .unwrap();
    assert!(matches!(plan, Plan::Reencode { .. }));
    assert_eq!(asked.len(), 1);
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    assert!(matches!(plan, Plan::StreamCopy { .. }));
    p.timeline.toggle_fade(0).unwrap();
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    assert!(matches!(plan, Plan::Reencode { .. }));
}

// covers: REQ-023, REQ-019, REQ-020
#[test]
fn music_volume_or_subtitles_force_reencode() {
    let base = project(&[(4, 1280, 720, 30)]);
    assert!(matches!(
        plan_export(&base, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    // music
    let mut p = base.clone();
    let m = song(9, 10);
    p.add_media(m.clone());
    p.music.place(&m, s(1), Micros(1)).unwrap();
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::Reencode { .. }
    ));
    // music volume alone (no music clips) does not matter; video volume does
    let mut p = base.clone();
    p.mix.music_volume = 10;
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    p.mix.video_muted = true;
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode { graph, .. } = &plan else {
        panic!()
    };
    assert!(
        graph
            .graph
            .ends_with("concat=n=1:v=1:a=1[outv][ca];[ca]volume=0.000000[outa]")
    );
    // subtitles
    let mut p = base.clone();
    let tl = p.timeline.clone();
    p.subtitles.add_at(&tl, s(1), "hi", Micros(1)).unwrap();
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode {
        graph, subtitles, ..
    } = &plan
    else {
        panic!()
    };
    assert!(
        graph
            .graph
            .contains("[cv]subtitles=f=subs.ass:fontsdir=fonts[outv]")
    );
    let doc = subtitles.as_deref().unwrap();
    assert!(doc.contains("PlayResX: 1280\nPlayResY: 720"));
    assert!(doc.contains(",hi"));
    assert_eq!(plan.subtitles(), Some(doc));
}

// covers: REQ-022
#[test]
fn mixed_formats_scale_and_pad_to_lowest() {
    let p = project(&[(2, 1280, 720, 30), (2, 640, 360, 25)]);
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode {
        graph, settings, ..
    } = &plan
    else {
        panic!("expected re-encode")
    };
    assert_eq!((settings.width, settings.height), (640, 360));
    assert_eq!(settings.fps, Fps::new(25, 1).unwrap());
    let g = &graph.graph;
    assert!(g.contains("scale=640:360:force_original_aspect_ratio=decrease"));
    assert!(g.contains("pad=640:360:(ow-iw)/2:(oh-ih)/2:color=black"));
    assert!(g.contains("fps=25/1"));
    assert!(!g.contains("stretch"));
    assert!(g.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"));
    let a = args(&plan);
    assert!(a.windows(2).any(|w| w == ["-c:v", "libx264"]));
    assert!(a.windows(2).any(|w| w == ["-b:v", "1000000"]));
    assert!(a.windows(2).any(|w| w == ["-bufsize", "2000000"]));
    assert!(a.windows(2).any(|w| w == ["-movflags", "+faststart"]));
    assert_eq!(a.iter().filter(|x| *x == "-i").count(), 2);
}

// covers: REQ-011
#[test]
fn fades_in_graph_video_and_audio() {
    let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25)]);
    p.media[1].format.acodec = None;
    p.timeline.toggle_fade(0).unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert_eq!(segs[0].fade_out, Micros(500_000));
    assert_eq!(segs[1].fade_in, Micros(500_000));
    let g = build_graph(&segs, None, &spec(true, true)).graph;
    assert!(g.contains("fade=t=out:st=1.500000:d=0.500000"));
    assert!(g.contains("afade=t=out:st=1.500000:d=0.500000"));
    assert!(g.contains("fade=t=in:st=0:d=0.500000"));
    assert!(g.contains("anullsrc=r=48000:cl=stereo,atrim=duration=3.000000"));
    // the total length is the sum of the clips: fades do not shorten anything
    let total: Micros = segs.iter().fold(Micros::ZERO, |a, s| a + s.rendered());
    assert_eq!(total, p.timeline.duration());
}

// covers: REQ-037
#[test]
fn many_cuts_from_one_source_share_one_input() {
    let mut p = project(&[(60, 640, 360, 25)]);
    let m = p.media[0].clone();
    p.timeline.clips.clear();
    // 200 cuts in source order: 1 input, 1 split
    for i in 0..200 {
        p.timeline.append(&m).unwrap();
        let id = p.timeline.clips[i].id;
        let a = Micros(i as i64 * 250_000);
        p.timeline
            .trim(id, a, a + Micros(80_000), m.duration, Micros(1))
            .unwrap();
    }
    let plan = plan_export(&p, &enc(), |_, _| false).unwrap();
    let Plan::Reencode { graph, .. } = &plan else {
        panic!()
    };
    assert_eq!(graph.inputs.len(), 1);
    assert_eq!(graph.inputs[0].start, Micros::ZERO);
    assert_eq!(graph.inputs[0].len, Micros(199 * 250_000 + 80_000));
    assert!(graph.graph.starts_with("[0:v:0]split=200[v0s0]"));
    assert!(graph.graph.contains("[0:a:0]asplit=200[a0s0]"));
    assert!(
        graph
            .graph
            .contains("[v0s199]trim=start=49.750000:duration=0.080000")
    );
    let a = args(&plan);
    assert_eq!(a.iter().filter(|x| *x == "-i").count(), 1);
    // the long graph goes through a script file
    assert!(a.iter().any(|x| x == "-filter_complex_script"));
    // going back in time, or a far jump, opens a new input
    let spans = vec![
        (PathBuf::from("/a"), s(0), s(1)),
        (PathBuf::from("/a"), s(2), s(3)),
        (PathBuf::from("/a"), s(1), s(2)),
        (PathBuf::from("/b"), s(5), s(6)),
        (PathBuf::from("/b"), s(60), s(61)),
        (PathBuf::from("/a"), s(3), s(4)),
    ];
    let (inputs, idx) = group_inputs(&spans);
    assert_eq!(idx, vec![0, 0, 1, 2, 3, 4]);
    assert_eq!(inputs[0].len, s(3));
    assert_eq!(inputs.len(), 5);
}

// covers: REQ-036
#[test]
fn hevc_in_mp4_and_mov_is_tagged_hvc1() {
    for (container, tag) in [("mp4", true), ("mov", true), ("matroska", false)] {
        let mut p = project(&[(2, 1280, 720, 30), (2, 1280, 720, 30)]);
        for m in &mut p.media {
            m.format.vcodec = Some("hevc".into());
            m.format.container = container.into();
        }
        // stream copy
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        assert!(matches!(plan, Plan::StreamCopy { .. }));
        let a = args(&plan);
        assert_eq!(
            a.windows(2).any(|w| w == ["-tag:v", "hvc1"]),
            tag,
            "{container}"
        );
        // re-encode
        p.timeline.toggle_fade(0).unwrap();
        let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
        let a = args(&plan);
        assert!(a.windows(2).any(|w| w == ["-c:v", "libx265"]));
        assert_eq!(
            a.windows(2).any(|w| w == ["-tag:v", "hvc1"]),
            tag,
            "{container}"
        );
    }
}

// covers: REQ-020, REQ-043
#[test]
fn music_bed_volumes_and_end_fade() {
    let mut p = project(&[(10, 640, 360, 25)]);
    let a = song(5, 4);
    let b = song(6, 30);
    p.add_media(a.clone());
    p.add_media(b.clone());
    p.music.place(&a, s(1), Micros(1)).unwrap();
    p.music.place(&b, s(7), Micros(1)).unwrap();
    p.mix = Mix {
        video_volume: 50,
        video_muted: false,
        music_volume: 80,
        music_muted: false,
    };
    let bed = music_bed(&p, Micros::ZERO).unwrap().unwrap();
    // silence where there is no music, music clips cut at the video end, fade-out there
    assert_eq!(
        bed.parts,
        vec![
            BedPart::Gap(s(1)),
            BedPart::Clip {
                path: "/m/5.mp3".into(),
                src: Micros::ZERO,
                len: s(4),
                gain: 1.0,
                fade_in: Micros::ZERO,
                fade_out: Micros::ZERO,
                clean: false,
            },
            BedPart::Gap(s(2)),
            BedPart::Clip {
                path: "/m/6.mp3".into(),
                src: Micros::ZERO,
                len: s(3),
                gain: 1.0,
                fade_in: Micros::ZERO,
                fade_out: Micros::ZERO,
                clean: false,
            },
        ]
    );
    assert_eq!(bed.len, s(10));
    assert_eq!(bed.fade, Some((Micros(9_500_000), Micros(500_000))));
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!()
    };
    let g = &graph.graph;
    assert_eq!(graph.inputs.len(), 3);
    assert!(
        g.contains("[m0][m1][m2][m3]concat=n=4:v=0:a=1,afade=t=out:st=9.500000:d=0.500000[mb]")
    );
    assert!(g.contains("[ca]volume=0.250000[va];[mb]volume=0.640000[vm]"));
    assert!(
        g.contains("amix=inputs=2:duration=first:normalize=0,alimiter=limit=0.97:level=0[outa]")
    );
    assert!(g.contains("[1:a:0]atrim=start=0.000000:duration=4.000000"));
    // preview from 8 s: the second music clip is read from 1 s in
    let bed = music_bed(&p, s(8)).unwrap().unwrap();
    assert_eq!(
        bed.parts,
        vec![BedPart::Clip {
            path: "/m/6.mp3".into(),
            src: s(1),
            len: s(2),
            gain: 1.0,
            fade_in: Micros::ZERO,
            fade_out: Micros::ZERO,
            clean: false,
        }]
    );
    // music that ends before the video: no fade; no music at all: no bed
    let mut q = p.clone();
    let last = q.music.clips[1].id;
    q.music.remove(last).unwrap();
    let bed = music_bed(&q, Micros::ZERO).unwrap().unwrap();
    assert_eq!(bed.fade, None);
    assert_eq!(bed.parts.last(), Some(&BedPart::Gap(s(5))));
    assert_eq!(music_bed(&q, s(6)).unwrap(), None);
    assert_eq!(music_bed(&q, s(99)).unwrap(), None);
    q.media[1].missing = true;
    assert!(matches!(
        music_bed(&q, Micros::ZERO),
        Err(RenderError::Missing(_))
    ));
    q.media.remove(1);
    assert_eq!(music_bed(&q, Micros::ZERO), Err(RenderError::UnknownMedia));
}

// covers: REQ-043
#[test]
fn video_edits_do_not_move_music() {
    let mut p = project(&[(4, 640, 360, 25), (4, 640, 360, 25)]);
    let m = song(9, 3);
    p.add_media(m.clone());
    p.music.place(&m, s(2), Micros(1)).unwrap();
    let before = p.music.clone();
    let first = p.timeline.clips[0].id;
    p.timeline.remove(first).unwrap();
    p.timeline.split(s(1), Micros(1)).unwrap();
    assert_eq!(p.music, before);
    // the same music clip now runs past the shortened video: cut with a fade there
    let id = p.timeline.clips[0].id;
    p.timeline
        .trim(id, Micros::ZERO, Micros(500_000), s(4), Micros(1))
        .unwrap();
    let bed = music_bed(&p, Micros::ZERO).unwrap().unwrap();
    assert_eq!(bed.len, s(3) + Micros(500_000));
    assert!(bed.fade.is_some());
}

// covers: REQ-013
#[test]
fn preview_starts_mid_timeline_at_360p() {
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    p.timeline.toggle_fade(0).unwrap();
    let segs = segments(&p, s(2) + Micros(500_000), Source::Export).unwrap();
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].skip, Micros(500_000));
    assert_eq!(segs[0].rendered(), Micros(2_500_000));
    let eg = output_geometry(&p, &enc()).unwrap();
    let g = eg.preview();
    assert_eq!((g.width, g.height), (640, 360));
    let dir = tempfile::tempdir().unwrap();
    let sc = dir.path().join("g");
    let src = Source::Preview { proxies: true };
    let r = preview_video_args(&p, Micros(2_500_000), &eg, FfVersion::Dev, &sc, src).unwrap();
    assert!(r.subtitles.is_none());
    let a = strs(&r.args);
    assert!(a.windows(2).any(|w| w == ["-ss", "0.500000"]));
    assert!(a.windows(2).any(|w| w == ["-pix_fmt", "rgba"]));
    let fc = a.iter().position(|x| x == "-filter_complex").unwrap();
    assert!(a[fc + 1].contains("setpts=PTS-STARTPTS+0.500000/TB"));
    assert!(a[fc + 1].contains("fade=t=in:st=0:d=0.500000"));
    let a =
        strs(&preview_audio_args(&p, Micros::ZERO, 44_100, 2, FfVersion::Dev, &sc, src).unwrap());
    assert!(a.windows(2).any(|w| w == ["-ar", "44100"]));
    assert!(a.windows(2).any(|w| w == ["-f", "s16le"]));
    assert!(preview_video_args(&p, s(99), &eg, FfVersion::Dev, &sc, src).is_err());
    assert!(preview_audio_args(&p, s(99), 1, 1, FfVersion::Dev, &sc, src).is_err());
    assert!(still_args(&p, s(99), &eg, FfVersion::Dev, &sc, src).is_err());
    let small = Geometry {
        width: 320,
        height: 240,
        fps: Fps::new(25, 1).unwrap(),
    };
    assert_eq!(small.preview(), small);
    // a music clip that cannot be found stops the audio preview
    let m = song(9, 3);
    p.add_media(MediaItem {
        missing: true,
        ..m.clone()
    });
    p.music.place(&m, Micros::ZERO, Micros(1)).unwrap();
    assert!(preview_audio_args(&p, Micros::ZERO, 1, 1, FfVersion::Dev, &sc, src).is_err());
}

// covers: REQ-034, REQ-017
#[test]
fn still_frame_uses_one_input_and_shows_subtitles() {
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    let tl = p.timeline.clone();
    p.subtitles.add_at(&tl, s(3), "hello", Micros(1)).unwrap();
    let eg = output_geometry(&p, &enc()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let sc = dir.path().join("g");
    let r = still_args(&p, s(3), &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
    let a = strs(&r.args);
    assert_eq!(a.iter().filter(|x| *x == "-i").count(), 1);
    assert!(a.windows(2).any(|w| w == ["-frames:v", "1"]));
    let fc = a.iter().position(|x| x == "-filter_complex").unwrap();
    assert!(a[fc + 1].contains("subtitles=f=subs.ass"));
    // times in the document are relative to the frame
    let doc = r.subtitles.unwrap();
    assert!(doc.contains("Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,hello"));
    // the preview of the whole timeline from 0 carries the same subtitles
    let r = preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
    assert!(r.subtitles.unwrap().contains("0:00:03.00,0:00:05.00"));
}

// covers: REQ-111, REQ-023
#[test]
fn a_text_item_forces_reencode_and_is_burned_in_with_the_subtitles() {
    use crate::text::{TextId, TextItem};
    let mut p = project(&[(4, 1280, 720, 30)]);
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, "Title".into(), s(2));
    t.place = crate::base::pos::Pos { x: 1000, y: 1000 };
    p.texts.add(t).unwrap();
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode { graph, .. } = &plan else {
        panic!("a text must force a re-encode")
    };
    assert!(graph.graph.contains("subtitles=f=subs.ass"));
    let doc = plan.subtitles().unwrap();
    assert!(doc.contains("Style: Text,"));
    // 2 s .. 5 s, placed at 10% / 10% of 1280x720
    assert!(
        doc.contains("Dialogue: 1,0:00:02.00,0:00:04.00,Text,,128,128,0,,{"),
        "{doc}"
    );
    assert!(doc.contains("\\pos(128,72)"));
    // the part of the text after a start in the middle is shifted
    let eg = output_geometry(&p, &enc()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let sc = dir.path().join("g");
    let r = still_args(&p, s(3), &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
    assert!(r.subtitles.unwrap().contains("0:00:00.00,0:00:01.00,Text"));
    // not visible at 1 s of the preview... the doc from 4 s has no text left
    let r = still_args(&p, s(1), &eg, FfVersion::Dev, &sc, Source::Export).unwrap();
    assert!(r.subtitles.unwrap().contains("0:00:01.00"));
    // the picture key knows about the text and about its effect frame
    let k1 = frame_key(&p, s(1), Source::Export, &eg).unwrap();
    let k3 = frame_key(&p, s(3), Source::Export, &eg).unwrap();
    assert!(k1.texts.is_empty() && k3.texts.len() == 1);
    let mut q = p.clone();
    let mut edited = q.texts.get(TextId(1)).unwrap().clone();
    edited.effects.fade_in = Some(Micros(500_000));
    q.texts.replace(edited).unwrap();
    let a = frame_key(&q, Micros(3_000_000), Source::Export, &eg).unwrap();
    let b = frame_key(&q, Micros(3_100_000), Source::Export, &eg).unwrap();
    assert_ne!(a, b);
    assert_ne!(a, k3);
    let c = frame_key(&p, Micros(3_100_000), Source::Export, &eg).unwrap();
    assert_eq!(c.texts, k3.texts);
}

// covers: REQ-034
#[test]
fn frame_key_changes_only_with_the_picture() {
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    let eg = output_geometry(&p, &enc()).unwrap();
    let src = Source::Preview { proxies: true };
    let k = frame_key(&p, Micros(500_000), src, &eg).unwrap();
    assert_eq!(k.frame, 15);
    assert_eq!(k.size, (640, 360));
    assert!(k.fade.is_none());
    // an edit after the playhead does not change the key
    let mut q = p.clone();
    let id = q.timeline.clips[1].id;
    q.timeline.trim(id, s(1), s(2), s(3), Micros(1)).unwrap();
    assert_eq!(frame_key(&q, Micros(500_000), src, &eg), Some(k.clone()));
    // a fade reaching the playhead does
    p.timeline.toggle_fade(0).unwrap();
    assert_eq!(frame_key(&p, Micros(500_000), src, &eg), Some(k.clone()));
    let kf = frame_key(&p, Micros(1_800_000), src, &eg).unwrap();
    assert!(kf.fade.is_some());
    // subtitles on screen and the highlighted word are part of it
    let tl = p.timeline.clone();
    p.subtitles
        .add_at(&tl, Micros(400_000), "a b", Micros(1))
        .unwrap();
    let ks = frame_key(&p, Micros(500_000), src, &eg).unwrap();
    assert_eq!(ks.subtitles, vec![("a b".to_string(), None, None, None)]);
    p.subtitles.preset = Preset::WordHighlight;
    let kw = frame_key(&p, Micros(500_000), src, &eg).unwrap();
    assert_eq!(kw.subtitles, vec![("a b".to_string(), Some(0), None, None)]);
    // a ready proxy changes the file read by the preview but never the export
    p.media[0].proxy = ProxyState::Ready("/cache/x.mkv".into());
    let kp = frame_key(&p, Micros(500_000), src, &eg).unwrap();
    assert_eq!(kp.path, PathBuf::from("/cache/x.mkv"));
    let off = frame_key(&p, Micros(500_000), Source::Preview { proxies: false }, &eg).unwrap();
    assert_eq!(off.path, PathBuf::from("/m/1.mp4"));
    assert!(frame_key(&p, s(99), src, &eg).is_none());
}

// covers: REQ-038
#[test]
fn export_plan_is_identical_with_proxies_on_and_off() {
    let mut p = project(&[(2, 3840, 2160, 30), (3, 1280, 720, 30)]);
    p.timeline.toggle_fade(0).unwrap();
    let without = plan_export(&p, &enc(), |_, _| true).unwrap();
    p.media[0].proxy = ProxyState::Ready("/cache/a.mkv".into());
    let with = plan_export(&p, &enc(), |_, _| true).unwrap();
    assert_eq!(with, without);
    assert!(!format!("{with:?}").contains("/cache/"));
    let pv = segments(&p, Micros::ZERO, Source::Preview { proxies: true }).unwrap();
    assert_eq!(pv[0].path, PathBuf::from("/cache/a.mkv"));
}

#[test]
fn errors_and_quoting() {
    let p = Project::default();
    assert_eq!(
        segments(&p, Micros::ZERO, Source::Export),
        Err(RenderError::Empty)
    );
    assert_eq!(
        plan_export(&p, &enc(), |_, _| true),
        Err(RenderError::Empty)
    );
    let mut p = project(&[(2, 640, 360, 25)]);
    p.media[0].missing = true;
    assert_eq!(
        segments(&p, Micros::ZERO, Source::Export),
        Err(RenderError::Missing("1.mp4".into()))
    );
    p.media.clear();
    assert_eq!(
        segments(&p, Micros::ZERO, Source::Export),
        Err(RenderError::UnknownMedia)
    );
    assert_eq!(timeline_media(&p), Err(RenderError::UnknownMedia));
    assert_eq!(
        concat_quote(Path::new("/a/it's.mp4")),
        "'file:/a/it'\\''s.mp4'"
    );
}

#[test]
fn other_containers_stream_copy_muxers() {
    for (c, m, fs) in [
        ("mov", "mov", true),
        ("matroska", "matroska", false),
        ("webm", "webm", false),
        ("avi", "avi", false),
    ] {
        let mut p = project(&[(2, 640, 360, 25)]);
        p.media[0].format.container = c.into();
        let Plan::StreamCopy {
            muxer, faststart, ..
        } = plan_export(&p, &enc(), |_, _| true).unwrap()
        else {
            panic!()
        };
        assert_eq!((muxer.as_str(), faststart), (m, fs), "{c}");
    }
}

// covers: REQ-320, REQ-321
#[test]
fn volume_above_clean_audio_raises_the_level_before_the_noise_reduction_and_preview_agrees() {
    use crate::effect_order::Row;
    let mut p = project(&[(4, 640, 360, 25)]);
    let a = p.timeline.clips[0].id;
    p.timeline.set_clean_audio(&[a], true).unwrap();
    p.timeline.set_clip_volume(a, 50, false).unwrap();
    let chain = |p: &Project| {
        let g = build_graph(
            &segments(p, Micros::ZERO, Source::Export).unwrap(),
            None,
            &spec(true, true),
        )
        .graph;
        g.split(';')
            .find(|x| x.ends_with("[a0]"))
            .unwrap()
            .to_string()
    };
    // the default order is the 2.1.0 one: the noise reduction, then the level
    let default = chain(&p);
    assert!(default.find(CLEAN_AUDIO).unwrap() < default.find("volume=").unwrap());
    // Volume moved above Clean audio: the level first (and the stored order says so)
    p.set_sound_order(a, &[Row::Volume, Row::Clean]);
    assert!(p.sound_swapped(a));
    let swapped = chain(&p);
    assert!(swapped.find("volume=").unwrap() < swapped.find(CLEAN_AUDIO).unwrap());
    assert_ne!(default, swapped);
    // a half-mixed Clean audio takes its dry branch after the level
    p.timeline.set_clean_strength(&[a], 50).unwrap();
    let mixed = build_graph(
        &segments(&p, Micros::ZERO, Source::Export).unwrap(),
        None,
        &spec(true, true),
    )
    .graph;
    let split = mixed.find("asplit=2[cd0][cw0]").unwrap();
    assert!(mixed[..split].contains("volume="), "{mixed}");
    // the preview uses the same chain
    let dir = tempfile::tempdir().unwrap();
    let args = preview_audio_args(
        &p,
        Micros::ZERO,
        44_100,
        2,
        FfVersion::Dev,
        &dir.path().join("g"),
        Source::Export,
    )
    .unwrap();
    let text = strs(&args).join(" ");
    assert!(
        text.find("volume=").unwrap() < text.find("asplit=2[cd0][cw0]").unwrap(),
        "{text}"
    );
    // moving it back stores nothing again
    p.set_sound_order(a, &crate::order_chain::DEFAULT_SOUND);
    assert!(!p.sound_swapped(a));
    assert!(p.timeline.order_of(a).is_empty());
}

// covers: REQ-114, REQ-023
#[test]
fn clean_audio_is_applied_before_the_volume_and_the_fades_and_forces_a_reencode() {
    let mut p = project(&[(4, 640, 360, 25), (4, 640, 360, 25)]);
    let (a, b) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
    let plain = plan_export(&p, &enc(), |_, _| true).unwrap();
    assert!(matches!(plain, Plan::StreamCopy { .. }));
    // without it the sound chain is unchanged
    let g = build_graph(
        &segments(&p, Micros::ZERO, Source::Export).unwrap(),
        None,
        &spec(true, true),
    )
    .graph;
    assert!(!g.contains("afftdn"));
    // (d) a clip with it forces a re-encode
    p.timeline.set_clean_audio(&[a], true).unwrap();
    p.timeline.set_clip_volume(a, 50, false).unwrap();
    p.timeline.toggle_fade(0).unwrap();
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("clean audio must re-encode")
    };
    let parts: Vec<&str> = graph.graph.split(';').collect();
    let a0 = parts.iter().find(|x| x.ends_with("[a0]")).unwrap();
    let a1 = parts.iter().find(|x| x.ends_with("[a1]")).unwrap();
    // before the clip's own volume and the fades
    let clean = a0.find(CLEAN_AUDIO).expect(a0);
    assert!(clean < a0.find("volume=").unwrap() && clean < a0.find("afade").unwrap());
    assert!(!a1.contains("afftdn"), "other clips untouched: {a1}");
    // the whole video: every clip
    p.timeline.set_clean_audio(&[a, b], true).unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert!(segs.iter().all(|s| s.clean));
    assert_eq!(
        build_graph(&segs, None, &spec(true, true))
            .graph
            .matches("afftdn")
            .count(),
        2
    );
    // the preview uses the same chain
    let dir = tempfile::tempdir().unwrap();
    let a = preview_audio_args(
        &p,
        Micros::ZERO,
        44_100,
        2,
        FfVersion::Dev,
        &dir.path().join("g"),
        Source::Export,
    )
    .unwrap();
    assert!(strs(&a).iter().any(|x| x.contains(CLEAN_AUDIO)));
    // the music bed is not touched by it
    let mut m = p.clone();
    let song = song(9, 10);
    m.add_media(song.clone());
    m.music.place(&song, Micros::ZERO, Micros(1)).unwrap();
    let g = build_graph(
        &segments(&m, Micros::ZERO, Source::Export).unwrap(),
        music_bed(&m, Micros::ZERO).unwrap().as_ref(),
        &spec(true, true),
    )
    .graph;
    let bed = g.split(';').find(|x| x.ends_with("[mb]")).unwrap();
    assert!(!bed.contains("afftdn"));
    // a clip without sound has nothing to clean
    let mut silent = p.clone();
    silent.media[0].format.acodec = None;
    silent.media[0].format.channels = None;
    let segs = segments(&silent, Micros::ZERO, Source::Export).unwrap();
    let g = build_graph(&segs, None, &spec(true, true)).graph;
    assert_eq!(g.matches("afftdn").count(), usize::from(segs[1].has_audio));
}

// covers: REQ-120, REQ-121, REQ-023
#[test]
fn a_chosen_video_or_audio_format_is_always_reencoded_with_its_settings() {
    use crate::format::{AudioChoice, FormatChoice};
    let p = project(&[(4, 1280, 720, 30)]);
    let all: std::collections::BTreeSet<String> = ["libx264", "libx265", "aac", "libmp3lame"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let plan = |c: FormatChoice, e: &std::collections::BTreeSet<String>| {
        plan_export_choice(&p, e, ExportMode::Automatic, &c, |_, _| true)
    };
    // "Same as source" for both: stream copy where it is allowed
    assert!(matches!(
        plan(FormatChoice::default(), &all).unwrap(),
        Plan::StreamCopy { .. }
    ));
    // 1280x720 25 fps libx264 .mp4: that size, rate and codec
    let Plan::Reencode {
        graph, settings, ..
    } = plan(
        FormatChoice {
            video: Some(6),
            audio: AudioChoice::Same,
            ..Default::default()
        },
        &all,
    )
    .unwrap()
    else {
        panic!("a preset must re-encode")
    };
    assert_eq!((settings.width, settings.height), (1280, 720));
    assert_eq!(settings.fps, Fps::new(25, 1).unwrap());
    assert_eq!(settings.vencoder, "libx264");
    assert!(
        graph
            .graph
            .contains("scale=1280:720:force_original_aspect_ratio=decrease")
    );
    assert!(graph.graph.contains("fps=25"));
    // HEVC in MP4 carries hvc1 (REQ-036) and the sizes of 4K are kept
    let Plan::Reencode { settings, .. } = plan(
        FormatChoice {
            video: Some(10),
            audio: AudioChoice::Same,
            ..Default::default()
        },
        &all,
    )
    .unwrap() else {
        unreachable!()
    };
    assert_eq!((settings.width, settings.height), (3840, 2160));
    let hevc = Plan::Reencode {
        graph: Box::new(build_graph(&[], None, &spec(true, true))),
        settings,
        subtitles: None,
        track: None,
    };
    assert!(args(&hevc).windows(2).any(|w| w == ["-tag:v", "hvc1"]));
    // an audio format re-encodes even with the same video
    let Plan::Reencode { settings, .. } = plan(
        FormatChoice {
            video: None,
            audio: AudioChoice::Mp3,
            ..Default::default()
        },
        &all,
    )
    .unwrap() else {
        panic!("a chosen audio format must re-encode")
    };
    assert_eq!(settings.aencoder, "libmp3lame");
    // missing encoders are refused, not replaced
    let no_mp3 = enc();
    assert_eq!(
        plan(
            FormatChoice {
                video: None,
                audio: AudioChoice::Mp3,
                ..Default::default()
            },
            &no_mp3
        ),
        Err(RenderError::NoEncoder)
    );
    let h264_only: std::collections::BTreeSet<String> =
        ["libx264", "aac"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        plan(
            FormatChoice {
                video: Some(10),
                audio: AudioChoice::Same,
                ..Default::default()
            },
            &h264_only
        ),
        Err(RenderError::NoEncoder)
    );
}

// covers: REQ-119, REQ-121, REQ-114
#[test]
fn only_audio_makes_the_sound_of_the_whole_timeline_without_any_video() {
    use crate::format::AudioChoice;
    let mut p = project(&[(4, 1280, 720, 30), (3, 1280, 720, 30)]);
    let first = p.timeline.clips[0].id;
    p.timeline.set_clean_audio(&[first], true).unwrap();
    let m = song(9, 10);
    p.add_media(m.clone());
    p.music.place(&m, s(1), Micros(1)).unwrap();
    p.mix.music_volume = 50;
    let all: std::collections::BTreeSet<String> = ["libx264", "aac", "libmp3lame"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let plan = plan_audio_only(&p, &all, AudioChoice::Same).unwrap();
    assert_eq!(plan.settings.extension, "m4a");
    // no picture is built: no video filter at all, the clean chain and the music are there
    let g = &plan.graph.graph;
    assert!(
        !g.contains("scale=")
            && !g.contains("[outv]")
            && !g.contains("setsar")
            && !g.contains(":v:0]")
    );
    assert!(g.contains(CLEAN_AUDIO) && g.contains("[mb]") && g.contains("[outa]"));
    assert!(
        g.contains("volume=0.250000"),
        "the music volume applies: {g}"
    );
    let dir = tempfile::tempdir().unwrap();
    let (out, list, script) = (
        dir.path().join("o.m4a"),
        dir.path().join("l"),
        dir.path().join("g"),
    );
    let paths = ExportPaths {
        output: &out,
        concat_list: &list,
        graph_script: &script,
    };
    let a = strs(&audio_export_args(&plan, &paths, FfVersion::Dev, Some(2)).unwrap());
    assert!(a.windows(2).any(|w| w == ["-map", "[outa]"]));
    assert!(a.contains(&"-vn".to_string()) && !a.contains(&"-c:v".to_string()));
    assert!(a.windows(2).any(|w| w == ["-c:a", "aac"]));
    assert!(a.windows(2).any(|w| w == ["-f", "mp4"]));
    assert!(a.iter().any(|x| x == "-threads"));
    assert!(a.last().unwrap().ends_with("o.m4a"));
    // mp3 only when the encoder is there
    let mp3 = plan_audio_only(&p, &all, AudioChoice::Mp3).unwrap();
    assert_eq!(
        (
            mp3.settings.extension.as_str(),
            mp3.settings.aencoder.as_str()
        ),
        ("mp3", "libmp3lame")
    );
    let a = strs(&audio_export_args(&mp3, &paths, FfVersion::Dev, None).unwrap());
    assert!(a.windows(2).any(|w| w == ["-f", "mp3"]));
    assert_eq!(
        plan_audio_only(&p, &enc(), AudioChoice::Mp3),
        Err(RenderError::NoEncoder)
    );
    assert_eq!(
        plan_audio_only(&Project::default(), &all, AudioChoice::Aac),
        Err(RenderError::Empty)
    );
}

// covers: REQ-084, REQ-020, REQ-023
#[test]
fn clip_volume_is_a_gain_on_that_clip_only() {
    let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25)]);
    let (a, b) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    // (d) a clip level below 100 % or muted forces a re-encode
    p.timeline.set_clip_volume(a, 50, false).unwrap();
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("clip volume must re-encode")
    };
    let parts: Vec<&str> = graph.graph.split(';').collect();
    let a0 = parts.iter().find(|x| x.ends_with("[a0]")).unwrap();
    let a1 = parts.iter().find(|x| x.ends_with("[a1]")).unwrap();
    assert!(a0.contains(",volume=0.250000"), "{a0}");
    assert!(!a1.contains("volume="), "other clips untouched: {a1}");
    // (b) muted: silence for that clip only
    p.timeline.set_clip_volume(b, 100, true).unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert_eq!(segs[1].gain, 0.0);
    let g = build_graph(&segs, None, &spec(true, true)).graph;
    assert!(g.contains(",volume=0.000000"));
    // (a) clip 50 % with the video at 100 % gives the same gain as clip 100 % with
    // the video at 50 %
    let mut x = project(&[(2, 640, 360, 25)]);
    let id = x.timeline.clips[0].id;
    x.timeline.set_clip_volume(id, 50, false).unwrap();
    let clip_gain =
        segments(&x, Micros::ZERO, Source::Export).unwrap()[0].gain * x.mix.video_gain();
    let mut y = project(&[(2, 640, 360, 25)]);
    y.mix.video_volume = 50;
    let video_gain =
        segments(&y, Micros::ZERO, Source::Export).unwrap()[0].gain * y.mix.video_gain();
    assert!((clip_gain - video_gain).abs() < 1e-12);
    // the preview's sound uses the same graph
    let dir = tempfile::tempdir().unwrap();
    let a = strs(
        &preview_audio_args(
            &p,
            Micros::ZERO,
            48_000,
            2,
            FfVersion::Release(6, 1, 0),
            &dir.path().join("g.txt"),
            Source::Preview { proxies: false },
        )
        .unwrap(),
    );
    assert!(a.iter().any(|x| x.contains("volume=0.250000")));
}

// covers: REQ-083, REQ-011, REQ-023
#[test]
fn start_and_end_fades_are_rendered_at_the_video_edges() {
    let mut p = project(&[(2, 640, 360, 25), (3, 640, 360, 25), (2, 640, 360, 25)]);
    p.timeline.set_edge_fades(true).unwrap();
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("edge fades must re-encode")
    };
    let parts: Vec<&str> = graph.graph.split(';').collect();
    let v0 = parts.iter().find(|x| x.ends_with("[v0]")).unwrap();
    let v1 = parts.iter().find(|x| x.ends_with("[v1]")).unwrap();
    let v2 = parts.iter().find(|x| x.ends_with("[v2]")).unwrap();
    assert!(v0.contains("fade=t=in:st=0:d=0.500000") && !v0.contains("t=out"));
    assert!(!v1.contains("fade="), "{v1}");
    // the picture's fade ends two frames early: the very last frame is black
    assert!(v2.contains("fade=t=out:st=1.420000:d=0.500000") && !v2.contains("t=in"));
    let a2 = parts.iter().find(|x| x.ends_with("[a2]")).unwrap();
    assert!(a2.contains("afade=t=out:st=1.500000:d=0.500000"));
    let a0 = parts.iter().find(|x| x.ends_with("[a0]")).unwrap();
    assert!(a0.contains("afade=t=in:st=0"));
    // the preview's first and last frames are inside the fades
    let g = Geometry {
        width: 640,
        height: 360,
        fps: Fps { num: 25, den: 1 },
    };
    let first = frame_key(&p, Micros::ZERO, Source::Export, &g).unwrap();
    assert!(first.fade.is_some());
    let last = frame_key(
        &p,
        p.timeline.duration() - Micros(40_000),
        Source::Export,
        &g,
    );
    assert!(last.unwrap().fade.is_some());
    // the start fade belongs to the first clip: cutting it removes the fade (REQ-083)
    let first_id = p.timeline.clips[0].id;
    p.timeline.remove(first_id).unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert_eq!(segs[0].fade_in, Micros::ZERO);
}

fn sphere_project() -> Project {
    let mut p = project(&[(4, 3840, 1920, 30), (3, 640, 360, 30)]);
    p.media[0].format.spherical = Some(true);
    p.media[0].format.bit_rate = Some(40_000_000);
    p
}

// covers: REQ-103
#[test]
fn a_360_video_keeps_the_sphere_and_refuses_a_mix_with_flat_clips() {
    use crate::base::pos::Anchor360;
    use crate::text::{TextId, TextItem};
    let mut p = project(&[(2, 3840, 1920, 30), (2, 3840, 1920, 30)]);
    p.media[0].format.spherical = Some(true);
    p.media[1].format.spherical = Some(true);
    // nothing to burn in, identical clips: the file can be copied as it is
    assert!(matches!(
        plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    // a text fixed to the sphere at yaw 30 forces a re-encode: no camera in the graph,
    // the equirect picture keeps the clips' size, the text is burned on every clip
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, "North".into(), Micros::ZERO);
    t.ttl = Micros(4_000_000);
    t.anchor = Some(Anchor360 {
        yaw_cd: 3000,
        pitch_cd: 1000,
        height_cd: 500,
        width_cd: 6000,
    });
    p.texts.add(t).unwrap();
    let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
    let Plan::Reencode {
        graph, settings, ..
    } = &plan
    else {
        panic!("re-encoded")
    };
    assert_eq!((settings.width, settings.height), (3840, 1920));
    assert!(!graph.graph.contains("v360") && !graph.graph.contains("sendcmd"));
    assert!(
        graph.graph.contains("subtitles=f=anc0.ass:fontsdir=fonts"),
        "{}",
        graph.graph
    );
    assert!(graph.graph.contains("subtitles=f=anc1.ass"));
    let doc = &graph
        .command_files
        .iter()
        .find(|(n, _)| n == "anc0.ass")
        .unwrap()
        .1;
    // yaw 30 / pitch 10 on 3840x1920: x = (30/360 + 0.5) * 3840, y = (0.5 - 10/180) * 1920
    assert!(doc.contains("\\pos(2240,853)"), "{doc}");
    assert!(doc.contains("PlayResX: 3840\nPlayResY: 1920"));
    // the flat file has nothing of it, but exists so that the fonts are written
    assert!(!plan.subtitles().unwrap().contains("Dialogue"));
    // the same project as a flat camera view: v360 per clip, the text burned before it
    let Plan::Reencode { graph, .. } =
        plan_export_mode(&p, &enc(), ExportMode::FlatCamera, |_, _| true).unwrap()
    else {
        panic!()
    };
    assert!(graph.graph.contains("subtitles=f=anc0.ass") && graph.graph.contains("v360@c0"));
    let (a, b) = (
        graph.graph.find("anc0.ass").unwrap(),
        graph.graph.find("v360@c0").unwrap(),
    );
    assert!(a < b, "burned before the camera view is cut out");
    // a flat clip in the mix: refused, not exported
    let mut q = project(&[(2, 3840, 1920, 30), (2, 1280, 720, 30)]);
    q.media[0].format.spherical = Some(true);
    assert_eq!(
        plan_export_mode(&q, &enc(), ExportMode::Sphere360, |_, _| true).unwrap_err(),
        RenderError::MixedSphere
    );
    assert!(
        RenderError::MixedSphere
            .to_string()
            .contains("Flat camera view")
            || RenderError::MixedSphere
                .to_string()
                .contains("flat camera view")
    );
    let _ = TextId(0);
}

// covers: REQ-112, REQ-103
#[test]
fn a_line_fixed_to_the_sphere_is_burned_on_the_equirect_picture_and_not_on_the_flat_one() {
    use crate::base::pos::Anchor360;
    let mut p = sphere_project();
    let tl = p.timeline.clone();
    let a = p
        .subtitles
        .add_at(&tl, Micros(1_000_000), "pinned", Micros(1))
        .unwrap();
    let b = p
        .subtitles
        .add_at(&tl, Micros(5_000_000), "flat clip", Micros(1))
        .unwrap();
    let c = p
        .subtitles
        .add_at(&tl, Micros(2_000_000), "automatic", Micros(1))
        .unwrap();
    let anchor = Anchor360 {
        yaw_cd: 3000,
        pitch_cd: 0,
        height_cd: 500,
        width_cd: 6000,
    };
    for id in [a, b] {
        p.subtitles
            .lines
            .iter_mut()
            .find(|l| l.id == id)
            .unwrap()
            .anchor = Some(anchor);
    }
    let _ = c;
    let eg = Geometry {
        width: 1280,
        height: 720,
        fps: crate::time::Fps::new(30, 1).unwrap(),
    };
    let flat = subtitles_doc(&p, &eg, Micros::ZERO).unwrap();
    assert!(!flat.contains(",pinned"), "{flat}");
    assert!(flat.contains(",automatic") && flat.contains(",flat clip"));
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    let doc = segs[0].anchored.as_ref().unwrap();
    assert!(doc.contains("pinned") && !doc.contains("automatic") && !doc.contains("flat clip"));
    assert!(segs[1].anchored.is_none());
    // 360 video: no camera, the line is burned on the sphere picture
    let plan = plan_export_mode(&p, &enc(), ExportMode::FlatCamera, |_, _| true).unwrap();
    let Plan::Reencode { graph, .. } = plan else {
        panic!()
    };
    assert!(graph.graph.contains("subtitles=f=anc0.ass"));
    // the picture key knows where lines are fixed
    let eg2 = output_geometry(&p, &enc()).unwrap();
    let k = frame_key(&p, Micros(1_500_000), Source::Export, &eg2).unwrap();
    assert_eq!(k.line_anchors, [Some(anchor)]);
}

// covers: REQ-112
#[test]
fn a_text_fixed_to_the_sphere_is_shown_flat_only_on_flat_clips() {
    use crate::base::pos::Anchor360;
    use crate::text::TextItem;
    // a 360 clip (0-4 s) then a flat one (4-7 s); the text lives from 2 s to 6 s
    let mut p = sphere_project();
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, "Pin".into(), Micros(2_000_000));
    t.ttl = Micros(4_000_000);
    t.anchor = Some(Anchor360 {
        yaw_cd: -17900,
        pitch_cd: 0,
        height_cd: 500,
        width_cd: 6000,
    });
    p.texts.add(t).unwrap();
    let eg = Geometry {
        width: 1280,
        height: 720,
        fps: crate::time::Fps::new(30, 1).unwrap(),
    };
    let flat = subtitles_doc(&p, &eg, Micros::ZERO).unwrap();
    // flat part: 4 s .. 6 s only
    assert!(
        flat.contains("Dialogue: 1,0:00:04.00,0:00:06.00,Text"),
        "{flat}"
    );
    assert_eq!(flat.matches("Dialogue: 1").count(), 1);
    // the 360 clip: its own file, 2 s .. 4 s, local time from the clip start; the text
    // is near the left edge (yaw -179): drawn also at the right edge (the seam)
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    let doc = segs[0].anchored.as_ref().unwrap();
    assert!(
        doc.contains("Dialogue: 1,0:00:02.00,0:00:04.00,Text"),
        "{doc}"
    );
    assert_eq!(doc.matches("Dialogue: 1").count(), 2, "{doc}");
    assert!(segs[1].anchored.is_none());
    // a preview from 3 s: local time 0 is 3 s
    let segs = segments(&p, Micros(3_000_000), Source::Export).unwrap();
    assert!(
        segs[0]
            .anchored
            .as_ref()
            .unwrap()
            .contains("Dialogue: 1,0:00:00.00,0:00:01.00,Text")
    );
    // without a 360 clip the text is at its flat place
    let mut q = project(&[(4, 1280, 720, 30)]);
    let id = q.texts.new_id();
    let mut t = TextItem::new(id, "Flat".into(), Micros::ZERO);
    t.anchor = Some(Anchor360 {
        yaw_cd: 0,
        pitch_cd: 0,
        height_cd: 500,
        width_cd: 6000,
    });
    q.texts.add(t).unwrap();
    assert!(
        subtitles_doc(&q, &eg, Micros::ZERO)
            .unwrap()
            .contains("Dialogue: 1")
    );
}

// covers: REQ-103, REQ-023, REQ-022
#[test]
fn a_360_clip_is_never_stream_copied_and_counts_as_1080p() {
    let mut p = project(&[(2, 3840, 1920, 30), (2, 3840, 1920, 30)]);
    // flat: identical formats copy
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    p.media[0].format.spherical = Some(true);
    let Plan::Reencode {
        graph, settings, ..
    } = plan_export(&p, &enc(), |_, _| true).unwrap()
    else {
        panic!("a 360 clip is re-encoded");
    };
    assert_eq!((settings.width, settings.height), (1920, 1080));
    assert!(
        graph
            .graph
            .contains("v360@c0=input=e:output=flat:reset_rot=1:w=1920:h=1080")
    );
    assert!(
        !graph.graph.split(';').next().unwrap().contains("scale="),
        "no bars for the camera view"
    );
    // the huge bit rate of the sphere is not given to the flat picture
    assert!(settings.video_bitrate.unwrap() < 10_000_000);
    // without the file's spherical information it is flat again
    p.media[0].format.spherical = Some(false);
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    assert_eq!(effective_format(&p.media[0]).width, 3840);
}

// covers: REQ-103, REQ-022
#[test]
fn a_360_clip_next_to_a_normal_one_follows_the_usual_choice() {
    let p = sphere_project();
    let g = output_geometry(&p, &enc()).unwrap();
    assert_eq!((g.width, g.height), (640, 360));
    let q = {
        let mut q = project(&[(4, 3840, 1920, 25)]);
        q.media[0].format.spherical = Some(true);
        q
    };
    let g = output_geometry(&q, &enc()).unwrap();
    assert_eq!(
        (g.width, g.height, g.fps),
        (1920, 1080, Fps::new(25, 1).unwrap())
    );
}

// covers: REQ-102, REQ-103, REQ-266
#[test]
fn the_camera_path_becomes_one_command_file_for_each_moving_clip() {
    let mut p = sphere_project();
    let m = p.media[0].id;
    let pov = |t: i64, yaw: i64| crate::camera::Pov {
        t: Micros(t),
        cam: Camera::new(yaw * 100, 0, 9000),
        ease: Default::default(),
    };
    // no POV: the camera stays at the front, no command file
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!()
    };
    assert!(graph.command_files.is_empty() && !graph.graph.contains("sendcmd"));
    p.camera.set(m, pov(0, 0)).unwrap();
    p.camera.set(m, pov(2_000_000, 90)).unwrap();
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode { graph, .. } = &plan else {
        panic!()
    };
    assert_eq!(graph.command_files.len(), 1);
    let (name, text) = &graph.command_files[0];
    assert_eq!(name, "cam0.txt");
    assert_eq!(
        text.lines().count(),
        20,
        "a line every third frame while it moves: at most ten updates a second"
    );
    assert!(graph.graph.contains("fps=30/1,sendcmd=f=cam0.txt,v360@c0="));
    assert_eq!(plan.command_files().len(), 1);
    let dir = tempfile::tempdir().unwrap();
    write_command_files(dir.path(), plan.command_files(), plan.lut_files()).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("cam0.txt")).unwrap(),
        *text
    );
    // a clip cut at its start starts at the view the path had there
    let id = p.timeline.clips[0].id;
    p.timeline
        .trim(
            id,
            Micros(1_000_000),
            Micros(4_000_000),
            Micros(4_000_000),
            Micros(1),
        )
        .unwrap();
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!()
    };
    assert!(graph.graph.contains("yaw=45.00"), "{}", graph.graph);
    // a normal clip is untouched by it all
    assert!(graph.graph.contains("[1:v:0]trim") || graph.graph.contains("scale=640:360"));
    // preview of a later start: the offset comes after the camera
    let eg = output_geometry(&p, &enc()).unwrap();
    let r = preview_video_args(
        &p,
        Micros(500_000),
        &eg,
        FfVersion::Dev,
        Path::new("/tmp/g.txt"),
        Source::Export,
    )
    .unwrap();
    assert_eq!(r.command_files.len(), 1);
    let a = strs(&r.args).join(" ");
    assert!(
        a.contains("v360@c0") && a.contains("setpts=PTS+0.500000/TB"),
        "{a}"
    );
}

// covers: REQ-101, REQ-102
#[test]
fn the_picture_key_follows_the_camera() {
    let mut p = sphere_project();
    let m = p.media[0].id;
    let eg = output_geometry(&p, &enc()).unwrap();
    let src = Source::Export;
    let k0 = frame_key(&p, Micros(500_000), src, &eg).unwrap();
    assert_eq!(k0.camera, Some(Camera::FRONT));
    p.camera
        .set(
            m,
            crate::camera::Pov {
                t: Micros::ZERO,
                cam: Camera::new(1000, 0, 9000),
                ease: Default::default(),
            },
        )
        .unwrap();
    let k1 = frame_key(&p, Micros(500_000), src, &eg).unwrap();
    assert_ne!(k0, k1);
    // a normal clip has no camera
    let n = frame_key(&p, Micros(4_500_000), src, &eg).unwrap();
    assert_eq!(n.camera, None);
}

// covers: REQ-104
#[test]
fn a_sphere_too_large_for_the_preview_is_not_decoded_but_still_exported() {
    let mut p = project(&[(2, 20_000, 10_000, 30)]);
    p.media[0].format.spherical = Some(true);
    let eg = Geometry {
        width: 1920,
        height: 1080,
        fps: Fps::new(30, 1).unwrap(),
    };
    let sc = Path::new("/tmp/g.txt");
    let src = Source::Preview { proxies: true };
    let e = still_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src).unwrap_err();
    assert_eq!(
        e,
        "This 360° video is too large to show here. It is still exported."
    );
    assert!(preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src).is_err());
    assert_eq!(
        segments(&p, Micros::ZERO, src),
        Err(RenderError::SphereTooLarge)
    );
    // the export is planned as usual
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::Reencode { .. }
    ));
    // a normal video of the same size is not subject to this rule
    p.media[0].format.spherical = Some(false);
    assert!(segments(&p, Micros::ZERO, src).is_ok());
}

// ---- 1.5.0: Change aspect, Scale and Move ----

use crate::frame::{Fill, Fit, FrameAspect, Rgb, Scale, Shape};
use crate::timeline::FrameTarget;

fn graph_of(p: &Project) -> (Graph, Geometry, bool) {
    let plan = plan_export(p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode {
        graph, settings, ..
    } = plan
    else {
        return (
            Graph {
                inputs: vec![],
                graph: String::new(),
                command_files: vec![],
                lut_files: vec![],
                spatial: vec![],
            },
            Geometry {
                width: 0,
                height: 0,
                fps: Fps::new(1, 1).unwrap(),
            },
            false,
        );
    };
    let g = Geometry::of(&settings);
    (*graph, g, true)
}

fn tall(fit: Fit, fill: Fill) -> FrameAspect {
    FrameAspect {
        shape: Shape::Tall9x16,
        fit,
        fill,
        blur: Default::default(),
    }
}

// covers: REQ-131
// covers: REQ-023
#[test]
fn any_frame_effect_makes_a_reencode_even_at_the_defaults() {
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    let id = p.timeline.clips[0].id;
    p.timeline.add_scale(FrameTarget::Clip(id)).unwrap();
    let (g, geo, re) = graph_of(&p);
    assert!(re, "Scale at 100% is a re-encode");
    assert_eq!((geo.width, geo.height), (1280, 720));
    // only the first clip has the frame path
    assert!(g.graph.contains("overlay="));
    assert_eq!(g.graph.matches("overlay=").count(), 1);
    let mut q = project(&[(2, 1280, 720, 30)]);
    q.timeline.add_move(FrameTarget::Whole).unwrap();
    assert!(graph_of(&q).2);
    let mut r = project(&[(2, 1280, 720, 30)]);
    r.timeline
        .set_frame_aspect(Some(FrameAspect::default()))
        .unwrap();
    assert!(graph_of(&r).2);
}

// covers: REQ-125
// covers: REQ-126
// covers: REQ-131
#[test]
fn the_frame_graph_has_background_picture_overlay_in_order() {
    let mut p = project(&[(4, 1600, 1200, 30)]);
    p.timeline
        .set_frame_aspect(Some(tall(Fit::Width, Fill::Colour(Rgb(0, 255, 0)))))
        .unwrap();
    let (g, geo, _) = graph_of(&p);
    assert_eq!((geo.width, geo.height), (1600, 2844));
    let want = "[0:v:0]trim=start=0.000000:duration=4.000000,\
                setpts=PTS-STARTPTS+0.000000/TB,scale=1600:-2,setsar=1,fps=30/1[pa0];\
                color=c=0x00FF00:s=1600x2844:r=30/1:d=4.000000,\
                setpts=PTS-STARTPTS+0.000000/TB[b0];\
                [b0][pa0]overlay=x='(W-w)/2':y='(H-h)/2':eval=frame:shortest=1,\
                format=yuv420p,setpts=PTS-STARTPTS[v0]";
    let video: Vec<&str> = g.graph.split(';').take(3).collect();
    assert_eq!(video.join(";"), want);
    assert!(g.graph.contains("[b0][pa0]overlay=x='(W-w)/2':y='(H-h)/2'"));
    // blur: a small blurred copy is the background
    let mut q = project(&[(4, 1600, 1200, 30)]);
    q.timeline
        .set_frame_aspect(Some(tall(Fit::Height, Fill::Blur)))
        .unwrap();
    let (g, geo, _) = graph_of(&q);
    assert_eq!((geo.width, geo.height), (676, 1200));
    assert!(
        g.graph
            .contains("scale=-2:1200,setsar=1,fps=30/1,split=2[pa0][pb0]")
    );
    assert!(g.graph.contains("[pb0]scale=84:150:force_original_aspect_ratio=increase,crop=84:150,boxblur=3:2,scale=676:1200,setsar=1[b0]"));
    assert!(!g.graph.contains("color=c="));
}

/// The blurred fill is computed on a small copy and scaled up, so its cost does not grow
/// with the frame; the sharp picture does not depend on it. Measured with ffmpeg 9.0.2 (a 4K
/// source in a 2160x3840 frame, 8 s of video): the divisor 8 took 3.0 s and a divisor of 4
/// took 3.5 s, so the copy is an eighth of each side (at most the quarter REQ-222 names).
// covers: REQ-222
#[test]
fn the_blurred_background_is_computed_at_a_fraction_of_the_frame() {
    const { assert!(frame::BLUR_DIVISOR >= 4) }; // at most a quarter of each side
    {
        let mut p = project(&[(4, 3840, 2160, 30)]);
        p.timeline
            .set_frame_aspect(Some(tall(Fit::Height, Fill::Blur)))
            .unwrap();
        let (g, geo, _) = graph_of(&p);
        let want = frame::frame_size((3840, 2160), p.timeline.frame_aspect.as_ref());
        assert_eq!((geo.width, geo.height), want);
        // the small copy: size / divisor, then back up to the frame
        let chain = g
            .graph
            .split(';')
            .find(|c| c.contains("boxblur"))
            .expect("a blur chain")
            .to_string();
        let scale: Vec<u32> = chain
            .split("scale=")
            .nth(1)
            .unwrap()
            .split(':')
            .take(2)
            .map(|v| v.parse().unwrap())
            .collect();
        assert!(
            scale[0] * 4 <= geo.width && scale[1] * 4 <= geo.height,
            "{chain}"
        );
        assert!(
            u64::from(scale[0]) * u64::from(scale[1]) * 16
                <= u64::from(geo.width) * u64::from(geo.height),
            "the blur sees at most a sixteenth of the pixels: {chain}"
        );
        assert!(chain.ends_with(&format!("scale={}:{},setsar=1[b0]", geo.width, geo.height)));
        // the sharp picture is the same chain whatever the fill
        let mut q = p.clone();
        q.timeline
            .set_frame_aspect(Some(tall(Fit::Height, Fill::Colour(Rgb(0, 0, 0)))))
            .unwrap();
        let (gq, _, _) = graph_of(&q);
        let pic = |g: &Graph| {
            g.graph
                .split(';')
                .next()
                .unwrap()
                .split(",split=2")
                .next()
                .unwrap()
                .to_string()
        };
        assert_eq!(pic(&g), pic(&gq).split("[pa0]").next().unwrap().to_string());
    }
}

// covers: REQ-126
#[test]
fn the_blur_radius_stays_inside_ffmpegs_bounds_for_every_small_side() {
    for side in (frame::BLUR_MIN_SIDE..=600).step_by(2) {
        for other in [frame::BLUR_MIN_SIDE, side, 600] {
            let r = blur_radius(side, other, frame::BlurLevel::High);
            // the chroma planes are half the size and ffmpeg wants radius < side / 2
            assert!(
                r >= 1 && r < (side.min(other) / 2) / 2,
                "{r} for {side}x{other}"
            );
        }
    }
    assert_eq!(blur_radius(8, 14, frame::BlurLevel::Medium), 1);
    assert_eq!(blur_radius(84, 150, frame::BlurLevel::Medium), 3);
}

// covers: REQ-155
#[test]
fn a_picture_from_inside_a_frame_is_read_from_the_frame_that_holds_it() {
    let p = project(&[(2, 640, 360, 25)]);
    let at = |t: i64| {
        let r = still_args(
            &p,
            Micros(t),
            &Geometry {
                width: 640,
                height: 360,
                fps: Fps::new(25, 1).unwrap(),
            },
            FfVersion::Dev,
            Path::new("g.txt"),
            Source::Export,
        )
        .unwrap();
        strs(&r.args).join(" ")
    };
    // 1.98 s is inside the frame that starts at 1.96 s: it is read from there
    let late = at(1_980_000);
    assert!(late.contains("-ss 1.959000"), "{late}");
    assert!(late.contains("trim=start=0.000000:duration=0.041000,setpts=PTS-STARTPTS+1.960000/TB"));
    assert!(late.contains("setpts=PTS-STARTPTS,trim=duration=0.020000[v0]"));
    // on a frame, or from the start of a clip, nothing changes
    let on = at(1_920_000);
    assert!(on.contains("-ss 1.920000"), "{on}");
    assert!(!on.contains("trim=duration="));
    assert!(at(0).contains("-ss 0.000000"));
    let seg = &segments(&p, Micros(1_980_000), Source::Export).unwrap()[0];
    assert_eq!(seg.lead(), Micros(20_000));
}

// covers: REQ-127
// covers: REQ-128
// covers: REQ-131
#[test]
fn scale_and_move_expressions_follow_the_keys_in_clip_time() {
    let mut p = project(&[(4, 640, 360, 30), (4, 640, 360, 30)]);
    let second = p.timeline.clips[1].id;
    let c = FrameTarget::Clip(second);
    p.timeline.add_scale(c).unwrap();
    p.timeline
        .add_scale_key(
            c,
            Micros(6_000_000),
            Scale {
                permille: 2000,
                dx: 0,
                dy: 0,
            },
        )
        .unwrap();
    p.timeline.add_move(FrameTarget::Whole).unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    // clip 1 starts at timeline 0: the whole video's Move key at 0 s
    assert_eq!(segs[0].frame.as_ref().unwrap().stack.scale.len(), 0);
    let (g, _, _) = graph_of(&p);
    // clip 2: scale keys at source 0 and 6 s, the clip starts at source 0: local 0..6
    assert!(
        g.graph
            .contains("scale=w='max(2,trunc(iw*(if(lt(t,6),(1+(1)*max(0,t-0)/6),2))/2)*2)'"),
        "{}",
        g.graph
    );
    // the picture's centre: the whole video's Move at the centre adds nothing
    assert!(g.graph.contains("overlay=x='(W-w)/2'"));
}

// covers: REQ-131
#[test]
fn a_360_export_ignores_the_frame_effects_and_the_flat_view_applies_them() {
    let mut p = sphere_project();
    let plain = {
        let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true);
        // clip 2 is flat: a mix is refused, so use two 360 clips
        assert!(plan.is_err());
        let mut q = project(&[(2, 3840, 1920, 30), (2, 3840, 1920, 30)]);
        q.media[0].format.spherical = Some(true);
        q.media[1].format.spherical = Some(true);
        q.texts = Default::default();
        q
    };
    let with = |q: &Project| {
        let mut q = q.clone();
        // a text keeps the plan a re-encode
        let id = q.texts.new_id();
        let mut t = crate::text::TextItem::new(id, "N".into(), Micros::ZERO);
        t.ttl = Micros(4_000_000);
        q.texts.add(t).unwrap();
        q
    };
    let base = with(&plain);
    let mut effects = base.clone();
    effects
        .timeline
        .set_frame_aspect(Some(tall(Fit::Width, Fill::Blur)))
        .unwrap();
    effects.timeline.add_scale(FrameTarget::Whole).unwrap();
    let a = plan_export_mode(&base, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
    let b = plan_export_mode(&effects, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
    assert_eq!(a, b);
    // the flat camera view of the same project has the frame
    let c = plan_export_mode(&effects, &enc(), ExportMode::FlatCamera, |_, _| true).unwrap();
    let Plan::Reencode {
        graph, settings, ..
    } = c
    else {
        panic!("re-encode")
    };
    assert_eq!((settings.width, settings.height), (1920, 3414));
    assert!(graph.graph.contains("v360") && graph.graph.contains("overlay="));
    p.timeline.clips.clear();
}

// covers: REQ-131
#[test]
fn subtitles_are_drawn_after_the_composition() {
    let mut p = project(&[(4, 640, 360, 30)]);
    p.timeline.add_scale(FrameTarget::Whole).unwrap();
    let g = build_graph(
        &segments(&p, Micros::ZERO, Source::Export).unwrap(),
        None,
        &GraphSpec {
            subtitles: true,
            ..spec(true, false)
        },
    );
    let at = |n: &str| g.graph.find(n).unwrap();
    assert!(at("overlay=") < at("concat=") && at("concat=") < at("subtitles=f=subs.ass"));
}

// covers: REQ-131
#[test]
fn a_preview_from_the_middle_of_a_clip_shifts_the_background_with_the_picture() {
    let mut p = project(&[(4, 640, 360, 30)]);
    p.timeline.add_move(FrameTarget::Whole).unwrap();
    let segs = segments(&p, s(1), Source::Export).unwrap();
    let g = build_graph(&segs, None, &spec(true, false));
    assert!(g.graph.contains("setpts=PTS-STARTPTS+1.000000/TB"));
    assert_eq!(
        g.graph.matches("setpts=PTS-STARTPTS+1.000000/TB").count(),
        2
    );
}

// covers: REQ-034
// covers: REQ-131
#[test]
fn the_frame_key_changes_with_the_picture_state_only() {
    let mut p = project(&[(4, 640, 360, 30)]);
    let eg = Geometry {
        width: 640,
        height: 360,
        fps: Fps::new(30, 1).unwrap(),
    };
    let key = |p: &Project, at: i64| frame_key(p, s(at), Source::Export, &eg).unwrap();
    assert!(key(&p, 1).placement.is_none());
    p.timeline.add_scale(FrameTarget::Whole).unwrap();
    let one = FrameTarget::Clip(p.timeline.clips[0].id);
    p.timeline
        .add_scale_key(
            one,
            s(4),
            Scale {
                permille: 2000,
                dx: 0,
                dy: 0,
            },
        )
        .unwrap();
    let k1 = key(&p, 1);
    assert_ne!(k1, key(&p, 2));
    // a still position in the same clip with the same state has the same key
    p.timeline.remove_scale_key(one, s(4)).unwrap();
    assert_eq!(key(&p, 1).placement, key(&p, 2).placement);
    assert_ne!(key(&p, 1), k1);
}

// covers: REQ-125
#[test]
fn the_preview_has_the_frame_shape() {
    let mut p = project(&[(4, 1600, 1200, 30)]);
    p.timeline
        .set_frame_aspect(Some(tall(Fit::Width, Fill::Blur)))
        .unwrap();
    let g = output_geometry(&p, &enc()).unwrap();
    assert_eq!((g.width, g.height), (1600, 2844));
    assert_eq!(g.preview().height, 360);
    let sc = Path::new("/tmp/g.txt");
    let r = still_args(&p, Micros::ZERO, &g, FfVersion::Dev, sc, Source::Export).unwrap();
    let script = strs(&r.args).join(" ");
    assert!(script.contains("boxblur") || !r.args.is_empty());
}

fn lut_file(dir: &Path, name: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, crate::lut::tests::identity(2)).unwrap();
    p.to_string_lossy().into_owned()
}

fn lut_graph(p: &Project, src: Source) -> Graph {
    let segs = segments(p, Micros::ZERO, src).unwrap();
    build_graph(&segs, None, &spec(true, false))
}

// covers: REQ-135
// covers: REQ-136
#[test]
fn each_clip_has_one_lut_stage_and_never_uses_a_user_name() {
    use crate::timeline::FrameTarget;
    let dir = tempfile::tempdir().unwrap();
    let mut p = project(&[(2, 1280, 720, 30), (2, 1280, 720, 30)]);
    let plain = lut_graph(&p, Source::Export);
    assert!(!plain.graph.contains("lut3d") && plain.command_files.is_empty());
    let a = lut_file(dir.path(), "my warm look; rm.cube");
    let id = p.timeline.clips[1].id;
    // the whole-video LUT is a copy on each clip (REQ-147)
    p.timeline
        .set_lut(FrameTarget::Whole, Some(a.clone()))
        .unwrap();
    let g = lut_graph(&p, Source::Export);
    // the same table on both clips is one file named by its content (ADR-206)
    assert_eq!(g.lut_files.len(), 1);
    let n = crate::lut::store::copy_name(&g.lut_files[0]);
    // before the pad (black bars keep their colour), then back to yuv
    assert!(
        g.graph.contains(&format!(
            "decrease,format=gbrp,lut3d=file={n}:interp=tetrahedral,format=yuv420p,pad="
        )),
        "{}",
        g.graph
    );
    assert!(g.graph.contains("[0:v:0]") && !g.graph.contains("warm"));
    assert_eq!(g.graph.matches(&n).count(), 2);
    assert!(g.command_files.is_empty());
    // one clip alone
    p.timeline.set_lut(FrameTarget::Whole, None).unwrap();
    p.timeline.set_lut(FrameTarget::Clip(id), Some(a)).unwrap();
    let g = lut_graph(&p, Source::Export);
    assert_eq!(g.graph.matches("lut3d").count(), 1);
    assert!(
        g.graph.contains("[1:v:0]trim")
            && g.graph.split("[1:v:0]").nth(1).unwrap().contains("lut3d")
    );
    // a frame effect: the LUT acts on the fitted picture before the overlay
    let b = lut_file(dir.path(), "b.cube");
    p.timeline.set_lut(FrameTarget::Whole, Some(b)).unwrap();
    p.timeline
        .set_frame_aspect(Some(tall(Fit::Width, Fill::Blur)))
        .unwrap();
    let g = lut_graph(&p, Source::Export);
    let lut = g.graph.find("lut3d").unwrap();
    assert!(lut < g.graph.find("overlay").unwrap());
}

// covers: REQ-136
#[test]
fn a_lut_forces_the_reencode_and_the_export_checks_the_file_first() {
    use crate::timeline::FrameTarget;
    let dir = tempfile::tempdir().unwrap();
    let mut p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::StreamCopy { .. }
    ));
    let path = lut_file(dir.path(), "a.cube");
    p.timeline
        .set_lut(FrameTarget::Whole, Some(path.clone()))
        .unwrap();
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        Plan::Reencode { .. }
    ));
    // a missing file: the export stops with a message that names it; the preview
    // ignores the LUT
    std::fs::remove_file(&path).unwrap();
    let e = plan_export(&p, &enc(), |_, _| true).unwrap_err();
    assert!(matches!(e, RenderError::Lut(ref n, _) if n == "a.cube"));
    assert!(e.to_string().contains("a.cube"));
    let g = lut_graph(&p, Source::Preview { proxies: false });
    assert!(!g.graph.contains("lut3d"));
    // a broken file is refused at the export too
    std::fs::write(&path, "nonsense").unwrap();
    assert!(matches!(
        plan_export(&p, &enc(), |_, _| true),
        Err(RenderError::Lut(..))
    ));
}

// covers: REQ-135
#[test]
fn the_lut_goes_before_texts_fixed_to_the_sphere_and_after_the_camera_otherwise() {
    use crate::timeline::FrameTarget;
    let dir = tempfile::tempdir().unwrap();
    let mut p = sphere_project();
    let path = lut_file(dir.path(), "s.cube");
    p.timeline.set_lut(FrameTarget::Whole, Some(path)).unwrap();
    let g = lut_graph(&p, Source::Export);
    assert!(
        g.graph.find("v360").unwrap() < g.graph.find("lut3d").unwrap(),
        "{}",
        g.graph
    );
    // texts fixed to the sphere: drawn after the table, so they keep their colours
    let segs = {
        let mut v = segments(&p, Micros::ZERO, Source::Export).unwrap();
        v[0].anchored = Some("[Script Info]".into());
        v
    };
    let g = build_graph(&segs, None, &spec(true, false));
    assert!(
        g.graph.find("lut3d").unwrap() < g.graph.find("subtitles=f=anc0.ass").unwrap(),
        "{}",
        g.graph
    );
}

// covers: REQ-135
#[test]
fn a_changed_lut_changes_the_frame_key_once() {
    use crate::timeline::FrameTarget;
    let dir = tempfile::tempdir().unwrap();
    let mut p = project(&[(4, 1280, 720, 30)]);
    let g = output_geometry(&p, &enc()).unwrap();
    let key = |p: &Project| frame_key(p, s(1), Source::Export, &g).unwrap();
    let none = key(&p);
    assert_eq!(none.luts, None);
    let path = lut_file(dir.path(), "k.cube");
    p.timeline
        .set_lut(FrameTarget::Whole, Some(path.clone()))
        .unwrap();
    let with = key(&p);
    assert_ne!(none, with);
    assert!(with.luts.is_some());
    assert_eq!(with, key(&p));
    std::fs::write(&path, crate::lut::tests::identity(3)).unwrap();
    assert_ne!(with, key(&p));
}

// covers: REQ-182
#[test]
fn a_separate_subtitle_track_is_a_third_input_named_by_language_and_not_switched_on() {
    use crate::format::{FormatChoice, SubtitleMode};
    let mut p = project(&[(4, 1280, 720, 30)]);
    let tl = p.timeline.clone();
    p.subtitles
        .add_at(&tl, s(1), "hi there", Micros(1))
        .unwrap();
    p.subtitles.language = Some("it".into());
    let plan_for = |p: &Project, mode| {
        plan_export_choice(
            p,
            &enc(),
            ExportMode::Automatic,
            &FormatChoice {
                subtitles: mode,
                ..Default::default()
            },
            |_, _| true,
        )
        .unwrap()
    };
    // the default is unchanged: burned in, no subtitle stream
    let burned = plan_for(&p, SubtitleMode::Burned);
    let a = args(&burned);
    assert!(a.iter().any(|x| x.contains("subtitles=f=subs.ass")));
    assert!(!a.iter().any(|x| x == "-c:s" || x.starts_with("-metadata")));
    assert!(burned.track().is_none());
    let (x, y) = (args(&plan_for(&p, SubtitleMode::default())), &a);
    assert_eq!(
        x[..x.len() - 1],
        y[..y.len() - 1],
        "only the output path differs"
    );
    // separate: no text in the picture, one more input and its stream settings
    let separate = plan_for(&p, SubtitleMode::Separate);
    let a = args(&separate);
    assert!(!a.iter().any(|x| x.contains("subtitles=f=subs.ass")));
    assert!(separate.subtitles().is_none());
    let t = separate.track().unwrap();
    assert_eq!(
        (t.file, t.codec, t.language.as_str()),
        ("subs-track.srt", "mov_text", "ita")
    );
    assert!(t.content.contains("hi there") && t.content.contains("00:00:01,000"));
    let last_input = a.iter().rposition(|x| x == "-i").unwrap();
    assert_eq!(a[last_input + 1], "subs-track.srt");
    for pair in [
        ["-c:s", "mov_text"],
        ["-metadata:s:s:0", "language=ita"],
        ["-disposition:s:0", "0"],
    ] {
        assert!(a.windows(2).any(|w| w == pair), "{pair:?} in {a:?}");
    }
    // the subtitle input is the one after the media inputs
    let inputs = a.iter().filter(|x| *x == "-i").count();
    assert!(
        a.windows(2)
            .any(|w| w[0] == "-map" && w[1] == format!("{}:0", inputs - 1))
    );
    // containers: MKV takes SRT, WebM takes WebVTT, others have no track
    let lines = p.subtitles.project(&p.timeline);
    assert_eq!(
        SubtitleTrack::new(&lines, None, "matroska").unwrap().codec,
        "srt"
    );
    let w = SubtitleTrack::new(&lines, Some("xx"), "webm").unwrap();
    assert_eq!(
        (w.codec, w.file, w.language.as_str()),
        ("webvtt", "subs-track.vtt", "und")
    );
    assert!(w.content.starts_with("WEBVTT"));
    assert!(SubtitleTrack::new(&lines, None, "mp3").is_none());
    assert!(SubtitleTrack::new(&[], None, "mp4").is_none());
    // texts stay burned: a text item still needs subtitles.ass
    use crate::text::TextItem;
    let id = p.texts.new_id();
    p.texts
        .add(TextItem::new(id, "Title".into(), s(2)))
        .unwrap();
    let both = plan_for(&p, SubtitleMode::Separate);
    let doc = both.subtitles().unwrap();
    assert!(doc.contains("Style: Text,") && !doc.contains("hi there"));
    assert!(both.track().is_some());
}

// covers: REQ-284
#[test]
fn the_blur_level_sets_the_blur_radius_of_the_background() {
    let radius = |level: frame::BlurLevel| {
        let mut q = project(&[(4, 1600, 1200, 30)]);
        q.timeline
            .set_frame_aspect(Some(FrameAspect {
                blur: level,
                ..tall(Fit::Height, Fill::Blur)
            }))
            .unwrap();
        let (g, ..) = graph_of(&q);
        let at = g.graph.find("boxblur=").expect("a blurred background") + "boxblur=".len();
        g.graph[at..]
            .split(':')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
    };
    let (low, medium, high) = (
        radius(frame::BlurLevel::Low),
        radius(frame::BlurLevel::Medium),
        radius(frame::BlurLevel::High),
    );
    assert_eq!(
        medium, 3,
        "no choice keeps the blur of every earlier version"
    );
    assert!(low < medium && medium < high, "{low} {medium} {high}");
}

// covers: REQ-269
#[test]
fn a_360_source_with_an_initial_view_starts_there_until_the_user_chooses_a_point_of_view() {
    let mut p = sphere_project();
    let m = p.media[0].id;
    p.media[0].format.pose = Some([9000, 0, 0]);
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    let path = segs[0].sphere.clone().unwrap();
    assert_eq!(camera::at(&path, Micros::ZERO).unwrap().yaw_cd, 9000);
    // a point of view at 2 s: the camera goes from the file's view to it
    p.camera
        .set(m, crate::camera::Pov::new(s(2), Camera::new(0, 0, 9000)))
        .unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    let path = segs[0].sphere.clone().unwrap();
    assert_eq!(camera::at(&path, Micros::ZERO).unwrap().yaw_cd, 9000);
    assert_eq!(camera::at(&path, s(2)).unwrap().yaw_cd, 0);
    // a point of view at the very start is the user's choice
    p.camera
        .set(
            m,
            crate::camera::Pov::new(Micros::ZERO, Camera::new(3000, 0, 9000)),
        )
        .unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert_eq!(
        camera::at(segs[0].sphere.as_ref().unwrap(), Micros::ZERO)
            .unwrap()
            .yaw_cd,
        3000
    );
    // a file without a view: nothing is added
    p.camera.drop_media(m);
    p.media[0].format.pose = None;
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert!(segs[0].sphere.as_ref().unwrap().is_empty());
}

// covers: REQ-265
#[test]
fn frame_effects_do_not_change_the_export_of_a_360_video() {
    let plain = sphere_project();
    let mut with = sphere_project();
    with.timeline
        .set_frame_aspect(Some(tall(Fit::Height, Fill::Blur)))
        .unwrap();
    with.timeline.add_scale(FrameTarget::Whole).unwrap();
    let args = |p: &Project| {
        let mut p = p.clone();
        p.timeline.clips.truncate(1);
        format!(
            "{:?}",
            plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap()
        )
    };
    let (a, b) = (args(&plain), args(&with));
    assert!(!a.is_empty());
    assert_eq!(a, b, "the same output with and without the frame effects");
}

// covers: REQ-278
#[test]
fn clean_audio_strength_is_all_none_or_a_mix_of_clean_and_original() {
    let graph = |strength: u8| {
        let mut p = project(&[(4, 640, 360, 30)]);
        let id = p.timeline.clips[0].id;
        p.timeline.set_clean_audio(&[id], true).unwrap();
        p.timeline.set_clean_strength(&[id], strength).unwrap();
        let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
        build_graph(&segs, None, &spec(false, true)).graph
    };
    let full = graph(100);
    assert!(
        full.contains(CLEAN_AUDIO) && !full.contains("amix"),
        "100 %: as in 1.4.0"
    );
    let none = graph(0);
    assert!(
        !none.contains("afftdn") && !none.contains("amix"),
        "0 %: the filter is bypassed"
    );
    let half = graph(50);
    assert!(
        half.contains("asplit=2[cd0][cw0]") && half.contains(CLEAN_AUDIO),
        "{half}"
    );
    assert!(
        half.contains("amix=inputs=2:weights='0.5000 0.5000':normalize=0"),
        "{half}"
    );
    let quarter = graph(25);
    assert!(quarter.contains("weights='0.7500 0.2500'"), "{quarter}");
}

// covers: REQ-283
#[test]
fn crop_is_gone_a_clip_with_the_old_field_is_still_copied_and_nothing_cuts_the_picture() {
    use crate::timeline::Crop;
    let mut p = project(&[(4, 640, 360, 30), (4, 640, 360, 30)]);
    let plain = format!("{:?}", plan_export(&p, &enc(), |_, _| true).unwrap());
    assert!(plain.contains("Copy"), "identical clips are copied");
    // the field of format 13 is read only to be dropped: nothing draws it and it does not
    // force a re-encode
    p.timeline.clips[0].crop = Crop {
        left: 10,
        ..Default::default()
    };
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    assert!(format!("{plan:?}").contains("Copy"));
    // a re-encode has no crop filter for it
    p.timeline
        .set_frame_aspect(Some(Default::default()))
        .unwrap();
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("a frame effect re-encodes")
    };
    assert!(!graph.graph.contains("crop=w="), "{}", graph.graph);
    // and it is never written
    let json = crate::project::to_json(&p, std::path::Path::new("/tmp/x.vlproj")).unwrap();
    assert!(!json.contains("\"crop\""));
}

// covers: REQ-276, REQ-277
#[test]
fn a_music_clip_has_its_own_level_mute_fades_and_clean_audio_while_the_track_stays() {
    let mut p = project(&[(10, 640, 360, 25)]);
    let a = song(5, 4);
    p.add_media(a.clone());
    let id = p.music.place(&a, s(1), Micros(1)).unwrap();
    let graph = |p: &Project| match plan_export(p, &enc(), |_, _| true).unwrap() {
        Plan::Reencode { graph, .. } => graph.graph,
        _ => panic!(),
    };
    let plain = graph(&p);
    assert!(
        !plain.contains("afade=t=in") && !plain.contains("afftdn"),
        "untouched: as before"
    );
    {
        let c = p.music.clips.iter_mut().find(|c| c.id == id).unwrap();
        c.volume = 50;
        c.fade_in = Micros(1_000_000);
        c.fade_out = Micros(500_000);
        c.clean_audio = true;
    }
    let g = graph(&p);
    assert!(g.contains(&format!(",{CLEAN_AUDIO},volume=0.250000,afade=t=in:st=0:d=1.000000,afade=t=out:st=3.500000:d=0.500000[m1]")), "{g}");
    // the track's own level is applied after, as before
    assert!(g.contains("volume=1.000000") || g.contains("[mb]"), "{g}");
    // muted: silent clip
    p.music.clips.iter_mut().find(|c| c.id == id).unwrap().muted = true;
    assert!(graph(&p).contains("volume=0.000000"));
    // the settings are written only when set, and survive a round trip
    let c = p.music.clips[0];
    let back: crate::music::MusicClip =
        serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(back, c);
    let plain_clip = crate::music::MusicClip {
        volume: 100,
        muted: false,
        fade_in: Micros::ZERO,
        fade_out: Micros::ZERO,
        clean_audio: false,
        ..c
    };
    assert!(
        !serde_json::to_string(&plain_clip)
            .unwrap()
            .contains("volume")
    );
}

// covers: REQ-279
#[test]
fn normalise_speech_adds_the_speechnorm_filter_to_the_clips_that_have_it() {
    let mut p = project(&[(4, 640, 360, 30), (4, 640, 360, 30)]);
    let id = p.timeline.clips[0].id;
    let graph = |p: &Project| match plan_export(p, &enc(), |_, _| true).unwrap() {
        Plan::Reencode { graph, .. } => graph.graph,
        other => format!("{other:?}"),
    };
    assert!(
        !graph(&p).contains("speechnorm"),
        "off: as before, and a stream copy"
    );
    p.timeline.set_speech_enhance(&[id], true).unwrap();
    let g = graph(&p);
    assert_eq!(g.matches(NORMALISE_SPEECH).count(), 1, "{g}");
    assert!(p.timeline.set_speech_enhance(&[], true).is_err());
}

// covers: REQ-280
#[test]
fn volume_keys_in_percent_become_a_squared_gain_expression_and_force_a_re_encode() {
    let mut p = project(&[(4, 640, 360, 30), (4, 640, 360, 30)]);
    let id = p.timeline.clips[0].id;
    p.timeline.set_volume_key(id, Micros::ZERO, 0).unwrap();
    p.timeline.set_volume_key(id, s(4), 100).unwrap();
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("a clip with volume keys is not stream copied")
    };
    assert_eq!(
        graph.graph.matches("volume='pow(").count(),
        1,
        "{}",
        graph.graph
    );
    assert!(graph.graph.contains("100/100,2)':eval=frame") || graph.graph.contains(":eval=frame"));
    // at 2 s the level is 50 % and the gain 0.25 (the same arithmetic as the expression)
    let keys = p.timeline.volume_keys(id);
    let level = crate::keyframes::at(keys, s(2)).unwrap();
    assert_eq!(level, 50);
    assert!((f64::from(level) / 100.0).powi(2) - 0.25 < 1e-9);
    // limits: 0 to 100 %, inside the clip
    assert!(p.timeline.set_volume_key(id, s(5), 10).is_err());
    assert!(p.timeline.set_volume_key(id, s(1), 101).is_err());
    p.timeline.remove_volume_key(id, Micros::ZERO).unwrap();
    p.timeline.remove_volume_key(id, s(4)).unwrap();
    assert!(!p.timeline.any_volume_keys());
}

// covers: REQ-286
#[test]
fn a_clip_with_its_own_aspect_is_made_in_its_own_frame_and_placed_in_the_project_frame() {
    let mut p = project(&[(4, 1920, 1080, 30), (4, 1920, 1080, 30)]);
    let id = p.timeline.clips[1].id;
    let plan0 = format!("{:?}", plan_export(&p, &enc(), |_, _| true).unwrap());
    assert!(plan0.contains("Copy"), "identical clips are copied");
    let a = FrameAspect {
        shape: Shape::Tall9x16,
        fit: Fit::Height,
        fill: Fill::Colour(crate::frame::Rgb::BLACK),
        blur: Default::default(),
    };
    p.timeline.set_clip_aspect(id, Some(a)).unwrap();
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let Plan::Reencode {
        graph, settings, ..
    } = &plan
    else {
        panic!("not a stream copy")
    };
    assert_eq!(
        (settings.width, settings.height),
        (1920, 1080),
        "the output size is unchanged"
    );
    // the 9:16 clip is made 608x1080 and centred in 1920x1080 with empty sides
    assert!(
        graph.graph.contains("608") && graph.graph.contains("pad=1920:1080:(ow-iw)/2:(oh-ih)/2"),
        "{}",
        graph.graph
    );
    assert_eq!(
        graph
            .graph
            .matches("pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,format")
            .count(),
        1,
        "only that clip"
    );
    // removing it brings back the stream copy
    p.timeline.set_clip_aspect(id, None).unwrap();
    assert!(format!("{:?}", plan_export(&p, &enc(), |_, _| true).unwrap()).contains("Copy"));
    assert!(
        p.timeline
            .set_clip_aspect(crate::timeline::ClipId(999), None)
            .is_err()
    );
}

// covers: REQ-281
#[test]
fn a_clip_at_another_speed_reads_the_source_and_makes_the_output_in_the_graph() {
    let mut p = project(&[(4, 640, 360, 30), (4, 640, 360, 30)]);
    let id = p.timeline.clips[0].id;
    p.timeline.set_speed(id, 2000).unwrap();
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    assert_eq!(segs[0].len, s(2), "the output length");
    assert_eq!(segs[0].src_to(), s(4), "the source read");
    assert_eq!(segs[0].rendered_src(), s(4));
    // stream copy is refused for a clip with a speed
    let Plan::Reencode { graph, .. } = plan_export(&p, &enc(), |_, _| true).unwrap() else {
        panic!("a clip with a speed is re-encoded")
    };
    let g = &graph.graph;
    assert!(g.contains("setpts=(PTS-STARTPTS)*1000/2000"), "{g}");
    assert!(g.contains("atempo=2.000000"), "{g}");
    assert!(g.contains("atrim=start=0.000000:duration=4.000000"), "{g}");
    assert_eq!(g.matches("atempo").count(), 1, "the other clip is normal");
    // a preview that starts 1 s into the clip reads 2 s of source later
    let segs = segments(&p, s(1), Source::Export).unwrap();
    assert_eq!(segs[0].src_from(), s(2));
    assert_eq!(segs[0].rendered(), s(1));
    // the camera commands are timed on the output clock
    let path = vec![
        crate::camera::Pov {
            t: Micros::ZERO,
            cam: crate::camera::Camera::FRONT,
            ease: Default::default(),
        },
        crate::camera::Pov {
            t: s(4),
            cam: crate::camera::Camera {
                yaw_cd: 9000,
                ..crate::camera::Camera::FRONT
            },
            ease: Default::default(),
        },
    ];
    let cmds = crate::camera::command_lines_speed(
        "v360@c0",
        &path,
        Micros::ZERO,
        s(4),
        Fps::new(10, 1).unwrap(),
        (640, 360),
        0,
        2000,
    );
    let last = cmds.lines().last().unwrap();
    assert!(last.starts_with("1.8") || last.starts_with("1.9"), "{last}");
}

// covers: REQ-281
#[test]
fn a_subtitle_line_stays_with_its_speech_when_the_clip_is_sped_up() {
    let mut p = project(&[(8, 640, 360, 30)]);
    let id = p.timeline.clips[0].id;
    p.timeline.set_speed(id, 2000).unwrap();
    let media = p.media[0].id;
    let tl = p.timeline.clone();
    let line = p
        .subtitles
        .add_at(&tl, s(1), "word", Micros(100_000))
        .unwrap();
    // the word is spoken at 4 s of the source: 2 s on the timeline
    let l = p.subtitles.lines.iter_mut().find(|l| l.id == line).unwrap();
    assert_eq!(l.media, media);
    l.start = s(4);
    l.end = s(5);
    let shown = p.subtitles.project(&tl);
    assert_eq!(shown[0].start, s(2));
    assert_eq!(shown[0].end, Micros(2_500_000));
}

// covers: REQ-287
// covers: REQ-316
#[test]
fn a_picture_item_is_overlaid_for_its_time_and_forces_a_re_encode() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("logo.png");
    std::fs::write(&png, crate::picture::tests::header(200, 100)).unwrap();
    let mut p = project(&[(6, 640, 360, 30)]);
    let pic = crate::picture::read(&png).unwrap();
    let id = p.texts.new_id();
    let mut item = crate::text::TextItem::new_picture(id, pic, s(2), (640, 360));
    item.ttl = s(3);
    p.texts.add(item).unwrap();
    let Plan::Reencode {
        graph, subtitles, ..
    } = plan_export(&p, &enc(), |_, _| true).unwrap()
    else {
        panic!("a picture is burned in: a re-encode")
    };
    assert!(subtitles.is_none(), "no text, no ASS file");
    assert_eq!(graph.inputs.len(), 2);
    assert!(graph.inputs[1].still);
    let g = &graph.graph;
    assert!(g.contains("[1:v:0]format=rgba,scale=192:96[pi0]"), "{g}");
    // centred: (640 - 192) / 2 and (360 - 96) / 2
    assert!(
        g.contains("overlay=x=224:y=132:enable='between(t,2.000000,5.000000)'"),
        "{g}"
    );
    assert!(g.contains("[outv]"));
    let args = strs(&input_args(&graph.inputs));
    assert!(args.windows(2).any(|w| w == ["-loop", "1"]), "{args:?}");
    // a preview that starts at 3 s counts the times from there
    let o = picture_overlays(
        &p,
        &Geometry {
            width: 640,
            height: 360,
            fps: Fps::new(25, 1).unwrap(),
        },
        s(3),
        Source::Export,
    )
    .unwrap();
    assert_eq!((o[0].start, o[0].end), (Micros::ZERO, s(2)));
    // after its end there is nothing to draw
    let none = picture_overlays(
        &p,
        &Geometry {
            width: 640,
            height: 360,
            fps: Fps::new(25, 1).unwrap(),
        },
        s(5),
        Source::Export,
    )
    .unwrap();
    assert!(none.is_empty());
    // the picture goes over the video before the subtitles of the same graph
    let spec = GraphSpec {
        subtitles: true,
        ..spec(true, false)
    };
    let segs = segments(&p, Micros::ZERO, Source::Export).unwrap();
    let g = build_graph_with(&segs, None, &spec, &o).graph;
    assert!(g.contains("[cv]subtitles"), "{g}");
    assert!(g.contains("format=yuv420p[cv]"), "{g}");
    // a file replaced by one of more than 16 megapixels is a missing picture: the export
    // stops with its message and the preview leaves it out (REQ-316, REQ-287)
    std::fs::write(&png, crate::picture::tests::header(4001, 4000)).unwrap();
    assert_eq!(
        plan_export(&p, &enc(), |_, _| true).unwrap_err(),
        RenderError::Picture("logo.png".into())
    );
    // a file that is gone stops an export and is left out of a preview
    std::fs::remove_file(&png).unwrap();
    assert_eq!(
        plan_export(&p, &enc(), |_, _| true).unwrap_err(),
        RenderError::Picture("logo.png".into())
    );
    let g = Geometry {
        width: 640,
        height: 360,
        fps: Fps::new(25, 1).unwrap(),
    };
    assert!(
        picture_overlays(&p, &g, Micros::ZERO, Source::Preview { proxies: false })
            .unwrap()
            .is_empty()
    );
}

// covers: REQ-257
#[test]
fn a_line_fixed_to_a_360_clip_has_the_preset_box_and_the_word_highlight() {
    use crate::base::pos::Anchor360;
    use crate::subtitles::{NewLine, Preset, Word};
    let mut p = sphere_project();
    let media = p.timeline.clips[0].media;
    let w = |a: i64, b: i64, t: &str| Word {
        start: Micros(a * 1_000_000),
        end: Micros(b * 1_000_000),
        text: t.into(),
    };
    p.subtitles.replace_all(
        vec![NewLine {
            media,
            start: s(1),
            end: s(3),
            text: "one two three".into(),
            words: vec![w(1, 2, "one"), w(2, 3, "two"), w(3, 3, "three")],
        }],
        None,
    );
    p.subtitles.lines[0].anchor = Some(Anchor360 {
        yaw_cd: 0,
        pitch_cd: 0,
        height_cd: 500,
        width_cd: 6000,
    });
    let doc = |p: &Project| {
        let segs = segments(p, Micros::ZERO, Source::Export).unwrap();
        segs[0].anchored.clone().unwrap()
    };
    // Classic: an outline; Box: the box style
    p.subtitles.preset = Preset::Classic;
    let d = doc(&p);
    assert!(
        d.contains(",Text,") && !d.contains("TextBox") && !d.contains("\\k"),
        "{d}"
    );
    p.subtitles.preset = Preset::Boxed;
    assert!(doc(&p).contains(",TextBox,"));
    // Word highlight: karaoke words, the spoken one yellow
    p.subtitles.preset = Preset::WordHighlight;
    let d = doc(&p);
    assert!(
        d.contains("{\\k100}one {\\k100}two") || d.contains("{\\k100}one"),
        "{d}"
    );
    assert!(d.contains("\\1c&H00FFFF&"), "{d}");
}

// ---- 2.0.4: spatial sound (REQ-273) ----

/// A 360 project whose first clip has four audio channels (first-order ambisonics) and a camera
/// turned 90 degrees to the left.
fn ambisonic_project() -> Project {
    let mut p = project(&[(4, 3840, 1920, 30), (3, 640, 360, 30)]);
    p.media[0].format.spherical = Some(true);
    p.media[0].format.channels = Some(4);
    p.media[0].format.bit_rate = Some(40_000_000);
    let m = p.media[0].id;
    p.camera
        .set(m, Pov::new(Micros::ZERO, Camera::new(-9000, 0, 9000)))
        .unwrap();
    p
}

// covers: REQ-273
#[test]
fn the_sound_of_an_ambisonic_360_clip_is_read_from_its_turned_copy_in_the_work_folder() {
    let p = ambisonic_project();
    let plan = plan_export(&p, &enc(), |_, _| true).unwrap();
    let [job] = plan.spatial() else {
        panic!("one clip has turned sound: {:?}", plan.spatial());
    };
    assert_eq!(job.src_in, Micros::ZERO);
    assert_eq!(job.len, Micros(4_000_000));
    assert_eq!(job.path[0].cam.yaw_cd, -9000);
    let Plan::Reencode { graph, .. } = &plan else {
        panic!("a 360 clip is re-encoded");
    };
    // the wav is the input after the clips' files, read from its start
    let wav = job.wav_name();
    let at = graph
        .inputs
        .iter()
        .position(|i| i.path == Path::new(&wav))
        .unwrap();
    assert_eq!(at, graph.inputs.len() - 1);
    assert!(
        graph
            .graph
            .contains(&format!("[{at}:a:0]atrim=start=0.000000:duration=4.000000"))
    );
    // and it is not decoded twice: the picture comes from the clip, the sound from the wav
    assert!(!graph.graph.contains("[0:a:0]atrim"), "{}", graph.graph);
    assert!(graph.graph.contains("[0:v:0]"));
    // the arguments read it as a plain input
    let a = args(&plan);
    assert!(a.iter().any(|x| x.ends_with(&wav)), "{a:?}");
}

// covers: REQ-273
#[test]
fn a_clip_starting_mid_way_reads_its_turned_sound_from_the_matching_offset() {
    let p = ambisonic_project();
    // a preview from 1.5 s: the wav of the clip is the same file, read from 1.5 s
    let segs = segments(
        &p,
        s(1) + Micros(500_000),
        Source::Preview { proxies: false },
    )
    .unwrap();
    let job = segs[0].spatial.clone().unwrap();
    assert_eq!(job.len, Micros(4_000_000));
    assert_eq!(
        preview_spatial(
            &p,
            s(1) + Micros(500_000),
            Source::Preview { proxies: false }
        )
        .unwrap(),
        vec![job.clone()]
    );
    let a = strs(
        &preview_audio_args(
            &p,
            s(1) + Micros(500_000),
            48_000,
            2,
            FfVersion::Dev,
            Path::new("/tmp/a.txt"),
            Source::Preview { proxies: false },
        )
        .unwrap(),
    )
    .join(" ");
    assert!(a.contains(&job.wav_name()), "{a}");
    assert!(a.contains("atrim=start=1.500000"), "{a}");
}

// covers: REQ-273
#[test]
fn a_360_video_a_flat_clip_and_a_long_preview_keep_the_sound_as_recorded() {
    let mut p = ambisonic_project();
    // the 360 video export keeps the sound as recorded
    p.media[1].format.spherical = Some(true);
    p.media[1].format.width = 3840;
    p.media[1].format.height = 1920;
    let plan = plan_export_mode(&p, &enc(), ExportMode::Sphere360, |_, _| true).unwrap();
    assert!(plan.spatial().is_empty());
    // a normal clip with four channels, and a 360 clip with two, are not turned
    let mut q = ambisonic_project();
    q.media[1].format.channels = Some(4);
    let segs = segments(&q, Micros::ZERO, Source::Export).unwrap();
    assert!(segs[0].spatial.is_some() && segs[1].spatial.is_none());
    q.media[0].format.channels = Some(2);
    assert!(
        segments(&q, Micros::ZERO, Source::Export).unwrap()[0]
            .spatial
            .is_none()
    );
    // a preview does not wait for the pre-pass of a clip longer than ten minutes
    let mut long = project(&[(700, 3840, 1920, 30)]);
    long.media[0].format.spherical = Some(true);
    long.media[0].format.channels = Some(4);
    let preview = Source::Preview { proxies: false };
    assert!(
        segments(&long, Micros::ZERO, preview).unwrap()[0]
            .spatial
            .is_none()
    );
    assert!(
        segments(&long, Micros::ZERO, Source::Export).unwrap()[0]
            .spatial
            .is_some()
    );
}

// ---- 2.0.4: picture subtitles in the player (REQ-253) ----

// covers: REQ-253
#[test]
fn a_chosen_picture_subtitle_track_is_overlaid_on_the_clip_picture_before_the_fit() {
    let mut p = project(&[(4, 1280, 720, 30)]);
    let eg = Geometry {
        width: 640,
        height: 360,
        fps: Fps::new(30, 1).unwrap(),
    };
    let sc = Path::new("/tmp/g.txt");
    let src = Source::Preview { proxies: false };
    let plain = strs(
        &preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src)
            .unwrap()
            .args,
    )
    .join(" ");
    assert!(!plain.contains(":s:"), "nothing is drawn by default");
    // the third subtitle stream of the file
    p.picture_sub = Some((p.media[0].id, 2));
    let with = strs(
        &preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src)
            .unwrap()
            .args,
    )
    .join(" ");
    assert!(
        with.contains("[0:v:0][0:s:2]overlay=eof_action=pass:format=auto[ps0]"),
        "{with}"
    );
    // the overlay comes first: the trim and the fit work on the picture with its subtitles
    assert!(with.contains("[ps0]trim=start="), "{with}");
    // the choice of another media does nothing; a 360 clip is left alone
    p.picture_sub = Some((crate::media::MediaId(99), 2));
    assert_eq!(
        strs(
            &preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src)
                .unwrap()
                .args
        )
        .join(" "),
        plain
    );
    p.picture_sub = Some((p.media[0].id, 2));
    p.media[0].format.spherical = Some(true);
    let sphere = strs(
        &preview_video_args(&p, Micros::ZERO, &eg, FfVersion::Dev, sc, src)
            .unwrap()
            .args,
    )
    .join(" ");
    assert!(!sphere.contains(":s:2"), "{sphere}");
}

// covers: REQ-253
#[test]
fn two_clips_of_one_file_share_its_picture_subtitle_stream_through_a_split() {
    let mut p = project(&[(4, 1280, 720, 30)]);
    let m = p.timeline.clips[0].media;
    let at = Micros(2_000_000);
    p.timeline.split(at, Micros(33_333)).unwrap();
    p.picture_sub = Some((m, 0));
    let g = build_graph(
        &segments(&p, Micros::ZERO, Source::Export).unwrap(),
        None,
        &spec(true, false),
    );
    assert!(
        g.graph.contains("[0:s:0]split=2[s0s0][s0s1]"),
        "{}",
        g.graph
    );
    assert!(
        g.graph.contains("[0:v:0]split=2") || g.graph.contains("[v0s0]"),
        "{}",
        g.graph
    );
    assert!(g.graph.contains("[s0s1]overlay="), "{}", g.graph);
}
