# Code — patterns and conventions

Back to the [index](code.md). All logic is in the library (`src/lib.rs` and its modules); `src/main.rs` only reads the project given on the command line, runs the desktop start-up hooks and opens the window; the player (`player/`) follows the same rule ([code-player.md](code-player.md)). The module map: `scripts/code-map.sh` (one line per file with its doc line), [code-index.md](code-index.md) (by feature) and [code-architecture.md](code-architecture.md) (layers and diagrams).

## Main patterns
- **One dispatcher.** Buttons, menus and shortcuts only create `Command` values; the UI runs them through `AppState::dispatch` at the end of the frame (the areas of `app/dispatch_<area>.rs`, see [code.md](code.md) § How to add an effect or a command). `AppState::is_enabled` / `disabled_reason` (`app/enabled.rs`) tell the UI what is available.
- **Edits on a copy.** An edit runs on a copy of the `Edit` (timeline, music, subtitles, volumes); only if it succeeds is the old copy pushed on the undo stack (`AppState::edit`, which also asks before an edit loses overlaps). A failed edit changes nothing.
- **Unsaved changes by comparison.** After every command the current `ProjectDoc` (ready files + `Edit`) is compared with the one last saved or opened; undoing back clears the marker. New, Open, close and Quit go through `PendingAction` (Save / Don't save / Cancel; a failed or cancelled save stops the action). The close button is intercepted with `ViewportCommand::CancelClose`.
- **Nothing slow on the UI thread.** ffprobe (a pool of 6 threads, the process cap decides how many run), proxies (1 thread), subtitle generation, the model download, the ffmpeg check, LUT loads and the export run on other threads and report through channels that `AppState::tick` drains once per frame. Imported files appear at once as "analysing". New/Open cancel the old project's jobs and replace their channels, so late results are dropped.
- **A bounded number of ffmpeg processes.** Every process goes through a `jobs::Limited` runner of one `Scheduler` (cap 3 at Light/Balanced, up to 6 at Maximum speed; lowering it never stops a running process). Priority: preview > export > hardware check > probe > background (proxies, audio extraction); first come first served inside a level. Playback takes 2 slots (video + audio). A waiting preview or export asks a running proxy job to stop (it restarts later). Speech recognition is not ffmpeg and runs one job at a time.
- **Still frames only when the picture changes.** The preview computes a `FrameKey` for the playhead (file, source frame, fades, size, visible subtitles, order, overlaps…); equal to the frame shown or requested: nothing is rendered; otherwise the request waits 80 ms for more edits and replaces any older one; an obsolete render is killed.
- **Playback clock.** The wall clock is the master; the audio is padded or trimmed when it drifts more than 40 ms.
- **Export plan.** Stream copy only when all clips have identical format, no fades, music, subtitles, volume change or speed, and every clip starts at 0 or at a keyframe. Otherwise one re-encode: consecutive pieces of one file share one input (`split`/`asplit` + `trim`), each piece is scaled keeping its ratio, letterboxed, converted to the output rate and faded; the music bed (silence where there is none, cut at the video end with a 0.5 s fade) is mixed with `amix` + `alimiter`; subtitles are burned with the `subtitles` filter. HEVC in MP4/MOV is tagged `hvc1`. Proxies are never used.
- **Burn-in without path quoting.** The ASS file and the fonts are written to the job's private work folder, ffmpeg's working folder, so filters use relative names. A file name with a control character is never written to a concat list (such projects are re-encoded).
- **Private, crash-safe temporary files.** Each job gets `<cache>/videolore/work/job-*/` (0700) with a locked `lock` file and a `targets` list of partial files outside it. Partial exports are 0600, 0644 when renamed. At start-up folders whose lock is free are removed with their listed partial files (known names only, never symlinks); folders of another running videolore are left alone.
- **Sources are read-only.** Inputs are `file:<absolute path>` with the protocol allow-list `file,pipe`; the export is written to `<folder>/.<name>.videolore-partial` and renamed on success; an output equal to a source is refused.
- **No shell.** Every process is started with an argument list; children are killed when their handle is dropped; no console window on Windows.
- **Network only for the speech files,** each after the user agrees: HTTPS only (checked before any connection, redirects included), fixed URL per level, size limit, SHA-256 checked before the rename and again (once per session) before loading. A file chosen by the user must match too. A full disk is "Not enough free disk space".
- **Untrusted files.** Project and settings files are read only when regular, through a size cap (16 MiB / 64 KiB; a pipe or device is refused without blocking); names are shown with control and bidi characters replaced; ffprobe gets 30 s per file. An unreadable settings file is copied to `settings.json.bad` and replaced by the recommended settings.
- **Errors** are typed with `thiserror` per module and shown as plain messages; user input and project files never cause a panic.
- **Tables over matches.** Facts that are data live in one table: `ui::actions::ACTIONS` (label, shortcuts, tooltip), `app::effect_kind::SPECS` (effect name, undo name, popup, frame flag), `effect_order::NAMED` (row names in the file), `format::VIDEO_PRESETS`, `ui::strings` (texts).

## Conventions
- Every `.rs` file starts with `// SPDX-License-Identifier: GPL-3.0-only` and `// Copyright (C) <year> Andrea Lorenzani`.
- `#![deny(unsafe_code)]`; `unsafe` is allowed only in `desktop::macos` (Objective-C calls: Apple Events, the menu bar) and `stt::whisper` (the cancel callback: a C function reading the flag through a pointer tied to its lifetime), each block with a `SAFETY:` comment. whisper-rs' `set_abort_callback_safe` must not be used: in 0.16.0 it reads heap garbage as the cancel flag. whisper.cpp error codes become text (`stt::whisper_full_error`, e.g. "the audio encoder failed or was interrupted (whisper.cpp code -6)").
- Times are integer microseconds (`Micros`) in the model, never floating point.
- Child modules of `app/` see `app.rs`'s imports through `use super::*`; methods they define for `app.rs` need `pub(super)`.
- Settings (`settings.json`, format 2): Preview quality, Subtitle quality, How hard videolore works, ffmpeg/ffprobe override (absolute paths only), the machine fingerprint, the interface. Proxies and work folders are in the user cache folder (`…/videolore/proxies`, `…/videolore/work`, 0700), speech files in the user data folder (`…/videolore/models`); projects never store them.

## Machine, recommendation and "How hard videolore works"
Recommendation (`settings::recommend`, from the fingerprint; memory rounded to the nearest GiB): Preview quality **Automatic**, How hard **Balanced**, Subtitle quality **Most accurate** with ≥ 12 logical cores and ≥ 15 GiB, **Balanced** with ≥ 4 cores and ≥ 7 GiB, else **Fast**. Balanced is greyed out below 4 GiB, Most accurate below 7 GiB; all three when the processor cannot run the speech engine; the recommendation never picks a greyed-out level.

`settings::budget` (L = logical cores, P = physical cores):

| | Light | Balanced (= 0.2.x) | Maximum speed |
|---|---|---|---|
| ffmpeg/ffprobe processes | 3 | 3 | clamp(L/2, 3, 6); 3 below 7 GiB |
| threads of each export / preview copy | max(1, L/2) | ffmpeg decides | ffmpeg decides |
| speech threads | clamp(min(P, L/2), 1, 8) | clamp(min(P, L−1), 1, 8), never below clamp(L/2, 1, 4) | clamp(P, 1, 16) |

New jobs read the budget when they start; running jobs keep theirs. Preview quality decides which sources get a preview copy (`proxy::wants_proxy`: Automatic = the 0.2 rule, Smoother = everything above 1280×720, Original = none) and never changes the export.

Speech files (`stt::builtin_models`, pinned whisper.cpp commit, MIT): Fast = `ggml-tiny-q5_1.bin` (32 MB), Balanced = `ggml-small-q5_1.bin` (190 MB), Most accurate = `ggml-large-v3-turbo-q5_0.bin` (574 MB). Greedy decoding, one decoding state per job, flash attention off (measured slower on the CPU).
