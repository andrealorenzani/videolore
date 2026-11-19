# Architecture: security

Part of [Architecture](architecture.md).

## Threat model
Treated as untrusted: media files (names and contents, including subtitle tracks inside them since 1.2.0), project files, the contents of the settings file, the `ffmpeg` found on `PATH`, downloaded files (the `ffmpeg` builds used at packaging time, the speech models), text typed into subtitles, and file paths passed by the operating system (double-click, and since 1.1.0 files dropped on the window and the recent-projects list).

The settings file is parsed defensively, but it is trusted at the level of the user account: anyone who can change it (for example to point to another `ffmpeg`) can already run programs as that user.

There are no secrets, accounts or telemetry.

## Running ffmpeg
- `ffmpeg` and `ffprobe` are started directly, never through a shell. File names are passed as separate arguments prefixed with `file:`, and only the `file` and `pipe` protocols are allowed, so crafted names or files cannot cause option injection or network access.
- The working folder of `ffmpeg` is always a private work folder created by `videolore` (used for subtitles).
- The bundled `ffmpeg` parses untrusted media, so its pinned build is refreshed for every release and verified by SHA-256 when packaging.
- `PATH` is searched last; Settings → Advanced shows the path, version and origin of the `ffmpeg` in use. A custom `ffmpeg` path must be absolute and point to an existing regular file, otherwise it is ignored with a notice.
- Analysing a file (`ffprobe`) is stopped after 30 s with "could not be analysed in time" (since 0.3.0), so a hostile file cannot keep it busy indefinitely.
- File names containing control characters (such as a line break) are never written into `ffmpeg`'s list of files to join; such exports use the re-encode path, where each file is a separate argument (since 0.3.0).
- It runs with the user's rights, never elevated.
- **Video hardware (0.4.0):** the hardware check encodes synthetic frames and reads no files. The VA-API device is chosen only among system device files named `renderD<number>` that are character devices. A hardware export is treated as untrusted media and decoded only by the software decoder when checked. Remembered hardware failures in the settings file are validated field by field. A driver crash or hang stays in the `ffmpeg` process, which is stopped; a hang of the whole system inside a driver cannot be prevented. See [hardware export](architecture-hardware-export.md#security).

## Project and settings files
- Only regular files are read (a pipe or device named like a project can no longer block the program), with a hard size limit: 16 MiB for projects, 64 KiB for settings. The limit is enforced while reading, so a file growing during the read cannot exceed it.
- Strict typed parsing and full validation after loading (clip ranges, music clips, subtitle lines, word timings, at most 200,000 words). Deeply nested data is rejected by the parser's depth limit.
- Projects from a newer version are refused and never written. A damaged project is refused with a clear message.
- A damaged settings file is replaced by recommended settings with a notice; the bad copy is kept as `settings.json.bad`.
- Paths in a project are used only as `ffmpeg` inputs, which are only read. Relative paths with `..` are resolved against the project folder; this is accepted because they are never written.
- Names shown on screen have control characters and text-direction override characters replaced, so a crafted name cannot disguise itself.
- **Layout and recent projects (1.1.0):** section proportions must be finite numbers between 0 and 1; anything else falls back to the default without a notice and the other settings are kept. Recent projects are checked one by one: only absolute paths of at most 1024 bytes without control characters, at most 10, duplicates removed. Paths that are not valid UTF-8 are never stored, so they cannot break later saves. Even with every field at its maximum the settings file stays well under the 64 KiB limit.
- **Project version 3 (1.2.0):** a clip volume above 100% or a subtitle position outside the picture makes the project refused as damaged; references to unknown clips or lines are refused; subtitle track descriptions are cleaned instead (see [project files](architecture-projects.md#loading-and-validation)).
- Automated tests feed hostile files (invalid or truncated data, huge or negative numbers, `..` and absolute paths, control characters, oversized files, a pipe in place of a project; since 1.2.0 also wrong values in the new project fields, 10,000 subtitle tracks, 1 MB track titles and damaged SRT data) and check: no crash, a clear refusal, nothing written, source files unchanged.

## Subtitles
- Subtitle text is escaped so it cannot add styling or override tags; only `videolore` emits line breaks, highlight tags and (since 1.2.0) position tags, the latter only from numbers checked to lie inside the picture. Text is limited to 1000 characters per line.
- **Subtitle tracks inside videos (1.2.0):** the track is converted by `ffmpeg` (a separate process) into a fixed file name inside the private work folder; `videolore` reads it only as a regular file of at most 16 MiB, with a parser that never panics, checks every time value and keeps at most 100,000 cues. Formatting tags and control characters are removed and the usual escaping still applies before burn-in. Track language and title from `ffprobe` are checked (language code pattern, title cleaned and at most 64 characters, at most 16 tracks); tracks are selected by a number counted by `videolore`, so nothing from the file reaches `ffmpeg`'s arguments. See [subtitle tracks](architecture-subtitle-tracks.md).
- **SRT export (1.2.0):** written only to the path chosen in the save dialog, through a temporary file and a rename, with text already cleaned.

## Network
- The only network access is the speech model download of the chosen quality level, after explicit consent each time a model is needed. There is no telemetry and no update check. Using a video's own subtitles without matching words (1.2.0) needs no network at all.
- Addresses come only from a fixed built-in table (an automated test checks that all are HTTPS); anything that is not HTTPS is refused before connecting; redirects are followed only to HTTPS, at most 5.
- The download is limited to the expected size and checked against the known SHA-256 before it is used; a partial or wrong file is deleted.
- A model file supplied by the user must match the same size and SHA-256 before it is loaded (whisper.cpp is C++ code parsing that file). Before each use the size is checked again, and the SHA-256 once per session.
- The HTTP client exists only in builds with speech-to-text and is used only by the downloader (checked by an automated test).

## Temporary files (since 0.3.0)
- Temporary files (extracted audio, subtitle files, filter scripts) live in a private work folder under `videolore`'s cache folder (`…/videolore/work/`), readable only by the user (mode 0700 on Linux and macOS), instead of the shared system temporary folder. This also stops long extracted audio from filling a memory-backed `/tmp`.
- Partial exports in the destination folder are created as user-only files (0600) and get normal permissions (0644) only when complete and renamed. Partial preview copies and partial model downloads are tracked the same way, and so is the partial file of a hardware export attempt (0.4.0, `.<name>.hw.videolore-partial`).
- Everything is removed when the job finishes, fails or is cancelled.
- **After a crash:** each work folder holds a lock while its job runs. At the next start, folders whose lock is free belong to a dead session; their tracked partial files are deleted, but only if they match `videolore`'s own naming patterns, are regular files and are not symbolic links, and then the folder itself is deleted. Folders still locked (another `videolore` window) are left alone. On Windows a file still in use is skipped and retried next time.
- The cache and data folders are created user-only on Linux and macOS.

## Other files written outside projects
- Preview copy cache: file names are hashes, not source paths; clearing deletes only files matching the preview-copy name pattern inside the cache folder.
- Speech files: removal deletes only the known model file names and their partial downloads.
- Linux menu entry: written only under `~/.local/share`; the launch command is quoted as the desktop-entry specification requires and points to an existing absolute path; the helper tools are run with fixed arguments, no shell, and their errors are ignored.
- macOS "open document" paths go through the same open and validation steps as a file picked by the user.
- **Dropped files (1.1.0):** dropped media go through exactly the same checks as File > Import (analysis in the background with its time limit); a dropped project goes through the same loading and validation as File > Open, after the unsaved-changes prompt. Drops are ignored while a dialog needs an answer.
- **Recent projects (1.1.0):** names are shown with control and direction-override characters replaced, the full path appears only in a tooltip, and a project is opened only when the user clicks it, through the normal loading path. A missing file is reported and removed. The list stores full project paths in the user's own settings folder; File > Open recent > Clear list removes them.
- Exports go to a temporary file renamed on success; source files are never written.

## Text items and fonts (1.3.0)
- Text and font names come from the project file, so they are untrusted: text goes through the subtitle escaping (no styling can be injected), a font name is limited to 100 characters from letters, digits, spaces and `- . _ ( )`, and a colour must be `#RRGGBB`.
- Blink and other effects are capped in number of ASS events (2,000 per item, 20,000 per project), so a hostile project cannot make `ffmpeg` work without end.
- Installed fonts are read only from the system font folders, as regular files of at most 64 MiB, on a background thread. System fonts are parsed by the font code of the operating system and of `ffmpeg` (trusted, as for any system font); a broken system font is outside the project's control.
- Anchors on the sphere are numbers validated on load and formatted by `videolore` before they reach the filter graph.

## Spherical marker (1.3.0)
- The 360 marker is written into the export's own MP4 or MOV file, never into a source file. Only the small movie box is read into memory, under a size limit; every size and offset is checked, and the code does not panic on any input. The rest of the file is copied unchanged.
- The marker is a fixed XML text written by `videolore`; nothing from the project or the media file reaches it.

## Export dialog and clean audio (1.4.0)
- The export file name and folder are typed or chosen by the user: the name is checked (no path separators, control characters, reserved Windows names, length limit; extension from a fixed table), the folder must be an existing directory, and the target is built from the validated name only.
- A target that is a media file of the project or the project file is refused (data-loss guard). An existing file is replaced only after confirmation; the output is written beside the target and renamed when complete.
- No user text reaches `ffmpeg`: presets, codecs and the clean audio values are constants; MP3 is offered only when the bundled `ffmpeg` lists its encoder.
- The dialog's choices are kept in memory for the session only; nothing is written to disk or sent over the network.
- Text widths and 360 anchor sizes are range-checked on load like the other values of the project.

## Frame effects (1.5.0)
- Change aspect, Scale and Move take their values from closed lists and numeric ranges only; the colour of the frame is a strict `#RRGGBB` value. No project text reaches an `ffmpeg` filter graph: the expressions are built from numbers.
- Frame sizes come from the base size and the chosen shape, capped at 4096 px on the longer side; numbers stored in a project cannot make the frame larger.
- Values outside their ranges in a project file are corrected on load with a note, and never crash the program.

## Apply LUT, player and assets (1.6.0, planned)
- A `.cube` file is untrusted input: read as a regular file, at most 16 MiB, UTF-8 only, strict tokens and counts, finite values only; never a panic (property tests). The checked cube is written into the private work folder under fixed names, and no user text enters a filter. A missing or broken file stops an export before any output is written. See [LUT](architecture-lut.md).
- The player plays untrusted media through the same `ffmpeg` process model as the editor (child processes, job limit, private folders). It never writes next to the video, never reads a `.vlproj` (refused by extension), and has no network code: CI checks that its dependency graph has no `ureq` or `whisper-rs`. See [player](architecture-player.md).
- The splash image, its summary and the icon font are compiled in: no file is read at run time and no network is used. The GIF decoder reads only the bundled file. See [splash](architecture-splash.md) and [icons](architecture-icons.md).

## Unsafe code
Rust's `unsafe` is forbidden in the whole program (enforced by the compiler since 0.3.0) except in two reviewed places: the cancel callback passed to whisper.cpp and the macOS "open document" handler.

## Unsigned installers
The installers are not code-signed (decided by Andrea), so Windows and macOS show a warning on first launch; the README explains the steps. The macOS app is sealed with an ad-hoc signature only. Signing is postponed, not rejected.

## Supply chain
- `Cargo.lock` is committed and release builds use `--locked`.
- `cargo-deny` runs in CI: licence allow-list (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unlicense, CDLA-Permissive-2.0, BSL-1.0, MPL-2.0), security advisories, withdrawn ("yanked") versions (since 0.3.0), banned crates. A known advisory may only be ignored with a written reason why it does not affect `videolore`; the ignore list is empty. A weekly scheduled advisory check is optional.
- `cargo-about` generates the notices of all Rust crates (`packaging/notices/rust-crates.txt`); CI regenerates it and fails if the committed copy is out of date.
- The `ffmpeg` archives and `appimagetool` are pinned by SHA-256 (`packaging/ffmpeg.lock`, `packaging/tools.lock`). For 0.3.0 the Linux and Windows `ffmpeg` builds were refreshed to the newest 9.0.2 build (see [packaging](architecture-packaging.md#bundled-ffmpeg-refresh-030)).
- CI jobs get read-only repository access; only the release job may write. Every third-party CI action is pinned to the full commit of a release (see [packaging](architecture-packaging.md#ci-runners-and-actions)).
- Dependencies and CI actions are kept on their latest releases, so security fixes arrive quickly.
