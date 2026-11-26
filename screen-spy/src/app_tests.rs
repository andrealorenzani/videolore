// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The recorder's state machine with fake processes, screens, sound, permission, dialogs
//! and clock.

use crate::app::{
    CHOSEN_REMOVED, Command, Deps, Dialogs, NO_FFMPEG, NO_SCREEN, Permission, Phase, RecorderState,
    SCAN_EVERY, SCREEN_REMOVED, Setup,
};
use crate::audio::{CANCELLED, Session, Sound};
use crate::plan::{AudioFormat, AudioInput, Os, PulseSource, VideoFormat};
use crate::process::{Child, Exit, STOP_TIMEOUT, Spawner};
use crate::screens::{Screen, ScreenSource};
use crate::session::Platform;
use crate::settings::RecorderSettings;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};
use videolore::ffmpeg::{CmdOutput, Process, Runner};

type Answer = dyn Fn(&[String]) -> Option<CmdOutput> + Send + Sync;

/// ffmpeg's short runs, answered by a function of the arguments.
pub(crate) struct Scripted {
    answer: Box<Answer>,
    pub calls: AtomicUsize,
}

impl Scripted {
    pub fn new(
        answer: impl Fn(&[String]) -> Option<CmdOutput> + Send + Sync + 'static,
    ) -> Scripted {
        Scripted {
            answer: Box::new(answer),
            calls: AtomicUsize::new(0),
        }
    }
}

/// A run that succeeded and printed `stdout`.
pub(crate) fn ok(stdout: &str) -> Option<CmdOutput> {
    Some(CmdOutput {
        success: true,
        stdout: stdout.into(),
        stderr: String::new(),
    })
}

/// A run that failed with `stderr`.
pub(crate) fn fail(stderr: &str) -> Option<CmdOutput> {
    Some(CmdOutput {
        success: false,
        stdout: Vec::new(),
        stderr: stderr.into(),
    })
}

impl Runner for Scripted {
    fn run(&self, _: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        (self.answer)(&args).ok_or_else(|| io::Error::other("cannot run"))
    }

    fn run_within(&self, p: &Path, a: &[OsString], _: Duration) -> io::Result<CmdOutput> {
        self.run(p, a)
    }

    fn spawn_in(&self, _: &Path, _: &[OsString], _: Option<&Path>) -> io::Result<Box<dyn Process>> {
        Err(io::Error::other("the recorder spawns elsewhere"))
    }
}

/// One fake capture process, seen by the test.
#[derive(Default)]
pub(crate) struct Proc {
    pub args: Vec<String>,
    pub quit: bool,
    pub killed: bool,
    pub exit: Option<Exit>,
}

impl Proc {
    pub fn recording(&self) -> bool {
        self.args.iter().any(|a| a == "-filter_complex")
    }

    pub fn alive(&self) -> bool {
        !self.killed && self.exit.is_none()
    }
}

/// The sound sources, used from the thread of the checks.
#[derive(Default)]
pub(crate) struct FakeSound {
    error: Mutex<Option<String>>,
    /// Open sound sessions.
    live: Arc<AtomicUsize>,
    /// While set, opening waits (until released or cancelled).
    pub hold: AtomicBool,
    /// Opening does not look at the cancel request.
    pub deaf: AtomicBool,
    pub panics: AtomicBool,
    /// The thread that opened last.
    pub thread: Mutex<Option<ThreadId>>,
}

impl FakeSound {
    pub fn fail(&self, error: Option<&str>) {
        *self.error.lock().unwrap() = error.map(str::to_string);
    }

    pub fn live(&self) -> usize {
        self.live.load(Ordering::SeqCst)
    }
}

/// Everything the fakes share with a test.
#[derive(Default)]
pub(crate) struct World {
    pub procs: RefCell<Vec<Rc<RefCell<Proc>>>>,
    pub spawn_fails: Cell<bool>,
    pub screens: RefCell<Vec<Screen>>,
    pub sound: Arc<FakeSound>,
    pub denied: Cell<bool>,
    /// The answer of the save dialog, and what it was asked.
    pub pick: RefCell<Option<PathBuf>>,
    pub asked: RefCell<Vec<(PathBuf, String, String)>>,
    pub seconds: Cell<u32>,
}

impl World {
    pub fn last(&self) -> Rc<RefCell<Proc>> {
        Rc::clone(self.procs.borrow().last().expect("a process"))
    }

