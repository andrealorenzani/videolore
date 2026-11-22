# Choices made by Andrea

Back to the [index](business.md). One line per choice Andrea made himself in this project: version or area, the question, his choice. Quotes are his own words or the option he picked. Sources: his answers to the design questions (AskUserQuestion), his instructions, and the decisions marked as his in the documents. Assumptions and defaults that nobody chose are not listed.

## Process
- 2.0.1 — splash pictures: origin and licence of the start-screen GIFs; "I created these gif myself, if they need a License, use MIT".
- 2.0.2 — Linux package: no `.deb`; the product ships an AppImage and a tar.gz ("No .deb").
- 2.0.2 — installers: stay unsigned; code signing and notarisation not planned (decided 2026-11-20 too).
- 2.0.4 — one development builds all remaining batches 5 to 15 as version 2.0.4 ("one unique task", D57).
- 2.0.4 — the interface is English only (REQ-245, D60).
- 2.0.4 — screen-reader support on Windows, macOS and Linux: "AccessKit on all three".
- 2.0.4 — CJK subtitle and text fonts: "System fonts only", no bundled CJK font (REQ-187/188, D61).
- 2.0.4 — 360 sound in stereo: "Rotated ambisonic to stereo" (REQ-273).
- 2.0.4 — Windows and player association: "Add under 'Open with'"; the default program stays the user's choice (REQ-248).
- 2.0.4 — 360 export speed: "Fastest within 1.5x size" (REQ-266).
- 2.0.4 — Enhance speech: "arnndn with a bundled model" (REQ-279); later D61: speech normalisation ("Normalise speech"), no model file.
- 2.0.4 — 360 marker in MKV and WebM: "Write the marker in-house" (REQ-261).
- 2.0.4 — hardware export: always through ffmpeg (the bundled one or the one the user chose); no vendor pipeline (2026-11-20).
- 2.0.4 — hardware scope: AV1, VP9 and decoding in scope (architecture-2.0.4-overview).
- 2.0.4 — export presets: keep the fixed presets and "Same as source", no custom resolutions, frame rates or bitrates (2026-11-20).
- 2.0.4 — whole-video keyframe copies: shown apart, one per clip, not edited all at once (2026-11-20).
- 2.0.4 — Windows player association: "Open with" only, own installer template (architecture-decisions).
- 2.0.4 — drag and drop: ship where supported (Windows, macOS, Linux X11); not on Wayland.
- 2.0.4 — translated texts: not applicable, English only.
- 2.0.4 — "Aspect of this clip": "Both" (Fit on width and on height) and "Fit + empty space" (REQ-326).
- 2.0.4 — Applied effects: every effect can be reordered inside its group (picture or sound); Speed fixed (2026-11-21, "GO").

## Text, subtitles and LUT
- 2.0.3 — old texts with a black outline: "Old texts become None".
- 2.0.3 — old texts' size: "Sizes change on open".
- 2.0.3 — subtitle accuracy target REQ-177 (every word within 0.3 s): "Reword the target".
- 2.2.0 — subtitle accuracy: build the speech-region method, parameters checked on more than one clip (target kept).
- 2.2.0 — LUT preview speed: keep the `.cube` route and show progress (option D, D63).
- 2.2.0 — text size cap: 10% of picture height by default, optional per-text maximum (D64).

## Speed, overlap and effects (2.3.0)
- Speed (REQ-281): "I like the idea of speeding up a clip, but it must be in another effect, like `Change speed`", not keyframable, not in Change aspect.
- Overlap media, GIF or MOV after the animation (REQ-328): "Play once, then transparent" (2026-11-22).
- Overlap of a clip that is split, trimmed or deleted (REQ-329): a warning, then remove the attached overlaps.
- Old Add Picture items (REQ-330): "Become overlaps".
- Overlap and Change speed: the overlap's times are rescaled with the speed.
- Enlarge word (REQ-333): the word grows around its own centre; the other words do not move.

## Open items
- Volume keyframes (REQ-280) and the whole-video mark rule (REQ-303): answers not recorded in the documents; not listed.
