// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Importing a `.srt` or `.vtt` file as subtitle lines (REQ-183, REQ-184, REQ-185).
//!
//! The file is read when the dialog opens (UTF-8 only, bounded), and nothing changes until
//! the user confirms. The cues are in the file's times, taken as the times of the source
//! file the import is for; the lines replace those of the clip's range (or of every part of
//! that source the timeline uses) as a creation does, in one undo step. With "re-time" the
//! speech is recognised for those ranges first (the consent and the progress of a creation)
//! and each line follows its first and last recognised word.

use super::*;
use crate::srt::{self, Cue};

/// A file read and waiting for the dialog's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDraft {
    /// The file's name, for the dialog.
    pub name: String,
    pub cues: Vec<Cue>,
    /// The clip whose range is filled (`None`: the whole video).
    pub clip: Option<ClipId>,
    /// The source file the cues belong to.
    pub media: MediaId,
    /// Re-time from the speech (off by default).
    pub retime: bool,
    /// This computer can recognise speech (the checkbox is offered).
    pub can_retime: bool,
    /// Lines edited by hand would be replaced.
    pub replaces_edits: bool,
}

/// The cues of an import that re-times, kept in a creation request.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportData {
    pub media: MediaId,
    pub cues: Vec<Cue>,
}

/// The subtitle file beside `video` with the same base name (`.srt` first, then `.vtt`).
pub fn sibling_file(video: &Path) -> Option<PathBuf> {
    ["srt", "vtt"]
        .into_iter()
        .map(|e| video.with_extension(e))
        .find(|p| p != video && p.is_file())
}

impl AppState {
    /// The parts of `media` (source ranges) an import fills: the clip's range, or every
    /// range of the timeline that uses the file, joined where they touch.
    pub fn import_ranges(&self, media: MediaId, clip: Option<ClipId>) -> Vec<(Micros, Micros)> {
        let tl = &self.project.timeline;
        let mut v: Vec<(Micros, Micros)> = match clip {
            Some(id) => tl
                .get(id)
                .filter(|c| c.media == media)
                .map(|c| vec![(c.src_in, c.src_out)])
                .unwrap_or_default(),
            None => tl
                .clips
                .iter()
                .filter(|c| c.media == media)
                .map(|c| (c.src_in, c.src_out))
                .collect(),
        };
        v.sort();
        let mut out: Vec<(Micros, Micros)> = Vec::new();
        for (a, b) in v {
            match out.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => out.push((a, b)),
            }
        }
        out
    }

    /// The file the Subtitles popup offers to import for `clip` (REQ-185).
    pub fn import_offer(&self, clip: ClipId) -> Option<&Path> {
        let media = self.project.timeline.get(clip)?.media;
        self.import_offers.get(&media).map(PathBuf::as_path)
    }

    /// Looks for the same-name subtitle file of every ready video (one `is_file` each).
    pub(super) fn refresh_import_offers(&mut self) {
        self.import_offers = self
            .project
            .media
            .iter()
            .filter(|m| m.is_ready() && !m.missing && m.has_video())
            .filter_map(|m| sibling_file(&m.path).map(|p| (m.id, p)))
            .collect();
    }

    /// Reads `path` and opens the import dialog.
    pub(super) fn open_import(
        &mut self,
        path: PathBuf,
        clip: Option<ClipId>,
    ) -> Result<(), String> {
        if self.stt.is_some() || self.model_install.is_some() {
            return Err(subs_create::BEING_CREATED.into());
        }
        let tl = &self.project.timeline;
        if tl.clips.is_empty() {
            return Err("add clips to the timeline first".into());
        }
        let cues = srt::read_file(&path).map_err(|e| e.to_string())?;
        let media = match clip {
            Some(id) => tl.get(id).ok_or(EditError::NoSuchClip.to_string())?.media,
            None => {
                let stem = path.file_stem();
                let same_name = tl.clips.iter().map(|c| c.media).find(|m| {
                    self.project
                        .media(*m)
                        .is_some_and(|i| i.path.file_stem() == stem)
                });
                same_name
                    .or_else(|| self.target_clip().and_then(|c| tl.get(c)).map(|c| c.media))
                    .unwrap_or(tl.clips[0].media)
            }
        };
        let ranges = self.import_ranges(media, clip);
        let replaces_edits = ranges
            .iter()
            .any(|(a, b)| self.project.subtitles.has_edits_in(media, *a, *b));
        self.pending_import = Some(ImportDraft {
            name: media::display_name(&path),
            cues,
            clip,
            media,
            retime: false,
            can_retime: stt::available(),
            replaces_edits,
        });
        self.modal = Some(Modal::ImportSubtitles);
        Ok(())
    }

    /// The answer "Import" of the dialog.
    pub(super) fn confirm_import(&mut self, now: Instant) -> Result<(), String> {
        if !matches!(self.modal, Some(Modal::ImportSubtitles)) {
            return Ok(());
        }
        let Some(draft) = self.pending_import.take() else {
            self.modal = None;
            return Ok(());
        };
        self.modal = None;
        let ranges = self.import_ranges(draft.media, draft.clip);
        if ranges.is_empty() {
            return Err("the clip no longer uses that video".into());
        }
        if draft.retime && draft.can_retime {
            if self.stt.is_some() || self.model_install.is_some() {
                return Err(subs_create::BEING_CREATED.into());
            }
            self.pending_create = Some(CreateRequest {
                clip: draft.clip,
                confirmed: true,
                import: Some(ImportData {
                    media: draft.media,
                    cues: draft.cues,
                }),
                ..Default::default()
            });
            return self.continue_create();
        }
        let parts: Vec<((Micros, Micros), Vec<crate::subtitles::NewLine>)> = ranges
            .iter()
            .map(|r| {
                let cues = stt::cues_in(&draft.cues, Some(*r));
                (*r, crate::align::spread(draft.media, &cues))
            })
            .collect();
        let n: usize = parts.iter().map(|(_, l)| l.len()).sum();
        if n == 0 {
            return Err("the file has no subtitles inside the part of the video in use".into());
        }
        let media = draft.media;
        self.edit(now, "Import subtitles", |e| {
            for ((a, b), lines) in parts {
                e.subtitles.replace_range(media, a, b, lines, None);
            }
            Ok(())
        })?;
        self.status = format!("Imported {n} subtitle line(s) from {}", draft.name);
        Ok(())
    }
}
