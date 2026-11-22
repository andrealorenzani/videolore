# Architecture

Index of the architecture. Each topic is in its own document; read only the one you need. Requirements are in the [business](business.md) documents, the code in [code](code.md), the history of each change in the [changelog](changelog.md).

## Overview
`videolore` is a desktop video editor with a native window (Linux, macOS, Windows). It does not decode or encode video itself: it drives `ffmpeg` and `ffprobe`, which are **bundled with the installers** and run as separate programs. The edit (clips, cuts, fades, music, effects, Scale and Move, LUTs, texts, subtitles, pictures and overlays) is kept in memory, previewed, and exported with a single `ffmpeg` run. The preview and the export come from the same graph builder, so what is seen is what is exported. The separate player (`videolore-player`) uses the same preview engine for one video, with no editing and no export.

Current build: 2.3.0 (built, not released). Last release: 1.6.3.

## Sub-documents
| Topic | Document |
|---|---|
| Components and code structure | [components](architecture-components.md) |
| Main technical decisions (one line each, with ADR ids) | [decisions](architecture-decisions.md) |
| Media engine: ffmpeg, job queue, preview, music, proxies, memory | [engine](architecture-engine.md) |
| Export: dialog, formats, faster export with video hardware | [export](architecture-export.md) |
| Performance, automatic setup and speed | [performance](architecture-performance.md) |
| Picture: frame, Scale, Move, LUT, overlays | [picture](architecture-picture.md) |
| Effects, order and the Applied effects column | [effects](architecture-effects.md) |
| Text items and picture items | [text](architecture-text.md) |
| Subtitles, speech-to-text and subtitle styles | [subtitles](architecture-subtitles.md) |
| 360 video | [360](architecture-360.md) |
| Interface, boxes, icons and splash | [UI](architecture-ui.md) |
| Videolore player | [player](architecture-player.md) |
| Project files and settings | [projects](architecture-projects.md) |
| Security | [security](architecture-security.md) |
| Non-functional targets, limits and measurements | [NFR](architecture-nfr.md) |
| Build, CI, tests and verification | [build](architecture-build.md) |
| Installers and release pipeline | [packaging](architecture-packaging.md) |
| Dependencies and tools | [dependencies](architecture-dependencies.md) |
| Open risks and open decisions | [risks](architecture-risks.md) |
| Module architecture (dependencies, threads, flows, mermaid diagrams) | [code-architecture](code-architecture.md) |

## Components
The module map, its layers and the rule of no module cycle: [components](architecture-components.md). The module graph, the threads and the import, preview, export and command flows: [code-architecture](code-architecture.md).

## Data flow
- **Start-up:** detect the computer, clean up a crashed session, read the settings, decide the recommended settings (first start, changed computer, older version, damaged file), apply them to the job queue and the preview. See [performance](architecture-performance.md).
- **Editing:** every user action changes the edit and is recorded for undo; when the picture at the playhead changes, one still frame is requested (bursts of edits merged). See [engine](architecture-engine.md#preview).
- **Import:** each file appears at once as "analysing"; `ffprobe` runs in the background; a large source may get a preview copy. See [engine](architecture-engine.md#proxies-req-038).
- **Export:** the plan is built from the original files (never copies); the worker runs one `ffmpeg` and reports progress; the result is renamed into place only when complete. With Faster export and working hardware, the re-encode runs on the hardware first, is checked, and is redone with the normal method if anything is wrong. See [export](architecture-export.md).
- **Effects and picture:** dragging, the clip's context menu, the Effects menu and double-click produce the same command; a whole-video effect is distributed as copies on the clips; each flat clip is composed in the frame (background, picture with its Scale and Move, fades, LUT, overlaps) by the same graph in preview and export. See [effects](architecture-effects.md) and [picture](architecture-picture.md).
- **Subtitles:** for one clip or the whole video; a source with a text track lets the user choose its text or speech recognition; the processor is checked; the model of the chosen level is used (downloaded after consent); the audio is transcribed on the machine; the result is applied as one undo step. See [subtitles](architecture-subtitles.md).
- **360 video and boxes:** a 360 clip is rendered through a virtual camera; dragging the picture moves a live camera; boxes and handles are drawn by the preview only. See [360](architecture-360.md).
- **Texts and overlays:** text items become ASS events, cut per segment and burned with the subtitles; picture items and overlap media are stages of the same graph. See [text](architecture-text.md) and [picture](architecture-picture.md#picture-overlays-and-overlap-media-230-built).
- **Spatial sound:** a 360 clip with four audio channels is turned by the camera before it is mixed (a pre-pass writes a float stereo WAV in the private work folder). See [engine](architecture-engine.md#export-pipeline-common-part).
- **Settings:** a change applies to the next job; running jobs continue; settings are saved atomically and never stored in projects.
- **Player:** builds an in-memory one-clip project for the same preview engine. See [player](architecture-player.md).
- **Double-click:** on Windows and Linux the path arrives on the command line; on macOS as a system event; both go through the unsaved-changes prompt.

The UI never waits on `ffmpeg` or on disk, apart from saving or loading the project file (at most 96 MiB from format 11, 16 MiB before), the settings file (at most 64 KiB) and a few small files at Linux start-up.

## Projects
Versioned JSON files; the current format is 14 (2.3.0). Files from a newer version are refused and never modified; loading validates strictly; the unsaved-changes prompt is shown on New, Open, close and Quit. Each older format opens with its documented conversion. See [projects](architecture-projects.md).

## Deviations from the first plan (as built)
- **Time to live of a text:** a file is refused as damaged only outside 0.1 s to 24 hours; a value longer than the video is accepted, and when a text is added or moved it is limited to the video length. See [text](architecture-text.md#deviations-as-built).
- **360 marker in MP4, MOV, MKV and WebM:** `ffmpeg` drops the spherical information on re-encode, so `videolore` writes the Spherical Video V1 marker itself (MKV and WebM since 2.0.4). See [360](architecture-360.md#export-as-360-video).
- **Anchored subtitle lines on 360 clips:** since 2.0.4 they keep their preset outline or box and their word highlight. See [360](architecture-360.md#anchored-text-and-subtitles-130-204).
- **Overlap storage:** overlaps live in `Timeline.clip_overlaps`, not in the `Copy` clip type (2.3.0). See [picture](architecture-picture.md#picture-overlays-and-overlap-media-230-built).

## Non-functional targets and security
- Headline targets: the window reacts within 200 ms per frame while an export and subtitle generation run together; `ffmpeg` processes never exceed the limit of the chosen level (3 for Light and Balanced, 3 to 6 for Maximum); memory does not grow with the source size; coverage at least 85 % and every implemented requirement covered by a test. Measured figures and their status (met, not met, estimate): [NFR](architecture-nfr.md).
- Security model: untrusted inputs (media, projects, settings, `ffmpeg` on `PATH`, downloads, typed text, LUT and overlay files), no shell, bounded reads, private work folders, one network use (speech model, after consent), unsafe code in three reviewed places. See [security](architecture-security.md).

## Build, CI, packaging and dependencies
Rust toolchain policy (latest stable, MSRV field 1.100), the workspace, the CI jobs and the version rules: [build](architecture-build.md). Installers, the release pipeline and the runners: [packaging](architecture-packaging.md). Crates and bundled tools with their reasons and licences: [dependencies](architecture-dependencies.md).

## Known risks
The open risks, the measured targets not met (REQ-177, REQ-315, QG-05) and the open product decisions are listed in [risks](architecture-risks.md).

