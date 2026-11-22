// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of the order of all the effects of a clip (2.2.0, REQ-320, REQ-321): each
//! pair of rows swapped gives the picture the order says, and the export and the preview draw it
//! the same way (the two use one builder).

use super::*;
use videolore::base::pos::{POS_MAX, Pos};
use videolore::effect_order::Row;
use videolore::frame::{FrameAspect, Rgb, Scale, Shape};
use videolore::text::{Readability, TextItem};
use videolore::timeline::{ClipId, Crop, FrameTarget};

/// A 320 x 180 clip: red on the left half, blue on the right half.
fn red_blue(env: &Env) -> PathBuf {
    env.fixture(
        "red-blue-320x180-3.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=160x180:r=10:d=3",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=160x180:r=10:d=3",
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

fn project(env: &Env) -> (Project, ClipId) {
    let mut p = Project::default();
    let m = env.probe(&mut p, &red_blue(env));
    p.media[0].format.bit_rate = Some(20_000_000);
    p.timeline.append(&m).unwrap();
    let c = p.timeline.clips[0].id;
    (p, c)
}

fn scale(p: &mut Project, c: ClipId, permille: u16) {
    let t = FrameTarget::Clip(c);
    p.timeline.add_scale(t).unwrap();
    let at = p.timeline.scale_keys(t)[0].t;
    p.timeline
        .set_scale_key(
            t,
            at,
            Scale {
                permille,
                dx: 0,
                dy: 0,
            },
        )
        .unwrap();
}

/// Moves the picture a quarter of the frame to the right.
fn move_right(p: &mut Project, c: ClipId) {
    let t = FrameTarget::Clip(c);
    p.timeline.add_move(t).unwrap();
    let at = p.timeline.move_keys(t)[0].t;
    p.timeline
        .set_move_key(
            t,
            at,
            Pos {
                x: POS_MAX / 2 + POS_MAX / 4,
                y: POS_MAX / 2,
            },
        )
        .unwrap();
}

fn invert_cube(env: &Env) -> String {
    let mut t = String::from("TITLE \"invert\"\nLUT_3D_SIZE 2\n");
    for b in 0..2u8 {
        for g in 0..2u8 {
            for r in 0..2u8 {
                t.push_str(&format!("{} {} {}\n", 1 - r, 1 - g, 1 - b));
            }
        }
    }
    let p = env.dir.path().join("invert.cube");
    std::fs::write(&p, t).unwrap();
    p.to_string_lossy().into_owned()
}

/// A frame as RGB with its size.
struct Shot {
    rgb: Vec<u8>,
    w: usize,
    h: usize,
}

impl Shot {
    fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y.min(self.h - 1) * self.w + x.min(self.w - 1)) * 3;
        [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
    }

    /// The pixel at the fraction (`fx`, `fy`) of the picture.
    fn f(&self, fx: f64, fy: f64) -> [u8; 3] {
        self.at((self.w as f64 * fx) as usize, (self.h as f64 * fy) as usize)
    }
}

fn is(p: [u8; 3], want: [u8; 3]) -> bool {
    p.iter()
        .zip(want)
        .all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() < 40)
}

const RED: [u8; 3] = [255, 0, 0];
const BLUE: [u8; 3] = [0, 0, 255];
const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const CYAN: [u8; 3] = [0, 255, 255];

fn export_shot(env: &Env, p: &Project, name: &str, t: f64) -> Shot {
    let (r, _) = env.export(p, &env.dir.path().join(name));
    let out = r.unwrap();
    let probe = Command::new(&env.info.ffprobe)
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
        .arg(&out)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&probe.stdout).trim().to_string();
    let (w, h) = text.split_once(',').unwrap();
    Shot {
        rgb: env.frame_rgb(&out, t),
        w: w.parse().unwrap(),
        h: h.parse().unwrap(),
    }
}

fn preview_shot(env: &Env, p: &Project, at: Micros) -> Shot {
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    pv.seek(p, at, Instant::now()).unwrap();
    let f = super::v20::first_frame(&mut pv);
    Shot {
        rgb: f.rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect(),
        w: f.width as usize,
        h: f.height as usize,
    }
}

