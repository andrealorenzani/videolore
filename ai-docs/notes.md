# ai-notes: documenter notes

Owner: documenter. Short notes for AI. The index is refreshed with `scripts/doc-index.sh <file>` after each change of a managed doc; read a paragraph with `scripts/doc-section.sh` or `sed -n 'A,Bp'`.

## Index of managed docs
Managed by the documenter: README.md, docs/roadmap.md, docs/changelog.md, docs/workflow.md (not to be edited in this round), docs/scripts.md and docs/andrea-choices.md (documented here, Andrea's choices list). Other docs/*.md belong to their owners, who compact them.

- README.md § videolore — L1-25 (25 lines)
- README.md § What it is — L26-28 (3 lines)
- README.md § System requirements — L29-34 (6 lines)
- README.md § Installation — L35-41 (7 lines)
- README.md § Windows (x86_64) — L42-47 (6 lines)
- README.md § macOS (Apple Silicon) — L48-53 (6 lines)
- README.md § Linux (x86_64, Ubuntu LTS and Fedora) — L54-60 (7 lines)
- README.md § From source — L61-69 (9 lines)
- README.md § Usage — L70-112 (43 lines)
- README.md § Known limitations — L113-191 (79 lines)
- README.md § Latest changes — L192-197 (6 lines)
- README.md § License — L198-203 (6 lines)
- README.md § Ode to the creator — L204-215 (12 lines)
- docs/roadmap.md § Roadmap — L1-4 (4 lines)
- docs/roadmap.md § Priority 1 — Release checks — L5-18 (14 lines)
- docs/roadmap.md § Priority 2 — Text and subtitles on the picture — L19-29 (11 lines)
- docs/roadmap.md § Priority 3 — Subtitle accuracy and subtitle features — L30-46 (17 lines)
- docs/roadmap.md § Priority 4 — Editing speed and safety — L47-52 (6 lines)
- docs/roadmap.md § Priority 5 — Export speed and hardware — L53-69 (17 lines)
- docs/roadmap.md § Priority 6 — Installation and platforms — L70-79 (10 lines)
- docs/roadmap.md § Priority 7 — Interface — L80-84 (5 lines)
- docs/roadmap.md § Priority 8 — Video player — L85-90 (6 lines)
- docs/roadmap.md § Priority 9 — 360 video — L91-103 (13 lines)
- docs/roadmap.md § Priority 10 — Effects and frame — L104-130 (27 lines)
- docs/roadmap.md § Priority 11 — Quality, tests and measurements — L131-154 (24 lines)
- docs/roadmap.md § Priority 12 — Splash, project page and release process — L155-173 (19 lines)
- docs/roadmap.md § Not planned — L174-190 (17 lines)
- docs/changelog.md § Changelog — L1-4 (4 lines)
- docs/changelog.md § 2.3.0 — 2026-11-22 11:17 +0000 — L5-5 (1 lines)
- docs/changelog.md § Release 2.3.0 — Multiple overlays and effect stack and order — L6-30 (25 lines)
- docs/changelog.md § 2.2.0 — 2026-11-21 22:07 +0000 — L31-44 (14 lines)
- docs/changelog.md § 2.1.0 — 2026-11-21 19:43 +0000 — L45-52 (8 lines)
- docs/changelog.md § 2.0.4 — 2026-11-21 00:15 +0000 — L53-71 (19 lines)
- docs/changelog.md § 2.0.3 — 2026-11-20 22:05 +0000 — L72-83 (12 lines)
- docs/changelog.md § 2.0.2 — 2026-11-20 20:34 +0000 — L84-88 (5 lines)
- docs/changelog.md § 2.0.1 — 2026-11-20 20:06 +0000 — L89-93 (5 lines)
- docs/changelog.md § 2.0.0 — 2026-11-20 19:50 +0000 — L94-95 (2 lines)
- docs/workflow.md § Agentic workflow — L1-4 (4 lines)
- docs/workflow.md § Agents — L5-21 (17 lines)
- docs/workflow.md § Commands — L22-31 (10 lines)
- docs/workflow.md § `/add-feature` pipeline — L32-50 (19 lines)
- docs/workflow.md § `/release` pipeline — L51-69 (19 lines)
- docs/workflow.md § Rules (full text in `AGENTS.md`) — L70-85 (16 lines)
- docs/workflow.md § Token economy — L86-96 (11 lines)
- docs/workflow.md § Skills — L97-99 (3 lines)
- docs/workflow.md § CI — L100-102 (3 lines)
- docs/workflow.md § Scripts — L103-116 (14 lines)
- docs/workflow.md § Hooks and settings — L117-119 (3 lines)
- docs/workflow.md § Repository — L120-122 (3 lines)
- docs/workflow.md § Where things live — L123-138 (16 lines)
- docs/workflow.md § Workflow manifest — L139-156 (18 lines)
- docs/workflow.md § Workflow history — L157-172 (16 lines)
- docs/scripts.md § Scripts that read the documents — L1-13 (13 lines)
- docs/scripts.md § Requirements — L14-16 (3 lines)
- docs/scripts.md § Reading a long document — L17-19 (3 lines)
- docs/andrea-choices.md § Choices made by Andrea — L1-4 (4 lines)
- docs/andrea-choices.md § Process — L5-27 (23 lines)
- docs/andrea-choices.md § Text, subtitles and LUT — L28-35 (8 lines)
- docs/andrea-choices.md § Speed, overlap and effects (2.3.0) — L36-43 (8 lines)
- docs/andrea-choices.md § Open items — L44-45 (2 lines)

## Release hand-off: known limitations
Cleaned in RELEASE-CLEAN 2.3.0 (sections of the README "Known limitations", which hold the same wording). Source in brackets: the version or REQ where the limit comes from; "README" = the README item had no version and its source is not recorded. Read by the Product Manager.

Installation and system
- Installers are not code-signed; one-time confirmation on each system. [2.0.2, Andrea's decision; signing not planned]
- Subtitles on Windows and Linux need AVX2, FMA and F16C; older processors get a message; tested with simulated processors only. [0.3.0 processor check]
- Drag and drop does not work on Linux Wayland. [1.1.0]
- macOS menu bar has no Quit item; Cmd+Q still asks about unsaved changes. [README; REQ-232 2.0.4 menu bar not checked on a real Mac]
- CJK subtitles and texts use the installed fonts (possible empty boxes). Arabic and Devanagari are bundled. [2.0.4 REQ-187/188, system fonts only, D61]
- Text items use installed fonts; a missing font is replaced by the bundled subtitle font with a note. [1.3.0 text items, 2.0.4 font rules]

Faster export (hardware)
- NVIDIA driver 610 or newer needed; older driver uses the normal method. [0.4.x hardware check; architecture non-functional targets]
- Linux (Intel, AMD): libva 2.21 or newer with the bundled ffmpeg (Ubuntu 24.04 has 2.20, hardware skipped); a system ffmpeg in Advanced can use hardware. [README; Linux hardware check]
- Fedora: RPM Fusion freeworld drivers for AMD and Intel. [README; Linux hardware check]
- Faster is not always quicker (14% quicker to 41% slower before 2.2.0; about 9% quicker in 2.2.0 on one Intel computer, target 20% not met). Files may be a little larger. [2.2.0, REQ-315 planned]
- Intel driver on Ubuntu 24.04: hardware HEVC not usable. [README; hardware check measurement]
- Verified on real hardware only on Linux with Intel integrated graphics. [0.4.0 QG-05]
- A busy computer can miss the hardware; then the normal method. [README; hardware check design]
- With several graphics cards NVIDIA is always preferred; the card cannot be chosen. [0.4.0]

Subtitles
- Reused subtitles: text and line times exact, word timing approximate; target not met (Fast: 82% of words, 0.48 s average start error; speech-region method 2.2.0: 43% within 0.3 s, target 90%). [2.2.0, REQ-177 planned]
- Matching works only as well as the text follows the speech; paraphrased subtitles get evenly spread words, with a message. [1.2.0]
- Moved subtitle lines shown together can overlap; a warning, no automatic move. [2.0.3, D53]
- The .srt file has no subtitle positions; text items are not written to it. [1.2.0 SRT export; 1.3.0 text items]
- Enlarge word and Typewriter are video effects only; .srt, .vtt and .ass keep plain text. Typewriter has no caret and no typing sound. [2.1.0]
- The shadow has one fixed distance and colour per style, not over time. [2.1.0]
- Scripts other than Latin, Greek and Cyrillic are not fully supported. [1.2.0]

Texts and boxes
- Text at most 10% of picture height unless its Maximum size is set (2% to 50%); a short text in a big box does not fill it; measured with the picture's font. [2.0.3 cap; 2.2.0 REQ-323, D64]
- Texts of projects saved before 2.0.3 open with Readability None and their box size; large multi-line texts may shrink; one-time message. [2.0.3, Andrea: "Sizes change on open"]
- Text or subtitle line dragged past the picture edge is cut in preview and export; status bar counts it; Move inside the picture brings it back. [1.4.0]
- Time to live above 24 h or below 0.1 s: project refused as damaged; while editing limited to the video length. [1.3.0 deviation]
- Subtitle lines and text items do not follow Scale or Move. [1.5.0]
- Overlap media and PNG picture items are not drawn in a 360 video export; picture items take no text effects. [2.3.0 REQ-327..331, 2.0.4]
- Overlap MOV: measured when added; no sound played; no keyframes or rotation; HEVC with alpha and other codecs without alpha read are drawn opaque. [2.3.0]
- Overlay above 1920 x 1080: half size in preview; a 4000 x 4000 PNG takes about 830 MB in export; a still is decoded per frame (4 s still about 5 s export). [2.3.0]

Editing, effects and LUTs
- Removing a media file drops the undo steps that refer to it. [README; undo history 1.1.0]
- Change aspect, Scale and Move make the export re-encode even at defaults; a clip with speed is re-encoded and clips with different speeds cannot be merged. [1.5.0; Change speed 2.3.0 REQ-281]
- Frame longer side capped at 4096 px; large frames export slower (not measured). [1.5.0, assumption]
- Keyframes are straight unless eased; shown in the popup and on the clip marker, not as timeline markers. [1.5.0; easing 2.0.4 REQ-282]
- Clips added after a whole-video effect do not get it; a note appears. [1.6.1 REQ-147]
- Whole-video Scale or Move is a separate copy per clip, edited in each clip's popup. [1.6.1 REQ-148; 2.0.4 REQ-200]
- A 1.6.0 clip with its own and a whole-video Scale or Move becomes one combined path on opening (exact at keyframes, straight line between). [1.6.1 migration]
- Clean audio at full strength does not remove noise completely; quiet speech (about -32 dB) can lose up to about 2.5 dB; cost on weak computers not measured. [1.4.0]
- Apply LUT: .cube up to 256 points and 512 MiB, loaded in the background with progress; preview of a 256-point LUT takes about 12 s to prepare (dev computer); export always uses it. [2.1.0 REQ-300/301, 2.2.0 REQ-312, D63]
- .3dl and .csp read whole, limit 64 MiB. [2.1.0 REQ-300]
- A missing LUT file stops the export until found, or export without the LUT. [1.6.0 REQ-292]
- Closing a project while a LUT loads: status error and the LUT is applied to a missing target. [2.2.0]
- LUT below Change aspect by default: the fill is not graded; a LUT moved above grades it too. [2.2.0 REQ-320]
- Order of effects cannot be copied from one clip to another; subtitle lines of a clip are one row in Applied effects; Change speed fixed. [2.2.0 REQ-320; 2.3.0]
- On 360 clips, anchored texts and subtitle lines keep fixed rows; Enlarge word and Typewriter drawn as plain text; 360 export ignores the order. [2.2.0 REQ-320; 2.0.4]
- Crop removed; an old crop is dropped on opening with one status line. [2.3.0 REQ-283]
- No 360 switch: a video without spherical information is always flat. [1.4.0]
- Copy and Cut put marker text on the clipboard. [2.0.4 REQ-195]

360 video, player and preview
- 360 video: single equirectangular picture only (no stereo 3D, cube maps, partial 180). [1.2.6; 2.0.4]
- Spatial sound follows yaw and pitch, not roll; four-channel 360 audio read as first-order ambisonics (assumed channel order). [2.0.4 REQ-273, D61]
- A 360 clip with a moving camera exports at about one third of real time. [1.2.6]
- 360 video ignores format presets and Change aspect, Scale and Move (the flat camera view applies them); keeps the sphere size. [1.3.0; 2.0.4 REQ-266]
- MP4 and MOV 360 files carry the marker; Matroska and WebM only when the check passes. [2.0.4 REQ-261]
- Player picture subtitles are not shown on 360 video; one already on screen when playback starts inside it appears at its next change. [2.0.4]
- Preview: spatial sound waits for its pre-pass; a clip over 10 minutes keeps its recorded sound there. [2.0.4]
- Preview copies: under Automatic, a 1280x720 source gets none; the cache is limited to 4 GB (Settings), oldest removed first, cleanup on the interface thread. [2.0.4 cache limit]
- A project mixing 360 and flat clips exports only as the flat camera view. [1.3.0]
- Video edits do not move text items. [1.3.0; not in README: check whether still true]

Export dialog, projects
- Export dialog choices last for the session only; formats are a fixed list of presets. [1.4.0; 2.0.4 presets kept]
- Project format 14 (2.3.0) is refused by 2.2.0 and earlier; format 13 (2.2.0) by 2.1.0 and earlier; format 12 (2.1.0) by 2.0.4 and earlier; 2.0.4 by 2.0.3; 2.0.3 by 2.0.2; 1.4.0 or later by 1.3.0 and earlier. Refused with a message, left untouched. [format history, architecture-projects]
- Versions before 1.1.0 drop the saved layout and recent-projects list when saving settings. [1.1.0]
- Projects saved by 2.2.0 keep the order per clip; a 1.3.0 project's 360 switch is dropped on opening. [2.2.0; 1.4.0]

## Consistency checks (RELEASE-CLEAN 2.3.0)
- README: Known limitations rewritten (60 items in plain wording, grouped; duplicates merged; fixed items and history dropped; two real limits restored: clipboard and scripts). Usage, Latest changes and poem unchanged. Badges unchanged (version 2.3.0, coverage 95.1 from the last code run).
- docs/business.md: Documents, key decisions and limitations rewritten by topic (per-version narrative removed; the limitations point to the README). Links to all business-*.md kept.
- docs/architecture.md: index rewritten by topic (version narrative removed); all architecture-*.md are linked; NFR targets kept with their figures.
- docs/andrea-choices.md: new; the list of Andrea's choices (AskUserQuestion answers, his instructions, decisions marked as his in the docs).
- docs-check runs in the index mapping for docs/andrea-choices.md (mapped to docs/business.md).

## Open items (documenter)
- Not compacted in this round (owners' documents): docs/code*.md, docs/architecture-*.md sub-documents (per-version pages such as code-2.0.4-batches-*, code-2.1.0, code-2.2.0, code-2.3.0, architecture-2.x overviews), docs/roadmap.md content (Product Manager writes it in RELEASE), docs/roadmap_tasks.md, docs/changelog.md (not cleaned by rule). README Usage (about 15 KB) is user-facing and kept; it could move to docs/usage.md in a later round.
- docs/workflow.md not edited (the user asked to keep AGENTS.md and workflow docs out of this round); its Scripts table does not yet list the new read-only scripts: docs/scripts.md describes them.
- AGENTS.md Scripts list: proposed line in the return message (to be added through /change-workflow).
- Roadmap item "Video edits do not move text items" to check (from the 1.3.0 limits, not in the README).

## Development log
- 1.6.3 RELEASE-CLEAN and RELEASE: known limitations hand-off, page data file, roadmap cleanup, changelog release summary, coverage badge.
- 2.0.0 POEM: new README poem after the 1.6.3 release.
- 2.0.1 to 2.0.3: one changelog entry per part; detailed checks not kept here.
- 2.0.4 business, architecture, code and close: changelog entries, roadmap, README latest changes, coverage 94.9 then close.
- 2.1.0 business, architecture, code, close: version set, changelog entries, README usage and known limitations, coverage 95.0.
- 2.2.0 business, architecture, code, close: changelog, roadmap, README; coverage 95.1 (badge via script, no test run).
- 2.3.0 business, architecture, code, close: version set, changelog, roadmap, README latest changes, coverage 95.1.
- 2.3.0 RELEASE (2026-11-22): changelog release summary "Multiple overlays and effect stack and order" (2.0.x to 2.3.0 highlights), docs-check links fixed (changelog, code-360, code-export, code-frame), README coverage badge 95.1 and format 14 note, docs-check ok, version and notices checks ok. Not changed: version, poem, roadmap, releases/.
- 2.3.0 RELEASE-CLEAN (2026-11-22): README known limitations cleaned; docs/business.md and docs/architecture.md compacted by topic; docs/andrea-choices.md and docs/scripts.md created; read-only scripts req.sh, reqs-status.sh, doc-section.sh, doc-index.sh added; docs-check maps andrea-choices to the business index; github_page/after release.md with the Documenter section; version unchanged (2.3.0), changelog and releases/ untouched, poem unchanged.
