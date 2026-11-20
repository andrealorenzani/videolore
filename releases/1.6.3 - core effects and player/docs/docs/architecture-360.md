# Architecture: 360 video

Part of [Architecture](architecture.md). Built in 1.2.6 (requirements REQ-100..105, see [business](business-picture.md)); the code is described in [code-360](code-360.md). Boxes, resizing and the align/reset commands are in [boxes](architecture-boxes.md). Anchored text and subtitle lines on the sphere, and export as 360 video, were built in 1.3.0 (REQ-103, REQ-112).

## Approach
- A 360 clip (equirectangular, one picture) is shown and exported through a **virtual camera**: yaw (left/right), pitch (up/down) and zoom (horizontal field of view). The output is always a normal flat picture.
- **One engine for preview and export:** the `ffmpeg` `v360` filter turns the sphere into the flat view. The preview graph and the export graph are built by the same code, so what you see is what you export, with fades, subtitles and music already working. Alternatives rejected: own CPU reprojection for the preview (two implementations that must agree within 1°, and subtitles would be distorted or need their own overlay) and a GPU shader (cannot be tested headless, the same two-implementation problem).
- Consequence: a camera change in the preview is a new still-frame request (latest request only, short delay), not an instant redraw. Measured latency on small test clips is about 125-155 ms; an instant in-process view stays a roadmap option.

## Recognising a 360 video
- `ffprobe` already prints the stream's spherical information. A video is 360 when it says equirectangular projection, with no cropped bounds and no stereo layout. Only these fixed values are read; no text from the file is shown or used.
- The result is stored with the media information (not recognised yet for media imported earlier: they are checked again in the background when the project opens, like the subtitle tracks of 1.2.0, without marking the project as changed).
- Until 1.3.0 the user's switch "Treat as 360° video" in Clip details was stored per source in the project and won over the detection. **Since 1.4.0 there is no switch**: a source is 360 exactly when its file says so (REQ-100). The switch of 1.3.0 projects is dropped when they open (a flat clip stays flat; a 360-tagged clip with the switch off becomes 360). The 360° tag and the camera view use the detection only.
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
- Spherical information is read only from what `ffprobe` reports; some cameras write none. Until 1.3.0 the switch covered them; since 1.4.0 such a file stays flat (documented product risk, roadmap: a choice at import).

## Anchored text and subtitles (1.3.0)
- A text item or a subtitle line added at the playhead on a 360 clip gets an **anchor**: the point of the sphere at the centre of the view at that moment, in hundredths of a degree (yaw, pitch), plus its world size (the default text height is 5% of the view's vertical field of view at add time, stored in degrees; wrap width 60% of the horizontal field, assumption). The anchor is stored in the project; the flat place is kept too and is used on flat clips.
- **Burn-in in the sphere:** in the preview and in the camera exports, anchored items are written into the equirectangular frame of their 360 segment **before** the `v360` filter, at the frame position of their point (x = (yaw/360 + ½)·width, y = (½ − pitch/180)·height; a box across the seam at ±180° is written twice). `v360` then projects them like the picture, so they stay put when the camera turns, with no per-frame update and no second projection. Fades and music are unchanged.
- **Projection math:** a pure `camera` function projects a sphere point into the view (rectilinear, same field of view as `v360`) and back. It draws the boxes and drags in the look-around view; a test compares it with real `ffmpeg` output within 1°.
- **Flat clips:** an anchored item shown while a flat clip plays is drawn at its flat place. A text that spans a flat and a 360 clip is cut per segment.
- **Consequence (decided by Andrea):** a text far from the equator is drawn flat on the sphere, so it looks narrower towards the poles, as any flat picture in a 360 video does. Text that is only seen while looking straight ahead is unaffected.
- **Automatic subtitle lines** on a 360 clip have no anchor: they keep the view-fixed place of 1.2.x (drawn after the projection in the camera views).
- **Slide in** of an anchored text moves in the plane of the sphere picture (its direction is that of the frame), not across the view, because a moving position per frame would need a second renderer.
- **Built differently from the plan:** an anchored subtitle line is drawn as plain white text, without its preset style (no box, no word highlight).
- **1.4.0:** an anchored text has a box of width 10–360° and a height of 20% of the view's vertical field at the time it was added; its size is the best fit in the equirectangular frame (see [text](architecture-text.md#box-best-fit-and-place-140)). Anchors store the width and height in hundredths of a degree.

## Export as 360 video (1.3.0)
- Only for a project with 360 clips, Export asks **360 video** or **Flat camera view** each time (REQ-103); the answer is not saved. Projects without 360 clips do not ask. **Since 1.4.0** the question is a choice in the Export dialog, shown only when all video clips are 360 clips; a mixed project is exported flat without a message (see [export dialog](architecture-export-dialog.md#360-choice)).
- **360 video** keeps the whole sphere: the graph has no `v360` and no camera commands; every segment is equirectangular at the 360 clips' own size (the automatic format rule with lowest pixel count). Anchored items are at their anchors; automatic subtitle lines are at their preset place in the frame (decided by Andrea: the lower third of the frame, about 30 degrees below the front). The output carries the Spherical Video V1 marker so players show a sphere. ffmpeg 6.1.1 drops the source's spherical information when it re-encodes (measured), so `videolore` writes the marker itself into MP4 and MOV files: only the small movie box is rewritten, and the rest of the file is copied unchanged (module `spherical`). MKV and WebM 360 exports carry no marker, so players show them as flat video.
- A project mixing 360 and flat clips is refused for a 360 video with a message; nothing is exported. (1.4.0: such a project offers no 360 choice and is exported as a flat camera view, silently.)
- The live camera and the camera path are not applied. Export speed has no target (accepted: slow).
- The output check (REQ-063) and "Export: Faster" apply as for any re-encode.

## Tests for 1.3.0
- Pure: projection and its inverse (edge cases behind the camera, pitch clamp), seam duplication, anchor from the view at the playhead, flat fallback.
- Real `ffmpeg`: a text anchored at yaw 30°, pitch 10° is 40° left of the centre after the camera turns 40° right (±1°) and returns to its first place; in the 360 video the text is at yaw 30° (±1°) at three sample times; the output has the spherical information.
- Performance: interface frames stay under 200 ms while looking around with texts during an export and subtitle generation (4K×2K and 8K×4K).
