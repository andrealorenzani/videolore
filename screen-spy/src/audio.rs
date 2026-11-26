// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The recorder's sound: computer sound and microphone (REQ-357, ADR-259, 260).
//!
//! On Linux ffmpeg reads both from PulseAudio itself; this module only checks that they
//! open. On Windows and macOS the two are captured here (`cpal`), turned into 16 bit
//! stereo and sent to ffmpeg over one connection each on this machine; while a source
//! sends nothing (a silent output sends nothing at all) silence fills its time, so that
//! the track keeps the length of the recording. Nothing is written to disk and only a
//! peer on this machine is accepted.

use crate::devices::{microphone_problem, parse_pulse_sources};
use crate::plan::{AudioInput, PulseSource};
use std::any::Any;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use videolore::ffmpeg::{Runner, argv};

/// How long a source may take to open before it counts as missing.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// How far a source may fall behind the clock before silence fills the gap.
pub const SILENCE_MARGIN: Duration = Duration::from_millis(200);
/// How often the feeder looks at the clock while a source is quiet.
const TICK: Duration = Duration::from_millis(20);
/// How long a write to ffmpeg may block before the connection counts as broken.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// The channels sent to ffmpeg.
pub const CHANNELS: u16 = 2;

/// The two open sources of one recording. Dropping it closes them.
pub struct Session {
    /// What ffmpeg reads, in the order of its inputs.
    pub inputs: Vec<AudioInput>,
    /// What lives as long as the recording.
    pub keep: Vec<Box<dyn Any + Send>>,
}

/// Opens the sound of a recording (faked in tests). It is called on a thread of its own,
/// never on the window's: opening a source, and asking for a permission, can take long.
pub trait Sound: Send + Sync {
    /// Both sources, or the message naming the one that cannot be recorded (REQ-357:
    /// never a recording of one source only). `cancel` is set when nobody waits any more.
    fn open(&self, ffmpeg: &Path, cancel: &AtomicBool) -> Result<Session, String>;
}

/// The message of a start that was cancelled (nobody reads it).
pub const CANCELLED: &str = "The start was cancelled";

