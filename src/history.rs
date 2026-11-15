// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Undo/redo by snapshots. Timelines are small, so whole-state snapshots are simple and
//! cannot get out of sync with the operations. Every step carries the name of the action
//! that made it ("Split", "Remove fade"), shown as "Undo Split" / "Redo Split".

use std::collections::VecDeque;

/// Maximum number of undo steps kept.
pub const HISTORY_CAP: usize = 100;

#[derive(Debug, Clone)]
pub struct History<T> {
    undo: VecDeque<(T, &'static str)>,
    redo: Vec<(T, &'static str)>,
    cap: usize,
}

impl<T: Clone> Default for History<T> {
    fn default() -> Self {
        Self::with_cap(HISTORY_CAP)
    }
}

impl<T: Clone> History<T> {
    pub fn with_cap(cap: usize) -> Self {
        History {
            undo: VecDeque::new(),
            redo: Vec::new(),
            cap: cap.max(1),
        }
    }

    /// Records the state *before* the edit called `label`. Clears the redo stack.
    pub fn record(&mut self, before: T, label: &'static str) {
        if self.undo.len() == self.cap {
            self.undo.pop_front();
        }
        self.undo.push_back((before, label));
        self.redo.clear();
    }

    /// Returns the state to restore, saving `current` for redo (the step keeps its name).
    pub fn undo(&mut self, current: T) -> Option<T> {
        let (prev, label) = self.undo.pop_back()?;
        self.redo.push((current, label));
        Some(prev)
    }

    /// Returns the state to restore, saving `current` for undo (the step keeps its name).
    pub fn redo(&mut self, current: T) -> Option<T> {
        let (next, label) = self.redo.pop()?;
        self.undo.push_back((current, label));
        Some(next)
    }

    /// Name of the action the next undo reverts.
    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.back().map(|(_, l)| *l)
    }

    /// Name of the action the next redo repeats.
    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|(_, l)| *l)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// True when any kept snapshot (undo or redo) satisfies `f`.
    pub fn any(&self, f: impl Fn(&T) -> bool) -> bool {
        self.undo.iter().chain(self.redo.iter()).any(|(t, _)| f(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-004
    #[test]
    fn undo_redo_fifty_and_cap() {
        let mut h = History::default();
        let mut state = 0;
        for i in 1..=60 {
            h.record(state, "step");
            state = i;
        }
        for _ in 0..50 {
            state = h.undo(state).unwrap();
        }
        assert_eq!(state, 10);
        for _ in 0..50 {
            state = h.redo(state).unwrap();
        }
        assert_eq!(state, 60);
        assert!(!h.can_redo());
        for i in 61..=200 {
            h.record(state, "step");
            state = i;
        }
        assert_eq!(h.undo_len(), HISTORY_CAP);
        let mut n = 0;
        while let Some(s) = h.undo(state) {
            state = s;
            n += 1;
        }
        assert_eq!(n, HISTORY_CAP);
        assert_eq!(state, 100);
        assert!(h.can_redo());
        h.record(state, "step"); // new edit clears redo
        assert!(!h.can_redo());
        assert_eq!(h.redo(state), None);
        assert!(h.can_undo());
        assert!(h.any(|s| *s == 100));
        assert!(!h.any(|s| *s == 5));
        h.clear();
        assert!(!h.can_undo());
        assert_eq!(History::<i32>::with_cap(0).cap, 1);
    }

    // covers: REQ-074
    #[test]
    fn steps_keep_their_names_between_undo_and_redo() {
        let mut h = History::default();
        assert_eq!((h.undo_label(), h.redo_label()), (None, None));
        h.record(0, "Split");
        h.record(1, "Remove fade");
        assert_eq!(h.undo_label(), Some("Remove fade"));
        let s = h.undo(2).unwrap();
        assert_eq!(h.undo_label(), Some("Split"));
        assert_eq!(h.redo_label(), Some("Remove fade"));
        let s = h.undo(s).unwrap();
        assert_eq!(s, 0);
        assert_eq!(h.undo_label(), None);
        assert_eq!(h.redo_label(), Some("Split"));
        h.redo(s).unwrap();
        assert_eq!(h.undo_label(), Some("Split"));
        assert_eq!(h.redo_label(), Some("Remove fade"));
    }
}
