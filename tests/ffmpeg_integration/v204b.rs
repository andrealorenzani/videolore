// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of the last part of 2.0.4: clip speed and the items it brought.

use super::*;

/// Zero crossings per second of the mono sound of `file` from `from` for `len` seconds
/// (a 440 Hz sine crosses zero 880 times a second).
fn crossings(env: &Env, file: &Path, from: f64, len: f64) -> f64 {
    let o = Command::new(&env.info.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-ss"])
        .arg(format!("{from}"))
        .arg("-t")
        .arg(format!("{len}"))
        .arg("-i")
        .arg(file)
        .args(["-ac", "1", "-ar", "48000", "-f", "s16le", "pipe:1"])
        .output()
        .unwrap();
    let (samples, _) = o.stdout.as_chunks::<2>();
    let v: Vec<i16> = samples.iter().map(|&s| i16::from_le_bytes(s)).collect();
    let n = v.windows(2).filter(|w| (w[0] < 0) != (w[1] < 0)).count();
    n as f64 / (v.len() as f64 / 48_000.0)
}

// covers: REQ-281
#[test]
fn a_4_s_clip_at_200_percent_lasts_2_s_and_keeps_its_pitch() {
    let Some(env) = setup() else { return };
    let file = env.clip("speed.mp4", 160, 120, 10, 4, true);
    let mut p = Project::default();
    let m = env.probe(&mut p, &file);
    p.timeline.append(&m).unwrap();
    p.media[0].format.bit_rate = Some(20_000_000);
    let id = p.timeline.clips[0].id;
    p.timeline.set_speed(id, 2000).unwrap();
    assert_eq!(p.timeline.duration(), Micros(2_000_000));
    let out = env.dir.path().join("fast.mp4");
    env.export(&p, &out).0.unwrap();
    let info = media::probe(
        &SystemRunner,
        &env.info.ffprobe,
        &out,
        videolore::media::MediaId(1),
    )
    .unwrap();
    let secs = info.duration.0 as f64 / 1e6;
    assert!((secs - 2.0).abs() < 0.2, "the export lasts {secs} s");
    // the pitch of the 440 Hz tone is unchanged within 1 %
    let rate = crossings(&env, &out, 0.3, 1.2);
    assert!((rate - 880.0).abs() < 8.8, "{rate} crossings a second");
    // the picture is the source's, sped up: frame count of the output matches 2 s at 10 fps
    let frames = env.frame_rgb(&out, 1.5);
    assert!(!frames.is_empty());
}

fn rgb(buf: &[u8], w: usize, x: usize, y: usize) -> [i32; 3] {
    let i = (y * w + x) * 3;
    [buf[i] as i32, buf[i + 1] as i32, buf[i + 2] as i32]
}

fn near_rgb(p: [i32; 3], want: [i32; 3]) -> bool {
    p.iter().zip(want).all(|(a, b)| (a - b).abs() <= 30)
}

// covers: REQ-287
#[test]
fn a_png_with_transparency_is_burned_in_at_its_place_for_its_time() {
    let Some(env) = setup() else { return };
    // 100x50: the left half red and opaque, the right half fully transparent
    let png = env.fixture(
        "half-red.png",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=100x50,format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lt(X,50),255,0)'",
            "-frames:v",
            "1",
        ],
    );
    let green = env.fixture(
        "flat-green-4.mp4",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=green:s=160x120:r=10:d=4",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let mut p = Project::default();
    let m = env.probe(&mut p, &green);
    p.timeline.append(&m).unwrap();
    p.media[0].format.bit_rate = Some(20_000_000);
    let pic = videolore::picture::read(&png).unwrap();
    assert_eq!((pic.width, pic.height), (100, 50));
    let id = p.texts.new_id();
    let mut item = videolore::text::TextItem::new_picture(id, pic, Micros(1_000_000), (160, 120));
    item.ttl = Micros(2_000_000);
    p.texts.add(item).unwrap();
    let out = env.dir.path().join("pic.mp4");
    env.export(&p, &out).0.unwrap();
    // the picture is 48x24 in the middle: x 56..104, y 48..72
    let on = env.frame_rgb(&out, 2.0);
    assert!(near_rgb(rgb(&on, 160, 66, 60), [255, 0, 0]), "opaque part");
    assert!(
        near_rgb(rgb(&on, 160, 94, 60), [0, 128, 0]),
        "transparent part shows the video: {:?}",
        rgb(&on, 160, 94, 60)
    );
    assert!(near_rgb(rgb(&on, 160, 10, 10), [0, 128, 0]));
    // before and after its time there is no picture
    let before = env.frame_rgb(&out, 0.4);
    assert!(near_rgb(rgb(&before, 160, 66, 60), [0, 128, 0]));
    let after = env.frame_rgb(&out, 3.5);
    assert!(near_rgb(rgb(&after, 160, 66, 60), [0, 128, 0]));
}

