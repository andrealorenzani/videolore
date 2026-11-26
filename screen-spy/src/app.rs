// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The state of the recorder: idle, showing the screen, checking before a start,
//! recording, or closing the file. The window only sends [`Command`]s and calls
//! [`RecorderState::tick`]; the clock, the processes, the screens, the sound and the
//! dialogs are injected.
//!
//! At most one capture process runs at a time: a preview-only one before and between
//! recordings, replaced by the recording one (which shows the preview too) at Start.
//! The checks that can take long (ffmpeg, the sound sources, a permission question) run
//! on a thread of their own; the window only polls for their result.

use crate::audio::{CANCELLED, Session, Sound};
use crate::devices;
use crate::plan::{self, AudioFormat, Capture, Os, Recording, VideoFormat};
use crate::preview::{self, Feed, Frame};
use crate::process::{Child, Exit, STOP_TIMEOUT, Spawner, failure_cause};
use crate::screens::{Screen, ScreenSource};
use crate::session::Platform;
use crate::settings::RecorderSettings;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};
use videolore::ffmpeg::{Runner, argv, parse_encoders};

/// How often the list of screens is read again.
pub const SCAN_EVERY: Duration = Duration::from_secs(2);
/// How long ffmpeg may take to list what it can do.
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// Shown when no ffmpeg was found.
pub const NO_FFMPEG: &str =
    "videolore screen spy needs ffmpeg and could not find it. Reinstall videolore screen spy.";
/// Shown when no screen can be read.
pub const NO_SCREEN: &str = "No screen was found.";
/// Why a recording stops when its screen is gone (REQ-350).
pub const SCREEN_REMOVED: &str = "The screen was removed";
/// Why a start is refused when the chosen screen went away.
pub const CHOSEN_REMOVED: &str = "The chosen screen was removed. Choose a screen and start again.";

/// What the user can ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Chooses the screen with this identifier.
    SelectScreen(u32),
    /// The text typed in the file name box.
    SetName(String),
    /// Choose…: the save dialog.
    PickFile,
    SetVideo(VideoFormat),
    SetAudio(AudioFormat),
    /// The Start recording / Stop recording button (it cancels while the start is being
    /// checked).
    ToggleRecording,
    /// "Replace the file?": Replace.
    Replace,
    /// "Replace the file?": Cancel.
    CancelReplace,
    /// The window's close control, or File > Quit.
    Close,
    /// "Stop the recording and close?": Stop and close.
    StopAndClose,
    /// "Stop the recording and close?": Cancel.
    CancelClose,
    DismissMessage,
    ShowAbout,
    CloseAbout,
}

/// The save dialog (faked in tests).
pub trait Dialogs {
    fn save_file(&mut self, folder: &Path, name: &str, extension: &str) -> Option<PathBuf>;
}

/// The permission to record the screen (macOS asks for it).
pub trait Permission {
    fn screen(&mut self) -> Result<(), String>;
}

/// Windows and Linux ask nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoPermissionNeeded;

