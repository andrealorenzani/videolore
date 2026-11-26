# Business — Screen recorder (videolore-screen-spy)

Back to the [index](business.md).

**videolore-screen-spy** is a third, small program of the project, built in 3.1.2. It records one screen to a video file, with a live preview, and does nothing else: no editing, no effects, no cloud. It shares the project's build and packaging, like the player, but it is a separate program with its own executable and installers.

## Features (3.1.2)
- **Preview**: a live picture of the chosen screen, before and during recording.
- **Screen**: a list of the screens connected to the computer; one is chosen.
- **Output file**: a text box with the file name and a **Choose…** button for the folder and the name.
- **Video format**: a short list of the most common formats (four choices).
- **Sound**: computer sound and the microphone, mixed into one sound track (REQ-357).
- **Audio format**: AAC, MP3 or WAV (WAV is saved as an MKV file).
- **Start / Stop recording**: one button; when recording it reads "Stop recording"; while the start checks run it reads "Cancel start".

## Requirements

### The program
- REQ-341 (implemented): videolore-screen-spy is a separate program in the same project, with its own executable and its own installers for Windows, macOS and Linux, built like the player's (REQ-138, REQ-040 to REQ-042). It works offline, it has no account, and it never edits, saves projects or exports. The editor's installer contains no screen recorder and the recorder's installer contains no editor or player. Acceptance: the recorder starts and records on Windows, on macOS 14.6 or later and on Linux under X11 or XWayland (REQ-358); on an older Mac it starts and shows the message of REQ-357; each installer lists only its own program.

### Window and preview
- REQ-342 (implemented): The window has four parts: **Preview**, **Screen**, **Output** (file name and folder, video format, audio format) and one **Start recording** button. Nothing else is needed to record; a File menu holds Quit and the About screen, which lists the recorder's own bundled components and licences (as REQ-255). Acceptance: all four parts are visible at the default window size; the About screen names only the recorder's components.
- REQ-343 (implemented): The preview shows the chosen screen live, before recording and while recording, at a low frame rate so that it does not slow the recording. The preview is not part of the file. Acceptance: moving a window on the screen shows in the preview within 1 second; the file has the frame rate of the chosen video format, not the preview's.

### Screen
- REQ-344 (implemented): The **Screen** list shows each connected screen by its number and its size (for example "Screen 2, 1920x1080"). The first screen is chosen by default; the list refreshes when a screen is connected or removed; the choice is locked while recording. Acceptance: with two screens both are listed; choosing the second changes the preview; the choice is disabled during a recording.

### Output file
- REQ-345 (implemented): **Output** has a text box with the file name and a **Choose…** button that opens a save dialog for the folder and the name. The default folder is the system's video folder (Movies on macOS, Videos on Windows and Linux) and the default name is "screen-recording" with the date and time of the start (MINOR). The file extension follows the audio format: .mp4 for AAC and MP3, .mkv for WAV (REQ-348). Acceptance: typing a name sets the file name; Choose… sets folder and name together; a new start of the program shows the default name; a WAV recording is saved as a .mkv file, the other audio formats as .mp4.
- REQ-346 (implemented): If the chosen file already exists, a popup asks "Replace the file?" with **Replace** and **Cancel**; Cancel starts nothing and keeps the existing file (MINOR). Acceptance: an existing file is not changed until Replace is chosen.

### Formats
- REQ-347 (implemented): The **Video format** list has four choices, each a size and a frame rate: 1280x720 30 fps; 1920x1080 30 fps (default); 1920x1080 60 fps; 4K UHD 2160p (3840x2160) 30 fps (decided by Andrea). The container is MP4, or MKV when the audio format is WAV (REQ-348; decided by Andrea 2026-11-26). Acceptance: a five-second recording with each choice has that size and frame rate; the list shows exactly these four.
- REQ-348 (implemented): The **Audio format** list has three choices: AAC (default), MP3 and WAV. The chosen format is the format of the file's sound, which is the mixed sound of REQ-357 (MINOR: sound inside the video file). The container follows the format: AAC and MP3 go into an MP4 file, WAV into an MKV file (decided by Andrea 2026-11-26: WAV kept, Matroska added; MP4 cannot hold WAV). No pair is refused. Acceptance: each audio choice records a sound track of that format: AAC and MP3 in a .mp4 file, WAV in a .mkv file; each file plays in the player (REQ-138) with its sound.

