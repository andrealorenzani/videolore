// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the command dispatcher and the background jobs it drives.

use super::*;
use crate::ffmpeg::FfVersion;
use crate::ffmpeg::fake::{FakeRunner, ok};
use crate::media::tests::MP4_JSON;
use crate::preview::NullSink;
use crate::settings::{PreviewQuality, SubtitleQuality};
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

#[derive(Default, Clone)]
pub struct FakeDialogs {
    pub media: Vec<PathBuf>,
    pub open: Option<PathBuf>,
    pub save: Option<PathBuf>,
    pub export: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
    pub model: Option<PathBuf>,
    pub lut: Option<PathBuf>,
    /// The folder "..." in the Export dialog chooses.
    pub folder: Option<PathBuf>,
    /// Where "Export subtitles" saves, and the name it proposed.
    pub srt: Option<PathBuf>,
    pub srt_name: Arc<std::sync::Mutex<String>>,
    /// How many times the "save as" dialog was shown.
    pub save_asked: Arc<AtomicUsize>,
}

impl Dialogs for FakeDialogs {
    fn pick_media(&mut self) -> Vec<PathBuf> {
        self.media.clone()
    }
    fn pick_project_open(&mut self) -> Option<PathBuf> {
        self.open.clone()
    }
    fn pick_project_save(&mut self) -> Option<PathBuf> {
        self.save_asked.fetch_add(1, Ordering::SeqCst);
        self.save.clone()
    }
    fn pick_export(&mut self, _extension: &str) -> Option<PathBuf> {
        self.export.clone()
    }
    fn pick_ffmpeg(&mut self) -> Option<PathBuf> {
        self.ffmpeg.clone()
    }
    fn pick_folder(&mut self, _start: &Path) -> Option<PathBuf> {
        self.folder.clone()
    }
    fn pick_model_file(&mut self) -> Option<PathBuf> {
        self.model.clone()
    }
    fn pick_lut(&mut self) -> Option<PathBuf> {
        self.lut.clone()
    }
    fn pick_subtitles_save(&mut self, default_name: &str) -> Option<PathBuf> {
        *self.srt_name.lock().unwrap() = default_name.to_string();
        self.srt.clone()
    }
}

const AUDIO_JSON: &str = r#"{"streams":[{"codec_type":"audio","codec_name":"mp3"}],
    "format":{"format_name":"mp3","duration":"30.0"}}"#;

const BIG_JSON: &str = r#"{"streams":[
    {"codec_type":"video","codec_name":"h264","width":3840,"height":2160,
     "avg_frame_rate":"30/1","r_frame_rate":"30/1","bit_rate":"40000000"},
    {"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}],
    "format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"5.0"}}"#;

/// ffprobe answers by file name; ffmpeg writes its output file and reports progress;
/// the speech extraction writes 3 s of silence.
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
            if last.ends_with("big.mov") {
                return ok(BIG_JSON);
            }
            if last.ends_with(".mp4") {
                return ok(MP4_JSON);
            }
            return crate::ffmpeg::fake::fail("Invalid data found");
        }
        if let Some(out) = last.strip_prefix("file:") {
            if a.iter().any(|x| x == "f32le") {
                std::fs::write(out, vec![0u8; 3 * 16_000 * 4]).ok()?;
                return ok("");
            }
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
        filters: ["subtitles", "psnr"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        encoders: ["libx264", "aac"].iter().map(|s| s.to_string()).collect(),
        source: Default::default(),
    }
}

pub fn state_with(dialogs: FakeDialogs, runner: FakeRunner) -> AppState {
    AppState::with_ffmpeg(
        Ok(info()),
        Arc::new(runner),
        Box::new(dialogs),
        Box::new(NullSink),
    )
}

pub fn state(dialogs: FakeDialogs) -> AppState {
    state_with(dialogs, runner())
}

/// Two 5 s video files and one 30 s audio file in `dir`.
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

/// Ticks until background work (analysis, proxies, subtitles) is over.
pub fn settle(app: &mut AppState) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.tick(Instant::now());
        let analysing = app
            .project
            .media
            .iter()
            .any(|m| m.status == MediaStatus::Analysing);
        if !analysing && app.proxy_jobs.is_empty() && !app.is_generating() {
            return;
        }
        assert!(Instant::now() < deadline, "background work did not finish");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Imports `files` and waits for the analysis.
pub fn import(app: &mut AppState, files: Vec<PathBuf>) {
    app.dispatch(Command::Import(files), Instant::now())
        .unwrap();
    settle(app);
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
    settle(&mut app);
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
    assert_eq!(app.title(), "Untitled • — videolore");
}

// covers: REQ-004
#[test]
fn fifty_operations_can_be_undone() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    import(&mut app, vec![f[0].clone()]);
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

