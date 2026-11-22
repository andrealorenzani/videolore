# ai-notes: business requirements (owner: product-manager)

Sub-document of [business index](business.md). Requirement texts live in the human docs (`docs/business*.md`); this file is the map and the short meaning of the early requirements.

## Status and map
- 2.2.0 PLAN (2026-11-21, not built): REQ-312..316 `(planned)` (speed doc). Amended `(planned)` in place: 076, 083, 129, 131, 147, 148, 177, 200, 201, 202, 303 (whole-video rule, REQ-177 build). REQ-300 wording fixed (implemented, no status change); REQ-305 accepted; REQ-287 unchanged (316 amends it). STATUS 2.2.0 done (2026-11-22): 27 lines marked implemented (see the checkpoint in ai-docs/business.md); REQ-315 and REQ-177 stay planned with measured results. Next free ID: REQ-324. Added by GO (2.2.0 part 2): REQ-317 (widget column), 318 and 319 (text box governs size; old sizes), 320 to 322 (order of all effects, undo and copies, storage), 323 (optional per-text maximum size, D64); amended planned: 116, 156, 157 (text), 302, 304, 311 (applied effects).
- 2.1.0 STATUS done (2026-11-21, version 2.1.0): REQ-300..311 `(implemented)`, each with a `// covers:` test (grep-checked, no test run by the PM); amended to the built behaviour (300, 301, 303, 304, 305, 306-310, 311). REQ-301 answered: option C, D62. Next free ID REQ-312.
- REQ-001..REQ-176 and REQ-178..179 are `(implemented)` (2.0.3). REQ-177 stays `(planned)` (accuracy target not met; open decision for Andrea). 2.0.4 STATUS done: all REQ-180..299 are `(implemented)` (120 requirements; each has a `// covers:` test, grep-checked); REQ-177 is the only 2.0.4 item left planned. Next free ID: REQ-300. QG-01..QG-05 implemented; QG-06 (CI time) open, to read from the first CI run; QG-07..QG-10 delivered 2.0.2; QG-11..QG-13 measured in 2.0.4 (no covers); QG-14..QG-18 built in 2.0.4 (no covers; egui_kittest not adopted).
- REQ -> human doc (grep the ID in `docs/business*.md`):
  - editing: 001-004, 005-013, 026-029, 032-035, 038, 039, 083, 094, 105, 113, 123, 144, 149, 155; 2.0.4 gestures and autosave 189-199 (editing-gestures)
  - ui: 024, 025, 066-074, 078-081, 093; 2.0.4 interface 234-247 (interface)
  - effects: 075, 076, 077, 082, 107, 147; 2.0.4 effects and frame 274-294 implemented (effects-frame)
  - subtitles: 014-018, 044, 085-092, 095; placement 168-176; 2.0.4 subtitle files 180-188 (subtitle-files)
  - subtitle-accuracy: 177 (planned, amended 2.2.0), 178-179
  - speed (2.2.0, planned): 312-316 (speed)
  - music: 019, 020, 043, 084, 114
  - export: 021-023, 036, 037, 055, 060-065, 118-122; 2.0.4 speed and format names 220-224 (export)
  - hardware (2.0.4): 206-219 (hardware-export)
  - keyframes (2.0.4): 200-205 (keyframes)
  - text: 108-111, 115, 116; readable text 156-167
  - picture: 096-099, 100-106, 112, 117, 146
  - 360 (2.0.4): 257-273 implemented (360-video)
  - frame: 124-132, 145, 148
  - lut: 135-137; 2.1.0 import and large LUTs 300-301 (implemented, business-lut)
  - 2.1.0 (implemented): applied effects 302-305 and project format 311 (applied-effects); shadow, highlight colours, Enlarge word, Typewriter 306-310 (subtitle-styles)
  - player: 138-141, 150; 2.0.4 opening, prefs, playlist 248-256 implemented (player)
  - splash: 133, 134, 154; 2.0.4 show option 295 (splash)
  - icons: 142, 143
  - install: 030, 031, 040-042
  - platforms (2.0.4): 225-233 (platforms)
  - performance: 045-054, 056-059, 151-153 (+ QG-01..10 without covers)
  - quality (2.0.4): 296-299 (quality; QG-11..18 without covers)
  - verification: no REQ (manual checklist; 2.0.4 items 9-28 for the manual parts of the new REQs)
