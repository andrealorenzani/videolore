// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 3.2.0: a project imported as media (REQ-371, REQ-373) and the
//! picture strips of the timeline (REQ-369).

use super::*;
use std::sync::atomic::Ordering;
use videolore::nested::{self, Tools};
use videolore::strips::{Key, Strips, Want};

/// A 320x180, 10 fps, 4 s clip with sound: one second each of black, white, black, white.
/// The test gets its own copy: two tests making the cached clip for the first time replace
/// the shared file, and an import notices a file that changed.
fn flashing(env: &Env) -> PathBuf {
    let own = env.dir.path().join("flashing.mp4");
    let shared = env.fixture(
        "flashing.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x180:r=10:d=4,\
             drawbox=c=white:t=fill:enable='between(mod(n,20),10,19)'",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=4",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ],
    );
    std::fs::copy(shared, &own).unwrap();
    own
}

fn tools(env: &Env) -> Tools {
    Tools {
        ffmpeg: env.info.ffmpeg.clone(),
        ffprobe: env.info.ffprobe.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
        can_burn_subtitles: env.info.has_subtitles(),
        threads: Some(2),
        work_root: env.dir.path().join("work"),
    }
}

/// A project of one clip of `source` for each source range (in seconds), saved at `path`.
fn project_of(env: &Env, source: &Path, ranges: &[(i64, i64)], path: &Path) {
    let mut p = Project::default();
    let m = env.probe(&mut p, source);
    for (from, to) in ranges {
        let id = p.timeline.append(&m).unwrap();
        p.timeline
            .trim(
                id,
                Micros(from * 1_000_000),
                Micros(to * 1_000_000),
                m.duration,
                Micros(1),
            )
            .unwrap();
    }
    project::save(&p, path).unwrap();
}

fn import(env: &Env, path: &Path, importer: Option<&Path>) -> MediaItem {
    let source = nested::read(path, importer).unwrap();
    let mut seen = Vec::new();
    let item = nested::render(
        source,
        &tools(env),
        &limited(Priority::Export),
        &Arc::new(AtomicBool::new(false)),
        &mut |p| seen.push(p),
    )
    .unwrap();
    assert!(seen.iter().all(|p| (0.0..=1.0).contains(p)));
    item
}

/// Whether the picture of `file` at `t` seconds is light.
fn light(env: &Env, file: &Path, t: f64) -> bool {
    mean(&env.frame_rgb(file, t)) > 128.0
}

