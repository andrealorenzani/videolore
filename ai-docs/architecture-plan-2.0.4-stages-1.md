# ai-notes: plan 2.0.4, stages 0–5 (owner: architect; index in ai-docs/architecture.md; main plan architecture-plan-2.0.4.md)

Template per stage: REQs; modules; data; tests (one `// covers:` per REQ); CI and packaging; performance; security; risks; manual (business-verification items); depends on and cut-back. Every stage ends with the full suite (PE only), then the hand-over.

## Stage 0 — preparation without behaviour change (QG-16 part; no REQ)
- Goal: keep the big files reviewable before stages 1–11 change them. No function changes behaviour; no argv changes.
- Modules: render.rs (3553 lines) split into `render/` submodules by area: picture composition, background and blur, audio chain, subtitle and text events, export argv (`export_args`), still and playback args. app.rs (3124 lines) split: `dispatch_inner` arms moved to the `app/` submodules by area (command, frame, effects, texts, lut, sphere, subs_create, export_flow, placing); public paths kept (ADR-141: no re-export shims beyond what compiles).
- Data: none. Tests: existing golden argv tests (render/golden_tests.rs, all fixtures) must pass unchanged; module-cycle test (tests/module_cycles.rs) passes; no test is changed except paths.
- CI and packaging: none.
- Performance: none expected; argv and frame timing gates unchanged.
- Security: none (no input path changes); `unsafe` count unchanged.
- Risks: visibility churn; keep `pub(crate)` minimal; do not move behaviour in the same commit as a split (the PE may split the work in two calls if large).
- Manual: none.
- Depends on: nothing. Cut-back: none (required before 3, 10, 11).
- ADR-168.

## Stage 1 — subtitle files and fonts (REQ-180..188; batch 5) — format 11 bump
- REQs: 180 export .srt/.vtt/.ass (File > Export subtitles…, renamed); 181 .ass with place and size; 182 Full video Subtitles: burned or separate track; 183 import .srt/.vtt (scope as REQ-085; UTF-8 with BOM; other encodings refused); 184 re-time from speech (checkbox, off, ±1 s window, REQ-014 consent and progress); 185 "Import <file name>" offer for a same-name file in the folder; 186 re-analysis of changed media on open; 187 fonts for Arabic and Devanagari bundled, CJK from system fonts with a note when none is installed (D61); 188 subtitle list shows those scripts.
- Modules:
  - `srt.rs`: `write_vtt` (WEBVTT header, HH:MM:SS.mmm, UTF-8 without BOM); `parse_vtt` (NOTE and STYLE blocks skipped, cue settings ignored, `strip_tags` as REQ-087); BOM accepted by the parse entry for both formats; non-UTF-8 refused with a typed error (no guessing).
  - `ass.rs`: `export_file(lines, style, frame)`: PlayResX/Y = project frame; per line `\an5\pos` (place from REQ-089) and `\fs` scale from size (REQ-097); reuses the existing line tag writer.
  - `subtitles.rs`: `import(cues, scope, timing)` where timing is "file" or "from speech"; REQ-044 conflict rule reused; one undo step. Re-time = `align.rs` word matching with a ±1 s window per cue; a cue without a match keeps its file times and the count goes to the status line.
  - `stt.rs`: no new recognition code; re-time runs the existing extraction and recognition jobs for the cue ranges (consent as REQ-014).
  - `export.rs` and `export/choice.rs`: subtitle mode Burned (default) or Separate track. Separate: the lines are written as SRT into the private work folder, a third input, `-c:s mov_text` (MP4, MOV) or `-c:s srt` (MKV), `-metadata:s:s:0 language=<iso639-2>` from `base::language`, `-disposition:s:0 0`; the picture has no burned text. Burned-in argv unchanged (golden).
  - `media.rs` and `project.rs`: `MediaEntry.file_size`, `modified_ms`. Analysis timestamp check on open (REQ-186): changed media is analysed again, its track list updated, and named in the status line. Migration from format 10: stats recorded without re-analysis (no analysis count change on first open). `SCHEMA_VERSION` 11; loads 1..11; newer refused; `project::v11` is not needed (format 10 → 11 field defaults only).
  - `fonts.rs`, `base/font.rs`, `assets/fonts/`: two committed OFL faces only, Noto Sans Arabic and Noto Sans Devanagari (family names and sha-256 checked by a test; SOURCE.txt updated; each licence in `packaging/notices/third-party.txt`, REQ-031). No CJK font is bundled (Q-3 closed by Andrea, D61): Chinese, Japanese and Korean use installed system fonts.
  - CJK coverage check: `fonts` scans the installed faces with skrifa's character map for each script's sample characters (kana and kanji, hangul, han); a script with no covering installed font shows the note "No installed font shows <script> text; install a font for it" in the subtitle list and in the export status line (REQ-187/188). Check is cached per font list, runs on the font scan thread, never on the UI path.
  - `ui/subtitles` popups (ui/popups.rs and ui/text_popup.rs for the subtitle list): Import button and menu item; "Import <file>" offer (REQ-185); the list and line popups use the added faces: egui receives them as fallback fonts on the first open of the subtitle list (ADR-172).
  - `ui/export_dialog.rs`: Subtitles row in Full video mode.
