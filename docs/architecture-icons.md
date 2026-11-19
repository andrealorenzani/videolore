# Architecture: icons

Part of [Architecture](architecture.md). Requirements REQ-142 and REQ-143 ([business](business-icons.md)). Planned for 1.6.0.

## Choice of set
- **Phosphor Icons, Regular weight** (MIT licence for the icons and for the font). Bundled as one font file, `assets/icons/Phosphor-Regular.ttf`, with `LICENSE-Phosphor.txt` and `SOURCE.txt` (version, source address, SHA-256), next to the other bundled font.
- Not chosen: `egui-phosphor` 0.14.0 (it requires egui 0.36, not 0.37; no 0.37 release was found); `egui-pop-phosphor` (a fork for 0.37 with an unknown maintainer); SVG icons through `egui_extras` (the SVG renderer is MPL-2.0, outside the permissive licences of REQ-143); Font Awesome (CC BY 4.0 requires attribution in the window, which REQ-143 excludes). Lucide (ISC) is the fallback if the Phosphor licence audit fails.

## How icons are drawn
- `ui::icons::install(ctx)` adds the font as a fallback of egui's proportional family, so an icon can be part of any text, and its width is measured with the text (toolbar widths keep working).
- Each icon is a named constant (a private-use code point) in `src/ui/icons.rs`. Nothing else in the code uses raw code points.
- The icon sits before the text in menus, lists and buttons. Tooltips and shortcut texts stay.
- A disabled control greys its icon with its text (the same colour rule).
- `ActionId` (the table of commands shared by menus, toolbar, shortcuts) has a required `icon` field, so a command without an icon does not compile.

## Tests (planned)
- Every icon constant exists in the font's character map (`ttf-parser`, a development dependency already in the tree through `fontdb`).
- Every `ActionId`, toolbar entry, Effects row, marker type and popup title has an icon (a table test).
- The font file's SHA-256 equals the value in `SOURCE.txt`.
- The notices list the icon set with its licence and source (`scripts/notices.sh --check`, and a test on the About table).

## Notices and packaging
- `packaging/notices/third-party.txt` and the About table list Phosphor Icons, its version, licence (MIT) and source.
- The font is bundled in both programs; nothing is downloaded at run time.

## Risks
- The licence is checked by hand per release (the file's licence text is kept with the font).
- A future icon set change touches every constant; the table test shows what is missing.
