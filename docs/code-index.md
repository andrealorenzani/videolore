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
- Splash — `src/splash.rs:Program`, `layout`, `fit_summary`, `decode_capped`; `src/ui/splash_window.rs`, `src/ui/splash_ui.rs`; `splash_window::{SUMMARY_MAX, SUMMARY_FLOOR, summary_height, summary_fits_at_floor}`; tests `src/ui/splash_ux_tests.rs`, `player/src/ui_tests.rs` (REQ-133, 134, 150, 154)
- Icons — `src/ui/icons.rs` (constants, `ALL`); tests `src/ui/icon_ux_tests.rs` (REQ-142, 143)
- Notices and About — `src/notices.rs:components`, `about_text`, `about_text_player` (REQ-031, 143)

## Application core

- Commands — `src/app/command.rs:Command`, `Command::undo_name`, `ZOOM_STEP`
- State and dispatch — `src/app.rs:AppState`, `dispatch`, `dispatch_inner` (the arms are in `src/app/dispatch_{project,subtitles,settings,playback,view}.rs`, answered with `Routed`), `src/app/enabled.rs:is_enabled`, `disabled_reason`, `src/app/tick.rs:tick`; tests `src/app/tests.rs` (REQ-001..045 basics)
- Undo — `src/history.rs:History`, `src/app.rs:AppState::edit` (REQ-004, 074)
- Unsaved changes, open/save — `src/app.rs:PendingAction`, `save_interactive`, `open_now`, `load_project`; `src/project.rs:ProjectDoc`
- Background jobs list — `src/app.rs:AppState::background_jobs`, `Job`
- Process cap and priorities — `src/jobs.rs:Scheduler`, `Limited`, `Priority`, `Pool` (REQ-033, 050)
- Private work folders — `src/base/private.rs:Work`, `create_private_file`, `publish`; `src/scratch.rs:cleanup_stale`, `default_root` (REQ-059, REQ-151, REQ-152)

## Media and project model

- Time — `src/time.rs:Micros`, `Fps`, `timecode`
- Media and probing — `src/media.rs:MediaItem`, `MediaFormat`, `SubtitleTrack`, `parse_ffprobe`, `probe`; `src/app.rs:AppState::import`, `probe_pool` (REQ-002, 057, 086, 100)
- Timeline — `src/timeline.rs:Timeline`, `Clip`, `ClipFrames`, `ClipLut`, `FrameTarget`, `EditError` (REQ-005..011, 083, 084, 094, 114, 124, 129..132, 135, 147, 148)
- Music and mix — `src/music.rs:MusicTrack`, `Mix`, `gain`, `clip_gain` (REQ-019, 020, 043, 084)
- Project file — `src/project.rs:Project`, `Edit`, `save`, `load`, `from_json`; `src/base/io.rs:read_limited`, `write_atomic`; `src/project/v9.rs` (format 8 → 9); tests `project::tests` (REQ-003, 016, 029, 057, 094, 105, 113, 123, 132, 136, 137, 149)
- Proxies — `src/proxy.rs:wants_proxy`, `ProxyCache`, `ProxyWorker`, `proxy_args`; `src/app.rs:AppState::maybe_proxy` (REQ-033, 038, 104)

## Effects

- Applying effects — `src/app/effects.rs`; `src/app.rs:EffectKind`, `Popup`, `Popups`, `Instance`; tests `src/app/effects_tests.rs` (REQ-082..092, 147, 148)
- Fades — `src/timeline.rs:Clip` (`fade_in`, `fade_out`), `src/render/segments.rs:segments` (REQ-008, 011)
- Clip volume and Clean audio — `src/timeline.rs:Clip`, `src/render/graph.rs:build_graph` (REQ-084, 114)
- Change aspect, Scale, Move — `src/frame.rs:FrameAspect`, `frame_size`, `Scale`, `Stack`, `Placement`; `src/keyframes.rs:Key`, `at`, `add`, `sanitise`; `src/app/frame.rs`; `src/render/graph.rs:compose_frame`; `src/ui/frame_popups.rs`, `src/ui/frame_handles.rs`; tests `src/app/frame_tests.rs`, `src/ui/frame_ux_tests.rs`, `tests/ffmpeg_integration/v15.rs` (REQ-124..132, 145, 148)
- Apply LUT — `src/lut/mod.rs:Lut`, `lut/parse.rs:parse_reader`, `lut/store.rs:cached`, `Load`, `write_copy`; `src/app/lut.rs`; `src/ui/lut_popup.rs`; tests `src/app/lut_tests.rs`, `src/ui/lut_ux_tests.rs`, `tests/ffmpeg_integration/v16.rs` (REQ-135..137)
- Effects list, menus, markers — `src/ui/effects.rs`, `src/ui/menu.rs`, `src/ui/markers.rs`, `src/ui/actions.rs` (REQ-069, 075..083, 090, 114)

## Subtitles and texts

