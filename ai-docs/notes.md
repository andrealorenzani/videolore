# ai-notes: documenter notes

## Index of managed docs (line ranges, refreshed 1.3.0 architecture part)
- README.md § title+badges — L1-21 (21 lines)
- README.md § What it is — L23-24 (2 lines)
- README.md § System requirements — L26-30 (5 lines)
- README.md § Installation — L32-62 (31 lines)
- README.md § Usage (steps 1-12, shortcuts) — L64-102 (39 lines)
- README.md § Known limitations — L103-137 (35 lines)
- README.md § Docs links — L139 (1 line)
- README.md § Latest changes — L141-146 (6 lines)
- README.md § License — L148-150 (3 lines)
- README.md § Poem — L152-165 (14 lines, 10 verses)
- docs/changelog.md § 1.3.0 — L5-31 (27 lines: Added, Changed, Quality, Known limitations)
- docs/changelog.md § 1.2.8 — L32-36; 1.2.7 — L37-41; 1.2.6 — L42-64 (approx. per entry)
- docs/changelog.md § 1.2.5 and older — from L65 (heading list: `grep -n '^## ' docs/changelog.md`)
- docs/roadmap.md § Export — L5-19 (13 items)
- docs/roadmap.md § Effects — L20-27 (6 items)
- docs/roadmap.md § 360 video — L28-45 (16 items)
- docs/roadmap.md § Text — L46-58 (11 items)
- docs/roadmap.md § Subtitles — L59-80 (20 items)
- docs/roadmap.md § Editing — L81-90 (8 items)
- docs/roadmap.md § Interface — L91-110 (18 items)
- docs/roadmap.md § Performance — L111-126 (14 items)
- docs/roadmap.md § Quality — L127-139 (11 items)
- docs/roadmap.md § Packaging — L140-152 (11 items)
- docs/roadmap.md § Workflow — L153-155 (1 item)
- docs/roadmap.md § Not planned — L156-160 (4 items)
- docs/workflow.md § Agents — L5-19; Commands — L20-28; /add-feature pipeline — L29-47; Rules — L48-61; Token economy — L62-72; Skills — L73-75; CI — L76-78; Scripts — L79-92; Hooks and settings — L93-95; Repository — L96-98; Where things live — L99-113; Workflow manifest — L114-130; Workflow history — L131-143

## Consistency checks (1.3.0 architecture part)
- version 1.3.0 unchanged (`scripts/version.sh check` consistent); README unchanged (no user-facing change; poem untouched)
- changelog 1.3.0: architecture planned bullet added under Added (with links to architecture docs)
- roadmap: 4 items added from the architecture part (360 subtitles keep preset place on export; spherical information of 360 export; font names across libraries; cost of loading an installed font); "Text in the .srt file" and "Font fallback check" already present, kept once
- all anchors in docs/*.md and README resolve (checked with a script in the scratchpad, not a repo script); docs-check ok
- docs/architecture.md is 153 lines and its own note asks for a split at the next architecture change (owner's task; the Workflow item "Shorter internal documents" already covers the size, not duplicated)
- architecture-text.md, architecture-ui.md and architecture-360.md consistent with each other on the selection, anchors and font rules (checked by reading the 1.3.0 diffs)
- open: docs/architecture-subtitles.md "Position on screen" and docs/architecture-ffmpeg.md say fonts are found by name through the system; the 1.3.0 font-name spike is on the roadmap

## Consistency checks (1.3.0 business part)
- version set to 1.3.0 (scripts/version.sh check consistent); README badge 1.3.0; README unchanged (nothing user-facing delivered; poem untouched)
- changelog 1.3.0 entry added (Added: one business bullet, planned, not built yet)
- roadmap: new "Text" group (7 items from the 1.3.0 future work); 360 items added (off-view indicator, sphere-anchored auto subtitles, 360 export speed in Performance); "Export 360 video" marked planned for 1.3.0; "More effect types" and "Other objects on the picture" no longer list text overlays (now text items); REQ-096 wording item in Interface
- fixed in docs (not mine, product-manager's text): business-subtitles REQ-090 "text marker" is now "text item's bar on the Text lane" (REQ-077 and REQ-110); business-picture Context Menu feature lists "Set POV and Reset view" as REQ-106 does
- docs-check ok; no forbidden terms in README
- open: business-text says text "runs past the box"; REQ-111 says "drawn past the bottom edge": same meaning, not reworded
- open: ai-docs/business.md (product-manager's index) has line ranges that changed with this development; the owner refreshes them

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

## Consistency checks (1.3.0 code part)
- version 1.3.0 consistent; coverage badge set to 96.0% (`scripts/coverage.sh --badge 96.0`); fontdb 0.24 badge added to README
- README: What it is 193 words (limit 200); Latest changes 1.3.0, 1.2.6, 1.2.2; usage steps 10-12, shortcut row, project-format limitation and the new 360/text limitations updated; poem untouched
- changelog 1.3.0: the two "planned" bullets replaced by Added / Changed / Quality / Known limitations; no Fixed bullet (no user-facing fix recorded for this development)
- roadmap: removed "Export 360 video" and "Box selection wording"; reworded "More effect types" and "Other objects on the picture"; "Multi-selection lost" added (Interface); "Font names across libraries" extended (CI); added 8 items (360 export check with texts, 360 subtitle preset box, 360 marker in other containers, text time cap, blinking text box, preview box with installed font, mixed group drags, 4K/8K frame times)
- docs-check ok; version check consistent

## Open items
- README poem: the text mentions "the version rose to 1.2.4" and the project page, so it was edited in a non-major development; it must stay unchanged until the next major release. Not changed here; for the caller to decide.
- README Known limitations and roadmap item (source 0.4.3) both say macOS and Windows installers for 0.4.2 come "with the next release"; not verified against the 0.4.3 release. Left as is.
- business.md "Known limitations (0.2.0)" may not list the three editing limitations kept in business-editing.md (not rechecked in 1.2.6).
- Subtitle lines overlap: README/changelog 1.2.6 limitation and roadmap item (1.2.0) describe the same issue; kept once in the roadmap.
- docs/architecture.md split (architecture owner's note, 153 lines): pending at the next architecture change.

- resolved 1.3.0 close: architecture docs no longer label 1.3.0 as planned (checked by grep).
- resolved 1.3.0 (documenter, close): the time-to-live rule is "refused on opening" above 24 h and below 0.1 s (product-manager text); changelog 1.3.0, README and roadmap "Longer text times" now say so.

## Development log
- 0.2.0: changelog entry, coverage badge 95.4, features 44/44; docs fixes (editing shortcut, limitations)
- 1.2.6 close: roadmap checked for lost/duplicate items (none), Workflow item reworded, README Latest changes checked, docs-check ok
- 1.3.0 business: version set, changelog entry, roadmap Text group and 360/Interface/Performance items, two small doc fixes, docs-check ok
- 1.3.0 architecture: changelog bullet, roadmap items (360 export placement, spherical check, font names, font load cost), anchors checked, docs-check ok
- 1.3.0 code: version kept, changelog 1.3.0 rewritten, roadmap and README updated, coverage badge 96.0, docs-check ok
- 1.3.0 close: changelog and README 24 h time-to-live wording changed to "refused as damaged on opening" (above 24 h or below 0.1 s); roadmap "Longer text times" reworded; roadmap item "Spherical marker on the bundled ffmpeg" added (360 video); roadmap index refreshed; docs-check ok; version check consistent; README Latest changes 1.3.0, 1.2.6, 1.2.2 (poem untouched, no major release)
