// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! One window for double-clicked projects (REQ-230, ADR-182; Windows and Linux).
//!
//! The first window holds a lock on `<cache>/instance/running.lock` (the standard library's
//! advisory file lock, released by the system when the process ends, however it ends) and
//! writes the time to `heartbeat` every [`BEAT_EVERY`]. A second start that was given a
//! project looks at the lock: if it is held and the heartbeat is fresh ([`HEARTBEAT_MAX_AGE`])
//! it leaves the project in `inbox/` and exits; the first window reads the inbox in its
//! normal tick and opens the project through the usual path (the unsaved-changes prompt of
//! REQ-028 included). If the first window cannot be reached (no lock, an old heartbeat, a
//! full inbox) the second start opens its own window, as before.
//!
//! The inbox is a file in the private (0700) folder of the user; what is read from it is
//! untrusted: at most [`MAX_INBOX_FILES`] files of [`MAX_INBOX_PATH`] bytes, an absolute path
//! of an existing file without control characters, nothing else. No socket, no pipe, no
//! `unsafe`.

use crate::base::{io as bio, names, private};
use std::fs::{self, File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often the running window writes its heartbeat and looks at the inbox.
pub const BEAT_EVERY: Duration = Duration::from_millis(250);
/// A heartbeat older than this means no window is running.
pub const HEARTBEAT_MAX_AGE: Duration = Duration::from_secs(2);
/// Most files the inbox may hold.
pub const MAX_INBOX_FILES: usize = 64;
/// Longest path an inbox file may hold, in bytes.
pub const MAX_INBOX_PATH: usize = 4096;

/// The window that holds the lock.
#[derive(Debug)]
pub struct Instance {
    dir: PathBuf,
    /// Held until the process ends.
    _lock: File,
    last: Option<Instant>,
}

/// What a start found out.
#[derive(Debug)]
pub enum Claim {
    /// This is the first window: it keeps the lock and polls the inbox.
    First(Instance),
    /// The project was handed to the running window: this process has nothing to do.
    Forwarded,
    /// No window to talk to (or no project to hand over): open a window as usual.
    Alone,
}

/// The time in milliseconds since the epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Whether a heartbeat written at `beat_ms` is fresh at `now_ms`.
pub fn is_fresh(beat_ms: Option<u64>, now_ms: u64) -> bool {
    beat_ms.is_some_and(|b| now_ms.saturating_sub(b) <= HEARTBEAT_MAX_AGE.as_millis() as u64)
}

/// The folder of the protocol in the user's cache.
pub fn default_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "videolore").map(|d| d.cache_dir().join("instance"))
}

fn read_beat(dir: &Path) -> Option<u64> {
    let bytes = bio::read_limited(&dir.join(names::INSTANCE_HEARTBEAT), 64).ok()?;
    String::from_utf8(bytes).ok()?.trim().parse().ok()
}

/// A path text the inbox may hold: absolute, UTF-8 (so it round-trips), short, no control
/// characters, the file exists.
pub fn valid_inbox_path(text: &str) -> Option<PathBuf> {
    if text.is_empty() || text.len() > MAX_INBOX_PATH || text.chars().any(char::is_control) {
        return None;
    }
    let p = PathBuf::from(text);
    (p.is_absolute() && p.is_file()).then_some(p)
}

/// Decides the role of this start. `project` is the file it was asked to open (if any);
/// `now` is the time in milliseconds. Never fails: any problem means [`Claim::Alone`].
pub fn claim(dir: &Path, project: Option<&Path>, now: u64) -> Claim {
    if private::private_dir(dir).is_err() {
        return Claim::Alone;
    }
    let Ok(lock) = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(names::INSTANCE_LOCK))
    else {
        return Claim::Alone;
    };
    match lock.try_lock() {
        Ok(()) => {
            // the first window: whatever a window that is gone left is stale
            let inbox = dir.join(names::INSTANCE_INBOX);
            if let Ok(rd) = fs::read_dir(&inbox) {
                for e in rd.flatten() {
                    let _ = fs::remove_file(e.path());
                }
            }
            let mut me = Instance {
                dir: dir.to_path_buf(),
                _lock: lock,
                last: None,
            };
            me.beat(now);
            Claim::First(me)
        }
        Err(TryLockError::WouldBlock) => match project {
            Some(p) if is_fresh(read_beat(dir), now) && forward(dir, p, now) => Claim::Forwarded,
            _ => Claim::Alone,
        },
        Err(TryLockError::Error(_)) => Claim::Alone,
    }
}