/// Both renders of the picture at `secs`: they must agree (REQ-131, REQ-321).
fn both(env: &Env, p: &Project, name: &str, secs: f64) -> (Shot, Shot) {
    let e = export_shot(env, p, name, secs);
    let v = preview_shot(env, p, Micros((secs * 1_000_000.0) as i64));
    // the same picture: about every pixel has the same colour (the two are scaled and encoded
    // differently, so the edges of a shape may differ by a pixel or two)
    // (the preview is smaller than a tall export: compared on a grid of the picture)
    let grid = 48;
    let mut n = 0;
    for gy in 0..grid {
        for gx in 0..grid {
            let (fx, fy) = (
                (gx as f64 + 0.5) / grid as f64,
                (gy as f64 + 0.5) / grid as f64,
            );
            let (a, b) = (e.f(fx, fy), v.f(fx, fy));
            if a.iter().zip(b).any(|(x, y)| x.abs_diff(y) > 60) {
                n += 1;
            }
        }
    }
    assert!(
        n * 20 < grid * grid,
        "{n} of {} places differ between the export and the preview",
        grid * grid
    );
    (e, v)
}

fn tall(fill: Rgb) -> FrameAspect {
    FrameAspect {
        shape: Shape::Tall9x16,
        fit: videolore::frame::Fit::Width,
        fill: videolore::frame::Fill::Colour(fill),
        blur: Default::default(),
    }
}

// covers: REQ-320, REQ-321
#[test]
fn scale_then_move_and_move_then_scale_place_the_picture_apart_and_the_preview_agrees() {
    let Some(env) = setup() else { return };
    let (mut p, c) = project(&env);
    scale(&mut p, c, 500);
    move_right(&mut p, c);
    // the default: Scale, then Move: the half-size picture is at x 160..320, the border of its
    // red and blue halves at x = 240
    assert!(p.timeline.order_of(c).is_empty());
    let (e, v) = both(&env, &p, "sm.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.44, 0.5), BLACK), "left of the picture is black");
        assert!(is(s.f(0.6, 0.5), RED), "{:?}", s.f(0.6, 0.5));
        assert!(is(s.f(0.9, 0.5), BLUE));
    }
    // Move, then Scale (Scale above Move in the list): the moved layer is scaled about the
    // centre of the frame: the picture is at x 120..280, the border at x = 200
    p.set_order(c, vec![Row::Scale, Row::Move]);
    assert!(!p.timeline.order_of(c).is_empty());
    let (e, v) = both(&env, &p, "ms.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.44, 0.5), RED), "{:?}", s.f(0.44, 0.5));
        assert!(is(s.f(0.7, 0.5), BLUE), "{:?}", s.f(0.7, 0.5));
        assert!(is(s.f(0.1, 0.5), BLACK) && is(s.f(0.95, 0.5), BLACK));
    }
    // back to the default order: nothing stored and the first picture again
    p.set_order(c, vec![Row::Move, Row::Scale]);
    assert!(p.timeline.order_of(c).is_empty());
}

// covers: REQ-283
#[test]
fn a_crop_left_over_from_format_13_changes_nothing_in_preview_or_export() {
    let Some(env) = setup() else { return };
    let (mut p, c) = project(&env);
    scale(&mut p, c, 500);
    let (plain, _) = both(&env, &p, "c1.mp4", 1.0);
    // the field is read only to be dropped on opening: nothing draws it any more
    for clip in &mut p.timeline.clips {
        clip.crop = Crop {
            left: 25,
            ..Default::default()
        };
    }
    let (cropped, _) = both(&env, &p, "c2.mp4", 1.0);
    for (x, y) in [
        (0.1, 0.5),
        (0.2, 0.5),
        (0.37, 0.5),
        (0.5, 0.5),
        (0.6, 0.5),
        (0.9, 0.5),
    ] {
        assert_eq!(plain.f(x, y), cropped.f(x, y), "{x} {y}");
    }
}

