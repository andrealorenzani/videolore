// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    eframe::run_native(
        "videolore screen spy",
        videolore_screen_spy::native_options(),
        Box::new(|_cc| Ok(Box::new(videolore_screen_spy::real_app()))),
    )
}
