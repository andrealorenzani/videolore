# Roadmap tasks

The [roadmap](roadmap.md) split into suggested versions and batches, in the order of implementation. This is a suggestion, not a commitment: Andrea chooses what is built and when.

## Version numbers
- Each development gets one new version: the last number goes up by one (the release of 1.6.3 opened 2.0.0). Batch N ships as 2.0.(N+1): batch 1 is 2.0.2, batch 15 is 2.0.16, except batches 2 to 4. 2.0.1 was the module-cycles refactor and is delivered.
- Batches 2, 3 and 4 (Readable text, Subtitle placement, Subtitle accuracy) are all built in 2.0.3 (one development).
- **All remaining batches (5 to 15 and the continuous items) are built together as version 2.0.4** (one development, REQ-180 to REQ-299, planned in the business plan). The per-batch headings below keep their planned versions as the record of the batch split; the roadmap items stay open until their status is recorded.
- Andrea can name a minor version (for example 2.1.0) for any batch.
- A batch may need more than one development; a development should not mix batches unless the items share code.

## Summary

| Version | Batch | Goal |
|---|---|---|
| 2.0.1 | delivered | Module-cycles refactor: shared helpers in one module, no module cycle, automatic cycle check |
| 2.0.2 | 1. Verify the release (delivered) | A green suite on the current code, the checks of the bundled ffmpeg 9.x and the splash files confirmed |
| 2.0.3 | 2. Readable text (built in 2.0.3) | Text stays readable on any picture and fits its box predictably |
| 2.0.3 | 3. Subtitle placement (built in 2.0.3) | Moved subtitle lines and text items stay inside the picture and are easy to find |
| 2.0.3 | 4. Subtitle accuracy (built in 2.0.3) | Reused and generated subtitles reach the accuracy target (the target decision REQ-177 is open) |
| 2.0.6 | 5. Subtitle files and fonts | Import and more export formats for subtitles; subtitle fonts for more scripts |
| 2.0.7 | 6. Editing gestures and safety | Autosave, timeline gestures and keyboard use |
| 2.0.8 | 7. Whole-video and keyframe clarity | Whole-video copies and keyframes are clear and easy to edit |
| 2.0.9 | 8. Faster hardware export | Faster really faster, and fewer wasted checks on busy computers |
| 2.0.10 | 9. Hardware reach | Hardware export verified on more systems and for more cases |
| 2.0.11 | 10. Export and preview speed | Measured speed of large frames, 4K, blurred fill and preview |
| 2.0.12 | 11. Platforms and installers | One window, file managers, Wayland, menu bar, installer checks |
| 2.0.13 | 12. Interface | Remembered windows, toolbar and shortcuts choice, accessibility |
| 2.0.14 | 13. Video player | Opening from the file manager, session positions, preferences |
| 2.0.15 | 14. 360 video | Anchored subtitles, 360 export details and camera |
| 2.0.16 | 15. Effects and frame | New effects and LUT options, each with its own requirements |
| continuous | Quality and measurements | Measurements and architecture debt, done alongside any batch |

## 2.0.1 — Module cycles (delivered)
- **Status**: delivered in 2.0.1. The shared helpers are in `src/base/`, the top-level modules form no cycle, and `tests/module_cycles.rs` fails `cargo test` when a cycle is added. Not a batch; see the [changelog](changelog.md).

## 2.0.2 — Verify the release (delivered)
- **Status**: delivered in 2.0.2; see the [changelog](changelog.md). The automated checks run in the test suite and in the automatic builds. The manual checks stay in [release verification](business-verification.md) until the maintainer records their results there.
- **Left over, placed in the roadmap**: the Windows and macOS test runs, the moving-camera measurement on 9.0.2, the UI frame gates under load, the hardware probe success fakes, the notices script after each version bump (Priority 1, 11 and 12 of the [roadmap](roadmap.md)). The paused-picture defect is the first item of 2.0.3.

## 2.0.3 — Readable text (batch 2, delivered in 2.0.3)
- **Status**: delivered in 2.0.3; see the [changelog](changelog.md). The remaining checks are in the [roadmap](roadmap.md), Priority 1 (font names and the cost of an installed font on macOS and Windows, REQ-165 and REQ-166).

## 2.0.3 — Subtitle placement (batch 3, delivered in 2.0.3)
- **Status**: delivered in 2.0.3; see the [changelog](changelog.md). Still open in the [roadmap](roadmap.md), Priority 2: text items and picture moves, and the libass checks on macOS and Windows.

