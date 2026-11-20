// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Private work folders and crash-safe cleanup of partial files.
//!
//! Each background job (export, subtitle generation, preview copy, speech-file download)
//! gets a [`Work`] folder `<root>/job-XXXX/` that only the user can read (0700 on Unix).
//! It holds the job's temporary files (extracted audio, subtitle file, font) and:
//! * a `lock` file, exclusively locked for as long as the job runs;
//! * a `targets` file listing partial files the job writes outside the folder (the
//!   `.name.videolore-partial` export, a preview copy's `.mkv.partial`, a download's
//!   `.part`).
//!
//! When the job ends normally the folder is removed. After a crash, [`cleanup_stale`] (run
//! at the next start) finds folders whose lock is free, deletes their listed partial files
//! (only regular files with one of the known partial-file names, never symlinks) and the
//! folders. Folders whose lock is held belong to another running videolore and are left
//! alone.

use crate::base::io::read_limited;
use crate::base::names::is_partial_name;
use crate::base::private::{LOCK_FILE, TARGETS_FILE};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Longest `targets` file read during cleanup.
const MAX_TARGETS_BYTES: u64 = 64 * 1024;

/// `<user cache>/work`, or the system temporary folder when there is no cache folder.
pub fn default_root() -> PathBuf {
    directories::ProjectDirs::from("", "", "videolore")
        .map(|d| d.cache_dir().join("work"))
        .unwrap_or_else(|| std::env::temp_dir().join("videolore-work"))
}

