# Code — text items, selection and 360° in 1.3.0

> Changed in 1.4.0: `TextItem.size` became `width` with a best-fit size, the schema is 6 and the
> 360 export question moved into the Export dialog. See [code-export.md](code-export.md).
> Changed in 2.0.3: the size is measured with the font, texts have a readability and a box
> height (schema 10), the events are wrapped by us. See [code-readable-text.md](code-readable-text.md).

What 1.3.0 added, as built: text items with effects, one selection for lines and texts, the
Context Menu group, texts fixed to the 360° sphere, the "360 video" export and the spherical
marker. The requirements are in [Text](business-text.md) and [Boxes on the picture and 360
video](business-picture.md); the design is in [architecture-text.md](architecture-text.md).

## Where things are

| Module | What |
|---|---|
| `text` | Pure. `TextItem`, `Texts`, effects, limits, validation, the ASS events of an item (`events`, `flat_events_in`, `equirect_events`), `alpha_at` (the visibility formula the events follow) |
| `selection` | `Selection` (subtitle lines and text items together), `ObjRef`; every transition is a method (`click`, `pick`, `drag_targets`, `esc`, `prune`) |
| `fonts` | Installed font families: `Fonts::start()` scans in a worker thread (fontdb 0.24), names are filtered by `valid_font` (letters, digits, space, `- . _ ( )`) |
| `spherical` | Writes the Spherical Video V1 `uuid` box into an MP4/MOV after a 360 export |
| `camera` | + `project` / `unproject` (rectilinear) and `anchor_at` (anchor of a new item) |
| `render` | + `ExportMode`, `plan_export_mode`, `Segment.anchored`, `flat_windows`, texts in `subtitles_doc`, `FrameKey.texts` |
| `app/texts.rs` | Commands of text items (add, edit, move, delete, box, anchor move, popup) |
| `ui/text_popup.rs`, `ui/timeline.rs` (`text_track`), `ui/picture.rs`, `ui/placement.rs` | The popup, the Text lane, the boxes on the picture |

## Model and file

- `TextItem { id, text, start, ttl, effects, colour [u8;3], font, place: Pos, size 250..4000, anchor }`.
  Times are **timeline** times: a text does not move when the video is edited (no ripple, like
  music). `ttl` is limited to the video length when editing, but a file is only refused above one
  day: a video may be cut shorter after a text was placed, and that file must still open
  (deviation from the plan, which said "video length").
- Project **schema 5**, always written. `texts` is a list; `Line.anchor` is optional. Loading
  refuses (as damaged): ttl < 0.1 s, effect times outside 0.1–5 s, bad text (empty, control
  characters, > 1000 characters a line, > 200 lines), font names outside the whitelist, size or
  place outside range, anchors outside ±180°/±90° or with size 0 or > 170°, duplicate ids, more
  than 2000 events in one item (blink 0.1 s over a long ttl) or 20 000 in the project, more than
  5000 items. A newer schema is refused untouched (`TooNew(6)`).
- `Edit`/`Project` carry `texts`; undo, redo and "unsaved changes" follow for free.

## Effects as ASS events

`text::events_with` is the one generator. A text without blink is one event with `\fad(in,out)`
(each limited to half of the ttl), `\move` for a slide (start one box outside the box and outside
the picture) and `\1c`, `\fn`, `\fs`, `\bord` written by us (text through `ass::escape`). A blink
makes one event per **shown** phase; a phase that overlaps a fade is written with `\alpha` and
`\t` steps. The same code handles an event that is only part of an item (cut at the end of the
video, or limited to the flat clips): slide start and alpha are recomputed at the cut. The slide
distance uses an estimated box (`estimate_box`: 0.6 em per character); the preview box is measured
by egui with the bundled font.

The flat events go into the same `subs.ass` as the subtitles, in the style `Text` (layer 1); a
project with a text is re-encoded (the subtitles rule of REQ-023) and the preview uses the same
document, so it shows the real font and effects.

## Texts fixed to the sphere

- Adding a text on a 360 clip stores `camera::anchor_at(view, w, h)`: the view centre, height 5 %
  of the view's vertical field of view, wrap 60 % of its width (assumptions of REQ-112).