    pub fn count(&self) -> usize {
        self.procs.borrow().len()
    }

    pub fn alive(&self) -> usize {
        let procs = self.procs.borrow();
        procs.iter().filter(|p| p.borrow().alive()).count()
    }

    pub fn end(&self, success: bool, stderr: &str) {
        self.last().borrow_mut().exit = Some(Exit {
            success,
            stderr: stderr.into(),
        });
    }

    /// The file of the last recording gets content, as ffmpeg would write it.
    pub fn write_file(&self) -> PathBuf {
        let p = self.last();
        let p = p.borrow();
        let file = p.args.iter().find_map(|a| a.strip_prefix("file:")).unwrap();
        std::fs::write(file, "video").unwrap();
        file.into()
    }
}

struct FakeChild(Rc<RefCell<Proc>>);

impl Child for FakeChild {
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        // one preview picture of a 1920x1080 or 1280x1024 screen
        let h = if self.0.borrow().args.join(" ").contains("scale=640:512") {
            512
        } else {
            360
        };
        Some(Box::new(Cursor::new(vec![40u8; 640 * h * 3])))
    }
    fn quit(&mut self) {
        self.0.borrow_mut().quit = true;
    }
    fn try_wait(&mut self) -> Option<Exit> {
        self.0.borrow().exit.clone()
    }
    fn kill(&mut self) {
        self.0.borrow_mut().killed = true;
    }
}

struct Fakes(Rc<World>);

impl Spawner for Fakes {
    fn spawn(&mut self, _: &Path, args: &[OsString]) -> io::Result<Box<dyn Child>> {
        if self.0.spawn_fails.get() {
            return Err(io::Error::other("not allowed to run"));
        }
        assert_eq!(self.0.alive(), 0, "at most one capture at a time");
        let proc = Rc::new(RefCell::new(Proc {
            args: args
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect(),
            ..Default::default()
        }));
        self.0.procs.borrow_mut().push(Rc::clone(&proc));
        Ok(Box::new(FakeChild(proc)))
    }
}

impl ScreenSource for Fakes {
    fn list(&mut self) -> Vec<Screen> {
        self.0.screens.borrow().clone()
    }
}

/// Counts the open sessions.
struct Open(Arc<AtomicUsize>);

impl Drop for Open {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Sound for FakeSound {
    fn open(&self, _: &Path, cancel: &AtomicBool) -> Result<Session, String> {
        *self.thread.lock().unwrap() = Some(std::thread::current().id());
        let end = Instant::now() + Duration::from_secs(10);
        while self.hold.load(Ordering::SeqCst) {
            if !self.deaf.load(Ordering::SeqCst) && cancel.load(Ordering::SeqCst) {
                return Err(CANCELLED.into());
            }
            assert!(Instant::now() < end, "never released");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!self.panics.load(Ordering::SeqCst), "the sound broke");
        if let Some(e) = self.error.lock().unwrap().clone() {
            return Err(e);
        }
        self.live.fetch_add(1, Ordering::SeqCst);
        Ok(Session {
            inputs: vec![
                AudioInput::Pulse(PulseSource::Computer),
                AudioInput::Pulse(PulseSource::Microphone),
            ],
            keep: vec![Box::new(Open(Arc::clone(&self.live)))],
        })
    }
}

impl Permission for Fakes {
    fn screen(&mut self) -> Result<(), String> {
        if self.0.denied.get() {
            Err(crate::session::SCREEN_NOT_ALLOWED.into())
        } else {
            Ok(())
        }
    }
}

impl Dialogs for Fakes {
    fn save_file(&mut self, folder: &Path, name: &str, extension: &str) -> Option<PathBuf> {
        let asked = (folder.to_path_buf(), name.into(), extension.into());
        self.0.asked.borrow_mut().push(asked);
        self.0.pick.borrow().clone()
    }
}

pub(crate) fn screen(id: u32, x: i32, width: u32, height: u32) -> Screen {
    Screen {
        id,
        x,
        y: 0,
        width,
        height,
    }
}

const DEVICES: &str = " --\n D  x11grab X\n D  pulse P\n";
const ENCODERS: &str = " V....D libx264 H.264\n A....D aac AAC\n A....D pcm_s16le PCM\n";

pub(crate) struct Rig {
    pub world: Rc<World>,
    pub app: RecorderState,
    pub dir: tempfile::TempDir,
    pub runner: Arc<Scripted>,
    pub now: Instant,
}

pub(crate) struct Options {
    pub env: Vec<(&'static str, &'static str)>,
    pub os: Os,
    pub ffmpeg: bool,
    pub runner: Scripted,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            env: vec![("DISPLAY", ":0")],
            os: Os::Linux,
            ffmpeg: true,
            runner: Scripted::new(|a| {
                ok(if a.contains(&"-devices".to_string()) {
                    DEVICES
                } else {
                    ENCODERS
                })
            }),
        }
    }
}

