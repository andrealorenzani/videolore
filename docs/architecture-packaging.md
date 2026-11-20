# Architecture: installers and packaging

Part of [Architecture](architecture.md).

## What is produced
Every release (version tag) gets five downloads, built by CI:

| Platform | File | How it installs | Built with |
|---|---|---|---|
| Windows x86_64 | `videolore-<ver>-windows-x86_64-setup.exe` | Wizard, per user, no administrator rights | cargo-packager (NSIS) |
| macOS Apple Silicon | `videolore-<ver>-macos-aarch64.dmg` | Drag `videolore` into Applications | cargo-packager, then ad-hoc code seal |
| Linux x86_64 | `videolore-<ver>-linux-x86_64.AppImage` | One file; allow executing once, then double-click | appimagetool |
| Linux x86_64 | `videolore-<ver>-linux-x86_64.tar.gz` | Extract, double-click `videolore` | `tar` |
| all | `videolore-<ver>-third-party-source.tar.xz` | Source of the bundled third-party programs | see [ffmpeg](architecture-ffmpeg.md) |

Each package contains `ffmpeg`, `ffprobe`, the subtitle font and the notices; nothing else needs to be installed.

**System requirements.** Generating subtitles on Windows and Linux x86_64 needs a CPU with AVX2, FMA and F16C (about 2013 or later); on older processors `videolore` shows a message instead (since 0.3.0) and the rest of the program has no special CPU requirement. Every Apple Silicon Mac is supported. See [subtitles](architecture-subtitles.md#cpu-requirements-since-021). Linux packages need glibc 2.39 or newer (Ubuntu 24.04 LTS and later, current Fedora).

**No tuning in the installers (0.3.0).** Installers only install. The program itself detects the computer and picks recommended settings on its first start, the same way on every system and in the AppImage (see [performance](architecture-performance.md)).

**Unsigned** (since 0.2.0) (decided by Andrea; signing is postponed, not rejected). The README explains the one-time first-launch steps: Windows SmartScreen "More info → Run anyway", macOS Gatekeeper "Open Anyway" in System Settings → Privacy & Security, and the Linux AppImage "Allow executing file as program".

## Windows
- Installs for the current user only (no administrator prompt, registry entries under the user's own keys).
- Start-menu shortcut; uninstall from Settings → Apps.
- `.vlproj` files open with `videolore`.
- The licence page of the wizard shows the third-party notices.
- The uninstaller removes only the install folder and `videolore`'s registry entries; it never deletes projects.

## macOS
- Disk image with the app to drag into Applications.
- The app declares the `.vlproj` document type, so Finder opens projects with it.
- After building, the whole app (including the bundled `ffmpeg`) is re-sealed with an ad-hoc signature, so the nested programs are consistent with the app. This is not a Gatekeeper signature.
- If Gatekeeper still blocks the bundled `ffmpeg`, the README gives the fallback `xattr -dr com.apple.quarantine /Applications/videolore.app`.

## Linux
Two options, chosen by Andrea, both self-contained:
- **AppImage**: one file. It uses the current AppImage runtime, which needs `fusermount3` (present on current Ubuntu LTS and Fedora) and not the older `libfuse2`. The image is assembled by hand and packed with `appimagetool`, whose version is pinned by SHA-256 in `packaging/tools.lock`. cargo-packager is not used here because its AppImage path does not control the runtime.
- **.tar.gz**: a folder `videolore-<ver>-linux-x86_64/` with `videolore`, `ffmpeg`, `ffprobe` (executable), `fonts/`, `notices/`, the icon, a `README.txt` and a marker file `.videolore-bundle`. Desktop archive managers keep the executable bit when extracting. Some file managers do not start programs on double-click; the README gives the fallback (right-click → Run, or run `./videolore` once).

**Application menu and file association.** Linux has no installer step, so `videolore` adds itself at startup, only when it runs from an AppImage or from the extracted folder (never for source builds or `cargo run`):
- `~/.local/share/applications/videolore.desktop` (menu entry, opens `.vlproj` files);
- `~/.local/share/icons/hicolor/256x256/apps/videolore.png`;
- `~/.local/share/mime/packages/videolore.xml` (the `.vlproj` file type).

It then refreshes the desktop and file-type databases if those tools exist (failures are ignored). If the AppImage or folder is moved, the entry is updated on the next start. Uninstalling means deleting the AppImage or the folder; the About screen offers "Remove from application menu", which deletes the three files above. Projects are never touched.

## Opening a project by double-click
- **Windows and Linux:** the system starts `videolore` with the project path. If `videolore` is already running, a second window opens with that project (Andrea's choice); the first window and its unsaved work are untouched.
- **macOS:** the system sends the file to the running app as an "open document" event. `videolore` receives it (through a small handler using the `objc2` crates, installed before the window starts so files opened at launch are also caught) and opens it through the unsaved-changes prompt.

## Icon
- An original icon, drawn as SVG in `assets/icon/videolore.svg`.
- The PNG, Windows `.ico`, macOS `.icns` and a raw 256x256 image for the window icon are generated once by `scripts/icons.sh` and committed, so building needs no image tools or image crates.
- The Windows executable itself carries no embedded icon; shortcut, installer and taskbar show it.
- The player (1.6.0) uses the same icon files as the editor (decided 2026-11-19); no separate player icon.

## Release pipeline
- A `package` job runs on each version tag (and on manual request) for Windows, macOS and Linux, after the check and MSRV jobs pass:
  1. download the pinned `ffmpeg` and run the smoke test;
  2. build the release executable with `--locked`;
  3. run `scripts/package.sh <target>`;
  4. attach the files to the GitHub release.
- The third-party source archive is built once, on Linux.
- cargo-packager is installed at a fixed version in CI only; it is not a dependency of `videolore`.
- The existing build job still produces bare executables for every push and pull request, but they are no longer attached to releases.
- On Linux, packaging fails if the `videolore` executable needs a glibc newer than 2.39, so the packages keep running on Ubuntu 24.04 LTS and current Fedora (since 0.3.0).

## CI runners and actions
Since 0.3.0, to remove GitHub's deprecation warnings and make builds reproducible:
- **Fixed runner versions** instead of `*-latest`: `ubuntu-24.04` (all Linux jobs), `macos-15`, `windows-2025`. Building on Ubuntu 24.04 (glibc 2.39) keeps the Linux packages running on Ubuntu 24.04 and 26.04 LTS and on current Fedora; the AppImage additionally needs `fusermount3`, present on all of them.
- **Actions on Node.js 24:** `actions/cache` moves from v4 (Node.js 20, deprecated) to v6.1.0. All third-party actions are pinned to the full commit of their current release, with the version in a comment:

| Action | Release |
|---|---|
| `actions/checkout` | v7.0.1 |
| `actions/cache` | v6.1.0 |
| `actions/upload-artifact` | v7.0.2 |
| `Swatinem/rust-cache` | v2.9.3 |
| `taiki-e/install-action` | v2.90.4 (no Node.js) |
| `softprops/action-gh-release` | v3.0.4 |
| `dtolnay/rust-toolchain` | pinned commit, `stable` or the minimum Rust version (no Node.js) |

- **Least privilege:** jobs get read-only access to the repository; only the release job can write.
- **Tests run once**, with coverage measurement, in the check job; the minimum-Rust job only builds (the minimum Rust version equals the current stable one by policy, and no test is dropped). The three speech models are cached in CI for the accuracy test.

## Bundled ffmpeg refresh (0.3.0)
- FFmpeg 9.0.2 is still the latest release. macOS keeps the Martin Riedl 9.0.2 build already pinned.
- Linux and Windows move to the newest BtbN build of the 9.0 branch (2026-11-13, version 9.0.2-57), pinned by SHA-256 in `packaging/ffmpeg.lock`.
- That is a daily build, kept by BtbN for a shorter time than month-end builds: if it disappears before a later re-package, the package job fails loudly and the pin moves to the 2026-11-30 month-end build.

## Two products (1.6.0)
- `scripts/package.sh <target> [editor|player]`: the default is the editor, so existing calls are unchanged. The player is built from the workspace member `player/` (see [build](architecture-build.md)).
- Editor: `videolore-<version>-…` (unchanged names). Player: `videolore-player-<version>-…` with the same kinds of file (AppImage and tar.gz on Linux, NSIS setup on Windows, disk image on macOS).
- Identifiers: `io.github.andrealorenzani.videolore` (editor) and `io.github.andrealorenzani.videolore-player` (player), so both install side by side. The player has no file association and no Linux menu entry written by the program.
- Both carry the same pinned `ffmpeg` and `ffprobe` (`packaging/ffmpeg.lock`), so the player's smoke test is the editor's list without the editing-only filters. Each installer has its own Rust notices: `rust-crates.txt` (editor) and `rust-crates-player.txt` (player); the third-party notices and the source archive are shared.
- Release: the package job runs both products on each runner; the release globs already attach both sets of files.

## Risks
- First launch of unsigned packages, the macOS launch-time "open document" event and Linux file-manager behaviour can only be checked by hand; a manual checklist is kept in [code packaging](code-packaging.md). If the macOS event fails, opening the project from inside the app still works.
- cargo-packager has had no release for about a year; the fallback is a hand-written NSIS script and `hdiutil`.
- Fixed runner versions will themselves be retired by GitHub one day and must then be moved forward deliberately (checking the glibc limit on Linux).
