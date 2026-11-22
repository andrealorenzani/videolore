// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of overlap media (2.3.0): the transparency of a PNG, a GIF and a qtrle MOV
//! (committed fixtures in `tests/fixtures/overlay`) survives in the preview and in the export
//! at the same place; a GIF or a video plays once and is then transparent, a PNG stays; a
//! changed shape crops a video and stretches a picture (REQ-325, REQ-327, REQ-328, REQ-331).

use super::*;
use videolore::base::pos::Pos;
use videolore::overlap::{Kind, Overlap};
use videolore::timeline::ClipId;

/// A 320 x 180 clip, blue all over, 4 s.
fn blue(env: &Env) -> PathBuf {
    env.fixture(
        "blue-320x180-4.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=320x180:r=10:d=4",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ],
    )
}

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/overlay")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// The clip with one overlap of the fixture `name`: from 1 s for 2 s in a box of `w` x `h`
/// thousandths, in the middle of the picture.
fn project(env: &Env, name: &str, (w, h): (u16, u16), reshaped: bool) -> (Project, ClipId) {
    let mut p = Project::default();
    let m = env.probe(&mut p, &blue(env));
    p.media[0].format.bit_rate = Some(20_000_000);
    p.timeline.append(&m).unwrap();
    let clip = p.timeline.clips[0].id;
    let id = p.timeline.new_overlap_id();
    p.timeline
        .add_overlap(
            clip,
            Overlap {
                id,
                path: fixture(name),
                kind: if name.ends_with(".png") {
                    Kind::Still
                } else {
                    Kind::Motion
                },
                from: Micros(1_000_000),
                len: Micros(2_000_000),
                place: Pos { x: 5000, y: 5000 },
                width: w,
                height: h,
                reshaped,
                src_w: 64,
                src_h: 64,
            },
        )
        .unwrap();
    (p, clip)
}

struct Shot {
    rgb: Vec<u8>,
    w: usize,
    h: usize,
}

impl Shot {
    /// The pixel at the fraction (`fx`, `fy`) of the picture.
    fn f(&self, fx: f64, fy: f64) -> [u8; 3] {
        let x = ((self.w as f64 * fx) as usize).min(self.w - 1);
        let y = ((self.h as f64 * fy) as usize).min(self.h - 1);
        let i = (y * self.w + x) * 3;
        [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
    }
}

fn is(p: [u8; 3], want: [u8; 3]) -> bool {
    p.iter()
        .zip(want)
        .all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() < 60)
}

const RED: [u8; 3] = [255, 0, 0];
const BLUE: [u8; 3] = [0, 0, 255];

