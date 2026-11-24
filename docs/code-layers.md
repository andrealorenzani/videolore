# Code — effect order, overlaps and project formats 12 to 14 (2.1.0 to 2.3.0)

Back to the [index](code.md). REQ-300 to REQ-333. Symbols and tests: [code-index.md](code-index.md) or `scripts/req-code.sh REQ-NNN`; measurements: [code-measurements.md](code-measurements.md).

## Large LUTs and their preparation (REQ-300, 301, 312)
| Where | What |
|---|---|
| `lut/parse.rs` | `parse_reader`: one pass over a `BufRead`, 256-byte line buffer, values quantised to 16 bits as read, progress and stop every MiB (`LutError::Cancelled`) |
| `lut/mod.rs` | `MAX_3D` 256, `MAX_BYTES` 512 MiB, `MAX_LINE` 256, `MAX_WHOLE` 64 MiB (`.3dl`/`.csp`); `Lut` 6 bytes an entry (144³ = 18 MB); `write_cube` (streamed), `cube_len` |
| `lut/store.rs` | `read_with` (a file changing while read is refused), `cached` (192 MB LRU), `peek`, `Load` (thread + channel, `percent`, stops when dropped), `copy_name`/`write_copy` (`lut-<hash16>.cube` in the Work folder, temp then rename), `Prep` (parse + a Work copy in each preview folder `play`, `still`; `is_prepared`) |
| `render/graph.rs` | `lut_chain` names the shared file; `Graph::lut_files`; `write_command_files(dir, files, luts)` |
| `app/lut.rs` | a file > 1 MiB not cached is read by a `Load` (`PendingLut`, `tick_lut_load`, "Reading name... N %"), applied as one undo step; `unready_luts` are left out of `shown_project` until `tick_lut_prep` ("Preparing LUT N %": read+write 60%, then 0.75 µs an entry up to 95%) |
| `project.rs` | `lut_copies`: a table above 20 MB of text is not copied, the project keeps the path (REQ-301) |

The maintainer's 144-point files are read by an optional test that skips when the folder is absent; they are never written or copied.

## Text and subtitle styles (REQ-306 to 310, 318, 319, 323, 333)
- libass (spike): in `BorderStyle 3` the box is the **outline** colour and its shadow the **back** colour, so `Shadow` is the distance and `BackColour` the shadow colour for every style. `\ko` draws no outline before its turn (with `\2a&HFF&` no fill); `\alpha&HFF&` hides a run and keeps its place.
- `base/shadow.rs` `Shadow { px, colour }` (0..=20 px at 1080); `TextItem.shadow`; `ass::style`, `text_styles` (`Text_<px>_<RRGGBB>`, `TextBox_...`), `document_styled`, `export_file_styled`, `shadow_px`; `subtitles::StyleSettings` (shadow, `highlighted`, `not_yet`, `enlarge` 12 px), `Preset::{EnlargeWord, Typewriter}`, `follows_words`; `word_styles` pure plans: `typewriter` (one event per word, typed with `\ko`), `enlarge`; `Command::SetSubtitleStyle` ("Subtitle style").
- **Enlarge word (REQ-333, 2.3.0; replaced by REQ-334 in 3.1.0, see [code-audio-tracks.md](code-audio-tracks.md)).** `measure::Shaped::place_words` places the words in their wrapped lines (still used); the 2.3.0 centre rule (`word_centres`, `enlarge(…, anchors)`) is gone. 360 sphere-fixed lines draw both styles as plain text.
- **Text size (2.2.0).** Best fit of the box (`measure::fit` via `text/layout.rs`) capped by `TextItem::cap()` = `max_size` (thousandths of the height, 20-500, written only when set; `text::parse_max_size`) or 100. `text/size_tests.rs` pins the 2.0.4 sizes (52.3, 56.5, 43.2, 43.1, 108 px at 1080p). `text/events.rs` anchors at the box centre (`\an5`).

## Whole video as copies (2.2.0; REQ-083, 129, 131, 147, 148, 200-202, 303)
- `FrameTarget::Whole` means "a copy on every clip" when applying (`add_scale`, `add_move`, `set_lut`); reading keys on `Whole` finds none. A double-click on Scale/Move adds a copy to each clip lacking one (one undo step) and opens the popup of the clip under the playhead (`AppState::clip_at_playhead_or_first`). Apply LUT: `Instance::Lut(Whole)` ("Apply LUT — every clip"). Only Change aspect says "— whole video" (`popup_title`). Fade IN/OUT stay flags of the first/last clip (`FadeStart`, `FadeEnd`); Change aspect is not copied.
- Picture limit (316): `picture::MAX_PICTURE_PIXELS` 16,000,000 checked at the PNG header; `picture::usable` re-reads it before drawing.

