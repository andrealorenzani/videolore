// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 2.0.3: readable text (size measured with the font, the size cap,
//! the outline and the background box, installed and missing fonts) and subtitle placement.

use super::*;
use videolore::base::pos::Pos;
use videolore::text::{Readability, TextId, TextItem};

include!("../support/patched_font.rs");

const W: usize = 640;
const H: usize = 360;

fn clip(env: &Env, colour: &str) -> PathBuf {
    env.fixture(
        &format!("{colour}-{W}x{H}-3.mp4"),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s={W}x{H}:r=10:d=3"),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ],
    )
}

fn project_of(env: &Env, colour: &str) -> Project {
    let mut p = Project::default();
    let m = env.probe(&mut p, &clip(env, colour));
    // a plain clip has a tiny bit rate: the export would blur the text away
    p.media[0].format.bit_rate = Some(20_000_000);
    p.timeline.append(&m).unwrap();
    p
}

fn add_text(p: &mut Project, f: impl FnOnce(&mut TextItem)) -> TextId {
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, "HHHH".into(), Micros::ZERO);
    t.place = Pos { x: 5000, y: 5000 };
    t.width = 1000;
    t.ttl = Micros(2_500_000);
    f(&mut t);
    p.texts.add(t).unwrap()
}

fn can_burn(env: &Env) -> bool {
    if !env.info.has_subtitles() {
        assert!(!required(), "the CI ffmpeg must have libass");
        return false;
    }
    true
}

/// Bounding box (x0, x1, y0, y1) of the white pixels (green above 128) of an RGB frame.
fn ink(frame: &[u8], w: usize) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut x1, mut y0, mut y1, mut n) = (usize::MAX, 0, usize::MAX, 0, 0);
    for (i, p) in frame.chunks(3).enumerate() {
        if p[1] > 128 {
            let (x, y) = (i % w, i / w);
            (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
            n += 1;
        }
    }
    (n > 5).then_some((x0, x1, y0, y1))
}

/// Number of pixels darker than `limit` (luma from green) in an RGB frame.
fn darker_than(frame: &[u8], limit: u8) -> usize {
    frame.chunks(3).filter(|p| p[1] < limit).count()
}

fn export_frame(env: &Env, p: &Project, name: &str, t: f64) -> Vec<u8> {
    let (r, _) = env.export(p, &env.dir.path().join(name));
    env.frame_rgb(&r.unwrap(), t)
}

// covers: REQ-156, REQ-157
#[test]
fn the_size_is_measured_with_the_font_and_capped_at_ten_percent_of_the_height() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    // a line of H letters: the drawn width is the measured one (within 2% of the picture)
    let mut p = project_of(&env, "black");
    let id = add_text(&mut p, |t| t.text = "H".repeat(20));
    let item = p.texts.get(id).unwrap().clone();
    let l = videolore::text::layout(&item, W as u32, H as u32);
    assert_eq!(l.lines.len(), 1);
    let f = export_frame(&env, &p, "ink.mp4", 1.0);
    let (x0, x1, y0, y1) = ink(&f, W).expect("the text");
    let drawn = (x1 - x0 + 1) as f64;
    assert!(
        (drawn - l.width).abs() <= 0.02 * W as f64,
        "drawn {drawn} px, measured {} px",
        l.width
    );
    // one line in the default box is capped at 10% of the picture height
    assert_eq!(l.px, H as f64 / 10.0);
    // the cap height of the capital letters is about half of the line height
    let cap = (y1 - y0 + 1) as f64;
    assert!((cap - 0.5246 * l.px).abs() <= 0.02 * H as f64, "{cap}");
    // "Hi": a short text in the default box is drawn at 10% of the picture height
    let mut q = project_of(&env, "black");
    add_text(&mut q, |t| t.text = "H".into());
    let f = export_frame(&env, &q, "hi.mp4", 1.0);
    let (_, _, y0, y1) = ink(&f, W).expect("the letter");
    let px = (y1 - y0 + 1) as f64 / 0.5246;
    assert!((px - H as f64 / 10.0).abs() <= 0.02 * H as f64, "{px}");
    // a long text is still fitted: it stays inside its box
    let mut r = project_of(&env, "black");
    add_text(&mut r, |t| {
        t.text = "The quick brown fox jumps over the lazy dog and keeps running far away".into();
        t.width = 500;
    });
    let f = export_frame(&env, &r, "long.mp4", 1.0);
    let (x0, x1, y0, y1) = ink(&f, W).expect("the text");
    assert!(x1 - x0 <= W / 2 + 4, "inside the box width");
    assert!(y1 - y0 <= H / 5 + 4, "inside the box height");
}

