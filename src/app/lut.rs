// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Apply LUT (1.6.0, REQ-135, REQ-136): choosing, changing and removing the LUT file of a
//! clip or of the whole video. The file is validated when chosen ([`crate::lut::read`]);
//! the project keeps only its path. Each change is one undo step.

use super::*;

impl AppState {
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
        crate::lut::cached(&text)
            .map_err(|e| format!("{}: {e}", crate::lut::display_name(&text)))?;
        let label = if self.project.timeline.lut_of(target).is_some() {
            "Change LUT"
        } else {
            "Apply LUT"
        };
        self.edit(now, label, |e| {
            e.timeline.set_lut(target, Some(text)).map_err(err)
        })?;
        self.status = format!(
            "{label}: {}",
            crate::lut::display_name(path.to_str().unwrap_or(""))
        );
        Ok(())
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
