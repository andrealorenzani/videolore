# ai-notes: plan 2.2.0 (owner: architect; STATUS BUILT 2026-11-22, not released, outcome in section 14; PLAN 2026-11-21; REFINE 2026-11-21 after Q-1 (route D, D63) and after REQ-317..323 (D64 and the order request); index in ai-docs/architecture.md; ADRs 214..229 in ai-docs/architecture-adr-2.2.0.md)

Scope: REQ-312..316 (docs/business-speed.md; REQ-312 as D63), REQ-317..323 (docs/business-applied-effects.md § Layout and the order of all effects; docs/business-text.md § Text box and size; D64), REQ-177 amended (docs/business-subtitle-accuracy.md), whole-video amendments REQ-083, 147, 148, 129, 131, 076, 200..202, 303, REQ-305 and old-project stacking accepted as built. Target version 2.2.0 (Cargo.toml reads 2.2.0). Built before: 2.1.0 (commit 281d2b2, pushed, not released; format 12; src/lut/, src/effect_order.rs, src/word_styles.rs, src/render/overlay.rs, src/ui/applied_effects.rs).

## 1. Facts found in the code (2026-11-21)
- Per-clip effects are stored per clip (`Timeline.clips`: fade flags, `scale` and `moved` keys, `clean_audio`, volume keys); by clip id: `clip_luts`, `clip_volume`, `clip_frames`, `clip_aspects`, `clip_order` (timeline.rs ~L331 `ClipOrder { clip, rows: Vec<effect_order::Row> }`, L407). Only `frame_aspect` (Change aspect) is project-wide.
- Whole-video UI routes still present (to remove, ADR-215): `FrameTarget::Whole` keyframe routes (timeline.rs `whole_ref`, `whole_edit`, `keys_of`), `Command::SelectCopy` (app/command.rs ~L482), `ApplyWholeToNew` and app/offer.rs, the mark `AppliedRow.whole` (app/applied.rs L28; ui/applied_effects.rs L91), dashed handles (ui/frame_handles.rs), the title "— whole video" for Scale and Move (app/effects.rs L128, L139; kept for Change aspect).
- effect_order.rs: `Row` = Lut, Text(u64), Picture(u64), Subtitles, Unknown (name parse/print at L12-60); `Present` (what a clip has); `legacy(p)` (texts reversed, subtitles, pictures reversed, LUT at the bottom); `resolve(stored, p)` (stored rows the clip has, then the rest on top in legacy order); `to_store` (nothing when the order is the legacy one); `is_changed`; `moved` and `dragged` inside the list.
- Render (render/graph.rs): `compose_frame` (L138: background or blurred copy (stage B), Scale and Move overlay (stage C), then the overlay with the layer); `lut_chain` (L94) in the clip chain; `fades` (L77); `overlay_chain` (L807) runs the 2.1.0 overlay steps (`OverlayStep::Lut`, `Ass`, `Pic`, render/overlay.rs L19) for clips whose order differs from the legacy one; `crop_filter` (L865). Audio: Clean audio before Volume (2.1.0 rule, ADR-080).
- Text size: text/layout.rs `layout(item, width, height)` = best fit between MIN_FIT and MAX_FIT (10% of height; `text.rs` L42-44 from measure::MIN_SIZE/MAX_SIZE); `layout_in(text, font, box_w, box_h, min, max)` (L129); measure.rs `fit(box_w, box_h, min_px, max_px) -> Fit` (L198, wrap at box width, lines fit by height); text/events.rs slide-in (L83-88) measures from the box width or height.
- Preview layout: ui/mod.rs `preview_panel` (L593-612): the Applied effects take `applied_effects::height(available)` below the picture (ui/applied_effects.rs L23: share, min height); the picture gets `available - controls - applied - gap`. ui/layout.rs: `Fractions`, `Mins` (L19: media, effects, preview, timeline, row), `sections`, `drag`, `effective` (minimums); settings.rs `SavedLayout` (lenient_layout L175, `Option<SavedLayout>` L342), settings version 2 with additive fields (ADR-044/195).
- Output check, blur, Faster, picture header, speech regions: as in the earlier plan (sections 2 and 3.4 to 3.7).

