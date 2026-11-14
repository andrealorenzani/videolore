# Architecture: performance and automatic setup

Part of [Architecture](architecture.md). How `videolore` (since 0.3.0) recognises the computer it runs on, which settings it recommends, what each setting controls, and the decisions of the 0.3.0 performance review.

## Detecting the computer
Done by the app itself at start-up, on every system (the AppImage included). Installers do no tuning: one code path, tested automatically.

| What | How |
|---|---|
| Logical cores | Rust's standard library (respects container limits and CPU affinity). |
| Physical cores | `sysinfo`; if unknown, half the logical cores rounded up; never more than the logical cores. |
| Memory | `sysinfo` total memory. |
| Operating system, architecture | Rust's standard library. |
| Processor features (x86-64) | SSE4.2, AVX, AVX2, FMA, F16C, BMI2, read with the standard library's feature check (which also confirms the operating system supports them). On Apple Silicon all are considered present. |

- No network, no benchmark, no timing: the same computer always gives the same result. Expected to take a few milliseconds; the guaranteed upper bound is 3 s.
- The result is summarised as a **machine fingerprint** (system, architecture, logical and physical cores, memory rounded to the nearest GiB, whether speech recognition can run). Rounding means small differences in reported memory between restarts do not count as a new computer.

## Start-up
The settings file (`settings.json` in the program's configuration folder) is read only if it is a regular file of at most 64 KiB. The decision is a pure function of the file contents and the detected computer:

| Situation | Result | Notice shown |
|---|---|---|
| No settings file (first start) | Recommended settings saved | "videolore has been set up for this computer. You can change this in Settings." (closes by itself after 8 s) |
| File from 0.2.x | Recommended settings, except: preview copies turned off becomes Preview quality "Original"; the custom `ffmpeg` path is kept; downloaded speech files are kept | Same as first start |
| Same computer as last time | Settings used as saved | None |
| Different computer, recommended settings in use | New recommendation saved silently | Same as first start |
| Different computer, custom settings | Custom settings kept | Offers "Use recommended settings" |
| Damaged or invalid file | Recommended settings; the bad file is kept as `settings.json.bad` | "Your settings could not be read, so videolore now uses the recommended settings for this computer." |
| File written by a newer version | Recommended settings in memory; the file is not overwritten unless the user changes a setting | None |

"Using recommended settings" is not stored: it means the three choices equal what the current computer would get. Settings belong to the program, never to a project.

## Recommended settings
| Setting | Recommendation |
|---|---|
| Preview quality | Automatic |
| How hard videolore works | Balanced |
| Subtitle quality | **Most accurate** with at least 12 logical cores and 15 GiB of memory; otherwise **Balanced** with at least 4 cores and 7 GiB; otherwise **Fast**. Always Fast when the processor cannot run speech recognition. |

Memory limits are in GiB as reported by the system: a computer sold as "8 GB" reports about 7.6 GiB, so 7 GiB is used as its threshold.

**Unavailable choices** are greyed out with a plain reason:
- All subtitle levels, when the processor is too old (see [subtitles](architecture-subtitles.md#cpu-requirements-since-021)).
- "Most accurate" below 7 GiB of memory and "Balanced" below 3.5 GiB: "Needs more memory than this computer has."

The recommendation never picks an unavailable choice. "Use recommended settings" in Settings re-detects the computer, resets only these three choices and shows what changed (or "Already using the recommended settings"); it never touches the custom `ffmpeg` path, projects, preview copies or speech files, and never downloads anything.

These thresholds are assumptions to be adjusted once speeds are measured on real machines.

## "How hard videolore works"
One choice controls all background work. L = logical cores, P = physical cores.

| Resource | Light | Balanced (same as 0.2.2) | Maximum |
|---|---|---|---|
| `ffmpeg`/`ffprobe` processes at once | 3 | 3 | L/2, between 3 and 6 (3 with less than 7 GiB of memory) |
| Threads per export or preview copy | half the cores (at least 1) | chosen by `ffmpeg` | chosen by `ffmpeg` |
| Speech recognition threads | half the cores, at most the physical cores, 1–8 | all cores but one, at most the physical cores, 1–8 | the physical cores, 1–16 |
| Preview copies made at the same time | 1 | 1 | 1 |
| Preview playback, file analysis, audio extraction | never limited | never limited | never limited |

Examples (with P = L/2 on processors with hyper-threading):

| Computer | Light: processes / export threads / speech threads | Balanced: processes / speech threads | Maximum: processes / speech threads |
|---|---|---|---|
| 1 core | 3 / 1 / 1 | 3 / 1 | 3 / 1 |
| 2 logical, 1 physical | 3 / 1 / 1 | 3 / 1 | 3 / 1 |
| 4 logical, 2 physical | 3 / 2 / 2 | 3 / 2 | 3 / 2 |
| 8 logical, 4 physical | 3 / 4 / 4 | 3 / 4 | 4 / 4 (3 with less than 7 GiB) |
| 8 cores, no hyper-threading (Apple M) | 3 / 4 / 4 | 3 / 7 | 4 / 8 |
| 16 logical, 8 physical | 3 / 8 / 8 | 3 / 8 | 6 / 8 |
| 32 logical, 16 physical | 3 / 16 / 8 | 3 / 8 | 6 / 16 |

- **At least 3 processes** at every level, because preview playback needs 2 and an export 1.
- Every number set by `videolore` is at least 1 and never more than the logical cores. In Light, each background job uses at most half the cores; two jobs running at once may together use all of them.
- In Balanced and Maximum, `ffmpeg` picks its own thread count, as in 0.2.2 (the H.264 encoder runs about 1.5 internal threads per core). This is outside the guarantee above; it keeps exports as fast as before.
- Speech threads follow physical cores, because whisper.cpp gains little from hyper-threads. Balanced never uses fewer threads than 0.2.2 did (half the cores, at most 4).
- A change applies to the next job; running jobs are not restarted. Lowering the process limit never stops a running process; new ones wait.
- Changing this level may change the exact bytes of an export (the H.264 encoder's output depends on thread count), never how it looks or how long it is. Preview quality never changes the export.
- Alternatives rejected: a strict total budget shared by all jobs (slower exports in Balanced, more complex); lowering the operating-system priority (needs unsafe platform calls); Linux control groups (Linux only).

## Preview quality
| Choice | Preview copies are made for |
|---|---|
| Automatic | The 0.2.x rule: sources above 1920x1080, or above 1280x720 on computers with fewer than 8 cores or less than 8 GB. |
| Smoother | Every source above 1280x720. |
| Original | None. |

Export always uses the original files, whatever the choice. Switching keeps existing preview copies and makes any newly needed ones in the background.

## Responsiveness while busy
An automated test runs an export and subtitle generation together, with speech recognition keeping every core busy, and sends play/pause, timeline scrolling and a Settings click over 30 frames. Each frame must finish within 200 ms and show the effect of its input. Design rule: the window's thread never waits for a lock held during background file or process work.

## 0.3.0 performance review
| Area | Decision | Reason |
|---|---|---|
| Process limit | 3 kept for Light and Balanced; up to 6 in Maximum | Playback needs 2 + export 1; more only helps imports and preview copies on large machines. |
| Speech threads | Physical cores, up to 8 (Balanced) or 16 (Maximum); was half the cores, at most 4 | Better use of large machines; whisper.cpp scales with physical cores. |
| whisper.cpp | One decoding state per job instead of per 120 s chunk; greedy decoding kept; flash attention off | Less work per chunk. Flash attention was measured slower and is off at every level. |
| 120 s chunks | Kept | They only bound memory. |
| Export encoder presets (x264/x265 "veryfast", SVT-AV1 preset 10) | Kept | Changing them changes quality or size: a product decision, not a speed bug. |
| Preview copies (H.264 "veryfast", quality 23, 960x540) | Kept | A faster preset would force every existing preview copy to be remade; later candidate. |
| Preview (80 ms debounce, latest frame only) | Kept | Already minimal. |
| Speech engine | whisper.cpp kept; larger models added as levels | See [subtitles](architecture-subtitles.md#subtitle-quality-levels-030). |
| Crates | None dropped, none added | None is slow or unmaintained. See [dependencies](architecture-dependencies.md). |

**Targets (quality goals).** At recommended settings, export and Fast subtitles at most 5% slower than 0.2.2 on the same machine; start-up at most 0.5 s slower. Measured by comparing 0.2.2 and 0.3.0 on the same computer; the figures are in the developer documentation, not here.

## Hardware video encoding (not in 0.3.0)
Using the graphics card to encode exports (REQ-055) is planned, not built.
- The bundled `ffmpeg` builds do contain NVIDIA, AMD, Intel and (on Linux) VA-API encoders, and macOS has VideoToolbox.
- Not built because: CI machines have no graphics card, so only the fallback could be tested; each vendor needs its own quality tuning to match the software encoder; driver bugs can produce damaged frames while reporting success, which an automatic fallback cannot catch.
- Planned design: a short background check of each encoder (3 frames, at most 1 s each, 3 s in total), hardware first with an automatic software re-run on failure, and a plain "Export: Faster / Best quality" choice.

## Risks
- Speeds and memory of the larger speech models are estimates; thresholds may change after measurement.
- On virtual machines or containers whose core count changes between starts, the computer looks "different": recommended settings are recomputed silently, custom ones only produce a notice.
- Flash attention was measured slower on the 0.3.0 reference computer and is off; it may be worth measuring again on other processors.
