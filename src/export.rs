// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Export worker: runs ffmpeg in a thread, reports progress over a channel, writes to
//! `<dir>/.<name>.videolore-partial` and renames it on success; deletes it on cancel/error.
//! Sources are only ever read. Burned-in subtitles are written, with the bundled font, to a
//! private temporary folder that is ffmpeg's working folder.

use crate::ass;
use crate::ffmpeg::{
    FfVersion, ProgressEvent, Runner, is_keyframe, parse_progress_line, probe_keyframes,
};
use crate::media::MediaId;
use crate::project::Project;
use crate::render::{self, ExportPaths, Plan};
use crate::time::Micros;
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

#[derive(Debug, Clone, PartialEq)]
pub enum ExportEvent {
    /// Fraction done, 0.0..=1.0.
    Progress(f32),
    /// Whether stream copy was used (sent once, before progress).
    Started {
        stream_copy: bool,
    },
    Done(PathBuf),
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
            Ok(p) => ExportEvent::Done(p),
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
    let plan = render::plan_export(&job.project, &job.encoders, |m, t| {
        let kf = cache
            .entry(m.id)
            .or_insert_with(|| probe_keyframes(runner, &job.ffprobe, &m.path).unwrap_or_default());
        let half_frame = m
            .format
            .fps
            .map(|f| Micros(f.frame_duration().0 / 2))
            .unwrap_or(Micros(1000));
        is_keyframe(kf, t, half_frame)
    })?;
    Ok(plan)
}

/// The export itself (synchronous). Always removes the partial file unless it succeeded.
pub fn run(
    job: &ExportJob,
    runner: &dyn Runner,
    cancel: &AtomicBool,
    tx: &Sender<ExportEvent>,
) -> Result<PathBuf, ExportError> {
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
    let work = crate::scratch::Work::new(&job.work_root).map_err(io)?;
    let list = work.path().join("list.ffconcat");
    let script = work.path().join("graph.txt");
    let paths = ExportPaths {
        output: &partial,
        concat_list: &list,
        graph_script: &script,
    };
    if let Some(doc) = plan.subtitles() {
        write_subtitles(work.path(), doc).map_err(io)?;
    }
    let args = render::export_args(&plan, &paths, job.version, job.threads).map_err(io)?;
    // The partial file is private until it is complete; a crash leaves it listed in the
    // work folder, so the next start removes it.
    work.register(&partial).map_err(io)?;
    drop(crate::scratch::create_private_file(&partial).map_err(io)?);
    let result = execute(job, runner, &args, work.path(), cancel, tx);
    match result {
        Ok(()) => match crate::scratch::publish(&partial)
            .and_then(|()| std::fs::rename(&partial, &output))
        {
            Ok(()) => Ok(output),
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

/// Writes `subs.ass` and `fonts/NotoSans-Bold.ttf` into `dir` (ffmpeg's working folder).
pub fn write_subtitles(dir: &Path, doc: &str) -> std::io::Result<()> {
    std::fs::write(dir.join(ass::ASS_FILE), doc)?;
    let fonts = dir.join(ass::FONTS_DIR);
    std::fs::create_dir_all(&fonts)?;
    let font = fonts.join(ass::FONT_FILE);
    if !font.is_file() {
        std::fs::write(font, ass::FONT)?;
    }
    Ok(())
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
mod tests {
    use super::*;
    use crate::ffmpeg::fake::{FakeRunner, fail, ok};
    use crate::timeline::tests::media;

    fn job(dir: &Path) -> ExportJob {
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
        assert_eq!(ev.last(), Some(&ExportEvent::Done(out.clone())));
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
        assert!(matches!(ev.last(), Some(ExportEvent::Done(_))));
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