## 2.0.3 — Subtitle accuracy (batch 4, delivered in 2.0.3 except the target)
- **Status**: delivered in 2.0.3 except REQ-177, the accuracy target of reused subtitles, which is planned and open: see the [roadmap](roadmap.md), Priority 1 (decision point) and Priority 3.
- **Still open in the roadmap**: the gated alignment test on CI's versions, the five-minute real-speech test, and the larger speech levels.

## 2.0.5 — Finish 2.0.4 (not built in 2.0.4)
- **Goal**: the items of 2.0.4 that were not built, and the engineering items that the new code needs first.
- **Items, in this order**:
  1. QG-14 test run and automatic build time; QG-15 test quality; QG-16 architecture debt (split the application module, the render module and the dispatch by area, one keyframe module for the 360 camera and the picture, before REQ-268 changes it); QG-17 splash and release items; QG-18 follow-ups of 2.0.2 and 2.0.3.
  2. REQ-281 clip speed 50% to 200%.
  3. REQ-286 per-clip Change aspect (reverses the project-wide rule; the project frame keeps one size).
  4. REQ-287 PNG picture items (3 s by default).
  5. REQ-288 drag Change aspect onto a clip (needs REQ-286 first).
  6. REQ-258 automatic subtitle lines anchored at the view centre.
  7. REQ-257 preset box and highlight on anchored lines.
  8. REQ-259 Re-anchor (needs REQ-258; one undo restores the old anchor).
  9. REQ-260 off-view arrows (needs anchors).
  10. REQ-268 per-clip camera paths (largest camera change; regression risk in the export graph).
  11. REQ-273 spatial sound (first-order ambisonic rotated to stereo; cut back if the real-time check fails).
  12. REQ-253 picture-based subtitles in the player (needs a real DVB or PGS sample; cut-back candidate).
  Also the manual checks of verification items 26, 29 and 30 (screen readers, Windows Open with, window places).
- **Why together**: all are the open items of the 2.0.4 development; the small effects go first (quick value, low risk), dependencies first, the riskiest last so that a cut-back costs least.
- **Depends on**: REQ-286 before REQ-288; REQ-258 before REQ-259 and REQ-260; QG-16 before REQ-268; real samples and Windows and macOS systems for REQ-253 and the manual checks.

## 2.0.6 — Subtitle files and fonts
- **Goal**: subtitles can be imported and exported in more formats, and scripts other than Latin, Greek and Cyrillic look right.
- **Items**: import .srt and .vtt files (with optional re-timing); WebVTT export; ASS export with positions; switchable subtitle track in MP4 and MKV; re-check the tracks of replaced files; subtitle fonts for other scripts; text from picture-based tracks (OCR, decide later).
- **Why together**: all are subtitle file work and touch the same export code.
- **Order inside**: WebVTT export, then ASS (needs positions), then import (re-timing uses batch 4); fonts need the licence check of each font, and can run in parallel.
- **Depends on**: batch 4 for import re-timing; the font licences are Andrea's check.

## 2.0.7 — Editing gestures and safety
- **Goal**: work is saved automatically, and the timeline can be edited with the mouse and the keyboard faster.
- **Items**: autosave and crash recovery (first); drag clip edges to trim; drag subtitle markers; drag fade markers; keyboard navigation of timeline markers; Ctrl/Cmd+V for clips; keep undo history when a file is removed; group actions on selected lines; snap and safe-area guides.
- **Why together**: all change the timeline's gestures and selection; autosave is independent and goes first because it protects work.
- **Depends on**: autosave must not change the project format (the project file is saved as it is); the gestures share the timeline hit code with batch 3.

## 2.0.8 — Whole-video and keyframe clarity
- **Goal**: a whole-video effect and its keyframes are clear, and clips added later are not a surprise.
- **Items**: whole-video copies diverge (show the copies apart, as decided by Andrea on 2026-11-20); message when clips are added after a whole-video effect; picture handles for every effect; keyframe popup usability; copy keyframes between clips; double-click on a clip that already has the LUT.
- **Why together**: all change how the whole-video copies and the popups behave.
- **Depends on**: none open. The picture handles follow the decision to show the copies apart.

## 2.0.9 — Faster hardware export
- **Goal**: Faster is quicker than Best quality on the common computers, and the check wastes less time on busy computers.
- **Items**: make Faster really faster on integrated graphics (cheaper check or per-kind target); limit the background work of the check; skip a candidate that timed out twice; tell a busy computer from a hanging candidate; show the repeat in Settings > Advanced; under extreme load the check can still find no hardware (accepted until measured).
- **Why together**: all change the check and its timing; the skip rule needs the busy-or-hanging test first.
- **Order inside**: measure the iGPU case first (the QG-05 target may change); then the busy-or-hanging test; then the skip rule and the background limit; then the Settings message.
- **Depends on**: Andrea's Intel laptop for the measurement; a new QG-05 baseline; the architecture plan of the check and of the failure memory rules.

