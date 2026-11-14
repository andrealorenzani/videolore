# Architecture

Version 0.1.5. Rust (edition 2024), minimum Rust 1.100 (the project follows the latest stable Rust). License GPL-3.0.

## Overview
`videolore` is a desktop video editor with a native window (Linux, macOS, Windows). It does not decode or encode video itself: it drives the external `ffmpeg` and `ffprobe` programs, which the user installs separately. The editor keeps the edit (clips, cuts, fades) in memory, previews it, and exports it with a single `ffmpeg` run.

Implemented (since 0.1.3): requirements REQ-001..013 and REQ-021..025 (see [business](business.md)). Planned, release not yet assigned: background music REQ-019..020; subtitles and speech-to-text REQ-014..018.

## Components
| Module | Role |
|---|---|
| `time` | Exact time arithmetic and timestamp formatting (no floating point in cut math). |
| `media` | Information about a media file (streams, codecs, size, frame rate, duration) read from `ffprobe`. |
| `timeline` | The edit: clips, in/out points, order, cuts, fades, and the rules that keep it valid. |
| `history` | Undo and redo. |
| `project` | Save/load projects as versioned JSON; detects missing media. |
| `format` | Output formats, codecs and which encoder to use, with fallbacks. |
| `render` | Turns the timeline into an export plan and the exact `ffmpeg` arguments. |
| `ffmpeg` | Finds `ffmpeg`/`ffprobe`, checks version and features, runs them, reads progress, cancels. The only module that starts processes. |
| `export` | Runs an export in the background and writes the result safely. |
| `preview` | Decodes frames and audio for the preview player. |
| `ui` | Windows and panels (media, timeline, preview, export, settings). Kept thin. |
| `app` | Application state and wiring between the UI and background workers. |

Data flow: a user action changes the timeline (recorded for undo); the preview worker is asked for frames and sends them back; on export, `render` builds a plan, the export worker runs `ffmpeg` and reports progress to the UI. The UI should never wait on disk or on `ffmpeg`; in 0.1.3 two exceptions remain (import runs `ffprobe` on the UI thread, and each edit refreshes the still preview frame with `ffmpeg`), see [code](code.md) Known limitations.

## Main technical decisions
- **GUI: eframe/egui, latest release.** Pure Rust, single executable, runs on all three platforms. Alternatives considered: iced, slint, gtk4-rs, tauri.
- **Media: `ffmpeg`/`ffprobe` as external programs**, not linked libraries. No complex build, works with any `ffmpeg` 4.4 or newer. They are **not bundled**: the user installs them. Lookup order: path set in settings, then environment variables `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`, then next to the `videolore` executable, then `PATH`. At startup the version (>= 4.4), required filters and available encoders are checked; missing pieces are shown in the UI.
- **Preview audio: cpal.** If no audio device is available, preview continues silently.
- **Concurrency: standard threads and channels** (no async runtime). Long tasks can be cancelled.
- **Export strategy:**
  - *Stream copy* (fast, lossless) only when all clips share the same format, the output format matches, there are no effects (fades), and every cut is on a keyframe.
  - Otherwise *one re-encode pass* in a single `ffmpeg` run.
  - Output is written to a temporary file in the destination folder and renamed when complete; a failed or cancelled export leaves no partial file. Source files are never overwritten.
- **Encoders:** each output format has a preferred encoder list with fallbacks; formats with no available encoder are disabled with an explanation.
- **Fades:** default length is 0.5 s or half the clip, whichever is shorter; video and audio fades are aligned.
- **Speech-to-text (decided now, built in a later release):** local whisper.cpp through `whisper-rs`, behind an optional build feature `stt`. Default model: "tiny" multilingual (q5_1). The model is downloaded once only after the user agrees, verified with a SHA-256 checksum, and then used offline. No cloud service.

