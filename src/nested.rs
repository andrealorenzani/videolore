// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! A project imported as media (3.2.0, REQ-371 to REQ-373, ADR-279).
//!
//! The imported item is a normal video media whose file is rendered from the source project
//! by the export pipeline ([`crate::render::ExportMode::Intermediate`]) into the private
//! folder [`FOLDER`] of the work folder. The item keeps a link to the source project: its
//! file name is a [key](Source::key) of the project, so a changed project gives a new file.
//!
//! The source project is untrusted input: it is read with the normal bounded loader, the
//! chain of imported projects is limited ([`MAX_DEPTH`], [`MAX_PROJECTS`]) and a project
//! that imports itself, directly or through another one, is refused.

use crate::base::io::read_limited;
use crate::export::{self, ExportEvent, ExportJob};
use crate::ffmpeg::{FfVersion, Runner};
use crate::jobs::Limited;
use crate::media::{self, MediaId, MediaItem};
use crate::project::{self, MAX_PROJECT_BYTES, Project};
use crate::time::Micros;
use crate::timeline::Timeline;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

/// Most projects imported inside one another.
pub const MAX_DEPTH: usize = 8;
/// Most projects read for one import (a project may import several).
pub const MAX_PROJECTS: usize = 64;
/// The folder of the rendered videos, inside the work folder.
pub const FOLDER: &str = "imports";
/// Part of the key: raise it when the rendered video changes for the same project.
const KEY_VERSION: &[u8] = b"intermediate-1";
/// The shortest clip a rebuilt item leaves.
const MIN_CLIP: Micros = Micros(40_000);

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum NestedError {
    #[error("'{name}' cannot be imported: {why}")]
    Unreadable { name: String, why: String },
    #[error("'{project}' cannot be imported: its file '{file}' is missing or cannot be used")]
    Missing { project: String, file: String },
    #[error("'{0}' cannot be imported: there is no video on its timeline")]
    Empty(String),
    #[error("a project cannot import itself, directly or through another project ('{0}')")]
    Itself(String),
    #[error("'{0}' cannot be imported: projects are imported inside one another more than 8 deep")]
    TooDeep(String),
    #[error("'{0}' cannot be imported: it holds more than 64 imported projects")]
    TooMany(String),
    #[error("'{name}' could not be rendered: {why}")]
    Render { name: String, why: String },
    #[error("cancelled")]
    Cancelled,
}

/// A source project read and checked.
#[derive(Debug)]
pub struct Source {
    /// The absolute path of the project file.
    pub path: PathBuf,
    pub name: String,
    pub project: Project,
    /// The imported projects its timeline uses, by media id.
    pub children: Vec<(MediaId, Source)>,
    /// 32 hex digits naming the rendered video: the project file, where it is, the size and
    /// time of the files it uses, and the keys of its children.
    pub key: String,
}

/// Reads the project at `path` and everything it imports. `importer` is the file of the
/// project that imports it (`None`: never saved).
pub fn read(path: &Path, importer: Option<&Path>) -> Result<Source, NestedError> {
    let mut chain: Vec<PathBuf> = importer.map(identity).into_iter().collect();
    let depth_of_importer = chain.len();
    let mut count = 0;
    walk(path, &mut chain, depth_of_importer, &mut count)
}

/// The same file however it is written.
fn identity(p: &Path) -> PathBuf {
    p.canonicalize()
        .or_else(|_| std::path::absolute(p))
        .unwrap_or_else(|_| p.to_path_buf())
}