## 2.0.10 — Hardware reach
- **Goal**: hardware export works on more systems and for more cases, and each system is verified.
- **Items**: verify hardware export on every system (Windows NVIDIA, Intel, AMD; Linux NVIDIA, AMD; macOS Apple); Ubuntu 24.04 with the included ffmpeg (libva 2.20); Fedora RPM Fusion note; Intel HEVC with newer drivers; constant-quality mode for Quick Sync; choose the graphics card; hardware for AV1 and VP9 and hardware decoding (in scope, decided by Andrea on 2026-11-20); hardware check of 360 export with texts; hardware export with frame effects.
- **Why together**: all need the same QG-05 checklist and the same test hardware.
- **Depends on**: batch 8 (the check is the base); real hardware of Andrea or of testers; the Ubuntu option (rebuild of ffmpeg or a shipped libva) is a decision for Andrea.

## 2.0.11 — Export and preview speed
- **Goal**: measured and, where needed, faster export and preview for large material.
- **Items**: export speed for 4K and HEVC presets; export speed of large frames (warn or limit, the 4096 px cap is an assumption); export time of blurred fill; faster blurred fill and Scale in the preview; hardware decoding for preview (if batch 9 allows); playback and export without waiting; measure Clean audio in the preview on weak computers (lighter values only if needed); codec names for non-experts.
- **Why together**: all are speed of picture processing and the export settings.
- **Order inside**: measure the cases first (4K, large frames, blurred fill), then decide on the warning or limit; the codec names are independent and can go in a separate development.
- **Depends on**: the measurements.

## 2.0.12 — Platforms and installers
- **Goal**: the programs behave like a normal window on each system; installers stay unsigned (decided by Andrea on 2026-11-20).
- **Items**: run the installer job before each release; one window for double-clicked projects (Windows and Linux); Linux file managers and the archive; native macOS menu bar with Hide and Minimize; drag and drop on Linux Wayland (depends on the library); pin the Windows installer toolchain; licence page encoding; detection in the installers (open question); builds of 1.2.0 on real systems; other systems (scope decision); old-processor check in CI; weekly security-advisory check; legal review of source offers (Andrea, outside code).
- **Why together**: all are installer or platform behaviour and CI checks.
- **Order inside**: CI checks first (installer job, advisories, old-processor check); then the platform items.
- **Depends on**: batch 1 (the installer configuration check); the legal review is Andrea's decision.

## 2.0.13 — Interface
- **Goal**: the window, popups and toolbar remember their state and suit each user; the program works with a screen reader.
- **Items**: remember window size, position and popup positions; better default popup positions; hide, collapse and dock sections; saved workspaces; customisable and icon toolbar; rebindable shortcuts; light and system theme; interface text size; translated interface; screen-reader support; group the Edit menu; multi-selection lost (report needed).
- **Why together**: all change the window's layout and settings; remembered positions come before better defaults.
- **Order inside**: remembered positions, then default positions; the rest by Andrea's choice; the screen reader depends on the toolkit's accessibility support.
- **Depends on**: the program settings rules (layout and window are program settings, never project).

## 2.0.14 — Video player
- **Goal**: the player opens videos from the file manager and remembers what is useful, without writing files next to the video.
- **Items**: file association for the player (installers); opening between the editor and the player (scope decision); subtitle positions kept in the player (session or a settings file); player preferences in a settings file; bitmap subtitle tracks (decoder work); playlist; the player's own third-party list; a lighter ffmpeg for the player (measure first); player sound for very large videos (preview copy with sound).
- **Why together**: all are player behaviour; the file association and the list come first because they are the most used.
- **Depends on**: batch 11 (file association needs the installers); the third-party list is needed before any player release with a new bundled component.

