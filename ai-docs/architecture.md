# ai-notes: architecture

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

target: 0.3.0 (ADR-028..036) | MSRV 1.100 (ADR-012) | toolchain: latest stable | ed 2024 | GPL-3.0-only
scope 0.2.0: resolve all 0.1.3 known limitations + REQ-026..044 + REQ-014..020 implemented (music, subtitles/STT) + bundled ffmpeg (C1 reversed) + installers.
scope 0.3.0: REQ-045..059 (auto setup, plain Settings, effort levels, preview quality, subtitle quality levels, model storage, old-CPU gate, responsiveness, untrusted-input hardening, safe downloads, private temp files) + QG-01..04 (perf review, vuln check + ffmpeg refresh, faster tests, CI warnings). REQ-055 (hw encode) stays planned (ADR-032). Andrea away: SEVERE items defaulted (ADR-030/031/032, effort semantics ADR-029, in-app Settings window ADR-028) -> confirm on return.
exact crate versions: Cargo.toml/Cargo.lock (PE owns) = source of truth. versions below = verified at plan time (2026-11-14), not pins.
full plans archived in session scratch (arch-0.2.0.md, brief-0.3.0.md; not durable); this file = durable summary.
human docs split: docs/architecture.md = index; -ffmpeg, -packaging, -media-pipeline, -subtitles, -security, -dependencies, -performance (0.3.0).
PE implements 0.3.0 in parallel with this UPDATE: numbers below are decisions/estimates, not measurements; QG-01/03 measured values come from PE report (M0 baseline vs M8).

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
- jobs (NEW): Scheduler (cap 3; 0.3.0: set_cap(u8) per effort level, MAX_CAP 6, ADR-029), Permit, LimitedRunner, spawn_group, preemption, cancel. wraps Runner -> every ffmpeg/ffprobe spawn goes through it.
- export: worker w/ Export-priority runner; writes ASS+font into private work folder used as cwd (0.3.0: scratch::Work, ADR-033); partial file + atomic rename. 0.3.0: threads:Option<u32> from budget.
- preview: wall-clock master + audio drift correction; latest-only still mailbox + 80 ms debounce; music+subs in graph; proxies.
- proxy (NEW): rule, key, cache dir, bg generation + progress, clear, size.
- stt (NEW): Transcriber + ModelFetcher traits + fakes, extraction job, chunking, word ts, lang detect/override. WhisperTranscriber + UreqFetcher cfg(feature="stt").
- machine (NEW; reworked 0.3.0, ADR-028): Machine{logical,physical,ram_bytes,os,arch,cpu:CpuFeatures}, detect(), speech_supported(), fingerprint(), own_rss(). tests inject Machine.
- settings (NEW 0.3.0, ADR-028/029): Settings v2 (moved out of app.rs), LegacySettings, startup() pure, recommend(), budget(), availability(), summary(), main_texts(), load_file()/save (only I/O).
- scratch (NEW 0.3.0, ADR-033): private work folders <cache>/work/<rand>/ 0700 + lock + targets registry; cleanup_stale() at start.
- ui/settings (NEW 0.3.0): egui::Window Settings (main 3 choices + status + storage; collapsed Advanced).
- notices (NEW): static table of bundled components + generated rust crate notices (include_str!). 0.3.0: all 3 speech models.
- desktop (NEW): linux self-integration (menu entry, icon, mime) + removal; macos odoc Apple Event handler.
- ui: thin egui panels. app: AppState::dispatch, tick() drains channels, settings.
- main.rs: run_native (+ macOS: event_loop_builder default menu off, odoc observer before run_native; argv[1] open).
data flow:
- UI ev -> AppState::dispatch -> mutate Edit -> history -> recompute dirty -> FrameKey@playhead -> if != shown_key post still req to mailbox after 80 ms debounce (injected now).
- import: Command::Import(paths) -> placeholder Analysing -> probe pool (3 thr, Probe prio) -> mpsc -> tick() Ready/Rejected -> proxy rule may enqueue proxy job.
- export: render::plan_export(project, Source::Export) (originals) -> export thread, Export runner (1 ffmpeg) -> progress -> rename.
- subtitles: per timeline source w/ audio: extract 16 kHz mono f32 temp file (Background, 1 slot, released after extraction) -> STT thread (own thread budget, outside cap) -> 120 s chunks -> Transcript -> lines in source time. edited lines -> confirm first (REQ-044).
- open by double-click: Win/Linux argv[1] -> Command::OpenPath; macOS odoc -> mpsc -> tick -> OpenPath. both through REQ-028 prompt.
- startup (0.3.0): Machine::detect -> scratch::cleanup_stale -> settings::load_file (regular file, <= 64 KiB+1 read) -> settings::startup(bytes,&m) pure -> {settings, notice, write} -> save if write -> budget(effort,&m) -> sched.set_cap -> preview mode from Preview quality -> notice.
- setting change (0.3.0): SetEffort -> budget + set_cap (running jobs keep args); SetPreviewQuality -> preview rule, queue newly needed proxies, keep existing; SetSubtitleQuality -> save only, never regenerate. all saves write_atomic.

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

