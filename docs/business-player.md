# Business — Video player

Back to the [index](business.md).

**videolore player** is a second program with its own build. It plays one video file to watch it, with the 360 view, subtitles from a file or from the video, and full screen. It does not edit, save or export. The 360 view and the moving of subtitles share code with the editor.

## Features
- **One video at a time**: File > Open… (Ctrl/Cmd+O), or drop a video onto the window where drag and drop works (see [Window, menus and effects](business-ui.md)). Since 3.1.2: Open… takes several videos at once, which play one after the other, and more videos can be added to the playlist at any time (REQ-355, REQ-356).
- **Playback**: Play/Pause (Space), a seek bar, volume and mute; the file name in the title.
- **360 video**: drag the picture to turn the camera, as in the editor (see [Boxes on the picture and 360 video](business-picture.md)).
- **Subtitles**: a Subtitles menu chooses None, a subtitle track of the video, or a `.srt` file opened with Subtitles > Open .srt…; a line can be dragged on the picture.
- **Audio track** (3.1.0): an Audio menu chooses which audio track of the video plays; the first one by default (REQ-338).
- **Full screen**: only the picture and its controls remain; Esc leaves it.
- **Start screen**: a splash screen like the editor's (REQ-150).

## Requirements

### The program
- REQ-138 (implemented): The player is a separate program with its own executable and its own installers for Windows, macOS and Linux, built like the editor's (REQ-040 to REQ-042). It opens one video file at a time, by File > Open… or by dropping a video file where drag and drop works (several files at once: REQ-355). It never edits, saves or exports, and it refuses a `.vlproj` project with a clear message, leaving the file untouched. It works offline. Its window and popups follow REQ-068 at 800x500. Acceptance: an MP4 opens and plays with Play/Pause (Space), seek bar, volume and mute; a `.vlproj` is refused with a message; the player's installer contains no editor and the editor's installer contains no player.

### 360 video
- REQ-139 (implemented): In the player a 360 video (recognised from the file's own information, REQ-100) is turned by dragging the picture, with the same gesture, limits and code as the editor's look-around (REQ-101). Acceptance: the same drag on the same 360 file gives the same view direction in the editor and in the player; the view stops at the limits; a flat video has no drag turning.

### Subtitles
- REQ-140 (implemented): The Subtitles menu lists None, every text subtitle track embedded in the video (named by its title or language), and every `.srt` file opened in this session with Subtitles > Open .srt…; the chosen one is shown on the picture at the video's time, drawn as in the editor preview. A subtitle line is dragged to another place as in REQ-089; the place is a fraction of the picture, kept while the player is open and not saved. On a 360 video the lines stay fixed on the screen. Acceptance: choosing an embedded track shows its lines at their times; Open .srt adds the file and selects it; dragging a line moves it, and reopening the file gives the default place.

### Audio track
- REQ-338 (implemented; Andrea, 2026-11-24): An **Audio** menu lists every audio track of the video, each named by its language or title and its channel count (MINOR). The first track is chosen by default; the chosen one is the sound that plays, and it stays chosen while the same file is open in the session (as REQ-251); it is not saved (REQ-252 keeps only the volume, mute and folder). The subtitle choice of REQ-140 and REQ-253 is unchanged and is the "enable or select a subtitle" menu. Acceptance: a video with two audio tracks lists both, track 1 plays first; choosing track 2 plays its sound in step with the picture (within 100 ms, REQ-256); a one-track video lists one entry; reopening the file in the session keeps the choice; after a restart the first track is used.

### Full screen
- REQ-141 (implemented): View > Full screen, F11 or F switches full screen on and off. In full screen the menu bar, the title and the status information are hidden; the picture and its controls remain, and the controls hide after 2 seconds without pointer movement and show again when the pointer moves. When full screen is entered, a popup says "Press Esc to exit full screen" with an OK button; it closes with OK or after 5 seconds, and shows again at the next entry. Esc always leaves full screen, and while a popup is open it closes the popup first. Acceptance: entering full screen hides the menu and the title; the popup appears; Esc returns to the normal window; F11 toggles both ways.

### Start screen
- REQ-150 (implemented): The player has a start screen like the editor's (REQ-133, REQ-134, REQ-154, REQ-295), with these differences. It opens at every start of the player without a path on the command line, on top of the player window, and closes in the same three ways (Close, Esc, the window's close control); a start with a project or video path shows none. Its picture is `assets/splash/splash-player.gif`, and "Version X.Y" is written in the rectangle from x 78 to x 386 and from y 388 to y 462 (pixels, origin at the top left corner), in the largest size that fits, centred, touching neither edge. Its popup is 400 pixels wide, as high as the picture, with the summary of this version from `assets/splash/summary-player.txt` and a **Close** button. Acceptance: at a start without a path the splash is visible and each closing method removes it; the next such start shows it again; a start with a video path shows none; "Version 1.6" lies entirely inside the rectangle on the player's picture; the popup is 400 pixels wide (±1 pixel) and as high as the picture (±1 pixel).

