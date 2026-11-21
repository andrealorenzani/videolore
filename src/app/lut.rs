// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Apply LUT (1.6.0, REQ-135, REQ-136): choosing, changing and removing the LUT file of a
//! clip or of the whole video. The file is validated when chosen ([`crate::lut::read`]);
//! the project keeps only its path. Each change is one undo step.

use super::*;
use crate::effect_order::Row;

/// A LUT file above this size is read in the background (a smaller one in no time).
pub(super) const SYNC_LUT_BYTES: u64 = 1024 * 1024;

/// A LUT file chosen and still being read.
pub(super) struct PendingLut {
    pub(super) load: crate::lut::Load,
    pub(super) target: FrameTarget,
    pub(super) path: PathBuf,
}

impl AppState {
    /// Follows the background read of a LUT file (the tick): shows its progress and applies
    /// the LUT when it is read, or says why it was refused (REQ-136). A newer choice
    /// replaces the pending one, which stops (ADR-205).
    pub(super) fn tick_lut_load(&mut self, now: Instant) {
        let Some(p) = &mut self.lut_load else {
            return;
        };
        let name = crate::lut::display_name(p.path.to_str().unwrap_or(""));
        let mut done = None;
        while let Some(m) = p.load.poll() {
            if let crate::lut::LoadMsg::Done(r) = m {
                done = Some(r);
                break;
            }
        }
        let Some(r) = done else {
            self.status = format!("Reading {name}... {} %", p.load.percent());
            return;
        };
        let (target, path) = (p.target, p.path.clone());
        self.lut_load = None;
        match r {
            Ok(_) => {
                self.status = match self.apply_lut_file(target, path.to_str().unwrap_or(""), now) {
                    Ok(()) => self.status.clone(),
                    Err(e) => e,
                }
            }
            Err(e) => self.status = format!("{name}: {e}"),
        }
    }

    /// Asks for a LUT file and applies it to `target` (nothing changes on Cancel).
    pub(super) fn pick_lut(&mut self, target: FrameTarget, now: Instant) -> Result<(), String> {
        match self.dialogs.pick_lut() {
            Some(path) => self.set_lut_file(target, &path, now),
            None => Ok(()),
        }
    }

    /// Applies the LUT file at `path`. A file that is not a valid `.cube` is refused with
    /// its message and nothing changes (REQ-136).
    pub(super) fn set_lut_file(
        &mut self,
        target: FrameTarget,
        path: &Path,
        now: Instant,
    ) -> Result<(), String> {
        let text = path
            .to_str()
            .filter(|t| crate::lut::path_ok(t))
            .ok_or("The LUT file name cannot be used (it is not plain text or is too long)")?
            .to_string();
        // the file the clip already has: nothing changes, no undo step (REQ-205)
        let tl = &self.project.timeline;
        let already = match target {
            FrameTarget::Clip(id) => {
                tl.lut_of(target) == Some(text.as_str()) && tl.get(id).is_some()
            }
            FrameTarget::Whole => {
                !tl.clips.is_empty()
                    && tl
                        .clips
                        .iter()
                        .all(|c| tl.lut_of(FrameTarget::Clip(c.id)) == Some(text.as_str()))
            }
        };
        if already {
            let name = crate::lut::display_name(&text);
            self.status = match target {
                FrameTarget::Clip(_) => format!("The clip already uses {name}"),
                FrameTarget::Whole => format!("Every clip already uses {name}"),
            };
            return Ok(());
        }
        // a large file is read on a thread of its own, with a progress in the status line
        let big = std::fs::metadata(&text).is_ok_and(|m| m.len() > SYNC_LUT_BYTES);
        if big && crate::lut::peek(&text).is_none() {
            self.status = format!("Reading {}... 0 %", crate::lut::display_name(&text));
            self.lut_load = Some(PendingLut {
                load: crate::lut::Load::start(&text),
                target,
                path: path.to_path_buf(),
            });
            return Ok(());
        }
        self.apply_lut_file(target, &text, now)
    }

    /// Checks the file (through the cache) and applies it: one undo step.
    fn apply_lut_file(
        &mut self,
        target: FrameTarget,
        text: &str,
        now: Instant,
    ) -> Result<(), String> {
        let path = Path::new(text);
        let text = text.to_string();
        crate::lut::cached(&text)
            .map_err(|e| format!("{}: {e}", crate::lut::display_name(&text)))?;
        let label = if self.project.timeline.lut_of(target).is_some() {
            "Change LUT"
        } else {
            "Apply LUT"
        };
        let had: Vec<ClipId> = self.lut_clips();
        self.edit(now, label, |e| {
            e.timeline.set_lut(target, Some(text)).map_err(err)
        })?;
        // a new LUT goes on top of the clip's other effects (REQ-304)
        let new: Vec<ClipId> = self
            .lut_clips()
            .into_iter()
            .filter(|c| !had.contains(c))
            .collect();
        self.project.put_on_top(Row::Lut, Some(&new));
        self.status = format!(
            "{label}: {}",
            crate::lut::display_name(path.to_str().unwrap_or(""))
        );
        Ok(())
    }

    /// The clips that have an Apply LUT.
    fn lut_clips(&self) -> Vec<ClipId> {
        self.project
            .timeline
            .clips
            .iter()
            .filter(|c| {
                self.project
                    .timeline
                    .lut_of(FrameTarget::Clip(c.id))
                    .is_some()
            })
            .map(|c| c.id)
            .collect()
    }

    /// Removes the LUT of `target` (its popup stays open and shows no file).
    pub(super) fn remove_lut(&mut self, target: FrameTarget, now: Instant) -> Result<(), String> {
        self.edit(now, "Remove LUT", |e| {
            e.timeline.set_lut(target, None).map_err(err)
        })
    }

    /// The LUT files of the project that are not there (names for a message).
    pub fn missing_luts(&self) -> Vec<String> {
        let tl = &self.project.timeline;
        // a whole-video LUT is the same file on several clips: named once
        let mut paths: Vec<&String> = tl.clip_luts.iter().map(|l| &l.path).collect();
        paths.sort();
        paths.dedup();
        paths
            .into_iter()
            .filter(|p| !Path::new(p.as_str()).exists() && !crate::lut::has_embedded(p))
            .map(|p| crate::lut::display_name(p))
            .collect()
    }
}