- Burn-in is on the **equirectangular picture, before `v360`**: each 360 `Segment` carries
  `anchored`, an ASS file in source pixels written to the work folder as `anc<N>.ass` and read by
  `subtitles=f=anc<N>.ass:fontsdir=fonts`. Position: `x = (yaw/360 + 0.5)·W`, `y = (0.5 − pitch/180)·H`
  (positive yaw is to the right, checked with the real `v360` in `tests/ffmpeg_integration/v13.rs`).
  A box that crosses the left or right edge is also written shifted by ±W (the seam).
- On flat clips an anchored text is drawn at its flat `place` by the flat file, limited to the
  time of the flat clips (`flat_windows`). In a project with no 360 clip it is just a text.
- In the look-around view `placement::anchored_box` projects the anchor (`camera::project`); no box
  while the point is outside the view. Dragging the box uses `camera::unproject` and
  `Command::MoveTextAnchor`. Resizing changes the size and keeps the anchor. On a flat clip the
  box of an anchored text can be resized but not moved (its flat place is kept as it was).
- **Subtitle lines** added by hand on a 360 clip get the same anchor (`AddSubtitle`); lines made by
  speech recognition or SRT keep the preset place. A fixed line is turned into a white `TextItem`
  of its size (`render::anchored_line_items`) and burned with the texts: before `v360` in the
  flat camera view, on the sphere picture in a 360 video. It is left out of the flat file
  (`line_on_sphere`) and kept there on a flat clip. In the look-around view its box follows the
  camera; dragging it sends `Command::MoveLineAnchor` (the flat position is not touched).
  `FrameKey.line_anchors` makes the still frame change when an anchor moves. Fixed lines are
  drawn as plain text (no word highlight, no box style).

## Export of a project with 360 clips

`Command::PickExport` first sets `Modal::ExportKind` ("360 video" / "Flat camera view"); the
answer is `Command::ChooseExportMode`. This only happens for projects with a 360 clip. Mixed
projects: "360 video" is refused with a message (`RenderError::MixedSphere`), before anything runs.
`ExportMode::Sphere360` keeps every segment equirectangular (`sphere = None`, no `v360`, no
`sendcmd`), takes the output size from the 360 clips' real size, burns the anchored texts and
leaves view-fixed subtitle lines at their preset place. Without text or effect and with equal
formats the plan is still a stream copy.

After the encode `export::mark_if_sphere` calls `spherical::mark_equirectangular` (software and
hardware path, before the file is published): ffmpeg 6.1 drops the source's spherical side data on
a re-encode and has no option to write it, so the `moov` box is rewritten (a `uuid` box with the V1
XML in the video track; chunk offsets grow by its size when `moov` comes before `mdat`). Only
`moov` is read into memory; the rest of the file is copied. Not MP4/MOV (Matroska, WebM) or
unusual boxes: left as they are. `ffprobe` shows `projection=equirectangular` afterwards.

## Selection and Context Menu

`app.selection` is one `Selection` (the old `selected_line` and `BTreeSet` are gone). A click
without movement: Ctrl, Cmd or Shift toggles, a plain click selects only that object; a plain
click on empty picture or timeline clears; Shift+drag on the timeline picks lines and texts that
start in the range and moves the playhead to the first. A drag of a box moves the objects of its
kind that are selected (lines with lines, texts with texts). The Context Menu is
`toolbar::context_group(app)`: Align H/V with a line selected, Set POV with a 360 clip under the
playhead, Reset with either; nothing for a text; the group is not drawn when empty and goes to the
"»" menu as a whole.

## Tests

Unit tests are next to the code (`text`, `selection`, `fonts`, `spherical`, `camera`, `project`,
`render`); app tests in `app/text_tests.rs`; window tests in `ui/text_ux_tests.rs` (popup typing,
Text lane drags at real positions, Shift/Ctrl clicks, preview drags and corner resize, the
export question, the context group, the "Fade IN/OUT" name). Real ffmpeg: `v13.rs` (fade 50 % at
half time, blink, slide, text at yaw 30° ±1.5° in the camera view at three times, 360 video with
text ±1°, seam, spherical marker). The fonts scan is tested on a temp folder with the bundled
font, never on the system fonts (it would keep the cores busy during the responsiveness gates).

## Known limits

Preview boxes of texts are measured with the bundled font (the picture itself uses the real one);
slide distance of the export is an estimate; a font missing on the computer falls back in libass
and the popup says so; the 4K/8K frame-time and event-build measurements of M11 were not made
separately (the existing responsiveness gates pass); mixed selections (lines and texts) are moved
per kind.
