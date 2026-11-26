# Contributing to videolore

Thank you for taking the time to help. This guide explains how to report a problem, how to propose a change and what a change must pass.

By taking part in this project you agree to follow the [code of conduct](CODE_OF_CONDUCT.md).

## Report a problem

- Questions and bugs: open an issue with the bug report form, see [SUPPORT.md](SUPPORT.md).
- Security problems: do not open a public issue; follow [SECURITY.md](SECURITY.md).

## Propose a change

1. For a large change or a new feature, open an issue first with the feature request form, so that the idea can be discussed before you write code.
2. Fork the repository and create a branch for your change.
3. Make the change, with tests that cover the changed behaviour.
4. Run the local checks listed below.
5. Open a pull request. The pull request template lists what a change must contain.

A pull request is reviewed by the maintainer, who may ask for changes. Acceptance is not guaranteed.

## Build prerequisites

- **Rust**: 1.100 or newer (`rust-version` in `Cargo.toml`); the project follows the latest stable Rust, so run `rustup update`. Install the components `rustfmt`, `clippy` and `llvm-tools-preview`.
- **Linux system packages**: `sudo apt install libasound2-dev pkg-config cmake libclang-dev` (ALSA headers for sound, cmake and libclang for the whisper.cpp subtitle build), plus `libxcb-randr0-dev` (XCB RandR, needed by the screen recorder) and a C++ compiler such as `g++` (needed by the whisper.cpp build). To build only the editor without sound and subtitles, use `cargo build --no-default-features`: it needs neither ALSA nor cmake, but the rest of the workspace still needs ALSA and libxcb-randr0-dev.
- **Coverage tool**: `cargo install cargo-llvm-cov`, needed by `scripts/coverage.sh`.
- **ffmpeg and ffprobe**: the tests use the ffmpeg 9.0.2 build pinned in `packaging/ffmpeg.lock`, and with `VIDEOLORE_REQUIRE_FFMPEG=1` the editor's integration tests fail if the version found differs from it. Download it with `scripts/fetch-ffmpeg.sh linux-x86_64 <folder>` and point `VIDEOLORE_FFMPEG` and `VIDEOLORE_FFPROBE` to the two files. A source build uses ffmpeg and ffprobe 4.4 or newer found on the system (`PATH`, or the two variables). Set `VIDEOLORE_REQUIRE_FFMPEG=1` to make a missing ffmpeg fail the tests instead of skipping them.
- **Screen recorder test on Linux**: the `real_x11` test records a real X11 screen with sound. It runs only with `VIDEOLORE_REQUIRE_CAPTURE=1` and is skipped otherwise, so its packages are optional. It needs the packages `xvfb`, `pulseaudio` and `pulseaudio-utils`, and a running X server (`DISPLAY`) and PulseAudio server. CI also sets `XDG_SESSION_TYPE=x11` and needs a microphone source that is not a monitor (it remaps one).

## Local checks

Run these commands from the root of the repository. They cover the three programs of the workspace (videolore, videolore-player and videolore-screen-spy). A change must pass all of them.

| Check | Command |
|---|---|
| Formatting | `cargo fmt --all --check` |
| Lints (warnings are errors) | `cargo clippy --workspace --all-targets -- -D warnings` |
| Tests | `cargo test --workspace` |
| Line coverage, at least 85% | `scripts/coverage.sh --fail-under 85` |
| Feature coverage, every implemented requirement has a test | `scripts/feature-coverage.sh` |

## Licence of contributions

videolore is licensed under the GNU General Public License version 3 (GPL-3.0), see [LICENSE](LICENSE). By submitting a contribution you agree that it is licensed under GPL-3.0 as well. No contributor agreement has to be signed.

Each new Rust source file starts with the licence header:

```rust
// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) <year> Andrea Lorenzani
```
