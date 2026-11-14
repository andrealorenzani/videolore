// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Application state and the single command dispatcher: every button, menu entry and
//! shortcut ends up in [`AppState::dispatch`].

use crate::export::{self, ExportEvent, ExportHandle, ExportJob};
use crate::ffmpeg::{self, FfmpegInfo, Runner};
use crate::history::History;
use crate::media::{self, MediaId};
use crate::preview::{AudioSink, Frame, Preview, PreviewConfig};
use crate::project::{self, Project};
use crate::render;
use crate::time::{Fps, Micros};
use crate::timeline::{Clip, ClipId, EditError, Timeline};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Every user operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    NewProject,
    PickOpenProject,
    OpenProject(PathBuf),
    SaveProject,
    PickSaveProjectAs,
    SaveProjectAs(PathBuf),
    PickImport,
    Import(Vec<PathBuf>),
    SelectMedia(Option<MediaId>),
    /// Appends the given media (or the selected one) at the end of the track.
    Append(Option<MediaId>),
    SelectClip(Option<ClipId>),
    Split,
    Cut,
    Copy,
    Paste,
    Delete,
    Merge,
    SetTrim {
        clip: ClipId,
        src_in: Micros,
        src_out: Micros,
    },
    MoveClip {
        clip: ClipId,
        to: usize,
    },
    MoveLeft,
    MoveRight,
    /// Toggles the fade at boundary `b` (between clips `b` and `b + 1`).
    ToggleFade(usize),
    /// Toggles the fade at the boundary nearest to the playhead.
    ToggleFadeAtPlayhead,
    Undo,
    Redo,
    PlayPause,
    Seek(Micros),
    StepFrames(i64),
    SetZoom(f32),
    PickExport,
    Export(PathBuf),
    CancelExport,
    PickFfmpeg,
    SetFfmpeg(PathBuf),
    RetryFfmpeg,
    DismissNotice,
}

/// Native file dialogs (faked in tests).
pub trait Dialogs {
    fn pick_media(&mut self) -> Vec<PathBuf>;
    fn pick_project_open(&mut self) -> Option<PathBuf>;
    fn pick_project_save(&mut self) -> Option<PathBuf>;
    fn pick_export(&mut self, extension: &str) -> Option<PathBuf>;
    fn pick_ffmpeg(&mut self) -> Option<PathBuf>;
}

/// User settings persisted between runs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
}

impl Settings {
    /// Default location: the platform config folder.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "videolore")
            .map(|d| d.config_dir().join("settings.json"))
    }

    pub fn load(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        project::write_atomic(path, json.as_bytes()).map_err(|e| e.to_string())
    }
}

/// State of a running or finished export, for the UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportStatus {
    pub running: bool,
    pub progress: f32,
    pub stream_copy: Option<bool>,
}

pub struct AppState {
    pub project: Project,
    pub project_path: Option<PathBuf>,
    pub dirty: bool,
    pub history: History<Timeline>,
    pub selected_clip: Option<ClipId>,
    pub selected_media: Option<MediaId>,
    pub playhead: Micros,
    pub clipboard: Option<Clip>,
    /// Timeline zoom in pixels per second (0 = fit all on next frame).
    pub zoom: f32,
    pub ffmpeg: Result<FfmpegInfo, String>,
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    runner: Arc<dyn Runner>,
    dialogs: Box<dyn Dialogs>,
    pub preview: Preview,
    /// Latest preview frame and a counter that changes with it.
    pub frame: Option<Frame>,
    pub frame_serial: u64,
    export: Option<ExportHandle>,
    pub export_status: ExportStatus,
    /// One-line status message.
    pub status: String,
    /// Message shown in a modal dialog until dismissed.
    pub notice: Option<String>,
}

fn check_ffmpeg(runner: &dyn Runner, settings: &Settings) -> Result<FfmpegInfo, String> {
    let (ff, fp) = ffmpeg::locate_both(settings.ffmpeg.as_deref(), settings.ffprobe.as_deref())
        .map_err(|e| e.to_string())?;
    ffmpeg::check(runner, &ff, &fp).map_err(|e| e.to_string())
}

impl AppState {
    /// Real start-up: loads settings and checks the ffmpeg installation.
    pub fn new(
        runner: Arc<dyn Runner>,
        dialogs: Box<dyn Dialogs>,
        sink: Box<dyn AudioSink>,
        settings_path: Option<PathBuf>,
    ) -> AppState {
        let settings = settings_path
            .as_deref()
            .map(Settings::load)
            .unwrap_or_default();
        let info = check_ffmpeg(runner.as_ref(), &settings);
        let mut s = AppState::with_ffmpeg(info, runner, dialogs, sink);
        s.settings = settings;
        s.settings_path = settings_path;
        s
    }