// covers: REQ-002, REQ-032
#[test]
fn unreadable_files_are_rejected_in_the_list_without_crashing() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("notes.txt");
    std::fs::write(&bad, "hello").unwrap();
    let mut f = files(dir.path());
    f.push(bad);
    f.push(dir.path().join("gone.mp4"));
    let mut app = state(FakeDialogs::default());
    app.dispatch(Command::Import(f), Instant::now()).unwrap();
    // the list shows every file at once while it is analysed
    assert_eq!(app.project.media.len(), 5);
    settle(&mut app);
    let status = |i: usize| app.project.media[i].status.clone();
    assert_eq!(status(0), MediaStatus::Ready);
    assert!(matches!(status(3), MediaStatus::Rejected(e) if e.contains("notes.txt")));
    assert!(matches!(status(4), MediaStatus::Rejected(e) if e.contains("gone.mp4")));
    assert!(app.status.contains("could not be imported"));
    // rejected files are not part of the project and can be removed from the list
    let rejected = app.project.media[3].id;
    assert!(app.is_enabled(&Command::RemoveMedia(rejected)));
    app.dispatch(Command::RemoveMedia(rejected), Instant::now())
        .unwrap();
    assert_eq!(app.project.media.len(), 4);
    // audio-only is accepted: it cannot go on the video track but goes on the music track
    let song = app.project.media[2].clone();
    assert!(song.is_ready() && !song.has_video());
    assert!(!app.is_enabled(&Command::Append(Some(song.id))));
    assert!(app.is_enabled(&Command::PlaceMusic(Some(song.id))));
    app.dispatch(Command::PlaceMusic(Some(song.id)), Instant::now())
        .unwrap();
    assert_eq!(app.project.music.clips.len(), 1);
    // a file on a track cannot be removed
    assert!(!app.is_enabled(&Command::RemoveMedia(song.id)));
    assert!(
        app.dispatch(Command::RemoveMedia(song.id), Instant::now())
            .is_err()
    );
    app.dispatch(Command::DismissNotice, Instant::now())
        .unwrap();
    assert!(app.notice.is_none());
}

/// ffprobe that takes a while, like on a slow disk.
fn slow_runner() -> FakeRunner {
    let inner = runner();
    FakeRunner::default().on(move |prog, a| {
        if prog.ends_with("ffprobe") {
            std::thread::sleep(Duration::from_millis(60));
        }
        let args: Vec<std::ffi::OsString> = a.iter().map(Into::into).collect();
        inner.run(prog, &args).ok()
    })
}

// covers: REQ-032, REQ-033
#[test]
fn importing_ten_files_never_blocks_editing() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state_with(FakeDialogs::default(), slow_runner());
    import(&mut app, vec![f[0].clone()]);
    let a = app.project.media[0].id;
    let many: Vec<PathBuf> = (0..10)
        .map(|i| {
            let p = dir.path().join(format!("clip{i}.mp4"));
            std::fs::write(&p, "x").unwrap();
            p
        })
        .collect();
    let t = Instant::now();
    app.dispatch(Command::Import(many), Instant::now()).unwrap();
    // ten probes of 60 ms each would take 600 ms on the UI thread
    assert!(t.elapsed() < Duration::from_millis(50), "{:?}", t.elapsed());
    assert_eq!(
        app.project
            .media
            .iter()
            .filter(|m| m.status == MediaStatus::Analysing)
            .count(),
        10
    );
    assert!(app.busy());
    // editing goes on meanwhile
    app.dispatch(Command::Append(Some(a)), Instant::now())
        .unwrap();
    app.dispatch(Command::Undo, Instant::now()).unwrap();
    app.tick(Instant::now());
    // analysing files are not saved and do not make the project dirty yet
    assert!(app.project.doc() == Project::default().doc() || app.dirty);
    settle(&mut app);
    assert!(app.project.media.iter().all(MediaItem::is_ready));
    assert!(app.scheduler().peak() <= MAX_PROCESSES);
    assert!(app.scheduler().peak() >= 2, "probes ran in parallel");
}

