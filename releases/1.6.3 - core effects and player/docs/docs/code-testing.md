# Code — tests and CI

## Tests

- **Unit tests** sit next to the code (`#[cfg(test)]`; the larger ones in `src/app/tests.rs` and
  `src/ui/tests.rs`). They use a fake `Runner` (`ffmpeg::fake::FakeRunner`), so no ffmpeg is
  needed. Background jobs really run on threads; tests wait for them with
  `app::tests::settle`.
- **Fakes for slow or external things:** a counting runner checks that the process cap of every
  "How hard" level (3, 4, 6) is never exceeded (20 probes + export + preview) and that lowering
  the cap never stops a running process; simulated machines (`Machine::simulated`, an x86-64
  machine without AVX2) drive the recommendation, the effort table and the old-processor
  message; a fake transcriber and a fake model downloader drive
  subtitle generation (consent, checksum, cancel, language); test machines are injected for the
  proxy rule; the proxy cache and the model folder are temporary folders.
- **UI tests** run whole frames headless with `egui::Context::run_ui` (layout at several sizes,
  every dialog, shortcuts, clicks and drags on the timeline and the music lane, the window close
  button via `ViewportEvent::Close`, the Settings window and the banner, read back from the
  painted text). `stays_responsive_while_exporting_and_transcribing` runs 30 frames with
  play/pause, timeline clicks and Settings while a fake export streams progress and a fake
  speech job keeps every core busy: every frame must take ≤ 200 ms and show its effect;
  `borders_and_popups_stay_responsive_and_export_cancels_from_the_status_bar` does the same
  while dragging a section border with three effect popups open, then cancels the export from
  the status bar and checks that no file is left.
- **Interface tests through the accessibility tree** (`src/ui/ux_tests.rs`): a small `Harness`
  turns on egui's AccessKit output and reads every widget's role, label, bounds and enabled
  state each frame; clicks, right-clicks, double-clicks, drags and keys are real input events at
  those positions. They check the menus against the action table (labels, shortcuts, greyed
  items), the toolbar's "»" menu, border drags (live, saved once), markers and the chooser,
  context menus, Clip details, Esc and the shortcuts, the shortcuts window, dropped files, the
  Effects section. The "everything reachable" test (REQ-068) runs at 800x500 with 50 media
  items, 200 subtitle lines and a 200-character name, with sections at their minimum sizes and
  with each popup, dialog and menu open: every interactive widget must be inside the window, and
  inside its section, unless it is inside a scroll area (recorded each frame in
  `UiCache::containers`) whose visible part can be reached.
- **1.2.0:** `src/app/effects_tests.rs` (effects on a clip or the whole video, the video's own
  subtitles with a fake runner that "extracts" an SRT track and a scripted recogniser, the
  consent choices, cancel at every step, line selection and positions, SRT export); pure
  tests in `align`, `srt`, `ui/ruler`, `ui/placement`; interface tests drag an effect row onto
  a clip, drag a subtitle in the preview, Ctrl/Cmd+click markers and Shift+drag the timeline;
  a third responsiveness test drags an effect and a subtitle while an export and a matching
  job run.
- **Frozen 1.1 output:** `tests/fixtures/project-v2-1.1.vlproj` (a 1.1 project with fades,
  music, mix and word-timed subtitles) and `project-v1.vlproj` must give exactly the export
  arguments and subtitle document in `tests/fixtures/export-argv-1.1.0.txt`, captured with the
  1.1.0 code. Regenerate it only on purpose
  (`cargo test --lib write_golden_1_1 -- --ignored`). Since 1.6.1 the tests compare with
  `export-argv-1.1.0-in-1.6.1.txt` (and `export-argv-1.2.5-in-1.6.1.txt`), where the video's
  sound level is applied per clip; the 1.1.0 and 1.2.5 files stay as the historical record.
- **Memory:** a hostile-input test must not build its input at the size of the attack (the
  subtitle track test once built 10 GB of JSON and brought a machine down). When running the
  suite on a small machine, cap it, e.g. `systemd-run --user --scope -p MemoryMax=6G
  -p MemorySwapMax=0 -- cargo test --workspace`. Every test, clippy and coverage run of a
  development is made under such a cap (8 GB, `MemorySwapMax=0`) and a `timeout`, one cargo job
  at a time; `target/llvm-cov-target` is deleted after coverage when the disk has less than 8 GB free.
- **Settings and hostile input:** the settings decisions are pure functions tested with corpora
  (bad JSON, unknown values, huge numbers, nesting, relative paths, newer formats, 0.2.x files);
  project files are tested with a hostile corpus (truncated, nested, out-of-range numbers,
  `../` and absolute paths, control and text-direction characters, more than 16 MiB, a FIFO).