    /// Start-up with an already known ffmpeg status (tests).
    pub fn with_ffmpeg(
        ffmpeg: Result<FfmpegInfo, String>,
        runner: Arc<dyn Runner>,
        dialogs: Box<dyn Dialogs>,
        sink: Box<dyn AudioSink>,
    ) -> AppState {
        let config = preview_config(&ffmpeg, &runner);
        AppState {
            project: Project::default(),
            project_path: None,
            dirty: false,
            history: History::default(),
            selected_clip: None,
            selected_media: None,
            playhead: Micros::ZERO,
            clipboard: None,
            zoom: 0.0,
            ffmpeg,
            settings: Settings::default(),
            settings_path: None,
            runner,
            dialogs,
            preview: Preview::new(config, sink),
            frame: None,
            frame_serial: 0,
            export: None,
            export_status: ExportStatus::default(),
            status: String::new(),
            notice: None,
        }
    }

    /// Output frame rate (frame grid for the playhead, split and zoom).
    pub fn fps(&self) -> Fps {
        let enc = self.encoders();
        render::output_geometry(&self.project, &enc)
            .map(|g| g.fps)
            .unwrap_or(Fps { num: 30, den: 1 })
    }

    fn encoders(&self) -> std::collections::BTreeSet<String> {
        self.ffmpeg
            .as_ref()
            .map(|i| i.encoders.clone())
            .unwrap_or_default()
    }

    /// Description of the export format (for the settings panel).
    pub fn output_description(&self) -> Option<String> {
        let media = render::timeline_media(&self.project).ok()?;
        let formats: Vec<_> = media.iter().map(|m| &m.format).collect();
        let chosen = crate::format::choose_output(&formats)?;
        let s = crate::format::encode_settings(&chosen, &self.encoders());
        let same = crate::format::all_identical(&formats);
        Some(format!(
            "{}x{} {} fps, {} + {}, .{}{}",
            s.width,
            s.height,
            s.fps,
            s.vencoder,
            s.aencoder,
            s.extension,
            if same { " (same as sources)" } else { "" }
        ))
    }

    pub fn is_exporting(&self) -> bool {
        self.export_status.running
    }

    /// The clip commands act on: the selection, else the clip under the playhead.
    pub fn target_clip(&self) -> Option<ClipId> {
        self.selected_clip
            .filter(|id| self.project.timeline.get(*id).is_some())
            .or_else(|| {
                self.project
                    .timeline
                    .locate(self.playhead)
                    .map(|(i, _)| self.project.timeline.clips[i].id)
            })
    }

    /// Whether a command can run now (used to disable buttons).
    pub fn is_enabled(&self, cmd: &Command) -> bool {
        let tl = &self.project.timeline;
        let has_ff = self.ffmpeg.is_ok();
        match cmd {
            Command::Undo => self.history.can_undo(),
            Command::Redo => self.history.can_redo(),
            Command::Split => tl.locate(self.playhead).is_some(),
            Command::Cut | Command::Copy | Command::Delete => self.target_clip().is_some(),
            Command::Paste => self.clipboard.is_some(),
            Command::Merge => self.target_clip().is_some_and(|c| tl.can_merge(c)),
            Command::MoveLeft => self
                .target_clip()
                .and_then(|c| tl.index_of(c))
                .is_some_and(|i| i > 0),
            Command::MoveRight => self
                .target_clip()
                .and_then(|c| tl.index_of(c))
                .is_some_and(|i| i + 1 < tl.clips.len()),
            Command::ToggleFadeAtPlayhead => tl.clips.len() > 1,
            Command::Append(id) => id
                .or(self.selected_media)
                .and_then(|id| self.project.media(id))
                .is_some_and(|m| m.has_video() && !m.missing),
            Command::PlayPause => has_ff && !tl.clips.is_empty(),
            Command::PickExport | Command::Export(_) => {
                has_ff && !tl.clips.is_empty() && !self.is_exporting()
            }
            Command::CancelExport => self.is_exporting(),
            Command::PickImport | Command::Import(_) => has_ff,
            _ => true,
        }
    }

    /// Runs a command. Errors are also put in the status line.
    pub fn dispatch(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        let r = self.dispatch_inner(cmd, now);
        if let Err(e) = &r {
            self.status = e.clone();
        }
        r
    }

