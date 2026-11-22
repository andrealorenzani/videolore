# Architecture: text items and picture items

Part of [Architecture](architecture.md). Requirements: [text](business-text.md) (REQ-108 to 111, 115 to 119, 156, 318, 319), [subtitle styles](business-subtitle-styles.md) (shadow REQ-306, 307). Text items reuse the subtitle path ([subtitles](architecture-subtitles.md)), so preview and export match. Module map: [code-architecture](code-architecture.md).

## Model
- A text item is stored in its own list (`text` module: `Texts`; not subtitle lines). Fields: multiline text (at most 1000 characters per line), start, time to live (0.1 s to 24 hours on load; added, edited or moved, clamped to the video length), effects, colour `#RRGGBB`, font family, a flat centre (1/10000 of the picture), **width** 10 to 100 % of the picture width (height fixed at 20 % of the picture height in 1.4.0; the height is a handle from 2.0.3, 5 to 100 %), readability, shadow (2.1.0), maximum size (2.2.0).
- Texts sit on the timeline by time only: no ripple (like the music track); a text running past the video end is cut there.
- Code: `text/timing.rs` (times and time to live), `text/layout.rs` (measured layout and fit), `text/events.rs` (ASS events cut per segment), `placement::padded_line_rect` (one box rule: the box you drag is the box exported; `ui/placement.rs` `line_box`).