/// Removes the leftovers of jobs that did not end (crash, power cut). Returns the number of
/// work folders removed. Folders still locked by a running videolore are kept.
pub fn cleanup_stale(root: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(root) else {
        return 0;
    };
    let mut removed = 0;
    for e in rd.filter_map(Result::ok) {
        let dir = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
        if !is_dir || !name.starts_with("job-") {
            continue;
        }
        let Ok(lock) = File::open(dir.join(LOCK_FILE)) else {
            // created but never locked (crash right at the start): only when clearly old
            let old = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_secs() > 60);
            if old && std::fs::remove_dir_all(&dir).is_ok() {
                removed += 1;
            }
            continue;
        };
        if lock.try_lock().is_err() {
            continue; // another videolore is using it
        }
        remove_targets(&dir.join(TARGETS_FILE));
        drop(lock);
        if std::fs::remove_dir_all(&dir).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn remove_targets(list: &Path) {
    let Ok(bytes) = read_limited(list, MAX_TARGETS_BYTES) else {
        return;
    };
    for line in String::from_utf8_lossy(&bytes).lines() {
        let p = Path::new(line);
        let ok_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(is_partial_name);
        let regular = std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_file());
        if p.is_absolute() && ok_name && regular {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::private::{Work, create_private_file, publish};
    use std::io::Write;

    fn mode(p: &Path) -> u32 {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(p).unwrap().permissions().mode() & 0o777
        }
        #[cfg(not(unix))]
        {
            let _ = p;
            0
        }
    }

    // covers: REQ-059, REQ-152
    #[test]
    fn work_folders_are_private_and_removed_at_the_end() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("work");
        let w = Work::new(&root).unwrap();
        let dir = w.path().to_path_buf();
        assert!(dir.join(LOCK_FILE).is_file());
        if cfg!(unix) {
            assert_eq!(mode(&root), 0o700);
            assert_eq!(mode(&dir), 0o700);
        }
        // a running job is never cleaned up, even by another videolore
        assert_eq!(cleanup_stale(&root), 0);
        assert!(dir.is_dir());
        drop(w);
        assert!(!dir.exists());
        assert!(default_root().ends_with("work") || default_root().ends_with("videolore-work"));
    }

    // covers: REQ-059, REQ-152
    #[test]
    fn crash_leftovers_are_removed_at_the_next_start() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("work");
        let out = base.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let partial = out.join(".film.mp4.videolore-partial");
        let proxy = out.join(format!("{}.mkv.partial", "a".repeat(32)));
        let model = out.join("ggml-small-q5_1.bin.part");
        let user_file = out.join("holiday.mp4");
        let w = Work::new(&root).unwrap();
        for p in [&partial, &proxy, &model, &user_file] {
            let mut f = create_private_file(p).unwrap();
            f.write_all(b"data").unwrap();
            w.register(p).unwrap();
        }
        if cfg!(unix) {
            assert_eq!(mode(&partial), 0o600);
        }
        std::fs::write(w.path().join("audio.f32"), b"pcm").unwrap();
        let dir = w.path().to_path_buf();
        // simulate a crash: the folder stays, its lock is released with the process
        let mut w = w;
        drop(w.lock.take());
        let kept = w.dir.take().unwrap().keep();
        assert_eq!(kept, dir);
        drop(w);
        assert!(dir.exists());
        // a folder created but never locked is left alone while recent
        std::fs::create_dir_all(root.join("job-starting")).unwrap();
        std::fs::create_dir_all(root.join("not-a-job")).unwrap();
        assert_eq!(cleanup_stale(&root), 1);
        assert!(!dir.exists());
        assert!(!partial.exists() && !proxy.exists() && !model.exists());
        assert!(
            user_file.exists(),
            "files with other names are never removed"
        );
        assert!(root.join("job-starting").exists());
        assert!(root.join("not-a-job").exists());
        assert_eq!(cleanup_stale(&base.path().join("nowhere")), 0);
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_never_follows_symlinks() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("work");
        let precious = base.path().join("precious.txt");
        std::fs::write(&precious, b"keep").unwrap();
        let link = base.path().join(".x.videolore-partial");
        std::os::unix::fs::symlink(&precious, &link).unwrap();
        let mut w = Work::new(&root).unwrap();
        w.register(&link).unwrap();
        drop(w.lock.take());
        let _ = w.dir.take().unwrap().keep();
        assert_eq!(cleanup_stale(&root), 1);
        assert!(precious.exists());
        // a new private file replaces a leftover symlink without touching its target
        let mut f = create_private_file(&link).unwrap();
        f.write_all(b"new").unwrap();
        assert_eq!(std::fs::read(&precious).unwrap(), b"keep");
        publish(&link).unwrap();
        assert_eq!(mode(&link), 0o644);
        assert!(create_private_file(base.path()).is_err());
        let w = Work::new(&root).unwrap();
        assert!(w.register(Path::new("/a\nb")).is_err());
    }

    // covers: REQ-151
    #[test]
    fn file_names_are_those_of_version_2_0_0() {
        use crate::base::names::{SPEECH_MODEL_FILES, is_cache_file, is_partial_name};
        assert_eq!(LOCK_FILE, "lock");
        assert_eq!(TARGETS_FILE, "targets");
        let base = tempfile::tempdir().unwrap();
        let w = Work::new(base.path()).unwrap();
        let name = w.path().file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("job-"));
        drop(w);
        // the speech-model names are the ones the download list uses, in the same order
        let models = crate::stt::builtin_models();
        for (m, f) in models.iter().zip(SPEECH_MODEL_FILES) {
            assert_eq!(m.file, f);
            assert!(is_partial_name(&format!("{f}.part")));
        }
        assert_eq!(
            SPEECH_MODEL_FILES,
            [
                "ggml-tiny-q5_1.bin",
                "ggml-small-q5_1.bin",
                "ggml-large-v3-turbo-q5_0.bin"
            ]
        );
        let key = "0123456789abcdef0123456789abcdef";
        assert!(
            is_cache_file(&format!("{key}.mkv")) && is_partial_name(&format!("{key}.mkv.partial"))
        );
        assert!(is_partial_name(".x.mp4.videolore-partial"));
        // the temporary name of an atomic write
        let f = base.path().join("p.vlproj");
        std::fs::create_dir(
            base.path()
                .join(format!(".p.vlproj.tmp-{}", std::process::id())),
        )
        .unwrap();
        assert!(
            crate::base::io::write_atomic(&f, b"x").is_err(),
            "the temp name is .<name>.tmp-<pid>"
        );
    }
}
