# Architecture: 2.3.0 overview (planned)

Part of [Architecture](architecture.md). Status: planned (2026-11-22). Requirements: REQ-324 (Add media), REQ-325 to REQ-331 ([overlap media](business-overlap.md)), REQ-326 ([frame](business-frame.md)), REQ-280 to REQ-283 and REQ-332 ([effects](business-effects-frame.md)), REQ-320 to REQ-322 ([applied effects](business-applied-effects.md)), REQ-295 ([splash](business-splash.md)), and the Enlarge word fix REQ-333, which fixes REQ-309 ([subtitle styles](business-subtitle-styles.md)). The overlap design is in [overlap media](architecture-2.3.0-overlap.md).

## Overview
Version 2.3.0 moves more of the edit into the clip. A picture placed over the video becomes an **overlap** that belongs to one clip (it moves, is cut and is removed with that clip); the Fit and empty-space choices of the whole video are offered for each clip; Volume and Change speed become effects in the Effects list; Crop is removed. The project file becomes format 14. Older versions refuse it.

## Components
| Part | Where | Role in 2.3.0 |
|---|---|---|
| Overlap model | `src/overlap.rs` (new, pure) | overlap type, clamp, cut, split, removals of an edit, speed rescale |
| Timeline and project | `src/timeline.rs`, `src/project.rs`, `src/project/v13.rs` (new) | `Clip.overlaps`, format 14, one-time conversion of format 13 |
| Order | `src/effect_order.rs`, `src/order_chain.rs` | `overlap:<id>` rows in the picture group; Crop row gone |
| Render | `src/render/plan.rs`, `graph.rs`, `steps.rs`, `inputs.rs`, `overlay.rs` | overlap drawing in three paths ([overlap](architecture-2.3.0-overlap.md)); Crop step gone |
| Picture and probe | `src/picture.rs`, `src/media.rs` | GIF size header; 16 megapixel check for overlap files |
| Preview | `src/preview.rs`, `src/hwdec.rs` | overlap files are never hardware-decoded |
| Settings | `src/settings.rs`, `src/splash.rs` | "Never" removed; a stored "Never" reads as "Once for each version" |
| Subtitle styles | `src/word_styles.rs`, `src/ass.rs`, `src/measure.rs` | Enlarge word draws the word at its measured place |
| Interface | `src/ui/effects.rs`, `popups.rs`, `media.rs`, `frame_handles.rs`, `applied_effects.rs`, new `overlap_lane.rs` and `removal_popup.rs` | Overlap media, Volume and Change speed effects, Fit and empty space per clip, Add media button, removal warning |

Module rules: `overlap.rs` depends on no interface, application or render module; the module-cycle test stays green ([architecture-components](architecture-components.md)).

## Main technical decisions
- **Overlap bound to a clip (REQ-329):** stored in the clip; the times are counted from the clip's start, so a ripple moves the overlap with its clip, a trim or split cuts it, and deleting the clip removes it.
- **Warning before removal (REQ-329):** the edit first lists the overlaps it would remove; if there are any, one popup asks once; Cancel changes nothing; one undo restores everything.
- **Format 14 (one bump, ADR-230):** written always; 13 and older refuse it with their newer-version message; 13 is converted at load in memory with one status line (crop dropped, picture items under a clip become overlaps; ADR-241).
- **Once, then transparent (REQ-328):** a GIF or MOV overlap plays once from its start; ffmpeg's overlay passes the main picture when the overlap input ends; a PNG stays shown.
- **Shape (REQ-327):** only Ctrl (Command) with an edge changes the box's ratio: a MOV is cropped to the box, a GIF or PNG is stretched.
- **Clip Fit and empty space (REQ-326):** the storage already held them per clip; the row offered only the shape and forced one Fit. The row now offers the same choices as the whole video; a first choice is Fit on width and black.
- **Volume and Change speed as effects (REQ-280, REQ-281):** the stored data is unchanged (the keys of 2.2.0 become the clip's Volume; the speed becomes Change speed). Change speed can be applied to all clips as copies.
- **Crop removed (REQ-283):** the model writer, the render step, the widget row and the Clip details row go; 13 crops are dropped with a count.
- **Start screen (REQ-295):** two choices in Settings; the stored value "Never" is mapped explicitly (ADR-240).
- **Enlarge word (REQ-333, fix of REQ-309):** the enlarged word is drawn alone at its measured place, the other words keep their place (ADR-235). The earlier note described this design; the build did not have it.
- **Add media (REQ-324):** the same action as File > Import media….

Decided: one shared overlap lane on the timeline (overlaps stacked in sub-rows, three rows visible, the rest by scroll); Change speed rescales the overlaps of its clip; overlay files above 1920 x 1080 are drawn at half size in the preview and at full size in the export.

## Non-functional targets
- Projects without overlaps, Volume keys, speeds or crops: the export command line is identical (gate, golden tests).
- Format 14 round trip and 13 conversion: gate. Older versions refuse 14: gate.
- Overlay alpha: transparent pixels show in preview and export (gate, fixtures).
- Enlarge word: centre within 2% of the picture height (gate).
- UI frame within 200 ms with the overlay lane and the warning open (gate, existing rule).
- Overlay cost: at most 10% more preview frame time for a 1080p still; export with one overlay at most 1.5 times without (estimate, measured, not gated).

## Security
- Overlay files are untrusted media: the size is read from a bounded header or ffprobe before decoding; over 16 megapixels is refused; paths never enter a filter text.
- Project format 14 input is validated; damaged overlaps are dropped with a count.
- No new crate, no unsafe code, no network. Overlay files are not copied into the project.

## Dependencies
No new crate. Bundled ffmpeg 9.0.2 (pinned, unchanged): PNG, GIF and MOV decoders; `overlay` with `eof_action`. Test fixtures: a small transparent GIF and a small MOV with alpha, committed with their checksums.

## Known limitations (planned)
- A MOV without an alpha channel is drawn opaque; a MOV codec that carries alpha only in some profiles may be drawn opaque too.
- A MOV overlay has no sound; overlays are not drawn in a 360 export.
- Overlays have no keyframes, no rotation and no own video track.
- Overlay files above 1920 x 1080 are drawn at half size in the preview only: the preview is softer than the export for them (the export is full size).
