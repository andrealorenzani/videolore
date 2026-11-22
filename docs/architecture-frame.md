# Architecture: frame (Change aspect, Scale, Move)

Part of [Architecture](architecture.md). Built in 1.5.0 (requirements REQ-124 to REQ-132, see [business: frame](business-frame.md)). The decisions below are as built; the deviations from the plan are listed in the [changelog](changelog.md) 1.5.0 entry and in [code-frame](code-frame.md).

## Overview
Three picture effects change what the picture looks like:
- **Change aspect** (whole video, one per project) sets the shape of the frame (4:3, 16:9 or 9:16), how the picture fits it (fit on width or on height), and what fills the empty space (a colour or a blurred copy of the video).
- **Scale** zooms the picture of one clip or of the whole video. **Move** shifts it. Both can be keyframed: their value changes in a straight line between keyframes.

The same `ffmpeg` graph builds the preview and the export, so they match. Projects without these effects produce exactly the same `ffmpeg` commands as before (checked by golden tests).

## Frame size (Change aspect)
- The base size is the size the video has without the effect: the format's size, or the video preset of the Export dialog (see [export dialog](architecture-export-dialog.md)).
- **Fit on width** keeps the base width and computes the height from the shape; **fit on height** keeps the base height and computes the width. Sides are even numbers.
- A side longer than 4096 px is scaled down with the shape kept. This limit is decided (see the last section) and its risks are in Risks.
- Example: a 1600x1200 base to 9:16 with fit on width gives a 1600x2844 frame; with fit on height, 675x1200.
- The picture is the source fitted into the base size, not the base frame with its own black bars. So the empty space is always the chosen background, and a colour fill shows exactly that colour.
- The picture is never stretched. Where it is longer than the frame, the extra part is cut equally from both sides; where it is shorter, the empty space is shown around it, centred.

## Picture composition
Per flat clip (and per camera view of a 360 clip), in this order, the same in preview and export:
1. The picture at base size (the usual fit, without the black padding).
2. The background at frame size: the colour, or a copy of the same video scaled to cover the frame and blurred. The blur is computed on a copy at one eighth of the frame size, which keeps its cost low. The strength is fixed.
3. The picture, scaled and moved by the clip's own Scale and Move. Values change per frame from the keyframes. (Until 1.6.0 the whole video's values were multiplied and added to these; since 1.6.1 there is no whole-video track, see below.)
4. Fade IN/OUT (also to black over the background) and then the rest of the export as before.
5. Subtitle lines and text items, at their places as fractions of the frame, drawn last on the whole video.

Any Change aspect, Scale or Move makes the export a re-encode, even at the default values. The hardware export and its output check apply.

## Scale and Move keyframes
- Each clip can have one Scale and one Move; the whole video can have one of each. A new one starts with one keyframe (100% or the centre).
- Values are stored as keyframes with a time. Clip keyframes are in the **source time** of the clip, as the 360 points of view are: trimming, splitting and cutting never delete them. Keyframes outside the part of the clip in use are hidden, not lost, and still shape the path; so the value at each new edge is the one the path had there. Whole-video keyframes are in **timeline time** and are not moved by edits (like the edge fades).
- Between two neighbouring keyframes the value changes in a straight line; before the first and after the last it stays the nearest value.
- Scale is stored in permille of the picture's base size (100 = 100%, between 100 and 4000). Move is stored as the centre of the picture in 1/10000 of the frame (5000, 5000 = centre).
- A corner drag keeps the opposite corner fixed. For that, a Scale keyframe also stores an offset of the picture centre, set by the drag and not shown. Move is never changed by Scale drags.
- At most 200 keyframes per effect (per clip, and for the whole video).
- Split: both halves keep the keyframes. Copy and paste keep them. Merge needs the same keyframes on both clips, as Volume and Clean audio do.