- Subtitle model — `src/subtitles.rs:Subtitles`, `Line`, `Pos`, `Preset`, `ShownLine` (REQ-016, 018, 044, 085, 089, 092, 097, 099)
- Generation (speech) — `src/stt.rs:SttJob`, `start`, `run`, `ModelStore`, `builtin_models`, `install_model`, `whisper_full_error`; `src/app/subs_create.rs` (REQ-014, 015, 052..054, 058, 085, 087, 095)
- Video's own tracks and word matching — `src/stt.rs:read_track`; `src/align.rs:align`, `spread`; `src/srt.rs:parse`, `write` (REQ-057, 087, 088, 095)
- ASS writing — `src/ass.rs:document`, `document_with_texts`, `font_size`, `line_tag` (REQ-017, 089, 097)
- Text items — `src/text.rs:TextItem`, `Texts`, `TextEffects`, `events`, `fit_px`; `src/base/pos.rs:Anchor360`; `src/app/texts.rs`; `src/ui/text_popup.rs`; tests `src/app/text_tests.rs`, `src/ui/text_ux_tests.rs` (REQ-106..113, 115..118, 122, 123)
- Selection — `src/selection.rs:Selection`, `ObjRef` (REQ-090..092)
- Boxes and positions on the picture — `src/ui/boxes.rs`, `src/ui/placement.rs`, `src/ui/picture.rs` (REQ-096..098, 116, 117, 146)
- Fonts — `src/fonts.rs:Fonts`, `families_of` (REQ-108)

## 360° video

- Camera — `src/camera.rs:Camera`, `Pov`, `CameraPaths`, `at`, `filter`, `command_lines`, `project`, `unproject`; `src/app/sphere.rs` (REQ-096..105, 112)
- 360 export marker — `src/spherical.rs:mark_equirectangular` (REQ-103)
- Tests — `src/app/sphere_tests.rs`, `src/ui/sphere_ux_tests.rs`, `tests/ffmpeg_integration/v126.rs`, `v13.rs`

## Preview

- Player of the preview — `src/preview.rs:Preview`, `request_still`, `play`, `seek`, `poll`, `AudioBuffer`, `AudioSink`, `audio_correction` (REQ-013, 034, 035, 039, 144)
- Preview command lines — `src/render/args.rs:frame_key`, `still_args`, `preview_video_args`, `preview_audio_args`
- Preview picture and handles — `src/ui/picture.rs`, `src/ui/zoom.rs` (REQ-012)

## Export

- Export dialog — `src/export/choice.rs:ExportChoice`, `Context`, `request`, `clash`, `blocked`; `src/ui/export_dialog.rs`; `src/app/export_flow.rs:export_context`, `start_export_with` (REQ-118..122)
- Formats and presets — `src/format.rs:choose_output`, `encode_settings`, `available_presets`, `audio_only_settings` (REQ-022, 120, 121)
- Plan and graph — `src/render/plan.rs:plan_export_choice`, `subtitles_doc`; `src/render/graph.rs:build_graph`, `compose_frame`; `src/render/args.rs:export_args`; `src/render/inputs.rs:music_bed`; golden tests `src/render/golden_tests.rs` (REQ-011..023, 029, 036..038, 062, 094, 097, 105, 131)
- Export worker — `src/export.rs:start`, `run`, `ExportJob`, `ExportEvent`, `ExportMethod`, `validate_output`, `write_subtitles` (REQ-017, 021, 023)
- SRT export — `src/srt.rs:write`; `src/app/effects.rs` (REQ-087, 088)

## Hardware export

- Candidates and arguments — `src/hwenc.rs:Vendor`, `Codec`, `Candidate`, `candidates`, `encode_args`, `HwEncode`; tests `src/hwenc/tests.rs` (REQ-061, 062)
- Hardware check — `src/hwenc.rs:probe`, `probe_all`, `ProbeLimits`; `src/app.rs:AppState::start_hw_probe`, `hw_offer`; tests `src/app/hw_tests.rs` (REQ-055, 060, 061, 064)
- Output check and redo — `src/outcheck.rs:check_metadata`, `compare_args`, `judge`; `src/export.rs:hw_choice`; tests `src/outcheck/tests.rs`, `src/export/hw_tests.rs`, `tests/ffmpeg_integration/hw.rs` (REQ-055, 061..063)
- Faster, 2.0.4 — see the sections "Faster hardware export" and "Hardware reach" below.

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
- CI — `.github/workflows/build.yml` (checked by `src/packaging.rs` tests); jobs `check` (pinned test ffmpeg), `package-config` (`package.sh --config-only` / `--stub`, QG-10)
- Timing helper — `src/base/testing.rs:best_of` (QG-09)

## Integration tests (real ffmpeg)