    fn dispatch_inner(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        match cmd {
            Command::NewProject => {
                self.load_project(Project::default(), None, now);
                self.status = "New project".into();
                Ok(())
            }
            Command::PickOpenProject => match self.dialogs.pick_project_open() {
                Some(p) => self.dispatch_inner(Command::OpenProject(p), now),
                None => Ok(()),
            },
            Command::OpenProject(path) => {
                let (p, missing) = project::load(&path).map_err(|e| e.to_string())?;
                self.load_project(p, Some(path), now);
                if !missing.is_empty() {
                    self.notice = Some(format!(
                        "These source files could not be found:\n{}",
                        missing.join("\n")
                    ));
                }
                self.status = "Project opened".into();
                Ok(())
            }
            Command::SaveProject => match self.project_path.clone() {
                Some(p) => self.save_to(p),
                None => self.dispatch_inner(Command::PickSaveProjectAs, now),
            },
            Command::PickSaveProjectAs => match self.dialogs.pick_project_save() {
                Some(p) => self.dispatch_inner(Command::SaveProjectAs(p), now),
                None => Ok(()),
            },
            Command::SaveProjectAs(mut p) => {
                if p.extension().is_none() {
                    p.set_extension(project::EXTENSION);
                }
                self.save_to(p)
            }
            Command::PickImport => {
                let files = self.dialogs.pick_media();
                if files.is_empty() {
                    return Ok(());
                }
                self.dispatch_inner(Command::Import(files), now)
            }
            Command::Import(files) => self.import(files),
            Command::SelectMedia(id) => {
                self.selected_media = id;
                Ok(())
            }
            Command::Append(id) => {
                let id = id
                    .or(self.selected_media)
                    .ok_or("select a file in the media list first")?;
                let m = self.project.media(id).cloned().ok_or("no such media")?;
                self.edit(now, |tl| tl.append(&m).map(|_| ()))
            }
            Command::SelectClip(id) => {
                self.selected_clip = id;
                Ok(())
            }
            Command::Split => {
                let min = self.fps().frame_duration();
                let at = self.playhead;
                self.edit(now, |tl| tl.split(at, min).map(|_| ()))
            }
            Command::Copy => {
                let id = self
                    .target_clip()
                    .ok_or(EditError::NothingSelected.to_string())?;
                self.clipboard = self.project.timeline.get(id).copied();
                self.status = "Clip copied".into();
                Ok(())
            }
            Command::Cut | Command::Delete => {
                let id = self
                    .target_clip()
                    .ok_or(EditError::NothingSelected.to_string())?;
                let clip = self.project.timeline.get(id).copied();
                self.edit(now, |tl| tl.remove(id).map(|_| ()))?;
                if cmd == Command::Cut {
                    self.clipboard = clip;
                }
                self.selected_clip = None;
                Ok(())
            }
            Command::Paste => {
                let clip = self
                    .clipboard
                    .ok_or(EditError::EmptyClipboard.to_string())?;
                if self.project.media(clip.media).is_none() {
                    return Err("the copied clip's source is no longer in the project".into());
                }
                let min = self.fps().frame_duration();
                let at = self.playhead;
                let mut new_id = None;
                self.edit(now, |tl| {
                    new_id = Some(tl.insert_at(&clip, at, min)?);
                    Ok(())
                })?;
                self.selected_clip = new_id;
                Ok(())
            }
            Command::Merge => {
                let id = self
                    .target_clip()
                    .ok_or(EditError::NothingSelected.to_string())?;
                self.edit(now, |tl| tl.merge(id))
            }
            Command::SetTrim {
                clip,
                src_in,
                src_out,
            } => {
                let c = self
                    .project
                    .timeline
                    .get(clip)
                    .ok_or(EditError::NoSuchClip.to_string())?;
                let dur = self
                    .project
                    .media(c.media)
                    .map(|m| m.duration)
                    .ok_or("no such media")?;
                let min = self.fps().frame_duration();
                self.edit(now, |tl| tl.trim(clip, src_in, src_out, dur, min))
            }
            Command::MoveClip { clip, to } => self.edit(now, |tl| tl.move_to(clip, to)),
            Command::MoveLeft | Command::MoveRight => {
                let id = self
                    .target_clip()
                    .ok_or(EditError::NothingSelected.to_string())?;
                let i = self
                    .project
                    .timeline
                    .index_of(id)
                    .ok_or(EditError::NoSuchClip.to_string())?;
                let to = if cmd == Command::MoveLeft {
                    i.checked_sub(1).ok_or("already the first clip")?
                } else {
                    i + 1
                };
                self.selected_clip = Some(id);
                self.edit(now, |tl| tl.move_to(id, to))
            }
            Command::ToggleFade(b) => self.edit(now, |tl| tl.toggle_fade(b).map(|_| ())),
            Command::ToggleFadeAtPlayhead => {
                let (b, _) = self
                    .project
                    .timeline
                    .nearest_boundary(self.playhead)
                    .ok_or(EditError::NoSuchBoundary.to_string())?;
                self.edit(now, |tl| tl.toggle_fade(b).map(|_| ()))
            }
            Command::Undo => {
                let prev = self
                    .history
                    .undo(self.project.timeline.clone())
                    .ok_or(EditError::NothingToUndo.to_string())?;
                self.replace_timeline(prev, now);
                Ok(())
            }
            Command::Redo => {
                let next = self
                    .history
                    .redo(self.project.timeline.clone())
                    .ok_or(EditError::NothingToRedo.to_string())?;
                self.replace_timeline(next, now);
                Ok(())
            }
            Command::PlayPause => {
                if self.preview.is_playing() {
                    self.preview.pause(now);
                    self.playhead = self.preview.position(now);
                    Ok(())
                } else {
                    self.preview.play(&self.project, self.playhead, now)
                }
            }
            Command::Seek(t) => {
                self.seek(t, now);
                Ok(())
            }
            Command::StepFrames(n) => {
                let f = self.fps();
                let t = f.frame_time(f.frame_index(self.playhead) + n);
                self.seek(t, now);
                Ok(())
            }
            Command::SetZoom(z) => {
                self.zoom = z.max(0.0);
                Ok(())
            }
            Command::PickExport => {
                let ext = self.export_extension();
                match self.dialogs.pick_export(&ext) {
                    Some(p) => self.dispatch_inner(Command::Export(p), now),
                    None => Ok(()),
                }
            }
            Command::Export(path) => self.start_export(path),
            Command::CancelExport => {
                if let Some(h) = &self.export {
                    h.cancel();
                }
                Ok(())
            }
            Command::PickFfmpeg => match self.dialogs.pick_ffmpeg() {
                Some(p) => self.dispatch_inner(Command::SetFfmpeg(p), now),
                None => Ok(()),
            },
            Command::SetFfmpeg(p) => {
                let probe = p.with_file_name(ffmpeg::exe_name("ffprobe"));
                self.settings.ffmpeg = Some(p);
                self.settings.ffprobe = probe.is_file().then_some(probe);
                if let Some(sp) = &self.settings_path {
                    self.settings.save(sp)?;
                }
                self.dispatch_inner(Command::RetryFfmpeg, now)
            }
            Command::RetryFfmpeg => {
                self.ffmpeg = check_ffmpeg(self.runner.as_ref(), &self.settings);
                self.preview = Preview::new(
                    preview_config(&self.ffmpeg, &self.runner),
                    crate::preview::default_sink(),
                );
                self.ffmpeg.as_ref().map(|_| ()).map_err(|e| e.clone())
            }
            Command::DismissNotice => {
                self.notice = None;
                Ok(())
            }
        }
    }

