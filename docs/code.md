# Code

videolore is a Rust desktop application (edition 2024, MSRV 1.100) built on egui/eframe 0.37. It never decodes video itself: every read and render is done by the `ffmpeg` and `ffprobe` programs, started as child processes (at most three at a time by default, up to six at "Maximum speed"). The installers ship their own copy of ffmpeg; speech recognition for subtitles is compiled in (whisper.cpp). The repository is a Cargo workspace with two more programs that share the editor's library and have no speech engine and no network code: **videolore player** (`player/`) and **videolore screen spy** (`screen-spy/`, the screen recorder).

| Document | Contents |
|---|---|
| [code-index.md](code-index.md) | Where each feature is: modules, main types and functions, test files and requirement IDs |
| [code-architecture.md](code-architecture.md) | Layers, module dependency graph, import → preview → export, a command's path, threads and channels (mermaid) |
| [code-modules.md](code-modules.md) | Main patterns, conventions, machine recommendation and "How hard videolore works" |
| [code-testing.md](code-testing.md) | Kinds of tests, golden output, integration test files, coverage, CI jobs, suite size and time |
| [code-effects.md](code-effects.md) | Effects on a clip or the whole video, clip volume, edge fades, the video's own subtitles and word matching, subtitle positions, SRT export, ruler, project format 3 (1.2.0, 1.6.1) |
| [code-360.md](code-360.md) | Boxes and corner handles, text size, align and reset, 360° video: camera maths, ffmpeg camera commands, preview copies, project format 4 (1.2.6) |
| [code-text.md](code-text.md) | Text items and effects, one selection, texts fixed to the 360° sphere, the "360 video" export, project format 5 (1.3.0) |
| [code-readable-text.md](code-readable-text.md) | Text measured with the font, readability, fonts, project format 10, box handles, placement rules, recognition pieces (2.0.3) |
| [code-export.md](code-export.md) | The Export dialog, Clean audio, best-fit text boxes, project format 6 (1.4.0) |
| [code-frame.md](code-frame.md) | Change aspect, Scale and Move with keyframes, project format 7 (1.5.0) |
| [code-lut.md](code-lut.md) | Apply LUT: the `.cube` parser, the filter graph, missing files, project format 8 (1.6.0) |
| [code-features.md](code-features.md) | 2.0.4: subtitle files and fonts, gestures and autosave, hardware reach and decoding, speed, platforms, interface settings, player, 360 additions, effects, quality items |
| [code-layers.md](code-layers.md) | 2.1.0 to 2.3.0: large LUTs, text and subtitle styles, whole video as copies, the order of the effects (overlay stage, step builder), overlap media, project formats 12 to 14 |
| [code-audio-tracks.md](code-audio-tracks.md) | 3.1.0: audio tracks (model, popups, render, player), Enlarge word from its base (`word_layout`), As subtitle track, the blurred fill fix, the Overlap media row, project format 15 |
| [code-measurements.md](code-measurements.md) | Every measurement: blur, order spikes, overlap memory, LUT start times, Faster on integrated graphics, speech regions, spatial pre-pass |
| [code-splash-icons.md](code-splash-icons.md) | Splash screens and icons (Phosphor) |
| [code-player.md](code-player.md) | The workspace and videolore player |
| [code-screen-spy.md](code-screen-spy.md) | 3.1.2: videolore screen spy (modules, capture, sound, tests, limits) and the player's playlist |
| [code-ui.md](code-ui.md) | Menus, toolbar, sections, shortcuts, popups, markers, status bar, recent projects, drag and drop |
| [code-packaging.md](code-packaging.md) | Installers, bundled ffmpeg, notices, release assets, manual checklist per OS |
| [code-hardware.md](code-hardware.md) | "Export: Faster": hardware methods and arguments, the check, the output check and redo, failure memory |
| [code-hardware-qg05.md](code-hardware-qg05.md) | The real-hardware verification of "Export: Faster" (QG-05): checklist and results |

Read-only helper scripts that print code facts (module map, where a requirement is implemented and tested, where a symbol is defined, module dependencies): [scripts.md](scripts.md).

