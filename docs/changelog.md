# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 1.0.0 — 2026-11-14 22:36 +0000
- Version 1.0.0 opened after release 0.4.2 "MVP". No user-facing changes yet.

## 0.4.3 — 2026-11-14 22:35 +0000

Fixes the macOS and Windows installers, which the 0.4.2 installer job could not build. The Linux installers of 0.4.2 were unaffected. The macOS and Windows installers of 0.4.2 are not in its GitHub release; they are confirmed only when the installer job runs again.

### Fixed
- The generated cargo-packager configuration (`scripts/package.sh`) had four errors, all fixed: the file association role is `editor` (was `Editor`), the NSIS key is `installMode` (was `install-mode`), `name = "videolore"` is present (without it cargo-packager stops with "Not a directory"), and all paths are absolute, because cargo-packager reads relative paths from the folder of the configuration file (on Windows they are written with `cygpath -m`).
- Missing DLLs of the Windows ffmpeg build, and a missing produced installer or `.app`, now stop the script with a clear error.

### Added
- `scripts/package.sh <target> --config-only` writes only the configuration (no ffmpeg, no build), so it can be checked on any machine.
- Test `packaging::tests::packager_config_is_valid_for_cargo_packager` checks the keys, enum values and paths of both configurations (covers REQ-040, REQ-041, REQ-042).

### Changed
- `hdiutil create` for the macOS disk image is retried up to 5 times.
- Validated locally with the real cargo-packager 0.11.8: the deb and app formats were built from both configurations, and the NSIS script was generated.

### Quality
- Line coverage 96.3%; feature coverage 65/65. Tests: 223 unit and 23 integration, 2 ignored; fmt, clippy, coverage and version checks pass.