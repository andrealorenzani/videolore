# Business — Window, menus and effects

Back to the [index](business.md).

Since 1.1.0 the window is reorganised (resizable sections, menu bar, toolbar, status bar, one popup per effect, markers on the timeline). Since 1.2.0 an effect applies to one video clip or to the whole video, and the ruler switches unit with the visible range. In 1.1.0 nothing changes in how projects are saved: the markers show data the project already contains. The project format changed in 1.2.0 (see [Editing](business-editing.md), REQ-094).

## Features
- **Window**: from top to bottom, a menu bar, a toolbar, the four sections (Media, Preview, Timeline, Effects) and a status bar. Each section has a title header.
- **Resizable sections**: the borders between Media and Preview, between Timeline and Effects, and between the top and bottom rows can be dragged. Sizes are remembered for the next start; View > Reset layout restores the defaults.
- **Everything reachable**: at the smallest window size (800x500) every control is visible or reachable with a scrollbar; scrollbars appear only when needed; long names are shortened with "…" and shown in full in a tooltip.
- **Menus**: File, Edit, Playback, Effects, View, Help, each item with its keyboard shortcut and greyed out when it cannot be used.
- **Toolbar**: the most common commands in one row; at narrow widths the rest goes into a "»" menu.
- **Status bar**: the latest message, background work with progress (imports, preview copies, speech file download, subtitle generation, export, with Cancel where possible) and the output folder.
- **Effects section**: a list of the effect names (Fade through black, Subtitles, Volume); clicking one opens its settings popup.
- **Applying effects** (1.2.0): drag an effect from the Effects list onto a video clip, or right-click the clip and choose Effects ▸, to apply it to that clip; double-click it in the list to apply it to the whole video.
- **Markers on the timeline**: every fade (between clips, at the start and at the end of the video), every subtitle line and every clip with its own volume has a marker; clicking it opens the settings of that single item. Clip boundaries without a fade show a small "Add fade" handle.
- **Timeline ruler** (1.2.0): seconds when up to 90 seconds are visible, minutes and seconds up to 90 minutes, hours beyond.
- **Clip details**: double-click a clip to trim it, set its fades, its volume or merge it (video), or trim, move, split and delete it (music).
- **Right-click menus** on clips, markers and the empty timeline.
- **Recent projects**: File > Open recent lists the last 10 projects.
- **Drag and drop**: drop media files onto the window to import them, or a project to open it (Windows, macOS and Linux X11; not on Linux Wayland).
- **Undo names the action**: "Undo Split", "Redo Remove fade".

## Requirements

### Layout
- REQ-024 (implemented): The window shows, from top to bottom, a menu bar, a toolbar, the content area and a status bar. By default the top row takes 2/3 of the content height (Media 30% of the width, Preview 70%) and the bottom row 1/3 (Timeline 70%, Effects 30%). Each section has a title header; the Timeline header holds the zoom slider and a "Fit" button. These defaults apply at first start and after View > Reset layout. Acceptance: with no saved layout, at 800x500, 1280x800 and 2560x1440 the proportions hold within 1 pixel, sections do not overlap and the content area is fully covered.
- REQ-066 (implemented): Sections can be resized by dragging three borders: Media | Preview, Timeline | Effects, and top row | bottom row. The pointer shows a resize cursor over a border; the sections update live while dragging and only the two sections next to the border change. Minimum sizes: Media and Effects 160 pixels wide, Preview and Timeline 240 pixels wide, each row 100 pixels high; a border stops there. Resizing the window keeps the current proportions. The menu bar, toolbar and status bar keep a fixed height that fits their content. Acceptance: (a) dragging each border by ±100 pixels resizes exactly its two neighbours; (b) dragging past a minimum stops at the minimum; (c) after resizing the window the proportions hold within 1 pixel; (d) while an export and subtitle generation run, every frame during a border drag takes at most 200 ms (REQ-056).
- REQ-067 (implemented): Section sizes are a program setting (never stored in a project) and are restored at the next start. View > Reset layout restores the REQ-024 defaults. If a saved layout does not fit the current window, the minimum sizes win. Missing or damaged layout values give the default layout without a crash (REQ-057), keeping the other settings. Opening a project never changes the layout. Acceptance: resize, restart, same proportions; Reset layout gives REQ-024; a damaged or out-of-range value gives the default and the other settings are kept.
- REQ-068 (implemented): At 800x500, with any section at its minimum size, every control and text is visible or reachable with a scrollbar: every section, every effect popup, the clip details popup, Settings, dialogs and the keyboard shortcuts window. Scrollbars appear only when the content does not fit. The preview always keeps its play controls and seek bar visible; the picture shrinks first. The timeline scrolls vertically when its tracks do not fit at minimum height (it already scrolls horizontally). Long names are shortened with "…" and shown in full in a tooltip. Acceptance: an interface test at 800x500 with every section at its minimum, 50 media items, 200 subtitle lines and a long file name finds every interactive control inside the visible area or inside a scroll area; none is cut off out of reach.

