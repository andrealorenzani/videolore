// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Application state and the single command dispatcher: every button, menu entry and
//! shortcut ends up in [`AppState::dispatch`]. Slow work (ffprobe, proxies, speech
//! recognition, the ffmpeg check, export) runs on other threads; [`AppState::tick`]
//! collects the results once per frame.

use crate::base::pos::Pos;
use crate::export::{self, ExportEvent, ExportHandle, ExportJob, ExportMethod};
use crate::ffmpeg::{self, FfmpegInfo, Runner};
use crate::frame::{FrameAspect, Scale};
use crate::history::History;
use crate::hwdec;
use crate::hwenc::{self, Codec, HwOffer, HwStatus, Platform, ProbeLimits};
use crate::jobs::{Limited, MAX_CAP, MAX_PROCESSES, Pool, Priority, Scheduler};
use crate::machine::{Fingerprint, GIB, Machine};
use crate::media::{self, MediaId, MediaItem, MediaStatus, ProxyState};
use crate::music::{Mix, MusicClipId};
use crate::preview::{AudioSink, Frame, Preview, PreviewConfig};
use crate::project::{self, Edit, Project, ProjectDoc};
use crate::proxy::{self, ProxyCache, ProxyEvent, ProxyTask, ProxyWorker};
use crate::render;
use crate::selection::ObjRef;
use crate::settings::interface::{Section, TextSize, Theme, Window};
use crate::settings::{
    self, Budget, Effort, ExportChoice, PreviewQuality, SavedLayout, Settings, StartNotice,
    SubtitleQuality,
};
use crate::splash;
use crate::stt::{
    self, ModelEvent, ModelFetcher, ModelSource, ModelStore, SttEvent, SttHandle, SttJob,
    TranscriberFactory,
};
use crate::subtitles::{LineId, Preset};
use crate::time::{Fps, Micros};
use crate::timeline::{Clip, ClipId, EditError, FrameTarget};
pub use applied::{Applied, AppliedRow, RowMove};
pub use command::{Command, ZOOM_STEP};
pub use effect_kind::{EffectKind, EffectSpec, Target};
pub use sphere::{Axis, LiveView, ResizeDrag};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// Result of one area of the dispatcher: the command was run, or it is not of that area.
pub(super) enum Routed {
    Done(Result<(), String>),
    Pass(Command),
}

/// One area of [`AppState::dispatch`]: it runs the commands of its area (`Routed::Done`)
/// and hands every other one back (`Routed::Pass`). The last argument is the undo name.
type DispatchArea = fn(&mut AppState, Command, Instant, &'static str) -> Routed;

/// The areas, in the order they see a command (each lives in `app/dispatch_<area>.rs`); a
/// new command is one arm in the module of its area.
const DISPATCH_AREAS: [DispatchArea; 8] = [
    AppState::dispatch_project,
    AppState::dispatch_subtitles,
    AppState::dispatch_settings,
    AppState::dispatch_interface,
    AppState::dispatch_playback,
    AppState::dispatch_view,
    AppState::dispatch_effects,
    AppState::dispatch_objects,
];

/// The text Copy and Cut put on the system clipboard (the clip stays inside the program).
pub const CLIPBOARD_MARKER: &str = "videolore clip";

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
    /// Reuse the video's subtitles without matching their words to the speech (no
    /// download; words spread evenly). Offered only when subtitles are being reused.
    UseWithoutMatching,
    Cancel,
}

/// The very start or the very end of the video.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    Start,
    End,
}

/// The edge of a text box that is dragged (REQ-159, REQ-161); the opposite one stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextEdge {
    Left,
    Right,
    Bottom,
}

/// Answer to "This video already contains subtitles…".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceChoice {
    /// Use the subtitle track with this label (index in the dialog's list).
    Use(usize),
    Recognise,
    Cancel,
}

/// A subtitle creation waiting for an answer or a background check.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreateRequest {
    /// `None`: the whole video.
    pub clip: Option<ClipId>,
    /// Replacing hand-edited lines was confirmed.
    pub confirmed: bool,
    /// The user's choice in the subtitle source dialog (`None`: not asked yet).
    pub source: Option<SourceChoice>,
    /// The labels of the subtitle tracks offered (dialog list).
    pub labels: Vec<String>,
    /// Reuse the tracks without matching words to the speech.
    pub spread: bool,
    /// Speech recognition level for this creation only (`None`: the setting).
    pub level: Option<SubtitleQuality>,
    /// An imported file whose cues are re-timed from the speech (REQ-184), instead of a
    /// creation.
    pub import: Option<subs_import::ImportData>,
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
    /// An autosaved copy newer than its project was found at start: Recover, Discard or
    /// Cancel (REQ-190); the copies are in `AppState::recoveries`.
    Recover,
    /// Import of a subtitle file: the question about re-timing and replacing (REQ-183,
    /// REQ-184); the data is in `AppState::pending_import`.
    ImportSubtitles,
    /// "This video already contains subtitles": the track labels to choose from, and
    /// whether this computer can match their words to the speech.
    SubtitleSource {
        labels: Vec<String>,
        can_match: bool,
    },
    /// Confirmation before removing the downloaded speech recognition files.
    ConfirmRemoveSpeechFiles,
    /// The Export dialog (REQ-118): the choices being made.
    Export(Box<export::choice::ExportChoice>),
    /// A LUT file of the project is missing at export (REQ-292): Find, Export without the
    /// LUT, or Cancel.
    MissingLut(Box<export::choice::ExportChoice>),
    /// The file of the export exists: asks before replacing it.
    ConfirmOverwrite(Box<export::choice::ExportChoice>),
    /// An edit would remove this many overlaps (REQ-329): OK does the edit and removes them,
    /// Cancel changes nothing; the edit waits in `AppState::pending_removal`.
    RemoveOverlaps(usize),
    About,
}

