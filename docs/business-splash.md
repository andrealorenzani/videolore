# Business — Splash screen

Back to the [index](business.md).

Added in 1.6.0. A splash screen opens at every start of the editor without a project or video path on the command line, and stays until it is closed. It shows the version (major and minor number) on the release image and a short summary of what changed in this version. The video player (see [Video player](business-player.md)) has no splash screen.

## Features
- **Image**: the release image `assets/splash/<version>.gif` (for 1.6.0, `1.6.0.gif`), with the text "Version X.Y" written over its lower rectangle.
- **Summary popup**: a 400 pixel wide panel as high as the image, beside it, with a short summary of the latest change of the version and a **Close** button.
- **Closing**: the Close button, Esc, or the window's own close control.

## Requirements

### Splash screen
- REQ-133 (implemented): At every start of the editor without a project or video path on the command line, a splash screen opens on top of the main window and stays until it is closed with its Close button, Esc or the window's close control. A start with a project or video path on the command line (for example a double-click on a project, first start or second window, REQ-041) shows no splash. There is no single-instance detection: a second window is a start like any other. Closing the splash never asks about unsaved changes. The main window stays usable while the splash is open (assumption). If the first-start notice of REQ-045 is due, it appears after the splash is closed, never on top of it. Acceptance: at a start without a path the splash is visible; each closing method removes it; the next such start shows it again; a start with a project path or a video path shows no splash; with the first-start notice both appear one after the other.
- REQ-134 (implemented): The splash has two parts side by side. Left: the image `assets/splash/<version>.gif` at its own size; the GIF plays as it is. Inside the image, in the rectangle from x 74 to x 408 and from y 381 to y 465 (pixels, origin at the top left corner, so the rectangle is 334 by 84 pixels), the text "Version X.Y" is written in the largest size that fits, centred, touching neither edge. X and Y are the major and minor numbers of the version (1.6.0 gives "Version 1.6"). Right: a popup 400 pixels wide and exactly as high as the image, with the summary of the latest change of this version (text written for each release) and a **Close** button. If the two parts do not fit the screen, both are scaled together, keeping proportions (assumption). Acceptance: the 1.6.0 image shows "Version 1.6" entirely inside the rectangle; the popup is 400 pixels wide (±1 pixel) and as high as the image (±1 pixel); Close closes it; the summary is the one written for 1.6.0.

## Key decisions
- (1.6.0) The splash shows at every start without a project or video path on the command line, not once per version, so the summary is seen by everyone who starts the program by hand. A start that opens a project or a video (first start or second window) shows no splash, so opening work is not delayed. No single-instance detection.
- (1.6.0) The splash does not block the editor, so a user who opens a project at once is not held up; it stays on top until closed.
- (1.6.0) The version text is written by the program from the version number, not drawn in the image, so each image needs no text of its own.

## Known limitations
- The summary text is written by hand for each release; a release without an updated summary shows the previous one.
- The splash image `assets/splash/<version>.gif` and its summary `assets/splash/<version>.txt` (a heading and at most five lines) must be in the repository before that version is built; the build fails without them.
- The origin and licence of the splash image are not yet confirmed; they must be checked before the release.

## Out of scope
- A "do not show again" option, a splash in the video player, animated text, sound.