fn walk(
    path: &Path,
    chain: &mut Vec<PathBuf>,
    base: usize,
    count: &mut usize,
) -> Result<Source, NestedError> {
    let name = media::display_name(path);
    let id = identity(path);
    if chain.contains(&id) {
        return Err(NestedError::Itself(name));
    }
    if chain.len() - base >= MAX_DEPTH {
        return Err(NestedError::TooDeep(name));
    }
    *count += 1;
    if *count > MAX_PROJECTS {
        return Err(NestedError::TooMany(name));
    }
    let unreadable = |why: String| NestedError::Unreadable {
        name: name.clone(),
        why,
    };
    let abs = std::path::absolute(path).map_err(|e| unreadable(e.to_string()))?;
    let bytes = read_limited(&abs, MAX_PROJECT_BYTES).map_err(|e| unreadable(e.to_string()))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| unreadable("it is not a project file".into()))?;
    let (project, _) = project::from_json(text, &crate::base::io::parent_dir(&abs))
        .map_err(|e| unreadable(e.to_string()))?;

    let used: BTreeSet<MediaId> = project
        .timeline
        .clips
        .iter()
        .map(|c| c.media)
        .chain(project.music.clips.iter().map(|c| c.media))
        .collect();
    chain.push(id);
    let mut children = Vec::new();
    for m in &project.media {
        match &m.from {
            Some(from) if used.contains(&m.id) => {
                children.push((m.id, walk(from, chain, base, count)?));
            }
            // a link that is not on the timeline still may not lead back (REQ-373)
            Some(from) => {
                if let Err(
                    e
                    @ (NestedError::Itself(_) | NestedError::TooDeep(_) | NestedError::TooMany(_)),
                ) = walk(from, chain, base, count)
                {
                    return Err(e);
                }
            }
            None => {}
        }
    }
    chain.pop();

    if project.timeline.clips.is_empty() {
        return Err(NestedError::Empty(name));
    }
    let mut hash = Sha256::new();
    hash.update(KEY_VERSION);
    hash.update(abs.as_os_str().as_encoded_bytes());
    hash.update([0]);
    hash.update(&bytes);
    for file in files_used(&project, &used).map_err(|file| NestedError::Missing {
        project: name.clone(),
        file,
    })? {
        // a file replaced on disk makes the video again
        let stat = media::FileStat::of(&file);
        hash.update(stat.map_or(0, |s| s.size).to_le_bytes());
        hash.update(stat.map_or(0, |s| s.modified_ms).to_le_bytes());
    }
    for (_, child) in &children {
        hash.update(child.key.as_bytes());
    }
    let key = hash
        .finalize()
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(Source {
        path: abs,
        name,
        project,
        children,
        key,
    })
}

/// The files the timeline of `project` reads (imported projects apart), or the name of one
/// that is missing or cannot be used.
fn files_used(project: &Project, used: &BTreeSet<MediaId>) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for m in &project.media {
        if m.from.is_none() && used.contains(&m.id) {
            if m.missing {
                return Err(m.name.clone());
            }
            files.push(m.path.clone());
        }
    }
    for t in &project.texts.items {
        if let Some(p) = &t.picture {
            let path = PathBuf::from(&p.path);
            if !crate::picture::usable(&path) {
                return Err(p.name());
            }
            files.push(path);
        }
    }
    for o in project.timeline.clip_overlaps.iter().flat_map(|c| &c.items) {
        let path = PathBuf::from(&o.path);
        if !crate::picture::overlay_usable(&path) {
            return Err(o.name());
        }
        files.push(path);
    }
    for l in &project.timeline.clip_luts {
        if crate::lut::cached(&l.path).is_err() {
            return Err(crate::lut::display_name(&l.path));
        }
    }
    Ok(files)
}

/// What rendering needs of the ffmpeg of the program.
#[derive(Debug, Clone)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub version: FfVersion,
    pub encoders: BTreeSet<String>,
    pub can_burn_subtitles: bool,
    pub threads: Option<u32>,
    pub work_root: PathBuf,
}

/// Where the video named `key` is kept.
pub fn intermediate(work_root: &Path, key: &str) -> PathBuf {
    work_root.join(FOLDER).join(format!("{key}.mkv"))
}

/// Gives media `id` of `project` the file and format of `new` (the id stays) and fits its
/// clips into the new length: a clip that used the source up to its end still does, the
/// others are shortened only when the source became shorter.
pub fn replace_media(project: &mut Project, id: MediaId, mut new: MediaItem) {
    let Some(slot) = project.media.iter_mut().find(|m| m.id == id) else {
        return;
    };
    let old = slot.duration;
    new.id = id;
    let len = new.duration;
    *slot = new;
    refit(&mut project.timeline, id, old, len);
}

/// The clip ranges of media `id` after its length changed from `old` to `new`.
pub fn refit(timeline: &mut Timeline, id: MediaId, old: Micros, new: Micros) {
    if old == new {
        return;
    }
    let clips: Vec<_> = timeline
        .clips
        .iter()
        .filter(|c| c.media == id)
        .copied()
        .collect();
    for c in clips {
        let out = if c.src_out >= old || c.src_out > new {
            new
        } else {
            c.src_out
        };
        let min = MIN_CLIP.min(new);
        let start = if c.src_in + min <= out {
            c.src_in
        } else {
            // the clip keeps its length where the source allows it
            (out - c.src_len()).max(Micros::ZERO)
        };
        // the overlaps of the clip follow the kept part (REQ-329)
        let _ = timeline.trim(c.id, start, out, new, Micros(1));
    }
}

/// Makes (or finds) the video of `source` and returns it as a media item.
/// `progress` goes from 0 to 1 over all the videos to make.
pub fn render(
    source: Source,
    tools: &Tools,
    runner: &Limited,
    cancel: &Arc<AtomicBool>,
    progress: &mut dyn FnMut(f32),
) -> Result<MediaItem, NestedError> {
    let total = count(&source);
    let mut done = 0;
    build(
        source,
        tools,
        &runner.cancellable(Arc::clone(cancel)),
        cancel,
        &mut |p, finished| {
            progress(((done as f32 + p) / total as f32).clamp(0.0, 1.0));
            done += usize::from(finished);
        },
    )
}