    fn export_extension(&self) -> String {
        render::timeline_media(&self.project)
            .ok()
            .and_then(|m| {
                let f: Vec<_> = m.iter().map(|m| &m.format).collect();
                crate::format::choose_output(&f)
            })
            .map(|f| crate::format::encode_settings(&f, &self.encoders()).extension)
            .unwrap_or_else(|| "mp4".into())
    }

    fn save_to(&mut self, path: PathBuf) -> Result<(), String> {
        project::save(&self.project, &path).map_err(|e| e.to_string())?;
        self.project_path = Some(path);
        self.dirty = false;
        self.status = "Project saved".into();
        Ok(())
    }

    fn load_project(&mut self, p: Project, path: Option<PathBuf>, now: Instant) {
        self.preview.invalidate(now);
        self.project = p;
        self.project_path = path;
        self.dirty = false;
        self.history.clear();
        self.selected_clip = None;
        self.selected_media = None;
        self.clipboard = None;
        self.frame = None;
        self.zoom = 0.0;
        self.seek(Micros::ZERO, now);
    }

    fn import(&mut self, files: Vec<PathBuf>) -> Result<(), String> {
        let info = self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        let ffprobe = info.ffprobe.clone();
        let mut errors = Vec::new();
        let mut added = 0;
        for f in files {
            let id = self.project.new_media_id();
            match media::probe(self.runner.as_ref(), &ffprobe, &f, id) {
                Ok(m) => {
                    self.selected_media = Some(m.id);
                    self.project.add_media(m);
                    added += 1;
                }
                Err(e) => errors.push(e.to_string()),
            }
        }
        if added > 0 {
            self.dirty = true;
        }
        self.status = format!("Imported {added} file(s)");
        if errors.is_empty() {
            Ok(())
        } else {
            let msg = format!("Some files could not be imported:\n{}", errors.join("\n"));
            self.notice = Some(msg.clone());
            Err(msg)
        }
    }

    fn start_export(&mut self, path: PathBuf) -> Result<(), String> {
        if self.is_exporting() {
            return Err("an export is already running".into());
        }
        let info = self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        export::validate_output(&self.project, &path).map_err(|e| e.to_string())?;
        let job = ExportJob {
            project: self.project.clone(),
            output: path,
            ffmpeg: info.ffmpeg.clone(),
            ffprobe: info.ffprobe.clone(),
            version: info.version,
            encoders: info.encoders.clone(),
        };
        self.export = Some(export::start(job, Arc::clone(&self.runner)));
        self.export_status = ExportStatus {
            running: true,
            progress: 0.0,
            stream_copy: None,
        };
        self.status = "Exporting…".into();
        Ok(())
    }

    /// Applies an edit to a copy of the timeline; on success records undo and refreshes.
    fn edit(
        &mut self,
        now: Instant,
        f: impl FnOnce(&mut Timeline) -> Result<(), EditError>,
    ) -> Result<(), String> {
        let before = self.project.timeline.clone();
        let mut tl = before.clone();
        f(&mut tl).map_err(|e| e.to_string())?;
        self.history.record(before);
        self.replace_timeline(tl, now);
        Ok(())
    }

    fn replace_timeline(&mut self, tl: Timeline, now: Instant) {
        self.project.timeline = tl;
        self.dirty = true;
        if let Some(id) = self.selected_clip
            && self.project.timeline.get(id).is_none()
        {
            self.selected_clip = None;
        }
        self.preview.invalidate(now);
        let t = self.playhead;
        self.seek(t, now);
    }

