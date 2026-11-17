// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 1.3.0: text items burned in with their effects, texts fixed to the
//! sphere in the flat camera view and in a 360 video.

use super::*;
use videolore::camera::{self, Camera, Pov};
use videolore::subtitles::Pos;
use videolore::text::{Anchor360, SlideDir, TextId, TextItem};

/// A black clip of `secs` seconds, `w` x `h`.
fn black(env: &Env, w: u32, h: u32, secs: u32) -> PathBuf {
    env.fixture(
        &format!("black-{w}x{h}-{secs}.mp4"),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c=black:s={w}x{h}:r=10:d={secs}"),
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
    // a black clip has a tiny bit rate: the export would blur the text away
    p.media[0].format.bit_rate = Some(20_000_000);
    p.timeline.append(&m).unwrap();
    p
}

fn add_text(p: &mut Project, f: impl FnOnce(&mut TextItem)) -> TextId {
    let id = p.texts.new_id();
    let mut t = TextItem::new(id, "MMMMMM".into(), Micros::ZERO);
    t.place = Pos { x: 5000, y: 5000 };
    t.size = 4000;
    f(&mut t);
    p.texts.add(t).unwrap()
}

/// The brightest green value of an RGB frame (white text on black).
fn peak(frame: &[u8]) -> f64 {
    f64::from(frame.chunks(3).map(|p| p[1]).max().unwrap_or(0))
}

/// Centroid (x, y) of the strongly red pixels of an RGB frame `w` wide, with their count.
fn red_centroid(frame: &[u8], w: usize) -> Option<(f64, f64, usize)> {
    let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
    for (i, p) in frame.chunks(3).enumerate() {
        if p[0] > 180 && p[1] < 90 && p[2] < 90 {
            sx += (i % w) as f64 + 0.5;
            sy += (i / w) as f64 + 0.5;
            n += 1;
        }
    }
    (n > 20).then(|| (sx / n as f64, sy / n as f64, n))
}

fn can_burn(env: &Env) -> bool {
    if !env.info.has_subtitles() {
        assert!(!required(), "the CI ffmpeg must have libass");
        return false;
    }
    true
}

// covers: REQ-109, REQ-111, REQ-023
#[test]
fn a_text_is_shown_between_its_start_and_its_end_and_forces_the_re_encode() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, &black(&env, 320, 180, 6));
    add_text(&mut p, |t| {
        t.start = Micros(2_000_000);
        t.ttl = Micros(3_000_000);
    });
    let (r, ev) = env.export(&p, &env.dir.path().join("t1.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let r = r.unwrap();
    for (t, shown) in [(1.0, false), (3.0, true), (4.5, true), (5.5, false)] {
        let f = env.frame_rgb(&r, t);
        assert_eq!(peak(&f) > 200.0, shown, "at {t} s: peak {}", peak(&f));
    }
}

// covers: REQ-109
#[test]
fn fade_in_is_half_visible_halfway_and_blink_hides_every_other_interval() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, &black(&env, 320, 180, 6));
    add_text(&mut p, |t| {
        t.ttl = Micros(4_000_000);
        t.effects.fade_in = Some(Micros(1_000_000));
    });
    let (r, _) = env.export(&p, &env.dir.path().join("fade.mp4"));
    let r = r.unwrap();
    let half = peak(&env.frame_rgb(&r, 0.5)) / peak(&env.frame_rgb(&r, 2.0));
    assert!((half - 0.5).abs() <= 0.08, "fade at 0.5 s of 1 s: {half}");
    // a fade longer than half of the time to live is limited to half of it
    let mut q = project_of(&env, &black(&env, 320, 180, 6));
    add_text(&mut q, |t| {
        t.ttl = Micros(2_000_000);
        t.effects.fade_in = Some(Micros(5_000_000));
    });
    let (r, _) = env.export(&q, &env.dir.path().join("fade2.mp4"));
    let r = r.unwrap();
    assert!(
        peak(&env.frame_rgb(&r, 0.9)) > 200.0,
        "full at half of the ttl"
    );
    // blink 0.5 s: shown, hidden, shown
    let mut b = project_of(&env, &black(&env, 320, 180, 6));
    add_text(&mut b, |t| {
        t.ttl = Micros(3_000_000);
        t.effects.blink = Some(Micros(500_000));
    });
    let (r, _) = env.export(&b, &env.dir.path().join("blink.mp4"));
    let r = r.unwrap();
    for (t, shown) in [
        (0.25, true),
        (0.75, false),
        (1.25, true),
        (1.75, false),
        (2.25, true),
    ] {
        assert_eq!(peak(&env.frame_rgb(&r, t)) > 200.0, shown, "blink at {t} s");
    }
    // blink with a fade in: the first phase is on its way up
    let mut c = project_of(&env, &black(&env, 320, 180, 6));
    add_text(&mut c, |t| {
        t.ttl = Micros(4_000_000);
        t.effects.blink = Some(Micros(500_000));
        t.effects.fade_in = Some(Micros(1_000_000));
    });
    let (r, _) = env.export(&c, &env.dir.path().join("blinkfade.mp4"));
    let r = r.unwrap();
    let early = peak(&env.frame_rgb(&r, 0.3));
    let later = peak(&env.frame_rgb(&r, 2.25));
    assert!(early < later * 0.8 && early > 20.0, "{early} {later}");
}

