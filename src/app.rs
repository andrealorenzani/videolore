// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Application state and the single command dispatcher: every button, menu entry and
//! shortcut ends up in [`AppState::dispatch`]. Slow work (ffprobe, proxies, speech
//! recognition, the ffmpeg check, export) runs on other threads; [`AppState::tick`]
//! collects the results once per frame.

use crate::export::{self, ExportEvent, ExportHandle, ExportJob, ExportMethod};
use crate::ffmpeg::{self, FfmpegInfo, Runner};
use crate::history::History;
use crate::hwenc::{self, Codec, HwOffer, HwStatus, Platform, ProbeLimits};
use crate::jobs::{Limited, MAX_CAP, MAX_PROCESSES, Pool, Priority, Scheduler};
use crate::machine::{Fingerprint, GIB, Machine};
use crate::media::{self, MediaId, MediaItem, MediaStatus, ProxyState};
use crate::music::{Mix, MusicClipId};
use crate::preview::{AudioSink, Frame, Preview, PreviewConfig};
use crate::project::{self, Edit, Project, ProjectDoc};
use crate::proxy::{self, ProxyCache, ProxyEvent, ProxyTask, ProxyWorker};
use crate::render;
use crate::settings::{
    self, Budget, Effort, ExportChoice, PreviewQuality, Settings, StartNotice, SubtitleQuality,
};
use crate::stt::{
    self, ModelEvent, ModelFetcher, ModelSource, ModelStore, SttEvent, SttHandle, SttJob,
    TranscriberFactory,
};
use crate::subtitles::{LineId, Preset};
use crate::time::{Fps, Micros};
use crate::timeline::{Clip, ClipId, EditError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// Answer to the "unsaved changes" prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedChoice {
    Save,
    Discard,
    Cancel,
}

/// Answer to the speech-file consent dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentChoice {
    /// Download the file of the chosen level (the only network access of the app).
    Download,
    /// Use the Fast file, already installed, for this generation only (no download).
    UseFastInstead,
    /// Download the small Fast file instead (when nothing is installed yet).
    DownloadFastInstead,
    /// Use a copy of the file the user already has.
    LocalFile,
    Cancel,
}

/// A non-blocking message at the top of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Banner {
    pub text: String,
    /// A button offered with the message.
    pub action: Option<BannerAction>,
    /// Closes by itself this long after it is first shown.
    pub lifetime: Option<Duration>,
    /// When it closes (set on the first tick).
    pub until: Option<Instant>,
}

/// What the banner's button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerAction {
    UseRecommended,
}

/// How long the "set up for this computer" message stays.
pub const SETUP_BANNER_TIME: Duration = Duration::from_secs(8);

/// Shown instead of generating subtitles on processors the speech engine cannot use.
pub const TOO_OLD_FOR_SUBTITLES: &str = "This computer's processor is too old for automatic \
     subtitles; you can still add subtitle lines by hand.";

impl Banner {
    pub fn from_notice(n: StartNotice) -> Banner {
        Banner {
            text: n.text().to_string(),
            action: (n == StartNotice::MachineChanged).then_some(BannerAction::UseRecommended),
            lifetime: (n == StartNotice::Setup).then_some(SETUP_BANNER_TIME),
            until: None,
        }
    }
}

/// What was interrupted by the "unsaved changes" prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    New,
    /// Open a project: the given file, or ask for one.
    Open(Option<PathBuf>),
    /// The window's close button.
    Close,
    /// Cmd/Ctrl+Q.
    Quit,
}

/// The dialog shown over the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    Unsaved(PendingAction),
    /// Consent to download the speech recognition file of a subtitle level.
    ModelConsent(SubtitleQuality),
    ConfirmRegenerate,
    /// Confirmation before removing the downloaded speech recognition files.
    ConfirmRemoveSpeechFiles,
    About,
}

/// Every user operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    NewProject,
    PickOpenProject,
    OpenProject(PathBuf),
    SaveProject,
    PickSaveProjectAs,
    SaveProjectAs(PathBuf),
    /// The window is being closed.
    RequestClose,
    Quit,
    ResolveUnsaved(UnsavedChoice),
    PickImport,
    Import(Vec<PathBuf>),
    RemoveMedia(MediaId),
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
    /// Puts the given audio file (or the selected one) on the music track at the playhead.
    PlaceMusic(Option<MediaId>),
    SelectMusic(Option<MusicClipId>),
    MoveMusic {
        clip: MusicClipId,
        start: Micros,
    },
    TrimMusic {
        clip: MusicClipId,
        src_in: Micros,
        src_out: Micros,
    },
    /// Splits the music clip under the playhead.
    SplitMusic,
    /// Deletes the selected music clip.
    DeleteMusic,
    SetMix(Mix),
    GenerateSubtitles,
    /// The user confirmed that manual subtitle edits may be replaced.
    ConfirmRegenerate,
    ModelConsent(ConsentChoice),
    CancelSubtitles,
    /// Adds a subtitle line at the playhead.
    AddSubtitle,
    DeleteSubtitle(LineId),
    SetSubtitleText(LineId, String),
    RetimeSubtitle {
        line: LineId,
        clip: ClipId,
        start: Micros,
        end: Micros,
    },
    SelectLine(Option<LineId>),
    SetPreset(Preset),
    /// A language code, or `None` to detect it.
    SetLanguage(Option<String>),
    OpenSettings,
    CloseSettings,
    SetPreviewQuality(PreviewQuality),
    SetSubtitleQuality(SubtitleQuality),
    SetEffort(Effort),
    SetExport(ExportChoice),
    /// Forgets the hardware problems and checks the video hardware again.
    TryHardwareAgain,
    /// Detects the machine again and applies the recommended choices.
    UseRecommended,
    /// Asks before removing the downloaded speech recognition files.
    RemoveSpeechFiles,
    ConfirmRemoveSpeechFiles,
    /// Uses the bundled (or system) ffmpeg again instead of the chosen one.
    ResetFfmpeg,
    /// The banner's button.
    BannerAction(BannerAction),
    DismissBanner,
    ClearProxyCache,
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
    ShowAbout,
    CloseModal,
    RemoveDesktopIntegration,
    DismissNotice,
}

/// Native file dialogs (faked in tests).
pub trait Dialogs {
    fn pick_media(&mut self) -> Vec<PathBuf>;
    fn pick_project_open(&mut self) -> Option<PathBuf>;
    fn pick_project_save(&mut self) -> Option<PathBuf>;
    fn pick_export(&mut self, extension: &str) -> Option<PathBuf>;
    fn pick_ffmpeg(&mut self) -> Option<PathBuf>;
    fn pick_model_file(&mut self) -> Option<PathBuf>;
}

/// State of a running or finished export, for the UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportStatus {
    pub running: bool,
    pub progress: f32,
    pub stream_copy: Option<bool>,
}

/// A file to analyse with ffprobe.
struct ProbeJob {
    id: MediaId,
    path: PathBuf,
    ffprobe: PathBuf,
    cancel: Arc<AtomicBool>,
}

type ProbeResult = (MediaId, Result<MediaItem, String>);

/// A proxy to generate; progress and the result go to `events`.
struct ProxyJob {
    worker: ProxyWorker,
    task: ProxyTask,
    events: Sender<ProxyEvent>,
}

/// The background check of the video hardware.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HwProbe {
    /// Not run (it runs only when Export is Faster or the user asks).
    #[default]
    NotChecked,
    /// Running; the number tells this run from older ones.
    Checking(u64),
    Done(HwStatus),
}

/// What the app knows about the video hardware (this session only).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HwState {
    pub probe: HwProbe,
    /// How the last export of this session was made.
    pub last_method: Option<ExportMethod>,
}

