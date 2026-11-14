# Architecture: media pipeline

Part of [Architecture](architecture.md). How `ffmpeg` work is scheduled, and how preview, export, music and proxies are built.

## Job queue: a fixed process limit
Never more than the limit of the chosen "How hard videolore works" level of `ffmpeg`/`ffprobe` processes run at once (REQ-033): 3 for Light and Balanced, between 3 and 6 for Maximum (see [performance](architecture-performance.md#how-hard-videolore-works)). It is never below 3, because playback needs 2 and an export 1. Lowering the limit never stops a running process; new ones wait. The `jobs` module enforces this: every part of the program receives a process runner from the queue, so the limit cannot be bypassed. Speech-to-text inference is not an `ffmpeg` process and has its own limit (see [subtitles](architecture-subtitles.md)).

Priorities, highest first: **preview**, **export**, **import analysis**, **background work** (proxies, audio extraction for subtitles). Within the same priority, first come first served.

| Job | Processes | Notes |
|---|---|---|
| Preview playback | 2, taken together | one for video, one for audio |
| Still preview frame | 1 | |
| Export | 1 | keyframe checks before it run one at a time |
| Import analysis (`ffprobe`) | 1 each | up to the limit in parallel; stopped after 30 s |
| Proxy generation | 1 | can be interrupted |
| Audio extraction for subtitles | 1 | short |

At the limit of 3, export + playback use all slots; everything else waits.

- **Interruption:** if preview or export cannot start, a running proxy job is stopped and restarted later from the beginning. Export is never interrupted.
- **Dropping stale work:** a newer still-frame request replaces an older one (an older render still running is stopped); restarting playback stops the previous pair; removing a media item cancels its analysis and proxy; New/Open/close cancels all jobs of the project.
- **Never on the UI thread:** import analysis and the startup `ffmpeg` check run in the background. While the check runs the UI shows "checking ffmpeg…" and import, preview and export are disabled.

## Preview
- **Still frame only when needed (REQ-034):** for the playhead position the program computes a description of the picture (source frame, fade state, output size, visible subtitles). A new frame is rendered only when that description changes, after 80 ms of quiet; editing after the playhead renders nothing, and a burst of edits renders at most once. The frame is rendered from the one clip at the playhead, not the whole timeline.
- **Clock (REQ-035):** the wall clock drives playback. Video never waits for audio. Audio is compared with the clock and corrected by skipping or inserting silence when it is more than 40 ms off; after an audio stall it catches up within that limit. (In 0.1.3 audio drove the clock, so video froze when audio stalled.)
- **Music and subtitles** are part of the preview, built with the same mix and the same subtitle file as the export, so preview and export look and sound the same.
- **Proxies** are used when ready; until then, the original.

## Export
- **Threads:** in Light, each export uses at most half the cores; in Balanced and Maximum `ffmpeg` chooses, as in 0.2.2.
- **Fewer inputs (REQ-037):** consecutive clips cut from the same source, in forward order, share one `ffmpeg` input, which is split inside `ffmpeg`. A new input is opened only when the order goes back in time (reordered or repeated clips). 200 segments cut in order from one source use one input and one decoder, and `ffmpeg` memory stays bounded. Music clips are grouped the same way. Large filter graphs go through a script file, as before.
- **HEVC tag (REQ-036):** HEVC in MP4 or MOV gets the `hvc1` tag (needed by Apple players), with stream copy and with re-encode.
- **Re-encode is needed** when there is any fade, music clip, subtitle line, video volume other than 100% or muted, mixed source formats, or a cut not on a keyframe (REQ-023).
- **Originals only:** export never uses proxies; the export plan is identical whatever the Preview quality.
- Cancel and failures still leave no partial file.
- **Video hardware (0.4.0):** with Export "Faster", an H.264/HEVC re-encode may run on the video hardware first, followed by an output check and, on any problem, a redo with the normal method. These processes run one after another in the export slot. See [hardware export](architecture-hardware-export.md).

## Music mix
- One music track below the video track; music clips never overlap and do not move when video is edited (REQ-019, REQ-043).
- The music bed is built from the music clips with silence in the gaps, cut at the end of the video. A clip that runs past the end fades out over the last 0.5 s. There is no music where there is no music clip.
- Video audio and music each have a volume (0–100%) and a mute switch, saved in the project (REQ-020). Volume follows a perceptual curve: gain = (volume / 100)², so 50% is about −12 dB; mute is silence.
- The two are mixed without automatic level reduction, followed by a limiter that prevents clipping.

## Proxies
Lower-resolution copies used only for preview when a source is "big for this machine" (REQ-038).
- **Rule (Preview quality "Automatic", the default):** a proxy is made when the picture is larger than 1920x1080, or larger than 1280x720 on a machine with fewer than 8 logical cores or less than 8 GB of RAM. It depends only on resolution (rotation included), cores and RAM, so it is deterministic. Audio-only files never get one. The rule is a tunable assumption.
- **Format:** H.264 at most 960x540, same frame rate, a keyframe every half second (fast seeking), AAC stereo, in Matroska.
- **Generation:** in the background, one at a time, with progress on the media item; written under a temporary name and renamed when complete. In Light it uses at most half the cores.
- **Cache:** in the system cache folder (`…/videolore/proxies/`), outside the project. The file name is a hash of the source path, size, modification time and proxy settings, so a changed source gets a new proxy. Proxies are reused across sessions and recreated if missing. Clearing deletes only proxy files not being generated. There is no automatic clean-up.
- **Preview quality (since 0.3.0, replaces the on/off switch):** Automatic (rule above), Smoother (every source above 1280x720), Original (no proxies). A program setting, not a project setting. 0.2.x users who had turned proxies off get Original.
- Settings shows the space used as "Preview copies" with a button to clear them.

## Memory
App memory does not depend on source file size (REQ-039). Every buffer has a fixed limit:

| Buffer | Limit |
|---|---|
| Preview frames | 8 frames of 640x360 (about 7 MB) |
| Preview audio | 1 s |
| `ffmpeg` error log | 64 KiB |
| Output of short `ffmpeg`/`ffprobe` runs | 16 MiB |
| Speech-to-text audio chunk | 120 s (about 7.7 MB) |
| Keyframe lists | read line by line |

Target: a long source uses at most 200 MB more peak memory than a short one of the same resolution. Assumption: CI measures this with a 2–5 minute source; because no buffer grows with file size, the result is expected to hold for a 4 GB file.

## Risks
- Two processes for playback leave only one slot for everything else while playing during an export; other jobs wait.
- A proxy interrupted by preview/export starts again from the beginning.
