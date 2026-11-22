// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! LUT reading with a cache by bytes, loads in the background and the copies in the Work
//! folder (2.1.0, ADR-205, ADR-206). Imports base, the parser and the parent types only
//! (never render, app or ui).
//!
//! A 256-point LUT takes seconds to read: [`Load::start`] reads on a thread of its own and
//! reports the bytes read, so the window stays responsive; a newer choice drops the
//! [`Load`], which stops the thread at the next MiB.

use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::{Lut, LutError, MAX_BYTES, MAX_WHOLE, parse_by_extension, parse_reader};

/// Bytes of tables the cache keeps (a 256-point table is 100 MB, a 144-point one 18 MB).
pub const CACHE_BUDGET: usize = 192 * 1024 * 1024;

/// A LUT file larger than this (normalised text, REQ-301) is not copied into the project.
pub const COPY_LIMIT_BYTES: usize = 20 * 1024 * 1024;

/// Reads and parses a LUT file (a regular file of at most 512 MiB). `progress` gets
/// `(bytes read, file size)` every MiB and stops the read when it returns `false`.
pub fn read_with(path: &Path, progress: &mut dyn FnMut(u64, u64) -> bool) -> Result<Lut, LutError> {
    use std::io::ErrorKind;
    let file = crate::base::io::open_regular(path).map_err(|e| match e.kind() {
        ErrorKind::InvalidInput => LutError::NotRegular,
        _ => LutError::Read(e.to_string()),
    })?;
    let meta = file.metadata().map_err(|e| LutError::Read(e.to_string()))?;
    let (len, stamp) = (meta.len(), meta.modified().ok());
    if len > MAX_BYTES {
        return Err(LutError::TooLarge);
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("cube");
    let lut = if matches!(ext.to_ascii_lowercase().as_str(), "3dl" | "csp") {
        if len > MAX_WHOLE {
            return Err(LutError::Invalid(
                "a .3dl or .csp file larger than 64 MiB is not supported".into(),
            ));
        }
        let bytes = crate::base::io::read_capped(file, MAX_WHOLE).map_err(|e| match e.kind() {
            ErrorKind::InvalidData => LutError::TooLarge,
            _ => LutError::Read(e.to_string()),
        })?;
        parse_by_extension(ext, &bytes)?
    } else {
        let reader = BufReader::with_capacity(1 << 20, file);
        parse_reader(reader, len, &mut |read| progress(read, len))?
    };
    // a file that changed under the reader may be half old and half new
    let after = std::fs::metadata(path).map_err(|e| LutError::Read(e.to_string()))?;
    if (after.len(), after.modified().ok()) != (len, stamp) {
        return Err(LutError::Changed);
    }
    Ok(lut)
}

/// [`read_with`] without a progress report.
pub fn read(path: &Path) -> Result<Lut, LutError> {
    read_with(path, &mut |_, _| true)
}

type Stamp = (u64, Option<SystemTime>);

struct Entry {
    path: String,
    stamp: Stamp,
    lut: Arc<Lut>,
}

/// Parsed LUTs by path, re-read when the file's size or modification time changes, the
/// least recently used first. The table in use is always kept; others go when the tables
/// together pass [`CACHE_BUDGET`].
static CACHE: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

/// LUT tables kept in the projects opened in this run, by the path the clips use (REQ-293):
/// used when the file itself is gone.
static EMBEDDED: Mutex<Vec<(String, Arc<Lut>)>> = Mutex::new(Vec::new());

/// Remembers the copy of a LUT that a project carries (at most 16 are kept).
pub fn embed(path: &str, lut: Lut) {
    let mut e = EMBEDDED.lock().unwrap_or_else(|e| e.into_inner());
    e.retain(|x| x.0 != path);
    if e.len() >= 16 {
        e.remove(0);
    }
    e.push((path.to_string(), Arc::new(lut)));
}

/// True when a copy of the LUT at `path` is kept from an opened project.
pub fn has_embedded(path: &str) -> bool {
    EMBEDDED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .any(|x| x.0 == path)
}

fn embedded(path: &str) -> Option<Arc<Lut>> {
    let e = EMBEDDED.lock().unwrap_or_else(|e| e.into_inner());
    e.iter().find(|x| x.0 == path).map(|x| x.1.clone())
}

/// The table of `path` when it is already known (nothing is read): from the cache while the
/// file is unchanged, or the copy a project carries when the file is gone.
pub fn peek(path: &str) -> Option<Arc<Lut>> {
    let Ok(meta) = std::fs::metadata(path) else {
        return embedded(path);
    };
    let stamp = (meta.len(), meta.modified().ok());
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let i = c.iter().position(|e| e.path == path && e.stamp == stamp)?;
    let e = c.remove(i);
    let lut = e.lut.clone();
    c.push(e);
    Some(lut)
}

/// Keeps `lut` for `path`, newest last, and drops the oldest tables over the budget.
fn remember(path: &str, stamp: Stamp, lut: &Arc<Lut>) {
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    remember_in(&mut c, path, stamp, lut, CACHE_BUDGET);
}

fn remember_in(c: &mut Vec<Entry>, path: &str, stamp: Stamp, lut: &Arc<Lut>, budget: usize) {
    c.retain(|e| e.path != path);
    c.push(Entry {
        path: path.to_string(),
        stamp,
        lut: lut.clone(),
    });
    let mut total: usize = c.iter().map(|e| e.lut.bytes()).sum();
    while total > budget && c.len() > 1 {
        total -= c.remove(0).lut.bytes();
    }
}

/// [`read`] through the cache. A changed file is read and validated again.
pub fn cached(path: &str) -> Result<Arc<Lut>, LutError> {
    cached_with(path, &mut |_, _| true)
}

/// [`cached`] with a progress report (see [`read_with`]).
pub fn cached_with(
    path: &str,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<Arc<Lut>, LutError> {
    if let Some(l) = peek(path) {
        return Ok(l);
    }
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return Err(LutError::Read(e.to_string())),
    };
    let stamp = (meta.len(), meta.modified().ok());
    let lut = Arc::new(read_with(Path::new(path), progress)?);
    remember(path, stamp, &lut);
    Ok(lut)
}

/// A LUT file being read on a thread of its own.
pub struct Load {
    rx: Receiver<LoadMsg>,
    cancel: Arc<AtomicBool>,
    /// The latest `(bytes read, file size)` seen.
    pub progress: (u64, u64),
}

/// What a [`Load`] reports.
pub enum LoadMsg {
    Progress(u64, u64),
    Done(Result<Arc<Lut>, LutError>),
}

impl Load {
    /// Starts reading `path` (the cache is checked first by the caller with [`peek`]).
    pub fn start(path: &str) -> Load {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let (flag, p) = (cancel.clone(), path.to_string());
        let failed = tx.clone();
        let spawned = std::thread::Builder::new()
            .name("lut-load".into())
            .spawn(move || {
                let tx2 = tx.clone();
                let r = cached_with(&p, &mut |read, total| {
                    let _ = tx2.send(LoadMsg::Progress(read, total));
                    !flag.load(Ordering::Relaxed)
                });
                let _ = tx.send(LoadMsg::Done(r));
            });
        if spawned.is_err() {
            let _ = failed.send(LoadMsg::Done(Err(LutError::Read("no thread".into()))));
        }
        Load {
            rx,
            cancel,
            progress: (0, 0),
        }
    }

    /// The next message, if one is ready (progress is also kept in [`Load::progress`]).
    pub fn poll(&mut self) -> Option<LoadMsg> {
        let m = self.rx.try_recv().ok()?;
        if let LoadMsg::Progress(a, b) = &m {
            self.progress = (*a, *b);
        }
        Some(m)
    }

    /// The share read, 0..=100.
    pub fn percent(&self) -> u8 {
        let (a, b) = self.progress;
        (a.min(b) * 100).checked_div(b).unwrap_or(0) as u8
    }
}

impl Drop for Load {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Files whose Work copies were written by a [`Prep`] for the preview (REQ-312): the
/// preview shows a LUT only once its copy is there, so the window never waits for the write.
static PREPARED: Mutex<Vec<(String, Stamp)>> = Mutex::new(Vec::new());

fn stamp_of(path: &str) -> Stamp {
    std::fs::metadata(path).map_or((0, None), |m| (m.len(), m.modified().ok()))
}

/// True when the copies of the LUT at `path` are ready in the preview's folders and the file
/// has not changed since.
pub fn is_prepared(path: &str) -> bool {
    let s = stamp_of(path);
    PREPARED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .any(|(p, st)| p == path && *st == s)
}

/// Forgets what is prepared (the preview's folders were removed).
pub fn forget_prepared() {
    PREPARED.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// What a [`Prep`] reports.
pub enum PrepMsg {
    /// Share done, 0..=100 (reading is the first half, writing the copies the second).
    Progress(u8),
    Done(Result<Arc<Lut>, LutError>),
}

/// The preparation of one LUT for the preview (REQ-312): its file is read and parsed, then
/// the Work copy is written into each folder of the preview, all on a thread of its own with
/// the share done reported. Dropping it stops the thread (a newer choice replaces it).
pub struct Prep {
    rx: Receiver<PrepMsg>,
    cancel: Arc<AtomicBool>,
    /// The latest share seen, 0..=100.
    pub percent: u8,
    /// The file being prepared.
    pub path: String,
}

impl Prep {
    /// Starts preparing `path` for the folders `dirs`.
    pub fn start(path: &str, dirs: Vec<std::path::PathBuf>) -> Prep {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let (flag, p) = (cancel.clone(), path.to_string());
        let failed = tx.clone();
        let spawned = std::thread::Builder::new()
            .name("lut-prep".into())
            .spawn(move || {
                let r = prepare(&p, &dirs, &flag, &tx);
                let _ = tx.send(PrepMsg::Done(r));
            });
        if spawned.is_err() {
            let _ = failed.send(PrepMsg::Done(Err(LutError::Read("no thread".into()))));
        }
        Prep {
            rx,
            cancel,
            percent: 0,
            path: path.to_string(),
        }
    }

    /// The next message, if one is ready.
    pub fn poll(&mut self) -> Option<PrepMsg> {
        let m = self.rx.try_recv().ok()?;
        if let PrepMsg::Progress(p) = &m {
            self.percent = *p;
        }
        Some(m)
    }
}

impl Drop for Prep {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn prepare(
    path: &str,
    dirs: &[std::path::PathBuf],
    cancel: &AtomicBool,
    tx: &std::sync::mpsc::Sender<PrepMsg>,
) -> Result<Arc<Lut>, LutError> {
    let stamp = stamp_of(path);
    let lut = cached_with(path, &mut |read, total| {
        let p = (read.min(total) * 50).checked_div(total).unwrap_or(0) as u8;
        let _ = tx.send(PrepMsg::Progress(p));
        !cancel.load(Ordering::Relaxed)
    })?;
    for (i, d) in dirs.iter().enumerate() {
        let base = 50 + 50 * i / dirs.len().max(1);
        let span = 50 / dirs.len().max(1);
        write_copy_with(d, &lut, &mut |n| {
            let p = base + span * n.min(lut.size().pow(3)) / lut.size().pow(3).max(1);
            let _ = tx.send(PrepMsg::Progress(p.min(99) as u8));
            !cancel.load(Ordering::Relaxed)
        })
        .map_err(|e| LutError::Read(e.to_string()))?;
    }
    {
        let mut done = PREPARED.lock().unwrap_or_else(|e| e.into_inner());
        done.retain(|(p, _)| p != path);
        done.push((path.to_string(), stamp));
    }
    let _ = tx.send(PrepMsg::Progress(100));
    Ok(lut)
}

/// The name of the Work folder copy of `lut` (`lut-<hash>.cube`, fixed by code: no user
/// text reaches a filter, ADR-206).
pub fn copy_name(lut: &Lut) -> String {
    format!("lut-{}.cube", &lut.hash()[..16])
}

/// Writes the Work folder copy of `lut` into `dir` once per content: a streamed write to a
/// temporary name, then a rename, so ffmpeg never reads half a file. Returns the file name.
pub fn write_copy(dir: &Path, lut: &Lut) -> std::io::Result<String> {
    write_copy_with(dir, lut, &mut |_| true)
}

/// [`write_copy`] with a progress report: `progress` gets the entries written so far and
/// stops the write when it returns `false` (the partial file is removed).
pub fn write_copy_with(
    dir: &Path,
    lut: &Lut,
    progress: &mut dyn FnMut(usize) -> bool,
) -> std::io::Result<String> {
    let name = copy_name(lut);
    let target = dir.join(&name);
    let want = lut.cube_len() as u64;
    if std::fs::metadata(&target).is_ok_and(|m| m.len() == want) {
        return Ok(name);
    }
    let tmp = dir.join(format!("{name}.part"));
    {
        let mut w = BufWriter::with_capacity(1 << 20, std::fs::File::create(&tmp)?);
        let done = lut.write_cube(&mut w, progress).and_then(|()| w.flush());
        if let Err(e) = done {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    }
    std::fs::rename(&tmp, &target)?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lut::parse::tests::Gen;

    fn small(n: usize) -> Arc<Lut> {
        let mut s = format!("LUT_3D_SIZE {n}\n");
        for _ in 0..n * n * n {
            s.push_str("0.25 0.5 0.75\n");
        }
        Arc::new(crate::lut::parse(s.as_bytes()).unwrap())
    }

    // covers: REQ-300
    #[test]
    fn the_cache_is_limited_by_bytes_and_keeps_the_newest() {
        let mut c = Vec::new();
        let (a, b, d) = (small(8), small(8), small(8));
        let budget = a.bytes() * 2;
        remember_in(&mut c, "a", (1, None), &a, budget);
        remember_in(&mut c, "b", (1, None), &b, budget);
        remember_in(&mut c, "c", (1, None), &d, budget);
        let names: Vec<&str> = c.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(names, ["b", "c"]);
        // one table over the budget alone is still kept
        remember_in(&mut c, "big", (1, None), &small(16), budget);
        assert_eq!(c.len(), 1);
    }

    // covers: REQ-300
    #[test]
    fn the_work_copy_is_named_by_content_written_once_and_read_back_exactly() {
        let lut = small(5);
        let d = tempfile::tempdir().unwrap();
        let name = write_copy(d.path(), &lut).unwrap();
        assert_eq!(name, copy_name(&lut));
        assert!(name.starts_with("lut-") && name.ends_with(".cube") && name.len() == 4 + 16 + 5);
        let file = d.path().join(&name);
        assert_eq!(
            std::fs::metadata(&file).unwrap().len() as usize,
            lut.cube_len()
        );
        let before = std::fs::metadata(&file).unwrap().modified().unwrap();
        assert_eq!(write_copy(d.path(), &lut).unwrap(), name);
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            before
        );
        assert!(!d.path().join(format!("{name}.part")).exists());
        let again = crate::lut::read(&file).unwrap();
        assert_eq!(again.hash(), lut.hash());
        assert_eq!(again, *lut);
    }

    // covers: REQ-300
    #[test]
    fn a_load_in_the_background_reports_progress_and_the_table() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("g.cube");
        let mut f = std::io::BufWriter::new(std::fs::File::create(&p).unwrap());
        std::io::copy(&mut Gen::new(40), &mut f).unwrap();
        std::io::Write::flush(&mut f).unwrap();
        drop(f);
        let mut load = Load::start(p.to_str().unwrap());
        let lut = loop {
            match load.poll() {
                Some(LoadMsg::Done(r)) => break r.unwrap(),
                Some(LoadMsg::Progress(..)) => {}
                None => std::thread::sleep(std::time::Duration::from_millis(2)),
            }
        };
        assert_eq!(lut.size(), 40);
        assert!(load.percent() >= 50);
        assert!(peek(p.to_str().unwrap()).is_some());
    }

    // covers: REQ-300
    #[test]
    fn a_dropped_load_stops_and_a_missing_file_says_so() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("g.cube");
        let mut f = std::io::BufWriter::new(std::fs::File::create(&p).unwrap());
        std::io::copy(&mut Gen::new(60), &mut f).unwrap();
        std::io::Write::flush(&mut f).unwrap();
        drop(f);
        let r = read_with(&p, &mut |_, _| false);
        assert_eq!(r, Err(LutError::Cancelled));
        assert!(peek(p.to_str().unwrap()).is_none());
        let mut load = Load::start("/no/such/file.cube");
        let r = loop {
            if let Some(LoadMsg::Done(r)) = load.poll() {
                break r;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        assert!(matches!(r, Err(LutError::Read(_))));
        drop(Load::start(p.to_str().unwrap()));
    }
}

#[cfg(test)]
mod prep_tests {
    use super::*;

    // covers: REQ-312
    #[test]
    fn a_preparation_reports_a_share_that_only_grows_and_leaves_the_copies_in_every_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("p.cube");
        std::fs::write(&file, crate::lut::tests::identity(52)).unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let path = file.to_str().unwrap();
        assert!(!is_prepared(path));
        let mut prep = Prep::start(path, vec![a.clone(), b.clone()]);
        let mut seen = Vec::new();
        let lut = loop {
            match prep.poll() {
                Some(PrepMsg::Progress(p)) => seen.push(p),
                Some(PrepMsg::Done(r)) => break r.unwrap(),
                None => std::thread::sleep(std::time::Duration::from_millis(1)),
            }
        };
        assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
        assert!(seen.len() >= 2 && *seen.last().unwrap() == 100, "{seen:?}");
        assert_eq!(lut.size(), 52);
        let name = copy_name(&lut);
        for d in [&a, &b] {
            assert_eq!(
                std::fs::metadata(d.join(&name)).unwrap().len(),
                lut.cube_len() as u64
            );
        }
        assert!(is_prepared(path));
        // the file changes: it is not prepared any more
        std::fs::write(&file, crate::lut::tests::identity(3)).unwrap();
        assert!(!is_prepared(path));
        // a preparation of a file that is not a LUT fails with its reason
        let bad = dir.path().join("bad.cube");
        std::fs::write(&bad, "nope").unwrap();
        let mut prep = Prep::start(bad.to_str().unwrap(), vec![a]);
        let r = loop {
            match prep.poll() {
                Some(PrepMsg::Done(r)) => break r,
                _ => std::thread::sleep(std::time::Duration::from_millis(1)),
            }
        };
        assert!(r.is_err());
    }
}
