# Architecture

Version 0.4.0. Rust (edition 2024), minimum Rust 1.100 (the project follows the latest stable Rust). License GPL-3.0.

## Overview
`videolore` is a desktop video editor with a native window (Linux, macOS, Windows). It does not decode or encode video itself: it drives the `ffmpeg` and `ffprobe` programs, which are **bundled with the installers** since 0.2.0. The editor keeps the edit (video clips, cuts, fades, a music track, subtitles) in memory, previews it, and exports it with a single `ffmpeg` run.

Implemented: requirements REQ-001..065 (see [business](business.md)). 0.3.0 added automatic setup for each computer, a plain-language Settings window, three subtitle quality levels, a processor check before generating subtitles, and security hardening (REQ-045..054, 056..059). 0.4.0 adds an opt-in faster export with the computer's video hardware (NVIDIA, Intel, AMD, Linux VA-API, Apple VideoToolbox), with an automatic check of every result and a silent redo with the normal method on any problem (REQ-055, 060..065; see [hardware export](architecture-hardware-export.md)).

## Sub-documents
| Document | Contents |
|---|---|
| [architecture-performance.md](architecture-performance.md) | Machine detection, recommended settings, the "How hard videolore works" levels and what they control, the 0.3.0 performance review. |
| [architecture-hardware-export.md](architecture-hardware-export.md) | Faster export with video hardware (0.4.0): supported hardware, encoder settings (including VA-API constant quality), background check (including the Linux libva check), output check, software redo, remembered failures, measured results, testing and known limitations. |
| [architecture-ffmpeg.md](architecture-ffmpeg.md) | The bundled `ffmpeg`: which builds, where they live, how they are found, licences and source code. |
| [architecture-packaging.md](architecture-packaging.md) | Installers per OS, file association, icon, Linux menu entry, release pipeline, CI runners. |
| [architecture-media-pipeline.md](architecture-media-pipeline.md) | Process limit and job queue, preview, export, music mix, proxies, memory limits. |
| [architecture-subtitles.md](architecture-subtitles.md) | Subtitle model, styles and burn-in, speech-to-text, subtitle quality levels and their downloads, processor requirements. |
| [architecture-security.md](architecture-security.md) | Threat model and protections, temporary files, downloads, supply chain. |
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
| `jobs` | The job queue: limits how many `ffmpeg`/`ffprobe` processes run at once (3 to 6, depending on the chosen level), by priority. Every process goes through it. |
| `export` | Runs an export in the background and writes the result safely; since 0.4.0 tries video hardware first when asked, and redoes the export with the normal method on any problem. |
| `hwenc` (0.4.0) | Video hardware candidates per system, their `ffmpeg` settings, and the background hardware check. |
| `outcheck` (0.4.0) | Checks a hardware export (file details, full decode, sample frames compared with the normal render). |
| `preview` | Decodes frames and audio for the preview player. |
| `proxy` | Lower-resolution preview copies of large sources, and their cache. |
| `stt` | Speech-to-text: audio extraction, transcription, language detection, the three model levels, download and storage. |
| `machine` | Detects the computer: cores, memory, operating system, processor features. |
| `settings` (0.3.0) | Program settings file, first-start setup, recommended settings, how each level translates into processes and threads; since 0.4.0 the Export choice and remembered hardware failures. |
| `scratch` (0.3.0) | Private work folders for temporary files, and clean-up after a crash. |
| `notices` | The third-party notices shown in About. |
| `desktop` | Linux application-menu entry and file association; macOS "open document" events. |
| `ui` | Windows and panels, including the Settings window. Kept thin. |
| `app` | Application state and wiring between the UI and background workers. |

