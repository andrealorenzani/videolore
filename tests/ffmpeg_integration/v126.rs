// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 1.2.6: 360 video (camera path in the preview and the export),
//! detection of spherical files and the burned-in text size.

use super::*;
use videolore::camera::{Camera, Pov};
use videolore::media::MediaFormat;
use videolore::render::Source;

/// Longitude (degrees, 0 = middle of the picture) of the white line of the test sphere.
const LINE_LON: f64 = 30.0;

/// Latitude (degrees, 0 = horizon) of the white parallel of the test sphere.
const LINE_LAT: f64 = 20.0;

/// A 2048x1024 equirectangular picture, black with one thin white meridian at
/// [`LINE_LON`] and one thin white parallel at [`LINE_LAT`]: where they show up in a view
/// tells the direction of the camera.
fn sphere_file(env: &Env, secs: u32) -> PathBuf {
    let x = ((LINE_LON + 180.0) / 360.0 * 2048.0) as u32 - 3;
    let y = ((90.0 - LINE_LAT) / 180.0 * 1024.0) as u32 - 3;
    let src = format!(
        "color=c=black:s=2048x1024:r=10:d={secs},drawbox=x={x}:y=0:w=6:h=1024:color=white:t=fill,\
         drawbox=x=0:y={y}:w=2048:h=6:color=white:t=fill"
    );
    env.fixture(
        &format!("sphere{secs}.mp4"),
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
    )
}

/// A project with one 360 clip of the sphere file.
fn sphere_project(env: &Env, secs: u32) -> Project {
    let mut p = Project::default();
    let file = sphere_file(env, secs);
    let m = env.probe(&mut p, &file);
    p.media[0].format.treat_as_360 = Some(true);
    p.timeline.append(&m).unwrap();
    p
}

fn pov(secs: f64, yaw_deg: f64, fov_deg: f64) -> Pov {
    Pov {
        t: Micros::from_secs_f64(secs),
        cam: Camera::new((yaw_deg * 100.0) as i64, 0, (fov_deg * 100.0) as i64),
    }
}

/// Direction (degrees right of the view's centre) of the white line in an RGB or RGBA
/// frame `w` wide, for a view `fov` degrees wide.
fn line_direction(frame: &[u8], w: usize, bytes: usize, fov: f64) -> f64 {
    let h = frame.len() / bytes / w;
    let row = h / 2;
    let (mut sum, mut n) = (0.0, 0.0);
    for x in 0..w {
        let px = &frame[(row * w + x) * bytes..][..3];
        if px.iter().all(|v| *v > 200) {
            sum += x as f64;
            n += 1.0;
        }
    }
    assert!(n > 0.0, "the line is not in the view");
    let x = sum / n + 0.5;
    let t = (x / w as f64 * 2.0 - 1.0) * (fov.to_radians() / 2.0).tan();
    t.atan().to_degrees()
}

/// Elevation (degrees above the view's centre) of the white parallel in a frame `w` wide,
/// for a view `fov` degrees wide (16:9), looked at in the column `col`.
fn parallel_elevation(frame: &[u8], w: usize, bytes: usize, fov: f64, col: usize) -> f64 {
    let h = frame.len() / bytes / w;
    let rows: Vec<usize> = (0..h)
        .filter(|y| frame[(y * w + col) * bytes..][..3].iter().all(|v| *v > 200))
        .collect();
    assert!(!rows.is_empty(), "the parallel is not in the view");
    let y = rows.iter().sum::<usize>() as f64 / rows.len() as f64 + 0.5;
    let half_h = (fov.to_radians() / 2.0).tan() * h as f64 / w as f64;
    (-((y / h as f64 * 2.0 - 1.0) * half_h).atan()).to_degrees()
}

/// The picture the preview shows at `at` (a still, the preview's own code path).
fn preview_still(env: &Env, p: &Project, at: Micros) -> (Vec<u8>, usize) {
    let eg = render::output_geometry(p, &env.info.encoders).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let r = render::still_args(
        p,
        at,
        &eg,
        env.info.version,
        &dir.path().join("g.txt"),
        Source::Export,
    )
    .unwrap();
    render::write_command_files(dir.path(), &r.command_files).unwrap();
    let o = Command::new(&env.info.ffmpeg)
        .args(&r.args)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    (o.stdout, eg.preview().width as usize)
}