// covers: REQ-320
#[test]
fn a_lut_above_change_aspect_grades_the_background_and_below_it_only_the_picture() {
    let Some(env) = setup() else { return };
    let (mut p, c) = project(&env);
    p.timeline
        .set_frame_aspect(Some(tall(Rgb(0, 0, 0))))
        .unwrap();
    p.timeline
        .set_lut(FrameTarget::Whole, Some(invert_cube(&env)))
        .unwrap();
    // the default: the LUT is under the frame: the picture is graded, the black bars are not
    assert!(p.timeline.order_of(c).is_empty());
    let (e, v) = both(&env, &p, "l1.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.5, 0.05), BLACK), "{:?}", s.f(0.5, 0.05));
        assert!(is(s.f(0.2, 0.5), CYAN), "{:?}", s.f(0.2, 0.5));
    }
    // the LUT above the frame grades the whole frame: the bars turn white
    p.set_order(c, vec![Row::Lut, Row::Frame]);
    let (e, v) = both(&env, &p, "l2.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.5, 0.05), WHITE), "{:?}", s.f(0.5, 0.05));
        assert!(is(s.f(0.2, 0.5), CYAN));
    }
    // a Scale above the frame scales the whole frame (the bars with it) and the LUT stays under
    // the frame: the corner is not covered any more (black), the bars are not graded
    scale(&mut p, c, 500);
    p.set_order(c, vec![Row::Scale, Row::Frame, Row::Lut]);
    let (e, v) = both(&env, &p, "l3.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.02, 0.02), BLACK), "{:?}", s.f(0.02, 0.02));
        assert!(is(s.f(0.5, 0.40), BLACK), "{:?}", s.f(0.5, 0.40));
    }
}

// covers: REQ-320
#[test]
fn a_fade_above_the_lut_keeps_the_start_black_and_below_it_the_lut_makes_it_white() {
    let Some(env) = setup() else { return };
    let (mut p, c) = project(&env);
    p.timeline.set_edge_fades(true).unwrap();
    p.timeline
        .set_lut(FrameTarget::Whole, Some(invert_cube(&env)))
        .unwrap();
    // the default: the fade is last: the first frame is black
    assert!(p.timeline.order_of(c).is_empty());
    let (e, v) = both(&env, &p, "f1.mp4", 0.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.5, 0.5), BLACK), "{:?}", s.f(0.5, 0.5));
    }
    // the LUT above the fade grades the black: white
    p.set_order(c, vec![Row::Lut, Row::Fade]);
    let (e, v) = both(&env, &p, "f2.mp4", 0.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.5, 0.5), WHITE), "{:?}", s.f(0.5, 0.5));
    }
}

// covers: REQ-320
#[test]
fn a_fade_below_change_aspect_fades_the_picture_only_and_above_it_the_background_too() {
    let Some(env) = setup() else { return };
    let (mut p, c) = project(&env);
    p.timeline.set_edge_fades(true).unwrap();
    p.timeline
        .set_frame_aspect(Some(tall(Rgb(0, 0, 255))))
        .unwrap();
    // the default: the fade is above the frame: the whole first frame is black
    let (e, v) = both(&env, &p, "g1.mp4", 0.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.5, 0.05), BLACK), "{:?}", s.f(0.5, 0.05));
    }
    // the frame above the fade: the picture fades, the blue background stays
    p.set_order(c, vec![Row::Frame, Row::Fade]);
    let (e, v) = both(&env, &p, "g2.mp4", 0.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.5, 0.05), BLUE), "{:?}", s.f(0.5, 0.05));
        assert!(is(s.f(0.5, 0.5), BLACK), "{:?}", s.f(0.5, 0.5));
    }
}

// covers: REQ-320
#[test]
fn a_blurred_background_is_made_from_the_picture_in_an_order_built_by_steps() {
    let Some(env) = setup() else { return };
    let (mut p, c) = project(&env);
    let mut aspect = tall(Rgb(0, 0, 0));
    aspect.fill = videolore::frame::Fill::Blur;
    p.timeline.set_frame_aspect(Some(aspect)).unwrap();
    scale(&mut p, c, 500);
    // the default: the picture is scaled over the blurred background: the corner is blurred
    // colour, not black
    let (e, _) = both(&env, &p, "b1.mp4", 1.0);
    assert!(!is(e.f(0.02, 0.02), BLACK), "{:?}", e.f(0.02, 0.02));
    // Scale above the frame: the whole frame, background included, is half the size and the
    // corner is black; inside the scaled frame the blurred background shows above the picture
    p.set_order(c, vec![Row::Scale, Row::Frame]);
    let (e, v) = both(&env, &p, "b2.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.02, 0.02), BLACK), "{:?}", s.f(0.02, 0.02));
        assert!(!is(s.f(0.3, 0.30), BLACK), "{:?}", s.f(0.3, 0.30));
    }
}