/// A floating window of the editor. None of them blocks editing or playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Popup {
    Settings,
    Fades,
    Subtitles,
    Volume,
    CleanAudio,
    ClipDetails,
    /// The settings of one fade or one subtitle line ([`Popups::instance`]).
    Instance,
    Shortcuts,
    /// A new text, or the settings of one text item ([`Popups::text`]).
    Text,
    /// The splash screen shown at the start of the editor (1.6.0, REQ-133).
    Splash,
}

impl Popup {
    pub const ALL: [Popup; 10] = [
        Popup::Settings,
        Popup::Fades,
        Popup::Subtitles,
        Popup::Volume,
        Popup::CleanAudio,
        Popup::ClipDetails,
        Popup::Instance,
        Popup::Shortcuts,
        Popup::Text,
        Popup::Splash,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Popup::Settings => settings::SETTINGS_TITLE,
            Popup::Fades => "Fade IN/OUT",
            Popup::Subtitles => "Subtitles",
            Popup::Volume => "Volume",
            Popup::CleanAudio => "Clean audio",
            Popup::ClipDetails => "Clip details",
            Popup::Instance => "Selected effect",
            Popup::Shortcuts => "Keyboard shortcuts",
            Popup::Text => "Text",
            Popup::Splash => "videolore",
        }
    }
}

/// One fade or one subtitle line on the timeline (what a marker stands for).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instance {
    /// The fade between these two adjacent clips.
    Fade(ClipId, ClipId),
    /// A subtitle line as shown in this clip.
    Line(LineId, ClipId),
    /// The fade from black at the very start of the video.
    FadeStart,
    /// The fade to black at the very end of the video.
    FadeEnd,
    /// The sound level of one video clip.
    ClipVolume(ClipId),
    /// Clean audio of one video clip.
    CleanAudio(ClipId),
    /// A point of view of a 360 source, at this source time.
    Pov(MediaId, Micros),
    /// Change aspect of the whole video (1.5.0).
    Aspect,
    /// Scale of one clip (1.5.0); always `FrameTarget::Clip` since 2.2.0: a whole-video
    /// Scale is a copy on each clip (REQ-148).
    Scale(FrameTarget),
    /// Move of one clip (1.5.0); always `FrameTarget::Clip` since 2.2.0.
    Move(FrameTarget),
    /// Apply LUT of a clip, or its file chooser for every clip (`Whole`, until a file is
    /// chosen, then the popup follows the copy on the clip under the playhead) (1.6.0).
    Lut(FrameTarget),
    /// Change speed of one clip, or its chooser for every clip (`Whole`) (2.3.0, REQ-281).
    Speed(FrameTarget),
}

/// Scale or Move, as the commands of the keyframes name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameFx {
    Scale,
    Move,
}

/// A value of Scale or Move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameValue {
    Scale(Scale),
    Move(Pos),
}

/// A Scale or Move value being set for `target` at the playhead (or the selected key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDrag {
    pub target: FrameTarget,
    pub value: FrameValue,
}

/// The open floating windows, most recently opened last.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Popups {
    pub open: Vec<Popup>,
    /// The fade or line shown by [`Popup::Instance`].
    pub instance: Option<Instance>,
    /// Overlapping markers the user must choose from.
    pub chooser: Option<Vec<Instance>>,
    /// The text item shown by [`Popup::Text`]; `None` = a new text.
    pub text: Option<crate::text::TextId>,
    /// What each effect popup was last applied to (shown in its title).
    pub targets: Vec<(Popup, Target)>,
    /// The keyframe selected in the Scale or Move popup (its key time).
    pub key: Option<Micros>,
}

impl Popups {
    pub fn is_open(&self, p: Popup) -> bool {
        self.open.contains(&p)
    }

    /// Opens `p` (or brings it forward when already open).
    pub fn show(&mut self, p: Popup) {
        self.open.retain(|x| *x != p);
        self.open.push(p);
    }

    pub fn close(&mut self, p: Popup) {
        self.open.retain(|x| *x != p);
        self.targets.retain(|(x, _)| *x != p);
        if p == Popup::Instance {
            self.instance = None;
        }
        if p == Popup::Text {
            self.text = None;
        }
    }

    /// What popup `p` shows the effect of, when it was opened by applying it.
    pub fn target(&self, p: Popup) -> Option<Target> {
        self.targets.iter().find(|(x, _)| *x == p).map(|(_, t)| *t)
    }

    fn set_target(&mut self, p: Popup, t: Target) {
        self.targets.retain(|(x, _)| *x != p);
        self.targets.push((p, t));
    }
}

/// Background work shown in the status bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub label: String,
    /// 0..=1 when known.
    pub progress: Option<f32>,
    /// The command that stops it, when it can be stopped.
    pub cancel: Option<Command>,
}

/// What dropping files on the window does (REQ-081).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropPlan {
    Nothing,
    /// Open this project; the number of other dropped files that are ignored.
    Open(PathBuf, usize),
    /// Import these files as media.
    Import(Vec<PathBuf>),
}

