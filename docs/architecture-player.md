# Architecture: videolore player

Part of [Architecture](architecture.md). Requirements REQ-138 to REQ-141 and REQ-150 ([business](business-player.md)). Since 1.6.0; the start screen (REQ-150) since 1.6.2; the settings file, playlist, "Open with" and picture subtitles of 2.0.4 are in the last section.

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
- The player uses the library's splash code and window (`videolore::splash`, `videolore::ui::splash_window`), with `splash::PLAYER`: the picture `assets/splash/splash-player.gif`, "Version X.Y" over the rectangle x 78 to 386, y 388 to 462, and the summary `assets/splash/summary-player.txt`. The rules are those of the editor's splash (see [splash](architecture-splash.md)).
- It opens at a start without a video on the command line, as a splash state of the player (its only other popup is the full-screen hint). A video on the command line opens at once, with no splash; so does a `.vlproj`, which is refused.
- Non-blocking: playback keys, F11 and the mouse keep working. Esc closes the splash first, then another popup, then leaves full screen.

## Opening files
- One video at a time: a path on the command line, File > Open, or a dropped file where drag and drop works.
- A `.vlproj` is refused by its extension, with a message; the file is not read.
- The player does not register file types in 1.6.0 (since 2.0.4: "Open with" only, below).

## Settings, files and privacy
- No settings file in 1.6.0. Start-up uses the detected computer and the recommended levels (Balanced, preview Automatic). Since 2.0.4 the player has a settings file of its own (below).
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
- Bitmap subtitles (PGS, DVD, DVB) are listed since 2.0.4 as "(picture)"; the picture subtitle is not draggable and is not shown on a 360 video.

## 2.0.4 additions (stages 9 and 14)
- **Settings file (REQ-248, ADR-184):** the player keeps its program settings in its own file in the user's configuration folder (`player/src/settings.rs`), never next to the video. It is read with the editor's rules (regular file, size cap, defaults on damage). Sibling programs are found by fixed name next to the player's own executable (`src/siblings.rs`), with an absolute path, argument lists and no shell.
- **Playlist (REQ-249 to REQ-252):** the videos played in the session form a playlist (`player/src/playlist.rs`).
- **Open with (REQ-254 to REQ-256):** the player is offered under "Open with" for MP4, MOV, MKV, WebM, M4V and AVI. Windows uses its own NSIS template, `packaging/windows-player.nsi`, with "Open with" entries only (`OpenWithProgids`); the default program stays the user's choice (decided by Andrea). The template is not run on Windows yet (manual item 22). Sibling programs are started by fixed name (ADR-184).
- **Picture subtitles (REQ-253, ADR-185):** `MediaFormat.picture_tracks` lists the bitmap subtitle streams of a file (ordinal among all subtitle streams, kind `pgs`, `dvd` or `dvb`, language, title; sanitised on load). The player's menu lists them after the text tracks, marked "(picture)" (`Source::Picture`). Choosing one sets the overlay; any other choice clears it. The overlay is part of the shared render graph, before the trim: `[k:v:0][k:s:N]overlay=eof_action=pass:format=auto`, so the subtitle is scaled and fitted with the picture. Other projects get the same graph as before (golden argv unchanged).
- **Limits:** a picture subtitle already on screen when a run starts inside it (or when a paused picture starts inside it) appears at its next change, because ffmpeg does not replay display sets after a seek; no picture subtitle on a 360 clip; nothing to drag.
- **Test:** the picture subtitle fixture is a PGS stream written by hand in `player/tests/ffmpeg.rs` (`pgs`, own work), muxed and checked at the pixels of the box before, inside and after its time.
- **Splash option (REQ-295, ADR-199):** the player uses the same rule as the editor (`should_show`: the option, the last shown version and a path argument).
