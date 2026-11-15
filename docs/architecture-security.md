# Architecture: security

Part of [Architecture](architecture.md).

## Threat model
Treated as untrusted: media files (names and contents), project files, the contents of the settings file, the `ffmpeg` found on `PATH`, downloaded files (the `ffmpeg` builds used at packaging time, the speech models), text typed into subtitles, and file paths passed by the operating system (double-click, and since 1.1.0 files dropped on the window and the recent-projects list).

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
- Automated tests feed hostile files (invalid or truncated data, huge or negative numbers, `..` and absolute paths, control characters, oversized files, a pipe in place of a project) and check: no crash, a clear refusal, nothing written, source files unchanged.

## Subtitles
- Subtitle text is escaped so it cannot add styling or override tags; only `videolore` emits line breaks and highlight tags. Text is limited to 1000 characters per line.

## Network
- The only network access is the speech model download of the chosen quality level, after explicit consent each time a model is needed. There is no telemetry and no update check.
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
