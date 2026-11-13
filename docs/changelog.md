# Changelog

Unreleased changes, newest first. Each entry: version and date-time. On release this file becomes `docs/releases/vX.Y.Z.md`.

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
