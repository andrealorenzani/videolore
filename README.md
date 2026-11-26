# videolore

![version](https://img.shields.io/badge/version-3.1.3-blue)
![rust](https://img.shields.io/badge/rust-1.100%2B-orange?logo=rust)
![license](https://img.shields.io/badge/license-GPL--3.0-green)
![coverage](https://img.shields.io/badge/coverage-95.0%25-brightgreen)
![eframe](https://img.shields.io/badge/egui%2Feframe-0.37-blue)
![cpal](https://img.shields.io/badge/cpal-0.18-blue)
![display-info](https://img.shields.io/badge/display--info-0.5-blue)
![jiff](https://img.shields.io/badge/jiff-1.0-blue)
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
`videolore` is an offline desktop video editor for short clips made for social media. Import video and audio, then cut, copy, paste, merge, trim and move pieces on one timeline that closes its gaps itself. Add fades through black between clips and a music track with its own volume. Set the video's shape (4:3, 16:9 or 9:16), zoom and move pictures with keyframes, and colour them with a LUT (`.cube`, `.3dl` or `.csp`). Add titles and captions as text items, with fades, blinking and sliding effects. Subtitles are generated on your computer from the speech, or taken from the video, in styles with word-by-word highlight, Enlarge word and Typewriter. Watch the result with sound, clean up background noise with Clean audio, and export one file, its sound or its subtitles, optionally with your computer's video hardware. Source files are never modified; projects can be saved, reopened and double-clicked. ffmpeg is included and set up on first start. Nothing is uploaded and no account is needed; the only download is the speech model you approve. 360° videos can be explored and exported flat or as 360 video. A separate player watches videos with sound and subtitles. Multitrack editing and colour grading beyond LUTs are out of scope.

## System requirements
- **Windows** (64-bit, x86_64) and **Linux** (64-bit, x86_64; current Ubuntu LTS and Fedora).
- **macOS** on Apple Silicon: every Apple Silicon Mac is supported.
- **Generating subtitles** on Windows and Linux needs a processor with AVX2, FMA and F16C: Intel from 2013 (Haswell) on, AMD from 2015 (Excavator) on, all Ryzen. On an older processor videolore says so in a message; the rest of the program works, and you can still add subtitle lines by hand. Everything else in videolore has no special processor requirement.
- **Internet**: only for the download of the speech model you choose, which you approve: 32 MB (Fast), 190 MB (Balanced) or 574 MB (Most accurate). You can also provide the model file yourself.

## Installation
Each release has one installer per system, with ffmpeg included. Download it from the [Releases](https://github.com/andrealorenzani/videolore/releases) page (tagged versions), or from the artifacts of the latest [Build](https://github.com/andrealorenzani/videolore/actions/workflows/build.yml) run.

The installers are not code-signed yet, so your system asks for a one-time confirmation the first time you open videolore. The steps are below.

The **videolore player** (for watching videos) is a separate download, with the same installer formats and names as videolore but `videolore-player` in place of `videolore`. Its steps are the same as below. The **videolore screen spy** (`videolore-screen-spy`) is a third separate program, with its own download on the same pages; its steps are the same too.

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
Requires Rust 1.100 or newer (`rustup update`). On Linux install the ALSA headers, cmake, libclang, the XCB RandR headers (screen recorder) and a C++ compiler first, with `sudo apt install libasound2-dev pkg-config cmake libclang-dev libxcb-randr0-dev g++`. A source build uses the ffmpeg and ffprobe 4.4 or newer found on your system (`PATH`, or `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`). To build only the editor without sound and subtitles, use `cargo build --no-default-features`: it needs neither ALSA nor cmake, but the rest of the workspace still needs ALSA and libxcb-randr0-dev.
```sh
git clone git@github.com:andrealorenzani/videolore.git
cd videolore
cargo build --release --locked
```
The player is built with `cargo build --release --locked -p videolore-player`.

## Usage
Start videolore from the menu, the Applications folder, the AppImage or the extracted folder. The window has a menu bar (File, Edit, Playback, Effects, View, Help) and a toolbar at the top, four sections in the middle (Media and Preview on top, Timeline and Effects below) and a status bar at the bottom, which shows the latest message and the progress of background work, with Cancel where it applies. Drag the borders between the sections to resize them; View > Reset layout restores the default. Sections can be hidden, collapsed or docked on the other side, and View > Workspaces saves and switches layouts. Every action is in the menus with its shortcut, and a greyed-out action says why in a tooltip. Press F1 for the list of shortcuts. A splash screen with the release summary opens at each start of the editor, unless a project or video is given on the command line; close it with its Close button or Esc. Buttons, menu items and list rows show icons (Phosphor Icons, MIT licence).

1. **Import** video or audio files with File > Import media… (Ctrl/Cmd+I), or drop them onto the window (Windows, macOS and Linux with X11). Each file shows "analysing" while it is read, then its name and duration; unreadable files are rejected with a message. Importing never blocks the window.
2. **Build the clip** by appending media to the timeline, then split, cut, copy, paste, merge, trim or move the pieces. Double-click a clip to open its Clip details, or right-click it for its menu. Clip details also sets the clip's own frame shape, and **Change speed** in the Effects list sets its speed (50% to 200%): drag it onto a clip, or double-click it for every clip.
3. **Add effects**: fades through black between two clips (use the "Add fade" handle at a clip boundary, or Effects > Fades). Drag an effect from the Effects list onto a clip, or right-click the clip and choose Effects, to apply it to that clip only. Double-click an effect in the list to apply it to every clip on the timeline at that moment, each clip getting its own copy; clips added later do not get it. For Fade IN/OUT, the fade from black goes to the first clip's start and the fade to black to the last clip's end. Each fade has a marker on the timeline; click it to change or remove that fade.
4. **Change the frame and the picture**: double-click **Change aspect** in the Effects list to set the whole video's shape to 4:3, 16:9 or 9:16. **Fit on width** or **Fit on height** decides how the picture fits; the empty space is filled with a colour you choose or with a blurred copy of the video. **Scale** and **Move** zoom and shift the picture of one clip, or of every clip on the timeline at once: drag them onto a clip, or double-click them for the whole video, which gives each clip its own copy of the keyframes. Drag a corner or a visible edge of the box to scale (10% to 400%), or the centre target to move. Each popup has a keyframe timeline with **Add keyframe** and **Remove keyframe**; the value changes evenly between two keyframes (eased in and out where the segment's Ease box is ticked) and stays the same before the first and after the last.
5. **Add music**: place audio files on the music track below the video. Music can be moved, trimmed, split and deleted. The volume of the video audio, for the whole video or for one clip, is the **Volume and track** effect in the Effects list (drag it onto a clip, or right-click the clip): it takes keyframes, straight or eased, like Scale and Move. Its popup chooses the clip's audio track, the first one by default; the chosen track is the only sound of the clip. The music has its own volume. **Clean audio** (in the Effects list, for one clip or the whole video) reduces background noise such as wind, music or people chatting in the video's sound; it does not change the music. Music does not move when you edit the video, and it ends with a fade-out where the video ends.
6. **Add subtitles**: choose **Generate subtitles** in the Subtitles window (Effects > Subtitles), or drag Subtitles onto one clip to create them only for that clip. Its **Audio track** choice (the first track by default) is the speech that is recognised and timed, and its text is taken from a subtitle track of the clip when there is one. The first time you use a subtitle quality level, you are asked to approve the download of its speech model (32 MB for Fast, 190 MB for Balanced, 574 MB for Most accurate), or you can provide the model file yourself. Choose the level in Settings. Recognition runs on your computer in the background, the language is detected automatically, and you can edit and re-word every line, and re-time it, from its marker on the timeline. If the video already contains a subtitle track (SRT, ASS/SSA, MP4 timed text or WebVTT), you can choose to use its text instead of recognising the speech; the words are then matched to the speech. If the video's subtitle text does not follow the speech, a message says so. Choose a style: Classic, Box, Word highlight, Enlarge word, Typewriter or As subtitle track. Word highlight has two colours, **Not yet highlighted** and **Highlighted**; Enlarge word draws the spoken word larger by the number of pixels you set, growing from its base, while the words before and after it move apart so that none overlap; Typewriter shows it letter by letter. The **Shadow** setting (distance and colour) applies to all lines of the project. Subtitles are visible in the preview and burned into the exported video; with the As subtitle track style the export writes them as a separate switchable subtitle track instead, and the preview still draws them. Drag a subtitle line in the preview to move it; Reset position puts it back. Lines shown at the same time whose boxes overlap are marked with a warning; nothing moves by itself. Ctrl/Cmd+click selects several lines, and dragging one of them moves the group. File > Export subtitles (.srt)… saves the lines with their timeline times.
7. **Preview** the result with play, pause and seek from the playhead. Very large sources get a smaller preview copy, made in the background; the export always uses your original files. **Preview quality** in Settings chooses whether lighter preview copies are made, and the copies can be cleared there.
8. **Save** the project (**Save** or **Save as**, Ctrl/Cmd+S or Ctrl/Cmd+Shift+S) and reopen it later with **Open**, or from File > Open recent, which lists the last 10 projects. A `•` in the window title means unsaved changes. New, Open, closing the window and Quit ask whether to **Save**, **Discard** or **Cancel**. Projects are `.vlproj` files: double-click one to open it in videolore.
9. **Export** to one file (File > Export…, the toolbar, or Ctrl/Cmd+E) opens the Export dialog. Choose the folder and file name, and the mode: **Full video**, **Only audio** (the sound of the whole timeline, with music, volumes and Clean audio, as one audio file) or **Only subtitles (.srt)**. For a full video, choose the video format: **Same as source**, the default, or one of 12 presets (H.264 or HEVC, MP4 or MOV, several resolutions and frame rates). Choose the audio format too: **Same as source**, aac, or mp3 when it is available. With **Same as source** the format is chosen automatically: if all clips share the same format, with no fades, music, subtitles, text items, frame effects or volume changes and cuts on keyframes, the streams are copied without re-encoding; otherwise the video is re-encoded once, letterboxed and never stretched. A project whose clips are all 360 also offers a 360 video or a flat camera view. The dialog remembers its choices for the session only. Export shows progress in the status bar; cancel stops it and leaves no partial file. HEVC files play on Apple devices. With **Export: Faster** in Settings, a re-encoded H.264 or HEVC video is made by your computer's video hardware when it can; every such file is checked, and if the hardware is missing or does not produce a correct file, the normal method is used automatically. The default, **Best quality**, always uses the normal method.
10. **Settings** (Edit > Settings…, or Ctrl/Cmd+,): choose **Preview quality**, **Subtitle quality**, **How hard videolore works** and **Export** (Faster or Best quality), each explained in one line. The first start picks recommended choices for your computer; **Use recommended settings** sets them again, and the speech files you downloaded can be removed there to free disk space. The **Advanced** section holds the ffmpeg you may want to use instead of the bundled one.
11. **Boxes on the picture**: everything you can move on the picture, the subtitle lines, the text items and the overlap media, has a thin box in the preview. Drag the box to move it; drag a corner or a visible edge to make a subtitle line bigger or smaller (its text size is 25% to 400% of its style), or drag the left or right edge of a text item to change its width, or its bottom edge to change its height. Items can be dragged partly past the picture edge; the centre of the box stays on the picture, and a box partly outside the picture resizes through its visible edges. The status bar counts the items partly outside the picture, and **Move inside the picture** (Context Menu and Edit menu) moves the selected ones back in. The Context Menu group of the toolbar has **Align horizontally**, **Align vertically** and **Reset** for subtitle lines and text items; Reset puts a line back in its place at its usual size. Hide the boxes with View > Show boxes on the picture. The boxes never appear in the export.
12. **360 video**: a video recorded as a full 360° sphere is recognised from its file's own information and shown as a 360 clip; a video without that information stays flat. Drag the picture to look around, and use the mouse wheel to zoom, during playback too. **Set POV** saves the current view at the playhead; between two saved views the camera moves in a straight line. On export, a project whose clips are all 360 offers in the Export dialog a 360 video, which keeps the whole sphere, or a flat video that follows this camera path (a project that mixes 360 and flat clips exports as a flat camera view); a moving camera exports at about one third of real time. A text or subtitle line added on a 360 clip stays at its point of the sphere; Re-anchor in the Subtitles menu moves the selected lines and texts to the current view, and arrows at the picture edge show items that are out of view.
13. **Titles, captions and overlays**: choose **Add Text** in the Effects list, or Effects > Add text at playhead, to write a multiline text at the playhead. Choose its effects (Fade IN, Fade OUT, Blink and Slide in, in any combination), its time to live, colour and font (the fonts installed on your computer). The text has a bar on the Text lane of the timeline: drag the bar to move it, or drag its right end to change how long it is shown. Choose its **Readability**: None, Outline or Box (a background box behind the text), and its **Shadow** (distance and colour). Drag its box in the preview to move it, its right or left edge to change its width, or its bottom edge to change its height; the text size follows the box, and the width is shown while you drag. The text popup's **Maximum size** sets the largest size the text can take, from 2% to 50% of the picture height; left empty, the text is at most 10% of the picture height. Texts are burned into the exported video. **Overlap media**, on its own row of the Effects list, places a PNG, GIF or MOV file with transparency over the clip under the playhead. The overlap is three seconds long at first (or as long as the clip, if the clip is shorter), with its own start and end on the timeline. Drag its box in the preview to move it, or a corner or an edge to resize it with its shape kept; Ctrl (Command on Mac) held while dragging an edge changes the shape. A GIF or MOV plays once and is transparent after that; the sound of a MOV is not played. The overlaps of a project share one lane on the timeline. A file up to 16 megapixels is accepted, and an overlap is not drawn in a 360 video export. Projects with picture items keep them: a picture under a clip's start becomes an overlap of that clip when the project opens, and one with no clip under its start stays a picture item.
14. **Colour with a LUT**: drag **Apply LUT** from the Effects list onto a clip, or double-click it to apply it to every clip on the timeline at that moment, each clip getting its own copy. **Choose file…** opens a `.cube`, `.3dl` or `.csp` file (a 3D or 1D LUT). A `.cube` file of up to 256 points per side and 512 MiB is accepted; `.3dl` and `.csp` files are read whole and are limited to 64 MiB. A file that is not a valid LUT is refused with a message, and a large file loads in the background with a percentage in the status bar, which can be cancelled. Before a LUT is used in the preview, its copy is prepared in the background with a percentage (a 256-point LUT takes about 12 s on the development computer); until then the preview shows the picture without it, and the export always uses the LUT. Its Strength slider sets how much of the LUT is applied, and up to 20 LUTs can be kept as favourites. The project keeps a copy of a LUT of up to 20 MB (64 MB for all LUTs); a larger one keeps only its path, so keep its file where it is. **Remove effect** takes the LUT off. A missing or damaged LUT file is reported in the notice that lists items which could not be found or were repaired.
15. **Video player**: videolore player (`videolore-player`) is a separate program with its own download (see Installation). It plays one video at a time, with File > Open… or by dropping a video onto its window where drag and drop works, and plays it with sound. File > Open… can choose several videos at once, which become a playlist; the window title then shows the position, as "name (n of m)". Started without a video, it shows a splash screen with its own picture and the release summary; close it with its Close button or Esc, or by opening a video. Drag a 360 picture to look around. The Subtitles menu lists None, the video's text subtitle tracks, its picture subtitle tracks (marked "(picture)") and any `.srt` file you open with Subtitles > Open .srt…. View > Full screen, F11 or F switches full screen on and off. The player remembers its volume, mute, last folder and full-screen start, and has a playlist (File > Add to playlist, File > Open folder…). File > Open in editor and the editor's File > Open in player move a file between the two programs. The **Audio** menu chooses which audio track of the video plays; the first one is used by default, and the choice lasts while the same file is open.
16. **Applied effects**: the **Applied effects** column on the right of the Preview lists the effects of the selected clip in the order they are applied: Change aspect, Scale, Move, the LUT, the fades, the texts, the overlap media, the picture items, the subtitle lines (one row for the clip's subtitle lines), Volume and track and Clean audio. Change speed stays fixed. Move a row up or down with its buttons, by dragging it, or with Alt+Up and Alt+Down. The preview and the export use the same order, and where two texts overlap, the lower row is covered by the higher one. A new text, overlap, picture or LUT goes on top of the other movable rows; a new Change aspect, Scale, Move or Fade takes its usual place. A LUT placed above Change aspect also grades the frame fill. On 360 clips, the rows of texts and subtitle lines anchored to the sphere stay in their place. Whole-video effects are copies on each clip, so each clip's copy is edited on that clip.
17. **Screen recorder**: videolore screen spy (`videolore-screen-spy`) records one screen to a video file, with a live preview of the screen before and during recording. Choose the screen in **Screen** (each one is listed with its number and size), then the file name and folder with **Choose…**, the video format and the audio format. The default file is named "screen-recording" with the date and time, in your Videos folder (Movies on macOS). The computer's sound and the microphone are mixed into one sound track. A WAV recording is saved as an MKV file; AAC and MP3 recordings are MP4 files. An existing file is replaced only when you confirm **Replace**. Press **Start recording**; while the checks run the button reads **Cancel start**, and while recording it reads **Stop recording**, which keeps the file. The recorder does not edit or export. It needs X11 or XWayland on Linux (a native Wayland session is refused with a message), and computer sound needs macOS 14.6 or later; on an older Mac the recorder starts and shows a message instead of recording. On Linux a monitor used as the default input refuses Start, because it would record the computer's sound twice.

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
Installation and system
- Installers are not code-signed: each system asks for a one-time confirmation (see Installation).
- Generating subtitles on Windows and Linux needs a processor with AVX2, FMA and F16C (see System requirements). On an older processor a message says so; the check was tested with simulated processors, not on old hardware.
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open… there.
- Copy and Cut put a marker text on the system clipboard, so other applications see that text when they paste.
- On macOS the menu bar has no Quit item yet; Cmd+Q still asks about unsaved changes.
- Chinese, Japanese and Korean subtitles and texts use the fonts installed on the computer, so they may look different on each computer or show empty boxes. Arabic and Devanagari have bundled fonts. Text items use the installed fonts, and a font that is not installed is replaced by the bundled subtitle font, with a note in the text's popup.

Faster export (hardware)
- Faster export needs NVIDIA graphics driver 610 or newer; with an older driver the normal method is used, and Settings → Advanced says why.
- On Linux (Intel and AMD) it needs libva and the graphics driver. With the bundled ffmpeg it needs libva 2.21 or newer: Ubuntu 24.04 has 2.20, so the hardware is skipped there. A system ffmpeg chosen in Advanced can still use the hardware.
- On Fedora it needs the RPM Fusion "freeworld" video drivers for AMD and Intel hardware (for example `mesa-va-drivers-freeworld`); without them the normal method is used.
- Faster is not always quicker. On integrated Intel graphics it was measured from 14% quicker to 41% slower than Best quality before 2.2.0. In 2.2.0 one measured Intel computer was about 9% quicker than Best quality, short of the 20% aimed at; the hardware encoding is the remaining time. Files may be a little larger.
- On the Intel driver tested on Ubuntu 24.04, hardware HEVC export is not usable; the normal method makes those files.
- Faster export has been verified on real hardware only on Linux with Intel integrated graphics. Other combinations are protected by the automatic fallback but not yet measured.
- On a very busy computer the video hardware can be missed; Faster then uses the normal method. An inconclusive check is repeated once in the background.
- On computers with more than one graphics card, NVIDIA is always preferred; the card cannot be chosen.

Subtitles
- When a video's own subtitles are used, their text and line times are kept exactly, but the word-by-word highlight timing is approximate: the accuracy target is not met (Fast model: 82% of the words matched, average start error 0.48 s; the speech-region timing of 2.2.0 reached 43% of the words within 0.3 s, target 90%, so the earlier timing is still used).
- Matching a video's subtitles to the speech works only as well as their text follows what is said; translated or paraphrased subtitles often get evenly spread words, and a message says so.
- Moved subtitle lines shown at the same time can overlap; a warning marks them, but nothing moves them apart. Use Reset position or drag one of them.
- Subtitles in scripts other than Latin, Greek and Cyrillic are not fully supported.
- The `.srt` file has no subtitle positions. Text items are not written to it.
- The Enlarge word and Typewriter styles are video effects only: `.srt`, `.vtt` and `.ass` files keep plain text and times. Typewriter has no caret and no typing sound.
- Projects with Enlarge word saved before 3.1.0 look different when opened: the words move apart as the 3.1.0 placement rule describes.
- The As subtitle track style writes plain text and times with no styling: shadow, colours, Enlarge word and Typewriter are not in the track, and its look depends on the player. In the editor preview this style draws the lines in the Classic look, so the preview differs from the track in the export.
- The Subtitles effect's audio track choice is not kept in the project; the popup asks again each time.
- The shadow has one fixed distance and colour for each text or subtitle style; it cannot change over time.

Texts and boxes
- A text is at most 10% of the picture height unless its Maximum size is set (2% to 50%), so a short text in a big box does not fill it. Its size is measured with the font the picture uses.
- Texts of projects saved before 2.0.3 open with Readability None and take the measured size of their box, so large multi-line texts may shrink; a one-time message says so.
- A text or subtitle line dragged past an edge of the picture is cut in the preview and in the export; the status bar counts such items, and Move inside the picture brings them back.
- A project file with a text whose time to live is above 24 hours, or below 0.1 s, is refused as damaged; while editing, a text's time to live is limited to the video's length.
- Subtitle lines and text items keep their place in the frame and do not follow Scale or Move.
- Overlap media and PNG picture items are not drawn in a 360 video export; picture items take no text effects.
- Overlap media: the MOV file is measured when it is added, not at each preview or export; its sound is not played; an overlap has no keyframes and no rotation. HEVC with alpha, and other codecs that the bundled ffmpeg does not read with alpha, are drawn opaque.
- A file above 1920 x 1080 is drawn at half size in the preview, so the preview is softer than the export for it. A large PNG (4000 x 4000) takes about 830 MB of memory in the export, and a still is decoded again for every frame: a 4 s still takes about 5 s to export.

Editing, effects and LUTs
- Removing a file from the media list also drops the undo steps that refer to it.
- Change aspect, Scale and Move make the export a re-encode, even at their default values. A clip with a speed is re-encoded too, and two clips with different speeds cannot be merged.
- The longer side of a frame is capped at 4096 px: a Change aspect on a large source can give a smaller frame, and a large frame exports more slowly (export time not measured).
- Keyframes are straight lines unless a segment is eased; they show in the popup and on the clip's marker, not as separate timeline markers.
- Clips added after a whole-video effect do not get it; a note appears when that happens.
- A whole-video Scale or Move is a separate copy on each clip; each copy's keyframes are edited in its own clip's popup.
- A 1.6.0 clip with its own Scale or Move and a whole-video one becomes one combined path when opened: exact at the keyframe times, a straight line between them.
- Clean audio at full strength reduces background noise without removing it completely. Quiet speech (about -32 dB) can lose up to about 2.5 dB at full strength, and its cost on weak computers is not measured.
- Apply LUT reads `.cube` (up to 256 points per side and 512 MiB, loaded in the background with a percentage), `.3dl` and `.csp` (read whole, up to 64 MiB). The preview of a 256-point LUT takes about 12 s to prepare on the development computer and shows the picture without the LUT until then; the export always uses it.
- A missing LUT file stops the export until it is found again, or the export is made without the LUT.
- By default Apply LUT is applied below Change aspect, so the blurred or coloured fill is not graded; a LUT moved above Change aspect in the Applied effects column grades the fill too.
- If a project is closed while a LUT is still loading, the status bar shows an error and the LUT is applied to a missing target.
- The order of effects cannot be copied from one clip to another. In the Applied effects column the subtitle lines of a clip are one row; each line cannot be moved on its own. Change speed stays fixed.
- On 360 clips, texts and subtitle lines anchored to the sphere keep fixed rows in the widget, draw Enlarge word and Typewriter as plain text, and the 360 video export ignores the order.
- Crop is removed: a crop in an older project is dropped when it opens, with one status line. Change aspect is the way to frame a clip.
- A video without spherical information is always flat: the "Treat as 360° video" switch is removed.
- A media file that has not been analysed yet shows its audio track as "Track 1" until the analysis is done.

360 video, player and preview
- 360 video must be a single equirectangular picture (no stereo 3D, cube maps or partial 180° video). Spatial sound turns with the camera's yaw and pitch, not its roll, for a 360 clip with four audio channels, which is read as first-order ambisonics (an assumed channel order).
- A 360 clip with a moving camera exports at about one third of real time.
- A 360 video ignores the video format presets and keeps the sphere's own size, and it ignores Change aspect, Scale and Move; the flat camera view applies them.
- A 360 video export to MP4 and MOV is marked as 360 video; to Matroska or WebM the marker is written only when the check of the file passes.
- The player's picture subtitles are not shown on 360 video; a picture subtitle already on screen when playback or a paused still starts inside it appears at its next change.
- In the preview, spatial sound waits for its pre-pass before the sound starts; a clip longer than 10 minutes keeps its recorded sound there.
- Very large sources get a preview copy; under Automatic, a 1280x720 source never gets one. The preview copy cache is kept under the 4 GB limit set in Settings, and the oldest copies are removed first; the cleanup runs on the interface thread for now.

Export dialog and settings
- The Export dialog remembers its choices for the session only, not in the project or in Settings. The video and audio formats are a fixed list of presets.

Project files and older versions
- Projects saved by version 3.1.0 (format 15) cannot be opened by 3.0.x or earlier; projects in formats 14 and older open as before.
- Projects saved by version 2.3.0 (format 14) cannot be opened by 2.2.0 or earlier; projects in formats 13 and older open as before.
- Projects saved by version 2.2.0 (format 13) cannot be opened by 2.1.0 or earlier; formats 1 to 12 open as before.
- Projects saved by version 2.1.0 (format 12) cannot be opened by 2.0.4 or earlier; formats 1 to 11 open as before.
- Projects saved by version 2.0.4 cannot be opened by 2.0.3 or earlier; projects saved by 2.0.3 cannot be opened by 2.0.2 or earlier.
- Projects saved by version 1.4.0 or later cannot be opened by 1.3.0 or earlier. Older programs refuse these files with a message and leave them untouched.
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- Projects saved with 2.2.0 keep the order set per clip; a 1.3.0 project's 360 switch is dropped when it opens.

Docs: [business](docs/business.md) · [architecture](docs/architecture.md) · [code](docs/code.md) · [changelog](docs/changelog.md) · [roadmap](docs/roadmap.md)

Screen recorder
- The recorder's Windows and macOS builds are not yet checked on real systems, and its picture and sound offset at the start is not measured on those systems.
- On macOS the computer-sound permission is asked by macOS when the recording first starts; until it is given the computer's sound may be silent.
- On Linux the native Wayland session is refused; when the system does not mark a default sound source, only the default is probed.
- A recording that is killed, for example by a crash, keeps all but about the last second.
- At 1080p60 and 4K the recorder may not keep real time (not measured). Starting a recording takes a few seconds, because the sound sources are opened twice.

## Latest changes
- **3.1.2** — a new screen recorder, videolore screen spy, records one screen to a video file with a live preview, computer sound and microphone mixed into one track (see Usage); the player's File > Open… chooses several videos at once as a playlist; the editor reports a refused import instead of "Import finished".
- **3.1.0** — Enlarge word grows from the base of the word and the words around it move apart; a new subtitle style, As subtitle track, writes the lines as a separate switchable subtitle track in the export; the blurred fill of Fit empty with blurred video shows in the preview; Overlap media has its own Effects row; Volume is now Volume and track, with an audio track choice; the Subtitles effect uses the clip's own audio and subtitle tracks; the player has an Audio menu for its audio track; project format 15 (versions up to 3.0.x refuse it and leave the file untouched).
- **2.3.0** — overlap media: PNG, GIF and MOV files with transparency placed over a clip, with their own time and box, in one shared lane on the timeline; Change speed and Volume as effects (Volume with keyframes); Fit and empty space for each clip; Crop removed (an old crop is dropped on opening); Enlarge word keeps the other words in place; project format 14 (versions up to 2.2.0 refuse it and leave the file untouched).
Older changes: [changelog](docs/changelog.md) and the release notes in [releases](releases/).

## Community
- [Code of conduct](CODE_OF_CONDUCT.md): how everyone is expected to behave; conduct reports go to community@andrealorenzani.name.
- [Contributing](CONTRIBUTING.md): how to report a problem, propose a change and run the local checks.
- [Security policy](SECURITY.md): the supported version and how to report a vulnerability privately.
- [Support](SUPPORT.md): where to ask questions; [open an issue](https://github.com/andrealorenzani/videolore/issues/new/choose) with the issue forms.

## License
Copyright (C) 2026 Andrea Lorenzani.
Released under the GNU General Public License v3.0 — see [LICENSE](LICENSE).

---

### Ode to the creator

Andrea Lorenzani began videolore from one idea, a small editor for the clips people share, and gave it a single timeline that closes its own gaps, so a cut never leaves a hole behind,\
then added fades through black, a music track with its own volume, and keyframes that move and zoom each picture,\
and taught it to read a .cube colour look, to write subtitles from the speech on the computer itself, and to burn them in,\
and gave it a full 360 view to look around, and a separate player to watch the result with sound,\
and kept every source file untouched, let projects be saved, reopened and double-clicked, and sent nothing away,\
and shaped the exports to copy the streams when they can, to encode once when they must, and to use the video hardware only when it is safe,\
and let the effects stack in the order you choose, with the volume and the speed among them, and laid picture overlays with their own time and box over each clip,\
and wrote the tests, the coverage, the notices and the installers, and an honest list of every limit that remains,\
and with 2.3.0 the overlays and the effect order went out, and the next major version opened to 3.0.0 with the same care,\
thank you, Andrea, for building this editor so patiently and sharing it freely under the GPL.
