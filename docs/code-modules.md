# Code — modules and patterns

All logic is in the library (`src/lib.rs` and its modules); `src/main.rs` only reads the
project given on the command line, runs the desktop start-up hooks and opens the window.

## Module map

| Module | Responsibility |
|---|---|
| `time` | `Micros` (integer microseconds), `Fps` (rational), frame rounding, timecodes |
| `media` | `MediaItem` (with `MediaStatus` analysing/ready/rejected and `ProxyState`), `MediaFormat`, defensive parser of `ffprobe` JSON, `probe()` |
| `timeline` | The video track: contiguous clips + fades at boundaries; every edit returns `Result<_, EditError>` |
| `music` | The music track (`MusicTrack`): audio clips at their own times, never overlapping; `Mix` (volume 0–100 % and mute for video sound and music, gain = (p/100)²) |
| `subtitles` | Subtitle lines stored in *source* time per file; `project()` maps them onto the timeline (cut removes, copy duplicates, trim hides, split keeps); edits; the three styles |
| `ass` | Writes the ASS subtitle file: styles, text size `min(0.055·H, 0.09·W)`, word highlight with `\k` tags, escaping of `{ } \` |
| `history` | Undo/redo by snapshots of `Edit` (timeline + music + subtitles + volumes), up to 100 steps |
| `project` | `.vlproj` file (JSON, `schema_version` 2; version 1 files are migrated on load, newer ones refused untouched): atomic save, validated load, `ProjectDoc` for unsaved-changes detection |
| `format` | Same-format check, lowest-quality output choice, codec to encoder mapping with H.264/AAC/MP4 fallback |
| `render` | Export plan (stream copy or one re-encode), filter graphs (clips grouped by source, music bed and mix, subtitle burn-in), `hvc1` tag, preview argv, `FrameKey` |
| `ffmpeg` | Locating binaries (setting, environment, bundled, `PATH`), version/filter/encoder checks, `Runner` trait (real processes or test fake), progress and keyframe parsers |
| `jobs` | The process scheduler: at most 3 ffmpeg/ffprobe processes, priorities, preemption, cancellation; `Limited` runner; small worker `Pool` |
| `export` | Export worker thread: progress, cancel, subtitle files in a private working folder, partial file and rename |
| `preview` | Preview player: still frames on demand (debounced, latest only) and playback (RGBA video + PCM audio from two ffmpeg processes), wall-clock master with audio correction |
| `proxy` | Small preview copies of big sources: the machine rule, the cache (keys, size, clear) and the generation job |
| `machine` | Cores and memory of the running machine; thread count for speech recognition |
| `stt` | Subtitle generation: audio extraction, 120 s chunks, language detection/override, cancel, word timings to lines; model store (size + SHA-256), download and local-file install; whisper.cpp and the HTTPS downloader behind the `stt` feature |
| `notices` | Bundled components with version, licence and source; the About text |
| `packaging` | Pinned ffmpeg builds and tools (`packaging/*.lock`) and release asset names |
| `desktop` | Linux menu entry / icon / `.vlproj` type for the AppImage and tar.gz; macOS Apple Event handler for double-clicked projects; queue of projects to open |
| `app` | `AppState`, the single command dispatcher (`Command`, `AppState::dispatch`), background results in `tick()` |
| `ui` | Panels (`layout`), media list (`media`), timeline with music lane (`timeline`), effects/settings panel (`effects`), dialogs (`modals`), zoom, shortcuts (`keymap`) |
| `dialogs` | Native file dialogs (rfd) |

## Main patterns

- **One dispatcher.** Buttons, menus and shortcuts only create `Command` values; the UI runs them
  through `AppState::dispatch` at the end of the frame. `AppState::is_enabled` tells the UI which
  commands are available.
- **Edits on a copy.** An edit is applied to a copy of the `Edit` (timeline, music, subtitles,
  volumes); only if it succeeds is the old copy pushed on the undo stack. A failed edit changes
  nothing.
- **Unsaved changes by comparison.** After every command the current `ProjectDoc` (ready files +
  `Edit`) is compared with the one last saved or opened. Undoing back to the saved state clears
  the marker. New, Open, closing the window and Quit go through `PendingAction`: when there are
  unsaved changes a dialog offers Save / Don't save / Cancel; a failed or cancelled save stops
  the action. The window's close button is intercepted with `ViewportCommand::CancelClose`.
- **Nothing slow on the UI thread.** ffprobe (3 worker threads), proxies (1 thread), subtitle
  generation, the model download, the ffmpeg check and the export run on other threads and
  report through channels that `AppState::tick` drains once per frame. Imported files appear at
  once as "analysing". New/Open cancel the jobs of the old project and replace their channels, so
  late results are dropped.
- **At most three ffmpeg processes.** Every process goes through a `jobs::Limited` runner of
  one `Scheduler` (cap 3). Priority: preview > export > probe > background (proxies, audio
  extraction for subtitles); first come first served inside a level. Playback takes 2 slots at
  once (video + audio). A waiting preview or export asks a running proxy job to stop
  (preemption); the proxy restarts later. Speech recognition itself does not count (it is not
  ffmpeg) and runs one job at a time.
- **Still frames only when the picture changes.** The preview computes a `FrameKey` for the
  playhead (file, source frame, fades, size, visible subtitles). If it equals the frame shown or
  requested, nothing is rendered; otherwise the request waits 80 ms for more edits and replaces
  any older one; an obsolete render is killed.
- **Playback clock.** The wall clock is the master; the audio is padded or trimmed when it drifts
  more than 40 ms, so the video keeps running if the sound stalls.
- **Export plan.** Stream copy only when all clips have identical format, there are no fades,
  music, subtitles or video volume change, and every clip starts at 0 or at a keyframe.
  Otherwise one re-encode: consecutive pieces of the same file read in order share one input
  (`split`/`asplit` + `trim`), each piece is scaled down keeping its aspect ratio, letterboxed,
  converted to the output frame rate and faded; the music bed (silence where there is no music,
  cut at the video end with a 0.5 s fade) is mixed with `amix` + `alimiter`; subtitles are burned
  in with the `subtitles` filter. HEVC in MP4/MOV is tagged `hvc1`. Proxies are never used.
- **Subtitles burn-in without path quoting.** The ASS file and the bundled font are written to a
  private temporary folder that is ffmpeg's working folder, so the filter only uses relative names.
- **Sources are read-only.** Inputs are passed as `file:<absolute path>` with the protocol
  allow-list `file,pipe`; the export is written to `<folder>/.<name>.videolore-partial` and
  renamed on success; an output equal to a source is refused.
- **No shell.** Every process is started with an argument list; children are killed when their
  handle is dropped; on Windows no console window is opened.
- **Network only for the speech model,** after the user agrees: HTTPS only, pinned URL, exact
  size and SHA-256 checked before the file is used. A model file chosen by the user must match too.
- **Errors** are typed with `thiserror` in each module and shown as plain messages; user input
  and project files never cause a panic.

## Conventions

- Every `.rs` file starts with `// SPDX-License-Identifier: GPL-3.0-only` and
  `// Copyright (C) <year> Andrea Lorenzani`.
- `unsafe` appears only in `desktop::macos` (Objective-C calls to register the Apple Event
  handler) and in `stt::whisper` (the cancel callback handed to whisper.cpp: a C function that
  reads the cancel flag through a pointer tied to the flag's lifetime), each block with a
  `SAFETY:` comment. Nowhere else. whisper-rs' `set_abort_callback_safe` must not be used: in
  0.16.0 it reads heap garbage as the cancel flag (see the testing notes).
- whisper.cpp error codes are turned into readable text (`stt::whisper_full_error`), e.g.
  "the audio encoder failed or was interrupted (whisper.cpp code -6)".
- Times are integer microseconds (`Micros`) inside the model, never floating point.
- Settings (`settings.json`): ffmpeg/ffprobe override and the proxy switch. Proxies live in the
  user cache folder (`…/videolore/proxies`), the speech model in the user data folder
  (`…/videolore/models`); projects never store them.
