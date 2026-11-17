// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The choices of the Export dialog (REQ-118 to REQ-122) and the rules that go with them:
//! what is written (mode, formats, 360 choice), where (folder and file name), and why
//! Export is not possible yet. Pure: the dialog window only shows this model and the app
//! turns it into an export. The file name is untrusted input (it is typed): see
//! [`file_name_ok`].

use crate::format::{AudioChoice, FormatChoice, VIDEO_PRESETS};
use crate::render::ExportMode;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Longest file name accepted (characters).
pub const MAX_NAME: usize = 200;

/// What the export writes (REQ-119).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Full,
    AudioOnly,
    Srt,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Full, Mode::AudioOnly, Mode::Srt];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Full => "Full video",
            Mode::AudioOnly => "Only audio",
            Mode::Srt => "Only subtitles (.srt)",
        }
    }
}

/// The 360 choice of an all-360 project (REQ-122, REQ-103).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sphere {
    #[default]
    Video360,
    Flat,
}

/// Everything the dialog chooses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportChoice {
    pub mode: Mode,
    /// A preset of [`VIDEO_PRESETS`]; `None`: same as source.
    pub video: Option<usize>,
    pub audio: AudioChoice,
    /// Used only when the project's video clips are all 360 clips.
    pub sphere: Sphere,
    pub folder: PathBuf,
    /// The file name as typed, with or without its extension.
    pub name: String,
}

/// What the app knows about the project, for the rules below.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub has_clips: bool,
    pub has_subtitles: bool,
    /// Every video clip is a 360 clip (and there is at least one).
    pub all_360: bool,
    pub encoders: BTreeSet<String>,
    /// Extension of the automatic video format ("same as source").
    pub auto_extension: String,
    /// The `.vlproj` file, when saved.
    pub project_file: Option<PathBuf>,
    /// The files the clips and the music come from.
    pub media_files: Vec<PathBuf>,
}

/// What the app starts after the dialog (REQ-118).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Video {
        mode: ExportMode,
        format: FormatChoice,
    },
    Audio(AudioChoice),
    Srt,
}

const KNOWN_EXTENSIONS: [&str; 5] = ["mp4", "mov", "m4a", "mp3", "srt"];

/// The first choices of a session: the project's name in the project's folder; the folder
/// of the last export of the session comes through `last_folder`; with neither, the
/// user's documents folder.
pub fn defaults(
    project_file: Option<&Path>,
    last_folder: Option<&Path>,
    documents: Option<&Path>,
) -> ExportChoice {
    let name = project_file
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| file_name_ok(s).is_ok())
        .unwrap_or_else(|| "video".into());
    let folder = project_file
        .and_then(Path::parent)
        .filter(|d| !d.as_os_str().is_empty())
        .or(last_folder)
        .or(documents)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    ExportChoice {
        mode: Mode::Full,
        video: None,
        audio: AudioChoice::Same,
        sphere: Sphere::Video360,
        folder,
        name,
    }
}

/// Why `name` cannot be a file name (REQ-118); `Ok` when it can.
pub fn file_name_ok(name: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        return Err("Type a file name");
    }
    if name.chars().count() > MAX_NAME {
        return Err("The file name is too long");
    }
    if name.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return Err("The file name has a character that is not allowed");
    }
    if name == "." || name == ".." || name.ends_with('.') || name.ends_with(' ') {
        return Err("The file name cannot end with a dot or a space");
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p)
                .is_some_and(|n| matches!(n.as_bytes(), [b'1'..=b'9']))
        });
    if reserved {
        return Err("That file name is reserved by the system");
    }
    Ok(())
}

/// `name` with the extension `ext` (without the dot): a known extension is replaced,
/// anything else is kept and the extension is added.
pub fn with_extension(name: &str, ext: &str) -> String {
    if let Some((stem, old)) = name.rsplit_once('.')
        && !stem.is_empty()
        && KNOWN_EXTENSIONS.contains(&old.to_ascii_lowercase().as_str())
    {
        return format!("{stem}.{ext}");
    }
    format!("{name}.{ext}")
}

