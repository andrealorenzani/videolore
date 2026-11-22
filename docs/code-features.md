# Code — feature notes (2.0.4)

Back to the [index](code.md). How the features of 2.0.4 (REQ-180 to REQ-299, QG-14 to QG-18) are built, by area. Where each symbol and test is: [code-index.md](code-index.md) or `scripts/req-code.sh REQ-NNN`. Later features: [code-layers.md](code-layers.md); measurements: [code-measurements.md](code-measurements.md).

## Code structure
- `src/render.rs` is a hub over private `src/render/` files: `segments` (what each clip contributes), `inputs` (geometry, runs of one file, the music bed), `graph` (the one `filter_complex`), `plan` (stream copy or re-encode, audio-only), `args` (export, preview, still and check argv, `FrameKey`), `overlay`, `steps`; re-exported with `pub use self::x::*` (one public path per item). Tests in `render/tests.rs`, `render/golden_tests.rs`.
- `src/app.rs` keeps the state and the dispatcher; the command arms are in `app/dispatch_<area>.rs` (see [code.md](code.md) § How to add an effect or a command); `is_enabled`/`disabled_reason` in `app/enabled.rs`, `tick` in `app/tick.rs`.
- QG-16: `text.rs` is split into `text/timing.rs`, `text/layout.rs`, `text/events.rs`; one box rule, `placement::padded_line_rect`, used by `placement::line_rect` and the preview hit boxes (`ui/placement.rs:line_box`, sharing `line_font`, `line_wrap`).

## Subtitle files and fonts (REQ-180 to 188), project format 11
- **Formats.** `srt::Format` by extension (`.vtt`, `.ass`, else `.srt`); `Command::ExportSrt` keeps its name. WebVTT: SRT cues under `WEBVTT`, a point before the milliseconds, `& < >` as entities. `.ass` is `ass::document` for the project's frame.
- **Separate track.** `FormatChoice.subtitles` (`SubtitleMode`) → `plan_export_choice`: `Separate` carries a `SubtitleTrack` (SRT, WebVTT for WebM) written in the work folder as a third input; argv `-c:s mov_text|srt|webvtt`, `-metadata:s:s:0 language=<iso 639-2>`, `-disposition:s:0 0`. Lines leave `subs.ass` (sphere-fixed ones stay burned); forces a re-encode. MP4 marks the first track of a kind enabled whatever `-disposition` says: `mp4tracks::disable_subtitle_tracks` clears that byte in place before publishing.
- **Import.** `srt::read_file` (≤ 16 MiB, UTF-8 with or without BOM). Target: the clip's source range or every used part of the file; `replace_range`, one undo step "Import subtitles"; warns before replacing hand-edited lines. "Re-time from the speech" goes through creation (`CreateRequest.import`, `stt::Mode::Retime`); `align::retime` moves a matched line to its first/last found word. `subs_import::sibling_file` (same-name file) is looked for when the Subtitles popup opens, after an import and on open; never imported without a click.
- **Changed media.** `MediaItem.stat` (size, mtime; format 11 `MediaEntry.file_size`, `modified_ms`); `find_changed_media` on open → re-probe (`TrackCheck.probed`), project not unsaved, status line names the files.
- **Fonts.** Noto Sans Arabic Bold and Devanagari Bold compiled in (`assets/fonts`, `SOURCE.txt`). libass does not look in the fonts folder for a missing glyph: `ass::escape_scripts` wraps each run in `{\fnNoto Sans Arabic}` … `{\fnNoto Sans}`; `export::write_subtitles` copies the three faces. No CJK font: `fonts::coverage_of` (skrifa, on the scan thread) and `missing_note_in` give "No installed font shows … text"; `ui::scripts::install_for` adds UI fallbacks.

## Editing gestures, autosave, safety (REQ-189 to 199)
- **Autosave.** `autosave` writes `<cache>/autosave/<sha-256 prefix>.vlproj` (+ `.path`; `session-<hex>` if never saved) from a clone on a worker thread, every ≤ 2 min, only with unsaved changes; Save, Discard, close and open remove it. `find_recoveries` at start: Recover (opens as unsaved), Discard, Cancel. Strict loader; `.path` bounded.
- **Gestures.** One command on release: `TrimEdge` (whole frames), `RetimeSubtitle` (`subtitles::dragged_edge`, 0.1 s steps), `MoveFade` (`Timeline::move_fade`). `[`/`]` = `PrevMarker`/`NextMarker` (`app/stops.rs`). Delete with lines selected = `DeleteSubtitles`; `ShiftLines` by 0.5 s.
- **Paste.** Copy/Cut put `CLIPBOARD_MARKER` on the system clipboard and `Event::PasteImage` is bound to Paste (egui sends nothing for an empty clipboard).
- `History::forget` drops snapshots of a removed file. `placement::snap` (centres, thirds, other boxes, within 1%); `snap_to_guides`, `show_safe_areas` are program settings.
- Keyframes: 12 px markers, 8 px pick, typed time (`MoveKey`), Left/Right with the bar focused (`bar_focus_id`), Copy/Paste keyframes (`KeysClip`, clip time). A LUT chosen again for the same file changes nothing and says so.

