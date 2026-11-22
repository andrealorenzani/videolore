# Roadmap tasks

The [roadmap](roadmap.md) split into suggested versions and batches, in the order of implementation. This is a suggestion, not a commitment: Andrea chooses what is built and when.

## Version numbers
- Each development gets one new version: the last number goes up by one. After the release of 2.3.0 the next version is 3.0.0; the batches below are the developments 3.0.1 to 3.0.10 in that order.
- A batch may need more than one development. Andrea can name a minor version (for example 3.1.0) for any batch.
- Format changes (project file) are grouped where possible, because each one makes older versions refuse the newer file.

## Summary

| Version | Batch | Goal |
|---|---|---|
| not a version | Release checks (Priority 1) | Manual checks, measurements and the decisions Andrea must make |
| 3.0.1 | Text and editing fixes | Texts stay useful when the picture or the video moves; safer editing |
| 3.0.2 | Overlap media follow-up | Overlaps are easier to edit and the large-file limits are settled; the one project format change of the batch |
| 3.0.3 | Subtitle accuracy | Reused subtitles reach the accuracy target, or the decision is recorded |
| 3.0.4 | Export and hardware speed | Measured and faster Faster, export of large frames and blurred fill |
| 3.0.5 | Hardware reach | Hardware export verified on more systems and cases |
| 3.0.6 | Platforms, installers and interface | Platform behaviour, Quit item, Wayland drops, ruler and multi-selection |
| 3.0.7 | Video player | Picture subtitles and a lighter player package |
| 3.0.8 | 360 video | 360 export details, sound and sphere-fixed items |
| 3.0.9 | Effects, frame and LUT | Clean audio default, effect order copies, LUT safety |
| 3.0.10 | Quality and project page | Shared keyframe module, split of the large modules, CI times, release notes |
| continuous | Measurements | Done with the batch they measure |

## Not a version — Release checks (Priority 1)
- **Goal**: the checks that need real systems, hardware and ears, and the decisions the maintainer makes.
- **Items**: manual checks of 2.0.4; measurements of 2.0.3; the REQ-177 decision and the second speech clip; the REQ-315 decision on integrated graphics; player builds on real systems; the 1.5.x format 8 check; Clean audio and MP3 by ear; Windows and macOS test runs; the moving-camera measurement on 9.0.2; one release run to review the process.
- **Depends on**: Andrea. The results go to [release verification](business-verification.md).

## 3.0.1 — Text and editing fixes
- **Goal**: texts stay useful when the picture or video moves; editing protects the user.
- **Items**: text follows the picture (or a warning); text follows video edits (decision); font scan repaint; longer text times (decision); migrated text sizes (only if asked); double-click on a clip with the LUT; keyframe popup usability; time-based LUT progress; undo steps when a media file is removed.
- **Why together**: all change text placement or editing behaviour in the same screens.
- **Order inside**: the decisions first (text follows edits, text in .srt); then the fixes.

## 3.0.2 — Overlap media follow-up
- **Goal**: overlaps are easy to edit, and the limits and the large-file cost are settled.
- **Items**: drawn handles on the overlap box; re-probe MOV at use; lane zoom; full-size preview of large overlays (after measuring); memory and time of large overlay files; alpha in hardware decoders (software path cost); MOV sound (decision); number of overlaps per clip (decision); split keeps overlaps per half (check); default place of new picture rows (check); picture item limits (decision); removal of old picture items (format change, only when no project needs it); overlay keyframes and rotation (format change, only if asked); overlap in 360 export (decision).
- **Why together**: all are overlap and picture-item behaviour; the format changes go into one development.
- **Order inside**: measurements and decisions first; then handles and lane; the format changes last, with the project format test.
- **Depends on**: the 16 MP limit and the overlay format of 2.3.0; a 2.3.0 project fixture for the format test.

## 3.0.3 — Subtitle accuracy
- **Goal**: reused and generated subtitles reach the 90% and 0.3 s target, or the decision is recorded and the strict test is set.
- **Items**: meet the accuracy target (REQ-177); pace-aware method judged on the second clip; five-minute real-speech test (REQ-178); gated alignment test on CI's versions; larger speech levels measured; replaced files re-checked; subtitle fonts for other scripts (licence check of each font); Enlarge word and Typewriter in subtitle files (decision).
- **Why together**: all change the alignment and subtitle export code.
- **Depends on**: Andrea's decision on the target and the second clip `tests/fixtures/align/second-clip-30s.wav`. Without them the batch is only the measurements and the font licence checks.

## 3.0.4 — Export and hardware speed
- **Goal**: Faster is quicker than Best quality on common computers, and export of large material is measured and faster.
- **Items**: make Faster really faster on integrated graphics (cheaper check, or a target per kind of graphics); decode and upload speed; busy-or-hanging test and skip rule; 4K and HEVC preset speed; large-frame export (warn or limit); blurred fill export time; playback of the blurred fill and Scale in the preview; Clean audio cost in the preview.
- **Why together**: all change the export and preview pipelines and their timing.
- **Order inside**: measure first (iGPU, 4K, large frames, blurred fill); then the check; then the preview.
- **Depends on**: Andrea's decision on REQ-315 and a QG-05 baseline on an Intel laptop.

