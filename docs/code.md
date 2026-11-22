# Code

videolore is a Rust desktop application (edition 2024, MSRV 1.100) built on egui/eframe 0.37. It
never decodes video itself: every read and render is done by the `ffmpeg` and `ffprobe` programs,
started as child processes (at most three at a time by default, up to six at "Maximum speed"). The installers ship their own copy of
ffmpeg; speech recognition for subtitles is compiled in (whisper.cpp). Since 1.6.0 the repository is a Cargo
workspace with a second program, **videolore player** (`player/`, no speech engine and no network code), that shares the
editor's engine.

This page covers building and running; the details are in the sub-documents:

| Document | Contents |
|---|---|
| [code-index.md](code-index.md) | Where each feature is: modules, main types and functions, tests and requirement IDs |
| [code-architecture.md](code-architecture.md) | Module layers and dependency graph, the import → preview → export flow, threads and channels (mermaid diagrams) |
| [code-modules.md](code-modules.md) | Module map, main patterns, background jobs, conventions |
| [code-effects.md](code-effects.md) | Effects on a clip or the whole video (since 1.6.1 a copy on each clip, project format 9), clip volume, fades at the video's edges, the video's own subtitles and word matching, subtitle positions, SRT export, timeline ruler, project format 3 (all since 1.2.0) |
| [code-360.md](code-360.md) | Boxes and corner handles, text size, align and reset, 360° video: camera maths, how the camera reaches ffmpeg, spike results, preview copies, project format 4 (all since 1.2.6) |
| [code-text.md](code-text.md) | Text items and their effects, one selection for lines and texts, the Context Menu group, texts fixed to the 360° sphere, the "360 video" export and its spherical marker, project format 5 (all since 1.3.0) |
| [code-readable-text.md](code-readable-text.md) | The text size measured with the font (`measure`), readability, fonts and missing fonts, project format 10, text box handles, placement rules, subtitle recognition pieces and messages, the blur and paused-picture fixes (all 2.0.3); REQ-177 is not met |
| [code-2.0.4-batches-5-7.md](code-2.0.4-batches-5-7.md) | 2.0.4 so far: the render and app split, subtitle files (.vtt, .ass, separate track, import, re-time), Arabic and Devanagari fonts, project format 11, autosave and recovery, editing gestures, snap guides, whole-video copies, offer for new clips, keyframe copy and paste |
| [code-2.0.4-batches-8-11.md](code-2.0.4-batches-8-11.md) | 2.0.4, stages 4 to 7: the short output check, quieter hardware check (pause, busy or hanging, session memory), AV1 and VP9 presets, Quick Sync constant quality, hardware decoding in the preview (`hwdec`), the graphics card choice, old libva message, plain codec names, the 4096 px warning, the kept blurred background, queue priorities, CI and installer checks, one window (`instance`), the Linux launcher, the macOS menu bar |
| [code-2.0.4-batches-12-15.md](code-2.0.4-batches-12-15.md) | 2.0.4, stages 8 to 12: interface settings (window, sections, workspaces, toolbar, shortcuts, theme, text size, text table, accessibility), the player (settings, playlist, open in editor), roll and easing of the camera, LUT readers, custom aspect, blur level, splash option, cache limit, page image and policy |
| [code-export.md](code-export.md) | The Export dialog (names, formats, presets, only audio, session memory), Clean audio, text boxes with best-fit size, project format 6 (all since 1.4.0) |
| [code-frame.md](code-frame.md) | Change aspect, Scale and Move with keyframes: frame size, expressions and graph, popups and handles, project format 7 (all since 1.5.0) |
| [code-2.1.0.md](code-2.1.0.md) | 2.1.0: large LUTs (256 points, streamed, background load, Work copies), the Applied effects widget and the order of the effects (the overlay stage), shadows, word colours, Enlarge word, Typewriter, project format 12, spike results and what is left |
| [code-2.2.0.md](code-2.2.0.md) | 2.2.0: the whole video as copies on the clips, the 16 megapixel picture limit, the background preparation of a LUT, project format 13 and the text maximum size, the order of all effects (sound and picture groups, the step builder), the Applied effects column and rows cache, the measurements that did not meet their target (Faster on integrated graphics, speech regions), spike and blur measurements, what is not built |
| [code-2.3.0.md](code-2.3.0.md) | 2.3.0: overlap media (model, the warning of an edit, render paths, lane and preview box), project format 14 and the conversion of 13, Volume and Change speed as effects, Crop removed, Change aspect of a clip, Add media, the start-screen choices, the Enlarge word fix, measurements, tests |
| [code-lut.md](code-lut.md) | Apply LUT: the `.cube` parser, the filter graph, missing files, project format 8 (1.6.0) |
| [code-splash-icons.md](code-splash-icons.md) | The splash screen (editor and player) and the icons (Phosphor), with the list of controls and their icons (1.6.0, 1.6.2) |
| [code-player.md](code-player.md) | The Cargo workspace, videolore player (open rules, playback, 360, subtitles, full screen) and its packaging (1.6.0) |
| [code-ui.md](code-ui.md) | The window: menus, toolbar, resizable sections, shortcuts, popups, timeline markers, status bar, recent projects, drag and drop |
| [code-testing.md](code-testing.md) | Unit, UI and integration tests, coverage, CI jobs |
| [code-packaging.md](code-packaging.md) | Installers, bundled ffmpeg, notices, release assets, manual checklist per OS, known limitations |
| [code-hardware.md](code-hardware.md) | "Export: Faster": video hardware methods and arguments, the hardware check, the output check and redo, failure memory, QG-05 checklist and results |
| [code-hardware-qg05.md](code-hardware-qg05.md) | The manual real-hardware verification of "Export: Faster" (QG-05): checklist and results |

