# ai-notes: documenter notes

## Index of managed docs (line ranges, refreshed 1.4.1 code part)
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
- docs/roadmap.md § Project page — L171-175 (5 lines)
- docs/roadmap.md § Workflow — L176-178 (3 lines)
- docs/roadmap.md § Not planned — L179-184 (6 lines)
- docs/changelog.md § 1.4.1 — 2026-11-18 00:01 +0000 L5-14 (10 lines)
- docs/changelog.md § 1.4.0 — 2026-11-17 21:16 +0000 L15-43 (29 lines)
- docs/changelog.md § 1.3.0 — 2026-11-17 19:08 +0000 L44-70 (27 lines)
- docs/changelog.md § 1.2.8 — 2026-11-17 18:59 +0000 L71-75 (5 lines)
- docs/changelog.md § 1.2.7 — 2026-11-17 18:55 +0000 L76-80 (5 lines)
- docs/changelog.md § 1.2.6 — 2026-11-16 23:30 +0000 L81-103 (23 lines)
- docs/changelog.md § 1.2.5 — 2026-11-16 22:40 +0000 L104-114 (11 lines)
- docs/changelog.md § 1.2.4 — 2026-11-15 22:02 +0000 L115-121 (7 lines)
- docs/changelog.md § 1.2.3 — 2026-11-15 22:01 +0000 L122-128 (7 lines)
- docs/changelog.md § 1.2.2 — 2026-11-15 21:59 +0000 L129-135 (7 lines)
- docs/changelog.md § 1.2.1 — 2026-11-15 21:52 +0000 L136-145 (10 lines)
- docs/changelog.md § 1.2.0 — 2026-11-15 21:42 +0000 L146-178 (33 lines)
- docs/changelog.md § 1.1.0 — 2026-11-15 17:52 +0000 L179-220 (42 lines)
- docs/changelog.md § 1.0.0 — 2026-11-14 22:36 +0000 L221-223 (3 lines)
- docs/changelog.md § 0.4.3 — 2026-11-14 22:35 +0000 L224-240 (17 lines)
- docs/workflow.md § Agents — L5-19; Commands — L20-28; /add-feature pipeline — L29-47; Rules — L48-61; Token economy — L62-72; Skills — L73-75; CI — L76-78; Scripts — L79-92; Hooks and settings — L93-95; Repository — L96-98; Where things live — L99-113; Workflow manifest — L114-130; Workflow history — L131-143 (not changed in 1.4.1)

## Latest consistency checks (1.4.1 code part)
- 1.4.1 code: version 1.4.1 consistent, docs-check ok; changelog 1.4.1 gained the page's lighter/keyboard/share-tag bullet and a note that the quality suite was not run locally (no Rust code changed); roadmap Project page +2 (og:image, CSP)
- README unchanged: the page is not the software, so Latest changes stays at 1.4.0, 1.3.0, 1.2.6; coverage badge 95.8 unchanged; no AI or workflow terms added
- docs/workflow.md still matches the page description (github_page/, pages.yml); no change needed

## Development log
- 1.3.0 business, architecture, code, close: version, changelog, roadmap, coverage badge 96.0, README, docs-check ok
- 1.4.0 business: version, changelog (planned), roadmap items, three stale doc fixes, docs-check ok
- 1.4.0 architecture: changelog planned bullet, roadmap +5, media-pipeline audio chain, export-dialog folder fallback, docs-check ok
- 1.4.0 code: changelog 1.4.0 finalised, roadmap delivered items removed and future work added, coverage badge 95.8, README (usage, shortcuts unchanged, known limitations, latest changes), 1.4.0 "planned" labels removed from docs/architecture*.md, stale switch sentence in docs/code-360.md reworded, docs-check ok
- 1.4.0 close: version check consistent, docs-check ok, README 1.4.0 entry "commands" reworded to "actions" (no command/workflow terms left), changelog 1.4.0 checked, no release (poem kept)
- 1.4.1 business: version set, changelog 1.4.1 entry, duplicate block in 1.4.0 removed, page today card, docs-check ok
- 1.4.1 architecture: roadmap +1 (Project page: smaller images, animated WebP/AV1), changelog unchanged (no user-facing change), README unchanged, docs-check ok, version check consistent
- 1.4.1 code and close: changelog 1.4.1 bullets (lighter page, keyboard timeline, share tags, quality suite not run locally), roadmap Project page +2 (og:image, CSP), README unchanged, coverage badge 95.8 unchanged, docs-check ok, version check consistent
