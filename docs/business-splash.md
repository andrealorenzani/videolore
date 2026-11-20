# Business — Splash screen

Back to the [index](business.md).

Added in 1.6.0 for the editor, and extended to the video player in 1.6.2 (REQ-150, see [Video player](business-player.md)). A splash screen opens at every start without a project or video path on the command line, and stays until it is closed. It shows the version (major and minor number) on the picture and a short summary of what changed in this version.

## Features
- **Picture**: one fixed picture per program (1.6.2; before, one picture per version): `assets/splash/videolore.gif` for the editor and `assets/splash/videolore-player.gif` for the player. The text "Version X.Y" is written over the lower rectangle of the picture.
- **Summary popup**: a 400 pixel wide panel as high as the picture, beside it, with a short summary of what changed in this version and a **Close** button. The summary is written for each release: `assets/splash/<version>.txt` for the editor, `assets/splash/<version>-player.txt` for the player.
- **Closing**: the Close button, Esc, or the window's own close control.

## Requirements

### Splash screen
- REQ-133 (implemented): At every start of the editor without a project or video path on the command line, a splash screen opens on top of the main window and stays until it is closed with its Close button, Esc or the window's close control. A start with a project or video path on the command line (for example a double-click on a project, first start or second window, REQ-041) shows no splash. There is no single-instance detection: a second window is a start like any other. Closing the splash never asks about unsaved changes. The main window stays usable while the splash is open (assumption). If the first-start notice of REQ-045 is due, it appears after the splash is closed, never on top of it. Acceptance: at a start without a path the splash is visible; each closing method removes it; the next such start shows it again; a start with a project path or a video path shows no splash; with the first-start notice both appear one after the other.
- REQ-134 (implemented): The splash has two parts side by side. Left: the picture `assets/splash/videolore.gif` at its own size, the same picture in every version; the GIF plays as it is. Inside the picture, in the rectangle from x 74 to x 408 and from y 381 to y 465 (pixels, origin at the top left corner, so the rectangle is 334 by 84 pixels), the text "Version X.Y" is written in the largest size that fits, centred, touching neither edge. X and Y are the major and minor numbers of the version (1.6.2 gives "Version 1.6"). Right: a popup 400 pixels wide and exactly as high as the picture, with the summary of this version from `assets/splash/<version>.txt` (written for each release) and a **Close** button. If the two parts do not fit the screen, both are scaled together, keeping proportions (assumption). Acceptance: the picture `videolore.gif` shows "Version 1.6" entirely inside the rectangle; the popup is 400 pixels wide (±1 pixel) and as high as the picture (±1 pixel); Close closes it; the summary is the one of the running version.

## Key decisions
- (1.6.0) The splash shows at every start without a project or video path on the command line, not once per version, so the summary is seen by everyone who starts the program by hand. A start that opens a project or a video (first start or second window) shows no splash, so opening work is not delayed. No single-instance detection.
- (1.6.0) The splash does not block the editor, so a user who opens a project at once is not held up; it stays on top until closed.
- (1.6.0) The version text is written by the program from the version number, not drawn in the picture, so each picture needs no text of its own.
- (1.6.2) The picture files have fixed names (`videolore.gif`, `videolore-player.gif`) and do not change with the version; only the summary text changes per release, so a release needs no new picture.
- (1.6.2) Each program has its own summary, because the editor and the player change in different ways. The 1.6.0 rule of no splash in the player is reversed (REQ-150).

## Known limitations
- The summary text is written by hand for each release; a release without an updated summary shows the previous one.
- The build needs `assets/splash/videolore.gif` and `assets/splash/videolore-player.gif`, and for the version being built `assets/splash/<version>.txt` and `assets/splash/<version>-player.txt` (each a heading and at most five lines; the player file from 1.6.2 on); it fails without them.
- The origin and licence of the splash pictures are not yet confirmed; they must be checked before the release.
- (1.6.1) Version 1.6.1 used the 1.6.0 picture, copied as `assets/splash/1.6.1.gif` (Andrea confirmed this; no new picture was supplied), with its summary `assets/splash/1.6.1.txt`. Since 1.6.2 the editor uses `videolore.gif`, which is that same picture.
- (1.6.2) A summary must fit the popup (400 pixels wide, as high as the picture); a longer summary uses a smaller text, it does not scroll (assumption, to be checked on the build).

## Out of scope
- A "do not show again" option, animated text, sound.
