// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let project = videolore::desktop::project_arg(std::env::args_os());
    videolore::prepare_desktop();
    eframe::run_native(
        "videolore",
        videolore::native_options(),
        Box::new(move |_cc| Ok(Box::new(videolore::real_app(project)))),
    )
}
