// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Spatial sound (REQ-273): the first-order ambisonic sound of a 360 clip (ACN channel order,
//! SN3D scaling: W, Y, Z, X) follows the camera. videolore decodes the four channels with
//! ffmpeg, turns the sound field by the camera's yaw and pitch in Rust, reduces it to stereo
//! with a fixed pair of cardioid virtual microphones (left and right, 90 degrees to each
//! side) and writes a 32-bit float stereo WAV in the work folder. The filter graph then reads
//! that file as one more input instead of the clip's own sound ([`SpatialJob::wav_name`]).
//! The same pre-pass serves the preview and the flat export; a 360 video export keeps the
//! sound as recorded.
//!
//! The camera path is evaluated in source time ([`crate::camera::at`]), so the rotation
//! follows the same moves as the picture. The angles are interpolated across each block of
//! [`BLOCK`] frames, so a move makes no clicks. Roll is not applied (a roll is a rare,
//! small move). The pair cannot tell front from back (both give half of W). Pitch turns the field about the left-right axis: that mixes X and Z and
//! leaves W and Y alone, so it does not change a horizontal pair of microphones; it is part
//! of [`rotate`] for completeness and is tested there.

use crate::camera::{self, Pov};
use crate::ffmpeg::{Runner, argv, file_arg};
use crate::time::Micros;
use sha2::{Digest, Sha256};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Sample rate of the pre-pass and of its file.
pub const RATE: u32 = 48_000;
/// Channels of first-order ambisonics.
pub const CHANNELS: usize = 4;
/// Frames between two camera readings (10 ms).
const BLOCK: usize = 480;
/// Longest clip the preview prepares (source time): a longer one keeps its sound as recorded,
/// because the preview waits for the pre-pass.
pub const PREVIEW_MAX: Micros = Micros(600_000_000);

/// What to prepare for one clip: its sound from `src_in` for `len`, turned by the camera
/// `path` (source time).
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialJob {
    /// The clip's own file (never a preview copy, which may have lost the channels).
    pub source: PathBuf,
    pub src_in: Micros,
    pub len: Micros,
    pub path: Vec<Pov>,
}

impl SpatialJob {
    /// The file name of the result in the work folder: it changes with everything that
    /// changes the sound, so a file found there is the right one.
    pub fn wav_name(&self) -> String {
        let mut h = Sha256::new();
        h.update(format!("spatial-v1:{self:?}"));
        let hex: String = h
            .finalize()
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("spatial-{hex}.wav")
    }
}

/// True for the media whose sound is read as first-order ambisonics: a 360 picture with four
/// audio channels (assumption: no other four-channel layout is common on a 360 clip).
pub fn is_ambisonic(format: &crate::media::MediaFormat) -> bool {
    format.is_360() && format.has_audio() && format.channels == Some(CHANNELS as u32)
}

/// A frame of the sound field `[W, Y, Z, X]` turned for a camera looking at `yaw` (to the
/// right) and `pitch` (up), in radians: the sound that was in the camera's direction is
/// now in front.
pub fn rotate(f: [f32; 4], yaw: f32, pitch: f32) -> [f32; 4] {
    let [w, y, z, x] = f;
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    // about the vertical axis: a source at azimuth a (to the left) moves to a + yaw
    let x1 = x * cy - y * sy;
    let y1 = y * cy + x * sy;
    // about the left-right axis: a source at elevation e moves to e - pitch
    let x2 = x1 * cp + z * sp;
    let z2 = z * cp - x1 * sp;
    [w, y1, z2, x2]
}

/// Left and right of the fixed cardioid pair (virtual microphones at 90 degrees to each
/// side): `0.5 * (W +- Y)`.
pub fn cardioid_pair(f: [f32; 4]) -> (f32, f32) {
    (0.5 * (f[0] + f[1]), 0.5 * (f[0] - f[1]))
}

/// Turns the sound by the camera path as it plays: feed it consecutive blocks.
pub struct Rotator<'a> {
    path: &'a [Pov],
    origin: Micros,
    frames: u64,
}

