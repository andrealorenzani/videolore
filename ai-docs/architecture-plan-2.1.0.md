# ai-notes: plan 2.1.0 (owner: architect; PLAN 2026-11-21, REFINE 2026-11-21 after Andrea's answers; index in ai-docs/architecture.md)

Scope: REQ-300..311 (docs/business-lut.md § 2.1.0, docs/business-applied-effects.md, docs/business-subtitle-styles.md). Target version 2.1.0. ADRs 202..213 in ai-docs/architecture-adr-2.1.0.md. Human docs: docs/architecture-2.1.0-overview.md, docs/architecture-applied-effects.md, docs/architecture-subtitle-styles.md, plus updates to architecture-lut, -projects, -text, -structure, -security.

Decisions by Andrea (REFINE 2026-11-21): Q-1 = B (LUT edge up to 256, file cap 512 MiB, load off the UI thread with progress); Q-2 = A (the whole frame is graded; the movable LUT row is applied after the frame composition and the fades); Q-3 = A (sphere-fixed texts and lines on 360 clips are fixed rows, not movable); REQ-307 kept as written (the ASS export writes the shadow as the style's Shadow and the colour as its BackColour, boxes included; the design below meets it, with a fallback named in ADR-211).

## 1. Why the 80 MB LUT was refused (measured by reading the code, 2026-11-21)
- src/lut.rs `MAX_3D = 65`: a 144-point cube fails with "size 144 is outside 2..=65".
- src/lut.rs `MAX_BYTES = 16 MiB`: an 80 MB file fails first ("larger than 16 MiB"), via `base::io::read_limited` in `lut::read`, shown by `app/lut.rs set_lut_file`.
- Even when parsed: `Lut::new` formats one text line per entry (about 3 M format! calls at 144^3) and keeps the full text (~80 MB) plus the f32 table; `to_cube` clones the text per graph build.
- Project copies (project.rs `lut_copies`): copies above `MAX_LUT_COPY` (20 MB) are skipped silently and keep only the path (REQ-301 says this is intended); total `MAX_LUT_COPIES` 64 MiB.
- Work folder: one text file per clip (`lut-clip-<i>.cube`, graph.rs `lut_chain`): 10 clips with one 80 MB LUT write 800 MB.
- ffmpeg is not the cause: vf_lut3d.c accepts 3D sizes up to MAX_LEVEL 256 (lut3d.h, FFmpeg master, read 2026-11-21; bundled 9.0.2 assumed the same, checked in spike S0a). ffmpeg's own table is float: 256^3 x 12 bytes = 201 MB per ffmpeg process.

## 2. LUT approach (Q-1 = B)
- Edge 2..=256 for 3D (1D 2..=4096 unchanged). A 256-point `.cube` is about 450 MB of text (16.8 M entries, about 27 bytes each): the file cap is 512 MiB, read streamed (no whole-file buffer). Line cap 256 bytes; exactly n^3 entries; no extra numeric line. `.3dl` and `.csp` follow the same edge and file rules.
- Pure streaming parser (lut/parse.rs): one pass over a BufRead (1 MiB buffer), std `f32` parse on tokens, no per-line allocations, no cube text in memory.
- Table storage u16 per channel (ADR-203 amended): 6 bytes per entry, so 256^3 = 100.7 MB, 144^3 = 17.9 MB. Quantisation 1/65535 is far below the two-level tolerance. Chosen to keep the 256 case within a sane budget; f32 would be 201 MB.
- Cache by bytes (ADR-205): budget 192 MB of tables, LRU; a table in use by the open project is evicted only when over budget. Copies from projects (<= 20 MB text each, 64 MiB total) are parsed to tables inside the same budget.
- Loading on a Background job (jobs module), with progress: the job sends `LoadProgress { read, total }` (bytes read of the file, then entries written of the Work copy) over the existing mpsc channel; the UI tick shows "Reading the LUT file... N %" in the status line. Cancel: a generation token checked every 1 MiB; a newer choice or closing the project stops the job and its partial Work copy (temp file removed). The UI never parses (GATE UI frame <= 200 ms while loading).
- Work copy named `lut-<hash16>.cube` (hash of the normalised text, SHA-256 prefix, streamed) in the private Work folder, written once per distinct content and strength by a streamed atomic write (temp + rename, base::io). Shared by all clips, preview and export; removed with the Work folder. Replaces `lut-whole.cube` and `lut-clip-<i>.cube`. At 256^3 it is about 450 MB on disk per distinct LUT.
- Project: LUT above 20 MB of normalised text is path only (REQ-301, option C); copy rule of REQ-293 unchanged below the limit; missing-file states use REQ-292 for path-only LUTs.
- Rejected: pass the user path to ffmpeg (escaping, TOCTOU, user text in a filter); symlink or hard link to the user file (cross-device, file may change); copy of the raw bytes (not the validated content); keep f32 at 256 (201 MB table); a parsing crate (std parse first; S2 decides).
- Budgets (ESTIMATES, measured in spike S0a, stages S2 and S4; gates after measurement): parse 144^3 <= 2 s and 256^3 <= 6 s; Work copy write 256^3 <= 3 s; load peak extra memory <= table + 2 MiB (100 MB at 256); ffmpeg process with a 256 LUT: about 200 MB float table plus the start-up parse of 450 MB text (measured S0a; if the start-up exceeds 2 s the preview is a risk, see section 11).

## 3. Applied effects (order), Q-2 = A and Q-3 = A
- Movable rows per clip: Lut, Text(id) (flat text items), Picture(id) (picture items), Subtitles (one row for the clip's subtitle lines). Fixed rows: Crop, Scale, Move, Change aspect, Fade IN/OUT, Volume, Clean audio, Speed. On a 360 clip the sphere-fixed texts and sphere-fixed subtitle lines are listed as fixed rows (not movable), drawn as in 2.0.4 (Q-3 = A).
- Legacy stacking (format <= 11, not stored; verified by reading the code), bottom to top: Lut (in the clip chain, under the texts), picture items (overlay after concat, before the ASS document), Subtitles (lines first in the flat ASS document), flat texts in item order (later items on top; `text::events::project_events` iterates `texts.items` and ASS draws same-layer events in file order). The PM's "later items on top" is correct.
- Stored order: `Timeline.clip_order` (format 12, sorted by clip id, empty entries dropped), movable rows top-first (the top row is drawn last). Rows not on the clip are not stored; a new row goes on top. Unknown ids dropped on load with a repair note.
- Graph (Q-2 = A): clips with a changed order are built by the overlay stage (render/overlay.rs): fixed chain first (crop, scale, move, frame composition, fades), then the movable rows bottom to top, one filter step each: Lut = lut3d on the shared Work copy (the whole frame is graded, bars and background included); Text and Subtitles = subtitles= on `ov-<clip>.ass` (clip-local times, that row's events only); Picture = overlay with an enable window (split label for reuse).
- Clips whose order equals the legacy order keep the legacy graph (argv identical).

## 4. Subtitle and text styles
- Shadow (REQ-306, REQ-307, wording unchanged): style Shadow = px * H / 1080 (ScaledBorderAndShadow yes, PlayResY = picture height) and BackColour = shadow colour, for every style including boxes. Box fill: BorderStyle 3 needs the box colour in a field other than BackColour. Design A (primary, to verify in spike S0d): libass draws the box fill from OutlineColour and the box shadow from BackColour; then the box colour moves to OutlineColour (today Boxed and TextBox set both to the same &H80000000, so the look is unchanged). Design B (fallback if S0d shows libass draws the box from BackColour): the box is drawn as a vector rectangle event (\p1) in the box colour, behind the text and in front of the shadow event, sized from the in-house measure; the style keeps Shadow and BackColour exactly as REQ-307 says. Risk: the rectangle must match the 2.0.4 box geometry at 1080 px (measured). Shadow 0 writes the 2.0.4 style values, so the output is unchanged.
- One style per distinct (box, shadow colour, box colour) combination for text items; subtitles use one project style.
- Subtitle colours (REQ-308): Primary = Highlighted, Secondary = Not yet highlighted (existing \k mapping; defaults white and yellow).
- Typewriter (REQ-310): one event per word: spoken words in the Not-yet colour (inline \1c), future words hidden (\alpha&HFF&), the current word with \k letter tags over a transparent Secondary (each letter switches to Highlighted at its time). Event count = words.
- Enlarge word (REQ-309): base line event per word time with the active word hidden (\alpha&HFF&, layout kept) plus one event for the enlarged word at its measured centre (\fs = size + N, \pos from the in-house layout). Accuracy depends on libass versus the in-house measure (spike S0b); fallback decided by the PE with the architect if it misses 2 px at 1080.
- Pure module src/word_styles.rs (plans; no ass import); src/ass.rs writes the dialogue lines; src/text/events.rs gets the shadow styles.

## 5. Format 12
- SCHEMA_VERSION 12, loads 1..=12, always written as 12. 2.0.4 and older refuse it (TooNew, file untouched, REQ-029).
- Additive fields with serde defaults: Timeline.clip_order; TextItem.shadow (px at 1080, colour; default 0 and black); subtitle style fields (shadow px and colour, highlighted and not-yet colours, enlarge px default 12); Preset variants EnlargeWord and Typewriter.
- Old projects: legacy order, no shadow, default colours, no new style (look unchanged).
- Validation: shadow 0..=20 px; enlarge 1..=40 px; colours #RRGGBB; order ids checked on load; damaged entries dropped with a line in the repair notice.

## 6. Components
- src/lut.rs -> src/lut/mod.rs (API), src/lut/parse.rs (pure streamed reader), src/lut/store.rs (states, LRU by bytes, job, progress, cancel, Work copy writer). Store uses base::io, base::private, jobs. Never render, app or ui.
- src/effect_order.rs (new, pure): rows, stored order, legacy order, sphere-fixed rows (fixed, not movable), move up and down, repair.
- src/word_styles.rs (new, pure): Highlight, Enlarge, Typewriter plans.
- src/render/overlay.rs (new): per-clip overlay stage.
- src/ui/applied_effects.rs (new): the widget (about 15% of the Preview section; movable rows, fixed rows, scroll, drag and Move up and down, keyboard).
- Changed: text/events.rs, ass.rs, subtitles.rs, project.rs (format 12), app/lut.rs (store states, progress, non-blocking choose), app (MoveRow command, one undo step).
- Unchanged: ffmpeg process model, scheduler, preview engine, export engine.

## 7. Module-cycle rules (tests/module_cycles.rs)
- lut::parse: std and thiserror only; lut::store: base, jobs, lut::parse; neither imports render, app or ui.
- effect_order: timeline and text ids only.
- word_styles: subtitles and base::pos only (no ass, render, app, ui).
- render::overlay: render helpers, effect_order, word_styles, lut; no app or ui.
- ui::applied_effects: ui and app commands, as other ui modules.

## 8. NFR and security
- See ai-docs/architecture-nfr.md § 2.1.0 and ai-docs/architecture-security.md § 2.1.0.

## 9. Test strategy
- Parser: 144^3 and 256^3 accepted; 257 refused with the message; line over 256 bytes refused; extra and missing entries refused; NaN and infinity refused; domain range; random bytes never panic; time gate best of 5 (base::testing) at 144^3 (QG-19); 256^3 time measured, gated after S0a.
- Store: an 80 MB and a 450 MB file chosen; project file not grown; states Loading (with progress), Ready; cancel removes the partial Work copy; UI frame gate while loading (QG-19); LRU by bytes (192 MB) with a 256 table; missing file gives REQ-292 choices; Find relinks; a file changed during the read is refused.
- Work copy: hash name; one file for several clips; real ffmpeg channel swap on a 144 LUT (exact colours); export vs preview within 2 levels (REQ-300).
- Order: widget tests through AccessKit (ADR-045 style), undo label "Move LUT down", one undo; sphere-fixed rows on a 360 clip are fixed (not movable); graph: overlap A above B (REQ-305) in preview and export; legacy argv byte-identical (golden formats 4 and 5; LUT name change only where LUTs are used).
- Shadow: 8 px red at 1080 within 2% of the frame in the export (real ffmpeg frame); 0 px output equals 2.0.4; boxed shadow behind the box and readable (box 50% darker than the frame); spike S0d result tested (box colour field).
- Styles: highlight colours in preview, export and ASS; Enlarge size within 2%, other words not moved; Typewriter 1 s / 5 letters: about 2 letters at 0.4 s, all at the end; export equals preview at three times.
- Format: 11 fixtures load with defaults; 12 round trip; damaged order dropped with notice; 13 refused.
- Coverage: each implemented REQ-300..311 has a `// covers:` test; line coverage >= 85%.

## 10. Stage order for the Principal Engineer
- S0 spikes (scratch, not committed; results to the architect in a short note; ADRs amended if changed): S0a ffmpeg lut3d start time and memory with a 144 and a 256 cube text on the bundled 9.0.2 (and whether binary .m3d is cheaper: read parse_m3d in the bundled source first); S0b libass placement of an enlarged word against the in-house layout (2 px at 1080); S0c \k instant reveal and hidden-letter alpha (libass); S0d BorderStyle 3: does libass draw the box fill from OutlineColour and the box shadow from BackColour (design A), or from BackColour (design B)?
- S1 format 12 and the order model: SCHEMA 12, effect_order.rs (sphere-fixed rows), shadow and style fields with defaults, migration tests, repair notice. Gate: all 2.0.4 fixtures load and export unchanged.
- S2 LUT reader: lut/parse.rs streamed, limits 256 / 512 MiB / 256 bytes, u16 table, .3dl and .csp limits, tests, time gates. Gate: parser tests; parse budgets measured.
- S3 LUT store: store.rs with LRU by bytes (192 MB), Background job, progress, cancel, path-only projects (> 20 MB), missing states, non-blocking choose. Gate: REQ-301, REQ-292 path-only; UI frame gate during load.
- S4 Work copy and graph names: lut-<hash16>.cube written once with progress, shared by clips, preview and export; golden check; real ffmpeg 144 and 256 tests. Gate: REQ-300 acceptance; export vs preview within 2 levels; write budget.
- S5 Applied effects widget: ui/applied_effects.rs, movable and fixed rows, scroll, Move up and down (keyboard), undo labels, whole-video marks (REQ-303), 15% layout. Gate: REQ-302..304.
- S6 Overlay stage: render/overlay.rs, per-clip ASS, LUT after fades for flagged clips; legacy graph unchanged. Gate: REQ-305, REQ-311 (old projects), golden unchanged, QG-20 parity.
- S7 Shadows: text and subtitle style shadows (design A or B from S0d), ASS export. Gate: REQ-306, REQ-307 as written.
- S8 Subtitle styles: highlight colours (REQ-308), Typewriter (REQ-310), Enlarge (REQ-309, after S0b). Gate: REQ-308..310; event caps (2000 per document, 20000 per file).
- S9 Gates and docs: QG-19 (LUT budgets measured), QG-20 (overlay parity), QG-21 (legacy argv golden); architect and documenter update docs; version 2.1.0 via scripts/version.sh.
- Each stage: fmt, clippy -D warnings, tests, coverage run once at the end (PE only).

## 11. Risks (2.1.0)
- 256-point LUT: about 450 MB Work copy and 450 MB of text parsed by each ffmpeg process at start (preview restarts); ffmpeg table 201 MB per process. S0a measures; if the preview start exceeds 2 s, a binary input (.m3d, if the bundled ffmpeg reads it) or a preview-only smaller table is the next step (decided with Andrea).
- Load time of 256^3 on weak machines unmeasured; progress and cancel keep the window responsive.
- Enlarge centre accuracy (S0b); typewriter depends on \k and alpha (S0c).
- Boxed shadows: design A needs libass to draw the box from OutlineColour (S0d); design B draws the box as a vector event, which must match the 2.0.4 geometry.
- Legacy stacking must stay byte-identical; the overlay stage activates only for changed orders.
- LUT after fades (new projects, decided with Q-2): a LUT that lifts black shows on a faded frame (ASSUMPTION).
- Golden argv of projects with LUTs changes (file name); the PE updates only the fixtures that use a LUT and reports it.
- Event counts: boxed shadows and the caps (2000/20000) re-checked on a long subtitle file.

## 12. Open questions
- none (Q-1, Q-2, Q-3 decided by Andrea 2026-11-21; REQ-307 kept as written).

## 13. Future work (parts postponed)
- 360 movable order for sphere-fixed items (postponed; Q-3 = A).
- Binary LUT input to ffmpeg (.m3d) if S0a shows the text parse dominates the preview start.
- Colour management of LUTs (out of scope, unchanged).
- Typing sound (out of scope, REQ-310).
