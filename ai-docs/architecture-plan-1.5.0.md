# ai-notes: architecture plan 1.5.0 (frame: Change aspect, Scale, Move, keyframes)

Owner: architect. PLAN 2026-11-19 (target 1.5.0). Requirements REQ-124..132 (docs/business-frame.md). Human version: docs/architecture-frame.md. ADR-093..104 (next free ADR-105). No new crate. Project format 6 -> 7.

## Code facts (verified 2026-11-19)
- render.rs: `Segment` (per clip: media, path, src_in, len, skip, fades, gain, clean, to_black, sphere: Option<Vec<Pov>>, anchored); `build_graph` (video part ~L560-660): flat clip = `trim,setpts,scale=w:h:force_original_aspect_ratio=decrease,pad=w:h:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,fps,format`; sphere (camera) clip = v360 with sendcmd command file, output = geometry, no pad; fades applied per segment (`fades()`); concat; subtitles burned on `[cv]` after concat.
- `Geometry{width,height,fps}`: built from EncodeSettings (`Geometry::of`), `preview()` = 360 high. Users: export_args, plan_export_choice (L929/1021), output_geometry (L1207), preview video_render, still_args, check_graph/check_size (CHECK_MAX_PIXELS 1920*1080), hwenc sizes.
- ExportMode {Automatic, Sphere360, FlatCamera}; Sphere360 uses the equirect branch (scale+pad of the 360 picture), no camera.
- Timeline: `Clip` is Copy+Eq (fields id, media, src_in, src_out, volume, muted, clean_audio): keyframe lists cannot live in Clip without dropping Copy. Timeline has fades (BTreeSet<(ClipId,ClipId)>), start_fade, end_fade (edge attributes, not per clip). Edit snapshot = timeline + music + subtitles + mix + camera + texts (history cap 100).
- camera.rs: CameraPaths(BTreeMap<MediaId, Vec<Pov>>), `at()` hold edges, linear, shortest yaw arc; MAX_POVS 200; command_lines for sendcmd. Keys in SOURCE time. Precedent for keys.
- subtitles::Pos {x:u16,y:u16} 1/10000 of frame, clamped. Sizes elsewhere permille (text 250..=4000, subtitle size).
- project.rs: SCHEMA_VERSION = 6 (since 1.4.0); newer versions refused (L328); `from_json` returns notes Vec<String> (precedent for "corrected with a note").
- preview.rs / render::FrameKey: fields path, frame, fade, size, subtitles, preset, camera, texts, line_anchors. frame_key() decides when a still is rendered (REQ-034).
- ffmpeg 6.1.1 locally (inspection): `scale` has `eval=frame` (w/h expressions per frame); `overlay` has `x`/`y` expressions with `eval=frame`. Whether `t` is valid in scale expressions is UNVERIFIED: PE checks on the bundled build (see step 5).
- app/effects.rs ApplyEffect{EffectKind{Fade,Subtitles,Volume,CleanAudio}, Target{Clip,WholeVideo}}; ui/boxes.rs has BoxItem, handle_at, resize(rect0,corner,p,bounds)->(rect,factor) (proportional, opposite corner fixed) reusable for Scale.

