# ai-notes: architecture (index; owner: architect)

<!-- AI-only notes. Owner agent edits only. Dense format OK. Read sub-documents by grep, then sed -n 'A,Bp'. -->

target: 2.0.1 PLAN (module cycles, REQ-151..153; plan ai-docs/architecture-plan-2.0.1.md; ADR-136..141; PE step 8 updates docs/code-architecture.md) | 1.6.3 RELEASE (consolidation 2026-11-20; verified against src/ and Cargo.toml; open before release: origin and licence of assets/splash/splash.gif and splash-player.gif, see architecture-risks.md) | built 1.6.2 (shared splash, ADR-128..135) | built 1.6.1 (visible edges, per-clip effects, format 9, CI time) | built 1.6.0 (splash, Apply LUT, workspace with player/, icons; ADR-105..119) | built 1.5.0 (frame: aspect, Scale, Move, keyframes; format 7; ADR-093..104) | 1.4.x (export dialog, clean audio, text width; ADR-080..091) | 1.3.0 (text items, 360 video; ADR-066..079) | 1.4.1 page (ADR-092).
product facts: Rust, edition 2024, rust-version 1.100 (MSRV = latest stable by policy, ADR-012), toolchain latest stable, GPL-3.0-only, author Andrea Lorenzani. Versions 2.0.1 in root Cargo.toml and player/Cargo.toml (both literal, ADR-116); 1.6.3 was the last release.
platforms: Linux x86_64 (Ubuntu 24.04+, Fedora current; AppImage + tar.gz), macOS aarch64 (dmg, ad-hoc sign), Windows x86_64 (NSIS setup). Editor = videolore, player = videolore-player (no speech, no network).
engine: ffmpeg + ffprobe as separate bundled processes (GPL builds, pinned SHA-256, ADR-002/013); whisper.cpp via whisper-rs (opt stt feature, ADR-021/022); egui/eframe 0.37 GUI (ADR-001); std threads + mpsc, no tokio (ADR-004); process scheduler cap 3..6 (ADR-015/029).
state: project JSON format 9 (SCHEMA_VERSION 9; loads 1..9; TooNew refused, never written); settings.json version 2 additive fields (ADR-028/040/044); private work folders 0700 with lock + crash cleanup (ADR-033).
testing: cargo test (no doctests), coverage >= 85% line (llvm-cov, main.rs excluded), feature coverage 100% via `// covers: REQ-NNN`; CI jobs check/msrv/build/package (ADR-035/036/117); 2.0.1 adds the module-cycle test (ADR-140).

## ai sub-documents (read only when a task needs them; all owned by architect)
- ai-docs/architecture-adr.md § whole file — L1-514 (514 lines): ADRs ADR-001..141 (ids, context, choice, alternatives, reason; status per ADR; superseded ones one line with replacement); area index at its top. Read with grep -n 'ADR-0NN' then sed.
- ai-docs/architecture-components.md § whole file — L1-98 (98 lines): verified module structure vs src (lib.rs modules, app/export/project/render/settings/ui submodules), import cycles (re-checked 2026-11-20), component roles per module and version, module dependency list, ai data flow for every version.
- ai-docs/architecture-nfr.md § whole file — L1-66 (66 lines): NFR targets by area: processes, UI responsiveness, preview, export and quality (QG-01, QG-05), memory, platforms and portability, page; GATE vs ESTIMATE marked.
- ai-docs/architecture-security.md § whole file — L1-39 (39 lines): threats, core rules (no shell, file: prefix, protocol whitelist, unsafe policy, downloads, supply chain, CI permissions), security by version 0.2.0..1.6.2.
- ai-docs/architecture-risks.md § whole file — L1-81 (81 lines): open risks by area (release items first), hardware export limits, subtitles, interface, 360, frames/LUT/player/splash, code debt; resolved items removed.
- ai-docs/architecture-plan-2.0.1.md § whole file — L1-125 (125 lines): plan 2.0.1 (measured cycles, base module layout, move table, cycle breaks, behaviour invariants, cycle check design, ordered PE steps, tests, risks, future work). Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-doc-index.md § whole file — L1-256 (256 lines): paragraph index of docs/architecture*.md (one line per heading: `- <file> § <heading> — L<start>-<end> (<n> lines)`); moved here from this file to keep it short; regenerate after each change of a managed doc.
- ai-docs/architecture-plan-1.3.0.md, -1.4.0.md, -1.5.0.md, -1.6.0.md, -1.6.1.md, -1.6.2.md: archived in the latest releases/<version> - <name>/docs/ai-docs/ snapshot (moved there by the command after the snapshot; do not look for them in ai-docs/ after the release). Their full plan text (ADR details, code facts, ordered steps, Future work) is read from the snapshot only when history is needed.

