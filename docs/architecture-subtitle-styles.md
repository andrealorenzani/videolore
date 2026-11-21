# Architecture: shadows, highlight colours and new subtitle styles (planned, 2.1.0)

Part of [Architecture](architecture.md) and [2.1.0 overview](architecture-2.1.0-overview.md). Requirements REQ-306 to REQ-310 ([business](business-subtitle-styles.md)). Status: planned.

## Shadows (texts and subtitles)
- Each text item has a shadow setting (0 to 20 px at a picture height of 1080, scaled with the picture height like the text size; default 0 with a default colour of black). The subtitle style has the same setting for all lines of the project.
- The shadow follows the letters, or the box when the box is on. It is drawn behind the outline or the box.
- Rule for the ASS writer, as REQ-307 says: the style's `Shadow` is the distance in pixels (`px * height / 1080`) and its `BackColour` is the shadow colour, for every style, boxed ones included. One text style is generated per distinct colour.
- Boxed styles (`BorderStyle 3`) need the box in a colour field other than `BackColour`. Design A (to be checked by a spike): the box fill is taken from `OutlineColour` in libass, so the box colour moves there (the look is unchanged, since both were the same value in 2.0.4). Design B (fallback if the spike disagrees): the box is drawn as a vector rectangle behind the text, in the box colour, sized from the measured text; the style keeps the shadow and back colour as REQ-307 says.
- Shadow 0 writes the 2.0.4 style values, so the 2.0.4 output is unchanged.
- The same ASS writer serves the preview, the export and the ASS export file. The SRT and WebVTT files carry no style.
- Blinking texts hide their shadow with the text.

## Highlight colours
- The Word highlight style has two colours: **Not yet highlighted** (default white, the style's secondary colour) and **Highlighted** (default yellow, the primary colour). The karaoke tags already in use reveal the highlighted colour word by word.
- The colours are stored in the project and written to the ASS export. The controls appear only for the styles that use them.

## Enlarge word (new style)
- The spoken word is drawn larger by N px at 1080 px height (1 to 40, default 12), scaled with the picture height. The other words do not move.
- Implementation (`src/word_styles.rs` makes the plan, `src/ass.rs` writes it): a base line event for each word time, with the spoken word hidden (`\alpha` fully transparent) so the layout stays; one event for the enlarged word at its measured centre (`\fs` larger, `\pos` at the centre of its place in the line).
- The line layout of the in-house measure must match the one libass draws; a spike checks it at 1080 px (open point, see [decisions and open points](architecture-2.1.0-overview.md#decisions-and-open-points)).
- A line without word times spreads its words evenly, as the word highlight does now.

## Typewriter (new style)
- The spoken word appears letter by letter over its time, as if typed. Words already spoken stay on the picture in the Not-yet colour; words not yet spoken are hidden and keep the layout.
- Implementation: one event per word. The current word uses karaoke letter tags over a transparent secondary colour, so each letter switches to the Highlighted colour at its time. A word shorter than 0.1 s appears at once.
- The preview and the export use the same events, so they show the same letters at the same times.
- No typing sound.

## Caps and cost
- Word styles add about two events per word; the subtitle document limit (2000 events) and the file limit (20000) are checked in tests.
- A shadow on a boxed item doubles that item's events.

## Tests (planned)
- A text with an 8 px red shadow: a red copy offset by 8 px at 1080 px height (within 2% of the frame) in the preview and the export; 0 px gives the 2.0.4 output.
- Boxed text with a shadow: the shadow is behind the box and the text stays readable (box at least 50% darker than the frame).
- Subtitle style with a 6 px shadow: the ASS file has the value and the colour; an old project shows no shadow.
- Highlight colours: the spoken word has the Highlighted colour in the preview, the export and the ASS file; defaults equal 2.0.4.
- Enlarge: the word at its time is N px larger (within 2%); the other words do not move; N = 0 refused; one undo.
- Typewriter: at 0.4 s of a 1 s five-letter word about two letters are shown; all five at the end (within one letter); the export equals the preview at three times.
