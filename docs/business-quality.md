# Business — Continuous quality, cache and project page

Back to the [index](business.md). Most continuous items are engineering goals: measured and reported, with no feature test (like QG-01 to QG-10 in [Settings, performance and safety](business-performance.md#quality-goals)). The few that users see are requirements.

## Preview copies and cache
- REQ-296 (implemented): After **Clear preview copies**, the copies of the open project are made again in the background at once, not only after reopening the project or changing Preview quality. Amends REQ-038. Acceptance: clear the copies with a project open: new copies appear in the background without reopening, and playback uses them when ready.
- REQ-297 (implemented): The preview copy cache has a size limit (MINOR default 4 GB) shown in Settings, with the oldest copies removed when it is above the limit. Copies of the open project are never removed. Amends REQ-038. Acceptance: with a limit of 1 GB and 1.5 GB of copies, the oldest copies not used by the open project are removed; the open project's copies stay; the size shown matches the disk.

## Project page
- REQ-298 (implemented): The project page has a social preview image (Open Graph `og:image`, a still PNG) so that a shared link shows a picture. Acceptance: the page's head names the image and the image file exists in the page's published files.
- REQ-299 (implemented): The project page has a Content-Security-Policy that allows only the hosts the page uses (its own, and the hosts its badges and download links redirect to). Acceptance: the page's policy is present in its head and every resource the page loads is allowed by it (checked in the page's test).

## Quality goals and engineering items (2.0.4; no feature tests)
- **QG-11 Export and preview speed** (measured): export time of 4K and HEVC presets; large frames (the 4096 px cap is an assumption); the blurred fill; Clean audio cost in the preview on a weak computer. Not a release gate.
- **QG-12 360 and camera measurements**: the 360 export on 8K x 4K material; the moving-camera export on the bundled ffmpeg (not yet measured); 4K and 8K frame times against REQ-104.
- **QG-13 Apply LUT cost**: a 33-point and a 65-point LUT, preview within 20% of playback without a LUT and export at most 1.5 times the re-encode (assumptions).
- **QG-14 Test run and automatic build time** (built): a committed speech fixture (`tests/fixtures/align/speech.mkv`) replaces espeak-ng in the CI packages; the 360 test picture is 1024 by 512; the cycle check takes 1.7 s; the pinned ffmpeg is 173 MB unpacked.
- **QG-15 Test quality** (built): pointer tests of the colour picker and of the Export dialog's dropdowns; golden argument tests for project formats 4 and 5; a large-picture memory test. The egui_kittest interface harness is not adopted.
- **QG-16 Architecture debt** (built; no behaviour change): the render and application modules are split by area; the text module is split by timing, layout and events; one line box rule is shared by the preview and the placement warnings.
- **QG-17 Splash and release items** (built): a clear build message for a missing splash summary; the version script runs the notices check as a reminder; the player's notice file has no version line; the test memory cap is a rule in the developer documentation.
- **QG-18 Follow-ups of 2.0.2 and 2.0.3** (built): the UI frame gates are sampled as the preview tests are (best of up to five, limits unchanged); the hardware probe success fakes have long limits under load; the test font's licence is in the notices (REQ-187). The accuracy numbers are in [subtitle accuracy](business-subtitle-accuracy.md).

## Key decisions
- The cache limit (REQ-297) is a default that the maintainer can change; copies of the open project are protected.
- The page's image and policy (REQ-298, REQ-299) follow the project page rules of the release process; they are checked at each release.

## Out of scope
- Measurements that need the maintainer's hardware are in [verification](business-verification.md), not here.
