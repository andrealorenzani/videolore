# Architecture: performance, automatic setup and speed

Part of [Architecture](architecture.md). Requirements: [performance](business-performance.md), [speed](business-speed.md). Targets in one place: [NFR](architecture-nfr.md). Export speed: [export](architecture-export.md).

## Detecting the computer
Done by the app at every start on every system (AppImage included); installers do no tuning. No network, no benchmark, no timing: the same computer gives the same result (a few milliseconds; upper bound 3 s).

| What | How |
|---|---|
| Logical cores | Rust standard library (container limits and affinity respected) |
| Physical cores | `sysinfo`; if unknown, half the logical cores rounded up; never above the logical cores |
| Memory | `sysinfo` total memory |
| OS, architecture | Rust standard library |
| Processor features (x86-64) | SSE4.2, AVX, AVX2, FMA, F16C, BMI2 via the standard library's feature check; all present on Apple Silicon |

The **machine fingerprint**: system, architecture, logical and physical cores, memory rounded to the nearest GiB, whether speech recognition can run.

## Start-up and settings
`settings.json` (program configuration folder) is read only if it is a regular file of at most 64 KiB. The decision is a pure function of the file and the computer:

| Situation | Result | Notice |
|---|---|---|
| No settings file | Recommended settings saved | "videolore has been set up for this computer. You can change this in Settings." (closes after 8 s) |
| File from 0.2.x | Recommended, except: preview copies off becomes Preview quality "Original"; custom ffmpeg path and downloaded speech files kept | as first start |
| Same computer | Settings used as saved | none |
| Different computer, recommended in use | New recommendation saved silently | as first start |
| Different computer, custom settings | Custom kept | offers "Use recommended settings" |
| Damaged or invalid | Recommended; the bad file kept as `settings.json.bad` | "Your settings could not be read, so videolore now uses the recommended settings for this computer." |
| Written by a newer version | Recommended in memory; file not overwritten unless a setting changes | none |

"Using recommended settings" is not stored: it means the choices equal what this computer gets. Settings belong to the program, never to a project. Settings version stays 2 (additive fields: ADR-028, ADR-040, ADR-044, ADR-195).

## Recommended settings
| Setting | Recommendation |
|---|---|
| Preview quality | Automatic |
| How hard videolore works | Balanced |
| Export | Best quality, on every computer |
| Subtitle quality | **Most accurate** with at least 12 logical cores and 15 GiB; otherwise **Balanced** with at least 4 cores and 7 GiB; otherwise **Fast**; always Fast when the processor cannot run speech recognition |

