# Business — Apply LUT

Back to the [index](business.md).

**Apply LUT** is a colour effect: the user chooses a LUT file (a table that changes the colours of the picture) and applies it to one video clip or to the whole video. It is the only colour effect; colour grading beyond a LUT is out of scope.

## Features
- **Choose from disk**: the popup has a **Choose file…** button and shows the file name; the full path is in a tooltip.
- **Where it applies**: drag Apply LUT from the Effects list onto a video clip, or right-click the clip > Effects ▸ Apply LUT; double-click gives every clip its own copy of the LUT (REQ-147; see [Window, menus and effects](business-ui.md)).
- **One per clip**: a clip has one LUT; a clip added later has none.
- **Preview and export**: the same colours; the export is always a re-encode.
- **Strength, favourites, background loading**: REQ-289, REQ-294 and REQ-300 below.

## Requirements

### Apply LUT
- REQ-135 (implemented): **Apply LUT** is an effect of a video clip or of the whole video. Its popup has a **Choose file…** button that opens a file dialog for a LUT file, the file name (full path in a tooltip), and a **Remove effect** button. A new Apply LUT is set when a file is chosen; a change of file and a removal are each one undo step. A clip can have one Apply LUT; a second one on the same clip opens the existing popup (REQ-130). A whole-video Apply LUT is not a separate layer: a double-click gives each clip its own LUT with the chosen file (REQ-147), replacing a LUT the clip had (MINOR). A clip that had its own LUT in a 1.6.0 project keeps it, and the whole-video LUT is not applied to it (REQ-149). The LUT changes the picture only: subtitle lines and text items (REQ-131) keep their colours (assumption). The solid colour fill and the bars of the frame (REQ-126) keep their colours; the blurred background is made from the coloured picture, so it takes the LUT colours. A clip with Apply LUT has one marker at its start (REQ-077). Acceptance: a LUT on clip 2 of 3 changes only that clip in the preview and in the export; a double-click on Apply LUT with a chosen file gives all three clips that LUT, and one undo restores them all; Remove effect gives back the source colours; dropping Apply LUT on the music track does nothing; the marker opens the popup. Conflict case: a 1.6.0 clip with its own LUT A and a whole-video LUT B shows A after opening, and the other clips show B.

### LUT files and export
- REQ-136 (implemented): The LUT file is read where it is, and the project stores its path (like media files, REQ-001 and REQ-003), except for copies of LUTs up to 20 MB (REQ-293). Supported: `.cube` files, 3D LUTs and 1D LUTs (assumption); see REQ-290 and REQ-291 in [New effects](business-effects-frame.md). A file that is not a valid LUT is refused when chosen, with a clear message; nothing changes and nothing crashes (REQ-057). When a project opens and a LUT file is missing, the project still opens and the file name is reported in the "Some files could not be found or were repaired" notice (REQ-003, REQ-057); a damaged LUT entry is removed with a line in that notice; exporting then stops before anything is written, with a message that names the file, until the file is back or the effect is removed (REQ-292 choices). An Apply LUT always makes the export a re-encode (REQ-023). The hardware export and its output check apply (REQ-062, REQ-063). The 360 video export (REQ-103) applies the LUT too, because colour is not geometry (assumption). Acceptance: a valid 3D `.cube` changes the picture as the file says, and the export matches the preview within 2 colour levels per channel on a test frame (assumption); a missing file gives the message and writes no output file; a broken `.cube` is refused with its message; a project whose only change is Apply LUT is re-encoded; the 360 video export shows the LUT colours.

### Project file
- REQ-137 (implemented): A project stores each Apply LUT (its file path, or its copy under REQ-293) for the whole video and for each clip. This is project format 1.6.0 (like REQ-132). A project saved with 1.6.0 cannot be opened by 1.5.x or earlier, which refuse it with their newer-version message and leave the file untouched (REQ-029). Projects saved by 1.5.x open in 1.6.0 with the same result. A damaged path entry is refused or cleared without a crash (REQ-057). Acceptance: a 1.5.0 example project opens and exports as before; a 1.6.0 file with a LUT is refused by 1.5.0 with its message; a LUT path that is not text is refused without a crash.

## Importing the user's LUT files (2.1.0)
Three 144-point `.cube` files of the maintainer's collection (`Black_White_LUT.cube`, `High_Contrast_LUT.cube`, `Sepia_tone_LUT.cube`, each about 80 MB as text) were refused by 2.0.4 because of its own limits (65 points an edge, 16 MB a file), not by ffmpeg. 2.1.0 reads them.
- REQ-300 (implemented): Apply LUT imports every valid LUT of the formats `.cube`, `.3dl` and `.csp` that its file describes, as a 3D LUT or a 1D LUT, up to 256 points on an edge (1D: the same limit, assumption), and up to 512 MiB a `.cube` file, so that the three files above can be chosen with **Choose file…**. `.3dl` and `.csp` files are read whole, up to 64 MiB. A file above 1 MiB is read in the background with a percentage on the status line: the program stays usable, the LUT is applied as one undo step when read, and a newer choice replaces the read in progress. The colours are kept to 16 bits. A LUT that cannot be imported is refused with one message that names the file and the reason (for example not a LUT, a damaged line with its line number, an unsupported input range); nothing changes and nothing crashes (REQ-057). Acceptance: each of the three files, chosen on a clip, changes the preview colours as described (checked by hand on a test frame and by a test with a small reference LUT of the same kind); the export matches the preview within 2 colour levels per channel; a copy of one of them with a broken line is refused with the file name and the line number.
- REQ-301 (implemented): Large LUTs: a LUT whose copy in the project would be above 20 MB (measured as the text the project would write, not the file on disk) is not copied into the project file; the project keeps only its path, and the missing-file choices of REQ-292 apply (**Find the LUT file…**, **Export without the LUT**, **Cancel**). A LUT of 20 MB or less is copied as REQ-293 says. The 64 MB total per project counts copies only. Acceptance: an 80 MB LUT is chosen and used in the preview and in the export; the project file does not grow by its size; after the file is moved or deleted, the export shows the REQ-292 choices and Find relinks it so the export uses the LUT; a LUT of 20 MB or less is still copied.

## Key decisions
- A LUT is a colour effect of the same kind as the others (one per clip, drag or double-click), consistent with REQ-130 (1.6.0). A whole-video LUT is a copy on each clip (REQ-147, 1.6.1).
- The project keeps the path of the LUT file, not a copy, below the limit of REQ-293 and above the limit of REQ-301 (option C, Andrea, 2.1.0): the project file stays small for large LUTs.
- No strength (mix) setting in 1.6.0; the strength setting came in 2.0.4 (REQ-289).
- The formats are `.cube`, `.3dl` and `.csp`, each 3D or 1D (REQ-300 wording, 2.2.0).

## Known limitations
- Colour is applied as the file describes it; there is no colour management, so a LUT made for another workflow can look different (see the [roadmap](roadmap.md)).
- The LUT is applied after Scale and Move, so it moves and scales with the picture. A solid colour fill is not changed by it; a blurred background shows the LUT colours.
- A clip has one LUT; a whole-video LUT is a copy on each clip, so a clip added later has none (REQ-147).

## Out of scope
- Colour grading controls (wheels, curves, levels), creating or editing LUTs.
- Colour management for LUTs (not planned yet); input ranges other than 0 to 1 are handled by REQ-291 (see [New effects](business-effects-frame.md)).
