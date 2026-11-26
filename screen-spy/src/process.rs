// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The capture process: the only place of the recorder that starts a long running
//! ffmpeg (ADR-257, 261). Arguments are always a list, never a shell command.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;

/// How long ffmpeg may take to close the file after Stop, before it is killed.
pub const STOP_TIMEOUT: Duration = Duration::from_secs(15);
/// The part of ffmpeg's error output that is kept, in bytes.
const STDERR_KEPT: usize = 8 * 1024;

/// How a capture process ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exit {
    pub success: bool,
    /// The end of the error output.
    pub stderr: String,
}

/// A running capture process.
pub trait Child {
    /// Takes the standard output (once).
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>>;
    /// Asks ffmpeg to end its files and stop (`q` on its standard input).
    fn quit(&mut self);
    /// How it ended, once it has.
    fn try_wait(&mut self) -> Option<Exit>;
    /// Kills it and waits for it to be gone.
    fn kill(&mut self);
}

/// Starts capture processes (faked in tests).
pub trait Spawner {
    fn spawn(&mut self, program: &Path, args: &[OsString]) -> io::Result<Box<dyn Child>>;
}

/// The real processes.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemSpawner;

impl Spawner for SystemSpawner {
    fn spawn(&mut self, program: &Path, args: &[OsString]) -> io::Result<Box<dyn Child>> {
        let mut c = Command::new(program);
        c.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            c.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = c.spawn()?;
        let pipe = child.stderr.take();
        let stderr = std::thread::spawn(move || pipe.map(tail_of).unwrap_or_default());
        Ok(Box::new(SystemChild {
            child,
            stderr: Some(stderr),
            exit: None,
        }))
    }
}

/// Reads `r` to its end and keeps the last [`STDERR_KEPT`] bytes.
pub fn tail_of(mut r: impl Read) -> String {
    let mut kept = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => kept.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        let over = kept.len().saturating_sub(STDERR_KEPT);
        kept.drain(..over);
    }
    String::from_utf8_lossy(&kept).into_owned()
}

struct SystemChild {
    child: std::process::Child,
    stderr: Option<JoinHandle<String>>,
    exit: Option<Exit>,
}

impl SystemChild {
    fn ended(&mut self, success: bool) -> Exit {
        let stderr = self
            .stderr
            .take()
            .and_then(|t| t.join().ok())
            .unwrap_or_default();
        let exit = Exit { success, stderr };
        self.exit = Some(exit.clone());
        exit
    }
}

impl Child for SystemChild {
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>)
    }

    fn quit(&mut self) {
        if let Some(stdin) = self.child.stdin.as_mut() {
            // a process that is gone already needs no asking
            let _ = stdin.write_all(b"q\n").and_then(|()| stdin.flush());
        }
    }

    fn try_wait(&mut self) -> Option<Exit> {
        if self.exit.is_some() {
            return self.exit.clone();
        }
        match self.child.try_wait() {
            Ok(Some(status)) => Some(self.ended(status.success())),
            Ok(None) => None,
            Err(_) => Some(self.ended(false)),
        }
    }

    fn kill(&mut self) {
        if self.try_wait().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.ended(false);
        }
    }
}

/// No ffmpeg outlives its owner.
impl Drop for SystemChild {
    fn drop(&mut self) {
        self.kill();
    }
}

/// The cause of a capture that failed, in the user's words, from ffmpeg's errors
/// (REQ-350).
pub fn failure_cause(stderr: &str) -> String {
    let has = |what: &[&str]| what.iter().any(|w| stderr.contains(w));
    if has(&[
        "No space left on device",
        "Disk quota exceeded",
        "not enough space",
    ]) {
        return "The disk is full".into();
    }
    if has(&[
        "Cannot get the image data",
        "Failed to capture image",
        "outside the screen size",
        "Invalid screen index",
    ]) {
        return "The screen could not be captured (was it removed?)".into();
    }
    if has(&["Permission denied"]) {
        return "Permission denied".into();
    }
    match stderr.lines().rev().find(|l| !l.trim().is_empty()) {
        Some(last) => {
            let last: String = last.trim().chars().take(200).collect();
            format!("The capture failed: {last}")
        }
        None => "The capture failed".into(),
    }
}