/// A hardware check in the background: its number, where its result arrives, its cancel
/// flag.
type RunningProbe = (u64, Receiver<(u64, HwStatus)>, Arc<AtomicBool>);

/// Message shown while the startup check of ffmpeg runs.
pub const CHECKING_FFMPEG: &str = "checking ffmpeg…";

pub struct AppState {
    pub project: Project,
    pub project_path: Option<PathBuf>,
    /// The project as last saved or opened (for the unsaved-changes marker).
    saved: ProjectDoc,
    pub dirty: bool,
    pub history: History<Edit>,
    pub selected_clip: Option<ClipId>,
    pub selected_media: Option<MediaId>,
    pub selected_music: Option<MusicClipId>,
    pub selected_line: Option<LineId>,
    pub playhead: Micros,
    pub clipboard: Option<Clip>,
    /// Timeline zoom in pixels per second (0 = fit all on next frame).
    pub zoom: f32,
    pub ffmpeg: Result<FfmpegInfo, String>,
    /// True while the ffmpeg check runs in the background.
    pub ffmpeg_checking: bool,
    ffmpeg_check: Option<Receiver<Result<FfmpegInfo, String>>>,
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    /// The Settings window is open.
    pub settings_open: bool,
    /// What "Use recommended settings" changed (shown in the Settings window).
    pub settings_summary: Vec<String>,
    /// What background work may use ("How hard videolore works").
    pub budget: Budget,
    /// Reads the machine again ("Use recommended settings"); replaced in tests.
    pub detect: fn() -> Machine,
    /// Root of the private work folders of background jobs.
    pub work_root: PathBuf,
    /// Bytes used by the downloaded speech recognition files.
    pub speech_files_size: u64,
    sched: Arc<Scheduler>,
    /// The unlimited runner; only ever used wrapped in a [`Limited`].
    runner: Arc<dyn Runner>,
    dialogs: Box<dyn Dialogs>,
    pub machine: Machine,
    probes: Pool<ProbeJob, ProbeResult>,
    probe_cancel: HashMap<MediaId, Arc<AtomicBool>>,
    proxy_pool: Pool<ProxyJob, ()>,
    proxy_events: (Sender<ProxyEvent>, Receiver<ProxyEvent>),
    proxy_jobs: HashMap<MediaId, (String, Arc<AtomicBool>)>,
    /// Where proxies are kept (`None`: no proxies).
    pub proxy_cache: Option<ProxyCache>,
    /// Bytes used by the proxy cache (refreshed when it changes).
    pub proxy_cache_size: u64,
    pub preview: Preview,
    /// Latest preview frame and a counter that changes with it.
    pub frame: Option<Frame>,
    pub frame_serial: u64,
    export: Option<ExportHandle>,
    pub export_status: ExportStatus,
    /// Video hardware for "Export: Faster".
    pub hw: HwState,
    hw_probe: Option<RunningProbe>,
    hw_generation: u64,
    /// Describes this computer for the hardware check; replaced in tests.
    pub platform: fn() -> Platform,
    /// Time limits of the hardware check.
    pub probe_limits: ProbeLimits,
    /// No progress for this long stops a hardware export (tests shorten it).
    pub stall_timeout: Duration,
    stt: Option<SttHandle>,
    /// Subtitle generation progress (0..=1) while it runs.
    pub stt_progress: Option<f32>,
    /// Where the speech model is installed (`None`: generation unavailable).
    pub model_store: Option<ModelStore>,
    model_install: Option<(Receiver<ModelEvent>, Arc<AtomicBool>)>,
    /// Model download progress (0..=1) while it runs.
    pub model_progress: Option<f32>,
    pub transcriber: TranscriberFactory,
    pub fetcher: Option<Arc<dyn ModelFetcher>>,
    /// Projects the OS asked to open (macOS Apple Events), drained every tick.
    pub open_requests: fn() -> Vec<PathBuf>,
    /// Whether this copy added itself to the Linux application menu.
    pub desktop_integrated: bool,
    pub modal: Option<Modal>,
    /// The window may close (nothing unsaved, or the user chose).
    pub close_allowed: bool,
    /// The UI must close the window.
    pub quit_requested: bool,
    /// One-line status message.
    pub status: String,
    /// Message shown in a modal dialog until dismissed.
    pub notice: Option<String>,
    /// Non-blocking message at the top of the window.
    pub banner: Option<Banner>,
}

fn check_ffmpeg(runner: &dyn Runner, settings: &Settings) -> Result<FfmpegInfo, String> {
    let located = ffmpeg::locate_both(settings.ffmpeg.as_deref(), settings.ffprobe.as_deref())
        .map_err(|e| e.to_string())?;
    ffmpeg::check_located(runner, &located).map_err(|e| e.to_string())
}

fn probe_pool(sched: &Arc<Scheduler>, runner: &Arc<dyn Runner>) -> Pool<ProbeJob, ProbeResult> {
    let limited = sched.runner(Priority::Probe, Arc::clone(runner));
    // As many workers as the highest cap: the scheduler decides how many actually run.
    Pool::new(MAX_CAP as usize, move |j: ProbeJob| {
        if j.cancel.load(Ordering::SeqCst) {
            return None;
        }
        let r = limited.cancellable(Arc::clone(&j.cancel));
        let result = media::probe(&r, &j.ffprobe, &j.path, j.id).map_err(|e| e.to_string());
        (!j.cancel.load(Ordering::SeqCst)).then_some((j.id, result))
    })
}

fn proxy_pool() -> Pool<ProxyJob, ()> {
    Pool::new(1, |j: ProxyJob| {
        if let Some(ev) = j.worker.generate(&j.task, &j.events) {
            let _ = j.events.send(ev);
        }
        None
    })
}

fn no_open_requests() -> Vec<PathBuf> {
    Vec::new()
}

/// The machine of [`AppState::with_ffmpeg`]: 16 logical / 8 physical cores, 64 GiB.
pub fn test_machine() -> Machine {
    Machine::simulated(16, 8, 64 * GIB)
}

/// Start-up settings: reads the file at `path` (if any), decides with
/// [`settings::startup`], keeps a copy of an unreadable file and writes the result when
/// needed. Returns the settings and the message to show.
pub fn start_settings(path: Option<&Path>, machine: &Machine) -> (Settings, Option<StartNotice>) {
    let fp = machine.fingerprint();
    let Some(path) = path else {
        return (
            Settings::with(settings::recommend(&fp), Some(fp.clone())),
            None,
        );
    };
    let loaded = settings::load_file(path);
    let st = settings::startup(&loaded, &fp);
    if st.backup {
        settings::backup(path, &loaded);
    }
    if st.write {
        let _ = settings::save(&st.settings, path);
    }
    (st.settings, st.notice)
}

impl AppState {
    /// Real start-up: detects the machine, removes leftovers of a crash, loads (or creates)
    /// the settings, starts the ffmpeg check in the background.
    pub fn new(
        runner: Arc<dyn Runner>,
        dialogs: Box<dyn Dialogs>,
        sink: Box<dyn AudioSink>,
        settings_path: Option<PathBuf>,
    ) -> AppState {
        let machine = Machine::detect();
        let work_root = crate::scratch::default_root();
        crate::scratch::cleanup_stale(&work_root);
        let (settings, notice) = start_settings(settings_path.as_deref(), &machine);
        let mut s = AppState::with_ffmpeg(Err(CHECKING_FFMPEG.to_string()), runner, dialogs, sink);
        s.detect = Machine::detect;
        s.machine = machine;
        s.settings = settings;
        s.settings_path = settings_path;
        s.banner = notice.map(Banner::from_notice);
        s.work_root = work_root.clone();
        s.apply_budget();
        s.preview.proxies = s.settings.preview_quality != PreviewQuality::Original;
        s.proxy_cache = ProxyCache::default_dir().map(ProxyCache::new);
        s.proxy_cache_size = s.proxy_cache.as_ref().map(ProxyCache::size).unwrap_or(0);
        s.model_store = ModelStore::default_dir().map(|d| ModelStore {
            work_root: Some(work_root),
            ..ModelStore::new(d)
        });
        s.refresh_speech_files_size();
        s.transcriber = stt::default_factory();
        s.fetcher = stt::default_fetcher();
        s.open_requests = crate::desktop::take_open_requests;
        s.desktop_integrated = crate::desktop::self_integrating();
        s.platform = Platform::detect;
        s.start_ffmpeg_check();
        s
    }