// covers: REQ-156
#[test]
fn the_preview_draws_the_text_as_large_as_the_export() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, "black");
    add_text(&mut p, |t| t.text = "WWWW iiii WWWW\nmmmm llll".into());
    let f = export_frame(&env, &p, "same.mp4", 1.0);
    let (x0, x1, y0, y1) = ink(&f, W).expect("the export");
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    pv.seek(&p, Micros(1_000_000), Instant::now()).unwrap();
    let shot = super::v20::first_frame(&mut pv);
    let rgb: Vec<u8> = shot
        .rgba
        .chunks(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let (px0, px1, py0, py1) = ink(&rgb, shot.width as usize).expect("the preview");
    let frac = |v: usize, size: usize| v as f64 / size as f64;
    for (a, b, what) in [
        (frac(x0, W), frac(px0, shot.width as usize), "left"),
        (frac(x1, W), frac(px1, shot.width as usize), "right"),
        (frac(y0, H), frac(py0, shot.height as usize), "top"),
        (frac(y1, H), frac(py1, shot.height as usize), "bottom"),
    ] {
        assert!((a - b).abs() <= 0.02, "{what}: export {a}, preview {b}");
    }
}

// covers: REQ-158
#[test]
fn outline_and_background_box_are_drawn_and_none_draws_nothing() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let dark = |r: Readability, name: &str| {
        let mut p = project_of(&env, "white");
        add_text(&mut p, |t| {
            t.colour = [255, 255, 255];
            t.readability = r;
        });
        export_frame(&env, &p, name, 1.0)
    };
    // white text on a plain white frame
    let none = dark(Readability::None, "r-none.mp4");
    assert_eq!(darker_than(&none, 200), 0, "no outline, no box");
    let outline = dark(Readability::Outline, "r-out.mp4");
    assert!(
        darker_than(&outline, 100) > 200,
        "dark pixels at the letter edges"
    );
    let boxed = dark(Readability::Box, "r-box.mp4");
    // the area behind the text is at least 50% darker than the frame: the padding at the
    // left of the first letter and a point between the letters
    let dark_px: Vec<usize> = boxed
        .chunks(3)
        .enumerate()
        .filter(|(_, p)| p[1] < 140)
        .map(|(i, _)| i)
        .collect();
    assert!(dark_px.len() > 2000, "a box: {}", dark_px.len());
    let (x0, y) = (dark_px.iter().map(|i| i % W).min().unwrap(), H / 2);
    let at = |x: usize, y: usize| boxed[(y * W + x) * 3 + 1];
    assert!(at(x0 + 2, y) <= 127, "behind the text: {}", at(x0 + 2, y));
    // the frame outside the box is still white
    assert!(at(2, 2) > 240);
}

// covers: REQ-162
#[test]
fn a_blinking_text_hides_its_box_and_outline_with_it() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, "white");
    add_text(&mut p, |t| {
        t.readability = Readability::Box;
        t.effects.blink = Some(Micros(500_000));
    });
    let (r, _) = env.export(&p, &env.dir.path().join("blink.mp4"));
    let out = r.unwrap();
    let shown = env.frame_rgb(&out, 0.25);
    let hidden = env.frame_rgb(&out, 0.75);
    assert!(darker_than(&shown, 140) > 2000, "a box at 0.25 s");
    assert_eq!(darker_than(&hidden, 200), 0, "no box at 0.75 s");
}

