// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The project (media list + timeline) and its `.vlproj` JSON file.
//!
//! Files are untrusted input: size-limited, schema-versioned, and every id is validated
//! before use. Saving is atomic (temporary file, fsync, rename).

use crate::media::{MediaFormat, MediaId, MediaItem};
use crate::time::Micros;
use crate::timeline::Timeline;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: u64 = 1;
pub const MAX_PROJECT_BYTES: u64 = 16 * 1024 * 1024;
pub const EXTENSION: &str = "vlproj";

/// In-memory project.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Project {
    pub media: Vec<MediaItem>,
    pub timeline: Timeline,
    pub next_media_id: u64,
}

impl Project {
    pub fn media(&self, id: MediaId) -> Option<&MediaItem> {
        self.media.iter().find(|m| m.id == id)
    }

    /// Reserves a new media id.
    pub fn new_media_id(&mut self) -> MediaId {
        self.next_media_id += 1;
        MediaId(self.next_media_id)
    }

    /// Adds an already probed media item (its id must come from [`Self::new_media_id`]).
    pub fn add_media(&mut self, item: MediaItem) {
        self.media.push(item);
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ProjectError {
    #[error("{0}")]
    Io(String),
    #[error("the project file is larger than 16 MiB")]
    TooLarge,
    #[error("not a valid project file: {0}")]
    Invalid(String),
    #[error("project schema version {0} is not supported by this version of videolore")]
    UnsupportedSchema(u64),
}

fn io_err(e: std::io::Error) -> ProjectError {
    ProjectError::Io(e.to_string())
}

#[derive(Serialize, Deserialize)]
struct MediaEntry {
    id: MediaId,
    path_abs: PathBuf,
    #[serde(default)]
    path_rel: Option<PathBuf>,
    name: String,
    duration: Micros,
    format: MediaFormat,
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    schema_version: u64,
    next_media_id: u64,
    media: Vec<MediaEntry>,
    timeline: Timeline,
}

/// Relative path from directory `from` to `to` (both absolute); `None` across roots.
pub fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let f: Vec<Component> = from.components().collect();
    let t: Vec<Component> = to.components().collect();
    let common = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..f.len() {
        out.push("..");
    }
    for c in &t[common..] {
        out.push(c.as_os_str());
    }
    Some(out)
}

fn project_dir(path: &Path) -> PathBuf {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    abs.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// Serialises the project for `path` (relative paths are computed against its folder).
pub fn to_json(project: &Project, path: &Path) -> Result<String, ProjectError> {
    let dir = project_dir(path);
    let file = ProjectFile {
        schema_version: SCHEMA_VERSION,
        next_media_id: project.next_media_id,
        media: project
            .media
            .iter()
            .map(|m| MediaEntry {
                id: m.id,
                path_abs: m.path.clone(),
                path_rel: relative_path(&dir, &m.path),
                name: m.name.clone(),
                duration: m.duration,
                format: m.format.clone(),
            })
            .collect(),
        timeline: project.timeline.clone(),
    };
    serde_json::to_string_pretty(&file).map_err(|e| ProjectError::Invalid(e.to_string()))
}

/// Atomically writes `contents` to `path`: temp file in the same folder, fsync, rename.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), ProjectError> {
    let dir = project_dir(path);
    let name = path
        .file_name()
        .ok_or_else(|| ProjectError::Io("invalid file name".into()))?;
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
    result.map_err(io_err)
}

/// Saves the project to `path`.
pub fn save(project: &Project, path: &Path) -> Result<(), ProjectError> {
    let json = to_json(project, path)?;
    write_atomic(path, json.as_bytes())
}

fn invalid(msg: &str) -> ProjectError {
    ProjectError::Invalid(msg.to_string())
}

/// Parses and validates project JSON. `dir` is the project's folder (for relative paths).
/// Returns the project and the names of sources that could not be found.
pub fn from_json(json: &str, dir: &Path) -> Result<(Project, Vec<String>), ProjectError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ProjectError::Invalid(e.to_string()))?;
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| invalid("missing schema_version"))?;
    if version != SCHEMA_VERSION {
        return Err(ProjectError::UnsupportedSchema(version));
    }
    let file: ProjectFile =
        serde_json::from_value(value).map_err(|e| ProjectError::Invalid(e.to_string()))?;

    let mut ids = BTreeSet::new();
    let mut missing = Vec::new();
    let mut media = Vec::with_capacity(file.media.len());
    for e in file.media {
        if !ids.insert(e.id) {
            return Err(invalid("duplicate media id"));
        }
        if e.duration <= Micros::ZERO {
            return Err(invalid("media with a non-positive duration"));
        }
        let rel = e
            .path_rel
            .as_ref()
            .filter(|r| r.is_relative())
            .map(|r| dir.join(r));
        let found = if e.path_abs.is_absolute() && e.path_abs.is_file() {
            Some(e.path_abs.clone())
        } else {
            rel.filter(|p| p.is_file())
                .and_then(|p| std::path::absolute(p).ok())
        };
        let name = if e.name.is_empty() {
            crate::media::display_name(&e.path_abs)
        } else {
            e.name
        };
        if found.is_none() {
            missing.push(name.clone());
        }
        media.push(MediaItem {
            id: e.id,
            path: found.clone().unwrap_or(e.path_abs),
            name,
            duration: e.duration,
            format: e.format,
            missing: found.is_none(),
        });
    }

    let timeline = file.timeline;
    let mut clip_ids = BTreeSet::new();
    for c in &timeline.clips {
        if !clip_ids.insert(c.id) {
            return Err(invalid("duplicate clip id"));
        }
        let m = media
            .iter()
            .find(|m| m.id == c.media)
            .ok_or_else(|| invalid("clip refers to an unknown media id"))?;
        if c.src_in < Micros::ZERO || c.src_out <= c.src_in || c.src_out > m.duration {
            return Err(invalid("clip range outside its source"));
        }
    }
    for (a, b) in &timeline.fades {
        if !clip_ids.contains(a) || !clip_ids.contains(b) {
            return Err(invalid("fade refers to an unknown clip"));
        }
    }
    let max_clip = clip_ids.iter().map(|c| c.0).max().unwrap_or(0);
    let max_media = ids.iter().map(|m| m.0).max().unwrap_or(0);
    let mut project = Project {
        media,
        timeline,
        next_media_id: file.next_media_id.max(max_media),
    };
    project.timeline.next_clip_id = project.timeline.next_clip_id.max(max_clip);
    Ok((project, missing))
}

