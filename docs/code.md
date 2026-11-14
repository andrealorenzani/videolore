# Code

videolore is a Rust desktop application (edition 2024, MSRV 1.100) built on egui/eframe 0.37. It never
decodes video itself: every read and render is done by the `ffmpeg` and `ffprobe` programs,
started as child processes.

## Build, test, run

| What | Command |
|---|---|
| Build | `cargo build` (release: `cargo build --release --locked`) |
| Run | `cargo run` |
| Tests | `cargo test` |
| Format / lint | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (CI also lints with `--no-default-features` and `--all-features`) |
| Line coverage (>= 85%) | `scripts/coverage.sh --fail-under 85` (needs `cargo-llvm-cov` and `rustup component add llvm-tools-preview`) |
| Feature coverage (100%) | `scripts/feature-coverage.sh` |

System requirements:

- **ffmpeg >= 4.4 and ffprobe** at run time (and for the integration tests). They are looked up
  in this order: the path chosen in the app (saved in the user's config folder as
  `settings.json`), the environment variables `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`, the folder
  of the videolore executable, then `PATH`. Without a usable ffmpeg the app shows a blocking
  message with install hints instead of crashing.
- **Linux build:** the ALSA headers (`sudo apt install libasound2-dev pkg-config`) for the
  preview audio. To build without audio output use `cargo build --no-default-features`
  (the preview is then silent and follows the wall clock).

Cargo features: `audio` (default, sound output via cpal), `stt` (reserved, empty: future
speech-to-text subtitles).

If line coverage suddenly drops far below the usual ~96% after a toolchain change, delete
`target/llvm-cov-target` and run again: old test binaries left there (cargo changed where it puts
them) are mixed into the report and count up-to-date lines as never run.

## Rust version and dependencies

The project follows the latest stable Rust. The minimum supported version (`rust-version` in
`Cargo.toml`, currently **1.100**) is raised together with the toolchain, and dependencies are
kept on their newest releases. Main dependencies: eframe/egui 0.37 (glow renderer, no default
features), rfd 0.18 (XDG portal on Linux), cpal 0.18 (optional, `audio`), serde 1, serde_json 1,
thiserror 2, directories 6, tempfile 3; tests use sha2 0.11.

## Layout of the code

All logic is in the library (`src/lib.rs` and its modules); `src/main.rs` only opens the window.

| Module | Responsibility |
|---|---|
| `time` | `Micros` (integer microseconds), `Fps` (rational), frame rounding, timecodes |
| `media` | `MediaItem`, `MediaFormat`, defensive parser of `ffprobe` JSON, `probe()` |
| `timeline` | the single video track: contiguous clips + fades at boundaries; every edit returns `Result<_, EditError>` |
| `history` | undo/redo by snapshots of the timeline (up to 100 steps) |
| `project` | `.vlproj` file (JSON, `schema_version` 1): atomic save, validated load, missing sources reported |
| `format` | same-format check, lowest-quality output choice, codec to encoder mapping with H.264/AAC/MP4 fallback |
| `render` | export plan (stream copy or one re-encode with a single `filter_complex`) and ffmpeg argv for export and preview |
| `ffmpeg` | locating binaries, version/filter/encoder checks, `Runner` trait (real processes or test fake), progress and keyframe parsers |
| `export` | export worker thread: progress, cancel, partial file and rename |
| `preview` | preview player: RGBA frames and PCM from ffmpeg pipes, audio-driven clock, seek |
| `app` | `AppState` and the single command dispatcher (`Command`, `AppState::dispatch`) |
| `ui` | panels (`ui::layout`), timeline zoom (`ui::zoom`), shortcuts (`ui::keymap`), drawing |
| `dialogs` | native file dialogs (rfd) |

## Main patterns

- **One dispatcher.** Buttons, context menus and shortcuts only create `Command` values; the UI
  runs them through `AppState::dispatch` at the end of the frame. `AppState::is_enabled` tells the
  UI which commands are available (e.g. Merge is disabled unless the clips are contiguous parts of
  the same file).
- **Edits on a copy.** An edit is applied to a copy of the timeline; only if it succeeds is the old
  timeline pushed on the undo stack. A failed edit changes nothing.
- **Sources are read-only.** Inputs are passed to ffmpeg as `file:<absolute path>`, the export is
  written to `<folder>/.<name>.videolore-partial` and renamed on success (deleted on cancel or
  error), and an output equal to a source is refused.
- **No shell.** Every process is started with an argument list; children are killed when their
  handle is dropped; on Windows no console window is opened.
- **Export plan.** Stream copy (concat demuxer, `-c copy`) only when all clips have identical
  format, there are no fades and every clip starts at 0 or at a keyframe; otherwise one re-encode
  where each clip is trimmed, scaled down keeping its aspect ratio, letterboxed, converted to the
  output frame rate, faded if needed, and concatenated. Clips without audio get silence.
- **Preview.** Two ffmpeg processes (video as raw RGBA at most 360 px high, audio as 16-bit PCM)
  render the edited timeline from the playhead; frames wait in a buffer of 8; the clock follows
  the audio actually played. Seeking restarts the processes.
- **Errors** are typed with `thiserror` in each module and shown to the user as plain messages;
  user input and project files never cause a panic.

## Tests

- Unit tests sit next to the code (`#[cfg(test)]`), using a fake `Runner` so no ffmpeg is needed.
- The UI is tested headless with `egui::Context::run_ui` (layout at several sizes, shortcuts, clicks).
- The fake `Runner` can keep the output of its fake processes open until they are killed
  (`holding_streams`), so tests about playback timing do not race the end of the stream.
- `tests/ffmpeg_integration.rs` runs the real ffmpeg. Test clips are generated at test time
  (`testsrc2` + `sine`) in a temporary folder; no media files are stored in git. The tests are
  skipped when ffmpeg is missing, unless `VIDEOLORE_REQUIRE_FFMPEG=1` (set in CI).
- Each implemented requirement of `docs/business.md` (`REQ-NNN (implemented)`) has at least one test marked `// covers: REQ-NNN` (checked by `scripts/feature-coverage.sh`).

## Conventions

- Every `.rs` file starts with `// SPDX-License-Identifier: GPL-3.0-only` and
  `// Copyright (C) <year> Andrea Lorenzani`.
- No `unsafe` code.
- Times are integer microseconds (`Micros`), never floating point, inside the model.

## Continuous integration

`.github/workflows/build.yml` installs `ffmpeg` and `libasound2-dev` on Linux, runs format, lint
(default, no and all features), tests (with `VIDEOLORE_REQUIRE_FFMPEG=1`), version check, line and
feature coverage on the latest stable Rust. A second job (`msrv`) reads `rust-version` from
`Cargo.toml` and builds and tests with exactly that toolchain. When both pass, it builds
release executables for Linux x86_64, macOS arm64 and Windows x86_64 on every push to `main`;
a `v*` tag attaches them to a GitHub release.

## Known limitations

Accepted by the author for this release; they are not bugs to fix silently.

- No "unsaved changes?" prompt on New, Open or quit: unsaved edits can be lost.
- Import runs `ffprobe` on the UI thread, so the window can freeze briefly on each file.
- Every edit starts `ffmpeg` to refresh the still preview frame.
- The preview video waits when the audio stalls.
- HEVC in MP4 is not tagged `hvc1`, so some players (e.g. Apple's) may not show it.
- Each clip is a separate ffmpeg input: many clips taken from one long file can hit decoder limits.
