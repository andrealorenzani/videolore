# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release this file is frozen in `release/<version>/docs/changelog.md` and reset.

## 0.1.5 — 2026-11-14 01:03 +0000
Summary: the project follows the latest Rust (1.100) and the latest libraries (egui/eframe 0.37, rfd 0.18); the editing features are unchanged.

- **Changed**: minimum supported Rust (`rust-version`) raised from 1.88 to 1.100, the current stable release; the code is built and tested with it in CI.
- **Changed**: eframe/egui 0.33 → 0.37 and rfd 0.17 → 0.18, with the UI code adapted to the new API. Other dependencies were already on their latest releases (cpal 0.18.2, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0, sha2 0.11.1).
- **Changed**: CI uses actions/checkout v7, actions/upload-artifact v7 and softprops/action-gh-release v3 (Node 24), which removes the Node 20 deprecation warning. Clippy also runs with `--no-default-features` and `--all-features`.
- **Added**: CI job `msrv` builds and tests with the Rust version in `rust-version`; executables are built only when both the `check` and `msrv` jobs pass.
- **Fixed**: code that would stop compiling in a future Rust (untyped float literals in UI strokes now have an explicit `f32` type).
- **Fixed**: two clippy lints for the latest Rust (chunked audio conversion in preview and integration tests; an index loop in the timeline).
- **Fixed**: a preview test that failed about once in 16 runs is now deterministic.
- **Removed**: the known limitation "CI tests with the latest stable Rust; MSRV only verified locally": both are now checked in CI.
- **Workflow**: the Architect's Rust toolchain policy (follow the latest stable Rust, fix new lints in the code instead of pinning old versions) is recorded in `docs/architecture.md`; planned features no longer carry a release number (music and subtitles: release not yet assigned).
- **Quality**: 71 unit tests and 7 integration tests pass; line coverage 96.0%; feature coverage 18/18 implemented requirements; no future-incompatibility warnings.

## 0.1.4 — 2026-11-14 00:39 +0000
- **Fixed**: `scripts/version.sh set` is now all-or-nothing: if any step fails, `Cargo.toml`, `Cargo.lock` and the README badge are restored, so the version can no longer be left half-updated. It refreshes `Cargo.lock` with `cargo update --workspace` instead of a build, so it no longer needs system libraries such as ALSA.
- **Workflow**: owners may split their human-readable doc into `docs/<area>-<topic>.md` sub-documents (and should past ~150 lines or with distinct topics), keeping `docs/<area>.md` as the index; rule in `AGENTS.md`, guidance and an example in each agent; `scripts/feature-coverage.sh` also reads `docs/business-*.md`; sub-documents are not copied by `/clone-workflow`.

## 0.1.3 — 2026-11-14 00:26 +0000
Summary: videolore becomes a desktop, non-destructive video editor built on ffmpeg, for short social videos: import, cut, join and trim clips on one timeline, fade between clips, preview with sound, save projects and export to one file.

- **Added**: desktop window with a fixed five-panel layout (media list 30% | preview 70% in the top 60%; timeline 70% | effects/settings 30% in the middle 30%; bottom 10% bar with the zoom slider) (REQ-024).
- **Added**: keyboard shortcuts for split, cut, copy, paste, undo, redo and play/pause (REQ-025). Undo/redo keeps up to 100 steps (REQ-004).
- **Added**: import of video and audio files into a media list showing name and duration; unreadable files are rejected with a message (REQ-002).
- **Added**: single video track that never overlaps: append (REQ-005), split at playhead (REQ-006), cut/copy/paste with ripple (REQ-007), merge contiguous pieces of the same source (REQ-008), trim at either edge within the source (REQ-009), move clips (REQ-010).
- **Added**: fade through black between two clips: 0.5 s fade-out and 0.5 s fade-in on video and audio, half the clip length for clips under 1 s; can be removed (REQ-011).
- **Added**: timeline zoom slider from showing the whole project to single-frame level (REQ-012).
- **Added**: preview with play, pause (Space) and seek from the playhead, at most 360 px high, with sound (cpal) or silent when no audio device is available (REQ-013).
- **Added**: save and open projects as `.vlproj` (JSON, schema version 1); missing source files are reported by name (REQ-003). Source files are never modified (REQ-001).
- **Added**: export to one file with progress and cancel; a cancelled or failed export leaves no partial file (REQ-021).
- **Added**: export does stream copy (no re-encode) when all clips share the same format, there are no fades and every cut starts on a keyframe (REQ-023); otherwise one re-encode pass to the lowest-quality format among the clips, letterboxed, never stretched (REQ-022).
- **Added**: requirement for ffmpeg and ffprobe 4.4 or newer, installed by the user (not bundled). They are found via the app setting, `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`, the folder of the executable, or `PATH`. Missing ffmpeg shows a blocking message with install hints.
- **Added**: Cargo features `audio` (default, preview sound through cpal) and `stt` (reserved, empty, for planned subtitles).
- **Added**: CI installs ffmpeg, `libasound2-dev` and `pkg-config`, and runs the ffmpeg integration tests with `VIDEOLORE_REQUIRE_FFMPEG=1` (they fail instead of being skipped).
- **Changed**: `src/main.rs` opens the editor window; the placeholder text output is removed. Logic lives in library modules: `time`, `media`, `timeline`, `history`, `project`, `format`, `render`, `ffmpeg`, `export`, `preview`, `app`, `dialogs`, `ui` (`layout`, `zoom`, `keymap`).
- **Changed**: README describes the product, installation, requirements and usage; badges for the main libraries and for the ffmpeg requirement replace the "no dependencies" badge.
- **Security**: ffmpeg is started without a shell, with separate argument lists; inputs use the `file:` prefix and ffmpeg accepts only the `file` and `pipe` protocols (no network access, no option injection from file names); project files are validated after loading; exports never overwrite a source file.
- **Dependencies**: added eframe 0.33.3 (glow backend), rfd 0.17.2 (xdg-portal), cpal 0.18.2 (optional, feature `audio`), serde 1, serde_json 1, thiserror 2, directories 6, tempfile 3; dev-dependency sha2 0.11.1.
- **Quality**: 71 unit tests and 7 integration tests that run the real ffmpeg on clips generated at test time; line coverage 96.4%; feature coverage 18/18 implemented requirements.

