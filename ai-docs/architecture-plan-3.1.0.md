# Technical plan 3.1.0 (PLAN 2026-11-24; owner: architect; archived at release)

Scope: REQ-334..340 (docs/business*.md). Not planned here: the replace-text script and the test loop (/change-workflow). No new crate, no new CI job, no change to Cargo.toml except the version.

## Components touched
- `media` (`MediaFormat.audio_tracks`, ffprobe parse of audio streams, cap 32, titles sanitised) — ADR-245.
- `timeline` (`Clip.audio_track`, Merge compare, load validation) — ADR-245, 254.
- `project` (format 15 written on every save, Andrea's answer to Q-F1; reader 1..=15; no conversion; `v15_tests.rs`) — ADR-244. No 14-writer path and no second golden set for saves.
- `subtitles` (`Preset::AsTrack`) — ADR-249.
- new pure `word_layout` (`src/word_layout.rs`, base imports only) and `ass` / `word_styles` (Enlarge from base) — ADR-248.
- `render` (`graph.rs` audio `[n:a:k]` and blur chain for clips; `steps.rs` blur gating; `plan.rs` track-only subtitles; `args.rs` argv of track 0 unchanged) — ADR-246, 249, 250.
- `export` (`choice.rs` T2 forced for k ≠ 0; subtitle choice fixed to Separate for AsTrack) — ADR-246, 249.
- `proxy` (`-map 0:a?`, layout number in the key) and `preview` (FrameKey and audio restart include k) — ADR-247.
- `spatial` (pre-pass `-map 0:a:k`, key includes k) — ADR-254.
- `stt` and `app/subs_create` (extraction `-map 0:a:k`; command argument) — ADR-253.
- `app` command set (new commands: `SetClipAudioTrack(ClipId, u8)`; Subtitles command with `audio: u8`; style via existing `SetSubtitleStyle`) and `ui/popups.rs`, `ui/frame_popups.rs`, `ui/effects.rs`, `ui/strings.rs` (texts) — ADR-251, 253, 254.
- `player` (`app.rs` Audio menu command, `ui.rs` menu, in-memory clip track) — ADR-252.

## Ordered steps for the Principal Engineer
1. Model and format: `AudioTrack`, `MediaFormat.audio_tracks` (None until probed; REQ-186 re-probe path as subtitles), `Clip.audio_track`, `Preset::AsTrack`, SCHEMA_VERSION 15, validation (ADR-244, 245). Tests: format 15 written and read; 16 refused; 14 opens with defaults (golden output of a 14 file unchanged); unknown track repaired to 0 with the status line; ffprobe parse with 0, 1 and 2 audio streams. Covers REQ-340 (model part), REQ-029 (refusal).
2. Preview and export of the track (ADR-246, 247): `[n:a:k]` in the graph; T2 forced for k ≠ 0; proxy keeps all audio, key layout bump; FrameKey includes k. Tests: track 0 argv identical (golden); k = 2 graph has `[n:a:2]`; T1 refused for k ≠ 0; a fixture with two audio tracks (different tone per track, committed with SHA-256 like the speech fixture) produces export audio equal to track 2 (ffprobe and a spectrum or sample check). Covers REQ-340.
3. Volume and track (ADR-254): popup choice, Merge compare, spatial pre-pass track. Tests: popup lists both tracks of a two-track file (one for one-track); clip without the effect uses track 1; Merge refuses differing tracks; undo of the choice. Covers REQ-340.
4. Subtitles effect audio (ADR-253): popup choice passed to the command; extraction and matching on track k; subtitle track text kept. Tests: extraction argv `-map 0:a:2`; with a subtitle track and Use the lines keep the track text; no track: Recognise uses the chosen track. Covers REQ-339.
5. Player Audio menu (ADR-252): menu list, default 0, in-memory choice, session kept, not saved; `cargo tree` check unchanged. Tests in `player/src/tests.rs`: two-track fixture lists both; choosing 2 changes the clip track; after reopen in session kept; a new start uses 0. Covers REQ-338.
6. Enlarge word from base (ADR-248): `word_layout` pure functions (positions, baseline, half-width shifts, no overlap at 40 px); ASS events for the line with an enlarged word; old base event kept for other lines. Tests: base within 2 % of the height; centre within 0.5 % of width; neighbours shifted by half the extra width (1 %); 40 px no overlap; undo; old 2.3.0 test `enlarge_with_anchors_...` rewritten for REQ-334 (requirement change, not weakening); preview and export within 2 %. Covers REQ-334 and replaces REQ-333 (its line becomes "implemented; replaced by REQ-334").
7. As subtitle track (ADR-249): style in the Subtitles popup; export without burned lines; track writer used; Subtitles choice fixed to Separate with the reason text; preview with Classic look. Tests: 3 lines give one subtitle stream with 3 cues in the project language (mp4 mov_text, mkv srt); no burned text in argv; other styles byte-identical argv; 360 export writes the track. Covers REQ-337.
8. Blurred fill fix (ADR-250): locate the clip-row gap, one path for video and clip. Tests: graph string for a clip row with Fit on width and blur contains `boxblur`; real ffmpeg pixel test on a 1600x1200 clip in 9:16 (bands blurred, middle matches REQ-125 within 1 px, not blurred); preview and export within 2 %; colour fill unchanged. Covers REQ-335 and fixes REQ-126 and REQ-326 as built.
9. Overlap media row (ADR-251): own Effects row after Add Text, same drawing as Scale. Tests: row list has Add Text and Overlap media on separate rows; clicking adds an overlap (REQ-325 path); no other row changes. Covers REQ-336.
10. Hand-off: the Principal Engineer reports the suite (fmt, clippy, test, coverage, feature coverage); the Documenter runs after each part.

## Assumptions (not measured)
- A-3.1-1 the preview of "As subtitle track" uses the Classic look.
- A-3.1-2 proxies keep all audio streams; the extra size is small (one AAC stream per track).
- A-3.1-3 the blur cause is in the clip-row path (steps 8); if it is in the UI, the fix moves to `ui/popups.rs`.
- A-3.1-4 the enlarged word's baseline offset comes from the font's descent measured by the existing shaping (no new metric source).

## Test plan summary
Unit (pure): `word_layout`, `timeline` validation, `media` audio parse, `project` format 15. Golden argv: tracks 0 and all non-3.1.0 features identical (QG-15). Real ffmpeg (pinned build): two-track export audio, AsTrack streams, blur pixels, Enlarge preview/export. Player: menu tests. Coverage target as usual (85 % line, 100 % feature).
