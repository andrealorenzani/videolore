# Architecture: user interface

Part of [Architecture](architecture.md). Since 1.1.0 (requirements REQ-024, REQ-025 and REQ-066..081, see [business](business.md)).

## Window structure
From top to bottom: menu bar, toolbar, content area, status bar. The three bars have a fixed height set by their content. The content area holds four sections, each with a title header:

| Top row (2/3 of the height) | | Bottom row (1/3) | |
|---|---|---|---|
| Media (30% of width) | Preview (70%) | Timeline (70%) | Effects (30%) |

These are the defaults at first start and after **View > Reset layout**. Each frame is drawn in a fixed order: background results are collected, dropped files are handled, keyboard shortcuts are matched, then the bars, the sections and their borders, the popups, the dialogs. Every control produces a command that the application state runs, so menus, toolbar, shortcuts and context menus share the same behaviour.

## Resizable sections
- Three borders can be dragged: Media | Preview, Timeline | Effects, and top row | bottom row. Only the two neighbouring sections change; the cursor changes over a border and the border is highlighted.
- Sizes are stored as proportions, so resizing the window keeps them. Borders are placed on whole pixels and the sections plus the 6-pixel gaps between them cover the content area exactly (within 1 pixel of the proportions at 800x500, 1280x800 and 2560x1440).
- Minimum sizes: Media and Effects 160 px wide, Preview and Timeline 240 px wide, each row 100 px high. A border stops at the minimum. If the window is too small for the saved proportions, the minimums win while drawing, but the saved proportions are kept, so enlarging the window again restores them.
- The layout is a **program setting**, never stored in a project; opening a project does not change it. It is saved once when a drag ends, not while dragging.
- The layout code is a small pure module (no drawing), tested on its own.

**Why our own layout:** egui's resizable panels store absolute widths, cannot keep proportions when the window changes, do not limit a drag to the two neighbours, and keep their state outside our settings file. Docking libraries (`egui_dock`, `egui_tiles`) would add a crate for docking features that are out of scope.

## Everything reachable
- Every section body and every popup is scrollable; scrollbars appear only when the content does not fit. Popups are kept inside the window.
- In the preview the play controls and seek bar are placed first; the picture takes what is left and shrinks first.
- The timeline scrolls vertically when its rows (ruler, marker lane, video track, music track) do not fit, and horizontally as before.
- Long names are shortened with "…"; the full name is shown on hover.
- The smallest window stays 800x500. An automated test at that size, with every section at its minimum, 50 media items, 200 subtitle lines and a very long file name, opens every popup in turn and checks that every control is visible or inside a scroll area that can reach it.