/// Loads a project file (at most 16 MiB).
pub fn load(path: &Path) -> Result<(Project, Vec<String>), ProjectError> {
    let meta = std::fs::metadata(path).map_err(io_err)?;
    if meta.len() > MAX_PROJECT_BYTES {
        return Err(ProjectError::TooLarge);
    }
    let text = std::fs::read_to_string(path).map_err(io_err)?;
    from_json(&text, &project_dir(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::tests::media;

    fn sample(dir: &Path) -> Project {
        let mut p = Project::default();
        for secs in [3, 5] {
            let id = p.new_media_id();
            let mut m = media(id.0, secs, true);
            m.path = dir.join(format!("src{}.mp4", id.0));
            std::fs::write(&m.path, b"fake video").unwrap();
            p.add_media(m);
        }
        let m1 = p.media[0].clone();
        let m2 = p.media[1].clone();
        p.timeline.append(&m1).unwrap();
        p.timeline.append(&m2).unwrap();
        p.timeline.split(Micros(4_000_000), Micros(1)).unwrap();
        p.timeline.toggle_fade(0).unwrap();
        p
    }

    // covers: REQ-003
    #[test]
    fn save_and_reopen_is_identical() {
        let dir = tempfile::tempdir().unwrap();
        let p = sample(dir.path());
        let file = dir.path().join("a.vlproj");
        save(&p, &file).unwrap();
        let (q, missing) = load(&file).unwrap();
        assert!(missing.is_empty());
        assert_eq!(p, q);
        assert!(p.media(MediaId(1)).is_some());
        // overwrite works and leaves no temp files behind
        save(&q, &file).unwrap();
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.contains(".tmp-")));
    }

    // covers: REQ-003
    #[test]
    fn missing_sources_are_reported_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let p = sample(dir.path());
        let file = dir.path().join("a.vlproj");
        save(&p, &file).unwrap();
        std::fs::remove_file(&p.media[1].path).unwrap();
        let (q, missing) = load(&file).unwrap();
        assert_eq!(missing, vec!["2.mp4".to_string()]);
        assert!(q.media[1].missing);
        assert_eq!(q.timeline, p.timeline);
    }

    // covers: REQ-003
    #[test]
    fn moved_project_folder_uses_relative_paths() {
        let base = tempfile::tempdir().unwrap();
        let a = base.path().join("a");
        std::fs::create_dir_all(&a).unwrap();
        let p = sample(&a);
        save(&p, &a.join("p.vlproj")).unwrap();
        let b = base.path().join("b");
        std::fs::rename(&a, &b).unwrap();
        let (q, missing) = load(&b.join("p.vlproj")).unwrap();
        assert!(missing.is_empty());
        assert!(q.media[0].path.starts_with(&b));
    }

    #[test]
    fn rejects_bad_files() {
        let d = Path::new("/");
        assert!(matches!(
            from_json("nope", d),
            Err(ProjectError::Invalid(_))
        ));
        assert!(matches!(from_json("{}", d), Err(ProjectError::Invalid(_))));
        assert_eq!(
            from_json(r#"{"schema_version": 2}"#, d).unwrap_err(),
            ProjectError::UnsupportedSchema(2)
        );
        assert!(matches!(
            from_json(r#"{"schema_version": 1}"#, d),
            Err(ProjectError::Invalid(_))
        ));
        let dir = tempfile::tempdir().unwrap();
        let p = sample(dir.path());
        let good = to_json(&p, &dir.path().join("x.vlproj")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&good).unwrap();
        let bad = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut w = v.clone();
            f(&mut w);
            from_json(&w.to_string(), dir.path())
        };
        assert!(matches!(
            bad(&|w| w["media"][1]["id"] = 1.into()),
            Err(ProjectError::Invalid(m)) if m.contains("duplicate media")
        ));
        assert!(matches!(
            bad(&|w| w["media"][0]["duration"] = 0.into()),
            Err(ProjectError::Invalid(_))
        ));
        assert!(matches!(
            bad(&|w| w["timeline"]["clips"][1]["id"] = 1.into()),
            Err(ProjectError::Invalid(m)) if m.contains("duplicate clip")
        ));
        assert!(matches!(
            bad(&|w| w["timeline"]["clips"][0]["media"] = 99.into()),
            Err(ProjectError::Invalid(m)) if m.contains("unknown media")
        ));
        assert!(matches!(
            bad(&|w| w["timeline"]["clips"][0]["src_out"] = 99_000_000.into()),
            Err(ProjectError::Invalid(m)) if m.contains("range")
        ));
        assert!(matches!(
            bad(&|w| w["timeline"]["fades"][0][0] = 99.into()),
            Err(ProjectError::Invalid(m)) if m.contains("fade")
        ));
        // ids are repaired upwards, names default to the file name
        let (q, _) = bad(&|w| {
            w["next_media_id"] = 0.into();
            w["timeline"]["next_clip_id"] = 0.into();
            w["media"][0]["name"] = "".into();
        })
        .unwrap();
        assert_eq!(q.next_media_id, 2);
        assert_eq!(q.timeline.next_clip_id, 3);
        assert_eq!(q.media[0].name, "src1.mp4");
    }

    #[test]
    fn size_limit_and_io_errors() {
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big.vlproj");
        let f = std::fs::File::create(&big).unwrap();
        f.set_len(MAX_PROJECT_BYTES + 1).unwrap();
        assert_eq!(load(&big).unwrap_err(), ProjectError::TooLarge);
        assert!(matches!(
            load(&dir.path().join("none.vlproj")),
            Err(ProjectError::Io(_))
        ));
        assert!(matches!(
            write_atomic(&dir.path().join("no/such/dir/x.vlproj"), b"x"),
            Err(ProjectError::Io(_))
        ));
        assert!(matches!(
            write_atomic(Path::new("/"), b"x"),
            Err(ProjectError::Io(_))
        ));
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_path(Path::new("/a/b"), Path::new("/a/c/d.mp4")),
            Some(PathBuf::from("../c/d.mp4"))
        );
        assert_eq!(
            relative_path(Path::new("/a"), Path::new("/a/x.mp4")),
            Some(PathBuf::from("x.mp4"))
        );
        assert_eq!(relative_path(Path::new("a"), Path::new("b")), None);
    }
}
