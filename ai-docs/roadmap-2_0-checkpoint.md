# ai-notes: 2.0 roadmap checkpoint (temporary; delete when 2.0.4 is released)

Shared checkpoint file. Each agent adds its own `## <agent>` section and edits only its own section (Edit, never overwrite).

## Documenter

### Written for 2.0.4 so far
- Version: 2.0.4 set by the business part and consistent (`scripts/version.sh check`); unchanged since.
- `docs/changelog.md` 2.0.4 entry: business bullets (written by the product manager). The documenter's architecture bullets were reverted: they described unbuilt work and live only here.
- `docs/roadmap.md`: the four future items I had added (quarter-size blurred fill, 360 preset and marker results, arnndn licence, architecture "planned" wording at release) were reverted. They are kept in this section below.
- `docs/architecture-2.0.4-quality.md`: my network-line correction was reverted (the line still says "the only network use is the consented speech model download"; this contradicts `docs/architecture-2.0.4-overview.md`, which says no network model downloads for Enhance speech. Architect to decide).
- README: not changed by the documenter in 2.0.4.
- `ai-docs/notes.md`: index refreshed, one dev-log line per part.
- `docs-check`: ok at the last run.

### Left for the documenter
- Part `code` (after the code checkpoint is final): coverage badge with the value the principal-engineer reports (`scripts/coverage.sh --badge <NN.N>`, no test run); changelog bullets for the built stages (see the built list in `ai-docs/code-checkpoint-2.0.4.md`); roadmap: delete the items that 2.0.4 delivers (only after the product manager marks them, see roadmap rules); docs pages that list what 2.0.4 contains (only built things); README latest changes (2.0.4 is user-facing; keep the last three versions).
- Part `close`: `scripts/version.sh check`, `scripts/docs-check.sh`, README latest changes and badges, poem unchanged (POEM mode only at the next major), notes dev-log line.
- Release flow (`/release`): RELEASE-CLEAN (compact docs, known-limitations hand-off in `ai-docs/notes.md`, `github_page/after release.md` Documenter section); RELEASE (changelog `### Release 2.0.4` summary, README coverage badge and Latest changes, docs-check); POEM after the next major is opened.

### Traps found
- A combined shell command with a long `python3` step and `scripts/docs-check.sh` stalled once and went to the background; run each check with `timeout` and in a separate call.
- Roadmap and changelog index lines in `ai-docs/notes.md` shift with every edit; refresh them with `grep -n '^#'` after each change.
- Stray scratch paths: `/tmp/x` was created by mistake and removed; scratch scripts live in the session scratchpad and are not durable.
- Unbuilt 2.0.4 items must not appear in human docs until built (the reverted items above). Plan pages (`docs/architecture-2.0.4-*.md`, `docs/business-*.md`) are plans and are marked "planned"; that is allowed.
- `docs/roadmap.md` still lists items that 2.0.4 covers in plan (360 marker in other containers, Video format presets for 360 video, Clean audio strength, Speech enhancement, Export time of blurred fill, codec names). They stay open until the code part marks them delivered.
- Architecture wording "planned 2.0.2/2.0.3" in `docs/architecture*.md` is to be removed at release clean-up.

### Future work kept here (not in human docs until built)
- Blurred fill at a quarter size: REQ-222 quarter-size blur against the current eighth (BLUR_DIVISOR 8, ADR-180); measure the visible difference and the time saved in stage 6.
- Record the 360 preset and marker results: fastest 360 preset within 1.5 times the Best-quality size; the Matroska and WebM 360 marker spike results (marker written in place, ADR-187); check with ffprobe.
- Licence of the Enhance speech model: the arnndn model bundled only if its licence is permissive; otherwise "Normalise speech". Current build uses speechnorm without a model file (ADR-194).
- Architecture wording after release: remove "planned 2.0.2/2.0.3" from `docs/architecture*.md` during release clean-up.

