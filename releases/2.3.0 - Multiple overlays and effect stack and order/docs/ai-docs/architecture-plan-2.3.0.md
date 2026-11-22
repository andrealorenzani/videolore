# Plan 2.3.0 (architect; PLAN 2026-11-22; REFINE 1 with Q1-Q9; REFINE 2 with Q-A, Q-B, Q-C answered the same day; no open SEVERE question)

Theme: the clip owns its overlays and its effects (overlap media, Fit and empty space per clip, Volume and Change speed as effects, Crop removed), Add media button, Never removed from the start screen, and the Enlarge word bug. Sources: ai-docs/business.md checkpoint 2.3.0 (PLAN and REFINE); docs/business-overlap.md (REQ-325, 327..331); REQ-280..283, 287, 295, 320..322, 324, 326, 332 (business-effects-frame, business-applied-effects, business-frame, business-splash, business-editing). Next free REQ: 333 (PM). ADRs 230..241 in architecture-adr-2.3.0.md.

## 1 Facts found in the code (read only, 2026-11-22)
- F1 Volume: keys are `Timeline.clip_volume` (timeline.rs:362-375, `ClipVolume{clip, keys: Vec<Key<u8>>}`); the Clip details row "Volume keys" is ui/popups.rs:647-661; render uses the keys, else the slider `Clip.volume`. Storage already gives one Volume per clip.
- F2 Speed: `Clip.speed` (timeline.rs:113-115, 500..2000) set by `Command::SetSpeed` from the Clip details row (ui/popups.rs:727). Storage needs no change.
- F3 Crop: `Clip.crop` (timeline.rs:103-105), `crop_filter` in render/graph.rs, `Step::Crop` (render/steps.rs:31), `Row::Crop` (effect_order.rs), the Crop % row of Clip details. All go (REQ-283 removed).
- F4 REQ-326 is a UI gap, not a storage gap: `FrameAspect` already has `shape`, `fit`, `fill`, `blur` (frame.rs:217-224) and `Timeline.clip_aspects` stores one per clip (timeline.rs:355-358). The clip row (ui/popups.rs:676-700) writes `fit: Fit::Height` on every shape choice and offers no fit, empty-space or blur choice. Per-clip render path exists (REQ-286, render/steps.rs Frame step).
- F5 Picture items: `TextItem.picture` (text), `picture.rs` (PNG header `parse_png_size`, `default_box`, `rect`, cap 16 Mpx at add, ADR-219), `render/plan.rs` `picture_overlays` (PicOverlay with `pieces`, render/graph.rs:262-273), still input `-loop` (render/inputs.rs:45, 85), overlay step (render/steps.rs ~380-386).
- F6 Hardware decoding exists only in preview: `hwdec::apply` (src/hwdec.rs:249, called from src/preview.rs:995) maps input files by codec; export uses hardware encode only.
- F7 Settings: `SplashOption::Never` (settings.rs:407); `lenient_splash` (settings.rs:421-424) parses any stored value with `serde_json::from_value(v).unwrap_or_default()`: once the variant is removed, a stored "Never" silently becomes Always (the default). An explicit mapping is needed. `splash::should_show` (splash.rs:72-85) matches Never; ui/settings.rs:401 lists it.
- F8 Enlarge word (bug, see § 8.8): `word_styles::enlarge` (src/word_styles.rs:115-147) puts `{\fs<big>}` inside the same line as the other words; `ass.rs` `line_tag` (ass.rs:167-178) gives the line `\an5` and `\pos`. The architecture note docs/architecture-subtitle-styles.md (2.1.0 section) says the built design is "one event for the enlarged word at its measured centre"; the code does not do that. Mismatch recorded; the note is corrected in this plan's stage (docs).
- F9 Project format: `SCHEMA_VERSION = 13` (project.rs:39); `version > SCHEMA_VERSION` gives `TooNew` (project.rs:439-440). The test at project.rs:1105 asserts `TooNew(14)` for the current format: it becomes 15 (test churn, not a behaviour change). Conversion modules: `src/project/v9.rs` (in-memory conversion, model for v13); `src/project/v13_tests.rs` exists.
- F10 Order: `Row` (src/effect_order.rs) has Lut, Text, Picture, Subtitles, Volume, Clean, Crop, Frame, Scale, Move, Fade, Unknown; names `picture:<id>`, `text:<id>`; `order_chain.rs` groups sound and picture rows (2.2.0); `project/order.rs` per-clip rows.
- F11 Timeline ids: `Timeline.next_clip_id` (timeline.rs:386) is the pattern for a new `next_overlap_id`.

