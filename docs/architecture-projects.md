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

- 0.3.0, 0.4.0 and 1.1.0 did not change the format (1.1.0 timeline markers are computed from the existing data; the layout and recent projects are program settings, never stored in a project).
- **Opening older files:** version 1 and 2 files are converted in memory when opened and give exactly the same picture, sound and export as before: fades only where they were, every clip at 100% volume, every subtitle line in its style's usual place. Automated tests compare the export commands of a 0.1 and a 1.1 project before and after the change.
- **Media imported before 1.2.0** have not been checked for subtitle tracks. When such a project opens, they are checked again in the background; this does not mark the project as changed. If subtitles are requested before the check has finished, it runs first.
- **Saving:** 1.2.0 always writes version 3, even when no new feature is used. The alternative (writing version 2 when possible) was rejected: it needs two writers and two sets of tests for little gain. Consequence: a project saved by 1.2.0 cannot be opened by 1.1 or earlier.
- **1.2.6:** always writes version 4 for the same reason as above; 1.2.5 refuses it. Versions 1–3 open with identical results (no 360 clips, no sizes, same export commands; golden tests). Media without the 360 flag are checked again in the background like the subtitle tracks.
- **Files from a newer version** are refused with a clear message and are never modified. 1.1 refuses version 3 files in this way.

## Loading and validation
- Only regular files are read (a pipe or device named like a project cannot block the program), at most 16 MiB, enforced while reading.
- After loading, everything is validated: clip ranges, music clips use audio-only files and do not overlap, subtitle lines refer to existing media, have start before end and at most 1000 characters, at most 200,000 words. Deeply nested data is rejected.
- **New in version 3:** a clip volume above 100% or a subtitle position outside the picture is refused as a damaged project; references to unknown clips or lines are refused. Subtitle track descriptions (language, title) are only cached information that `ffprobe` can produce again, so bad values there are cleaned or shortened instead of refusing the project. A start or end fade on an empty timeline is ignored.
- See [security](architecture-security.md#project-and-settings-files) for the hostile-file tests.

- **New in version 5:** text items and anchors are validated as damaged data when out of range: a time to live under 0.1 s or over 24 hours (a value longer than the video is accepted; it is limited to the video length when the item is added, edited or moved), an effect time outside 0.1–5 s, colour not `#RRGGBB`, a font name outside the allowed characters or longer than 100, more than 2,000 blink events per item or 20,000 per project, anchors outside the sphere. Version 5 files are refused by 1.2.6 with its "newer version" message. Projects from 1.2.6 and earlier open unchanged.
- **New in version 4:** more than 200 POVs for a source, POVs of an unknown media file, a negative or duplicate POV time or a line size outside 25–400% refuse the project as damaged; angles and zoom outside their limits are clamped (they are only view settings). POVs belong to the undo history and to the unsaved-changes comparison.

## Unsaved changes
- Unsaved changes are detected by comparing the edit with the last saved one, so undoing back to the saved state clears the marker. Background updates that are not edits (such as the subtitle-track check above) update both and never set the marker.
- The window title shows the project name with a `•` when there are unsaved changes, or "Untitled" before the first save.
- New, Open, closing the window and Quit (Cmd+Q on macOS, Ctrl+Q elsewhere) ask Save / Discard / Cancel when there are unsaved changes. A failed or cancelled save stops the action. On macOS the default application menu is turned off so that Cmd+Q reaches this prompt; as a consequence the standard Hide and Minimize menu items are not available.
- Saving is atomic (temporary file, then rename).

## What is not in a project
Program settings (layout, show boxes on the picture, recent projects, quality levels, the `ffmpeg` path), the selection of subtitle lines and text items, the list of installed fonts, the export choice for 360 projects (360 video or flat camera view, asked each time), the live 360 camera, and positions and sizes while a subtitle is being dragged. The undo history is not saved.