- `tests/ffmpeg_integration.rs` — shared `Env`, fixtures, basics (REQ-001, 002, 011..023, 036..039, 043, 052)
- `tests/ffmpeg_integration/v12.rs` (effects, own subtitles), `v126.rs` (360), `v13.rs` (texts), `v14.rs` (export dialog, Clean audio), `v15.rs` (frame), `v16.rs` (LUT), `v20.rs` (2.0.2: paused frame at a fade REQ-144, format 8 project REQ-137, encoders and filters REQ-121), `hw.rs` (hardware paths); `tests/ffmpeg_integration.rs:assert_pinned` (the pinned ffmpeg in CI, QG-08)

## Readable text, placement and accuracy (2.0.3)

Details in [code-readable-text.md](code-readable-text.md).

- Blur radius (step 0) — `src/render/graph.rs:blur_radius`, `src/frame.rs:BLUR_MIN_SIDE`; tests `render::tests::the_blur_radius_stays_inside_ffmpegs_bounds_for_every_small_side`, `tests/ffmpeg_integration/v15.rs` (REQ-126)
- Paused picture at a clip end — `src/render/segments.rs:Segment::lead`, `read_back`, `Segment.grid`; tests `render::tests::a_picture_from_inside_a_frame_is_read_from_the_frame_that_holds_it`, `v20::a_paused_picture_in_the_last_frames_of_a_clip_is_shown` (REQ-155)
- Text measure — `src/measure.rs:Shaped::new`, `fit`, `at_size`, `line_units`; `src/text.rs:layout`, `layout_in`, `layout_at`, `fit_px`, `legacy_fit_px`, `TextLayout`; tests `measure::tests`, `text::tests`, `v21::the_size_is_measured_with_the_font_and_capped_at_ten_percent_of_the_height`, `v21::the_preview_draws_the_text_as_large_as_the_export` (REQ-116, 156, 157, 159, 163)
- Fonts — `src/fonts.rs:resolve`, `install`, `install_dir`, `copy_used`, `bundled`, `Face`; tests `fonts::tests`, `v21::an_installed_font_is_the_one_drawn_and_it_is_measured_for_its_box`, `v21::a_missing_font_is_drawn_in_the_bundled_font_and_the_project_keeps_its_name` (REQ-163..166)
- Readability — `src/text.rs:Readability`, `TextItem.readability`, `TextItem.height`; `src/ass.rs:text_styles`, `TEXT_BOX_STYLE`, `AssEvent.boxed`; `src/app/texts.rs:set_readability`; tests `v21::outline_and_background_box_are_drawn_and_none_draws_nothing`, `v21::a_blinking_text_hides_its_box_and_outline_with_it`, `text::tests::readability_is_written_as_none_outline_or_a_box` (REQ-158, 162)
- Project format 10 and the migration note — `src/project.rs:SCHEMA_VERSION`, `TextMigration`, `from_json_noted`, `load_noted`; `src/app.rs:open_now`; tests `project::tests::a_project_from_before_2_0_3_reports_its_texts_once`, `app::text_tests::opening_a_project_from_before_2_0_3_says_once_what_changed_in_its_texts` (REQ-158, 167)
- Text box handles — `src/app/texts.rs:resize_text_box`, `Command::ResizeTextBox`, `TextEdge`; `src/ui/boxes.rs:text_edge_at`, `resize_edge`; `src/ui/picture.rs:text_box_drag`, `readout`, `text_flags`; tests `app::text_tests`, `ui::text_ux_tests` (REQ-159..162)
- Placement rules — `src/placement.rs:Rect`, `line_rect`, `text_rect`, `least_inside`, `place_after_text_edit`, `overlapping_lines`, `partly_outside_at`; `src/app/placing.rs:move_selection`, `move_inside`, `partly_outside`, `overlapping_lines`; tests `placement::tests`, `app::placing_tests`, `v21::a_second_line_of_a_subtitle_keeps_the_centre_of_its_box_in_the_export` (REQ-168, 169, 172, 174..176)
- Placement UI — `src/ui/status.rs:SELECTION_HINT`, `outside_text`; `src/ui/mod.rs:UiCache::overlapping_lines`; `src/ui/picture.rs:pick_at`, `dashed_rect`; `ActionId::MoveInside`; tests `ui::placing_ux_tests` (REQ-169..172, 175)
- Align on the style's place — `src/app/sphere.rs:align_plan`, `ALREADY_CENTRED` (REQ-173)
- Recognition pieces and messages — `src/stt.rs:recognise`, `CHUNK`, `OVERLAP`, `unmatched_message`; tests `stt::tests::words_at_the_boundary_of_a_piece_are_neither_lost_nor_doubled`, `stt::tests::a_track_that_does_not_follow_the_speech_says_so_with_the_count_and_one_that_does_does_not` (REQ-178, 179)
- Accuracy gate (not met) — `tests/ffmpeg_integration/v12.rs:alignment_matches_real_speech` (ignored), `check_fixture`, `speech_model`; fixture `tests/fixtures/align/` (REQ-177, REQ-095)