/// Leaves `project` in the inbox of the running window. False when it cannot be done.
fn forward(dir: &Path, project: &Path, now: u64) -> bool {
    let Ok(abs) = std::path::absolute(project) else {
        return false;
    };
    let Some(text) = abs.to_str().filter(|t| valid_inbox_path(t).is_some()) else {
        return false;
    };
    let inbox = dir.join(names::INSTANCE_INBOX);
    if private::private_dir(&inbox).is_err() {
        return false;
    }
    let taken = fs::read_dir(&inbox)
        .map(|r| r.count())
        .unwrap_or(usize::MAX);
    if taken >= MAX_INBOX_FILES {
        return false;
    }
    let name = format!("{now}-{}.path", std::process::id());
    let partial = inbox.join(format!("{name}.partial"));
    bio::write_atomic(&partial, text.as_bytes())
        .and_then(|()| fs::rename(&partial, inbox.join(&name)))
        .is_ok()
}

impl Instance {
    /// Writes the heartbeat.
    fn beat(&mut self, now: u64) {
        let _ = bio::write_atomic(
            &self.dir.join(names::INSTANCE_HEARTBEAT),
            now.to_string().as_bytes(),
        );
    }

    /// Called from the window's tick: at most every [`BEAT_EVERY`] it writes the heartbeat and
    /// reads the inbox. Returns the projects other starts asked this window to open, oldest
    /// first (each file is removed once read).
    pub fn poll(&mut self, at: Instant, now: u64) -> Vec<PathBuf> {
        if self.last.is_some_and(|l| at.duration_since(l) < BEAT_EVERY) {
            return Vec::new();
        }
        self.last = Some(at);
        self.beat(now);
        let inbox = self.dir.join(names::INSTANCE_INBOX);
        let Ok(rd) = fs::read_dir(&inbox) else {
            return Vec::new();
        };
        let mut found: Vec<(String, PathBuf)> = rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().into_string().ok()?;
                names::is_inbox_file(&n).then(|| (n, e.path()))
            })
            .collect();
        // oldest first: by time, then number (both are plain digits)
        found.sort_by_key(|(n, _)| {
            let (a, b) = n
                .trim_end_matches(".path")
                .split_once('-')
                .unwrap_or(("0", "0"));
            (
                a.parse::<u64>().unwrap_or(u64::MAX),
                b.parse::<u64>().unwrap_or(0),
            )
        });
        found.truncate(MAX_INBOX_FILES);
        let mut out = Vec::new();
        for (_, path) in found {
            let text = bio::read_limited(&path, MAX_INBOX_PATH as u64 + 1)
                .ok()
                .and_then(|b| String::from_utf8(b).ok());
            let _ = fs::remove_file(&path);
            if let Some(p) = text.as_deref().and_then(valid_inbox_path) {
                out.push(p);
            }
        }
        out
    }
}

static CURRENT: Mutex<Option<Instance>> = Mutex::new(None);

/// Makes `instance` the window the program polls from [`poll_current`].
pub fn install(instance: Instance) {
    if let Ok(mut c) = CURRENT.lock() {
        *c = Some(instance);
    }
}

/// The project another start asked this window to open, if any. Of several, only the first
/// is opened (REQ-081); the others are dropped. Does nothing in a window that does not hold
/// the lock.
pub fn poll_current() -> Option<PathBuf> {
    poll_slot(&CURRENT)
}

/// [`poll_current`] of the window in `slot`.
fn poll_slot(slot: &Mutex<Option<Instance>>) -> Option<PathBuf> {
    let mut c = slot.lock().ok()?;
    c.as_mut()?
        .poll(Instant::now(), now_ms())
        .into_iter()
        .next()
}

/// Start-up of the program: true when this process should open its window; false when its
/// project was handed to the running window. Only on Windows and Linux (macOS keeps the one
/// running application by itself).
pub fn start(project: Option<&Path>) -> bool {
    if !cfg!(any(windows, target_os = "linux")) {
        return true;
    }
    let Some(dir) = default_dir() else {
        return true;
    };
    match claim(&dir, project, now_ms()) {
        Claim::First(i) => {
            install(i);
            true
        }
        Claim::Forwarded => false,
        Claim::Alone => true,
    }
}

#[cfg(test)]
mod tests;
