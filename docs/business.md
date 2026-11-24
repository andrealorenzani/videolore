# Business

## Purpose
videolore is a simple, offline desktop video editor for people who make short clips for social media. It focuses on fast cutting and joining of clips, fades, background music, automatic subtitles, effects and quick export. It installs with one double-click on Windows, macOS and Linux, with everything it needs (including ffmpeg), and sets itself up for the computer at first start.

All edits are non-destructive: source files are never modified. The separate **videolore player** (since 1.6.0) only watches videos; it never edits.

## Users
- **Social content creators** who are not editing experts and want a publishable clip quickly, often with captions and music.
- **Hobbyists** editing personal videos.

Not the target: professional editors who need multitrack video, colour grading beyond LUTs or advanced effects.

## Constraints
- **ffmpeg is bundled** in every installer; a user-chosen ffmpeg can be configured instead.
- **Works fully offline.** The only network access is the one-time, user-approved download of the speech model of the chosen subtitle quality level, from fixed secure addresses (or the user provides the file).
- **Settings belong to the program, never to a project**; the main part of Settings uses plain words, technical details stay in "Advanced".
- **Installers are not code-signed** (decided by Andrea in 2.0.2; signing and notarisation are not planned). The README explains the first-launch steps.
- **Every choice of Andrea** is listed in [Choices made by Andrea](andrea-choices.md).

## Key decisions
- Export has a dialog (file, mode, video and audio format, 360 choice); the output format is automatic ("Same as source") by default. Presets stay fixed.
- Stream copy only when the result is guaranteed frame-accurate; correctness before speed.
- Fades: 0.1 s up to half the shorter clip (REQ-274). The layout is resizable, hideable and dockable; every command is in the menus.
- Each effect applies to one clip (drag or right-click) or to the whole video (double-click). A whole-video effect is a copy on each clip (REQ-147, REQ-148). Whole-video Fade IN/OUT keeps first start and last end (REQ-083).
- Hardware export always goes through ffmpeg (bundled, or the one the user chose); no vendor pipeline. Best quality stays the default; every hardware result is checked and redone with the normal method on any problem.
- The 360 export prioritises speed (fastest preset within 1.5 times the Best quality size).
- Subtitles: when a video has its own subtitles, the user chooses between their text (matched to the speech) and recognition; the video's text is never replaced.
- Text items are objects on the picture with their own lane, not video tracks; the text size follows its box, capped at 10% of the picture height, with an optional per-text maximum (D64).
- Change aspect applies to the whole video, or to a clip with Fit and empty space (REQ-326). Scale, Move and Volume are keyframed; the change between keyframes is straight unless eased.
- A 360 clip is recognised only from its file's information; a 360 video is shown through a camera the user turns; texts and subtitles on a 360 clip are anchored to the sphere.
- Effects in the Applied effects column are reorderable inside their group (picture or sound); Change speed stays fixed and first.
- An overlap (Overlap media) is bound to one video clip; its GIF or MOV plays once and is transparent after the animation (REQ-328 to REQ-330).
- Crop is removed (REQ-283); Speed is the Change speed effect, not keyframed (REQ-281); Volume is its own keyframable effect (REQ-280).
- Project files are versioned; a project saved by a newer format is refused by an older program with a message and left untouched.
- Older processors get a clear message for subtitles instead of a fallback speech engine.

## Documents
Each document is a short business description with its requirements (REQ-NNN) and acceptance criteria. Status of each requirement: `scripts/reqs-status.sh`; text: `scripts/req.sh REQ-NNN`.

### Editing and projects
- [Editing](business-editing.md) — media list, timeline editing, fades, Add media button, preview, projects, memory use.
- [Editing gestures, autosave and safety](business-editing-gestures.md) — autosave and crash recovery, timeline gestures, undo on file removal, guides and safe areas.
- [Window, menus and effects](business-ui.md) — window layout, menus, toolbar, shortcuts, status bar, clip details, right-click menus, recent projects, drag and drop.
- [Interface](business-interface.md) — remembered window and popups, hidden or docked sections, workspaces, toolbar, shortcuts, themes, text size, accessibility, English only.
- [Platforms and installers](business-platforms.md) — CI checks, installer and toolchain pins, double-clicked projects, the macOS menu bar, Linux archive launcher.

### Effects and frame
- [Effects](business-effects.md) — the Effects list and popups, applying to a clip or the whole video, markers on the timeline.
- [Effects and frame options](business-effects-frame.md) — fades, music effects, Clean audio, Enhance speech, easing, Volume (planned: Volume and track), blur strength, LUT strength, Change speed, Fit and empty space, PNG picture items.
- [Frame](business-frame.md) — Change aspect (4:3, 16:9, 9:16; Fit on width or height; empty space as colour or blurred video; planned fix of the blurred fill), Scale and Move with keyframes.
- [Whole-video and keyframe clarity](business-keyframes.md) — whole-video keyframe copies shown apart, notice for clips added later, copying keyframes between clips.
- [Applied effects](business-applied-effects.md) — the column on the right of the preview: order of the effects of the selected clip, reorder inside each group, Change speed fixed.
- [Overlap media](business-overlap.md) — PNG, GIF and MOV with transparency over one video clip, with its own start, end and box; one shared lane on the timeline.
- [Icons](business-icons.md) — icons from a free, GPL-compatible set across the editor and the player.

