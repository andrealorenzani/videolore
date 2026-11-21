// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! File-name rules of the cache, the partial files and the speech models.

/// True for file names the cache owns: `<32 hex>.mkv` or `<32 hex>.mkv.partial`.
pub fn is_cache_file(name: &str) -> bool {
    let stem = name
        .strip_suffix(".mkv.partial")
        .or_else(|| name.strip_suffix(".mkv"));
    stem.is_some_and(|s| {
        s.len() == 32
            && s.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    })
}

/// The stem of an autosave copy (REQ-189): `<32 hex>` (a saved project: SHA-256 of its path)
/// or `session-<16 hex>` (a project never saved).
pub fn is_autosave_stem(stem: &str) -> bool {
    let hex = |s: &str, n: usize| {
        s.len() == n
            && s.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    };
    hex(stem, 32) || stem.strip_prefix("session-").is_some_and(|s| hex(s, 16))
}

/// True for the files of an autosave copy: `<stem>.vlproj` and `<stem>.path`.
pub fn is_autosave_file(name: &str) -> bool {
    name.strip_suffix(".vlproj")
        .or_else(|| name.strip_suffix(".path"))
        .is_some_and(is_autosave_stem)
}

/// The files of the one-window protocol (REQ-230), in the instance folder of the cache.
pub const INSTANCE_LOCK: &str = "running.lock";
/// The running window writes the time here (milliseconds since the epoch, as text).
pub const INSTANCE_HEARTBEAT: &str = "heartbeat";
/// The folder where a second start leaves the project it was asked to open.
pub const INSTANCE_INBOX: &str = "inbox";

/// True for the files of the inbox: `<milliseconds>-<number>.path`.
pub fn is_inbox_file(name: &str) -> bool {
    name.strip_suffix(".path")
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(a, b)| {
            [a, b]
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 20 && p.bytes().all(|c| c.is_ascii_digit()))
        })
}

/// The three speech-model files (Fast, Balanced, Most accurate).
pub const SPEECH_MODEL_FILES: [&str; 3] = [
    "ggml-tiny-q5_1.bin",
    "ggml-small-q5_1.bin",
    "ggml-large-v3-turbo-q5_0.bin",
];

/// File names videolore uses for partial files outside its work folders.
pub fn is_partial_name(name: &str) -> bool {
    let export = name.len() > ".videolore-partial".len() + 1
        && name.starts_with('.')
        && name.ends_with(".videolore-partial");
    let proxy = name.strip_suffix(".partial").is_some_and(is_cache_file);
    let model = SPEECH_MODEL_FILES
        .iter()
        .any(|m| name.strip_suffix(".part") == Some(m));
    export || proxy || model
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-189
    #[test]
    fn autosave_names() {
        assert!(is_autosave_file(&format!("{}.vlproj", "a".repeat(32))));
        assert!(is_autosave_file(&format!("{}.path", "0".repeat(32))));
        assert!(is_autosave_file("session-0123456789abcdef.vlproj"));
        assert!(!is_autosave_file("session-xyz.vlproj"));
        assert!(!is_autosave_file(&format!("{}.mkv", "a".repeat(32))));
        assert!(!is_autosave_file("notes.vlproj"));
        assert!(!is_autosave_file(&format!("{}.vlproj", "A".repeat(32))));
    }

    #[test]
    fn partial_names() {
        assert!(is_partial_name(".a.mp4.videolore-partial"));
        assert!(!is_partial_name(".videolore-partial"));
        assert!(!is_partial_name("a.mp4.videolore-partial"));
        assert!(is_partial_name(&format!("{}.mkv.partial", "0".repeat(32))));
        assert!(!is_partial_name(&format!("{}.mkv", "0".repeat(32))));
        assert!(is_partial_name("ggml-tiny-q5_1.bin.part"));
        assert!(!is_partial_name("ggml-tiny-q5_1.bin"));
        assert!(!is_partial_name("notes.part"));
    }
}
