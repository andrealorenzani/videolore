// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 2.0.4: subtitles as a separate track (REQ-182) and the glyphs of
//! the bundled Arabic and Devanagari faces in libass (REQ-187).

use super::*;
use videolore::ass;
use videolore::format::{FormatChoice, SubtitleMode};
use videolore::subtitles::Preset;

fn gray_clip(env: &Env, ext: &str) -> PathBuf {
    env.fixture(
        &format!("gray-640x360-3.{ext}"),
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=gray:s=640x360:r=10:d=3",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=3",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ],
    )
}

/// A project of the gray clip with a line from 1 s to 2 s (language Italian).
fn with_line(env: &Env, with: bool) -> Project {
    with_line_in(env, with, "mp4")
}

fn with_line_in(env: &Env, with: bool, ext: &str) -> Project {
    let mut p = Project::default();
    let m = env.probe(&mut p, &gray_clip(env, ext));
    p.media[0].format.bit_rate = Some(20_000_000);
    p.timeline.append(&m).unwrap();
    if with {
        let tl = p.timeline.clone();
        p.subtitles
            .add_at(&tl, Micros(1_000_000), "HHHH HHHH", Micros(1_000_000))
            .unwrap();
        p.subtitles.language = Some("it".into());
    }
    p
}

fn export_as(env: &Env, p: &Project, out: &Path, mode: SubtitleMode) {
    let mut job = env.job(p, out);
    job.format = FormatChoice {
        subtitles: mode,
        ..Default::default()
    };
    let (tx, _rx) = channel();
    export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
}

/// `(codec_name, language, default disposition)` of every subtitle stream.
fn subtitle_streams(env: &Env, file: &Path) -> Vec<(String, String, i64)> {
    let o = Command::new(&env.info.ffprobe)
        .args(["-v", "error", "-print_format", "json", "-show_streams"])
        .arg(file)
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    v["streams"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["codec_type"] == "subtitle")
        .map(|s| {
            (
                s["codec_name"].as_str().unwrap().to_string(),
                s["tags"]["language"].as_str().unwrap_or("").to_string(),
                s["disposition"]["default"].as_i64().unwrap(),
            )
        })
        .collect()
}

// covers: REQ-182
#[test]
fn a_separate_track_is_one_subtitle_stream_with_the_language_and_the_picture_has_no_text() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        return;
    }
    let p = with_line(&env, true);
    let plain = with_line(&env, false);
    let d = env.dir.path();
    // burned in (the default): no subtitle stream, text in the picture at 1.5 s
    let burned = d.join("burned.mp4");
    export_as(&env, &p, &burned, SubtitleMode::default());
    assert!(subtitle_streams(&env, &burned).is_empty());
    // a separate track in MP4 and in MKV
    let sep = d.join("separate.mp4");
    export_as(&env, &p, &sep, SubtitleMode::Separate);
    assert_eq!(
        subtitle_streams(&env, &sep),
        [("mov_text".to_string(), "ita".to_string(), 0)]
    );
    let mkv = d.join("separate.mkv");
    export_as(
        &env,
        &with_line_in(&env, true, "mkv"),
        &mkv,
        SubtitleMode::Separate,
    );
    assert_eq!(
        subtitle_streams(&env, &mkv),
        [("subrip".to_string(), "ita".to_string(), 0)]
    );
    // the picture at the line's time has no text: it is the picture of an export without lines
    let none = d.join("none.mp4");
    export_as(&env, &plain, &none, SubtitleMode::default());
    let (a, b, c) = (
        env.frame_rgb(&sep, 1.5),
        env.frame_rgb(&none, 1.5),
        env.frame_rgb(&burned, 1.5),
    );
    assert!(
        (mean(&a) - mean(&b)).abs() < 0.5,
        "{} vs {}",
        mean(&a),
        mean(&b)
    );
    assert!(
        (mean(&c) - mean(&b)).abs() > 0.1,
        "burned text changes the picture: {} {}",
        mean(&c),
        mean(&b)
    );
    // the sound and the length are those of a normal export
    let dur = |f: &Path| {
        let o = Command::new(&env.info.ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=p=0",
            ])
            .arg(f)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse::<f64>()
            .unwrap()
    };
    assert!((dur(&sep) - dur(&burned)).abs() < 0.2);
}

