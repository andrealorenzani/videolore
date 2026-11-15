# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 1.1.0 — 2026-11-15 17:52 +0000

The window is reorganised around a menu bar, a toolbar, a status bar, resizable sections and an Effects list. Projects and the project file format are unchanged.

### Added
- **Menu bar** with File, Edit, Playback, Effects, View and Help. Each item shows its shortcut and is greyed out exactly when its command is not available; the greyed main commands explain why in a tooltip (for example "Select a clip first" or "Nothing to undo").
- **Toolbar** with Import, Add to timeline, Add music, Split, Cut, Copy, Paste, Delete, Undo, Redo, Play/Pause and Export (Cancel export while exporting). When the window is narrow, the buttons that do not fit go into a "»" menu.
- **Status bar**: the latest message on the left; background work with progress in the middle (import analysis, preview copies, speech model download, subtitle generation and export), with Cancel for subtitles and export; the output file on the right.
- **Resizable sections**: drag the borders between Media and Preview, between Timeline and Effects, and between the top and bottom rows. The sizes are remembered for the next start, and View > Reset layout restores the default layout.
- **Effects section** listing Fade through black, Subtitles and Volume, with a count or the current levels. Each one opens its own settings window, which does not block editing or playback.
- **Markers on the timeline** for every fade and every subtitle line. Clicking a marker opens the settings of that fade or line. Where two clips meet without a fade, an "Add fade" handle adds one. Overlapping markers open a small list to choose from.
- **Clip details** window: double-click a clip, or use Edit > Clip details…, to trim, set fades, merge, move, split or delete it.
- **Right-click menus** on video and music clips, on markers and on the empty timeline.
- **Open recent**: File > Open recent lists the last 10 projects, most recent first, with Clear list. A project that is no longer there is reported and removed from the list.
- **Drag and drop** of media files onto the window to import them, and of a project to open it (Windows, macOS and Linux with X11). If several projects are dropped, the first one opens and the others are reported as ignored.
- **Keyboard shortcuts**: N new project, O open, Shift+S save as, I import, E export, Delete or Backspace to delete, Left and Right to step one frame, Home and End to go to the start and end, Ctrl/Cmd+= and Ctrl/Cmd+- to zoom, Ctrl/Cmd+0 to fit the project, F1 for the Keyboard shortcuts window. Esc closes the topmost popup. Shortcuts never fire while typing in a text field or while a dialog is open.
- **Undo and Redo name the action**: Edit > "Undo Split", "Redo Remove fade".
- **Scrollbars** wherever content can overflow, so every control and text stays reachable with the window at 800x500.

### Changed
- Window layout from top to bottom: menu bar, toolbar, content area, status bar. The content area has Media (30% of the top row) beside Preview (70%), and Timeline (70%) beside Effects (30%) in the bottom row, which takes one third of the height. Each section has a title.
- The zoom slider is in the Timeline header; View > Zoom in, Zoom out and Fit project zoom too.
- Export and its progress moved to the File menu, the toolbar and the status bar. The command bar is replaced by the toolbar.
- Clicking a fade marker opens its settings instead of removing the fade. A fade is removed from its settings or from the right-click menu.
- Video and music volume are in the Volume window; trim and music controls are in the Clip details window.
- Settings is in the Edit menu; Ctrl/Cmd+, still opens it.
- The keyboard shortcut list moved from the Effects section to Help > Keyboard shortcuts.
- Ctrl/Cmd+= and Ctrl/Cmd+- zoom the timeline; the window no longer changes its text size with them.
- Long file names are shortened with "…", with the full name in a tooltip.

### Removed
- The command bar at the top of the window, replaced by the menu bar, the toolbar and the status bar.

### Quality
- Line coverage 96.1%; feature coverage 81/81 (REQ-024 and REQ-025 reworded, REQ-066 to REQ-081 added). Tests: 263 unit and 23 integration; fmt, clippy, coverage and version checks pass.

### Known limitations
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open….
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- Popup positions and the window size and position are not remembered between sessions.
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text. Paste in the menu and toolbar always works.

## 1.0.0 — 2026-11-14 22:36 +0000
- Version 1.0.0 opened after release 0.4.2 "MVP". No user-facing changes yet.

## 0.4.3 — 2026-11-14 22:35 +0000

Fixes the macOS and Windows installers, which the 0.4.2 installer job could not build. The Linux installers of 0.4.2 were unaffected. The macOS and Windows installers of 0.4.2 are not in its GitHub release; they are confirmed only when the installer job runs again.

### Fixed
- The generated cargo-packager configuration (`scripts/package.sh`) had four errors, all fixed: the file association role is `editor` (was `Editor`), the NSIS key is `installMode` (was `install-mode`), `name = "videolore"` is present (without it cargo-packager stops with "Not a directory"), and all paths are absolute, because cargo-packager reads relative paths from the folder of the configuration file (on Windows they are written with `cygpath -m`).
- Missing DLLs of the Windows ffmpeg build, and a missing produced installer or `.app`, now stop the script with a clear error.

### Added
- `scripts/package.sh <target> --config-only` writes only the configuration (no ffmpeg, no build), so it can be checked on any machine.
- Test `packaging::tests::packager_config_is_valid_for_cargo_packager` checks the keys, enum values and paths of both configurations (covers REQ-040, REQ-041, REQ-042).

### Changed
- `hdiutil create` for the macOS disk image is retried up to 5 times.
- Validated locally with the real cargo-packager 0.11.8: the deb and app formats were built from both configurations, and the NSIS script was generated.

### Quality
- Line coverage 96.3%; feature coverage 65/65. Tests: 223 unit and 23 integration, 2 ignored; fmt, clippy, coverage and version checks pass.