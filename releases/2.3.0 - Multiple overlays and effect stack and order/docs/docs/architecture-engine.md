# Architecture: media engine (ffmpeg, job queue, preview, music, proxies)

Part of [Architecture](architecture.md). Requirements: [platforms](business-platforms.md), [export](business-export.md), [player](business-player.md), [music](business-music.md), [performance](business-performance.md). Module map: [code-architecture](code-architecture.md). Export: [export](architecture-export.md). Machine and settings: [performance](architecture-performance.md).

## Bundled ffmpeg
- **Why:** since 0.2.0 import, preview and export work right after installation (REQ-030). videolore runs `ffmpeg` and `ffprobe` as **separate programs**, never linked, which keeps the build simple and the licences separate (ADR-002, ADR-013).
- **Builds (GPL, FFmpeg 9.0.x):** Windows x86_64 BtbN `win64-gpl-shared` (shared DLLs); Linux x86_64 BtbN `linux64-gpl` (static); macOS arm64 Martin Riedl "release" static builds. LGPL builds rejected (no x264 or x265); building in CI (30+ minutes); evermeet.cx (Intel only); Homebrew (shared libraries spread over the system).
- **Pin:** `packaging/ffmpeg.lock` lists per platform URL, SHA-256, version, provider, licence, the provider's build-script commit and the ffmpeg.org tarball with its SHA-256. `scripts/fetch-ffmpeg.sh <target> <dest>` verifies and extracts only `ffmpeg`, `ffprobe` and the DLLs needed. Refreshed each release (ffmpeg parses untrusted media). BtbN keeps month-end builds 2 years; 0.3.0 pinned a daily build (if it disappears the build fails loudly). Riedl URL retention unknown.
- **Smoke test before packaging:** version; encoders `libx264`, `libx265`, `libvpx-vp9`, `libopus`, `libmp3lame` (1.4.0); filters `subtitles`, `amix`, `alimiter`, `afftdn` (1.4.0), `lut3d` (1.6.0); hardware encoders (0.4.0): `h264_nvenc`, `hevc_nvenc`, `h264_qsv`, `hevc_qsv`, `h264_vaapi`, `hevc_vaapi` (Linux), `h264_amf`, `hevc_amf` (Windows), `h264_videotoolbox`, `hevc_videotoolbox` (macOS).
- **Location:** beside the executable: Windows installer folder (with DLLs); macOS `Contents/MacOS/` inside the signed app; AppImage `usr/bin/`; tar.gz top folder. Notices and fonts beside them or in `Contents/Resources/`, `usr/share/videolore/`.
- **Lookup order:** settings path; `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`; bundled copy (executable folder, then `../Resources`); `PATH`. Settings show the path, version and origin.
- **Start-up check (background):** version, encoders and filters (also `subtitles`, `amix`, `alimiter`, `volume`, `split`, `asplit`). Messages: "bundled copy damaged" (reinstall) or "not installed" (source builds).
- **Licences:** videolore is GPL-3.0-only; the GPL ffmpeg builds (`--enable-gpl --enable-version3`; x264 and x265 GPL 2 or later) are compatible. Each release has `videolore-<version>-third-party-source.tar.xz` (ffmpeg source and signature, provider build scripts at the pinned commit, Noto Sans source and licence, README). Notices carry a written offer valid 3 years and a note on H.264/HEVC patent licensing. Assumption: not reviewed by a lawyer.
- **Subtitle rendering and fonts:** `subtitles` (libass) in all builds; bundled Noto Sans Bold (SIL OFL 1.1, about 0.5 MB, `assets/fonts/`); installed fonts for text items by family name through the system. Limitation: other scripts fall back to system fonts.

