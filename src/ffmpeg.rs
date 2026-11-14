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
];

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

/// Starts external programs. The real implementation is [`SystemRunner`].
pub trait Runner: Send + Sync {
    /// Runs to completion capturing stdout and stderr.
    fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput>;
    /// Spawns with stdout piped (stdin null, stderr captured in the background).
    fn spawn(&self, program: &Path, args: &[OsString]) -> io::Result<Box<dyn Process>>;
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
        let out = command(program, args).output()?;
        Ok(CmdOutput {
            success: out.status.success(),
            stdout: out.stdout,
            stderr: tail(String::from_utf8_lossy(&out.stderr).into_owned()),
        })
    }

    fn spawn(&self, program: &Path, args: &[OsString]) -> io::Result<Box<dyn Process>> {
        let mut child = command(program, args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
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

/// Finds a binary: app setting > environment variable > next to our executable > PATH.
pub fn locate(
    base: &str,
    setting: Option<&Path>,
    env_value: Option<&OsStr>,
    exe_dir: Option<&Path>,
    path_var: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(p) = setting.filter(|p| p.is_file()) {
        return Some(p.to_path_buf());
    }
    if let Some(p) = env_value.map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    let name = exe_name(base);
    if let Some(p) = exe_dir.map(|d| d.join(&name)).filter(|p| p.is_file()) {
        return Some(p);
    }
    path_var
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|d| d.join(&name))
        .find(|p| p.is_file())
}

/// Locates `ffmpeg` and `ffprobe` using the real environment.
pub fn locate_both(
    ffmpeg_setting: Option<&Path>,
    ffprobe_setting: Option<&Path>,
) -> Result<(PathBuf, PathBuf), FfmpegError> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    let path = std::env::var_os("PATH");
    let ffmpeg = locate(
        "ffmpeg",
        ffmpeg_setting,
        std::env::var_os("VIDEOLORE_FFMPEG").as_deref(),
        exe_dir.as_deref(),
        path.as_deref(),
    )
    .ok_or_else(|| FfmpegError::NotFound("ffmpeg".into()))?;
    let ffprobe = locate(
        "ffprobe",
        ffprobe_setting,
        std::env::var_os("VIDEOLORE_FFPROBE").as_deref(),
        exe_dir.as_deref(),
        path.as_deref(),
    )
    .ok_or_else(|| FfmpegError::NotFound("ffprobe".into()))?;
    Ok((ffmpeg, ffprobe))
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
    })
}

/// How to install ffmpeg on this platform (shown in the blocking startup message).
pub fn install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "Install it with Homebrew: brew install ffmpeg"
    } else if cfg!(windows) {
        "Install it with: winget install Gyan.FFmpeg (or put ffmpeg.exe and ffprobe.exe next to videolore.exe)"
    } else {
        "Install it with your package manager, e.g. sudo apt install ffmpeg"
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

    type Responder = Box<dyn Fn(&Path, &[String]) -> Option<CmdOutput> + Send + Sync>;

    /// Answers each call with the first responder that returns `Some`.
    #[derive(Default)]
    pub struct FakeRunner {
        pub responders: Vec<Responder>,
        pub calls: Mutex<Vec<Vec<String>>>,
    }

    impl FakeRunner {
        pub fn on(
            mut self,
            f: impl Fn(&Path, &[String]) -> Option<CmdOutput> + Send + Sync + 'static,
        ) -> Self {
            self.responders.push(Box::new(f));
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

    struct FakeProcess(Option<CmdOutput>);

    impl Process for FakeProcess {
        fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
            let out = self.0.as_mut()?.stdout.clone();
            Some(Box::new(Cursor::new(out)))
        }
        fn kill(&mut self) {}
        fn wait(&mut self) -> io::Result<CmdOutput> {
            Ok(self.0.take().unwrap_or_default())
        }
    }

    impl Runner for FakeRunner {
        fn run(&self, program: &Path, args: &[OsString]) -> io::Result<CmdOutput> {
            self.answer(program, args)
        }
        fn spawn(&self, program: &Path, args: &[OsString]) -> io::Result<Box<dyn Process>> {
            Ok(Box::new(FakeProcess(Some(self.answer(program, args)?))))
        }
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
            Some(setting.clone())
        );
        assert_eq!(
            locate("ffmpeg", None, Some(setting.as_os_str()), Some(&a), None),
            Some(setting.clone())
        );
        assert_eq!(
            locate(
                "ffmpeg",
                Some(&dir.path().join("nope")),
                None,
                Some(&a),
                Some(&path_var)
            ),
            Some(a.join(&name))
        );
        assert_eq!(
            locate("ffmpeg", None, None, None, Some(&path_var)),
            Some(b.join(&name))
        );
        assert_eq!(locate("ffmpeg", None, None, None, None), None);
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
