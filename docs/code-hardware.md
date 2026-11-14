# Code — hardware export ("Export: Faster")

With **Export: Faster** (an opt-in choice; Best quality is the default and the recommendation),
an export that has to be re-encoded to H.264 or HEVC runs on the computer's video hardware
when a working method was found, and every result is checked before it is published. Anything
that goes wrong ends with the normal method making the file, without an error message.

## Modules

| Module | Responsibility |
|---|---|
| `hwenc` | Candidates per system in a fixed order, encoder arguments per method, VA-API render devices, the background hardware check (`probe`, `probe_all`) |
| `outcheck` | The output check: ffprobe metadata rules, the strict decode, the comparison pass against the normal method's graph, the PSNR judgement |
| `export` | The hardware attempt (own partial file, stall watchdog, check, publish) and the silent redo; `ExportMethod` says how a file was made |
| `render` | `export_args(…, hw)`: `None` gives exactly the 0.3.0 argv (frozen by a golden file); `Some` adds the device setup, the graph tail, the label and the hardware video arguments; `check_graph` (video-only graph of the export) |
| `settings` | `ExportChoice` (fourth plain choice), `hardware_off` (methods turned off after a problem) |
| `app` | When the check runs, what an export may use (`hw_offer`), recording problems, `SetExport`, `TryHardwareAgain` |
| `ui::settings` | The Export choice and its note in the main section; hardware lines, last export method and "Try video hardware again" in Advanced |

## Methods and arguments

Order: Windows NVIDIA > Intel Quick Sync > AMD; Linux NVIDIA > Intel Quick Sync > VA-API (each
`/dev/dri/renderD*` character device in number order, at most 4); macOS VideoToolbox. A method
whose encoder is missing from the ffmpeg in use (`-encoders`) is skipped without running
anything, so a user-chosen ffmpeg is handled the same way.

The hardware arguments replace only the software video arguments; inputs, filter graph,
audio, container and thread limits stay the same. `T` is the source bitrate times the
method's factor, the cap is `T×5/4`, the buffer `2T`; every method uses `-g 250` and profile
High (H.264) or Main (HEVC); HEVC in MP4/MOV is tagged `hvc1`.

| Method | Extra options | Picture format | Factor |
|---|---|---|---|
| NVIDIA (`*_nvenc`) | `-preset p5 -tune hq -rc vbr -spatial_aq 1` | `yuv420p` | 100% |
| Intel Quick Sync (`*_qsv`) | `-preset medium` | `nv12` | 100% |
| AMD, Windows (`*_amf`) | `-usage transcoding -quality quality -rc vbr_peak` | `nv12` | 120% |
| VA-API, Linux (`*_vaapi`) | `-rc_mode VBR`; device `-init_hw_device vaapi=va:<node> -filter_hw_device va`, graph tail `[outv]format=nv12,hwupload[outvhw]` | on the device | 110% |
| VA-API, constant quality | `-rc_mode CQP -qp 23` (no bitrate options), same device setup and tail; only when the driver refuses `VBR` | on the device | – |
| Apple VideoToolbox (`*_videotoolbox`) | `-allow_sw 0 -realtime 0` | `yuv420p` | 120% |

**Constant quality (VA-API only).** Some drivers (for example Ubuntu's free Intel media
driver) accept only a constant quantiser. The check then tries the same device with
`-rc_mode CQP`. The first quantiser is 23 (measured below: never more than 3 VMAF points under
Best quality). Because a constant quantiser cannot follow the source bitrate, the size is
watched: once 2 s (or a tenth) of the video is written, a run growing faster than 1.4× the
normal method's bitrate (video + audio) is stopped and restarted once with a coarser
quantiser (about 6 steps halve the size, aiming at 1.3×, at most 40). The output check also
refuses a constant-quality file larger than 1.5× the usual size; the normal method then makes
the file, and this is not remembered as a hardware problem (it depends on the video).

An unknown source bitrate, stream copy, or any other format (VP9, AV1, MPEG-4) always uses the
normal method.

