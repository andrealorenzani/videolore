# Business — Window, menus and effects

Back to the [index](business.md).

The window has a menu bar, a toolbar, four sections (Media, Preview, Timeline, Effects) and a status bar. Effects, their popups and markers are in [Effects](business-effects.md).

## Features
- **Window**: from top to bottom, menu bar, toolbar, the four sections (each with a title header) and a status bar.
- **Resizable sections**: drag the borders; sizes are remembered; View > Reset layout restores the defaults.
- **Everything reachable**: at 800x500 every control is visible or reachable by a scrollbar; long names end in "…" with the full name in a tooltip.
- **Menus**: File, Edit, Playback, Effects, View, Help; each item shows its shortcut and is greyed out when unavailable.
- **Toolbar**: the common commands in one row; the rest goes into a "»" menu when narrow.
- **Status bar**: latest message, background work with progress and Cancel where possible, and the output folder.
- **Effects section**: a list of effect names; clicking one opens its popup.
- **Applying effects**: drag an effect onto a video clip, or right-click the clip and choose Effects ▸, to apply it to that clip; double-click it in the list to apply it to the whole video (a copy on each clip, see [Effects](business-effects.md)).
- **Markers on the timeline**: each fade, subtitle line and clip with its own volume, Clean audio, Scale, Move or LUT has a marker; clicking it opens that item's settings. Clip boundaries without a fade show an "Add fade" handle.
- **Clip details**: double-click a clip to trim, set fades, volume, mute or merge (video), or trim, move, split and delete (music).
- **Recent projects**: File > Open recent lists the last 10 projects.
- **Drag and drop**: media files import; a project opens. Not available on Linux Wayland.
- **Undo names the action**: "Undo Split", "Redo Remove fade".

## Requirements

### Layout
- REQ-024 (implemented): The window shows, from top to bottom, a menu bar, a toolbar, the content area and a status bar. By default the top row takes 2/3 of the content height (Media 30% of the width, Preview 70%) and the bottom row 1/3 (Timeline 70%, Effects 30%). Each section has a title header; the Timeline header holds the zoom slider and a "Fit" button. These defaults apply at first start and after View > Reset layout. Acceptance: with no saved layout, at 800x500, 1280x800 and 2560x1440 the proportions hold within 1 pixel, sections do not overlap and the content area is fully covered.
- REQ-066 (implemented): Sections can be resized by dragging three borders: Media | Preview, Timeline | Effects, and top row | bottom row. The pointer shows a resize cursor over a border; the sections update live while dragging and only the two sections next to the border change. Minimum sizes: Media and Effects 160 pixels wide, Preview and Timeline 240 pixels wide, each row 100 pixels high; a border stops there. Resizing the window keeps the current proportions. The menu bar, toolbar and status bar keep a fixed height that fits their content. Acceptance: (a) dragging each border by ±100 pixels resizes exactly its two neighbours; (b) dragging past a minimum stops at the minimum; (c) after resizing the window the proportions hold within 1 pixel; (d) while an export and subtitle generation run, every frame during a border drag takes at most 200 ms (REQ-056).
- REQ-067 (implemented): Section sizes are a program setting (never stored in a project) and are restored at the next start. View > Reset layout restores the REQ-024 defaults. If a saved layout does not fit the current window, the minimum sizes win. Missing or damaged layout values give the default layout without a crash (REQ-057), keeping the other settings. Opening a project never changes the layout. Acceptance: resize, restart, same proportions; Reset layout gives REQ-024; a damaged or out-of-range value gives the default and the other settings are kept.
- REQ-068 (implemented): At 800x500, with any section at its minimum size, every control and text is visible or reachable with a scrollbar: every section, every effect popup, the clip details popup, Settings, dialogs and the keyboard shortcuts window. Scrollbars appear only when the content does not fit. The preview always keeps its play controls and seek bar visible; the picture shrinks first. The timeline scrolls vertically when its tracks do not fit at minimum height. Long names are shortened with "…" and shown in full in a tooltip. Acceptance: an interface test at 800x500 with every section at its minimum, 50 media items, 200 subtitle lines and a long file name finds every interactive control inside the visible area or inside a scroll area.

