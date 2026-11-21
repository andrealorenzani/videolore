# Architecture: project files

Part of [Architecture](architecture.md). How projects are saved, opened, checked and kept compatible between versions.

## Format versions
Projects are JSON files (`.vlproj`) with a format version number.

| Version | Written by | Adds |
|---|---|---|
| 1 | 0.1.x | Media list and the video track (clips, cuts, fades between clips). |
| 2 | 0.2.0 – 1.1.x | Music track, volume and mute of video audio and music, subtitles (language, style, lines with word timings). |
| 3 | 1.2.0 | Fade from black at the start and to black at the end of the video, volume and mute of each video clip, the on-screen position of each subtitle line, and the text subtitle tracks found in each media file. |
| 4 | 1.2.6 | 360 video: the "Treat as 360° video" choice and the recognised flag of each media file, and the POVs (camera path points) of each source; a size for each subtitle line. |
| 5 | 1.3.0 | Text items (their text, times, effects, colour, font, box and size) and the 360 anchors of text items and subtitle lines. |
| 6 | 1.4.0 | Clean audio on each video clip; the text width replaces the text size (the size follows from the box); 360 anchors of text store width and height. The "Treat as 360° video" choice is no longer written (the flag is read only to be dropped). |
| 7 | 1.5.0 | Change aspect of the whole video; Scale and Move with their keyframes for the whole video and for each clip (clip keyframes in source time). Format 6 files open with no picture effects; 1.4.x refuses format 7. See [frame](architecture-frame.md#project-format). |
| 8 | 1.6.0 | Apply LUT: the path of one `.cube` file for the whole video and one per clip (no copy of the file). Format 7 files open with no LUT; 1.5.x refuses format 8. Damaged paths are cleared with a note. See [LUT](architecture-lut.md#storage-and-project-format-8). |
| 9 | 1.6.1 | Whole-video effects are stored on the clips: each clip has its own Scale and Move keyframes and LUT, and its own start and end fade. Change aspect stays one setting of the video. Format 8 files are converted when opened; 1.6.0 refuses format 9. See [effects](architecture-effects.md#whole-video-per-clip-161). |
| 10 | 2.0.3 | Text readability (None, Outline or Box) and box height of each text item. See the format 10 section below. |
| 11 | 2.0.4 | Media file size and date of analysis (stage 1); 360 marks, roll and easing (stage 10); fade lengths and sides, crop, Clean audio strength, speech enhancement, music clip effects, LUT copies and strength, blur level (stage 11). See the format 11 section below. |

- 0.3.0, 0.4.0 and 1.1.0 did not change the format (1.1.0 timeline markers are computed from the existing data; the layout and recent projects are program settings, never stored in a project).
- **Opening older files:** version 1 and 2 files are converted in memory when opened and give exactly the same picture, sound and export as before: fades only where they were, every clip at 100% volume, every subtitle line in its style's usual place. Automated tests compare the export commands of a 0.1 and a 1.1 project before and after the change.
- **Media imported before 1.2.0** have not been checked for subtitle tracks. When such a project opens, they are checked again in the background; this does not mark the project as changed. If subtitles are requested before the check has finished, it runs first.
- **Saving:** 1.2.0 always writes version 3, even when no new feature is used. The alternative (writing version 2 when possible) was rejected: it needs two writers and two sets of tests for little gain. Consequence: a project saved by 1.2.0 cannot be opened by 1.1 or earlier.
- **1.2.6:** always writes version 4 for the same reason as above; 1.2.5 refuses it. Versions 1–3 open with identical results (no 360 clips, no sizes, same export commands; golden tests). Media without the 360 flag are checked again in the background like the subtitle tracks.
- **Files from a newer version** are refused with a clear message and are never modified. 1.1 refuses version 3 files in this way.

## Format 10 (planned 2.0.3)
- Text items gain two stored values: their readability (None, Outline or Box) and their box height (5% to 100% of the picture height).
- Migration from format 9: texts read as readability None (their 2.0.2 outline is dropped) and box height 20% (the 2.0.2 box); their size is fitted again on opening, so it may change (see [text](architecture-text.md#203-planned-readable-text)). The status bar shows once the number of texts whose size changed and that the old outline was removed; the message appears only for a project with at least one text item and not again once the project is saved in format 10.
- Format 10 is written from 2.0.3 on. Version 2.0.2 and older refuse such a file ("newer format") and never change it.
- Projects of formats 1 to 9 otherwise open as before.

## Format 11 (2.0.4)
One format for the whole 2.0.4 development (decision D58). Format 10 files open unchanged; 2.0.3 and older refuse format 11 and never write it.
- **Stage 1:** each media entry records the file size and modification time of its analysis; a changed file is analysed again on opening (the status line names it). Files of format 10 get the stats recorded without a new analysis.
- **Stage 10:** a clip can be marked as a 360 video (explicit mark); a point of view stores its roll, and a segment can be eased. The camera paths are still kept per source; per-clip paths are not built yet.
- **Stage 11:** fade lengths and sides per clip; crop of a video clip; easing per keyframe; Clean audio strength and speech enhancement; music clip volume, mute, fades and Clean audio; LUT strength; the blur level; a copy of each LUT file used, so a moved file does not break the project. Not built yet: clip speed, volume keyframes, the clip's own Change aspect, picture items.
- The read limit rises from 16 MiB to 96 MiB in this format, because LUT copies (at most 20 MB each, 64 MiB per project) are stored in the file. Every other limit stays.
- Values added after a stage are optional with defaults, so a format 11 file written by an earlier stage opens with the same result.

## Loading and validation
- Only regular files are read (a pipe or device named like a project cannot block the program), at most 16 MiB, enforced while reading.
- After loading, everything is validated: clip ranges, music clips use audio-only files and do not overlap, subtitle lines refer to existing media, have start before end and at most 1000 characters, at most 200,000 words. Deeply nested data is rejected.
- **New in version 3:** a clip volume above 100% or a subtitle position outside the picture is refused as a damaged project; references to unknown clips or lines are refused. Subtitle track descriptions (language, title) are only cached information that `ffprobe` can produce again, so bad values there are cleaned or shortened instead of refusing the project. A start or end fade on an empty timeline is ignored.
- See [security](architecture-security.md#project-and-settings-files) for the hostile-file tests.

- **New in version 6:** written always; versions 1 to 6 open, newer files are refused. A text width outside 10–100% or an anchor width or height outside its range refuses the project as damaged. The text size of version 5 and the 360 switch of version 4 are read only to be converted in memory (no undo step, the project is not marked as changed when it opens); the result is the same picture and export, except that large multi-line text items may shrink to fit their new box (confirmed choice).
- **New in version 5:** text items and anchors are validated as damaged data when out of range: a time to live under 0.1 s or over 24 hours (a value longer than the video is accepted; it is limited to the video length when the item is added, edited or moved), an effect time outside 0.1–5 s, colour not `#RRGGBB`, a font name outside the allowed characters or longer than 100, more than 2,000 blink events per item or 20,000 per project, anchors outside the sphere. Version 5 files are refused by 1.2.6 with its "newer version" message. Projects from 1.2.6 and earlier open unchanged.
- **New in version 4:** more than 200 POVs for a source, POVs of an unknown media file, a negative or duplicate POV time or a line size outside 25–400% refuse the project as damaged; angles and zoom outside their limits are clamped (they are only view settings). POVs belong to the undo history and to the unsaved-changes comparison.

- **Version 8 to 9 (1.6.1):** converted in memory, with no undo step and no change mark. A whole-video Scale or Move becomes a copy of its keyframes in each clip (key times in clip time, then stored in source time like the clip's own keys; the value at each clip edge is kept); a whole-video LUT becomes the same LUT on each clip; the whole-video start and end fades become the first clip's start fade and the last clip's end fade; the project-wide video level is multiplied into each clip's level (rounded to 1%) and set to 100%. Where a clip already had its own Scale or Move, the two are combined into one path (scales multiplied, moves added, at the key times of both; exact at those times). Where it already had its own Apply LUT, the clip keeps it and the whole-video LUT is not applied to that clip. Above 200 key times, the clip's own key times are kept first. When any clip changed this way, the status line at opening says how many. Keyframe counts are capped at 200 per clip and the usual range checks apply.

## Unsaved changes
- Unsaved changes are detected by comparing the edit with the last saved one, so undoing back to the saved state clears the marker. Background updates that are not edits (such as the subtitle-track check above) update both and never set the marker.
- The window title shows the project name with a `•` when there are unsaved changes, or "Untitled" before the first save.
- New, Open, closing the window and Quit (Cmd+Q on macOS, Ctrl+Q elsewhere) ask Save / Discard / Cancel when there are unsaved changes. A failed or cancelled save stops the action. On macOS the default application menu is turned off so that Cmd+Q reaches this prompt; as a consequence the standard Hide and Minimize menu items are not available.
- Saving is atomic (temporary file, then rename).

## What is not in a project
Program settings (layout, show boxes on the picture, recent projects, quality levels, the `ffmpeg` path), the selection of subtitle lines and text items, the list of installed fonts, the export choices (format, mode, folder, 360 choice; kept for the session only, see [export dialog](architecture-export-dialog.md)), the live 360 camera, and positions and sizes while a subtitle is being dragged. The undo history is not saved.