/// Renders `doc` on black with libass (the ass filter) in a folder holding `fonts`, with
/// an empty font configuration so that no installed font can help; returns the gray picture.
fn render_ass(env: &Env, doc: &str, fonts: &[(&str, &[u8])]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let fdir = dir.path().join("fonts");
    std::fs::create_dir_all(&fdir).unwrap();
    for (name, bytes) in fonts {
        std::fs::write(fdir.join(name), bytes).unwrap();
    }
    std::fs::write(dir.path().join("subs.ass"), doc).unwrap();
    std::fs::write(
        dir.path().join("fc.conf"),
        format!(
            "<?xml version=\"1.0\"?><!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\
             <fontconfig><cachedir>{}</cachedir></fontconfig>",
            dir.path().join("cache").display()
        ),
    )
    .unwrap();
    let o = Command::new(&env.info.ffmpeg)
        .current_dir(dir.path())
        .env("FONTCONFIG_FILE", dir.path().join("fc.conf"))
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg("color=c=black:s=640x360:d=1")
        .args(["-vf", "ass=subs.ass:fontsdir=fonts,format=gray"])
        .args(["-frames:v", "1", "-f", "rawvideo", "pipe:1"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    o.stdout
}

fn doc_of(text: &str) -> String {
    use videolore::subtitles::ShownLine;
    let l = ShownLine {
        line: videolore::subtitles::LineId(1),
        clip: videolore::timeline::ClipId(1),
        start: Micros::ZERO,
        end: Micros(900_000),
        text: text.into(),
        words: vec![],
        position: None,
        size: None,
        anchor: None,
    };
    ass::document(&[l], Preset::Classic, 640, 360, Micros::ZERO)
}

// covers: REQ-187
#[test]
fn arabic_and_devanagari_render_their_glyphs_like_a_reference_render() {
    let Some(env) = setup() else { return };
    if !env.info.has_subtitles() {
        return;
    }
    let all = ass::BUNDLED_FACES;
    for (text, face_file, family) in [
        (
            "مرحبا بالعالم",
            "NotoSansArabic-Bold.ttf",
            ass::FAMILY_ARABIC,
        ),
        (
            "नमस्ते दुनिया",
            "NotoSansDevanagari-Bold.ttf",
            ass::FAMILY_DEVANAGARI,
        ),
    ] {
        let ours = render_ass(&env, &doc_of(text), &all);
        // the picture without the added runs: empty boxes (the base font has no glyphs)
        let unrun = doc_of(text).replace(&format!("{{\\fn{family}}}"), "");
        let boxes = render_ass(&env, &unrun, &all);
        // the reference: the whole line in the face of the script, nothing else
        let face = all.iter().find(|(n, _)| *n == face_file).unwrap();
        let reference_doc = doc_of(text)
            .replace(&format!("{{\\fn{family}}}"), "")
            .replace("{\\fnNoto Sans}", "")
            .replace(
                "Style: Default,Noto Sans,",
                &format!("Style: Default,{family},"),
            );
        let reference = render_ass(&env, &reference_doc, &[*face]);
        let ink = |p: &[u8]| p.iter().filter(|v| **v > 128).count();
        assert!(ink(&ours) > 100, "{family}: the line is drawn");
        assert_eq!(
            ours, reference,
            "{family}: the glyphs are those of the face"
        );
        assert_ne!(
            ours, boxes,
            "{family}: not the empty boxes of the base font"
        );
    }
}

/// With a blurred fill, a change of Scale at the same playhead draws the picture again over
/// the kept background; the result is the picture the whole render makes (the picture at the
/// same place, the background the same up to the rounding of two colour conversions).
// covers: REQ-223
#[test]
fn the_kept_blurred_background_gives_the_same_picture_as_the_whole_render() {
    use videolore::frame::{Fill, Fit, FrameAspect, Scale, Shape};
    use videolore::timeline::FrameTarget;
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let file = env.clip("kept-bg.mp4", 320, 180, 25, 2, false);
    let m = env.probe(&mut p, &file);
    p.timeline.append(&m).unwrap();
    p.timeline
        .set_frame_aspect(Some(FrameAspect {
            shape: Shape::Tall9x16,
            fit: Fit::Width,
            fill: Fill::Blur,
            blur: Default::default(),
        }))
        .unwrap();
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
    assert_eq!((kept.width, kept.height), (202, 360));
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
    // pixel for pixel, within a small tolerance (RGBA, 4 bytes each)
    let mut far = 0usize;
    let mut sum = 0u64;
    for (a, b) in kept.rgba.iter().zip(&out.stdout) {
        let d = u64::from(a.abs_diff(*b));
        sum += d;
        far += usize::from(d > 24);
    }
    let mean = sum as f64 / kept.rgba.len() as f64;
    assert!(mean < 2.0, "mean difference {mean}");
    assert!(far * 100 < kept.rgba.len(), "{far} bytes differ a lot");
    // the picture was drawn twice, the background once
    assert_eq!((pv.background_renders(), pv.still_renders()), (1, 2));
}

/// Where the video and the audio packets of a file end, in seconds (the last packet's time
/// plus its length: Matroska files have no duration on their streams).
fn stream_durations(env: &Env, file: &Path) -> (Option<f64>, Option<f64>) {
    let end = |kind: &str| {
        let out = Command::new(&env.info.ffprobe)
            .args(["-v", "error", "-select_streams", kind])
            .args([
                "-show_entries",
                "packet=pts_time,duration_time",
                "-of",
                "csv=p=0",
            ])
            .arg(file)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let (t, d) = l.split_once(',')?;
                Some(t.trim().parse::<f64>().ok()? + d.trim().parse::<f64>().unwrap_or(0.0))
            })
            .reduce(f64::max)
    };
    (end("v"), end("a"))
}

