// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Unit tests of videolore player with a scripted ffprobe/ffmpeg (no real programs).
//! The real-ffmpeg tests are in `tests/ffmpeg.rs`.

use crate::app::{
    Command, Dialogs, Folders, NO_FFMPEG, PROJECT_REFUSED, PlayerState, is_project_file,
};
use crate::fullscreen::HINT_TIME;
use crate::subs::Source;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use videolore::base::pos::Pos;
use videolore::camera::Camera;
use videolore::ffmpeg::{BinSource, CmdOutput, FfVersion, FfmpegInfo, Process, Runner};
use videolore::machine::Machine;
use videolore::preview::NullSink;
use videolore::time::Micros;

const VIDEO: &str = r#"{"streams":[
  {"codec_type":"video","codec_name":"h264","width":1280,"height":720,
   "avg_frame_rate":"30/1","r_frame_rate":"30/1","bit_rate":"2000000"},
  {"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}],
  "format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"5.0"}}"#;

const SPHERE: &str = r#"{"streams":[
  {"codec_type":"video","codec_name":"h264","width":2048,"height":1024,
   "avg_frame_rate":"30/1","r_frame_rate":"30/1","bit_rate":"2000000",
   "side_data_list":[{"side_data_type":"Spherical Mapping","projection":"equirectangular"}]}],
  "format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"5.0"}}"#;

const TRACKS: &str = r#"{"streams":[
  {"codec_type":"video","codec_name":"h264","width":1280,"height":720,
   "avg_frame_rate":"30/1","r_frame_rate":"30/1","bit_rate":"2000000"},
  {"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"eng","title":"Main"}},
  {"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","tags":{"language":"fra"}},
  {"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"ita"}}],
  "format":{"format_name":"matroska,webm","duration":"5.0"}}"#;

const AUDIO: &str = r#"{"streams":[{"codec_type":"audio","codec_name":"mp3"}],
  "format":{"format_name":"mp3","duration":"30.0"}}"#;

pub(crate) const SRT: &str = "1\n00:00:01,000 --> 00:00:02,500\nHello there\n\n2\n00:00:03,000 --> 00:00:04,000\nSecond line\n";

/// A process that never produces a picture and ends when it is killed: a player that is
/// "playing" without frames.
struct Hold(Arc<AtomicBool>);

struct HoldRead(Arc<AtomicBool>);

impl io::Read for HoldRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        while !self.0.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(0)
    }
}

impl Process for Hold {
    fn take_stdout(&mut self) -> Option<Box<dyn io::Read + Send>> {
        Some(Box::new(HoldRead(Arc::clone(&self.0))))
    }
    fn kill(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn wait(&mut self) -> io::Result<CmdOutput> {
        Ok(CmdOutput {
            success: true,
            ..Default::default()
        })
    }
}

/// ffprobe answers by file name; ffmpeg writes a subtitle track when asked for one.
pub(crate) struct Fake;

fn out(stdout: &str) -> io::Result<CmdOutput> {
    Ok(CmdOutput {
        success: true,
        stdout: stdout.as_bytes().to_vec(),
        stderr: String::new(),
    })
}

impl Runner for Fake {
    fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
        let a: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let last = a.last().cloned().unwrap_or_default();
        if program.ends_with("ffprobe") {
            return match () {
                _ if last.ends_with("sphere.mp4") => out(SPHERE),
                _ if last.ends_with("tracks.mkv") => out(TRACKS),
                _ if last.ends_with(".mp3") => out(AUDIO),
                _ if last.ends_with(".mp4") => out(VIDEO),
                _ => Ok(CmdOutput {
                    success: false,
                    stdout: Vec::new(),
                    stderr: "Invalid data found".into(),
                }),
            };
        }
        if a.iter().any(|x| x == "0:s:2") {
            // the third subtitle stream of the file (the second text track)
            std::fs::write(last.strip_prefix("file:").unwrap_or(&last), SRT)?;
            return out("");
        }
        if a.iter().any(|x| x.starts_with("0:s:")) {
            return Ok(CmdOutput {
                success: false,
                stdout: Vec::new(),
                stderr: "no such stream".into(),
            });
        }
        out("")
    }

