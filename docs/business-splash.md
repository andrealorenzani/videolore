# Business — Splash screen

Back to the [index](business.md).

Added in 1.6.0 for the editor, and extended to the video player in 1.6.2 (REQ-150, see [Video player](business-player.md)). A splash screen opens at every start without a project or video path on the command line, and stays until it is closed. It shows the version (major and minor number) on the picture and a short summary of what changed in this version.

## Features
- **Picture**: one fixed picture per program, never copied per version (1.6.2, names fixed in 1.6.3): `assets/splash/splash.gif` for the editor and `assets/splash/splash-player.gif` for the player. The text "Version X.Y" is written over the lower rectangle of the picture.
- **Summary popup**: a 400 pixel wide panel as high as the picture, beside it, with a short summary of what changed in this version and a **Close** button. The summary is rewritten for each release in a fixed file: `assets/splash/summary.txt` for the editor, `assets/splash/summary-player.txt` for the player.
- **Closing**: the Close button, Esc, or the window's own close control.

## Requirements

### Splash screen
- REQ-133 (implemented): At every start of the editor without a project or video path on the command line, a splash screen opens on top of the main window and stays until it is closed with its Close button, Esc or the window's close control. A start with a project or video path on the command line (for example a double-click on a project, first start or second window, REQ-041) shows no splash. There is no single-instance detection: a second window is a start like any other. Closing the splash never asks about unsaved changes. The main window stays usable while the splash is open (assumption). If the first-start notice of REQ-045 is due, it appears after the splash is closed, never on top of it. Acceptance: at a start without a path the splash is visible; each closing method removes it; the next such start shows it again; a start with a project path or a video path shows no splash; with the first-start notice both appear one after the other.
- REQ-134 (implemented): The splash has two parts side by side. Left: the picture `assets/splash/splash.gif` at its own size, the same picture in every version; the GIF plays as it is. Inside the picture, in the rectangle from x 74 to x 408 and from y 381 to y 465 (pixels, origin at the top left corner, so the rectangle is 334 by 84 pixels), the text "Version X.Y" is written in the largest size that fits, centred, touching neither edge. X and Y are the major and minor numbers of the version (1.6.2 gives "Version 1.6"). Right: a popup 400 pixels wide and exactly as high as the picture, with the summary of this version from `assets/splash/summary.txt` (written for each release) and a **Close** button. If the two parts do not fit the screen, both are scaled together, keeping proportions (assumption). Acceptance: the picture `splash.gif` shows "Version 1.6" entirely inside the rectangle; the popup is 400 pixels wide (±1 pixel) and as high as the picture (±1 pixel); Close closes it; the summary is the one of the running version.
- REQ-154 (implemented): The summary is shown in the largest text, from 15 down to 7 image pixels, at which it fits the popup; a summary that is longer than the popup at 15 pixels uses a smaller text and is never scrolled. The summaries shipped with a version must fit at 7 pixels or more; a summary that does not fit fails the test suite, so it is not released. Acceptance: a summary with more lines than fit at 15 pixels is shown smaller, and every line is inside the popup; no scroll bar appears; the shipped editor and player summaries fit their popups; a summary that cannot fit even at 7 pixels fails the tests.

### Show option (2.0.4)
- REQ-295 (implemented): Settings has **Show the start screen**: "At every start" (the default, as now), "Once for each version" or "Never". The choice is a program setting; the player's start screen follows the same choice for the player. A start with a path on the command line never shows it. Acceptance: "Once for each version" shows the splash at the first start of a version and not at the next starts of that version; "Never" shows none; the default shows it at every start.

## Key decisions
- (1.6.0) The splash shows at every start without a project or video path on the command line, not once per version, so the summary is seen by everyone who starts the program by hand. A start that opens a project or a video (first start or second window) shows no splash, so opening work is not delayed. No single-instance detection.
- (1.6.0) The splash does not block the editor, so a user who opens a project at once is not held up; it stays on top until closed.
- (1.6.0) The version text is written by the program from the version number, not drawn in the picture, so each picture needs no text of its own.
- (1.6.2, names in 1.6.3) The picture files (`splash.gif`, `splash-player.gif`) and the summary files (`summary.txt`, `summary-player.txt`) have fixed names; nothing is copied per version. Only the text of the summaries is rewritten for each release.
- (1.6.2) Each program has its own summary, because the editor and the player change in different ways. The 1.6.0 rule of no splash in the player is reversed (REQ-150).

## Known limitations
- The summary text is written by hand for each release; a release without an updated summary shows the previous one.
- The build needs `assets/splash/splash.gif` and `assets/splash/splash-player.gif`, and the two summaries `assets/splash/summary.txt` and `assets/splash/summary-player.txt` (each a heading and at most five lines, rewritten for each version); it fails without them.
- The splash pictures are the author's own work, licensed MIT; their origin and licence are recorded in the third-party notices (since 1.6.3).
- A summary longer than the popup uses a smaller text, down to 7 pixels, and does not scroll (REQ-154). The rule is checked by the test suite for the shipped summaries.

## Out of scope
- Animated text, sound. The "do not show again" choice is the Show option of REQ-295 (2.0.4).
- An animated splash image: the picture is kept as supplied until the requirement wording (REQ-134) is decided (open, MINOR default: static).