### Sound
- REQ-357 (implemented): The recorder's sound is **computer sound and the microphone, mixed into one sound track** of the video file; there is no choice of source (decided by Andrea). Computer sound needs macOS 14.6 or later (decided by Andrea 2026-11-26); an older Mac still runs the recorder, and Start refuses with a message that names the macOS version needed. If one of the two cannot be recorded (no microphone, or computer sound not available or not permitted), **Start recording** does not start and a message names the missing source (MINOR: no silent single-source recording). On Linux the sound comes from the system's default input and default output: when the default input is a monitor (the computer's own sound), Start is refused with a message, because the computer's sound would be recorded twice; the user changes the default input in the system's sound settings. On macOS, a microphone permission not yet asked is asked when Start is pressed, and recording waits for the answer. Acceptance: play a sound on the computer while speaking into the microphone: the file has both in its one sound track; with the microphone denied, the message names the microphone and nothing is recorded; on macOS 14.5 or older, Start shows the message naming macOS 14.6 and nothing is recorded; on Linux with a monitor as default input, Start shows the message and nothing is recorded.

### Platforms
- REQ-358 (implemented): On Linux the recorder works on an X11 session and on XWayland. A native Wayland session is not supported in 3.1.2 (decided by Andrea 2026-11-26): a message appears before Start that says screen recording needs an X11 session and that Wayland recording comes later; nothing is recorded. Acceptance: on an X11 session a 10-second recording works; on a native Wayland session the message appears before Start, no file is created and the button still reads Start recording.

### Recording
- REQ-349 (implemented): **Start recording** starts the recording of the chosen screen to the output file; the button then reads **Stop recording** and the Screen, Output and Preview choices are locked. While the start checks run (the sound sources are opened first, a few seconds), the button reads **Cancel start**: a click stops the start, nothing is recorded and the choices unlock (MINOR). **Stop recording** ends the file, which is complete and playable, and the button reads **Start recording** again; a second recording can follow in the same session. Acceptance: start and stop a 10-second recording: the file opens in the player (REQ-138) and lasts about 10 seconds; the button label changes at each press; a second recording works without restarting the program; Cancel start during the checks creates no file and unlocks the choices.
- REQ-350 (implemented): If the recording stops by itself (the disk is full, the screen is removed, the capture fails), the file keeps what was written, the button reads **Start recording** again and a message names the cause. Acceptance: with a full disk or a removed screen the message appears and the part already written plays in the player.
- REQ-351 (implemented): Closing the window during a recording asks "Stop the recording and close?" with **Stop and close** and **Cancel**; Stop and close finishes the file before the program ends (MINOR). Acceptance: after Stop and close the file plays completely; Cancel keeps the program and the recording running.

### Settings
- REQ-352 (implemented): The program remembers the last folder, the video format and the audio format in a settings file in the user's settings folder, never next to the recording (as REQ-252). Acceptance: change the folder and the formats, restart: the same choices; the recording folder has no settings file.

## Key decisions (3.1.2)
- A separate small program, like the player, so that people who only record a screen get a simple tool (MINOR).
- Minimal by request: one screen, one button, a short list of formats; no region selection, no webcam, no pause, no hotkeys (out of scope).
- The sound is computer sound and the microphone, mixed into one track (decided by Andrea, REQ-357); REQ-348 sets only the audio format of that track.
- The recorder does not share the editor's project file; its output is a plain video file.
- WAV sound is kept and goes into an MKV file (decided by Andrea 2026-11-26); MP4 cannot hold WAV.
- Linux: X11 and XWayland only in 3.1.2; native Wayland recording comes later (decided by Andrea 2026-11-26).
- macOS: computer sound needs macOS 14.6 or later (decided by Andrea 2026-11-26); the recorder's own installer minimum is not 14.6, so older Macs can open it and read the message at Start (MINOR, from Andrea's answer).

## Known limitations
- Computer sound and the microphone need the system's permission; the recorder names the missing source in its message (REQ-357).
- macOS: the microphone is asked for and awaited before the first recording, so its first recording is not silent. The permission for computer sound is given by macOS when the sound is first taken; until it is given, the computer's sound may be silent, also on the first recording (not verified on a Mac).
- Computer sound on macOS needs macOS 14.6 or later; an older Mac cannot record and gets a message at Start (REQ-357).
- Linux: a monitor as the default input refuses Start (REQ-357), and the recorder does not let the user pick the source; a cancelled start waits for the source being tried, which can take a few seconds.
- Linux: X11 and XWayland only; on a native Wayland session the recorder shows a message before Start and records nothing (REQ-358).
- Linux: the picture and the sound start together; on Windows and macOS the offset between them at the start is not measured.
- Windows and macOS are not verified on real hardware (screen capture offsets and scaling, sound capture, permissions, installers); their manual checks stay pending (see [verification](business-verification.md)). Whether computer sound needs an extra system driver on some systems is not known.
- Platform permissions for screen capture (macOS) may ask the user for approval on first use; the recorder explains it in the message, not in advance.
- At 1080p60 and 4K the recording may not keep real time on a weak computer (not measured).
- A recording is one video file; a long recording is limited only by the disk space (assumption).
- WAV sound is saved in an MKV file, which may open less readily in some players than MP4 (assumption; checked by hand, see [verification](business-verification.md)).
- Only the containers and codecs that the bundled ffmpeg supports are offered.

## Out of scope
- Region or window capture, webcam, pause and resume, hotkeys, scheduled recordings, cursor options.
- Recording on a native Wayland session in 3.1.2 (later, REQ-358).
- Recording one sound source only (computer sound or microphone alone), and separate sound tracks.
- Editing, trimming, effects, subtitles or export of the recording (the editor and the player do that work on the file).
- Streaming, upload, cloud storage, accounts; custom resolutions, frame rates and bitrates.
