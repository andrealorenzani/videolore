# Architecture: dependencies

Part of [Architecture](architecture.md).

Exact versions are in `Cargo.toml` and `Cargo.lock`, which are the source of truth; the versions below are those reviewed for 0.3.0. All crates are kept on their latest releases and all licences are compatible with GPL-3.0 (checked in CI by `cargo-deny`).

## Build features
| Feature | Default | What it adds |
|---|---|---|
| `audio` | on | Preview sound through `cpal`. Without it the preview is silent. |
| `stt` | on | Speech-to-text (`whisper-rs`) and the model downloader (`ureq`). Needs `cmake`, a C++ compiler and libclang to build. |

`cargo build --no-default-features` gives the minimal build (no sound, no speech-to-text; subtitle editing and burn-in still work).

## Rust crates
No new crate was added in 1.5.0 (frame effects use the existing `egui` colour picker and core `ffmpeg` filters).

2.0.1: one development dependency, `syn` 2 (features `full`, `visit`; MIT or Apache-2.0), used only by `tests/module_cycles.rs` to read the import paths of `src/`. It is already in the lock file through the derive crates, so no new package is downloaded. No runtime dependency is added.

| Crate | Version | Licence | Why |
|---|---|---|---|
| `eframe` / `egui` | 0.37 | MIT/Apache-2.0 | Native GUI. |
| `cpal` (feature `audio`) | 0.18 | Apache-2.0 | Audio output for preview. |
| `rfd` | 0.18 | MIT | Native open/save dialogs. |
| `serde`, `serde_json` | 1.0 | MIT/Apache-2.0 | Projects, settings, `ffprobe` output. |
| `thiserror` | 2.0 | MIT/Apache-2.0 | Error types. |
| `directories` | 6.0 | MIT/Apache-2.0 | Platform folders for settings, preview-copy cache, work folders and speech models. |
| `tempfile` | 3.27 | MIT/Apache-2.0 | Random-named private folders and files (since 0.3.0 created inside `videolore`'s own work folder). |
| `sha2` | 0.11.1 | MIT/Apache-2.0 | Proxy cache names and speech model verification (uses the processor's SHA instructions where available); also used by tests to prove sources are never modified and to name cached test media. |
| `sysinfo` | 0.40.3 | MIT | Machine memory and physical cores for the recommended settings and the proxy rule, and memory measurement in tests. Only the "system" part is enabled. Replacing it would need platform-specific unsafe code. |
| `whisper-rs` (feature `stt`) | 0.16.0 | Unlicense (whisper.cpp: MIT) | Local speech-to-text. |
| `ureq` (feature `stt`) | 3.5.2 | MIT/Apache-2.0 | HTTPS download of the speech models. Uses `rustls` (with `ring` and `webpki-roots`), so no system OpenSSL. Chosen over `reqwest`, which needs an async runtime. |
| `objc2`, `objc2-foundation` (macOS only) | 0.6.5 / 0.3.2 | MIT | Receive "open document" events from Finder. |

**0.3.0 review.** Every crate was checked for speed, maintenance and latest version: none is dropped and none is added. Machine detection uses the Rust standard library (cores, processor features) plus `sysinfo`; crash-safe locking of work folders uses the standard library's file locks. `eframe` keeps its OpenGL (`glow`) renderer; the `wgpu` renderer was not chosen because it is heavier.

**0.4.0.** Hardware export adds no crate. The hardware encoders and the VMAF quality measurement used for manual checks are already in the bundled `ffmpeg` builds; graphics drivers and their libraries (libva, the NVIDIA and Intel runtimes) come from the user's system and are not bundled. See [hardware export](architecture-hardware-export.md).

**1.1.0.** The new interface adds no crate. It uses features already in egui 0.37 (menu bar, popups, windows, scroll areas, shortcut display, and the accessibility tree for the interface tests). Drag and drop comes from eframe through winit 0.30, which does not report dropped files in native Linux Wayland sessions. Docking libraries (`egui_dock`, `egui_tiles`) and native menus (`muda`) were considered and not chosen. The `egui_kittest` test library (0.37) is kept as a fallback for the interface tests, not added. See [user interface](architecture-ui.md).

**1.4.0.** No crate is added. Clean audio uses the bundled `ffmpeg`'s `afftdn` filter (a core filter, present in every build; checked on `ffmpeg` 6.1.1), and MP3 export uses `libmp3lame` from the same builds. The best fit of text uses an in-house estimate, so no font-metrics crate is needed (`ttf-parser` is already in the tree through `fontdb` and stays a transitive dependency only).

**1.3.0.** One crate is added: `fontdb` 0.24.0 (MIT, rust-version 1.71), with its default features, to list the fonts installed on the computer for text items (system folders, also those of fontconfig on Linux). Its tree adds `fontconfig-parser` 0.5.8 (MIT), `memmap2` 0.9 (MIT/Apache-2.0), `roxmltree` (MIT/Apache-2.0) and `ttf-parser` 0.25 (MIT/Apache-2.0), all checked by `cargo-deny` in CI. Rejected: `font-kit` (native code per system), `fontique` (larger tree), direct system font calls (three implementations). Text layout uses egui's own; no other crate is added. The MP4/MOV spherical marker of 360 exports is written in-house (`spherical` module, no crate), because ffmpeg 6.1.1 drops it on re-encode.

**1.2.0.** No crate is added. Dragging effects uses egui's built-in drag and drop; subtitle text is measured with egui's text layout and the bundled font. The SRT reader and writer, the word matching (sequence alignment and edit distance) and the table of language codes are small in-house modules; the crates `srtparse`, `subparse`, `strsim` and `isolang`, Rust MKV/MP4 readers, font crates (`ab_glyph`, `fontdue`) and separate forced-alignment tools were considered and not chosen. Subtitle tracks are read with the bundled `ffmpeg`. See [subtitle tracks](architecture-subtitle-tracks.md).

**1.6.0 .** One crate is added: `image` 0.25.10 with only its `gif` feature (`default-features = false`), to decode the splash image; it is already in the tree through `eframe`, and its GIF decoder adds the `gif` and `weezl` crates (MIT/Apache-2.0 or MIT, checked by `cargo-deny`). `ttf-parser` (already in the tree through `fontdb`) is declared as a development dependency only, to check that every icon exists in the font. Rejected: `egui-phosphor` 0.14.0 (requires egui 0.36, not the 0.37 in use), `egui-pop-phosphor` (fork), `egui_extras` (its image loaders are not needed; its `gif` feature gives no animation). The `.cube` reader and the icon constants are in-house. The player is a second member of the workspace that uses the same libraries without the speech feature, so it adds no crate. See [LUT](architecture-lut.md), [splash](architecture-splash.md), [icons](architecture-icons.md) and [player](architecture-player.md).

**1.6.2 .** No crate is added and no version changes. The splash code of both programs is one library module (`image` with its `gif` feature, `eframe` for drawing); the player uses it through the library it already depends on. Rejected: a copy of the splash code in the player (two text-fit rules would drift apart) and a new crate for the splash (a fifth workspace member for about 400 lines). See [splash](architecture-splash.md).

Indirect licences worth noting: `ring` (Apache-2.0 AND ISC), `webpki-roots` (CDLA-Permissive-2.0). Both are listed in the notices and allowed by the CI licence check.

**2.0.4.** New direct dependencies, all checked by cargo-deny: `objc2-app-kit` 0.3.2 (macOS only, minimal features; the one-call menu bar of REQ-232, no menu crate); the `accesskit` feature of eframe (accesskit_winit 0.34.2 on Windows, macOS and Linux, REQ-246). No other crate: the one-window lock uses `std::fs::File::try_lock`; the VTT and ASS writers, the `.3dl` and `.csp` readers, the spatial sound (ambisonics in Rust, a WAV writer by hand), the picture size (PNG header) and the MKV marker are in-house. `image` keeps only its `gif` feature (no `png`). The speech fixture is committed, so no speech synthesiser is installed in CI. The test font is named in both notice files (QG-18); the picture subtitle test stream is written by hand in `player/tests/ffmpeg.rs` (own work).

## Font crates for measuring text (2.0.3)
- `skrifa` 0.44 (MIT or Apache-2.0) reads a font's metrics and glyph widths; `harfrust` 0.12 (MIT) shapes the text, kerning included, as the subtitle path does. Both are already in the program's lock file (through the GUI library), so no new package is downloaded; they become direct dependencies. Their minimum Rust version (1.85) is below the policy of the latest stable toolchain.
- Not added: `rustybuzz`, `ttf-parser`, `fontdue` and `ab_glyph` (a second shaper or rasteriser for one measure).
- The whisper.cpp word-timing presets use the existing `whisper-rs` 0.16 binding; no new crate.

## Bundled with the installers (not Rust)
| Component | Licence | Why |
|---|---|---|
| `ffmpeg`, `ffprobe` (GPL builds: BtbN for Windows/Linux, Martin Riedl for macOS) | GPL-3.0-or-later (includes x264, x265: GPL-2.0-or-later; libass and others) | All media decoding, encoding and subtitle rendering. See [ffmpeg](architecture-ffmpeg.md). |
| Noto Sans Bold | SIL Open Font License 1.1 | Subtitle font. |
| Phosphor Icons, Regular weight (1.6.0) | MIT | Icons of the whole interface, bundled as one font file. See [icons](architecture-icons.md). |
| Noto Sans Arabic Bold and Noto Sans Devanagari Bold (2.0.4, committed in `assets/fonts`) | SIL Open Font License 1.1 | Subtitle and text fonts for these scripts; CJK is not bundled. |
| Splash pictures `assets/splash/splash.gif` (editor) and `assets/splash/splash-player.gif` (player) | Own work of the author, MIT (recorded in the notices since 1.6.3) | Splash screens of both programs. See [splash](architecture-splash.md). |

Downloaded after consent, not bundled: the whisper models of the three subtitle quality levels (`ggml-tiny-q5_1.bin`, `ggml-small-q5_1.bin`, `ggml-large-v3-turbo-q5_0.bin`, all MIT). See [subtitles](architecture-subtitles.md#subtitle-quality-levels-030).

## Tools used only to build, test or package
Not part of `videolore`; not linked.

| Tool | Version | Why |
|---|---|---|
| cargo-packager | 0.11.8 | Windows installer (NSIS) and macOS disk image. |
| appimagetool | pinned in `packaging/tools.lock` | Linux AppImage. |
| cargo-deny | 0.20.2 | Licence, advisory and banned-crate checks. |
| cargo-about | 0.9.3 | Generates the Rust crate notices. |
| cargo-llvm-cov | installed in CI | Line coverage; since 0.3.0 CI runs the tests once, under coverage. |
| `qemu-user` (optional) | system package | Running the tests on an emulated older processor to confirm the old-processor check. |
| `cmake`, libclang, C++ compiler | system | Building whisper.cpp. |
| `ffmpeg` (Ubuntu package) | system | Integration tests in CI. |
| `rsvg-convert` / ImageMagick | system | Generating icon files once (`scripts/icons.sh`), not needed to build. |

## Platform build requirements (from source)
- All: latest stable Rust (`rust-version` 1.100).
- Linux: `libasound2-dev` and `pkg-config` (feature `audio`); `cmake`, `libclang-dev` and a C++ compiler (feature `stt`).
- macOS: Xcode command-line tools and `cmake`.
- Windows: MSVC, `cmake` and LLVM (set `LIBCLANG_PATH` if libclang is not found).
- whisper.cpp (feature `stt`) is built with `GGML_NATIVE=OFF`, set in `.cargo/config.toml`: a fixed AVX2/FMA/F16C baseline on x86-64 instead of the build machine's CPU (since 0.2.1). See [subtitles](architecture-subtitles.md#cpu-requirements-since-021).
