# Architecture: Applied effects column and text size (2.2.0, planned)

Part of [2.2.0 overview](architecture-2.2.0-overview.md). Requirements REQ-317 (column, [applied effects](business-applied-effects.md)), REQ-318, REQ-319 and REQ-323 ([text](business-text.md), D64). Status: built 2026-11-22, not released.

## Applied effects column (REQ-317)
- The Applied effects list is a column on the right of the Preview section, beside the picture. The picture and its controls take the rest; the controls sit under the picture only.
- Width: 15% of the Preview section's width, never less than 180 px. Built: at 1280 x 800 the section is about 885 px, so the column is 180 px (about 20%); 15% governs from a 1200 px section. Wording to be adjusted by the Product Manager. The picture shrinks first.
- The width is kept in the saved layout (an additive settings field, `applied_share`, in thousandths of the section width, default 150, range 80 to 300; the settings version stays 2). A vertical border between picture and column is dragged with the same rules as the other section borders.
- The column scrolls when its rows do not fit. Popups are floating windows and do not move into the column.

## Text size (REQ-318, REQ-319, REQ-323)
- The text box governs the size: the text block, wrapped at the box width, is drawn at the largest size that fits the box in width and height. The size is measured from the glyphs of the text's font (the 2.0.3 best fit), and the block is centred in the box.
- The cap is 10% of the picture height by default. A text may set its own maximum, in percent of the picture height, from 2 to 50 with one decimal; an empty field keeps the default. Other values are refused with the message of the business text, and the stored value stays.
- The ratio of the block is not stored; it follows from the wrap at the current box width.
- Slide-in starts one box width (or height) away, so a larger box gives a longer slide.
- Texts saved before 2.2.0 have no maximum and keep their sizes.
- Subtitle lines keep their style size and have no box.
