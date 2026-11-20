# ai-notes: architecture plan 1.6.1 (PLAN 2026-11-19)

Owner: architect. Target 1.6.1 (version already 1.6.1 in Cargo.toml files). Business: ai-docs/business.md § 1.6.1 PLAN and REFINE; REQ-034, 082, 083, 085, 097, 116, 129, 130, 132, 135, 137, 144..149; QG-06. Human docs updated: docs/architecture-build.md, -media-pipeline.md, -frame.md, -effects.md, -projects.md, -splash.md, -boxes.md, -security.md.

## Code facts (read 2026-11-19)
- Preview: src/preview.rs `Preview` (still: `shown_key`, `posted_key`, `wanted`, `generation`, `mailbox` latest-only, `still_thread`). app: src/app.rs `refresh_still`, `seek`, `tick` (poll), PlayPause; app/frame.rs, effects.rs call refresh_still. render::frame_key (src/render.rs ~1613) = content identity (path, frame index, fade, size, subtitles, placement, luts).
- Timeline (src/timeline.rs): Clip {id, media, src_in, src_out, volume, muted, clean_audio}; Timeline {clips, fades: BTreeSet<(ClipId,ClipId)>, start_fade, end_fade (video-level), frame_aspect, whole_scale, whole_move, clip_frames, whole_lut, clip_luts}; fades_of(i) uses start_fade/end_fade for first/last clip; SCHEMA_VERSION 8 (src/project.rs:27), OLDEST_SCHEMA, from_json migrations (version < 6 legacy text sizes), TooNew for > SCHEMA_VERSION.
- Frame handles: src/ui/frame_handles.rs `Hit{Corner,Target}`, `hit()`, `scale_to(g, corner, p)` (opposite corner fixed, max of both axes), `move_to`, `scaled_rect`. Limits MIN_SCALE 100 (10%), MAX_SCALE 4000 (400%) permille. Subtitle/text boxes: src/ui/boxes.rs `Corner`; text width handle REQ-116.
- Splash: src/splash.rs include_bytes!/include_str! of assets/splash/<CARGO_PKG_VERSION>.gif/.txt -> build fails without 1.6.1.gif and 1.6.1.txt. 1.6.0.txt = heading "New in this version" + 4 lines.
- CI: .github/workflows/build.yml: check job (apt step, 3 clippy, llvm-cov tests, cargo-deny via taiki-e, `cargo install --locked cargo-about --version 0.9.3` in notices step), msrv job (apt), build job (needs check+msrv, apt on Linux), package job (tags/dispatch; cargo install cargo-packager on macOS/Windows).
- Tests: tests/ffmpeg_integration.rs uses `VIDEOLORE_REQUIRE_FFMPEG`; espeak-ng used at run time by real_speech_is_transcribed (line ~908) and an accuracy test (~1036); tests/fixtures/align/speech.mkv (67 KB) + reference.json + track.srt committed.
- Features default = ["audio", "stt"]; audio needs libasound2-dev (pkg-config), stt needs cmake + libclang (whisper-rs-sys bindgen).

## 1. Black preview when paused (REQ-144) — cause and fix (ADR-120)
Cause (code, to be confirmed by a failing test first):
- `request_still` skips the render when `shown_key == key` or `posted_key == key`. Both marks are set when a request is posted, but the marks are not kept in step with the newest request:
  (a) early return (key equal to shown or posted) does NOT bump `generation` and does not cancel an older in-flight render; then an older result is accepted (no bump) or dropped (bump by a newer request) while posted_key still names a key whose result never arrives.
  (b) `still_thread` skips a superseded request (`req.generation != latest`) without sending a result; `posted_key` is cleared only when a result of the current generation is accepted, so posted_key keeps a dead key.
  (c) `play()` clears `shown_key` and `wanted` but not `posted_key`; a result that arrives while playing is dropped (`!is_playing()`), so posted_key stays.
- Consequence: after the dead key is set, `request_still` for that same key returns early forever (until play, forget_frame or a key change), so the screen keeps the last frame it got: the playback frame or an older still, black when that frame was a fade-through-black (start, end, boundary) frame.
- Repro sequence for the PE test (fake runner, Preview::new(config(..)), REQ-144): shown K0 (fade black frame at 0 s) -> request K1 (2 s), poll at due (posted K1) -> request K2 (3 s, wanted) -> request K0 (shown: early return, wanted cleared, no bump) -> request K1 again: expected render, actual early return. Assert a still for K1 is produced and poll returns its frame.
- Second sequence: play from 1 s, request_still of the key that was posted before play, pause: expected a still, actual none.
Fix (ADR-120): one rule: every request_still that changes the target bumps `generation` and clears `wanted` and `posted_key` (superseded results are dropped by generation); a request equal to `posted_key` keeps the in-flight render; a request equal to `shown_key` bumps and cancels any in-flight render (its result would be stale) and shows nothing new; `play()` and `seek()` bump, clear `shown_key`, `posted_key`, `wanted`; `still_thread` on a superseded request sends nothing (unchanged) and posted_key is no longer relied on. REQ-034 kept: an edit that does not change the key renders nothing; 10 edits in a burst -> at most one render (existing test).
Fallback if the test shows another cause: render on every pause (one render per pause; cost ~ one still render).
Not changed: frame_key content (it is correct: key equality = same picture).