    /// Start-up with an already known ffmpeg status and nothing outside the process: no
    /// proxy cache, no speech model, no OS requests (tests).
    pub fn with_ffmpeg(
        ffmpeg: Result<FfmpegInfo, String>,
        runner: Arc<dyn Runner>,
        dialogs: Box<dyn Dialogs>,
        sink: Box<dyn AudioSink>,
    ) -> AppState {
        let sched = Scheduler::new(MAX_PROCESSES);
        let config = preview_config(&ffmpeg, &sched, &runner);
        let project = Project::default();
        let machine = test_machine();
        let fp = machine.fingerprint();
        let settings = Settings::with(settings::recommend(&fp), Some(fp.clone()));
        let budget = settings::budget(settings.effort, &fp);
        AppState {
            saved: project.doc(),
            project,
            project_path: None,
            dirty: false,
            history: History::default(),
            selected_clip: None,
            selected_media: None,
            selected_music: None,
            selected_line: None,
            playhead: Micros::ZERO,
            clipboard: None,
            zoom: 0.0,
            ffmpeg,
            ffmpeg_checking: false,
            ffmpeg_check: None,
            settings,
            settings_path: None,
            settings_open: false,
            settings_summary: Vec::new(),
            budget,
            detect: test_machine,
            work_root: std::env::temp_dir().join("videolore-test-work"),
            speech_files_size: 0,
            probes: probe_pool(&sched, &runner),
            probe_cancel: HashMap::new(),
            proxy_pool: proxy_pool(),
            proxy_events: channel(),
            proxy_jobs: HashMap::new(),
            proxy_cache: None,
            proxy_cache_size: 0,
            sched,
            runner,
            dialogs,
            machine,
            preview: Preview::new(config, sink),
            frame: None,
            frame_serial: 0,
            export: None,
            export_status: ExportStatus::default(),
            hw: HwState::default(),
            hw_probe: None,
            hw_generation: 0,
            platform: Platform::none,
            probe_limits: ProbeLimits::default(),
            stall_timeout: export::STALL_TIMEOUT,
            stt: None,
            stt_progress: None,
            model_store: None,
            model_install: None,
            model_progress: None,
            transcriber: Arc::new(|_, _| Err(stt::SttError::NotAvailable)),
            fetcher: None,
            open_requests: no_open_requests,
            desktop_integrated: false,
            modal: None,
            close_allowed: false,
            quit_requested: false,
            status: String::new(),
            notice: None,
            banner: None,
        }
    }

    /// The scheduler every ffmpeg/ffprobe process goes through.
    pub fn scheduler(&self) -> &Arc<Scheduler> {
        &self.sched
    }

    fn limited(&self, priority: Priority) -> Limited {
        self.sched.runner(priority, Arc::clone(&self.runner))
    }

    fn start_ffmpeg_check(&mut self) {
        let (tx, rx) = channel();
        let runner = self.limited(Priority::Probe);
        let settings = self.settings.clone();
        std::thread::spawn(move || {
            let _ = tx.send(check_ffmpeg(&runner, &settings));
        });
        self.ffmpeg = Err(CHECKING_FFMPEG.to_string());
        self.ffmpeg_checking = true;
        self.ffmpeg_check = Some(rx);
    }

    fn set_ffmpeg(&mut self, info: Result<FfmpegInfo, String>) {
        self.ffmpeg = info;
        self.ffmpeg_checking = false;
        self.ffmpeg_check = None;
        // another ffmpeg may have other hardware methods: check again when they matter
        self.stop_hw_probe();
        self.hw.probe = HwProbe::NotChecked;
        if self.settings.export == ExportChoice::Faster {
            self.start_hw_probe();
        }
        let mut preview = Preview::new(
            preview_config(&self.ffmpeg, &self.sched, &self.runner),
            crate::preview::default_sink(),
        );
        preview.proxies = self.settings.preview_quality != PreviewQuality::Original;
        self.preview = preview;
        let now = Instant::now();
        let t = self.playhead;
        self.seek(t, now);
        let ids: Vec<MediaId> = self.project.media.iter().map(|m| m.id).collect();
        for id in ids {
            self.maybe_proxy(id);
        }
    }

    /// Starts the background check of the video hardware (needs a working ffmpeg; an
    /// older run still going is cancelled and its result ignored).
    fn start_hw_probe(&mut self) {
        self.stop_hw_probe();
        let Ok(info) = &self.ffmpeg else {
            self.hw.probe = HwProbe::NotChecked;
            return;
        };
        self.hw_generation += 1;
        let generation = self.hw_generation;
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let runner = self
            .limited(Priority::Probe)
            .cancellable(Arc::clone(&cancel));
        let ffmpeg = info.ffmpeg.clone();
        let encoders = info.encoders.clone();
        let platform = self.platform;
        let limits = self.probe_limits;
        let flag = Arc::clone(&cancel);
        std::thread::spawn(move || {
            let status = hwenc::probe_all(&runner, &ffmpeg, &encoders, &platform(), limits, &flag);
            if !flag.load(Ordering::SeqCst) {
                let _ = tx.send((generation, status));
            }
        });
        self.hw.probe = HwProbe::Checking(generation);
        self.hw_probe = Some((generation, rx, cancel));
    }

    fn stop_hw_probe(&mut self) {
        if let Some((_, _, cancel)) = self.hw_probe.take() {
            cancel.store(true, Ordering::SeqCst);
        }
    }

    /// The hardware the next export may use: only with Export = Faster, once the check
    /// has finished, and not for a format whose method had a problem (that method is
    /// tried again when the check picks a different one).
    pub fn hw_offer(&self) -> Option<HwOffer> {
        if self.settings.export != ExportChoice::Faster {
            return None;
        }
        // the output check needs the `psnr` filter
        if !self
            .ffmpeg
            .as_ref()
            .is_ok_and(|i| i.filters.contains(crate::outcheck::CHECK_FILTER))
        {
            return None;
        }
        let HwProbe::Done(status) = &self.hw.probe else {
            return None;
        };
        let allowed = |codec: Codec| {
            status
                .get(codec)
                .filter(|c| !self.settings.hardware_is_off(codec, c.vendor))
                .cloned()
        };
        let offer = HwOffer {
            h264: allowed(Codec::H264),
            hevc: allowed(Codec::Hevc),
        };
        (offer.h264.is_some() || offer.hevc.is_some()).then_some(offer)
    }

    /// The machine characteristics the recommendations are based on.
    pub fn fingerprint(&self) -> Fingerprint {
        self.machine.fingerprint()
    }

    /// The three choices are the recommended ones for this machine.
    pub fn is_recommended(&self) -> bool {
        self.settings.is_recommended(&self.fingerprint())
    }

    /// Recomputes the budget of "How hard videolore works"; new jobs use it, running jobs
    /// keep theirs.
    fn apply_budget(&mut self) {
        self.budget = settings::budget(self.settings.effort, &self.fingerprint());
        self.sched.set_cap(self.budget.max_processes);
    }

