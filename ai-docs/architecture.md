# ai-notes: architecture (index; owner: architect)

<!-- AI-only notes. Owner agent edits only. Read a sub-document by grep, then sed -n 'A,Bp'. -->

state: 2.3.0 released (verified at RELEASE). PLAN 3.1.0 done (2026-11-24, target 3.1.0): REQ-334..340 planned; ADR-244..254 in architecture-adr-3.1.0.md; plan in architecture-plan-3.1.0.md (archived at release); format 15 decided (ADR-244: every save writes 15). No new crate. Previous: 2.3.0 built, 2.2.0 built (REQ-315 NOT MET 8.7 % of 20 %; REQ-177 NOT MET 43 % of 90 %), 2.1.0 built, 2.0.4 built, 2.0.3, 2.0.2, 2.0.1 built, 1.6.3 last release. Current product facts: Rust edition 2024, rust-version 1.100 (ADR-012), latest stable toolchain, GPL-3.0-only, author Andrea Lorenzani. Platforms: Linux x86_64 (Ubuntu 24.04 LTS, current Fedora; AppImage and tar.gz), macOS aarch64 (dmg, ad-hoc seal), Windows x86_64 (NSIS per user). Editor `videolore`, player `videolore-player` (no speech, no network).
engine: ffmpeg and ffprobe as separate bundled processes (GPL builds, pinned SHA-256; ADR-002, 013; test ffmpeg = the pinned build, 142); whisper.cpp through whisper-rs (feature stt, 021, 022); egui/eframe 0.37 GUI (001; AccessKit 197); std threads and mpsc, no tokio (004); process cap 3..6 (015, 029).
state of the format: project JSON format 14 (SCHEMA_VERSION 14; 15 from 3.1.0, ADR-244: loads 1..15, every save writes 15; newer refused, never written; read cap 96 MiB from format 11, 191); settings.json version 2 additive fields (028, 040, 044, 195); private work folders 0700 with lock and crash clean-up (033); one window by lock file and inbox (182); autosave copies in the cache (173).
testing: cargo test (no doctests); coverage ≥ 85 % line (llvm-cov, main.rs excluded; 94.9 % measured at 2.0.4); feature coverage 100 % (`// covers: REQ-NNN`); CI jobs check, msrv, build, package, package-config, installer-config (reusable), advisories (weekly); module-cycle test (140; syn); build.rs checks the splash summaries (QG-17).

