# Code — 2.1.0 (large LUTs, Applied effects, shadows, word styles, project format 12)

Back to the [index](code.md). Requirements REQ-300, REQ-301 (large LUTs), REQ-302 to REQ-305 (the Applied
effects widget, the whole-video marks, the movable order, overlapping texts), REQ-306 to REQ-310 (shadows,
word colours, Enlarge word, Typewriter), REQ-311 (project format 12).

## Why the 80 MB LUT was refused (measured)

The three files of the maintainer are 144-point cubes of 80,621,697 bytes (2,985,984 entries of 27 bytes).
The old reader refused them by its own limits, not by ffmpeg: the edge cap `MAX_3D = 65` ("size 144 is
outside 2..=65") and the file cap `MAX_BYTES = 16 MiB` ("larger than 16 MiB"). Measured with the pinned
ffmpeg 9.0.2 on the development machine (`lut3d=file=...:interp=tetrahedral`): a 144-point cube starts in
2.3 s and the process peaks at 80 MB; a generated 256-point cube (453 MB of text) starts in 12.4 s and peaks
at 233 MB (ffmpeg parses the text each time it starts, so a preview restart with a 256-point LUT is slow).

## Large LUTs (REQ-300, REQ-301)

| Where | What |
|---|---|
| `src/lut/parse.rs` | `parse_reader`: one pass over a `BufRead`, a line buffer of 256 bytes, values quantised to 16 bits as they are read, progress and stop every MiB (`LutError::Cancelled`) |
| `src/lut/mod.rs` | limits (`MAX_3D` 256, `MAX_BYTES` 512 MiB, `MAX_LINE` 256, `MAX_WHOLE` 64 MiB for `.3dl`/`.csp`), `Lut` (6 bytes an entry: 256^3 = 100 MB, 144^3 = 18 MB), `write_cube` (streamed text, no `format!` per entry), `cube_len` |
| `src/lut/store.rs` | `read_with` (a file that changes while it is read is refused), `cached` (192 MB, least recently used), `peek`, `Load` (thread + channel, `percent`, stops when dropped), `copy_name` and `write_copy` (the Work folder file `lut-<hash16>.cube`, written once per table: temporary name then rename) |
| `src/render/graph.rs` | `lut_chain` names the shared file; `Graph::lut_files` carries the tables; `write_command_files(dir, files, luts)` writes them (preview, export and the output check call it) |
| `src/app/lut.rs` | a file above 1 MiB that is not cached is read by a `Load` (`PendingLut`, `tick_lut_load`): the status line shows "Reading name... N %" and the LUT is applied, as one undo step, when it is read; a newer choice replaces the pending one |
| `src/project.rs` | `lut_copies`: a table whose text is above 20 MB (`Lut::cube_len`) is not copied, the project keeps the path (REQ-301); copies are written from `to_cube` |

Tests: `lut/parse.rs` (a generated 144-point stream, the 256 limit, a broken line number, stop, random
streams, an optional read of the maintainer's folder that is skipped when it is absent), `lut/store.rs`
(cache by bytes, Work copy name and content, background load, stop), `app/lut_tests.rs` (background choice and
undo, an 80 MB file used with a path-only project), `tests/ffmpeg_integration/v16.rs` (a 144-point channel
swap: preview and export within 2 levels). The three maintainer files are never written or copied.

Replaced tests: the exact `0.5` comparisons in `lut::tests` now allow 1/65535 (16-bit table); the render test
`each_clip_has_one_lut_stage_and_never_uses_a_user_name` expected `lut-clip-N.cube`, it now expects the shared
`lut-<hash16>.cube`; the 512 MiB `parse` size test became a test of the 256-byte line limit (the file size
limit stays tested through a sparse file).

## Shadows, word colours, Enlarge word, Typewriter (REQ-306 to REQ-310)

Spike S0d (libass, ffmpeg 9.0.2): in `BorderStyle 3` the box is drawn from the **outline** colour and its
shadow from the **back** colour, so design A holds: the style's `Shadow` is the distance and its `BackColour`
the shadow colour for every style, boxes included; the box colour stays in `OutlineColour`. With no shadow the
style lines are the 2.0.4 ones (the output is unchanged). Spike S0c: `\ko` (not `\k`) draws no outline for a
letter before its turn, and with `\2a&HFF&` its fill is transparent too; `\alpha&HFF&` hides a run and keeps
its place.

| Where | What |
|---|---|
| `src/base/shadow.rs` | `Shadow { px, colour }` (0..=20 px at 1080), a leaf shared by `text` and `subtitles` (no module cycle) |
| `src/text.rs` | `TextItem.shadow` (not written when 0); the Text popup row |
| `src/ass.rs` | `style` writes Shadow/BackColour, `text_styles` one style per (box, shadow) pair (`Text_<px>_<RRGGBB>`, `TextBox_...`), `document_styled`, `export_file_styled`, `shadow_px` (scaled with the height), `scaled_px` |
| `src/subtitles.rs` | `StyleSettings` (shadow, `highlighted` yellow, `not_yet` white, `enlarge` 12 px), `Subtitles.style`, `Preset::EnlargeWord`, `Preset::Typewriter`, `Preset::follows_words` |
| `src/word_styles.rs` | pure plans: `typewriter` (one event per word; spoken words in the other colour, the word typed letter by letter with `\ko`, later words hidden; a render that starts inside a word shows the typed letters at once) and `enlarge` (two events per word) |
| `src/app/dispatch_subtitles.rs`, `ui/effects.rs` | `Command::SetSubtitleStyle` (one undo step "Subtitle style", values clamped), the Subtitles popup rows |

Enlarge word is not placed from a measured centre (the plan's idea): a centred line whose other words are
hidden at their size and whose spoken word is `N` px larger keeps that word's centre exactly (the line grows by
the word's growth, half on each side), so the second event only changes `\fs`. The vertical position shifts by
the font's descent difference (a few pixels at 1080). The 360 sphere-fixed lines draw these two styles as plain
text. The preview cache key follows the spoken word, not each letter.

Tests: `ass/style_tests.rs`, `word_styles.rs`, `app/style_tests.rs`, `project/v12_tests.rs`, and in
`tests/ffmpeg_integration/v21.rs`: a red 8 px shadow down and right of the letters (export), the typed letters
at three times in the export and the preview, the enlarged word (height) and the other words unchanged.
Replaced assertions: the preset count (3 to 5) in two tests.

## Applied effects and the order of the effects (REQ-302 to REQ-305)

The order is data first: a pure module, a field of the timeline, and the project's view of a clip.

| Where | What |
|---|---|
| `src/effect_order.rs` | `Row` (`Lut`, `Text(id)`, `Picture(id)`, `Subtitles`; written as `lut`, `subtitles`, `text:<id>`, `picture:<id>`, any other name reads as `Unknown`), `Present` (the rows a clip has), `legacy` (the order of the older versions, top first: texts with the later one on top, the subtitle lines, the pictures, the LUT at the bottom), `resolve` (the stored order restricted to the rows the clip has; new rows on top), `to_store` (nothing when the result is the legacy order), `is_changed`, `moved`, `dragged` |
| `src/timeline.rs` | `Timeline.clip_order` (`ClipOrder { clip, rows }`, sorted by clip id, not written when empty, read leniently), `order_of`, `set_order`; a removed or merged clip drops its entry, a split gives both halves the order |
| `src/project/order.rs` | `Project::clip_rows` (per clip: its time range, the rows it has, and on a 360 clip the texts and lines fixed to the sphere, which are fixed rows), `order_of`, `changed_orders` (the clips drawn by the overlay stage), `set_order`, `put_on_top`, `repair_orders` |
| `src/app/applied.rs` | `AppState::applied()` (the rows of the selected clip, or of the clip at the playhead), `Command::MoveRow { clip, row, mv }`, `RowMove::{Up, Down, To}` |
| `src/ui/applied_effects.rs` | the widget |

A row belongs to a clip when its item overlaps the clip in time; a text over two clips has the order of each
clip. Only a clip whose order differs from the legacy one has an entry. A new LUT (on the clips that had none)
or a new picture is put on top once the edit is done (it is not a second undo step: one undo removes both).
Texts need nothing: the legacy order already puts the later item on top.

**The widget** takes about 15% of the Preview section (`applied_effects::SHARE`, at least 58 px) under the
play controls, and the picture shrinks first. It lists the movable rows top first, then the fixed rows in the
order the picture is built: Crop, Scale, Move, Change aspect, Fade IN/OUT, Volume, Clean audio, Speed (on a 360
clip the texts and lines fixed to the sphere come before them). Each row is a button: a click selects the clip
and opens the popup of the effect. A movable row has Move up and Move down buttons (named "Move up: <row>" for
a screen reader), moves by drag (it ends where the pointer is) and with Alt+Up and Alt+Down while it is focused
or under the pointer. A move is one undo step named "Move LUT up", "Move text down", "Move picture up",
"Move subtitles down", or "Move effect" for a drag. Rows are announced as "Applied effect: <name>".

**The whole-video mark** (REQ-303): "whole video" is shown on an effect that every clip has with the same
settings when the timeline has two clips or more (Volume, Scale, Move, Apply LUT, Clean audio, Speed), on the
project's Change aspect on every clip, on Fade IN of the first clip and Fade OUT of the last one (these are the
whole-video fades). Assumption: this is how a whole-video application is told from a clip's own, because every
whole-video effect is a copy on each clip (REQ-147).

## The overlay stage (REQ-304, REQ-305)

A clip whose order differs from the legacy one is built by `src/render/overlay.rs`; every other clip, and every
project saved before 2.1.0, is built exactly as before (its arguments do not change).

| Where | What |
|---|---|
| `render/overlay.rs` | `attach(project, segs, geometry, from, src, burn_lines, steps)` gives the segments of the changed clips their steps, bottom to top, and takes their LUT out of the clip chain; `OverlayStep::{Lut, Ass { name, doc }, Pic}`; `legacy_windows` (the timeline parts of the unchanged clips, neighbours joined), `changed_ids` |
| `render/graph.rs` | `overlay_chain` appends the steps after the fades (the frame composition, the Scale and Move, and the clip's own aspect are done): a LUT is `format=gbrp,lut3d=...,format=<pix>` on the whole frame, a text or the lines is `subtitles=f=ov<clip>-t<id>.ass` or `ov<clip>-s.ass` in the clip's own time, a picture closes the chain and overlays its file (one more still input per picture, after the global ones) |
| `render/plan.rs` | the document after the concat and the global picture overlays leave out the changed clips: `text::project_events_in` limits each item to the unchanged windows, the lines of changed clips are dropped, `PicOverlay.pieces` is the enable window of a picture that shows over unchanged clips only; a plan with overlay steps is never a stream copy |
| `render/args.rs` | the preview, the still, the background cache (`steps` false: only the LUT is taken out) and the output check call `attach`; `FrameKey.order` makes a changed order draw the picture again |

Decisions: the whole frame is graded by the LUT row (bars and the Change aspect background too), and the LUT
comes after the fades, so a LUT that lifts black shows on a faded frame. A Sphere360 export keeps one document
and ignores the order (it draws no flat pictures either). The lines of a clip that go to a separate subtitle
track are not drawn by the clip. A picture file that is gone stops an export and is left out of a preview, as
before.

Tests: `effect_order` (unit), `project/order_tests.rs`, `render/overlay_tests.rs`, `app/applied_tests.rs`,
`ui/applied_ux_tests.rs`, and in `tests/ffmpeg_integration/v21.rs` the order of a text and a LUT, two
overlapping texts, and a picture over two clips with a different order each, each in the export and the preview
(real ffmpeg). The old golden argument files pass unchanged. One existing test was adapted:
`frame_ux_tests::a_corner_handle_scales_around_the_opposite_corner_in_one_undo_step` uses a window of 1280x1000
(the shrunk picture put the Scale popup over its corner).

## Project format 12 (REQ-311)

`SCHEMA_VERSION` 12: text `shadow`, `subtitles.style` and `timeline.clip_order` are written only when not
default, so a project without them is byte-identical in those parts; formats 1 to 11 open with no shadow, the
2.0.4 colours, the `enlarge` default and the legacy order; a project of format 13 is refused with `TooNew` (the
older programs refuse 12 the same way). Out-of-range shadow and enlarge values are brought inside their range on
load. A damaged order is repaired on load (`Project::repair_orders`): an entry of a clip that is not there, a
repeated entry, an unknown or repeated row, a text or picture that is not in the project, or an entry that is
not an entry at all, is dropped, and the notice "Some files could not be found or were repaired" gets the line
"A damaged effect order was repaired". Tests that asserted `"schema_version": 11` now assert 12 and the
"newer" tests use 13.

## Not built (to continue)

- A project closed while a LUT is being read still gets the LUT applied to a missing target (an error line only);
  `MissingLutFind` reads the chosen file on the UI thread.
- The preview restart with a 256-point LUT takes about 12 s (ffmpeg parses the text); a binary input or a
  smaller preview table is the next step.
- The order of the texts and lines fixed to the sphere of a 360 clip is not movable (decision Q-3).