/// The extension (no dot) the choice writes.
pub fn extension(choice: &ExportChoice, ctx: &Context) -> String {
    match choice.mode {
        Mode::Srt => "srt".into(),
        Mode::AudioOnly => match choice.audio {
            AudioChoice::Mp3 => "mp3".into(),
            _ => "m4a".into(),
        },
        Mode::Full => match choice.video.and_then(|i| VIDEO_PRESETS.get(i)) {
            Some(p) if !(ctx.all_360 && choice.sphere == Sphere::Video360) => p.extension().into(),
            _ => ctx.auto_extension.clone(),
        },
    }
}

/// The file the choice writes: the folder and the name with its extension.
pub fn target_path(choice: &ExportChoice, ctx: &Context) -> PathBuf {
    choice
        .folder
        .join(with_extension(&choice.name, &extension(choice, ctx)))
}

/// Why the folder cannot be written to; `None` when it can. One metadata call.
pub fn folder_problem(folder: &Path) -> Option<&'static str> {
    match std::fs::metadata(folder) {
        Err(_) => Some("The folder does not exist"),
        Ok(m) if !m.is_dir() => Some("That is not a folder"),
        Ok(m) if m.permissions().readonly() => Some("The folder cannot be written to"),
        Ok(_) => None,
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| {
        p.canonicalize().ok().or_else(|| {
            let parent = p.parent()?.canonicalize().ok()?;
            Some(parent.join(p.file_name()?))
        })
    };
    match (canon(a), canon(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// The target would replace a file of the project: a source or the project file itself.
pub fn clash(target: &Path, ctx: &Context) -> Option<&'static str> {
    if ctx.media_files.iter().any(|m| same_file(m, target)) {
        return Some("That file is a source of the project");
    }
    if ctx
        .project_file
        .as_deref()
        .is_some_and(|p| same_file(p, target))
    {
        return Some("That file is the project file");
    }
    None
}

/// Why Export is not possible with `choice`; `None` when it is (REQ-118, REQ-119).
pub fn blocked(choice: &ExportChoice, ctx: &Context) -> Option<String> {
    if let Err(e) = file_name_ok(&choice.name) {
        return Some(e.into());
    }
    if let Some(e) = folder_problem(&choice.folder) {
        return Some(e.into());
    }
    match choice.mode {
        Mode::Srt if !ctx.has_subtitles => return Some("No subtitles on the timeline".into()),
        Mode::AudioOnly | Mode::Full if !ctx.has_clips => {
            return Some("There is nothing on the timeline".into());
        }
        _ => {}
    }
    if choice.mode != Mode::Srt && !choice.audio.available(&ctx.encoders) {
        return Some("This ffmpeg cannot make that audio format".into());
    }
    if choice.mode == Mode::Full
        && choice.video.is_some_and(|i| {
            VIDEO_PRESETS
                .get(i)
                .is_none_or(|p| !ctx.encoders.contains(p.encoder()))
        })
    {
        return Some("This ffmpeg cannot make that video format".into());
    }
    clash(&target_path(choice, ctx), ctx).map(str::to_string)
}

/// Why a field is greyed out in the mode of `choice`, or `None` when it is active.
pub fn video_field_reason(choice: &ExportChoice, ctx: &Context) -> Option<&'static str> {
    match choice.mode {
        Mode::Full if ctx.all_360 && choice.sphere == Sphere::Video360 => {
            Some("A 360 video keeps the picture of its sources")
        }
        Mode::Full => None,
        Mode::AudioOnly => Some("Only audio has no video"),
        Mode::Srt => Some("Only subtitles has no video"),
    }
}

/// Why the audio format is greyed out, or `None` when it is active.
pub fn audio_field_reason(choice: &ExportChoice) -> Option<&'static str> {
    (choice.mode == Mode::Srt).then_some("Only subtitles has no audio")
}

/// True when the 360 choice is shown: Full video of a project whose video clips are all
/// 360 clips (REQ-122).
pub fn shows_sphere_choice(choice: &ExportChoice, ctx: &Context) -> bool {
    choice.mode == Mode::Full && ctx.all_360
}

