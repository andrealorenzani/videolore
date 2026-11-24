// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 3.1.0: the blurred fill of a Change aspect (REQ-335).

use super::*;
use videolore::frame::{Fill, Fit, FrameAspect, Shape};

/// A 1600x1200, 5 fps, 2 s clip: the left half white, the right half black.
fn wide_edge(env: &Env) -> PathBuf {
    env.fixture(
        "edge-1600x1200.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=white:s=800x1200:r=5:d=2",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=800x1200:r=5:d=2",
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

fn px(buf: &[u8], w: usize, x: usize, y: usize) -> i32 {
    i32::from(buf[(y * w + x) * 3])
}

fn tall_blur() -> FrameAspect {
    FrameAspect {
        shape: Shape::Tall9x16,
        fit: Fit::Width,
        fill: Fill::Blur,
        blur: Default::default(),
    }
}

fn frame_of(env: &Env, p: &Project, name: &str) -> (usize, usize, Vec<u8>) {
    let out = env.dir.path().join(name);
    env.export(p, &out).0.unwrap();
    let o = Command::new(&env.info.ffprobe)
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
        .args(["stream=width,height", "-of", "csv=p=0"])
        .arg(&out)
        .output()
        .unwrap();
    let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
    let (w, h) = t.split_once(',').unwrap();
    (
        w.parse().unwrap(),
        h.parse().unwrap(),
        env.frame_rgb(&out, 0.6),
    )
}

// covers: REQ-335
#[test]
fn the_blurred_fill_shows_for_the_whole_video_and_for_a_clips_own_aspect() {
    let Some(env) = setup() else { return };
    let file = wide_edge(&env);
    // the whole video: 1600x1200 in 9:16 with Fit on width
    let mut p = Project::default();
    let m = env.probe(&mut p, &file);
    p.timeline.append(&m).unwrap();
    p.timeline.set_frame_aspect(Some(tall_blur())).unwrap();
    let (w, h, f) = frame_of(&env, &p, "whole.mp4");
    assert_eq!((w, h), (1600, 2844));
    // the picture fills rows 822..2022 and keeps its sharp edge in the middle
    let sharp = (px(&f, w, 796, h / 2) - px(&f, w, 804, h / 2)).abs();
    assert!(sharp > 200, "{sharp}");
    // the bands show a blurred copy: bright left, dark right, a gentle step at the middle,
    // and no sharp edge at the picture's border (rows 821 and 823 are alike)
    let (l, r) = (px(&f, w, 100, 300), px(&f, w, 1500, 300));
    assert!(l > 150 && r < 100, "{l} {r}");
    assert!((px(&f, w, 790, 300) - px(&f, w, 810, 300)).abs() < 120);
    assert!((px(&f, w, 100, 815) - px(&f, w, 100, 830)).abs() < 40);
    // a clip's own Change aspect: the frame is placed in the 1600x1200 picture
    let mut q = Project::default();
    let m = env.probe(&mut q, &file);
    q.timeline.append(&m).unwrap();
    let id = q.timeline.clips[0].id;
    q.timeline.set_clip_aspect(id, Some(tall_blur())).unwrap();
    let (w, h, f) = frame_of(&env, &q, "own.mp4");
    assert_eq!((w, h), (1600, 1200));
    // the 9:16 frame is 675 px wide in the middle; the picture is 506 px high in it
    let sharp = (px(&f, w, 796, 600) - px(&f, w, 804, 600)).abs();
    assert!(sharp > 200, "{sharp}");
    let (l, r) = (px(&f, w, 500, 100), px(&f, w, 1100, 100));
    assert!(
        l > 150 && r < 100,
        "the band above is a blurred copy: {l} {r}"
    );
}

// covers: REQ-335
#[test]
fn the_preview_of_a_clips_own_blurred_aspect_matches_the_whole_render_and_has_the_fill() {
    use videolore::frame::Scale;
    use videolore::timeline::FrameTarget;
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let file = env.clip("own-bg.mp4", 320, 240, 25, 2, false);
    let m = env.probe(&mut p, &file);
    p.timeline.append(&m).unwrap();
    let id = p.timeline.clips[0].id;
    p.timeline.set_clip_aspect(id, Some(tall_blur())).unwrap();
    p.timeline.add_scale(FrameTarget::Whole).unwrap();
    let scale = |p: &mut Project, permille: u16| {
        let c0 = FrameTarget::Clip(p.timeline.clips[0].id);
        p.timeline
            .set_scale_key(
                c0,
                Micros::ZERO,
                Scale {
                    permille,
                    dx: 0,
                    dy: 0,
                },
            )
            .unwrap();
    };
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(NullSink));
    let mut still = |p: &Project| {
        pv.request_still(p, Micros(1_000_000), Instant::now())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(f) = pv.poll(Instant::now() + Duration::from_secs(1)) {
                return f;
            }
            assert!(Instant::now() < deadline, "no frame");
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    scale(&mut p, 500);
    still(&p);
    scale(&mut p, 800);
    let kept = still(&p);
    // the whole render of the same picture
    let eg = render::output_geometry(&p, &env.info.encoders).unwrap();
    let work = tempfile::tempdir().unwrap();
    let r = render::still_args(
        &p,
        Micros(1_000_000),
        &eg,
        env.info.version,
        &work.path().join("g.txt"),
        render::Source::Preview { proxies: false },
    )
    .unwrap();
    let out = Command::new(&env.info.ffmpeg)
        .current_dir(work.path())
        .args(&r.args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout.len(), kept.rgba.len());
    let sum: u64 = kept
        .rgba
        .iter()
        .zip(&out.stdout)
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum();
    let mean = sum as f64 / kept.rgba.len() as f64;
    assert!(mean < 2.0, "mean difference {mean}");
    // the preview is not the original picture again: the band above the picture inside the
    // frame is a blurred copy (not black) while the pillars outside the frame are black
    let (w, h) = (kept.width as usize, kept.height as usize);
    let at = |x: usize, y: usize| i32::from(kept.rgba[(y * w + x) * 4 + 1]);
    assert!(at(w / 2, h / 12) > 20, "the fill shows above the picture");
    assert_eq!(at(2, h / 2), 0, "outside the frame is black");
}

/// A 2 s clip with a flat blue picture and two sounds: a 440 Hz tone first and a 880 Hz tone
/// second, one stream each.
fn two_sounds(env: &Env) -> PathBuf {
    env.fixture(
        "two-sounds.mkv",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=160x90:r=10:d=2",
            "-f",
            "lavfi",
            "-i",
            "sine=f=440:d=2:sample_rate=48000",
            "-f",
            "lavfi",
            "-i",
            "sine=f=880:d=2:sample_rate=48000",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ],
    )
}

/// The count of streams of `kind` (`a`, `s`) in a file and their codec names.
fn streams(env: &Env, file: &Path, kind: &str) -> Vec<String> {
    let o = Command::new(&env.info.ffprobe)
        .args(["-v", "error", "-select_streams", kind])
        .args(["-show_entries", "stream=codec_name", "-of", "csv=p=0"])
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// The pitch of the first audio stream, from the crossings of zero in its first second.
fn tone_hz(env: &Env, file: &Path) -> usize {
    let o = Command::new(&env.info.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(file)
        .args(["-map", "0:a:0", "-t", "1", "-ac", "1", "-ar", "8000"])
        .args(["-f", "s16le", "pipe:1"])
        .output()
        .unwrap();
    let (samples, _) = o.stdout.as_chunks::<2>();
    let v: Vec<i16> = samples.iter().map(|s| i16::from_le_bytes(*s)).collect();
    let crossings = v.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count();
    crossings / 2
}

// covers: REQ-340, REQ-086
#[test]
fn a_file_with_two_sounds_lists_both_and_the_export_has_the_chosen_one_only() {
    let Some(env) = setup() else { return };
    let file = two_sounds(&env);
    let mut p = Project::default();
    let m = env.probe(&mut p, &file);
    // the analysis records both tracks, in order
    let tracks = m.format.audio_tracks.clone().unwrap();
    assert_eq!(tracks.iter().map(|t| t.ordinal).collect::<Vec<_>>(), [0, 1]);
    p.timeline.append(&m).unwrap();
    // the first track by default
    let out = env.dir.path().join("first.mkv");
    env.export(&p, &out).0.unwrap();
    let hz = tone_hz(&env, &out);
    assert!((420..=460).contains(&hz), "the first track is 440 Hz: {hz}");
    // the second track only: one audio stream in the file, and it is the 880 Hz tone
    let id = p.timeline.clips[0].id;
    p.timeline.set_audio_track(id, 1).unwrap();
    let out = env.dir.path().join("second.mkv");
    env.export(&p, &out).0.unwrap();
    assert_eq!(streams(&env, &out, "a").len(), 1, "one sound only");
    let hz = tone_hz(&env, &out);
    assert!(
        (850..=910).contains(&hz),
        "the second track is 880 Hz: {hz}"
    );
}

// covers: REQ-340
#[test]
fn the_preview_plays_the_chosen_track() {
    let Some(env) = setup() else { return };
    let file = two_sounds(&env);
    let mut p = Project::default();
    let m = env.probe(&mut p, &file);
    p.timeline.append(&m).unwrap();
    let id = p.timeline.clips[0].id;
    let pitch = |p: &Project| {
        let work = tempfile::tempdir().unwrap();
        let args = render::preview_audio_args(
            p,
            Micros::ZERO,
            8000,
            1,
            env.info.version,
            &work.path().join("a.txt"),
            render::Source::Preview { proxies: false },
        )
        .unwrap();
        let o = Command::new(&env.info.ffmpeg)
            .current_dir(work.path())
            .args(&args)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let (samples, _) = o.stdout.as_chunks::<2>();
        let v: Vec<i16> = samples.iter().map(|s| i16::from_le_bytes(*s)).collect();
        let first = &v[..8000.min(v.len())];
        first
            .windows(2)
            .filter(|w| (w[0] < 0) != (w[1] < 0))
            .count()
            / 2
    };
    assert!((420..=460).contains(&pitch(&p)));
    p.timeline.set_audio_track(id, 1).unwrap();
    let hz = pitch(&p);
    assert!(
        (850..=910).contains(&hz),
        "the preview plays the second track: {hz}"
    );
}

/// A 2 s flat blue clip in `name` (`.mp4` or `.mkv`).
fn blue(env: &Env, name: &str) -> PathBuf {
    env.fixture(
        name,
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

// covers: REQ-337
#[test]
fn as_subtitle_track_writes_one_track_with_the_cues_and_no_text_on_the_picture() {
    use videolore::subtitles::Preset;
    let Some(env) = setup() else { return };
    for name in ["as-track.mp4", "as-track.mkv"] {
        let file = blue(&env, name);
        let mut p = Project::default();
        let m = env.probe(&mut p, &file);
        p.timeline.append(&m).unwrap();
        let tl = p.timeline.clone();
        for (t, text) in [(0, "first line"), (1, "second line"), (2, "third line")] {
            p.subtitles
                .add_at(&tl, Micros(t * 1_000_000), text, Micros(900_000))
                .unwrap();
        }
        p.subtitles.language = Some("it".into());
        p.subtitles.preset = Preset::AsTrack;
        let out = env.dir.path().join(format!("out-{name}"));
        env.export(&p, &out).0.unwrap();
        let subs = streams(&env, &out, "s");
        let codec = if name.ends_with("mp4") {
            "mov_text"
        } else {
            "subrip"
        };
        assert_eq!(subs, [codec], "one subtitle stream in {name}");
        // its cues, as SRT
        let srt = env.dir.path().join(format!("{name}.srt"));
        let o = Command::new(&env.info.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(&out)
            .args(["-map", "0:s:0"])
            .arg(&srt)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        let text = std::fs::read_to_string(&srt).unwrap();
        assert_eq!(text.matches("-->").count(), 3, "{text}");
        assert!(text.contains("second line"));
        // the language of the project
        let lang = Command::new(&env.info.ffprobe)
            .args(["-v", "error", "-select_streams", "s", "-show_entries"])
            .args(["stream_tags=language", "-of", "csv=p=0"])
            .arg(&out)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&lang.stdout).trim(), "ita");
        // the picture at a line's time has no text: flat blue all over
        let f = env.frame_rgb(&out, 1.3);
        let white = f
            .chunks(3)
            .filter(|c| c[0] > 150 && c[1] > 150 && c[2] > 150)
            .count();
        assert_eq!(white, 0, "no text is drawn in {name}");
    }
    // with another style the lines are drawn and there is no subtitle stream
    let file = blue(&env, "as-track.mp4");
    let mut p = Project::default();
    let m = env.probe(&mut p, &file);
    p.timeline.append(&m).unwrap();
    let tl = p.timeline.clone();
    p.subtitles
        .add_at(&tl, Micros(1_000_000), "drawn line", Micros(1_500_000))
        .unwrap();
    let out = env.dir.path().join("classic.mp4");
    env.export(&p, &out).0.unwrap();
    assert!(streams(&env, &out, "s").is_empty());
}
