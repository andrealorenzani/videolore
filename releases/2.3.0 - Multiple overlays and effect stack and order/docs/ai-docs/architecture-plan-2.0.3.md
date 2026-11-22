# ai-notes: plan 2.0.3 — readable text, subtitle placement, subtitle accuracy (owner: architect; index in ai-docs/architecture.md)

Scope: ONE development 2.0.3 with three batches in the order of D50: A readable text (REQ-155 paused fix first, then REQ-156..167), B placement (REQ-168..176), C accuracy (REQ-177..179). Step 0 is a CI regression fix (blur chroma radius). ADR-152..167. Q-1 and Q-2 are CLOSED by Andrea (2026-11-20): Q-1 = B (old texts become Readability None and lose the 8% outline on opening; REQ-158 as written; no hidden value); Q-2 = A (old texts change size on opening as REQ-167 is written; one count message; no restore command; no size_rule field).

## 0. Facts found by reading (2026-11-20, HEAD e32376d, tree version 2.0.3)
- Blur fill: src/render.rs:604-614 `compose_frame`: small picture `sw=(w/BLUR_DIVISOR).max(2)&!1`, `sh` likewise (BLUR_DIVISOR 8, src/frame.rs:29); luma radius `BLUR_RADIUS(3).min((min(sw,sh)/4).max(1))`; chroma radius is the literal `2` (`boxblur={r}:2`). The failing case of tests/ffmpeg_integration/v15.rs (second half of `a_4_3_video_in_a_tall_frame_with_a_colour_fill`): frame 68x120 -> sw 8, sh 14 -> chroma plane 4x7 -> ffmpeg bound for chroma radius < 2. It is a PRODUCT bug for pictures whose blurred background is smaller than about 16 px on one side (frames about 64 px wide or less): the export fails with an ffmpeg error. Colour fill (`color=` source) is not affected.
- Paused still at a clip end: `render::still_args` -> `segments(project, at)` -> `skip = at - start`; the flat graph (render.rs ~line 741-745 and 790-800) uses `trim=start={off}:duration={d}` with `off = src_in + skip - input.start`. `trim` keeps frames whose pts >= off; the frame that covers the playhead starts before it (1.96 s for 1.98 s at 25 fps) and is dropped; the remaining window (d = 20 ms) holds no frame -> no picture. 1.92 s works because a frame starts exactly there. Same path for playback started mid-clip (preview.rs `launch` -> `preview_video_args`). Export starts at skip 0 (unchanged).
- Text fit today: src/text.rs:401-446 `rows_at` (0.6 em per character), `fit_in` (bisection, rows x 1.2 em), `fit_px` (min `MIN_FIT` 20/1000 of H, floor 6 px), `estimate_box` (ui boxes). No font measurement anywhere; no size cap.
- Text ASS: src/text.rs:555-570 `base` = `\an5\fn{font}\fs{px}\bord{outline(px)}\1c...`; lines joined with `\N`; the style `Text` (src/ass.rs:228-233) has BorderStyle 1, OutlineColour `&H00000000`, Outline `outline(size)` = 8% of size. So a text of 2.0.2 already has a black outline of 8% of its size (this is the "look" that REQ-158 and the migration rule must keep: Q-1).
- Subtitle preset `Boxed` already draws a semi-transparent box (src/ass.rs:139, `box_padding`); the text-item box can reuse that construction (a separate style row with BorderStyle 3).
- Fonts: src/fonts.rs has families and the background scan only; NO face bytes (the components note's "bytes for layout (cap 64 MiB, LRU 8)" is stale, see Future work). Export already copies the bundled font into the work folder's `fonts/` (src/export.rs:827) and the filters use `fontsdir=fonts` (render.rs:744, 774, 859).
- Fonts already in Cargo.lock: skrifa 0.44.0 (MIT OR Apache-2.0, MSRV 1.85), read-fonts 0.41.0, harfrust 0.12.0 (MIT, MSRV 1.85), both via epaint 0.37 (egui). fontdb 0.24.0 (MIT) has `with_face_data`. No new package is needed for measuring.
- STT: src/stt.rs `recognise` (line ~675-730): audio extracted once to `audio.f32` (16 kHz f32), read in 120 s chunks (`CHUNK`, line 88) with NO overlap, `transcribe(&pcm, offset, ...)`, words appended. `WhisperTranscriber::load(model, threads)` (src/stt.rs:973) sets no DTW parameters; `FullParams` sets `set_token_timestamps(true)`, `set_split_on_word(true)`, `set_max_len(1)` (line ~1039). whisper-rs 0.16.0 has `WhisperContextParameters::dtw_parameters` with `DtwMode` and `DtwModelPreset` (Tiny, Small, LargeV3Turbo, ...).
- Accuracy test: tests/ffmpeg_integration/v12.rs:333-345 `alignment_matches_real_speech` is `#[ignore]` and calls `alignment_with_real_speech(true)`; it needs `VIDEOLORE_STT_MODEL` (CI sets the tiny model, build.yml:27, sha-checked at build.yml:87). Fixture tests/fixtures/align (speech.mkv, reference.json, track.srt, make.sh; espeak-ng is used only by make.sh, build.yml:42).
- align.rs: `WINDOW` 1 s, `MIN_WORD` 10 ms, `MIN_MATCHED` 0.5, `Alignment.unmatched` (cues spread evenly) — REQ-179 can use `unmatched` directly.
- Project format: SCHEMA_VERSION 9 (src/project.rs:30), loads 1..9, TooNew refused (line ~308). TextItem (src/text.rs:108-130): id, text, start, ttl, effects, colour, font, place (centre), width (permille, `#[serde(default)]` for new fields is the pattern), anchor. Subtitle `Line.position: Option<Pos>` exists (ADR-057).
- Subtitle lines: `line_tag` (src/ass.rs:98) uses `\an5\pos` for an explicit place (the centre is kept by libass); a line on the style's place is bottom-anchored (its centre moves when it grows).
- Module sizes: src/text.rs 1211 lines, src/render.rs 3439, src/app.rs 3073, src/stt.rs 1957 (debt, see risks).
- Business headings: docs/business-subtitles.md says "2.0.4" for REQ-169..176 and docs/business-subtitle-accuracy.md says "2.0.5" for REQ-177..179; this plan builds all of them in 2.0.3 (command). Mismatch recorded for the Product Manager (Future work).

## 1. Components touched (per batch)
| Module | Change | Batch |
|---|---|---|
| `render` (render.rs) | blur chroma clamp (compose_frame); trim start on the frame that holds the playhead when skip > 0 (segment graph); text layouts passed in (pure); fonts copies named in the input | 0, A, B |
| `export` (export.rs) | copy the chosen installed font files into the work `fonts/` next to the bundled font | A |
| `preview` (preview.rs) | the same font copies in the preview work folder (still and playback); no other change | A |
| NEW `measure` (src/measure.rs, pure) | face metrics (skrifa), shaping widths (harfrust), greedy wrap, best fit (REQ-156/157/159), min 2% rule | A |
| `fonts` (fonts.rs) | face bytes on demand via fontdb `with_face_data`, cap 64 MiB, LRU 8; resolution of a family name (installed or missing, REQ-164); copy into a folder | A |
| `text` (text.rs) | TextItem: `readability` (ADR-157), `height` (permille 50..1000, default 200); `fit_px` replaced by measured fit; `events` takes the layout; `legacy_fit_px` (2.0.2 estimate) kept for the migration count; `\q2`; Readability styles | A, B |
| `ass` (ass.rs) | style rows `Text` (None/Outline, bord set per event) and `TextBox` (BorderStyle 3, `&H66000000` = 60% opaque); constants for the new styles | A |
| `project` (project.rs) | SCHEMA_VERSION 10 (ADR-158); loads 1..10; the migration count (REQ-167) computed at load | A |
| `app` (app.rs, app/texts.rs, app/command.rs) | commands: SetReadability, SetTextHeight, edge-resize with live box (commit on release), MoveInside, group move of lines and texts (REQ-172), Align on the style's place (REQ-173), centre keeping (REQ-174), REQ-168 after enlarging edit, status counts | A, B |
| NEW `placement` (src/placement.rs, pure) | box rectangles of texts and lines (from `measure` and `ass` sizes), least move inside (REQ-168/175), centre keeping (REQ-174), overlap pairs (REQ-169), partly-outside test (REQ-175/176) | B |
| `ui` (boxes.rs, picture.rs, text_popup.rs, timeline.rs, popups.rs, status.rs) | bottom and left handles, live box and readout (REQ-159/160/161), warning on box, Readability choice and font note (REQ-158/164), blink dashed outline (REQ-162), stacked hit cycling (REQ-170), overlap warnings (REQ-169), dashed outline and status count (REQ-175), hint (REQ-171) | A, B |
| `stt` (stt.rs) | 10 ms energy envelope kept during recognition (ADR-164); piece reads with 2 s margins and ownership by start (ADR-165) | C |
| `align` (align.rs) | one-sided onset snap of matched word starts, window [t-1.0, t-0.2] s (ADR-164, D56) | C |
| `app/subs_create.rs` | REQ-179 status message from `Alignment.unmatched` | C |
| `player` | no code of its own: inherits REQ-155 (render) and the text rules (ass/render); its tests run on the shared code | A |

Layering: `measure` depends on no crate module; `placement` depends on `measure`, `text`, `subtitles`, `base::pos`; `app` and `ui` use `placement`. No new cycle (tests/module_cycles.rs must pass). `text` keeps producing events; the font layouts are computed by `app` (cache) and passed into `render` as data, so render stays pure.

## 2. Step 0 — CI regression: blur chroma radius (ADR-152)
- Rule: `sw = (w / BLUR_DIVISOR).max(BLUR_MIN_SIDE)&!1` with `BLUR_MIN_SIDE = 8` (was 2); `sh` likewise.
- Luma radius unchanged: `BLUR_RADIUS.min((min(sw,sh)/4).max(1))`.
- Chroma radius: `BLUR_CHROMA_RADIUS(2).min((min(sw,sh)/4).saturating_sub(1))`; the graph is `boxblur={r}:{c}`.
- Bound derived from the ffmpeg message (chroma radius must be < min(chroma_w, chroma_h)/2 with chroma = luma/2): for sw=8, sh=14 the maximum is 1. To verify on the pinned 9.0.2 build with a unit test over sizes 8..=600 (the graph's radii satisfy both bounds) and by the v15 test.
- Golden: for w/8 >= 12 the chroma radius stays 2 and the strings of render.rs:3131 are unchanged (`boxblur=3:2`).
- Tests: `covers: REQ-126` (existing blurred-fill tests) plus the new unit test for the bounds (no REQ, it is a fix; the feature test is v15).

## 3. Batch A — readable text (REQ-155..167)
### 3.1 REQ-155 paused picture at the clip end (ADR-153)
- Cause (reasoned from the graph, to be confirmed by the test): the flat segment trims from the exact playhead; the frame covering the playhead starts earlier and is dropped; for the last frame interval there is no later frame in the window. By this reasoning the failing window is the whole last frame interval (about 40 ms at 25 fps, from 1.96 s), while REQ-155 says "~20 ms"; the test measures the real window and the requirement wording is kept (the defect is the same).
- Fix (only when skip > 0, i.e. still and playback from mid-clip): start the trim at the source frame that contains the playhead (its pts <= off), keep `setpts=PTS-STARTPTS+{o}` so the picture is at the playhead, and set duration to reach the segment end: `trim=start=F:duration=d+(off-F)`. F is found from the source frame grid (media fps; the PE measures the real pts with ffprobe in the test and falls back to the exact `off` for VFR media without a grid).
- Export (skip 0) argv unchanged: the golden tests must stay byte-identical.
- Tests: `covers: REQ-155` in tests/ffmpeg_integration/v20.rs (2 s clip at 25 fps, still at 1.98 s and 1.995 s, reached by seek and by an edit; with and without effects: picture not black; a fade through black still black at its own time); a unit test on the graph string (trim start on the grid); REQ-144 keeps its tests.

### 3.2 Best fit with measured widths (ADR-154; REQ-156, 157, 159 part)
- Method (chosen): shape each text once per (text, face) with harfrust (kerning and ligatures as libass does), advances in font units from skrifa metrics; scale with px/units_per_em. Line height = (ascender - descender) of the hhea table, scaled (assumption, checked against libass by a test that renders one line and reads its ink rows, see ADR-154).
- Wrap: greedy at word spaces at the box width; a word wider than the box is split by characters; `\n` in the text stays a hard break. The wrapped lines are written as text joined by `\N` with `\q2` (no automatic wrap by libass), so preview and export show the same lines.
- Fit: largest px in 0.1 px steps with `lines(px) * lh(px) <= box_h`, px <= 10% of picture height (REQ-157), px >= 2% of picture height (REQ-116 minimum: text that still does not fit runs past the box, REQ-111; a warning, REQ-159, when the text does not fit at the minimum). The fill is then 90–100% of the box height by construction (the step is 0.1 px), except at the cap or the minimum.
- "Fill" is measured as lines x line height over box height (assumption; ink height is smaller and is not used).
- Alternatives rejected: keep the character estimate with a new constant (still a guess, the 85–115% error stays); render-and-measure through ffmpeg (a process per edit, too slow); libass metrics (not exposed by the binding); skrifa advances only (no kerning: about 1–3% width error, accepted only as a fallback if harfrust gives trouble).
- Cost: shaping of a 200-character text ~ms (estimate); cached in `app` per (text, font, box w, box h, picture h); bisection is on widths already in font units (no shaping per step).
- Migration count (REQ-167): `legacy_fit_px` (the 2.0.2 estimate, kept as it is) and `fit_px` (new) compare per item at load of a format <= 9 project; the count of items whose px differs is shown once.

### 3.3 Readability (ADR-157; REQ-158)
- Stored: `Readability::{None, Outline, Box}` as serde snake_case enum, `#[serde(default)]` = None for old items (format <= 9 texts open as None, Q-1 = B); new items = Outline. No other value.
- None: `\bord0`, style `Text`. Outline: style `Text`, `\bord{round(6% of px)}` black (OutlineColour `&H00000000`). Box: style `TextBox` (BorderStyle 3, OutlineColour `&H66000000`), `\bord{round(10% of px)}` as box padding (libass uses the outline width as the box padding for BorderStyle 3; test to confirm).
- Old texts lose their 8% black outline on opening (Q-1 = B). The one-time REQ-167 status message (shown only when the project has at least one text item; never again once saved as format 10; no restore command) names both: the count of texts whose size changed (size changes only) and that the old outline was removed. The count stays of size changes only.
- BorderStyle is a style field: the Box needs its own style row referenced by `\r`; the `\alpha` of a blink (REQ-162) covers the box and the outline.
- Same ASS in preview and export: parity by construction (both call `text::events`).
- Tests: `covers: REQ-158`: export pixels — with Outline, dark pixels at the letter edges; with Box, the area behind the text at least 50% darker than the white frame; with None, no outline or box; a format 9 text opens as None (no outline in its events).

### 3.4 Size cap, height, handles (REQ-157, 159, 160, 161)
- Cap 10% of picture height (REQ-157): applied inside `fit`; "Hi" in the default box is 10% high.
- Height: `TextItem.height` permille 50..=1000 (5%..100%), default 200 (= 2.0.2 BOX_HEIGHT). The top edge stays; the bottom handle changes height; `place.y` moves by half the change (the centre moves).
- Left handle (REQ-161): the right edge stays; `place.x` moves by half of the width change; width 100..=1000.
- Live box and readout (REQ-160): while a handle is dragged the UI draws the box and readout from a UI-state value; the command is sent on release (one undo step). No history entry during the drag.
- Warning (REQ-159): on the box in the preview when `fit` returns the minimum size and the text does not fit; export unchanged.

### 3.5 Fonts and missing fonts (REQ-163, 164, 165, 166; ADR-155, 156)
- Resolution in `app` (once per font change): installed family -> face bytes (fontdb `with_face_data`, read on demand, cap 64 MiB, LRU 8); not installed -> the bundled subtitle font is written as `\fn Noto Sans` explicitly (never libass's silent fallback), and the popup shows "Font not installed: the bundled font is used" (REQ-164).
- Drawing an installed font (ADR-156): copy its file into the work folder's `fonts/` (the folder export already uses for `fontsdir`), named by its sha-256 prefix; the family name written in `\fn` is the name table ID 1 (what libass matches). TECHNICAL: verify that libass picks the copied file before the system provider on each OS; fallback if not: `\fn` with the family name only (REQ-165 test by hand on three systems).
- Measure and draw use the same face selection: weight normal, style normal (no bold tag in text items).
- REQ-166: the scan stays on its background thread (src/fonts.rs `Fonts::start`); a face is read only when a text uses it; tests: the window-independent part (scan not on the UI path; a 20 MB face is not read at start: `covers: REQ-166`); the 0.5 s window delay is a manual measurement (business-verification).

### 3.6 Project format 10 (ADR-158)
- SCHEMA_VERSION 10; loads 1..=10; writes 10. 2.0.2 refuses format 10 (TooNew) — a project with the new fields is never silently changed by 2.0.2.
- New TextItem fields: `readability` (default None on load), `height` (default 200). No size flag: old texts change size on open (Q-2 = A).
- No other format change: subtitle positions, Line.position, presets are unchanged.

## 4. Batch B — placement (REQ-168..176)
Pure rules in `placement` (ADR-161): `rect(item or line, picture)` (text box from `measure`; line box from the ASS text size as the existing `ass` layout), `least_inside(rect, picture) -> (dx, dy)` (centred on an axis when larger than the picture), `keep_centre(old_rect, new_size)`, `overlaps(a, b)` for lines shown at the same time (time overlap from the lines' times, box overlap from rect), `partly_outside(rect)`.
- REQ-168: after a text edit that enlarges the box (new fit larger than before or more lines), `least_inside` applies; one undo step with the edit (app/texts.rs).
- REQ-169: derived warnings (no stored flag, ADR-161): Subtitles popup list and timeline marks; cleared when boxes or times no longer overlap. Export unchanged.
- REQ-170: UI state (ADR-162): stacked hits at the click point ordered by playhead visibility; a second click on the same point (same place within 4 px, within 1 s) selects the next; not saved.
- REQ-171: status hint text built in ui/status.rs from the selection state; wording is an engineering choice.
- REQ-172: group move as ONE command (`MoveSelection{ids, dx, dy}` in app; ADR-163) for lines and texts, one undo step; each selected item ends at the dragged item's final place.
- REQ-173: Align horizontally/vertically on a line at the style's place: store the explicit place (`Line.position`) with the axis at the picture centre; command greyed with "already centred" when all selected items are centred on the axis (is_enabled/disabled_reason in app.rs).
- REQ-174: when a line's size or wrap changes (REQ-097 or a new line count) and the line is on the style's place, store an explicit place with the old box centre before the change (centre kept); lines with an explicit place already use `\an5` (centre) so libass keeps the centre. Then REQ-168 rules apply.
- REQ-175: dashed outline in the preview for partly-outside items; status "N items are partly outside the picture" counted at the playhead; `MoveInside` command (ADR-161) for the selection; greyed out when none is partly outside.
- REQ-176: Scale, Move and Change aspect never move items (existing placement by frame fraction, REQ-131); the warnings of REQ-175 apply.
- Tests: `covers: REQ-168..176` with pure unit tests in `placement` (boxes, centre, least move, overlaps) and app tests (undo steps, greyed reasons), UI tests (hit cycling, dashed outline, hint), and one ffmpeg test for REQ-174 (centre within 0.5% of picture height in preview and export: the preview uses the same events, the export test reads the ASS from the work folder).

## 5. Batch C — subtitle accuracy (REQ-177..179)
### 5.1 Measures and levers (ADR-164, D56)
1. Measure the current state on the reference clip with the Fast level (the existing ignored test, PE runs it with VIDEOLORE_STT_MODEL). Record matched share, mean and maximum start error, and per-word errors in a scratch note (not committed).
2. Lever A (DROPPED, D56 2026-11-20): DTW token timestamps were measured and reverted: 23 of 28 words matched (5 misheard), matched words 0.43 to 0.75 s late. No DTW preset, no change to `WhisperTranscriber::load`.
3. Lever (the one used, D56): one-sided onset snap in align.rs (pure function): for each matched word, its start moves to the latest energy onset in the window [t-1.0 s, t-0.2 s] (t = the recognised start), with onsets taken from a 10 ms RMS energy envelope of the 16 kHz audio kept during recognition. Measured on the reference: energy onsets lie within 0.03 s of the reference starts. No fuzzy matching, no larger model.
4. Lever C (not planned): a larger model. Decided by Andrea only (D54); not part of 2.0.3 (the reworded REQ-177 target is measured with the current Fast model; if it cannot be met, the PE stops and reports, see Future work).
### 5.2 REQ-178 pieces (ADR-165)
- Pieces of CHUNK (120 s). Piece c reads audio [c*CHUNK - 2 s, (c+1)*CHUNK + 2 s] (clamped to the audio), transcribes with offset = the read start, and keeps only the words whose start time lies in [c*CHUNK, (c+1)*CHUNK) (absolute, base-adjusted). A word is then kept once (by its start), and a word cut at a boundary is heard in full by the piece that owns its start (words < 2 s).
- Cost: +3.3% audio work at the Fast level (4 s per 120 s); QG-01 allows +5%.
- Rejected: 5 s margins (+8.3%, over QG-01); one-sided margin (a boundary word heard as a fragment by the next piece starts after the boundary and is kept twice).
- Reading: seek in audio.f32 by byte offset (`start * 16000 * 4`); buffer of CHUNK + 4 s (about 8.6 MB), memory bounded as today.
- Tests: `covers: REQ-178`: unit test with a fake transcriber over a synthetic 5-minute audio (a word every 0.7 s with known times, some across each boundary): no word missing, none twice (`stt` tests, no model); env-gated test with a 5-minute speech fixture (tests/fixtures/align-long, transcript committed) for the real model.
### 5.3 REQ-179 message (ADR-167)
- After matching, when the track was used (REQ-087) and `Alignment.unmatched * 2 >= lines.len()` (lines counted with text), the status message adds: the video's subtitle text does not follow the speech closely, so the word timing is approximate; the text is kept (wording in English as the other messages; the count is shown: "N of M lines").
- Tests: `covers: REQ-179`: app/subs_create test with the fake transcriber and a paraphrased track (tests/fixtures/align/paraphrase.srt, new, hand-written): message with count; the committed track: no extra message.
### 5.4 REQ-177 gate (ADR-166)
- Gate as reworded by Andrea (D56, docs/business-subtitle-accuracy.md): at the Fast level, at least 90% of the MATCHED words start within 0.3 s of the reference, after the onset snap. The matched share (words found) is reported, not required. No larger model, no fuzzy matching. Balanced and Most accurate are measured and must not be worse than Fast (not gated).
- The strict test `alignment_matches_real_speech` loses its `#[ignore]`; it asserts the reworded gate: every word counted as matched starts within 0.3 s of reference.json for at least 90% of the matched words; the matched count and the share are printed.
- CI sets `VIDEOLORE_REQUIRE_STT=1` (new, same rule as VIDEOLORE_REQUIRE_FFMPEG, ADR-142): a missing model or fixture fails, it never skips silently. The model file is the sha-checked tiny one already in build.yml.
- The fixture is checked by sha-256 in the test (speech.mkv, reference.json, track.srt) so a changed fixture fails; espeak-ng is recorded as the generator version in tests/fixtures/align/make.sh (only used to regenerate, never at test time).
- Balanced and Most accurate are measured (scratch note) and must not be worse than Fast; not gated beyond that.

## 6. Data model and format impact (summary)
- Format 10 (ADR-158): TextItem.readability, TextItem.height. No subtitle format change. Projects of 2.0.2 open in 2.0.3 unchanged except texts (REQ-167 count).
- Settings: none. Work folder: `fonts/` gains installed font copies (cleaned with the folder).
- Undo: each command is one step (SetReadability, SetTextHeight, edge resize on release, MoveSelection, MoveInside, Align, REQ-168 with the edit).

## 7. Export and preview parity
- Both call `text::events` with the same layouts (`app` cache) and the same font copies: outline, box, size, wrap, height and position agree; the preview is the same ffmpeg subtitles path (no separate drawing).
- The only preview-only things: the dashed outline of a selected hidden text, the live box during a handle drag, the warning on the box, the status counts.
- Export tests compare the drawn pixels (REQ-158) and the ASS (REQ-156/174).

## 8. Performance and security
Performance (NFR details in ai-docs/architecture-nfr.md): layouts cached (no shaping per frame); fit cost per text ~ms (estimate, `base::testing::best_of` gate in tests); UI frame <= 200 ms with 200 texts (existing gate); font reading off the UI path; pieces +3.3% audio work (QG-01 +5%); the onset envelope adds memory (about 7 MB per hour, estimate; to measure; the Most accurate note of 1.3 GB stays the reference).
Security (details in ai-docs/architecture-security.md): font files are read from the system font database only; the project keeps a family name, never a path; copies capped (64 MiB, sha-named) and cleaned with the work folder; skrifa/harfrust parse untrusted bytes: no unsafe in videolore; any parse failure gives no layout and the bundled font (test with truncated bytes); no user text in style names (`TextBox`, `Text` are constants); names written into `\fn` pass `valid_font`.

## 9. Tests per REQ (summary; each test carries `// covers: REQ-NNN`)
| REQ | Test (where) |
|---|---|
| step 0 | unit bounds for sizes 8..600 (render); v15 blurred fill 68x120 (tests/ffmpeg_integration/v15.rs) |
| 155 | v20 paused at 1.98 s and 1.995 s, seek and edit, effects, fade through black; graph unit test |
| 156 | measure unit tests (ten texts: short, wide W, narrow i, 1–4 lines; fill 90–100%); export size = preview size within 2% (ffmpeg test) |
| 157 | "Hi" at 10% of H in preview and export (within 2%); long text still fitted |
| 158 | v13 pixel tests (Outline dark edges; Box >= 50% darker; None no outline; format 9 text opens as None, no outline in its events) |
| 159 | height 20% -> 40% grows; 3% stops at 5%; long text in 5% box warns; one undo |
| 160 | right handle to 50% shows 50% and the size; no undo step during drag; release = stored box |
| 161 | left edge: 40% wide at 50..90 -> dragged to 30 gives 60% and right edge 90; stops at 10%; one undo |
| 162 | blink 0.5 s: no box in export at 0.75 s, box at 0.25 s; hidden selected text has dashed outline only in preview |
| 163 | installed test font wider than the bundled one: preview width = export width within 2%; fits its box |
| 164 | missing font opens; drawn in bundled font; note; saved name kept |
| 165 | lookup of a test font installed for the test (Linux in CI; macOS and Windows by hand, business-verification) |
| 166 | scan off the UI path; face not read at start (20 MB face) |
| 167 | old project with one large text: count shown once; widening the box makes it larger |
| 168 | enlarge -> on the picture edge; shorter -> unchanged; one undo |
| 169 | two overlapping lines listed with the warning; moved away clears it; different times no warning |
| 170 | stacked lines: first click one, second click the other; a line at another time not picked |
| 171 | hint names both gestures with one line selected; none with nothing selected |
| 172 | line and text group move ends at the dragged place; one undo |
| 173 | style-place line Align horizontally: centre on vertical centre line; greyed "already centred"; one undo |
| 174 | centre within 0.5% after a second line (preview and export) |
| 175 | dashed outline and count; Move inside puts the whole box inside; one undo; greyed when all inside |
| 176 | Scale 150% at the right edge: status and dashed outline; place as fraction unchanged |
| 177 | strict alignment test un-ignored, CI with VIDEOLORE_REQUIRE_STT=1 |
| 178 | synthetic 5-minute piece test (no loss, no duplicate) and env-gated real 5-minute fixture |
| 179 | paraphrased track: message with count; committed track: no message |

## 10. Ordered steps for the Principal Engineer (one development, 2.0.3)
0. Step 0 (blur): fix render.rs compose_frame (BLUR_MIN_SIDE 8, chroma clamp), unit bounds test, run v15. Commit-ready.
1. REQ-155: trim start on the playhead frame when skip > 0 (render.rs segment graph); v20 tests; golden unchanged.
Batch A:
2. `measure` (new) with unit tests (fit, wrap, cap, minimum, legacy count); face metrics and shaping from fonts bytes; test fonts from the bundled file.
3. `fonts`: face bytes on demand (fontdb `with_face_data`, cap, LRU 8), name resolution, copy into work `fonts/`; tests REQ-163/164/166 (unit + ffmpeg where needed).
4. `text`: readability and height fields (serde defaults), new `fit_px` (measured) via the layouts, `legacy_fit_px` kept; `events` with `\q2`, explicit `\N`, Readability styles in `ass`; tests REQ-156/157/158.
5. `project`: SCHEMA_VERSION 10; load of 1..9 with defaults; count of changed sizes at load (REQ-167); tests (format 9 fixture, format 10 round trip, TooNew for 11).
6. `app`: commands SetReadability, SetTextHeight, edge resize (live via UI state, commit on release), undo labels; cache of layouts per (text, font, box, picture); tests REQ-159/160/161 (app side), REQ-167.
7. `ui`: handles (bottom, left), live box and readout, warning, Readability choice and font note in text_popup; blink dashed outline (REQ-162); UI tests.
8. Verify batch A: `cargo fmt`, clippy, tests (PE), coverage, feature coverage. Report.
Batch B:
9. `placement` (new, pure) with unit tests for every rule (REQ-168/169/172/174/175/176 rules).
10. `app`: REQ-168 after edit; MoveSelection (lines and texts); Align on style place with greyed reason; REQ-174 centre keeping; MoveInside; derived warnings and the status count at the playhead.
11. `ui`: stacked hit cycling (REQ-170), dashed outline (REQ-175), overlap warnings in the Subtitles list and timeline (REQ-169), hint (REQ-171), group drag (REQ-172).
12. Verify batch B (suite, coverage, feature coverage).
Batch C:
13. Measure (scratch, not committed): Fast level on the reference clip before any change; record numbers.
14. (Dropped, D56: no DTW.) Skip; the onset snap below is the accuracy lever.
15. `stt`: pieces with 2 s margins and ownership by start (REQ-178); unit test (synthetic) and env-gated 5-minute test (fixture added with make.sh; espeak version recorded).
16. `app/subs_create`: REQ-179 message and paraphrase fixture (tests/fixtures/align/paraphrase.srt).
17. Strict test un-ignored; CI env VIDEOLORE_REQUIRE_STT=1 and fixture sha checks in build.yml (PE owns .github/workflows).
18. align.rs: the one-sided onset snap (5.1, D56): 10 ms RMS envelope kept during recognition in stt.rs (pure function in align.rs over the envelope); tests: synthetic onsets, window edges, no onset in window keeps the recognised start. Re-measure the reference; if the 90% of matched words within 0.3 s is still missed: STOP and report (Future work: Andrea decides between a larger model, D54, and no other change).
19. Verify batch C; then the full suite and report the coverage and feature coverage.

## 11. Risks (also in ai-docs/architecture-risks.md)
- libass may prefer the system font over the copy in `fonts/`: verify on Linux in the test; on macOS and Windows by hand (REQ-165).
- Line-box fill vs ink: the "fill" is defined on the line box (assumption); a reviewer may expect ink height.
- REQ-177 as reworded (D56) may still be missed by the Fast model after the onset snap; then Andrea decides (larger model, D54). Matched share is 23 of 28 (82%) on the reference; reported, not gated.
- Recognition pieces with a 2 s margin fail for words longer than 2 s (none expected in speech; test).
- No DTW (D56): no extra memory from it; the envelope adds about 2 bytes per 10 ms (about 7 MB per hour of audio, estimate).
- Texts saved before 2.0.3 change size on opening (Q-2 = A) and lose their 8% outline (Q-1 = B); both are product decisions of Andrea, recorded in Future work and in the notes of the Product Manager.
- text.rs is 1211 lines and grows; if it passes ~1500 lines the event builder moves to `text/events.rs` (PE decision, not planned now).

## 12. Questions (closed 2026-11-20 by Andrea)
- Q-1 CLOSED = B: the 2.0.2 text has a black outline of 8% of its size (src/ass.rs style Text). Old texts become Readability None on opening and lose the outline; REQ-158 as written (None = no outline); no hidden value. The REQ-167 one-time status message names the outline removal as well as the size count.
- Q-2 CLOSED = A: old texts change size on opening (REQ-156/157 as written); the status bar shows the count once (REQ-167); widening the box is the way back; no restore command; no size flag.

## 13. Future work
- Batch numbers: business docs say 2.0.4 (placement) and 2.0.5 (accuracy); this development builds them in 2.0.3 (command). Product Manager to align headings and roadmap.
- Stale component claim: architecture-components.md says fonts keep "bytes for layout (cap 64 MiB, LRU 8)"; no such code exists in 2.0.2 (src/fonts.rs has families only). Built in 2.0.3 (ADR-155); the note is corrected by the architect at release.
- Cycle state: architecture-components.md lists 9 pairs and a 16-module group, code-architecture.md says none; the 2.0.1 cycle check is the authority: architect to re-check at release.
- REQ-165 cross-platform proof is manual (three systems); a test font for the lookup test must be OFL/MIT with its notice in packaging/notices.
- Subtitle preset Boxed and text Box share one construction: later, the Boxed preset could use the same rules (not planned).
- Speech model larger than Most accurate: not planned (D54).
- Old projects lose their outline on open (Q-1 = B): the REQ-167 one-time status message names the outline removal too (decided by the Product Manager, 2026-11-20); the wording is an engineering choice to settle in the UI tests.
- Format 10 migration note: opening a format 9 project and saving it writes format 10; 2.0.2 and older refuse that file (TooNew) — stated in the user documentation at release.