/// The first dropped project wins (everything else is ignored); without a project every
/// file is imported.
pub fn drop_plan(paths: Vec<PathBuf>) -> DropPlan {
    let is_project = |p: &PathBuf| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(project::EXTENSION))
    };
    match paths.iter().position(is_project) {
        Some(i) => {
            let ignored = paths.len() - 1;
            DropPlan::Open(paths[i].clone(), ignored)
        }
        None if paths.is_empty() => DropPlan::Nothing,
        None => DropPlan::Import(paths),
    }
}

/// Native file dialogs (faked in tests).
pub trait Dialogs {
    fn pick_media(&mut self) -> Vec<PathBuf>;
    fn pick_project_open(&mut self) -> Option<PathBuf>;
    fn pick_project_save(&mut self) -> Option<PathBuf>;
    fn pick_export(&mut self, extension: &str) -> Option<PathBuf>;
    fn pick_ffmpeg(&mut self) -> Option<PathBuf>;
    /// A folder, starting at `start`.
    fn pick_folder(&mut self, start: &Path) -> Option<PathBuf>;
    fn pick_model_file(&mut self) -> Option<PathBuf>;
    /// A LUT file (`.cube`).
    fn pick_lut(&mut self) -> Option<PathBuf>;
    /// Where to save the subtitles (`.srt`), proposing `default_name`.
    fn pick_subtitles_save(&mut self, default_name: &str) -> Option<PathBuf>;
    /// A `.srt` or `.vtt` file to import (REQ-183).
    fn pick_subtitles_open(&mut self) -> Option<PathBuf> {
        None
    }
    /// A PNG, GIF or video file to show over a clip (REQ-325).
    fn pick_picture(&mut self) -> Option<PathBuf> {
        None
    }
}

/// State of a running or finished export, for the UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportStatus {
    pub running: bool,
    pub progress: f32,
    pub stream_copy: Option<bool>,
}

/// What a later check of a file found: its subtitle tracks (empty when the check failed)
/// and whether it carries spherical information.
/// One result of the check of a file opened with a project: its subtitle tracks and
/// spherical flag, and the whole analysis when the file changed since the last one.
struct TrackCheck {
    id: MediaId,
    tracks: Vec<media::SubtitleTrack>,
    spherical: bool,
    probed: Option<MediaItem>,
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
    /// The hardware decoders the check found working (REQ-214).
    pub decoders: hwdec::Working,
    /// How the last export of this session was made.
    pub last_method: Option<ExportMethod>,
}

