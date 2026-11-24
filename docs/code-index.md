# Code — index

Where each important part of the code is, by area: the module, the main types and functions (`file:symbol`), the test files and the requirements (`REQ-NNN`, described in `docs/business*.md`). The test functions of a requirement: `scripts/req-code.sh REQ-NNN` (every one carries `// covers: REQ-NNN`). One line per file: `scripts/code-map.sh [word]`; where a symbol is defined: `scripts/symbol.sh Name`. Diagrams: [code-architecture.md](code-architecture.md).

## Programs and start-up
- Editor entry — `src/main.rs:main` → `src/lib.rs:real_app`, `native_options`, `native_options_for`, `prepare_desktop`, `app_icon`
- Window — `src/ui/mod.rs:VideoloreApp`, `frame_ui`, `UiCache`, `preview_panel`, `section_menu`; tests `ui/tests.rs`, `ui/ux_tests.rs`
- Start-up settings — `src/settings.rs:startup`, `recommend`, `budget`, `Settings`, `SavedLayout`; `src/app.rs:start_settings`; tests `settings/tests.rs`, `app/settings_tests.rs` (REQ-044..054, 059, 060, 064, 065, 096, 227 `cpu_message`)
- Machine — `src/machine.rs:Machine`, `Fingerprint`, `CpuFeatures`, `own_rss`, `process_cpu_ms` (REQ-045, 054)
- Finding ffmpeg — `src/ffmpeg.rs:locate_both`, `check`, `FfmpegInfo`, `install_hint`, `Runner`, `Process::cpu_ms`, `fake::FakeRunner` (REQ-030)
- Desktop — `src/desktop.rs:integrate_at_startup`, `project_arg`, `push_open_request`, `take_open_requests`, `macos_menu`, `macos::install_menu` (REQ-041, 232, 233); one window `src/instance.rs:claim`, `Instance::poll`, `poll_current`, `start`; tests `instance/tests.rs` (REQ-230)
- Splash — `src/splash.rs:Program`, `layout`, `fit_summary`, `decode_capped`, `should_show`; `src/ui/splash_window.rs` (`SUMMARY_MAX`, `SUMMARY_FLOOR`, `summary_fits_at_floor`), `src/ui/splash_ui.rs`; `SplashOption::{Always, OncePerVersion}`, `lenient_splash`; tests `ui/splash_ux_tests.rs`, `player/src/ui_tests.rs` (REQ-133, 134, 150, 154, 295)
- Icons — `src/ui/icons.rs` (constants, `ALL`); tests `ui/icon_ux_tests.rs` (REQ-142, 143). Notices — `src/notices.rs:components`, `components_player`, `about_text` (REQ-031, 143, 255)
- Other program — `src/siblings.rs` (`OpenInPlayer`, player `OpenInEditor`; REQ-249, 250)

## Application core
- Commands — `src/app/command.rs:Command`, `Command::undo_name`, `ZOOM_STEP`
- Dispatch — `src/app.rs:AppState::dispatch`, `dispatch_inner`, `DISPATCH_AREAS`, `Routed`; areas `src/app/dispatch_{project,subtitles,settings,interface,playback,view,effects,objects}.rs`; `src/app/enabled.rs:is_enabled`, `disabled_reason`; `src/app/tick.rs:tick`; tests `app/tests.rs` (REQ-001..045)
- Undo — `src/history.rs:History`, `forget`; `src/app.rs:AppState::edit` (REQ-004, 074, 196)
- Open, save, unsaved — `src/app.rs:PendingAction`, `save_interactive`, `open_now`, `load_project`, `remove_media`; `src/project.rs:ProjectDoc`
- Background jobs — `src/app.rs:AppState::background_jobs`, `Job`, `probe_pool`; `src/jobs.rs:Scheduler`, `Limited`, `Priority`, `Pool` (REQ-033, 050, 224)
- Private work folders — `src/base/private.rs:Work`, `create_private_file`, `publish`; `src/scratch.rs:cleanup_stale`, `default_root`; `src/base/names.rs` (REQ-059, 151, 152)
- Autosave — `src/autosave.rs:due`, `name_for`, `write`, `find`, `load`, `start`; `src/app/recovery.rs:tick_autosave`, `clear_autosave`, `recover_autosave`; `Modal::Recover`; tests `app/recovery_tests.rs` (REQ-189, 190)
- Module rules — `src/base/` (leaf: `io`, `private`, `pos`, `names`, `language`, `shadow`, `font`, `testing::best_of`); `tests/module_cycles.rs` (REQ-153, QG-09)

