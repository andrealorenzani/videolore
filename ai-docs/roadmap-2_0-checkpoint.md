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

STATUS done, all 2.0.4 REQs implemented except REQ-177 (2026-11-21, version 2.0.4). Second STATUS call: REQ-253, 257-260, 268, 273, 281, 286-288 marked `(implemented)`, each with a `// covers:` test (grep-checked). No test was run by the product manager.

### State after STATUS (second call)
- All REQ-180..299 (120) are `(implemented)`. REQ-177 is outside that range and stays `(planned)`. Each implemented REQ has a `// covers:` test (grep-checked for the 11 of the second call).
- Amended in the REQ texts to the built behaviour (second call): 253, 257, 258, 259, 260, 268, 273, 281, 287. Deviations taken from the PE section; REQ-273 pitch is applied (the code), the PE line "pitch not applied" is not used. REQ-286 and 288 unchanged.
- REQ-177 wording: the acceptance no longer names espeak; it names the pinned ffmpeg and the committed speech fixture (`tests/fixtures/align/speech.mkv`). Status still planned (open decision).
- Future work (from the PE list, in the 2.0.5 batch of `docs/roadmap_tasks.md`): async spatial pre-pass; earlier display of picture subtitles; picture subtitles on 360 clips and paused stills; roll in the ambisonic rotation; CI time measurement (QG-06); REQ-177 decision; REQ-206 skeleton render; blurred fill in playback; macOS Quit item. Manual checks: verification items 26, 29, 30, 31-35.
- The 2.0.5 batch keeps only these items. Flag for Andrea: the QG-16/17 items that the PE did not report (one keyframe module for the 360 camera and the picture; smaller project page images; player package list) are kept there as "not built, no batch yet".

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
- First call: 109 REQs marked `(implemented)`. Second call: the 11 above. Now all REQ-180..299 are implemented; REQ-177 stays `(planned)`.
- Deviations accepted as built (user had not answered), amended in the REQ text: 202, 195, 222, 244, 248, 279, 293; 183 and 182 amended with the built behaviour; 206 spec confirmed by the batch note (one-second ranges), no change. Open for Andrea: see ai-docs/business.md § Checkpoint 2.0.4 and business-decisions.md § Open questions.
- REQ-280 was listed as not built in the checkpoint, but `covers: REQ-280` tests exist (render/tests.rs, tests/ffmpeg_integration/v204.rs); marked implemented per the user's list.
- Future work: the list in "State after STATUS" above; REQ-177 (decision).

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



Update (resume call): REQ-286 and 288 built (`src/timeline.rs:clip_aspects`, `src/render/graph.rs` own frame + pad, `src/render/segments.rs:SegFrame.own`, drop handling in `src/ui/timeline.rs`; tests in `src/render/tests.rs`, `tests/ffmpeg_integration/v15.rs`). The old REQ-075/130 "cannot be dropped on a clip" tests were replaced by REQ-286/288 ones. Left, in order: 281, 287, 257..260, 268, 273, QG-14..18, 253.

Suite after REQ-286/288 (resume call end): fmt ok; clippy clean (default and no-default-features); 805 lib (2 ignored), 89 integration (3 ignored), 7 module_cycles, 2 page, 44 + 3 player; line coverage 95.0 %; feature coverage 287/287; notices and docs-check ok. REQ-281 (speed) was judged too large to do safely: every source to timeline mapping (`Clip::len`, locate, split, trim, subtitle times, keys, camera) assumes the two lengths are equal. QG-17 memory rule is already in `docs/code-testing.md`.

