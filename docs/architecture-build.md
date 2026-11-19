# Architecture: build, CI and versions

Part of [Architecture](architecture.md). Planned for 1.6.0 with the workspace; the rest is as built since 0.2.0.

## Rust toolchain policy
- The project follows the most recent stable Rust; `rust-version` in `Cargo.toml` is the current stable release (1.100). Building from source needs an up-to-date Rust (`rustup update`).
- All dependencies are kept on their latest releases. When a new Rust release brings new warnings or lints, the code is fixed; old versions are not pinned.

## Workspace (1.6.0, planned)
- Root package `videolore` (library and editor) and member `player/` (package `videolore-player`). `default-members` is the root, so `cargo build` and `cargo test` keep building the editor; CI uses `--workspace` where both must be built and tested.
- The player uses the library without the speech feature: its build needs no `cmake` and no libclang.
- Versions: root and player `Cargo.toml` both carry the version; `scripts/version.sh` writes both, and the `Cargo.lock` entries of both packages, and `check` compares them with the README badge.
- A later extraction of the engine into its own crate is possible (see the plan's future work).

## CI (planned changes for 1.6.0)
- Check job: fmt, clippy (three feature sets) and tests with `--workspace`; coverage over the workspace (both `main.rs` files excluded), 85% minimum, as before.
- New check: `cargo tree -p videolore-player` must not contain `ureq` or `whisper-rs`.
- Notices: `scripts/notices.sh --check` checks the editor's and the player's Rust notices.
- Feature coverage scans `src`, `tests` and `player`.
- Build job: a player executable is built and named like the editor's.
- Package job: the installers of both products on each runner (`scripts/package.sh <target>` and `scripts/package.sh <target> player`); release files are the same globs as before, so both products are attached to the release.
- The bundled `ffmpeg` smoke test also checks `lut3d`.

## CI (as built)
- CI runs on fixed runner versions (Ubuntu 24.04, macOS 15, Windows Server 2025) instead of "latest", and every third-party CI action is pinned to an exact commit of a release that runs on Node.js 24. See [packaging](architecture-packaging.md#ci-runners-and-actions).
- CI installs `ffmpeg`, `libasound2-dev`, `cmake`, `libclang-dev` and `espeak-ng` on Linux. The check job sets `VIDEOLORE_REQUIRE_FFMPEG=1`, so tests that need `ffmpeg` fail instead of being skipped.
- The tests run once in CI, with coverage measurement (since 0.3.0); the minimum-Rust job only builds. Test media are generated once and reused, and are as small as each test allows.
- Building from source needs `libasound2-dev` on Linux (audio) and, for the default speech-to-text feature, `cmake`, a C++ compiler and libclang. `--no-default-features` builds without both.
- CI also checks licences, security advisories and withdrawn ("yanked") versions of all crates (`cargo-deny`) and that the generated crate notices are up to date.
- Release installers are built by CI for each version tag, see [packaging](architecture-packaging.md).
