# Architecture

Version 0.2.0. Rust (edition 2024), minimum Rust 1.100 (the project follows the latest stable Rust). License GPL-3.0.

## Overview
`videolore` is a desktop video editor with a native window (Linux, macOS, Windows). It does not decode or encode video itself: it drives the `ffmpeg` and `ffprobe` programs, which are **bundled with the installers** since 0.2.0. The editor keeps the edit (video clips, cuts, fades, a music track, subtitles) in memory, previews it, and exports it with a single `ffmpeg` run.

Implemented: requirements REQ-001..044 (see [business](business.md)), including music (REQ-019, 020, 043), subtitles with offline speech-to-text (REQ-014..018, 044), save/unsaved-changes handling (REQ-026..029), the bundled `ffmpeg` (REQ-030, 031), responsiveness and memory work (REQ-032..039) and per-OS installers (REQ-040..042). All known limitations of 0.1.3 are addressed.

## Sub-documents
| Document | Contents |
|---|---|
| [architecture-ffmpeg.md](architecture-ffmpeg.md) | The bundled `ffmpeg`: which builds, where they live, how they are found, licences and source code. |
| [architecture-packaging.md](architecture-packaging.md) | Installers per OS, file association, icon, Linux menu entry, release pipeline. |
| [architecture-media-pipeline.md](architecture-media-pipeline.md) | Process limit and job queue, preview, export, music mix, proxies, memory limits. |
| [architecture-subtitles.md](architecture-subtitles.md) | Subtitle model, styles and burn-in, speech-to-text, model download. |
| [architecture-security.md](architecture-security.md) | Threat model and protections, supply chain. |
| [architecture-dependencies.md](architecture-dependencies.md) | Rust crates, build features, external tools and bundled components, with reasons. |

## Components
| Module | Role |
|---|---|
| `time` | Exact time arithmetic and timestamp formatting (no floating point in cut math). |
| `media` | Information about a media file read from `ffprobe`; status "analysing / ready / rejected"; video or audio-only. |
| `timeline` | The video track: clips, in/out points, order, cuts, fades, and the rules that keep it valid. |
| `music` | The music track: audio clips that never overlap; place, move, trim, split, delete. |
| `subtitles` | Subtitle lines stored against the source speech, and how they appear on the timeline. |
| `ass` | Writes the subtitle file (ASS format) used by `ffmpeg` for the chosen style. |
| `history` | Undo and redo of the whole edit (video, music, subtitles, volumes). |
| `project` | Save/load projects as versioned JSON (version 2, opens version 1); unsaved-changes tracking. |
| `format` | Output formats, codecs and which encoder to use, with fallbacks. |
| `render` | Turns the edit into an export plan and the exact `ffmpeg` arguments; also the preview graphs. |
| `ffmpeg` | Finds `ffmpeg`/`ffprobe` (bundled copy first), checks features, runs them, reads progress, cancels. |
| `jobs` | The job queue: at most 3 `ffmpeg`/`ffprobe` processes at a time, by priority. Every process goes through it. |
| `export` | Runs an export in the background and writes the result safely. |
| `preview` | Decodes frames and audio for the preview player. |
| `proxy` | Lower-resolution preview copies of large sources, and their cache. |
| `stt` | Speech-to-text: audio extraction, transcription, language detection, model download. |
| `machine` | CPU cores and memory of the running machine. |
| `notices` | The third-party notices shown in About. |
| `desktop` | Linux application-menu entry and file association; macOS "open document" events. |
| `ui` | Windows and panels. Kept thin. |
| `app` | Application state and wiring between the UI and background workers. |

## Data flow
- **Editing:** a user action changes the edit and is recorded for undo; the unsaved-changes marker is recomputed; if the picture at the playhead changed, one still frame is requested (bursts of edits are merged).
- **Import:** each file appears at once as "analysing"; `ffprobe` runs in the background; the item then shows name and duration or the reason it was rejected. A large source may then get a proxy in the background.
- **Export:** `render` builds a plan from the original files (never proxies); the export worker runs one `ffmpeg` and reports progress; the result is renamed into place only when complete.
- **Subtitles:** the audio of each source used on the timeline is extracted, transcribed on the machine, and turned into subtitle lines.
- **Opening a project by double-click:** on Windows and Linux the file path arrives on the command line; on macOS it arrives as a system event. Both go through the unsaved-changes prompt.

The UI never waits on `ffmpeg` or on disk, apart from saving or loading the project file itself (at most 16 MiB) and a few small files written at Linux startup.

