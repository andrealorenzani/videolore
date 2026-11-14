# Roadmap

Known limitations and improvements identified so far, to consider for future versions. Grouped by area; each item says where it came from. Delivered items are removed and mentioned in the [changelog](changelog.md).

## Export
- **Make Faster really faster on integrated graphics** — on an Intel Iris Plus G7 the hardware export plus its safety check ranged from 14% quicker to 41% slower than Best quality. Make the check cheaper (render the normal picture only around the three sample frames, or decode in parallel with the export) and/or set the speed target per kind of graphics. (source: 0.4.0, QG-05 measurement)
- **Verify hardware export on every system** — only Linux with Intel graphics has been tried on real hardware; Windows (NVIDIA, Intel, AMD), Linux (NVIDIA, AMD) and macOS (Apple) still need the QG-05 checklist in [code-hardware](code-hardware.md). (source: 0.4.0, known limitation)
- **Hardware export on Ubuntu 24.04 with the included ffmpeg** — Intel/AMD hardware is skipped there because the system video library (libva 2.20) is older than the included ffmpeg needs (2.21). Options: an ffmpeg build that works with older libva, or shipping a compatible libva. (source: 0.4.0, known limitation)
- **Intel HEVC hardware** — broken on the tested Ubuntu Intel driver even at constant quality; revisit with newer drivers. (source: 0.4.0, known limitation)
- **Constant-quality mode for Quick Sync** — only VA-API has the constant-quality fallback; Quick Sync drivers that refuse bitrate control fall back to the normal method. (source: 0.4.0, implementation)
- **Skip a hardware candidate that timed out twice** — a candidate that keeps running out of time (for example HEVC VA-API on the tested Intel driver: about 14 s, then it fails) still costs 5 s and a repeat on every check (about 6.4 s idle, up to 10 s under load on that laptop). Skip it after two time-outs, in the automatic repeat or for the session. (source: 0.4.2, release verification of 0.4.1)
- **Tell a busy computer from a hanging candidate** — use the output produced before the time-out and the CPU time or load, so that only slow starts get the retry and the repeat. (source: 0.4.2, release verification of 0.4.1)
- **Limit the background work of the hardware check** — a check costs about 13–20 s per request, takes one process slot and may wake the discrete graphics card on hybrid laptops; pause or delay the repeat while an export or subtitle generation runs. (source: 0.4.2, release verification of 0.4.1)
- **Show the repeat in Settings → Advanced** — say "check inconclusive, repeating" while the automatic repeat runs. (source: 0.4.2, release verification of 0.4.1)
- **Under extreme load the check can still find no hardware** — after its repeat, a very busy computer can still end with "no hardware", so Faster uses the normal method. (source: 0.4.2, known limitation)

## Subtitles
- **Measure the larger speech levels** — speed and memory of Balanced and Most accurate are estimates; measure them and tune the recommendation thresholds. (source: 0.3.0, assumption)

## Performance
- **Faster test suite** — tests got 25% faster in 0.3.0 against a 30% goal; most of the remaining time is the two real speech-recognition tests. (source: 0.3.0, QG-03 result)
- **Preview copies with a lighter preset** — `ultrafast` would decode a little cheaper, but every existing preview copy would be regenerated. (source: 0.3.0, performance review)
- **Measured quality goals** — the numbers of the quality goals (detection time, regressions, speed) are assumptions to confirm with measurements. (source: 0.3.0, A10)

## Quality
- **Split the hardware developer document** — [code-hardware](code-hardware.md) is 237 lines and mixes the check, the export, the tests and the QG-05 results; split it into smaller documents. (source: 0.4.2, documentation size)

## Packaging
- **Code signing and notarisation** — installers are unsigned, so Windows and macOS warn on first launch. Postponed, not rejected. (source: 0.2.0, decision)
- **Detection in the installers** — the computer is set up at the first start of the app; detection could also run during installation. Open question. (source: 0.3.0, A1)
- **Old-processor check in CI** — run the tests under an emulated pre-AVX2 processor to prove the friendly message path. (source: 0.3.0, skipped item)
- **Weekly security-advisory check** — a scheduled CI run of the dependency advisory check, not only on pushes. (source: 0.3.0, skipped item)

## Not planned
- **Slower speech engine for old processors** — dropped on 2026-11-14: processors without AVX2 get a clear message and subtitle lines can still be added by hand.
