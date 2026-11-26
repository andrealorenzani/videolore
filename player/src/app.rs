// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The state of videolore player (REQ-138..141): one video at a time, played by the
//! editor's own preview engine from an in-memory one-clip project that is never saved.
//!
//! The command enum has no save, export or edit command: the player cannot change a file
//! because it has no way to ask. It reads the video, and writes only the preview-copy
//! cache and private work folders.

use crate::fullscreen::{EscapeResult, FullScreen};
use crate::playlist::Playlist;
use crate::settings::PlayerSettings;
use crate::subs::{self, Source, Subs};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use videolore::base::pos::Pos;
use videolore::camera::{Camera, Pov};
use videolore::ffmpeg::{FfmpegInfo, Runner};
use videolore::jobs::{MAX_PROCESSES, Priority, Scheduler};
use videolore::machine::Machine;
use videolore::media::{self, MediaId, MediaItem, ProxyState, display_name};
use videolore::preview::{AudioSink, Frame, Preview, PreviewConfig};
use videolore::project::Project;
use videolore::proxy::{self, ProxyCache, ProxyEvent, ProxyTask, ProxyWorker};
use videolore::settings::PreviewQuality;
use videolore::srt::Cue;
use videolore::subtitles::LineId;
use videolore::time::{Fps, Micros};

/// Why a `.vlproj` is not opened (REQ-138).
pub const PROJECT_REFUSED: &str =
    "videolore player plays video files. Projects are opened in videolore (the editor).";
/// Shown when ffmpeg cannot be started.
pub const NO_FFMPEG: &str =
    "videolore player needs ffmpeg and could not start it. Reinstall videolore player.";
/// Quiet time after the last drag before playback restarts with the new camera.
pub const CAMERA_RESTART: Duration = Duration::from_millis(100);
/// Seconds the arrow keys move.
pub const SKIP: Micros = Micros(5_000_000);

/// What the user can ask. There is no save, export or edit here, on purpose.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// File > Open… (asks for one or several files).
    PickOpen,
    /// Opens this file (argument or drop).
    Open(PathBuf),
    /// The answer of Open…: these videos become the playlist and the first one plays
    /// (REQ-355).
    OpenVideos(Vec<PathBuf>),
    PlayPause,
    Seek(Micros),
    /// Moves by whole frames (negative = back).
    Step(i32),
    /// Moves by this much (negative = back).
    Skip(Micros),
    /// Volume, 0..=100.
    SetVolume(u8),
    ToggleMute,
    /// Audio menu: the ordinal of the audio track to play (REQ-338).
    ChooseAudio(u8),
    /// Subtitles menu: `None` = no subtitles, else the source's number.
    ChooseSubtitles(Option<usize>),
    /// Subtitles > Open .srt… (asks for a file).
    PickSrt,
    /// Adds this `.srt` and chooses it.
    OpenSrt(PathBuf),
    /// Lines being dragged (shown at once); `None` = the drag ended.
    DragLines(Option<(Vec<LineId>, Pos)>),
    /// Puts lines at a place (the end of a drag).
    PlaceLines {
        ids: Vec<LineId>,
        pos: Pos,
    },
    /// Look around in a 360 video: pixels dragged on a picture `view_w` wide.
    DragCamera {
        dx: f32,
        dy: f32,
        view_w: f32,
    },
    /// Wheel zoom of a 360 video.
    ZoomCamera(f32),
    /// View > Full screen, F11, F.
    ToggleFullScreen,
    /// Esc.
    Escape,
    /// Close on the start screen (or its window's close control).
    CloseSplash,
    /// OK on "Press Esc to exit full screen".
    DismissFullScreenHint,
    ShowAbout,
    CloseAbout,
    DismissNotice,
    /// File > Add to playlist… (asks for one or several files).
    AddToPlaylist,
    /// Adds these videos at the end of the playlist (REQ-356).
    AddToPlaylistPaths(Vec<PathBuf>),
    /// File > Open folder… (asks for a folder), or this folder: its videos in name order.
    OpenFolder,
    OpenFolderPath(PathBuf),
    /// The next and the previous video of the playlist.
    Next,
    Previous,
    /// File > Open in editor: the video playing, in videolore.
    OpenInEditor,
    Quit,
}

/// Native file dialogs (faked in tests).
pub trait Dialogs {
    /// The videos chosen, in the dialog's order; empty when cancelled.
    fn pick_videos(&mut self) -> Vec<PathBuf>;
    fn pick_srt(&mut self) -> Option<PathBuf>;
    /// Asks for a folder (the dialogs of the player start in the last one used).
    fn pick_folder(&mut self) -> Option<PathBuf> {
        None
    }
    /// The folder the next dialog starts in.
    fn start_in(&mut self, _folder: Option<PathBuf>) {}
}

