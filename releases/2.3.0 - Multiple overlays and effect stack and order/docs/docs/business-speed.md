# Business — Speed (2.2.0)

Back to the [index](business.md). Built in 2.2.0 (theme: speed), except REQ-315 and REQ-177, which are measured and not met. It covers the large-LUT preview, the Applied effects column, the blurred fill in playback, Faster export on integrated graphics, the reused-subtitle timing (see [Subtitle accuracy](business-subtitle-accuracy.md)) and one picture limit (REQ-316). Whole-video changes of 2.2.0 are in [Effects](business-effects.md), [Editing](business-editing.md), [Frame](business-frame.md), [Whole-video and keyframe clarity](business-keyframes.md) and [Applied effects](business-applied-effects.md).

Numbers marked "assumption" are targets to confirm with the first measurement; the measured values are recorded in the developer documentation.

## Features
- **Preparing a large LUT in the background**: a 256-point LUT still takes about 12 s to prepare; the preview shows a "Preparing LUT" state with progress and the interface stays usable.
- **Quicker Applied effects column**: its rows are not rebuilt on every frame.
- **Blurred fill in playback**: measured first, then made cheaper if it costs too much.
- **Faster export on integrated graphics**: Faster is really quicker than Best quality there (target not met in 2.2.0).
- **Picture items limited to 16 megapixels**: a larger picture is refused with a message.

## Requirements

### Large LUTs in the preview
- REQ-312 (implemented): Decision D63. Preparing a large LUT for the preview shows its progress and never blocks the interface. The `.cube` route is kept: no colour change and no change of the processing graph. A 256-point LUT takes about 12 s measured on the development machine (144 points 2.3 s, measured in 2.1.0); while it runs the status line shows "Preparing LUT" with a percentage that moves at least once per second. Meanwhile the preview shows the picture without the new LUT (assumption); when the LUT is ready the preview shows it at once (REQ-300). Acceptance: (a) during the preparation a click on a menu item or a popup responds within 200 ms (test), and playback and export started meanwhile are not delayed (REQ-224); (b) the status line shows "Preparing LUT" with a percentage that increases during the preparation (test); (c) the preview and the export differ by at most 2 colour levels per channel on a test frame, as REQ-300; (d) a newer LUT choice replaces the preparation in progress (REQ-300). The 12 s is measured and reported, not gated. Built: the preview shows a LUT only after its copies are written.

### Applied effects column
- REQ-313 (implemented): The rows of the Applied effects column (REQ-302) are kept and rebuilt only when the selected clip, its order (REQ-304) or its effects change; a change of the clip under the playhead counts only when no clip is selected. Acceptance: a test counts the row builds: 0 over 60 frames with no change, 1 after one change (add, remove, move, or another clip selected); the rows, their order, icons and clicks are the same as before (REQ-302 acceptance).

### Blurred fill in playback
- REQ-314 (implemented): The blurred background of Fit empty with blurred video (REQ-126) does not slow playback. Facts: the blur is computed at one eighth of the frame (REQ-222); the cache of REQ-223 does not help during playback, because the video under the blur changes on every frame. Measurement first: the cost of the blur step per frame is recorded in the developer documentation at 1080p and at 4K, for a 9:16 Change aspect with Medium blur on the reference clip (median and 95th percentile of 300 frames). Acceptance: (a) at 1080p, playback with the blur keeps the frame rate of the clip with at most 1% dropped frames in 60 s (assumption); (b) at 4K, the median blur step is at most one frame time of the clip (40 ms at 25 fps; assumption); if above it, the number is reported and the item stays open; (c) the picture stays as REQ-126 and REQ-223 say, and the preview equals the export within 2% of the frame height. Scale changes in the preview are a roadmap item. Measured in 2.2.0: the blur step costs about 2 ms a frame at 1080p and at 4K (1080p 2.4 to 2.8 ms plain, 4.1 to 4.8 ms with the blur; 4K 8.3 to 8.9 ms plain, 10.7 to 11.2 ms with the blur), against the 40 ms budget of (b): met.