## Build, test, run

| What | Command |
|---|---|
| Build | `cargo build` (editor only, the default member); `cargo build --workspace` also builds videolore player (release: `cargo build --release --locked --workspace`) |
| Run | `cargo run` (or `cargo run -- path/to/project.vlproj`); the player: `cargo run -p videolore-player -- video.mp4` |
| Tests | `cargo test --workspace` (integration fixtures are generated once and reused from `target/tmp/fixtures`) |
| Format / lint | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` (CI also lints with `--no-default-features` and `--all-features`) |
| Line coverage (>= 85%) | `scripts/coverage.sh --fail-under 85` over the whole workspace (needs `cargo-llvm-cov` and `rustup component add llvm-tools-preview`); CI runs the tests once, instrumented, with `cargo llvm-cov --workspace --fail-under-lines 85` |
| Feature coverage (100%) | `scripts/feature-coverage.sh` |
| CI failure annotations | `scripts/test-annotations.sh <saved cargo test output>` (used by CI; prints `::error` lines) |
| Licences and advisories | `cargo deny check` (cargo-deny 0.20.2, config in `deny.toml`) |
| Rust crate notices | `scripts/notices.sh` (rewrites `rust-crates.txt` and `rust-crates-player.txt`) / `scripts/notices.sh --check` (CI); needs `cargo install --locked cargo-about --version 0.9.3 --features cli` |
| Icons | `scripts/icons.sh` (only after changing `assets/icon/videolore.svg`) |
| Installers | `scripts/fetch-ffmpeg.sh <target> <dir>` and `scripts/package.sh <target> [player]`, see [code-packaging.md](code-packaging.md) |

### System requirements for building

- **Linux:** ALSA headers for the preview audio, plus cmake, a C++ compiler and libclang for
  whisper.cpp: `sudo apt install libasound2-dev pkg-config cmake libclang-dev`.
- **macOS:** Xcode command line tools and cmake.
- **Windows:** MSVC, cmake and LLVM (set `LIBCLANG_PATH=C:\Program Files\LLVM\bin` if bindgen
  does not find libclang).
- **ffmpeg** (any build >= 4.4 with the usual filters) on `PATH` to run a source build and the
  integration tests; installed copies use the bundled one.

### Settings and machine detection

At every start videolore reads the machine (logical and physical cores, memory, OS, processor
family and the processor features the speech engine needs) in a few milliseconds, without
network or benchmark. Without a settings file it saves the recommended choices for that machine
and shows a short message; a later start on a different machine recomputes them (or, when the
user had changed them, keeps them and offers "Use recommended settings"). Settings files of
0.1.x/0.2.x are upgraded (proxies off becomes Preview quality "Original"; the ffmpeg choice is
kept); 0.3.x files load with Export = Best quality. Details and the exact tables are in
[code-modules.md](code-modules.md); the hardware export is in [code-hardware.md](code-hardware.md).

### CPU baseline for speech recognition

`.cargo/config.toml` builds whisper.cpp for a fixed baseline instead of the build machine's CPU
(`GGML_NATIVE=OFF`): on x86-64 it needs **AVX2, FMA, F16C (and BMI2)**, i.e. Intel Haswell (2013)
/ AMD Excavator or newer; on Apple silicon it uses NEON. On older x86-64 processors the app
checks the CPU first and shows a plain message instead of starting the speech engine. The same flags apply to local, CI and
release builds. whisper-rs-sys does not notice when these values change: after editing them run
`cargo clean -p whisper-rs-sys` (and `--release`) and bump `prefix-key` of the rust-cache steps in
`.github/workflows/build.yml`; a test checks that the file and the workflow agree.

### Cargo features

| Feature | Default | What it adds |
|---|---|---|
| `audio` | yes | Sound output in the preview (cpal). Without it the preview is silent. |
| `stt` | yes | Subtitle generation: whisper.cpp through whisper-rs, and the download of the speech recognition files (ureq, HTTPS only, each with consent). |

`cargo build --no-default-features` is the minimal build: no cmake/libclang needed, no sound,
and "Generate subtitles" reports that it is not available (subtitle editing, styles and burn-in
still work; a video's own subtitles can be used, with their words spread evenly).

### Finding ffmpeg

Order: the path chosen in Settings → Advanced (saved in `settings.json` in the user's config folder), the
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
| directories | 6 | Config (settings), cache (proxies, work folders) and data (speech files) folders |
| tempfile | 3 | Private work folders for background jobs |
| sha2 | 0.11 | Proxy cache keys, speech file checksums |
| sysinfo | 0.40 (`system` only) | Installed memory and physical cores (machine detection), own memory use (tests) |
| whisper-rs | 0.16 | Speech recognition (optional, `stt`) |
| ureq | 3.5 (`rustls` only) | Speech model download (optional, `stt`) |
| objc2, objc2-foundation | 0.6, 0.3 | macOS only: opening double-clicked projects |
| image | 0.25 (`gif` only) | Decoding the splash image |
| Phosphor Icons | 2.1.2 (font file, MIT) | The icons (bundled in `assets/icons`, not a crate) |
| fontdb | 0.24 | Installed font families for the text font choice (default features: fontconfig directories, memory-mapped files) |
| winit | 0.30 | macOS only: switching off the default menu so Cmd+Q asks about unsaved changes |

If line coverage suddenly drops far below the usual ~95% after a toolchain change, delete
`target/llvm-cov-target` and run again: old test binaries left there are mixed into the report.
