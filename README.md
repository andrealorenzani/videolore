# videolore

![version](https://img.shields.io/badge/version-2.0.4-blue)
![rust](https://img.shields.io/badge/rust-1.100%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-95.0%25-brightgreen)
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
![harfrust](https://img.shields.io/badge/harfrust-0.12-blue)
![skrifa](https://img.shields.io/badge/skrifa-0.44-blue)
![image](https://img.shields.io/badge/image-0.25-blue)
![ffmpeg](https://img.shields.io/badge/ffmpeg-9.0.2%20bundled-informational)
[![build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml/badge.svg)](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml)

## What it is
`videolore` is an offline desktop video editor for short clips made for social media. Import video and audio, then cut, copy, paste, merge, trim and move pieces on one timeline that closes its gaps itself. Add fades through black between clips and a music track with its own volume. Set the video's shape (4:3, 16:9 or 9:16), zoom and move pictures with keyframes, and colour them with a `.cube` LUT. Add titles and captions as text items, with fades, blinking and sliding effects. Subtitles are generated on your computer from the speech, or taken from the video, in styles with word-by-word highlight. Watch the result with sound, clean up background noise with Clean audio, and export one file, its sound or its subtitles, optionally with your computer's video hardware. Source files are never modified; projects can be saved, reopened and double-clicked. ffmpeg is included and set up on first start. Nothing is uploaded and no account is needed; the only download is the speech model you approve. 360° videos can be explored and exported flat or as 360 video. A separate player watches videos with sound and subtitles. Multitrack editing and colour grading beyond LUTs are out of scope.

## System requirements
- **Windows** (64-bit, x86_64) and **Linux** (64-bit, x86_64; current Ubuntu LTS and Fedora).
- **macOS** on Apple Silicon: every Apple Silicon Mac is supported.
- **Generating subtitles** on Windows and Linux needs a processor with AVX2, FMA and F16C: Intel from 2013 (Haswell) on, AMD from 2015 (Excavator) on, all Ryzen. On an older processor videolore says so in a message; the rest of the program works, and you can still add subtitle lines by hand. Everything else in videolore has no special processor requirement.
- **Internet**: only for the download of the speech model you choose, which you approve: 32 MB (Fast), 190 MB (Balanced) or 574 MB (Most accurate). You can also provide the model file yourself.

## Installation
Each release has one installer per system, with ffmpeg included. Download it from the [Releases](https://github.com/andrealorenzani/videolore/releases) page (tagged versions), or from the artifacts of the latest [Build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml) run.

The installers are not code-signed yet, so your system asks for a one-time confirmation the first time you open videolore. The steps are below.

The **videolore player** (for watching videos) is a separate download, with the same installer formats and names as videolore but `videolore-player` in place of `videolore`. Its steps are the same as below.

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
The player is built with `cargo build --release --locked -p videolore-player`.

## Usage
Start videolore from the menu, the Applications folder, the AppImage or the extracted folder. The window has a menu bar (File, Edit, Playback, Effects, View, Help) and a toolbar at the top, four sections in the middle (Media and Preview on top, Timeline and Effects below) and a status bar at the bottom, which shows the latest message and the progress of background work, with Cancel where it applies. Drag the borders between the sections to resize them; View > Reset layout restores the default. Every action is in the menus with its shortcut, and a greyed-out action says why in a tooltip. Press F1 for the list of shortcuts. A splash screen with the release summary opens at each start of the editor, unless a project or video is given on the command line; close it with its Close button or Esc. Buttons, menu items and list rows show icons (Phosphor Icons, MIT licence).

1. **Import** video or audio files with File > Import media… (Ctrl/Cmd+I), or drop them onto the window (Windows, macOS and Linux with X11). Each file shows "analysing" while it is read, then its name and duration; unreadable files are rejected with a message. Importing never blocks the window.
2. **Build the clip** by appending media to the timeline, then split, cut, copy, paste, merge, trim or move the pieces. Double-click a clip to open its Clip details, or right-click it for its menu.
3. **Add effects**: fades through black between two clips (use the "Add fade" handle at a clip boundary, or Effects > Fades). Drag an effect from the Effects list onto a clip, or right-click the clip and choose Effects, to apply it to that clip only. Double-click an effect in the list to apply it to every clip on the timeline at that moment, each clip getting its own copy; clips added later do not get it. For Fade IN/OUT, the fade from black goes to the first clip's start and the fade to black to the last clip's end. Each fade has a marker on the timeline; click it to change or remove that fade.
4. **Change the frame and the picture**: double-click **Change aspect** in the Effects list to set the whole video's shape to 4:3, 16:9 or 9:16. **Fit on width** or **Fit on height** decides how the picture fits; the empty space is filled with a colour you choose or with a blurred copy of the video. **Scale** and **Move** zoom and shift the picture of one clip, or of every clip on the timeline at once: drag them onto a clip, or double-click them for the whole video, which gives each clip its own copy of the keyframes. Drag a corner or a visible edge of the box to scale (10% to 400%), or the centre target to move. Each popup has a keyframe timeline with **Add keyframe** and **Remove keyframe**; the value changes evenly between two keyframes and stays the same before the first and after the last.
5. **Add music**: place audio files on the music track below the video. Music can be moved, trimmed, split and deleted. The volume and mute of the video audio, for the whole video or for one clip, and of the music are set with Effects > Volume. **Clean audio** (in the Effects list, for one clip or the whole video) reduces background noise such as wind, music or people chatting in the video's sound; it does not change the music. Music does not move when you edit the video, and it ends with a fade-out where the video ends.
6. **Add subtitles**: choose **Generate subtitles** in the Subtitles window (Effects > Subtitles), or drag Subtitles onto one clip to create them only for that clip. The first time you use a subtitle quality level, you are asked to approve the download of its speech model (32 MB for Fast, 190 MB for Balanced, 574 MB for Most accurate), or you can provide the model file yourself. Choose the level in Settings. Recognition runs on your computer in the background, the language is detected automatically, and you can edit and re-word every line, and re-time it, from its marker on the timeline. If the video already contains a subtitle track (SRT, ASS/SSA, MP4 timed text or WebVTT), you can choose to use its text instead of recognising the speech; the words are then matched to the speech. If the video's subtitle text does not follow the speech, a message says so. Choose a style: Classic, Box or Word highlight. Subtitles are visible in the preview and burned into the exported video. Drag a subtitle line in the preview to move it; Reset position puts it back. Lines shown at the same time whose boxes overlap are marked with a warning; nothing moves by itself. Ctrl/Cmd+click selects several lines, and dragging one of them moves the group. File > Export subtitles (.srt)… saves the lines with their timeline times.
7. **Preview** the result with play, pause and seek from the playhead. Very large sources get a smaller preview copy, made in the background; the export always uses your original files. **Preview quality** in Settings chooses whether lighter preview copies are made, and the copies can be cleared there.
8. **Save** the project (**Save** or **Save as**, Ctrl/Cmd+S or Ctrl/Cmd+Shift+S) and reopen it later with **Open**, or from File > Open recent, which lists the last 10 projects. A `•` in the window title means unsaved changes. New, Open, closing the window and Quit ask whether to **Save**, **Discard** or **Cancel**. Projects are `.vlproj` files: double-click one to open it in videolore.
9. **Export** to one file (File > Export…, the toolbar, or Ctrl/Cmd+E) opens the Export dialog. Choose the folder and file name, and the mode: **Full video**, **Only audio** (the sound of the whole timeline, with music, volumes and Clean audio, as one audio file) or **Only subtitles (.srt)**. For a full video, choose the video format: **Same as source**, the default, or one of 12 presets (H.264 or HEVC, MP4 or MOV, several resolutions and frame rates). Choose the audio format too: **Same as source**, aac, or mp3 when it is available. With **Same as source** the format is chosen automatically: if all clips share the same format, with no fades, music, subtitles, text items, frame effects or volume changes and cuts on keyframes, the streams are copied without re-encoding; otherwise the video is re-encoded once, letterboxed and never stretched. A project whose clips are all 360 also offers a 360 video or a flat camera view. The dialog remembers its choices for the session only. Export shows progress in the status bar; cancel stops it and leaves no partial file. HEVC files play on Apple devices. With **Export: Faster** in Settings, a re-encoded H.264 or HEVC video is made by your computer's video hardware when it can; every such file is checked, and if the hardware is missing or does not produce a correct file, the normal method is used automatically. The default, **Best quality**, always uses the normal method.
10. **Settings** (Edit > Settings…, or Ctrl/Cmd+,): choose **Preview quality**, **Subtitle quality**, **How hard videolore works** and **Export** (Faster or Best quality), each explained in one line. The first start picks recommended choices for your computer; **Use recommended settings** sets them again, and the speech files you downloaded can be removed there to free disk space. The **Advanced** section holds the ffmpeg you may want to use instead of the bundled one.
11. **Boxes on the picture**: everything you can move on the picture, the subtitle lines and the text items, has a thin box in the preview. Drag the box to move it; drag a corner or a visible edge to make a subtitle line bigger or smaller (its text size is 25% to 400% of its style), or drag the left or right edge of a text item to change its width, or its bottom edge to change its height. Items can be dragged partly past the picture edge; the centre of the box stays on the picture, and a box partly outside the picture resizes through its visible edges. The status bar counts the items partly outside the picture, and **Move inside the picture** (Context Menu and Edit menu) moves the selected ones back in. The Context Menu group of the toolbar has **Align horizontally**, **Align vertically** and **Reset** for subtitle lines and text items; Reset puts a line back in its place at its usual size. Hide the boxes with View > Show boxes on the picture. The boxes never appear in the export.
12. **360 video**: a video recorded as a full 360° sphere is recognised from its file's own information and shown as a 360 clip; a video without that information stays flat. Drag the picture to look around, and use the mouse wheel to zoom, during playback too. **Set POV** saves the current view at the playhead; between two saved views the camera moves in a straight line. On export, a project whose clips are all 360 offers in the Export dialog a 360 video, which keeps the whole sphere, or a flat video that follows this camera path (a project that mixes 360 and flat clips exports as a flat camera view); a moving camera exports at about one third of real time. A text or subtitle line added on a 360 clip stays at its point of the sphere.
13. **Titles and captions**: choose **Add Text** in the Effects list, or Effects > Add text at playhead, to write a multiline text at the playhead. Choose its effects (Fade IN, Fade OUT, Blink and Slide in, in any combination), its time to live, colour and font (the fonts installed on your computer). The text has a bar on the Text lane of the timeline: drag the bar to move it, or drag its right end to change how long it is shown. Choose its **Readability**: None, Outline or Box (a background box behind the text). Drag its box in the preview to move it, its right or left edge to change its width, or its bottom edge to change its height; the text size fits the box, and the width is shown while you drag. Texts are burned into the exported video.
14. **Colour with a LUT**: drag **Apply LUT** from the Effects list onto a clip, or double-click it to apply it to every clip on the timeline at that moment, each clip getting its own copy. **Choose file…** opens a `.cube` file (3D or 1D LUT); a file that is not a valid LUT is refused with a message. The project keeps the file's path, not a copy, so keep the file where it is. **Remove effect** takes the LUT off. A missing or damaged LUT file is reported in the notice that lists items which could not be found or were repaired.
15. **Video player**: videolore player (`videolore-player`) is a separate program with its own download (see Installation). It opens one video at a time, with File > Open… or by dropping a video onto its window where drag and drop works, and plays it with sound. Started without a video, it shows a splash screen with its own picture and the release summary; close it with its Close button or Esc, or by opening a video. Drag a 360 picture to look around. The Subtitles menu lists None, the video's text subtitle tracks and any `.srt` file you open with Subtitles > Open .srt…. View > Full screen, F11 or F switches full screen on and off. The player keeps no settings and has no playlist.

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
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open… there.
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text. Paste in the Edit menu and the toolbar always works.
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- The window size and position are not remembered, and popups keep their place only until videolore is closed.
- On Windows and Linux, double-clicking a project while videolore is open opens a second window. On macOS the menu bar is inside the window, not at the top of the screen, and the Hide and Minimize menu items are not available.
- Projects saved by version 2.0.3 cannot be opened by 2.0.2 or earlier; they are refused with a message and left untouched.
- Projects saved by version 1.4.0 or later cannot be opened by version 1.3.0 or earlier; they are refused with a message and left untouched. Projects from 1.3.0 and earlier open with the same result, except as listed below.
- When a video's own subtitles are used, their text and line times are kept exactly, but the word-by-word highlight timing is approximate: the accuracy target is not met yet with the Fast model (on the reference clip 82% of the words were matched, with an average start error of 0.48 s).
- Matching a video's subtitles to the speech works only as well as their text follows what is said; translated or paraphrased subtitles often get evenly spread words, and a message says so.
- Moved subtitle lines shown at the same time can overlap; a warning marks them, but nothing moves them apart. Use Reset position or drag one of them.
- The `.srt` file has no subtitle positions.
- Removing a file from the media list also drops the undo history when an undo step refers to it.
- Cleared preview copies are made again only after reopening the project or changing Preview quality. Under Automatic, a 1280x720 source never gets a preview copy.
- The preview copy cache is never cleaned automatically.
- A 360 clip with a moving camera exports at about one third of real time.
- 360 video must be a single equirectangular picture (no stereo 3D, cube maps or partial 180° video). The camera cannot roll or be steered from the keyboard, and spatial sound does not follow the camera.
- Text items use the fonts installed on your computer (so they can look different on another computer), and a font that is not installed is replaced by the bundled subtitle font, with a note in the text's popup. Text items are not written to the `.srt` file.
- A project file with a text whose time to live is above 24 hours, or below 0.1 s, is refused as damaged on opening; while editing, a text's time to live is limited to the video's length.
- A subtitle line placed on a 360 clip is plain white text, with no preset box or word highlight.
- A 360 video export is marked as 360 video only in MP4 and MOV files.
- Clean audio has one fixed strength: it reduces background noise but does not remove it completely. Quiet speech (about -32 dB) can lose up to about 2.5 dB, and its cost on weak computers is not measured.
- A text is never larger than 10% of the picture height, so a short text in a big box does not fill it. Its size is measured with the font the picture uses.
- Texts of projects saved before 2.0.3 open with Readability None and take the measured size of their box when opened, so large multi-line texts may shrink; a one-time message says so. A 360 switch set in a 1.3.0 project is dropped when it opens.
- A text or a subtitle line dragged past an edge of the picture is cut in the preview and in the export; the status bar counts such items, and Move inside the picture brings them back.
- A video without spherical information is always flat: the "Treat as 360° video" switch is removed.
- The Export dialog remembers its choices for the session only, not in the project or in Settings. The video and audio formats are a fixed list of presets.
- A 360 video ignores the video format presets and keeps the sphere's own size.
- A 360 video export ignores Change aspect, Scale and Move; the flat camera view applies them.
- Change aspect, Scale and Move make the export a re-encode, even at their default values.
- The longer side of a frame is capped at 4096 px: a Change aspect on a large source can give a smaller frame, and a large frame exports more slowly (export time not measured).
- Keyframes are straight lines with no easing, cannot be copied from one clip to another, and show in the popup and on the clip's marker, not as separate timeline markers.
- Subtitle lines and text items keep their place in the frame and do not follow Scale or Move.
- The Scale corners and the Move target show one effect per kind: the clip's own, otherwise the whole video's.
- A 1.6.0 clip with its own Scale or Move and a whole-video one becomes one combined path when opened: exact at the keyframe times, a straight line between them.
- Clips added after a whole-video effect do not get it, and nothing says so.
- After a clip's keyframes are changed alone, the whole-video Scale or Move popup shows and changes only the first clip's keyframes.
- The blur of Fit empty with blurred video has one fixed strength.
- Apply LUT reads `.cube` files only, at full strength. The project keeps the LUT's path, not a copy, so a moved LUT file stops the export until it is found again.
- Apply LUT is applied before the frame effect of Change aspect, so the blurred or coloured fill is not graded.
- The Change aspect row in the Effects list has no tooltip and cannot be dragged.
- The player has no settings, no playlist and no file association, lists text subtitle tracks only (no bitmap subtitles such as Blu-ray or DVB), plays very large videos from a silent preview copy, and its About screen shows the editor's list of third-party components.

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md) · [roadmap](docs/roadmap.md)

## Latest changes
- **2.0.4** — Subtitle files (WebVTT and ASS export, .srt and .vtt import, a separate subtitle track), autosave and crash recovery, drag to trim and re-time, faster hardware export with AV1 and VP9, a 360 marker for MKV and WebM, remembered window layouts, workspaces, themes and screen-reader names, the player's file association and playlist, and new effects (fade length, Crop, volume keyframes, Clean audio strength, LUT strength).
- **2.0.3** — Readable text: texts can have an outline or a background box (Readability), their size is measured to fit the box, and the box height and the width can be set with handles; moved subtitle lines and text items get warnings for overlaps and can be moved back inside the picture; a paused picture at the end of a clip is shown again; projects saved by this version are refused by 2.0.2 and older.
- **2.0.2** — The splash summary is shown in the largest text that fits its popup, down to a smaller size for a longer summary; it is never scrolled.
Older changes: [changelog](docs/changelog.md) and the release notes in [releases](releases/).

## License
Copyright (C) 2026 Andrea Lorenzani.
Released under the GNU General Public License v3.0 — see [LICENSE](LICENSE).

---

### Ode to the creator

Andrea Lorenzani began videolore from one idea, a small editor for the clips people share,\
and gave it a single timeline that closes its own gaps, so a cut never leaves a hole behind,\
then added fades through black, a music track with its own volume, and keyframes that move and zoom each picture,\
and taught it to read a .cube colour look, to write subtitles from the speech on the computer itself, and to burn them in,\
and gave it a full 360 view to look around, and a separate player to watch the result with sound,\
and kept every source file untouched, let projects be saved, reopened and double-clicked, and sent nothing away,\
and shaped the exports to copy the streams when they can, to encode once when they must, and to use the video hardware only when it is safe,\
and wrote the tests, the coverage, the notices and the installers, and an honest list of every limit that remains,\
and with 1.6.3 the core effects and the player went out, and the next major version opened to 2.0.0 with the same care,\
thank you, Andrea, for building this editor so patiently and sharing it freely under the GPL.
