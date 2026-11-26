# ai-notes: risks and open items (owner: architect; index in ai-docs/architecture.md)

Resolved items are removed. [OPEN] needs a decision or measurement; [TECH] technical risk accepted; [MANUAL] checked by hand only. The human list: docs/architecture-risks.md.

## Open decisions and measurements
- [DECIDED, Andrea, REFINE 2026-11-26] Q-A: WAV into MKV (ADR-262). Q-B: X11 and XWayland only; native Wayland refused at Start with a message (ADR-265, REQ-358). Q-C: computer sound needs macOS 14.6 at Start; installer minimum 11.0 (ADR-259, ADR-269).
- [TECH, 3.1.2 built] Launch on macOS 11 to 14.5: CoreAudio linked weakly through .cargo/config.toml (`-weak_framework`, not dlsym; CI `nm -m` check of the process-tap symbols); NOT RUN on macOS 11 to 14.5 yet (A-3.1.2-13).
- [TECH, 3.1.2 built] A-3.1.2 checks: Linux (pinned ffmpeg devices, pulse `@DEFAULT_MONITOR@`, Xvfb and PulseAudio capture test, amix, MP4 and MKV recordings) done on the dev box and in CI; NOT verified: avfoundation (screen by name `Capture screen N`, ADR-258), gdigrab negative offsets, cpal loopback on Windows, the macOS tap, display-info MSRV (none declared), libx264 real time (not measured), playlist after end (A-3.1.2-12 as built, ADR-267).
- [TECH, 3.1.2 built] macOS Info.plist keys in `packaging/macos/screen-spy-info.plist` (NOT verified on macOS: prompts unchecked); CGPreflight and CGRequest screen capture access use safe objc2-core-graphics bindings (no unsafe block).
- [TECH, accepted, Andrea Q-F1 2026-11-24, 3.1.0] format 15 on every save (ADR-244): 3.0.x refuses every project saved by 3.1.0 or later; release note line required.
- [TECH, PE, 3.1.0] blurred fill cause in the clip path (ADR-250: steps.rs frame_row gate or SegFrame aspect; UI popup as fallback A-3.1-3); Enlarge baseline anchor from the font descent (A-3.1-4, ADR-248); proxy layout bump re-encodes proxies once (ADR-247); T2 for a non-first track is a time cost (ADR-246, future: `-map 0:a:k` copy).
- [OPEN, Andrea] REQ-177 subtitle timing (2.0.3 D56, 2.2.0): NOT MET (10 of 23 = 43 %, target 90 %; FROZEN None). A second clip (about 30 s real speech) is the next check; a pace-aware method is needed. Continuous speech may fail the frozen setting.
- [OPEN, Andrea] REQ-315 Faster on integrated graphics: NOT MET (8.7 % against 20 %; the rest is the hardware encode phase). Levers (b) (0.1 s) and (c) (hardware decode of the check) not taken. Decide: re-baseline per hardware class, or a cheaper check.
- [OPEN, product] REQ-222 blur divisor: built 1/8 (measured, ADR-180); the requirement says 1/4. PM decides the wording.
- [OPEN, Andrea] picture items (REQ-287, ADR-192): no count (20), file size (8 MiB) or pixel limit in code; picture kept by path. Add limits in `picture.rs`/`text`, or record that the header read suffices.
- [OPEN, PE to record] speed-first 360 preset (Q-5): the rule is built; the preset name is not in the notes.
- [OPEN, 2.1.0 spike result] boxed shadow: design A (libass draws the box fill from OutlineColour) or design B (vector box, must match 2.0.4 geometry at 1080 px); spike S0d recorded in the plan.
- [OPEN, spike] Enlarge word: in-house layout vs libass at 1080 px (S0b); fallback decided with Andrea if it misses 2 px.
- [OPEN] AccessKit size on Linux (Q-4 closed; size measured in 2.0.4 stage 8, recorded where?).
- [OPEN, PE] module-cycle result of 2.2.0 and 2.3.0: confirmed at VERIFY (2.3.0 cycle test green per the PE; the list changed).

## Hardware export (0.4.0 to 2.0.4)
- [MANUAL] real hardware only via QG-05 (no GPU in CI). QG-05 g FAILED on Intel iGPU VA-API CQP (−14 % / +41 % / +6 %). [OPEN] re-baseline g or segment-only check render.
- [TECH] NVIDIA needs driver ≥ 610 (SDK 13.1); older: probe fails fast, software export, Advanced explains.
- [TECH] Linux VA-API and QSV need system libva and a vendor driver (intel-media-driver, Mesa radeonsi); Fedora's default Mesa lacks H.264/HEVC encode (RPM Fusion `mesa-va-drivers-freeworld`); QSV uses the default render node only.
- [TECH] bundled Linux ffmpeg 9.0.2 needs libva ≥ 2.21 (`vaMapBuffer2`); Ubuntu 24.04 ships 2.20 → VA-API and QSV skipped with the bundled build (guard prevents a core dump). Revisit: pin a BtbN build with older libva, or bundle libva (driver ABI coupling).
- [TECH] VA-API CQP (ADR-037 deviation): size and quality depend on the content; QP 23 near the quality limit on some sources; size held only by the watch-restart and the 1.5x check; oversize → software, not remembered. QSV has no CQ variant. HEVC VA-API broken on Ubuntu's free intel-media-driver 24.1 even at CQP → software.
- [TECH] libva byte scan ~3.1 s on the 173 MB binary (background, once per session), delays the first probe result.
- [TECH] probe timeouts (0.4.2): a slow-failing candidate costs 5 s and makes the answer inconclusive → always repeated → 13 to 20 s background work and one Probe slot per trigger (possible dGPU wake on hybrid laptops). A busy computer and a hanging candidate are not told apart; 10 s total can still end "no hardware" under extreme load. A cheaper probe is future work.
- [ASSUMPTION] 25 dB PSNR threshold and vendor bitrate factors → tune from QG-05. Corruption only between sampled frames passes the output check (QG-05 d). Encoder presence in bundled binaries confirmed only by the smoke test.
- [TECH] check cost: strict decode plus full graph render; on iGPU about as much as the saving.

