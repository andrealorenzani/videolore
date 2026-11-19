// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 1.5.0: Change aspect, Scale and Move with keyframes. Fixtures are
//! flat colours so that pixels can be sampled with a small tolerance.

use super::*;
use videolore::frame::{Fill, Fit, FrameAspect, Rgb, Scale, Shape};
use videolore::subtitles::Pos;
use videolore::timeline::FrameTarget;

/// A 4 s, 160x120, 10 fps clip of one flat colour.
fn flat(env: &Env, colour: &str) -> PathBuf {
    env.fixture(
        &format!("flat-{colour}.mp4"),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s=160x120:r=10:d=4"),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "10",
        ],
    )
}

/// Left half white, right half black: a sharp vertical edge in the middle.
fn edge(env: &Env) -> PathBuf {
    env.fixture(
        "edge.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=white:s=80x120:r=10:d=4",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=80x120:r=10:d=4",
            "-filter_complex",
            "[0][1]hstack",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ],
    )
}

fn project_of(env: &Env, file: &Path) -> Project {
    let mut p = Project::default();
    let m = env.probe(&mut p, file);
    p.timeline.append(&m).unwrap();
    // a rate that does not hide the colours
    p.media[0].format.bit_rate = Some(20_000_000);
    p
}

fn size(env: &Env, file: &Path) -> (u64, u64) {
    let o = Command::new(&env.info.ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0",
        ])
        .arg(file)
        .output()
        .unwrap();
    let t = String::from_utf8_lossy(&o.stdout);
    let mut it = t.trim().split(',').map(|v| v.parse::<u64>().unwrap());
    (it.next().unwrap(), it.next().unwrap())
}

fn px(buf: &[u8], w: u64, x: u64, y: u64) -> [i32; 3] {
    let i = ((y * w + x) * 3) as usize;
    [buf[i] as i32, buf[i + 1] as i32, buf[i + 2] as i32]
}

fn is(p: [i32; 3], want: [i32; 3]) -> bool {
    p.iter().zip(want).all(|(a, b)| (a - b).abs() <= 24)
}

const RED: [i32; 3] = [255, 0, 0];
const BLACK: [i32; 3] = [0, 0, 0];

/// Width of the red run in row `y` (the picture's width).
fn red_run(buf: &[u8], w: u64, y: u64) -> (u64, u64) {
    let xs: Vec<u64> = (0..w).filter(|x| is(px(buf, w, *x, y), RED)).collect();
    (
        xs.first().copied().unwrap_or(0),
        xs.last().copied().unwrap_or(0),
    )
}

fn export(env: &Env, p: &Project, name: &str) -> PathBuf {
    let out = env.dir.path().join(name);
    env.export(p, &out).0.unwrap();
    out
}

fn aspect(shape: Shape, fit: Fit, fill: Fill) -> FrameAspect {
    FrameAspect { shape, fit, fill }
}

// covers: REQ-125
// covers: REQ-126
#[test]
fn a_4_3_video_in_a_tall_frame_with_a_colour_fill() {
    let Some(env) = setup() else { return };
    let file = flat(&env, "red");
    let mut p = project_of(&env, &file);
    p.timeline
        .set_frame_aspect(Some(aspect(
            Shape::Tall9x16,
            Fit::Width,
            Fill::Colour(Rgb(0, 255, 0)),
        )))
        .unwrap();
    let out = export(&env, &p, "tall.mp4");
    let (w, h) = size(&env, &out);
    assert_eq!((w, h), (160, 284));
    let f = env.frame_rgb(&out, 1.0);
    let green = [0, 255, 0];
    // 82 px empty above and below, the picture 160x120 in the middle
    assert!(is(px(&f, w, 80, 40), green), "{:?}", px(&f, w, 80, 40));
    assert!(is(px(&f, w, 80, 244), green));
    assert!(is(px(&f, w, 80, 142), RED));
    assert!(is(px(&f, w, 2, 90), RED));
    assert!(is(px(&f, w, 157, 195), RED));
    // fit on height: the frame is narrower than the picture, which is cut at both sides
    let mut q = project_of(&env, &file);
    q.timeline
        .set_frame_aspect(Some(aspect(Shape::Tall9x16, Fit::Height, Fill::Blur)))
        .unwrap();
    let out = export(&env, &q, "narrow.mp4");
    let (w, h) = size(&env, &out);
    assert_eq!((w, h), (68, 120));
    let f = env.frame_rgb(&out, 1.0);
    // no empty space: the whole frame is the (cut) picture
    assert!(is(px(&f, w, 2, 2), RED) && is(px(&f, w, 65, 117), RED));
}

