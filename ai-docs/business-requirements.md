# ai-notes: business requirements (owner: product-manager)

Sub-document of [business index](business.md). Requirement texts live in the human docs (`docs/business*.md`); this file is the map and the short meaning of the early requirements.

## Status and map
- REQ-001..REQ-176 and REQ-178..179 are `(implemented)` (2.0.3 STATUS: 155..176, 178, 179 built in the 2.0.3 development). REQ-177 stays `(planned)`: accuracy target not met (Fast: 23 of 28 words matched, 0.48 s average start error; DTW, one-sided onset snap and speech-region method measured and removed; strict test ignored). Open question for Andrea in business-decisions.md. Next free ID: REQ-180. QG-01..QG-05 implemented; QG-06 (CI time) open, not reported by the PE; QG-07..QG-10 (2.0.2, maintainer goals, no covers) delivered in 2.0.2, CI changes not yet verified on a real runner.
- REQ -> human doc (grep the ID in `docs/business*.md`):
  - editing: 001-004, 005-013, 026-029, 032-035, 038, 039, 083, 094, 105, 113, 123, 144, 149, 155
  - ui: 024, 025, 066-074, 078-081, 093
  - effects: 075, 076, 077, 082, 107, 147
  - subtitles: 014-018, 044, 085-092, 095; placement 168-176
  - subtitle-accuracy: 177 (planned), 178-179 (new doc)
  - music: 019, 020, 043, 084, 114
  - export: 021-023, 036, 037, 055, 060-065, 118-122
  - text: 108-111, 115, 116; readable text 156-167
  - picture: 096-099, 100-106, 112, 117, 146
  - frame: 124-132, 145, 148
  - lut: 135-137
  - player: 138-141, 150
  - splash: 133, 134, 154
  - icons: 142, 143
  - install: 030, 031, 040-042
  - performance: 045-054, 056-059, 151-153 (+ QG-01..10 without covers)
  - verification: no REQ (manual checklist; automated parts point to REQ-029, 114, 121, 103, 133-134, 138-141, 144, 150, 154)
- Coverage rule: every implemented REQ has a `// covers:` test (PE, feature-coverage 100%).
- Batch 1 "Verify the release" (version 2.0.2, STATUS done): no new REQ for test reliability or bundled-ffmpeg checks; they test existing REQ-029, 103, 114, 121, 144. Only REQ-154 is new (observable splash behaviour). REQ-144 has a known defect (last ~20 ms of a clip, no paused picture): still `(implemented)`, fix is Future work.

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