Memory is in GiB as reported (an "8 GB" computer reports about 7.6 GiB; threshold 7 GiB). Unavailable choices are greyed with a reason: all levels with an old processor ([subtitles](architecture-subtitles.md#cpu-requirements)); "Most accurate" below 7 GiB and "Balanced" below 3.5 GiB ("Needs more memory than this computer has."). The recommendation never picks an unavailable choice. "Use recommended settings" re-detects, resets only these choices, and shows what changed (or "Already using the recommended settings"); it never touches the custom ffmpeg path, projects, preview copies or speech files and never downloads. The speech models and thresholds were confirmed by Andrea on 2026-11-14 (assumptions until measured).

## "How hard videolore works" (ADR-029)
L = logical cores, P = physical cores.

| Resource | Light | Balanced (as 0.2.2) | Maximum |
|---|---|---|---|
| ffmpeg/ffprobe processes at once | 3 | 3 | L/2, between 3 and 6 (3 with less than 7 GiB) |
| Threads per export or preview copy | half the cores (at least 1) | chosen by ffmpeg | chosen by ffmpeg |
| Speech threads | half the cores, at most physical, 1 to 8 | all but one, at most physical, 1 to 8 | physical, 1 to 16 |
| Preview copies at a time | 1 | 1 | 1 |
| Preview playback, analysis, audio extraction | never limited | never limited | never limited |

| Computer | Light: processes / export threads / speech | Balanced: processes / speech | Maximum: processes / speech |
|---|---|---|---|
| 1 core | 3 / 1 / 1 | 3 / 1 | 3 / 1 |
| 2 logical, 1 physical | 3 / 1 / 1 | 3 / 1 | 3 / 1 |
| 4 logical, 2 physical | 3 / 2 / 2 | 3 / 2 | 3 / 2 |
| 8 logical, 4 physical | 3 / 4 / 4 | 3 / 4 | 4 / 4 (3 with less than 7 GiB) |
| 8 cores, no hyper-threading (Apple M) | 3 / 4 / 4 | 3 / 7 | 4 / 8 |
| 16 logical, 8 physical | 3 / 8 / 8 | 3 / 8 | 6 / 8 |
| 32 logical, 16 physical | 3 / 16 / 8 | 3 / 8 | 6 / 16 |

- At least 3 processes at every level (playback needs 2, export 1). Every number is at least 1 and at most the logical cores. In Light each background job uses at most half the cores.
- In Balanced and Maximum ffmpeg picks its thread count (about 1.5 internal threads per core for H.264), outside the guarantee, as in 0.2.2.
- Speech threads follow physical cores. Balanced never uses fewer than 0.2.2 (half the cores, at most 4).
- A change applies to the next job; running jobs are not restarted; lowering the limit never stops a running process.
- Changing the level may change the exact bytes of an export, never its look or length. Preview quality never changes the export.
- Rejected: a strict total budget shared by all jobs; lowering OS priority (needs unsafe platform calls); Linux control groups.

## Preview quality (0.3.0)
Automatic (default: sources above 1920x1080, or above 1280x720 on computers with fewer than 8 cores or less than 8 GB); Smoother (every source above 1280x720); Original (none). Export always uses the originals. Switching keeps existing copies and makes new ones in the background. Rule details: [engine](architecture-engine.md#proxies-req-038).

## Responsiveness while busy
An automated test runs an export and subtitle generation together, with speech recognition keeping every core busy, and sends play/pause, timeline scrolling and a Settings click over 30 frames. Each frame must finish within 200 ms and show the effect of its input. Design rule: the window's thread never waits for a lock held during file or process work. Importing 10 files does not block the UI (each frame under 50 ms).

## Speed decisions
### 0.3.0 review
| Area | Decision | Reason |
|---|---|---|
| Process limit | 3 for Light and Balanced; up to 6 in Maximum | playback 2 + export 1; more helps imports and copies on large machines |
| Speech threads | physical cores, up to 8 (Balanced) or 16 (Maximum) | whisper.cpp scales with physical cores |
| whisper.cpp | one decoding state per job; greedy decoding; flash attention off | flash attention measured slower |
| 120 s pieces | kept | they only bound memory |
| Export presets (x264/x265 veryfast, SVT-AV1 preset 10) | kept | changing them changes quality or size (a product decision) |
| Preview copies (H.264 veryfast, quality 23, 960x540) | kept | a faster preset would remake every copy |
| Preview (80 ms debounce, latest frame only) | kept | already minimal |
| Crates | none dropped, none added | [dependencies](architecture-dependencies.md) |

Targets: at recommended settings export and Fast subtitles at most 5 % slower than 0.2.2 on the same machine; start-up at most 0.5 s slower.

### 2.0.3 and 2.0.4 measurements
- Text sizes measured once per text, font and box, and kept until one changes; fonts read when used, not at start-up. The 2 s margins of speech pieces add about 3 % at Fast (target 5 %).
- Blurred fill at 1/8 of each side (ADR-180; `BLUR_DIVISOR` 8): 4K 2.9 to 3.1 s against 3.3 to 3.7 s for 1/4; 1080p 1.4 s against 1.5 to 1.6 s (ffmpeg 9.0.2, three runs). REQ-222 names 1/4 (open product decision).
- Spatial sound pre-pass: 120 s of four-channel sound in 1.7 s (debug profile, 69 times real time). Preview copy cache: under 100 ms for 1000 copies (measured once).
- Hardware check: reference render at most three one-second ranges (gate); limits 1 s and 4 s per candidate, 10 s total, one repeat (unchanged since 0.4.2).

### 2.2.0 speed (REQ-312 to 316; ADR-214 to 219; measured 2026-11-22)
- **Large LUT in the preview (REQ-312, D63, ADR-214, route D):** the `.cube` route is kept (same Work copy, `lut3d` tetrahedral, same colours). The preparation runs in the background with a percentage ("Preparing LUT <name>... N %", updated at least once a second; during the ffmpeg start a time-based estimate capped at 95 %). Until ready, the preview shows the picture without the LUT; a newer choice replaces the job; export waits for Ready in its own worker. Start about 12 s for 256 points (12.4 to 13.0 s), 2.26 s for 144 points; not gated. Rejected spikes: `.m3d` (ffmpeg 9.0.2 reads it as text: 3.4 to 5.1 s, 293 MB); binary 16-bit HALD with `haldclut` (0.36 s start, trilinear interpolation, a 16-bit path needed, unverified).
- **Applied rows (REQ-313, ADR-216):** rebuilt only when the selected clip, its order or its effects change (revision counter, undo and redo included); test: 0 rebuilds over 60 frames without change, 1 after one change.
- **Blurred fill in playback (REQ-314, ADR-217, built):** a `boxblur` of a copy at 1/8, scaled back; measured on the built graph (testsrc2, 9:16, Medium, 300 frames, no decode): about 2 ms a frame at 1080p and 4K; frame time 40 ms: met, no change. If missed, the first change removes a full-size background scale; a blur size change is a product decision.
- **Faster on integrated graphics (REQ-315, ADR-218): NOT MET.** Lever (a) built: the strict decode and the three sample renders run at the same time within the process cap. Iris Plus G7, median of three: Best 12.3 s, Faster 11.3 s (8.7 % lower; target 20 %). Levers (b) (0.1 s) and (c) (hardware decode of the check, a weaker check) not taken. The output check stays in full (`src/outcheck.rs`, REQ-206). Fewer samples not used.
- **Picture limit (REQ-316, ADR-219):** 16,000,000 pixels (PNG header); refused on Add picture; a project whose picture grew beyond it has a missing picture.
- Limits: the first graded frame of a 256-point LUT after about 12 s; the percentage at ffmpeg start is an estimate; scale changes in the preview are out of scope (roadmap).

## Security and privacy notes
- No network except the consented speech model download. The machine detection reads no user data. The settings file is read as a bounded regular file (64 KiB); a damaged file is kept as `.bad`, never executed.

## Risks
Listed in [risks](architecture-risks.md#technical-risks): speech model figures are estimates; containers whose core count changes between starts get recomputed settings; flash attention may be worth measuring on other processors; REQ-315 not met.
