# ai-notes: architecture

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

target: 0.2.0 | MSRV 1.100 (ADR-012) | toolchain: latest stable | ed 2024 | GPL-3.0-only
scope 0.2.0: resolve all 0.1.3 known limitations + REQ-026..044 + REQ-014..020 implemented (music, subtitles/STT) + bundled ffmpeg (C1 reversed) + installers.
exact crate versions: Cargo.toml/Cargo.lock (PE owns) = source of truth. versions below = verified at plan time (2026-11-14), not pins.
full plan archived in session scratch arch-0.2.0.md (not durable); this file = durable summary.
human docs split (0.2.0): docs/architecture.md = index; -ffmpeg, -packaging, -media-pipeline, -subtitles, -security, -dependencies.

## component map (src/ modules)
- time: µs/rational time, ts parse/format, no floats in cut math. pure.
- media: MediaInfo from ffprobe json; 0.2.0: MediaStatus{Analysing,Ready,Rejected(reason)}; audio-only accepted, kind() Video/Audio. probe on probe pool.
- timeline: video track; clips, cuts/splits, fades, invariants. pure. semantics unchanged.
- music (NEW): MusicTrack{clips: Vec<MusicClip{id,media,src_in,src_out,start}>, next_id}; sorted, non-overlap; place/move/trim/split/delete. pure. audio-only media only.
- subtitles (NEW): Subtitles{language:Option<String>, language_overridden, preset:PresetId, lines: Vec<Line{id,media,start,end,text,words:Vec<Word{start,end,text}>,edited}>, next_id}. times = SOURCE media time (REQ-018 free). project(&Timeline)->Vec<ShownLine> pure projection; edit ops; presets.
- ass (NEW): pure ASS writer: presets, size formula, \k karaoke, escape.
- history: snapshot Edit{timeline,music,subtitles,mix}; Mix{video_volume:u8,video_muted,music_volume:u8,music_muted}. cap 100.
- project: schema v2 + v1 migration + TooNew; ProjectDoc (persisted model) for dirty tracking; Save/Save As. pure+fs.
- format: output format/encoder table (unchanged).
- render: ExportPlan; 0.2.0: monotone-run input grouping, music graph, subtitle filter, -tag:v hvc1, proxy substitution (Source::Preview only), single-segment still graph, FrameKey. pure.
- ffmpeg: locate (bundled), version/filters/encoders probe, Runner trait spawn(+cwd)/run (stdout cap 16 MiB), progress parse, kill, stderr ring 64 KiB. only module spawning ffmpeg.
- jobs (NEW): Scheduler (cap 3), Permit, LimitedRunner, spawn_group, preemption, cancel. wraps Runner -> every ffmpeg/ffprobe spawn goes through it.
- export: worker w/ Export-priority runner; writes ASS+font into private tempdir used as cwd; temp file + atomic rename (unchanged).
- preview: wall-clock master + audio drift correction; latest-only still mailbox + 80 ms debounce; music+subs in graph; proxies.
- proxy (NEW): rule, key, cache dir, bg generation + progress, clear, size.
- stt (NEW): Transcriber + ModelFetcher traits + fakes, extraction job, chunking, word ts, lang detect/override. WhisperTranscriber + UreqFetcher cfg(feature="stt").
- machine (NEW): MachineInfo{cores, ram_bytes} (std + sysinfo), stt_threads().
- notices (NEW): static table of bundled components + generated rust crate notices (include_str!).
- desktop (NEW): linux self-integration (menu entry, icon, mime) + removal; macos odoc Apple Event handler.
- ui: thin egui panels. app: AppState::dispatch, tick() drains channels, settings.
- main.rs: run_native (+ macOS: event_loop_builder default menu off, odoc observer before run_native; argv[1] open).
data flow:
- UI ev -> AppState::dispatch -> mutate Edit -> history -> recompute dirty -> FrameKey@playhead -> if != shown_key post still req to mailbox after 80 ms debounce (injected now).
- import: Command::Import(paths) -> placeholder Analysing -> probe pool (3 thr, Probe prio) -> mpsc -> tick() Ready/Rejected -> proxy rule may enqueue proxy job.
- export: render::plan_export(project, Source::Export) (originals) -> export thread, Export runner (1 ffmpeg) -> progress -> rename.
- subtitles: per timeline source w/ audio: extract 16 kHz mono f32 temp file (Background, 1 slot, released after extraction) -> STT thread (own thread budget, outside cap) -> 120 s chunks -> Transcript -> lines in source time. edited lines -> confirm first (REQ-044).
- open by double-click: Win/Linux argv[1] -> Command::OpenPath; macOS odoc -> mpsc -> tick -> OpenPath. both through REQ-028 prompt.