    fn save_settings(&mut self) -> Result<(), String> {
        match &self.settings_path {
            Some(p) => settings::save(&self.settings, p),
            None => Ok(()),
        }
    }

    pub fn refresh_speech_files_size(&mut self) {
        self.speech_files_size = self
            .model_store
            .as_ref()
            .map(ModelStore::downloaded_bytes)
            .unwrap_or(0);
    }

    /// Applies the Preview quality choice: proxies no longer wanted stop being used (their
    /// files stay in the cache), newly wanted ones are queued. The export never changes.
    fn apply_preview_quality(&mut self, now: Instant) {
        let q = self.settings.preview_quality;
        self.preview.proxies = q != PreviewQuality::Original;
        let unwanted: Vec<MediaId> = self
            .project
            .media
            .iter()
            .filter(|m| !proxy::wants_proxy(&m.format, q, &self.machine))
            .map(|m| m.id)
            .collect();
        for id in unwanted {
            self.cancel_proxy(id);
            if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id) {
                m.proxy = ProxyState::None;
            }
        }
        let ids: Vec<MediaId> = self.project.media.iter().map(|m| m.id).collect();
        for id in ids {
            self.maybe_proxy(id);
        }
        self.refresh_still(now);
    }

    /// Detects the machine again and applies its recommended choices. The ffmpeg choice,
    /// projects, preview copies and speech recognition files are left alone; nothing is
    /// downloaded.
    fn use_recommended(&mut self, now: Instant) -> Result<(), String> {
        self.machine = (self.detect)();
        let fp = self.fingerprint();
        let old = self.settings.choices();
        let new = settings::recommend(&fp);
        self.settings_summary = settings::summary(old, new);
        self.settings.set_choices(new);
        self.settings.machine = Some(fp);
        // the video hardware gets a fresh chance
        self.settings.hardware_off.clear();
        self.start_hw_probe();
        self.banner = None;
        self.apply_budget();
        self.apply_preview_quality(now);
        self.status = self.settings_summary.join("; ");
        self.save_settings()
    }

    /// Output frame rate (frame grid for the playhead, split and zoom).
    pub fn fps(&self) -> Fps {
        let enc = self.encoders();
        render::output_geometry(&self.project, &enc)
            .map(|g| g.fps)
            .unwrap_or(Fps { num: 30, den: 1 })
    }

    fn frame_len(&self) -> Micros {
        self.fps().frame_duration()
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

    /// Subtitle generation or model installation in progress.
    pub fn is_generating(&self) -> bool {
        self.stt.is_some() || self.model_install.is_some()
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

    /// True when `id` is on the video or music track.
    fn media_in_use(&self, id: MediaId) -> bool {
        let used = |e: &Edit| {
            e.timeline.clips.iter().any(|c| c.media == id)
                || e.music.clips.iter().any(|c| c.media == id)
        };
        used(&self.project.edit())
    }

    fn usable(&self, id: Option<MediaId>) -> Option<&MediaItem> {
        id.or(self.selected_media)
            .and_then(|id| self.project.media(id))
            .filter(|m| m.is_ready() && !m.missing)
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
            Command::Append(id) => self.usable(*id).is_some_and(MediaItem::has_video),
            Command::PlaceMusic(id) => self
                .usable(*id)
                .is_some_and(|m| !m.has_video() && m.format.has_audio()),
            Command::RemoveMedia(id) => {
                self.project.media(*id).is_some() && !self.media_in_use(*id)
            }
            Command::SplitMusic => self.project.music.at(self.playhead).is_some(),
            Command::DeleteMusic => self
                .selected_music
                .is_some_and(|id| self.project.music.get(id).is_some()),
            Command::PlayPause => has_ff && !tl.clips.is_empty(),
            Command::PickExport | Command::Export(_) => {
                has_ff && !tl.clips.is_empty() && !self.is_exporting()
            }
            Command::CancelExport => self.is_exporting(),
            Command::PickImport | Command::Import(_) => has_ff,
            Command::GenerateSubtitles => {
                has_ff && stt::available() && !tl.clips.is_empty() && !self.is_generating()
            }
            Command::CancelSubtitles => self.is_generating(),
            Command::AddSubtitle => tl.locate(self.playhead).is_some(),
            Command::ClearProxyCache => self.proxy_cache.is_some(),
            Command::RemoveSpeechFiles | Command::ConfirmRemoveSpeechFiles => {
                self.model_store.is_some() && !self.is_generating()
            }
            Command::SetSubtitleQuality(q) => {
                settings::availability(&self.fingerprint())[q.index()].is_none()
            }
            Command::RemoveDesktopIntegration => self.desktop_integrated,
            _ => true,
        }
    }

    /// Runs a command. Errors are also put in the status line.
    pub fn dispatch(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        let r = self.dispatch_inner(cmd, now);
        if let Err(e) = &r {
            self.status = e.clone();
        }
        self.refresh_dirty();
        r
    }

    fn refresh_dirty(&mut self) {
        self.dirty = self.project.doc() != self.saved;
    }

    /// Runs `action` now, or asks first when there are unsaved changes.
    fn guarded(&mut self, action: PendingAction, now: Instant) -> Result<(), String> {
        self.refresh_dirty();
        if self.dirty {
            self.modal = Some(Modal::Unsaved(action));
            return Ok(());
        }
        self.continue_action(action, now)
    }

    fn continue_action(&mut self, action: PendingAction, now: Instant) -> Result<(), String> {
        match action {
            PendingAction::New => {
                self.load_project(Project::default(), None, now);
                self.status = "New project".into();
                Ok(())
            }
            PendingAction::Open(None) => match self.dialogs.pick_project_open() {
                Some(p) => self.open_now(p, now),
                None => Ok(()),
            },
            PendingAction::Open(Some(p)) => self.open_now(p, now),
            PendingAction::Close | PendingAction::Quit => {
                self.close_allowed = true;
                self.quit_requested = true;
                Ok(())
            }
        }
    }

    fn dispatch_inner(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        match cmd {
            Command::NewProject => self.guarded(PendingAction::New, now),
            Command::PickOpenProject => self.guarded(PendingAction::Open(None), now),
            Command::OpenProject(path) => self.guarded(PendingAction::Open(Some(path)), now),
            Command::SaveProject => self.save_interactive().map(|_| ()),
            Command::PickSaveProjectAs => match self.dialogs.pick_project_save() {
                Some(p) => self.save_as(p),
                None => Ok(()),
            },
            Command::SaveProjectAs(p) => self.save_as(p),
            Command::RequestClose => self.guarded(PendingAction::Close, now),
            Command::Quit => self.guarded(PendingAction::Quit, now),
            Command::ResolveUnsaved(choice) => {
                let Some(Modal::Unsaved(action)) = self.modal.take() else {
                    return Ok(());
                };
                match choice {
                    UnsavedChoice::Cancel => Ok(()),
                    UnsavedChoice::Discard => self.continue_action(action, now),
                    UnsavedChoice::Save => {
                        if self.save_interactive()? {
                            self.continue_action(action, now)
                        } else {
                            Ok(())
                        }
                    }
                }
            }
            Command::PickImport => {
                let files = self.dialogs.pick_media();
                if files.is_empty() {
                    return Ok(());
                }
                self.dispatch_inner(Command::Import(files), now)
            }
            Command::Import(files) => self.import(files),
            Command::RemoveMedia(id) => self.remove_media(id),
            Command::SelectMedia(id) => {
                self.selected_media = id;
                Ok(())
            }
            Command::Append(id) => {
                let m = self
                    .usable(id)
                    .cloned()
                    .ok_or("select a file that is ready in the media list first")?;
                self.edit(now, |e| e.timeline.append(&m).map(|_| ()).map_err(err))
            }
            Command::SelectClip(id) => {
                self.selected_clip = id;
                Ok(())
            }
            Command::Split => {
                let min = self.frame_len();
                let at = self.playhead;
                self.edit(now, |e| e.timeline.split(at, min).map(|_| ()).map_err(err))
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
                self.edit(now, |e| e.timeline.remove(id).map(|_| ()).map_err(err))?;
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
                let min = self.frame_len();
                let at = self.playhead;
                let mut new_id = None;
                self.edit(now, |e| {
                    new_id = Some(e.timeline.insert_at(&clip, at, min).map_err(err)?);
                    Ok(())
                })?;
                self.selected_clip = new_id;
                Ok(())
            }
            Command::Merge => {
                let id = self
                    .target_clip()
                    .ok_or(EditError::NothingSelected.to_string())?;
                self.edit(now, |e| e.timeline.merge(id).map_err(err))
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
                let dur = self.duration_of(c.media)?;
                let min = self.frame_len();
                self.edit(now, |e| {
                    e.timeline
                        .trim(clip, src_in, src_out, dur, min)
                        .map_err(err)
                })
            }
            Command::MoveClip { clip, to } => {
                self.edit(now, |e| e.timeline.move_to(clip, to).map_err(err))
            }
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
                self.edit(now, |e| e.timeline.move_to(id, to).map_err(err))
            }
            Command::ToggleFade(b) => {
                self.edit(now, |e| e.timeline.toggle_fade(b).map(|_| ()).map_err(err))
            }
            Command::ToggleFadeAtPlayhead => {
                let (b, _) = self
                    .project
                    .timeline
                    .nearest_boundary(self.playhead)
                    .ok_or(EditError::NoSuchBoundary.to_string())?;
                self.edit(now, |e| e.timeline.toggle_fade(b).map(|_| ()).map_err(err))
            }
            Command::PlaceMusic(id) => {
                let m = self
                    .usable(id)
                    .cloned()
                    .ok_or("select an audio file that is ready in the media list first")?;
                let (at, min) = (self.playhead, self.frame_len());
                let mut placed = None;
                self.edit(now, |e| {
                    placed = Some(e.music.place(&m, at, min).map_err(err)?);
                    Ok(())
                })?;
                self.selected_music = placed;
                Ok(())
            }
            Command::SelectMusic(id) => {
                self.selected_music = id;
                Ok(())
            }
            Command::MoveMusic { clip, start } => {
                self.edit(now, |e| e.music.move_to(clip, start).map_err(err))
            }
            Command::TrimMusic {
                clip,
                src_in,
                src_out,
            } => {
                let c = self.project.music.get(clip).ok_or("no such music clip")?;
                let dur = self.duration_of(c.media)?;
                let min = self.frame_len();
                self.edit(now, |e| {
                    e.music.trim(clip, src_in, src_out, dur, min).map_err(err)
                })
            }
            Command::SplitMusic => {
                let (at, min) = (self.playhead, self.frame_len());
                self.edit(now, |e| e.music.split(at, min).map(|_| ()).map_err(err))
            }
            Command::DeleteMusic => {
                let id = self.selected_music.ok_or("select a music clip first")?;
                self.edit(now, |e| e.music.remove(id).map(|_| ()).map_err(err))?;
                self.selected_music = None;
                Ok(())
            }
            Command::SetMix(mut mix) => {
                mix.video_volume = mix.video_volume.min(100);
                mix.music_volume = mix.music_volume.min(100);
                if mix == self.project.mix {
                    return Ok(());
                }
                self.edit(now, |e| {
                    e.mix = mix;
                    Ok(())
                })
            }
            Command::GenerateSubtitles => {
                if !stt::available() {
                    return Err("subtitle generation is not available in this build".into());
                }
                // The speech engine is never reached on a processor it cannot run on.
                if !self.machine.speech_supported() {
                    self.notice = Some(TOO_OLD_FOR_SUBTITLES.into());
                    return Ok(());
                }
                if self.project.subtitles.has_edits() {
                    self.modal = Some(Modal::ConfirmRegenerate);
                    return Ok(());
                }
                self.generate_subtitles(None)
            }
            Command::ConfirmRegenerate => {
                self.modal = None;
                self.generate_subtitles(None)
            }
            Command::ModelConsent(choice) => {
                let level = match self.modal.take() {
                    Some(Modal::ModelConsent(l)) => l,
                    other => {
                        self.modal = other;
                        return Ok(());
                    }
                };
                let store = self
                    .model_store
                    .clone()
                    .ok_or("no folder for speech recognition files")?;
                let (level, source) = match choice {
                    ConsentChoice::Cancel => return Ok(()),
                    ConsentChoice::UseFastInstead => {
                        if !store.is_installed(SubtitleQuality::Fast) {
                            return Err("the Fast speech file is not installed".into());
                        }
                        // this generation only: the setting does not change
                        return self.generate_subtitles(Some(SubtitleQuality::Fast));
                    }
                    ConsentChoice::Download => (level, ModelSource::Download),
                    ConsentChoice::DownloadFastInstead => {
                        (SubtitleQuality::Fast, ModelSource::Download)
                    }
                    ConsentChoice::LocalFile => match self.dialogs.pick_model_file() {
                        Some(p) => (level, ModelSource::LocalFile(p)),
                        None => return Ok(()),
                    },
                };
                let cancel = Arc::new(AtomicBool::new(false));
                let rx = stt::install_model(
                    store,
                    level,
                    source,
                    self.fetcher.clone(),
                    Arc::clone(&cancel),
                );
                self.model_install = Some((rx, cancel));
                self.model_progress = Some(0.0);
                self.status = "Installing the speech recognition file…".into();
                Ok(())
            }
            Command::CancelSubtitles => {
                if let Some(h) = &self.stt {
                    h.cancel();
                }
                if let Some((_, c)) = &self.model_install {
                    c.store(true, Ordering::SeqCst);
                }
                Ok(())
            }
            Command::AddSubtitle => {
                let (at, min) = (self.playhead, self.frame_len());
                let mut added = None;
                self.edit(now, |e| {
                    added = Some(
                        e.subtitles
                            .add_at(&e.timeline, at, "New subtitle", min)
                            .map_err(err)?,
                    );
                    Ok(())
                })?;
                self.selected_line = added;
                Ok(())
            }
            Command::DeleteSubtitle(id) => {
                self.edit(now, |e| e.subtitles.delete(id).map_err(err))?;
                if self.selected_line == Some(id) {
                    self.selected_line = None;
                }
                Ok(())
            }
            Command::SetSubtitleText(id, text) => {
                let cur = self
                    .project
                    .subtitles
                    .get(id)
                    .ok_or("no such subtitle line")?;
                if cur.text == crate::subtitles::clean_text(&text) {
                    return Ok(());
                }
                self.edit(now, |e| e.subtitles.set_text(id, &text).map_err(err))
            }
            Command::RetimeSubtitle {
                line,
                clip,
                start,
                end,
            } => {
                let min = self.frame_len();
                self.edit(now, |e| {
                    e.subtitles
                        .retime(line, &e.timeline, clip, start, end, min)
                        .map_err(err)
                })
            }
            Command::SelectLine(id) => {
                self.selected_line = id;
                Ok(())
            }
            Command::SetPreset(p) => {
                if p == self.project.subtitles.preset {
                    return Ok(());
                }
                self.edit(now, |e| {
                    e.subtitles.preset = p;
                    Ok(())
                })
            }
            Command::SetLanguage(lang) => {
                if let Some(l) = &lang
                    && !(2..=3).contains(&l.len())
                {
                    return Err(format!("'{l}' is not a language code"));
                }
                if let Some(l) = &lang
                    && !l.chars().all(|c| c.is_ascii_lowercase())
                {
                    return Err(format!("'{l}' is not a language code"));
                }
                self.edit(now, |e| {
                    e.subtitles.language_overridden = lang.is_some();
                    e.subtitles.language = lang;
                    Ok(())
                })
            }
            Command::OpenSettings => {
                self.settings_open = true;
                self.settings_summary.clear();
                self.refresh_speech_files_size();
                Ok(())
            }
            Command::CloseSettings => {
                self.settings_open = false;
                Ok(())
            }
            Command::SetPreviewQuality(q) => {
                self.settings.preview_quality = q;
                self.settings_summary.clear();
                self.apply_preview_quality(now);
                self.save_settings()
            }
            Command::SetSubtitleQuality(q) => {
                let fp = self.fingerprint();
                if let Some(reason) = settings::availability(&fp)[q.index()] {
                    return Err(reason.to_string());
                }
                // only the choice changes: existing subtitles are never regenerated
                self.settings.subtitle_quality = q;
                self.settings_summary.clear();
                self.save_settings()
            }
            Command::SetEffort(e) => {
                self.settings.effort = e;
                self.settings_summary.clear();
                self.apply_budget();
                self.save_settings()
            }
            Command::SetExport(e) => {
                self.settings.export = e;
                self.settings_summary.clear();
                if e == ExportChoice::Faster && self.hw.probe == HwProbe::NotChecked {
                    self.start_hw_probe();
                }
                // a running export keeps its method; the next one uses the new choice
                self.save_settings()
            }
            Command::TryHardwareAgain => {
                self.settings.hardware_off.clear();
                self.start_hw_probe();
                self.save_settings()
            }
            Command::UseRecommended | Command::BannerAction(BannerAction::UseRecommended) => {
                self.use_recommended(now)
            }
            Command::RemoveSpeechFiles => {
                self.modal = Some(Modal::ConfirmRemoveSpeechFiles);
                Ok(())
            }
            Command::ConfirmRemoveSpeechFiles => {
                self.modal = None;
                let store = self
                    .model_store
                    .clone()
                    .ok_or("no folder for speech recognition files")?;
                let freed = store.remove_downloaded();
                self.refresh_speech_files_size();
                self.status = format!(
                    "Removed the speech recognition files ({} MB freed)",
                    freed / 1_000_000
                );
                Ok(())
            }
            Command::ResetFfmpeg => {
                self.settings.ffmpeg = None;
                self.settings.ffprobe = None;
                self.save_settings()?;
                self.dispatch_inner(Command::RetryFfmpeg, now)
            }
            Command::DismissBanner => {
                self.banner = None;
                Ok(())
            }
            Command::ClearProxyCache => {
                let cache = self.proxy_cache.clone().ok_or("no proxy cache")?;
                let busy: Vec<String> = self.proxy_jobs.values().map(|(k, _)| k.clone()).collect();
                let freed = cache.clear(&busy);
                for m in &mut self.project.media {
                    if let ProxyState::Ready(p) = &m.proxy
                        && !p.is_file()
                    {
                        m.proxy = ProxyState::None;
                    }
                }
                self.proxy_cache_size = cache.size();
                self.status = format!("Freed {} MB of proxies", freed / 1_000_000);
                self.refresh_still(now);
                Ok(())
            }
            Command::Undo => {
                let prev = self
                    .history
                    .undo(self.project.edit())
                    .ok_or(EditError::NothingToUndo.to_string())?;
                self.replace_edit(prev, now);
                Ok(())
            }
            Command::Redo => {
                let next = self
                    .history
                    .redo(self.project.edit())
                    .ok_or(EditError::NothingToRedo.to_string())?;
                self.replace_edit(next, now);
                Ok(())
            }
            Command::PlayPause => {
                if self.preview.is_playing() {
                    self.preview.pause(now);
                    self.playhead = self.preview.position(now);
                    self.refresh_still(now);
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
                if !p.is_absolute() {
                    return Err("choose ffmpeg with its full path".into());
                }
                let probe = p.with_file_name(ffmpeg::exe_name("ffprobe"));
                self.settings.ffmpeg = Some(p);
                self.settings.ffprobe = probe.is_file().then_some(probe);
                if let Some(sp) = &self.settings_path {
                    settings::save(&self.settings, sp)?;
                }
                self.dispatch_inner(Command::RetryFfmpeg, now)
            }
            Command::RetryFfmpeg => {
                self.start_ffmpeg_check();
                Ok(())
            }
            Command::ShowAbout => {
                self.modal = Some(Modal::About);
                Ok(())
            }
            Command::CloseModal => {
                self.modal = None;
                Ok(())
            }
            Command::RemoveDesktopIntegration => {
                crate::desktop::remove_integration().map_err(|e| e.to_string())?;
                self.desktop_integrated = false;
                self.status = "Removed from the application menu".into();
                Ok(())
            }
            Command::DismissNotice => {
                self.notice = None;
                Ok(())
            }
        }
    }

    fn duration_of(&self, id: MediaId) -> Result<Micros, String> {
        self.project
            .media(id)
            .map(|m| m.duration)
            .ok_or_else(|| "no such media".to_string())
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

    /// Save; asks for a path when the project was never saved. `Ok(false)`: the user
    /// cancelled the file dialog.
    fn save_interactive(&mut self) -> Result<bool, String> {
        match self.project_path.clone() {
            Some(p) => self.save_to(p).map(|_| true),
            None => match self.dialogs.pick_project_save() {
                Some(p) => self.save_as(p).map(|_| true),
                None => Ok(false),
            },
        }
    }

    fn save_as(&mut self, mut p: PathBuf) -> Result<(), String> {
        if p.extension().is_none() {
            p.set_extension(project::EXTENSION);
        }
        self.save_to(p)
    }

    fn save_to(&mut self, path: PathBuf) -> Result<(), String> {
        project::save(&self.project, &path).map_err(|e| e.to_string())?;
        self.project_path = Some(path);
        self.saved = self.project.doc();
        self.status = "Project saved".into();
        Ok(())
    }

    fn open_now(&mut self, path: PathBuf, now: Instant) -> Result<(), String> {
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

    /// Stops everything that belongs to the current project.
    fn cancel_project_jobs(&mut self) {
        for c in self.probe_cancel.values() {
            c.store(true, Ordering::SeqCst);
        }
        self.probe_cancel.clear();
        for (_, c) in self.proxy_jobs.values() {
            c.store(true, Ordering::SeqCst);
        }
        self.proxy_jobs.clear();
        // fresh pools and channels: late results of the old project go nowhere
        self.probes = probe_pool(&self.sched, &self.runner);
        self.proxy_pool = proxy_pool();
        self.proxy_events = channel();
        self.stt = None;
        self.stt_progress = None;
    }

    fn load_project(&mut self, p: Project, path: Option<PathBuf>, now: Instant) {
        self.cancel_project_jobs();
        self.preview.invalidate(now);
        self.preview.forget_frame();
        self.saved = p.doc();
        self.project = p;
        self.project_path = path;
        self.history.clear();
        self.selected_clip = None;
        self.selected_media = None;
        self.selected_music = None;
        self.selected_line = None;
        self.clipboard = None;
        self.frame = None;
        self.zoom = 0.0;
        self.close_allowed = false;
        let ids: Vec<MediaId> = self.project.media.iter().map(|m| m.id).collect();
        for id in ids {
            self.maybe_proxy(id);
        }
        self.seek(Micros::ZERO, now);
        self.refresh_dirty();
    }

    /// Adds the files to the list at once ("analysing") and probes them in the background.
    fn import(&mut self, files: Vec<PathBuf>) -> Result<(), String> {
        let info = self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        let ffprobe = info.ffprobe.clone();
        let n = files.len();
        for path in files {
            let id = self.project.new_media_id();
            self.project.add_media(MediaItem::analysing(id, &path));
            let cancel = Arc::new(AtomicBool::new(false));
            self.probe_cancel.insert(id, Arc::clone(&cancel));
            self.probes.submit(ProbeJob {
                id,
                path,
                ffprobe: ffprobe.clone(),
                cancel,
            });
        }
        self.status = format!("Analysing {n} file(s)…");
        Ok(())
    }

    fn remove_media(&mut self, id: MediaId) -> Result<(), String> {
        let m = self.project.media(id).ok_or("no such media")?;
        if self.media_in_use(id) {
            return Err(format!("'{}' is used on the timeline", m.name));
        }
        let name = m.name.clone();
        if let Some(c) = self.probe_cancel.remove(&id) {
            c.store(true, Ordering::SeqCst);
        }
        self.cancel_proxy(id);
        self.project.media.retain(|m| m.id != id);
        // Subtitle lines of the file go too; an undo step must never bring back a clip
        // whose file is gone, so the history is dropped when it refers to the file.
        self.project.subtitles.lines.retain(|l| l.media != id);
        if self.history.any(|e| {
            e.timeline.clips.iter().any(|c| c.media == id)
                || e.music.clips.iter().any(|c| c.media == id)
                || e.subtitles.lines.iter().any(|l| l.media == id)
        }) {
            self.history.clear();
        }
        if self.selected_media == Some(id) {
            self.selected_media = None;
        }
        if self.clipboard.is_some_and(|c| c.media == id) {
            self.clipboard = None;
        }
        self.status = format!("Removed {name}");
        Ok(())
    }

    /// Queues a proxy for `id` when the source is big for this machine.
    fn maybe_proxy(&mut self, id: MediaId) {
        if self.proxy_jobs.contains_key(&id) {
            return;
        }
        let (Some(cache), Ok(info)) = (&self.proxy_cache, &self.ffmpeg) else {
            return;
        };
        let Some(m) = self.project.media(id) else {
            return;
        };
        let wanted = m.is_ready()
            && !m.missing
            && m.has_video()
            && matches!(m.proxy, ProxyState::None)
            && proxy::wants_proxy(&m.format, self.settings.preview_quality, &self.machine);
        if !wanted {
            return;
        }
        let Ok(key) = proxy::key_of(&m.path) else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let job = ProxyJob {
            worker: ProxyWorker {
                runner: self.limited(Priority::Background),
                ffmpeg: info.ffmpeg.clone(),
                version: info.version,
                cache: cache.clone(),
                threads: self.budget.threads,
                work_root: Some(self.work_root.clone()),
            },
            task: ProxyTask {
                media: id,
                source: m.path.clone(),
                key: key.clone(),
                format: m.format.clone(),
                duration: m.duration,
                cancel: Arc::clone(&cancel),
            },
            events: self.proxy_events.0.clone(),
        };
        self.proxy_jobs.insert(id, (key, cancel));
        self.proxy_pool.submit(job);
        if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id) {
            m.proxy = ProxyState::Queued;
        }
    }

    fn cancel_proxy(&mut self, id: MediaId) {
        if let Some((_, c)) = self.proxy_jobs.remove(&id) {
            c.store(true, Ordering::SeqCst);
            if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id)
                && matches!(m.proxy, ProxyState::Queued | ProxyState::Generating(_))
            {
                m.proxy = ProxyState::None;
            }
        }
    }

    /// Starts a generation at `level` (default: the Subtitle quality setting), asking for
    /// consent first when its speech recognition file is not installed.
    fn generate_subtitles(&mut self, level: Option<SubtitleQuality>) -> Result<(), String> {
        if self.stt.is_some() {
            return Err("subtitles are already being generated".into());
        }
        if !self.machine.speech_supported() {
            self.notice = Some(TOO_OLD_FOR_SUBTITLES.into());
            return Ok(());
        }
        let info = self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        let store = self
            .model_store
            .clone()
            .ok_or("no folder for speech recognition files")?;
        let mut level = level.unwrap_or(self.settings.subtitle_quality);
        if settings::availability(&self.fingerprint())[level.index()].is_some() {
            level = SubtitleQuality::Fast;
        }
        if !store.is_installed(level) {
            self.modal = Some(Modal::ModelConsent(level));
            return Ok(());
        }
        let mut sources: Vec<(MediaId, PathBuf)> = Vec::new();
        for c in &self.project.timeline.clips {
            if sources.iter().any(|(id, _)| *id == c.media) {
                continue;
            }
            if let Some(m) = self.project.media(c.media)
                && m.format.has_audio()
                && !m.missing
            {
                sources.push((m.id, m.path.clone()));
            }
        }
        if sources.is_empty() {
            return Err("the clips on the timeline have no sound to transcribe".into());
        }
        let subs = &self.project.subtitles;
        let job = SttJob {
            sources,
            language: subs.language.clone().filter(|_| subs.language_overridden),
            ffmpeg: info.ffmpeg.clone(),
            model: store.path(level),
            threads: self.budget.stt_threads,
            work_root: self.work_root.clone(),
            model_check: Some(store.spec(level).clone()),
        };
        self.stt = Some(stt::start(
            job,
            self.limited(Priority::Background),
            Arc::clone(&self.transcriber),
        ));
        self.stt_progress = Some(0.0);
        self.status = "Generating subtitles…".into();
        Ok(())
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
            can_burn_subtitles: info.has_subtitles(),
            threads: self.budget.threads,
            work_root: self.work_root.clone(),
            hw: self.hw_offer(),
            stall_timeout: self.stall_timeout,
            check_timeout: export::CHECK_TIMEOUT,
        };
        let runner: Arc<dyn Runner> = Arc::new(self.limited(Priority::Export));
        self.export = Some(export::start(job, runner));
        self.export_status = ExportStatus {
            running: true,
            progress: 0.0,
            stream_copy: None,
        };
        self.status = "Exporting…".into();
        Ok(())
    }

    /// Applies an edit to a copy of the editable state; on success records undo and
    /// refreshes.
    fn edit(
        &mut self,
        now: Instant,
        f: impl FnOnce(&mut Edit) -> Result<(), String>,
    ) -> Result<(), String> {
        let before = self.project.edit();
        let mut e = before.clone();
        f(&mut e)?;
        self.history.record(before);
        self.replace_edit(e, now);
        Ok(())
    }

    fn replace_edit(&mut self, e: Edit, now: Instant) {
        self.project.set_edit(e);
        if let Some(id) = self.selected_clip
            && self.project.timeline.get(id).is_none()
        {
            self.selected_clip = None;
        }
        if let Some(id) = self.selected_music
            && self.project.music.get(id).is_none()
        {
            self.selected_music = None;
        }
        if let Some(id) = self.selected_line
            && self.project.subtitles.get(id).is_none()
        {
            self.selected_line = None;
        }
        if self.preview.is_playing() {
            self.preview.invalidate(now);
            self.playhead = self.preview.position(now);
        }
        let t = self.playhead;
        self.seek(t, now);
    }

    fn seek(&mut self, t: Micros, now: Instant) {
        let dur = self.project.timeline.duration();
        let t = self.fps().snap(t.clamp(Micros::ZERO, dur)).min(dur);
        self.playhead = t;
        if self.project.timeline.clips.is_empty() {
            self.preview.invalidate(now);
            self.preview.forget_frame();
            self.frame = None;
            return;
        }
        if self.ffmpeg.is_ok() {
            let _ = self.preview.seek(&self.project, t, now);
        }
    }

    /// Asks for the picture at the playhead again (it is rendered only if it changed).
    fn refresh_still(&mut self, now: Instant) {
        if !self.preview.is_playing()
            && !self.project.timeline.clips.is_empty()
            && self.ffmpeg.is_ok()
        {
            let t = self.playhead;
            let _ = self.preview.request_still(&self.project, t, now);
        }
    }

    /// Per-frame housekeeping: background results, preview frames, playhead, export.
    pub fn tick(&mut self, now: Instant) {
        if let Some(b) = &mut self.banner
            && let Some(life) = b.lifetime
        {
            let until = *b.until.get_or_insert(now + life);
            if now >= until {
                self.banner = None;
            }
        }
        if let Some(rx) = &self.ffmpeg_check
            && let Ok(r) = rx.try_recv()
        {
            self.set_ffmpeg(r);
        }
        if let Some((generation, rx, _)) = &self.hw_probe
            && let Ok((g, status)) = rx.try_recv()
        {
            if g == *generation && self.hw.probe == HwProbe::Checking(g) {
                self.hw.probe = HwProbe::Done(status);
            }
            self.hw_probe = None;
        }
        for p in (self.open_requests)() {
            let _ = self.dispatch(Command::OpenProject(p), now);
        }
        self.tick_probes();
        self.tick_proxies(now);
        self.tick_stt(now);
        self.tick_model();
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
                ExportEvent::Done { path, method } => {
                    // a constant-quality file that came out too big is about the video, not a
                    // fault of the hardware: nothing is remembered
                    if let ExportMethod::SoftwareAfterProblem {
                        codec,
                        vendor,
                        problem,
                    } = &method
                        && !matches!(
                            problem,
                            export::Problem::CheckFailed(crate::outcheck::CheckFailure::TooLarge(
                                _
                            ))
                        )
                    {
                        self.settings
                            .turn_hardware_off(*codec, *vendor, env!("CARGO_PKG_VERSION"));
                        let _ = self.save_settings();
                    }
                    self.hw.last_method = Some(method);
                    self.finish_export(format!("Exported to {}", path.display()));
                }
                ExportEvent::Failed(m) => {
                    self.notice = Some(format!("Export failed: {m}"));
                    self.finish_export("Export failed".into());
                }
                ExportEvent::Cancelled => self.finish_export("Export cancelled".into()),
            }
        }
    }

    fn tick_probes(&mut self) {
        let results = self.probes.results();
        if results.is_empty() {
            return;
        }
        let mut rejected = Vec::new();
        for (id, r) in results {
            if self.probe_cancel.remove(&id).is_none() {
                continue; // removed meanwhile
            }
            let Some(slot) = self.project.media.iter_mut().find(|m| m.id == id) else {
                continue;
            };
            match r {
                Ok(item) => {
                    *slot = item;
                    if self.selected_media.is_none() {
                        self.selected_media = Some(id);
                    }
                    self.maybe_proxy(id);
                }
                Err(e) => {
                    slot.status = MediaStatus::Rejected(e.clone());
                    rejected.push(e);
                }
            }
        }
        if self.probe_cancel.is_empty() {
            self.status = if rejected.is_empty() {
                "Import finished".into()
            } else {
                format!("Some files could not be imported: {}", rejected.join("; "))
            };
        }
        self.refresh_dirty();
    }

    fn tick_proxies(&mut self, now: Instant) {
        let events: Vec<ProxyEvent> = self.proxy_events.1.try_iter().collect();
        let mut changed = false;
        for ev in events {
            let (id, state) = match ev {
                ProxyEvent::Progress(id, p) => (id, ProxyState::Generating(p)),
                ProxyEvent::Ready(id, p) => (id, ProxyState::Ready(p)),
                ProxyEvent::Failed(id, e) => (id, ProxyState::Failed(e)),
            };
            if !self.proxy_jobs.contains_key(&id) {
                continue; // cancelled
            }
            if !matches!(state, ProxyState::Generating(_)) {
                self.proxy_jobs.remove(&id);
                changed = true;
            }
            if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id) {
                m.proxy = state;
            }
        }
        if changed {
            self.proxy_cache_size = self.proxy_cache.as_ref().map(ProxyCache::size).unwrap_or(0);
            self.refresh_still(now);
        }
    }

    fn tick_stt(&mut self, now: Instant) {
        let Some(h) = &self.stt else { return };
        let events: Vec<SttEvent> = h.rx.try_iter().collect();
        for ev in events {
            match ev {
                SttEvent::Progress(p) => self.stt_progress = Some(p),
                SttEvent::Done { language, lines } => {
                    self.stt = None;
                    self.stt_progress = None;
                    let n = lines.len();
                    let _ = self.edit(now, |e| {
                        e.subtitles.replace_all(lines, language);
                        Ok(())
                    });
                    self.refresh_dirty();
                    self.status = format!("Generated {n} subtitle line(s)");
                }
                SttEvent::Failed(m) => {
                    self.stt = None;
                    self.stt_progress = None;
                    self.notice = Some(format!("Subtitles could not be generated: {m}"));
                    self.status = "Subtitle generation failed".into();
                }
                SttEvent::Cancelled => {
                    self.stt = None;
                    self.stt_progress = None;
                    self.status = "Subtitle generation cancelled".into();
                }
            }
        }
    }

    fn tick_model(&mut self) {
        let Some((rx, _)) = &self.model_install else {
            return;
        };
        let events: Vec<ModelEvent> = rx.try_iter().collect();
        for ev in events {
            match ev {
                ModelEvent::Progress(p) => self.model_progress = Some(p),
                ModelEvent::Installed(level, _) => {
                    self.model_install = None;
                    self.model_progress = None;
                    self.refresh_speech_files_size();
                    self.status = "Speech recognition file installed".into();
                    if let Err(e) = self.generate_subtitles(Some(level)) {
                        self.status = e;
                    }
                }
                ModelEvent::Failed(m) => {
                    self.model_install = None;
                    self.model_progress = None;
                    self.notice = Some(format!(
                        "The speech recognition file could not be installed: {m}"
                    ));
                }
            }
        }
    }

    fn finish_export(&mut self, status: String) {
        self.export = None;
        self.export_status.running = false;
        self.status = status;
    }

    /// Window title: project name, a dot when there are unsaved changes.
    pub fn title(&self) -> String {
        let name = self
            .project_path
            .as_deref()
            .map(media::display_name)
            .unwrap_or_else(|| "Untitled".into());
        format!("{name}{} — videolore", if self.dirty { " •" } else { "" })
    }

    /// True while background work may change what is shown (repaint soon).
    pub fn busy(&self) -> bool {
        self.preview.busy()
            || self.is_exporting()
            || self.is_generating()
            || self.ffmpeg_checking
            || self.hw_probe.is_some()
            || !self.probe_cancel.is_empty()
            || !self.proxy_jobs.is_empty()
    }

    /// Waits for a running export (tests and shutdown).
    pub fn wait_export(&mut self, now: Instant) {
        if let Some(h) = self.export.take() {
            let events = h.join();
            let (tx, rx) = channel();
            for e in events {
                let _ = tx.send(e);
            }
            self.export = Some(ExportHandle::from_events(rx));
            self.tick(now);
        }
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn preview_config(
    ffmpeg: &Result<FfmpegInfo, String>,
    sched: &Arc<Scheduler>,
    runner: &Arc<dyn Runner>,
) -> Option<PreviewConfig> {
    ffmpeg.as_ref().ok().map(|i| PreviewConfig {
        runner: sched.runner(Priority::Preview, Arc::clone(runner)),
        ffmpeg: i.ffmpeg.clone(),
        version: i.version,
        encoders: i.encoders.clone(),
    })
}

#[cfg(test)]
mod hw_tests;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
pub(crate) mod tests;
