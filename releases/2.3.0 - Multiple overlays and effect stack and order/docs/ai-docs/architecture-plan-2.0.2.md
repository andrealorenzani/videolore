# ai-notes: plan 2.0.2 — verify the release (owner: architect; index in ai-docs/architecture.md)

Scope: batch 1 of docs/roadmap_tasks.md (2.0.2). Requirements: REQ-154 (planned, becomes implemented), QG-07..QG-10 (docs/business-performance.md). The manual checks of docs/business-verification.md stay manual (not agent work). No behaviour change: no file format, setting, file name, output, UI text or limit changes. Version 2.0.2 (root and player/Cargo.toml through scripts/version.sh). Human version: docs/architecture-verification.md.

## 1. Facts found by reading (2026-11-20, HEAD 30c398f, tree 2.0.1)
- Test ffmpeg: tests take ffmpeg from `ffmpeg::locate` (setting > env value > bundled copy > PATH; src/ffmpeg.rs:361). CI check job (.github/workflows/build.yml:37-46) installs apt `ffmpeg` from the runner: not pinned (the 1.6.1 spike used 6.1.1; assumption that the runner is 6.1.x). Bundled build: 9.0.2 (packaging/ffmpeg.lock: linux-x86_64 BtbN linux64-gpl, windows-x86_64 win64-gpl-shared, macos-aarch64 Martin Riedl).
- `VIDEOLORE_REQUIRE_FFMPEG=1` makes the integration tests fail instead of skip (tests/ffmpeg_integration.rs:44-46). CI does not set it yet.
- Splash summary: src/ui/splash_window.rs:219-220 `SUMMARY_MAX = 15.0`, `SUMMARY_FLOOR = 7.0`; size by `splash::fit_summary` (bisection, 20 steps, src/splash.rs:249). So the REQ-154 range is already in HEAD. docs/architecture-splash.md and ADR-130 still say 14 px down to 10 px in 0.5 steps: stale, fixed by this plan (ADR-143).
- Summary tests: src/splash.rs `the_summary_fits_the_largest_size_that_leaves_room` (generic numbers); src/ui/splash_ux_tests.rs:148 `the_summary_fits_its_panel_without_clipping_or_scrolling` (EDITOR only). No test at 7 px, none for PLAYER, no `covers: REQ-154`.
- Timing assertions in tests (wall clock): src/project.rs:726 (< 1 s), src/media.rs:596 (< 5 s), src/machine.rs:213 (< 3 s); UI frame-time tests in src/ui/tests.rs (200 ms, REQ-104; one busy-desktop failure at 212 ms, roadmap); src/app/ux_tests.rs and frame_tests.rs (Instant based); src/app/hw_tests.rs (10 s loops); tests/ffmpeg_integration.rs preview test (30 s deadlines, 10 ms polls); two hardware tests with 30 ms probe timeouts (roadmap, load risk).
- Hardware probe limits: `ProbeLimits { per, retry, total }` (src/hwenc.rs:717), default 1 s / 4 s / 10 s, already injectable; src/hwenc/tests.rs:381 pins the defaults.
- Notices: scripts/notices.sh --check exits 1 when a committed file differs, 0 when up to date; CI step build.yml:102-104 (check job). QG-10 wants the exit code recorded.
- cargo-about: taiki-e/install-action pinned by SHA, tool `cargo-about@0.9.3` (build.yml:54). Roadmap: its verification is open.
- Installer config: scripts/package.sh `--config-only` accepts only windows-x86_64 and macos-aarch64 (package.sh:180-184). Package job runs on `v*` tags and manual runs only (build.yml:209). Linux products are AppImage (hand-built AppDir, appimagetool pinned in packaging/tools.lock) and tar.gz. NO .deb is produced (docs/architecture-packaging.md § Linux). QG-10 and roadmap.md:24 say "deb and app formats": mismatch; decided (REFINE 2026-11-20): no .deb, the check covers the AppImage.
- Format 8: fixtures in tests/fixtures are v1, v2-1.1, v3-1.2.5 projects only. No 1.5.x or 1.6.0 code in the tree; git tags end at v1.6.3 (no 1.6.0 tag). The 1.5.x refusal can only be a real-build check (business-verification item 2).
- Spherical marker: tests exist in tests/ffmpeg_integration/v126.rs and v13.rs (and v16 mentions it); CI runs them on the runner ffmpeg today.
- Clean audio and MP3: tests/ffmpeg_integration/v14.rs:207 (AudioChoice::Mp3).
- Paused preview: src/preview.rs:1103 (`paused_seek_shows_a_still_frame_and_pause_keeps_position`), :1225; no test of a paused frame at a fade (REQ-144 acceptance (a) wants the picture, not black).
- 360 camera numbers: docs/architecture-360.md § Spike results (ffmpeg 6.1.1): about 75-100 ms per command; still latency tests 125 ms (4K) and 155 ms (4096 copy) in `big_spheres_get_a_4096_copy_and_camera_stills_are_quick_enough`. Not measured on 9.x.
- Splash pictures: origin and licence settled (REFINE 2026-11-20): Andrea's own work, MIT; already recorded in packaging/notices/third-party.txt (line 71, with a test). Q-E closed; nothing to change.

