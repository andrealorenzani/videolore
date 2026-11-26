# Architecture: videolore player

Part of [Architecture](architecture.md). Requirements: [player](business-player.md) (REQ-138 to 141, 150, and 2.0.4 items 248 to 256). Engine: [engine](architecture-engine.md). Installers: [packaging](architecture-packaging.md).

## Overview
- A second program, `videolore-player`, with its own executable and installers (Windows, macOS, Linux). It plays one video with the 360 view, subtitles and full screen. It never edits, saves or exports. It reuses the editor's playback engine, so a video looks the same in both programs.

## Crate layout
- Root package `videolore` (library and editor); member `player/` (package `videolore-player`): `main.rs` (arguments, window), `app.rs` (state, commands), `ui.rs` (egui), `subs.rs` (subtitle sources and places), `settings.rs` and `playlist.rs` (2.0.4), tests.
- Depends on the library with `default-features = false, features = ["audio"]`: `whisper-rs` and `ureq` are not in its graph (CI checks with `cargo tree`). Root `cargo build` and `cargo test` build the editor only; `--workspace` covers both.

## How it works
- It builds an in-memory project with one clip (the whole file) and drives the existing preview engine, ffmpeg graphs, ASS subtitles, the camera and the job queue. There is no second command builder.
- Its state has no save, export or edit command, so none can run from the player.
- Subtitles are the lines of the chosen source in the video's own time. Embedded text tracks use the same extraction as the editor's "use the video's subtitles"; `.srt` files use the same reader (at most 16 MiB). Subtitle places are kept in the session only; never written.
- **360 video:** dragging the picture turns the camera with the same function and limits as the editor; a flat video does not turn. The preview restarts from the playhead with the fixed camera.
- **Full screen:** F11 or F toggles it. Esc closes an open popup first, then leaves full screen. The menu, title and status line hide; controls hide after 2 s without pointer movement. On entering, a popup says "Press Esc to exit full screen" and closes with OK or after 5 s (injected clock in tests).

## Start screen (1.6.2)
- Uses the library's splash code and window with `splash::PLAYER`: picture `assets/splash/splash-player.gif`, "Version X.Y" over x 78 to 386, y 388 to 462, summary `assets/splash/summary-player.txt`; rules as in [UI](architecture-ui.md#splash-160-162-204).
- Shown at a start without a video on the command line, as a splash state (its other popup is the full-screen hint). A video on the command line opens at once, with no splash; a `.vlproj` is refused and also means no splash.
- Non-blocking: playback keys, F11 and the mouse keep working. Esc closes the splash first, then another popup, then leaves full screen.

## Opening files, settings and privacy
- One video at a time: a command-line path, File > Open, or a dropped file where drag and drop works.
- A `.vlproj` is refused by extension with a message; the file is not read.
- No settings file in 1.6.0 (detected computer, recommended levels: Balanced, preview Automatic). 2.0.4 adds its own settings file (REQ-248, ADR-184) in the user's configuration folder, never next to the video; read with the editor's rules (regular file, size cap, defaults on damage).
- Writes only to the application cache (preview copies) and private work folders; never next to the video. No network; works offline.
- Sibling programs (editor and player) are found by fixed name next to the executable (`siblings`), absolute path, argument lists, no shell.

## 2.0.4 additions
- **Playlist (REQ-249 to 252):** the videos played in the session form a playlist (`player/src/playlist.rs`).
- **Open with (REQ-254 to 256):** offered for MP4, MOV, MKV, WebM, M4V and AVI. Windows uses its own NSIS template `packaging/windows-player.nsi` with "Open with" entries only (`OpenWithProgids`); the default program stays the user's choice (decided by Andrea); not run on Windows yet (manual item 22). The player does not register file types otherwise.
- **Picture subtitles (REQ-253, ADR-185):** `MediaFormat.picture_tracks` lists the bitmap subtitle streams (ordinal among all subtitle streams; kind `pgs`, `dvd` or `dvb`; language; title; sanitised on load). The menu lists them after text tracks marked "(picture)" (`Source::Picture`). Choosing one sets the overlay; any other choice clears it. The overlay is part of the shared graph before the trim: `[k:v:0][k:s:N]overlay=eof_action=pass:format=auto`. Other projects keep the same graph (golden argv unchanged). Limits: a subtitle already on screen when a run starts inside it appears at its next change (ffmpeg does not replay display sets after a seek); no picture subtitle on a 360 clip; not draggable. Test: a PGS stream written by hand in `player/tests/ffmpeg.rs`, checked at the box pixels before, inside and after its time.
- **Splash option (REQ-295, ADR-199):** the same `should_show` rule as the editor.
- **Audio menu (REQ-338, planned 3.1.0, ADR-252):** lists every audio track of the video (`MediaFormat.audio_tracks`, language or title, channel count); the first is chosen by default. The choice sets the in-memory clip's `audio_track` and is kept while the file stays open in the session; it is never saved. Subtitles keep their own menu (REQ-140, REQ-253). No network code is added.

## Several videos (3.1.2, REQ-355, 356; ADR-267)
- **File > Open…** takes several videos at once and replaces the list; they play in the order the dialog gives (name order when it gives none). A single file is a list of one.
- **File > Add to playlist** can be used while a video plays: the chosen videos are added at the end, in the chosen order; the current video keeps playing. Duplicates are refused as before. After the last video has ended, an added video does not start by itself (assumption).
- The window title reads "name (n of m)" when the list has more than one video. The list is kept for the session only (REQ-254).
- Code: `player/src/playlist.rs` (`replace`, `add`), commands `OpenVideos` and `AddToPlaylistPaths`; no new crate. As built: a video opened by itself (drop, command line) heads the list it starts.

## Installers and identity
- `scripts/package.sh <target> player`: AppImage and tar.gz (Linux), NSIS setup (Windows), disk image (macOS); the same pinned ffmpeg and ffprobe as the editor; own notices (`rust-crates-player.txt`). Identifier `io.github.andrealorenzani.videolore-player` (editor `…videolore`), so both install side by side. No file association and no Linux menu entry written by the program (2.0.4: "Open with" only).
- Icon: the same artwork as the editor (decided 2026-11-19).

## Tests
- Network and speech code absent from the dependency graph (CI check).
- An MP4 opens and plays (real ffmpeg); `.vlproj` refused without reading the file.
- The same drag on the same 360 file gives the same view in both programs (one shared function); limits; a flat video does not turn.
- Embedded track naming; `.srt` opened and selected; a dragged line moves; reopening gives the default place; no file written during the session.
- Full screen: F11 and F toggle; Esc order with a popup; popup timing and auto-hide with an injected clock.
- Installer configuration for both products (identifiers; file association only for the editor, Open with for the player in 2.0.4).

## Risks
- Scope creep: a playlist or file association was out of scope in 1.6.0 (playlist added in 2.0.4).
- A second copy of ffmpeg in the player installer adds download size (not measured).
- The player and the editor share the preview engine: a change is tested in both.
- Picture subtitles are not draggable and not shown on a 360 video.