pub(crate) fn rig_with(options: Options) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let world = Rc::new(World::default());
    *world.screens.borrow_mut() = vec![screen(10, 0, 1920, 1080), screen(20, 1920, 1280, 1024)];
    let runner = Arc::new(options.runner);
    let (w, seconds) = (Rc::clone(&world), Rc::clone(&world));
    let fakes = move || Box::new(Fakes(Rc::clone(&w)));
    let deps = Deps {
        spawner: fakes(),
        runner: runner.clone(),
        screens: fakes(),
        sound: world.sound.clone(),
        permission: fakes(),
        dialogs: fakes(),
        stamp: Box::new(move || {
            seconds.seconds.set(seconds.seconds.get() + 1);
            format!("2026-11-26-1015{:02}", seconds.seconds.get())
        }),
    };
    let env = options.env.clone();
    let platform = Platform::detect(
        options.os,
        |k| {
            env.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| OsString::from(v))
        },
        None,
    );
    let setup = Setup {
        platform,
        ffmpeg: options.ffmpeg.then(|| PathBuf::from("ffmpeg")),
        settings: RecorderSettings::default(),
        settings_path: Some(dir.path().join("config/screen-spy.json")),
        default_folder: dir.path().to_path_buf(),
    };
    let now = Instant::now();
    let mut app = RecorderState::new(deps, setup);
    app.tick(now);
    Rig {
        world,
        app,
        dir,
        runner,
        now,
    }
}

pub(crate) fn rig() -> Rig {
    rig_with(Options::default())
}

impl Rig {
    /// Runs a command and, when it started the checks of a recording that nothing
    /// holds, waits for their end.
    pub fn run(&mut self, c: Command) {
        self.app.dispatch(c, self.now);
        if !self.world.sound.hold.load(Ordering::SeqCst) {
            self.settle();
        }
    }