    fn run_within(&self, p: &Path, a: &[OsString], _: Duration) -> io::Result<CmdOutput> {
        self.run(p, a)
    }

    fn spawn_in(&self, _: &Path, _: &[OsString], _: Option<&Path>) -> io::Result<Box<dyn Process>> {
        Ok(Box::new(Hold(Arc::new(AtomicBool::new(false)))))
    }
}

pub(crate) struct FakeDialogs {
    pub video: Option<PathBuf>,
    pub srt: Option<PathBuf>,
}

impl Dialogs for FakeDialogs {
    fn pick_video(&mut self) -> Option<PathBuf> {
        self.video.clone()
    }
    fn pick_srt(&mut self) -> Option<PathBuf> {
        self.srt.clone()
    }
}

pub(crate) fn info() -> FfmpegInfo {
    FfmpegInfo {
        ffmpeg: "/usr/bin/ffmpeg".into(),
        ffprobe: "/usr/bin/ffprobe".into(),
        version: FfVersion::Release(6, 1, 1),
        filters: ["subtitles"].iter().map(|s| s.to_string()).collect(),
        encoders: ["libx264", "aac"].iter().map(|s| s.to_string()).collect(),
        source: BinSource::Path,
    }
}

fn player_with(
    ffmpeg: Result<FfmpegInfo, String>,
    dialogs: FakeDialogs,
    work: &Path,
) -> PlayerState {
    PlayerState::new(
        ffmpeg,
        Arc::new(Fake),
        Box::new(dialogs),
        Box::new(NullSink),
        Machine::detect(),
        Folders {
            cache_dir: None,
            work_root: work.to_path_buf(),
        },
        Instant::now(),
    )
}

pub(crate) fn player(work: &Path) -> PlayerState {
    player_with(
        Ok(info()),
        FakeDialogs {
            video: None,
            srt: None,
        },
        work,
    )
}

fn now() -> Instant {
    Instant::now()
}

/// Ticks until the background work of the player is over.
fn settle(p: &mut PlayerState) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        p.tick(Instant::now());
        if !p.opening() && p.idle_for_tests() {
            return;
        }
        assert!(Instant::now() < end, "the player did not settle");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn file(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, name).unwrap();
    p
}

fn open(p: &mut PlayerState, path: PathBuf) {
    p.dispatch(Command::Open(path), now());
    settle(p);
}

// covers: REQ-138
#[test]
fn a_video_opens_as_a_one_clip_project_and_names_the_window() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    assert_eq!(p.title(), "videolore player");
    assert!(!p.has_video());
    open(&mut p, file(d.path(), "holiday.mp4"));
    assert!(p.has_video() && p.notice.is_none());
    assert_eq!(p.project.timeline.clips.len(), 1);
    assert_eq!(p.duration(), Micros(5_000_000));
    assert_eq!(p.title(), "holiday.mp4 — videolore player");
    assert!(!p.is_360());
    assert_eq!(p.output_size(), Some((1280, 720)));
    // another video replaces it: still one clip, a new title
    open(&mut p, file(d.path(), "other.mp4"));
    assert_eq!(p.project.timeline.clips.len(), 1);
    assert_eq!(p.title(), "other.mp4 — videolore player");
    // the dialog's answer opens too; a cancelled dialog does nothing
    let mut q = player_with(
        Ok(info()),
        FakeDialogs {
            video: Some(file(d.path(), "dialog.mp4")),
            srt: None,
        },
        d.path(),
    );
    q.dispatch(Command::PickOpen, now());
    settle(&mut q);
    assert_eq!(q.title(), "dialog.mp4 — videolore player");
    let mut r = player(d.path());
    r.dispatch(Command::PickOpen, now());
    assert!(!r.opening() && !r.has_video());
}

