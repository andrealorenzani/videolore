// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Capped and atomic file input and output.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// The folder of `path` (made absolute first); empty when it has none.
pub fn parent_dir(path: &Path) -> PathBuf {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    abs.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// Reads at most `limit` bytes from `r`; errors when there is more.
pub fn read_capped(mut r: impl Read, limit: u64) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    r.by_ref().take(limit + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    Ok(buf)
}

/// Reads a regular file of at most `limit` bytes. Anything else (a folder, a pipe, a
/// device) is refused without blocking; a file that grows while being read is refused too.
/// Errors of kind `InvalidData` mean "too large", `InvalidInput` "not a regular file".
pub fn read_limited(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    use io::{Error, ErrorKind};
    let not_regular = || Error::new(ErrorKind::InvalidInput, "not a regular file");
    if !std::fs::metadata(path)?.is_file() {
        return Err(not_regular());
    }
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    // Non-blocking open: a pipe swapped in after the check above cannot hang the caller.
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(0o4000); // O_NONBLOCK
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(0x0004); // O_NONBLOCK
    }
    let f = o.open(path)?;
    if !f.metadata()?.is_file() {
        return Err(not_regular());
    }
    read_capped(f, limit)
}

/// Atomically writes `contents` to `path`: temp file in the same folder, fsync, rename.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let dir = parent_dir(path);
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(format!(".tmp-{}", std::process::id()));
    let tmp = dir.join(tmp_name);
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)?;
        #[cfg(unix)]
        if let Ok(d) = std::fs::File::open(&dir) {
            let _ = d.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_leaves_no_temp_file_and_refuses_bad_names() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("a.txt");
        write_atomic(&f, b"x").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"x");
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
        let e = write_atomic(Path::new("/"), b"x").unwrap_err();
        assert_eq!(e.to_string(), "invalid file name");
        assert!(write_atomic(&d.path().join("no/such/x"), b"x").is_err());
        assert_eq!(read_limited(&f, 1).unwrap(), b"x");
        assert_eq!(
            read_limited(d.path(), 10).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
