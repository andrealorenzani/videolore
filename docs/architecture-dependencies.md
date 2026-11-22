# Architecture: dependencies and tools

Part of [Architecture](architecture.md). Exact versions: `Cargo.toml` and `Cargo.lock` (source of truth). Versions below are the 2.3.0 values of `Cargo.toml` unless marked. Licences are checked by `cargo-deny` in CI; all are compatible with GPL-3.0. Policy: latest stable toolchain, MSRV field 1.100 (ADR-012), crates on their latest releases.

## Build features
| Feature | Default | Adds | Needs |
|---|---|---|---|
| `audio` | on | preview sound through `cpal` 0.18.2 (without it the preview is silent and wall-clocked) | Linux: `libasound2-dev`, `pkg-config` |
| `stt` | on | speech-to-text `whisper-rs` 0.16.0 and the model downloader `ureq` 3.5.2 (ADR-021, ADR-022) | `cmake`, a C++ compiler, libclang |

`cargo build --no-default-features` gives the minimal build (no sound, no speech; editing and burn-in still work). The player is the second workspace member (`player/`, `videolore-player`) built with `audio` and without `stt`.

## Rust crates (editor, runtime)
| Crate | Version | Licence | Why |
|---|---|---|---|
| `eframe` / `egui` | 0.37.0 (features `accesskit`, `glow`, `default_fonts`, `wayland`, `x11`) | MIT/Apache-2.0 | Native GUI, OpenGL (`glow`) renderer (`wgpu` heavier, not chosen); accessibility tree for the UI tests (ADR-045, feature ADR-197) |
| `rfd` | 0.18.0 (`xdg-portal`) | MIT | Native open and save dialogs |
| `serde`, `serde_json` | 1.0.229, 1.0.154 | MIT/Apache-2.0 | Projects, settings, ffprobe output |
| `thiserror` | 2.0.21 | MIT/Apache-2.0 | Error types |
| `directories` | 6.0.0 | MIT/Apache-2.0 | Platform folders (settings, caches, work folders, speech models) |
| `tempfile` | 3.27.0 | MIT/Apache-2.0 | Random-named private files inside videolore's work folder |
| `sha2` | 0.11.1 | MIT/Apache-2.0 | Cache names, speech model and LUT content hashes, tests proving sources unchanged |
| `sysinfo` | 0.40.4 (`system` only) | MIT | Physical cores and memory for the recommended settings and the proxy rule; replacing it needs platform unsafe code |
| `fontdb` | 0.24.0 | MIT | Installed fonts for text items (system folders, fontconfig on Linux; 1.3.0; rejected `font-kit`, `fontique`, direct calls) |
| `skrifa` | 0.44.0 | MIT/Apache-2.0 | Font metrics and glyph widths for measured text fit (2.0.3) |
| `harfrust` | 0.12.0 | MIT | Text shaping with kerning, as the subtitle path (2.0.3; not `rustybuzz`) |
| `image` | 0.25.10 (`default-features = false`, feature `gif` only) | MIT/Apache-2.0 | Splash GIF (1.6.0); no `png` feature (PNG header read by hand) |
| `objc2`, `objc2-foundation`, `objc2-app-kit` | 0.6.5, 0.3.2, 0.3.2 (macOS only) | MIT | Finder "open document" events (1.x) and the one-call menu bar (2.0.4, ADR-183) |
| `winit` | 0.30.13 (macOS, `default-features = false`) | Apache-2.0/MIT | Switching off winit's default macOS menu so Cmd+Q reaches the app (2.0.4) |

Dev-dependencies include `syn` 2 (features `full`, `visit`, MIT/Apache-2.0), used only by `tests/module_cycles.rs` to read the import paths of `src/` (2.0.1; already in the lock). Test-only font helper `ttf-parser` (1.6.0) checks that every icon exists in the font.

