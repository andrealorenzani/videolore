// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Autosave and recovery in the application (REQ-189, REQ-190): the timer in `tick`, the
//! removal of the copy when the project is saved, discarded or closed, and the offer at the
//! next start. The policy and the files are in [`crate::autosave`].

use super::*;
use crate::autosave as store;
use std::time::SystemTime;

impl AppState {
    /// The stem of the autosave copy of the open project.
    fn autosave_stem(&self) -> String {
        store::name_for(self.project_path.as_deref(), &self.autosave_session)
    }

    /// Writes the copy when it is due (called once per frame). The project is cloned here
    /// (the cost of the clone is that of the clips); the writing is on a worker thread.
    pub(super) fn tick_autosave(&mut self, now: Instant) {
        if let Some(run) = &self.autosave_run
            && let Some(r) = run.poll()
        {
            self.autosave_run = None;
            if let Err(e) = r {
                self.status = format!("Autosave failed: {e}");
            }
        }
        let Some(dir) = self.autosave_dir.clone() else {
            return;
        };
        if self.autosave_run.is_some() || !store::due(now, self.autosave_last, self.dirty) {
            return;
        }
        self.autosave_last = now;
        self.autosave_run = Some(store::start(
            dir,
            self.autosave_stem(),
            self.project.clone(),
            self.project_path.clone(),
        ));
    }

    /// Waits for a running autosave write (tests and before the copy is removed).
    pub fn finish_autosave(&mut self) {
        if let Some(run) = self.autosave_run.take()
            && let Err(e) = run.wait()
        {
            self.status = format!("Autosave failed: {e}");
        }
    }

    /// Removes the copy of the open project: it was saved, its changes were discarded or
    /// the program closes.
    pub(super) fn clear_autosave(&mut self, now: Instant) {
        self.finish_autosave();
        self.autosave_last = now;
        if let Some(dir) = &self.autosave_dir {
            store::remove(dir, &self.autosave_stem());
        }
    }

    /// Looks for the copies a crash left (called once at start).
    pub(super) fn find_recoveries(&mut self) {
        let Some(dir) = &self.autosave_dir else {
            return;
        };
        self.recoveries = store::find(dir);
        if !self.recoveries.is_empty() && self.modal.is_none() {
            self.modal = Some(Modal::Recover);
        }
    }

    /// The name and age shown by the recovery dialog.
    pub fn recovery_text(&self) -> Option<String> {
        let c = self.recoveries.first()?;
        let what = match &c.original {
            Some(p) => media::display_name(p),
            None => "a project that was never saved".into(),
        };
        let age = SystemTime::now()
            .duration_since(c.modified)
            .map(|d| d.as_secs() / 60)
            .unwrap_or(0);
        Some(format!(
            "videolore found an autosaved copy of {what} from {age} minute(s) ago that is newer \
             than the saved project."
        ))
    }

    /// Recover: opens the copy as the unsaved changes of its project (REQ-190).
    pub(super) fn recover_autosave(&mut self, now: Instant) -> Result<(), String> {
        if self.recoveries.is_empty() {
            self.modal = None;
            return Ok(());
        }
        let copy = self.recoveries.remove(0);
        self.modal = None;
        let (p, missing) =
            store::load(&copy).map_err(|e| format!("the copy cannot be read: {e}"))?;
        // the project the copy belongs to is the saved state; the changes are unsaved
        let saved = copy
            .original
            .as_deref()
            .and_then(|o| project::load(o).ok())
            .map(|(p, _)| p.doc())
            .unwrap_or_else(|| Project::default().doc());
        self.load_project(p, copy.original.clone(), now);
        self.saved = saved;
        self.refresh_dirty();
        // the copy stays until the project is saved or discarded: it is the only one
        self.autosave_stem_hint(&copy);
        self.recoveries.clear();
        self.status = "Recovered the autosaved copy: save the project to keep it".into();
        if !missing.is_empty() {
            self.notice = Some(format!(
                "Some sources could not be found: {}",
                missing.join(", ")
            ));
        }
        Ok(())
    }

    /// An untitled copy keeps its own name until it is saved: the session name follows it.
    fn autosave_stem_hint(&mut self, copy: &store::Copy) {
        if copy.original.is_none() {
            self.autosave_session = copy
                .stem
                .strip_prefix("session-")
                .unwrap_or(&self.autosave_session)
                .to_string();
        }
    }

    /// Discard: deletes the copy offered; the next one (if any) is offered.
    pub(super) fn discard_autosave(&mut self) {
        if !self.recoveries.is_empty() {
            let c = self.recoveries.remove(0);
            if let Some(dir) = &self.autosave_dir {
                store::remove(dir, &c.stem);
            }
        }
        self.modal = (!self.recoveries.is_empty()).then_some(Modal::Recover);
    }

    /// Cancel: nothing is opened or deleted; the copies are offered again at the next start.
    pub(super) fn cancel_recovery(&mut self) {
        self.recoveries.clear();
        self.modal = None;
    }
}