## Subtitle files and fonts (2.0.4, batch 5)

- Export as .srt, .vtt or .ass — `src/srt.rs:Format`, `write`, `write_vtt`; `src/ass.rs:export_file`; `src/app/effects.rs:export_srt` (the extension chooses); tests `srt::tests::webvtt_has_the_same_cues_as_srt_with_a_header_points_and_no_bom`, `ass::tests::the_ass_file_has_every_line_with_its_own_place_and_size`, `app::subs_import_tests::export_subtitles_writes_srt_vtt_and_ass_from_the_same_lines` (REQ-180, 181)
- Subtitles as a separate track — `src/format.rs:SubtitleMode`, `FormatChoice.subtitles`; `src/render/plan.rs:SubtitleTrack`, `Plan::Reencode.track`, `subtitles_doc_with`; `src/render/args.rs:export_args` (third input, `-c:s`, `-metadata:s:s:0`, `-disposition:s:0 0`); `src/mp4tracks.rs:disable_subtitle_tracks` (MP4 and MOV: the track is switched off); `src/export/choice.rs:shows_subtitle_choice`; `src/ui/export_dialog.rs`; tests `render::tests::a_separate_subtitle_track_is_a_third_input_named_by_language_and_not_switched_on`, `mp4tracks::tests`, `v204::a_separate_track_is_one_subtitle_stream_with_the_language_and_the_picture_has_no_text` (REQ-182)
- Import of .srt and .vtt — `src/srt.rs:read_file`, `parse_vtt`, `FileError`; `src/app/subs_import.rs:ImportDraft`, `open_import`, `confirm_import`, `sibling_file`; `Command::{PickImportSubtitles, OpenImportSubtitles, SetImportRetime, ConfirmImportSubtitles}`; `src/ui/modals.rs:import_dialog`; tests `app::subs_import_tests`, `ui::script_ux_tests` (REQ-183, 185)
- Re-time from the speech — `src/stt.rs:Mode::Retime`, `cues_in`; `src/align.rs:retime`; `src/app/subs_create.rs:continue_create` (import branch), `finish_create`; tests `align::tests::re_timing_moves_a_line_to_its_first_and_last_recognised_word`, `app::subs_import_tests::re_timing_*` (REQ-184)
- Changed media on open — `src/media.rs:FileStat`, `MediaItem.stat`; `src/project.rs:SCHEMA_VERSION` 11, `MediaEntry.file_size`, `modified_ms`; `src/app/subs_create.rs:find_changed_media`, `start_track_check`, `apply_reanalysis`; tests `app::subs_import_tests::media_that_changed_on_disk_is_analysed_again_and_named_unchanged_media_is_not`, `a_project_of_2_0_3_records_the_file_stats_without_an_analysis` (REQ-186)
- Scripts and fonts — `src/base/font.rs:Script`, `script_of`, `scripts_in`; `src/ass.rs:FONT_ARABIC`, `FONT_DEVANAGARI`, `BUNDLED_FACES`, `escape_scripts`; `src/fonts.rs:font_covers`, `coverage_of`, `uncovered`, `missing_note_in`, `face_for`; `src/ui/scripts.rs:install_for`; `src/ui/effects.rs:subtitle_script_note`; `assets/fonts/SOURCE.txt`; tests `fonts::tests`, `ass::tests::the_bundled_faces_are_the_ones_recorded_with_their_licence_and_fit_the_size_limit`, `v204::arabic_and_devanagari_render_their_glyphs_like_a_reference_render`, `ui::script_ux_tests` (REQ-187, 188)

## Editing gestures, autosave and safety (2.0.4, batch 6)

- Autosave and recovery — `src/autosave.rs:due`, `is_newer`, `name_for`, `write`, `find`, `load`, `start`; `src/app/recovery.rs:tick_autosave`, `clear_autosave`, `recover_autosave`; `Modal::Recover`; `src/base/names.rs:is_autosave_file`; tests `autosave::tests`, `app::recovery_tests` (REQ-189, 190)
- Trim by dragging a clip edge — `src/timeline.rs:ClipEdge`, `trimmed_by_edge`; `Command::TrimEdge`; `src/ui/timeline.rs:edge_strips`; tests `timeline::tests::dragging_an_edge_trims_in_whole_frames_inside_the_source`, `ui::gesture_ux_tests` (REQ-191)
- Dragging subtitle and fade markers — `src/subtitles.rs:LineEdge`, `dragged_edge`; `src/timeline.rs:move_fade`; `Command::MoveFade`; `src/ui/timeline.rs:marker_drags`, `MarkerDrag`; tests `subtitles::tests`, `timeline::tests::a_fade_moves_to_an_empty_boundary_only`, `ui::gesture_ux_tests` (REQ-192, 193)
- Marker keys — `src/app/stops.rs:stops`, `step_marker`; `ActionId::{PrevMarker, NextMarker}`; tests `ui::gesture_ux_tests` (REQ-194)
- Paste always works — `src/ui/keymap.rs:action_for` (`Event::PasteImage`), `src/app.rs:CLIPBOARD_MARKER`, `AppState::clipboard_text` (REQ-195)
- Undo kept when a file is removed — `src/history.rs:forget`; `src/app.rs:remove_media` (REQ-196)
- Delete and shift of several lines — `src/subtitles.rs:delete_many`, `shift`; `Command::{DeleteSubtitles, ShiftLines}`; `ActionId::{ShiftLinesEarlier, ShiftLinesLater}` (REQ-197)
- Snap guides and safe areas — `src/placement.rs:snap`, `Guides`, `safe_area`; `src/ui/placement.rs:snapped`, `draw_guides`; `src/ui/picture.rs:draw_safe_areas`; `Settings.snap_to_guides`, `show_safe_areas` (program settings), `Command::{ToggleSnap, ToggleSafeAreas}` (REQ-198, 199)