## The hardware check (probe)

Runs only when needed: at start-up (after the ffmpeg check) if Export is Faster, when the user
picks Faster, on "Try video hardware again", on "Use recommended settings", and when the ffmpeg
changes while Faster is selected. With Best quality nothing runs. Each candidate encodes 8
synthetic 640×360 frames with the real export arguments to `-f null`: at most 1 s each (from
the start of its process), at most 3 s in total (untried ones then count as not working), one
process slot at probe priority, no network. The first working method per format wins, so the
same working set always gives the same choice. A VA-API driver that refuses bitrate control
(a plain failure, not a missing device) is tried again at constant quality within the same
time budget.

**Old system libva.** The bundled Linux ffmpeg loads libva at run time and aborts (core dump)
when the system library lacks `vaMapBuffer2` (libva 2.21+; Ubuntu 24.04 ships 2.20). Before
probing, videolore reads (as plain bytes, never loading it) whether the ffmpeg in use names
that function and whether the system `libva.so.2` (searched in `LD_LIBRARY_PATH` and the usual
library folders) has it. If the ffmpeg needs it and the library is missing or older, VA-API
and Quick Sync are not started at all and Advanced says "needs a newer system video library
(libva 2.21 or newer)". A system ffmpeg linked to the system libva is not affected. The
170 MB bundled binary is read once per session (remembered by path, size and date). Results last for the session; a newer check
replaces an older one (a generation number ignores late results). An export started before the
check ends uses the normal method. The hardware is also never used when the ffmpeg lacks the
`psnr` filter the output check needs.

## The export

1. The hardware run writes `.<name>.hw.videolore-partial` (private, registered for crash
   clean-up). Progress 0–85%. A watchdog reads `-progress` every 100 ms: cancel, or no progress
   for 30 s, kills the process (after a kill it waits at most 5 s, then leaves the process to a
   background thread, which keeps its slot until it really ends).
