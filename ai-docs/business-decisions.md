# ai-notes: business decisions, defaults and history (owner: product-manager)

Sub-document of [business index](business.md). Compacted in 1.6.3 RELEASE (from the per-version PLAN/STATUS logs). Human-facing wording of each decision is in `docs/business*.md`; this file keeps the reasons, the MINOR defaults still in force, the rejected ideas and the risks.

## Standing rules (C, D numbers kept)
- C1 ffmpeg bundled in every installer (0.2.0 reversal; reason: user friendliness). GPL builds for x264/x265; source offered per release; licence compliance not lawyer-reviewed (assumption).
- C2 offline: only network = consented one-time speech model download (fixed https, SHA-256). No telemetry, no update check.
- C3 settings are program-level, never in project.
- D1 fade fixed length. D2 output format automatic by default (1.4.0: Export dialog, default "Same as source"). D3 layout fixed except resizing (1.1.0 reversal: resizable; move/dock/hide OOS). D4 stream copy only when frame-accurate. D5 installers unsigned, signing deferred not rejected. D6 Linux = AppImage + tar.gz. D7 double-click project while running: second window on Win/Linux, macOS = running app + prompt. D8 bundled subtitle font Noto Sans Bold (Latin/Greek/Cyrillic). D9 music does not ripple. D10 proxies never used for export. D11/D36 new schema = older versions refuse. D12 volume curve gain=(p/100)^2. D13 text edit drops word timings (even spread). D14 macOS default app menu off (Cmd+Q prompt).
- D15 detection in the app at first start, installers do nothing (A1 still pending: could also run in installers). D16 single effort level (Light/Balanced/Maximum), Light = per-job half cores (confirmed). D17 Settings = in-app floating window (confirmed). D18 custom settings win on machine change. D19 speech levels Fast/Balanced/Most accurate (confirmed). D20 older CPUs: message only, slow fallback engine DROPPED 2026-11-14 (out of scope). D22 temp files in <cache>/videolore/work/.
- D23-D27 (0.4.0) hardware export: BQ default, Faster opt-in; all vendors; probe only when needed; hw targets sw bitrate (VBR); silent fallback; output check (decode, metadata, 3 samples PSNR >= 25 dB, Architect threshold). Quality bar S4 3 VMAF / 1.5x size. D25 exception VA-API: constant quality + 1.5x size guard (driver refuses bitrate). D27 Faster wording "often quicker, but not on every computer".
- D28 (0.4.2) probe limits 1 s + one 4 s retry per candidate, 10 s total, one automatic full recheck if inconclusive; time-outs never remembered.
- D29 one action table drives menus, toolbar, tooltips, shortcuts window. D30 layout and recent list = program settings. D31 drag&drop where supported (Win, macOS, X11); Wayland limitation.
- D32 (1.2.0) effects per video clip (drag/right-click) or whole video (double-click); single click = popup only. Effects on music OOS. D33 (1.2.0) start/end fades attached to video edges (REVERSED 1.6.1, see below). D34 reuse track: track text wins, STT only times words, always degrades to even spread. D35 one preset per project; position per line as fraction of picture. D36 schema bump v3 (1.2.0).
- D37 (1.5.0) keyframes are straight lines, no easing. Change aspect = whole video only (one frame per project). Subtitles and text keep place when picture moves.
- D38 (1.6.0) player = separate program, own build and installers; watch-only; opens video only, refuses .vlproj; shares 360 and subtitle-move code with editor.
- D39 (1.6.0) Apply LUT = only colour effect; project keeps the LUT path, not a copy.
- D40 (1.6.0) icons from a free GPL-compatible set (MIT/ISC/BSD/Apache-2.0), bundled.
- D41 (1.6.1) whole-video effects apply to each clip present at that moment and stay with it; clips added later get none (REQ-147). Whole-video fades = first clip start and last clip end (reverses D33; fades move with their clip).
- D42 (1.6.1) Q1=A: whole-video Scale/Move = each clip gets its own copy of the keyframes in clip time (REQ-148). Q-F (1.6.1 REFINE 2): clip with own Scale/Move + whole video = one combined path (REQ-149 (a)); clip's own LUT wins over whole-video LUT (REQ-149 (b), REQ-135).
- D43 (1.6.2) player has a start screen like the editor (REQ-150), reversing the 1.6.0 "no splash in player" rule. Each program has its own summary.
- D44 (1.6.2, names in 1.6.3) pictures have FIXED names (`splash.gif`, `splash-player.gif`); only the summaries (`summary.txt`, `summary-player.txt`) change per release. Build fails without them.
- D5 confirmed 2.0.2 (Andrea): installers stay unsigned; code signing and notarisation are NOT PLANNED (not merely postponed).
- D45 (2.0.2, Andrea) Export keeps the fixed presets and "Same as source"; custom resolutions, frame rates, bitrates, encoder options are not planned (roadmap item to Not planned at release).
- D46 (2.0.2, Andrea) Hardware export always goes through ffmpeg (bundled or user-chosen); no separate vendor pipeline.
- D47 (2.0.2, Andrea) AV1 and VP9 encoding and hardware decoding are IN SCOPE (replaces the 0.4.0 out-of-scope lines); requirements come with the batch that builds them, not now.
- D48 (2.0.2, Andrea) The 360 export always prioritises speed (detail open, see MINOR 2.0.2 and Open questions; REQ-103 unchanged until its batch).
- D49 (2.0.2, Andrea) Whole-video keyframe copies are shown apart (one per clip) in the popup; popup change not built; REQ-148 stands until then.
- Format: each project format bump refuses older versions; 1.6.1 format converts 1.6.0 whole-video effects once on open (one status note).