type ProbeResult = Result<MediaItem, String>;
type TrackResult = (usize, Result<Vec<Cue>, String>);

/// The folders the player writes to (and nowhere else).
#[derive(Debug, Clone)]
pub struct Folders {
    /// The preview-copy cache shared with the editor; `None` = no copies are made.
    pub cache_dir: Option<PathBuf>,
    /// Where private work folders are made.
    pub work_root: PathBuf,
}

/// The player.
pub struct PlayerState {
    pub ffmpeg: Result<FfmpegInfo, String>,
    sched: Arc<Scheduler>,
    runner: Arc<dyn Runner>,
    dialogs: Box<dyn Dialogs>,
    machine: Machine,
    cache_dir: Option<PathBuf>,
    /// Where private work folders are made (a temporary folder in tests).
    work_root: PathBuf,
    pub project: Project,
    pub file: Option<PathBuf>,
    pub preview: Preview,
    pub playhead: Micros,
    pub frame: Option<Frame>,
    pub frame_serial: u64,
    pub status: String,
    pub notice: Option<String>,
    pub show_about: bool,
    /// The start screen is open (REQ-150): at a start without a video.
    pub splash: bool,
    pub subs: Subs,
    pub fullscreen: FullScreen,
    /// The 360 camera you dragged (not saved, there is nothing to save it in).
    live: Option<Camera>,
    camera_restart: Option<Instant>,
    /// Lines being dragged, shown at their new place.
    drag_pos: Option<(Vec<LineId>, Pos)>,
    probe: Option<(Receiver<ProbeResult>, Arc<AtomicBool>, PathBuf)>,
    pending_id: MediaId,
    tracks: Option<Receiver<TrackResult>>,
    proxy_events: Option<Receiver<ProxyEvent>>,
    proxy_cancel: Option<Arc<AtomicBool>>,
    pub quit: bool,
    /// What the player remembers between runs (REQ-252) and where it is kept.
    pub settings: PlayerSettings,
    settings_path: Option<PathBuf>,
    /// The videos to play one after the other, for this session only (REQ-254).
    pub playlist: Playlist,
    /// Where lines were dragged, by video, for this session only (REQ-251).
    placed_memory: HashMap<PathBuf, HashMap<(usize, usize), Pos>>,
    /// The audio track chosen for each video, for this session only (REQ-338, not saved).
    audio_memory: HashMap<PathBuf, u8>,
    /// The video was playing at the last tick (to see it end).
    was_playing: bool,
    /// Start playing as soon as the next video of the playlist is open.
    pending_play: bool,
}

/// The key of a video in the session's memory: its place on disk when it can be told.
fn memory_key(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// True for a project file, by its extension only (the file is never read).
pub fn is_project_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(videolore::project::EXTENSION))
}

impl PlayerState {
    pub fn new(
        ffmpeg: Result<FfmpegInfo, String>,
        runner: Arc<dyn Runner>,
        dialogs: Box<dyn Dialogs>,
        sink: Box<dyn AudioSink>,
        machine: Machine,
        folders: Folders,
        now: Instant,
    ) -> PlayerState {
        let Folders {
            cache_dir,
            work_root,
        } = folders;
        let sched = Scheduler::new(MAX_PROCESSES);
        let config = ffmpeg.as_ref().ok().map(|i| PreviewConfig {
            runner: sched.runner(Priority::Preview, Arc::clone(&runner)),
            ffmpeg: i.ffmpeg.clone(),
            version: i.version,
            encoders: i.encoders.clone(),
        });
        let notice = ffmpeg.as_ref().err().map(|_| NO_FFMPEG.to_string());
        PlayerState {
            ffmpeg,
            sched,
            runner,
            dialogs,
            machine,
            cache_dir,
            work_root,
            project: Project::default(),
            file: None,
            preview: Preview::new(config, sink),
            playhead: Micros::ZERO,
            frame: None,
            frame_serial: 0,
            status: String::new(),
            notice,
            show_about: false,
            splash: false,
            subs: Subs::default(),
            fullscreen: FullScreen::new(now),
            live: None,
            camera_restart: None,
            drag_pos: None,
            probe: None,
            pending_id: MediaId(0),
            tracks: None,
            proxy_events: None,
            proxy_cancel: None,
            quit: false,
            settings: PlayerSettings::default(),
            settings_path: None,
            playlist: Playlist::default(),
            placed_memory: HashMap::new(),
            audio_memory: HashMap::new(),
            was_playing: false,
            pending_play: false,
        }
    }

    /// Uses these settings (volume, sound, full screen, last folder) and keeps them in
    /// `path` (REQ-252).
    pub fn with_settings(
        mut self,
        settings: PlayerSettings,
        path: Option<PathBuf>,
        now: Instant,
    ) -> Self {
        if settings.fullscreen && !self.fullscreen.on {
            self.fullscreen.toggle(now);
        }
        self.settings = settings;
        self.settings_path = path;
        self
    }

