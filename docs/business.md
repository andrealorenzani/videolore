# Business

## Purpose
videolore is a simple, offline desktop video editor for people who make short clips for social media. It focuses on fast cutting and joining of clips, simple fade transitions, background music, automatic subtitles and quick export. It installs with one double-click on Windows, macOS and Linux, with everything it needs (including ffmpeg) included, and sets itself up for the computer it runs on at first start, so no technical knowledge is needed.

All edits are non-destructive: source files are never modified.

## Users
- **Social content creators** who are not editing experts and want to go from raw footage to a publishable clip quickly, often with captions and music.
- **Hobbyists** editing personal videos.

Professional editors needing multitrack video, color grading or advanced effects are not the target.

## Documents
- [Editing](business-editing.md) — media list, projects (save, open, unsaved changes, compatibility with older project files), timeline editing, fades between clips and at the start and end of the video, preview, responsiveness, preview proxies and memory use.
- [Window, menus and effects](business-ui.md) — window layout and resizable sections, menus, toolbar, keyboard shortcuts, tooltips, status bar, the Effects section and its popups, applying effects to a clip or to the whole video, markers on the timeline, the timeline ruler, clip details, right-click menus, recent projects, drag and drop.
- [Export](business-export.md) — automatic output format, stream copy vs re-encode, progress and cancel, Apple-compatible HEVC, long timelines, faster export with the computer's video hardware (with automatic fallback and output check).
- [Music and volume](business-music.md) — the music track, mixing and volumes, the volume of each video clip, how music behaves when the video is edited.
- [Subtitles](business-subtitles.md) — automatic offline subtitles, the speech model download, editing, styles, how subtitles follow edits, subtitles for one clip, reusing the subtitles already in a video, `.srt` export, moving lines on screen and selecting several lines.
- [Text](business-text.md) — 1.3.0: "Add Text" in the Effects list, multiline texts with the effects Fade IN, Fade OUT, Blink and Slide in, time to live on the timeline, colour, font installed on the computer, text on the picture and in the export.
- [Boxes on the picture and 360 video](business-picture.md) — 1.2.6: visible boxes to move and enlarge what sits on the picture, effect commands (align horizontally, align vertically, reset) in the toolbar, 360° videos with look-around and points of view (POV) the camera moves between in a straight line. 1.3.0: the toolbar group becomes the Context Menu, Export of a 360 project asks for a 360 video or the flat camera view, text and subtitles on a 360 clip stay at their place in the sphere.
- [Installation](business-install.md) — installers per operating system, bundled ffmpeg, file association and icon, third-party notices, first-launch steps.
- [Settings, performance and safety](business-performance.md) — automatic setup at first start, the Settings window (preview quality, subtitle quality, how hard videolore works, export speed), more accurate speech models, older processors, responsiveness, safe handling of files and downloads, private temporary files, quality goals (including the real-hardware check of faster export).

## Constraints
- **ffmpeg is bundled** in every installer (since 0.2.0; before, it had to be installed by the user). A user-chosen ffmpeg can still be configured instead.
- **The product works fully offline.** The only network access is the one-time, user-approved download of the speech model of the chosen subtitle quality level (each model has its own consent), from fixed secure addresses and checked before use; a model can also be provided as a local file. There is no telemetry and no update check.
- **Settings belong to the program, never to a project**, and the main part of Settings uses plain words only; technical details stay in a collapsed "Advanced" section.

## Key decisions (general)
- Output format is automatic: there is no export settings dialog (since 1.3.0 one question is asked for projects with 360 clips, see below).
- Stream copy is used only when the result is guaranteed frame-accurate; correctness comes before speed.
- Fade length is fixed (not user-tunable) to keep the editor simple.
- (1.1.0) Window sections can be resized by dragging their borders, and the sizes are remembered; sections still cannot be moved, docked or hidden. (Until 1.0 the layout was fixed.)
- (1.1.0) Every command is in the menus; effects are configured in popups that do not block editing, opened from the Effects section or from markers on the timeline.
- Installers are not code-signed in 0.2.0; signing is postponed, not rejected. The README explains the one-time first-launch steps.
- (0.3.0) videolore detects the computer at its first start and picks recommended settings; installers do no tuning. Everything can be changed in Settings, and "Use recommended settings" restores the recommendation.
- (0.3.0) Older processors get a clear message instead of a fallback speech engine; in 0.4.0 the fallback engine was dropped (not planned).
- (1.2.0) Every effect can be applied to one video clip (drag and drop or right-click) or to the whole video (double-click in the Effects list).
- (1.2.0) When a video already contains subtitles, the user chooses between using their text (matched word by word to the speech) and recognising the speech; the video's text is never replaced.
- (1.2.0) The project file format changes to store start and end fades, clip volumes and subtitle positions; older projects open with the same result.
- (1.2.6) A 360° video is shown through a camera the user can turn; the export was a normal flat video that follows the camera path set with points of view. This is the only keyframe-like feature, limited to the camera of 360 clips.
- (1.3.0) For a project with 360 clips, Export asks whether to make a 360 video (the whole sphere) or the flat camera view of 1.2.6. This is an explicit exception to the automatic output format, only for such projects.
- (1.3.0) Text items are a new kind of object on the picture, on their own timeline lane: they are not a video track, so multiple video tracks stay out of scope. The text effects Fade IN and Fade OUT have a set time; the video effect "Fade through black" is named "Fade IN/OUT" and keeps its fixed length.
- (1.3.0) Text and subtitle lines added on a 360 clip are anchored to a point of the sphere and do not move when the camera turns.
- (0.4.0) Export can use the computer's video hardware (NVIDIA, Intel, AMD, Apple) when the user chooses "Export: Faster" in Settings. Best quality stays the default. Every hardware result is checked, and on any problem the export is silently redone with the normal method, so the user always gets a correct file.

