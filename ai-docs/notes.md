# ai-notes: documenter notes

Owner: documenter. Index refreshed in the 2.0.4 close (README, roadmap and changelog line ranges from `grep -n '^#'`; refresh after every change of a managed doc).

## Index of managed docs
README.md, docs/roadmap.md, docs/changelog.md and docs/workflow.md (no sub-documents). Other docs/*.md belong to their owners.

- README.md § videolore — L1-25 (25 lines)
- README.md § What it is — L26-27 (2 lines)
- README.md § System requirements — L29-33 (5 lines)
- README.md § Installation — L35-40 (6 lines)
- README.md § Windows (x86_64) — L42-46 (5 lines)
- README.md § macOS (Apple Silicon) — L48-52 (5 lines)
- README.md § Linux (x86_64, Ubuntu LTS and Fedora) — L54-59 (6 lines)
- README.md § From source — L61-68 (8 lines)
- README.md § Usage — L70-110 (41 lines)
- README.md § Known limitations — L112-165 (54 lines)
- README.md § Latest changes — L166-171 (6 lines)
- README.md § License — L172-176 (5 lines)
- README.md § Ode to the creator — L178-189 (12 lines)
- docs/roadmap.md § Roadmap — L1-4 (4 lines)
- docs/roadmap.md § Priority 1 — Release checks — L5-16 (12 lines)
- docs/roadmap.md § Priority 2 — Text and subtitles on the picture — L17-27 (11 lines)
- docs/roadmap.md § Priority 3 — Subtitle accuracy and subtitle features — L28-41 (14 lines)
- docs/roadmap.md § Priority 4 — Editing speed and safety — L42-46 (5 lines)
- docs/roadmap.md § Priority 5 — Export speed and hardware — L47-61 (15 lines)
- docs/roadmap.md § Priority 6 — Installation and platforms — L62-71 (10 lines)
- docs/roadmap.md § Priority 7 — Interface — L72-76 (5 lines)
- docs/roadmap.md § Priority 8 — Video player — L77-82 (6 lines)
- docs/roadmap.md § Priority 9 — 360 video — L83-93 (11 lines)
- docs/roadmap.md § Priority 10 — Effects and frame — L94-104 (11 lines)
- docs/roadmap.md § Priority 11 — Quality, tests and measurements — L105-127 (23 lines)
- docs/roadmap.md § Priority 12 — Splash, project page and release process — L128-142 (15 lines)
- docs/roadmap.md § Not planned — L143-156 (14 lines)
- docs/changelog.md § Changelog — L1-4 (4 lines)
- docs/changelog.md § 2.0.4 — 2026-11-21 00:15 +0000 — L5-23 (19 lines)
- docs/changelog.md § 2.0.3 — 2026-11-20 22:05 +0000 — L24-35 (12 lines)
- docs/changelog.md § 2.0.2 — 2026-11-20 20:34 +0000 — L36-40 (5 lines)
- docs/changelog.md § 2.0.1 — 2026-11-20 20:06 +0000 — L41-45 (5 lines)
- docs/changelog.md § 2.0.0 — 2026-11-20 19:50 +0000 — L46-47 (2 lines)
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

The list is from 1.6.3. Since then the 2.0.4 close rewrote the README limitations to the built state (window memory, Ctrl/Cmd+V, macOS menu, format 11, speed, picture items, 360 text, spatial sound, player features, LUT copies); the next RELEASE-CLEAN rewrites this section from the README.

Installation and platforms
- Installers are not code-signed: each system asks for a one-time confirmation (source: 0.2.0, roadmap code signing).
- Generating subtitles on Windows and Linux needs AVX2, FMA and F16C (Intel from 2013, AMD from 2015, all Ryzen); on older processors a message says so and subtitle lines can be added by hand; the check was tested with simulated processors, not on old hardware (source: 0.3.0).
- Versions before 1.1.0 drop the saved layout and the recent-projects list when they save their settings (source: 1.1.0, decision of 2026-11-15).
- On macOS the menu bar has no Quit item yet; Cmd+Q still asks about unsaved changes (source: 2.0.4 review).
- Drag and drop does not work in Linux Wayland sessions; use File > Import media… and File > Open… (source: 1.1.0).
- Ctrl/Cmd+V pastes clips only when the system clipboard holds text; Paste in the Edit menu and the toolbar always works (source: 1.1.0). Note: since 2.0.4 Ctrl/Cmd+V pastes the most recent cut or copy made in videolore; this item is stale and goes at the next RELEASE-CLEAN.
- Projects saved by version 2.0.4 cannot be opened by 2.0.3 or earlier; projects saved by 2.0.3 cannot be opened by 2.0.2 or earlier; projects saved by 1.4.0 or later cannot be opened by 1.3.0 or earlier (source: 2.0.3, 2.0.4 format 11, 1.4.0).

Export and hardware
- Faster export needs NVIDIA graphics driver 610 or newer; with an older driver the normal method is used and Settings > Advanced says why (source: 0.4.0).
- Faster export on Linux (Intel and AMD) needs libva; with the bundled ffmpeg it needs libva 2.21 or newer, so Ubuntu 24.04 (libva 2.20) uses the normal method; a system ffmpeg chosen in Advanced can still use the hardware (source: 0.4.0).
- Faster export on Fedora needs the RPM Fusion freeworld video drivers for AMD and Intel (source: 0.4.0).
- Faster is not always quicker: on integrated Intel graphics it was measured from 14% quicker to 41% slower than Best quality (source: 0.4.0, QG-05 measurement).
- On the Intel driver tested on Ubuntu 24.04, hardware HEVC export is not usable (source: 0.4.0).
- Faster export has been verified on real hardware only on Linux with Intel integrated graphics (source: 0.4.0, QG-05 checklist).
- On a very busy computer the video hardware can still be missed; Faster then uses the normal method (source: 0.4.2).
- On computers with more than one graphics card NVIDIA is always preferred (source: 0.4.0).
- A 360 clip with a moving camera exports at about one third of real time (source: 1.2.6).
- 360 video must be a single full equirectangular picture; spatial sound turns with yaw and pitch, not roll, for four-channel 360 clips (assumed channel order) (source: 1.2.6, 2.0.4 REQ-273).
- A 360 export to MP4 and MOV is marked as 360 video; Matroska and WebM carry the marker when the file check passes (source: 1.3.0, 2.0.4 REQ-261).
- A 360 video ignores the video format presets and keeps the sphere's own size (source: 1.4.0).
- A 360 video export ignores Change aspect, Scale and Move; the flat camera view applies them (source: 1.5.0).
- Change aspect, Scale and Move make the export a re-encode; a clip with a speed too, and two clips with different speeds cannot be merged (source: 1.5.0, 2.0.4 REQ-281).
- The longer side of a frame is capped at 4096 px (source: 1.5.0).
- The Export dialog remembers its choices for the session only; the video and audio formats are a fixed list of presets (source: 1.4.0).
- Apply LUT is applied before the frame effect of Change aspect, so the blurred or coloured fill is not graded (source: 1.6.0, accepted deviation).
- The preview copy cache is kept under a 4 GB limit; the cleanup runs on the interface thread (source: 2.0.4 REQ-297).
- Under Automatic, a 1280x720 source never gets a preview copy (source: 0.2.0).
- Clean audio at full strength does not remove noise completely; quiet speech (about -32 dB) can lose up to about 2.5 dB (source: 1.4.0, 2.0.4 REQ-278).

Subtitles and speech
- Chinese, Japanese and Korean subtitles and texts use the installed fonts and may show empty boxes; Arabic and Devanagari have bundled fonts (source: 0.2.0, 2.0.4 decision D61).
- Speech is recognised in pieces of about two minutes; a word cut at a boundary can be lost (source: 0.3.0).
- When a video's own subtitles are used, the word-by-word highlight timing is approximate (82% of the words matched, average start error 0.48 s on the reference clip) (source: 1.2.0, REQ-095, REQ-177).
- Matching a video's subtitles to the speech works only as well as their text follows what is said (source: 1.2.0).
- Moved subtitle lines shown at the same time can overlap (source: 1.2.0).
- The .srt file has no subtitle positions (source: 1.2.0).
- Align horizontally does nothing for a subtitle line that is still at its style's place (source: 1.2.6).

Editing, preview and projects
- Removing a file from the media list drops the undo steps that refer to it (source: 0.2.0).
- A project file with a text whose time to live is above 24 hours, or below 0.1 s, is refused as damaged on opening (source: 1.3.0).
- Texts of projects from before 2.0.3 open with Readability None and take the measured size of their box (source: 2.0.3, REQ-155..167).
- A video without spherical information is always flat (source: 1.4.0).
- A text or a subtitle line dragged past an edge of the picture is cut in the preview and in the export (source: 1.4.0).
- A text's size is never above 10% of the picture height (source: 2.0.3, REQ-156).
- Text items use the installed fonts and are not written to the .srt file (source: 1.3.0).
- Subtitle lines and text items keep their place when the picture is scaled or moved (source: 1.5.0).
- Clips added after a whole-video effect do not get it; a note appears (source: 1.6.1, 2.0.4).
- A 1.6.0 clip with its own and a whole-video Scale or Move becomes one combined path (source: 1.6.1).
- Keyframes are straight lines unless a segment is eased (source: 1.5.0, 2.0.4 REQ-282).
- Apply LUT reads .cube, .3dl and .csp; a missing LUT stops the export until found, or the export is made without it (source: 1.6.0, 2.0.4 REQ-292).
- PNG picture items are not drawn in a 360 video export (source: 2.0.4 REQ-287).

Player
- The player's picture subtitles are not shown on 360 video; a picture subtitle already on screen at a start or a pause appears at its next change (source: 2.0.4 REQ-253).

## Consistency checks (2.0.4 close)
- Read in full: README, docs/roadmap.md, docs/changelog.md (2.0.4 part), the 2.0.4 checkpoint and the batches-12-15 page.
- README: stale limitations and usage text corrected to the built state (2.0.4 features, the window memory, the player, LUT, Ctrl/Cmd+V, macOS menu, 360 text); Latest changes 2.0.4, 2.0.3, 2.0.2; badge 94.9 via `scripts/coverage.sh --badge`; poem unchanged.
- Roadmap: delivered 2.0.4 items removed (speed and pictures are new, anchored 360 text and paths, picture subtitles partly, quality items), 2.0.5 items added (manual checks, roll and pre-pass, picture subtitles on 360 and at a start or pause, picture item limits, REQ-177 decision kept, blurred fill and REQ-206 kept), Enhance speech model moved to Not planned.
- Splash summaries: editor and player summaries rewritten for the built 2.0.4 items, five lines each.
- docs/architecture-2.0.4-overview.md: a forbidden reference to `ai-docs/` removed (docs-check).

## Open items (documenter)
- The known-limitations hand-off above is from 1.6.3 with the 2.0.4 corrections; the next RELEASE-CLEAN rewrites it from the README.
- Roadmap item "Migrated text sizes" is kept for the option to offer the old size again.
- Roadmap item "Gated alignment test on CI's versions" is kept: not checked whether the real-speech test runs on CI with the pinned speech fixture.
- Picture item limits (ADR-192) and REQ-177 are open decisions; the roadmap keeps them.
- Other docs/*.md not compacted in 2.0.4 (owners clean their own docs at release).

## Development log
- 1.0.0 to 1.6.2: one line per development in the changelog and in the earlier notes (compacted in 1.6.3; the detailed per-part checks of those versions are not kept here).
- 1.6.3: version 1.6.3 (no change needed in RELEASE-CLEAN), changelog entry Changed (splash file names) and Workflow (release flow, project page rules, /analysis), no user-facing change, README latest changes unchanged.
- 1.6.3 RELEASE 1.6.3 "core effects and player": changelog release summary and a Changed bullet (splash origin and licence: own work, MIT); roadmap: splash origin item and hardware-document split removed, dispatch_inner, module cycles and 360 test pictures added to priority 11; docs updated on the splash origin; coverage badge 95.6; README unchanged (1.6.3 is workflow-only, latest changes are 1.6.2, 1.6.1, 1.6.0); docs-check ok, version check ok.
- 1.6.3 RELEASE-CLEAN: known limitations hand-off, github_page/after release.md with the Documenter section, roadmap cleanup, README known limitations cleanup, architecture.md label, workflow LICENSE wording, notes compacted, docs-check ok, version 1.6.3 unchanged, poem unchanged, releases/ not touched.
- 2.0.0 POEM: new README poem written after 1.6.3 release (opening of 2.0.0), docs-check ok, version 2.0.0 consistent.
- 2.0.1 to 2.0.3: one line per part in the changelog (business, architecture, code); the detailed checks are not kept here.
- 2.0.4 business, architecture, checkpoint: version set to 2.0.4; changelog entry; the 2.0.4 architecture bullets were kept in the checkpoint file until built (ai-docs/roadmap-2_0-checkpoint.md, section Documenter); docs-check ok.
- 2.0.4 close (2026-11-21 19:27 +0000, final pass): version 2.0.4 consistent; coverage badge 94.9 (`scripts/coverage.sh --badge 94.9`); changelog 2.0.4 bullets for speed, picture items, per-clip aspect, 360 text and paths, spatial sound, player picture subtitles and quality items; README Latest changes 2.0.4 updated, Usage and Known limitations corrected to the built state, poem unchanged; roadmap: 2.0.4 items removed and the 2.0.5 items added (manual checks, roll, pre-pass, picture subtitle limits, picture item limits, CI QG-06 items already present); splash summaries (editor and player) rewritten within five lines; docs/architecture-2.0.4-overview.md forbidden ai-docs reference removed; docs-check ok, version check ok. The checkpoint file ai-docs/roadmap-2_0-checkpoint.md is kept (not deleted).