impl<'a> Rotator<'a> {
    /// `origin` is the source time of the first frame.
    pub fn new(path: &'a [Pov], origin: Micros) -> Rotator<'a> {
        Rotator {
            path,
            origin,
            frames: 0,
        }
    }

    fn angles(&self, frame: u64) -> (f32, f32) {
        let t = self.origin + Micros((frame * 1_000_000 / u64::from(RATE)) as i64);
        let c = camera::at(self.path, t).unwrap_or(camera::Camera::FRONT);
        (
            c.yaw_deg().to_radians() as f32,
            c.pitch_deg().to_radians() as f32,
        )
    }

    /// `input` holds whole frames of four channels (W, Y, Z, X); the stereo result is
    /// appended to `out`. The camera is read at the start and the end of each block and the
    /// angles are interpolated between them (the shorter way round for the yaw).
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        for block in input.chunks(BLOCK * CHANNELS) {
            let n = block.len() / CHANNELS;
            let (y0, p0) = self.angles(self.frames);
            let (y1, p1) = self.angles(self.frames + n as u64);
            let dy = (y1 - y0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            for (i, f) in block.as_chunks::<CHANNELS>().0.iter().enumerate() {
                let k = i as f32 / n.max(1) as f32;
                let r = rotate([f[0], f[1], f[2], f[3]], y0 + dy * k, p0 + (p1 - p0) * k);
                let (l, rr) = cardioid_pair(r);
                out.push(l);
                out.push(rr);
            }
            self.frames += n as u64;
        }
    }
}

/// The header of a 32-bit float stereo WAV with `frames` frames.
pub fn wav_header(frames: u64) -> [u8; 44] {
    let data = (frames * 8).min(u64::from(u32::MAX - 36)) as u32;
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36 + data).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&3u16.to_le_bytes()); // IEEE float
    h[22..24].copy_from_slice(&2u16.to_le_bytes());
    h[24..28].copy_from_slice(&RATE.to_le_bytes());
    h[28..32].copy_from_slice(&(RATE * 8).to_le_bytes());
    h[32..34].copy_from_slice(&8u16.to_le_bytes());
    h[34..36].copy_from_slice(&32u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data.to_le_bytes());
    h
}

/// The ffmpeg arguments that decode the first audio stream of the job's clip as raw 32-bit
/// float, four channels, [`RATE`] Hz, on stdout.
pub fn decode_args(job: &SpatialJob) -> Vec<std::ffi::OsString> {
    let mut a = argv([
        "-hide_banner",
        "-nostdin",
        "-loglevel",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-ss",
    ]);
    a.extend(argv([
        job.src_in.ff(),
        "-t".to_string(),
        job.len.ff(),
        "-i".to_string(),
    ]));
    a.push(file_arg(&job.source));
    a.extend(argv([
        "-map",
        "0:a:0",
        "-vn",
        "-ac",
        "4",
        "-ar",
        &RATE.to_string(),
        "-f",
        "f32le",
        "pipe:1",
    ]));
    a
}