## 2. CI time (QG-06, ADR-126)
Measured (GitHub public API, runs 2026-11-14..19): push of 1.6.0 (3b09f97) 28 min to last executable; check job 25.2 min, of which "System packages" step 20.0 min (others: 0.2..1.5 min; aa6f993 5.4 min). Other check steps ~5.2 min: clippy x3 0.3, llvm-cov tests 2.3, notices 1.05 (cargo-about built from source), rest <0.5 each. MSRV job 0.9..3.6 min; executables 0.8..2.7 min.
Runner image 20261111.370.1 (Ubuntu 24.04): cmake 3.31.6 and pkg-config listed; clang 18.1.3 listed; ffmpeg, libasound2-dev, espeak-ng not listed. Local `apt-get -s` (not a measure of the runner): --no-install-recommends pulls 6 packages for the set, 10 with recommends.
Plan (steps for PE in build.yml):
1. Install only missing packages: `dpkg -s` check for libasound2-dev, libclang-dev, ffmpeg, espeak-ng; `apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=30 update` only when something is missing; `install --no-install-recommends`. cmake and pkg-config not installed again (listed on image; the check fails loudly if missing).
2. `timeout-minutes: 30` on check (bounds a stall), MSRV 30, build 30.
3. cargo-about: taiki-e/install-action with `cargo-about@0.9.3` (pinned action SHA already in file) instead of cargo install (verify the action supports cargo-about; else keep cargo install and say so).
4. Optional (PE decides, test-only, no assertion weakened): real_speech_is_transcribed and the accuracy test read tests/fixtures/align/speech.mkv instead of espeak-ng; if done, drop espeak-ng from apt.
5. Not done (rejected): third-party apt cache action (supply chain, unmeasured gain); starting executables in parallel with check (builds untested binaries); removing clippy feature sets or llvm-cov; pinned ffmpeg for tests (candidate for 1.6.2: tests would use the shipped build; needs test lookup change).
Estimate: check job ~4.4-4.7 min (-80% vs 25.2 run; -20..25% vs typical ~5.9 min); push to last executable ~7 min vs 28. Target 30% is ASSUMPTION (QG-06, not a gate). PE reports measured time of the first run against both baselines (outlier and typical).

## 3. Visible edges (REQ-145, REQ-146; ADR-121)
- Pure geometry (ui/frame_handles.rs and ui/boxes.rs): `visible_part(rect, frame) -> Option<Rect>`; edge segments = box edges clipped to frame rect; hit order: corners of visible picture, visible edges, target (Scale) / visible width handle (text). `Hit::Edge(Side)`.
- Drag: `scale_to_edge(g, side, p)`: f = pointer distance from the opposite edge / side length at start, along the normal; same permille clamp 100..4000; centre offset keeps opposite edge fixed (generalises scale_to; scale_to becomes the corner case).
- Subtitle lines: same proportional rule (REQ-097 25..400%); text items: only right-edge width handle, on its visible part; no left handle.
- Tests: pure (hit order, segment clip, ratio kept, opposite edge fixed, limits, clip to picture), ux (16:9 in 9:16 Fit height, drag visible side, 300% scale back in one drag, one undo), subtitle centre on right edge enlarged via top edge, text right-edge handle at picture edge changes width only.
- Alternative rejected: free aspect edge drag (breaks REQ-127 ratio); separate widgets per edge.