// covers: REQ-138
#[test]
fn a_project_file_is_refused_by_its_name_without_being_read() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    for name in ["a.vlproj", "B.VLPROJ", "c.VlProj"] {
        // a file nobody can read, and one that does not exist: neither is touched
        let path = d.path().join(name);
        std::fs::write(&path, b"{").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        p.dispatch(Command::Open(path.clone()), now());
        assert_eq!(p.notice.as_deref(), Some(PROJECT_REFUSED), "{name}");
        assert!(!p.opening() && !p.has_video(), "no probe was started");
        p.dispatch(Command::DismissNotice, now());
        assert!(p.notice.is_none());
        assert!(is_project_file(&path));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"{", "left untouched");
    }
    p.dispatch(Command::Open("/nowhere/x.vlproj".into()), now());
    assert_eq!(p.notice.as_deref(), Some(PROJECT_REFUSED));
    assert!(!is_project_file(Path::new("movie.mp4")));
    assert!(!is_project_file(Path::new("vlproj")));
}

// covers: REQ-138
#[test]
fn files_that_cannot_be_played_are_refused_with_a_message() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    open(&mut p, file(d.path(), "song.mp3"));
    assert!(p.notice.as_deref().unwrap().contains("no picture"));
    assert!(!p.has_video());
    p.dispatch(Command::DismissNotice, now());
    open(&mut p, file(d.path(), "broken.bin"));
    assert!(p.notice.is_some() && !p.has_video());
    open(&mut p, d.path().join("missing.mp4"));
    assert!(p.notice.is_some());
    // without ffmpeg nothing opens, and the message says why
    let mut q = player_with(
        Err("not found".into()),
        FakeDialogs {
            video: None,
            srt: None,
        },
        d.path(),
    );
    assert_eq!(q.notice.as_deref(), Some(NO_FFMPEG));
    q.dispatch(Command::DismissNotice, now());
    q.dispatch(Command::Open(file(d.path(), "a.mp4")), now());
    assert_eq!(q.notice.as_deref(), Some(NO_FFMPEG));
    assert!(!q.opening());
}

// covers: REQ-138
#[test]
fn the_player_has_no_save_export_or_edit_command() {
    // An exhaustive match with no wildcard: adding a command that edits, saves or exports
    // forces this test to be changed, which a reviewer sees.
    fn viewing_only(c: &Command) -> bool {
        match c {
            Command::PickOpen
            | Command::Open(_)
            | Command::PlayPause
            | Command::Seek(_)
            | Command::Step(_)
            | Command::Skip(_)
            | Command::SetVolume(_)
            | Command::ToggleMute
            | Command::ChooseSubtitles(_)
            | Command::PickSrt
            | Command::OpenSrt(_)
            | Command::DragLines(_)
            | Command::PlaceLines { .. }
            | Command::DragCamera { .. }
            | Command::ZoomCamera(_)
            | Command::ToggleFullScreen
            | Command::Escape
            | Command::CloseSplash
            | Command::DismissFullScreenHint
            | Command::ShowAbout
            | Command::CloseAbout
            | Command::DismissNotice
            | Command::AddToPlaylist
            | Command::AddToPlaylistPath(_)
            | Command::OpenFolder
            | Command::OpenFolderPath(_)
            | Command::Next
            | Command::Previous
            | Command::OpenInEditor
            | Command::Quit => true,
        }
    }
    assert!(viewing_only(&Command::PlayPause));
}

// covers: REQ-138
#[test]
fn the_player_never_writes_next_to_the_video() {
    let d = tempfile::tempdir().unwrap();
    let videos = d.path().join("videos");
    std::fs::create_dir(&videos).unwrap();
    let work = d.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let movie = file(&videos, "tracks.mkv");
    let before = listing(&videos);
    let mut p = player(&work);
    open(&mut p, movie);
    p.dispatch(Command::ChooseSubtitles(Some(1)), now());
    settle(&mut p);
    p.dispatch(
        Command::PlaceLines {
            ids: vec![],
            pos: Pos { x: 1, y: 1 },
        },
        now(),
    );
    p.dispatch(Command::Seek(Micros(1_000_000)), now());
    p.dispatch(Command::PlayPause, now());
    p.dispatch(Command::SetVolume(40), now());
    assert_eq!(listing(&videos), before, "the video's folder is unchanged");
}