fn near(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() < 1.0,
        "{what}: got {got:.2} want {want:.2}"
    );
}

/// The camera path is followed frame by frame in the preview and in the export: at the
/// middle of 0 -> 90 degrees the camera looks 45 degrees to the right, the same in both.
// covers: REQ-102, REQ-103, REQ-101
#[test]
fn the_camera_follows_the_path_in_preview_and_export() {
    let Some(env) = setup() else { return };
    let mut p = sphere_project(&env, 2);
    let m = p.media[0].id;
    p.camera.set(m, pov(0.0, 0.0, 90.0)).unwrap();
    p.camera.set(m, pov(2.0, 90.0, 90.0)).unwrap();
    // preview, at three times (also fixes the sign: yaw > 0 looks right)
    for (t, yaw) in [(0.0, 0.0), (1.0, 45.0), (1.5, 67.5)] {
        let (f, w) = preview_still(&env, &p, Micros::from_secs_f64(t));
        near(
            line_direction(&f, w, 4, 90.0),
            LINE_LON - yaw,
            &format!("preview at {t}"),
        );
    }
    // the export: flat 16:9, the same view at the same times
    let (r, ev) = env.export(&p, &env.dir.path().join("sphere.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let r = r.unwrap();
    for (t, yaw) in [(0.0, 0.0), (1.0, 45.0)] {
        let f = env.frame_rgb(&r, t);
        assert_eq!(f.len(), 1920 * 1080 * 3, "output size at {t}");
        near(
            line_direction(&f, 1920, 3, 90.0),
            LINE_LON - yaw,
            &format!("export at {t}"),
        );
    }
    let probe = Command::new(&env.info.ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height:stream_side_data=:stream_tags=",
            "-of",
            "csv=p=0",
        ])
        .arg(&r)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&probe.stdout).to_string();
    assert!(text.starts_with("1920,1080"), "{text}");
    assert!(!text.contains("Spherical"), "the output is flat: {text}");
}

/// Pitch and zoom are commanded frame by frame in the export too: the parallel at 20
/// degrees is at the centre when the camera looks up 20 degrees, and the field of view
/// follows the path.
// covers: REQ-102, REQ-103
#[test]
fn pitch_and_zoom_are_followed_in_the_export() {
    let Some(env) = setup() else { return };
    let mut p = sphere_project(&env, 2);
    let m = p.media[0].id;
    let cam = |pitch: f64, fov: f64| Camera::new(0, (pitch * 100.0) as i64, (fov * 100.0) as i64);
    p.camera
        .set(
            m,
            Pov {
                t: Micros::ZERO,
                cam: cam(0.0, 90.0),
            },
        )
        .unwrap();
    p.camera
        .set(
            m,
            Pov {
                t: Micros::from_secs_f64(2.0),
                cam: cam(40.0, 50.0),
            },
        )
        .unwrap();
    let (r, _) = env.export(&p, &env.dir.path().join("pitch.mp4"));
    let r = r.unwrap();
    // 1.0 s: pitch 20, field of view 70
    let f = env.frame_rgb(&r, 1.0);
    near(
        parallel_elevation(&f, 1920, 3, 70.0, 960),
        0.0,
        "parallel at the centre",
    );
    // 0.5 s: pitch 10, field of view 80: the parallel is 10 degrees above the centre
    let f = env.frame_rgb(&r, 0.5);
    near(parallel_elevation(&f, 1920, 3, 80.0, 960), 10.0, "pitch 10");
}

