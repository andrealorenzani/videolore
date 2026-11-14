# videolore

![version](https://img.shields.io/badge/version-0.2.1-blue)
![rust](https://img.shields.io/badge/rust-1.100%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-95.9%25-brightgreen)
![eframe](https://img.shields.io/badge/egui%2Feframe-0.37-blue)
![cpal](https://img.shields.io/badge/cpal-0.18-blue)
![rfd](https://img.shields.io/badge/rfd-0.18-blue)
![whisper](https://img.shields.io/badge/whisper--rs-0.16-blue)
![ureq](https://img.shields.io/badge/ureq-3.5-blue)
![serde](https://img.shields.io/badge/serde-1-blue)
![serde_json](https://img.shields.io/badge/serde__json-1-blue)
![thiserror](https://img.shields.io/badge/thiserror-2-blue)
![directories](https://img.shields.io/badge/directories-6-blue)
![tempfile](https://img.shields.io/badge/tempfile-3-blue)
![sha2](https://img.shields.io/badge/sha2-0.11-blue)
![sysinfo](https://img.shields.io/badge/sysinfo-0.40-blue)
![ffmpeg](https://img.shields.io/badge/ffmpeg-9.0.2%20bundled-informational)
[![build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml/badge.svg)](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml)

## What it is
`videolore` is a simple, offline desktop video editor for short clips made for social media. Import video and audio files, then cut, copy, paste, merge, trim and move pieces on one timeline that closes its gaps by itself. Add fades through black between clips and a music track with its own volume. Subtitles are generated on your computer from the speech, in ready-made styles, including word-by-word highlight. Watch the result with sound in the preview and export it to one file with progress and cancel. Editing is non-destructive: your source files are never modified, and projects can be saved, reopened and double-clicked. ffmpeg is included, so there is nothing else to install. Nothing is uploaded and no account is needed; the only network access is the one-time download of the speech model, which you approve. It is made for content creators who want to go from raw footage to a publishable clip quickly, and for anyone editing personal videos. Multitrack editing and color grading are out of scope.

## System requirements
- **Windows** (64-bit, x86_64) and **Linux** (64-bit, x86_64; current Ubuntu LTS and Fedora).
- **macOS** on Apple Silicon: every Apple Silicon Mac is supported.
- **Generating subtitles** on Windows and Linux needs a processor with AVX2, FMA and F16C: Intel from 2013 (Haswell) on, AMD from 2015 (Excavator) on, all Ryzen. Everything else in videolore has no special processor requirement.
- **Internet**: only for the one-time download of the speech model (about 32 MB), which you approve; you can also provide the model file yourself.

## Installation
Each release has one installer per system, with ffmpeg included. Download it from the [Releases](https://github.com/andrealorenzani/videolore/releases) page (tagged versions), or from the artifacts of the latest [Build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml) run.

The installers are not code-signed yet, so your system asks for a one-time confirmation the first time you open videolore. The steps are below.

### Windows (x86_64)
1. Run `videolore-<version>-windows-x86_64-setup.exe`. The installer needs no administrator rights and installs for your user only.
2. Start videolore from the Start menu.
3. First launch: if SmartScreen shows "Windows protected your PC", click **More info**, then **Run anyway**.
4. To uninstall, use Settings → Apps. Your projects are kept.

### macOS (Apple Silicon)
1. Open `videolore-<version>-macos-aarch64.dmg` and drag **videolore** into **Applications**.
2. First launch: macOS says the developer cannot be verified. Open **System Settings → Privacy & Security**, click **Open Anyway** next to videolore, then confirm. Only the first launch needs this.
3. If macOS still blocks the bundled ffmpeg, run `xattr -dr com.apple.quarantine /Applications/videolore.app` in Terminal once.
4. To uninstall, drag videolore out of Applications. Your projects are kept.

### Linux (x86_64, Ubuntu LTS and Fedora)
Choose one of the two files:
- **AppImage** (`videolore-<version>-linux-x86_64.AppImage`): make it executable once (file properties → **Allow executing file as program**, or `chmod +x` in a terminal), then double-click it. It needs `fusermount3`, which current Ubuntu LTS and Fedora have.
- **Archive** (`videolore-<version>-linux-x86_64.tar.gz`): extract it and double-click `videolore` in the new folder. If your file manager does not start programs on double-click, right-click the file and choose **Run**, or run `./videolore` from a terminal.

The first start adds videolore to the application menu and associates `.vlproj` files with it. To remove that entry, use **Remove from application menu** in the About screen; deleting the AppImage or folder uninstalls the program, and your projects are kept.

### From source
Requires Rust 1.100 or newer (`rustup update`). On Linux install the ALSA headers, cmake, a C++ compiler and libclang first (`sudo apt install libasound2-dev pkg-config cmake libclang-dev`). A source build uses the ffmpeg and ffprobe 4.4 or newer found on your system (`PATH`, or `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`). To build without sound and subtitles, which needs neither ALSA nor cmake, use `--no-default-features`.
```sh
git clone git@github.com:andrealorenzani/videolore.git
cd videolore
cargo build --release --locked
```

## Usage
Start videolore from the menu, the Applications folder, the AppImage or the extracted folder. The window has five panels: the media list (import files here) and the preview at the top, the timeline with the effects and settings panel in the middle, and a bar with the zoom slider at the bottom. Actions are also in the toolbar at the top.

1. **Import** video or audio files into the media list. Each file shows "analysing" while it is read, then its name and duration; unreadable files are rejected with a message. Importing never blocks the window.
2. **Build the clip** by appending media to the timeline, then split, cut, copy, paste, merge, trim or move the pieces.
3. **Add fades** between two clips to get a fade through black on video and audio.
4. **Add music**: place audio files on the music track below the video. Music can be moved, trimmed, split and deleted. Each of the video audio and the music has its own volume and mute. Music does not move when you edit the video, and it ends with a fade-out where the video ends.
5. **Add subtitles**: choose **Generate subtitles**. The first time, you are asked to approve a download of the speech model (about 32 MB), or you can provide the model file yourself. Recognition runs on your computer in the background, the language is detected automatically, and you can edit, re-word and re-time every line. Choose a style: Classic, Box or Word highlight. Subtitles are visible in the preview and burned into the exported video.
6. **Preview** the result with play, pause and seek from the playhead. Very large sources get a smaller preview copy, made in the background; the export always uses your original files. The preview copies can be cleared in the settings.
7. **Save** the project (**Save** or **Save As**, Ctrl/Cmd+S) and reopen it later with **Open**. A `•` in the window title means unsaved changes. New, Open, closing the window and Quit ask whether to **Save**, **Discard** or **Cancel**. Projects are `.vlproj` files: double-click one to open it in videolore.
8. **Export** to one file, with progress; cancel stops it and leaves no partial file. The output format is chosen automatically: if all clips share the same format, with no fades, music, subtitles or volume changes and cuts on keyframes, the streams are copied without re-encoding; otherwise the video is re-encoded once, letterboxed and never stretched. HEVC files play on Apple devices.

The About screen lists every bundled third-party component with its version, licence and where to get its source code.

| Action | Shortcut |
|---|---|
| Play / pause | Space |
| Split at playhead | Ctrl/Cmd + B |
| Cut / copy / paste | Ctrl/Cmd + X / C / V |
| Undo | Ctrl/Cmd + Z |
| Redo | Ctrl/Cmd + Shift + Z, or Ctrl/Cmd + Y |
| Save | Ctrl/Cmd + S |
| Quit | Ctrl/Cmd + Q |

### Known limitations
- Installers are not code-signed: each system asks for a one-time confirmation (see Installation).
- On Windows and Linux, generating subtitles needs a processor with AVX2, FMA and F16C (see System requirements). On older processors it may crash; the rest of videolore keeps working. This has not been tested on such hardware.
- Subtitles in scripts other than Latin, Greek and Cyrillic (for example Chinese, Japanese, Arabic or Hindi) may look different on each computer or show empty boxes, and cannot be read in the subtitle editor.
- Speech is recognised in pieces of about two minutes; a word cut at a boundary can be lost.
- On Windows and Linux, double-clicking a project while videolore is open opens a second window. On macOS the Hide and Minimize menu items are not available.
- Projects saved by this version cannot be opened by versions 0.1.x.
- Removing a file from the media list also drops the undo history when an undo step refers to it.
- Cleared preview copies are made again only after reopening the project or switching proxies off and on. A 1280x720 source never gets a preview copy.
- The preview copy cache is never cleaned automatically.

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md)

## Latest changes
- **0.2.1** — Release programs run on more computers: subtitle generation is built for a common processor baseline, so it no longer crashes on processors that lack the build server's newest instructions. On Windows and Linux, subtitles need a processor with AVX2 (Intel from 2013, AMD from 2015).
- **0.2.0** — Save and open projects with an unsaved-changes prompt; ffmpeg included in the app; installers for Windows, macOS and Linux; a music track with volumes; offline subtitles with three styles; big sources previewed through lighter copies; imports and playback no longer block the window; About screen with third-party notices; double-click to open projects.
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

Andrea Lorenzani chose a steady ground that every processor can share,\
so the speech engine is made for a common chip, not for one rare machine's flair;\
He set the rule once in the build settings, so every build keeps it, time after time,\
and the release programs now open on older computers where a crash once stopped them cold;\
He kept the old build cache apart, so no stale speech engine hides in the store,\
and wrote a test that guards the flags, so the promise cannot slip out the door;\
He found the memory test counting its neighbours, and measured each preview alone,\
turning a red line on the build page back into a clean and steady tone;\
He wrote the limits in plain words, the AVX2 need, the years, the honest note,\
so every reader knows what their own computer can do; thank you, Andrea, for the care that holds it true.