- **Integration tests** (`tests/ffmpeg_integration.rs`) run the real ffmpeg. Clips are generated
  at test time (`testsrc2`, `sine`, `color` + `geq`); no media is stored in git. Generated clips
  are cached in `target/tmp/fixtures/<hash of parameters and ffmpeg>` (written under a unique
  name and renamed, so parallel tests are safe) and reused by later runs. They cover stream copy, letterbox, fades, cancel, preview frames, `hvc1` tagging
  (needs libx265), 200 pieces of one file exported frame-accurately with a single input, music
  mix/cut/fade/volume, subtitle burn-in (needs libass), a real proxy of a 1440p clip, and memory
  use with a 2 s vs a 120 s source (160×90, 10 fps: the test is about length) (each measured in its own process, re-running the test
  binary on the hidden helper `memory_peak_of_one_preview`, so tests running in parallel, such as
  the speech model, do not count). They are skipped when ffmpeg is missing, unless
  `VIDEOLORE_REQUIRE_FFMPEG=1` (set in CI, where missing libx265/libass also fail).
- **Real speech recognition** (`real_speech_is_transcribed`, feature `stt`) runs only when
  `VIDEOLORE_STT_MODEL` points to the pinned Fast file and `espeak-ng` is installed (both in CI).
  `higher_levels_are_at_least_as_accurate` needs `VIDEOLORE_STT_MODELS_DIR` with the three
  speech files (cached in CI): the word error rate of Balanced and Most accurate on a spoken
  reference sentence must not be worse than Fast's.
  `abort_check_of_whisper_ignores_heap_garbage` (needs only the model) fills the heap with freed
  `0xFF` blocks before each transcription: it guards against the whisper-rs 0.16 cancel callback
  bug, which read the cancel flag from uninitialised heap memory and so made every transcription
  fail with "code -6" on CI only.
- **Hardware export** ("Export: Faster"): CI has no graphics hardware, so a fake ffmpeg (unit
  tests) and a test runner that stands in for each vendor with libx264/libx265 and damages files
  on request (integration tests) drive every failure path; the tests are listed in
  [code-hardware.md](code-hardware.md). `VIDEOLORE_REAL_HW=1` exports for real on the machine's
  own video hardware, and the ignored `qg05_reference_render` measures QG-05 (time, size, VMAF).
- **Fixtures:** `tests/fixtures/project-v1.vlproj` is a project saved by 0.1.x; it must keep
  opening (and exporting) identically. `tests/fixtures/export-argv-0.3.0.txt` holds the export
  commands of 0.3.0 for 6 plans; Best quality must keep producing exactly these.
- Each implemented requirement (`REQ-NNN (implemented)` in `docs/business*.md`) has at least one
  test marked `// covers: REQ-NNN`, checked by `scripts/feature-coverage.sh`.

## Coverage

`scripts/coverage.sh --fail-under 85` (locally; CI uses the equivalent single
`cargo llvm-cov --fail-under-lines 85` run) measures line coverage of the library (`src/main.rs` is
excluded: it only wires the library). Not covered on purpose: the native file dialogs
(`dialogs.rs`), the macOS Apple Event code (compiled only on macOS) and the real whisper.cpp
glue outside the gated test.

## Continuous integration (`.github/workflows/build.yml`)

| Job | When | What |
|---|---|---|
| `check` | every push / PR, `ubuntu-24.04` | apt `ffmpeg libasound2-dev pkg-config cmake libclang-dev espeak-ng`; the three speech files cached (checked by SHA-256); fmt; clippy with default, no and all features; the tests once, instrumented (`cargo llvm-cov`, line coverage ≥ 85 %) with `VIDEOLORE_REQUIRE_FFMPEG=1`; `cargo deny check` (vulnerabilities and yanked crates fail; exceptions only in `deny.toml` with a reason); Rust crate notices up to date; version check; feature coverage 100 %; frozen release docs untouched; fmt, clippy, tests and coverage run with `--workspace`, and the player's `cargo tree` is searched for network and speech crates |
| (all jobs) | | `Swatinem/rust-cache` with `prefix-key: v2-rust-ggml-portable`, bumped whenever the whisper.cpp build flags change so no old build is reused |
| (test steps) | | `cargo test` output is saved; on failure `scripts/test-annotations.sh` turns failing tests (name, file/line, panic message), compiler errors and crashed test binaries into GitHub error annotations, readable without logging in |
| `msrv` | every push / PR | Build all targets with exactly the `rust-version` of `Cargo.toml` (equal to stable by policy; the tests run in `check`) |
| (security) | | Read-only token by default (`contents: write` only in `package`); every action pinned to a full commit SHA (tag in a comment, all on Node 24); runners pinned (`ubuntu-24.04`, `macos-15`, `windows-2025`); a unit test checks both |
| `build` | after `check` + `msrv` | Bare release executables for Linux x86_64, macOS arm64 and Windows x86_64, uploaded as artifacts |
| `package` | `v*` tags and manual runs, after `check` + `msrv` | Per OS: fetch the pinned ffmpeg, build, `scripts/package.sh` (with the ffmpeg smoke test; on Linux it also refuses an executable needing glibc newer than 2.39), the third-party source archive on Linux; artifacts; on tags the five release assets are attached to the GitHub release |