## 4. Per-clip whole-video effects, format 9 (REQ-147, 082, 083, 148, 149; ADR-122..125)
ADR-122 (whole-video = per-clip state): Timeline methods per effect loop over clips and write clip state; no whole-video state in format 9 except frame_aspect (Change aspect, REQ-124). Rejected: keep whole range (two rules forever), virtual layer at render time (no per-clip deletion semantics).
ADR-123 (edge fades on the clip): `Clip.fade_in: bool`, `Clip.fade_out: bool` (serde default false, skip if false); Timeline `start_fade`/`end_fade` removed (migrated). fades_of(i): fin = boundary(i-1) || clip.fade_in; fout = boundary(i) || clip.fade_out. Deleting first clip removes its fade (REQ-083). Rejected: keep video-level flags with first/last rule (fades would not move with clip, violating REQ-083).
ADR-124 (format 9 + in-memory conversion from 8): SCHEMA_VERSION 9; OLDEST_SCHEMA unchanged; migration in a new file src/project/v9.rs (project.rs gets `mod v9;`) run in from_json for version 8 only, pure, unit-tested, no undo step, no change mark (same as legacy text sizes). Conversion per clip: 
  - whole_scale / whole_move (timeline time) -> clip copy: keys shifted by clip start, keys inside the clip kept, plus a key at 0 and at clip length with the path's value there (sampled with Key::at), so the picture at every clip time is unchanged; keys outside dropped; cap 200; if the clip already has its own key of this kind -> one combined path (Decided Q-F).
  - whole_lut -> clip_luts for every clip (same path); clip with own LUT keeps its own, whole-video LUT not applied to it (Decided Q-F).
  - start_fade -> clips[0].fade_in = true; end_fade -> clips[last].fade_out = true.
  - Volume: in 1.6.0 the whole-video level (REQ-084/020, `video level`) multiplies every clip. Conversion: clip level = round(video level x clip level) to 1%, video level set to 100% (same sound; assumption: 1% rounding is inaudible). 1.6.1 has no project-wide level.
  - Hostile values: keys sanitised (keyframes::sanitise), counts capped, invalid -> refused as damaged (REQ-057).
  - 1.6.0 refuses schema 9 by TooNew (REQ-029) automatically.
