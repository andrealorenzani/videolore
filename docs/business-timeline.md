# Business — Timeline precision (3.2.0)

Back to the [index](business.md).

Three aids on the editor's timeline, built in 3.2.0: a ruler with finer marks as the user zooms, a line that shows where the pointer acts, and a strip of picture snapshots inside each clip. They help to edit precisely and to see what each clip contains without playing it.

## Features (3.2.0)
- **Ruler marks**: small marks on the top ruler, with a step that follows the zoom: per frame, per second, per minute, per 10 minutes or per hour.
- **Pointer line**: a thin vertical line follows the pointer over the timeline and shows where a click or an edit acts.
- **Picture strip**: in each clip, from the middle of its height to the bottom, snapshots of the clip from its start, recalculated when the zoom changes. A setting turns it off.

## Requirements

### Ruler marks
- REQ-367 (implemented): Amends the REQ-093 tick steps. The timeline ruler shows small marks whose step follows the zoom: one mark per frame at the highest zoom, then per second, per minute, per 10 minutes and per hour as the zoom goes out. The labels of REQ-093 stay. Marks and labels never overlap. Acceptance: at three zoom levels the marks have the stated step (frame, second, minute); at the minimum and maximum zoom no two marks or labels overlap.

### Pointer line
- REQ-368 (implemented): When the pointer is over the timeline, a thin vertical line follows it through the ruler and the clips, showing the time where a click or an edit acts. It disappears when the pointer leaves the timeline. Acceptance: the line sits at the time a click at the same point would set (within one frame); the line is not shown when the pointer is outside the timeline.

### Picture strip
- REQ-369 (implemented): In each clip, from the middle of the clip's height to its bottom, snapshots of the clip's picture are shown from the clip's start, spaced to fit the zoom. A snapshot that does not fit leaves its space blank; a short clip shows only the snapshots that fit. The snapshots are recalculated when the zoom changes. They show the source picture at its own time (effects are not drawn). They are made in the background and never freeze the window (as REQ-032). Acceptance: a 10-second clip at a zoom where five snapshots fit shows five snapshots, the first at the clip's start; zooming out shows fewer; a clip shorter than one snapshot space shows only its first snapshot or none, with the rest blank.
- REQ-370 (implemented): Settings has the switch "Show picture strips on the timeline", on by default (MINOR). With it off, no snapshot is made and the lower half of the clips is blank. It is a program setting, not a project setting (as REQ-252). Acceptance: switch off: no snapshot on the timeline; switch on again: the snapshots return; a restart keeps the choice.

## Key decisions (3.2.0)
- The picture strip shows source frames only; effects are not drawn, so the strip is cheap and never mistaken for the export (MINOR).
- Ruler marks and labels follow one rule of steps, so the ruler never shows two different scales at once (MINOR).

## Out of scope
- Effects in the picture strips; snapshots of the sound or of the subtitles; timecodes on the clips.
- Zoom presets or a timecode field (not requested).

## Known limitations
- The ruler, the pointer line and the strips are not yet checked by hand on a real display or a slow computer (manual checks 42, 43 and 50 in [verification](business-verification.md), pending). Strips cost CPU and disk on a weak computer; REQ-370 turns them off.