// covers: REQ-003, REQ-026, REQ-027
#[test]
fn save_and_open_through_commands() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let proj = dir.path().join("my");
    let dialogs = FakeDialogs {
        save: Some(proj.clone()),
        open: Some(proj.with_extension("vlproj")),
        ..Default::default()
    };
    let asked = Arc::clone(&dialogs.save_asked);
    let mut app = state(dialogs);
    let now = Instant::now();
    import(&mut app, f.clone());
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    assert_eq!(app.title(), "Untitled • — videolore");
    app.dispatch(Command::SaveProject, now).unwrap(); // never saved: asks for a path
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    let saved = proj.with_extension("vlproj");
    assert_eq!(app.project_path.as_deref(), Some(saved.as_path()));
    assert!(!app.dirty);
    assert_eq!(app.title(), "my.vlproj — videolore");
    // REQ-026: edit, Save -> the same file is updated without a dialog
    app.dispatch(Command::Append(Some(app.project.media[1].id)), now)
        .unwrap();
    assert_eq!(app.title(), "my.vlproj • — videolore");
    app.dispatch(Command::SaveProject, now).unwrap();
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    assert!(!app.dirty);
    let (on_disk, _) = project::load(&saved).unwrap();
    assert_eq!(on_disk.timeline.clips.len(), 2);
    // REQ-027: edit then undo back to the saved state clears the marker
    app.dispatch(Command::Split, now).unwrap_or(());
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    assert!(app.dirty);
    app.dispatch(Command::Undo, now).unwrap();
    while app.dirty && app.history.can_undo() {
        app.dispatch(Command::Undo, now).unwrap();
        if app.project.timeline.clips.len() == 2 && !app.project.timeline.has_fade(0) {
            break;
        }
    }
    assert!(!app.dirty);
    // Save As writes a new file and switches to it
    app.dispatch(Command::SaveProjectAs(dir.path().join("other")), now)
        .unwrap();
    assert!(dir.path().join("other.vlproj").is_file());
    app.dispatch(Command::PickSaveProjectAs, now).unwrap();
    assert_eq!(app.project_path.as_deref(), Some(saved.as_path()));
    let snapshot = app.project.clone();
    app.dispatch(Command::NewProject, now).unwrap();
    assert!(app.project.media.is_empty());
    assert_eq!(app.title(), "Untitled — videolore");
    std::fs::remove_file(&f[1]).unwrap();
    app.dispatch(Command::PickOpenProject, now).unwrap();
    assert_eq!(app.project.timeline, snapshot.timeline);
    assert!(app.notice.as_deref().unwrap().contains("b.mp4"));
    assert!(app.project.media[1].missing);
    assert!(!app.dirty);
    assert!(
        app.dispatch(Command::OpenProject(dir.path().join("none.vlproj")), now)
            .is_err()
    );
    assert!(!app.status.is_empty());
}

/// A project with one clip and unsaved changes, plus a saved project to open (the one the
/// open dialog picks).
fn dirty_app() -> (tempfile::TempDir, AppState, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let other = dir.path().join("other.vlproj");
    let mut app = state(FakeDialogs {
        open: Some(other.clone()),
        ..Default::default()
    });
    import(&mut app, vec![f[0].clone()]);
    app.dispatch(Command::SaveProjectAs(other.clone()), Instant::now())
        .unwrap();
    app.dispatch(Command::Append(None), Instant::now()).unwrap();
    assert!(app.dirty);
    (dir, app, other)
}

// covers: REQ-028
#[test]
fn unsaved_changes_prompt_before_new_open_close_and_quit() {
    let now = Instant::now();
    let actions = [
        Command::NewProject,
        Command::PickOpenProject,
        Command::RequestClose,
        Command::Quit,
    ];
    for action in actions {
        // Cancel: nothing changes
        let (_d, mut app, _) = dirty_app();
        let clips = app.project.timeline.clips.len();
        app.dispatch(action.clone(), now).unwrap();
        assert!(matches!(app.modal, Some(Modal::Unsaved(_))), "{action:?}");
        app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Cancel), now)
            .unwrap();
        assert!(app.modal.is_none());
        assert_eq!(app.project.timeline.clips.len(), clips);
        assert!(app.dirty && !app.quit_requested && !app.close_allowed);
        // Discard: the action goes on, the file on disk keeps the old content
        let (_d, mut app, other) = dirty_app();
        app.dispatch(action.clone(), now).unwrap();
        app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Discard), now)
            .unwrap();
        match action {
            Command::NewProject => assert!(app.project.media.is_empty()),
            Command::PickOpenProject => {
                assert!(app.project.timeline.clips.is_empty());
                assert_eq!(app.project_path.as_deref(), Some(other.as_path()));
            }
            _ => assert!(app.quit_requested && app.close_allowed),
        }
        assert!(project::load(&other).unwrap().0.timeline.clips.is_empty());
        // Save: saved first, then the action goes on
        let (_d, mut app, other) = dirty_app();
        app.dispatch(action.clone(), now).unwrap();
        app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Save), now)
            .unwrap();
        assert_eq!(project::load(&other).unwrap().0.timeline.clips.len(), 1);
        if matches!(action, Command::RequestClose | Command::Quit) {
            assert!(app.quit_requested);
        }
    }
    // a failed save stops the action
    let (dir, mut app, _) = dirty_app();
    app.project_path = Some(dir.path().join("no/such/folder/p.vlproj"));
    app.dispatch(Command::Quit, now).unwrap();
    assert!(
        app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Save), now)
            .is_err()
    );
    assert!(!app.quit_requested && app.dirty);
    // a cancelled Save As stops the action
    let mut app = state(FakeDialogs::default());
    let dir = tempfile::tempdir().unwrap();
    import(&mut app, vec![files(dir.path())[0].clone()]);
    app.dispatch(Command::Append(None), now).unwrap();
    app.dispatch(Command::NewProject, now).unwrap();
    app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Save), now)
        .unwrap();
    assert_eq!(app.project.timeline.clips.len(), 1);
    // nothing to resolve: no-op
    app.dispatch(Command::ResolveUnsaved(UnsavedChoice::Discard), now)
        .unwrap();
    assert_eq!(app.project.timeline.clips.len(), 1);
    // no prompt when there is nothing to save
    let mut clean = state(FakeDialogs::default());
    clean.dispatch(Command::RequestClose, now).unwrap();
    assert!(clean.modal.is_none() && clean.close_allowed && clean.quit_requested);
}