### Commands
- `scripts/version.sh check`
- `scripts/docs-check.sh`
- `scripts/coverage.sh --badge <NN.N>` (value from the principal-engineer report only)
- Index refresh: `grep -n '^#' docs/roadmap.md docs/changelog.md`, then update the lines in `ai-docs/notes.md`

## Product Manager

Checkpoint of 2.0.4 (asked by the user). Tree: nothing committed, version 2.0.4, no REQ marked `(implemented)` yet (all REQ-180..299 are `(planned)` in `docs/`). "Built" below = the Principal Engineer's checkpoint (`ai-docs/code-checkpoint-2.0.4.md`) plus a `// covers:` test for the REQ in `ai-docs/code-req-tests.md`. The product manager did not run tests.

### State of REQ-180..299 (120 REQs)
- Built with `// covers:` tests, to be marked implemented at STATUS (108):
  - 180-233 (batches 5 to 11, stages 1 to 7; notes in `docs/code-2.0.4-batches-5-7.md` and `-8-11.md`): all 54 built. REQ-233 is "no code" (the X11 drop path, File menu kept; test `src/ui/ux_tests.rs`).
  - 234-247 interface (stage 8): all 14 built.
  - 248-252 and 254-256 player (stage 9): 8 built (248 with the Windows deviation below).
  - 261-267 and 269-272 360 (stage 10): 11 built.
  - 274-279, 282-285, 289-294 effects and LUT (stages 11): 16 built.
  - 295-299 continuous (stage 12): 5 built.
- Not built (12), stay `(planned)`: 253, 257, 258, 259, 260, 268, 273, 280, 281, 286, 287, 288. None has a `covers:` test.
  - Conflict to settle: `docs/code-2.0.4-batches-12-15.md` line 42 describes volume keys (REQ-280) as built, but there is no REQ-280 test and the PE checkpoint says not built. Treated as not built; the PE corrects the batch note.
- Amended planned lines whose old text stays until built (map in `ai-docs/business-requirements.md`): REQ-011, 020, 033, 038, 041, 061, 069, 071, 075, 082, 083, 084, 096, 100, 101, 102, 103, 104, 112, 114, 120, 124, 126, 127, 128, 129, 130, 131, 133, 135, 136, 140, 147, 148.

### REQs still to do, in order (suggestion; Andrea may reorder)
1. REQ-280 volume keys in percent (small; needs its test and the batch note fixed).
2. REQ-281 speed 50-200 %.
3. REQ-286 per-clip Change aspect (reverses the 1.6.1 project-wide rule; project frame keeps one size).
4. REQ-287 PNG picture items (3 s default).
5. REQ-288 drag Change aspect onto a clip (needs 286 first).
6. REQ-258 automatic subtitle lines anchored at the view centre.
7. REQ-257 preset box and highlight on anchored lines (amends the 1.3.0 plain-text limitation).
8. REQ-259 Re-anchor (needs 258; one undo restores the old anchor).
9. REQ-260 off-view arrows (needs anchors).
10. REQ-268 per-clip camera paths (largest change: the camera model is keyed by source in many places; regression risk in the export graph).
11. REQ-273 spatial sound (first-order ambisonic rotated in Rust, fixed stereo pair; cut back to the roadmap if the real-time check fails, ADR-189).
12. REQ-253 picture subtitles in the player (needs a real DVB or PGS sample, because the pinned ffmpeg cannot make one from text; the overlay changes the shared render graph; cut-back candidate to the roadmap).
Then, without REQ: QG-14..18 (speech fixture for CI, pointer tests, splash size per program, test-memory rule in the developer docs, UI frame gates as preview tests, text.rs split QG-16); human checks in `docs/business-verification.md` items 26, 29, 30 (screen readers, Windows Open with, window places).
Order logic: small effects first (quick user value, low risk); dependencies first (286 before 288; 258 before 259 and 260); the riskiest items last, so a cut-back costs least.