    /// Writes the settings when the volume, the sound or full screen changed.
    fn sync_settings(&mut self) {
        let mut next = self.settings.clone();
        next.volume = self.project.mix.video_volume;
        next.muted = self.project.mix.video_muted;
        next.fullscreen = self.fullscreen.on;
        // a project without a video keeps what was remembered
        if !self.has_video() {
            next.volume = self.settings.volume;
            next.muted = self.settings.muted;
        }
        if next != self.settings {
            self.settings = next;
            self.save_settings();
        }
    }

    fn save_settings(&self) {
        if let Some(p) = &self.settings_path {
            let _ = self.settings.save(p);
        }
    }

    /// The start picture at a start without a video (REQ-295): by the editor's choice, which
    /// the player follows; the version that showed it is remembered here.
    pub fn start_splash(&mut self, option: videolore::settings::SplashOption) {
        let version = env!("CARGO_PKG_VERSION");
        let last = self.settings.last_splash_version.clone();
        if videolore::splash::should_show(option, last.as_deref(), version, false) {
            self.show_splash();
            if last.as_deref() != Some(version) {
                self.settings.last_splash_version = Some(version.to_string());
                self.save_settings();
            }
        }
    }

    /// Opens the start screen (REQ-150): at a start without a video on the command line.
    pub fn show_splash(&mut self) {
        self.splash = true;
    }

    /// The window title: the file's name, its place in a playlist of several videos
    /// (REQ-356) and the program's name.
    pub fn title(&self) -> String {
        match &self.file {
            Some(p) => {
                let place = self
                    .playlist
                    .place()
                    .map(|(n, m)| format!(" ({n} of {m})"))
                    .unwrap_or_default();
                format!("{}{place} — videolore player", display_name(p))
            }
            None => "videolore player".to_string(),
        }
    }

    pub fn duration(&self) -> Micros {
        self.project.timeline.duration()
    }

    /// A video is open and ready.
    pub fn has_video(&self) -> bool {
        !self.project.timeline.clips.is_empty()
    }

    /// The 360 clip is shown (the file says so).
    pub fn is_360(&self) -> bool {
        self.project
            .media
            .first()
            .is_some_and(|m| m.format.is_360())
    }

    pub fn is_playing(&self) -> bool {
        self.preview.is_playing()
    }

    /// A video is being opened.
    pub fn opening(&self) -> bool {
        self.probe.is_some()
    }

    pub fn volume(&self) -> u8 {
        self.project.mix.video_volume
    }

    pub fn muted(&self) -> bool {
        self.project.mix.video_muted
    }

    /// The size of the picture (output frame, pixels).
    pub fn output_size(&self) -> Option<(u32, u32)> {
        let enc = self
            .ffmpeg
            .as_ref()
            .map(|i| i.encoders.clone())
            .unwrap_or_default();
        videolore::render::output_geometry(&self.project, &enc)
            .ok()
            .map(|g| (g.width, g.height))
    }

    fn frame_len(&self) -> Micros {
        let fps = self
            .project
            .media
            .first()
            .and_then(|m| m.format.fps)
            .or(Fps::new(30, 1))
            .map(Fps::frame_duration);
        fps.unwrap_or(Micros(33_333))
    }

    /// The tick of every frame: preview pictures, the playhead, background results.
    pub fn tick(&mut self, now: Instant) {
        self.fullscreen.tick(now);
        self.tick_camera(now);
        self.tick_probe();
        self.tick_tracks(now);
        self.tick_proxy(now);
        if let Some(f) = self.preview.poll(now) {
            self.frame = Some(f);
            self.frame_serial += 1;
        }
        if self.preview.is_playing() {
            self.playhead = self.preview.position(now);
        } else if self.preview.position(now) != self.playhead
            && self.preview.position(now) == self.duration()
        {
            self.playhead = self.duration();
        }
        // the video played to its end: the next one of the playlist starts, or it stops
        // after the last (REQ-254)
        let ended = self.was_playing
            && !self.preview.is_playing()
            && self.has_video()
            && self.playhead >= self.duration();
        self.was_playing = self.preview.is_playing();
        if ended {
            self.after_end(now);
        }
    }

    /// The video played to its end: the next one of the playlist starts playing; after the
    /// last one it stops (REQ-254). True when another video was started.
    pub fn after_end(&mut self, now: Instant) -> bool {
        let Some(next) = self.playlist.next().cloned() else {
            return false;
        };
        let _ = self.open(next, now);
        self.pending_play = true;
        true
    }

    /// Something is going on that needs frequent repaints.
    pub fn busy(&self) -> bool {
        self.is_playing() || self.probe.is_some() || self.tracks.is_some() || self.preview.busy()
    }