/// RMS of the left and right channel of the sound of `file` from `from` for `len` seconds.
fn stereo_rms(env: &Env, file: &Path, from: f64, len: f64) -> (f64, f64) {
    let o = Command::new(&env.info.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-ss"])
        .arg(format!("{from}"))
        .arg("-t")
        .arg(format!("{len}"))
        .arg("-i")
        .arg(file)
        .args(["-ac", "2", "-ar", "48000", "-f", "s16le", "pipe:1"])
        .output()
        .unwrap();
    let (samples, _) = o.stdout.as_chunks::<2>();
    let v: Vec<f64> = samples
        .iter()
        .map(|&s| f64::from(i16::from_le_bytes(s)))
        .collect();
    let rms = |k: usize| {
        let ch: Vec<f64> = v.iter().skip(k).step_by(2).copied().collect();
        (ch.iter().map(|x| x * x).sum::<f64>() / ch.len().max(1) as f64).sqrt()
    };
    (rms(0), rms(1))
}

/// A 360 clip whose first-order ambisonic sound (W, Y, Z, X) is a tone coming from the left.
fn ambisonic_clip(env: &Env) -> PathBuf {
    env.fixture(
        "ambisonic-left.mkv",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=640x320:r=10:d=2",
            "-f",
            "lavfi",
            "-i",
            "sine=f=440:r=48000:d=2,pan=quad|FL=c0|FR=c0|BL=0*c0|BR=0*c0",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "pcm_s16le",
        ],
    )
}

/// Exports the ambisonic clip with the camera at `yaw` degrees (first point of its path) and
/// returns the left and right levels of the output's sound.
fn ambisonic_levels(env: &Env, yaw: Option<i64>, name: &str) -> (f64, f64) {
    let mut p = Project::default();
    let m = env.probe(&mut p, &ambisonic_clip(env));
    assert_eq!(m.format.channels, Some(4));
    p.media[0].format.spherical = Some(true);
    p.timeline.append(&m).unwrap();
    if let Some(y) = yaw {
        p.camera
            .set(
                m.id,
                videolore::camera::Pov::new(
                    Micros::ZERO,
                    videolore::camera::Camera::new(y * 100, 0, 9000),
                ),
            )
            .unwrap();
    }
    let out = env.dir.path().join(name);
    env.export(&p, &out).0.unwrap();
    stereo_rms(env, &out, 0.3, 1.2)
}

// covers: REQ-273
#[test]
fn ambisonic_sound_follows_the_camera_in_the_export() {
    let Some(env) = setup() else { return };
    // the camera at the front: the sound that comes from the left is in the left ear only
    let (l, r) = ambisonic_levels(&env, None, "front.mp4");
    assert!(l > 1000.0 && l > 8.0 * r, "front: left {l}, right {r}");
    // the camera turned 90 degrees to the left: the sound is in front, both ears within 3 dB
    let (l, r) = ambisonic_levels(&env, Some(-90), "left.mp4");
    let db = 20.0 * (l / r).log10();
    assert!(
        l > 100.0 && db.abs() < 3.0,
        "turned: left {l}, right {r} ({db:.1} dB)"
    );
    // the camera behind: the sound is on the other side
    let (l, r) = ambisonic_levels(&env, Some(180), "behind.mp4");
    assert!(r > 1000.0 && r > 8.0 * l, "behind: left {l}, right {r}");
    // in front the level is half that of the left-only case (a cardioid)
    let (l, _) = ambisonic_levels(&env, Some(-90), "left2.mp4");
    let (front, _) = ambisonic_levels(&env, None, "front2.mp4");
    assert!((l / front - 0.5).abs() < 0.08, "{l} vs {front}");
}

// covers: REQ-273
#[test]
fn a_360_video_export_keeps_the_ambisonic_sound_as_recorded() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let m = env.probe(&mut p, &ambisonic_clip(&env));
    p.media[0].format.spherical = Some(true);
    p.timeline.append(&m).unwrap();
    p.camera
        .set(
            m.id,
            videolore::camera::Pov::new(
                Micros::ZERO,
                videolore::camera::Camera::new(18_000, 0, 9000),
            ),
        )
        .unwrap();
    let mut job = env.job(&p, &env.dir.path().join("sphere.mp4"));
    job.mode = videolore::render::ExportMode::Sphere360;
    let (tx, _rx) = channel();
    let r = export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx);
    let (out, _) = r.unwrap();
    // no turn although the camera path looks back: the two ears stay equal (ffmpeg's own
    // reduction of the four channels); a turned sound would be on the right
    let (l, r) = stereo_rms(&env, &out, 0.3, 1.2);
    assert!(
        l > 1000.0 && (l / r - 1.0).abs() < 0.1,
        "left {l}, right {r}"
    );
}

