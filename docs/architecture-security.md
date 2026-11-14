# Architecture: security

Part of [Architecture](architecture.md).

## Threat model
Treated as untrusted: media files (names and contents), project files, the `ffmpeg` found on `PATH`, downloaded files (the `ffmpeg` builds used at packaging time, the speech model), text typed into subtitles, and file paths passed by the operating system.

There are no secrets, accounts or telemetry.

## Running ffmpeg
- `ffmpeg` and `ffprobe` are started directly, never through a shell. File names are passed as separate arguments prefixed with `file:`, and only the `file` and `pipe` protocols are allowed, so crafted names or files cannot cause option injection or network access.
- The working folder of `ffmpeg` is always a private temporary folder created by `videolore` (used for subtitles).
- The bundled `ffmpeg` parses untrusted media, so its pinned build is refreshed for every release and verified by SHA-256 when packaging.
- `PATH` is searched last; settings show the path, version and origin of the `ffmpeg` in use.
- It runs with the user's rights, never elevated.

## Project files
- Strict typed parsing, 16 MiB size limit, and full validation after loading (clip ranges, music clips, subtitle lines, word timings, at most 200,000 words).
- Files from a newer version are refused and never written.
- Paths in a project are used only as `ffmpeg` inputs.

## Subtitles
- Subtitle text is escaped so it cannot add styling or override tags; only `videolore` emits line breaks and highlight tags. Text is limited to 1000 characters per line.

## Network
- The only network access is the optional speech model download, after explicit consent. It uses HTTPS only (also for redirects), a fixed URL revision, and exact size and SHA-256 checks before the file is used.
- A model file supplied by the user must match the same size and SHA-256 before it is loaded (whisper.cpp is C++ code parsing that file).
- The HTTP client exists only in builds with speech-to-text and is used only by the downloader.

## Files written outside projects
- Proxy cache: file names are hashes, not source paths; "Clear proxy cache" deletes only files matching the proxy name pattern inside the cache folder.
- Linux menu entry: written only under `~/.local/share`; the launch command is quoted as the desktop-entry specification requires and points to an existing absolute path; the helper tools are run with fixed arguments, no shell, and their errors are ignored.
- macOS "open document" paths go through the same open and validation steps as a file picked by the user.
- Exports go to a temporary file renamed on success; source files are never written.

## Unsigned installers
In 0.2.0 the installers are not code-signed (decided by Andrea), so Windows and macOS show a warning on first launch; the README explains the steps. The macOS app is sealed with an ad-hoc signature only. Signing is postponed, not rejected.

## Supply chain
- `Cargo.lock` is committed and release builds use `--locked`.
- `cargo-deny` runs in CI: licence allow-list (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unlicense, CDLA-Permissive-2.0, BSL-1.0, MPL-2.0), security advisories, banned crates.
- `cargo-about` generates the notices of all Rust crates (`packaging/notices/rust-crates.txt`); CI regenerates it and fails if the committed copy is out of date.
- The `ffmpeg` archives and `appimagetool` are pinned by SHA-256 (`packaging/ffmpeg.lock`, `packaging/tools.lock`).
- Dependencies and CI actions are kept on their latest releases, so security fixes arrive quickly.
