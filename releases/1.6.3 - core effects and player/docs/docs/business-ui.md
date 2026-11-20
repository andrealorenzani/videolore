# Business — Window, menus and effects

Back to the [index](business.md).

Since 1.1.0 the window is reorganised (resizable sections, menu bar, toolbar, status bar, one popup per effect, markers on the timeline). Since 1.2.0 an effect applies to one video clip or to the whole video, and the ruler switches unit with the visible range. In 1.1.0 nothing changes in how projects are saved: the markers show data the project already contains. The project format changed in 1.2.0 (see [Editing](business-editing.md), REQ-094). In 1.3.0: "Fade through black" is renamed "Fade IN/OUT", the Effects list gets "Add Text", and the selection works across the preview and the timeline. In 1.4.0: the Effects list gets "Clean audio", the Effects menu lists only the effects, and File > Export opens the Export dialog. In 1.6.0: the Effects list and menus get "Apply LUT", and the interface gets icons (see [Icons](business-icons.md)). The effects themselves (the Effects list, the popups and the markers) are in [Effects](business-effects.md).

## Features
- **Window**: from top to bottom, a menu bar, a toolbar, the four sections (Media, Preview, Timeline, Effects) and a status bar. Each section has a title header.
- **Resizable sections**: the borders between Media and Preview, between Timeline and Effects, and between the top and bottom rows can be dragged. Sizes are remembered for the next start; View > Reset layout restores the defaults.
- **Everything reachable**: at the smallest window size (800x500) every control is visible or reachable with a scrollbar; scrollbars appear only when needed; long names are shortened with "…" and shown in full in a tooltip.
- **Menus**: File, Edit, Playback, Effects, View, Help, each item with its keyboard shortcut and greyed out when it cannot be used.
- **Toolbar**: the most common commands in one row; at narrow widths the rest goes into a "»" menu.
- **Status bar**: the latest message, background work with progress (imports, preview copies, speech file download, subtitle generation, export, with Cancel where possible) and the output folder.
- **Effects section**: a list of the effect names (Fade IN/OUT, Subtitles, Volume; since 1.3.0 also "Add Text"; since 1.4.0 also "Clean audio"; since 1.6.0 also "Apply LUT"); clicking one opens its settings popup.
- **Applying effects** (1.2.0): drag an effect from the Effects list onto a video clip, or right-click the clip and choose Effects ▸, to apply it to that clip; double-click it in the list to apply it to the whole video.
- **Markers on the timeline**: every fade (between clips, at the start and at the end of the video), every subtitle line and every clip with its own volume or Clean audio has a marker; clicking it opens the settings of that single item. Clip boundaries without a fade show a small "Add fade" handle.
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
  - File: New, Open…, Open recent ▸, Save, Save as…, Import media…, Export… (opens the Export dialog, REQ-118; Cancel export while exporting), Export subtitles (.srt)… (since 1.2.0, REQ-088), Quit.
  - Edit: Undo, Redo, Cut, Copy, Paste, Delete, Split at playhead, Merge, Move left, Move right, Add selected media to timeline, Add selected media to music track, Clip details…, Settings…; since 1.4.0 also Generate subtitles, Add subtitle line at playhead, Add/remove fade at playhead, Align horizontally, Align vertically, Reset and Reset subtitle position (moved from the Effects menu).
  - Playback: Play/Pause, Go to start, Go to end, Previous frame, Next frame, Set POV (REQ-102, on a 360 clip; the only menu place of Set POV since 1.4.0).
  - Effects (since 1.4.0 only effects): Fades…, Subtitles…, Volume…, Clean audio…, Add text at playhead (1.3.0); since 1.5.0 also Change aspect…, Scale… and Move…; since 1.6.0 also Apply LUT…; "Apply to selected clip ▸" and "Apply to whole video ▸" (each with Fade IN/OUT, Subtitles, Volume, Clean audio, Scale, Move, Apply LUT; Change aspect only for the whole video).
  - View: Zoom in, Zoom out, Fit project, Reset layout.
  - Help: Keyboard shortcuts, About videolore and third-party notices.
