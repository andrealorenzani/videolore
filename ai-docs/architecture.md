# ai-notes: architecture (index; owner: architect)

<!-- AI-only notes. Owner agent edits only. Dense format OK. Read sub-documents by grep, then sed -n 'A,Bp'. -->

target: 2.0.4 BUILT at the checkpoint of 2026-11-21 (not released; ONE development D57; REQ-180..299: 108 built, 12 not built: 253, 257-260, 268, 273, 280, 281, 286-288; QG-14..18 not built; the designs of the not built items are in ai-docs/roadmap-2_0-checkpoint.md § Architect; plans ai-docs/architecture-plan-2.0.4.md (+ -stages-1.md, -stages-2.md); ADR-168..201 with status in the ADR block; format 11 built (ADR-169); questions Q-1..Q-7 closed by Andrea (D61), see plan § 6) | built 2.0.3 (readable text, placement, accuracy; REQ-155..179 except REQ-177 target open; ADR-152..167; plan ai-docs/architecture-plan-2.0.3.md archived at release; Q-1 = B, Q-2 = A) | built 2.0.2 (verify the release, REQ-154, QG-07..10; plan archived at release; ADR-142..151) | built 2.0.1 (module cycles, REQ-151..153; ADR-136..141; plan ai-docs/architecture-plan-2.0.1.md, archived at release) | 1.6.3 RELEASE (consolidation 2026-11-20; verified against src/ and Cargo.toml; open before release: origin and licence of assets/splash/splash.gif and splash-player.gif, see architecture-risks.md) | built 1.6.2 (shared splash, ADR-128..135) | built 1.6.1 (visible edges, per-clip effects, format 9, CI time) | built 1.6.0 (splash, Apply LUT, workspace with player/, icons; ADR-105..119) | built 1.5.0 (frame: aspect, Scale, Move, keyframes; format 7; ADR-093..104) | 1.4.x (export dialog, clean audio, text width; ADR-080..091) | 1.3.0 (text items, 360 video; ADR-066..079) | 1.4.1 page (ADR-092).
product facts: Rust, edition 2024, rust-version 1.100 (MSRV = latest stable by policy, ADR-012), toolchain latest stable, GPL-3.0-only, author Andrea Lorenzani. Versions 2.0.4 in root Cargo.toml and player/Cargo.toml (both literal, ADR-116); 1.6.3 was the last release.
platforms: Linux x86_64 (Ubuntu 24.04+, Fedora current; AppImage + tar.gz), macOS aarch64 (dmg, ad-hoc sign), Windows x86_64 (NSIS setup). Editor = videolore, player = videolore-player (no speech, no network).
engine: ffmpeg + ffprobe as separate bundled processes (GPL builds, pinned SHA-256, ADR-002/013); whisper.cpp via whisper-rs (opt stt feature, ADR-021/022); egui/eframe 0.37 GUI (ADR-001); std threads + mpsc, no tokio (ADR-004); process scheduler cap 3..6 (ADR-015/029).
state: project JSON format 10 in code (SCHEMA_VERSION 10; loads 1..10; TooNew refused, never written; built 2.0.3: texts readability and height, ADR-158); 2.0.4 built: format 11 (SCHEMA_VERSION 11; media size and date, ADR-169; read cap 96 MiB, ADR-191); settings.json version 2 additive fields (ADR-028/040/044); private work folders 0700 with lock + crash cleanup (ADR-033).
testing: cargo test (no doctests), coverage >= 85% line (llvm-cov, main.rs excluded), feature coverage 100% via `// covers: REQ-NNN`; CI jobs check/msrv/build/package (ADR-035/036/117); 2.0.1 adds the module-cycle test (ADR-140); 2.0.2 (plan) pins the test ffmpeg to the bundled 9.0.2, samples wall-clock gates (best of 5), adds the package-config job (ADR-142..149); 2.0.4 built: every built REQ has a `// covers:` test; golden argv gate for projects without new features (ADR-168); installer job on packaging PRs (reusable workflow installer-config.yml) and a named cpu_message test; weekly advisories workflow (ADR-181).

