// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! End-to-end tests of videolore player with the real ffmpeg. Fixtures are generated at
//! test time (lavfi) in a temporary folder. Without ffmpeg the tests are skipped, unless
//! VIDEOLORE_REQUIRE_FFMPEG=1 (then they fail).

use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::Arc;
use std::time::{Duration, Instant};
use videolore::camera::Camera;
use videolore::ffmpeg::{self, FfmpegInfo, SystemRunner};
use videolore::preview::NullSink;
use videolore::time::Micros;
use videolore_player::app::{Command, Dialogs, Folders, PlayerState};

struct NoDialogs;

impl Dialogs for NoDialogs {
    fn pick_video(&mut self) -> Option<PathBuf> {
        None
    }
    fn pick_srt(&mut self) -> Option<PathBuf> {
        None
    }
}

struct Env {
    info: FfmpegInfo,
    dir: tempfile::TempDir,
}

fn setup() -> Option<Env> {
    let found = ffmpeg::locate_both(None, None)
        .map_err(|e| e.to_string())
        .and_then(|l| ffmpeg::check_located(&SystemRunner, &l).map_err(|e| e.to_string()));
    match found {
        Ok(info) => Some(Env {
            info,
            dir: tempfile::tempdir().unwrap(),
        }),
        Err(e) => {
            if std::env::var("VIDEOLORE_REQUIRE_FFMPEG").as_deref() == Ok("1") {
                panic!("ffmpeg is required but unusable: {e}");
            }
            eprintln!("skipping: ffmpeg unavailable ({e})");
            None
        }
    }
}

impl Env {
    fn ffmpeg(&self, args: &[&str]) {
        let o = Process::new(&self.info.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(args)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }

    fn videos(&self) -> PathBuf {
        let d = self.dir.path().join("videos");
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A 3 s 320x180 clip: a moving test pattern with a sine.
    fn video(&self, name: &str) -> PathBuf {
        let p = self.videos().join(name);
        self.ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=320x180:r=10:d=3",
            "-f",
            "lavfi",
            "-i",
            "sine=f=440:d=3",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            p.to_str().unwrap(),
        ]);
        p
    }

    /// The repository's 360 fixture (an equirectangular file that says so), copied.
    fn sphere(&self, name: &str) -> PathBuf {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/spherical-360.mp4");
        let p = self.videos().join(name);
        std::fs::copy(src, &p).unwrap();
        p
    }

    /// An mkv with a picture and two text subtitle tracks (English, Italian).
    fn with_tracks(&self, name: &str) -> PathBuf {
        let d = self.dir.path();
        let (en, it) = (d.join("en.srt"), d.join("it.srt"));
        std::fs::write(&en, "1\n00:00:00,500 --> 00:00:02,500\nHELLO WORLD\n").unwrap();
        std::fs::write(&it, "1\n00:00:00,500 --> 00:00:02,500\nCIAO MONDO\n").unwrap();
        let p = self.videos().join(name);
        self.ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:s=320x180:r=10:d=3",
            "-i",
            en.to_str().unwrap(),
            "-i",
            it.to_str().unwrap(),
            "-map",
            "0:v",
            "-map",
            "1",
            "-map",
            "2",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:s",
            "srt",
            "-metadata:s:s:0",
            "language=eng",
            "-metadata:s:s:1",
            "language=ita",
            p.to_str().unwrap(),
        ]);
        p
    }

    /// An mkv with a blue picture, a text track and a Blu-ray picture track (PGS) made by
    /// [`pgs`]: a white box 60 x 20 at (120, 130) from 1 s to 2 s.
    fn with_picture_track(&self, name: &str) -> PathBuf {
        let d = self.dir.path();
        let (sup, en) = (d.join("box.sup"), d.join("en.srt"));
        std::fs::write(&sup, pgs(320, 180, (120, 130, 60, 20), 1.0, 2.0)).unwrap();
        std::fs::write(&en, "1\n00:00:00,500 --> 00:00:02,500\nHELLO WORLD\n").unwrap();
        let p = self.videos().join(name);
        self.ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=320x180:r=10:d=3",
            "-i",
            en.to_str().unwrap(),
            "-f",
            "sup",
            "-i",
            sup.to_str().unwrap(),
            "-map",
            "0:v",
            "-map",
            "1",
            "-map",
            "2",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:s:0",
            "srt",
            "-c:s:1",
            "copy",
            "-metadata:s:s:1",
            "language=eng",
            p.to_str().unwrap(),
        ]);
        p
    }

    fn player(&self) -> PlayerState {
        PlayerState::new(
            Ok(self.info.clone()),
            Arc::new(SystemRunner),
            Box::new(NoDialogs),
            Box::new(NullSink),
            videolore::machine::Machine::detect(),
            Folders {
                cache_dir: None,
                work_root: self.dir.path().join("work"),
            },
            Instant::now(),
        )
    }
}