- REQ-070 (implemented): A toolbar (replacing the 0.x command bar) shows in one row: Import, Add to timeline, Add music, Split, Cut, Copy, Paste, Delete, Undo, Redo, Play/Pause, Export (Cancel export while exporting). When the window is too narrow, the buttons that do not fit go into a "»" menu at the right end. Acceptance: at 800 pixels wide all 12 commands can be triggered.
- REQ-025 (implemented): Every operation is reachable from the menus; the common ones are also on the toolbar, in the sections or in right-click menus; keyboard shortcuts follow REQ-071.
- REQ-071 (implemented): The existing shortcuts are kept (Ctrl/Cmd+X/C/V/Z, Space, Ctrl/Cmd+B, Ctrl/Cmd+S, Ctrl/Cmd+Q, Ctrl/Cmd+, for Settings). New ones (Ctrl on Windows and Linux, Cmd on macOS): N new project, O open, Shift+S save as, I import, E export; Delete or Backspace delete; Left/Right one frame back/forward; Home/End go to start/end; Ctrl/Cmd+= and Ctrl/Cmd+- zoom in/out, Ctrl/Cmd+0 fit project; F1 opens the keyboard shortcuts window; Esc closes the topmost popup. Shortcuts never fire while typing in a text field or while a blocking dialog is open. Menus, tooltips and the shortcuts window use the same list as the actual bindings. Acceptance: a test per binding; the shortcuts window lists exactly the bound shortcuts.
- REQ-072 (implemented): Every toolbar button, timeline marker, "Add fade" handle, section border and icon-like control has a tooltip saying what it does and its shortcut. Greyed-out controls give a short reason, at least for Split, Merge, Paste, Undo, Redo, Export, Generate subtitles and Add music (for example "Select a clip first", "Nothing to undo"). Acceptance: the tooltip text exists for each listed control and each listed disabled reason.
- REQ-073 (implemented): A status bar at the bottom, with a fixed height, shows on the left the latest status message; in the middle the background work with progress: imports being analysed (count), preview copies, speech file download, subtitle generation (with Cancel) and export (with Cancel); on the right "Output: …" (previously in the effects panel). Acceptance: during an export the status bar shows its progress and a Cancel button; Cancel behaves as REQ-021 (no partial file).
- REQ-074 (implemented): The Undo and Redo menu items and toolbar tooltips name the action: "Undo <action>" / "Redo <action>". Acceptance: after a split the menu shows "Undo Split"; after undoing it shows "Redo Split".

### Effects
- REQ-075, REQ-107, REQ-082, REQ-147, REQ-076, REQ-077 (the Effects list, its names and tooltips, applying effects to a clip or to the whole video, the effect popups and the markers on the timeline) are in [Effects](business-effects.md).

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
- Only the existing effects are listed (Fade IN/OUT, Subtitles, Volume), plus "Add Text" since 1.3.0, "Clean audio" since 1.4.0, and Change aspect, Scale, Move (1.5.0) and Apply LUT (1.6.0).
- (1.4.0) The Effects menu holds only effects. Commands that act on subtitles, fades or picture items are in the Edit menu, so the Effects menu is a plain list of what can be added. The whole-video Volume has no timeline marker; since 1.2.0 a clip with its own volume (not 100% or muted) has one.
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

### Changed in 1.3.0
- REQ-107: "Fade through black" is "Fade IN/OUT" in the Effects list, menus (Effects ▸ and right-click ▸ entries), popups and tooltips.
- REQ-075: the Effects list gets "Add Text"; the popup and the timeline lane are in [Text](business-text.md) (REQ-108 to REQ-111).
- REQ-082 (drag and double-click) and REQ-079 (Effects ▸ entries): the three effects are unchanged apart from the name; "Add Text" is not applied to clips.
- REQ-077: text items are bars on a Text lane of the timeline, not markers; their popup opens from the bar (REQ-110).
- REQ-106 (Context Menu, see [Boxes on the picture and 360 video](business-picture.md)): the toolbar group is shown only with a selection or a 360 clip under the playhead.
- REQ-090 to REQ-092: a selection stays while the selected boxes are dragged in the preview; Shift+click adds to the selection like Ctrl/Cmd+click; a Shift+drag on the timeline keeps the selection (see [Subtitles](business-subtitles.md)).
- REQ-071: Delete removes a selected text item; the shortcuts window lists Shift+click on the picture and the timeline.