## 2. Components touched
- CI: `.github/workflows/build.yml` (check job: ffmpeg pin, cargo-about version step; new job `package-config`, every push and pull request).
- Scripts: `scripts/package.sh` (new `--stub` mode for linux-x86_64; `--config-only` unchanged for windows/macos); `scripts/fetch-ffmpeg.sh` reused unchanged for the test ffmpeg (its smoke test runs the binary).
- Tests: `tests/ffmpeg_integration.rs` (pin assertion under VIDEOLORE_REQUIRE_FFMPEG; wait deadlines), new `tests/ffmpeg_integration/v20.rs` (2.0.2 tests), `tests/fixtures/project-v8-2.0.2.vlproj` (new, hand-written from ADR-111), `src/ui/splash_ux_tests.rs` (PLAYER, 7 px, overflow), `src/hwenc` and `src/app/hw_tests.rs` (injected limits), timing tests listed in section 1, new `#[cfg(test)]` helper `src/base/testing.rs` (best-of-N sampling; std only, no `crate::` path, so the 2.0.1 cycle check holds).
- Docs owned by the architect: this plan, ADRs, human `docs/architecture-verification.md` (new), `docs/architecture-splash.md` (rule fixed to 15/7), `docs/architecture-build.md` (link), `docs/architecture.md` (sub-document row).
- No change: src/splash.rs layout and fit code, src/ui/splash_window.rs constants, src/ffmpeg.rs lookup order, product limits (REQ-061 1 s/4 s/10 s, REQ-104 200 ms, REQ-029 format refusals).

## 3. Decisions (full text ADR-142..151 in ai-docs/architecture-adr.md)
- ADR-142 test ffmpeg = the pinned bundled build (linux-x86_64 entry), not the runner's apt package. [TECHNICAL]
- ADR-143 summary fit 15 down to 7 px as built; tests for both programs at 7 px, overflow test; ADR-130 superseded. [TECHNICAL]
- ADR-144 load-independent tests: (a) logic on injected limits and clocks, (b) wall-clock product gates as best-of-5 minimum, (c) waits are harness timeouts (may be raised). Product constants pinned by unit tests. [TECHNICAL]
- ADR-145 hardware probe tests: success fakes get seconds of limit; the hanging fake keeps a short limit (it never answers, so it stays deterministic). [TECHNICAL]
- ADR-146 installer configuration check on every push: `package-config` job, no ffmpeg, no cargo-packager on Linux; Windows and macOS `--config-only`; Linux AppImage from stub binaries (`--stub`). No .deb (decided 2026-11-20). [TECHNICAL]
- ADR-147 notices exit codes: 0 up to date, 1 out of date, cargo-about errors keep their own code (set -e). Recorded in the docs with a temporary-edit run. [TECHNICAL]
- ADR-148 cargo-about: keep the pinned action, add a version assertion (`cargo-about 0.9.3`); fallback `cargo install --locked cargo-about --version 0.9.3 --features cli` if the action's binary cannot be verified. [TECHNICAL]
- ADR-149 bundled ffmpeg 9.x: existing tests run on the pinned build (QG-07); new v20 tests for REQ-144 (paused frame at a fade) and a format 8 project read with its LUTs; the 1.5.x refusal stays manual (checklist 2). [TECHNICAL]
- ADR-150 360 camera re-measured on the pinned build, recorded beside the 6.1.1 spike; no gate change. [TECHNICAL]
- ADR-151 inherited decisions (no change in 2.0.2): hardware export always through ffmpeg; AV1 and VP9 encoding and hardware decoding in scope (2.0.10: the pinned build must offer the encoders; checked then); installers unsigned.

