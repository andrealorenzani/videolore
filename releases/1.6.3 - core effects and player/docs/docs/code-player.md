# Code — videolore player and the workspace (1.6.0, start screen 1.6.2)

Back to the [index](code.md). Requirements REQ-138 to REQ-141, REQ-150 (start screen) (and REQ-142 for its icons).

## Workspace

The root `Cargo.toml` is a workspace: `members = ["player"]`, `default-members = ["."]`, `resolver = "3"`. The root package
`videolore` (library and the editor) is unchanged; `player/` is the package `videolore-player` (library + a thin `main.rs`).
`videolore-player` depends on `videolore` with `default-features = false, features = ["audio"]`: no whisper-rs, no ureq, no
cmake or libclang. CI (and a unit test) check with `cargo tree -p videolore-player -e normal` that neither `ureq`, `whisper-rs`,
`rustls` nor an HTTP client is in its graph. Both packages carry the same literal version (`scripts/version.sh` writes both and
both Cargo.lock entries). `--workspace` is used by CI for fmt, clippy (three feature sets), tests with coverage and MSRV;
`scripts/coverage.sh` covers the whole workspace (`src/main.rs` and `player/src/main.rs` excluded) and
`scripts/feature-coverage.sh` also scans `player/`.

Shared code moved or opened for the player (no copy): `stt::read_track(ffmpeg, runner, work, source, ordinal, range)` (reads an
embedded text track; the speech matching uses it too), `videolore::app_icon()`, `ui::subtitle_font`, `notices::about_text_player`
(own Rust crate list `packaging/notices/rust-crates-player.txt`), plus the existing `preview`, `render`, `camera`, `srt`,
`media::probe`, `proxy`, `ui::placement`, `ui::icons`, `jobs`.

## Modules of `player/src`

| File | What |
|---|---|
| `main.rs` | thin: the argument, `native_options`, `real_app` |
| `lib.rs` | `real_app` (opens the start screen when no video is given), `native_options`, `file_arg`, native dialogs (rfd) |
| `app.rs` | `PlayerState`, `Command`, the open rules, transport, 360 camera, subtitle commands, probing and preview copies |
| `fullscreen.rs` | the full-screen state machine with an injected clock |
| `subs.rs` | subtitle sources (embedded tracks, `.srt` files), session positions, `read_srt` |
| `ui.rs` | the egui window: menu bar, controls, picture and its drags, popups (the start screen first) |
| `tests.rs`, `ui_tests.rs`, `tests/ffmpeg.rs` | unit tests with a scripted ffprobe, UI tests, real-ffmpeg tests |

## How it works (ADR-105..108, 118, 119)

- **One-clip project.** Opening a file probes it in the background (`media::probe`, Probe priority) and builds an in-memory
  `Project` with one clip. The editor's `Preview` plays it; nothing is ever saved. Big videos get a preview copy through
  `proxy::ProxyWorker` into the cache shared with the editor (Automatic rule, `machine::detect`); no settings file exists yet.
- **Commands.** `Command` has no save, export or edit variant, and a test matches it exhaustively. `dispatch` runs a command and
  puts an error in `status`. `PlayerState::new(ffmpeg, runner, dialogs, sink, machine, Folders, now)` takes everything from
  outside, so tests inject a scripted `Runner`.
- **Open rules.** argument, File > Open…, drop. A name ending in `.vlproj` (any case) is refused with a notice **before anything
  is read**; a file without a picture or one ffprobe cannot read is refused with a message.
- **Playback.** Space or the button; the seek slider; arrows move 5 s; volume and mute live in `project.mix` and restart the
  stream when playing; clicking the picture plays or pauses, double-clicking goes full screen. A volume change is heard after a
  short restart of the stream (the engine builds the sound once per run).
- **360 (REQ-139).** The file's own information decides. Dragging uses `Camera::dragged` and `zoomed` (same code and
  clamping as the editor); the camera is `live`, a one-POV overlay on the shown project (`shown_project`), playback restarts
  100 ms after the last drag. A flat video refuses the drag with a message.
- **Subtitles (REQ-140).** The Subtitles menu lists None, the file's text tracks (named by `SubtitleTrack::label`) and every `.srt`
  opened. A track is converted by ffmpeg in a private work folder (`stt::read_track`), a file is read with a 16 MiB limit; the
  cues become `Subtitles` lines of the in-memory project, which the preview burns in like the editor's. A line is dragged with
  `ui::placement` (box, hit, `drag_to`, clamp as the editor; on 360 the whole box stays on the picture); the place is kept per
  source and line in `Subs::placed` for the session only.
- **Full screen (REQ-141).** F11, F or View > Full screen; the window gets `ViewportCommand::Fullscreen`. In full screen the menu
  and status are not drawn and the controls float at the bottom, hidden after 2 s without pointer events. The hint popup
  ("Press Esc to exit full screen" + OK) closes with OK or after 5 s; Esc closes About, then a notice, then the hint, then leaves.
- **Writes.** Only the preview-copy cache, the private work folders and the preview's scratch folder; a test and the real-ffmpeg
  tests compare the video's folder before and after.

## Packaging (ADR-108, 115)

`scripts/package.sh <target> [editor|player] [--config-only]`: the player has its own `Packager.toml` (name `videolore-player`,
product name "videolore player", identifier `io.github.andrealorenzani.videolore-player`, **no** `.vlproj` association, the
editor's icons), an AppImage desktop entry with `Categories=AudioVideo;Player;Video;` and no MimeType, a tar.gz README that says
how to start it (no application-menu self-integration, no `.videolore-bundle` marker), the same pinned ffmpeg (a second copy
inside the player's installer) and `rust-crates-player.txt` as its crate notice. Artifacts: `videolore-player-<ver>-<target>.{AppImage,tar.gz,dmg}`
and `-setup.exe`. The CI `package` job builds both products; the `build` job uploads both bare executables. A unit test generates the
configuration of both products for Windows and macOS and checks keys, distinct identifiers and the association rule.

## Known limits

No player settings (volume and window size are not remembered), no playlist, no file association, no bitmap subtitles, subtitle
styling is the editor's default, no preview copy shown in the UI (the copy is made silently). The installer carries a second ffmpeg
(size). The About text shares `third-party.txt` with the editor, which also describes the editor's speech files. A 360 video wider
than the editor's limit is refused like in the editor.

## Start screen (1.6.2, REQ-150)

`PlayerState::splash` (set by `show_splash`, which `real_app(None)` calls) is drawn by `popups()` through the shared window
`videolore::ui::splash_window::show` with `videolore::splash::PLAYER` (picture `assets/splash/splash-player.gif`, text
rectangle x 78..386 y 388..462, summary `assets/splash/summary-player.txt`). `Command::CloseSplash` (Close or the window's
control) and `Command::Escape` (before About, the notice, the full-screen hint and full screen) close it; opening a video
closes it too. It is not modal, so Space and F11 keep working. `PlayerCache` holds the textures and `splash_rects` (for tests).
Details: [code-splash-icons.md](code-splash-icons.md).
