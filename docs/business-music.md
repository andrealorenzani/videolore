# Business — Music

Back to the [index](business.md).

## Features
- One music track, shown below the video track in the timeline panel.
- Audio files from the media list are placed on it, then moved, trimmed, split or deleted; every action can be undone.
- Music is mixed with the audio of the video in preview and export, with a separate volume (0–100%) and mute for each.

## Requirements
- REQ-019 (implemented): There is one music track below the video track; music clips never overlap. Audio files from the media list can be placed on it, moved, trimmed, split and deleted, and all these actions can be undone.
- REQ-020 (implemented): Music is mixed with the video audio in preview and export; the video audio and the music each have a volume from 0 to 100% and a mute switch, saved in the project.
- REQ-043 (implemented): Editing the video does not move the music; music that continues past the end of the video is cut there with a 0.5 s fade-out; where there is no music clip there is no music.

## Key decisions
- Only audio files can go on the music track.
- Music does not follow video edits (no ripple): the music track is an independent background bed, which keeps it predictable.
- Volume sliders follow a perceptual curve (50% sounds roughly half as loud, about -12 dB), not a linear one.
- Any music in the project means the export is re-encoded.

## Out of scope
- Automatic lowering of music under speech (ducking).
- Fades on individual music clips, volume curves over time.
- More than one music track.