/// The thread that owns open sound streams; dropping it closes them.
struct Holder {
    release: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Holder {
    fn drop(&mut self) {
        self.release = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Runs `open` on a new thread, which keeps what it opened until the session is dropped
/// and closes it there: a sound stream never changes threads (Windows ties it to the
/// thread that opened it).
pub fn hold<T: 'static>(
    open: impl FnOnce() -> Result<(Vec<AudioInput>, T), String> + Send + 'static,
) -> Result<Session, String> {
    let (opened, result) = channel();
    let (release, released) = channel::<()>();
    let thread = std::thread::spawn(move || match open() {
        Ok((inputs, kept)) => {
            let _ = opened.send(Ok(inputs));
            // returns when the sender is dropped
            let _ = released.recv();
            drop(kept);
        }
        Err(e) => {
            let _ = opened.send(Err(e));
        }
    });
    let holder = Holder {
        release: Some(release),
        thread: Some(thread),
    };
    let inputs = result
        .recv()
        .map_err(|_| "The sound could not be opened".to_string())??;
    Ok(Session {
        inputs,
        keep: vec![Box::new(holder)],
    })
}

/// The message for a source that cannot be recorded.
pub fn source_error(source: PulseSource, detail: &str) -> String {
    let name = match source {
        PulseSource::Computer => "Computer sound",
        PulseSource::Microphone => "The microphone",
    };
    let detail = detail
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().chars().take(200).collect::<String>());
    match detail {
        Some(d) => format!("{name} is not available: {d}. Nothing was recorded."),
        None => format!("{name} is not available. Nothing was recorded."),
    }
}

/// Linux: ffmpeg's own PulseAudio inputs, each tried for a tenth of a second.
pub struct PulseSound {
    pub runner: Arc<dyn Runner>,
}

impl PulseSound {
    fn probe(&self, ffmpeg: &Path, source: PulseSource) -> Result<(), String> {
        let args = argv([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "pulse",
            "-i",
            source.device(),
            "-t",
            "0.1",
            "-f",
            "null",
            "-",
        ]);
        match self.runner.run_within(ffmpeg, &args, PROBE_TIMEOUT) {
            Ok(o) if o.success => Ok(()),
            Ok(o) => Err(source_error(source, &o.stderr)),
            Err(e) => Err(source_error(source, &e.to_string())),
        }
    }
}

impl Sound for PulseSound {
    fn open(&self, ffmpeg: &Path, cancel: &AtomicBool) -> Result<Session, String> {
        let list = argv(["-hide_banner", "-sources", "pulse"]);
        if let Ok(o) = self.runner.run_within(ffmpeg, &list, PROBE_TIMEOUT)
            && let Some(why) =
                microphone_problem(&parse_pulse_sources(&String::from_utf8_lossy(&o.stdout)))
        {
            return Err(source_error(PulseSource::Microphone, why));
        }
        let mut inputs = Vec::new();
        for source in [PulseSource::Computer, PulseSource::Microphone] {
            if cancel.load(Ordering::SeqCst) {
                return Err(CANCELLED.into());
            }
            self.probe(ffmpeg, source)?;
            inputs.push(AudioInput::Pulse(source));
        }
        Ok(Session {
            inputs,
            keep: Vec::new(),
        })
    }
}

/// A sample of any of the kinds a sound device gives, as 16 bit.
pub trait Sample: Copy {
    fn to_i16(self) -> i16;
}

impl Sample for i16 {
    fn to_i16(self) -> i16 {
        self
    }
}

impl Sample for u16 {
    fn to_i16(self) -> i16 {
        (i32::from(self) - 32768) as i16
    }
}

impl Sample for i32 {
    fn to_i16(self) -> i16 {
        (self >> 16) as i16
    }
}

impl Sample for f32 {
    fn to_i16(self) -> i16 {
        // a cast saturates, and turns a value that is not a number into 0
        (self * 32767.0).round() as i16
    }
}

/// Appends the frames of `input` (`channels` samples each) to `out` as little endian
/// 16 bit stereo: mono goes to both sides; of more than two channels the front pair is
/// kept, with the centre (the third) added to both.
pub fn to_stereo_s16le<T: Sample>(input: &[T], channels: u16, out: &mut Vec<u8>) {
    let n = usize::from(channels.max(1));
    out.reserve(input.len() / n * 4);
    for frame in input.chunks_exact(n) {
        let first = frame[0].to_i16();
        let (mut left, mut right) = (first, frame.get(1).map_or(first, |s| s.to_i16()));
        if let Some(centre) = frame.get(2) {
            let c = (i32::from(centre.to_i16()) * 707 / 1000) as i16;
            (left, right) = (left.saturating_add(c), right.saturating_add(c));
        }
        out.extend_from_slice(&left.to_le_bytes());
        out.extend_from_slice(&right.to_le_bytes());
    }
}

/// How many frames of silence are due: none while the source is at most
/// [`SILENCE_MARGIN`] behind the clock; beyond that, enough to be half a margin behind.
pub fn silence_due(elapsed: Duration, rate: u32, written: u64) -> u64 {
    let frames = |d: Duration| (d.as_micros() * u128::from(rate) / 1_000_000) as u64;
    let behind = frames(elapsed).saturating_sub(written);
    if behind > frames(SILENCE_MARGIN) {
        behind - frames(SILENCE_MARGIN) / 2
    } else {
        0
    }
}

/// Only this machine may receive the sound.
pub fn allowed_peer(peer: &SocketAddr) -> bool {
    peer.ip() == IpAddr::V4(Ipv4Addr::LOCALHOST)
}

/// Sends one source to ffmpeg. Dropping it ends the connection.
pub struct Feeder {
    pub port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Feeder {
    /// Listens on this machine, on a free port, for the one connection of ffmpeg;
    /// `pcm` brings 16 bit stereo at `rate` samples per second.
    pub fn start(pcm: Receiver<Vec<u8>>, rate: u32) -> io::Result<Feeder> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            if let Some(peer) = accept(listener, &flag) {
                // what was captured before ffmpeg listened is not part of the file
                while pcm.try_recv().is_ok() {}
                send(peer, &pcm, rate, &flag);
            }
        });
        Ok(Feeder {
            port,
            stop,
            thread: Some(thread),
        })
    }

    /// The input ffmpeg reads this source from.
    pub fn input(&self, rate: u32) -> AudioInput {
        AudioInput::Tcp {
            port: self.port,
            rate,
            channels: CHANNELS,
        }
    }
}

