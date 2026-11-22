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

An unknown source bitrate, stream copy, or any other format (MPEG-4) always uses the normal
method. Since 2.0.4 the formats are H.264, HEVC, AV1 and VP9: the encoders are `<format>_<method>`
(`av1_nvenc`, `av1_qsv`, `av1_amf`, `av1_vaapi`, `av1_videotoolbox`, `vp9_vaapi`, `vp9_qsv`) and exist
only where the ffmpeg in use has them; AV1 and VP9 take no `-profile:v`. VP9 hardware is rare
(software is the usual answer). A preset has no bitrate of its own, so a hardware export of a
preset aims at `hwenc::preset_bitrate` (about 8 Mb/s for H.264 at 1920x1080 and 25 fps, in
proportion to the pixels per second; HEVC 0.6, VP9 0.65, AV1 0.5 of it). Quick Sync also has a
constant-quality variant (`-global_quality 23`, no bitrate) for drivers that refuse bitrate
control, with the same size guard (REQ-215).

## The hardware check (probe)

Runs only when needed: at start-up (after the ffmpeg check) if Export is Faster, when the user
picks Faster, on "Try video hardware again", on "Use recommended settings", and when the ffmpeg
changes while Faster is selected. With Best quality nothing runs. Each candidate encodes 8
synthetic 640×360 frames with the real export arguments to `-f null`, using one process slot
at probe priority and no network. Time limits (since 0.4.2), counted from the start of each
process (waiting for a slot does not count):

| Limit | Value | What happens |
|---|---|---|
| First try of a candidate | 1 s | if it does not answer in time it is **tried once more at once** (the next candidate waits, so the preference order never changes) |
| The retry | 4 s | if it still does not answer, the candidate is noted "did not answer in time" and the next one is tried |
| Whole check (hard bound) | 10 s | untried candidates are noted "did not answer in time"; a try cut short by this bound is not retried |

The retry exists because a busy computer (an export, speech recognition or a build using every
core) can make a working encoder start slowly; before 0.4.2 a single 1 s try then reported "no
hardware". The first working method per format wins, so the same working set always gives the
same choice. A VA-API driver that refuses bitrate control (a plain failure, not a missing
device) is tried again at constant quality, with the same first try + retry, within the same
total bound.

**A time-out is never a final "does not work".** If the check ends with a format that has no
working hardware and one of its candidates did not answer in time, the answer is inconclusive:
the app repeats the whole check once by itself, keeping the first answer in use (and on screen)
until the repeat ends. A time-out is never recorded in the "turned off after a problem" memory:
only a failed hardware export is. "Try video hardware again" starts a new check (with its own
possible repeat). Measured on the development laptop: the HEVC VA-API test encode of its Intel
driver runs about 14 s and then fails, so each check spends 5 s on it (about 6.4 s per check on
an idle machine, up to the 10 s bound under full CPU load) and is repeated once; H.264 stays
usable during the repeat.

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

### Changes of 2.0.4 (REQ-206 to 219)

The probe covers the four formats (H.264, HEVC, AV1, VP9) with the same limits (1 s, retry 4 s, 10 s in total) and keeps every working candidate (Automatic is the first of each format). Busy or hanging tries, the session memory, the pause during jobs, the decoders, the graphics card and the libva note: [code-features.md](code-features.md) § Hardware export and decoding.

## The export

1. The hardware run writes `.<name>.hw.videolore-partial` (private, registered for crash
   clean-up). Progress 0–85%. A watchdog reads `-progress` every 100 ms: cancel, or no progress
   for 30 s, kills the process (after a kill it waits at most 5 s, then leaves the process to a
   background thread, which keeps its slot until it really ends). The bar never goes back during
   a run (`Attempt::progress` keeps the highest value; ffmpeg's last time can be a little under an
   earlier one); it starts again from 0 only for the redo and for the second constant-quality run.
2. The output check (85–100%), all with software tools:
   - ffprobe metadata: codec, profile, `yuv420p`, size, frame rate, square pixels, `hvc1`,
     audio codec / 48 kHz / stereo, packet count;
   - a strict full decode (`-xerror`, no error output): one frame per packet;
   - (since 2.0.4, REQ-206) when the file has the frame count the plan gives (`render::expected_frames`,
     within one frame), three short runs of the normal method's graph, one per sample
     (`outcheck::sample_ranges`, `compare_range_args`: the graph from the start of the range, cut
     after one second, next to the file read from the same place). A sample that fails that way,
     or a file with another count (a clip whose length is not whole frames gets a frame more from
     the rounding of `fps`, which shifts every later frame), goes to the exact check:
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

- `render/golden_tests.rs`: Best quality runs exactly the argv of 0.3.0 (`export-argv-0.3.0.txt`, QG-01); every method changes only the video encoding.
- `hwenc/tests.rs`: order per system, encoder filtering, device enumeration, the argument sets (golden), the second-run quantiser, constant-quality fallback, old-libva skip, the probe with a fake ffmpeg (limits with processes that never end, late answers, cancel). `outcheck/tests.rs`: metadata rules, sample selection, PSNR parsing, the neighbour rule.
- `export/hw_tests.rs`: a simulated ffmpeg per method × error exit, crash, stall, green and dropped frames; cancel in the run, the check and the redo; a failing redo; constant quality too big. `app/hw_tests.rs`, `ui/tests.rs`: when the check runs, Best quality never touching the hardware, the failure memory, the single repeat, the Settings texts, responsiveness during a hardware export.
- `tests/ffmpeg_integration/hw.rs` (real ffmpeg): a runner stands in for each vendor with libx264/libx265 (vendor options removed, `hwupload` → `null`) and damages files on request; every method × H.264/HEVC gives the Best quality format; empty, truncated, missing-frame, wrong-size, green and garbage files are caught by their step; a rich project (60 uneven cuts, two frame rates, fades, letterbox, music, subtitles) passes; stream copy is byte-identical; the real check of the machine stays within 10 s (+1.5 s start-up) twice under load with the same picks and finds nothing on CI. `VIDEOLORE_REAL_HW=1` exports every method found for real.

## Real-hardware verification

The manual QG-05 checklist and its measured results are in
[code-hardware-qg05.md](code-hardware-qg05.md).

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
