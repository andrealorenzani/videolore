# Architecture: 360 video

Part of [Architecture](architecture.md). Built in 1.2.6 (requirements REQ-100..105, see [business](business-picture.md)); the code is described in [code-360](code-360.md). Boxes, resizing and the align/reset commands are in [boxes](architecture-boxes.md).

## Approach
- A 360 clip (equirectangular, one picture) is shown and exported through a **virtual camera**: yaw (left/right), pitch (up/down) and zoom (horizontal field of view). The output is always a normal flat picture.
- **One engine for preview and export:** the `ffmpeg` `v360` filter turns the sphere into the flat view. The preview graph and the export graph are built by the same code, so what you see is what you export, with fades, subtitles and music already working. Alternatives rejected: own CPU reprojection for the preview (two implementations that must agree within 1°, and subtitles would be distorted or need their own overlay) and a GPU shader (cannot be tested headless, the same two-implementation problem).
- Consequence: a camera change in the preview is a new still-frame request (latest request only, short delay), not an instant redraw. Measured latency on small test clips is about 125-155 ms; an instant in-process view stays a roadmap option.

## Recognising a 360 video
- `ffprobe` already prints the stream's spherical information. A video is 360 when it says equirectangular projection, with no cropped bounds and no stereo layout. Only these fixed values are read; no text from the file is shown or used.
- The result is stored with the media information (not recognised yet for media imported earlier: they are checked again in the background when the project opens, like the subtitle tracks of 1.2.0, without marking the project as changed).
- The user's switch "Treat as 360° video" in Clip details is stored per source in the project; it wins over the detection. The 360° tag and the camera view use the combined result. Changing the switch is not an undo step (like importing) but marks the project as changed.
- The initial orientation written in the file is ignored: "front" is the middle of the picture.

## Camera and POVs
- A POV is a time in the **source** file with yaw, pitch and zoom, kept per source in the project (whole hundredths of a degree, so values compare exactly). Because the time is the source's, the path stays continuous after a split, trim or cut and POVs outside the kept part are hidden, not lost, exactly like subtitles. Consequence: two copies of the same part of one source share their POVs.
- The camera at a time: before the first POV and after the last it stays on them; between two it moves linearly with time, yaw the short way round (350° to 10° passes through 0°), pitch and zoom directly. No POV at all: the live view (below), or front, level, 90° if the user never looked around. Up to 200 POVs per source; setting one at an existing time replaces it.
- The maths lives in a pure `camera` module: limits (pitch ±90°, zoom 30°–110°, yaw wrapped), interpolation, mapping a drag of N pixels to an angle (`angle = pixels × zoom ÷ preview width`, the same in preview and playback), and wheel/pinch zoom.
- **Live view:** the camera you drag is window state (one per source), not saved and not in the undo history. With POVs, a drag overrides the path until seek, play or Reset; without POVs it simply stays. **Set POV** stores the camera shown at the playhead.
- A subtitle box under the pointer is dragged instead of the camera.

## Preview and export graph
- The segment of a 360 clip gets, after the usual trim and frame-rate steps, a `sendcmd` filter and a `v360` filter named `v360@cN` (N = segment number; equirectangular in, flat out, `reset_rot=1`) whose output size is the output picture. Subtitles, fades and the rest follow unchanged. A still camera needs no command file: its values are the filter's initial values.
- A moving camera is written to a small command file in the private work folder: numbers only, from checked values, so nothing from a project can reach the command. Commands are **absolute** (`reset_rot=1`; without it `v360` adds each command to the current rotation). Each command has its own time interval of **half a frame either side** of its frame, because a command at a single instant is missed when no frame has exactly that time. Only the **values that changed** since the previous frame are sent. The size is bounded (steps are doubled if a very long path would exceed it).
- The camera view **fills the output picture** (output aspect, no black bars). The zoom is sent as one command, `d_fov` (the diagonal field of view); `v360` derives the horizontal and vertical ones from the output aspect, and the result equals the horizontal zoom to one grey level.
- For the automatic output format (REQ-022) a 360 clip counts as 1920x1080 with the source's frame rate. A 360 clip is never stream-copied (REQ-023). Projects without 360 clips produce exactly the same commands as 1.2.5 (golden test).
- Faster export and the output check work as before: the camera filter runs in software before the encoder; the check compares sample frames with the normal render.
- **Preview copies:** a 360 source wider than 4096 pixels gets a 4096x2048 copy (the normal rule would give 1280 pixels for a whole sphere, only about 320 across a 90° view). Export always uses the original.
- **Size limit:** above 16384x8192 the preview shows "This 360° video is too large to show here. It is still exported." and nothing is decoded; the export still runs.

## Performance and limits
- Camera still latency: targets of 400 ms (4K source) and 700 ms (8K preview copy) are **estimates, not gates**; measured about 125 ms and 155 ms on tiny test clips (ffmpeg start-up included). The only gate: the interface stays under 200 ms per frame while looking around during an export and subtitle generation (automated test, 4K x 2K and 8K x 4K files).
- **Export of a moving camera is slow: about one third of real time or less at 1080p** (measured with `ffmpeg` 6.1.1; the earlier target of half real time was not met). Each command makes `v360` recompute its whole mapping (75-100 ms at 1080p), so one changed value per frame costs about 0.3x real time and three would cost 0.1x; hence only changed values are sent and zoom is one command. A path that only turns (yaw) is cheapest; a still camera has no extra cost. Accepted for 1.2.6; speed work (fewer commands per second, an in-process projection) is on the [roadmap](roadmap.md). The bundled `ffmpeg` 9.x was not measured; the same behaviour is assumed.
- Memory: no frame is kept by `videolore`; `ffmpeg` holds a few frames (an 8K frame is about 100 MB at 8 bits).
- Path lookup is a binary search; writing the command file for a 10-minute 30 fps path takes under 100 ms (unit test).

## Tests
- Pure: path interpolation (45° at the middle of 0°→90°, short arc, hold before and after, single POV, replace, 200 limit), clamps, drag and zoom mapping, edge views after a split.
- Real `ffmpeg` (`tests/ffmpeg_integration/v126.rs`): a generated equirectangular test picture with coloured longitude and latitude bands; the view at several angles must show the expected bands (this fixes the yaw and pitch signs and proves 1° accuracy); preview and export of the same time agree; command-file timing frame by frame; a committed tiny file with spherical information (under 100 KB) for detection; the same file without it is flat until the switch is on.
- Project: format version 4 round trip, limits and hostile values (see [project files](architecture-projects.md)).

## Risks
- Checked and resolved by the spike: `v360` accepts run-time changes of yaw, pitch and zoom (`d_fov`); positive yaw looks right, positive pitch looks up, in degrees. The cost per command is the export-speed limit above.
- Remaining: `reset_rot` and command behaviour were measured only on `ffmpeg` 6.1.1, not on the bundled 9.x; the real-`ffmpeg` picture tests guard sign, unit and timing conventions on whatever version runs them.
- Camera stills from large sources may feel slow while dragging; possibly an instant in-process view (roadmap).
- Spherical information is read only from what `ffprobe` reports; some cameras write none (the switch covers it).