## Whole-video and keyframe clarity (2.0.4, batch 7)

- Copies shown apart — `src/app/frame.rs:selected_copy`, `select_copy`; `Popups.copy`; `src/ui/frame_popups.rs:copy_rows`, `key_body`; tests `app::clarity_tests`, `ui::clarity_ux_tests` (REQ-200)
- Offer for a new clip — `src/app/offer.rs:NewClipOffer`, `note_new_clips`, `apply_whole_to_new`; `src/ui/status.rs:APPLY_TO_NEW` (REQ-201)
- Two sets of picture handles — `src/app/frame.rs:whole_handle_clip`, `picture_rect_of_copy`; `src/ui/picture.rs:handle_set`, `grab_at` (REQ-202; the dashed set is the copy selected in the whole-video popup)
- Keyframe marker and typed time — `src/ui/frame_popups.rs:KEY_MARKER`, `KEY_TOLERANCE`, `key_time_field`; `src/keyframes.rs:retime`; `Command::MoveKey`; arrow keys through `bar_focus_id` and `src/ui/keymap.rs` (REQ-203)
- Copy and paste keyframes — `src/timeline.rs:KeysClip`, `copy_scale_keys`, `paste_keys`; `src/keyframes.rs:relative`, `place`; `Command::{CopyKeys, PasteKeys}` (REQ-204)
- Same-file LUT — `src/app/lut.rs:set_lut_file` (REQ-205)

## Faster hardware export (2.0.4, batch 8)

Details in [code-2.0.4-batches-8-11.md](code-2.0.4-batches-8-11.md) and [code-hardware.md](code-hardware.md).

- Short output check — `src/outcheck.rs:SampleRange`, `sample_ranges`, `compare_range_args`, `counts_match`, `RANGE_LEN`, `judge_sample`; `src/render/args.rs:check_range_graph`, `CheckGraph`, `expected_frames`; `src/export.rs:Attempt::check_ranges`, `compare`, `check_whole`; tests `outcheck::tests::the_range_comparison_reads_one_second_of_each_side`, `export::hw_tests::the_check_renders_at_most_three_ranges_of_one_second`, `a_file_with_another_frame_count_is_compared_over_the_whole_graph`, `hw::every_hardware_path_makes_the_same_format_as_best_quality` (REQ-206, 063)
- Integrated graphics note — `src/hwenc.rs:Graphics`, `Candidate::graphics`; `src/ui/settings.rs:INTEGRATED_NOTE`, `integrated_note`; tests `hwenc::tests::integrated_and_dedicated_graphics_are_told_apart`, `app::hw_tests::faster_says_when_the_graphics_are_part_of_the_processor` (REQ-207)
- Check waits for jobs — `src/app.rs:AppState::pause_or_resume_hw_probe`, `hw_paused`; test `app::hw_tests::the_check_waits_for_an_export_or_a_subtitle_job` (REQ-208)
- Session memory and busy or hanging — `src/hwenc.rs:Activity`, `classify`, `Reason::{Hanging, SkippedSlow}`, `HwStatus.slow`; `src/ffmpeg.rs:Process::cpu_ms`; `src/machine.rs:process_cpu_ms`; `src/app.rs:AppState.hw_slow`; tests `hwenc::tests::a_busy_candidate_gets_the_retry_and_a_hanging_one_is_skipped_at_once`, `a_candidate_that_ran_out_of_time_twice_is_skipped_in_the_next_check`, `app::hw_tests::a_candidate_that_ran_out_of_time_twice_is_skipped_until_the_restart` (REQ-209, 210)
- Repeat text — `src/ui/settings.rs:REPEATING`; `AppState::hw_repeating`; test `app::hw_tests::a_check_without_an_answer_in_time_is_repeated_once` (REQ-211)

## Hardware reach (2.0.4, batch 9)