## Main technical decisions
- **GUI: eframe/egui, latest release.** Pure Rust, single executable, all three platforms. Alternatives considered: iced, slint, gtk4-rs, tauri.
- **Media: `ffmpeg`/`ffprobe` as separate programs**, not linked libraries, now bundled. See [ffmpeg](architecture-ffmpeg.md).
- **Preview audio: cpal.** Without an audio device the preview continues silently.
- **Concurrency: standard threads and channels**, no async runtime. A job queue caps `ffmpeg` processes at 3. See [media pipeline](architecture-media-pipeline.md).
- **Export strategy:**
  - *Stream copy* (fast, lossless) only when all clips share the same format, the output format matches, there are no effects (fades, music, subtitles, video volume changes) and every cut is on a keyframe.
  - Otherwise *one re-encode pass* in a single `ffmpeg` run.
  - Output goes to a temporary file in the destination folder and is renamed when complete; a failed or cancelled export leaves no partial file. Source files are never overwritten.
- **Encoders:** each output format has a preferred encoder list with fallbacks; formats with no available encoder are disabled with an explanation.
- **Fades:** 0.5 s or half the clip, whichever is shorter; video and audio aligned.
- **Speech-to-text:** local whisper.cpp, included in release builds; the model is downloaded once after consent. See [subtitles](architecture-subtitles.md).
- **Projects:** format version 2; unsaved changes are detected by comparing the edit with the last saved one, so undoing back to the saved state clears the marker. See [Projects](#projects).
- **Installers:** unsigned in 0.2.0. See [packaging](architecture-packaging.md).

## Projects
- Version 2 adds the music track, the volume/mute settings and the subtitles (language, style, lines with word timings).
- Version 1 files (0.1.x) are converted in memory when opened and export exactly as before. Once saved they become version 2 and can no longer be opened by 0.1.x.
- Files from a newer version are refused with a clear message and are never modified.
- After loading, everything is validated: music clips use audio-only files and do not overlap, subtitle lines refer to existing media, have start before end and at most 1000 characters; the file is at most 16 MiB.
- The window title shows the project name with a `•` when there are unsaved changes, or "Untitled" before the first save.
- New, Open, closing the window and Quit (Cmd+Q on macOS, Ctrl+Q elsewhere) ask Save / Discard / Cancel when there are unsaved changes. A failed or cancelled save stops the action. On macOS the default application menu is turned off so that Cmd+Q reaches this prompt; as a consequence the standard Hide and Minimize menu items are not available.

## Non-functional targets
- At most 3 `ffmpeg`/`ffprobe` processes at any time (guaranteed by the job queue, tested with 20 imports + export + preview).
- Window opens in under 1 s; the `ffmpeg` check runs in the background ("checking ffmpeg…").
- Importing 10 files does not block the UI (each frame under 50 ms).
- Still preview frame: no render for edits that do not change the picture at the playhead; at most one render for a burst of 10 edits.
- Playback: video keeps real time if audio stalls; audio/video drift after the audio resumes at most 40 ms (requirement: 100 ms).
- Export of 200 segments from one source: one decoder, frame-accurate.
- Memory does not grow with source file size: at most 200 MB more peak memory for a long source than a short one of the same resolution. Assumption: tested in CI with a 2–5 minute source; since no internal buffer depends on file size, the result is expected to hold for multi-gigabyte files.
- Targets, not gates: playback start with a proxy under 300 ms; speech-to-text at least 10x faster than real time on 4 threads; installers about 70–130 MB each (estimates).
- Export progress at least twice per second; cancel within 1 s; project saves are atomic.
- Platforms: Linux x86_64 (current Ubuntu LTS and Fedora), macOS Apple Silicon, Windows x86_64.
- Code coverage >= 85% and every requirement covered by at least one test: logic is kept out of the UI.

## Rust toolchain policy
- The project follows the most recent stable Rust; `rust-version` in `Cargo.toml` is the current stable release (1.100). Building from source needs an up-to-date Rust (`rustup update`).
- All dependencies are kept on their latest releases. When a new Rust release brings new warnings or lints, the code is fixed; old versions are not pinned.
- Why: in 0.1.5 CI on the latest Rust failed on lints that Rust 1.88 did not have; upgrading was chosen over freezing CI on an old Rust.

## Build and CI
- CI installs `ffmpeg`, `libasound2-dev`, `cmake`, `libclang-dev` and `espeak-ng` on Linux. The check job sets `VIDEOLORE_REQUIRE_FFMPEG=1`, so tests that need `ffmpeg` fail instead of being skipped.
- Building from source needs `libasound2-dev` on Linux (audio) and, for the default speech-to-text feature, `cmake`, a C++ compiler and libclang. `--no-default-features` builds without both.
- CI builds with the latest stable Rust and, separately, with exactly `rust-version`. Clippy runs with default, no and all features.
- CI also checks licences and security advisories of all crates (`cargo-deny`) and that the generated crate notices are up to date.
- Release installers are built by CI for each version tag, see [packaging](architecture-packaging.md).

## Known risks
Listed in each sub-document; the main ones:
- Some behaviour can only be checked by hand on real machines: first launch of unsigned installers, double-click on a project on macOS while starting, Linux file managers.
- Speech-to-text needs a C++ toolchain and libclang on every CI platform, which makes builds slower and more fragile.
- egui releases often change its API: GUI code is kept thin so upgrades stay small.
- A new stable Rust can add lints that make CI fail: the code is fixed promptly.