## ADR index (one line each; full text in ai-docs/architecture-adr.md)
- ADR-001 GUI eframe/egui (built; MSRV reasoning superseded by ADR-012)
- ADR-002 media engine: ffmpeg/ffprobe CLI subprocess (built; "not bundled" superseded by ADR-013)
- ADR-003 preview audio cpal (clock part superseded by ADR-016)
- ADR-004 concurrency std::thread + mpsc, no tokio
- ADR-005 support crates serde, serde_json, thiserror, directories, rfd, tempfile
- ADR-006 export tiers: stream copy only when safe, else one re-encode; temp + rename
- ADR-007 encoder mapping sw first, -encoders probe; hw opt-in (ADR-037)
- ADR-008 fades min(0.5 s, clip/2)
- ADR-009 STT whisper.cpp (superseded in parts by ADR-022 and ADR-030)
- ADR-010 music via ffmpeg amix/volume/afade; detail ADR-019
- ADR-011 CI jobs (cargo-deny closed by ADR-025)
- ADR-012 toolchain policy: latest stable; rust-version 1.100
- ADR-013 bundled ffmpeg (GPL builds: BtbN Windows/Linux, Martin Riedl macOS; lock file; smoke test)
- ADR-014 installers and packaging per OS (NSIS, dmg, AppImage, tar.gz; unsigned)
- ADR-015 process scheduler: priorities, slots, preemption (cap 3 floor; set_cap 0.3.0)
- ADR-016 preview: latest-only still mailbox, FrameKey, wall-clock master, audio correction
- ADR-017 export inputs grouped in monotone runs; hvc1 tag rule
- ADR-018 proxies: needs_proxy rule, cache key, libx264 proxy format, memory caps
- ADR-019 music mix bed and gain (p/100)^2, alimiter
- ADR-020 subtitles model, projection, burn-in, ASS escape, presets
- ADR-021 STT pipeline: whisper-rs 0.16, model download with consent, ureq + rustls
- ADR-022 cargo features default = audio + stt; minimal build without stt
- ADR-023 project schema v2 + dirty tracking; title marker; close guard
- ADR-024 quit path and macOS default menu off
- ADR-025 supply chain: cargo-deny licence list, cargo-about notices, pinned archives
- ADR-026 notices and About
- ADR-027 whisper.cpp CPU baseline GGML_NATIVE=OFF (superseded in part by ADR-031)
- ADR-028 machine detection, settings v2, start-up decision table (first start, legacy, same, changed, malformed, newer)
- ADR-029 effort levels -> resource budget (Light/Balanced/Maximum), worked values
- ADR-030 three subtitle quality levels = three whisper models with sha and size
- ADR-031 older processors: CPU gate + message, no fallback engine
- ADR-032 hardware export deferred (superseded by ADR-037..040)
- ADR-033 private work folder + crash cleanup (lock file, targets registry)
- ADR-034 security hardening (unsafe policy, bounded reads, FIFO refusal, downloads, ffmpeg refresh pin)
- ADR-035 test speed (fixtures, smaller media, one llvm-cov run in CI; open note on local double run)
- ADR-036 CI runners ubuntu-24.04, macos-15, windows-2025; SHA-pinned actions
- ADR-037 hardware encode args per vendor (NVENC, QSV, AMF, VA-API incl. CQ deviation, VideoToolbox)
- ADR-038 hardware probe (synthetic lavfi; 0.4.2 time-outs 1 s + 4 s retry, 10 s total, repeat once when inconclusive; libva guard)
- ADR-039 hw-first export: stall watchdog, output check (ffprobe + psnr position samples), silent sw redo
- ADR-040 Export choice, failure memory (hardware_off), settings fields export and hardware_off
- ADR-041 own proportional splitter layout (Fractions, Mins, GAP 6 px)
- ADR-042 single action table for menus, toolbar, shortcuts, keymap
- ADR-043 popup and instance state in AppState driven by Commands
- ADR-044 settings v2 additive fields layout and recent (VERSION stays 2)
- ADR-045 UI acceptance tests via AccessKit tree; REQ-068 audit
- ADR-046 egui keyboard zoom off; timeline zoom only
- ADR-047 drag and drop via winit; Linux Wayland not supported (ship where supported)
- ADR-048 timeline markers derived; undo labels; disabled reasons
- ADR-049 start/end fades on video edges (1.2.0; moved to clip flags by ADR-123)
- ADR-050 per-clip volume in Clip; gain (vol/100)^2 x video gain; undo on value change (Q2)
- ADR-051 project schema v3 always written; validation Q6
- ADR-052 one ApplyEffect command; effect drag via egui DragAndDrop
- ADR-053 embedded text subtitle tracks via ffprobe + ffmpeg SRT extraction
- ADR-054 in-house srt.rs parse/write; SRT export
- ADR-055 subtitle creation per range: Recognise / Align / Spread; all-or-nothing; one undo
- ADR-056 word alignment per cue Needleman-Wunsch, in-house, deterministic
- ADR-057 per-line position via ASS \pos; preview drag; selection as UI state
- ADR-058 ruler labels by visible range; round tick steps
- ADR-059 360 projection engine: ffmpeg v360 + sendcmd for preview and export (AS BUILT spike rules)
- ADR-060 360 detection (spherical metadata enumerated values) + override (removed 1.4.0, ADR-086)
- ADR-061 camera path model per source media in source time, centi-degrees
- ADR-062 live view, drag, zoom (window state, not saved)
- ADR-063 360 export geometry (fill frame, d_fov, proxy 4096 rule, too-large message)
- ADR-064 boxes, per-line size, Align and Reset
- ADR-065 project schema v4 (spherical, treat_as_360, camera, line.size)
- ADR-066 text items through the subtitle ASS/libass path (one engine)
- ADR-067 text effects as ASS events (fad, move, blink phases; caps 2000/20000)
- ADR-068 installed fonts via fontdb 0.24 (background scan, charset whitelist)
- ADR-069 360 anchor (yaw, pitch, size, wrap) burned into the sphere frame before v360; anchored subtitles plain white (DEV-3)
- ADR-070 rectilinear projection in camera.rs (same convention as v360)
- ADR-071 text events per segment; flat in flat ASS, anchored before v360; no extra process
- ADR-072 ExportMode Automatic/Sphere360/FlatCamera (ask modal removed by ADR-085)
- ADR-073 one Selection value (BTreeSet<ObjRef>) in src/selection.rs
- ADR-074 Context group rule over selection and playhead-on-360
- ADR-075 label "Fade IN/OUT"; internal name unchanged
- ADR-076 schema v5: texts and line.anchor; TTL rule (DEV-1)
- ADR-077 TECHNICAL: anchored slide and fade move in the sphere plane; Q1 = A (flat sticker)
- ADR-078 360 video output: anchored items at anchor; view-fixed subtitles at preset place (Q2 = A); Spherical V1 marker written by own MP4/MOV writer (DEV-2)
- ADR-079 TECHNICAL: anchored text size 5% of view vertical FOV, wrap 60% of horizontal FOV
- ADR-080 clean audio = ffmpeg afftdn per video clip before clip volume (1.4.0)
- ADR-081 clean_audio flag (schema v6); whole video = every video clip
- ADR-082 export dialog: pure export/choice.rs + modal; session memory; target guards
- ADR-083 video presets: fixed 12-entry table, always re-encoded
- ADR-084 audio formats Same (AAC) / aac / mp3 (probed); only-audio graph
- ADR-085 360 choice only when all video clips are 360 (default 360 video)
- ADR-086 360 switch removed; is_360 = spherical metadata; treat_as_360 dropped on load
- ADR-087 text width permille 100..1000, height 20% H; best fit by in-house estimate (0.6 em)
- ADR-088 v6 migration: width by search, anchor height from old lines, clean=false; Q1 = A
- ADR-089 flat text and subtitle drag limit: centre on picture (grab rule)
- ADR-090 Effects menu = effects + Add text + Apply; item commands in Edit menu
- ADR-091 Context group and Align/Reset for text items; width handle right edge only
- ADR-092 static project page: one HTML, inline CSS, no JS; TECH items for PE (page)
- ADR-093 storage: frame_aspect, whole_scale/move, clip_frames (1.5.0; whole-video Scale/Move removed by ADR-125)
- ADR-094 time base: clip keys in source time; whole-video keys in timeline time
- ADR-095 limits: Key<T>, <=200 per effect, Scale 100..4000 permille, Move 0..10000, hold + linear
- ADR-096 Scale pivot: hidden centre offset; corner drags keep opposite corner fixed
- ADR-097 frame size: pure frame.rs; longer side cap 4096 (decided 2026-11-19)
- ADR-098 layers: fitted picture without pad, background, overlay, fades after composition
- ADR-099 expressions from numbers only; script file when long; sampled fallback
- ADR-100 colour #RRGGBB -> 0xRRGGBB; blur fixed strength
- ADR-101 edit rules: split/copy keep keys, merge needs equal keys, remove drops; one undo
- ADR-102 export: any frame effect re-encodes; Sphere360 ignores frame effects
- ADR-103 preview hit priority: text/subtitle boxes > Scale corners > Move target > picture
- ADR-104 format 7 always written; keys outside media dropped; REQ-132 clip wording flagged
- ADR-105 Cargo workspace: root videolore (lib + editor) + player/ (videolore-player)
- ADR-106 player = in-memory one-clip Project + preview engine; no save/export/edit
- ADR-107 player: no settings file; writes only cache and work folders
- ADR-108 player installers via scripts/package.sh <target> [editor|player]; no file association
- ADR-109 LUT reader src/lut.rs: .cube 3D 2..65, 1D 2..4096, 16 MiB cap, clamp 0..1
- ADR-110 LUT in graph: fixed-name cube in Work folder; lut3d tetrahedral; per segment order
- ADR-111 LUT storage: Timeline.whole_lut + clip_luts; format 8; paths UTF-8
- ADR-112 missing LUT file: project opens, LUT inactive, export stops naming the file
- ADR-113 splash (1.6.0): pure splash.rs + window; include_bytes of assets; skipped with a path argument (superseded by ADR-128..131)
- ADR-114 icons: vendored Phosphor Regular TTF (MIT) as fallback font; ui/icons.rs constants
- ADR-115 notices: rust-crates.txt (editor) + rust-crates-player.txt; Phosphor entry; origin of splash image open (Q-E)
- ADR-116 versions: both Cargo.toml literal; version.sh writes and checks both lock entries and the badge
- ADR-117 gates: --workspace; coverage ignores both main.rs; feature coverage scans player/; player has no ureq/whisper-rs
- ADR-118 player open rules: argv[1] video, File > Open, drop; .vlproj refused by extension
- ADR-119 player subtitles and camera: shared embedded-track extraction, .srt 16 MiB cap, positions session-only
- ADR-120 paused preview fix: generation bump on target change; REQ-034 kept (1.6.1)
- ADR-121 visible edges handles: Hit::Edge(Side), scale_to_edge, 10..400% / 25..400% (1.6.1)
- ADR-122 whole-video effects written per clip; format 9 keeps only Change aspect at video level (1.6.1)
- ADR-123 edge fades as Clip.fade_in / fade_out (replace Timeline start/end fade) (1.6.1)
- ADR-124 format 9: in-memory conversion src/project/v9.rs; Q-F rule (Scale/Move combined; clip LUT wins) (1.6.1)
- ADR-125 copies at application: one Scale/Move per clip; whole-video popup applies to copies; one undo (1.6.1)
- ADR-126 CI time: install only missing apt packages, timeouts, prebuilt cargo-about (QG-06) (1.6.1)
- ADR-127 splash 1.6.1 picture = copy of 1.6.0 (superseded by ADR-134 names)
- ADR-128 shared splash: splash::Program EDITOR and PLAYER; ui::splash_window (1.6.2)
- ADR-129 splash assets compiled in from CARGO_MANIFEST_DIR; no build.rs (1.6.2; 1.6.3 fixed names)
- ADR-130 summary fit: no scroll, no clip, 14 px down to 10 px floor, 372 px wide (1.6.2)
- ADR-131 version text fit in each program rectangle; colour from mean brightness (1.6.2)
- ADR-132 player splash = a splash state in the player, non-blocking, Esc order (1.6.2)
- ADR-133 decode caps: 64 frames, 4096 px, 64 MiB RGBA per picture (1.6.2)
- ADR-134 retired files; fixed names splash.gif, splash-player.gif, summary.txt, summary-player.txt (1.6.2; 1.6.3 AGENTS 4f)
- ADR-135 test memory cap: systemd-run MemoryMax=8G on the development machine (1.6.2)
- ADR-136 (2.0.1 PLAN) shared helpers in one module `src/base/` (io, private, pos, names, language); base has no crate:: path
- ADR-137 (2.0.1 PLAN) cycle breaks: move the shared item, not the caller; simulation: no cycle left
- ADR-138 (2.0.1 PLAN) AssEvent, font_px, BASE_HEIGHT move to ass.rs, not base
- ADR-139 (2.0.1 PLAN) align_h, align_v move from ui/boxes.rs to base::pos; app must not use ui
- ADR-140 (2.0.1 PLAN) cycle check: tests/module_cycles.rs with syn (dev-dependency, in lock), production code, top-level grain
- ADR-141 (2.0.1 PLAN) no re-exports; callers (player included) take the new paths
- Q-A/Q-B/Q-C (1.6.0) closed 2026-11-19 (icon artwork shared; splash GIF kept; no splash with a path argument); Q-E open before release (splash origin).

