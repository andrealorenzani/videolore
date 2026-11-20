// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The project (media list, video track, music track, mix, subtitles) and its `.vlproj`
//! JSON file.
//!
//! Files are untrusted input: size-limited, schema-versioned, and every id is validated
//! before use. Saving is atomic (temporary file, fsync, rename). Schema 1 files (videolore
//! 0.1.x) are migrated in memory; files from a newer version are refused untouched.

use crate::camera::CameraPaths;
use crate::media::{MediaFormat, MediaId, MediaItem};
use crate::music::{Mix, MusicTrack};
use crate::subtitles::{self, Subtitles};
use crate::text::{TextItem, Texts};
use crate::time::Micros;
use crate::timeline::Timeline;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

mod v9;

/// Schema written by this version (9 since 1.6.1: every whole-video effect is a copy on each clip,
/// edge fades are flags of the clips; 8 since 1.6.0: Apply LUT paths; 7 since 1.5.0: Change aspect, Scale and Move with
/// keyframes; 6 since 1.4.0: text widths; 5 since 1.3.0: text items and 360 anchors; 4 since 1.2.6: subtitle sizes, the 360° switch and
/// the camera paths; 3 since 1.2.0: clip volumes, fades at the video's edges, subtitle
/// positions, subtitle tracks of media).
pub const SCHEMA_VERSION: u64 = 9;
/// Oldest schema that can still be opened (0.1.x).
pub const OLDEST_SCHEMA: u64 = 1;
pub const MAX_PROJECT_BYTES: u64 = 16 * 1024 * 1024;
pub const EXTENSION: &str = "vlproj";

/// Everything an edit can change: what undo/redo snapshots.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edit {
    pub timeline: Timeline,
    pub music: MusicTrack,
    pub subtitles: Subtitles,
    pub mix: Mix,
    /// Points of view of the 360 sources (source time).
    pub camera: CameraPaths,
    /// Text items on the timeline (1.3.0).
    pub texts: Texts,
}

/// In-memory project.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Project {
    pub media: Vec<MediaItem>,
    pub timeline: Timeline,
    pub music: MusicTrack,
    pub subtitles: Subtitles,
    pub mix: Mix,
    pub camera: CameraPaths,
    pub texts: Texts,
    pub next_media_id: u64,
}

/// The saved content of a project (for "unsaved changes" detection): media that finished
/// analysing plus the edit. Transient state (analysis, proxies) is not part of it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectDoc {
    media: Vec<(MediaId, PathBuf, String, Micros, MediaFormat)>,
    edit: Edit,
}

impl ProjectDoc {
    /// Records the subtitle tracks found by a later check of a file (the project does not
    /// become "unsaved" because of it).
    pub fn set_subtitle_tracks(&mut self, id: MediaId, tracks: Vec<crate::media::SubtitleTrack>) {
        if let Some(m) = self.media.iter_mut().find(|m| m.0 == id) {
            m.4.subtitle_tracks = Some(tracks);
        }
    }

    /// Records what a later check of a file found about spherical information (the
    /// project does not become "unsaved" because of it).
    pub fn set_spherical(&mut self, id: MediaId, spherical: bool) {
        if let Some(m) = self.media.iter_mut().find(|m| m.0 == id) {
            m.4.spherical = Some(spherical);
        }
    }
}

impl Project {
    /// A copy of the editable part.
    pub fn edit(&self) -> Edit {
        Edit {
            timeline: self.timeline.clone(),
            music: self.music.clone(),
            subtitles: self.subtitles.clone(),
            mix: self.mix,
            camera: self.camera.clone(),
            texts: self.texts.clone(),
        }
    }

    /// Replaces the editable part.
    pub fn set_edit(&mut self, e: Edit) {
        self.timeline = e.timeline;
        self.music = e.music;
        self.subtitles = e.subtitles;
        self.mix = e.mix;
        self.camera = e.camera;
        self.texts = e.texts;
    }

    /// What would be saved.
    pub fn doc(&self) -> ProjectDoc {
        ProjectDoc {
            media: self
                .media
                .iter()
                .filter(|m| m.is_ready())
                .map(|m| {
                    (
                        m.id,
                        m.path.clone(),
                        m.name.clone(),
                        m.duration,
                        m.format.clone(),
                    )
                })
                .collect(),
            edit: self.edit(),
        }
    }

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
    #[error(
        "this project was saved by a newer version of videolore (format {0}): update videolore to open it"
    )]
    TooNew(u64),
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
    // Added in schema 2; absent (= empty / defaults) in schema 1 files.
    #[serde(default)]
    music: MusicTrack,
    #[serde(default)]
    mix: Mix,
    #[serde(default)]
    subtitles: Subtitles,
    // Added in schema 4; absent (= no points of view) before.
    #[serde(default, skip_serializing_if = "CameraPaths::is_empty")]
    camera: CameraPaths,
    // Added in schema 5; absent (= no text items) before.
    #[serde(default)]
    texts: Vec<TextItem>,
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
            .filter(|m| m.is_ready())
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
        music: project.music.clone(),
        mix: project.mix,
        subtitles: project.subtitles.clone(),
        camera: project.camera.clone(),
        texts: project.texts.items.clone(),
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

/// Takes the `size` of every text out of a project of schema 5 or older (a text of 1.4.0
/// has a `width` instead) and gives each a placeholder width.
fn take_legacy_text_sizes(value: &mut serde_json::Value) -> Result<Vec<u16>, ProjectError> {
    let Some(items) = value.get_mut("texts").and_then(|t| t.as_array_mut()) else {
        return Ok(Vec::new());
    };
    let mut sizes = Vec::with_capacity(items.len());
    for t in items {
        let Some(obj) = t.as_object_mut() else {
            return Err(invalid("text item is not an object"));
        };
        let size = obj
            .remove("size")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .filter(|v| (subtitles::SIZE_MIN..=subtitles::SIZE_MAX).contains(v))
            .ok_or_else(|| invalid("text item with an invalid place or size"))?;
        obj.insert("width".into(), crate::text::DEFAULT_WIDTH.into());
        sizes.push(size);
    }
    Ok(sizes)
}

