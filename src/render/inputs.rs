// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Output geometry, ffmpeg inputs (runs of one file) and the music bed.

use super::*;

/// Output picture geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
    pub fps: Fps,
}

impl Geometry {
    pub fn of(s: &EncodeSettings) -> Geometry {
        Geometry {
            width: s.width,
            height: s.height,
            fps: s.fps,
        }
    }

    /// Same aspect ratio, at most [`PREVIEW_HEIGHT`] high, even dimensions.
    pub fn preview(self) -> Geometry {
        if self.height <= PREVIEW_HEIGHT {
            return self;
        }
        let w = (self.width as u64 * PREVIEW_HEIGHT as u64 / self.height.max(1) as u64) as u32;
        Geometry {
            width: (w & !1).max(2),
            height: PREVIEW_HEIGHT,
            fps: self.fps,
        }
    }
}

/// One ffmpeg input: `[start, start + len)` of `path` (plus one second of margin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub path: PathBuf,
    pub start: Micros,
    pub len: Micros,
}

/// Groups source spans `(path, start, end)` into inputs. Returns the inputs and, for each
/// span, the index of its input. A span joins the previous input when it reads the same
/// file at or after the previous span's end (and not too far after it).
pub fn group_inputs(spans: &[(PathBuf, Micros, Micros)]) -> (Vec<Input>, Vec<usize>) {
    let mut inputs: Vec<Input> = Vec::new();
    let mut index = Vec::with_capacity(spans.len());
    let mut last_end = Micros::ZERO;
    for (path, start, end) in spans {
        let joins = inputs.last().is_some_and(|i| {
            &i.path == path && *start >= last_end && *start - last_end <= MAX_RUN_GAP
        });
        if joins {
            let i = inputs.last_mut().expect("checked above");
            i.len = *end - i.start;
        } else {
            inputs.push(Input {
                path: path.clone(),
                start: *start,
                len: *end - *start,
            });
        }
        index.push(inputs.len() - 1);
        last_end = *end;
    }
    (inputs, index)
}

/// `-ss <start> -t <len + 1 s> -i file:<path>` for each input.
pub fn input_args(inputs: &[Input]) -> Vec<OsString> {
    let mut a = Vec::new();
    for i in inputs {
        a.extend(argv([
            "-protocol_whitelist".to_string(),
            "file,pipe".to_string(),
            "-ss".to_string(),
            i.start.ff(),
            "-t".to_string(),
            (i.len + Micros::SECOND).ff(),
            "-i".to_string(),
        ]));
        a.push(file_arg(&i.path));
    }
    a
}

/// A piece of the music bed.
#[derive(Debug, Clone, PartialEq)]
pub enum BedPart {
    /// Silence.
    Gap(Micros),
    /// `len` of `path` from source time `src`.
    Clip {
        path: PathBuf,
        src: Micros,
        len: Micros,
        /// The clip's own gain (REQ-276), 1.0 when untouched.
        gain: f64,
        /// Fades at the start and end of this part (0 where the clip is cut) (REQ-276).
        fade_in: Micros,
        fade_out: Micros,
        /// Clean audio on this clip (REQ-277).
        clean: bool,
    },
}

/// The music under the video from `from` to the video's end.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicBed {
    pub parts: Vec<BedPart>,
    pub len: Micros,
    /// Fade-out `(start, duration)` relative to the bed, when music runs past the video.
    pub fade: Option<(Micros, Micros)>,
}

/// Builds the music bed for the timeline from `from`; `None` when no music plays there.
pub fn music_bed(project: &Project, from: Micros) -> Result<Option<MusicBed>, RenderError> {
    let end = project.timeline.duration();
    if from >= end {
        return Ok(None);
    }
    let mut parts = Vec::new();
    let mut cursor = from;
    let mut past_end = false;
    for c in &project.music.clips {
        if c.end() <= from || c.start >= end {
            continue;
        }
        let m = project.media(c.media).ok_or(RenderError::UnknownMedia)?;
        if m.missing {
            return Err(RenderError::Missing(m.name.clone()));
        }
        let s = c.start.max(from);
        let e = c.end().min(end);
        if s > cursor {
            parts.push(BedPart::Gap(s - cursor));
        }
        parts.push(BedPart::Clip {
            path: m.path.clone(),
            src: c.src_in + (s - c.start),
            len: e - s,
            gain: c.gain(),
            // a fade only at an end the clip really has (not where the video starts or ends in it)
            fade_in: if s == c.start {
                c.fade_in.min(Micros((e - s).0 / 2))
            } else {
                Micros::ZERO
            },
            fade_out: if e == c.end() {
                c.fade_out.min(Micros((e - s).0 / 2))
            } else {
                Micros::ZERO
            },
            clean: c.clean_audio,
        });
        cursor = e;
        past_end |= c.end() > end;
    }
    if parts.is_empty() {
        return Ok(None);
    }
    if cursor < end {
        parts.push(BedPart::Gap(end - cursor));
    }
    let len = end - from;
    let fade = past_end.then(|| {
        let d = END_FADE.min(len);
        (len - d, d)
    });
    Ok(Some(MusicBed { parts, len, fade }))
}