// covers: REQ-163, REQ-165
#[test]
fn an_installed_font_is_the_one_drawn_and_it_is_measured_for_its_box() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    // a font installed for the test (not known to the system): only the copy that the
    // export puts next to the bundled font lets the renderer find it
    let fonts = env.dir.path().join("fontsrc");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::write(fonts.join("wide.ttf"), patched_font("Wide Face", 900)).unwrap();
    videolore::fonts::install_dir(&fonts);
    let text = "Hello wide world, a text that fills its box and goes on and on, so that it needs several lines and its size is set by the width".to_string();
    let drawn = |font: &str, name: &str| {
        let mut p = project_of(&env, "black");
        let id = add_text(&mut p, |t| {
            t.text = text.clone();
            t.font = font.into();
            t.width = 600;
        });
        let item = p.texts.get(id).unwrap().clone();
        let f = export_frame(&env, &p, name, 1.0);
        (
            videolore::text::layout(&item, W as u32, H as u32),
            ink(&f, W).unwrap(),
        )
    };
    let (wide, wink) = drawn("Wide Face", "wide.mp4");
    let (plain, pink) = drawn("", "plain.mp4");
    // measured with the installed face: a smaller size for the same box, and it fits
    assert!(wide.px < plain.px, "{} {}", wide.px, plain.px);
    assert!(wide.fits);
    for (l, (x0, x1, y0, y1)) in [(&wide, wink), (&plain, pink)] {
        let widest = (x1 - x0 + 1) as f64;
        assert!(
            (widest - l.width).abs() <= 0.02 * W as f64,
            "drawn {widest}, measured {}",
            l.width
        );
        assert!((y1 - y0 + 1) as f64 <= l.height() + 2.0, "inside its lines");
        assert!(widest <= 0.6 * W as f64 + 2.0, "inside the box");
    }
    // at the same size the installed font is wider: the glyphs are not the bundled ones
    assert!(
        (wink.1 - wink.0) as f64 / wide.px > 1.1 * (pink.1 - pink.0) as f64 / plain.px,
        "the glyphs of the installed font are drawn"
    );
}

// covers: REQ-164
#[test]
fn a_missing_font_is_drawn_in_the_bundled_font_and_the_project_keeps_its_name() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut a = project_of(&env, "black");
    add_text(&mut a, |t| t.font = "Not A Real Face".into());
    let mut b = project_of(&env, "black");
    add_text(&mut b, |t| t.font = String::new());
    let fa = export_frame(&env, &a, "missing.mp4", 1.0);
    let fb = export_frame(&env, &b, "bundled.mp4", 1.0);
    assert_eq!(
        ink(&fa, W),
        ink(&fb, W),
        "the same picture as the bundled font"
    );
    assert_eq!(a.texts.items[0].font, "Not A Real Face");
    // saved and opened again, the name is still there
    let file = env.dir.path().join("missing.vlproj");
    project::save(&a, &file).unwrap();
    let (back, _) = project::load(&file).unwrap();
    assert_eq!(back.texts.items[0].font, "Not A Real Face");
}

// covers: REQ-174
#[test]
fn a_second_line_of_a_subtitle_keeps_the_centre_of_its_box_in_the_export() {
    use videolore::placement;
    use videolore::subtitles::NewLine;
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, "black");
    let media = p.media[0].id;
    let line = |text: &str| NewLine {
        media,
        start: Micros::ZERO,
        end: Micros(2_500_000),
        text: text.into(),
        words: vec![],
    };
    p.subtitles.replace_all(vec![line("Hello there")], None);
    let id = p.subtitles.lines[0].id;
    let frame = (W as u32, H as u32);
    let old = {
        let l = p.subtitles.get(id).unwrap();
        placement::line_rect(p.subtitles.preset, &l.text, l.position, l.size, frame)
    };
    // the text gets a second line (it wraps): the box keeps its centre
    let long = "Hello there, this is a subtitle that is long enough to need a second line, and then a few more words to be sure";
    let pos = placement::place_after_text_edit(&p, id, long, frame).expect("an own place");
    p.subtitles.set_text(id, long).unwrap();
    p.subtitles.set_position(&[id], Some(pos)).unwrap();
    let l = p.subtitles.get(id).unwrap();
    let new = placement::line_rect(p.subtitles.preset, &l.text, l.position, l.size, frame);
    assert!(new.h > old.h * 1.5, "two lines: {} {}", new.h, old.h);
    assert!(
        (new.cy - old.cy).abs() <= 0.005 * H as f64 && (new.cx - old.cx).abs() <= 0.005 * W as f64
    );
    // the export draws it around that centre, not from the bottom of the style's place
    let f = export_frame(&env, &p, "centre.mp4", 1.0);
    let (x0, x1, y0, y1) = ink(&f, W).expect("the subtitle");
    let (cx, cy) = ((x0 + x1) as f64 / 2.0, (y0 + y1) as f64 / 2.0);
    assert!(
        (cy - old.cy).abs() <= 0.03 * H as f64,
        "ink centre {cy}, box centre {}",
        old.cy
    );
    assert!((cx - old.cx).abs() <= 0.03 * W as f64, "{cx} {}", old.cx);
}

