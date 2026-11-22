# Business — Splash screen

Back to the [index](business.md).

The splash screen opens at every start of the editor or the player without a project or video path on the command line (REQ-150, see [Video player](business-player.md)), and stays until it is closed. It shows the version (major and minor number) on the picture and a short summary of what changed.

## Features
- **Picture**: one fixed picture per program, never copied per version: `assets/splash/splash.gif` for the editor and `assets/splash/splash-player.gif` for the player. The text "Version X.Y" is written over the lower rectangle of the picture.
- **Summary popup**: a 400 pixel wide panel as high as the picture, beside it, with the summary and a **Close** button. The summary is rewritten for each release in a fixed file: `assets/splash/summary.txt` for the editor, `assets/splash/summary-player.txt` for the player.
- **Closing**: the Close button, Esc, or the window's own close control.

## Requirements

### Splash screen
- REQ-133 (implemented): At every start of the editor without a project or video path on the command line, a splash screen opens on top of the main window and stays until it is closed with its Close button, Esc or the window's close control. A start with a project or video path on the command line (for example a double-click on a project, first start or second window, REQ-041) shows no splash. There is no single-instance detection: a second window is a start like any other. Closing the splash never asks about unsaved changes. The main window stays usable while the splash is open (assumption). If the first-start notice of REQ-045 is due, it appears after the splash is closed, never on top of it. Acceptance: at a start without a path the splash is visible; each closing method removes it; the next such start shows it again; a start with a project path or a video path shows no splash; with the first-start notice both appear one after the other.
- REQ-134 (implemented): The splash has two parts side by side. Left: the picture `assets/splash/splash.gif` at its own size, the same picture in every version; the GIF plays as it is. Inside the picture, in the rectangle from x 74 to x 408 and from y 381 to y 465 (pixels, origin at the top left corner, so the rectangle is 334 by 84 pixels), the text "Version X.Y" is written in the largest size that fits, centred, touching neither edge. X and Y are the major and minor numbers of the version (1.6.2 gives "Version 1.6"). Right: a popup 400 pixels wide and exactly as high as the picture, with the summary of this version from `assets/splash/summary.txt` and a **Close** button. If the two parts do not fit the screen, both are scaled together, keeping proportions (assumption). Acceptance: the picture `splash.gif` shows "Version 1.6" entirely inside the rectangle; the popup is 400 pixels wide (±1 pixel) and as high as the picture (±1 pixel); Close closes it; the summary is the one of the running version.
- REQ-154 (implemented): The summary is shown in the largest text, from 15 down to 7 image pixels, at which it fits the popup; a summary that is longer than the popup at 15 pixels uses a smaller text and is never scrolled. The summaries shipped with a version must fit at 7 pixels or more; a summary that does not fit fails the test suite, so it is not released. Acceptance: a summary with more lines than fit at 15 pixels is shown smaller, and every line is inside the popup; no scroll bar appears; the shipped summaries fit their popups; a summary that cannot fit even at 7 pixels fails the tests.

### Show option (2.0.4)
- REQ-295 (implemented; amended 2.3.0: the "Never" choice is removed): Settings has **Show the start screen** with two choices: "At every start" (the default) and "Once for each version". The choice is a program setting; the player's start screen follows the same choice. A start with a path on the command line never shows it. A "Never" stored in an older Settings file is read as "Once for each version" (Andrea, 2026-11-22). Acceptance: "Once for each version" shows the splash at the first start of a version and not at the next starts of that version; the default shows it at every start; the Settings window has no "Never" choice.

## Key decisions
- The splash shows at every start without a path on the command line, not once per version, so everyone who starts the program by hand sees the summary. A start that opens a project or a video shows no splash, so opening work is not delayed. No single-instance detection.
- The splash does not block the editor; it stays on top until closed.
- The version text is written by the program from the version number, so each picture needs no text of its own.
- The picture files and the summary files have fixed names; nothing is copied per version. Only the text of the summaries is rewritten for each release.
- Each program has its own summary, because the editor and the player change in different ways.

## Known limitations
- The summary text is written by hand for each release; a release without an updated summary shows the previous one.
- The build needs both pictures and both summaries (each summary a heading and at most five lines); it fails without them.
- The splash pictures are the author's own work, licensed MIT; their origin and licence are in the third-party notices.
- A summary longer than the popup uses a smaller text, down to 7 pixels, and does not scroll (REQ-154).

## Out of scope
- Animated text, sound, and an animated splash picture (the picture is kept as supplied; the wording of REQ-134 is open in the [roadmap](roadmap.md)).
