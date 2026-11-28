// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The picture strips of the timeline (3.2.0, REQ-369, REQ-370, ADR-277): snapshots of the
//! source picture in the lower half of each clip, from the clip's start, spaced to fit the
//! zoom. Effects are not drawn.
//!
//! * [`box_size`], [`grid_step`] and [`slots`] decide what to show (pure).
//! * The window tells which snapshots are visible ([`Strips::want`]); [`Strips::tick`]
//!   makes the missing ones, one ffmpeg at a time with the lowest priority, and keeps them
//!   in memory only (at most [`CACHE_CAP`] bytes, the least recently shown go first).

use crate::base::steps::step;
use crate::ffmpeg::{Process, Runner, argv, file_arg};
use crate::jobs::Limited;
use crate::media::MediaId;
use crate::time::Micros;
use crate::timeline::Clip;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Space between two snapshots, at least.
pub const GAP: f32 = 4.0;
/// The height of a snapshot: at least, at most, and the step it grows by.
pub const MIN_HEIGHT: u32 = 8;
pub const MAX_HEIGHT: u32 = 64;
const HEIGHT_STEP: u32 = 4;
/// The widest snapshot.
pub const MAX_WIDTH: u32 = 256;
/// Most snapshots of one clip in one frame of the window.
pub const MAX_SLOTS: usize = 256;
/// Most bytes of snapshots kept in memory.
pub const CACHE_CAP: usize = 32 * 1024 * 1024;
/// Up to this source step the snapshots are read in one run; further apart each is sought.
pub const BATCH_MAX_STEP: Micros = Micros(2_000_000);
/// Most snapshots of one run.
pub const MAX_BATCH: usize = 64;
/// A run with no result for this long is stopped.
pub const JOB_TIMEOUT: Duration = Duration::from_secs(60);

/// The size of the snapshots for a clip whose lower half is `lower` points high and whose
/// picture is `width` x `height`; `None` when there is no room.
pub fn box_size(lower: f32, width: u32, height: u32) -> Option<(u32, u32)> {
    if !lower.is_finite() || lower < MIN_HEIGHT as f32 || width == 0 || height == 0 {
        return None;
    }
    let h = (lower.min(MAX_HEIGHT as f32) as u32) / HEIGHT_STEP * HEIGHT_STEP;
    let w = (u64::from(h) * u64::from(width)).div_ceil(u64::from(height));
    let w = (w.min(u64::from(MAX_WIDTH)) as u32).max(MIN_HEIGHT);
    Some((w + (w & 1), h))
}

/// The timeline step (seconds) between snapshots `width` wide at `zoom` points per second:
/// the smallest ruler step that leaves them [`GAP`] apart.
pub fn grid_step(zoom: f32, width: u32) -> Option<u64> {
    if !(zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    (0..64)
        .map(step)
        .find(|s| *s as f32 * zoom >= width as f32 + GAP)
}

/// One place of a snapshot in a clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// From the start of the clip, in points.
    pub x: f32,
    /// The time in the source.
    pub at: Micros,
}

/// The places of the snapshots of `clip` that show in `visible` (points from the clip's
/// start), and the source time between two of them. A snapshot that does not fit in the
/// clip has no place.
pub fn slots(clip: &Clip, zoom: f32, width: u32, visible: (f32, f32)) -> (Vec<Slot>, Micros) {
    let Some(seconds) = grid_step(zoom, width) else {
        return (Vec::new(), Micros::ZERO);
    };
    let every = Micros((seconds as i64).saturating_mul(1_000_000));
    let src_step = clip.to_src(every);
    let spacing = seconds as f32 * zoom;
    // a thousandth of a point forgives the rounding of the lengths
    let len = (clip.len().as_secs_f64() * f64::from(zoom)) as f32 + 0.001;
    let first = ((visible.0 - width as f32).max(0.0) / spacing).floor() as i64;
    let mut out = Vec::new();
    for k in first.. {
        let x = k as f32 * spacing;
        if x > visible.1 || x + width as f32 > len || out.len() >= MAX_SLOTS {
            break;
        }
        let at = clip.src_in + Micros(src_step.0.saturating_mul(k));
        if at >= clip.src_out {
            break;
        }
        out.push(Slot { x, at });
    }
    (out, src_step)
}

/// Which snapshot: of which media, at which source time, in which size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub media: MediaId,
    pub at: Micros,
    pub width: u32,
    pub height: u32,
}

/// A snapshot the window shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Want {
    pub key: Key,
    /// The source time between this snapshot and the next one of its clip.
    pub step: Micros,
    /// The file to read: the preview copy when there is one.
    pub path: PathBuf,
}

