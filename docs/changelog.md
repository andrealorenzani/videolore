# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 0.4.1 — 2026-11-14 21:29 +0000
### Workflow
- `/release [name]` now releases the **current** version: it creates `releases/<version> - <name>/` with a copy of all docs (README, docs, ai-docs) unchanged, `sources.zip` and `RELEASE_NOTES.md` taken from the changelog, cleans the changelog, commits, tags `v<version>` and pushes, watches CI, then moves to the next major version, commits and pushes. The name is asked when missing.
- The snapshot folder is renamed `release/` → `releases/`; `scripts/collect-release.sh <version> "<name>"` and `scripts/release-guard.sh` use it; `.claude/settings.json` denies Edit and Write there; CI takes the GitHub release notes from `releases/<version> - */RELEASE_NOTES.md`.
- New `docs/roadmap.md`, owned by the Documenter: known limitations and improvements for future versions. Every agent returns a `## Future work` section; `/add-feature` collects them, plus the features a plan postpones, and passes them to the Documenter. Seeded with the open items of 0.2.0–0.4.0.
- README "Latest changes" keeps only the last 3 versions, with a link to the changelog and the release notes.
- `AGENTS.md` scripts list completed.

## 0.4.0 — 2026-11-14 20:05 +0000
Summary: export can use the computer's video hardware. Settings has a new **Export** choice: Best quality (the default, unchanged) or Faster. Faster encodes H.264 and HEVC with the video hardware of NVIDIA, Intel, AMD or Apple when it works. Every hardware file is checked, and on any problem the export is silently redone with the normal method, so the result is always a correct file. Measured on Linux with Intel integrated graphics, Faster was not always quicker, so it stays opt-in. Two real-hardware fixes are included: Linux drivers that refuse bitrate control and older system libva versions.

- **Added**: **Export: Faster / Best quality** in Settings, with a one-line explanation each. Faster uses the video hardware for H.264 and HEVC re-encodes: NVIDIA (NVENC) on Windows and Linux, Intel Quick Sync on Windows and Linux, AMD (AMF) on Windows, VA-API on Linux (Intel and AMD) and VideoToolbox on macOS Apple Silicon (REQ-055, REQ-060).
- **Added**: background hardware check, at most 1 second per candidate and 3 seconds in total. It runs when Faster is saved or chosen, on "Use recommended settings" and on "Try video hardware again"; with Best quality nothing runs. It never delays the window, start-up or preview (REQ-061).
- **Added**: every hardware export is checked before its file appears: the whole file must decode without errors, the frame count, duration, size and frame rate must be as expected, and three sample frames (start, middle, end) must match the normal method's picture (PSNR at least 25 dB). Progress shows the check (REQ-063).
- **Added**: silent redo. If the hardware is missing, exits with an error, stalls for 30 seconds or fails the check, the export is redone with the normal method, with no error message. Cancelling during the hardware run, the check or the redo leaves no partial file (REQ-055, REQ-021).
- **Added**: a hardware failure is remembered per format, so later exports skip the hardware. The memory is cleared by a change of computer, an update of videolore, "Use recommended settings", or "Try video hardware again" in Advanced (REQ-064).
- **Added**: Advanced shows which video hardware works for each format, whether it was turned off after a problem, and which method the last export used (REQ-061, REQ-064).
- **Added**: the Export choice is saved with the program settings, never in a project. Settings files from 0.3.x and 0.2.x, and damaged settings, load as Best quality with no notice; a 0.3.x file keeps its "recommended" or "custom" status. The settings file stays format 2 with new optional fields, so 0.3.x can still read it (REQ-065).
- **Changed**: Best quality exports are unchanged: the same ffmpeg arguments as 0.3.0 (checked by tests against the 0.3.0 commands), so the same files and the same speed. Stream-copy exports are identical byte for byte with either choice (REQ-062, REQ-023).
- **Changed**: hardware exports match Best quality in container, video format, resolution, frame rate, 8-bit 4:2:0 picture, audio, frame count, timing, letterboxing, fades, music and burned-in subtitles. HEVC in MP4 or MOV carries the Apple tag hvc1 (REQ-062, REQ-036). Hardware exports aim at the same average bitrate, so file sizes stay close to Best quality.
- **Fixed**: Linux VA-API drivers that refuse bitrate control, such as Ubuntu's free Intel driver, are used at constant quality. A hardware file more than 1.5 times the expected size is redone with the normal method, and this does not count as a hardware failure.
- **Fixed**: with the bundled ffmpeg on Linux, VA-API and Quick Sync are skipped without a crash when the system libva is older than 2.21 (Ubuntu 24.04 ships 2.20). The normal method is used and Advanced says why. A system ffmpeg chosen by the user can still use the hardware there.
- **Security**: hardware runs count within the process limit (REQ-033); a stalled run is stopped; no output reaches its destination before it passes the check; temporary files are cleaned up as before (REQ-059).
- **Removed**: the known limitations "export uses the processor only" and "export does not use the video hardware yet" (replaced by Faster export).
- **Dependencies**: none added or removed.
- **Quality**: 219 unit tests and 23 integration tests pass (2 ignored); the 6 hardware integration tests pass with the bundled ffmpeg; fmt; clippy with default, no-default and all features, plus macOS and Windows cross-checks; cargo-deny; notices check; version check. Line coverage 96.3%; feature coverage 65/65 implemented requirements.
- **Quality (not reached)**: QG-05, real hardware, measured on Linux (Ubuntu 24.04) with Intel Iris Plus G7 and the system ffmpeg. Size (at most 1.5 times Best quality) and picture quality (at most 3 VMAF points below) were met on three projects. Speed was not met: Faster ranged from 14% quicker to 41% slower than Best quality, so the explanation says "often quicker, but not on every computer". On this driver hardware HEVC is broken; the output check catches it and the normal method makes the file. Not yet measured: Windows with NVIDIA, Intel or AMD; Linux with NVIDIA or AMD; macOS.
- **Workflow**: none.