## 2 Decisions (ADR-230..241 carry the reasons)
- Overlap is a field of its clip (`Clip.overlaps`), offsets in the clip's timeline time: split, trim, ripple and delete then follow the clip without a second data structure (ADR-230, ADR-231).
- Format 14 is one bump; 13 and older refuse it through the existing `TooNew` rule; 13 is converted in memory once at load with one status line (ADR-230, ADR-241).
- Overlap drawing reuses the picture-overlay path (after the concat for the default order; a step for a moved row); the three render paths are defined in § 5 (ADR-232).
- Alpha: overlay inputs are never hardware-decoded; size checked from headers or ffprobe before any decode (ADR-233).
- Edits that remove overlaps go through one popup built from a pure list of removals, one undo step (ADR-234).
- Enlarge word: the enlarged word is its own event at its measured place (ADR-235).
- REQ-326 changes only the UI; Volume and Change speed change only their UI and the copy rule; Crop removed from model, render and UI (ADR-236..238).
- Add media = the existing import action (ADR-239); Never removed with an explicit mapping (ADR-240).

## 3 Components
| Part | Where | Change in 2.3.0 |
|---|---|---|
| Overlap model (new, pure) | `src/overlap.rs` | `Overlap` type, clamp, cut, split, removals of an edit, speed rescale; imports only `time`, ids |
| Timeline | `src/timeline.rs` | `Clip.overlaps`, `Timeline.next_overlap_id`; `Clip.crop` read-only legacy field (13), never written |
| Project format | `src/project.rs`, `src/project/v13.rs` (new) | SCHEMA 14; load 1..=14; 13 converted in memory (crop dropped, picture items become overlaps, order rows rewritten) |
| Order | `src/effect_order.rs`, `src/order_chain.rs`, `src/project/order.rs` | `Row::Overlap(u64)` (`overlap:<id>`, picture group, on top by default); `Row::Crop` removed (read as Unknown, dropped by v13) |
| Render | `src/render/plan.rs` (picture_overlays generalised), `graph.rs` (PicOverlay kind and reshape), `inputs.rs`, `steps.rs` (Crop step removed; Over for overlaps), `overlay.rs` | three paths § 5 |
| Picture | `src/picture.rs` | `gif_size` (13-byte header, bounded), 16 Mpx check per kind; `parse_png_size` unchanged |
| Probe | `src/media.rs` (existing ffprobe use) | size of MOV overlay files at add and at each preview or export |
| Preview | `src/preview.rs`, `src/hwdec.rs` | overlay inputs excluded from `hwdec::apply` |
| Settings | `src/settings.rs`, `src/splash.rs`, `ui/settings.rs` | `Never` removed; `lenient_splash` maps "Never" to OncePerVersion |
| Commands | `src/app/command.rs`, new `src/app/overlap.rs`, `src/app/effects.rs`, `dispatch_*` | AddOverlap, SetOverlap (place, box, times, reshape), RemoveOverlap; Volume effect add or remove; Change speed copy to all clips; precheck of removals |
| UI | `ui/effects.rs` (rows), `ui/popups.rs` (clip rows: Fit, empty space; Volume keyframes popup; Change speed; Crop and Volume keys rows removed), new `ui/overlap_lane.rs` (timeline lane), new `ui/removal_popup.rs`, `ui/frame_handles.rs` (Ctrl or Command with an edge), `ui/media.rs` (Add media), `ui/applied_effects.rs` (Overlap rows, Crop row removed), `ui/strings.rs` | § 6 |
| Subtitle styles | `src/word_styles.rs` (enlarge), `src/ass.rs` (caller passes measured places), `src/measure.rs` (word advances; existing) | § 8.8 |