/// Gives a text of 1.3.0 the width (and for a text fixed to the sphere the height) that
/// keep its look (REQ-123, ADR-088).
fn migrate_text(item: &mut crate::text::TextItem, old_size: u16) {
    item.width = crate::text::legacy_width(&item.text, old_size);
    if let Some(a) = item.anchor.as_mut() {
        a.height_cd =
            crate::text::legacy_anchor_height(&item.text, a.height_cd, a.width_cd, old_size);
    }
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
    if version > SCHEMA_VERSION {
        return Err(ProjectError::TooNew(version));
    }
    if version < OLDEST_SCHEMA {
        return Err(ProjectError::UnsupportedSchema(version));
    }
    let mut value = value;
    // texts of 1.3.0 and before have a size, not a width: read it aside (ADR-088)
    let legacy_sizes = if version < 6 {
        take_legacy_text_sizes(&mut value)?
    } else {
        Vec::new()
    };
    // the whole-video effects of 1.6.0 and before are read aside (REQ-149, ADR-124)
    let legacy = if version < 9 {
        Some(v9::Legacy::take(&mut value)?)
    } else {
        None
    };
    let mut file: ProjectFile =
        serde_json::from_value(value).map_err(|e| ProjectError::Invalid(e.to_string()))?;
    for (item, size) in file.texts.iter_mut().zip(legacy_sizes) {
        migrate_text(item, size);
    }

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
        // The stored name is only shown: hostile characters are neutralised, length capped.
        let name = if e.name.is_empty() {
            crate::media::display_name(&e.path_abs)
        } else {
            crate::media::sanitize_for_display(&e.name.chars().take(255).collect::<String>())
        };
        if found.is_none() {
            missing.push(name.clone());
        }
        let mut format = e.format;
        // subtitle track descriptions come from the file: made safe, never refused
        format.subtitle_tracks = format.subtitle_tracks.map(|v| {
            v.into_iter()
                .take(crate::media::MAX_SUBTITLE_TRACKS)
                .map(crate::media::SubtitleTrack::sanitised)
                .collect()
        });
        media.push(MediaItem {
            id: e.id,
            path: found.clone().unwrap_or(e.path_abs),
            name,
            duration: e.duration,
            format,
            missing: found.is_none(),
            status: Default::default(),
            proxy: Default::default(),
        });
    }

    let mut timeline = file.timeline;
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
        if c.volume > crate::timeline::FULL_VOLUME {
            return Err(invalid("clip volume above 100%"));
        }
    }
    for (a, b) in &timeline.fades {
        if !clip_ids.contains(a) || !clip_ids.contains(b) {
            return Err(invalid("fade refers to an unknown clip"));
        }
    }
    let mut mix = file.mix;
    if mix.video_volume > 100 || mix.music_volume > 100 {
        return Err(invalid("volume above 100%"));
    }
    validate_frames(&mut timeline, &media);
    if let Some(old) = legacy {
        v9::apply(&mut timeline, &mut mix, old, &mut missing);
    }
    validate_luts(&mut timeline, &mut missing);
    let mut music = file.music;
    validate_music(&mut music, &media)?;
    let mut subs = file.subtitles;
    validate_subtitles(&mut subs, &media)?;
    let mut camera = file.camera;
    camera
        .validate(|m| media.iter().find(|x| x.id == m).map(|x| x.duration))
        .map_err(invalid)?;
    let texts = Texts {
        next_id: file.texts.iter().map(|t| t.id.0).max().unwrap_or(0),
        items: file.texts,
    };
    texts.validate().map_err(|m| invalid(&m))?;
    let max_clip = clip_ids.iter().map(|c| c.0).max().unwrap_or(0);
    let max_media = ids.iter().map(|m| m.0).max().unwrap_or(0);
    let mut project = Project {
        media,
        timeline,
        music,
        subtitles: subs,
        mix,
        camera,
        texts,
        next_media_id: file.next_media_id.max(max_media),
    };
    project.timeline.next_clip_id = project.timeline.next_clip_id.max(max_clip);
    Ok((project, missing))
}

/// Corrects damaged Change aspect, Scale and Move values without refusing the file
/// (REQ-132): values are brought inside their limits, keys outside the media are dropped, equal times keep the first key, at most 200
/// keys per path stay, entries of unknown clips and empty entries are removed.
fn validate_frames(tl: &mut crate::timeline::Timeline, media: &[MediaItem]) {
    use crate::keyframes::sanitise;
    let entries = std::mem::take(&mut tl.clip_frames);
    let mut seen = BTreeSet::new();
    for mut e in entries {
        let Some(c) = tl.get(e.clip).copied() else {
            continue;
        };
        if !seen.insert(e.clip) {
            continue;
        }
        let limit = media
            .iter()
            .find(|m| m.id == c.media)
            .map_or(c.src_out, |m| m.duration);
        for k in &mut e.scale {
            k.v.clamp();
        }
        for k in &mut e.moved {
            clamp_pos(&mut k.v);
        }
        sanitise(&mut e.scale, limit);
        sanitise(&mut e.moved, limit);
        tl.put_frames(e);
    }
}

/// Checks the Apply LUT paths (REQ-137): an entry that is empty, longer than 4096
/// characters, holds a NUL, or belongs to an unknown clip is removed with a note; a file
/// that is not there is reported like a missing source (the project still opens, REQ-136).
fn validate_luts(tl: &mut crate::timeline::Timeline, notes: &mut Vec<String>) {
    use crate::timeline::FrameTarget;
    let mut check = |p: &str| -> bool {
        if !crate::lut::path_ok(p) {
            notes.push("A damaged colour (LUT) entry was removed".to_string());
            return false;
        }
        if !Path::new(p).exists() {
            let note = format!("{} (colour LUT file)", crate::lut::display_name(p));
            // one file on several clips (a whole-video LUT) is named once
            if !notes.contains(&note) {
                notes.push(note);
            }
        }
        true
    };
    let entries = std::mem::take(&mut tl.clip_luts);
    let mut seen = BTreeSet::new();
    for e in entries {
        if tl.get(e.clip).is_none() || !seen.insert(e.clip) {
            continue;
        }
        if check(&e.path) {
            let _ = tl.set_lut(FrameTarget::Clip(e.clip), Some(e.path));
        }
    }
}

fn clamp_pos(p: &mut crate::subtitles::Pos) {
    p.x = p.x.min(crate::subtitles::POS_MAX);
    p.y = p.y.min(crate::subtitles::POS_MAX);
}

fn validate_music(music: &mut MusicTrack, media: &[MediaItem]) -> Result<(), ProjectError> {
    let mut ids = BTreeSet::new();
    for c in &music.clips {
        if !ids.insert(c.id) {
            return Err(invalid("duplicate music clip id"));
        }
        let m = media
            .iter()
            .find(|m| m.id == c.media)
            .ok_or_else(|| invalid("music clip refers to an unknown media id"))?;
        if m.has_video() {
            return Err(invalid("music clip refers to a video file"));
        }
        if c.src_in < Micros::ZERO
            || c.src_out <= c.src_in
            || c.src_out > m.duration
            || c.start < Micros::ZERO
        {
            return Err(invalid("music clip range outside its source"));
        }
    }
    music.clips.sort_by_key(|c| (c.start, c.id));
    if music.clips.windows(2).any(|w| w[0].end() > w[1].start) {
        return Err(invalid("music clips overlap"));
    }
    music.next_id = music
        .next_id
        .max(ids.iter().map(|i| i.0).max().unwrap_or(0));
    Ok(())
}

