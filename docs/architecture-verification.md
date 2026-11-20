# Architecture: verifying the release (2.0.2)

Part of [Architecture](architecture.md). Planned for version 2.0.2 (the batch "Verify the release"). It changes no behaviour of the programs: it makes the tests and the automatic builds check the release the same way on every run. The manual checks are in [release verification](business-verification.md).

## Overview
- The tests run the same ffmpeg build that the installers ship (9.0.2, pinned in `packaging/ffmpeg.lock`), not the runner's package.
- Tests that measure time do not fail because the computer is busy: a product limit is checked on the best of several samples, and the product limits themselves do not change.
- The splash summaries are checked at the smallest allowed text size (7 px).
- Automatic builds check the notices, the cargo-about install and the installer configuration on every push.
- The 360 camera timing is measured again on the bundled build and recorded.

## Test ffmpeg (QG-07, QG-08)
- The CI check job no longer installs `ffmpeg` from apt. It downloads the linux-x86_64 build pinned in `packaging/ffmpeg.lock` with `scripts/fetch-ffmpeg.sh` (SHA-256 checked, smoke test) and caches it under the lock's hash.
- `VIDEOLORE_REQUIRE_FFMPEG=1` is set in CI, so a missing ffmpeg fails the job and does not skip the integration tests.
- The integration tests compare the version of the ffmpeg they find with the pinned version and fail on a difference.
- New 2.0.2 tests (`tests/ffmpeg_integration/v20.rs`): a paused frame at a fade is not black (REQ-144); a format 8 project with its LUTs reads and renders (REQ-137); the build offers the MP3 encoder and the frame, LUT and 360 filters. The frame, LUT, 360 marker and Clean audio tests of 1.5.0 to 1.6.0 run on this build unchanged.
- Rejected: an apt pin (Ubuntu's ffmpeg is older than 9.0.2); building ffmpeg from source (too slow for each run); keeping the runner's ffmpeg with wider limits (hides version changes).

## Splash summaries (REQ-154)
- The summary text starts at 15 px and shrinks to a floor of 7 px until it fits the popup above the Close button. It never scrolls and is never silently clipped.
- Tests: the editor and the player summaries fit at their chosen size and at 7 px; a summary that does not fit at 7 px fails the test.
- The 1.6.2 rule (14 px down to 10 px in 0.5 px steps) is replaced by this one; see [splash](architecture-splash.md#text-rules).

## Tests that do not depend on the load (QG-09)
- Logic is tested with injected limits and clocks. The hardware probe limits (1 s, 4 s retry, 10 s total, REQ-061) are a value type; their defaults are pinned by a test.
- Product gates measured in wall-clock time (the 200 ms interface frame of REQ-104, the camera stills, the start-up and load timings) pass when the best of up to five samples is under the limit. Every sample is printed when all of them fail. A real slowdown in every sample still fails.
- Waits for a result (preview frames, hardware checks) are harness timeouts, not product limits, and are longer (120 s).
- Hardware probe tests that expect a working candidate use seconds of limit; only the test of a candidate that never answers keeps a short limit, because it does not depend on speed.
- Rejected: raising product limits; ignoring the timing tests; a multiplier for slow machines (it would hide real regressions).

## Automatic builds (QG-10)
- Notices: `scripts/notices.sh --check` exits 0 when the committed notice files match the crates, 1 when one is out of date (with the difference shown), and cargo-about's own code on any other failure. CI runs it on every push. The 2.0.2 run checks the exit codes with a temporary edit that is reverted, and records the result here.
- cargo-about: installed by the pinned install action at version 0.9.3; a step checks that the installed tool reports `cargo-about 0.9.3`.
- Installer configuration, job `package-config` on every push and pull request, Ubuntu, no ffmpeg download:
  - Windows (NSIS) and macOS (app): `scripts/package.sh <target> --config-only` writes the packager configuration, which the job checks.
  - Linux (AppImage): `scripts/package.sh linux-x86_64 --stub` builds the AppDir from stub binaries and packs it with the pinned appimagetool; the job checks the desktop entry, icon, AppRun and the notices folder. The stub output is never uploaded.
  - There is no `.deb` package and none is planned (decided by Andrea, 2026-11-20; see [packaging](architecture-packaging.md#linux)).

## Bundled ffmpeg 9.x checks (QG-07)
| Check | Where |
|---|---|
| Frame: Change aspect, Scale, Move | existing `tests/ffmpeg_integration/v15.rs` on the pinned build |
| Apply LUT (.cube, clip and whole video) | existing `v16.rs`; format 8 read in `v20.rs` |
| Spherical marker (MP4 and MOV) | existing `v126.rs` and `v13.rs` (checked through ffprobe) |
| Clean audio and MP3 output | existing `v14.rs`; encoder presence in `v20.rs` |
| Paused frame at a fade (REQ-144) | new `v20.rs` |
| Format 8 refusal by 1.5.x | manual (checklist item 2): there is no 1.5.x code in the tree |

## 360 camera timing (QG-07)
- The camera timing figures of [360 video](architecture-360.md#performance-and-limits) were measured with ffmpeg 6.1.1. The 2.0.2 run repeats them on the bundled 9.0.2 build (median of three runs) and records both sets. The targets (400 ms and 700 ms for stills) are not changed; a missed target goes to the roadmap.

## Decisions already taken (unchanged in 2.0.2)
- Hardware export always goes through ffmpeg (no separate vendor pipeline).
- AV1 and VP9 encoding and hardware decoding are in scope (2.0.10; the pinned build is checked for the encoders then).
- Installers stay unsigned.

## 2.0.3 (planned): speech test and blur fix
- The CI check job fails on the blurred-fill export of a small frame (a bundled 9.0.2 regression found on the runner); the fix is the first step of 2.0.3.
- The strict speech-accuracy test runs in CI without being skipped; CI sets `VIDEOLORE_REQUIRE_STT=1`, so a missing model or fixture fails instead of skipping. The fixture files are checked by their SHA-256 sums; the generator (espeak-ng) is recorded beside them and is not needed to run the tests.
- The 2.0.3 manual checks are the three-system font list (five fonts drawn as chosen) and the 20 MB font window delay, as in the business verification list.

## Risks
- Pinned ffmpeg downloads add CI time and depend on the download site; the archive is cached by the lock hash.
- The integration tests run on Linux only with the pinned build; Windows and macOS builds of 9.0.2 are not tested by the suite.
- The 1.6.0 format 8 file is not generated by the tests (the fixture is written by hand from the format description); the real file is checked by hand.
- The best-of-five rule could hide an intermittent slowdown; the printed samples and the manual load check show it.
- Settled (2026-11-20): the splash pictures are Andrea's own work under MIT and are named in the notices; no `.deb` package is planned.
