# Business

## Purpose
videolore is a simple, offline desktop video editor for people who make short clips for social media. It focuses on fast cutting and joining of clips, simple fade transitions and quick export; automatic subtitles and background music are planned.

All edits are non-destructive: source files are never modified.

## Users
- **Social content creators** who are not editing experts and want to go from raw footage to a publishable clip quickly.
- **Hobbyists** editing personal videos.

Professional editors needing multitrack video, color grading or advanced effects are not the target.

## Features
- **Media and projects**: import video files into a media list, save and reopen projects, undo/redo of at least 50 steps.
- **Timeline editing**: append, split, cut/copy/paste, merge, trim and reorder segments; the timeline closes gaps automatically.
- **Transitions**: fade through black between clips.
- **Preview**: play, pause and seek, with effects visible.
- **Export**: automatic output format, as fast as possible, with progress and cancel.
- **Layout**: fixed five-panel window with a command bar and keyboard shortcuts.
- **Music** (planned, 0.1.4): one background music track mixed with the video audio.
- **Subtitles** (planned, 0.1.5): automatic offline subtitles with social-media styles.

## Requirements
- REQ-001 (implemented): Editing is non-destructive; source media files are never modified.
- REQ-002 (implemented): Media files can be imported into a media list showing name and duration; unsupported files are rejected with a clear message. Audio-only files are accepted into the list but can only be placed on the music track once music (REQ-019) exists.
- REQ-003 (implemented): A project can be saved and reopened; missing source files are reported by file name.
- REQ-004 (implemented): Undo and redo support at least 50 steps.
- REQ-005 (implemented): A media item can be appended to the end of the timeline.
- REQ-006 (implemented): A segment can be split at the playhead.
- REQ-007 (implemented): Segments can be cut, copied and pasted; the timeline ripples so no gaps remain.
- REQ-008 (implemented): Contiguous segments from the same source with contiguous source ranges can be merged; the action is disabled otherwise.
- REQ-009 (implemented): A segment can be trimmed at either edge, within the bounds of its source.
- REQ-010 (implemented): Segments can be reordered; the timeline ripples.
- REQ-011 (implemented): A transition between clips is a fade-out followed by a fade-in through black, on video and audio, and can be removed; each fade lasts 0.5 s, or half the clip length for clips shorter than 1 s, so fades never overlap.
- REQ-012 (implemented): A zoom slider scales the timeline from showing the whole project to single-frame level.
- REQ-013 (implemented): The preview supports play, pause and seek, and shows effects.
- REQ-014 (planned): Subtitles are generated automatically and offline; the speech model is downloaded once after explicit user consent (or provided by the user as a local file), after which everything works offline.
- REQ-015 (planned): The subtitle language is detected automatically and can be overridden by the user.
- REQ-016 (planned): Subtitle text and timing can be edited.
- REQ-017 (planned): At least 3 social-media subtitle style presets are available, including word-by-word highlight; subtitles are burned into the video.
- REQ-018 (planned): Subtitles follow timeline edits.
- REQ-019 (planned): A single music track holds music clips that do not overlap.
- REQ-020 (planned): Music is mixed with the video audio, with separate volumes for each.
- REQ-021 (implemented): Export shows progress and can be cancelled; a cancelled export leaves no partial file.
- REQ-022 (implemented): The output format is chosen automatically: if all clips share the same format it is kept; otherwise the lowest by pixel count, then frame rate, then bitrate is used, with letterboxing and never stretching.
- REQ-023 (implemented): Export does as little processing as possible: stream copy (no re-encode) when all clips share an identical format, there are no effects (fades; later subtitles/music) and every cut starts on a keyframe; otherwise a single re-encode pass, always frame-accurate.
- REQ-024 (implemented): The window has a fixed five-panel layout: top 60% (media list 30%, preview 70%), middle 30% (timeline 70%, effects 30%), bottom 10% command bar including the zoom slider.
- REQ-025 (implemented): Every operation is reachable from the command bar or panels; keyboard shortcuts exist for split, cut, copy, paste, undo, redo and play/pause.

## Constraints
- **ffmpeg 4.4 or newer must be installed by the user**; it is not bundled in 0.1.3. If it is missing, the application shows a clear blocking message with installation hints.
- The product works fully offline. The only network access is the one-time, user-approved download of the speech model for subtitles (planned).

## Key decisions
- Output format is automatic: there is no export settings dialog.
- Stream copy is used only when the result is guaranteed frame-accurate; correctness comes before speed.
- Fade length is fixed (not user-tunable) to keep the editor simple.
- The window layout is fixed; panels cannot be moved or resized.

## Out of scope
- Multiple video tracks, overlays, picture-in-picture.
- Color grading, keyframe animation, transitions other than fade through black.
- Accounts, cloud storage, sync, collaboration, direct upload to social networks.
- Mobile versions.
- Export settings dialog.
- Bundled ffmpeg (0.1.3).