## ADRs
ADR-001 GUI. eframe/egui latest (0.37 at 0.1.5). alt iced/slint/gtk4-rs/tauri. reason: pure Rust, single binary, MIT/Apache. [MSRV reasoning superseded by ADR-012]
ADR-002 media engine. ffmpeg/ffprobe CLI subprocess. alt ffmpeg-next/sys (link pain, ABI churn, static licence linkage), gstreamer-rs, pure-Rust decoders. reason: zero build deps, robust.
  rules (kept): Command no shell; `file:` prefix; `-protocol_whitelist file,pipe`; `-nostdin -hide_banner`, -y only on our temp output.
  ["not bundled" + "any ffmpeg >= 4.4 required" SUPERSEDED by ADR-013 in 0.2.0]. user override kept.
ADR-003 preview audio cpal; null sink fallback. [clock part superseded by ADR-016]
ADR-004 concurrency std::thread + mpsc + Mutex/Condvar; no tokio. cancel AtomicBool + kill. [process cap: ADR-015]
ADR-005 support crates serde/serde_json/thiserror/directories/rfd/tempfile. MIT/Apache.
ADR-006 export tiers. T1 stream copy only if same src format AND compatible output AND no effects AND keyframe cuts. 0.2.0: "effects" = fades, music clips, subtitle lines, video volume != 100% or muted (REQ-023 reworded). else T2 single re-encode. temp in dest dir + rename; never overwrite source. [input layout: ADR-017]
ADR-007 encoder mapping, sw first, `-encoders` probe at startup; unavailable formats disabled.
ADR-008 fades min(0.5 s, clip/2), video+audio aligned.
ADR-009 STT engine whisper.cpp via whisper-rs; model ggml tiny-q5_1 multilingual, not bundled, download after consent. ["stt off by default" SUPERSEDED by ADR-022]. alt cloud APIs (privacy), vosk (quality), candle-whisper (perf/maturity).
ADR-010 music via ffmpeg amix/volume/afade in T2; no new crate. detailed in ADR-019.
ADR-011 CI: check job + Linux build: apt ffmpeg libasound2-dev; VIDEOLORE_REQUIRE_FFMPEG=1 in check job. 0.2.0 adds apt cmake libclang-dev espeak-ng; STT model cache; cargo-deny; notices freshness; package job (ADR-014). CI files owned by PE.
ADR-012 toolchain policy: latest stable; rust-version = current stable (1.100); fix new lints, never pin old toolchain/crates. CI latest + MSRV job.

