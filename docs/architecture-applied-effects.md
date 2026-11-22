# Architecture: Applied effects widget and order (built 2.1.0, not released)

Part of [Architecture](architecture.md) and [2.1.0 overview](architecture-2.1.0-overview.md). Requirements REQ-302 to REQ-305 and REQ-311 ([business](business-applied-effects.md)). Status: planned.

## Widget
- Built 2.2.0 (REQ-317): a column on the right of the Preview section, beside the picture ([layout](architecture-2.2.0-layout.md)); the rows of all effects, in two groups, are reordered inside their group ([order](architecture-2.2.0-order.md)). The text below describes 2.1.0.
- A strip of about 15% of the Preview section, below the picture; the picture shrinks first. It shows the effects of the selected clip, or of the clip under the playhead when none is selected.
- Rows from the top: the movable rows in their order (Apply LUT, each text item and picture item that overlaps the clip in time, the clip's subtitle lines as one row "Subtitles"), then the fixed rows in build order (Crop, Scale, Move, Change aspect, Fade IN/OUT, Volume, Clean audio, Speed).
- Built 2.2.0 (REQ-303 amended): no "whole video" mark. A whole-video effect is a copy on each clip (REQ-147) and shows as that clip's ordinary row. The 2.1.0 mark, inferred from equal settings on two or more clips, is removed.
- Built 2.2.0 (REQ-313): the rows are kept and rebuilt only when the selected clip, its order or its effects change (ADR-216); 0 rebuilds over 60 frames without a change.
- Click opens the row's popup. Movable rows move by drag, by Move up and Move down buttons, and from the keyboard. Each move is one undo step named after it, for example "Move LUT down".
- With no clip: "Select a clip to see its effects". The strip scrolls when its rows do not fit.

## Order model (`src/effect_order.rs`, pure)
- Movable rows: `Lut`, `Text(id)` (flat text items), `Picture(id)` (picture items), `Subtitles`.
- The order of each clip is stored in the project (`clip_order`, format 12, sorted by clip id, empty entries dropped). A new row (LUT, text, picture item) goes to the top.
- Rows that are not on a clip are not stored. A stored id that no longer exists is dropped on load with a line in the repair notice.
- Projects saved before 2.1.0 have no stored order. They use the **legacy order**, which is the stacking of the 2.0.4 picture (verified by reading the code): from the bottom, the LUT (in the clip's own chain, under the texts), the picture items, the subtitle lines, then the flat texts in item order. A later text is drawn over an earlier one, so the texts keep their stacking.

## Render graph
- A clip whose order equals the legacy order is built exactly as in 2.0.4: its argv and output do not change.
- A clip with another order is built by the overlay stage (`src/render/overlay.rs`): the fixed chain first (crop, scale, move, frame, fades), then the movable rows from the bottom up, one filter step each:
  - LUT: `lut3d` on the shared Work copy of the LUT (see [LUT](architecture-lut.md#large-luts-210-planned)).
  - Texts and subtitle lines: `subtitles=` on a per-clip ASS file with clip-local times; only the events of that row.
  - Picture items: `overlay` with an enable window over the clip.
- The LUT of such a clip comes after the frame composition and the fades (the widget order is the render order). Decided: the whole frame is graded, background and bars included. Consequence: a LUT that lifts black shows on a faded frame.
- Preview and export use the same builder, so the picture is the same in both.
- 360 clips: sphere-fixed texts and lines keep their 2.0.4 path (burned before the camera projection). The widget lists them as fixed rows, not movable (decided).

## Format 12
- Adds `clip_order`. Older programs refuse a format 12 file with their newer-version message and leave it untouched.
- A file saved by 2.0.4 or earlier opens with the legacy order: the same look and the same export.

## Tests (planned)
- Model: move up and down at the ends; insert on top; repair of unknown ids; legacy order of a 2.0.4 example with overlapping texts.
- Graph: legacy clip argv identical to 2.0.4 (golden); a text above the LUT and below it differ in colours; overlapping texts A above B cover B in the preview and export; one undo restores the order.
- Widget: rows and their order through the accessibility tree; the empty text; the scroll at 800 x 500 with the Preview section at its minimum.
- Project: format 12 round trip; 2.0.4 file opens unchanged; damaged order entry dropped with the notice.

## Risks
- Legacy stacking must stay byte-identical; a gate test pins it before the overlay stage is added.
- A new filter step per movable row adds graph size; measured on a clip with several texts.
