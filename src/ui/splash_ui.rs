// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The editor's splash (REQ-133): glue between [`Popup::Splash`] and the shared window
//! ([`super::splash_window`]), with the editor's picture and summary.

use super::UiCache;
use super::splash_window::{self, SplashOutcome};
use crate::app::{AppState, Command, Popup};
use crate::splash;
use eframe::egui::Context;

/// Draws the splash window when [`Popup::Splash`] is open.
pub fn show(ctx: &Context, _app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let (outcome, rects) = splash_window::show(ctx, &mut cache.splash, &splash::EDITOR);
    cache.splash_rects = rects;
    if outcome == SplashOutcome::Close {
        cmds.push(Command::ClosePopup(Popup::Splash));
    }
}