/// A large video's preview copy keeps its sound, in step with the picture (REQ-256).
// covers: REQ-256
#[test]
fn the_preview_copy_of_a_large_video_keeps_its_audio_in_step() {
    let Some(env) = setup() else { return };
    if !env.info.encoders.contains("libx264") {
        assert!(!required());
        return;
    }
    let mut p = Project::default();
    let big = env.clip("big-audio.mp4", 2560, 1440, 25, 1, true);
    let m = env.probe(&mut p, &big);
    let (sv, sa) = stream_durations(&env, &big);
    assert!(sv.is_some() && sa.is_some(), "the source has sound");
    let cache = videolore::proxy::ProxyCache::new(env.dir.path().join("proxies-audio"));
    let worker = videolore::proxy::ProxyWorker {
        runner: limited(Priority::Background),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        cache,
        threads: Some(2),
        work_root: Some(env.dir.path().join("work")),
    };
    let task = videolore::proxy::ProxyTask {
        media: m.id,
        source: m.path.clone(),
        key: videolore::proxy::key_of(&m.path).unwrap(),
        format: m.format.clone(),
        duration: m.duration,
        cancel: Arc::new(AtomicBool::new(false)),
        hwaccel: Vec::new(),
    };
    let (tx, _rx) = channel();
    let videolore::proxy::ProxyEvent::Ready(_, path) = worker.generate(&task, &tx).unwrap() else {
        panic!("the copy failed");
    };
    let (cv, ca) = stream_durations(&env, &path);
    let (cv, ca) = (
        cv.expect("a picture"),
        ca.expect("the copy keeps the sound"),
    );
    assert!((cv - ca).abs() <= 0.1, "picture {cv} s, sound {ca} s");
    assert!(
        (ca - sa.unwrap()).abs() <= 0.1,
        "the sound is as long as the source's"
    );
}

