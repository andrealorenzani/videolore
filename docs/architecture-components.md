# Architecture — components

Sub-document of [architecture](architecture.md). The module dependencies, the module graph, the threads and the import flow are in [code-architecture](code-architecture.md) (written by the Principal Engineer, checked against the code by the architect on 2026-11-20). The table gives the role of each module. Version 2.0.3 added two pure modules, `measure` (font measurement and the best fit of text) and `placement` (box rules and warnings for texts and subtitle lines); the fonts module gained the font bytes for measuring and drawing. Version 2.0.4 added the modules listed at the end of the table.

| Module | Role |
|---|---|
| `time` | Exact time arithmetic and timestamp formatting (no floating point in cut math). |
| `media` | Information about a media file read from `ffprobe`; status "analysing / ready / rejected"; video or audio-only; since 1.2.0 its text subtitle tracks; since 1.2.6 whether it is 360. |
| `timeline` | The video track: clips, in/out points, order, cuts, fades (since 1.2.0 also from and to black at the video's edges), each clip's volume, and (1.4.0) its clean audio flag; the rules that keep it valid. |
| `export/choice` (1.4.0) | Pure model of the Export dialog: mode, formats, 360 choice, file name and target rules, reasons, and the mapping to the export plan. See [export dialog](architecture-export-dialog.md). |
| `music` | The music track: audio clips that never overlap; place, move, trim, split, delete. |
| `subtitles` | Subtitle lines stored against the source speech, and how they appear on the timeline; since 1.2.0 each line's position on screen. |
| `srt` (1.2.0) | Reads and writes SRT subtitle files (tracks found in videos, SRT export). |
| `align` (1.2.0) | Matches the words of a subtitle track to the recognised speech to time each word. |
| `camera` (1.2.6) | Pure camera maths for 360 video: limits, POV path (linear, shortest way round), drag and zoom mapping, the command file for the `v360` filter (absolute commands, changed values only). Since 1.3.0 also the projection between the sphere and the view. |
| `ass` | Writes the subtitle file (ASS format) used by `ffmpeg` for the chosen style. |
| `history` | Undo and redo of the whole edit (video, music, subtitles, volumes); since 1.1.0 each step has a name ("Undo Split"). |
| `project` | Save/load projects as versioned JSON (version 9 since 1.6.1; opens all older versions; version 10 since 2.0.3, version 11 since 2.0.4); unsaved-changes tracking. See [project files](architecture-projects.md). |
| `format` | Output formats, codecs and which encoder to use, with fallbacks. |
| `render` | Turns the edit into an export plan and the exact `ffmpeg` arguments; also the preview graphs. |
| `ffmpeg` | Finds `ffmpeg`/`ffprobe` (bundled copy first), checks features, runs them, reads progress, cancels. |
| `jobs` | The job queue: limits how many `ffmpeg`/`ffprobe` processes run at once (3 to 6, depending on the chosen level), by priority. Every process goes through it. |
| `export` | Runs an export in the background and writes the result safely; since 0.4.0 tries video hardware first when asked, and redoes the export with the normal method on any problem. |
| `hwenc` (0.4.0) | Video hardware candidates per system, their `ffmpeg` settings, and the background hardware check. |
| `outcheck` (0.4.0) | Checks a hardware export (file details, full decode, sample frames compared with the normal render). |
| `preview` | Decodes frames and audio for the preview player. |
| `proxy` | Lower-resolution preview copies of large sources, and their cache. |
| `stt` | Speech-to-text: audio extraction (since 1.2.0 also of one clip's part, and of subtitle tracks), transcription, word matching jobs, language detection, the three model levels, download and storage. 2.0.3: speech models with word timing presets per level, and recognition in pieces with overlapping margins. |
| `machine` | Detects the computer: cores, memory, operating system, processor features. |
| `settings` (0.3.0) | Program settings file, first-start setup, recommended settings, how each level translates into processes and threads; since 0.4.0 the Export choice and remembered hardware failures; since 1.1.0 the section layout and recent projects. |
| `scratch` (0.3.0) | Private work folders for temporary files, and clean-up after a crash. |
| `notices` | The third-party notices shown in About. |
| `desktop` | Linux application-menu entry and file association; macOS "open document" events. |
| `ui` | Windows and panels, including the Settings window. Kept thin. Since 1.1.0 also: the section layout and borders, the table of actions shared by menus, toolbar and shortcuts, the menu bar, toolbar, status bar, effect and clip popups, and the timeline markers. Since 1.2.0 also the time ruler, dragging effects onto clips and subtitles in the preview. See [user interface](architecture-ui.md). |
| `text` (1.3.0) | Text items: pure generation of their ASS events (fades, blink, slide, cuts at segment boundaries), caps and the font-name rule. 2.0.3: readability choice, box height, best fit from the measured layout. 2.0.4: picture items, the split into `timing`, `layout` and `events`. See [text](architecture-text.md). |
| `fonts` (1.3.0) | Installed fonts: background scan (`fontdb`), family list, name check. 2.0.3: font bytes read on demand (cached), name resolution, copies for the picture. See [text](architecture-text.md#203-readable-text). |
| `measure` (2.0.3) | Pure font measurement: glyph widths and line height of a face, word wrap and the best-fit size of a text box. See [text](architecture-text.md#203-readable-text). |
| `placement` (2.0.3) | Pure box rules for texts and subtitle lines: least move inside the picture, keeping the centre, overlaps and parts outside the picture (warnings only); since 2.0.4 the one box rule `padded_line_rect`. See [subtitles](architecture-subtitles.md#203-placement-and-warnings). |
| `spatial` (2.0.4) | Spatial sound for 360 clips: first-order ambisonic rotation by the camera, the stereo cardioid pair, the WAV writer and the pre-pass runner. See [media pipeline](architecture-media-pipeline.md#spatial-sound-pre-pass-204). |
| `picture` (2.0.4) | Picture items: the PNG file (size from the header, limits), the fit of the image in its box. See [text](architecture-text.md#picture-items-204). |
| `text/` (2.0.4) | Split of `text`: `timing`, `layout` and `events` (the ASS events of texts). See [text](architecture-text.md#text-split-and-one-box-rule-204-qg-16). |
| `autosave` (2.0.4) | Autosave policy (pure), the worker that writes the copy, and the recovery offer. |
| `instance` (2.0.4) | One window: lock file, heartbeat and the inbox of a second start. |
| `hwdec` (2.0.4) | Hardware decoding probe and its arguments for the preview. |
| `siblings` (2.0.4) | Start of the sibling program (editor or player) by its fixed name next to the executable. |
| `selection` (1.3.0) | The one selection of subtitle lines and text items, and its pure transitions (click, Shift range, group move). |
| `spherical` (1.3.0) | Writes the Spherical Video V1 marker into MP4 and MOV exports of 360 video, which ffmpeg 6.1.1 drops on re-encode. |
| `keyframes` (1.5.0) | Pure keyframe lists for Scale and Move: linear between keys, value held before the first and after the last, limits and unique times. |
| `frame` (1.5.0) | Pure frame model: Change aspect shape, fit and fill, and the frame size from the base size. See [frame](architecture-frame.md). |
| `lut` (1.6.0) | Pure reader of `.cube` colour tables (3D and 1D), with limits; the normalised cube written for `ffmpeg`. See [LUT](architecture-lut.md). |
| `splash` (1.6.0, 1.6.2) | Pure splash layout, text fit, summaries and the bundled pictures of both programs (`EDITOR`, `PLAYER`). See [splash](architecture-splash.md). |
| `ui::splash_window` (1.6.2) | The splash window, drawn for either program from its own picture and summary; the editor's `ui::splash_ui` is a thin adapter. See [splash](architecture-splash.md). |
| `ui::icons` (1.6.0) | The icon font and the icon constants shared by both programs. See [icons](architecture-icons.md). |
| `player/` (1.6.0) | The `videolore-player` program: its state and window, built on the engine above. See [player](architecture-player.md). |
| `app` | Application state and wiring between the UI and background workers; since 1.1.0 also which popups are open, recent projects and dropped files; since 1.2.0 applying effects, the subtitle creation dialogs and the selection of subtitle lines. |