## 2. Spike results (history and new spikes)
- History (route D chosen; the alternatives are the record): S0a `.m3d` is text in ffmpeg 9.0.2 (`in`, `out`, `values 0 1 2`, red outermost); 256-point starts: `.cube` 12.4-13.0 s, `.m3d` float 5.1 s, `.m3d` integer 3.4-3.6 s; S0a-2 HALD via `haldclut` 0.36 s (trilinear, 16-bit planar path unverified). S0b blur proxy: +3.2 ms per frame at 1080p, +1.8 ms at 4K (no decode). S0d REQ-177 prototype results (ai-docs/code.md, 2.0.3): 21-22 of 23 for 4 of 120 settings, overfit.
- New spikes for the order and text (PE, scratch, before the stage that needs them):
  - S0e pairs of the order (real ffmpeg, bundled 9.0.2, small frames): (1) Crop before and after Scale and Frame; (2) Scale then Move and Move then Scale on a picture with an RGBA layer; (3) LUT above and below Frame (the background graded or not) and LUT with alpha (gbrap or alpha re-attached); (4) Fade above and below LUT and Frame (black stays black only if the LUT maps black to black); (5) Volume and Clean audio in both orders, including the dry branch of the Clean audio mix. Result: which formulation gives the expected output, and the argv of the default is untouched. Decides ADR-224 and ADR-225 details.
  - S0f text size: check that measure::fit with the cap as the only change reproduces the 2.0.4 example's frame (REQ-319) and that the block is centred by the built layout (if not, the centring is added); check the slide distances (text/events.rs L83-88) against REQ-318 (room used).
  - S0g layout: check ui/layout `drag` and minimums for a vertical splitter inside the Preview section at 800 x 500 and 1280 x 800 (no code change expected).
- S0c (Faster phases) and S0b (blur in the real preview) are still PE measurements (plan § 10).

## 3. Approach by requirement
### 3.1 Format 13 (REQ-322, REQ-323, REQ-029) — ADR-222
- SCHEMA_VERSION 13; loads 1..=13; always written as 13. 2.1.0 and older refuse 13 (TooNew, file untouched). 12 loads with the 2.1.0 meaning of `clip_order` and no maximum. New fields with serde defaults: `TextItem.max_size` (Option<u16>, permille, skipped when absent). Row names: crop, frame, scale, move, fade, volume, clean (added to the 2.1.0 names).
- Validation on load: row names by `Row::parse` (unknown -> Row::Unknown, repaired with the 2.1.0 line); `max_size` 20..=500 else dropped with a note; the migration 12 -> 13 is in memory only.

### 3.2 Order of all rows (REQ-320, REQ-321, REQ-322) — ADR-223, ADR-229
- Groups: picture group (Crop, Change aspect (`frame`), Apply LUT, texts, picture items, subtitle lines, Fade IN/OUT, Scale, Move) and sound group (Volume, Clean audio); Speed first and fixed (never stored).
- Default: the 2.1.0 build order written as the default sequence (the PE writes it into the golden test list from the code; it must reproduce the 2.1.0 argv). Empty stored list = default (no argv change).
- Stored: the full order of the two groups, written when a clip's order differs from the default (as 2.1.0 `to_store`).
- 2.1.0 expansion: stored movable rows (2.1.0 names) go on top of the default sequence of the other rows, in their stored order; the look of a 2.1.0 project with a changed order is kept.
- New rows and copies: top of their group (REQ-321); a move only inside its group; undo label "Move <row> up/down" and "Move effect" for a drag (REQ-321).
- `order_chain.rs` (pure) owns: groups, default sequences, expansion, resolve, move and drag inside a group, new row placement, repair.

### 3.3 Render with a free order (REQ-320, REQ-321, REQ-131 parity) — ADR-224, ADR-225
- A clip with the default order keeps the 2.1.0 legacy path: its argv does not change (golden gates).
- Other orders: step builder `render/steps.rs` (generalises the 2.1.0 overlay stage). Picture layer = RGBA stream of the source at its current size. Steps: Crop (percent of the current size, even pixels), Frame (fit into the project frame, background composed under the layer), Scale (about the picture centre), Move (overlay offset in the frame), Fade (black alpha ramp), Apply LUT (grades the stream it sits on; alpha kept), texts, picture items and subtitle lines (2.1.0 overlay steps). Preview and export use the same builder.
- Pairs and formulations (S0e decides the exact filters): Crop/Frame and Crop/Scale use one percent-based crop; Scale/Move need the RGBA layer (their results differ by order); LUT above Frame grades the whole frame, below Frame the layer only; Fade above Frame fades the background too, below Frame only the layer; Fade above LUT is graded as the LUT maps black.
- Sound: the sound group in its order (default Clean audio then Volume, as 2.1.0); the Clean audio dry branch takes its input at the filter's point in the chain; audio fades stay fixed after the sound group (derived from REQ-320, [TECHNICAL], confirm at the checkpoint); music keeps its chain; Speed applied first to both (setpts, atempo).
- Preview and export share the builder (REQ-131).

