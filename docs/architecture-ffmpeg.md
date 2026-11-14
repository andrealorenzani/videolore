# Architecture: bundled ffmpeg

Part of [Architecture](architecture.md).

## Why ffmpeg is bundled
Since 0.2.0 import, preview and export work right after installation, with no `ffmpeg` on the machine (REQ-030). Earlier versions required the user to install `ffmpeg` 4.4 or newer; that requirement is gone. `videolore` still runs `ffmpeg` and `ffprobe` as **separate programs** (not linked into the executable), which keeps the build simple and the licences separate.

## Which builds
All three platforms use full-featured (GPL) builds from the same FFmpeg release line (9.0.x at the time of the decision; confirmed when the builds are pinned).

| Platform | Build | Notes |
|---|---|---|
| Windows x86_64 | BtbN/FFmpeg-Builds `win64-gpl-shared`, release branch | `ffmpeg.exe` and `ffprobe.exe` share their DLLs, about half the size of two standalone programs. |
| Linux x86_64 | BtbN/FFmpeg-Builds `linux64-gpl`, release branch | Standalone (static). |
| macOS Apple Silicon | Martin Riedl build server, "release" static arm64 builds | Each build has a published SHA-256. |

Alternatives rejected:
- LGPL builds: they lack the H.264 (x264) and HEVC (x265) encoders, so H.264/HEVC export would be lost.
- Building `ffmpeg` in CI: 30+ minutes per platform and ongoing maintenance.
- evermeet.cx (Intel Macs only) and Homebrew (shared libraries spread across the system).

## Pinning and download
- `packaging/ffmpeg.lock` lists, per platform: download URL, SHA-256, `ffmpeg` version, provider, licence, the provider's build-script source and commit, and the matching ffmpeg.org source tarball with its SHA-256.
- `scripts/fetch-ffmpeg.sh <target> <dest>` downloads the archive, verifies the SHA-256 and extracts only `ffmpeg`, `ffprobe` and the needed DLLs.
- BtbN keeps the last build of each month for 2 years, so a month-end build is pinned. How long Riedl's per-build URLs stay online is unknown; if a pinned URL disappears, the release build fails loudly and the pin is refreshed.
- The pin is refreshed for every release, because `ffmpeg` parses untrusted media and must receive security fixes.
- Before packaging, a smoke test runs the bundled copy: it must report its version, list the `libx264`, `libx265`, `libvpx-vp9` and `libopus` encoders, and the `subtitles`, `amix` and `alimiter` filters.

## Where it is installed
Always next to the main executable.

| Package | `ffmpeg`, `ffprobe` | Fonts and notices |
|---|---|---|
| Windows installer | install folder (with the DLLs) | `fonts/` and `notices/` beside the executable |
| macOS app | `videolore.app/Contents/MacOS/` (inside the app's signature seal) | `Contents/Resources/` |
| Linux AppImage | `usr/bin/` inside the image | `usr/share/videolore/` |
| Linux .tar.gz | top folder, next to `videolore` | `fonts/` and `notices/` in the same folder |

## How it is found
Lookup order:
1. path set in settings;
2. environment variables `VIDEOLORE_FFMPEG` / `VIDEOLORE_FFPROBE`;
3. the bundled copy: the executable's folder, then `../Resources` (macOS);
4. `PATH`.

The user override (1 and 2) is kept for people who want their own `ffmpeg`. Settings show the resolved path, the version and where it came from (bundled, override or `PATH`).

At startup, in the background, `videolore` checks the version, the encoders and the filters it needs (now also `subtitles`, `amix`, `alimiter`, `volume`, `split`, `asplit`). The messages distinguish:
- **bundled copy damaged**: the file is there but does not run or lacks features (reinstall);
- **not installed**: no copy found anywhere (only possible in source builds or after deleting files).

## Subtitle rendering and fonts
- Subtitles are burned in with `ffmpeg`'s `subtitles` filter (libass), present in all three builds.
- The bundled font is Noto Sans Bold (SIL Open Font License 1.1, about 0.5 MB), stored in `assets/fonts/` with its licence text and source.
- It covers Latin, Greek and Cyrillic only. **Limitation:** other scripts (for example Arabic, Chinese, Hindi) fall back to whatever system fonts libass finds, so they may look different per OS or show as empty boxes, and the app's subtitle editor cannot display them.

## Licences and source code
- `videolore` is GPL-3.0-only. It runs `ffmpeg` as a separate program, but because we distribute GPL `ffmpeg` binaries we must also make their source available.
- The chosen builds are GPL version 3 or later (`--enable-gpl --enable-version3`); x264 and x265 are GPL version 2 or later. All are compatible with GPL-3.0.
- Every release has an extra download, `videolore-<version>-third-party-source.tar.xz`, with:
  - the matching ffmpeg.org source tarball and its signature;
  - a snapshot of each provider's build scripts at the pinned commit (which pins every library version);
  - the Noto Sans source reference and licence;
  - a README.
- The third-party notices (About screen and installer) contain a written offer, valid for 3 years, to provide any source not included, and a note that H.264/HEVC patent licensing varies by country.
- Assumption: this compliance approach has not been reviewed by a lawyer.

## Size
Bundling makes each installer roughly 70–130 MB (estimate; measured in CI).

## Tests
- Development and CI tests use the `ffmpeg` from the Ubuntu packages, which includes x264, x265 and libass, so the HEVC tagging and subtitle burn-in tests run for real.
- The release build runs the smoke test above on the bundled copy on each platform.

## Risks
- The macOS build host's URL retention is unknown (release build fails loudly; re-pin).
- The source-code compliance has not been reviewed by a lawyer.