- Data (format 11): `MediaEntry.file_size` (u64, absent = unknown), `modified_ms` (i64 ms, absent = unknown).
- Tests (`// covers:`): 180 (writer: 3 lines → 3 cues, same times and text; cancelled write leaves no file; greyed with 0 lines), 181 (position within 1 % and size 150 % of style size; reference ASS parser reads all lines), 182 (ffprobe lists one subtitle stream with language; picture at a line's time has no text; default argv unchanged), 183 (3-cue SRT; VTT positions ignored; Latin-1 refused and nothing changes; one undo), 184 (simulated recognition moves a 0.6 s early cue; checkbox off = file times; cancel changes nothing), 185 (offer present with a same-name file; no line without click), 186 (replaced MKV re-analysed and named; unchanged media count zero), 187 (Arabic and Devanagari glyph test against a reference render; CJK note present without an installed CJK font and absent with one; notices list each bundled font and licence), 188 (UI test: each script line visible in the popup list). Plus format 11 tests: load 10 and 11, refuse 12, migration of 10 keeps the analysis count at zero.
- CI and packaging: the installer configuration check reports the font size (Arabic and Devanagari only, a few MB, well under the 25 MB MINOR budget, which no longer has CJK pressure); notices script lists the two new fonts with OFL-1.1; font sha-256 checked by a test. Tests for the note: a fake font list without CJK shows the note for Han, kana and hangul; with an installed CJK font the note is absent.
- Performance: import of 10 000 cues < 100 ms (estimate, unit); VTT write of 10 000 cues < 100 ms (estimate); UI frame ≤ 200 ms during import; UI font parse on the first open of the subtitle list, measured (target ≤ 100 ms once, estimate). Installer size + the two faces (estimate 1–2 MB each, to measure).
- Security: VTT and SRT read through `base::io::read_limited` (16 MiB cap), cue cap 100 000 (existing `srt::parse` limits), times finite and checked, text sanitised (control and bidi as the display rule), ASS escape on every line text; no network; the fonts are trusted build inputs, checked by sha-256.
- Risks: libass shaping of Arabic and Devanagari needs libass with HarfBuzz in the bundled ffmpeg (check `ffmpeg -buildconf` in the test); font copy naming (ADR-156); re-time on a clip with no speech.
- Manual: items 9 (Arabic and Devanagari look right on three OS, licences of the two faces, installer size; Chinese, Japanese and Korean depend on the system fonts and show the note where none is installed) and 10 (separate track in a player).
- Depends on: stage 0. Cut-back: REQ-184 to roadmap if the align window fails the 0.3 s gate; Arabic or Devanagari font only if its licence check fails.
- ADR-169 (format 11 bump), ADR-170 (VTT and ASS writers and import), ADR-171 (separate track), ADR-172 (fonts; system fonts for CJK, D61).

## Stage 2 — editing gestures, autosave and safety (REQ-189..199; batch 6)
- REQs: 189 autosave every 2 minutes to cache; 190 recover, discard, cancel at start; 191 trim by dragging a clip edge; 192 drag subtitle start and end markers (0.1 s steps); 193 drag fade markers between boundaries; 194 keys [ and ]; 195 Ctrl/Cmd+V pastes the last cut or copy of videolore; 196 undo kept when a file is removed; 197 Delete and 0.5 s shifts of selected lines; 198 snap guides for dragged lines and texts; 199 safe-area overlay (preview only).
- Modules:
  - NEW `autosave.rs`: pure policy (`due(now, last, dirty)`, `is_newer(copy, save)`, `name_for(path)`), writer on a worker thread fed by a clone of the project (ADR-173). Names: `<cache>/videolore/autosave/<sha-256 prefix of canonical path>.vlproj` plus `.path` with the original path (≤ 1024 bytes); untitled projects get a random session name. Written with `base::io::write_atomic` in format 11 (same serializer as Save).
  - `app/`: autosave tick from `tick`; Discard and Recover commands; the start-up offer (modals) before the first project is shown; Save and Discard remove the copy of the open project (own name only).
  - `timeline.rs`: `trim_edge(clip, edge, delta_frames, source limits)` (one frame steps; ≥ 1 frame); `move_fade(from_boundary, to_boundary)` (one fade per boundary, drop on occupied or empty space = no change).
  - `subtitles.rs`: `set_edge(line, edge, t)` (0.1 s steps, start < end, ≥ 0.1 s); `shift_lines(ids, delta, floor 0)`; Delete of several lines as one command.
  - `history.rs`: `forget_media(id)` drops every snapshot whose state references the media (snapshots are full states, so the rest stay in order) (ADR-175).
  - `app/command.rs`: clipboard of the last cut or copy inside AppState (not the system clipboard); Ctrl/Cmd+V pastes it (menu and toolbar Paste too).
  - `placement.rs`: `snap(rect, picture, others, tolerance 1 %) -> (rect, guides)` for centre, thirds (33 %, 67 %) and other items' centres.
  - `ui/timeline.rs`, `ui/markers.rs`: drag states in UI only; one command on release (ADR-174).
  - `ui/keymap.rs`, `ui/actions.rs`: [ and ] (no modifier), listed in the shortcut window (REQ-071).
  - `ui/picture.rs`: guides drawn while snapped; safe areas drawn on the preview only.
  - `settings.rs`: program settings `snap_to_guides` (default on), `show_safe_areas` (default off) (additive, VERSION 2).
- Data: none new in the project (autosave is a copy of format 11).
- Tests: 189 (copy in cache after 2 min of simulated time; project file checksum unchanged; copy gone after Save; no pause: UI frame test during autosave), 190 (killed-process case simulated by a copy newer than the save; Recover marks unsaved; Discard deletes; clean exit = no offer), 191 (drag 1 s shortens by 1 s ± 1 frame; source limit; one undo), 192 (0.5 s later; stops at 0.1 s before the end; one undo), 193 (fade boundary 1 → 2; drop on occupied changes nothing; one undo), 194 ([ and ] with markers at 2, 5, 9 s; nothing while typing), 195 (Ctrl+V pastes the clip with text on the system clipboard), 196 (10 steps; unused media removed keeps 10), 197 (three lines Delete and one undo; shift 0.5 s; stop at 0 s), 198 (drop within 1 % snaps and the guide shows; option off = nothing; export unchanged), 199 (rectangles at 90 % and 80 %, not in export, option off = none).
- CI and packaging: none new.
- Performance: autosave clone + serialise of a 1 h project (≤ 10 MB JSON): on the worker; UI frame ≤ 200 ms (gate, harness); interval 2 min (MINOR); disk writes ≤ one per interval.
- Security: autosave folder inside the private cache; the copy is read back with the same bounded, strict loader as a project (untrusted); `.path` text bounded and validated; Discard removes only its own pattern; no path from the copy is opened without the same validation as File > Open.
- Risks: autosave of a large project costs disk time (business risk); undo of removed media may hide a wanted step (accepted by REQ-196); snap may annoy users (manual item 11).
- Manual: items 11 (gestures by hand) and 12 (autosave after a crash, each system).
- Depends on: 0 and 1 (format writer). Cut-back: REQ-198 safe-area and guides to roadmap if the snap feel fails (item 11).
- ADR-173 (autosave), ADR-174 (gestures as UI drag then one command), ADR-175 (history forget rule).

## Stage 3 — whole-video and keyframe clarity (REQ-200..205; batch 7)
- REQs: 200 whole-video popup shows one row per clip (copies apart; amends REQ-148); 201 notice "The new clip has no <effect>" with Apply to new clips; 202 two sets of picture handles (clip solid, whole video dashed; amends REQ-127, REQ-128); 203 keyframe marker ≥ 12 px, pick within 8 px, typed time and value, Left and Right move by one frame; 204 Copy keyframes and Paste keyframes to; 205 same-file LUT double-click no-op.
- Modules:
  - `app/frame.rs`: whole-video copies addressed by clip id in the popup state; Add, Remove, value changes apply to the selected copy; Remove effect removes all copies in one undo.
  - `app/effects.rs`: `after_clip_added(ids)` hook called from append, paste and split; when a whole-video effect is on the timeline and a clip lacks it, AppState holds an offer (status bar with the button); the button dispatches `ApplyWholeToNew` (one undo step; REQ-147 unchanged).
  - `keyframes.rs`: `copy_keys(track) -> KeysClip`, `paste_keys(track, keys)` in clip time (replaces only that effect).
  - NEW `ui/keyframe_timeline.rs`: one widget for Scale, Move (stage 11 adds Volume): marker size, hit test, typed fields, keyboard focus rule (Left and Right only when the popup has focus).
  - `ui/frame_handles.rs`: second handle set (dashed for the whole video); drag changes only the effect it belongs to.
  - `ui/frame_popups.rs`: rows per clip, Copy keyframes, Paste keyframes to (list of other clips).
  - `app/lut.rs`: same-file double-click returns "The clip already uses <file name>" with no undo step.
- Data: none (copies already per clip).
- Tests: 200 (three rows; clip 2 change leaves 1 and 3; Remove effect removes three; one undo), 201 (notice on append; button sets 50 % on the new clip only; undo; without button nothing), 202 (two boxes; corner drag changes only the clip's Scale), 203 (click 6 px from a key selects it; typed value; Right moves one frame with focus), 204 (copy clip 1 Scale, paste clip 3: same clip times; one undo), 205 (same file = no undo step, message; other file replaces). REQ-148 and REQ-127/128 tests changed with their amendments.
- CI: none. Performance: keyframe widget and popups inside the 200 ms UI gate with 200 keys (existing harness).
- Security: none (no new input).
- Risks: keyframe popup usability (manual item 13: rework before release if users struggle); the notice must never change the project by itself (REQ-147).
- Manual: item 13 (three users or the maintainer on three projects).
- Depends on: 0. Cut-back: REQ-203 typed fields only if the widget grows beyond the budget (keep the marker rules).
- ADR-176 (keyframe widget, copies apart, offer).

## Stage 4 — faster hardware export (REQ-206..211; batch 8)
- REQs: 206 check renders only three one-second ranges with the normal method; 207 Faster explanation on integrated graphics; the QG-05 time target per kind (integrated and dedicated); 208 check paused while an export or subtitle job runs, one check process at a time (amends REQ-061); 209 candidate skipped for the session after two time-outs (restart retries); 210 busy (output or CPU) gets the retry, hanging (no output and no CPU) is skipped after its first try; 211 Advanced shows "check inconclusive, repeating".
- Modules:
  - `outcheck.rs`: builds the normal-method reference for the three sample frames only: one-second ranges around each sample (trim from the segment start), not the whole export; the fault list of REQ-063 unchanged.
  - `hwenc.rs`: probe result carries `Activity` per candidate (bytes written by the child, CPU time of the child through sysinfo (existing dependency; API checked in this stage), last output time); classification busy or hanging; session skip set in AppState (not settings).
  - `jobs.rs`: the check is a job at the lowest priority; it is stopped when an export or subtitle job starts and restarted when the last job ends; `Runner` gains `activity` (default unknown, fakes report it).
  - `export.rs` and `app/export_flow.rs`: the check state visible to the Settings text (integrated or dedicated, from the check).
  - `ui/settings.rs`: the Faster explanation sentence (REQ-207) and "check inconclusive, repeating" (REQ-211) texts.
- Data: none (session memory is not stored).
- Tests: 206 (normal-method render covers at most three one-second ranges; each REQ-063 fault still fails), 207 (sentence with simulated integrated graphics; not with dedicated), 208 (export started during a check: no check process until the end; check resumes), 209 (two simulated time-outs: third try skipped without waiting; restart retries), 210 (output after 1.5 s gets the retry and is found; no output and no CPU after 1 s skipped at once), 211 (text during repeat, replaced by result).
- CI: none. Performance: check time budget unchanged (10 s total, repeat ≤ 20 s, REQ-061); QG-05 g measured per class on Andrea's Intel laptop (manual, item 14); the reference render cost at three ranges is the only export overhead.
- Security: no new process type; the check still uses the fixed synthetic lavfi input (ADR-038).
- Risks: busy-or-hanging false positives under heavy load (tests use fake activity only); CPU time API may not be exposed for every OS (fallback: output only).
- Manual: item 14 (Faster and Best quality at 1080p on the Intel laptop).
- Depends on: none in 1–3. Cut-back: REQ-210 to "time-out counts once" if the activity signal is unreliable on macOS or Windows (record in the risks).
- ADR-177 (check: samples-only render, busy or hanging, session memory, pause).

## Stage 5 — hardware reach (REQ-212..219; batch 9)
- REQs: 212 AV1 preset in MP4 (hardware when the check finds an AV1 encoder); 213 VP9 preset in WebM; 214 hardware decode in preview and preview copies with Faster, fallback on failure without a black frame; 215 Quick Sync constant quality when bitrate control is refused, with the 1.5× size guard, not a failure; 216 graphics card list in Advanced, Automatic default, choice in settings (program setting); 217 libva older than 2.21: message with both versions, normal method; 218 360 export with texts under Faster checked or redone; 219 Change aspect, Scale and Move under Faster checked or redone.
- Modules:
  - `hwenc.rs`: `Codec` gains `Av1` and `Vp9`; candidates per vendor (av1_nvenc, av1_qsv, av1_amf, av1_vaapi, av1 VideoToolbox where the probe passes); vp9_vaapi and vp9_qsv where present; VP9 hardware encoders are rare, so software is the usual result (risk stated in the Advanced text); `encode_args` extended; the QSV constant-quality path (REQ-215).
  - NEW `hwdec.rs`: decoder candidates per vendor and OS (cuda or nvdec, qsv, vaapi on Linux, videotoolbox on macOS, d3d11va on Windows); decode probe: a short clip encoded by the bundled software encoder (libx264, libx265, libsvtav1 or libaom-av1, libvpx-vp9: presence checked by `-encoders`) and decoded with each candidate, output checked by frame count; the probe limits of REQ-061 apply per candidate.
  - `format.rs`: `VIDEO_PRESETS` gets the AV1 (MP4) and VP9 (WebM) rows with the existing resolutions and frame rates; a preset is listed only when its encoder exists (`available` table).
  - `preview.rs`: when the Export choice is Faster and a decoder is chosen, the playback spawn gets `-hwaccel` arguments; on a decoder failure the run restarts at the playhead with software decode, the last frame stays on screen (REQ-144).
  - `settings.rs`: `graphics_card` = Automatic or Device(name); the name must match the enumerated list at use (never a raw argument) (ADR-179).
  - `export.rs`, `app/hw_tests.rs`: candidate chosen by the setting; `outcheck` for 360 with texts and for frame effects.
  - `ui/settings.rs`: Advanced list of devices, Automatic, the libva message (REQ-217): the installed and required versions from the library file name found by the existing libva guard (unknown version = "older than 2.21").
- Data: none (settings only).
- Tests: 212 (preset listed only with the encoder; simulated AV1 hardware run passes the check or equals Best format; no hardware = normal method), 213 (as 212 with .webm), 214 (decoder used with Faster, not with Best quality; simulated failure mid-play keeps the picture and falls back), 215 (driver refuses bitrate: CQ export; > 1.5× redone; no failure remembered), 216 (two simulated devices; choosing the second is used next; Automatic restores; survives restart; not in project), 217 (2.20 message with both versions; normal method), 218 and 219 (simulated hardware run with each feature: checked or redone; frame count and format equal Best quality). Codec table tests for AV1 and VP9 argv.
- CI: the probe tests stay fake-based; a CI job runs the software encoders presence check (the bundled ffmpeg has the AV1 and VP9 encoders: test). No new CI runner.
- Performance: the AV1 and VP9 probes add candidates to the probe: total probe bounded by REQ-061 (10 s); hardware decode in preview only with Faster; measured in QG-05 manual items.
- Security: hardware decode of untrusted media in a driver: contained in a child process (ADR-037 class); device path only from enumerated `renderD\d+` nodes; the device name in settings is validated against the enumerated list before use; the decoded output is untrusted media (sw decoder only for checks, unchanged).
- Risks: AV1 and VP9 hardware missing on most systems (speed claims unmeasured, business risk); VideoToolbox AV1 only on recent Apple Silicon (probe decides); Ubuntu 24.04 libva 2.20 (message only, REQ-217; the rebuild stays on the roadmap).
- Manual: item 15 (hardware by system, two-card laptop); items 16–17 belong to stage 7.
- Depends on: 4 (check base), 0. Cut-back: REQ-214 (preview decode) and REQ-216 (choice) as in the cut-back list of the main plan.
- ADR-178 (AV1 and VP9 codecs and decoders), ADR-179 (graphics card choice).
