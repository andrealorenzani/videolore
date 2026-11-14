// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Preview playback: ffmpeg renders the edited timeline (fades included) from the playhead
//! as raw RGBA frames (<= 360p) and s16le PCM on stdout. Frames go through a bounded
//! channel (8 frames); the clock follows the audio actually played (or the wall clock when
//! there is no audio device). A seek kills the workers and starts new ones.

use crate::ffmpeg::{FfVersion, Process, Runner};
use crate::project::Project;
use crate::render::{self, Geometry};
use crate::time::Micros;
use std::collections::VecDeque;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Frames buffered between the decoder and the UI.
pub const FRAME_BUFFER: usize = 8;

/// One decoded preview frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub pts: Micros,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// PCM shared between the decoder thread and the audio device callback.
#[derive(Debug, Default)]
pub struct AudioBuffer {
    samples: Mutex<VecDeque<i16>>,
    /// Frames (samples per channel) handed to the device so far.
    played: AtomicU64,
    /// Decoder reached the end.
    finished: AtomicBool,
}

impl AudioBuffer {
    /// Max samples kept in memory (about one second of stereo 48 kHz).
    pub const CAP: usize = 96_000;

    pub fn push(&self, data: &[i16]) {
        if let Ok(mut q) = self.samples.lock() {
            q.extend(data.iter().copied());
        }
    }

    pub fn len(&self) -> usize {
        self.samples.lock().map(|q| q.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `out` with samples (silence when starved) mapped by `conv`; counts played frames.
    pub fn pull<T>(&self, out: &mut [T], channels: u16, conv: impl Fn(i16) -> T) {
        let mut taken = 0usize;
        if let Ok(mut q) = self.samples.lock() {
            for o in out.iter_mut() {
                match q.pop_front() {
                    Some(s) => {
                        *o = conv(s);
                        taken += 1;
                    }
                    None => *o = conv(0),
                }
            }
        }
        self.played
            .fetch_add((taken / channels.max(1) as usize) as u64, Ordering::SeqCst);
    }

    pub fn played_frames(&self) -> u64 {
        self.played.load(Ordering::SeqCst)
    }
}

/// Where preview audio goes.
pub trait AudioSink {
    /// Device sample rate and channel count (what ffmpeg must produce).
    fn format(&self) -> (u32, u16);
    /// Starts pulling from `buffer`. Returns false when no audio can be played.
    fn play(&mut self, buffer: Arc<AudioBuffer>) -> bool;
    fn stop(&mut self);
}

/// No audio device: the preview is silent and clocked by the wall clock.
#[derive(Debug, Default)]
pub struct NullSink;

impl AudioSink for NullSink {
    fn format(&self) -> (u32, u16) {
        (render::SAMPLE_RATE, 2)
    }
    fn play(&mut self, _buffer: Arc<AudioBuffer>) -> bool {
        false
    }
    fn stop(&mut self) {}
}

#[cfg(feature = "audio")]
mod device {
    //! cpal output (thin glue, exercised manually: CI machines have no sound card).
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SampleFormat, SizedSample};

    pub struct CpalSink {
        device: cpal::Device,
        config: cpal::StreamConfig,
        format: SampleFormat,
        stream: Option<cpal::Stream>,
    }

    impl CpalSink {
        pub fn new() -> Option<CpalSink> {
            let device = cpal::default_host().default_output_device()?;
            let supported = device.default_output_config().ok()?;
            Some(CpalSink {
                format: supported.sample_format(),
                config: supported.config(),
                device,
                stream: None,
            })
        }

        fn build<T: SizedSample + FromSample<i16> + Send + 'static>(
            &self,
            buffer: Arc<AudioBuffer>,
        ) -> Option<cpal::Stream> {
            let ch = self.config.channels;
            self.device
                .build_output_stream::<T, _, _>(
                    self.config,
                    move |out: &mut [T], _| buffer.pull(out, ch, T::from_sample),
                    |_| {},
                    None,
                )
                .ok()
        }
    }

