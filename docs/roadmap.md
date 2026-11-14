# Roadmap

Known limitations and improvements identified so far, to consider for future versions. Grouped by area; each item says where it came from. Delivered items are removed and mentioned in the [changelog](changelog.md).

## Export
- **Make Faster really faster on integrated graphics** — on an Intel Iris Plus G7 the hardware export plus its safety check ranged from 14% quicker to 41% slower than Best quality. Make the check cheaper (render the normal picture only around the three sample frames, or decode in parallel with the export) and/or set the speed target per kind of graphics. (source: 0.4.0, QG-05 measurement)
- **Verify hardware export on every system** — only Linux with Intel graphics has been tried on real hardware; Windows (NVIDIA, Intel, AMD), Linux (NVIDIA, AMD) and macOS (Apple) still need the QG-05 checklist in [code-hardware](code-hardware.md). (source: 0.4.0, known limitation)
- **Hardware export on Ubuntu 24.04 with the included ffmpeg** — Intel/AMD hardware is skipped there because the system video library (libva 2.20) is older than the included ffmpeg needs (2.21). Options: an ffmpeg build that works with older libva, or shipping a compatible libva. (source: 0.4.0, known limitation)
- **Intel HEVC hardware** — broken on the tested Ubuntu Intel driver even at constant quality; revisit with newer drivers. (source: 0.4.0, known limitation)
- **Constant-quality mode for Quick Sync** — only VA-API has the constant-quality fallback; Quick Sync drivers that refuse bitrate control fall back to the normal method. (source: 0.4.0, known limitation)
- **Choose the graphics card** — on computers with several graphics cards NVIDIA is always preferred and Quick Sync uses the default device; let the user pick the card. (source: 0.4.0, known limitation)
- **Hardware for more formats** — AV1 and VP9 encoding, and hardware decoding, are not used; preview copies and effects also stay on the normal method. (source: 0.4.0, scope decision)
- **Skip a hardware candidate that timed out twice** — a candidate that keeps running out of time (for example HEVC VA-API on the tested Intel driver: about 14 s, then it fails) still costs 5 s and a repeat on every check (about 6.4 s idle, up to 10 s under load on that laptop). Skip it after two time-outs, in the automatic repeat or for the session. (source: 0.4.2, release verification of 0.4.1)
- **Tell a busy computer from a hanging candidate** — use the output produced before the time-out and the CPU time or load, so that only slow starts get the retry and the repeat. (source: 0.4.2, release verification of 0.4.1)
- **Limit the background work of the hardware check** — a check costs about 13–20 s per request, takes one process slot and may wake the discrete graphics card on hybrid laptops; the repeat is not paused while an export or subtitle generation runs, so pause or delay it then. (source: 0.4.2, release verification of 0.4.1 and known limitation)
- **Show the repeat in Settings → Advanced** — say "check inconclusive, repeating" while the automatic repeat runs. (source: 0.4.2, release verification of 0.4.1)
- **Under extreme load the check can still find no hardware** — after its repeat, a very busy computer can still end with "no hardware", so Faster uses the normal method. (source: 0.4.2, known limitation)

## Subtitles
- **Measure the larger speech levels** — speed and memory of Balanced and Most accurate are estimates; measure them and tune the recommendation thresholds. (source: 0.3.0, assumption)
- **Subtitle fonts for other scripts** — Chinese, Japanese, Arabic, Hindi and similar scripts use whatever fonts the system has, so they may look different per computer or show empty boxes, and the subtitle editor cannot show them. Bundle fonts for these scripts. (source: 0.2.0, known limitation)
- **Speech in pieces without lost words** — speech is recognised in pieces of about two minutes, and a word cut at a boundary can be lost; overlap the pieces or join them at word boundaries. (source: 0.3.0, known limitation)

## Editing
- **Keep undo history when a file is removed** — removing a file from the media list drops the undo history when an undo step refers to it; keep the steps that do not depend on the file. (source: 0.2.0, known limitation)
- **One window for double-clicked projects** — on Windows and Linux a double-clicked project opens a second window instead of reusing the open one (macOS keeps one window). (source: 0.2.0, known limitation)
- **Hide and Minimize on macOS** — the standard menu items are switched off so that Cmd+Q asks about unsaved changes; restore them while keeping that prompt. (source: 0.2.0, known limitation)