// covers: REQ-041, REQ-028
#[test]
fn projects_opened_by_the_os_go_through_the_prompt() {
    fn requests() -> Vec<PathBuf> {
        vec![PathBuf::from("/nowhere/x.vlproj")]
    }
    let (_d, mut app, _) = dirty_app();
    app.open_requests = requests;
    app.tick(Instant::now());
    assert_eq!(
        app.modal,
        Some(Modal::Unsaved(PendingAction::Open(Some(
            "/nowhere/x.vlproj".into()
        ))))
    );
    // when clean it opens directly (here it fails: the file does not exist)
    let mut clean = state(FakeDialogs::default());
    clean.open_requests = requests;
    clean.tick(Instant::now());
    assert!(clean.modal.is_none());
    assert!(!clean.status.is_empty());
}

// covers: REQ-019, REQ-043, REQ-020
#[test]
fn music_track_commands_are_undoable_and_independent_of_video_edits() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    import(&mut app, f);
    let (a, b, song) = (
        app.project.media[0].id,
        app.project.media[1].id,
        app.project.media[2].id,
    );
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    app.dispatch(Command::Append(Some(b)), now).unwrap();
    // place at the playhead
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    assert!(!app.is_enabled(&Command::PlaceMusic(Some(a))));
    app.dispatch(Command::PlaceMusic(Some(song)), now).unwrap();
    let m = app.selected_music.unwrap();
    assert_eq!(app.project.music.get(m).unwrap().start, s(1));
    // overlapping placement is refused
    assert!(app.dispatch(Command::PlaceMusic(Some(song)), now).is_err());
    // move, trim, split, delete
    app.dispatch(
        Command::MoveMusic {
            clip: m,
            start: s(2),
        },
        now,
    )
    .unwrap();
    app.dispatch(
        Command::TrimMusic {
            clip: m,
            src_in: Micros::ZERO,
            src_out: s(6),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.music.get(m).unwrap().end(), s(8));
    // video edits do not move the music
    let before = app.project.music.clone();
    app.dispatch(
        Command::SelectClip(Some(app.project.timeline.clips[0].id)),
        now,
    )
    .unwrap();
    app.dispatch(Command::Delete, now).unwrap();
    assert_eq!(app.project.music, before);
    app.dispatch(Command::Undo, now).unwrap();
    app.dispatch(Command::Seek(s(4)), now).unwrap();
    assert!(app.is_enabled(&Command::SplitMusic));
    app.dispatch(Command::SplitMusic, now).unwrap();
    assert_eq!(app.project.music.clips.len(), 2);
    app.dispatch(Command::SelectMusic(Some(m)), now).unwrap();
    assert!(app.is_enabled(&Command::DeleteMusic));
    app.dispatch(Command::DeleteMusic, now).unwrap();
    assert_eq!(app.project.music.clips.len(), 1);
    assert!(app.selected_music.is_none());
    assert!(app.dispatch(Command::DeleteMusic, now).is_err());
    assert!(
        app.dispatch(
            Command::TrimMusic {
                clip: MusicClipId(99),
                src_in: Micros::ZERO,
                src_out: s(1)
            },
            now
        )
        .is_err()
    );
    for _ in 0..3 {
        app.dispatch(Command::Undo, now).unwrap();
    }
    assert_eq!(app.project.music.get(m).unwrap().start, s(2));
    // volumes and mute are undoable and saved
    let mix = Mix {
        video_volume: 40,
        video_muted: false,
        music_volume: 250,
        music_muted: true,
    };
    app.dispatch(Command::SetMix(mix), now).unwrap();
    assert_eq!(app.project.mix.music_volume, 100);
    let undo_len = app.history.undo_len();
    app.dispatch(Command::SetMix(app.volume_mix()), now)
        .unwrap();
    assert_eq!(
        app.history.undo_len(),
        undo_len,
        "no-op mix is not an undo step"
    );
    let path = dir.path().join("mix.vlproj");
    app.dispatch(Command::SaveProjectAs(path.clone()), now)
        .unwrap();
    let (loaded, _) = project::load(&path).unwrap();
    assert_eq!(loaded.mix, app.project.mix);
    assert_eq!(loaded.music, app.project.music);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.mix, Mix::default());
}

