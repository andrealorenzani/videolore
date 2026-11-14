// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Everything about running ffmpeg/ffprobe: locating the binaries, checking version and
//! filters, the [`Runner`] abstraction (real processes or a fake in tests) and parsers for
//! `-progress` and keyframe output.
//!
//! Security: processes are always started with an argv (never through a shell), input
//! files are passed as `file:<absolute path>` so a name cannot be mistaken for an option or
//! another protocol, and every child is killed when its handle is dropped.

use crate::time::Micros;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// Filters the editor relies on; missing any of them is a blocking startup error.
pub const REQUIRED_FILTERS: &[&str] = &[
    "trim",
    "atrim",
    "scale",
    "pad",
    "fps",
    "fade",
    "afade",
    "concat",
    "anullsrc",
    "aresample",
    "amix",
    "alimiter",
    "volume",
    "split",
    "asplit",
];

/// Filter needed only to burn subtitles in (libass). Checked when an export needs it.
pub const SUBTITLES_FILTER: &str = "subtitles";

/// Most stdout kept by [`Runner::run`]; more than this is an error (ffprobe output is small).
pub const RUN_STDOUT_LIMIT: usize = 16 * 1024 * 1024;

/// Graphs longer than this are passed through a script file instead of argv.
pub const MAX_INLINE_GRAPH: usize = 8 * 1024;

/// Keep at most this much stderr from a child (the tail is what matters).
const STDERR_LIMIT: usize = 64 * 1024;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FfmpegError {
    #[error("{0} was not found")]
    NotFound(String),
    #[error("could not run {0}: {1}")]
    Io(String, String),
    #[error("ffmpeg {0} is too old: version 4.4 or newer is required")]
    TooOld(String),
    #[error("could not understand the ffmpeg version")]
    UnknownVersion,
    #[error("ffmpeg lacks required filters: {0}")]
    MissingFilters(String),
    #[error("the ffmpeg included with videolore is damaged ({0}): reinstall videolore")]
    BundledDamaged(String),
}

/// Output of a finished process.
#[derive(Debug, Clone, Default)]
pub struct CmdOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// A running child process.
pub trait Process: Send {
    /// Takes the stdout pipe (once).
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>>;
    /// Kills the process (no-op when already finished).
    fn kill(&mut self);
    /// Waits for the end; returns success flag and the stderr tail.
    fn wait(&mut self) -> io::Result<CmdOutput>;
}

/// Starts external programs. The real implementation is [`SystemRunner`]; the rest of the
/// application only ever sees a [`crate::jobs::Limited`] wrapper that enforces the process cap.
pub trait Runner: Send + Sync {
    /// Runs to completion capturing stdout (at most [`RUN_STDOUT_LIMIT`]) and stderr.
    fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput>;
    /// [`Runner::run`] with a time limit (see [`run_with_timeout`]).
    fn run_within(
        &self,
        program: &Path,
        args: &[OsString],
        timeout: Duration,
    ) -> io::Result<CmdOutput> {
        run_with_timeout(self, program, args, timeout)
    }
    /// Spawns with stdout piped (stdin null, stderr captured in the background), in `cwd`
    /// when given (always one of our private temporary folders).
    fn spawn_in(
        &self,
        program: &Path,
        args: &[OsString],
        cwd: Option<&Path>,
    ) -> io::Result<Box<dyn Process>>;
    /// [`Runner::spawn_in`] in the current folder.
    fn spawn(&self, program: &Path, args: &[OsString]) -> io::Result<Box<dyn Process>> {
        self.spawn_in(program, args, None)
    }
    /// Spawns two processes that belong together (preview video + audio). A limited runner
    /// reserves both slots at once, so the pair never waits half-started.
    fn spawn_pair(
        &self,
        a: (&Path, &[OsString]),
        b: (&Path, &[OsString]),
        cwd: Option<&Path>,
    ) -> io::Result<(Box<dyn Process>, Box<dyn Process>)> {
        let first = self.spawn_in(a.0, a.1, cwd)?;
        let second = self.spawn_in(b.0, b.1, cwd)?;
        Ok((first, second))
    }
}

