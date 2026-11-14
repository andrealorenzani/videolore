# ai-notes: code

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

## State (0.1.5 target: toolchain/deps upgrade)
0.1.5 upgrade: MSRV 1.100 (policy: follow latest stable, raise rust-version with toolchain, deps always newest). Coverage 96.0% lines on rustc 1.100. 71 unit + 7 integration.

## State (0.1.3)
Implemented REQ-001..013, 021..025. Planned: REQ-014..018 (subs/STT, 0.1.5, feature `stt` reserved empty), REQ-019..020 (music, 0.1.4).
Coverage at 0.1.3: 96.4% lines (unit-only 96.4%; integration adds little). 71 unit + 7 integration tests.

## Deps (rustc 1.100; requirements pinned to latest minor in Cargo.toml)
eframe 0.37.0 (rust-version 1.95; no default feats: glow, default_fonts, wayland, x11; no clipboard feat, same as before).
rfd 0.18.0 (no default, xdg-portal). cpal 0.18.2 optional behind feature `audio` (default). serde/serde_json, thiserror 2, directories 6, tempfile 3 (normal dep: private temp dirs for concat list/graph script/preview script). Dev: sha2 0.11.
egui_kittest NOT used (0.37 available): headless `ctx.run_ui(RawInput{screen_rect,..}, |ui| frame_ui(ui,..)).drop_without_applying_deltas()` is enough.
Linux build needs libasound2-dev (alsa-sys via pkg-config). Only libasound is linked; winit/glutin/wayland/xkb dlopen'd -> no other -dev pkgs. Local machine lacked it: used extracted .deb + PKG_CONFIG_PATH in scratchpad (not repo).