## MINOR defaults still in force (Andrea may overrule)
- 2.0.2 (implemented, STATUS): REQ-154 splash summary: largest size 15 px down to floor 7 px (code SUMMARY_MAX/SUMMARY_FLOOR), never scrolled; a shipped summary below the floor fails the suite (the code returns the floor and the text may overflow, so the test is the guard). User-visible fallback for a summary that cannot fit even at 7 px is not defined (no shipping of it).
- 2.0.2: batch 1 items with no user-visible behaviour are QG-07..QG-10 (maintainer goals, no REQ, no covers), not REQs. Items needing a real system are a manual checklist in docs/business-verification.md, not blockers; result recorded at the next release.
- 2.0.2: the paused frame at a fade gets a test under the existing REQ-144 (no new REQ); the bundled-ffmpeg checks test REQ-103, 114, 121 (no new REQ).
- 2.0.2 STATUS [MINOR]: paused still in the last ~20 ms of a clip returns no frame (1.98 s of a 2 s 25 fps clip; 1.92 s works). Verdict: implementation defect against REQ-144, not a requirement-wording bug (REQ-144 already says the picture at the playhead wherever the timeline has picture). REQ-144 stays `(implemented)` (its tested cases pass); gap recorded as known limitation in business-editing.md and as Future work (fix item, test at clip end and last frame, under REQ-144, no new REQ). Default if Andrea wants it counted as not delivered: mark REQ-144 `(planned)` until fixed.
- 2.0.2: 360 export "always prioritises speed" (Andrea) is recorded as decision D48; its meaning (which path, whether REQ-103 gets a speed acceptance) is open until the batch that changes the 360 export is planned. [MINOR] default until then: REQ-103 stands as written.
- 2.0.2 batch numbering: batch N of roadmap_tasks ships as 2.0.(N+1); the batch "Verify the release" is 2.0.2, so "Readable text" is 2.0.3 and later batches shift by one at the next release (Documenter/PM to renumber roadmap_tasks).
- 1.6.2: summary per program; player splash rule = editor rule (no splash with a path on the command line; not blocking; on top until closed); rectangle read as x 78-386, y 388-462 top-left origin; "Version X.Y" centred, largest fit; text length kept to five lines; popup text smaller if it does not fit (M6, ASSUMPTION, check at build).
- 1.6.1: Change aspect stays project-wide (M2, exception to "only one exception" wording); whole fades attach to first/last clip; 1.6.0 projects get the same effect on existing clips at load; no second copy of an effect on a clip; edge drag proportional, 10-400%, one undo; REQ-144 fix only, no preview redesign.
- 1.6.1 Q1 details: whole-video popup shows first clip's keys; Add key goes into each copy long enough; value change sets key in each copy; Remove removes from each copy (keeps last key); one undo for all copies; copies follow trims like clip keys.
- 1.6.1 REQ-135: a double-click giving a clip a LUT it already has replaces it (decision in roadmap as open).
- 1.6.1 REQ-149 max 200 keys: own keys first, then shifted whole keys in time order; 0 and clip length always kept (approximation rule, human-visible).
- 1.5.0: D3 Fit on height = picture height fills frame, sides cut evenly (not "bars"); D4 empty space = frame background; D5 4096 px cap (codec limits; ASSUMPTION); D8 any effect present = re-encode; D9 360 export ignores the three effects; D10 clip over whole video stacks (scales multiply, moves add).
- 1.4.0: Clean audio = video sound only, fixed strength, before clip volume, marker at clip start; thresholds 6 dB / 3 dB (ASSUMPTION, measured later: -9 dB noise, speech +-0.2 dB on fixture); text box right-edge handle, default 80%W x 20%H, min 2% H; Effects menu = effects only (item commands to Edit menu); 360 switch removed (metadata only; a 1.3.0 switch dropped on open); presets 12 (fixed table, offered only when encoder present); project format 1.4.0 bump; export choices not stored in project (session only).
- 1.3.0: text items on own lane, no ripple; text effect times 0.1-5 s, default 0.5 s; Add Text TTL 3 s, white, lower third, 5% height; Context Menu hidden (not greyed) when nothing to act on; Shift+click = Ctrl/Cmd+click; 360 text anchored at add time, not drawn outside view; 360 + flat in one project: 360 choice refused, flat camera view allowed; Delete removes text; text not in .srt; text max length = subtitle limit.
- 1.2.6: export = flat (not 360 output) [biggest assumption]; boxes generic, only subtitle lines movable today; align = centre on picture; size per line 25-400%; 360 detected by metadata + manual switch (removed 1.4.0); POV = direction + zoom, no roll; POV max 200; 360 counts as 1920x1080 for REQ-022; ambisonic audio not rotated; AC tolerance 1 degree / 2% H.
- 1.2.0: ruler right edge; drops video clips only; click = popup, double-click = whole video; 0-100 volume, no boost; subtitle positions belong to line; fraction clamped 2%; asked every time when track exists; one subtitle job at a time; alignment threshold 50%; fallback even spread + message; all or nothing on cancel; SRT separate command; schema bump; background re-check on open.
- 1.1.0: mins Media/Effects 160 px, Preview/Timeline 240 px, rows 100 px; recent = 10; overlap chooser; fade marker click opens popup; popups non-modal; layout persisted + Reset.
- 0.3.0 and 0.4.x: see D15-D28 above (Settings wording: banned technical words in main section, case-insensitive whole words).
- 1.6.2/1.6.3 length goal: summary body 129 words vs 1.6.0 52 words (2.5x, inside 2-3x goal; applied to editor only).
- 2.0.1 (implemented, STATUS): version 2.0.1 = module-cycle refactor (user's choice), not roadmap batch 1 "verify the release"; batch 1 items are Future work and move to a later version. REQ-151..153 only (no change for users; cycle gone; cycle check for maintainers), all implemented. Refactor adds no migration, notice, format bump or setting. Tests: existing REQ-057/059 tests must pass unchanged. Roadmap item "Break the module cycles" (Priority 11) is removed by the Documenter after STATUS.

## Rejected or considered (do not re-propose without new reason)
- Per-version splash pictures (copy each release); one summary for both programs; splash once per version; blocking splash; splash in player in 1.6.0.
- Player opening projects or editing; player playlist in 1.6.0; file association in 1.6.0.
- LUT copy into the project (1.6.0, reconsidered later as roadmap item); LUT strength slider (1.6.0); LUT on frame background (1.6.0).
- Whole-video keyframes as one timeline path with ripple (Q1 option B); no whole-video popup for Scale/Move (Q1 option C); bake LUT as second track (Q-F option C).
- Drop whole-video value on the clip when the clip has its own LUT (Q-F option B).
- Crop-to-fill both modes; free aspect ratios; per-clip aspect (1.5.0); keyframes for other effects; easing; blur strength setting; colour for Scale empty space.
- Keep the old whole-range rule for old files (1.6.1 M4).
- Single-instance detection (no IPC, no lock) for second window.
- Export settings UI with technical choices (1.4.0 chose Andrea's plain example wording; codec names remain a roadmap item).
- Slower speech fallback engine for old processors (dropped 2026-11-14).
- Merging the Product Manager and Architect notes (ownership rule).
- Parallel executables job with checks (2026-11-19).
- Code signing and notarisation (2.0.2, Andrea: not planned; was "postponed").
- Custom export resolutions, frame rates, bitrates and encoder options (2.0.2, Andrea: presets kept).

## Product risks (current)
- Stale splash summary per program (release without update); longer summary may not fit the popup (smaller text, check at build).
- Users expect a new clip to get a whole-video effect (documented rule, no message yet).
- Reused LUT path lost when file moves (export stops until found).
- LUT colour differs from other tools (no colour management).
- Player scope creep (playlist, file association expected).
- Frame effects make export a re-encode (slow on big frames); 360 export ignores them (may confuse).
- Hardware Faster can be slower on integrated graphics; check cost.
- Unsigned installers stay (decided 2.0.2): first-launch warnings on macOS and Windows remain for every release.
- Discoverability of Ctrl/Cmd+click and Shift+drag.
- Paused picture missing in the last ~20 ms of a clip (REQ-144 defect, known limitation until fixed).

## Open questions (MINOR, not blocking)
- REQ-134 says "the GIF plays as it is", but the picture is static (one frame) in build. Requirement or picture to change: open (roadmap Priority 12).
- QG-06 CI time: not reported; wording "planned 1.6.1" left in business-performance (no REQ, no covers).
- 360 export "always prioritises speed" (D48): which path and whether REQ-103 gets a speed acceptance; decide in the batch that changes the 360 export (roadmap batch 14, 2.0.15 after shift).
- Splash picture origin and licence: resolved (own work, MIT, recorded in the notices since 1.6.3); not an open question.
- Whole-video copies: resolved 2.0.2 (D49, shown apart); the popup change is still to be planned.
