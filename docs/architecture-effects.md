# Architecture: effects on clips and on the whole video

Part of [Architecture](architecture.md). Since 1.2.0 (requirements REQ-082..084, REQ-011, REQ-020 and REQ-023 reworded, see [business](business.md)). Subtitles, the third effect, are described in [subtitles](architecture-subtitles.md) and [subtitle tracks](architecture-subtitle-tracks.md).

## Applying an effect
- Each effect (Fade through black, Subtitles, Volume) can be applied **to one video clip** or **to the whole video**. Every way of doing it produces the same single command ("apply this effect to this target"), so the result is identical whichever way is used:
  - dragging the effect from the Effects list onto a video clip;
  - right-clicking a clip, Effects ▸ <effect>;
  - Effects menu ▸ Apply to selected clip ▸ / Apply to whole video ▸;
  - double-clicking the effect in the Effects list (whole video).
- A single click on an effect still opens its popup. Applying opens the popup of the fade, volume or subtitles that was created or changed; the popup title says "— whole video" or "— <clip name>".
- **Drag and drop inside the window** uses egui's built-in drag-and-drop support (no new crate). Over a video clip the clip is highlighted and the pointer shows that a drop is possible; anywhere else (music track, empty space) the pointer shows "not allowed" and dropping does nothing. This works on every system, including Linux Wayland, because it does not involve the operating system (the Wayland limitation concerns files dropped from outside, see [user interface](architecture-ui.md#drag-and-drop)).
- **Undo:** applying a fade or subtitles is one undo step. Applying Volume only opens its popup; the undo step is recorded when a value actually changes (slider released, mute switched), so opening the popup without changing anything leaves no empty step.

## Fade through black
- **On a clip:** fades at both edges of the clip. A boundary with a neighbour gets the usual fade-out plus fade-in (as in earlier versions); if the clip is the first one it also fades in from black at the start, if it is the last one it fades out to black at the end. Removing the effect from a clip removes the fades at both its edges.
- **On the whole video:** one fade from black at the very start and one to black at the very end.
- **Design decision:** the start and end fades belong to the **edges of the video**, not to a clip. They stay at the start and end whatever is edited later (cutting or moving the first clip, adding clips). The alternative, attaching them to the first and last clip, was rejected because cutting the first clip would remove the fade from black. Consequence: if the first clip is moved into the middle, the fade from black stays at the start of the video.
- Length: 0.5 s, or half the clip for clips shorter than 1 s; picture and sound together (unchanged rule).
- Each fade (start, end, each boundary) has its own marker on the timeline and can be removed on its own. The "+" handle at boundaries without a fade stays.
- Projects from 1.1 and earlier have no start or end fades and look exactly as before.

## Clip volume
- Each video clip has its own sound level, 0–100% (no boost), and a mute switch. The project-wide video level still exists and multiplies every clip level: a clip at 50% with the video at 100% sounds the same as a clip at 100% with the video at 50%.
- The perceptual curve is unchanged: gain = (level / 100)², so 50% is about −12 dB; muted is silence.
- The clip level follows the clip: a split gives both halves the same level, copy/paste and reordering keep it. Merging two clips needs equal levels; otherwise Merge is greyed with "The clips have different volumes".
- A clip that is not at 100% or is muted shows a marker at its start; clicking it opens that clip's volume popup. The level and mute are also in Clip details.
- **In the export command:** a clip with a level other than 100% gets a `volume` filter in its own audio chain, before its fades. Clips at 100% get nothing, so projects that do not use clip volume produce exactly the same `ffmpeg` command as 1.1 (checked by a test against a saved 1.1 command).

## When the export must re-encode
In addition to the earlier rules (see [media pipeline](architecture-media-pipeline.md#export)), a start or end fade, or any clip whose level is not 100% or which is muted, rules out stream copy.

## Tests
- Timeline rules: fades on clip 2 of 3 (two boundaries, no start or end fade), on clip 1 (start fade plus one boundary), whole video (start and end; still at the edges after cutting the first clip), fade length on short clips, removal; clip volume through split, copy, reorder and merge.
- Export command: `volume=0.250000` for a clip at 50%, equivalence of clip and video levels, muted clip silent, re-encode forced.
- With real `ffmpeg`: first and last frame black with a whole-video fade; a muted clip is silent in the export while the others are not.
- Interface: dropping an effect on clip 2 gives the same result as the right-click menu; dropping on the music track changes nothing; one undo restores.