### User decisions in force for 2.0.4
- D57: one development, version 2.0.4, builds all remaining batches (5 to 15) and the continuous items.
- D58: project format 11, one bump; 2.0.3 and older refuse it (TooNew).
- D59: program settings added (window, popups, workspaces, toolbar, shortcuts, theme, text size, language, graphics card, favourite LUTs, splash option, cache limit); player preferences in their own file (REQ-252).
- D60 (Andrea, AskUserQuestion): interface English only; texts in one table (REQ-245).
- D61 (Andrea, AskUserQuestion, 2026-11-21):
  - Player: "Open with" entries only, never the default; the default is the user's choice (REQ-248).
  - Spatial sound: first-order ambisonic rotated to stereo, fixed cardioid pair, preview and flat export only (REQ-273).
  - CJK: system fonts only, no CJK font bundled; the note when no font covers a script (REQ-187, 188). The 25 MB budget covers Arabic and Devanagari.
  - AccessKit on Windows, macOS and Linux (REQ-246).
  - 360 export preset: the fastest preset within 1.5 times the Best-quality size (REQ-266).
  - Enhance speech: speechnorm ("Normalise speech") instead of arnndn; no model file (REQ-279). The arnndn licence item is closed.
  - MKV and WebM 360 marker: in-house Matroska writer, written in place, conditional on passing the ffprobe check (REQ-261).

### Deviations to confirm at STATUS
- REQ-202 (reading): a clip has one Scale and one Move, so the dashed handle set is the whole-video copy selected in the open whole-video popup when it belongs to another clip than the one under the playhead; a drag changes only the copy its handle belongs to. Ask: accept this reading? If yes, STATUS amends REQ-202 to it.
- REQ-195 (clipboard marker): Copy and Cut put a marker text on the system clipboard; Paste is bound to the paste event, so it works whatever the clipboard holds. Ask: accept that other applications see the marker text when they paste?
- REQ-182 (track-enabled bit): the spec says the separate track is not switched on by default. The PE's detail is not in the checkpoint. PE must state which flag is written and its default before STATUS; it must match "not on by default".
- REQ-183 (scope): PE must state the built scope against REQ-085 (not in the checkpoint). Spec: range or whole video, one undo step.
- REQ-206 (short check): spec: a one-second range around each of three sample frames. Built: the check renders only the sample frames. PE confirms the ranges match the spec before STATUS.
- REQ-222 (blur divisor 8): spec says a quarter; built an eighth (`frame::BLUR_DIVISOR` 8). Measured at 4K: 2.9-3.1 s against 3.3-3.7 s for a quarter. Options: (a) accept an eighth and amend REQ-222 (PM recommendation: cheaper, the 2.0.3 output stays, each side below the named quarter); (b) PE codes a quarter.
- REQ-244 (collapsed header): spec says "main section". Built: Text size sits in a collapsed "Appearance and start-up" header of Settings. Options: (a) accept, amend REQ-244 to name the header; (b) PE moves it to an open section.
- REQ-279 (speechnorm): spec is arnndn with a bundled model if permissive, else speechnorm. Built: speechnorm only, by Andrea's answer. STATUS amends REQ-279 to speechnorm "Normalise speech", no model file.
- REQ-292 and REQ-293 (LUT missing and LUT copies): PE flagged them without detail. Built: the Find / Export without the LUT / Cancel modal (`Modal::MissingLut`); project copies with limits written as "20 MB / 64 MB, project limit 96 MiB" in the batch note. Spec: a LUT above 20 MB is refused. PE states what the 64 MB and 96 MiB limits are; the extra limits become MINOR defaults in the decisions file.
- REQ-248 (Windows NSIS): built as own template `packaging/windows-player.nsi` with "Open with" entries only (`OpenWithProgids`), not run on Windows (verification item 29). Options: (a) mark implemented, the Windows check stays a manual item (PM default); (b) keep REQ-248 planned until a Windows run.

### REQ-177 (open decision, untouched in 2.0.4; stays `(planned)`)
Options (`ai-docs/business-decisions.md` § Open questions): (a) timing method with a different speech model (consent, size, checksum); (b) restate the target; (c) keep planned with a known limitation (default if no answer).

