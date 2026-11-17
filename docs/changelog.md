# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 1.2.8 — 2026-11-17 18:59 +0000

### Workflow
- Commit messages stay one line but may now include the co-author trailer as the only extra line (rule 9).

## 1.2.7 — 2026-11-17 18:55 +0000

### Workflow
- Agent models: Product Manager, Architect and Documenter run on Haiku; the Principal Engineer runs on Opus when the session model is Opus, otherwise on Sonnet (new rule 13 in the workflow rules).

## 1.2.6 — 2026-11-16 23:30 +0000

Boxes on the picture and 360 video: every movable object on the picture gets a visible box that moves and resizes, a group of Align and Reset commands is added to the toolbar, and 360° videos can be looked around in and exported as a flat video that follows a camera path set by the user.

### Added
- **Boxes on the picture**: the preview draws a thin box around each movable object (today the subtitle lines); drag it to move, drag a corner to enlarge or shrink. The box is never exported, and View > Show boxes on the picture hides it. One drag is one undo step. Requirements REQ-096 and REQ-097 in [Boxes on the picture and 360 video](business-picture.md).
- **Align horizontally, Align vertically and Reset**: a separate toolbar group, also in the Effects and right-click menus, with shortcuts and tooltips. Align centres the selected object(s) on the picture on that axis; Reset returns a subtitle line to its style's place and 100% size, or a 360 view to the front (REQ-098, REQ-099).
- **Per-line text size**: each subtitle line can be 25–400% of its style's text size, set with its box, kept in the project and shown the same in the preview and the export (REQ-097).
- **360° video**: a full equirectangular 360° video is recognised from its file information (or set per clip with "Treat as 360° video") and shown as a camera view; drag to look around, mouse wheel or pinch to zoom, during playback too. **Set POV** stores the view at the playhead; between POVs the camera moves in a straight line at constant speed, and the path follows the clip when it is split or trimmed (REQ-100 to REQ-102). The camera view is made from a preview copy for large sources, and every interface frame stays under 200 ms while looking around (REQ-104).
- **360 export**: a 360 clip is exported as a normal flat video of the camera path, with fades, subtitles, music and volumes (REQ-103). Architecture in [360 video](architecture-360.md) and [boxes](architecture-boxes.md).

### Changed
- **Project format version 4** (REQ-105): stores the subtitle line sizes, the "360°" switch of each clip and the POVs of each source. Projects from 1.2.5 and earlier open with the same result; projects saved by 1.2.6 cannot be opened by 1.2.5 or earlier.

### Known limitations
- Exporting a 360 clip with a moving camera runs at about one third of real time (a 30 s clip takes about 90 s); the interface stays responsive meanwhile.
- Only single-picture equirectangular 360 video is supported; spatial sound is played as it is and does not follow the camera; no camera roll; no keyboard control of the camera.
- Align horizontally does nothing for a line that is already at the style's place; an enlarged line's text may wrap and shift its centre slightly.
- Subtitle lines shown at the same time can overlap after moving or enlarging.

### Quality
- Line coverage 96.0%; feature coverage 105/105 (REQ-096 to REQ-105 added and implemented).

## 1.2.5 — 2026-11-16 22:40 +0000

Workflow only: no change to the program or to what the app does.

### Workflow
- **Notes are indexes**: each `ai-docs/` file starts with an index of every paragraph of the human docs its owner manages (`- <file> § <heading> — L<start>-<end> (<n> lines)`), so an agent finds a paragraph with `grep` and reads only its lines. Detail that only an AI needs goes in `ai-docs/<area>-<topic>.md`, indexed from the main file. The current long notes are converted by their owners in their next development (roadmap).
- **Feature pipeline by files**: the product-manager and the architect write their plans into the notes and docs and return only their `[SEVERE]` questions; the principal-engineer returns its results. The command relays only the user's answers, file paths and counts, not the agents' reports.
- **Documenter after each part**: the documenter runs after the business, architecture and code parts, and updates the docs only. The version is bumped once per development. It may write any `docs/` file and the README, with no owner guardrail; it never edits another agent's notes.
- **README poem**: rewritten only when a new major version is released; other README changes keep it.
- **Docs compaction**: on each release the documenter reads and compacts every `docs/` file, removing only useless data. Done once now: the workflow doc was rewritten, the business UI doc lost repeated text, and the roadmap workflow item was updated. The other docs were checked for repeated lines (four found, kept as summaries) and will be read in full at the next release.

## 1.2.4 — 2026-11-15 22:02 +0000

Workflow only: the project page can be published.

### Workflow
- **Fixed the Pages deployment**: the workflow no longer asks GitHub to switch Pages on (the default token is not allowed to), so the deployment step failed. Turning Pages on is now a one-time manual step, see 1.2.3.

## 1.2.3 — 2026-11-15 22:01 +0000

Workflow only: the project page is published on GitHub Pages.

### Workflow
- **GitHub Pages deployment** with `.github/workflows/pages.yml`: on every push to `main` that changes `github_page/`, the page is published with the official Pages actions, pinned to commit SHAs like the other workflows. It can also be started by hand. Before the first run, turn Pages on once in the repository settings: Settings → Pages → Build and deployment → Source: GitHub Actions.