## Media and project model
- Time — `src/time.rs:Micros`, `Fps`, `timecode`
- Media — `src/media.rs:MediaItem`, `MediaFormat` (`audio_tracks`), `AudioTrack`, `audio_tracks`, `SubtitleTrack`, `PictureTrack`, `FileStat`, `parse_ffprobe`, `probe`, `spherical_pose`; `src/app.rs:AppState::import` (REQ-002, 057, 086, 100, 186, 253, 340)
- Timeline — `src/timeline.rs:Timeline`, `Clip` (`speed`, `audio_track`, `len`, `src_len`, `to_timeline`, `to_src`, `clean_strength`), `set_audio_track`, `DIFFERENT_AUDIO_TRACKS`, `ClipEdge`, `KeysClip`, `FrameTarget`, `ClipOrder`, `ClipOverlaps`, `EditError` (REQ-005..011, 083, 084, 094, 114, 124, 129..132, 135, 147, 148, 191, 193, 274, 275, 281)
- Music — `src/music.rs:MusicTrack`, `Mix`, `gain`, `clip_gain`, `set_effects` (REQ-019, 020, 043, 084, 276, 277)
- Project file — `src/project.rs:Project`, `Edit`, `SCHEMA_VERSION` (15), `save`, `load`, `from_json_noted`, `lut_copies`, `validate_overlaps`, `validate_audio_tracks`; `src/project/v9.rs`, `v13.rs` (13 → 14 conversion, `Conversion`), `order.rs`; `src/base/io.rs:read_limited`, `write_atomic`; tests `project::tests`, `project/v12_tests.rs`, `v13_tests.rs`, `v14_tests.rs`, `v15_tests.rs` (REQ-003, 016, 029, 057, 094, 105, 113, 123, 132, 136, 137, 149, 167, 311, 330, 332, 337, 340)
- Proxies — `src/proxy.rs:wants_proxy`, `ProxyCache`, `enforce_limit`, `ProxyWorker`, `proxy_args` (`-map 0:a?`), `PARAMS_VERSION` (3); `src/app.rs:AppState::maybe_proxy` (REQ-033, 038, 104, 256, 297)

