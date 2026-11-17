# Architecture

Built up to version 1.3.0. The 1.3.0 parts are text items, one selection for the preview and the timeline, the Context Menu, 360 anchors and 360 video export (REQ-106..113; see [text](architecture-text.md) and [360 video](architecture-360.md#anchored-text-and-subtitles-130)). The 1.2.6 parts (360 video and boxes) are described below. Deviations from the plan are listed at the end of this page. Rust (edition 2024), minimum Rust 1.100 (the project follows the latest stable Rust). License GPL-3.0.

## Overview
`videolore` is a desktop video editor with a native window (Linux, macOS, Windows). It does not decode or encode video itself: it drives the `ffmpeg` and `ffprobe` programs, which are **bundled with the installers** since 0.2.0. The editor keeps the edit (video clips, cuts, fades, a music track, subtitles) in memory, previews it, and exports it with a single `ffmpeg` run.

Implemented: requirements REQ-001..113 (see [business](business.md)). 0.3.0 added automatic setup for each computer, a plain-language Settings window, three subtitle quality levels, a processor check before generating subtitles, and security hardening (REQ-045..054, 056..059). 0.4.0 adds an opt-in faster export with the computer's video hardware (NVIDIA, Intel, AMD, Linux VA-API, Apple VideoToolbox), with an automatic check of every result and a silent redo with the normal method on any problem (REQ-055, 060..065; see [hardware export](architecture-hardware-export.md)). 1.1.0 restructures the window: resizable sections, a menu bar, a toolbar, a status bar, an Effects list with popups, markers for fades and subtitle lines on the timeline, recent projects and drag and drop (REQ-066..081; see [user interface](architecture-ui.md)). 1.2.0 lets every effect be applied to one clip or to the whole video (fades from and to black, per-clip volume, subtitles for one clip), reuses text subtitle tracks found inside videos with words matched to the speech, exports subtitles as SRT, lets each subtitle line be placed anywhere on screen (alone or as a selected group), and labels the time ruler in seconds, minutes or hours (REQ-082..095; see [effects](architecture-effects.md) and [subtitle tracks](architecture-subtitle-tracks.md)). 1.2.6 adds a visible box with corner handles around every movable object on the picture (today the subtitle lines) with a per-line size, toolbar commands Align horizontally, Align vertically and Reset, and 360 video with look-around, points of view (POVs) and a linear camera path exported as a flat video (REQ-096..105; see [boxes](architecture-boxes.md) and [360 video](architecture-360.md)). 1.3.0 adds text items (titles and captions with fades, blinking, slide-in, a colour and an installed font), one selection shared by the preview and the timeline, the Context Menu, texts and subtitle lines anchored to a point of a 360 sphere, and 360 video export that keeps the whole sphere (REQ-106..113; see [text](architecture-text.md) and [360 video](architecture-360.md)).

## Sub-documents
| Document | Contents |
|---|---|
| [architecture-decisions.md](architecture-decisions.md) | The main technical decisions of the whole program, with reasons and alternatives. |
| [architecture-effects.md](architecture-effects.md) | Applying effects to a clip or the whole video (1.2.0): fades from and to black (named Fade IN/OUT since 1.3.0), clip volume, drag from the Effects list, export rules, tests. |
| [architecture-subtitle-tracks.md](architecture-subtitle-tracks.md) | Subtitle tracks inside videos (1.2.0): detection, reading, "use or recognise", matching words to the speech, fallbacks, SRT export, tests and risks. |
| [architecture-boxes.md](architecture-boxes.md) | 1.2.6: boxes and corner handles, per-line size, Align and Reset commands, toolbar group, show-boxes setting, tests. 1.3.0: Context Menu and text boxes. |
| [architecture-360.md](architecture-360.md) | 1.2.6: recognising 360 video, virtual camera and POVs, one `ffmpeg` engine for preview and export, preview copies, size limit, performance, tests, risks. 1.3.0: anchored text and subtitle lines on the sphere, export as 360 video. |
| [architecture-text.md](architecture-text.md) | 1.3.0: text items, their effects as ASS events, installed fonts, safety, tests. |
| [architecture-projects.md](architecture-projects.md) | Project files: format versions 1–5 and compatibility, loading and validation, unsaved changes. |
| [architecture-ui.md](architecture-ui.md) | The window (1.1.0): layout and resizable sections, menus, toolbar, shortcuts, status bar, effect popups, timeline markers, recent projects, drag and drop, interface tests. 1.3.0: one selection, Add Text popup. |
| [architecture-performance.md](architecture-performance.md) | Machine detection, recommended settings, the "How hard videolore works" levels and what they control, the 0.3.0 performance review. |
| [architecture-hardware-export.md](architecture-hardware-export.md) | Faster export with video hardware (0.4.0): supported hardware, encoder settings (including VA-API constant quality), background check (including the Linux libva check), output check, software redo, remembered failures, measured results, testing and known limitations. |
| [architecture-ffmpeg.md](architecture-ffmpeg.md) | The bundled `ffmpeg`: which builds, where they live, how they are found, licences and source code. |
| [architecture-packaging.md](architecture-packaging.md) | Installers per OS, file association, icon, Linux menu entry, release pipeline, CI runners. |
| [architecture-media-pipeline.md](architecture-media-pipeline.md) | Process limit and job queue, preview, export, music mix, proxies, memory limits. |
| [architecture-subtitles.md](architecture-subtitles.md) | Subtitle model, styles and burn-in, creating subtitles for a clip or the whole video, position on screen and selection of lines (1.2.0), speech-to-text, subtitle quality levels and their downloads, processor requirements. |
| [architecture-security.md](architecture-security.md) | Threat model and protections, temporary files, downloads, supply chain, text items and fonts, the 360 marker. |
| [architecture-dependencies.md](architecture-dependencies.md) | Rust crates, build features, external tools and bundled components, with reasons. |

## Components
| Module | Role |
|---|---|
| `time` | Exact time arithmetic and timestamp formatting (no floating point in cut math). |
| `media` | Information about a media file read from `ffprobe`; status "analysing / ready / rejected"; video or audio-only; since 1.2.0 its text subtitle tracks; since 1.2.6 whether it is 360. |
| `timeline` | The video track: clips, in/out points, order, cuts, fades (since 1.2.0 also from and to black at the video's edges), each clip's volume, and the rules that keep it valid. |
| `music` | The music track: audio clips that never overlap; place, move, trim, split, delete. |
| `subtitles` | Subtitle lines stored against the source speech, and how they appear on the timeline; since 1.2.0 each line's position on screen. |
| `srt` (1.2.0) | Reads and writes SRT subtitle files (tracks found in videos, SRT export). |
| `align` (1.2.0) | Matches the words of a subtitle track to the recognised speech to time each word. |
| `camera` (1.2.6) | Pure camera maths for 360 video: limits, POV path (linear, shortest way round), drag and zoom mapping, the command file for the `v360` filter (absolute commands, changed values only). Since 1.3.0 also the projection between the sphere and the view. |
| `ass` | Writes the subtitle file (ASS format) used by `ffmpeg` for the chosen style. |
| `history` | Undo and redo of the whole edit (video, music, subtitles, volumes); since 1.1.0 each step has a name ("Undo Split"). |
| `project` | Save/load projects as versioned JSON (version 5 since 1.3.0; opens all older versions); unsaved-changes tracking. See [project files](architecture-projects.md). |
| `format` | Output formats, codecs and which encoder to use, with fallbacks. |
| `render` | Turns the edit into an export plan and the exact `ffmpeg` arguments; also the preview graphs. |
| `ffmpeg` | Finds `ffmpeg`/`ffprobe` (bundled copy first), checks features, runs them, reads progress, cancels. |
| `jobs` | The job queue: limits how many `ffmpeg`/`ffprobe` processes run at once (3 to 6, depending on the chosen level), by priority. Every process goes through it. |
| `export` | Runs an export in the background and writes the result safely; since 0.4.0 tries video hardware first when asked, and redoes the export with the normal method on any problem. |
| `hwenc` (0.4.0) | Video hardware candidates per system, their `ffmpeg` settings, and the background hardware check. |
| `outcheck` (0.4.0) | Checks a hardware export (file details, full decode, sample frames compared with the normal render). |
| `preview` | Decodes frames and audio for the preview player. |
| `proxy` | Lower-resolution preview copies of large sources, and their cache. |
| `stt` | Speech-to-text: audio extraction (since 1.2.0 also of one clip's part, and of subtitle tracks), transcription, word matching jobs, language detection, the three model levels, download and storage. |
| `machine` | Detects the computer: cores, memory, operating system, processor features. |
| `settings` (0.3.0) | Program settings file, first-start setup, recommended settings, how each level translates into processes and threads; since 0.4.0 the Export choice and remembered hardware failures; since 1.1.0 the section layout and recent projects. |
| `scratch` (0.3.0) | Private work folders for temporary files, and clean-up after a crash. |
| `notices` | The third-party notices shown in About. |
| `desktop` | Linux application-menu entry and file association; macOS "open document" events. |
| `ui` | Windows and panels, including the Settings window. Kept thin. Since 1.1.0 also: the section layout and borders, the table of actions shared by menus, toolbar and shortcuts, the menu bar, toolbar, status bar, effect and clip popups, and the timeline markers. Since 1.2.0 also the time ruler, dragging effects onto clips and subtitles in the preview. See [user interface](architecture-ui.md). |
| `text` (1.3.0) | Text items: pure generation of their ASS events (fades, blink, slide, cuts at segment boundaries), caps and the font-name rule. |
| `fonts` (1.3.0) | Installed fonts: background scan (`fontdb`), family list, name check, font bytes for layout (cached). |
| `selection` (1.3.0) | The one selection of subtitle lines and text items, and its pure transitions (click, Shift range, group move). |
| `spherical` (1.3.0) | Writes the Spherical Video V1 marker into MP4 and MOV exports of 360 video, which ffmpeg 6.1.1 drops on re-encode. |
| `app` | Application state and wiring between the UI and background workers; since 1.1.0 also which popups are open, recent projects and dropped files; since 1.2.0 applying effects, the subtitle creation dialogs and the selection of subtitle lines. |

## Data flow
- **Start-up (0.3.0):** detect the computer, clean up leftovers of a crashed session, read the settings file, decide the settings (first start, same computer, changed computer, older version, damaged file), apply them to the job queue and preview, and show a notice if needed. Details in [performance](architecture-performance.md#start-up).
- **Editing:** a user action changes the edit and is recorded for undo; the unsaved-changes marker is recomputed; if the picture at the playhead changed, one still frame is requested (bursts of edits are merged).
- **Import:** each file appears at once as "analysing"; `ffprobe` runs in the background; the item then shows name and duration or the reason it was rejected. A large source may then get a proxy in the background.
- **Export:** `render` builds a plan from the original files (never proxies); the export worker runs one `ffmpeg` and reports progress; the result is renamed into place only when complete. With Export "Faster" (0.4.0) and working video hardware, an H.264/HEVC re-encode runs on the hardware first, is checked, and is redone with the normal method if anything is wrong. Details in [hardware export](architecture-hardware-export.md#export-with-hardware).
- **Subtitles:** for one clip or the whole video; if a source has a text subtitle track the user chooses its text or speech recognition; the processor is checked; the model of the chosen quality level is used (downloaded after consent if missing); the audio of the part concerned is extracted and transcribed on the machine, and turned into subtitle lines, or used to time the words of the track's lines. The result is applied as one undo step only when everything has finished. See [subtitles](architecture-subtitles.md#creating-subtitles).
- **Applying an effect (1.2.0):** dragging, the clip's context menu, the Effects menu and double-click all produce the same command. See [effects](architecture-effects.md).
- **Changing a setting:** applies immediately to the next job; running jobs continue unchanged. Settings are saved atomically and never stored in projects.
- **User interface (1.1.0):** every menu item, toolbar button, shortcut and context-menu entry produces a command from one shared table; the application state runs it. Dragging a section border updates the layout live and saves it once when released. See [user interface](architecture-ui.md).
- **Recent projects and drag and drop (1.1.0):** opening a recent or dropped project goes through the unsaved-changes prompt; dropped media follow the same path as File > Import.
- **360 video and boxes (1.2.6):** a 360 clip is rendered by `ffmpeg` through a camera (preview and export share one graph); dragging the picture changes a live camera, Set POV saves it into the project; boxes and handles are drawn by the preview only and change a line's saved position and size. See [360 video](architecture-360.md) and [boxes](architecture-boxes.md).
- **Text items and anchors (1.3.0):** text items are generated as ASS events on the timeline, cut per segment, and burned with the subtitles; on 360 clips anchored items are burned into the sphere frame before the camera projection; a 360 video export keeps the sphere and writes its marker. See [text](architecture-text.md) and [360 video](architecture-360.md).
- **Opening a project by double-click:** on Windows and Linux the file path arrives on the command line; on macOS it arrives as a system event. Both go through the unsaved-changes prompt.

The UI never waits on `ffmpeg` or on disk, apart from saving or loading the project file itself (at most 16 MiB), the settings file (at most 64 KiB) and a few small files written at Linux startup.

## Main technical decisions
The decisions with their reasons and alternatives are in [architecture-decisions.md](architecture-decisions.md). The 1.3.0 ones: text items through the subtitle path (libass), installed fonts with one new crate (`fontdb`), anchoring on the existing projection engine, one selection, format 5, and an in-house MP4/MOV marker writer.

## Projects
Versioned JSON files, format version 3 since 1.2.0, version 4 since 1.2.6, version 5 since 1.3.0 (opens versions 1 to 4 with exactly the same result; files from a newer version are refused and never modified); strict validation after loading; unsaved-changes prompt on New, Open, close and Quit. Details in [project files](architecture-projects.md).

## Deviations from the 1.3.0 plan (as built)
- **Time to live:** on load a text item is refused as damaged only outside 0.1 s to 24 hours (the plan said: longer than the video). A value longer than the video is accepted and the text is cut at the end of the video; when an item is added, edited or moved, its time to live is clamped to the video length. See [text](architecture-text.md#deviations-130).
- **360 marker in MP4 and MOV:** ffmpeg 6.1.1 drops the source's spherical information on re-encode (measured), so `videolore` writes the Spherical Video V1 marker itself into MP4 and MOV exports. MKV and WebM 360 exports carry no marker. See [360 video](architecture-360.md#export-as-360-video-130).
- **Anchored subtitle lines:** a subtitle line anchored on a 360 clip is drawn as plain white text, without its preset style (no box, no word highlight). See [360 video](architecture-360.md#anchored-text-and-subtitles-130).
- **Selection:** one selection value in `src/selection.rs`, as the plan's design; see [user interface](architecture-ui.md#selection-130).

## Non-functional targets
- `ffmpeg`/`ffprobe` processes never exceed the limit of the chosen level: 3 for Light and Balanced, 3 to 6 for Maximum (guaranteed by the job queue, tested with 20 imports + export + preview at every level).
- Window opens in under 1 s; 0.3.0 start-up at most 0.5 s slower than 0.2.2; the `ffmpeg` check runs in the background ("checking ffmpeg…").
- Computer detection: deterministic, no network, at most 3 s (expected: a few milliseconds).
- At recommended settings, export and Fast subtitles at most 5% slower than 0.2.2 on the same machine (measured by comparing the two versions). With "Best quality" (the default), 0.4.0 exports run the same commands as 0.3.0.
- Hardware check (0.4.0, limits changed in 0.4.2): 1 s first try per candidate plus one 4 s retry if it does not answer in time, at most 10 s in total, repeated once automatically when a time-out left a format without hardware (at most about 20 s), in the background, only when "Faster" is chosen. Hardware export: no progress for 30 s means a stall; cancel within 1 s in every phase. Target: at least 40% faster than "Best quality" at 1080p including the check; not met on the integrated Intel graphics measured for 0.4.0 (see [hardware export](architecture-hardware-export.md#performance)).
- The window keeps reacting while an export and subtitle generation run together at any level: every frame within 200 ms in an automated test (since 1.1.0 also while dragging a section border and with popups open).
- Interface (1.1.0): the smallest window stays 800x500; there every control is visible or reachable by scrolling; section proportions hold within 1 pixel at 800x500, 1280x800 and 2560x1440; all 12 toolbar commands usable at 800 px wide. Goal: under 16 ms per frame at 1280x800 (estimate).
- Subtitles and effects (1.2.0): matching words for 10,000 lines in under 50 ms (the cost is the speech recognition of the same audio); writing an SRT file of 10,000 lines in under 100 ms; burned subtitle position within 2% of the frame height of the preview position; real-speech matching test: at least 90% of words matched, each within 0.3 s; the window stays within 200 ms per frame while dragging effects and subtitles during an export and word matching.
- 360 and boxes (1.2.6): every interface frame within 200 ms while looking around, dragging boxes and playing during an export and subtitle generation (4K x 2K and 8K x 4K test files); camera still under 400 ms (4K) and 700 ms (8K copy), estimates; camera view within 1° of the export; box and size within 2% of the picture height; projects without 360 clips export with exactly the 1.2.5 commands.
- Importing 10 files does not block the UI (each frame under 50 ms).
- Still preview frame: no render for edits that do not change the picture at the playhead; at most one render for a burst of 10 edits.
- Playback: video keeps real time if audio stalls; audio/video drift after the audio resumes at most 40 ms (requirement: 100 ms).
- Export of 200 segments from one source: one decoder, frame-accurate.
- Memory does not grow with source file size: at most 200 MB more peak memory for a long source than a short one of the same resolution. Assumption: tested in CI with a 2–5 minute source. The "Most accurate" subtitle level needs about 1.3 GB more while transcribing (estimate) and is unavailable below about 8 GB of RAM.
- Targets, not gates: playback start with a proxy under 300 ms; installers about 70–130 MB each (estimates).
- Export progress at least twice per second; cancel within 1 s; project and settings saves are atomic.
- Platforms: Linux x86_64 (current Ubuntu LTS and Fedora), macOS Apple Silicon, Windows x86_64. Generating subtitles on x86_64 needs a CPU with AVX2, FMA and F16C (about 2013 or later); older processors get a message, never a crash. Hardware export: NVIDIA, Intel and AMD on Windows; NVIDIA, Intel and AMD (through VA-API) on Linux; VideoToolbox on macOS. NVIDIA needs driver 610 or newer.
- Text items (1.3.0): every interface frame within 200 ms with texts during an export and subtitle generation; event building under 50 ms for 200 items; font scan on a background thread (never delays the window); at most 2,000 events per item and 20,000 per project; 360 text centre within 1° and flat text centre within 2% of the picture height; 360 video export has no speed target.
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
- Drag and drop does not work in native Linux Wayland sessions (1.1.0); File > Import… and Open… remain the way there. Dragging effects inside the window (1.2.0) works everywhere.
- Word matching (1.2.0) depends on how closely a subtitle track follows the speech; whisper's word times are accurate to about 0.1–0.3 s. Projects saved by 1.2.0 cannot be opened by 1.1. See [subtitle tracks](architecture-subtitle-tracks.md#risks).
- 360 (1.2.6): exporting a moving camera runs at about a third of real time or less (accepted, speed work on the roadmap); camera stills while dragging may feel slow on large sources; two copies of the same part of one source share their POVs; projects saved by 1.2.6 cannot be opened by 1.2.5. See [360 video](architecture-360.md#risks).
- 1.3.0: libass and `fontdb` may name a font family differently (silent fallback in the export only); text drawn on the sphere narrows towards the poles (accepted); the 360 video export is slow (no speed promised); projects saved by 1.3.0 cannot be opened by 1.2.6. See [text](architecture-text.md#risks) and [360 video](architecture-360.md).
- egui releases often change its API; a new stable Rust can add lints: GUI code is kept thin and the code is fixed promptly.