## 1.2.2 — 2026-11-15 21:59 +0000

A project page for the GitHub repository: the brief, the story of the project as a timeline of every version, and the copyright and licence.

### Added
- **GitHub project page** in `github_page/index.html` (one static page, no build step): the brief taken from the README, the story as a horizontal timeline from the first commit (13 November 2026) to today, with each version's number, date and time and a sentence on what it is, and the copyright and licence. It scrolls sideways on phones and follows the light or dark setting.

## 1.2.1 — 2026-11-15 21:52 +0000

Workflow only: no change to the program or to what the app does.

### Workflow
- **Fewer tokens for each development.** The Documenter no longer reads all of `docs/` (355 KB at 1.2.0) each time: `scripts/docs-check.sh` checks links, sub-document indexes, forbidden AI/agent terms, changelog headers, and the README's latest changes and poem, and the Documenter reads in full only the files the development changed. A full prose read happens on release.
- **Shorter test reports.** The Principal Engineer runs the tests in quiet mode and reads only the summaries and the failures, not the full logs.
- **Rule 12 (token economy)** in `AGENTS.md`: agents read only what their task needs; long notes are compacted by their owner, never by another agent.
- Considered and not done: merging agents (saves one cold start per feature, the notes still dominate the cost) and compacting the notes now (owners' files, planned in the roadmap). Details in `docs/workflow.md`, section Token economy.

## 1.2.0 — 2026-11-15 21:42 +0000

Effects can be applied to one clip or to the whole video, subtitles can reuse a video's own subtitle track, and each subtitle line can be moved on the picture. Projects saved by this version use a newer format: versions 1.1.0 and earlier cannot open them.

### Added
- **Apply an effect to one clip or to the whole video.** Drag an effect from the Effects list onto a clip, or right-click the clip and choose Effects. Double-click an effect in the list to apply it to the whole video.
- **Fade through black at the start and at the end** of the video, in the same way as the fades between clips. The end fade starts two frames early, so the last frame is black.
- **Volume and mute per clip**, through the Volume effect, on one clip or on the whole video.
- **Subtitles for one clip**: the subtitles effect creates lines only for the part of the source the clip uses; the lines of the other clips are untouched. The undo step reads "Create subtitles" for a clip and "Generate subtitles" for the whole video.
- **Use a video's own subtitles.** When a video contains a text subtitle track (SRT, ASS/SSA, MP4 timed text or WebVTT), videolore asks whether to use its text or to recognise the speech. Using the text keeps the words and the line times exactly, and the words are matched to the speech. Picture-based tracks (DVD, Blu-ray, DVB) are ignored. A track with an unknown language is labelled by its code, or "Track N". Media imported before 1.2.0 is checked in the background when a project is opened.
- **File > Export subtitles (.srt)…** writes the lines as they are on the timeline, after cuts, trims and copies. It is greyed out when there are no subtitles.
- **Subtitle positions**: drag a subtitle line in the preview to its own place. The text box stays inside the picture, each drag is one undo step, and Reset position restores the style's place.
- **Several lines at once**: Ctrl/Cmd+click on subtitle markers or on lines in the Subtitles list adds or removes a line from the selection; Shift+drag on the timeline selects every line that starts in the dragged range. Dragging one selected line moves the whole group to the same place.
- **Timeline ruler** shows seconds up to 90 seconds, minutes and seconds up to 90 minutes, and hours and minutes beyond.

### Changed
- The Effects list shows the effect names only, without counts or levels.
- A fade applied to a clip opens the Fades list popup.
- A recognition failure during matching shows a notice dialog and the number of lines that could not be matched; the lines keep the video's text and times.
- Project format version 3. Projects saved by 1.2.0 cannot be opened by 1.1.0 or earlier: those versions refuse them with the "newer version" message and leave the file untouched. Projects saved by earlier versions open unchanged.

### Quality
- Line coverage 95.9%; feature coverage 95/95 (REQ-082 to REQ-095 added, some requirements reworded). Tests: 310 unit (1 ignored) and 29 integration (3 ignored); fmt, clippy, coverage and version checks pass.

### Known limitations
- When a video's own subtitles are reused, the word-by-word highlight timing is approximate. The accuracy target (at least 90% of words matched, each within 0.3 s) is not met yet with the Fast model: on the reference clip 23 of 28 words (82%) were matched, with word starts 0.48 s off on average and 1.10 s at worst. The strict test is marked ignored until the target is met.
- Matching works only as well as the track's text follows the speech; translated or paraphrased tracks often get evenly spread words.
- Subtitle lines moved to their own place are not kept apart: two lines shown together can overlap.
- Editing a line's text after moving it can make it larger, and in the export it can then reach past the edge of the picture; drag it again or use Reset position.
- Start and end fades stay at the video's edges when clips are moved.
- The `.srt` export has no positions.
- Drag and drop does not work in Linux Wayland sessions (as in 1.1.0).

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