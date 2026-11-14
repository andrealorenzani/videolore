// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Undo/redo by snapshots. Timelines are small, so whole-state snapshots are simple and
//! cannot get out of sync with the operations.

use std::collections::VecDeque;

/// Maximum number of undo steps kept.
pub const HISTORY_CAP: usize = 100;

#[derive(Debug, Clone)]
pub struct History<T> {
    undo: VecDeque<T>,
    redo: Vec<T>,
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

    /// Records the state *before* an edit. Clears the redo stack.
    pub fn record(&mut self, before: T) {
        if self.undo.len() == self.cap {
            self.undo.pop_front();
        }
        self.undo.push_back(before);
        self.redo.clear();
    }

    /// Returns the state to restore, saving `current` for redo.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop_back()?;
        self.redo.push(current);
        Some(prev)
    }

    /// Returns the state to restore, saving `current` for undo.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push_back(current);
        Some(next)
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
        self.undo.iter().chain(self.redo.iter()).any(f)
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
            h.record(state);
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
            h.record(state);
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
        h.record(state); // new edit clears redo
        assert!(!h.can_redo());
        assert_eq!(h.redo(state), None);
        assert!(h.can_undo());
        assert!(h.any(|s| *s == 100));
        assert!(!h.any(|s| *s == 5));
        h.clear();
        assert!(!h.can_undo());
        assert_eq!(History::<i32>::with_cap(0).cap, 1);
    }
}