    /// Ticks until the checks before a recording are over.
    pub fn settle(&mut self) {
        let end = Instant::now() + Duration::from_secs(10);
        while self.app.phase() == Phase::Starting {
            self.app.tick(self.now);
            assert!(Instant::now() < end, "the checks did not end");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Lets a held start go on and waits for it.
    pub fn release(&mut self) {
        self.world.sound.hold.store(false, Ordering::SeqCst);
        self.settle();
    }

    pub fn after(&mut self, d: Duration) {
        self.now += d;
        self.app.tick(self.now);
    }

    /// Presses Start and checks that it records.
    pub fn record(&mut self) {
        self.run(Command::ToggleRecording);
        assert_eq!(self.app.phase(), Phase::Recording, "{:?}", self.app.message);
    }

    fn picture(&mut self) {
        let end = Instant::now() + Duration::from_secs(5);
        while self.app.frame.is_none() {
            self.app.tick(self.now);
            assert!(Instant::now() < end, "no preview picture");
            std::thread::yield_now();
        }
    }
}

// covers: REQ-343, REQ-344
#[test]
fn the_first_screen_is_chosen_shown_and_another_choice_changes_the_preview() {
    let mut r = rig();
    assert_eq!(r.app.phase(), Phase::Previewing);
    assert_eq!(r.app.chosen, Some(10));
    assert!(r.world.last().borrow().args.contains(&":0+0,0".to_string()));
    r.picture();
    assert_eq!(r.app.frame.as_ref().unwrap().height, 360);
    r.run(Command::SelectScreen(20));
    assert_eq!((r.app.chosen_index(), r.world.count()), (Some(1), 2));
    assert!(r.world.procs.borrow()[0].borrow().killed);
    assert!(
        r.world
            .last()
            .borrow()
            .args
            .contains(&":0+1920,0".to_string())
    );
    r.picture();
    assert_eq!(r.app.frame.as_ref().unwrap().height, 512);
    // the same choice or an unknown one changes nothing
    r.run(Command::SelectScreen(20));
    r.run(Command::SelectScreen(99));
    assert_eq!(r.world.count(), 2);
}

// covers: REQ-344
#[test]
fn the_screen_list_follows_screens_being_connected_and_removed() {
    let mut r = rig();
    r.run(Command::SelectScreen(20));
    // not looked at before its time
    r.world
        .screens
        .borrow_mut()
        .push(screen(30, 3200, 800, 600));
    r.after(Duration::from_millis(100));
    assert_eq!(r.app.screens.len(), 2);
    let spawned = r.world.count();
    r.after(SCAN_EVERY);
    assert_eq!(r.app.screens.len(), 3);
    // the chosen screen is as before: the preview goes on
    assert_eq!((r.app.chosen, r.world.count()), (Some(20), spawned));
    // the chosen one is removed: the first one is shown
    r.world.screens.borrow_mut().remove(1);
    r.after(SCAN_EVERY);
    assert_eq!((r.app.chosen, r.world.count()), (Some(10), spawned + 1));
    // all gone, then back
    r.world.screens.borrow_mut().clear();
    r.after(SCAN_EVERY);
    assert_eq!((r.app.chosen, r.app.phase()), (None, Phase::Idle));
    r.run(Command::ToggleRecording);
    assert_eq!(r.app.message.as_deref(), Some(NO_SCREEN));
    r.world.screens.borrow_mut().push(screen(40, 0, 640, 480));
    r.after(SCAN_EVERY);
    assert_eq!((r.app.chosen, r.app.phase()), (Some(40), Phase::Previewing));
}

// covers: REQ-349, REQ-343, REQ-344, REQ-357
#[test]
fn start_records_the_chosen_screen_with_the_choices_locked_and_stop_finishes_it() {
    let mut r = rig();
    assert_eq!(r.app.button_label(), "Start recording");
    r.run(Command::SelectScreen(20));
    r.run(Command::SetVideo(VideoFormat::Hd1080Fast));
    r.record();
    assert_eq!(r.app.button_label(), "Stop recording");
    // the preview process is gone, the recording one shows the preview
    assert_eq!((r.world.alive(), r.world.sound.live()), (1, 1));
    let p = r.world.last();
    assert!(p.borrow().recording());
    assert!(p.borrow().args.contains(&":0+1920,0".to_string()));
    r.picture();
    let file = r.world.write_file();
    assert_eq!(
        file,
        r.dir.path().join("screen-recording-2026-11-26-101502.mp4"),
        "named with the time of the start"
    );
    // nothing can be changed while recording
    let before = (r.app.chosen, r.app.name.clone(), r.app.folder.clone());
    for c in [
        Command::SelectScreen(10),
        Command::SetName("x".into()),
        Command::PickFile,
        Command::SetVideo(VideoFormat::Uhd),
        Command::SetAudio(AudioFormat::Wav),
    ] {
        r.run(c);
    }
    assert_eq!(
        before,
        (r.app.chosen, r.app.name.clone(), r.app.folder.clone())
    );
    assert_eq!(r.app.video, VideoFormat::Hd1080Fast);
    assert_eq!(r.app.audio, AudioFormat::Aac);
    assert!(r.world.asked.borrow().is_empty());
    r.after(Duration::from_secs(10));
    assert_eq!(r.app.recording_for(r.now), Some(Duration::from_secs(10)));
    r.run(Command::ToggleRecording);
    assert_eq!(r.app.phase(), Phase::Stopping);
    assert!(p.borrow().quit && !p.borrow().killed);
    // pressed again while the file is closed: nothing
    r.run(Command::ToggleRecording);
    r.after(Duration::from_secs(1));
    assert_eq!(r.app.phase(), Phase::Stopping);
    r.world.end(true, "");
    r.after(Duration::from_millis(10));
    assert_eq!(r.app.phase(), Phase::Previewing);
    assert_eq!(r.app.button_label(), "Start recording");
    assert_eq!(r.app.message, None);
    assert_eq!(r.app.status, format!("Saved {}", file.display()));
    assert_eq!(r.world.sound.live(), 0, "the sound is closed");
    assert!(!r.app.quit);
    // a second recording, with a new name, without restarting
    r.record();
    assert_ne!(r.world.write_file(), file);
    // ffmpeg was asked what it can do only once
    assert_eq!(r.runner.calls.load(Ordering::SeqCst), 2);
}

// covers: REQ-350
#[test]
fn a_recording_that_stops_by_itself_keeps_the_file_and_names_the_cause() {
    let mut r = rig();
    r.record();
    let file = r.world.write_file();
    r.world
        .end(false, "av_write_frame: No space left on device\n");
    r.after(Duration::from_millis(10));
    assert_eq!(r.app.button_label(), "Start recording");
    let m = r.app.message.clone().unwrap();
    assert!(m.contains("The disk is full"), "{m}");
    assert!(
        m.contains(&format!("already written is in {}", file.display())),
        "{m}"
    );
    assert!(file.is_file());
    assert_eq!(
        (r.app.phase(), r.world.sound.live()),
        (Phase::Previewing, 0)
    );
    r.run(Command::DismissMessage);
    // ends with success although nobody asked
    r.record();
    r.world.end(true, "");
    r.after(Duration::from_millis(10));
    let m = r.app.message.clone().unwrap();
    assert!(m.contains("ended by itself") && m.contains("Nothing was written"));
    // fails while closing
    r.record();
    r.run(Command::ToggleRecording);
    r.world.end(false, "Conversion failed!");
    r.after(Duration::from_millis(10));
    assert!(
        r.app
            .message
            .clone()
            .unwrap()
            .contains("Conversion failed!")
    );
}

// covers: REQ-350
#[test]
fn a_removed_screen_stops_the_recording_with_that_cause() {
    let mut r = rig();
    r.record();
    r.world.write_file();
    r.world.screens.borrow_mut().remove(0);
    r.after(SCAN_EVERY);
    assert_eq!(r.app.phase(), Phase::Stopping);
    assert!(r.world.last().borrow().quit);
    // the choice stays while the file is closed
    assert_eq!(r.app.chosen, Some(10));
    r.world.end(true, "");
    r.after(Duration::from_millis(10));
    assert!(r.app.message.clone().unwrap().contains(SCREEN_REMOVED));
    assert_eq!((r.app.chosen, r.app.phase()), (Some(20), Phase::Previewing));
}

// covers: REQ-349, REQ-350
#[test]
fn ffmpeg_that_does_not_finish_is_ended_after_the_time_limit() {
    let mut r = rig();
    r.record();
    let stuck = r.world.last();
    r.run(Command::ToggleRecording);
    r.after(STOP_TIMEOUT - Duration::from_millis(1));
    assert!(!stuck.borrow().killed);
    r.after(Duration::from_millis(1));
    assert!(stuck.borrow().killed);
    assert!(r.app.message.clone().unwrap().contains("within 15 seconds"));
    assert_eq!(r.app.phase(), Phase::Previewing);
}

// covers: REQ-346
#[test]
fn an_existing_file_is_replaced_only_after_replace() {
    let mut r = rig();
    let old = r.dir.path().join("talk.mp4");
    std::fs::write(&old, "keep me").unwrap();
    r.run(Command::SetName("talk".into()));
    let previews = r.world.count();
    r.run(Command::ToggleRecording);
    assert_eq!(r.app.replace.as_deref(), Some(old.as_path()));
    assert_eq!(
        (r.app.phase(), r.world.count()),
        (Phase::Previewing, previews)
    );
    r.run(Command::CancelReplace);
    assert_eq!(r.app.replace, None);
    r.run(Command::Replace);
    assert_eq!(
        r.world.count(),
        previews,
        "nothing was asked: nothing starts"
    );
    assert_eq!(std::fs::read_to_string(&old).unwrap(), "keep me");
    r.run(Command::ToggleRecording);
    r.run(Command::Replace);
    assert_eq!(r.app.phase(), Phase::Recording);
    assert!(r.world.last().borrow().args.contains(&"-y".to_string()));
    // a checked start that fails after Replace says why
    r.run(Command::ToggleRecording);
    r.world.end(true, "");
    r.after(Duration::from_millis(10));
    r.world.sound.fail(Some("no sound"));
    r.run(Command::ToggleRecording);
    r.run(Command::Replace);
    assert_eq!(r.app.message.as_deref(), Some("no sound"));
}

// covers: REQ-345, REQ-348
#[test]
fn the_name_box_the_dialog_and_the_sound_format_set_the_file() {
    let mut r = rig();
    assert_eq!(r.app.name, "screen-recording-2026-11-26-101501.mp4");
    assert_eq!(r.app.folder, r.dir.path());
    r.run(Command::SetAudio(AudioFormat::Wav));
    assert_eq!(r.app.name, "screen-recording-2026-11-26-101501.mkv");
    // a cancelled dialog changes nothing
    r.run(Command::PickFile);
    assert_eq!(r.app.folder, r.dir.path());
    assert_eq!(
        r.world.asked.borrow()[0],
        (r.dir.path().to_path_buf(), r.app.name.clone(), "mkv".into())
    );
    // folder and name together
    let sub = r.dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    r.world.pick.replace(Some(sub.join("lesson.mp4")));
    r.run(Command::PickFile);
    assert_eq!(
        (r.app.folder.clone(), r.app.name.as_str()),
        (sub.clone(), "lesson.mkv")
    );
    r.record();
    assert_eq!(r.world.write_file(), sub.join("lesson.mkv"));
    r.run(Command::ToggleRecording);
    r.world.end(true, "");
    r.after(Duration::from_millis(10));
    // a chosen name stays
    assert_eq!(r.app.name, "lesson.mkv");
    // names and places that cannot be recorded to
    for (name, why) in [("", "Type a file name"), ("a/b", "cannot hold a folder")] {
        r.run(Command::SetName(name.into()));
        r.run(Command::ToggleRecording);
        assert!(r.app.message.take().unwrap().contains(why));
    }
    std::fs::create_dir(sub.join("dir.mkv")).unwrap();
    r.run(Command::SetName("dir".into()));
    r.run(Command::ToggleRecording);
    assert!(r.app.message.take().unwrap().contains("is a folder"));
    r.run(Command::SetName("ok".into()));
    r.app.folder = sub.join("gone");
    r.run(Command::ToggleRecording);
    assert!(r.app.message.take().unwrap().contains("does not exist"));
    assert_eq!(r.app.phase(), Phase::Previewing);
}

// covers: REQ-352
#[test]
fn the_folder_and_the_formats_are_remembered_outside_the_recording_folder() {
    let mut r = rig();
    let path = r.dir.path().join("config/screen-spy.json");
    assert!(!path.exists());
    r.run(Command::SetVideo(VideoFormat::Hd720));
    r.run(Command::SetAudio(AudioFormat::Mp3));
    assert_eq!(RecorderSettings::load(&path).folder, None);
    let sub = r.dir.path().join("recordings");
    std::fs::create_dir(&sub).unwrap();
    r.world.pick.replace(Some(sub.join("a")));
    r.run(Command::PickFile);
    assert_eq!(
        RecorderSettings::load(&path),
        RecorderSettings {
            folder: Some(sub.clone()),
            video: VideoFormat::Hd720,
            audio: AudioFormat::Mp3,
        }
    );
    assert_eq!(std::fs::read_dir(&sub).unwrap().count(), 0);
    // settings that cannot be written are reported
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    r.run(Command::SetVideo(VideoFormat::Uhd));
    assert!(r.app.status.contains("could not be saved"));
}

// covers: REQ-351
#[test]
fn closing_during_a_recording_asks_and_finishes_the_file_first() {
    let mut r = rig();
    r.record();
    let p = r.world.last();
    r.run(Command::Close);
    assert!(r.app.ask_close && !r.app.quit);
    r.run(Command::CancelClose);
    assert!(!r.app.ask_close);
    assert_eq!(r.app.phase(), Phase::Recording);
    assert!(!p.borrow().quit);
    r.run(Command::Close);
    r.run(Command::StopAndClose);
    assert!(p.borrow().quit && !r.app.quit, "the file is not finished");
    r.run(Command::Close);
    r.run(Command::StopAndClose);
    r.world.end(true, "");
    r.after(Duration::from_millis(10));
    assert!(r.app.quit);
    // a file that cannot be finished keeps the program open with the message
    let mut r = rig();
    r.record();
    r.run(Command::Close);
    r.run(Command::StopAndClose);
    r.world.end(false, "boom");
    r.after(Duration::from_millis(10));
    assert!(!r.app.quit && r.app.message.is_some() && !r.app.ask_close);
    // the recording ended while the question was open: closing goes on
    r.run(Command::StopAndClose);
    assert!(r.app.quit);
    // without a recording the window closes at once
    let mut r = rig();
    r.run(Command::Close);
    assert!(r.app.quit);
    assert_eq!(r.world.alive(), 0);
}

// covers: REQ-358
#[test]
fn on_wayland_start_shows_the_message_and_creates_nothing() {
    let mut r = rig_with(Options {
        env: vec![("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0")],
        ..Options::default()
    });
    assert_eq!((r.app.phase(), r.world.count()), (Phase::Idle, 0));
    assert!(r.app.platform.blocked.is_some(), "shown before Start");
    r.run(Command::ToggleRecording);
    assert!(
        r.app
            .message
            .clone()
            .unwrap()
            .contains("needs an X11 session")
    );
    assert_eq!(r.app.button_label(), "Start recording");
    assert_eq!(r.world.count(), 0);
    assert_eq!(std::fs::read_dir(r.dir.path()).unwrap().count(), 0);
}

// covers: REQ-357, REQ-341
#[test]
fn on_an_older_mac_the_screen_is_shown_and_start_names_the_version_needed() {
    let mut r = rig_with(Options {
        os: Os::MacOs,
        ..Options::default()
    });
    // no version known counts as too old
    assert_eq!(r.app.phase(), Phase::Previewing);
    r.run(Command::ToggleRecording);
    assert!(r.app.message.clone().unwrap().contains("macOS 14.6"));
    assert_eq!(
        (r.app.phase(), r.world.sound.live()),
        (Phase::Previewing, 0)
    );
}

// covers: REQ-357, REQ-350
#[test]
fn start_refuses_and_names_what_is_missing() {
    // a source
    let mut r = rig();
    r.world.sound.fail(Some(
        "The microphone is not available. Nothing was recorded.",
    ));
    r.run(Command::ToggleRecording);
    assert!(r.app.message.take().unwrap().contains("microphone"));
    assert_eq!((r.app.phase(), r.world.alive()), (Phase::Previewing, 1));
    assert!(!r.world.last().borrow().recording());
    // the screen permission
    r.world.sound.fail(None);
    r.world.denied.set(true);
    r.run(Command::ToggleRecording);
    assert!(
        r.app
            .message
            .take()
            .unwrap()
            .contains("Screen recording is not allowed")
    );
    r.world.denied.set(false);
    // the screen went away since the last look
    r.run(Command::SelectScreen(20));
    r.world.screens.borrow_mut().remove(1);
    r.run(Command::ToggleRecording);
    assert!(r.app.message.take().unwrap().contains("was removed"));
    // an encoder
    r.run(Command::SetAudio(AudioFormat::Mp3));
    r.run(Command::ToggleRecording);
    assert!(
        r.app
            .message
            .take()
            .unwrap()
            .contains("the encoder libmp3lame")
    );
    // ffmpeg cannot be started for the recording
    r.run(Command::SetAudio(AudioFormat::Aac));
    r.world.spawn_fails.set(true);
    r.run(Command::ToggleRecording);
    assert!(
        r.app
            .message
            .take()
            .unwrap()
            .contains("could not be started")
    );
    assert!(
        r.app
            .preview_error
            .clone()
            .unwrap()
            .contains("not allowed to run")
    );
    assert_eq!(r.world.sound.live(), 0);
    // no ffmpeg at all
    let mut r = rig_with(Options {
        ffmpeg: false,
        ..Options::default()
    });
    assert_eq!(r.app.message.take().as_deref(), Some(NO_FFMPEG));
    r.run(Command::ToggleRecording);
    assert_eq!(r.app.message.as_deref(), Some(NO_FFMPEG));
    assert_eq!(r.world.count(), 0);
    // an ffmpeg that answers with an error, or not at all
    for runner in [
        Scripted::new(|_| fail("broken build")),
        Scripted::new(|_| None),
    ] {
        let mut r = rig_with(Options {
            runner,
            ..Options::default()
        });
        r.run(Command::ToggleRecording);
        assert!(r.app.message.clone().unwrap().contains("ffmpeg"));
        assert_eq!(r.app.phase(), Phase::Previewing);
    }
}

// covers: REQ-343
#[test]
fn a_preview_that_ends_says_why_and_is_not_restarted_in_a_loop() {
    let mut r = rig();
    r.world.end(false, "[x11grab] Cannot get the image data");
    r.after(Duration::from_millis(10));
    assert!(
        r.app
            .preview_error
            .clone()
            .unwrap()
            .contains("could not be captured")
    );
    assert_eq!(r.app.phase(), Phase::Idle);
    r.after(SCAN_EVERY);
    assert_eq!(r.world.count(), 1);
    r.run(Command::SelectScreen(20));
    assert_eq!((r.app.phase(), r.world.count()), (Phase::Previewing, 2));
    assert_eq!(r.app.preview_error, None);
}

// covers: REQ-342
#[test]
fn about_is_opened_and_closed() {
    let mut r = rig();
    r.run(Command::ShowAbout);
    assert!(r.app.show_about);
    r.run(Command::CloseAbout);
    assert!(!r.app.show_about);
}

// covers: REQ-349, REQ-357
#[test]
fn the_checks_before_a_start_run_off_the_windows_thread_with_the_choices_locked() {
    let mut r = rig();
    r.world.sound.hold.store(true, Ordering::SeqCst);
    // the command returns while the sound is still being opened
    r.run(Command::ToggleRecording);
    assert_eq!(r.app.phase(), Phase::Starting);
    assert!(r.app.locked() && !r.app.recording() && !r.app.cancelling());
    assert_eq!(r.app.button_label(), "Cancel start");
    assert!(r.app.status.starts_with("Checking"));
    // no capture runs, and the window goes on
    assert_eq!(r.world.alive(), 0);
    r.after(Duration::from_millis(100));
    r.run(Command::SetVideo(VideoFormat::Uhd));
    r.run(Command::SelectScreen(20));
    assert_eq!((r.app.video, r.app.chosen), (VideoFormat::Hd1080, Some(10)));
    r.release();
    assert_eq!(r.app.phase(), Phase::Recording, "{:?}", r.app.message);
    let thread = r.world.sound.thread.lock().unwrap().unwrap();
    assert_ne!(thread, std::thread::current().id());
    assert_eq!((r.world.sound.live(), r.world.alive()), (1, 1));
}

// covers: REQ-349, REQ-351
#[test]
fn a_start_being_checked_can_be_cancelled_or_closed_and_records_nothing() {
    for deaf in [false, true] {
        let mut r = rig();
        r.world.sound.hold.store(true, Ordering::SeqCst);
        r.world.sound.deaf.store(deaf, Ordering::SeqCst);
        r.run(Command::ToggleRecording);
        r.run(Command::ToggleRecording);
        assert!(r.app.cancelling());
        // pressed again: still cancelling
        r.run(Command::ToggleRecording);
        if deaf {
            r.after(Duration::from_millis(5));
            assert_eq!(r.app.phase(), Phase::Starting, "waits for the checks");
            r.release();
        } else {
            r.settle();
        }
        assert_eq!(r.app.phase(), Phase::Previewing);
        assert_eq!((r.app.status.as_str(), &r.app.message), (CANCELLED, &None));
        // a sound opened all the same is closed again
        assert_eq!(r.world.sound.live(), 0);
        assert!(
            r.world
                .procs
                .borrow()
                .iter()
                .all(|p| !p.borrow().recording())
        );
    }
    // closing the window asks nothing: nothing is being recorded
    let mut r = rig();
    r.world.sound.hold.store(true, Ordering::SeqCst);
    r.run(Command::ToggleRecording);
    r.run(Command::Close);
    assert!(r.app.quit && !r.app.ask_close);
}

// covers: REQ-350, REQ-344
#[test]
fn a_screen_removed_or_a_failure_during_the_checks_is_told() {
    let mut r = rig();
    r.world.sound.hold.store(true, Ordering::SeqCst);
    r.run(Command::ToggleRecording);
    r.world.screens.borrow_mut().remove(0);
    r.after(SCAN_EVERY);
    assert_eq!(r.app.chosen, Some(10), "locked");
    r.release();
    assert_eq!(r.app.message.take().as_deref(), Some(CHOSEN_REMOVED));
    assert_eq!((r.app.phase(), r.app.chosen), (Phase::Previewing, Some(20)));
    assert_eq!(r.world.sound.live(), 0);
    r.world.sound.panics.store(true, Ordering::SeqCst);
    r.run(Command::ToggleRecording);
    assert!(
        r.app
            .message
            .clone()
            .unwrap()
            .contains("checks before the recording failed")
    );
    assert_eq!(r.app.phase(), Phase::Previewing);
}
