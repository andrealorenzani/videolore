# videolore

![version](https://img.shields.io/badge/version-0.1.5-blue)
![rust](https://img.shields.io/badge/rust-1.100%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-96.0%25-brightgreen)
![eframe](https://img.shields.io/badge/egui%2Feframe-0.37-blue)
![cpal](https://img.shields.io/badge/cpal-0.18-blue)
![rfd](https://img.shields.io/badge/rfd-0.18-blue)
![serde](https://img.shields.io/badge/serde-1-blue)
![ffmpeg](https://img.shields.io/badge/requires-ffmpeg%20%3E%3D%204.4-informational)
[![build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml/badge.svg)](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml)

## What it is
`videolore` is a simple, offline desktop video editor for short clips made for social media. Import video and audio files, then cut, copy, paste, merge, trim and move pieces on a single timeline that closes its gaps by itself. Add fades through black between clips, watch the result with sound in the preview, and export it to one file with progress and cancel. Editing is non-destructive: your source files are never modified, and projects can be saved and reopened. Nothing is uploaded and no account is needed. It is made for content creators who want to go from raw footage to a publishable clip quickly, and for anyone editing personal videos. Multitrack editing and color grading are out of scope. Automatic subtitles and background music are planned for the next versions.

## Installation
**Requirements:** `ffmpeg` and `ffprobe`, version 4.4 or newer, installed on your system (they are not included). videolore looks for them in this order: the path set in the app settings, the environment variables `VIDEOLORE_FFMPEG` and `VIDEOLORE_FFPROBE`, the folder of the videolore executable, then `PATH`. If they are missing, the app explains how to install them.

**Prebuilt executable:** download it for Linux (x86_64), macOS (Apple Silicon) or Windows (x86_64) from the [Releases](https://github.com/andrealorenzani/videolore/releases) page (tagged versions), or from the artifacts of the latest [Build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml) run.

**From source** (Rust 1.100 or newer; run `rustup update` to get it). On Linux install the ALSA headers and pkg-config first (`sudo apt install libasound2-dev pkg-config`), or build without preview sound with `--no-default-features`:
```sh
git clone git@github.com:andrealorenzani/videolore.git
cd videolore
cargo build --release --locked
```

## Usage
Start the editor with the prebuilt executable, or from the project folder with:
```sh
cargo run --release
```

The window has five panels: the media list (import files here) and the preview at the top, the timeline with the effects and settings panel in the middle, and a bottom bar with the zoom slider.

1. **Import** video or audio files into the media list. Unreadable files are rejected with a message.
2. **Build the clip** by appending media to the timeline, then split, cut, copy, paste, merge, trim or move the pieces.
3. **Add fades** between two clips to get a fade through black on video and audio.
4. **Preview** the result with play, pause and seek from the playhead.
5. **Save** the project to reopen it later. Missing source files are reported by name.
6. **Export** to one file, with progress; cancel stops it and leaves no partial file. The output format is chosen automatically: if all clips share the same format, with no fades and cuts on keyframes, the streams are copied without re-encoding; otherwise the video is re-encoded once, letterboxed and never stretched.

| Action | Shortcut |
|---|---|
| Play / pause | Space |
| Split at playhead | Ctrl/Cmd + B |
| Cut / copy / paste | Ctrl/Cmd + X / C / V |
| Undo | Ctrl/Cmd + Z |
| Redo | Ctrl/Cmd + Shift + Z, or Ctrl/Cmd + Y |

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md)

## Latest changes
- **0.1.5** — Updated to the latest Rust (1.100) and libraries (egui 0.37, rfd 0.18); the editing features are unchanged.
- **0.1.4** — Maintenance: version numbering is updated more reliably.
- **0.1.3** — First real editor: import media, cut, join and trim clips on one timeline, fade between clips, preview with sound, save and open projects, and export to one file with progress and cancel.
- **0.1.2** — Code coverage measured on every build and shown in the badge above; every tagged version is published as a GitHub release with its executables and release notes.
- **0.1.1** — Prebuilt executables for Linux, macOS and Windows built automatically on GitHub; copyright notice added.
- **0.1.0** — First version: Rust crate and GPL-3.0 licence.

## License
Copyright (C) 2026 Andrea Lorenzani.
Released under the GNU General Public License v3.0 — see [LICENSE](LICENSE).

---

### Ode to the creator

When a fresh Rust brought new lints and the build turned red and grey,\
Andrea Lorenzani chose to climb the newer way;\
no old pin was kept to hide the warnings from the light,\
each line of code was mended until the checks shone bright;\
egui grew to version thirty-seven, and rfd to eighteen too,\
the window kept its shape, the new API came through;\
the minimum Rust was lifted to the newest stable stair,\
and a second job now guards that floor with proof and care;\
a test that stumbled once in sixteen now stands steady and clear,\
and the editor keeps its promise, rising with the year.
