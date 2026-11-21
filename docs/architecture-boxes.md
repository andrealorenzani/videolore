# Architecture: boxes, size and effect commands

Part of [Architecture](architecture.md). Built in 1.2.6 (REQ-096..099). The 360 camera is in [360 video](architecture-360.md).

## Boxes on the picture
- A box is a rectangle in the preview (fractions of the picture) around one movable object, with corner handles. Today the only objects are subtitle lines; the box layer takes a list of boxes, so a future object type needs no new interaction code.
- The boxes are computed by the existing text-measuring code (same font, size and wrap width as the export) and drawn only by the preview; nothing is added to the exported picture. A test compares an export with and without boxes visible.
- View > Show boxes on the picture is a program setting (an optional field in the settings file, on by default; the file stays at version 2). When off, no outline or handle is drawn and corner resizing is unavailable; dragging the text itself still moves it as in 1.2.

## Moving and resizing
- Pure geometry (no egui): which handle is under the pointer, the cursor to show, and the resize rule. Dragging a corner scales the box proportionally with the opposite corner fixed; the scale is the pointer's projection on the box diagonal, limited so the box stays inside the picture and the size stays between 25% and 400%.
- A subtitle line gets an optional **size** (thousandths of the preset's text size, 250 to 4000, none = 100%), stored per line like the position, so it survives a change of output format. In the subtitle file the line gets a font-size tag (and proportionally scaled outline and shadow); preview and export use the same file, so the size is identical. A line without a size produces exactly the 1.2 output (golden test). Text edited after enlarging can push the box over an edge, as for positions.
- Several lines selected: all get the dragged line's size; each keeps its own position. Position and size of one drag are one undo step.

## Effect commands
- **Align horizontally / vertically:** each selected line's centre is set to the middle of the picture on that axis; the other coordinate and the size stay. One undo step. Greyed with "Select something on the picture first" when no line is selected.
- **Reset:** with lines selected, clears position and size of each (back to the style's place, 100%); greyed with a reason when all are already original. With no line selected and a 360 clip at the playhead, it returns the live camera to front, level, default zoom and keeps the POVs (no undo: not part of the project). "Reset position" of 1.2 is the same command.
- They are actions in the shared table: toolbar group (separated, after the existing actions, so on a narrow window they move first into the "»" menu), the menus (Effects until 1.3.0; **Edit since 1.4.0**, where the Effects menu lists only effects), right-click menus of the preview and of a subtitle marker, tooltips and shortcuts (a test keeps shortcuts unique). Set POV is an action too (preview button, Playback and Effects menus, right-click menu).
- The toolbar has 15 commands (12 up to 1.2.5); at 800 pixels wide every one stays reachable. Since 1.3.0 the Context Menu group (below) is shown in addition, only when it applies.

## Context Menu and text boxes (1.3.0)
- The Context Menu is a toolbar group shown only when it has something to act on (REQ-106): **Align horizontally, Align vertically, Reset** when a subtitle line is selected; **Set POV and Reset view** when the playhead is over a 360 clip (Set POV also with nothing selected). In 1.3.0 a selected text item showed no group (since 1.4.0 it does, see below). The rule is one pure function over the selection and the playhead. The commands stay the shared table actions, so the menus and shortcuts do not change.
- Text items get the same box, corner handles and resize as subtitle lines (REQ-111); the box layer already takes any list of boxes. On a 360 clip the box of an anchored item is the projection of its sphere area into the view (see [360 video](architecture-360.md#anchored-text-and-subtitles-130)).
- Align and Reset apply to subtitle lines only in 1.3.0; their greyed-out state is removed from the group (the menus keep their reasons).

## Text items, width and dragging (1.4.0)
- A selected **text item** shows the same Context Menu group: Align horizontally, Align vertically and Reset (REQ-106, REQ-115). Several selected objects get the command each; one command is one undo step.
- A text box changes **width only** through its right-edge handle (corners do nothing); see [text](architecture-text.md#box-best-fit-and-place-140). Its size follows the width by the best-fit rule.
- **Dragging limit (REQ-117):** a text item or a subtitle line can be dragged past the edges of the picture; the centre of its box must stay on the picture, so the box can always be grabbed again. On a 360 clip the limit is the look-around view.
- The box of a subtitle line keeps its size rule (25–400%, corners resize within the picture); only the drag limit changed.
- When the boxes are hidden (View > Show boxes off), dragging the text itself still moves it, as before.

## Visible edges (1.6.1, REQ-145, REQ-146)
- A box partly outside the picture (Change aspect cuts a side, a subtitle line or text item sits on the edge, or Scale is above 100%) keeps its handles on the **visible part**: a corner outside the frame is reached through the visible part of its edges.
- The visible edge of a box is a resize handle. A subtitle line's edge is proportional like its corners (25% to 400%, the opposite edge fixed). A text item keeps only its width handle, on the visible part of its right edge; it gets no left handle.
- The Scale box of a picture uses the same rule (see [frame](architecture-frame.md#visible-edges-161)). One drag is one undo step.

## One box rule (2.0.4, QG-16)
- Every box of a text or subtitle line is computed by `placement::padded_line_rect`: the measured box and the preview's hit boxes (`ui/placement.rs` `line_box`) use the same rule and the same font and wrap helpers, so the box that is dragged and the box that is exported are one rule. Picture items use their text item's box (see [text](architecture-text.md#picture-items-204)).

## Tests
- Pure: handle hit-testing, resize with the opposite corner fixed, limits (25%, 400%, picture edge), align, reset, group size.
- Pure (1.3.0): the Context Menu rule table (nothing, line, 360 playhead, text only, both).
- Pure (1.4.0): the rule table with a selected text item; width handle (left edge fixed, 10–100%, clamp); centre clamp to the picture for dragging past edges; align and reset of text items.
- With `ffmpeg`: the burned text size and centre match the preview within 2% of the picture height; the output has no box.
- Interface (headless): box drawn only when the option is on, corner drag changes size, one undo restores, greyed reasons, 800x500 reachability.
- Setting: show-boxes survives save and reload; hostile settings values give the default.
