// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Preview playback. ffmpeg renders the edited timeline (fades, music, subtitles included)
//! from the playhead as raw RGBA frames (<= 360p) and s16le PCM on stdout.
//!
//! * **Clock:** the wall clock is the master. Audio is corrected towards it (dropping or
//!   padding samples when it drifts more than 40 ms), so the video never waits for audio.
//! * **Playback** needs two ffmpeg processes (video + audio), started together from a
//!   worker thread so the UI never waits for a free slot.
//! * **Still frames** (paused): a single-input render of the frame at the playhead. A frame
//!   is rendered only when its [`FrameKey`] differs from the one shown, after an 80 ms quiet
//!   period, through a latest-only mailbox (an older pending or running render is dropped).
//! * Proxies are used when ready and enabled.

use crate::export::write_subtitles;
use crate::ffmpeg::{FfVersion, Process, Runner};
use crate::hwdec::{self, Decoder};
use crate::hwenc::Codec;
use crate::jobs::Limited;
use crate::project::Project;
use crate::render::{self, FrameKey, Geometry, Source};
use crate::time::Micros;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel, sync_channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// The format of each video file a preview run of `project` reads, by the argument that
/// names it (`file:<path>`). A preview copy is always H.264.
fn input_codecs(project: &Project, src: Source) -> HashMap<OsString, Codec> {
    project
        .media
        .iter()
        .filter_map(|m| {
            let path = src.path_of(m);
            let codec = if path == m.path {
                Codec::from_name(m.format.vcodec.as_deref()?)?
            } else {
                Codec::H264
            };
            Some((crate::ffmpeg::file_arg(&path), codec))
        })
        .collect()
}

/// Frames buffered between the decoder and the UI.
pub const FRAME_BUFFER: usize = 8;
/// Quiet period before a still frame is rendered (edit bursts are coalesced).
pub const STILL_DEBOUNCE: Duration = Duration::from_millis(80);
/// Audio is corrected when it drifts more than this from the wall clock.
pub const DRIFT_THRESHOLD: Micros = Micros(40_000);

/// One decoded preview frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub pts: Micros,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// What to do to bring the audio back to the wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correction {
    None,
    /// Skip this many frames (audio is late).
    Drop(u64),
    /// Play this many frames of silence (audio is early).
    Pad(u64),
}

/// Compares the audio stream position with the wall clock (both in frames at `rate`).
pub fn audio_correction(stream_frames: u64, wall_frames: u64, rate: u32) -> Correction {
    let threshold = u64::from(rate) * DRIFT_THRESHOLD.0 as u64 / 1_000_000;
    if wall_frames > stream_frames + threshold {
        Correction::Drop(wall_frames - stream_frames)
    } else if stream_frames > wall_frames + threshold {
        Correction::Pad(stream_frames - wall_frames)
    } else {
        Correction::None
    }
}

/// PCM shared between the decoder thread and the audio device callback.
#[derive(Debug)]
pub struct AudioBuffer {
    samples: Mutex<VecDeque<i16>>,
    channels: u16,
    /// Frames of the stream used so far (played or dropped): the audio clock.
    consumed: AtomicU64,
    /// Frames handed to the device, silence included.
    device: AtomicU64,
    /// Frames still to drop from data not received yet.
    owed_drop: AtomicU64,
    /// Frames of silence still to play before the next samples.
    owed_silence: AtomicU64,
    /// Decoder reached the end.
    finished: AtomicBool,
}

impl Default for AudioBuffer {
    fn default() -> Self {
        AudioBuffer::new(2)
    }
}

impl AudioBuffer {
    /// Max samples kept in memory (about one second of stereo 48 kHz), whatever the file.
    pub const CAP: usize = 96_000;

    pub fn new(channels: u16) -> AudioBuffer {
        AudioBuffer {
            samples: Mutex::new(VecDeque::new()),
            channels: channels.max(1),
            consumed: AtomicU64::new(0),
            device: AtomicU64::new(0),
            owed_drop: AtomicU64::new(0),
            owed_silence: AtomicU64::new(0),
            finished: AtomicBool::new(false),
        }
    }

    fn ch(&self) -> usize {
        usize::from(self.channels)
    }

    /// Appends decoded samples, first paying any pending drop.
    pub fn push(&self, data: &[i16]) {
        let owed = self.owed_drop.load(Ordering::SeqCst) as usize;
        let skip_frames = owed.min(data.len() / self.ch());
        if skip_frames > 0 {
            self.owed_drop
                .fetch_sub(skip_frames as u64, Ordering::SeqCst);
            self.consumed
                .fetch_add(skip_frames as u64, Ordering::SeqCst);
        }
        if let Ok(mut q) = self.samples.lock() {
            q.extend(data[skip_frames * self.ch()..].iter().copied());
        }
    }