## Module map
- time: Micros(i64, serde transparent, saturating add/sub, ff()->"s.uuuuuu"), Fps{num,den} (parse "a/b", frame_index/time/snap, cmp_value), timecode/parse_timecode.
- media: MediaId, MediaFormat{container normalized(mp4|mov|matroska|webm|first token), vcodec, acodec, w,h,fps, bit_rate(video stream else container), audio_bit_rate, sample_rate, channels}, identity() tuple for REQ-022 equality, parse_ffprobe(serde_json::Value, defensive; attached_pic skipped; missing fps -> 30), probe(runner, ffprobe, path, id).
- timeline: Clip{id,media,src_in,src_out}; Timeline{clips Vec contiguous, fades BTreeSet<(left,right)>, next_clip_id}. prune_fades after ops; split renames (A,next)->(B,next); merge renames (B,next)->(A,next). insert_at splits clip under t (or snaps to nearest boundary when < min_len from edge). fades_of(i)->(in,out) lens. fade len = min(500ms, len/2).
- history: History<T> snapshots, cap 100, record() clears redo. Snapshot = Timeline only (imports not undoable).
- project: Project{media, timeline, next_media_id}. File: schema_version, next_media_id, media[{id,path_abs,path_rel,name,duration,format}], timeline. Load: 16 MiB limit, schema check first via Value, validate dup ids / clip media refs / ranges within media duration / fades refs; ids repaired upward; path_abs then dir.join(path_rel); missing -> MediaItem.missing (serde skip) + names list. write_atomic: `.<name>.tmp-<pid>` + sync_all + rename + dir fsync (unix).
- format: all_identical, choose_output (min_by_key (pixels, fps, bitrate None=MAX) -> first wins ties = timeline order), encode_settings (h264 libx264 -preset veryfast; hevc libx265; vp9 libvpx-vp9 -deadline good -cpu-used 5; av1 libsvtav1|libaom-av1; mpeg4; audio aac/libopus/libmp3lame; webm needs vpx/av1 + opus; else fallback H.264/AAC/MP4). Dimensions forced even.
- render: Segment{path,has_audio,src_in,len,skip,fade_in,fade_out}; segments(project, from). EACH CLIP = OWN INPUT `-protocol_whitelist file,pipe -ss <src_in+skip> -t <rendered+1s> -i file:<p>`, graph refs [i:v:0]/[i:a:0]. Video chain: trim=duration -> setpts=PTS-STARTPTS+skip/TB -> scale decrease -> pad center black -> setsar=1 -> fps -> format -> fade in st=0 / out st=len-d -> setpts reset. (+skip/TB trick keeps fades right when preview starts mid-clip.) Audio: atrim -> asetpts -> aresample 48k -> aformat fltp stereo -> apad whole_dur -> atrim -> asetpts +skip -> afade -> reset; no audio -> anullsrc+atrim. concat n=N v a. plan_export: StreamCopy iff identical && no fades && all src_in==0||keyframe (closure, lazy). StreamCopy list "ffconcat version 1.0", `file 'file:<p>'` with ' -> '\'' , inpoint/outpoint; argv `-f concat -safe 0 -i file:<list> -map 0:v:0 -map 0:a? -c copy`. Reencode: -b:v/-maxrate = chosen bitrate, bufsize 2x, -pix_fmt yuv420p, -ar 48000, +faststart for mp4/mov. Always `-progress pipe:1 -nostats` + `file:<partial>` last. preview_video_args (rgba rawvideo pipe:1), preview_audio_args (s16le, device rate/channels).
- ffmpeg: Runner trait {run, spawn->Box<dyn Process>{take_stdout, kill, wait}}; SystemRunner (stdin null, stderr drained in thread, tail 64 KiB, CREATE_NO_WINDOW on windows, kill on Drop). locate(base, setting, env, exe_dir, PATH). parse_version (N-/git -> Dev = new), parse_filters (tok[2] contains "->"), parse_encoders (6-char flags). check(): >=4.4, REQUIRED_FILTERS, encoders, ffprobe runs. parse_progress_line (out_time_us|out_time_ms, progress=end). parse_keyframes/probe_keyframes (-skip_frame nokey frame=pts_time csv). filter_graph_args: >8 KiB -> script file (-filter_complex_script <7.1, -/filter_complex >=7.1). `fake` module (cfg(test)): FakeRunner with responders (prog, args)->Option<CmdOutput>; FakeProcess stdout = Cursor.
- export: ExportJob (owned snapshot), validate_output (canonical; refuses source == out or == partial; parent must exist), partial_path `.<name>.videolore-partial`, plan() (keyframes cached per media, tolerance half frame), run() sync (testable), start() thread + mpsc; events Started{stream_copy}, Progress, Done, Failed, Cancelled. Cancel = AtomicBool checked per progress line (ffmpeg prints every 0.5 s) -> kill. Partial always removed unless renamed. Handle Drop -> cancel. ExportHandle::from_events for tests (AppState::wait_export).
- preview: AudioBuffer (Mutex<VecDeque<i16>>, played frames counter), AudioSink trait (format, play(buf)->bool, stop), NullSink, cpal CpalSink (feature audio; f32/i16/u16/i32), default_sink(). Preview: state Paused|Playing; position = start + played/rate (audio) or wall clock; video worker thread read_exact w*h*4 -> sync_channel(8); still frame mode for paused seek (first frame then drop worker); `pending` slot for not-yet-due frame; end detection on Disconnected. Worker Drop kills process (Arc<Mutex<Box<dyn Process>>>).
- app: Command enum (Pick* variants go through Dialogs trait -> concrete command). AppState::dispatch -> dispatch_inner; errors also into status. is_enabled gates UI + shortcuts. edit() = apply on clone, record, replace_timeline (dirty, drop stale selection, invalidate preview, re-seek -> still frame). target_clip = selection else clip under playhead. Settings {ffmpeg, ffprobe} json in directories config dir. RetryFfmpeg rebuilds Preview.
- ui: frame_ui(ctx, app, cache, now): tick, texture update (serial), title, keymap::collect unless modal/text focus, CentralPanel NONE + layout::panels + scope_builder(max_rect) + Frame::group; panels push Commands; modals (ffmpeg missing: choose/retry/quit; notice). Timeline: ScrollArea horizontal, background click seek, clip click select+seek, drag_stopped -> MoveClip(drop_index), boundary circles toggle fade, fade triangles, playhead red. Zoom px/s: range(fit=width/dur, max=12*fps), log slider via to_slider/from_slider; app.zoom 0 => fit set on first frame. Trim UI: ±1 frame buttons and "at playhead" (no DragValue: avoids undo spam).
- keymap: Cmd/Ctrl+B split, X/C/V (egui emits Event::Cut/Copy/Paste -> handled), Z undo, Shift+Z / Y redo, Space play. Modifiers.command = Cmd on mac.
- dialogs: NativeDialogs (rfd) — untested glue (0% cov, ~27 lines). lib.rs real_app/native_options.
- main.rs: eframe::run_native only; windows_subsystem attr on release.

