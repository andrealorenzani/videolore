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