/// Bounding box (x0, x1, y0, y1) and count of the red pixels of an RGB frame.
fn red_ink(frame: &[u8], w: usize) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut x1, mut y0, mut y1, mut n) = (usize::MAX, 0, usize::MAX, 0, 0);
    for (i, p) in frame.chunks(3).enumerate() {
        if p[0] > 150 && p[1] < 90 && p[2] < 90 {
            let (x, y) = (i % w, i / w);
            (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
            n += 1;
        }
    }
    (n > 5).then_some((x0, x1, y0, y1))
}

// covers: REQ-306
#[test]
fn a_red_shadow_is_drawn_down_and_right_of_the_letters_in_the_export_and_zero_draws_none() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let shadowed = |px: u8, name: &str| {
        let mut p = project_of(&env, "black");
        add_text(&mut p, |t| {
            t.colour = [255, 255, 255];
            t.readability = Readability::None;
            t.shadow = videolore::base::shadow::Shadow {
                px,
                colour: [255, 0, 0],
            };
        });
        export_frame(&env, &p, name, 1.0)
    };
    let plain = shadowed(0, "sh0.mp4");
    assert!(red_ink(&plain, W).is_none(), "no shadow, no red");
    // 8 px at 1080 is 2.7 px on this 360 picture
    let f = shadowed(8, "sh8.mp4");
    let white = ink(&f, W).expect("the letters");
    let red = red_ink(&f, W).expect("the shadow");
    let (dx, dy) = (red.1 as i32 - white.1 as i32, red.3 as i32 - white.3 as i32);
    // within 2% of the frame (7 px) of the distance, and on the right and below
    assert!(
        (1..=6).contains(&dx),
        "shadow edge {dx} px right of the letters"
    );
    assert!(
        (1..=6).contains(&dy),
        "shadow edge {dy} px below the letters"
    );
}

/// A project with one subtitle line of `text` whose words come one per second.
fn spoken_project(env: &Env, text: &str, preset: videolore::subtitles::Preset) -> Project {
    use videolore::subtitles::{NewLine, Word};
    let mut p = project_of(env, "black");
    let media = p.media[0].id;
    let words: Vec<Word> = text
        .split(' ')
        .enumerate()
        .map(|(i, w)| Word {
            start: Micros(i as i64 * 1_000_000),
            end: Micros((i as i64 + 1) * 1_000_000),
            text: w.into(),
        })
        .collect();
    let end = Micros(words.len() as i64 * 1_000_000);
    p.subtitles.replace_all(
        vec![NewLine {
            media,
            start: Micros::ZERO,
            end,
            text: text.into(),
            words,
        }],
        None,
    );
    p.subtitles.preset = preset;
    p
}

// covers: REQ-310
#[test]
fn typewriter_shows_the_letters_of_the_word_on_their_times_and_not_the_next_words() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let p = spoken_project(
        &env,
        "ab WORDS zz",
        videolore::subtitles::Preset::Typewriter,
    );
    let (r, _) = env.export(&p, &env.dir.path().join("tw.mp4"));
    let out = r.unwrap();
    let count = |t: f64| ink(&env.frame_rgb(&out, t), W).map_or(0, |b| b.1 - b.0);
    // 1.1 s: "ab" is spoken and the first letter of WORDS (5 letters over 1 s) is typed
    let (early, mid, end) = (count(1.1), count(1.5), count(1.95));
    assert!(early < mid && mid < end, "{early} {mid} {end}");
    // the word after it is not drawn yet at 1.95 s: the ink stays narrower than the line
    let full = count(2.5);
    assert!(end < full, "{end} < {full}");
    // the preview draws the same letters at the same times (same graph and ASS)
    let still = |t: f64| {
        let eg = render::output_geometry(&p, &env.info.encoders).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let r = render::still_args(
            &p,
            Micros::from_secs_f64(t),
            &eg,
            env.info.version,
            &dir.path().join("g.txt"),
            render::Source::Export,
        )
        .unwrap();
        render::write_command_files(dir.path(), &r.command_files, &r.lut_files).unwrap();
        std::fs::write(
            dir.path().join(videolore::ass::ASS_FILE),
            r.subtitles.as_ref().unwrap(),
        )
        .unwrap();
        let o = Command::new(&env.info.ffmpeg)
            .args(&r.args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        // RGBA bytes: the width of the letters' ink (green above 128)
        let w = eg.preview().width as usize;
        let (mut lo, mut hi) = (usize::MAX, 0);
        for (i, p) in o.stdout.chunks(4).enumerate() {
            if p[1] > 128 {
                lo = lo.min(i % w);
                hi = hi.max(i % w);
            }
        }
        hi.saturating_sub(lo)
    };
    let (a, b) = (still(1.1), still(1.95));
    assert!(a > 0 && a < b, "preview {a} < {b}");
}

