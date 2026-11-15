# Business — Music and volume

Back to the [index](business.md).

## Features
- One music track, shown below the video track in the timeline panel.
- Audio files from the media list are placed on it, then moved, trimmed, split or deleted; every action can be undone.
- Music is mixed with the audio of the video in preview and export, with a separate volume (0–100%) and mute for each.
- (1.2.0) Each video clip can have its own volume (0–100%) and mute, on top of the video sound level: apply Volume to a clip from the Effects list or the right-click menu, or use Clip details.

## Requirements
- REQ-019 (implemented): There is one music track below the video track; music clips never overlap. Audio files from the media list can be placed on it, moved, trimmed, split and deleted, and all these actions can be undone.
- REQ-020 (implemented): Music is mixed with the video audio in preview and export; the video audio and the music each have a volume from 0 to 100% and a mute switch, saved in the project. Since 1.2.0 each video clip also has its own volume and mute (REQ-084), multiplied by the video audio level.
- REQ-043 (implemented): Editing the video does not move the music; music that continues past the end of the video is cut there with a 0.5 s fade-out; where there is no music clip there is no music.
- REQ-084 (implemented): Volume as an effect (REQ-082). **On a clip**: the clip's own sound level from 0 to 100% (same perceptual curve as the other sliders) and a mute switch. **On the whole video**: the Volume popup of REQ-020; the video sound level multiplies every clip's level. A clip whose level is not 100% or that is muted shows a marker at its start (REQ-077); the clip volume is also in Clip details (REQ-078). The clip volume is kept when the clip is split (both halves), copied and pasted or moved; Merge needs equal volumes and is otherwise greyed out with "The clips have different volumes". Applying Volume opens its popup; the undo step is recorded when a value changes (slider release or mute). Acceptance: (a) a clip at 50% with the video at 100% sounds the same as the clip at 100% with the video at 50%; (b) a muted clip is silent in preview and export while the others are not; (c) splitting keeps the level on both halves; (d) a clip level other than 100%, or a muted clip, forces a re-encode (REQ-023).

## Key decisions
- Only audio files can go on the music track.
- Music does not follow video edits (no ripple): the music track is an independent background bed, which keeps it predictable.
- Volume sliders follow a perceptual curve (50% sounds roughly half as loud, about -12 dB), not a linear one.
- Any music in the project means the export is re-encoded.
- (1.2.0) A clip's volume can only lower its sound (0–100%, no boost); it belongs to the clip and travels with it through split, copy and move.

## Out of scope
- Automatic lowering of music under speech (ducking).
- Fades on individual music clips, volume curves over time, per-clip volume of music clips (effects apply only to video clips).
- Volume above 100% (boost).
- More than one music track.