ADR-013 (0.2.0) bundled ffmpeg. ctx: REQ-030 works w/o ffmpeg installed; C1 reversed. supersedes ADR-002 "not bundled"/">=4.4".
  choice: GPL builds, same FFmpeg release line on all OS (9.0.x at plan time; confirm at pin):
    win x86_64: BtbN/FFmpeg-Builds win64-gpl-shared (release branch; exes share DLLs ~half size of 2 static).
    linux x86_64: BtbN linux64-gpl static (release branch).
    macOS aarch64: Martin Riedl build server (ffmpeg.martin-riedl.de) release static arm64 ffmpeg+ffprobe, published SHA-256; incl x264 x265 libass freetype fontconfig harfbuzz vpx opus lame svt-av1.
  alt rejected: LGPL builds (no x264/x265 -> REQ-030 H.264/HEVC lost); build in CI (30+ min/OS, maintenance); evermeet (Intel only); Homebrew (dylibs).
  pinning: packaging/ffmpeg.lock per target: url, sha256, ffmpeg version, provider, licence, build-script src URL+commit, ffmpeg.org src tarball URL+sha256. scripts/fetch-ffmpeg.sh <target> <dest> curl + sha256sum / shasum -a 256, extract only ffmpeg/ffprobe/DLLs. BtbN keeps month-end builds 2 y -> pin month-end. Riedl retention unknown -> CI fails loudly, re-pin.
  smoke test on packaged copy: -version; -encoders has libx264 libx265 libvpx-vp9 libopus; -filters has subtitles amix alimiter.
  location: always next to main exe. win: install dir (+ *.dll), fonts/ notices/ beside. macOS: Contents/MacOS/ (bundle seal covers nested code), resources Contents/Resources/. AppImage: usr/bin/, usr/share/videolore/. tar.gz: top folder next to videolore, fonts/ notices/.
  lookup: settings > env VIDEOLORE_FFMPEG/FFPROBE > bundled_dirs(exe_dir)=[exe_dir, exe_dir/../Resources] > PATH. resources: exe_dir/{fonts,notices}, exe_dir/../Resources/..., exe_dir/../share/videolore/... "bundled damaged" (present but fails -version/caps) vs "not installed" messages distinct.
  new feature checks: subtitles amix alimiter volume split asplit.
  licence: app GPL-3.0-only, ffmpeg run as separate program (aggregation) but we distribute GPL binaries -> provide source. BtbN gpl = --enable-gpl --enable-version3 -> GPLv3+; x264/x265 GPLv2+; all compat. release asset videolore-<ver>-third-party-source.tar.xz: ffmpeg.org tarball+.asc, git archive of provider build scripts at pinned commit, Noto Sans source ref + OFL, README. notices: written offer 3 y (GPLv3 §6), H.264/HEVC patent note. ASSUMPTION: no lawyer review.
  fonts: libass in all 3 builds; bundle Noto Sans Bold (OFL-1.1, ~0.5 MB) at assets/fonts/ + OFL text + source url/sha. Latin/Greek/Cyrillic only (Andrea). others -> libass system fallback (may differ per OS/boxes); egui default fonts cannot show them in editor. documented limitation.
  size est 70–130 MB per installer (measure in CI, not gate).
  tests: CI keeps apt ffmpeg (Ubuntu build has x264/x265/libass) -> hvc1 + burn-in integration tests run; package job smoke-tests bundled copy per OS.