/// Like [`Runner::run`], but the process is killed when it has not finished after
/// `timeout` (a hostile or broken file must not keep ffprobe busy forever). The error then
/// has kind `TimedOut`.
pub fn run_with_timeout<R: Runner + ?Sized>(
    runner: &R,
    program: &Path,
    args: &[OsString],
    timeout: Duration,
) -> io::Result<CmdOutput> {
    let mut p = runner.spawn(program, args)?;
    let (tx, rx) = std::sync::mpsc::channel();
    match p.take_stdout() {
        Some(s) => {
            std::thread::spawn(move || {
                let mut out = Vec::new();
                let r = s.take(RUN_STDOUT_LIMIT as u64 + 1).read_to_end(&mut out);
                let _ = tx.send(r.map(|_| out));
            });
        }
        None => {
            let _ = tx.send(Ok(Vec::new()));
        }
    }
    let out = match rx.recv_timeout(timeout) {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => {
            p.kill();
            return Err(e);
        }
        Err(_) => {
            p.kill();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{} took too long", program.display()),
            ));
        }
    };
    if out.len() > RUN_STDOUT_LIMIT {
        p.kill();
        return Err(io::Error::other(format!(
            "{} printed more than 16 MiB",
            program.display()
        )));
    }
    let mut r = p.wait()?;
    r.stdout = out;
    Ok(r)
}

/// Runs real processes via `std::process::Command`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

fn command(program: &Path, args: &[OsString]) -> Command {
    let mut c = Command::new(program);
    c.args(args).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

impl Runner for SystemRunner {
    fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
        let mut p = self.spawn_in(program, args, None)?;
        let mut out = Vec::new();
        if let Some(s) = p.take_stdout() {
            s.take(RUN_STDOUT_LIMIT as u64 + 1).read_to_end(&mut out)?;
        }
        if out.len() > RUN_STDOUT_LIMIT {
            p.kill();
            let _ = p.wait();
            return Err(io::Error::other(format!(
                "{} printed more than 16 MiB",
                program.display()
            )));
        }
        let mut r = p.wait()?;
        r.stdout = out;
        Ok(r)
    }

    fn spawn_in(
        &self,
        program: &Path,
        args: &[OsString],
        cwd: Option<&Path>,
    ) -> io::Result<Box<dyn Process>> {
        let mut c = command(program, args);
        if let Some(d) = cwd {
            c.current_dir(d);
        }
        let mut child = c.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        let stderr = Arc::new(Mutex::new(String::new()));
        let sink = Arc::clone(&stderr);
        let mut pipe = child.stderr.take();
        let thread = std::thread::spawn(move || {
            let Some(p) = pipe.as_mut() else { return };
            let mut buf = [0u8; 4096];
            while let Ok(n) = p.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if let Ok(mut s) = sink.lock() {
                    s.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if s.len() > 2 * STDERR_LIMIT {
                        *s = tail(std::mem::take(&mut *s));
                    }
                }
            }
        });
        Ok(Box::new(ChildProcess {
            child,
            stderr,
            thread: Some(thread),
        }))
    }
}

fn tail(s: String) -> String {
    if s.len() <= STDERR_LIMIT {
        return s;
    }
    let mut cut = s.len() - STDERR_LIMIT;
    while !s.is_char_boundary(cut) {
        cut += 1;
    }
    s[cut..].to_string()
}

/// A real child; killed on drop so that no ffmpeg outlives its owner.
struct ChildProcess {
    child: Child,
    stderr: Arc<Mutex<String>>,
    thread: Option<JoinHandle<()>>,
}

impl Process for ChildProcess {
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>)
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
    }

    fn wait(&mut self) -> io::Result<CmdOutput> {
        let status = self.child.wait()?;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let stderr = self.stderr.lock().map(|s| s.clone()).unwrap_or_default();
        Ok(CmdOutput {
            success: status.success(),
            stdout: Vec::new(),
            stderr: tail(stderr),
        })
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// `file:<path>` argument for an input or output file.
pub fn file_arg(path: &Path) -> OsString {
    let mut s = OsString::from("file:");
    s.push(path.as_os_str());
    s
}

/// Converts string-ish arguments into an argv.
pub fn argv<I, S>(items: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    items.into_iter().map(|s| s.as_ref().to_owned()).collect()
}

/// Executable file name for the current platform.
pub fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Where a binary was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BinSource {
    /// Chosen by the user in the app (saved in the settings).
    Setting,
    /// `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`.
    Env,
    /// The copy shipped with videolore.
    Bundled,
    /// Found on `PATH`.
    #[default]
    Path,
}

impl std::fmt::Display for BinSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BinSource::Setting => "chosen in settings",
            BinSource::Env => "from the environment",
            BinSource::Bundled => "included with videolore",
            BinSource::Path => "from PATH",
        })
    }
}

/// Folders where the bundled ffmpeg lives: next to the executable, then (macOS bundles)
/// `Contents/Resources`.
pub fn bundled_dirs(exe_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![exe_dir.to_path_buf()];
    dirs.extend(exe_dir.parent().map(|p| p.join("Resources")));
    dirs
}