## Component map (short; roles and versions in ai-docs/architecture-components.md)
- Programs: main.rs -> lib.rs (real_app, native_options, prepare_desktop); player/src/main.rs -> player lib.
- Interface: ui (+ actions, boxes, effects, export_dialog, frame_handles, frame_popups, icons, keymap, layout, lut_popup, markers, media, menu, modals, picture, placement, popups, ruler, settings, splash_ui, splash_window, status, text_popup, timeline, toolbar, zoom), dialogs, splash.
- Application: app (AppState, Command, dispatch, tick; submodules command, effects, export_flow, frame, lut, sphere, subs_create, texts); player app, subs.
- Engine (processes, threads): export (+ export::choice), outcheck, hwenc, preview, proxy, stt, jobs, ffmpeg, scratch.
- Rendering (pure): render (filter graphs and argv), format, ass, camera, frame, keyframes, lut, spherical.
- Model (pure): project (+ project::v9), timeline, music, subtitles, text, selection, history, media, srt, align.
- Base: time, settings, machine, fonts, notices, packaging, desktop.
- 2.0.1 PLAN: base (src/base/: io = capped read and atomic write; private = private folders, Work, file names of the work folder; pos = Pos, POS_MAX, Anchor360, align_h/v; names = cache, partial and speech-model file names; language = language table). scratch keeps default_root and cleanup_stale; ass keeps AssEvent and font_px.
- Verified 2026-11-20 against src/ (module list matches lib.rs; cycles re-checked: 9 pairs + one 16-module group + app<->ui; see ai-docs/architecture-components.md).
- Data flow summary: UI event -> AppState::dispatch -> Edit + history -> FrameKey -> latest-only still; export: render plan -> export worker -> ffmpeg via scheduler -> rename; subtitles: extraction -> STT thread -> lines -> one undo step; start-up: detect machine -> cleanup -> settings decision -> budget.