/// Ticks until `done` or 20 seconds pass.
fn until(p: &mut PlayerState, what: &str, mut done: impl FnMut(&PlayerState) -> bool) {
    let end = Instant::now() + Duration::from_secs(20);
    loop {
        p.tick(Instant::now());
        if done(p) {
            return;
        }
        assert!(
            Instant::now() < end,
            "timed out waiting for: {what} ({})",
            p.status
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn open(p: &mut PlayerState, path: &Path) {
    p.dispatch(Command::Open(path.to_path_buf()), Instant::now());
    until(p, "the video to open", |p| {
        p.has_video() || p.notice.is_some()
    });
    assert!(p.has_video(), "{:?}", p.notice);
}

/// Waits for the next preview picture after `serial`.
fn picture_after(p: &mut PlayerState, serial: u64) -> Vec<u8> {
    until(p, "a preview picture", |p| {
        p.frame_serial > serial && p.frame.is_some()
    });
    p.frame.as_ref().unwrap().rgba.clone()
}

fn snapshot(dir: &Path) -> Vec<(String, u64)> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                e.metadata().unwrap().len(),
            )
        })
        .collect();
    v.sort();
    v
}

// covers: REQ-138
#[test]
fn an_mp4_opens_plays_pauses_seeks_and_changes_volume_without_touching_its_folder() {
    let Some(env) = setup() else { return };
    let movie = env.video("a.mp4");
    let folder = movie.parent().unwrap().to_path_buf();
    let before = snapshot(&folder);
    let mut p = env.player();
    open(&mut p, &movie);
    assert_eq!(p.duration(), Micros(3_000_000));
    // the first picture appears by itself
    let first = picture_after(&mut p, 0);
    assert!(!first.is_empty());
    // Play: the playhead moves
    p.dispatch(Command::PlayPause, Instant::now());
    assert!(p.is_playing());
    until(&mut p, "the playhead to move", |p| {
        p.playhead > Micros(600_000)
    });
    // volume and mute while playing: playback goes on
    p.dispatch(Command::SetVolume(40), Instant::now());
    p.dispatch(Command::ToggleMute, Instant::now());
    assert!(p.is_playing() && p.muted() && p.volume() == 40);
    p.dispatch(Command::PlayPause, Instant::now());
    assert!(!p.is_playing());
    let stopped = p.playhead;
    // Seek: a different picture
    let serial = p.frame_serial;
    p.dispatch(Command::Seek(Micros(2_000_000)), Instant::now());
    let later = picture_after(&mut p, serial);
    assert_ne!(first, later, "another moment, another picture");
    assert_eq!(p.playhead, Micros(2_000_000));
    assert_ne!(stopped, p.playhead);
    // playing to the end stops at the end
    p.dispatch(Command::PlayPause, Instant::now());
    until(&mut p, "the end", |p| {
        !p.is_playing() || p.playhead >= Micros(3_000_000)
    });
    drop(p);
    assert_eq!(
        snapshot(&folder),
        before,
        "nothing was written next to the video"
    );
}