Known limitations (accepted for this release):
- No "unsaved changes?" prompt on New, Open or quit.
- Import runs ffprobe on the UI thread, which can freeze the window briefly.
- Every edit starts ffmpeg to refresh the still preview frame.
- The preview video waits when audio stalls.
- HEVC in MP4 is not tagged `hvc1`.
- Each clip is a separate ffmpeg input, so many clips taken from one long file can hit decoder limits.
- CI tests with the latest stable Rust; MSRV 1.88 is verified only locally.

## 0.1.2 — 2026-11-13 23:31 +0000
- **Added**: README coverage badge (one decimal, currently 100.0%); code moved into `src/lib.rs` with a unit test, `src/main.rs` only prints.
- **Added**: CI checks version consistency, line coverage ≥ 85%, feature coverage 100% and that frozen release docs are untouched; tag releases use the release log as notes.
- **Removed**: `docs/releases/` (replaced by `release/<version>/`).
- **Workflow**: `scripts/version.sh` changes the version everywhere (Cargo.toml, Cargo.lock, README) and checks it; new `/release` command (major bump, freeze README/docs/ai-docs in `release/<version>/`, reset changelog, tag `v<version>`, push; CI creates the GitHub release); edits under `release/` denied in `.claude/settings.json`; Principal Engineer must reach ≥ 85% line coverage and 100% feature coverage (`REQ-NNN` ↔ `// covers:` tests); scripts `coverage.sh`, `feature-coverage.sh`, `collect-release.sh`, `release-guard.sh`.

## 0.1.1 — 2026-11-13 23:16 +0000
- **Added**: GitHub Actions build (`.github/workflows/build.yml`): fmt/clippy/test, then release executables for Linux x86_64, macOS arm64 and Windows x86_64, uploaded as artifacts and attached to GitHub releases on `v*` tags.
- **Added**: copyright notice for Andrea Lorenzani in `README.md` and SPDX/copyright header in `src/main.rs` (the GPL-3.0 text in `LICENSE` stays unmodified, as the licence requires).
- **Changed**: README "Latest changes" lists versions instead of dates; README contains no references to AI or the workflow; poem lines render as separate lines on GitHub.
- **Workflow**: every change bumps the version (last number +1, unless the user names a version); changelog entries carry version and date-time; commits use a human-readable one-line message; new source files carry the SPDX/copyright header.

## 0.1.0 — 2026-11-13 23:04 +0000
- **Workflow**: Documenter agent color changed to yellow; repository linked to `git@github.com:andrealorenzani/videolore.git` (`origin`, branch `main`).

## 0.1.0 — 2026-11-13 23:01 +0000
- **Added**: Rust crate `videolore` 0.1.0 (edition 2024, Rust 1.88), GPL-3.0 licence.
- **Workflow**: agents Product Manager, Architect, Principal Engineer, Documenter; commands `/add-feature`, `/change-workflow`, `/clone-workflow`; rules in `AGENTS.md` (`CLAUDE.md` symlink); workflow described in `docs/workflow.md`.