    /// Runs a command; the error text goes to the status line.
    pub fn dispatch(&mut self, cmd: Command, now: Instant) {
        if let Err(e) = self.run(cmd, now) {
            self.status = e;
        }
        self.sync_settings();
    }

    /// Open…: the videos chosen replace the playlist and the first one plays (REQ-355).
    fn open_videos(&mut self, files: Vec<PathBuf>, now: Instant) -> Result<(), String> {
        let (projects, videos): (Vec<_>, Vec<_>) =
            files.into_iter().partition(|p| is_project_file(p));
        if videos.is_empty() {
            // only projects: the refusal of REQ-138; nothing chosen: nothing happens
            return projects
                .into_iter()
                .next()
                .map_or(Ok(()), |p| self.open(p, now));
        }
        self.playlist.replace(videos);
        let first = self.playlist.items[0].clone();
        self.open(first, now)?;
        // a missing ffmpeg opens nothing: nothing is waiting to play
        self.pending_play = self.probe.is_some();
        Ok(())
    }

    /// Adds videos at the end of the playlist; the one playing goes on (REQ-356).
    fn add_to_playlist(&mut self, files: Vec<PathBuf>, now: Instant) -> Result<(), String> {
        // a video opened by itself is the first of the list it starts
        if self.playlist.items.is_empty()
            && let Some(open) = self
                .probe
                .as_ref()
                .map(|(_, _, p)| p.clone())
                .or_else(|| self.file.clone())
        {
            self.playlist.add(open.clone());
            self.playlist.play(&open);
        }
        let mut added = Vec::new();
        for p in files {
            if crate::playlist::is_video(&p) && !is_project_file(&p) && self.playlist.add(p.clone())
            {
                added.push(p);
            }
        }
        match added.as_slice() {
            [] => return Ok(()),
            [one] => self.status = format!("Added {} to the playlist", display_name(one)),
            many => self.status = format!("Added {} videos to the playlist", many.len()),
        }
        // with nothing open, the first video added starts
        if !self.has_video() && self.probe.is_none() {
            let first = added.swap_remove(0);
            return self.open(first, now);
        }
        Ok(())
    }

    fn open_folder(&mut self, dir: &Path, now: Instant) -> Result<(), String> {
        if self.playlist.set_folder(dir) == 0 {
            return Err("There are no videos in that folder".into());
        }
        self.remember_folder(dir);
        let first = self.playlist.items[0].clone();
        self.open(first, now)
    }

    fn remember_folder(&mut self, dir: &Path) {
        if crate::settings::remember_folder(dir)
            && self.settings.last_folder.as_deref() != Some(dir)
        {
            self.settings.last_folder = Some(dir.to_path_buf());
            self.save_settings();
        }
    }

    fn run(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        match cmd {
            Command::PickOpen => {
                self.dialogs.start_in(self.settings.last_folder.clone());
                let files = self.dialogs.pick_videos();
                self.open_videos(files, now)
            }
            Command::Open(p) => self.open(p, now),
            Command::OpenVideos(files) => self.open_videos(files, now),
            Command::PlayPause => self.play_pause(now),
            Command::Seek(t) => self.seek(t, now),
            Command::Step(n) => {
                let to = self.playhead + Micros(self.frame_len().0 * i64::from(n));
                self.seek(to, now)
            }
            Command::Skip(d) => self.seek(self.playhead + d, now),
            Command::SetVolume(v) => {
                self.project.mix.video_volume = v.min(100);
                self.restart_playback(now)
            }
            Command::ToggleMute => {
                self.project.mix.video_muted = !self.project.mix.video_muted;
                self.restart_playback(now)
            }
            Command::ChooseAudio(track) => self.choose_audio(track, now),
            Command::ChooseSubtitles(s) => self.choose_subtitles(s, now),
            Command::PickSrt => match self.dialogs.pick_srt() {
                Some(p) => self.open_srt(&p, now),
                None => Ok(()),
            },
            Command::OpenSrt(p) => self.open_srt(&p, now),
            Command::DragLines(d) => {
                if d.is_some() && self.is_playing() {
                    self.pause(now);
                }
                self.drag_pos = d;
                self.refresh_still(now);
                Ok(())
            }
            Command::PlaceLines { ids, pos } => self.place_lines(&ids, pos, now),
            Command::DragCamera { dx, dy, view_w } => {
                let cam = self.live.unwrap_or(Camera::FRONT).dragged(dx, dy, view_w);
                self.set_camera(cam, now)
            }
            Command::ZoomCamera(scroll) => {
                let cam = self.live.unwrap_or(Camera::FRONT).zoomed(scroll);
                self.set_camera(cam, now)
            }
            Command::ToggleFullScreen => {
                self.fullscreen.toggle(now);
                Ok(())
            }
            Command::CloseSplash => {
                self.splash = false;
                Ok(())
            }
            Command::Escape => {
                if self.splash {
                    self.splash = false;
                } else if self.show_about {
                    self.show_about = false;
                } else if self.notice.is_some() {
                    self.notice = None;
                } else if self.fullscreen.escape() == EscapeResult::Nothing {
                    // not in full screen: Esc does nothing else
                }
                Ok(())
            }
            Command::DismissFullScreenHint => {
                self.fullscreen.dismiss_hint();
                Ok(())
            }
            Command::ShowAbout => {
                self.show_about = true;
                Ok(())
            }
            Command::CloseAbout => {
                self.show_about = false;
                Ok(())
            }
            Command::DismissNotice => {
                self.notice = None;
                Ok(())
            }
            Command::AddToPlaylist => {
                self.dialogs.start_in(self.settings.last_folder.clone());
                let files = self.dialogs.pick_videos();
                self.add_to_playlist(files, now)
            }
            Command::AddToPlaylistPaths(files) => self.add_to_playlist(files, now),
            Command::OpenFolder => {
                self.dialogs.start_in(self.settings.last_folder.clone());
                match self.dialogs.pick_folder() {
                    Some(d) => self.open_folder(&d, now),
                    None => Ok(()),
                }
            }
            Command::OpenFolderPath(d) => self.open_folder(&d, now),
            Command::Next => match self.playlist.next().cloned() {
                Some(p) => self.open(p, now),
                None => Err("This is the last video of the playlist".into()),
            },
            Command::Previous => match self.playlist.previous().cloned() {
                Some(p) => self.open(p, now),
                None => Err("This is the first video of the playlist".into()),
            },
            Command::OpenInEditor => match self.file.clone() {
                Some(p) => videolore::siblings::launch(videolore::siblings::EDITOR, &p),
                None => Err("Open a video first".into()),
            },
            Command::Quit => {
                self.quit = true;
                Ok(())
            }
        }
    }

