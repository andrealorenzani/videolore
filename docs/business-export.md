# Business — Export

Back to the [index](business.md).

## Features
- Automatic output format, with no settings dialog.
- As fast as possible: no re-encoding when it is not needed.
- Progress and cancel; a cancelled export leaves no partial file.
- Music and subtitles are included in the exported file (subtitles are burned into the picture).
- HEVC files play on Apple devices.

## Requirements
- REQ-021 (implemented): Export shows progress and can be cancelled; a cancelled export leaves no partial file.
- REQ-022 (implemented): The output format is chosen automatically: if all clips share the same format it is kept; otherwise the lowest by pixel count, then frame rate, then bitrate is used, with letterboxing and never stretching.
- REQ-023 (implemented): Export does as little processing as possible: stream copy (no re-encode) when all clips share an identical format, there are no effects (fades, subtitles, music) and every cut starts on a keyframe; otherwise a single re-encode pass, always frame-accurate.
- REQ-036 (implemented): Exported HEVC video in MP4 or MOV carries the Apple-compatible tag (hvc1), both with stream copy and with re-encode.
- REQ-037 (implemented): A timeline of 200 segments cut from one source exports successfully and frame-accurately.

## Key decisions
- Output format is automatic: there is no export settings dialog.
- Stream copy is used only when the result is guaranteed frame-accurate; correctness comes before speed.
- Changing the video audio volume (or muting it) also requires re-encoding, like music and subtitles.
- H.264 and HEVC export are available with the bundled ffmpeg; output formats are the same as in 0.1.3.

## Out of scope
- Export settings dialog.
- Separate subtitle files (SRT/VTT) or switchable subtitles; subtitles are always burned in.
- Direct upload to social networks.