### 3.4 Applied effects widget and rows (REQ-313, REQ-320, REQ-321) — ADR-216, ADR-223
- Two groups and one fixed row in one list (the business order), move up and down inside a group, keyboard and drag inside the group; rows cached by revision (ADR-216, the revision bumped by every order, effect, text, picture, LUT or subtitle command).

### 3.5 Text size (REQ-318, REQ-319, REQ-323) — ADR-227, ADR-228
- The 2.0.3 best fit (measure::fit via text/layout.rs) with the cap = `max_size` or 100 permille; the block is centred in the box on both axes (assumption; the PE confirms the built centring in S0f); the ratio is computed at the current wrap, not stored.
- The text popup: one optional "Maximum size" field (2 to 50, one decimal, empty = default cap), refused with the business message when out of range; one undo step per change; the Add Text popup has the same field.
- Slide-in: distance from the box width or height (as built); a larger box gives a longer slide.
- Old texts: no maximum, cap 10%, sizes unchanged (REQ-319 acceptance on the 2.0.4 example).
- Subtitle lines: untouched (style size).

### 3.6 Applied effects column (REQ-317) — ADR-226
- The Preview section: picture and controls on the left, the column on the right at full section height; column width max(180 px, share x section width), share default 15%, saved as the additive field `applied_share` (permille, default 150, 80..300) in SavedLayout (settings version 2 kept); the vertical border is a splitter (ui/layout.rs rules: minimums, drag, window resizes keep proportions); picture shrinks first; controls under the picture only; popups not placed in the column; the column scrolls.

### 3.7 Whole-video removal (REQ-083, 147, 148, 129, 131, 076, 200..202, 303) — ADR-215
- As before: remove the whole-video popup and routes, the mark, the offer, SelectCopy and dashed handles; keep the copy routine and Change aspect. The copies go to the top of their group on each clip (ADR-223).

### 3.8 LUT preparation (REQ-312, D63) — ADR-214 (route D)
- Background preparation in the 2.1.0 store (read with progress, parse, Work copy `.cube` written once with progress); "Preparing LUT ... N %" at least once a second (during the ffmpeg start a time-based estimate, capped at 95%); the graph builder uses `peek` and omits the LUT row while not Ready (unprocessed picture); newer choice replaces the preparation; export waits in its worker. No change of route, colours or argv.

### 3.9 Picture limit (REQ-316) — ADR-219; 3.10 blur (REQ-314) — ADR-217; 3.11 Faster (REQ-315) — ADR-218; 3.12 REQ-177 — ADR-220 (unchanged: pure speech_regions.rs, frozen grid of 81 settings, second clip check).

## 4. Components (2.2.0 changes)
| Part | Where | Change |
|---|---|---|
| Project format | src/project.rs (+ project module) | format 13; 12 migration in memory; `TextItem.max_size`; validation |
| Order model | src/order_chain.rs (new, pure); src/effect_order.rs (2.1.0 names kept; extended rows) | groups, default sequences, expansion, resolve, moves; replaces the picture-only parts of effect_order |
| Timeline | src/timeline.rs (`ClipOrder`, copies) | full order rows; copies on top of group |
| Render steps | src/render/steps.rs (new; generalises overlay.rs), src/render/graph.rs (legacy path unchanged), src/render/overlay.rs | step builder for non-default orders; sound group chain |
| Text size | src/text/layout.rs, src/measure.rs (unchanged API), src/text/events.rs (slide unchanged) | cap = max_size or default |
| Text popup | src/ui/text_popup.rs (field), src/app/texts.rs (command) | Maximum size field, validation, undo |
| Applied widget | src/ui/applied_effects.rs, src/app/applied.rs | two groups, all rows, cache |
| Preview layout | src/ui/mod.rs (preview_panel), src/ui/layout.rs (splitter), src/settings.rs (SavedLayout `applied_share`) | column beside the picture |
| Whole video | (ADR-215 list) | removals |
| LUT store | src/lut/store.rs, app/lut.rs, tick (ADR-214) | preparation, status, peek |
| Picture | src/picture.rs | 16 Mpx |
| Speech regions | src/speech_regions.rs (new, pure), src/align.rs | REQ-177 |
Unchanged: scheduler, preview engine, export engine, ffmpeg process model.