impl Drop for Feeder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The first connection from this machine; the listener closes with it.
fn accept(listener: TcpListener, stop: &AtomicBool) -> Option<TcpStream> {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) if allowed_peer(&peer) => {
                stream.set_nonblocking(false).ok()?;
                stream.set_write_timeout(Some(WRITE_TIMEOUT)).ok()?;
                return Some(stream);
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(TICK),
            Err(_) => return None,
        }
    }
    None
}

/// Writes the sound as it comes, and silence when it does not, until told to stop or the
/// other side is gone.
fn send(mut to: impl Write, pcm: &Receiver<Vec<u8>>, rate: u32, stop: &AtomicBool) {
    let start = Instant::now();
    let mut written = 0u64;
    let frame = usize::from(CHANNELS) * 2;
    while !stop.load(Ordering::SeqCst) {
        let data = match pcm.recv_timeout(TICK) {
            Ok(d) => d,
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            // the stream is gone (a device unplugged): silence from here on
            Err(RecvTimeoutError::Disconnected) => {
                std::thread::sleep(TICK);
                Vec::new()
            }
        };
        written += (data.len() / frame) as u64;
        if to.write_all(&data).is_err() {
            return;
        }
        let silence = silence_due(start.elapsed(), rate, written);
        if silence > 0 {
            written += silence;
            if to.write_all(&vec![0; silence as usize * frame]).is_err() {
                return;
            }
        }
    }
}

#[cfg(any(windows, target_os = "macos"))]
mod device {
    //! The two `cpal` streams (thin glue, checked by hand: test machines have no sound
    //! devices). On Windows the computer's sound is the default output opened as an input
    //! (WASAPI loopback). On macOS 14.6 or later (the recorder never gets here on an older
    //! one) it is the input of a device made around a tap of everything the computer plays
    //! (`crate::macos::SystemTap`): `cpal`'s own loopback would record the inputs of an
    //! output device that has some.
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{SampleFormat, SizedSample};
    use std::sync::mpsc::{SyncSender, sync_channel};

    /// Blocks of samples that may wait for the feeder.
    const QUEUE: usize = 256;

    pub struct DeviceSound;

    /// One open source; the fields are dropped in this order.
    struct Open {
        _stream: cpal::Stream,
        _feeder: Feeder,
        #[cfg(target_os = "macos")]
        _tap: Option<crate::macos::SystemTap>,
    }

