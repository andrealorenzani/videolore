# Architecture: text items (1.3.0)

Part of [Architecture](architecture.md). Built in 1.3.0 (requirements REQ-108..111, see [Text](business-text.md)); the 360 anchoring is in [360 video](architecture-360.md#anchored-text-and-subtitles-130). Text items reuse the subtitle rendering path, so preview and export match as they do for subtitles.

## Model
- A text item is a new kind of object, stored in the project in its own list (not as subtitle lines). It has: multiline text (at most 1000 characters per line), start, time to live (0.1 s up to 24 hours on load; when added, edited or moved it is limited to the video length), its effects, colour (`#RRGGBB`), font (family name), a flat place (centre, as subtitle positions, 1/10000 of the picture) and a size (25–400% of the default text size of 5% of the picture height, assumption).
- Text items sit on the timeline by time only: they do not ripple when the video is edited (like the music track), and a text running past the end of the video is cut there.
- Selection, moves and resizes of text items use the same box and selection code as subtitle lines (see [Selection](architecture-ui.md#selection-130)).

## Rendering
- Text items are written as ASS events into the same subtitle file format used for subtitles (`ass` module). Preview and export use the same file at the same resolution, so the look is identical.
- A text item makes the export a re-encode (REQ-023); the hardware export and its check apply as for subtitles.
- Fonts: the family names of the fonts installed on the computer. The subtitle renderer (libass) finds them by name through the system (fontconfig on Linux, the system font API on macOS and Windows). The picker lists the families found by `fontdb`.
- Layout (wrap, box size) is measured with egui's text layout, the same way as subtitle boxes. For an installed font, its bytes are loaded into egui by a background job when the font is first used; until then the box uses the bundled font.

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

## Tests
- Pure: effect envelopes against formulas (fade 50% at half the fade time; blink hidden between one and two intervals; fade capped at half the time to live; slide start one box width away and arrival after its duration), caps, cut-at-boundary moves, validation of hostile values, font name sanitiser.
- With real `ffmpeg`: a text from 2 s to 5 s is present at 3 s and absent at 1 s and 6 s in the export; its centre is within 2% of the picture height of the preview; a project with one text item is re-encoded; the slide of a 400-pixel box reaches its place within one frame.
- Interface (headless): "Add Text" creates one item at the playhead in one undo step, Esc creates nothing, the Text lane bar length equals the time to live at three zoom levels (±1 pixel).

## Risks
- libass and `fontdb` may name a family differently; then the export falls back silently to another font. Mitigation: a test with a font installed on the CI runner.
- egui and libass wrap text a few pixels differently (as for subtitles).
- Blink with a short interval produces many events: capped as above.
- Fonts parsed by the system's font code (libass, FreeType in `ffmpeg`) are trusted system files; a broken system font is an operating-system issue, recorded here.

## Deviations (1.3.0)
- **Time to live:** the plan refused a time to live longer than the video on load. As built, on load a text item is refused as damaged only outside 0.1 s to 24 hours. A longer-than-video value is accepted and the text is cut at the end of the video; when the item is added, edited or moved, its time to live is clamped to the video length.
