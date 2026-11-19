// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Full screen (REQ-141): a small state machine with an injected clock, so the 5 second
//! hint and the 2 second hiding of the controls are tested without waiting.

use std::time::{Duration, Instant};

/// How long "Press Esc to exit full screen" stays.
pub const HINT_TIME: Duration = Duration::from_secs(5);
/// Idle time (no pointer movement) after which the controls hide in full screen.
pub const HIDE_AFTER: Duration = Duration::from_secs(2);
/// The text of the hint popup.
pub const HINT_TEXT: &str = "Press Esc to exit full screen";

/// What Esc did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeResult {
    /// The popup was closed (full screen stays).
    ClosedHint,
    /// Full screen was left.
    Left,
    /// Nothing to do.
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullScreen {
    pub on: bool,
    hint_until: Option<Instant>,
    last_move: Instant,
}

impl FullScreen {
    pub fn new(now: Instant) -> FullScreen {
        FullScreen {
            on: false,
            hint_until: None,
            last_move: now,
        }
    }

    /// F11, F or View > Full screen: switches on or off. Entering shows the hint again.
    pub fn toggle(&mut self, now: Instant) {
        if self.on {
            self.leave();
        } else {
            self.on = true;
            self.hint_until = Some(now + HINT_TIME);
            self.last_move = now;
        }
    }

    pub fn leave(&mut self) {
        self.on = false;
        self.hint_until = None;
    }

    /// The popup is on screen.
    pub fn hint_visible(&self) -> bool {
        self.on && self.hint_until.is_some()
    }

    /// OK on the popup.
    pub fn dismiss_hint(&mut self) {
        self.hint_until = None;
    }

    /// The pointer moved (or was clicked): the controls show again.
    pub fn pointer_moved(&mut self, now: Instant) {
        self.last_move = now;
    }

    /// Closes the hint when its time is over.
    pub fn tick(&mut self, now: Instant) {
        if self.hint_until.is_some_and(|t| now >= t) {
            self.hint_until = None;
        }
    }

    /// The playback controls are shown: always outside full screen, and in full screen
    /// until [`HIDE_AFTER`] without pointer movement.
    pub fn controls_visible(&self, now: Instant) -> bool {
        !self.on || now < self.last_move + HIDE_AFTER
    }

    /// Esc always leaves full screen; while the popup is open it closes the popup first.
    pub fn escape(&mut self) -> EscapeResult {
        if self.hint_visible() {
            self.dismiss_hint();
            EscapeResult::ClosedHint
        } else if self.on {
            self.leave();
            EscapeResult::Left
        } else {
            EscapeResult::Nothing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-141
    #[test]
    fn entering_shows_the_hint_which_goes_after_five_seconds_or_with_ok() {
        let t0 = Instant::now();
        let mut f = FullScreen::new(t0);
        assert!(!f.on && !f.hint_visible());
        f.toggle(t0);
        assert!(f.on && f.hint_visible());
        f.tick(t0 + Duration::from_millis(4_900));
        assert!(f.hint_visible());
        f.tick(t0 + HINT_TIME);
        assert!(!f.hint_visible() && f.on);
        // OK closes it at once; the next entry shows it again
        f.leave();
        f.toggle(t0 + Duration::from_secs(10));
        assert!(f.hint_visible());
        f.dismiss_hint();
        assert!(!f.hint_visible());
        f.toggle(t0 + Duration::from_secs(11));
        assert!(!f.on, "F11 toggles both ways");
        f.toggle(t0 + Duration::from_secs(12));
        assert!(f.hint_visible());
    }

    // covers: REQ-141
    #[test]
    fn escape_closes_the_hint_first_then_leaves_full_screen() {
        let t0 = Instant::now();
        let mut f = FullScreen::new(t0);
        assert_eq!(f.escape(), EscapeResult::Nothing);
        f.toggle(t0);
        assert_eq!(f.escape(), EscapeResult::ClosedHint);
        assert!(f.on);
        assert_eq!(f.escape(), EscapeResult::Left);
        assert!(!f.on);
        assert_eq!(f.escape(), EscapeResult::Nothing);
    }

    // covers: REQ-141
    #[test]
    fn the_controls_hide_after_two_idle_seconds_and_show_when_the_pointer_moves() {
        let t0 = Instant::now();
        let mut f = FullScreen::new(t0);
        // outside full screen they are always there
        assert!(f.controls_visible(t0 + Duration::from_secs(60)));
        f.toggle(t0);
        assert!(f.controls_visible(t0 + Duration::from_millis(1_999)));
        assert!(!f.controls_visible(t0 + HIDE_AFTER));
        f.pointer_moved(t0 + Duration::from_secs(3));
        assert!(f.controls_visible(t0 + Duration::from_millis(4_500)));
        assert!(!f.controls_visible(t0 + Duration::from_secs(5)));
    }
}
