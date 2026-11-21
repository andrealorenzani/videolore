// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Autosave and recovery (REQ-189, REQ-190).
//!
//! A project with unsaved changes is copied to videolore's private cache folder every two
//! minutes, never next to the project file and never over it. The copy is the project as
//! [`project::to_json`] writes it, plus a small `.path` file with the path of the project
//! it belongs to. The writing runs on a worker thread from a clone of the project, so
//! typing and playback never wait for the disk. A normal save, discarding the changes or a
//! clean exit remove the copy; after a crash the next start finds a copy that is newer than
//! the saved project and offers to recover it.
//!
//! The copy is read back as any project file is: with the same size limit and the same strict
//! loader. The `.path` text is bounded and checked; nothing else from the cache is trusted.

use crate::base::io::{read_limited, write_atomic};
use crate::base::names::{is_autosave_file, is_autosave_stem};
use crate::project::{self, Project};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant, SystemTime};

/// Time between two autosaves of a project with unsaved changes (a MINOR decision).
pub const INTERVAL: Duration = Duration::from_secs(120);
/// Longest project path kept in a `.path` file (bytes).
pub const MAX_PATH_BYTES: usize = 1024;

/// `<user cache>/autosave`.
pub fn default_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "videolore").map(|d| d.cache_dir().join("autosave"))
}

/// True when an autosave is due: there are unsaved changes and the last copy (or the last
/// save or open) is at least [`INTERVAL`] old.
pub fn due(now: Instant, last: Instant, dirty: bool) -> bool {
    dirty && now.saturating_duration_since(last) >= INTERVAL
}

/// True when a copy written at `copy` is newer than the project file saved at `save`
/// (`None`: never saved, or the file is gone).
pub fn is_newer(copy: SystemTime, save: Option<SystemTime>) -> bool {
    save.is_none_or(|s| copy > s)
}

/// The stem of the copy of `path` (SHA-256 of the path, first 32 hex digits), or of an
/// untitled project of this `session`.
pub fn name_for(path: Option<&Path>, session: &str) -> String {
    match path {
        Some(p) => {
            let sum = Sha256::digest(p.to_string_lossy().as_bytes());
            sum.iter().take(16).map(|b| format!("{b:02x}")).collect()
        }
        None => format!("session-{session}"),
    }
}

/// A random name for the untitled projects of this run: 16 hex digits.
pub fn new_session() -> String {
    let mut h = Sha256::new();
    h.update(std::process::id().to_le_bytes());
    h.update(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    h.update(format!("{:p}", &h).as_bytes());
    h.finalize()
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn project_file(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.vlproj"))
}

fn path_file(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.path"))
}

/// A copy found in the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copy {
    pub stem: String,
    /// The copy itself.
    pub file: PathBuf,
    /// The project it belongs to (`None`: a project that was never saved).
    pub original: Option<PathBuf>,
    pub modified: SystemTime,
}

/// The project path recorded in a `.path` file: absolute, one line, short, no control
/// characters; anything else reads as "none".
fn read_original(file: &Path) -> Option<PathBuf> {
    let bytes = read_limited(file, MAX_PATH_BYTES as u64).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let p = PathBuf::from(&text);
    (!text.is_empty() && p.is_absolute() && !text.chars().any(char::is_control)).then_some(p)
}

/// The copies in `dir` that are newer than their saved project (or belong to a project
/// that was never saved), oldest first. Files that are not ours are ignored.
pub fn find(dir: &Path) -> Vec<Copy> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Copy> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".vlproj")?.to_string();
            if !is_autosave_stem(&stem) {
                return None;
            }
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let modified = meta.modified().ok()?;
            let original = read_original(&path_file(dir, &stem));
            let saved = original
                .as_ref()
                .and_then(|o| std::fs::metadata(o).ok())
                .and_then(|m| m.modified().ok());
            is_newer(modified, saved).then(|| Copy {
                stem,
                file: e.path(),
                original,
                modified,
            })
        })
        .collect();
    out.sort_by_key(|c| (c.modified, c.stem.clone()));
    out
}

