# ai-notes: architecture

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

target: 0.1.3 | MSRV 1.88 | ed 2024 | GPL-3.0-only
scope 0.1.3: REQ-001..013 + REQ-021..025 impl. planned: REQ-019..020 music (0.1.4), REQ-014..018 subtitles/STT (0.1.5).
exact crate versions: Cargo.toml/Cargo.lock (PE owns). this file = choices + constraints, not pins.

## component map (src/ modules)
- time: rational/int time base (µs i64 or frame-exact rational), ts parse/format, no floats in cut math. pure.
- media: MediaInfo (from ffprobe json): container, streams, codec, w/h, fps (rational), sr, ch, duration, keyframe idx (lazy). pure model + parser.
- timeline: clips (src ref, in/out), order, cuts/splits, fades; invariants (in<out, non-overlap, bounds). pure.
- history: undo/redo, command pattern or snapshot of timeline (timeline small -> snapshot OK). pure.
- project: save/load JSON (serde), schema version field, relative+abs src paths, missing-media detection. pure + fs.
- format: output format/codec table, encoder mapping + fallback list, container<->codec compat. pure.
- render: ExportPlan builder: timeline+media+format -> tier decision + ffmpeg arg vector (Vec<OsString>). pure, heavily unit-tested.
- ffmpeg: binary locate, version/filter probe, spawn (Command, no shell), progress parse (-progress pipe:1), cancel (kill), stderr capture (bounded ring). only module that spawns processes.
- export: job runner on worker thread: run plan -> temp file in dest dir -> fsync? -> rename atomically; cleanup on fail/cancel.
- preview: frame decode via ffmpeg subprocess (rawvideo rgba pipe, scaled), audio via ffmpeg pcm f32 pipe -> cpal; null sink fallback.
- ui: egui panels (media bin, timeline, preview, export dialog, settings). thin; no business logic (coverage gate 85% -> logic must live in pure modules).
- app: eframe App, state, channel pump, settings load/save.
- main.rs: eframe::run_native only.
data flow: UI event -> app -> timeline/history mutate -> preview req (mpsc) -> worker -> frames back (mpsc) -> UI ctx.request_repaint. export: app -> render::plan -> export worker -> ffmpeg -> progress msgs -> UI.

## ADRs
ADR-001 GUI. ctx: desktop editor, Lin/mac/Win, MSRV 1.88. choice: eframe/egui 0.33 (MSRV 1.88 ok). alt: iced (heavier, async model), slint (licence dual GPL/commercial ok but DSL+build), gtk4-rs (system deps on win/mac), tauri (webview, JS). reason: pure Rust, immediate mode simple, single binary, wgpu/glow backends, MIT/Apache.
ADR-002 media engine. choice: ffmpeg/ffprobe CLI as subprocess. alt: ffmpeg-next/ffmpeg-sys (link libav*, build pain, ABI/version churn, licence linkage issues w/ static), gstreamer-rs (system deps heavy), pure-Rust decoders (incomplete codecs). reason: zero build deps, robust, any ffmpeg >=4.4.
  rules: Command::new(path).args(vec) never shell; inputs prefixed `file:` (avoid protocol/option injection from names starting with `-` or `xxx:`); `-protocol_whitelist file,pipe`; `-nostdin -hide_banner -y` only on our temp output; output to temp path we generate.
  lookup order: settings path > env (VIDEOLORE_FFMPEG / VIDEOLORE_FFPROBE) > next to exe > PATH. require version >= 4.4 + check needed filters present (`-filters`: e.g. trim/atrim/concat/afade/fade/scale/aresample). not bundled (size, licence variants GPL/LGPL, patents) -> clear UI message w/ install hint if missing.