Cycle rules (ADR-140 test must stay green): `overlap.rs` imports no `app`, `ui`, `render`, `project`; `render` and `project` may import `overlap`; `word_styles` stays pure (positions come in as numbers from `ass`, which may use `measure`). The PE checks with tests/module_cycles.rs.

## 4 Project format 14 (summary)
- `Clip.overlaps: Vec<Overlap>` (serde default, not written when empty). `Overlap { id: u64, path: String (UTF-8, as picture items), kind: Still | Motion, from: Micros (offset from the clip's timeline start, >= 0), len: Micros (> 0, from + len <= clip length), place: (u16, u16) and width, height: u16 permille (the same convention as TextItem.place/width/height, drawn with Picture::rect), reshaped: bool }`.
- `Timeline.next_overlap_id: u64` (serde default 1). Overlap ids are unique in the project.
- `Clip.crop` is not written; loading format 13 reads it only to convert.
- Order rows: `overlap:<id>` in the picture group; `crop` is no longer a row name.
- Unchanged in storage: `Timeline.clip_volume` (Volume effect), `Clip.speed` (Change speed), `Timeline.clip_aspects` (REQ-326), `TextItem.picture` (picture items kept for those without a clip under their start).
- Load rules: formats 1..=14 open; 15 and newer refused (`TooNew`, REQ-029). Format 14 read by 2.2.0 and older: `TooNew(14)` as today.
- Validation (REQ-057): kind outside the two values, `len` 0, `from` outside the clip, ids repeated, permille outside 0..=1000 for place, box or size: the overlap is dropped and counted (one line); the file never crashes.
- 13 to 14 conversion (in memory, no undo step, no change mark, ADR-241): crop dropped (count of clips); each picture item whose start lies inside a clip becomes an overlap of that clip (`from` = item start minus clip start, `len` = the item's visible part inside the clip, place and box copied, kind Still, path copied), its order row `picture:<id>` becomes `overlap:<new id>` on that clip; picture items with no clip under their start stay picture items. One status line: "Opening: N clip(s) changed by the conversion (M crop(s) dropped)." (REQ-332; the picture count is part of the same line, MINOR, PM to confirm).

## 5 Overlap rendering: the three render paths
The user request asks for "the three render paths"; read here as (1) the legacy argv path (clip with the default order), (2) the step builder (`render/steps.rs`, a clip whose order was changed), (3) the preview (same plan, preview source). Export uses (1) and (2); the 360 sphere export draws no overlap (as no picture item). If Andrea meant another set, the PE stops and asks.
- Path 1: picture-like overlays after the concat, through `plan.rs picture_overlays` generalised: items and overlaps are the same `PicOverlay`-like list with timeline times; `pieces` follow the legacy windows as today. Argv for projects without overlaps unchanged (golden).
- Path 2: the overlap row is a `Step::Over(n)` input of the clip chain (picture inputs list shared with picture items); `Step::Crop` removed.
- Path 3: preview builds the same plan with `Source::Preview`; `still` is latest-only as today; overlay inputs are excluded from hardware decoding (§ 6); overlay files above 1920 x 1080 are drawn at half their box size in the preview only (§ 6, Q-C).
- Filters per overlap (numbers only in expressions, ADR-099):
  - Still (PNG): `-loop 1` input, `format=rgba`, scale to the box, `overlay=…:enable='between(t,a,b)':shortest=1`.
  - Motion (GIF, MOV): no loop, `[n:v:0]` only (no sound), `setpts=PTS-STARTPTS+a/TB`, `overlay=…:enable='between(t,a,b)':eof_action=pass`, no `shortest`: after the animation ends the overlay is transparent (REQ-328).
  - Box: default = the media's own ratio (box is kept so by the editor), `scale=w:h:force_original_aspect_ratio=decrease`. `reshaped` with a Motion MOV: Cover, `scale=w:h:force_original_aspect_ratio=increase,crop=w:h` (REQ-327). `reshaped` with GIF or PNG: Stretch, `scale=w:h`.
- Any overlap makes the export a re-encode (REQ-023, REQ-331); no stream copy.
- Missing file or file over 16 Mpx at export or preview: export stops with the message (REQ-331, REQ-287); preview skips it with the status line.

## 6 Alpha decoding
- Decoders: PNG and GIF natively (GIF transparency survives `format=rgba`); MOV with alpha: codecs known to carry alpha in ffmpeg are qtrle and ProRes 4444 (assumption; checked by fixtures in the PE stage). Other codecs, e.g. HEVC with alpha, are drawn opaque (known limit, business-overlap).
- Software decoding for every overlap input. Only the preview uses hardware decoding; `hwdec::apply` in `preview.rs` skips every input whose file is an overlap file (a file that is both a clip and an overlap is software-decoded in the preview too). Reason: hardware decoders may drop alpha (ASSUMPTION, not measured on every GPU).
- Size at add (REQ-316 for overlap files, REQ-331): PNG via `parse_png_size`; GIF via `gif_size` (logical screen size, bytes 6-9); MOV via the existing ffprobe width and height. Above 16 000 000 pixels: refused with the REQ-316 message and nothing is added. The check is repeated when the file is used (a replaced file that is too large is treated as missing).
- Memory: a 16 Mpx overlay decodes at full size; the chain scales it to the box as the first filter. Peak measured (§ 9).
- Preview size (Q-C, decided by Andrea 2026-11-22): an overlay file (PNG, GIF or video) whose frame is larger than 1920 x 1080 is drawn at half its box size in the preview graph; the export draws it at full size. The preview and the export therefore differ in detail for such files. REQ-325 and REQ-331 say the preview and the export show the same picture: the PM amends that wording for files above 1920 x 1080 (ADR-243). Files of 1920 x 1080 or less are drawn the same in both.

## 7 Edits of clips with overlaps (REQ-329)
- Pure functions in `overlap.rs`: `kept_part(overlap, window) -> Option<Overlap>` (cut to the kept part, offsets moved), `removals(edit) -> Vec<OverlapId>` (overlaps with no part in the kept part, and every overlap of a deleted clip).
- Ripple: offsets are clip-relative, so an overlap moves with its clip. Trim: start moved by the trimmed amount. Split: each part keeps the part of each overlap that lies in it (cut, no warning); an overlap wholly in the other part is removed (warning).
- Warning: `dispatch` computes `removals` before any change; if the list is not empty and not confirmed, a popup (count, OK, Cancel) holds the pending command; OK applies the command and the removal in one history step; Cancel changes nothing. One undo restores both (ADR-234).
- Overlap drag on the lane: clamped to the clip (`from >= 0`, `from + len <= clip length`), minimum length 0.1 s (ASSUMPTION).
- Several overlaps per clip may overlap in time; drawn in row order (top first on top).
- Change speed on a clip with overlaps: offsets and lengths rescaled by old speed / new speed so each overlap keeps its part of the clip (decided by Andrea, Q-B, 2026-11-22; ADR-231).

## 8 Other changes
### 8.1 Volume effect (REQ-280, REQ-282)
- Storage unchanged (`clip_volume`). The Effects list gets Volume (drag, right-click, popup with the REQ-129 keyframe timeline, straight or ease from `keyframes`). One Volume per clip is the storage rule already.
- A new Volume starts with one key at the clip's slider level (MINOR). The "Volume keys" row of Clip details is removed. The slider stays; while the effect exists it is shown disabled with the note that keys set the volume (TECHNICAL, follows REQ-280's "the slider shows").
- Old keys become the clip's Volume with the same keys (REQ-332, no data change).
### 8.2 Change speed (REQ-281)
- Storage unchanged (`Clip.speed`). An Effects list entry Change speed (50 to 200%, not keyed); applied to one clip (drag, right-click) or to all clips (double-click; one copy on each clip, REQ-147 copy rule, one undo step). Not in Change aspect, not a Clip details row. The widget row is fixed first (REQ-320). Speed is not in the stored order.
- Copy rule for a non-keyed effect is new use of REQ-147 (R8 in business); the copy is `Clip.speed` set on each clip.
### 8.3 Crop removed (REQ-283)
- Removed: `Step::Crop`, `crop_filter` use, `Row::Crop`, the Crop row of the widget, the Crop % row of Clip details, the `Clip.crop` writer. The 13 reader drops crops with the count (REQ-332). Clips without crop: argv unchanged.
### 8.4 Clip Change aspect (REQ-326)
- No storage change. The clip row offers Off, the shapes, Fit (Fit on width, Fit on height) and Empty space (colour with its chooser, or blurred with Low, Medium, High). First shape choice of a clip with no own aspect uses `FrameAspect::default()` (Fit on width, black, MINOR); an existing own aspect keeps its fit, fill and blur when the shape changes (fixes popups.rs:689 which forced Fit::Height).
- The whole-video popup (REQ-124..126) is checked in the running popup by the PE (Q1 answered "both"); defects fixed in code.
### 8.5 Add media (REQ-324)
- The Media section's button calls the same action as File > Import media… (ADR-239); no state.
### 8.6 Start screen Never removed (REQ-295)
- `SplashOption` keeps Always and OncePerVersion; `lenient_splash` maps the string "Never" to OncePerVersion (ADR-240); the settings file stays version 2; the Settings window shows two choices.
### 8.7 Overlap UI (REQ-325, REQ-327, REQ-331)
- Effects list row and Edit menu entry Overlap media: file dialog, probe, playhead rule (clip under the playhead; none: status line, nothing added).
- Timeline lane (Q-A, decided by Andrea 2026-11-22): one shared overlap lane for the whole timeline (`ui/overlap_lane.rs`). Overlapping overlaps are stacked in sub-rows; at most 3 sub-rows are visible, the rest by scroll. Each overlap shows its two ends, dragged on the lane (ADR-242).
- Preview box: drag moves (centre kept, REQ-117); edge or corner resize keeps the opposite side (REQ-145); Ctrl or Command with an edge sets `reshaped`; one drag, one undo step.
- Widget: overlap rows in the picture group, on top by default (MINOR), movable (REQ-320, REQ-321).
### 8.8 Enlarge word: bug fix (REQ-333, the fix of REQ-309; REQ-309 acceptance not met)
- Cause: the enlarged word is laid out inside its line with the other words at normal size, under `\an5` and `\pos` of the line (word_styles.rs:111-147, ass.rs line_tag). Growing one run changes the line's width and its vertical box, so the word is not enlarged around its own place; the user sees it shifted (vertical direction to be confirmed by the PE on one rendered frame; not a test run).
- Fix (ADR-235): the base event stays (the whole line, the word hidden). The enlarged event shows only the word: `{\an5\pos(cx,cy)\fs<big>}word`, cx = the line's centre minus half the measured line width plus the word's left edge plus half its advance; cy chosen so the baseline equals the normal baseline (ascent and descent from the font as the renderer uses them). Widths from `measure.rs` (the same shaping as the text fit, harfrust and skrifa, already direct dependencies). Unreadable font: the bundled font (measure's rule). 360 lines and Typewriter unchanged.
- Acceptance: centre within 2% of the picture height of its normal place; other words' event text unchanged; enlarged and base events have the same times.
- Risk: difference between measured and libass layout (kerning, fallback fonts). Fallback if over 2%: `\fscx` and `\fscy` on the word at its measured left baseline, then a `\pos` shift (PE and architect decide with the measurement).

## 9 NFR (2.3.0; GATE is a test or a check, ESTIMATE and ASSUMPTION are not gated)
- GATE default argv unchanged for projects without overlaps, Volume keys, speeds or crops (golden tests, ADR-168, ADR-224).
- GATE format 14 round trip; 13 fixture converted (counts, status line once); 13 and older refuse 14; 15 refused by 14 (TooNew).
- GATE alpha: committed fixtures (a transparent GIF, a qtrle MOV with alpha) show transparent pixels in preview and export at the same place.
- GATE REQ-328: a 1 s GIF in a 3 s overlap: animation in the first second, transparent after; PNG still for its whole length.
- GATE REQ-316 for overlaps: 16 Mpx accepted; 4001 x 4000 PNG refused; GIF and MOV sizes read as PNG.
- GATE REQ-329: removals listed once; one popup; Cancel changes nothing; one undo restores.
- GATE REQ-309 (REQ-333 in the business): centre within 2% of the picture height; other words unchanged in the event text.
- GATE Q-C: an overlay above 1920 x 1080 is half size in the preview argv and full size in the export argv; files of 1920 x 1080 or less identical in both.
- GATE lane: at most 3 sub-rows visible; more overlapping overlaps reached by scroll (UI test).
- GATE REQ-295: "Never" in a file reads as OncePerVersion; no Never in the Settings window.
- GATE UI frame <= 200 ms with the overlay lane and the removal popup open (existing gate, ADR-196).
- ESTIMATE: an overlay (1080p still) adds at most 10% to preview frame time; export with one overlay at most 1.5 times the same export without it. Measured by the PE on the reference project; not gated.
- ASSUMPTION: 16 Mpx overlay decode peak memory around 64 MB per frame at full size; the PE measures the peak with one 4000 x 4000 PNG and reports it.
- No change to start-up (QG-01) or to project load time for format 14 beyond the reader.

## 10 Security (2.3.0)
- Overlay files are untrusted media: path kept as UTF-8, never inside a filter text (inputs by index, `file:` prefix, protocol whitelist as today); numbers only in expressions.
- Size checked from a bounded header (PNG, GIF) or ffprobe output before the file is decoded; over 16 Mpx refused.
- Project 14 input: kinds, times, ids and permille values validated; damaged overlaps dropped with the count (REQ-057); the 96 MiB project cap (ADR-191) applies.
- No new crate, no unsafe, no network. Overlay files are not copied into the project or the Work folder (path only, as picture items).
- Hardware decoders never see overlay files (preview); the export does not use hardware decoding.

## 11 Tests (name the REQ in `// covers:`)
- REQ-325, 327, 328, 329, 331: overlap model unit tests (clamp, cut, split, removals, speed rescale); render argv tests (still, motion, reshape, eof_action=pass, no `-an` audio); alpha pixel tests on the committed fixtures; removal popup state tests (app level).
- REQ-330, REQ-332, format 14: fixtures `tests/fixtures/project/v13-2.2.0-*.vlproj` (volume keys, speed 150%, a crop, a picture item on a clip) and the converted expectations; TooNew checks for 13 and 15 (project.rs test at line 1105 moves to 15).
- REQ-324: the button is the last item of the Media section; the action equals the import command (UI acceptance tree, ADR-045).
- REQ-326: the clip row writes Fit and fill as the whole-video popup; the default of a new clip; undo per choice.
- REQ-280, 281, 283: Volume effect add, one key at the slider level, remove; Change speed to all clips (one copy each); Crop absent from widget and popup.
- REQ-295: lenient mapping test ("Never" file value); should_show with OncePerVersion.
- REQ-333 (the Enlarge fix, business id; code comment `// covers: REQ-333` and REQ-309 kept): enlarge positions test (measured centre); base event unchanged.
- Q-C half-size preview argv test (above and at 1920 x 1080); export argv full size.
- Lane: sub-row layout (pure function) and visible-row cap test.
- Changed tests: format 13 and 14 fixtures; picture item tests (REQ-287, REQ-316 keep, add the overlap cases); splash Never tests; render golden stays.
- Coverage: every new REQ implemented in the business gets a `// covers:` test (feature coverage 100%); line coverage >= 85%.

## 12 Stage order for the Principal Engineer (one quality run at the end)
- S0 Enlarge word fix (independent; measure.rs positions; word_styles, ass caller; tests REQ-309).
- S1 Never removal (settings lenient mapping, splash.rs, ui/settings.rs; tests REQ-295).
- S2 Format 14 and model: timeline fields, overlap.rs (pure), project.rs schema 14, project/v13.rs conversion, order rows (Row::Overlap, Crop dropped), fixtures from 13; TooNew tests moved to 15.
- S3 Volume effect and Change speed (UI and copy rule) and Crop removal (model writer, render step, rows, UI).
- S4 REQ-326 clip row Fit and empty space; whole-video popup check.
- S5 Edit rules (split, trim, delete, ripple, speed rescale) and the removal popup (REQ-329), undo grouping.
- S6 Render: picture_overlays generalised, Still and Motion, reshape, steps Over, hardware exclusion in preview, gif_size and MOV probe at add; alpha fixtures (GIF, qtrle MOV); golden unchanged.
- S7 Overlap UI: Effects and Edit entries, playhead rule, lane (Q-A), preview box with Ctrl, widget rows.
- S8 Add media button (REQ-324).
- S9 Human docs by the Documenter; the PE runs the quality suite once after S8 and reports coverage (one decimal) and feature coverage.

## 13 Questions (SEVERE, answered 2026-11-22)
All three SEVERE questions are answered by Andrea (2026-11-22); no question is open:
- Q-A answered: one shared overlap lane for the timeline; overlapping overlaps in sub-rows; at most 3 rows visible, the rest by scroll (ADR-242, § 8.7).
- Q-B answered: Change speed rescales the clip's overlap times with the speed (ADR-231, § 7).
- Q-C answered: overlay files above 1920 x 1080 are drawn at half size in the preview only; the export is full size; the difference is documented (ADR-243, § 6).

## 14 TECHNICAL decisions taken (reasons in the ADRs)
- Overlap as a clip field, clip-relative times (ADR-230, ADR-231).
- Removal list computed before the edit; one popup (ADR-234).
- Motion overlays use `eof_action=pass` for "once then transparent" (ADR-232).
- Software decoding for overlays (ADR-233).
- Enlarge fix by measured positions (ADR-235).
- Volume slider disabled with the note while keys exist (8.1).
- Mixed picture-item conversion counted in the one status line (MINOR, PM).
- Overlap lane shared, sub-rows, 3 visible rows (ADR-242, Q-A). Preview half size above 1920 x 1080 (ADR-243, Q-C).

## 15 Mismatches and items for the Product Manager
- REQ-309 is marked implemented but its acceptance "the other words do not move; the enlarged word keeps its centre" is not met by the build (the bug). The PM adds REQ-333 "Enlarge word keeps the word's place" (planned, the fix of ADR-235; REQ-309 keeps its text).
- REQ-330 gets the one status line of § 4 (PM, in parallel with this refine; wording shared with REQ-332).
- REQ-325 and REQ-331 say preview and export show the same picture: the PM amends them for overlay files above 1920 x 1080 (preview at half size, export full size, ADR-243).
- Docs: docs/architecture-subtitle-styles.md described a measured-centre event that the code never had; corrected in this development.

## 16 Risks and future work
- R1 alpha of other MOV codecs (HEVC, VP9) opaque: known limit; a later version may read them after a measurement.
- R2 test churn: format 13 fixtures, TooNew 14 to 15, picture item tests, splash Never tests, copy tests of Volume and Speed.
- R3 Enlarge: measured layout versus libass may differ above 2%; fallback in § 8.8.
- R4 Speed rescale of overlaps (decided, Q-B) and the Volume slider note (8.1, TECHNICAL) may need a later change after use.
- R8 Preview differs from export for overlay files above 1920 x 1080 (decided, Q-C): a user may see a softer preview; the export is full size.
- R5 Undo snapshots grow with overlay data (small; paths only).
- R6 Keyframes for overlays, rotation, MOV sound, separate video tracks: out of scope (business-overlap).
- R7 The whole-video copy rule for Change speed is new use of REQ-147 (R8 in business).
- Future work: a full-size preview of large overlays if the measured cost allows (would replace ADR-243); remove the picture items entirely when no old project needs them; overlay lane zoom if three visible sub-rows prove too few.