// covers: REQ-309
#[test]
fn enlarge_word_draws_the_spoken_word_larger_and_leaves_the_others_where_they_are() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = spoken_project(&env, "HH HH HH", videolore::subtitles::Preset::EnlargeWord);
    p.subtitles.style.enlarge = 40;
    let (r, _) = env.export(&p, &env.dir.path().join("en.mp4"));
    let out = r.unwrap();
    let height_of = |t: f64, x0: usize, x1: usize| {
        let f = env.frame_rgb(&out, t);
        let sub: Vec<u8> = (0..H)
            .flat_map(|y| f[(y * W + x0) * 3..(y * W + x1) * 3].to_vec())
            .collect();
        ink(&sub, x1 - x0).map_or(0, |b| b.3 - b.2)
    };
    let f = env.frame_rgb(&out, 0.5);
    let (a0, a1, _, _) = ink(&f, W).expect("the line");
    // the first word is spoken at 0.5 s: it is taller than the last one, which is not
    let third = a1 - (a1 - a0) / 5;
    let first = height_of(0.5, a0, a0 + (a1 - a0) / 4);
    let last = height_of(0.5, third, a1 + 1);
    assert!(first > last + 3, "enlarged {first} vs {last}");
    // at 2.5 s the last word is the large one and the first is back to the normal size
    let first2 = height_of(2.5, a0, a0 + (a1 - a0) / 4);
    let last2 = height_of(2.5, third, a1 + 1);
    assert!(last2 > first2 + 3, "{last2} vs {first2}");
    assert!(first2.abs_diff(last) <= 2, "the others keep their size");
}

/// A 2x2x2 `.cube` file that inverts the colours.
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

/// How many pixels of an RGB frame are (strongly) red, green, and cyan.
fn colours(frame: &[u8]) -> (usize, usize, usize) {
    let mut n = (0, 0, 0);
    for p in frame.chunks(3) {
        let (r, g, b) = (p[0], p[1], p[2]);
        if r > 200 && g < 80 && b < 80 {
            n.0 += 1;
        } else if g > 200 && r < 80 && b < 80 {
            n.1 += 1;
        } else if r < 80 && g > 200 && b > 200 {
            n.2 += 1;
        }
    }
    n
}

fn preview_rgb(env: &Env, p: &Project, at: Micros) -> Vec<u8> {
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    pv.seek(p, at, Instant::now()).unwrap();
    let shot = super::v20::first_frame(&mut pv);
    shot.rgba
        .chunks(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect()
}

// covers: REQ-304
// covers: REQ-311
#[test]
fn a_text_above_the_lut_keeps_its_colours_and_below_it_is_graded_in_the_export_and_the_preview() {
    use videolore::effect_order::Row;
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, "black");
    let id = add_text(&mut p, |t| {
        t.colour = [255, 0, 0];
        t.readability = Readability::None;
    });
    let lut = invert_cube(&env);
    p.timeline
        .set_lut(videolore::timeline::FrameTarget::Whole, Some(lut))
        .unwrap();
    let clip = p.timeline.clips[0].id;
    // the old order: the text above the LUT keeps its own red on the inverted (white) frame
    assert!(p.timeline.clip_order.is_empty());
    let (red, _, cyan) = colours(&export_frame(&env, &p, "o-old.mp4", 1.0));
    assert!(red > 100 && cyan == 0, "red {red}, cyan {cyan}");
    // the LUT above the text grades the text too: cyan on white
    p.set_order(clip, vec![Row::Lut, Row::Text(id.0)]);
    let (red, _, cyan) = colours(&export_frame(&env, &p, "o-new.mp4", 1.0));
    assert!(cyan > 100 && red == 0, "red {red}, cyan {cyan}");
    // the preview draws the same (parity of the two renders)
    let (pr, _, pc) = colours(&preview_rgb(&env, &p, Micros(1_000_000)));
    assert!(pc > 100 && pr == 0, "preview red {pr}, cyan {pc}");
    // and the order saved with the project is read back
    let json = videolore::project::to_json(&p, &env.dir.path().join("x.vlproj")).unwrap();
    let (q, _) = videolore::project::from_json(&json, env.dir.path()).unwrap();
    assert_eq!(q.timeline.clip_order, p.timeline.clip_order);
}

