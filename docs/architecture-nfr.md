# Architecture: non-functional targets, limits and measurements

Part of [Architecture](architecture.md). Gates are checked by tests in the suite; estimates and quality items are measured and recorded in the developer documentation; manual items need a real system.

Status: GATE = checked by a test (fails the build); ESTIMATE = design figure, not gated; MEASURED = from a run (version or date given); NOT MET = measured and missed; ASSUMPTION = to be measured.

## Responsiveness and resources
- **Window (GATE):** every frame under 200 ms while an export and subtitle generation run, while dragging borders, with popups open, looking around, dragging boxes or an effect onto a clip, and with the 2.0.4 popups, theme, autosave and the LUT load (sampled tests: best of up to five, QG-18). ESTIMATE: under 16 ms per frame at 1280x800 with 50 media and 200 lines.
- **Start-up:** window open in under 1 s; ffmpeg check in the background; computer detection deterministic, no network, at most 3 s (ESTIMATE: milliseconds); at most 0.5 s slower than 0.2.2 (0.3.0 target).
- **Import:** 10 files without blocking the UI (each frame under 50 ms); `ffprobe` stopped after 30 s.
- **Process cap (GATE):** ffmpeg and ffprobe never above the level's limit (3 Light and Balanced; 3 to 6 Maximum; never below 3).
- **Preview (GATE):** no render for edits that do not change the picture at the playhead; at most one render for a burst of 10 edits (REQ-034); first frame within 200 ms with ten imports queued; a proxy start under 300 ms; the blurred background drawn once for ten picture changes at one playhead.
- **Playback (REQ-035):** keeps real time if audio stalls; drift after resumption at most 40 ms (requirement 100 ms).
- **Export:** progress at least twice per second; cancel within 1 s in every phase; 200 segments from one source use one decoder, frame-accurate (REQ-037); memory: at most 200 MB more peak for a 2 to 5 minute source than a short one of the same resolution (CI test; expected to hold for 4 GB); Best quality commands identical to 0.3.0 (GATE).
- **Export speed (0.3.0 target):** at recommended settings at most 5 % slower than 0.2.2.
- **Faster export (QG-05, target 40 % lower than Best quality at 1080p): NOT MET** on Intel Iris Plus G7 (0.4.0): −14 %, +41 %, +6 %. Quality VMAF −1.4, +1.4, +6.7 (never more than 3 below); size 0.96x, 1.32x, 1.38x (limit 1.5x).
- **Faster (REQ-315, target 20 % quicker): NOT MET** (2.2.0, Iris Plus G7, median of three): Best 12.3 s, Faster 11.3 s, 8.7 % lower. Before 2.2.0 (2.0.4): from 14 % quicker to 41 % slower.
- **Hardware check (GATE):** first try 1 s per candidate, retry 4 s, whole check 10 s, one repeat for an inconclusive answer; the reference render covers at most three one-second ranges; time-outs are not failures. Output check: 25 dB PSNR per plane at 10 %, 50 %, 90 % of frames; good encodes 33 to 45 dB (ASSUMPTION), garbage below 15 to 20 dB.
- **Blurred fill (REQ-314, MEASURED 2.2.0):** about 2 ms a frame at 1080p and 4K (testsrc2, 9:16, Medium, 300 frames), frame time 40 ms: met. Divisor 1/8 (ADR-180): 4K 2.9 to 3.1 s against 3.3 to 3.7 s for 1/4; 1080p 1.4 s against 1.5 to 1.6 s (ffmpeg 9.0.2). REQ-222 names 1/4 (product decision open).
- **Spatial sound (MEASURED 2.0.4, debug profile):** 120 s of four-channel sound in 1.7 s (69 times real time); a one-minute clip waits about 1 s for the preview sound. Real-time CPU on a weak computer unmeasured.
- **360 camera:** still latency 400 ms (4K) and 700 ms (8K preview copy) are ESTIMATES, measured about 125 ms and 155 ms on tiny clips; moving-camera export about one third of real time or less at 1080p (MEASURED, ffmpeg 6.1.1; the half-real-time target NOT MET); at most 10 camera updates per second (REQ-266); half-size drag, full size within 200 ms (REQ-267).
- **LUT (2.2.0 MEASURED):** a 256-point LUT takes about 12 s to the first graded preview frame (12.4 to 13.0 s), 144 points 2.26 s (not gated). ESTIMATE: parse 144 points at most 2 s and 256 at most 6 s; Work copy of 256 points at most 3 s; the window frame at most 200 ms while loading. ASSUMPTION: a 1080p preview with a 33-point LUT within 20 % of the playback rate without one; export with a LUT at most 1.5 times the re-encode.
- **Frame effects:** a 1080p export to 9:16 with Scale and Move at most 1.5 times the same re-encode (ASSUMPTION); the blurred background adds at most 10 % to composition (ESTIMATE); a still with effects under 80 ms (ESTIMATE).
- **Text (GATE):** best fit for 200 items under 10 ms; event building for 200 items under 50 ms; at most 2000 events per item and 20000 per project; font scan on a background thread; a 1.5 s fade lasts 1.5 s within one frame.
- **Subtitles:** matching 10,000 lines under 50 ms; SRT of 10,000 lines under 100 ms (GATE); burned position within 2 % of the frame height (GATE); real-speech matching at least 90 % within 0.3 s: NOT MET (REQ-177, 43 % at 2.2.0); Enlarge word centre within 2 % of the picture height (GATE, REQ-333).
- **Clean audio (real-ffmpeg gate on a committed clip):** noise-only parts at least 6 dB quieter, speech within 3 dB (ASSUMPTION).
- **Overlap media (2.3.0):** overlay cost at most 10 % more preview frame time for a 1080p still; export with one overlay at most 1.5 times without (ESTIMATE); alpha transparent in preview and export (GATE); format 14 round trip and 13 conversion (GATE).
- **Installers:** 70 to 130 MB each (ESTIMATE, measured in CI). **CI:** check job about 4.4 to 4.7 min after the QG-06 change (ESTIMATE; 25.2 min before); push to executable about 7 min instead of 28.
- **Coverage (GATE):** line coverage at least 85 % (`main.rs` excluded); feature coverage 100 %. Measured 94.9 % at 2.0.4.

