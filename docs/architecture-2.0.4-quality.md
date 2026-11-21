# Architecture — 2.0.4 quality, security and verification (built, not released)

Back to the [architecture index](architecture.md) and the [2.0.4 overview](architecture-2.0.4-overview.md).

## Non-functional targets
Gates are checked by automated tests in the suite; estimates are measured by the Principal Engineer and recorded in the developer documentation; manual items need a real system.
- **Compatibility gate:** a project without the new features exports with the same commands as 2.0.3 (golden tests). A blurred project may change its background pixels only by the blur of an eighth of each side (ADR-180).
- **Responsiveness gate:** every interface frame within 200 ms while an export and subtitle work run, with the new popups, theme and autosave active.
- **Playback gate:** the first frame within 200 ms with ten imports queued; the blurred background drawn once for ten picture changes at one playhead.
- **Hardware check gate:** the reference render covers at most three one-second ranges; the check's time limits stay as they were (1 s and 4 s per candidate, 10 s in total, one repeat).
- **360 gates:** the camera view is half size while dragging and full size within 200 ms after release; at most 10 camera updates per second of video; look-around within 200 ms (existing gate).
- **Effects gates:** LUT strength, `.3dl`, `.csp` and their input ranges within two colour levels of their references; a 1.5 s fade lasts 1.5 s within one frame.
- **Estimates:** cache enforcement under 100 ms for 1000 copies; project load under 1 s at 96 MiB; autosave copy cost proportional to the clip count.
- **Quality items (not gates, recorded):** export time of 4K and HEVC, large frames, the blurred fill and Clean audio on a weak computer; Apply LUT cost with 33 and 65-point LUTs; 360 export on 8K x 4K; test-suite time.
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

## Security model additions
- **New untrusted inputs:** subtitle files (VTT and SRT: capped read, UTF-8 or refused, finite times, control and bidi characters removed); autosave copies (read by the strict project loader); the inbox of a second start (fixed folder, size and count caps, the same validation as File > Open); the player's settings file (capped, corpus test); LUT copies and `.3dl` and `.csp` files (limits before use, the `.cube` rules); camera roll, easing and spherical pose values (range checks); the device name in settings (checked against the enumerated list).
- **Processes:** no new kind of process. Hardware decoders run in ffmpeg child processes like the encoders, from enumerated render nodes or vendor lists. The player starts its sibling programs by fixed name next to its own executable, with an absolute path, argument lists and no shell.
- **Files:** autosave, the one-window lock and inbox live in the private work or cache folders with fixed or hashed names. Cache cleaning deletes only its own names inside the cache folder, after a containment check. The player never writes next to a video. The 360 marker of MKV and WebM is written into the file the export made, with our own fields only.
- **Network:** unchanged. The only network use is the consented speech model download (whisper). Enhance speech adds no download. No telemetry, no secrets.
- **Unsafe code:** one more allow-listed call in the macOS part of `desktop.rs` (the menu bar item constructor, ADR-183), which amends ADR-034. Unsafe sites are now the whisper abort callback (`stt.rs`) and the macOS handler and menu (`desktop.rs`). The one-window lock uses the standard library.
- **Dependencies:** cargo-deny checks each new crate; the weekly advisory run is read-only; the Windows installer tool is pinned by version and hash.
- **Web page:** a Content-Security-Policy meta tag limits what the page loads (own host, the badge host, image data); connections are not allowed; the social image is a static file.

## Portability
- Linux x86_64 (glibc 2.39 maximum), macOS aarch64, Windows x86_64: unchanged build targets. Intel Macs, ARM Windows and ARM Linux stay out of scope.
- The one window works through a file and a lock on Windows and Linux; macOS keeps the running application.
- Hardware: AV1 is offered where an encoder is found; VP9 hardware encoders are rare, so software is the usual result. VideoToolbox AV1 needs recent Apple Silicon. Ubuntu 24.04 (libva 2.20) gets a message, not a rebuilt ffmpeg.
- Accessibility: AccessKit on Windows, macOS and Linux (AT-SPI on Linux); the names depend on the windowing toolkit.

## Verification
- The automated checks run in the test suite (fmt, clippy, tests, line coverage 85 % or more, a test for every implemented requirement) and in the continuous build.
- Items that need a real system, a real ear or a person are in the [business verification document](business-verification.md#manual-checks-of-204-planned-not-a-blocker), items 9 to 28. They are not blockers; results are recorded at release.

## Risks and cut-backs
- Not built yet: REQ-253, 257 to 260, 268, 273, 280, 281, 286 to 288 and QG-14 to QG-18. They stay planned; their design is kept with the development notes until built.
- Hardware AV1 and VP9 cannot be checked on every system; the output check protects files, not speed.
- The MKV and WebM 360 marker and the macOS menu bar are built, but the real-player check (item 23) and the menu bar on a Mac (item 16) are still manual.
- Enhance speech is level normalisation (`speechnorm`), not a neural noise model.
- The blurred fill is an eighth of each side; REQ-222 names a quarter. The requirement wording is a product decision.
- Project files with LUT copies are larger; the read limit is raised but still bounded.
- Screen-reader support on Linux adds the AT-SPI stack; its size is to be measured and recorded.