## ADRs (1.5.0)
- ADR-093 Storage. Change aspect = `Timeline.frame_aspect: Option<FrameAspect>` (whole video, one per project; None = no effect). Whole-video keys = `Timeline.whole_scale: Vec<Key<Scale>>`, `Timeline.whole_move: Vec<Key<Pos>>`. Clip keys = `Timeline.clip_frames: Vec<ClipFrames{clip: ClipId, scale: Vec<Key<Scale>>, moved: Vec<Key<Pos>>}>` sorted by clip id, empty entries removed. All `#[serde(default)]`. Rejected: fields in Clip (breaks Copy+Eq), BTreeMap<ClipId,_> (serde_json map keys must be strings; newtype u64 fails), separate Project map (sync with every timeline op). Reason: lives in Timeline so undo, split/merge/remove are handled in one pure module.
- ADR-094 Time base. Clip keys in SOURCE time of the clip's media (as POVs, REQ-102). Trim and split never delete or shift keys: the full stored path is evaluated at source time, so the value at a new edge is the one the path had there (REQ-129), keys outside the kept part are hidden (not drawn, still used for interpolation). Whole-video keys in TIMELINE time; edits do not move them (like music REQ-019 and edge fades). Rejected: clip-local time (trim would move or drop keys), re-keying at each edit (edge arithmetic, more undo risk).
- ADR-095 Values and limits. `Key<T>{t: Micros, v: T}`; sorted, unique t, at most 200 per effect per clip and per whole video. Scale = permille of the picture's base size, 100..=4000 (10%..400%). Move = `Pos` centre of the picture, 0..=10000 (default 5000,5000). `at()`: hold before first and after last, linear between neighbours, no easing. Implemented in NEW `src/keyframes.rs` (generic; camera.rs NOT refactored in 1.5.0, debt in Future work).
- ADR-096 Scale pivot. Corner drag keeps the opposite corner fixed (REQ-127). Scale keys carry an invisible centre offset `(dx,dy)` in 1/10000 of the frame (set by corner drags; 0,0 by default). Move is independent and never written by Scale drags (no implicit Move effect). Picture centre = frame centre + Move offset + Scale offset (both interpolated). Alternatives: Move key written by corner drag (creates a Move effect the user did not ask for); scale about the centre (breaks the REQ-127 acceptance). TECHNICAL.
- ADR-097 Frame geometry. Pure `src/frame.rs`: `frame_size(base:(u32,u32), aspect) -> (u32,u32)`. Base = the output geometry as today (format choice or 1.4.0 video preset; REQ-022/REQ-120). Fit on width: w = base w, h = round_even(w / shape_ratio); Fit on height: h = base h, w = round_even(h * shape_ratio). Even sides. Longer side > 4096: scale both sides by 4096/long, keep shape, even (decided: Q-A = A, 2026-11-19). Shapes 4:3, 16:9, 9:16 as exact ratios. `Geometry` is built from this in export/preview/check/hw paths (one function, no other size logic).
- ADR-098 Picture layers (per flat segment, only when a frame effect applies to the segment, i.e. aspect set or any Scale/Move key present). Stage A: the existing fit, WITHOUT pad: `scale=w:h:force_original_aspect_ratio=decrease` to the base size (the source's own picture; base bars are not part of the picture). Camera view of 360 clips unchanged (fills base size). Stage B (new): background of frame size: colour (`color=c=0xRRGGBB:s=WxH:d=<len>:r=<fps>`) or blurred copy (split; `scale` to 1/8 size with cover + crop; `boxblur` fixed radius; `scale` back to frame size). Stage C: `overlay` of the picture at per-frame x,y, the picture scaled per frame (`scale=eval=frame`), with the combined expressions of ADR-099. Stage D: fades (existing `fades()`) on the composed segment, so fade to black covers background too. Then concat, then subtitles/text (unchanged, whole frame: REQ-131 (4)). With no frame effect the argv is byte-identical to 1.4.1 (golden test). Rejected: `pad` with computed margins (cannot animate); `zoompan` (zoom >= 1 only, cannot shrink); per-frame stills (too slow); runtime `sendcmd` on scale (filter-chain caveats, fragile). Fallback if `scale`/`overlay` cannot take `t` in the bundled ffmpeg: sampled command file like camera (one line per change, bounded by MAX_COMMAND_LINES).
- ADR-099 Expressions. Built only from numbers in Rust (integers and fixed-decimal floats), never from user strings. Piecewise linear: nested `if(lt(T,t_i), a_i + (b_i-a_i)*(T-t_i)/(t_{i+1}-t_i), ...)`, T = segment-local time plus constant offset (timeline time for whole video; source time for clip keys). Combined: scale = whole * clip (product), centre offset = sum. Sizes forced even: `trunc(x/2)*2`. Filter string over the script-file threshold goes through `-filter_complex_script` (existing mechanism). Guard: a non-finite value is never written (pure check + test). Keys above 200 per effect never reach the generator (validation).
- ADR-100 Colour and blur. Colour: `#RRGGBB` chosen with egui colour picker, default `#000000`, validated by a pure parser and written as `0xRRGGBB`. Blur: fixed strength chosen by videolore (REQ-126), constants in one function, computed on a 1/8-size copy (cost). No user string enters a filter graph.
- ADR-101 Edit rules. Frame effects live in Timeline, so every edit is in the undo snapshot. Clip split: both halves get a copy of the keys. Copy/paste: copy with the new ClipId. Remove clip: drop its entry. Merge: needs equal keys on both clips (like volume and clean audio: "The clips have different frame settings"), so no key is lost. One undo per Add, Remove, value change at release, Remove effect, aspect change. Drag of a corner or the target: transient in the preview (like drag_pos), one undo step at release.
- ADR-102 Export rules. Any frame effect forces re-encode (REQ-023/131), even at default values; hardware export applies (REQ-062/063). Sphere360 export (ExportMode::Sphere360): ignores aspect, Scale and Move (segments built with frame effects stripped, geometry = base). FlatCamera/Automatic: Scale/Move apply after camera view (stage C). Preview uses the same functions at preview geometry (`Geometry::preview()` of the frame size). Fades and text stay as before.
- ADR-103 Preview interaction priority (flat picture at playhead): subtitle/text boxes (existing topmost) > Scale corner handles > Move target > picture body (camera drag on 360 clips, existing). Scale/Move overlay (box, handles, target) drawn for each effect present at the playhead on the selected clip or whole video. Handles reuse ui/boxes.rs logic (bounds 10..400%). Pure logic in `src/frame_ui.rs`-like module (or ui/frame_boxes.rs): egui stays thin.
- ADR-104 Project format 7 (REQ-132). SCHEMA_VERSION 7 always written (1.4.x refuse it with their newer-version message; file untouched). Load of 6 unchanged (serde defaults). Validation on load, corrections with a note in the existing notes list: scale clamped to 100..=4000; Move clamped to 0..=10000; keys sorted, duplicate times: first kept; key times < 0 or beyond the media duration dropped; more than 200 keys per effect: extra dropped; unknown shape/fit/fill: effect removed with a note. Never a crash (REQ-057). Note: REQ-132 says "outside the clip"; ADR-094 needs "outside the media" (keys outside the kept part are legal). REQ-132 wording is PM's: flagged in the 1.5.0 REFINE return (2026-11-19), not edited by architect.

## Components and changes
- `src/keyframes.rs` (NEW, pure): Key<T>, Lerp for Scale(u16 permille) and Pos, `at(keys, t) -> Option<T>`, `insert/replace/remove/edit` with the 200 limit and unique times, `validate`.
- `src/frame.rs` (NEW, pure): FrameAspect{shape: Shape{Wide4x3,Wide16x9,Tall9x16}, fit: Fit{Width,Height}, fill: Fill{Colour(Rgb), Blur}} (Default: 16:9, Width, black); frame_size(); picture_base(); constants (4096, blur radius, 1/8 factor).
- `src/timeline.rs`: fields from ADR-093; ops: split/copy/paste/merge/remove hooks; `clip_frames(id)`, `set_frame_aspect`, key ops on clip or whole video (pure, return EditError on rule violation).
- `src/render.rs`: Segment gains `frame: Option<SegFrame{aspect: Option<(u32,u32) geometry>, scale_expr: String, x_expr, y_expr, bg: Fill, base_size}>` built in `segments()`; build_graph branch "frame" (stage A-D); `plan_export_choice` re-encode rule; geometry from frame::frame_size; Sphere360 strips frame.
- `src/project.rs`: schema 7, validation notes (ADR-104).
- `src/app/frame.rs` (NEW submodule; app.rs is 3353 lines): commands ApplyFrame{Target}, SetFrameAspect, RemoveFrameAspect, AddKey, RemoveKey, SetKeyValue, DragPicture (transient), RemoveEffect{Target, Scale|Move}. Undo names "Change aspect", "Add keyframe"...
- `src/ui/frame_popups.rs` (NEW, egui thin): aspect popup (3 combos + colour + Remove effect); Scale/Move popup (timeline widget via pure `layout` helper in `src/keytimeline.rs`, Add/Remove buttons, value field in percent).
- `src/ui/effects.rs`, `src/ui/menu.rs`, `src/ui/actions.rs`: 8 rows (REQ-130); EffectKind gains Scale, Move (drag-able), ChangeAspect (not drag-able).
- `src/preview.rs` / `render::frame_key`: FrameKey gains the frame state at playhead (aspect, scale/move values) plus transient drag override.

## Ordered steps for the Principal Engineer
1. keyframes.rs + unit tests: hold, linear, limits 200, unique times, validate (REQ-129, REQ-132 parts).
2. frame.rs + tests: the REQ-125 acceptance numbers (1600x1200: 9:16 Fit width -> 1600x2844, picture 1600x1200 centred; Fit height -> 675x1200), even sides, 4096 cap, same shape when already matching (no empty space) (REQ-125).
3. timeline.rs fields + ops + rules (split/copy/paste/merge/remove; undo via Edit) tests (REQ-129, REQ-130 rules).
4. project.rs v7 + load validation + refusal of v7 by 1.4.1 (test: v6 unchanged output; v7 write; corrections with notes) (REQ-132, REQ-029, REQ-057).
5. Real ffmpeg check (VIDEOLORE_REQUIRE_FFMPEG=1): a 3-frame test clip; `scale=eval=frame` with `t` expression and overlay x/y with `t` on the bundled build. If it fails: switch to the sampled command file (ADR-098 fallback) and record a deviation in the plan.
6. render.rs: stage A-D, expression generator (pure string tests with golden text for 1 and 3 keys), no-effect golden argv unchanged, Sphere360 ignores frame, FlatCamera applies after camera view, re-encode rule (REQ-023, REQ-131, REQ-022, REQ-120).
7. Real ffmpeg behaviour tests (skip only if no ffmpeg locally; CI requires it): 1600x1200 to 9:16 frame size and empty-space colour exact (colour); blur region not sharp at the picture border (blur); Scale 50% background visible; Move 10% of frame width = 10% (+-1%); Scale 100% at 0 s and 200% at 4 s gives 150% at 2 s (sample by pixel size); constant before first and after last key; subtitles at bottom stay bottom at Scale 50% (within 2%); 360 video export unchanged with and without effects.
8. export/hw: stream copy refused with any frame effect; hardware path accepts the geometry (hwenc tests with simulated encoder); output check uses frame size (outcheck tests).
9. preview: FrameKey includes frame state; still render only on change; burst of drags renders at most once (REQ-034 test).
10. app: commands, one undo per action, Esc/ClosePopup, popup opens for second Scale on a clip (REQ-130), "Remove effect" (REQ-124/129), dropping Change aspect on a clip ignored (REQ-130).
11. ui: effects rows, menus, drag rules, popups, preview boxes and handle priority (ux tests as existing ui/ux_tests.rs pattern) (REQ-124, 127, 128, 129, 130).
12. Coverage: every REQ-124..132 has `// covers:` tests; line coverage >= 85%; feature coverage 100%.

## Tests per REQ (summary)
- REQ-124: dropdown options exact; one undo per change; Remove effect restores source frame (ux + app tests).
- REQ-125: frame_size numbers and even sides; cap; no empty space when shape equal (frame.rs), real-ffmpeg frame size.
- REQ-126: blurred border no sharp edge; colour exact pixel; unchanged sharp picture (real ffmpeg, pixel sampling with tolerance).
- REQ-127: corner drag keeps opposite corner (pure), ratio kept, limits 10/400, 50% background visible, one undo.
- REQ-128: target drag 10% = 10% +-1%, clamp at edges, fraction survives frame size change.
- REQ-129: keyframe math (keyframes.rs), Add at 1 s changes nothing, first-key removal, split keeps values at edges, 200 limit, popup timeline layout at widths.
- REQ-130: row names exact, drop rules, marker opens popup, second Scale opens existing popup, clip + whole stacking (product/sum) in expression tests.
- REQ-131: composition order (golden text), any effect re-encodes at defaults, subtitles stay within 2%, 360 export identical.
- REQ-132: v7 write, 1.4.1 refusal (message test in existing version tests), v6 unchanged, damaged values corrected with notes (scale 0 and 1000 -> 100 and 4000; no panic on garbage).

## NFR (1.5.0, targets; estimates unless marked measured)
- Interface: every frame under 200 ms while popups open and corners dragged during export and playback (existing NFR).
- Still preview of a frame-effect picture at 360 high: under 80 ms (estimate); no render for edits that do not change the picture (REQ-034 unchanged).
- Export of a 1080p source to 9:16 (1080x1920 frame) with Scale/Move: at most 1.5x the time of the same re-encode without effects (assumption; measured by QG).
- Blur background at 1/8 size: adds at most 10% to the frame's composition time (estimate).
- Filter script for a segment: at most 1 MB; if more, sampled command file (fallback).
- Memory: no new buffer; at most one picture + one frame in ffmpeg per segment; frames up to 4096 px on the longer side.
- Project load and validation of 200 keys x 10 effects under 10 ms.

## Security (1.5.0)
- Inputs: project numbers (validated ranges), colour string (strict #RRGGBB), aspect enums (closed sets). No free text reaches a filter graph (ADR-099/100). Expression text only from integers and fixed decimals; a fuzz-like test with random keys checks that no character outside `0-9 . , ( ) + - * / < > t if lt trunc` set appears.
- Large frames: 4096 cap limits memory and encoder stress (protects against a crafted project with 16384-pixel frames; frame size comes from base and shape only, never from file values).
- No network, no new file types, no new temporary files beyond the existing segment script.

## Crates
None new. ffmpeg filters used: color, split, scale, boxblur, overlay, trim, setpts, format, concat (all core, in every bundled build per ADR-037 smoke test list; PE confirms `boxblur` and `color` in the bundled `-filters` list). egui colour picker: part of egui (existing dependency).

## Risks
- Scale/overlay may not accept `t` on the bundled ffmpeg (step 5): fallback in ADR-098.
- Huge frames (4K 9:16 => 2160x3840 and more): slow export, encoder limits, hardware encoder refusal (silent software redo exists, REQ-063).
- Keyframes in a small popup: usability (timeline width at small windows; min width 240 px assumption).
- Subtitles off the picture after Scale/Move: not an issue by design (subtitles drawn after composition), but text items stay put (REQ-131) and may end over the picture edge.
- Blur cost unmeasured; 360 export shows no effect (may confuse; documented in limitations).
- Stacking whole-video and clip keys of Scale and Move (product and sum) is not what a user may expect from the popups; shown as tooltip count only.
- Whole-video keys do not follow trims (timeline time): a whole-video key placed after a removed section stays at its absolute time.

## Questions
- Q-A [SEVERE PRODUCT-CHANGING] CLOSED 2026-11-19: product owner chose (A), cap the longer side at 4096 px, keep the shape, scale down (ADR-097 as written). Not chosen: (B) 1920 px cap, (C) no cap (some players and hardware encoders refuse frames above 4096 px).
- Other items: TECHNICAL (decided above: ADR-093..104).

## Future work
- camera.rs and keyframes.rs share the same rules: move camera onto keyframes.rs (debt; risk to 360 tests).
- Whole-video keys that follow trims (move keys with the cut).
- Easing curves per keyframe (PM: out of scope in 1.5.0).
- Blur strength setting (PM: no setting in 1.5.0).
- Measured blur and composition cost; faster path for Scale 100% and Move-only (no scale filter).
- Text items following Scale/Move (PM: not keyframed in 1.5.0).