## Gotchas
- egui 0.33: screen_rect deprecated -> content_rect; ui.close() not close_menu; Modal::new(id).show(ctx,..).
- egui/eframe 0.37: App::update removed -> `fn ui(&mut self, ui: &mut Ui, frame)`; frame_ui takes the root `&mut Ui` (ctx = ui.ctx().clone()); CentralPanel::show(ui,..) (show_inside deprecated); Ui::id -> scope_id; Id::new deprecated -> Id::unique; ctx.wants_keyboard_input -> egui_wants_keyboard_input; Context::run removed -> run_ui/run_pass; FullOutput with texture deltas PANICS on drop (debug_assert) -> tests call drop_without_applying_deltas(). Stroke::new width is plain f32 (explicit `_f32` literals kept anyway: rustc float_literal_f32_fallback future-incompat lint hit `impl Into<f32>` in 0.33).
- clippy 1.100: chunks_exact_to_as_chunks -> `as_chunks::<2>()` (preview audio + integration test); needless_range_loop in nearest_boundary -> iterate `clips[..len-1]`.
- Coverage gotcha: cargo 1.100 moved test binaries (target/llvm-cov-target/debug/build/<pkg>/<hash>/out/); stale binaries in debug/deps/ from older cargo are still picked by cargo-llvm-cov 0.9.1 and `llvm-cov clean` does not remove them -> mismatched mappings, coverage fell to 73%. Fix: rm -rf target/llvm-cov-target.
- FakeRunner::holding_streams(): FakeProcess stdout blocks after scripted bytes until kill() (Condvar). Used by app play_pause_and_tick, which was flaky: empty fake video stream -> Disconnected -> end-of-playback race with the tick.
- let-chains used (stable since 1.88, edition 2024).
- CI: check job (stable, clippy x3 feature sets, --locked), msrv job (sed rust-version from Cargo.toml -> dtolnay/rust-toolchain@master toolchain=<v>, build+test --locked, ffmpeg tests required), build needs [check, msrv]. Actions: checkout@v7, upload-artifact@v7, action-gh-release@v3, rust-cache@v2, install-action@v2 (latest majors on 2026-11-14).
- cpal 0.18: SampleRate = u32, StreamConfig is Copy, build_output_stream(config by value, data, err, None).
- Integration tests: fixtures generated with lavfi; letterbox check uses 480x360 (4:3) vs 1280x720 so bars are visible. Cancel test uses 15x4 s 720p + fade to make export slow enough.
- `system_runner_runs_and_kills` unit test uses `rustc` from PATH as a harmless program.
- Paste inside a clip splits it (insert edit semantics).

## Debt / open
- No "unsaved changes?" prompt on New/Open/close.
- Import probes synchronously on the UI thread.
- Every edit respawns a preview ffmpeg for the still frame.
- Preview audio clock: if audio stalls, video waits (no wall-clock fallback mid-run).
- HEVC in mp4 not tagged hvc1.
- Export uses one input per clip (-ss); many clips of one long source = many decoders.