## 5. Data flow changes
- Save: order of a changed clip (full picture and sound groups) and text maximum -> format 13.
- Load: 12 or 13 -> expansion of 2.1.0 orders -> resolved order per clip; damaged entries repaired with the note.
- Build: per clip, default order -> legacy path; else steps (picture layer, frame, fades, LUT, overlays; sound chain) -> filter text, same in preview and export.
- Widget: commands -> revision -> rows rebuilt once -> two groups shown; move inside group -> one undo step.
- Text: box and cap -> measured fit -> block centred -> ASS events; subtitle lines unchanged.
- Layout: SavedLayout applied_share -> column width -> picture fit rectangle.

## 6. Module-cycle rules (tests/module_cycles.rs, ADR-140)
- order_chain: timeline ids and effect_order names only (std, serde); no render, app, ui.
- render/steps: render helpers, order_chain, lut, overlay types; no app, ui.
- project (format 13): no app or ui.
- ui/applied_effects and the column: ui and app commands (as other ui modules); the splitter in ui/layout only.
- speech_regions: std and time only; align imports speech_regions; not the reverse.
- lut store: base, jobs, lut::parse (as 2.1.0); the graph builder reads through peek only.
- app/offer.rs removed.

## 7. NFR and performance budgets (2.2.0)
Marked GATE, ASSUMPTION or MEASURED (as before).
- Format: 2.1.0 files open with the same look and export (GATE on fixtures: formats 4, 5, 12 and the 2.1.0 example); format 13 round trip (GATE); 2.1.0 refuses 13 (documented; the 2.1.0 test is run once with a 13 fixture where possible: not gated on 2.1.0 since it is not in this tree).
- Order: default argv byte-identical (GATE golden); a non-default clip: the step builder adds about one filter per row (ASSUMPTION: at most 10% more time per frame for a clip with six extra steps; MEASURED by the PE); each swap pair differs where expected and preview equals export (GATE).
- Sound: default chain unchanged (GATE); swapped Volume/Clean audio differ (GATE).
- Widget: 0 row builds over 60 frames without a change; one move = one undo step; rows in two groups (GATE, AccessKit).
- Text: a text with maximum 20 in a box fitting 15% is drawn at 15% in preview and export (GATE); the 2.0.4 example project exports the same frame (GATE, REQ-319); the popup has no size field besides the maximum (GATE).
- Layout: column 15% of the section width, min 180 px at minimum window (GATE at 1280 x 800 and at the minimum); the saved share is kept after restart (GATE).
- REQ-312 (route D): progress at least once a second (GATE); UI response <= 200 ms while preparing (GATE); playback first frame <= 200 ms with fallback (GATE).
- REQ-313, REQ-314, REQ-315, REQ-316, REQ-177: as ADR-216..220 (unchanged).
- Coverage: line >= 85%; feature coverage 100% for REQ-312..323 and the amended REQs.

## 8. Security model (2.2.0)
- Format 13: every new field validated on load (row names strictly parsed, `max_size` range, applied_share range in settings); damaged entries dropped with a note; no new unbounded read (project cap 96 MiB unchanged).
- Step builder: no user text in any filter (LUT copy name by hash, ASS file names fixed by code, numbers from validated values only); crop and scale values are numbers from the model.
- Layout share: range-checked, no effect on paths.
- LUT copy, picture header limit, REQ-177 fixture: as in the plan's earlier sections (unchanged).
- Unsafe: none added; no new crate.

