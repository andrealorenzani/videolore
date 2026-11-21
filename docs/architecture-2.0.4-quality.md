# Architecture — 2.0.4 quality, security and verification (built, not released)

Back to the [architecture index](architecture.md) and the [2.0.4 overview](architecture-2.0.4-overview.md).

## Non-functional targets
Gates are checked by automated tests in the suite; estimates and quality items are measured by the Principal Engineer and recorded in the developer documentation; manual items need a real system.
- **Compatibility gate:** a project without the new features exports with the same commands as 2.0.3 (golden tests, `render/golden_tests.rs`). A blurred project may change its background pixels only by the blur of an eighth of each side (ADR-180). The picture subtitle overlay is added only when a picture subtitle is chosen, so the golden argv of other projects is unchanged. The golden argument file of the format 4 and 5 projects (QG-15) is `tests/fixtures/export-argv-v4-v5-in-2.0.4.txt`.
- **Responsiveness gate:** every interface frame within 200 ms while an export and subtitle work run, with the new popups, theme and autosave active. The four UI frame gates run as sampled tests (best of up to five runs, the same limits).
- **Playback gate:** the first frame within 200 ms with ten imports queued; the blurred background drawn once for ten picture changes at one playhead.
- **Hardware check gate:** the reference render covers at most three one-second ranges; the check's time limits stay as they were (1 s and 4 s per candidate, 10 s in total, one repeat). The probe fakes that answer have long limits under load (200 ms per try).
- **360 gates:** the camera view is half size while dragging and full size within 200 ms after release; at most 10 camera updates per second of video; look-around within 200 ms (existing gate).
- **Effects gates:** LUT strength, `.3dl`, `.csp` and their input ranges within two colour levels of their references; a 1.5 s fade lasts 1.5 s within one frame; volume keys 0 % at 0 s, 100 % at 4 s, 50 % at 2 s (gain 0.25), preview and export agree.
- **Speed gate (REQ-281):** 4 s at 200 % lasts 2 s; pitch within 1 %; a subtitle within 0.1 s of its word.
- **Estimates:** cache enforcement under 100 ms for 1000 copies; project load under 1 s at 96 MiB; autosave copy cost proportional to the clip count.
- **Measured (not gated):** the spatial pre-pass runs at 69 times real time in the debug test profile (120 s of four-channel sound in 1.7 s; `the_pre_pass_runs_far_faster_than_real_time` prints it); a one-minute clip waits about 1 s before the first sound in the preview. The blurred fill: 4K 2.9 to 3.1 s (divisor 8) against 3.3 to 3.7 s (divisor 4). The cycle check runs in 1.7 s; the pinned ffmpeg and ffprobe are 173 MB each unpacked.
- **Quality items (not gates, recorded):** export time of 4K and HEVC, large frames, the blurred fill and Clean audio on a weak computer; Apply LUT cost with 33 and 65-point LUTs; 360 export on 8K x 4K; test-suite time; the AccessKit size on Linux.
- **Hardware measurement:** the integrated and dedicated graphics classes are timed separately; the target of the Faster export is set per class after the measurement (manual item 14).

## Sizes and limits
| Item | Limit | Why |
|---|---|---|
| Project file read | 96 MiB (was 16 MiB) | LUT copies are stored in the project |
| LUT copy | 20 MB each, 64 MiB per project | bounded memory and file size |
| Settings file | 64 KiB | unchanged; worst case about 35 KB |
| Autosave copy | interval 2 minutes, one copy per project | disk time |
| Preview copy cache | 4 GB by default (setting) | the open project is never removed |
| Installer fonts | Arabic and Devanagari faces, a few MB; CJK not bundled | installer size; the 25 MB MINOR budget is not under pressure |
| Inbox of a second start | 64 files of 4 KiB | untrusted input bounded |
| Subtitle file | 16 MiB, 100 000 cues | the existing import limits |
| Clip speed | 500 to 2000 thousandths (50 % to 200 %) | the `atempo` range of one filter |
| Volume keys | 0 to 100 % per key, at most 200 keys per clip | the keyframe limits |
| Picture item | PNG; side at most 16384 px; path at most 4096 bytes; read of the 24-byte header only | the picture is decoded by ffmpeg, never by videolore |
| Spatial sound | 48 kHz, four channels, 10 ms blocks; preview pre-pass only for clips up to 10 minutes | fixed format of the pre-pass; the preview waits at most a few seconds |
| Picture subtitles | the file's own subtitle stream; none on a 360 clip | the overlay follows the picture's scale |

Open against the plan: the picture items have no count limit (20 in ADR-192), no file size limit (8 MiB) and no decode pixel limit in the code, because videolore only reads the header; a decision is open (see the risks below).

