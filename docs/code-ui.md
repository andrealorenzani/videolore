# Code — the window (UI)

Back to the [index](code.md).

The interface is egui 0.37 in immediate mode: every frame `ui::frame_ui` draws the whole window
from `AppState` and collects `Command`s; they run through `AppState::dispatch` at the end of the
frame (greyed-out controls are never dispatched: `is_enabled` is checked again).

## Window structure

| Part | Module | Notes |
|---|---|---|
| Menu bar | `ui::menu` | `egui::MenuBar` in a top panel; File, Edit, Playback, Effects, View, Help; File > Open recent ▸ is a submenu; a menu taller than the window scrolls |
| Toolbar | `ui::toolbar` | 12 actions in one row; `toolbar::fit` decides how many fit, the rest goes in a "»" menu |
| Content area | `ui::layout` + `ui::mod` | Media, Preview, Timeline, Effects with three draggable borders |
| Status bar | `ui::status` | latest message, `AppState::background_jobs()` (label, progress, Cancel command), "Output: …" |
| Floating windows | `ui::popups` | Settings, Fades, Subtitles, Volume, Clip details, the selected fade or line, Keyboard shortcuts, the marker chooser |
| Dialogs | `ui::modals` | blocking `egui::Modal`s (ffmpeg missing, notices, unsaved changes, speech file consent, confirmations, About) |

Frame order: `tick` → dropped files → shortcuts → bars → sections and borders → floating windows
→ banner → dialogs → dispatch. egui's own keyboard zoom (`Options::zoom_with_keyboard`) is turned
off so that Ctrl/Cmd + `=` / `-` / `0` zoom the timeline.

## Layout

`ui::layout` is pure geometry. The layout is three shares (`settings::SavedLayout`, also called
`layout::Fractions`): top row of the content height, Media of the top row, Timeline of the bottom
row; default 2/3, 0.30, 0.70. Sections are separated by a 6 px gap that holds the border.

| Function | What it does |
|---|---|
| `sections(area, shares, MINS)` | section and border rectangles, whole pixels, covering the area exactly |
| `effective(shares, area, MINS)` | the shares actually shown once minimum sizes are applied |
| `drag(shares, border, delta, area, MINS)` | moves one border; only its two neighbours change; stops at their minimum |

Minimum sizes: Media and Effects 160 px wide, Preview and Timeline 240 px, each row 100 px. They
are applied only when drawing, so a small window does not change the stored shares and growing
it back restores the layout. When even the minimums do not fit, the space is split in proportion
to them (never negative, never a panic). A border drag is measured from where it started, kept in
`UiCache` while dragging and saved once on release (`Command::SetLayout`); View > Reset layout
removes the saved value.

Each section has a title header (the Timeline header holds the zoom slider and "Fit") and a body
in a scroll area whose bars appear only when needed. The Preview gives its controls room first;
the picture shrinks to nothing if needed. The Timeline scrolls in both directions: its content is
at least ruler + marker lane + video track (40 px) + music track (24 px) high.

## Actions, menus, shortcuts

`ui::actions::ACTIONS` is the single table of user actions (`ActionId`, label, shortcuts,
tooltip). `ActionId::command(&AppState)` gives the command to run now (Export becomes Cancel
export while exporting; Go to end seeks to the current duration), `label()` names the undo step
("Undo Split"). Menus, the toolbar, tooltips (`ui::action_tooltip`: text + shortcut), the
keyboard shortcuts window and the key bindings all read this table.

`ui::keymap::collect` matches the frame's key events against the table (Ctrl on Windows/Linux,
Cmd on macOS; Shift must match exactly, except Ctrl/Cmd + `=`/`+` with or without Shift) and
removes matched events before any widget sees them. Nothing fires while a text field has focus
or a blocking dialog is open. Esc closes the topmost floating window (or the marker chooser);
when an egui menu or combo box is open, egui closes it instead. egui turns Ctrl/Cmd+C/X/V into
Copy/Cut/Paste events, which map to the same actions.

| Shortcut | Action |
|---|---|
| Ctrl/Cmd+N, O, S, Shift+S, I, E, Q | New, Open, Save, Save as, Import, Export, Quit |
| Ctrl/Cmd+Z, Shift+Z or Y | Undo, Redo |
| Ctrl/Cmd+X, C, V, B | Cut, Copy, Paste, Split at playhead |
| Delete, Backspace | Delete |
| Space, Home, End, Left, Right | Play/Pause, go to start/end, previous/next frame |
| Ctrl/Cmd + `=` (or `+`), `-`, `0` | Zoom in, zoom out, fit project |
| Ctrl/Cmd + `,` | Settings |
| F1 | Keyboard shortcuts window |
| Esc | Close the topmost popup |

Greyed-out controls show `AppState::disabled_reason` as tooltip ("Nothing to undo", "Select a
clip first", …).

## Floating windows and markers

Open windows are application state (`AppState::popups`: open list in opening order, the selected
fade or line, the chooser list), changed by `OpenPopup`, `ClosePopup`, `OpenInstance` and
`ChooseInstance`. Each window is an `egui::Window` with a fixed id (`keymap::popup_id`), a close
button, `constrain(true)` and its own scroll area; none blocks editing or playback.

`ui::markers` is pure: from the project, the zoom and the visible range it returns one marker per
fade (diamond at the clip boundary), per subtitle line (flag at its start with a bar to its end)
and an "Add fade" handle on every boundary without a fade, each with its tooltip. A click on the
marker lane hits every marker within 4 px: one opens it (`OpenInstance`) or adds the fade
(`SetFade`), several open the chooser. Markers are recomputed each frame, so they follow ripple
edits, cuts and zoom. "Play this line" seeks, plays and stops at the line's end (`tick` checks
`stop_at`).

Right-click menus exist on video clips, music clips, markers and the empty timeline; a double
click on a clip opens Clip details.

## Undo names, recent projects, drag and drop

- `History` stores each snapshot with the name of its action (`Command::undo_name`), moved
  between the undo and redo stacks with it.
- Recent projects: `settings::push_recent` (most recent first, no duplicates, at most 10;
  only absolute UTF-8 paths up to 1024 bytes without control characters) after a successful
  open or save; `OpenRecent` of a missing file shows a message and removes it.
- Dropped files arrive as `Command::DropFiles`; `app::drop_plan` opens the first `.vlproj`
  (through the unsaved-changes question) and ignores the rest with a message, or imports
  everything. Drops are ignored while a dialog is open. Linux Wayland sessions do not report
  dropped files (winit 0.30), so there File > Import media… and Open… are the way.

## Settings file

`layout` (optional, left out when default) and `recent` are added to the settings format 2
without changing its version: files of 1.0 load silently, damaged values become the default
(each recent entry is checked alone) and the rest of the file is kept. Videolore 1.0 drops the
two fields when it saves.
