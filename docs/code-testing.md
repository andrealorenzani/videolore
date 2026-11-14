# Code — tests and CI

## Tests

- **Unit tests** sit next to the code (`#[cfg(test)]`; the larger ones in `src/app/tests.rs` and
  `src/ui/tests.rs`). They use a fake `Runner` (`ffmpeg::fake::FakeRunner`), so no ffmpeg is
  needed. Background jobs really run on threads; tests wait for them with
  `app::tests::settle`.
- **Fakes for slow or external things:** a counting runner checks that no more than 3 processes
  ever run (20 probes + export + preview); a fake transcriber and a fake model downloader drive
  subtitle generation (consent, checksum, cancel, language); test machines are injected for the
  proxy rule; the proxy cache and the model folder are temporary folders.
- **UI tests** run whole frames headless with `egui::Context::run_ui` (layout at several sizes,
  every dialog, shortcuts, clicks and drags on the timeline and the music lane, the window close
  button via `ViewportEvent::Close`).
- **Integration tests** (`tests/ffmpeg_integration.rs`) run the real ffmpeg. Clips are generated
  at test time (`testsrc2`, `sine`, `color` + `geq`) in a temporary folder; no media is stored
  in git. They cover stream copy, letterbox, fades, cancel, preview frames, `hvc1` tagging
  (needs libx265), 200 pieces of one file exported frame-accurately with a single input, music
  mix/cut/fade/volume, subtitle burn-in (needs libass), a real proxy of a 1440p clip, and memory
  use with a 2 s vs a 120 s source (each measured in its own process, re-running the test
  binary on the hidden helper `memory_peak_of_one_preview`, so tests running in parallel, such as
  the speech model, do not count). They are skipped when ffmpeg is missing, unless
  `VIDEOLORE_REQUIRE_FFMPEG=1` (set in CI, where missing libx265/libass also fail).
- **Real speech recognition** (`real_speech_is_transcribed`, feature `stt`) runs only when
  `VIDEOLORE_STT_MODEL` points to the pinned model file and `espeak-ng` is installed (both in CI).
- **Fixtures:** `tests/fixtures/project-v1.vlproj` is a project saved by 0.1.x; it must keep
  opening (and exporting) identically.
- Each implemented requirement (`REQ-NNN (implemented)` in `docs/business*.md`) has at least one
  test marked `// covers: REQ-NNN`, checked by `scripts/feature-coverage.sh`.

## Coverage

`scripts/coverage.sh --fail-under 85` measures line coverage of the library (`src/main.rs` is
excluded: it only wires the library). Not covered on purpose: the native file dialogs
(`dialogs.rs`), the macOS Apple Event code (compiled only on macOS) and the real whisper.cpp
glue outside the gated test.

## Continuous integration (`.github/workflows/build.yml`)

| Job | When | What |
|---|---|---|
| `check` | every push / PR | apt `ffmpeg libasound2-dev pkg-config cmake libclang-dev espeak-ng`; cached speech model (checked by SHA-256); fmt; clippy with default, no and all features; tests with `VIDEOLORE_REQUIRE_FFMPEG=1`; `cargo deny check`; Rust crate notices up to date (`scripts/notices.sh --check`); version check; line coverage ≥ 85 %; feature coverage 100 %; frozen release docs untouched |
| (all jobs) | | `Swatinem/rust-cache` with `prefix-key: v1-rust-ggml-portable`, bumped whenever the whisper.cpp build flags change so no old build is reused |
| `msrv` | every push / PR | Build and test with exactly the `rust-version` of `Cargo.toml` |
| `build` | after `check` + `msrv` | Bare release executables for Linux x86_64, macOS arm64 and Windows x86_64, uploaded as artifacts |
| `package` | `v*` tags and manual runs, after `check` + `msrv` | Per OS: fetch the pinned ffmpeg, build, `scripts/package.sh` (with the ffmpeg smoke test), the third-party source archive on Linux; artifacts; on tags the five release assets are attached to the GitHub release |