impl Permission for NoPermissionNeeded {
    fn screen(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// What the recorder works with.
pub struct Deps {
    pub spawner: Box<dyn Spawner>,
    /// Short runs of ffmpeg (what it can do).
    pub runner: Arc<dyn Runner>,
    pub screens: Box<dyn ScreenSource>,
    pub sound: Arc<dyn Sound>,
    pub permission: Box<dyn Permission>,
    pub dialogs: Box<dyn Dialogs>,
    /// The local date and time, as in a default file name.
    pub stamp: Box<dyn Fn() -> String>,
}

/// What it starts with.
pub struct Setup {
    pub platform: Platform,
    pub ffmpeg: Option<PathBuf>,
    pub settings: RecorderSettings,
    /// Where the settings are kept; `None` = they are not.
    pub settings_path: Option<PathBuf>,
    /// The folder offered when none is remembered.
    pub default_folder: PathBuf,
}

/// What the program is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Previewing,
    /// The checks before a recording run.
    Starting,
    Recording,
    /// Stop was asked: ffmpeg is closing the file.
    Stopping,
}

/// ffmpeg's capture devices and encoders.
type Abilities = (BTreeSet<String>, BTreeSet<String>);

/// What the checks before a recording found.
struct Prepared {
    abilities: Abilities,
    sound: Session,
}

/// A start whose checks run on their own thread.
struct Start {
    result: Receiver<Result<Prepared, String>>,
    cancel: Arc<AtomicBool>,
    path: PathBuf,
    overwrite: bool,
    screen: u32,
}

/// What the thread of the checks works with.
struct Checks {
    runner: Arc<dyn Runner>,
    sound: Arc<dyn Sound>,
    ffmpeg: PathBuf,
    os: Os,
    audio: AudioFormat,
    /// Known from an earlier start.
    known: Option<Abilities>,
}

impl Checks {
    /// ffmpeg's devices and encoders, asked until they are known.
    fn abilities(&mut self) -> Result<Abilities, String> {
        if let Some(known) = self.known.take() {
            return Ok(known);
        }
        let ask = |what: &str| {
            let args = argv(["-hide_banner", what]);
            match self.runner.run_within(&self.ffmpeg, &args, CHECK_TIMEOUT) {
                Ok(o) if o.success => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
                Ok(o) => Err(format!("ffmpeg could not be asked: {}", o.stderr.trim())),
                Err(e) => Err(format!("ffmpeg could not be started: {e}")),
            }
        };
        Ok((
            devices::parse_devices(&ask("-devices")?),
            parse_encoders(&ask("-encoders")?),
        ))
    }

    /// ffmpeg can record what was chosen, and both sound sources open.
    fn run(mut self, cancel: &AtomicBool) -> Result<Prepared, String> {
        let abilities = self.abilities()?;
        let missing = devices::missing(self.os, self.audio, &abilities.0, &abilities.1);
        if !missing.is_empty() {
            return Err(format!(
                "This ffmpeg cannot record: it lacks {}.",
                missing.join(", ")
            ));
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(CANCELLED.into());
        }
        let sound = self.sound.open(&self.ffmpeg, cancel)?;
        Ok(Prepared { abilities, sound })
    }
}

/// The recording that runs.
struct Job {
    child: Box<dyn Child>,
    /// The open sound, alive as long as the process.
    _sound: Session,
    path: PathBuf,
    screen: u32,
    since: Instant,
}

enum Run {
    Idle,
    Previewing(Box<dyn Child>),
    Starting(Start),
    Recording(Job),
    Stopping {
        job: Job,
        deadline: Instant,
        /// Why it was stopped, when the user did not ask.
        cause: Option<String>,
    },
}

/// The recorder.
pub struct RecorderState {
    deps: Deps,
    pub platform: Platform,
    ffmpeg: Option<PathBuf>,
    /// ffmpeg's capture devices and encoders, once they are known.
    abilities: Option<Abilities>,
    pub screens: Vec<Screen>,
    /// The identifier of the chosen screen.
    pub chosen: Option<u32>,
    next_scan: Option<Instant>,
    /// The text of the file name box.
    pub name: String,
    /// The user typed or chose the name: it is not replaced by a new default.
    name_edited: bool,
    pub folder: PathBuf,
    pub video: VideoFormat,
    pub audio: AudioFormat,
    settings_path: Option<PathBuf>,
    default_folder: PathBuf,
    run: Run,
    feed: Option<Arc<Feed>>,
    /// The newest preview picture and its number.
    pub frame: Option<Frame>,
    pub frame_serial: u64,
    /// Why there is no preview.
    pub preview_error: Option<String>,
    /// A message the user must close.
    pub message: Option<String>,
    /// The last thing that happened.
    pub status: String,
    /// "Replace the file?" is asked about this file.
    pub replace: Option<PathBuf>,
    /// "Stop the recording and close?" is asked.
    pub ask_close: bool,
    /// Close when the file is finished.
    quit_after: bool,
    pub show_about: bool,
    pub quit: bool,
}

impl RecorderState {
    pub fn new(deps: Deps, setup: Setup) -> RecorderState {
        let Setup {
            platform,
            ffmpeg,
            settings,
            settings_path,
            default_folder,
        } = setup;
        let name = plan::default_name(&(deps.stamp)(), settings.audio);
        let message = ffmpeg.is_none().then(|| NO_FFMPEG.to_string());
        RecorderState {
            deps,
            platform,
            ffmpeg,
            abilities: None,
            screens: Vec::new(),
            chosen: None,
            next_scan: None,
            name,
            name_edited: false,
            folder: settings.folder.unwrap_or_else(|| default_folder.clone()),
            video: settings.video,
            audio: settings.audio,
            settings_path,
            default_folder,
            run: Run::Idle,
            feed: None,
            frame: None,
            frame_serial: 0,
            preview_error: None,
            message,
            status: String::new(),
            replace: None,
            ask_close: false,
            quit_after: false,
            show_about: false,
            quit: false,
        }
    }

