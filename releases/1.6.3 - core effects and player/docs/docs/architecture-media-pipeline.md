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
| Audio extraction for subtitles | 1 | short; since 1.2.0 can be limited to one clip's part |
| Reading a subtitle track (1.2.0) | 1 | converts one text track to SRT; reads the file without decoding video |

At the limit of 3, export + playback use all slots; everything else waits.

- **Interruption:** if preview or export cannot start, a running proxy job is stopped and restarted later from the beginning. Export is never interrupted.
- **Dropping stale work:** a newer still-frame request replaces an older one (an older render still running is stopped); restarting playback stops the previous pair; removing a media item cancels its analysis and proxy; New/Open/close cancels all jobs of the project.
- **Never on the UI thread:** import analysis and the startup `ffmpeg` check run in the background. While the check runs the UI shows "checking ffmpeg…" and import, preview and export are disabled.

## Preview
- **Still frame only when needed (REQ-034):** for the playhead position the program computes a description of the picture (source frame, fade state, output size, visible subtitles). A new frame is rendered only when that description changes, after 80 ms of quiet; editing after the playhead renders nothing, and a burst of edits renders at most once. The frame is rendered from the one clip at the playhead, not the whole timeline.
- **Clock (REQ-035):** the wall clock drives playback. Video never waits for audio. Audio is compared with the clock and corrected by skipping or inserting silence when it is more than 40 ms off; after an audio stall it catches up within that limit. (In 0.1.3 audio drove the clock, so video froze when audio stalled.)
- **Music and subtitles** are part of the preview, built with the same mix and the same subtitle file as the export, so preview and export look and sound the same.
- **Proxies** are used when ready; until then, the original.
- **Paused picture (1.6.1, REQ-144):** the preview shows the picture at the playhead after a pause, a seek or an edit, never a stale or black frame where the timeline has picture. The likely cause (code reading; the test that confirms it is the first step of the 1.6.1 work) is the REQ-034 skip rule: a still frame that was requested but then superseded or dropped (a seek during the 80 ms quiet period, or a render that finished during playback) can leave its key marked as "shown or requested", so the same picture is not rendered again and the last frame on screen stays (possibly black, from a fade). The fix keeps REQ-034 (no render when the picture is unchanged, one render per burst of edits) and makes the marks follow the newest request only; play, pause and seek always start from a clean state. Alternative rejected: render on every pause (correct, but a render for each pause and seek to the same picture; kept as the fallback if the fix is not enough).

## Export
- **Threads:** in Light, each export uses at most half the cores; in Balanced and Maximum `ffmpeg` chooses, as in 0.2.2.
- **Fewer inputs (REQ-037):** consecutive clips cut from the same source, in forward order, share one `ffmpeg` input, which is split inside `ffmpeg`. A new input is opened only when the order goes back in time (reordered or repeated clips). 200 segments cut in order from one source use one input and one decoder, and `ffmpeg` memory stays bounded. Music clips are grouped the same way. Large filter graphs go through a script file, as before.
- **HEVC tag (REQ-036):** HEVC in MP4 or MOV gets the `hvc1` tag (needed by Apple players), with stream copy and with re-encode.
- **Re-encode is needed** when there is any fade (including, since 1.2.0, a fade from or to black at the start or end), music clip, subtitle line, video volume or clip volume other than 100% or muted, mixed source formats, or a cut not on a keyframe (REQ-023). Since 1.4.0 also any clip with Clean audio, a video preset or an audio format other than "Same as source".
- **Audio chain per clip (1.4.0):** each video clip's sound runs through its own chain: Clean audio (`afftdn`, when on), then the clip volume, then the fades. The music is mixed after the chains. Preview and export build the chain with the same function, and the Only audio export uses the whole timeline's audio graph without any video branch. See [effects](architecture-effects.md#clean-audio-140) and [export dialog](architecture-export-dialog.md#what-the-export-does-with-the-choices).
- **Clip volume (1.2.0):** a clip whose level is not 100% gets its own `volume` filter before its fades; see [effects](architecture-effects.md#clip-volume).
- **Originals only:** export never uses proxies; the export plan is identical whatever the Preview quality.
- Cancel and failures still leave no partial file.
- **Video hardware (0.4.0):** with Export "Faster", an H.264/HEVC re-encode may run on the video hardware first, followed by an output check and, on any problem, a redo with the normal method. These processes run one after another in the export slot. See [hardware export](architecture-hardware-export.md).

## Music mix
- One music track below the video track; music clips never overlap and do not move when video is edited (REQ-019, REQ-043).
- The music bed is built from the music clips with silence in the gaps, cut at the end of the video. A clip that runs past the end fades out over the last 0.5 s. There is no music where there is no music clip.
- Video audio and music each have a volume (0–100%) and a mute switch, saved in the project (REQ-020); since 1.2.0 each video clip also has its own, multiplied by the video level. Volume follows a perceptual curve: gain = (volume / 100)², so 50% is about −12 dB; mute is silence.
- The two are mixed without automatic level reduction, followed by a limiter that prevents clipping.

## Proxies
Lower-resolution copies used only for preview when a source is "big for this machine" (REQ-038).
- **Rule (Preview quality "Automatic", the default):** a proxy is made when the picture is larger than 1920x1080, or larger than 1280x720 on a machine with fewer than 8 logical cores or less than 8 GB of RAM. It depends only on resolution (rotation included), cores and RAM, so it is deterministic. Audio-only files never get one. The rule is a tunable assumption.
- **360 video (1.2.6):** a 360 source wider than 4096 pixels gets a 4096x2048 copy instead of the rule above, so the view stays sharp; export always uses the original. See [360 video](architecture-360.md#preview-and-export-graph).
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