Context-limit stop (this call): nothing new was built; the tree is unchanged from the previous update and green (805 lib, 89 integration, coverage 95.0 %, feature 287/287). Plan for REQ-281 so the next pass starts at once: (1) store `speed_permille` (500..2000, default 1000, skip when 1000) in a `Timeline` map like `clip_aspects` (Clip is Copy); (2) split `Clip::len()` in two: `src_len()` = `src_out - src_in` and `len()` = timeline length = `src_len * 1000 / speed`: this needs the speed on `Clip`, so either give `Clip` a `u16` field (Copy) or pass the speed; a `u16` field `speed_permille` on Clip is the simplest and keeps Copy; (3) audit the 87 sites that use `c.len()`, `src_out - src_in` or `src_in + offset` (grep `-E '\b(c|clip|left|right)\.len\(\)|src_in \+ '` outside tests): timeline positions use `len()`, source reads use `src_len()` and `offset * speed / 1000`; key places: `Timeline::duration/start_of/locate/split/trim`, `render/segments.rs` (`s.src_in + s.skip` becomes source time = `skip * speed`; segment `len` is timeline length), `subtitles::project`, texts, camera `at(src_in + offset)`, `keyframes` clip-time keys, preview position mapping; (4) graph: video `setpts=(PTS-STARTPTS)*1000/speed` after trim, audio `atempo` (one filter covers 0.5 to 2.0) and `atrim` on source time; fades use timeline length; (5) tests: 4 s at 200 % lasts 2 s, pitch within 1 %, subtitle within 0.1 s of its word, stream copy refused. Then continue: 287, 257..260, 268, 273, QG-14..18, 253 as listed above.


Update (third call): REQ-281 built: `Clip.speed` (permille 500..2000, `Clip::len()` is now TIMELINE length, `src_len()`, `to_timeline/to_src/src_at`), keys stay in source time and `Clip::keys_to_timeline` maps them in `frame_stack`; `Segment.speed` with `src_from/src_to/rendered_src/lead_src`; graph `rate_pts` + `atempo`; `camera::command_lines_speed`; `Command::SetSpeed`, Clip details Speed field, `DIFFERENT_SPEEDS` merge block, stream copy refused (`any_speed`). Volume keys stay in timeline clip time. Tests: `timeline.rs` (2), `render/tests.rs` (2), `app/frame_tests.rs`, `tests/ffmpeg_integration/v204b.rs` (pitch by zero crossings). REQ-287 built as a TextItem with `picture: Option<Picture>` (`src/picture.rs`: PNG header size, `rect` fit in box; text = file name; no ASS events): `render::picture_overlays` + `PicOverlay` + `build_graph_pics/with` (looped `-loop 1` Input with `still`, overlay with `enable=between(t,..)` before subtitles), `Command::AddPicture(Option<PathBuf>)`, `Dialogs::pick_picture`, Effects row and `Action::AddPicture`, picture popup (time + Delete). Not on a 360 export (sphere mode skips pictures); no text effects on pictures. Tests: `picture.rs`, `app/text_tests.rs` (2), `render/tests.rs`, `v204b.rs`. Left: 257..260, 268, 273, QG-14..18, 253.

Update (third call, later): REQ-257 (`TextItem.karaoke` non-saved cs list, `ass::highlight_chunks`, `segments::anchored_line_items` uses preset readability and chunks), REQ-258 (`AppState::generation_anchors` in `app/subs_create.rs`, applied in `finish_create`), REQ-259 (`Command::ReAnchor`, `texts.rs:re_anchor_plan/re_anchor`, Action `ReAnchor` in Edit > Subtitles, picture right-click and Context Menu group when a line is selected and the playhead is on a 360 clip; the old group test now expects ReAnchor before SetPov), REQ-260 (`ui/placement.rs:off_view_arrow(s)`, `ui/picture.rs:off_view_arrows`, click sends `TurnCamera` with the delta) built and tested (`app/text_tests.rs`, `render/tests.rs`, `ui/placement.rs`). Left: 268, 273, QG-14..18, 253, full suite, docs.

Update (third call, end): REQ-268 built (COW per-clip paths, see docs Stage 13). Full suite green at this point: fmt, clippy workspace, 820 lib, 91 integration, 7 cycles, 2 page, 44+3 player; notices note for the test font added to both notice files; coverage run started (see report). NOT built: REQ-273, REQ-253, QG-14..17 (QG-18 test font only). Reasons: 273 needs a per-segment audio input and a pre-pass runner in export and preview; 253 needs the player crate work and a bitmap-subtitle fixture.