### STATUS done (2026-11-21)
- Marked `(implemented)`: 109 REQs (180..252, 254..256, 261..267, 269..272, 274..280, 282..285, 289..299). Each has a `// covers:` line by grep. Still `(planned)`: 253, 257-260, 268, 273, 281, 286-288, and 177.
- Deviations accepted as built (user had not answered), amended in the REQ text: 202, 195, 222, 244, 248, 279, 293; 183 and 182 amended with the built behaviour; 206 spec confirmed by the batch note (one-second ranges), no change. Open for Andrea: see ai-docs/business.md § Checkpoint 2.0.4 and business-decisions.md § Open questions.
- REQ-280 was listed as not built in the checkpoint, but `covers: REQ-280` tests exist (render/tests.rs, tests/ffmpeg_integration/v204.rs); marked implemented per the user's list.
- Future work: 11 not built (reasons in the "REQs still to do" list above); verification items 26, 29, 30; REQ-177; QG-14..18 in the 2.0.5 batch of roadmap_tasks.

### Revert check (owned docs)
- Checked `docs/business*.md`: the 2.0.4 texts are requirements (`(planned)`) and decisions, which stay. Nothing describes unbuilt work speculatively, so nothing is reverted there.
- Process notes (State, Open and the list of changed docs) moved from `ai-docs/business.md` into this section; that file now points here.

## Principal Engineer

State: nothing committed, version 2.0.4 unchanged, no REQ marked implemented. Groups 1 to 4 are in the working tree. Full per-area detail: `docs/code-2.0.4-batches-12-15.md`, `docs/code-index.md`.

Built (all have `// covers:` tests): stage 8 REQ-234..247 (`src/settings/interface.rs`, `src/ui/{place,theme,strings,view_menu}.rs`, `src/ui/layout.rs:arrange`, `src/ui/keymap.rs`, `src/app/dispatch_interface.rs`); stage 9 REQ-248..252, 254..256 (`player/src/{settings,playlist}.rs`, `src/siblings.rs`, `packaging/windows-player.nsi` never run on Windows); stage 12 REQ-295..299 (page CSP style hash must be recomputed when the style block changes); stage 10 REQ-261..267, 269..272 (`src/spherical.rs`, `src/camera.rs`, `src/preview.rs:draft_geometry`, `src/ui/picture.rs`); stage 11 REQ-274..280, 282..285, 289..294 (`src/timeline.rs`, `src/music.rs`, `src/render/graph.rs`, `src/keyframes.rs`, `src/frame.rs`, `src/lut.rs`, `src/project.rs:lut_copies`). REQ-280 assumes `t` on the audio chain is the clip's own time (checked on bare ffmpeg, not on a full exported graph).

Left, in order: REQ-286 then 288; 281 (speed); 287 (PNG items); 257..260; 268 (per-clip camera, migrate the source-keyed `CameraPaths`); 273 (spatial sound); QG-14..18; REQ-253 (needs a bitmap-subtitle fixture: hand-written minimal PGS or VobSub with a licence note, and an overlay in the shared graph guarded by the golden argv).

Decisions: REQ-279 is `speechnorm` (no model); 282 export is a smoothstep; 289 blends the table with the identity; 293 keeps the normalised cube, project limit 96 MiB, copies 20 MB and 64 MB total; 292 modal replaces the refusal; 263 mark is not an undo step; Text size is in a collapsed Appearance header; blur divisor 8; cache limit runs on the UI thread. `Clip` is `Copy`: non-Copy per-clip data goes in `Timeline` maps.

Suite (last full green before REQ-280): fmt, clippy (also no-default-features), 803 lib, 87 integration, 7 module_cycles, 2 page, 44+3 player, coverage 95.0 %, feature 178/178, notices and docs-check ok. After REQ-280 the clip-details test needed the music controls moved to the end of the popup; rerun the full suite and coverage before trusting these numbers.