### Menus, toolbar, shortcuts and status bar
- REQ-069 (implemented): A menu bar has the menus File, Edit, Playback, Effects, View and Help. Each item shows its keyboard shortcut and is greyed out exactly when its command is unavailable. Acceptance: every item runs its command, its enabled state matches the command's availability, and an open menu scrolls if it does not fit in the window.
  - File: New, Open…, Open recent ▸, Save, Save as…, Import media…, Export… (opens the Export dialog, REQ-118; Cancel export while exporting), Export subtitles (.srt)… (REQ-088), Quit.
  - Edit: Undo, Redo, Cut, Copy, Paste, Delete, Split at playhead, Merge, Move left, Move right, Add selected media to timeline, Add selected media to music track, Clip details…, Settings…, Generate subtitles, Add subtitle line at playhead, Add/remove fade at playhead, Align horizontally, Align vertically, Reset and Reset subtitle position.
  - Playback: Play/Pause, Go to start, Go to end, Previous frame, Next frame, Set POV (REQ-102, on a 360 clip).
  - Effects (only effects): Fades…, Subtitles…, Volume…, Clean audio…, Add text at playhead, Change aspect…, Scale…, Move…, Apply LUT…, "Apply to selected clip ▸" and "Apply to whole video ▸" (Fade IN/OUT, Subtitles, Volume, Clean audio, Scale, Move, Apply LUT; Change aspect only for the whole video).
  - View: Zoom in, Zoom out, Fit project, Reset layout.
  - Help: Keyboard shortcuts, About videolore and third-party notices.
- REQ-070 (implemented): A toolbar shows in one row: Import, Add to timeline, Add music, Split, Cut, Copy, Paste, Delete, Undo, Redo, Play/Pause, Export (Cancel export while exporting). When the window is too narrow, the buttons that do not fit go into a "»" menu at the right end. Acceptance: at 800 pixels wide all 12 commands can be triggered.
- REQ-025 (implemented): Every operation is reachable from the menus; the common ones are also on the toolbar, in the sections or in right-click menus; keyboard shortcuts follow REQ-071.
- REQ-071 (implemented): The existing shortcuts are kept (Ctrl/Cmd+X/C/V/Z, Space, Ctrl/Cmd+B, Ctrl/Cmd+S, Ctrl/Cmd+Q, Ctrl/Cmd+, for Settings). New ones (Ctrl on Windows and Linux, Cmd on macOS): N new project, O open, Shift+S save as, I import, E export; Delete or Backspace delete; Left/Right one frame back/forward; Home/End go to start/end; Ctrl/Cmd+= and Ctrl/Cmd+- zoom in/out, Ctrl/Cmd+0 fit project; F1 opens the keyboard shortcuts window; Esc closes the topmost popup and, when none is open, clears the subtitle line selection. Ctrl/Cmd+click adds a subtitle line to the selection; Shift+drag on the timeline selects lines in a time range; Shift+click on the picture and the timeline adds to the selection; Delete removes a selected text item. Shortcuts never fire while typing in a text field or while a blocking dialog is open. Menus, tooltips and the shortcuts window use the same list as the actual bindings. Acceptance: a test per binding; the shortcuts window lists exactly the bound shortcuts.
- REQ-072 (implemented): Every toolbar button, timeline marker, "Add fade" handle, section border and icon-like control has a tooltip saying what it does and its shortcut. Greyed-out controls give a short reason, at least for Split, Merge, Paste, Undo, Redo, Export, Generate subtitles and Add music (for example "Select a clip first", "Nothing to undo", "No subtitles on the timeline", "Subtitles are being created", "The clips have different volumes"). Acceptance: the tooltip text exists for each listed control and each listed disabled reason.
- REQ-073 (implemented): A status bar at the bottom, with a fixed height, shows on the left the latest status message; in the middle the background work with progress: imports being analysed (count), preview copies, speech file download, checking videos for subtitle tracks, "Matching subtitles to the speech", subtitle generation (with Cancel) and export (with Cancel); on the right "Output: …". Acceptance: during an export the status bar shows its progress and a Cancel button; Cancel behaves as REQ-021 (no partial file).
- REQ-074 (implemented): The Undo and Redo menu items and toolbar tooltips name the action: "Undo <action>" / "Redo <action>". Acceptance: after a split the menu shows "Undo Split"; after undoing it shows "Redo Split".

### Effects
- REQ-075, REQ-107, REQ-082, REQ-147, REQ-076, REQ-077 (the Effects list, its names and tooltips, applying effects to a clip or to the whole video, the effect popups and the markers on the timeline) are in [Effects](business-effects.md).
- REQ-076 (implemented): Only Change aspect says "— whole video" in its popup title; Fades, Volume and Clean audio popups have no suffix; the LUT chooser says "Apply LUT — every clip". Whole-video copies open their clips' popups (REQ-147).

