# ai-notes: documenter notes

## Index of managed docs (line ranges, refreshed 1.2.6)
- README.md § title+badges — L1-20 (20 lines)
- README.md § What it is — L22-23 (2 lines)
- README.md § System requirements — L25-29 (5 lines)
- README.md § Installation — L31-61 (31 lines)
- README.md § Usage (steps 1-11, shortcuts) — L63-99 (37 lines)
- README.md § Known limitations — L101-131 (31 lines)
- README.md § Docs links — L133 (1 line)
- README.md § Latest changes — L135-140 (6 lines)
- README.md § License — L142-144 (3 lines)
- README.md § Poem — L146-159 (14 lines, 10 verses)
- docs/changelog.md § 1.2.6 — L5-27 (23 lines)
- docs/changelog.md § 1.2.5 … 1.2.1 (workflow-only except 1.2.2) — L28-69
- docs/changelog.md § 1.2.0 — L70-102; 1.1.0 — L103-144; 1.0.0 — L145-147; 0.4.3 and older — L148-165
- docs/roadmap.md § Export — L5-18 (14 items)
- docs/roadmap.md § Effects — L19-26 (6 items)
- docs/roadmap.md § 360 video — L27-38 (12 items)
- docs/roadmap.md § Subtitles — L39-60 (22 items)
- docs/roadmap.md § Editing — L61-69 (5 items)
- docs/roadmap.md § Interface — L70-88 (~17 items)
- docs/roadmap.md § Performance — L89-101 (11 items)
- docs/roadmap.md § Quality — L102-114 (~12 items)
- docs/roadmap.md § Packaging — L115-127 (~11 items)
- docs/roadmap.md § Workflow — L128-130 (1 item)
- docs/roadmap.md § Not planned — L131-135 (4 items)
- docs/workflow.md § Agents — L5-19; Commands — L20-28; /add-feature pipeline — L29-47; Rules — L48-61; Token economy — L62-72; Skills — L73-75; CI — L76-78; Scripts — L79-92; Hooks and settings — L93-95; Repository — L96-98; Where things live — L99-113; Workflow manifest — L114-130; Workflow history — L131-143

## Consistency checks (1.2.6 close)
- version 1.2.6 set, `scripts/version.sh check` consistent; badge in README 1.2.6
- coverage badge 96.0% matches the 1.2.6 changelog (line coverage 96.0%, features 105/105)
- scripts/docs-check.sh: ok
- README Latest changes: 1.2.6, 1.2.2, 1.2.0 (1.2.1, 1.2.3, 1.2.4, 1.2.5 are workflow-only)
- README and roadmap free of AI/agent/command/workflow words (the only "workflow" hit was a roadmap item, reworded)
- roadmap: diff against HEAD has only additions (no lost items), no duplicated titles; 18 items from 1.2.6 plus one "Not planned" item present once
- roadmap: "Workflow" item rewritten without workflow/notes references and with current sizes (architecture 148.5 KB, code 62.2 KB, business 52.6 KB)
- docs/code-hardware.md is 237 lines (roadmap item matches)
- .claude/ and AGENTS.md unchanged in 1.2.6: docs/workflow.md needs no update
- AGENTS.md states MSRV 1.100 (matches Cargo.toml rust-version); the earlier "MSRV 1.88" open item is closed

## Open items
- README poem: the text mentions "the version rose to 1.2.4" and the project page, so it was edited in a non-major development; it must stay unchanged until the next major release. Not changed here; for the caller to decide.
- README Known limitations and roadmap item (source 0.4.3) both say macOS and Windows installers for 0.4.2 come "with the next release"; not verified against the 0.4.3 release. Left as is.
- business.md "Known limitations (0.2.0)" may not list the three editing limitations kept in business-editing.md (not rechecked in 1.2.6).
- Subtitle lines overlap: README/changelog 1.2.6 limitation and roadmap item (1.2.0) describe the same issue; kept once in the roadmap.

## Development log
- 0.2.0: changelog entry, coverage badge 95.4, features 44/44; docs fixes (editing shortcut, limitations)
- 1.2.6 close: roadmap checked for lost/duplicate items (none), Workflow item reworded, README Latest changes checked, docs-check ok
