# Business

## Purpose
videolore is a simple, offline desktop video editor for people who make short clips for social media. It focuses on fast cutting and joining of clips, simple fade transitions, background music, automatic subtitles and quick export. It installs with one double-click on Windows, macOS and Linux, with everything it needs (including ffmpeg) included, and sets itself up for the computer it runs on at first start, so no technical knowledge is needed.

All edits are non-destructive: source files are never modified.

## Users
- **Social content creators** who are not editing experts and want to go from raw footage to a publishable clip quickly, often with captions and music.
- **Hobbyists** editing personal videos.

Professional editors needing multitrack video, color grading or advanced effects are not the target.

## Documents
- [Editing](business-editing.md) — media list, projects (save, open, unsaved changes), timeline editing, transitions, preview, layout, responsiveness, preview proxies and memory use.
- [Export](business-export.md) — automatic output format, stream copy vs re-encode, progress and cancel, Apple-compatible HEVC, long timelines.
- [Music](business-music.md) — the music track, mixing and volumes, how music behaves when the video is edited.
- [Subtitles](business-subtitles.md) — automatic offline subtitles, the speech model download, editing, styles, how subtitles follow edits.
- [Installation](business-install.md) — installers per operating system, bundled ffmpeg, file association and icon, third-party notices, first-launch steps.
- [Settings, performance and safety](business-performance.md) — automatic setup at first start, the Settings window (preview quality, subtitle quality, how hard videolore works), more accurate speech models, older processors, responsiveness, safe handling of files and downloads, private temporary files, quality goals.

## Constraints
- **ffmpeg is bundled** in every installer (since 0.2.0; before, it had to be installed by the user). A user-chosen ffmpeg can still be configured instead.
- **The product works fully offline.** The only network access is the one-time, user-approved download of the speech model of the chosen subtitle quality level (each model has its own consent), from fixed secure addresses and checked before use; a model can also be provided as a local file. There is no telemetry and no update check.
- **Settings belong to the program, never to a project**, and the main part of Settings uses plain words only; technical details stay in a collapsed "Advanced" section.

## Key decisions (general)
- Output format is automatic: there is no export settings dialog.
- Stream copy is used only when the result is guaranteed frame-accurate; correctness comes before speed.
- Fade length is fixed (not user-tunable) to keep the editor simple.
- The window layout is fixed; panels cannot be moved or resized.
- Installers are not code-signed in 0.2.0; signing is postponed, not rejected. The README explains the one-time first-launch steps.
- (0.3.0) videolore detects the computer at its first start and picks recommended settings; installers do no tuning. Everything can be changed in Settings, and "Use recommended settings" restores the recommendation.
- (0.3.0) Hardware video encoding for export is postponed (REQ-055 planned); older processors get a clear message instead of a fallback speech engine, which is also planned for later.

Topic-specific decisions are in each document.

## Known limitations (0.3.0)
- Subtitles in scripts other than Latin, Greek and Cyrillic are not fully supported (see [Subtitles](business-subtitles.md)).
- x86-64 processors without AVX2 (from before about 2013) cannot generate subtitles automatically; videolore shows a clear message instead of crashing, and lines can still be added by hand (see [Settings, performance and safety](business-performance.md)).
- Installers are unsigned, so each operating system shows a warning on first launch (see [Installation](business-install.md)).
- On Windows and Linux, double-clicking a project while videolore is open opens a second window.
- Projects saved by 0.2.0 cannot be opened by 0.1.x versions.
- On macOS the standard Hide and Minimize menu items are not available.

## Out of scope
- Multiple video tracks, overlays, picture-in-picture.
- Color grading, keyframe animation, transitions other than fade through black.
- Accounts, cloud storage, sync, collaboration, direct upload to social networks.
- Mobile versions.
- Export settings dialog.
- Recent-projects list, autosave, crash recovery, automatic updates.
- Code signing and notarisation (postponed).
- Telemetry, online services for detecting the computer, performance settings per project.

Out-of-scope items specific to a topic are listed in its document.
