# Code — modules and patterns

All logic is in the library (`src/lib.rs` and its modules); `src/main.rs` only reads the
project given on the command line, runs the desktop start-up hooks and opens the window. The second program,
videolore player, is the package in `player/` and follows the same rule (see [code-player.md](code-player.md)).

## Module map

| Module | Responsibility |
|---|---|
| `time` | `Micros` (integer microseconds), `Fps` (rational), frame rounding, timecodes |
| `media` | `MediaItem` (with `MediaStatus` analysing/ready/rejected and `ProxyState`), `MediaFormat` (with the text subtitle tracks of the file, `SubtitleTrack`), defensive parser of `ffprobe` JSON, `probe()` |
| `timeline` | The video track: contiguous clips (each with its own volume and mute) + fades at boundaries and at the video's start and end; every edit returns `Result<_, EditError>` |
| `music` | The music track (`MusicTrack`): audio clips at their own times, never overlapping; `Mix` (volume 0–100 % and mute for video sound and music, gain = (p/100)²) |
| `subtitles` | Subtitle lines stored in *source* time per file; `project()` maps them onto the timeline (cut removes, copy duplicates, trim hides, split keeps); edits; the three styles; each line's own position on screen (`Pos`); replacing the lines of one source range |
| `srt` | SubRip: writing the timeline's subtitles (`write`), lenient reading of a track converted by ffmpeg (`parse`, capped), tag removal (`strip_tags`) |
| `align` | Matching the words of a subtitle track to recognised words (per-cue Needleman–Wunsch, close spellings, clamped times, even spread of words not found) |
| `camera` | The virtual camera of 360° video (pure): `Camera`, `Pov`, `CameraPaths` (per source, source time), interpolation, drag/wheel mapping, the `v360` filter and `sendcmd` file text (see [code-360.md](code-360.md)) |
| `text`, `selection`, `fonts`, `spherical` | Text items and their ASS events, the selection of lines and texts, installed fonts, the 360 marker of exported MP4 files (see [code-text.md](code-text.md)) |
| `ass` | Writes the ASS subtitle file: styles, text size `min(0.055·H, 0.09·W)`, word highlight with `\k` tags, escaping of `{ } \` |
| `history` | Undo/redo by snapshots of `Edit` (timeline + music + subtitles + volumes), up to 100 steps, each with the name of its action ("Undo Split") |
| `project` | `.vlproj` file (JSON, `schema_version` 9 since 1.6.1; older files load with defaults and `project/v9.rs` turns format 8 or older into 9, newer ones are refused untouched): atomic save, validated load, `ProjectDoc` for unsaved-changes detection |
| `format` | Same-format check, lowest-quality output choice, codec to encoder mapping with H.264/AAC/MP4 fallback |
| `render` | Export plan (stream copy or one re-encode), filter graphs (clips grouped by source, music bed and mix, subtitle burn-in), `hvc1` tag, export argv (normal method or a hardware method), preview argv, `FrameKey` |
| `ffmpeg` | Locating binaries (setting, environment, bundled, `PATH`), version/filter/encoder checks, `Runner` trait (real processes or test fake), progress and keyframe parsers |
| `jobs` | The process scheduler: a cap of 3–6 ffmpeg/ffprobe processes (changeable while running), priorities, preemption, cancellation; `Limited` runner; small worker `Pool` |
| `export/choice` | The Export dialog's choices and rules: file name, target, clashes, reasons, what to start (see [code-export.md](code-export.md)) |
| `export` | Export worker thread: progress, cancel, subtitle files in a private working folder, partial file and rename; with Export = Faster the hardware attempt, its output check and the silent redo with the normal method (`ExportMethod`) |
| `hwenc` | Video hardware: candidates per system (NVIDIA, Intel Quick Sync, AMD, VA-API, VideoToolbox) in a fixed order, their encoder arguments, the background hardware check |
| `outcheck` | Output check of a hardware file: ffprobe metadata rules, strict decode, frame-by-frame comparison of 3 samples with the normal method's graph (`psnr`) |
| `preview` | Preview player: still frames on demand (debounced, latest only) and playback (RGBA video + PCM audio from two ffmpeg processes), wall-clock master with audio correction |
| `proxy` | Small preview copies of big sources: the machine rule, the cache (keys, size, clear) and the generation job |
| `machine` | Detection of the running machine (`Machine`: logical/physical cores, memory, OS, architecture, CPU features) and its `Fingerprint` stored with the settings |
| `settings` | Program settings file (format 2): the four plain choices (Export added in 0.4), ffmpeg override, machine fingerprint, hardware methods turned off after a problem, section sizes and recent projects (1.1); start-up decision (`startup`), recommendation, "How hard" budget table, availability of subtitle levels, change summary, plain texts of the Settings window |
| `scratch` | Private work folders of background jobs (0700, locked while in use), private partial files (0600), crash cleanup at the next start |
| `stt` | Subtitle generation: audio extraction, 120 s chunks, language detection/override, cancel, word timings to lines; the table of the three speech files (`builtin_models`), the store (size + SHA-256, per level), download and local-file install, disk use and removal; whisper.cpp and the HTTPS downloader behind the `stt` feature |
| `notices` | Bundled components with version, licence and source; the About text |
| `packaging` | Pinned ffmpeg builds and tools (`packaging/*.lock`) and release asset names |
| `desktop` | Linux menu entry / icon / `.vlproj` type for the AppImage and tar.gz; macOS Apple Event handler for double-clicked projects; queue of projects to open |
| `app` | `AppState`, the single command dispatcher (`AppState::dispatch`), background results in `tick()`; `app/command.rs` (`Command` and its undo names), `app/export_flow.rs` (Export dialog context and starting an export), `app/effects.rs` (applying effects, clip volume, edge fades, line selection and positions, SRT export), `app/subs_create.rs` (creating subtitles for a clip or the whole video), `app/sphere.rs` (360 camera, points of view, resize/align/reset), `app/frame.rs`, `app/lut.rs`, `app/texts.rs` |
| `ui` | The window (see [code-ui.md](code-ui.md)): action table (`actions`), menu bar (`menu`), toolbar (`toolbar`), status bar (`status`), resizable sections (`layout`), media list (`media`), timeline with marker lane and music track (`timeline`, `markers`), Effects section and effect popups (`effects`), floating windows (`popups`), Settings body and banner (`settings`), dialogs (`modals`), zoom, shortcuts (`keymap`), ruler labels (`ruler`), subtitle boxes and their measuring (`placement`), box geometry (`boxes`) and everything done on the preview picture (`picture`) |
| `lut` | Apply LUT: the `.cube` parser (untrusted input), the normalised 3D cube and its hash, the parsed-file cache (see [code-lut.md](code-lut.md)) |
| `splash` | The splash screen's pure parts (`Program` with `EDITOR` and `PLAYER`, layout, text and summary fit, colour, GIF decoding); the window is `ui/splash_window.rs`, shared with the player (see [code-splash-icons.md](code-splash-icons.md)) |
| `ui/icons`, `ui/lut_popup`, `ui/splash_window`, `ui/splash_ui` | The icon font and constants, the Apply LUT popup body, the shared splash window and the editor's adapter |
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
- **Nothing slow on the UI thread.** ffprobe (a pool of 6 threads, the process cap decides how many run), proxies (1 thread), subtitle
  generation, the model download, the ffmpeg check and the export run on other threads and
  report through channels that `AppState::tick` drains once per frame. Imported files appear at
  once as "analysing". New/Open cancel the jobs of the old project and replace their channels, so
  late results are dropped.
- **A bounded number of ffmpeg processes.** Every process goes through a `jobs::Limited` runner of
  one `Scheduler` (cap 3 at Light/Balanced, up to 6 at Maximum speed; lowering it never stops a
  running process, new ones wait). Priority: preview > export > probe > background (proxies, audio
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
- **Subtitles burn-in without path quoting.** The ASS file and the bundled font are written to the
  job's private work folder, which is ffmpeg's working folder, so the filter only uses relative
  names. A file name with a control character (line break) is never written to a concat list:
  such projects are re-encoded.
- **Private, crash-safe temporary files.** Each job (export, subtitle generation, preview copy,
  download) gets `<cache>/videolore/work/job-*/` (0700) holding a locked `lock` file and a
  `targets` list of partial files outside it. Partial exports are created 0600 and become 0644
  when renamed. At start-up, folders whose lock is free (the job died) are removed with their
  listed partial files (only known partial-file names, never symlinks); folders locked by
  another running videolore are left alone.
- **Sources are read-only.** Inputs are passed as `file:<absolute path>` with the protocol
  allow-list `file,pipe`; the export is written to `<folder>/.<name>.videolore-partial` and
  renamed on success; an output equal to a source is refused.
- **No shell.** Every process is started with an argument list; children are killed when their
  handle is dropped; on Windows no console window is opened.
- **Network only for the speech files,** each after the user agrees: HTTPS only (checked before
  any connection, redirects included), fixed URL per level, size limit, SHA-256 checked before
  the file is renamed into place and again (once per session) before it is loaded. A file chosen
  by the user must match the level's file too. A full disk is reported as "Not enough free disk
  space".
- **Untrusted files.** Project and settings files are read only when they are regular files,
  through a size cap (16 MiB / 64 KiB; a pipe or device is refused without blocking); names are
  shown with control and text-direction characters replaced; ffprobe gets 30 s per file.
  An unreadable settings file is copied to `settings.json.bad` and replaced by the recommended
  settings.
- **Errors** are typed with `thiserror` in each module and shown as plain messages; user input
  and project files never cause a panic.

## Conventions

- Every `.rs` file starts with `// SPDX-License-Identifier: GPL-3.0-only` and
  `// Copyright (C) <year> Andrea Lorenzani`.