- Amendments still pending (old text stays until built): 011, 020, 033, 038, 041, 061, 069, 071, 075, 082, 083, 084, 096, 100, 101, 102, 103, 104, 112, 114, 120, 124, 126, 127, 128, 129, 130, 131, 133, 135, 136, 140, 147, 148 (only where the built REQ text is not yet written into the human doc; STATUS 2.0.4 did not apply them, see the checkpoint). Amendments written into built 2.0.4 lines at STATUS: 183 (import replaces range), 195 (clipboard marker), 202 (dashed-handle reading), 206 (no change, spec confirmed), 222 (eighth), 244 (collapsed header), 248 (Windows template), 279 (speechnorm), 293 (LUT limits 20/64 MB, 96 MiB read cap). 182 gained the built default flag. Second STATUS (2.0.4, same day): 253 (picture limits, PGS fixture), 257-260 and 268 (built form), 273 (detection, no roll, pre-pass), 281 (timeline length, no stream copy, merge), 287 (text item with a picture, 360 skip, missing file stops export), 177 (speech fixture wording, still planned).
- Coverage rule: every implemented REQ has a `// covers:` test (PE, feature-coverage 100%).
- 2.0.4 batches: 5 subtitle files 180-188; 6 gestures 189-199; 7 keyframes 200-205; 8 hardware check 206-211; 9 hardware reach 212-219; 10 export speed 220-224; 11 platforms 225-233; 12 interface 234-247; 13 player 248-256; 14 360 257-273; 15 effects 274-294; continuous 295-299.

## Early requirements (short, 0.2.0 wording; current text in the human docs)
- 001 non-destructive: sources never modified; edits are project data.
- 002 import media (video + audio) with name and duration; unsupported = clear rejection; audio-only placeable on music track.
- 003 save and reopen project; missing sources reported by file name, project still opens.
- 004 undo/redo >= 50 steps, covers music, mix and subtitle edits.
- 005 append media to the timeline end. 006 split at playhead. 007 cut/copy/paste with ripple (no gaps). 008 merge contiguous segments of same source. 009 trim within source bounds. 010 reorder with ripple.
- 011 fade through black per side (min 0.5 s, clip/2), never overlaps; now per clip (1.2.0/1.6.1 rules, see decisions).
- 012 zoom slider fit-all .. frame level. 013 preview play/pause/seek, shows effects, music and subtitles.
- 014 offline subtitles from video audio only; background, progress, cancel; model download with consent (only network use). 015 language auto-detected, overridable. 016 lines added/deleted/re-texted/re-timed, undoable. 017 >= 3 presets incl. word highlight, text size scales with height. 018 subtitles tied to source speech (cut removes, move moves, trim hides, split keeps).
- 019 one music track below video; clips never overlap; audio files only. 020 music mixed with video audio; separate volumes 0-100% + mute; saved in project.
- 021 export with progress; cancel leaves no partial file. 022 auto output format (identical -> same, else lowest pixels, fps, bitrate; letterbox/pillarbox). 023 stream copy only when identical format, no effects and keyframe cuts; else one frame-accurate re-encode.
- 024 layout (menu, toolbar, content, status); 025 every operation in menus.
- 026 Save/Save As; 027 title with unsaved marker; 028 prompt on New/Open/close/quit when dirty.
- 029 0.1.x projects open identically; newer files refused, untouched.
- 030 import/preview/export work without system ffmpeg; user override kept. 031 About and third-party notices for every bundled component.
- 032 import never freezes the window. 033 ffmpeg processes capped by effort level (050). 034 preview re-rendered only when the picture changes, bursts coalesced. 035 video runs real time if audio stalls.
- 036 HEVC in MP4/MOV tagged hvc1. 037 200 segments export frame-accurate.
- 038 proxies (lower-res preview copies) for big sources; export always originals; cache outside project.
- 039 memory does not grow with source file size.
- 040 one double-click installer per OS, no terminal, ffmpeg included. 041 app icon; .vlproj association; double-click opens it. 042 installers built per release, unsigned, README first-launch steps.
- 043 music does not move with video edits; music past video end cut with 0.5 s fade-out. 044 regenerating subtitles asks before replacing manual edits.
