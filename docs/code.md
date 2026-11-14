# Code

videolore is a Rust desktop application (edition 2024, MSRV 1.100) built on egui/eframe 0.37. It
never decodes video itself: every read and render is done by the `ffmpeg` and `ffprobe` programs,
started as child processes (at most three at a time). The installers ship their own copy of
ffmpeg; speech recognition for subtitles is compiled in (whisper.cpp).

This page covers building and running; the details are in the sub-documents:

| Document | Contents |
|---|---|
| [code-modules.md](code-modules.md) | Module map, main patterns, background jobs, conventions |
| [code-testing.md](code-testing.md) | Unit, UI and integration tests, coverage, CI jobs |
| [code-packaging.md](code-packaging.md) | Installers, bundled ffmpeg, notices, release assets, manual checklist per OS, known limitations |

## Build, test, run

| What | Command |
|---|---|
| Build | `cargo build` (release: `cargo build --release --locked`) |
| Run | `cargo run` (or `cargo run -- path/to/project.vlproj`) |
| Tests | `cargo test` |
| Format / lint | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (CI also lints with `--no-default-features` and `--all-features`) |
| Line coverage (>= 85%) | `scripts/coverage.sh --fail-under 85` (needs `cargo-llvm-cov` and `rustup component add llvm-tools-preview`) |
| Feature coverage (100%) | `scripts/feature-coverage.sh` |
| Licences and advisories | `cargo deny check` (cargo-deny 0.20.2, config in `deny.toml`) |
| Rust crate notices | `scripts/notices.sh` (rewrite) / `scripts/notices.sh --check` (CI); needs `cargo install --locked cargo-about --version 0.9.3 --features cli` |
| Icons | `scripts/icons.sh` (only after changing `assets/icon/videolore.svg`) |
| Installers | `scripts/fetch-ffmpeg.sh <target> <dir>` and `scripts/package.sh <target>`, see [code-packaging.md](code-packaging.md) |

### System requirements for building

- **Linux:** ALSA headers for the preview audio, plus cmake, a C++ compiler and libclang for
  whisper.cpp: `sudo apt install libasound2-dev pkg-config cmake libclang-dev`.
- **macOS:** Xcode command line tools and cmake.
- **Windows:** MSVC, cmake and LLVM (set `LIBCLANG_PATH=C:\Program Files\LLVM\bin` if bindgen
  does not find libclang).
- **ffmpeg** (any build >= 4.4 with the usual filters) on `PATH` to run a source build and the
  integration tests; installed copies use the bundled one.

### CPU baseline for speech recognition

`.cargo/config.toml` builds whisper.cpp for a fixed baseline instead of the build machine's CPU
(`GGML_NATIVE=OFF`): on x86-64 it needs **AVX2, FMA, F16C (and BMI2)**, i.e. Intel Haswell (2013)
/ AMD Excavator or newer; on Apple silicon it uses NEON. The same flags apply to local, CI and
release builds. whisper-rs-sys does not notice when these values change: after editing them run
`cargo clean -p whisper-rs-sys` (and `--release`) and bump `prefix-key` of the rust-cache steps in
`.github/workflows/build.yml`; a test checks that the file and the workflow agree.

### Cargo features

| Feature | Default | What it adds |
|---|---|---|
| `audio` | yes | Sound output in the preview (cpal). Without it the preview is silent. |
| `stt` | yes | Subtitle generation: whisper.cpp through whisper-rs, and the one-time model download (ureq, HTTPS only). |

`cargo build --no-default-features` is the minimal build: no cmake/libclang needed, no sound,
and "Generate subtitles" reports that it is not available (subtitle editing, styles and burn-in
still work).

### Finding ffmpeg

Order: the path chosen in the app (saved in `settings.json` in the user's config folder), the
environment variables `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`, the copy next to the executable
(or in `../Resources` inside a macOS app), then `PATH`. A bundled copy that does not work is
reported as damaged ("reinstall videolore"), anything else gets install hints. The check runs in
the background at start-up; import, preview and export are enabled when it finishes.

### Dependencies

The project follows the latest stable Rust; `rust-version` in `Cargo.toml` (currently **1.100**)
is raised together with the toolchain, and dependencies are kept on their newest releases.

| Crate | Version | Why |
|---|---|---|
| eframe / egui | 0.37 | Window and UI (glow renderer, no default features) |
| rfd | 0.18 | Native file dialogs (XDG portal on Linux) |
| cpal | 0.18 | Preview sound (optional, `audio`) |
| serde, serde_json | 1 | Project and settings files |
| thiserror | 2 | Error types |
| directories | 6 | Config, cache (proxies) and data (speech model) folders |
| tempfile | 3 | Private working folders for ffmpeg |
| sha2 | 0.11 | Proxy cache keys, speech model checksum |
| sysinfo | 0.40 (`system` only) | Installed memory (proxy rule), own memory use (tests) |
| whisper-rs | 0.16 | Speech recognition (optional, `stt`) |
| ureq | 3.5 (`rustls` only) | Speech model download (optional, `stt`) |
| objc2, objc2-foundation | 0.6, 0.3 | macOS only: opening double-clicked projects |
| winit | 0.30 | macOS only: switching off the default menu so Cmd+Q asks about unsaved changes |

If line coverage suddenly drops far below the usual ~95% after a toolchain change, delete
`target/llvm-cov-target` and run again: old test binaries left there are mixed into the report.
