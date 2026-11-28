# ai-notes: PLAN 3.2.0 (2026-11-28; target 3.2.0 named by Andrea; planned, not built; owner: architect)

Decisions: ai-docs/architecture-adr-3.2.0.md (ADR-276..284). Requirements: REQ-367..377 (docs/business-timeline.md, business-editing.md, business-player.md, business-screen-spy.md).

## Verified facts (2026-11-28)
- Cargo.toml is already at 3.2.0 (version applied by the command). Next free REQ REQ-378; next free ADR ADR-285.
- Project format: SCHEMA_VERSION 15 (src/project.rs:45); 3.2.0 writes 16 (ADR-280).
- Editor clean audio: `afftdn=nr=25:nf=-20:tn=0:rf=-60` (src/render.rs:51, ADR-080); `afftdn` is in the list of filters checked in src/ffmpeg.rs:39.
- Ruler: src/ui/ruler.rs (`STEPS` seconds table, `ticks`, `label`, `LABEL_GAP`).
- Jobs: src/jobs.rs `Priority` = Preview 0, Export 1, Probe 2, Background 3; `Scheduler::runner`.
- Preview frames: src/preview.rs `Frame { pts, width, height, rgba }`.
- Player: player/src/playlist.rs `Playlist { items: Vec<PathBuf>, current: Option<usize> }`; menu in player/src/ui.rs `menu_bar` (File, View, Audio, Subtitles, Help); the Next/Previous buttons at ui.rs ~337.
- Scratch names: src/scratch.rs (`.partial` names, `is_partial_name`, `cleanup_stale`).
- Recorder: screen-spy/src/plan.rs `sound_input` (`-f pulse -i @DEFAULT_SOURCE@` / `@DEFAULT_MONITOR@`), `filter_graph` (amix normalize=0, alimiter level=false), `record_args`. screen-spy depends on the videolore library with default features off.
- image 0.25.10 with features ["gif"] (Cargo.toml:38); png 0.18.1 already in the tree through eframe (cargo tree -i png).
- Read-only check on the report's machine: ALSA card 0 "HDA Intel PCH", mixer Realtek ALC295; Capture Volume 63/63 (max, +30 dB by its scale), Internal Mic Boost Volume 2/3 (+20 dB), source volume 76 %; default source alsa_input...analog-stereo (internal); ffmpeg 6.1.1 (Ubuntu), pulse input defaults 48000 Hz, 2 channels.

## Components touched (new in CAPS)
- src/ui/ruler.rs: `marks` (new pure function), steps (new constant), labels unchanged.
- src/ui/timeline.rs: pointer line (new overlay), strip painting in the lower half of clips.
- src/strips.rs (NEW): planner (pure: grid step, sample times, box size), LRU cache (32 MiB), job submission (Priority::Background), cancel on change.
- src/nested.rs (NEW): project as media: source load (bounded), reference walk (depth 8), self-import refusal, key (SHA-256), intermediate render job (Priority::Export through export::run), rebuild on reopen.
- src/media.rs: `MediaKind::Project { source }`, item field `from`.
- src/project.rs: SCHEMA_VERSION 16, load 1..16, save 16, refusal above 16.
- src/scratch.rs: name pattern of the intermediate (`<32 hex>.mkv.partial` and `<32 hex>.mkv`).
- src/settings/interface.rs: `show_picture_strips` (default true); settings.json version 2 additive field.
- src/app/ (import command, popup state), src/ui/menu.rs (File > Import project as media…), src/ui/popups.rs (the REQ-372 popup).
- player/src/playlist.rs: `durations`, `move_up`, `move_down`, `remove`, `total`.
- player/src/ui.rs: Playlist menu, total label `[hh:mm:ss]`, File > Take screenshot.
- player/src/screenshot.rs (NEW): crop of the picture rectangle, PNG encode, save path (pure parts tested).
- Cargo.toml: `image` features ["gif", "png"]. No new crate.
- screen-spy/src/plan.rs: `filter_graph` with `afftdn` before `alimiter`, `[rec]` delayed by the measured delay L; screen-spy/src/tests.rs and a fixture test.