## Filter graph
- Scale: a per-frame size from the keyframes (`scale` with per-frame evaluation). Move: per-frame position for `overlay`. The expressions are piecewise linear, written from numbers only; no text from the project enters a filter graph. The colour is validated as `#RRGGBB` and written as `0xRRGGBB`.
- A long graph goes through a script file (as the existing exports do).
- If the bundled `ffmpeg` does not accept time-dependent sizes, the fallback is a command file with changed values, like the 360 camera (not needed: ffmpeg 6.1.1 accepted the time-dependent sizes in the spike; the bundled 9.x build is not checked yet).

## Export and 360 video
- Flat export (automatic or flat camera view of 360 clips): all three effects apply.
- 360 video export (the whole sphere, equirectangular): Change aspect, Scale and Move are ignored; the output keeps its usual size. A 360 video export is the same with and without these effects.

## Preview
- The preview draws a box with a square handle at each corner for Scale, and a round target at the centre for Move. The priority of pointer hits is: subtitle and text boxes, then the Scale corners, then the Move target, then the picture (camera drag on 360 clips).
- A drag is shown live; the still frame is requested after a short quiet period as today. One drag is one undo step.
- The keyframe popup is non-modal and keeps working during playback. Its timeline fills the popup width at any clip length.

## Project format
- Format 7 (1.5.0): Change aspect, and each Scale and Move with its keyframes, for the whole video and for each clip. Format 6 files open with no picture effects.
- 1.4.x refuses format 7 with its newer-version message and never modifies the file.
- Damaged values are corrected on load with a note: scale limited to 10%..400%, centre to the frame, duplicate times dropped, keyframes outside the media dropped, more than 200 keyframes cut, unknown shapes, fits or colours replaced by the defaults. Nothing crashes.

## Non-functional targets (1.5.0)
- Every interface frame within 200 ms while popups are open and corners are dragged during an export and playback (existing target).
- Still frame of a picture with effects at preview size: under 80 ms (estimate).
- Export of a 1080p source to a 9:16 frame: at most 1.5 times the time of the same re-encode without effects (assumption, to be measured).
- The blurred background adds at most 10% to the composition time (estimate).
- Memory: no new buffer; frames up to 4096 px on the longer side.

