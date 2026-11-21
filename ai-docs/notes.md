# ai-notes: documenter notes

Owner: documenter. Index refreshed in the 2.0.4 business part (changelog lines)  and before that in the 2.0.3 code part (README, roadmap and changelog) and in the 1.6.3 RELEASE-CLEAN (line ranges from `grep -n '^#'`; refresh after every change of a managed doc).

## Index of managed docs
README.md, docs/roadmap.md, docs/changelog.md and docs/workflow.md (no sub-documents). Other docs/*.md belong to their owners.

- README.md § videolore — L1-23 (23 lines)
- README.md § What it is — L24-26 (3 lines)
- README.md § System requirements — L27-32 (6 lines)
- README.md § Installation — L33-39 (7 lines)
- README.md § Windows (x86_64) — L40-45 (6 lines)
- README.md § macOS (Apple Silicon) — L46-51 (6 lines)
- README.md § Linux (x86_64, Ubuntu LTS and Fedora) — L52-58 (7 lines)
- README.md § From source — L59-67 (9 lines)
- README.md § Usage — L68-109 (42 lines)
- README.md § Known limitations — L110-165 (56 lines)
- README.md § Latest changes — L166-171 (6 lines)
- README.md § License — L172-177 (6 lines)
- README.md § Ode to the creator — L178-189 (12 lines)
- docs/roadmap.md § Roadmap — L1-4 (4 lines)
- docs/roadmap.md § Priority 1 — Release checks — L5-15 (11 lines)
- docs/roadmap.md § Priority 2 — Text and subtitles on the picture — L16-26 (11 lines)
- docs/roadmap.md § Priority 3 — Subtitle accuracy and subtitle features — L27-40 (14 lines)
- docs/roadmap.md § Priority 4 — Editing speed and safety — L41-45 (5 lines)
- docs/roadmap.md § Priority 5 — Export speed and hardware — L46-60 (15 lines)
- docs/roadmap.md § Priority 6 — Installation and platforms — L61-70 (10 lines)
- docs/roadmap.md § Priority 7 — Interface — L71-75 (5 lines)
- docs/roadmap.md § Priority 8 — Video player — L76-80 (5 lines)
- docs/roadmap.md § Priority 9 — 360 video — L81-95 (15 lines)
- docs/roadmap.md § Priority 10 — Effects and frame — L96-108 (13 lines)
- docs/roadmap.md § Priority 11 — Quality, tests and measurements — L109-140 (32 lines)
- docs/roadmap.md § Priority 12 — Splash, project page and release process — L141-155 (15 lines)
- docs/roadmap.md § Not planned — L156-169 (14 lines)
- docs/changelog.md § Changelog — L1-4 (4 lines)
- docs/changelog.md § 2.0.4 — 2026-11-21 00:15 +0000 — L5-19 (15 lines)
- docs/changelog.md § 2.0.3 — 2026-11-20 22:05 +0000 — L20-31 (12 lines)
- docs/changelog.md § 2.0.2 — 2026-11-20 20:34 +0000 — L32-36 (5 lines)
- docs/changelog.md § 2.0.1 — 2026-11-20 20:06 +0000 — L37-41 (5 lines)
- docs/changelog.md § 2.0.0 — 2026-11-20 19:50 +0000 — L42-44 (3 lines)
- docs/workflow.md § Agentic workflow — L1-3 (3 lines)
- docs/workflow.md § Agents — L5-20 (16 lines)
- docs/workflow.md § Commands — L22-30 (9 lines)
- docs/workflow.md § `/add-feature` pipeline — L32-49 (18 lines)
- docs/workflow.md § `/release` pipeline — L51-68 (18 lines)
- docs/workflow.md § Rules (full text in `AGENTS.md`) — L70-84 (15 lines)
- docs/workflow.md § Token economy — L86-95 (10 lines)
- docs/workflow.md § Skills — L97-98 (2 lines)
- docs/workflow.md § CI — L100-101 (2 lines)
- docs/workflow.md § Scripts — L103-115 (13 lines)
- docs/workflow.md § Hooks and settings — L117-118 (2 lines)
- docs/workflow.md § Repository — L120-121 (2 lines)
- docs/workflow.md § Where things live — L123-137 (15 lines)
- docs/workflow.md § Workflow manifest — L139-155 (17 lines)
- docs/workflow.md § Workflow history — L157-172 (16 lines)

## Release hand-off: known limitations
For the Product Manager (RELEASE step 2, prioritising the roadmap). Cleaned from the README "Known limitations" in the 1.6.3 RELEASE-CLEAN. Removed as fixed: the 0.4.2 installer note (fixed in 0.4.3, tag v0.4.3); no other item removed, all README limits kept. Each item keeps its source (the version where it was recorded, from the roadmap or the changelog).

Installation and platforms
- Installers are not code-signed: each system asks for a one-time confirmation (source: 0.2.0, roadmap code signing).
- Generating subtitles on Windows and Linux needs AVX2, FMA and F16C (Intel from 2013, AMD from 2015, all Ryzen); on older processors a message says so and subtitle lines can be added by hand; the check was tested with simulated processors, not on old hardware (source: 0.3.0).
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings (source: 1.1.0, decision of 2026-11-15).
- The window size and position are not remembered, and popups keep their place only until videolore is closed (source: 1.1.0).
- On Windows and Linux, double-clicking a project while videolore is open opens a second window (source: 0.2.0); on macOS the menu bar is inside the window and Hide and Minimize are not available (source: 1.1.0).
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open… (source: 1.1.0).
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text; Paste in the Edit menu and the toolbar always works (source: 1.1.0).
- Projects saved by version 1.4.0 or later cannot be opened by 1.3.0 or earlier; they are refused with a message and left untouched; projects from 1.3.0 and earlier open with the same result, except as listed below (source: 1.4.0).

Export and hardware
- Faster export needs NVIDIA graphics driver 610 or newer; with an older driver the normal method is used and Settings > Advanced says why (source: 0.4.0).
- Faster export on Linux (Intel and AMD) needs libva; with the bundled ffmpeg it needs libva 2.21 or newer, so Ubuntu 24.04 (libva 2.20) uses the normal method; a system ffmpeg chosen in Advanced can still use the hardware (source: 0.4.0).
- Faster export on Fedora needs the RPM Fusion freeworld video drivers for AMD and Intel (source: 0.4.0).
- Faster is not always quicker: on integrated Intel graphics it was measured from 14% quicker to 41% slower than Best quality, and files may be a little larger (source: 0.4.0, QG-05 measurement).
- On the Intel driver tested on Ubuntu 24.04, hardware HEVC export is not usable; the normal method makes those files (source: 0.4.0).
- Faster export has been verified on real hardware only on Linux with Intel integrated graphics (source: 0.4.0, QG-05 checklist).
- On a very busy computer the video hardware can still be missed; Faster then uses the normal method, and an inconclusive check is repeated once in the background (source: 0.4.2).
- On computers with more than one graphics card NVIDIA is always preferred; the card cannot be chosen (source: 0.4.0).
- A 360 clip with a moving camera exports at about one third of real time (source: 1.2.6).
- 360 video must be a single full equirectangular picture (no stereo 3D, cube maps or partial 180° video); the camera cannot roll or be steered from the keyboard, and spatial sound does not follow the camera (source: 1.2.6).
- A 360 video export is marked as 360 video only in MP4 and MOV files (source: 1.3.0).
- A 360 video ignores the video format presets and keeps the sphere's own size (source: 1.4.0).
- A 360 video export ignores Change aspect, Scale and Move; the flat camera view applies them (source: 1.5.0).
- Change aspect, Scale and Move make the export a re-encode, even at their default values (source: 1.5.0).
- The longer side of a frame is capped at 4096 px: a Change aspect on a large source can give a smaller frame, and a large frame exports more slowly (export time not measured) (source: 1.5.0).
- The Export dialog remembers its choices for the session only, not in the project or in Settings; the video and audio formats are a fixed list of presets (source: 1.4.0).
- Apply LUT is applied before the frame effect of Change aspect, so the blurred or coloured fill is not graded (source: 1.6.0, accepted deviation).

Subtitles and speech
- Subtitles in scripts other than Latin, Greek and Cyrillic may look different on each computer or show empty boxes, and cannot be read in the subtitle editor (source: 0.2.0).
- Speech is recognised in pieces of about two minutes; a word cut at a boundary can be lost (source: 0.3.0).
- When a video's own subtitles are used, the word-by-word highlight timing is approximate: the accuracy target is not met with the Fast model (82% of the words matched, average start error 0.48 s on the reference clip) (source: 1.2.0, REQ-095).
- Matching a video's subtitles to the speech works only as well as their text follows what is said; translated or paraphrased subtitles often get evenly spread words (source: 1.2.0).
- Moved subtitle lines shown at the same time can overlap, and editing a line's text after moving it can push it past the edge of the picture in the export (source: 1.2.0).
- The .srt file has no subtitle positions (source: 1.2.0).
- Align horizontally does nothing for a subtitle line that is still at its style's place; an enlarged line's text may wrap and shift its centre slightly (source: 1.2.6).
- A subtitle line placed on a 360 clip is plain white text, with no preset box or word highlight (source: 1.3.0).
- Clean audio has one fixed strength: it reduces background noise but does not remove it completely; quiet speech (about -32 dB) can lose up to about 2.5 dB, and its cost on weak computers is not measured (source: 1.4.0).

Editing, preview and projects
- Removing a file from the media list also drops the undo history when an undo step refers to it (source: 0.2.0).
- Cleared preview copies are made again only after reopening the project or changing Preview quality; under Automatic, a 1280x720 source never gets a preview copy (source: 0.3.0; 0.2.0 for the 720p case).
- The preview copy cache is never cleaned automatically (source: 0.2.0).
- A project file with a text whose time to live is above 24 hours, or below 0.1 s, is refused as damaged on opening; while editing, a text's time to live is limited to the video's length (source: 1.3.0).
- Texts of projects from 1.3.0 take the best-fit size of their box when opened, so large multi-line texts may shrink; a 360 switch set in a 1.3.0 project is dropped when it opens (source: 1.4.0).
- A video without spherical information is always flat: the "Treat as 360° video" switch is removed (source: 1.4.0).
- A text or a subtitle line dragged past an edge of the picture is cut in the preview and in the export (source: 1.4.0).
- The text size of a text item is an estimate of the best fit for its box, so a text can fill about 85% to 115% of it; short texts become large and long ones small (source: 1.4.0).
- Text items have no outline or background box, use the fonts installed on the computer (they can look different on another computer) and are not written to the .srt file (source: 1.3.0).
- Text items keep their place when the picture is scaled or moved and do not follow Scale or Move; subtitle lines do not follow them either (source: 1.5.0).
- Clips added after a whole-video effect do not get it, and nothing says so (source: 1.6.1).
- After a clip's keyframes are changed alone, the whole-video Scale or Move popup shows and changes only the first clip's keyframes; the Scale corners and the Move target show one effect per kind (source: 1.6.1 and 1.5.0).
- A 1.6.0 clip with its own and a whole-video Scale or Move becomes one combined path: exact at the keyframe times, a straight line between them (source: 1.6.1).
- A text item's width changes only from its right edge (source: 1.6.1).
- Keyframes are straight lines with no easing, cannot be copied from one clip to another, and show in the popup and on the clip's marker, not as separate timeline markers (source: 1.5.0).
- The blur of Fit empty with blurred video has one fixed strength (source: 1.5.0).
- Apply LUT reads `.cube` files only, at full strength; the project keeps the LUT's path, not a copy, so a moved LUT file stops the export until it is found again (source: 1.6.0).
- The Change aspect row in the Effects list has no tooltip and cannot be dragged (source: 1.6.0, accepted deviation).

Player
- The player has no settings, no playlist and no file association; it lists text subtitle tracks only (no bitmap subtitles such as Blu-ray or DVB); it plays very large videos from a silent preview copy; its About screen shows the editor's list of third-party components (source: 1.6.0).

## Latest consistency checks (1.6.3 RELEASE-CLEAN)
- Read in full: README, docs/roadmap.md, docs/workflow.md. Other docs/*.md (business*, architecture*, code*) were not read in full in this pass: grep for stale process notes only (see open items). Changelog not touched.
- README: Known limitations: the 0.4.2 installer note removed (fixed in 0.4.3); the compatibility item "Projects saved by this version" reworded to 1.4.0, the version that wrote that format. Badges checked against Cargo.toml (versions, rust-version 1.100). Latest changes 1.6.2, 1.6.1, 1.6.0 kept (1.6.3 has no user-facing change); poem kept; What it is 199 words.
- Roadmap: removed the 0.4.2 macOS/Windows installers item (delivered in 0.4.3), removed the internal document-size item and the planning-merge item (both are about AI notes, moved below); the ffmpeg pin item keeps source 1.6.1 with no retarget note.
- Workflow: "LICENSE written by the Documenter" corrected (LICENSE is never edited). The workflow history is unchanged.
- architecture.md: "8 planned for 1.6.0" label removed from the project-file row.
- docs-check ok (run after the edits).

## Open items (documenter)
- Other docs/*.md were not compacted in full in 1.6.3 RELEASE-CLEAN: a later release should read business*, architecture*, code* in full (owners clean their own docs in their RELEASE step).
- Stale process notes seen by grep, owners to fix: docs/architecture-frame.md (line 91 "checked on the bundled build before the code is written"), docs/architecture.md (line 142 "Still open" sentence about 0.3.0 decisions). The splash origin item is resolved in 1.6.3 (docs updated).
- Not mine, to fix by their owners: ai-docs/code.md (Cycles and dispatch_inner notes, pointing at the new roadmap items), docs/roadmap_tasks.md was fixed by the documenter on the coordinator's request (splash origin removed, dispatch_inner merged into the application module item); the Product Manager should keep it so at its next rewrite.
- Shorter internal documents (AI notes only, not for the roadmap): ai-docs/architecture.md about 1089 lines, ai-docs/code.md 376, ai-docs/business.md 568; each owner should split them into an index and sub-documents at its next compaction.
- Decision kept out of the roadmap (not planned, 2026-11-15): merging the Product Manager and Architect notes into one step, rejected because one owner would hold two private notes files against the ownership rule.
- Roadmap items still to check before the next release (kept in docs/roadmap.md): ffmpeg 9.x checks, the workflow release check.

## Development log
- 1.0.0 to 1.6.2: one line per development in the changelog and in the earlier notes (compacted in 1.6.3; the detailed per-part checks of those versions are not kept here).
- 1.6.3: version 1.6.3 (no change needed in RELEASE-CLEAN), changelog entry Changed (splash file names) and Workflow (release flow, project page rules, /analysis), no user-facing change, README latest changes unchanged.
- 1.6.3 RELEASE 1.6.3 "core effects and player": changelog release summary and a Changed bullet (splash origin and licence: own work, MIT); roadmap: splash origin item and hardware-document split removed, dispatch_inner, module cycles and 360 test pictures added to priority 11; docs updated on the splash origin (business-splash, architecture, architecture-packaging, architecture-security); coverage badge 95.6; README unchanged (1.6.3 is workflow-only, latest changes are 1.6.2, 1.6.1, 1.6.0); docs-check ok, version check ok.
- 1.6.3 RELEASE-CLEAN: known limitations hand-off above, github_page/after release.md with the Documenter section, roadmap cleanup, README known limitations cleanup, architecture.md label, workflow LICENSE wording, notes compacted, docs-check ok, version 1.6.3 unchanged, poem unchanged, releases/ not touched.
- 2.0.1 architecture (2026-11-20 20:13 +0000): version 2.0.1 unchanged, changelog bullet (base module, cycles broken, cycle check, syn dev-dependency), roadmap: two items added to priority 11 (cycle check CI cost QG-06, cycle check limits), "Break the module cycles" kept until the code part; app.rs and render.rs size item already in roadmap, not duplicated; docs-check ok. Architect's future work on the Components table move (next release) not added to the roadmap: it is a doc split for RELEASE-CLEAN, noted here.
- 2.0.0 POEM: new README poem written after 1.6.3 release (opening of 2.0.0), docs-check ok, version 2.0.0 consistent.
- 2.0.1 code (2026-11-20): version 2.0.1 unchanged; coverage badge 95.6 (line coverage; feature coverage 150/150); changelog code bullet; roadmap: 'Break the module cycles' removed (also from roadmap_tasks), two items added to priority 12 (leaves wording, player notice version line); architecture planned statuses set to delivered (architecture, architecture-decisions, architecture-dependencies, architecture-structure); docs-check ok.

- 2.0.1: business plan for the code structure (REQ-151..153 planned, no behaviour change); changelog entry Changed; roadmap item 'Break the module cycles' kept until the code is delivered; roadmap batch 1 'Verify the release' not part of this version (Future work); docs-check ok, version 2.0.1 consistent, README unchanged (not user-facing).
- 2.0.1 close (2026-11-20 20:30 +0000): version 2.0.1 consistent everywhere (version.sh check ok), docs-check ok, README badge 2.0.1, Latest changes unchanged (2.0.1 is not user-facing), poem unchanged, coverage badge unchanged at 95.6.
- 2.0.2 architecture (2026-11-20 21:32 +0000): version 2.0.2 unchanged; changelog bullet (architecture for 2.0.2); QG-10 wording fixed in business-performance (no .deb, Windows and macOS config-only, Linux AppImage from stubs) and roadmap priority 1 installer item; stale "Product Manager corrects QG-10" sentence removed from architecture-verification; roadmap: three future items added (Windows and macOS runs with ffmpeg 9.0.2 in P1, camera targets in P9, CI cost of pinned ffmpeg and package-config in P11); roadmap and changelog index lines refreshed; docs-check ok, version check ok. Open: docs/code-packaging.md line 36 still says `-f deb` (code owner's doc, not changed here).
- 2.0.2 business (2026-11-20 20:34 +0000): version set to 2.0.2 (consistent); changelog entry Changed (business plan: REQ-154, QG-07..10, verification checklist, Andrea's decisions, batch renumbering); roadmap: code signing and custom resolutions moved to Not planned, hardware-for-more-formats, whole-video copies and 360 speed items reworded as decided, manual checks kept with a pointer to business-verification.md; docs/roadmap_tasks.md renumbered (batch N = 2.0.(N+1), 2.0.1 delivered), decisions moved to a decided list, licence sentence of batch 1 removed, cycle items added to continuous; README unchanged (no user-facing change); docs-check ok.
- 2.0.2 code (part code): coverage badge 95.6%; changelog 2.0.2 code bullet and REQ-154 wording corrected to implemented; README latest changes now 2.0.2, 1.6.2, 1.6.1 (1.6.0 dropped); roadmap batch 1 delivered items removed (manual checks, Windows and macOS runs, moving-camera measurement on 9.0.2 kept or added), paused-picture defect added as first 2.0.3 item, UI frame gates, hardware probe fakes and notices-after-bump added; roadmap_tasks 2.0.2 marked delivered; notes index refreshed; docs-check ok.
- 2.0.2 close (2026-11-20 22:00 +0000): version 2.0.2 consistent (version.sh check ok), docs-check ok; README latest changes checked (2.0.2, 1.6.2, 1.6.1; 2.0.0 and 2.0.1 are not user-facing), poem unchanged; roadmap priority 1 item 'Check the reworked release flow' restored (removed by mistake in batch 1); docs/roadmap_tasks.md does not list it yet (Product Manager's file, reported).
- 2.0.3 business (2026-11-20 22:05 +0000): version set to 2.0.3 (consistent), notices version line regenerated; changelog 2.0.3 business entry (batches 2 to 4 as one development, REQ-155..179 planned); roadmap: maintainer measurements and speech-model decision added to Priority 1 and 3 (items for the font, levels and 20 MB measurements already existed in Priority 2 and 3, not duplicated); docs/roadmap_tasks.md notes that 2.0.3 carries batches 2 to 4; index refreshed; README unchanged (no user-facing change delivered); docs-check ok. Inconsistency left for the Product Manager: docs/business-text.md, business-subtitles.md and business.md still label the placement and accuracy batches as planned 2.0.4 and 2.0.5.
- 2.0.3 architecture (2026-11-20 22:25 +0000): version 2.0.3 unchanged; changelog 2.0.3 architecture bullet, format 10 and refit notes, Removed outline, CI fix of e32376d (Fixed); roadmap_tasks headings and table relabelled (batches 2 to 4 built in 2.0.3, batches 5 to 15 renumbered at next release); roadmap: speech model item turned into the REQ-177 decision point, libass and box padding check, text module split, test font licence and cycle-note re-check added; roadmap and changelog index refreshed; docs-check ok; README unchanged (no user-facing change built yet). Open: docs/business-text.md, business-subtitles.md and business.md still say the placement and accuracy batches are 2.0.4 and 2.0.5 (Product Manager).
- 2.0.3 code (2026-11-20 23:55 +0000): version 2.0.3 unchanged; coverage badge 95.8 (line coverage; feature coverage 154/154); changelog 2.0.3 bullets (added readable text and placement warnings, code, REQ-177 status, fixed paused picture); README: badge, Latest changes 2.0.3, 2.0.2, 1.6.2, usage items 6, 11, 13 and known limitations updated (overlap warnings, text size and width, format 10); roadmap: delivered batch items removed (readable text, placement, accuracy), REQ-177 kept as the decision point, future work added (other UI frame gates, five-minute speech test, font scan repaint, text module split, line box rule); roadmap tasks: batches 2 to 4 marked delivered; docs-check ok.

- 2.0.3 close (2026-11-20 23:58 +0000): version.sh check consistent (2.0.3); docs-check ok; README latest changes checked (2.0.3, 2.0.2, 1.6.2; 1.6.3 is workflow-only per its release notes), poem unchanged, no AI/agent terms (the Actions workflow URL in the build badge is kept); rust-version 1.100 matches badge; no file changed in this call.

- 2.0.4 business (2026-11-21 00:15 +0000): version set to 2.0.4 (consistent, notices version line regenerated); changelog 2.0.4 entry (one development for all remaining batches, REQ-180..299 planned, REQ-177 untouched, Andrea's decisions, format 11 bump, manual checks 9-28); docs/roadmap_tasks.md notes that batches 5 to 15 and the continuous items are built together as 2.0.4; roadmap.md unchanged (items stay open until STATUS); README unchanged (no user-facing change delivered); index refreshed; docs-check ok.
- 2.0.4 architecture (2026-11-21 00:40 +0000): version 2.0.4 unchanged; the architecture bullets, four roadmap items and the quality network-line fix were reverted at the coordinator's checkpoint (unbuilt work lives in ai-docs/roadmap-2_0-checkpoint.md, section Documenter); index refreshed.
- 2.0.4 checkpoint (2026-11-21): ai-docs/roadmap-2_0-checkpoint.md created with the Documenter section; docs describe only what is built or planned as the product manager and architect wrote it.
- 2.0.4 close (2026-11-21): version 2.0.4 consistent; changelog 2.0.4 entry rewritten to the built items per area (planned items removed); roadmap: delivered items removed, unbuilt items kept, 2.0.5 note in the intro, future items added (idle heartbeat, Quit in the macOS menu, cache limit on a worker, CSP hashes, playback of a blurred fill); README latest changes 2.0.4, 2.0.3, 2.0.2; badges coverage 95.0 (coverage.sh --badge), harfrust and skrifa badges added; splash summaries rewritten for 2.0.4 (editor and player); checkpoint file ai-docs/roadmap-2_0-checkpoint.md kept; docs-check ok, version check ok; poem unchanged.

## Open items (documenter, 2.0.3)
- The known-limitations hand-off above is from 1.6.3; the next RELEASE-CLEAN rewrites it from the README.
- Roadmap item "Text items and picture moves" (Priority 2) is kept: it is not in the REQ-155..179 list. Andrea or the Product Manager to confirm whether it is delivered.
- Roadmap item "Migrated text sizes" is kept for the option to offer the old size again; the status message (REQ-167) is delivered.
- Roadmap item "Gated alignment test on CI's versions" is kept: not checked whether the real-speech test runs on CI with the pinned espeak.