ADR-003 preview audio. choice: cpal. alt: rodio (on top of cpal, extra), sdl2 (C dep). Linux needs libasound2-dev at build. fallback: no device/err -> null sink (video-only preview, clock from Instant). reason: minimal, maintained, Apache-2.0.
ADR-004 concurrency. choice: std::thread + std::sync::mpsc (crossbeam not needed). alt: tokio (overkill, async in egui awkward). workers: preview decode, audio, export, probe. cancellation via AtomicBool + child.kill().
ADR-005 support crates: serde+serde_json (project/settings/ffprobe json), thiserror (error enums per module), directories (config dir for settings), rfd (native file dialogs). all MIT/Apache -> GPL-3.0 compatible.
ADR-006 export tiers.
  T1 stream copy (-c copy): ONLY IF all clips same src format (codec/res/fps/sr/ch/timebase identical) AND output format == source-compatible AND no effects (no fades) AND every cut on keyframe (in-point at keyframe). else
  T2 single re-encode pass: one ffmpeg invocation, filter_complex trim/atrim+setpts per clip, fades, concat, scale/fps/aresample normalize to output. no multi-pass intermediates.
  always write temp file in dest dir (same fs) then rename; delete temp on error/cancel; never overwrite source.
ADR-007 codec/encoder mapping. format table maps target (mp4/h264+aac, webm/vp9+opus, mov, mkv...) to encoder preference list e.g. h264: libx264 > h264 hw encoders? (keep sw first for determinism) > fallback; check availability via `ffmpeg -encoders` once at startup, cache. if none available -> disable format in UI w/ reason.
ADR-008 fades. default fade len = min(0.5 s, clip_len/2) per edge; video fade + audio afade aligned.
ADR-009 STT (deferred to 0.1.5, decided now). whisper.cpp via whisper-rs behind cargo feature `stt` (off by default -> default build no C++ toolchain need). default model ggml tiny-q5_1 multilingual. model not bundled: one-time download only after explicit user consent, pinned URL + SHA-256 verify, stored in data dir; offline thereafter. alt: cloud APIs (privacy, cost, rejected), vosk (quality), candle-whisper (perf/maturity; reconsider).
ADR-010 music (0.1.4): reuse ffmpeg amix/volume/afade in T2; no new crate expected.
ADR-011 CI (applied 0.1.3 by PE; .github/workflows owned by PE per AGENTS.md -> NOT via /change-workflow; prior plan saying so corrected). check job + Linux build job: apt install ffmpeg libasound2-dev. check job env VIDEOLORE_REQUIRE_FFMPEG=1 -> integration tests needing ffmpeg fail instead of skip. elsewhere tests skip if ffmpeg absent. recommendations NOT done: MSRV 1.88 CI job (cargo +1.88 check), cargo-deny (licences/advisories/dup).

## NFR targets
- startup < 1 s to window (excl. ffmpeg probe, async).
- probe: ffprobe per file < 2 s typical; non-blocking UI.
- UI 60 fps idle/edit; never block UI thread on I/O or subprocess.
- preview scrub latency < 300 ms for 1080p (seek -ss before -i, scaled down to preview size).
- export T1 ~ I/O speed; T2 bounded by encoder; progress updates >= 2/s; cancel < 1 s.
- memory: preview frame buffers bounded (ring, few frames); stderr ring bounded (~64 KiB).
- reliability: no partial output files; project save atomic (temp+rename); project schema versioned.
- portability: Linux x86_64, macOS aarch64, Windows x86_64 (CI matrix).
- coverage gate 85% -> pure modules (time, timeline, history, project, format, render, media parser) carry tests; ffmpeg integration tests gated by availability.

## security
threat: malicious media file names/contents, malicious project JSON, PATH hijack of ffmpeg.
- no shell; args as OsString vec; file: prefix; protocol whitelist file,pipe -> no network fetches from crafted playlists/concat lists.
- project JSON: serde strict types, size cap, validate invariants after load (reject bad in/out), paths only used as file inputs.
- ffmpeg is untrusted-input parser: run as user, no elevated; decoding bugs are ffmpeg's surface (user-supplied binary).
- PATH lookup last; show resolved ffmpeg path+version in settings.
- STT model download: consent, https, SHA-256 pinned, no telemetry. no network otherwise.
- secrets: none.
- supply chain: Cargo.lock committed, --locked builds; cargo-deny recommended.

## risks
- ffmpeg version/build variance (missing encoders/filters) -> startup capability probe + fallback.
- keyframe detection cost for T1 -> lazy, only when T1 candidate; else T2.
- A/V sync in preview w/ separate pipes -> audio clock master, drop video frames.
- egui 0.33 MSRV bump in future -> pin minor.
- Linux audio build dep (libasound2-dev) for source builders -> doc it.
- whisper-rs C++ build on Windows/mac CI (0.1.5) -> feature-gated.
- coverage gate vs GUI code -> keep ui thin.