/// Deletes the copy with this stem (its own two files only).
pub fn remove(dir: &Path, stem: &str) {
    if !is_autosave_stem(stem) {
        return;
    }
    for f in [project_file(dir, stem), path_file(dir, stem)] {
        if is_autosave_file(&f.file_name().unwrap_or_default().to_string_lossy()) {
            let _ = std::fs::remove_file(f);
        }
    }
}

/// Reads a copy back as a project: the strict loader, with the relative paths of the
/// project it belongs to resolved against that project's folder.
pub fn load(copy: &Copy) -> Result<(Project, Vec<String>), project::ProjectError> {
    let dir = copy
        .original
        .as_deref()
        .map(crate::base::io::parent_dir)
        .unwrap_or_default();
    project::load_in(&copy.file, &dir)
}

/// Writes the copy of `project` (a clone made by the caller). Both files are written
/// atomically; the `.path` file is only there for a saved project.
pub fn write(
    dir: &Path,
    stem: &str,
    project: &Project,
    original: Option<&Path>,
) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir).map_err(io)?;
    // relative paths are computed against the folder of the project the copy belongs to
    let base = original
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_file(dir, stem));
    let json = project::to_json(project, &base).map_err(|e| e.to_string())?;
    write_atomic(&project_file(dir, stem), json.as_bytes()).map_err(io)?;
    match original {
        Some(o) if o.to_string_lossy().len() <= MAX_PATH_BYTES => {
            write_atomic(&path_file(dir, stem), o.to_string_lossy().as_bytes()).map_err(io)
        }
        _ => {
            let _ = std::fs::remove_file(path_file(dir, stem));
            Ok(())
        }
    }
}

/// A write running on a worker thread.
pub struct Running {
    rx: Receiver<Result<(), String>>,
}

/// Starts writing a copy of `project` on a worker thread.
pub fn start(dir: PathBuf, stem: String, project: Project, original: Option<PathBuf>) -> Running {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(write(&dir, &stem, &project, original.as_deref()));
    });
    Running { rx }
}