## Order of the effects (REQ-302 to 305, 320 to 322)
| Where | What |
|---|---|
| `effect_order.rs` | `Row` (`Lut`, `Text(id)`, `Picture(id)`, `Overlap(id)`, `Subtitles`, `Volume`, `Clean`, `Frame`, `Scale`, `Move`, `Fade`, read-only `Crop`, `Unknown`; names in `NAMED`/`WITH_ID`), `Present`, `legacy` (default, top first: flat texts later on top, lines, overlaps, pictures, then `FIXED` = Fade, Frame, Move, Scale, LUT, Crop), `resolve`, `to_store` (nothing for the default), `is_changed`, `moved`, `dragged` |
| `order_chain.rs` | sound group: `DEFAULT_SOUND` (Clean audio then Volume), `resolve_sound`, `sound_swapped`, `with_sound`, `with_picture`, `moved_sound`, `expand_2_1_0` |
| `timeline.rs` | `Timeline.clip_order` (`ClipOrder { clip, rows }`, written when not empty), `order_of`, `set_order`; split gives both halves the order |
| `project/order.rs` | `Project::clip_rows`, `order_of`, `changed_orders`, `set_order`, `put_on_top`, `repair_orders` ("A damaged effect order was repaired") |
| `app/applied.rs` | `AppState::applied()` (rows cache keyed by clip + `rows_serial` + counts; `applied_builds()`), `Command::MoveRow { clip, row, mv }`, `RowMove::{Up, Down, To}`, `move_label` |
| `ui/applied_effects.rs` | the column right of the Preview (`layout::applied_width` = max(180 px, share), `drag_applied`, `SavedLayout.applied` 0.05-0.5, default 0.15) |

- A row belongs to a clip when its item overlaps the clip in time. A new text, picture, overlap or LUT goes on top of the other movable rows (`put_on_top`); a new Frame, Scale, Move, Fade or sound row takes **its default place** (deviation from REQ-321 "on top": a Scale above Change aspect would scale the blurred background).
- Moves are one undo step named after the row ("Move Scale up", "Move LUT down", "Move effect" for a drag). `Segment.volume_first` orders the two sound filters in `render/graph.rs`; a sound swap never uses the overlay stage.
- **Three builds**, chosen by the resolved order (`overlay_form`, `needs_steps`): the default order → the legacy chain (golden argv unchanged, nothing stored); a 2.1.0 order (movable rows above the rest) → the overlay stage; any other → the step builder.
- **Overlay stage** (`render/overlay.rs`): `attach(project, segs, geometry, from, src, burn_lines, steps)` gives changed clips their `OverlayStep::{Lut, Ass { name, doc }, Pic}` bottom to top; `graph.rs:overlay_chain` appends them after the fades (LUT `format=gbrp,lut3d=…`, a text `subtitles=f=ov<clip>-t<id>.ass`, lines `ov<clip>-s.ass`, in clip time); `plan.rs` leaves the changed clips out of the global document (`text::project_events_in`, `PicOverlay.pieces`); `FrameKey.order`.
- **Step builder** (`render/steps.rs`, ADR-224): the stream is a *source* (Crop of format 13 cut it), a *layer* (fitted to the frame on a transparent RGBA surround: LUT and Fade via `format=gbrap`, Scale/Move on a transparent canvas so their order matters, a text drawn by a **matte**: the document on black and white, `255 - (white - black)` = coverage, `black * 255 / covered` = colours, `alphamerge`), or a *frame* (composed over the background at the Change aspect row; `blur_chain`). Texts use `doc_shift` = the clip's start. The whole-sphere 360 export ignores the order; the flat camera view keeps it. Real-ffmpeg tests: `tests/ffmpeg_integration/v21.rs`, `v22order.rs`.

