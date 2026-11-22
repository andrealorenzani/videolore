# Architecture: order of all effects and project format 13 (2.2.0, planned)

Part of [2.2.0 overview](architecture-2.2.0-overview.md). Requirements REQ-320, REQ-321, REQ-322 ([applied effects](business-applied-effects.md)), REQ-323 and REQ-318 for the text maximum ([text](business-text.md)). Status: built 2026-11-22, not released. Deviation from REQ-321 "new rows on top": a new Crop, Change aspect, Scale, Move, Fade, Volume or Clean audio keeps its default place (a Scale on top of Change aspect would scale the background too; a Crop on top of a text would cut it). A new text, picture or LUT still goes on top of the other movable rows. Three builds: the default (2.1.0 legacy path), the 2.1.0 overlay form, and the step builder for other orders; text on the layer is drawn as a black and white matte.

## Project format 13
- 2.2.0 writes format 13. It adds the order of every row of a clip and the optional maximum size of a text item. Format 12 (built in 2.1.0, not released) still opens, with the 2.1.0 meaning of the stored order and no maximum.
- Why a new number and not optional fields in format 12: a 2.1.0 program would open such a file, lose the rows it does not know and write the file back without them. Older programs refuse format 13 with their newer-version message and leave the file untouched (REQ-029).
- Migration 12 to 13 happens in memory; nothing is written until the project is saved.

## Groups and default order
- **Picture group** (applied first to last in the order of the list, top row drawn last): Crop, Change aspect (the frame fit and background), Apply LUT, text items, picture items, subtitle lines, Fade IN/OUT, Scale, Move.
- **Sound group**: Clean audio, Volume.
- **Speed**: fixed and applied first; it is not stored and cannot be moved.
- The default order is the order the 2.1.0 build uses. A clip with the default order is built exactly as in 2.1.0 (the same ffmpeg arguments, checked by the golden tests).
- Only a clip whose order differs from the default stores its full order. A project saved in 2.1.0 with a changed order of its movable rows opens with those rows on top of the default order of the other rows, so its look does not change.

## Moves and new rows
- A row moves only inside its group. A move is one undo step: "Undo Move Volume up", a drag is "Move effect".
- A new row (a LUT, a text, a picture item, a Crop, a Volume or another effect) takes the top place of its group.
- A whole-video effect is a copy on each clip; each copy takes the top place of its group on its clip. Later whole-video changes do not reorder copies.
- 360 clips: the texts and lines fixed to the sphere keep fixed rows (2.1.0 rule); the 360 whole-sphere export ignores the order.

## How a changed order is drawn
- The picture is a stream with a transparent surround (RGBA), so each step acts on what the picture is at its place in the order.
- Crop cuts the content by percent of its current size, so its place in the order decides what is cropped: before Scale it crops the source, after Scale the scaled picture.
- Scale resizes about the picture's centre; Move offsets the picture inside the frame. Move before Scale and Scale before Move place the picture differently.
- Change aspect fits the picture into the project frame and composes the background under it.
- Apply LUT grades the stream it sits on. Above Change aspect it grades the whole frame, fill included; below it, the picture only.
- Fade IN/OUT fades the stream it sits on: above Change aspect it fades the background too.
- Texts, picture items and subtitle lines are drawn as in 2.1.0.
- The sound group is a chain: Clean audio and Volume in their order. The dry part of the Clean audio mix takes its input at the same point, so the mix stays correct after a swap. Audio fades keep their place after the sound group (the picture Fade IN/OUT row is the picture's fade; this follows from REQ-320).
- Preview and export use the same builder.

## Tests
- Default order: the golden arguments of formats 4, 5 and 12 unchanged.
- Pairs: each swap of the pairs above gives a different picture where the requirement says so, and preview equals export.
- Sound: a swap of Volume and Clean audio gives a different sound.
- Model: groups, moves inside a group, new rows on top, copies on top, expansion of 2.1.0 orders, unknown names repaired with the 2.1.0 line.