fn export_shot(env: &Env, p: &Project, name: &str, t: f64) -> Shot {
    let (r, _) = env.export(p, &env.dir.path().join(name));
    let out = r.unwrap();
    Shot {
        rgb: env.frame_rgb(&out, t),
        w: 320,
        h: 180,
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

/// What both renders show at `secs`: the middle of the picture, and a place inside the box where
/// the file is transparent.
fn middle_and_corner(e: &Shot) -> ([u8; 3], [u8; 3]) {
    (e.f(0.5, 0.5), e.f(0.375, 0.28))
}

// covers: REQ-325, REQ-331
#[test]
fn the_transparency_of_a_png_a_gif_and_a_mov_survives_in_the_preview_and_the_export() {
    let Some(env) = setup() else { return };
    for name in ["alpha.png", "alpha.gif", "alpha.mov"] {
        let (p, _) = project(&env, name, (300, 500), false);
        let e = export_shot(&env, &p, &format!("a-{name}.mp4"), 1.5);
        let v = preview_shot(&env, &p, Micros(1_500_000));
        for (what, s) in [("export", &e), ("preview", &v)] {
            let (middle, corner) = middle_and_corner(s);
            assert!(is(middle, RED), "{name} {what}: the red square: {middle:?}");
            assert!(
                is(corner, BLUE),
                "{name} {what}: the video shows through: {corner:?}"
            );
        }
        // before and after its time nothing of it is on the picture
        let before = export_shot(&env, &p, &format!("b-{name}.mp4"), 0.5);
        assert!(is(before.f(0.5, 0.5), BLUE), "{name}: before");
        let after = export_shot(&env, &p, &format!("c-{name}.mp4"), 3.5);
        assert!(is(after.f(0.5, 0.5), BLUE), "{name}: after");
    }
}

// covers: REQ-328
#[test]
fn a_one_second_animation_in_a_two_second_overlap_is_transparent_after_it_and_a_png_stays() {
    let Some(env) = setup() else { return };
    for name in ["alpha.gif", "alpha.mov"] {
        let (p, _) = project(&env, name, (300, 500), false);
        // the overlap is from 1 s to 3 s, the animation lasts 1 s: it plays from 1 s to 2 s
        let during = export_shot(&env, &p, &format!("d-{name}.mp4"), 1.5);
        assert!(is(during.f(0.5, 0.5), RED), "{name}: during the animation");
        let after = export_shot(&env, &p, &format!("e-{name}.mp4"), 2.5);
        assert!(
            is(after.f(0.5, 0.5), BLUE),
            "{name}: transparent after it: {:?}",
            after.f(0.5, 0.5)
        );
        // the preview that starts after the animation is transparent too
        let v = preview_shot(&env, &p, Micros(2_500_000));
        assert!(is(v.f(0.5, 0.5), BLUE), "{name}: preview after it");
        let v = preview_shot(&env, &p, Micros(1_500_000));
        assert!(is(v.f(0.5, 0.5), RED), "{name}: preview during it");
    }
    let (p, _) = project(&env, "alpha.png", (300, 500), false);
    let late = export_shot(&env, &p, "f.mp4", 2.5);
    assert!(
        is(late.f(0.5, 0.5), RED),
        "a PNG shows for the whole overlap"
    );
}

// covers: REQ-327
#[test]
fn a_changed_shape_crops_a_video_to_the_box_and_stretches_a_picture_to_it() {
    let Some(env) = setup() else { return };
    // a wide box (160 x 36 pixels in the middle); the file is a square with a red square in its
    // middle: the video fills the box from its middle (the box is red from x 120 to 200 over its
    // whole height); the picture is stretched (red only in the middle half of the height)
    let (p, _) = project(&env, "alpha.mov", (500, 200), true);
    let e = export_shot(&env, &p, "m.mp4", 1.5);
    let top_of_box = e.f(0.5, 76.0 / 180.0);
    assert!(
        is(top_of_box, RED),
        "video cropped to the box: {top_of_box:?}"
    );
    assert!(
        is(e.f(100.0 / 320.0, 0.5), BLUE),
        "left part of the box is transparent"
    );
    let (p, _) = project(&env, "alpha.png", (500, 200), true);
    let e = export_shot(&env, &p, "n.mp4", 1.5);
    assert!(is(e.f(0.5, 0.5), RED));
    let top_of_box = e.f(0.5, 76.0 / 180.0);
    assert!(
        is(top_of_box, BLUE),
        "picture stretched to the box: {top_of_box:?}"
    );
    // not changed: the same wide box keeps the ratio of the file (a square 36 pixels high), so
    // the red square is small and the sides of the box are the video's
    let (p, _) = project(&env, "alpha.png", (500, 200), false);
    let e = export_shot(&env, &p, "o.mp4", 1.5);
    assert!(is(e.f(0.5, 0.5), RED));
    assert!(is(e.f(0.45, 0.5), BLUE) || is(e.f(0.45, 0.5), RED));
    assert!(
        is(e.f(0.3, 0.5), BLUE),
        "outside the square nothing is drawn"
    );
}

/// The number and the centre of the pixels that have ink in `with` and none in `without` (two
/// gray pictures `w` wide): what one word adds to a picture. The centre of mass is not moved
/// by the few edge pixels that anti-aliasing turns on or off.
fn added_ink(with: &[u8], without: &[u8], w: usize) -> (usize, f64, f64) {
    let (mut n, mut sx, mut sy) = (0usize, 0.0, 0.0);
    for (i, (a, c)) in with.iter().zip(without).enumerate() {
        if *a > 128 && *c <= 128 {
            n += 1;
            sx += (i % w) as f64;
            sy += (i / w) as f64;
        }
    }
    (n, sx / n.max(1) as f64, sy / n.max(1) as f64)
}

// covers: REQ-333, REQ-309
#[test]
fn the_enlarged_word_keeps_its_centre_and_the_other_words_do_not_move_in_a_real_render() {
    use videolore::ass;
    use videolore::subtitles::{LineId, Preset, ShownLine, StyleSettings, Word};
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        return;
    }
    let w = |a: i64, b: i64, t: &str| Word {
        start: Micros(a * 1_000_000),
        end: Micros(b * 1_000_000),
        text: t.into(),
    };
    let line = ShownLine {
        line: LineId(1),
        clip: ClipId(1),
        start: Micros::ZERO,
        end: Micros(3_000_000),
        text: "mmmm wwww oooo".into(),
        words: vec![w(0, 1, "mmmm"), w(1, 2, "wwww"), w(2, 3, "oooo")],
        position: None,
        size: None,
        anchor: None,
    };
    // the document 1.5 s in (the middle word is the one that is spoken), as the 2.3.0 build
    // writes it
    let doc = |enlarge: u8| {
        let st = StyleSettings {
            enlarge,
            ..StyleSettings::default()
        };
        ass::document_styled(
            std::slice::from_ref(&line),
            Preset::EnlargeWord,
            &st,
            1920,
            1080,
            Micros(1_500_000),
            &[],
        )
    };
    let render =
        |doc: &str| super::v204::render_ass_sized(&env, doc, &ass::BUNDLED_FACES, (1920, 1080));
    let base_only = |d: &str| {
        // without the events of the enlarged word (they have their own place)
        d.lines()
            .filter(|l| !l.contains("\\an5\\pos("))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let enlarged = doc(12);
    assert!(enlarged.contains("\\an5\\pos("), "{enlarged}");
    // the line with the other words only, the line as it is at normal size (the hidden word shown
    // and no enlarged event), and the enlarged word on top of the base line
    let others = render(&base_only(&enlarged));
    let normal = render(
        &base_only(&enlarged)
            .replace("{\\alpha&HFF&}", "")
            .replace("{\\alpha&H00&}", ""),
    );
    let big = render(&enlarged);
    // the other words did not move: the enlarged picture has all their ink
    let lost = others
        .iter()
        .zip(&big)
        .filter(|(o, b)| **o > 128 && **b <= 128)
        .count();
    // (the outline of the larger word may cover a few pixels at the edge of its neighbours)
    let ink = others.iter().filter(|o| **o > 128).count();
    assert!(
        lost * 20 < ink,
        "{lost} of {ink} pixels of the other words are gone"
    );
    // the word at its normal size and enlarged: the same centre, and larger
    let (nb, eb) = (
        added_ink(&normal, &others, 1920),
        added_ink(&big, &others, 1920),
    );
    assert!(nb.0 > 500, "the word at normal size has ink: {nb:?}");
    assert!(eb.0 > nb.0 + nb.0 / 5, "larger: {nb:?} {eb:?}");
    // the plan allows 2% of the height (21 px); the measured place is within 4 px
    assert!((nb.1 - eb.1).abs() <= 4.0, "x: {nb:?} {eb:?}");
    assert!((nb.2 - eb.2).abs() <= 4.0, "y: {nb:?} {eb:?}");
}