### Decisions by version
- **0.3.0 review:** no crate dropped or added. Machine detection uses the standard library plus `sysinfo`; work-folder locks use the standard library's file locks (`File::try_lock`, 2.0.4 one-window lock).
- **1.1.0:** no crate; egui menus, popups, windows, scroll areas, shortcut display. Drag and drop from eframe through winit 0.30 (no dropped files in native Linux Wayland sessions). Rejected: `egui_dock`, `egui_tiles`, `muda`; `egui_kittest` kept as a fallback for interface tests.
- **1.2.0:** no crate; egui drag and drop for effects; SRT reader and writer, word matching (sequence alignment, edit distance) and language-code table in-house. Rejected: `srtparse`, `subparse`, `strsim`, `isolang`, Rust MKV/MP4 readers, `ab_glyph`, `fontdue`, forced-alignment tools.
- **1.3.0:** `fontdb` added with `fontconfig-parser` 0.5.8, `memmap2` 0.9, `roxmltree`, `ttf-parser` 0.25 (all MIT or Apache-2.0). The 360 spherical marker is in-house (ffmpeg 6.1.1 drops it on re-encode).
- **1.4.0:** no crate. Clean audio uses `afftdn`, MP3 uses `libmp3lame`, both from the bundled ffmpeg; text best fit in-house (`ttf-parser` only transitive).
- **1.5.0:** no crate (egui colour picker, core ffmpeg filters).
- **1.6.0:** `image` with `gif` only (adds `gif`, `weezl`). Dev `ttf-parser`. Rejected: `egui-phosphor` 0.14.0 (needs egui 0.36), `egui-pop-phosphor` (fork), `egui_extras` (no animation with its gif feature). The `.cube` reader and icon constants are in-house. The player shares the libraries.
- **1.6.2:** no crate or version change; the splash code is one library module used by both programs (rejected: a copy in the player; a new crate).
- **2.0.1:** dev `syn` 2 only.
- **2.0.3:** `skrifa` and `harfrust` become direct dependencies (already in the lock through the GUI library; their Rust minimum 1.85 is below the latest stable policy). Rejected: `rustybuzz`, `ttf-parser`, `fontdue`, `ab_glyph`. The whisper word-timing presets use the existing `whisper-rs`.
- **2.0.4:** `objc2-app-kit` (macOS, minimal features; REQ-232), eframe `accesskit` feature (accesskit_winit 0.34.2 on Windows, macOS, Linux; REQ-246). In-house: VTT and ASS writers, `.3dl` and `.csp` readers, spatial sound (ambisonics, WAV writer), PNG header size, MKV marker, one-window lock. `image` keeps only `gif`.
- **2.1.0 to 2.3.0:** no new crate. LUT streaming, overlay alpha and the Enlarge word fix use in-house code and the bundled ffmpeg (`lut3d`, `overlay eof_action`, `alphamerge`, `subtitles`).
- **Accepted transitive licences:** `ring` (Apache-2.0 AND ISC) and `webpki-roots` (CDLA-Permissive-2.0), listed in the notices and allowed by the CI licence list.

## Bundled with the installers (not Rust)
| Component | Licence | Why |
|---|---|---|
| `ffmpeg`, `ffprobe` (GPL builds: BtbN for Windows and Linux, Martin Riedl for macOS) | GPL (x264, x265 GPL 2 or later; libass and others) | All media decoding, encoding, subtitle and LUT rendering; see [engine](architecture-engine.md#bundled-ffmpeg) |
| Noto Sans Bold | SIL OFL 1.1 | Subtitle font (about 0.5 MB, `assets/fonts/`) |
| Noto Sans Arabic Bold, Noto Sans Devanagari Bold (2.0.4) | SIL OFL 1.1 | Text and subtitle fonts for these scripts; CJK is not bundled |
| Phosphor Icons, Regular (1.6.0) | MIT | Interface icons as one font file; see [UI](architecture-ui.md#icons-160-req-142-143) |
| Splash pictures `assets/splash/splash.gif`, `splash-player.gif` | author's own work, MIT (notices since 1.6.3) | Splash screens of both programs |

Downloaded after consent, not bundled: the whisper models of the three quality levels (`ggml-tiny-q5_1.bin`, `ggml-small-q5_1.bin`, `ggml-large-v3-turbo-q5_0.bin`, MIT; see [subtitles](architecture-subtitles.md#quality-levels-and-models-030)).

Not bundled, from the system: graphics drivers and libraries (libva, NVIDIA and Intel runtimes); Chinese, Japanese and Korean fonts when installed.

## Tools used only to build, test or package
Not part of videolore; not linked.

| Tool | Version | Why |
|---|---|---|
| cargo-packager | 0.11.8 | Windows NSIS installer and macOS disk image |
| appimagetool | pinned in `packaging/tools.lock` | Linux AppImage |
| cargo-deny | 0.20.2 | Licence, advisory, banned-crate checks |
| cargo-about | 0.9.3 | Rust crate notices (pinned prebuilt binary since 1.6.1) |
| cargo-llvm-cov | in CI | Line coverage (one run under coverage, 0.3.0) |
| qemu-user (optional) | system | Tests on an emulated older processor (old-processor check) |
| cmake, libclang, C++ compiler | system | whisper.cpp build |
| ffmpeg (Ubuntu package) | system | Integration tests in CI (x264, x265, libass present) |
| rsvg-convert or ImageMagick | system | Icon files, once (`scripts/icons.sh`), not needed to build |

## Platform build requirements (from source)
- All: latest stable Rust (`rust-version` 1.100).
- Linux: `libasound2-dev`, `pkg-config` (`audio`); `cmake`, `libclang-dev`, C++ compiler (`stt`).
- macOS: Xcode command-line tools and `cmake`.
- Windows: MSVC, `cmake`, LLVM (set `LIBCLANG_PATH` if libclang is not found).
- whisper.cpp is built with `GGML_NATIVE=OFF` from `.cargo/config.toml`: a fixed AVX2, FMA, F16C baseline instead of the build machine's CPU (0.2.1; [subtitles](architecture-subtitles.md#cpu-requirements)). `SOURCE_DATE_EPOCH` must stay unset.
