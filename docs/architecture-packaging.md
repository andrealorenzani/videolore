# Architecture: installers, packaging and release pipeline

Part of [Architecture](architecture.md). Build and CI gates: [build](architecture-build.md). Bundled ffmpeg: [engine](architecture-engine.md#bundled-ffmpeg). Security of the installers: [security](architecture-security.md).

## What is produced
Every release (version tag) gets these downloads, built by CI (names with `<ver>` = version, from the release):

| Platform | File | How it installs | Built with |
|---|---|---|---|
| Windows x86_64 | `videolore-<ver>-windows-x86_64-setup.exe` | Wizard, per user, no administrator rights | cargo-packager (NSIS) |
| macOS Apple Silicon | `videolore-<ver>-macos-aarch64.dmg` | Drag `videolore` into Applications | cargo-packager, then ad-hoc seal |
| Linux x86_64 | `videolore-<ver>-linux-x86_64.AppImage` | One file; allow executing once, then double-click | appimagetool |
| Linux x86_64 | `videolore-<ver>-linux-x86_64.tar.gz` | Extract, double-click `videolore` | `tar` |
| all | `videolore-<ver>-third-party-source.tar.xz` | Source of the bundled third-party programs | see [engine](architecture-engine.md#bundled-ffmpeg) |

- Player (1.6.0): `videolore-player-<ver>-…`, the same kinds of file (AppImage and tar.gz, NSIS setup, disk image). Identifiers `io.github.andrealorenzani.videolore` (editor) and `io.github.andrealorenzani.videolore-player` (player), so both install side by side. The player has no file association and writes no Linux menu entry. `scripts/package.sh <target> [editor|player]` (default editor). Each installer has its own Rust notices (`rust-crates.txt`, `rust-crates-player.txt`); the third-party notices and the source archive are shared.
- Each package holds `ffmpeg`, `ffprobe`, the subtitle font and the notices; nothing else to install. Installers grow by the Arabic and Devanagari fonts (2.0.4, ADR-172; a few MB); CJK is not bundled. Size 70 to 130 MB each (estimate, measured in CI).
- **System requirements:** subtitles on Windows and Linux x86_64 need AVX2, FMA, F16C (about 2013 or later); older processors get a message ([subtitles](architecture-subtitles.md#cpu-requirements)). Every Apple Silicon Mac is supported. Linux packages need glibc 2.39 or newer (Ubuntu 24.04 LTS, current Fedora): the package job fails if the executable needs a newer glibc (0.3.0).
- **No tuning in installers (0.3.0):** the program detects the computer and sets recommended settings on its first start, the same everywhere ([performance](architecture-performance.md)).
- **Unsigned (since 0.2.0; decided by Andrea; postponed, not rejected):** the README explains first launch: Windows SmartScreen "More info → Run anyway"; macOS Gatekeeper "Open Anyway" in System Settings → Privacy & Security; Linux AppImage "Allow executing file as program". The macOS app carries an ad-hoc seal only.

## Windows
- Installs for the current user only (no administrator prompt; registry entries under the user's keys). Start-menu shortcut; uninstall from Settings → Apps.
- `.vlproj` files open with `videolore`; the licence page shows the third-party notices (its file has a byte-order mark).
- The uninstaller removes only the install folder and videolore's registry entries; never projects.
- Installer tools pinned by version and SHA-256 (ADR-181). Player: NSIS template `packaging/windows-player.nsi` ("Open with" entries only); not run on Windows yet.

## macOS
- Disk image with the app to drag into Applications. The app declares the `.vlproj` document type.
- After the build, the whole app (with the bundled ffmpeg) is re-sealed ad hoc so nested programs are consistent. Not a Gatekeeper signature. If Gatekeeper blocks the bundled ffmpeg the README gives `xattr -dr com.apple.quarantine /Applications/videolore.app`.
- Double-click: the "open document" event goes to the running app (`objc2`, installed before the window starts so launch-time files are caught) and opens through the unsaved-changes prompt. Fallback: opening from inside the app works.
- Menu bar: one allow-listed call (2.0.4, ADR-183) and default menu switched off so Cmd+Q reaches the app (ADR-024).

## Linux
Two self-contained options, chosen by Andrea:
- **AppImage:** one file; current AppImage runtime, which needs `fusermount3` (present on current Ubuntu LTS and Fedora), not the older `libfuse2`. Assembled by hand and packed with `appimagetool`, pinned by SHA-256 in `packaging/tools.lock`; cargo-packager is not used because it does not control the runtime. The check job builds it from stub binaries (`--stub`) and checks desktop entry, icon, AppRun and notices folder; the stub output is never uploaded.
- **.tar.gz:** folder `videolore-<ver>-linux-x86_64/` with `videolore`, `ffmpeg`, `ffprobe` (executable), `fonts/`, `notices/`, the icon, `README.txt`, the marker `.videolore-bundle`, a desktop entry, a launcher script (REQ-233). Some file managers do not run programs on double-click: README fallback (right-click → Run, or `./videolore` once).
- **Menu and file association:** no installer step, so videolore adds itself at start-up only when run from an AppImage or the extracted folder (never from source builds or `cargo run`): `~/.local/share/applications/videolore.desktop` (menu entry, opens `.vlproj`), `~/.local/share/icons/hicolor/256x256/apps/videolore.png`, `~/.local/share/mime/packages/videolore.xml` (the `.vlproj` type). It then refreshes the desktop and mime databases (failures ignored). A moved AppImage or folder is updated at the next start. About → "Remove from application menu" deletes the three files; projects are never touched. The launch command is quoted per the desktop-entry spec (1.1.0).
- **No `.deb` package and none planned** (decided by Andrea, 2026-11-20).
- Wayland: drag and drop of files from outside does not work in native Linux Wayland sessions (File > Import and Open remain).

## Double-click on a project (1.x, 2.0.x)
- **Windows and Linux:** the system starts `videolore` with the project path. If it is already running, a second window opens with that project (Andrea's choice); the first window and its unsaved work are untouched. One window per lock file and inbox (ADR-182).
- **macOS:** see above.

## Icon
- Original icon, SVG in `assets/icon/videolore.svg`. PNG, Windows `.ico`, macOS `.icns` and a raw 256x256 window icon generated once by `scripts/icons.sh` and committed (no image tools or crates at build time). The Windows executable carries no embedded icon; shortcut, installer and taskbar show it. The player uses the same icon files (decided 2026-11-19).

## Release pipeline
- Package job on each version tag (and manual request) for Windows, macOS and Linux, after the check and MSRV jobs: (1) download the pinned ffmpeg and run its smoke test; (2) build the release executable with `--locked`; (3) `scripts/package.sh <target>`; (4) attach to the GitHub release. The third-party source archive is built once, on Linux (with the ffmpeg.org tarball, its signature, a snapshot of the provider build scripts at the pinned commit, the Noto Sans source and licence, and a README).
- cargo-packager: fixed version in CI only; not a dependency of videolore. Fallback if it stops: a hand-written NSIS script and `hdiutil` (no release for about a year).
- Release notes come from the changelog; the release is created by the tag push and watched until it ends.
- Fixed runner versions will be retired one day and must then be moved deliberately (check the glibc limit on Linux).

## CI runners and actions (0.3.0)
- Fixed runners: `ubuntu-24.04` (Linux jobs), `macos-15`, `windows-2025`. Building on Ubuntu 24.04 (glibc 2.39) keeps the Linux packages on Ubuntu 24.04 and 26.04 LTS and current Fedora.
- Actions on Node.js 24; `actions/cache` v4 to v6.1.0. Third-party actions pinned to full commits with the version in a comment:

| Action | Release |
|---|---|
| `actions/checkout` | v7.0.1 |
| `actions/cache` | v6.1.0 |
| `actions/upload-artifact` | v7.0.2 |
| `Swatinem/rust-cache` | v2.9.3 |
| `taiki-e/install-action` | v2.90.4 (no Node.js) |
| `softprops/action-gh-release` | v3.0.4 |
| `dtolnay/rust-toolchain` | pinned commit, `stable` or the minimum Rust version (no Node.js) |

- Least privilege: read-only repository access for jobs; only the release job writes.

## Bundled ffmpeg refresh (0.3.0)
- FFmpeg 9.0.2 is the latest release; macOS keeps the Martin Riedl 9.0.2 build. Linux and Windows moved to the newest BtbN build of the 9.0 branch (2026-11-27, version 9.0.2-982-g836df4e858), pinned by SHA-256 in `packaging/ffmpeg.lock`. It is a daily build kept for a shorter time than month-end builds: if it disappears, the package job fails and the pin moves to the 2026-11-30 month-end build. Each release refreshes the pin (ffmpeg parses untrusted media).

## Third program: the screen recorder (3.1.2)
- `scripts/package.sh <target> screen-spy` builds the recorder's installers the same way as the player's: AppImage and tar.gz (Linux), NSIS setup (Windows), disk image (macOS). Identifier `io.github.andrealorenzani.videolore-screen-spy`; own notices `rust-crates-screen-spy.txt` and `third-party-screen-spy.txt`; no file association, no desktop entry written by the program; the default NSIS template; macOS minimum system version 11.0, as the editor and the player: the recorder must open on older Macs to show its message (computer sound needs macOS 14.6, checked at Start).
- Each recorder installer bundles its own pinned ffmpeg (REQ-040) and is about 70 to 130 MB (estimate). The smoke test of the bundled build also checks the capture devices (`gdigrab`, `avfoundation`, `x11grab`, `pulse`) and the encoders (`libx264`, `aac`, `libmp3lame`).
- CI: the executables job names the recorder's executable; the package matrix calls it; `installer-config.yml` checks its packager files; the network check covers it; version and feature-coverage scripts include it; the Linux runner runs a real capture test under Xvfb with a PulseAudio null sink.

## Assets in packages (1.6.2, 1.6.3)
- Splash pictures and summaries are compiled into the programs: packages gain no new file. The notices name `assets/splash/splash.gif` (editor) and `splash-player.gif` (player): the author's own work, MIT (recorded from 1.6.3).
- Icon: see above; the splash: [UI](architecture-ui.md#splash-160-162-204).

## Risks
- First launch of unsigned packages, the macOS launch-time open event, and Linux file-manager behaviour are checked by hand (manual checklist in the packaging part of the developer documentation). If the macOS event fails, opening from inside the app still works.
- The Windows player installer has not been run on Windows.
- Fixed runner versions and the glibc limit (see above).
