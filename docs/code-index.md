# Code — index

Where each important part of the code is: for each area, the module, the main types and
functions (`file:symbol`), the tests and the requirements they cover (`REQ-NNN`, described in
`docs/business*.md`). Every test that covers a requirement carries `// covers: REQ-NNN`, so
`grep -rn "covers:.*REQ-123" src tests player` finds them all. The module diagrams are in
[code-architecture.md](code-architecture.md).

## Programs and start-up

- Editor entry — `src/main.rs:main` → `src/lib.rs:real_app`, `native_options`, `prepare_desktop`, `app_icon`
- Window — `src/ui/mod.rs:VideoloreApp`, `frame_ui`, `UiCache`; tests `src/ui/tests.rs`, `src/ui/ux_tests.rs`
- Start-up settings — `src/settings.rs:startup`, `recommend`, `budget`, `Settings`; `src/app.rs:start_settings`; tests `src/settings/tests.rs`, `src/app/settings_tests.rs` (REQ-044..054, 059, 060, 064, 065, 096)
- Machine — `src/machine.rs:Machine`, `Fingerprint`, `CpuFeatures`, `own_rss` (REQ-045, 054)
- Finding ffmpeg — `src/ffmpeg.rs:locate_both`, `check`, `FfmpegInfo`, `install_hint` (REQ-030)
- Desktop integration — `src/desktop.rs:integrate_at_startup`, `project_arg`, `push_open_request` (REQ-041)
- Splash — `src/splash.rs:Program`, `layout`, `fit_summary`, `decode_capped`; `src/ui/splash_window.rs`, `src/ui/splash_ui.rs`; tests `src/ui/splash_ux_tests.rs` (REQ-133, 134, 150)
- Icons — `src/ui/icons.rs` (constants, `ALL`); tests `src/ui/icon_ux_tests.rs` (REQ-142, 143)
- Notices and About — `src/notices.rs:components`, `about_text`, `about_text_player` (REQ-031, 143)

## Application core

- Commands — `src/app/command.rs:Command`, `Command::undo_name`, `ZOOM_STEP`
- State and dispatch — `src/app.rs:AppState`, `dispatch`, `dispatch_inner`, `is_enabled`, `disabled_reason`, `tick`; tests `src/app/tests.rs` (REQ-001..045 basics)
- Undo — `src/history.rs:History`, `src/app.rs:AppState::edit` (REQ-004, 074)
- Unsaved changes, open/save — `src/app.rs:PendingAction`, `save_interactive`, `open_now`, `load_project`; `src/project.rs:ProjectDoc`
- Background jobs list — `src/app.rs:AppState::background_jobs`, `Job`
- Process cap and priorities — `src/jobs.rs:Scheduler`, `Limited`, `Priority`, `Pool` (REQ-033, 050)
- Private work folders — `src/scratch.rs:Work`, `create_private_file`, `publish`, `cleanup_stale` (REQ-059)

## Media and project model

- Time — `src/time.rs:Micros`, `Fps`, `timecode`
- Media and probing — `src/media.rs:MediaItem`, `MediaFormat`, `SubtitleTrack`, `parse_ffprobe`, `probe`; `src/app.rs:AppState::import`, `probe_pool` (REQ-002, 057, 086, 100)
- Timeline — `src/timeline.rs:Timeline`, `Clip`, `ClipFrames`, `ClipLut`, `FrameTarget`, `EditError` (REQ-005..011, 083, 084, 094, 114, 124, 129..132, 135, 147, 148)
- Music and mix — `src/music.rs:MusicTrack`, `Mix`, `gain`, `clip_gain` (REQ-019, 020, 043, 084)
- Project file — `src/project.rs:Project`, `Edit`, `save`, `load`, `from_json`, `read_limited`; `src/project/v9.rs` (format 8 → 9); tests `project::tests` (REQ-003, 016, 029, 057, 094, 105, 113, 123, 132, 136, 137, 149)
- Proxies — `src/proxy.rs:wants_proxy`, `ProxyCache`, `ProxyWorker`, `proxy_args`; `src/app.rs:AppState::maybe_proxy` (REQ-033, 038, 104)

