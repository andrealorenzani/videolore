# Business

## Purpose
videolore is a simple, offline desktop video editor for people who make short clips for social media. It focuses on fast cutting and joining of clips, simple fade transitions, background music, automatic subtitles and quick export. It installs with one double-click on Windows, macOS and Linux, with everything it needs (including ffmpeg) included.

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

## Constraints
- **ffmpeg is bundled** in every installer (since 0.2.0; before, it had to be installed by the user). A user-chosen ffmpeg can still be configured instead.
- **The product works fully offline.** The only network access is the one-time, user-approved download of the speech model for subtitles; the model can also be provided as a local file. There is no telemetry and no update check.

## Key decisions (general)
- Output format is automatic: there is no export settings dialog.
- Stream copy is used only when the result is guaranteed frame-accurate; correctness comes before speed.
- Fade length is fixed (not user-tunable) to keep the editor simple.
- The window layout is fixed; panels cannot be moved or resized.
- Installers are not code-signed in 0.2.0; signing is postponed, not rejected. The README explains the one-time first-launch steps.

Topic-specific decisions are in each document.

## Known limitations (0.2.0)
- Subtitles in scripts other than Latin, Greek and Cyrillic are not fully supported (see [Subtitles](business-subtitles.md)).
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

Out-of-scope items specific to a topic are listed in its document.
