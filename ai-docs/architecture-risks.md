# ai-notes: risks (owner: architect; index in ai-docs/architecture.md)

Resolved items are removed (0.3.0 SEVERE defaults confirmed 2026-11-14; 1.2.6 time-dependent command spike). Items with [open] need a decision or measurement.

## Release and manual checks
- [2.0.2 decided 2026-11-20] no `.deb` package (AppImage and tar.gz only); QG-10 wording corrected by the Product Manager. Splash pictures: Andrea's own work, MIT, in packaging/notices/third-party.txt (Q-E closed).
- [2.0.2 plan] wall-clock gates in tests can fail on a busy machine; best-of-5 sampling (ADR-144) keeps the product limits; check by a load run (plan step 9).
- [2.0.2 plan] pinned test ffmpeg adds a network download and cache to the check job (QG-06 time).
- [2.0.2 plan] the suite tests the pinned build on Linux only; Windows and macOS builds of 9.0.2 are untested by the suite (Future work).
- [2.0.2 plan] format 8 fixture is hand-written (no 1.6.0 writer in the tree); the real 1.6.0 file is checked by hand (checklist 2).
- Manual-only checks: macOS launch-time odoc timing, Gatekeeper on nested ffmpeg (fallback `xattr -dr com.apple.quarantine /Applications/videolore.app`), SmartScreen on unsigned installers, Linux file managers (tar.gz ELF double-click; README fallback). odoc failure -> in-app Open still works. Checklist in docs/code.md (PE).
- GPL corresponding-source adequacy not lawyer-reviewed (ASSUMPTION).
- H.264/HEVC patent licensing varies by country -> note in notices.
- cargo-packager no release for ~1 y -> fallback hand NSIS + hdiutil (ADR-014).
- Riedl macOS ffmpeg URL retention unknown -> CI fails loudly, re-pin. BtbN daily build pins expire -> re-pin month-end.
- Speech-to-text needs C++ toolchain + libclang on every CI platform (slow, fragile builds; whisper-rs-sys bindgen).
- Test runs: local rule-7 suite runs tests twice (cargo test + coverage.sh) [open, ADR-035]; the single llvm-cov run is CI only.

## Readable text, placement, accuracy (2.0.3 plan; plan ai-docs/architecture-plan-2.0.3.md)
- [2.0.3 decided 2026-11-20] Q-1 = B: the 2.0.2 text outline (8% black) disappears for old texts on opening (REQ-158 as written); Future work: the REQ-167 message may mention it. Q-2 = A: old texts change size on opening (REQ-156/157, REQ-167 count, no restore).
- [2.0.3 plan, D56] REQ-177 reworded: >= 90% of the MATCHED words within 0.3 s at Fast after the one-sided onset snap (ADR-164); the snap may not reach it (measured DTW was 0.43 to 0.75 s late; onsets within 0.03 s of the reference); then Andrea decides (larger model, D54). The PE stops and reports at that point; no silent gate change.
- [2.0.3 plan] libass may prefer the system font over the copy in the work `fonts/` folder (ADR-156): Linux test; macOS and Windows by hand (REQ-165).
- [2.0.3 plan] libass box padding for BorderStyle 3 and the outline width as padding (ADR-157): confirm by a pixel test; if not, the Box padding is drawn by a second outline value.
- [2.0.3 plan] "fill" of a box is the line box (lines x hhea ascender-descender), not the ink height (assumption, ADR-154); a reviewer may expect ink.
- [2.0.3 plan] harfrust/skrifa on odd fonts: any failure gives the bundled font (test); a font whose name ID 1 differs from its family name in fontdb may not be found by libass (ADR-156 detail).
- [2.0.3 plan] piece margins of 2 s: words longer than 2 s could be lost at a boundary (none expected in speech; the synthetic test covers 1.5 s words).
- [2.0.3 plan, D56] the one-sided snap misses words the Fast model mishears (5 of 28 on the reference); they are not matched and are not gated. Onset snap depends on the audio energy: noisy speech may give wrong onsets (test with noise is a PE decision).
- [2.0.3 plan] the blur fix (ADR-152) relies on the ffmpeg bound derived from one message; verified on 9.0.2 by a size sweep; other ffmpeg builds (system override) may differ.
- [2.0.3 plan] text.rs (1211 lines) and render.rs (3439) grow: PE may split the event builder (text/events.rs) if text.rs passes ~1500 lines.

