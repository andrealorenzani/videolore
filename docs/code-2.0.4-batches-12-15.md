# Code — 2.0.4, batches 12 to 15 (interface, player, 360 video, effects, quality)

Stages 8 to 12 of the 2.0.4 development. What is built and what was cut back is listed per stage; the plan is in [architecture-2.0.4-overview.md](architecture-2.0.4-overview.md).

## Stage 8 — interface (REQ-234 to 247)

- **Settings model.** `src/settings/interface.rs` (`Interface`): window, popup places, hidden/collapsed/flipped sections, workspaces (at most 10, names of 1 to 40 characters), toolbar list, shortcuts, theme, text size. Read leniently (`Interface::from_value`): every wrong value is dropped alone. Part of `Settings.interface`.
- **Window and popups (REQ-234 to 236).** `ui/place.rs`: `window_to_save` / `viewport_of` (the window is saved 0.5 s after it stops changing, restored by `native_options_for`; a place far outside the screens is replaced by the system's at the first frame), `saved` (a remembered popup place only when it is on the screen), `default_place` and `fit_width` (in a window of at least 1280x800 a popup goes in the column right of the timeline or above it, never over it).
- **Sections (REQ-237 to 239).** `ui/layout.rs` `arrange` + `Arrangement` (hide, collapse to a 120 px column, dock on the other side); header buttons in `ui/mod.rs` (`section_menu`); `ui/view_menu.rs` (Sections, Workspaces, Theme, Customise toolbar); commands in `app/dispatch_interface.rs`. Reset layout clears all of it.
- **Toolbar (REQ-240, 241).** `toolbar::fixed` is the user's list or the default 12; icon-only by default; every button has its command's name for screen readers (`widget_info`).
- **Shortcuts (REQ-242).** `ui/keymap.rs`: overrides are stored in the settings as "Cmd+Shift+B"; `install` makes them current each frame; `capture` takes the next key, reports a conflict (`RebindConflict`) or applies it. The Shortcuts popup has a Change button per row and Reset shortcuts.
- **Theme and text size (REQ-243, 244).** `ui/theme.rs` (explicit text colours, contrast test ≥ 4.5:1, System follows egui's system theme; text sizes scale the text styles, not the zoom). The choices are in View > Theme and Settings > Appearance and start-up.
- **Text table (REQ-245).** `ui/strings.rs`: `Key` + English table + `tr!(Name)`; a test scans `src/ui/*.rs` for texts written directly in label, button, menu and tooltip calls. Add a key with a new variant at the end of both lists.
- **Accessibility (REQ-246).** eframe `accesskit` feature on (editor and player); tests read the accessibility tree.
- **Edit menu (REQ-247).** Clipboard, History, Clip, Subtitles submenus and Settings… (5 entries).

## Stage 9 — video player (REQ-248 to 252, 254 to 256)

- **Associations (REQ-248).** `scripts/package.sh`: the player's `file-associations` (role viewer) for mp4, m4v, mov, mkv, webm, avi, and the AppImage desktop entry `MimeType`. Windows installer: `packaging/windows-player.nsi` is cargo-packager's template with the video types registered as Open with entries only (`OpenWithProgids`), never as the default; it has not been run on Windows (verification item 29).
- **Open in player / editor (REQ-249, 250).** `src/siblings.rs` (fixed names next to the running program, one argument, no shell); editor: `Command::OpenInPlayer` (File menu); a video given to the editor (`OpenProject` of a non-project path) is imported into the (new, unsaved) project; player: `OpenInEditor`.
- **Session positions (REQ-251).** `PlayerState.placed_memory` by canonical path. **Settings (REQ-252).** `player/src/settings.rs` (`player.json` in the config folder; bounded read, checked values). **Playlist (REQ-254).** `player/src/playlist.rs` (session only, folders capped at 10 000, name order), Next/Previous, `after_end`. **About (REQ-255).** `notices::components_player` and `packaging/notices/third-party-player.txt` (a test checks it is the editor's file without the speech block). **Preview copies with sound (REQ-256).** The copy always mapped the audio; `PARAMS_VERSION` is 2 so older copies are made again; real-ffmpeg test of the sync.

## Stage 10 — 360 video (REQ-261 to 267, 269 to 272)

- **Marker (REQ-261, 262).** `spherical::mark_matroska_with`: ffmpeg writes padding (a Void element) before the Info element; the Projection element grows the Tracks element and the padding shrinks by the same bytes, so nothing after Tracks moves and only the SeekHead entries of the moved elements change (same byte count). Done in place. `mark_equirectangular_with` and the Matroska writer carry the source's initial view; the export gives them the first 360 source's `format.pose`.
- **Initial view (REQ-269).** `media::spherical_pose` (ffprobe side data, range checked) and `spherical::read_pose` (Spherical V1 XML of MP4/MOV, which ffprobe does not report); `camera::start_camera`; `render/segments.rs:start_path` puts the pose as the first point of the path when the user has none at 0.
- **Mark (REQ-263).** `MediaFormat.marked_360` (only a 2:1 picture); Clip details checkbox, `SetMarked360` (not an undo step: media data).
- **Presets and note (REQ-264, 265).** `choice::video_field_reason`, `choice::sphere_frame_note`; the 360 plan drops frame effects (`plan.rs`).
- **Speed and drag (REQ-266, 267).** `camera::command_lines_capped` with `GraphSpec.camera_updates` (10 in an export, 0 in the preview); `Preview.draft` halves the still while a camera change is live, back after `CAMERA_RESTART`.
- **Camera model (REQ-270, 271).** `Camera.roll_cd`, `Ease`, Shift+drag (`RollCamera`), Roll and "Ease in and out" in the POV popup; roll checked on the real `v360`.
- **Keyboard (REQ-272).** The picture takes the focus on a click (`keyboard_camera_id` tells `keymap::collect` to leave the arrows): arrows turn 5°, Page Up/Down zoom, Enter makes a POV.

## Stage 11 — effects, frame, LUT (REQ-274 to 280, 282 to 285, 289 to 294)

- **Fades (274, 275).** `Timeline.fade_length` (0 = 0.5 s), `fade_sides` (`FadeSide`), `fades_of`; Fades popup length field, side radios in the boundary popup.
- **Music clips (276, 277).** `MusicClip.volume/muted/fade_in/fade_out/clean_audio`; `BedPart::Clip` carries gain, fades and clean flag; Clip details controls.
- **Clean audio (278, 279).** `Clip.clean_strength`: 100 as before, 0 bypass, between a mix (`asplit` and `amix`, weights from the strength); "Normalise speech" is `speechnorm` (`render::NORMALISE_SPEECH`), the permissive branch of requirement 279 (no model file).
- **Crop (283).** `Clip.crop` (0 to 45 % a side, even pixels, before the fit; forces a re-encode; not in 360 export).
- **Keyframe easing (282).** `Key.ease`, `keyframes::set_ease`, the expression generator writes a smoothstep for an eased segment, checkbox in the keyframe popup.
- **Blur and aspect (284, 285).** `BlurLevel`, `Shape::Custom`.
- **LUT (289..294).** Strength (`Lut::with_strength`, the table is blended with the identity, no graph change), `.3dl` and `.csp` readers, domain scaling, missing-file modal (`Modal::MissingLut`, Find, Export without, Cancel), copies in the project (`ProjectFile.lut_copies`, limits 20 MB / 64 MB, project limit 96 MiB), favourites.
- **Volume keys (280).** `Timeline.clip_volume`; `frame::volume_expression` (level interpolated, gain squared) in a `volume=...:eval=frame` filter.

## Stage 12 — quality, cache, page (REQ-295 to 299)

- `splash::should_show`; `AppState::start_splash` (the player follows the editor's choice and keeps its own last version). `ProxyCache::enforce_limit` (oldest first, own finished files, canonical containment, protected = open project's copies) run after each new copy; `Clear preview copies` queues the open project's copies again. Page: `og-card.png`, a Content-Security-Policy with the hash of the one style block (`tests/page.rs` checks both; regenerate the hash when the style changes).

## Tests

Interface: `ui/interface_ux_tests.rs`, `ui/place.rs`, `ui/theme.rs`, `ui/strings.rs`, `settings/interface.rs`. Player: `player/src/tests.rs`, `settings.rs`, `playlist.rs`. Others carry `// covers:` lines.
