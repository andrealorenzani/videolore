# ai-notes: business

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

## State
- Target version 0.1.3. Next free ID: REQ-026. Never reuse IDs.
- impl in 0.1.3: 001-013, 021-025. planned: 014-018 (subtitles, target 0.1.5), 019-020 (music, target 0.1.4).

## Product
- videolore: desktop, offline, non-destructive video editor for creators of short social clips. Simple fixed UI, fast export, (later) auto subtitles + music.
- Users: U1 social content creator (non-expert, wants speed, captions); U2 hobbyist editing personal clips; not pro editors (no multitrack video, no color grading).

## Requirements (short)
- 001 non-destructive: sources never modified; edits = project data.
- 002 import media list: show name+duration; unsupported -> clear rejection msg. Audio-only accepted in list; placeable only on music track once 019 exists (in 0.1.3: listed, not placeable on video timeline).
- 003 save/reopen project; missing sources reported by file name (project still opens).
- 004 undo/redo >= 50 steps.
- 005 append media to timeline end.
- 006 split at playhead.
- 007 cut/copy/paste segments, ripple (no gaps).
- 008 merge contiguous segments of same source & contiguous source ranges; action disabled otherwise.
- 009 trim segment edges within source bounds.
- 010 reorder segments, ripple.
- 011 transition = fade-out + fade-in through black, video+audio, removable. Length per side = min(0.5 s, clip_len/2); i.e. fixed 0.5 s when clip >= 1 s. Never overlaps.
- 012 zoom slider: fit-all .. frame level.
- 013 preview: play/pause/seek, shows effects.
- 014 auto offline subtitles (speech model). Model downloaded once after explicit consent OR user supplies local file; afterwards fully offline. ONLY network access of the product.
- 015 language auto-detect + manual override.
- 016 edit subtitle text/timing.
- 017 >= 3 social style presets incl. word-by-word highlight; burned into video.
- 018 subtitles follow timeline edits.
- 019 single music track, clips no overlap.
- 020 music mix, separate volumes video audio vs music.
- 021 export w/ progress; cancel -> no partial file left.
- 022 auto output format: all identical -> same; else lowest by pixels, then fps, then bitrate; letterbox/pillarbox, never stretch.
- 023 (reworded w/ Architect) minimal processing: stream copy iff all clips identical format AND no effects (fades; later subs/music) AND every cut on keyframe; else single re-encode pass, always frame-accurate.
- 024 fixed 5-panel layout: top 60% (media 30% | preview 70%); middle 30% (timeline 70% | effects 30%); bottom 10% command bar incl. zoom slider.
- 025 all ops reachable from command bar/panels; shortcuts: split, cut, copy, paste, undo, redo, play/pause.

## Constraints / decisions
- C1 ffmpeg >= 4.4 user-installed, not bundled in 0.1.3. Missing -> blocking message with install hints. Reason: licensing/size; revisit bundling later.
- D1 fade fixed length (no user-tunable duration) for simplicity; formula avoids overlap on short clips.
- D2 output format automatic (no export settings dialog) to keep UX simple.
- D3 layout fixed (no resizable/dockable panels) in 0.1.x.
- D4 stream copy only when frame-accurate result guaranteed; correctness > speed.

## MINOR defaults (from PLAN)
- Undo depth 50 min. Shortcuts conventional (Ctrl+X/C/V/Z, Ctrl+Shift+Z or Ctrl+Y, Space play/pause, S split) -- exact bindings left to engineering, assumption.
- Letterbox color black.

## Out of scope
- Multiple video tracks/overlays, color grading, keyframe animation, transitions other than fade-through-black, cloud/account/sync, online services (except one-time model download), mobile, collaboration, direct social upload, export settings UI, bundled ffmpeg (0.1.3).