/// The roll of the camera reaches the real `v360` by a runtime command (REQ-270): the
/// picture before the command and the one after it differ, and the ones before the
/// command are those of the plain view.
// covers: REQ-270
#[test]
fn a_roll_command_turns_the_picture_in_the_real_filter() {
    use videolore::camera::{self, Camera, Pov};
    let Some(env) = setup() else { return };
    if !env.info.filters.contains("v360") {
        assert!(!required());
        return;
    }
    let path = [
        Pov::new(Micros(0), Camera::FRONT),
        Pov::new(Micros(1_000_000), Camera::FRONT.with_roll(3000)),
    ];
    let fps = videolore::time::Fps::new(10, 1).unwrap();
    let mut cmds = camera::command_lines(
        "v360@c0",
        &path,
        Micros(0),
        Micros(1_000_000),
        fps,
        320,
        180,
    );
    // the end of the move: the roll stays turned
    cmds.push_str("0.95-1.05 v360@c0 roll 30.00;\n");
    let file = env.dir.path().join("roll-cmd.txt");
    std::fs::write(&file, cmds).unwrap();
    let vf = format!(
        "fps=10,sendcmd=f={},{}",
        file.display(),
        camera::filter("c0", Camera::FRONT, 320, 180)
    );
    let out = Command::new(&env.info.ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=1280x640:r=10:d=1.5",
            "-vf",
            &vf,
        ])
        .args(["-pix_fmt", "gray", "-f", "rawvideo", "-"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let frame = 320 * 180;
    let frames: Vec<&[u8]> = out
        .stdout
        .chunks(frame)
        .filter(|c| c.len() == frame)
        .collect();
    assert!(frames.len() >= 14, "{} frames", frames.len());
    assert_ne!(frames[0], frames[12], "the roll turned the picture");
    // the pictures after the move are all the turned one
    assert_eq!(frames[11], frames[13]);
}

/// MKV and WebM files are marked as 360 by the in-house writer: ffprobe reports an
/// equirectangular projection, the file keeps its size and plays through (REQ-261).
// covers: REQ-261
#[test]
fn matroska_and_webm_files_get_the_equirectangular_marker_in_place() {
    let Some(env) = setup() else { return };
    for (name, codec, muxer) in [
        ("m.mkv", "libx264", "matroska"),
        ("m.webm", "libvpx-vp9", "webm"),
    ] {
        if !env.info.encoders.contains(codec) {
            assert!(!required());
            continue;
        }
        let file = env.dir.path().join(name);
        let st = Command::new(&env.info.ffmpeg)
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=320x160:r=10:d=1",
            ])
            .args(["-c:v", codec, "-f", muxer])
            .arg(&file)
            .status()
            .unwrap();
        assert!(st.success());
        let size = std::fs::metadata(&file).unwrap().len();
        let probe = |f: &Path| {
            let mut p = Project::default();
            env.probe(&mut p, f).format.spherical
        };
        assert_eq!(probe(&file), Some(false));
        assert!(
            videolore::spherical::mark_matroska(&file).unwrap(),
            "{name}"
        );
        assert_eq!(std::fs::metadata(&file).unwrap().len(), size, "same size");
        assert_eq!(probe(&file), Some(true), "{name} is equirectangular");
        // marked already: nothing more is written
        assert!(!videolore::spherical::mark_matroska(&file).unwrap());
        // the file still decodes completely and has its frames
        let out = Command::new(&env.info.ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(&file)
            .args(["-f", "null", "-"])
            .output()
            .unwrap();
        assert!(
            out.status.success() && out.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // seeking works (the SeekHead and cues are intact)
        let seek = Command::new(&env.info.ffmpeg)
            .args(["-v", "error", "-ss", "0.5", "-i"])
            .arg(&file)
            .args(["-frames:v", "1", "-f", "null", "-"])
            .output()
            .unwrap();
        assert!(seek.status.success() && seek.stderr.is_empty());
    }
}

/// The initial view of a source is written again in the exported file, in MP4 and in
/// Matroska, and read back by ffprobe (REQ-262, REQ-269).
// covers: REQ-262
// covers: REQ-269
#[test]
fn the_initial_view_of_a_360_file_is_kept_in_mp4_and_matroska() {
    let Some(env) = setup() else { return };
    for (name, codec, muxer) in [
        ("p.mp4", "libx264", "mp4"),
        ("p.mkv", "libx264", "matroska"),
    ] {
        if !env.info.encoders.contains(codec) {
            assert!(!required());
            continue;
        }
        let file = env.dir.path().join(name);
        let st = Command::new(&env.info.ffmpeg)
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=320x160:r=10:d=1",
            ])
            .args(["-c:v", codec, "-f", muxer])
            .arg(&file)
            .status()
            .unwrap();
        assert!(st.success());
        let pose = Some([9000, -500, 0]);
        let marked = if muxer == "mp4" {
            videolore::spherical::mark_equirectangular_with(&file, pose).unwrap()
        } else {
            videolore::spherical::mark_matroska_with(&file, pose).unwrap()
        };
        assert!(marked, "{name}");
        let mut p = Project::default();
        let m = env.probe(&mut p, &file);
        assert_eq!(m.format.spherical, Some(true), "{name}");
        let raw = Command::new(&env.info.ffprobe)
            .args(["-v", "error", "-show_streams", "-of", "json"])
            .arg(&file)
            .output()
            .unwrap();
        let got = m.format.pose.unwrap_or_else(|| {
            panic!(
                "{name}: no view: {}",
                String::from_utf8_lossy(&raw.stdout)
                    .split("side_data_list")
                    .nth(1)
                    .unwrap_or("none")
                    .chars()
                    .take(400)
                    .collect::<String>()
            )
        });
        assert!(
            (got[0] - 9000).abs() <= 100 && (got[1] + 500).abs() <= 100,
            "{name}: {got:?}"
        );
        assert!((videolore::camera::start_camera(m.format.pose).yaw_cd - 9000).abs() <= 100);
    }
}