    pub fn phase(&self) -> Phase {
        match self.run {
            Run::Idle => Phase::Idle,
            Run::Previewing(_) => Phase::Previewing,
            Run::Starting(_) => Phase::Starting,
            Run::Recording(_) => Phase::Recording,
            Run::Stopping { .. } => Phase::Stopping,
        }
    }

    /// A recording starts, runs or is being closed: the choices are locked.
    pub fn locked(&self) -> bool {
        matches!(self.run, Run::Starting(_)) || self.recording()
    }

    /// A recording runs or its file is being closed: closing the window asks first.
    pub fn recording(&self) -> bool {
        matches!(self.run, Run::Recording(_) | Run::Stopping { .. })
    }

    /// A cancelled start is waiting for its checks to end.
    pub fn cancelling(&self) -> bool {
        matches!(&self.run, Run::Starting(s) if s.cancel.load(Ordering::SeqCst))
    }

    /// How long the recording has been running.
    pub fn recording_for(&self, now: Instant) -> Option<Duration> {
        match &self.run {
            Run::Recording(job) => Some(now.saturating_duration_since(job.since)),
            _ => None,
        }
    }

    /// The label of the one button (REQ-349).
    pub fn button_label(&self) -> &'static str {
        match self.run {
            Run::Starting(_) => "Cancel start",
            Run::Recording(_) | Run::Stopping { .. } => "Stop recording",
            Run::Idle | Run::Previewing(_) => "Start recording",
        }
    }

    /// The position of the chosen screen in the list.
    pub fn chosen_index(&self) -> Option<usize> {
        self.screens.iter().position(|s| Some(s.id) == self.chosen)
    }

    fn capture(&self) -> Option<Capture> {
        let i = self.chosen_index()?;
        Some(self.screens[i].capture(i))
    }

    /// Reads the screens, collects the preview and watches the processes and the checks;
    /// called for every frame of the window.
    pub fn tick(&mut self, now: Instant) {
        if self.next_scan.is_none_or(|t| now >= t) {
            self.next_scan = Some(now + SCAN_EVERY);
            let first = self.chosen.is_none();
            if self.scan(now) || first {
                self.start_preview();
            }
        }
        if let Some(frame) = self.feed.as_ref().and_then(|f| f.take()) {
            self.frame = Some(frame);
            self.frame_serial += 1;
        }
        self.watch(now);
        self.poll_start(now);
    }

    /// Reads the list of screens; true when the preview must start again because the
    /// screen shown changed.
    fn scan(&mut self, now: Instant) -> bool {
        let list = self.deps.screens.list();
        if list == self.screens {
            return false;
        }
        let before = self.capture();
        self.screens = list;
        if let Run::Recording(job) = &self.run
            && !self.screens.iter().any(|s| s.id == job.screen)
        {
            self.stop(Some(SCREEN_REMOVED.into()), now);
        }
        if self.locked() {
            return false;
        }
        self.keep_a_screen();
        before != self.capture()
    }

    /// The first screen is chosen when the chosen one is not there.
    fn keep_a_screen(&mut self) {
        if self.chosen_index().is_none() {
            self.chosen = self.screens.first().map(|s| s.id);
        }
    }

    /// Ends the preview process, if one runs.
    fn stop_preview(&mut self) {
        if let Run::Previewing(child) = &mut self.run {
            child.kill();
            self.run = Run::Idle;
        }
        self.feed = None;
    }