/// A made snapshot: `width * height` RGBA pixels.
#[derive(Debug, PartialEq)]
pub struct Thumb {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The ffmpeg arguments that read `count` snapshots of `want`'s file, the first at its
/// time and the others `step` apart.
pub fn args(want: &Want, count: usize) -> Vec<OsString> {
    let (w, h) = (want.key.width, want.key.height);
    let mut a = argv([
        "-hide_banner",
        "-nostdin",
        "-loglevel",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-threads",
        "1",
        "-ss",
    ]);
    a.push(want.key.at.ff().into());
    a.push("-i".into());
    a.push(file_arg(&want.path));
    let rate = if count > 1 {
        format!("fps=1000000/{}:round=up,", want.step.0.max(1))
    } else {
        String::new()
    };
    a.extend(argv([
        "-an".to_string(),
        "-sn".to_string(),
        "-dn".to_string(),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-vf".to_string(),
        format!(
            "{rate}scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"
        ),
        "-frames:v".to_string(),
        count.to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pix_fmt".to_string(),
        "rgba".to_string(),
        "pipe:1".to_string(),
    ]));
    a
}

enum Message {
    Thumb(Key, Thumb),
    /// The run ended; `true` when it was stopped for a more urgent job.
    End(bool),
}

type Child = Arc<Mutex<Option<Box<dyn Process>>>>;

struct Job {
    keys: Vec<Key>,
    rx: Receiver<Message>,
    cancel: Arc<AtomicBool>,
    child: Child,
    started: Instant,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(p) = lock(&self.child).as_mut() {
            p.kill();
        }
    }
}

struct Entry {
    thumb: Arc<Thumb>,
    used: u64,
}

/// The snapshots in memory and the run that makes more.
#[derive(Default)]
pub struct Strips {
    wanted: Mutex<Vec<Want>>,
    cache: HashMap<Key, Entry>,
    bytes: usize,
    clock: u64,
    /// Snapshots that could not be made (not tried again).
    failed: HashSet<Key>,
    job: Option<Job>,
    /// Runs started so far.
    pub started: u64,
}

impl Strips {
    /// The window starts a new frame: nothing is wanted yet.
    pub fn begin_frame(&self) {
        lock(&self.wanted).clear();
    }

    /// The window shows this snapshot.
    pub fn want(&self, w: Want) {
        lock(&self.wanted).push(w);
    }

    /// What the window shows.
    pub fn wanted(&self) -> Vec<Want> {
        lock(&self.wanted).clone()
    }

    pub fn get(&self, key: &Key) -> Option<Arc<Thumb>> {
        self.cache.get(key).map(|e| Arc::clone(&e.thumb))
    }

    /// Bytes of the snapshots kept.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// A run is going on, or a shown snapshot is still to be made.
    pub fn busy(&self) -> bool {
        self.job.is_some()
            || lock(&self.wanted)
                .iter()
                .any(|w| !self.cache.contains_key(&w.key) && !self.failed.contains(&w.key))
    }

    /// Stops the run and forgets everything (another project, or the strips are off).
    pub fn clear(&mut self) {
        self.job = None;
        self.cache.clear();
        self.failed.clear();
        self.bytes = 0;
        lock(&self.wanted).clear();
    }

    /// The file of media `id` changed: its snapshots are made again.
    pub fn forget(&mut self, id: MediaId) {
        if self
            .job
            .as_ref()
            .is_some_and(|j| j.keys.iter().any(|k| k.media == id))
        {
            self.job = None;
        }
        self.cache.retain(|k, _| k.media != id);
        self.failed.retain(|k| k.media != id);
        self.bytes = self.cache.values().map(|e| e.thumb.rgba.len()).sum();
        lock(&self.wanted).retain(|w| w.key.media != id);
    }

