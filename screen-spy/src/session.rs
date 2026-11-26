// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Whether this machine can record at all (REQ-357, REQ-358): pure decisions from the
//! environment, the macOS version and the macOS permissions.

use crate::plan::Os;
use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

/// Shown on a native Wayland session (REQ-358).
pub const WAYLAND_MESSAGE: &str = "Screen recording needs an X11 session. This is a Wayland \
    session: recording on Wayland comes later. Log in with an X11 session to record.";
/// Shown on Linux without a usable X display.
pub const NO_DISPLAY_MESSAGE: &str = "Screen recording needs an X11 session: no X display was \
    found.";
/// Shown when macOS does not let the program record the screen (ADR-264).
pub const SCREEN_NOT_ALLOWED: &str = "Screen recording is not allowed for this program. Allow \
    videolore screen spy under System Settings, Privacy & Security, Screen Recording; macOS \
    applies it after the program is closed and started again.";
/// Shown when the microphone permission was denied on macOS.
pub const MICROPHONE_NOT_ALLOWED: &str = "The microphone is not allowed for this program. Allow \
    it under System Settings, Privacy & Security, Microphone, then press Start recording again.";

/// Shown when the system's question about the microphone stays unanswered.
pub const MICROPHONE_NO_ANSWER: &str = "The microphone was not allowed yet: answer the \
    question of macOS, then press Start recording again. Nothing was recorded.";
/// How long the answer to a permission question is waited for.
pub const PERMISSION_WAIT: Duration = Duration::from_secs(120);

/// The microphone permission of macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophonePermission {
    NotAsked,
    Denied,
    Allowed,
}

impl MicrophonePermission {
    /// From `AVAuthorizationStatus`: 0 not determined, 1 restricted, 2 denied,
    /// 3 authorized; anything else is not a permission.
    pub fn of_status(status: isize) -> MicrophonePermission {
        match status {
            0 => Self::NotAsked,
            3 => Self::Allowed,
            _ => Self::Denied,
        }
    }
}

/// How the microphone permission is read and asked for (macOS; faked in tests).
pub trait MicrophoneAccess {
    fn status(&self) -> MicrophonePermission;
    /// Shows the system's question; the answer is sent to `answer` when it is given.
    fn ask(&self, answer: Sender<bool>);
}

/// Passes only when the microphone is allowed: a permission not asked yet is asked for
/// and its answer awaited (at most `wait`, or until `cancel` is set), because macOS
/// records silence until then. Never call this on the window's thread.
pub fn microphone_gate(
    access: &dyn MicrophoneAccess,
    wait: Duration,
    cancel: &AtomicBool,
) -> Result<(), String> {
    match access.status() {
        MicrophonePermission::Allowed => return Ok(()),
        MicrophonePermission::Denied => return Err(MICROPHONE_NOT_ALLOWED.into()),
        MicrophonePermission::NotAsked => {}
    }
    let (tx, rx) = channel();
    access.ask(tx);
    let end = Instant::now() + wait;
    while !cancel.load(Ordering::SeqCst) && Instant::now() < end {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(true) => return Ok(()),
            Ok(false) => return Err(MICROPHONE_NOT_ALLOWED.into()),
            Err(RecvTimeoutError::Timeout) => {}
            // no answer will come
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Err(MICROPHONE_NO_ANSWER.into())
}

/// A macOS version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MacVersion {
    pub major: u32,
    pub minor: u32,
}

/// The first macOS that can record the computer's sound.
pub const COMPUTER_SOUND_MACOS: MacVersion = MacVersion {
    major: 14,
    minor: 6,
};

/// Why recording cannot start on this Mac; `None` when it can.
pub fn mac_blocker(version: Option<MacVersion>) -> Option<String> {
    let needed = format!(
        "Computer sound needs macOS {}.{} or later",
        COMPUTER_SOUND_MACOS.major, COMPUTER_SOUND_MACOS.minor
    );
    match version {
        Some(v) if v >= COMPUTER_SOUND_MACOS => None,
        Some(v) => Some(format!(
            "{needed}; this Mac has macOS {}.{}. Nothing can be recorded on it.",
            v.major, v.minor
        )),
        None => Some(format!(
            "{needed}, and the version of this macOS could not be read."
        )),
    }
}

/// Whether `value` can be the name of an X display (`:0`, `:0.0`, `host:10.0`); nothing
/// else from the environment reaches ffmpeg.
pub fn valid_display(value: &str) -> bool {
    let Some((host, number)) = value.rsplit_once(':') else {
        return false;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let (display, screen) = match number.split_once('.') {
        Some((d, s)) => (d, Some(s)),
        None => (number, None),
    };
    value.len() <= 255
        && digits(display)
        && screen.is_none_or(digits)
        && !host.starts_with('-')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/' | ':'))
}

/// The Linux session: the X display to capture, or why there is none.
pub fn linux_display(env: impl Fn(&str) -> Option<OsString>) -> Result<String, &'static str> {
    let wayland = env("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
        || env("XDG_SESSION_TYPE").is_some_and(|v| v.eq_ignore_ascii_case("wayland"));
    if wayland {
        return Err(WAYLAND_MESSAGE);
    }
    env("DISPLAY")
        .and_then(|v| v.into_string().ok())
        .filter(|d| valid_display(d))
        .ok_or(NO_DISPLAY_MESSAGE)
}

/// What the recorder knows about the machine it runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    /// The X display (Linux); empty elsewhere.
    pub display: String,
    /// Why nothing can be recorded on this machine, shown before Start and at Start.
    pub blocked: Option<String>,
}

impl Platform {
    pub fn detect(
        os: Os,
        env: impl Fn(&str) -> Option<OsString>,
        mac: Option<MacVersion>,
    ) -> Platform {
        let mut p = Platform {
            os,
            display: String::new(),
            blocked: None,
        };
        match os {
            Os::Linux => match linux_display(env) {
                Ok(d) => p.display = d,
                Err(e) => p.blocked = Some(e.to_string()),
            },
            Os::MacOs => p.blocked = mac_blocker(mac),
            Os::Windows => {}
        }
        p
    }

    /// Whether the screen can be shown at all: on an older Mac the preview still works,
    /// on Linux without X11 it cannot.
    pub fn can_preview(&self) -> bool {
        self.os != Os::Linux || self.blocked.is_none()
    }
}