/// The mix of the clean sound and the original sound is a valid graph in the real ffmpeg and
/// its level is between the two (REQ-278).
// covers: REQ-278
#[test]
fn the_clean_audio_mix_runs_and_lies_between_the_original_and_the_clean_sound() {
    let Some(env) = setup() else { return };
    let rms = |graph: &str| -> f64 {
        let m = Command::new(&env.info.ffmpeg)
            .args([
                "-v",
                "info",
                "-f",
                "lavfi",
                "-i",
                "anoisesrc=d=2:c=pink:a=0.3:r=48000",
            ])
            .args(["-af", &format!("{graph},volumedetect"), "-f", "null", "-"])
            .output()
            .unwrap();
        assert!(m.status.success(), "{}", String::from_utf8_lossy(&m.stderr));
        let text = String::from_utf8_lossy(&m.stderr).into_owned();
        let at = text.find("mean_volume:").expect("mean volume") + "mean_volume:".len();
        text[at..]
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    let clean = videolore::render::CLEAN_AUDIO;
    let original = rms("anull");
    let full = rms(clean);
    let mixed = rms(&format!(
        "asplit=2[d][w];[w]{clean}[c];[d][c]amix=inputs=2:weights='0.5000 0.5000':normalize=0:duration=first"
    ));
    // the two sounds are not in phase (the filter delays its sound), so the mix of equal
    // parts is up to 3 dB under the average of the two levels
    let (lo, hi) = (original.min(full) - 6.0, original.max(full) + 0.5);
    assert!(
        mixed >= lo && mixed <= hi,
        "{mixed} dB is not near {original} and {full}"
    );
}

/// A 10 % crop of the left side makes the picture 10 % narrower in the real ffmpeg, in even
/// pixels (REQ-283).
// covers: REQ-283
#[test]
fn a_ten_percent_left_crop_removes_a_tenth_of_the_width() {
    let Some(env) = setup() else { return };
    let out = Command::new(&env.info.ffmpeg)
        .args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=s=642x360:r=10:d=0.2"])
        .args([
            "-vf",
            "crop=w='trunc(iw*90/200)*2':h='trunc(ih*100/200)*2':x='trunc(iw*10/200)*2':y='trunc(ih*0/200)*2'",
        ])
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // 642 * 0.9 = 577.8: the width is the even number at or under it, the height is whole
    assert_eq!(out.stdout.len(), 576 * 360);
}

/// The bundled ffmpeg has `speechnorm` and it raises quiet speech-like sound (REQ-279, CI).
// covers: REQ-279
#[test]
fn the_bundled_ffmpeg_has_speechnorm_and_it_evens_out_the_level() {
    let Some(env) = setup() else { return };
    assert!(
        env.info.filters.contains("speechnorm") || !required(),
        "speechnorm is missing"
    );
    if !env.info.filters.contains("speechnorm") {
        return;
    }
    let level = |graph: &str| -> f64 {
        let m = Command::new(&env.info.ffmpeg)
            .args([
                "-v",
                "info",
                "-f",
                "lavfi",
                "-i",
                "sine=f=300:d=2,volume=0.02",
            ])
            .args(["-af", &format!("{graph}volumedetect"), "-f", "null", "-"])
            .output()
            .unwrap();
        assert!(m.status.success(), "{}", String::from_utf8_lossy(&m.stderr));
        let t = String::from_utf8_lossy(&m.stderr).into_owned();
        let at = t.find("max_volume:").unwrap() + "max_volume:".len();
        t[at..].split_whitespace().next().unwrap().parse().unwrap()
    };
    let quiet = level("");
    let raised = level(&format!("{},", videolore::render::NORMALISE_SPEECH));
    assert!(raised > quiet + 6.0, "{raised} dB vs {quiet} dB");
}

/// The volume-key expression runs in the real ffmpeg: 0 % at the start and 100 % at 4 s give
/// a gain of 0.25 at 2 s (REQ-280).
// covers: REQ-280
#[test]
fn a_volume_ramp_from_zero_to_full_has_a_quarter_of_the_gain_at_the_middle() {
    let Some(env) = setup() else { return };
    let expr = "pow(if(lt(t,4),(0+(100)*max(0,t-0)/4),100)/100,2)";
    let level = |from: f64| -> f64 {
        let m = Command::new(&env.info.ffmpeg)
            .args([
                "-v",
                "info",
                "-f",
                "lavfi",
                "-i",
                "sine=f=300:d=4,volume=0.5",
            ])
            .args([
                "-af",
                &format!("volume='{expr}':eval=frame,atrim=start={from}:duration=0.2,volumedetect"),
            ])
            .args(["-f", "null", "-"])
            .output()
            .unwrap();
        assert!(m.status.success(), "{}", String::from_utf8_lossy(&m.stderr));
        let t = String::from_utf8_lossy(&m.stderr).into_owned();
        let at = t.find("max_volume:").unwrap() + "max_volume:".len();
        t[at..].split_whitespace().next().unwrap().parse().unwrap()
    };
    // the level of the source (sine at half of its default amplitude), then with the ramp:
    // at 2 s the gain is 0.25 (12 dB less), at 3.8 s 0.9 (about 1 dB less)
    let base = Command::new(&env.info.ffmpeg)
        .args([
            "-v",
            "info",
            "-f",
            "lavfi",
            "-i",
            "sine=f=300:d=4,volume=0.5",
        ])
        .args([
            "-af",
            "atrim=start=1.9:duration=0.2,volumedetect",
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&base.stderr).into_owned();
    let at = text.find("max_volume:").unwrap() + "max_volume:".len();
    let base: f64 = text[at..]
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let (mid, end) = (level(1.9), level(3.8));
    assert!((mid - (base - 12.0)).abs() < 2.0, "{mid} vs {base}");
    assert!((end - (base - 1.0)).abs() < 2.0, "{end} vs {base}");
}