    /// Shows the chosen screen (again).
    fn start_preview(&mut self) {
        if self.locked() {
            return;
        }
        self.stop_preview();
        self.frame = None;
        self.frame_serial += 1;
        self.preview_error = None;
        if !self.platform.can_preview() {
            return;
        }
        let (Some(ffmpeg), Some(screen)) = (self.ffmpeg.clone(), self.capture()) else {
            return;
        };
        let args = plan::preview_args(self.platform.os, &screen, &self.platform.display);
        match self.deps.spawner.spawn(&ffmpeg, &args) {
            Ok(mut child) => {
                self.attach_feed(child.as_mut(), &screen);
                self.run = Run::Previewing(child);
            }
            Err(e) => self.preview_error = Some(format!("The preview is not available: {e}")),
        }
    }

    fn attach_feed(&mut self, child: &mut dyn Child, screen: &Capture) {
        let (w, h) = plan::preview_size(screen.width, screen.height);
        self.feed = child.take_stdout().map(|out| preview::start(out, w, h));
    }

    fn watch(&mut self, now: Instant) {
        match &mut self.run {
            Run::Idle | Run::Starting(_) => {}
            Run::Previewing(child) => {
                if let Some(exit) = child.try_wait() {
                    self.preview_error = Some(format!(
                        "The preview is not available. {}",
                        failure_cause(&exit.stderr)
                    ));
                    self.run = Run::Idle;
                    self.feed = None;
                }
            }
            Run::Recording(job) => {
                if let Some(exit) = job.child.try_wait() {
                    let cause = if exit.success {
                        "The capture ended by itself".to_string()
                    } else {
                        failure_cause(&exit.stderr)
                    };
                    self.finish(Some(cause));
                }
            }
            Run::Stopping {
                job,
                deadline,
                cause,
            } => {
                let cause = cause.clone();
                if let Some(Exit { success, stderr }) = job.child.try_wait() {
                    let cause = cause.or_else(|| (!success).then(|| failure_cause(&stderr)));
                    self.finish(cause);
                } else if now >= *deadline {
                    job.child.kill();
                    self.finish(cause.or_else(|| {
                        Some(format!(
                            "ffmpeg did not finish within {} seconds and was ended",
                            STOP_TIMEOUT.as_secs()
                        ))
                    }));
                }
            }
        }
    }

    /// After a start or a recording: the choices are free again.
    fn show_again(&mut self) {
        // the screens may have changed while the choice was locked
        self.next_scan = None;
        self.keep_a_screen();
        self.start_preview();
    }

    /// The recording process is gone; `cause` when it did not end as asked.
    fn finish(&mut self, cause: Option<String>) {
        let (Run::Recording(job) | Run::Stopping { job, .. }) =
            std::mem::replace(&mut self.run, Run::Idle)
        else {
            return;
        };
        let path = job.path.display().to_string();
        let kept = job.path.metadata().is_ok_and(|m| m.len() > 0);
        drop(job);
        self.feed = None;
        self.ask_close = false;
        match cause {
            None => {
                self.status = format!("Saved {path}");
                self.quit |= self.quit_after;
            }
            Some(cause) => {
                let what = if kept {
                    format!("The part already written is in {path}.")
                } else {
                    "Nothing was written.".to_string()
                };
                self.status = "The recording stopped".to_string();
                self.message = Some(format!("The recording stopped. {cause}. {what}"));
            }
        }
        self.quit_after = false;
        self.refresh_name();
        self.show_again();
    }

    /// A name nobody typed carries the time of now.
    fn refresh_name(&mut self) {
        if !self.name_edited {
            self.name = plan::default_name(&(self.deps.stamp)(), self.audio);
        }
    }

    fn stop(&mut self, why: Option<String>, now: Instant) {
        if !matches!(self.run, Run::Recording(_)) {
            return;
        }
        let Run::Recording(mut job) = std::mem::replace(&mut self.run, Run::Idle) else {
            return;
        };
        job.child.quit();
        self.status = "Finishing the file…".to_string();
        self.run = Run::Stopping {
            job,
            deadline: now + STOP_TIMEOUT,
            cause: why,
        };
    }