    fn seek(&mut self, t: Micros, now: Instant) {
        let dur = self.project.timeline.duration();
        let t = self.fps().snap(t.clamp(Micros::ZERO, dur)).min(dur);
        self.playhead = t;
        if self.project.timeline.clips.is_empty() {
            self.preview.invalidate(now);
            self.frame = None;
            return;
        }
        if self.ffmpeg.is_ok() {
            let _ = self.preview.seek(&self.project, t, now);
        }
    }

    /// Per-frame housekeeping: preview frames, playhead, export events.
    pub fn tick(&mut self, now: Instant) {
        if let Some(f) = self.preview.poll(now) {
            self.frame = Some(f);
            self.frame_serial += 1;
        }
        if self.preview.is_playing() {
            self.playhead = self.preview.position(now);
        } else if self.preview.position(now) != self.playhead
            && self.preview.position(now) == self.project.timeline.duration()
        {
            // playback reached the end
            self.playhead = self.preview.position(now);
        }
        let events: Vec<ExportEvent> = self
            .export
            .as_ref()
            .map(|h| h.rx.try_iter().collect())
            .unwrap_or_default();
        for e in events {
            match e {
                ExportEvent::Started { stream_copy } => {
                    self.export_status.stream_copy = Some(stream_copy)
                }
                ExportEvent::Progress(p) => self.export_status.progress = p,
                ExportEvent::Done(p) => {
                    self.finish_export(format!("Exported to {}", p.display()));
                }
                ExportEvent::Failed(m) => {
                    self.notice = Some(format!("Export failed: {m}"));
                    self.finish_export("Export failed".into());
                }
                ExportEvent::Cancelled => self.finish_export("Export cancelled".into()),
            }
        }
    }

    fn finish_export(&mut self, status: String) {
        self.export = None;
        self.export_status.running = false;
        self.status = status;
    }

    /// Window title.
    pub fn title(&self) -> String {
        let name = self
            .project_path
            .as_deref()
            .map(media::display_name)
            .unwrap_or_else(|| "untitled".into());
        format!("{}{name} — videolore", if self.dirty { "* " } else { "" })
    }

    /// Waits for a running export (tests and shutdown).
    pub fn wait_export(&mut self, now: Instant) {
        if let Some(h) = self.export.take() {
            let events = h.join();
            let (tx, rx) = std::sync::mpsc::channel();
            for e in events {
                let _ = tx.send(e);
            }
            self.export = Some(ExportHandle::from_events(rx));
            self.tick(now);
        }
    }
}

