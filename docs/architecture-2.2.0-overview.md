# Architecture: 2.2.0 overview (built 2026-11-22, not released)

Part of [Architecture](architecture.md). Theme: speed. Requirements REQ-312 to REQ-316 ([speed](business-speed.md)), REQ-177 amended ([subtitle accuracy](business-subtitle-accuracy.md)), and the whole-video amendments of REQ-083, 147, 148, 129, 131, 076, 200 to 202 and 303 ([effects](business-effects.md), [frame](business-frame.md), [keyframes](business-keyframes.md), [applied effects](business-applied-effects.md)). Status: built 2026-11-22, not released. Not met, measured: REQ-315 (Faster on integrated graphics, 8.7% lower against 20%) and REQ-177 (reused subtitles, 43% of matched words within 0.3 s against 90%); see the [developer record](code-2.2.0.md) for the figures and the deviation from REQ-321 (new rows keep their default place). The decisions are in [speed](architecture-speed.md) and [subtitle accuracy](architecture-subtitle-accuracy.md); the stage order is in the developer notes.

## Overview
Version 2.2.0 makes four things faster or clearer: a 256-point LUT is prepared in the background with a visible percentage, the interface never waits for it, and the preview shows the unprocessed picture until the LUT is ready (the LUT route itself does not change: the same colours, about 12 s to the first graded frame). The Applied effects rows are kept between frames; the blurred fill is measured in playback; Faster export is made quicker on integrated graphics; the picture limit becomes 16 megapixels. It also removes the whole-video popup and mark (a whole-video effect is already a copy on each clip; only the interface routes go), and it builds the speech-region timing of reused subtitles (REQ-177), if the second speech clip confirms it. The order of all effects of a clip (REQ-320 to REQ-322) and the per-text maximum size (REQ-323) need project format 13: 2.1.0 refuses it, format 12 still opens ([order and format](architecture-2.2.0-order.md)). The Applied effects list moves to a column beside the picture, and the text size follows the box with the cap as its maximum ([layout and text size](architecture-2.2.0-layout.md)).

## Components
| Part | Where | Role in 2.2.0 |
|---|---|---|
| LUT store | `src/lut/store.rs` (2.1.0) | the preparation job: read with progress, parse, Work copy `lut-<hash16>.cube` written once with progress; `peek` for the graph builder |
| LUT status | `src/app/lut.rs`, tick | "Preparing LUT ... N %", at least once per second; export waits for Ready in its worker |
| Render graph | `src/render/graph.rs`, `src/render/segments.rs` | the LUT row is left out while not Ready; no parse or write while building |
| Applied effects | `src/app/applied.rs`, `src/ui/applied_effects.rs` | rows kept until a change; no whole-video mark |
| Whole video | timeline, effects, popups, handles | popup, routes, offer and selection removed; copies kept |
| Picture | `src/picture.rs` | 16 megapixel limit on the header |
| Speech regions | `src/speech_regions.rs` (new, pure), `src/align.rs` | word times from speech regions |
| Blur, Faster | `src/render/graph.rs`, `src/outcheck.rs`, export flow | changed only as measured |

## Main technical decisions
- **LUT preview (REQ-312, Andrea's choice of 2026-11-21):** the `.cube` route stays. The LUT is prepared in the background, with a percentage that moves at least once a second. Until the LUT is ready, the preview shows the picture without the LUT row; a newer choice replaces the preparation. The start time (about 12 s for 256 points) is recorded, not gated. Alternatives measured and not chosen (`.m3d` text 3.4 to 5.1 s, binary 16-bit image 0.36 s with a different interpolation) are in [speed](architecture-speed.md) and ADR-214.
- **Order of all effects (REQ-320 to REQ-322):** each group (picture, sound) keeps its order per clip; Speed is fixed first. A clip with the default order is drawn as in 2.1.0; other orders are drawn step by step from the stored order, with pair rules for Crop, Scale, Move, Change aspect, LUT and Fade ([order](architecture-2.2.0-order.md)).
- **Text size (REQ-318, 319, 323):** the box governs the size, the cap of 10% is the default maximum; a text may set its own maximum.
- **Whole video (REQ-147, 148):** a whole-video effect is a copy on every clip; the popup and the whole-video mark are removed. The file format does not change (ADR-215, ADR-221).
- **Applied effects (REQ-313):** rows are rebuilt only when the selected clip, its order or its effects change (ADR-216).
- **Blur (REQ-314) and Faster (REQ-315):** measured first; the check of Faster is kept in full (ADR-217, ADR-218).
- **Picture limit (REQ-316):** one constant, checked on the PNG header (ADR-219).
- **Subtitle timing (REQ-177):** speech regions, one frozen setting checked on a second clip (ADR-220).

## Non-functional targets
- LUT: percentage moves at least once a second; menu and popup response within 200 ms while preparing; playback and export not delayed; preview and export colours within 2 levels (gates). Start about 12 s, not gated.
- Applied effects: 0 row builds over 60 frames without a change (gate).
- Blur: 1% dropped frames at 1080p in 60 s; one frame time for the 4K blur step (assumptions, measured first).
- Faster on the Iris Plus G7: at least 20% quicker than Best quality (assumption, measured).
- Picture: 16 megapixels exactly (gate).
- Reused subtitles: at least 90% of matched words within 0.3 s on the reference clip and the second clip (gate).
Details in the developer notes (NFR of 2.2.0).

## Security
- The LUT copy is written by the program from the validated table; its name is the content hash; the Work folder is private; a partial copy is removed on cancel or replacement.
- The picture limit reads the PNG header only (bounded reader).
- No new crate, no unsafe code, no network.

## Dependencies
No new crate. Bundled ffmpeg 9.0.2 (pinned, unchanged): `lut3d`, `boxblur`, `psnr`.

## Known limitations (built)
- The first graded LUT frame of a 256-point LUT comes after about 12 s; until then the picture is unprocessed.
- The 360 whole-sphere export ignores the order of the Applied effects rows (sphere-fixed rows, documented).
- Whole-video multi-clip removal is not built (roadmap candidate).
