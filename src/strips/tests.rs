// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

use super::*;
use crate::ffmpeg::CmdOutput;
use crate::ffmpeg::fake::FakeRunner;
use crate::jobs::{Priority, Scheduler};
use crate::timeline::tests::media;

fn clip(secs: i64) -> Clip {
    let mut tl = crate::timeline::Timeline::default();
    tl.append(&media(1, secs, true)).unwrap();
    tl.clips[0]
}

/// ffmpeg answers with as many pictures as asked for, at most `most`.
pub(crate) fn pictures(most: usize) -> FakeRunner {
    FakeRunner::default().on(move |_, a| {
        let after = |k: &str| a.iter().position(|x| x == k).map(|i| a[i + 1].clone());
        let n: usize = after("-frames:v")?.parse().ok()?;
        let filter = after("-vf")?;
        let size = filter.split("scale=").nth(1)?.split(":force").next()?;
        let (w, h) = size.split_once(':')?;
        let bytes = w.parse::<usize>().ok()? * h.parse::<usize>().ok()? * 4 * n.min(most);
        Some(CmdOutput {
            success: true,
            stdout: vec![7; bytes],
            stderr: String::new(),
        })
    })
}

fn limited(sched: &Arc<Scheduler>, runner: &Arc<FakeRunner>) -> Limited {
    sched.runner(Priority::Background, runner.clone())
}

fn want(at: i64, step: i64) -> Want {
    Want {
        key: Key {
            media: MediaId(1),
            at: Micros(at * 1_000_000),
            width: 36,
            height: 20,
        },
        step: Micros(step * 1_000_000),
        path: "/m/1.mp4".into(),
    }
}

/// Frames of the window until nothing is left to make.
fn settle(s: &mut Strips, wants: &[Want], r: &Limited) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        s.begin_frame();
        for w in wants {
            s.want(w.clone());
        }
        s.tick(Instant::now(), r, Path::new("ffmpeg"));
        if !s.busy() {
            return;
        }
        assert!(Instant::now() < end, "the snapshots did not come");
        std::thread::sleep(Duration::from_millis(1));
    }
}

// covers: REQ-369
#[test]
fn a_clip_shows_the_snapshots_that_fit_the_zoom_from_its_start() {
    assert_eq!(box_size(20.0, 1280, 720), Some((36, 20)));
    let c = clip(10);
    // 20 points a second: a step of 2 s, five snapshots, the first at the start
    let (s, step) = slots(&c, 20.0, 36, (0.0, 10_000.0));
    assert_eq!(s.len(), 5);
    assert_eq!(
        s[0],
        Slot {
            x: 0.0,
            at: Micros::ZERO
        }
    );
    assert_eq!(step, Micros(2_000_000));
    assert_eq!(
        (s[4].x, s[4].at),
        (160.0, Micros(8_000_000)),
        "the last one fits before the end"
    );
    // zooming out shows fewer
    let fewer = slots(&c, 8.0, 36, (0.0, 10_000.0)).0;
    assert_eq!(fewer.len(), 2, "{fewer:?}");
    // a clip shorter than one snapshot shows none, one a little longer only the first
    assert!(slots(&clip(1), 20.0, 36, (0.0, 100.0)).0.is_empty());
    assert_eq!(slots(&clip(2), 20.0, 36, (0.0, 100.0)).0.len(), 1);
    // only the visible part is planned, and the times follow the clip's start and speed
    let mut long = clip(3600);
    long.src_in = Micros(100_000_000);
    long.speed = 2000;
    let (s, step) = slots(&long, 40.0, 36, (4_000.0, 4_200.0));
    assert!(s.len() <= 8 && s.iter().all(|x| x.x >= 3_960.0 && x.x <= 4_200.0));
    assert_eq!(step, Micros(2_000_000), "one second of the timeline");
    assert_eq!(s[1].at - s[0].at, step);
    assert_eq!(s[0].at.0 % 2_000_000, 0);
    assert!(slots(&c, f32::NAN, 36, (0.0, 1.0)).0.is_empty());
}

#[test]
fn sizes_follow_the_lower_half_and_the_picture_shape() {
    assert_eq!(box_size(7.0, 1280, 720), None);
    assert_eq!(box_size(f32::NAN, 1280, 720), None);
    assert_eq!(box_size(20.0, 0, 720), None);
    assert_eq!(box_size(500.0, 1280, 720), Some((114, 64)));
    assert_eq!(box_size(33.0, 720, 1280), Some((18, 32)));
    assert_eq!(box_size(64.0, 100_000, 10), Some((MAX_WIDTH, 64)));
    assert_eq!(grid_step(1000.0, 36), Some(1));
    assert_eq!(grid_step(10.0, 36), Some(5));
    assert_eq!(grid_step(0.0, 36), None);
}

#[test]
fn the_arguments_read_the_file_only_and_never_a_shell() {
    let mut w = want(3, 2);
    w.path = "/m/-odd name.mp4".into();
    let text: Vec<String> = args(&w, 4)
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let has = |run: &[&str]| text.windows(run.len()).any(|x| x == run);
    assert!(has(&["-protocol_whitelist", "file,pipe"]));
    assert!(has(&["-ss", "3.000000", "-i", "file:/m/-odd name.mp4"]));
    assert!(has(&["-frames:v", "4"]));
    assert!(text.iter().any(|a| a.starts_with(
        "fps=1000000/2000000:round=up,scale=36:20:force_original_aspect_ratio=decrease"
    )));
    assert!(
        !args(&w, 1)
            .iter()
            .any(|a| a.to_string_lossy().contains("fps="))
    );
}

