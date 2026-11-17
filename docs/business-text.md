# Business — Text

Back to the [index](business.md).

Added in 1.3.0. Text items are titles and captions written by the user and placed on the picture for a chosen time, with optional text effects, colour and font. They are not subtitles (subtitles come from speech or are typed line by line, see [Subtitles](business-subtitles.md)), and they are not a video track: multiple video tracks remain out of scope.

Changed in 1.4.0: a text item's box changes its width only, and its text size fits the box; a text item has the Align and Reset commands of a subtitle line; text items and subtitle lines can be dragged past the edges of the picture.

## Features
- **Add Text**: an "Add Text" entry in the Effects list opens a popup to write a multiline text and choose its effects, time to live, colour and font.
- **Text effects**: Fade IN, Fade OUT, Blink and Slide in, each with its own setting (fade time, blink interval, slide direction and duration). Any combination can be chosen.
- **Time to live**: a text is shown for a set time; its bar on the timeline can be dragged at either end to move it or to make it longer or shorter.
- **Box on the picture**: a text can be moved, also past the edges of the picture, and its box can be widened; the text size fits the box. It is burned into the export.
- **Align and Reset** (1.4.0): the same Context Menu commands as a subtitle line.
- **360 videos**: a text (and a subtitle line) added on a 360 clip stays at the point of the sphere where the camera was looking (see [Boxes on the picture and 360 video](business-picture.md)).

## Requirements

### Adding text
- REQ-108 (implemented): The Effects list has an "Add Text" row, and Effects > Add text at playhead does the same. A click opens a floating, non-modal popup (REQ-076 rules) that creates a text item starting at the playhead. The popup has a multiline text field (Enter starts a new line; the same length limit as subtitle text), the text effects of REQ-109 (any combination of Fade IN, Fade OUT, Blink and Slide in), the time to live (default 3 s), a colour (colour picker, default white) and a font chosen from the fonts installed on the computer (the default is the bundled subtitle font). "Add" creates the item, selects it and is one undo step; Esc or Close creates nothing. On a 360 clip the text is placed as REQ-112 says. Acceptance: with the playhead at 4 s, two lines of text, Fade IN and "Add": a 3 s text item from 4 s with those two lines appears; one undo removes it; Esc creates nothing.

### Text effects
- REQ-109 (implemented): Each text item may have any combination of four text effects, and each chosen effect has its own setting (0.1 to 5 s for all times). **Fade IN** and **Fade OUT** (default 0.5 s): Fade IN goes from invisible to shown at the start, Fade OUT from shown to invisible at the end of the time to live. **Blink** (default interval 0.5 s): the text starts shown and alternates hidden and shown every interval for its whole time to live. **Slide in** (default from left, duration 0.5 s): directions are from left, from right, from top and from bottom. The text starts one box width away (one box height for top and bottom), outside its box and outside the picture, moves at constant speed and reaches its box after the duration; the part outside the picture is not drawn. Each fade is limited to half of the time to live. Effects combine (for example Slide in with Fade IN). Changing a setting is one undo step. Acceptance: (a) a box 400 pixels wide with Slide in from left starts with its left edge 400 pixels left of the box and reaches the box after the set duration (±1 frame); (b) Fade IN of 1 s is 50% visible at 0.5 s (±5%); (c) Blink with an interval of 0.5 s is hidden between 0.5 s and 1.0 s after the start; (d) a fade longer than half the time to live is limited to half of it.

### On the timeline and on the picture
- REQ-110 (implemented): A text item is a bar on a Text lane of the timeline, between the video track and the music track, from its start for its time to live. Dragging the bar moves it in time; dragging its right end changes the time to live (minimum 0.1 s); dragging its left end moves the start and keeps the end. Text items do not move when the video is edited (no ripple, as music in REQ-043); a text running past the end of the video is cut there. A click selects it (REQ-090), moves the playhead to its start and opens its popup (text, effects, time to live, colour, font and Delete; non-modal, Esc closes it). The Delete key and the right-click menu remove the selected text item. Each move, change and removal is one undo step. Acceptance: a 3 s text whose right end is dragged from 3 s to 6 s is visible from its start to 6 s in the preview and the export; one undo brings it back to 3 s; the bar length equals the time to live at the minimum, middle and maximum zoom (±1 pixel).
- REQ-111 (implemented): A text item has a box on the picture at its time, as REQ-096: dragged to move (as REQ-117), and its right edge dragged to change its width only (REQ-116). Its text size is the best fit of REQ-116, and the text wraps inside the box width. A new text starts centred horizontally in the lower third (assumption). The text is drawn in the preview and burned into the export with the same font, colour, effects and place: the centre and the size are within 2% of the picture height in both, as REQ-089. Text that does not fit its box is drawn past the bottom edge of the box. Any project with a text item is re-encoded (REQ-023); the hardware export (REQ-055) and its check (REQ-063) apply. Acceptance: a text from 2 s to 5 s is in the export at 3 s and absent at 1 s and at 6 s; a text dragged to the top left is at the top left in the export; a project with one text item and otherwise identical clips is re-encoded.