// covers: REQ-140
#[test]
fn a_chosen_embedded_track_and_an_opened_srt_show_their_lines_on_the_picture() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        eprintln!("skipping: this ffmpeg cannot burn subtitles");
        return;
    }
    let movie = env.with_tracks("tracks.mkv");
    let folder = movie.parent().unwrap().to_path_buf();
    let before = snapshot(&folder);
    let mut p = env.player();
    open(&mut p, &movie);
    let names: Vec<String> = p.subs.sources.iter().map(|s| s.name()).collect();
    assert_eq!(names, ["English", "Italian"]);
    p.dispatch(Command::Seek(Micros(1_500_000)), Instant::now());
    let plain = picture_after(&mut p, 0);
    // the Italian track: its text appears at its time (the picture changes)
    let serial = p.frame_serial;
    p.dispatch(Command::ChooseSubtitles(Some(1)), Instant::now());
    until(&mut p, "the track to be read", |p| {
        p.subs.selected == Some(1) && !p.project.subtitles.lines.is_empty()
    });
    assert_eq!(p.project.subtitles.lines[0].text, "CIAO MONDO");
    let with = picture_after(&mut p, serial);
    assert_ne!(plain, with, "the line is drawn on the picture");
    // None takes it away again
    let serial = p.frame_serial;
    p.dispatch(Command::ChooseSubtitles(None), Instant::now());
    let without = picture_after(&mut p, serial);
    assert_eq!(plain, without);
    // an .srt file is listed and drawn too, then dragging a line moves it
    let srt = env.dir.path().join("own.srt");
    std::fs::write(&srt, "1\n00:00:00,500 --> 00:00:02,500\nMY OWN TEXT\n").unwrap();
    let serial = p.frame_serial;
    p.dispatch(Command::OpenSrt(srt), Instant::now());
    let own = picture_after(&mut p, serial);
    assert_ne!(own, plain);
    assert_eq!(p.subs.sources.len(), 3);
    let id = p.project.subtitles.lines[0].id;
    let serial = p.frame_serial;
    p.dispatch(
        Command::PlaceLines {
            ids: vec![id],
            pos: videolore::base::pos::Pos { x: 2_000, y: 1_500 },
        },
        Instant::now(),
    );
    let moved = picture_after(&mut p, serial);
    assert_ne!(moved, own, "the line is somewhere else");
    drop(p);
    assert_eq!(
        snapshot(&folder),
        before,
        "no file written next to the video"
    );
}

// covers: REQ-139
#[test]
fn dragging_a_360_video_turns_the_picture_and_the_view_is_the_editors_camera() {
    let Some(env) = setup() else { return };
    let movie = env.sphere("sphere.mp4");
    let mut p = env.player();
    open(&mut p, &movie);
    assert!(p.is_360(), "the file says it is a 360 video");
    let front = picture_after(&mut p, 0);
    let serial = p.frame_serial;
    p.dispatch(
        Command::DragCamera {
            dx: 120.0,
            dy: 30.0,
            view_w: 480.0,
        },
        Instant::now(),
    );
    assert_eq!(p.camera(), Camera::FRONT.dragged(120.0, 30.0, 480.0));
    let turned = picture_after(&mut p, serial);
    assert_ne!(front, turned, "the picture shows another direction");
    // zoom changes the picture too, within the limits
    let serial = p.frame_serial;
    p.dispatch(Command::ZoomCamera(200.0), Instant::now());
    assert!(p.camera().is_in_range());
    let zoomed = picture_after(&mut p, serial);
    assert_ne!(turned, zoomed);
    // while playing a drag restarts the stream from the playhead
    p.dispatch(Command::PlayPause, Instant::now());
    until(&mut p, "playing", |p| p.playhead > Micros(300_000));
    p.dispatch(
        Command::DragCamera {
            dx: -50.0,
            dy: 0.0,
            view_w: 480.0,
        },
        Instant::now(),
    );
    until(&mut p, "the restart", |p| p.is_playing());
    std::thread::sleep(Duration::from_millis(300));
    p.tick(Instant::now());
    assert!(p.is_playing());
}

/// One PGS (Blu-ray) segment: the "PG" tag, the time in 90 kHz ticks, the type and the size.
fn pgs_segment(ticks: u32, kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = b"PG".to_vec();
    v.extend(ticks.to_be_bytes());
    v.extend(0u32.to_be_bytes());
    v.push(kind);
    v.extend((payload.len() as u16).to_be_bytes());
    v.extend(payload);
    v
}