## How to add an effect or a command
- **A command:** a variant of `Command` (`src/app/command.rs`) and its undo name in `Command::undo_name` (a test fails on "Edit" for an undoable one); one arm in the dispatch area of its topic (`src/app/dispatch_<area>.rs`; the areas and their order are `DISPATCH_AREAS` in `src/app.rs`; an unknown command ends as "command not handled"); when it can be unavailable, `is_enabled` and `disabled_reason` (`src/app/enabled.rs`); the behaviour in the `app/` file of its topic, through `AppState::edit` for an undoable change. A menu or toolbar entry: one row of `ui::actions::ACTIONS`, a variant of `ActionId`, its arm in `ActionId::command` and `icon`.
- **An effect:** a variant of `EffectKind` and its row in `SPECS` (`src/app/effect_kind.rs`: name, undo name, popup, frame flag; `EffectKind::ALL` follows the table); its icon in `ui::effects::effect_icon`; its arm in `AppState::apply_effect` (`src/app/effects.rs`); menu actions are one `ActionId::effect` row each (their command and icon follow). The Effects section, the timeline drop targets and the undo names read the table. If it draws on the picture: a `Row` of `effect_order.rs` (its file name in `NAMED`), its step in `render/steps.rs` and the filter in `render/graph.rs`; its Applied-effects row in `app/applied.rs`; project fields written only when not default (the golden argv tests in `render/golden_tests.rs` must not change for projects without it).
- **Tests:** a `// covers: REQ-NNN` test per requirement; a real-ffmpeg test in `tests/ffmpeg_integration/` when the output changes.

