# Architecture: overlap media (2.3.0, planned)

Part of [2.3.0 overview](architecture-2.3.0-overview.md). Requirements: [overlap media](business-overlap.md) (REQ-325, REQ-327 to REQ-331), REQ-316 (16 megapixel limit, applied to overlap files), REQ-023 (export re-encodes).

## Data
- An **overlap** is stored in its clip: a file path (kept as text, the file is not copied), a kind (**still** for PNG, **motion** for GIF and video), a start offset from the clip's start and a length (in timeline time, so a speed change rescales them), a place and a box as permille of the frame (the same convention as text items), and a flag for a changed shape (Ctrl or Command).
- A clip may hold several overlaps; they may overlap in time and are drawn in their order.
- Format 14 (ADR-230). A file of format 13 gets its picture items converted once at load: a picture item whose start lies in a clip becomes an overlap of that clip with the same times and place; other picture items stay picture items.

## Editing rules
| Edit | Effect on an overlap | Warning |
|---|---|---|
| Ripple (a clip moves) | moves with its clip | no |
| Trim or split | cut to the part of the clip that is kept | no, when a part is kept |
| Trim, split or delete leaving no part in the kept time | removed | one popup for all of them: count, OK removes them with the edit, Cancel cancels the edit |
| Delete the clip | removed | same popup |
| Change speed | times rescaled with the speed (so the overlap keeps its part of the clip) | none (decided by Andrea: rescaled with the speed) |

One undo restores the edit and the removed overlaps. Lane drags are clamped to the clip; the minimum length is 0.1 s (assumption).

## Interface
- **Overlap media** is an Effects list row and an Edit menu entry. It adds the chosen file at the playhead on the clip under the playhead; with no clip there, nothing is added and the status line says so. Files over 16 megapixels are refused with the REQ-316 message.
- **Add media** (REQ-324) is the button under the Media list; it opens the same dialog as File > Import media….
- The timeline lane shows each overlap; its two ends are dragged (one shared lane for the timeline; overlapping overlaps are stacked in sub-rows, at most three visible, the rest reached by scrolling).
- In the preview a drag moves the box (its centre stays on the frame); a drag of an edge or corner keeps the media's ratio; with Ctrl (Command) held on an edge the shape may change: a MOV is cropped to the box, a GIF or PNG is stretched. One drag is one undo step.
- The Applied effects widget lists overlaps in the picture group; they are on top by default and can be moved (REQ-320, REQ-321).

## Rendering
Three paths; the same picture in preview and export, except large overlay files (see Performance and limits):
1. **Default order (legacy argument list):** overlaps are drawn after the concatenation, as picture items are today. A project without overlaps has the same argument list as before.
2. **Changed order (step builder):** the overlap row is one step of the clip's own chain, in the order the user chose.
3. **Preview:** the same plan built for the preview; a still is updated only for the latest frame, as today.

The 360 whole-sphere export draws no overlap, as it draws no picture item. Any overlap makes the export a re-encode.

Per overlap:
- **Still (PNG):** looped still input, scaled to the box, shown for the overlap's length.
- **Motion (GIF, MOV):** played once from the overlap's start, without sound; when the animation ends the overlay passes the picture through, so it is transparent for the rest of the overlap (REQ-328).
- **Box:** normally the media's own ratio, so the picture is never stretched. A changed MOV is cropped to the box (cover, cut evenly); a changed GIF or PNG is stretched to the box.

## Alpha and decoding
- PNG and GIF carry transparency that the bundled ffmpeg keeps when the picture is converted to RGBA. MOV keeps alpha in codecs such as qtrle and ProRes 4444 (assumption, checked with committed fixtures). A codec without alpha is drawn opaque.
- Overlay files are always decoded in software. In the preview, hardware decoding skips overlay files, because a hardware decoder may drop alpha (assumption, not measured on every graphics card). The export never uses hardware decoding.
- Sizes: PNG from its header, GIF from its screen size (13 bytes), MOV from ffprobe. The 16 megapixel limit is checked when the file is added and again when it is used; a file that became too large counts as missing (export stops with its message; the preview leaves it out).

## Performance and limits
- The overlay is scaled to its box as the first filter; the full-size decode is the memory cost (about 64 MB per frame for 16 megapixels at RGBA; measured by the developer).
- A large MOV overlay can slow the preview; overlay files above 1920 x 1080 (PNG, GIF or video) are therefore drawn at half their box size in the preview only; the export draws them at full size. The preview and the export differ in detail for those files; files of 1920 x 1080 or less are drawn the same in both.