## Speech and subtitles
- [TECH] Whisper word times ±0.1 to 0.3 s; the 0.3 s gate may be borderline with the Fast model (ASSUMPTION); the one-sided onset snap misses words the Fast model mishears (5 of 28 on the reference); not gated. Noisy speech may give wrong onsets.
- [TECH] alignment depends on how closely a track follows the speech (paraphrased tracks spread evenly); the cost is about a full recognition.
- [ASSUMPTION] cue times relative to the file start (MKV with start_time ≠ 0 test).
- [TECH] piece margins of 2 s: words longer than 2 s could be lost at a boundary (synthetic test covers 1.5 s).
- [TECH] pre-AVX2 x86-64: subtitles message (ADR-031); static ggml init residual assumption (optional qemu check).
- [TECH] speed and memory of larger models are estimates (A10).
- [TECH] afftdn may muffle speech (gate measured on the fixture).
- [TECH] projects saved by 1.2.0 and later cannot be opened by older versions (schema bumps; ADR-051, 065, 076, 093, 104, 124, 169, 222, 230).
- [TECH] text edited after a drag may cross the edge (no re-clamp); positioned lines overlapping in time overlap on screen (no collision avoidance); egui and libass width may differ by pixels at the edge clamp (ASS authoritative).
- [TECH] libass may prefer the system font over the work copy (`fonts/`, ADR-156): Linux tested; macOS and Windows by hand (REQ-165). libass box padding for BorderStyle 3 and outline as padding (ADR-157): pixel test; else a second outline value. Box fill is the line box (ADR-154 assumption; a reviewer may expect ink). Font name ID 1 may differ from the family name in fontdb (libass may not find it).
- [TECH] the blur fix (ADR-152) relies on an ffmpeg bound derived from one message; verified on 9.0.2 by a size sweep; a system ffmpeg override may differ.

## Interface and platform
- [TECH] drag and drop does nothing on native Linux Wayland (winit 0.30); Import and Open remain. Re-check on eframe or winit upgrades.
- [TECH] egui API churn (MenuBar, Popup, Panel): thin UI, logic in pure modules; new stable lints fixed promptly.
- [TECH] global shortcuts vs text fields guarded by `egui_wants_keyboard_input()`, tested per key.
- [TECH] AccessKit bounds inside ScrollArea are content-relative: own `UiCache.containers`; switch to `egui_kittest` if the helper grows beyond about 150 lines.
- [TECH] in-window menu bar on macOS (ADR-024) looks non-native; Hide and Minimize missing.
- [TECH] toolbar overflow with 15 actions at 800 px; shortcut collisions (1.2.6).
- [ACCEPTED] 1.0 builds drop the layout and recent fields on their next save; settings: 1.0 drops unknown fields.
- [TECH] edge handles vs picture drag (PE test); 1.6.1 combined-path rule approximate between key times for rare projects.
- [MANUAL] macOS launch-time open-document timing, Gatekeeper on the nested ffmpeg (fallback `xattr -dr com.apple.quarantine /Applications/videolore.app`), SmartScreen on unsigned installers, Linux file managers (tar.gz ELF double-click; README fallback), the Windows player "Open with" (not run on Windows), macOS menu bar (ADR-183, checked for aarch64-apple-darwin only, not run), Wayland drop on a real session. Open-document failure: in-app Open still works.
- [TECH] "Hide" and "Minimize" absent on macOS (ADR-024).