    impl AudioSink for CpalSink {
        fn format(&self) -> (u32, u16) {
            (self.config.sample_rate, self.config.channels)
        }

        fn play(&mut self, buffer: Arc<AudioBuffer>) -> bool {
            self.stop();
            let stream = match self.format {
                SampleFormat::F32 => self.build::<f32>(buffer),
                SampleFormat::I16 => self.build::<i16>(buffer),
                SampleFormat::U16 => self.build::<u16>(buffer),
                SampleFormat::I32 => self.build::<i32>(buffer),
                _ => None,
            };
            match stream {
                Some(s) if s.play().is_ok() => {
                    self.stream = Some(s);
                    true
                }
                _ => false,
            }
        }

        fn stop(&mut self) {
            self.stream = None;
        }
    }
}

/// The best available sink: the default sound device, or [`NullSink`].
pub fn default_sink() -> Box<dyn AudioSink> {
    #[cfg(feature = "audio")]
    if let Some(s) = device::CpalSink::new() {
        return Box::new(s);
    }
    Box::new(NullSink)
}

/// A decoding worker: the child process (killable from the UI) and its thread.
struct Worker {
    process: Arc<Mutex<Box<dyn Process>>>,
    stop: Arc<AtomicBool>,
}

impl Worker {
    fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut p) = self.process.lock() {
            p.kill();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_video(
    mut process: Box<dyn Process>,
    g: Geometry,
    from: Micros,
) -> Option<(Worker, Receiver<Frame>)> {
    let mut out = process.take_stdout()?;
    let (tx, rx) = sync_channel(FRAME_BUFFER);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    std::thread::spawn(move || {
        let size = g.width as usize * g.height as usize * 4;
        let mut n = 0i64;
        while !flag.load(Ordering::SeqCst) {
            let mut buf = vec![0u8; size];
            if out.read_exact(&mut buf).is_err() {
                break;
            }
            let frame = Frame {
                pts: from + g.fps.frame_time(n),
                width: g.width,
                height: g.height,
                rgba: buf,
            };
            if tx.send(frame).is_err() {
                break;
            }
            n += 1;
        }
    });
    Some((
        Worker {
            process: Arc::new(Mutex::new(process)),
            stop,
        },
        rx,
    ))
}

fn spawn_audio(mut process: Box<dyn Process>, buffer: Arc<AudioBuffer>) -> Option<Worker> {
    let mut out = process.take_stdout()?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    std::thread::spawn(move || {
        let mut bytes = [0u8; 8192];
        let mut carry: Option<u8> = None;
        'outer: while !flag.load(Ordering::SeqCst) {
            let n = match out.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let mut data: Vec<u8> = carry.take().into_iter().collect();
            data.extend_from_slice(&bytes[..n]);
            if data.len() % 2 == 1 {
                carry = data.pop();
            }
            let samples: Vec<i16> = data
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect();
            while buffer.len() > AudioBuffer::CAP {
                if flag.load(Ordering::SeqCst) {
                    break 'outer;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            buffer.push(&samples);
        }
        buffer.finished.store(true, Ordering::SeqCst);
    });
    Some(Worker {
        process: Arc::new(Mutex::new(process)),
        stop,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Paused,
    Playing,
}

/// Everything the preview needs to start ffmpeg.
#[derive(Clone)]
pub struct PreviewConfig {
    pub runner: Arc<dyn Runner>,
    pub ffmpeg: PathBuf,
    pub version: FfVersion,
    pub encoders: std::collections::BTreeSet<String>,
}

/// The preview player (seek state machine + clock + frame pacing).
pub struct Preview {
    config: Option<PreviewConfig>,
    sink: Box<dyn AudioSink>,
    state: PlayState,
    /// Timeline position where the current run started (or the paused position).
    start: Micros,
    wall_origin: Instant,
    audio: Option<(Worker, Arc<AudioBuffer>)>,
    video: Option<(Worker, Receiver<Frame>)>,
    /// Paused: show the first frame that arrives, then stop decoding.
    still: bool,
    /// A decoded frame that is not due yet.
    pending: Option<Frame>,
    end: Micros,
    scratch: Option<tempfile::TempDir>,
    pub error: Option<String>,
}

impl Preview {
    pub fn new(config: Option<PreviewConfig>, sink: Box<dyn AudioSink>) -> Preview {
        Preview {
            config,
            sink,
            state: PlayState::Paused,
            start: Micros::ZERO,
            wall_origin: Instant::now(),
            audio: None,
            video: None,
            still: false,
            pending: None,
            end: Micros::ZERO,
            scratch: tempfile::Builder::new()
                .prefix("videolore-preview-")
                .tempdir()
                .ok(),
            error: None,
        }
    }

    pub fn state(&self) -> PlayState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        self.state == PlayState::Playing
    }

    /// Current playback position.
    pub fn position(&self, now: Instant) -> Micros {
        if self.state == PlayState::Paused {
            return self.start;
        }
        let elapsed = match &self.audio {
            Some((_, buf)) => {
                let rate = self.sink.format().0.max(1) as i64;
                Micros(buf.played_frames() as i64 * 1_000_000 / rate)
            }
            None => Micros(now.saturating_duration_since(self.wall_origin).as_micros() as i64),
        };
        (self.start + elapsed).min(self.end)
    }

    fn stop_workers(&mut self) {
        self.sink.stop();
        self.audio = None;
        self.video = None;
        self.still = false;
        self.pending = None;
    }

    fn launch(&mut self, project: &Project, from: Micros, with_audio: bool) -> Result<(), String> {
        self.stop_workers();
        self.error = None;
        let cfg = self.config.clone().ok_or("ffmpeg is not available")?;
        let g = render::output_geometry(project, &cfg.encoders)
            .map_err(|e| e.to_string())?
            .preview();
        let dir = self
            .scratch
            .as_ref()
            .map(|d| d.path().to_path_buf())
            .unwrap_or_else(std::env::temp_dir);
        let vargs = render::preview_video_args(project, from, &g, cfg.version, &dir.join("v.txt"))?;
        let vp = cfg
            .runner
            .spawn(&cfg.ffmpeg, &vargs)
            .map_err(|e| e.to_string())?;
        self.video = spawn_video(vp, g, from);
        if with_audio {
            let (rate, ch) = self.sink.format();
            let aargs = render::preview_audio_args(
                project,
                from,
                rate,
                ch,
                cfg.version,
                &dir.join("a.txt"),
            )?;
            let buf = Arc::new(AudioBuffer::default());
            if self.sink.play(Arc::clone(&buf)) {
                let ap = cfg
                    .runner
                    .spawn(&cfg.ffmpeg, &aargs)
                    .map_err(|e| e.to_string())?;
                self.audio = spawn_audio(ap, Arc::clone(&buf)).map(|w| (w, buf));
            } else {
                self.sink.stop();
            }
        }
        Ok(())
    }

    /// Starts playing from `from`.
    pub fn play(&mut self, project: &Project, from: Micros, now: Instant) -> Result<(), String> {
        self.end = project.timeline.duration();
        let from = if from >= self.end { Micros::ZERO } else { from };
        self.start = from;
        self.wall_origin = now;
        self.state = PlayState::Playing;
        let r = self.launch(project, from, true);
        if let Err(e) = &r {
            self.state = PlayState::Paused;
            self.error = Some(e.clone());
        }
        r
    }

    /// Pauses at the current position.
    pub fn pause(&mut self, now: Instant) {
        self.start = self.position(now);
        self.state = PlayState::Paused;
        self.stop_workers();
    }

    /// Moves to `to`: restarts playback there, or shows the still frame when paused.
    pub fn seek(&mut self, project: &Project, to: Micros, now: Instant) -> Result<(), String> {
        let to = to.clamp(Micros::ZERO, project.timeline.duration());
        if self.is_playing() {
            return self.play(project, to, now);
        }
        self.start = to;
        self.end = project.timeline.duration();
        if project.timeline.clips.is_empty() || to >= self.end {
            self.stop_workers();
            return Ok(());
        }
        let r = self.launch(project, to, false);
        self.still = r.is_ok();
        if let Err(e) = &r {
            self.error = Some(e.clone());
        }
        r
    }

    /// Stops everything (e.g. after an edit) keeping the position.
    pub fn invalidate(&mut self, now: Instant) {
        self.pause(now);
    }

    /// Returns the frame to show now, if a newer one is due. Detects the end of playback.
    pub fn poll(&mut self, now: Instant) -> Option<Frame> {
        let pos = self.position(now);
        let mut shown = None;
        if let Some(f) = self.pending.take() {
            if self.still || f.pts <= pos {
                shown = Some(f);
            } else {
                self.pending = Some(f);
                return None;
            }
        }
        let mut ended = false;
        if let Some((_, rx)) = &self.video {
            while !(self.still && shown.is_some()) {
                match rx.try_recv() {
                    Ok(f) if self.still || f.pts <= pos => shown = Some(f),
                    Ok(f) => {
                        self.pending = Some(f);
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        ended = true;
                        break;
                    }
                }
            }
        }
        if self.still && shown.is_some() {
            self.video = None;
            self.still = false;
        }
        if ended && self.state == PlayState::Playing {
            self.start = self.end;
            self.state = PlayState::Paused;
            self.stop_workers();
        }
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg::CmdOutput;
    use crate::ffmpeg::fake::FakeRunner;
    use crate::time::Fps;
    use crate::timeline::tests::media;

    const W: u32 = 64;
    const H: u32 = 36;

    fn project() -> Project {
        let mut p = Project::default();
        let id = p.new_media_id();
        let mut m = media(id.0, 2, true);
        m.format.width = W;
        m.format.height = H;
        m.format.fps = Fps::new(25, 1);
        p.add_media(m.clone());
        p.timeline.append(&m).unwrap();
        p
    }

    fn runner(frames: usize) -> Arc<dyn Runner> {
        Arc::new(FakeRunner::default().on(move |_, a| {
            let joined = a.join(" ");
            let stdout = if joined.contains("rawvideo") {
                (0..frames)
                    .flat_map(|i| vec![i as u8; (W * H * 4) as usize])
                    .collect()
            } else {
                // 0.1 s of stereo 48 kHz s16le plus an odd trailing byte
                let mut v = vec![1u8; 48_000 / 10 * 4];
                v.push(7);
                v
            };
            Some(CmdOutput {
                success: true,
                stdout,
                stderr: String::new(),
            })
        }))
    }

    fn config(frames: usize) -> Option<PreviewConfig> {
        Some(PreviewConfig {
            runner: runner(frames),
            ffmpeg: "ffmpeg".into(),
            version: FfVersion::Release(6, 1, 0),
            encoders: ["libx264", "aac"].iter().map(|s| s.to_string()).collect(),
        })
    }

    #[derive(Default)]
    struct FakeSink {
        ok: bool,
        buf: Arc<Mutex<Option<Arc<AudioBuffer>>>>,
    }

    impl AudioSink for FakeSink {
        fn format(&self) -> (u32, u16) {
            (48_000, 2)
        }
        fn play(&mut self, buffer: Arc<AudioBuffer>) -> bool {
            *self.buf.lock().unwrap() = Some(buffer);
            self.ok
        }
        fn stop(&mut self) {}
    }

    fn poll_until(
        p: &mut Preview,
        at: Instant,
        f: impl Fn(&Preview, &Option<Frame>) -> bool,
    ) -> Option<Frame> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut last = None;
        while Instant::now() < deadline {
            let fr = p.poll(at);
            if fr.is_some() {
                last = fr.clone();
            }
            if f(p, &fr) {
                return last;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("timeout");
    }

    // covers: REQ-013
    #[test]
    fn plays_from_playhead_and_stops_at_end() {
        let mut p = Preview::new(config(3), Box::new(NullSink));
        let t0 = Instant::now();
        p.play(&project(), Micros(1_000_000), t0).unwrap();
        assert!(p.is_playing());
        // Not due yet at t0 + 0: only the first frame (pts == start).
        let first = poll_until(&mut p, t0, |_, f| f.is_some()).unwrap();
        assert_eq!(first.pts, Micros(1_000_000));
        assert_eq!(first.rgba.len(), (W * H * 4) as usize);
        assert_eq!((first.width, first.height), (W, H));
        // A second later everything is due and the stream ends.
        let later = t0 + Duration::from_secs(1);
        let last = poll_until(&mut p, later, |p, _| !p.is_playing()).unwrap();
        assert_eq!(last.pts, Micros(1_080_000));
        assert_eq!(p.state(), PlayState::Paused);
        assert_eq!(p.position(later), Micros(2_000_000));
    }

    // covers: REQ-013
    #[test]
    fn paused_seek_shows_a_still_frame_and_pause_keeps_position() {
        let mut p = Preview::new(config(5), Box::new(NullSink));
        let t0 = Instant::now();
        let proj = project();
        p.seek(&proj, Micros(500_000), t0).unwrap();
        let f = poll_until(&mut p, t0, |_, f| f.is_some()).unwrap();
        assert_eq!(f.pts, Micros(500_000));
        assert!(p.poll(t0).is_none()); // decoding stopped after the still
        assert_eq!(p.position(t0), Micros(500_000));
        p.seek(&proj, Micros(99_000_000), t0).unwrap(); // clamped to the end: nothing to show
        assert_eq!(p.position(t0), Micros(2_000_000));
        // playing from the end restarts from zero
        p.play(&proj, Micros(2_000_000), t0).unwrap();
        assert_eq!(p.position(t0), Micros::ZERO);
        p.seek(&proj, Micros(300_000), t0).unwrap(); // seek while playing restarts there
        assert!(p.is_playing());
        p.pause(t0 + Duration::from_millis(100));
        assert_eq!(p.position(t0 + Duration::from_secs(9)), Micros(400_000));
        p.invalidate(t0);
        assert!(!p.is_playing());
    }

    // covers: REQ-013
    #[test]
    fn audio_drives_the_clock() {
        let sink = FakeSink {
            ok: true,
            ..Default::default()
        };
        let shared = Arc::clone(&sink.buf);
        let mut p = Preview::new(config(1), Box::new(sink));
        let t0 = Instant::now();
        p.play(&project(), Micros::ZERO, t0).unwrap();
        let buf = shared.lock().unwrap().clone().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while buf.len() < 9_600 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(buf.len(), 9_600);
        // the wall clock does not matter, only played samples do
        assert_eq!(p.position(t0 + Duration::from_secs(10)), Micros::ZERO);
        let mut out = vec![0f32; 4_800 * 2];
        buf.pull(&mut out, 2, |s| s as f32);
        assert_eq!(p.position(t0), Micros(100_000));
        assert_eq!(out[0], 257.0);
        // starved: silence, no progress
        let mut out = vec![9i16; 4];
        buf.pull(&mut out, 2, |s| s);
        assert_eq!(out, vec![0; 4]);
        assert_eq!(buf.played_frames(), 4_800);
        assert!(buf.is_empty());
    }

    #[test]
    fn errors_are_reported() {
        let mut p = Preview::new(None, Box::new(NullSink));
        let t0 = Instant::now();
        assert!(p.play(&project(), Micros::ZERO, t0).is_err());
        assert!(!p.is_playing());
        assert!(p.error.is_some());
        assert!(p.seek(&project(), Micros(10), t0).is_err());
        let mut p = Preview::new(config(1), Box::new(FakeSink::default()));
        assert!(p.play(&Project::default(), Micros::ZERO, t0).is_err());
        assert!(p.seek(&Project::default(), Micros::ZERO, t0).is_ok());
        // a sink that refuses to play falls back to the wall clock
        p.play(&project(), Micros::ZERO, t0).unwrap();
        assert_eq!(p.position(t0 + Duration::from_millis(250)), Micros(250_000));
        let n = NullSink;
        assert_eq!(n.format(), (48_000, 2));
        let _ = default_sink().format();
    }
}