impl Running {
    /// `None` while it runs, then the result.
    pub fn poll(&self) -> Option<Result<(), String>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("autosave stopped".into()))
            }
        }
    }

    /// Waits for the end (tests and exit).
    pub fn wait(&self) -> Result<(), String> {
        self.rx
            .recv()
            .unwrap_or_else(|_| Err("autosave stopped".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    // covers: REQ-189
    #[test]
    fn a_copy_is_due_every_two_minutes_but_only_with_unsaved_changes() {
        let t0 = Instant::now();
        assert!(!due(t0 + secs(119), t0, true));
        assert!(due(t0 + secs(120), t0, true));
        assert!(due(t0 + secs(3600), t0, true));
        assert!(!due(t0 + secs(3600), t0, false), "nothing unsaved");
        assert!(!due(t0, t0 + secs(5), true), "the clock never goes back");
        assert_eq!(INTERVAL, secs(120));
    }

    // covers: REQ-190
    #[test]
    fn a_copy_is_offered_when_it_is_newer_than_the_save_or_there_is_no_save() {
        let now = SystemTime::now();
        let later = now + secs(10);
        assert!(is_newer(later, Some(now)));
        assert!(!is_newer(now, Some(later)));
        assert!(!is_newer(now, Some(now)));
        assert!(is_newer(now, None), "a project never saved");
    }

    // covers: REQ-189
    #[test]
    fn names_come_from_the_path_or_the_session_and_are_ours_only() {
        let a = name_for(Some(Path::new("/p/a.vlproj")), "s");
        assert_eq!(a, name_for(Some(Path::new("/p/a.vlproj")), "other"));
        assert_ne!(a, name_for(Some(Path::new("/p/b.vlproj")), "s"));
        assert!(is_autosave_stem(&a) && a.len() == 32);
        let u = name_for(None, &new_session());
        assert!(is_autosave_stem(&u), "{u}");
        assert_ne!(new_session(), new_session());
    }

    fn project_with_clip(dir: &Path) -> Project {
        let mut p = Project::default();
        let id = p.new_media_id();
        let mut m = crate::timeline::tests::media(id.0, 5, true);
        let file = dir.join("a.mp4");
        std::fs::write(&file, b"x").unwrap();
        m.path = file;
        p.add_media(m.clone());
        p.timeline.append(&m).unwrap();
        p
    }

    // covers: REQ-189, REQ-190
    #[test]
    fn a_copy_is_written_found_read_back_and_removed() {
        let d = tempfile::tempdir().unwrap();
        let cache = d.path().join("cache");
        let p = project_with_clip(d.path());
        let original = d.path().join("proj.vlproj");
        std::fs::write(&original, "old").unwrap();
        // saved a while ago
        std::fs::File::options()
            .write(true)
            .open(&original)
            .unwrap()
            .set_modified(SystemTime::now() - secs(60))
            .unwrap();
        let stem = name_for(Some(&original), "s");
        // the writing runs on a worker
        let run = start(
            cache.clone(),
            stem.clone(),
            p.clone(),
            Some(original.clone()),
        );
        run.wait().unwrap();
        assert_eq!(
            std::fs::read_to_string(&original).unwrap(),
            "old",
            "the project is untouched"
        );
        let found = find(&cache);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].original.as_deref(), Some(original.as_path()));
        let (back, missing) = load(&found[0]).unwrap();
        assert!(missing.is_empty());
        assert_eq!(back.timeline.clips.len(), 1);
        // a project file saved after the copy makes the copy stale: nothing is offered
        let later = std::fs::metadata(&found[0].file)
            .unwrap()
            .modified()
            .unwrap()
            + secs(5);
        std::fs::File::options()
            .write(true)
            .open(&original)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert!(find(&cache).is_empty());
        // an untitled project has a session copy without a .path file
        let u = name_for(None, "0123456789abcdef");
        write(&cache, &u, &p, None).unwrap();
        let found = find(&cache);
        assert_eq!(found.len(), 1);
        assert!(found[0].original.is_none());
        assert!(!cache.join(format!("{u}.path")).exists());
        // removing deletes the two files of that copy only
        std::fs::write(cache.join("notes.txt"), "mine").unwrap();
        remove(&cache, &stem);
        remove(&cache, &u);
        remove(&cache, "not-a-stem");
        assert!(find(&cache).is_empty());
        assert!(cache.join("notes.txt").is_file());
        assert!(!cache.join(format!("{stem}.vlproj")).exists());
    }

    // covers: REQ-190
    #[test]
    fn the_recorded_project_path_is_checked_and_foreign_files_are_ignored() {
        let d = tempfile::tempdir().unwrap();
        let cache = d.path().join("c");
        std::fs::create_dir_all(&cache).unwrap();
        let p = project_with_clip(d.path());
        let stem = name_for(Some(Path::new("/x/y.vlproj")), "s");
        write(&cache, &stem, &p, Some(Path::new("/x/y.vlproj"))).unwrap();
        for bad in [
            "relative/path.vlproj",
            "/x/y\nz.vlproj",
            "",
            &"/a".repeat(2000),
        ] {
            std::fs::write(path_file(&cache, &stem), bad).unwrap();
            let c = find(&cache);
            assert_eq!(c.len(), 1);
            assert!(c[0].original.is_none(), "{bad:?} is not a path");
        }
        std::fs::write(path_file(&cache, &stem), [0xff, 0xfe]).unwrap();
        assert!(find(&cache)[0].original.is_none());
        // files that are not ours are not copies
        std::fs::write(cache.join("other.vlproj"), "x").unwrap();
        std::fs::create_dir(cache.join(format!("{}.vlproj", "b".repeat(32)))).unwrap();
        assert_eq!(find(&cache).len(), 1);
        assert!(find(&d.path().join("missing")).is_empty());
        // a copy that is not a valid project is refused by the loader
        let broken = Copy {
            stem: "x".into(),
            file: cache.join("other.vlproj"),
            original: None,
            modified: SystemTime::now(),
        };
        assert!(load(&broken).is_err());
        // a path that is too long is not written
        let long = PathBuf::from(format!("/{}", "a".repeat(2000)));
        write(&cache, &stem, &p, Some(&long)).unwrap();
        assert!(!path_file(&cache, &stem).exists());
    }
}