    // ---- opening ----

    /// Starts opening a video: a project is refused by its name alone (never read); the
    /// probe runs in the background.
    fn open(&mut self, path: PathBuf, now: Instant) -> Result<(), String> {
        // a video given or chosen makes the start screen pointless
        self.splash = false;
        if is_project_file(&path) {
            self.notice = Some(PROJECT_REFUSED.into());
            return Ok(());
        }
        let ffprobe = match &self.ffmpeg {
            Ok(i) => i.ffprobe.clone(),
            Err(_) => {
                self.notice = Some(NO_FFMPEG.into());
                return Ok(());
            }
        };
        self.close_current(now);
        let mut p = Project::default();
        self.pending_id = p.new_media_id();
        let (id, name) = (self.pending_id, display_name(&path));
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let runner = self
            .sched
            .runner(Priority::Probe, Arc::clone(&self.runner))
            .cancellable(Arc::clone(&cancel));
        let target = path.clone();
        std::thread::spawn(move || {
            let r = media::probe(&runner, &ffprobe, &target, id).map_err(|e| e.to_string());
            let _ = tx.send(r);
        });
        self.probe = Some((rx, cancel, path));
        self.status = format!("Opening {name}…");
        Ok(())
    }

    /// Stops what belongs to the video that is open now.
    fn close_current(&mut self, now: Instant) {
        self.preview.pause(now);
        if let Some((_, c, _)) = self.probe.take() {
            c.store(true, Ordering::SeqCst);
        }
        if let Some(c) = self.proxy_cancel.take() {
            c.store(true, Ordering::SeqCst);
        }
        self.proxy_events = None;
        self.tracks = None;
        // the places of dragged lines are kept for this session (REQ-251)
        if let Some(f) = &self.file {
            let placed = self.subs.take_placed();
            self.placed_memory.insert(memory_key(f), placed);
        }
        // the volume and the sound stay what they were (REQ-252)
        if self.has_video() {
            self.settings.volume = self.project.mix.video_volume.min(100);
            self.settings.muted = self.project.mix.video_muted;
        }
        self.project = Project::default();
        self.file = None;
        self.subs = Subs::default();
        self.live = None;
        self.drag_pos = None;
        self.camera_restart = None;
        self.playhead = Micros::ZERO;
        self.frame = None;
        self.frame_serial += 1;
        self.preview.forget_frame();
    }

    fn tick_probe(&mut self) {
        let Some((rx, _, path)) = &self.probe else {
            return;
        };
        let Ok(result) = rx.try_recv() else {
            return;
        };
        let path = path.clone();
        self.probe = None;
        match result {
            Err(e) => {
                self.notice = Some(e);
                self.status.clear();
            }
            Ok(m) if !m.has_video() => {
                self.notice = Some(format!(
                    "{} has no picture. videolore player plays video files.",
                    display_name(&path)
                ));
                self.status.clear();
            }
            Ok(m) => {
                self.loaded(m, path);
                if std::mem::take(&mut self.pending_play) {
                    let _ = self.play_pause(Instant::now());
                }
            }
        }
        self.pending_play = false;
    }