fn listing(dir: &Path) -> Vec<(String, u64)> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                e.metadata().unwrap().len(),
            )
        })
        .collect();
    v.sort();
    v
}

// covers: REQ-138
#[test]
fn transport_commands_move_the_playhead_and_set_the_sound() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    // nothing open: transport does nothing and never fails
    p.dispatch(Command::PlayPause, now());
    p.dispatch(Command::Seek(Micros(1)), now());
    assert_eq!(p.playhead, Micros::ZERO);
    open(&mut p, file(d.path(), "a.mp4"));
    p.dispatch(Command::Seek(Micros(2_000_000)), now());
    assert_eq!(p.playhead, Micros(2_000_000));
    p.dispatch(Command::Seek(Micros(99_000_000)), now());
    assert_eq!(p.playhead, Micros(5_000_000), "clamped to the end");
    p.dispatch(Command::Seek(Micros(-5)), now());
    assert_eq!(p.playhead, Micros::ZERO);
    p.dispatch(Command::Step(3), now());
    assert_eq!(p.playhead, Micros(99_999), "three frames at 30 fps");
    p.dispatch(Command::Step(-1), now());
    assert_eq!(p.playhead, Micros(66_666));
    p.dispatch(Command::Skip(Micros(5_000_000)), now());
    assert_eq!(p.playhead, Micros(5_000_000));
    p.dispatch(Command::Skip(Micros(-5_000_000)), now());
    assert_eq!(p.playhead, Micros::ZERO);
    // volume and mute
    assert_eq!((p.volume(), p.muted()), (100, false));
    p.dispatch(Command::SetVolume(40), now());
    assert_eq!(p.volume(), 40);
    p.dispatch(Command::SetVolume(250), now());
    assert_eq!(p.volume(), 100);
    p.dispatch(Command::ToggleMute, now());
    assert!(p.muted());
    p.dispatch(Command::ToggleMute, now());
    assert!(!p.muted());
    // Play starts the engine (its processes are not real here: the error comes back through
    // the preview, never as a crash); Play again pauses
    p.dispatch(Command::PlayPause, now());
    assert!(p.is_playing() && p.busy());
    p.dispatch(Command::SetVolume(30), now());
    assert!(
        p.is_playing(),
        "a volume change restarts the sound, playback goes on"
    );
    p.dispatch(Command::PlayPause, now());
    assert!(!p.is_playing());
}

// covers: REQ-140
#[test]
fn an_srt_file_is_opened_chosen_and_listed() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    let srt = d.path().join("film.srt");
    std::fs::write(&srt, SRT).unwrap();
    // no video yet
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    assert!(p.status.contains("Open a video first"));
    open(&mut p, file(d.path(), "a.mp4"));
    assert!(p.subs.sources.is_empty(), "this file has no embedded track");
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    assert_eq!(p.subs.sources.len(), 1);
    assert_eq!(p.subs.selected, Some(0));
    assert_eq!(p.subs.sources[0].name(), "film.srt");
    assert_eq!(p.project.subtitles.lines.len(), 2);
    let l = &p.project.subtitles.lines[0];
    assert_eq!(
        (l.start, l.end, l.text.as_str()),
        (Micros(1_000_000), Micros(2_500_000), "Hello there")
    );
    // the same file again is not listed twice; None removes the lines
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    assert_eq!(p.subs.sources.len(), 1);
    p.dispatch(Command::ChooseSubtitles(None), now());
    assert_eq!(p.subs.selected, None);
    assert!(p.project.subtitles.lines.is_empty());
    p.dispatch(Command::ChooseSubtitles(Some(0)), now());
    assert_eq!(p.project.subtitles.lines.len(), 2);
    // a list that does not exist and a bad file give a message and change nothing
    p.dispatch(Command::ChooseSubtitles(Some(9)), now());
    assert!(p.status.contains("no longer exists"));
    let bad = d.path().join("bad.srt");
    std::fs::write(&bad, "nothing").unwrap();
    p.dispatch(Command::OpenSrt(bad), now());
    assert!(p.status.contains("no subtitles"));
    assert_eq!(p.subs.sources.len(), 1);
    // the Subtitles menu's dialog
    let mut q = player_with(
        Ok(info()),
        FakeDialogs {
            video: None,
            srt: Some(srt),
        },
        d.path(),
    );
    open(&mut q, file(d.path(), "b.mp4"));
    q.dispatch(Command::PickSrt, now());
    assert_eq!(q.project.subtitles.lines.len(), 2);
}