Known limitations (0.4.0):
- NVIDIA needs graphics driver 610 or newer; with an older driver the normal method is used and Advanced says "graphics driver too old".
- Linux Intel and AMD hardware export needs system libva and the vendor driver (Intel media driver or Mesa). With the bundled ffmpeg it needs libva 2.21 or newer (Ubuntu 24.04 ships 2.20: skipped there, see Fixed). Fedora's default Mesa has no H.264/HEVC encoding; users there need the RPM Fusion freeworld VA drivers (for example `mesa-va-drivers-freeworld`).
- Faster is not always quicker, especially on integrated graphics (see Quality). Files may be a little larger.
- Combinations of system and graphics vendor not verified on real hardware are still protected by the redo and the output check.
- The output check samples three frames; damage between them can pass unnoticed (the decode check and frame count still apply).
- On computers with several graphics cards NVIDIA is always preferred; the card cannot be chosen. Quick Sync on Linux uses the default render device.
- Hardware is not used for preview copies, previews, effects, subtitles, decoding, AV1, VP9 or stream copy.
- Decisions taken for 0.4.0 are listed in [business-export](business-export.md#key-decisions).

## 0.3.0 — 2026-11-14 15:18 +0000
Summary: videolore now sets itself up for the computer it runs on at first start, and a plain-language Settings window controls preview quality, subtitle quality and how hard it works. Subtitles get three quality levels, and older processors get a clear message instead of a crash. Export and Fast subtitles keep their 0.2.2 speed. Security is hardened, the test run is shorter, and the CI warnings are gone.

- **Added**: automatic setup at first start on every system, the AppImage included. videolore detects the computer in milliseconds (no network, no benchmark), saves recommended settings and shows a notice that closes by itself after 8 seconds. Settings are saved as a new settings file (format 2); 0.2.x settings are migrated, keeping a custom ffmpeg and downloaded speech files, and turning preview copies off becomes Preview quality "Original" (REQ-045, 049).
- **Added**: Settings window, from the new **Settings** button in the command bar or Ctrl/Cmd+,, with **Preview quality** (Automatic / Smoother / Original), **Subtitle quality** (Fast / Balanced / Most accurate) and **How hard videolore works** (Light / Balanced / Maximum speed), each with a one-line explanation, a status line ("Using recommended settings for this computer" or "Custom settings") and a collapsed **Advanced** section (ffmpeg override, paths, versions and origin of bundled components). Choices apply without restart (REQ-046).
- **Added**: **Use recommended settings** re-checks the computer, resets the three choices and says what changed. It never touches the custom ffmpeg, projects, preview copies or downloaded speech files (REQ-047).
- **Added**: when the computer changes, recommended settings are recomputed silently with the notice; custom settings are kept and a notice offers "Use recommended settings" (REQ-048).
- **Added**: subtitle quality levels: Fast (32 MB, the 0.2.x model), Balanced (190 MB) and Most accurate (574 MB). A level's model is downloaded only when subtitles are generated with it, after a consent dialog showing size and licence; the dialog offers "Use Fast instead (no download)" when Fast is present. Most accurate is recommended only where it runs reasonably fast, and levels needing more memory than the computer has are greyed out. A local model file can still be chosen for each level (REQ-014, 052).
- **Added**: Settings shows the disk space used by downloaded speech files, with "Remove downloaded speech files" (confirmation). Projects and their subtitles are untouched (REQ-053).
- **Added**: older processors. On x86-64 computers without the instructions the speech engine needs (AVX2, FMA, F16C, BMI2, plus SSE4.2 and AVX), "Generate subtitles" shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand." and no download is offered. The rest of videolore works normally (REQ-054).
- **Added**: the window stays responsive while an export and subtitle generation run together, at any "How hard" level: play/pause, timeline scrolling and opening Settings are handled within 200 ms in a test (REQ-056).
- **Changed**: "How hard videolore works" sets the background work. Light uses at most half of the logical cores for each job; Balanced is the recommendation and matches 0.2.2; Maximum speed may use all cores. Never more threads than the computer has. Ffmpeg processes at once: 3 (Light and Balanced) or up to 6 (Maximum). The change applies to the next job; running jobs are not restarted (REQ-050).
- **Changed**: Preview quality replaces the preview-copy switch. Automatic is the 0.2.x rule, Smoother makes a copy of any source larger than 1280x720, Original makes none. The exported file does not depend on this choice (REQ-051).
- **Changed**: the "Clear proxy cache" command bar button is replaced by **Settings**; the cache is cleared with "Clear preview copies" in Settings.
- **Changed**: subtitle speed: one decoding state is reused for the whole job instead of one per 120 s chunk. Flash attention was measured and kept off because it made subtitles slower. Decoding stays greedy at every level.
- **Changed**: CI runs on pinned runners (`ubuntu-24.04`, `macos-15`, `windows-2025`) instead of `*-latest`. Every action is pinned to a full commit SHA on Node.js 24 (`actions/cache` v6.1.0 replaces v4). The token is read-only by default, tests run once under coverage measurement, the MSRV job only builds, and the Linux packages are checked to need glibc 2.39 or older. This removes the Node.js 20 deprecation warning and the `ubuntu-latest` migration notice (QG-04).
- **Changed**: bundled ffmpeg for Linux and Windows is updated to the BtbN build of 2026-11-13 (ffmpeg 9.0.2, build 57) (QG-02).
- **Changed**: the test run is shorter (`cargo test` 18.5 s before, 13.8 s now on the same computer, −25%); no test was removed or weakened. The test `abort_check_of_whisper_ignores_heap_garbage` was renamed from `whisper_is_not_aborted_by_heap_garbage`.
- **Changed**: export and Fast subtitles are not slower (same computer, 0.2.2 → 0.3.0): export 20.22 s → 20.14 s, Fast subtitles 4.12 s → 4.12 s, start-up 0.009 s → 0.009 s.
- **Fixed**: subtitle generation on older processors no longer crashes; the processor is checked first (see the Known limitations of 0.2.1 and 0.2.2).
- **Security**: work folders for temporary files are private (0700) and their partial files are user-only (0600); exports become readable by others only when complete. Leftovers of a crashed session are removed at the next start (REQ-059). Project and settings files are read only if they are regular files, with size limits enforced while reading (16 MiB and 64 KiB). Settings that cannot be read are replaced by the recommended ones; the bad file is kept as `settings.json.bad` (REQ-057). The ffprobe run has a 30-second timeout. Stream copy refuses file names with control characters, and displayed names are sanitised. Rust `unsafe` is forbidden crate-wide except in two reviewed places. Speech files are downloaded only from fixed HTTPS addresses, refused before connecting otherwise, and checked against size and SHA-256 before use; partial or wrong files are deleted (REQ-058).
- **Removed**: the known limitation "subtitles may crash on processors without AVX2" (replaced by the message above). The "Clear proxy cache" command bar button (see Changed).
- **Dependencies**: none added or removed.
- **Quality**: 181 unit tests and 17 integration tests pass; fmt; clippy with default, no-default and all features, plus macOS and Windows cross-checks; cargo-deny (advisories, licences, sources); notices check; version check. Line coverage 96.2%; feature coverage 58/58 implemented requirements. Word error rate on the reference clip: Fast 0.23, Balanced 0.15, Most accurate 0.00.
- **Quality (not reached)**: the `cargo test` target was at least 30% faster; the measured gain is 25%.

Known limitations (0.3.0):
- x86-64 processors without AVX2, FMA, F16C and BMI2 (about 2013 and older) cannot generate subtitles automatically. There is no slower fallback engine yet; lines can still be added by hand.
- Export does not use the hardware video encoder yet (REQ-055 planned).
- The speed and memory figures of Balanced and Most accurate, and the thresholds that recommend them, are estimates until measured on more computers.
- On virtual machines or containers whose number of cores changes between starts, videolore may treat each start as a new computer. With custom settings this only shows a notice.
- Cleared preview copies are made again only after reopening the project or changing Preview quality.
- Installers are unsigned (see 0.2.0). Speech is still recognised in pieces of about two minutes.
- Decisions taken while Andrea was away and awaiting his confirmation: detection at first app start on every system, three speech levels with consent, the message instead of a slower fallback for old processors, hardware export left out, the limits of "Light", and Settings as a window inside videolore.

## 0.2.2 — 2026-11-14 11:21 +0000
Summary: fixes subtitle generation failing at random on some computers, which affected 0.2.0 and 0.2.1. The cause was a memory-safety bug in the speech-recognition library, not the memory test named in the 0.2.1 entry. Failed subtitle generations now explain the reason in readable words.

- **Fixed**: subtitle generation could fail with "Generic whisper error ... Error code: -6" at random. Cause: whisper-rs 0.16.0 (latest on crates.io) has a bug in `FullParams::set_abort_callback_safe`: its callback read the cancel flag from uninitialised heap memory. On computers whose heap held leftover data, every subtitle generation was reported as interrupted. The same bug leaked memory on each run. videolore now uses its own cancel callback, which reads exactly one atomic flag, with checked lifetimes and SAFETY comments (`src/stt.rs`); the buggy function is no longer used.
- **Fixed**: whisper.cpp error codes are shown as readable messages, e.g. "the audio encoder failed or was interrupted (whisper.cpp code -6)".
- **Correction**: the 0.2.1 entry said the CI failure of `real_speech_is_transcribed` was probably caused by the memory test. That was wrong; the cause is the callback bug above. The memory-test change of 0.2.1 stays, as a fix for flakiness.
- **Changed**: CI test steps keep the `cargo test` output; on failure `scripts/test-annotations.sh` adds GitHub error annotations with the failing test names and panic messages, readable without opening the logs.
- **Quality**: new integration test `whisper_is_not_aborted_by_heap_garbage`, which failed 3 out of 3 runs with the old code, plus unit tests for the cancel callback, its wiring and the error texts. 139 unit tests and 15 integration tests (plus 1 ignored helper) pass; fmt and clippy (default, no-default and all features) pass; line coverage 95.9%; feature coverage 44/44 implemented requirements.

Known limitations (0.2.2): unchanged from 0.2.1.

## 0.2.1 — 2026-11-14 11:02 +0000
Summary: fixes the failed test run of 0.2.0 on GitHub and makes the release programs run on more computers. Subtitle generation (speech recognition) is now built for a common x86-64 processor baseline instead of the CPU of the build server, so the programs no longer crash on processors that lack the build server's instructions. Generating subtitles on Windows and Linux needs a processor with AVX2 (see Known limitations).

- **Changed**: whisper.cpp is compiled for a fixed x86-64 baseline (AVX2, FMA, F16C and BMI2 enabled; the build machine's CPU is not detected) through `.cargo/config.toml`. macOS Apple Silicon is not affected and remains supported. A build for the oldest processors (SSE only) was rejected because subtitle generation is about 7 times slower; see [subtitles](architecture-subtitles.md#cpu-requirements-since-021).
- **Fixed**: the `cargo test` run of 0.2.0 failed on GitHub. The memory test (REQ-039) counted the memory of the speech-recognition test running in parallel in the same process; each preview is now measured in its own process. The cause is probable, not confirmed, because the CI log could not be read.
- **Workflow**: CI build caches use the key prefix `v1-rust-ggml-portable`, so no whisper.cpp build made for the old native flags is reused. New test `whisper_is_built_for_a_portable_cpu_baseline` guards the flags and the cache prefix.
- **Quality**: 135 unit tests and 14 integration tests (including the real speech test) pass; fmt, clippy, coverage and feature coverage pass; line coverage 95.9%; feature coverage 44/44 implemented requirements.

Known limitations (0.2.1):
- Generating subtitles on Windows and Linux (x86-64) needs a processor with AVX2, FMA and F16C: Intel from 2013 (Haswell), AMD from 2015 (Excavator, all Ryzen). On older processors, subtitle generation crashes with an illegal-instruction error (assumption, not tested on such hardware); the rest of the program works. The program does not check the processor beforehand.
- Installers are unsigned: macOS Gatekeeper, Windows SmartScreen and the AppImage permission need a one-time confirmation.

## 0.2.0 — 2026-11-14 10:17 +0000
Summary: the first complete version for people who are not editing experts. Projects can be saved and reopened, ffmpeg is included in the app, subtitles are generated offline, a music track is added, and every platform gets a one-click installer. All known limitations of 0.1.3 are resolved.

- **Added**: save and open projects (`.vlproj`), Save and Save As, a title marker for unsaved changes; New, Open, closing the window and Quit ask Save / Discard / Cancel when there are unsaved changes (REQ-026, 027, 028).
- **Added**: 0.1.x projects open unchanged; projects from a newer version are refused with a message and not modified (REQ-029).
- **Added**: ffmpeg and ffprobe are bundled in every installer, so import, preview and export work without installing anything; a user-chosen ffmpeg (settings or environment variable) still overrides the bundled copy (REQ-030).
- **Added**: About screen (also shown by the Windows installer) listing every bundled component with version, licence and where to get its source (REQ-031).
- **Added**: installers, built and attached to every version tag: Windows setup (per user, no administrator rights, Start-menu entry), macOS disk image, Linux AppImage and `.tar.gz`; plus a third-party source archive. Installers are not code-signed in 0.2.0 (REQ-040, 042).
- **Added**: application icon; `.vlproj` files open in videolore with a double-click; on Windows and Linux a second window opens with that project, on macOS the running window asks about unsaved changes (REQ-041).
- **Added**: music track below the video track: audio files are placed, moved, trimmed, split and deleted; separate volume (0–100%) and mute for video audio and music; music does not move when the video is edited and is cut at the video end with a 0.5 s fade-out (REQ-019, 020, 043, 002).
- **Added**: subtitles generated offline from the speech with whisper.cpp, in the background with progress and cancel; the speech model is downloaded once after a consent dialog (about 32 MB) or provided as a local file; language detected automatically with manual override; lines added, deleted, re-worded and re-timed; three style presets (Classic, Box, Word highlight) visible in the preview and burned into the export; subtitles follow the speech when the timeline is edited; regenerating asks before replacing edited lines (REQ-014–018, 044).
- **Added**: preview proxies for big sources: a lower-resolution preview copy, made in the background with progress on the media item, stored outside the project and reused across sessions; the rule depends on resolution, cores and memory; export always uses the originals; the cache size is shown and can be cleared (REQ-038).
- **Added**: job queue that keeps at most 3 ffmpeg/ffprobe processes running, by priority (preview, export, import analysis, background work) (REQ-033).
- **Added**: still preview frame re-rendered only when the picture at the playhead changes, and bursts of edits are merged (REQ-034).
- **Added**: HEVC in MP4 and MOV is tagged `hvc1`, so it plays on Apple devices, both with stream copy and re-encode (REQ-036).
- **Changed**: import analysis runs in the background; several imports can run while editing, and the window is never blocked (REQ-032). Audio-only files are accepted into the media list.
- **Changed**: playback keeps the video in real time when audio output stalls; audio is corrected towards the clock, with drift under 40 ms (REQ-035).
- **Changed**: export runs one ffmpeg process with all clips grouped by source file, so a timeline of 200 segments from one source exports frame-accurately (REQ-037). Stream copy is used only when no fades, music, subtitles or volume changes are present and every cut is on a keyframe (REQ-023).
- **Changed**: memory does not grow with the size of the source file (REQ-039).
- **Changed**: project format version 2 (music, volumes, subtitles); saved projects can no longer be opened by 0.1.x.
- **Changed**: the README gives one install section per operating system; the requirement to install ffmpeg 4.4 or newer is gone.
- **Changed**: the Linux AppImage and `.tar.gz` add an application-menu entry and the `.vlproj` file type when run; "Remove from application menu" in the About screen undoes it.
- **Changed**: the macOS default application menu is switched off so that Cmd+Q reaches the unsaved-changes prompt; the standard Hide and Minimize items are therefore not available.
- **Changed**: `docs/business.md`, `docs/architecture.md` and `docs/code.md` became indexes with sub-documents per topic.
- **Fixed**: the known limitations of 0.1.3 are resolved: unsaved-changes prompt (added); import probing on the UI thread (now background); an ffmpeg process per edit for the still frame (now only when the picture changes); video waiting for a stalled audio output (now real time); HEVC in MP4 not tagged `hvc1` (tagged); one ffmpeg input per clip (inputs grouped by source).
- **Security**: the bundled ffmpeg and the packaging tools are pinned and checked by SHA-256; the installer build refuses an ffmpeg without the required encoders and filters; the speech model comes from a fixed revision over HTTPS, is checked by size and SHA-256 before use, and is the only network access, after consent; project files are limited to 16 MiB and validated after loading; cargo-deny checks licences, advisories and crate sources in CI.
- **Removed**: the requirement to install ffmpeg before using videolore; the 0.1.3 known limitations listed above; the subtitle feature placeholder (`stt` feature is now real).
- **Workflow**: the Documenter split the human docs into sub-documents; CI gained a `package` job (installers on Windows, macOS and Linux for version tags), cargo-deny and a notices-freshness check.
- **Dependencies**: added sha2 (normal dependency), sysinfo, whisper-rs and ureq (optional, feature `stt`), cpal (optional, feature `audio`), and on macOS objc2, objc2-foundation and winit; cargo-packager is used only in CI.
- **Quality**: 134 unit tests and 14 integration tests pass; fmt, clippy (default, no-default and all features, and cross-checks for macOS and Windows), cargo-deny, notices and version checks pass; line coverage 95.4%; feature coverage 44/44 implemented requirements.

Known limitations (0.2.0):
- Installers are unsigned: macOS Gatekeeper, Windows SmartScreen and the AppImage permission need a one-time confirmation.
- Subtitle text in scripts other than Latin, Greek and Cyrillic may look different per computer or show boxes, and cannot be read in the subtitle editor.
- Removing a file from the media list drops the undo history when an undo step refers to it.
- Cleared preview copies are regenerated only after reopening the project or toggling proxies; a 1280x720 source never gets a proxy.
- On Windows and Linux, double-clicking a project while videolore is open opens a second window.
- Some Linux file managers do not start the program from the `.tar.gz` folder on double-click.
- Speech is transcribed in 2-minute pieces; a word cut at a boundary can be lost.

## 0.1.6 — 2026-11-14 01:10 +0000
- **Workflow**: tests are run only by the Principal Engineer, after coding, in a loop (run the suite, fix the code, rerun) until everything passes; it reports line coverage and feature coverage. No other agent or command runs tests, builds or coverage: `/add-feature` commits after static checks only, `/release` asks the Principal Engineer for a VERIFY run, the Documenter writes the coverage badge from the reported value. CI still reruns everything on GitHub.
- **Added**: `scripts/coverage.sh --badge NN.N` writes the README coverage badge without running tests.

## 0.1.5 — 2026-11-14 01:03 +0000
Summary: the project follows the latest Rust (1.100) and the latest libraries (egui/eframe 0.37, rfd 0.18); the editing features are unchanged.

- **Changed**: minimum supported Rust (`rust-version`) raised from 1.88 to 1.100, the current stable release; the code is built and tested with it in CI.
- **Changed**: eframe/egui 0.33 → 0.37 and rfd 0.17 → 0.18, with the UI code adapted to the new API. Other dependencies were already on their latest releases (cpal 0.18.2, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0, sha2 0.11.1).
- **Changed**: CI uses actions/checkout v7, actions/upload-artifact v7 and softprops/action-gh-release v3 (Node 24), which removes the Node 20 deprecation warning. Clippy also runs with `--no-default-features` and `--all-features`.
- **Added**: CI job `msrv` builds and tests with the Rust version in `rust-version`; executables are built only when both the `check` and `msrv` jobs pass.
- **Fixed**: code that would stop compiling in a future Rust (untyped float literals in UI strokes now have an explicit `f32` type).
- **Fixed**: two clippy lints for the latest Rust (chunked audio conversion in preview and integration tests; an index loop in the timeline).
- **Fixed**: a preview test that failed about once in 16 runs is now deterministic.
- **Removed**: the known limitation "CI tests with the latest stable Rust; MSRV only verified locally": both are now checked in CI.
- **Workflow**: the Architect's Rust toolchain policy (follow the latest stable Rust, fix new lints in the code instead of pinning old versions) is recorded in `docs/architecture.md`; planned features no longer carry a release number (music and subtitles: release not yet assigned).
- **Quality**: 71 unit tests and 7 integration tests pass; line coverage 96.0%; feature coverage 18/18 implemented requirements; no future-incompatibility warnings.

## 0.1.4 — 2026-11-14 00:39 +0000
- **Fixed**: `scripts/version.sh set` is now all-or-nothing: if any step fails, `Cargo.toml`, `Cargo.lock` and the README badge are restored, so the version can no longer be left half-updated. It refreshes `Cargo.lock` with `cargo update --workspace` instead of a build, so it no longer needs system libraries such as ALSA.
- **Workflow**: owners may split their human-readable doc into `docs/<area>-<topic>.md` sub-documents (and should past ~150 lines or with distinct topics), keeping `docs/<area>.md` as the index; rule in `AGENTS.md`, guidance and an example in each agent; `scripts/feature-coverage.sh` also reads `docs/business-*.md`; sub-documents are not copied by `/clone-workflow`.

## 0.1.3 — 2026-11-14 00:26 +0000
Summary: videolore becomes a desktop, non-destructive video editor built on ffmpeg, for short social videos: import, cut, join and trim clips on one timeline, fade between clips, preview with sound, save projects and export to one file.

- **Added**: desktop window with a fixed five-panel layout (media list 30% | preview 70% in the top 60%; timeline 70% | effects/settings 30% in the middle 30%; bottom 10% bar with the zoom slider) (REQ-024).
- **Added**: keyboard shortcuts for split, cut, copy, paste, undo, redo and play/pause (REQ-025). Undo/redo keeps up to 100 steps (REQ-004).
- **Added**: import of video and audio files into a media list showing name and duration; unreadable files are rejected with a message (REQ-002).
- **Added**: single video track that never overlaps: append (REQ-005), split at playhead (REQ-006), cut/copy/paste with ripple (REQ-007), merge contiguous pieces of the same source (REQ-008), trim at either edge within the source (REQ-009), move clips (REQ-010).
- **Added**: fade through black between two clips: 0.5 s fade-out and 0.5 s fade-in on video and audio, half the clip length for clips under 1 s; can be removed (REQ-011).
- **Added**: timeline zoom slider from showing the whole project to single-frame level (REQ-012).
- **Added**: preview with play, pause (Space) and seek from the playhead, at most 360 px high, with sound (cpal) or silent when no audio device is available (REQ-013).
- **Added**: save and open projects as `.vlproj` (JSON, schema version 1); missing source files are reported by name (REQ-003). Source files are never modified (REQ-001).
- **Added**: export to one file with progress and cancel; a cancelled or failed export leaves no partial file (REQ-021).
- **Added**: export does stream copy (no re-encode) when all clips share the same format, there are no fades and every cut starts on a keyframe (REQ-023); otherwise one re-encode pass to the lowest-quality format among the clips, letterboxed, never stretched (REQ-022).
- **Added**: requirement for ffmpeg and ffprobe 4.4 or newer, installed by the user (not bundled). They are found via the app setting, `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`, the folder of the executable, or `PATH`. Missing ffmpeg shows a blocking message with install hints.
- **Added**: Cargo features `audio` (default, preview sound through cpal) and `stt` (reserved, empty, for planned subtitles).
- **Added**: CI installs ffmpeg, `libasound2-dev` and `pkg-config`, and runs the ffmpeg integration tests with `VIDEOLORE_REQUIRE_FFMPEG=1` (they fail instead of being skipped).
- **Changed**: `src/main.rs` opens the editor window; the placeholder text output is removed. Logic lives in library modules: `time`, `media`, `timeline`, `history`, `project`, `format`, `render`, `ffmpeg`, `export`, `preview`, `app`, `dialogs`, `ui` (`layout`, `zoom`, `keymap`).
- **Changed**: README describes the product, installation, requirements and usage; badges for the main libraries and for the ffmpeg requirement replace the "no dependencies" badge.
- **Security**: ffmpeg is started without a shell, with separate argument lists; inputs use the `file:` prefix and ffmpeg accepts only the `file` and `pipe` protocols (no network access, no option injection from file names); project files are validated after loading; exports never overwrite a source file.
- **Dependencies**: added eframe 0.33.3 (glow backend), rfd 0.17.2 (xdg-portal), cpal 0.18.2 (optional, feature `audio`), serde 1, serde_json 1, thiserror 2, directories 6, tempfile 3; dev-dependency sha2 0.11.1.
- **Quality**: 71 unit tests and 7 integration tests that run the real ffmpeg on clips generated at test time; line coverage 96.4%; feature coverage 18/18 implemented requirements.

Known limitations (accepted for this release):
- No "unsaved changes?" prompt on New, Open or quit.
- Import runs ffprobe on the UI thread, which can freeze the window briefly.
- Every edit starts ffmpeg to refresh the still preview frame.
- The preview video waits when audio stalls.
- HEVC in MP4 is not tagged `hvc1`.
- Each clip is a separate ffmpeg input, so many clips taken from one long file can hit decoder limits.
- CI tests with the latest stable Rust; MSRV 1.88 is verified only locally.

## 0.1.2 — 2026-11-13 23:31 +0000
- **Added**: README coverage badge (one decimal, currently 100.0%); code moved into `src/lib.rs` with a unit test, `src/main.rs` only prints.
- **Added**: CI checks version consistency, line coverage ≥ 85%, feature coverage 100% and that frozen release docs are untouched; tag releases use the release log as notes.
- **Removed**: `docs/releases/` (replaced by `release/<version>/`).
- **Workflow**: `scripts/version.sh` changes the version everywhere (Cargo.toml, Cargo.lock, README) and checks it; new `/release` command (major bump, freeze README/docs/ai-docs in `release/<version>/`, reset changelog, tag `v<version>`, push; CI creates the GitHub release); edits under `release/` denied in `.claude/settings.json`; Principal Engineer must reach ≥ 85% line coverage and 100% feature coverage (`REQ-NNN` ↔ `// covers:` tests); scripts `coverage.sh`, `feature-coverage.sh`, `collect-release.sh`, `release-guard.sh`.

## 0.1.1 — 2026-11-13 23:16 +0000
- **Added**: GitHub Actions build (`.github/workflows/build.yml`): fmt/clippy/test, then release executables for Linux x86_64, macOS arm64 and Windows x86_64, uploaded as artifacts and attached to GitHub releases on `v*` tags.
- **Added**: copyright notice for Andrea Lorenzani in `README.md` and SPDX/copyright header in `src/main.rs` (the GPL-3.0 text in `LICENSE` stays unmodified, as the licence requires).
- **Changed**: README "Latest changes" lists versions instead of dates; README contains no references to AI or the workflow; poem lines render as separate lines on GitHub.
- **Workflow**: every change bumps the version (last number +1, unless the user names a version); changelog entries carry version and date-time; commits use a human-readable one-line message; new source files carry the SPDX/copyright header.

## 0.1.0 — 2026-11-13 23:04 +0000
- **Workflow**: Documenter agent color changed to yellow; repository linked to `git@github.com:andrealorenzani/videolore.git` (`origin`, branch `main`).

## 0.1.0 — 2026-11-13 23:01 +0000
- **Added**: Rust crate `videolore` 0.1.0 (edition 2024, Rust 1.88), GPL-3.0 licence.
- **Workflow**: agents Product Manager, Architect, Principal Engineer, Documenter; commands `/add-feature`, `/change-workflow`, `/clone-workflow`; rules in `AGENTS.md` (`CLAUDE.md` symlink); workflow described in `docs/workflow.md`.