## Rendering and effects
- One pure function builds the ASS events from times, effects and the timeline (tested without ffmpeg). Written into the same subtitle file as subtitles (`ass` module); preview and export use the same file at the same resolution. A text item makes the export a re-encode (REQ-023).
- **Fade IN / Fade OUT:** `\fad` or `\fade`, each limited to half the time to live. **Blink:** one event per visible phase; at most 2000 events per item and 20000 per project; more is refused with a message. **Slide in:** `\move` from one box width outside (one box height for top and bottom) to the box, times relative to the event start; recomputed when an event is cut at a segment boundary; plays only at the start. Effects combine (blink phases carry the fades).
- **Readability (2.0.3):** None (no outline, no box), Outline (black outline, 6 % of the text size; new texts start with it), Background box (black box 60 % opaque, padded 10 % of the text size). Blinking hides the box too. Texts saved before 2.0.3 open as None (their 8 % outline is removed; one status line once per project, not after format 10).
- **Shadow (2.1.0):** 0 to 20 px at 1080 px height, default 0, colour default black. Without a box: the ASS style's shadow; with a box: a copy of the text drawn before it in a shadow style, shifted by the distance. Full rule in [subtitles](architecture-subtitles.md#styles-and-burn-in). Stacking of flat texts: later items on top; from 2.1.0 the order can move a text below the LUT ([effects](architecture-effects.md)).
- Escaping: `{`, `}` and `\` are replaced, so text cannot inject styling.

## Box, size and place
- **Best fit (1.4.0, pure estimate):** the largest size at which the wrapped text fits the box height, never more than one line filling the height, never below 2 % of the picture height; estimate 0.6 em per character, 1.2 em per line, wrap = box width minus subtitle margins, 40 halving steps. Same function in preview and export at the export geometry (identical sizes). A text may fill 85 to 115 % of its box (accepted).
- **Best fit from the font (2.0.3):** measured with the font's glyph widths (kerning included); the text fills 90 to 100 % of the box height, unless the size hits its limits: at most 10 % of the picture height (default cap) or at least 2 % (then the text runs past the box and the box shows a warning). Ten texts (short, wide, narrow, one to four lines) fill 90 to 100 %; preview and export sizes agree within 2 % of the picture height.
- **Maximum (2.2.0, REQ-318, 319, 323, ADR-227, 228):** the box governs the size; the cap is 10 % of the picture height by default; a text may set its own maximum from 2 to 50 % (one decimal; empty keeps the default; other values refused with the business message, the stored value stays). `TextItem::cap()` = `max_size` (thousandths of the height, 20 to 500, written only when set; `text::parse_max_size`) or 100. Texts saved before 2.2.0 keep their sizes. The block ratio is not stored; it follows the wrap. Slide-in starts one box width (height) away. Subtitle lines keep their style size and have no box. `text/size_tests.rs` pins 2.0.4 sizes (52.3, 56.5, 43.2, 43.1, 108 px at 1080p). Anchored at the box centre (`\an5`).
- **Handles:** width only from the right edge (ADR-091; the left edge stays); height from the bottom edge (2.0.3); corners do not resize text (REQ-116). Live drag readout of width and size in % of the height; one drag is one undo step. **Reset** (Context Menu, Edit, right-click): default place (centre of the lower third) and width 80 %. **Align horizontally/vertically** keeps the other coordinate and width (REQ-115).
- **Dragging:** the box centre stays on the picture (REQ-117); the part outside is not drawn. On 360 clips the limit is the look-around view.
- **Migration (1.4.0, schema 6, choice A of the product owner):** each size replaced by a width found by search, so the best fit gives the old size where it fits the 20 % box; larger multi-line texts shrink. Anchored text: width from the old wrap width, height from the old lines. Other items unchanged.
- **Anchored text on 360 (1.3.0 and later):** box in the equirectangular frame of its segment: width 10 to 360 degrees (hundredths of a degree), height 20 % of the view's vertical field at the time it was added ([360 video](architecture-360.md)).

## Fonts and safety
- Family names come from the project (untrusted): at most 100 characters of letters, digits, spaces and `- . _ ( )`; others refused on load.
- An installed font is drawn as chosen; a missing font is kept, shown "not installed here", and drawn in the bundled subtitle font (libass fallback; the popup says so). Opening never fails for a font. The picker lists families found by `fontdb` (0.24, background scan at start-up, charset whitelist, ADR-068); installed fonts are read when a text uses them.
- Font files are read only as regular files, at most 64 MiB each. System fonts parsed by libass and FreeType are trusted OS files.
- Family names may differ between libass and fontdb (silent fallback risk; mitigated by a test with a font on the CI runner).

## Picture items (2.0.4)
- A picture item is a text item with `TextItem.picture` (`Picture` in `src/picture.rs`: path and pixel size from the PNG header, 24 bytes; ffmpeg decodes). It keeps every text rule (lane, box, move, resize, align, undo, time to live, default 3 s); its text is the file name.
- Fitted in the box and centred (`Picture::rect`); no ASS events: `render::picture_overlays` gives one overlay per item; `build_graph_pics` adds a looped still (`-loop 1`) and `overlay enable='between(t,…)'` before the subtitles. A missing file stops an export (`RenderError::Picture`) and is skipped in preview. Text effects do not apply; the 360 export has no pictures.
- Commands: `AddPicture(Option<PathBuf>)` (chooser when none; Effects list row; `Action::AddPicture`), picture popup with time and Delete.
- Limits as built: PNG only; each side at most 16384 px; path at most 4096 bytes; kept by path, not embedded. (The plan's embedded copy and its 20 items, 8 MiB, 16 MP limits are not built; open.) Since 2.3.0 the 16 MP check and the overlap model apply to overlay files, see [picture](architecture-picture.md).

## Deviations (as built)
- Time to live: the plan refused a value longer than the video on load; as built a text is refused only outside 0.1 s to 24 hours, and a longer value is cut at the video end.

## Risks
- libass and fontdb may name a family differently (silent fallback); egui and libass wrap a few pixels differently (libass is the only wrapper of text items since 1.4.0).
- Best-fit estimate may differ from real glyphs (85 to 115 %); a measured fit is on the roadmap if visible.
- Blink with a short interval makes many events (capped). Fonts parsed by system code are trusted.

## Tests
- Pure: effect envelopes (fade 50 % at half the fade time; blink hidden between one and two intervals; fade capped at half the time to live; slide starts one box width away); caps; cut-at-boundary moves; hostile values; font name sanitiser; best fit grows with width, shrinks with lines, never below 2 %; same size for preview and export; 200 items under 10 ms; migration from the 1.3.0 fixture; width handle clamps 10 to 100 %; event building for 200 items under 50 ms; at most 2000 events per item and 20000 per project.
- Real ffmpeg: text 2 s to 5 s present at 3 s, absent at 1 s and 6 s; centre within 2 % of the picture height; a project with one text is re-encoded; slide of a 400-pixel box reaches its place within one frame; 8 px red shadow copy offset within 2 %; boxed shadow behind the box.
- Interface (headless): Add Text creates one item at the playhead in one undo step; Esc creates nothing; the Text lane bar length equals the time to live at three zoom levels (±1 px); readability choice drawn the same in preview and export.