## Release and tooling
- [TECH] GPL corresponding-source adequacy not lawyer-reviewed (ASSUMPTION). H.264 and HEVC patent licensing varies by country (notice).
- [TECH] cargo-packager has had no release for about a year: fallback hand NSIS and `hdiutil` (ADR-014).
- [TECH] Riedl macOS ffmpeg URL retention unknown (CI fails loudly, re-pin). BtbN daily build pins expire (re-pin to month-end).
- [TECH] speech-to-text needs a C++ toolchain and libclang on every CI platform (slow, fragile; whisper-rs-sys bindgen).
- [OPEN, ADR-035] local suite runs tests twice (cargo test and coverage); CI uses one llvm-cov run.
- [TECH] wall-clock gates can fail on a busy machine: best-of-5 sampling (ADR-144); load run in the release checklist.
- [TECH] the pinned test ffmpeg adds a download and cache to the check job (QG-06 time); the suite tests the pinned build on Linux only (Windows and macOS 9.0.2 untested by the suite).
- [TECH] the format 8 fixture is hand-written; the real 1.6.0 file is checked by hand.
- [TECH] ffmpeg build variance for a user override or PATH: startup capability probe and fallback.
- [DOC] docs/business-export.md REQ-061 still says "at most 1 s each, at most 3 s in total" (superseded by the 0.4.2 deviation, ADR-038); PM to reword.

## Frames, LUT, player, splash, pictures
- [TECH] Frame: longer side 4096 px (decided 2026-11-19); Scale and Move on 360 act on the flat camera view only; keyframes in a small popup are hard to use; texts keep their place when the picture moves (may end over the edge).
- [TECH] whole-video Scale and Move copies: a key before a new start is hidden, not lost; clips added later do not get the effect (documented, no message).
- [TECH] LUT: `lut3d` options on the bundled 9.x unverified (PE step); YUV round trip 1 to 2 levels (test tolerance); a LUT that lifts black shows on a faded frame (new projects, ASSUMPTION); a 256-point LUT: about 450 MB Work copy and about 293 MB disk per Work folder, plus ffmpeg float table about 201 MB per process (measured 225 MB RSS).
- [TECH] player: duplicates ffmpeg in its installer (size); one file at a time; format 8 refusal checked by hand only.
- [TECH] splash: a stale or long summary fails the test (shrinks, not clipped); both pictures compiled into both binaries (size to measure); origin and licence recorded (1.6.3).
- [TECH] picture subtitles: none on 360 clips; a subtitle on screen at a run start appears at its next change; no drag. Spatial sound: four channels read as W, Y, Z, X (SN3D assumed); a file with another layout turns wrongly; roll not applied; the preview waits for the pre-pass (clips over 10 minutes keep recorded sound); real-time CPU on weak hardware unmeasured.
- [TECH] 360: a moving camera exports at about one third of real time or less at 1080p (accepted; roadmap: fewer commands per second, in-process projection); camera stills from an 8K source may feel slow; two copies of the same part share their POVs (accepted); CI ffmpeg is distro apt 6.1 for the picture tests (`reset_rot` and command semantics not re-measured on 9.x; assumed same); libass vs fontdb names (silent fallback in export); text on the sphere narrows toward the poles (Andrea Q1 = A); slide leaves the view (ADR-077); a 360 file without spherical information stays flat (1.4.0); the marker is MP4, MOV, MKV and WebM (MKV and WebM in place, ADR-187; the real player check is manual).
- [TECH] text best-fit size estimate 85 to 115 % of the box until 2.0.3 (ADR-154 measured fit); migration by search may shrink large multi-line texts (confirmed by the product owner, 1.4.0).
- [TECH] afftdn may muffle speech (gate on the fixture).
- [TECH] Settings file cap 64 KiB; project read cap 96 MiB; LUT copies 20 MB each, 64 MiB per project (ADR-191).
- [ACCEPTED] the 2.0.2 Q-1 = B: old text outline (8 %) disappears for old texts on opening (REQ-158); Q-2 = A: old texts change size on opening (REQ-156, 157; REQ-167 count, no restore).

## Code structure and debt
- [DEBT] large files: `app.rs` ~2100 core lines after the 2.0.4 split; `render.rs` split to `render/`; `ui/mod.rs` large; `text.rs` split (2.0.4). New logic goes into submodules; further splits are future work.
- [DEBT] `camera.rs` not refactored into keyframes (ADR-095).
- [DEBT] the cycle check is syntactic (cycles through macros not seen); planned for 2.0.1 (ADR-136 to 141) and done.

## Plan-era items now built (kept for history)
- 2.0.4 (ADR-168 to 201): closed decisions D61 (Q-1 player Open with only; Q-2 spatial pair; Q-3 no bundled CJK; Q-4 AccessKit; Q-5 360 preset rule; Q-6 speechnorm not arnndn; Q-7 MKV marker in place). Open at build: [TECH] Wayland drop unverified; busy-or-hanging classification tested with fakes only; the preview hardware decode failure mid-playback restarts at the playhead, keeping the last frame (stage 5).
- 2.1.0: Q-1 = B (edge 256, 512 MiB, background load with progress); Q-2 = A (whole frame graded); Q-3 = A (sphere-fixed items are fixed rows); REQ-307 kept as written. The Work copy of 256^3 LUTs is about 450 MB; the golden argv of projects with LUTs changes (`lut-<hash16>.cube`).
- 2.2.0: Q-1 route D (LUT about 12 s start, unprocessed picture meanwhile, D63); format 13 refused by 2.1.0 (release notes say so); whole-video removal changes what 2.1.0 users see (accepted by Andrea; release notes say it); deviation from REQ-321 for new rows (default place); column minimum 180 px (about 20 % at 1280 x 800; business accepted).
