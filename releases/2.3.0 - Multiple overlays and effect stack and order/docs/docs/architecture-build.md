# Architecture: build, CI, tests and verification

Part of [Architecture](architecture.md). Installers: [packaging](architecture-packaging.md). Quality gates and targets: [NFR](architecture-nfr.md). Manual release checks: [release verification](business-verification.md).

## Toolchain and workspace
- Latest stable Rust is the policy (ADR-012): `rust-version` 1.100 in `Cargo.toml`; all dependencies on their latest releases; new lints are fixed in the code, versions are not pinned. Build from source needs `rustup update`.
- Workspace (1.6.0): root package `videolore` (library and editor) and member `player/` (`videolore-player`). `default-members` is the root, so `cargo build` and `cargo test` build the editor; CI uses `--workspace`. The player uses the library without the speech feature: no cmake, no libclang needed for it.
- Versions: root and player `Cargo.toml` both carry the version, `scripts/version.sh` writes both and the `Cargo.lock` entries and checks the README badge (ADR-116). Extraction of the engine into its own crate: possible, not planned.
- Features: `audio` (default), `stt` (default; ADR-022). `--no-default-features` builds without sound and speech.
- whisper.cpp: `GGML_NATIVE=OFF` in `.cargo/config.toml` (fixed AVX2, FMA, F16C baseline; [subtitles](architecture-subtitles.md#cpu-requirements)).

## Quality gates (as run before a release and by CI)
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (three feature sets in CI), `cargo test` (no doctests), `scripts/coverage.sh --fail-under 85` (line coverage, `main.rs` excluded), `scripts/feature-coverage.sh` (every implemented requirement has a `// covers: REQ-NNN` test), `scripts/docs-check.sh`, `scripts/test-annotations.sh` (CI failures as annotations).
- Tests run once in CI under coverage (since 0.3.0). The minimum-Rust job only builds.
- Module architecture: `tests/module_cycles.rs` parses `src/` with `syn` and fails on a cycle, on a `crate::` path in `base`, or on `#[path]` (ADR-140; 2.0.1). The check takes about 1.7 s.
- UI acceptance tests use the AccessKit tree (ADR-045); interface frame gates run as sampled tests (best of up to five, same limits; QG-18).

## CI (`.github/workflows/build.yml`)
- Check job: fmt, clippy, tests with `--workspace`; coverage over the workspace (both `main.rs` excluded), 85 % minimum; feature coverage 100 %; version consistency and frozen release docs; notices check (`scripts/notices.sh --check`, cargo-about 0.9.3 pinned prebuilt, 1.6.1); `cargo tree -p videolore-player` must not contain `ureq` or `whisper-rs` (1.6.0).
- Build job: bare executables for every push and pull request (not attached to releases); a player executable beside the editor's.
- Package job: installers of both products on each runner on each version tag (see [packaging](architecture-packaging.md)); `package-config` job on every push (installer configuration with `--config-only` and `--stub`, no ffmpeg download; 2.0.2); reusable `installer-config.yml` on packaging PRs (ADR-181); weekly advisories (`advisories.yml`, read-only).
- Runners: `ubuntu-24.04`, `macos-15`, `windows-2025` (ADR-036; fixed versions). Actions pinned to commits (see packaging). `VIDEOLORE_REQUIRE_FFMPEG=1` makes missing ffmpeg fail, not skip; `VIDEOLORE_REQUIRE_STT=1` the same for the speech model and fixtures (2.0.3). The bundled ffmpeg smoke test also checks `lut3d` (1.6.0).
- CI installs `libasound2-dev` and `libclang-dev` on Linux when missing (`cmake` is on the image); espeak-ng is not installed (2.0.4).
- Cache key of whisper.cpp builds changed (1.x) so that old builds are not reused; three speech models cached for the accuracy test.
- Feature coverage scans `src`, `tests` and `player`. Notices: `scripts/notices.sh --check` checks the editor's and the player's notices; `scripts/version.sh set` runs it as a reminder.
- Tests with `ffmpeg` use the pinned bundled build (`packaging/ffmpeg.lock`, SHA-256 checked by `scripts/fetch-ffmpeg.sh`, cached under the lock's hash), not the runner's package (QG-07, QG-08, 2.0.2). Rejected: an apt pin (Ubuntu's ffmpeg is older than 9.0.2), building ffmpeg from source (too slow), wider limits on the runner's ffmpeg (hides version changes). The integration tests compare the found ffmpeg version with the pinned one and fail on a difference.

## CI time (1.6.1, QG-06)
- **Measured** (public GitHub API, runs of 2026-11-14 to 2026-11-19): the push of 1.6.0 took 28 min to its last executable; its check job 25.2 min, of which the system-packages step 20.0 min (other runs 0.2 to 1.5 min, once 5.4 min); the rest of the check job about 5.2 min (clippy 0.3 min, instrumented tests 2.3 min, crate notices 1.1 min, cargo-about compiled from source then).
- **Steps taken:** install only the packages the image lacks (`cmake` and `pkg-config` are on the image); `libasound2-dev` and `libclang-dev` with `--no-install-recommends`, after `apt-get update` with retries and a 30 s HTTP timeout; check job `timeout-minutes`; cargo-about as a prebuilt binary through the pinned install action (about 1 min saved); the speech test reads the committed `tests/fixtures/align/speech.mkv` (2.0.4, QG-14). Unchanged: every check, the tests, coverage thresholds, executables and the MSRV build; no untested binary is built on a push; no third-party apt cache action (supply chain).
- **Estimate (not measured on the new workflow):** check job 4.4 to 4.7 min (about 80 % below 25.2 min); push to last executable about 7 min instead of 28. The 30 % target of QG-06 is an assumption; the first run is reported against both baselines.

## Assets and build script
- Four splash files are compiled in with `include_bytes!` and `include_str!` in `src/splash.rs` (paths from `CARGO_MANIFEST_DIR`): `assets/splash/splash.gif`, `splash-player.gif`, `summary.txt`, `summary-player.txt` (1.6.2). Fixed names: a version bump needs no new file; the summaries are rewritten per version (rule 4f). 1.6.3 removed the per-version picture files.
- `build.rs` (2.0.4, QG-17): a missing or empty summary stops the build with a message naming the program and the file (a heading and at most five lines); re-runs on change (`cargo:rerun-if-changed`).
- Each splash picture (88 KB) stays in the shared library; the linker is assumed to drop the unused one (not measured).
- Git: the picture and summaries are committed with the code; `sources.zip` takes untracked non-ignored files.

## Tests by area
- **Test media and speed:** media generated once and reused, as small as each test allows; test ffmpeg as above; test-suite memory cap (below); fixtures checked by SHA-256 (speech fixture, reference clip; ADR-035 for test speed).
- **Time-dependent gates (QG-09, 2.0.2):** logic tested with injected limits and clocks; hardware probe limits (1 s, 4 s retry, 10 s total, REQ-061) are a value type with pinned defaults; wall-clock product gates (200 ms interface frame of REQ-104, camera stills, start-up and load) pass when the best of up to five samples is under the limit; every sample printed when all fail; waits for results (preview frames, hardware checks) are harness timeouts of 120 s. Rejected: raising product limits, ignoring timing tests, a multiplier for slow machines.
- **Splash (REQ-154, 2.0.2):** the summary starts at 15 px and shrinks to 7 px until it fits the popup above Close; never scrolls or clips silently; tests at the chosen size and at 7 px (replaces the 1.6.2 rule 14 to 10 px in 0.5 px steps; [splash](architecture-ui.md#splash-160-162-204)).
- **360 camera timing (QG-07):** figures of [360 video](architecture-360.md#performance-targets) measured with ffmpeg 6.1.1; the 2.0.2 run repeated them on 9.0.2 (median of three); targets 400 ms and 700 ms for stills not changed.
- **Bundled ffmpeg 9.x checks (QG-07):** frame effects `v15.rs`; LUT `v16.rs`, format 8 `v20.rs`; spherical marker `v126.rs`, `v13.rs` (ffprobe); Clean audio and MP3 `v14.rs`, encoder presence `v20.rs`; paused frame at a fade (REQ-144) `v20.rs`; format 8 refusal by 1.5.x: manual (no 1.5.x code in the tree).
- **2.0.4 (QG-14 to QG-18):** speech tests on the committed fixture (360 test sphere 1024 x 512); pointer tests for colour picker and Export dropdowns (`ui/text_ux_tests.rs`), keyframe bar (`ui/clarity_ux_tests.rs`); golden arguments of formats 4 and 5 (`tests/fixtures/project-v4-1.2.6.vlproj`, `project-v5-1.3.0.vlproj`, `export-argv-v4-v5-in-2.0.4.txt`, made from hand-made projects); large-picture memory test; the notices check for bundled and test fonts; manual items 31 to 35 in [business verification](business-verification.md).
- **Speech accuracy (2.0.3):** strict test runs without skip in CI; fixtures checked by SHA-256; not regenerated by the tests.
- **Overlay media (2.3.0):** `tests/ffmpeg_integration/v23overlap.rs` with `tests/fixtures/overlay/alpha.{png,gif,mov}`; Enlarge word and other 2.3.0 gates per [subtitles](architecture-subtitles.md).

## Test-suite memory cap (1.6.2, development machine)
- Every build and test run on the 15 GB development machine is capped: `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 <command>` (e.g. `… cargo test --locked --workspace`); child processes (rustc, test binaries, ffmpeg) stay in the scope; coverage too. A run killed by the cap is a failure reported with the memory cause; the cap is raised only with a written reason. CI runners are not capped by this rule. The 8 GB value is an assumption to check on the first full run.

## Risks
- Pinned ffmpeg downloads add CI time and depend on the download site (cache by lock hash).
- Integration tests run on Linux only with the pinned build; Windows and macOS builds of 9.0.2 are not tested by the suite.
- The 1.6.0 format 8 file is written by hand from the format description (not generated); the real file is checked by hand.
- The best-of-five rule may hide an intermittent slowdown; printed samples and the manual load check show it.
- The check job's apt step depends on the mirror (fail fast and retry).