- `#![deny(unsafe_code)]` for the crate; `unsafe` is allowed (`#[allow(unsafe_code)]`) only in `desktop::macos` (Objective-C calls to register the Apple Event
  handler) and in `stt::whisper` (the cancel callback handed to whisper.cpp: a C function that
  reads the cancel flag through a pointer tied to the flag's lifetime), each block with a
  `SAFETY:` comment. Nowhere else. whisper-rs' `set_abort_callback_safe` must not be used: in
  0.16.0 it reads heap garbage as the cancel flag (see the testing notes).
- whisper.cpp error codes are turned into readable text (`stt::whisper_full_error`), e.g.
  "the audio encoder failed or was interrupted (whisper.cpp code -6)".
- Times are integer microseconds (`Micros`) inside the model, never floating point.
- Settings (`settings.json`, format 2): Preview quality, Subtitle quality, How hard videolore
  works, ffmpeg/ffprobe override (absolute paths only) and the machine fingerprint. Proxies and
  work folders live in the user cache folder (`…/videolore/proxies`, `…/videolore/work`, 0700),
  the speech files in the user data folder (`…/videolore/models`); projects never store them.

## Machine, recommendation and "How hard videolore works"

Recommendation (`settings::recommend`, from the stored fingerprint; memory rounded to the
nearest GiB): Preview quality **Automatic**, How hard **Balanced**, Subtitle quality **Most
accurate** with ≥ 12 logical cores and ≥ 15 GiB, **Balanced** with ≥ 4 cores and ≥ 7 GiB, else
**Fast**. Balanced is greyed out below 4 GiB, Most accurate below 7 GiB; all three are greyed
out when the processor cannot run the speech engine. The recommendation never picks a greyed-out
level.

`settings::budget` (L = logical cores, P = physical cores):

| | Light | Balanced (= 0.2.x) | Maximum speed |
|---|---|---|---|
| ffmpeg/ffprobe processes | 3 | 3 | clamp(L/2, 3, 6); 3 below 7 GiB |
| threads of each export / preview copy | max(1, L/2) | ffmpeg decides | ffmpeg decides |
| speech threads | clamp(min(P, L/2), 1, 8) | clamp(min(P, L−1), 1, 8), never below clamp(L/2, 1, 4) | clamp(P, 1, 16) |

New jobs read the budget when they start; running jobs keep theirs. Preview quality decides
which sources get a preview copy (`proxy::wants_proxy`: Automatic = the 0.2 rule, Smoother =
everything above 1280×720, Original = none) and never changes the export.

Speech files (`stt::builtin_models`, pinned whisper.cpp commit, MIT): Fast = `ggml-tiny-q5_1.bin`
(32 MB), Balanced = `ggml-small-q5_1.bin` (190 MB), Most accurate =
`ggml-large-v3-turbo-q5_0.bin` (574 MB). Greedy decoding, one decoding state per job, flash
attention off (measured slower on the CPU).