Update (fourth call): built QG-14 (speech tests use the committed `tests/fixtures/align/speech.mkv`, espeak-ng removed from CI packages, sphere fixture 1024x512 in v126; measured: cycle check 1.7 s, pinned ffmpeg 173 MB unpacked), QG-15 (colour picker and Export dropdown pointer tests in `ui/text_ux_tests.rs`, keyframe bar empty click in `ui/clarity_ux_tests.rs`, golden argv for formats 4 and 5 `tests/fixtures/export-argv-v4-v5-in-2.0.4.txt` with `project-v4/v5` fixtures, large picture memory test), QG-16 (`text.rs` split into `text/{timing,layout,events}.rs`; one box rule `placement::padded_line_rect` shared with `ui/placement.rs`), QG-17 (`build.rs` summary check, `version.sh set` runs the notices check as a reminder; player notice has no version line; splash GIFs 88 KB each left in the library), QG-18 (sampled frame gates in `ui/tests.rs`, longer limits in `app/hw_tests.rs`, test-font notice test). REQ-273: `src/spatial.rs` (rotate, cardioid pair, WAV writer, `prepare` pre-pass through the Runner), `Segment.spatial`, `Graph.spatial`, `Plan::spatial()`, `export::prepare_spatial`, `Preview::prepare_spatial` (synchronous, wall clock shifted by `prepass`; preview skips clips over 10 min); measured in the debug test profile: 120 s of 4-channel sound prepared in 1.7 s (69x real time). Roll and the pitch of the camera do not change the stereo pair (documented). REQ-253: `MediaFormat.picture_tracks`, `Project.picture_sub` (runtime), `Segment.picture_sub`, overlay `[k:v:0][k:s:N]overlay` before the trim in `render/graph.rs`, player `Source::Picture`; fixture is a hand-written PGS stream built in `player/tests/ffmpeg.rs:pgs`. Limits: a picture subtitle already on screen when a run starts inside it appears at its next change (also for paused stills); not on 360 clips. Final suite of this call (pinned ffmpeg 9.0.2, VIDEOLORE_REQUIRE_FFMPEG=1): fmt, clippy workspace all targets and no-default-features, 841 lib (3 ignored), 96 integration (3 ignored), 7 cycles, 2 page, 44 + 4 player, line coverage 94.9 %, feature 287/287, notices and docs-check ok. Manual-check items 31 to 35 added to docs/business-verification.md. Nothing of the 2.0.4 plan is left to build; REQs are still `(planned)` for the PM/documenter to mark.

## Architect

Final state of 2026-11-21 (2.0.4 built; supersedes the first checkpoint of the same day). Sources: `ai-docs/architecture-plan-2.0.4.md` (main), `-stages-1.md` (stages 0-5), `-stages-2.md` (stages 6-12), `ai-docs/architecture-adr.md` § 2.0.4, `docs/code-2.0.4-batches-12-15.md` (PE stages 8 to 14), and the tree. Nothing was run by the architect; nothing committed.

Done by the architect for 2.0.4: ADR statuses (below), the owned human docs in `docs/architecture*.md` describe the built state (overview, quality, 360, text, projects, effects, LUT, player, subtitles, media pipeline, build, verification, security, dependencies, components, structure, splash, performance, frame, boxes, UI, packaging, hardware export, export dialog, decisions), the owned AI notes are updated (index, ADR block, components, NFR, security, risks). `ai-docs/architecture-doc-index.md` regenerated at the end of this pass.

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
| 10 360 video | stages 0, 1 (format 11), 3 (keyframe widget) | built (REQ-261..267, 269..272); 257..260 and 268 in stage 13; 273 in stage 14 |
| 11 effects and frame | stage 6 (blur cache keys), stage 10 (camera and keyframe model), stage 0 | built (REQ-274..285, 289..294); 280 volume keyframes built here; 286 and 288 in stage 13 |
| 12 continuous quality | stage 11 (settings complete), stage 2 (cache names), stage 1 (font notices) | built (REQ-295..299) |
| 13 speed, pictures, 360 text and paths | stages 10, 11 | built: REQ-281, 287, 257..260, 268 (see the stage list in `docs/code-2.0.4-batches-12-15.md`) |
| 14 spatial sound, picture subtitles, quality items | stages 10, 13 | built: REQ-273, REQ-253, QG-14..18 |

