// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Projects imported as media (3.2.0, REQ-371 to REQ-373): the question before the import,
//! the rendering in the background (one at a time) and the rebuild when the project that
//! holds the item is opened again. The work itself is in [`crate::nested`].

use super::*;
use crate::nested::{self, NestedError};

/// The question asked before a project is imported (REQ-372).
pub const IMPORT_PROJECT_QUESTION: &str =
    "If someone changes the project, this project may look different when it is reloaded.";

/// An item whose video is to be made (or checked) from its source project.
#[derive(Debug, Clone)]
struct Task {
    media: MediaId,
    source: PathBuf,
    /// A new item: it is removed again when it cannot be made.
    new: bool,
}

struct Run {
    task: Task,
    handle: nested::Running,
    progress: Option<f32>,
}

/// The imports waiting and the one running.
#[derive(Default)]
pub struct Imports {
    waiting: std::collections::VecDeque<Task>,
    run: Option<Run>,
}

impl Imports {
    /// An import is waiting or running.
    pub fn busy(&self) -> bool {
        self.run.is_some() || !self.waiting.is_empty()
    }

    /// The video of item `id` is still to be made or checked.
    pub fn pending(&self, id: MediaId) -> bool {
        self.run.as_ref().is_some_and(|r| r.task.media == id)
            || self.waiting.iter().any(|t| t.media == id)
    }

    /// What the media list says about item `id` while its video is not ready.
    pub fn line(&self, id: MediaId) -> Option<String> {
        if let Some(r) = self.run.as_ref().filter(|r| r.task.media == id) {
            return Some(match r.progress {
                Some(p) => format!("rendering {}%", (p.clamp(0.0, 1.0) * 100.0) as u32),
                None => "analysing the project…".into(),
            });
        }
        self.waiting
            .iter()
            .any(|t| t.media == id)
            .then(|| "waiting to be rendered".into())
    }

    /// Progress of the running rendering, for the status bar.
    pub(super) fn progress(&self) -> Option<Option<f32>> {
        self.run.as_ref().map(|r| r.progress)
    }

    fn remove(&mut self, id: MediaId) {
        self.waiting.retain(|t| t.media != id);
        if self.run.as_ref().is_some_and(|r| r.task.media == id) {
            self.run = None; // dropping the handle stops it
        }
    }
}

impl AppState {
    /// File > Import project as media…, once the file is chosen: checks it and everything it
    /// imports, then asks the question of REQ-372. A refusal is told in a message.
    pub(super) fn ask_import_project(&mut self, path: PathBuf) -> Result<(), String> {
        self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        if let Err(e) = nested::read(&path, self.project_path.as_deref()) {
            let e = e.to_string();
            self.notice = Some(e.clone());
            return Err(e);
        }
        self.modal = Some(Modal::ImportProject(path));
        Ok(())
    }

    /// "OK": the item is added and its video is made in the background.
    pub(super) fn confirm_import_project(&mut self) -> Result<(), String> {
        let Some(Modal::ImportProject(path)) = self.modal.clone() else {
            return Ok(());
        };
        self.modal = None;
        let source = std::path::absolute(&path).map_err(err)?;
        let id = self.project.new_media_id();
        let mut item = MediaItem::analysing(id, &source);
        item.from = Some(source.clone());
        self.status = format!("Rendering {}…", item.name);
        self.project.add_media(item);
        self.imports.waiting.push_back(Task {
            media: id,
            source,
            new: true,
        });
        self.start_next_import();
        Ok(())
    }

    /// The items of the opened project that follow a source project are checked and, when
    /// their video is not the one of the source as it is now, made again.
    pub(super) fn queue_rebuilds(&mut self) {
        for m in &self.project.media {
            if let Some(source) = &m.from {
                self.imports.waiting.push_back(Task {
                    media: m.id,
                    source: source.clone(),
                    new: false,
                });
            }
        }
    }

    /// Stops the rendering of imported projects: new items are removed.
    pub(super) fn cancel_imports(&mut self) {
        let imports = std::mem::take(&mut self.imports);
        let tasks = imports.run.iter().map(|r| &r.task).chain(&imports.waiting);
        for t in tasks {
            if t.new {
                self.project.media.retain(|m| m.id != t.media);
            }
        }
        self.status = "Import cancelled".into();
    }

