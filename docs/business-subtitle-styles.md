# Business — Subtitle and text styles: shadow, colours, Enlarge word, Typewriter (2.1.0)

Back to the [index](business.md). Built in 2.1.0, with [Applied effects](business-applied-effects.md). The styles it extends are in [Subtitles](business-subtitles.md) (REQ-017, the word-by-word highlight) and [Text](business-text.md) (REQ-158 readability). The subtitle file rules are in [Subtitle files and fonts](business-subtitle-files.md) (REQ-181, the ASS export).

Before 2.1.0 a text or a subtitle line had an outline or a box and no shadow; the word-by-word highlight used white and yellow fixed; there were no word-size or typing effects.

## Features
- **Shadow** for a text item and for the subtitle style: a distance in pixels and a colour.
- **Two highlight colours** for the word-by-word style: not yet highlighted, and highlighted.
- **Enlarge word** style: the word being spoken grows by a number of pixels the user chooses.
- **Typewriter** style: the word being spoken appears letter by letter, as if it were being typed; no sound.

## Requirements

### Shadow
- REQ-306 (implemented): A text item has a **Shadow** setting in its popup (and in the Add Text popup): a distance in pixels from 0 to 20 at a picture height of 1080 pixels, scaled with the picture height like the text size (assumption), and a colour (colour picker, default black). 0 means no shadow and is the default, so existing texts look as before. A value outside the range is brought into it when a project opens. The shadow follows the letters; with a **Background box** it follows the box. It is drawn behind the outline or the box and does not replace them (REQ-158); it hides with a blinking text (REQ-162). It is drawn the same in the preview and in the export. Acceptance: a text with an 8 px red shadow shows a red copy offset to the right and down by 8 px at 1080 px height (within 2% of the frame) in the preview and in the export; with 0 px the output equals the 2.0.4 output; with Background box the shadow is behind the box and the text stays readable (the box stays at least 50% darker than the frame, REQ-158).

- REQ-307 (implemented): The subtitle style has the same **Shadow** setting (REQ-306) for all lines of the project, in the Subtitles popup; a project has one style (existing decision). It is drawn the same in the preview and in the export and stored in the project. The ASS export (REQ-181) writes the distance as the style's shadow and the colour as its back colour, boxed styles included; the SRT and WebVTT files carry no style. A project saved before 2.1.0 has no shadow (0); a value outside the range is brought into it when a project opens. Acceptance: a style with a 6 px shadow in the preview and in the export; the ASS file has the shadow value and colour; an old project shows no shadow.

### Word highlight colours
- REQ-308 (implemented): When the style is **Word highlight**, the Subtitles popup shows two colours: **Not yet highlighted** (default white) and **Highlighted** (default yellow, the colour of the word being spoken today). Both are stored in the project and used in the preview, in the export and in the ASS export (the two karaoke colours of the style). The controls are hidden for the other styles. A change is one undo step. Acceptance: with Not yet highlighted blue and Highlighted red, the spoken word is red and the others blue in the preview and in the export; the ASS file carries the two colours; with the default values the output equals the 2.0.4 output.

### Enlarge word and Typewriter
- REQ-309 (implemented): A subtitle style **Enlarge word**: the word being spoken is drawn larger than the others by a number of pixels the user chooses in the Subtitles popup, from 1 to 40 at a picture height of 1080 pixels, scaled with the picture height, default 12 (assumption). A value outside the range is brought into it when a project opens. The other words keep their size and their place: the line stays centred, so the enlarged word keeps its centre and may overlap the edges of its neighbours by a few pixels (assumption, to avoid words moving while they are spoken). The style needs per-word timing: a line without it spreads its words evenly, as the word-by-word highlight does today. On a 360 clip's line fixed to the sphere the style is drawn as plain text. Acceptance: at a word's time that word is drawn N px larger than the others in the preview and in the export (within 2%); the other words do not move; N=1 is the smallest allowed value and N=0 is refused; one undo restores the value.

- REQ-310 (implemented): A subtitle style **Typewriter**: the word being spoken appears letter by letter, as if it had just been typed, the letters lit one after another over the word's time (a word shorter than 0.1 s appears at once: assumption). A word appears only when its turn starts; words already spoken stay on the picture until the line ends; words not yet spoken are not shown (assumption, [MINOR]). No typing sound (out of scope). It uses the colours of REQ-308 for the typed letters' colour (the highlighted colour for the word being typed, the other colour for the words already spoken; assumption). A preview or an export that starts in the middle of a word shows its typed letters at once. On a 360 clip's line fixed to the sphere the style is drawn as plain text. The export and the preview show the same letters at the same time. Acceptance: a 1 s word of 5 letters shows about 2 letters at its first 0.4 s and all 5 at its end (within one letter); the words after it are not drawn; the export equals the preview at three sample times.

## Edge cases and rules for the builders
- **Old projects**: every new setting has a default that gives the 2.0.4 look (shadow 0, the two colours white and yellow, no new style). Projects saved before 2.1.0 open with that look (REQ-311).
- **Preview and export parity**: the same font measure, the same shadow offset and the same letter timing in both; the 360 export (REQ-257) draws the anchored lines with the same styles.
- **Shadow on a box**: a Background box or Outline with a shadow must still reach the readability contrast of REQ-158; the shadow never covers the text's own colour.
- **Colours**: the colour pickers use the same colour choice as the text colour (default values as above); an ASS colour is written with its alpha; a text with a colour of its own keeps it (REQ-108).
- **Subtitle styles and word timing**: Enlarge word and Typewriter need word timing; a changed line loses it (existing rule) and then uses the even spread.
- **LUT order**: a shadow and a typewriter letter follow the LUT order of the clip (REQ-304): a LUT above the text colours them too.
- **Undo**: each setting change is one undo step, named after the setting.

## Key decisions (2.1.0)
- [MINOR] The shadow is a style setting for subtitles and a text setting for texts, in pixels scaled with the picture height, default off; the colour default is black. Reason: the user asked for a distance in pixels and a colour; scaling keeps preview and export equal (REQ-017).
- [MINOR] The Enlarge word style does not move other words, and the Typewriter style keeps spoken words on the picture until the line ends. Reason: the user asked for the word to be enlarged or typed, not for a re-layout of the line.
- [MINOR] ASS export writes the shadow and the two colours, but not Enlarge word or Typewriter (the ASS file has no simple form for them); the video shows them. Reason: the subtitle files keep the same text and times (REQ-180 rule); the new styles are video effects.

## Known limitations
- The Enlarge word and Typewriter styles are not in the `.srt`, `.vtt` or `.ass` files; the subtitle files keep the plain text and times.
- Enlarged words can overlap their neighbours by a few pixels, and their vertical position can shift by a few pixels at 1080 px.
- On a 360 clip's line fixed to the sphere, Enlarge word and Typewriter are drawn as plain text (no word-size or letter effect there).

## Out of scope (2.1.0)
- Typing sound, a caret, and any sound effect of the typewriter.
- Shadow with its own opacity or blur, and several shadows per text.
- Per-word colours beyond the two highlight colours; a style editor; a style per line.
- Animated shadows and keyframes for the shadow.