## Effects

- Applying effects — `src/app/effects.rs`; `src/app.rs:EffectKind`, `Popup`, `Popups`, `Instance`; tests `src/app/effects_tests.rs` (REQ-082..092, 147, 148)
- Fades — `src/timeline.rs:Clip` (`fade_in`, `fade_out`), `src/render.rs:segments` (REQ-008, 011)
- Clip volume and Clean audio — `src/timeline.rs:Clip`, `src/render.rs:build_graph` (REQ-084, 114)
- Change aspect, Scale, Move — `src/frame.rs:FrameAspect`, `frame_size`, `Scale`, `Stack`, `Placement`; `src/keyframes.rs:Key`, `at`, `add`, `sanitise`; `src/app/frame.rs`; `src/render.rs:compose_frame`; `src/ui/frame_popups.rs`, `src/ui/frame_handles.rs`; tests `src/app/frame_tests.rs`, `src/ui/frame_ux_tests.rs`, `tests/ffmpeg_integration/v15.rs` (REQ-124..132, 145, 148)
- Apply LUT — `src/lut.rs:Lut`, `parse`, `read`, `cached`; `src/app/lut.rs`; `src/ui/lut_popup.rs`; tests `src/app/lut_tests.rs`, `src/ui/lut_ux_tests.rs`, `tests/ffmpeg_integration/v16.rs` (REQ-135..137)
- Effects list, menus, markers — `src/ui/effects.rs`, `src/ui/menu.rs`, `src/ui/markers.rs`, `src/ui/actions.rs` (REQ-069, 075..083, 090, 114)

## Subtitles and texts

- Subtitle model — `src/subtitles.rs:Subtitles`, `Line`, `Pos`, `Preset`, `ShownLine` (REQ-016, 018, 044, 085, 089, 092, 097, 099)
- Generation (speech) — `src/stt.rs:SttJob`, `start`, `run`, `ModelStore`, `builtin_models`, `install_model`, `whisper_full_error`; `src/app/subs_create.rs` (REQ-014, 015, 052..054, 058, 085, 087, 095)
- Video's own tracks and word matching — `src/stt.rs:read_track`; `src/align.rs:align`, `spread`; `src/srt.rs:parse`, `write` (REQ-057, 087, 088, 095)
- ASS writing — `src/ass.rs:document`, `document_with_texts`, `font_size`, `line_tag` (REQ-017, 089, 097)
- Text items — `src/text.rs:TextItem`, `Texts`, `TextEffects`, `Anchor360`, `events`, `fit_px`; `src/app/texts.rs`; `src/ui/text_popup.rs`; tests `src/app/text_tests.rs`, `src/ui/text_ux_tests.rs` (REQ-106..113, 115..118, 122, 123)
- Selection — `src/selection.rs:Selection`, `ObjRef` (REQ-090..092)
- Boxes and positions on the picture — `src/ui/boxes.rs`, `src/ui/placement.rs`, `src/ui/picture.rs` (REQ-096..098, 116, 117, 146)
- Fonts — `src/fonts.rs:Fonts`, `families_of` (REQ-108)

## 360° video

- Camera — `src/camera.rs:Camera`, `Pov`, `CameraPaths`, `at`, `filter`, `command_lines`, `project`, `unproject`; `src/app/sphere.rs` (REQ-096..105, 112)
- 360 export marker — `src/spherical.rs:mark_equirectangular` (REQ-103)
- Tests — `src/app/sphere_tests.rs`, `src/ui/sphere_ux_tests.rs`, `tests/ffmpeg_integration/v126.rs`, `v13.rs`

## Preview

- Player of the preview — `src/preview.rs:Preview`, `request_still`, `play`, `seek`, `poll`, `AudioBuffer`, `AudioSink`, `audio_correction` (REQ-013, 034, 035, 039, 144)
- Preview command lines — `src/render.rs:frame_key`, `still_args`, `preview_video_args`, `preview_audio_args`
- Preview picture and handles — `src/ui/picture.rs`, `src/ui/zoom.rs` (REQ-012)

## Export

