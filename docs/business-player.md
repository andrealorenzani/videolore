# Business — Video player

Back to the [index](business.md).

Added in 1.6.0: **videolore player**, a second program with its own build. It plays one video file to watch it, with the 360 view, subtitles from a file or from the video, and full screen. It does not edit, save or export; editing stays in the editor. The code for the 360 view and for moving subtitles is shared with the editor, not copied.

## Features
- **One video at a time**: File > Open… (Ctrl/Cmd+O), or drop a video onto the window where drag and drop works (see [Window, menus and effects](business-ui.md)).
- **Playback**: Play/Pause (Space), a seek bar, volume and mute; the file name is shown in the title.
- **360 video**: drag the picture to turn the camera, as in the editor (see [Boxes on the picture and 360 video](business-picture.md)).
- **Subtitles**: a Subtitles menu chooses None, a subtitle track embedded in the video, or a `.srt` file opened with Subtitles > Open .srt…; a line is dragged to another place on the picture.
- **Full screen**: only the picture and its playback controls remain; Esc leaves it.

## Requirements

### The program
- REQ-138 (implemented): videolore player is a separate program with its own executable and its own installers for Windows, macOS and Linux, built like the editor's (REQ-040 to REQ-042; assumption). It opens one video file at a time, by File > Open… or by dropping a video file where drag and drop works. It never edits, saves or exports, and it refuses a `.vlproj` project with a clear message, leaving the file untouched. It works offline, with no network access. Its window and popups follow REQ-068 at 800x500. Acceptance: an MP4 opens and plays with Play/Pause (Space), seek bar, volume and mute; a `.vlproj` is refused with a message; the player's installer contains no editor and the editor's installer contains no player.

### 360 video
- REQ-139 (implemented): In the player a 360 video (recognised from the file's own information, REQ-100) is turned by dragging the picture, with the same drag gesture, the same limits and the same code as the editor's look-around (REQ-112 view limits). Acceptance: the same drag on the same 360 file gives the same view direction in the editor and in the player; the view stops at the limits; a flat video has no drag turning.

### Subtitles
- REQ-140 (implemented): The Subtitles menu lists None, every text subtitle track embedded in the video (named by its title or language from the file), and every `.srt` file opened in this session with Subtitles > Open .srt…; the chosen one is shown on the picture at the video's time, drawn as in the editor preview. A subtitle line is dragged to another place on the picture as in REQ-089; the place is a fraction of the picture, kept while the player is open and not saved (assumption). On a 360 video the lines stay fixed on the screen (assumption). Acceptance: choosing an embedded track shows its lines at their times; Open .srt adds the file and selects it; dragging a line moves it, and reopening the file gives the default place.

### Full screen
- REQ-141 (implemented): View > Full screen, F11 or F switches full screen on and off. In full screen the menu bar, the title and the status information are hidden; the picture and its playback controls remain, and the controls hide after 2 seconds without pointer movement and show again when the pointer moves (assumption). When full screen is entered, a popup says "Press Esc to exit full screen" with an OK button; it closes with OK or after 5 seconds (assumption), and it shows again at the next entry. Esc always leaves full screen, and while a popup is open it closes the popup first. Acceptance: entering full screen hides the menu and the title; the popup appears; Esc returns to the normal window; F11 toggles both ways.

## Key decisions
- (1.6.0) The player is a separate program so that people who only watch videos get a small, simple tool, and the editor stays focused on making clips.
- (1.6.0) The player opens video files only, never projects: a project belongs to the editor and its source files (REQ-001).
- (1.6.0) Subtitle positions chosen in the player are not saved, so the player never writes files next to the video (assumption).

## Known limitations
- Embedded subtitles drawn as pictures (for example Blu-ray or DVB subtitles) are not listed; only text subtitle tracks are.
- Subtitle positions in the player are lost when the player is closed.
- The player does not open a file from the editor, and the editor does not open a file in the player.
- Video files are not associated with the player in 1.6.0 (no double-click from the file manager).
- A big video is played through a preview copy that the player makes in the background without a status message (accepted deviation, 1.6.0; the copy is the editor's preview copy rule).

## Out of scope
- Editing, cutting, effects, export, playlists, streaming, network sources, subtitle styling in the player, saving positions or playlists.
- Picture-in-picture, a second video track.
