# ai-notes: documenter notes

## Index of managed docs (line ranges, refreshed 1.4.0 code part)
- README.md § title+badges — L1-22 (22 lines)
- README.md § What it is — L23-25 (3 lines)
- README.md § System requirements — L26-31 (6 lines)
- README.md § Installation — L32-36 (5 lines)
- README.md § Windows (x86_64) — L37-42 (6 lines)
- README.md § macOS (Apple Silicon) — L43-48 (6 lines)
- README.md § Linux (x86_64, Ubuntu LTS and Fedora) — L49-55 (7 lines)
- README.md § From source — L56-63 (8 lines)
- README.md § Usage — L64-102 (39 lines)
- README.md § Known limitations — L103-147 (45 lines)
- README.md § Latest changes — L148-154 (7 lines)
- README.md § License — L155-160 (6 lines)
- README.md § Poem (Ode to the creator) — L161-172 (12 lines)
- docs/roadmap.md § Export — L5-22 (18 lines)
- docs/roadmap.md § Effects — L23-33 (11 lines)
- docs/roadmap.md § 360 video — L34-53 (20 lines)
- docs/roadmap.md § Text — L54-69 (16 lines)
- docs/roadmap.md § Subtitles — L70-91 (22 lines)
- docs/roadmap.md § Editing — L92-101 (10 lines)
- docs/roadmap.md § Interface — L102-123 (22 lines)
- docs/roadmap.md § Performance — L124-141 (18 lines)
- docs/roadmap.md § Quality — L142-156 (15 lines)
- docs/roadmap.md § Packaging — L157-170 (14 lines)
- docs/roadmap.md § Workflow — L171-173 (3 lines)
- docs/roadmap.md § Not planned — L174-179 (6 lines)
- docs/changelog.md § 1.4.0 — 2026-11-17 21:16 +0000 — L5-33 (29 lines)
- docs/changelog.md § 1.3.0 — 2026-11-17 19:08 +0000 — L34-60 (27 lines)
- docs/changelog.md § 1.2.8 — 2026-11-17 18:59 +0000 — L61-65 (5 lines)
- docs/changelog.md § 1.2.7 — 2026-11-17 18:55 +0000 — L66-70 (5 lines)
- docs/changelog.md § 1.2.6 — 2026-11-16 23:30 +0000 — L71-93 (23 lines)
- docs/changelog.md § 1.2.5 — 2026-11-16 22:40 +0000 — L94-104 (11 lines)
- docs/changelog.md § 1.2.4 — 2026-11-15 22:02 +0000 — L105-111 (7 lines)
- docs/changelog.md § 1.2.3 — 2026-11-15 22:01 +0000 — L112-118 (7 lines)
- docs/changelog.md § 1.2.2 — 2026-11-15 21:59 +0000 — L119-125 (7 lines)
- docs/changelog.md § 1.2.1 — 2026-11-15 21:52 +0000 — L126-135 (10 lines)
- docs/changelog.md § 1.2.0 — 2026-11-15 21:42 +0000 — L136-168 (33 lines)
- docs/changelog.md § 1.1.0 — 2026-11-15 17:52 +0000 — L169-210 (42 lines)
- docs/changelog.md § 1.0.0 — 2026-11-14 22:36 +0000 — L211-213 (3 lines)
- docs/changelog.md § 0.4.3 — 2026-11-14 22:35 +0000 — L214-231 (18 lines)
- docs/workflow.md § Agents — L5-19; Commands — L20-28; /add-feature pipeline — L29-47; Rules — L48-61; Token economy — L62-72; Skills — L73-75; CI — L76-78; Scripts — L79-92; Hooks and settings — L93-95; Repository — L96-98; Where things live — L99-113; Workflow manifest — L114-130; Workflow history — L131-143 (not changed in 1.4.0)

## Latest consistency checks (1.4.0 code part)
- coverage badge 95.8% (`scripts/coverage.sh --badge 95.8`); version 1.4.0 unchanged
- changelog 1.4.0 rewritten: Added, Changed, Quality, Known limitations; the 1.3 project 360 switch drop and the text migration are in Changed and Known limitations
- roadmap: removed "Align and Reset for text items" and "Check the export presets and audio formats"; "Remember export choices" moved to Not planned; added 8 items (Clean audio cost and quiet speech, width drag ghost outline, 360 video presets, Edit menu grouping, golden argument tests, dropdown clicks in UI tests, bundled ffmpeg 9.x Clean audio and MP3 output)
- README: usage steps 4, 8, 10, 11, 12 updated; Known limitations (format 6, 360 switch, Clean audio, best fit, drag past edges, export choices, 360 presets); Latest changes 1.4.0, 1.3.0, 1.2.6; What it is 199 words; poem unchanged; no AI or workflow terms
- architecture docs: "planned" labels for 1.4.0 removed (anchors `-140-planned` renamed to `-140`), architecture.md now says "built up to 1.4.0" and REQ-001..123; docs-check ok
- open: docs/architecture.md (about 150 lines) split still pending, owner's task (Workflow roadmap item covers the size)
- open: the 0.4.2 macOS and Windows installer sentence (README Known limitations and roadmap Packaging) is not verified against a later release
- open: the README poem names version 1.2.4 and the project page; it is kept as is (no major release). Last poem rewrite date not recorded in the changelog

## Development log
- 1.3.0 business, architecture, code, close: version, changelog, roadmap, coverage badge 96.0, README, docs-check ok
- 1.4.0 business: version, changelog (planned), roadmap items, three stale doc fixes, docs-check ok
- 1.4.0 architecture: changelog planned bullet, roadmap +5, media-pipeline audio chain, export-dialog folder fallback, docs-check ok
- 1.4.0 code: changelog 1.4.0 finalised, roadmap delivered items removed and future work added, coverage badge 95.8, README (usage, shortcuts unchanged, known limitations, latest changes), 1.4.0 "planned" labels removed from docs/architecture*.md, stale switch sentence in docs/code-360.md reworded, docs-check ok
- 1.4.0 close: version check consistent, docs-check ok, README 1.4.0 entry "commands" reworded to "actions" (no command/workflow terms left), changelog 1.4.0 checked, no release (poem kept)