## Data flow
- **Start-up (0.3.0):** detect the computer, clean up leftovers of a crashed session, read the settings file, decide the settings (first start, same computer, changed computer, older version, damaged file), apply them to the job queue and preview, and show a notice if needed. Details in [performance](architecture-performance.md#start-up).
- **Editing:** a user action changes the edit and is recorded for undo; the unsaved-changes marker is recomputed; if the picture at the playhead changed, one still frame is requested (bursts of edits are merged).
- **Import:** each file appears at once as "analysing"; `ffprobe` runs in the background; the item then shows name and duration or the reason it was rejected. A large source may then get a proxy in the background.
- **Export:** `render` builds a plan from the original files (never proxies); the export worker runs one `ffmpeg` and reports progress; the result is renamed into place only when complete. With Export "Faster" (0.4.0) and working video hardware, an H.264/HEVC re-encode runs on the hardware first, is checked, and is redone with the normal method if anything is wrong. Details in [hardware export](architecture-hardware-export.md#export-with-hardware).
- **Subtitles:** the processor is checked; the model of the chosen quality level is used (downloaded after consent if missing); the audio of each source on the timeline is extracted, transcribed on the machine, and turned into subtitle lines.
- **Changing a setting:** applies immediately to the next job; running jobs continue unchanged. Settings are saved atomically and never stored in projects.
- **Opening a project by double-click:** on Windows and Linux the file path arrives on the command line; on macOS it arrives as a system event. Both go through the unsaved-changes prompt.

The UI never waits on `ffmpeg` or on disk, apart from saving or loading the project file itself (at most 16 MiB), the settings file (at most 64 KiB) and a few small files written at Linux startup.

## Main technical decisions
- **GUI: eframe/egui, latest release.** Pure Rust, single executable, all three platforms. Alternatives considered: iced, slint, gtk4-rs, tauri.
- **Media: `ffmpeg`/`ffprobe` as separate programs**, not linked libraries, now bundled. See [ffmpeg](architecture-ffmpeg.md).
- **Preview audio: cpal.** Without an audio device the preview continues silently.
- **Concurrency: standard threads and channels**, no async runtime. A job queue caps `ffmpeg` processes. See [media pipeline](architecture-media-pipeline.md).
- **Automatic setup in the app, not the installers (0.3.0):** the first start on every system (the AppImage too) detects the computer in milliseconds, without network or benchmarks, and saves recommended settings. Installers do no tuning, so there is one tested code path. See [performance](architecture-performance.md).
- **Plain settings (0.3.0, Export added in 0.4.0):** Preview quality, Subtitle quality, How hard videolore works and Export (Faster / Best quality), in a window inside `videolore` (not a separate system window, so it looks and behaves the same everywhere and can be tested automatically). Technical options sit in a collapsed "Advanced" section.
- **Export strategy:**
  - *Stream copy* (fast, lossless) only when all clips share the same format, the output format matches, there are no effects (fades, music, subtitles, video volume changes) and every cut is on a keyframe.
  - Otherwise *one re-encode pass* in a single `ffmpeg` run.
  - Output goes to a temporary file in the destination folder and is renamed when complete; a failed or cancelled export leaves no partial file. Source files are never overwritten.
- **Encoders:** each output format has a preferred encoder list with fallbacks; formats with no available encoder are disabled with an explanation. Software encoders by default.
- **Hardware export (0.4.0):** opt-in ("Best quality" stays the default everywhere), H.264 and HEVC only, the same target bitrate as the software method so files are about the same size, a background check run only when "Faster" is chosen, a 30 s stall watchdog, a check of every hardware result (strict full decode, file details, and a comparison with the normal pipeline's frames matched by position, 3 samples by PSNR) and a silent software redo on any problem. Drivers that refuse bitrate control get a VA-API constant-quality variant with size limits; with the bundled `ffmpeg`, VA-API and Quick Sync are skipped on Linux systems whose libva is older than 2.21 (for example Ubuntu 24.04). The settings file stays at version 2 with two optional fields, so 0.3.x can still read it. See [hardware export](architecture-hardware-export.md).
- **Fades:** 0.5 s or half the clip, whichever is shorter; video and audio aligned.
- **Speech-to-text:** local whisper.cpp, included in release builds; three quality levels, each model downloaded once after consent. See [subtitles](architecture-subtitles.md).
- **whisper.cpp CPU baseline (0.2.1):** compiled for AVX2/FMA/F16C on x86-64 instead of for the build machine's CPU. Since 0.3.0 the processor is checked first, so older computers get a clear message instead of a crash. A slower speech engine for older processors is not planned (decided on 2026-11-14). See [subtitles](architecture-subtitles.md#cpu-requirements-since-021).
- **Projects:** format version 2; unsaved changes are detected by comparing the edit with the last saved one, so undoing back to the saved state clears the marker. See [Projects](#projects).
- **Installers:** unsigned. See [packaging](architecture-packaging.md).

## Projects
- Version 2 adds the music track, the volume/mute settings and the subtitles (language, style, lines with word timings). 0.3.0 does not change the project format.
- Version 1 files (0.1.x) are converted in memory when opened and export exactly as before. Once saved they become version 2 and can no longer be opened by 0.1.x.
- Files from a newer version are refused with a clear message and are never modified.
- Loading reads only regular files, at most 16 MiB (0.3.0: a special file such as a pipe can no longer block the program). After loading, everything is validated: music clips use audio-only files and do not overlap, subtitle lines refer to existing media, have start before end and at most 1000 characters.
- The window title shows the project name with a `•` when there are unsaved changes, or "Untitled" before the first save.
- New, Open, closing the window and Quit (Cmd+Q on macOS, Ctrl+Q elsewhere) ask Save / Discard / Cancel when there are unsaved changes. A failed or cancelled save stops the action. On macOS the default application menu is turned off so that Cmd+Q reaches this prompt; as a consequence the standard Hide and Minimize menu items are not available.

## Non-functional targets
- `ffmpeg`/`ffprobe` processes never exceed the limit of the chosen level: 3 for Light and Balanced, 3 to 6 for Maximum (guaranteed by the job queue, tested with 20 imports + export + preview at every level).
- Window opens in under 1 s; 0.3.0 start-up at most 0.5 s slower than 0.2.2; the `ffmpeg` check runs in the background ("checking ffmpeg…").
- Computer detection: deterministic, no network, at most 3 s (expected: a few milliseconds).
- At recommended settings, export and Fast subtitles at most 5% slower than 0.2.2 on the same machine (measured by comparing the two versions). With "Best quality" (the default), 0.4.0 exports run the same commands as 0.3.0.
- Hardware check (0.4.0, limits changed in 0.4.2): 1 s first try per candidate plus one 4 s retry if it does not answer in time, at most 10 s in total, repeated once automatically when a time-out left a format without hardware (at most about 20 s), in the background, only when "Faster" is chosen. Hardware export: no progress for 30 s means a stall; cancel within 1 s in every phase. Target: at least 40% faster than "Best quality" at 1080p including the check; not met on the integrated Intel graphics measured for 0.4.0 (see [hardware export](architecture-hardware-export.md#performance)).
- The window keeps reacting while an export and subtitle generation run together at any level: every frame within 200 ms in an automated test.
- Importing 10 files does not block the UI (each frame under 50 ms).
- Still preview frame: no render for edits that do not change the picture at the playhead; at most one render for a burst of 10 edits.
- Playback: video keeps real time if audio stalls; audio/video drift after the audio resumes at most 40 ms (requirement: 100 ms).
- Export of 200 segments from one source: one decoder, frame-accurate.
- Memory does not grow with source file size: at most 200 MB more peak memory for a long source than a short one of the same resolution. Assumption: tested in CI with a 2–5 minute source. The "Most accurate" subtitle level needs about 1.3 GB more while transcribing (estimate) and is unavailable below about 8 GB of RAM.
- Targets, not gates: playback start with a proxy under 300 ms; installers about 70–130 MB each (estimates).
- Export progress at least twice per second; cancel within 1 s; project and settings saves are atomic.
- Platforms: Linux x86_64 (current Ubuntu LTS and Fedora), macOS Apple Silicon, Windows x86_64. Generating subtitles on x86_64 needs a CPU with AVX2, FMA and F16C (about 2013 or later); older processors get a message, never a crash. Hardware export: NVIDIA, Intel and AMD on Windows; NVIDIA, Intel and AMD (through VA-API) on Linux; VideoToolbox on macOS. NVIDIA needs driver 610 or newer.
- Code coverage >= 85% and every requirement covered by at least one test: logic is kept out of the UI.

## Rust toolchain policy
- The project follows the most recent stable Rust; `rust-version` in `Cargo.toml` is the current stable release (1.100). Building from source needs an up-to-date Rust (`rustup update`).
- All dependencies are kept on their latest releases. When a new Rust release brings new warnings or lints, the code is fixed; old versions are not pinned.

## Build and CI
- CI runs on fixed runner versions (Ubuntu 24.04, macOS 15, Windows Server 2025) instead of "latest", and every third-party CI action is pinned to an exact commit of a release that runs on Node.js 24. See [packaging](architecture-packaging.md#ci-runners-and-actions).
- CI installs `ffmpeg`, `libasound2-dev`, `cmake`, `libclang-dev` and `espeak-ng` on Linux. The check job sets `VIDEOLORE_REQUIRE_FFMPEG=1`, so tests that need `ffmpeg` fail instead of being skipped.
- The tests run once in CI, with coverage measurement (since 0.3.0); the minimum-Rust job only builds. Test media are generated once and reused, and are as small as each test allows.
- Building from source needs `libasound2-dev` on Linux (audio) and, for the default speech-to-text feature, `cmake`, a C++ compiler and libclang. `--no-default-features` builds without both.
- CI also checks licences, security advisories and withdrawn ("yanked") versions of all crates (`cargo-deny`) and that the generated crate notices are up to date.
- Release installers are built by CI for each version tag, see [packaging](architecture-packaging.md).

## Known risks
Listed in each sub-document; the main ones:
- The 0.3.0 choices made while Andrea was away were confirmed on 2026-11-14 (message only for old processors, with no slower engine planned; the two larger speech models and their thresholds; "Light" limits each job; Settings as a window inside the app). Hardware encoding, left out in 0.3.0, is built in 0.4.0. Still open: whether computer detection could also run in the installers (currently the app does it), the measured values of the quality goals, and a single local test-and-coverage run.
- Hardware export cannot be tested on real graphics hardware in CI: it is tested with a simulated encoder and checked by hand per release. Encoder settings and the output-check threshold are assumptions until measured; damage only between the sampled frames is not detected automatically.
- Speed and memory of the larger speech models are estimates; the recommendation thresholds may be adjusted after measurement.
- Some behaviour can only be checked by hand on real machines: first launch of unsigned installers, double-click on a project on macOS while starting, Linux file managers.
- Speech-to-text needs a C++ toolchain and libclang on every CI platform, which makes builds slower and more fragile.
- egui releases often change its API; a new stable Rust can add lints: GUI code is kept thin and the code is fixed promptly.
