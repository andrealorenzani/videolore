# Architecture: speed (2.2.0, planned)

Part of [2.2.0 overview](architecture-2.2.0-overview.md). Requirements REQ-312 to REQ-316 ([speed](business-speed.md)). Status: built 2026-11-22. REQ-312 route D, REQ-313, REQ-314 (met) and REQ-316 are built; REQ-315 is measured and NOT met (8.7% against 20%). The numbers below are measured spikes or assumptions, as marked.

## Large LUT in the preview (REQ-312, D63, ADR-214)
- **Route:** the 2.1.0 `.cube` route is kept: the same Work copy `lut-<hash16>.cube`, the same `lut3d` filter with tetrahedral interpolation, the same colours. Nothing in the picture changes.
- **Preparation in the background:** choosing a LUT starts one job in the existing LUT store: the file is read and parsed in a streamed way (2.1.0), the Work copy is written once with progress, and the store reports Ready. The interface thread never parses or writes.
- **Progress:** the status line shows "Preparing LUT <name>... N %", updated at least once a second. Reading and writing have measured percentages. During the ffmpeg start, which has no progress signal of its own, the percentage is a time-based estimate (about 12 s expected, capped at 95% until the first graded frame); it is marked as an estimate in the developer notes.
- **Unprocessed picture meanwhile:** while the LUT is not Ready, the preview graph is built without the LUT row. The picture shows the other effects, not the LUT. When the copy is Ready, the preview restarts with the LUT and "Preparing LUT" ends at the first graded frame.
- **Newer choice:** replaces the preparation; the old job stops and a partial copy is removed.
- **Export:** waits for Ready in its own worker, with the same percentage in the status line.
- **Start time:** about 12 s for 256 points (12.4 to 13.0 s measured), 2.26 s for 144 points. Not gated (D63).
- **Record of the alternatives (spikes, not used):** lut3d reads `.m3d` by extension, but as text in ffmpeg 9.0.2: integer values with `out 65536` start in 3.4 to 3.6 s (293 MB), float text in 5.1 s; a binary 16-bit HALD image read by `haldclut` starts in 0.36 s but interpolates trilinearly and needs a 16-bit path in the 8-bit graph (not verified). They were rejected for route D (colours and graph unchanged).

## Applied effects rows (REQ-313, ADR-216)
- The widget keeps its rows. A revision number of the clip set, the order and the effects changes with each edit (also undo and redo); the rows are rebuilt once after a change and read from the cache otherwise.
- Test: a counter of row builds (0 over 60 frames without a change; 1 after one change).

## Blurred fill in playback (REQ-314, ADR-217)
- The background is a copy at one eighth of the frame, blurred and scaled back into the frame (`boxblur`).
- Proxy measurement (no decoding, 300 frames, bundled ffmpeg 9.0.2): the blur adds about 3.2 ms per frame at 1080p and about 1.8 ms at 4K. The real preview graph is measured before any change.
- **Built:** measured on the built graph (testsrc2, 9:16, Medium blur, 300 frames, no decode): about 2 ms a frame at 1080p and 4K, one frame time is 40 ms: target met, no change.
- If the measurement misses the budget (one frame time at 4K, 40 ms at 25 fps; 1% dropped frames at 1080p), the first change keeps the picture: removing a full-size scale of the background. A change of the blur size is a product decision.

## Faster on integrated graphics (REQ-315, ADR-218)
- **Built (lever (a)):** the strict decode and the three sample renders run at the same time (within the process cap). Measured on the Iris Plus G7, median of three: Best quality 12.3 s, Faster 11.3 s (8.7% lower; target 20%, NOT MET). Levers (b) (0.1 s) and (c) (hardware decode of the check, a weaker check) are not taken. The rest of the time is the hardware encode phase.
- Faster is a hardware encode followed by the output check (`src/outcheck.rs`): a full strict decode of the file, then three one-second renders of the normal graph compared with psnr (REQ-206). The check stays in full.
- Levers, in order, once the phase times are measured on the Iris Plus G7: run the strict decode and the sample renders at the same time within the process cap; reuse information between the passes; a hardware decoder for the strict pass only with a note and a test. Fewer samples are not used.
- Measured in 2.0.4 (before 2.2.0): from 14% quicker to 41% slower than Best quality.

## Picture limit (REQ-316, ADR-219)
- One constant of 16,000,000 pixels; the PNG header gives width and height. Refused on Add picture with the message of the business text; a project whose picture grew beyond the limit is a missing picture (the export stops with its message, the project opens).

## Limits of 2.2.0
- The first graded LUT frame of a 256-point LUT comes after about 12 s; the picture is unprocessed until then.
- The percentage during the ffmpeg start is an estimate.
- Scale changes in the preview are out of scope (roadmap).
