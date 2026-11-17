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
- They are actions in the shared table: toolbar group (separated, after the existing actions, so on a narrow window they move first into the "»" menu), Effects menu, right-click menus of the preview and of a subtitle marker, tooltips and shortcuts (a test keeps shortcuts unique). Set POV is an action too (preview button, Playback and Effects menus, right-click menu).
- The toolbar will have 15 commands (12 up to 1.2.5); at 800 pixels wide every one stays reachable.

## Tests
- Pure: handle hit-testing, resize with the opposite corner fixed, limits (25%, 400%, picture edge), align, reset, group size.
- With `ffmpeg`: the burned text size and centre match the preview within 2% of the picture height; the output has no box.
- Interface (headless): box drawn only when the option is on, corner drag changes size, one undo restores, greyed reasons, 800x500 reachability.
- Setting: show-boxes survives save and reload; hostile settings values give the default.