// covers: REQ-305
#[test]
fn the_lower_of_two_overlapping_texts_is_covered_and_moving_a_row_swaps_them() {
    use videolore::effect_order::Row;
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, "black");
    let a = add_text(&mut p, |t| {
        t.colour = [255, 0, 0];
        t.readability = Readability::None;
    });
    let b = add_text(&mut p, |t| {
        t.colour = [0, 255, 0];
        t.readability = Readability::None;
    });
    let clip = p.timeline.clips[0].id;
    let check = |p: &Project, name: &str, top: usize| {
        for (what, rgb) in [
            ("export", export_frame(&env, p, name, 1.0)),
            ("preview", preview_rgb(&env, p, Micros(1_000_000))),
        ] {
            let (r, g, _) = colours(&rgb);
            let (shown, hidden) = if top == 0 { (r, g) } else { (g, r) };
            assert!(
                shown > 100,
                "{what}: the top text shows ({r} red, {g} green)"
            );
            assert!(
                hidden * 10 < shown,
                "{what}: the lower is covered ({r}, {g})"
            );
        }
    };
    // the older order: the later item (green) on top
    check(&p, "ov-old.mp4", 1);
    // A (red) above B: A covers B
    p.set_order(clip, vec![Row::Text(a.0), Row::Text(b.0)]);
    check(&p, "ov-a.mp4", 0);
    // moved down: B covers A again, which is the older order
    p.set_order(clip, vec![Row::Text(b.0), Row::Text(a.0)]);
    assert!(p.timeline.clip_order.is_empty());
    check(&p, "ov-b.mp4", 1);
}

// covers: REQ-304
#[test]
fn a_picture_item_over_two_clips_follows_the_order_of_each_clip_in_the_export_and_the_preview() {
    use videolore::effect_order::Row;
    let Some(env) = setup() else { return };
    let png = env.fixture(
        "red-64.png",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x64:d=1",
            "-frames:v",
            "1",
        ],
    );
    let mut p = project_of(&env, "black");
    let m = p.media[0].clone();
    p.timeline.append(&m).unwrap();
    let inv = invert_cube(&env);
    p.timeline
        .set_lut(videolore::timeline::FrameTarget::Whole, Some(inv))
        .unwrap();
    let id = p.texts.new_id();
    let pic = videolore::picture::read(&png).unwrap();
    let mut item = TextItem::new_picture(id, pic, Micros(1_000_000), (W as u32, H as u32));
    item.ttl = Micros(4_000_000);
    p.texts.add(item).unwrap();
    let (c1, c2) = (p.timeline.clips[0].id, p.timeline.clips[1].id);
    // the picture was added after the LUTs: on clip 2 the LUT goes above it (graded cyan on
    // white); clip 1 keeps the old order (the picture above the LUT keeps its red)
    p.set_order(c2, vec![Row::Lut, Row::Picture(id.0)]);
    assert!(p.timeline.order_of(c1).is_empty());
    let (r, _) = env.export(&p, &env.dir.path().join("pic.mp4"));
    let out = r.unwrap();
    let (red1, _, cyan1) = colours(&env.frame_rgb(&out, 2.0));
    assert!(red1 > 100 && cyan1 == 0, "clip 1: red {red1}, cyan {cyan1}");
    let (red2, _, cyan2) = colours(&env.frame_rgb(&out, 4.0));
    assert!(cyan2 > 100 && red2 == 0, "clip 2: red {red2}, cyan {cyan2}");
    // the preview draws both clips the same way
    let (pr1, _, pc1) = colours(&preview_rgb(&env, &p, Micros(2_000_000)));
    assert!(pr1 > 100 && pc1 == 0, "preview clip 1: {pr1} {pc1}");
    let (pr2, _, pc2) = colours(&preview_rgb(&env, &p, Micros(4_000_000)));
    assert!(pc2 > 100 && pr2 == 0, "preview clip 2: {pr2} {pc2}");
}
