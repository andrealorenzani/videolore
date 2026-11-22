# Architecture: 360 video

Part of [Architecture](architecture.md). Requirements: [360 picture](business-picture.md) (REQ-100 to 105), [text](business-text.md), [export](business-export.md). Handles: [UI](architecture-ui.md#boxes-on-the-picture-126-140-161). Built in 1.2.6, 1.3.0, 1.4.0 and 2.0.4 (stages 10 and 13).

## Approach and recognition
- A 360 clip (equirectangular) is shown and exported through a **virtual camera**: yaw, pitch and zoom (horizontal field of view). The output is a flat picture.
- **One engine for preview and export:** the ffmpeg `v360` filter; preview and export graphs come from the same code, so fades, subtitles and music work unchanged. Rejected: own CPU reprojection (two implementations to agree within 1°) and a GPU shader (untestable headless).
- A camera change in the preview is a new still request (latest only): about 125 to 155 ms on tiny clips; an in-process view is a roadmap option.
- **Detection:** `ffprobe` spherical information; a video is 360 when it says equirectangular with no cropped bounds and no stereo layout; only these fixed values are read. Stored with the media; older media are checked again in the background (no change mark).
- **1.4.0: no switch.** A source is 360 exactly when its file says so (REQ-100). The 1.3.0 "Treat as 360° video" switch is dropped on opening (a 360-tagged clip with the switch off becomes 360). A file without spherical information stays flat (product risk).
- The initial orientation of the file is ignored ("front" is the middle). 2.0.4: a clip can be marked as 360 explicitly (format 11).

## Camera and points of view
- A **POV** is a time in the **source** file with yaw, pitch, zoom and roll (2.0.4; centi-degrees, ±180°, default 0), stored per source in whole hundredths of a degree. POVs outside the kept part are hidden, not lost; two copies of one part share their POVs. Up to 200 per source; setting one at an existing time replaces it.
- Between two POVs the camera moves linearly in time; yaw the short way round (350° to 10° passes 0°); pitch and zoom directly; before the first and after the last it stays. Segments can be **eased** (2.0.4: smoothstep, exact at both ends; `ease` flag per POV). With no POV: the live view, or front, level, 90°.
- Maths in the pure `camera` module: limits (pitch ±90°, zoom 30° to 110°, yaw wrapped), interpolation, drag mapping `angle = pixels × zoom ÷ preview width` (same in preview and playback), wheel and pinch zoom, projection between sphere and view (rectilinear, the field of `v360`).
- **Live view:** the dragged camera is window state (one per source): not saved, not in undo. With POVs a drag overrides the path until seek, play or Reset. **Set POV** stores the camera at the playhead. A subtitle box under the pointer is dragged instead of the camera.
- **Per-clip paths (2.0.4, REQ-268):** `CameraPaths` keeps each source's path and, per clip, its own (`clips`, keyed by clip id). A clip follows its source's path until its path is first changed; then it is copy on write (`set_in_clip`, `edit_in_clip`, `remove_in_clip`). Copied, pasted (`clipboard_camera`) and split clips get a copy. `AppState::clip_for` finds a POV's clip. Saved with a `clip` field; entries of clips not in the project are dropped on opening.

## Preview and export graph
- A 360 segment gets, after trim and frame-rate steps, a `sendcmd` filter and `v360@cN` (equirectangular in, flat out, `reset_rot=1`) whose output is the output picture. A still camera needs no command file.
- A moving camera is a command file in the private work folder: numbers only, from checked values. Commands are **absolute** (`reset_rot=1`). Each has its own interval of **half a frame either side** (a single instant is missed); only **changed values** are sent; the size is bounded (steps doubled for very long paths).
- The view **fills the output picture** (no black bars). Zoom is one command, `d_fov` (diagonal field); `v360` derives the horizontal and vertical fields from the output aspect.
- Automatic output format: a 360 clip counts as 1920x1080 at the source's frame rate; never stream copied (REQ-023). Projects without 360 clips give the same commands as 1.2.5 (golden).
- Faster export and the output check apply as usual (camera filter in software before the encoder).
- **Preview copies:** a 360 source wider than 4096 px gets a 4096x2048 copy (the normal rule would give 1280 px, about 320 across a 90° view); export uses the original. Above 16384x8192 the preview shows "This 360° video is too large to show here. It is still exported." and decodes nothing.
- **Command rate (REQ-266):** at most 10 updates per second of video; during a drag the view is half size and returns to full size within 200 ms of release (REQ-267).
- **Speed:** a moving camera exports at about one third of real time or less at 1080p (ffmpeg 6.1.1; the target of half real time not met; accepted). Each command makes `v360` recompute its mapping (75 to 100 ms at 1080p); a path that only turns is cheapest. Memory: no frame kept by videolore; writing the command file for a 10-minute 30 fps path takes under 100 ms; path lookup is a binary search.

## Performance targets
- Camera still latency: 400 ms (4K source) and 700 ms (8K preview copy) are **estimates**; measured about 125 ms and 155 ms on tiny clips. Gate: the interface stays under 200 ms per frame while looking around during an export and subtitle generation (4K x 2K and 8K x 4K).

## Anchored text and subtitles (1.3.0, 2.0.4)
- A text item or subtitle line added at the playhead on a 360 clip gets an **anchor**: the sphere point at the view centre, in hundredths of a degree (yaw, pitch), with its size (default height 5 % of the view's vertical field at add time; wrap 60 % of the horizontal field; assumption). The flat place is kept and used on flat clips.
- **Burn-in in the sphere:** in preview and camera exports anchored items are written into the equirectangular frame of their segment **before** `v360`, at x = (yaw/360 + ½)·width and y = (½ − pitch/180)·height; a box across ±180° is written twice. `v360` projects them like the picture: no per-frame update, no second projection.
- **Box (1.4.0):** width 10 to 360° (hundredths of a degree), height 20 % of the view's vertical field at add time; size = best fit in the frame ([text](architecture-text.md#box-size-and-place)).
- **Consequence (Andrea's decision):** text far from the equator is drawn flat on the sphere, narrower towards the poles, as in any flat picture in a 360 video.
- **Slide in** of an anchored text moves in the plane of the sphere picture, not across the view.
- **Automatic subtitle lines** on a 360 clip have no anchor (view-fixed, drawn after the projection). Lines generated on a 360 source are anchored at the view centre when created (`AppState::generation_anchors`, `finish_create`); lines created without a 360 view keep the view-fixed place.
- **2.0.4 (ADR-188, REQ-257, 258):** an anchored line is a text item in the sphere frame with the preset's outline or box and the word highlight (karaoke `ass::highlight_chunks`; the highlight state is not saved; `render/segments.rs` `anchored_line_items`). This supersedes the plain white of 1.3.0 (DEV-3).
- **Re-anchor (2.0.4, REQ-259):** `Command::ReAnchor` (`texts.rs` `re_anchor_plan`) moves selected anchors to the current view, one undo step; in Edit > Subtitles, the picture's right-click menu and the Context Menu group (a selected line and the playhead on a 360 clip).
- **Off-view arrows (2.0.4, REQ-260):** `ui/placement.rs` `off_view_arrow` gives the nearest picture edge and direction; `ui/picture.rs` `off_view_arrows` draws them; a click sends `TurnCamera`.

## Export as 360 video
- Only for projects with 360 clips. Since 1.4.0 the choice **360 video** or **Flat camera view** is in the Export dialog, shown only when all video clips are 360; a mixed project is exported flat without a message ([export](architecture-export.md#video-and-audio-formats)). 1.3.0 asked each time (REQ-103).
- **360 video** keeps the whole sphere: no `v360`, no camera commands; each segment equirectangular at the 360 clips' size (the automatic rule with the lowest pixel count). Anchored items at their anchors; automatic subtitle lines at their preset place (Andrea's decision: the lower third, about 30 degrees below the front). It ignores Change aspect, Scale, Move, the render order, pictures and overlaps.
- **Spherical marker:** ffmpeg 6.1.1 drops the source's spherical information on re-encode (measured). videolore writes the Spherical Video V1 marker into MP4 and MOV (`spherical` module: only the small movie box is rewritten; the rest copied). **MKV and WebM (2.0.4, REQ-261, ADR-187):** the Projection element of the Matroska video track is written in place: the Void padding between SeekHead and Info shrinks by the bytes Tracks grows, so nothing after Tracks moves; pose elements use IDs 0x7673 to 0x7675 (0x7672 is ProjectionPrivate). Checked with ffprobe; the real-player check is manual.
- A mixed project: 1.3.0 refused the 360 video with a message; 1.4.0 exports a flat camera view silently.
- The live camera and camera path are not applied; the output check and Export: Faster apply; the speed-first preset (2.0.4, Q-5): the fastest encoder preset within 1.5 times the Best-quality size.
- **Spatial sound (2.0.4, REQ-273):** a 360 clip with four channels has its sound turned by the camera (see [engine](architecture-engine.md#export-pipeline-common-part)); the stereo pair does not follow the roll; the 360 video export keeps the sound as recorded.

## Tests
- Pure: path interpolation (45° at the middle of 0° to 90°; short arc; hold; single POV; replace; 200 limit); clamps; drag and zoom mapping; edge views after a split; projection and inverse (behind the camera; pitch clamp); seam duplication; anchor from the view; flat fallback.
- Real ffmpeg (`tests/ffmpeg_integration/v126.rs`): a generated equirectangular picture with coloured bands; the view shows the expected bands (yaw and pitch signs; 1°); preview and export agree; command timing frame by frame; a committed tiny file with spherical information (under 100 KB) is detected. Test sphere 1024 x 512 (2.0.4).
- Anchored text at yaw 30°, pitch 10° is 40° left of centre after a 40° right turn (±1°) and returns; in the 360 video it is at yaw 30° (±1°) at three sample times; the output carries the spherical information.
- Interface frames under 200 ms while looking around with texts during export and subtitle generation (4K x 2K and 8K x 4K). Project format 4 round trip and hostile values ([projects](architecture-projects.md)).

## Risks
Listed in [risks](architecture-risks.md#technical-risks): `reset_rot` and the commands measured on 6.1.1 only; camera stills from large sources may feel slow; files without spherical information stay flat.
