# Code — Change aspect, Scale, Move and keyframes (1.5.0)

What 1.5.0 added, as built. The requirements are in [Frame](business-frame.md) (REQ-124 to
REQ-132); the design is in [architecture-frame.md](architecture-frame.md).

## Where things are

| Module | What |
|---|---|
| `keyframes.rs` | Pure. `Key<T>`, `Lerp`, `at` (hold at both ends, straight line between), `add`, `set`, `remove` (the last key stays), `sanitise` (damaged keys), `MAX_KEYS` = 200 |
| `frame.rs` | Pure. `FrameAspect` (`Shape`, `Fit`, `Fill`, `Rgb`), `frame_size`, `preview_size`, `picture_size`, `camera_size`, `fit_filter`; `Scale`, `Stack`/`Track` (the keys that act on one clip), `Placement`; the ffmpeg expressions (`scale_factor`, `offset_x`, `offset_y`) |
| `timeline.rs` | `frame_aspect`, `whole_scale`, `whole_move`, `clip_frames` (`ClipFrames`), the edit methods (`add_scale`, `set_scale_key`, `remove_move_key`, ...), `frame_stack(index)`, `has_frame_effect`; split, remove and merge keep the keys consistent |
| `project.rs` | Schema 7, `validate_frames` (damaged values) |
| `render.rs` | `Segment.frame` (`SegFrame`), `compose_frame` (background, picture, overlay), frame size in `plan_export_choice` and `output_geometry`, `FrameKey.placement` |
| `app/frame.rs` | Commands of the effects, the keyframe popups' state, `picture_rect`, the value being dragged (`frame_drag`) |
| `ui/frame_popups.rs` | The Change aspect popup and the Scale/Move popup; the pure `KeyBar` lays out the keyframe timeline |
| `ui/frame_handles.rs` | Pure geometry of the corner handles and the target (`hit`, `scale_to`, `move_to`) |
| `ui/picture.rs` | Draws the handles and runs the drags (after subtitle and text boxes, before the 360 camera drag) |

## Storage and time (REQ-132)

Change aspect is one value of the whole video. Scale and Move keys of the whole video are in
timeline time and edits do not move them. Keys of a clip are in the **source** time of its
media (as the 360 points of view): a trim or split never deletes or shifts them; a key outside
the kept part is hidden. A split gives both halves a copy of the keys; a merge needs equal
keys on both ("The clips have different frame settings"); a removed clip drops its entry; copy,
cut and paste keep them (`AppState::clipboard_frames`). `Clip` stays `Copy`: the keys live in
`Timeline`, so undo (the `Edit` snapshot) covers them.

Project format 7 is always written; 1.4.x refuses it with its newer-version message. Format 6
files load unchanged. `validate_frames` corrects damaged values and never refuses the file:
scale to 10%-400%, move to the frame, keys sorted, equal times keep the first, keys with a time
below 0 or beyond the media (whole video: beyond its length) dropped, at most 200 keys per effect,
entries of unknown clips dropped. An unknown shape, fit or fill reads as "no effect".

## The picture (REQ-125, 126, 131)

`frame_size(base, aspect)`: Fit on width keeps the base width and takes the height from the
shape, Fit on height the other way round; both sides even; the longer side is at most 4096 (both
sides scaled, shape kept). The frame is the output size: `plan_export_choice` and
`output_geometry` replace the encoder settings' width and height with it, so the hardware export,
the output check and the preview use it without any other size logic.

The picture is not computed from the base size but from the frame: Fit on width scales the
video to the frame's width (`scale=W:-2`), Fit on height to its height (`scale=-2:H`), no aspect
to fit inside the frame (`force_original_aspect_ratio=decrease`), with no bars. For 1600x1200 this gives the numbers of
REQ-125 (frame 1600x2844 with the picture 1600x1200; frame 676x1200 with the picture 1600 wide,
cut at both sides). A 360 clip's camera view is rendered at `camera_size` (the frame, or a 16:9
picture on the kept side).

A segment has `frame: Some(SegFrame)` when Change aspect is set or any Scale/Move acts on it;
without it the graph is byte-identical to 1.4.1 (golden tests). `compose_frame` builds, per
segment: the picture chain (`[pa]`), the background (`color` source, or the picture split and
cut to a small blurred copy: 1/8 size, `boxblur` radius 3, scaled back), the optional
`scale=...:eval=frame` with the combined factor, and `overlay=x=...:y=...:eval=frame:shortest=1`;
fades and the concat come after, then subtitles and texts on the whole frame. Expressions are
built from numbers only: nested `if(lt(t,T),a+(b-a)*max(0,t-t0)/span,...)`, time `t` of the
segment (it includes the preview start offset), key times shifted by the clip's source start or
the clip's timeline start; whole-video and clip terms multiply (scale) and add (centre offset).
A Scale key also carries an invisible centre offset (`dx`, `dy`, 1/10000 of the frame) set by the
corner drags so that the opposite corner stays fixed. A 360 video export (`ExportMode::Sphere360`)
drops the frame of every segment and keeps the base size. Any frame effect forbids the stream copy.

## Interface (REQ-124, 127..130)

- Effects list and menus: `EffectKind` has `ChangeAspect`, `Scale`, `Move` after Clean audio
  (7 rows with Add Text apart). A click opens the whole-video popup (`Command::OpenFrameEffect`),
  a double-click applies, a drop applies to the clip (not Change aspect: it is ignored, also in the
  clip's Effects submenu). Actions: `ChangeAspect`, `Scale`, `Move`, `ApplyClipScale/Move`,
  `ApplyWholeChangeAspect/Scale/Move`.
- Popups use `Popup::Instance` with `Instance::Aspect`, `Scale(FrameTarget)`, `Move(FrameTarget)`:
  one at a time. The keyframe timeline is as wide as the popup (at least 240 px); a click selects a
  key (6 px) or moves the playhead; **Add keyframe** adds one at the playhead with the value shown
  there, **Remove keyframe** needs a selection, the last key cannot go. A change of the value (field
  or handle) sets the selected key, otherwise makes one at the playhead (`Command::SetPicture`).
- Markers: a clip's Scale and Move have a marker 24 and 36 px after its start.
- Preview: the handles belong to the clip's effect, else to the whole video's; priority: subtitle
  and text boxes, then Scale corners, then the Move target, then the camera drag. A drag is shown
  through `shown_project()` (`frame_drag`) and is one undo step on release.

## Tests

Unit tests are in each module. Real ffmpeg (`tests/ffmpeg_integration/v15.rs`, flat colours): the
frame size and exact fill colour, blurred fill (no sharp edge, sharp picture), Scale 50%, Move 10%,
keyframes in a straight line, composition under a fade. `render::tests` pin the graph text and
the 360 and stream copy rules; `app/frame_tests.rs` the commands and undo steps;
`ui/frame_ux_tests.rs` the rows, popups, handles and their priority.

## Known limits

- The blur has one fixed strength; the preview of a blurred fill is rendered as a still on every
  change of the picture state.
- On a 360 video export the effects are ignored.
- Whole-video keys do not follow trims; text items and subtitles do not follow Scale or Move.
- The picture handles show only one effect per kind (the clip's, else the whole video's).
- Keyframes are only straight lines; they cannot be copied from clip to clip.