fn validate_subtitles(subs: &mut Subtitles, media: &[MediaItem]) -> Result<(), ProjectError> {
    let mut ids = BTreeSet::new();
    let mut words = 0usize;
    for l in &subs.lines {
        if !ids.insert(l.id) {
            return Err(invalid("duplicate subtitle id"));
        }
        if !media.iter().any(|m| m.id == l.media) {
            return Err(invalid("subtitle refers to an unknown media id"));
        }
        if l.start < Micros::ZERO || l.end <= l.start {
            return Err(invalid("subtitle with an invalid time range"));
        }
        if l.text.chars().count() > subtitles::MAX_TEXT {
            return Err(invalid("subtitle text too long"));
        }
        if l.position.is_some_and(|p| !p.is_valid()) {
            return Err(invalid("subtitle position outside the picture"));
        }
        if l.size
            .is_some_and(|v| !(subtitles::SIZE_MIN..=subtitles::SIZE_MAX).contains(&v))
        {
            return Err(invalid("subtitle size outside 25%-400%"));
        }
        if l.anchor.is_some_and(|a| !a.is_valid()) {
            return Err(invalid("subtitle 360 anchor out of range"));
        }
        if l.words
            .iter()
            .any(|w| w.start < l.start || w.end > l.end || w.end < w.start)
        {
            return Err(invalid("subtitle word outside its line"));
        }
        words += l.words.len();
    }
    if words > subtitles::MAX_WORDS {
        return Err(invalid("too many subtitle words"));
    }
    if subs
        .language
        .as_ref()
        .is_some_and(|l| l.len() > 16 || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
    {
        return Err(invalid("invalid subtitle language"));
    }
    subs.next_id = subs.next_id.max(ids.iter().map(|i| i.0).max().unwrap_or(0));
    Ok(())
}

/// Reads a regular file of at most `limit` bytes. Anything else (a folder, a pipe, a
/// device) is refused without blocking; a file that grows while being read is refused too.
/// Errors of kind `InvalidData` mean "too large", `InvalidInput` "not a regular file".
pub fn read_limited(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, ErrorKind};
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
    crate::settings::read_capped(f, limit)
}

/// Loads a project file (a regular file of at most 16 MiB).
pub fn load(path: &Path) -> Result<(Project, Vec<String>), ProjectError> {
    let bytes = read_limited(path, MAX_PROJECT_BYTES).map_err(|e| match e.kind() {
        std::io::ErrorKind::InvalidData => ProjectError::TooLarge,
        std::io::ErrorKind::InvalidInput => ProjectError::Invalid("not a regular file".into()),
        _ => io_err(e),
    })?;
    let text = String::from_utf8(bytes).map_err(|_| invalid("the file is not UTF-8 text"))?;
    from_json(&text, &project_dir(path))
}

#[cfg(test)]
pub(crate) mod tests {
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