## 2.0.4 additions (stages 6 and 11)
- **Blur levels (REQ-284, ADR-193):** Low, Medium (default) and High for the blurred fill. **Blur divisor (ADR-180):** the blurred background is computed at an eighth of each side (`BLUR_DIVISOR` 8), measured on the bundled 9.0.2 build (4K: 2.9 to 3.1 s, against 3.3 to 3.7 s for a quarter); REQ-222 names a quarter, and the product decision is open. A project without blur is unchanged.
- **Custom aspect (REQ-285, ADR-193):** any ratio from 1:4 to 4:1 can be typed; the frame's longer side keeps the 4096 px cap.
- **Background of a still:** one raw RGBA file in the private folder, for stills only (see [media pipeline](architecture-media-pipeline.md#preview-and-export-additions-204)).
- **Clip Change aspect (REQ-286):** a clip's own frame, padded into the project frame (see [effects](architecture-effects.md#effects-added-in-204)).

## 2.0.3: blur fill on small pictures
- A blurred background is made from a picture of at least 8 pixels on each side (it was 2), and its blur radius for the colour planes is limited to what the picture can take. Before, a frame about 64 pixels wide or less with a blurred fill made the export fail with an ffmpeg error. Pictures of normal size are unchanged.
- Test: the blurred fill of a 68 by 120 frame exports; the radius rule holds for sizes from 8 to 600 pixels.

## Security
- All inputs are checked: keyframe numbers within their ranges, shapes and fits from closed lists, colours as `#RRGGBB`.
- Frame sizes come only from the base size and the chosen shape, never from numbers stored in a file; the 4096 px cap bounds memory and encoder load.
- No network access and no new temporary files beyond the existing filter script.

## Dependencies
No new crate. The `ffmpeg` filters used (`color`, `split`, `scale`, `boxblur`, `overlay`, `format`) are core filters in the bundled builds. The colour picker is part of `egui`, which is already used.

## Visible edges (1.6.1)
- The Scale box works through its visible edges (REQ-145): the part of each edge inside the frame is a handle, so a side cut by Change aspect or a Scale above 100% can still be dragged. Corners are hit first, then the visible edges, then the Move target.
- Dragging an edge scales the picture proportionally: the factor is the pointer's distance from the opposite edge over the box's side at the start, applied to both sides; the opposite side stays fixed. Limits 10% and 400%, one drag is one undo step. The corners are drawn on the visible part of the picture.
- Pure geometry in `ui/frame_handles` (`Hit` gets an edge variant; `scale_to` takes the axis of the edge). Subtitle and text boxes use the same visible-part rule (see [boxes](architecture-boxes.md#visible-edges-161-req-145-req-146)).

## Whole-video Scale and Move (1.6.1)
- Built 2.2.0 (REQ-148, REQ-200..202): the whole-video Scale and Move popup, its clip rows and the dashed handles of a whole-video copy are removed; each copy is edited in its own clip's popup and on the picture, with its own Remove effect.
- There is no whole-video Scale or Move in the format any more (REQ-148). A double-click gives each clip its own copy: its keyframes are in the source time of the clip, from the clip's start on, like the clip's own keys, so a trim or cut never moves a copy (its keys stay on the same part of the source). The whole-video popup shows the first clip's keyframes on a timeline as long as the longest clip, and each change in it is applied to every copy at the same clip time (add where the copy is long enough, set, or remove keeping each copy's last key).
- A change in one clip's popup or on the picture changes that clip only. A later whole-video change sets all copies again. The popup shows only the first clip's keys after one clip was changed alone (known limit).
- A clip has at most one Scale and one Move, so the stacking of the whole video and the clip is gone: the composition uses the clip's own values only.
- Conversion of 1.6.0 files (format 8) into copies is in [project files](architecture-projects.md). Keyframes beyond 200 per clip are refused by the cap. A 1.6.0 clip that had its own Scale or Move as well as the whole video's gets one combined path: the scales are multiplied and the moves added, at the key times of both paths (exact at those times, between them an approximation). A clip that had its own Apply LUT keeps it, and the whole-video LUT is not applied to that clip. The status line at opening says which clips changed (see [project files](architecture-projects.md#loading-and-validation)).

## Tests
- Pure: keyframe values (hold, linear, limits, unique times), frame sizes from the requirement examples (1600x1200 to 9:16 both ways), even sides and the 4096 cap, timeline rules for split, copy, merge and remove, project format 7 load and save, damaged values.
- Command: the expressions and the filter text for one and for several keyframes; no-effect commands unchanged; 360 video export without the effects; re-encode forced.
- With a real `ffmpeg`: frame size, colour in the empty space, blurred border without a sharp edge, Scale at 50% shows the background, Move of 10% moves the picture by 10% (within 1%), Scale 150% at 2 s between 100% at 0 s and 200% at 4 s, subtitles at the bottom stay at the bottom at Scale 50%.
- Interface: effect rows and menus, drop rules, popup opening, handle priority, one undo per action.

## Risks
- Time-dependent sizes (Scale and Move keyframes) were accepted by `ffmpeg` 6.1.1 in the spike; the bundled 9.x build is not checked yet (on the roadmap).
- Large frames (4K in a tall shape) make exports slower and may be refused by a hardware encoder; the software redo handles that.
- Keyframes in a small popup are harder to use; the timeline has a minimum width.
- The per-clip copies made from a whole-video Scale or Move (1.6.1) are kept through trims and cuts, as the clip's own keys are (source time): a key before a new start is hidden, not lost, and a key after a removed part keeps its source time.
- Text items keep their place when the picture moves (not keyframed), so a text may end up over the edge of the picture.

## Decided: size limit of the frame
- **Size limit of the frame**: the longer side is capped at 4096 px and the shape is kept. Decided by the product owner on 2026-11-19. The alternatives, a 1920 px cap (smaller files, less detail for 4K sources) and no cap (some players and encoders refuse frames above 4096 px), are not used.
