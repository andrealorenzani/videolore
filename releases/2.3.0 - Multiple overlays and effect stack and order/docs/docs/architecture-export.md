# Architecture: export (dialog, formats, faster export with video hardware)

Part of [Architecture](architecture.md). Requirements: [export](business-export.md), [hardware export](business-hardware-export.md), [quality](business-quality.md). Engine: [engine](architecture-engine.md). Speed and settings: [performance](architecture-performance.md).

## Export dialog (1.4.0, REQ-118 to 122)
- **Export** (File > Export…, toolbar, Ctrl/Cmd+E) opens a modal window of the program (ADR-082); during an export the same action cancels it. Choices are kept for the session only.
- **Modes:** Full video; Only audio (the timeline's sound: video sound, music, volumes, Clean audio; no video decoded); Only subtitles (.srt, the File > Export subtitles text). Inapplicable fields are greyed with a reason ("No subtitles on the timeline" when there are none).
- **File:** one field for folder and name, with a "..." chooser; default: the project's name in the project's folder, else the session's last export folder, else the documents folder (assumption). The extension (.mp4, .mov, .m4a, .mp3, .srt) follows the format.
- **Disabled with the reason:** empty or invalid name; folder missing or unwritable; the target would replace a media file of the project or the project file. An existing file asks first. Name rules: no `/ \ : * ? " < > |`, no control characters, not `.` or `..`, no trailing dot or space, no reserved Windows name (CON, PRN, AUX, NUL, COM1 to 9, LPT1 to 9), at most 200 characters.
- Cancel writes nothing; partial files are written beside the target and renamed when complete.

### Video and audio formats
- **Video (Full video):** Same as source (REQ-022; stream copy when REQ-023 allows) or one of 12 fixed presets, always re-encoded, shown like "1280x720 25fps, libx264, .mp4":

| Size | Frame rate | Codec | File |
|---|---|---|---|
| 1920x1080 | 25, 30, 60 fps | libx264 | .mp4 |
| 1920x1080 | 25 fps | libx264 | .mov |
| 1920x1080 | 25, 30 fps | libx265 | .mp4 |
| 1280x720 | 25, 30 fps | libx264 | .mp4 |
| 1280x720 | 25 fps | libx265 | .mp4 |
| 854x480 | 25 fps | libx264 | .mp4 |
| 3840x2160 | 25 fps | libx265 | .mp4 |
| 3840x2160 | 30 fps | libx265 | .mov |

  2.0.4 added AV1 and VP9 presets (REQ-212, 213) with the encoders the bundled build lists; the codec name is plain and the encoder is in the tooltip (REQ-220). Pictures keep their shape (letterboxed); HEVC in MP4 or MOV gets `hvc1`. The list is fixed (documented limit).
- **Audio (Full video, Only audio):** Same as source (AAC), aac, or mp3 (offered only when the bundled ffmpeg lists libmp3lame). A chosen format re-encodes the audio; Only audio writes .m4a (AAC) or .mp3.
- **360 choice (1.4.0, ADR-085):** only when all video clips are 360: **360 video** (default, assumption) or **Flat camera view**; a mixed project is exported flat without a message. 2.0.4: the 360 video uses the speed-first preset, the fastest encoder preset whose file stays within 1.5 times the Best-quality size (Q-5, decided by Andrea) ([360](architecture-360.md)).
- A reset button sets Same as source.
- **What the choices do:** they become the same plan as before: Full video runs the worker; Only audio runs one ffmpeg process with the timeline's audio graph and no video branch; Only subtitles writes the SRT text. A preset or an audio format other than Same as source forces a re-encode (REQ-023), and so does Clean audio on any clip. The same job limits and cancel rules apply.

## Faster export with video hardware (0.4.0, REQ-055, 060 to 065)
With "Faster" the export may run on the computer's video hardware (H.264 and HEVC only): NVIDIA (NVENC), Intel Quick Sync, AMD (AMF on Windows, VA-API on Linux), Apple VideoToolbox. Anything going wrong redoes the export silently with the normal software method.

### When hardware is used
All must hold; otherwise the export is the 0.3.0 command, byte for byte (tested): Settings → Export is **Faster** (opt-in; the recommendation is Best quality); working hardware was found for the format by the background check; the output is H.264 or HEVC and needs re-encoding (stream copy is identical either way; AV1, VP9 and others use the normal method); the source bitrate is known (it sets the target); the bundled ffmpeg has `psnr`; the hardware was not turned off for that format after a problem.

### Order and encoder settings
| System | Order of preference |
|---|---|
| Windows x86_64 | NVIDIA (NVENC) > Intel (Quick Sync) > AMD (AMF) |
| Linux x86_64 | NVIDIA (NVENC) > Intel (Quick Sync) > VA-API, one candidate per `/dev/dri/renderD*` (number order, at most 4); AMD through VA-API |
| macOS Apple Silicon | VideoToolbox |

The order is fixed (the same hardware gives the same choice; NVIDIA first on multi-GPU computers). A candidate whose encoder is missing from `ffmpeg -encoders` is skipped at no cost. The bundled builds were checked at their pinned commits (2026-11-14); drivers are system components.

- Target bitrate T = source bitrate x vendor factor; cap = T x 5/4; buffer = 2T; profile High (H.264) or Main (HEVC); keyframe at least every 250 frames; B-frames left to the encoder.

| Hardware | Main settings | Picture format | Factor |
|---|---|---|---|
| NVIDIA | preset p5, high-quality tuning, VBR, spatial AQ | yuv420p | 100 % |
| Intel Quick Sync | preset medium | NV12 | 100 % |
| AMD (Windows) | transcoding usage, quality mode, peak-constrained VBR | NV12 | 120 % |
| VA-API (Linux) | VBR; frames uploaded to the device at the end of the graph | on device (NV12) | 110 % |
| VA-API, constant quality | CQP starting at 23, no bitrate options; for drivers refusing VBR | on device | – |
| Apple VideoToolbox | hardware only, not real-time | yuv420p | 120 % |

- The filter graph is unchanged (ends in yuv420p): letterboxing, fades, music and subtitles are identical; NV12 is the same 8-bit 4:2:0 picture. The cap is above the target because Intel and VA-API switch to constant bitrate when equal.
- Assumption: factors are estimates (expected files at most about 1.2 times the Best-quality size); to be tuned on real hardware (QG-05). Rejected: constant-quality modes for other vendors; AMD AMF on Linux (out of scope).
- **VA-API constant quality:** some drivers (Ubuntu's free Intel media driver 24.1) accept only a constant quantiser. Watch: after 2 s of video (or a tenth), if the file grows faster than 1.4 times the normal bitrate (video and audio), the run stops and restarts **once** with quantiser 23 + ceil(6 x log2(ratio / 1.3)), at most 40. Final: the output check refuses a CQP file over 1.5 times the usual size; the normal method then makes the file (not remembered as a hardware problem). No single quantiser met both the 3-VMAF-point and 1.5x limits on a real Intel laptop.

### Background hardware check
- **When:** at start-up if Export is already Faster; when switched to Faster; on "Try video hardware again"; on "Use recommended settings"; when the custom ffmpeg changes while Faster is set. With Best quality nothing runs (Advanced: "not checked (only checked when Export is Faster)").
- **How:** per format (H.264, HEVC) each candidate in order encodes 8 synthetic frames (640x360, generated in ffmpeg) with the real export settings until one works (typically 2 to 4 short runs). One process at a time, background priority, inside the process limit (REQ-033), no network. An export started before the check ends uses the normal method.
- **Limits (0.4.2; counted from each process start, waiting for a slot not counted):** first try of a candidate **1 s** (on time-out tried once more at once; the next candidate waits, so the order never changes); retry **4 s** ("did not answer in time", next candidate); whole check **10 s** (untried candidates noted; a cut-short try is not retried). Before 0.4.2: 1 s per candidate and 3 s in total.
- **Inconclusive answer:** when a format ends with no hardware and a candidate timed out, the whole check repeats once by itself; the first answer stays on screen until then (about 20 s of background work at most).
- **Time-outs are not failures:** never recorded in the "turned off" memory (REQ-064); only a failed hardware export is.
- **Constant-quality retry:** a VA-API device failing with VBR (not a missing device or encoder) is retried with CQP inside the same 10 s limit; Advanced shows "constant quality".
- **libva guard (Linux):** the bundled ffmpeg loads the system libva at run time and crashes if it is older than 2.21 (no `vaMapBuffer2`; Ubuntu 24.04 has 2.20). Before any check videolore reads the ffmpeg binary and the system `libva.so.2` as plain bytes (never loaded or run). If needed and missing: VA-API and Quick Sync are skipped without a process; Advanced says "needs a newer system video library (libva 2.21 or newer)". A system ffmpeg built against the system libva is not affected. Reading the 173 MB binary takes about 3 s, once per session (remembered by path, size and date), outside the 10 s limit.
- The result is kept for the session; a newer check replaces an older; late results are ignored. A too-old NVIDIA driver is recognised from its message.

### Export with hardware
1. Writes its own temporary file `.<name>.hw.videolore-partial` beside the destination (tracked for crash clean-up); progress 0 to 85 %; with CQP the size watch may restart once.
2. Watchdog every 250 ms: no new frame for **30 s** stops the process. Cancel takes effect within 1 s.
3. Successful exit → the **output check** (85 to 100 %) → pass: rename into place.
4. Any problem (error exit, crash, stall, failed check): the process is stopped (at most 5 s wait, then a background thread **keeps the slot until the process really ends**, so the limit holds with a hung driver), the partial is deleted and the export is **redone with the normal method**; the progress bar returns to zero once, with no message. A user cancel starts no redo.
5. If the normal method also fails: the 0.3.0 error; nothing remembered about hardware.
- Processes run one after another in the export slot; at most one partial file.

### Output check (REQ-063)
Every hardware result is checked with software tools before it appears:
- **File details (ffprobe):** codec and profile as planned, yuv420p, size, frame rate and pixel shape, `hvc1` for HEVC in MP4 or MOV, audio present, packet count; a CQP file over 1.5 times the usual size fails. Colour tags are not compared (labels, not damage).
- **Strict full decode** by the software decoder: any error, or a decoded frame count different from the packet count, fails (same 30 s stall rule).
- **Comparison with the normal graph:** one ffmpeg pass renders the normal video graph unencoded beside the hardware file. Frame count within ±1 of the graph and duration within 2 frames. At 10 %, 50 % and 90 % of frames, the frame and its two neighbours are compared with `psnr`; a sample passes when its best neighbour reaches **25 dB PSNR on brightness and both colour planes** (colour faults are the likeliest hardware failure). Frames above 1920x1080 are scaled down first.
- **Matching by position:** the encoder received exactly the graph's frames, so the n-th frame of the file is the n-th of the graph. The first design (timeline-derived counts, PSNR in Rust) was wrong in 15 of 30 real projects.
- Assumptions: good encodes score about 33 to 45 dB; garbage, green, wrong-size or colour-swapped frames below 15 to 20 dB.
- **Cost:** a full decode and a second render of the graph (about the normal pipeline's cost; on integrated graphics this cancels the saving). **Limit:** damage between samples is not caught (manual QG-05 checks).
- **2.0.4 (REQ-206 to 211, ADR-177):** the reference render covers at most three one-second ranges; busy and hanging are told apart by sample times; the check pauses while other jobs run; a skipped candidate is remembered for the session only.

### Remembering failures (REQ-064)
- After a hardware problem for a format, when the normal redo succeeded, later exports of that format skip hardware; Advanced shows "Video hardware turned off after a problem". Cancelling never counts.
- Per format and method ("HEVC on NVIDIA"); a different method later (new card or driver) is a change and hardware is tried again.
- Cleared when the computer changes (fingerprint), when videolore is updated, by "Use recommended settings" and by "Try video hardware again" (both also rerun the check).

### Settings and window
- Settings stay **version 2** with optional `export` (`best_quality` or `faster`) and `hardware_off` (format, method, program version per failure). A 0.3.x file loads as Best quality and is not rewritten (REQ-065). Unknown entries and entries over 32 characters are dropped one by one; at most 8 are kept. Version 3 would not open in 0.3.x after a downgrade.
- **Main section:** "Export: Faster / Best quality" with one-line explanations; "Faster" is never greyed (with no hardware its text says both give the same result); choosing it makes the status "Custom settings". Banned in the UI: "GPU" and "encoder" ("video hardware" is allowed).
- **Advanced:** for H.264 and HEVC which hardware works ("works", "none found", "checking…", "not checked", "turned off after a problem"); one plain line per problem (e.g. "NVIDIA: graphics driver too old (needs 610 or newer)"); the method of the last export in this session; the "Try video hardware again" button.
- Graphics card: Automatic or a device name from the enumerated list; a raw argument is never used (REQ-212 to 219).

### Performance (QG-05)
- Best quality is not slower than 0.3.0 (same commands, no check); Faster without working hardware costs nothing extra.
- **Target:** total time including the check at least 40 % lower than Best quality at 1080p.
- **Measured (0.4.0, Intel Iris Plus G7, Ubuntu 24.04, free Intel media driver 24.1, system ffmpeg 6.1.1, VA-API H.264 CQP, 3 projects):** quality and size pass (VMAF −1.4, +1.4, +6.7 against Best quality, never more than 3 below; size 0.96x, 1.32x, 1.38x). **Speed fails:** Faster −14 %, +41 %, +6 % against Best quality. HEVC through VA-API fails in that driver even at CQP, so HEVC uses the normal method. The 2.0.4 measurement changed nothing.
- For later: a speed target per class of hardware; a cheaper check (normal graph only around the samples, keeping the strict full decode); or decoding for the check in parallel with the export.

### Security
- The check uses synthetic input; the libva test reads only the ffmpeg binary and `libva.so.2` as bytes from known places. The VA-API device name is built only from character devices named `renderD<number>`; no shell.
- Hardware output is untrusted media, decoded only by the software decoder. No new crate, no network. A driver crash or hang stays in the ffmpeg process, which is stopped and left.

### Testing
- Each vendor path runs with a **simulated hardware encoder** (the CI ffmpeg with the software encoder swapped in) and injected faults: empty, truncated, missing frames, wrong size, green, garbage, error exit, crash, stall. Each ends with a correct normal-method file, no error shown and no leftover file; a correct hardware file passes. Real software exports check for false alarms.
- The real check runs twice against the CI ffmpeg: each within 10 s (plus 1.5 s start-up), same answer, no usable hardware. Simulated tests cover the first try, the retry, the whole-check limit, a slow-then-working candidate (also CQP) and the single repeat.
- Manual per release (QG-05): Windows with NVIDIA, Intel, AMD; Linux with NVIDIA (proprietary), Intel, AMD; macOS Apple Silicon: a reference project in H.264 and HEVC with both choices (hardware used, identical format and frame count, `hvc1` playback on Apple devices, no visible damage, at most 3 VMAF points below Best quality, size at most 1.5x, speed, a forced failure).

### Known limitations
- **NVIDIA needs driver 610 or newer** (bundled build: Video Codec SDK 13.1); older drivers fail fast, the normal method is used. Common on long-term-support Linux.
- **Linux:** VA-API and Quick Sync need the system libva and the vendor driver (Intel media driver, or Mesa for AMD). **Fedora's default Mesa cannot encode H.264/HEVC**; AMD users need RPM Fusion's `mesa-va-drivers-freeworld`. Quick Sync uses the default device only.
- **libva older than 2.21 (Ubuntu 24.04):** with the bundled ffmpeg Intel and AMD hardware is not used (no crash); the system ffmpeg in Settings → Advanced can reach it.
- **Constant-quantiser drivers:** size held only by the restart and the 1.5x check. HEVC through VA-API does not work with Ubuntu's free Intel driver 24.1.
- Out of scope: hardware for preview copies, playback, effects or decoding (except the 2.0.4 preview decode); hardware AV1 and VP9 speed; choosing a card; AMD AMF on Linux; Intel Macs; ARM Windows and Linux. Unverified combinations: [product documentation](business-export.md).
- Risks: [risks](architecture-risks.md#technical-risks).
