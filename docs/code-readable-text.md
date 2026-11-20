# Code — readable text, placement and subtitle accuracy (2.0.3)

What 2.0.3 built, as built: the text size measured with the font (REQ-155 to REQ-167), where
boxes end up (REQ-168 to REQ-176) and the recognition pieces and messages of subtitle matching
(REQ-178, REQ-179). The requirements are in [Editing](business-editing.md),
[Text](business-text.md), [Subtitles](business-subtitles.md) and
[Subtitle accuracy](business-subtitle-accuracy.md); the design is in
[architecture-text.md](architecture-text.md) and [architecture-subtitles.md](architecture-subtitles.md).
**REQ-177 is not met** (see the last section).

## Step 0 — blur chroma radius (CI regression)

`render::blur_radius(sw, sh)`: the small copy of the blurred background is at least
`frame::BLUR_MIN_SIDE` (8) pixels on each side, and the radius is `BLUR_RADIUS` (3) limited to
`min(sw, sh) / 4 - 1`. `boxblur=r:2` uses `r` for the luma **and**, unless told otherwise,
for the chroma planes, which are half the size (yuv420), and ffmpeg 9 refuses a radius that is
not under half of the smaller side of a plane. A 68x120 frame (small copy 8x14) failed
(`Invalid chroma_param radius value 2, must be >= 0 and < 2`). The strings of larger frames
are unchanged (`boxblur=3:2`). Test: `render::tests::the_blur_radius_stays_inside_ffmpegs_bounds_for_every_small_side`
(sizes 8 to 600), `v15::a_4_3_video_in_a_tall_frame_with_a_colour_fill` on the pinned ffmpeg.

## REQ-155 — the paused picture at the end of a clip

`render::Segment.grid` (the source frame rate) and `Segment::lead()`: when a picture starts
inside a source frame (`skip > 0`, not a 360 clip), the source frame that holds the first
instant starts `lead` before it. The input is read from `lead + 1 ms` earlier
(`read_back`), the trim starts there (`trim=start=…:duration=d+back`), `setpts` puts that
frame at its own time (`PTS-STARTPTS+{skip-lead}/TB`) and a final `trim=duration=d` keeps the
segment's length. Before, `trim` dropped that frame and `fps` rounded the next one past the
end of the segment, so the last frame interval showed nothing. A `max()` in `setpts` was tried
and rejected: it loses the frame's duration and `fps` drops the frame. Export (`skip = 0`) is
unchanged (the golden strings are the same). Tests: `v20::a_paused_picture_in_the_last_frames_of_a_clip_is_shown`,
`render::tests::a_picture_from_inside_a_frame_is_read_from_the_frame_that_holds_it`.

## Measuring the text (REQ-156, 157, 159, 163)

`measure` (pure, no crate dependency; `skrifa` reads the font tables, `harfrust` shapes with
kerning and ligatures): `Shaped::new(bytes, index, text)` shapes each word once; `fit` finds
the largest size in 0.1 px steps at which the text, wrapped greedily at the box width (a word
wider than the box is cut by characters, `\n` is a hard break), fits the box height, between
2% (REQ-116, `fits = false` when it still does not fit: the warning of REQ-159) and 10%
(REQ-157) of the picture height. **The renderer's size rule** (found by measuring libass with
the bundled font): the ASS `\fs` is the height of one line, `usWinAscent + usWinDescent` of the
OS/2 table (libass mimics GDI), not the em; a glyph is `size / (ascent + descent)` em tall and a
line is exactly `size` high. `measure::line_units` follows it (hhea only when there is no OS/2
table).

`text::layout(item, w, h)` and `layout_in` give the `TextLayout { px, lines, width, fits }`;
`text::events` writes the lines joined with `\N` and `\q2` (the renderer never wraps), so the
preview and the export show the same lines. Shaping is cached by (font file, text)
(`text::shaped_for`, 512 entries). `legacy_fit_px` / `legacy_fit_in` keep the 2.0.2 estimate for
the migration count and for `legacy_width` (1.3.0 projects). `text::estimate_box` is the measured
box plus the readability padding (slide distance, boxes).

Deviation from the plan: the layouts are not computed by `app` and passed into `render`; `text`
reads the faces through the process-wide cache of `fonts` (below), which keeps `render`, `export`,
`preview` and the player unchanged.

## Fonts (REQ-163, 164, 165, 166)

`fonts` keeps, after the scan (worker thread, unchanged), where the font files are (the
`fontdb::Database` of the scan, `fonts::install`). `fonts::resolve(family)` returns the
regular-weight face of an installed family, read from its file on first use (cap 64 MiB per
file and in all, the 8 newest files kept); `None` for the bundled font, an unknown family or a
file that cannot be read, and the caller then uses the bundled bytes. A text in a missing font is
written as `\fnNoto Sans\b1` (never the renderer's silent fallback, REQ-164); the name stays in
the project. An installed font is written as `\fn{family}\b0`: the face measured is the face
drawn. `fonts::copy_used(dir, doc)` copies the files of the installed fonts a document names
into the work folder's `fonts/` (named by a checksum, called by `export::write_subtitles` and
`render::write_command_files`), so the renderer finds exactly the measured face on every system.
The test font is the bundled font under another name with another `usWinAscent`
(`tests/support/patched_font.rs`, shared with `include!`; `fonts::install_patched` in unit tests).