    /// Puts a snapshot in the cache, making room for it.
    pub fn insert(&mut self, key: Key, thumb: Thumb) {
        self.clock += 1;
        self.bytes += thumb.rgba.len();
        let entry = Entry {
            thumb: Arc::new(thumb),
            used: self.clock,
        };
        if let Some(old) = self.cache.insert(key, entry) {
            self.bytes -= old.thumb.rgba.len();
        }
        while self.bytes > CACHE_CAP {
            let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k)
            else {
                break;
            };
            if let Some(e) = self.cache.remove(&oldest) {
                self.bytes -= e.thumb.rgba.len();
            }
        }
    }

    /// Takes the results of the run, stops a run nobody waits for and starts the next one.
    pub fn tick(&mut self, now: Instant, runner: &Limited, ffmpeg: &Path) {
        self.receive();
        let wanted = self.wanted();
        self.clock += 1;
        for w in &wanted {
            if let Some(e) = self.cache.get_mut(&w.key) {
                e.used = self.clock;
            }
        }
        if let Some(job) = &self.job {
            let needed = job
                .keys
                .iter()
                .any(|k| !self.cache.contains_key(k) && wanted.iter().any(|w| w.key == *k));
            let late = now.saturating_duration_since(job.started) > JOB_TIMEOUT;
            if late {
                self.failed.extend(job.keys.iter().copied());
            }
            if late || !needed {
                self.job = None;
            }
        }
        if self.job.is_none() {
            let missing =
                |w: &&Want| !self.cache.contains_key(&w.key) && !self.failed.contains(&w.key);
            if let Some(first) = wanted.iter().find(missing) {
                let batch = batch_of(first, wanted.iter().filter(missing));
                self.job = Some(start(first.clone(), batch, now, runner, ffmpeg));
                self.started += 1;
            }
        }
    }

    fn receive(&mut self) {
        let Some(job) = self.job.take() else { return };
        loop {
            match job.rx.try_recv() {
                Ok(Message::Thumb(k, t)) => self.insert(k, t),
                Ok(Message::End(preempted)) => {
                    if !preempted {
                        self.failed.extend(
                            job.keys
                                .iter()
                                .filter(|k| !self.cache.contains_key(k))
                                .copied(),
                        );
                    }
                    return;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.failed.extend(job.keys.iter().copied());
                    return;
                }
            }
        }
        self.job = Some(job);
    }
}

/// The snapshots one run makes, starting with `first`: those that follow it one step apart
/// in the same file and size, when the step is short.
fn batch_of<'a>(first: &Want, missing: impl Iterator<Item = &'a Want>) -> Vec<Key> {
    let mut keys = vec![first.key];
    if first.step <= Micros::ZERO || first.step > BATCH_MAX_STEP {
        return keys;
    }
    let others: HashSet<Key> = missing.map(|w| w.key).collect();
    while keys.len() < MAX_BATCH {
        let next = Key {
            at: first.key.at + Micros(first.step.0.saturating_mul(keys.len() as i64)),
            ..first.key
        };
        if !others.contains(&next) {
            break;
        }
        keys.push(next);
    }
    keys
}

fn start(first: Want, keys: Vec<Key>, now: Instant, runner: &Limited, ffmpeg: &Path) -> Job {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let child: Child = Arc::default();
    let preempt = Arc::new(AtomicBool::new(false));
    let runner = runner
        .cancellable(Arc::clone(&cancel))
        .preemptible(Arc::clone(&preempt));
    let ffmpeg = ffmpeg.to_path_buf();
    let run = Run {
        args: args(&first, keys.len()),
        keys: keys.clone(),
        cancel: Arc::clone(&cancel),
        preempt,
        child: Arc::clone(&child),
        tx,
    };
    std::thread::spawn(move || run.work(&runner, &ffmpeg));
    Job {
        keys,
        rx,
        cancel,
        child,
        started: now,
    }
}

struct Run {
    args: Vec<OsString>,
    keys: Vec<Key>,
    cancel: Arc<AtomicBool>,
    preempt: Arc<AtomicBool>,
    child: Child,
    tx: Sender<Message>,
}

impl Run {
    fn stopped(&self) -> bool {
        self.cancel.load(Ordering::SeqCst) || self.preempt.load(Ordering::SeqCst)
    }

    fn work(self, runner: &Limited, ffmpeg: &Path) {
        if let Ok(mut p) = runner.spawn(ffmpeg, &self.args) {
            let out = p.take_stdout();
            *lock(&self.child) = Some(p);
            if let Some(mut out) = out {
                for key in &self.keys {
                    if self.stopped() {
                        break;
                    }
                    let mut rgba = vec![0; key.width as usize * key.height as usize * 4];
                    if out.read_exact(&mut rgba).is_err() {
                        break;
                    }
                    let thumb = Thumb {
                        width: key.width,
                        height: key.height,
                        rgba,
                    };
                    if self.tx.send(Message::Thumb(*key, thumb)).is_err() {
                        break;
                    }
                }
            }
            // ends the process and frees its slot
            if let Some(mut p) = lock(&self.child).take() {
                p.kill();
            }
        }
        let _ = self
            .tx
            .send(Message::End(self.preempt.load(Ordering::SeqCst)));
    }
}

#[cfg(test)]
pub(crate) mod tests;
