# ai-notes: architecture 1.3.0 (text items, selection, context menu, 360 anchors, 360 export): plan and as built

Owner: architect. Index: [architecture.md](architecture.md). ADRs ADR-066..079 are in the index file. Human docs: docs/architecture-text.md, architecture-360.md, architecture-boxes.md, architecture-ui.md, architecture-projects.md, architecture-effects.md, architecture-dependencies.md, architecture-security.md.

## Code facts that drive the plan (read 2026-11-17)
- src/ass.rs: ASS writer (escape(), pos_tag(), line_tag(), document()); bundled Noto Sans Bold copied to ffmpeg work folder; PlayRes = export size.
- src/camera.rs: Camera{yaw,pitch,fov} in 1/100 deg; filter() builds `v360@<id>` (input=e output=flat reset_rot=1 d_fov=...); diagonal_fov_deg/vertical_fov_deg; command_lines() for sendcmd.
- src/app/sphere.rs: 360 app logic (live view, POV, align/reset). src/ui/picture.rs + src/ui/boxes.rs: box drawing/hit test (boxes.rs has rect_at).
- Selection today is TWO states: app.selection: HashSet<LineId> and app.selected_line: Option<LineId> (src/app/effects.rs ~l.170-225, 336). Likely cause of "multi-selection lost on preview click". Fix = one Selection value.
- Subtitles burn after concat in the flat graph; 360 segments get `v360@cN` per segment before concat (architecture-360.md).
- egui text layout measures boxes with the bundled font only; loading an installed font needs its bytes into egui (FontDefinitions, set_fonts): done by a worker, cached LRU.

## Crate decision
- fontdb 0.24.0 (MIT, rust-version 1.71). Default features (fs, memmap, fontconfig -> fontconfig-parser 0.5.8 MIT, memmap2 0.9 MIT/Apache, roxmltree MIT/Apache, ttf-parser 0.25 MIT/Apache). Enumerates system font families (Linux /usr/share/fonts, ~/.fonts, XDG + fontconfig <dir>; macOS /System/Library/Fonts, /Library/Fonts, ~/Library/Fonts; Windows C:\Windows\Fonts). Default features kept so the picker matches fontconfig's directories (libass uses fontconfig on Linux).
- No other crate. Colour picker = egui built-in. Measuring = egui layout.
- Rejected: font-kit 0.14.3 (native per-OS deps, more crates), fontique 0.13.0 (Apache/MIT, heavier, MSRV 1.88 but larger tree), per-OS APIs (three implementations).

