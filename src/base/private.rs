// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Private folders and files, and a job's private work folder ([`Work`]).

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Name of the lock file inside a work folder.
pub const LOCK_FILE: &str = "lock";
/// Name of the list of outside partial files.
pub const TARGETS_FILE: &str = "targets";

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
    pub(crate) lock: Option<File>,
    pub(crate) dir: Option<tempfile::TempDir>,
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