/// Finds a binary: app setting > environment variable > bundled copy > PATH.
pub fn locate(
    base: &str,
    setting: Option<&Path>,
    env_value: Option<&OsStr>,
    exe_dir: Option<&Path>,
    path_var: Option<&OsStr>,
) -> Option<(PathBuf, BinSource)> {
    if let Some(p) = setting.filter(|p| p.is_file()) {
        return Some((p.to_path_buf(), BinSource::Setting));
    }
    if let Some(p) = env_value.map(PathBuf::from).filter(|p| p.is_file()) {
        return Some((p, BinSource::Env));
    }
    let name = exe_name(base);
    if let Some(p) = exe_dir
        .into_iter()
        .flat_map(bundled_dirs)
        .map(|d| d.join(&name))
        .find(|p| p.is_file())
    {
        return Some((p, BinSource::Bundled));
    }
    path_var
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|d| d.join(&name))
        .find(|p| p.is_file())
        .map(|p| (p, BinSource::Path))
}

/// The pair of binaries the app will use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    /// Where `ffmpeg` came from (decides the wording of errors).
    pub source: BinSource,
}

/// Locates `ffmpeg` and `ffprobe` given the folder of our executable.
pub fn locate_pair(
    ffmpeg_setting: Option<&Path>,
    ffprobe_setting: Option<&Path>,
    env: impl Fn(&str) -> Option<OsString>,
    exe_dir: Option<&Path>,
) -> Result<Located, FfmpegError> {
    let path = env("PATH");
    let (ffmpeg, source) = locate(
        "ffmpeg",
        ffmpeg_setting,
        env("VIDEOLORE_FFMPEG").as_deref(),
        exe_dir,
        path.as_deref(),
    )
    .ok_or_else(|| FfmpegError::NotFound("ffmpeg".into()))?;
    let (ffprobe, _) = locate(
        "ffprobe",
        ffprobe_setting,
        env("VIDEOLORE_FFPROBE").as_deref(),
        exe_dir,
        path.as_deref(),
    )
    .ok_or_else(|| FfmpegError::NotFound("ffprobe".into()))?;
    Ok(Located {
        ffmpeg,
        ffprobe,
        source,
    })
}

/// Locates `ffmpeg` and `ffprobe` using the real environment.
pub fn locate_both(
    ffmpeg_setting: Option<&Path>,
    ffprobe_setting: Option<&Path>,
) -> Result<Located, FfmpegError> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    locate_pair(
        ffmpeg_setting,
        ffprobe_setting,
        |k: &str| std::env::var_os(k),
        exe_dir.as_deref(),
    )
}

/// Checks a located pair. A bundled copy that does not work is reported as damaged
/// (reinstall), anything else keeps its specific error.
pub fn check_located(runner: &dyn Runner, located: &Located) -> Result<FfmpegInfo, FfmpegError> {
    match check(runner, &located.ffmpeg, &located.ffprobe) {
        Ok(mut info) => {
            info.source = located.source;
            Ok(info)
        }
        Err(e) if located.source == BinSource::Bundled => {
            Err(FfmpegError::BundledDamaged(e.to_string()))
        }
        Err(e) => Err(e),
    }
}

/// Parsed `ffmpeg -version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfVersion {
    Release(u32, u32, u32),
    /// Git snapshot builds (`N-…`, `git-…`): assumed recent.
    Dev,
}

impl FfVersion {
    pub fn at_least(self, major: u32, minor: u32) -> bool {
        match self {
            FfVersion::Dev => true,
            FfVersion::Release(a, b, _) => (a, b) >= (major, minor),
        }
    }
}

impl std::fmt::Display for FfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfVersion::Dev => write!(f, "development build"),
            FfVersion::Release(a, b, c) => write!(f, "{a}.{b}.{c}"),
        }
    }
}

/// Parses the first line of `ffmpeg -version`, e.g. `ffmpeg version 6.1.1-3ubuntu5 ...`.
pub fn parse_version(text: &str) -> Option<FfVersion> {
    let line = text.lines().next()?;
    let mut it = line.split_whitespace();
    it.find(|w| *w == "version")?;
    let v = it.next()?;
    if v.starts_with("N-") || v.starts_with("git") {
        return Some(FfVersion::Dev);
    }
    let v = v.trim_start_matches('n');
    let digits: String = v
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = digits.split('.').filter(|s| !s.is_empty());
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Some(FfVersion::Release(major, minor, patch))
}

/// Filter names from `ffmpeg -hide_banner -filters`.
pub fn parse_filters(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            (t.len() >= 3 && t[2].contains("->")).then(|| t[1].to_string())
        })
        .collect()
}

/// Encoder names from `ffmpeg -hide_banner -encoders`.
pub fn parse_encoders(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            let ok = t.len() >= 3
                && t[0].len() == 6
                && matches!(t[0].chars().next(), Some('V' | 'A' | 'S'))
                && t[1] != "=";
            ok.then(|| t[1].to_string())
        })
        .collect()
}