### Changed in 1.4.0
- REQ-069 and REQ-075: the Effects menu lists only effects (see Menus above); the Effects list gets the "Clean audio" row (REQ-114). Align horizontally, Align vertically, Reset and Reset subtitle position moved from the Effects menu to the Edit menu, and the Context Menu of REQ-106 is the other place for them.
- REQ-118: File > Export…, the Export toolbar button and Ctrl/Cmd+E open the Export dialog (see [Export](business-export.md)); during an export they still cancel it (REQ-021).
- REQ-114: Clean audio can be dragged onto a clip, applied to the whole video, and is listed in the Effects ▸ menus (REQ-079, REQ-082) and shows a marker at the clip start (REQ-077).
- REQ-106 and REQ-115: the Context Menu also appears for a selected text item.
- REQ-100: Clip details has no "Treat as 360° video" switch.

### Changed in 1.5.0
- REQ-075 and REQ-069: the Effects list gets Change aspect, Scale and Move (REQ-130); the Effects menu lists them too.
- REQ-079 and REQ-082: a clip's right-click Effects ▸ and the drag from the Effects list include Scale and Move; Change aspect is for the whole video only.
- REQ-077: a clip with Scale or Move shows one marker at its start (as Clean audio); the marker tooltip gives the keyframe count.
- REQ-076: the Change aspect, Scale and Move popups follow the popup rules; the keyframe timeline of Scale and Move is in the popup (REQ-129, see [Frame](business-frame.md)).

### Changed in 1.6.0
- REQ-075 and REQ-069: the Effects list and the Effects menu get Apply LUT (REQ-135), after Move and before Add Text.
- REQ-079 and REQ-082: a clip's right-click Effects ▸ and the drag from the Effects list include Apply LUT; the whole-video Apply LUT is a double-click; Change aspect stays for the whole video only.
- REQ-077: a clip with Apply LUT shows one marker at its start, like Clean audio, Scale and Move; the tooltip names the LUT file.
- REQ-142 and REQ-143: icons on the toolbar, menus, Effects list, markers, popups and dialogs (see [Icons](business-icons.md)).

### Changed in 1.6.1
- REQ-082 and REQ-147: a double-click on an effect applies it to every clip on the timeline (not to one range over the whole video); clips added later do not get it. Whole-video Fade IN/OUT is a fade in at the first clip and a fade out at the last clip (REQ-083).
- A whole-video Scale or Move gives each clip its own copy of the keyframes, which that clip's popup edits alone; the whole-video popup changes all copies (REQ-148, see [Frame](business-frame.md)).
- REQ-145 and REQ-146: the visible edges of a box (the picture's Scale box, a subtitle line, a text item) work as resize handles when the box is partly outside the picture.

## Known limitations
- Drag and drop does not work in Linux Wayland sessions (the windowing library used by videolore does not report dropped files there); use File > Import media… and File > Open….
- Popup positions are kept only until videolore is closed.
- The window size and position are not remembered.
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings.
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text; Paste in the Edit menu and the toolbar always works.

## Out of scope
- Moving, docking, hiding, collapsing or detaching sections; saved layouts or workspaces; resizing the menu bar, toolbar or status bar.
- A customisable toolbar; changing keyboard shortcuts.
- New effect types (speed, colour other than Apply LUT, other transitions) and per-fade length (fade length stays fixed); fades on one side of a boundary only.
- Effects on music clips (dropping an effect on the music track does nothing).
- Dragging markers to re-time lines or move fades; dragging clip edges to trim.
- Ruler labels below one second.
- A native macOS menu bar.
- Themes or light mode, interface text size, translation.
- Autosave and crash recovery.