- Export dialog — `src/export/choice.rs:ExportChoice`, `Context`, `request`, `clash`, `blocked`; `src/ui/export_dialog.rs`; `src/app/export_flow.rs:export_context`, `start_export_with` (REQ-118..122)
- Formats and presets — `src/format.rs:choose_output`, `encode_settings`, `available_presets`, `audio_only_settings` (REQ-022, 120, 121)
- Plan and graph — `src/render.rs:plan_export_choice`, `build_graph`, `compose_frame`, `export_args`, `music_bed`, `subtitles_doc`; golden tests `src/render/golden_tests.rs` (REQ-011..023, 029, 036..038, 062, 094, 097, 105, 131)
- Export worker — `src/export.rs:start`, `run`, `ExportJob`, `ExportEvent`, `ExportMethod`, `validate_output`, `write_subtitles` (REQ-017, 021, 023)
- SRT export — `src/srt.rs:write`; `src/app/effects.rs` (REQ-087, 088)

## Hardware export

- Candidates and arguments — `src/hwenc.rs:Vendor`, `Codec`, `Candidate`, `candidates`, `encode_args`, `HwEncode`; tests `src/hwenc/tests.rs` (REQ-061, 062)
- Hardware check — `src/hwenc.rs:probe`, `probe_all`, `ProbeLimits`; `src/app.rs:AppState::start_hw_probe`, `hw_offer`; tests `src/app/hw_tests.rs` (REQ-055, 060, 061, 064)
- Output check and redo — `src/outcheck.rs:check_metadata`, `compare_args`, `judge`; `src/export.rs:hw_choice`; tests `src/outcheck/tests.rs`, `src/export/hw_tests.rs`, `tests/ffmpeg_integration/hw.rs` (REQ-055, 061..063)

## Interface

- Layout and sections — `src/ui/layout.rs` (REQ-024, 066, 067)
- Menus, toolbar, shortcuts — `src/ui/menu.rs`, `src/ui/toolbar.rs`, `src/ui/keymap.rs`, `src/ui/actions.rs` (REQ-025, 026, 028, 069..071, 106)
- Timeline view — `src/ui/timeline.rs`, `src/ui/markers.rs`, `src/ui/ruler.rs` (REQ-077, 083, 084, 093)
- Popups, modals, settings window — `src/ui/popups.rs`, `src/ui/modals.rs`, `src/ui/settings.rs`, `src/ui/status.rs`
- File dialogs — `src/dialogs.rs` (native, rfd), `src/app.rs:Dialogs` (trait, faked in tests)

## videolore player

- Entry — `player/src/main.rs` → `player/src/lib.rs:real_app`, `native_options`, `file_arg`
- State — `player/src/app.rs:PlayerState`, `Command`, `Folders`, `is_project_file`; tests `player/src/tests.rs` (REQ-138..141, 150)
- Window — `player/src/ui.rs:PlayerApp`, `frame_ui`, `line_boxes`; tests `player/src/ui_tests.rs` (REQ-138..142, 150)
- Subtitles — `player/src/subs.rs:Subs`, `read_srt`, `to_lines` (REQ-140)
- Full screen — `player/src/fullscreen.rs:FullScreen` (REQ-141)
- Real ffmpeg — `player/tests/ffmpeg.rs` (REQ-138..140)

## Packaging and build

- Release assets and pins — `src/packaging.rs:release_assets`, `parse_lock`, `ffmpeg_pin` (REQ-030, 040..042, 138)
- Scripts — `scripts/package.sh`, `fetch-ffmpeg.sh`, `notices.sh`, `icons.sh`, `version.sh`, `coverage.sh`, `feature-coverage.sh`
- CI — `.github/workflows/build.yml` (checked by `src/packaging.rs` tests)

## Integration tests (real ffmpeg)

- `tests/ffmpeg_integration.rs` — shared `Env`, fixtures, basics (REQ-001, 002, 011..023, 036..039, 043, 052)
- `tests/ffmpeg_integration/v12.rs` (effects, own subtitles), `v126.rs` (360), `v13.rs` (texts), `v14.rs` (export dialog, Clean audio), `v15.rs` (frame), `v16.rs` (LUT), `hw.rs` (hardware paths)