### Opening, preferences and playlist (2.0.4)
- REQ-248 (implemented): The player's installers add the player to "Open with" for MP4, MOV, MKV, WebM, M4V and AVI files. The player becomes the default for these types only when the user chooses it in the operating system; no installer sets it silently. The editor's `.vlproj` association stays. The Windows installer uses its own template `packaging/windows-player.nsi`, with the "Open with" entries only; its run on a real Windows system is a manual check (verification item 29). Acceptance: the installer configuration check lists the "Open with" entries for each system; after the user's choice, a double-click opens the file in the player (checked by hand).
- REQ-249 (implemented): **Open in player** (File menu of the editor, for the selected media item) starts the player with that file; the editor stays open. If the player is not installed, the editor says so. Acceptance: with a media item selected the player starts with its path; without the player a message names it; the editor's project is unchanged.
- REQ-250 (implemented): **Open in editor** (File menu of the player) opens a new, unsaved editor project with the playing file as its media item. Nothing is written next to the video. Acceptance: the editor opens with the file in its media list; the folder of the video has no new file.
- REQ-251 (implemented): Subtitle positions chosen in the player are kept while the same file stays open in the session, including after its subtitle list is changed; they are lost when the player closes (MINOR). Acceptance: move a line, open another file and open the first again in the same session: the line keeps its place; after a restart the default place is used.
- REQ-252 (implemented): The player remembers the volume, mute, the last folder used for File > Open and the start in full screen, in a settings file in the user's settings folder, never next to the video. Acceptance: change the volume and restart: the same volume; the folder of the video has no new file.
- REQ-253 (implemented): The Subtitles menu also lists picture-based tracks (DVB, DVD and Blu-ray PGS), marked "picture". Their lines are drawn on the picture at their times, scaled to the video, and are not moved by dragging. Limits: a picture subtitle already on screen when a run starts (also on a paused still) appears at its next change; picture subtitles are not drawn on 360 clips. Amends REQ-140. Acceptance: a fixture with a picture subtitle track is listed and drawn at its time; a text track is shown as before. Tested with a Blu-ray PGS fixture; DVB and DVD tracks are a manual check (see [verification](business-verification.md)).
- REQ-254 (implemented): **Playlist**: File > Add to playlist adds a video, and File > Open folder… adds every video of a folder in name order. The player plays the list in order, with Next and Previous (Ctrl/Cmd+Right and Ctrl/Cmd+Left) and stops after the last video. The list is kept for the session only. Acceptance: three files play in order; Next and Previous move between them; after the last one playback stops; a restart has an empty list.
- REQ-255 (implemented): The player's About screen lists the player's own bundled components (its ffmpeg and ffprobe, the icon set, the fonts) with their licences, from the player's own notice file. The editor's notices are not shown there. Acceptance: the About screen of the player names its own components; the editor's list is not in it.
- REQ-256 (implemented): A very large video plays with its sound: the preview copy the player makes keeps the audio track, and the sound stays in step with the picture (REQ-035). Acceptance: a large fixture gives a preview copy with an audio stream; playback keeps the sound in step within 100 ms.
- REQ-355 (implemented): 3.1.2. **File > Open…** accepts several video files at once in its file dialog (multi-selection). The selected videos become the playlist, in the order the dialog gives (name order when it gives none), and play one after the other; a single file is a list of one. Open… replaces the current list (decided by Andrea); the list is kept for the session only (REQ-254). Acceptance: select three files in Open…: they play in order, Next and Previous move between them; a fourth Open… with one file leaves one file in the list.
- REQ-356 (implemented): 3.1.2; amends REQ-254. **File > Add to playlist** can be used at any time, also while a video plays, and adds one or more selected videos at the end of the list without stopping the current video (MINOR). The title shows the file name and its place in the list ("2 of 5"). Acceptance: while the first of two videos plays, add two more: the current video keeps playing, and the two added play after the last video of the list, in the chosen order.