## 4. NFR and quality goals for 2.0.2
- QG-07: full suite green on the pinned build; in CI no ffmpeg test is skipped (VIDEOLORE_REQUIRE_FFMPEG=1).
- QG-08: the CI test job's ffmpeg version equals `packaging/ffmpeg.lock` [linux-x86_64] version; mismatch fails the job.
- QG-09: no product limit changed; suite green while the machine is loaded (PE check: run `cargo test` with 4 busy-loop processes; record result and worst sample times).
- QG-10: notices exit codes recorded; cargo-about version step; `package-config` job green on every push.
- Cost (estimate, to measure in the first CI run, QG-06): pinned ffmpeg archive about 100 MB, cached by lock hash; package-config about 2 to 3 min (appimagetool download included).

## 5. Security
- The test ffmpeg is downloaded over HTTPS and its SHA-256 is checked by fetch-ffmpeg.sh before any use; the cache key is the lock hash; no secret; `permissions` of the workflow stay read-only.
- appimagetool: pinned version and SHA-256 in packaging/tools.lock; PE confirms that fetch_tool checks it.
- `--stub` output is written under a temporary dist folder and never uploaded as an artefact; the stub binaries are never shipped.
- The test ffmpeg is never distributed (the bundled one is the same build family, unchanged licence chain).

## 6. Ordered steps for the Principal Engineer
1. Baseline: run the whole suite (fmt, clippy default/no/all features, test, coverage, feature coverage) on the current code; record failures and times. Fix code only (roadmap: rebuild and test first).
2. Timing and load (ADR-144, ADR-145): add `src/base/testing.rs` (`#[cfg(test)]`, std only) with `best_of(n, sample) -> Duration` (stop at the first sample under the limit; print every sample when all fail). Apply it to the wall-clock product gates (UI frame 200 ms, camera stills, the timings at project.rs:726, media.rs:596, machine.rs:213). Raise harness waits (preview 30 s, hw loops 10 s) to 120 s. Hardware probe success fakes get `ProbeLimits` of seconds; the 30 ms limit stays only on the hanging fake. Add a unit test of `best_of` (passes when one sample is under, fails when all are over).
3. Splash (ADR-143): in src/ui/splash_ux_tests.rs add: the PLAYER summary fits at its chosen size; both programs fit at 7 px (`fit_summary` floor); an over-long summary at 7 px fails the overflow check. Each test carries `// covers: REQ-154`. Keep SUMMARY_MAX 15 and SUMMARY_FLOOR 7 as they are.
4. Test ffmpeg pin (ADR-142): in tests/ffmpeg_integration.rs, when VIDEOLORE_REQUIRE_FFMPEG=1, assert the found ffmpeg's version equals the linux-x86_64 version in `include_str!("../packaging/ffmpeg.lock")` (parse the section; no hard-coded number). CI (step 5) sets the lookup env value to the fetched binary and prepends its folder to PATH.
5. CI (ADR-142, ADR-147, ADR-148): check job: remove `ffmpeg` from the apt list; add a step `scripts/fetch-ffmpeg.sh linux-x86_64 "$RUNNER_TEMP/ffmpeg-test"` with actions/cache keyed by the lock hash; set VIDEOLORE_REQUIRE_FFMPEG=1; add `cargo about --version` step that must print 0.9.3. Notices step unchanged.
6. New tests (ADR-149): tests/ffmpeg_integration/v20.rs, registered with `#[path]` in tests/ffmpeg_integration.rs:
   - `a_paused_frame_at_a_fade_is_not_black` (covers REQ-144): two bright clips, fade at the cut; pause inside the fade; the still frame has picture luma (not black).
   - `a_format_8_project_reads_its_luts` (covers REQ-137): loads tests/fixtures/project-v8-2.0.2.vlproj (hand-written from the format 8 description, ADR-111), checks whole-video and clip LUT paths, renders one frame with the pinned build.
   - `the_bundled_build_offers_mp3_and_the_frame_and_lut_filters`: `ffmpeg -encoders` has libmp3lame; `-filters` has lut3d, v360, and the filters of frame and text; the test fails with the missing name.
   - Existing v12..v16 and v126 tests are the 9.x checks of the frame, LUT, spherical marker and Clean audio; the PE confirms they assert the marker through ffprobe.