- AV1 and VP9 — `src/hwenc.rs:Codec::{Av1, Vp9}`, `preset_bitrate`; `src/format.rs:PresetCodec`, `PresetContainer`, `VIDEO_PRESETS` (21), `preset_settings`, `available_presets`; `src/export.rs:hw_choice`; tests `format::tests::av1_and_vp9_presets_need_their_encoders`, `hwenc::tests::encode_arguments_of_every_method`, `hw::av1_and_vp9_presets_use_the_hardware_and_match_best_quality` (REQ-212, 213)
- Hardware decoding — `src/hwdec.rs:Decoder`, `probe`, `probe_all`, `choose`, `apply`; `src/preview.rs:Preview::set_decoders`, `take_hw_fallback`, `HW_DECODE_FAILED`, `input_codecs`; `src/app.rs:AppState::apply_hw_decoders`; `src/proxy.rs:ProxyTask.hwaccel`; tests `hwdec::tests`, `app::hw_tests::the_preview_decodes_with_the_hardware_only_with_faster`, `a_failing_hardware_decoder_falls_back_without_a_message` (REQ-214)
- Quick Sync constant quality — `src/hwenc.rs:Candidate::constant_quality`, `encode_args`; tests `hwenc::tests::quick_sync_falls_back_to_constant_quality_when_the_driver_refuses_bitrate_control`, `export::hw_tests::constant_quality_keeps_the_file_size_in_check` (REQ-215)
- Graphics card — `src/hwenc.rs:HwStatus::{working, devices, chosen}`, `HwOffer`; `src/settings.rs:Settings.graphics_card`; `Command::SetGraphicsCard`; `src/ui/settings.rs:graphics_card_choices`, `graphics_card_choice`; tests `hwenc::tests::the_chosen_device_is_used_only_when_it_works_for_the_format`, `app::hw_tests::the_chosen_graphics_card_is_used_and_saved` (REQ-216)
- Old libva — `src/hwenc.rs:libva_version_from_name`, `libva_message`, `Platform.libva_version`, `HwStatus.libva_note`; tests `hwenc::tests::the_libva_version_is_read_from_the_file_name`, `old_system_libva_skips_va_api_and_quick_sync`, `app::hw_tests::advanced_names_both_libva_versions` (REQ-217)
- 360 and frame effects under Faster — tests `hw::a_360_export_with_a_text_under_faster_is_checked_and_matches_best_quality`, `hw::frame_effects_under_faster_are_checked_and_match_best_quality` (REQ-218, 219)

## Export and preview speed (2.0.4, batch 10)

- Plain names — `src/format.rs:VideoPreset::label`, `tooltip`; `src/ui/export_dialog.rs:video_text`; tests `format::tests::entries_show_the_plain_codec_name_and_the_tooltip_the_encoder`, `app::speed_tests` (REQ-220)
- 4096 px warning — `src/frame.rs:over_cap`; `src/app/export_flow.rs:frame_over_cap`; `src/ui/export_dialog.rs:OVER_CAP_WARNING`, `over_cap_warning`; test `app::speed_tests::a_frame_larger_than_4096_px_gets_a_warning_before_export` (REQ-221)
- Blurred fill size — `src/frame.rs:BLUR_DIVISOR`; test `render::tests::the_blurred_background_is_computed_at_a_fraction_of_the_frame` (REQ-222)
- Kept background — `src/render/graph.rs:Background`, `build_graph_bg`; `src/render/args.rs:still_args_bg`, `FrameKey::background_name`; `src/preview.rs:render_with_kept_background`, `KeptBackground`, `background_renders`; tests `preview::tests::a_change_of_scale_draws_the_picture_again_but_not_the_blurred_background`, `v204::the_kept_blurred_background_gives_the_same_picture_as_the_whole_render` (REQ-223)
- Queue behind playback and export — `src/app.rs:probe_pool` (Background), `src/app/subs_create.rs:start_track_check`; test `app::speed_tests::imports_wait_behind_playback_and_export` (REQ-224)

## Platforms and installers (2.0.4, batch 11)

