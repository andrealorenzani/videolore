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

/// The preview restarted with a LUT that is ready, until its first graded picture arrives
/// (REQ-312): ffmpeg reads the table itself, which takes a time known only by estimate.
pub(super) struct LutStart {
    pub(super) since: Instant,
    pub(super) est: Duration,
    pub(super) serial: u64,
}

/// Seconds ffmpeg's `lut3d` takes to read one table entry (measured 12.4 s for 256^3).
const FFMPEG_READ_PER_ENTRY: f64 = 0.75e-6;

impl AppState {
    /// The LUT files of the clips whose Work copies are not ready for the preview yet
    /// (and that did not fail): the preview shows the picture without them (REQ-312).
    pub fn unready_luts(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .project
            .timeline
            .clip_luts
            .iter()
            .filter(|l| !crate::lut::is_prepared(&l.path) && !self.lut_failed.contains(&l.path))
            .map(|l| l.path.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// The preparation of the LUTs for the preview (the tick, REQ-312): one at a time in the
    /// background, with "Preparing LUT N %" on the status line; when a table is ready the
    /// preview starts again with it. A newer choice replaces the one in progress.
    pub(super) fn tick_lut_prep(&mut self, now: Instant) {
        let used: Vec<&str> = self
            .project
            .timeline
            .clip_luts
            .iter()
            .map(|l| l.path.as_str())
            .collect();
        self.lut_failed.retain(|p| used.contains(&p.as_str()));
        let pending = self.unready_luts();
        if self
            .lut_prep
            .as_ref()
            .is_some_and(|p| !pending.contains(&p.path))
        {
            self.lut_prep = None;
        }
        if self.lut_prep.is_none()
            && self.lut_load.is_none()
            && self.ffmpeg.is_ok()
            && let Some(path) = pending.first()
        {
            let dirs = self.preview.work_dirs();
            self.lut_prep = Some(crate::lut::Prep::start(path, dirs));
        }
        let mut finished = None;
        let mut share = None;
        if let Some(p) = &mut self.lut_prep {
            while let Some(m) = p.poll() {
                match m {
                    crate::lut::PrepMsg::Progress(x) => share = Some(x),
                    crate::lut::PrepMsg::Done(r) => {
                        finished = Some((p.path.clone(), r));
                        break;
                    }
                }
            }
        }
        // reading and writing the copies are the first 60 %, the start of ffmpeg the rest
        if let Some(x) = share {
            self.status = format!("Preparing LUT {} %", u32::from(x) * 60 / 100);
        }
        if let Some((path, r)) = finished {
            self.lut_prep = None;
            match r {
                Ok(lut) => {
                    let entries = lut.size().pow(3) as f64;
                    self.lut_start = Some(LutStart {
                        since: now,
                        est: Duration::from_secs_f64(entries * FFMPEG_READ_PER_ENTRY),
                        serial: self.frame_serial,
                    });
                    self.restart_preview_for_lut(now);
                }
                Err(e) => {
                    self.lut_failed.push(path.clone());
                    self.status = format!("{}: {e}", crate::lut::display_name(&path));
                }
            }
        }
        if let Some(s) = &self.lut_start {
            let elapsed = now.saturating_duration_since(s.since);
            if self.frame_serial > s.serial || elapsed > s.est * 3 + Duration::from_secs(10) {
                self.lut_start = None;
                if self.status.starts_with("Preparing LUT") {
                    self.status.clear();
                }
            } else {
                let share = (elapsed.as_secs_f64() / s.est.as_secs_f64().max(0.5)).min(1.0);
                self.status = format!("Preparing LUT {} %", (60.0 + 35.0 * share) as u32);
            }
        }
    }

    /// Draws the picture again, or plays on from the playhead, now that a LUT is ready.
    fn restart_preview_for_lut(&mut self, now: Instant) {
        if self.preview.is_playing() {
            let at = self.preview.position(now);
            let shown = self.shown_project().into_owned();
            let _ = self.preview.play(&shown, at, now);
        } else {
            self.refresh_still(now);
        }
    }

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
        // the file for every clip is chosen: from now on each copy is a clip's own, and the
        // popup is the one of the clip under the playhead (REQ-147)
        if target == FrameTarget::Whole
            && self.popups.instance == Some(Instance::Lut(FrameTarget::Whole))
            && let Some(id) = self.clip_at_playhead_or_first()
        {
            self.popups.instance = Some(Instance::Lut(FrameTarget::Clip(id)));
            self.selected_clip = Some(id);
        }
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