/// A hardware check in the background: its number, where its result arrives, its cancel
/// flag.
type RunningProbe = (
    u64,
    Receiver<(u64, HwStatus, hwdec::Working)>,
    Arc<AtomicBool>,
);

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
    /// Selected subtitle lines and text items (Ctrl/Cmd/Shift+click, Shift+drag): moved
    /// together. Not part of the project or of undo.
    pub selection: crate::selection::Selection,
    /// Lines being dragged in the preview and where (shown at once, saved on release).
    pub drag_pos: Option<(Vec<LineId>, Pos)>,
    /// A Scale or Move value being dragged in the preview (shown at once, saved on release).
    pub frame_drag: Option<FrameDrag>,
    /// The keyframes of the copied clip (REQ-130: a paste keeps them).
    pub clipboard_frames: Option<crate::timeline::ClipFrames>,
    /// The LUT of the copied clip (1.6.0).
    pub clipboard_lut: Option<String>,
    /// The own camera path of the copied clip (REQ-268: a paste gets a copy of it).
    pub clipboard_camera: Option<(MediaId, Vec<crate::camera::Pov>)>,
    /// Lines whose corner is being dragged in the preview and their new size.
    pub resize_drag: Option<ResizeDrag>,
    /// The camera of each 360 source you looked through (window state: not saved, not
    /// in undo). A moved camera without POVs stays; with POVs a drag overrides the path
    /// until the next seek, play or Reset.
    pub live: HashMap<MediaId, LiveView>,
    /// When playback restarts with the dragged camera (a short quiet time after the drag).
    camera_restart: Option<Instant>,
    /// A subtitle creation waiting for an answer or a background check.
    pub pending_create: Option<CreateRequest>,
    /// The clip the running creation is for (`None`: the whole video).
    creating_for: Option<Option<ClipId>>,
    /// A subtitle file read and waiting for the import dialog's answer (REQ-183).
    pub pending_import: Option<subs_import::ImportDraft>,
    /// The running creation is an import that re-times from the speech (REQ-184).
    creating_import: bool,
    /// The subtitle files of the same name beside each video (REQ-185).
    pub import_offers: std::collections::BTreeMap<MediaId, PathBuf>,
    /// The running creation matches subtitle tracks to the speech.
    pub stt_matching: bool,
    /// Media whose subtitle tracks are being checked (media of projects before 1.2.0).
    pub tracks_checking: std::collections::BTreeSet<MediaId>,
    track_check: Option<(Receiver<TrackCheck>, Arc<AtomicBool>)>,
    /// Media whose file changed since its last analysis (size or modification time) and
    /// that are analysed again (REQ-186).
    changed_media: std::collections::BTreeSet<MediaId>,
    /// The names of the files analysed again, for the status line.
    reanalysed: Vec<String>,
    /// The Scale or Move keyframes of the last "Copy keyframes" (REQ-204).
    pub clipboard_keys: Option<crate::timeline::KeysClip>,
    /// Text the window puts on the system clipboard after Copy or Cut, so that Ctrl/Cmd+V
    /// always reaches the program even when nothing else was copied (REQ-195): the clip
    /// itself is kept inside the program.
    pub clipboard_text: Option<String>,
    /// Where the autosave copies are kept (`None`: no autosave, tests without a folder).
    pub autosave_dir: Option<PathBuf>,
    /// When the last copy was written, or the project was saved or opened.
    autosave_last: Instant,
    /// The name of the copy of a project that was never saved, for this run.
    autosave_session: String,
    autosave_run: Option<crate::autosave::Running>,
    /// The copies a crash left, offered one at a time at start (REQ-190).
    pub recoveries: Vec<crate::autosave::Copy>,
    pub playhead: Micros,
    pub clipboard: Option<Clip>,
    /// Timeline zoom in pixels per second (0 = fit all on next frame).
    pub zoom: f32,
    pub ffmpeg: Result<FfmpegInfo, String>,
    /// True while the ffmpeg check runs in the background.
    pub ffmpeg_checking: bool,
    ffmpeg_check: Option<Receiver<Result<FfmpegInfo, String>>>,
    /// A large LUT file being read in the background (2.1.0, REQ-300).
    lut_load: Option<lut::PendingLut>,
    /// Counts every change of the project (edit, undo, redo): the rows of the Applied
    /// effects are kept until it changes (REQ-313).
    rows_serial: u64,
    applied_cache: std::sync::Mutex<Option<(applied::AppliedKey, Option<Applied>)>>,
    applied_builds: std::sync::atomic::AtomicU32,
    /// The LUT being prepared for the preview in the background (2.2.0, REQ-312).
    lut_prep: Option<crate::lut::Prep>,
    /// LUT files whose preparation failed (not tried again until the project changes).
    lut_failed: Vec<String>,
    /// The preview restarted with a prepared LUT, until its first picture.
    lut_start: Option<lut::LutStart>,
    pub settings: Settings,
    pub(crate) settings_path: Option<PathBuf>,
    /// The floating windows (Settings, effect popups, clip details, shortcuts).
    pub popups: Popups,
    /// "Play this line" stops playback here.
    stop_at: Option<Micros>,
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
    /// Shortcut being changed: the action's name and the conflict found (REQ-242).
    pub rebind: Option<(String, Option<String>)>,
    /// The next export leaves out the LUT files that are missing (REQ-292).
    export_without_missing_luts: bool,
    /// The overlap that is selected (the lane, the widget and the preview follow it, REQ-325).
    pub selected_overlap: Option<u64>,
    /// An edit that would remove overlaps waits for the answer of the warning (REQ-329): the
    /// number of overlaps it would remove, noted by [`AppState::edit`].
    removal_ask: Option<usize>,
    /// The command that waits for the answer of the removal warning.
    pending_removal: Option<Command>,
    /// The edit being run again was confirmed: it removes its overlaps.
    removal_confirmed: bool,
    /// The window changed at this time and is not saved yet (REQ-234).
    window_dirty: Option<Instant>,
    pub preview: Preview,
    /// Latest preview frame and a counter that changes with it.
    pub frame: Option<Frame>,
    pub frame_serial: u64,
    export: Option<ExportHandle>,
    pub export_status: ExportStatus,
    /// The choices of the Export dialog, kept for the session (REQ-118): never saved.
    pub export_memory: Option<export::choice::ExportChoice>,
    /// The folder of the last export of the session.
    pub last_export_folder: Option<PathBuf>,
    /// Video hardware for "Export: Faster".
    pub hw: HwState,
    hw_probe: Option<RunningProbe>,
    /// The check of the current request was already repeated once (its first answer was
    /// inconclusive because something did not answer in time).
    hw_rechecked: bool,
    hw_generation: u64,
    /// Candidates that ran out of time twice in this session: not tried again until the
    /// program restarts (REQ-209). Never saved.
    hw_slow: BTreeSet<(Codec, hwenc::Vendor)>,
    /// The hardware decoder of each format the preview and the preview copies use now.
    pub hw_decoders: HashMap<Codec, hwdec::Decoder>,
    /// The check waits because an export or a subtitle job runs (REQ-208); it starts again
    /// when the job ends.
    hw_paused: bool,
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
    /// Fonts installed on this computer (scanned in the background).
    pub fonts: crate::fonts::Fonts,
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
    /// The first-start banner (REQ-045) held back while the splash is open (REQ-133).
    pub banner_after_splash: Option<Banner>,
}

fn check_ffmpeg(runner: &dyn Runner, settings: &Settings) -> Result<FfmpegInfo, String> {
    let located = ffmpeg::locate_both(settings.ffmpeg.as_deref(), settings.ffprobe.as_deref())
        .map_err(|e| e.to_string())?;
    ffmpeg::check_located(runner, &located).map_err(|e| e.to_string())
}