## Hardware export and decoding (REQ-206 to 219)
Detail: [code-hardware.md](code-hardware.md).
- **Short output check (206).** When frame counts match (`render::expected_frames`, `outcheck::counts_match` ±1), only one-second ranges around the three samples are rendered (`outcheck::sample_ranges`, `render::check_range_graph`, `outcheck::compare_range_args`, cut at `RANGE_LEN`); each range has its own work folder. A non-whole-frame clip shifts later frames: then (and on a mismatch) the exact whole-graph check runs (`render::check_graph`, `compare_args`, `judge`). A faulty file never passes.
- **Quieter check (208).** `AppState::pause_or_resume_hw_probe` (in `tick`) stops the check while an export or subtitle job runs; `hw_paused`. One process at a time.
- **Busy or hanging (210).** `hwenc::timed_run` watches stdout and `Process::cpu_ms` (`machine::process_cpu_ms`): output or 20 ms CPU = `Activity::Busy` (retry), neither = `Idle` (`Reason::Hanging`), no reading = `Unknown` (retry).
- **Session memory (209).** Hanging or out of time twice → `HwStatus.slow` → `AppState.hw_slow` (never saved) → `skip` (`Reason::SkippedSlow`). Repeat, "Try video hardware again" and "Use recommended" retry everything.
- `ui::settings::INTEGRATED_NOTE` (`Candidate::graphics`: Quick Sync, VideoToolbox integrated; NVENC, AMF dedicated; VA-API by `/sys/class/drm/<node>/device/vendor`); `REPEATING` while `hw_repeating`.
- **AV1, VP9 (212, 213).** `hwenc::Codec::{Av1, Vp9}` (no profile); `format::VIDEO_PRESETS` 21 entries (AV1 in MP4: 1080p25/30, 720p25, 4K25, 480p25; VP9 in WebM: 1080p25/30, 720p25, 480p25); `preset_settings` is `None` without libsvtav1/libaom-av1, libvpx-vp9 or libopus. `hwenc::preset_bitrate` (8 Mb/s for H.264 1080p25 by pixel rate; HEVC 0.6, VP9 0.65, AV1 0.5). libvpx-vp9 `-crf 32 -b:v 0`.
- **Quick Sync constant quality (215).** `-global_quality <qp>`, the VA-API size guard.
- **Hardware decoding (214).** `hwdec::Decoder` (`-hwaccel cuda|qsv|d3d11va|vaapi|videotoolbox`, `-hwaccel_device`), `probe` (15 frames, no fallback words), `apply`; runs after the encoder probe (`HwState.decoders`); only with Faster (`Preview::set_decoders`, `AppState::apply_hw_decoders`). A bad run sends a sentinel; `Preview::take_hw_fallback` restarts at the playhead without decoders, silently. `ProxyTask.hwaccel` retries once without.
- **Graphics card (216).** `HwStatus.working` (best first, `devices()`), `Settings.graphics_card` (`None` = Automatic) compared only with those names (`HwStatus::chosen`); `SetGraphicsCard` also picks the decoder (`hwdec::choose`); cleared when the machine changes.
- **Old libva (217).** `Platform.libva_version` from the library file name; `HwStatus.libva_note`. 360 and frame effects (218, 219): the check graph is the export graph.

## Export and preview speed (REQ-220 to 224)
- `VideoPreset::label` plain codec name, `tooltip` the encoder (220). `frame::over_cap`, `AppState::frame_over_cap`, `ui::export_dialog::over_cap_warning` (221).
- Blurred fill at an eighth of each side: `frame::BLUR_DIVISOR` 8 (REQ-222 says a quarter; measured, see [code-measurements.md](code-measurements.md)).
- **Kept background (223).** `render::Background::{Normal, Only, Cached}` in `build_graph_bg`; `FrameKey::background_name` (hash of source frame, size, shape, fit, LUT; none for colour fill, 360, fade); `preview::render_with_kept_background` writes `bg-<hash>.rgba` once in the still folder; `background_renders`, `still_renders` count. Playback not cached.
- **Queue (224).** Import analysis (`probe_pool`) and the track check at `Priority::Background`; preview and export outrank the hardware check, which outranks them.