### Box, size and Context Menu (1.4.0)
- REQ-115 (implemented): The Context Menu (REQ-106) also works for a selected text item, as for a subtitle line: **Align horizontally**, **Align vertically** and **Reset** (REQ-098, REQ-099). Align centres the text item's box on the picture on that axis, keeping its other coordinate and its width. Reset returns the item to the default place and the default width (REQ-116). One command is one undo step, and the commands and shortcuts are the ones of subtitle lines (REQ-069). Acceptance: a text item moved to the lower left and then Align horizontally has its box centred on the picture's vertical centre line and its height position unchanged; Reset gives the default place and width; with a text item and a subtitle line both selected, both get the commands.
- REQ-116 (implemented): A text item's box changes **width only**: a handle on its right edge widens or narrows it; dragging does not change its height, and the corners do not resize it. While the handle is dragged only a ghost outline shows the new width. A text anchored on a 360 clip (REQ-112) has the same width handle. The text size is the **best fit**: the largest size at which the text, wrapped at the box width, fits inside the box height, but never larger than one line filling the box height. The fit is an estimate from character widths (no font measurement), so the text may fill about 85% to 115% of the box. It is never below 2% of the picture height (assumption); text that still does not fit runs past the box bottom (REQ-111). The size follows the box width in the preview and in the export alike. The box width is between 10% and 100% of the picture width (assumption). A new text box is 80% of the picture width and 20% of the picture height (assumption). One drag is one undo step. Acceptance: widening the box of a two-line text makes the text larger and narrowing it makes the text smaller; the same text in a box of the same width but three times longer lines is smaller; the export size equals the preview size within 2% of the picture height; a box dragged narrower than 10% of the picture width stops at 10%.

## Key decisions
- (1.3.0) A text item is a new kind of object on the picture, with its own timeline lane. It is not a video track and not a subtitle line.
- (1.3.0) Text items keep their timeline time when the video is edited, like the music track (REQ-043); this is an assumption that Andrea can overrule.
- (1.3.0) The text effects Fade IN and Fade OUT have a set time. The video fade "Fade IN/OUT" keeps its fixed length (D1) and is only renamed (REQ-107).
- (1.3.0) Text items and subtitle lines share the box and the move of REQ-096 and REQ-097. (1.4.0) The resize differs: a text item changes its width only (REQ-116).
- (1.4.0) The user does not choose the text size of a text item: it fits the box, so the box width is the way to make text larger or smaller.
- (1.3.0) Text uses fonts installed on the computer (answer of the author); the project stores the font name, and the look may differ per computer.

## Impact on existing features
- REQ-075 and REQ-107: the Effects list gets the "Add Text" row; "Fade through black" is shown as "Fade IN/OUT".
- REQ-077: a text item is a bar on the Text lane, not a marker; its popup opens from its bar.
- REQ-082: a text item cannot be the target of a drag from the Effects list; the "Add Text" row cannot be dropped on a clip.
- REQ-090 to REQ-092: text items are selected, and moved as a group, like subtitle lines.
- REQ-071: Delete removes the selected text item.
- (1.4.0) REQ-069, REQ-098 and REQ-099: the Align and Reset commands of a text item are in the same menus as for subtitle lines; they are not in the Effects menu any more (see [Window, menus and effects](business-ui.md)).
- (1.4.0) REQ-117: a text item can be dragged past the edges of the picture, like a subtitle line.
- REQ-023 and REQ-062: a text item makes the export a re-encode, and the hardware export carries the text as the subtitles do.
- REQ-113: projects with text items are a newer format (see [Editing](business-editing.md)).

## Known limitations
- A text has no outline, shadow or background box, so its readability depends on the colour chosen and on the picture behind it.
- A text that does not fit its box, even at the minimum size, runs past the box; the box height never changes by itself.
- Very short texts become large and very long ones small, because the size only fits the box (1.4.0).
- The best-fit size is an estimate from character widths, not a measurement of the font, so a text can fill about 85% to 115% of its box.
- A text of a 1.3.0 project keeps its size where it fits: it gets the narrowest box width at which it still has its old size, or the full picture width; a large multi-line text can therefore become smaller when the project is opened in 1.4.0.
- Text items are not included in the `.srt` subtitle file.
- Fonts are the ones installed on the computer, so the same text can look different on another computer, and a font missing on a computer is replaced by a default one there (assumption).
- A project file with a text whose time to live is above 24 hours is refused as damaged on opening. When a text is added, edited or moved, its time to live is limited to the video's length; a video cut shorter later keeps its texts and still opens.

## Out of scope
- Keyframes or paths for a text; text moving along a line; animated size.
- Rich text (different bold, italic or colour per word); saved text styles.
- Text-to-speech, text templates, stickers, images and shapes.
- Text on the music track.