    pub(super) fn forget_import(&mut self, id: MediaId) {
        self.imports.remove(id);
    }

    fn start_next_import(&mut self) {
        if self.imports.run.is_some() {
            return;
        }
        let Ok(info) = &self.ffmpeg else {
            return; // started when ffmpeg is ready
        };
        let tools = nested::Tools {
            ffmpeg: info.ffmpeg.clone(),
            ffprobe: info.ffprobe.clone(),
            version: info.version,
            encoders: info.encoders.clone(),
            can_burn_subtitles: info.has_subtitles(),
            threads: self.budget.threads,
            work_root: self.work_root.clone(),
        };
        let Some(task) = self.imports.waiting.pop_front() else {
            return;
        };
        // the rendering is an export of the job queue
        let runner = self.limited(Priority::Export);
        let handle = nested::start(
            task.source.clone(),
            self.project_path.clone(),
            tools,
            runner,
        );
        self.imports.run = Some(Run {
            task,
            handle,
            progress: None,
        });
    }

    pub(super) fn tick_imports(&mut self, now: Instant) {
        let events: Vec<nested::Event> = match &self.imports.run {
            Some(r) => r.handle.rx.try_iter().collect(),
            None => Vec::new(),
        };
        for e in events {
            match e {
                nested::Event::Progress(p) => {
                    if let Some(r) = &mut self.imports.run {
                        r.progress = Some(p);
                    }
                }
                nested::Event::Done(item) => {
                    if let Some(r) = self.imports.run.take() {
                        self.import_done(r.task, *item, now);
                    }
                }
                nested::Event::Failed(e) => {
                    if let Some(r) = self.imports.run.take() {
                        self.import_failed(r.task, &e, now);
                    }
                }
            }
        }
        self.start_next_import();
    }

    fn import_done(&mut self, task: Task, mut item: MediaItem, now: Instant) {
        let id = task.media;
        let Some(old) = self.project.media(id) else {
            return;
        };
        if task.new {
            item.id = id;
            self.status = format!("Imported {}", item.name);
            if let Some(slot) = self.project.media.iter_mut().find(|m| m.id == id) {
                *slot = item;
            }
            if self.selected_media.is_none() {
                self.selected_media = Some(id);
            }
        } else {
            let same = !old.missing
                && old.path == item.path
                && old.duration == item.duration
                && old.format == item.format;
            if same {
                return;
            }
            self.status = format!("{} was rendered again from its project", item.name);
            self.cancel_proxy(id);
            let length = item.duration;
            nested::replace_media(&mut self.project, id, item);
            // an undo step must not bring back a clip longer than the new video
            self.history.forget(|e| {
                e.timeline
                    .clips
                    .iter()
                    .any(|c| c.media == id && c.src_out > length)
            });
            self.strips.forget(id);
            self.rows_serial += 1;
        }
        self.maybe_proxy(id);
        self.preview.invalidate(now);
        let at = self.playhead;
        self.seek(at, now);
        self.refresh_dirty();
    }

    fn import_failed(&mut self, task: Task, e: &NestedError, now: Instant) {
        let name = self
            .project
            .media(task.media)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        if task.new {
            self.project.media.retain(|m| m.id != task.media);
        } else if let Some(m) = self.project.media.iter_mut().find(|m| m.id == task.media) {
            // the item is refused until its source can be read again
            m.missing = true;
            self.preview.invalidate(now);
        }
        self.status = format!("{name} could not be rendered");
        self.notice = Some(e.to_string());
        self.refresh_dirty();
    }

    /// The name of an item on the timeline whose video is not ready yet.
    pub(super) fn waiting_import(&self) -> Option<String> {
        let tl = &self.project.timeline;
        tl.clips
            .iter()
            .map(|c| c.media)
            .find(|m| self.imports.pending(*m))
            .and_then(|m| self.project.media(m))
            .map(|m| m.name.clone())
    }
}
