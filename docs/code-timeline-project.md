# Code — 3.2.0: timeline aids, project as media, project format 16

Back to the [index](code.md). Player and recorder additions of the same version: [code-player.md](code-player.md), [code-screen-spy.md](code-screen-spy.md).

## Ruler marks and pointer line (REQ-367, REQ-368)
- `ui/ruler.rs:marks` picks the smallest step (frame, 1 s, 1 min, 10 min, 1 h, then 2, 4, 8… h) whose marks are at least 6 points apart (`MARK_GAP`). Marks are 3 points high at the bottom of the ruler (`timeline::MARK_H`, `RULER_H` 15), under the labels of REQ-093 (`ticks`, unchanged). The step table lives in `base/steps.rs`, shared with the picture strips.
- The pointer line is painted by `ui::timeline::tracks` from the ruler to the music track, only while the pointer is over the timeline (`UiCache::pointer_line`). It is drawn at the pointer's own x, so the time under it is the time a click sets (then snapped to a frame).

## Picture strips (REQ-369, REQ-370)
- `strips.rs`. Pure: `box_size` (lower half of the clip, 8 to 64 points high in steps of 4, width by the picture's shape, at most 256), `grid_step` (smallest ruler step that leaves 4 points between snapshots), `slots` (only the visible places, only snapshots that fit whole, source times follow the clip's start and speed).
- The window calls `Strips::begin_frame` and `want` for each visible place, and paints what `get` returns (textures in `UiCache`, dropped when no longer shown). `AppState::tick_strips` runs `Strips::tick`.
- One ffmpeg at a time, `Priority::Background`, preemptible, reading the preview copy when there is one. Snapshots up to 2 s apart in the source come from one run (`fps` filter); further apart each is sought on its own. A run nobody waits for is killed; one that takes over 60 s is stopped; a snapshot that cannot be made stays blank and is not tried again.
- Memory only: at most 32 MiB of RGBA, least recently shown first. Cleared when another project is loaded or the switch is off.
- Switch: Settings > Appearance and start-up, "Show picture strips on the timeline" (`Settings::show_picture_strips`, on when absent).

## Project as media (REQ-371 to REQ-373)
- The item is a normal video `MediaItem` with `from: Some(source project)`; `path` is the rendered video.
- `nested::read` loads the source with the normal bounded loader, walks the imported projects (at most 8 deep, 64 in all), refuses a chain that returns to a project already in it (also through links that are not on the timeline), a missing source, overlay or LUT file (by name) and a timeline without video. It runs on the window's thread when the file is chosen (it reads project files only) and again in the background job.
- The key (32 hex digits) is a SHA-256 of a version tag, the project's path and bytes, the size and time of the files it uses and the keys of the projects it imports. The video is `<work folder>/imports/<key>.mkv` (folder 0700); it is reused when present and readable.
- `nested::render` makes the videos of imported projects first (bottom up), fits their new lengths into the clips (`replace_media`, `refit`), then runs the normal export with `ExportMode::Intermediate`: always re-encoded, H.264 `veryfast` CRF 12, AAC 256 kbit/s, Matroska, software encoder, the source's size and frame rate. The export's own partial file and crash clean-up apply.
- `app/imports.rs`: File > Import project as media… → check → question (`Modal::ImportProject`) → item in "rendering" state (not saved until ready) → one background job at a time at export priority, shown in the status bar with Cancel. When a project is opened every imported item is queued: same video, nothing changes; another video, the item takes it (a clip that went to the end still does, the others are only shortened when needed; undo steps longer than the new video are dropped); a source that cannot be read marks the item missing with a message. Export waits while an item on the timeline is queued or rendering.

## Project format 16 (REQ-029)
A media entry may hold `from_abs` and `from_rel` (the source project, found beside the project when the folder moved). Every save writes 16; 1 to 15 open as before; 3.1.x refuses 16.

## Tests
`ui/ruler.rs`, `strips/tests.rs`, `nested/tests.rs`, `app/imports_tests.rs`, `ui/timeline_aids_ux_tests.rs`, `project/v15_tests.rs` (format 16), real ffmpeg: `tests/ffmpeg_integration/v32.rs`.

## Known limits
- The imported video is one more generation of encoding and takes disk space: about 84 MB and 21 s per minute for a synthetic 1080p30 clip on an 8-thread computer (direct ffmpeg run with the same settings). Old videos in `imports/` are never removed.
- A change of the source project is seen when the project holding the item is opened again.
- Cancelling the rebuild of an item leaves it as it was (missing when it had no video) until the project is opened again.
- Snapshot times (same computer, 1080p30 clip, ffmpeg 6.1.1): one sought snapshot 0.3 s; one run of ten snapshots 4 s apart 4.4 s, hence the 2 s limit of a run.