### Text and subtitles
- [Text](business-text.md) — multiline texts with Fade IN, Fade OUT, Blink and Slide in, time to live, colour, installed fonts, readability, size fitted to the box, Align and Reset.
- [Subtitles](business-subtitles.md) — automatic offline subtitles, speech model download, editing, reuse of a video's own subtitles, moving lines on screen, multi-selection; planned: the Subtitles effect takes its audio track and subtitle track from the clip.
- [Subtitle accuracy](business-subtitle-accuracy.md) — long recognition has no lost words; the accuracy target of reused subtitles (REQ-177) is not met (see the [roadmap](roadmap.md)).
- [Subtitle styles](business-subtitle-styles.md) — Classic, Box, Word highlight (two colours), Enlarge word, Typewriter; shadows for texts and subtitles; planned: As subtitle track, Enlarge word keeping its base.
- [Subtitle files and fonts](business-subtitle-files.md) — `.srt`, `.vtt` and ASS export, a switchable subtitle track, importing `.srt` and `.vtt`, fonts for Arabic and Devanagari.

### Picture, 360 and LUT
- [Boxes on the picture and 360 video](business-picture.md) — visible boxes to move and resize what sits on the picture, Context Menu commands, 360 looking and camera points of view.
- [360 video](business-360-video.md) — 360 marker in MKV and WebM, camera roll, keyboard control, speed-first 360 export, texts and subtitles anchored to the sphere, camera paths, spatial sound.
- [Apply LUT](business-lut.md) — `.cube`, `.3dl` and `.csp` LUTs on a clip or the whole video; favourites; background preparation with progress.
- [Speed](business-speed.md) — preview of a 256-point LUT with progress, responsive interface, Faster export quicker on integrated graphics, 16 megapixel picture limit.

### Export, audio and performance
- [Export](business-export.md) — the Export dialog, stream copy or re-encode, progress and cancel, Apple-compatible HEVC, outputs.
- [Faster export](business-hardware-export.md) — hardware export (NVIDIA, Intel, AMD, Apple) with a check and automatic fallback, AV1 and VP9 presets, hardware decoding for the preview.
- [Music and volume](business-music.md) — the music track, volumes, Clean audio, music behaviour when the video is edited.
- [Settings, performance and safety](business-performance.md) — automatic setup at first start, Settings, older processors, responsiveness, safe handling of files and downloads.
- [Continuous quality and cache](business-quality.md) — preview cache limit, project page, engineering quality goals.

### Player and installation
- [Video player](business-player.md) — the separate player: one video, 360 view, subtitles, planned audio track menu, full screen, start screen, playlist, "Open with" entries.
- [Splash screen](business-splash.md) — the start screen of the editor and the player: fixed pictures, summary of the version, "Show the start screen".
- [Installation](business-install.md) — installers per operating system, bundled ffmpeg, file association and icon, notices, first-launch steps.
- [Release verification](business-verification.md) — checks of a release on real systems; a manual checklist, not a blocker.

## Version 3.1.0
Seven requirements, implemented (REQ-334 to REQ-340), each with its acceptance in the document named: Enlarge word keeps its base; As subtitle track style; blurred fill fix; Overlap media as its own Effects list row; player audio track menu; Subtitles effect with the clip's audio and subtitle tracks; Volume and track. REQ-333 is replaced by REQ-334.
Accepted deviations (Andrea, 2026-11-24): the Subtitles effect's audio track choice is popup state and is not stored in the project; the blurred fill defect (REQ-335) was in the preview, not the export; the "As subtitle track" preview uses the Classic look; a file not yet analysed shows "Track 1".

## Known limitations
The list for users is in the [README](../README.md#known-limitations). The open items are in the [roadmap](roadmap.md); the cleaned hand-off of each release is in the release notes of the [releases](../releases/) folder.

## Out of scope
- Multiple video tracks; picture-in-picture only as an overlap bound to one clip; mixing several audio tracks of one clip into one sound (one track is chosen per clip, REQ-340).
- Colour grading other than applying a LUT file; keyframe animation other than Scale, Move and Volume; transitions other than fade through black.
- Editing, saving or exporting in the video player.
- Accounts, cloud storage, sync, collaboration, direct upload to social networks, mobile versions.
- Custom resolutions, frame rates, bitrates and encoder options; a separate vendor pipeline for hardware export; code signing (see [roadmap](roadmap.md#not-planned)).
