# Business — Continuous quality, cache and project page (2.0.4)

Back to the [index](business.md). The continuous items of the [roadmap tasks](roadmap_tasks.md), done alongside the batches above. Most are engineering goals: they are measured and reported, with no feature test and no `covers` (like QG-01 to QG-10 in [Settings, performance and safety](business-performance.md#quality-goals)). The few that users see are requirements.
## Preview copies and cache
- REQ-296 (implemented): After **Clear preview copies**, the copies of the open project are made again in the background at once, not only after reopening the project or changing Preview quality. Amends REQ-038 and the known limitation of 0.3.0. Acceptance: clear the copies with a project open: new copies appear in the background without reopening, and playback uses them when ready.
- REQ-297 (implemented): The preview copy cache has a size limit (MINOR default 4 GB) shown in Settings, with the oldest copies removed when it is above the limit. Copies of the open project are never removed. Amends REQ-038 (the cache was never cleaned before). Acceptance: with a limit of 1 GB and 1.5 GB of copies, the oldest copies not used by the open project are removed; the open project's copies stay; the size shown matches the disk.

## Project page
- REQ-298 (implemented): The project page has a social preview image (Open Graph `og:image`, a still PNG) so that a shared link shows a picture. Acceptance: the page's head names the image and the image file exists in the page's published files.
- REQ-299 (implemented): The project page has a Content-Security-Policy that allows only the hosts the page uses (its own, and the hosts its badges and download links redirect to). Acceptance: the page's policy is present in its head and every resource the page loads is allowed by it (checked in the page's test).

## Quality goals and engineering items (QG-11..13 measured in 2.0.4; QG-14..18 built in 2.0.4; no feature tests)
- **QG-11 Export and preview speed** (batch 10, measured): export time of 4K and HEVC presets at 1080p and 4K; export time of large frames (the 4096 px cap is an assumption to measure); export time of the blurred fill; Clean audio cost in the preview on a weak computer (lighter preview values only if playback underruns; the export keeps the tuned values). Results go to the developer documentation. Not a release gate.
- **QG-12 360 and camera measurements**: the 360 export on 8K x 4K material; the moving-camera export on the bundled ffmpeg 9.0.2 (not yet measured); 4K and 8K frame times of the preview and of the 360 view against REQ-104; the instant look-around (REQ-267) if needed. The GPU or SIMD path for reprojection is only started if REQ-104 is missed.
- **QG-13 Apply LUT cost**: measured on real clips with a 33-point and a 65-point LUT (preview within 20% of playback without a LUT; export at most 1.5 times the re-encode without one, assumptions).
- **QG-14 Test run and automatic build time** (built): a committed speech fixture (`tests/fixtures/align/speech.mkv`) replaces espeak-ng in the CI packages; the 360 test picture of the integration tests is 1024 by 512; measured: the cycle check takes 1.7 s, the pinned ffmpeg is 173 MB unpacked. The test job time (QG-06) is read from the first CI run after this change.
- **QG-15 Test quality** (built): pointer tests of the colour picker and of the Export dialog's dropdowns; the keyframe bar's empty click; golden argument tests for project formats 4 and 5 (`tests/fixtures/export-argv-v4-v5-in-2.0.4.txt`); a large-picture memory test. Deviation: the egui_kittest interface harness is not adopted.
- **QG-16 Architecture debt** (built; no behaviour change): the render and application modules are split by area; `text.rs` is split into `text/{timing,layout,events}.rs`; one line box rule (`placement::padded_line_rect`) is shared by the preview and the placement warnings; the module-cycle check passes.
- **QG-17 Splash and release items** (built): a clear build message for a missing splash summary; `scripts/version.sh set` runs the notices check as a reminder after a version bump; the player's notice file has no version line; the test memory cap is a rule in the developer documentation. The two splash pictures (88 KB each) stay in the library, unchanged in size.
- **QG-18 Follow-ups of 2.0.2 and 2.0.3** (built): the UI frame gates are sampled as the preview tests are (best of up to five, limits unchanged); the hardware probe success fakes have long limits under load; the test font's licence is in the notices (also REQ-187). The accuracy numbers of 2.0.3 are kept in the [subtitle accuracy](business-subtitle-accuracy.md) document.

## Key decisions (2.0.4)
- [MINOR] The cache limit (REQ-297) is a default that Andrea can change; copies of the open project are protected.
- [MINOR] The page's image and policy (REQ-298, REQ-299) follow the project page rules of the release process; they are checked at each release.

## Out of scope (2.0.4)
- Measurements that need Andrea's hardware are in [verification](business-verification.md), not here.