## Ordered steps for the Principal Engineer (tests with the code; one suite run at the end)
1. Project format 16 (ADR-280): MediaKind::Project, load 1..16, save 16, refusal of 17. Tests: load of 15 files, save writes 16, a 17 file refused and not written. Covers REQ-371 (part), REQ-029 kept.
2. Nested project module (ADR-279): bounded load, reference walk, self and cycle refusal (depth 8), key, intermediate render through export::run with the preset, `.partial` then rename, scratch names. Tests with a small fixture project made in the test (temporary folder, tiny clips, ffmpeg from the bundled build). Covers REQ-371, REQ-373.
3. Import UI and reopen (REQ-371, REQ-372): File > Import project as media…, popup with OK and Cancel (Cancel adds nothing), "analysing" and "rendering" states, rebuild on reopen when the key changed, missing source refused with its name. Tests: popup per import, Cancel leaves the list unchanged, one media item of the total length, split and fade on an imported item (REQ-373 acceptance).
4. Ruler marks (ADR-276): `marks` with steps 1/fps, 1, 60, 600, 3600 at MARK_GAP 6 px; labels per REQ-093 unchanged. Tests at three zooms (frame, second, minute) and at the minimum and maximum zoom no overlap of marks or labels. Covers REQ-367.
5. Pointer line (ADR-276): drawn only while hovered; time under the line equals the click time within one frame. Covers REQ-368.
6. Picture strips (ADR-277, ADR-278): planner, cache with cap, background job with cancel, setting in Settings > Interface (default on; off = no snapshot, lower half blank; restart keeps it). Tests with a fake runner: a 10 s clip where five snapshots fit gives five, first at the clip's start; zoom out gives fewer; a clip shorter than one space gives one or none; the setting off makes no job. Covers REQ-369, REQ-370.
7. Player playlist (ADR-281): move, remove, current kept, Next and Previous follow the order; durations probed once per item; total shown from two items on, dashes until all known. Covers REQ-374, REQ-375.
8. Take screenshot (ADR-282): Viewport screenshot, crop to the picture rectangle, PNG through the `image` png feature, save dialog in the Pictures folder, name `<stem> hh.mm.ss.mmm.png`; nothing written next to the video. Tests: crop rectangle maths, name, PNG round trip of a small image, no write when the dialog is cancelled. Covers REQ-376.
9. Recorder noise (ADR-283): `afftdn` on the mixed track before `alimiter`; fixture test (synthetic steady noise plus speech-like bursts, fixed seed) for at least 10 dB lower quiet parts and speech within 3 dB; measure the filter delay L on the fixture and apply it to the `[rec]` branch; a test that the graph has afftdn before alimiter and the delay term. Manual check on Linux (microphone recording) stays with Andrea. Covers REQ-377.
10. Feature annotations `// covers: REQ-367` … `REQ-377` on the tests; the Principal Engineer's VERIFY gate (fmt, clippy -D warnings, test, coverage ≥ 85, feature coverage 100 %).

## Assumptions
- A-3.2-1: 10 dB noise target and 3 dB speech limit are assumptions checked on the fixture (REQ-377).
- A-3.2-2: intermediate render preset (crf 12, veryfast) keeps the picture close enough for edits; not measured.
- A-3.2-3: Pictures folder exists on the three systems (fallback: home).
- A-3.2-4: egui 0.37 `ViewportCommand::Screenshot` returns the frame on Linux (X11, Wayland), macOS and Windows; to verify.
- A-3.2-5: ruler and strip thresholds (6 px, 4 px gap, 64 px box) are technical choices.

## Risks
- R-A1: project-as-media rebuild is slow and the item changes with its source (popup, release note; R-T1).
- R-A2: a moved source project refuses the item (R-T2).
- R-A3: the microphone can record the computer sound (echo) when the speakers are on; not addressed (system echo cancellation is a system setting).
- R-A4: noise reduction may thin quiet speech (R-T4; fallback to microphone-only filter, ADR-283).
- R-A5: the 3.2.0 project format is refused by 3.1.x (release note).
- R-A6: readback screenshot shows the frame drawn at that moment (R-T5 subtitle lines included).

## NFR and security targets (for the VERIFY gate)
- Frame ≤ 200 ms with strips decoding (REQ-056 rule, test with fake runner); strip cache ≤ 32 MiB; one strip job at a time; process cap of ADR-015 kept.
- Recorder noise: quiet parts ≥ 10 dB lower, speech within 3 dB (fixture); picture-sound offset within one frame after the measured L is applied.
- Imported project: bounded read 16 MiB, depth 8, no shell, `-protocol_whitelist file,pipe`, intermediate in the 0700 work folder, crash clean-up names.
- Screenshot: PNG only to the chosen path; nothing next to the video.
- Measured by PE and reported: intermediate size per minute (crf 12), import time per minute of source, strip decode time per thumbnail (targets to be set after measuring; NOT MEASURED yet).

## Test plan summary
Unit tests per module in its tests file; ffmpeg tests use the bundled build (pinned); no network; no microphone (fixture only). Feature coverage per REQ-367..377 with `// covers:`.