// covers: REQ-140, REQ-253
#[test]
fn embedded_text_tracks_are_listed_by_name_and_read_when_chosen() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    open(&mut p, file(d.path(), "tracks.mkv"));
    // the text tracks, then the picture track, marked in the menu (REQ-253)
    let names: Vec<String> = p.subs.sources.iter().map(Source::name).collect();
    assert_eq!(names, ["English — Main", "Italian", "French"]);
    assert_eq!(p.subs.sources[2].menu_name(), "French (picture)");
    assert_eq!(p.subs.sources[0].menu_name(), "English — Main");
    // the first is not readable by the fake (error shown, nothing chosen)
    p.dispatch(Command::ChooseSubtitles(Some(0)), now());
    settle(&mut p);
    assert_eq!(p.subs.selected, None);
    assert!(!p.status.is_empty());
    // the second one is read and its lines appear at their times
    p.dispatch(Command::ChooseSubtitles(Some(1)), now());
    assert!(p.opening() || p.busy());
    settle(&mut p);
    assert_eq!(p.subs.selected, Some(1));
    let shown = p
        .project
        .subtitles
        .shown_at(&p.project.timeline, Micros(1_500_000));
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].text, "Hello there");
    assert!(
        p.project
            .subtitles
            .shown_at(&p.project.timeline, Micros(2_700_000))
            .is_empty()
    );
    // chosen again later: no second extraction (the cues are kept)
    p.dispatch(Command::ChooseSubtitles(None), now());
    p.dispatch(Command::ChooseSubtitles(Some(1)), now());
    assert!(!p.busy() || p.subs.cues(1).is_some());
    assert_eq!(p.project.subtitles.lines.len(), 2);
}

// covers: REQ-140
#[test]
fn a_dragged_line_keeps_its_place_for_the_session_only() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    let srt = d.path().join("film.srt");
    std::fs::write(&srt, SRT).unwrap();
    open(&mut p, file(d.path(), "a.mp4"));
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    let id = p.project.subtitles.lines[1].id;
    assert_eq!(p.project.subtitles.lines[1].position, None);
    // dragging shows the line at the new place at once, a drop keeps it
    let pos = Pos { x: 2_000, y: 3_000 };
    p.dispatch(Command::DragLines(Some((vec![id], pos))), now());
    assert_eq!(p.shown_project().subtitles.lines[1].position, Some(pos));
    assert_eq!(p.project.subtitles.lines[1].position, None);
    p.dispatch(Command::DragLines(None), now());
    assert_eq!(p.shown_project().subtitles.lines[1].position, None);
    p.dispatch(Command::PlaceLines { ids: vec![id], pos }, now());
    assert_eq!(p.project.subtitles.lines[1].position, Some(pos));
    // None and back: the place is remembered while the player is open
    p.dispatch(Command::ChooseSubtitles(None), now());
    p.dispatch(Command::ChooseSubtitles(Some(0)), now());
    assert_eq!(p.project.subtitles.lines[1].position, Some(pos));
    assert_eq!(p.project.subtitles.lines[0].position, None);
    // an unknown line is refused with a message
    p.dispatch(
        Command::PlaceLines {
            ids: vec![videolore::subtitles::LineId(999)],
            pos,
        },
        now(),
    );
    assert!(!p.status.is_empty());
    // another video and back: the place is kept for the session (REQ-251); a new run
    // starts with the default place
    open(&mut p, file(d.path(), "a.mp4"));
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    assert_eq!(p.project.subtitles.lines[1].position, Some(pos));
    let mut again = player(d.path());
    open(&mut again, file(d.path(), "a.mp4"));
    again.dispatch(Command::OpenSrt(srt), now());
    assert!(
        again
            .project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none())
    );
}

