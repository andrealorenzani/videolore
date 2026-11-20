# Architecture: videolore player

Part of [Architecture](architecture.md). Requirements REQ-138 to REQ-141 and REQ-150 ([business](business-player.md)). Since 1.6.0; the start screen (REQ-150) since 1.6.2.

## Overview
The player is a second program, `videolore-player`, with its own executable and its own installers for Windows, macOS and Linux. It plays one video file with the 360 view, subtitles and full screen. It never edits, saves or exports. It reuses the editor's playback engine, so a video looks the same in both programs.

## Crate layout (workspace)
- Root package `videolore` (library and editor): unchanged layout.
- Member `player/` (package `videolore-player`): `main.rs` (arguments, window), `app.rs` (state, commands), `ui.rs` (egui), `subs.rs` (subtitle sources and places), tests.
- The player depends on the library with `default-features = false, features = ["audio"]`. The speech-to-text feature is off, so `whisper-rs` and `ureq` (network) are not in the player's dependency graph. CI checks this with `cargo tree`.
- `cargo build` and `cargo test` at the root build the editor only (`default-members`); `--workspace` covers both.

## How it works
- The player builds an in-memory project with one clip (the file, whole length) and drives the existing preview engine, `ffmpeg` graphs, ASS subtitles, the camera and the job queue. There is no second command builder.
- The player's state has no save, export or edit command, so none of them can run from the player.
- Subtitles are the lines of the chosen source, in the video's own time. Embedded text tracks use the same extraction as the editor's "use the video's subtitles". `.srt` files are read with the same reader (at most 16 MiB).
- Subtitle places on the picture are kept in the session only; they are never written.
- 360 video: dragging the picture turns the camera with the same drag function and limits as the editor. The preview restarts from the playhead with the fixed camera, as in the editor.
- Full screen: F11 or F toggles it (window fullscreen). Esc closes an open popup first, then leaves full screen. The menu, title and status line are hidden; the controls hide after 2 seconds without pointer movement. On entering full screen a popup says "Press Esc to exit full screen" and closes with OK or after 5 seconds.

## Start screen (1.6.2)
- The player uses the library's splash code and window (`videolore::splash`, `videolore::ui::splash_window`), with `splash::PLAYER`: the picture `assets/splash/videolore-player.gif`, "Version X.Y" over the rectangle x 78 to 386, y 388 to 462, and the summary `assets/splash/<version>-player.txt`. The rules are those of the editor's splash (see [splash](architecture-splash.md)).
- It opens at a start without a video on the command line, as a splash state of the player (its only other popup is the full-screen hint). A video on the command line opens at once, with no splash; so does a `.vlproj`, which is refused.
- Non-blocking: playback keys, F11 and the mouse keep working. Esc closes the splash first, then another popup, then leaves full screen.

## Opening files
- One video at a time: a path on the command line, File > Open, or a dropped file where drag and drop works.
- A `.vlproj` is refused by its extension, with a message; the file is not read.
- The player does not register file types (no double-click from the file manager in 1.6.0).

## Settings, files and privacy
- No settings file in 1.6.0. Start-up uses the detected computer and the recommended levels (Balanced, preview Automatic).
- The player writes only to the application cache (preview copies) and to its private work folders. It never writes next to the video.
- No network access; the player works offline.

## Installers
- `scripts/package.sh <target> player` builds the player's installers (AppImage, tar.gz for Linux; NSIS setup for Windows; disk image for macOS). They carry the same pinned `ffmpeg` and `ffprobe` as the editor and their own notices (see [packaging](architecture-packaging.md)).
- Identifiers differ from the editor's, so both programs install side by side.
- Icon: the player uses the same icon artwork as the editor (decided 2026-11-19), so the two programs look alike in the Dock and taskbar.

## Tests
- Network and speech code absent from the player's dependency graph (CI check).
- An MP4 opens and plays (real `ffmpeg`); `.vlproj` refused without reading the file.
- The same drag on the same 360 file gives the same view in both programs (one shared function); the view stops at the limits; a flat video does not turn.
- Embedded track naming; `.srt` opened and selected; a dragged line moves; reopening gives the default place; no file is written during the session.
- Full screen: F11 and F toggle; Esc order with a popup; popup timing and auto-hide with an injected clock.
- Installer configuration for both products (identifiers, file association only for the editor).

## Risks
- Scope creep: users may expect a playlist or file association (both out of scope for 1.6.0).
- A second copy of `ffmpeg` in the player installer adds download size (not measured).
- The player and the editor share the preview engine: a change to the engine is tested in both.
- Bitmap subtitles (Blu-ray, DVB) are not listed.
