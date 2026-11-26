// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! What the ffmpeg found can do (ADR-266): its capture devices, its encoders and, on
//! Linux, the PulseAudio sources it sees. Pure readers of ffmpeg's lists.

use crate::plan::{AudioFormat, Os};
use std::collections::BTreeSet;

/// The device names of `ffmpeg -devices` that can be read from.
pub fn parse_devices(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(flags) = words.next() else {
            continue;
        };
        // the flag column: "D", "DE", "D." ... (the legend lines have a "=" after it)
        if !flags.contains('D') || !flags.chars().all(|c| matches!(c, 'D' | 'E' | '.')) {
            continue;
        }
        // newer ffmpeg has a second flag column "d" (a device) before the name
        let Some(name) = words.find(|w| !matches!(*w, "d" | ".")) else {
            continue;
        };
        if name != "=" && !words.clone().next().is_some_and(|w| w == "=") {
            out.extend(name.split(',').map(str::to_string));
        }
    }
    out
}

/// The capture devices the system needs.
pub fn required_devices(os: Os) -> &'static [&'static str] {
    match os {
        Os::Linux => &["x11grab", "pulse"],
        Os::Windows => &["gdigrab"],
        Os::MacOs => &["avfoundation"],
    }
}

/// The devices and encoders a recording needs and ffmpeg does not have.
pub fn missing(
    os: Os,
    audio: AudioFormat,
    devices: &BTreeSet<String>,
    encoders: &BTreeSet<String>,
) -> Vec<String> {
    let devices = required_devices(os)
        .iter()
        .filter(|d| !devices.contains(**d))
        .map(|d| format!("the capture device {d}"));
    let encoders = ["libx264", audio.encoder()]
        .into_iter()
        .filter(|e| !encoders.contains(*e))
        .map(|e| format!("the encoder {e}"));
    devices.chain(encoders).collect()
}

/// The name avfoundation knows the screen at `index` of the system's list by. A name is
/// used because the plain device numbers count the cameras first.
pub fn avfoundation_screen(index: usize) -> String {
    format!("Capture screen {index}")
}

/// One source of `ffmpeg -sources pulse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseDevice {
    pub name: String,
    /// The one `@DEFAULT_SOURCE@` stands for (marked `*`).
    pub default: bool,
}

/// The sources of `ffmpeg -sources pulse`.
pub fn parse_pulse_sources(text: &str) -> Vec<PulseDevice> {
    text.lines()
        .skip_while(|l| !l.contains("sources for pulse"))
        .skip(1)
        .filter_map(|l| {
            let l = l.trim_start();
            let (default, l) = match l.strip_prefix('*') {
                Some(rest) => (true, rest.trim_start()),
                None => (false, l),
            };
            let name = l.split_whitespace().next()?.to_string();
            l.contains('[').then_some(PulseDevice { name, default })
        })
        .collect()
}

/// Why the default source cannot stand for the microphone: the monitor of an output
/// records what the computer plays, which would then be in the track twice (REQ-357).
/// `None` when it can, or when the list tells nothing (the source is still tried).
pub fn microphone_problem(sources: &[PulseDevice]) -> Option<&'static str> {
    let monitor = |s: &PulseDevice| s.name.ends_with(".monitor");
    if !sources.is_empty() && sources.iter().all(monitor) {
        Some("no microphone was found")
    } else if sources.iter().any(|s| s.default && monitor(s)) {
        Some(
            "the default sound input is the monitor of an output, which would record the \
             computer's sound twice. Choose your microphone as the default input in the \
             system's sound settings",
        )
    } else {
        None
    }
}
