# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release this file is frozen in `release/<version>/docs/changelog.md` and reset.

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