// covers: REQ-251
#[test]
fn a_dragged_line_keeps_its_place_when_another_video_is_played_and_the_first_again() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    let srt = d.path().join("film.srt");
    std::fs::write(&srt, SRT).unwrap();
    let (a, b) = (file(d.path(), "a.mp4"), file(d.path(), "b.mp4"));
    open(&mut p, a.clone());
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    let id = p.project.subtitles.lines[0].id;
    let pos = Pos { x: 1_000, y: 9_000 };
    p.dispatch(Command::PlaceLines { ids: vec![id], pos }, now());
    open(&mut p, b);
    p.dispatch(Command::OpenSrt(srt.clone()), now());
    assert_eq!(
        p.project.subtitles.lines[0].position, None,
        "the other video has its own"
    );
    open(&mut p, a);
    p.dispatch(Command::OpenSrt(srt), now());
    assert_eq!(p.project.subtitles.lines[0].position, Some(pos));
}

// covers: REQ-252
#[test]
fn volume_sound_folder_and_full_screen_are_remembered_in_the_players_own_file() {
    use crate::settings::PlayerSettings;
    let d = tempfile::tempdir().unwrap();
    let videos = d.path().join("videos");
    std::fs::create_dir(&videos).unwrap();
    let cfg = d.path().join("config/player.json");
    let before = listing(&videos);
    let make =
        || player(d.path()).with_settings(PlayerSettings::load(&cfg), Some(cfg.clone()), now());
    let mut p = make();
    open(&mut p, file(&videos, "a.mp4"));
    let before = {
        let mut b = before;
        b.push(("a.mp4".into(), 5));
        b.sort();
        b
    };
    p.dispatch(Command::SetVolume(35), now());
    p.dispatch(Command::ToggleMute, now());
    p.dispatch(Command::ToggleFullScreen, now());
    // restart: the same volume, sound off, full screen, and the dialogs start in the folder
    let mut q = make();
    assert!(q.fullscreen.on);
    assert_eq!(q.settings.last_folder.as_deref(), Some(videos.as_path()));
    open(&mut q, file(&videos, "a.mp4"));
    assert_eq!((q.volume(), q.muted()), (35, true));
    // nothing was written in the folder of the videos: the file is in the configuration folder
    assert_eq!(listing(&videos), before);
    assert!(cfg.is_file());
    // a damaged file gives the defaults
    std::fs::write(&cfg, "[1,").unwrap();
    let r = make();
    assert_eq!(
        (r.settings.volume, r.settings.muted, r.fullscreen.on),
        (100, false, false)
    );
}

// covers: REQ-254
#[test]
fn a_playlist_plays_in_order_goes_next_and_back_and_stops_after_the_last() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    let names = ["a.mp4", "b.mp4", "c.mp4"];
    for n in names {
        p.dispatch(Command::AddToPlaylistPath(file(d.path(), n)), now());
        settle(&mut p);
    }
    // the first one added started playing; the list is in the order added
    assert_eq!(p.title(), "a.mp4 — videolore player");
    assert_eq!(p.playlist.items.len(), 3);
    p.dispatch(Command::Next, now());
    settle(&mut p);
    assert_eq!(p.title(), "b.mp4 — videolore player");
    p.dispatch(Command::Previous, now());
    settle(&mut p);
    assert_eq!(p.title(), "a.mp4 — videolore player");
    p.dispatch(Command::Previous, now());
    assert!(p.status.contains("first video"));
    // the end of a video starts the next; after the last it stops
    assert!(p.after_end(now()));
    settle(&mut p);
    assert_eq!(p.title(), "b.mp4 — videolore player");
    assert!(p.after_end(now()));
    settle(&mut p);
    assert_eq!(p.title(), "c.mp4 — videolore player");
    assert!(!p.after_end(now()));
    p.dispatch(Command::Next, now());
    assert!(p.status.contains("last video"));
    assert_eq!(p.title(), "c.mp4 — videolore player");
    // a folder is a playlist in name order; the list is not kept by a new run
    let folder = d.path().join("shows");
    std::fs::create_dir(&folder).unwrap();
    for n in ["z.mp4", "m.mp4", "notes.txt"] {
        file(&folder, n);
    }
    p.dispatch(Command::OpenFolderPath(folder.clone()), now());
    settle(&mut p);
    assert_eq!(p.title(), "m.mp4 — videolore player");
    assert_eq!(p.playlist.items.len(), 2);
    p.dispatch(Command::OpenFolderPath(d.path().join("none")), now());
    assert!(p.status.contains("no videos"));
    assert!(player(d.path()).playlist.items.is_empty());
}