## Text items: model (project v5)
TextItem { id: TextId(u64), text: String (lines by \n, <=1000 chars per line as subtitles), start: Micros, ttl: Micros (>=100 ms, <= video length), effects: TextEffects { fade_in: Option<Micros>, fade_out: Option<Micros>, blink: Option<Micros>, slide: Option<(SlideDir, Micros)> }, colour: [u8;3] (#RRGGBB), font: String (family name, <=100 chars, charset [letters digits space - . _ ( )]), place: Pos (flat centre, 1/10000, same type as subtitles Pos), size_permille: u16 (250..4000), anchor: Option<Anchor360> }
Anchor360 { yaw_cd: i32, pitch_cd: i32 (centre of box on the sphere), size_cd: u32 (world height of text at 100%, = 5% of view vertical FOV at add time, 1/100 deg; clamp 1..=17000), wrap_cd: u32 (wrap width in deg = 60% of the view horizontal FOV at add time, assumption), }
SubtitleLine gains anchor: Option<Anchor360> (#[serde(default, skip_serializing_if=None)]); line.position stays the flat place (used when a flat segment shows the line).
Defaults: default effect times 0.5 s (fade), blink 0.5 s, slide from left 0.5 s; ttl default 3 s; colour white.
Limits: effect settings 100..=5000 ms; ttl 100 ms..video length; text 1..=N lines; events caps below.

## Effects -> ASS events (pure, src/text.rs NEW, no I/O)
Input: item, global timeline (segments), frame size. Output: Vec<AssEvent{start,end,layer,tags,text}> computed on the TIMELINE, then cut per segment (see Graph).
- Envelope a(t) = fade_in ramp * fade_out ramp * blink square wave * (1 if slide done else 0 for position). Each fade limited to ttl/2 (REQ-109 d).
- Fade IN/OUT only: one event with \fad(in_ms,out_ms) when no other envelope; with blink: each visible blink phase is one event whose \fad applies to its own start/end (fade ramp computed per phase: alpha tags via \alpha or \fade with absolute times relative to event start). PE may use \t(\alpha) for alpha steps if \fade is insufficient; test by sampling alpha at 0.5 s steps vs formula within 5%.
- Slide: \move(x1,y1,x2,y2,t1,t2) with times RELATIVE to event start; start = box one width (height for top/bottom) outside, outside picture; reaches box after duration. Clipping a move event at a segment boundary recomputes x1,y1 and t1/t2 for the clipped start.
- Blink: visible phases [0,I),[2I,3I),... (I = interval), hidden between. One Dialogue event per visible phase. Cap: 2000 events per item; 20000 per project; over cap = refused at add/edit with a message (validation, not silent).
- Escape: text through the existing ass::escape (replaces { } \ with look-alikes); newlines -> \N written by us; wrap by measured layout as subtitles.
- Font: \fn<family> from sanitised family; missing family -> libass fallback (documented), UI shows "not installed here".
- Colour: \1c&HBBGGRR&.

## Flat render (REQ-108..111)
- Text items with no anchor, and anchored items on flat segments (flat place), go to the flat ASS burned after concat (same file as subtitles, extra style per distinct font/colour via inline tags). Any text item -> re-encode (REQ-023).
- Box: egui layout with the item's font loaded in egui (worker-loaded bytes, LRU 8, cap 64 MiB per file). Box code shared with subtitle boxes (ui/boxes.rs) - "box layer takes a list of boxes" (ADR-064).
- Default place: centre x = 50%, centre y = 83% (lower third); size factor 100% = 5% of picture height (assumption from REQ-111).

## Selection (REQ-090..092) - ADR-073
- window state: `Selection { items: BTreeSet<ObjRef> }`, ObjRef = Line(LineId) | Text(TextId) | (later) Pov. Remove selected_line and HashSet selection; `primary: Option<ObjRef>` only for popup focus.
- Pure functions in src/selection.rs: `click(sel, obj, mods) -> Sel` (plain: select only obj unless obj already selected and no mods -> keep (for drag); mods Ctrl/Cmd/Shift: toggle), `shift_range(sel, objs_with_start, range, mods)`, `drag_targets(sel, obj) -> Vec<ObjRef>` (selection if obj in it and len>1 else obj), `clear_on_empty_click`, `esc(sel, popup_open)`, `prune(sel, existing)`.
- Preview: plain click on a box that is selected keeps the selection, drag moves group (REQ-092); Ctrl/Shift+click on box toggles without moving.
- Tests: pure transitions table; headless UI: Ctrl+click 3 markers -> 3; preview Shift+click; drag keeps selection; undo group move 1 step.

## Context menu (REQ-106) - ADR-074
- pure `context_group(sel: &Selection, playhead_on_360: bool) -> Option<ContextGroup>`: Some(Subtitle) if any Line selected (Align H, Align V, Reset); Some(Camera) if playhead on 360 (Set POV + Reset view) -> if both: both sets; None otherwise. Text selected alone -> None.
- Align/Reset/Set POV stay as shared table actions (no new commands). Greyed reasons dropped from the group only.

## 360 anchors (REQ-112) - ADR-069/070
- Projection, pure camera.rs additions: `project(cam, w, h, yaw_deg, pitch_deg) -> Option<(f64,f64)>` rectilinear: f = (w/2)/tan(hfov/2); right/forward/up from yaw/pitch (sign per v360 spike: positive yaw looks right, positive pitch looks up); None when behind the view. `unproject(cam, w, h, x, y) -> (yaw, pitch)`. Box corners projected in the sphere's tangent plane at the anchor; hit test in view.
- Equirect mapping for burn-in: x = (yaw/360 + 0.5)*W (mod W), y = (0.5 - pitch/180)*H; px per degree H/180 vertical, W/360 horizontal. Sign of yaw verified by band-picture test (same as 1.2.6 spike).
- Anchored text burn-in: laid out in equirect pixels (size = size_cd*H/180*scale; wrap = wrap_cd*W/360*...), centred on the anchor; if the box crosses x=0 or x=W, the event is written twice (x and x±W) with the same tags; events outside [0,W) clipped by libass.
- Per 360 segment in camera modes: ASS file (local times) burned on the source after trim/fps and BEFORE v360@cN; then v360; fades and concat as today. Flat-segment pieces stay in the flat ASS.
- Drag in look-around view: anchor = unproject(pointer-to-centre offset); still frame request; box stays inside the view.
- Add at playhead on 360: anchor = centre of view at that time (live view or POV path at playhead), size per above; if view is in a 360 segment only.
- Slide of an anchored text moves in equirect pixels (direction left/right/top/bottom of the frame), distance = one box width in px. (TECHNICAL, ADR-077.)

## 360 video export (REQ-103) - ADR-072/078
- `ExportMode { Automatic, Sphere360, FlatCamera }` in the export plan; UI asks only for projects with 360 clips (modal, not saved). Sphere360 refused with message when any flat clip exists.
- Graph in Sphere360: no v360, no sendcmd; all segments equirect at the 360 clips' size (REQ-022 lowest pixel count); anchored text burn-in as above; view-fixed subtitle lines at their preset place in the frame (ADR-078, confirmed Q2=A).
- Metadata: the output must carry Spherical Video V1 (equirectangular, no bounds). SPIKE FIRST: check whether bundled ffmpeg writes spherical side data for re-encoded output (side data on decoder frames -> encoder packets; v360 not in graph so frames may keep it). If not, fallback options in order: (1) `-spherical-side-data` style option if the bundled build has it (check `ffmpeg -h full`), (2) remux step with stream copy from a side-data-carrying stream (check), (3) write the sv3d/st3d boxes ourselves in an MP4 remux (rejected unless required: adds binary writer). Record result as ADR-078 addendum. RESULT (1.3.0): ffmpeg 6.1.1 drops the side data; option (3) taken, see "Built in 1.3.0".

## Export graph preservation
- A project with no texts, no anchors and no 360 change keeps the 1.2.6 argv (golden fixtures v1..v4 byte-identical, tests/fixtures export-argv for 1.2.6).
- Process count unchanged (filters in the same ffmpeg process).

## Project v5 (REQ-113) - ADR-076
- SCHEMA_VERSION 5 written always (ADR-051/065 reason). Loads 1..5; TooNew > 5.
- Fields: texts (#[serde(default)] Vec<TextItem>), line.anchor (#[serde(default, skip_serializing_if)]).
- Validation (refuse as damaged): ttl < 100 ms or > video length; effect times outside 100..5000 ms; more than 2000 events per item or 20000 total; colour not #RRGGBB; font not in charset or > 100 chars; text > 1000 chars per line or > 200 lines; size_permille outside 250..4000; anchor yaw/pitch outside +-180/+-90 deg; size_cd 0 or > 17000; unknown ids; duplicate ids.
- Font not installed on this computer: kept in project, render falls back, UI notes it (no refusal).
- Dirty tracking: texts and anchors in ProjectDoc comparison (automatic); text popup commit = one undo step.

## Rename (REQ-107) - ADR-075
- UI label only: "Fade through black" -> "Fade IN/OUT" in one label table (ui/actions or effects table). Internal EffectKind name unchanged (no format change). Test: no user-visible string contains "Fade through black" (grep over labels, tooltips, shortcut window).

## Tests (feature coverage: every REQ-106..113 with `// covers:`)
- REQ-106: context_group table test; headless UI at 800x500 (group visible/hidden as specified).
- REQ-107: label test.
- REQ-108: popup headless: Enter newline, Add -> one text item at playhead, one undo; Esc creates nothing.
- REQ-109: pure event tests + real ffmpeg: slide start/finish +-1 frame, fade 50% at 0.5 s of 1 s (+-5%), blink hidden 0.5-1.0 s, fade cap ttl/2.
- REQ-110: timeline lane geometry (ruler helpers) bar length = ttl at 3 zooms (+-1 px); drag ends; no ripple; cut at video end; undo.
- REQ-111: box drag/resize bounds (25..400%), flat export centre within 2% H of preview (existing harness), absent at 1 s/6 s, re-encode forced (argv test).
- REQ-112: project math tests (yaw 30 pitch 10; turn 40 right -> 40 left; back to first place); real ffmpeg: anchored text at 3 sample times on flat camera view within 1 deg; seam case (text across yaw 180) drawn once on each side.
- REQ-103: Sphere360 argv has no v360; mixed project refused; metadata check (ffprobe side data) ; text at yaw 30 at 3 sample times +-1 deg in 360 output (sample frame analysis with colour marker).
- REQ-113: round trip v5; v4 fixture opens unchanged (golden argv); hostile values list; TooNew for v6.
- REQ-090..092: selection pure table + headless preview/timeline (in REQ-090 ids).

## Ordered implementation steps (done in 1.3.0; kept as the record of the order)
M0 baselines BEFORE change: golden argv fixtures v1..v4 and 1.2.6 export argv; headless frame time; SPIKES (record results in architecture.md ADR addenda): (a) spherical metadata on re-encoded 360 output with bundled ffmpeg; (b) libass family lookup for a font name from fontdb on Linux CI (DejaVu/Noto); (c) yaw sign for equirect burn-in (band picture); (d) egui set_fonts cost for one installed font of 20 MB.
M1 Selection (REQ-090..092): single Selection, pure transitions, fix multi-selection persistence. Existing tests may change meaning only with replacements (no weakening).
M2 Project v5 + TextItem model + validation + migration + hostile tests (REQ-113, 108 model).
M3 text.rs pure event generation + caps + tests (REQ-109).
M4 fonts module: background scan, sanitiser, worker byte load, LRU, missing-font handling (REQ-108 font part).
M5 Flat text burn-in and boxes: ass integration, re-encode forced, box code shared (REQ-111, 110 box).
M6 Add Text popup, Effects row, Text lane, popup/Delete/undo, selection hookup (REQ-108, 110, 075, 082, 071).
M7 Context group and rename (REQ-106, 107).
M8 camera project/unproject + real v360 test (REQ-112 math).
M9 anchors: add at playhead on 360, box in view, drag in view, equirect burn per 360 segment, flat fallback, seam (REQ-112).
M10 ExportMode::Sphere360, dialog state, graph without v360, metadata fix from spike, mixed refusal (REQ-103).
M11 perf: 4K/8K look-around frame times with texts during export (<200 ms gate), event build cost (< 50 ms for 200 items).
M12 quality: fmt, clippy -D warnings, test, coverage >= 85 %, feature coverage 100 %, CI green; PE reports numbers.

## Performance and limits (decisions)
- Text events: O(items) build, cached per (project revision, segment); no ffmpeg start per text.
- Font scan: worker thread at start, never blocks window; "Loading fonts..." in picker.
- Font bytes for measure: worker, cap 64 MiB, LRU 8; egui set_fonts only when a new font is first used.
- 360 text burn on preview copy 4096x2048: cost measured in M11 (gate: frames < 200 ms).
- Sphere360 export: no speed promise (user decision).

## Security (1.3.0)
- ASS injection: text through ass::escape; font name charset whitelist; colour hex only; numbers formatted by us.
- Font files: system fonts parsed by fontdb (safe Rust) and libass/FreeType in ffmpeg (C, trusted OS fonts; risk recorded). Reads: regular files only, cap 64 MiB.
- Resource caps: 2000 events/item, 20000/project; font LRU 8; 200 lines per text.
- No network, no new unsafe.
- Project hostile values validated on load (REQ-113).

## Risks (for later)
- Flat text near poles looks narrower (accepted, Q1=A).
- Spherical metadata: resolved in 1.3.0 by the own MP4/MOV writer (see "Built in 1.3.0"); MKV/WebM unmarked.
- libass vs fontdb family naming mismatch -> silent fallback in export only (test with a CI font).
- Blink with small intervals costs events (capped).
- egui vs libass wrap differences (already known for subtitles).
- Slide in equirect plane may leave the view while sliding (ADR-077).
- Sphere360 export slow (accepted).

## Answers (REFINE 2026-11-17)
Q1 = A: 360 text is a flat sticker on the sphere (exact place, one projection engine, narrower toward the poles). Q2 = A: automatic (view-fixed) subtitle lines in a 360 video export keep their preset place in the frame (lower third, about 30 deg below the front). No open questions. TECHNICAL: ADR-066..079 decisions.

## Built in 1.3.0 and deviations (implemented; measured values in docs/code-text.md)
- Modules: src/text.rs (events, caps, validate; MIN_TTL 100 ms, DEFAULT_TTL 3 s, MAX_TTL 86_400_000_000 us = 24 h), src/fonts.rs, src/selection.rs (ObjRef, Selection), src/spherical.rs, src/app/texts.rs, src/ui/text_popup.rs; tests src/app/text_tests.rs, src/ui/text_ux_tests.rs, tests/ffmpeg_integration/v13.rs.
- DEV-1 time to live (ADR-076 addendum): on load a text item is refused only when ttl < 100 ms or > 24 h (text.rs TextItem::validate). A ttl longer than the video is NOT refused on load (the text is cut at the video end). On add, edit and move ttl is clamped to MIN_TTL..=video length (app/texts.rs clamp_ttl). Plan said "longer than video = refused"; changed. The lower bound 100 ms on load is kept as in the plan.
- DEV-2 spherical marker (ADR-078 addendum): ffmpeg 6.1.1 (measured) drops source spherical side data on re-encode. Option (3) taken: src/spherical.rs rewrites the moov box of the export's own MP4/MOV and adds a uuid box ffcc8263-f855-4a93-8814-587a02521fdd with the Spherical Video V1 XML (equirectangular, no bounds). Only the small moov box is read (size-capped); the rest is copied as is; every size checked, no panic. MKV and WebM outputs are NOT marked. Bundled 9.x not measured for this path.
- DEV-3 anchored subtitle lines (ADR-066/069 consequence): an anchored subtitle line is drawn as a plain white TextItem, not with its preset style (no box, no word highlight). Automatic (view-fixed) lines unchanged. Listed as a known limitation in the changelog.
- DEV-4 selection (ADR-073): one Selection in src/selection.rs, as ADR-073 states; no deviation recorded.