## Performance
- **Faster test suite** — tests got 25% faster in 0.3.0 against a 30% goal; most of the remaining time is the two real speech-recognition tests. (source: 0.3.0, QG-03 result)
- **Preview copies with a lighter preset** — `ultrafast` would decode a little cheaper, but every existing preview copy would be regenerated. (source: 0.3.0, performance review)
- **Regenerate cleared preview copies at once** — cleared preview copies are made again only after reopening the project or changing Preview quality. (source: 0.3.0, known limitation)
- **Preview copies for 720p sources under Automatic** — a 1280x720 source never gets a preview copy in Automatic; check whether slower computers need one. (source: 0.2.0, known limitation)
- **Limit the preview copy cache** — the cache is never cleaned automatically; "Clear preview copies" is the only way to empty it. Add an age or size limit. (source: 0.2.0, known limitation)
- **Playback and export without waiting** — while an export and playback run together, other ffmpeg work (analysis, proxies) waits. (source: 0.3.0, known limitation)
- **Measured quality goals** — the numbers of the quality goals (detection time, regressions, speed) are assumptions to confirm with measurements. (source: 0.3.0, assumption)

## Quality
- **Check the installer configuration on every push** — install cargo-packager 0.11.8 in a Linux job, run `scripts/package.sh <target> --config-only` and build the deb and app formats from stub binaries, so packaging errors are not found only at release time. (source: 0.4.3, release of 0.4.2)
- **Split the hardware developer document** — [code-hardware](code-hardware.md) is 237 lines and mixes the check, the export, the tests and the QG-05 results; split it into smaller documents. (source: 0.4.2, documentation size)
- **Large-file memory test** — the memory acceptance test runs on a few minutes of video, not 4 GB; add a run on a large file. (source: 0.4.2, assumption in the product documentation)
- **Stable computer identity on virtual machines** — a virtual machine or container whose number of cores changes between starts is treated as a new computer; with custom settings this only shows a notice. (source: 0.3.0, known limitation)

## Packaging
- **Run the installer job before each release** — a manual run, or on pull requests that touch `scripts/package.sh`, `packaging/` or `assets/icon/`; the 0.4.2 macOS and Windows failures were found only at release time. (source: 0.4.3, release of 0.4.2)
- **Pin the Windows installer toolchain** — the NSIS toolchain that cargo-packager downloads on Windows should get a fixed version and SHA-256 in `packaging/tools.lock`, like the other downloads. (source: 0.4.3)
- **Licence page encoding** — the NSIS licence page shows `third-party.txt`; non-ASCII text without a byte-order mark may look garbled (cosmetic, not verified). (source: 0.4.3)
- **macOS and Windows installers for 0.4.2** — the 0.4.2 GitHub release has only the Linux installers; provide the macOS and Windows installers with the next release. (source: 0.4.3, release of 0.4.2)
- **Code signing and notarisation** — installers are unsigned, so Windows and macOS warn on first launch. Postponed, not rejected. (source: 0.2.0, decision)
- **Linux file managers and the archive** — some file managers do not start the program from the extracted `.tar.gz` on double-click; ship a fix or a launcher that works in them. (source: 0.2.0, known limitation)
- **Legal review of source offers** — the way the source code of bundled components is offered has not been reviewed by a lawyer. (source: 0.2.0, assumption)
- **Detection in the installers** — the computer is set up at the first start of the app; detection could also run during installation. Open question. (source: 0.3.0, open question)
- **Old-processor check in CI** — run the tests under an emulated pre-AVX2 processor to prove the friendly message path. (source: 0.3.0, skipped item)
- **Weekly security-advisory check** — a scheduled CI run of the dependency advisory check, not only on pushes. (source: 0.3.0, skipped item)
- **Other systems** — builds exist for Windows and Linux on x86-64 and for macOS on Apple Silicon only; Intel Macs and ARM Windows or Linux are not built. (source: 0.4.0, scope decision)

## Not planned
- **Slower speech engine for old processors** — dropped on 2026-11-14: processors without AVX2 get a clear message and subtitle lines can still be added by hand.
