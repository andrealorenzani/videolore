# Business — Editing gestures, autosave and safety

Back to the [index](business.md).

## Autosave and recovery
- REQ-189 (implemented): A project with unsaved changes is autosaved every 2 minutes (MINOR interval) to videolore's private cache folder, never to the project file or next to it. The autosave changes no project file. A normal Save or Discard removes the autosave copy. Acceptance: after an edit and 2 minutes the cache holds a copy with the edit; the project file's checksum is unchanged; after Save the copy is gone; typing and playback do not pause during the autosave.
- REQ-190 (implemented): At the next start, when an autosave copy is newer than the last save of its project (or exists for a project never saved), a dialog offers **Recover**, **Discard** and **Cancel**. Recover opens the copy as unsaved changes of the project (saving then writes the project path chosen by the user, as REQ-026). Discard deletes the copy. No offer appears after a clean exit. Acceptance: kill the process after an edit; the next start offers recovery; Recover restores the edit with the unsaved marker (REQ-027); Discard deletes the copy; after a clean exit there is no offer.

## Timeline gestures
- REQ-191 (implemented): Dragging the left or right edge of a video clip trims it within its source bounds (REQ-009), in steps of one frame, showing the new in and out times while dragging; the timeline closes the gap as trims do. No clip is shorter than one frame. One drag is one undo step. Acceptance: dragging the right edge 1 s left shortens the clip by 1 s (within one frame), the same as the Clip details trim; dragging past a source limit stops at it; one undo restores the clip.
- REQ-192 (implemented): Dragging the start or the end marker of a subtitle line changes that time in steps of 0.1 s; the start cannot pass the end and each line lasts at least 0.1 s. One drag is one undo step. Acceptance: the start marker dragged 0.5 s later moves the start by 0.5 s; dragging it past the end stops at 0.1 s before the end; one undo restores it.
- REQ-193 (implemented): Dragging a fade marker onto another clip boundary moves the fade there (the fade leaves its old boundary, and a boundary keeps at most one fade). Dropping it on an occupied boundary or on empty space changes nothing. Start and end fades of the video follow REQ-083. One drag is one undo step. Acceptance: a fade dragged from boundary 1 to boundary 2 is at boundary 2 and absent at boundary 1; a drop on an occupied boundary changes nothing; one undo restores it.
- REQ-194 (implemented): The keys **[** and **]** move the playhead to the previous and next timeline marker (fades, subtitle lines, text items, clip markers) and select it. They are listed in the shortcuts window (REQ-071) and never fire while typing. Acceptance: with markers at 2, 5 and 9 s, from 0 s **]** goes to 2 s, **]** to 5 s, **[** back to 2 s; while typing in a text field nothing moves.
- REQ-195 (implemented): Ctrl/Cmd+V pastes the clips of the most recent cut or copy made in videolore, whatever the system clipboard holds. The menu and toolbar Paste do the same (amends REQ-071). Copy and Cut also put a marker text on the system clipboard, so other applications see that text when they paste. Acceptance: copy a clip, put text on the system clipboard, press Ctrl/Cmd+V: the clip is pasted.

## Undo and selection
- REQ-196 (implemented): Removing a media file keeps the undo history: only the steps that depend on the removed file are dropped, together with later steps that depend on them. Other steps stay undoable in order. Acceptance: with 10 undo steps and an unused media file removed, all 10 remain; undo of the earlier steps works as before.
- REQ-197 (implemented): With several subtitle lines selected (REQ-090), Delete removes all of them, and two commands **Shift lines earlier by 0.5 s** and **Shift lines later by 0.5 s** move them together, keeping their lengths and never before 0 s. Each command is one undo step. Acceptance: three selected lines are removed by Delete and restored by one undo; a shift later moves all three by 0.5 s; a shift that would pass 0 s stops at 0 s.

## Guides
- REQ-198 (implemented): While a subtitle line or a text item is dragged in the preview, its box centre snaps to the picture's centre lines and thirds (33% and 67%) and to the centre of other items, within 1% of the picture size; a guide line is drawn while it is snapped. **Snap to guides** is a View option, on by default, kept as a program setting. Acceptance: a line dropped within 1% of the centre is centred exactly and the guide shows; with the option off nothing snaps; the export is unchanged.
- REQ-199 (implemented): **View > Show safe areas** (off by default, a program setting) draws the action-safe area (90% of width and height) and the title-safe area (80%) on the preview only. Acceptance: the two rectangles are drawn at those sizes; the export has no rectangle; with the option off none is shown.

## Key decisions
- Autosave is in the private cache, so it never changes the project format or writes next to the project.
- A fade marker moves between clip boundaries; its length is not changed here (length is in [Effects and frame options](business-effects-frame.md), REQ-274).
- Keyboard marker keys are **[** and **]**, with no modifier, so they do not clash with the existing shortcuts.
- Group actions on subtitle lines are Delete and a 0.5 s shift; other group actions are not planned.

## Out of scope
- Dragging clip edges of music clips; multi-track video.