Topic-specific decisions are in each document.

## Known limitations (1.2.0, 1.3.0)
- (1.3.0) Text uses the fonts installed on the computer, so the same text can look different on each computer. A project that mixes 360 and flat clips can only be exported as the flat camera view.
- (1.3.0) Text items have no outline or background box; a text that does not fit its box runs past it; video edits do not move text items.
- (1.3.0) The 360 export is slow (no speed is promised); subtitle lines generated automatically on a 360 clip follow the camera instead of a point of the sphere.
- (1.3.0) A subtitle line placed on a 360 clip is drawn as plain white text (no preset box, no word highlight).
- (1.3.0) A 360 video export carries its 360 marker only in MP4 and MOV files.
- (1.3.0) A project file with a text whose time to live is above 24 hours is refused as damaged when opened; while editing, a text's time to live is limited to the video's length.
- Dropping files onto the window does not work in Linux Wayland sessions; File > Import media… and File > Open… work there (see [Window, menus and effects](business-ui.md)).
- Subtitles in scripts other than Latin, Greek and Cyrillic are not fully supported (see [Subtitles](business-subtitles.md)).
- x86-64 processors without AVX2 (from before about 2013) cannot generate subtitles automatically; videolore shows a clear message instead of crashing, and lines can still be added by hand (see [Settings, performance and safety](business-performance.md)).
- Installers are unsigned, so each operating system shows a warning on first launch (see [Installation](business-install.md)).
- On Windows and Linux, double-clicking a project while videolore is open opens a second window.
- Projects saved by 0.2.0 cannot be opened by 0.1.x versions, and projects saved by 1.2.0 cannot be opened by 1.1 or earlier.
- When a video's own subtitles are reused, their text and line times are kept exactly, but the word-by-word highlight timing is approximate: the REQ-095 accuracy target (90% of words, within 0.3 s) is not met yet with the Fast level (measured 82%, 0.48 s average word-start error; see [Subtitles](business-subtitles.md)).
- Matching a video's own subtitles to the speech works only as well as their text follows what is said; translated or paraphrased subtitles often get evenly spread words (see [Subtitles](business-subtitles.md)).
- (1.2.6) Projects saved by 1.2.6 cannot be opened by 1.2.5 or earlier; exporting a moving 360 camera runs at about one third of real time; only equirectangular single-picture 360 video is supported and it is exported flat (see [Boxes on the picture and 360 video](business-picture.md)).
- Subtitle lines moved to their own place can overlap when shown at the same time; editing a line's text after moving it can push it past the edge of the picture in the export (see [Subtitles](business-subtitles.md)).
- On macOS the standard Hide and Minimize menu items are not available, and the menu bar is inside the window rather than at the top of the screen.
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text.
- Faster export needs NVIDIA graphics driver 610 or newer; on Fedora, AMD and Intel hardware export needs the RPM Fusion "freeworld" video drivers; on Linux, Intel and AMD hardware with the bundled ffmpeg needs system libva 2.21 or newer (Ubuntu 24.04 has 2.20, so it is skipped there); on integrated Intel graphics Faster was measured to be sometimes slower than Best quality; on computers with several graphics cards NVIDIA is always preferred; some combinations are not yet verified on real hardware. In every case a problem with the hardware leads to the normal method automatically, so the exported file is still correct (see [Export](business-export.md)).

## Out of scope
- Multiple video tracks, overlays, picture-in-picture.
- Color grading, keyframe animation, transitions other than fade through black.
- Accounts, cloud storage, sync, collaboration, direct upload to social networks.
- Mobile versions.
- Export settings dialog.
- Autosave, crash recovery, automatic updates.
- Code signing and notarisation (postponed).
- Telemetry, online services for detecting the computer, performance settings per project.

Out-of-scope items specific to a topic are listed in its document.