    fn build<T: SizedSample + Sample + 'static>(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        tx: SyncSender<Vec<u8>>,
    ) -> Result<cpal::Stream, cpal::Error> {
        let channels = config.channels;
        device.build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                let mut pcm = Vec::new();
                to_stereo_s16le(data, channels, &mut pcm);
                // the sound thread never waits: a block that does not fit is dropped
                let _ = tx.try_send(pcm);
            },
            |_| {},
            None,
        )
    }

    fn open_one(source: PulseSource) -> Result<(AudioInput, Open), String> {
        let fail = |e: &str| source_error(source, e);
        let host = cpal::default_host();
        #[cfg(target_os = "macos")]
        let mut tap = None;
        let (device, config) = match source {
            PulseSource::Microphone => {
                let d = host
                    .default_input_device()
                    .ok_or_else(|| fail("no microphone was found"))?;
                let c = d.default_input_config();
                (d, c)
            }
            #[cfg(windows)]
            PulseSource::Computer => {
                let d = host
                    .default_output_device()
                    .ok_or_else(|| fail("no sound output was found"))?;
                let c = d.default_output_config();
                (d, c)
            }
            #[cfg(target_os = "macos")]
            PulseSource::Computer => {
                let made = crate::macos::SystemTap::new().map_err(|e| fail(&e))?;
                let id = cpal::DeviceId::new(host.id(), &made.uid);
                // the new device may take a moment to be listed
                let mut found = host.device_by_id(&id);
                for _ in 0..20 {
                    if found.is_some() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    found = host.device_by_id(&id);
                }
                let d = found.ok_or_else(|| fail("its capture device did not appear"))?;
                let c = d.default_input_config();
                tap = Some(made);
                (d, c)
            }
        };
        let config = config.map_err(|e| fail(&e.to_string()))?;
        let (format, config) = (config.sample_format(), config.config());
        let (tx, rx) = sync_channel(QUEUE);
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, config, tx),
            SampleFormat::I16 => build::<i16>(&device, config, tx),
            SampleFormat::U16 => build::<u16>(&device, config, tx),
            SampleFormat::I32 => build::<i32>(&device, config, tx),
            other => return Err(fail(&format!("unsupported sample format {other}"))),
        }
        .map_err(|e| fail(&e.to_string()))?;
        stream.play().map_err(|e| fail(&e.to_string()))?;
        let feeder = Feeder::start(rx, config.sample_rate).map_err(|e| fail(&e.to_string()))?;
        Ok((
            feeder.input(config.sample_rate),
            Open {
                _stream: stream,
                _feeder: feeder,
                #[cfg(target_os = "macos")]
                _tap: tap,
            },
        ))
    }

    impl Sound for DeviceSound {
        fn open(&self, _ffmpeg: &Path, cancel: &AtomicBool) -> Result<Session, String> {
            // a permission not asked yet is asked for and answered before anything is
            // recorded: until then macOS gives silence
            #[cfg(target_os = "macos")]
            crate::session::microphone_gate(
                &crate::macos::Microphone,
                crate::session::PERMISSION_WAIT,
                cancel,
            )?;
            if cancel.load(Ordering::SeqCst) {
                return Err(CANCELLED.into());
            }
            hold(|| {
                let mut inputs = Vec::new();
                let mut kept = Vec::new();
                for source in [PulseSource::Microphone, PulseSource::Computer] {
                    let (input, open) = open_one(source)?;
                    inputs.push(input);
                    kept.push(open);
                }
                Ok((inputs, kept))
            })
        }
    }
}

/// The sound of this machine.
#[cfg(any(windows, target_os = "macos"))]
pub fn system_sound(_runner: Arc<dyn Runner>) -> Arc<dyn Sound> {
    Arc::new(device::DeviceSound)
}

/// The sound of this machine.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn system_sound(runner: Arc<dyn Runner>) -> Arc<dyn Sound> {
    Arc::new(PulseSound { runner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;

    /// Collects what is written and reports a broken pipe after `limit` bytes.
    struct Cut {
        got: Vec<u8>,
        limit: usize,
    }

    impl Write for Cut {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self.got.len() >= self.limit {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            self.got.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // covers: REQ-357
    #[test]
    fn a_quiet_or_unplugged_source_is_filled_with_silence_until_the_other_side_is_gone() {
        let (tx, rx) = sync_channel(8);
        tx.send(vec![7u8; 400]).unwrap();
        let mut to = Cut {
            got: Vec::new(),
            limit: 48_000,
        };
        let start = Instant::now();
        // nothing more comes, then the stream is gone: the feeder still keeps time
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            drop(tx);
        });
        send(&mut to, &rx, 48_000, &AtomicBool::new(false));
        t.join().unwrap();
        assert!(start.elapsed() >= SILENCE_MARGIN);
        assert_eq!(&to.got[..400], &[7u8; 400][..]);
        assert!(to.got.len() >= 48_000 && to.got[400..].iter().all(|b| *b == 0));
        assert_eq!(to.got.len() % 4, 0, "whole frames only");
        // a broken connection while sound arrives ends it too
        let (tx, rx) = sync_channel(8);
        tx.send(vec![1u8; 4]).unwrap();
        let mut gone = Cut {
            got: Vec::new(),
            limit: 0,
        };
        send(&mut gone, &rx, 48_000, &AtomicBool::new(false));
        // told to stop: nothing is written
        let mut none = Cut {
            got: Vec::new(),
            limit: 10,
        };
        send(&mut none, &rx, 48_000, &AtomicBool::new(true));
        assert!(none.got.is_empty());
    }
}
