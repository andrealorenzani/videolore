# Business — Music and volume

Back to the [index](business.md).

## Features
- One music track, shown below the video track in the timeline panel.
- Audio files from the media list are placed on it, then moved, trimmed, split or deleted; every action can be undone.
- Music is mixed with the audio of the video in preview and export, with a separate volume (0–100%) and mute for each.
- (1.2.0) Each video clip can have its own volume (0–100%) and mute, on top of the video sound level: apply Volume to a clip from the Effects list or the right-click menu, or use Clip details.
- (1.4.0) **Clean audio**: an effect that reduces background noise (wind, music, people chatting) in the video's sound, on a clip or on the whole video (REQ-114). It is not a perfect cleaner: it tries to lower the noise.

## Requirements
- REQ-019 (implemented): There is one music track below the video track; music clips never overlap. Audio files from the media list can be placed on it, moved, trimmed, split and deleted, and all these actions can be undone.
- REQ-020 (implemented): Music is mixed with the video audio in preview and export; the video audio and the music each have a volume from 0 to 100% and a mute switch, saved in the project. Since 1.2.0 each video clip also has its own volume and mute (REQ-084), multiplied by the video audio level.
- REQ-043 (implemented): Editing the video does not move the music; music that continues past the end of the video is cut there with a 0.5 s fade-out; where there is no music clip there is no music.
- REQ-084 (implemented): Volume as an effect (REQ-082). **On a clip**: the clip's own sound level from 0 to 100% (same perceptual curve as the other sliders) and a mute switch. **On the whole video**: the Volume popup of REQ-020; the video sound level multiplies every clip's level. A clip whose level is not 100% or that is muted shows a marker at its start (REQ-077); the clip volume is also in Clip details (REQ-078). The clip volume is kept when the clip is split (both halves), copied and pasted or moved; Merge needs equal volumes and is otherwise greyed out with "The clips have different volumes". Applying Volume opens its popup; the undo step is recorded when a value changes (slider release or mute). Acceptance: (a) a clip at 50% with the video at 100% sounds the same as the clip at 100% with the video at 50%; (b) a muted clip is silent in preview and export while the others are not; (c) splitting keeps the level on both halves; (d) a clip level other than 100%, or a muted clip, forces a re-encode (REQ-023).

### Clean audio (1.4.0)
- REQ-114 (implemented): **Clean audio** is an effect of the video's sound: the "Clean audio" row of the Effects list (REQ-075). It is applied to a clip, or to the whole video (every video clip), as Volume is (REQ-082). It reduces background noise such as wind, music or people chatting while the speech stays. It does not need to be perfect and has no strength setting: a clip has it on or off. It works in the preview and in the export. Its popup explains the effect and offers Remove. It is applied before the clip's own volume (REQ-084); the music track is not changed (REQ-020). A clip with it shows a marker at its start (REQ-077). Split keeps it on both halves; copy, paste and move keep it; Merge needs the same setting, as with volumes. Any clip with it forces a re-encode (REQ-023). Applying and removing are each one undo step. Acceptance: (a) on a reference clip with speech and added wind, music or chatter, with Clean audio the noise-only parts are at least 6 dB lower and the speech parts stay within 3 dB of the original (targets; assumption). Measured by the Principal Engineer on a generated reference clip: noise-only parts about 9 dB lower, speech within 0.2 dB; (b) without it the sound is unchanged; (c) the music sounds the same with and without it; (d) a clip with it forces a re-encode; (e) a split keeps it on both halves; (f) one undo removes it.

## Key decisions
- Only audio files can go on the music track.
- Music does not follow video edits (no ripple): the music track is an independent background bed, which keeps it predictable.
- Volume sliders follow a perceptual curve (50% sounds roughly half as loud, about -12 dB), not a linear one.
- Any music in the project means the export is re-encoded.
- (1.2.0) A clip's volume can only lower its sound (0–100%, no boost); it belongs to the clip and travels with it through split, copy and move.
- (1.4.0) Clean audio works on the video's sound only, as an on/off effect with no technical controls. It is an effect for everyday noisy clips, not a studio tool.

## Known limitations
- (1.4.0) Clean audio has one fixed strength. It reduces noise but does not remove it completely; very quiet noise can be removed entirely.
- (1.4.0) Quiet speech (about -32 dB in manual tests) can lose up to about 2.5 dB.
- (1.4.0) Its cost in the preview on weak computers has not been measured.
- (1.4.0) It works on the video's sound only, not on the music track.

## Out of scope
- Automatic lowering of music under speech (ducking).
- (1.4.0) A strength or amount setting for Clean audio; cleaning the music track; voice isolation or speech enhancement beyond noise reduction.
- Fades on individual music clips, volume curves over time, per-clip volume of music clips (effects apply only to video clips).
- Volume above 100% (boost).
- More than one music track.
