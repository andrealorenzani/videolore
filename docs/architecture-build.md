# Architecture: build, CI and versions

Part of [Architecture](architecture.md). Since 1.6.0 with the workspace; the rest is as built since 0.2.0.

## Rust toolchain policy
- The project follows the most recent stable Rust; `rust-version` in `Cargo.toml` is the current stable release (1.100). Building from source needs an up-to-date Rust (`rustup update`).
- All dependencies are kept on their latest releases. When a new Rust release brings new warnings or lints, the code is fixed; old versions are not pinned.

## Workspace (1.6.0)
- Root package `videolore` (library and editor) and member `player/` (package `videolore-player`). `default-members` is the root, so `cargo build` and `cargo test` keep building the editor; CI uses `--workspace` where both must be built and tested.
- The player uses the library without the speech feature: its build needs no `cmake` and no libclang.
- Versions: root and player `Cargo.toml` both carry the version; `scripts/version.sh` writes both, and the `Cargo.lock` entries of both packages, and `check` compares them with the README badge.
- A later extraction of the engine into its own crate is possible (not planned; see the roadmap).

## CI (changes in 1.6.0)
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

## CI time (1.6.1, QG-06)
- **Measured** (public GitHub API, job and step times of the runs of 2026-11-14 to 2026-11-19): the push of 1.6.0 (`3b09f97`) took 28 min to its last executable. Its check job took 25.2 min, of which the system-packages step took 20.0 min. In the other recent runs that step took 0.2 to 1.5 min, once 5.4 min. The rest of the check job took about 5.2 min: the three clippy runs 0.3 min, the instrumented tests 2.3 min, the crate notices 1.1 min (`cargo-about` is compiled from source on every run).
- **Steps:**
  1. Install only the packages the image does not have. The Ubuntu 24.04 runner image lists `cmake` and `pkg-config`, so they are not installed again. `libasound2-dev` (audio, needed for the default build), `libclang-dev` (bindgen for the speech engine), `ffmpeg` (tests) and `espeak-ng` (speech tests) are installed with `--no-install-recommends`, after `apt-get update` with retries and a 30 s HTTP timeout, so a stalled mirror fails fast and is retried. The check job gets a `timeout-minutes` limit.
  2. `cargo-about` 0.9.3 from a prebuilt binary (the already pinned `taiki-e/install-action`), not `cargo install` (about 1 min saved per run).
  3. The speech test may read the committed `tests/fixtures/align/speech.mkv` instead of making speech with `espeak-ng` (the test's assertions stay; one package less). Open to the developer's choice.
  4. Not changed: every check, the tests and coverage thresholds, the executables and the MSRV build. The executables still start after the check job, so no untested binary is built on a push. No third-party apt cache action is added (supply chain).
- **Estimate** (not yet measured on the new workflow): check job about 4.4 to 4.7 min, i.e. about 80% shorter than the 25.2 min of the 1.6.0 run and about 20 to 25% shorter than a typical run of the same steps (about 5.9 min). Push to last executable: about 7 min instead of 28. The 30% target of QG-06 is an assumption: the measured time of the first run after the change is reported against both baselines.

## Assets and CI effects (1.6.2)
- Four splash files are compiled in, by `include_bytes!` and `include_str!` in the library (`src/splash.rs`), with paths from `CARGO_MANIFEST_DIR`: `assets/splash/splash.gif`, `assets/splash/splash-player.gif`, `assets/splash/summary.txt`, `assets/splash/summary-player.txt`. There is no `build.rs`: a missing file is the compiler's error, naming the file.
- Every build needs the two summaries, the editor's and the player's (both are in the library). Their names are fixed (`summary.txt`, `summary-player.txt`), so a version bump needs no new file; the summaries are rewritten for each version.
- CI adds no step. Fmt, clippy, tests, the MSRV build and the executables all compile the library, so a missing asset fails them at once.
- Git: the new player picture and the summaries must be committed with the code; the release `sources.zip` takes untracked files that are not ignored, so it is complete either way.
- 1.6.3: the per-version files (`1.6.0.gif`, `1.6.1.gif`, `<version>.txt`, `<version>-player.txt`) are gone; the pictures and summaries have fixed names.

## CI checks of 2.0.2 (planned)
The test job uses the pinned bundled ffmpeg (not the runner's), the cargo-about version is checked, and a `package-config` job checks the installer configuration on every push. Details: [verification](architecture-verification.md#automatic-builds-qg-10).

## Test memory cap (1.6.2, development machine)
- Every build and test run on the development machine (15 GB RAM) is capped: `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 <command>`, for example `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 cargo test --locked --workspace`. Child processes (rustc, test binaries, ffmpeg) stay in the same scope, so they are capped too. Coverage runs under the same cap.
- A run killed by the cap is a failed run, reported with the memory cause; the cap is raised only with a written reason.
- CI runners are not capped by this rule (they have their own limits). The 8 GB value is an assumption to check on the first full run.
