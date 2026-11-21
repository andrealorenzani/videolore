// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Starting the other program of videolore: "Open in player" in the editor and "Open in
//! editor" in the player (REQ-249, REQ-250, ADR-184). The program is the sibling file
//! with a fixed name in the folder of the running one: an absolute path, one argument,
//! no shell. A missing program gives a plain message; nothing is installed or downloaded.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The editor's file name (without the system's suffix).
pub const EDITOR: &str = "videolore";
/// The player's file name.
pub const PLAYER: &str = "videolore-player";

/// The message when the other program is not next to this one.
pub fn missing_message(name: &str) -> String {
    let what = if name == PLAYER {
        "videolore player"
    } else {
        "videolore (the editor)"
    };
    format!("{what} is not installed next to this program. Install it to use this command.")
}

/// Where the sibling `name` is, given the folder of the running program.
pub fn sibling_in(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// Starts the sibling `name` found in `dir` with `file` as its only argument.
pub fn launch_in(dir: &Path, name: &str, file: &Path) -> Result<(), String> {
    let exe = sibling_in(dir, name);
    if !dir.is_absolute() || !exe.is_file() {
        return Err(missing_message(name));
    }
    let arg: OsString = file.as_os_str().to_os_string();
    std::process::Command::new(&exe)
        .arg(arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|_| missing_message(name))
}

/// Starts the sibling `name` next to the running program.
pub fn launch(name: &str, file: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|_| missing_message(name))?;
    let dir = exe.parent().ok_or_else(|| missing_message(name))?;
    launch_in(dir, name, file)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    // covers: REQ-249, REQ-250
    #[test]
    fn the_sibling_program_is_started_with_one_argument_and_a_missing_one_is_explained() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("args.txt");
        let script = dir.path().join(PLAYER);
        std::fs::write(
            &script,
            format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", out.display()),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        // a name with spaces and a quote is one argument (no shell)
        let video = dir.path().join("my \"video\" $(x).mp4");
        launch_in(dir.path(), PLAYER, &video).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !out.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            video.display().to_string()
        );
        // the editor is not there
        let e = launch_in(dir.path(), EDITOR, &video).unwrap_err();
        assert!(e.contains("not installed next to this program"), "{e}");
        assert!(launch_in(Path::new("relative"), PLAYER, &video).is_err());
        assert_eq!(
            sibling_in(Path::new("/x"), EDITOR),
            PathBuf::from("/x/videolore")
        );
    }
}