2. The output check (85–100%), all with software tools:
   - ffprobe metadata: codec, profile, `yuv420p`, size, frame rate, square pixels, `hvc1`,
     audio codec / 48 kHz / stereo, packet count;
   - a strict full decode (`-xerror`, no error output): one frame per packet;
   - one pass that renders the normal method's video graph (no encoding) next to the file: the
     graph's frame count must equal the file's (±1) and its duration the file's (within 2 frames),
     and 3 samples (10%, 50%, 90% of the frames,
     each with its two neighbours) are compared frame by frame with ffmpeg's `psnr` filter. A
     sample passes when one of its neighbours reaches 25 dB on Y, U and V. Frames are matched by
     their position in the stream (the encoder received exactly the graph's frames), so clips
     that are not whole frames long, many cuts and variable frame rate sources need no timing
     model. Pictures above 1920×1080 are compared scaled down (same filter on both sides).
3. Pass: the file is made public and renamed to the destination. Any failure (error exit,
   crash, stall, failed check): the hardware partial is deleted, the progress bar goes back to 0
   once and the normal method runs into the usual partial file. If that fails too, the export
   fails exactly as before and nothing is remembered.

## Failure memory

After a hardware problem whose redo succeeded, the app saves
`{format, method, version}` in `hardware_off` (at most 8 entries, validated one by one when
read: an unknown entry is dropped, the rest of the file stays valid). That format then goes
straight to the normal method while the check still picks the same method. It is forgotten when
the computer changes (fingerprint), when the program version changes, on "Use recommended
settings" and on "Try video hardware again"; a different method winning the check counts as
new hardware. A cancelled export is never a problem. The settings file stays at format 2:
`export` and `hardware_off` are optional fields, so a 0.3.x file loads as Best quality and
0.3.x can still read a 0.4 file.

## Tests

- `render/golden_tests.rs`: the Best quality argv of 6 export plans is compared with
  `tests/fixtures/export-argv-0.3.0.txt`, captured from 0.3.0 before any change (QG-01: Best
  quality runs exactly the same commands); every method changes only the video encoding.
- `hwenc/tests.rs`: order per system, encoder filtering, render-device enumeration, the 10
  argument sets and the constant-quality ones (golden), the second-run quantiser, the
  constant-quality fallback of the probe, the old-libva skip (pure rule and file scan across
  read boundaries), the probe with a fake ffmpeg (first working method, notes and reasons,
  per-candidate and total time limits with processes that never end, cancel, chosen ffmpeg).
- `outcheck/tests.rs`: metadata rules on ffprobe JSON, sample selection, PSNR log parsing and the
  neighbour rule.
- `export/hw_tests.rs`: a simulated ffmpeg for each of the 5 methods × error exit, crash, stall,
  green frames, dropped frames; cancel during the run, the check and the redo; a failing redo;
  constant quality growing too big (one restart with a coarser quantiser; still too big: the
  normal method).
- `app/hw_tests.rs` and `ui/tests.rs`: when the check runs, Best quality never touching the
  hardware, the failure memory and its resets, the Settings window texts, and the
  responsiveness harness with a hardware export running.
- `tests/ffmpeg_integration/hw.rs` (real ffmpeg): a test runner stands in for each vendor by
  running the real command with libx264/libx265 (vendor options and device setup removed,
  `hwupload` replaced by `null`) and then damages the file on request. All 5 methods × H.264 and
  HEVC give the Best quality format; empty, truncated, missing-frame, wrong-size, green and
  garbage files are each caught by the step meant for them on the NVENC and VA-API paths;
  a rich project (60 uneven cuts, two frame rates, fades, letterbox, music, subtitles) passes
  the check; stream copy is byte-identical with either choice; the real check of the machine
  stays within its time budget and finds nothing on CI. With `VIDEOLORE_REAL_HW=1`, every
  method the check finds on the machine is exported for real and must pass the check.

## QG-05: real-hardware verification (manual)

CI has no graphics hardware. Before a release, on each system of the matrix, export a reference
project (mixed formats with letterbox, 50+ cuts, fades, music, subtitles) in H.264 and HEVC with
Faster and with Best quality. The measuring helper does the exports, a lossless reference and
the VMAF scores (the bundled ffmpeg has libvmaf):

```
VIDEOLORE_QG05_PROJECT=reference.vlproj VIDEOLORE_QG05_OUT=qg05-out \
  cargo test --release --test ffmpeg_integration qg05 -- --ignored --nocapture
```

Checklist per system:

- a. Settings → Advanced shows the hardware for each format and "Last export: video hardware (…)";
- b. format and frame count identical to Best quality;
- c. HEVC carries `hvc1` and plays in QuickTime and on an iPhone;
- d. full-screen viewing: no blocking, banding, colour shift or green frames;
- e. VMAF at most 3 points below Best quality;
- f. file at most 1.5× the Best quality size;
- g. total time including the check at least 40% lower than Best quality at 1080p (assumption,
  adjustable after measurement);
- h. with the driver disabled or unavailable, the export still produces a correct file with the
  normal method.

### Results

Constant-quality calibration (same laptop, `h264_vaapi -rc_mode CQP` against libx264
veryfast at the source bitrate, 20 s of each video, VMAF against a lossless copy of the
source). The quantiser 23 is the start value; the second run handles the cases marked "too big".

| Source | QP 18 | QP 21 | QP 24 | QP 27 | QP 30 |
|---|---|---|---|---|---|
| 1080×1920, 5.6 Mb/s | 2.63×, −2.6 | 1.68×, −2.1 | 1.14×, −1.3 | 0.83×, −0.3 | 0.60×, +1.5 |
| 1920×1080, 4.4 Mb/s | 0.61×, +0.6 | 0.45×, +1.7 | 0.36×, +2.9 | 0.28×, +5.5 | 0.20×, +9.4 |
| 1280×720, 3.3 Mb/s | 4.29×, −5.9 | 2.78×, −5.3 | 1.87×, −4.0 (too big) | 1.35×, −2.4 | 0.97×, +0.2 |
| 1024×576, 0.9 Mb/s | 6.26×, −16.7 | 4.03×, −15.8 | 2.73×, −14.1 (too big) | 2.03×, −12.2 | 1.46×, −8.8 |
| 3840×2160, 21.7 Mb/s | 2.81×, −3.6 | 1.77×, −2.8 | 1.18×, −1.3 | 0.85×, +0.2 | 0.60×, +2.8 |

Each cell: size relative to Best quality, VMAF difference (Best − constant quality; QG-05 e
needs ≤ 3). No single quantiser or bits-per-pixel rule meets both limits on all five, hence
the start at 23 plus the size watch.

| Version | System | Method | a | b | c | d | e (VMAF Δ) | f (size) | g (time) | h | Notes |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0.4.0 | Windows + NVIDIA | NVENC | – | – | – | – | – | – | – | – | not verified yet |
| 0.4.0 | Windows + Intel | Quick Sync | – | – | – | – | – | – | – | – | not verified yet |
| 0.4.0 | Windows + AMD | AMF | – | – | – | – | – | – | – | – | not verified yet |
| 0.4.0 | Linux + NVIDIA (proprietary driver) | NVENC | – | – | – | – | – | – | – | – | not verified yet |
| 0.4.0 | Linux + Intel (Ubuntu 24.04, Iris Plus G7, free `intel-media-va-driver` 24.1, system ffmpeg 6.1.1) | VA-API H.264, constant quality | yes ("VA-API (renderD128, constant quality) — works") | yes (3 projects) | – (HEVC: driver fails, normal method used) | not checked by eye | −1.4 / +1.4 / +6.7 (Faster − Best) | 0.96× / 1.32× / 1.38× | 13.0 vs 15.1 s (−14%), 28.0 vs 19.9 s (+41%), 6.0 vs 5.6 s (+6%): **fails** | yes (with the bundled ffmpeg VA-API and Quick Sync are skipped: libva 2.20 too old; the normal method is used, no crash) | Projects from real phone/camera videos: 1080×1920 5.6 Mb/s (8 cuts, 16 s); 1280×720 3.3 Mb/s + 1080p 4.4 Mb/s (20 cuts, 60 s, the first run grew too big and was restarted with a coarser quantiser); 1024×576 0.9 Mb/s (15 cuts, 30 s). VA-API refuses `-rc_mode VBR`, so constant quality is used. On this integrated GPU the hardware encode plus the output check (strict decode + graph render) is not faster than x264 veryfast. |
| 0.4.0 | Linux + AMD | VA-API | – | – | – | – | – | – | – | – | not verified yet |
| 0.4.0 | macOS Apple Silicon | VideoToolbox | – | – | – | – | – | – | – | – | not verified yet |

## Known limitations

- NVIDIA needs graphics driver 610 or newer (the bundled ffmpeg is built with Video Codec SDK
  13.1); with an older driver the check fails quickly and Advanced says why.
- Linux VA-API and Quick Sync use the system's libva and vendor driver. Fedora's default Mesa
  has no H.264/HEVC encoding (AMD users need RPM Fusion `mesa-va-drivers-freeworld`). Drivers
  that only offer constant-QP encoding (for example Ubuntu's free Intel media driver) are used
  at constant quality, whose size is only held by the restart and the 1.5× check. The bundled
  Linux ffmpeg needs libva 2.21 or newer; with an older libva (Ubuntu 24.04 ships 2.20) its
  VA-API and Quick Sync are skipped and only the normal method is used (a system ffmpeg, chosen
  in Settings → Advanced, can reach the hardware there).
- Quick Sync has no constant-quality fallback.
- Quick Sync on Linux uses the default render device; an Intel GPU on another device is reached
  through VA-API. Machines with several GPUs always prefer NVIDIA.
- The check compares 3 samples: damage limited to frames between the samples is not detected
  (the strict decode and the frame count still are).
- A driver that hangs the whole system cannot be prevented; a hung or crashed encoder process is
  killed and left behind.
