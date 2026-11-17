# Business — Text

Back to the [index](business.md).

Added in 1.3.0. Text items are titles and captions written by the user and placed on the picture for a chosen time, with optional text effects, colour and font. They are not subtitles (subtitles come from speech or are typed line by line, see [Subtitles](business-subtitles.md)), and they are not a video track: multiple video tracks remain out of scope.

## Features
- **Add Text**: an "Add Text" entry in the Effects list opens a popup to write a multiline text and choose its effects, time to live, colour and font.
- **Text effects**: Fade IN, Fade OUT, Blink and Slide in, each with its own setting (fade time, blink interval, slide direction and duration). Any combination can be chosen.
- **Time to live**: a text is shown for a set time; its bar on the timeline can be dragged at either end to move it or to make it longer or shorter.
- **Box on the picture**: a text can be moved and resized like a subtitle line; it is burned into the export.
- **360 videos**: a text (and a subtitle line) added on a 360 clip stays at the point of the sphere where the camera was looking (see [Boxes on the picture and 360 video](business-picture.md)).

## Requirements

### Adding text
- REQ-108 (implemented): The Effects list has an "Add Text" row, and Effects > Add text at playhead does the same. A click opens a floating, non-modal popup (REQ-076 rules) that creates a text item starting at the playhead. The popup has a multiline text field (Enter starts a new line; the same length limit as subtitle text), the text effects of REQ-109 (any combination of Fade IN, Fade OUT, Blink and Slide in), the time to live (default 3 s), a colour (colour picker, default white) and a font chosen from the fonts installed on the computer (the default is the bundled subtitle font). "Add" creates the item, selects it and is one undo step; Esc or Close creates nothing. On a 360 clip the text is placed as REQ-112 says. Acceptance: with the playhead at 4 s, two lines of text, Fade IN and "Add": a 3 s text item from 4 s with those two lines appears; one undo removes it; Esc creates nothing.

### Text effects
- REQ-109 (implemented): Each text item may have any combination of four text effects, and each chosen effect has its own setting (0.1 to 5 s for all times). **Fade IN** and **Fade OUT** (default 0.5 s): Fade IN goes from invisible to shown at the start, Fade OUT from shown to invisible at the end of the time to live. **Blink** (default interval 0.5 s): the text starts shown and alternates hidden and shown every interval for its whole time to live. **Slide in** (default from left, duration 0.5 s): directions are from left, from right, from top and from bottom. The text starts one box width away (one box height for top and bottom), outside its box and outside the picture, moves at constant speed and reaches its box after the duration; the part outside the picture is not drawn. Each fade is limited to half of the time to live. Effects combine (for example Slide in with Fade IN). Changing a setting is one undo step. Acceptance: (a) a box 400 pixels wide with Slide in from left starts with its left edge 400 pixels left of the box and reaches the box after the set duration (±1 frame); (b) Fade IN of 1 s is 50% visible at 0.5 s (±5%); (c) Blink with an interval of 0.5 s is hidden between 0.5 s and 1.0 s after the start; (d) a fade longer than half the time to live is limited to half of it.

### On the timeline and on the picture
- REQ-110 (implemented): A text item is a bar on a Text lane of the timeline, between the video track and the music track, from its start for its time to live. Dragging the bar moves it in time; dragging its right end changes the time to live (minimum 0.1 s); dragging its left end moves the start and keeps the end. Text items do not move when the video is edited (no ripple, as music in REQ-043); a text running past the end of the video is cut there. A click selects it (REQ-090), moves the playhead to its start and opens its popup (text, effects, time to live, colour, font and Delete; non-modal, Esc closes it). The Delete key and the right-click menu remove the selected text item. Each move, change and removal is one undo step. Acceptance: a 3 s text whose right end is dragged from 3 s to 6 s is visible from its start to 6 s in the preview and the export; one undo brings it back to 3 s; the bar length equals the time to live at the minimum, middle and maximum zoom (±1 pixel).
- REQ-111 (implemented): A text item has a box on the picture at its time, as REQ-096: dragged to move (it stays inside the picture, as REQ-089), dragged at a corner to enlarge or shrink with its proportions kept (as REQ-097). Its size is a factor of the default text size, from 25% to 400%, and the text wraps inside the box width. The default text size is 5% of the picture height and a new text starts centred horizontally in the lower third (both assumptions). The text is drawn in the preview and burned into the export with the same font, colour, effects and place: the centre and the size are within 2% of the picture height in both, as REQ-089. Text that does not fit its box is drawn past the bottom edge of the box. Any project with a text item is re-encoded (REQ-023); the hardware export (REQ-055) and its check (REQ-063) apply. Acceptance: a text from 2 s to 5 s is in the export at 3 s and absent at 1 s and at 6 s; a text dragged to the top left is at the top left in the export; a project with one text item and otherwise identical clips is re-encoded.

## Key decisions
- (1.3.0) A text item is a new kind of object on the picture, with its own timeline lane. It is not a video track and not a subtitle line.
- (1.3.0) Text items keep their timeline time when the video is edited, like the music track (REQ-043); this is an assumption that Andrea can overrule.
- (1.3.0) The text effects Fade IN and Fade OUT have a set time. The video fade "Fade IN/OUT" keeps its fixed length (D1) and is only renamed (REQ-107).
- (1.3.0) Text items and subtitle lines share the box, the move and the resize of REQ-096 and REQ-097.
- (1.3.0) Text uses fonts installed on the computer (answer of the author); the project stores the font name, and the look may differ per computer.

## Impact on existing features
- REQ-075 and REQ-107: the Effects list gets the "Add Text" row; "Fade through black" is shown as "Fade IN/OUT".
- REQ-077: a text item is a bar on the Text lane, not a marker; its popup opens from its bar.
- REQ-082: a text item cannot be the target of a drag from the Effects list; the "Add Text" row cannot be dropped on a clip.
- REQ-090 to REQ-092: text items are selected, and moved as a group, like subtitle lines.
- REQ-071: Delete removes the selected text item.
- REQ-023 and REQ-062: a text item makes the export a re-encode, and the hardware export carries the text as the subtitles do.
- REQ-113: projects with text items are a newer format (see [Editing](business-editing.md)).

## Known limitations
- A text has no outline, shadow or background box, so its readability depends on the colour chosen and on the picture behind it.
- A text that does not fit its box runs past the box; the box is not resized by itself.
- Text items are not included in the `.srt` subtitle file.
- Fonts are the ones installed on the computer, so the same text can look different on another computer, and a font missing on a computer is replaced by a default one there (assumption).
- A project file with a text whose time to live is above 24 hours is refused as damaged on opening. When a text is added, edited or moved, its time to live is limited to the video's length; a video cut shorter later keeps its texts and still opens.

## Out of scope
- Keyframes or paths for a text; text moving along a line; animated size.
- Rich text (different bold, italic or colour per word); saved text styles.
- Text-to-speech, text templates, stickers, images and shapes.
- Text on the music track.