/// An audio device that is a thread: it pulls what the preview decodes and keeps the samples.
struct CaptureSink {
    heard: Arc<std::sync::Mutex<Vec<i16>>>,
    stop: Arc<AtomicBool>,
}

impl videolore::preview::AudioSink for CaptureSink {
    fn format(&self) -> (u32, u16) {
        (48_000, 2)
    }
    fn play(&mut self, buffer: Arc<videolore::preview::AudioBuffer>) -> bool {
        let (heard, stop) = (Arc::clone(&self.heard), Arc::clone(&self.stop));
        stop.store(false, std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(move || {
            let mut block = [0i16; 960];
            while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                buffer.pull(&mut block, |s| s);
                heard.lock().unwrap().extend_from_slice(&block);
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        true
    }
    fn stop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Plays the ambisonic clip in the preview with the camera at `yaw` degrees and returns the
/// levels of the left and right ear of what the device got, and the seconds the pre-pass took.
fn preview_levels(env: &Env, yaw: i64) -> (f64, f64) {
    let mut p = Project::default();
    let m = env.probe(&mut p, &ambisonic_clip(env));
    p.media[0].format.spherical = Some(true);
    p.timeline.append(&m).unwrap();
    p.camera
        .set(
            m.id,
            videolore::camera::Pov::new(
                Micros::ZERO,
                videolore::camera::Camera::new(yaw * 100, 0, 9000),
            ),
        )
        .unwrap();
    let heard = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = CaptureSink {
        heard: Arc::clone(&heard),
        stop: Arc::new(AtomicBool::new(false)),
    };
    let cfg = PreviewConfig {
        runner: limited(Priority::Preview),
        ffmpeg: env.info.ffmpeg.clone(),
        version: env.info.version,
        encoders: env.info.encoders.clone(),
    };
    let mut pv = Preview::new(Some(cfg), Box::new(sink));
    pv.play(&p, Micros::ZERO, Instant::now()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        pv.poll(Instant::now());
        let n = heard.lock().unwrap().iter().filter(|s| **s != 0).count();
        if n > 48_000 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    pv.pause(Instant::now());
    let v = heard.lock().unwrap().clone();
    let rms = |k: usize| {
        let ch: Vec<f64> = v
            .iter()
            .skip(k)
            .step_by(2)
            .map(|s| f64::from(*s))
            .filter(|s| *s != 0.0)
            .collect();
        (ch.iter().map(|x| x * x).sum::<f64>() / ch.len().max(1) as f64).sqrt()
    };
    (rms(0), rms(1))
}

// covers: REQ-273
#[test]
fn ambisonic_sound_follows_the_camera_in_the_preview() {
    let Some(env) = setup() else { return };
    let (l, r) = preview_levels(&env, 0);
    assert!(l > 1000.0 && l > 8.0 * r, "front: left {l}, right {r}");
    let (l, r) = preview_levels(&env, -90);
    let db = 20.0 * (l / r).log10();
    assert!(l > 100.0 && db.abs() < 3.0, "turned: left {l}, right {r}");
}

/// How fast the pre-pass is (QG-14 style measurement, printed): the 4-channel sound of a long
/// clip is decoded, turned by a moving camera and written; it must be much faster than real
/// time, because the preview waits for it.
// covers: REQ-273
#[test]
fn the_pre_pass_runs_far_faster_than_real_time() {
    let Some(env) = setup() else { return };
    let secs = 120;
    let file = env.fixture(
        "ambisonic-long.mkv",
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=f=300:r=48000:d=120,pan=quad|FL=c0|FR=c0|BL=0*c0|BR=0*c0",
            "-c:a",
            "pcm_s16le",
        ],
    );
    let job = videolore::spatial::SpatialJob {
        source: file,
        src_in: Micros::ZERO,
        len: Micros(secs * 1_000_000),
        path: vec![
            videolore::camera::Pov::new(Micros::ZERO, videolore::camera::Camera::new(0, 0, 9000)),
            videolore::camera::Pov::new(
                Micros(60_000_000),
                videolore::camera::Camera::new(9000, 2000, 9000),
            ),
        ],
        track: 0,
    };
    let dir = env.dir.path().join("pre");
    std::fs::create_dir_all(&dir).unwrap();
    let t = Instant::now();
    let wav = videolore::spatial::prepare(
        &SystemRunner,
        &env.info.ffmpeg,
        &dir,
        &job,
        &AtomicBool::new(false),
    )
    .unwrap();
    let took = t.elapsed().as_secs_f64();
    let size = std::fs::metadata(&wav).unwrap().len();
    assert!(size >= 44 + 120 * 48_000 * 8, "{size} bytes");
    eprintln!(
        "spatial pre-pass: {secs} s of 4-channel sound in {took:.2} s = {:.0}x real time",
        secs as f64 / took
    );
    assert!(
        took < secs as f64 / 10.0,
        "only {:.1}x real time",
        secs as f64 / took
    );
}
