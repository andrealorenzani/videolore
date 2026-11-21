# Architecture: 2.1.0 overview (planned)

Part of [Architecture](architecture.md). Requirements REQ-300 to REQ-311 ([LUT](business-lut.md), [applied effects](business-applied-effects.md), [subtitle styles](business-subtitle-styles.md)). Status: planned; the decisions of the product owner are recorded below.

## Overview
Version 2.1.0 adds four things: LUT files up to 256 points per side (a 144-point `.cube` is about 80 MB, a 256-point one about 450 MB), a per-clip order of the colour and text effects shown in a new Applied effects widget, shadows for texts and subtitles, and three subtitle styles (highlight colours, Enlarge word, Typewriter). The project format becomes 12.

## Components
| Part | Where | Role |
|---|---|---|
| LUT reader | `src/lut/parse.rs` (new, pure) | Streams a `.cube`, `.3dl` or `.csp` file once; checks every line; builds the table (u16 per channel). |
| LUT store | `src/lut/store.rs` (new) | Cache by bytes, Loading (with progress), Ready and Failed states, cancel, the Background job, the Work folder copy. |
| Effect order | `src/effect_order.rs` (new, pure) | Movable rows per clip, sphere-fixed rows, legacy order, moves, repair on load. |
| Word styles | `src/word_styles.rs` (new, pure) | Word plans for Highlight, Enlarge word and Typewriter. |
| Overlay stage | `src/render/overlay.rs` (new) | Per-clip graph for clips whose order changed. |
| Widget | `src/ui/applied_effects.rs` (new) | The Applied effects list in the Preview section. |
| Changed | `src/ass.rs`, `src/text/events.rs`, `src/subtitles.rs`, `src/project.rs`, `src/app/lut.rs` | Shadows, colours, styles, format 12, non-blocking LUT choice. |

Module rules (cycle test, `tests/module_cycles.rs`): the parser imports only the standard library and `thiserror`; the store uses `base`, the job module and the parser; the pure modules import no render, app or interface code. Details: [structure](architecture-structure.md).

## Main technical decisions
- **Why the 80 MB LUT was refused:** the reader accepted 3D sizes up to 65 and read at most 16 MiB; ffmpeg accepts 256, so the limit was ours. Decision: sizes 2 to 256 and files up to 512 MiB, read in one streamed pass.
- **LUT memory and time:** a 256-point table takes about 101 MB (6 bytes per point, 16-bit channels; the error is far below one 8-bit level). The cache keeps at most 192 MB of tables. Loading runs on a background job with a progress percentage in the status line, and can be cancelled by choosing another file or closing the project; the window does not wait.
- **Large LUTs in projects:** above 20 MB the project keeps only the path (REQ-301); the missing-file choices of REQ-292 apply.
- **One Work copy per content:** the normalised cube is written once as `lut-<hash>.cube` in the private Work folder (about 450 MB at 256 points) and shared by all clips, the preview and the export. The user's path never reaches the ffmpeg filter string.
- **Applied effects order:** the widget lists the movable rows of a clip (Apply LUT, texts, picture items, subtitle lines) top first; the top row is drawn last. The LUT is applied after the frame composition and the fades, so it grades the whole frame. Sphere-fixed texts and lines on 360 clips are fixed rows. Projects saved before 2.1.0 keep the stacking they have today. Details: [applied effects](architecture-applied-effects.md).
- **Shadows and styles:** see [subtitle styles](architecture-subtitle-styles.md). The ASS export writes the shadow as the style's shadow and colour as its back colour, as REQ-307 says.
- **Format 12:** one bump for this development; every new field has a default that gives the 2.0.4 look; 2.0.4 refuses files saved in 12.

## Non-functional targets (planned; measured in the build)
- Parse of a 144-point LUT at most 2 s and of a 256-point LUT at most 6 s on the development machine (estimates).
- Work copy of a 256-point LUT written in at most 3 s (estimate); progress is shown.
- Window responsiveness: frame at most 200 ms while a LUT loads (gate).
- Memory: LUT tables at most 192 MB in the cache; loading adds the table plus 2 MiB.
- Preview with a LUT within 20% of the same clip without one; export at most 1.5 times the re-encode without one (estimates, measured). The start-up of each preview process with a 256-point LUT is measured first (risk).
- Projects without a changed order: graph and output unchanged (gate: golden tests).
- Event caps of the subtitle document kept (2000 per document, 20000 per file).

## Security model
- A LUT file is untrusted input: opened read-only as a regular file, streamed with a 512 MiB cap, 256-byte lines, finite numbers clamped to 0..1, exactly the entries the header promises.
- The file is never written; its copy has a fixed, hashed name.
- Stored order ids, shadow values and colours are validated on load; damaged entries are dropped with a line in the repair notice.
- Subtitle and text lines keep the escaping rules; the style lines are generated from validated numbers.

## Dependencies
No new crate. The parser uses the standard library; a faster parser crate is allowed only by a later decision if the time gate fails (see [dependencies](architecture-dependencies.md)).

## Stages (implementation order)
Spikes first (ffmpeg LUT start time and memory with 144- and 256-point cubes, libass placement of an enlarged word, the `\k` reveal, the box colour field of libass), then format 12 and the order model, the LUT reader, the LUT store, the Work copy, the widget, the overlay stage, the shadows, the subtitle styles, and the gates and documents. Each stage has its own tests; the requirement-to-test map is in [verification](architecture-verification.md).

## Decisions and open points
- Largest LUT edge: 256 (decided by the product owner).
- LUT over the frame background and bars: graded (decided).
- Sphere-fixed texts and lines on 360 clips: fixed rows, not movable (decided).
- Boxed shadows: the style rule of REQ-307 is kept; the libass box colour field is checked by a spike, with a vector-box fallback recorded in the architecture notes.
