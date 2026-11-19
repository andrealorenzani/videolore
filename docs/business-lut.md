# Business — Apply LUT

Back to the [index](business.md).

Added in 1.6.0. **Apply LUT** is a colour effect: the user chooses a LUT file (a table that changes the colours of the picture) from the disk and applies it to one video clip or to the whole video. It is the only colour effect; colour grading beyond a LUT stays out of scope.

## Features
- **Choose from disk**: the popup has a **Choose file…** button and shows the file name; the full path is in a tooltip.
- **Where it applies**: drag Apply LUT from the Effects list onto a video clip, or right-click the clip > Effects ▸ Apply LUT, to apply it to that clip; double-click it in the list to apply it to the whole video (see [Window, menus and effects](business-ui.md)).
- **Order**: the whole video's LUT is applied first, then the clip's.
- **Preview and export**: the same colours in the preview and in the export; the export is always a re-encode.

## Requirements

### Apply LUT
- REQ-135 (implemented): **Apply LUT** is an effect of a video clip or of the whole video. Its popup has a **Choose file…** button that opens a file dialog for a LUT file, the file name (its full path in a tooltip), and a **Remove effect** button. A new Apply LUT is set when a file is chosen; a change of file and a removal are each one undo step. A clip can have one Apply LUT and the whole video one; a second one on the same clip opens the existing popup (REQ-130). When both exist, the whole video's LUT is applied first and then the clip's. The LUT changes the picture of the video only: the subtitle lines and text items (REQ-131) keep their colours (assumption). The solid colour fill and the black or coloured bars of the frame (REQ-126) keep their colours; the blurred background is made from the coloured picture, so it takes the LUT colours (accepted deviation, 1.6.0). A clip with Apply LUT has one marker at its start (as Clean audio, REQ-077); the whole video's Apply LUT has no marker. The colours show in the preview at once. Acceptance: a LUT on clip 2 of 3 changes only that clip in the preview and in the export; one undo restores the previous state; Remove effect gives back the source colours; dropping Apply LUT on the music track does nothing; the marker opens the popup.

### LUT files and export
- REQ-136 (implemented): The LUT file is read where it is, and the project stores its path, not a copy (like media files, REQ-001 and REQ-003). Supported: `.cube` files, 3D LUTs and 1D LUTs (assumption). A file that is not a valid LUT of these kinds is refused when chosen, with a clear message; nothing changes and nothing crashes (REQ-057). When a project opens and a LUT file is missing, the project still opens and the file name is reported in the existing "Some files could not be found or were repaired" notice (REQ-003, REQ-057); a damaged LUT entry is removed with a line in that notice; exporting then stops before anything is written, with a message that names the file, until the file is back or the effect is removed (assumption). An Apply LUT always makes the export a re-encode (REQ-023), even at its first setting. The hardware export and its output check apply as for the other picture effects (REQ-062, REQ-063). The 360 video export (REQ-103) applies the LUT too, because colour is not geometry (assumption). Acceptance: a valid 3D `.cube` changes the picture as the file says, and the export matches the preview within 2 colour levels per channel on a test frame (assumption); a missing file gives the message and writes no output file; a broken `.cube` is refused with its message; a project whose only change is Apply LUT is re-encoded; the 360 video export shows the LUT colours.

### Project file
- REQ-137 (implemented): A project stores each Apply LUT (its file path) for the whole video and for each clip. This is project format 1.6.0 (like REQ-132). A project saved with 1.6.0 cannot be opened by 1.5.x or earlier, which refuse it with their newer-version message and leave the file untouched (REQ-029). Projects saved by 1.5.x open in 1.6.0 with the same result. A damaged path entry is refused or cleared without a crash (REQ-057). Acceptance: a 1.5.0 example project opens and exports as before; a 1.6.0 file with a LUT is refused by 1.5.0 with its message; a LUT path that is not text is refused without a crash.

## Key decisions
- (1.6.0) A LUT is a colour effect of the same kind as the others (one per clip, one for the whole video, drag or double-click), so the user learns one way of working (consistent with REQ-130).
- (1.6.0) The project keeps the path of the LUT file, not a copy: the project stays small and the file stays where the user keeps it. The price is a missing-file message when the file moves.
- (1.6.0) No strength (mix) setting for the LUT: the file alone decides the look. A strength setting can come later.

## Known limitations
- Colour is applied as the file describes it; there is no colour management, so a LUT made for another workflow can look different.
- Only `.cube` files are read in 1.6.0.
- The LUT is applied to the picture after Scale and Move, so it moves and scales with the picture. A solid colour fill is not changed by it; a blurred background shows the LUT colours.

## Out of scope
- Colour grading controls (wheels, curves, levels), LUT strength, creating or editing LUTs, LUT libraries, LUT files copied into projects.
