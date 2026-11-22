// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Export worker: runs ffmpeg in a thread, reports progress over a channel, writes to
//! `<dir>/.<name>.videolore-partial` and renames it on success; deletes it on cancel/error.
//! Sources are only ever read. Burned-in subtitles are written, with the bundled font, to a
//! private temporary folder that is ffmpeg's working folder.
//!
//! With "Export: Faster" ([`ExportJob::hw`]) a re-encode to H.264/HEVC first runs on the
//! video hardware into `.<name>.hw.videolore-partial`, watched for stalls, and the result is
//! checked ([`crate::outcheck`]) before it is published; any problem silently redoes the
//! export with the normal method, whose commands are exactly those of 0.3.0.

pub mod choice;

use crate::ass;
use crate::ffmpeg::{
    CmdOutput, FfVersion, Process, ProgressEvent, Runner, is_keyframe, parse_progress_line,
    probe_keyframes,
};
use crate::hwenc::{self, Candidate, Codec, HwOffer, RateControl, Vendor};
use crate::media::MediaId;
use crate::outcheck::{self, CheckFailure, Expected, ExpectedAudio};
use crate::project::Project;
use crate::render::{self, ExportPaths, Plan};
use crate::time::Micros;
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A hardware run (or a check step) with no progress for this long is stopped.
pub const STALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Longest metadata read of the output check.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// After a kill, how long to wait for a process to end before leaving it behind.
pub const REAP_TIMEOUT: Duration = Duration::from_secs(5);
/// Share of the progress bar for the hardware run; the check takes the rest.
const HW_SHARE: f64 = 0.85;

/// What went wrong with the video hardware.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// Ended with an error or crashed.
    ExitedWithError,
    /// No progress for [`STALL_TIMEOUT`].
    Stalled,
    /// The file failed the output check.
    CheckFailed(CheckFailure),
}

impl Problem {
    pub fn text(&self) -> String {
        match self {
            Problem::ExitedWithError => "it stopped with an error".into(),
            Problem::Stalled => "it stopped responding".into(),
            Problem::CheckFailed(f) => format!("the result failed the check: {f}"),
        }
    }
}