// covers: REQ-109
#[test]
fn a_text_slides_in_from_the_left_and_reaches_its_box() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = project_of(&env, &black(&env, 320, 180, 6));
    add_text(&mut p, |t| {
        t.ttl = Micros(4_000_000);
        t.size = 1000;
        t.effects.slide = Some((SlideDir::Left, Micros(1_000_000)));
    });
    let (r, _) = env.export(&p, &env.dir.path().join("slide.mp4"));
    let r = r.unwrap();
    // the text is in the left half first and centred at the end
    let centroid = |f: &[u8]| -> Option<f64> {
        let (mut sx, mut n) = (0.0, 0.0);
        for (i, p) in f.chunks(3).enumerate() {
            if p[1] > 128 {
                sx += (i % 320) as f64;
                n += 1.0;
            }
        }
        (n > 10.0).then(|| sx / n)
    };
    let early = centroid(&env.frame_rgb(&r, 0.3)).expect("text coming in");
    assert!(early < 110.0, "coming from the left: {early}");
    let late = centroid(&env.frame_rgb(&r, 2.0)).expect("text at its box");
    assert!((late - 160.0).abs() < 12.0, "at its box: {late}");
    // at the very start nothing of the box is in the picture
    assert!(peak(&env.frame_rgb(&r, 0.0)) < 60.0);
}

/// A 2048x1024 equirectangular black clip of `secs` seconds.
fn sphere_black(env: &Env, secs: u32) -> PathBuf {
    black(env, 2048, 1024, secs)
}

fn sphere_project(env: &Env, secs: u32) -> Project {
    let mut p = project_of(env, &sphere_black(env, secs));
    p.media[0].format.treat_as_360 = Some(true);
    p
}

fn pin(t: &mut TextItem, yaw: f64, pitch: f64) {
    t.colour = [255, 0, 0];
    t.size = 1000;
    t.text = "MM".into();
    t.ttl = Micros(4_000_000);
    t.anchor = Some(Anchor360 {
        yaw_cd: (yaw * 100.0) as i32,
        pitch_cd: (pitch * 100.0) as i32,
        size_cd: 600,
        wrap_cd: 6000,
    });
}

// covers: REQ-112
#[test]
fn a_text_fixed_to_the_sphere_stays_at_its_point_while_the_camera_turns() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = sphere_project(&env, 4);
    let m = p.media[0].id;
    let pov = |t: f64, yaw: f64| Pov {
        t: Micros::from_secs_f64(t),
        cam: Camera::new((yaw * 100.0) as i64, 0, 9000),
    };
    p.camera.set(m, pov(0.0, 10.0)).unwrap();
    p.camera.set(m, pov(4.0, 60.0)).unwrap();
    add_text(&mut p, |t| pin(t, 30.0, 5.0));
    let (r, _) = env.export(&p, &env.dir.path().join("pin.mp4"));
    let r = r.unwrap();
    for t in [0.5, 1.5, 2.5] {
        let f = env.frame_rgb(&r, t);
        let (x, y, _) = red_centroid(&f, 1920).unwrap_or_else(|| panic!("no text at {t} s"));
        let cam = camera::at(p.camera.path(m), Micros::from_secs_f64(t)).unwrap();
        let (yaw, pitch) = camera::unproject(cam, 1920, 1080, x, y);
        assert!(
            (yaw - 30.0).abs() <= 1.5 && (pitch - 5.0).abs() <= 1.5,
            "at {t} s: yaw {yaw} pitch {pitch}"
        );
    }
    // the point leaves the view when the camera looks away: no text
    let mut q = sphere_project(&env, 4);
    let m = q.media[0].id;
    q.camera.set(m, pov(0.0, -120.0)).unwrap();
    add_text(&mut q, |t| pin(t, 30.0, 5.0));
    let (r, _) = env.export(&q, &env.dir.path().join("away.mp4"));
    assert!(red_centroid(&env.frame_rgb(&r.unwrap(), 1.0), 1920).is_none());
}