## 2.0.15 — 360 video
- **Goal**: 360 subtitles and texts stay where they were put, and 360 export is clear about what it keeps.
- **Items**: preset box and word highlight for 360 subtitles; 360 subtitles anchored to the sphere; automatic subtitles keep their preset place (anchor or choose); indicator for off-view items; 360 marker in Matroska and WebM; spherical check on real players; 360 files without spherical information (ask at import or mark); video format presets for 360 (decide); frame effects in 360 export (apply or say so); faster 360 export and moving-camera export; GPU or SIMD path for reprojection; instant look-around; separate points of view per clip; starting direction; camera roll; easing between points of view; keyboard control; spatial sound that follows the camera; stereo 3D, cube maps and 180° video (scope decision).
- **Why together**: all are 360 video behaviour and its camera.
- **Order inside**: the subtitle and text items first (most visible); then the export details; then the camera; the large camera items wait for measurements.
- **Depends on**: the camera items share the keyframe code with batch 15 (the 360 camera and the picture share rules; see the continuous quality items). The 360 export prioritises speed (decided 2026-11-20); its speed requirement is written when this batch is planned.

## 2.0.16 — Effects and frame
- **Goal**: new effects and LUT options, each with its own requirements written first.
- **Items**: length of each fade; one-sided boundary fades; effects on music clips; clean the music track; Clean audio strength; speech enhancement; volume over time; more effect types; keyframes for other effects; easing curves; crop tool; blur strength; free aspect ratios; aspect ratio per clip; other objects on the picture; Change aspect tooltip and drag; LUT strength; other LUT formats (.3dl, .csp); LUT input ranges other than 0 to 1; missing LUT file on export; LUT copied into the project; colour management for LUTs; LUT order with the frame effect; LUT libraries; exact conversion of combined Scale and Move.
- **Why together**: all are new effect scope; each one needs requirements and tests before code.
- **Order inside**: small LUT and Change aspect items first; the missing-file and copy items protect projects (they come before the others); the new effects wait for their requirements.
- **Depends on**: the requirements of each item (business documents) before the architecture plan is written.

## Continuous: quality and measurements
- **Items**: measured quality goals; measure the Apply LUT cost; record 4K and 8K frame times; headless frame-time baseline; faster test suite; shorter check job on push (QG-06); speech fixture for CI tests; preview copies with a lighter preset; regenerate cleared preview copies at once; preview copies for 720p sources; limit the preview copy cache; large-file memory test; stable computer identity on virtual machines; interface tests with a test harness; pointer tests of the colour picker and keyframe timeline; click the Export dialog's dropdowns in UI tests; golden argument tests for format 4 and 5; split the application module (with dispatch_inner); smaller 360 pictures in the integration tests; one keyframe module for the 360 camera and the picture; shared core crate (only if a third program is planned); the Mac and Windows builds of 1.2.0; test memory cap as a project rule; package list for the player on crates.io (only if published); splash picture size in the programs; clearer message for a missing splash summary; measure the cycle check's CI cost; cycle check limits; wording of the leaves layer; player notice file version line.
- **Splash, project page and release items**: animated splash image (decision on REQ-134 wording first); do not show again; splash summary from the changelog; missing splash picture or summary (build without the splash or not); smaller project page images; social preview image; Content-Security-Policy for the project page.
- **How to use**: the measurements go with the batch they measure (for example the 4K frame times with batch 10); the architecture items go when the code they touch is changed.

## Dependencies between batches (short)
- 2.0.2 (batch 1) before everything (the release checks and the test reliability).
- 2.0.3 (batches 2 to 4) delivered: the text box and its width are used by the placement rules.
- Batch 4 (delivered in 2.0.3) before the subtitle import re-timing in 2.0.6 (batch 5).
- 2.0.5 (the open 2.0.4 items) follows 2.0.4; its order is inside the section.
- 2.0.9 (batch 8) before 2.0.10 (batch 9): the hardware reach uses the check.
- 2.0.12 (batch 11) before 2.0.14 (batch 13): the file association needs the installers.
- 2.0.15 (batch 14) and 2.0.16 (batch 15) share the camera and keyframe code: the camera items wait for the shared module.

## Decisions Andrea needs to make
- Accuracy target of reused subtitles (REQ-177, 2.0.3): a larger speech model, or keep the target with its known limitation (default: keep planned).
- The Ubuntu 24.04 hardware export: a different ffmpeg build or a shipped libva (2.0.10).
- Scope of the 360 video export: presets and frame effects (2.0.15).

## Decided by Andrea (2026-11-20)
- Installers stay unsigned; code signing and notarisation are not planned (2.0.12). See [Not planned](roadmap.md#not-planned).
- The copies of whole-video keyframes are shown apart, one per clip (2.0.8).
- AV1 and VP9 encoding and hardware decoding are in scope (2.0.10).
- The Export dialog keeps its presets; custom resolutions, frame rates and bitrates are not planned (2.0.11). See [Not planned](roadmap.md#not-planned).
- Hardware export always goes through ffmpeg (no separate vendor pipeline).
- The 360 video export always prioritises speed (2.0.15).