### Playlist and picture (3.2.0)
- REQ-374 (implemented): Amends REQ-254 and REQ-356. A **Playlist** menu shows the list in its order. The chosen video can be moved up or down, or removed; the order changes at once and Next and Previous follow it. Removing the playing video starts the next one, or stops after the last one (MINOR). Acceptance: with three videos, move the third to the first place: Next and Previous follow the new order; remove the second: it is not played; remove the playing video: the next one plays.
- REQ-375 (implemented): When a list of two or more videos is open, the time shown after the current video's total length is the total length of the whole list in brackets, `[hh:mm:ss]`, for example "00:01:30 / 00:03:00 [01:15:00]". It changes when the list changes. Until the length of every video in the list is known, the bracket shows dashes instead of a time, never a partial total (MINOR). A single video shows no bracket (MINOR). Acceptance: a 1-minute and a 2-minute video show [00:03:00]; removing the 2-minute video changes it to [00:01:00].
- REQ-376 (implemented): The player's menu has **Take screenshot**: it saves the picture on the screen at the current moment, as shown (subtitle lines drawn, the 360 view as turned), to a PNG file chosen in a save dialog. The dialog opens in the Pictures folder with the name "video name" plus the time of the frame (MINOR). Nothing is written next to the video (REQ-251, REQ-252). Acceptance: pause on a frame with a subtitle line and choose Take screenshot: the PNG shows the same picture and line; the video's folder has no new file.

## Key decisions
- The player is a separate program so that people who only watch videos get a small, simple tool.
- The player has a start screen like the editor's (REQ-150, 1.6.2).
- The player opens video files only, never projects: a project belongs to the editor and its source files (REQ-001).
- Subtitle positions chosen in the player are not saved, so the player never writes files next to the video.

## Known limitations
- Subtitle positions in the player are lost when the player is closed.
- The player becomes the default for a video type only when the user chooses it in the operating system.
- Picture-based subtitle tracks are not drawn on 360 clips, and a picture subtitle already on screen when a run starts appears at its next change (REQ-253).
- A big video is played through a preview copy that the player makes in the background without a status message.
- The player's Windows, macOS and Linux builds are not yet run on real systems (see [Release verification](business-verification.md)).
- The list total shows dashes until every video's length is known; the Playlist menu and Take screenshot are not yet checked by hand on a real display (manual checks 45 and 46, pending).

## Out of scope
- Editing, cutting, effects, export, streaming, network sources, subtitle styling in the player.
- Saving subtitle positions or the playlist beyond the session (REQ-251, REQ-254).
- A lighter ffmpeg for the player (to be measured first; see the [roadmap](roadmap.md)); picture-in-picture; a second video track.
