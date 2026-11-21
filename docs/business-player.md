# Business — Video player

Back to the [index](business.md).

Added in 1.6.0: **videolore player**, a second program with its own build. It plays one video file to watch it, with the 360 view, subtitles from a file or from the video, and full screen. It does not edit, save or export; editing stays in the editor. The code for the 360 view and for moving subtitles is shared with the editor, not copied.

## Features
- **One video at a time**: File > Open… (Ctrl/Cmd+O), or drop a video onto the window where drag and drop works (see [Window, menus and effects](business-ui.md)).
- **Playback**: Play/Pause (Space), a seek bar, volume and mute; the file name is shown in the title.
- **360 video**: drag the picture to turn the camera, as in the editor (see [Boxes on the picture and 360 video](business-picture.md)).
- **Subtitles**: a Subtitles menu chooses None, a subtitle track embedded in the video, or a `.srt` file opened with Subtitles > Open .srt…; a line is dragged to another place on the picture.
- **Full screen**: only the picture and its playback controls remain; Esc leaves it.
- **Start screen** (1.6.2): a splash screen like the editor's, shown at every start without a video path on the command line (REQ-150).

## Requirements

### The program
- REQ-138 (implemented): videolore player is a separate program with its own executable and its own installers for Windows, macOS and Linux, built like the editor's (REQ-040 to REQ-042; assumption). It opens one video file at a time, by File > Open… or by dropping a video file where drag and drop works. It never edits, saves or exports, and it refuses a `.vlproj` project with a clear message, leaving the file untouched. It works offline, with no network access. Its window and popups follow REQ-068 at 800x500. Acceptance: an MP4 opens and plays with Play/Pause (Space), seek bar, volume and mute; a `.vlproj` is refused with a message; the player's installer contains no editor and the editor's installer contains no player.

### 360 video
- REQ-139 (implemented): In the player a 360 video (recognised from the file's own information, REQ-100) is turned by dragging the picture, with the same drag gesture, the same limits and the same code as the editor's look-around (REQ-112 view limits). Acceptance: the same drag on the same 360 file gives the same view direction in the editor and in the player; the view stops at the limits; a flat video has no drag turning.

### Subtitles
- REQ-140 (implemented): The Subtitles menu lists None, every text subtitle track embedded in the video (named by its title or language from the file), and every `.srt` file opened in this session with Subtitles > Open .srt…; the chosen one is shown on the picture at the video's time, drawn as in the editor preview. A subtitle line is dragged to another place on the picture as in REQ-089; the place is a fraction of the picture, kept while the player is open and not saved (assumption). On a 360 video the lines stay fixed on the screen (assumption). Acceptance: choosing an embedded track shows its lines at their times; Open .srt adds the file and selects it; dragging a line moves it, and reopening the file gives the default place.

### Full screen
- REQ-141 (implemented): View > Full screen, F11 or F switches full screen on and off. In full screen the menu bar, the title and the status information are hidden; the picture and its playback controls remain, and the controls hide after 2 seconds without pointer movement and show again when the pointer moves (assumption). When full screen is entered, a popup says "Press Esc to exit full screen" with an OK button; it closes with OK or after 5 seconds (assumption), and it shows again at the next entry. Esc always leaves full screen, and while a popup is open it closes the popup first. Acceptance: entering full screen hides the menu and the title; the popup appears; Esc returns to the normal window; F11 toggles both ways.

### Start screen
- REQ-150 (implemented): The player has a start screen like the editor's (REQ-133, REQ-134), with these differences. It opens at every start of the player without a path on the command line, on top of the player window, and closes in the same three ways (Close, Esc, the window's close control); a start with a project or video path on the command line shows none. Its picture is `assets/splash/splash-player.gif`, and "Version X.Y" is written in the rectangle from x 78 to x 386 and from y 388 to y 462 (pixels, origin at the top left corner, so the rectangle is 308 by 74 pixels), in the largest size that fits, centred, touching neither edge. Its popup is 400 pixels wide, as high as the picture, with the summary of this version from `assets/splash/summary-player.txt` and a **Close** button. Acceptance: at a start without a path the splash is visible and each closing method removes it; the next such start shows it again; a start with a video path shows none; "Version 1.6" lies entirely inside the rectangle on the player's picture; the popup is 400 pixels wide (±1 pixel) and as high as the picture (±1 pixel); the summary is the player's text for the running version.