## Documents (ai sub-documents; all owned by the architect)
- ai-docs/architecture-adr.md § whole file — L1-254 (254 lines): ADR-001..201 (format: id, version, status, choice, alternatives, reason; area index at the top; superseded ones one line with the replacement; 2.0.4 block at the end). Read with `grep -n 'ADR-0NN' ai-docs/architecture-adr.md`.
- ai-docs/architecture-adr-2.1.0.md § whole file — L1-27 (27 lines): ADR-202..213 (LUT large files, order model, shadows, styles, format 12).
- ai-docs/architecture-adr-2.2.0.md § whole file — L1-35 (35 lines): ADR-214..229 (route D for the LUT; order groups; format 13; column; text maximum; blur; Faster; picture limit; REQ-177 frozen setting).
- ai-docs/architecture-adr-2.3.0.md § whole file — L1-29 (29 lines): ADR-230..243 (format 14, overlap model, render paths, alpha, removal popup, Enlarge fix ADR-235, clip aspect UI, Volume and speed effects, Crop removed, Add media, Never, lane ADR-242, preview size ADR-243).
- ai-docs/architecture-adr-3.1.0.md § whole file — L1-27 (27 lines) (PLAN 3.1.0, planned): ADR-244..254 (format 15 always written, audio track model by ordinal, T2 for a non-first track, proxies keep all audio, Enlarge from base via word_layout, As subtitle track, blur fix, Overlap row, player Audio menu, Subtitles audio argument, Volume and track). Read with `grep -n 'ADR-2NN' ai-docs/architecture-adr-3.1.0.md`.
- ai-docs/architecture-plan-3.1.0.md § whole file — L1-38 (38 lines) (PLAN 3.1.0, planned): components touched, ordered steps 1..10 for the PE with tests per REQ, assumptions A-3.1-1..4, test plan. Archived in the release snapshot at RELEASE (drop this line then).
- ai-docs/architecture-components.md § whole file — L1-68 (68 lines): verified module structure (2.3.0), roles, dependencies and rules, data flows (ai view), threads, planned-then-built modules.
- ai-docs/architecture-nfr.md § whole file — L1-67 (67 lines): NFR targets with status (GATE, ESTIMATE, MEASURED, NOT MET), sizes and limits, security targets, portability, verification.
- ai-docs/architecture-security.md § whole file — L1-46 (46 lines): threat model, core rules (no shell, file: prefix, protocol whitelist, unsafe policy, downloads, supply chain, CI permissions), rules by version (0.2.0 to 2.3.0).
- ai-docs/architecture-risks.md § whole file — L1-87 (87 lines): open decisions and measurements (REQ-177, REQ-315, QG-05, REQ-222, picture item limits, Q-5 name, boxed shadow, Enlarge spike), technical risks by area, debt, plan-era items kept for history.
- ai-docs/architecture-doc-index.md § whole file — L1-193 (193 lines): paragraph index of docs/architecture*.md (regenerate with scripts/doc-index.sh).
- The plans (archived at release; not edited): ai-docs/architecture-plan-2.0.1.md, -2.0.2.md, -2.0.3.md, -2.0.4.md, -2.0.4-stages-1.md, -2.0.4-stages-2.md, -2.1.0.md, -2.2.0.md, -2.3.0.md. After the release they are in `releases/<version> - <name>/docs/ai-docs/`; their index lines are dropped at that point.
- ai-docs/roadmap-2_0-checkpoint.md § Architect — shared temporary checkpoint of the 2.0.4 build (delete when the 2.0.4 release is done; the architect's section only).

## Component map and data flow
Full role list, dependency rules and flows: ai-docs/architecture-components.md. Mermaid diagrams and the module graph: docs/code-architecture.md (PE). Pure modules never import render, app or ui; no module cycle since 2.0.1 (`base/` imports nothing).

## Decisions
One line per decision with its ADR id: docs/architecture-decisions.md. Full records (context, alternatives, reason): ai-docs/architecture-adr.md (area index at its top) and the 2.1.0, 2.2.0 and 2.3.0 ADR files. Format bumps: 2 (0.2.0), 3 (1.2.0), 4 (1.2.6), 5 (1.3.0), 6 (1.4.0), 7 (1.5.0), 8 (1.6.0), 9 (1.6.1), 10 (2.0.3), 11 (2.0.4), 12 (2.1.0), 13 (2.2.0), 14 (2.3.0), 15 (3.1.0 planned, ADR-244).

## NFR targets (summary; full list in ai-docs/architecture-nfr.md)
- GATE: process cap per level; UI frame ≤ 200 ms during export and subtitle work; import of 10 files without blocking (frames under 50 ms); memory does not grow with source size (≤ 200 MB more for a long source); line coverage ≥ 85 %; feature coverage 100 %; golden argv for projects without new features (identical export commands).
- NOT MET (measured): REQ-315 Faster on integrated graphics (8.7 % against 20 %); QG-05 export speed (−14 % to +41 % against 40 %); REQ-177 real-speech matching (43 % against 90 %); moving-camera export speed (about 1/3 real time at 1080p; target half).
- MEASURED: blurred fill 1/8 at 4K 2.9 to 3.1 s (1/4: 3.3 to 3.7 s); spatial pre-pass 69x real time (debug); LUT first graded frame 12 s at 256 points; hardware output check 25 dB per plane; Iris Plus G7 VMAF −1.4, +1.4, +6.7.
- Limits: project read 96 MiB (format 11+); LUT 512 MiB and 256 points (2.1.0); picture and overlay 16 MP; frame longer side 4096 px; keyframes 200 per effect; clip speed 50 to 200 %; text maximum 2 to 50 % (2.2.0).

## Security model (summary; full in ai-docs/architecture-security.md)
Untrusted: media, projects, settings, PATH ffmpeg, downloads, typed text, LUT and overlay files. No shell; `file:` prefix; `-protocol_whitelist file,pipe`; private work folders 0700; bounded reads (16 MiB projects, 96 MiB from format 11, 64 KiB settings); strict validation; unsafe only in three reviewed places (whisper abort callback; macOS open-document handler; macOS menu bar call, ADR-183); network only for the consented speech model download (HTTPS, SHA-256); no telemetry; installers unsigned (decided by Andrea).

## Crate shortlist (Cargo.toml 2.3.0; reviewed for licence GPL-3.0 and maintenance)
eframe 0.37.0 (egui; features accesskit, glow, default_fonts, wayland, x11); cpal 0.18.2 (optional `audio`); rfd 0.18.0 (xdg-portal); serde 1.0.229 and serde_json 1.0.154; thiserror 2.0.21; directories 6.0.0; tempfile 3.27.0; sha2 0.11.1; sysinfo 0.40.4 (system only); fontdb 0.24.0; skrifa 0.44.0; harfrust 0.12.0; image 0.25.10 (gif only); whisper-rs 0.16.0 and ureq 3.5.2 (optional `stt`); macOS only: objc2 0.6.5, objc2-foundation 0.3.2, objc2-app-kit 0.3.2, winit 0.30.13 (default features off). Dev: syn 2 (module-cycle test), ttf-parser (icon check). Rejected with reasons in ai-docs/architecture-adr.md (muda, egui-phosphor, font-kit, fontique, srtparse, subparse, egui_dock, egui_tiles, smithay-client-toolkit, cargo-nextest, reqwest, native-tls).

## Open items (see ai-docs/architecture-risks.md)
3.1.0 [TECH, PE to check]: blur cause in the clip path (ADR-250), baseline offset in the Enlarge event (A-3.1-4), proxy size and T2 time (measured after build).
REQ-177 (needs a second clip, Andrea); REQ-315 (decide the target per hardware class); REQ-222 blur divisor wording (PM); picture item limits (ADR-192, Andrea or PE); speed-first preset name (PE record); muda spike result (232); AccessKit size on Linux (197); boxed shadow design A or B and the Enlarge spike (2.1.0); module-cycle result of 2.2.0 and 2.3.0 at VERIFY.

## Human docs index
The paragraph index of docs/architecture*.md is in ai-docs/architecture-doc-index.md (sub-document; read with grep -n, then sed -n 'A,Bp' on the docs file). Regenerate its lines after every change to a human architecture doc (`scripts/doc-index.sh docs/architecture*.md`).
