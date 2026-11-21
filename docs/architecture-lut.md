# Architecture: Apply LUT

Part of [Architecture](architecture.md). Requirements REQ-135 to REQ-137 ([business](business-lut.md)). Since 1.6.0.

## Overview
Apply LUT is a colour effect, one per clip and one for the whole video. The user chooses a `.cube` file; the project stores its path (not a copy). The file is read and checked when chosen, and again at each export. `ffmpeg` applies it with the `lut3d` filter in the same graph as preview and export.

## Reading the file (`src/lut.rs`, pure)
- Format: `.cube` only, UTF-8 text. Keywords `LUT_3D_SIZE` or `LUT_1D_SIZE`, `TITLE`, `DOMAIN_MIN`, `DOMAIN_MAX`; `#` starts a comment.
- Read as a regular file. Up to 2.0.4: at most 16 MiB and 3D size 2 to 65. From 2.1.0 (planned): streamed up to 512 MiB, 3D size 2 to 256, see [large LUTs](#large-luts-210-planned).
- 3D: exactly size^3 entries. 1D: size 2 to 4096, exactly size entries.
- Every value must be a finite number; values outside 0 to 1 are clamped.
- `DOMAIN_MIN`/`DOMAIN_MAX` other than 0 and 1 are refused with a message (support is later work).
- A file with both kinds, unknown numbers or bad counts is refused. The message names the problem; nothing changes in the project.
- A 1D table is turned into a 3D cube of size 33 by sampling each channel (one code path, one filter).
- Tests: valid 3D and 1D, each refusal, sizes at the limits, NaN and infinity, property tests with random bytes (no panic).

## Storage and project format 8
- `Timeline` holds `whole_lut` (one optional path) and `clip_luts` (path per clip, sorted by clip id, empty entries removed), both `#[serde(default)]`. The clip type stays `Copy`.
- Split copies the LUT to both halves; merge needs the same LUT on both (message "The clips have different colour effects"); removing a clip removes its entry.
- Every choose, change or removal is one undo step.
- Project format 8 is written by 1.6.0; 1.5.x refuses it with its newer-version message and leaves the file untouched. Older files open unchanged. A damaged path (empty, too long, or with a control character) is cleared with a note when the project opens.
- Paths must be valid UTF-8; a file name that is not is refused with a message when chosen.

## Missing or changed files
- A file missing when the project opens: the project opens, the status line names the file, and the LUT is not shown in the preview.
- Export with a missing file stops before any output file is written, with a message naming the file.
- A changed file is checked again at the next choose and at the next export. Its content hash (SHA-256 of the validated table) decides when the preview redraws, so a changed file never mixes with an old still.

## Where it is applied (per segment)
1. Picture scaled to the segment's picture size (and by the Scale keyframes, if any).
2. Whole-video LUT (if any): `format=gbrp`, `lut3d`, `format=yuv420p`.
3. Clip LUT (if any), same filter.
4. Picture placed on the frame background (Change aspect), so the background keeps its colours.
5. Fades to black.
Then the segments are joined, and subtitles and text are drawn as before. In 360 video export the LUT is applied after the sphere is projected (colour, not geometry). The whole-video LUT always comes before the clip LUT.

The cube is written by the program into the private work folder with a fixed name (`lut-whole.cube`, `lut-clip-<n>.cube`). No user text reaches the filter string; the file name is fixed. The filter uses `interp=tetrahedral` (the bundled `ffmpeg` build is checked for this option).

## Export
- Any LUT makes the export a re-encode, even at default settings.
- Hardware export uses the same picture graph as the other picture effects; its output check compares with the reference render of the same plan.

## Strength, other formats and copies (2.0.4)
- **Strength (REQ-289, ADR-191):** the Apply LUT popup has a strength from 0 % to 100 % (default 100 %). The table is blended with the identity by the chosen percent (`lut.rs` `with_strength`: every entry mixed with the identity; 0 = no change, 100 = the table), so one table serves every strength and no second filter runs.
- **Formats (REQ-290, REQ-291):** `.3dl` and `.csp` files are read as well as `.cube`, with the same refusal rules and the same caps and clamping. A `.cube` input range (DOMAIN_MIN, DOMAIN_MAX) that is not 0 to 1 is applied when it is read. Fixtures check the results within two colour levels of their references (gate).
- **Missing file at export (REQ-292):** a three-button modal replaces the refusal of 1.6.0: cancel, or export without the LUT for that export only. The project keeps the LUT.
- **Copies in the project (REQ-293, ADR-191):** the project stores a copy of each LUT file it uses, as the normalised cube text (a JSON string in `lut_copies`, keyed by SHA-256). A copy is at most 20 MB, and at most 64 MiB per project; a copy over the limit keeps only the path. The project read cap is 96 MiB (see [projects](architecture-projects.md#format-11-204)).
- **Favourites (REQ-294):** up to 20 LUT files in the popup, kept in the program settings (never in a project).

## Large LUTs (2.1.0, planned)
- **Why 2.0.4 refused a 144-point `.cube` (about 80 MB):** the size limit was 65 points and the read limit 16 MiB. ffmpeg accepts 256 points, so the limit was in the program.
- **Reading:** one streamed pass over the file (a 1 MiB buffer), with no whole-file or whole-text copy in memory. Lines are at most 256 bytes; the number of entries must be exactly size^3 and no numeric line may follow. Edge 2 to 256; file at most 512 MiB (a 256-point cube is about 450 MB).
- **Memory:** a table takes 6 bytes per point, 16-bit per channel (100.7 MB at 256 points, 17.9 MB at 144). The cache keeps at most 192 MB of tables, least recently used first.
- **Loading:** on a background job; the window does not wait. The status line shows the percentage read ("Reading the LUT file... N %"). Choosing another file or closing the project cancels the job and removes its partial copy. While a LUT loads, the preview shows the picture without it; the picture is redrawn when the table is ready. A file that changes while it is read is refused.
- **Work copy:** the normalised cube is written once per distinct content as `lut-<hash>.cube` in the private Work folder (streamed, atomic, with progress; about 450 MB at 256 points) and shared by all clips, the preview and the export. The filter names the fixed file; the user's path is never in the filter.
- **Project:** a LUT above 20 MB is not copied into the project; the project keeps its path (REQ-301). The copy rule of REQ-293 is unchanged below the limit.
- **Missing file:** the REQ-292 choices apply (Find the LUT file, Export without the LUT, Cancel).
- **Order:** from 2.1.0 the LUT is a movable row of the Applied effects widget; see [applied effects](architecture-applied-effects.md).
- **Targets (estimates, measured in the build):** parse of 144 points at most 2 s and of 256 points at most 6 s; Work copy of 256 points at most 3 s; the window frame at most 200 ms while loading. The start-up of each preview process with a 256-point LUT is measured first.

## Non-functional targets (1.6.0)
- A 1080p preview with a 33-point LUT keeps the playback rate of the same clip without a LUT within 20% (assumption, measured by hand).
- Export with a LUT at most 1.5 times the time of the same re-encode without one (assumption, to measure).
- The largest cube (65 points) is about 6 MB of text in the work folder, deleted with it.

## Tests
- Real `ffmpeg`: a known 3D LUT (channel swap) gives exact colours on a test frame; clip 2 of 3 changes only that clip; the 360 export shows the LUT colours; the export matches the preview graph.
- App: one undo per action; LUT on the music track is ignored; marker at the clip start only; a second LUT on the same clip opens the existing popup.
- Project: format 8 round trip; format 7 unchanged; damaged path cleared with a note.

## Risks
- Colour matrix round trip (YUV to RGB and back) moves values by one or two levels; tests compare with a tolerance.
- `lut3d` options on the bundled 9.x build are checked by hand before release.
- The LUT is applied as the file says; there is no colour management, so LUTs made for other workflows may look different.
- Cost on weak computers is not measured.