- CI — `.github/workflows/installer-config.yml`, `build.yml` (`package-config`, `needs`, the `cpu_message` step), `advisories.yml`; tests `packaging::tests::the_installer_configuration_runs_on_packaging_pull_requests_and_gates_the_executables`, `the_advisory_check_runs_every_week_and_cannot_write`, `the_cpu_message_test_runs_on_every_push_and_fails_when_it_is_missing`, `app::settings_tests::cpu_message_is_shown_when_the_processor_features_are_forced_off` (REQ-225, 226, 227)
- Pinned NSIS and the licence file — `packaging/tools.lock` `[nsis]`; `scripts/package.sh` (`fetch_tool`, `tool NAME`, `packager_config`); tests `packaging::tests::the_nsis_toolchain_is_pinned_by_checksum`, `a_wrong_hash_fails_the_tool_check`, `the_licence_file_of_the_windows_installer_starts_with_a_byte_order_mark` (REQ-228, 229)
- Linux archive — `packaging/linux/launcher.sh`, `launcher.desktop`; `scripts/package.sh` (tar.gz branch); test `packaging::tests::the_linux_archive_has_a_launcher_and_a_desktop_entry` (REQ-231)
- One window — `src/instance.rs:claim`, `Instance::poll`, `poll_current`, `start`; `src/base/names.rs:is_inbox_file`; `src/desktop.rs:take_open_requests`; `src/main.rs`; tests `instance::tests`, `app::tests::projects_opened_by_the_os_go_through_the_prompt` (REQ-230)
- macOS menu bar — `src/desktop.rs:macos_menu`, `MenuSpec`, `MenuEntry`, `macos::install_menu`; test `desktop::tests::the_mac_menu_bar_has_hide_hide_others_and_minimize_and_leaves_cmd_q_to_the_program` (REQ-232)
- Drops on Wayland — the dropped-files path of `src/ui/mod.rs` is the X11 one; File > Import media… and Open… remain; test `ui::ux_tests::dropping_a_project_asks_about_unsaved_changes_then_opens_it` (REQ-233)

## Interface, player, 360 video, effects, quality (2.0.4, batches 12 to 15)

- Interface settings — `src/settings/interface.rs:Interface`, `Window`, `Workspace`; tests same file (REQ-234, 235, 237..244)
- Window and popup places — `src/ui/place.rs:window_to_save`, `viewport_of`, `saved`, `default_place`, `fit_width`; `src/lib.rs:native_options_for` (REQ-234..236)
- Sections, workspaces, toolbar, shortcuts — `src/ui/layout.rs:arrange`, `src/ui/view_menu.rs`, `src/ui/toolbar.rs:fixed`, `src/ui/keymap.rs:overrides`, `capture`; `src/app/dispatch_interface.rs`; tests `src/ui/interface_ux_tests.rs` (REQ-237..242)
- Theme, text size, text table, accessibility — `src/ui/theme.rs`, `src/ui/strings.rs:tr!`, accessibility-tree tests in `src/ui/interface_ux_tests.rs` (REQ-243..246); Edit menu groups `src/ui/menu.rs` (REQ-247)
- Player settings, playlist, links — `player/src/settings.rs`, `player/src/playlist.rs`, `src/siblings.rs`, `src/notices.rs:components_player`; Windows installer template `packaging/windows-player.nsi`; tests `player/src/tests.rs`, `src/packaging.rs` (REQ-248..252, 254..256)
- 360 marker and view — `src/spherical.rs:mark_matroska_with`, `read_pose`, `marker_box_with`; `src/media.rs:spherical_pose`, `marked_360`; `src/camera.rs` (`roll_cd`, `Ease`, `start_camera`, `command_lines_capped`); `src/preview.rs:draft_geometry`; `src/export/choice.rs:sphere_frame_note`; tests `tests/ffmpeg_integration/v204.rs`, `src/app/sphere_tests.rs`, `src/ui/sphere_ux_tests.rs` (REQ-261..267, 269..272)
- Fades, clean audio, speech, crop, music clips — `src/timeline.rs` (`fade_length`, `FadeSide`, `Clip.clean_strength`, `speech_enhance`, `Crop`), `src/music.rs:set_effects`, `src/render/graph.rs` (`crop_filter`, mix of clean sound, bed gains); tests `src/render/tests.rs`, `src/app/effects_tests.rs` (REQ-274..279, 283)
- Keyframe easing, blur, custom aspect, LUT — `src/keyframes.rs:Ease`, `src/frame.rs:BlurLevel`, `Shape::Custom`, `piecewise`; `src/lut.rs` (`parse_3dl`, `parse_csp`, domain, `with_strength`, `embed`); `src/project.rs:lut_copies`; tests `src/lut.rs`, `src/app/lut_tests.rs`, `src/app/frame_tests.rs` (REQ-282, 284, 285, 289..294)
- Splash option, cache limit, page — `src/splash.rs:should_show`, `src/proxy.rs:enforce_limit`, `tests/page.rs` (REQ-295..299)

