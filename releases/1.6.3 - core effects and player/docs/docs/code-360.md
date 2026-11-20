# Code — boxes, text size and 360° video (1.2.6)

Part of [Code](code.md). Design: [architecture-360.md](architecture-360.md) and
[architecture-boxes.md](architecture-boxes.md). Requirements REQ-096..105.

## Where things are

| Piece | Module | Notes |
|---|---|---|
| Camera maths | `src/camera.rs` | Pure. `Camera` (yaw, pitch, zoom in hundredths of a degree), `Pov`, `CameraPaths`, `at()` interpolation, drag and wheel mapping, `filter()` and `command_lines()` |
| 360 in the render | `src/render.rs` | A 360 `Segment` has `sphere: Some(path)`; `build_graph` writes `fps, sendcmd, v360, setsar` instead of scale/pad; `Graph.command_files` are written next to the ASS file (`write_command_files`) |
| Recognising 360 | `src/media.rs` | `MediaFormat.spherical` (from ffprobe's side data); `is_360()` reads only it (the 1.2.6 switch `treat_as_360` was removed in 1.4.0) |
| Preview copies | `src/proxy.rs` | 360 rule: `SPHERE_COPY_ABOVE` (4096 wide), `SPHERE_COPY` (4096x2048), `sphere_too_large()` (over 16384x8192) |
| App state and commands | `src/app/sphere.rs` | `AppState.live` (window state), `effective_camera`, Set POV and its edits, `Set360`, resize / align / reset |
| Boxes (pure) | `src/ui/boxes.rs` | Handle hit-testing, cursors, proportional resize, align |
| Preview interaction | `src/ui/picture.rs` | Boxes and handles drawing, drag to move, corner resize, camera drag, wheel and pinch, right-click menu |
| Text size | `src/subtitles.rs`, `src/ass.rs`, `src/ui/placement.rs` | `Line.size` in thousandths (250..4000), `\fs`+`\bord` tags, boxes measured at the line's size |
| Project file | `src/project.rs` | Schema 4: `camera` section, `size` of lines, `spherical` of media (and, until 1.4.0, `treat_as_360`) |
| Setting | `src/settings.rs` | `show_boxes` (optional field, file stays version 2, lenient reading) |

## How the camera reaches ffmpeg

- One engine for preview and export: the segment of a 360 clip is
  `trim, setpts, fps, sendcmd=f=camN.txt, v360@cN=input=e:output=flat:reset_rot=1:w:h:d_fov:yaw:pitch:interp=linear, setsar=1, setpts=PTS+offset, format`.
  `N` is the segment index; the command file is only written (and `sendcmd` only added) when the path
  moves. A still camera (no POV, or one POV) is just the initial values of `v360`.
- The initial values are the camera at the segment start; the command file has one line for every
  frame at which the camera differs from the previous one, carrying only the values that changed:
  `0.050000-0.150000 v360@c0 yaw 4.50, v360@c0 pitch 1.00;`. Times are relative to the segment start.
- Preview stills and playback build the same graph from `shown_project()`: a copy of the project in
  which a camera you dragged (or a source without POVs but a live view) is one fixed POV at time 0.
  The live view therefore never reaches the saved project.
- FrameKey now holds the camera at the position and the subtitle size, so a camera change is a new
  still (latest request only) and nothing else is rendered twice.
- The camera view fills the output frame: no scale, no pad. For the automatic output format a 360
  clip counts as 1920x1080 at the source frame rate (`render::effective_format`, which also scales a
  huge bit rate down to the pixel count); a 360 clip is never stream-copied.

## Spike results (milestone 0, measured with ffmpeg 6.1.1)

- `yaw`, `pitch`, `d_fov` (and `h_fov`, `v_fov`) are commandable at run time (`T` flag). Positive yaw
  looks right, positive pitch looks up, degrees. The test picture with a white meridian and parallel
  checks this and the 1 degree accuracy (`tests/ffmpeg_integration/v126.rs`).
- **Commands add to the rotation unless the filter has `reset_rot=1`.** Without it `yaw 10` three
  times looks 30 degrees away. `camera::filter` always sets `reset_rot=1`, so every command is absolute.
- **A command at one instant is missed** when no frame has exactly that rounded time (frames are at
  `k/fps`; the instant needs `ts == start`). Each command therefore has an interval of half a frame
  either side of its frame (`START-END target command arg;`), which contains that frame only.
- **Cost:** each command re-computes the whole mapping of the output picture: about 75-100 ms at
  1920x1080 with 8 threads (4K source). One value per frame is about 0.3 x real time; three values
  per frame would be 0.1 x. Hence only the changed values are sent, and the zoom is one command
  (`d_fov`, the diagonal view: `v360` derives the horizontal and vertical ones from the output
  aspect; `camera::diagonal_fov_deg` converts, and the result equals `h_fov` to one grey level).
  A path that only turns (yaw) is the cheapest; this is the main known limit (see below).
- Instance names (`v360@cN`) let several 360 clips in one graph be commanded separately.

## Settled points

- **Size of the 360 preview copy:** 4096x2048 for a source wider than 4096 pixels (an 8K sphere);
  a source up to 4096 wide is read as it is; a normal video still gets the 960x540 copy. The
  "1280 pixels" in the first plan was only the width the normal rule would have given and is not used.
  The 360 copy has its own cache key (`proxy::cache_key_kind`), so a 360 and a flat copy of the same
  source never mix; the kind comes from the file (the "Treat as 360°" switch is removed since 1.4.0).
  "Preview quality: Original" never uses a copy; the "too large" gate (over 16384x8192) applies to
  the preview only, `RenderError::SphereTooLarge`.
- **Camera still latency:** the targets (400 ms from a 4K source, 700 ms from the 8K copy) are
  estimates, not gates. Measured on the development machine (test
  `big_spheres_get_a_4096_copy_and_camera_stills_are_quick_enough`): about 125 ms from a 4K x 2K file
  and 155 ms from the 4096x2048 copy (a tiny clip, ffmpeg start-up included). The only gate is that
  every interface frame stays under 200 ms during an export and subtitle generation
  (`look_around_and_resize_stay_responsive_on_big_spheres_while_exporting_and_matching`, 4K x 2K and
  8K x 4K metadata, camera drags, wheel and a box resize).
- **During playback** a drag restarts the stream 100 ms after the last movement
  (`AppState::tick_camera`), with the dragged camera as a fixed view.

## Boxes, size and commands

- Corner drag: `boxes::resize` scales the box proportionally with the opposite corner fixed; the
  factor is the pointer's projection on the diagonal, limited to 25%..400% of the normal size and to
  the picture. `ResizeLines` gives every selected line the size, the dragged line its new centre, one
  undo step. 1000 (100%) is stored as no size, so a line without size stays byte-identical to 1.2.5
  output (golden test `tests/fixtures/export-argv-1.2.5.txt`).
- Align: a line without a position is already centred horizontally; vertically it becomes the
  centre of the picture. Reset: selected lines lose position and size; with no line selected and a
  360 clip at the playhead the live view returns to the front (no undo).
- Toolbar: 12 commands, a separator, then Align horizontally / Align vertically / Reset; the group
  moves into the "»" menu as a whole (`toolbar::visible`). Shortcuts: Set POV `P`, Align `Ctrl/Cmd+Shift+H` / `J`,
  Reset `Ctrl/Cmd+Shift+R`.

## Tests and fixtures

- Pure: camera (path, short arc, clamps, drag/zoom, command lines), boxes, ASS tags, sizes.
- Real ffmpeg (`tests/ffmpeg_integration/v126.rs`): view directions at several times in preview and
  export, short way round, split edges, pitch and zoom commands, a 360 clip next to a normal one,
  detection with `tests/fixtures/spherical-360.mp4` and `flat-2to1.mp4` (made by
  `scripts/make-spherical-fixture.sh`, developer-only), text size ratio, 8K copy.
- Project: v4 round trip, 1.2.5 projects unchanged, hostile values (`project::tests`).
- App and interface: `src/app/sphere_tests.rs`, `src/ui/sphere_ux_tests.rs`, the responsiveness gate in `src/ui/tests.rs`.

## Known limits

- Export speed of a moving camera is about a third of real time or less at 1080p (see the cost
  above); the camera path is exact, not simplified. A cheaper update (fewer commands per second)
  or an in-process projection are future work.
- Two copies of the same part of one source share their POVs (POVs belong to the source).
- 360 audio does not follow the camera; no roll; no keyboard control of the camera.
