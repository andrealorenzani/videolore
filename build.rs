// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Build check (QG-17): the splash screens embed their summary text with `include_str!`, and a
//! missing file would stop the build with a message about the macro. This says what is wrong
//! and what to write.

use std::path::Path;

fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    for (file, program) in [("summary.txt", "editor"), ("summary-player.txt", "player")] {
        let path = Path::new(&root).join("assets/splash").join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        match std::fs::read_to_string(&path) {
            Ok(text) if !text.trim().is_empty() => {}
            Ok(_) => fail(&path, program, "is empty"),
            Err(e) => fail(&path, program, &format!("cannot be read ({e})")),
        }
    }
}

fn fail(path: &Path, program: &str, why: &str) -> ! {
    eprintln!(
        "error: the splash summary of the {program} {why}: {}\n\
         Write a heading and at most five lines (one per change users see) in that file.",
        path.display()
    );
    std::process::exit(1);
}