### Menus, toolbar, shortcuts and status bar
- REQ-069 (implemented): A menu bar has the menus File, Edit, Playback, Effects, View and Help. Each item shows its keyboard shortcut and is greyed out exactly when its command is unavailable. Acceptance: every item runs its command, its enabled state matches the command's availability, and an open menu scrolls if it does not fit in the window.
  - File: New, Open…, Open recent ▸, Save, Save as…, Import media…, Export… (Cancel export while exporting), Export subtitles (.srt)… (since 1.2.0, REQ-088), Quit.
  - Edit: Undo, Redo, Cut, Copy, Paste, Delete, Split at playhead, Merge, Move left, Move right, Add selected media to timeline, Add selected media to music track, Clip details…, Settings…
  - Playback: Play/Pause, Go to start, Go to end, Previous frame, Next frame.
  - Effects: Fades…, Subtitles…, Volume…, Add/remove fade at playhead, Generate subtitles, Add subtitle line at playhead; since 1.2.0 also "Apply to selected clip ▸" and "Apply to whole video ▸" (each with Fade through black, Subtitles, Volume) and "Reset subtitle position".
  - View: Zoom in, Zoom out, Fit project, Reset layout.
  - Help: Keyboard shortcuts, About videolore and third-party notices.
- REQ-070 (implemented): A toolbar (replacing the 0.x command bar) shows in one row: Import, Add to timeline, Add music, Split, Cut, Copy, Paste, Delete, Undo, Redo, Play/Pause, Export (Cancel export while exporting). When the window is too narrow, the buttons that do not fit go into a "»" menu at the right end. Acceptance: at 800 pixels wide all 12 commands can be triggered.
- REQ-025 (implemented): Every operation is reachable from the menus; the common ones are also on the toolbar, in the sections or in right-click menus; keyboard shortcuts follow REQ-071.
- REQ-071 (implemented): The existing shortcuts are kept (Ctrl/Cmd+X/C/V/Z, Space, Ctrl/Cmd+B, Ctrl/Cmd+S, Ctrl/Cmd+Q, Ctrl/Cmd+, for Settings). New ones (Ctrl on Windows and Linux, Cmd on macOS): N new project, O open, Shift+S save as, I import, E export; Delete or Backspace delete; Left/Right one frame back/forward; Home/End go to start/end; Ctrl/Cmd+= and Ctrl/Cmd+- zoom in/out, Ctrl/Cmd+0 fit project; F1 opens the keyboard shortcuts window; Esc closes the topmost popup. Shortcuts never fire while typing in a text field or while a blocking dialog is open. Menus, tooltips and the shortcuts window use the same list as the actual bindings. Acceptance: a test per binding; the shortcuts window lists exactly the bound shortcuts.
- REQ-072 (implemented): Every toolbar button, timeline marker, "Add fade" handle, section border and icon-like control has a tooltip saying what it does and its shortcut. Greyed-out controls give a short reason, at least for Split, Merge, Paste, Undo, Redo, Export, Generate subtitles and Add music (for example "Select a clip first", "Nothing to undo"). Acceptance: the tooltip text exists for each listed control and each listed disabled reason.
- REQ-073 (implemented): A status bar at the bottom, with a fixed height, shows on the left the latest status message; in the middle the background work with progress: imports being analysed (count), preview copies, speech file download, subtitle generation (with Cancel) and export (with Cancel); on the right "Output: …" (previously in the effects panel). Acceptance: during an export the status bar shows its progress and a Cancel button; Cancel behaves as REQ-021 (no partial file).
- REQ-074 (implemented): The Undo and Redo menu items and toolbar tooltips name the action: "Undo <action>" / "Redo <action>". Acceptance: after a split the menu shows "Undo Split"; after undoing it shows "Redo Split".