## Sizes and limits
| Item | Limit | Area |
|---|---|---|
| Project file read | 16 MiB to format 10; 96 MiB from format 11 (LUT copies) | [projects](architecture-projects.md) |
| LUT file | 16 MiB and size 2 to 65 (2.0.4); 512 MiB streamed, size 2 to 256, 256-byte lines (2.1.0); `.3dl`/`.csp` 64 MiB | [picture](architecture-picture.md) |
| LUT in memory | 6 bytes a point (100.7 MB at 256, 17.9 MB at 144); cache 192 MB LRU | [picture](architecture-picture.md) |
| LUT copy in a project | 20 MB each, 64 MiB per project; above that only the path (REQ-293, 301) | [picture](architecture-picture.md) |
| Settings file | 64 KiB (worst case about 35 KB) | [performance](architecture-performance.md) |
| Autosave | every 2 minutes, one copy per project | [projects](architecture-projects.md) |
| Preview copy cache | 4 GB default; the open project never removed | [engine](architecture-engine.md) |
| Inbox of a second start | 64 files of 4 KiB | [UI](architecture-ui.md) |
| Subtitle import / track | 16 MiB and 100,000 cues; 16 tracks per file; title 64 characters | [subtitles](architecture-subtitles.md) |
| Subtitle text | 1000 characters a line; 2000 events per item, 20000 per file | [subtitles](architecture-subtitles.md) |
| Speech chunk | 120 s (about 7.7 MB), 2 s margins | [subtitles](architecture-subtitles.md) |
| Text items | time to live 0.1 s to 24 h; width 10 to 100 %; height 5 to 100 %; maximum 2 to 50 % (2.2.0); font file 64 MiB | [text](architecture-text.md) |
| Picture (PNG) | side 16384 px; path 4096 bytes; header only (24 bytes) | [text](architecture-text.md) |
| Picture or overlay file | 16,000,000 pixels (REQ-316) | [picture](architecture-picture.md) |
| Picture items | no count, file size or decode limit in code (plan: 20, 8 MiB): open | [risks](architecture-risks.md#open-product-decisions) |
| Frame | longer side 4096 px (decided 2026-11-19) | [picture](architecture-picture.md) |
| Keyframes | 200 per effect and clip; Scale 100 to 4000 permille; Move 0 to 10000 | [picture](architecture-picture.md) |
| Clip speed | 50 % to 200 % (500 to 2000 thousandths) | [effects](architecture-effects.md) |
| Volume keys | 0 to 100 %; 200 per clip | [effects](architecture-effects.md) |
| Blur | 1/8 of each side (`BLUR_DIVISOR` 8) | [performance](architecture-performance.md) |
| POVs | 200 per source; zoom 30° to 110°; pitch ±90° | [360](architecture-360.md) |
| 360 picture | preview copy 4096x2048 above 4096 px; too large above 16384x8192 | [360](architecture-360.md) |
| Spatial sound | 48 kHz, four channels, 10 ms blocks; preview pre-pass for clips up to 10 minutes | [engine](architecture-engine.md) |
| Preview frames | 8 frames of 640x360 (about 7 MB); audio 1 s | [engine](architecture-engine.md) |
| ffmpeg output | error log 64 KiB; short runs 16 MiB | [engine](architecture-engine.md) |
| Recent projects | 10 paths of at most 1024 bytes | [security](architecture-security.md) |
| Remembered hardware failures | 8 entries; 32 characters each | [export](architecture-export.md) |
| Section proportions | finite numbers 0 to 1; minimum widths 160 and 240 px; rows 100 px | [UI](architecture-ui.md) |
| Applied column share | 80 to 300 thousandths; default 150 | [effects](architecture-effects.md) |
| Installer fonts | Arabic and Devanagari faces, a few MB; CJK not bundled | [packaging](architecture-packaging.md) |

- 3.1.0 planned: audio tracks per file 32 (listed; the clip's track is 0 to 31); Enlarge word of a line: per-word events only for a line with an enlarged word (no change for other lines); a non-first audio track is re-encoded on export (time cost not measured); proxies keep every audio stream (size not measured). Acceptance figures are the ones of REQ-334 to 340 (docs/business*.md); none is measured yet.

## Security targets
- No shell; `file:` prefix; `file` and `pipe` protocols only (GATE). Hostile project and settings files: no crash, a clear refusal, nothing written, sources unchanged (GATE). Full list: [security](architecture-security.md).
- No network except the consented speech download; no telemetry; HTTPS-only table test; the HTTP client only in `stt` builds; the player graph without `ureq` or `whisper-rs` (CI).
- Work folders 0700 and files 0600 on Linux and macOS; partial files removed after crash or cancel.
- Unsafe code: three reviewed sites ([security](architecture-security.md#unsafe-code)).

## Portability
- Linux x86_64 (glibc 2.39 maximum), macOS Apple Silicon, Windows x86_64. Intel Macs, ARM Windows and ARM Linux out of scope.
- Subtitles need AVX2, FMA, F16C on x86-64 (about 2013 or later); older processors get a message.
- Hardware export: NVIDIA (driver 610 or newer), Intel and AMD on Windows; NVIDIA, Intel and AMD through VA-API on Linux (libva 2.21 or newer for the bundled ffmpeg); VideoToolbox on macOS.
- Drag and drop from outside: not on Linux Wayland. Accessibility: AccessKit on Windows, macOS and Linux (AT-SPI on Linux; size to be measured).
- Fonts: Latin, Greek and Cyrillic bundled; other scripts from system fonts.

## Verification of the targets
- Manual items per release (business verification): real hardware encoders (QG-05), first launch of unsigned installers, the macOS open-document event, Linux file managers, the MKV and WebM 360 marker in a real player, the menu bar on a Mac, the Windows player "Open with".
- Measurement: the best of up to five samples in tests with printed samples on failure; manual load checks.