## NFR targets (summary; full list in ai-docs/architecture-nfr.md)
- GATE processes <= level cap (3 / 3 / clamp(L/2,3,6)); 20 imports + export + preview test at L=1,8,16.
- GATE UI frame <= 200 ms during export + STT (and camera, boxes, texts, 4K/8K tests).
- GATE memory: <= 200 MB more peak for long vs short source of the same resolution.
- GATE Best quality export argv byte-identical to 0.3.0 (golden); projects without new features unchanged.
- QG-01 recommended export and Fast subtitles <= 0.2.2 +5%; start-up <= 0.2.2 + 0.5 s.
- QG-05 manual hardware matrix (e <= 3 VMAF below, f <= 1.5x size, g >= 40% faster FAILED on Intel iGPU, recorded).
- Coverage >= 85% line; feature coverage 100%.
- Platforms: Linux x86_64 glibc 2.39 max; STT needs AVX2+FMA+F16C+BMI2 on x86_64.
- Estimates: installers 70–130 MB; probe ~6.4 s idle, up to 10 s loaded; STT speeds per model.
- 2.0.1 (REQ-151..153, no behaviour change): test-suite time for the cycle check estimate 1 to 3 s (to measure in CI).

## Security model (summary; full in ai-docs/architecture-security.md)
- Threats: malicious media and project/settings JSON, PATH hijack, tampered downloads, ffmpeg/ASS/ffconcat injection, desktop entry injection, hostile drivers (contained).
- No shell; OsString args; `file:` prefix; protocol whitelist file,pipe; cwd only private 0700 folder.
- Bounded reads (project 16 MiB, settings 64 KiB, track text 16 MiB), regular-file checks (FIFO refused), strict validation, TooNew refused.
- unsafe forbidden crate-wide except whisper abort callback and macOS odoc handler; no network except consented model download (https, sha-256, size); player has no network code (CI).
- Unsigned installers (decided); no secrets; no telemetry; cargo-deny + cargo-about + pinned archives; CI least privilege, SHA-pinned actions.
- 2.0.1: capped reads, atomic writes and private folders move to base::io and base::private unchanged (REQ-057, REQ-059).