    /// The file the box names, when it can be recorded to.
    fn target(&self) -> Result<PathBuf, String> {
        let name = plan::file_name(&self.name, self.audio)?;
        if !self.folder.is_dir() {
            return Err(format!(
                "The folder {} does not exist. Choose another one with Choose…",
                self.folder.display()
            ));
        }
        let path = std::path::absolute(self.folder.join(name)).map_err(|e| e.to_string())?;
        if path.is_dir() {
            return Err(format!("{} is a folder", path.display()));
        }
        Ok(path)
    }

    /// Start recording: every check comes before anything is written (ADR-264). The
    /// quick ones run here (macOS wants the screens and their permission asked on the
    /// window's thread), the others on a thread of their own.
    fn start(&mut self, replace: Option<PathBuf>) -> Result<(), String> {
        if let Some(why) = &self.platform.blocked {
            return Err(why.clone());
        }
        let overwrite = replace.is_some();
        let path = match replace {
            Some(p) => p,
            None => {
                self.refresh_name();
                let path = self.target()?;
                if path.exists() {
                    self.replace = Some(path);
                    return Ok(());
                }
                path
            }
        };
        let ffmpeg = self.ffmpeg.clone().ok_or_else(|| NO_FFMPEG.to_string())?;
        // one capture at a time, and no preview while the checks run
        self.stop_preview();
        let result = self.check(ffmpeg, path, overwrite);
        if result.is_err() {
            self.start_preview();
        }
        result
    }

    fn check(&mut self, ffmpeg: PathBuf, path: PathBuf, overwrite: bool) -> Result<(), String> {
        let wanted = self.chosen;
        self.screens = self.deps.screens.list();
        self.keep_a_screen();
        let Some(screen) = self.chosen else {
            return Err(NO_SCREEN.into());
        };
        if wanted != self.chosen {
            return Err(CHOSEN_REMOVED.into());
        }
        self.deps.permission.screen()?;
        let checks = Checks {
            runner: Arc::clone(&self.deps.runner),
            sound: Arc::clone(&self.deps.sound),
            ffmpeg,
            os: self.platform.os,
            audio: self.audio,
            known: self.abilities.clone(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let (tx, result) = channel();
        std::thread::spawn(move || {
            // a sound opened for a start nobody waits for is closed with the result
            let _ = tx.send(checks.run(&flag));
        });
        self.status = if self.platform.os == Os::MacOs {
            "Checking the sound… If macOS asks to use the microphone, answer it."
        } else {
            "Checking ffmpeg and the sound…"
        }
        .to_string();
        self.run = Run::Starting(Start {
            result,
            cancel,
            path,
            overwrite,
            screen,
        });
        Ok(())
    }

    /// Takes the result of the checks, once there is one, and starts the recording.
    fn poll_start(&mut self, now: Instant) {
        let Run::Starting(start) = &self.run else {
            return;
        };
        let result = match start.result.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("The checks before the recording failed".to_string())
            }
        };
        let Run::Starting(start) = std::mem::replace(&mut self.run, Run::Idle) else {
            return;
        };
        if start.cancel.load(Ordering::SeqCst) {
            self.status = CANCELLED.to_string();
        } else if let Err(e) = result.and_then(|prepared| self.begin(start, prepared, now)) {
            self.status.clear();
            self.message = Some(e);
        }
        if !self.recording() {
            self.show_again();
        }
    }