fn preview_config(
    ffmpeg: &Result<FfmpegInfo, String>,
    runner: &Arc<dyn Runner>,
) -> Option<PreviewConfig> {
    ffmpeg.as_ref().ok().map(|i| PreviewConfig {
        runner: Arc::clone(runner),
        ffmpeg: i.ffmpeg.clone(),
        version: i.version,
        encoders: i.encoders.clone(),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ffmpeg::FfVersion;
    use crate::ffmpeg::fake::{FakeRunner, ok};
    use crate::media::tests::MP4_JSON;
    use crate::preview::NullSink;

    #[derive(Default, Clone)]
    pub struct FakeDialogs {
        pub media: Vec<PathBuf>,
        pub open: Option<PathBuf>,
        pub save: Option<PathBuf>,
        pub export: Option<PathBuf>,
        pub ffmpeg: Option<PathBuf>,
    }

    impl Dialogs for FakeDialogs {
        fn pick_media(&mut self) -> Vec<PathBuf> {
            self.media.clone()
        }
        fn pick_project_open(&mut self) -> Option<PathBuf> {
            self.open.clone()
        }
        fn pick_project_save(&mut self) -> Option<PathBuf> {
            self.save.clone()
        }
        fn pick_export(&mut self, _extension: &str) -> Option<PathBuf> {
            self.export.clone()
        }
        fn pick_ffmpeg(&mut self) -> Option<PathBuf> {
            self.ffmpeg.clone()
        }
    }

    const AUDIO_JSON: &str = r#"{"streams":[{"codec_type":"audio","codec_name":"mp3"}],
        "format":{"format_name":"mp3","duration":"3.0"}}"#;

    /// ffprobe answers by file name; ffmpeg writes its output and reports progress.
    pub fn runner() -> FakeRunner {
        FakeRunner::default().on(|prog, a| {
            let last = a.last()?.clone();
            if prog.ends_with("ffprobe") {
                if a.iter().any(|x| x == "nokey") {
                    return ok("0.0\n");
                }
                if last.ends_with(".mp3") {
                    return ok(AUDIO_JSON);
                }
                if last.ends_with(".mp4") {
                    return ok(MP4_JSON);
                }
                return crate::ffmpeg::fake::fail("Invalid data found");
            }
            if let Some(out) = last.strip_prefix("file:") {
                std::fs::write(out, b"exported").ok()?;
                return ok("out_time_us=2500000\nprogress=end\n");
            }
            ok("")
        })
    }

    pub fn info() -> FfmpegInfo {
        FfmpegInfo {
            ffmpeg: "/usr/bin/ffmpeg".into(),
            ffprobe: "/usr/bin/ffprobe".into(),
            version: FfVersion::Release(6, 1, 1),
            filters: Default::default(),
            encoders: ["libx264", "aac"].iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn state(dialogs: FakeDialogs) -> AppState {
        AppState::with_ffmpeg(
            Ok(info()),
            Arc::new(runner()),
            Box::new(dialogs),
            Box::new(NullSink),
        )
    }

    /// Two 5 s video files and one audio file in `dir`.
    pub fn files(dir: &Path) -> Vec<PathBuf> {
        ["a.mp4", "b.mp4", "song.mp3"]
            .iter()
            .map(|n| {
                let p = dir.join(n);
                std::fs::write(&p, n.as_bytes()).unwrap();
                p
            })
            .collect()
    }

    fn s(n: i64) -> Micros {
        Micros(n * 1_000_000)
    }

    // covers: REQ-025, REQ-005, REQ-006, REQ-007, REQ-008, REQ-009, REQ-010, REQ-011, REQ-004
    #[test]
    fn every_edit_goes_through_the_dispatcher() {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let mut app = state(FakeDialogs {
            media: f.clone(),
            ..Default::default()
        });
        let now = Instant::now();
        let run = |app: &mut AppState, c: Command| app.dispatch(c, now);
        run(&mut app, Command::PickImport).unwrap();
        assert_eq!(app.project.media.len(), 3);
        let (a, b, song) = (
            app.project.media[0].id,
            app.project.media[1].id,
            app.project.media[2].id,
        );
        // REQ-005 append
        run(&mut app, Command::Append(Some(a))).unwrap();
        run(&mut app, Command::SelectMedia(Some(b))).unwrap();
        run(&mut app, Command::Append(None)).unwrap();
        assert!(!app.is_enabled(&Command::Append(Some(song))));
        assert!(run(&mut app, Command::Append(Some(song))).is_err());
        assert_eq!(app.project.timeline.duration(), s(10));
        // REQ-006 split at the playhead
        run(&mut app, Command::Seek(s(2))).unwrap();
        assert!(app.is_enabled(&Command::Split));
        run(&mut app, Command::Split).unwrap();
        assert_eq!(app.project.timeline.clips.len(), 3);
        assert_eq!(app.project.timeline.duration(), s(10));
        // REQ-008 merge back (enabled only for contiguous parts)
        let first = app.project.timeline.clips[0].id;
        run(&mut app, Command::SelectClip(Some(first))).unwrap();
        assert!(app.is_enabled(&Command::Merge));
        run(&mut app, Command::Merge).unwrap();
        assert_eq!(app.project.timeline.clips.len(), 2);
        assert!(!app.is_enabled(&Command::Merge));
        // REQ-007 copy / paste at playhead (ripple) / cut
        run(&mut app, Command::Copy).unwrap();
        run(&mut app, Command::Seek(s(5))).unwrap();
        run(&mut app, Command::Paste).unwrap();
        assert_eq!(app.project.timeline.duration(), s(15));
        let pasted = app.selected_clip.unwrap();
        assert_eq!(app.project.timeline.index_of(pasted), Some(1));
        run(&mut app, Command::Cut).unwrap();
        assert_eq!(app.project.timeline.duration(), s(10));
        assert!(app.clipboard.is_some());
        // REQ-009 trim
        let c0 = app.project.timeline.clips[0].id;
        run(
            &mut app,
            Command::SetTrim {
                clip: c0,
                src_in: s(1),
                src_out: s(99),
            },
        )
        .unwrap();
        assert_eq!(app.project.timeline.clips[0].src_in, s(1));
        assert_eq!(app.project.timeline.clips[0].src_out, s(5));
        // REQ-010 move
        run(&mut app, Command::SelectClip(Some(c0))).unwrap();
        assert!(!app.is_enabled(&Command::MoveLeft));
        run(&mut app, Command::MoveRight).unwrap();
        assert_eq!(app.project.timeline.index_of(c0), Some(1));
        run(&mut app, Command::MoveLeft).unwrap();
        run(&mut app, Command::MoveClip { clip: c0, to: 1 }).unwrap();
        assert_eq!(app.project.timeline.index_of(c0), Some(1));
        // REQ-011 fades
        run(&mut app, Command::Seek(s(4))).unwrap();
        run(&mut app, Command::ToggleFadeAtPlayhead).unwrap();
        assert!(app.project.timeline.has_fade(0));
        run(&mut app, Command::ToggleFade(0)).unwrap();
        assert!(!app.project.timeline.has_fade(0));
        // REQ-004 undo / redo
        let before = app.project.timeline.clone();
        run(&mut app, Command::Undo).unwrap();
        assert!(app.project.timeline.has_fade(0));
        run(&mut app, Command::Redo).unwrap();
        assert_eq!(app.project.timeline, before);
        assert!(run(&mut app, Command::Redo).is_err());
        // delete
        run(&mut app, Command::SelectClip(Some(c0))).unwrap();
        run(&mut app, Command::Delete).unwrap();
        assert_eq!(app.project.timeline.clips.len(), 1);
        assert!(app.selected_clip.is_none());
        // misc
        run(&mut app, Command::SetZoom(-3.0)).unwrap();
        assert_eq!(app.zoom, 0.0);
        run(&mut app, Command::StepFrames(3)).unwrap();
        assert_eq!(app.playhead, Micros(4_100_000));
        assert!(app.dirty);
        assert!(app.title().starts_with("* untitled"));
    }

    // covers: REQ-004
    #[test]
    fn fifty_operations_can_be_undone() {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let mut app = state(FakeDialogs::default());
        let now = Instant::now();
        app.dispatch(Command::Import(vec![f[0].clone()]), now)
            .unwrap();
        for _ in 0..50 {
            app.dispatch(Command::Append(None), now).unwrap();
        }
        assert_eq!(app.project.timeline.clips.len(), 50);
        for _ in 0..50 {
            app.dispatch(Command::Undo, now).unwrap();
        }
        assert!(app.project.timeline.clips.is_empty());
        assert!(!app.is_enabled(&Command::Undo));
        for _ in 0..50 {
            app.dispatch(Command::Redo, now).unwrap();
        }
        assert_eq!(app.project.timeline.clips.len(), 50);
    }

    // covers: REQ-002
    #[test]
    fn unreadable_import_is_reported_without_crashing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("notes.txt");
        std::fs::write(&bad, "hello").unwrap();
        let mut f = files(dir.path());
        f.push(bad);
        f.push(dir.path().join("gone.mp4"));
        let mut app = state(FakeDialogs::default());
        let e = app
            .dispatch(Command::Import(f), Instant::now())
            .unwrap_err();
        assert!(e.contains("notes.txt"));
        assert!(e.contains("gone.mp4"));
        assert_eq!(app.project.media.len(), 3);
        assert!(app.notice.is_some());
        app.dispatch(Command::DismissNotice, Instant::now())
            .unwrap();
        assert!(app.notice.is_none());
        // audio-only is in the list but cannot go on the video track
        let song = app.project.media[2].clone();
        assert!(!song.has_video());
        assert!(!app.is_enabled(&Command::Append(Some(song.id))));
    }

    // covers: REQ-003
    #[test]
    fn save_and_open_through_commands() {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let proj = dir.path().join("my");
        let mut app = state(FakeDialogs {
            save: Some(proj.clone()),
            open: Some(proj.with_extension("vlproj")),
            ..Default::default()
        });
        let now = Instant::now();
        app.dispatch(Command::Import(f.clone()), now).unwrap();
        app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
            .unwrap();
        app.dispatch(Command::SaveProject, now).unwrap(); // no path yet: asks
        let saved = proj.with_extension("vlproj");
        assert_eq!(app.project_path.as_deref(), Some(saved.as_path()));
        assert!(!app.dirty);
        assert_eq!(app.title(), "my.vlproj — videolore");
        app.dispatch(Command::SaveProject, now).unwrap();
        let snapshot = app.project.clone();
        app.dispatch(Command::NewProject, now).unwrap();
        assert!(app.project.media.is_empty());
        std::fs::remove_file(&f[1]).unwrap();
        app.dispatch(Command::PickOpenProject, now).unwrap();
        assert_eq!(app.project.timeline, snapshot.timeline);
        assert!(app.notice.as_deref().unwrap().contains("b.mp4"));
        assert!(app.project.media[1].missing);
        assert!(
            app.dispatch(Command::OpenProject(dir.path().join("none.vlproj")), now)
                .is_err()
        );
        assert!(!app.status.is_empty());
    }

    // covers: REQ-021, REQ-001
    #[test]
    fn export_through_commands() {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let out = dir.path().join("out.mp4");
        let mut app = state(FakeDialogs {
            export: Some(out.clone()),
            ..Default::default()
        });
        let now = Instant::now();
        assert!(!app.is_enabled(&Command::PickExport));
        app.dispatch(Command::Import(f.clone()), now).unwrap();
        app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
            .unwrap();
        assert!(
            app.output_description()
                .unwrap()
                .contains("same as sources")
        );
        // refuse to overwrite a source
        assert!(app.dispatch(Command::Export(f[0].clone()), now).is_err());
        app.dispatch(Command::PickExport, now).unwrap();
        assert!(app.is_exporting());
        assert!(app.is_enabled(&Command::CancelExport));
        assert!(app.dispatch(Command::Export(out.clone()), now).is_err());
        app.wait_export(now);
        assert!(!app.is_exporting());
        assert_eq!(app.export_status.stream_copy, Some(true));
        assert_eq!(app.export_status.progress, 1.0);
        assert!(app.status.starts_with("Exported to"));
        assert_eq!(std::fs::read(&out).unwrap(), b"exported");
        // sources untouched
        assert_eq!(std::fs::read(&f[0]).unwrap(), b"a.mp4");
        app.dispatch(Command::CancelExport, now).unwrap();
    }

    #[test]
    fn export_failure_shows_notice() {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let failing = FakeRunner::default().on(|prog, a| {
            if prog.ends_with("ffprobe") {
                return if a.iter().any(|x| x == "nokey") {
                    ok("0.0\n")
                } else {
                    ok(MP4_JSON)
                };
            }
            crate::ffmpeg::fake::fail("disk full")
        });
        let mut app = AppState::with_ffmpeg(
            Ok(info()),
            Arc::new(failing),
            Box::new(FakeDialogs::default()),
            Box::new(NullSink),
        );
        let now = Instant::now();
        app.dispatch(Command::Import(vec![f[0].clone()]), now)
            .unwrap();
        app.dispatch(Command::Append(None), now).unwrap();
        app.dispatch(Command::Export(dir.path().join("o.mp4")), now)
            .unwrap();
        app.wait_export(now);
        assert!(app.notice.as_deref().unwrap().contains("disk full"));
        assert_eq!(app.status, "Export failed");
    }

    #[test]
    fn without_ffmpeg_everything_degrades_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("cfg/settings.json");
        let mut app = AppState::with_ffmpeg(
            Err("ffmpeg was not found".into()),
            Arc::new(FakeRunner::default()),
            Box::new(FakeDialogs {
                ffmpeg: Some(dir.path().join("my-ffmpeg")),
                ..Default::default()
            }),
            Box::new(NullSink),
        );
        app.settings_path = Some(settings.clone());
        let now = Instant::now();
        assert!(!app.is_enabled(&Command::PickImport));
        assert!(app.dispatch(Command::Import(vec![]), now).is_err());
        assert!(
            app.dispatch(Command::Export(dir.path().join("x.mp4")), now)
                .is_err()
        );
        assert!(app.dispatch(Command::PlayPause, now).is_err());
        // choosing a binary saves the setting and re-checks
        assert!(app.dispatch(Command::PickFfmpeg, now).is_err());
        assert_eq!(
            Settings::load(&settings).ffmpeg,
            Some(dir.path().join("my-ffmpeg"))
        );
        assert_eq!(
            Settings::load(&dir.path().join("none")),
            Settings::default()
        );
        assert_eq!(app.fps(), Fps { num: 30, den: 1 });
        assert!(app.output_description().is_none());
        assert_eq!(app.export_extension(), "mp4");
    }

    #[test]
    fn dialogs_cancelled_do_nothing() {
        let mut app = state(FakeDialogs::default());
        let now = Instant::now();
        for c in [
            Command::PickImport,
            Command::PickOpenProject,
            Command::PickSaveProjectAs,
            Command::PickExport,
            Command::PickFfmpeg,
            Command::CancelExport,
        ] {
            app.dispatch(c, now).unwrap();
        }
        for c in [
            Command::Copy,
            Command::Cut,
            Command::Paste,
            Command::Merge,
            Command::MoveLeft,
            Command::Undo,
            Command::ToggleFadeAtPlayhead,
            Command::Append(None),
            Command::SetTrim {
                clip: ClipId(1),
                src_in: Micros::ZERO,
                src_out: Micros(1),
            },
        ] {
            assert!(app.dispatch(c.clone(), now).is_err(), "{c:?}");
        }
    }

    #[test]
    fn play_pause_and_tick() {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        // Preview streams stay open until killed, so the end of stream cannot race the clock.
        let mut app = AppState::with_ffmpeg(
            Ok(info()),
            Arc::new(runner().holding_streams()),
            Box::new(FakeDialogs::default()),
            Box::new(NullSink),
        );
        let t0 = Instant::now();
        app.dispatch(Command::Import(vec![f[0].clone()]), t0)
            .unwrap();
        app.dispatch(Command::Append(None), t0).unwrap();
        app.dispatch(Command::PlayPause, t0).unwrap();
        assert!(app.preview.is_playing());
        app.tick(t0 + std::time::Duration::from_millis(500));
        assert_eq!(app.playhead, Micros(500_000));
        app.dispatch(
            Command::PlayPause,
            t0 + std::time::Duration::from_millis(600),
        )
        .unwrap();
        assert!(!app.preview.is_playing());
        assert_eq!(app.playhead, Micros(600_000));
        // paste of a clip whose source was removed is refused
        app.clipboard = Some(Clip {
            id: ClipId(9),
            media: MediaId(99),
            src_in: Micros::ZERO,
            src_out: Micros(1),
        });
        assert!(app.dispatch(Command::Paste, t0).is_err());
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a/b/settings.json");
        let s = Settings {
            ffmpeg: Some("/x/ffmpeg".into()),
            ffprobe: None,
        };
        s.save(&p).unwrap();
        assert_eq!(Settings::load(&p), s);
        assert!(Settings::default_path().is_some() || cfg!(not(unix)));
        let app = AppState::new(
            Arc::new(FakeRunner::default()),
            Box::new(FakeDialogs::default()),
            Box::new(NullSink),
            Some(p),
        );
        assert_eq!(app.settings, s);
    }
}
