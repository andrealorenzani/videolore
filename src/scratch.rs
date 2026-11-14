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

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Name of the lock file inside a work folder.
pub const LOCK_FILE: &str = "lock";
/// Name of the list of outside partial files.
pub const TARGETS_FILE: &str = "targets";
/// Longest `targets` file read during cleanup.
const MAX_TARGETS_BYTES: u64 = 64 * 1024;

/// `<user cache>/work`, or the system temporary folder when there is no cache folder.
pub fn default_root() -> PathBuf {
    directories::ProjectDirs::from("", "", "videolore")
        .map(|d| d.cache_dir().join("work"))
        .unwrap_or_else(|| std::env::temp_dir().join("videolore-work"))
}

/// Creates `dir` (and its parents) and makes it private to the user on Unix.
pub fn private_dir(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Creates a new file only the user can read (0600 on Unix). An old file of that name is
/// removed first (a leftover; never followed if it is a symlink).
pub fn create_private_file(path: &Path) -> io::Result<File> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a folder has that name",
            ));
        }
        Ok(_) => std::fs::remove_file(path)?,
        Err(_) => {}
    }
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

/// Makes a finished private file readable like any other document (0644 on Unix).
pub fn publish(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// A job's private work folder; removed on drop.
#[derive(Debug)]
pub struct Work {
    // Field order matters: the lock is closed before the folder is removed (Windows cannot
    // delete an open file).
    lock: Option<File>,
    dir: Option<tempfile::TempDir>,
}

impl Work {
    /// Creates a private folder in `root` and locks it for this job.
    pub fn new(root: &Path) -> io::Result<Work> {
        private_dir(root)?;
        let dir = tempfile::Builder::new().prefix("job-").tempdir_in(root)?;
        private_dir(dir.path())?;
        let lock = File::create(dir.path().join(LOCK_FILE))?;
        lock.lock()?;
        Ok(Work {
            lock: Some(lock),
            dir: Some(dir),
        })
    }

    pub fn path(&self) -> &Path {
        self.dir.as_ref().map(|d| d.path()).unwrap_or(Path::new(""))
    }

    /// Records a partial file written outside the folder, so that a crash leaves nothing.
    pub fn register(&self, target: &Path) -> io::Result<()> {
        let target = std::path::absolute(target)?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path().join(TARGETS_FILE))?;
        let line = target.to_string_lossy();
        if line.contains(['\n', '\r']) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "file names with line breaks are not supported",
            ));
        }
        writeln!(f, "{line}")
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        drop(self.lock.take());
        if let Some(d) = self.dir.take() {
            let _ = d.close();
        }
    }
}

/// File names videolore uses for partial files outside its work folders.
pub fn is_partial_name(name: &str) -> bool {
    let export = name.len() > ".videolore-partial".len() + 1
        && name.starts_with('.')
        && name.ends_with(".videolore-partial");
    let proxy = name
        .strip_suffix(".partial")
        .is_some_and(crate::proxy::is_cache_file);
    let model = crate::stt::builtin_models()
        .iter()
        .any(|m| name == format!("{}.part", m.file));
    export || proxy || model
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
    let Ok(bytes) = crate::project::read_limited(list, MAX_TARGETS_BYTES) else {
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

    // covers: REQ-059
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

    // covers: REQ-059
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