/// Serves, for each built-in address, the file `store_for` expects for that level.
pub struct LevelFetcher {
    pub store: ModelStore,
    pub body: Vec<u8>,
    pub calls: Arc<AtomicUsize>,
}

impl ModelFetcher for LevelFetcher {
    fn fetch(
        &self,
        url: &str,
        max_bytes: u64,
        out: &mut dyn std::io::Write,
        progress: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), stt::SttError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let i = self
            .store
            .specs
            .iter()
            .position(|s| s.url == url)
            .expect("a built-in address");
        let body = crate::stt::tests::level_body(&self.body, i);
        crate::stt::tests::FakeFetcher::new(body).fetch(url, max_bytes, out, progress, cancel)
    }
}

/// Speech files + transcriber fakes installed on `app` (Subtitle quality: Fast); returns
/// the base bytes of the fake files (see `stt::tests::store_for`).
pub fn with_fake_speech(app: &mut AppState, dir: &Path) -> Vec<u8> {
    let body = b"tiny fake model".to_vec();
    let store = crate::stt::tests::store_for(dir, &body);
    app.fetcher = Some(Arc::new(LevelFetcher {
        store: store.clone(),
        body: body.clone(),
        calls: Arc::new(AtomicUsize::new(0)),
    }));
    app.model_store = Some(store);
    app.settings.subtitle_quality = SubtitleQuality::Fast;
    app.transcriber = Arc::new(|_, _| {
        Ok(Box::new(crate::stt::tests::FakeTranscriber::default()) as Box<dyn stt::Transcriber>)
    });
    body
}

// covers: REQ-014, REQ-015, REQ-044, REQ-016
#[test]
fn subtitles_are_generated_after_consent_and_edited_with_undo() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    import(&mut app, f.clone());
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    if !stt::available() {
        assert!(!app.is_enabled(&Command::GenerateSubtitles));
        assert!(app.dispatch(Command::GenerateSubtitles, now).is_err());
        return;
    }
    // without a model folder nothing can be generated
    assert!(app.dispatch(Command::GenerateSubtitles, now).is_err());
    let body = with_fake_speech(&mut app, dir.path());
    // consent comes before any download
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert_eq!(app.modal, Some(Modal::ModelConsent(SubtitleQuality::Fast)));
    assert!(!app.model_store.as_ref().unwrap().dir.exists());
    app.dispatch(Command::ModelConsent(ConsentChoice::Cancel), now)
        .unwrap();
    assert!(app.modal.is_none() && !app.is_generating());
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
        .unwrap();
    assert!(app.is_generating());
    assert!(app.is_enabled(&Command::CancelSubtitles));
    settle(&mut app);
    assert!(
        app.model_store
            .as_ref()
            .unwrap()
            .is_installed(SubtitleQuality::Fast)
    );
    // the fake transcriber hears one word per second of the 3 s extracted
    assert_eq!(app.project.subtitles.lines.len(), 3);
    assert_eq!(app.project.subtitles.language.as_deref(), Some("it"));
    assert!(app.status.contains("3 subtitle"));
    // generation is undoable
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.subtitles.lines.is_empty());
    app.dispatch(Command::Redo, now).unwrap();
    // REQ-016 edits
    let first = app.project.subtitles.lines[0].id;
    app.dispatch(Command::SetSubtitleText(first, "Hello there".into()), now)
        .unwrap();
    let undo_len = app.history.undo_len();
    app.dispatch(Command::SetSubtitleText(first, "Hello there".into()), now)
        .unwrap();
    assert_eq!(app.history.undo_len(), undo_len);
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::RetimeSubtitle {
            line: first,
            clip,
            start: Micros(100_000),
            end: Micros(900_000),
        },
        now,
    )
    .unwrap();
    let l = app.project.subtitles.get(first).unwrap();
    assert_eq!(
        (l.start, l.end, l.text.as_str()),
        (Micros(100_000), Micros(900_000), "Hello there")
    );
    app.dispatch(Command::Seek(s(4)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let added = app.selection.primary_line().unwrap();
    assert_eq!(app.project.subtitles.lines.len(), 4);
    app.dispatch(Command::SelectLine(Some(added)), now).unwrap();
    app.dispatch(Command::DeleteSubtitle(added), now).unwrap();
    assert!(app.selection.primary_line().is_none());
    assert!(app.dispatch(Command::DeleteSubtitle(added), now).is_err());
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.subtitles.get(added).is_some());
    app.dispatch(Command::SetPreset(Preset::WordHighlight), now)
        .unwrap();
    app.dispatch(Command::SetPreset(Preset::WordHighlight), now)
        .unwrap();
    assert_eq!(app.project.subtitles.preset, Preset::WordHighlight);
    // REQ-015: a chosen language is used for the next generation
    assert!(
        app.dispatch(Command::SetLanguage(Some("English".into())), now)
            .is_err()
    );
    assert!(
        app.dispatch(Command::SetLanguage(Some("E1".into())), now)
            .is_err()
    );
    app.dispatch(Command::SetLanguage(Some("en".into())), now)
        .unwrap();
    assert!(app.project.subtitles.language_overridden);
    // REQ-044: manual edits exist, so regenerating asks first
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert_eq!(app.modal, Some(Modal::ConfirmRegenerate));
    app.dispatch(Command::CloseModal, now).unwrap();
    assert_eq!(app.project.subtitles.lines.len(), 4);
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    app.dispatch(Command::ConfirmRegenerate, now).unwrap();
    assert!(app.dispatch(Command::GenerateSubtitles, now).is_err() || app.modal.is_some());
    settle(&mut app);
    assert_eq!(app.project.subtitles.lines.len(), 3);
    assert_eq!(app.project.subtitles.language.as_deref(), Some("en"));
    // back to automatic detection
    app.dispatch(Command::SetLanguage(None), now).unwrap();
    assert!(!app.project.subtitles.language_overridden);
    // a local model file must be the pinned one
    let other = tempfile::tempdir().unwrap();
    let local = other.path().join("model.bin");
    std::fs::write(&local, crate::stt::tests::level_body(&body, 0)).unwrap();
    let mut app2 = state(FakeDialogs {
        model: Some(local),
        ..Default::default()
    });
    with_fake_speech(&mut app2, other.path());
    app2.modal = Some(Modal::ModelConsent(SubtitleQuality::Fast));
    app2.dispatch(Command::ModelConsent(ConsentChoice::LocalFile), now)
        .unwrap();
    settle(&mut app2);
    assert!(
        app2.model_store
            .as_ref()
            .unwrap()
            .is_installed(SubtitleQuality::Fast)
    );
    // the timeline is empty: nothing to transcribe
    assert!(app2.status.contains("no sound") || app2.status.contains("installed"));
}