## 9. Test strategy
- Pure: order_chain (default sequences; expansion of 2.1.0 orders with the 2.1.0 fixtures; resolve; move and drag inside a group; new rows and copies on top; unknown names repaired; Speed never stored); text size (cap, centring, max validation, old texts); column width function.
- Golden: default argv for formats 4, 5, 12 and the 2.1.0 example: byte-identical; the default sequence written as a test list.
- Real ffmpeg (VIDEOLORE_REQUIRE_FFMPEG=1): each swap pair of S0e: output differs where expected, preview equals export within 2 levels; Volume/Clean audio swap changes the sound.
- Project: format 13 round trip; 12 loads with defaults; 13 with damaged order repaired; `max_size` out of range dropped.
- Widget: AccessKit rows and groups; move up/down inside group; undo labels; no move across groups; drag inside group; row build counter.
- Layout: column at 1280 x 800 and at minimum; splitter drag; picture shrinks first; settings field kept.
- Whole video, LUT preparation, picture limit, REQ-177, blur and Faster: as in ADR-214..220 (unchanged).

## 10. Stage order for the Principal Engineer (each stage: fmt, clippy -D warnings, tests; coverage once at the end; a checkpoint report to the coordinator after each stage)
- S0 spikes (scratch, not committed): S0e (order pairs with real ffmpeg, LUT alpha, Clean audio dry branch), S0f (text fit, centring, slide, on the 2.0.4 example), S0g (layout splitter at 800 x 500 and 1280 x 800), then S0b (blur in the real preview) and S0c (Faster phases on the Iris). Each result goes to the architect in a short note; ADRs amended if a result changes a choice.
- S1 whole-video removal (ADR-215): routes, mark, offer, SelectCopy, dashed handles, titles; copies kept; replacement tests. Gate: REQ-083, 147, 148, 129, 131, 076, 200..203, 303 tests.
- S2 picture limit (ADR-219) and LUT preparation (ADR-214, route D): independent, small. Gate: REQ-316; REQ-312 (progress, responsiveness, fallback, colours within 2 levels).
- S3 format 13 and text size: project format 13 (ADR-222), `max_size` (ADR-228), layout cap and popup field (ADR-227). Gate: format fixtures (12 loads, 13 round trip, damaged repair), REQ-318, REQ-319 (2.0.4 example export unchanged), REQ-323 (validation, undo).
- S4 order model and render steps: order_chain.rs (ADR-223, ADR-229), default sequences from the 2.1.0 code written into the golden list, expansion of 2.1.0 orders, render/steps.rs (ADR-224), sound chain (ADR-225). Gate: golden argv unchanged; pair tests from S0e; expansion keeps 2.1.0 look.
- S5 widget: two groups and all rows, move inside group, undo labels, cache by revision (ADR-216), copies on top of group. Gate: REQ-320, REQ-321, REQ-313; AccessKit tests.
- S6 Applied column layout (ADR-226): column, splitter, saved share, picture shrink. Gate: REQ-317 acceptance at 1280 x 800 and minimum; restart keeps width.
- S7 measurements and gates for speed: blur (ADR-217: change only if S0b misses), Faster (ADR-218 levers until S0c target or report). Gate: REQ-314, REQ-315 (b) and fault tests.
- S8 REQ-177 (ADR-220): ask Andrea for the second clip (about 30 s, real speech, transcript; committed with sha-256) at this stage; then the one check with the frozen grid selection; enable the strict test; if it fails, REQ-177 stays planned.
- S9 gates and docs: QG-22 (LUT preparation), QG-23 (rows), QG-24 (picture limit), QG-25 (format 13 and 2.1.0 compatibility), QG-26 (order swap pairs), QG-27 (text size and column), QG-28 (blur and Faster measurements), QG-29 (REQ-177 check); architect and documenter update docs; version 2.2.0 already in Cargo.toml (scripts/version.sh check).

## 11. Questions and decisions
- No open question. Decided by the architect (recorded in ADRs): format 13 (ADR-222; the rule for 2.1.0 files: expansion in memory, no loss); the default order is the 2.1.0 build; the sound fades stay fixed after the sound group ([TECHNICAL], ADR-225); the column share is an additive settings field (ADR-226); the text cap is the per-text maximum in permille (ADR-228).
- Earlier decisions kept: route D (D63), 16 Mpx, whole-video copies and no mark, REQ-177 grid and check.