// covers: REQ-320, REQ-321
#[test]
fn a_360_clip_flat_camera_view_keeps_the_changed_order_in_the_export_and_the_preview() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let m = env.probe(
        &mut p,
        &env.clip("sphere-order.mp4", 640, 320, 10, 3, false),
    );
    p.media[0].format.spherical = Some(true);
    p.timeline.append(&m).unwrap();
    let c = p.timeline.clips[0].id;
    scale(&mut p, c, 500);
    move_right(&mut p, c);
    // Scale above Move: built from the camera view; both renders agree and the surround of the
    // half-size view is black
    p.set_order(c, vec![Row::Scale, Row::Move]);
    let (e, v) = both(&env, &p, "sph.mp4", 1.0);
    for s in [&e, &v] {
        assert!(is(s.f(0.05, 0.5), BLACK), "{:?}", s.f(0.05, 0.5));
        assert!(
            !is(s.f(0.6, 0.5), BLACK),
            "the view is drawn: {:?}",
            s.f(0.6, 0.5)
        );
    }
}

/// A big red text in the middle of the picture.
fn add_red_text(p: &mut Project) -> u64 {
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, "HHHH".into(), Micros::ZERO);
    t.place = Pos { x: 5000, y: 5000 };
    t.width = 1000;
    t.ttl = Micros(2_500_000);
    t.colour = [255, 0, 0];
    t.readability = Readability::None;
    p.texts.add(t).unwrap().0
}

// covers: REQ-320
#[test]
fn a_text_below_scale_is_scaled_with_the_picture_and_above_it_is_not() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        assert!(!required(), "the CI ffmpeg must have libass");
        return;
    }
    // a blue clip would hide a blue text: the text is red over the clip's blue half
    let (mut p, c) = project(&env);
    let t = add_red_text(&mut p);
    scale(&mut p, c, 500);
    // the default: the text is above Scale and keeps its size (the left half of the clip is red
    // too: only the text over the blue half is counted)
    let count_text = |s: &Shot| {
        let mut n = 0;
        for y in 0..s.h {
            for x in (s.w / 2 + 2)..s.w {
                let p = s.at(x, y);
                if p[0] > 200 && p[1] < 80 && p[2] < 80 {
                    n += 1;
                }
            }
        }
        n
    };
    let (e0, _) = both(&env, &p, "t0.mp4", 1.0);
    // Scale above the text: the text is drawn on the frame and scaled with it
    p.set_order(c, vec![Row::Scale, Row::Text(t)]);
    let (e1, _) = both(&env, &p, "t1.mp4", 1.0);
    let (n0, n1) = (count_text(&e0), count_text(&e1));
    assert!(n0 > 15, "the text is drawn in the default order: {n0}");
    assert!(n1 < n0, "the scaled text is smaller: {n1} against {n0}");
    // Change aspect above the text: the text is drawn on the layer (a matte) and then composed;
    // with a Scale between them it is scaled with the picture, over the blue background
    p.timeline
        .set_frame_aspect(Some(tall(Rgb(0, 0, 255))))
        .unwrap();
    p.set_order(c, vec![Row::Frame, Row::Scale, Row::Text(t)]);
    let (e2, v2) = both(&env, &p, "t2.mp4", 1.0);
    for s in [&e2, &v2] {
        assert!(
            is(s.f(0.5, 0.03), BLUE),
            "the background: {:?}",
            s.f(0.5, 0.03)
        );
    }
    // the text above the Scale (drawn after it, on the layer) keeps its size
    p.set_order(c, vec![Row::Frame, Row::Text(t), Row::Scale]);
    let (e3, _) = both(&env, &p, "t3.mp4", 1.0);
    let (n2, n3) = (count_text(&e2), count_text(&e3));
    assert!(n3 > 15, "the matte draws the text on the layer: {n3}");
    assert!(n2 < n3, "scaled with the picture: {n2} against {n3}");
}