    /// The video is probed: build the one-clip project and show its first picture.
    fn loaded(&mut self, m: MediaItem, path: PathBuf) {
        let mut p = Project {
            next_media_id: self.pending_id.0.max(m.id.0),
            ..Project::default()
        };
        p.add_media(m.clone());
        if let Err(e) = p.timeline.append(&m) {
            self.notice = Some(e.to_string());
            return;
        }
        self.subs = Subs::for_media(
            m.format.subtitle_tracks.as_deref().unwrap_or(&[]),
            &m.format.picture_tracks,
        );
        // lines dragged earlier in this session are where they were left (REQ-251)
        if let Some(placed) = self.placed_memory.get(&memory_key(&path)) {
            self.subs.set_placed(placed.clone());
        }
        // the audio track chosen earlier in this session is played again (REQ-338)
        if let Some(track) = self.audio_memory.get(&memory_key(&path)).copied()
            && let Some(c) = p.timeline.clips.first().map(|c| c.id)
            && usize::from(track) < m.format.audio_tracks.as_ref().map_or(1, Vec::len)
        {
            let _ = p.timeline.set_audio_track(c, track);
        }
        // the volume and the sound are the ones remembered (REQ-252)
        p.mix.video_volume = self.settings.volume;
        p.mix.video_muted = self.settings.muted;
        self.project = p;
        self.playlist.play(&path);
        if let Some(dir) = path.parent() {
            let dir = dir.to_path_buf();
            self.remember_folder(&dir);
        }
        self.file = Some(path);
        self.status = format!("Opened {}", m.name);
        self.start_proxy(&m);
        self.refresh_still(Instant::now());
    }

    // ---- preview copy of big videos ----