Commands: scratchpad `/tmp/claude-1000/-home-andrea-workspace-videolore/b3c0d406-9d64-4088-8a02-f78f8ab68794/scratchpad` has `env.sh` (pinned ffmpeg 9.0.2 in `ff9/`, `VIDEOLORE_REQUIRE_FFMPEG=1`, `CARGO_INCREMENTAL=0`) and `run.sh` (8 GB cap); every cargo run goes through `run.sh`; `rm -rf target/debug/incremental` often, delete `target/debug` before `scripts/coverage.sh --fail-under 85` (about 10 minutes, run in the background); add interface texts with `py/addkeys.py '{"Name":"Text"}'`.

Flakes: `app::hw_tests::a_failing_hardware_decoder_falls_back_without_a_message` once under load; UI tests that drag on the picture depend on popup places (set `settings.interface.popups`).

Traps: `Timeline.fade_length` 0 means 0.5 s; `camera::start_camera` must stay in `camera.rs` (module cycle); ffprobe gives no initial view for Spherical V1 MP4; Matroska pose IDs 0x7673..0x7675; egui moves focus on arrow keys without a focus lock filter; patch scripts for struct literals must skip `-> Type {` bodies; `Pov`, `Key`, `FrameAspect`, `MediaFormat` gained fields; lavfi `sine` is 1/8 full scale; rustfmt splits long `strings.rs` entries; the clip-details popup scrolls at 420 px so new controls go at the end.

Final safety state (stop point): the tree compiles; nothing half-built. Quick checks run once: fmt ok; clippy `--workspace --all-targets -D warnings` clean; `cargo test --workspace` with the pinned ffmpeg: 804 lib (2 ignored), 88 integration (3 ignored), 7 module_cycles, 2 page, 44 + 3 player, all green; feature coverage and notices `--check` ok at the last run; the real `scripts/coverage.sh` was started in the background and its number (last known 95.0 %) was not read: rerun it. REQ-280 (volume keys) IS built and tested (docs line is correct), but its graph offset (`t` is clip time) is verified only on bare ffmpeg.

