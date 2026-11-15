# Architecture: project files

Part of [Architecture](architecture.md). How projects are saved, opened, checked and kept compatible between versions.

## Format versions
Projects are JSON files (`.vlproj`) with a format version number.

| Version | Written by | Adds |
|---|---|---|
| 1 | 0.1.x | Media list and the video track (clips, cuts, fades between clips). |
| 2 | 0.2.0 – 1.1.x | Music track, volume and mute of video audio and music, subtitles (language, style, lines with word timings). |
| 3 | 1.2.0 | Fade from black at the start and to black at the end of the video, volume and mute of each video clip, the on-screen position of each subtitle line, and the text subtitle tracks found in each media file. |

- 0.3.0, 0.4.0 and 1.1.0 did not change the format (1.1.0 timeline markers are computed from the existing data; the layout and recent projects are program settings, never stored in a project).
- **Opening older files:** version 1 and 2 files are converted in memory when opened and give exactly the same picture, sound and export as before: fades only where they were, every clip at 100% volume, every subtitle line in its style's usual place. Automated tests compare the export commands of a 0.1 and a 1.1 project before and after the change.
- **Media imported before 1.2.0** have not been checked for subtitle tracks. When such a project opens, they are checked again in the background; this does not mark the project as changed. If subtitles are requested before the check has finished, it runs first.
- **Saving:** 1.2.0 always writes version 3, even when no new feature is used. The alternative (writing version 2 when possible) was rejected: it needs two writers and two sets of tests for little gain. Consequence: a project saved by 1.2.0 cannot be opened by 1.1 or earlier.
- **Files from a newer version** are refused with a clear message and are never modified. 1.1 refuses version 3 files in this way.

## Loading and validation
- Only regular files are read (a pipe or device named like a project cannot block the program), at most 16 MiB, enforced while reading.
- After loading, everything is validated: clip ranges, music clips use audio-only files and do not overlap, subtitle lines refer to existing media, have start before end and at most 1000 characters, at most 200,000 words. Deeply nested data is rejected.
- **New in version 3:** a clip volume above 100% or a subtitle position outside the picture is refused as a damaged project; references to unknown clips or lines are refused. Subtitle track descriptions (language, title) are only cached information that `ffprobe` can produce again, so bad values there are cleaned or shortened instead of refusing the project. A start or end fade on an empty timeline is ignored.
- See [security](architecture-security.md#project-and-settings-files) for the hostile-file tests.

## Unsaved changes
- Unsaved changes are detected by comparing the edit with the last saved one, so undoing back to the saved state clears the marker. Background updates that are not edits (such as the subtitle-track check above) update both and never set the marker.
- The window title shows the project name with a `•` when there are unsaved changes, or "Untitled" before the first save.
- New, Open, closing the window and Quit (Cmd+Q on macOS, Ctrl+Q elsewhere) ask Save / Discard / Cancel when there are unsaved changes. A failed or cancelled save stops the action. On macOS the default application menu is turned off so that Cmd+Q reaches this prompt; as a consequence the standard Hide and Minimize menu items are not available.
- Saving is atomic (temporary file, then rename).

## What is not in a project
Program settings (layout, recent projects, quality levels, the `ffmpeg` path), the selection of subtitle lines, and positions while a subtitle is being dragged. The undo history is not saved.