## Build, test, run
| What | Command |
|---|---|
| Build | `cargo build` (editor, the default member); `cargo build --workspace` also builds the player and the screen recorder (release: `cargo build --release --locked --workspace`) |
| Run | `cargo run [-- path/to/project.vlproj]`; the player: `cargo run -p videolore-player -- video.mp4`; the recorder: `cargo run -p videolore-screen-spy` |
| Tests | `cargo test --workspace` (integration fixtures generated once, reused from `target/tmp/fixtures`) |
| Format / lint | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` (CI also with `--no-default-features` and `--all-features`) |
| Line coverage (≥ 85%) | `scripts/coverage.sh --fail-under 85` (needs `cargo-llvm-cov` and `rustup component add llvm-tools-preview`) |
| Feature coverage (100%) | `scripts/feature-coverage.sh` |
| CI failure annotations | `scripts/test-annotations.sh <saved cargo test output>` |
| Licences and advisories | `cargo deny check` (cargo-deny 0.20.2, `deny.toml`) |
| Rust crate notices | `scripts/notices.sh` (rewrites `rust-crates.txt`, `rust-crates-player.txt`, `rust-crates-screen-spy.txt`) / `--check` (CI); needs `cargo install --locked cargo-about --version 0.9.3 --features cli`; rerun after any `Cargo.lock` change |
| Icons | `scripts/icons.sh` (after changing `assets/icon/videolore.svg`) |
| Installers | `scripts/fetch-ffmpeg.sh <target> <dir>`, `scripts/package.sh <target> [player\|screen-spy]` ([code-packaging.md](code-packaging.md)) |

### System requirements for building
- **Linux:** `sudo apt install libasound2-dev libxcb-randr0-dev pkg-config cmake libclang-dev` (ALSA for the preview sound; XCB RandR for the recorder's screen list; cmake, a C++ compiler and libclang for whisper.cpp).
- **macOS:** Xcode command line tools and cmake. **Windows:** MSVC, cmake and LLVM (`LIBCLANG_PATH=C:\Program Files\LLVM\bin` if bindgen does not find libclang).
- **ffmpeg** (≥ 4.4 with the usual filters) on `PATH` for a source build and the integration tests; installed copies use the bundled one. The integration tests in CI use the pinned build of `packaging/ffmpeg.lock` (9.0.2).
- Cross-checking the macOS and Windows code from Linux: `rustup target add aarch64-apple-darwin x86_64-pc-windows-msvc`, then `cargo clippy --target <t> --no-default-features --all-targets -- -D warnings`.

### Settings and machine detection
At every start videolore reads the machine (cores, memory, OS, processor features for the speech engine) in a few milliseconds, without network or benchmark. Without a settings file it saves the recommended choices and shows a short message; on a different machine it recomputes them (or, when the user had changed them, keeps them and offers "Use recommended settings"). Settings of 0.1.x/0.2.x are upgraded; 0.3.x files load with Export = Best quality. Tables: [code-modules.md](code-modules.md); hardware export: [code-hardware.md](code-hardware.md).

### CPU baseline for speech recognition
`.cargo/config.toml` builds whisper.cpp for a fixed baseline (`GGML_NATIVE=OFF`): on x86-64 **AVX2, FMA, F16C (and BMI2)**, i.e. Intel Haswell (2013) / AMD Excavator or newer; NEON on Apple silicon. Older x86-64 processors get a plain message instead of the speech engine. whisper-rs-sys does not notice when these values change: then run `cargo clean -p whisper-rs-sys` (and `--release`) and bump `prefix-key` of the rust-cache steps in `.github/workflows/build.yml`; a test checks that the file and the workflow agree.

### Cargo features
| Feature | Default | What it adds |
|---|---|---|
| `audio` | yes | Sound in the preview (cpal). Without it the preview is silent. |
| `stt` | yes | Subtitle generation (whisper.cpp through whisper-rs) and the download of the speech files (ureq, HTTPS only, each with consent). |

`cargo build --no-default-features` is the minimal build: no cmake/libclang, no sound, "Generate subtitles" reports that it is not available (editing, styles, burn-in and a video's own subtitles still work).

### Finding ffmpeg
Order: the path chosen in Settings → Advanced (`settings.json`), the environment variables `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`, the copy next to the executable (or `../Resources` in a macOS app), then `PATH`. A bundled copy that does not work is reported as damaged ("reinstall videolore"), anything else gets install hints. The check runs in the background at start-up; import, preview and export are enabled when it finishes.

### Dependencies
The project follows the latest stable Rust; `rust-version` (**1.100**) is raised with the toolchain, and dependencies are kept on their newest releases (every requirement in `Cargo.toml` names the version in `Cargo.lock`).

| Crate | Version | Why |
|---|---|---|
| eframe / egui | 0.37 | Window and UI (glow renderer, accesskit, no default features) |
| rfd | 0.18 | Native file dialogs (XDG portal on Linux) |
| cpal | 0.18 | Preview sound (optional, `audio`); the recorder's two sources on Windows and macOS |
| serde, serde_json | 1 | Project and settings files |
| thiserror | 2 | Error types |
| directories | 6 | Config, cache and data folders |
| tempfile | 3 | Private work folders |
| sha2 | 0.11 | Cache keys, speech file checksums |
| sysinfo | 0.40 (`system`) | Memory, physical cores, process times |
| whisper-rs | 0.16 | Speech recognition (optional, `stt`) |
| ureq | 3.5 (`rustls`) | Speech file download (optional, `stt`) |
| image | 0.25 (`gif`) | Splash image |
| fontdb | 0.24 | Installed font families |
| skrifa, harfrust | 0.44, 0.12 | Text measured with the font (the versions egui's epaint uses) |
| objc2, objc2-foundation, objc2-app-kit | 0.6, 0.3, 0.3 | macOS only: Apple Events, the menu bar |
| winit | 0.30 | macOS only: switching off the default menu so Cmd+Q reaches the program |
| display-info | 0.5.10 | Recorder: the connected screens |
| jiff | 1.0 | Recorder: local date and time of the default file name |
| block2, objc2-core-audio, objc2-core-foundation, objc2-core-graphics | 0.6, 0.3 | Recorder, macOS only: the microphone question, the tap of the computer's sound, the screen permission |
| syn (dev) | 2 | `tests/module_cycles.rs` |
| Phosphor Icons | 2.1.2 (font, MIT) | The icons (`assets/icons`, not a crate) |