## Menus, toolbar and shortcuts
- One **table of actions** lists every command with its label, shortcuts and tooltip. The menu bar (File, Edit, Playback, Effects, View, Help), the toolbar, tooltips, the Keyboard shortcuts window and the keyboard handling all read it, so they cannot disagree. A test checks that the shortcuts window lists exactly the bound shortcuts and that no shortcut is used twice.
- An item is greyed exactly when its command is unavailable; greyed toolbar buttons and menu items explain why on hover (for example "Select a clip first", "Nothing to undo").
- Undo and Redo name the action ("Undo Split"). The undo history stores a short name with each step.
- Shortcuts use Cmd on macOS and Ctrl elsewhere, shown in the system's style. They do nothing while typing in a text field or while a dialog that needs an answer is open. Esc closes the topmost popup.
- egui's own keyboard zoom (Ctrl/Cmd with +, - and 0 scaling the whole interface) is turned off, because those keys zoom the timeline.
- The toolbar shows the common commands as one row (12 up to 1.2.5; 15 since 1.2.6 with a separated group Align horizontally, Align vertically, Reset, see [boxes](architecture-boxes.md)). When the window is too narrow, the commands that do not fit move into a "»" menu at the right end, so all of them stay usable at 800 px.
- Menus are drawn inside the window on every system, including macOS, where the system menu bar stays off so that Cmd+Q asks about unsaved changes (see [project files](architecture-projects.md#unsaved-changes)). Very long menus scroll.
- **1.4.0:** the Effects menu lists only effects (Fade IN/OUT, Subtitles, Volume, Clean audio, Add text, and the Apply ▸ submenus); the item commands (Generate subtitles, Add subtitle line, Add or remove fade, Align horizontally and vertically, Reset, Reset subtitle position) are in the Edit menu, with the same shortcuts and reasons. File > Export… and the toolbar's Export button open the [export dialog](architecture-export-dialog.md); Ctrl/Cmd+E does the same. The Clean audio row is also in the Effects list, and the Effects ▸ and drag lists gain it.

## Status bar
Left: the latest message. Middle: background work with progress (files being analysed, preview copies, speech model download, subtitle generation with Cancel, export with Cancel). Right: the output folder. The list of background jobs is computed by a function of the application state, tested without a window.

## Effects and popups
- The Effects section lists the effect types (Fade through black, Subtitles, Volume). Since 1.2.0 each row shows only the name, with a tooltip explaining how to apply it: drag it onto a clip, or double-click it for the whole video. Clicking one opens its popup. See [effects](architecture-effects.md).
- Popups (Fades, Subtitles, Volume, Clip details, the selected fade or subtitle line, Keyboard shortcuts, Settings) are windows inside the main window, like Settings since 0.3.0. They are **not modal**: editing and playback continue while they are open. There is one popup per type; its position is kept for the session.
- Which popups are open, and which fade or subtitle line is selected, is part of the application state and changed only through commands. This makes the behaviour testable without drawing and lets menus, shortcuts, the timeline and the Effects list open the same popups.
- Each change in a popup is one undo step (a text edit when the field loses focus, a slider when released, "Remove all fades" as a whole). A popup for a fade or line that no longer exists (for example after a cut) closes.
- "Play this line" plays from the start of a subtitle line and pauses at its end.

## Timeline markers
- Each fade and each subtitle line has a marker in a lane above the video track: a diamond for a fade at its clip boundary, a flag with a thin bar for a subtitle line. A boundary without a fade shows a small "+" handle that adds one.
- Markers are computed from the edit every time it is drawn (nothing new is stored, so the project format does not change). They follow cuts, ripples and zoom, and disappear with the clip they belong to. Only markers in the visible part of the timeline are drawn.
- Clicking a marker selects it, moves the playhead there and opens its popup. When several markers are under the pointer, a small list lets the user choose.
- Since 1.2.0 there are also markers for the fade from black at the start and to black at the end, and for a clip whose volume is not 100% or is muted. Ctrl/Cmd+click on subtitle line markers selects several lines, and Shift+drag on the timeline selects the lines starting in a range (see [subtitles](architecture-subtitles.md#position-on-screen-120)). Dragging an effect from the Effects list onto a video clip applies it there. Since 1.2.6: a marker for each POV of a 360 clip, opening a popup with time, direction, zoom and Remove (see [360 video](architecture-360.md)).

## Time ruler (1.2.0)
- Labels depend on the time at the right edge of the visible part: up to 90 s in seconds ("45s"), up to 90 minutes as minutes and seconds ("01:40", minutes may go past 59, e.g. "75:30"), beyond that as hours, minutes and seconds ("01:05:00").
- Ticks fall on round steps (1, 2, 5, 10, 15, 30 s; 1, 2, 5, 10, 15, 30 min; 1 h and more). The step is the smallest one that leaves room for the widest label, so labels never overlap at any zoom; only visible ticks are computed. The rule is a small pure module, tested on its own.
- Double-clicking a clip opens Clip details; right-clicking a clip, a marker or the empty timeline opens a context menu with the same commands as the menus.

## Recent projects
Up to 10 projects, most recent first, are stored in the program settings after a successful open or save. A missing file shows a message and is removed from the list. Opening a recent project goes through the unsaved-changes prompt.

## Drag and drop
- Dropping media files on the window imports them exactly like File > Import. Dropping a project opens it after the unsaved-changes prompt; if several projects are dropped, only the first is opened and the status bar says the other files were ignored. Drops are ignored while a dialog needs an answer.
- **Known limitation:** drag and drop works on Windows, macOS and Linux with X11, but not in native Linux Wayland sessions, because the windowing library used by egui (winit 0.30) does not report dropped files there. File > Import… and File > Open… remain available. Forcing X11 on Linux was rejected because it would degrade every Wayland user (scaling, input methods). This will be revisited when egui moves to a winit release that supports it.

## Settings file
The settings file stays at version 2 with two new optional fields: the section proportions and the recent projects. Files written by 1.0 open without any notice. A damaged or out-of-range value falls back to its default and every other setting is kept. 1.0 can still read files written by 1.1, but drops these two fields when it saves. See [security](architecture-security.md#project-and-settings-files) for the validation rules.

## Testing the interface
The interface is tested headless (no window): egui's accessibility tree, which lists every control with its role, label, position and enabled state, is used to find controls, click them, drag borders and check that everything is reachable. Positions inside scroll areas are checked against a list of scroll areas recorded while drawing. If this helper grows too large, the `egui_kittest` test library (same egui release) will be used instead. Screen-reader support itself is not enabled in the application yet.

## Performance
- While an export and subtitle generation run, every frame stays under 200 ms, also while dragging a border or with popups open (automated test). The goal is under 16 ms per frame at 1280x800 with 50 media items and 200 subtitle lines (estimate, not a gate).
- Since 1.2.0 the same 200 ms limit is tested while dragging an effect onto a clip and a subtitle in the preview during an export and word matching.
- The layout is computed in constant time, markers outside the view are skipped, the subtitle list draws only visible rows, and the settings file is written only when a drag ends, the layout is reset or the recent list changes.

## Selection (1.3.0)
- Before 1.3.0 the window kept two selection states (one set of subtitle lines and one "selected line"); a plain click in the preview could lose a multi-selection. Since 1.3.0 there is **one selection** of objects (subtitle lines and text items) in window state, held by the pure module `selection`, not saved and not in the undo history.
- The rules are pure functions: a plain click selects only the object (unless it is already selected, so a drag can move the group); Ctrl/Cmd or Shift click toggles it; Shift+drag on the timeline selects the objects whose start is in the range; a plain click on empty space or Esc (when no popup is open) clears it. Every surface (timeline markers, Subtitles popup list, the preview boxes, the Text lane) sends the same events.
- Dragging a selected box in the preview moves the whole group to the dragged object's final place, as one undo step.

## Add Text popup (1.3.0)
- The Effects list's "Add Text" row (and Effects > Add text at playhead) opens a non-modal popup with the multiline text, the effects, time to live, colour and font (see [text items](architecture-text.md)). "Add" is one command and one undo step; Esc or Close creates nothing. The popup's list of installed fonts is filled by the background scan; while it runs the picker shows "Loading fonts…".
- Fade through black is labelled "Fade IN/OUT" everywhere in the interface (label change only, one label table; the internal name and the project format are unchanged).

## 2.0.4 additions (stages 2, 3, 7 and 8)
- **Interface (REQ-234 to REQ-247):** remembered window and popup places with an off-screen rule (`ui/place.rs`, `ui/view_menu.rs`); sections that hide, collapse and dock; workspaces; the toolbar and shortcut editors (`ui/keymap.rs`); a theme (Light, Dark or System, `ui/theme.rs`); text size through the egui text styles; all visible words in one English table (`ui/strings.rs`, a test rejects literal labels in `ui/*.rs`); accessibility names through eframe's AccessKit feature.
- **Gestures (REQ-189 to REQ-199):** a drag (trim, markers, text handles) is live while held and one command on release (`ui/gesture_ux_tests.rs`); the `[` and `]` keys are bound as listed in the shortcut table; guides and safe areas are part of the preview's view.
- **Whole video and keyframes (REQ-200 to REQ-205):** one row per clip in the whole-video popup; an offer for new clips is a status item with a button; keyframes can be copied and pasted; there is one keyframe widget for Scale and Move and no separate file for it.
- **One window (REQ-225 to REQ-233):** a second start writes a file to the inbox of the running window (`src/instance.rs`) and exits; the running window opens it through the usual open path.

## Risks
- egui changes its menu, popup and panel API between releases; the logic lives in small pure modules (layout, actions, markers, background jobs) so the drawing code stays thin.
- Global keys such as Delete, Backspace and the arrows must not fire while typing; this is checked by tests for each key.
- The menu bar inside the window looks less native on macOS, where Hide and Minimize remain unavailable.

## Icons, splash and the Apply LUT popup (1.6.0)
- Every toolbar button, menu item, Effects row, marker type and popup or dialog title gets an icon from one bundled font; the icon's command table is the same one that drives menus and shortcuts (see [icons](architecture-icons.md)).
- The splash opens at editor start as an in-app popup with the same closing rules (see [splash](architecture-splash.md)). Since 1.6.2 the window itself is shared with the player (`ui::splash_window`); the editor only adds its popup command.
- Apply LUT has a popup with Choose file, the file name (full path in a tooltip) and Remove effect (see [LUT](architecture-lut.md)).
