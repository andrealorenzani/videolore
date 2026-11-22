# Architecture: open risks and open decisions

Part of [Architecture](architecture.md). Each entry names its area document. Product decisions that need Andrea are marked **decision**; technical items are marked **risk**.

## Measured targets not met (2.3.0)
- **REQ-177, subtitle timing on real speech (NOT MET, 2.2.0):** 43 % of matched words within 0.3 s against 90 %; the frozen-setting path is not used (`FROZEN = None`); a second speech clip (about 30 s, Andrea's) is needed before the check; a pace-aware method is needed. **Decision:** accept, rework, or drop the target. [subtitles](architecture-subtitles.md).
- **REQ-315, Faster export on integrated graphics (NOT MET, 2.2.0):** 8.7 % lower against 20 %; the rest is the hardware encode phase. Levers (b) and (c) are not taken. **Decision:** re-set the target per class of hardware, or make the check cheaper. [performance](architecture-performance.md), [export](architecture-export.md).
- **QG-05 export speed at 1080p (NOT MET, 0.4.0):** −14 % to +41 % against a 40 % target on Intel Iris Plus G7; quality and size pass. [export](architecture-export.md).
- **Camera export speed (MEASURED):** about one third of real time or less at 1080p; target of half real time not met; accepted. [360](architecture-360.md).

## Open product decisions
- **Blur divisor (REQ-222):** the requirement names a quarter; the build uses an eighth (measured faster). **Decision:** keep the eighth and change the requirement wording, or revert. [performance](architecture-performance.md).
- **Picture items limits (ADR-192):** the plan's 20 items, 8 MiB file and 16 megapixels are not enforced by the code (only the header is read). **Decision:** enforce or drop the limits. [text](architecture-text.md), [NFR](architecture-nfr.md).
- **Default 360 choice in export:** 360 video (assumption). [export](architecture-export.md).
- **Speed-first 360 preset:** the fastest preset within 1.5 times the Best-quality size is built; its name is not recorded in the notes. [360](architecture-360.md).
- **Unsigned installers:** postponed (decided by Andrea); signing is a later decision. [packaging](architecture-packaging.md).
- **Subtitle quality defaults and memory thresholds:** confirmed 2026-11-14 as assumptions; to adjust after measurements on real machines. [performance](architecture-performance.md).
- **Old processors:** no slower engine (decided 2026-11-14). [subtitles](architecture-subtitles.md).
- **Container and no-metadata 360 files:** a file without spherical information stays flat (since 1.4.0); a choice at import is a roadmap item. [360](architecture-360.md).
- **Boxed shadow design (2.1.0):** design A (box colour from `OutlineColour`, a spike) or design B (vector box behind the text); the choice follows the spike. [subtitles](architecture-subtitles.md).
- **Enlarge word:** the in-house layout must match libass at 1080 px (spike). [subtitles](architecture-subtitles.md).

## Technical risks
- **Bundled ffmpeg 9.x not checked for:** time-dependent Scale and Move sizes (accepted by 6.1.1 in the spike), `lut3d` options, `reset_rot` and the 360 commands, the camera timings. The 2.0.2 run repeated the camera timings on 9.0.2. [picture](architecture-picture.md), [360](architecture-360.md), [build](architecture-build.md).
- **Hardware paths:** encoder factors and the 25 dB threshold are assumptions until QG-05 is measured on every vendor; only Intel VA-API (CQP) measured; the output check protects files, not speed; damage between samples passes; a hung driver can stop the whole system (not preventable). NVIDIA needs driver 610 or newer; libva older than 2.21 blocks VA-API and Quick Sync with the bundled ffmpeg (Ubuntu 24.04). [export](architecture-export.md).
- **Slow failing candidates:** the HEVC VA-API test takes about 14 s then fails; each check takes about 6.4 s idle and up to 10 s under load, repeated once (13 to 20 s of background work). [export](architecture-export.md).
- **Speech tooling:** the C++ build of whisper.cpp on three platforms slows CI and can break on toolchain changes; `SOURCE_DATE_EPOCH` set silently selects the slow baseline. [subtitles](architecture-subtitles.md), [build](architecture-build.md).
- **Speech speed and memory:** larger models' speed and memory are estimates; "Most accurate" needs about 1.3 GB more (estimate). The word-timing setting may use more memory (measured value to record). [subtitles](architecture-subtitles.md).
- **Word timing:** whisper word times are accurate to about 0.1 to 0.3 s (assumption); paraphrased tracks fall back to evenly spread words. [subtitles](architecture-subtitles.md).
- **Fonts:** libass and fontdb may name a family differently (silent fallback); CJK depends on system fonts; a best-fit estimate may differ by 85 to 115 % (the font-glyph fit of 2.0.3 reduces it). [text](architecture-text.md).
- **Colour:** the LUT round trip moves values by one or two levels; no colour management; LUTs made for other workflows look different. [picture](architecture-picture.md).
- **Overlay media:** a MOV without alpha is drawn opaque; a MOV overlay can slow the preview; the preview of files above 1920x1080 is softer than the export; no keyframes, rotation or own track. [picture](architecture-picture.md).
- **Clean audio and Enhance speech:** level normalisation only (`speechnorm`), not a neural noise model; the acceptance gate is measured on one clip. [effects](architecture-effects.md).
- **Applied effects order:** a LUT of a changed-order clip grades the whole frame including bars; legacy stacking must stay byte-identical (gate); a filter step per row adds graph size. [effects](architecture-effects.md).
- **Keyframes and text:** keyframes in a small popup are harder to use; texts do not follow Scale or Move. [picture](architecture-picture.md), [text](architecture-text.md).
- **Spatial sound:** real-time CPU cost on a weak computer unmeasured; the stereo pair does not follow roll; the preview waits for the pre-pass; four channels assumed (W, Y, Z, X). [engine](architecture-engine.md).
- **Picture subtitles (player):** a subtitle already on screen when a run starts inside it appears at its next change; none on 360; hand-made test fixtures. [player](architecture-player.md).
- **Platform behaviour checked only by hand:** first launch of unsigned installers, double-click on macOS, Linux file managers, the Windows player "Open with" template (not run on Windows), the macOS menu bar, Wayland drag and drop (not supported). [packaging](architecture-packaging.md), [UI](architecture-ui.md).
- **CI:** pinned ffmpeg downloads depend on the download site; the macOS build host's URL retention is unknown (the release build fails loudly, then re-pin); fixed runner versions will be retired; the best-of-five timing rule may hide an intermittent slowdown. [build](architecture-build.md), [packaging](architecture-packaging.md).
- **Licences:** the GPL source-offer approach for ffmpeg, and the splash pictures under MIT, have not been reviewed by a lawyer. Patent licensing of H.264 and HEVC varies by country (notice). [engine](architecture-engine.md), [security](architecture-security.md).
- **Settings on virtual machines:** a container whose core count changes between starts gets recommended settings recomputed silently. [performance](architecture-performance.md).
- **Code debt:** `app.rs` (about 2,100 lines) and `render` are still large; the cycle check is syntactic (a cycle through a macro is not seen); the blurred fill and the Enlarge layout depend on spikes. [components](architecture-components.md).
- **Accessibility:** AT-SPI stack size on Linux to be measured; screen-reader support is not enabled in the application's own tests. [UI](architecture-ui.md).
- **Preview copies:** the cache limit runs on the UI thread (estimate under 100 ms for 1000 copies); a faster preset for preview copies would remake every copy (later candidate). [engine](architecture-engine.md), [performance](architecture-performance.md).
- **Changes to the project format:** each development bumps the format once; older programs refuse newer files, so users need the newer program to open them (documented).
