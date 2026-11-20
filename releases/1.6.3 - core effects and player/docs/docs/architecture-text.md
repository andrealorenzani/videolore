# Architecture: text items (1.3.0)

Part of [Architecture](architecture.md). Built in 1.3.0 (requirements REQ-108..111, see [Text](business-text.md)); the 360 anchoring is in [360 video](architecture-360.md#anchored-text-and-subtitles-130). Text items reuse the subtitle rendering path, so preview and export match as they do for subtitles.

## Model
- A text item is a new kind of object, stored in the project in its own list (not as subtitle lines). It has: multiline text (at most 1000 characters per line), start, time to live (0.1 s up to 24 hours on load; when added, edited or moved it is limited to the video length), its effects, colour (`#RRGGBB`), font (family name), a flat place (centre, as subtitle positions, 1/10000 of the picture) and, since 1.4.0, a **width** (10–100% of the picture width; the height is fixed at 20% of the picture height). Until 1.3.0 it had a size of 25–400%; since 1.4.0 the size follows from the box (see below).
- Text items sit on the timeline by time only: they do not ripple when the video is edited (like the music track), and a text running past the end of the video is cut there.
- Selection, moves and resizes of text items use the same box and selection code as subtitle lines (see [Selection](architecture-ui.md#selection-130)).

## Rendering
- Text items are written as ASS events into the same subtitle file format used for subtitles (`ass` module). Preview and export use the same file at the same resolution, so the look is identical.
- A text item makes the export a re-encode (REQ-023); the hardware export and its check apply as for subtitles.
- Fonts: the family names of the fonts installed on the computer. The subtitle renderer (libass) finds them by name through the system (fontconfig on Linux, the system font API on macOS and Windows). The picker lists the families found by `fontdb`.
- Layout (wrap, box size) until 1.3.0 was measured with egui's text layout. Since 1.4.0 the box and the text size are computed by one pure function, `text::fit`, from the box width and the export geometry, so preview and export agree by construction and no font bytes are needed (see Best fit below). libass still does the actual wrapping at the box width.

## Effects
- One pure function builds the ASS events of an item from its times, effects and the timeline. It is tested without `ffmpeg`.
- **Fade IN / Fade OUT**: ASS fade tags (`\fad` or `\fade`), each limited to half of the time to live.
- **Blink**: one event per visible phase (a phase lasts the interval, hidden for the next interval, and so on). At most 2,000 events per item and 20,000 per project; more is refused with a message instead of being written.
- **Slide in**: an ASS `\move` from one box width outside the picture (one box height for top and bottom) to the box, with times relative to the event start. When an event is cut at a segment boundary, the move is recomputed for the cut start.
- Effects combine: the visible phases of a blink carry the fades; a slide plays only at the start.

## Fonts and safety
- Family names come from the project file, which is untrusted: a name is limited to 100 characters from letters, digits, spaces and `- . _ ( )`; anything else is refused on load.
- A font that is not installed on this computer is kept in the project, shown as "not installed here", and drawn with the fallback that libass chooses (documented limitation). Opening the project never fails because of a font.
- Font files are read only as regular files and at most 64 MiB each; the scan of system font folders runs on a background thread at start-up and never blocks the window.
- Text is escaped with the subtitle escaping (`{`, `}` and `\` are replaced), so text cannot inject styling.

## Box, best fit and place (1.4.0)
- **Width only:** a handle on the right edge widens or narrows the box; the left edge stays where it is; the height never changes; corners do not resize a text (REQ-116). One drag is one undo step. The width is 10–100% of the picture width; a new text box is 80% wide.
- **Best fit:** the text size is the largest size at which the text, wrapped at the box width, fits in the box height, but never more than one line filling the height. It is never below 2% of the picture height; text that still does not fit runs past the bottom edge of the box. The estimate is pure: each character counts 0.6 of the font size, a line 1.2 times the font size, the wrap width is the box width minus the subtitle margins; the size is found by 40 halving steps. The same function runs for the preview and for the export at the export geometry, so the two sizes are identical.
- **Consequence:** the real glyph widths can differ from the estimate, so a text may fill 85–115% of its box. Measured on the test clips and accepted; a measured fit with the font's own glyph widths is on the roadmap if the difference is visible.
- **Anchored text on a 360 clip:** the box is computed in the equirectangular frame of its segment: width 10–360° (stored in hundredths of a degree), height 20% of the view's vertical field of view at the time it was added. The same best fit applies in that frame.
- **Reset** (Context Menu, Edit menu, right-click): back to the default place (centre of the lower third) and the default width of 80%.
- **Align horizontally / vertically:** centres the box on that axis, keeping the other coordinate and the width (REQ-115).
- **Dragging:** the centre of the box stays on the picture, so a text can be dragged partly past an edge; the part outside is not drawn in the preview or in the export (REQ-117). On a 360 clip the limit is the look-around view.
- **Migration from 1.3.0 (schema 6):** the size of each item is replaced by a width found by search, so that the best fit gives the old size where the text fits the 20% box; text that cannot reach the old size at full width shrinks. Anchored text takes its width from the old wrap width and its height from the old lines at the old size. This is the confirmed choice (A) of the product owner: the old look is kept where the text fits the box; large multi-line texts shrink. Projects keep their other items unchanged.

## Tests
- Pure: effect envelopes against formulas (fade 50% at half the fade time; blink hidden between one and two intervals; fade capped at half the time to live; slide start one box width away and arrival after its duration), caps, cut-at-boundary moves, validation of hostile values, font name sanitiser.
- With real `ffmpeg`: a text from 2 s to 5 s is present at 3 s and absent at 1 s and 6 s in the export; its centre is within 2% of the picture height of the preview; a project with one text item is re-encoded; the slide of a 400-pixel box reaches its place within one frame.
- Interface (headless): "Add Text" creates one item at the playhead in one undo step, Esc creates nothing, the Text lane bar length equals the time to live at three zoom levels (±1 pixel).
- (1.4.0) Pure: best fit grows with the box width and shrinks with more or longer lines, never below 2% of the height; the same size for preview and export geometry; 200 items in under 10 ms; migration from the 1.3.0 size on the committed 1.3.0 fixture; width handle clamps to 10–100%.

## Risks
- libass and `fontdb` may name a family differently; then the export falls back silently to another font. Mitigation: a test with a font installed on the CI runner.
- egui and libass wrap text a few pixels differently (as for subtitles). Since 1.4.0 libass is the only wrapper of text items.
- (1.4.0) The best-fit size is an estimate: a text may fill 85–115% of its box. Text items from 1.3.0 keep their look where the text fits the box; large multi-line texts shrink when opened in 1.4.0 (confirmed choice).
- Blink with a short interval produces many events: capped as above.
- Fonts parsed by the system's font code (libass, FreeType in `ffmpeg`) are trusted system files; a broken system font is an operating-system issue, recorded here.

## Deviations (1.3.0)
- **Time to live:** the plan refused a time to live longer than the video on load. As built, on load a text item is refused as damaged only outside 0.1 s to 24 hours. A longer-than-video value is accepted and the text is cut at the end of the video; when the item is added, edited or moved, its time to live is clamped to the video length.