ADR-015 (0.2.0) process scheduler. REQ-033. jobs::Scheduler{cap:3} Mutex<State>+Condvar. [0.3.0: cap per effort level via set_cap, 3..=6, ADR-029]
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
  cache: ProjectDirs::cache_dir()/proxies/<key>.mkv; key = first 32 hex of SHA-256(abs path, size, mtime ns, PROXY_PARAMS_VERSION) (sha2 -> runtime dep). reused across sessions, recreated if missing, size in settings. clear deletes only ^[0-9a-f]{32}\.mkv(\.partial)?$ not being generated. no auto eviction. on by default; switch in settings (not project). export never uses proxies: plan_export identical on/off. [0.3.0: switch SUPERSEDED by Preview quality Automatic/Smoother/Original, ADR-029; cache moves under Settings "Preview copies"]
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
  threads clamp(cores/2,1,4) [0.3.0 SUPERSEDED: budget.stt_threads ADR-029]; one STT job at a time; outside ffmpeg cap; extraction = 1 Background slot. chunks 120 s from extracted temp file (boundary artefacts accepted). cancel via abort callback AtomicBool; progress = chunk/total.
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

ADR-027 (0.2.1) whisper.cpp CPU baseline (decided by Andrea). ctx: whisper-rs-sys -> cmake ggml GGML_NATIVE default ON (not cross, no SOURCE_DATE_EPOCH) -> -march=native of GH runner (may have AVX-512) -> release exe SIGILL on user CPUs lacking runner ISA.
  choice: .cargo/config.toml [env] GGML_NATIVE = "OFF". whisper-rs-sys 0.15.0 build.rs forwards every GGML_*/WHISPER_*/CMAKE_* env var as cmake -D (verified source 2026-11-14). ggml CMakeLists (vendored): NATIVE OFF + NATIVE_DEFAULT ON -> INS_ENB ON -> SSE4.2, AVX, AVX2, BMI2, FMA, F16C ON (MSVC: /arch:AVX2, FMA/F16C implied); AVX512*, AVX-VNNI, AMX OFF (verified source).
  result: x86-64 STT baseline = AVX2+FMA+F16C+BMI2 (Intel Haswell 2013+, AMD Excavator 2015+/Zen; ~2013+). macOS aarch64: NEON/ARMv8 baseline as before (unaffected). applies to source builds too (config.toml in repo); user may override GGML_NATIVE=ON for a local native build.
  pre-AVX2 x86-64: ASSUMPTION (not verified on hw): STT path (whisper/ggml code) hits illegal instruction -> process crash (SIGILL / STATUS_ILLEGAL_INSTRUCTION) when generating subtitles; rest of app unaffected because ggml code only runs inside STT (ASSUMPTION: compiler does not hoist AVX2 into code run at startup; ggml objects only reached via whisper calls). no runtime CPU check in 0.2.1 (possible follow-up: is_x86_feature_detected!("avx2"/"fma"/"f16c") gate -> "not supported on this CPU" message instead of crash; or GGML_CPU_ALL_VARIANTS + GGML_BACKEND_DL dynamic dispatch).
  CI: rust-cache key bumped so cached native-built whisper-rs-sys artefacts are not reused (build.rs has no rerun-if-env-changed on GGML_NATIVE).
  alt rejected: native (SIGILL on users); SSE-only (GGML_AVX*/FMA/F16C OFF -> ~7x slower STT, per Andrea's measurement/estimate); runtime dispatch GGML_CPU_ALL_VARIANTS (needs GGML_BACKEND_DL = shipping loadable backend libs; not supported by whisper-rs-sys static build; more packaging) -> deferred.
  risk: setting SOURCE_DATE_EPOCH in build env flips NATIVE_DEFAULT OFF -> INS_ENB OFF -> silent SSE-only build (slow); keep it unset or set GGML_AVX2/FMA/F16C/BMI2 ON explicitly if ever needed. future whisper-rs-sys bump: re-verify env forwarding + ggml defaults.
  0.3.0: "no runtime CPU check" SUPERSEDED by ADR-031 (gate + message).

ADR-028 (0.3.0) machine detection + settings v2 + startup decision. REQ-045..049, 057. A1 defaulted.
  ctx: auto-setup on first start on every OS; zero technical knowledge; Settings plain words.
  detection (machine.rs): logical=std available_parallelism (cgroup/affinity aware); physical=sysinfo physical_core_count, unknown -> max(1,ceil(L/2)), clamp <= L; ram=sysinfo total_memory (0 unknown); os/arch std::env::consts; CpuFeatures{sse42,avx,avx2,fma,f16c,bmi2} via is_x86_feature_detected! (checks OS AVX support); aarch64 all true. speech_supported = aarch64 || all six (ADR-027 baseline). fingerprint{os,arch,logical,physical,ram_gib=(ram+GIB/2)/GIB nearest, speech_supported}. no timing, no network -> deterministic, ms (<= 3 s guaranteed; test detect()<3 s + equal fingerprints twice).
  where: app first start on every OS incl AppImage (A1); installers do no tuning. alt rejected: Windows installer runs detection; installer writes flag (two code paths, untestable).
  settings.json v2 (config dir): {version:2, preview_quality: automatic|smoother|original, subtitle_quality: fast|balanced|most_accurate, effort: light|balanced|maximum, ffmpeg, ffprobe (abs or null), machine:{fingerprint}}. serde rename_all snake_case, unknown fields allowed (fwd compat), any parse error = malformed. validate logical/physical 1..=4096, ram_gib <= 1048576. "recommended in use" = 3 choices == recommend(current machine); no stored flag.
  startup() pure table:
    no file -> recommend(m)+fp, notice Setup, write.
    legacy (no "version", has proxies_enabled/ffmpeg/ffprobe) -> recommended, but Preview=Original if proxies_enabled==false; override kept; models untouched; Setup; write. (REQ-049)
    v2 same fp -> as is, no notice, no write.
    v2 diff fp, choices == recommend(saved fp) -> recommend(new), new fp, Setup, write. (REQ-048)
    v2 diff fp, custom -> keep choices, new fp, MachineChanged notice w/ "Use recommended settings" action, write. (A9)
    malformed (bad JSON, unknown enum, out of range, > 64 KiB, not regular file) -> recommended, Malformed notice, write; bad file copied to settings.json.bad first (overwrite).
    version > 2 -> recommended in memory only, no notice, no write until user changes something.
    override not absolute / not existing regular file -> dropped + Malformed notice.
  recommend(m) pure: preview Automatic; effort Balanced always; subtitles: Fast if !speech_supported, else MostAccurate if L>=12 && R>=15 GiB, else Balanced if L>=4 && R>=7 GiB, else Fast. thresholds in reported GiB (8 GB box ~7.6 GiB). availability: all levels greyed if !speech_supported ("This computer's processor is too old for automatic subtitles."); MostAccurate greyed R<7 GiB, Balanced R<3.5 GiB ("Needs more memory than this computer has."); recommend never picks greyed.
  UI: in-app egui::Window (A7) from command bar "Settings"; main: Preview quality / Subtitle quality / How hard videolore works + one-line explanations, status line, storage lines ("Preview copies" + Clear, "Speech recognition files" + "Remove downloaded speech files"); collapsed Advanced: ffmpeg override (moved from effects panel), bundled component paths/versions/origin, speech engine, processor support. proxy switch removed from ui/effects. main strings only from settings::main_texts() -> banned-word test (thread core proxy codec encoder ffmpeg AVX CPU GPU model quantised; case-insensitive whole words).
  notices: Notice{text, action:Option<NoticeAction>, until:Option<Instant>}; Setup self-closes after 8 s (injected now); MachineChanged w/ action; Malformed.
  "Use recommended settings" (REQ-047): re-detect, set 3 choices = recommend(m), summary(old,new) -> "Subtitle quality: Fast → Balanced" lines or "Already using the recommended settings"; never touches override, projects, proxy cache, models; never fetches.
  alt rejected: toml/ron (serde_json already dep, same format); OS multi-viewport window (Wayland inconsistency, not headless-testable); raw-cpuid/cpufeatures (std suffices); num_cpus / raw /proc,sysctl,Win32 (sysinfo already dep).

ADR-029 (0.3.0) effort levels -> resource budget. REQ-050, REQ-033 reworded, QG-01. A5 + SEVERE defaulted semantics.
  budget(effort,&m) pure, deterministic. L logical, P physical (<= L), R reported RAM.
    max_processes: Light 3 | Balanced 3 | Maximum clamp(L/2,3,6), 3 if R<7 GiB.
    ffmpeg threads (export + proxy): Light Some(max(1,L/2)) | Balanced None | Maximum None (None = ffmpeg auto = 0.2.2).
    stt_threads: Light clamp(min(P,max(1,L/2)),1,8) | Balanced clamp(min(P,max(1,L-1)),1,8) | Maximum clamp(P,1,16).
    proxy workers 1 all levels; preview, ffprobe, audio extraction never limited (ffmpeg auto).
  cap floor 3: playback 2 + export 1 (ADR-015). MAX_CAP 6; probe pool 6 threads, scheduler limits.
  Some(t): export adds -threads t before each -i, -filter_complex_threads t, output -threads t; proxy -threads t -filter_threads t.
  set_cap: lowering never kills; new grants wait until in_use+slots<=cap. level change applies to next job; running jobs keep args.
  worked (P=L/2 w/ HT): L1P1 3/1/1|3/1|3/1; L2P1 3/1/1|3/1|3/1; L4P2 3/2/2|3/2|3/2; L8P4 3/4/4|3/4|4/4 (3 if R<7); L8P8 3/4/4|3/7|4/8; L16P8 3/8/8|3/8|6/8; L32P16 3/16/8|3/8|6/16. (Light cap/threads/stt | Balanced cap/stt | Maximum cap/stt) = expected test output.
  guarantees: every value >=1 and <=L; Light threads + stt <= max(1,L/2); Balanced stt >= 0.2.2 clamp(L/2,1,4) (no regression). Balanced/Maximum: ffmpeg internal auto threading (x264 ~1.5xL) outside guarantee, documented. Light = per job limit, two jobs together may use all cores (SEVERE defaulted).
  output: effort may change export bytes (x264 thread-dependent) but not look/length; Preview quality never changes export (REQ-051).
  preview quality (REQ-051): Automatic = needs_proxy unchanged (ADR-018, A6); Smoother = w*h > 1280*720; Original = never. plan_export identical for all.
  alt rejected: strict total thread budget split across jobs (slower Balanced exports -> fails QG-01, complex); OS niceness (unsafe setpriority/libc); cgroups (Linux only).

ADR-030 (0.3.0) subtitle quality levels = 3 whisper models. REQ-052/053, REQ-014 reworded. A2 SEVERE defaulted. extends ADR-009/021 (single model superseded).
  all huggingface.co/ggerganov/whisper.cpp, MIT, URL https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/<file>; size+sha verified HF tree API 2026-11-14:
    Fast ggml-tiny-q5_1.bin 32152673 818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7 (unchanged)
    Balanced ggml-small-q5_1.bin 190085487 ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb
    MostAccurate ggml-large-v3-turbo-q5_0.bin 574041195 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
  ESTIMATES (ASSUMPTION, PE measures): vs tiny small 6–8x slower, turbo 20–30x; 8 AVX2 thr tiny ~30x RT, small ~5x, turbo ~1–2x; peak mem small ~0.5 GB, turbo ~1.3 GB.
  rejected: base-q5_1 60 MB (small gain); medium-q5_0 539 MB (slower + less accurate than turbo); q8_0 (+~40% size, little gain); f16 (3–6x size); .en (no multilingual). greedy best_of 1 all levels (beam 2–3x slower).
  code: ModelSpec table replaces constants; ModelStore per spec, downloaded_bytes() (sum of known files present), remove_downloaded() (only known names + .part). flash attention on (WhisperContextParameters::flash_attn; no DTW) + one WhisperState per job (not per 120 s chunk); PE keeps flash attn for Fast only if measured tiny time not worse. stt threads from budget (ADR-029).
  consent modal per level: level name, size MB, MIT, source. options: Download | "Use Fast instead (no download)" only if tiny installed, this run only, setting unchanged | "Download the Fast one instead (32 MB)" only if nothing installed | "Choose a file on this computer…" must match level size+sha | Cancel (nothing changes). declining changes nothing; changing level never regenerates; REQ-044 confirm when regenerating at other level.
  integrity: full sha at install; before each load size check + sha once per session (memo by path,size,mtime). download limit spec.size+1. disk full -> delete .part, "Not enough free disk space".
  storage (REQ-053): sum of known files; remove after confirmation; subtitles in projects unaffected; re-download needs consent.
  accuracy test: higher_levels_are_at_least_as_accurate (integration, feature stt, gated VIDEOLORE_STT_MODELS_DIR; CI cache keyed by 3 sha): espeak-ng sentence w/o brand names, word-level Levenshtein WER after lowercase + strip punct; WER(small)<=WER(tiny), WER(turbo)<=WER(tiny).
  engine unchanged: whisper-rs 0.16.0 (latest; abort-callback workaround stays). alt candle-whisper, vosk, cloud rejected (ADR-021).

ADR-031 (0.3.0) older processors: CPU gate + message, no fallback engine. REQ-054. A4 SEVERE defaulted. supersedes ADR-027 "no runtime check".
  infeasible fallback in 0.3.0: whisper-rs-sys links one static ggml -> second ISA variant = symbol clash; GGML_CPU_ALL_VARIANTS needs GGML_BACKEND_DL (unsupported by whisper-rs-sys). only route = separate SSE-only helper exe + IPC + packaging -> 0.4 candidate (~7x slower).
  0.3.0: !speech_supported -> Generate shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand."; no download offered; 3 levels greyed w/ same reason; ggml never reached. test: inject CpuFeatures{avx2:false,..} -> no transcriber call, message, recommend Fast, all unavailable.
  optional CI (M7): lib test under qemu-x86_64 -cpu Westmere on gate test (proves startup incl ggml static init doesn't SIGILL). ASSUMPTION qemu user-mode refuses AVX2 for that model; PE verifies with one-line AVX2 probe first, else skip + manual checklist.

ADR-032 (0.3.0) hardware export encoding deferred. REQ-055 stays (planned). A3 SEVERE defaulted.
  feasible in principle (BtbN gpl: nvenc, amf, qsv/libvpl, vaapi on linux; macOS VideoToolbox) but: no GPU on GH runners -> only fallback testable; per-vendor rate control tuning (nvenc -cq, qsv -global_quality, amf -qp_*, vaapi hwupload+device, vt -q:v); driver bugs -> bad frames w/ exit 0 not caught by fallback (risk REQ-022/023); shares 3 s detection budget; does not fit session.
  future design: bg probe (3-frame lavfi encode per candidate, 1 s timeout each, total <= 3 s, result stored in fingerprint), hw-first export, sw re-run on non-zero exit. "Export: Faster / Best quality" choice only then.
  alt: all vendors w/ fallback; VideoToolbox only (still unverifiable in CI).

ADR-033 (0.3.0) private work folder + crash cleanup. REQ-059.
  scratch::Work::new(kind) -> <cache_dir>/videolore/work/<rand>/ mode 0700 + `lock` file (std File::try_lock, stable 1.89, exclusive for job lifetime) + optional `targets` file listing partial paths outside the folder.
  contents: extracted audio, ASS, font copy, filter scripts. ffmpeg cwd = work folder.
  export partial: .<name>.videolore-partial in dest dir, create_new + 0600, registered in targets; ffmpeg -y truncates but keeps mode; success -> 0644 + rename; fail/cancel -> remove. proxy <key>.mkv.partial and model <file>.part registered same way.
  cleanup_stale() at start: each work folder whose lock try_lock succeeds (= stale) -> delete each target whose name matches ^\..+\.videolore-partial$ | ^[0-9a-f]{32}\.mkv\.partial$ | <known model>.part AND is regular file AND not symlink; then delete folder. held lock (other window) -> untouched. windows in-use file -> skip.
  cache + data dirs created 0700 on Unix.
  alt rejected: system temp (/tmp often RAM-backed, long audio fills RAM; shared); sysinfo PID liveness (PID reuse); age-based deletion (races w/ long jobs).

ADR-034 (0.3.0) security hardening. REQ-057/058, QG-02. findings from code review 2026-11-14.
  unsafe: only whisper abort callback (reviewed ok, 'f lifetime binding) + desktop::macos objc2. lib.rs #![deny(unsafe_code)], #[allow(unsafe_code)] on stt::whisper + desktop::macos only.
  BUG project::load: metadata().len() then read_to_string -> FIFO/device .vlproj blocks UI forever; growing file beats 16 MiB cap. fix: open, file.metadata()?.is_file(), read take(MAX+1), refuse over. same in settings::load_file (64 KiB). serde_json recursion limit 128 -> deep nesting = error (test > 200 levels).
  media paths from projects: read-only by ffprobe/ffmpeg; probe rejects non-regular; NEW 30 s probe timeout -> kill + "could not be analysed in time". relative `..` resolved vs project folder, still read-only: accepted, documented.
  BUG concat_quote escapes ' but not \n/\r -> name w/ newline injects ffconcat directives (file protocol, read-only). fix: any control char in a source path -> no stream copy, re-encode path (-i args safe).
  display: control chars + bidi overrides U+202A–202E, U+2066–2069 -> U+FFFD in displayed names only.
  settings: override must be abs existing regular file; settings file trusted at user-account level (writer can already run code as user) -> stated in threat model.
  downloads (REQ-058): URLs only from ModelSpec (test: all https://); fetch refuses non-https before any connection (offline testable); ureq https_only also redirects, max 5 redirects, timeouts kept; size limit; sha before rename; partial / wrong sha -> delete .part. test: ureq used only in stt::download (include_str! grep test, like packaging tests). no telemetry/update check.
  corpus tests (REQ-057): bad/truncated JSON, nesting > 200, > 16 MiB, 1e309, u64::MAX, negative times, ../../etc/passwd, abs paths to non-media, control + bidi in names, Unix FIFO project (return < 1 s). every case: no crash, clear refusal, nothing written, sources unchanged. settings corpus same idea.
  supply chain: cargo-deny already fails on advisories; add yanked = "deny", documented ignore = [] (future entries "RUSTSEC-…", # reason not affecting). optional weekly schedule run of cargo deny check advisories.
  CI: top-level permissions contents: read; contents: write only package job; third-party actions pinned by full SHA w/ tag comment (ADR-036).
  ffmpeg refresh: 9.0.2 still latest FFmpeg release; Riedl macOS 1789931890_9.0.2 already pinned. Linux/Windows re-pin BtbN autobuild-2026-11-13-12-43 (9.0 branch): linux64-gpl ffmpeg-n9.0.2-57-gd838f9f9ae-linux64-gpl-9.0.tar.xz sha256 01f1a5a6721de2d4e2e49dbadf6eb30bb750934429f685f6f5a389168ac27e23; win64-gpl-shared ffmpeg-n9.0.2-57-gd838f9f9ae-win64-gpl-shared-9.0.zip sha256 f4e068477787394da18a44b27d3e2cb19e05b21495b836a4fd5a31958b608932. source = full commit of gd838f9f9ae (PE resolves), build_scripts_commit = BtbN matching. supersedes ADR-013 "pin month-end" for this pin; risk daily build expires sooner -> re-pin 2026-11-30 month-end on failure (loud).

ADR-035 (0.3.0) test speed. QG-03.
  fixtures generated once per param set -> env!("CARGO_TARGET_TMPDIR")/fixtures/<sha256(params + ffmpeg version)>.mp4, written .partial + rename (parallel-safe, reused across runs).
  smaller media where size irrelevant: 640x360 -> 320x180 (except letterbox/geometry tests); 120 s memory source -> 160x90 @10 fps; proxy source 0.5 s.
  fixed sleeps -> channels/condvars where possible.
  Cargo.toml: [lib] doctest = false (no doctests); [profile.dev] debug = "line-tables-only"; try [profile.dev.package."*"] opt-level = 1, keep only if warm cargo test measurably faster. whisper.cpp already Release (no gain).
  target: cargo test wall >= 30% below M0 baseline (3 runs median). no test removed/weakened.
  runner: cargo test (rule 7). alt cargo-nextest rejected (breaks scripts/test-annotations.sh output parsing; rule names cargo test).
  CI check job: single instrumented run `cargo llvm-cov --locked --ignore-filename-regex 'src/main\.rs$' --fail-under-lines 85 --summary-only` in existing tee + test-annotations pattern (tests once, whisper.cpp built once). msrv job: cargo build --locked --all-targets only (MSRV == stable by policy; tests run in check; none removed).
  open (not blocking, needs /change-workflow): local rule-7 suite runs tests twice (cargo test + coverage.sh); single llvm-cov run locally would ~halve PE verification.

ADR-036 (0.3.0) CI runners + actions. QG-04, REQ-040/042 reworded.
  runners: ubuntu-24.04 (check, msrv, build, package; glibc 2.39 -> runs on Ubuntu 24.04 + 26.04 LTS + current Fedora; AppImage needs fusermount3, present on all), macos-15, windows-2025 (PE confirms labels). removes ubuntu-latest migration notice.
  scripts/package.sh linux: fail if highest GLIBC_ symbol version needed by videolore > 2.39.
  actions (all Node 24 or composite; verified releases 2026-11-14):
    actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0 (replaces @v4 -> Node 20 warning gone)
    actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
    actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
    Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3
    taiki-e/install-action@4a76afe23f9e08d2d9550fb56e556444e8611d21 # v2.90.4 (composite)
    softprops/action-gh-release@8a0cd651e5b2365bd640854a3fa5dfc70e4875f3 # v3.0.4
    dtolnay/rust-toolchain@683b588c487a69d01f6c6ddf40f036ff682cc2c8 (composite) w/ toolchain: stable | msrv
  speech-model cache keyed by 3 sha, VIDEOLORE_STT_MODELS_DIR. packaging/notices/rust-crates.txt regenerated if Cargo.lock changes.
  optional: qemu old-CPU step (ADR-031).

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
- platforms: Linux x86_64 (Ubuntu LTS, Fedora current), macOS aarch64, Windows x86_64. STT on x86_64 needs AVX2+FMA+F16C+BMI2 CPU (ADR-027); everything else no extra CPU req.
- coverage >= 85% line, feature coverage 100%; every REQ >= 1 `// covers: REQ-0xx` test.

## NFR targets (0.3.0 additions/changes)
- procs: "<= 3" -> <= budget.max_processes of current level (3/3/clamp(L/2,3,6)); test 20 imports + export + preview at every level, L=1,8,16: high-water <= cap, all complete.
- detection: < 50 ms typical (estimate), <= 3 s guaranteed, deterministic, no network.
- startup (QG-01): <= 0.2.2 + 0.5 s (detection + cleanup ms; Settings window built only when opened).
- QG-01 at recommended: export <= 0.2.2 +5% (Balanced ffmpeg args == 0.2.2); Fast subtitles <= 0.2.2 +5% (same model, >= threads, flash attn gated by measurement, 1 state/job).
- REQ-056 harness ui::tests::stays_responsive_while_exporting_and_transcribing: FakeRunner export streaming progress ~3 s + fake transcriber busy on available_parallelism threads (Maximum) + 30 headless frames each injecting Space / timeline scroll / Settings click; pass = each frame (tick + run_ui) <= 200 ms wall and input effect visible next frame. rule: UI thread never waits on lock held across worker I/O (try_lock / channels only; review point).
- memory REQ-039 unchanged; MostAccurate +~1.3 GB (estimate), greyed < 7 GiB.
- platforms: STT on x86_64 still AVX2+FMA+F16C+BMI2 (+SSE4.2, AVX); without -> gate message, never crash (ADR-031).
- QG-03: cargo test wall >= 30% lower than M0; CI time not growing.

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
- 0.3.0 (ADR-033/034): settings file untrusted for parsing (malformed -> recommended), trusted at account level for override; FIFO/device project + settings refused (regular-file check, bounded read); probe timeout 30 s; concat control-char guard; display sanitising (control + bidi); deny(unsafe_code) crate-wide w/ 2 allows; downloads https-only pre-connection check + redirect cap 5 + model URL table test + ureq-only-in-downloader test; private work folder 0700 + partial exports 0600 + lock-based crash cleanup restricted to own name patterns, regular non-symlink files; cache/data dirs 0700; cargo-deny yanked=deny; CI least-privilege permissions + SHA-pinned actions.

## crate shortlist (verified 2026-11-14; Cargo.toml/lock authoritative)
runtime existing: eframe/egui 0.37, cpal 0.18.2 (opt audio), rfd 0.18, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0.
new runtime: sha2 0.11.1 (dev -> dep; proxy key + model check); sysinfo 0.40.3 (MIT, rust-version 1.95, default-features=false features=[system]); whisper-rs 0.16.0 (opt stt, Unlicense); ureq 3.5.2 (opt stt, rustls); macOS-only objc2 0.6.5 + objc2-foundation 0.3.2 (MIT, minimal features).
tools (CI only, not linked): cargo-packager 0.11.8, appimagetool (pinned tools.lock), cargo-deny 0.20.2, cargo-about 0.9.3, espeak-ng (test fixture).
bundled non-Rust: ffmpeg GPL builds (BtbN, Riedl), Noto Sans Bold OFL-1.1, model ggml-tiny-q5_1 MIT (downloaded, not bundled).
0.3.0 review (QG-01): no new crates, none dropped. all latest 2026-11-14: eframe 0.37 (glow; wgpu heavier, rejected), cpal 0.18.2, rfd 0.18, sysinfo 0.40.3 (dropping needs per-OS unsafe -> keep; now also physical cores), ureq 3.5.2, sha2 0.11.1 (CPU SHA ext), serde/serde_json, thiserror, directories 6, tempfile 3.27, whisper-rs 0.16.0, objc2/winit macOS. downloaded models now 3 (ADR-030), all MIT. CI tools: + cargo-llvm-cov (already used by coverage.sh), qemu-user optional.

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
12. (0.2.1) pre-AVX2 x86-64 users: STT crash (ASSUMPTION) -> README/system requirements state min CPU; runtime gate = follow-up candidate (ADR-027). 0.3.0: gate done (ADR-031); residual ASSUMPTION: static ggml init does not run AVX2 at startup (optional qemu check).
13. (0.3.0) model speed/memory + recommendation thresholds are estimates -> adjust after PE measurement (A10).
14. (0.3.0) fingerprint changes on VMs/containers w/ variable cores -> custom path only notifies; recommended path recomputes silently.
15. (0.3.0) flash attention on CPU may slow tiny -> measured, gated.
16. (0.3.0) BtbN daily build pin may expire before re-package -> re-pin month-end.
17. (0.3.0) Windows: in-use file in stale work folder not deletable -> skipped, retried next start.
18. (0.3.0) SEVERE defaults pending Andrea: hw encode deferred, old-CPU message only, model choice + thresholds, Light semantics, in-app Settings window.

## 0.2.0 milestones (PE)
M0 deps+tooling (Cargo.toml, version, deny.toml, cargo-about, CI apt + model cache) | M1 projects (v2, dirty, save, prompt, close/quit, argv) | M2 scheduler + probe pool + async ffmpeg check | M3 FrameKey/mailbox, clock, run-grouped inputs, hvc1 | M4 music | M5 proxies + machine + memory | M6 subtitles + ass + stt | M7 packaging, icon, notices/About, desktop, locks, scripts, CI package job. then documenter; architect UPDATE.

## 0.3.0 milestones (PE)
M0 baselines (cargo test wall 3-run median, per-test integration times, 60 s 1080p export, Fast STT 60 s, start-to-first-frame) | M1 machine + settings pure + tests (REQ-045/048/049/050/057 settings) | M2 set_cap, threads args, stt threads, preview quality (REQ-033/050/051) | M3 Settings UI + notices + proxy switch removed (REQ-046/047) | M4 ModelSpec levels, consent, storage, CPU gate, flash attn + state reuse, notices (REQ-052/053/054/058) + WER test | M5 scratch, project::load hardening, probe timeout, concat guard, display sanitising, deny(unsafe), 0700 (REQ-057/059) | M6 REQ-056 harness | M7 test speed + CI (runners, SHA pins, llvm-cov single run, msrv build-only, model cache, ffmpeg re-pin, yanked=deny, glibc check, optional qemu) | M8 suite + coverage + feature coverage + QG-01/03 numbers vs M0; documenter (0.3.0); architect UPDATE.