// covers: REQ-250
#[test]
fn open_in_editor_needs_a_video_and_a_missing_editor_is_explained() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    p.dispatch(Command::OpenInEditor, now());
    assert!(p.status.contains("Open a video"));
    let movie = file(d.path(), "a.mp4");
    open(&mut p, movie.clone());
    let before = listing(d.path());
    p.dispatch(Command::OpenInEditor, now());
    assert!(
        p.status.contains("not installed next to this program"),
        "{}",
        p.status
    );
    assert_eq!(
        listing(d.path()),
        before,
        "nothing is written next to the video"
    );
}

// covers: REQ-139
#[test]
fn only_a_360_video_turns_and_the_view_stops_at_the_limits() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    open(&mut p, file(d.path(), "a.mp4"));
    // a flat video has no drag turning
    p.dispatch(
        Command::DragCamera {
            dx: 50.0,
            dy: 20.0,
            view_w: 800.0,
        },
        now(),
    );
    assert_eq!(p.camera(), Camera::FRONT);
    assert!(p.status.contains("360"));
    p.dispatch(Command::ZoomCamera(10.0), now());
    assert_eq!(p.camera(), Camera::FRONT);
    // a 360 video: the same drag gives the editor's own result
    open(&mut p, file(d.path(), "sphere.mp4"));
    assert!(p.is_360());
    p.dispatch(
        Command::DragCamera {
            dx: 100.0,
            dy: -40.0,
            view_w: 800.0,
        },
        now(),
    );
    assert_eq!(p.camera(), Camera::FRONT.dragged(100.0, -40.0, 800.0));
    assert!(
        p.camera().yaw_cd < 0,
        "dragging right turns the picture with the pointer"
    );
    // the shown project carries the camera for the preview
    let shown = p.shown_project();
    let m = shown.media[0].id;
    assert_eq!(shown.camera.path(m)[0].cam, p.camera());
    drop(shown);
    // limits: pitch stops at 90 degrees, zoom inside its range
    for _ in 0..40 {
        p.dispatch(
            Command::DragCamera {
                dx: 0.0,
                dy: 500.0,
                view_w: 800.0,
            },
            now(),
        );
    }
    assert_eq!(p.camera().pitch_cd, 9000);
    for _ in 0..200 {
        p.dispatch(Command::ZoomCamera(100.0), now());
    }
    assert!(p.camera().is_in_range());
    assert_eq!(p.camera().fov_cd, 3000);
    // a new video starts at the front again
    open(&mut p, file(d.path(), "sphere.mp4"));
    assert_eq!(p.camera(), Camera::FRONT);
}