Deviations to keep in mind: REQ-182 (MP4 separate subtitle track: the disabled flag is written by clearing the tkhd enabled bit in `mp4tracks`, because ffmpeg always enables the first track of a kind; MKV uses -disposition); REQ-183 (whole-video scope = every range of the file's source on the timeline); REQ-292 (a missing LUT at export now opens a three-button modal instead of refusing; "Export without the LUT" is for that export only); REQ-293 (the copy is the normalised cube, project limit 96 MiB, copies 20 MB each and 64 MB in all; a copy over the limit keeps only the path).


## Architect

Checkpoint of 2026-11-21 (asked by the user). Sources: `ai-docs/architecture-plan-2.0.4.md` (main), `-stages-1.md` (stages 0-5), `-stages-2.md` (stages 6-12), `ai-docs/architecture-adr.md` § 2.0.4 PLAN, `ai-docs/code-checkpoint-2.0.4.md`, `ai-docs/code.md`, and the tree (`Cargo.toml`, `src/`, `packaging/`, `.github/workflows/`). Nothing was run; nothing committed.

### Plan files (ai-docs, owner architect; archived in the release snapshot after release)
- `architecture-plan-2.0.4.md`: scope, facts found by reading, approach, components by stage, format 11 by stage (ADR-169), stage order and dependencies, cut-back list (items 1-9), Q-1..Q-7 closed by Andrea (D61), risks, future work.
- `architecture-plan-2.0.4-stages-1.md`: stages 0-5 (template: REQs, modules, data, tests per REQ, CI, performance, security, risks, manual items, depends on and cut-back, ADRs).
- `architecture-plan-2.0.4-stages-2.md`: stages 6-12 (same template).

### What each stage assumes, and its state
| Stage | Assumes | State at checkpoint |
|---|---|---|
| 0 split | nothing; golden argv tests and module-cycle test guard it | built (render/ and app/ submodules) |
| 1 subtitle files, fonts, format 11 | stage 0; the one format bump (D58); Arabic and Devanagari OFL faces, CJK from system fonts (D61) | built (REQ-180..188; SCHEMA_VERSION 11; both faces in assets/fonts) |
| 2 gestures, autosave | stage 0 and the format 11 writer; a project clone on the UI thread is O(clips) | built (REQ-189..199; autosave.rs, app/recovery.rs) |
| 3 whole-video and keyframes | stage 0; popups of the harness | built (REQ-200..205); no file `ui/keyframe_timeline.rs` in the tree (ADR-176 update) |
| 4 hardware check | nothing earlier; existing outcheck and jobs; sysinfo for CPU time | built (REQ-206..211) |
| 5 hardware reach | stage 4 check base; stage 0 | built (REQ-212..219; hwdec.rs; Codec Av1 and Vp9) |
| 6 export and preview speed | stage 5 (preview.rs shared); the measured blur size | built (REQ-220..224; BLUR_DIVISOR 8) |
| 7 platforms, installers | nothing earlier; Windows and Linux file locks; macOS menu spike | built (REQ-225..233; installer-config.yml, advisories.yml, instance.rs, macOS menu) |
| 8 interface | stage 3 popups; stage 7 single window and settings | built (REQ-234..247; accesskit feature on) |
| 9 player | stage 7 installers and sibling names; stage 1 preview cache rules | built except REQ-253 (248, 249..252, 254..256; siblings.rs, player/src/settings.rs, playlist.rs) |
| 10 360 video | stages 0, 1 (format 11), 3 (keyframe widget) | partly: 261..267, 269..272 built; 257..260, 268, 273 not built |
| 11 effects and frame | stage 6 (blur cache keys), stage 10 (camera and keyframe model), stage 0 | partly: 274..279, 282..285, 289..294 built; 280, 281, 286, 287, 288 not built |
| 12 continuous quality | stage 11 (settings complete), stage 2 (cache names), stage 1 (font notices) | built (REQ-295..299); QG-14..18 not built |

### ADR status and updates recorded on 2026-11-21
- ADR-168..201 are built as planned, except the ones in the next lists. The status line of the 2.0.4 block in `ai-docs/architecture-adr.md` says so.
- Updates recorded now (ai-docs/architecture-adr.md § 2.0.4 PLAN, and docs/architecture-decisions.md):
  - ADR-034 amended: one more allow-listed unsafe call, `initWithTitle:action:keyEquivalent:` on objc2-app-kit `NSMenuItem`, in the macOS part of `src/desktop.rs` (menu of ADR-183). Unsafe sites are now stt.rs (whisper abort callback) and desktop.rs (macOS odoc handler and menu).
  - ADR-180: blur divisor 8 (an eighth of each side; REQ-222 text says a quarter). Measured with ffmpeg 9.0.2: 4K source (2160x3840 blur), 8 s, 3 runs: divisor 8 = 2.9-3.1 s, divisor 4 = 3.3-3.7 s; 1080p (1080x1920): 1.4 s vs 1.5-1.6 s. Background cache = raw RGBA file `bg-<hash>.rgba` in the private still folder, one at a time, stills only (ffmpeg needs a file input); not an in-memory image as planned.
  - ADR-181: installer job is the reusable workflow `.github/workflows/installer-config.yml`, called by build.yml (`needs`) and run alone on pull requests with the path filter; advisories.yml built; named test `cpu_message`.
  - ADR-183: objc2-app-kit 0.3.2 (macOS only, minimal features) builds the menu in `desktop.rs` (Hide, Hide Others, Show All, Minimize, Zoom; no Quit, Cmd+Q stays with egui's prompt). Built and clippy-checked for aarch64-apple-darwin only, not run. The muda spike result is not in the notes (open).
  - ADR-184: Windows player installer uses the custom NSIS template `packaging/windows-player.nsi` ("Open with" entries only, `OpenWithProgids`; cargo-packager would register the default handler). Not run on Windows.
  - ADR-187: MKV and WebM 360 marker written in place (`src/spherical.rs`): the Void element ffmpeg leaves as padding between SeekHead and Info is shrunk by the bytes that the Tracks element grows; nothing after Tracks moves. Matroska pose IDs 0x7673..0x7675 (0x7672 is ProjectionPrivate). No Q-7 cut.
  - ADR-191: LUT strength is a blended table (`lut.rs` `with_strength(percent)`: every entry mixed with the identity by the percent; 0 = no change, 100 = the table). Project read cap 96 MiB (`MAX_PROJECT_BYTES` 16 MiB to 96 MiB, `project.rs`). LUT copies raw, 20 MB each, 64 MiB per project.
  - ADR-194: Enhance speech built as `speechnorm` named "Normalise speech" (`render.rs` `NORMALISE_SPEECH` = `speechnorm=e=6.25:r=0.0001:l=1`); no model file, no arnndn; the arnndn licence check is not needed unless a model is added later.
  - ADR-186: built except per-clip camera paths (REQ-268). The preset chosen by the speed-first rule is not in the PE notes (open).
  - ADR-190: built: fade length and sides, crop, Clean audio strength mix. Not built: speed (REQ-281), volume keys (REQ-280), per-clip frame (REQ-286).
  - ADR-176: the keyframe widget has no file `ui/keyframe_timeline.rs`; the PE notes do not name the file. Architect checks the location at release.
  - ADR-197: eframe feature `accesskit` on; accesskit_winit 0.34.2 in Cargo.lock. The size measurement asked by Q-4 is not in the notes (open).
  - ADR-185, 188, 189, 192: not built (see the next list).
  - ADR-201: QG-14..18 not built (see the next list).
- Human docs: docs/architecture-2.0.4-overview.md and -quality.md rewritten to the built state; unbuilt designs removed from them (kept in this section); docs/architecture-projects.md format 11 section and docs/architecture-decisions.md updated. docs/architecture-2.0.4-quality.md network line: the only network use stays the consented speech model download (whisper); Enhance speech adds none (answers the Documenter's note).

### Not built: designs kept here, removed from the docs
- REQ-253 (ADR-185), player picture subtitles: the bitmap stream overlaid in the same playback process; no extra process; not draggable; listed as "picture". Blocker: the pinned ffmpeg cannot make a DVB or PGS fixture from text; the overlay changes the shared render graph (golden argv guard). Cut-back item 4.
- REQ-257 (ADR-188), 360 subtitles keep the preset box and word highlight in the sphere-frame ASS events (the ADR-069/071 path); amends REQ-112.
- REQ-258, automatic lines anchored at the view centre when generated (test: yaw 30; after a turn to 0 they sit 30 degrees aside; export keeps yaw within 1 degree).
- REQ-259, Re-anchor for selected lines and texts; one undo.
- REQ-260, off-view arrows at the nearest picture edge (pure `arrow_for(item, camera) -> edge, direction`); a click turns the view.
- REQ-268 (part of ADR-186), per-clip camera paths: `Clip.camera`; load migration copies each media-keyed path into the clips that use the media; copies diverge. Blocker: `CameraPaths` is keyed by source in many places (PE). Largest change; regression risk in the export graph.
- REQ-273 (ADR-189, Q-2), spatial sound: first-order ACN/SN3D; yaw and pitch rotation of decoded PCM in Rust; fixed cardioid pair to stereo; preview on the audio thread; export pre-pass to a 32-bit float WAV with a fixed name in the work folder, then mix; no time-varying ffmpeg pan. Estimates: at most 10 % of one core; pre-pass at most the clip duration; cut-back item 1 if real time fails. Test: fixture sound left, camera 90 degrees left: front within 3 dB.
- REQ-280 volume keys in percent: `Clip.volume_keys` (Vec of Key<u8>), gain (p/100)^2 (D12), through the expression generator (ADR-099). Gate: 0 % at 0 s, 100 % at 4 s, 50 % at 2 s = gain 0.25; preview and export agree. PM conflict: `docs/code-2.0.4-batches-12-15.md` describes REQ-280 as built; treated as not built (no `covers:` test; see the PM section).
- REQ-281 speed 50-200 %: `Clip.speed_permille` 500..=2000 (default 1000); setpts and an atempo chain; subtitle and text event times mapped (REQ-018). Gate: 4 s at 200 % lasts 2 s; pitch within 1 %; subtitle within 0.1 s of its word.
- REQ-286 per-clip Change aspect (`Clip.frame`): composed in its own frame, then padded into the project frame; no stream copy for that clip; cut-back if the graph regresses (largest graph change). REQ-288 (drag onto a clip) needs it.
- REQ-287 (ADR-192), PNG picture items: new `pictures.rs` (id, name, PNG embedded, path for display, start, ttl 3 s default, place, width); at most 20 items, 8 MiB each, 16 megapixels decoded (ImageReader limits before pixels); `image` crate `png` feature (today only `gif`, cargo-deny check); overlay per item with `enable`. Cut-back item 3 if the 200 ms frame gate breaks.
- QG-14: speech fixture for CI in place of espeak-ng where equivalent; measure the pinned ffmpeg download and the cycle check cost (QG-06).
- QG-15: pointer tests (colour picker, keyframe timeline), Export dialog dropdown clicks, golden argv for formats 4 and 5, large-file memory test, interface harness when it grows.
- QG-16 remainder: split text.rs by area; one hit-box rule shared by preview and the measured box (ui/boxes.rs, placement.rs); cycle check re-run; docs/code-architecture.md update.
- QG-17: splash picture size per program; clearer message for a missing splash summary; notices run after each version bump; player notice version line; test-memory cap as a developer-docs rule.
- QG-18: UI frame gates sampled as preview tests (best of up to five, limits unchanged); hardware probe success fakes with long limits under load; font licence test in the notices; measured accuracy numbers kept with the subtitle accuracy document.
- QG-11..13 are measured, not gated (recorded in the developer docs at release).
- Gates and estimates for these items are removed from `docs/architecture-2.0.4-quality.md` and `ai-docs/architecture-nfr.md` until built.

### Open items for other agents (not decided by the architect)
- PE: muda spike result (ADR-183); speed-first preset chosen (ADR-186); AccessKit size measured (ADR-197, Q-4); keyframe widget file (ADR-176).
- PM: REQ-222 wording (quarter vs eighth; PM options already in the PM section above; the architect records the built value and the measurement); REQ-279 wording to "Normalise speech" (plan Q-6 fallback).
- Andrea: nothing architectural is waiting for a decision beyond the PM items.

### Architect: what remains
1. When a not-built item is built or cut: set the status of ADR-185, 188, 189, 192 and of the parts of ADR-186, 190, 201; move its design back into the docs; update the components map, NFR gates, security list and crate shortlist.
2. Architecture docs for what is built later: at 2.0.4 release, the per-area sub-documents get the 2.0.4 facts (frame: blur divisor; projects: format 11; subtitles: VTT, ASS, separate track; player; hardware export: AV1, VP9, decode, check; packaging: installer workflow and NSIS template; performance: measured blur, check timings; security: unsafe sites). Also remove "planned" wording of 2.0.2 and 2.0.3 items.
3. Cycle check: re-run `tests/module_cycles.rs` (7 tests green per the PE) after QG-16 (text split) and after any new module (`pictures`, `text/events`, `ui/keyframe_timeline` if created); update the cycle list in `ai-docs/architecture-components.md`. Known: media -> camera is avoided by `camera::start_camera` (in camera.rs; the media.rs test calls it, test code only).
4. Verify at release (step 2.4 of /release) against `docs/code-architecture.md`: new modules autosave, instance, hwdec, siblings, measure, placement; macOS code inline in `desktop.rs` (no `desktop/macos.rs` file); no `ui/keyframe_timeline.rs`; render/ and app/ split; unsafe sites vs the ADR-034 amendment.
5. Crate shortlist at release: objc2-app-kit 0.3.2 (macOS only, new direct dependency, minimal features); accesskit feature of eframe (accesskit_winit 0.34.2); image stays `gif` only unless REQ-287 is built; no muda; `std::fs::File::try_lock` (no crate).
6. At release: archive the three plan files (the command moves them), drop their index lines, regenerate `ai-docs/architecture-doc-index.md`.