## 3.0.5 — Hardware reach
- **Goal**: hardware export works on more systems and cases, and each system is verified.
- **Items**: verify on Windows (NVIDIA, Intel, AMD), Linux (NVIDIA, AMD) and macOS (Apple); Ubuntu 24.04 with the included ffmpeg (decision: ffmpeg build or libva); Fedora note; Intel HEVC with newer drivers; choice of the graphics card; hardware check of 360 export with texts; hardware export with frame effects; alpha decoding on real graphics (with 3.0.2).
- **Why together**: all need the same QG-05 checklist and test hardware.
- **Depends on**: 3.0.4 (the check is the base); real hardware of Andrea or testers.

## 3.0.6 — Platforms, installers and interface
- **Goal**: the programs behave like normal windows on each system.
- **Items**: Wayland drag and drop (when the library supports it); macOS Quit item; idle heartbeat for the one-window lock; detection during installation (decision); builds of 1.2.0 on real systems; sub-second ruler labels; multi-selection lost (report needed).
- **Why together**: window, menu and lock behaviour.
- **Depends on**: the windowing library's support for Wayland drops.

## 3.0.7 — Video player
- **Goal**: the player shows every subtitle and is smaller.
- **Items**: picture subtitles on 360 clips; picture subtitles at a start or a pause; lighter ffmpeg build for the player (measure first).
- **Depends on**: nothing open; the 360 subtitles follow 3.0.8 if they share its code.

## 3.0.8 — 360 video
- **Goal**: 360 export is clear about what it keeps, and the sound and the sphere-fixed items are right.
- **Items**: faster 360 export and moving-camera export; GPU or SIMD path for reprojection (if the look-around target is missed); instant look-around; roll in the spatial sound; four-channel sound order check; spatial sound pre-pass in the background; spherical information check in real players; video format presets and frame effects in 360 export (decision); mixed 360 and flat clips; movable order of sphere-fixed items; Enlarge word and Typewriter on 360 clips; stereo 3D, cube maps and 180° (scope decision, last).
- **Why together**: all are 360 behaviour and its camera.
- **Order inside**: subtitle and text items first (most visible); then export details; then the camera.
- **Depends on**: 3.0.10 (the shared keyframe module) before the camera items.

## 3.0.9 — Effects, frame and LUT
- **Goal**: new effect choices are safe for existing projects; LUT work keeps the colours the user expects.
- **Items**: Clean audio default on quiet speech; keyframes for the text shadow; remove several whole-video copies at once; per-line order in the Applied effects widget; copy the order between clips; audio fades after the sound group (check); LUT colour management (decision); LUT loading cancelled on close; Find the LUT file read in the background; exact conversion of combined Scale and Move (only if a user reports a change); more effect types (each with its own requirements first).
- **Why together**: all change the effect model and the order of effects.
- **Order inside**: requirements first for any new effect; the small fixes (cancel on close, background read) first.
- **Depends on**: requirements in the business documents before any new effect.

## 3.0.10 — Quality and project page
- **Goal**: the code stays healthy, the largest modules are split, and the project page and release notes are right.
- **Items**: one keyframe module for the 360 camera and the picture; split the application module and the graph module, with `dispatch_inner` split by area; smaller 360 pictures in tests; faster test suite; CI time measurements (QG-06); cycle check cost and limits; interface tests with a test harness (only if the helper grows); preview copies with a lighter preset and copies for 720p sources; cache cleanup on a worker; large-file memory test; stable computer identity; splash items (animated picture decision, summary from the changelog, missing-file behaviour, picture size); project page images and Content-Security-Policy hashes; notices after each version bump; release notes of the format changes; test memory cap as a workflow rule; player package list (only if published); shared core crate (only if a third program is planned).
- **Why together**: all are code health and release process; they go alongside the batches that touch the same code.
- **Order inside**: the keyframe module before 3.0.8's camera items; the splits when the code they touch changes.

## Dependencies between batches (short)
- Release checks (decisions) before 3.0.3 and 3.0.4 (Andrea's decision on REQ-177 and REQ-315).
- 3.0.2 (overlay format) before any other format change; format changes are kept in 3.0.2 and 3.0.9 only, with one fixture test each.
- 3.0.4 before 3.0.5: the hardware reach uses the check.
- 3.0.10 (keyframe module) before 3.0.8 camera items.
- 3.0.7's picture subtitles on 360 clips follow 3.0.8 when both change the same 360 subtitle code.

## Decisions Andrea needs to make
- REQ-177: a larger speech model, or keep the target with its known limitation.
- REQ-315: accept the 9% on integrated graphics, or keep the 20% target open.
- Ubuntu 24.04 hardware export: an ffmpeg build or a shipped libva (3.0.5).
- Text follows the picture and the video edits (3.0.1).
- Scope of 360 export presets and frame effects (3.0.8).
- Sound of MOV overlaps, and the overlap limits (3.0.2).
- LUT colour management (3.0.9).
- Clean audio default on quiet speech (3.0.9).