## 12. Risks (2.2.0)
- Format 13 is refused by 2.1.0 builds (not released): developers who test 2.1.0 must update; the release notes say it.
- Pair formulations (Scale/Move, LUT alpha, Clean audio dry branch) may need more filter work than estimated; S0e decides before S4.
- Non-default orders add filter steps: time per frame to be measured (ASSUMPTION 10% for six extra steps).
- LUT placed below Frame grades only the layer; LUT above Frame grades the fill: the look differs by order (user choice, REQ-320).
- Text centring and slide distance may differ from the business wording (S0f).
- Column width at the minimum window size takes 180 px of the Preview section: the picture gets less room (business accepted).
- The percentage during the ffmpeg start is time-based (estimate).
- Blur and Faster: as before (measured first; REQ-315 may stay open).
- REQ-177: may fail on the second clip (then planned).

## 13. Future work (postponed)
- Per-clip order of groups across clips (paste of an order): not in 2.2.0.
- Binary LUT route (HALD) or integer `.m3d` route: only if a shorter start is asked later (route D kept).
- Blur resolution change (picture change) only with a product decision if S0b misses.
- Faster hardware-decode check (ADR-218 c) only with a note and a test.
- Scale changes in the preview cost (roadmap).
- Multi-clip remove of copies (roadmap candidate).
- 360 whole-sphere export ignores the order (documented limitation; sphere-fixed rows, ADR-207).
- Audio fades movable (not in 2.2.0; fixed after the sound group).
- Work copy LRU if several large LUTs fill the disk.
- REQ-305 stacking for old projects accepted as built (no work).

## 14. Built state (2026-11-22; the PE's record is docs/code-2.2.0.md, the checkpoint section "Principal Engineer 2.2.0")
- Built: REQ-312 (route D: background preparation with progress, unprocessed picture meanwhile; start about 12 s, not gated), REQ-313 (rows cache by serial), REQ-314 (blur measured about 2 ms a frame at 1080p and 4K, target met, no change), REQ-316 (16 Mpx), REQ-317 (column beside the picture, saved share), REQ-318 and REQ-319 (box governs the size; only the cap changed; 2.0.4 sizes pinned by a golden test), REQ-320 to REQ-322 (order of the picture and sound groups; format 13), REQ-323 (per-text maximum), whole-video amendments (copies; no popup, mark or offer).
- Format 13 (ADR-222): built; 2.1.0 refuses it; 12 loads with the 2.1.0 meaning of the order.
- Three render builds (ADR-224): the default legacy path (argv unchanged, golden tests), the 2.1.0 overlay form (movable rows above the default sequence), and the step builder render/steps.rs (picture layer RGBA; text as a black and white matte with alphamerge, because libass leaves an RGBA frame's alpha as it finds it).
- Deviation from REQ-321 "new rows on top": a new Crop, Change aspect, Scale, Move, Fade, Volume or Clean audio keeps its default place (a Scale on top of Change aspect would scale the blurred background; a Crop on top of a text would cut it; the sound group is the same, user-decided). A new text, picture or LUT still goes on top of the other movable rows. Reported to the Product Manager.
- Column (ADR-226): the 180 px minimum wins at 1280 x 800 (about 20%, not 15%: 15% governs from a 1200 px section). Wording to be adjusted by the Product Manager.
- NOT MET, measured: REQ-315 Faster on integrated graphics: 8.7% lower than Best quality (target 20%; lever (a) only, the strict decode and the three samples run at the same time; levers (b) 0.1 s and (c) hardware decode, a weaker check, not taken). The remaining time is the hardware encode phase (ADR-218).
- NOT MET, measured: REQ-177 speech regions: at most 10 of 23 matched words within 0.3 s (43%; target 90%, eligibility 75%) over the 81 settings. FROZEN = None; the recognised times stay the default path; a pace-aware method is needed (roadmap). ADR-220 status NOT MET.
- Module cycles: the 2.2.0 PE section does not record a result for tests/module_cycles.rs; to be confirmed at the PE's VERIFY step (last recorded green in earlier checkpoints: 7 tests).
- Not built: the 360 whole-sphere export ignores the order (as REQ-321 says); a text below Change aspect is drawn by a matte (costs more); audio fades keep their place after the sound group (ADR-225); REQ-315 levers (b) and (c); REQ-177 until a new method.
- Open decisions for Andrea: (1) whether REQ-315 stays open or the target is restated (the remaining time is in the hardware encode phase); (2) whether REQ-177 is kept planned or the method is changed (pace-aware or a second method); both are product decisions and are not taken by the architect.
- Release: the plans are archived at /release (the command moves them into the snapshot; this file is not moved by the architect).