    pub fn len(&self) -> usize {
        self.samples.lock().map(|q| q.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `out` (interleaved) with samples mapped by `conv`; silence when starved or
    /// while padding.
    pub fn pull<T>(&self, out: &mut [T], conv: impl Fn(i16) -> T) {
        let ch = self.ch();
        let frames = out.len() / ch;
        let pad = (self.owed_silence.load(Ordering::SeqCst) as usize).min(frames);
        self.owed_silence.fetch_sub(pad as u64, Ordering::SeqCst);
        let mut taken = 0usize;
        if let Ok(mut q) = self.samples.lock() {
            for (i, o) in out.iter_mut().enumerate() {
                if i < pad * ch {
                    *o = conv(0);
                    continue;
                }
                match q.pop_front() {
                    Some(s) => {
                        *o = conv(s);
                        taken += 1;
                    }
                    None => *o = conv(0),
                }
            }
        }
        self.consumed
            .fetch_add((taken / ch) as u64, Ordering::SeqCst);
        self.device.fetch_add(frames as u64, Ordering::SeqCst);
    }

    /// Stream position in frames, counting corrections already decided.
    pub fn position_frames(&self) -> u64 {
        (self.consumed.load(Ordering::SeqCst) + self.owed_drop.load(Ordering::SeqCst))
            .saturating_sub(self.owed_silence.load(Ordering::SeqCst))
    }

    /// Frames handed to the device so far.
    pub fn device_frames(&self) -> u64 {
        self.device.load(Ordering::SeqCst)
    }

    /// Applies a correction.
    pub fn correct(&self, c: Correction) {
        match c {
            Correction::None => {}
            Correction::Drop(n) => {
                let ch = self.ch();
                let mut dropped = 0u64;
                if let Ok(mut q) = self.samples.lock() {
                    let k = (n as usize).min(q.len() / ch);
                    q.drain(..k * ch);
                    dropped = k as u64;
                }
                self.consumed.fetch_add(dropped, Ordering::SeqCst);
                self.owed_drop.fetch_add(n - dropped, Ordering::SeqCst);
            }
            Correction::Pad(n) => {
                self.owed_silence.fetch_add(n, Ordering::SeqCst);
            }
        }
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

/// No audio device: the preview is silent.
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
            self.device
                .build_output_stream::<T, _, _>(
                    self.config,
                    move |out: &mut [T], _| buffer.pull(out, T::from_sample),
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

/// Processes of a playback run (killable from the UI thread) and the stop flag.
struct Worker {
    stop: Arc<AtomicBool>,
    processes: Arc<Mutex<Vec<Box<dyn Process>>>>,
}

impl Worker {
    fn new() -> Worker {
        Worker {
            stop: Arc::new(AtomicBool::new(false)),
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut ps) = self.processes.lock() {
            for p in ps.iter_mut() {
                p.kill();
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn read_frames(
    mut out: Box<dyn Read + Send>,
    g: Geometry,
    from: Micros,
    stop: &AtomicBool,
    tx: &std::sync::mpsc::SyncSender<Frame>,
) {
    let size = g.width as usize * g.height as usize * 4;
    let mut n = 0i64;
    while !stop.load(Ordering::SeqCst) {
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
}

fn read_audio(mut out: Box<dyn Read + Send>, buffer: &AudioBuffer, stop: &AtomicBool) {
    let mut bytes = [0u8; 8192];
    let mut carry: Option<u8> = None;
    'outer: while !stop.load(Ordering::SeqCst) {
        let n = match out.read(&mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut data: Vec<u8> = carry.take().into_iter().collect();
        data.extend_from_slice(&bytes[..n]);
        if data.len() % 2 == 1 {
            carry = data.pop();
        }
        // `data` has an even length here, so the remainder is always empty.
        let (pairs, _) = data.as_chunks::<2>();
        let samples: Vec<i16> = pairs.iter().map(|&p| i16::from_le_bytes(p)).collect();
        while buffer.len() > AudioBuffer::CAP {
            if stop.load(Ordering::SeqCst) {
                break 'outer;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        buffer.push(&samples);
    }
    buffer.finished.store(true, Ordering::SeqCst);
}

/// Sent on the error channel when a run that decodes with the video hardware ended badly
/// (REQ-214): the preview goes on in software, without a message.
const HW_DECODE_FAILED: &str = "\u{0}hardware decoding failed";

/// Everything playback needs, moved into its worker thread.
struct PlaybackJob {
    runner: Limited,
    ffmpeg: PathBuf,
    cwd: PathBuf,
    video: Vec<OsString>,
    audio: Option<(Vec<OsString>, Arc<AudioBuffer>)>,
    geometry: Geometry,
    from: Micros,
    /// The video process decodes with the video hardware.
    hw: bool,
}

/// Starts a playback run: spawning waits for scheduler slots inside the thread.
fn start_playback(job: PlaybackJob, errors: Sender<String>) -> (Worker, Receiver<Frame>) {
    let worker = Worker::new();
    let (tx, rx) = sync_channel(FRAME_BUFFER);
    let stop = Arc::clone(&worker.stop);
    let processes = Arc::clone(&worker.processes);
    std::thread::spawn(move || {
        let runner = job.runner.cancellable(Arc::clone(&stop));
        let ff = job.ffmpeg.as_path();
        let spawned = match &job.audio {
            Some((aargs, _)) => runner
                .spawn_pair((ff, &job.video), (ff, aargs), Some(&job.cwd))
                .map(|(v, a)| (v, Some(a))),
            None => runner
                .spawn_in(ff, &job.video, Some(&job.cwd))
                .map(|v| (v, None)),
        };
        let (mut vp, ap) = match spawned {
            Ok(p) => p,
            Err(e) => {
                if !stop.load(Ordering::SeqCst) {
                    let _ = errors.send(e.to_string());
                }
                return;
            }
        };
        let vout = vp.take_stdout();
        let mut audio_reader = None;
        if let (Some(mut ap), Some((_, buf))) = (ap, job.audio) {
            if let Some(aout) = ap.take_stdout() {
                let stop = Arc::clone(&stop);
                audio_reader = Some(std::thread::spawn(move || read_audio(aout, &buf, &stop)));
            }
            if let Ok(mut ps) = processes.lock() {
                ps.push(ap);
            }
        }
        if let Ok(mut ps) = processes.lock() {
            ps.push(vp);
            if stop.load(Ordering::SeqCst) {
                ps.iter_mut().for_each(|p| p.kill());
            }
        }
        if let Some(out) = vout {
            read_frames(out, job.geometry, job.from, &stop, &tx);
        }
        drop(tx);
        if job.hw && !stop.load(Ordering::SeqCst) {
            // the video process is the last one started; its end tells whether the hardware
            // decoder worked
            let ended = processes
                .lock()
                .ok()
                .and_then(|mut ps| ps.last_mut().map(|p| p.wait()));
            if !ended.is_some_and(|r| r.is_ok_and(|o| o.success)) && !stop.load(Ordering::SeqCst) {
                let _ = errors.send(HW_DECODE_FAILED.to_string());
            }
        }
        if let Some(t) = audio_reader {
            let _ = t.join();
        }
    });
    (worker, rx)
}

/// What the still thread has drawn so far.
#[derive(Debug, Default)]
struct StillCounters {
    /// Pictures drawn (a whole still, or the picture over a kept background).
    pictures: AtomicU64,
    /// Blurred backgrounds drawn for the cache.
    backgrounds: AtomicU64,
}

/// How a still keeps the blurred background of its frame (REQ-223).
struct KeptBackground {
    /// The file (in the still folder) that holds the background.
    name: String,
    /// The picture drawn over the kept background.
    picture_args: Vec<OsString>,
    /// Makes the background alone (the raw frame on stdout).
    make_args: Vec<OsString>,
}

/// A still frame to render.
struct StillRequest {
    generation: u64,
    args: Vec<OsString>,
    /// With a blurred fill: the kept background and the way to use it.
    background: Option<KeptBackground>,
    subtitles: Option<String>,
    command_files: Vec<(String, String)>,
    lut_files: Vec<Arc<crate::lut::Lut>>,
    geometry: Geometry,
    at: Micros,
    key: FrameKey,
}

/// The render in progress: its cancel flag and (once started) its process.
type Running = (Arc<AtomicBool>, Option<Arc<Mutex<Box<dyn Process>>>>);

/// Latest-only mailbox between the UI and the still-frame thread.
#[derive(Default)]
struct Mailbox {
    slot: Mutex<Option<StillRequest>>,
    cv: Condvar,
    latest: AtomicU64,
    quit: AtomicBool,
    running: Mutex<Option<Running>>,
}

impl Mailbox {
    fn post(&self, req: StillRequest) {
        self.latest.store(req.generation, Ordering::SeqCst);
        self.post_cancel();
        if let Ok(mut s) = self.slot.lock() {
            *s = Some(req);
        }
        self.cv.notify_all();
    }

    fn take(&self) -> Option<StillRequest> {
        let mut s = self.slot.lock().ok()?;
        loop {
            if self.quit.load(Ordering::SeqCst) {
                return None;
            }
            if let Some(r) = s.take() {
                return Some(r);
            }
            s = self.cv.wait(s).ok()?;
        }
    }

    fn shutdown(&self) {
        self.quit.store(true, Ordering::SeqCst);
        self.post_cancel();
        self.cv.notify_all();
    }

    /// Stops the render in progress: it is obsolete.
    fn post_cancel(&self) {
        let running = self.running.lock().ok().and_then(|mut r| r.take());
        if let Some((flag, process)) = running {
            flag.store(true, Ordering::SeqCst);
            if let Some(p) = process
                && let Ok(mut p) = p.lock()
            {
                p.kill();
            }
        }
    }
}

/// Result of a still render.
/// `None` when the render failed (so the request is not considered in flight forever).
type StillResult = (u64, FrameKey, Option<Frame>);

/// Renders one still: `None` when it failed or was cancelled.
fn render_still(
    mailbox: &Mailbox,
    config: &PreviewConfig,
    cwd: &std::path::Path,
    counters: &StillCounters,
    req: &StillRequest,
) -> Option<Frame> {
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut r) = mailbox.running.lock() {
        *r = Some((Arc::clone(&cancel), None));
    }
    if let Some(doc) = &req.subtitles {
        write_subtitles(cwd, doc).ok()?;
    }
    render::write_command_files(cwd, &req.command_files, &req.lut_files).ok()?;
    let runner = config.runner.cancellable(Arc::clone(&cancel));
    // a blurred background that did not change is not drawn again (REQ-223); a render that
    // does not work out falls back to the whole picture
    if let Some(kept) = &req.background
        && let Some(frame) =
            render_with_kept_background(mailbox, &runner, config, cwd, counters, req, kept, &cancel)
    {
        return Some(frame);
    }
    if cancel.load(Ordering::SeqCst) {
        return None;
    }
    counters.pictures.fetch_add(1, Ordering::SeqCst);
    run_still(mailbox, &runner, config, cwd, &req.args, req, &cancel)
}

/// The still drawn over the background kept in its file, making the background first when
/// the file is not there.
#[allow(clippy::too_many_arguments)]
fn render_with_kept_background(
    mailbox: &Mailbox,
    runner: &dyn Runner,
    config: &PreviewConfig,
    cwd: &std::path::Path,
    counters: &StillCounters,
    req: &StillRequest,
    kept: &KeptBackground,
    cancel: &Arc<AtomicBool>,
) -> Option<Frame> {
    let file = cwd.join(&kept.name);
    let bytes = req.geometry.width as u64 * req.geometry.height as u64 * 4;
    let there = |f: &std::path::Path| std::fs::metadata(f).is_ok_and(|m| m.len() == bytes);
    if !there(&file) {
        counters.backgrounds.fetch_add(1, Ordering::SeqCst);
        let bg = run_still(mailbox, runner, config, cwd, &kept.make_args, req, cancel)?;
        // one background at a time: the older ones go
        if let Ok(rd) = std::fs::read_dir(cwd) {
            for e in rd.flatten() {
                let n = e.file_name();
                if n.to_string_lossy().starts_with("bg-") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let partial = cwd.join(format!("{}.partial", kept.name));
        std::fs::write(&partial, &bg.rgba).ok()?;
        std::fs::rename(&partial, &file).ok()?;
    }
    if !there(&file) || cancel.load(Ordering::SeqCst) {
        return None;
    }
    counters.pictures.fetch_add(1, Ordering::SeqCst);
    run_still(
        mailbox,
        runner,
        config,
        cwd,
        &kept.picture_args,
        req,
        cancel,
    )
}

/// Runs one still render and reads its frame.
fn run_still(
    mailbox: &Mailbox,
    runner: &dyn Runner,
    config: &PreviewConfig,
    cwd: &std::path::Path,
    args: &[OsString],
    req: &StillRequest,
    cancel: &Arc<AtomicBool>,
) -> Option<Frame> {
    let mut p = runner.spawn_in(&config.ffmpeg, args, Some(cwd)).ok()?;
    let stdout = p.take_stdout();
    let p = Arc::new(Mutex::new(p));
    if let Ok(mut r) = mailbox.running.lock() {
        match r.as_mut() {
            // a newer request may have arrived meanwhile
            Some((flag, slot)) if Arc::ptr_eq(flag, cancel) => *slot = Some(Arc::clone(&p)),
            _ => cancel.store(true, Ordering::SeqCst),
        }
    }
    let g = req.geometry;
    let mut buf = vec![0u8; g.width as usize * g.height as usize * 4];
    let ok = !cancel.load(Ordering::SeqCst)
        && stdout.is_some_and(|mut s| s.read_exact(&mut buf).is_ok())
        && !cancel.load(Ordering::SeqCst);
    if let Ok(mut p) = p.lock() {
        p.kill();
    }
    ok.then_some(Frame {
        pts: req.at,
        width: g.width,
        height: g.height,
        rgba: buf,
    })
}

fn still_thread(
    mailbox: Arc<Mailbox>,
    config: PreviewConfig,
    cwd: PathBuf,
    renders: Arc<StillCounters>,
    out: Sender<StillResult>,
) {
    while let Some(req) = mailbox.take() {
        if req.generation != mailbox.latest.load(Ordering::SeqCst) {
            continue;
        }
        let frame = render_still(&mailbox, &config, &cwd, &renders, &req);
        if out.send((req.generation, req.key, frame)).is_err() {
            return;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Paused,
    Playing,
}

/// Everything the preview needs to start ffmpeg.
#[derive(Clone)]
pub struct PreviewConfig {
    /// Preview-priority runner.
    pub runner: Limited,
    pub ffmpeg: PathBuf,
    pub version: FfVersion,
    pub encoders: std::collections::BTreeSet<String>,
}

/// A still frame waiting for the quiet period to end.
struct Wanted {
    request: StillRequest,
    due: Instant,
}

/// The preview player.
pub struct Preview {
    config: Option<PreviewConfig>,
    sink: Box<dyn AudioSink>,
    state: PlayState,
    /// Timeline position where the current run started (or the paused position).
    start: Micros,
    wall_origin: Instant,
    /// How long the last launch waited for the ambisonic pre-pass (REQ-273): the wall clock
    /// of the run starts after it.
    prepass: Duration,
    end: Micros,
    playback: Option<(Worker, Receiver<Frame>)>,
    audio: Option<Arc<AudioBuffer>>,
    errors: (Sender<String>, Receiver<String>),
    /// A decoded frame that is not due yet.
    pending: Option<Frame>,
    // still frames
    mailbox: Arc<Mailbox>,
    stills: Option<Receiver<StillResult>>,
    renders: Arc<StillCounters>,
    /// The generation whose result may be shown (older results are stale and dropped).
    generation: u64,
    /// Last generation handed out (never reused, so a dropped request cannot be confused
    /// with a newer one).
    issued: u64,
    wanted: Option<Wanted>,
    shown_key: Option<FrameKey>,
    /// The render in flight: its key and generation (REQ-144).
    posted: Option<(FrameKey, u64)>,
    scratch: Option<tempfile::TempDir>,
    /// Use proxies when ready.
    pub proxies: bool,
    /// Pictures are made at half the size while the camera is being dragged (REQ-267).
    draft: bool,
    pub error: Option<String>,
    /// The video hardware decoder of each format (empty: the normal decoder; REQ-214).
    decoders: HashMap<Codec, Decoder>,
    /// A run with the hardware decoder failed: the app restarts at the playhead.
    hw_fallback: bool,
    /// Runs that fell back to the normal decoder.
    pub hw_fallbacks: u32,
}

/// Half of each side, even, at least 2: the picture made while a 360 camera is dragged.
pub fn draft_geometry(g: &render::Geometry) -> render::Geometry {
    render::Geometry {
        width: ((g.width / 2) & !1).max(2),
        height: ((g.height / 2) & !1).max(2),
        fps: g.fps,
    }
}

impl Preview {
    /// Pictures at half the size (REQ-267) while a camera is dragged, full size otherwise.
    pub fn set_draft(&mut self, on: bool) {
        self.draft = on;
    }

    pub fn is_draft(&self) -> bool {
        self.draft
    }

    pub fn new(config: Option<PreviewConfig>, sink: Box<dyn AudioSink>) -> Preview {
        // Private (0700): it holds the project's subtitle text.
        let scratch = tempfile::Builder::new()
            .prefix("videolore-preview-")
            .tempdir()
            .ok()
            .filter(|d| crate::base::private::private_dir(d.path()).is_ok());
        let mailbox = Arc::new(Mailbox::default());
        let renders = Arc::new(StillCounters::default());
        let stills = match (&config, &scratch) {
            (Some(cfg), Some(dir)) => {
                let cwd = dir.path().join("still");
                let _ = std::fs::create_dir_all(&cwd);
                let (tx, rx) = channel();
                let (m, c, r) = (Arc::clone(&mailbox), cfg.clone(), Arc::clone(&renders));
                std::thread::spawn(move || still_thread(m, c, cwd, r, tx));
                Some(rx)
            }
            _ => None,
        };
        Preview {
            config,
            sink,
            state: PlayState::Paused,
            start: Micros::ZERO,
            wall_origin: Instant::now(),
            prepass: Duration::ZERO,
            end: Micros::ZERO,
            playback: None,
            audio: None,
            errors: channel(),
            pending: None,
            mailbox,
            stills,
            renders,
            generation: 0,
            issued: 0,
            wanted: None,
            shown_key: None,
            posted: None,
            scratch,
            proxies: true,
            draft: false,
            error: None,
            decoders: HashMap::new(),
            hw_fallback: false,
            hw_fallbacks: 0,
        }
    }

    /// The hardware decoders playback uses (REQ-214): the app gives them with Export:
    /// Faster and an empty map with Best quality, where no hardware decoding runs.
    pub fn set_decoders(&mut self, decoders: HashMap<Codec, Decoder>) {
        self.decoders = decoders;
    }

    /// True once after a run with a hardware decoder failed: the decoders are dropped and
    /// the caller starts the playback again at the playhead (the last picture stays on
    /// screen, REQ-144). No message is shown.
    pub fn take_hw_fallback(&mut self) -> bool {
        std::mem::take(&mut self.hw_fallback)
    }

    /// Whether a run would decode with hardware now.
    pub fn uses_hw_decoding(&self) -> bool {
        !self.decoders.is_empty()
    }

    pub fn state(&self) -> PlayState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        self.state == PlayState::Playing
    }

    /// Still frames actually rendered so far (spawned ffmpeg runs).
    pub fn still_renders(&self) -> u64 {
        self.renders.pictures.load(Ordering::SeqCst)
    }

    /// Blurred backgrounds drawn for the cache so far (REQ-223): a change of Scale, Move or
    /// keyframes draws the picture again but not the background.
    pub fn background_renders(&self) -> u64 {
        self.renders.backgrounds.load(Ordering::SeqCst)
    }

    /// True while something is about to change on screen (repaint soon).
    pub fn busy(&self) -> bool {
        self.is_playing() || self.wanted.is_some() || self.posted.is_some()
    }

    fn source(&self) -> Source {
        Source::Preview {
            proxies: self.proxies,
        }
    }

    fn dir(&self, sub: &str) -> PathBuf {
        let d = self
            .scratch
            .as_ref()
            .map(|d| d.path().to_path_buf())
            .unwrap_or_else(std::env::temp_dir)
            .join(sub);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    /// Current playback position (wall clock, pauses excluded).
    pub fn position(&self, now: Instant) -> Micros {
        if self.state == PlayState::Paused {
            return self.start;
        }
        let elapsed = Micros(now.saturating_duration_since(self.wall_origin).as_micros() as i64);
        (self.start + elapsed).min(self.end)
    }

    fn stop_workers(&mut self) {
        self.sink.stop();
        self.playback = None;
        self.audio = None;
        self.pending = None;
    }

    /// Turns the ambisonic sound of the clips from `from` on by their cameras into the play
    /// folder, where the audio graph reads it (REQ-273). The files of earlier camera paths
    /// are removed. The preview waits for it; [`Self::prepass`] says how long.
    fn prepare_spatial(
        &mut self,
        cfg: &PreviewConfig,
        cwd: &std::path::Path,
        project: &Project,
        from: Micros,
        src: Source,
    ) -> Result<(), String> {
        let jobs = render::preview_spatial(project, from, src)?;
        let keep: Vec<String> = jobs.iter().map(|j| j.wav_name()).collect();
        if let Ok(rd) = std::fs::read_dir(cwd) {
            for e in rd.filter_map(Result::ok) {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("spatial-") && !keep.contains(&name) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let started = Instant::now();
        let never = AtomicBool::new(false);
        for j in &jobs {
            crate::spatial::prepare(&cfg.runner, &cfg.ffmpeg, cwd, j, &never)
                .map_err(|e| format!("The sound could not be turned with the camera: {e}"))?;
        }
        if !jobs.is_empty() {
            self.prepass = started.elapsed();
        }
        Ok(())
    }

    fn launch(&mut self, project: &Project, from: Micros) -> Result<(), String> {
        self.stop_workers();
        self.error = None;
        self.prepass = Duration::ZERO;
        let cfg = self.config.clone().ok_or("ffmpeg is not available")?;
        let eg = render::output_geometry(project, &cfg.encoders).map_err(|e| e.to_string())?;
        let cwd = self.dir("play");
        let src = self.source();
        let v =
            render::preview_video_args(project, from, &eg, cfg.version, &cwd.join("v.txt"), src)?;
        if let Some(doc) = &v.subtitles {
            write_subtitles(&cwd, doc).map_err(|e| e.to_string())?;
        }
        render::write_command_files(&cwd, &v.command_files, &v.lut_files)
            .map_err(|e| e.to_string())?;
        self.prepare_spatial(&cfg, &cwd, project, from, src)?;
        let (rate, ch) = self.sink.format();
        let aargs = render::preview_audio_args(
            project,
            from,
            rate,
            ch,
            cfg.version,
            &cwd.join("a.txt"),
            src,
        )?;
        let buf = Arc::new(AudioBuffer::new(ch));
        let audio = if self.sink.play(Arc::clone(&buf)) {
            self.audio = Some(Arc::clone(&buf));
            Some((aargs, buf))
        } else {
            self.sink.stop();
            None
        };
        let hw = !self.decoders.is_empty();
        let video = if hw {
            let known = input_codecs(project, src);
            hwdec::apply(v.args, &self.decoders, &|f| known.get(f).copied())
        } else {
            v.args
        };
        let job = PlaybackJob {
            runner: cfg.runner,
            ffmpeg: cfg.ffmpeg,
            cwd,
            video,
            audio,
            geometry: eg.preview(),
            from,
            hw,
        };
        self.playback = Some(start_playback(job, self.errors.0.clone()));
        Ok(())
    }

    /// Starts playing from `from`.
    pub fn play(&mut self, project: &Project, from: Micros, now: Instant) -> Result<(), String> {
        self.end = project.timeline.duration();
        let from = if from >= self.end { Micros::ZERO } else { from };
        self.start = from;
        self.wall_origin = now;
        self.state = PlayState::Playing;
        self.supersede();
        let r = self.launch(project, from);
        // the wait for the turned sound is not part of the playing time
        self.wall_origin = now + self.prepass;
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

    /// Moves to `to`: restarts playback there, or asks for the still frame when paused.
    pub fn seek(&mut self, project: &Project, to: Micros, now: Instant) -> Result<(), String> {
        let to = to.clamp(Micros::ZERO, project.timeline.duration());
        if self.is_playing() {
            return self.play(project, to, now);
        }
        self.start = to;
        self.end = project.timeline.duration();
        self.request_still(project, to, now)
    }

    /// Stops playback (e.g. after an edit) keeping the position.
    pub fn invalidate(&mut self, now: Instant) {
        self.pause(now);
    }

    /// Asks for the picture at `at` (paused). Nothing happens when the picture would be the
    /// one already shown or already requested; otherwise it is rendered after
    /// [`STILL_DEBOUNCE`] without newer requests.
    pub fn request_still(
        &mut self,
        project: &Project,
        at: Micros,
        now: Instant,
    ) -> Result<(), String> {
        self.start = at;
        self.end = project.timeline.duration();
        let Some(cfg) = self.config.clone() else {
            let e = "ffmpeg is not available".to_string();
            self.error = Some(e.clone());
            return Err(e);
        };
        let Ok(eg) = render::output_geometry(project, &cfg.encoders) else {
            self.wanted = None;
            return Ok(());
        };
        let eg = if self.draft { draft_geometry(&eg) } else { eg };
        let src = self.source();
        let Some(key) = render::frame_key(project, at, src, &eg) else {
            self.wanted = None;
            return Ok(());
        };
        // One rule (REQ-144): the newest request owns the screen. A request for the render in
        // flight keeps it; a request for the picture already shown makes every other render
        // obsolete; anything else supersedes what is posted or waiting.
        if let Some((k, g)) = &self.posted
            && *k == key
        {
            self.wanted = None;
            self.generation = *g;
            return Ok(());
        }
        if self.shown_key.as_ref() == Some(&key) {
            self.supersede();
            self.shown_key = Some(key);
            self.mailbox.post_cancel();
            return Ok(());
        }
        let script = self.dir("still").join("g.txt");
        let r = render::still_args(project, at, &eg, cfg.version, &script, src);
        let r = match r {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(e.clone());
                return Err(e);
            }
        };
        let background = self.kept_background(project, at, &eg, &cfg, src, &key);
        self.error = None;
        self.wanted = None;
        self.posted = None;
        self.issued += 1;
        self.generation = self.issued;
        self.wanted = Some(Wanted {
            request: StillRequest {
                generation: self.generation,
                args: r.args,
                background,
                subtitles: r.subtitles,
                command_files: r.command_files,
                lut_files: r.lut_files,
                geometry: eg.preview(),
                at,
                key,
            },
            due: now + STILL_DEBOUNCE,
        });
        Ok(())
    }

    /// The way to keep the blurred background of the still at `at` (REQ-223): `None` when
    /// the picture has no blurred fill that can be kept.
    fn kept_background(
        &self,
        project: &Project,
        at: Micros,
        eg: &Geometry,
        cfg: &PreviewConfig,
        src: Source,
        key: &FrameKey,
    ) -> Option<KeptBackground> {
        let name = key.background_name()?;
        let dir = self.dir("still");
        let raw = dir.join(&name);
        let args = |bg, script: &str, raw: Option<&std::path::Path>| {
            render::still_args_bg(
                project,
                at,
                eg,
                cfg.version,
                &dir.join(script),
                src,
                bg,
                raw,
            )
            .ok()
            .map(|r| r.args)
        };
        Some(KeptBackground {
            picture_args: args(render::Background::Cached(0), "gc.txt", Some(&raw))?,
            make_args: args(render::Background::Only, "gb.txt", None)?,
            name,
        })
    }

    /// Forgets the shown picture (the next request renders even if the key is the same).
    pub fn forget_frame(&mut self) {
        self.supersede();
    }

    /// Makes every render requested so far obsolete and forgets the shown picture.
    fn supersede(&mut self) {
        self.issued += 1;
        self.generation = self.issued;
        self.shown_key = None;
        self.posted = None;
        self.wanted = None;
    }

    fn poll_stills(&mut self, now: Instant) -> Option<Frame> {
        if let Some(w) = self.wanted.take() {
            if now >= w.due {
                self.posted = Some((w.request.key.clone(), w.request.generation));
                self.mailbox.post(w.request);
            } else {
                self.wanted = Some(w);
            }
        }
        let mut shown = None;
        if let Some(rx) = &self.stills {
            for (generation, key, frame) in rx.try_iter() {
                if generation == self.generation && !self.is_playing() {
                    self.posted = None;
                    if let Some(frame) = frame {
                        self.shown_key = Some(key);
                        shown = Some(frame);
                    }
                }
            }
        }
        shown
    }

    /// Returns the frame to show now, if a newer one is due. Keeps the audio in sync with
    /// the wall clock and detects the end of playback.
    pub fn poll(&mut self, now: Instant) -> Option<Frame> {
        if let Ok(e) = self.errors.1.try_recv() {
            if e == HW_DECODE_FAILED {
                self.decoders.clear();
                self.hw_fallback = true;
                self.hw_fallbacks += 1;
            } else {
                self.error = Some(e);
                if self.is_playing() {
                    self.pause(now);
                }
            }
        }
        if self.hw_fallback {
            return None;
        }
        if !self.is_playing() {
            return self.poll_stills(now);
        }
        let pos = self.position(now);
        if let Some(buf) = &self.audio {
            let rate = self.sink.format().0;
            let wall = ((pos - self.start).0.max(0) as u128 * u128::from(rate) / 1_000_000) as u64;
            buf.correct(audio_correction(buf.position_frames(), wall, rate));
        }
        let mut shown = None;
        if let Some(f) = self.pending.take() {
            if f.pts <= pos {
                shown = Some(f);
            } else {
                self.pending = Some(f);
                return None;
            }
        }
        let mut ended = false;
        if let Some((_, rx)) = &self.playback {
            loop {
                match rx.try_recv() {
                    Ok(f) if f.pts <= pos => shown = Some(f),
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
        if ended || pos >= self.end {
            self.start = self.end;
            self.state = PlayState::Paused;
            self.stop_workers();
        }
        shown
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.mailbox.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg::CmdOutput;
    use crate::ffmpeg::fake::FakeRunner;
    use crate::jobs::{Priority, Scheduler};
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

    fn fake(frames: usize) -> FakeRunner {
        FakeRunner::default().on(move |_, a| {
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
        })
    }

    fn config_with(runner: FakeRunner) -> (Option<PreviewConfig>, Arc<FakeRunner>) {
        let fake = Arc::new(runner);
        let sched = Scheduler::new(3);
        let cfg = PreviewConfig {
            runner: sched.runner(Priority::Preview, fake.clone()),
            ffmpeg: "ffmpeg".into(),
            version: FfVersion::Release(6, 1, 0),
            encoders: ["libx264", "aac"].iter().map(|s| s.to_string()).collect(),
        };
        (Some(cfg), fake)
    }

    fn config(frames: usize) -> Option<PreviewConfig> {
        config_with(fake(frames)).0
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
        assert!(p.busy());
        // At t0 only the first frame (pts == start) is due.
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
        p.seek(&proj, Micros(520_000), t0).unwrap();
        // nothing before the quiet period, then the frame
        assert!(p.poll(t0).is_none());
        let later = t0 + STILL_DEBOUNCE;
        let f = poll_until(&mut p, later, |_, f| f.is_some()).unwrap();
        assert_eq!(f.pts, Micros(520_000));
        assert_eq!(p.still_renders(), 1);
        assert_eq!(p.position(t0), Micros(520_000));
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

    // covers: REQ-034
    #[test]
    fn still_frames_render_only_when_the_picture_changes() {
        let (cfg, fake) = config_with(fake(1));
        let mut p = Preview::new(cfg, Box::new(NullSink));
        let t0 = Instant::now();
        let mut proj = project();
        let m = proj.media[0].clone();
        proj.timeline.append(&m).unwrap();
        p.request_still(&proj, Micros(200_000), t0).unwrap();
        let due = t0 + STILL_DEBOUNCE;
        poll_until(&mut p, due, |_, f| f.is_some());
        assert_eq!(p.still_renders(), 1);
        // an edit after the playhead does not change the picture: no render
        let id = proj.timeline.clips[1].id;
        proj.timeline
            .trim(
                id,
                Micros(500_000),
                Micros(2_000_000),
                m.duration,
                Micros(1),
            )
            .unwrap();
        p.request_still(&proj, Micros(200_000), due).unwrap();
        assert!(!p.busy());
        for i in 0..5 {
            assert!(p.poll(due + Duration::from_secs(i)).is_none());
        }
        assert_eq!(p.still_renders(), 1);
        // 10 rapid edits that each change the picture: at most one render
        let mut now = due + Duration::from_secs(10);
        for i in 0..10 {
            let id = proj.timeline.clips[0].id;
            proj.timeline
                .trim(
                    id,
                    Micros(40_000 * (i + 1)),
                    Micros(2_000_000),
                    m.duration,
                    Micros(1),
                )
                .unwrap();
            p.request_still(&proj, Micros(200_000), now).unwrap();
            assert!(p.poll(now).is_none());
            now += Duration::from_millis(10);
        }
        let f = poll_until(&mut p, now + STILL_DEBOUNCE, |_, f| f.is_some()).unwrap();
        assert_eq!(f.pts, Micros(200_000));
        assert_eq!(p.still_renders(), 2);
        // the one render used one input
        let calls = fake.calls.lock().unwrap();
        let last = calls.last().unwrap();
        assert_eq!(last.iter().filter(|a| *a == "-i").count(), 1);
        assert!(last.iter().any(|a| a == "-frames:v"));
        drop(calls);
        // forgetting the frame forces a new render of the same picture
        p.forget_frame();
        p.request_still(&proj, Micros(200_000), now).unwrap();
        assert!(p.busy());
    }

    // covers: REQ-223
    #[test]
    fn a_change_of_scale_draws_the_picture_again_but_not_the_blurred_background() {
        use crate::frame::{Fill, Fit, FrameAspect, Scale, Shape};
        use crate::timeline::FrameTarget;
        let (cfg, fake) = config_with(fake(1));
        let mut p = Preview::new(cfg, Box::new(NullSink));
        let mut proj = project();
        proj.timeline
            .set_frame_aspect(Some(FrameAspect {
                shape: Shape::Tall9x16,
                fit: Fit::Height,
                fill: Fill::Blur,
                blur: Default::default(),
            }))
            .unwrap();
        proj.timeline.add_scale(FrameTarget::Whole).unwrap();
        let mut now = Instant::now();
        let still = |p: &mut Preview, proj: &Project, at: i64, now: &mut Instant| {
            p.request_still(proj, Micros(at), *now).unwrap();
            *now += STILL_DEBOUNCE;
            let f = poll_until(p, *now, |_, f| f.is_some()).unwrap();
            *now += Duration::from_millis(1);
            f
        };
        let scale = |proj: &mut Project, permille: u16| {
            proj.timeline
                .set_scale_key(
                    FrameTarget::Whole,
                    Micros::ZERO,
                    Scale {
                        permille,
                        dx: 0,
                        dy: 0,
                    },
                )
                .unwrap();
        };
        scale(&mut proj, 500);
        still(&mut p, &proj, 200_000, &mut now);
        assert_eq!((p.background_renders(), p.still_renders()), (1, 1));
        // ten changes of Scale at one playhead: the picture ten times, the background not
        for k in 1..=10 {
            scale(&mut proj, 500 + 50 * k as u16);
            still(&mut p, &proj, 200_000, &mut now);
        }
        assert_eq!(p.background_renders(), 1, "the background was drawn once");
        assert_eq!(p.still_renders(), 11, "the picture every time");
        // the run that drew the picture read the kept background and had no blur of its own
        let calls = fake.calls.lock().unwrap().clone();
        let last = calls.last().unwrap().join(" ");
        assert!(
            last.contains("-f rawvideo -pixel_format rgba -video_size"),
            "{last}"
        );
        assert!(last.contains("bg-") && !last.contains("boxblur"), "{last}");
        // the run that made it drew the background alone: no picture, no overlay
        let made = calls
            .iter()
            .find(|c| c.iter().any(|a| a.contains("boxblur")))
            .unwrap()
            .join(" ");
        assert!(!made.contains("overlay"), "{made}");
        // a move of the playhead to another source frame needs another background
        still(&mut p, &proj, 600_000, &mut now);
        assert_eq!(p.background_renders(), 2);
        // and so does another shape of the frame
        proj.timeline
            .set_frame_aspect(Some(FrameAspect {
                shape: Shape::Wide4x3,
                fit: Fit::Height,
                fill: Fill::Blur,
                blur: Default::default(),
            }))
            .unwrap();
        still(&mut p, &proj, 600_000, &mut now);
        assert_eq!(p.background_renders(), 3);
        // one file at a time in the still folder
        let kept: Vec<_> = std::fs::read_dir(p.dir("still"))
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("bg-"))
            .collect();
        assert_eq!(kept.len(), 1);
        // a plain colour fill keeps nothing: every still is one run
        proj.timeline
            .set_frame_aspect(Some(FrameAspect {
                shape: Shape::Wide4x3,
                fit: Fit::Height,
                fill: Fill::Colour(crate::frame::Rgb(0, 0, 0)),
                blur: Default::default(),
            }))
            .unwrap();
        let before = p.background_renders();
        still(&mut p, &proj, 900_000, &mut now);
        assert_eq!(p.background_renders(), before);
    }

    /// The key of the picture at `at`, as the preview computes it.
    fn key_at(p: &Preview, proj: &Project, at: Micros) -> FrameKey {
        let cfg = p.config.clone().unwrap();
        let eg = render::output_geometry(proj, &cfg.encoders).unwrap();
        render::frame_key(proj, at, p.source(), &eg).unwrap()
    }

    // covers: REQ-144
    #[test]
    fn a_still_is_rendered_again_after_a_superseded_request() {
        let mut p = Preview::new(config(1), Box::new(NullSink));
        let t0 = Instant::now();
        let proj = project();
        // K0 is shown
        p.request_still(&proj, Micros::ZERO, t0).unwrap();
        poll_until(&mut p, t0 + STILL_DEBOUNCE, |p, _| p.shown_key.is_some());
        // K1 is posted, then K2 supersedes it, then K0 (shown) is asked again
        let due = t0 + Duration::from_secs(1);
        p.request_still(&proj, Micros(400_000), due).unwrap();
        p.poll(due + STILL_DEBOUNCE);
        p.request_still(&proj, Micros(800_000), due).unwrap();
        p.request_still(&proj, Micros::ZERO, due).unwrap();
        // the old K1 render may finish now and is dropped; asking K1 again must render it
        std::thread::sleep(Duration::from_millis(100));
        p.request_still(&proj, Micros(400_000), due).unwrap();
        let k1 = key_at(&p, &proj, Micros(400_000));
        let f = poll_until(&mut p, due + STILL_DEBOUNCE * 2, |p, _| {
            p.shown_key.as_ref() == Some(&k1)
        })
        .unwrap();
        assert_eq!(f.pts, Micros(400_000));
    }

    // covers: REQ-144
    #[test]
    fn a_still_is_rendered_after_playing_and_pausing_on_a_posted_key() {
        let mut p = Preview::new(config(1), Box::new(NullSink));
        let t0 = Instant::now();
        let proj = project();
        // a render for 1 s is posted, then playback starts before its result is taken
        p.request_still(&proj, Micros(1_000_000), t0).unwrap();
        p.poll(t0 + STILL_DEBOUNCE);
        p.play(&proj, Micros(1_000_000), t0).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        p.pause(t0);
        p.request_still(&proj, Micros(1_000_000), t0).unwrap();
        let k = key_at(&p, &proj, Micros(1_000_000));
        let later = t0 + STILL_DEBOUNCE * 2;
        poll_until(&mut p, later, |p, _| p.shown_key.as_ref() == Some(&k));
    }

    // covers: REQ-035
    #[test]
    fn video_follows_the_wall_clock_when_audio_stalls() {
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
        assert_eq!(buf.len(), 9_600, "0.1 s of stereo audio decoded");
        // The audio device stalls (nothing is pulled) for 1.2 s: the video clock goes on.
        let t1 = t0 + Duration::from_millis(1_200);
        assert_eq!(p.position(t1), Micros(1_200_000));
        // When polled, the audio is moved to the wall clock: the backlog is dropped.
        let _ = p.poll(t1);
        let wall = 48_000 * 12 / 10;
        let drift = buf.position_frames().abs_diff(wall);
        assert!(drift <= 4_800, "drift {drift} frames > 100 ms");
        assert!(buf.is_empty());
        // The device resumes: it plays from the corrected position.
        let mut out = vec![0i16; 960];
        buf.pull(&mut out, |s| s);
        assert_eq!(buf.device_frames(), 480);
        assert!(buf.position_frames().abs_diff(wall) <= 4_800);
        p.pause(t1);
    }

    // covers: REQ-035
    #[test]
    fn audio_correction_drops_or_pads() {
        assert_eq!(audio_correction(48_000, 48_000, 48_000), Correction::None);
        assert_eq!(audio_correction(48_000, 49_000, 48_000), Correction::None);
        assert_eq!(
            audio_correction(0, 48_000, 48_000),
            Correction::Drop(48_000)
        );
        assert_eq!(
            audio_correction(10_000, 4_000, 48_000),
            Correction::Pad(6_000)
        );
        // padding plays silence first, then the samples
        let b = AudioBuffer::new(2);
        b.push(&[5, 5, 6, 6]);
        b.correct(Correction::Pad(1));
        assert_eq!(b.position_frames(), 0);
        let mut out = vec![9i16; 6];
        b.pull(&mut out, |s| s);
        assert_eq!(out, vec![0, 0, 5, 5, 6, 6]);
        assert_eq!(b.position_frames(), 2);
        // a drop larger than the buffer is paid by the next data
        b.push(&[1, 1]);
        b.correct(Correction::Drop(3));
        assert!(b.is_empty());
        assert_eq!(b.position_frames(), 5);
        b.push(&[2, 2, 3, 3, 4, 4]);
        assert_eq!(b.len(), 2);
        let mut out = vec![0i16; 2];
        b.pull(&mut out, |s| s);
        assert_eq!(out, vec![4, 4]);
        b.correct(Correction::None);
        // starved: silence, the stream position does not move
        let mut out = vec![9i16; 4];
        b.pull(&mut out, |s| s);
        assert_eq!(out, vec![0; 4]);
        assert_eq!(b.position_frames(), 6);
        assert_eq!(AudioBuffer::default().channels, 2);
    }

    // covers: REQ-039
    #[test]
    fn buffers_are_bounded_whatever_the_source_size() {
        // frames in flight: FRAME_BUFFER frames of at most 360p preview size
        let g = Geometry {
            width: 7680,
            height: 4320,
            fps: Fps::new(30, 1).unwrap(),
        }
        .preview();
        let frame_bytes = (g.width * g.height * 4) as usize;
        assert!(FRAME_BUFFER * frame_bytes < 32 * 1024 * 1024);
        // audio buffer: about one second
        assert_eq!(AudioBuffer::CAP, 96_000);
        assert_eq!(crate::ffmpeg::RUN_STDOUT_LIMIT, 16 * 1024 * 1024);
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
        // a sink that refuses to play: silent, wall clock
        p.play(&project(), Micros::ZERO, t0).unwrap();
        assert_eq!(p.position(t0 + Duration::from_millis(250)), Micros(250_000));
        let n = NullSink;
        assert_eq!(n.format(), (48_000, 2));
        let _ = default_sink().format();
        // a missing source: the still frame cannot be prepared
        let mut proj = project();
        proj.media[0].missing = true;
        let mut p = Preview::new(config(1), Box::new(NullSink));
        assert!(p.request_still(&proj, Micros(10), t0).is_err());
        // ffmpeg that cannot be started: the error reaches the UI and playback stops
        let (cfg, _) = config_with(FakeRunner::default());
        let mut p = Preview::new(cfg, Box::new(NullSink));
        p.play(&project(), Micros::ZERO, t0).unwrap();
        poll_until(&mut p, t0, |p, _| p.error.is_some());
        assert!(!p.is_playing());
    }
}