/// A minimal PGS subtitle stream, written by hand for the tests (own work, GPL-3.0 like the
/// rest of the repository: no file of a film or of another project is used): an empty picture
/// at 0 s, a white box `(x, y, w, h)` on a `width` x `height` picture from `start` to `end`
/// seconds, and an empty picture again. Each display set is a presentation, a window, a palette
/// (one white entry), an object (run-length coded rows of palette entry 1) and the end segment.
fn pgs(
    width: u16,
    height: u16,
    (x, y, w, h): (u16, u16, u16, u16),
    start: f64,
    end: f64,
) -> Vec<u8> {
    let display = |secs: f64, number: u16, boxed: bool| {
        let t = (secs * 90_000.0) as u32;
        let mut pcs = Vec::new();
        pcs.extend(width.to_be_bytes());
        pcs.extend(height.to_be_bytes());
        pcs.extend([0x10]);
        pcs.extend(number.to_be_bytes());
        pcs.extend([0x80, 0, 0, u8::from(boxed)]);
        let mut out = Vec::new();
        if boxed {
            pcs.extend([0, 0, 0, 0]);
            pcs.extend(x.to_be_bytes());
            pcs.extend(y.to_be_bytes());
            out.extend(pgs_segment(t, 0x16, &pcs));
            let mut wds = vec![1, 0];
            for v in [x, y, w, h] {
                wds.extend(v.to_be_bytes());
            }
            out.extend(pgs_segment(t, 0x17, &wds));
            // palette 0, version 0: entry 1 = white (Y 235, Cr 128, Cb 128), opaque
            out.extend(pgs_segment(t, 0x14, &[0, 0, 1, 235, 128, 128, 255]));
            // a row: a run of w pixels of entry 1 (0x00, 0x80 | length, entry), then 0x00 0x00
            let row = [0, 0x80 | w as u8, 1, 0, 0];
            let rle: Vec<u8> = row
                .iter()
                .copied()
                .cycle()
                .take(row.len() * usize::from(h))
                .collect();
            let mut ods = vec![0, 0, 0, 0xC0];
            ods.extend(&((rle.len() + 4) as u32).to_be_bytes()[1..]);
            ods.extend(w.to_be_bytes());
            ods.extend(h.to_be_bytes());
            ods.extend(rle);
            out.extend(pgs_segment(t, 0x15, &ods));
        } else {
            out.extend(pgs_segment(t, 0x16, &pcs));
        }
        out.extend(pgs_segment(t, 0x80, &[]));
        out
    };
    // the stream starts with an empty picture: ffmpeg counts the time from the first segment
    let mut v = display(0.0, 1, false);
    v.extend(display(start, 2, true));
    v.extend(display(end, 3, false));
    v
}

// covers: REQ-253
#[test]
fn a_picture_subtitle_track_is_listed_marked_picture_and_drawn_at_its_time_without_dragging() {
    let Some(env) = setup() else { return };
    let movie = env.with_picture_track("pgs.mkv");
    let mut p = env.player();
    open(&mut p, &movie);
    let menu: Vec<String> = p.subs.sources.iter().map(|s| s.menu_name()).collect();
    assert_eq!(
        menu,
        ["Track 1", "English (picture)"],
        "text first, then the picture track"
    );
    // the box is white at (150, 140) of the 320 x 180 picture; the background is blue
    let white = |p: &PlayerState| {
        let f = p.frame.as_ref().unwrap();
        let i = (140 * f.width as usize + 150) * 4;
        f.rgba[i] > 200 && f.rgba[i + 2] > 200 && f.rgba[i + 1] > 200
    };
    p.dispatch(Command::ChooseSubtitles(Some(1)), Instant::now());
    assert_eq!(p.subs.selected, Some(1));
    assert!(p.project.picture_sub.is_some() && p.project.subtitles.lines.is_empty());
    assert!(
        p.line_boxes(&mut |_, _, _| eframe::egui::Vec2::ZERO)
            .is_empty(),
        "nothing to drag"
    );
    p.dispatch(Command::PlayPause, Instant::now());
    let mut seen_before = false;
    let mut seen_inside = false;
    let end = Instant::now() + Duration::from_secs(30);
    let mut last_pts = Micros::ZERO;
    while Instant::now() < end && last_pts < Micros(2_600_000) {
        p.tick(Instant::now());
        if let Some(f) = &p.frame
            && f.pts != last_pts
        {
            last_pts = f.pts;
            let w = white(&p);
            match f.pts.0 {
                t if t < 900_000 => {
                    assert!(!w, "no box at {t} us");
                    seen_before = true;
                }
                t if (1_200_000..1_800_000).contains(&t) => {
                    assert!(w, "the box is drawn at {t} us");
                    seen_inside = true;
                }
                t if t > 2_300_000 => assert!(!w, "the box is gone at {t} us"),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        seen_before && seen_inside,
        "frames before and inside the box were checked"
    );
    // a text track chosen afterwards takes the picture track away
    p.dispatch(Command::PlayPause, Instant::now());
    p.dispatch(Command::ChooseSubtitles(None), Instant::now());
    assert!(p.project.picture_sub.is_none());
}
