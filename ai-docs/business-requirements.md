# ai-notes: business requirements (owner: product-manager)

Sub-document of [business index](business.md). Requirement texts live in the human docs (`docs/business*.md`); this file is the map and the short meaning of the early requirements.

## Status and map
- Implemented: all REQ-001..176, 178..179, 180..299, 300..311, 312..314, 316..323, 324..333 (all `(implemented)` after 2.3.0). 309 stays implemented and is fixed by REQ-333. REQ-283 is removed (line kept, "removed in 2.3.0").
- Planned: REQ-177 (accuracy target, not met) and REQ-315 (integrated graphics, not met). Both carry their measured result in the text.
- Next free ID: REQ-334.
- Amended in place in 2.3.0 (texts in the human docs): 280 (Volume keyframed effect), 281 (Change speed), 286 and 326 (Fit and empty space), 287 (Add Picture replaced by Overlap media), 295 ("Never" removed), 320-322 (order: no Crop, Overlap media in, Change speed fixed), 332 (conversion of 2.2.0 projects).
- Earlier amendments kept: 076, 083, 116, 129, 131, 147, 148, 156, 157, 200-202, 302, 303, 304, 311 (2.2.0); 300, 301, 303-310 (2.1.0); 183, 195, 202, 222, 244, 248, 253, 257-260, 268, 273, 281, 287, 293 (2.0.4 as built).

## REQ -> human doc (grep the ID in `docs/business*.md`)
- editing: 001-013, 026-029, 032-035, 038, 039, 083, 094, 105, 113, 123, 144, 149, 155, 324 (Add media); gestures and autosave 189-199 (editing-gestures)
- overlap: 325, 327-331 (overlap)
- effects: 075, 076, 077, 082, 107, 147; 274-294 (effects-frame; 283 removed); 332 conversion (effects-frame)
- applied effects: 302-305, 311, 320-322 (applied-effects)
- subtitle styles: 306-310, 333 (subtitle-styles)
- ui: 024, 025, 066-074, 078-081, 093; interface 234-247 (interface)
- subtitles: 014-018, 044, 085-092, 095; placement 168-176; subtitle files 180-188 (subtitle-files)
- subtitle accuracy: 177 (planned), 178, 179 (subtitle-accuracy)
- speed: 312-316 (speed)
- music: 019, 020, 043, 084, 114
- export: 021-023, 036, 037, 055, 060-065, 118-122; speed and format names 220-224 (export)
- hardware: 206-219 (hardware-export)
- keyframes: 200-205 (keyframes)
- text: 108-111, 115, 116, 156-167 (text)
- picture: 096-099, 100-106, 112, 117, 146 (picture)
- 360: 257-273 (360-video)
- frame: 124-132, 145, 148; 326 (frame)
- lut: 135-137, 300, 301 (lut)
- player: 138-141, 150, 248-256 (player)
- splash: 133, 134, 154, 295 (splash)
- icons: 142, 143
- install: 030, 031, 040-042
- platforms: 225-233 (platforms)
- performance: 045-054, 056-059, 151-153; QG-01..10 without covers (performance)
- quality: 296-299 (quality); QG-11..18 without covers
- verification: no REQ (manual checklist)

## Coverage rule
Every implemented REQ has a `// covers:` test (feature-coverage 100%, checked by the Principal Engineer).

## Early requirements (short meaning, 0.2.0 wording; current text in the human docs)
- 001 non-destructive: sources never modified; edits are project data.
- 002 import video and audio with name and duration; unsupported = clear rejection; audio placeable on the music track.
- 003 save and reopen; missing sources reported by file name, project still opens.
- 004 undo/redo of at least 50 steps, covers music, mix and subtitle edits.
- 005 append to the end; 006 split at playhead; 007 cut/copy/paste with ripple; 008 merge contiguous segments of the same source; 009 trim within source bounds; 010 reorder with ripple.
- 011 fade through black per side (min 0.5 s, clip/2), never overlapping.
- 012 zoom from fit-all to frame level; 013 preview play/pause/seek shows effects, music and subtitles.
- 014 offline subtitles from video audio; background, progress, cancel; model download with consent (the only network use). 015 language detected, overridable. 016 lines added, deleted, re-texted, re-timed, undoable. 017 at least three presets incl. word highlight; text size scales with height. 018 subtitles tied to the speech (cut removes, move moves, trim hides, split keeps).
- 019 one music track below the video; clips never overlap; audio files only. 020 music mixed with video audio; separate volumes 0-100% and mute; saved in project.
- 021 export with progress; cancel leaves no partial file. 022 automatic output format (identical -> same, else lowest pixels, fps, bitrate; letterbox or pillarbox). 023 stream copy only when identical format, no effects and keyframe cuts; else one frame-accurate re-encode.
- 024 layout (menu, toolbar, content, status); 025 every operation in the menus.
- 026 Save and Save As; 027 title with unsaved marker; 028 prompt on New, Open, close or quit when changed.
- 029 0.1.x projects open identically; newer files refused, left untouched.
- 030 import, preview and export work without a system ffmpeg; user override kept. 031 About and third-party notices for every bundled component.
- 032 import never freezes the window; 033 ffmpeg processes capped by effort level; 034 preview re-rendered only when the picture changes; 035 video runs in real time if audio stalls.
- 036 HEVC in MP4 and MOV tagged hvc1; 037 200 segments export frame-accurate.
- 038 preview copies (lower-resolution) for big sources; export always uses the originals; cache outside the project.
- 039 memory does not grow with the source file size.
- 040 one double-click installer per OS, no terminal, ffmpeg included. 041 app icon; project file association. 042 installers built per release, unsigned, README first-launch steps.
- 043 music does not move with video edits; music past the video end cut with 0.5 s fade-out. 044 regenerating subtitles asks before replacing manual edits.
