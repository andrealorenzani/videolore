# Business — Continuous quality, cache and project page (2.0.4)

Back to the [index](business.md). The continuous items of the [roadmap tasks](roadmap_tasks.md), done alongside the batches above. Most are engineering goals: they are measured and reported, with no feature test and no `covers` (like QG-01 to QG-10 in [Settings, performance and safety](business-performance.md#quality-goals)). The few that users see are requirements.
## Preview copies and cache
- REQ-296 (implemented): After **Clear preview copies**, the copies of the open project are made again in the background at once, not only after reopening the project or changing Preview quality. Amends REQ-038 and the known limitation of 0.3.0. Acceptance: clear the copies with a project open: new copies appear in the background without reopening, and playback uses them when ready.
- REQ-297 (implemented): The preview copy cache has a size limit (MINOR default 4 GB) shown in Settings, with the oldest copies removed when it is above the limit. Copies of the open project are never removed. Amends REQ-038 (the cache was never cleaned before). Acceptance: with a limit of 1 GB and 1.5 GB of copies, the oldest copies not used by the open project are removed; the open project's copies stay; the size shown matches the disk.

## Project page
- REQ-298 (implemented): The project page has a social preview image (Open Graph `og:image`, a still PNG) so that a shared link shows a picture. Acceptance: the page's head names the image and the image file exists in the page's published files.
- REQ-299 (implemented): The project page has a Content-Security-Policy that allows only the hosts the page uses (its own, and the hosts its badges and download links redirect to). Acceptance: the page's policy is present in its head and every resource the page loads is allowed by it (checked in the page's test).

## Quality goals and engineering items (QG-11..13 measured in 2.0.4; QG-14..18 planned for 2.0.5; no feature tests)
- **QG-11 Export and preview speed** (batch 10, measured): export time of 4K and HEVC presets at 1080p and 4K; export time of large frames (the 4096 px cap is an assumption to measure); export time of the blurred fill; Clean audio cost in the preview on a weak computer (lighter preview values only if playback underruns; the export keeps the tuned values). Results go to the developer documentation. Not a release gate.
- **QG-12 360 and camera measurements**: the 360 export on 8K x 4K material; the moving-camera export on the bundled ffmpeg 9.0.2 (not yet measured); 4K and 8K frame times of the preview and of the 360 view against REQ-104; the instant look-around (REQ-267) if needed. The GPU or SIMD path for reprojection is only started if REQ-104 is missed.
- **QG-13 Apply LUT cost**: measured on real clips with a 33-point and a 65-point LUT (preview within 20% of playback without a LUT; export at most 1.5 times the re-encode without one, assumptions).
- **QG-14 Test run and automatic build time**: keep the test suite faster (QG-03); shorter 360 pictures in the integration tests; a speech fixture for CI tests in place of espeak-ng where it is equivalent; measure the pinned ffmpeg download and the cycle check's CI cost; the test job time reported as QG-06.
- **QG-15 Test quality**: pointer tests of the colour picker and the keyframe timeline; clicks on the Export dialog's dropdowns in the interface tests; golden argument tests for project format 4 and 5 exports; a large-file memory test (a run on a large file, not a few minutes); the interface test harness (egui_kittest) when the helper grows.
- **QG-16 Architecture debt** (engineering; no behaviour change): split the application module (`app.rs`), the render module and `dispatch_inner` by area; one keyframe module for the 360 camera and the picture (before REQ-268 to REQ-271 and REQ-200 change them); split the text module; one line box rule for the preview's hit boxes and the measured box; the module-cycle note and the wording of the "leaves" layer rechecked; a shared core crate only if a third program is planned.
- **QG-17 Splash and release items**: the splash picture size in each program (move the player's picture into its crate if the size matters); a clearer build message for a missing splash summary; a smaller project page image set (WebP or AV1 only after browser support is checked); the notices script run after each version bump; the player's notice file version line decided; a package list for the player only if it is published on crates.io; the test memory cap written as a development rule.
- **QG-18 Follow-ups of 2.0.2 and 2.0.3**: the other UI frame gates sampled as the preview tests are (best of up to five, limits unchanged); the hardware probe success fakes given long limits under load; the test font licence recorded in the notices (also REQ-187); the measured numbers of the 2.0.3 accuracy work kept with the subtitle accuracy document.

## Key decisions (2.0.4)
- [MINOR] The cache limit (REQ-297) is a default that Andrea can change; copies of the open project are protected.
- [MINOR] The page's image and policy (REQ-298, REQ-299) follow the project page rules of the release process; they are checked at each release.

## Out of scope (2.0.4)
- Measurements that need Andrea's hardware are in [verification](business-verification.md), not here.
