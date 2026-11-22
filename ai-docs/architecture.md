# ai-notes: architecture (index; owner: architect)

<!-- AI-only notes. Owner agent edits only. Dense format OK. Read sub-documents by grep, then sed -n 'A,Bp'. -->

target: 2.3.0 PLAN (2026-11-22, not built; REFINE with Andrea's Q1-Q9 applied; theme the clip: overlap media, Fit per clip, Volume and Change speed effects, Crop removed, Never removed, Enlarge word fix; format 14 (ADR-230); plan ai-docs/architecture-plan-2.3.0.md; ADR-230..241 in ai-docs/architecture-adr-2.3.0.md; REFINE 2: Q-A, Q-B, Q-C answered by Andrea, plan § 13; ADR-242, ADR-243; PM adds REQ-333 (Enlarge fix) and the REQ-330 status line) | built 2.2.0 (2026-11-22, not released; REQ-315 NOT MET 8.7% of 20%, REQ-177 NOT MET 43% of 90%, both measured; plan section 14; was PLAN 2026-11-21; theme speed; REQ-312..316, REQ-177 amended, whole-video amendments REQ-083/147/148/129/131/076/200..202/303; plan ai-docs/architecture-plan-2.2.0.md; ADR-214..221 in ai-docs/architecture-adr-2.2.0.md; Q-1 answered 2026-11-21: route D, `.cube` kept, background preparation with progress and unprocessed picture meanwhile (D63); REFINE after REQ-317..323 (D64 and the order request): project format 13 (ADR-222; 2.1.0 refuses it; 12 read with defaults); per-clip order of all rows (ADR-223..225), Applied effects column (ADR-226), text size with the per-text maximum (ADR-227, ADR-228), order model (ADR-229)) | built 2.1.0 (not released; commit 281d2b2; LUT edge 256 and Applied effects widget, shadows, styles, format 12; ADR-202..213 in ai-docs/architecture-adr-2.1.0.md; open decisions closed by Andrea 2026-11-21) | built 2.0.4 (final 2026-11-21; not released; ONE development D57; REQ-180..299 all built, QG-14..18 built; ADR-168..201 final statuses in the ADR block; format 11 (ADR-169); questions Q-1..Q-7 closed by Andrea (D61); open items in § Risks and open items) | built 2.0.3 (readable text, placement, accuracy; REQ-155..179 except REQ-177 target open; ADR-152..167; Q-1 = B, Q-2 = A) | built 2.0.2 (verify the release, REQ-154, QG-07..10; ADR-142..151) | built 2.0.1 (module cycles, REQ-151..153; ADR-136..141) | 1.6.3 RELEASE (ADR-134) | built 1.6.2 (shared splash, ADR-128..135) | built 1.6.1 (visible edges, per-clip effects, format 9, CI time) | built 1.6.0 (splash, Apply LUT, workspace with player/, icons; ADR-105..119) | built 1.5.0 (frame: aspect, Scale, Move, keyframes; format 7; ADR-093..104) | 1.4.x (export dialog, clean audio, text width; ADR-080..091) | 1.3.0 (text items, 360 video; ADR-066..079) | 1.4.1 page (ADR-092).
product facts: Rust, edition 2024, rust-version 1.100 (MSRV = latest stable by policy, ADR-012), toolchain latest stable, GPL-3.0-only, author Andrea Lorenzani. Versions literal in root Cargo.toml and player/Cargo.toml (ADR-116); the current development is 2.3.0 (PLAN); 2.2.0 built, not released; 1.6.3 was the last release.
platforms: Linux x86_64 (Ubuntu 24.04+, Fedora current; AppImage + tar.gz), macOS aarch64 (dmg, ad-hoc sign), Windows x86_64 (NSIS setup). Editor = videolore, player = videolore-player (no speech, no network).
engine: ffmpeg + ffprobe as separate bundled processes (GPL builds, pinned SHA-256, ADR-002/013; CI downloads the pinned 9.0.2 build for tests, ADR-142); whisper.cpp via whisper-rs (opt stt feature, ADR-021/022); egui/eframe 0.37 GUI (ADR-001, accesskit feature ADR-197); std threads + mpsc, no tokio (ADR-004); process scheduler cap 3..6 (ADR-015/029). 2.2.0 finding (history, not used): lut3d reads `.m3d` as TEXT in 9.0.2 (ADR-214; plan § 2).
state: project JSON format 12 in code (SCHEMA_VERSION 12; loads 1..12; TooNew refused, never written by older; read cap 96 MiB, ADR-191); format 11 (ADR-169) and 10 (ADR-158) historical; settings.json version 2 additive fields (ADR-028/040/044/195); private work folders 0700 with lock + crash cleanup (ADR-033); one window by lock file and inbox (ADR-182); autosave copies in the private cache (ADR-173).
testing: cargo test (no doctests), coverage >= 85% line (llvm-cov, main.rs excluded; 2.0.4 measured 94.9% at the last run, to be re-read by the PE), feature coverage 100% via `// covers: REQ-NNN`; CI jobs check/msrv/build/package (ADR-035/036/117); module-cycle test (ADR-140; 7 tests); pinned test ffmpeg, golden argv gates (ADR-168, format 4 and 5 fixtures QG-15); speech test on the committed fixture (QG-14); installer job on packaging PRs (installer-config.yml, ADR-181); weekly advisories (ADR-181); build.rs checks the splash summaries (QG-17).

## ai sub-documents (read only when a task needs them; all owned by architect)
- ai-docs/architecture-adr.md § whole file — L1-584 (584 lines): ADRs ADR-001..201 (ids, context, choice, alternatives, reason; status per ADR; superseded ones one line with replacement); area index at its top; 2.0.4 block (ADR-168..201) with the final status line. Read with grep -n 'ADR-0NN' then sed.
- ai-docs/architecture-adr-2.1.0.md § whole file — L1-27 (27 lines): ADR-202..213 of 2.1.0 (built; kept with the snapshot after release, then its ADRs move to architecture-adr.md).
- ai-docs/architecture-adr-2.2.0.md § whole file — L1-35 (35 lines): ADR-214..229 of 2.2.0 (ADR-221 superseded by ADR-222 format 13; ADR-222 format; ADR-223..225 order and render steps; ADR-226 column; ADR-227..228 text size; ADR-229 order model) (PLAN; REFINE after Q-1: route D, A/B/C rejected with the measurements; whole-video without popup or mark, rows cache, blur, Faster, picture limit, speech regions, format stays 12).
- ai-docs/architecture-components.md § whole file — L1-130 (130 lines): verified module structure vs src (lib.rs modules, app/export/project/render/settings/ui submodules), import cycles (re-checked 2026-11-20), component roles per module and version, module dependency list, ai data flow for every version, 2.0.4 module map and flows as built; § 2.1.0 PLAN roles and flows; § 2.2.0 PLAN roles, flows and cycle rules.
- ai-docs/architecture-nfr.md § whole file — L1-145 (145 lines): NFR targets by area: processes, UI responsiveness, preview, export and quality (QG-01, QG-05), memory, platforms and portability, page, 2.0.2 and 2.0.3 gates, 2.0.4 gates; § 2.1.0 PLAN targets; § 2.2.0 PLAN targets (LUT start and colours, rows, blur, Faster, picture, REQ-177); GATE vs ESTIMATE vs MEASURED marked.
- ai-docs/architecture-security.md § whole file — L1-56 (56 lines): threats, core rules (no shell, file: prefix, protocol whitelist, unsafe policy with three sites, downloads, supply chain, CI permissions), security by version 0.2.0..2.0.4; § 2.1.0 PLAN (LUT file as untrusted input, Work copy name, order validation); § 2.2.0 PLAN (`.m3d` Work copy from the table, picture header, REQ-177 fixture).
- ai-docs/architecture-risks.md § whole file — L1-144 (144 lines): open risks by area (release items first), hardware export limits, subtitles, interface, 360, frames/LUT/player/splash, code debt, 2.0.4 risks and open items; § 2.1.0 PLAN decisions and risks; § 2.2.0 PLAN open items (Q-1 SEVERE first, `.m3d` is text, blur, Faster, REQ-177 check, whole-video change, CI time). Last section.
- ai-docs/architecture-adr-2.3.0.md § whole file — L1-29 (29 lines): ADR-230..243 of 2.3.0 (PLAN; 242 lane and 243 preview size decided by Andrea): format 14, overlap model, render paths, alpha, removal popup, Enlarge fix, clip aspect UI, Volume and speed effects, Crop removed, Add media, Never, 13 conversion and order rows.
- ai-docs/architecture-plan-2.3.0.md § whole file — L1-190 (190 lines): plan 2.3.0 (PLAN, REFINE 2 answers in § 13): facts F1..F11 from the code, decisions, components, format 14, overlap model, three render paths, alpha, edits, other changes incl. the Enlarge bug cause and fix (§ 8.8), NFR, security, tests, stage order S0..S9 for the PE, SEVERE questions Q-A..Q-C (§ 13), mismatches for the PM (§ 15), risks. Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-plan-2.2.0.md § whole file — L1-175 (175 lines): BUILT status in the header; section 14 = built state, what was not met (REQ-315, REQ-177), deviation from REQ-321, open decisions for Andrea: plan 2.2.0 (REFINE after Q-1 = route D): facts found in the code (whole-video data per clip; routes to remove; LUT store and `.cube` route; blur; output check; picture header); spike history (S0a `.m3d` text format and timings, S0a-2 HALD, S0b blur proxy, S0d REQ-177 prior results); approach per REQ (LUT preparation with progress and unprocessed fallback, whole-video removal, rows cache, blur, Faster, picture limit, speech regions); components; data flow; cycle rules; NFR and gates; security; tests; stage order S0..S8 for the PE; Q-1 answered; risks; future work. Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-plan-2.1.0.md § whole file — L1-106 (106 lines): plan 2.1.0 (why the 80 MB LUT was refused, LUT approach with 256 edge and budgets, applied-effects order and stacking, shadows and styles, format 12, components, cycle rules, tests, stage order S0..S9, risks, decisions Q-1..Q-3 and REQ-307 kept). Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-plan-2.0.1.md § whole file — L1-125 (125 lines): plan 2.0.1. Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-plan-2.0.2.md § whole file — L1-90 (90 lines): plan 2.0.2. Archived after the release (drop this line then).
- ai-docs/architecture-plan-2.0.3.md § whole file — L1-217 (217 lines): plan 2.0.3. Archived at release (drop this line then).
- ai-docs/architecture-plan-2.0.4.md § whole file — L1-135 (135 lines): plan 2.0.4 (scope, facts, approach, components by stage, format 11 by stage, stage order, cut-back list, Q-1..Q-7 closed by Andrea, risks, future work). Archived in the release snapshot after release (drop this line then). Not edited after the build.
- ai-docs/architecture-plan-2.0.4-stages-1.md § whole file — L1-118 (118 lines): plan 2.0.4 stages 0–5 (template per stage). Archived after release.
- ai-docs/architecture-plan-2.0.4-stages-2.md § whole file — L1-153 (153 lines): plan 2.0.4 stages 6–12 (same template). Archived after release.
- ai-docs/roadmap-2_0-checkpoint.md § Architect — SHARED temporary checkpoint (delete when 2.0.4 is released; each agent edits only its own section): the architect's final 2.0.4 state, ADR statuses, where the built designs are documented, open items, what remains for the release.
- ai-docs/architecture-plan-1.3.0.md, -1.4.0.md, -1.5.0.md, -1.6.0.md, -1.6.1.md, -1.6.2.md, and the 2.0.1 to 2.0.4 plans: archived in the latest releases/<version> - <name>/docs/ai-docs/ snapshot (moved there by the command after the snapshot; do not look for them in ai-docs/ after the release). Their full plan text is read from the snapshot only when history is needed.
- The plans of 2.1.0 and 2.2.0 are archived in the latest releases/<version> - <name>/docs/ai-docs/ snapshot after their release.

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
- ADR-034 security hardening (unsafe policy, bounded reads, FIFO refusal, downloads, ffmpeg refresh pin); amended 2.0.4 by ADR-183 (one more macOS menu call)
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
- ADR-069 360 anchor (yaw, pitch, size, wrap) burned into the sphere frame before v360; anchored subtitles plain white (DEV-3; superseded in 2.0.4 by ADR-188)
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
- ADR-094 time base: clip keys in source time, whole-video keys in timeline time
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
- ADR-107 player: no settings file; writes only cache and work folders (2.0.4: own settings file, ADR-184)
- ADR-108 player installers via scripts/package.sh <target> [editor|player]; no file association (2.0.4: "Open with" only, ADR-184)
- ADR-109 LUT reader src/lut.rs: .cube 3D 2..65, 1D 2..4096, 16 MiB cap, clamp 0..1
- ADR-110 LUT in graph: fixed-name cube in Work folder; lut3d tetrahedral; per segment order
- ADR-111 LUT storage: Timeline.whole_lut + clip_luts; format 8; paths UTF-8
- ADR-112 missing LUT file: project opens, LUT inactive, export stops naming the file (2.0.4: three-button modal, ADR-191/REQ-292)
- ADR-113 splash (1.6.0): pure splash.rs + window; include_bytes of assets; skipped with a path argument (superseded by ADR-128..131)
- ADR-114 icons: vendored Phosphor Regular TTF (MIT) as fallback font; ui/icons.rs constants
- ADR-115 notices: rust-crates.txt (editor) + rust-crates-player.txt; Phosphor entry; origin of splash image recorded (Q-E closed 2026-11-20)
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
- ADR-126 CI time (QG-06): install only missing apt packages, timeouts, prebuilt cargo-about (1.6.1; espeak-ng removed 2.0.4, ADR-201)
- ADR-127 splash 1.6.1 picture = copy of 1.6.0 (superseded by ADR-134 names)
- ADR-128 shared splash: splash::Program EDITOR and PLAYER; ui::splash_window (1.6.2)
- ADR-129 splash assets compiled in from CARGO_MANIFEST_DIR; no build.rs (1.6.2; 1.6.3 fixed names; 2.0.4 build.rs summary check, ADR-201)
- ADR-130 summary fit: no scroll, no clip, 372 px wide (1.6.2; the 14/10 px values superseded by ADR-143)
- ADR-131 version text fit in each program rectangle; colour from mean brightness (1.6.2)
- ADR-132 player splash = a splash state in the player, non-blocking, Esc order (1.6.2)
- ADR-133 decode caps: 64 frames, 4096 px, 64 MiB RGBA per picture (1.6.2)
- ADR-134 retired files; fixed names splash.gif, splash-player.gif, summary.txt, summary-player.txt (1.6.2; 1.6.3 AGENTS 4f)
- ADR-135 test memory cap: systemd-run MemoryMax=8G on the development machine (1.6.2)
- ADR-136 (2.0.1) shared helpers in one module `src/base/` (io, private, pos, names, language); base has no crate:: path
- ADR-137 (2.0.1) cycle breaks: move the shared item, not the caller; no cycle left
- ADR-138 (2.0.1) AssEvent, font_px, BASE_HEIGHT move to ass.rs, not base
- ADR-139 (2.0.1) align_h, align_v move from ui/boxes.rs to base::pos; app must not use ui
- ADR-140 (2.0.1) cycle check: tests/module_cycles.rs with syn (dev-dependency, in lock), production code, top-level grain
- ADR-141 (2.0.1) no re-exports; callers (player included) take the new paths
- ADR-142 (2.0.2) test ffmpeg = pinned bundled 9.0.2 (linux-x86_64 lock), fetched and cached in CI; VIDEOLORE_REQUIRE_FFMPEG=1; version asserted
- ADR-143 (2.0.2) summary fit 15 down to 7 px as built (SUMMARY_MAX/FLOOR); EDITOR and PLAYER tests at 7 px; supersedes ADR-130 values
- ADR-144 (2.0.2) load-independent tests: injected limits; wall-clock product gates best of 5 (src/base/testing.rs); harness waits 120 s; constants pinned
- ADR-145 (2.0.2) hardware probe tests: seconds for working fakes; 30 ms only for the one that never answers
- ADR-146 (2.0.2) package-config job on every push: Windows/macOS --config-only, Linux --stub AppImage; no deb (decided 2026-11-20)
- ADR-147 (2.0.2) notices exit codes 0/1 recorded; other failures keep cargo-about's code
- ADR-148 (2.0.2) cargo-about: pinned action + version assertion; fallback cargo install
- ADR-149 (2.0.2) bundled ffmpeg 9.x: existing tests on pinned build; v20.rs (REQ-144 paused fade, format 8 LUT read, encoder/filter presence); 1.5.x refusal manual
- ADR-150 (2.0.2) 360 camera re-measured on 9.0.2 (median of 3); no gate change
- ADR-151 (2.0.2) inherited decisions: hardware export via ffmpeg; AV1/VP9 in scope; unsigned installers
- ADR-152 (2.0.3, step 0) blur chroma radius clamp; BLUR_MIN_SIDE 8 (render.rs compose_frame)
- ADR-153 (2.0.3) paused still at clip end: trim from the frame that holds the playhead when skip > 0 (REQ-155)
- ADR-154 (2.0.3) measured best fit: harfrust shaping + skrifa metrics (in lock), own wrap with \N and \q2, px 2%..10% H, 0.1 px steps; supersedes the drawing of ADR-087
- ADR-155 (2.0.3) fonts: face bytes on demand (fontdb, cap 64 MiB, LRU 8), missing name -> bundled font written explicitly (REQ-164)
- ADR-156 (2.0.3) installed font drawn by a copy in the work `fonts/` folder, \fn = name ID 1; libass precedence verified on Linux, by hand elsewhere
- ADR-157 (2.0.3) Readability as ASS: None \bord0, Outline 6%, Box = style TextBox BorderStyle 3, 60% opaque, padding 10%; old texts read as None (Q-1 = B)
- ADR-158 (2.0.3) project format 10: TextItem.readability (default None), height (permille, default 200); 2.0.2 refuses 10
- ADR-159 (2.0.3) migration at load (REQ-167) with the 2.0.2 estimate kept (legacy_fit_px); one status message; no restore command
- ADR-160 (2.0.3) text handles: bottom changes height (top fixed), left changes width (right fixed); live box from UI state; one command on release
- ADR-161 (2.0.3) pure src/placement.rs: rect, least move inside, centre keeping, overlaps, partly outside; warnings derived, no auto move except REQ-168
- ADR-162 (2.0.3) stacked hit cycling in UI state (4 px, 1 s); extends ADR-103
- ADR-163 (2.0.3) group move of mixed lines and texts as one MoveSelection command
- ADR-164 (2.0.3, D56) accuracy lever: one-sided onset snap [t-1.0, t-0.2] s from a 10 ms RMS envelope; DTW measured and reverted (not used); larger model not planned (D54); onset snap code removed later (2.0.3 measurements)
- ADR-165 (2.0.3) recognition pieces 120 s with 2 s margins each side; ownership by start (+3.3% audio at Fast)
- ADR-166 (2.0.3, D56) strict alignment test un-ignored with the reworded gate (>= 90% of matched words within 0.3 s, Fast); CI VIDEOLORE_REQUIRE_STT=1 (missing model or fixture fails, never skips; same rule as ADR-142); fixture sha-256 checks; espeak only for make.sh
- ADR-167 (2.0.3) REQ-179 paraphrase message when unmatched lines * 2 >= lines
- ADR-168 (2.0.4, stage 0) no-behaviour split of render.rs and app.rs before the features (QG-16 part); built (render/, app/)
- ADR-169 (2.0.4, stage 1) project format 11 = the one bump (D58); media size and modified time; migration 10 -> 11 in memory
- ADR-170 (2.0.4, stage 1) VTT writer and reader, ASS file export from the same lines; BOM accepted; non-UTF-8 refused
- ADR-171 (2.0.4, stage 1) separate subtitle track: SRT input, mov_text (MP4, MOV) or srt (MKV), not default; MP4 disabled flag via tkhd enabled bit (deviation REQ-182)
- ADR-172 (2.0.4, stage 1) committed OFL faces for Arabic and Devanagari only; CJK from system fonts with a note (D61, Q-3)
- ADR-173 (2.0.4, stage 2) autosave: pure policy, worker writer, copies in private cache named by hash, offer at start
- ADR-174 (2.0.4, stage 2) gestures: UI drag state, one command on release
- ADR-175 (2.0.4, stage 2) removed media: drop snapshots that reference it
- ADR-176 (2.0.4, stage 3) one keyframe widget; copies apart; offer for new clips as status item with a button (UPDATE: no file ui/keyframe_timeline.rs exists)
- ADR-177 (2.0.4, stage 4) hardware check: one-second sample ranges, busy or hanging, pause during jobs, session skip memory
- ADR-178 (2.0.4, stage 5) AV1 and VP9 codecs, presets and encoders; hardware decode probe (hwdec.rs); VP9 hardware rare
- ADR-179 (2.0.4, stage 5) graphics card: Automatic or validated device name; program setting
- ADR-180 (2.0.4, stage 6) codec names plain; BLUR_DIVISOR 8 (an eighth, measured; REQ-222 says a quarter); background cache = raw RGBA file for stills only; Background job priority
- ADR-181 (2.0.4, stage 7) CI: installer job reusable workflow installer-config.yml (path filter, needs); advisories workflow weekly; cpu_message named test
- ADR-182 (2.0.4, stage 7) one window: std File::try_lock lock, heartbeat, inbox folder; no IPC, no unsafe
- ADR-183 (2.0.4, stage 7) macOS menu bar: one allow-listed objc2 call in desktop.rs (amends ADR-034); muda not used; spike result open
- ADR-184 (2.0.4, stage 9) player settings file in config folder; sibling programs by fixed name; Windows NSIS template for "Open with" only
- ADR-185 (2.0.4, stage 14) BUILT: player picture subtitles overlaid in the shared render graph, only when chosen; golden argv kept
- ADR-186 (2.0.4, stage 10 and 13) BUILT: 360 camera roll, ease, 10 updates per second, half-size drag, speed-first export; per-clip paths copy on write (REQ-268); speed-first preset name open
- ADR-187 (2.0.4, stage 10) BUILT: MKV and WebM 360 marker in-house, written in place (spherical.rs; D61, Q-7)
- ADR-188 (2.0.4, stages 10 and 13) BUILT: anchored 360 lines as text items with preset outline or box and karaoke highlight; re-anchor; off-view arrows; amends REQ-112
- ADR-189 (2.0.4, stage 14) BUILT: spatial sound (ambisonics W,Y,Z,X assumed, rotation in Rust, cardioid pair, pre-pass to WAV; preview pre-pass synchronous) (D61, Q-2)
- ADR-190 (2.0.4, stages 11 and 13) BUILT: fade length and sides, crop, Clean audio mix, speed, volume keyframes, per-clip frame
- ADR-191 (2.0.4, stage 11) BUILT: LUT copies raw in project (20 MB each, 64 MiB), project cap 96 MiB, .3dl and .csp readers, strength = blended table (lut.rs with_strength)
- ADR-192 (2.0.4, stage 13) BUILT as a text item with a picture (REQ-287): header read only, path kept, no embedded copy; the plan's 20 items, 8 MiB, 16 Mpx limits NOT in the code (OPEN; 16 Mpx decided in 2.2.0, ADR-219)
- ADR-193 (2.0.4, stage 11) blur levels Low, Medium (default), High; custom aspect 1:4 to 4:1
- ADR-194 (2.0.4, stage 11) BUILT: Enhance speech as speechnorm named "Normalise speech" (D61, Q-6 fallback); no model file
- ADR-195 (2.0.4, stage 8) settings v2 additive fields; cap 64 KiB kept; validation; never in project
- ADR-196 (2.0.4, stage 8) interface text table (English), theme, text size via styles, popup positions with off-screen rule
- ADR-197 (2.0.4, stage 8) accessibility: eframe accesskit feature on Windows, macOS and Linux (D61, Q-4; size open)
- ADR-198 (2.0.4, stage 12) preview cache: clear re-queues, limit by mtime, open project protected
- ADR-199 (2.0.4, stage 12) splash option: pure should_show with last version
- ADR-200 (2.0.4, stage 12) project page: og:image and CSP meta, no header
- ADR-201 (2.0.4, stage 14) BUILT: QG-14..18 (speech fixture, pointer tests, golden argv formats 4 and 5, text split, one box rule, build.rs, sampled UI gates); QG-11..13 measured, not gated
- ADR-202 (2.1.0, built) LUT edge 2..=256, streamed 512 MiB file cap, 256-byte lines; the 65 cap and 16 MiB cap were the refusal cause
- ADR-203 (2.1.0, built) streaming pure parser lut/parse.rs, std f32 parse, u16 table (6 bytes per entry; 256^3 = 100.7 MB); no new crate
- ADR-204 (2.1.0, built) LUTs above 20 MB: path only in the project (REQ-301 option C); REQ-292 choices for path-only
- ADR-205 (2.1.0, built) LutStore: Loading with progress, Ready/Failed; Background job; cancel by generation; LRU by bytes (192 MB); UI never parses
- ADR-206 (2.1.0, built) Work folder copy `lut-<hash16>.cube`, one per content, streamed atomic write with progress (kept by 2.2.0, ADR-214)
- ADR-207 (2.1.0, built) applied-effects order: Timeline.clip_order (format 12); sphere-fixed items on 360 clips are fixed rows; legacy order for format <= 11
- ADR-208 (2.1.0, built) render overlay stage render/overlay.rs for clips whose order differs from legacy; legacy argv unchanged
- ADR-209 (2.1.0, built, Q-2 = A) movable LUT row after the frame composition and fades (whole frame graded)
- ADR-210 (2.1.0, built) format 12 (one bump): clip_order, TextItem.shadow, subtitle style fields, presets EnlargeWord and Typewriter; 2.0.4 refuses 12
- ADR-211 (2.1.0, built; REQ-307 kept) shadow as style Shadow + BackColour for every style; design A box fill in OutlineColour (spike S0d), design B vector box event as fallback (result recorded by the PE)
- ADR-212 (2.1.0, built) highlight colours = Primary (Highlighted) and Secondary (Not yet) of the WordHighlight style
- ADR-213 (2.1.0, built) Typewriter per word with \k letters and hidden future words; Enlarge with a placeholder line and a measured-centre event (word_styles.rs)
- ADR-214 (2.2.0 BUILT, route D) LUT preview: the `.cube` route of 2.1.0 stays (no colour or graph change); background preparation in the store with a percentage moving at least once per second; the graph is built without the LUT row until Ready (unprocessed picture); a newer choice replaces the preparation; A (`.m3d` integer 3.4-3.6 s), B (float 5.1 s), C (binary HALD 0.36 s, trilinear) rejected, measurements kept as the record
- ADR-215 (2.2.0 BUILT) whole-video effects without a stored change: popup, FrameTarget::Whole keyframe routes, SelectCopy, ApplyWholeToNew and offer, widget mark, dashed handles removed; copies kept; format unchanged
- ADR-216 (2.2.0 BUILT) Applied effects rows cached with a revision key; rebuilt on change only (REQ-313)
- ADR-217 (2.2.0 BUILT, measured, met) blur in playback: no change until the real graph is measured; proxy +3.2 ms per frame at 1080p, +1.8 ms at 4K; candidate changes keep the picture
- ADR-218 (2.2.0 BUILT, lever (a) only, NOT MET) Faster on integrated graphics: check kept in full; levers: concurrent strict decode and renders, reuse, hardware strict decode only with a note; fewer samples rejected
- ADR-219 (2.2.0 BUILT) picture limit 16 megapixels: one constant, PNG header check at add, preview and export (REQ-316)
- ADR-220 (2.2.0 BUILT, NOT MET, FROZEN None) REQ-177 speech regions: src/speech_regions.rs (pure); 81-setting grid declared in advance; selection on the reference only; frozen; one check on the second clip (90%)
- ADR-221 (2.2.0 PLAN; SUPERSEDED by ADR-222) project format stays 12
- ADR-222 (2.2.0 BUILT) project format 13: order of all rows and text maximum; 2.1.0 and older refuse it (REQ-029); 12 read with defaults
- ADR-223 (2.2.0 BUILT, deviation: new rows keep their default place) per-clip full order of the picture and sound groups; default = 2.1.0 build; 2.1.0 orders expanded with their movable rows on top; new rows and copies on top of their group
- ADR-224 (2.2.0 BUILT, three builds) render steps for a free order (picture layer RGBA; pair formulations; default keeps the legacy argv)
- ADR-225 (2.2.0 BUILT) sound group chain (Volume, Clean audio; dry branch; audio fades fixed after the group; Speed first)
- ADR-226 (2.2.0 BUILT, 180 px minimum wins at 1280 x 800) Applied column beside the picture; share 15%, min 180 px; additive settings field applied_share
- ADR-227 (2.2.0 BUILT, only the cap changed) text size = 2.0.3 best fit with the cap as the maximum; block centred; old texts unchanged
- ADR-228 (2.2.0 BUILT, D64) TextItem.max_size permille 20..500; empty = default cap 100
- ADR-229 (2.2.0 BUILT) pure order model order_chain.rs (groups, defaults, expansion, moves; no render, app or ui)
- ADR-230 (2.3.0 PLAN) project format 14: Clip.overlaps, Timeline.next_overlap_id; Clip.crop not written; 13 and older refuse 14 (TooNew)
- ADR-231 (2.3.0 PLAN) overlap model pure src/overlap.rs; offsets from the clip start (ripple moves them); cut, split, removals, speed rescale
- ADR-232 (2.3.0 PLAN) three render paths: legacy argv (overlaps after the concat), step builder (Over step), preview; motion once with overlay eof_action=pass; box cover (MOV) or stretch (GIF, PNG) only when reshaped
- ADR-233 (2.3.0 PLAN) alpha: software decoding for overlay files (preview hwdec skips them); size from PNG or GIF header or ffprobe; 16 Mpx at add and at use
- ADR-234 (2.3.0 PLAN) removal warning (REQ-329): removals computed before the edit; one popup holds the pending command; one undo step
- ADR-235 (2.3.0 PLAN, BUG FIX) Enlarge word: the enlarged word is its own event at its measured centre (measure.rs widths, baseline kept); cause: the word grows inside the \an5 \pos line
- ADR-236 (2.3.0 PLAN) Change aspect per clip (REQ-326): no storage change, UI only (popups.rs forced Fit::Height); default Fit on width and black
- ADR-237 (2.3.0 PLAN) Volume and Change speed as effects: storage unchanged; Volume keys row and Speed row removed; Change speed to all clips as copies
- ADR-238 (2.3.0 PLAN) Crop removed (REQ-283): Step::Crop, Row::Crop, Crop row, writer; 13 crops dropped with the count
- ADR-239 (2.3.0 PLAN) Add media (REQ-324): the import action, second button
- ADR-240 (2.3.0 PLAN) Never removed (REQ-295): lenient_splash maps "Never" to OncePerVersion explicitly (default would be Always)
- ADR-241 (2.3.0 PLAN) 13 to 14 in-memory conversion (src/project/v13.rs): crop dropped, picture items under a clip become overlaps, order rows rewritten; Row::Overlap(u64) "overlap:<id>"; one status line
- ADR-242 (2.3.0, DECIDED Andrea Q-A) one shared overlap lane, sub-rows (greedy), 3 rows visible then scroll; ends dragged on the lane
- ADR-243 (2.3.0, DECIDED Andrea Q-C) overlay files above 1920 x 1080: half size in the preview only, full size in the export; preview and export differ for them (REQ-325, REQ-331 wording amended by the PM)
- Q-A/Q-B/Q-C (1.6.0) closed 2026-11-19 (icon artwork shared; splash GIF kept; no splash with a path argument); Q-E closed 2026-11-20 (splash pictures are Andrea's own work, MIT; recorded in packaging/notices/third-party.txt).

## Component map (short; roles and versions in ai-docs/architecture-components.md)
- Programs: main.rs -> lib.rs (real_app, native_options, prepare_desktop); player/src/main.rs -> player lib.
- Interface: ui (+ actions, boxes, effects, export_dialog, frame_handles, frame_popups, icons, keymap, layout, lut_popup, markers, media, menu, modals, picture, placement, popups, ruler, settings, splash_ui, splash_window, status, strings, text_popup, theme, timeline, toolbar, view_menu, zoom, applied_effects), dialogs, splash. No ui/keyframe_timeline.rs (ADR-176).
- Application: app (AppState, Command, dispatch, tick; submodules command, dispatch_*, effects, export_flow, frame, lut, applied, offer (removed in 2.2.0), recovery, sphere, stops, subs_create, subs_import, texts, tick); player app, subs, settings, playlist.
- Engine (processes, threads): export (+ export::choice), outcheck, hwenc, hwdec, preview, proxy, stt, jobs, ffmpeg, scratch, autosave, instance, siblings, spatial (pre-pass runner).
- Rendering (pure): render (args, graph, inputs, plan, segments, overlay; golden tests), format, ass, camera, frame, keyframes, lut, spherical, picture (PNG header, fit in box), spatial (rotation, cardioid pair, WAV writer), effect_order, word_styles.
- LUT (2.1.0): lut/ (mod, parse, store); 2.2.0 changes only the store's preparation (phases, percentage, peek for the graph builder; route D).
- Model (pure): project (+ project::v9), timeline (speed, clip_aspects), music, subtitles, text (timing, layout, events), placement (padded_line_rect, least move, warnings), measure, selection, history, media, srt (VTT and SRT), align; 2.2.0 adds speech_regions.rs (pure, REQ-177).
- Base: time, settings, machine, fonts, notices, packaging, desktop (macOS menu bar is one unsafe site), base (io, private, pos, names, language).
- Build: build.rs (splash summary check).
- 2.2.0 BUILT order and layout: order_chain.rs (new, pure), render/steps.rs (new, step builder; legacy path for default orders), project format 13 (TextItem.max_size), text/layout.rs cap, ui/layout.rs splitter and settings applied_share, ui/applied_effects.rs two groups.
- 2.2.0 PLAN changes (roles and flows in ai-docs/architecture-components.md § 2.2.0; plan § 4): lut/m3d.rs (new), lut/store.rs (name and writer), render/graph.rs (name; blur only if measured), app/applied.rs and ui/applied_effects.rs (rows cache, no mark), whole-video routes in timeline, app/effects, app/command, app/frame, ui/frame_popups, ui/popups, ui/frame_handles, ui/strings (removed), app/offer.rs (deleted), picture.rs (16 Mpx), speech_regions.rs (new), align.rs (retime), outcheck.rs and export (Faster, if measured).

## Data flow summary (short; full in components and sub-documents)
UI event -> AppState::dispatch -> Edit + history -> FrameKey -> latest-only still; export: render plan -> export worker -> ffmpeg via scheduler -> rename; spatial export: pre-pass WAV -> graph input in place of the clip's sound; preview: synchronous spatial pre-pass before the audio process; picture item: looped still overlay before subtitles; subtitles: extraction -> STT thread -> lines -> one undo step; start-up: detect machine -> cleanup -> settings decision -> budget; 2.1.0 LUT: choose -> store Loading (background job, progress) -> Ready -> Work copy by hash -> lut3d; 2.2.0 LUT (route D): preparation job (read, parse, Work copy `.cube` once, progress) -> Ready -> lut3d; not Ready: graph without the row; applied commands -> revision -> rows rebuilt once; REQ-177: recogniser samples -> speech regions -> placement per cue -> word times.

## NFR targets (summary; full list in ai-docs/architecture-nfr.md)
- GATE processes <= level cap (3 / 3 / clamp(L/2,3,6)); 20 imports + export + preview test at L=1,8,16.
- GATE UI frame <= 200 ms during export + STT (and camera, boxes, texts, 4K/8K tests).
- GATE memory: <= 200 MB more peak for long vs short source of the same resolution.
- GATE Best quality export argv byte-identical to 0.3.0 (golden); projects without new features unchanged (2.0.4: format 4 and 5 golden argv files; picture subtitle overlay only when chosen).
- QG-01 recommended export and Fast subtitles <= 0.2.2 +5%; start-up <= 0.2.2 + 0.5 s.
- QG-05 manual hardware matrix (e <= 3 VMAF below, f <= 1.5x size, g >= 40% faster FAILED on Intel iGPU, recorded).
- Coverage >= 85% line; feature coverage 100%.
- Platforms: Linux x86_64 glibc 2.39 max; STT needs AVX2+FMA+F16C+BMI2 on x86_64.
- Estimates: installers 70–130 MB; probe ~6.4 s idle, up to 10 s loaded; STT speeds per model.
- 2.0.1 (REQ-151..153): test-suite time for the cycle check 1.7 s measured in 2.0.4 (estimate was 1 to 3 s).
- 2.0.2 (QG-07..10): bundled ffmpeg 9.x checks; wall-clock product gates best of 5; notices exit 0/1; cargo-about 0.9.3 asserted; package-config job; pinned ffmpeg cache ~173 MB unpacked each.
- 2.0.3 (REQ-155..179): text fit cached per (text, font, box, picture); piece margins +3.3% audio at Fast (QG-01 +5%); blur fill no longer fails on small pictures; REQ-177 gate: >= 90% of words matched and every matched word within 0.3 s at Fast (reworded D56).
- 2.0.4 (built, REQ-180..299; full list in ai-docs/architecture-nfr.md § 2.0.4): golden argv for projects without new features (GATE); UI frame <= 200 ms with popups and autosave (GATE); check renders <= 3 one-second ranges (GATE REQ-206); first playback frame <= 200 ms with 10 imports (GATE REQ-224); camera <= 10 updates per second and half-size drag (GATE REQ-266/267); LUT strength and .3dl/.csp by fixture (GATE REQ-289..291); speed 50-200% (GATE REQ-281); volume keyframes (GATE REQ-280); spatial pre-pass measured 69x real time (debug); estimates for cache enforcement and project load at 96 MiB; QG-11..13 measured, not gated.
- 2.1.0 (built; REQ-300..311; full in ai-docs/architecture-nfr.md § 2.1.0): LUT 256^3 parse <= 6 s, 144^3 <= 2 s and Work copy 256^3 write <= 3 s (ESTIMATES); UI frame <= 200 ms while a LUT loads (GATE QG-19); LUT tables <= 192 MB cached; event caps 2000/20000 kept; legacy argv unchanged for projects without a changed order (GATE QG-21).
- 2.2.0 BUILT order and text (REQ-317..323; measured: REQ-315 8.7% of the 20% target, REQ-177 43% of 90%, both NOT MET): default argv identical (GATE); non-default orders about +1 filter per row, 10% per frame with six extra steps (ASSUMPTION); widget 0 builds over 60 frames (GATE); column 15%, min 180 px (GATE); text maximum drawn exactly (GATE); format 13 round trip and 12 read (GATE).
- 2.3.0 PLAN (full in ai-docs/architecture-plan-2.3.0.md § 9): GATE golden argv for projects without overlaps, Volume keys, speeds, crops; format 14 round trip and 13 conversion; alpha fixtures (preview and export transparent); REQ-328 once then transparent; 16 Mpx for overlaps; REQ-329 one popup; Enlarge centre within 2% of the picture height; "Never" maps to Once; UI frame 200 ms; ESTIMATE overlay cost (+10% preview frame, export at most 1.5x).
- 2.2.0 PLAN (REQ-312..316, REQ-177; full in ai-docs/architecture-nfr.md § 2.2.0): LUT start about 12 s not gated (`.cube`, measured 12.4-13.0 s at 256 points); LUT percentage at least once per second (GATE); UI response <= 200 ms while preparing (GATE); unprocessed fallback argv (GATE); preview = export colours (GATE 2 levels); rows 0 builds over 60 frames (GATE); blur measured first (proxy +3.2 ms per frame 1080p); Faster >= 20% quicker on the Iris (ASSUMPTION, measured); 16 Mpx exact (GATE); REQ-177 >= 90% within 0.3 s on two clips (GATE).

## Security model (summary; full in ai-docs/architecture-security.md)
- Threats: malicious media and project/settings JSON, PATH hijack, tampered downloads, ffmpeg/ASS/ffconcat injection, desktop entry injection, hostile drivers (contained).
- No shell; OsString args; `file:` prefix; protocol whitelist file,pipe; cwd only private 0700 folder.
- Bounded reads (project 96 MiB since 2.0.4, settings 64 KiB, track text 16 MiB, subtitle files 16 MiB), regular-file checks (FIFO refused), strict validation, TooNew refused.
- unsafe forbidden crate-wide except three reviewed sites: whisper abort callback (stt.rs), macOS odoc handler and macOS menu call (desktop.rs; ADR-183, ADR-034 amended); no network except consented model download (https, sha-256, size); player has no network code (CI).
- Unsigned installers (decided); no secrets; no telemetry; cargo-deny + cargo-about + pinned archives; CI least privilege, SHA-pinned actions; weekly advisories read-only.
- 2.0.4: new untrusted inputs bounded before use (VTT and SRT, autosave copies, inbox, player settings, LUT copies and .3dl/.csp, camera values, device names, picture PNG header, speed and volume keyframes); sibling programs by fixed name without shell; spatial WAV private and hashed; cache clean only own names; page CSP.
- 2.0.1: capped reads, atomic writes and private folders in base::io and base::private (REQ-057, REQ-059).
- 2.0.3: font bytes read from the system font database only; project keeps names, never paths; font copies capped (64 MiB) and sha-named; no unsafe in videolore for skrifa/harfrust; CI VIDEOLORE_REQUIRE_STT=1 with fixture sha-256 checks (ADR-166).
- 2.1.0 (built): LUT path read as a streamed regular file (512 MiB, 256-byte lines, exactly n^3 entries); Work copy named by content hash; no user text in a filter; shadow and colour values validated; order ids validated on load (ai-docs/architecture-security.md § 2.1.0).
- 2.3.0 PLAN (ai-docs/architecture-plan-2.3.0.md § 10): overlay files untrusted: size from bounded header or ffprobe before decoding, 16 Mpx refused, paths never in filter text, numbers only in expressions; format 14 input validated, damaged overlaps dropped with a count; no new crate, no unsafe, no network; overlay files not copied; hardware decoders never see overlay files.
- 2.2.0 PLAN: LUT Work copy unchanged (`.cube`, hash name, no user text); preparation job through the bounded reader; picture header limit 16 Mpx (u64 product); REQ-177 second clip committed with sha-256 (ai-docs/architecture-security.md § 2.2.0).

## Crate shortlist (verified against Cargo.lock 2026-11-21; Cargo.toml authoritative)
runtime (root, always): eframe 0.37.0 (glow, default_fonts, wayland, x11, accesskit; egui 0.37.0), fontdb 0.24.0 (1.3.0), image 0.25.10 (gif only, no png feature), rfd 0.18.0 (xdg-portal), serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0, sha2 0.11.1, sysinfo 0.40.4 (system feature), skrifa 0.44.0 and harfrust 0.12.0 (direct since 2.0.3; in lock through epaint).
runtime optional: cpal 0.18.2 (feature audio, default); whisper-rs 0.16.0 (feature stt, default; Unlicense); ureq 3.5.2 (feature stt, rustls, default-features off).
macOS only: objc2 0.6.5; objc2-foundation 0.3.2 (minimal features); objc2-app-kit 0.3.2 (minimal features; menu bar, ADR-183; new direct dependency 2.0.4); winit 0.30.13 (default-features off; direct dependency to switch off the default menu, ADR-024; same version as eframe's).
transitive of note: accesskit_winit 0.34.2 (eframe accesskit feature), fontconfig-parser 0.5.8, memmap2 0.9.12, roxmltree 0.20.0 (via fontdb), rustls/ring (ureq).
dev: syn 2 (features full, visit; MIT OR Apache-2.0) for tests/module_cycles.rs (2.0.1); no other dev crate added in 2.0.4.
2.0.4 built: no muda; no interprocess or single-instance crate; std File::try_lock (no crate); no ambisonic library; no image png feature; no crate for VTT, ASS, LUT readers, MKV marker, speed, spatial sound, picture items.
2.1.0 built and 2.2.0 planned: no new crate (LUT parser and writer, speech regions, rows cache std only; `haldclut` is an ffmpeg filter, not a crate).
Fonts (bundled non-Rust): Noto Sans Bold (OFL-1.1), Noto Sans Arabic Bold and Noto Sans Devanagari Bold (OFL-1.1, 2.0.4), Phosphor Regular (MIT). CJK from system fonts (D61).
tools (CI only, not linked): cargo-packager 0.11.8, appimagetool (packaging/tools.lock), cargo-deny 0.20.2, cargo-about 0.9.3 (prebuilt via pinned install action), cargo-llvm-cov, optional qemu-user. espeak-ng: not installed in CI since 2.0.4 (speech fixture committed; make.sh only regenerates it).
bundled non-Rust: ffmpeg/ffprobe GPL builds (9.0.2 pinned: BtbN linux64-gpl and win64-gpl-shared, Riedl macOS arm64; lock in packaging/ffmpeg.lock), splash pictures (own work, MIT, recorded), Downloaded after consent: whisper models ggml-tiny-q5_1, ggml-small-q5_1, ggml-large-v3-turbo-q5_0 (MIT).
rejected across versions: egui-phosphor, egui-pop-phosphor, egui_extras, resvg (MPL), Font Awesome (CC BY), font-kit, fontique, muda, egui_dock/egui_tiles, egui_kittest (fallback only), srtparse/subparse/srtlib, strsim, ab_glyph/fontdue, matroska/mp4 demuxers, candle-whisper, vosk, reqwest, native-tls, toml/ron, num_cpus, raw-cpuid, smithay-client-toolkit, cargo-nextest (as runner), interprocess and single-instance crates (ADR-182), fonttools (build tool; committed subset, ADR-172), hound (WAV written by hand), an ambisonic library (ADR-189), rustybuzz, ttf-parser (not used in the tree), arnndn model (licence not checked; not bundled), a LUT or HDR image crate for the LUT route (2.2.0 not needed: the table is written by hand, ADR-214).

## Risks and open items (summary; full in ai-docs/architecture-risks.md)
- [2.0.2 plan] wall-clock test gates on a busy machine (best of 5, ADR-144); pinned test ffmpeg is Linux-only in the suite; format 8 fixture hand-written (risks: ai-docs/architecture-risks.md).
- [2.0.3 decided 2026-11-20] Q-1 = B: old texts lose their 8% black outline on opening (Readability None; the one-time REQ-167 message names the outline removal). Q-2 = A: old texts change size on opening, one count message, no restore command.
- [2.0.3, D56] REQ-177 reworded by Andrea: >= 90% of the matched words within 0.3 s at Fast after the onset snap; matched share reported only.
- [open] QG-05 g failed on Intel iGPU; re-baseline or cheaper check (Future work).
- [open] CI time after the 1.6.1 change measured in 2.0.4 only as estimate; first CI run of 2.0.4 reports the times.
- [2.0.4 built, OPEN] picture item limits (ADR-192): 16 Mpx decided and planned in 2.2.0 (ADR-219); the count (20) and size (8 MiB) limits are not planned (business 2.2.0: rejected).
- [2.0.4 built, OPEN] speed-first 360 preset (Q-5): name not in the notes (ADR-186); muda spike result (ADR-183) not in the notes; AccessKit size (ADR-197, Q-4) not measured in the notes; Arabic and Devanagari OFL notices to confirm (PE).
- [2.0.4 built, TECHNICAL assumption] spatial sound: four channels read as W, Y, Z, X, SN3D taken as given; roll not applied to the pair; the preview waits for the pre-pass (clips over 10 min keep recorded sound); real-time CPU on weak hardware unmeasured.
- [2.0.4 built] picture subtitles: none on 360 clips; a subtitle on screen at a run start appears at its next change; no drag. Picture items: none on the 360 export; text effects do not apply.
- [2.0.4 built] Enhance speech is speechnorm, not a neural model; the blurred fill is an eighth (REQ-222 says a quarter; the product decision is open with the PM); LUT copies raise project size (cap 96 MiB).
- [closed D61] Q-1 player under "Open with" only; Q-2 cardioid stereo; Q-3 no bundled CJK font (system fonts, note when missing); Q-4 AccessKit on three systems; Q-5 fastest preset within 1.5x Best quality size; Q-6 "Normalise speech"; Q-7 in-house MKV and WebM marker.
- [1.6.x resolved] splash pictures' origin and licence recorded (own work, MIT; since 1.6.3).
- [2.1.0 built, decided 2026-11-21] Q-1 = B (edge 256, 512 MiB cap, progress and background load); Q-2 = A (whole frame graded); Q-3 = A (sphere-fixed items are fixed rows); REQ-307 kept as written.
- [2.2.0 BUILT, route D; open decisions for Andrea: REQ-315 target or open; REQ-177 planned or new method (plan section 14)] LUT preview keeps the `.cube` route (about 12 s to the first graded frame, not gated); progress, responsiveness and the unprocessed fallback are the 2.2.0 gates. Rejected alternatives with measurements: ADR-214 (`.m3d` text is what ffmpeg 9.0.2 reads, not binary).
- [2.3.0 PLAN] no open SEVERE question: Q-A lane (ADR-242), Q-B speed rescale (ADR-231), Q-C half-size preview above 1920x1080 (ADR-243) answered by Andrea (plan § 13). Risks and future work: plan § 16 (R8 preview differs from export for large files). Enlarge word fix (REQ-333): measured-position risk, ADR-235 fallback.
- [2.2.0 PLAN] the percentage during the ffmpeg start is a time-based estimate (plan § 12).
- [2.2.0 PLAN] blur, Faster and REQ-177 are measured or checked before any change; REQ-177 may fail on the second clip (then planned); whole-video removal changes 2.1.0 users' view (accepted).
- egui releases often change its API; a new stable Rust can add lints: GUI code is kept thin and the code is fixed promptly.

## Human docs index
- The paragraph index of docs/architecture*.md is in ai-docs/architecture-doc-index.md (sub-document, read with grep -n, then sed -n 'A,Bp' on the docs file).
- 2.3.0 human docs (PLAN): docs/architecture-2.3.0-overview.md, docs/architecture-2.3.0-overlap.md; the Enlarge word note in docs/architecture-subtitle-styles.md was corrected (as built vs ADR-235).
- 2.2.0 human docs: docs/architecture-2.2.0-overview.md, docs/architecture-speed.md, docs/architecture-subtitle-accuracy.md, docs/architecture-2.2.0-order.md, docs/architecture-2.2.0-layout.md (new in the 2.2.0 PLAN); changed for the whole-video removal and status: docs/architecture-effects.md, architecture-applied-effects.md, architecture-frame.md, architecture-ui.md, architecture-lut.md, architecture-2.1.0-overview.md, architecture-subtitle-styles.md, architecture.md.