/// From 350 to 10 degrees the camera passes through 0, not through 180.
// covers: REQ-102
#[test]
fn the_camera_takes_the_short_way_round() {
    let Some(env) = setup() else { return };
    let mut p = sphere_project(&env, 2);
    let m = p.media[0].id;
    p.camera.set(m, pov(0.0, -10.0, 90.0)).unwrap();
    p.camera.set(m, pov(2.0, 10.0, 90.0)).unwrap();
    let (f, w) = preview_still(&env, &p, Micros::from_secs_f64(1.0));
    near(line_direction(&f, w, 4, 90.0), LINE_LON, "through 0");
}

/// After a split both clips show the view the path had at the cut.
// covers: REQ-102
#[test]
fn a_split_keeps_the_view_at_the_edges() {
    let Some(env) = setup() else { return };
    let mut p = sphere_project(&env, 2);
    let m = p.media[0].id;
    p.camera.set(m, pov(0.0, 0.0, 90.0)).unwrap();
    p.camera.set(m, pov(2.0, 80.0, 90.0)).unwrap();
    p.timeline.split(Micros(1_000_000), Micros(1)).unwrap();
    // timeline 0.95 s is the last frame of the first clip, 1.0 s the first of the second
    let (a, w) = preview_still(&env, &p, Micros(900_000));
    let (b, _) = preview_still(&env, &p, Micros(1_000_000));
    near(
        line_direction(&a, w, 4, 90.0),
        LINE_LON - 36.0,
        "before the cut",
    );
    near(
        line_direction(&b, w, 4, 90.0),
        LINE_LON - 40.0,
        "after the cut",
    );
}

/// Zoom changes the field of view along the path (the 30..110 degree range).
// covers: REQ-101, REQ-102
#[test]
fn the_zoom_follows_the_path_too() {
    let Some(env) = setup() else { return };
    let mut p = sphere_project(&env, 2);
    let m = p.media[0].id;
    p.camera.set(m, pov(0.0, 40.0, 110.0)).unwrap();
    p.camera.set(m, pov(2.0, 40.0, 50.0)).unwrap();
    let (f, w) = preview_still(&env, &p, Micros::ZERO);
    near(line_direction(&f, w, 4, 110.0), LINE_LON - 40.0, "wide");
    let (f, w) = preview_still(&env, &p, Micros::from_secs_f64(1.0));
    near(line_direction(&f, w, 4, 80.0), LINE_LON - 40.0, "half way");
}

/// A 360 clip next to a normal clip: both are framed, the file is flat.
// covers: REQ-103, REQ-022, REQ-023
#[test]
fn a_360_clip_and_a_normal_clip_export_together() {
    let Some(env) = setup() else { return };
    let mut p = sphere_project(&env, 2);
    let flat = env.clip("flat.mp4", 640, 360, 10, 2, true);
    let m = env.probe(&mut p, &flat);
    p.timeline.append(&m).unwrap();
    let (r, ev) = env.export(&p, &env.dir.path().join("mixed.mp4"));
    assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
    let r = r.unwrap();
    let means = env.luma_means(&r, 640, 360);
    assert!(means.len() >= 38, "{}", means.len());
    // the normal clip (testsrc2) is bright, the sphere view is mostly black
    assert!(
        means[5] < 60.0 && means[30] > 60.0,
        "{} {}",
        means[5],
        means[30]
    );
}

/// The committed spherical file is recognised; the same picture without the information
/// is a normal video until the switch is turned on.
// covers: REQ-100
#[test]
fn spherical_files_are_recognised() {
    let Some(env) = setup() else { return };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut p = Project::default();
    let sph = env.probe(&mut p, &dir.join("spherical-360.mp4"));
    assert_eq!(sph.format.spherical, Some(true));
    assert!(sph.format.is_360());
    let flat = env.probe(&mut p, &dir.join("flat-2to1.mp4"));
    assert_eq!(flat.format.spherical, Some(false));
    assert!(!flat.format.is_360());
    let on = MediaFormat {
        treat_as_360: Some(true),
        ..flat.format
    };
    assert!(on.is_360());
}