    /// The checks passed: the recording process starts.
    fn begin(&mut self, start: Start, prepared: Prepared, now: Instant) -> Result<(), String> {
        let Prepared { abilities, sound } = prepared;
        self.abilities = Some(abilities);
        let ffmpeg = self.ffmpeg.clone().ok_or_else(|| NO_FFMPEG.to_string())?;
        self.screens = self.deps.screens.list();
        let screen = self
            .screens
            .iter()
            .position(|s| s.id == start.screen)
            .map(|i| self.screens[i].capture(i))
            .ok_or_else(|| CHOSEN_REMOVED.to_string())?;
        let args = plan::record_args(&Recording {
            os: self.platform.os,
            screen,
            display: &self.platform.display,
            video: self.video,
            audio: self.audio,
            sources: &sound.inputs,
            output: &start.path,
            overwrite: start.overwrite,
        });
        let mut child = self
            .deps
            .spawner
            .spawn(&ffmpeg, &args)
            .map_err(|e| format!("ffmpeg could not be started: {e}"))?;
        self.frame = None;
        self.frame_serial += 1;
        self.preview_error = None;
        self.attach_feed(child.as_mut(), &screen);
        self.status = format!("Recording to {}", start.path.display());
        self.run = Run::Recording(Job {
            child,
            _sound: sound,
            path: start.path,
            screen: start.screen,
            since: now,
        });
        Ok(())
    }

    /// Asks the checks of a start to end; their result is thrown away.
    fn cancel_start(&mut self) {
        if let Run::Starting(start) = &self.run {
            start.cancel.store(true, Ordering::SeqCst);
            self.status = "Cancelling…".to_string();
        }
    }

    fn save_settings(&mut self) {
        let Some(path) = &self.settings_path else {
            return;
        };
        let settings = RecorderSettings {
            // the system's folder is not written down: it follows the system
            folder: (self.folder != self.default_folder).then(|| self.folder.clone()),
            video: self.video,
            audio: self.audio,
        };
        if let Err(e) = settings.save(path) {
            self.status = format!("The settings could not be saved: {e}");
        }
    }

    fn pick_file(&mut self) {
        let suggested = plan::with_extension(&self.name, self.audio);
        let Some(chosen) =
            self.deps
                .dialogs
                .save_file(&self.folder, &suggested, self.audio.extension())
        else {
            return;
        };
        let (Some(folder), Some(name)) = (chosen.parent(), chosen.file_name()) else {
            return;
        };
        self.folder = folder.to_path_buf();
        self.name = plan::with_extension(&name.to_string_lossy(), self.audio);
        self.name_edited = true;
        self.save_settings();
    }

    /// Runs a command.
    pub fn dispatch(&mut self, command: Command, now: Instant) {
        use Command as C;
        let locked = self.locked();
        match command {
            C::SelectScreen(id) if !locked => {
                if self.chosen != Some(id) && self.screens.iter().any(|s| s.id == id) {
                    self.chosen = Some(id);
                    self.start_preview();
                }
            }
            C::SetName(name) if !locked => {
                self.name = name;
                self.name_edited = true;
            }
            C::PickFile if !locked => self.pick_file(),
            C::SetVideo(video) if !locked => {
                self.video = video;
                self.save_settings();
            }
            C::SetAudio(audio) if !locked => {
                self.audio = audio;
                self.name = plan::with_extension(&self.name, audio);
                self.save_settings();
            }
            C::SelectScreen(_) | C::SetName(_) | C::PickFile | C::SetVideo(_) | C::SetAudio(_) => {}
            C::ToggleRecording => match self.run {
                Run::Recording(_) => self.stop(None, now),
                Run::Stopping { .. } => {}
                Run::Starting(_) => self.cancel_start(),
                Run::Idle | Run::Previewing(_) => {
                    if let Err(e) = self.start(None) {
                        self.message = Some(e);
                    }
                }
            },
            C::Replace => {
                if let Some(path) = self.replace.take()
                    && !locked
                    && let Err(e) = self.start(Some(path))
                {
                    self.message = Some(e);
                }
            }
            C::CancelReplace => self.replace = None,
            C::Close => {
                if self.recording() {
                    self.ask_close = true;
                } else {
                    // nothing is being recorded: a start being checked is dropped
                    self.cancel_start();
                    self.stop_preview();
                    self.quit = true;
                }
            }
            C::StopAndClose => {
                self.ask_close = false;
                if self.recording() {
                    self.quit_after = true;
                    self.stop(None, now);
                } else {
                    self.dispatch(C::Close, now);
                }
            }
            C::CancelClose => self.ask_close = false,
            C::DismissMessage => self.message = None,
            C::ShowAbout => self.show_about = true,
            C::CloseAbout => self.show_about = false,
        }
    }
}
