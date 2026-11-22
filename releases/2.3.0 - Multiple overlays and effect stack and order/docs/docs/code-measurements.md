# Code — measurements

Back to the [index](code.md). Numbers measured on the development machine (i7-1065G7, 4 cores, Intel Iris Plus G7) unless said otherwise; ffmpeg is the pinned 9.0.2 build unless said otherwise.

## Rendering
- **Blurred fill size (REQ-222, 2.0.4).** 4K source in a 2160x3840 frame, 8 s, 3 runs: `BLUR_DIVISOR` 8 took 2.9-3.1 s, divisor 4 took 3.3-3.7 s (1080p source in 1080x1920: 1.4 s vs 1.5-1.6 s). Kept 8.
- **Blurred fill cost (REQ-314, 2.2.0).** `testsrc2` through `compose_frame` (9:16, Medium blur, 300 frames): 1080p 2.4-2.8 ms a frame plain, 4.1-4.8 ms with the blur; 4K 8.3-8.9 ms vs 10.7-11.2 ms. One frame at 25 fps is 40 ms: met 20 times over. Test `v22::the_blurred_fill_costs_less_than_one_frame_time_per_frame_at_1080p_and_4k`.
- **Order spikes (2.2.0).** On an RGBA layer, Scale then Move puts a test border at x=160, Move then Scale at x=200. A packed `rgba` `fade` also fades alpha (use `format=gbrap,fade`); `alphaextract` after `split` of rgba fails; libass keeps the alpha of an RGBA frame (hence the matte). Volume and Clean audio in the two orders: RMS -36.4 and -38.9 dB.
- **Overlap memory (2.3.0).** A 4000x4000 PNG overlay (`-loop 1`, scaled as the first filter) peaks at about 830 MB (one decode 156 MB; the plan assumed 64 MB). A 4 s still exports in 5.1 s (every looped frame decoded again); with `-framerate 1`: 0.9 s and 760 MB. Not changed (golden argv). Not measured: frame time with an overlay (estimate +10%), export time (estimate 1.5x).
- **Enlarge word (REQ-333).** Real libass render at 1080p: the word's centre within 4 px of its normal place (21 px allowed).

## LUTs
- The maintainer's files: 144-point cubes of 80,621,697 bytes (2,985,984 entries of 27 bytes). Before 2.1.0 they were refused by our limits (`MAX_3D` 65, `MAX_BYTES` 16 MiB), not by ffmpeg.
- `lut3d=…:interp=tetrahedral`: a 144-point cube starts in 2.3 s, ffmpeg peaks at 80 MB; a 256-point cube (453 MB of text) starts in 12.4 s, 233 MB (ffmpeg parses the text at every start).

## Hardware export (REQ-315, 2.2.0, system ffmpeg 6.1.1, VA-API H.264)
`hw::req315_phase_times` (ignored; `VIDEOLORE_FFMPEG=/usr/bin/ffmpeg VIDEOLORE_FFPROBE=/usr/bin/ffprobe cargo test --test ffmpeg_integration req315 -- --ignored --nocapture`): three cuts of a 1080p30 8 Mb/s clip (30 s) with a fade. Median of three:

| | Best quality (x264) | Faster, 2.1.0 check | Faster, concurrent check |
|---|---|---|---|
| total | 12.3 s | 12.2 s | 11.3 s (8.7% lower) |
| hardware encode | | 7.55 s | 7.5 s |
| strict decode | | 2.4 s | 2.0 s (with the samples) |
| three samples | | 2.25 s | concurrent; check 3.75 s in all |

Taken (ADR-218 lever a): `Attempt::check` runs `strict_decode` and `check_ranges` at once (one thread a sample, still through the runner's slots). Not taken: (b) the metadata pass is 0.1 s; (c) hardware decoding in the check is more forgiving. Target 20% **not met** (REQ-315 `measured-not-met`); the rest is the encode phase.

## Speech
- **Speech regions (REQ-177, 2.2.0).** `speech_regions.rs` (10 ms dB envelope, 10th-percentile floor, regions, word placement, a frozen grid of 81 settings, 75% eligibility). Best: 10 of 23 matched words within 0.3 s (43%; recognised times give 7 of 23; target 90%): `v12::speech_region_grid_on_the_reference_clip` (ignored). So `speech_regions::FROZEN` is `None` and `v12::alignment_matches_real_speech` stays ignored; `v22::the_frozen_speech_region_setting_is_checked_on_the_second_clip_when_it_exists` reads `tests/fixtures/align/second-clip-30s.wav` (SKIPPED while absent). REQ-177 stays planned.
- **Spatial sound pre-pass (REQ-273).** Debug build: 120 s of 4-channel sound in 1.7 s (69x real time, printed by `the_pre_pass_runs_far_faster_than_real_time`); a one-minute clip waits about 1 s before the first sound.

## Build and suite
- Cycle check (`tests/module_cycles.rs`) 1.7 s; the pinned ffmpeg and ffprobe are 173 MB each unpacked.
- Suite times: [code-testing.md](code-testing.md) § Suite size and time.