## Overlap media (REQ-325, 327 to 331)
- **Model** (`src/overlap.rs`, pure): `Overlap { id, path, kind: Still | Motion, from, len, place, width, height, reshaped, src_w, src_h }`, `from` relative to the clip start. `is_valid`, `clamped`, `kept_part`, `rescaled`, `rect` (ratio kept in the box unless `reshaped`), `is_large` (> 1920x1080), `drag_ends`, `sub_rows`, `lost`.
- **Storage (deviation from the plan, which put them in `Clip`, a `Copy` type):** `Timeline.clip_overlaps: Vec<ClipOverlaps { clip, items }>` + `next_overlap_id` (stays 0 until the first overlap), written when not empty.
- **Clip edits:** `split` cuts each overlap into both halves (new id on the right), `trim` drops one with nothing left, `remove` drops the clip's, `merge` moves the second part's, `set_speed` rescales (old/new speed).
- **Warning (329):** `AppState::edit` compares overlap ids before/after; an unconfirmed loss changes nothing and returns `app::overlap::REMOVAL_ASK`; `dispatch` opens `Modal::RemoveOverlaps(n)` holding `pending_removal`; `ConfirmRemoval` reruns it with `removal_confirmed` (one undo step), `CancelRemoval` drops it.
- **Commands** (`app/overlap.rs`): `AddOverlap(path)` (clip under the playhead, 3 s cut to the clip, file ratio, selected, on top), `SetOverlapBox`, `SetOverlapTimes`, `RemoveOverlap`, `SelectOverlap`. `picture::read_overlay` (PNG/GIF from the header, others by ffprobe, > 16 MP refused), `picture::overlay_usable` at use (a MOV is trusted by its size at add).
- **Render:** `render::PicOverlay.look: Look { motion, skip, cover, half }`; `plan::picture_overlays` merges picture items and overlaps; default order draws them after the concat (argv of projects without overlaps unchanged), a changed order as `OverlayStep::Pic` in clip time. Still: looped input (`shortest=1`); GIF/video read once (`-ss` inside it), `setpts` to its start, `eof_action=pass` (328); reshaped video `increase` + `crop`, reshaped still stretched; video stream only; > 1920x1080 halved in the preview only; no overlap in a 360 export; never hardware-decoded (`preview::input_codecs`); `FrameKey.overlaps`.
- **Interface:** Effects row "Overlap media" (shares the first row with Add Text: the section holds 10 rows at 1280x800), action `AddOverlap`; `ui/overlap_lane.rs` (a lane only while overlaps exist, ≤ 3 sub-rows visible, wheel scroll, body and ends dragged); `ui/overlap_box.rs` (click selects, drag moves, edge/corner resizes with the ratio, Ctrl/Cmd at the start reshapes; one command on release); Applied rows `Overlap: <file>`. Fixtures: `tests/fixtures/overlay/alpha.{png,gif,mov}` (64x64 red square on transparent; GIF and qtrle MOV 1 s), test `tests/ffmpeg_integration/v23overlap.rs`.

## Other 2.3.0 changes (REQ-280, 281, 283, 295, 324, 326)
- Volume is an effect with keyframes in the clip's popup (Add keyframe, value, Ease, remove, Remove effect); applying it gives one key at the slider level. Change speed: `EffectKind::ChangeSpeed`, `Instance::Speed(FrameTarget)`, `SetSpeedAll` (double-click: a copy on each clip, one undo step), not keyframed. Crop removed (`Step::Crop`, `crop_filter`, `set_crop`, `SetCrop`); `Clip.crop` is read only (`skip_serializing`).
- `ui::frame_popups::aspect_fields` draws shape, ratio, Fit and Empty space for the whole-video popup and the clip row (`popups::clip_aspect_rows`). `ui::media::ADD_MEDIA` sends `PickImport`. `SplashOption::{Always, OncePerVersion}`; a stored "Never" reads as `OncePerVersion` (`lenient_splash`).

## Project formats 12 to 14 (REQ-311, 330, 332, 029, 057)
- **12** (2.1.0): text `shadow`, `subtitles.style`, `timeline.clip_order`, each written only when not default; out-of-range shadow/enlarge clamped on load; damaged orders repaired.
- **13** (2.2.0): `TextItem.max_size` (out of range dropped: "A text size maximum outside 2 to 50 was removed"); the full row list of an ordered clip, written only when not default.
- **14** (2.3.0): `SCHEMA_VERSION` 14; 1 to 14 open, newer is `TooNew` (file untouched). `project/v13.rs` converts ≤ 13 in memory: crops dropped, a picture item whose start lies in a clip becomes an overlap (its `picture:<id>` row becomes `overlap:<new id>`), an item with no clip stays a picture item; `Project.converted: Option<Conversion>` → one status line "Opening: N clip(s) changed by the conversion (M crop(s) dropped[, K picture(s) made overlaps])"; no undo step, not unsaved. `validate_overlaps` drops damaged overlaps ("N damaged overlap(s) were removed").