### Effects
- REQ-075 (implemented): The Effects section is a vertical list of the effect types Fade through black, Subtitles and Volume. Since 1.2.0 each row shows only the effect name, with the tooltip "Drag onto a clip to apply it to that clip; double-click to apply it to the whole video". Clicking a row opens that effect's popup; double-clicking applies it to the whole video (REQ-082); rows can be dragged onto video clips. The list scrolls if needed. Clip details, music clip details, export and the shortcut list are not in this section. (In 1.1.0 each row also showed a one-line description and a count.) Acceptance: the three rows' text is exactly the effect names; clicking each opens its popup; the tooltip exists.
- REQ-082 (implemented): Each effect (Fade through black, Subtitles, Volume) can be applied **to a video clip**, by dragging it from the Effects list onto the clip or by right-clicking the clip and choosing Effects ▸ <effect>, or **to the whole video**, by double-clicking it in the Effects list (the whole video counts as one clip from the start to the end of the timeline). A single click still opens the effect's popup (REQ-076). Each application opens the popup of the instance created or changed and is one undo step; for Volume the undo step is recorded when a value is changed in the popup that opens (slider release or mute), not when the popup opens. Dropping an effect anywhere other than a video clip (for example on the music track or empty space) does nothing, and while dragging the pointer shows whether a drop is possible. Acceptance: (a) for each effect, dropping it on clip 2 of 3 gives the same result as the right-click menu, and one undo restores the previous state (for Volume, one undo restores the changed level); (b) double-clicking gives the whole-video result (REQ-083, REQ-084, REQ-085) and one undo restores the previous state; (c) dropping on the music track or on empty space changes nothing.
- REQ-076 (implemented): Effect popups are floating windows inside videolore (like Settings), not blocking: editing and playback continue while one is open. There is one per effect type; each can be moved and closed with its close button or Esc. Changes apply immediately, show in the preview and are each one undo step.
  - Fades: explains the effect (fixed length, REQ-011; video and audio); "Add fade at the boundary nearest the playhead"; list of all fades with their time (clicking one moves the playhead there and opens that fade's popup); "Remove all fades" (one undo step).
  - Subtitles: style, language, Generate / Cancel with progress and speech file download progress, "Add line at playhead", and a scrollable list of lines (clicking one opens that line's popup). REQ-014 to REQ-018, REQ-044 and REQ-052 to REQ-054 are unchanged.
  - Volume: video sound and music sliders and mute switches (REQ-020 unchanged; one undo step per slider release).
  - Acceptance: changing the subtitle style changes the preview and undo restores it; with 3 fades, "Remove all fades" leaves 0 and one undo brings back 3; playback continues while a popup is open.
- REQ-077 (implemented): Every fade and every subtitle line has a marker on the timeline at its position; since 1.2.0 so do the fade from black at the start of the video, the fade to black at its end (REQ-083), and every video clip whose own volume is not 100% or is muted (marker at the clip start, REQ-084). Markers differ by type (shape and colour); the tooltip gives the type, the time and, for subtitles, the start of the line's text. Clicking a marker selects that instance, moves the playhead to its start, highlights it and opens its popup:
  - Fade between clips: the two clips it sits between, its time, and Remove. Start or end fade: its time and Remove.
  - Clip volume: that clip's volume slider and mute switch.
  - Subtitle line: a text field, start and end adjustable by ±0.1 s, Delete, "Play this line" and (since 1.2.0) "Reset position" (REQ-089). Ctrl+click (Cmd+click on macOS) on a line marker adds it to or removes it from the selection instead (REQ-090).
  - A clip boundary without a fade shows a small "Add fade" handle; clicking it adds a fade (one undo step). Clicking an existing fade marker opens its popup (it no longer removes the fade directly).
  - When several markers overlap under the pointer, a small chooser lists them all.
  - Markers follow edits (ripple, REQ-018) and zoom.
  - Acceptance: (a) a marker's position equals its instance's start within 1 pixel at minimum, middle and maximum zoom; (b) clicking a fade marker opens the popup for that boundary, Remove makes the marker disappear and undo brings it back; (c) clicking a line marker opens the popup with that line's text, and editing it can be undone; (d) after cutting a clip its line markers are gone; (e) with overlapping markers the chooser lists all of them.

### Timeline ruler
- REQ-093 (implemented): The timeline ruler labels depend on the time at the right edge of the visible part of the timeline: up to 90 seconds, plain seconds ("45s"); up to 90 minutes, minutes and seconds `mm:ss` (minutes may go past 59, for example "75:30"); beyond that, `hh:mm:ss`. Ticks fall on round steps (1, 2, 5, 10, 15, 30 seconds; 1, 2, 5, 10, 15, 30 minutes; 1 hour and more) chosen so that labels never overlap. Acceptance: (a) with 0–60 s visible the labels read "Ns"; (b) with 0–5 min visible, or scrolled to 100–110 s, they read `mm:ss`; (c) with 0–2 h visible they read `hh:mm:ss`; (d) labels do not overlap at three zoom levels including the minimum and maximum.

### Clips, context menus, recent projects, drag and drop
- REQ-078 (implemented): A Clip details popup opens by double-clicking a clip (video or music track), from Edit > Clip details… or from the right-click menu. For a video clip: source, in/out/length, trim by ±1 frame or "at playhead", fade in/out, Merge and (since 1.2.0) the clip's volume slider and mute switch (REQ-084). For a music clip: start/end trim, Move to playhead, Split and Delete. It does not block the editor, closes with Esc and follows the selection. Acceptance: its trim buttons give the same results as the REQ-009 and REQ-019 tests.
- REQ-079 (implemented): Right-click menus: on a video clip Split here, Cut, Copy, Paste after, Delete, Merge with next, Add/remove fade before/after, Clip details… and (since 1.2.0) Effects ▸ Fade through black, Subtitles, Volume (REQ-082); on a music clip Split, Delete, Move to playhead, Clip details…; on a marker Open settings and Remove/Delete, plus "Reset position" on a subtitle line marker (since 1.2.0); on the empty timeline Paste and Add subtitle line here. Media items keep their existing menu. Entries are greyed out when their command is unavailable.
- REQ-080 (implemented): File > Open recent lists up to 10 projects opened or saved, most recent first, by file name with the full path in a tooltip, plus "Clear list". Choosing one goes through the unsaved-changes prompt (REQ-028). A missing file gives a clear message and is removed from the list. The list is a program setting, never stored in a project. Acceptance: after opening 12 projects the 10 most recent are listed in order; a deleted file gives the message and is removed; Clear list empties it; a damaged list is reset without a crash (REQ-057).
- REQ-081 (implemented): On Windows, macOS and Linux X11, dropping media files onto the window imports them with the usual rules (REQ-002, REQ-032). Dropping a `.vlproj` project opens it after the unsaved-changes prompt (REQ-028); if the dropped files include one or more projects, only the first project is opened and a status message says the other files were ignored. The empty media list reads "Import media… or drop files here". On Linux Wayland sessions drag and drop is not available (known limitation); File > Import media… and File > Open… work there as usual. Acceptance: dropping 3 videos and 1 unsupported file imports the 3 and shows 1 rejection; dropping a project while there are unsaved changes shows the prompt.

## Key decisions
- Sections can be resized but not moved, docked, hidden or detached; this reverses the 0.x "fixed layout" decision only for resizing, to keep the window simple.
- The layout and the recent-projects list are program settings, never part of a project (like every other setting). Older settings files load without any notice; old 1.0 builds drop these two values when they save their settings (accepted).
- Effect and clip popups do not block the editor, so the user can keep playing and editing while one is open.
- Clicking a fade marker opens its settings instead of removing it, so a fade is never removed by accident; a separate "Add fade" handle adds one.
- Only the existing effects are listed (Fade through black, Subtitles, Volume). The whole-video Volume has no timeline marker; since 1.2.0 a clip with its own volume (not 100% or muted) has one.
- The command bar of 0.x is replaced by the toolbar and the status bar; the zoom slider moves to the Timeline header.
- Settings… is in the Edit menu, and Ctrl/Cmd+, still opens it.
- When a drop contains several projects, the first one wins and the other files are ignored, with a message.
- The menu bar is drawn inside the window on every operating system, also on macOS (see the macOS limitation in the index).

## Impact on existing features
- REQ-011: clicking a fade on the timeline opens its popup; a fade is removed from the popup or the right-click menu.
- REQ-012: the zoom slider is in the Timeline header; View > Zoom in / Zoom out / Fit project and their shortcuts also zoom.
- REQ-016 and REQ-044: subtitle lines are edited in the Subtitles popup and in each line's popup; the rules are unchanged.
- REQ-020: the volume controls are in the Volume popup.
- REQ-009 and REQ-019: trim and music clip controls are in the Clip details popup.
- REQ-021: export progress and Cancel are in the status bar; Export is in the File menu and on the toolbar.
- REQ-028: also applies to Open recent and to a dropped project.
- REQ-031 and REQ-046: About is opened from the Help menu, Settings from the Edit menu.
- REQ-056: responsiveness also covers dragging section borders and open popups.
- REQ-057: the damaged-settings test set also covers layout values and the recent-projects list.
- The project file format does not change in 1.1.0 (REQ-029 unaffected). It changes in 1.2.0 (REQ-094).

### Since 1.2.0
- REQ-071: the keyboard shortcuts window also lists the gestures Ctrl/Cmd+click (add a subtitle line to the selection) and Shift+drag on the timeline (select lines in a time range); Esc first closes the topmost popup and, when none is open, clears the subtitle line selection.
- REQ-072: new greyed-out reasons: "No subtitles on the timeline", "Subtitles are being created", "Select a clip first", "The clips have different volumes".
- REQ-073: the status bar also shows checking videos for subtitle tracks and "Matching subtitles to the speech", with Cancel.
- REQ-076: popups say whether they show the whole-video instance or a clip's ("— whole video" or "— <clip name>"); new popups for a clip's volume and for the start and end fades.
- REQ-068: the new dialog, menu items and popup fields fit at 800x500.

## Known limitations
- Drag and drop does not work in Linux Wayland sessions (the windowing library used by videolore does not report dropped files there); use File > Import media… and File > Open….
- Popup positions are kept only until videolore is closed.
- The window size and position are not remembered.
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text; Paste in the Edit menu and the toolbar always works.

## Out of scope
- Moving, docking, hiding, collapsing or detaching sections; saved layouts or workspaces; resizing the menu bar, toolbar or status bar.
- A customisable toolbar; changing keyboard shortcuts.
- New effect types (titles or text overlays, speed, colour, other transitions) and per-fade length (fade length stays fixed); fades on one side of a boundary only.
- Effects on music clips (dropping an effect on the music track does nothing).
- Dragging markers to re-time lines or move fades; dragging clip edges to trim.
- Ruler labels below one second.
- A native macOS menu bar.
- Themes or light mode, interface text size, translation.
- Autosave and crash recovery.
