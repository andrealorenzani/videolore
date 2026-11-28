# Architecture: timeline aids and project as media (3.2.0, built)

Back to the [architecture index](architecture.md). Decisions: [ADR-276 to 284](architecture-decisions.md#built-in-320-adr-276-to-284). Requirements: [timeline](business-timeline.md), [project as media](business-editing.md), [picture and playlist](business-player.md).

Status: built in 3.2.0.

## Ruler marks and pointer line (REQ-367, REQ-368)
- A pure function in `src/ui/ruler.rs` picks the mark step from frame, 1 s, 1 min, 10 min and 1 h: the smallest step whose spacing is at least 6 px at the current zoom. The labels keep the REQ-093 table.
- The pointer line is an overlay on the timeline, drawn only while the pointer is over the timeline. It uses the same x-to-time mapping as a click.

## Picture strips (REQ-369, REQ-370)
- New module `src/strips.rs`. In the lower half of each clip, one thumbnail per grid step, from the clip's start. The step is the smallest ruler step whose spacing fits the thumbnail at the current zoom.
- Thumbnails come from one background ffmpeg run per request (lowest priority, one at a time), reading the preview copy when there is one. They are kept in memory only (LRU, 32 MiB cap); nothing is written to disk.
- Effects are not drawn. A program setting "Show picture strips on the timeline" (on by default, Settings > Interface) turns the strips off; with it off nothing is decoded.

## Project as media (REQ-371 to REQ-373)
- File > Import project as media… reads the chosen `.vlproj` with the bounded loader. Unreadable or missing sources are refused with the file name.
- The imported item is a normal video media. Its file is an intermediate rendered by the export pipeline with a fixed preset (H.264 crf 12, AAC 256 kbit/s), in the private work folder. Its key is the SHA-256 of the source file.
- The item keeps a link to the source. On reopen, a changed source (or a missing intermediate) starts a rebuild; the item shows "rendering" until it is done. The window never waits.
- A project cannot import itself, directly or through another project (reference walk, depth at most 8).
- The project file format becomes 16; every save writes 16, and older programs refuse it (ADR-280).

## Player additions (REQ-374 to REQ-376)
- Playlist menu: move up, move down and remove; Next and Previous follow the order. Durations are probed once per item in the background.
- The total of the list is shown in brackets after the current video's total when the list has two or more videos. Until every duration is known it shows dashes.
- File > Take screenshot: the picture as on screen (subtitle lines and the 360 view included), saved as PNG through a save dialog that opens in the Pictures folder, named after the video and the time of the frame. Nothing is written next to the video.

## Limits and targets
- The frame stays within 200 ms while strips are decoded (REQ-056 rule). Strip cache 32 MiB. One strip job at a time. The process cap of ADR-015 holds.
- Project as media: the rendering takes the export slot of the job queue; progress and cancel as for an export.

## Security
- An imported project is untrusted input: the same bounded read and validation as any project; no shell; ffmpeg with `-protocol_whitelist file,pipe`.
- The intermediate lives in the private work folder (0700), its names are known to the crash clean-up.
- The screenshot is written only to the path the user chose.
