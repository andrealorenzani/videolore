# Architecture: splash screen

Part of [Architecture](architecture.md). Requirements REQ-133 and REQ-134 ([business](business-splash.md)). Planned for 1.6.0.

## Overview
At every start of the editor without a file on the command line a splash window shows the release image, the text "Version X.Y" and a summary of the version's change. It is an in-app window on top of the main window, closed by its Close button, Esc or its title-bar close control. It never blocks the editor. The video player has no splash.

## Assets
| File | What | Checked at build time |
|---|---|---|
| `assets/splash/<version>.gif` | Release image, 486 x 506 px for 1.6.0 | Yes: `include_bytes!` of `concat!(env!("CARGO_MANIFEST_DIR"), "/assets/splash/", env!("CARGO_PKG_VERSION"), ".gif")`. A missing file for the release is a build error. |
| `assets/splash/<version>.txt` | Summary of the version's change, plain text, at most 6 lines that fit the 400 px popup | Yes, the same way (`include_str!`). |

The text "Version X.Y" is drawn by the program over the rectangle x 74 to 408, y 381 to 465 of the image (334 x 84 px, origin at the top left). The largest size that fits is used, centred. Its colour is dark or light, chosen from the mean brightness of the rectangle in the image.

## Layout
- Image at its own size, multiplied by one scale factor.
- Popup: 400 px wide and as high as the image, beside it, with the summary and a Close button.
- If the screen or window is too small for both, both are scaled by the same factor (`min(1, available width / total width, available height / image height)`).
- The layout is a pure function (`splash::layout`), tested with numbers.

## Behaviour
- Shown when the editor starts without a path on the command line. A project or video given on the command line (double-click, or a second window) shows no splash. Decided 2026-11-19: this includes a double-click first start, so the summary is not shown once in that case; no single-instance detection is used.
- Closing by Close, Esc, or the title-bar control; Esc closes the splash first when it is the topmost popup (the same rule as the other popups).
- Closing never asks about unsaved changes.
- The first-start notice (REQ-045), if due, appears after the splash is closed.
- The GIF is decoded once, on first display, with the `image` crate's GIF decoder (about 100 ms budget). All frames are kept as textures; a repaint is requested only while the splash is open. The 1.6.0 image has one frame, so it is still. Decided 2026-11-19: the image is kept as supplied; a multi-frame file would animate with no code change.

## Tests (planned)
- Layout numbers: popup 400 px (plus or minus 1), popup height equal to the image height, scaling when the screen is small.
- Text: "Version 1.6" for 1.6.0, fits inside the rectangle at its chosen size.
- Assets: the summary file of the version exists; the GIF decodes to 486 x 506.
- State: open at start, closed by each of the three methods, not shown with a path argument, the notice waits for the close, opened again at the next start.

## Risks
- A stale summary: each release needs its own `.txt` before it builds (intended, but a process step).
- The image's origin and licence must be recorded in the notices before release (open item).
- The splash is an in-app window, not a separate OS window (multi-window behaviour on Wayland and macOS is not tested).
