# Code — 2.2.0 (whole video as copies, picture limit, LUT preparation, project format 13, text maximum, order of all effects, Applied column)

Back to the [index](code.md). This page describes only what is built. The requirements are REQ-312, REQ-313,
REQ-314 (measured), REQ-316, REQ-317, REQ-318, REQ-319, REQ-320, REQ-321 and REQ-322 (the order of the picture group and
of the sound group), REQ-323, and the whole-video amendments REQ-076, REQ-083, REQ-129, REQ-131, REQ-147, REQ-148, REQ-200 to
REQ-202, REQ-303. Built and **measured, target not met**: REQ-315 (Faster on integrated graphics) and REQ-177 (speech regions): see
[Measured, not met](#measured-not-met). Not built: see [Not built](#not-built).

## Measured before the code (spikes)

- **Picture pairs of the order (S0e, ffmpeg 9.0.2).** Crop before Frame is a percent crop of the source, then the fit;
  Crop after Frame crops the framed canvas and scales it back to the frame size: both keep the frame size. On an RGBA layer
  over a transparent canvas (`color=c=0x00000000`, `overlay=format=auto`) Scale then Move puts the red/blue border of a
  test picture at x=160 and Move then Scale at x=200: they differ, as the business says. An invert LUT grades the black bars
  when it is above Frame and leaves them black below it; on the layer use `format=gbrap,lut3d` (alpha is kept). A packed
  `rgba` `fade` also fades the alpha (wrong for a layer); `format=gbrap,fade` fades the colours only. `alphaextract` after a
  `split` of an rgba layer fails ("Requested planes not available"). These are the formulations of the picture-group step builder
  (see below). Added when it was built: libass leaves the alpha of an RGBA frame as it finds it (a text on a transparent layer is invisible),
  hence the matte of black and white renders (checked against the direct drawing on a grey frame: identical to within a level).
- **Sound pair.** Volume and Clean audio in the two orders give different levels (RMS -36.4 and -38.9 dB), also with the
  dry branch of a half-strength Clean audio (-36.1 and -37.3 dB). The existing chain puts the `asplit` at the filter's point, so
  a swap needs no new formulation.
- **Text (S0f).** `text/events.rs` anchors the event at the centre of its box (`\an5`) and slides from one box width or height
  away, so the block is already centred on both axes and a larger box gives a longer slide (REQ-318): only the cap changed.
- **Layout (S0g).** The default layout is top row 2/3, Media 30%. At 1280 x 800 the Preview section is about 885 px wide:
  15% is 133 px, under the 180 px minimum, so **the column is 180 px (about 20%)**; the 15% share governs from a 1200 px
  section. At 800 x 500 the Preview is about 550 px: column 180 px, picture about 370 px. The requirement says "about 15%" at
  1280 x 800: the Product Manager adjusts the wording.
- **Blurred fill (REQ-314).** `testsrc2` through the graph of `compose_frame` (9:16 Change aspect, Medium blur, 300 frames, no
  decode, i7-1065G7): 1080p plain 2.4 to 2.8 ms a frame, with the blur 4.1 to 4.8 ms (the step costs about 2 ms); 4K plain 8.3 to
  8.9 ms, with the blur 10.7 to 11.2 ms (about 2 ms). One frame time at 25 fps is 40 ms: the target (b) is met by a factor of 20, the
  blur size is not changed. Test `v22::the_blurred_fill_costs_less_than_one_frame_time...`.

## Whole video as copies (REQ-083, 129, 131, 147, 148, 200 to 202, 303, 076)

- `FrameTarget::Whole` no longer names a state. It means "a copy on every clip" when an effect is applied (`add_scale`,
  `add_move`, `set_lut`); reading keys or editing them on `Whole` finds none or fails. `Timeline::whole_ref` and `whole_edit` are gone.
- A double-click on Scale or Move adds a copy to each clip that lacks one (one undo step, none when every clip has it) and opens the
  popup of the clip under the playhead (`AppState::clip_at_playhead_or_first`). A click on the row opens the selected clip's own
  popup, or says that the clip has none. `Command::SelectCopy`, `ApplyWholeToNew`, `Popups.copy`, `app/offer.rs`, the dashed
  handles, the row list of the popup, the status-bar offer and the "whole video" mark of the widget (`AppliedRow.whole`) are removed.
- Apply LUT: the double-click opens the file chooser for every clip (`Instance::Lut(Whole)`, "Apply LUT — every clip"); when the file
  is applied the popup follows to the clip under the playhead.
- Titles: only Change aspect says "— whole video" (`popup_title`); the Fades, Volume and Clean audio popups, which work on all clips,
  have plain titles.
- Fade IN/OUT stay flags of the first and last clip (their popups `FadeStart` and `FadeEnd`); Change aspect is not copied.

## Picture limit (REQ-316) and the preparation of a LUT (REQ-312)

- `picture::MAX_PICTURE_PIXELS` (16,000,000, width times height) is checked where the PNG header is read; the message is
  "The picture is larger than 16 megapixels (W x H) and was not added." `picture::usable` (the header is read again) decides whether
  a picture file can be drawn: a file replaced by a larger one is a missing picture for the export (it stops with its message) and
  the preview (left out).
- `lut::Prep` (in `lut/store.rs`) reads and parses the table, then writes its Work copy into each folder of the preview
  (`Preview::work_dirs`: `play` and `still`), on its own thread, with the share done (0 to 100, half reading and half writing); dropping it
  stops it. `lut::is_prepared(path)` says that the copies are there and the file has not changed.
- `AppState::unready_luts` lists the clip LUTs that are not prepared (and did not fail). `shown_project` takes them out of the
  project the preview draws, so the preview shows the unprocessed picture meanwhile; the export is not affected. `tick_lut_prep`
  runs one preparation at a time, shows "Preparing LUT N %" (reading and writing are the first 60%, then the start of ffmpeg is an
  estimate of 0.75 microseconds an entry, up to 95%, until the first new picture), restarts the preview when the table is ready, and
  says why when it cannot be prepared (the LUT is then left out and not tried again). A newer choice replaces the preparation.
- Measured earlier (2.1.0): a 256-point LUT starts in about 12 s, a 144-point one in 2.3 s; this is reported, not gated.

## Project format 13 and the text size (REQ-318, 319, 323, 322, 029)

- `SCHEMA_VERSION` is 13; 1 to 13 load, 13 is always written; 2.1.0 and older refuse it with their newer-version message.
  `TextItem.max_size` (thousandths of the picture height, 20 to 500) is written only when set; a value outside the range is dropped on
  load with the note "A text size maximum outside 2 to 50 was removed". `text::parse_max_size` reads the field (percent, one decimal).
- The size of a text is the best fit of its box (`measure::fit` through `text/layout.rs`) with the cap `TextItem::cap()` = its
  maximum or 100 thousandths. Old texts have no maximum: the golden test `text/size_tests.rs` pins the 2.0.4 sizes (52.3, 56.5, 43.2,
  43.1 and 108 px at 1920 x 1080).
- The Maximum size field is in the shadow row of the text popup (and of the Add Text popup); a wrong value shows "Enter a size from 2
  to 50, or leave it empty" and changes nothing; a committed change is one `EditText` undo step.

## The order of the sound group (REQ-320, REQ-321, REQ-322)

- `order_chain.rs` (pure): the groups, `DEFAULT_SOUND` (Clean audio, then Volume, as since 1.4.0), `resolve_sound`, `sound_swapped`,
  `with_sound`, `with_picture`, `moved_sound`. A sound list reads top first as the order of the build: Volume above Clean audio raises the
  level before the noise reduction. `Row::Volume` and `Row::Clean` (names `volume` and `clean`) are stored in the clip's order beside the
  picture rows, only when the order is not the default; the picture functions ignore them, so a sound swap never makes the clip use the
  overlay stage. A 2.1.0 order has no sound rows and keeps its look (`expand_2_1_0`).
- `Segment.volume_first` (from `Project::sound_swapped`) orders the two filters in `render/graph.rs`; the dry branch of the mix takes its
  input at the filter's point. Preview and export share the chain.
- The widget lists the sound rows in their order after the picture group; they move with Move up and Move down inside the group only
  (undo "Move Volume up" and so on); Speed stays the last, fixed row.
- **Assumption (the user's decision for the sound group, extended to the picture rows below):** a new Volume or Clean audio takes the
  default place (Clean audio, then Volume), not the top of its group.

## The order of the picture group (REQ-320, REQ-321, REQ-322)

- **Rows.** `Row::{Crop, Frame, Scale, Move, Fade}` (file names `crop`, `frame`, `scale`, `move`, `fade`) join `Lut`, `Text`, `Picture` and
  `Subtitles`. `Frame` is Change aspect (the clip's own or the project's), `Fade` is every fade of the clip as one row (named "Fade IN",
  "Fade OUT", "Fade IN/OUT with the previous/next clip", or "Fade IN/OUT" when there are several). The widget lists the whole group (every
  row has Move up/down, a drag and Alt+Up/Down), then the rows fixed to the sphere, the sound group and Speed.
- **Default order** (`effect_order::legacy`, top first): the flat texts (later on top), the lines, the pictures, then `FIXED` = Fade, Frame,
  Move, Scale with Apply LUT between Scale and Crop, then Crop. It is exactly what the build did before: a clip with this order is drawn by
  the legacy chain and the golden arguments (formats 4, 5 and 12, the LUT projects) do not change. Nothing is stored for it.
- **New rows.** A new text, picture or LUT goes on top of the *other movable rows* (`Project::put_on_top`; a LUT on a clip with no text,
  picture or lines stays at its default place, as in 2.1.0). A new Crop, Change aspect, Scale, Move or Fade takes **its default place**
  (`effect_order::resolve` puts it above the first lower fixed row, else at the bottom). **Deviation from REQ-321 "new rows on top"**: a
  Scale put on top of Change aspect would scale the blurred background too, and a Crop on top of a text would cut the text; both would
  change what every project with these effects shows. The user decided the same for the sound group; reported to the Product Manager.
- **Three builds, decided by the resolved order only** (`overlay_form`, `needs_steps`): the default order (legacy chain); a 2.1.0 order, the
  movable rows above the rest in the default sequence (the 2.1.0 overlay stage, unchanged); any other order (`render/steps.rs`).
  `Project::changed_orders` lists the clips that are not default, `overlay::attach` gives them their `Segment.overlay` and, for the step
  builder, `Segment.steps`.
- **Storage (format 13).** The list holds every row of the clip that the user ordered, written only when it differs from the default.
  A 2.1.0 order (movable rows only) resolves to those rows above the fixed ones in the default sequence, so its look does not change
  (`resolve`); a damaged entry is repaired with the 2.1.0 note.
- **Moves.** One undo step named after the row ("Move Scale up", "Move Change aspect down", "Move the Move effect up", "Move Fade up",
  a drag is "Move effect"); a move of a picture row keeps the stored sound rows (`order_chain::with_picture`). Copies are per clip.

### The step builder (`render/steps.rs`, ADR-224)

The stream is in one of three states, and each row acts on what the picture is at its place:

| State | What it is | What the rows do |
|---|---|---|
| source | the file's picture | Crop cuts the source |
| layer | the picture fitted to the frame on a transparent surround (RGBA, the frame's size) | Crop cuts the frame and scales it back; Apply LUT (`format=gbrap`) and Fade (`gbrap`) change the colours and keep the transparency; Scale resizes the whole layer about the frame's centre and Move offsets it (overlay on a transparent canvas), so Scale then Move and Move then Scale differ; a picture item is overlaid; a text is drawn by a matte |
| frame | the layer composed over the background (Change aspect), opaque | Scale and Move act on the whole frame (what it no longer covers is black); a LUT grades the background too; a Fade fades it; texts and lines are burned as in 2.1.0 |

The fit is done by the first row other than Crop; the background is composed at the Change aspect row (the blurred one is made from the
picture as it is at the fit, `blur_chain`; Apply LUT below the frame does not grade it); a clip without Change aspect is composed over
black before its first text, picture or line, or at the end. A clip's own frame (REQ-286) is placed in the output after the compose.
The documents of the clip's texts and lines are written in the clip's own time (`doc_shift` = the clip's start), because the chain keeps
the clip's clock until its last filter. A text on the layer is a **matte**: libass leaves the transparency of a frame as it finds it, so
the document is drawn on black and on white, `255 - (white - black)` is what it covers, `black * 255 / covered` its colours, and
`alphamerge` joins them (exact against the direct drawing on a grey frame in the spike). The 360 camera view is the source of a 360 clip
(no Crop, as before); the whole-sphere export ignores the order. The preview does not cache the blurred background of such a clip.

Real-ffmpeg tests (`tests/ffmpeg_integration/v22order.rs`, preview and export compared on a grid of the picture): Scale/Move swapped, Crop/Scale
swapped, LUT above and below Change aspect (and Scale above the frame), Fade above and below the LUT and below the frame, a text below Scale
and on the layer (matte), a blurred background, a 360 clip.

## Rows cache (REQ-313) and the column (REQ-317)

- `AppState::applied()` keeps the rows under a key (clip, `rows_serial` that every edit, undo, redo and project replacement increments, and
  the numbers of clips, texts and lines); `applied_builds()` counts the builds (60 frames without a change: none; one change: one).
- The widget is a column on the right of the Preview section at its full height (`ui/mod.rs preview_panel`, `applied_border`): width
  `layout::applied_width` = max(180 px, share x section width), never more than the section; the picture and the controls are on the left
  and shrink first. The border is dragged with `layout::drag_applied` and the share is saved with the layout (`SavedLayout.applied`, 0.05 to 0.5, 0.15 when the
  file has none). Rows shorten their names to leave room for the Move buttons.

## Measured, not met

### REQ-315 Faster on integrated graphics (2026-11-22, Intel Iris Plus G7, i7-1065G7, VA-API H.264, system ffmpeg 6.1.1)

The reference project is `hw::req315_phase_times` (ignored, real hardware; run with the system ffmpeg: `VIDEOLORE_FFMPEG=/usr/bin/ffmpeg
VIDEOLORE_FFPROBE=/usr/bin/ffprobe cargo test --test ffmpeg_integration req315 -- --ignored --nocapture`): three cuts of a 1080p, 30 fps,
8 Mb/s H.264 clip (30 s in all) with a fade between the first two, so it is re-encoded. The phase times come from the progress events (85% =
the hardware encode is done, 90% = the strict decode, 100% = the samples). Median of three runs:

| | Best quality (x264) | Faster, 2.1.0 check | Faster, concurrent check |
|---|---|---|---|
| total | 12.3 s | 12.2 s (0.7% lower) | 11.3 s (8.7% lower) |
| hardware encode | | 7.55 s | 7.5 s |
| strict decode | | 2.4 s | 2.0 s (at the same time as the samples) |
| three samples | | 2.25 s | at the same time; the check takes 3.75 s in all |

What was taken from ADR-218: lever (a), the strict decode and the three sample renders run at the same time (`Attempt::check` runs
`strict_decode` and `check_ranges`, the latter one thread for each sample; every process still goes through the runner's slots, so the
cap of three processes holds). The strict decode, the samples and the metadata test are all kept, in the same order of failures. Not taken: lever
(b), the metadata pass is 0.1 s; lever (c), decoding the file with the hardware decoder, would cut the CPU of the check (2.6 s of processor
for 300 frames against 0.9 s) but a hardware decoder is more forgiving than the software one that the check uses on purpose, which is a
weaker check. Running the four processes at once is limited by the processors (4 cores), not by the waiting: decode and samples at once
(11.2 to 11.3 s) is no better than the decode at once with the samples one after another (11.2 s). **The target (at least 20% lower than Best
quality, i.e. 9.8 s) is not reached: 8.7% lower is measured. REQ-315 stays `measured-not-met`**; the remaining time is the hardware encode phase
(7.5 s, decode of the sources and upload), which is not part of the check.

### REQ-177 reused subtitles with speech regions (2026-11-22, Fast model, reference clip)

`speech_regions.rs` (pure) has the 10 ms envelope in dB, the noise floor (10th percentile), the regions (threshold, bridged gaps, minimum 0.1 s),
the dynamic programme that places the words of a cue over the regions, the frozen grid of 81 settings and the selection rule (best score, ties
to the first, 75% minimum on the reference clip). It is wired: `align::align_with` and `retime_with` take `SpeechRegions` (the regions of the source in
source time, `SpeechRegions::new(levels, start, params)`); the words of a matched cue (found or not) are put over the regions of its window by `place`,
the ones the placement leaves out keep their recognised time; which words were found is still the matching's own result. `stt::recognise_with_level` gives
the levels of the speech from the samples the recogniser already has (the part each piece owns, no second decode) and `run` uses them for `Mode::Align`
and `Mode::Retime` when `speech_regions::FROZEN` holds a setting.

**Measured: at most 10 of 23 matched words within 0.3 s (43%) over the 81 settings** (`v12::speech_region_grid_on_the_reference_clip`, ignored; the
recognised times give 7 of 23; the target is 90%, a setting needs 75% to be eligible). The clip's speech has no pause the envelope can see: one
region for each of its four cues, so the words of a region are shared by their length, and the words of this clip are not said at a pace that
follows their length (a hybrid that keeps the recognised times inside the region and takes only the region's start for the first word gave 7 of
23). So `FROZEN` is `None`, the old path (the recognised times) stays the default, and `v12::alignment_matches_real_speech` stays ignored. The check on
the second clip is `v22::the_frozen_speech_region_setting_is_checked_on_the_second_clip_when_it_exists`: it reads
`tests/fixtures/align/second-clip-30s.wav`, says SKIPPED while the file is absent, and fails once the file is there until its check is written (it cannot
pass while no setting is chosen). REQ-177 stays planned.

## Not built

- The whole-sphere (360) export ignores the order, as REQ-321 says; the flat camera view keeps it, and its Crop does nothing, as before.
- A text below Change aspect is drawn by a matte (two renders of its document and a blend), so an order of that kind costs more than the others.
- The audio fades keep their place after the sound group (ADR-225); the Fade row is the picture's.
- REQ-315 levers (b) and (c) and the speed of the hardware encode phase; REQ-177 until a pace-aware or second method is found (roadmap).

## Tests

`timeline` (copies), `app/clarity_tests`, `app/applied_tests` (rows, cache, sound rows), `ui/clarity_ux_tests`, `ui/frame_ux_tests`, `picture`, `render/tests`
(picture replaced by a larger one, sound order), `lut/store` (`prep_tests`), `app/lut_tests` (preparation), `text/size_tests`, `project/v13_tests`,
`ui/text_ux_tests` (the field), `ui/layout` (`applied_tests`), `ui/applied_ux_tests` (the column and the picture rows), `order_chain`, `effect_order`,
`project/order_tests` (every row, 2.1.0 expansion, format 13), `render/overlay_tests` (the builds), `speech_regions`, `align` (regions), `stt` (levels),
`tests/ffmpeg_integration/v14.rs` (sound order, real ffmpeg), `v22.rs` (blur), `v22order.rs` (the picture group, real ffmpeg, preview against export),
`hw.rs` (`req315_phase_times`, ignored) and `v12.rs` (`speech_region_grid_on_the_reference_clip`, ignored).

Tests replaced because they contradicted an amended requirement: `app/applied_tests::the_widget_lists_the_movable_rows_top_first_then_the_fixed_rows`
(Crop is a movable row now: three rows in the picture group, `movable()` is 3). Test changed in form only: `effect_order::tests::p()` (a `Present` with new fields).
