# Code — hardware export: real-hardware verification (QG-05)

The manual check of "Export: Faster" on real graphics hardware, and its results. How the
hardware export works is in [code-hardware.md](code-hardware.md).

## Checklist

CI has no graphics hardware. Before a release, on each system of the matrix, export a reference
project (mixed formats with letterbox, 50+ cuts, fades, music, subtitles) in H.264 and HEVC with
Faster and with Best quality. The measuring helper does the exports, a lossless reference and
the VMAF scores (the bundled ffmpeg has libvmaf):

```
VIDEOLORE_QG05_PROJECT=reference.vlproj VIDEOLORE_QG05_OUT=qg05-out \
  cargo test --release --test ffmpeg_integration qg05 -- --ignored --nocapture
```

Since 2.0.4 (REQ-207) the time target is measured per kind of graphics: on a computer whose Faster uses integrated graphics (Intel Quick Sync, Apple silicon, an Intel VA-API device) and on one with a dedicated card, each against Best quality; the integrated result is what the Faster note in Settings is about (it says Faster may not be quicker there). The helper prints the method of each Faster export; the kind follows `Candidate::graphics`. Also by system: AV1 and VP9 where the check finds an encoder, and the device choice on a two-card computer (business-verification items 14 and 15). Nothing of this can be measured on CI or on the development computer, which has no graphics hardware the check finds.

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

## Results

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