// covers: REQ-103, REQ-112
#[test]
fn a_360_video_keeps_the_whole_sphere_with_the_text_at_its_point() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = sphere_project(&env, 4);
    add_text(&mut p, |t| pin(t, 30.0, 10.0));
    let out = env.dir.path().join("sphere.mp4");
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut job = env.job(&p, &out);
    job.mode = render::ExportMode::Sphere360;
    let (done, _) = export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    // the file says it is an equirectangular 360 video
    let probe = Command::new(&env.info.ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream_side_data=type,projection",
        ])
        .arg(&done)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&probe.stdout).to_string();
    assert!(
        text.contains("projection=equirectangular"),
        "spherical information: {text}"
    );
    // the picture is the sphere itself (2048x1024), not a camera view
    let f = env.frame_rgb(&done, 1.0);
    assert_eq!(f.len(), 2048 * 1024 * 3);
    for t in [0.5, 1.5, 3.0] {
        let f = env.frame_rgb(&done, t);
        let (x, y, _) = red_centroid(&f, 2048).unwrap_or_else(|| panic!("no text at {t} s"));
        let yaw = (x / 2048.0 - 0.5) * 360.0;
        let pitch = (0.5 - y / 1024.0) * 180.0;
        assert!(
            (yaw - 30.0).abs() <= 1.0 && (pitch - 10.0).abs() <= 1.0,
            "at {t} s: {yaw} {pitch}"
        );
    }
}

// covers: REQ-112
#[test]
fn a_text_across_the_seam_is_drawn_on_both_sides() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = sphere_project(&env, 3);
    add_text(&mut p, |t| {
        pin(t, 179.5, 0.0);
        t.text = "MMMMMMMMMM".into();
        t.size = 2000;
    });
    let out = env.dir.path().join("seam.mp4");
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut job = env.job(&p, &out);
    job.mode = render::ExportMode::Sphere360;
    let (done, _) = export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    let f = env.frame_rgb(&done, 1.0);
    let w = 2048;
    let side = |range: std::ops::Range<usize>| -> usize {
        f.chunks(3)
            .enumerate()
            .filter(|(i, p)| range.contains(&(i % w)) && p[0] > 180 && p[1] < 90)
            .count()
    };
    let (left, right) = (side(0..60), side(w - 60..w));
    assert!(left > 20 && right > 20, "left {left} right {right}");
}

// covers: REQ-112, REQ-103
#[test]
fn a_subtitle_line_fixed_to_the_sphere_stays_while_the_camera_turns_and_in_a_360_video() {
    let Some(env) = setup() else { return };
    if !can_burn(&env) {
        return;
    }
    let mut p = sphere_project(&env, 4);
    let m = p.media[0].id;
    let pov = |t: f64, yaw: f64| Pov {
        t: Micros::from_secs_f64(t),
        cam: Camera::new((yaw * 100.0) as i64, 0, 9000),
    };
    p.camera.set(m, pov(0.0, 10.0)).unwrap();
    p.camera.set(m, pov(4.0, 60.0)).unwrap();
    let id = p
        .subtitles
        .add_at(&p.timeline, Micros::ZERO, "MM", Micros(40_000))
        .unwrap();
    p.subtitles.set_text(id, "MM").unwrap();
    p.subtitles.lines.iter_mut().for_each(|l| {
        l.end = Micros(4_000_000);
        l.anchor = Some(Anchor360 {
            yaw_cd: 3000,
            pitch_cd: 500,
            size_cd: 600,
            wrap_cd: 6000,
        });
    });
    // the line is white: look for white pixels
    let white = |f: &[u8], w: usize| -> Option<(f64, f64)> {
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
        for (i, p) in f.chunks(3).enumerate() {
            if p.iter().all(|v| *v > 200) {
                sx += (i % w) as f64 + 0.5;
                sy += (i / w) as f64 + 0.5;
                n += 1;
            }
        }
        (n > 20).then(|| (sx / n as f64, sy / n as f64))
    };
    let (r, _) = env.export(&p, &env.dir.path().join("line.mp4"));
    let r = r.unwrap();
    for t in [0.5, 1.5, 2.5] {
        let f = env.frame_rgb(&r, t);
        let (x, y) = white(&f, 1920).unwrap_or_else(|| panic!("no line at {t} s"));
        let cam = camera::at(p.camera.path(m), Micros::from_secs_f64(t)).unwrap();
        let (yaw, pitch) = camera::unproject(cam, 1920, 1080, x, y);
        assert!(
            (yaw - 30.0).abs() <= 1.5 && (pitch - 5.0).abs() <= 1.5,
            "at {t} s: {yaw} {pitch}"
        );
    }
    // in a 360 video it is at the same point of the sphere
    p.camera = Default::default();
    let out = env.dir.path().join("line360.mp4");
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut job = env.job(&p, &out);
    job.mode = render::ExportMode::Sphere360;
    let (done, _) = export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    let f = env.frame_rgb(&done, 1.0);
    let (x, y) = white(&f, 2048).expect("the line is in the 360 video");
    let yaw = (x / 2048.0 - 0.5) * 360.0;
    let pitch = (0.5 - y / 1024.0) * 180.0;
    assert!(
        (yaw - 30.0).abs() <= 1.0 && (pitch - 5.0).abs() <= 1.0,
        "{yaw} {pitch}"
    );
}
