# Architecture: effects on clips and on the whole video

Part of [Architecture](architecture.md). Since 1.2.0 (requirements REQ-082..084, REQ-011, REQ-020 and REQ-023 reworded, see [business](business.md)). Subtitles, the third effect, are described in [subtitles](architecture-subtitles.md) and [subtitle tracks](architecture-subtitle-tracks.md).

## Applying an effect
- Each effect (Fade through black, Subtitles, Volume, and Clean audio since 1.4.0) can be applied **to one video clip** or **to the whole video**. Every way of doing it produces the same single command ("apply this effect to this target"), so the result is identical whichever way is used:
  - dragging the effect from the Effects list onto a video clip;
  - right-clicking a clip, Effects ▸ <effect>;
  - Effects menu ▸ Apply to selected clip ▸ / Apply to whole video ▸ (since 1.4.0 the Effects menu lists only effects; item commands such as Align or Reset are in the Edit menu);
  - double-clicking the effect in the Effects list (whole video).
- A single click on an effect still opens its popup. Applying opens the popup of the fade, volume or subtitles that was created or changed; the popup title says "— <clip name>". Built 2.2.0: the title says "— whole video" for Change aspect only, because a whole-video Scale, Move or other effect is a copy on each clip and there is no whole-video popup (REQ-148, REQ-076 amended; the built 1.6.1 to 2.1.0 behaviour is described in [whole video per clip](#whole-video-per-clip-161) and [frame](architecture-frame.md)).
- **Drag and drop inside the window** uses egui's built-in drag-and-drop support (no new crate). Over a video clip the clip is highlighted and the pointer shows that a drop is possible; anywhere else (music track, empty space) the pointer shows "not allowed" and dropping does nothing. This works on every system, including Linux Wayland, because it does not involve the operating system (the Wayland limitation concerns files dropped from outside, see [user interface](architecture-ui.md#drag-and-drop)).
- **Undo:** applying a fade or subtitles is one undo step. Applying Volume only opens its popup; the undo step is recorded when a value actually changes (slider released, mute switched), so opening the popup without changing anything leaves no empty step.

## Fade through black
- Since 1.3.0 the user sees the name **Fade IN/OUT** instead of "Fade through black" everywhere (label change only; the internal name and the project format are unchanged). The text effects Fade IN and Fade OUT of text items are separate, see [text items](architecture-text.md).
- **On a clip:** fades at both edges of the clip. A boundary with a neighbour gets the usual fade-out plus fade-in (as in earlier versions); if the clip is the first one it also fades in from black at the start, if it is the last one it fades out to black at the end. Removing the effect from a clip removes the fades at both its edges.
- **On the whole video (until 1.6.0):** one fade from black at the very start and one to black at the very end, attached to the edges of the video.
- **Changed in 1.6.1 (REQ-083):** the whole-video fades are applied to the clips: a fade in at the first clip's start and a fade out at the last clip's end. The start and end fades belong to the clip they sit on: they move with it and are removed with it (the 1.2.0 rule "attached to the video edges" is reversed). See [whole video per clip](#whole-video-per-clip-161).
- Length: 0.5 s, or half the clip for clips shorter than 1 s; picture and sound together (unchanged rule).
- Each fade (start, end, each boundary) has its own marker on the timeline and can be removed on its own. The "+" handle at boundaries without a fade stays.
- Projects from 1.1 and earlier have no start or end fades and look exactly as before.

## Clip volume
- Each video clip has its own sound level, 0–100% (no boost), and a mute switch. The project-wide video level still exists and multiplies every clip level: a clip at 50% with the video at 100% sounds the same as a clip at 100% with the video at 50%.
- The perceptual curve is unchanged: gain = (level / 100)², so 50% is about −12 dB; muted is silence.
- The clip level follows the clip: a split gives both halves the same level, copy/paste and reordering keep it. Merging two clips needs equal levels; otherwise Merge is greyed with "The clips have different volumes".
- A clip that is not at 100% or is muted shows a marker at its start; clicking it opens that clip's volume popup. The level and mute are also in Clip details.
- **In the export command:** a clip with a level other than 100% gets a `volume` filter in its own audio chain, before its fades. Clips at 100% get nothing, so projects that do not use clip volume produce exactly the same `ffmpeg` command as 1.1 (checked by a test against a saved 1.1 command).

## Clean audio (1.4.0)
- Clean audio is the fourth effect (REQ-114): it reduces background noise (wind, music, people talking) in the video's own sound and keeps the speech. It has no strength setting: a clip has it on or off. Applied to one clip, or to the whole video (every video clip), like Volume.
- **Engine:** the FFmpeg filter `afftdn` (spectral noise reduction, part of the core FFmpeg filters, so it is in every bundled build; no model file). Its three values are constants in one function of the audio code, tuned once against the acceptance gate. Alternatives rejected: `anlmdn` (much heavier on the processor), `arnndn` (needs a downloaded model file), `agate` and `highpass` (do not remove noise under speech).
- **Where:** in the clip's own audio chain, before the clip volume and the fades. The music track is not changed. Preview and export use the same chain.
- **Popup:** explains the effect and offers Remove. A clip with it shows a marker at its start (as the clip volume does).
- **Edit rules:** split keeps it on both halves; copy, paste and moving keep it; Merge needs the same setting on both clips ("The clips have different clean audio settings"). Applying and removing are one undo step each.
- **Export:** any clip with Clean audio forces a re-encode (REQ-023). Without it the export command is unchanged.
- **Acceptance (measured on a committed test clip of speech with added noise):** noise-only parts at least 6 dB quieter; speech parts within 3 dB of the original (assumptions).

## Whole video per clip (1.6.1)
- A double-click on an effect (REQ-082) applies it to every video clip on the timeline at that moment, with the same result as applying it to that clip (REQ-147). The result belongs to the clip: deleting the clip removes it, a clip added later has none, and a clip that already has the effect gets no second copy. One double-click is one undo step.
- **Storage (format 9):** each clip holds its own effects. The start and end fades are two flags of the clip (`fade_in`, `fade_out`) that replace the video-level flags of 1.2.0, so they move and are removed with the clip. Boundary fades, Scale and Move keyframes, the LUT and the volume were already per clip. The video-level Scale, Move and LUT fields are removed from the format. Change aspect stays one setting of the video.
- **Per effect:** Fade IN/OUT: fade in at the first clip's start, fade out at the last clip's end (length rule unchanged). Subtitles: created per clip as for one clip (REQ-085). Volume: each clip's level is set to the chosen value. Clean audio: on for each clip. Scale and Move: a copy of the keyframes in each clip (see [frame](architecture-frame.md#whole-video-scale-and-move-161)). Apply LUT: the same file on each clip.
- **Preview and export** are unchanged in code: they read the clip state through `fades_of`, the clip's frame stack and its audio chain.
- **Conversion of format 8** is described in [project files](architecture-projects.md#loading-and-validation).
- **Tests:** fades on clips 1 to 3 (start of clip 1, end of clip 3 only); deleting clip 2 removes only its effects; a clip appended later has none; Volume 50% sets three clips and one undo restores them; the conversion of a 1.6.0 example keeps each clip's sound and picture.

## Effects added in 2.0.4
Built in stages 11 and 13 (REQ-274 to REQ-294 and REQ-281, REQ-286 to REQ-288; see the [2.0.4 overview](architecture-2.0.4-overview.md)). Each value is a clip setting, not an effect row.
- **Fades (REQ-274, REQ-275, ADR-190):** the length of each fade is chosen from 0.1 s up to half of the shorter clip (0 means the default 0.5 s); a boundary can have a fade in, out, both, or none. A 1.5 s fade lasts 1.5 s within one frame (gate).
- **Music clips (REQ-276, REQ-277):** each music clip has its own volume and mute, and Clean audio.
- **Clean audio strength (REQ-278):** a dry and wet mix from 0 % to 100 % (0 bypasses the filter). **Enhance speech (REQ-279):** ffmpeg `speechnorm` named "Normalise speech" (`NORMALISE_SPEECH`); no model file.
- **Volume keyframes (REQ-280, REQ-282):** a clip's volume can have keyframes on the Scale and Move timeline (`Clip.volume_keys`, `Key<u8>`, percent, at most 200), each segment Straight or Ease; the gain is (percent / 100)², through the expression generator. 0 % at 0 s, 100 % at 4 s and 50 % at 2 s gives gain 0.25 (gate). Preview and export agree.
- **Speed (REQ-281):** a clip's speed is 50 % to 200 % (`Clip.speed` in thousandths, 1000 = normal). The clip's timeline length is its source length divided by the speed (`Clip::len`; `src_len` is the source part); keys and subtitle times stay in source time and are mapped. The picture uses `setpts`, the sound one `atempo`. A clip with a speed is never stream copied, and clips of different speeds do not merge (`DIFFERENT_SPEEDS`). Set in Clip details (`Command::SetSpeed`).
- **Crop (REQ-283):** a video clip's crop is applied in even pixels before the fit to the frame.
- **Blur levels and custom ratios (REQ-284, REQ-285):** Change aspect has Low, Medium (default) and High blur, and a custom aspect from 1:4 to 4:1 (ADR-193).
- **Change aspect of one clip (REQ-286):** a clip's own aspect (`timeline::clip_aspects`, by clip id) is composed in its own frame and then padded into the project frame; that clip is re-encoded.
- **Drop on a clip (REQ-288):** the Change aspect row can be dragged onto a video clip and then applies to that clip (the drop handling is in `ui/timeline.rs`).
- The picture overlays of text items and the 360 camera are described in [text](architecture-text.md#picture-items-204) and [360 video](architecture-360.md#anchored-lines-and-camera-paths-204).

## When the export must re-encode
In addition to the earlier rules (see [media pipeline](architecture-media-pipeline.md#export)), a start or end fade, any clip whose level is not 100% or which is muted, and (since 1.4.0) any clip with Clean audio, rules out stream copy. The Export dialog's explicit formats (see [export dialog](architecture-export-dialog.md)) also force a re-encode.

## Tests
- Timeline rules: fades on clip 2 of 3 (two boundaries, no start or end fade), on clip 1 (start fade plus one boundary), whole video (start and end; still at the edges after cutting the first clip), fade length on short clips, removal; clip volume through split, copy, reorder and merge.
- Export command: `volume=0.250000` for a clip at 50%, equivalence of clip and video levels, muted clip silent, re-encode forced.
- With real `ffmpeg`: first and last frame black with a whole-video fade; a muted clip is silent in the export while the others are not.
- Interface: dropping an effect on clip 2 gives the same result as the right-click menu; dropping on the music track changes nothing; one undo restores.
- (1.4.0) Clean audio: flag through split, copy, paste and merge; the filter sits before the clip volume in the command; no filter and identical command without it; a clip with it is re-encoded; real `ffmpeg` gate on the test clip (noise −6 dB, speech ±3 dB).
