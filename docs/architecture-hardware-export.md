# Architecture: faster export with video hardware

Part of [Architecture](architecture.md). Since 0.4.0, `videolore` can use the computer's video hardware to export: NVIDIA, Intel Quick Sync, AMD, Linux VA-API and Apple VideoToolbox (REQ-055, REQ-060..065). If anything goes wrong, it silently redoes the export with the normal software method.

## When the hardware is used
All of these must be true. Otherwise the export runs exactly as in 0.3.0.
- **Settings → Export is "Faster".** This is opt-in: the default and the recommendation on every computer is "Best quality".
- The background check has finished and found working hardware for the output format.
- The output is **H.264 or HEVC** and the export needs re-encoding. Stream-copy exports are byte-identical with either choice. AV1, VP9 and other formats always use the normal method.
- The source bitrate is known (it almost always is); it sets the size target for every method, including the VA-API constant-quality variant below.
- The `ffmpeg` in use has the `psnr` filter, which the output check needs.
- The hardware has not been turned off for that format after an earlier problem (see [Remembering failures](#remembering-failures)).

With "Best quality", the program builds the same `ffmpeg` command as 0.3.0, byte for byte (checked by a test against the 0.3.0 commands), and runs no hardware check at all.

## Supported hardware
| System | Order of preference |
|---|---|
| Windows x86_64 | NVIDIA (NVENC) > Intel (Quick Sync) > AMD (AMF) |
| Linux x86_64 | NVIDIA (NVENC) > Intel (Quick Sync) > VA-API, one candidate per graphics device (`/dev/dri/renderD*`, in number order, at most 4). AMD on Linux is reached through VA-API. |
| macOS Apple Silicon | VideoToolbox |

The order is fixed, so the same set of working hardware always gives the same choice. On computers with several graphics cards NVIDIA is always preferred. A candidate whose encoder is missing from `ffmpeg -encoders` is skipped at no cost, so a custom `ffmpeg` (REQ-030) is checked the same way.

The encoders come from the bundled `ffmpeg` builds, which were checked at their pinned build-script commits (2026-11-14): the Linux and Windows builds enable NVIDIA, AMD, Intel (oneVPL) and, on Linux, VA-API; the macOS build includes VideoToolbox. The release smoke test confirms each encoder is really present. Graphics drivers are system components: nothing new is bundled.

## Encoder settings
The normal method aims at the source bitrate with a cap. The hardware aims at the same average bitrate, so files come out about the same size.

T = source bitrate x a per-vendor factor; cap = T x 5/4; buffer = 2T; profile High (H.264) or Main (HEVC); one keyframe at least every 250 frames (the software encoders' default; without it some hardware makes much larger files). B-frames are left to each encoder, because forcing them breaks older graphics cards.

| Hardware | Main settings | Picture format | Factor |
|---|---|---|---|
| NVIDIA | preset p5, high-quality tuning, variable bitrate, spatial adaptive quantisation | yuv420p | 100% |
| Intel Quick Sync | preset medium | NV12 | 100% |
| AMD (Windows) | transcoding usage, quality mode, peak-constrained variable bitrate | NV12 | 120% |
| VA-API (Linux) | variable bitrate; frames uploaded to the chosen device at the end of the filter graph | on device (NV12) | 110% |
| VA-API, constant quality | constant quantiser (`-rc_mode CQP`, starting at 23), no bitrate options; same device setup; only for drivers that refuse variable bitrate | on device (NV12) | – |
| Apple VideoToolbox | hardware only (no software fallback inside the encoder), not real-time | yuv420p | 120% |

- The filter graph is unchanged and still ends in yuv420p, so letterboxing, fades, music and burned-in subtitles are identical. HEVC in MP4/MOV still gets the `hvc1` tag. Audio settings do not change. "Light" still limits the threads of the software filtering.
- NV12 holds the same 8-bit 4:2:0 picture as yuv420p; the output check confirms the result reads as yuv420p.
- The cap is above the target because Intel and VA-API switch to constant bitrate when they are equal, which lowers quality.
- **Assumption:** the factors are estimates (AMD, Apple and VA-API are expected to give slightly less quality per bit than the software encoder; NVIDIA and Intel about the same). Expected files at most about 1.2x the Best quality size. They will be tuned from the real-hardware measurements (QG-05).
- Alternatives rejected: constant-quality modes for the other vendors (no evidence they match the software quality setting, larger size risk); AMD AMF on Linux (out of scope).

### VA-API constant quality
Some Linux drivers accept only a constant quantiser; Ubuntu's free Intel media driver (24.1) refuses variable bitrate. For these, VA-API is used with a constant quantiser, and the size is held by two rules:
- **Watch and restart:** once 2 s of video (or a tenth of it, whichever comes first) has been written, if the file is growing faster than 1.4x the normal method's bitrate (video plus audio), the run is stopped and restarted **once** with a coarser quantiser: 23 + ceil(6 x log2(ratio / 1.3)), at most 40 (about 6 steps halve the size; the aim is 1.3x).
- **Final size check:** the output check refuses a constant-quality file larger than 1.5x the usual size. The normal method then makes the file. This is **not** remembered as a hardware problem, because it depends on the video, not the hardware.

Calibration on a real Intel laptop showed that no single quantiser (and no bits-per-pixel rule) meets both the quality limit (at most 3 VMAF points below) and the size limit (at most 1.5x) for every kind of video; the measurements are in the [developer documentation](code-hardware.md). Hence a start value that kept quality in every case measured, plus the size rules. Quick Sync and the other vendors have no constant-quality variant.

## Background hardware check
- **When:** only when it can matter: at start-up if Export is already "Faster", when the user switches to "Faster", on "Try video hardware again", on "Use recommended settings", and when the custom `ffmpeg` changes while "Faster" is selected. With "Best quality" nothing runs, and Advanced shows "not checked (only checked when Export is Faster)".
- **Why not at every start:** most users keep "Best quality"; checking would wake the discrete graphics card on hybrid laptops and use up to 20 s of background work for nothing.
- **How:** for each format (H.264, HEVC), each candidate in order encodes 8 synthetic frames (640x360, generated inside `ffmpeg`, no file read) with exactly the settings of a real export, until one works. Typically 2 to 4 short runs.
- **Limits (since 0.4.2):** times count from the start of each process (waiting for a free process slot does not count).

  | Limit | Value | What happens |
  |---|---|---|
  | First try of a candidate | 1 s | if it does not answer in time, it is tried once more straight away (the next candidate waits, so the preference order never changes) |
  | Retry | 4 s | if it still does not answer, it is noted "did not answer in time" and the next candidate is tried |
  | Whole check | 10 s | untried candidates are noted "did not answer in time"; a try cut short by this limit is not retried |

  One process at a time, at background priority, within the process limit (REQ-033); no network. An export started before the check finishes uses the normal method.
- **Why the retry (0.4.2):** before 0.4.2 the limits were 1 s per candidate and 3 s in total. On a busy computer (an export, subtitle generation or a build using every core) a working encoder can take longer than 1 s to start, so the check reported "no hardware" at random. A time-out is therefore never a final "does not work".
- **Inconclusive answer, one repeat:** if a format ends with no working hardware and one of its candidates did not answer in time, the answer is inconclusive. `videolore` then repeats the whole check once by itself; the first answer stays in use (and on screen) until the repeat finishes. At most one repeat per request, so at most about 20 s of background work. A new request (for example "Try video hardware again") starts a fresh check, which may again be repeated once.
- **Time-outs are not failures:** a candidate that did not answer in time is never recorded in the "turned off after a problem" memory (REQ-064); only a failed hardware export is.
- **Constant-quality retry:** a VA-API device that fails with variable bitrate (an ordinary failure, not a missing device or encoder) is tried again with the constant-quality variant, with the same first try and retry, inside the same 10 s limit. Advanced then shows the method as "constant quality".
- **Old system video library (Linux):** the bundled Linux `ffmpeg` loads the system's libva at run time and crashes (core dump) if libva is older than 2.21 (it lacks the function `vaMapBuffer2`). Ubuntu 24.04 ships 2.20. Before any check, `videolore` reads the `ffmpeg` file and the system `libva.so.2` (from `LD_LIBRARY_PATH` and the usual library folders) as plain bytes, never loading or running them, to see whether `ffmpeg` needs that function and the library has it. If not, VA-API and Quick Sync are skipped without starting any process, and Advanced says the hardware "needs a newer system video library (libva 2.21 or newer)". A system `ffmpeg` built against the system libva is not affected.
- Reading the 173 MB bundled binary takes about 3 s; it runs in the background once per session (remembered by path, size and date) and is not part of the 10 s limit, which counts only the check processes.
- The result is kept only for the session, so driver changes are noticed at the next start. A newer check replaces an older one; late results of an older check are ignored. A too-old NVIDIA driver is recognised from its message and shown in Advanced.

## Export with hardware
1. The hardware export writes to its own temporary file (`.<name>.hw.videolore-partial`) in the destination folder, tracked for crash clean-up like other partial files. Progress covers 0–85%. With VA-API constant quality, the size watch above may restart the run once.
2. A watchdog checks progress at least every 250 ms. If no new frame is written for **30 s**, the process is stopped. Cancel takes effect within 1 s.
3. If `ffmpeg` exits successfully, the **output check** runs (85–100%). If it passes, the file is renamed into place.
4. On any problem (error exit, crash, stall, failed check) the hardware process is stopped (the program waits at most 5 s for it, then leaves it to a background thread that **keeps its process slot until it really ends**, so the process limit holds even with a hung driver), its partial file is deleted, and the export is **redone with the normal method**. The progress bar goes back to zero once, with no message. If the user cancelled, no redo starts.
5. If the normal method also fails, the error is reported exactly as in 0.3.0 and nothing is remembered about the hardware (it was not the cause).

Processes run one after another, each in the export slot; there is at most one partial file at a time.

## Output check
Hardware can report success while writing damaged frames, so every hardware result is checked with software tools before it appears at its destination (REQ-063):
- **File details** (`ffprobe`): codec and profile as planned, picture format yuv420p, size, frame rate and pixel shape as planned, `hvc1` tag for HEVC in MP4/MOV, audio present as planned, packet count. For constant-quality VA-API, a file larger than 1.5x the usual size fails. Colour tags are not compared: they are labels, not damage, and comparing them would wrongly turn off a working encoder.
- **Strict full decode:** the whole file is decoded by the software decoder; any error, or a decoded frame count different from the packet count, fails the check. The same 30 s stall rule applies.
- **Comparison with the normal pipeline:** one `ffmpeg` pass renders the normal method's video graph (unencoded) next to the hardware file. The file must have the same number of frames as the graph (±1) and the same duration (within 2 frames). At 10%, 50% and 90% of the frames, the frame and its two neighbours are compared with `ffmpeg`'s `psnr` filter; each sample passes when its best neighbour reaches **25 dB PSNR on brightness and on both colour planes**. Checking colour planes matters because colour-plane faults are the most likely hardware failure. Frames larger than 1920x1080 are scaled down on both sides first.
- **Why frames are matched by position:** the encoder received exactly the graph's frames, so the n-th frame of the file is the n-th frame of the graph. The first design computed the expected frame count and sample times from the timeline and compared against separately rendered stills, with PSNR computed in Rust; on real projects that count was wrong in 15 of 30 cases (clips that are not a whole number of frames long, many cuts, variable frame rate sources). Matching by position needs no timing model.
- **Assumptions:** good encodes are expected to score about 33–45 dB and green, garbage, wrong-size or colour-swapped frames below 15–20 dB; the threshold may move once more real values are recorded.
- **Cost:** the check decodes the whole file and renders the whole graph a second time, so it costs roughly as much as the normal pipeline's decoding and filtering. On an integrated graphics chip this cancels the time saved by the hardware encoder (see [Performance](#performance)).
- **Limit:** damage only on frames between the samples is not caught; the manual real-hardware checks (QG-05) cover this.

## Remembering failures
- After a hardware problem for a format, when the normal redo succeeded, later exports in that format skip the hardware (REQ-064). Advanced shows "Video hardware turned off after a problem". Cancelling never counts as a failure.
- The memory is per format and per method (for example "HEVC on NVIDIA"). If the check later picks a different method (new graphics card or driver), that counts as a change and the hardware is tried again.
- It is cleared when the computer changes (fingerprint), when `videolore` is updated to a new version, by "Use recommended settings", and by "Try video hardware again" in Advanced. The last two also rerun the check.

## Settings file
- Stays at **version 2** with two optional fields: `export` (`best_quality` or `faster`) and `hardware_off` (format, method and program version for each remembered failure).
- A 0.3.x file without these fields loads as "Best quality" with no notice and is not rewritten, so its recommended/custom status is unchanged (REQ-065). Files from 0.2.x and damaged files get "Best quality" too. Because 0.3.x ignores unknown fields, a 0.4 file still opens in 0.3.x after a downgrade; a version 3 file would not.
- Remembered failures with an unknown format or method, or longer than 32 characters, are dropped one by one (the file is not treated as damaged); at most 8 are kept.

## Settings window
- **Main section:** "Export: Faster / Best quality" with one-line explanations. "Faster" is never greyed out; when the check found no usable hardware for either format, its explanation adds that both choices give the same result. Choosing "Faster" makes the status "Custom settings". The banned-word test still applies (no "GPU", "encoder" and so on; "video hardware" is allowed).
- **Advanced:** for H.264 and HEVC, which video hardware works ("works", "none found", "checking…", "not checked", "turned off after a problem"); one plain line per problem found (for example "NVIDIA: graphics driver too old (needs 610 or newer)"); the method used by the last export in this session (video hardware, normal method, normal method after a video hardware problem, or copied without re-encoding); and the "Try video hardware again" button.

## Performance
- **Best quality is not slower than 0.3.0:** same commands, no check, no extra start-up work. "Faster" on a computer without working hardware costs nothing extra either.
- **Target (QG-05):** total export time including the check at least 40% lower than Best quality at 1080p. Hardware speeds up only the encoding, while decoding and filtering often limit 1080p exports, and the check adds a second decode and graph render.
- **Measured (0.4.0, Intel Iris Plus G7 laptop, Ubuntu 24.04, free Intel media driver 24.1, system `ffmpeg` 6.1.1, VA-API H.264 at constant quality, 3 real projects):** quality and size pass (VMAF −1.4 / +1.4 / +6.7 relative to Best quality, i.e. never more than 3 below; size 0.96x / 1.32x / 1.38x). **Speed fails:** "Faster" took −14%, +41% and +6% time compared with "Best quality". On this integrated chip, hardware encoding plus the check is not faster than the software encoder's `veryfast` preset. HEVC through VA-API fails in that driver even at constant quality, so HEVC uses the normal method.
- **Recommendation for a later version:** either re-set the speed target per class of hardware (for example dedicated graphics cards versus integrated chips), or make the check cheaper by rendering the normal graph only for short segments around the three samples instead of the whole export (keeping the strict full decode). Decoding for the check in parallel with the export is another option.

## Testing
- CI has no graphics hardware. Every vendor path is tested with a **simulated hardware encoder**: the real CI `ffmpeg` with the hardware encoder swapped for the software one, plus faults injected on purpose (empty file, truncated file, missing frames, wrong size, green frames, garbage frames, error exit, crash, stall). Each must end with a correct normal-method file, no error shown and no leftover temporary file; a correct hardware file must pass.
- The check is also run on real software exports of the existing test media to make sure it raises no false alarms.
- The real check runs twice against the CI `ffmpeg`: each run must finish within the 10 s limit (plus 1.5 s for process start-up), both must give the same answer, and they must find no usable hardware. Simulated tests cover the first try, the retry, the whole-check limit, a candidate that is slow once and then works (also at constant quality), and the single repeat of an inconclusive answer.
- **Manual, per release (QG-05):** Windows with NVIDIA, Intel and AMD; Linux with NVIDIA (proprietary driver), Intel and AMD; macOS Apple Silicon. A reference project is exported in H.264 and HEVC with both choices and checked for hardware use, identical format and frame count, `hvc1` playback on Apple devices, visible damage, quality (at most 3 VMAF points below Best quality, measured against a lossless render with the bundled `ffmpeg`'s VMAF), size (at most 1.5x), speed, and a forced failure. Results are recorded in the [developer documentation](code.md).

## Security
- The check uses synthetic input and reads no media files. The libva compatibility test only reads the `ffmpeg` binary and the system `libva.so.2` as bytes, from known places; it never loads or runs them. The VA-API device name is built only from device files found on the system whose names match `renderD<number>` and that are character devices; no shell is involved.
- The hardware output is treated as untrusted media and decoded only by the software decoder, like an imported file.
- No new crates and no network access. A driver crash or hang stays inside the `ffmpeg` process, which is stopped and left behind; a hang of the whole system inside a driver cannot be prevented.

## Known limitations
- **NVIDIA needs graphics driver 610 or newer** (the bundled build uses Video Codec SDK 13.1). With an older driver the check fails quickly, the normal method is used, and Advanced explains why. This may be common on long-term-support Linux systems.
- **Linux:** VA-API and Quick Sync need the system's libva and the vendor driver (Intel media driver, or Mesa for AMD). **Fedora's default Mesa cannot encode H.264/HEVC**; AMD users there need RPM Fusion's `mesa-va-drivers-freeworld`. Quick Sync uses the default graphics device only; an Intel GPU on another device is reached through VA-API.
- **Ubuntu 24.04 and other systems with libva older than 2.21:** the bundled `ffmpeg` cannot use VA-API or Quick Sync there, so **with the bundled `ffmpeg`, Intel and AMD hardware is not used at all**; exports use the normal method, without crashing. Choosing the system `ffmpeg` in Settings → Advanced can reach the hardware.
- **Drivers that only offer a constant quantiser** (for example Ubuntu's free Intel media driver) are used at constant quality; size is held only by the restart and the 1.5x check, and quality depends on the video. Quick Sync has no constant-quality variant. HEVC through VA-API does not work with Ubuntu's free Intel media driver 24.1.
- **Speed:** on integrated Intel graphics "Faster" was measured as not faster than "Best quality" (see [Performance](#performance)).
- Combinations not yet verified on real hardware are listed as known limitations in the [product documentation](business-export.md).
- Out of scope: hardware for preview copies, playback, effects or decoding; hardware AV1/VP9; choosing a specific graphics card; AMD AMF on Linux; Intel Macs and ARM Windows/Linux.

## Risks
- Encoder factors and the 25 dB threshold are assumptions until QG-05 is measured on every vendor; so far only Intel VA-API (constant quality) has been measured.
- The 40% speed target was not met on integrated Intel graphics and may not be met elsewhere at 1080p; it should be re-set or the check made cheaper.
- Constant quality: a restart with a coarser quantiser may lower quality more than 3 VMAF points on videos unlike those measured.
- Reading the bundled `ffmpeg` for the libva check takes about 3 s in the background, which delays the first check result in a session.
- **Slow failing candidates cost more since 0.4.2:** a candidate that is slow and then fails now costs 5 s (first try plus retry) instead of 1 s, and makes the answer inconclusive, so the check is always repeated once. Measured on the development laptop (Intel Iris Plus G7, Ubuntu 24.04, system `ffmpeg`): the HEVC VA-API test encode runs about 14 s and then fails, so each check takes about 6.4 s when the computer is idle and up to 10 s under load, and is always repeated (about 13–20 s of background work). H.264 hardware is usable from the first answer.
- The check cannot tell a busy computer from a candidate that hangs; under extreme load it can still end with "no hardware" after its repeat.
- Damage limited to frames between the samples passes the automatic check.
- Whether each hardware encoder is present in each bundled build is confirmed only by the release smoke test.
