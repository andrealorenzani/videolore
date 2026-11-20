# Architecture: export dialog, formats and audio formats (1.4.0)

Part of [Architecture](architecture.md). Requirements REQ-118 to REQ-122 (see [export](business-export.md)); the 360 choice is also in [360 video](architecture-360.md#export-as-360-video-130).

## Opening the dialog
- **Export** (File > Export…, the Export toolbar button and Ctrl/Cmd+E) opens a modal dialog instead of exporting at once. During an export the same action still cancels it.
- The dialog is a window of the program (like the Settings window), not a system dialog, so it looks the same everywhere and is tested without a window.
- Session memory: the last choices (mode, formats, 360 choice, folder) are kept while the program runs. They are **not** saved in the project or in the settings file.

## Modes
- **Full video:** the export of the earlier versions, with the video and audio formats below.
- **Only audio:** the sound of the whole timeline as one file (video sound, music, volumes and clean audio), in the audio format below. No video is decoded.
- **Only subtitles (.srt):** the same text as File > Export subtitles (.srt)…; that menu item stays.
- Fields that do not apply to the chosen mode are greyed out with a reason. Only subtitles is greyed out ("No subtitles on the timeline") when there are none.

## Export file
- One field holds the folder and the file name; a "..." button opens a folder chooser. The first time the name is the project's name in the project's folder; the folder is the project's folder, then the last export folder of the session, then the user's documents folder (assumption).
- The extension follows the chosen format (.mp4, .mov, .m4a, .mp3, .srt) and is set automatically. An extension typed by the user that is one of these is replaced; other dots are kept.
- Export is disabled, with the reason shown, when the name is empty or invalid, when the folder does not exist or cannot be written, or when the file would replace a file used by the project (a media file or the project file itself). An existing file asks for confirmation before it is replaced.
- Name rules: no `/ \ : * ? " < > |`, no control characters, not `.` or `..`, no trailing dot or space, not a Windows reserved name (CON, PRN, AUX, NUL, COM1–COM9, LPT1–LPT9), at most 200 characters.
- Cancel writes nothing. Partial files are written next to the target and renamed when complete (as before).

## Video format (Full video only)
- **Same as source** (default): the automatic format of REQ-022; stream copy when REQ-023 allows it.
- Presets: a fixed table of 12 entries, shown like "1920x1080 25fps, libx264, .mp4". Each is always re-encoded and can use the hardware export.

| Size | Frame rate | Codec | File |
|---|---|---|---|
| 1920x1080 | 25, 30, 60 fps | libx264 | .mp4 |
| 1920x1080 | 25 fps | libx264 | .mov |
| 1920x1080 | 25, 30 fps | libx265 | .mp4 |
| 1280x720 | 25, 30 fps | libx264 | .mp4 |
| 1280x720 | 25 fps | libx265 | .mp4 |
| 854x480 | 25 fps | libx264 | .mp4 |
| 3840x2160 | 25 fps | libx265 | .mp4 |
| 3840x2160 | 30 fps | libx265 | .mov |

- Pictures keep their shape and are letterboxed, never stretched. HEVC in MP4 or MOV gets the hvc1 tag.
- A reset button sets "Same as source".
- The list is fixed: sizes, frame rates, bitrates and codecs outside it cannot be chosen (documented limitation).

## Audio format (Full video and Only audio)
- **Same as source:** today's automatic audio (AAC; to be verified in the code path), stream copy when REQ-023 allows it.
- **aac** and **mp3**. mp3 is offered only when the bundled `ffmpeg` can encode it (its encoder list is checked at start-up, as for every format; libmp3lame).
- A chosen audio format always re-encodes the audio. Only audio writes .m4a for AAC and .mp3 for MP3; "Same as source" in Only audio is AAC in .m4a.
- A reset button sets "Same as source".

## 360 choice
- Shown only for a project whose video clips are **all** 360 clips: **360 video** or **Flat camera view**. Default 360 video (assumption).
- Projects without 360 clips, and mixed projects, show no choice; a mixed project is exported as a flat camera view without a message.

## What the export does with the choices
- The choices become the same export plan as before: the chosen preset or the automatic format, the chosen audio encoder, the 360 choice. Full video uses the existing worker; Only audio runs one `ffmpeg` process with the timeline's audio graph and no video branch; Only subtitles writes the SRT text.
- A preset or an audio format other than "Same as source" forces a re-encode (REQ-023). Clean audio on any clip forces it too.
- Export of the chosen file goes through the same job limits and cancel rules as any export.

## Tests
- Pure: name rules (reserved names, separators, length), extension replacement, clash with project media and the project file, folder checks, reasons for greyed fields, the 360 rule (all-360, mixed, none), mapping from choices to the plan and the arguments.
- Presets: the argv of "1280x720 25fps libx264 .mp4" has that size, rate and codec (ffmpeg probe of the output); libx265 in MP4 has hvc1; Same as source with identical clips and no effects re-encodes nothing.
- Only audio: a 10 s timeline with music gives a 10 s audio file with the music at its volume; the mp3 option is offered only when the encoder is listed.
- Interface (headless): Ctrl/Cmd+E and File > Export open the dialog; "..." changes the folder; a missing folder disables Export with its reason; an existing file asks first; Cancel writes nothing; a second opening shows the last choices; the dialog fits 800x500 with every control reachable.

## Risks
- Codec names in the main dialog may confuse people who do not know them (product risk accepted by Andrea; wording can change).
- Export choices are lost when the program restarts (by design).
- 4K and HEVC re-encodes are slow; no speed is promised.