7. Installer configuration (ADR-146): scripts/package.sh: add `--stub` for linux-x86_64 (AppDir per the existing Linux branch, stub `videolore` and `ffmpeg`/`ffprobe` files, appimagetool from the pinned tools lock, no smoke test, output in a temporary folder); keep `--config-only`. build.yml: new job `package-config` on push and pull request (ubuntu-24.04, timeout, read-only permissions): `package.sh windows-x86_64 --config-only`, `package.sh macos-aarch64 --config-only`, `package.sh linux-x86_64 --stub`, each followed by a check that the expected files exist (Packager.toml fields for NSIS and app; AppDir desktop entry, icon, AppRun).
8. Camera (ADR-150): run `big_spheres_get_a_4096_copy_and_camera_stills_are_quick_enough` and a filter-cost run with `--nocapture` on the pinned build (at least 3 runs, median). Record in docs/architecture-360.md § Performance and limits (2.0.2 line, next to the 6.1.1 numbers) and in ai-docs/architecture-nfr.md. No gate change; if a target (400 ms, 700 ms) is missed, the number goes to Future work.
9. Load check (QG-09): run the suite with busy-loop processes on the machine; record pass/fail and the worst sample of best_of.
10. Gates: coverage >= 85%, feature coverage 100%, `scripts/version.sh check`. Report line coverage (one decimal), feature coverage, the notices exit codes and the camera numbers.

## 7. Tests (summary)
- New: splash PLAYER and 7 px tests (REQ-154); overflow test; best_of unit test; paused fade (REQ-144); format 8 read (REQ-137); ffmpeg pin assertion; encoder and filter presence; `package-config` job; cargo-about version step.
- Changed (same assertions, injected or sampled): UI frame timing, project/media/machine timings, preview wait, hw probe success fakes.
- Not changed: REQ-061 and REQ-104 constants (ProbeLimits::default and 200 ms are pinned by unit tests).

## 8. Risks
- best_of can hide intermittent slowness; mitigated by printing every sample on failure and by the load check (step 9).
- Test ffmpeg download can fail on the network; fetch-ffmpeg.sh retries 3 times; the job fails loudly (not silently skipped).
- CI time grows (download, cache, package-config): measured in the first run (QG-06).
- The suite runs the pinned build on Linux only; Windows and macOS builds of 9.0.2 are not tested by the suite (Future work).
- Format 8 fixture is hand-written, not written by 1.6.0 code; the real 1.6.0 file check is manual (checklist 2).
- 7 px summary text is small; readability is judged in the manual check 5.
## 9. Questions
- none open. Decided by Andrea 2026-11-20: Q-1 (splash pictures are his own work, MIT, already in the notices: no change); Q-2 (no .deb; the installer check covers the AppImage).
- [TECHNICAL] all decisions in section 3.

## 10. Future work
- Windows and macOS test runs with the pinned ffmpeg (cost: two more long jobs).
- Camera targets (400 ms, 700 ms) if missed on 9.x: roadmap batch 15 (2.0.15, 360 video speed).
- Product Manager: QG-10 wording (no .deb, AppImage checked; decided 2026-11-20); REQ-154 status to implemented after the tests pass.
- docs/architecture.md is near 160 lines: the verification part is a new sub-document (done in this plan).