ADR-125 (copies at application time): a double-click creates one Scale/Move per clip with one key at 0 (100% / centre); whole-video popup edits all copies at the same clip time (add where long enough; value set adds where missing; remove keeps each copy's last key); popup view = first clip's keys on a timeline as long as the longest clip; one undo for all. Clip popup and picture handles change one copy. Deleting a clip deletes its copy; clips added later get none.
- Stacking ends (REQ-130): Stack composes one clip track only; frame_stack(i) loses the whole-video track; the export/preview graph has the same shape as one clip track.
- Other whole-video effects: Subtitles (per clip, REQ-085 rules), Clean audio (per clip), Volume (sets each clip level: the double-click sets the level of each clip), Fade (clip flags), LUT (same path per clip, REQ-135 one per clip; a clip that has one keeps it (conversion: Decided Q-F), double-click on a clip with a LUT replaces it as the 1.6.0 Apply LUT rule says).
- Model edits go through one Edit snapshot (project.edit / set_edit) so undo is one step (existing history).
- Tests: timeline (fades per clip, delete, append, idempotence "no second copy"), app (double-click Fade/Volume/Scale/LUT/Subtitles on clips 1..3, undo, delete clip 2, appended clip none), ux (double-click, popups), project (v8 example conversion keeps each clip's picture/sound: compare render argv before and after on a 1.6.0 example, hostile values, schema 9 refused by 1.6.0 check = schema 10 refusal test exists), frame copies (copy edit scope, remove keeps last key).

## 5. Splash 1.6.1 (ADR-127)
assets/splash/1.6.1.gif = copy of 1.6.0.gif (Andrea 2026-11-19). assets/splash/1.6.1.txt = "New in this version" + 4 lines from business.md REFINE (fits 6-line limit). Splash tests: summary file exists and <= 6 lines; GIF decodes 486x506. Origin/licence still open (business limitation).

## Decided: Q-F (2026-11-19, Andrea: option A for Scale and Move; the clip's own LUT wins) — CLOSED
Was: a 1.6.0 clip that already has its own Scale, Move or Apply LUT while the whole video has the same effect (1.6.1 "one per clip, no stacking" cannot keep the 1.6.0 picture exactly). Options were A (combine Scale/Move; clip LUT wins), B (drop the whole-video value on those clips), C (bake the LUT, second track; rejected). Answer: A for Scale and Move, clip's own LUT wins for LUT.
Rule (format 8 -> 9 in src/project/v9.rs; the PE implements the details):
- Scale and Move, when the clip has its own key of that kind and the whole video has a path of that kind: one clip path. Times T = {0, clip length} + the clip's own key times + the whole-video key times shifted to clip time (whole time minus clip start), only those inside [0, clip length]. At each t in T, sampled with Key::at: scale = clip_scale(t) x whole_scale(clip start + t) / 1000, rounded, clamped to MIN_SCALE..MAX_SCALE (100..4000); move = clip_move(t) + whole_move(clip start + t) per axis, clamped to the existing move limit. Exact at each t in T; between two times the picture is an approximation (product of two piecewise-linear paths). An extreme product that hits the scale clamp changes the picture for that clip only.
- More than 200 times (TECHNICAL, decided): 0 and clip length are always kept; then the clip's own key times, then the shifted whole-video times, in time order, until 200. Own keys win; the picture is exact only at kept times.
- Clip without its own key of that kind: copied as in the plan (exact), as before.
- Apply LUT: a clip with its own LUT keeps it and the whole-video LUT is not applied to it. Other clips get the whole-video LUT as their clip LUT.
- Note (TECHNICAL, wording by PE, PM may refine): when a clip was combined or lost the whole-video LUT, one line in the status line at opening (the channel used for a missing LUT file) says how many clips were combined and how many kept their own LUT. No undo step, no change mark. No line when no clip has both.
- Tests (PE): 1.6.0 example with a clip Scale and whole Scale, a clip Move and whole Move, a clip LUT and whole LUT; exact picture at each time of T (pure function test); 300-time case gives 200 keys with own keys kept; hostile values refused; note only when a clip changed. Q-F test is no longer open.
- No other SEVERE question. Volume and Fade conversion exact (volume rounded to 1%, assumption).

## Ordered steps for the Principal Engineer
1. build.yml (CI, independent of code): steps 1-3 above; PE measures on the next push (follow CI memory: poll the API until the run ends).
2. Preview: fix ADR-120; add the two repro tests (REQ-144) and keep REQ-034 tests green.
3. Pure frame/box geometry: Hit::Edge, visible_part, scale_to_edge; tests REQ-145/146 (pure first).
4. Model: Clip.fade_in/fade_out, remove start_fade/end_fade and whole_* / whole_lut fields from Timeline (keep behaviour via migration); fades_of; set_clip_fades on clip flags; tests REQ-083.
5. Format 9: SCHEMA_VERSION 9, src/project/v9.rs conversion from 8 (Q-F cases: rule in Decided Q-F), tests REQ-149 (example 1.6.0 file, hostile values, 1.6.0 refusal, Q-F cases).
6. Whole-video commands per effect (REQ-147, 082): Timeline methods, app commands double-click, one undo step, no second copy; tests.
7. Whole-video Scale/Move copies (REQ-148): popup logic (first clip view, change all copies), picture handles on clip under playhead; tests.
8. Splash assets 1.6.1.gif (cp) and 1.6.1.txt; splash tests.
9. Coverage >= 85%, feature coverage 100% (`// covers:` on every REQ-144..149, 082, 083, 145, 146, 034 kept); docs (documenter).

## Tests per REQ (summary)
REQ-144: two repro tests (preview.rs), play->pause-same-key, end-of-playback pause; ffmpeg integration: paused frame at fade and at picture after edit (existing harness). REQ-145/146: see 3. REQ-147/082/083: see 4. REQ-148: copies. REQ-149: conversion, incl. Q-F cases (combined path exact at its times, 200 cap with own keys kept, LUT conflict, status line note only when a clip changed, no note otherwise). QG-06: no covers; measured.

## NFR (1.6.1)
- Pause/seek still: one render after the 80 ms quiet period, as before; no extra render for edits that do not change the picture (REQ-034 test).
- Double-click on N clips: O(N) state writes; a 200-key cap per copy; project conversion O(clips x keys).
- CI: check job <= 5 min typical (estimate), no test removed, coverage >= 85%.
- Portability: no new crate, no OS-specific code; MSRV 1.100 unchanged.

## Security (1.6.1)
- Conversion reads untrusted numbers: checked and capped before copies; damaged -> refused, never crash (REQ-057, property-style tests).
- CI: no new action; cargo-about prebuilt via the pinned install action (supply chain check: cargo-deny unchanged).
- No network, no new file written outside the existing private folders.

## Crates (1.6.1)
No new crate. Versions unchanged (cpal 0.18.2, whisper-rs 0.16.0, cargo-about 0.9.3 build tool, cargo-deny 0.20.2, cargo-llvm-cov via taiki-e).

## Risks (1.6.1)
- Users expect a clip added later to get the whole-video effect (documented rule; no message yet).
- Whole-video keyframe copies diverge; the whole-video popup shows the first clip only (known limit).
- Q-F rule (decided) changes the picture of rare old projects that had both effects: exact at key times, approximate between them; the clip's own LUT replaces the whole-video LUT there.
- Handle clash: edge handles vs picture drag (Move) and clip drags (PE test).
- CI speed target depends on the apt mirror; the 1.6.0 stall (20 min) is outside our control; timeouts bound it.
- Splash image is the 1.6.0 one (stale look); origin/licence open.
- The first clip's start fade moves with the clip (accepted by PM).

## Future work (architect)
- Whole-video copy divergence message (idea); a message when a clip is added after a whole-video effect.
- Pinned ffmpeg for the tests (shipped build), if the apt step stays slow.
- Q-F: if a user reports a changed picture in an old project that had both effects, reconsider exact baking (Q-F option C, rejected 2026-11-19).
- Parallel executable job (rejected for now; revisit if CI time remains the issue).
- ai-docs/architecture.md is 1044 lines (176 KB), far above the short-note rule: compaction of the 1.2.x..1.5.x ADRs and NFR history into architecture-adr*.md is owner work, not done in this PLAN.
