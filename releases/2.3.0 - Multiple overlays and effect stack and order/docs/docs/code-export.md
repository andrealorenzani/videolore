# Code — Export dialog, Clean audio and text boxes (1.4.0)

What 1.4.0 added, as built. The requirements are in [Export](business-export.md), [Music and
sound](business-music.md), [Text](business-text.md) and [Boxes on the picture](business-picture.md);
the design is in [architecture-export.md](architecture-export.md#export-dialog-140-req-118-to-122).

## Where things are

| Module | What |
|---|---|
| `export/choice.rs` | Pure. `ExportChoice` (mode, video preset, audio, 360 choice, folder, name), `Context` (what the project is), `file_name_ok`, `with_extension`, `target_path`, `clash`, `folder_problem`, `blocked` (the reason Export is greyed), `request` (what the app starts) |
| `ui/export_dialog.rs` | The window: it shows the model and sends the changed choice back as `Command::SetExportChoice`; the overwrite question |
| `format.rs` | `VIDEO_PRESETS` (12 entries), `AudioChoice`, `FormatChoice`, `preset_settings`, `audio_only_settings` |
| `render/plan.rs` | `plan_export_choice`, `plan_audio_only` + `audio_export_args`, `CLEAN_AUDIO`, `Segment.clean` |
| `export.rs` | `ExportJob.format` / `.audio_only`; `run_audio` (same partial file and rename rule) |
| `app.rs`, `app/effects.rs`, `app/texts.rs` | `PickExport` opens the dialog; `StartExport`, `ConfirmOverwrite`, `SetCleanAudio`, `ResizeTextWidth`, `ResizeTextAnchorWidth` |

## Export dialog

- `File > Export…`, the toolbar button and Ctrl/Cmd+E run `Command::PickExport`, which opens
  `Modal::Export`; while an export runs the same action is "Cancel export". The window edits a
  copy of the choice and sends it back; `StartExport` checks `choice::blocked` again (the dialog
  is not trusted), and if the file exists asks with `Modal::ConfirmOverwrite`.
- The choices are kept in `AppState::export_memory` (session only: never in the project, never in
  Settings). The first time: the project's name in the project's folder, else the last export
  folder of the session, else the user's documents folder.
- The file name is typed, so it is untrusted: no `/ \ : * ? " < > |`, control characters or NUL,
  not `.` or `..`, no trailing dot or space, no Windows reserved name, at most 200 characters.
  The target is `folder.join(name + extension)`: a separator cannot get in. A known extension
  (`mp4 mov m4a mp3 srt`) is replaced, anything else is kept and the extension added.
- Export refuses a target that is a source file or the project file (`clash`, after
  canonicalising) on top of `export::validate_output`.
- 360: the choice is shown only when every video clip is a 360 clip; a mixed project exports
  flat without a message (`choice::request`). The 1.3.0 question before each export is gone.

## Formats

- `VIDEO_PRESETS`: 1920x1080 at 25/30/60 (H.264 .mp4), 25 (H.264 .mov), 25/30 (HEVC .mp4);
  1280x720 at 25/30 (H.264), 25 (HEVC); 854x480 at 25 (H.264); 3840x2160 at 25 (HEVC .mp4), 30
  (HEVC .mov); since 2.0.4, at the end of the list, AV1 in .mp4 (1920x1080 at 25/30, 1280x720 at 25,
  3840x2160 at 25, 854x480 at 25) and VP9 in .webm (1920x1080 at 25/30, 1280x720 at 25, 854x480 at
  25). Labels read `1280x720 25fps, H.264, .mp4` (the plain codec name; the tooltip says `Encoder:
  libx264`, REQ-220). A preset is always re-encoded (`plan_export_choice`), pictures keep
  letterboxing, only presets the ffmpeg in use can make, with the audio encoder of their container
  (libopus for WebM), are offered (`available_presets` = `preset_settings` is `Some`). A 360 video
  ignores the preset (it keeps the sphere's size). When Change aspect makes a frame longer than
  4096 px the dialog warns (`over_cap_warning`, REQ-221); the export uses the cap.
- Audio: "Same as source" is the automatic choice (the encoder of the source's codec, AAC by
  default); `aac` and `mp3` (`libmp3lame`, only when the encoder list has it) always re-encode.
- "Only audio" builds the audio graph of the whole timeline with the same `build_graph`
  (`video: false`), maps `[outa]` with `-vn`, writes `.m4a` (AAC, mp4 muxer) or `.mp3`.
- "Only subtitles" runs the existing `ExportSrt`.

## Clean audio

- `Clip.clean_audio` (saved, always written). A `Segment.clean` clip gets `afftdn` in its own
  audio chain, **before** the clip volume and the fades (`render::CLEAN_AUDIO`); the music bed is
  not touched; the preview uses the same graph. Any clip with it forces the re-encode.
- The filter is `afftdn=nr=25:nf=-20:tn=0:rf=-60`. Its own noise tracking (`tn=1`) gave results
  that differed by 4 dB with the very first samples of the stream, so a fixed noise floor is
  used; it was tuned on the generated fixture of `tests/ffmpeg_integration/v14.rs` (pink noise
  all the time, a speech-like tone from 2 s to 5 s): noise-only parts come out 9 dB lower, speech
  within 0.2 dB. Quiet speech (about -32 dB) lost up to 2.5 dB in manual tests, and noise much
  quieter than the fixture's is removed completely: a limit of the single fixed setting.
- Marker: a small green square 12 px right of the volume marker (`MarkerKind::CleanAudio`);
  popup "Clean audio" (clip: Remove/Apply; whole video: counts, Apply, Remove); undo names
  "Clean audio" and "Remove clean audio". Merge needs the same setting
  (`DIFFERENT_CLEAN_AUDIO`).

## Text boxes (REQ-111, 115, 116, 117)

- `TextItem.width` (thousandths of the picture width, 100..1000, default 800) replaces `size`; the
  box height is fixed at 20% of the picture. The size is the **best fit** (`text::fit_in`,
  `fit_px`): the largest font height at which the text, wrapped at the box width (characters are
  0.6 of the height wide, rows 1.2), fits the box height; at most a line filling the box, at
  least 2% of the picture height. The same function sizes the text in the preview (`placement::
  text_box` is the box, not a measurement by egui) and in the export (ASS events with left and
  right margins that make the wrap width), so they agree by construction. The estimate can be
  15% off the real glyph widths: a text may fill 85–115% of the box.
- On a 360 clip the box is an angle box: `Anchor360 { yaw_cd, pitch_cd, width_cd, height_cd }`
  (hundredths of a degree). For a subtitle line fixed to the sphere `height_cd` stays the height
  of one line at 100% (its own size is a factor); files of 1.3.0 call the two angles `size_cd` and
  `wrap_cd` and are read through serde aliases.
- Only the right edge changes the width (`Hit::Edge`, `boxes::resize_width`, one undo step); the
  corners do not. Dragging keeps the **centre** on the picture (`placement::clamp_to_picture`),
  so the box may go partly outside; on a 360 view the whole box stays inside the view.
- Align and Reset work for text items too (`app/sphere.rs`): Reset gives the default place
  and width. The Context group is shown for any selection.

## Project format 6

`SCHEMA_VERSION = 6`, always written. New: `clips[].clean_audio`, `texts[].width`,
`anchor.width_cd/height_cd`; gone: `texts[].size`, `media[].format.treat_as_360` (never written).
Files of schema 1 to 5 open; newer ones are refused untouched (`TooNew`). The 360 switch of 1.3.0
is dropped on load (only the file's spherical information counts). A text of 1.3.0 is migrated in
memory (`project::take_legacy_text_sizes`, `migrate_text`): its width is the narrowest whose best
fit is at least the old font size (`text::legacy_width`, measured on 1920x1080), else 100%; an
anchored text gets `height_cd` from the rows it took (`legacy_anchor_height`). One-line and most
texts look as before; a big multi-line text shrinks to fit the box. Saving writes version 6.

## Tests

`export/choice.rs` (names, extensions, clash, reasons, 360 rule), `format.rs` (presets, audio),
`render::tests` (clean chain before volume and fades, formats, audio-only graph and argv),
`text::tests` (fit is monotone, minimum, speed), `project::tests` (migration, hostile values,
`TooNew(7)`), `app/text_tests.rs` (dialog flow, widths, align and reset), `ui` tests (dialog inside
an 800x500 window, Ctrl+E, popup, markers, width handle, menus). Real ffmpeg:
`tests/ffmpeg_integration/v14.rs` (the Clean audio gate, only audio length and music, audio
formats, presets) and two tests in `v13.rs` (text cut at the picture edge, size follows width).

## Known limits

- Best fit is an estimate (no font measurement); a measured fit is future work.
- Clean audio has one fixed strength; very quiet speech can lose up to about 2.5 dB.
- A 360 file without spherical information stays flat and there is no switch.
- Export choices are lost when the program closes (by decision).