## Hardware export (0.4.0+)
- No GPU in CI: real hardware only via QG-05 manual matrix; unverified combinations are known limitations.
- NVIDIA needs driver >= 610 (SDK 13.1 headers); older -> probe fails fast, software export, Advanced explains. Likely common late 2026.
- Linux VA-API/QSV need system libva + vendor driver (intel-media-driver / Mesa radeonsi); Fedora default Mesa lacks H.264/HEVC encode (AMD needs RPM Fusion mesa-va-drivers-freeworld). QSV on linux uses default render node only. Multi-GPU prefers NVIDIA.
- Bundled linux ffmpeg 9.0.2 needs libva >= 2.21 (vaMapBuffer2); Ubuntu 24.04 ships 2.20 -> VA-API and QSV skipped with the bundled ffmpeg on Ubuntu 24.04 (only system ffmpeg override uses them). Guard prevents core dump. Revisit: pin a BtbN build vs older libva, or bundle libva (driver ABI coupling).
- [open] QG-05 g (>= 40% faster) FAILED on Intel iGPU VA-API CQ (-14% / +41% / +6%): iGPU encode + check not faster than x264 veryfast. Known limitation; recommend re-baseline g or segment-only check render.
- VA-API CQ (ADR-037 deviation): content-dependent size/quality; QP 23 near the e limit on some sources; size held only by watch-restart + 1.5x check; oversize -> software, not remembered. QSV has no CQ variant.
- HEVC VA-API broken on Ubuntu free intel-media-driver 24.1 even at CQ -> HEVC software on such systems.
- libva byte scan ~3.1 s on the 173 MB bundled binary (background, once per session) delays first probe result (not in the run budget).
- Probe timeouts (0.4.2): a slow-failing candidate costs 5 s per probe and makes the answer inconclusive -> always repeated once -> ~13–20 s bg work + one Probe slot per trigger, possible dGPU wake on hybrid laptops. No distinction between busy computer and hanging candidate. 10 s total can still end "no hardware" under extreme load (repeat mitigates).
- PSNR 25 dB threshold and vendor bitrate factors are ASSUMPTIONS -> tune from QG-05.
- Corruption only between sampled frames passes the output check -> QG-05 d.
- Hardware presence of encoders in bundled binaries not verified from build scripts alone -> packaging smoke test.
- Check cost: strict decode + full graph render; on iGPU ~ as much as the software encode saving (QG-05 g fail).

## Speech and subtitles
- Speed and memory of the larger speech models are estimates; recommendation thresholds may move after measurement (A10).
- Whisper token timestamps ±0.1–0.3 s; 0.3 s alignment gate may be borderline with the Fast model (ASSUMPTION) -> fixture with clear pauses; PE reports measured error.
- Alignment quality depends on how closely a track follows the speech (translated or paraphrased tracks often fall back to even spread); cost ≈ full recognition of the range.
- Track cue times assume ffmpeg normalises to file start (ASSUMPTION) -> integration test MKV with start_time != 0.
- Pre-AVX2 x86-64 users: subtitles gate message (ADR-031); residual ASSUMPTION: static ggml init does not run AVX2 at start-up (optional qemu check, PE verifies).
- Projects saved by 1.2.0 and later cannot be opened by older versions (schema bumps; see ADR-051/065/076/093/104/124).
- Text edited after a drag can make the box cross the edge (no re-clamp); positioned lines overlapping in time overlap on screen (no collision avoidance); egui vs libass width may differ by pixels at edge clamp (ASS authoritative).
- [1.5.0 spike] Time-dependent sizes: accepted by ffmpeg 6.1.1; bundled 9.x not checked yet (roadmap).

## Interface
- Drag and drop does nothing on native Linux Wayland (winit 0.30): documented limitation; Import/Open menus remain. Re-check on eframe/winit upgrade.
- egui API churn (MenuBar/Popup/Panel) -> keep UI thin; logic in pure modules.
- Global shortcuts vs text fields (Delete/Backspace/Left/Right/Home/End) -> guarded by egui_wants_keyboard_input(), tested per key.
- AccessKit bounds inside ScrollArea are content-relative/clipped -> own UiCache.containers registry; if the helper grows > ~150 lines switch to egui_kittest.
- In-window menu bar on macOS (ADR-024) looks non-native; Hide/Minimize missing.
- Toolbar overflow with 15 actions at 800 px; shortcut collisions (1.2.6).
- Downgrade: 1.0 builds drop layout + recent fields on their next save (accepted).
- Edge handles vs picture drag (PE test); 1.6.1 Q-F rule approximate between key times for rare projects.

## 360 video and text
- Exporting a moving camera runs at about one third of real time or less at 1080p (accepted; speed work on roadmap: fewer commands per second, in-process projection).
- Camera stills while dragging an 8K source may feel slow; quick in-process view = Future work.
- Two copies of the same part of one source share their POVs (accepted; Future work).
- CI ffmpeg on Linux is distro apt (6.1): v360 command flags may differ from bundled 9.x (BtbN reset_rot/command semantics not re-measured; assumed same). Tests gated on the spike result.
- libass vs fontdb family names may differ (silent fallback in export only); text on the sphere narrows toward the poles (accepted, Q1 = A); slide leaves the view (ADR-077); blink event cost (capped); egui vs libass wrap.
- 360 video export slow (no speed promised); 360 file without spherical information stays flat (1.4.0).
- Spherical marker: MP4/MOV only (own writer, ADR-078); MKV/WebM unmarked; bundled 9.x not measured.
- Text best-fit size is an estimate (85–115% of the box) until 2.0.3 (ADR-154 measured fit, plan); migration by search may shrink large multi-line texts (confirmed by product owner, 1.4.0); REQ-167 count in 2.0.3.
- afftdn may muffle speech (gate measured on fixture).