/// What the app starts for `choice`.
pub fn request(choice: &ExportChoice, ctx: &Context) -> Request {
    match choice.mode {
        Mode::Srt => Request::Srt,
        Mode::AudioOnly => Request::Audio(choice.audio),
        Mode::Full => {
            let sphere = ctx.all_360 && choice.sphere == Sphere::Video360;
            Request::Video {
                mode: if sphere {
                    ExportMode::Sphere360
                } else if ctx.all_360 {
                    ExportMode::FlatCamera
                } else {
                    ExportMode::Automatic
                },
                format: FormatChoice {
                    video: choice.video,
                    audio: choice.audio,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            has_clips: true,
            has_subtitles: true,
            all_360: false,
            encoders: ["libx264", "libx265", "aac", "libmp3lame"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            auto_extension: "mp4".into(),
            project_file: None,
            media_files: Vec::new(),
        }
    }

    fn choice(dir: &Path) -> ExportChoice {
        ExportChoice {
            folder: dir.to_path_buf(),
            name: "movie".into(),
            ..defaults(None, None, Some(dir))
        }
    }

    // covers: REQ-118
    #[test]
    fn names_are_checked_and_extensions_follow_the_format() {
        for ok in ["movie", "my movie 2", "a.b", "été", "x.mp4"] {
            assert_eq!(file_name_ok(ok), Ok(()), "{ok}");
        }
        for bad in [
            "", "  ", ".", "..", "a/b", "a\\b", "a:b", "a*", "a?", "a\"b", "a<b", "a>b", "a|b",
            "a\nb", "a\0b", "end.", "end ", "CON", "con.txt", "Nul", "COM1", "lpt9.mp4",
        ] {
            assert!(file_name_ok(bad).is_err(), "{bad:?}");
        }
        assert_eq!(file_name_ok("COM10"), Ok(()));
        assert!(file_name_ok(&"a".repeat(MAX_NAME + 1)).is_err());
        assert_eq!(file_name_ok(&"a".repeat(MAX_NAME)), Ok(()));
        // a known extension is replaced, anything else is kept
        assert_eq!(with_extension("movie", "mp4"), "movie.mp4");
        assert_eq!(with_extension("movie.mov", "mp4"), "movie.mp4");
        assert_eq!(with_extension("movie.MP3", "m4a"), "movie.m4a");
        assert_eq!(with_extension("my.video", "mp4"), "my.video.mp4");
        assert_eq!(with_extension(".mp4", "mp4"), ".mp4.mp4");
        let dir = tempfile::tempdir().unwrap();
        let mut c = choice(dir.path());
        let x = ctx();
        assert_eq!(target_path(&c, &x), dir.path().join("movie.mp4"));
        c.video = Some(3);
        assert_eq!(target_path(&c, &x), dir.path().join("movie.mov"));
        c.mode = Mode::AudioOnly;
        assert_eq!(extension(&c, &x), "m4a");
        c.audio = AudioChoice::Mp3;
        assert_eq!(extension(&c, &x), "mp3");
        c.mode = Mode::Srt;
        assert_eq!(target_path(&c, &x), dir.path().join("movie.srt"));
    }

    // covers: REQ-118
    #[test]
    fn the_first_choices_come_from_the_project_and_the_session() {
        let p = Path::new("/work/trip/my trip.vlproj");
        let d = defaults(Some(p), Some(Path::new("/last")), Some(Path::new("/docs")));
        assert_eq!(d.name, "my trip");
        assert_eq!(d.folder, Path::new("/work/trip"));
        // never saved: the session's folder, then the documents folder
        let d = defaults(None, Some(Path::new("/last")), Some(Path::new("/docs")));
        assert_eq!(
            (d.name.as_str(), d.folder.as_path()),
            ("video", Path::new("/last"))
        );
        let d = defaults(None, None, Some(Path::new("/docs")));
        assert_eq!(d.folder, Path::new("/docs"));
        // an unusable project name is replaced
        let d = defaults(Some(Path::new("/w/CON.vlproj")), None, None);
        assert_eq!(d.name, "video");
        assert_eq!(defaults(None, None, None).folder, PathBuf::new());
    }

    // covers: REQ-118, REQ-119
    #[test]
    fn export_is_blocked_with_a_reason() {
        let dir = tempfile::tempdir().unwrap();
        let mut x = ctx();
        let mut c = choice(dir.path());
        assert_eq!(blocked(&c, &x), None);
        // name and folder
        c.name = String::new();
        assert_eq!(blocked(&c, &x).as_deref(), Some("Type a file name"));
        c.name = "movie".into();
        c.folder = dir.path().join("missing");
        assert_eq!(
            blocked(&c, &x).as_deref(),
            Some("The folder does not exist")
        );
        let file = dir.path().join("a-file");
        std::fs::write(&file, b"x").unwrap();
        c.folder = file;
        assert_eq!(blocked(&c, &x).as_deref(), Some("That is not a folder"));
        c.folder = dir.path().to_path_buf();
        // modes
        c.mode = Mode::Srt;
        x.has_subtitles = false;
        assert_eq!(
            blocked(&c, &x).as_deref(),
            Some("No subtitles on the timeline")
        );
        x.has_subtitles = true;
        assert_eq!(blocked(&c, &x), None);
        c.mode = Mode::AudioOnly;
        x.has_clips = false;
        assert!(blocked(&c, &x).is_some());
        x.has_clips = true;
        // a format the ffmpeg cannot make
        c.audio = AudioChoice::Mp3;
        x.encoders.remove("libmp3lame");
        assert!(blocked(&c, &x).unwrap().contains("audio format"));
        c.audio = AudioChoice::Same;
        c.mode = Mode::Full;
        c.video = Some(11);
        x.encoders.remove("libx265");
        assert!(blocked(&c, &x).unwrap().contains("video format"));
        // the sources and the project file are never overwritten
        let src = dir.path().join("movie.mp4");
        std::fs::write(&src, b"source").unwrap();
        c.video = None;
        x.media_files = vec![src.clone()];
        assert_eq!(
            blocked(&c, &x).as_deref(),
            Some("That file is a source of the project")
        );
        x.media_files.clear();
        x.project_file = Some(src);
        assert_eq!(
            blocked(&c, &x).as_deref(),
            Some("That file is the project file")
        );
        x.project_file = Some(dir.path().join("elsewhere.vlproj"));
        assert_eq!(blocked(&c, &x), None);
    }

    #[cfg(unix)]
    // covers: REQ-118
    #[test]
    fn a_folder_that_cannot_be_written_blocks_export() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let ro = dir.path().join("ro");
        std::fs::create_dir(&ro).unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
        assert_eq!(folder_problem(&ro), Some("The folder cannot be written to"));
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(folder_problem(&ro), None);
    }

    // covers: REQ-119
    #[test]
    fn fields_are_greyed_with_a_reason_in_the_modes_that_do_not_use_them() {
        let dir = tempfile::tempdir().unwrap();
        let x = ctx();
        let mut c = choice(dir.path());
        assert_eq!(video_field_reason(&c, &x), None);
        assert_eq!(audio_field_reason(&c), None);
        c.mode = Mode::AudioOnly;
        assert!(video_field_reason(&c, &x).is_some());
        assert_eq!(audio_field_reason(&c), None);
        c.mode = Mode::Srt;
        assert!(video_field_reason(&c, &x).is_some());
        assert!(audio_field_reason(&c).is_some());
        assert_eq!(Mode::ALL.map(Mode::label)[2], "Only subtitles (.srt)");
        assert_eq!(
            request(&c, &x),
            Request::Srt,
            "only subtitles is a subtitle file"
        );
        c.mode = Mode::AudioOnly;
        c.audio = AudioChoice::Aac;
        assert_eq!(request(&c, &x), Request::Audio(AudioChoice::Aac));
    }

    // covers: REQ-122
    #[test]
    fn the_360_choice_is_for_all_360_projects_and_a_mixed_project_exports_flat() {
        let dir = tempfile::tempdir().unwrap();
        let mut x = ctx();
        let mut c = choice(dir.path());
        assert!(!shows_sphere_choice(&c, &x));
        // no 360 clip, or a mixed project (all_360 false): automatic, no message
        assert_eq!(
            request(&c, &x),
            Request::Video {
                mode: ExportMode::Automatic,
                format: FormatChoice::default()
            }
        );
        x.all_360 = true;
        assert!(shows_sphere_choice(&c, &x));
        assert!(matches!(
            request(&c, &x),
            Request::Video {
                mode: ExportMode::Sphere360,
                ..
            }
        ));
        assert!(video_field_reason(&c, &x).is_some());
        c.sphere = Sphere::Flat;
        assert!(matches!(
            request(&c, &x),
            Request::Video {
                mode: ExportMode::FlatCamera,
                ..
            }
        ));
        assert_eq!(video_field_reason(&c, &x), None);
        c.mode = Mode::AudioOnly;
        assert!(!shows_sphere_choice(&c, &x));
    }
}