### Faster export on integrated graphics
- REQ-315 (planned; measured, target not met): On integrated graphics Faster export is quicker than Best quality. Facts: on an Intel Iris Plus G7 (Linux, VA-API, H.264) the 2.0.4 measurement of the hardware export plus its output check ranged from 14% quicker to 41% slower than Best quality; the check renders the normal method around its three sample frames (REQ-206). Acceptance, on that machine with the QG-05 reference project at 1080p in H.264: (a) the median of 3 runs of Faster, total time including the output check, is at least 20% lower than the median of 3 runs of Best quality (assumption: the lower end of the 20-45% estimate of QG-05 g; the 40% of QG-05 g stays a measured goal, not a gate); (b) the output of Faster has the same format and frame count as Best quality, and every fault of REQ-063 is still caught and redone (QG-05 b and h); (c) on any other computer no gate applies: the time is recorded per kind of graphics (REQ-207), and the explanation of REQ-207 stays when the result is not quicker. The check must stay (no removal of a fault test). **Measured in 2.2.0** (Intel Iris Plus G7, system ffmpeg 6.1.1, VA-API, H.264, one run each): Best quality 12.3 s, Faster 12.2 s before; 11.3 s with the concurrent strict decode and the three sample frames (8.7% lower). Target (a), 20% lower (9.8 s), **not met**. The rest of the time is the hardware encode phase. Levers not taken: a 0.1 s change and hardware decode, which weakens the check.

### Picture items
- REQ-316 (implemented): Amends REQ-287. Adding a picture refuses a PNG with more than 16 megapixels (width times height above 16,000,000 pixels) with the message "The picture is larger than 16 megapixels (<width> x <height>) and was not added."; nothing is added and the status line shows the message. This is the only limit: no count of items, no file size limit and no copy of the picture in the project (the project keeps the path). Acceptance: a 4000 x 4000 PNG (16.0 megapixels) is added; a 4001 x 4000 PNG is refused with the message naming the file's size; a project whose picture file is replaced by a larger one is treated as a missing picture (REQ-287: the export stops with its message, the project still opens) (assumption).

### Reused subtitles (REQ-177, amended in 2.2.0)
The target of REQ-177 is kept, and it is built with a speech-region method. The text, its acceptance and its status are in [Subtitle accuracy](business-subtitle-accuracy.md): the parameters are fixed once and checked on more than one clip, and the strict test `v12::alignment_matches_real_speech` is enabled.

## Key decisions
- (D63, Andrea, 2.2.0) The large-LUT preview keeps the `.cube` route with progress; no colour or graph change. Rejected: an integer `.m3d` table (3.4 s restart: ffmpeg 9.0.2 reads `.m3d` as text); a float `.m3d` table (5.1 s); a binary HALD table (0.36 s, but trilinear interpolation and a new 16-bit path, so the colours change). A smaller preview table breaks exact colours. Binary input and smaller preview table are roadmap items marked not planned.
- (Andrea) REQ-177 is built with the speech-region method and the 90% target kept; the second clip (about 30 s with transcript) is needed before the build.
- (Andrea) Picture items get one limit only: 16 megapixels. No count limit, no file size limit, no copy in the project.
- (MINOR) The speed numbers marked "assumption" are targets to confirm with the first measurement.

## Scope
- In: the requirements above, the whole-video amendments listed in the index, and the REQ-177 method.
- Out of scope: Scale changes in the preview; the five-minute speech test and the gated CI alignment test (roadmap); the 20-item and 8 MiB picture limits and an embedded copy of pictures (rejected); binary LUT input and smaller preview tables (not planned); Enlarge word and Typewriter in subtitle files (roadmap); the Apply LUT cost on weak computers (roadmap).

## Known limitations
- The large-LUT preparation stays about 12 s for 256 points; the preview shows the unprocessed picture meanwhile.
- The exact-colour rule of REQ-312 is checked on test frames, not on every clip.
- REQ-314 and REQ-315 depend on the measurements of the development machine and of the Iris Plus G7; other computers are recorded, not gated.
- REQ-316 does not limit the total memory of several large pictures.