### ADR status and updates recorded on 2026-11-21
- ADR-168..201: all built. Final statuses (ai-docs/architecture-adr.md § 2.0.4, final status line): ADR-185 (REQ-253) BUILT, overlay in the shared graph, golden argv kept; ADR-188 (REQ-257..260) BUILT, anchored lines as text items with preset box and karaoke highlight (amends 1.3.0 plain white); ADR-189 (REQ-273) BUILT, pre-pass in export and synchronous in preview; ADR-192 (REQ-287) BUILT as a text item with a picture, header read only, the plan's count and size limits NOT in the code (open); ADR-186 BUILT (per-clip paths copy on write, REQ-268); ADR-190 BUILT (speed, volume keyframes REQ-280, clip frame REQ-286); ADR-201 BUILT (QG-14..18).
- Updates recorded (ai-docs/architecture-adr.md § 2.0.4, and docs/architecture-decisions.md):
  - ADR-034 amended: one more allow-listed unsafe call, `initWithTitle:action:keyEquivalent:` on objc2-app-kit `NSMenuItem`, in the macOS part of `src/desktop.rs` (menu of ADR-183). Unsafe sites are now stt.rs (whisper abort callback) and desktop.rs (macOS odoc handler and menu).
  - ADR-180: blur divisor 8 (an eighth of each side; REQ-222 text says a quarter). Measured with ffmpeg 9.0.2: 4K source (2160x3840 blur), 8 s, 3 runs: divisor 8 = 2.9-3.1 s, divisor 4 = 3.3-3.7 s; 1080p (1080x1920): 1.4 s vs 1.5-1.6 s. Background cache = raw RGBA file `bg-<hash>.rgba` in the private still folder, one at a time, stills only (ffmpeg needs a file input); not an in-memory image as planned.
  - ADR-181: installer job is the reusable workflow `.github/workflows/installer-config.yml`, called by build.yml (`needs`) and run alone on pull requests with the path filter; advisories.yml built; named test `cpu_message`.
  - ADR-183: objc2-app-kit 0.3.2 (macOS only, minimal features) builds the menu in `desktop.rs` (Hide, Hide Others, Show All, Minimize, Zoom; no Quit, Cmd+Q stays with egui's prompt). Built and clippy-checked for aarch64-apple-darwin only, not run. The muda spike result is not in the notes (open).
  - ADR-184: Windows player installer uses the custom NSIS template `packaging/windows-player.nsi` ("Open with" entries only, `OpenWithProgids`; cargo-packager would register the default handler). Not run on Windows.
  - ADR-187: MKV and WebM 360 marker written in place (`src/spherical.rs`): the Void element ffmpeg leaves as padding between SeekHead and Info is shrunk by the bytes that the Tracks element grows; nothing after Tracks moves. Matroska pose IDs 0x7673..0x7675 (0x7672 is ProjectionPrivate). No Q-7 cut.
  - ADR-191: LUT strength is a blended table (`lut.rs` `with_strength(percent)`: every entry mixed with the identity by the percent; 0 = no change, 100 = the table). Project read cap 96 MiB (`MAX_PROJECT_BYTES` 16 MiB to 96 MiB, `project.rs`). LUT copies raw, 20 MB each, 64 MiB per project.
  - ADR-194: Enhance speech built as `speechnorm` named "Normalise speech" (`render.rs` `NORMALISE_SPEECH` = `speechnorm=e=6.25:r=0.0001:l=1`); no model file, no arnndn; the arnndn licence check is not needed unless a model is added later.
  - ADR-186: built, per-clip paths included. The speed-first preset chosen by the rule is not recorded by name (open, PE).
  - ADR-190: built in full: fade length and sides, crop, Clean audio strength mix, speed, volume keyframes, clip frame.
  - ADR-176: no file `ui/keyframe_timeline.rs` exists (checked by the architect); the keyframe widget is in the existing UI modules.
  - ADR-197: eframe feature `accesskit` on; accesskit_winit 0.34.2 in Cargo.lock. The size measurement asked by Q-4 is not in the notes (open).
  - ADR-185 (picture subtitles), 188 (anchored lines), 189 (spatial sound), 192 (picture items), 201 (QG-14..18): built, see the list below.
- Human docs: docs/architecture-2.0.4-overview.md and -quality.md describe the built state. docs/architecture-2.0.4-quality.md network line: the only network use stays the consented speech model download (whisper); Enhance speech adds none.

### Built designs: where they are recorded (replaces the former "not built" list)
- REQ-253 picture subtitles: `docs/architecture-player.md` § 2.0.4 additions; the decision in `docs/architecture-decisions.md`. Limits: none on 360 clips; a subtitle on screen at a run start appears at its next change.
- REQ-257..260 anchored 360 lines, automatic anchors, re-anchor, off-view arrows: `docs/architecture-360.md` § Anchored lines and camera paths (2.0.4); text drawing in `docs/architecture-text.md`.
- REQ-268 per-clip camera paths (copy on write, no copy at load): `docs/architecture-360.md` § Per-clip camera paths (2.0.4); `docs/architecture-projects.md` format 11, stage 10 and 13.
- REQ-273 spatial sound: `docs/architecture-media-pipeline.md` § Spatial sound pre-pass (2.0.4); the assumptions in the risks above.
- REQ-280 volume keyframes, REQ-281 speed, REQ-286 clip Change aspect, REQ-288 drop on a clip: `docs/architecture-effects.md` § Effects added in 2.0.4; time mapping in `docs/architecture-media-pipeline.md` § Preview and export additions (2.0.4).
- REQ-287 picture items: `docs/architecture-text.md` § Picture items (2.0.4); the limits decision is open (see the list below).
- QG-14..18: `docs/architecture-verification.md` § 2.0.4; gates in `docs/architecture-2.0.4-quality.md`.
- QG-11..13 measured, not gated (recorded in the developer docs at release).

### Open items for other agents and Andrea
- PE: speed-first preset name (ADR-186); muda spike result (ADR-183); AccessKit size (ADR-197, Q-4); the Arabic and Devanagari OFL notices to confirm; picture item limits (ADR-192): either add the plan's caps (20 items, 8 MiB, 16 Mpx) or record that the header read is enough (Andrea decides).
- PM: REQ-222 wording (quarter vs eighth; the architect records the built value and the measurement); REQ-279 wording "Normalise speech"; REQ-287 wording if the limits change; mark REQ-180..299 `(implemented)` as built.
- Andrea: the picture item limits decision above.

### Architect: what remains for the release
1. Verify at release (step 2.4 of /release) against `docs/code-architecture.md`: new modules autosave, instance, hwdec, siblings, measure, placement, spatial, picture, `text/`; macOS code inline in `desktop.rs` (no `desktop/macos.rs`); no `ui/keyframe_timeline.rs`; render/ and app/ split; unsafe sites (stt.rs, desktop.rs) vs the ADR-034 amendment; `build.rs`.
2. Cycle check: `tests/module_cycles.rs` (7 tests, green per the PE) after the text split; the PE re-runs it at release; update the cycle list in `ai-docs/architecture-components.md` if it changed.
3. Crate shortlist at release: objc2-app-kit 0.3.2 (macOS only, new direct dependency, minimal features); accesskit feature of eframe (accesskit_winit 0.34.2); image stays `gif` only (picture items read the PNG header; no `png` feature); no muda; `std::fs::File::try_lock` (no crate).
4. At release: archive the three plan files (the command moves them), drop their index lines in `ai-docs/architecture.md`, regenerate `ai-docs/architecture-doc-index.md`; the shared checkpoint file is deleted with the release.
5. Documents still to clean at release (compaction, planned wording of 2.0.2 and 2.0.3 items that now read as built, the long 2.0.4 overview tables).

## Principal Engineer 2.1.0
(checkpoint 1, LUT stages) Spike S0a, measured 2026-11-21 with the pinned ffmpeg 9.0.2 on the dev box (user LUTs read only, never copied): the three maintainer LUTs are 144^3 cubes of 80,621,697 bytes (2,985,984 entries of 27 bytes, 6 header lines). ffmpeg lut3d reads a 144 cube straight: 2.3 s, 80 MB peak; a generated 256 cube (453 MB, `scripts` scratch gen.py): ffmpeg start 12.4 s, 233 MB peak (preview start would be slow: risk, see future work). Cause of the refusal confirmed by code and by the old unit tests: `lut::MAX_3D = 65` ("size 144 is outside 2..=65") and `MAX_BYTES = 16 MiB` ("larger than 16 MiB"); ffmpeg is not the cause. Built: src/lut/{mod,parse,store}.rs (edge 2..=256, 512 MiB, line 256 bytes, streamed parser, u16 table 6 B/entry, cache by bytes 192 MB, Load thread with progress and cancel on drop, Work copy `lut-<hash16>.cube` by `write_copy` temp+rename, shared by clips), Graph/PreviewRender/StillRequest `lut_files` + `write_command_files(dir, files, luts)`, app background read for files > 1 MiB (`PendingLut`, `tick_lut_load`), path-only above 20 MB by `Lut::cube_len` (REQ-301). Traps: `.3dl`/`.csp` are read whole (cap 64 MiB); tests using exact 0.5 now use a 1/65535 tolerance (ADR-203 u16); old render test `each_clip_has_one_lut_stage...` replaced (it asserted `lut-clip-N.cube`); the Load thread is a plain thread (not the jobs scheduler: not an ffmpeg process). Debt: MissingLutFind reads synchronously; a project closed while a LUT loads still applies the LUT to a missing target (error in status); m3d not tried.
(checkpoint 2, end of this call) Also built: REQ-306..310 (src/base/shadow.rs, StyleSettings in subtitles.rs, ass.rs `style`/`text_styles`/`document_styled`, src/word_styles.rs, Command::SetSubtitleStyle, popups rows) and format 12 fields. Spikes S0c (`\ko` + `\2a&HFF&`) and S0d (box fill from OutlineColour: design A) measured with ffmpeg 9.0.2; S0b not needed (Enlarge = two events, the line is centred so the word keeps its centre). LEFT: S5 (effect_order.rs, Timeline.clip_order, ui/applied_effects.rs, REQ-302/303/304 undo labels) and S6 (render/overlay.rs per-clip ASS + LUT after fades, REQ-304/305, legacy argv unchanged), plus the order field and the repair note of REQ-311, plus QG-20/21 gates. Do not add the widget's Move up/down before the overlay stage exists. Traps: see ai-docs/code.md State 2.1.0. Docs: docs/code-2.1.0.md.
(checkpoint 3, second call) Built S5 + S6 + the rest of REQ-311: `src/effect_order.rs` (Row, Present, legacy/resolve/to_store/is_changed/moved/dragged; rows serialise as `lut`, `subtitles`, `text:<id>`, `picture:<id>`, unknown strings = `Row::Unknown`), `Timeline.clip_order: Vec<ClipOrder>` (lenient reader: a damaged entry becomes clip 0 + Unknown and is dropped with the note "A damaged effect order was repaired"; remove/merge drop, split copies), `src/project/order.rs` (`clip_rows`, `rows_of`, `order_of`, `changed_orders`, `set_order`, `put_on_top(row, only)`, `repair_orders` called at the end of load), `src/render/overlay.rs` (`OverlayStep::{Lut,Ass,Pic}`, `attach(project, segs, g, from, src, burn_lines, steps)`, `legacy_windows`, `changed_ids`), `Segment.clip/overlay`, `PicOverlay.pieces`, `graph.rs overlay_chain` (steps appended after the fades, per-clip pictures are extra still inputs after the global pictures), `plan.rs subtitles_doc_ordered` + `picture_overlays` leave out the changed clips' windows (`text::project_events_in`, `item_events`, `intersect`), `FrameKey.order`, `app/applied.rs` (`AppState::applied()`, `Command::MoveRow{clip,row,mv}`, `RowMove{Up,Down,To}`, labels "Move LUT up|down", "Move text ...", "Move picture ...", "Move subtitles ...", "Move effect" for a drag), a new LUT/picture goes on top (`put_on_top` after the edit, not a separate undo step), `ui/applied_effects.rs` (15% share, min 58 px, rows announced "Applied effect: <name>", Move up/down buttons "Move up: <name>", drag, Alt+Up/Down on the focused or hovered row). Tests: `effect_order` unit, `project/order_tests.rs`, `render/overlay_tests.rs` (9), `app/applied_tests.rs` (9), `ui/applied_ux_tests.rs` (6), `tests/ffmpeg_integration/v21.rs` (3 new: text/LUT order, overlapping texts, picture per clip; export and preview agree). Replaced/adapted: `frame_ux_tests::a_corner_handle...` window 1280x800 -> 1280x1000 (the shrunk picture put the Scale popup over its corner). Traps: ScrollArea has `min_scrolled_height` 64 (set 0 in the widget); the harness `find` picks the status line label when it says the same words as a row (the widget rows are announced with the prefix "Applied effect:"); AddText over a 360 clip anchors the text to the sphere; Sphere360 export ignores the order (one document, no overlay). Assumption: "whole video" mark = every clip of a timeline of two or more has the same effect (copies, REQ-147), plus the edge fades (first clip Fade IN, last clip Fade OUT) and the project's Change aspect. LEFT: docs (docs/code-2.1.0.md, ai-docs/code.md), full suite with coverage, QG-20/21 are covered by tests (v21.rs parity, overlay_tests legacy equality, the old goldens pass unchanged).