- Spatial sound — `src/spatial.rs:rotate`, `cardioid_pair`, `Rotator`, `prepare`, `SpatialJob`, `is_ambisonic`; `src/render/segments.rs:spatial_job`, `Segment.spatial`; `src/render/graph.rs` (input `spatial-<hash>.wav`, `Graph.spatial`); `src/render/plan.rs:Plan::spatial`; `src/export.rs:prepare_spatial`; `src/preview.rs:Preview::prepare_spatial`; tests `spatial::tests`, `render::tests` (`ambisonic_project`), `export::tests`, `tests/ffmpeg_integration/v204b.rs` (REQ-273)
- Picture subtitles in the player — `src/media.rs:PictureTrack`, `picture_tracks`, `MediaFormat.picture_tracks`; `src/project.rs:Project.picture_sub`; `src/render/graph.rs` (overlay of `[k:s:N]`); `player/src/subs.rs:Source::Picture`, `menu_name`; `player/src/app.rs:choose_subtitles`; tests `media::tests`, `render::tests`, `player/src/tests.rs`, `player/src/ui_tests.rs`, `player/tests/ffmpeg.rs:pgs` (REQ-253)
- Whole video as copies (2.2.0) — `src/timeline.rs:FrameTarget::Whole`, `add_scale`, `add_move`, `set_lut`; `src/app/frame.rs:apply_frame_effect`, `open_frame_effect`, `clip_at_playhead_or_first`; `src/app/effects.rs:popup_title`; tests `timeline::tests`, `app/clarity_tests`, `ui/clarity_ux_tests`, `app/frame_tests` (REQ-076, 083, 129, 131, 147, 148, 200..202, 303)
- Picture limit — `src/picture.rs:MAX_PICTURE_PIXELS`, `PictureError::TooManyPixels`, `usable`; `src/render/{plan,overlay}.rs`; tests `picture::tests`, `render/tests` (REQ-316)
- LUT preparation — `src/lut/store.rs:Prep`, `PrepMsg`, `is_prepared`, `write_copy_with`; `src/app/lut.rs:unready_luts`, `tick_lut_prep`; `src/app/effects.rs:shown_project`; tests `lut/store prep_tests`, `app/lut_tests` (REQ-312)
- Format 13 and the text maximum — `src/project.rs:SCHEMA_VERSION`, `src/text.rs:TextItem::max_size`, `cap`, `parse_max_size`, `src/ui/text_popup.rs:max_size_field`; tests `project/v13_tests`, `text/size_tests`, `ui/text_ux_tests` (REQ-029, 318, 319, 322, 323)
- Sound group order — `src/order_chain.rs`, `src/effect_order.rs:Row::{Volume,Clean}`, `src/project/order.rs:sound_order`, `sound_swapped`, `src/render/graph.rs` (`volume_first`), `src/app/applied.rs:move_row`; tests `order_chain::tests`, `app/applied_tests`, `render/tests`, `tests/ffmpeg_integration/v14.rs` (REQ-320, 321, 322)
- Picture group order — `src/effect_order.rs:Row::{Crop,Frame,Scale,Move,Fade}`, `FIXED`, `legacy` (the default), `resolve`, `overlay_form`, `needs_steps`; `src/project/order.rs:clip_rows`, `put_on_top`; `src/render/overlay.rs:attach` (`by_steps`, `doc_shift`); `src/render/steps.rs:Step`, `Ctx`, `chain`, `Build::{fit,compose,crop,place_picture,fade,matte}`; `src/render/graph.rs` (`blur_chain`, `colour_chain`, the `by_steps` branch); `src/frame.rs:Stack::{scale,move}_offset_{x,y}`; `src/render/args.rs:FrameKey::background_name`; `src/app/applied.rs:build_applied`, `move_row`, `move_label`; tests `effect_order::tests`, `project/order_tests`, `render/overlay_tests`, `app/applied_tests`, `ui/applied_ux_tests`, `tests/ffmpeg_integration/v22order.rs` (REQ-320, 321, 322)
- Faster output check (REQ-315, measured) — `src/export.rs:Attempt::check`, `strict_decode`, `check_ranges`, `report`, `CheckProgress`; tests `tests/ffmpeg_integration/hw.rs` (`req315_phase_times`, ignored, real hardware), the simulated hardware tests of the same file
- Applied effects rows cache and column — `src/app/applied.rs:applied`, `applied_builds`; `src/ui/mod.rs:preview_panel`, `applied_border`; `src/ui/layout.rs:applied_width`, `drag_applied`; `src/settings.rs:SavedLayout.applied`; tests `app/applied_tests`, `ui/applied_ux_tests`, `ui/layout applied_tests` (REQ-313, 317)
- Speech regions and the blur measurement — `src/speech_regions.rs:grid`, `regions`, `place`, `select`, `FROZEN`; `src/align.rs:SpeechRegions`, `align_with`, `retime_with`, `place_in_speech`; `src/stt.rs:recognise_with_level`, `speech_of`; tests `align::tests`, `stt::tests`, `speech_regions::tests`, `tests/ffmpeg_integration/v12.rs:speech_region_grid_on_the_reference_clip` (ignored), `v22.rs` (REQ-177 built and measured, not met; REQ-314)
- Quality items — `src/text/{timing,layout,events}.rs` (QG-16 split), `src/placement.rs:padded_line_rect`, `line_font`, `line_wrap` (one box rule, QG-16), `build.rs` (QG-17), `src/ui/tests.rs:sampled` (QG-18), `src/render/golden_tests.rs:projects_of_formats_4_and_5_export_with_the_recorded_arguments`, `tests/ffmpeg_integration.rs:memory_stays_small_with_a_large_picture` (QG-15)
