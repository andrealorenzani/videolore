# Architecture: security

Part of [Architecture](architecture.md). Dependencies and supply chain: [dependencies](architecture-dependencies.md). Build and CI gates: [build](architecture-build.md).

## Threat model
- **Untrusted:** media files (names and contents, including subtitle tracks inside them); project files; the settings file (parsed defensively; whoever can change it already runs programs as that user); the `ffmpeg` found on `PATH`; downloads (ffmpeg builds at packaging time, speech models); typed subtitle and text; paths from the operating system (double-click, dropped files, recent projects, macOS "open document"); LUT files (`.cube`, `.3dl`, `.csp`); overlay pictures and videos (2.3.0).
- **Trusted build inputs:** the bundled splash pictures and icon font (compiled in; caps guard a bad replacement), the bundled ffmpeg (pinned by SHA-256, refreshed every release), system fonts (as any system font).
- **Assets:** source media (never written), the project file, the export output, the settings. No secrets, accounts or telemetry.

## Running ffmpeg
- `ffmpeg` and `ffprobe` start directly, never through a shell; arguments are lists. File names are passed as separate arguments prefixed `file:`; only the `file` and `pipe` protocols are allowed, so crafted names cannot inject options or reach the network.
- The working folder is private. No user text enters a filter: numbers, closed lists and `#RRGGBB` colours only; long graphs go through a script file.
- `PATH` is searched last. A custom path must be absolute and an existing regular file, else ignored with a notice. Settings → Advanced shows path, version and origin.
- `ffprobe` stops after 30 s ("could not be analysed in time", 0.3.0). File names with control characters never enter the concat list; such exports re-encode with each file a separate argument (0.3.0). Runs with the user's rights, never elevated.
- **Video hardware (0.4.0):** the check encodes synthetic frames and reads no files; the VA-API device is a character device named `renderD<number>`; hardware output is untrusted media, decoded by software when checked; remembered failures are validated field by field ([export](architecture-export.md#security)).
- **Preview (2.0.4, 2.3.0):** hardware decoding only where allowed; overlay files are never hardware-decoded.

## Project and settings files
- Only regular files are read (a pipe or device named like a project cannot block). Caps: projects 16 MiB (96 MiB from format 11, for LUT copies, ADR-191); settings 64 KiB; enforced while reading.
- Strict typed parsing and full validation after load: clip ranges, music clips, subtitle lines, word timings (at most 200,000 words); nesting depth limited. Projects of a newer version are refused and never written; a damaged project is refused with a message.
- A damaged settings file is replaced by recommended settings and kept as `settings.json.bad`.
- Project paths are used only as ffmpeg inputs (read only); relative paths with `..` resolve against the project folder.
- Names on screen have control and text-direction override characters replaced; the full path only in a tooltip.
- **Recent projects and layout (1.1.0):** absolute paths of at most 1024 bytes without control characters, at most 10, no duplicates; non-UTF-8 paths never stored; opened only on click through the normal loading path; a missing file is reported and removed. Proportions must be finite numbers between 0 and 1.
- **Per-version validation:** format 3 clip volume above 100 % or a subtitle position outside the picture → damaged; unknown clips or lines refused (1.2.0). Format 4 POV and line-size limits → damaged; angles clamped (1.2.6). Format 5 ranges, colours, font names, event caps → damaged (1.3.0). Format 6 text width and anchor limits (1.4.0). Format 9 conversion from 8: counts capped at 200 per clip, damaged files refused (1.6.1). Format 12 order ids, shadows, enlarge sizes, colours (2.1.0). Format 13 text maximum (2.2.0). Format 14 overlaps: damaged ones dropped with a count (2.3.0). Speed 500 to 2000 thousandths and volume keys 0 to 100 % (2.0.4).
- **Audio tracks (3.1.0 planned, ADR-245):** the track number is a checked integer 0 to 31 written into argv as `[n:a:k]` or `0:a:k`; no text from a project or from ffprobe reaches argv. Audio track lists from ffprobe are capped (32 entries), titles sanitised and at most 64 characters, languages through the subtitle language check.
- **As subtitle track (3.1.0 planned, ADR-249):** track text goes through the SRT writer (escaping and control characters as for the other tracks).
- Hostile-file tests (invalid, truncated, huge or negative numbers, `..` and absolute paths, control characters, oversized files, a pipe in place of a project, 10,000 tracks, 1 MB titles, damaged SRT) check: no crash, a clear refusal, nothing written, sources unchanged.

## Subtitles, text, fonts
- Subtitle text is escaped; only videolore writes line breaks, highlight tags and position tags (numbers checked inside the picture).
- **Tracks (1.2.0):** ffmpeg converts the track to a fixed file name in the private folder; videolore reads it as a regular file of at most 16 MiB with a parser that never panics; times checked; at most 100,000 cues; formatting and control characters removed; language pattern; title at most 64 characters; at most 16 tracks; chosen by a number counted by videolore.
- **SRT import and export (1.2.0, 2.0.4):** regular files, UTF-8 (BOM accepted), size and cue caps; written only to the chosen path, through a temporary file and rename.
- **Text items and fonts (1.3.0, 2.0.3):** text escaped; font names at most 100 characters of letters, digits, spaces and `- . _ ( )`; colours `#RRGGBB`; ASS events capped (2000 per item, 20000 per project). Installed fonts are read only from system font folders (regular files, at most 64 MiB, background thread). A project stores the font name; the font file is copied into the private folder with a checksum prefix and removed with it; a missing font is never read from a project path. `skrifa` and `harfrust` are memory-safe Rust; a broken font gives no measurement.
- 360 anchors are validated on load and formatted by videolore before they reach the graph.

## Network
- The only network use is the speech model download of the chosen level, after consent each time a model is needed. No telemetry, no update check. Using a video's own subtitles without matching needs no network.
- Addresses come only from a fixed table (a test checks all are HTTPS); non-HTTPS refused before connecting; redirects only to HTTPS, at most 5.
- The download is limited to the expected size and checked against the known SHA-256 before use; a partial or wrong file is deleted. A user-supplied model must match size and SHA-256 before loading (whisper.cpp parses that file); size checked before each use, SHA-256 once per session.
- The HTTP client exists only in `stt` builds and only the downloader uses it (test). The player's graph has no `ureq` or `whisper-rs` (CI).

## Temporary and other files
- Temporary files (extracted audio, subtitle files, filter scripts, LUT work copies, spatial WAV, background RGBA) live in a private work folder under the cache (`…/videolore/work/`), mode 0700 on Linux and macOS (0.3.0), not in the shared temporary folder.
- Partial exports beside the target are 0600 and become 0644 only when complete and renamed; partial previews, model downloads and hardware attempts (`.<name>.hw.videolore-partial`) are tracked the same way; everything is removed when a job ends, fails or is cancelled.
- **After a crash:** each work folder is locked while its job runs. At the next start an unlocked folder belongs to a dead session: tracked partials are deleted only if they match videolore's naming patterns, are regular files and not symbolic links; then the folder is deleted. Locked folders (another window) are left alone; on Windows a file in use is skipped and retried later. Cache and data folders are user-only on Linux and macOS.
- Preview copies have hashed names; clearing deletes only files matching the preview-copy pattern. Speech removal deletes only the known model files and their partials.
- Linux menu entry (1.1.0): written only under `~/.local/share`; the launch command quoted as the desktop-entry specification requires; helper tools run with fixed arguments, no shell, errors ignored.
- Dropped files pass the same checks as File > Import; a dropped project goes through File > Open's loading after the unsaved-changes prompt.
- Exports go to a temporary file renamed on success; source files are never written (tests hash the sources).
- **Export dialog (1.4.0):** the name is checked (separators, control characters, reserved Windows names, length), the extension from a fixed table, the folder must exist; a target that is a project media file or the project file is refused; an existing file is replaced only after confirmation. No user text reaches ffmpeg (presets, codecs and clean-audio values are constants).
- **360 marker (1.3.0, 2.0.4):** written into the export's own MP4 or MOV (only the small movie box is read, under a size limit, every offset checked; the rest copied unchanged), or into the MKV or WebM track in place; the content is a fixed XML text or fixed element values.

## Frame, LUT, player, splash, overlays, pictures
- **Frame (1.5.0):** values from closed lists and numeric ranges; frame size from the base size and the shape, capped at 4096 px on the longer side; out-of-range values corrected on load with a note.
- **LUT (1.6.0, 2.1.0):** untrusted: a regular file opened without following FIFOs or devices (refused on the open handle); UTF-8 only; strict tokens and counts; finite values clamped to 0..1; property tests (no panic); streamed with a 512 MiB cap and 256-byte lines (2.1.0); never written; the checked table goes to the work folder under a name from its content hash; a file changing while read is refused; a missing or broken file stops the export before any output.
- **Player (1.6.0):** untrusted media through the same process model (child processes, job limit, private folders); never writes next to the video; never reads a `.vlproj` (refused by extension); no network code. Its settings file is in the user's configuration folder (2.0.4; capped; corpus test).
- **Splash (1.6.0, 1.6.2):** images and summaries compiled in, no file read at run time; the summary is plain text (no markup, no links); the picture is decoded with caps (64 frames, 4096 px per side, 64 MiB RGBA). Origin and licence (author's own work, MIT) in the notices (1.6.3).
- **Overlay media (2.3.0):** size read from a bounded PNG or GIF header, or ffprobe, before decoding; over 16 megapixels refused (REQ-316); a MOV is trusted by its size when added and checked at use; paths never enter a filter text; not copied into the project.
- **Picture items (2.0.4):** a PNG read only for its 24-byte header (each side at most 16384 px; path at most 4096 bytes); ffmpeg decodes it; not embedded, not written.
- **Spatial sound (2.0.4):** the WAV under a fixed prefix and a hashed name in the private folder (`.part`, then rename); a clip's own file is read, never a preview copy; cancel and failure leave nothing; four channels assumed, not checked.
- **Speech fixture:** committed and checked by SHA-256; no synthesiser runs in tests; `VIDEOLORE_REQUIRE_STT=1` makes a missing model a failure (2.0.3).

## Unsafe code
Rust `unsafe` is forbidden in the whole program (compiler-enforced since 0.3.0) except three reviewed places: the cancel callback given to whisper.cpp (`stt.rs`); `desktop.rs` for the macOS "open document" handler; and since 2.0.4 (ADR-183, amends ADR-034) the one allow-listed call that builds the macOS menu bar.

## Signing and supply chain
- Installers are not code-signed (decided by Andrea; postponed, not rejected): Windows and macOS warn on first launch; the README explains the steps; the macOS app carries an ad-hoc seal only ([packaging](architecture-packaging.md#macos)).
- `Cargo.lock` committed; release builds use `--locked`. `cargo-deny` in CI: licence allow-list (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unlicense, CDLA-Permissive-2.0, BSL-1.0, MPL-2.0), advisories, yanked versions (0.3.0), banned crates; an advisory is ignored only with a written reason (the list is empty); a weekly advisory run.
- `cargo-about` regenerates `packaging/notices/rust-crates.txt`; CI fails when it is out of date. ffmpeg archives and `appimagetool` are pinned by SHA-256 (`packaging/ffmpeg.lock`, `packaging/tools.lock`). CI jobs are read-only; only the release job writes; third-party actions pinned to commits (ADR-036).
- Assumption: the GPL source-offer approach for ffmpeg is not reviewed by a lawyer ([engine](architecture-engine.md#bundled-ffmpeg)).

## Security by version
0.2.x: no shell, `file:` prefix, protocol whitelist, bounded reads (ADR-034). 0.3.0: private work folders 0700, unsafe in three sites, pinned downloads, FIFO refusal. 0.4.0: hardware check with synthetic input, renderD devices only. 1.2.0: subtitle track and SRT validation. 1.4.0: export name rules and data-loss guard. 1.6.0: LUT input and player rules. 2.0.4: pictures, speed and volume keys, speech fixture, menu call. 2.1.0: LUT streaming, order validation. 2.3.0: overlay media limits, format 14 validation.