## Non-functional targets
- Window opens in under 1 s; file inspection runs in the background (not met in 0.1.3: import runs `ffprobe` on the UI thread, see [code](code.md) Known limitations).
- UI stays responsive (target 60 fps) during editing, preview and export.
- Preview seek under about 300 ms for 1080p video.
- Export progress updates at least twice per second; cancel takes effect within 1 s.
- Bounded memory for preview frames and `ffmpeg` logs.
- No partial outputs; project saves are atomic.
- Platforms: Linux x86_64, macOS Apple Silicon, Windows x86_64.
- Code coverage >= 85%: editing, project, format and export-planning logic is kept out of the UI so it can be tested.

## Security model
- Media files and project files are treated as untrusted.
- `ffmpeg` is started directly, never through a shell; file names are passed as separate arguments prefixed with `file:`, and only the `file` and `pipe` protocols are allowed, so crafted names or files cannot trigger network access or option injection.
- Project files are validated after loading (types, size, valid in/out points).
- The resolved `ffmpeg` path and version are shown in settings; `PATH` is searched last.
- No network access, except the optional, user-approved, checksum-verified speech model download (future speech-to-text feature). No telemetry, no secrets.
- `Cargo.lock` is committed and release builds use `--locked`.
- Dependencies and CI actions are kept on their latest releases, so security fixes arrive quickly.

## Rust toolchain policy
- The project follows the most recent stable Rust. The minimum supported version (`rust-version` in `Cargo.toml`) is raised to the current stable release, now 1.100 (previously 1.88). Building from source therefore needs an up-to-date Rust (for example via `rustup update`).
- All dependencies are kept on their latest releases.
- When a new Rust release brings new warnings or lints, the code is fixed; old Rust or crate versions are not pinned to avoid them.
- Why: in 0.1.5 the CI, running the latest stable Rust (1.100), failed on lints that Rust 1.88 does not have (an upcoming change to how untyped decimal numbers become 32-bit floats, and two code-style lints). Upgrading was chosen over freezing CI on 1.88, which would hide future problems and keep the GUI library on an old version.

## Build and CI
- CI (check job and Linux build job) installs `ffmpeg` and `libasound2-dev`. The check job sets `VIDEOLORE_REQUIRE_FFMPEG=1`, so tests that need `ffmpeg` fail instead of being skipped; elsewhere they are skipped when `ffmpeg` is missing.
- Building from source on Linux needs `libasound2-dev` (ALSA headers, for audio).
- CI builds with the latest stable Rust and, in a separate MSRV job, with exactly the Rust version in `rust-version` of `Cargo.toml` (1.100). GitHub Actions are kept on their latest major versions (`actions/checkout` on a Node 24 compatible release).
- Recommended, not yet done: `cargo-deny` for licence and advisory checks.

## Dependencies
Exact versions are in `Cargo.toml` and `Cargo.lock`; all crates are kept on their latest releases. All are compatible with GPL-3.0 (MIT and/or Apache-2.0).

| Crate | Why |
|---|---|
| `eframe` / `egui` | Native GUI. |
| `cpal` | Audio output for preview. |
| `rfd` | Native open/save dialogs. |
| `serde`, `serde_json` | Projects, settings, `ffprobe` output. |
| `thiserror` | Error types. |
| `directories` | Platform config folder for settings. |
| `tempfile` | Private temporary folders for the concat list, the filter graph and the preview scripts. |
| `sha2` (dev) | Checksums in the integration tests, to prove source files are never modified. |
| `whisper-rs` (planned, optional feature `stt`) | Local speech-to-text. |

External programs (not bundled): `ffmpeg` and `ffprobe` 4.4 or newer.

## Known risks
- `ffmpeg` builds differ (missing encoders/filters): handled by the startup check and fallbacks.
- Audio/video sync in preview: audio drives the clock, late video frames are dropped.
- egui releases often change its API: GUI code is kept thin so upgrades stay small.
- A new stable Rust can add lints that make CI fail (warnings are errors): the code is fixed promptly rather than staying on an older Rust.
- Speech-to-text needs a C++ toolchain: kept behind an optional feature.