## Readability and project format 10 (REQ-158, 167)

`text::Readability { None, Outline, Box }` (serde `snake_case`, default `None`); a new text
starts with `Outline`. `\bord` is 6% of the size (Outline), 10% (Box); the Box events use the
style `TextBox` (`ass::TEXT_BOX_STYLE`, BorderStyle 3, outline colour `&H66000000` = 60% opaque,
libass draws the box in the outline colour and pads it by `\bord`), the others `Text`.
`TextItem.height` is the box height in thousandths (50 to 1000, default 200). `SCHEMA_VERSION` is
10; 1 to 10 load, 11 is `TooNew`. A text of a file saved before 2.0.3 has neither field, so it
opens as `None` with the old height. `project::from_json_noted` / `load_noted` also return
`TextMigration { texts, resized }` for a format below 10 with at least one text (the count of
texts whose measured size at 1920x1080 differs from the 2.0.2 estimate); `app::open_now` shows
`TextMigration::message()` as the status once. Saving writes format 10.

## Commands and handles (REQ-159 to 162)

`Command::ResizeTextBox { ids, edge: TextEdge, value }` (left or bottom edge) and
`ResizeTextWidth` (right edge) keep the opposite edge: the centre moves by half of the change, so
the box on the screen is the box that was dragged. `SetReadability`. `ui::boxes`: `Hit::Edge(Side)`,
`text_edge_at`, `resize_edge`; text boxes have handles on the right, left and bottom edges;
`ui::picture::text_box_drag` draws the live box with the readout `picture::readout`
(`50% wide, 20% high, text 7.5%`) and stores one command on release (no undo step during the
drag). `picture::text_flags` gives the warning (red outline and note on a text that does not fit)
and the blink (a hidden text shows only a dashed outline when selected).

## Placement (REQ-168 to 176)

`placement` (pure, frame pixels): `Rect`, `line_rect` (the box of a line from the measured text and
`ass` sizes), `text_rect`, `least_inside`, `overlapping_lines`, `partly_outside_at`,
`place_after_text_edit`. `app/placing.rs` applies it: the text edit of a line keeps the centre of
its box and moves it back inside by the least move when it grew (`SetSubtitleText`, REQ-168,
174), `MoveSelection` (REQ-172, lines and texts together, ends at the dragged place),
`MoveInside` (REQ-175), `AlignSelected` gives a line at the style's place its own place (REQ-173;
`ALREADY_CENTRED` is the reason when nothing changes), `AppState::partly_outside` and
`overlapping_lines` (derived, never stored; REQ-169, 175, 176). The UI: `ui::status`
(`SELECTION_HINT`, `outside_text`), `UiCache::overlapping_lines` (the warning in the Subtitles list
and on the timeline markers), `picture::pick_at` (a second click on the same spot, 4 px and 1 s,
takes the next box of the stack, REQ-170), the dashed outline of an item partly outside, the
**Move inside the picture** action (Edit menu and the Context Menu group). Scale, Move and Change
aspect never change an item's place (REQ-176).

## Subtitle accuracy (REQ-177 to 179)

- **REQ-178**: `stt::recognise` reads pieces of `CHUNK` (120 s) with `OVERLAP` (2 s) more on each
  side (seek by byte offset, buffer 124 s) and keeps the words whose start lies in the piece:
  a word is kept once and a word cut by a boundary is heard whole by the piece that owns its start.
- **REQ-179**: `stt::unmatched_message` adds that the subtitle text does not follow the speech
  closely when at least half of the lines fell back to evenly spread words.
- **REQ-177 is not met** (reworded 2026-11-20: at least 90% of the MATCHED words within 0.3 s,
  the matched share reported). Measured on the committed clip with the Fast file (pinned ffmpeg
  and model): 23 of 28 words matched, mean start error 0.48 s, largest 1.10 s. DTW token times
  (`DtwModelPreset::Tiny`) lag every word by 0.43 to 0.75 s (dropped). The one-sided onset snap
  (latest energy onset 0.2 to 1.0 s before the model's time, 10 ms RMS envelope) was built and
  measured: 7 of 23 matched words within 0.3 s (mean 0.45 s, largest 1.13 s), because the
  model's word times (contiguous segments) are off by up to 1.1 s in both directions; the
  nearest onset within 0.5 s or 1.0 s gives 8 of 23. The snap was removed. The energy onsets of the
  clip themselves match the reference within 0.03 s, so a method that does not start from the
  model's times could work. Tried offline on the clip's own envelope (speech regions from the 10 ms RMS, words of the recognised list placed in them by dynamic programming: region capacity from the word length in characters at a fixed rate, a weak pull towards the model's time, regions may stay empty): 12 of 23 matched words within 0.3 s with neutral settings, 21 or 22 of 23 only for 4 of 120 settings (pull 1 to 3, skip cost 1 to 3, rate 8 characters per second). That is tuning to one synthetic clip with words separated by pauses; continuous speech has no pauses, so it was not built into the job. The
  strict test `v12::alignment_matches_real_speech` (gate: matched words within 0.3 s >= 90%,
  matched share printed) stays `#[ignore]`. The fixture files are checked by SHA-256 in the speech
  tests (`check_fixture`) and CI sets `VIDEOLORE_REQUIRE_STT=1`.