/// How an export was made.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportMethod {
    StreamCopy,
    Software,
    Hardware(Vendor),
    /// The hardware had a problem; the normal method made the file.
    SoftwareAfterProblem {
        codec: Codec,
        vendor: Vendor,
        problem: Problem,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportEvent {
    /// Fraction done, 0.0..=1.0.
    Progress(f32),
    /// Whether stream copy was used (sent once, before progress).
    Started {
        stream_copy: bool,
    },
    Done {
        path: PathBuf,
        method: ExportMethod,
    },
    Failed(String),
    Cancelled,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExportError {
    #[error("the output file cannot be one of the source files")]
    OutputIsSource,
    #[error("the output folder does not exist")]
    NoFolder,
    #[error("{0}")]
    Render(#[from] render::RenderError),
    #[error("{0}")]
    Io(String),
    #[error("ffmpeg failed: {0}")]
    Ffmpeg(String),
    #[error("cancelled")]
    Cancelled,
    #[error("this ffmpeg cannot burn in subtitles (it lacks the libass 'subtitles' filter)")]
    NoSubtitleFilter,
}

/// What the worker needs (an owned snapshot, so editing can continue meanwhile).
#[derive(Debug, Clone)]
pub struct ExportJob {
    pub project: Project,
    pub output: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub version: FfVersion,
    pub encoders: BTreeSet<String>,
    /// The ffmpeg has the `subtitles` filter (needed when there are subtitles).
    pub can_burn_subtitles: bool,
    /// ffmpeg threads ("How hard videolore works"); `None`: ffmpeg decides.
    pub threads: Option<u32>,
    /// Root of the private work folder (subtitle file, font, crash cleanup).
    pub work_root: PathBuf,
    /// Hardware methods the export may use ("Export: Faster"); `None`: normal method.
    pub hw: Option<HwOffer>,
    /// No progress for this long stops a hardware run or check step ([`STALL_TIMEOUT`]).
    pub stall_timeout: Duration,
    /// Time limit of the check's metadata read ([`CHECK_TIMEOUT`]).
    pub check_timeout: Duration,
    /// What to make of a project with 360° clips (REQ-103).
    pub mode: render::ExportMode,
    /// The video preset and audio format chosen in the Export dialog (REQ-120, REQ-121).
    pub format: crate::format::FormatChoice,
    /// Only the sound of the timeline is exported (REQ-119).
    pub audio_only: bool,
}

/// `<dir>/.<name>.videolore-partial`.
pub fn partial_path(output: &Path) -> PathBuf {
    let name = output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    output.with_file_name(format!(".{name}.videolore-partial"))
}

fn canonical(p: &Path) -> Option<PathBuf> {
    if let Ok(c) = p.canonicalize() {
        return Some(c);
    }
    let parent = p.parent().filter(|d| !d.as_os_str().is_empty())?;
    Some(parent.canonicalize().ok()?.join(p.file_name()?))
}

/// Refuses outputs that would overwrite a source (or its folder is missing).
pub fn validate_output(project: &Project, output: &Path) -> Result<PathBuf, ExportError> {
    let abs = std::path::absolute(output).map_err(|e| ExportError::Io(e.to_string()))?;
    if !abs.parent().is_some_and(Path::is_dir) {
        return Err(ExportError::NoFolder);
    }
    let out = canonical(&abs).ok_or(ExportError::NoFolder)?;
    let partial = partial_path(&out);
    for m in &project.media {
        let src = canonical(&m.path).unwrap_or_else(|| m.path.clone());
        if src == out || src == partial {
            return Err(ExportError::OutputIsSource);
        }
    }
    Ok(out)
}

/// A running export.
pub struct ExportHandle {
    pub rx: Receiver<ExportEvent>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ExportHandle {
    /// A finished handle replaying already collected events.
    pub fn from_events(rx: Receiver<ExportEvent>) -> ExportHandle {
        ExportHandle {
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Waits for the worker to end (used by tests and on shutdown).
    pub fn join(mut self) -> Vec<ExportEvent> {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.rx.try_iter().collect()
    }
}

impl Drop for ExportHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Starts the export in a worker thread.
pub fn start(job: ExportJob, runner: Arc<dyn Runner>) -> ExportHandle {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let thread = std::thread::spawn(move || {
        let ev = match run(&job, runner.as_ref(), &flag, &tx) {
            Ok((path, method)) => ExportEvent::Done { path, method },
            Err(ExportError::Cancelled) => ExportEvent::Cancelled,
            Err(e) => ExportEvent::Failed(e.to_string()),
        };
        let _ = tx.send(ev);
    });
    ExportHandle {
        rx,
        cancel,
        thread: Some(thread),
    }
}

/// Builds the plan, probing keyframes (cached per source) only when stream copy is possible.
pub fn plan(job: &ExportJob, runner: &dyn Runner) -> Result<Plan, ExportError> {
    let mut cache: HashMap<MediaId, Vec<Micros>> = HashMap::new();
    let plan = render::plan_export_choice(
        &job.project,
        &job.encoders,
        job.mode,
        &job.format,
        |m, t| {
            let kf = cache.entry(m.id).or_insert_with(|| {
                probe_keyframes(runner, &job.ffprobe, &m.path).unwrap_or_default()
            });
            let half_frame = m
                .format
                .fps
                .map(|f| Micros(f.frame_duration().0 / 2))
                .unwrap_or(Micros(1000));
            is_keyframe(kf, t, half_frame)
        },
    )?;
    Ok(plan)
}

/// What makes an export use the video hardware: a re-encode to H.264 or HEVC with a
/// known bitrate target, and a working method for that format in `job.hw`.
pub fn hw_choice(job: &ExportJob, plan: &Plan) -> Option<(Candidate, Codec, u64)> {
    let Plan::Reencode { settings, .. } = plan else {
        return None;
    };
    let codec = Codec::of_software_encoder(&settings.vencoder)?;
    let cand = job.hw.as_ref()?.get(codec)?.clone();
    // a preset has no bitrate of its own (the normal method uses constant quality): the
    // hardware gets one by the size of the picture (REQ-212, REQ-213)
    let bps = settings.video_bitrate.or_else(|| {
        job.format
            .video
            .is_some()
            .then(|| hwenc::preset_bitrate(codec, settings.width, settings.height, settings.fps))
    })?;
    Some((cand, codec, bps))
}

/// The export itself (synchronous). Always removes the partial files unless it succeeded.
///
/// With a hardware method ([`hw_choice`]) the export first runs on the hardware into its
/// own partial file, watched for stalls, then checks the result ([`crate::outcheck`]); on
/// any problem it silently redoes the export with the normal method (progress goes back to
/// 0 once). Cancelling stops every phase and never starts the redo.
pub fn run(
    job: &ExportJob,
    runner: &dyn Runner,
    cancel: &AtomicBool,
    tx: &Sender<ExportEvent>,
) -> Result<(PathBuf, ExportMethod), ExportError> {
    if job.audio_only {
        return run_audio(job, runner, cancel, tx);
    }
    let output = validate_output(&job.project, &job.output)?;
    let partial = partial_path(&output);
    let plan = plan(job, runner)?;
    let _ = tx.send(ExportEvent::Started {
        stream_copy: matches!(plan, Plan::StreamCopy { .. }),
    });
    let io = |e: std::io::Error| ExportError::Io(e.to_string());
    if plan.subtitles().is_some() && !job.can_burn_subtitles {
        return Err(ExportError::NoSubtitleFilter);
    }
    let work = crate::base::private::Work::new(&job.work_root).map_err(io)?;
    if let Some(doc) = plan.subtitles() {
        write_subtitles(work.path(), doc).map_err(io)?;
    }
    if let Some(t) = plan.track() {
        std::fs::write(work.path().join(t.file), &t.content).map_err(io)?;
    }
    render::write_command_files(work.path(), plan.command_files(), plan.lut_files()).map_err(io)?;
    prepare_spatial(job, runner, plan.spatial(), work.path(), cancel)?;
    let mut after = None;
    if let Some((cand, codec, bps)) = hw_choice(job, &plan) {
        let attempt = Attempt {
            job,
            runner,
            plan: &plan,
            work: &work,
            cancel,
            tx,
            stop: AtomicBool::new(false),
            shown: AtomicU32::new(0),
        };
        match attempt.hardware(&output, &cand, codec, bps) {
            Ok(()) => return Ok((output, ExportMethod::Hardware(cand.vendor))),
            Err(HwError::Fatal(e)) => return Err(e),
            Err(HwError::Problem(problem)) => {
                if cancel.load(Ordering::SeqCst) {
                    return Err(ExportError::Cancelled);
                }
                after = Some((codec, cand.vendor, problem));
                let _ = tx.send(ExportEvent::Progress(0.0));
            }
        }
    }
    let list = work.path().join("list.ffconcat");
    let script = work.path().join("graph.txt");
    let paths = ExportPaths {
        output: &partial,
        concat_list: &list,
        graph_script: &script,
    };
    let args = render::export_args(&plan, &paths, job.version, job.threads, None).map_err(io)?;
    // The partial file is private until it is complete; a crash leaves it listed in the
    // work folder, so the next start removes it.
    work.register(&partial).map_err(io)?;
    drop(crate::base::private::create_private_file(&partial).map_err(io)?);
    let result = execute(job, runner, &args, work.path(), cancel, tx);
    let method = match (after, &plan) {
        (Some((codec, vendor, problem)), _) => ExportMethod::SoftwareAfterProblem {
            codec,
            vendor,
            problem,
        },
        (None, Plan::StreamCopy { .. }) => ExportMethod::StreamCopy,
        (None, Plan::Reencode { .. }) => ExportMethod::Software,
    };
    match result {
        Ok(()) => match finish_file(job, &plan, &partial)
            .and_then(|()| crate::base::private::publish(&partial))
            .and_then(|()| std::fs::rename(&partial, &output))
        {
            Ok(()) => Ok((output, method)),
            Err(e) => {
                let _ = std::fs::remove_file(&partial);
                Err(ExportError::Io(e.to_string()))
            }
        },
        Err(e) => {
            let _ = std::fs::remove_file(&partial);
            Err(e)
        }
    }
}

/// Turns the ambisonic sound of the 360 clips by their cameras into the work folder, where the
/// filter graph reads it (REQ-273).
fn prepare_spatial(
    job: &ExportJob,
    runner: &dyn Runner,
    jobs: &[crate::spatial::SpatialJob],
    dir: &Path,
    cancel: &AtomicBool,
) -> Result<(), ExportError> {
    for sj in jobs {
        crate::spatial::prepare(runner, &job.ffmpeg, dir, sj, cancel).map_err(|e| {
            if e.kind() == std::io::ErrorKind::Interrupted {
                ExportError::Cancelled
            } else {
                ExportError::Io(format!("spatial sound: {e}"))
            }
        })?;
    }
    Ok(())
}

/// "Only audio" (REQ-119): the sound of the whole timeline written to the output through
/// the same partial-file rule as a video.
fn run_audio(
    job: &ExportJob,
    runner: &dyn Runner,
    cancel: &AtomicBool,
    tx: &Sender<ExportEvent>,
) -> Result<(PathBuf, ExportMethod), ExportError> {
    let output = validate_output(&job.project, &job.output)?;
    let partial = partial_path(&output);
    let plan = render::plan_audio_only(&job.project, &job.encoders, job.format.audio)?;
    let _ = tx.send(ExportEvent::Started { stream_copy: false });
    let io = |e: std::io::Error| ExportError::Io(e.to_string());
    let work = crate::base::private::Work::new(&job.work_root).map_err(io)?;
    prepare_spatial(job, runner, &plan.graph.spatial, work.path(), cancel)?;
    let list = work.path().join("list.ffconcat");
    let script = work.path().join("graph.txt");
    let paths = ExportPaths {
        output: &partial,
        concat_list: &list,
        graph_script: &script,
    };
    let args = render::audio_export_args(&plan, &paths, job.version, job.threads).map_err(io)?;
    work.register(&partial).map_err(io)?;
    drop(crate::base::private::create_private_file(&partial).map_err(io)?);
    let result = execute(job, runner, &args, work.path(), cancel, tx);
    match result {
        Ok(()) => match crate::base::private::publish(&partial)
            .and_then(|()| std::fs::rename(&partial, &output))
        {
            Ok(()) => Ok((output, ExportMethod::Software)),
            Err(e) => {
                let _ = std::fs::remove_file(&partial);
                Err(ExportError::Io(e.to_string()))
            }
        },
        Err(e) => {
            let _ = std::fs::remove_file(&partial);
            Err(e)
        }
    }
}

/// A 360 video carries the spherical-video information (REQ-103): written into the
/// finished file before it is published. Other containers are left as they are.
fn mark_if_sphere(job: &ExportJob, file: &Path) -> std::io::Result<()> {
    if job.mode == render::ExportMode::Sphere360 {
        // MP4 and MOV by the movie box, Matroska and WebM by the track's Projection (REQ-261);
        // each leaves the other kind of file alone
        // the initial view of the source is written again (REQ-262)
        let pose = job
            .project
            .media
            .iter()
            .find(|m| m.format.is_360())
            .and_then(|m| m.format.pose);
        crate::spherical::mark_equirectangular_with(file, pose)?;
        crate::spherical::mark_matroska_with(file, pose)?;
    }
    Ok(())
}

/// A separate subtitle track in an MP4 or MOV file is switched off by default (REQ-182);
/// the other containers keep their disposition.
fn finish_file(job: &ExportJob, plan: &Plan, file: &Path) -> std::io::Result<()> {
    mark_if_sphere(job, file)?;
    if plan.track().is_some()
        && matches!(
            job.output.extension().and_then(|e| e.to_str()),
            Some("mp4" | "mov" | "m4v")
        )
    {
        crate::mp4tracks::disable_subtitle_tracks(file)?;
    }
    Ok(())
}

/// `<dir>/.<name>.hw.videolore-partial`: the hardware attempt's own partial file.
pub fn hw_partial_path(output: &Path) -> PathBuf {
    let name = output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    output.with_file_name(format!(".{name}.hw.videolore-partial"))
}

/// Why a hardware run ended without a file.
enum Encoded {
    /// Constant quality was growing this many times too fast: try a coarser quantiser.
    TooBig(f64),
    Problem(Problem),
}

/// Why the hardware attempt ended without a file.
enum HwError {
    /// Redo with the normal method (or stop, when cancelled).
    Problem(Problem),
    /// Stop the export with this error.
    Fatal(ExportError),
}

/// Everything the hardware attempt needs.
struct Attempt<'a> {
    job: &'a ExportJob,
    runner: &'a dyn Runner,
    plan: &'a Plan,
    work: &'a crate::base::private::Work,
    cancel: &'a AtomicBool,
    tx: &'a Sender<ExportEvent>,
    /// Stops the running process without cancelling the export (a constant-quality run
    /// that grows too big is restarted).
    stop: AtomicBool,
    /// The highest progress shown by this run (bits of an `f32`): the bar never goes back
    /// while a run goes on (ffmpeg's last time can be a little under an earlier one).
    shown: AtomicU32,
}

/// How far the strict decode (thousandths) and the sample comparisons (how many are done) of
/// the output check have got: they run at the same time (REQ-315).
#[derive(Default)]
struct CheckProgress {
    decode: AtomicU32,
    ranges: AtomicU32,
}

/// Outcome of a watched process.
enum Watched {
    Done(CmdOutput),
    Cancelled,
    Stalled,
}

/// How often a watched process is looked at (cancel and stall).
const WATCH_POLL: Duration = Duration::from_millis(100);

/// Kills `child` and waits at most [`REAP_TIMEOUT`] for it; a process that does not end
/// (a driver hang) is left to a background thread, which keeps its slot until it ends.
fn reap(mut child: Box<dyn Process>) {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        child.kill();
        let _ = child.wait();
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(REAP_TIMEOUT);
}

/// Progress marks seen so far: output time and frame.
#[derive(Default)]
struct Marks {
    time: i64,
    frame: u64,
}

impl Marks {
    /// True when `line` shows the process moved on.
    fn advanced(&mut self, line: &str) -> bool {
        match parse_progress_line(line) {
            Some(ProgressEvent::Time(t)) if t.0 > self.time => {
                self.time = t.0;
                true
            }
            Some(ProgressEvent::End) => true,
            _ => match line.trim().strip_prefix("frame=").map(str::trim) {
                Some(f) => match f.parse::<u64>() {
                    Ok(n) if n > self.frame => {
                        self.frame = n;
                        true
                    }
                    _ => false,
                },
                None => false,
            },
        }
    }
}

/// Follows `child`'s `-progress` lines until it ends; kills it when `cancel` is set or when
/// it makes no progress for `stall` (counted from the start, then from the last progress).
fn supervise(
    mut child: Box<dyn Process>,
    cancel: &AtomicBool,
    stop: &AtomicBool,
    stall: Duration,
    mut on_line: impl FnMut(&str),
) -> Result<Watched, String> {
    let cancel = || cancel.load(Ordering::SeqCst) || stop.load(Ordering::SeqCst);
    let (tx, rx) = channel::<String>();
    if let Some(out) = child.take_stdout() {
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    } else {
        drop(tx);
    }
    let mut marks = Marks::default();
    let mut last = Instant::now();
    loop {
        if cancel() {
            reap(child);
            return Ok(Watched::Cancelled);
        }
        match rx.recv_timeout(WATCH_POLL.min(stall)) {
            Ok(line) => {
                if marks.advanced(&line) {
                    last = Instant::now();
                }
                on_line(&line);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if last.elapsed() >= stall {
            reap(child);
            return Ok(Watched::Stalled);
        }
    }
    if cancel() {
        reap(child);
        return Ok(Watched::Cancelled);
    }
    child.wait().map(Watched::Done).map_err(|e| e.to_string())
}

impl Attempt<'_> {
    fn progress(&self, f: f64) {
        let v = f.clamp(0.0, 1.0) as f32;
        // non-negative floats order like their bits
        let before = f32::from_bits(self.shown.fetch_max(v.to_bits(), Ordering::SeqCst));
        let _ = self.tx.send(ExportEvent::Progress(v.max(before)));
    }

    /// The bar starts again from the beginning (a constant-quality run is restarted).
    fn restart_progress(&self) {
        self.shown.store(0, Ordering::SeqCst);
        self.progress(0.0);
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Spawns a watched process; a cancelled wait for a slot is a cancel.
    fn watch(
        &self,
        args: &[std::ffi::OsString],
        cwd: Option<&Path>,
        on_line: impl FnMut(&str),
    ) -> Result<Watched, String> {
        match self.runner.spawn_in(&self.job.ffmpeg, args, cwd) {
            Ok(child) => supervise(
                child,
                self.cancel,
                &self.stop,
                self.job.stall_timeout,
                on_line,
            ),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(Watched::Cancelled),
            Err(e) => Err(e.to_string()),
        }
    }

    /// The hardware run, its check and the publication of the file.
    fn hardware(
        &self,
        output: &Path,
        cand: &Candidate,
        codec: Codec,
        bps: u64,
    ) -> Result<(), HwError> {
        let partial = hw_partial_path(output);
        let fatal = |e: std::io::Error| HwError::Fatal(ExportError::Io(e.to_string()));
        let dir = self.work.path().join("hw");
        std::fs::create_dir_all(&dir).map_err(fatal)?;
        let list = dir.join("list.ffconcat");
        let script = dir.join("graph.txt");
        let paths = ExportPaths {
            output: &partial,
            concat_list: &list,
            graph_script: &script,
        };
        self.work.register(&partial).map_err(fatal)?;
        let mut target = hwenc::Target::new(bps);
        let cq = cand.rc == RateControl::ConstantQuality;
        let mut retried = false;
        let r = loop {
            let enc = hwenc::encode_args(cand, codec, target);
            let args = render::export_args(
                self.plan,
                &paths,
                self.job.version,
                self.job.threads,
                Some(&enc),
            )
            .map_err(fatal)?;
            drop(crate::base::private::create_private_file(&partial).map_err(fatal)?);
            match self.encode(&args, bps, cq && !retried) {
                Err(Encoded::TooBig(ratio)) => {
                    // constant quality making a much bigger file: once, a coarser quantiser
                    retried = true;
                    target.qp = hwenc::cq_retry_qp(target.qp, ratio);
                    self.stop.store(false, Ordering::SeqCst);
                    self.restart_progress();
                }
                Err(Encoded::Problem(p)) => break Err(HwError::Problem(p)),
                Ok(()) => {
                    self.progress(HW_SHARE);
                    break self
                        .check(&partial, codec, cq.then_some(bps))
                        .map_err(|f| HwError::Problem(Problem::CheckFailed(f)));
                }
            }
        };
        let r = r.and_then(|()| {
            finish_file(self.job, self.plan, &partial)
                .and_then(|()| crate::base::private::publish(&partial))
                .and_then(|()| std::fs::rename(&partial, output))
                .map_err(fatal)
        });
        if r.is_err() {
            let _ = std::fs::remove_file(&partial);
        }
        r
    }

    /// The hardware run. With `size_watch`, a run whose output grows faster than
    /// [`hwenc::CQ_RETRY_RATIO`] times the normal method's bitrate (`bps` plus the audio) is
    /// stopped early and reported as [`Encoded::TooBig`].
    fn encode(
        &self,
        args: &[std::ffi::OsString],
        bps: u64,
        size_watch: bool,
    ) -> Result<(), Encoded> {
        let total = self.job.project.timeline.duration().0.max(1) as f64;
        let budget = (bps + self.audio_bps()) as f64;
        let mut size = 0u64;
        let mut too_big = None;
        let watched = self.watch(args, Some(self.work.path()), |line| {
            if let Some(v) = line.trim().strip_prefix("total_size=") {
                size = v.trim().parse().unwrap_or(size);
            }
            if let Some(ProgressEvent::Time(t)) = parse_progress_line(line) {
                self.progress(t.0 as f64 / total * HW_SHARE);
                let secs = t.0 as f64 / 1e6;
                // judged once enough is written: 2 s, or a tenth of the video
                if size_watch && too_big.is_none() && secs >= (total / 1e7).max(2.0) {
                    let ratio = size as f64 * 8.0 / (budget * secs);
                    too_big = Some(ratio);
                    if ratio > hwenc::CQ_RETRY_RATIO {
                        self.stop.store(true, Ordering::SeqCst);
                    }
                }
            }
        });
        if self.cancelled() {
            return Err(Encoded::Problem(Problem::ExitedWithError));
        }
        match watched {
            Ok(Watched::Done(out)) if out.success => Ok(()),
            Ok(Watched::Cancelled) => match too_big {
                Some(r) if self.stop.load(Ordering::SeqCst) => Err(Encoded::TooBig(r)),
                _ => Err(Encoded::Problem(Problem::ExitedWithError)),
            },
            Ok(Watched::Stalled) => Err(Encoded::Problem(Problem::Stalled)),
            Ok(Watched::Done(_)) | Err(_) => Err(Encoded::Problem(Problem::ExitedWithError)),
        }
    }

    /// The audio bitrate of the export (AAC's usual 128 kb/s when not given).
    fn audio_bps(&self) -> u64 {
        match self.plan {
            Plan::Reencode { settings, .. } => settings.audio_bitrate.unwrap_or(128_000),
            Plan::StreamCopy { .. } => 0,
        }
    }

    /// The output check (REQ-063) of the hardware file. `size_of` (constant quality): the
    /// normal method's video bitrate the file size is held to.
    fn check(
        &self,
        partial: &Path,
        codec: Codec,
        size_of: Option<u64>,
    ) -> Result<(), CheckFailure> {
        let Plan::Reencode { settings: s, .. } = self.plan else {
            return Err(CheckFailure::Decode);
        };
        let exp = Expected {
            codec,
            width: s.width,
            height: s.height,
            fps: s.fps,
            hvc1: codec == Codec::Hevc && matches!(s.muxer.as_str(), "mp4" | "mov"),
            audio: Some(ExpectedAudio {
                codec: outcheck::audio_codec_of(&s.aencoder),
                sample_rate: render::SAMPLE_RATE,
                channels: 2,
            }),
        };
        // (a) metadata and packet count
        let meta = self
            .runner
            .run_within(
                &self.job.ffprobe,
                &outcheck::metadata_args(partial),
                self.job.check_timeout,
            )
            .map_err(|_| CheckFailure::Decode)?;
        if !meta.success {
            return Err(CheckFailure::Decode);
        }
        let measured = outcheck::check_metadata(&meta.stdout, &exp)?;
        let packets = measured.packets;
        // (b) the strict full decode and (c) the normal method's graph next to the file run at
        // the same time (REQ-315, ADR-218): the samples need the frame count of the metadata
        // only, and every process still goes through the runner's slots (the cap of three).
        // When the file has the frame count of the plan, only three short runs around the
        // samples are compared (REQ-206); if a sample does not match that way (or the count
        // differs: clips got a frame more from the rounding of the graph) the whole graph is
        // rendered once, which is exact.
        let samples = outcheck::sample_indices(packets);
        let groups = outcheck::neighbours(packets, &samples);
        let expected = render::expected_frames(&self.job.project, s.fps);
        let planned_us =
            (expected as i128 * 1_000_000 * s.fps.den as i128 / s.fps.num as i128) as i64;
        let shares = CheckProgress::default();
        let (decoded, ranges) = std::thread::scope(|scope| {
            let decode = scope.spawn(|| self.strict_decode(partial, packets, &shares));
            let ranges = outcheck::counts_match(packets, expected)
                .then(|| self.check_ranges(partial, s, &groups, planned_us, &measured, &shares));
            (decode.join().unwrap_or(Err(CheckFailure::Decode)), ranges)
        });
        // a file that does not decode is judged first, as when the decode ran first
        decoded?;
        if let Some(Ok(())) = ranges {
            if let Some(bps) = size_of {
                let bytes = std::fs::metadata(partial)
                    .map(|m| m.len())
                    .unwrap_or(u64::MAX);
                outcheck::check_size(bytes, bps + self.audio_bps(), planned_us)?;
            }
            return Ok(());
        }
        self.check_whole(partial, s, packets, &measured, &groups, size_of)
    }

    /// The strict full decode of the file: every packet gives a frame, no error.
    fn strict_decode(
        &self,
        partial: &Path,
        packets: u64,
        shares: &CheckProgress,
    ) -> Result<(), CheckFailure> {
        let mut decoded = None;
        let args = outcheck::decode_args(partial, self.job.threads);
        let watched = self.watch(&args, None, |line| {
            if let Some(n) = outcheck::last_frame(&[line.to_string()]) {
                decoded = Some(n);
                let done = (n as f64 / packets.max(1) as f64).min(1.0);
                shares
                    .decode
                    .store((done * 1000.0) as u32, Ordering::SeqCst);
                self.report(shares);
            }
        });
        match watched {
            Ok(Watched::Done(out)) if out.success && out.stderr.trim().is_empty() => {}
            _ => return Err(CheckFailure::Decode),
        }
        if self.cancelled() {
            return Err(CheckFailure::Decode);
        }
        if decoded != Some(packets) {
            return Err(CheckFailure::FrameCount {
                expected: packets,
                got: decoded.unwrap_or(0),
            });
        }
        shares.decode.store(1000, Ordering::SeqCst);
        self.report(shares);
        Ok(())
    }

    /// Shows the progress of the check: the strict decode is 5% of the whole export and the
    /// three samples 10%, whichever ends first.
    fn report(&self, shares: &CheckProgress) {
        let decode = f64::from(shares.decode.load(Ordering::SeqCst)) / 1000.0;
        let ranges = f64::from(shares.ranges.load(Ordering::SeqCst)) / 3.0;
        self.progress(HW_SHARE + 0.05 * decode + 0.1 * ranges.min(1.0));
    }

    /// The three short comparisons (REQ-206), run at the same time: the duration is the plan's
    /// and every sample matches its range of the normal method.
    fn check_ranges(
        &self,
        partial: &Path,
        s: &crate::format::EncodeSettings,
        groups: &[Vec<u64>; 3],
        planned_us: i64,
        measured: &outcheck::Measured,
        shares: &CheckProgress,
    ) -> Result<(), CheckFailure> {
        outcheck::check_duration(measured, planned_us, s.fps)?;
        let geometry = render::Geometry::of(s);
        let ranges = outcheck::sample_ranges(groups, s.fps);
        let scores: Vec<Result<Vec<[f64; 3]>, CheckFailure>> = std::thread::scope(|scope| {
            let runs: Vec<_> = ranges
                .iter()
                .enumerate()
                .map(|(index, range)| {
                    let geometry = &geometry;
                    scope.spawn(move || {
                        let scores = self.compare(partial, s, geometry, range, index);
                        shares.ranges.fetch_add(1, Ordering::SeqCst);
                        self.report(shares);
                        scores
                    })
                })
                .collect();
            runs.into_iter()
                .map(|run| run.join().unwrap_or(Err(CheckFailure::Decode)))
                .collect()
        });
        // the first sample that fails, in the order of the samples
        for (index, scores) in scores.into_iter().enumerate() {
            outcheck::judge_sample(index, &scores?)?;
        }
        Ok(())
    }

    /// The exact comparison: the whole graph once next to the file (frame count, duration
    /// and samples).
    fn check_whole(
        &self,
        partial: &Path,
        s: &crate::format::EncodeSettings,
        packets: u64,
        measured: &outcheck::Measured,
        groups: &[Vec<u64>; 3],
        size_of: Option<u64>,
    ) -> Result<(), CheckFailure> {
        let geometry = render::Geometry::of(s);
        let graph =
            render::check_graph(&self.job.project, &geometry).map_err(|_| CheckFailure::Decode)?;
        let indices = outcheck::extracted(groups);
        let inputs = match self.job.threads {
            Some(t) => render::threaded_inputs(render::input_args(&graph.inputs), t),
            None => render::input_args(&graph.inputs),
        };
        let dir = self.work.path();
        render::write_command_files(dir, &graph.command_files, &graph.lut_files)
            .map_err(|_| CheckFailure::Decode)?;
        let args = outcheck::compare_args(
            inputs,
            graph.inputs.len(),
            &graph.graph,
            partial,
            &indices,
            s.width,
            s.height,
            self.job.version,
            self.job.threads,
            &dir.join("check-graph.txt"),
        )
        .map_err(|_| CheckFailure::Decode)?;
        let _ = std::fs::remove_file(dir.join(outcheck::PSNR_LOG));
        let mut rendered = None;
        let mut rendered_time = 0i64;
        let watched = self.watch(&args, Some(dir), |line| {
            if let Some(ProgressEvent::Time(t)) = parse_progress_line(line) {
                rendered_time = rendered_time.max(t.0);
            }
            if let Some(n) = outcheck::last_frame(&[line.to_string()]) {
                rendered = Some(n);
                self.progress(HW_SHARE + 0.05 + 0.1 * (n as f64 / packets as f64).min(1.0));
            }
        });
        match watched {
            Ok(Watched::Done(out)) if out.success => {}
            _ => return Err(CheckFailure::Decode),
        }
        if self.cancelled() {
            return Err(CheckFailure::Decode);
        }
        let rendered = rendered.unwrap_or(0);
        if rendered.abs_diff(packets) > 1 {
            return Err(CheckFailure::FrameCount {
                expected: rendered,
                got: packets,
            });
        }
        outcheck::check_duration(measured, rendered_time, s.fps)?;
        if let Some(bps) = size_of {
            let bytes = std::fs::metadata(partial)
                .map(|m| m.len())
                .unwrap_or(u64::MAX);
            outcheck::check_size(bytes, bps + self.audio_bps(), rendered_time)?;
        }
        let log = std::fs::read_to_string(dir.join(outcheck::PSNR_LOG))
            .map_err(|_| CheckFailure::Decode)?;
        outcheck::judge(&indices, &outcheck::parse_psnr_log(&log), groups)
    }

    /// One comparison run (REQ-206): the normal method's graph from the start of `range`
    /// (in its own working folder: its subtitles and camera files are cut to the range)
    /// against the hardware file. Returns the PSNR of the compared frames.
    fn compare(
        &self,
        partial: &Path,
        s: &crate::format::EncodeSettings,
        geometry: &render::Geometry,
        range: &outcheck::SampleRange,
        index: usize,
    ) -> Result<Vec<[f64; 3]>, CheckFailure> {
        let fail = |_: std::io::Error| CheckFailure::Decode;
        let dir = self.work.path().join(format!("check-{index}"));
        std::fs::create_dir_all(&dir).map_err(fail)?;
        let graph = render::check_range_graph(
            &self.job.project,
            geometry,
            range.from,
            outcheck::RANGE_LEN,
            self.plan.track().is_none(),
        )
        .map_err(|_| CheckFailure::Decode)?;
        if let Some(doc) = &graph.subtitles {
            write_subtitles(&dir, doc).map_err(fail)?;
        }
        render::write_command_files(&dir, &graph.graph.command_files, &graph.graph.lut_files)
            .map_err(fail)?;
        let inputs = match self.job.threads {
            Some(t) => render::threaded_inputs(render::input_args(&graph.graph.inputs), t),
            None => render::input_args(&graph.graph.inputs),
        };
        let args = outcheck::compare_range_args(
            inputs,
            graph.graph.inputs.len(),
            &graph.graph.graph,
            partial,
            range,
            s.width,
            s.height,
            self.job.version,
            self.job.threads,
            &dir.join("check-graph.txt"),
        )
        .map_err(fail)?;
        let watched = self.watch(&args, Some(&dir), |_| {});
        let ok = matches!(&watched, Ok(Watched::Done(out)) if out.success);
        let log = std::fs::read_to_string(dir.join(outcheck::PSNR_LOG));
        let _ = std::fs::remove_dir_all(&dir);
        if !ok || self.cancelled() {
            return Err(CheckFailure::Decode);
        }
        Ok(outcheck::parse_psnr_log(&log.map_err(fail)?))
    }
}

/// Writes `subs.ass` and the bundled faces (`fonts/NotoSans-Bold.ttf` and the Arabic and
/// Devanagari ones, REQ-187) into `dir` (ffmpeg's working folder), with the files of the
/// installed fonts the texts use (REQ-165).
pub fn write_subtitles(dir: &Path, doc: &str) -> std::io::Result<()> {
    std::fs::write(dir.join(ass::ASS_FILE), doc)?;
    let fonts = dir.join(ass::FONTS_DIR);
    std::fs::create_dir_all(&fonts)?;
    for (name, bytes) in ass::BUNDLED_FACES {
        let font = fonts.join(name);
        if !font.is_file() {
            std::fs::write(font, bytes)?;
        }
    }
    crate::fonts::copy_used(dir, doc)
}

fn execute(
    job: &ExportJob,
    runner: &dyn Runner,
    args: &[std::ffi::OsString],
    workdir: &Path,
    cancel: &AtomicBool,
    tx: &Sender<ExportEvent>,
) -> Result<(), ExportError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportError::Cancelled);
    }
    let total = job.project.timeline.duration().0.max(1) as f64;
    let mut child = match runner.spawn_in(&job.ffmpeg, args, Some(workdir)) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            return Err(ExportError::Cancelled);
        }
        Err(e) => return Err(ExportError::Io(e.to_string())),
    };
    if let Some(out) = child.take_stdout() {
        for line in BufReader::new(out).lines() {
            if cancel.load(Ordering::SeqCst) {
                child.kill();
                let _ = child.wait();
                return Err(ExportError::Cancelled);
            }
            let Ok(line) = line else { break };
            match parse_progress_line(&line) {
                Some(ProgressEvent::Time(t)) => {
                    let f = (t.0 as f64 / total).clamp(0.0, 1.0) as f32;
                    let _ = tx.send(ExportEvent::Progress(f));
                }
                Some(ProgressEvent::End) => {
                    let _ = tx.send(ExportEvent::Progress(1.0));
                }
                None => {}
            }
        }
    }
    let status = child.wait().map_err(|e| ExportError::Io(e.to_string()))?;
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportError::Cancelled);
    }
    if !status.success {
        return Err(ExportError::Ffmpeg(status.stderr.trim().to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod hw_tests;
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ffmpeg::fake::{FakeRunner, fail, ok};
    use crate::timeline::tests::media;

    pub(crate) fn job(dir: &Path) -> ExportJob {
        let mut p = Project::default();
        let id = p.new_media_id();
        let mut m = media(id.0, 2, true);
        m.path = dir.join("src.mp4");
        std::fs::write(&m.path, b"source").unwrap();
        p.add_media(m.clone());
        p.timeline.append(&m).unwrap();
        ExportJob {
            project: p,
            output: dir.join("out.mp4"),
            ffmpeg: "ffmpeg".into(),
            ffprobe: "ffprobe".into(),
            version: FfVersion::Release(6, 1, 0),
            encoders: ["libx264", "aac"].iter().map(|s| s.to_string()).collect(),
            can_burn_subtitles: true,
            threads: None,
            work_root: dir.join("work"),
            hw: None,
            stall_timeout: STALL_TIMEOUT,
            check_timeout: CHECK_TIMEOUT,
            mode: Default::default(),
            format: Default::default(),
            audio_only: false,
        }
    }

    /// Fake ffmpeg that writes its output file and prints progress.
    fn writer() -> FakeRunner {
        FakeRunner::default().on(|_, a| {
            let out = a.last()?.strip_prefix("file:")?.to_string();
            std::fs::write(out, b"video").ok()?;
            ok("out_time_us=1000000\nprogress=continue\nout_time_us=2000000\nprogress=end\n")
        })
    }

    // covers: REQ-273
    #[test]
    fn a_360_clip_with_ambisonic_sound_is_turned_first_and_a_failure_or_cancel_stops_the_export() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = job(dir.path());
        j.project.media[0].format.spherical = Some(true);
        j.project.media[0].format.channels = Some(4);
        let fake = |ok_pcm: bool| {
            FakeRunner::default().on(move |_, a| {
                if a.iter().any(|x| x == "f32le") {
                    // the pre-pass: four channels of raw samples on stdout
                    return Some(crate::ffmpeg::CmdOutput {
                        success: ok_pcm,
                        stdout: vec![0; 4 * 4 * 480],
                        stderr: "no sound".into(),
                    });
                }
                let out = a.last()?.strip_prefix("file:")?.to_string();
                std::fs::write(out, b"video").ok()?;
                ok("out_time_us=2000000\nprogress=end\n")
            })
        };
        // it works: the pre-pass ran once before the export and its file is read by the graph
        let runner = Arc::new(fake(true));
        let ev = start(j.clone(), runner.clone()).join();
        assert!(
            matches!(ev.last(), Some(ExportEvent::Done { .. })),
            "{ev:?}"
        );
        let calls = runner.calls.lock().unwrap().clone();
        assert!(calls[0].iter().any(|x| x == "f32le"), "{calls:?}");
        assert!(
            calls[1]
                .iter()
                .any(|x| x.contains("spatial-") && x.ends_with(".wav")),
            "{calls:?}"
        );
        // it fails: the export fails with the reason and writes nothing
        std::fs::remove_file(dir.path().join("out.mp4")).unwrap();
        let ev = start(j.clone(), Arc::new(fake(false))).join();
        let Some(ExportEvent::Failed(m)) = ev.last() else {
            panic!("{ev:?}");
        };
        assert!(m.contains("spatial sound") && m.contains("no sound"), "{m}");
        assert!(!dir.path().join("out.mp4").exists());
        // cancelled during the pre-pass
        let (tx, _rx) = std::sync::mpsc::channel();
        let cancel = AtomicBool::new(true);
        let e = run(&j, &fake(true), &cancel, &tx).unwrap_err();
        assert_eq!(e, ExportError::Cancelled);
    }

    // covers: REQ-021
    #[test]
    fn output_equal_to_source_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = job(dir.path());
        j.output = dir.path().join("src.mp4");
        let ev = start(j, Arc::new(writer())).join();
        assert_eq!(
            ev.last(),
            Some(&ExportEvent::Failed(
                ExportError::OutputIsSource.to_string()
            ))
        );
        assert_eq!(
            std::fs::read(dir.path().join("src.mp4")).unwrap(),
            b"source"
        );
        let mut j = job(dir.path());
        j.output = dir.path().join("nope/out.mp4");
        assert_eq!(
            validate_output(&j.project, &j.output),
            Err(ExportError::NoFolder)
        );
    }

    // covers: REQ-021
    #[test]
    fn success_renames_partial() {
        let dir = tempfile::tempdir().unwrap();
        let j = job(dir.path());
        let ev = start(j, Arc::new(writer())).join();
        assert_eq!(ev[0], ExportEvent::Started { stream_copy: true });
        assert!(ev.contains(&ExportEvent::Progress(0.5)));
        assert!(ev.contains(&ExportEvent::Progress(1.0)));
        let out = dir.path().join("out.mp4").canonicalize().unwrap();
        assert_eq!(
            ev.last(),
            Some(&ExportEvent::Done {
                path: out.clone(),
                method: ExportMethod::StreamCopy
            })
        );
        assert_eq!(std::fs::read(&out).unwrap(), b"video");
        assert!(!partial_path(&out).exists());
    }

    // covers: REQ-021
    #[test]
    fn failure_and_cancel_remove_partial() {
        let dir = tempfile::tempdir().unwrap();
        let failing = FakeRunner::default().on(|_, a| {
            let out = a.last()?.strip_prefix("file:")?.to_string();
            std::fs::write(out, b"half").ok()?;
            fail("Conversion failed!")
        });
        let ev = start(job(dir.path()), Arc::new(failing)).join();
        assert!(
            matches!(ev.last(), Some(ExportEvent::Failed(m)) if m.contains("Conversion failed"))
        );
        let partial = partial_path(&dir.path().canonicalize().unwrap().join("out.mp4"));
        assert!(!partial.exists());
        assert!(!dir.path().join("out.mp4").exists());

        let (tx, rx) = channel();
        let cancel = AtomicBool::new(true);
        let r = run(&job(dir.path()), &writer(), &cancel, &tx);
        assert_eq!(r, Err(ExportError::Cancelled));
        assert!(!partial.exists());
        assert!(!dir.path().join("out.mp4").exists());
        drop(rx);

        let none = FakeRunner::default();
        let (tx, _rx) = channel();
        let r = run(&job(dir.path()), &none, &AtomicBool::new(false), &tx);
        assert!(matches!(r, Err(ExportError::Io(_))));
    }

    // covers: REQ-017, REQ-023
    #[test]
    fn subtitles_are_burned_from_a_private_working_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = job(dir.path());
        let tl = j.project.timeline.clone();
        j.project
            .subtitles
            .add_at(&tl, Micros(100_000), "hi {there}", Micros(1))
            .unwrap();
        let seen = Arc::new(std::sync::Mutex::new(None));
        let s2 = Arc::clone(&seen);
        let fake = Arc::new(FakeRunner::default().on(move |prog, a| {
            if prog.ends_with("ffprobe") {
                return ok("0.0\n");
            }
            assert!(
                a.iter()
                    .any(|x| x.contains("subtitles=f=subs.ass:fontsdir=fonts"))
            );
            let out = a.last()?.strip_prefix("file:")?.to_string();
            std::fs::write(out, b"video").ok()?;
            *s2.lock().unwrap() = Some(());
            ok("progress=end\n")
        }));
        let runner: Arc<dyn Runner> = fake.clone();
        let ev = start(j.clone(), runner).join();
        assert_eq!(ev[0], ExportEvent::Started { stream_copy: false });
        assert!(matches!(
            ev.last(),
            Some(ExportEvent::Done {
                method: ExportMethod::Software,
                ..
            })
        ));
        assert!(seen.lock().unwrap().is_some());
        // ffmpeg ran inside a private work folder holding the ASS file and the font
        let cwd = fake.cwds.lock().unwrap().last().cloned().flatten().unwrap();
        assert!(
            cwd.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("job-")
        );
        assert!(cwd.starts_with(&j.work_root));
        assert!(!cwd.exists(), "the working folder is removed afterwards");
        // an ffmpeg without libass cannot export subtitles
        j.can_burn_subtitles = false;
        let (tx, _rx) = channel();
        let r = run(&j, fake.as_ref(), &AtomicBool::new(false), &tx);
        assert_eq!(r, Err(ExportError::NoSubtitleFilter));
        // the files themselves
        let d = tempfile::tempdir().unwrap();
        write_subtitles(d.path(), "doc").unwrap();
        write_subtitles(d.path(), "doc2").unwrap();
        assert_eq!(
            std::fs::read_to_string(d.path().join("subs.ass")).unwrap(),
            "doc2"
        );
        assert_eq!(
            std::fs::read(d.path().join("fonts/NotoSans-Bold.ttf")).unwrap(),
            ass::FONT
        );
    }

    #[test]
    fn keyframes_probed_once_per_source() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = job(dir.path());
        let m = j.project.media[0].clone();
        j.project.timeline.append(&m).unwrap();
        let id = j.project.timeline.clips[1].id;
        j.project
            .timeline
            .trim(
                id,
                Micros(1_000_000),
                Micros(2_000_000),
                m.duration,
                Micros(1),
            )
            .unwrap();
        j.project.timeline.append(&m).unwrap();
        let id = j.project.timeline.clips[2].id;
        j.project
            .timeline
            .trim(
                id,
                Micros(1_000_000),
                Micros(2_000_000),
                m.duration,
                Micros(1),
            )
            .unwrap();
        let r = FakeRunner::default().on(|_, _| ok("0.0\n1.0\n"));
        assert!(matches!(plan(&j, &r).unwrap(), Plan::StreamCopy { .. }));
        assert_eq!(r.calls.lock().unwrap().len(), 1);
        let r = FakeRunner::default().on(|_, _| ok("0.0\n"));
        assert!(matches!(plan(&j, &r).unwrap(), Plan::Reencode { .. }));
        let mut empty = j.clone();
        empty.project.timeline.clips.clear();
        assert!(matches!(plan(&empty, &r), Err(ExportError::Render(_))));
        assert_eq!(
            partial_path(Path::new("/x/a.mp4")),
            PathBuf::from("/x/.a.mp4.videolore-partial")
        );
    }
}