// covers: REQ-014
#[test]
fn subtitle_generation_can_be_cancelled_and_failures_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    import(&mut app, f);
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    if !stt::available() {
        return;
    }
    with_fake_speech(&mut app, dir.path());
    // a failing download is reported and nothing is installed
    app.fetcher = Some(Arc::new(crate::stt::tests::FakeFetcher::new(
        b"wrong".to_vec(),
    )));
    app.modal = Some(Modal::ModelConsent(SubtitleQuality::Fast));
    app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
        .unwrap();
    settle(&mut app);
    assert!(
        app.notice
            .take()
            .unwrap()
            .contains("speech recognition file")
    );
    // install, then cancel a running generation
    with_fake_speech(&mut app, dir.path());
    app.transcriber = Arc::new(|_, _| {
        std::thread::sleep(Duration::from_millis(100));
        Ok(Box::new(crate::stt::tests::FakeTranscriber::default()) as Box<dyn stt::Transcriber>)
    });
    app.modal = Some(Modal::ModelConsent(SubtitleQuality::Fast));
    app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
        .unwrap();
    // the model arrives and generation starts by itself
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.stt.is_none() && Instant::now() < deadline {
        app.tick(Instant::now());
    }
    assert!(app.stt.is_some());
    assert!(app.dispatch(Command::ConfirmRegenerate, now).is_err());
    app.dispatch(Command::CancelSubtitles, now).unwrap();
    settle(&mut app);
    assert_eq!(app.status, "Subtitle generation cancelled");
    assert!(app.project.subtitles.lines.is_empty());
    // a transcriber that cannot load
    app.transcriber = Arc::new(|_, _| Err(stt::SttError::Model("broken".into())));
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    settle(&mut app);
    assert!(app.notice.as_deref().unwrap().contains("broken"));
    // only silent clips: nothing to transcribe
    let mut silent = app.project.media[0].clone();
    silent.format.acodec = None;
    app.project.media[0] = silent;
    assert!(app.dispatch(Command::GenerateSubtitles, now).is_err());
}