fn count(s: &Source) -> usize {
    1 + s.children.iter().map(|(_, c)| count(c)).sum::<usize>()
}

fn build(
    source: Source,
    tools: &Tools,
    runner: &Limited,
    cancel: &Arc<AtomicBool>,
    progress: &mut dyn FnMut(f32, bool),
) -> Result<MediaItem, NestedError> {
    let Source {
        path,
        name,
        mut project,
        children,
        key,
    } = source;
    for (id, child) in children {
        let item = build(child, tools, runner, cancel, progress)?;
        replace_media(&mut project, id, item);
    }
    let failed = |why: String| NestedError::Render {
        name: name.clone(),
        why,
    };
    let target = intermediate(&tools.work_root, &key);
    let probe = || media::probe(runner, &tools.ffprobe, &target, MediaId(0));
    let mut item = match target.is_file().then(probe) {
        Some(Ok(item)) => item,
        // not there, or damaged
        _ => {
            crate::base::private::private_dir(&tools.work_root.join(FOLDER))
                .map_err(|e| failed(e.to_string()))?;
            export_to(&project, &target, tools, runner, cancel, &mut |p| {
                progress(p, false)
            })
            .map_err(|e| match e {
                None => NestedError::Cancelled,
                Some(why) => failed(why),
            })?;
            probe().map_err(|e| failed(e.to_string()))?
        }
    };
    if cancel.load(Ordering::SeqCst) {
        return Err(NestedError::Cancelled);
    }
    // the length of the item is that of the source timeline (the file may end a little later)
    item.duration = item.duration.min(project.timeline.duration());
    item.name = name;
    item.from = Some(path);
    progress(1.0, true);
    Ok(item)
}

/// Runs the export of `project` to `target`; `Err(None)`: cancelled.
fn export_to(
    project: &Project,
    target: &Path,
    tools: &Tools,
    runner: &Limited,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<(), Option<String>> {
    let job = ExportJob {
        project: project.clone(),
        output: target.to_path_buf(),
        ffmpeg: tools.ffmpeg.clone(),
        ffprobe: tools.ffprobe.clone(),
        version: tools.version,
        encoders: tools.encoders.clone(),
        can_burn_subtitles: tools.can_burn_subtitles,
        threads: tools.threads,
        work_root: tools.work_root.clone(),
        hw: None,
        stall_timeout: export::STALL_TIMEOUT,
        check_timeout: export::CHECK_TIMEOUT,
        mode: crate::render::ExportMode::Intermediate,
        format: Default::default(),
        audio_only: false,
    };
    let runner: Arc<dyn Runner> = Arc::new(runner.clone());
    let handle = export::start(job, runner);
    loop {
        if cancel.load(Ordering::SeqCst) {
            handle.cancel();
        }
        match handle.rx.recv_timeout(Duration::from_millis(20)) {
            Ok(ExportEvent::Progress(p)) => progress(p),
            Ok(ExportEvent::Started { .. }) => {}
            Ok(ExportEvent::Done { .. }) => return Ok(()),
            Ok(ExportEvent::Failed(m)) => return Err(Some(m)),
            Ok(ExportEvent::Cancelled) => return Err(None),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(Some("the rendering stopped".into()));
            }
        }
    }
}

/// What a running import tells.
#[derive(Debug)]
pub enum Event {
    /// Fraction of the rendering done.
    Progress(f32),
    Done(Box<MediaItem>),
    Failed(NestedError),
}

/// An import running in the background; dropping it stops it.
pub struct Running {
    pub rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

/// Reads the project at `path`, makes its video and sends the result.
pub fn start(path: PathBuf, importer: Option<PathBuf>, tools: Tools, runner: Limited) -> Running {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    std::thread::spawn(move || work(&path, importer.as_deref(), &tools, &runner, &flag, &tx));
    Running { rx, cancel }
}

fn work(
    path: &Path,
    importer: Option<&Path>,
    tools: &Tools,
    runner: &Limited,
    cancel: &Arc<AtomicBool>,
    tx: &Sender<Event>,
) {
    let result = read(path, importer).and_then(|source| {
        render(source, tools, runner, cancel, &mut |p| {
            let _ = tx.send(Event::Progress(p));
        })
    });
    let _ = tx.send(match result {
        Ok(item) => Event::Done(Box::new(item)),
        Err(e) => Event::Failed(e),
    });
}

#[cfg(test)]
pub(crate) mod tests;
