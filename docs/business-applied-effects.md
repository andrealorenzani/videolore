# Business — Applied effects and their order (2.1.0)

Back to the [index](business.md). Built in 2.1.0, with [Subtitle and text styles](business-subtitle-styles.md) and the LUT import in [Apply LUT](business-lut.md). The current rules it changes are in [Effects](business-effects.md), [Text](business-text.md) and [Frame](business-frame.md).

Before 2.1.0 the picture was built in a fixed order: frame and Scale, Move, Change aspect, then Apply LUT, then the text items and subtitle lines on top. Nothing let the user choose the order. This release adds a list that shows what is applied to the selected clip and lets the user choose where a LUT or a text sits in the order.

## Features
- **Applied effects widget**: a new part of the preview, about 15% of its space, that lists the effects of the selected clip.
- **Order**: the top row is applied last (drawn on top); the lower rows are applied first (drawn underneath).
- **Reorder**: a LUT, a text item, a picture item or the subtitle lines of the clip can be moved up or down, so a LUT can colour a text or leave it alone.
- **Whole-video marks**: effects applied to the whole video are shown on each clip they affect, with a mark.

## Requirements

### Applied effects widget
- REQ-302 (implemented): The preview has an **Applied effects** widget of about 15% of the preview space (assumption: a strip below the picture; the picture shrinks first, REQ-068). It lists the effects of the selected clip, or of the clip at the playhead when no clip is selected (assumption). Rows, from the top: the movable rows in the order of REQ-304 (Apply LUT, each text item and picture item that overlaps the clip in time, the clip's subtitle lines as one row "Subtitles"), then the fixed rows in the order the picture is built: Crop, Scale, Move, Change aspect, Fade IN/OUT, Volume, Clean audio, Speed (assumption: fixed rows sit below the movable ones, because the picture is built before them). Each row shows its icon, its name and, for texts, the time range. A click on a row opens its popup. With no clip, the widget says "Select a clip to see its effects". The widget scrolls when its rows do not fit. Acceptance: on a clip with Apply LUT, then one text added: the text row is above the LUT row; a clip with no effects shows only the empty text; a click on the LUT row opens the Apply LUT popup; at 800x500 with the Preview section at its minimum every row is reachable by the scroll.

- REQ-303 (implemented): An effect applied to the whole video (REQ-147) is shown in the widget of each clip it affects, with a **whole video** mark. The mark is inferred, not stored: an effect is marked when the timeline has two or more clips and every clip has it with the same settings (Volume, Scale, Move, Apply LUT, Clean audio, Speed); the project's Change aspect is marked on every clip; Fade IN is marked on the first clip and Fade OUT on the last clip. Acceptance: whole-video Fade IN/OUT on clips 1 to 3: the Fade row with the mark shows on clip 1 and clip 3 only; whole-video Volume shows the mark on each clip; a whole-video Change aspect shows the mark on each clip.

### Order and reorder
- REQ-304 (implemented): The order of the movable rows is chosen by the user in the widget: with **Move up** and **Move down** buttons, by drag, or with Alt+Up and Alt+Down while a row has the focus or the pointer. The order decides the picture: a row higher up is applied later and drawn on top. The movable rows are Apply LUT, each text item, each picture item and the subtitle lines of the clip. The fixed rows (Crop, Scale, Move, Change aspect, Fade IN/OUT, Volume, Clean audio, Speed) keep their place and cannot be moved. The order belongs to the clip: a text that overlaps two clips can be above the LUT in one and below it in the other. A new LUT or picture item is put on top once the edit is done, so a LUT applied to a clip with texts grades the texts; a new text is on top too, as the later item. Each move is one undo step named after it (REQ-074, for example "Undo Move LUT down"; a drag is "Move effect"). Acceptance: a text above a LUT on a clip shows the text in its own colours in the preview and in the export; the same text moved below the LUT shows the LUT colours; one undo restores the order; a text overlapping clips 1 and 2 with the LUT only on clip 2 gets the order of clip 2 there and no change on clip 1.

- REQ-305 (implemented): Texts overlapping on the picture: the lower row is covered by the higher row where the two boxes overlap, in the preview and in the export. Moving a row swaps which text covers which. Projects saved before 2.1.0 keep the drawing order they have today (assumption: a later text is drawn on top of an earlier one, not yet confirmed by Andrea), so their picture does not change. Acceptance: two texts overlapping at the same time, A above B: A covers B in the preview and in the export; after Move down on A, B covers A in both; a project saved in 2.0.4 with overlapping texts exports the same picture as before.

### Project file
- REQ-311 (implemented): The project stores, for each clip, the order of its movable rows (REQ-304), and the new settings of REQ-306 to REQ-310 (shadows, highlight colours, the Enlarge word size, the new styles). This is project format 12. Projects saved by 2.0.4 or earlier open in 2.1.0 with the same look and export (the defaults of the edge cases). A 2.1.0 project is refused by 2.0.4 and older with their newer-version message, and the file is left untouched (REQ-029). A damaged order entry is dropped with the line "A damaged effect order was repaired" in the "Some files could not be found or were repaired" notice, without a crash (REQ-057). Acceptance: a 2.0.4 example project opens and exports as before; a 2.1.0 file with a moved row is refused by 2.0.4 with its message; a damaged order entry is dropped with that line in the notice, without a crash.

## Edge cases and rules for the builders
- **Old projects**: a project saved before 2.1.0 has no stored order. It opens with the order that the picture has today (LUT below the texts and subtitle lines; texts in their present stacking), so its look and export do not change (REQ-311).
- **Render order**: the fixed rows keep the order of the render graph. Only the place of the movable rows relative to each other and to the picture layers changes. A LUT is always applied after Scale, Move and Change aspect, as today.
- **Export and preview parity**: both use the same order; the export is always a re-encode when the order differs from the default (REQ-023).
- **360 clips**: the order applies to anchored texts and anchored subtitle lines (REQ-257) and to the whole sphere picture in the same way as on a flat clip; those anchored items are fixed rows (see Known limitations). A 360 video export (the whole sphere) ignores the order; the flat camera view export keeps it.
- **Blink**: a hidden text (REQ-162) takes no place in the picture; its row stays in the widget.
- **Shadow and readability**: the order decides whether a LUT colours a text's shadow and outline too; the shadow of a text is part of the text (REQ-306).
- **Undo**: each reorder, and each change of a row's place in the list, is one undo step; the widget itself does not change the project.
- **Project file**: the order is stored in the project per clip (REQ-311).

## Key decisions (2.1.0)
- [MINOR] The movable rows are Apply LUT, texts, picture items and the subtitle lines; the other effects keep their place, because moving them would change the picture build. Reason: the user asked for the LUT and the texts; the other order choices are not asked for and would change many existing outputs.
- [MINOR] The order is per clip, not per timeline: one text can have a different order on each clip it overlaps. Reason: the widget shows one clip, and a whole-video LUT is a copy on each clip (REQ-147).
- [MINOR] A new text or LUT is applied last (top). Reason: the user adds an effect and then decides whether an earlier one is applied to it.
- [MINOR] The rows follow the user's list, and the list is the "order of application" the user asked for; the fixed rows are shown below the movable rows because they are applied first.

## Known limitations
- Overlapping subtitle lines (REQ-169) are not ordered by the widget; subtitle lines of one clip stay in their own order.
- Texts that overlap several clips can have a different order on each clip; the widget shows the order of the selected clip only.
- The texts and lines fixed to the sphere of a 360 clip keep fixed rows; they cannot be moved in the order (decision Q-3, 2.1.0).
- A 360 video export (the whole sphere) ignores the order.
- The subtitle lines that go to a separate subtitle track are not drawn by the clip, so the order does not apply to them.

## Out of scope (2.1.0)
- Reordering fades, volume, Clean audio, crop, speed, Scale, Move or Change aspect.
- Ordering of individual subtitle lines in the widget; layers on the timeline (multiple video tracks stay out of scope).
- Keyframes for the order, and a separate order for each word.