// covers: REQ-141
#[test]
fn full_screen_commands_follow_the_rules_of_the_state_machine() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    let t0 = now();
    p.dispatch(Command::ToggleFullScreen, t0);
    assert!(p.fullscreen.on && p.fullscreen.hint_visible());
    // About is on top of the hint: Esc closes it first, then the hint, then full screen
    p.dispatch(Command::ShowAbout, t0);
    p.dispatch(Command::Escape, t0);
    assert!(!p.show_about && p.fullscreen.hint_visible());
    p.dispatch(Command::Escape, t0);
    assert!(p.fullscreen.on && !p.fullscreen.hint_visible());
    p.dispatch(Command::Escape, t0);
    assert!(!p.fullscreen.on);
    // OK closes the hint; five seconds close it by themselves
    p.dispatch(Command::ToggleFullScreen, t0);
    p.dispatch(Command::DismissFullScreenHint, t0);
    assert!(!p.fullscreen.hint_visible());
    p.dispatch(Command::ToggleFullScreen, t0);
    p.dispatch(Command::ToggleFullScreen, t0);
    assert!(p.fullscreen.hint_visible(), "shown again at the next entry");
    p.tick(t0 + HINT_TIME);
    assert!(!p.fullscreen.hint_visible() && p.fullscreen.on);
    // a notice is closed by Esc before anything else
    p.notice = Some("x".into());
    p.dispatch(Command::Escape, t0);
    assert!(p.notice.is_none() && p.fullscreen.on);
    p.dispatch(Command::Quit, t0);
    assert!(p.quit);
}

#[test]
fn commands_have_a_busy_state_and_a_size_before_anything_opens() {
    let d = tempfile::tempdir().unwrap();
    let mut p = player(d.path());
    assert!(p.output_size().is_none());
    p.dispatch(Command::Open(file(d.path(), "a.mp4")), now());
    assert!(p.opening() && p.busy());
    assert!(p.status.contains("Opening"));
    settle(&mut p);
    assert!(!p.opening());
    assert!(
        p.line_boxes(&mut |_, _, _| eframe::egui::vec2(10.0, 10.0))
            .is_empty()
    );
}

// covers: REQ-138
#[test]
fn the_players_dependency_graph_has_no_network_and_no_speech_code() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = std::process::Command::new(cargo)
        .args(["tree", "--offline", "--locked", "-p", "videolore-player"])
        .args(["-e", "normal", "--prefix", "none"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = tree
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert!(names.contains(&"videolore") && names.contains(&"eframe"));
    for banned in [
        "ureq",
        "whisper-rs",
        "whisper-rs-sys",
        "rustls",
        "reqwest",
        "hyper",
    ] {
        assert!(!names.contains(&banned), "the player depends on {banned}");
    }
    // the editor's own graph does have them: the check can fail
    let editor =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args([
                "tree",
                "--offline",
                "--locked",
                "-p",
                "videolore",
                "-e",
                "normal",
                "--prefix",
                "none",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .unwrap();
    assert!(String::from_utf8_lossy(&editor.stdout).contains("ureq"));
}

// covers: REQ-150
#[test]
fn a_start_without_a_video_opens_the_start_screen_and_one_with_a_video_does_not() {
    let without = crate::real_app(None);
    assert!(without.state.splash);
    let with = crate::real_app(Some("/nowhere/a.mp4".into()));
    assert!(!with.state.splash);
    // Esc and CloseSplash close it
    let mut s = crate::real_app(None).state;
    s.dispatch(Command::CloseSplash, now());
    assert!(!s.splash);
    s.show_splash();
    s.dispatch(Command::Escape, now());
    assert!(!s.splash);
}

// covers: REQ-295
#[test]
fn the_player_follows_the_splash_option_and_remembers_the_version() {
    use videolore::settings::SplashOption::*;
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("player.json");
    let make = || {
        player(d.path()).with_settings(
            crate::settings::PlayerSettings::load(&cfg),
            Some(cfg.clone()),
            now(),
        )
    };
    let mut never = make();
    never.start_splash(Never);
    assert!(!never.splash);
    let mut always = make();
    always.start_splash(Always);
    assert!(always.splash);
    // once for each version: the first start shows it, the next one does not
    std::fs::remove_file(&cfg).unwrap();
    let mut first = make();
    first.start_splash(OncePerVersion);
    assert!(first.splash);
    let mut second = make();
    second.start_splash(OncePerVersion);
    assert!(!second.splash);
}
