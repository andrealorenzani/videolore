// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let file = videolore_player::file_arg(std::env::args_os());
    eframe::run_native(
        "videolore player",
        videolore_player::native_options(),
        Box::new(move |_cc| Ok(Box::new(videolore_player::real_app(file)))),
    )
}