## Job queue
- At most the limit of "How hard videolore works" ffmpeg/ffprobe processes at once (REQ-033): 3 (Light, Balanced), 3 to 6 (Maximum). Never below 3 (playback needs 2, export 1). Lowering the limit never stops a running process. `jobs`: every part gets a process runner from the queue (`ffmpeg::Runner`; `SystemRunner` in programs, fakes in tests); arguments are lists, never a shell. Speech inference is not a process and has its own thread limit.
- **Priorities:** preview, export, import analysis, background (proxies, audio extraction); first come, first served within a priority.
- **Jobs and slots:** preview playback 2 (video and audio together); still frame 1; export 1 (keyframe checks run one at a time before it); import `ffprobe` 1 each (stopped after 30 s); proxy 1 (interruptible); audio extraction for subtitles 1 (can be limited to one clip's part); reading a subtitle track 1. At the limit of 3, export plus playback use all slots.
- **Interruption:** a preview or export that cannot start stops a running proxy, which restarts later from the beginning. Export is never interrupted.
- **Stale work dropped:** a newer still request replaces an older (its render stopped); restarting playback stops the previous pair; removing a media cancels its analysis and proxy; New, Open and close cancel the project's jobs.
- **Never on the UI thread:** import analysis and the start-up check run in the background ("checking ffmpeg…" disables import, preview and export meanwhile).

## Preview
- **Still only when needed (REQ-034):** the playhead's picture is described (`FrameKey`: source frame, fade state, output size, visible subtitles, LUT, order, overlaps); a frame is rendered only when the description changes, after 80 ms quiet; a burst renders at most once; only the clip at the playhead is rendered.
- **Latest request only:** `preview::still_thread` (background; `mpsc: Frame`). Paused picture (1.6.1, REQ-144): after a pause, seek or edit the picture at the playhead is shown; the marks follow the newest request; play, pause and seek start clean (alternative "render on every pause" kept as fallback). **Clip end (2.0.3):** a seek or pause in the last frame of a clip shows its picture (the graph starts from the frame holding the playhead).
- **Clock (REQ-035):** the wall clock drives playback; video never waits for audio; audio is compared with the clock and corrected by skipping or inserting silence beyond 40 ms (0.1.3 let audio drive the clock and video froze during stalls). Threads: `preview::start_playback` (`sync_channel: Frame`), an audio reader thread, `AudioBuffer` (Arc), the cpal callback `AudioSink` (feature `audio`; without it the preview is silent and wall-clocked).
- **Music and subtitles** are in the preview, built with the same mix and subtitle file as the export.
- **Hardware decoding (2.0.4, stage 5):** with Export "Faster" the preview may decode on the video hardware; a failure mid-playback restarts at the playhead with software (`hwdec.rs`). Overlay files are never hardware-decoded (2.3.0).
- **Proxies** are used when ready; until then the original. **Still background (REQ-223, ADR-180):** a blurred background kept as one raw RGBA file `bg-<hash>.rgba` in the private still folder (stills only); playback blurs each frame. Background work runs at background priority.
- **Preview copy cache (REQ-297, ADR-198):** clearing re-queues the needed copies; the limit removes the oldest by modification time, never the open project's; it runs on the UI thread (estimate under 100 ms for 1000 copies).

## Export pipeline (common part)
- **Threads:** Light: each export at most half the cores; Balanced and Maximum: ffmpeg chooses.
- **Originals only:** export never uses proxies; the plan is the same whatever the Preview quality. Cancel and failure leave no partial file (temporary name, then rename).
- **Fewer inputs (REQ-037):** consecutive clips cut from one source, in forward order, share one input split inside ffmpeg; a new input only when time goes back. Music clips are grouped the same way. Large graphs go through a script file.
- **HEVC tag (REQ-036):** `hvc1` in MP4 and MOV, with stream copy and re-encode.
- **Re-encode needed (REQ-023):** any fade (edge fades to black included), music clip, subtitle line, text or picture item, a volume other than 100 % or a muted clip, Clean audio (1.4.0), speed (2.0.4), Change aspect, Scale, Move, LUT, overlap (2.3.0), mixed source formats, a cut not on a keyframe, a video preset or an audio format other than Same as source (1.4.0). Otherwise stream copy when safe.
- **Audio chain per video clip (1.4.0):** Clean audio (`afftdn`, when on), then the clip volume, then the fades; the music is mixed after the chains. Preview and export use the same function; Only audio uses the timeline's audio graph without video. Details: [effects](architecture-effects.md).
- **Spatial sound pre-pass (2.0.4):** a 360 clip with a 360 picture and four channels has its sound turned by the camera (first-order ambisonics, order W, Y, Z, X assumed; a fixed cardioid stereo pair; roll not applied). `spatial::prepare` is one ffmpeg process through the queue, decoding the four channels at 48 kHz as raw float on a pipe; blocks of 480 samples (10 ms) are rotated by the camera path (read in source time, interpolated inside a block, so no click). Export: `export::prepare_spatial` after the work folder (also for Only audio) writes `spatial-<hash>.wav` (32-bit float stereo, `.part` then rename; the name hashes file, range and camera path). The 360 video export clears it. Preview: `Preview::prepare_spatial` runs before the audio starts; the wall clock starts after it (`Preview.prepass`); a clip over 10 minutes keeps its recorded sound.

## Music
- One music track below the video track; music clips never overlap and do not move when video is edited (REQ-019, REQ-043). The bed is built from music clips with silence in the gaps, cut at the video end; a clip running past the end fades out over the last 0.5 s. No music where there is no clip.
- Video audio and music have volume 0 to 100 % and mute, saved in the project (REQ-020); each video clip has its own too (1.2.0), multiplied by the video level. Gain = (volume / 100)^2 (50 % about −12 dB). Music clips have their own volume, mute and Clean audio (2.0.4, REQ-276, 277).
- Mixed without automatic level reduction (`amix`), then a limiter (`alimiter`) prevents clipping (ADR-010, ADR-019).

## Proxies (REQ-038)
- **Rule (Preview quality Automatic, default):** a proxy when the picture is larger than 1920x1080, or larger than 1280x720 on a machine with fewer than 8 logical cores or less than 8 GB RAM. It depends only on resolution (rotation included), cores and RAM; audio-only files never get one. Tunable assumption.
- **360 (1.2.6):** a 360 source wider than 4096 px gets a 4096x2048 copy; export uses the original.
- **Format:** H.264 at most 960x540, same frame rate, a keyframe every half second, AAC stereo, Matroska. Background, one at a time, progress on the media item; temporary name then rename; in Light at most half the cores.
- **Cache:** the system cache folder (`…/videolore/proxies/`), outside the project; name = hash of source path, size, modification time and settings; reused across sessions; recreated if missing; clearing deletes only files not being generated; no automatic clean-up beyond the cache limit. Settings shows "Preview copies" with a clear button.
- **Preview quality (0.3.0):** Automatic (rule above), Smoother (every source above 1280x720), Original (none). A program setting; 0.2.x users who had proxies off get Original.

## Memory (REQ-039)
Memory does not depend on source size. Fixed buffers: preview frames 8 of 640x360 (about 7 MB); preview audio 1 s; ffmpeg error log 64 KiB; short ffmpeg and ffprobe output 16 MiB; speech chunk 120 s (about 7.7 MB); keyframe lists read line by line. Target and test: [NFR](architecture-nfr.md).

## Risks
- Two playback processes leave one slot for everything else during an export. A proxy interrupted by preview or export starts again. Others: [risks](architecture-risks.md#technical-risks).