## Integration test files

| File | What it runs with the real ffmpeg |
|---|---|
| `tests/ffmpeg_integration.rs` | The shared `Env` (fixtures, `job`, `export`, `frame_rgb`) and the basics: stream copy, letterbox, fades, cancel, preview frames, `hvc1`, 200 pieces of one file, music, burn-in, proxies, memory |
| `ffmpeg_integration/v12.rs` | A whole-video fade starts and ends in black (also after cutting the first clip); a muted clip is silent and a 50 % clip is a quarter of the level; a positioned subtitle is burned in within 2 % of the frame height (measured 0.5 px off at 320x180) and the preview uses the same document; the exported SRT is read back; MKV + SRT and MP4 + timed text tracks with their language, also with time stamps starting at 5 s |
| `ffmpeg_integration/v126.rs` | 360° video: a generated 2048x1024 sphere with a white meridian and parallel (an 8K file is generated at test time, never committed); camera paths in preview and export (see [code-360.md](code-360.md)) |
| `ffmpeg_integration/v13.rs` | Texts: black clips with a raised bit rate so the text is not blurred away, a 2048x1024 black sphere, texts and subtitle lines fixed to the sphere (see [code-text.md](code-text.md)) |
| `ffmpeg_integration/v14.rs` | Export dialog and Clean audio with its own pink-noise and speech-like clips; the Clean audio gate compares the export with and without the effect through the same encoding pass, because a mono source that becomes stereo lowers the measured level by itself (see [code-export.md](code-export.md)) |
| `ffmpeg_integration/v15.rs` | Change aspect, Scale, Move: flat colour clips so pixels can be sampled with a small tolerance (see [code-frame.md](code-frame.md)) |
| `ffmpeg_integration/v16.rs` | Apply LUT: flat colour clips and 2x2x2 cubes that move whole colours (channel swaps, inversion) (see [code-lut.md](code-lut.md)) |
| `ffmpeg_integration/hw.rs` | Hardware export with simulated vendors; the exports of one test that do not depend on each other run on parallel threads (`in_parallel`) (see [code-hardware.md](code-hardware.md)) |
| `player/tests/ffmpeg.rs` | The player: play, pause, seek, volume, an embedded track, an `.srt`, a 360 drag; the video's folder is compared before and after |

The golden argv files of 1.1.0 and 1.2.5 still pass: a project without the effects added later
exports as before. The 1.2.5 file `tests/fixtures/export-argv-1.2.5.txt` was written by the
ignored test `write_golden_1_2_5`, run only with the 1.2.5 code.

The alignment fixture `tests/fixtures/align/` is committed: `speech.mkv` (synthetic speech,
Opus, with an English SRT track that has one misspelt word), `reference.json` (where each word
really starts) and `track.srt`. `make.sh` rebuilds them (development only; needs espeak-ng and
ffmpeg). Two tests use it when `VIDEOLORE_STT_MODEL` points to the Fast speech file (as in CI):
`alignment_keeps_the_track_and_times_its_words_from_real_speech` (text and cue times kept,
every word timed inside its line, at least half of the words found) and the accuracy target
`alignment_matches_real_speech`, which is `#[ignore]`d because it is not met yet (see
[code-effects.md](code-effects.md)).

## Other test notes

- The four responsiveness gates in `src/ui/tests.rs` run one at a time (`one_gate_at_a_time`)
  because each keeps every core busy. Under a heavily loaded machine (or the instrumented
  coverage run) a frame can pass the 200 ms limit; the gates are not relaxed for it.
- Timing tests with fake processes use limits long enough for a busy machine (the hardware
  check repeat test uses 200/400 ms per try).
- Player: `player/src/tests.rs` uses a scripted `Runner` (ffprobe answers by file name, ffmpeg
  writes a subtitle track, spawned processes hold their output until killed). The UI test
  harnesses of both programs read egui's accessibility tree; the editor's strips the leading
  icon from labels (`Node::label`, with the raw one in `Node::raw`).
- Splash: `src/splash.rs` tests both programs (`check_program`: size, rectangle inside the
  picture, one to five summary lines, colour), `fit_summary` and the 64 MiB cap
  (`decode_capped`); `src/ui/splash_ux_tests.rs` measures that the summary fits its panel
  (`SplashRects::summary_used` against `summary_room`); the player's start screen tests
  (REQ-150) are in `player/src/ui_tests.rs` and `player/src/tests.rs` (`real_app`).

## Suite size and time

Measured on the development machine (8 threads, 15 GiB, ffmpeg 6.1.1) for 1.6.3: 563 library
tests (2 ignored), 62 integration tests (3 ignored), 33 player tests, 3 player integration
tests; 13-16 s for the library tests, 45-62 s for the integration tests and 72-99 s for the whole `cargo test --workspace` (they use about
270 s of processor time, so the run is bound by the processor, not by waiting). The slowest
tests export 1920x1080 views of 360° videos (about 25 s each).