## Platforms and installers (REQ-225 to 233)
- CI: reusable `installer-config.yml` (job `package-config`, needed by executables and installers); a step runs `cargo test … cpu_message` and fails if no test ran (227); `advisories.yml` runs `cargo deny check advisories` every Monday.
- `packaging/tools.lock` `[nsis]` (3.09 + SHA-256); `scripts/package.sh tool NAME` (`VIDEOLORE_TOOLS_LOCK`); the Windows licence file is the notices with a UTF-8 BOM (`dist/stage-…/third-party-licence.txt`). Linux archive: `<name>.sh` (`packaging/linux/launcher.sh`), `<name>.desktop` (`Exec` via `%k`).
- **One window (230).** `instance` (Windows, Linux): `<cache>/instance/running.lock` (`File::try_lock`), heartbeat 250 ms, `inbox/`; `desktop::take_open_requests` adds `instance::poll_current`; `main.rs` calls `instance::start`.
- **macOS menu (232).** `desktop::macos_menu` data (Hide, Hide Others, Show All; Minimize, Zoom; no Quit so Cmd+Q reaches the unsaved prompt); `desktop::macos::install_menu` with `objc2-app-kit` (one `unsafe` call with a SAFETY comment). Wayland drops (233): no code.

## Interface (REQ-234 to 247)
- `src/settings/interface.rs` (`Interface`, lenient `from_value`): window, popup places, sections, workspaces (≤ 10, names 1-40 chars), toolbar, shortcuts, theme, text size.
- `ui/place.rs`: `window_to_save`/`viewport_of` (saved 0.5 s after it stops changing; off-screen replaced), `saved`, `default_place`, `fit_width` (≥ 1280x800: popups right of or above the timeline).
- `ui/layout.rs` `arrange` + `Arrangement` (hide, collapse to 120 px, dock); `section_menu` in `ui/mod.rs`; `ui/view_menu.rs`; commands in `app/dispatch_interface.rs`.
- `toolbar::fixed` (user list or default 12, icon-only, screen-reader names). `ui/keymap.rs`: overrides stored as "Cmd+Shift+B", `install`, `capture` (`RebindConflict`). `ui/theme.rs` (contrast ≥ 4.5:1). `ui/strings.rs`: `Key` + English table + `tr!(Name)`; a test scans `src/ui/*.rs` for literal texts (new key = new variant at the end of both lists). eframe `accesskit` on; tests read the tree. Edit menu: Clipboard, History, Clip, Subtitles, Settings….

## Video player (REQ-248 to 256)
- Associations: `scripts/package.sh` player `file-associations` (viewer role: mp4, m4v, mov, mkv, webm, avi), AppImage `MimeType`; `packaging/windows-player.nsi` registers Open with only (`OpenWithProgids`).
- `src/siblings.rs` (fixed names next to the program, one argument, no shell): `Command::OpenInPlayer`, player `OpenInEditor`; a video given to the editor is imported into a new project.
- `PlayerState.placed_memory`; `player/src/settings.rs` (`player.json`); `player/src/playlist.rs` (session only, folders ≤ 10 000, name order); `notices::components_player`, `third-party-player.txt`. Preview copies always map audio; `PARAMS_VERSION` 2.
- **Picture subtitles (253).** `MediaFormat.picture_tracks` (`media::PictureTrack`: ordinal, `pgs`/`dvd`/`dvb`, language, title). `Project.picture_sub` (never saved) → `Segment.picture_sub` (not 360): `[k:v:0][k:s:N]overlay=eof_action=pass:format=auto[psI]` before the trim (`split` when several clips use it). Player `Source::Picture`, "(picture)". Limits: a subtitle already on screen at a seek shows at its next change. Fixture: hand-written PGS in `player/tests/ffmpeg.rs:pgs`.