### Timeline ruler
- REQ-093 (implemented): The timeline ruler labels depend on the time at the right edge of the visible part of the timeline: up to 90 seconds, plain seconds ("45s"); up to 90 minutes, minutes and seconds `mm:ss` (minutes may go past 59, for example "75:30"); beyond that, `hh:mm:ss`. Ticks fall on round steps (1, 2, 5, 10, 15, 30 seconds; 1, 2, 5, 10, 15, 30 minutes; 1 hour and more) chosen so that labels never overlap. Acceptance: (a) with 0–60 s visible the labels read "Ns"; (b) with 0–5 min visible, or scrolled to 100–110 s, they read `mm:ss`; (c) with 0–2 h visible they read `hh:mm:ss`; (d) labels do not overlap at three zoom levels including the minimum and maximum.

### Clips, context menus, recent projects, drag and drop
- REQ-078 (implemented): A Clip details popup opens by double-clicking a clip (video or music track), from Edit > Clip details… or from the right-click menu. For a video clip: source, in/out/length, trim by ±1 frame or "at playhead", fade in/out, Merge, the clip's volume slider and mute switch (REQ-084). For a music clip: start/end trim, Move to playhead, Split and Delete. It does not block the editor, closes with Esc and follows the selection. Acceptance: its trim buttons give the same results as the REQ-009 and REQ-019 tests.
- REQ-079 (implemented): Right-click menus: on a video clip Split here, Cut, Copy, Paste after, Delete, Merge with next, Add/remove fade before/after, Clip details… and Effects ▸ (Fade IN/OUT, Subtitles, Volume, Clean audio, Scale, Move, Apply LUT; Change aspect for the whole video only); on a music clip Split, Delete, Move to playhead, Clip details…; on a marker Open settings and Remove/Delete, plus "Reset position" on a subtitle line marker; on the empty timeline Paste and Add subtitle line here; on a selected text item the Context Menu (REQ-106). Media items keep their menu. Entries are greyed out when their command is unavailable.
- REQ-080 (implemented): File > Open recent lists up to 10 projects opened or saved, most recent first, by file name with the full path in a tooltip, plus "Clear list". Choosing one goes through the unsaved-changes prompt (REQ-028). A missing file gives a clear message and is removed from the list. The list is a program setting, never stored in a project. Acceptance: after opening 12 projects the 10 most recent are listed in order; a deleted file gives the message and is removed; Clear list empties it; a damaged list is reset without a crash (REQ-057).
- REQ-081 (implemented): On Windows, macOS and Linux X11, dropping media files onto the window imports them with the usual rules (REQ-002, REQ-032). Dropping a `.vlproj` project opens it after the unsaved-changes prompt (REQ-028); if the dropped files include one or more projects, only the first project is opened and a status message says the other files were ignored. The empty media list reads "Import media… or drop files here". On Linux Wayland sessions drag and drop is not available (known limitation); File > Import media… and File > Open… work there. Acceptance: dropping 3 videos and 1 unsupported file imports the 3 and shows 1 rejection; dropping a project while there are unsaved changes shows the prompt.

## Key decisions
- Sections can be resized but not moved, docked, hidden or detached (the 0.x fixed layout changed only for resizing). Docking and hiding came in 2.0.4 (see [Interface](business-interface.md)).
- The layout and the recent-projects list are program settings, never part of a project. Versions before 1.1.0 drop them when they save their settings (accepted).
- Effect and clip popups do not block the editor.
- Clicking a fade marker opens its settings instead of removing it; a separate "Add fade" handle adds one.
- The Effects menu holds only effects; commands on subtitles, fades or picture items are in the Edit menu. The whole-video Volume has no timeline marker; a clip with its own volume (not 100% or muted) has one.
- Settings… is in the Edit menu; Ctrl/Cmd+, still opens it.
- When a drop contains several projects, the first wins and the others are ignored, with a message.
- The menu bar is drawn inside the window on every operating system, also on macOS.
- Only the effects of the Effects list are offered; the list grew by the effects named in REQ-075 (Add Text 1.3.0, Clean audio 1.4.0, Change aspect, Scale, Move 1.5.0, Apply LUT 1.6.0).

## Known limitations
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open….
- Popup positions are kept only until videolore is closed (remembered from 2.0.4 per [Interface](business-interface.md)).
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text; Paste in the Edit menu and the toolbar always works.

## Out of scope
- Free movement or detaching of sections; resizing the menu bar, toolbar or status bar.
- Ruler labels below one second (open in the [roadmap](roadmap.md)).
- Effects on music clips and new effect types (see [Effects and frame options](business-effects-frame.md)).