// covers: REQ-369
#[test]
fn near_snapshots_come_from_one_run_and_far_ones_one_by_one() {
    let sched = Scheduler::new(3);
    let runner = Arc::new(pictures(usize::MAX));
    let r = limited(&sched, &runner);
    let mut s = Strips::default();
    let near: Vec<Want> = (0..5).map(|k| want(2 * k, 2)).collect();
    settle(&mut s, &near, &r);
    assert_eq!(s.started, 1, "one ffmpeg for the five");
    assert_eq!(s.get(&near[4].key).unwrap().rgba.len(), 36 * 20 * 4);
    assert_eq!(s.bytes(), 5 * 36 * 20 * 4);
    // shown again: nothing is read again
    settle(&mut s, &near, &r);
    assert_eq!(s.started, 1);
    let far: Vec<Want> = (1..4).map(|k| want(600 * k, 600)).collect();
    settle(&mut s, &far, &r);
    assert_eq!(s.started, 4);
    // the file of the media changed: made again
    s.forget(MediaId(1));
    assert_eq!(s.bytes(), 0);
    settle(&mut s, &near, &r);
    assert_eq!(s.started, 5);
    s.clear();
    assert!(s.wanted().is_empty() && s.get(&near[0].key).is_none());
}

#[test]
fn a_snapshot_that_cannot_be_read_stays_blank_and_is_not_tried_again() {
    let sched = Scheduler::new(3);
    let runner = Arc::new(pictures(2));
    let r = limited(&sched, &runner);
    let mut s = Strips::default();
    let five: Vec<Want> = (0..5).map(|k| want(k, 1)).collect();
    settle(&mut s, &five, &r);
    assert_eq!(s.started, 1);
    assert!(s.get(&five[1].key).is_some() && s.get(&five[2].key).is_none());
    settle(&mut s, &five, &r);
    assert_eq!(s.started, 1);
    // an ffmpeg that cannot start
    let none = Arc::new(FakeRunner::default());
    let mut s = Strips::default();
    settle(&mut s, &five, &limited(&sched, &none));
    assert_eq!((s.started, s.bytes()), (1, 0));
    assert_eq!(sched.in_use(), 0, "every slot is given back");
}

// covers: REQ-369
#[test]
fn a_run_nobody_waits_for_is_stopped_and_one_runs_at_a_time() {
    let sched = Scheduler::new(3);
    let runner = Arc::new(pictures(0).holding_streams());
    let r = limited(&sched, &runner);
    let mut s = Strips::default();
    let ffmpeg = Path::new("ffmpeg");
    let wait = |done: &dyn Fn() -> bool| {
        let end = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    s.begin_frame();
    s.want(want(0, 600));
    s.want(want(600, 600));
    s.tick(Instant::now(), &r, ffmpeg);
    wait(&|| sched.in_use() == 1);
    // the window never waits for the pictures
    let t = Instant::now();
    s.tick(Instant::now(), &r, ffmpeg);
    assert!(t.elapsed() < Duration::from_millis(200));
    assert_eq!((s.started, sched.in_use()), (1, 1));
    // the zoom changed: the old run is killed and the new one starts
    s.begin_frame();
    s.want(want(9, 600));
    s.tick(Instant::now(), &r, ffmpeg);
    assert_eq!(s.started, 2);
    wait(&|| sched.in_use() == 1);
    // a run that never answers is stopped, and its snapshot is not asked again
    s.tick(Instant::now() + JOB_TIMEOUT * 2, &r, ffmpeg);
    wait(&|| sched.in_use() == 0);
    assert!(!s.busy());
    assert_eq!(s.started, 2);
    // dropping the strips stops the run
    s.begin_frame();
    s.want(want(99, 600));
    s.tick(Instant::now(), &r, ffmpeg);
    wait(&|| sched.in_use() == 1);
    drop(s);
    wait(&|| sched.in_use() == 0);
}

// covers: REQ-369
#[test]
fn the_cache_keeps_at_most_32_mib_and_drops_the_oldest() {
    let mut s = Strips::default();
    let one = 256 * 64 * 4;
    let key = |k: i64| Key {
        media: MediaId(1),
        at: Micros(k),
        width: 256,
        height: 64,
    };
    let n = (CACHE_CAP / one) as i64 + 5;
    for k in 0..n {
        s.insert(
            key(k),
            Thumb {
                width: 256,
                height: 64,
                rgba: vec![0; one],
            },
        );
        assert!(s.bytes() <= CACHE_CAP);
    }
    assert!(s.get(&key(0)).is_none() && s.get(&key(4)).is_none());
    assert!(s.get(&key(5)).is_some() && s.get(&key(n - 1)).is_some());
    // the same snapshot again does not count twice
    let before = s.bytes();
    s.insert(
        key(n - 1),
        Thumb {
            width: 256,
            height: 64,
            rgba: vec![1; one],
        },
    );
    assert_eq!(s.bytes(), before);
}