## 360 video (REQ-257 to 273)
- **Marker (261, 262).** `spherical::mark_matroska_with`: the Projection element grows Tracks and the Void padding shrinks by the same bytes (in place; only SeekHead entries change). Initial view (269): `media::spherical_pose`, `spherical::read_pose` (Spherical V1 XML), `camera::start_camera`, `render/segments.rs:start_path`.
- `MediaFormat.marked_360` (2:1 only, `SetMarked360`, not an undo step); `choice::video_field_reason`, `choice::sphere_frame_note`; the 360 plan drops frame effects. `camera::command_lines_capped` (`GraphSpec.camera_updates` 10 in export, 0 in preview); `Preview.draft` halves the still while the camera moves (`CAMERA_RESTART`).
- `Camera.roll_cd`, `Ease`, Shift+drag `RollCamera`; keyboard (272): `keyboard_camera_id`, arrows 5°, Page Up/Down zoom, Enter makes a POV.
- **360 text (257 to 260).** Anchored lines drawn as text items (`segments::anchored_line_items`), karaoke highlight (`TextItem.karaoke`, `ass::highlight_chunks`); generated lines anchored at the view centre (`AppState::generation_anchors`); `Command::ReAnchor`; off-view arrows (`ui/placement.rs:off_view_arrow`, `ui/picture.rs:off_view_arrows`).
- **Camera paths per clip (268).** `CameraPaths.clips`, copy on write (`set_in_clip`, `edit_in_clip`, `remove_in_clip`), `AppState::clip_for`; copied, pasted (`clipboard_camera`) or split clips get a copy.
- **Spatial sound (273).** `src/spatial.rs`: 4 channels + 360 picture = first-order ambisonics (ACN W, Y, Z, X — an assumption). `rotate` by yaw/pitch, `cardioid_pair` (`0.5 * (W ± Y)`), `Rotator` every 10 ms (roll not applied). Pre-pass `spatial::prepare` (ffmpeg via the `Runner`, raw float pipe) writes `spatial-<hash>.wav` in the work folder (`.part` then rename); `Segment.spatial`, `Graph.spatial`; export `export::prepare_spatial` (360 video export keeps the recorded sound); preview `Preview::prepare_spatial` before the audio process (`Preview.prepass`), clips over 10 min keep recorded sound.

## Effects, frame, LUT (REQ-274 to 294)
- Fades: `Timeline.fade_length` (0 = 0.5 s), `fade_sides` (`FadeSide`), `fades_of`. Music clips: `MusicClip.volume/muted/fade_in/fade_out/clean_audio`, `BedPart::Clip`.
- Clean audio: `Clip.clean_strength` (100 as before, 0 bypass, between = `asplit`+`amix`); "Normalise speech" = `speechnorm` (`render::NORMALISE_SPEECH`).
- Keyframe easing: `Key.ease`, `keyframes::set_ease`, smoothstep in the expression. `BlurLevel`, `Shape::Custom`.
- LUT: `Lut::with_strength` (blend with identity), `.3dl`/`.csp` readers, domain scaling, `Modal::MissingLut` (Find, Export without, Cancel), `ProjectFile.lut_copies` (20 MB / 64 MB, project 96 MiB), favourites.
- Per-clip Change aspect (286, 288): `Timeline.clip_aspects`, the clip made in its own frame and padded into the project frame.
- Volume keys (280): `Timeline.clip_volume`, `frame::volume_expression` in `volume=...:eval=frame`. Crop (283) was removed in 2.3.0.
- **Speed (281).** `Clip.speed` in thousandths (500-2000, saved when not 1000). `Clip::len()` is the timeline length, `src_len()` the source part; `to_timeline`, `to_src`, `src_at`; keys stay in source time (`Clip::keys_to_timeline`); `Segment.speed`, `setpts=(PTS-STARTPTS)*1000/speed` + one `atempo`; `camera::command_lines_speed`; never stream copied; `DIFFERENT_SPEEDS` blocks a merge.
- **Picture items (287).** A `TextItem` with `picture: Option<Box<Picture>>` (`src/picture.rs`, PNG size from the header); no ASS events: `render::picture_overlays` → `PicOverlay`, `build_graph_pics` (looped still `-loop 1`, `overlay` with `enable='between(t,..)'`). A missing file stops an export (`RenderError::Picture`), is skipped in a preview. Since 2.3.0 the interface offers Overlap media instead.

## Quality, cache, page (REQ-295 to 299, QG-14 to 18)
- `splash::should_show`, `AppState::start_splash`. `ProxyCache::enforce_limit` (oldest first, own files, protected = open project's copies). Page: `og-card.png`, a CSP with the hash of the one style block (`tests/page.rs`; regenerate the hash when the style changes).
- QG-14: speech tests use `tests/fixtures/align/speech.mkv` (no synthesiser in CI); 360 test sphere 1024 x 512. QG-15: pointer tests (colour picker, Export dropdowns, keyframe bar), golden argv of formats 4 and 5 (`project-v4-1.2.6.vlproj`, `project-v5-1.3.0.vlproj`, `export-argv-v4-v5-in-2.0.4.txt`), large-picture memory test. QG-17: `build.rs` explains a missing splash summary; `scripts/version.sh set` runs the notices check. QG-18: the four UI frame gates are `sampled` (best of ≤ 5 runs); probe fakes have 200 ms limits; a test checks the font notices.