ADR-014 (0.2.0) installers/packaging. REQ-040/041/042. unsigned (Andrea; signing deferred).
  win x86_64: NSIS videolore-<ver>-windows-x86_64-setup.exe via cargo-packager 0.11.8 (Apache-2.0 OR MIT; last release 2025-11-27; CI-only tool, `cargo install --locked --version 0.11.8`). install_mode=currentUser (no admin, HKCU), start-menu shortcut, Apps uninstall entry, .vlproj assoc HKCU\Software\Classes via file_associations, licence page = notices. uninstaller removes install dir + our HKCU keys only, never projects.
  macOS aarch64: videolore-<ver>-macos-aarch64.dmg via cargo-packager (drag to Applications); file_associations -> CFBundleDocumentTypes + UTExportedTypeDeclarations; then `codesign --force --deep --sign -` ad-hoc reseal (nested ffmpeg consistent). no Gatekeeper signing.
  linux AppImage: videolore-<ver>-linux-x86_64.AppImage via appimagetool (AppImage/appimagetool release pinned URL+SHA-256 in packaging/tools.lock, PE picks current) on hand-built AppDir: AppRun, videolore.desktop (MimeType=application/x-videolore-project;), videolore.png, usr/bin/{videolore,ffmpeg,ffprobe}, usr/share/videolore/{fonts,notices}. static type2 runtime -> needs fusermount3 (current Ubuntu LTS + Fedora), not libfuse2. cargo-packager NOT used for AppImage (linuxdeploy, unspecified runtime).
  linux tar.gz (Andrea: also tar.gz): top folder videolore-<ver>-linux-x86_64/: videolore 0755, ffmpeg/ffprobe 0755, fonts/, notices/, videolore.png, README.txt (extract, double-click), marker .videolore-bundle. risk: some file managers don't run ELF on double-click -> README fallback (right-click Run / ./videolore once).
  linux self-integration (desktop::linux, startup, idempotent): only if $APPIMAGE set (abs existing file -> Exec target $APPIMAGE) else exe_dir/.videolore-bundle exists (target canonical current_exe()); else nothing (source/cargo run). writes ~/.local/share/applications/videolore.desktop (Exec="<target>" %f quoted per desktop-entry spec, MimeType), ~/.local/share/icons/hicolor/256x256/apps/videolore.png, ~/.local/share/mime/packages/videolore.xml (glob *.vlproj). best-effort update-mime-database + update-desktop-database via Command (abs args, no shell, errors ignored). rewrite when target moved. About (Linux): "Remove from application menu" deletes the 3 files.
  icon: original SVG assets/icon/videolore.svg; committed derived icon.png(1024) .ico .icns icon-256.rgba (include_bytes! -> ViewportBuilder::with_icon; no image crate). scripts/icons.sh dev-only. no Windows exe resource icon.
  double-click .vlproj (REQ-041 reworded): win/linux argv[1] -> OpenPath at startup; running app -> OS starts 2nd window (Andrea), no IPC, 1st untouched. macOS: Apple Event odoc (winit 0.30 doesn't expose) -> desktop::macos NSAppleEventManager kAEOpenDocuments handler via objc2 0.6.5 + objc2-foundation 0.3.2 (MIT, min features), installed from NSApplicationWillFinishLaunchingNotification observer registered in main before run_native -> mpsc -> tick -> OpenPath -> REQ-028 prompt.
  CI package job (PE): matrix 3 OS, needs [check, msrv], on v* tags + workflow_dispatch: fetch ffmpeg + smoke, cargo build --release --locked, scripts/package.sh <target>, upload, tag -> softprops/action-gh-release@v3. 5 assets: setup.exe, dmg, AppImage, tar.gz, third-party-source.tar.xz (from linux leg or small notices job). existing build job: bare exes on push/PR as artifacts, no longer attached to releases.
  alt: hand NSIS script + hdiutil (fallback if cargo-packager breaks); linuxdeploy; flatpak/snap/deb (out of scope).

ADR-015 (0.2.0) process scheduler. REQ-033. jobs::Scheduler{cap:3} Mutex<State>+Condvar.
  Priority{Preview=0, Export=1, Probe=2, Background=3}. acquire(prio, slots:u8, cancel:&AtomicBool, preempt:Option<Arc<AtomicBool>>) -> Option<Permit>: grant when in_use+slots<=cap and no earlier-ranked waiter (strict prio, FIFO within level); None on cancel. Permit drop -> release + notify_all.
  preemption: Preview/Export waiter unsatisfiable -> set preempt flag of lowest-prio running preemptible permit (proxy only) -> kill ffmpeg, drop permit, requeue, restart from 0. probes + STT extraction short, not preempted. export never.
  structural: Scheduler::runner(prio)->Arc<dyn Runner> = LimitedRunner: run() holds 1 slot for call; spawn() -> Process wrapper keeps permit until wait/kill/Drop; spawn_group(&[(prog,args);2]) 2 slots atomically. all callers get such runner -> cap unbypassable.
  slots: playback 2 (video+audio together; single NUT-muxed proc rejected: demuxer complexity); still 1; export 1 (keyframe probes sequential at Export prio before); probe 1 each (pool 3 thr); proxy 1 (1 worker, preemptible); STT extraction 1 (Background). worst: export+playback=3.
  obsolete: still frames latest-only mailbox Mutex<Option<Req>>+generation; newer replaces pending, running older gen killed. playback restart kills old pair. media removal cancels its probe/proxy. New/Open/close cancels all project jobs.
  startup ffmpeg capability check on bg thread; UI "checking ffmpeg…", import/preview/export disabled until done.
  STT inference NOT in cap (only ffmpeg/ffprobe per PM default); own thread budget.

ADR-016 (0.2.0) preview. REQ-034/035. supersedes ADR-003 "audio clock master" + risk "audio drives clock".
  FrameKey (pure, render) at t: segment media + effective path (proxy|orig), source frame idx (fps-snapped), fade params at t, output geometry, visible subtitle cues (text, preset, highlighted word idx). render only if key != shown_key after 80 ms debounce (injected now). edit after playhead -> 0 renders; 10-edit burst -> <=1.
  still graph: single-segment at playhead (1 input).
  clock: wall clock master (Instant), anchored play/seek, pause excluded. audio callback tracks device_pos (counts silence written when starved). pure audio_correction(device_pos, wall_pos)->{None,Drop(n),Pad(n)} threshold 40 ms. decoder stall -> silence then drop backlog; device stall -> corrected on resume. video never waits for audio.
  music: preview audio proc uses same render::audio_graph as export, from `from` w/ +skip trick. subtitles: same ASS+font in preview private tempdir = cwd; subtitles filter after scale; ASS PlayResX/Y = export geometry -> same look. proxies via render::segments(project, from, Source::Preview{proxies_on}) proxy if Ready else orig.
  alt rejected: audio master (video stalls with audio, the 0.1.3 limitation).

ADR-017 (0.2.0) export inputs grouped in monotone runs. REQ-037. supersedes "one input per clip".
  consecutive segments same source with src_in(i+1) >= src_out(i) share one input: -ss <run_start> -t <run_len+1s> -i file:<p>; [k:v:0]split=n / [k:a:0]asplit=n; per seg trim/atrim, setpts, existing normalisation. new input only when order goes back in time -> ffmpeg memory bounded (no branch queues ahead). 200 in-order segs from 1 src = 1 input/decoder. graph via script file when > 8 KiB (existing). music clips grouped same way.
  alt rejected: one input per clip (200 decoders); one input per source w/ split for reordered clips (unbounded frame queues).
  hvc1 (REQ-036): -tag:v hvc1 whenever video codec HEVC and muxer mp4|mov, copy and re-encode.

ADR-018 (0.2.0) proxies. REQ-038/039.
  rule pure: needs_proxy(w,h,MachineInfo{cores,ram_bytes}) = w*h > 1920*1080 || (w*h > 1280*720 && (cores < 8 || ram < 8 GiB)). display size (rotation). audio-only never. cores=available_parallelism; RAM=sysinfo (system feature) once at startup.
  format: libx264 -preset veryfast -crf 23 -g round(fps/2), fit 960x540 even dims, -fps_mode passthrough, AAC 128k stereo, .mkv timestamps from 0. <key>.mkv.partial -> rename. -progress pipe:1 -> media item.
  cache: ProjectDirs::cache_dir()/proxies/<key>.mkv; key = first 32 hex of SHA-256(abs path, size, mtime ns, PROXY_PARAMS_VERSION) (sha2 -> runtime dep). reused across sessions, recreated if missing, size in settings. clear deletes only ^[0-9a-f]{32}\.mkv(\.partial)?$ not being generated. no auto eviction. on by default; switch in settings (not project). export never uses proxies: plan_export identical on/off.
  memory caps (REQ-039, nothing scales w/ file size): preview frame chan 8x640x360 RGBA (~7 MB); audio buf 1 s; stderr 64 KiB; run() stdout 16 MiB; STT chunk 120 s x 16 kHz f32 (~7.7 MB); keyframe output parsed line by line.

ADR-019 (0.2.0) music mix. details ADR-010. REQ-019/020/043.
  bed = concat of anullsrc gaps + music segs (atrim, asetpts, aresample=48000, aformat=fltp:stereo), atrim=duration=<video_len>. clip past video end -> afade=t=out:st=<len-0.5>:d=0.5. no music where no clips. [va]volume=gv, [mus]volume=gm, amix=inputs=2:duration=first:normalize=0, alimiter=limit=0.97. gain=(p/100)^2 (50% ≈ -12 dB), mute -> 0. music doesn't ripple with video edits.
  UI: music lane under video track inside existing timeline panel, proportions unchanged (REQ-024); sliders record undo on drag stop.

ADR-020 (0.2.0) subtitles model + burn-in. REQ-016/017/018.
  lines stored in source time; shown via projection: for each video clip in order, lines of that media intersecting [src_in,src_out) clipped and mapped clip_start+(s-src_in); words outside hidden. cut removes, copy dupes, split keeps. re-time converts through the shown clip, clamped. add-at-playhead uses clip under playhead (2 s default, clamped). re-text of line shown twice changes both. text edit clears words -> highlight preset spreads evenly by word count.
  burn-in: ass::write(shown, preset, geometry) -> subs.ass + NotoSans-Bold.ttf in fonts/ in private tempfile dir; ffmpeg cwd = that dir; filter `subtitles=f=subs.ass:fontsdir=fonts` after [outv]. relative names avoid filtergraph path escaping (win C:\, quotes).
  size = min(0.055*H, 0.09*W); margins scale; wrap per preset. presets: Classic (white, black outline, bottom); Box (white on semi-opaque box BorderStyle=3, lower third); Word highlight (bold white, current word yellow via {\kNN} SecondaryColour->PrimaryColour, centre-low, <= ~4 words/chunk).
  escape: { -> (, } -> ), \ -> U+FF3C; only our code emits \N \k; text cap 1000 chars.

ADR-021 (0.2.0) STT pipeline. REQ-014/015/044.
  whisper-rs 0.16.0 (Unlicense; whisper-rs-sys 0.15.0 rust-version 1.88; build deps cmake 0.1 + bindgen 0.72 -> cmake, C++ compiler, libclang). whisper.cpp MIT. CPU only, no metal/cuda.
  model ggml-tiny-q5_1.bin multilingual MIT: https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny-q5_1.bin size 32152673 sha256 818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7 (verified HF API 2026-11-14). stored ProjectDirs::data_dir()/models/; .part + exact size + sha -> rename. consent dialog (32 MB, MIT, source, "only network access") before any network call. local file option: same sha+size, copied into models dir. choosing another model out of scope.
  http: ureq 3.5.2 (MIT/Apache, rust-version 1.85) optional under stt, default-features=false features=[rustls] -> rustls 0.23 + ring 0.17 (Apache-2.0 AND ISC) + webpki-roots 1.0 (CDLA-Permissive-2.0). https only, redirects https only, connect+read timeouts, HTTPS_PROXY honoured. only downloader module uses network.
  traits: Transcriber{transcribe(&self, pcm:&[f32], offset:Micros, lang:Option<&str>, cancel:&AtomicBool)->Result<Transcript,SttError>}, ModelFetcher; fakes for orchestration tests; real glue (~100 lines) covered by integration test gated VIDEOLORE_STT_MODEL (CI: cached model by sha, espeak-ng "hello world, this is videolore", assert "hello" + increasing word ts).
  word ts: set_token_timestamps(true), set_split_on_word(true), set_max_len(1); group words into lines at pauses > 0.6 s or preset chunk limit.
  language: auto on first chunk (set_language(Some("auto")) -> detected id), stored in project; override re-runs. one per project.
  threads clamp(cores/2,1,4); one STT job at a time; outside ffmpeg cap; extraction = 1 Background slot. chunks 120 s from extracted temp file (boundary artefacts accepted). cancel via abort callback AtomicBool; progress = chunk/total.
  scope: each timeline source transcribed in full (trim extend reveals subs); video audio only, never music; regenerate replaces all lines, confirm if any edited (REQ-044).
  alt rejected: cloud APIs; vosk; candle-whisper; reqwest (tokio, heavier) vs ureq; native-tls (OpenSSL on linux) vs rustls.

ADR-022 (0.2.0) cargo features: default = ["audio","stt"]; stt = ["dep:whisper-rs","dep:ureq"]. supersedes ADR-009 "stt off by default". release builds include STT. source builds need cmake + C++ + libclang; --no-default-features = minimal build: "Generate subtitles" shows "not available in this build"; edit/presets/burn-in still work. CI clippy/test default, --no-default-features, --all-features.
  CI per OS: ubuntu apt cmake libclang-dev espeak-ng (+ffmpeg libasound2-dev pkg-config); macOS Xcode clang + cmake; windows MSVC + cmake + LLVM from image, LIBCLANG_PATH=C:\Program Files\LLVM\bin if needed. rust-cache keeps whisper.cpp build.

ADR-023 (0.2.0) project schema v2 + dirty. REQ-026..029.
  v2 adds music{clips[{id,media,src_in,src_out,start}],next_id}, mix{video_volume 0..=100, video_muted, music_volume 0..=100, music_muted}, subtitles{language?, language_overridden, preset, lines[{id,media,start,end,text,edited,words[{start,end,text}]}], next_id}.
  load: v1 -> migrate in memory (empty music, mix 100/unmuted, no subs; export identical). >2 -> ProjectError::TooNew(v), clear message, never written on load. validate: music refs audio-only, no overlap, in range; sub lines ref existing media, start<end, text <= 1000 chars, words inside line, <= 200k words total; 16 MiB cap kept.
  save always v2 -> v1 file saved in 0.2.0 unreadable by 0.1.x (changelog note).
  dirty: saved:Option<ProjectDoc>; ProjectDoc = persisted model (media w/o transient status/proxy, + Edit, abs paths). dirty = current_doc() != saved, recomputed after mutation/undo/redo/import/removal (not per frame). undo to saved clears. Analysing media excluded until Ready.
  title "<name> •" when dirty, "Untitled" before first save, e.g. "clip.vlproj • — videolore".
  Save = write project_path or Save As. PendingAction{New, Open(Option<PathBuf>), Close, Quit}; modal Save/Discard/Cancel only when dirty; failed/cancelled save stops action.
  close (eframe 0.37): ui(): close_requested && dirty && !allow_close -> ViewportCommand::CancelClose + modal(Close); after Discard/successful Save -> allow_close + ViewportCommand::Close.
  alt rejected: dirty counter/flag (undo back to saved wouldn't clear); hash compare (no gain over struct eq at this size).

ADR-024 (0.2.0) quit + macOS menu. Command::Quit from Cmd+Q (macOS) / Ctrl+Q. macOS: NativeOptions::event_loop_builder + EventLoopBuilderExtMacOS::with_default_menu(false) (winit via eframe re-export, no new direct dep) so Cmd+Q reaches keymap -> REQ-028 prompt. loss: standard Hide/Minimize menu items (accepted).

ADR-025 (0.2.0) supply chain. cargo-deny 0.20.2 in check job, deny.toml: licences allow MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unlicense, CDLA-Permissive-2.0, BSL-1.0, MPL-2.0; advisories; bans. cargo-about 0.9.3 -> packaging/notices/rust-crates.txt (plain-text template), committed, CI freshness = regenerate + diff. ffmpeg archives + appimagetool pinned SHA-256. Cargo.lock committed, --locked. closes ADR-011 "cargo-deny not done".

ADR-026 (0.2.0) notices/About (REQ-031). notices module: ffmpeg (version read at runtime), x264/x265/libass/other libs + licences, Noto Sans OFL-1.1, whisper.cpp MIT, model MIT, rust crates (generated), source links + written offer, patent note. About reachable from command bar; also installer licence page (win).

## NFR targets (0.2.0)
- <= 3 ffmpeg/ffprobe procs always; by construction (ADR-015); test atomic high-water mark: 20 imports + export + preview.
- UI thread: no subprocess/blocking I/O except project save/load (<= 16 MiB) + small linux desktop writes at startup.
- import 10 files: dispatch returns immediately, frame < 50 ms.
- still frame: 0 renders for non-changing edits; <= 1 per 10-edit burst.
- A/V: video advances during audio stall; drift after resume <= 40 ms (business 100 ms).
- export 200 segs/1 src: 1 decoder; frame-accurate (frame count = Σ seg frames; framemd5 spot checks).
- memory: <= 200 MB peak RSS growth short vs long same-res source (sysinfo own pid); CI long = 2–5 min lavfi; ASSUMPTION: no size-dependent structure -> holds for 4 GB; unit tests on buffer caps.
- playback start with proxy < 300 ms (target, not gate). STT tiny >= 10x real time @4 thr (estimate). installer 70–130 MB (estimate, measured). startup to window < 1 s (ffmpeg check bg).
- export progress >= 2/s, cancel < 1 s, no partial outputs, atomic project save (unchanged).
- platforms: Linux x86_64 (Ubuntu LTS, Fedora current), macOS aarch64, Windows x86_64.
- coverage >= 85% line, feature coverage 100%; every REQ >= 1 `// covers: REQ-0xx` test.

## security
threat: malicious media names/contents, malicious project JSON, PATH hijack, tampered downloads (ffmpeg build, model), filtergraph/ASS injection, desktop entry injection.
- no shell; OsString args; file: prefix; protocol whitelist file,pipe.
- bundled ffmpeg = shipped untrusted-media parser: refresh pin every release; SHA-256 pinned at build; settings show path, version, source (bundled/override/PATH). PATH last.
- spawn cwd only our private temp dir.
- ASS escape (ADR-020), text cap.
- model: consent, https only, pinned commit URL, exact size + sha before use, .part->rename; user file must match sha+size before whisper.cpp (C++ parser) loads it. ureq only under stt + only downloader. no telemetry.
- proxy cache: hash names; clear only our pattern in our dir.
- linux desktop: writes only ~/.local/share; Exec quoted per spec; target abs existing file; helpers abs args no shell errors ignored.
- macOS odoc paths -> same open/validation path as user pick.
- project v2: strict serde, size cap, full invariant validation.
- supply chain ADR-025. secrets: none. unsigned installers (Andrea) -> README first-launch steps; signing deferred.

## crate shortlist (verified 2026-11-14; Cargo.toml/lock authoritative)
runtime existing: eframe/egui 0.37, cpal 0.18.2 (opt audio), rfd 0.18, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0.
new runtime: sha2 0.11.1 (dev -> dep; proxy key + model check); sysinfo 0.40.3 (MIT, rust-version 1.95, default-features=false features=[system]); whisper-rs 0.16.0 (opt stt, Unlicense); ureq 3.5.2 (opt stt, rustls); macOS-only objc2 0.6.5 + objc2-foundation 0.3.2 (MIT, minimal features).
tools (CI only, not linked): cargo-packager 0.11.8, appimagetool (pinned tools.lock), cargo-deny 0.20.2, cargo-about 0.9.3, espeak-ng (test fixture).
bundled non-Rust: ffmpeg GPL builds (BtbN, Riedl), Noto Sans Bold OFL-1.1, model ggml-tiny-q5_1 MIT (downloaded, not bundled).

## risks
1. manual-only checks: macOS launch-time odoc timing, Gatekeeper on nested ffmpeg (fallback `xattr -dr com.apple.quarantine /Applications/videolore.app`), SmartScreen, linux file-manager behaviour. odoc fail -> in-app Open still works. manual checklist in docs/code.md (PE).
2. cargo-packager no release ~1 y -> fallback hand NSIS + hdiutil.
3. Riedl URL retention unknown -> CI fails loudly, re-pin.
4. whisper-rs-sys bindgen/libclang on 3 OS -> build time + CI fragility.
5. non-Latin subtitle scripts -> system fonts (documented limitation).
6. GPL corresponding-source adequacy not lawyer-reviewed.
7. some linux file managers won't run ELF from tar.gz -> README fallback.
8. scope largest so far, long CI -> milestones M0–M7 each green.
9. egui frequent breaking releases (ADR-012) -> thin ui. new stable lints -> fix promptly.
10. ffmpeg build variance for user override/PATH -> startup capability probe + fallback; bundled copy has known caps.
11. H.264/HEVC patent licensing varies by country -> note in notices.

## 0.2.0 milestones (PE)
M0 deps+tooling (Cargo.toml, version, deny.toml, cargo-about, CI apt + model cache) | M1 projects (v2, dirty, save, prompt, close/quit, argv) | M2 scheduler + probe pool + async ffmpeg check | M3 FrameKey/mailbox, clock, run-grouped inputs, hvc1 | M4 music | M5 proxies + machine + memory | M6 subtitles + ass + stt | M7 packaging, icon, notices/About, desktop, locks, scripts, CI package job. then documenter; architect UPDATE.
