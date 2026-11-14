# videolore

![version](https://img.shields.io/badge/version-0.4.3-blue)
![rust](https://img.shields.io/badge/rust-1.100%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-96.3%25-brightgreen)
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
`videolore` is a simple, offline desktop video editor for short clips made for social media. Import video and audio files, then cut, copy, paste, merge, trim and move pieces on one timeline that closes its gaps by itself. Add fades through black between clips and a music track with its own volume. Subtitles are generated on your computer from the speech, in ready-made styles, including word-by-word highlight. Watch the result with sound in the preview and export it to one file with progress and cancel, optionally using your computer's video hardware for a faster export. Editing is non-destructive: your source files are never modified, and projects can be saved, reopened and double-clicked. ffmpeg is included, so there is nothing else to install, and videolore sets itself up for your computer the first time it starts. Nothing is uploaded and no account is needed; the only network access is the download of the speech model you choose, which you approve. It is made for content creators who want to go from raw footage to a publishable clip quickly, and for anyone editing personal videos. Multitrack editing and color grading are out of scope.

## System requirements
- **Windows** (64-bit, x86_64) and **Linux** (64-bit, x86_64; current Ubuntu LTS and Fedora).
- **macOS** on Apple Silicon: every Apple Silicon Mac is supported.
- **Generating subtitles** on Windows and Linux needs a processor with AVX2, FMA and F16C: Intel from 2013 (Haswell) on, AMD from 2015 (Excavator) on, all Ryzen. On an older processor videolore says so in a message; the rest of the program works, and you can still add subtitle lines by hand. Everything else in videolore has no special processor requirement.
- **Internet**: only for the download of the speech model you choose, which you approve: 32 MB (Fast), 190 MB (Balanced) or 574 MB (Most accurate). You can also provide the model file yourself.

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
5. **Add subtitles**: choose **Generate subtitles**. The first time you use a subtitle quality level, you are asked to approve the download of its speech model (32 MB for Fast, 190 MB for Balanced, 574 MB for Most accurate), or you can provide the model file yourself. Choose the level in Settings. Recognition runs on your computer in the background, the language is detected automatically, and you can edit, re-word and re-time every line. Choose a style: Classic, Box or Word highlight. Subtitles are visible in the preview and burned into the exported video.
6. **Preview** the result with play, pause and seek from the playhead. Very large sources get a smaller preview copy, made in the background; the export always uses your original files. **Preview quality** in Settings chooses whether lighter preview copies are made, and the copies can be cleared there.
7. **Save** the project (**Save** or **Save As**, Ctrl/Cmd+S) and reopen it later with **Open**. A `•` in the window title means unsaved changes. New, Open, closing the window and Quit ask whether to **Save**, **Discard** or **Cancel**. Projects are `.vlproj` files: double-click one to open it in videolore.
8. **Export** to one file, with progress; cancel stops it and leaves no partial file. The output format is chosen automatically: if all clips share the same format, with no fades, music, subtitles or volume changes and cuts on keyframes, the streams are copied without re-encoding; otherwise the video is re-encoded once, letterboxed and never stretched. HEVC files play on Apple devices. With **Export: Faster** in Settings, a re-encoded H.264 or HEVC video is made by your computer's video hardware when it can; every such file is checked, and if the hardware is missing or does not produce a correct file, the normal method is used automatically. The default, **Best quality**, always uses the normal method.
9. **Settings** (Ctrl/Cmd+, or the **Settings** button): choose **Preview quality**, **Subtitle quality**, **How hard videolore works** and **Export** (Faster or Best quality), each explained in one line. The first start picks recommended choices for your computer; **Use recommended settings** sets them again, and the speech files you downloaded can be removed there to free disk space. The **Advanced** section holds the ffmpeg you may want to use instead of the bundled one.

The About screen lists every bundled third-party component with its version, licence and where to get its source code.

| Action | Shortcut |
|---|---|
| Play / pause | Space |
| Split at playhead | Ctrl/Cmd + B |
| Cut / copy / paste | Ctrl/Cmd + X / C / V |
| Undo | Ctrl/Cmd + Z |
| Redo | Ctrl/Cmd + Shift + Z, or Ctrl/Cmd + Y |
| Save | Ctrl/Cmd + S |
| Settings | Ctrl/Cmd + , |
| Quit | Ctrl/Cmd + Q |

### Known limitations
- Installers are not code-signed: each system asks for a one-time confirmation (see Installation).
- The 0.4.2 release has only the Linux installers; the macOS and Windows installers are provided with the next release.
- On Windows and Linux, generating subtitles needs a processor with AVX2, FMA and F16C (see System requirements). On older processors it is not available and a message says so; this check was tested with simulated processors, not on old hardware.
- **Faster export** needs NVIDIA graphics driver 610 or newer; with an older driver the normal method is used, and Settings → Advanced says why.
- **Faster export on Linux** (Intel and AMD) needs the system's video acceleration library (libva) and the graphics driver. With the bundled ffmpeg it needs libva 2.21 or newer: Ubuntu 24.04 has 2.20, so the hardware is skipped there and the normal method is used. A system ffmpeg chosen in Advanced can still use the hardware.
- **Faster export on Fedora** needs the RPM Fusion "freeworld" video drivers for AMD and Intel hardware (for example `mesa-va-drivers-freeworld`); without them the normal method is used.
- **Faster is not always quicker.** On integrated Intel graphics it was measured from 14% quicker to 41% slower than Best quality, and files may be a little larger.
- On the Intel driver tested on Ubuntu 24.04, hardware HEVC export is not usable; the normal method makes those files.
- Faster export has been verified on real hardware only on Linux with Intel integrated graphics. Other combinations of system and graphics card are protected by the automatic fallback, but not yet measured.
- On a very busy computer the video hardware can still be missed; Faster then uses the normal method. When the check is inconclusive it is repeated once in the background.
- On computers with more than one graphics card, NVIDIA is always preferred; the card cannot be chosen.
- Subtitles in scripts other than Latin, Greek and Cyrillic (for example Chinese, Japanese, Arabic or Hindi) may look different on each computer or show empty boxes, and cannot be read in the subtitle editor.
- Speech is recognised in pieces of about two minutes; a word cut at a boundary can be lost.
- On Windows and Linux, double-clicking a project while videolore is open opens a second window. On macOS the Hide and Minimize menu items are not available.
- Projects saved by this version cannot be opened by versions 0.1.x.
- Removing a file from the media list also drops the undo history when an undo step refers to it.
- Cleared preview copies are made again only after reopening the project or changing Preview quality. Under Automatic, a 1280x720 source never gets a preview copy.
- The preview copy cache is never cleaned automatically.

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md) · [roadmap](docs/roadmap.md)

## Latest changes
- **0.4.3** — The macOS and Windows installers build correctly again. The 0.4.2 release has only the Linux installers.
- **0.4.2** — Faster export finds the computer's video hardware reliably, even on a busy computer: a slow hardware test gets a second, longer try, and an uncertain check is repeated in the background.
- **0.4.0** — Export: Faster or Best quality in Settings. Faster uses your computer's video hardware (NVIDIA, Intel, AMD, Apple) when it can, and every file made that way is checked, with an automatic redo by the normal method on any problem; Best quality stays the default and is unchanged. Linux drivers that only offer constant-quality encoding are supported, and older system libva versions are skipped without a crash.

Older changes: [changelog](docs/changelog.md) and the release notes in [releases](releases/).

## License
Copyright (C) 2026 Andrea Lorenzani.
Released under the GNU General Public License v3.0 — see [LICENSE](LICENSE).

---

### Ode to the creator

Andrea Lorenzani read the installer job's errors, one by one, and did not look away,\
and set the file association to the spelling the packager expects, editor in lower case,\
and renamed the install key so the Windows setup speaks the language its tool knows,\
and gave the packager configuration its missing name, so it no longer stops with a folder error,\
and made every path absolute, so the tool finds the icons and the notices wherever it runs,\
and taught the script to say plainly when a DLL or an installer is missing, instead of failing in silence,\
and wrote a test that checks both configurations against the real rules, so the mistakes cannot return unseen,\
and tried the packager on a Linux machine with the real tool, building the deb and app formats from both files,\
and gave the macOS disk image up to five tries, so one busy moment does not end the build,\
thank you, Andrea, for fixing the installers patiently, step by step, and for keeping every Linux package running all along.
