# Architecture: project files and settings

Part of [Architecture](architecture.md). Requirements: [business](business.md) and the project items of each feature. Program settings: [performance](architecture-performance.md#start-up-and-settings) and [export](architecture-export.md#settings-and-window). Hostile-file rules: [security](architecture-security.md#project-and-settings-files).

## Format versions
Projects are JSON files (`.vlproj`) with a format number. The writer always writes the current number; older files open with the conversions below; newer files are refused with a message and never modified (1.1 refuses 3, and so on).

| Format | Written by | Adds |
|---|---|---|
| 1 | 0.1.x | media list, video track (clips, cuts, fades between clips) |
| 2 | 0.2.0 to 1.1.x | music track, volume and mute of video audio and music, subtitles (language, style, lines with word timings) |
| 3 | 1.2.0 | fades from and to black at the video's edges, volume and mute per video clip, subtitle line positions, text subtitle tracks of each media file |
| 4 | 1.2.6 | 360 video: "Treat as 360° video" choice and recognised flag per media, POVs (camera path points) per source; line size |
| 5 | 1.3.0 | text items (text, times, effects, colour, font, box, size); 360 anchors of texts and lines |
| 6 | 1.4.0 | clean audio per video clip; text width replaces the size; 360 anchor width and height; the 360 switch is no longer written (read to be dropped) |
| 7 | 1.5.0 | Change aspect of the video; Scale and Move with keyframes for the video and each clip (clip keys in source time) |
| 8 | 1.6.0 | Apply LUT: path of one `.cube` per video and per clip (no copy) |
| 9 | 1.6.1 | whole-video effects stored on the clips (own Scale, Move, LUT; own start and end fade); Change aspect stays one setting |
| 10 | 2.0.3 | text readability (None, Outline, Box) and box height 5 to 100 % |
| 11 | 2.0.4 | media size and date of analysis; 360 marks, roll, easing, per-clip camera paths (`clip` field, copy on write, REQ-268); fade lengths and sides; crop (removed in 2.3.0); clean audio strength and speech enhancement; music clip volume, mute, fades, clean audio; volume keys (`Key<u8>`); LUT strength and copies; blur level; clip speed (500 to 2000 thousandths, written only when not 1000); clip Change aspect (`clip_aspects`); picture items |
| 12 | 2.1.0 | `clip_order` (movable rows, top first); text shadow; subtitle style shadow, highlight colours, enlarge size; presets Enlarge word and Typewriter |
| 13 | 2.2.0 | the full row list of an ordered clip (written only when not default); text maximum size (`max_size`, 2 to 50 %, out of range dropped with a note); 2.1.0 refuses it (ADR-222) |
| 14 | 2.3.0 | overlaps per clip (`clip_overlaps`, ADR-230); Crop removed (read only); 1 to 14 open, newer refused |

- Nothing is written in versions 0.3.0, 0.4.0 or 1.1.0 (1.1 timeline markers are computed; layout and recent projects are settings).
- **Why a new number for 2.2.0:** a 2.1.0 program would otherwise lose rows it does not know; older programs refuse 13 with their newer-version message (REQ-029). Migration 12 to 13 in memory; written on save only.
- **Writing choice (1.2.0):** always the current version, even when no new feature is used. The alternative (writing older versions when possible) was rejected: two writers, two test sets. Consequence: a project saved by a newer version cannot be opened by an older one.
- **Format 11 is one format for the whole 2.0.4 development (D58):** later stages add optional fields with defaults; a file from an earlier stage opens with the same result. Read limit rises to 96 MiB (LUT copies of at most 20 MB each, 64 MiB per project).
- **Format 10 (2.0.3):** texts read as readability None (2.0.2 outline dropped) and box height 20 % (the 2.0.2 box); size fitted again on opening (the status bar says once how many changed and that the outline was removed; not again once saved in format 10).

## Conversions when opening (in memory)
- **Formats 1 and 2:** give the same picture, sound and export: fades only where they were, every clip at 100 % volume, every subtitle line in its style's usual place. Tests compare export commands of 0.1 and 1.1 projects.
- **Format 3 to 4 (1.2.6):** versions 1 to 3 open with identical results (no 360 clips, no sizes; golden tests). Media without the 360 flag, and media imported before 1.2.0 without the subtitle-track check, are checked again in the background (no change mark; a subtitle request runs the check first).
- **Format 5 to 6 (1.4.0):** the text size of 5 and the 360 switch of 4 are converted with no undo step and no change mark; large multi-line texts may shrink to fit their new box (confirmed choice A).
- **Format 6 to 7:** no picture effects; **7 to 8:** no LUT.
- **Format 8 to 9 (1.6.1):** whole-video Scale or Move becomes a copy of its keys in each clip (key times in clip time, stored in source time; the value at each edge kept); whole-video LUT becomes the same LUT per clip; whole-video start and end fades become the first clip's start fade and the last clip's end fade; the project-wide video level is multiplied into each clip's level (rounded to 1 %) and set to 100 %. A clip with its own Scale or Move gets one combined path (scales multiplied, moves added at the key times of both; exact there). A clip with its own LUT keeps it. Above 200 keys the clip's own are kept first. The status line at opening says how many clips changed.
- **Format 12 to 13 (2.2.0):** texts read with no maximum (keep their sizes); a project with a changed order of movable rows opens with those rows on top of the default order (its look does not change).
- **Format 13 to 14 (2.3.0, ADR-241):** crops dropped (count); a picture item whose start lies in a clip becomes an overlap of that clip (its `picture:<id>` row becomes `overlap:<new id>`); an item with no clip stays a picture item. `Project.converted: Option<Conversion>` gives one status line: "Opening: N clip(s) changed by the conversion (M crop(s) dropped[, K picture(s) made overlaps])"; no undo step; not unsaved.

## Loading and validation
- Only regular files are read (pipe or device refused), at most 16 MiB for formats up to 10 and at most 96 MiB from format 11; the cap is enforced while reading.
- After loading everything is validated: clip ranges; music clips use audio-only files and do not overlap; subtitle lines refer to existing media, start before end, at most 1000 characters; at most 200,000 words; deeply nested data refused. A start or end fade on an empty timeline is ignored.
- Format 3: clip volume above 100 % or a subtitle position outside the picture → refused as damaged; references to unknown clips or lines refused. Track descriptions (language, title) are cached `ffprobe` information: cleaned or shortened, never a refusal.
- Format 4: more than 200 POVs for a source, POVs of an unknown media file, negative or duplicate POV times, or a line size outside 25 to 400 % → damaged; angles and zoom clamped (view settings). POVs belong to undo history and the unsaved-changes comparison.
- Format 5: text time to live under 0.1 s or over 24 hours → damaged (longer than the video accepted, limited when added, edited or moved); effect time outside 0.1 to 5 s; colour not `#RRGGBB`; font name outside the allowed characters or over 100; more than 2000 blink events per item or 20000 per project; anchors outside the sphere.
- Format 6: text width outside 10 to 100 % or anchor width or height outside its range → damaged.
- Format 7 and 8: picture values corrected on load with a note (scale 10 % to 400 %, centre to the frame, duplicate times dropped, keys outside the media dropped, more than 200 cut, unknown shapes, fits or colours replaced); damaged LUT paths (empty, too long, control character) cleared with a note; paths must be UTF-8.
- Format 9: keyframe counts, times and values checked and capped (200 per clip) before the conversion; a damaged file is refused, never a crash.
- Format 10 and 11: readability and box height ranges; stage fields optional with defaults; out-of-range speed, volume keys, blur level and LUT strength corrected or dropped with a note.
- Format 12: an unknown order id dropped; shadow and enlarge values out of range set to defaults; each adds a line to the repair notice (2.1.0).
- Format 13: text maximum outside 2 to 50 dropped ("A text size maximum outside 2 to 50 was removed"); the stored order repaired ("A damaged effect order was repaired").
- Format 14: `validate_overlaps` drops damaged overlaps ("N damaged overlap(s) were removed").
- A missing font, LUT file or media is kept and reported (the project opens; export names the file).

## Saving and unsaved changes
- Saving is atomic (temporary file, then rename). Projects are written only in the current format.
- Unsaved changes are detected by comparing the edit with the last saved one: undo back to the saved state clears the marker. Background updates that are not edits (track checks, conversions) never set it.
- The window title shows the project name with `•` when there are unsaved changes, or "Untitled" before the first save.
- New, Open, closing the window and Quit (Cmd+Q on macOS, Ctrl+Q elsewhere) ask Save / Discard / Cancel when there are unsaved changes; a failed or cancelled save stops the action. On macOS the default application menu is off so Cmd+Q reaches the prompt (ADR-024); Hide and Minimize are therefore not in the menu.
- Autosave (2.0.4): a clone of an unsaved project is written to the cache every two minutes by a worker (`autosave`); recovery is offered at the next start.
- Recovery and work folders: see [security](architecture-security.md#temporary-and-other-files).

## What is not in a project
Program settings (layout, show boxes, recent projects, quality levels, the ffmpeg path, LUT favourites); the selection of lines and texts; the list of installed fonts; export choices (kept for the session only, [export](architecture-export.md#export-dialog-140-req-118-to-122)); the live 360 camera; positions and sizes while a subtitle is dragged; the undo history.