// covers: REQ-371, REQ-373
#[test]
fn an_imported_project_shows_its_timeline_and_follows_its_source() {
    let Some(env) = setup() else { return };
    let clip = flashing(&env);
    let source = env.dir.path().join("source.vlproj");
    // three clips: white, black, white
    project_of(&env, &clip, &[(1, 2), (2, 3), (3, 4)], &source);
    let item = import(&env, &source, None);
    assert!(
        near(item.duration, Micros(3_000_000), Micros(100_000)),
        "{:?}",
        item.duration
    );
    assert_eq!((item.format.width, item.format.height), (320, 180));
    assert_eq!(item.format.vcodec.as_deref(), Some("h264"));
    assert_eq!(item.format.acodec.as_deref(), Some("aac"));
    assert_eq!(item.from.as_deref(), Some(source.as_path()));
    let colours = |file: &Path| [0.5, 1.5, 2.5].map(|t| light(&env, file, t));
    assert_eq!(colours(&item.path), [true, false, true]);
    // made once: the same video is used again
    let made = sha256(&item.path);
    let modified = std::fs::metadata(&item.path).unwrap().modified().unwrap();
    let again = import(&env, &source, None);
    assert_eq!(again.path, item.path);
    assert_eq!(
        std::fs::metadata(&again.path).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(sha256(&again.path), made);
    // imported into another project, split and faded: the export shows the edited clips
    let mut p = Project::default();
    let mut m = item.clone();
    m.id = p.new_media_id();
    p.add_media(m.clone());
    p.timeline.append(&m).unwrap();
    p.timeline
        .split(Micros(1_000_000), Micros(100_000))
        .unwrap();
    p.timeline.move_to(p.timeline.clips[0].id, 1).unwrap();
    p.timeline.set_end_fade(true).unwrap();
    let outer = env.dir.path().join("outer.vlproj");
    project::save(&p, &outer).unwrap();
    let out = env.dir.path().join("out.mp4");
    env.export(&p, &out).0.unwrap();
    // black, white, then the first second (white) fading out
    assert_eq!(
        [0.3, 1.3, 2.1].map(|t| light(&env, &out, t)),
        [false, true, true]
    );
    assert!(mean(&env.frame_rgb(&out, 2.9)) < 90.0, "the fade");
    // two levels: a project that imports the project that imports the source
    let item = import(&env, &outer, None);
    assert!(near(item.duration, Micros(3_000_000), Micros(100_000)));
    // the source changes: a new video with its new timeline
    project_of(&env, &clip, &[(0, 1), (1, 2)], &source);
    let changed = import(&env, &source, None);
    assert_ne!(changed.path, again.path);
    assert!(near(changed.duration, Micros(2_000_000), Micros(100_000)));
    assert_eq!(
        [0.5, 1.5].map(|t| light(&env, &changed.path, t)),
        [false, true]
    );
}

// covers: REQ-371
#[test]
fn cancelling_an_import_leaves_no_file_behind() {
    let Some(env) = setup() else { return };
    let clip = env.clip("cancel.mp4", 320, 180, 10, 20, true);
    let source = env.dir.path().join("long.vlproj");
    project_of(&env, &clip, &[(0, 20), (0, 20), (0, 20)], &source);
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    flag.store(true, Ordering::SeqCst);
    let r = nested::render(
        nested::read(&source, None).unwrap(),
        &tools(&env),
        &limited(Priority::Export),
        &cancel,
        &mut |_| {},
    );
    assert_eq!(r.unwrap_err(), nested::NestedError::Cancelled);
    let left = std::fs::read_dir(env.dir.path().join("work").join(nested::FOLDER))
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(left, 0);
}

fn snapshots(env: &Env, wants: &[Want]) -> Vec<f64> {
    let runner = limited(Priority::Background);
    let mut s = Strips::default();
    let end = Instant::now() + Duration::from_secs(60);
    loop {
        s.begin_frame();
        for w in wants {
            s.want(w.clone());
        }
        s.tick(Instant::now(), &runner, &env.info.ffmpeg);
        if !s.busy() {
            break;
        }
        assert!(Instant::now() < end, "no snapshots");
        std::thread::sleep(Duration::from_millis(5));
    }
    wants
        .iter()
        .map(|w| {
            let t = s.get(&w.key).expect("a snapshot");
            // the colour part of the pixels
            let (pixels, _) = t.rgba.as_chunks::<4>();
            let rgb: Vec<u8> = pixels.iter().flat_map(|p| &p[..3]).copied().collect();
            mean(&rgb)
        })
        .collect()
}

// covers: REQ-369
#[test]
fn the_snapshots_show_the_source_at_their_own_times() {
    let Some(env) = setup() else { return };
    let clip = flashing(&env);
    let want = |at: i64, step: i64| Want {
        key: Key {
            media: videolore::media::MediaId(1),
            at: Micros(at),
            width: 36,
            height: 20,
        },
        step: Micros(step),
        path: clip.clone(),
    };
    // one run: a second apart from the start
    let near: Vec<Want> = (0..4).map(|k| want(k * 1_000_000, 1_000_000)).collect();
    let started = Instant::now();
    let m = snapshots(&env, &near);
    eprintln!("4 snapshots of one run: {:?}", started.elapsed());
    assert!(
        m[0] < 60.0 && m[1] > 190.0 && m[2] < 60.0 && m[3] > 190.0,
        "{m:?}"
    );
    // sought one by one, further apart than a run reads
    let far = [want(1_500_000, 9_000_000), want(2_500_000, 9_000_000)];
    let m = snapshots(&env, &far);
    assert!(m[0] > 190.0 && m[1] < 60.0, "{m:?}");
}