// covers: REQ-126
#[test]
fn the_blurred_fill_has_no_sharp_edge_and_the_picture_stays_sharp() {
    let Some(env) = setup() else { return };
    let file = edge(&env);
    let mut p = project_of(&env, &file);
    p.timeline
        .set_frame_aspect(Some(aspect(Shape::Tall9x16, Fit::Width, Fill::Blur)))
        .unwrap();
    let out = export(&env, &p, "blur.mp4");
    let (w, h) = size(&env, &out);
    assert_eq!((w, h), (160, 284));
    let f = env.frame_rgb(&out, 1.0);
    // the picture (rows 82..202) keeps its sharp edge in the middle
    let sharp = (px(&f, w, 78, 142)[0] - px(&f, w, 82, 142)[0]).abs();
    assert!(sharp > 200, "{sharp}");
    // the empty rows are a blurred copy: bright on the left, dark on the right, and the
    // step at the middle is gentle
    let (l, r) = (px(&f, w, 10, 40)[0], px(&f, w, 150, 40)[0]);
    assert!(l > 150 && r < 100, "{l} {r}");
    let step = (px(&f, w, 79, 40)[0] - px(&f, w, 81, 40)[0]).abs();
    assert!(step < 60, "{step}");
}

// covers: REQ-127
#[test]
fn scale_makes_the_picture_smaller_and_shows_the_background() {
    let Some(env) = setup() else { return };
    let file = flat(&env, "red");
    let mut p = project_of(&env, &file);
    p.timeline.add_scale(FrameTarget::Whole).unwrap();
    p.timeline
        .set_scale_key(
            FrameTarget::Whole,
            Micros::ZERO,
            Scale {
                permille: 500,
                dx: 0,
                dy: 0,
            },
        )
        .unwrap();
    let out = export(&env, &p, "half.mp4");
    assert_eq!(size(&env, &out), (160, 120));
    let f = env.frame_rgb(&out, 1.0);
    assert!(is(px(&f, 160, 80, 60), RED));
    assert!(is(px(&f, 160, 5, 5), BLACK));
    assert!(is(px(&f, 160, 155, 115), BLACK));
    let (a, b) = red_run(&f, 160, 60);
    assert!((b - a + 1).abs_diff(80) <= 2, "{a}..{b}");
}

// covers: REQ-128
#[test]
fn move_shifts_the_picture_by_the_fraction_of_the_frame() {
    let Some(env) = setup() else { return };
    let file = flat(&env, "red");
    let mut p = project_of(&env, &file);
    p.timeline.add_move(FrameTarget::Whole).unwrap();
    p.timeline
        .set_move_key(FrameTarget::Whole, Micros::ZERO, Pos { x: 6000, y: 5000 })
        .unwrap();
    let out = export(&env, &p, "moved.mp4");
    let f = env.frame_rgb(&out, 1.0);
    // 10% of 160 px = 16 px to the right: the left strip is background
    let (a, b) = red_run(&f, 160, 60);
    assert!(a.abs_diff(16) <= 2, "{a}");
    assert_eq!(b, 159);
    assert!(is(px(&f, 160, 8, 60), BLACK));
}

// covers: REQ-129
#[test]
fn keyframes_change_the_scale_in_a_straight_line_and_hold_at_the_ends() {
    let Some(env) = setup() else { return };
    let file = flat(&env, "red");
    let mut p = project_of(&env, &file);
    let c = FrameTarget::Clip(p.timeline.clips[0].id);
    p.timeline.add_scale(c).unwrap();
    let sc = |permille| Scale {
        permille,
        dx: 0,
        dy: 0,
    };
    p.timeline.set_scale_key(c, Micros::ZERO, sc(500)).unwrap();
    p.timeline
        .add_scale_key(c, Micros(2_000_000), sc(1000))
        .unwrap();
    let out = export(&env, &p, "keys.mp4");
    let width = |t: f64| {
        let f = env.frame_rgb(&out, t);
        let (a, b) = red_run(&f, 160, 60);
        b - a + 1
    };
    // 50% at 0 s, 75% at 1 s, 100% from 2 s on
    assert!(width(0.0).abs_diff(80) <= 3, "{}", width(0.0));
    assert!(width(1.0).abs_diff(120) <= 4, "{}", width(1.0));
    assert!(width(2.5).abs_diff(160) <= 2);
    assert!(width(3.5).abs_diff(160) <= 2);
}

// covers: REQ-131
#[test]
fn the_picture_is_composed_under_the_fades_and_a_360_export_ignores_it() {
    let Some(env) = setup() else { return };
    let file = flat(&env, "red");
    let mut p = project_of(&env, &file);
    p.timeline.add_scale(FrameTarget::Whole).unwrap();
    p.timeline
        .set_scale_key(
            FrameTarget::Whole,
            Micros::ZERO,
            Scale {
                permille: 500,
                dx: 0,
                dy: 0,
            },
        )
        .unwrap();
    p.timeline.set_edge_fades(true).unwrap();
    let out = export(&env, &p, "faded.mp4");
    // the very first frame is black everywhere (the background fades too)
    let f = env.frame_rgb(&out, 0.0);
    assert!(is(px(&f, 160, 80, 60), BLACK) && is(px(&f, 160, 5, 5), BLACK));
    assert!(is(px(&env.frame_rgb(&out, 2.0), 160, 80, 60), RED));
}
