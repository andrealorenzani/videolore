# Business — Icons

Back to the [index](business.md).

Added in 1.6.0. The whole interface (the editor and the video player) gets meaningful icons: toolbar buttons, menu items, the Effects list, timeline markers, popup and dialog titles. The text stays next to every icon; the icons only help the user recognise a command faster.

## Features
- **Toolbar**: each button has an icon with its name and shortcut in the tooltip.
- **Menus**: each item of File, Edit, Playback, Effects, View and Help has an icon where one fits.
- **Effects list and markers**: each effect row and each marker type has its own icon.
- **Popups and dialogs**: each popup title and each dialog (Export, Settings, About, shortcuts, confirmation dialogs) shows an icon that matches its purpose.
- **Licence**: the icons come from a free set whose licence allows use in a GPL-3.0 program; they are bundled, never downloaded.

## Requirements

### Icons
- REQ-142 (implemented): Every toolbar button, every menu item, every row of the Effects list, every timeline marker type and every popup or dialog title of the editor, and every toolbar button and menu item of the video player, shows a meaningful icon. Icons sit beside the text in menus and lists, and on the buttons of the toolbar; the text and the tooltip (REQ-072) stay, so no control is icon-only without a tooltip. The list of controls and their icons is kept with the code documentation. Acceptance: every control on that list has an icon; a reviewer finds no toolbar button or menu item without one; each dialog and popup title has one; the greyed-out state (REQ-069) also greys the icon.
- REQ-143 (implemented): The icons come from a free icon set licensed under MIT, ISC, BSD or Apache-2.0, or another licence that GPL-3.0 accepts, and no icon needs attribution in the program's window. The icons are bundled with the program; nothing is downloaded at run time. The set's name, version and licence are listed in About and in the third-party notices (REQ-031), and in the installers. Acceptance: the notices list the icon set with its licence text and source; the program starts and draws every icon with no network access; no icon of an unlisted licence is present.

## Key decisions
- (1.6.0) Icons are added to the existing text, not instead of it, because a non-expert user reads words faster than pictures.
- (1.6.0) The licence rule is "GPL-3.0 compatible permissive licence", not one named set; the choice of set is made in the technical documentation.
- (1.6.0) Icons are bundled, not fetched, so the offline rule holds (see [Constraints](business.md)).

## Known limitations
- The icon set is fixed in 1.6.0; users cannot choose another one or change the icon size.

## Out of scope
- Custom icon themes, icon-only controls without text, animated icons, a new application icon for the editor (the existing one stays, REQ-041).