/// The burned-in text is as big as the preview's box says: a line at 200 % has twice
/// the height of one at 100 % (REQ-097, REQ-017).
// covers: REQ-097
#[test]
fn a_sized_subtitle_is_burned_in_at_its_size() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        assert!(!required(), "the CI ffmpeg must have libass");
        return;
    }
    let black = env.fixture(
        "black720.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=1280x720:r=10:d=2",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let mut p = Project::default();
    let m = env.probe(&mut p, &black);
    p.timeline.append(&m).unwrap();
    let tl = p.timeline.clone();
    let id = p
        .subtitles
        .add_at(&tl, Micros::ZERO, "HHH", Micros(40_000))
        .unwrap();
    p.subtitles
        .set_boxes(
            &[id],
            Some(Some(videolore::subtitles::Pos { x: 5000, y: 5000 })),
            None,
        )
        .unwrap();
    let extent = |p: &Project, name: &str| {
        let (r, _) = env.export(p, &env.dir.path().join(name));
        let rgb = env.frame_rgb(&r.unwrap(), 0.5);
        let (mut top, mut bottom) = (usize::MAX, 0);
        for (i, px) in rgb.as_chunks::<3>().0.iter().enumerate() {
            if px.iter().all(|v| *v > 150) {
                top = top.min(i / 1280);
                bottom = bottom.max(i / 1280);
            }
        }
        (bottom - top + 1) as f64
    };
    let normal = extent(&p, "size100.mp4");
    p.subtitles
        .set_boxes(&[id], None, Some(Some(2000)))
        .unwrap();
    let big = extent(&p, "size200.mp4");
    let ratio = big / normal;
    assert!(
        (1.8..2.2).contains(&ratio),
        "ratio {ratio} ({normal} -> {big})"
    );
}

/// Camera stills from a 4K x 2K file and from the 4096 x 2048 copy of an 8K x 4K file:
/// the copy has the promised size, both stills show the right direction and the latency
/// is measured (an estimate of the plan, not a gate; only a generous bound is checked).
// covers: REQ-104
#[test]
fn big_spheres_get_a_4096_copy_and_camera_stills_are_quick_enough() {
    let Some(env) = setup() else { return };
    let line = |w: u32, h: u32| {
        let x = ((LINE_LON + 180.0) / 360.0 * f64::from(w)) as u32 - w / 700;
        format!(
            "color=c=black:s={w}x{h}:r=5:d=1,drawbox=x={x}:y=0:w={}:h={h}:color=white:t=fill",
            w / 340
        )
    };
    let big = env.fixture(
        "sphere8k.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            &line(7680, 3840),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let mut p = Project::default();
    let m = env.probe(&mut p, &big);
    assert_eq!((m.format.width, m.format.height), (7680, 3840));
    p.media[0].format.treat_as_360 = Some(true);
    assert!(p.media[0].format.is_360());
    // the copy: 4096 x 2048, made with the proxy's own arguments
    let copy = env.dir.path().join("copy.mkv");
    let args =
        videolore::proxy::proxy_args(&big, &copy, &p.media[0].format, env.info.version, Some(2));
    let out = Command::new(&env.info.ffmpeg).args(&args).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut q = Project::default();
    let c = env.probe(&mut q, &copy);
    assert_eq!((c.format.width, c.format.height), (4096, 2048));
    // a camera still from the copy (what the preview reads) and from a 4K original
    let four = sphere_file(&env, 1);
    for (name, file) in [("copy", copy), ("4K", four)] {
        let mut p = Project::default();
        env.probe(&mut p, &file);
        p.media[0].format.treat_as_360 = Some(true);
        let m = p.media[0].id;
        let mut item = p.media[0].clone();
        item.format.treat_as_360 = Some(true);
        p.timeline.append(&item).unwrap();
        p.camera.set(m, pov(0.0, 10.0, 90.0)).unwrap();
        let t = Instant::now();
        let (f, w) = preview_still(&env, &p, Micros::ZERO);
        let took = t.elapsed();
        eprintln!("camera still from the {name}: {took:?}");
        near(
            line_direction(&f, w, 4, 90.0),
            LINE_LON - 10.0,
            &format!("still from the {name}"),
        );
        assert!(took < Duration::from_secs(10), "{name}: {took:?}");
    }
}
