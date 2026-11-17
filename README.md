# videolore

![version](https://img.shields.io/badge/version-1.4.0-blue)
![rust](https://img.shields.io/badge/rust-1.100%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-95.8%25-brightgreen)
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
![fontdb](https://img.shields.io/badge/fontdb-0.24-blue)
![ffmpeg](https://img.shields.io/badge/ffmpeg-9.0.2%20bundled-informational)
[![build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml/badge.svg)](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml)

## What it is
`videolore` is an offline desktop video editor for short clips made for social media. Import video and audio files, then cut, copy, paste, merge, trim and move pieces on one timeline that closes its gaps itself. Add fades through black between clips and a music track with its own volume. Add titles and captions as text items, with fades, blinking and sliding effects. Subtitles are generated on your computer from the speech, or taken from a video's own subtitles, in styles with word-by-word highlight. Watch the result with sound, clean up its background noise with Clean audio, and export it to one file or only its sound or subtitles, optionally using your computer's video hardware. Your source files are never modified, and projects can be saved, reopened and double-clicked. ffmpeg is included and set up at the first start. Nothing is uploaded and no account is needed; the only network access is the download of the speech model you choose, which you approve. Made for content creators who want to go from raw footage to a publishable clip. 360° videos can be explored and exported flat or as 360 video. Multitrack editing and color grading are out of scope.

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
Start videolore from the menu, the Applications folder, the AppImage or the extracted folder. The window has a menu bar (File, Edit, Playback, Effects, View, Help) and a toolbar at the top, four sections in the middle (Media and Preview on top, Timeline and Effects below) and a status bar at the bottom, which shows the latest message and the progress of background work, with Cancel where it applies. Drag the borders between the sections to resize them; View > Reset layout restores the default. Every action is in the menus with its shortcut, and a greyed-out action says why in a tooltip. Press F1 for the list of shortcuts.

1. **Import** video or audio files with File > Import media… (Ctrl/Cmd+I), or drop them onto the window (Windows, macOS and Linux with X11). Each file shows "analysing" while it is read, then its name and duration; unreadable files are rejected with a message. Importing never blocks the window.
2. **Build the clip** by appending media to the timeline, then split, cut, copy, paste, merge, trim or move the pieces. Double-click a clip to open its Clip details, or right-click it for its menu.
3. **Add effects**: fades through black between two clips (use the "Add fade" handle at a clip boundary, or Effects > Fades), and fades from black at the start and to black at the end of the video. Drag an effect from the Effects list onto a clip, or right-click the clip and choose Effects, to apply it to that clip only; double-click an effect in the list to apply it to the whole video. Each fade has a marker on the timeline; click it to change or remove that fade.
4. **Add music**: place audio files on the music track below the video. Music can be moved, trimmed, split and deleted. The volume and mute of the video audio, for the whole video or for one clip, and of the music are set with Effects > Volume. **Clean audio** (in the Effects list, for one clip or the whole video) reduces background noise such as wind, music or people chatting in the video's sound; it does not change the music. Music does not move when you edit the video, and it ends with a fade-out where the video ends.
5. **Add subtitles**: choose **Generate subtitles** in the Subtitles window (Effects > Subtitles), or drag Subtitles onto one clip to create them only for that clip. The first time you use a subtitle quality level, you are asked to approve the download of its speech model (32 MB for Fast, 190 MB for Balanced, 574 MB for Most accurate), or you can provide the model file yourself. Choose the level in Settings. Recognition runs on your computer in the background, the language is detected automatically, and you can edit and re-word every line, and re-time it, from its marker on the timeline. If the video already contains a subtitle track (SRT, ASS/SSA, MP4 timed text or WebVTT), you can choose to use its text instead of recognising the speech; the words are then matched to the speech. Choose a style: Classic, Box or Word highlight. Subtitles are visible in the preview and burned into the exported video. Drag a subtitle line in the preview to move it; Reset position puts it back. Ctrl/Cmd+click selects several lines, and dragging one of them moves the group. File > Export subtitles (.srt)… saves the lines with their timeline times.
6. **Preview** the result with play, pause and seek from the playhead. Very large sources get a smaller preview copy, made in the background; the export always uses your original files. **Preview quality** in Settings chooses whether lighter preview copies are made, and the copies can be cleared there.
7. **Save** the project (**Save** or **Save as**, Ctrl/Cmd+S or Ctrl/Cmd+Shift+S) and reopen it later with **Open**, or from File > Open recent, which lists the last 10 projects. A `•` in the window title means unsaved changes. New, Open, closing the window and Quit ask whether to **Save**, **Discard** or **Cancel**. Projects are `.vlproj` files: double-click one to open it in videolore.
8. **Export** to one file (File > Export…, the toolbar, or Ctrl/Cmd+E) opens the Export dialog. Choose the folder and file name, and the mode: **Full video**, **Only audio** (the sound of the whole timeline, with music, volumes and Clean audio, as one audio file) or **Only subtitles (.srt)**. For a full video, choose the video format: **Same as source**, the default, or one of 12 presets (H.264 or HEVC, MP4 or MOV, several resolutions and frame rates). Choose the audio format too: **Same as source**, aac, or mp3 when it is available. With **Same as source** the format is chosen automatically: if all clips share the same format, with no fades, music, subtitles, text items or volume changes and cuts on keyframes, the streams are copied without re-encoding; otherwise the video is re-encoded once, letterboxed and never stretched. A project whose clips are all 360 also offers a 360 video or a flat camera view. The dialog remembers its choices for the session only. Export shows progress in the status bar; cancel stops it and leaves no partial file. HEVC files play on Apple devices. With **Export: Faster** in Settings, a re-encoded H.264 or HEVC video is made by your computer's video hardware when it can; every such file is checked, and if the hardware is missing or does not produce a correct file, the normal method is used automatically. The default, **Best quality**, always uses the normal method.
9. **Settings** (Edit > Settings…, or Ctrl/Cmd+,): choose **Preview quality**, **Subtitle quality**, **How hard videolore works** and **Export** (Faster or Best quality), each explained in one line. The first start picks recommended choices for your computer; **Use recommended settings** sets them again, and the speech files you downloaded can be removed there to free disk space. The **Advanced** section holds the ffmpeg you may want to use instead of the bundled one.
10. **Boxes on the picture**: everything you can move on the picture, the subtitle lines and the text items, has a thin box in the preview. Drag the box to move it; drag a corner to make a subtitle line bigger or smaller (its text size is 25% to 400% of its style), or drag the right edge of a text item to change its width. Items can be dragged partly past the picture edge; the centre of the box stays on the picture. The Context Menu group of the toolbar has **Align horizontally**, **Align vertically** and **Reset** for subtitle lines and text items; Reset puts a line back in its place at its usual size. Hide the boxes with View > Show boxes on the picture. The boxes never appear in the export.
11. **360 video**: a video recorded as a full 360° sphere is recognised from its file's own information and shown as a 360 clip; a video without that information stays flat. Drag the picture to look around, and use the mouse wheel to zoom, during playback too. **Set POV** saves the current view at the playhead; between two saved views the camera moves in a straight line. On export, a project whose clips are all 360 offers in the Export dialog a 360 video, which keeps the whole sphere, or a flat video that follows this camera path (a project that mixes 360 and flat clips exports as a flat camera view); a moving camera exports at about one third of real time. A text or subtitle line added on a 360 clip stays at its point of the sphere.
12. **Titles and captions**: choose **Add Text** in the Effects list, or Effects > Add text at playhead, to write a multiline text at the playhead. Choose its effects (Fade IN, Fade OUT, Blink and Slide in, in any combination), its time to live, colour and font (the fonts installed on your computer). The text has a bar on the Text lane of the timeline: drag the bar to move it, or drag its right end to change how long it is shown. Drag its box in the preview to move it, or its right edge to change its width; the text size follows the width. Texts are burned into the exported video.

The About screen lists every bundled third-party component with its version, licence and where to get its source code.

| Action | Shortcut |
|---|---|
| New project | Ctrl/Cmd + N |
| Open | Ctrl/Cmd + O |
| Save / Save as | Ctrl/Cmd + S / Ctrl/Cmd + Shift + S |
| Import media | Ctrl/Cmd + I |
| Export | Ctrl/Cmd + E |
| Play / pause | Space |
| Previous / next frame | Left / Right |
| Go to start / end | Home / End |
| Split at playhead | Ctrl/Cmd + B |
| Delete | Delete or Backspace |
| Cut / copy / paste | Ctrl/Cmd + X / C / V |
| Undo | Ctrl/Cmd + Z |
| Redo | Ctrl/Cmd + Shift + Z, or Ctrl/Cmd + Y |
| Zoom in / out / fit project | Ctrl/Cmd + = / Ctrl/Cmd + - / Ctrl/Cmd + 0 |
| Settings | Ctrl/Cmd + , |
| Keyboard shortcuts | F1 |
| Select several subtitle lines or text items | Ctrl/Cmd + click on markers, text bars, boxes or in the list; Shift + drag on the timeline |
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
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open… there.
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text. Paste in the Edit menu and the toolbar always works.
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- The window size and position are not remembered, and popups keep their place only until videolore is closed.
- On Windows and Linux, double-clicking a project while videolore is open opens a second window. On macOS the menu bar is inside the window, not at the top of the screen, and the Hide and Minimize menu items are not available.
- Projects saved by this version cannot be opened by version 1.3.0 or earlier; they are refused with a message and left untouched. Projects from 1.3.0 and earlier open with the same result, except as listed below.
- When a video's own subtitles are used, their text and line times are kept exactly, but the word-by-word highlight timing is approximate: the accuracy target is not met yet with the Fast model (on the reference clip 82% of the words were matched, with an average start error of 0.48 s).
- Matching a video's subtitles to the speech works only as well as their text follows what is said; translated or paraphrased subtitles often get evenly spread words.
- Moved subtitle lines shown at the same time can overlap, and editing a line's text after moving it can push it past the edge of the picture in the export; use Reset position or drag it again.
- Start and end fades stay at the edges of the video when clips are moved.
- The `.srt` file has no subtitle positions.
- Removing a file from the media list also drops the undo history when an undo step refers to it.
- Cleared preview copies are made again only after reopening the project or changing Preview quality. Under Automatic, a 1280x720 source never gets a preview copy.
- The preview copy cache is never cleaned automatically.
- A 360 clip with a moving camera exports at about one third of real time.
- 360 video must be a single equirectangular picture (no stereo 3D, cube maps or partial 180° video). The camera cannot roll or be steered from the keyboard, and spatial sound does not follow the camera.
- Text items have no outline or background box, use the fonts installed on your computer (so they can look different on another computer) and are not written to the `.srt` file.
- A project file with a text whose time to live is above 24 hours, or below 0.1 s, is refused as damaged on opening; while editing, a text's time to live is limited to the video's length.
- A subtitle line placed on a 360 clip is plain white text, with no preset box or word highlight.
- A 360 video export is marked as 360 video only in MP4 and MOV files.
- Align horizontally does nothing for a subtitle line that is still at its style's place. An enlarged line's text may wrap and shift its centre slightly.
- Clean audio has one fixed strength: it reduces background noise but does not remove it completely. Quiet speech (about -32 dB) can lose up to about 2.5 dB, and its cost on weak computers is not measured.
- The text size of a text item is an estimate of the best fit for its box, so a text can fill about 85% to 115% of it. Short texts become large and long ones small.
- Texts of projects from 1.3.0 take the best-fit size of their box when opened, so large multi-line texts may shrink. A 360 switch set in a 1.3.0 project is dropped when it opens.
- A text or a subtitle line dragged past an edge of the picture is cut in the preview and in the export.
- A video without spherical information is always flat: the "Treat as 360° video" switch is removed.
- The Export dialog remembers its choices for the session only, not in the project or in Settings. The video and audio formats are a fixed list of presets.
- A 360 video ignores the video format presets and keeps the sphere's own size.

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md) · [roadmap](docs/roadmap.md)

## Latest changes
- **1.4.0** — An Export dialog with the file, a mode (full video, only audio or only subtitles), video formats (12 presets, H.264 or HEVC, MP4 or MOV) and audio formats (aac or mp3), and a 360 choice for projects made of 360 clips. Clean audio reduces background noise in a clip's sound. A text item's box changes width only, and its text size fits the box. Texts and subtitle lines can be dragged past the picture edges, and text items have the Align and Reset actions. The 360° switch of Clip details is removed: a video is 360 only when its file says so. Projects saved by this version need version 1.4.0 or later.
- **1.3.0** — Titles and captions as text items: written at the playhead, with Fade IN, Fade OUT, Blink and Slide in effects, a time to live shown on their own lane of the timeline, and fonts installed on your computer. Text items and subtitle lines can be selected and moved together. A text or subtitle line added on a 360° video stays at its point of the sphere, and Export can make a 360 video that keeps the whole sphere. The Context Menu group appears in the toolbar only when it has something to act on, and "Fade through black" is now named "Fade IN/OUT". Projects saved by this version need version 1.3.0 or later.
- **1.2.6** — Boxes on the picture: every movable subtitle line gets a visible box to move and resize, with its own text size; Align horizontally, Align vertically and Reset in the toolbar. 360° videos can be looked around in, with points of view set by the user, and are exported as a flat video that follows that camera path. Projects saved by this version need version 1.2.6 or later.

Older changes: [changelog](docs/changelog.md) and the release notes in [releases](releases/).

## License
Copyright (C) 2026 Andrea Lorenzani.
Released under the GNU General Public License v3.0 — see [LICENSE](LICENSE).

---

### Ode to the creator

Andrea Lorenzani gathered the story of this project in one place, from its very first commit,\
and set the brief beside it, in the same words the README keeps,\
and laid every version out along a single line, with its day and its hour,\
so that a visitor can walk from the first draft to today in one unbroken stride,\
and each small step, each fix and each release, found its own sentence and its own place,\
and the page was made plain and light, to read on a phone as well as on a large screen,\
and it follows the light or the dark of the reader's own setting,\
and it states the copyright and the licence plainly, under the GPL the project was given,\
and the version rose to 1.2.4, written in every place it lives at once,\
thank you, Andrea, for bringing this story to the open web with such care and patience.