## Security model additions
- **New untrusted inputs:** subtitle files (VTT and SRT: capped read, UTF-8 or refused, finite times, control and bidi characters removed); autosave copies (read by the strict project loader); the inbox of a second start (fixed folder, size and count caps, the same validation as File > Open); the player's settings file (capped, corpus test); LUT copies and `.3dl` and `.csp` files (limits before use, the `.cube` rules); camera roll, easing and spherical pose values (range checks); the device name in settings (checked against the enumerated list); picture items (PNG header read, side and path limits; the file is an ffmpeg input only); picture subtitles (the file's own stream, sanitised track list).
- **Processes:** no new kind of process. Hardware decoders run in ffmpeg child processes like the encoders, from enumerated render nodes or vendor lists. The player starts its sibling programs by fixed name next to its own executable, with an absolute path, argument lists and no shell. The spatial pre-pass is one more ffmpeg process through the job queue (decoding four channels to raw float on a pipe).
- **Files:** autosave, the one-window lock and inbox, the spatial WAV (`spatial-<hash>.wav`, written as `.part` and renamed) live in the private work or cache folders with fixed or hashed names; the WAV name hashes the file, range and camera path, so a found file is the right one. Cache cleaning deletes only its own names inside the cache folder, after a containment check. The player never writes next to a video. The 360 marker of MKV and WebM is written into the file the export made, with our own fields only.
- **Network:** unchanged. The only network use is the consented speech model download (whisper). Enhance speech adds no download. No telemetry, no secrets.
- **Unsafe code:** one more allow-listed call in the macOS part of `desktop.rs` (the menu bar item constructor, ADR-183), which amends ADR-034. Unsafe sites are now the whisper abort callback (`stt.rs`) and the macOS handler and menu (`desktop.rs`). The one-window lock uses the standard library.
- **Dependencies:** cargo-deny checks each new crate; the weekly advisory run is read-only; the Windows installer tool is pinned by version and hash.
- **Web page:** a Content-Security-Policy meta tag limits what the page loads (own host, the badge host, image data); connections are not allowed; the social image is a static file.

## Portability
- Linux x86_64 (glibc 2.39 maximum), macOS aarch64, Windows x86_64: unchanged build targets. Intel Macs, ARM Windows and ARM Linux stay out of scope.
- The one window works through a file and a lock on Windows and Linux; macOS keeps the running application.
- Hardware: AV1 is offered where an encoder is found; VP9 hardware encoders are rare, so software is the usual result. VideoToolbox AV1 needs recent Apple Silicon. Ubuntu 24.04 (libva 2.20) gets a message, not a rebuilt ffmpeg.
- Spatial sound: the pre-pass and the stereo pair are the same on every system (pure Rust and ffmpeg). Four-channel input is assumed to be first-order ambisonics in the order W, Y, Z, X; no other layout is recognised.
- Accessibility: AccessKit on Windows, macOS and Linux (AT-SPI on Linux); the names depend on the windowing toolkit.

## Verification
- The automated checks run in the test suite (fmt, clippy with all targets and without default features, tests, line coverage 85 % or more, a test for every implemented requirement) and in the continuous build. The module cycle check (`tests/module_cycles.rs`, seven tests) passes after the text split.
- Items that need a real system, a real ear or a person are in the [business verification document](business-verification.md#manual-checks-of-204-planned-not-a-blocker), items 9 to 35. They are not blockers; results are recorded at release.
- The speech fixture `tests/fixtures/align/speech.mkv` is committed and checked by SHA-256. Its generator, espeak-ng (`tests/fixtures/align/make.sh`), is not installed in CI and is not needed to run the tests.
- `build.rs` fails the build with a message when a splash summary is missing or empty; `scripts/version.sh set` runs the notices check afterwards as a reminder.

## Risks and cut-backs
- The designs of all built items are in the [overview](architecture-2.0.4-overview.md) and the area sub-documents. Nothing is cut back.
- Hardware AV1 and VP9 cannot be checked on every system; the output check protects files, not speed.
- The MKV and WebM 360 marker and the macOS menu bar are built, but the real-player check (item 23) and the menu bar on a Mac (item 16) are still manual.
- Enhance speech is level normalisation (`speechnorm`), not a neural noise model.
- The blurred fill is an eighth of each side; REQ-222 names a quarter. The requirement wording is a product decision.
- Spatial sound (REQ-273): the real-time CPU cost on a weak computer is unmeasured; roll is not applied to the stereo pair (pitch changes only the X and Z channels, which the pair does not use); the preview waits for the pre-pass before its sound starts; clips longer than 10 minutes keep their recorded sound in the preview. The export of a 360 video keeps the recorded sound.
- Picture subtitles: a subtitle already on screen when a run starts inside it appears at its next change (also for paused pictures); none on a 360 clip; the two test fixtures are hand-made PGS streams.
- Picture items: no count, file size or decode limit is enforced by the code (the plan's 20 items, 8 MiB and 16 megapixels); decision open.
- Project files with LUT copies are larger; the read limit is raised but still bounded.
- Screen-reader support on Linux adds the AT-SPI stack; its size is to be measured and recorded.
- The speed-first 360 preset (Q-5: the fastest preset within 1.5 times the Best-quality size) is not recorded by name in the notes; the rule is built.