## Crate shortlist (verified against Cargo.lock 2026-11-20; Cargo.toml authoritative)
runtime (root, always): eframe 0.37.0 (glow, default_fonts, wayland, x11; egui 0.37.0), fontdb 0.24.0 (1.3.0), image 0.25.10 (gif only, 1.6.0), rfd 0.18.0 (xdg-portal), serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0, sha2 0.11.1, sysinfo 0.40.4 (system feature).
runtime optional: cpal 0.18.2 (feature audio, default); whisper-rs 0.16.0 (feature stt, default; Unlicense); ureq 3.5.2 (feature stt, rustls, default-features off).
macOS only: objc2 0.6.5; objc2-foundation 0.3.2 (minimal features); winit 0.30.13 (default-features off; direct dependency to switch off the default menu, ADR-024; same version as eframe's).
transitive of note: fontconfig-parser 0.5.8, memmap2 0.9.12, roxmltree 0.20.0 (via fontdb), rustls/ring (ureq).
dev (2.0.1 PLAN): syn 2 (features full, visit; MIT OR Apache-2.0; already in Cargo.lock as 2.0.119) for tests/module_cycles.rs; no new package.
not in the tree (planned, dropped): ttf-parser 0.25 dev-dependency of the 1.6.0 shortlist is absent from Cargo.toml and Cargo.lock at 1.6.3 (no test needs it; Future work: decide).
rejected across versions: egui-phosphor, egui-pop-phosphor, egui_extras, resvg (MPL), Font Awesome (CC BY), font-kit, fontique, muda, egui_dock/egui_tiles, egui_kittest (fallback only), srtparse/subparse/srtlib, strsim, ab_glyph/fontdue, matroska/mp4 demuxers, candle-whisper, vosk, reqwest, native-tls, toml/ron, num_cpus, raw-cpuid, smithay-client-toolkit, cargo-nextest (as runner).
tools (CI only, not linked): cargo-packager 0.11.8, appimagetool (packaging/tools.lock), cargo-deny 0.20.2, cargo-about 0.9.3 (prebuilt via pinned install action), cargo-llvm-cov, espeak-ng (test fixture), optional qemu-user.
bundled non-Rust: ffmpeg/ffprobe GPL builds (9.0.2 pinned: BtbN linux64-gpl and win64-gpl-shared, Riedl macOS arm64; lock in packaging/ffmpeg.lock), Noto Sans Bold (OFL-1.1), Phosphor Regular (MIT), splash pictures (origin open). Downloaded after consent: whisper models ggml-tiny-q5_1, ggml-small-q5_1, ggml-large-v3-turbo-q5_0 (MIT).

## Risks and open items (summary; full in ai-docs/architecture-risks.md)
- [open, release] origin and licence of the splash pictures (Q-E).
- [open] QG-05 g failed on Intel iGPU; re-baseline or cheaper check (Future work).
- [open] CI time after the 1.6.1 change not measured here (estimate only).
- [2.0.1 plan] module cycles: plan in ai-docs/architecture-plan-2.0.1.md; the cycle check is syntactic (macro-made paths and #[path] not followed; #[path] fails the test); oversized files app.rs (about 3000 lines), render.rs (about 3400 lines) remain debt; docs/code-architecture.md must gain the app<->ui pair and the base layer (PE step 8).
- [open] docs/architecture.md is about 160 lines: split its Components table at the next release (architect).

## Human docs index
- The paragraph index of docs/architecture*.md is in ai-docs/architecture-doc-index.md (sub-document, read with grep -n, then sed -n 'A,Bp' on the docs file).
