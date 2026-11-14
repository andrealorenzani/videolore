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
  speech job keeps every core busy: every frame must take ≤ 200 ms and show its effect.
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
| `check` | every push / PR, `ubuntu-24.04` | apt `ffmpeg libasound2-dev pkg-config cmake libclang-dev espeak-ng`; the three speech files cached (checked by SHA-256); fmt; clippy with default, no and all features; the tests once, instrumented (`cargo llvm-cov`, line coverage ≥ 85 %) with `VIDEOLORE_REQUIRE_FFMPEG=1`; `cargo deny check` (vulnerabilities and yanked crates fail; exceptions only in `deny.toml` with a reason); Rust crate notices up to date; version check; feature coverage 100 %; frozen release docs untouched |
| (all jobs) | | `Swatinem/rust-cache` with `prefix-key: v1-rust-ggml-portable`, bumped whenever the whisper.cpp build flags change so no old build is reused |
| (test steps) | | `cargo test` output is saved; on failure `scripts/test-annotations.sh` turns failing tests (name, file/line, panic message), compiler errors and crashed test binaries into GitHub error annotations, readable without logging in |
| `msrv` | every push / PR | Build all targets with exactly the `rust-version` of `Cargo.toml` (equal to stable by policy; the tests run in `check`) |
| (security) | | Read-only token by default (`contents: write` only in `package`); every action pinned to a full commit SHA (tag in a comment, all on Node 24); runners pinned (`ubuntu-24.04`, `macos-15`, `windows-2025`); a unit test checks both |
| `build` | after `check` + `msrv` | Bare release executables for Linux x86_64, macOS arm64 and Windows x86_64, uploaded as artifacts |
| `package` | `v*` tags and manual runs, after `check` + `msrv` | Per OS: fetch the pinned ffmpeg, build, `scripts/package.sh` (with the ffmpeg smoke test; on Linux it also refuses an executable needing glibc newer than 2.39), the third-party source archive on Linux; artifacts; on tags the five release assets are attached to the GitHub release |
