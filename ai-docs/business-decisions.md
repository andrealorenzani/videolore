# ai-notes: business decisions, defaults and history (owner: product-manager)

Sub-document of [business index](business.md). Human-facing wording of each decision is in `docs/business*.md`; this file keeps the reasons, the MINOR defaults in force, the rejected ideas, the risks and the closed questions. Compacted at RELEASE 2.3.0.

## Standing rules (C, D numbers kept)
- C1 ffmpeg bundled in every installer (user friendliness). C2 offline: the only network use is the consented one-time speech model download (fixed https, SHA-256); no telemetry, no update check. C3 settings are program-level, never in a project.
- D1 fade length: 0.1 s up to half the shorter clip (REQ-274, reversed the fixed length). D2 output format automatic ("Same as source"). D3 layout resizable, hideable, dockable (free moves out). D4 stream copy only when frame-accurate. D5 installers unsigned; code signing not planned (Andrea 2.0.2, re-confirmed 2.0.4 and 2.0.2 release). D6 Linux = AppImage + tar.gz. D7 one window: Windows and Linux use the running instance (REQ-230); macOS = running app + prompt. D8 bundled subtitle font Noto Sans Bold (Latin, Greek, Cyrillic); Arabic and Devanagari bundled (REQ-187/188). D9 music does not ripple. D10 preview copies never used for export. D11/D36 new schema: older versions refuse. D12 volume gain = (percent/100)^2 (REQ-280). D13 text edit drops word timings. D14 macOS menu bar with Hide and Minimize (REQ-232).
- D15 detection in the app at first start; installers do nothing. D16 one effort level (Light, Balanced, Maximum). D17 Settings = in-app floating window. D18 custom settings win on machine change. D19 speech levels Fast, Balanced, Most accurate. D20 older CPUs: message only (fallback engine dropped 2026-11-14). D22 temp files in the cache folder.
- D23-D28 hardware export (0.4.x): Best quality default, Faster opt-in; probe only when needed; silent fallback; output check (decode, metadata, three samples PSNR >= 25 dB); probe limits 1 s + one 4 s retry, 10 s total; time-outs narrowed by REQ-209/210 (skip after two time-outs for the session; busy vs hanging).
- D25 VA-API exception: constant quality + 1.5x size guard. D27 Faster wording "often quicker, but not on every computer".
- D29 one action table drives menus, toolbar, tooltips, shortcuts. D30 layout and recent list = program settings.
- D32 effects per video clip or whole video. D34 a reused track text wins. D35 one preset per project. D36 schema bump (1.2.0).
- D37 keyframes straight unless eased (REQ-282). Change aspect: per clip as its own frame inside the project frame (REQ-286; 2.0.4). Subtitles and texts keep place when the picture moves (REQ-176).
- D38 player = separate program, watch-only; opens from the editor and back (REQ-249/250); playlist and preferences session-only or in its own settings file (REQ-251, 252, 254); never writes next to the video.
- D39 Apply LUT = colour effect; project keeps the path and, up to 20 MB, a copy (REQ-293). D40 icons from a free GPL-compatible set. D41 whole-video effects apply to clips present (REQ-147). D42 whole-video Scale or Move = a copy per clip (REQ-148). D43 player start screen (REQ-150). D44 fixed picture names; summary per version.
- D45 export keeps fixed presets (AV1 and VP9 added 2.0.4, no free entry). D46 hardware export always through ffmpeg, no vendor pipeline, no shipped libva. D47 AV1, VP9 and hardware decoding in scope (built 2.0.4). D48 360 export prioritises speed (fastest preset within 1.5x Best-quality size; REQ-266). D49 whole-video keyframe copies shown apart (REQ-200).
- D50-D56 (2.0.3): D51 text readability None, Outline, Background (old texts open as None). D52 text size cap 10% of height. D53 subtitle placement warns, does not move (REQ-168). D54 accuracy gate at Fast only. D55 old-text migration, no restore command. D56 REQ-177 target stated, not met, open decision for Andrea.
- D57 one development may build several batches as one version (Andrea's instruction, 2.0.4). D58 one project format bump per development. D59 program settings list (window, popups, workspaces, toolbar, shortcuts, theme, text size, language, graphics card, favourite LUTs, splash option, cache limit). D60 English only; texts kept in one table (REQ-245).
- D61 (2.0.4, Andrea): player under "Open with" for six video types, default only by the user's choice (REQ-248). Ambisonic rotation in Rust, yaw and pitch, reduced to stereo, pre-pass (REQ-273). CJK fonts are system fonts only; a note when no font covers the script (REQ-187). AccessKit on Windows, macOS, Linux (REQ-246). Fastest preset within 1.5x Best-quality size (REQ-266). Enhance speech = speech-level normalisation "Normalise speech", no model file (REQ-279). In-house Matroska projection writer, cut to roadmap if the spike fails (REQ-261).
- D62 (2.1.0, Andrea REFINE, option C): a LUT above 20 MB is not copied; the project keeps its path and the REQ-292 choices apply when it is missing. Options not chosen: raised limits (about 100 MB per LUT), resampled copy (lossy).
- D63 (2.2.0): `.cube` route kept with background preparation and progress (REQ-312). Rejected: integer `.m3d` 3.4 s, float `.m3d` 5.1 s, binary HALD 0.36 s (trilinear, colours change), smaller preview table (colours change). The preparation time of about 12 s stays (measured, not gated).
- D64 (2.2.0, Andrea): the 10% text cap stays the default; each text may set an optional maximum, percent of picture height, 2 to 50, empty = cap (REQ-323). Overrides the 1.4.0 no-size-field rule for this one field.

## Closed questions (kept for the reason)
- 2.3.0 (Andrea, REFINE, all closed): Q1 both clip row and whole-video popup (REQ-326; the whole-video popup keeps Fit and empty space). Q2 Fit plus empty space (colour, or blurred video with a blur level), as the whole video. Q3 Volume is a keyframable effect of its own; the Clip details "Volume keys" row goes; 2.2.0 keys migrate unchanged (REQ-280, 332). Q4 Crop removed; old crops dropped with one status line (REQ-283, 332). Q5 Change speed kept as its own effect, 50-200%, not keyframed; old speeds kept (REQ-281, 332). Q6 overlap GIF/MOV plays once then transparent; PNG still (REQ-328). Q7 an edit that removes overlaps shows one warning with the count; OK removes, Cancel cancels the edit; one undo restores (REQ-329). Q8 old Add Picture items become overlaps on the clip under their start; no clip: stays a picture item (REQ-330). Q9 stored "Never" read as "Once for each version" (REQ-295).
- 2.3.0 status-line wording confirmed: "Opening: N clip(s) changed by the conversion (M crop(s) dropped, K picture(s) made overlaps)".
- 2.3.0 deviations accepted by Andrea (written into the texts): split keeps each overlap part on its half, with no popup (REQ-329); overlap lane shown only while overlaps exist, one to three sub-rows visible with wheel scroll (REQ-325); Effects row "Overlap media" shares the first row with "Add Text"; Volume popup has an ease choice per key and Add keyframe at the playhead (REQ-280).
- 2.2.0 (REFINE): Q1 = D64. Q2 Speed fixed first; fades movable in the picture group. Q3 today's order default; a LUT moved above Change aspect grades the fill (user choice).
- 2.1.0: REQ-301 option C (D62). Project format 12.
- 2.0.4: translated interface English only (D60); CJK system fonts (D61); deviations accepted as built: REQ-202 (dashed handles = the copy selected in the open popup), REQ-195 (Copy and Cut put a marker text on the clipboard), REQ-222 (blur at one eighth), REQ-244 (text size in the collapsed header), REQ-248 (Windows run stays a manual check).
- 2.0.4 REQ-280 (volume keyframes) confirmed built with tests (render/tests.rs, v204.rs).

## MINOR defaults in force
- 2.3.0: Add media = File > Import media… (label "Add media", REQ-324); overlap default length 3 s, cut to the clip; start and end clamped to the clip; MOV sound not played; overlap on top of the picture group by default; several overlaps per clip; 16 MP limit applies to overlap files (REQ-331); a 360 export does not draw overlaps; MOV without alpha drawn opaque; Change aspect first choice Fit on width and black (REQ-326); normal edge drag keeps the media's ratio (REQ-327); Ctrl or Command: MOV cropped, GIF and PNG stretched; overlap placed at the playhead, refused with a status line when no clip is under it; Change speed kept first in the order (REQ-320); Change speed rescales overlap times (REQ-329); overlay above 1920 x 1080 previewed at half size (REQ-331).
- 2.2.0: whole-video popup removed; Change aspect not copied (double-click sets the project frame); whole-video mark removed (REQ-303); a copy's Remove is in its own clip popup; the new picture rows take their default place, not the top (REQ-321); picture item limit = 16,000,000 px for PNG only (REQ-316).
- 2.1.0: shadow in px at 1080 px height, default off, colour black; Enlarge word default 12 px (range 1-40 at 1080 px); Typewriter spreads letters over the word time, no caret, no sound; project format 12.
- 2.0.4 (kept): fade default 0.5 s; Clean audio 100% default; blur Medium default; custom ratio 1:4 to 4:1; picture items PNG only, 3 s default; LUT strength 100%; LUT copy 20 MB, 64 MB total, read cap 96 MiB; favourites up to 20; autosave every 2 min to cache; preview cache 4 GB; splash every start by default; 360 frame effects not applied to 360 video (note); sphere size, no preset; roll by Shift+drag; subtitles .vtt and .ass from the same lines; switchable track optional, burned-in default; import is a user action; interface text size 100/125/150%; toolbar icons-only default; theme System default; workspaces max 10; a text dragged past the edge is cut in preview and export.
- 2.0.3 (kept): readability Outline black 6%, Background 60% black, padding 10%; box height 5% to 100%; overlap warning list; piece overlap 120 s + 2 s; paraphrase message at half of the lines.

## Rejected or considered (do not re-propose without new reason)
- Per-version splash pictures; one summary for both programs; blocking splash.
- Player that edits or exports; player playlist saved in the project.
- LUT on the frame background (decision: not changed); bake LUT as a second track.
- Whole-video keyframes as one timeline path with ripple; a whole-video Scale or Move popup.
- Crop-to-fill both modes (1.5.0); per-clip aspect as a copy of the whole video (1.5.0; built as own frame in 2.0.4).
- Keyframes for effects other than Scale, Move and Volume (2.0.4 scope; text shadow still open in roadmap).
- Colour for the Scale empty space.
- Keeping the old whole-range rule for old files (1.6.1).
- Export settings UI with technical choices (1.4.0); codec names only in the tooltip.
- Slower speech engine for old processors (dropped 2026-11-14).
- Parallel executables job with the checks (2026-11-19).
- Code signing and notarisation; custom export resolutions, frame rates, bitrates and encoder options (Andrea 2.0.2 and 2.0.4).
- A separate vendor pipeline; shipped libva; rebuilt ffmpeg for Ubuntu 24.04 (not built, MINOR).
- Colour management for LUTs (no colour-workflow rule yet); shapes on the picture; exact conversion of combined Scale and Move (conditional on a user report).
- Accuracy methods measured and removed: DTW, one-sided onset snap, speech-region method (2.2.0 measured 43%).
- Stereo 3D, cube maps, 180 video (out of scope; roadmap).
- Translation beyond English (D60).
- 2.2.0 (D63): integer and float `.m3d`, binary HALD, smaller preview table.
- 2.2.0: whole-video mark and whole-video popup (replaced by copies).
- 2.2.0 (ADR-192 parts): picture item count limit 20, file size limit 8 MiB, embedded picture copies (only the 16 MP limit was chosen).
- 2.3.0: overlap as a free object on the picture (contradicts the one-clip rule); a separate video track for overlaps (multiple tracks out of scope); copying the picture into the project (not requested); Fit-only clip row (Q2 option b); keeping Crop (Q4 option a or c).

## Product risks (current, after 2.3.0)
- R1 alpha video: MOV with alpha and GIF transparency must decode with alpha; hardware decoders may drop it, so overlaps use the software path where needed (roadmap 3.0.2, 3.0.5).
- R2 project format 14 is refused by 2.2.0 and older; the release notes must say so.
- R5 migration of 2.2.0 projects (volume keys, speed, crop): covered by REQ-332 tests; a real 2.2.0 file check is advisable (not confirmed in the docs).
- R6 the clip Fit default (width vs height) changes a clip's picture only when the user picks again.
- R8 Change speed "all clips" is a new use of the copy rule for a non-keyframed effect.
- Measured limits users may meet: REQ-315 (about 9% against 20% on one Intel computer); REQ-177 (43-82% against 90%); overlays above 1920 x 1080 at half size in preview; a 4000 x 4000 PNG overlay about 830 MB in export.
- Unsigned installers: one-time confirmation on first launch.
- Wayland: drops do not work where the library does not report them.
- Autosave writes to the cache every 2 minutes; large projects may cost disk time.
- Accessibility support may cost frame time; check with the quality gates.

## Assumptions
- Measured on the developer's computers only: 1% dropped frames for 60 s at 1080p (REQ-314 gate), the 4K blur step, Iris Plus G7 target as the low end of the QG-05 estimate (REQ-315).
- Speech-recognition alignment tests run on the development machine only; the CI version is to confirm.
- Project format numbers as assigned by the code (14 for 2.3.0).