    fn sha(p: &Path) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        Sha256::digest(std::fs::read(p).unwrap()).to_vec()
    }

    fn files_in(dir: &Path) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        v.sort();
        v
    }

    // covers: REQ-057
    #[test]
    fn hostile_project_files_are_refused_or_contained() {
        let dir = tempfile::tempdir().unwrap();
        let p = sample(dir.path());
        let good: serde_json::Value =
            serde_json::from_str(&to_json(&p, &dir.path().join("x.vlproj")).unwrap()).unwrap();
        let sources: Vec<(PathBuf, Vec<u8>)> = p
            .media
            .iter()
            .map(|m| (m.path.clone(), sha(&m.path)))
            .collect();
        let mutate = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v = good.clone();
            f(&mut v);
            v.to_string()
        };
        let deep = format!(
            "{{\"schema_version\":2,\"x\":{}{}}}",
            "[".repeat(300),
            "]".repeat(300)
        );
        let refused = [
            "".to_string(),
            "{".to_string(),
            "null".to_string(),
            "\u{0}\u{1}".to_string(),
            good.to_string()[..good.to_string().len() / 2].to_string(),
            deep,
            good.to_string()
                .replacen("\"schema_version\":", "\"schema_version\":1e309,\"x\":", 1),
            mutate(&|v| v["next_media_id"] = serde_json::json!(-1)),
            mutate(&|v| v["media"][0]["duration"] = serde_json::json!(-5)),
            good.to_string()
                .replacen("\"duration\":", "\"duration\":1e309,\"x\":", 1),
            mutate(&|v| v["timeline"]["clips"][0]["src_in"] = serde_json::json!(-1)),
            mutate(&|v| v["timeline"]["clips"][0]["src_out"] = serde_json::json!(u64::MAX)),
            mutate(&|v| v["timeline"]["clips"][0]["src_out"] = serde_json::json!(i64::MAX)),
            mutate(&|v| v["mix"]["video_volume"] = serde_json::json!(u64::MAX)),
            mutate(&|v| v["media"][1]["id"] = v["media"][0]["id"].clone()),
        ];
        for (i, text) in refused.iter().enumerate() {
            let f = dir.path().join(format!("bad{i}.vlproj"));
            std::fs::write(&f, text).unwrap();
            let before = files_in(dir.path());
            let e = load(&f).unwrap_err();
            assert!(!e.to_string().is_empty(), "{i}");
            assert_eq!(files_in(dir.path()), before, "nothing written ({i})");
        }
        // paths outside the project and to files that are not media: only ever read
        let outside = tempfile::tempdir().unwrap();
        let notes = outside.path().join("notes.txt");
        std::fs::write(&notes, b"private").unwrap();
        let text = mutate(&|v| {
            v["media"][0]["path_abs"] = serde_json::json!(notes);
            v["media"][0]["path_rel"] = serde_json::json!("../../etc/passwd");
            v["media"][1]["path_abs"] = serde_json::json!("relative/evil.mp4");
            v["media"][1]["path_rel"] = serde_json::json!("/etc/passwd");
            v["media"][0]["name"] =
                serde_json::json!(format!("a\u{202E}vom.exe\n{}", "x".repeat(5000)));
        });
        let f = dir.path().join("paths.vlproj");
        std::fs::write(&f, &text).unwrap();
        let (q, missing) = load(&f).unwrap();
        assert_eq!(q.media[0].path, notes);
        assert!(q.media[1].missing, "a relative absolute path is never used");
        assert_eq!(missing.len(), 1);
        assert!(!q.media[0].name.contains(['\n', '\u{202E}']));
        assert!(q.media[0].name.chars().count() <= 255);
        assert_eq!(std::fs::read(&notes).unwrap(), b"private");
        // too big
        let big = dir.path().join("big.vlproj");
        std::fs::File::create(&big)
            .unwrap()
            .set_len(MAX_PROJECT_BYTES + 1)
            .unwrap();
        assert_eq!(load(&big).unwrap_err(), ProjectError::TooLarge);
        // not a regular file
        assert!(matches!(load(dir.path()), Err(ProjectError::Invalid(_))));
        #[cfg(unix)]
        {
            let fifo = dir.path().join("pipe.vlproj");
            let made = std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .is_ok_and(|s| s.success());
            if made {
                let t = std::time::Instant::now();
                assert!(matches!(load(&fifo), Err(ProjectError::Invalid(_))));
                assert!(t.elapsed() < std::time::Duration::from_secs(1));
            }
        }
        let bin = dir.path().join("bin.vlproj");
        std::fs::write(&bin, [0xff, 0xfe, 0x00]).unwrap();
        assert!(load(&bin).is_err());
        // the sources were never touched
        for (path, digest) in sources {
            assert_eq!(sha(&path), digest);
        }
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
            from_json(r#"{"schema_version": 0}"#, d).unwrap_err(),
            ProjectError::UnsupportedSchema(0)
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

    /// A 0.1.x project file (schema 1), committed as a fixture.
    pub const V1_FIXTURE: &str = include_str!("../tests/fixtures/project-v1.vlproj");

    /// Opens the v1 fixture next to two dummy sources.
    pub fn open_v1(dir: &Path) -> Project {
        for n in ["a.mp4", "b.mp4"] {
            std::fs::write(dir.join(n), n).unwrap();
        }
        let file = dir.join("old.vlproj");
        std::fs::write(&file, V1_FIXTURE).unwrap();
        let (p, missing) = load(&file).unwrap();
        assert!(missing.is_empty());
        p
    }

    // covers: REQ-029
    #[test]
    fn old_projects_open_identically_and_newer_ones_are_refused_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let p = open_v1(dir.path());
        assert_eq!(p.media.len(), 2);
        assert_eq!(p.timeline.clips.len(), 3);
        assert!(p.timeline.has_fade(0));
        assert!(p.music.is_empty());
        assert_eq!(p.mix, Mix::default());
        assert!(p.subtitles.is_empty());
        // the same project built by hand gives exactly the same export
        let mut q = Project::default();
        for m in &p.media {
            q.add_media(m.clone());
        }
        q.next_media_id = 2;
        q.timeline = p.timeline.clone();
        let enc = ["libx264", "aac"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            crate::render::plan_export(&p, &enc, |_, _| true),
            crate::render::plan_export(&q, &enc, |_, _| true)
        );
        // saving writes the new format, which reopens identically
        let file = dir.path().join("old.vlproj");
        save(&p, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"schema_version\": 9"));
        assert_eq!(load(&file).unwrap().0, p);
        // a file from a newer videolore is refused and left byte-for-byte unchanged
        let newer = dir.path().join("newer.vlproj");
        let body = V1_FIXTURE.replace("\"schema_version\": 1", "\"schema_version\": 10");
        std::fs::write(&newer, &body).unwrap();
        let e = load(&newer).unwrap_err();
        assert_eq!(e, ProjectError::TooNew(10));
        assert!(e.to_string().contains("newer version of videolore"));
        assert_eq!(std::fs::read_to_string(&newer).unwrap(), body);
    }

    // covers: REQ-020, REQ-019, REQ-016
    #[test]
    fn music_mix_and_subtitles_are_saved_and_validated() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = sample(dir.path());
        let id = p.new_media_id();
        let mut song = crate::music::tests::song(id.0, 8);
        song.path = dir.path().join("song.mp3");
        std::fs::write(&song.path, b"mp3").unwrap();
        p.add_media(song.clone());
        p.music.place(&song, Micros(1_000_000), Micros(1)).unwrap();
        p.mix = Mix {
            video_volume: 40,
            video_muted: true,
            music_volume: 70,
            music_muted: false,
        };
        let tl = p.timeline.clone();
        p.subtitles
            .add_at(&tl, Micros(500_000), "hello", Micros(1))
            .unwrap();
        p.subtitles.language = Some("en".into());
        let file = dir.path().join("v2.vlproj");
        save(&p, &file).unwrap();
        let (q, _) = load(&file).unwrap();
        assert_eq!(q, p);
        assert_eq!(q.doc(), p.doc());
        // analysing items are neither saved nor part of the document
        let mut r = q.clone();
        let rid = r.new_media_id();
        r.add_media(MediaItem::analysing(rid, &dir.path().join("x.mp4")));
        assert_eq!(r.doc(), q.doc());
        assert!(!to_json(&r, &file).unwrap().contains("x.mp4"));

        let v: serde_json::Value = serde_json::from_str(&to_json(&p, &file).unwrap()).unwrap();
        let bad = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut w = v.clone();
            f(&mut w);
            from_json(&w.to_string(), dir.path())
        };
        let msg = |r: Result<(Project, Vec<String>), ProjectError>| match r {
            Err(ProjectError::Invalid(m)) => m,
            other => panic!("{other:?}"),
        };
        assert!(msg(bad(&|w| w["music"]["clips"][0]["media"] = 1.into())).contains("video"));
        assert!(msg(bad(&|w| w["music"]["clips"][0]["media"] = 77.into())).contains("unknown"));
        assert!(
            msg(bad(
                &|w| w["music"]["clips"][0]["src_out"] = 99_000_000.into()
            ))
            .contains("range")
        );
        assert!(
            msg(bad(&|w| {
                let c = w["music"]["clips"][0].clone();
                w["music"]["clips"].as_array_mut().unwrap().push(c);
            }))
            .contains("duplicate")
        );
        assert!(
            msg(bad(&|w| {
                let mut c = w["music"]["clips"][0].clone();
                c["id"] = 50.into();
                w["music"]["clips"].as_array_mut().unwrap().push(c);
            }))
            .contains("overlap")
        );
        assert!(msg(bad(&|w| w["mix"]["video_volume"] = 101.into())).contains("volume"));
        assert!(msg(bad(&|w| w["subtitles"]["lines"][0]["media"] = 77.into())).contains("unknown"));
        assert!(msg(bad(&|w| w["subtitles"]["lines"][0]["end"] = 0.into())).contains("range"));
        assert!(
            msg(bad(
                &|w| w["subtitles"]["lines"][0]["text"] = "x".repeat(1001).into()
            ))
            .contains("long")
        );
        assert!(
            msg(bad(&|w| {
                w["subtitles"]["lines"][0]["words"] =
                    serde_json::json!([{"start": 0, "end": 99_000_000, "text": "x"}]);
            }))
            .contains("word")
        );
        assert!(
            msg(bad(&|w| {
                let c = w["subtitles"]["lines"][0].clone();
                w["subtitles"]["lines"].as_array_mut().unwrap().push(c);
            }))
            .contains("duplicate")
        );
        assert!(msg(bad(&|w| w["subtitles"]["language"] = "e n!".into())).contains("language"));
        // ids are repaired upwards
        let (q, _) = bad(&|w| {
            w["music"]["next_id"] = 0.into();
            w["subtitles"]["next_id"] = 0.into();
        })
        .unwrap();
        assert_eq!(q.music.next_id, 1);
        assert_eq!(q.subtitles.next_id, 1);
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

    /// What videolore 1.1 did with the schema number (`SCHEMA_VERSION` was 2 there).
    fn opened_by_1_1(json: &str) -> Result<(), ProjectError> {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let version = v["schema_version"].as_u64().unwrap();
        if version > 2 {
            return Err(ProjectError::TooNew(version));
        }
        Ok(())
    }

    // covers: REQ-094, REQ-029
    #[test]
    fn projects_of_1_1_open_with_defaults_and_1_2_files_are_newer_for_1_1() {
        let dir = tempfile::tempdir().unwrap();
        let p = crate::render::golden_tests::open_fixture(
            dir.path(),
            crate::render::golden_tests::V2_FIXTURE,
        );
        assert_eq!(p.timeline.clips.len(), 3);
        assert_eq!(p.timeline.fades.len(), 2, "boundary fades kept");
        assert!(!p.timeline.start_fade() && !p.timeline.end_fade());
        // the video level of 1.6.0 is now the level of each clip (REQ-149)
        assert!(p.timeline.clips.iter().all(|c| c.volume == 80 && !c.muted));
        assert!(p.subtitles.lines.iter().all(|l| l.position.is_none()));
        assert!(p.media.iter().all(|m| m.format.subtitle_tracks.is_none()));
        assert_eq!(p.mix.video_volume, 100);
        // saved by 1.2: schema 3 with the new data, which 1.1 refuses (file untouched)
        let mut q = p.clone();
        q.timeline.set_edge_fades(true).unwrap();
        let first = q.timeline.clips[0].id;
        q.timeline.set_clip_volume(first, 30, true).unwrap();
        let line = q.subtitles.lines[0].id;
        q.subtitles
            .set_position(&[line], Some(crate::subtitles::Pos { x: 100, y: 200 }))
            .unwrap();
        q.media[0].format.subtitle_tracks = Some(vec![]);
        let file = dir.path().join("new.vlproj");
        save(&q, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"schema_version\": 9"));
        assert_eq!(opened_by_1_1(&text), Err(ProjectError::TooNew(9)));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        assert_eq!(
            opened_by_1_1(crate::render::golden_tests::V2_FIXTURE),
            Ok(())
        );
        assert_eq!(load(&file).unwrap().0, q);
    }

    // covers: REQ-094, REQ-057
    #[test]
    fn hostile_values_of_1_2_fields_are_refused_or_contained() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = sample(dir.path());
        let tl = p.timeline.clone();
        p.subtitles
            .add_at(&tl, Micros(100_000), "x", Micros(1))
            .unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&to_json(&p, &dir.path().join("x.vlproj")).unwrap()).unwrap();
        let try_with = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut w = v.clone();
            f(&mut w);
            from_json(&w.to_string(), dir.path())
        };
        for (what, f) in [
            (
                "volume 101",
                &(|w: &mut serde_json::Value| w["timeline"]["clips"][0]["volume"] = 101.into())
                    as &dyn Fn(&mut serde_json::Value),
            ),
            ("volume -1", &|w| {
                w["timeline"]["clips"][0]["volume"] = (-1).into()
            }),
            ("volume NaN-ish", &|w| {
                w["timeline"]["clips"][0]["volume"] = "loud".into()
            }),
            ("muted number", &|w| {
                w["timeline"]["clips"][0]["muted"] = 3.into()
            }),
            ("start fade string", &|w| {
                w["schema_version"] = 8.into();
                w["timeline"]["start_fade"] = "yes".into()
            }),
            ("pos x", &|w| {
                w["subtitles"]["lines"][0]["position"] = serde_json::json!({"x": 10001, "y": 0})
            }),
            ("pos huge", &|w| {
                w["subtitles"]["lines"][0]["position"] = serde_json::json!({"x": 1e300, "y": 0})
            }),
            ("pos negative", &|w| {
                w["subtitles"]["lines"][0]["position"] = serde_json::json!({"x": -5, "y": 0})
            }),
            ("pos text", &|w| {
                w["subtitles"]["lines"][0]["position"] = "top".into()
            }),
            ("tracks text", &|w| {
                w["media"][0]["format"]["subtitle_tracks"] = "many".into()
            }),
        ] {
            assert!(try_with(f).is_err(), "{what} must be refused");
        }
        // subtitle track descriptions are made safe, never refused
        let (q, _) = try_with(&|w| {
            let t = serde_json::json!({"ordinal": 3, "codec": "srt",
                "language": "!!", "title": format!("a\u{202E}{}", "b".repeat(5000))});
            w["media"][0]["format"]["subtitle_tracks"] = serde_json::Value::Array(vec![t; 1000]);
        })
        .unwrap();
        let t = q.media[0].format.subtitle_tracks.as_ref().unwrap();
        assert_eq!(t.len(), crate::media::MAX_SUBTITLE_TRACKS);
        assert_eq!(t[0].language, None);
        assert!(t[0].title.as_ref().unwrap().chars().count() <= crate::media::MAX_TRACK_TITLE);
        assert!(!t[0].title.as_ref().unwrap().contains('\u{202E}'));
        // edge fades on an empty timeline are harmless
        let (q, _) = try_with(&|w| {
            w["timeline"]["clips"] = serde_json::json!([]);
            w["timeline"]["fades"] = serde_json::json!([]);
            w["schema_version"] = 8.into();
            w["timeline"]["start_fade"] = true.into();
            w["subtitles"]["lines"] = serde_json::json!([]);
        })
        .unwrap();
        assert!(q.timeline.clips.is_empty());
        // a clip volume of 100 and a valid position are accepted
        let (q, _) = try_with(&|w| {
            w["timeline"]["clips"][0]["volume"] = 0.into();
            w["subtitles"]["lines"][0]["position"] = serde_json::json!({"x": 10000, "y": 0});
        })
        .unwrap();
        assert_eq!(q.timeline.clips[0].volume, 0);
    }

    /// What videolore 1.2.5 did with the schema number (`SCHEMA_VERSION` was 3 there).
    fn opened_by_1_2_5(text: &str) -> Result<(), ProjectError> {
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        match v["schema_version"].as_u64().unwrap() {
            n if n > 3 => Err(ProjectError::TooNew(n)),
            _ => Ok(()),
        }
    }

    fn v4_sample(dir: &Path) -> Project {
        let mut p = open_v1(dir);
        p.media[0].format.spherical = Some(true);
        p.media[0].format.spherical = Some(true);
        let m = p.media[0].id;
        p.camera
            .set(
                m,
                crate::camera::Pov {
                    t: Micros(1_000_000),
                    cam: crate::camera::Camera::new(9000, 500, 7000),
                },
            )
            .unwrap();
        p.subtitles.replace_all(
            vec![crate::subtitles::NewLine {
                media: m,
                start: Micros(0),
                end: Micros(1_000_000),
                text: "hi".into(),
                words: vec![],
            }],
            None,
        );
        p.subtitles.lines[0].size = Some(1500);
        p
    }

    // covers: REQ-105, REQ-029
    #[test]
    fn projects_of_1_2_5_open_unchanged_and_1_2_6_files_are_newer_for_1_2_5() {
        let dir = tempfile::tempdir().unwrap();
        let (p, _) = from_json(crate::render::golden_tests::V3_FIXTURE, dir.path()).unwrap();
        assert!(p.camera.is_empty());
        assert!(p.subtitles.lines.iter().all(|l| l.size.is_none()));
        assert!(p.media.iter().all(|m| !m.format.is_360()));
        assert_eq!(
            opened_by_1_2_5(crate::render::golden_tests::V3_FIXTURE),
            Ok(())
        );
        let q = v4_sample(dir.path());
        let file = dir.path().join("new.vlproj");
        save(&q, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"schema_version\": 9"));
        assert!(text.contains("\"camera\"") && !text.contains("\"treat_as_360\""));
        assert_eq!(opened_by_1_2_5(&text), Err(ProjectError::TooNew(9)));
        assert_eq!(load(&file).unwrap().0, q);
        // a project without points of view writes no camera section
        let plain = dir.path().join("plain.vlproj");
        save(&p, &plain).unwrap();
        assert!(!std::fs::read_to_string(&plain).unwrap().contains("camera"));
    }

    // covers: REQ-105, REQ-057
    #[test]
    fn hostile_values_of_1_2_6_fields_are_refused_or_contained() {
        let dir = tempfile::tempdir().unwrap();
        let q = v4_sample(dir.path());
        let file = dir.path().join("q.vlproj");
        save(&q, &file).unwrap();
        let good = std::fs::read_to_string(&file).unwrap();
        let load_edit = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
            f(&mut v);
            from_json(&v.to_string(), dir.path())
        };
        // sizes outside 25%..400%
        for bad in [249, 4001, 65535] {
            let r = load_edit(&|v| v["subtitles"]["lines"][0]["size"] = bad.into());
            assert!(matches!(r, Err(ProjectError::Invalid(_))), "{bad}");
        }
        assert!(load_edit(&|v| v["subtitles"]["lines"][0]["size"] = 4000.into()).is_ok());
        // camera: structure refused
        let pov = |t: i64| serde_json::json!({"t": t, "cam": {"yaw_cd": 0, "pitch_cd": 0, "fov_cd": 9000}});
        let many: Vec<_> = (0..201).map(pov).collect();
        for bad in [
            serde_json::json!([{"media": 1, "povs": many}]),
            serde_json::json!([{"media": 77, "povs": [pov(1)]}]),
            serde_json::json!([{"media": 1, "povs": [pov(-1)]}]),
            serde_json::json!([{"media": 1, "povs": [pov(9_000_000_000)]}]),
            serde_json::json!([{"media": 1, "povs": [pov(5), pov(5)]}]),
            serde_json::json!([{"media": 1, "povs": []}, {"media": 1, "povs": []}]),
            serde_json::json!("many"),
            serde_json::json!([{"media": 1, "povs": [{"t": 1.5, "cam": {}}]}]),
        ] {
            let r = load_edit(&|v| v["camera"] = bad.clone());
            assert!(matches!(r, Err(ProjectError::Invalid(_))), "{bad}");
        }
        // camera: values out of range are brought into range
        let wild = serde_json::json!([{"media": 1, "povs": [
            {"t": 1, "cam": {"yaw_cd": 99999, "pitch_cd": -99999, "fov_cd": 1}}]}]);
        let (p, _) = load_edit(&|v| v["camera"] = wild.clone()).unwrap();
        let c = p.camera.path(MediaId(1))[0].cam;
        assert!(c.is_in_range());
        // the switch of 1.3.0 is dropped whatever it holds (REQ-123)
        let r = load_edit(&|v| v["media"][0]["format"]["treat_as_360"] = "yes".into());
        assert!(r.is_ok());
    }

    // covers: REQ-113, REQ-123
    #[test]
    fn text_items_and_anchors_round_trip_and_hostile_values_are_refused() {
        use crate::text::{Anchor360, TextId, TextItem};
        let dir = tempfile::tempdir().unwrap();
        let mut q = v4_sample(dir.path());
        let id = q.texts.new_id();
        let mut t = TextItem::new(id, "Hello".into(), Micros(1_000_000));
        t.anchor = Some(Anchor360 {
            yaw_cd: 3000,
            pitch_cd: 1000,
            height_cd: 500,
            width_cd: 6000,
        });
        q.texts.add(t).unwrap();
        if let Some(l) = q.subtitles.lines.first_mut() {
            l.anchor = Some(Anchor360 {
                yaw_cd: -100,
                pitch_cd: 0,
                height_cd: 300,
                width_cd: 4000,
            });
        }
        let file = dir.path().join("t.vlproj");
        save(&q, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"texts\"") && text.contains("\"anchor\""));
        assert_eq!(load(&file).unwrap().0, q);
        // 1.2.6 refuses it, untouched
        assert_eq!(opened_by_1_2_6(&text), Err(ProjectError::TooNew(9)));
        // a project of 1.2.6 has no texts
        let old = text.replace("\"schema_version\": 6", "\"schema_version\": 4");
        let mut v: serde_json::Value = serde_json::from_str(&old).unwrap();
        v.as_object_mut().unwrap().remove("texts");
        let (p, _) = from_json(&v.to_string(), dir.path()).unwrap();
        assert!(p.texts.is_empty());
        // hostile
        let item = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
            f(&mut v["texts"][0]);
            from_json(&v.to_string(), dir.path()).map(|_| ())
        };
        assert!(item(&|_| {}).is_ok());
        type Case = Box<dyn Fn(&mut serde_json::Value)>;
        let cases: Vec<Case> = vec![
            Box::new(|t| t["ttl"] = 0.into()),
            Box::new(|t| t["ttl"] = i64::MAX.into()),
            Box::new(|t| t["effects"]["fade_in"] = 10.into()),
            Box::new(|t| t["effects"]["blink"] = 9_000_000.into()),
            Box::new(|t| t["text"] = "x".repeat(1001).into()),
            Box::new(|t| t["text"] = "".into()),
            Box::new(|t| t["width"] = 1001.into()),
            Box::new(|t| t["width"] = 99.into()),
            Box::new(|t| t["width"] = "wide".into()),
            Box::new(|t| t["font"] = "a{b".into()),
            Box::new(|t| t["colour"] = serde_json::json!([1, 2])),
            Box::new(|t| t["colour"] = serde_json::json!([1, 2, 300])),
            Box::new(|t| t["place"]["x"] = 20_000.into()),
            Box::new(|t| t["anchor"]["yaw_cd"] = 99_999.into()),
            Box::new(|t| t["anchor"]["height_cd"] = 0.into()),
            Box::new(|t| t["effects"]["slide"] = serde_json::json!(["up", 500_000])),
        ];
        for (n, f) in cases.iter().enumerate() {
            assert!(
                matches!(item(f.as_ref()), Err(ProjectError::Invalid(_))),
                "case {n}"
            );
        }
        // duplicate ids
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let first = v["texts"][0].clone();
        v["texts"].as_array_mut().unwrap().push(first);
        assert!(from_json(&v.to_string(), dir.path()).is_err());
        let _ = TextId(1);
        // a line anchor out of range is damaged too
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["subtitles"]["lines"][0]["anchor"]["pitch_cd"] = 99_999.into();
        assert!(from_json(&v.to_string(), dir.path()).is_err());
        // too new
        let new = text.replace("\"schema_version\": 9", "\"schema_version\": 10");
        assert_eq!(
            from_json(&new, dir.path()).unwrap_err(),
            ProjectError::TooNew(10)
        );
    }

    /// What videolore 1.2.6 did with the schema number (`SCHEMA_VERSION` was 4 there).
    fn opened_by_1_2_6(text: &str) -> Result<(), ProjectError> {
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        match v["schema_version"].as_u64().unwrap() {
            n if n > 4 => Err(ProjectError::TooNew(n)),
            _ => Ok(()),
        }
    }

    // covers: REQ-123
    #[test]
    fn a_project_of_1_3_0_opens_with_its_texts_migrated_and_1_4_0_data_round_trips() {
        use crate::text::{Anchor360, TextId, TextItem};
        let dir = tempfile::tempdir().unwrap();
        let mut q = v4_sample(dir.path());
        let id = q.texts.new_id();
        q.texts
            .add(TextItem::new(id, "Hello".into(), Micros(1_000_000)))
            .unwrap();
        let id2 = q.texts.new_id();
        let mut pinned = TextItem::new(id2, "Pinned text on the sphere".into(), Micros(0));
        pinned.anchor = Some(Anchor360 {
            yaw_cd: 100,
            pitch_cd: 0,
            height_cd: 700,
            width_cd: 5000,
        });
        q.texts.add(pinned).unwrap();
        let first = q.timeline.clips[0].id;
        q.timeline.set_clean_audio(&[first], true).unwrap();
        let file = dir.path().join("v6.vlproj");
        save(&q, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        // the new fields are always written, the old ones are gone
        assert!(text.contains("\"clean_audio\": true") && text.contains("\"width\""));
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(v["texts"][0].get("size").is_none() && !text.contains("size_cd"));
        assert_eq!(load(&file).unwrap().0, q);
        // the same project as 1.3.0 wrote it
        v["schema_version"] = 5.into();
        v["media"][0]["format"]["treat_as_360"] = false.into();
        for c in v["timeline"]["clips"].as_array_mut().unwrap() {
            c.as_object_mut().unwrap().remove("clean_audio");
        }
        for (i, size) in [(0, 2000), (1, 1000)] {
            let t = v["texts"][i].as_object_mut().unwrap();
            t.remove("width");
            t.insert("size".into(), size.into());
        }
        let a = v["texts"][1]["anchor"].as_object_mut().unwrap();
        a.remove("height_cd");
        a.remove("width_cd");
        a.insert("size_cd".into(), 500.into());
        a.insert("wrap_cd".into(), 3000.into());
        let (p, _) = from_json(&v.to_string(), dir.path()).unwrap();
        assert!(!p.timeline.any_clean_audio());
        // the big text of the old size gets the box that keeps its size
        let (big, pinned) = (&p.texts.items[0], &p.texts.items[1]);
        assert!((100..=1000).contains(&big.width));
        assert_eq!(big.width, crate::text::legacy_width("Hello", 2000));
        let a = pinned.anchor.unwrap();
        assert_eq!(a.width_cd, 3000, "the wrap angle is the box width");
        assert!(a.is_valid());
        // and saving writes the new format
        let out = dir.path().join("migrated.vlproj");
        save(&p, &out).unwrap();
        assert_eq!(load(&out).unwrap().0, p);
        // an old text without a valid size is damaged
        for bad in [
            serde_json::json!(5000),
            serde_json::json!("x"),
            serde_json::json!(1),
        ] {
            let mut w = v.clone();
            w["texts"][0]["size"] = bad;
            assert!(matches!(
                from_json(&w.to_string(), dir.path()),
                Err(ProjectError::Invalid(_))
            ));
        }
        let mut w = v.clone();
        w["texts"][0].as_object_mut().unwrap().remove("size");
        assert!(from_json(&w.to_string(), dir.path()).is_err());
        let mut w = v.clone();
        w["texts"][0] = 7.into();
        assert!(from_json(&w.to_string(), dir.path()).is_err());
        let _ = TextId(1);
    }

    /// What videolore 1.4.1 did with the schema number (`SCHEMA_VERSION` was 6 there).
    fn opened_by_1_4_1(text: &str) -> Result<(), ProjectError> {
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        match v["schema_version"].as_u64().unwrap() {
            n if n > 6 => Err(ProjectError::TooNew(n)),
            _ => Ok(()),
        }
    }

    fn framed(dir: &Path) -> Project {
        use crate::frame::{FrameAspect, Scale};
        use crate::keyframes::Key;
        use crate::timeline::FrameTarget;
        let mut p = sample(dir);
        let first = p.timeline.clips[0].id;
        p.timeline
            .set_frame_aspect(Some(FrameAspect::default()))
            .unwrap();
        p.timeline.add_scale(FrameTarget::Clip(first)).unwrap();
        p.timeline.add_move(FrameTarget::Whole).unwrap();
        p.timeline
            .add_scale_key(
                FrameTarget::Clip(first),
                Micros(2_000_000),
                Scale {
                    permille: 2000,
                    dx: 5,
                    dy: -5,
                },
            )
            .unwrap();
        let _: Option<Key<Scale>> = None;
        p
    }

    // covers: REQ-132
    #[test]
    fn frame_effects_are_saved_in_format_9_and_reopen_identically() {
        let dir = tempfile::tempdir().unwrap();
        let p = framed(dir.path());
        let file = dir.path().join("f.vlproj");
        save(&p, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"schema_version\": 9"));
        assert!(text.contains("\"frame_aspect\""));
        assert_eq!(load(&file).unwrap().0, p);
        // 1.4.1 and earlier refuse it
        assert_eq!(opened_by_1_4_1(&text), Err(ProjectError::TooNew(9)));
    }

    // covers: REQ-132
    #[test]
    fn a_project_of_1_4_opens_without_picture_effects() {
        let dir = tempfile::tempdir().unwrap();
        let p = sample(dir.path());
        let file = dir.path().join("f.vlproj");
        save(&p, &file).unwrap();
        let text = std::fs::read_to_string(&file)
            .unwrap()
            .replace("\"schema_version\": 9", "\"schema_version\": 6");
        assert!(!text.contains("frame_aspect") && !text.contains("clip_frames"));
        let (q, _) = from_json(&text, dir.path()).unwrap();
        assert!(!q.timeline.has_frame_effect());
        assert_eq!(q, p);
    }

    // covers: REQ-132
    // covers: REQ-057
    #[test]
    fn damaged_frame_values_are_corrected_without_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        let p = framed(dir.path());
        let file = dir.path().join("f.vlproj");
        save(&p, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let tl = |v: &serde_json::Value| from_json(&v.to_string(), dir.path()).unwrap().0.timeline;

        // a scale of 0% and of 1000% are brought to the limits
        let mut w = v.clone();
        w["timeline"]["clip_frames"][0]["scale"][0]["v"]["permille"] = 0.into();
        w["timeline"]["clip_frames"][0]["scale"][1]["v"]["permille"] = 10_000.into();
        w["timeline"]["clip_frames"][0]["moved"][0]["v"]["x"] = 65_000.into();
        let t = tl(&w);
        let id = t.clips[0].id;
        let keys = t.scale_keys(crate::timeline::FrameTarget::Clip(id));
        assert_eq!((keys[0].v.permille, keys[1].v.permille), (100, 4000));
        assert_eq!(
            t.move_keys(crate::timeline::FrameTarget::Clip(id))[0].v.x,
            10_000
        );

        // keys outside the media are dropped; equal times keep the first
        let mut w = v.clone();
        let keys = &mut w["timeline"]["clip_frames"][0]["scale"];
        let first = keys[0].clone();
        let mut late = first.clone();
        late["t"] = 999_000_000.into();
        let mut early = first.clone();
        early["t"] = (-5).into();
        keys.as_array_mut().unwrap().extend([first, late, early]);
        let t = tl(&w);
        assert_eq!(
            t.scale_keys(crate::timeline::FrameTarget::Clip(id)).len(),
            2
        );

        // more than 200 keys: the extra ones go; entries of unknown clips go
        let mut w = v.clone();
        let one = w["timeline"]["clip_frames"][0]["moved"][0].clone();
        let many: Vec<serde_json::Value> = (0..300)
            .map(|i| {
                let mut k = one.clone();
                k["t"] = (i * 1000).into();
                k
            })
            .collect();
        w["timeline"]["clip_frames"][0]["moved"] = many.into();
        let mut ghost = w["timeline"]["clip_frames"][0].clone();
        ghost["clip"] = 777.into();
        w["timeline"]["clip_frames"]
            .as_array_mut()
            .unwrap()
            .push(ghost);
        let t = tl(&w);
        assert_eq!(
            t.move_keys(crate::timeline::FrameTarget::Clip(id)).len(),
            200
        );
        assert_eq!(t.clip_frames.len(), 3);

        // an unknown shape reads as no effect
        let mut w = v.clone();
        w["timeline"]["frame_aspect"]["shape"] = "3:1".into();
        assert!(tl(&w).frame_aspect.is_none());
        // garbage types are refused as damaged files, not crashes
        let mut w = v.clone();
        w["timeline"]["clip_frames"][0]["moved"] = "lots".into();
        assert!(matches!(
            from_json(&w.to_string(), dir.path()),
            Err(ProjectError::Invalid(_))
        ));
    }

    /// What videolore 1.5.x did with the schema number (`SCHEMA_VERSION` was 7 there).
    fn opened_by_1_5(text: &str) -> Result<(), ProjectError> {
        let v: serde_json::Value = serde_json::from_str(text).unwrap();
        match v["schema_version"].as_u64().unwrap() {
            n if n > 7 => Err(ProjectError::TooNew(n)),
            _ => Ok(()),
        }
    }

    // covers: REQ-137
    #[test]
    fn apply_lut_is_saved_in_format_9_and_old_files_open_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = sample(dir.path());
        let lut = dir.path().join("warm.cube");
        std::fs::write(&lut, "LUT_1D_SIZE 2\n0 0 0\n1 1 1\n").unwrap();
        let path = lut.to_string_lossy().to_string();
        let id = p.timeline.clips[0].id;
        p.timeline
            .set_lut(crate::timeline::FrameTarget::Clip(id), Some(path.clone()))
            .unwrap();
        p.timeline
            .set_lut(crate::timeline::FrameTarget::Whole, Some(path.clone()))
            .unwrap();
        let file = dir.path().join("l.vlproj");
        save(&p, &file).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"schema_version\": 9"));
        let (q, notes) = load(&file).unwrap();
        assert_eq!(q, p);
        assert!(notes.is_empty());
        // 1.5.x refuses the file with its newer-version message and leaves it alone
        assert_eq!(opened_by_1_5(&text), Err(ProjectError::TooNew(9)));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        // a project of 1.5 (no LUT keys) opens with no colour effect
        let plain = sample(dir.path());
        save(&plain, &file).unwrap();
        let old = std::fs::read_to_string(&file)
            .unwrap()
            .replace("\"schema_version\": 9", "\"schema_version\": 7");
        assert!(!old.contains("whole_lut") && !old.contains("clip_luts"));
        let (r, _) = from_json(&old, dir.path()).unwrap();
        assert!(!r.timeline.any_lut());
        assert_eq!(r, plain);
    }

    // covers: REQ-137
    // covers: REQ-136
    // covers: REQ-057
    #[test]
    fn damaged_and_missing_lut_entries_never_crash() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = sample(dir.path());
        let id = p.timeline.clips[0].id;
        let t = |v: &serde_json::Value| from_json(&v.to_string(), dir.path());
        p.timeline
            .set_lut(
                crate::timeline::FrameTarget::Whole,
                Some("/nowhere/w.cube".into()),
            )
            .unwrap();
        p.timeline
            .set_lut(
                crate::timeline::FrameTarget::Clip(id),
                Some("/nowhere/x.cube".into()),
            )
            .unwrap();
        let file = dir.path().join("l.vlproj");
        save(&p, &file).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        // missing files: the project opens, the names are reported, the paths are kept
        let (q, notes) = t(&v).unwrap();
        assert!(q.timeline.any_lut());
        assert!(
            notes.iter().any(|n| n.contains("x.cube"))
                && notes.iter().any(|n| n.contains("w.cube"))
        );
        // damaged paths are cleared with a note
        let mut w = v.clone();
        w["schema_version"] = 8.into();
        w["timeline"]["whole_lut"] = "".into();
        w["timeline"]["clip_luts"][0]["path"] = "a\u{0}b".into();
        let mut ghost = w["timeline"]["clip_luts"][0].clone();
        ghost["clip"] = 777.into();
        w["timeline"]["clip_luts"]
            .as_array_mut()
            .unwrap()
            .push(ghost.clone());
        let (q, notes) = t(&w).unwrap();
        // the damaged entry of the first clip is gone; the other clips keep their (missing) file
        assert!(
            q.timeline
                .lut_of(crate::timeline::FrameTarget::Clip(id))
                .is_none()
        );
        assert_eq!(notes.iter().filter(|n| n.contains("damaged")).count(), 2);
        // an unknown clip and a duplicate entry are dropped
        let mut w = v.clone();
        let first = w["timeline"]["clip_luts"][0].clone();
        w["timeline"]["clip_luts"]
            .as_array_mut()
            .unwrap()
            .extend([first, ghost]);
        let (q, _) = t(&w).unwrap();
        assert_eq!(q.timeline.clip_luts.len(), q.timeline.clips.len());
        // a path that is not text is refused as a damaged file
        let mut w = v.clone();
        w["schema_version"] = 8.into();
        w["timeline"]["whole_lut"] = 5.into();
        assert!(matches!(t(&w), Err(ProjectError::Invalid(_))));
        let mut w = v.clone();
        w["timeline"]["clip_luts"][0]["path"] = serde_json::json!(["a"]);
        assert!(matches!(t(&w), Err(ProjectError::Invalid(_))));
    }
}
