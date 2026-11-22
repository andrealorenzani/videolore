# Business — Whole-video and keyframe clarity (2.0.4)

Back to the [index](business.md). Batch 7 of the [roadmap tasks](roadmap_tasks.md), built in the 2.0.4 development. Decided by Andrea on 2026-11-20: the whole-video keyframe copies are shown apart, one per clip, and are not edited all together. In 2.2.0 (planned, Andrea 2026-11-21) the whole-video popup itself is removed: each copy is a clip effect, so REQ-200 to REQ-202 are amended below; REQ-203 to REQ-205 stand.
## Requirements
- REQ-200 (implemented): There is no whole-video popup any more (REQ-148 amended): a whole-video Scale or Move is one copy on each clip, and each copy is edited in its own clip's popup, with its own Remove effect. The 2.0.4 rows of one per clip are therefore removed. Acceptance: double-clicking Scale on three clips gives three copies, each opened in its clip's popup; changing clip 2's keyframe leaves clips 1 and 3 unchanged; one undo restores the removed copy.
- REQ-201 (implemented): The notice "The new clip has no <effect>" and its button **Apply to new clips** are removed: a whole-video effect is a copy on each clip (REQ-147), and a clip added later gets none, with no notice. Acceptance: a whole-video Volume of 50% and an appended clip: no notice appears and the new clip has no Volume; the other clips are unchanged.
- REQ-202 (implemented): The picture shows only the handles of the clip's own Scale and Move (solid). The dashed handles of a whole-video copy are removed, because a whole-video Scale or Move is a copy on each clip (REQ-148). Acceptance: a clip with Scale shows one box; dragging its corner changes that clip's Scale only; the other clips are unchanged.
- REQ-203 (implemented): In the Scale and Move keyframe timeline, a keyframe marker is at least 12 pixels across, and a click within 8 pixels of a keyframe selects it. The selected keyframe's time and value are shown in fields that accept typed values. When the popup has keyboard focus, Left and Right move the selected keyframe by one frame. Acceptance: a click 6 pixels from a keyframe selects it; a typed value changes it; with focus in the popup Right moves the keyframe by one frame; the usability check in real use is in [verification](business-verification.md).
- REQ-204 (implemented): A clip's Scale or Move popup has **Copy keyframes** and a **Paste keyframes to** list with the other clips. Pasting replaces the target clip's keyframes of that effect with the copied ones, in clip time, and does not change its other effects. One undo step per paste. Acceptance: copy the Scale keyframes of clip 1 and paste them to clip 3: clip 3 has the same keyframes at the same clip times; one undo restores clip 3.
- REQ-205 (implemented): A double-click that gives a clip the Apply LUT file it already has changes nothing: the status line reads "The clip already uses <file name>" and no undo step is added. A different file replaces the clip's LUT as before (REQ-135). Acceptance: double-click with the same file leaves the clip and the undo history unchanged and shows the message; with another file the LUT changes.

## Key decisions (2.0.4)
- Decided by Andrea on 2026-11-20 (2.0.2 record): the copies are shown apart; the whole-video popup no longer edits all of them at once (REQ-200).
- [MINOR] The notice of REQ-201 is an offer, never an automatic change, so REQ-147 keeps its meaning.
- [MINOR] A same-file double-click keeps the clip's current LUT; a LUT strength (REQ-289) would be kept with it.

## Impact on existing features
- REQ-148 changes as in REQ-200; its tests change with it.
- REQ-202 changes the handles drawn on the picture of a clip with its own and a whole-video effect.