### Opening, preferences and playlist (2.0.4)
Batch 13 of the [roadmap tasks](roadmap_tasks.md). REQ-253 is not built yet and stays `(planned)`, on the roadmap.
- REQ-248 (implemented): The player's installers add the player to "Open with" for MP4, MOV, MKV, WebM, M4V and AVI files. The player becomes the default for these types only when the user chooses it in the operating system; no installer sets it silently. The editor's `.vlproj` association stays. Built: the Windows installer uses its own template `packaging/windows-player.nsi`, with the "Open with" entries only (`OpenWithProgids`); its run on a real Windows system is a manual check (verification item 29). Acceptance: the installer configuration check lists the "Open with" entries for each system; after the user's choice, a double-click opens the file in the player (checked by hand, see [verification](business-verification.md)).
- REQ-249 (implemented): **Open in player** (File menu of the editor, for the selected media item) starts the player with that file; the editor stays open. If the player is not installed, the editor says so. Acceptance: with a media item selected the player starts with its path; without the player a message names it; the editor's project is unchanged.
- REQ-250 (implemented): **Open in editor** (File menu of the player) opens a new, unsaved editor project with the playing file as its media item. Nothing is written next to the video. Acceptance: the editor opens with the file in its media list; the folder of the video has no new file.
- REQ-251 (implemented): Subtitle positions chosen in the player are kept while the same file stays open in the session, including after its subtitle list is changed; they are lost when the player closes (MINOR default, see decisions). Acceptance: move a line, open another file and open the first again in the same session: the line keeps its place; after a restart the default place is used.
- REQ-252 (implemented): The player remembers the volume, mute, the last folder used for File > Open and the start in full screen, in a settings file in the user's settings folder, never next to the video. Acceptance: change the volume and restart: the same volume; the folder of the video has no new file.
- REQ-253 (planned): The Subtitles menu also lists picture-based tracks (DVB, DVD and Blu-ray PGS), marked "picture". Their lines are drawn on the picture at their times, scaled to the video, and are not moved by dragging. Amends REQ-140 and the limitation of 1.6.0. Acceptance: a fixture with a DVB subtitle track is listed and drawn at its time; a text track is shown as before.
- REQ-254 (implemented): **Playlist**: File > Add to playlist adds a video, and File > Open folder… adds every video of a folder in name order. The player plays the list in order, with Next and Previous (Ctrl/Cmd+Right and Ctrl/Cmd+Left) and stops after the last video. The list is kept for the session only. Acceptance: three files play in order; Next and Previous move between them; after the last one playback stops; a restart has an empty list.
- REQ-255 (implemented): The player's About screen lists the player's own bundled components (its ffmpeg and ffprobe, the icon set, the fonts) with their licences, from the player's own notice file. The editor's notices are not shown there. Acceptance: the About screen of the player names its own components; the editor's list is not in it (test of the notice file).
- REQ-256 (implemented): A very large video plays with its sound: the preview copy the player makes keeps the audio track, and the sound stays in step with the picture (REQ-035). Amends the accepted deviation of 1.6.0. Acceptance: a large fixture gives a preview copy with an audio stream; playback keeps the sound in step within 100 ms.

## Key decisions
- (1.6.0) The player is a separate program so that people who only watch videos get a small, simple tool, and the editor stays focused on making clips.
- (1.6.2) The player has a start screen like the editor's (REQ-150); this reverses the 1.6.0 rule that the player has no splash screen.
- (1.6.0) The player opens video files only, never projects: a project belongs to the editor and its source files (REQ-001).
- (1.6.0) Subtitle positions chosen in the player are not saved, so the player never writes files next to the video (assumption).

## Known limitations
- Embedded subtitles drawn as pictures (for example Blu-ray or DVB subtitles) are not listed; only text subtitle tracks are.
- Subtitle positions in the player are lost when the player is closed.
- Video files are offered in "Open with" (REQ-248), but the player becomes the default only when the user chooses it in the operating system.
- Picture-based subtitle tracks (DVB, DVD, Blu-ray) are not listed or drawn yet (REQ-253, on the roadmap).
- The player's Windows, macOS and Linux builds and installers are not yet run on real systems; the checklist is in [Release verification](business-verification.md).
- A big video is played through a preview copy that the player makes in the background without a status message (accepted deviation, 1.6.0; the copy is the editor's preview copy rule).

## Out of scope
- Editing, cutting, effects, export, streaming, network sources, subtitle styling in the player.
- Saving subtitle positions or the playlist beyond the session (REQ-251, REQ-254).
- A lighter ffmpeg for the player: to be measured first (quality goals), not a requirement.
- Picture-in-picture, a second video track.
