# Code — splash screen and icons (1.6.0)

Back to the [index](code.md). Requirements REQ-133, REQ-134 (splash), REQ-142, REQ-143 (icons).

## Splash

| Where | What |
|---|---|
| `src/splash.rs` (pure) | `IMAGE` / `SUMMARY` (compiled in from `assets/splash/<version>.gif` and `.txt`), `version_text`, `layout`, `fit_text`, `mean_colour`, `text_colour`, `decode` (all GIF frames) |
| `src/ui/splash_ui.rs` | the egui window (`Order::Foreground`, title bar with the close control), textures, `fit_version`, `frame_index` (loops a multi-frame GIF with timed repaints) |
| `src/app.rs` | `Popup::Splash`, `show_splash`, `banner_after_splash` |
| `src/lib.rs` | `real_app(None)` opens the splash; `real_app(Some(path))` does not |

- **Build check.** `include_bytes!`/`include_str!` use `CARGO_PKG_VERSION`: a version without its `.gif` and `.txt` does
  not compile (so every version bump needs the files, even a patch bump; copy the previous ones when only the code changed).
- **Layout.** The image keeps its size (486 x 506 for 1.6.0); the panel is 400 wide and as high as the image; both
  are scaled by the same factor when the window is small (never enlarged). The text "Version X.Y" (major and minor,
  from the version) is fitted by binary search into the rectangle x 74..408, y 381..465 minus a 6 px margin, centred; its
  colour (white or near-black) comes from the mean colour of the rectangle in the first frame.
- **Closing.** Close button, Esc (`keymap::topmost_popup` returns the splash first) and the window's own control all send
  `ClosePopup(Splash)`; no unsaved-changes question. It does not block the editor (it is a window, not a modal).
- **First-start banner (REQ-045)** is held in `banner_after_splash` while the splash is open and shown by `tick` afterwards.
- **A file on the command line** shows no splash; a file opened by the system while the splash is open closes it.
- The image is static today (one frame); the decoder keeps up to 64 frames and the window loops them. The origin and licence
  of the image are noted in `packaging/notices/third-party.txt` as to be confirmed by the author before the release.

## Icons

Phosphor Icons 2.1.2, Regular weight, MIT: `assets/icons/Phosphor-Regular.ttf` with `LICENSE-Phosphor.txt` and `SOURCE.txt`
(version, URL, SHA-256; a test checks the file against it). `ui::icons` holds the constants (generated from the font's own
table; `ALL` lists them), `install(ctx)` adds the font as the last fallback of the proportional and monospace families (so an
icon is one character inside any text and greys out with it), and `label(icon, text)` writes `"{icon} {text}"`. A test
checks every constant against a context whose only font is Phosphor. Nothing is downloaded. Notices: `third-party.txt` (full MIT
text), `notices::components` (About), and the installers copy `assets/icons/{LICENSE-Phosphor.txt,SOURCE.txt}` to `icons/`.

How icons reach the controls: `ActionId::icon()` (exhaustive `match`, so a new action cannot lack one) feeds the menus
(`menu::action_item`), the toolbar (`toolbar::text`, Play becomes Pause while playing) and the submenus;
`effects::effect_icon` the Effects rows; `markers::icon_of` the timeline markers (drawn on the marker's shape);
`popups::popup_icon` the popup titles; the modal headings and the Export dialog have their own constants.

| Control | Icon | Control | Icon |
|---|---|---|---|
| New / Open / Save / Save as | file-plus / folder-open / floppy-disk / floppy-disk-back | Import / Export / Export subtitles | download-simple / export / subtitles |
| Quit | sign-out | Undo / Redo | arrow-u-up-left / arrow-u-up-right |
| Cut / Copy / Paste / Delete | scissors / copy / clipboard / trash | Split / Merge | split-horizontal / arrows-merge |
| Move left / right | arrow-left / arrow-right | Add to timeline / Add music | film-strip / music-note |
| Clip details | sliders | Settings | gear |
| Play / Pause | play / pause | Go to start / end | skip-back / skip-forward |
| Previous / next frame | caret-left / caret-right | Fade IN/OUT (row, marker, popup) | circle-half |
| Subtitles (row, marker, popup, lines) | subtitles | Volume | speaker-high |
| Clean audio | waveform | Change aspect | frame-corners |
| Scale | arrows-out | Move | arrows-out-cardinal |
| Apply LUT | palette | Generate subtitles | magic-wand |
| Add text / Text popup | text-t | Align horizontally / vertically | align-center-horizontal / -vertical |
| Reset | arrow-counter-clockwise | Set POV (marker, popup) | crosshair / video-camera |
| Show boxes | bounding-box | Zoom in / out / fit | magnifying-glass-plus / -minus / corners-out |
| Reset layout | sidebar | Keyboard shortcuts | keyboard |
| About, notices | info | Warnings (ffmpeg missing, replace) | warning |
| Splash window / Close | info / x | Add-fade handle | plus |

videolore player has its own short list (`player/src/ui.rs`, module `icon`): Open (folder-open), Quit (sign-out), Full screen
(corners-out), Subtitles and each track (subtitles), None (x), Open .srt (file-video), About (info), Play/Pause, Start
(skip-back), Sound (speaker-high / speaker-slash), OK (check).

## Tests

`src/ui/icon_ux_tests.rs` (every menu item, the toolbar, the greyed Redo, every popup and dialog title, every marker kind and
effect row), `src/ui/icons.rs` (font cmap, SHA-256 and licence), `src/notices.rs` (REQ-143), `src/splash.rs`,
`src/ui/splash_ux_tests.rs` (opening, three ways of closing, Esc order, banner order, 400 x image-height panel, text fitted,
scaling on a small window). The UI test harness strips the icon from labels (`without_icon`) and keeps the raw label in `Node::raw`.