fn probe_pool(sched: &Arc<Scheduler>, runner: &Arc<dyn Runner>) -> Pool<ProbeJob, ProbeResult> {
    // Import analysis waits behind playback and export (REQ-224).
    let limited = sched.runner(Priority::Background, Arc::clone(runner));
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
        s.fonts = crate::fonts::Fonts::start();
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
        s.autosave_dir = crate::autosave::default_dir();
        s.find_recoveries();
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
            selection: Default::default(),
            drag_pos: None,
            frame_drag: None,
            clipboard_frames: None,
            clipboard_lut: None,
            clipboard_camera: None,
            resize_drag: None,
            live: HashMap::new(),
            camera_restart: None,
            pending_create: None,
            creating_for: None,
            pending_import: None,
            creating_import: false,
            import_offers: Default::default(),
            stt_matching: false,
            tracks_checking: Default::default(),
            track_check: None,
            changed_media: Default::default(),
            reanalysed: Vec::new(),
            clipboard_text: None,
            clipboard_keys: None,
            autosave_dir: None,
            autosave_last: Instant::now(),
            autosave_session: crate::autosave::new_session(),
            autosave_run: None,
            recoveries: Vec::new(),
            playhead: Micros::ZERO,
            clipboard: None,
            zoom: 0.0,
            ffmpeg,
            ffmpeg_checking: false,
            ffmpeg_check: None,
            lut_load: None,
            rows_serial: 0,
            applied_cache: std::sync::Mutex::new(None),
            applied_builds: std::sync::atomic::AtomicU32::new(0),
            lut_prep: None,
            lut_failed: Vec::new(),
            lut_start: None,
            settings,
            settings_path: None,
            popups: Popups::default(),
            stop_at: None,
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
            export_memory: None,
            last_export_folder: None,
            hw: HwState::default(),
            hw_probe: None,
            hw_rechecked: false,
            hw_generation: 0,
            hw_slow: BTreeSet::new(),
            hw_decoders: HashMap::new(),
            hw_paused: false,
            platform: Platform::none,
            probe_limits: ProbeLimits::default(),
            stall_timeout: export::STALL_TIMEOUT,
            stt: None,
            stt_progress: None,
            model_store: None,
            fonts: crate::fonts::Fonts::with(Vec::new()),
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
            banner_after_splash: None,
            rebind: None,
            export_without_missing_luts: false,
            selected_overlap: None,
            removal_ask: None,
            pending_removal: None,
            removal_confirmed: false,
            window_dirty: None,
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
        self.apply_hw_decoders();
        let now = Instant::now();
        let t = self.playhead;
        self.seek(t, now);
        let ids: Vec<MediaId> = self.project.media.iter().map(|m| m.id).collect();
        for id in ids {
            self.maybe_proxy(id);
        }
        self.start_track_check();
    }

    /// Starts the background check of the video hardware (needs a working ffmpeg; an
    /// older run still going is cancelled and its result ignored). If its answer is
    /// inconclusive (a candidate did not answer in time, e.g. on a busy computer), it is
    /// repeated once by itself; the first answer stays in use until the repeat ends.
    fn start_hw_probe(&mut self) {
        self.hw_rechecked = false;
        self.launch_hw_probe();
    }

    fn launch_hw_probe(&mut self) {
        self.stop_hw_probe();
        let Ok(info) = &self.ffmpeg else {
            self.hw.probe = HwProbe::NotChecked;
            return;
        };
        self.hw_generation += 1;
        let generation = self.hw_generation;
        let repeat = self.hw_rechecked && matches!(self.hw.probe, HwProbe::Done(_));
        // an export or a subtitle job goes first: the check starts when it ends (REQ-208)
        if self.is_exporting() || self.is_generating() {
            self.hw_paused = true;
            if !repeat {
                self.hw.probe = HwProbe::Checking(generation);
            }
            return;
        }
        self.hw_paused = false;
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let runner = self
            .limited(Priority::Probe)
            .cancellable(Arc::clone(&cancel));
        let ffmpeg = info.ffmpeg.clone();
        let encoders = info.encoders.clone();
        let platform = self.platform;
        let limits = self.probe_limits;
        // the automatic repeat of an inconclusive check tries everything again (REQ-211)
        let skip = if self.hw_rechecked {
            BTreeSet::new()
        } else {
            self.hw_slow.clone()
        };
        let flag = Arc::clone(&cancel);
        std::thread::spawn(move || {
            let status = hwenc::probe_all(
                &runner,
                &ffmpeg,
                &encoders,
                &platform(),
                limits,
                &skip,
                &flag,
            );
            let decoders = if flag.load(Ordering::SeqCst) {
                hwdec::Working::new()
            } else {
                hwdec::probe_all(&runner, &ffmpeg, &encoders, &platform(), limits, &flag)
            };
            if !flag.load(Ordering::SeqCst) {
                let _ = tx.send((generation, status, decoders));
            }
        });
        if !repeat {
            self.hw.probe = HwProbe::Checking(generation);
        }
        self.hw_probe = Some((generation, rx, cancel));
    }

    fn stop_hw_probe(&mut self) {
        self.hw_paused = false;
        if let Some((_, _, cancel)) = self.hw_probe.take() {
            cancel.store(true, Ordering::SeqCst);
        }
    }

    /// REQ-208: a check in progress is stopped when an export or a subtitle job starts and
    /// begun again when the last one ends. One check process at a time (the check runs its
    /// candidates one after the other).
    fn pause_or_resume_hw_probe(&mut self) {
        let job = self.is_exporting() || self.is_generating();
        if job && self.hw_probe.is_some() {
            // the answer of the stopped run is ignored; the repeat state is kept
            self.launch_hw_probe();
        } else if !job && self.hw_paused {
            self.launch_hw_probe();
        }
    }

    /// REQ-214: the hardware decoders of the preview and the preview copies. Only with
    /// Export: Faster, once the check has found them, on the chosen device (REQ-216);
    /// with Best quality no hardware decoding runs.
    pub(crate) fn apply_hw_decoders(&mut self) {
        self.hw_decoders = match &self.hw.probe {
            HwProbe::Done(_) if self.settings.export == ExportChoice::Faster => {
                hwdec::choose(&self.hw.decoders, self.settings.graphics_card.as_deref())
            }
            _ => HashMap::new(),
        };
        self.preview.set_decoders(self.hw_decoders.clone());
    }

    /// The check is being repeated because its first answer was inconclusive (REQ-211):
    /// Advanced says so until the result replaces the first answer.
    pub fn hw_repeating(&self) -> bool {
        self.hw_rechecked && matches!(self.hw.probe, HwProbe::Done(_)) && self.hw_probe.is_some()
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
        let device = self.settings.graphics_card.as_deref();
        let found = Codec::ALL
            .into_iter()
            .filter_map(|codec| {
                let c = status.chosen(codec, device)?;
                (!self.settings.hardware_is_off(codec, c.vendor)).then(|| (codec, c.clone()))
            })
            .collect();
        let offer = HwOffer { found };
        (!offer.is_empty()).then_some(offer)
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
        self.hw_slow.clear();
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

    /// Size of the exported picture (the preview's subtitles are placed in it).
    pub fn output_size(&self) -> Option<(u32, u32)> {
        let enc = self.encoders();
        render::output_geometry(&self.project, &enc)
            .ok()
            .map(|g| (g.width, g.height))
    }

    pub fn frame_len(&self) -> Micros {
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

    /// Runs a command. Errors are also put in the status line.
    pub fn dispatch(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        // with overlaps in the project an edit that would remove some asks first (REQ-329)
        let again = self.project.timeline.any_overlap().then(|| cmd.clone());
        let r = self.dispatch_inner(cmd, now);
        if let (Err(e), Some(again)) = (&r, again)
            && e == overlap::REMOVAL_ASK
        {
            let n = self.removal_ask.take().unwrap_or(0);
            self.pending_removal = Some(again);
            self.modal = Some(Modal::RemoveOverlaps(n));
            self.refresh_dirty();
            return Ok(());
        }
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
                // a clean exit leaves no copy, so the next start offers nothing
                self.clear_autosave(now);
                self.close_allowed = true;
                self.quit_requested = true;
                Ok(())
            }
        }
    }

    fn dispatch_inner(&mut self, cmd: Command, now: Instant) -> Result<(), String> {
        let label = cmd.undo_name();
        let mut cmd = cmd;
        for area in DISPATCH_AREAS {
            match area(self, cmd, now, label) {
                Routed::Done(r) => return r,
                Routed::Pass(c) => cmd = c,
            }
        }
        Err("command not handled".into())
    }

    /// Index of the boundary between `left` and `right`, when they are adjacent.
    pub fn boundary_of(&self, left: ClipId, right: ClipId) -> Option<usize> {
        self.project
            .timeline
            .clips
            .windows(2)
            .position(|w| w[0].id == left && w[1].id == right)
    }

    /// Where a fade or line starts on the timeline (`None` when it no longer exists).
    pub fn instance_start(&self, i: Instance) -> Option<Micros> {
        let tl = &self.project.timeline;
        match i {
            Instance::Fade(l, r) => {
                let b = self.boundary_of(l, r).filter(|b| tl.has_fade(*b))?;
                Some(tl.start_of(b + 1))
            }
            Instance::Line(id, clip) => self
                .project
                .subtitles
                .project(tl)
                .into_iter()
                .find(|x| x.line == id && x.clip == clip)
                .map(|x| x.start),
            Instance::FadeStart => {
                (tl.start_fade() && !tl.clips.is_empty()).then_some(Micros::ZERO)
            }
            Instance::FadeEnd => {
                let last = tl.clips.last().filter(|_| tl.end_fade())?;
                Some(tl.duration() - last.fade_len_of(tl.fade_len()))
            }
            Instance::ClipVolume(c) | Instance::CleanAudio(c) => {
                tl.index_of(c).map(|i| tl.start_of(i))
            }
            Instance::Pov(m, t) => self.pov_position(m, t),
            Instance::Aspect
            | Instance::Lut(FrameTarget::Whole)
            | Instance::Speed(FrameTarget::Whole) => (!tl.clips.is_empty()).then_some(Micros::ZERO),
            Instance::Scale(FrameTarget::Whole) | Instance::Move(FrameTarget::Whole) => None,
            Instance::Scale(FrameTarget::Clip(c))
            | Instance::Move(FrameTarget::Clip(c))
            | Instance::Lut(FrameTarget::Clip(c))
            | Instance::Speed(FrameTarget::Clip(c)) => tl.index_of(c).map(|i| tl.start_of(i)),
        }
    }

    /// Opens popup `p` showing the effect as applied to `t`.
    fn show_popup_for(&mut self, p: Popup, t: Target) {
        self.popups.show(p);
        self.popups.set_target(p, t);
    }

    /// The speech-file consent dialog offers "Use without matching words": subtitles of
    /// the video are being reused and nothing else needs the speech file.
    pub fn consent_offers_spread(&self) -> bool {
        let Some(req) = &self.pending_create else {
            return false;
        };
        if !matches!(req.source, Some(SourceChoice::Use(_))) {
            return false;
        }
        // every part with a track can do without; parts without one would still need it
        self.create_ranges(req.clip).iter().all(|(m, _)| {
            self.project
                .media(*m)
                .and_then(|x| x.format.subtitle_tracks.as_ref())
                .is_some_and(|t| !t.is_empty())
        })
    }

    /// "Undo Split", or "Undo" when there is nothing to undo.
    pub fn undo_text(&self) -> String {
        match self.history.undo_label() {
            Some(l) => format!("Undo {l}"),
            None => "Undo".into(),
        }
    }

    /// "Redo Split", or "Redo" when there is nothing to redo.
    pub fn redo_text(&self) -> String {
        match self.history.redo_label() {
            Some(l) => format!("Redo {l}"),
            None => "Redo".into(),
        }
    }

    /// Remembers a project in the recent list (saved when it changed).
    fn remember_recent(&mut self, path: &Path) {
        let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        if settings::push_recent(&mut self.settings.recent, &abs) {
            let _ = self.save_settings();
        }
    }

    /// Background work for the status bar, with progress and the command that stops it.
    pub fn background_jobs(&self) -> Vec<Job> {
        let mut jobs = Vec::new();
        let analysing = self
            .project
            .media
            .iter()
            .filter(|m| m.status == MediaStatus::Analysing)
            .count();
        if analysing > 0 {
            jobs.push(Job {
                label: format!("Analysing {analysing} file(s)"),
                progress: None,
                cancel: None,
            });
        }
        let copies: Vec<f32> = self
            .project
            .media
            .iter()
            .filter_map(|m| match m.proxy {
                ProxyState::Queued => Some(0.0),
                ProxyState::Generating(p) => Some(f32::from(p.min(100)) / 100.0),
                _ => None,
            })
            .collect();
        if !copies.is_empty() {
            jobs.push(Job {
                label: format!("Preview copies: {}", copies.len()),
                progress: Some(copies.iter().sum::<f32>() / copies.len() as f32),
                cancel: None,
            });
        }
        if let Some(p) = self.model_progress {
            jobs.push(Job {
                label: "Downloading the speech recognition file".into(),
                progress: Some(p),
                cancel: Some(Command::CancelSubtitles),
            });
        }
        if !self.tracks_checking.is_empty() {
            jobs.push(Job {
                label: subs_create::CHECKING_TRACKS.trim_end_matches('…').into(),
                progress: None,
                cancel: None,
            });
        }
        if let Some(p) = self.stt_progress {
            jobs.push(Job {
                label: if self.stt_matching {
                    "Matching subtitles to the speech".into()
                } else {
                    "Generating subtitles".into()
                },
                progress: Some(p),
                cancel: Some(Command::CancelSubtitles),
            });
        }
        if self.is_exporting() {
            let s = &self.export_status;
            let mode = match s.stream_copy {
                Some(true) => "Exporting (fast, no re-encoding)",
                Some(false) => "Exporting (re-encoding)",
                None => "Exporting (preparing)",
            };
            jobs.push(Job {
                label: mode.into(),
                progress: Some(s.progress),
                cancel: Some(Command::CancelExport),
            });
        }
        jobs
    }

    fn duration_of(&self, id: MediaId) -> Result<Micros, String> {
        self.project
            .media(id)
            .map(|m| m.duration)
            .ok_or_else(|| "no such media".to_string())
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
        self.remember_recent(&path);
        self.project_path = Some(path);
        self.saved = self.project.doc();
        self.status = "Project saved".into();
        self.clear_autosave(Instant::now());
        Ok(())
    }

    fn open_now(&mut self, path: PathBuf, now: Instant) -> Result<(), String> {
        let (p, missing, migration) = project::load_noted(&path).map_err(|e| e.to_string())?;
        // a project of format 13 or older: Crop dropped and picture items made overlaps, said once
        // on the status line, with no undo step and no change mark (REQ-332)
        let converted = p.converted.map(|c| c.message());
        self.remember_recent(&path);
        self.load_project(p, Some(path), now);
        if !missing.is_empty() {
            self.notice = Some(format!(
                "Some files could not be found or were repaired:\n{}",
                missing.join("\n")
            ));
        }
        // a project from before 2.0.3: one message about its text items, once (REQ-167)
        self.status = migration.map_or_else(|| "Project opened".into(), |m| m.message());
        if let Some(line) = converted {
            self.status = line;
        }
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
        self.stop_track_check();
        self.pending_create = None;
        self.creating_for = None;
        self.pending_import = None;
        self.creating_import = false;
        self.import_offers.clear();
    }

    fn load_project(&mut self, p: Project, path: Option<PathBuf>, now: Instant) {
        self.clear_autosave(now);
        self.cancel_project_jobs();
        self.preview.invalidate(now);
        self.preview.forget_frame();
        self.saved = p.doc();
        self.project = p;
        self.rows_serial += 1;
        self.project_path = path;
        self.history.clear();
        self.selected_clip = None;
        self.selected_media = None;
        self.selected_music = None;
        self.selection.clear();
        self.drag_pos = None;
        self.clipboard = None;
        self.frame = None;
        self.zoom = 0.0;
        self.stop_at = None;
        self.popups.instance = None;
        self.popups.chooser = None;
        self.popups.close(Popup::Instance);
        self.popups.close(Popup::Text);
        self.popups.targets.clear();
        self.close_allowed = false;
        let ids: Vec<MediaId> = self.project.media.iter().map(|m| m.id).collect();
        for id in ids {
            self.maybe_proxy(id);
        }
        self.find_changed_media();
        self.start_track_check();
        self.refresh_import_offers();
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
        // whose file is gone, so the states that refer to the file are dropped from the
        // history and the other steps stay undoable in order (REQ-196).
        self.project.subtitles.lines.retain(|l| l.media != id);
        self.project.camera.drop_media(id);
        self.live.remove(&id);
        self.history.forget(|e| {
            e.timeline.clips.iter().any(|c| c.media == id)
                || e.music.clips.iter().any(|c| c.media == id)
                || e.subtitles.lines.iter().any(|l| l.media == id)
                || !e.camera.path(id).is_empty()
        });
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
        let Ok(key) = proxy::key_of_kind(&m.path, m.format.is_360()) else {
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
                hwaccel: m
                    .format
                    .vcodec
                    .as_deref()
                    .and_then(Codec::from_name)
                    .and_then(|c| self.hw_decoders.get(&c))
                    .map(hwdec::Decoder::input_args)
                    .unwrap_or_default(),
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

    /// The video has a 360° clip.
    pub fn has_sphere_clip(&self) -> bool {
        self.project.timeline.clips.iter().any(|c| {
            self.project
                .media(c.media)
                .is_some_and(|m| m.format.is_360())
        })
    }

    /// The video has a clip that is not 360°.
    fn has_flat_clip(&self) -> bool {
        self.project.timeline.clips.iter().any(|c| {
            self.project
                .media(c.media)
                .is_none_or(|m| !m.format.is_360())
        })
    }

    /// Applies an edit to a copy of the editable state; on success records undo and
    /// refreshes.
    fn edit(
        &mut self,
        now: Instant,
        label: &'static str,
        f: impl FnOnce(&mut Edit) -> Result<(), String>,
    ) -> Result<(), String> {
        let before = self.project.edit();
        let mut e = before.clone();
        f(&mut e)?;
        // an edit that removes overlaps (a trim, a delete of their clip) is asked about first,
        // before anything is changed (REQ-329, ADR-234)
        let lost = crate::overlap::lost(&before.timeline.overlap_ids(), &e.timeline.overlap_ids());
        if !lost.is_empty() && !self.removal_confirmed {
            self.removal_ask = Some(lost.len());
            return Err(overlap::REMOVAL_ASK.into());
        }
        self.history.record(before, label);
        self.replace_edit(e, now);
        Ok(())
    }

    /// The levels the Volume popup shows: the music's, and the video sound as the first
    /// clip's own level (a whole-video Volume sets the level of every clip, REQ-147).
    pub fn volume_mix(&self) -> Mix {
        let mut m = self.project.mix;
        if let Some(c) = self.project.timeline.clips.first() {
            m.video_volume = c.volume;
            m.video_muted = c.muted;
        }
        m
    }

    fn replace_edit(&mut self, e: Edit, now: Instant) {
        self.project.set_edit(e);
        self.rows_serial += 1;
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
        self.prune_selection();
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
        self.clear_overrides();
        self.camera_restart = None;
        if self.project.timeline.clips.is_empty() {
            self.preview.invalidate(now);
            self.preview.forget_frame();
            self.frame = None;
            return;
        }
        if self.ffmpeg.is_ok() {
            let shown = self.shown_project().into_owned();
            let _ = self.preview.seek(&shown, t, now);
        }
    }

    /// Asks for the picture at the playhead again (it is rendered only if it changed).
    fn refresh_still(&mut self, now: Instant) {
        if !self.preview.is_playing()
            && !self.project.timeline.clips.is_empty()
            && self.ffmpeg.is_ok()
        {
            let t = self.playhead;
            let shown = self.shown_project().into_owned();
            let _ = self.preview.request_still(&shown, t, now);
        }
    }

    /// The start-up splash (REQ-295): shown by the user's choice; the version that showed
    /// it is remembered for "Once for each version".
    pub fn start_splash(&mut self) {
        let version = env!("CARGO_PKG_VERSION");
        let last = self.settings.last_splash_version.clone();
        if splash::should_show(self.settings.show_splash, last.as_deref(), version, false) {
            self.show_splash();
            if self.settings.last_splash_version.as_deref() != Some(version) {
                self.settings.last_splash_version = Some(version.to_string());
                let _ = self.save_settings();
            }
        }
    }

    /// Opens the splash screen (REQ-133). The first-start banner waits until it is closed.
    pub fn show_splash(&mut self) {
        self.popups.show(Popup::Splash);
        if self.banner.is_some() {
            self.banner_after_splash = self.banner.take();
        }
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Undo name of toggling a fade that is (`true`) or is not there.
fn fade_label(present: bool) -> &'static str {
    if present { "Remove fade" } else { "Add fade" }
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

mod applied;
mod command;
mod dispatch_effects;
mod dispatch_interface;
mod dispatch_objects;
mod dispatch_playback;
mod dispatch_project;
mod dispatch_settings;
mod dispatch_subtitles;
mod dispatch_view;
mod effect_kind;
mod effects;
mod enabled;
mod export_flow;
mod frame;
mod lut;
pub mod overlap;
mod placing;
mod recovery;
mod sphere;
pub mod stops;
mod subs_create;
pub mod subs_import;
mod texts;
mod tick;
pub use subs_create::pick_track;

#[cfg(test)]
mod applied_tests;
#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod clarity_tests;
#[cfg(test)]
mod effects_tests;
#[cfg(test)]
mod frame_tests;
#[cfg(test)]
mod hw_tests;
#[cfg(test)]
mod lut_tests;
#[cfg(test)]
mod overlap_tests;
#[cfg(test)]
mod placing_tests;
#[cfg(test)]
mod player_link_tests;
#[cfg(test)]
mod recovery_tests;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod speed_tests;
#[cfg(test)]
mod sphere_tests;
#[cfg(test)]
mod style_tests;
#[cfg(test)]
mod subs_import_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod ux_tests;