## Frames, LUT, player, splash (1.5.0–1.6.2)
- Frame: longer side capped at 4096 px (decided 2026-11-19); Scale and Move on a 360 clip act on its flat camera view only; keyframes in a small popup are harder to use; text items keep their place when the picture moves (may end over the edge).
- Whole-video Scale/Move copies: a key before a new start is hidden, not lost; users expecting clips added later to get the effect (rule documented, no message).
- Q-F rule (1.6.1): combined path approximate between key times for rare projects.
- LUT: lut3d options on the bundled 9.x unverified (PE step); yuv round trip 1–2 levels (test tolerance); origin/licence of the splash image (see release item above).
- Player: duplicates ffmpeg in its installer (size); file handling limited (no file association, one file at a time); 1.5.x refusal of format 8 checked by hand only.
- Splash: stale or long summary fails the test (shrinks, not clipped); build breaks until both summaries exist (intended); both pictures compiled into both binaries (measure).

## Code structure and debt
- Large files: app.rs 3072 lines, render.rs 3437 lines, ui/mod.rs 582 lines; new logic goes into submodules (app/effects.rs, app/subs_create.rs, app/lut.rs, app/texts.rs, ...); further split = Future work.
- Import cycles (9 pairs, one 16-module group, app<->ui): planned for 2.0.1 in ai-docs/architecture-plan-2.0.1.md (base module, check in cargo test, ADR-136..141). Open until the Principal Engineer's step 8 and the release verification.
- camera.rs not refactored into keyframes (debt, ADR-095).
- Settings file: 1.0 builds drop unknown fields (accepted).
- Hardware probe: each VA-API candidate may cost 5 s per probe; cheaper probe is Future work.

## Process and tooling
- egui frequent breaking releases (ADR-012) -> thin UI; new stable lints -> fix promptly.
- Scope: largest so far; long CI -> milestones each green.
- ffmpeg build variance for user override or PATH -> startup capability probe + fallback; bundled copy has known caps.
- The 1.1.0 risk "app.rs grows" superseded by the 1.2.0 submodule split (see above).
- Documentation mismatch: docs/business-export.md REQ-061 still states "at most 1 second each, at most 3 seconds in total" (superseded by ADR-038 deviation 0.4.2); PM to reword.

## 2.0.4 (checkpoint 2026-11-21; plan ai-docs/architecture-plan-2.0.4.md § 7; stage numbers in brackets)
- [closed D61, Q-1] player under "Open with" only; the default is the user's choice. Manual check per OS (stage 9, item 22).
- [closed D61, Q-2; REQ-273 NOT BUILT] spatial sound: fixed cardioid pair to stereo; real-time CPU unmeasured; it stays a roadmap item (cut-back 1) until built.
- [closed D61, Q-3] no bundled CJK font: Chinese, Japanese and Korean depend on installed system fonts; a Linux system without them gets the note, not a silent box. Risk: rare CJK users see the note (stage 1, manual item 9).
- [closed D61, Q-4] AccessKit on all three systems; Linux AT-SPI adds crates and binary size, measured in stage 8 and recorded.
- [closed D61, Q-5] 360 export preset: the fastest preset within 1.5 times the Best-quality size, measured in stage 10.
- [closed D61, Q-6] Enhance speech: built as speechnorm named "Normalise speech", no model file; a bundled arnndn model would need its licence check first.
- [closed D61, Q-7] MKV and WebM 360 marker: built in place (spherical.rs, ADR-187); the real-player check is manual item 23.
- Hardware AV1 and VP9 on real systems: encoders rare, decoders unverified; the output check protects files, not speed (stages 5, manual 15).
- Busy-or-hanging classification under load: false "hanging" would skip a working candidate; tested with fakes only (stage 4).
- Preview hardware decode failure mid-playback: the restart must keep the last frame (stage 5).
- Export graph regressions (crop, fades, Clean audio strength, LUT strength; speed, per-clip frame and picture overlays are not built): golden argv gate for projects without the features; the new paths are new tests (stages 6, 11).
- REQ-222 says a quarter-size blur; built an eighth (BLUR_DIVISOR 8, measured, ADR-180 update); the requirement wording is the product manager's decision (checkpoint).
- Project size: LUT copies up to 20 MB and 64 MiB per project; read cap 96 MiB (stage 11, ADR-191).
- macOS menu bar: built as an allow-listed objc2 call (ADR-183, ADR-034 amended), checked for aarch64-apple-darwin only, not run; Wayland drop unverified on a real session (manual 17).
- Autosave: disk time for large projects every 2 minutes (stage 2); copy on the UI thread costs O(clips).
- Fonts raise every installer by the Arabic and Devanagari faces only (stage 1); the UI font parse happens on the first open of the subtitle list (measured). CJK depends on system fonts (D61).
- Stage 0 split: large diff with no behaviour change; the PE checks golden tests and the cycle test before any feature (ADR-168).