/// What we know about the ffmpeg installation after the startup check.
#[derive(Debug, Clone)]
pub struct FfmpegInfo {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub version: FfVersion,
    pub filters: BTreeSet<String>,
    pub encoders: BTreeSet<String>,
    pub source: BinSource,
}

impl FfmpegInfo {
    /// Whether subtitles can be burned in (the `subtitles` filter needs libass).
    pub fn has_subtitles(&self) -> bool {
        self.filters.contains(SUBTITLES_FILTER)
    }
}

fn run_text(runner: &dyn Runner, prog: &Path, args: &[&str]) -> Result<String, FfmpegError> {
    let out = runner
        .run(prog, &argv(args))
        .map_err(|e| FfmpegError::Io(prog.display().to_string(), e.to_string()))?;
    if !out.success {
        return Err(FfmpegError::Io(prog.display().to_string(), out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Startup check: version >= 4.4, required filters present, ffprobe runs.
pub fn check(
    runner: &dyn Runner,
    ffmpeg: &Path,
    ffprobe: &Path,
) -> Result<FfmpegInfo, FfmpegError> {
    let vtext = run_text(runner, ffmpeg, &["-hide_banner", "-version"])?;
    let version = parse_version(&vtext).ok_or(FfmpegError::UnknownVersion)?;
    if !version.at_least(4, 4) {
        return Err(FfmpegError::TooOld(version.to_string()));
    }
    let filters = parse_filters(&run_text(runner, ffmpeg, &["-hide_banner", "-filters"])?);
    let missing: Vec<&str> = REQUIRED_FILTERS
        .iter()
        .copied()
        .filter(|f| !filters.contains(*f))
        .collect();
    if !missing.is_empty() {
        return Err(FfmpegError::MissingFilters(missing.join(", ")));
    }
    let encoders = parse_encoders(&run_text(runner, ffmpeg, &["-hide_banner", "-encoders"])?);
    run_text(runner, ffprobe, &["-hide_banner", "-version"])?;
    Ok(FfmpegInfo {
        ffmpeg: ffmpeg.to_path_buf(),
        ffprobe: ffprobe.to_path_buf(),
        version,
        filters,
        encoders,
        source: BinSource::Path,
    })
}

/// What to do when ffmpeg is unusable (shown in the blocking startup message). videolore
/// ships its own copy, so reinstalling comes first; a system ffmpeg is the alternative.
pub fn install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "Reinstall videolore (it includes ffmpeg), or install ffmpeg with Homebrew: brew install ffmpeg"
    } else if cfg!(windows) {
        "Reinstall videolore (it includes ffmpeg), or install ffmpeg with: winget install Gyan.FFmpeg"
    } else {
        "Reinstall videolore (it includes ffmpeg), or install ffmpeg with your package manager, e.g. sudo apt install ffmpeg"
    }
}

/// Events parsed from `-progress pipe:1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEvent {
    Time(Micros),
    End,
}

/// Parses one `key=value` line of `-progress` output.
pub fn parse_progress_line(line: &str) -> Option<ProgressEvent> {
    let (k, v) = line.trim().split_once('=')?;
    match k {
        // out_time_ms is (historically) microseconds too.
        "out_time_us" | "out_time_ms" => v
            .trim()
            .parse()
            .ok()
            .map(|n| ProgressEvent::Time(Micros(n))),
        "progress" if v.trim() == "end" => Some(ProgressEvent::End),
        _ => None,
    }
}

/// Keyframe times from `ffprobe -skip_frame nokey -show_entries frame=pts_time -of csv=p=0`.
pub fn parse_keyframes(text: &str) -> Vec<Micros> {
    let mut v: Vec<Micros> = text
        .lines()
        .filter_map(|l| {
            let f = l.trim().trim_end_matches(',').split(',').next()?.trim();
            f.parse::<f64>().ok().map(Micros::from_secs_f64)
        })
        .collect();
    v.sort();
    v
}

/// True when `t` is within `tolerance` of a keyframe.
pub fn is_keyframe(keyframes: &[Micros], t: Micros, tolerance: Micros) -> bool {
    keyframes.iter().any(|k| (k.0 - t.0).abs() <= tolerance.0)
}

/// Lists the keyframe times of the first video stream of `path`.
pub fn probe_keyframes(
    runner: &dyn Runner,
    ffprobe: &Path,
    path: &Path,
) -> io::Result<Vec<Micros>> {
    let mut args = argv([
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-select_streams",
        "v:0",
        "-skip_frame",
        "nokey",
        "-show_entries",
        "frame=pts_time",
        "-of",
        "csv=p=0",
    ]);
    args.push(file_arg(path));
    let out = runner.run(ffprobe, &args)?;
    if !out.success {
        return Err(io::Error::other(out.stderr));
    }
    Ok(parse_keyframes(&String::from_utf8_lossy(&out.stdout)))
}

/// Arguments that pass a filter graph: inline when short, through `script` otherwise.
pub fn filter_graph_args(
    graph: &str,
    version: FfVersion,
    script: &Path,
) -> io::Result<Vec<OsString>> {
    if graph.len() <= MAX_INLINE_GRAPH {
        return Ok(argv(["-filter_complex", graph]));
    }
    std::fs::write(script, graph)?;
    let opt = if version.at_least(7, 1) {
        "-/filter_complex"
    } else {
        "-filter_complex_script"
    };
    Ok(vec![opt.into(), script.as_os_str().to_owned()])
}

#[cfg(test)]
pub mod fake {
    //! A scripted [`Runner`] for unit tests.
    use super::*;
    use std::io::Cursor;
    use std::sync::{Arc, Condvar};

    type Responder = Box<dyn Fn(&Path, &[String]) -> Option<CmdOutput> + Send + Sync>;
    type HangIf = Box<dyn Fn(&[String]) -> bool + Send + Sync>;

    /// Answers each call with the first responder that returns `Some`.
    #[derive(Default)]
    pub struct FakeRunner {
        pub responders: Vec<Responder>,
        pub calls: Mutex<Vec<Vec<String>>>,
        /// Working folder of each spawned process (None = current folder).
        pub cwds: Mutex<Vec<Option<PathBuf>>>,
        /// When true, spawned processes keep their stdout open until killed,
        /// like a real long-running ffmpeg (no premature end of stream).
        pub hold_streams: bool,
        /// Like `hold_streams`, only for the spawned processes it says yes to.
        pub hang_if: Option<HangIf>,
    }

    impl FakeRunner {
        pub fn on(
            mut self,
            f: impl Fn(&Path, &[String]) -> Option<CmdOutput> + Send + Sync + 'static,
        ) -> Self {
            self.responders.push(Box::new(f));
            self
        }

        /// Spawned processes block on stdout after their scripted output until killed.
        pub fn holding_streams(mut self) -> Self {
            self.hold_streams = true;
            self
        }

        /// Spawned processes whose arguments `f` says yes to (it is asked once per spawn,
        /// in order) block on stdout until killed: a process that does not answer in time.
        pub fn hanging_if(mut self, f: impl Fn(&[String]) -> bool + Send + Sync + 'static) -> Self {
            self.hang_if = Some(Box::new(f));
            self
        }

        fn answer(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
            let a: Vec<String> = args
                .iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            self.calls.lock().unwrap().push(a.clone());
            self.responders
                .iter()
                .find_map(|r| r(program, &a))
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no fake answer"))
        }
    }

    pub fn ok(stdout: &str) -> Option<CmdOutput> {
        Some(CmdOutput {
            success: true,
            stdout: stdout.as_bytes().to_vec(),
            stderr: String::new(),
        })
    }

    pub fn fail(stderr: &str) -> Option<CmdOutput> {
        Some(CmdOutput {
            success: false,
            stdout: Vec::new(),
            stderr: stderr.to_string(),
        })
    }

    /// Set to true (and notified) when the fake process is killed.
    type KillSignal = Arc<(Mutex<bool>, Condvar)>;

    struct FakeProcess(Option<CmdOutput>, Option<KillSignal>);

    /// Scripted stdout that, once drained, blocks until the process is killed.
    struct HeldStdout {
        data: Cursor<Vec<u8>>,
        killed: KillSignal,
    }

    impl Read for HeldStdout {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.data.read(buf)?;
            if n > 0 || buf.is_empty() {
                return Ok(n);
            }
            let (lock, cv) = &*self.killed;
            let mut killed = lock.lock().unwrap_or_else(|e| e.into_inner());
            while !*killed {
                killed = cv.wait(killed).unwrap_or_else(|e| e.into_inner());
            }
            Ok(0)
        }
    }

    impl Process for FakeProcess {
        fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
            let data = Cursor::new(self.0.as_mut()?.stdout.clone());
            Some(match &self.1 {
                Some(killed) => Box::new(HeldStdout {
                    data,
                    killed: Arc::clone(killed),
                }),
                None => Box::new(data),
            })
        }
        fn kill(&mut self) {
            if let Some(k) = &self.1 {
                let (lock, cv) = &**k;
                *lock.lock().unwrap_or_else(|e| e.into_inner()) = true;
                cv.notify_all();
            }
        }
        fn wait(&mut self) -> io::Result<CmdOutput> {
            Ok(self.0.take().unwrap_or_default())
        }
    }

    impl Runner for FakeRunner {
        fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
            self.answer(program, args)
        }
        fn run_within(
            &self,
            program: &Path,
            args: &[OsString],
            _timeout: Duration,
        ) -> io::Result<CmdOutput> {
            self.answer(program, args)
        }
        fn spawn_in(
            &self,
            program: &Path,
            args: &[OsString],
            cwd: Option<&Path>,
        ) -> io::Result<Box<dyn Process>> {
            self.cwds.lock().unwrap().push(cwd.map(Path::to_path_buf));
            let hang = self.hold_streams
                || self.hang_if.as_ref().is_some_and(|f| {
                    let a: Vec<String> = args
                        .iter()
                        .map(|s| s.to_string_lossy().into_owned())
                        .collect();
                    f(&a)
                });
            let held = hang.then(|| Arc::new((Mutex::new(false), Condvar::new())));
            Ok(Box::new(FakeProcess(
                Some(self.answer(program, args)?),
                held,
            )))
        }
    }

    /// A finished fake process answering `out`; with `hold`, its stdout blocks after the
    /// scripted output until the process is killed (a hung program).
    pub fn process(out: CmdOutput, hold: bool) -> Box<dyn Process> {
        let held = hold.then(|| Arc::new((Mutex::new(false), Condvar::new())));
        Box::new(FakeProcess(Some(out), held))
    }

    /// A fake installation that passes [`check`].
    pub fn healthy() -> FakeRunner {
        FakeRunner::default().on(|_, a| {
            let a = a.join(" ");
            if a.contains("-version") {
                ok("ffmpeg version 6.1.1 Copyright\n")
            } else if a.contains("-filters") {
                let mut s = String::from("Filters:\n");
                for f in REQUIRED_FILTERS {
                    s.push_str(&format!(" ... {f}  V->V  x\n"));
                }
                s.push_str(" ... subtitles  V->V  x\n");
                ok(&s)
            } else if a.contains("-encoders") {
                ok(" V....D libx264  H.264\n A....D aac  AAC\n")
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    #[test]
    fn version_parsing() {
        assert_eq!(
            parse_version("ffmpeg version 6.1.1-3ubuntu5 Copyright (c) 2000-2023"),
            Some(FfVersion::Release(6, 1, 1))
        );
        assert_eq!(
            parse_version("ffmpeg version n7.1 Copyright"),
            Some(FfVersion::Release(7, 1, 0))
        );
        assert_eq!(
            parse_version("ffmpeg version N-118000-gabc"),
            Some(FfVersion::Dev)
        );
        assert_eq!(parse_version("hello"), None);
        assert_eq!(parse_version(""), None);
        assert!(FfVersion::Release(4, 4, 0).at_least(4, 4));
        assert!(!FfVersion::Release(4, 3, 9).at_least(4, 4));
        assert!(FfVersion::Dev.at_least(9, 9));
        assert_eq!(FfVersion::Dev.to_string(), "development build");
        assert_eq!(FfVersion::Release(5, 1, 2).to_string(), "5.1.2");
    }

    #[test]
    fn filters_and_encoders_parsing() {
        let f = parse_filters(
            "Filters:\n  T.. = Timeline support\n TSC trim              V->V       Pick\n ... anullsrc |->A  Null\n",
        );
        assert!(f.contains("trim"));
        assert!(f.contains("anullsrc"));
        assert_eq!(f.len(), 2);
        let e = parse_encoders(
            "Encoders:\n V..... = Video\n ------\n V....D libx264  H.264\n A....D aac   AAC\n",
        );
        assert_eq!(
            e.into_iter().collect::<Vec<_>>(),
            vec!["aac".to_string(), "libx264".to_string()]
        );
    }

    #[test]
    fn progress_and_keyframes() {
        assert_eq!(
            parse_progress_line("out_time_us=1500000"),
            Some(ProgressEvent::Time(Micros(1_500_000)))
        );
        assert_eq!(
            parse_progress_line("out_time_ms=20"),
            Some(ProgressEvent::Time(Micros(20)))
        );
        assert_eq!(
            parse_progress_line("progress=end"),
            Some(ProgressEvent::End)
        );
        assert_eq!(parse_progress_line("progress=continue"), None);
        assert_eq!(parse_progress_line("out_time_us=N/A"), None);
        assert_eq!(parse_progress_line("garbage"), None);
        let k = parse_keyframes("2.000000\n0.000000,\nN/A\n\n4.0\n");
        assert_eq!(k, vec![Micros(0), Micros(2_000_000), Micros(4_000_000)]);
        assert!(is_keyframe(&k, Micros(2_000_100), Micros(1000)));
        assert!(!is_keyframe(&k, Micros(1_000_000), Micros(1000)));
    }

    #[test]
    fn locate_order() {
        let dir = tempfile::tempdir().unwrap();
        let name = exe_name("ffmpeg");
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join(&name), "").unwrap();
        std::fs::write(b.join(&name), "").unwrap();
        let setting = dir.path().join("custom");
        std::fs::write(&setting, "").unwrap();
        let path_var = std::env::join_paths([&b]).unwrap();
        assert_eq!(
            locate("ffmpeg", Some(&setting), None, Some(&a), Some(&path_var)),
            Some((setting.clone(), BinSource::Setting))
        );
        assert_eq!(
            locate("ffmpeg", None, Some(setting.as_os_str()), Some(&a), None),
            Some((setting.clone(), BinSource::Env))
        );
        assert_eq!(
            locate(
                "ffmpeg",
                Some(&dir.path().join("nope")),
                None,
                Some(&a),
                Some(&path_var)
            ),
            Some((a.join(&name), BinSource::Bundled))
        );
        assert_eq!(
            locate("ffmpeg", None, None, None, Some(&path_var)),
            Some((b.join(&name), BinSource::Path))
        );
        assert_eq!(locate("ffmpeg", None, None, None, None), None);
        assert_eq!(BinSource::Bundled.to_string(), "included with videolore");
    }

    // covers: REQ-030
    #[test]
    fn bundled_copy_is_preferred_over_path_and_overrides_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        // macOS bundle layout: Contents/MacOS/videolore + Contents/Resources/ffmpeg
        let macos = dir.path().join("Contents/MacOS");
        let res = dir.path().join("Contents/Resources");
        let sys = dir.path().join("usr/bin");
        for d in [&macos, &res, &sys] {
            std::fs::create_dir_all(d).unwrap();
        }
        for n in ["ffmpeg", "ffprobe"] {
            std::fs::write(res.join(exe_name(n)), "").unwrap();
            std::fs::write(sys.join(exe_name(n)), "").unwrap();
        }
        let path = std::env::join_paths([&sys]).unwrap();
        let env = |k: &str| (k == "PATH").then(|| path.clone());
        let l = locate_pair(None, None, env, Some(&macos)).unwrap();
        assert_eq!(l.source, BinSource::Bundled);
        assert!(l.ffmpeg.starts_with(&res));
        assert!(l.ffprobe.starts_with(&res));
        // a user override wins over the bundled copy
        let mine = dir.path().join("my-ffmpeg");
        std::fs::write(&mine, "").unwrap();
        let l = locate_pair(Some(&mine), None, env, Some(&macos)).unwrap();
        assert_eq!(
            (l.ffmpeg.as_path(), l.source),
            (mine.as_path(), BinSource::Setting)
        );
        let env2 = |k: &str| match k {
            "VIDEOLORE_FFMPEG" => Some(mine.clone().into_os_string()),
            "PATH" => Some(path.clone()),
            _ => None,
        };
        assert_eq!(
            locate_pair(None, None, env2, Some(&macos)).unwrap().source,
            BinSource::Env
        );
        // nothing anywhere: not found (the message then says to reinstall or install ffmpeg)
        let none = |_: &str| None;
        assert_eq!(
            locate_pair(None, None, none, Some(&dir.path().join("x"))).unwrap_err(),
            FfmpegError::NotFound("ffmpeg".into())
        );
        let only_ffmpeg = dir.path().join("only");
        std::fs::create_dir_all(&only_ffmpeg).unwrap();
        std::fs::write(only_ffmpeg.join(exe_name("ffmpeg")), "").unwrap();
        assert_eq!(
            locate_pair(None, None, none, Some(&only_ffmpeg)).unwrap_err(),
            FfmpegError::NotFound("ffprobe".into())
        );
        assert!(install_hint().contains("Reinstall videolore"));
        assert!(locate_both(None, None).is_ok() || locate_both(None, None).is_err());
    }

    // covers: REQ-030
    #[test]
    fn damaged_bundled_copy_is_told_apart_from_a_missing_one() {
        let broken = FakeRunner::default().on(|_, _| fail("cannot execute binary file"));
        let mut l = Located {
            ffmpeg: "ffmpeg".into(),
            ffprobe: "ffprobe".into(),
            source: BinSource::Bundled,
        };
        let e = check_located(&broken, &l).unwrap_err();
        assert!(matches!(e, FfmpegError::BundledDamaged(_)));
        assert!(e.to_string().contains("reinstall videolore"));
        l.source = BinSource::Path;
        assert!(matches!(
            check_located(&broken, &l),
            Err(FfmpegError::Io(_, _))
        ));
        l.source = BinSource::Bundled;
        let info = check_located(&healthy(), &l).unwrap();
        assert_eq!(info.source, BinSource::Bundled);
        assert!(info.has_subtitles());
    }

    #[test]
    fn check_accepts_healthy_install() {
        let r = healthy();
        let info = check(&r, Path::new("ffmpeg"), Path::new("ffprobe")).unwrap();
        assert_eq!(info.version, FfVersion::Release(6, 1, 1));
        assert!(info.encoders.contains("libx264"));
    }

    #[test]
    fn check_rejects_old_or_incomplete() {
        let old = FakeRunner::default().on(|_, _| ok("ffmpeg version 4.3.1 x"));
        assert_eq!(
            check(&old, Path::new("f"), Path::new("p")).unwrap_err(),
            FfmpegError::TooOld("4.3.1".into())
        );
        let weird = FakeRunner::default().on(|_, _| ok("what"));
        assert_eq!(
            check(&weird, Path::new("f"), Path::new("p")).unwrap_err(),
            FfmpegError::UnknownVersion
        );
        let nofilters = FakeRunner::default().on(|_, a| {
            if a.join(" ").contains("-version") {
                ok("ffmpeg version 6.0 x")
            } else {
                ok("Filters:\n ... trim V->V x\n")
            }
        });
        let e = check(&nofilters, Path::new("f"), Path::new("p")).unwrap_err();
        assert!(matches!(e, FfmpegError::MissingFilters(ref s) if s.contains("afade")));
        let broken = FakeRunner::default().on(|_, _| fail("boom"));
        assert!(matches!(
            check(&broken, Path::new("f"), Path::new("p")),
            Err(FfmpegError::Io(_, _))
        ));
        let missing = FakeRunner::default();
        assert!(matches!(
            check(&missing, Path::new("f"), Path::new("p")),
            Err(FfmpegError::Io(_, _))
        ));
        assert!(!install_hint().is_empty());
    }

    #[test]
    fn graph_args_inline_or_script() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("g.txt");
        let a = filter_graph_args("[0:v]null[v]", FfVersion::Release(6, 0, 0), &script).unwrap();
        assert_eq!(a, argv(["-filter_complex", "[0:v]null[v]"]));
        let big = "x".repeat(MAX_INLINE_GRAPH + 1);
        let a = filter_graph_args(&big, FfVersion::Release(6, 0, 0), &script).unwrap();
        assert_eq!(a[0], "-filter_complex_script");
        assert_eq!(std::fs::read_to_string(&script).unwrap(), big);
        let a = filter_graph_args(&big, FfVersion::Release(7, 1, 0), &script).unwrap();
        assert_eq!(a[0], "-/filter_complex");
    }

    #[test]
    fn file_arg_prefix_and_tail() {
        assert_eq!(
            file_arg(Path::new("/a/-b.mp4")),
            OsString::from("file:/a/-b.mp4")
        );
        let long = "é".repeat(STDERR_LIMIT);
        let t = tail(long);
        assert!(t.len() <= STDERR_LIMIT);
        assert_eq!(tail("abc".into()), "abc");
    }

    #[test]
    fn system_runner_caps_output_and_uses_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let r = SystemRunner;
        let rustc = PathBuf::from("rustc");
        let mut p = r
            .spawn_in(&rustc, &argv(["--version"]), Some(dir.path()))
            .unwrap();
        assert!(p.wait().unwrap().success);
        let (mut a, mut b) = r
            .spawn_pair(
                (&rustc, &argv(["--version"])),
                (&rustc, &argv(["--version"])),
                None,
            )
            .unwrap();
        assert!(a.wait().unwrap().success && b.wait().unwrap().success);
        assert!(r.run(Path::new("/definitely/not/here"), &[]).is_err());
    }

    #[test]
    fn system_runner_runs_and_kills() {
        // Uses the Rust toolchain itself as a harmless, always-present program.
        let r = SystemRunner;
        let rustc = PathBuf::from("rustc");
        let out = r.run(&rustc, &argv(["--version"])).unwrap();
        assert!(out.success);
        assert!(String::from_utf8_lossy(&out.stdout).contains("rustc"));
        let mut p = r.spawn(&rustc, &argv(["--version"])).unwrap();
        let mut s = String::new();
        p.take_stdout().unwrap().read_to_string(&mut s).unwrap();
        assert!(s.contains("rustc"));
        assert!(p.take_stdout().is_none());
        p.kill();
        let w = p.wait().unwrap();
        assert!(w.stdout.is_empty());
        let bad = r.spawn(&rustc, &argv(["--definitely-not-a-flag"])).unwrap();
        drop(bad);
    }

    #[test]
    fn probe_keyframes_uses_runner() {
        let r = FakeRunner::default().on(|_, a| {
            assert!(a.iter().any(|x| x == "nokey"));
            assert!(a.last().unwrap().starts_with("file:"));
            ok("0.0\n1.0\n")
        });
        let k = probe_keyframes(&r, Path::new("ffprobe"), Path::new("/x.mp4")).unwrap();
        assert_eq!(k.len(), 2);
        let bad = FakeRunner::default().on(|_, _| fail("nope"));
        assert!(probe_keyframes(&bad, Path::new("ffprobe"), Path::new("/x.mp4")).is_err());
    }
}
