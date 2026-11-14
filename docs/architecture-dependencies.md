# Architecture: dependencies

Part of [Architecture](architecture.md).

Exact versions are in `Cargo.toml` and `Cargo.lock`, which are the source of truth; the versions below are those chosen for 0.2.0. All crates are kept on their latest releases and all licences are compatible with GPL-3.0 (checked in CI by `cargo-deny`).

## Build features
| Feature | Default | What it adds |
|---|---|---|
| `audio` | on | Preview sound through `cpal`. Without it the preview is silent. |
| `stt` | on | Speech-to-text (`whisper-rs`) and the model downloader (`ureq`). Needs `cmake`, a C++ compiler and libclang to build. |

`cargo build --no-default-features` gives the minimal build (no sound, no speech-to-text; subtitle editing and burn-in still work).

## Rust crates
| Crate | Version | Licence | Why |
|---|---|---|---|
| `eframe` / `egui` | 0.37 | MIT/Apache-2.0 | Native GUI. |
| `cpal` (feature `audio`) | 0.18 | Apache-2.0 | Audio output for preview. |
| `rfd` | 0.18 | MIT | Native open/save dialogs. |
| `serde`, `serde_json` | 1.0 | MIT/Apache-2.0 | Projects, settings, `ffprobe` output. |
| `thiserror` | 2.0 | MIT/Apache-2.0 | Error types. |
| `directories` | 6.0 | MIT/Apache-2.0 | Platform folders for settings, proxy cache and speech model. |
| `tempfile` | 3.27 | MIT/Apache-2.0 | Private temporary folders (filter graphs, preview scripts, subtitle files, audio extraction). |
| `sha2` | 0.11.1 | MIT/Apache-2.0 | Proxy cache names and speech model verification; also used by tests to prove sources are never modified. Was test-only before 0.2.0. |
| `sysinfo` | 0.40.3 | MIT | Machine RAM for the proxy rule, and memory measurement in tests. Only the "system" part is enabled. |
| `whisper-rs` (feature `stt`) | 0.16.0 | Unlicense (whisper.cpp: MIT) | Local speech-to-text. |
| `ureq` (feature `stt`) | 3.5.2 | MIT/Apache-2.0 | HTTPS download of the speech model. Uses `rustls` (with `ring` and `webpki-roots`), so no system OpenSSL. Chosen over `reqwest`, which needs an async runtime. |
| `objc2`, `objc2-foundation` (macOS only) | 0.6.5 / 0.3.2 | MIT | Receive "open document" events from Finder. |

Indirect licences worth noting: `ring` (Apache-2.0 AND ISC), `webpki-roots` (CDLA-Permissive-2.0). Both are listed in the notices and allowed by the CI licence check.

## Bundled with the installers (not Rust)
| Component | Licence | Why |
|---|---|---|
| `ffmpeg`, `ffprobe` (GPL builds: BtbN for Windows/Linux, Martin Riedl for macOS) | GPL-3.0-or-later (includes x264, x265: GPL-2.0-or-later; libass and others) | All media decoding, encoding and subtitle rendering. See [ffmpeg](architecture-ffmpeg.md). |
| Noto Sans Bold | SIL Open Font License 1.1 | Subtitle font. |

Downloaded after consent, not bundled: the whisper "tiny" multilingual model (`ggml-tiny-q5_1.bin`, MIT). See [subtitles](architecture-subtitles.md).

## Tools used only to build, test or package
Not part of `videolore`; not linked.

| Tool | Version | Why |
|---|---|---|
| cargo-packager | 0.11.8 | Windows installer (NSIS) and macOS disk image. |
| appimagetool | pinned in `packaging/tools.lock` | Linux AppImage. |
| cargo-deny | 0.20.2 | Licence, advisory and banned-crate checks. |
| cargo-about | 0.9.3 | Generates the Rust crate notices. |
| `espeak-ng` | system package | Synthetic speech for the speech-to-text test. |
| `cmake`, libclang, C++ compiler | system | Building whisper.cpp. |
| `ffmpeg` (Ubuntu package) | system | Integration tests in CI. |
| `rsvg-convert` / ImageMagick | system | Generating icon files once (`scripts/icons.sh`), not needed to build. |

## Platform build requirements (from source)
- All: latest stable Rust (`rust-version` 1.100).
- Linux: `libasound2-dev` and `pkg-config` (feature `audio`); `cmake`, `libclang-dev` and a C++ compiler (feature `stt`).
- macOS: Xcode command-line tools and `cmake`.
- Windows: MSVC, `cmake` and LLVM (set `LIBCLANG_PATH` if libclang is not found).
- whisper.cpp (feature `stt`) is built with `GGML_NATIVE=OFF`, set in `.cargo/config.toml`: a fixed AVX2/FMA/F16C baseline on x86-64 instead of the build machine's CPU (since 0.2.1). See [subtitles](architecture-subtitles.md#cpu-requirements-since-021).