## ai sub-documents (read only when a task needs them; all owned by architect)
- ai-docs/architecture-adr.md § whole file — L1-584 (584 lines): ADRs ADR-001..201 (ids, context, choice, alternatives, reason; status per ADR; superseded ones one line with replacement); area index at its top. Read with grep -n 'ADR-0NN' then sed.
- ai-docs/architecture-components.md § whole file — L1-106 (106 lines): verified module structure vs src (lib.rs modules, app/export/project/render/settings/ui submodules), import cycles (re-checked 2026-11-20), component roles per module and version, module dependency list, ai data flow for every version, 2.0.4 module map and flows as built (checkpoint 2026-11-21).
- ai-docs/architecture-nfr.md § whole file — L1-110 (110 lines): NFR targets by area: processes, UI responsiveness, preview, export and quality (QG-01, QG-05), memory, platforms and portability, page, 2.0.4 PLAN gates and estimates; GATE vs ESTIMATE marked.
- ai-docs/architecture-security.md § whole file — L1-41 (41 lines): threats, core rules (no shell, file: prefix, protocol whitelist, unsafe policy, downloads, supply chain, CI permissions), security by version 0.2.0..1.6.2.
- ai-docs/architecture-risks.md § whole file — L1-116 (116 lines): open risks by area (release items first), hardware export limits, subtitles, interface, 360, frames/LUT/player/splash, code debt, 2.0.4 plan risks and open questions Q-1..Q-7; resolved items removed.
- ai-docs/architecture-plan-2.0.1.md § whole file — L1-125 (125 lines): plan 2.0.1 (measured cycles, base module layout, move table, cycle breaks, behaviour invariants, cycle check design, ordered PE steps, tests, risks, future work). Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-plan-2.0.2.md § whole file — L1-90 (90 lines): plan 2.0.2 (facts found by reading, components, ADR-142..151 summary, QG-07..10, security, ordered PE steps 1-10, tests, risks, questions Q-1 and Q-2, future work). Archived in the release snapshot after the 2.0.2 release (drop this line then).
- ai-docs/architecture-plan-2.0.4.md § whole file — L1-135 (135 lines): plan 2.0.4 (scope, facts found by reading, approach, components by stage, format 11 fields by stage, stage order and dependencies, cut-back list, questions Q-1..Q-7 (SEVERE, open), risks summary, future work). Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-plan-2.0.4-stages-1.md § whole file — L1-118 (118 lines): plan 2.0.4 stages 0–5 (template per stage: REQs, modules, data, tests per REQ, CI, performance, security, risks, manual items, dependencies, cut-back, ADRs). Archived after release.
- ai-docs/architecture-plan-2.0.4-stages-2.md § whole file — L1-153 (153 lines): plan 2.0.4 stages 6–12 (same template). Archived after release.
- ai-docs/architecture-plan-2.0.3.md § whole file — L1-217 (217 lines): plan 2.0.3 (facts by reading; step 0 blur; batch A REQ-155..167 with measured fit method, fonts, readability, format 10; batch B placement REQ-168..176; batch C accuracy REQ-177..179 with levers and pieces; tests per REQ; ordered PE steps 0-19; questions Q-1, Q-2; future work). Archived in the release snapshot after release (drop this line then).
- ai-docs/architecture-doc-index.md § whole file — L1-293 (293 lines): paragraph index of docs/architecture*.md (one line per heading: `- <file> § <heading> — L<start>-<end> (<n> lines)`); moved here from this file to keep it short; regenerate after each change of a managed doc.
- ai-docs/roadmap-2_0-checkpoint.md § Architect — SHARED temporary checkpoint (delete when 2.0.4 is released; each agent edits only its own section): the architect's stage assumptions, plan files, ADR status and updates of 2.0.4, the designs of the not built items, open items, what remains for the architect.
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
- ADR-126 CI time: install only missing apt packages, timeouts, prebuilt cargo-about (QG-06) (1.6.1)
- ADR-127 splash 1.6.1 picture = copy of 1.6.0 (superseded by ADR-134 names)
- ADR-128 shared splash: splash::Program EDITOR and PLAYER; ui::splash_window (1.6.2)
- ADR-129 splash assets compiled in from CARGO_MANIFEST_DIR; no build.rs (1.6.2; 1.6.3 fixed names)
- ADR-130 summary fit: no scroll, no clip, 372 px wide (1.6.2; the 14/10 px values superseded by ADR-143)
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
- ADR-141 (2.0.1) no re-exports; callers (player included) take the new paths
- ADR-142 (2.0.2 PLAN) test ffmpeg = pinned bundled 9.0.2 (linux-x86_64 lock), fetched and cached in CI; VIDEOLORE_REQUIRE_FFMPEG=1; version asserted
- ADR-143 (2.0.2 PLAN) summary fit 15 down to 7 px as built (SUMMARY_MAX/FLOOR); EDITOR and PLAYER tests at 7 px; supersedes ADR-130 values
- ADR-144 (2.0.2 PLAN) load-independent tests: injected limits; wall-clock product gates best of 5 (src/base/testing.rs); harness waits 120 s; constants pinned
- ADR-145 (2.0.2 PLAN) hardware probe tests: seconds for working fakes; 30 ms only for the one that never answers
- ADR-146 (2.0.2 PLAN) package-config job on every push: Windows/macOS --config-only, Linux --stub AppImage; no deb (decided 2026-11-20)
- ADR-147 (2.0.2 PLAN) notices exit codes 0/1 recorded; other failures keep cargo-about's code
- ADR-148 (2.0.2 PLAN) cargo-about: pinned action + version assertion; fallback cargo install
- ADR-149 (2.0.2 PLAN) bundled ffmpeg 9.x: existing tests on pinned build; v20.rs (REQ-144 paused fade, format 8 LUT read, encoder/filter presence); 1.5.x refusal manual
- ADR-150 (2.0.2 PLAN) 360 camera re-measured on 9.0.2 (median of 3); no gate change
- ADR-151 (2.0.2) inherited decisions: hardware export via ffmpeg; AV1/VP9 in scope; unsigned installers
- ADR-152 (2.0.3 PLAN, step 0) blur chroma radius clamp; BLUR_MIN_SIDE 8 (render.rs compose_frame)
- ADR-153 (2.0.3 PLAN) paused still at clip end: trim from the frame that holds the playhead when skip > 0 (REQ-155)
- ADR-154 (2.0.3 PLAN) measured best fit: harfrust shaping + skrifa metrics (in lock), own wrap with \N and \q2, px 2%..10% H, 0.1 px steps; supersedes the drawing of ADR-087
- ADR-155 (2.0.3 PLAN) fonts: face bytes on demand (fontdb, cap 64 MiB, LRU 8), missing name -> bundled font written explicitly (REQ-164)
- ADR-156 (2.0.3 PLAN) installed font drawn by a copy in the work `fonts/` folder, \fn = name ID 1; libass precedence verified on Linux, by hand elsewhere
- ADR-157 (2.0.3 PLAN) Readability as ASS: None \bord0, Outline 6%, Box = style TextBox BorderStyle 3, 60% opaque, padding 10%; old texts read as None (Q-1 = B)
- ADR-158 (2.0.3 PLAN) project format 10: TextItem.readability (default None), height (permille, default 200); 2.0.2 refuses 10
- ADR-159 (2.0.3 PLAN) migration at load (REQ-167) with the 2.0.2 estimate kept (legacy_fit_px); one status message naming the size-change count and the outline removal; no restore command
- ADR-160 (2.0.3 PLAN) text handles: bottom changes height (top fixed), left changes width (right fixed); live box from UI state; one command on release
- ADR-161 (2.0.3 PLAN) pure src/placement.rs: rect, least move inside, centre keeping, overlaps, partly outside; warnings derived, no auto move except REQ-168
- ADR-162 (2.0.3 PLAN) stacked hit cycling in UI state (4 px, 1 s); extends ADR-103
- ADR-163 (2.0.3 PLAN) group move of mixed lines and texts as one MoveSelection command
- ADR-164 (2.0.3 PLAN, D56) accuracy lever: one-sided onset snap [t-1.0, t-0.2] s from a 10 ms RMS envelope; DTW measured and reverted (not used); larger model not planned (D54)
- ADR-165 (2.0.3 PLAN) recognition pieces 120 s with 2 s margins each side; ownership by start (+3.3% audio at Fast)
- ADR-166 (2.0.3 PLAN, D56) strict alignment test un-ignored with the reworded gate (>= 90% of matched words within 0.3 s, Fast); CI VIDEOLORE_REQUIRE_STT=1; fixture sha-256 checks; espeak only for make.sh
- ADR-167 (2.0.3 PLAN) REQ-179 paraphrase message when unmatched lines * 2 >= lines (existing align result)
- ADR-168..201 (2.0.4) status 2026-11-21: built as written except NOT BUILT 185, 188, 189, 192; PARTLY 186, 190, 201; UPDATED 176, 180, 181, 183, 184, 187, 191, 194 (text in ADR-NNN lines below and in the ADR block)
- ADR-168 (2.0.4 PLAN, stage 0) no-behaviour split of render.rs and app.rs before the features (QG-16 part)
- ADR-169 (2.0.4 PLAN, stage 1) project format 11 = the one bump (D58); media size and modified time; migration 10 -> 11 in memory
- ADR-170 (2.0.4 PLAN, stage 1) VTT writer and reader, ASS file export from the same lines; BOM accepted; non-UTF-8 refused
- ADR-171 (2.0.4 PLAN, stage 1) separate subtitle track: SRT input, mov_text (MP4, MOV) or srt (MKV), not default
- ADR-172 (2.0.4 PLAN, stage 1) committed OFL faces for Arabic and Devanagari only; CJK from system fonts with a note (D61, Q-3); UI fallback on first open of the subtitle list
- ADR-173 (2.0.4 PLAN, stage 2) autosave: pure policy, worker writer, copies in private cache named by hash, offer at start
- ADR-174 (2.0.4 PLAN, stage 2) gestures: UI drag state, one command on release
- ADR-175 (2.0.4 PLAN, stage 2) removed media: drop snapshots that reference it
- ADR-176 (2.0.4 PLAN, stage 3) one keyframe widget; copies apart; offer for new clips as status item with a button
- ADR-177 (2.0.4 PLAN, stage 4) hardware check: one-second sample ranges, busy or hanging, pause during jobs, session skip memory
- ADR-178 (2.0.4 PLAN, stage 5) AV1 and VP9 codecs, presets and encoders; hardware decode probe (hwdec.rs); VP9 hardware rare
- ADR-179 (2.0.4 PLAN, stage 5) graphics card: Automatic or validated device name; program setting
- ADR-180 (2.0.4 BUILT, stage 6) codec names plain; blurred fill BLUR_DIVISOR 8 (an eighth, measured; REQ-222 says a quarter); background cache = raw RGBA file for stills only; Background job priority
- ADR-181 (2.0.4 PLAN, stage 7) CI: installer job path filter and needs; advisories workflow weekly; cpu_message named test
- ADR-182 (2.0.4 PLAN, stage 7) one window: std File::try_lock lock, heartbeat, inbox folder; no IPC, no unsafe
- ADR-183 (2.0.4 BUILT, stage 7) macOS menu bar: one allow-listed objc2 call built in desktop.rs (amends ADR-034); muda not used; spike result open
- ADR-184 (2.0.4 PLAN, stage 9) player settings file in config folder; sibling programs by fixed name; editor opens media argument
- ADR-185 (2.0.4 NOT BUILT, stage 9) player picture subtitles overlaid in the playback process
- ADR-186 (2.0.4 PARTLY BUILT, stage 10; per-clip paths not built) 360 camera per clip, roll, ease, 10 updates per second, half-size drag, speed-first export
- ADR-187 (2.0.4 BUILT, stage 10) MKV and WebM 360 marker: in-house Matroska Projection writer, written in place (ffmpeg's Void padding shrinks as Tracks grows; spherical.rs) (D61, Q-7)
- ADR-188 (2.0.4 NOT BUILT, stage 10) 360 subtitles keep preset box and highlight in the sphere-frame events
- ADR-189 (2.0.4 NOT BUILT, stage 10) spatial sound: first-order ACN/SN3D rotated in Rust, fixed cardioid pair to stereo (D61, Q-2); cut-back only if not real time
- ADR-190 (2.0.4 PARTLY BUILT, stage 11) effects arithmetic: built fade length, crop, Clean audio mix; not built speed, clip frame padding, volume keys
- ADR-191 (2.0.4 BUILT, stage 11) LUT copies raw in project (20 MB each, 64 MiB), project cap 96 MiB, .3dl and .csp readers, strength = blended table (lut.rs with_strength)
- ADR-192 (2.0.4 NOT BUILT, stage 11) picture items: PNG only, decode limits before pixels, embedded copy, 20 items
- ADR-193 (2.0.4 PLAN, stage 11) blur levels Low, Medium (default), High; custom aspect 1:4 to 4:1
- ADR-194 (2.0.4 BUILT, stage 11) Enhance speech: speechnorm named "Normalise speech", no model file, no arnndn (D61, Q-6 fallback)
- ADR-195 (2.0.4 PLAN, stage 8) settings v2 additive fields; cap 64 KiB kept; validation; never in project
- ADR-196 (2.0.4 PLAN, stage 8) interface text table (English), theme, text size via styles, popup positions with off-screen rule
- ADR-197 (2.0.4 PLAN, stage 8) accessibility: eframe accesskit feature on Windows, macOS and Linux (D61, Q-4)
- ADR-198 (2.0.4 PLAN, stage 12) preview cache: clear re-queues, limit by mtime, open project protected
- ADR-199 (2.0.4 PLAN, stage 12) splash option: pure should_show with last version
- ADR-200 (2.0.4 PLAN, stage 12) project page: og:image and CSP meta, no header
- ADR-201 (2.0.4 PLAN, stage 12) QG-14..18 items; QG-11..13 measured, not gated
- Q-A/Q-B/Q-C (1.6.0) closed 2026-11-19 (icon artwork shared; splash GIF kept; no splash with a path argument); Q-E closed 2026-11-20 (splash pictures are Andrea's own work, MIT; recorded in packaging/notices/third-party.txt).

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
- 2.0.3 PLAN: NEW pure src/measure.rs (font metrics, shaping, wrap, best fit) and src/placement.rs (box rules, warnings); fonts.rs gains face bytes and copies into the work fonts/ folder; text.rs gets readability and height; ass.rs gets the Readability styles; project format 10; app gets layouts cache and MoveSelection/MoveInside; ui gets handles, live box, hit cycling, warnings; stt gets 10 ms energy envelope and 2 s piece margins; render gets the paused-trim and blur clamp. Layering: measure has no crate dependency; placement uses measure, text, subtitles, base::pos.
- 2.0.4 BUILT (checkpoint 2026-11-21): NEW modules autosave, instance, hwdec, siblings, ui/strings, ui/theme, player settings.rs and playlist.rs; render/ and app/ split. No file ui/keyframe_timeline.rs (ADR-176 update). NOT BUILT: pictures (REQ-287). Map and flows: ai-docs/architecture-components.md § 2.0.4 PLAN.
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
- 2.0.2 PLAN (QG-07..10; full in ai-docs/architecture-nfr.md): bundled ffmpeg 9.x checks on the pinned build; no product limit changed; wall-clock product gates best of 5; notices exit 0/1; cargo-about 0.9.3 asserted; package-config job; cache of the pinned ffmpeg (about 100 MB, estimate); camera timing re-measured on 9.0.2 (median of 3).
- 2.0.3 PLAN (REQ-155..179; full in ai-docs/architecture-nfr.md): text fit cached per (text, font, box, picture) and never per frame; font reads off the UI path; piece margins +3.3% audio at Fast (QG-01 +5%); blur fill no longer fails on small pictures (CI gate); REQ-177 gate: >= 90% of words matched and every matched word within 0.3 s on the reference clip at Fast.
- 2.0.4 (built 2026-11-21, REQ-180..299 except the 12 not built; full list in ai-docs/architecture-nfr.md § 2.0.4): golden argv for projects without new features (GATE); UI frame ≤ 200 ms with new popups and autosave (GATE); check renders ≤ 3 one-second ranges (GATE REQ-206); first playback frame ≤ 200 ms with 10 imports (GATE REQ-224); camera ≤ 10 updates per second and half-size drag (GATE REQ-266/267); LUT strength and .3dl/.csp by fixture (GATE REQ-289..291); estimates for cache enforcement and project load at 96 MiB; QG-11..13 measured, not gated.

## Security model (summary; full in ai-docs/architecture-security.md)
- Threats: malicious media and project/settings JSON, PATH hijack, tampered downloads, ffmpeg/ASS/ffconcat injection, desktop entry injection, hostile drivers (contained).
- No shell; OsString args; `file:` prefix; protocol whitelist file,pipe; cwd only private 0700 folder.
- Bounded reads (project 16 MiB, settings 64 KiB, track text 16 MiB), regular-file checks (FIFO refused), strict validation, TooNew refused.
- unsafe forbidden crate-wide except whisper abort callback, macOS odoc handler and macOS menu call (2.0.4, ADR-183/034); no network except consented model download (https, sha-256, size); player has no network code (CI).
- Unsigned installers (decided); no secrets; no telemetry; cargo-deny + cargo-about + pinned archives; CI least privilege, SHA-pinned actions.
- 2.0.1: capped reads, atomic writes and private folders move to base::io and base::private unchanged (REQ-057, REQ-059).
- 2.0.3 PLAN: font bytes read from the system font database only; project keeps names, never paths; font copies capped (64 MiB) and sha-named in the work folder; no unsafe in videolore for skrifa/harfrust (they parse untrusted bytes: failure = bundled font, test with truncated bytes); CI VIDEOLORE_REQUIRE_STT=1 with fixture sha-256 checks (ADR-166).
- 2.0.4 built (full in ai-docs/architecture-security.md, last bullet): new untrusted inputs bounded before use (VTT and SRT, autosave copies, one-window inbox, player settings, LUT copies and .3dl/.csp, camera values, device names); sibling programs by fixed name without shell; cache clean only own names; page CSP; one new unsafe call (macOS menu, ADR-183; ADR-034 amended).

## Crate shortlist (verified against Cargo.lock 2026-11-20; Cargo.toml authoritative)
runtime (root, always): eframe 0.37.0 (glow, default_fonts, wayland, x11; egui 0.37.0), fontdb 0.24.0 (1.3.0), image 0.25.10 (gif only, 1.6.0), rfd 0.18.0 (xdg-portal), serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0, sha2 0.11.1, sysinfo 0.40.4 (system feature).
runtime optional: cpal 0.18.2 (feature audio, default); whisper-rs 0.16.0 (feature stt, default; Unlicense); ureq 3.5.2 (feature stt, rustls, default-features off).
macOS only: objc2 0.6.5; objc2-foundation 0.3.2 (minimal features); winit 0.30.13 (default-features off; direct dependency to switch off the default menu, ADR-024; same version as eframe's).
transitive of note: fontconfig-parser 0.5.8, memmap2 0.9.12, roxmltree 0.20.0 (via fontdb), rustls/ring (ureq).
dev (2.0.1 PLAN): syn 2 (features full, visit; MIT OR Apache-2.0; already in Cargo.lock as 2.0.119) for tests/module_cycles.rs; no new package.
2.0.2 PLAN: no new crate; test ffmpeg = bundled 9.0.2 build fetched in CI (not a crate); appimagetool (packaging/tools.lock) also runs in package-config; cargo-about 0.9.3 version asserted.
2.0.3 PLAN: direct dependencies skrifa 0.44.0 (MIT OR Apache-2.0, MSRV 1.85) and harfrust 0.12.0 (MIT, MSRV 1.85), both already in Cargo.lock through epaint 0.37 (no new package, new edges only); fontdb 0.24 `with_face_data` (existing); whisper-rs 0.16 unchanged (no DTW, D56). ttf-parser not used (skrifa chosen). Not added: rustybuzz, ttf-parser, fontdue, ab_glyph.
2.0.4 built (checked by cargo-deny): eframe feature `accesskit` on (accesskit_winit 0.34.2 in the lock); objc2-app-kit 0.3.2 (macOS only, minimal features, new direct dependency, ADR-183); no muda; no image `png` feature (REQ-287 not built; image stays `gif`); std File::try_lock (no crate). Fonts: two committed OFL faces (Arabic, Devanagari); CJK from system fonts (D61). Dev: none new.
not in the tree (planned, dropped): ttf-parser 0.25 dev-dependency of the 1.6.0 shortlist is absent from Cargo.toml and Cargo.lock at 1.6.3 (no test needs it; Future work: decide).
rejected across versions: egui-phosphor, egui-pop-phosphor, egui_extras, resvg (MPL), Font Awesome (CC BY), font-kit, fontique, muda, egui_dock/egui_tiles, egui_kittest (fallback only), srtparse/subparse/srtlib, strsim, ab_glyph/fontdue, matroska/mp4 demuxers, candle-whisper, vosk, reqwest, native-tls, toml/ron, num_cpus, raw-cpuid, smithay-client-toolkit, cargo-nextest (as runner); 2.0.4: interprocess and single-instance crates (IPC not needed, ADR-182), fonttools (build tool; committed subset, ADR-172), hound (WAV written by hand), an ambisonic library (ADR-189); muda not used (ADR-183; its earlier rejection reason and the spike result are not in the notes).
tools (CI only, not linked): cargo-packager 0.11.8, appimagetool (packaging/tools.lock), cargo-deny 0.20.2, cargo-about 0.9.3 (prebuilt via pinned install action), cargo-llvm-cov, espeak-ng (test fixture), optional qemu-user.
bundled non-Rust: ffmpeg/ffprobe GPL builds (9.0.2 pinned: BtbN linux64-gpl and win64-gpl-shared, Riedl macOS arm64; lock in packaging/ffmpeg.lock), Noto Sans Bold (OFL-1.1), Phosphor Regular (MIT), splash pictures (origin open). Downloaded after consent: whisper models ggml-tiny-q5_1, ggml-small-q5_1, ggml-large-v3-turbo-q5_0 (MIT).

## Risks and open items (summary; full in ai-docs/architecture-risks.md)
- [2.0.2 plan] wall-clock test gates on a busy machine (best of 5, ADR-144); pinned test ffmpeg is Linux-only in the suite; format 8 fixture hand-written (risks: ai-docs/architecture-risks.md).
- [2.0.3 decided 2026-11-20] Q-1 = B: old texts lose their 8% black outline on opening (Readability None; the one-time REQ-167 message names the outline removal). Q-2 = A: old texts change size on opening, one count message, no restore command. Plan section 12.
- [2.0.3 plan, D56] REQ-177 reworded by Andrea: >= 90% of the matched words within 0.3 s at Fast after the onset snap; matched share reported only. If the snap does not reach it, Andrea decides (larger model, D54); the PE stops and reports.
- [2.0.3 plan] libass precedence of the copied font (ADR-156) and the libass box padding of BorderStyle 3 are to be confirmed by tests; line-box fill definition (assumption).
- [open] QG-05 g failed on Intel iGPU; re-baseline or cheaper check (Future work).
- [open] CI time after the 1.6.1 change not measured here (estimate only).
- [2.0.1 plan] module cycles: plan in ai-docs/architecture-plan-2.0.1.md; the cycle check is syntactic (macro-made paths and #[path] not followed; #[path] fails the test); oversized files app.rs (about 3000 lines), render.rs (about 3400 lines) remain debt; docs/code-architecture.md must gain the app<->ui pair and the base layer (PE step 8).
- [open] docs/architecture.md is 121 lines (index, with the 2.0.4 pair of documents): no split needed now.
- [2.0.4, closed by D61] Q-1 player under "Open with" only; Q-2 cardioid stereo of ambisonic sound; Q-3 no bundled CJK font (system fonts, note when missing); Q-4 AccessKit on three systems; Q-5 fastest preset within 1.5x Best quality size; Q-6 "Normalise speech" (speechnorm, built); Q-7 in-house MKV and WebM marker (built in place). Open items (2026-11-21): muda spike result, speed-first preset, AccessKit size, real-time spatial sound (REQ-273 not built), Arabic and Devanagari faces licence check (OFL, to confirm in notices). Full risk list: ai-docs/architecture-risks.md § 2.0.4 plan.

## Human docs index
- The paragraph index of docs/architecture*.md is in ai-docs/architecture-doc-index.md (sub-document, read with grep -n, then sed -n 'A,Bp' on the docs file).