## Effects
- The effect table — `src/app/effect_kind.rs:EffectKind`, `SPECS`, `EffectSpec`, `Target`; applying `src/app/effects.rs:apply_effect`, `popup_title`, `shown_project`; `src/ui/effects.rs:rows`, `effect_icon`; tests `app/effects_tests.rs` (REQ-082..092, 147, 148)
- Fades — `src/timeline.rs` (`fade_in`, `fade_out`, `fade_length`, `FadeSide`, `move_fade`); `src/render/segments.rs:segments` (REQ-008, 011, 192, 274, 275)
- Volume and Clean audio — `src/timeline.rs:clip_volume`; `src/frame.rs:volume_expression`; `src/render/graph.rs:build_graph` (`NORMALISE_SPEECH`, `volume_first`); `src/app/effects.rs:add_volume_effect`, `set_clip_audio_track`, `audio_track_labels`; `src/ui/effects.rs:audio_track_choice` (the popup of "Volume and track"); `render/graph.rs:fan_out` (`[n:a:k]`); `spatial::SpatialJob.track`; tests `app/v31_tests.rs`, `ui/v31_ux_tests.rs`, `render/v31_tests.rs`, `v31.rs` (REQ-084, 114, 278..280, 340; [code-audio-tracks.md](code-audio-tracks.md))
- Change speed — `EffectKind::ChangeSpeed`, `Instance::Speed`, `Command::{SetSpeed, SetSpeedAll}`; `src/render/segments.rs` (`Segment.speed`); tests `app/speed_tests.rs` (REQ-281)
- Change aspect, Scale, Move — `src/frame.rs:FrameAspect`, `frame_size`, `Scale`, `Stack`, `Placement`, `BlurLevel`, `Shape::Custom`, `BLUR_DIVISOR`, `over_cap`; `src/keyframes.rs:Key`, `Ease`, `at`, `add`, `retime`, `set_ease`; `src/app/frame.rs:apply_frame_effect`, `open_frame_effect`, `clip_at_playhead_or_first`; `src/render/graph.rs:compose_frame`, `blur_chain`; `src/render/args.rs:FrameKey::own_aspect` (no kept background for a clip's own frame, REQ-335); `src/ui/frame_popups.rs` (`aspect_fields`, `key_time_field`), `frame_handles.rs`; tests `app/frame_tests.rs`, `app/clarity_tests.rs`, `ui/frame_ux_tests.rs`, `ui/clarity_ux_tests.rs`, `v15.rs`, `v31.rs` (REQ-124..132, 145, 148, 200..204, 221, 222, 282, 284..286, 288, 326, 335)
- Apply LUT — `src/lut/mod.rs:Lut` (`with_strength`), `lut/parse.rs:parse_reader`, `lut/store.rs:cached`, `Load`, `Prep`, `is_prepared`, `write_copy`; `src/app/lut.rs:set_lut_file`, `unready_luts`, `tick_lut_prep`; `src/ui/lut_popup.rs`; tests `app/lut_tests.rs`, `ui/lut_ux_tests.rs`, `v16.rs` (REQ-135..137, 205, 289..294, 300, 301, 312)
- Effects list, menus, markers — `src/ui/effects.rs`, `src/ui/menu.rs`, `src/ui/markers.rs`, `src/ui/actions.rs:ACTIONS`, `ActionId::effect`, `EffectUse`, `new_object_row` (Add Text and Overlap media rows, REQ-336) (REQ-069, 075..083, 090, 114, 336)
- Whole video as copies — `src/timeline.rs:add_scale`, `add_move`, `set_lut` with `FrameTarget::Whole` (REQ-076, 083, 129, 131, 147, 148, 200..202, 303)

## Order of the effects and the Applied effects column
- Rows — `src/effect_order.rs:Row`, `NAMED`, `FIXED`, `legacy`, `resolve`, `to_store`, `overlay_form`, `needs_steps`; sound `src/order_chain.rs`; per clip `src/project/order.rs:clip_rows`, `changed_orders`, `put_on_top`, `repair_orders`, `sound_swapped` (REQ-302..305, 320..322)
- Build — `src/render/overlay.rs:attach`, `OverlayStep`; `src/render/steps.rs:Step`, `Ctx`, `chain`, `Build::{fit, compose, crop, place_picture, fade, matte}`; `src/render/graph.rs:overlay_chain`; `src/frame.rs:Stack::{scale,move}_offset_{x,y}`; tests `render/overlay_tests.rs`, `project/order_tests.rs`, `v21.rs`, `v22order.rs`
- Column — `src/app/applied.rs:applied`, `applied_builds`, `build_applied`, `move_row`, `move_label`, `RowMove`; `src/ui/applied_effects.rs`; `src/ui/layout.rs:applied_width`, `drag_applied`; tests `app/applied_tests.rs`, `ui/applied_ux_tests.rs` (REQ-313, 317)

## Overlap media and picture items
- Model — `src/overlap.rs:Overlap`, `Kind`, `rect`, `kept_part`, `rescaled`, `sub_rows`, `lost`; `src/timeline.rs:clip_overlaps`, `next_overlap_id` (REQ-325, 327..331)
- Commands — `src/app/overlap.rs:add_overlap`, `set_overlap_box`, `set_overlap_times`, `remove_overlap`, `REMOVAL_ASK`, `confirm_removal`; `Modal::RemoveOverlaps`; tests `app/overlap_tests.rs` (REQ-329)
- Files — `src/picture.rs:Picture`, `MAX_PICTURE_PIXELS`, `usable`, `read_overlay`, `overlay_usable`, `parse_gif_size` (REQ-287, 316)
- Render — `src/render/plan.rs:picture_overlays`, `PicOverlay`, `Look`; `src/render/graph.rs:build_graph_pics`; `src/preview.rs:input_codecs`; tests `render/overlap_tests.rs`, `v23overlap.rs`
- Interface — `src/ui/overlap_lane.rs`, `src/ui/overlap_box.rs`, `src/ui/media.rs:ADD_MEDIA`; tests `ui/overlap_ux_tests.rs` (REQ-324, 325, 327)

## Subtitles and texts
- Subtitle model — `src/subtitles.rs:Subtitles`, `Line`, `Preset` (`EnlargeWord`, `Typewriter`, `AsTrack`), `StyleSettings`, `ShownLine`, `LineEdge`, `dragged_edge`, `delete_many`, `shift` (REQ-016, 018, 044, 085, 089, 092, 097, 099, 192, 197, 306..310)
- Generation — `src/stt.rs:SttJob`, `RangeJob.audio`, `extraction_args(.., track)`, `start`, `run`, `recognise`, `recognise_with_level`, `Mode::{Align, Retime}`, `ModelStore`, `builtin_models`, `whisper_full_error`; `src/app/subs_create.rs:continue_create`, `finish_create`, `find_changed_media`, `generation_anchors` (REQ-014, 015, 052..054, 058, 085, 087, 095, 178, 179, 339)
- Own tracks and matching — `src/stt.rs:read_track`; `src/align.rs:align`, `spread`, `retime`, `align_with`, `SpeechRegions`; `src/speech_regions.rs:FROZEN` (None); tests `align::tests`, `v12.rs`, `v22.rs` (REQ-057, 087, 088, 095, 177 not met, 184)
- Subtitle files — `src/srt.rs:Format`, `parse`, `parse_vtt`, `read_file`, `write`, `write_vtt`; `src/ass.rs:export_file`; `src/app/subs_import.rs:ImportDraft`, `confirm_import`, `sibling_file`; separate track and "As subtitle track" `src/format.rs:SubtitleMode`, `src/render/plan.rs:SubtitleTrack` (`as_track`), `src/export/choice.rs:subtitle_mode`, `AS_TRACK_REASON`, `src/mp4tracks.rs:disable_subtitle_tracks`; tests `app/subs_import_tests.rs`, `v204.rs`, `v31.rs` (REQ-180..185, 337)
- ASS — `src/ass.rs:document`, `document_styled`, `text_styles`, `font_size`, `line_tag`, `escape_scripts`, `highlight_chunks`, `word_block`; `src/word_styles.rs:typewriter`, `enlarge`, `enlarge_placed`; `src/word_layout.rs:Block::with_enlarged`; `src/measure.rs:Shaped::descent`; tests `ass/style_tests.rs`, `app/style_tests.rs`, `word_layout::tests`, `v23overlap.rs` (REQ-017, 089, 097, 187, 306..310, 333, 334)
- Texts — `src/text.rs:TextItem` (`readability`, `shadow`, `max_size`, `cap`), `Texts`, `TextEffects`, `parse_max_size`; `src/text/{timing,layout,events}.rs`; `src/measure.rs:Shaped`, `fit`, `place_words`; `src/app/texts.rs` (`resize_text_box`, `set_readability`); `src/ui/text_popup.rs`; tests `app/text_tests.rs`, `ui/text_ux_tests.rs`, `text/size_tests.rs`, `v21.rs` (REQ-106..113, 115..118, 122, 123, 156..163, 318, 319, 323)
- Fonts — `src/fonts.rs:Fonts`, `resolve`, `install`, `copy_used`, `coverage_of`, `missing_note_in`; `src/base/font.rs:Script`; `src/ui/scripts.rs:install_for`; tests `fonts::tests`, `ui/script_ux_tests.rs` (REQ-108, 163..166, 187, 188)
- Selection and placement — `src/selection.rs:Selection`, `ObjRef`; `src/placement.rs:Rect`, `line_rect`, `padded_line_rect`, `snap`, `safe_area`; `src/app/placing.rs:move_selection`, `move_inside`; `src/ui/boxes.rs`, `src/ui/placement.rs`, `src/ui/picture.rs`; tests `app/placing_tests.rs`, `ui/placing_ux_tests.rs`, `ui/gesture_ux_tests.rs` (REQ-090..092, 096..098, 116, 117, 146, 168..176, 198, 199)

## 360° video
- Camera — `src/camera.rs:Camera` (`roll_cd`, `Ease`), `Pov`, `CameraPaths` (`clips`), `at`, `filter`, `command_lines_capped`, `command_lines_speed`, `start_camera`; `src/app/sphere.rs` (`align_plan`, `clip_for`); tests `app/sphere_tests.rs`, `ui/sphere_ux_tests.rs`, `v126.rs`, `v13.rs` (REQ-096..105, 112, 173, 257..272)
- Markers — `src/spherical.rs:mark_equirectangular_with`, `mark_matroska_with`, `read_pose`; `src/export/choice.rs:sphere_frame_note` (REQ-103, 261..265)
- Spatial sound — `src/spatial.rs:rotate`, `cardioid_pair`, `Rotator`, `prepare`; `src/export.rs:prepare_spatial`; `src/preview.rs:Preview::prepare_spatial`; tests `spatial::tests`, `v204b.rs` (REQ-273)

## Preview
- Player — `src/preview.rs:Preview`, `request_still`, `play`, `seek`, `poll`, `AudioSink`, `audio_correction`, `render_with_kept_background`, `set_decoders`, `take_hw_fallback` (REQ-013, 034, 035, 039, 144, 155, 214, 223)
- Command lines — `src/render/args.rs:FrameKey`, `frame_key`, `still_args`, `still_args_bg`, `preview_video_args`, `preview_audio_args`
- Picture — `src/ui/picture.rs`, `src/ui/zoom.rs` (REQ-012)

## Export
- Dialog — `src/export/choice.rs:ExportChoice`, `Context`, `request`, `clash`; `src/ui/export_dialog.rs` (`over_cap_warning`); `src/app/export_flow.rs:export_context`, `start_export_with` (REQ-118..122, 220, 221)
- Formats — `src/format.rs:choose_output`, `encode_settings`, `VIDEO_PRESETS` (21), `preset_settings`, `available_presets`, `VideoPreset::label` (REQ-022, 120, 121, 212, 213, 220)
- Plan and graph — `src/render/plan.rs:plan_export_choice`, `subtitles_doc`; `src/render/graph.rs:build_graph`, `build_graph_bg`, `Background`; `src/render/args.rs:export_args`; `src/render/inputs.rs:music_bed`; golden `render/golden_tests.rs` (REQ-011..023, 029, 036..038, 062, 094, 097, 105, 131)
- Worker — `src/export.rs:start`, `run`, `ExportJob`, `ExportEvent`, `ExportMethod`, `validate_output`, `write_subtitles` (REQ-017, 021, 023)

## Hardware export ([code-hardware.md](code-hardware.md))
- Methods — `src/hwenc.rs:Vendor`, `Codec`, `Candidate` (`constant_quality`, `graphics`), `candidates`, `encode_args`, `preset_bitrate`, `HwStatus` (`working`, `slow`, `chosen`, `libva_note`), `Activity`, `Reason` (REQ-061, 062, 207, 209, 210, 212, 213, 215..217)
- Check — `src/hwenc.rs:probe`, `probe_all`, `timed_run`, `ProbeLimits`; `src/app.rs:start_hw_probe`, `pause_or_resume_hw_probe`, `hw_offer`; tests `hwenc/tests.rs`, `app/hw_tests.rs` (REQ-055, 060, 061, 064, 208, 211)
- Output check and redo — `src/outcheck.rs:sample_ranges`, `compare_range_args`, `counts_match`, `judge`; `src/render/args.rs:check_range_graph`, `expected_frames`; `src/export.rs:Attempt::check`, `strict_decode`, `check_ranges`, `hw_choice`; tests `outcheck/tests.rs`, `export/hw_tests.rs`, `hw.rs` (REQ-055, 061..063, 206, 218, 219, 315 measured)
- Decoding — `src/hwdec.rs:Decoder`, `probe`, `choose`, `apply`; tests `hwdec/tests.rs` (REQ-214)

## Interface
- Layout — `src/ui/layout.rs:arrange`, `Arrangement`, `applied_width`; `src/ui/view_menu.rs`; `src/ui/place.rs:window_to_save`, `default_place`; `src/settings/interface.rs:Interface`; tests `ui/interface_ux_tests.rs` (REQ-024, 066, 067, 234..241)
- Actions, menus, shortcuts — `src/ui/actions.rs:ACTIONS`, `ActionId`; `src/ui/menu.rs`, `toolbar.rs:fixed`, `keymap.rs:capture`, `action_for` (REQ-025, 026, 028, 069..071, 106, 194, 195, 242, 247)
- Theme, texts, accessibility — `src/ui/theme.rs`, `src/ui/strings.rs:tr!` (REQ-243..246)
- Timeline view — `src/ui/timeline.rs` (`edge_strips`, `marker_drags`), `markers.rs`, `ruler.rs`, `overlap_lane.rs` (REQ-077, 083, 084, 093, 191..193)
- Popups and dialogs — `src/ui/popups.rs` (`clip_aspect_rows`), `modals.rs`, `settings.rs` (`INTEGRATED_NOTE`, `REPEATING`, graphics card), `status.rs`; `src/dialogs.rs` (native, rfd), `src/app.rs:Dialogs` (faked in tests)

## videolore player
- `player/src/lib.rs:real_app`; `player/src/app.rs:PlayerState`, `Command`; `player/src/ui.rs:PlayerApp`, `audio_menu`; `PlayerState::audio_tracks`, `choose_audio` (REQ-338); `player/src/subs.rs:Subs`, `Source::Picture`; `player/src/fullscreen.rs`; `player/src/settings.rs`, `player/src/playlist.rs`; tests `player/src/tests.rs`, `player/src/ui_tests.rs`, `player/tests/ffmpeg.rs` (REQ-138..142, 150, 248..256, 338)

## Packaging, build and quality
- `src/packaging.rs:release_assets`, `parse_lock`, `ffmpeg_pin` (tests of `build.yml`, installers, NSIS pin, launcher; REQ-030, 040..042, 138, 225..229, 231); `scripts/package.sh`, `fetch-ffmpeg.sh`, `notices.sh`, `icons.sh`, `version.sh`; `.github/workflows/{build,installer-config,advisories,pages}.yml`; `tests/page.rs` (REQ-298, 299)
- Quality items — `src/ui/tests.rs:sampled` (QG-18), `src/render/golden_tests.rs` (QG-15), `build.rs` (QG-17), `tests/ffmpeg_integration.rs:assert_pinned` (QG-08)
