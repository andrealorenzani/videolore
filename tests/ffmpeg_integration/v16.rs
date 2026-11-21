// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 1.6.0: Apply LUT. Fixtures are flat colours, the tables are 2x2x2
//! cubes that move whole colours, so pixels can be sampled with a small tolerance.

use super::*;
use videolore::render::Source;
use videolore::timeline::FrameTarget;

const RED: [i32; 3] = [255, 0, 0];
const GREEN: [i32; 3] = [0, 255, 0];
const BLUE: [i32; 3] = [0, 0, 255];

/// A 6 s, 160x120, 10 fps clip of one flat colour.
fn flat(env: &Env, colour: &str) -> PathBuf {
    env.fixture(
        &format!("lut-flat-{colour}.mp4"),
        &[
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={colour}:s=160x120:r=10:d=6"),
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

/// A 2x2x2 `.cube` file where each output channel is `f(r, g, b)` of the corner.
fn cube(env: &Env, name: &str, f: impl Fn([u8; 3]) -> [u8; 3]) -> String {
    let mut t = String::from("TITLE \"test\"\nLUT_3D_SIZE 2\n");
    for b in 0..2u8 {
        for g in 0..2u8 {
            for r in 0..2u8 {
                let o = f([r, g, b]);
                t.push_str(&format!("{} {} {}\n", o[0], o[1], o[2]));
            }
        }
    }
    let p = env.dir.path().join(name);
    std::fs::write(&p, t).unwrap();
    p.to_string_lossy().into_owned()
}

fn three_clips(env: &Env, colour: &str) -> Project {
    let mut p = Project::default();
    let m = env.probe(&mut p, &flat(env, colour));
    p.media[0].format.bit_rate = Some(20_000_000);
    for i in 0..3 {
        p.timeline.append(&m).unwrap();
        let id = p.timeline.clips[i].id;
        let a = Micros(i as i64 * 2_000_000);
        p.timeline
            .trim(id, a, a + Micros(2_000_000), m.duration, Micros(1))
            .unwrap();
    }
    p
}

fn px(buf: &[u8], w: usize, x: usize, y: usize) -> [i32; 3] {
    let i = (y * w + x) * 3;
    [buf[i] as i32, buf[i + 1] as i32, buf[i + 2] as i32]
}

fn close(p: [i32; 3], want: [i32; 3], tol: i32) -> bool {
    p.iter().zip(want).all(|(a, b)| (a - b).abs() <= tol)
}

/// The preview's own still (RGBA) at `at`, as the centre pixel.
fn preview_centre(env: &Env, p: &Project, at: Micros) -> [i32; 3] {
    centre_with(env, p, at, Source::Export)
}

fn centre_with(env: &Env, p: &Project, at: Micros, src: Source) -> [i32; 3] {
    let eg = render::output_geometry(p, &env.info.encoders).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let r =
        render::still_args(p, at, &eg, env.info.version, &dir.path().join("g.txt"), src).unwrap();
    render::write_command_files(dir.path(), &r.command_files, &r.lut_files).unwrap();
    let o = Command::new(&env.info.ffmpeg)
        .args(&r.args)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let w = eg.preview().width as usize;
    let h = o.stdout.len() / 4 / w;
    let i = ((h / 2) * w + w / 2) * 4;
    [
        o.stdout[i] as i32,
        o.stdout[i + 1] as i32,
        o.stdout[i + 2] as i32,
    ]
}

fn export_to(env: &Env, p: &Project, name: &str) -> (PathBuf, usize) {
    let out = env.dir.path().join(name);
    env.export(p, &out).0.unwrap();
    (out, 160)
}

// covers: REQ-135
// covers: REQ-136
#[test]
fn a_lut_on_clip_2_of_3_changes_only_that_clip_in_the_preview_and_the_export() {
    let Some(env) = setup() else { return };
    let mut p = three_clips(&env, "red");
    // red -> blue, everything else kept simple: output = (b, g, r)
    let swap = cube(&env, "swap.cube", |[r, g, b]| [b, g, r]);
    let id = p.timeline.clips[1].id;
    p.timeline
        .set_lut(FrameTarget::Clip(id), Some(swap))
        .unwrap();
    // preview
    for (t, want) in [(1.0, RED), (3.0, BLUE), (5.0, RED)] {
        let got = preview_centre(&env, &p, Micros::from_secs_f64(t));
        assert!(close(got, want, 4), "preview at {t}: {got:?}");
    }
    // export: re-encoded (a project whose only change is Apply LUT), same colours
    let (out, w) = export_to(&env, &p, "lut.mp4");
    for (t, want) in [(1.0, RED), (3.0, BLUE), (5.0, RED)] {
        let f = env.frame_rgb(&out, t);
        let got = px(&f, w, 80, 60);
        assert!(close(got, want, 8), "export at {t}: {got:?}");
        let pv = preview_centre(&env, &p, Micros::from_secs_f64(t));
        assert!(close(got, pv, 8), "export {got:?} vs preview {pv:?}");
    }
    // removing the effect gives the source colours back
    p.timeline.set_lut(FrameTarget::Clip(id), None).unwrap();
    let got = preview_centre(&env, &p, Micros::from_secs_f64(3.0));
    assert!(close(got, RED, 4));
}

// covers: REQ-135, REQ-147
#[test]
fn the_whole_videos_lut_is_a_copy_on_each_clip_and_a_clips_own_replaces_it() {
    let Some(env) = setup() else { return };
    let mut p = three_clips(&env, "red");
    // whole: red -> blue on every clip; then clip 2 gets red -> red (its own replaces the copy)
    let whole = cube(&env, "w.cube", |[r, g, b]| [b, g, r]);
    let clip = cube(&env, "c.cube", |[r, g, b]| [r, b, g]);
    let id = p.timeline.clips[1].id;
    p.timeline.set_lut(FrameTarget::Whole, Some(whole)).unwrap();
    p.timeline
        .set_lut(FrameTarget::Clip(id), Some(clip))
        .unwrap();
    let t = |s: f64| preview_centre(&env, &p, Micros::from_secs_f64(s));
    assert!(close(t(1.0), BLUE, 4), "{:?}", t(1.0));
    assert!(close(t(3.0), RED, 4), "{:?}", t(3.0));
    assert!(close(t(5.0), BLUE, 4), "{:?}", t(5.0));
    let (out, w) = export_to(&env, &p, "order.mp4");
    let got = px(&env.frame_rgb(&out, 3.0), w, 80, 60);
    assert!(close(got, RED, 8), "{got:?}");
}

// covers: REQ-135
#[test]
fn the_empty_background_keeps_its_colour() {
    let Some(env) = setup() else { return };
    let mut p = three_clips(&env, "red");
    // out = (g, b, r): a green background would turn red, a red picture blue
    let rot = cube(&env, "rot.cube", |[r, g, b]| [g, b, r]);
    p.timeline.set_lut(FrameTarget::Whole, Some(rot)).unwrap();
    p.timeline
        .set_frame_aspect(Some(FrameAspect {
            shape: Shape::Tall9x16,
            fit: Fit::Width,
            fill: Fill::Colour(Rgb(0, 255, 0)),
            blur: Default::default(),
        }))
        .unwrap();
    let out = env.dir.path().join("bg.mp4");
    env.export(&p, &out).0.unwrap();
    let f = env.frame_rgb(&out, 1.0);
    let w = 160;
    assert!(
        close(px(&f, w, 80, 142), BLUE, 10),
        "{:?}",
        px(&f, w, 80, 142)
    );
    assert!(
        close(px(&f, w, 80, 30), GREEN, 10),
        "{:?}",
        px(&f, w, 80, 30)
    );
}

use videolore::frame::{Fill, Fit, FrameAspect, Rgb, Shape};

// covers: REQ-136
#[test]
fn a_missing_lut_file_stops_the_export_before_anything_is_written() {
    let Some(env) = setup() else { return };
    let mut p = three_clips(&env, "red");
    let id = p.timeline.clips[0].id;
    let path = env.dir.path().join("gone.cube");
    std::fs::write(&path, "LUT_3D_SIZE 2\n").unwrap(); // broken content too
    p.timeline
        .set_lut(
            FrameTarget::Clip(id),
            Some(path.to_string_lossy().into_owned()),
        )
        .unwrap();
    let out = env.dir.path().join("none.mp4");
    let (r, _) = env.export(&p, &out);
    let e = r.unwrap_err().to_string();
    assert!(e.contains("gone.cube"), "{e}");
    assert!(!out.exists());
    std::fs::remove_file(&path).unwrap();
    let (r, _) = env.export(&p, &out);
    assert!(r.unwrap_err().to_string().contains("gone.cube"));
    assert!(!out.exists());
    // in the preview a missing file is ignored: the source colours show
    let src = Source::Preview { proxies: false };
    let got = centre_with(&env, &p, Micros::from_secs_f64(1.0), src);
    assert!(close(got, RED, 4));
}

// covers: REQ-136
#[test]
fn the_360_video_export_shows_the_lut_colours() {
    let Some(env) = setup() else { return };
    let x = (2048.0 * 0.75) as u32;
    let src = format!(
        "color=c=black:s=2048x1024:r=10:d=2,drawbox=x={x}:y=0:w=8:h=1024:color=white:t=fill"
    );
    let file = env.fixture(
        "lut-sphere.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            &src,
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "14",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let mut p = Project::default();
    let m = env.probe(&mut p, &file);
    p.media[0].format.spherical = Some(true);
    p.timeline.append(&m).unwrap();
    // inversion: black -> white
    let inv = cube(&env, "inv.cube", |[r, g, b]| [1 - r, 1 - g, 1 - b]);
    p.timeline.set_lut(FrameTarget::Whole, Some(inv)).unwrap();
    let out = env.dir.path().join("lut360.mp4");
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut job = env.job(&p, &out);
    job.mode = render::ExportMode::Sphere360;
    let (done, _) = export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
    let f = env.frame_rgb(&done, 1.0);
    assert_eq!(f.len(), 2048 * 1024 * 3);
    assert!(close(px(&f, 2048, 100, 500), [255, 255, 255], 10));
    assert!(close(px(&f, 2048, x as usize + 4, 500), [0, 0, 0], 30));
}

/// A cube of `n` points an edge: the output of grid point `(r, g, b)` (0..=1) is `f`.
fn big_cube(env: &Env, name: &str, n: usize, f: impl Fn([f64; 3]) -> [f64; 3]) -> String {
    let mut t = format!("TITLE \"big\"\nLUT_3D_SIZE {n}\n");
    let m = (n - 1) as f64;
    for b in 0..n {
        for g in 0..n {
            for r in 0..n {
                let o = f([r as f64 / m, g as f64 / m, b as f64 / m]);
                t.push_str(&format!("{:.6} {:.6} {:.6}\n", o[0], o[1], o[2]));
            }
        }
    }
    let p = env.dir.path().join(name);
    std::fs::write(&p, t).unwrap();
    p.to_string_lossy().into_owned()
}

// covers: REQ-300
#[test]
fn a_144_point_lut_swaps_the_channels_in_the_preview_and_the_export_within_two_levels() {
    let Some(env) = setup() else { return };
    let mut p = three_clips(&env, "red");
    let swap = big_cube(&env, "swap144.cube", 144, |[r, g, b]| [b, g, r]);
    let id = p.timeline.clips[1].id;
    p.timeline
        .set_lut(FrameTarget::Clip(id), Some(swap))
        .unwrap();
    let pv = preview_centre(&env, &p, Micros::from_secs_f64(3.0));
    assert!(close(pv, BLUE, 4), "preview {pv:?}");
    let (out, w) = export_to(&env, &p, "lut144.mp4");
    let f = env.frame_rgb(&out, 3.0);
    let got = px(&f, w, 80, 60);
    assert!(close(got, BLUE, 8), "export {got:?}");
    assert!(close(got, pv, 8), "export {got:?} vs preview {pv:?}");
    // the other clips are not changed
    let f = env.frame_rgb(&out, 1.0);
    assert!(close(px(&f, w, 80, 60), RED, 8));
}
