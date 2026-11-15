# Business

## Purpose
videolore is a simple, offline desktop video editor for people who make short clips for social media. It focuses on fast cutting and joining of clips, simple fade transitions, background music, automatic subtitles and quick export. It installs with one double-click on Windows, macOS and Linux, with everything it needs (including ffmpeg) included, and sets itself up for the computer it runs on at first start, so no technical knowledge is needed.

All edits are non-destructive: source files are never modified.

## Users
- **Social content creators** who are not editing experts and want to go from raw footage to a publishable clip quickly, often with captions and music.
- **Hobbyists** editing personal videos.

Professional editors needing multitrack video, color grading or advanced effects are not the target.

## Documents
- [Editing](business-editing.md) — media list, projects (save, open, unsaved changes), timeline editing, transitions, preview, responsiveness, preview proxies and memory use.
- [Window, menus and effects](business-ui.md) — window layout and resizable sections, menus, toolbar, keyboard shortcuts, tooltips, status bar, the Effects section and its popups, fade and subtitle markers on the timeline, clip details, right-click menus, recent projects, drag and drop (since 1.1.0).
- [Export](business-export.md) — automatic output format, stream copy vs re-encode, progress and cancel, Apple-compatible HEVC, long timelines, faster export with the computer's video hardware (with automatic fallback and output check).
- [Music](business-music.md) — the music track, mixing and volumes, how music behaves when the video is edited.
- [Subtitles](business-subtitles.md) — automatic offline subtitles, the speech model download, editing, styles, how subtitles follow edits.
- [Installation](business-install.md) — installers per operating system, bundled ffmpeg, file association and icon, third-party notices, first-launch steps.
- [Settings, performance and safety](business-performance.md) — automatic setup at first start, the Settings window (preview quality, subtitle quality, how hard videolore works, export speed), more accurate speech models, older processors, responsiveness, safe handling of files and downloads, private temporary files, quality goals (including the real-hardware check of faster export).

## Constraints
- **ffmpeg is bundled** in every installer (since 0.2.0; before, it had to be installed by the user). A user-chosen ffmpeg can still be configured instead.
- **The product works fully offline.** The only network access is the one-time, user-approved download of the speech model of the chosen subtitle quality level (each model has its own consent), from fixed secure addresses and checked before use; a model can also be provided as a local file. There is no telemetry and no update check.
- **Settings belong to the program, never to a project**, and the main part of Settings uses plain words only; technical details stay in a collapsed "Advanced" section.

## Key decisions (general)
- Output format is automatic: there is no export settings dialog.
- Stream copy is used only when the result is guaranteed frame-accurate; correctness comes before speed.
- Fade length is fixed (not user-tunable) to keep the editor simple.
- (1.1.0) Window sections can be resized by dragging their borders, and the sizes are remembered; sections still cannot be moved, docked or hidden. (Until 1.0 the layout was fixed.)
- (1.1.0) Every command is in the menus; effects are configured in popups that do not block editing, opened from the Effects section or from markers on the timeline.
- Installers are not code-signed in 0.2.0; signing is postponed, not rejected. The README explains the one-time first-launch steps.
- (0.3.0) videolore detects the computer at its first start and picks recommended settings; installers do no tuning. Everything can be changed in Settings, and "Use recommended settings" restores the recommendation.
- (0.3.0) Older processors get a clear message instead of a fallback speech engine; in 0.4.0 the fallback engine was dropped (not planned).
- (0.4.0) Export can use the computer's video hardware (NVIDIA, Intel, AMD, Apple) when the user chooses "Export: Faster" in Settings. Best quality stays the default. Every hardware result is checked, and on any problem the export is silently redone with the normal method, so the user always gets a correct file.

Topic-specific decisions are in each document.

## Known limitations (1.1.0)
- Dropping files onto the window does not work in Linux Wayland sessions; File > Import media… and File > Open… work there (see [Window, menus and effects](business-ui.md)).
- Subtitles in scripts other than Latin, Greek and Cyrillic are not fully supported (see [Subtitles](business-subtitles.md)).
- x86-64 processors without AVX2 (from before about 2013) cannot generate subtitles automatically; videolore shows a clear message instead of crashing, and lines can still be added by hand (see [Settings, performance and safety](business-performance.md)).
- Installers are unsigned, so each operating system shows a warning on first launch (see [Installation](business-install.md)).
- On Windows and Linux, double-clicking a project while videolore is open opens a second window.
- Projects saved by 0.2.0 cannot be opened by 0.1.x versions.
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