    fn start_proxy(&mut self, m: &MediaItem) {
        let (Some(dir), Ok(info)) = (self.cache_dir.clone(), self.ffmpeg.as_ref()) else {
            return;
        };
        if !proxy::wants_proxy(&m.format, PreviewQuality::Automatic, &self.machine) {
            return;
        }
        let Ok(key) = proxy::key_of_kind(&m.path, m.format.is_360()) else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let worker = ProxyWorker {
            runner: self
                .sched
                .runner(Priority::Background, Arc::clone(&self.runner))
                .cancellable(Arc::clone(&cancel)),
            ffmpeg: info.ffmpeg.clone(),
            version: info.version,
            cache: ProxyCache::new(dir),
            threads: None,
            work_root: Some(self.work_root.clone()),
        };
        let task = ProxyTask {
            media: m.id,
            source: m.path.clone(),
            key,
            format: m.format.clone(),
            duration: m.duration,
            cancel: Arc::clone(&cancel),
            hwaccel: Vec::new(),
        };
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            if let Some(ev) = worker.generate(&task, &tx) {
                let _ = tx.send(ev);
            }
        });
        self.proxy_events = Some(rx);
        self.proxy_cancel = Some(cancel);
        if let Some(it) = self.project.media.first_mut() {
            it.proxy = ProxyState::Queued;
        }
    }

    fn tick_proxy(&mut self, now: Instant) {
        let Some(rx) = &self.proxy_events else {
            return;
        };
        let events: Vec<ProxyEvent> = rx.try_iter().collect();
        for ev in events {
            let (state, done) = match ev {
                ProxyEvent::Progress(_, p) => (ProxyState::Generating(p), false),
                ProxyEvent::Ready(_, p) => (ProxyState::Ready(p), true),
                ProxyEvent::Failed(_, e) => (ProxyState::Failed(e), true),
            };
            if let Some(m) = self.project.media.first_mut() {
                m.proxy = state;
            }
            if done {
                self.proxy_events = None;
                self.proxy_cancel = None;
                self.refresh_still(now);
                break;
            }
        }
    }

    // ---- the picture ----

    /// The project with what is being dragged applied (for the preview).
    pub fn shown_project(&self) -> std::borrow::Cow<'_, Project> {
        if self.drag_pos.is_none() && self.live.is_none() {
            return std::borrow::Cow::Borrowed(&self.project);
        }
        let mut q = self.project.clone();
        if let Some((ids, p)) = &self.drag_pos {
            let _ = q.subtitles.set_position(ids, Some(*p));
        }
        if let (Some(cam), Some(m)) = (self.live, self.project.media.first())
            && m.format.is_360()
        {
            q.camera.drop_media(m.id);
            let _ = q.camera.set(
                m.id,
                Pov {
                    t: Micros::ZERO,
                    cam,
                    ease: Default::default(),
                },
            );
        }
        std::borrow::Cow::Owned(q)
    }

    fn refresh_still(&mut self, now: Instant) {
        if !self.is_playing() && self.has_video() && self.ffmpeg.is_ok() {
            let shown = self.shown_project().into_owned();
            let _ = self.preview.request_still(&shown, self.playhead, now);
        }
    }

    // ---- transport ----

    fn pause(&mut self, now: Instant) {
        self.preview.pause(now);
        self.playhead = self.preview.position(now);
        self.refresh_still(now);
    }

    fn play_pause(&mut self, now: Instant) -> Result<(), String> {
        if !self.has_video() {
            return Ok(());
        }
        if self.is_playing() {
            self.pause(now);
            return Ok(());
        }
        let from = if self.playhead >= self.duration() {
            Micros::ZERO
        } else {
            self.playhead
        };
        let shown = self.shown_project().into_owned();
        self.playhead = from;
        self.preview.play(&shown, from, now)
    }

    fn seek(&mut self, to: Micros, now: Instant) -> Result<(), String> {
        if !self.has_video() {
            return Ok(());
        }
        let to = to.clamp(Micros::ZERO, self.duration());
        self.playhead = to;
        let shown = self.shown_project().into_owned();
        self.preview.seek(&shown, to, now)
    }

    /// Volume and mute reach the sound only when the stream restarts.
    fn restart_playback(&mut self, now: Instant) -> Result<(), String> {
        if self.is_playing() {
            let at = self.preview.position(now);
            self.playhead = at;
            let shown = self.shown_project().into_owned();
            return self.preview.play(&shown, at, now);
        }
        Ok(())
    }

    // ---- 360 ----

    fn set_camera(&mut self, cam: Camera, now: Instant) -> Result<(), String> {
        if !self.is_360() {
            return Err("Only a 360° video can be turned".into());
        }
        if self.live == Some(cam) {
            return Ok(());
        }
        self.live = Some(cam);
        if self.is_playing() {
            self.camera_restart = Some(now + CAMERA_RESTART);
        } else {
            self.refresh_still(now);
        }
        Ok(())
    }

    /// Where the 360 camera looks now.
    pub fn camera(&self) -> Camera {
        self.live.unwrap_or(Camera::FRONT)
    }

    fn tick_camera(&mut self, now: Instant) {
        let Some(due) = self.camera_restart else {
            return;
        };
        if now < due {
            return;
        }
        self.camera_restart = None;
        if self.is_playing() {
            let at = self.preview.position(now);
            self.playhead = at;
            let shown = self.shown_project().into_owned();
            if let Err(e) = self.preview.play(&shown, at, now) {
                self.status = e;
            }
        }
    }

    // ---- audio track (REQ-338) ----

    /// The audio tracks of the open video as `(ordinal, name)`; the first one when the
    /// file was not analysed for tracks; none for a video without sound.
    pub fn audio_tracks(&self) -> Vec<(u8, String)> {
        let Some(m) = self.project.media.first() else {
            return Vec::new();
        };
        match &m.format.audio_tracks {
            Some(v) => v.iter().map(|t| (t.ordinal, t.label())).collect(),
            None if m.format.has_audio() => vec![(0, "Track 1".to_string())],
            None => Vec::new(),
        }
    }

    /// The audio track the open video plays (0 = the first).
    pub fn audio_track(&self) -> u8 {
        self.project
            .timeline
            .clips
            .first()
            .map_or(0, |c| c.audio_track)
    }

    fn choose_audio(&mut self, track: u8, now: Instant) -> Result<(), String> {
        let id = self
            .project
            .timeline
            .clips
            .first()
            .map(|c| c.id)
            .ok_or("Open a video first")?;
        if !self.audio_tracks().iter().any(|(o, _)| *o == track) {
            return Err("That audio track no longer exists".into());
        }
        self.project
            .timeline
            .set_audio_track(id, track)
            .map_err(|e| e.to_string())?;
        if let Some(f) = &self.file {
            self.audio_memory.insert(memory_key(f), track);
        }
        // the sound changes at the place the video is at (REQ-256)
        self.restart_playback(now)
    }

    // ---- subtitles ----

    fn choose_subtitles(&mut self, choice: Option<usize>, now: Instant) -> Result<(), String> {
        let Some(i) = choice else {
            self.subs.selected = None;
            self.project.picture_sub = None;
            self.apply_cues(&[], now);
            return Ok(());
        };
        let source = self
            .subs
            .sources
            .get(i)
            .cloned()
            .ok_or("That subtitle list no longer exists")?;
        self.subs.selected = Some(i);
        // a picture track is drawn by the shared graph from the file itself (REQ-253); every
        // other choice takes it away
        self.project.picture_sub = match (&source, self.project.media.first()) {
            (Source::Picture(t), Some(m)) => Some((m.id, t.ordinal)),
            _ => None,
        };
        if matches!(source, Source::Picture(_)) {
            self.apply_cues(&[], now);
            return Ok(());
        }
        if let Some(c) = self.subs.cues(i).cloned() {
            self.apply_cues(&c, now);
            return Ok(());
        }
        match source {
            Source::File(p) => {
                let cues = subs::read_srt(&p)?;
                self.subs.set_cues(i, cues.clone());
                self.apply_cues(&cues, now);
                Ok(())
            }
            Source::Embedded(t) => self.read_embedded(i, t.ordinal),
            Source::Picture(_) => Ok(()),
        }
    }

    /// Reads an embedded text track with ffmpeg in the background.
    fn read_embedded(&mut self, index: usize, ordinal: u8) -> Result<(), String> {
        let (Ok(info), Some(path)) = (&self.ffmpeg, &self.file) else {
            return Err(NO_FFMPEG.into());
        };
        let (ffmpeg, path, root) = (info.ffmpeg.clone(), path.clone(), self.work_root.clone());
        let runner = self.sched.runner(Priority::Probe, Arc::clone(&self.runner));
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let r = read_track(&ffmpeg, &runner, &root, &path, ordinal);
            let _ = tx.send((index, r));
        });
        self.tracks = Some(rx);
        self.status = "Reading the subtitles…".into();
        Ok(())
    }

    fn tick_tracks(&mut self, now: Instant) {
        let Some(rx) = &self.tracks else {
            return;
        };
        let Ok((i, r)) = rx.try_recv() else {
            return;
        };
        self.tracks = None;
        match r {
            Ok(cues) => {
                self.subs.set_cues(i, cues.clone());
                // the user may have chosen something else meanwhile
                if self.subs.selected == Some(i) {
                    self.apply_cues(&cues, now);
                }
                self.status.clear();
            }
            Err(e) => {
                if self.subs.selected == Some(i) {
                    self.subs.selected = None;
                }
                self.status = e;
            }
        }
    }

    fn open_srt(&mut self, path: &Path, now: Instant) -> Result<(), String> {
        if !self.has_video() {
            return Err("Open a video first".into());
        }
        let cues = subs::read_srt(path)?;
        let i = self.subs.add_file(path);
        self.subs.set_cues(i, cues);
        self.choose_subtitles(Some(i), now)
    }

    /// Puts `cues` on the picture, with the places the user gave the lines.
    fn apply_cues(&mut self, cues: &[Cue], now: Instant) {
        let Some(m) = self.project.media.first().map(|m| m.id) else {
            return;
        };
        self.project
            .subtitles
            .replace_all(subs::to_lines(m, cues), None);
        let ids: Vec<LineId> = self.project.subtitles.lines.iter().map(|l| l.id).collect();
        for (n, id) in ids.into_iter().enumerate() {
            if let Some(p) = self.subs.placed(n) {
                let _ = self.project.subtitles.set_position(&[id], Some(p));
            }
        }
        self.preview.forget_frame();
        if self.is_playing() {
            let _ = self.restart_playback(now);
        } else {
            self.refresh_still(now);
        }
    }

    fn place_lines(&mut self, ids: &[LineId], pos: Pos, now: Instant) -> Result<(), String> {
        self.drag_pos = None;
        self.project
            .subtitles
            .set_position(ids, Some(pos))
            .map_err(|e| e.to_string())?;
        for id in ids {
            if let Some(n) = self
                .project
                .subtitles
                .lines
                .iter()
                .position(|l| l.id == *id)
            {
                self.subs.place(n, pos);
            }
        }
        self.preview.forget_frame();
        self.refresh_still(now);
        Ok(())
    }

    /// The lines on screen now, with their pictures' boxes (for dragging).
    pub fn line_boxes(
        &self,
        measure: &mut dyn FnMut(&str, f32, f32) -> eframe::egui::Vec2,
    ) -> Vec<(LineId, eframe::egui::Rect)> {
        let Some(frame) = self.output_size() else {
            return Vec::new();
        };
        let shown = self.shown_project();
        videolore::ui::placement::boxes(&shown, self.playhead, frame, measure)
    }

    /// No background reading is going on (tests wait for it).
    pub fn idle_for_tests(&self) -> bool {
        self.probe.is_none() && self.tracks.is_none()
    }
}

/// Converts the text track `ordinal` of `path` to cues in a private work folder.
fn read_track(
    ffmpeg: &Path,
    runner: &dyn Runner,
    root: &Path,
    path: &Path,
    ordinal: u8,
) -> Result<Vec<Cue>, String> {
    let work = videolore::base::private::Work::new(root).map_err(|e| e.to_string())?;
    let cues = videolore::stt::read_track(ffmpeg, runner, work.path(), path, ordinal, None)
        .map_err(|e| e.to_string())?;
    if cues.is_empty() {
        return Err("This subtitle track has no text".into());
    }
    Ok(cues)
}