// covers: REQ-038
#[test]
fn big_sources_get_a_proxy_that_only_the_preview_uses() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let big = dir.path().join("big.mov");
    std::fs::write(&big, "4k").unwrap();
    let small = files(dir.path())[0].clone();
    let mut app = state(FakeDialogs::default());
    app.proxy_cache = Some(ProxyCache::new(cache.clone()));
    app.machine = Machine::simulated(4, 4, 4 * GIB);
    app.dispatch(Command::Import(vec![big.clone(), small]), Instant::now())
        .unwrap();
    settle(&mut app);
    let (b, s_) = (&app.project.media[0], &app.project.media[1]);
    let proxy = b.proxy_path().expect("proxy ready").to_path_buf();
    assert!(proxy.starts_with(&cache) && proxy.is_file());
    // exactly 1280x720 is not above the small-machine limit: no proxy
    assert_eq!(s_.proxy, ProxyState::None);
    assert!(app.proxy_cache_size > 0);
    // export always reads the originals
    app.dispatch(
        Command::Append(Some(app.project.media[0].id)),
        Instant::now(),
    )
    .unwrap();
    let segs = render::segments(&app.project, Micros::ZERO, render::Source::Export).unwrap();
    assert_eq!(segs[0].path, app.project.media[0].path);
    let segs = render::segments(
        &app.project,
        Micros::ZERO,
        render::Source::Preview { proxies: true },
    )
    .unwrap();
    assert_eq!(segs[0].path, proxy);
    // Preview quality "Original" is saved and used by the preview
    let settings = dir.path().join("settings.json");
    app.settings_path = Some(settings.clone());
    app.dispatch(
        Command::SetPreviewQuality(PreviewQuality::Original),
        Instant::now(),
    )
    .unwrap();
    assert!(!app.preview.proxies);
    assert_eq!(app.project.media[0].proxy, ProxyState::None);
    assert_eq!(
        read_settings(&settings).preview_quality,
        PreviewQuality::Original
    );
    assert!(proxy.exists(), "the copy stays in the cache");
    // clearing the cache deletes our files only
    let foreign = cache.join("keep-me.txt");
    std::fs::write(&foreign, "x").unwrap();
    app.dispatch(Command::ClearProxyCache, Instant::now())
        .unwrap();
    assert!(!proxy.exists() && foreign.exists());
    assert_eq!(app.project.media[0].proxy, ProxyState::None);
    assert_eq!(app.proxy_cache_size, 0);
    // on again: proxies are made again
    app.dispatch(
        Command::SetPreviewQuality(PreviewQuality::Automatic),
        Instant::now(),
    )
    .unwrap();
    settle(&mut app);
    assert!(app.project.media[0].proxy_path().is_some());
    // a new project cancels pending work
    app.dispatch(
        Command::SetPreviewQuality(PreviewQuality::Original),
        Instant::now(),
    )
    .unwrap();
    app.dispatch(Command::ClearProxyCache, Instant::now())
        .unwrap();
    app.dispatch(
        Command::SetPreviewQuality(PreviewQuality::Automatic),
        Instant::now(),
    )
    .unwrap();
    app.dirty = false;
    app.saved = app.project.doc();
    app.dispatch(Command::NewProject, Instant::now()).unwrap();
    assert!(app.proxy_jobs.is_empty());
    let no_cache = state(FakeDialogs::default());
    assert!(!no_cache.is_enabled(&Command::ClearProxyCache));
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
    import(&mut app, f.clone());
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
    let Some(Modal::Export(c)) = app.modal.clone() else {
        panic!("Export opens the dialog");
    };
    let mut c = *c;
    c.folder = dir.path().to_path_buf();
    c.name = "out".into();
    app.dispatch(Command::StartExport(Box::new(c)), now)
        .unwrap();
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
    let mut app = state_with(FakeDialogs::default(), failing);
    let now = Instant::now();
    import(&mut app, vec![f[0].clone()]);
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
    // choosing a binary saves the setting and re-checks in the background
    app.dispatch(Command::PickFfmpeg, now).unwrap();
    assert!(app.ffmpeg_checking);
    assert_eq!(
        app.ffmpeg.as_ref().err().map(String::as_str),
        Some(CHECKING_FFMPEG)
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.ffmpeg_checking && Instant::now() < deadline {
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!app.ffmpeg_checking);
    assert_eq!(
        read_settings(&settings).ffmpeg,
        Some(dir.path().join("my-ffmpeg"))
    );
    // a relative path is refused; the included ffmpeg can be chosen again
    assert!(
        app.dispatch(Command::SetFfmpeg("ffmpeg".into()), now)
            .is_err()
    );
    app.dispatch(Command::ResetFfmpeg, now).unwrap();
    assert_eq!(read_settings(&settings).ffmpeg, None);
    assert_eq!(app.fps(), Fps { num: 30, den: 1 });
    assert!(app.output_description().is_none());
    assert_eq!(app.export_extension(), "mp4");
    assert!(app.create_subtitles(None).is_err());
}

