# Code — 2.3.0 (overlap media, Fit and empty space per clip, Volume and Change speed as effects, Crop removed, project format 14, Add media, one start-screen choice less, Enlarge word fixed)

Back to the [index](code.md). This page describes only what is built. The requirements are REQ-325, REQ-327, REQ-328,
REQ-329, REQ-330, REQ-331 (overlap media, see [business](business-overlap.md)), REQ-326 (Change aspect of one clip), REQ-280 and
REQ-281 (Volume and Change speed as effects), REQ-283 (Crop removed), REQ-332 (opening a project of an older format), REQ-324
(Add media), REQ-295 (the start-screen choices) and REQ-333 (Enlarge word keeps the word's place, the fix of REQ-309). The plan is in
[the architecture](architecture-2.3.0-overview.md).

## Overlap media (REQ-325, 327, 328, 329, 331)
- **Model** (`src/overlap.rs`, pure): `Overlap { id, path, kind: Still | Motion, from, len, place, width, height, reshaped, src_w, src_h }`.
  `from` is an offset from the start of its clip on the timeline, so a ripple moves the overlap with its clip. Functions: `is_valid` (REQ-057),
  `clamped`, `kept_part` (the part of the overlap inside a kept window), `rescaled` (a speed change), `rect` (the picture in its box: the file
  keeps its ratio inside the box; a reshaped overlap fills it), `is_large` (above 1920 x 1080), `drag_ends`, `sub_rows` (the lane layout), `lost`.
- **Storage is a deviation from the plan.** The plan put `overlaps` in `Clip`; `Clip` is `Copy` and every other per-clip datum (volume keys, aspects,
  LUTs, order) already lives in a `Timeline` side list, so overlaps are `Timeline.clip_overlaps: Vec<ClipOverlaps { clip, items }>` with
  `next_overlap_id`, written only when not empty. The behaviour is the one of the plan.
- **Edits of a clip** (`Timeline`): `split` cuts each overlap to the part in each half (an overlap across the cut is in both, the right one under a new
  id; nothing is lost); `trim` moves the offsets and drops an overlap with nothing left; `remove` drops the overlaps of the clip; `merge` moves the second part's
  overlaps after the first; `set_speed` rescales the offsets and lengths (`old speed / new speed`).
- **The warning (REQ-329).** `AppState::edit` compares the overlap ids before and after the edit; when some are gone and the edit is not confirmed it
  changes nothing and returns `app::overlap::REMOVAL_ASK`. `dispatch` turns it into `Modal::RemoveOverlaps(n)` and keeps the command in `pending_removal`;
  `ConfirmRemoval` runs it again with `removal_confirmed`, `CancelRemoval` drops it. One edit, one undo step. Removing an overlap with its own command asks nothing.
- **Commands** (`src/app/overlap.rs`): `AddOverlap(path)` (the clip under the playhead, 3 s cut to the clip, a box of the file's ratio, selected, on top of
  the clip's rows; no clip: the status line says so), `SetOverlapBox`, `SetOverlapTimes`, `RemoveOverlap`, `SelectOverlap`. The file is read by
  `picture::read_overlay` (PNG and GIF from the header, any other file by ffprobe, above 16 megapixels refused as REQ-316) and checked again when used
  (`picture::overlay_usable`; a MOV is trusted by the size read when it was added).
- **Render.** `render::PicOverlay` has a `Look { motion, skip, cover, half }`. `plan::picture_overlays` lists the picture items and the overlaps; the default
  order draws them after the concat (the argument list of a project without overlaps does not change), a changed order draws them as `OverlayStep::Pic` in the
  clip's own time (`overlay.rs`, `steps.rs`). A still is a looped input (`shortest=1`); a GIF or video is read once from its start (`-ss` when the render begins
  inside it), `setpts` puts it at the overlap's start and `eof_action=pass` lets the picture through after it (REQ-328); a reshaped video is scaled up and cropped
  to the box (`increase`, `crop`), a reshaped GIF or picture is stretched. Only the video stream is read: no sound. A file above 1920 x 1080 is halved first in the
  preview only (`scale=iw/2:ih/2`). A 360 export draws no overlap. The preview never decodes an overlay file in hardware (`preview::input_codecs`).
  `FrameKey.overlaps` tells a picture with an overlap on screen from one without.
- **Order rows.** `Row::Overlap(id)` (`overlap:<id>`) is a row of the picture group; the default order puts the overlaps above the picture items and below the texts
  and lines (a drawn row never changes the arguments of a project that does not use them); a new one goes on top (`put_on_top`).
- **Interface.** The Effects section has the row **Overlap media** (it shares its row with Add Text: nine effects and two rows did not fit the section at
  1280 x 800) and the Edit-menu action `AddOverlap`. The timeline has one shared lane (`ui/overlap_lane.rs`) only while overlaps exist: sub-rows by greedy start time, at
  most three visible, the mouse wheel scrolls the others; the body and the two ends of a bar are dragged (one undo step, kept inside the clip). In the preview
  (`ui/overlap_box.rs`) a click selects a box, a drag of the selected one moves it, a drag of an edge or corner resizes it with the opposite side fixed and the ratio
  kept, and Ctrl (Command) held when the drag starts changes the shape (`reshaped`); the outline follows the pointer and one command is sent when the button is released.
  The Applied effects widget lists `Overlap: <file>` rows with their time range.

## Format 14 and the conversion (REQ-330, REQ-332, REQ-029, REQ-057)
- `SCHEMA_VERSION` is 14; formats 1 to 14 open, 15 and newer are refused (`TooNew`, the file untouched). A 2.2.0 program refuses 14 as it refuses any newer format.
- `project/v13.rs` converts a file of format 13 or older in memory when it is opened: the crop of every clip is dropped (`Clip.crop` stays only to be read,
  `skip_serializing`), each picture item whose start lies inside a clip becomes an overlap of that clip (same picture, place, box and times, cut to the clip) and its
  order row `picture:<id>` becomes `overlap:<new id>`; a crop row goes without a note; an item with no clip under its start stays a picture item. Volume keys and
  speeds need no change. `Project.converted` holds the counts and `app` shows the one status line "Opening: N clip(s) changed by the conversion (M crop(s) dropped[, K picture(s) made overlaps])" (no line
  when nothing changed); the project is not marked changed and has no undo step.
- `validate_overlaps` drops a damaged overlap (an unknown kind, no length, outside its clip, a number outside its range, a repeated id) and says "N damaged overlap(s) were removed".

## Volume and Change speed as effects, Crop removed (REQ-280, 281, 283)
- Volume: the clip's popup has the keyframes (Add keyframe at the playhead with the level there, the value and Ease of each key, remove a key, Remove effect); applying
  Volume to a clip gives it one key at the slider's level; the slider is shown off while keys exist. The keys are `Timeline.clip_volume` as before; the "Volume keys" row of Clip details is gone.
- Change speed: `EffectKind::ChangeSpeed`, `Instance::Speed(FrameTarget)`; one clip (drag, right-click) or every clip (double-click: `SetSpeedAll`, one copy on each clip, one undo
  step); not keyframed; the value is `Clip.speed` as before and its Applied row opens the popup. The Speed row of Clip details is gone.
- Crop: `Step::Crop`, `crop_filter`, `Timeline::set_crop`, `Command::SetCrop`, the Clip details row and the Applied row are removed; `Row::Crop` is read only so that a stored crop row is dropped without a note.

## Other changes
- **Change aspect of a clip (REQ-326).** `ui::frame_popups::aspect_fields` draws the shape, custom ratio, Fit, Empty space (colour with its chooser, or blurred with its level) for the
  whole video's popup and for the clip row of Clip details (`popups::clip_aspect_rows`); the first shape gives the default aspect, a later shape keeps the Fit and the Empty space (the old row forced Fit on height).
- **Add media (REQ-324).** `ui::media::ADD_MEDIA` ends the Media section and sends `Command::PickImport`, the command of File > Import media….
- **Start screen (REQ-295).** `SplashOption` has `Always` and `OncePerVersion`; a stored "Never" reads as `OncePerVersion` (`lenient_splash`, otherwise it would have become the default).
- **Enlarge word (REQ-333).** `measure::Shaped::place_words` places the words of a line in their wrapped lines; `ass::word_centres` gives each word's centre (the line is centred at its
  own place or the style's); `word_styles::enlarge` keeps the base event (the word hidden) and draws the word alone as `{\an5\pos(x,y)\fs..}`, so its centre stays and the other words
  do not move. A line that cannot be measured keeps the old in-line form. The centre (not the baseline) is kept, as REQ-333 says.

## Measured
- Memory of a 16 megapixel PNG overlay (4000 x 4000, ffmpeg 9.0.2, `-loop 1`, scaled to a box as the first filter): the peak is about 830 MB (a single frame decode is 156 MB); the plan assumed 64 MB. Time: a 4 s still takes
  5.1 s to export because every looped frame is decoded again; with `-framerate 1` the same graph takes 0.9 s and 760 MB. This is the behaviour of picture items since 2.0.4; not changed (the argument lists are fixed by golden tests).
- A real render checks the Enlarge word centre (1080p, libass): within 4 px of its normal place (the plan allows 21 px).
- Not measured: the frame time with an overlay (estimate 10%) and the export time (estimate 1.5 times).

## Tests
- `src/overlap.rs` (model), `src/project/v14_tests.rs` (format 14, damaged overlaps, the conversion), `src/render/overlap_tests.rs` (argument lists: still, motion, reshape, half size, missing or large file, order step),
  `src/app/overlap_tests.rs` (add, edit, warning and undo, speed, Volume, the opening line), `src/ui/overlap_ux_tests.rs` (Add media, Change aspect of a clip, Settings, the lane, the preview box, the widget rows),
  `src/ui/overlap_lane.rs` and `src/ui/overlap_box.rs` (pure layout and geometry), `tests/ffmpeg_integration/v23overlap.rs` with the committed fixtures `tests/fixtures/overlay/alpha.{png,gif,mov}`
  (a 64 x 64 picture with a red square on a transparent ground; the GIF and the qtrle MOV last 1 s): transparency in the preview and in the export, once then transparent, reshaped, and the Enlarge word centre in a real render.
- Changed tests: the schema tests (13 to 14, "newer" 14 to 15), the splash tests (no Never), the Crop tests (now: a crop left in the field changes nothing), the Applied widget tests (Change speed row), the Effects list (nine rows), the Volume effect test (one undo step at apply).

## Not built or limited
- A MOV overlay is measured by ffprobe when it is added, not again at every preview or export.
- HEVC with alpha and other codecs that the bundled ffmpeg does not read with alpha are drawn opaque.
- The overlap lane has no zoom; the preview box has no handles drawn (the edges and corners are found by the pointer).
- `Command::AddPicture` (picture items) stays in the code for the tests and for items that have no clip under their start; the interface offers Overlap media only.
