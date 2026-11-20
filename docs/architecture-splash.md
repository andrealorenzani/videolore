# Architecture: splash screen

Part of [Architecture](architecture.md). Requirements REQ-133, REQ-134 (editor) and REQ-150 (player) in [business splash](business-splash.md) and [business player](business-player.md). Since 1.6.0 for the editor. The player's splash and the 1.6.2 rules are implemented in 1.6.2.

## Overview
At every start without a file on the command line, the editor and the player show a splash window: a picture with "Version X.Y" written over its lower rectangle, and beside it a panel with a summary of the version and a Close button. It is an in-app window (egui) above the main window, closed by Close, Esc, or the title-bar close control. It never blocks the program. Both programs use the same code: the library module `videolore::splash` (pure) and `videolore::ui::splash_window` (drawing). Each program passes its own picture and summary.

## Assets
| File | Used by | Checked at build time |
|---|---|---|
| `assets/splash/videolore.gif` | editor | Yes: `include_bytes!` in the library (`splash::EDITOR`). Fixed name since 1.6.2. |
| `assets/splash/videolore-player.gif` | player | Yes: `include_bytes!` (`splash::PLAYER`). |
| `assets/splash/<version>.txt` | editor | Yes: `include_str!` of `CARGO_PKG_VERSION`. A missing file is a build error. |
| `assets/splash/<version>-player.txt` | player | Yes, the same way. Required for every build from 1.6.2 (the editor build too, since both live in the library). |

The four rules:
- Paths are built from `CARGO_MANIFEST_DIR` of the root package, so the player gets the same files. No `build.rs`: the compiler already names the missing file.
- The picture names do not change with the version; only the two summaries are written per release.
- Each summary is UTF-8 text: a heading line, then at most five lines, one item each.
- The old picture `1.6.0.gif` is removed from `assets/splash/` (byte-identical to `videolore.gif`; the git history and the 1.6.0 release keep it). `1.6.0.txt` and `1.6.1.txt` stay as history; the build does not read them.

## Text rules
- **Version text.** "Version X.Y" (major and minor number) is written over the program's rectangle, centred, in the largest size that fits inside the rectangle minus a 6 px margin on each side. Editor rectangle: x 74 to 408, y 381 to 465 (334 x 84 px). Player rectangle: x 78 to 386, y 388 to 462 (308 x 74 px). Origin top left. Its colour is dark or light, chosen from the mean brightness of the rectangle. The rectangles are checked to lie inside their pictures by a test.
- **Summary (no scroll, no silent clip).** The heading and the items are laid out at the panel's inner width (400 px minus 14 px on each side). The body starts at egui's body size (14 px, scaled with the picture) and goes down in steps of 0.5 px, to a floor of 10 px, until the whole summary fits above the Close button. The heading is the body size plus 4 px. A summary that does not fit even at the floor fails a test; it is not clipped in the shipped program. At run time a text that still does not fit is clipped, as before (never the case for the shipped files).

## Layout
- Image at its own size, multiplied by one scale factor.
- Panel: 400 px wide and as high as the image, beside it, with the summary and the Close button.
- If the screen is too small for both, both are scaled by the same factor: `min(1, available width / total width, available height / image height)`.
- Pure functions (`splash::layout`, `splash::fit_text`, `splash::fit_summary`), tested with numbers.

## Behaviour
- **Editor.** Shown at a start without a project or video on the command line. A start with a path (double-click, second window) shows none. No single-instance detection.
- **Player.** Shown at a start without a video on the command line. A video on the command line opens at once, with no splash. The player never opens projects, so a `.vlproj` on the command line also means no splash.
- Closing: Close, Esc, or the title-bar control. Esc closes the splash first, when it is the topmost popup. In the player the order is: splash, other popup, full screen.
- Non-blocking: the main window (editor or player) stays usable, including Space and F11 in the player. The splash does not ask about unsaved changes.
- The editor's first-start notice (REQ-045), if due, appears after the splash closes.
- Decoding: once per program on first display, with the `image` crate's GIF decoder (about 100 ms budget). Every frame (up to 64) is kept as a texture and loops; repaint is requested only while the splash is open. Caps: side 4096 px, and a total of 64 MiB of RGBA per picture (1.6.2).
- The summary's fitted size is computed once when the splash opens and again when the scale changes, then cached.

## Tests
- Assets: each picture decodes at its own size (editor 486 x 506, player 487 x 505); each summary exists and has a heading and 1 to 5 items; each text rectangle lies inside its picture.
- Layout numbers: panel 400 px (plus or minus 1), panel height equal to the picture height, scaling when the screen is small.
- Text: "Version 1.6" fits its rectangle for the editor and for the player; the summary of each program fits at a size of at least 10 px, measured with egui's wrapping.
- State (editor, unchanged): open at start, closed by each of the three methods, not shown with a path, the notice waits for the close, opened again at the next start.
- State (player, REQ-150): the same rules; a video on the command line opens none; Esc closes the splash before full screen.

## Risks
- A stale summary: each release needs its own `.txt` and `-player.txt` before it builds (intended; a process step).
- The pictures' origin and licence must be recorded in the notices before release (open item).
- The splash is an in-app window, not a separate OS window (multi-window behaviour on Wayland and macOS is not tested).
- Both pictures are compiled into both programs' library; the binary may grow by up to about 175 KB (an estimate, not yet measured; fallback: move the player picture into the player crate).