#[test]
fn dialogs_cancelled_do_nothing() {
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    for c in [
        Command::PickImport,
        Command::PickOpenProject,
        Command::PickSaveProjectAs,
        Command::SaveProject,
        Command::PickExport,
        Command::PickFfmpeg,
        Command::CancelExport,
        Command::CancelSubtitles,
        Command::ShowAbout,
        Command::CloseModal,
        Command::SelectLine(None),
    ] {
        app.dispatch(c, now).unwrap();
    }
    assert!(app.project_path.is_none());
    for c in [
        Command::Copy,
        Command::Cut,
        Command::Paste,
        Command::Merge,
        Command::MoveLeft,
        Command::Undo,
        Command::ToggleFadeAtPlayhead,
        Command::Append(None),
        Command::PlaceMusic(None),
        Command::RemoveMedia(MediaId(7)),
        Command::AddSubtitle,
        Command::SetSubtitleText(LineId(3), "x".into()),
        Command::SetTrim {
            clip: ClipId(1),
            src_in: Micros::ZERO,
            src_out: Micros(1),
        },
    ] {
        assert!(app.dispatch(c.clone(), now).is_err(), "{c:?}");
    }
    // the model consent needs a folder for the model
    app.modal = Some(Modal::ModelConsent(SubtitleQuality::Fast));
    assert!(
        app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
            .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    with_fake_speech(&mut app, dir.path());
    // without the dialog nothing happens
    app.dispatch(Command::ModelConsent(ConsentChoice::LocalFile), now)
        .unwrap();
    app.modal = Some(Modal::ModelConsent(SubtitleQuality::Fast));
    app.dispatch(Command::ModelConsent(ConsentChoice::LocalFile), now)
        .unwrap();
    assert!(!app.is_generating());
    assert!(!app.is_enabled(&Command::RemoveDesktopIntegration));
}

#[test]
fn play_pause_and_tick() {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    // Preview streams stay open until killed, so the end of stream cannot race the clock.
    let mut app = state_with(FakeDialogs::default(), runner().holding_streams());
    let t0 = Instant::now();
    import(&mut app, vec![f[0].clone()]);
    app.dispatch(Command::Append(None), t0).unwrap();
    app.dispatch(Command::PlayPause, t0).unwrap();
    assert!(app.preview.is_playing());
    assert!(app.busy());
    app.tick(t0 + Duration::from_millis(500));
    assert_eq!(app.playhead, Micros(500_000));
    app.dispatch(Command::PlayPause, t0 + Duration::from_millis(600))
        .unwrap();
    assert!(!app.preview.is_playing());
    assert_eq!(app.playhead, Micros(600_000));
    // an edit during playback stops it where it was
    app.dispatch(Command::PlayPause, t0 + Duration::from_millis(600))
        .unwrap();
    let mix = Mix {
        video_volume: 50,
        ..Mix::default()
    };
    app.dispatch(Command::SetMix(mix), t0 + Duration::from_millis(900))
        .unwrap();
    assert!(!app.preview.is_playing());
    assert!(
        (app.playhead.0 - 900_000).abs() <= 33_334,
        "{:?}",
        app.playhead
    );
    // paste of a clip whose source was removed is refused
    app.clipboard = Some(Clip::new(ClipId(9), MediaId(99), Micros::ZERO, Micros(1)));
    assert!(app.dispatch(Command::Paste, t0).is_err());
    // removing the clip's file is refused while it is on the timeline
    let id = app.project.media[0].id;
    assert!(app.dispatch(Command::RemoveMedia(id), t0).is_err());
    // once off the timeline it can go; undo can no longer bring the clip back
    app.dispatch(Command::Delete, t0).unwrap();
    app.clipboard = None;
    app.dispatch(Command::RemoveMedia(id), t0).unwrap();
    assert!(!app.history.can_undo());
    assert!(app.project.media.is_empty());
}

pub fn read_settings(p: &Path) -> Settings {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

// covers: REQ-049, REQ-045
#[test]
fn upgrade_from_0_2_and_background_ffmpeg_check() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a/b/settings.json");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        r#"{"ffmpeg":"/x/ffmpeg","ffprobe":"/x/ffprobe","proxies_enabled":false}"#,
    )
    .unwrap();
    let mut app = AppState::new(
        Arc::new(crate::ffmpeg::fake::healthy()),
        Box::new(FakeDialogs::default()),
        Box::new(NullSink),
        Some(p.clone()),
    );
    assert_eq!(app.settings.preview_quality, PreviewQuality::Original);
    assert_eq!(app.settings.ffmpeg, Some("/x/ffmpeg".into()));
    assert!(!app.preview.proxies);
    assert_eq!(
        app.banner.as_ref().map(|b| b.text.as_str()),
        Some(settings::SETUP_NOTICE)
    );
    assert_eq!(read_settings(&p), app.settings, "the upgrade is saved");
    // the window opens at once; the check runs in the background
    assert!(app.ffmpeg_checking);
    assert!(!app.is_enabled(&Command::PickImport));
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.ffmpeg_checking && Instant::now() < deadline {
        app.tick(Instant::now());
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!app.ffmpeg_checking);
    assert!(app.machine.logical >= 1);
    assert_eq!(app.scheduler().cap(), app.budget.max_processes);
    // the next start: no message, same settings
    let again = AppState::new(
        Arc::new(crate::ffmpeg::fake::healthy()),
        Box::new(FakeDialogs::default()),
        Box::new(NullSink),
        Some(p),
    );
    assert!(again.banner.is_none());
    assert_eq!(again.settings, app.settings);
}