/// Writes the rotated sound of `job` to `dir/<wav_name>` unless it is already there.
/// Returns the path. Cancelling stops the work and leaves nothing behind.
pub fn prepare(
    runner: &dyn Runner,
    ffmpeg: &Path,
    dir: &Path,
    job: &SpatialJob,
    cancel: &AtomicBool,
) -> io::Result<PathBuf> {
    let target = dir.join(job.wav_name());
    if target.is_file() {
        return Ok(target);
    }
    let part = dir.join(format!("{}.part", job.wav_name()));
    let result = write_wav(runner, ffmpeg, dir, job, &part, cancel);
    match result {
        Ok(()) => {
            std::fs::rename(&part, &target)?;
            Ok(target)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

fn write_wav(
    runner: &dyn Runner,
    ffmpeg: &Path,
    dir: &Path,
    job: &SpatialJob,
    part: &Path,
    cancel: &AtomicBool,
) -> io::Result<()> {
    let mut child = runner.spawn_in(ffmpeg, &decode_args(job), Some(dir))?;
    let mut file = crate::base::private::create_private_file(part)?;
    file.write_all(&wav_header(0))?;
    let mut stdout = child
        .take_stdout()
        .ok_or_else(|| io::Error::other("no output from ffmpeg"))?;
    let mut rot = Rotator::new(&job.path, job.src_in);
    let mut buf = vec![0u8; 64 * 1024];
    let mut held = 0usize; // bytes of an unfinished frame at the start of `buf`
    let (mut samples, mut out) = (Vec::new(), Vec::new());
    let mut frames = 0u64;
    loop {
        if cancel.load(Ordering::SeqCst) {
            child.kill();
            let _ = child.wait();
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let n = stdout.read(&mut buf[held..])?;
        if n == 0 {
            break;
        }
        let have = held + n;
        let whole = have / (4 * CHANNELS) * (4 * CHANNELS);
        samples.clear();
        samples.extend(
            buf[..whole]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b)),
        );
        out.clear();
        rot.process(&samples, &mut out);
        let mut bytes = Vec::with_capacity(out.len() * 4);
        for v in &out {
            bytes.extend_from_slice(&v.clamp(-1.0, 1.0).to_le_bytes());
        }
        file.write_all(&bytes)?;
        frames += (whole / (4 * CHANNELS)) as u64;
        buf.copy_within(whole..have, 0);
        held = have - whole;
    }
    let status = child.wait()?;
    if !status.success {
        return Err(io::Error::other(status.stderr.trim().to_string()));
    }
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&wav_header(frames))?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::ffmpeg::CmdOutput;
    use crate::ffmpeg::fake::FakeRunner;
    use std::sync::Arc;

    const LEFT: [f32; 4] = [1.0, 1.0, 0.0, 0.0]; // a plane wave from the left: W = 1, Y = 1
    const FRONT: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

    fn db(a: f32, b: f32) -> f32 {
        20.0 * (a.abs().max(1e-9) / b.abs().max(1e-9)).log10()
    }

    // covers: REQ-273
    #[test]
    fn a_sound_on_the_left_is_in_front_when_the_camera_turns_90_degrees_left() {
        // facing front the sound is only in the left ear
        let (l, r) = cardioid_pair(rotate(LEFT, 0.0, 0.0));
        assert!((l - 1.0).abs() < 1e-6 && r.abs() < 1e-6);
        // the camera turned 90 degrees to the left (yaw -90): the sound is in front, both
        // channels within 3 dB of each other
        let (l, r) = cardioid_pair(rotate(LEFT, -90f32.to_radians(), 0.0));
        assert!(db(l, r).abs() < 3.0, "{l} {r}");
        assert!((l - 0.5).abs() < 1e-5 && (r - 0.5).abs() < 1e-5);
        // the camera looking back: the sound is on the other side, in the right ear only
        let (l, r) = cardioid_pair(rotate(LEFT, 180f32.to_radians(), 0.0));
        assert!(r > 0.99 && l.abs() < 1e-5, "{l} {r}");
        // the pair does not tell front from back: a front sound and a back sound are alike
        let front = cardioid_pair(rotate(FRONT, 0.0, 0.0));
        let back = cardioid_pair(rotate(FRONT, 180f32.to_radians(), 0.0));
        assert!((front.0 - back.0).abs() < 1e-5 && (front.1 - back.1).abs() < 1e-5);
        // a front sound goes to the left ear when the camera turns right (yaw +90)
        let (l, r) = cardioid_pair(rotate(FRONT, 90f32.to_radians(), 0.0));
        assert!(l > 0.99 && r.abs() < 1e-5, "{l} {r}");
    }

    // covers: REQ-273
    #[test]
    fn pitch_moves_a_front_sound_below_the_camera_and_leaves_the_pair_alone() {
        // the camera looks up 90 degrees: the front sound is now below (Z = -1)
        let r = rotate(FRONT, 0.0, 90f32.to_radians());
        assert!(r[2] < -0.99 && r[3].abs() < 1e-5, "{r:?}");
        // a horizontal pair does not hear the difference
        let a = cardioid_pair(rotate(LEFT, 0.0, 0.0));
        let b = cardioid_pair(rotate(LEFT, 0.0, 70f32.to_radians()));
        assert!((a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6);
    }

    // covers: REQ-273
    #[test]
    fn the_rotation_follows_the_camera_path_without_clicks() {
        // the camera turns from front to 90 degrees left over 1 s: a steady left sound
        // slides from the left ear to the middle
        let path = [
            Pov::new(Micros::ZERO, Camera::new(0, 0, 9000)),
            Pov::new(Micros(1_000_000), Camera::new(-9000, 0, 9000)),
        ];
        let frames = RATE as usize;
        let input: Vec<f32> = (0..frames).flat_map(|_| LEFT).collect();
        let mut out = Vec::new();
        Rotator::new(&path, Micros::ZERO).process(&input, &mut out);
        assert_eq!(out.len(), frames * 2);
        let (l0, r0) = (out[0], out[1]);
        let (l1, r1) = (out[out.len() - 2], out[out.len() - 1]);
        assert!(l0 > 0.99 && r0 < 0.01, "{l0} {r0}");
        assert!(
            (l1 - 0.5).abs() < 0.01 && (r1 - 0.5).abs() < 0.01,
            "{l1} {r1}"
        );
        // the left channel falls steadily: no step larger than a block's share of the move
        let worst = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| c[0])
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 0.001, "{worst}");
        // a path that starts later is read in source time: origin 10 s is past the last POV
        let mut late = Vec::new();
        Rotator::new(&path, Micros(10_000_000)).process(&input[..8], &mut late);
        assert!((late[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn the_yaw_is_interpolated_the_short_way_across_the_back() {
        let path = [
            Pov::new(Micros::ZERO, Camera::new(17_900, 0, 9000)),
            Pov::new(Micros(1_000_000), Camera::new(-17_900, 0, 9000)),
        ];
        let input: Vec<f32> = (0..BLOCK).flat_map(|_| LEFT).collect();
        let mut out = Vec::new();
        Rotator::new(&path, Micros(500_000)).process(&input, &mut out);
        // half way the camera looks back (the short way round, not through the front): a
        // sound on the left is in the right ear
        assert!(out[1] > 0.99 && out[0] < 0.01, "{:?}", &out[..2]);
    }

    #[test]
    fn the_header_describes_a_float_stereo_file() {
        let h = wav_header(100);
        assert_eq!(&h[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(h[4..8].try_into().unwrap()), 36 + 800);
        assert_eq!(u16::from_le_bytes(h[20..22].try_into().unwrap()), 3);
        assert_eq!(u16::from_le_bytes(h[22..24].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(h[24..28].try_into().unwrap()), RATE);
        assert_eq!(u32::from_le_bytes(h[40..44].try_into().unwrap()), 800);
    }

    fn job(path: Vec<Pov>) -> SpatialJob {
        SpatialJob {
            source: "/m/a.mp4".into(),
            src_in: Micros(2_000_000),
            len: Micros(500_000),
            path,
        }
    }

    #[test]
    fn the_file_name_changes_with_everything_that_changes_the_sound() {
        let a = job(vec![]);
        assert_eq!(a.wav_name(), job(vec![]).wav_name());
        let mut b = job(vec![]);
        b.src_in = Micros(2_000_001);
        assert_ne!(a.wav_name(), b.wav_name());
        let c = job(vec![Pov::new(Micros::ZERO, Camera::new(100, 0, 9000))]);
        assert_ne!(a.wav_name(), c.wav_name());
        assert!(a.wav_name().starts_with("spatial-") && a.wav_name().ends_with(".wav"));
        let args: Vec<String> = decode_args(&a)
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.windows(2).any(|w| w == ["-ss", "2.000000"]),
            "{args:?}"
        );
        assert!(args.windows(2).any(|w| w == ["-ac", "4"]));
        assert!(args.last().is_some_and(|l| l == "pipe:1"));
    }

    fn pcm(frames: &[[f32; 4]]) -> Vec<u8> {
        frames
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    // covers: REQ-273
    #[test]
    fn the_pre_pass_writes_a_stereo_float_file_once_and_reuses_it() {
        let dir = tempfile::tempdir().unwrap();
        let stdout = pcm(&[LEFT; 1000]);
        let runner = Arc::new(FakeRunner::default().on(move |_, _| {
            Some(CmdOutput {
                success: true,
                stdout: stdout.clone(),
                stderr: String::new(),
            })
        }));
        let cancel = AtomicBool::new(false);
        let j = job(vec![Pov::new(Micros::ZERO, Camera::new(-9000, 0, 9000))]);
        let path = prepare(&*runner, Path::new("ffmpeg"), dir.path(), &j, &cancel).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 44 + 1000 * 8);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8000);
        let l = f32::from_le_bytes(bytes[44..48].try_into().unwrap());
        let r = f32::from_le_bytes(bytes[48..52].try_into().unwrap());
        assert!((l - 0.5).abs() < 1e-5 && (r - 0.5).abs() < 1e-5, "{l} {r}");
        assert_eq!(path.file_name().unwrap().to_string_lossy(), j.wav_name());
        // the second call finds the file and does not start ffmpeg again
        let again = prepare(&*runner, Path::new("ffmpeg"), dir.path(), &j, &cancel).unwrap();
        assert_eq!(again, path);
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
        assert_eq!(runner.cwds.lock().unwrap()[0].as_deref(), Some(dir.path()));
    }

    #[test]
    fn a_failing_or_cancelled_pre_pass_leaves_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let j = job(vec![]);
        let cancel = AtomicBool::new(false);
        let bad = FakeRunner::default().on(|_, _| crate::ffmpeg::fake::fail("no audio"));
        let e = prepare(&bad, Path::new("ffmpeg"), dir.path(), &j, &cancel).unwrap_err();
        assert!(e.to_string().contains("no audio"), "{e}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        let ok = FakeRunner::default().on(|_, _| {
            Some(CmdOutput {
                success: true,
                stdout: vec![0; 64],
                stderr: String::new(),
            })
        });
        cancel.store(true, Ordering::SeqCst);
        let e = prepare(&ok, Path::new("ffmpeg"), dir.path(), &j, &cancel).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::Interrupted);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_partial_frame_at_the_end_of_a_read_is_carried_over() {
        // 3 frames delivered as 16 + 20 + 12 bytes: frames must not be split or lost
        struct Chunks(Vec<Vec<u8>>);
        impl Read for Chunks {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if self.0.is_empty() {
                    return Ok(0);
                }
                let c = self.0.remove(0);
                buf[..c.len()].copy_from_slice(&c);
                Ok(c.len())
            }
        }
        struct P(Option<Chunks>);
        impl crate::ffmpeg::Process for P {
            fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
                self.0.take().map(|c| Box::new(c) as Box<dyn Read + Send>)
            }
            fn kill(&mut self) {}
            fn wait(&mut self) -> io::Result<CmdOutput> {
                Ok(CmdOutput {
                    success: true,
                    ..Default::default()
                })
            }
        }
        struct R(Vec<u8>);
        impl Runner for R {
            fn run(&self, _: &Path, _: &[std::ffi::OsString]) -> io::Result<CmdOutput> {
                unreachable!()
            }
            fn spawn_in(
                &self,
                _: &Path,
                _: &[std::ffi::OsString],
                _: Option<&Path>,
            ) -> io::Result<Box<dyn crate::ffmpeg::Process>> {
                let all = &self.0;
                Ok(Box::new(P(Some(Chunks(vec![
                    all[..16].to_vec(),
                    all[16..36].to_vec(),
                    all[36..48].to_vec(),
                ])))))
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let all = pcm(&[LEFT, LEFT, LEFT]);
        let path = prepare(
            &R(all),
            Path::new("ffmpeg"),
            dir.path(),
            &job(vec![]),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(std::fs::metadata(path).unwrap().len(), 44 + 3 * 8);
    }
}
