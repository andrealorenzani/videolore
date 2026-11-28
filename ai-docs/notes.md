# ai-notes: documenter notes

Owner: documenter. Short notes for AI. The index is refreshed with `scripts/doc-index.sh <file>` after each change of a managed doc; read a paragraph with `scripts/doc-section.sh` or `sed -n 'A,Bp'`.

## Index of managed docs
Managed by the documenter: README.md, docs/roadmap.md, docs/changelog.md, docs/workflow.md, docs/scripts.md and docs/andrea-choices.md (documented here, Andrea's choices list). Other docs/*.md belong to their owners, who compact them.

- README.md § videolore — L1-27 (27 lines)
- README.md § What it is — L28-30 (3 lines)
- README.md § System requirements — L31-36 (6 lines)
- README.md § Installation — L37-43 (7 lines)
- README.md § Windows (x86_64) — L44-49 (6 lines)
- README.md § macOS (Apple Silicon) — L50-55 (6 lines)
- README.md § Linux (x86_64, Ubuntu LTS and Fedora) — L56-62 (7 lines)
- README.md § From source — L63-71 (9 lines)
- README.md § Usage — L72-115 (44 lines)
- README.md § Known limitations — L116-206 (91 lines)
- README.md § Latest changes — L207-213 (7 lines)
- README.md § Community — L214-219 (6 lines)
- README.md § License — L220-225 (6 lines)
- README.md § Ode to the creator — L226-237 (12 lines)
- docs/changelog.md § Changelog — L1-4 (4 lines)
- docs/changelog.md § 3.2.0 — 2026-11-28 15:22 +0000 — L5-5 (1 lines)
- docs/changelog.md § Added — L6-12 (7 lines)
- docs/changelog.md § Changed — L13-17 (5 lines)
- docs/changelog.md § 3.1.3 — 2026-11-26 21:01 +0000 — L18-18 (1 lines)
- docs/changelog.md § Added — L19-28 (10 lines)
- docs/changelog.md § Fixed — L29-32 (4 lines)
- docs/changelog.md § 3.1.2 — 2026-11-26 19:03 +0000 — L33-33 (1 lines)
- docs/changelog.md § Added — L34-38 (5 lines)
- docs/changelog.md § Changed — L39-40 (2 lines)
- docs/changelog.md § Fixed — L41-42 (2 lines)
- docs/changelog.md § Workflow — L43-45 (3 lines)
- docs/changelog.md § 3.1.1 — 2026-11-26 00:20 +0000 — L46-46 (1 lines)
- docs/changelog.md § Workflow — L47-50 (4 lines)
- docs/changelog.md § 3.1.0 — 2026-11-24 19:37 +0000 — L51-51 (1 lines)
- docs/changelog.md § Added — L52-56 (5 lines)
- docs/changelog.md § Changed — L57-60 (4 lines)
- docs/changelog.md § Fixed — L61-62 (2 lines)
- docs/changelog.md § Workflow — L63-64 (2 lines)
- docs/changelog.md § Tests — L65-67 (3 lines)
- docs/changelog.md § 3.0.0 — 2026-11-22 17:22 +0000 — L68-69 (2 lines)
- docs/roadmap.md § Roadmap — L1-4 (4 lines)
- docs/roadmap.md § Priority 1 — Release checks and decisions — L5-24 (20 lines)
- docs/roadmap.md § Priority 2 — Text and subtitles on the picture — L25-35 (11 lines)
- docs/roadmap.md § Priority 3 — Subtitle accuracy and subtitle features — L36-52 (17 lines)
- docs/roadmap.md § Priority 4 — Editing speed and safety — L53-62 (10 lines)
- docs/roadmap.md § Priority 5 — Overlap media and picture items — L63-79 (17 lines)
- docs/roadmap.md § Priority 6 — Export speed and hardware — L80-96 (17 lines)
- docs/roadmap.md § Priority 7 — Installation and platforms — L97-106 (10 lines)
- docs/roadmap.md § Priority 8 — Interface — L107-113 (7 lines)
- docs/roadmap.md § Priority 9 — Video player — L114-119 (6 lines)
- docs/roadmap.md § Priority 10 — 360 video — L120-134 (15 lines)
- docs/roadmap.md § Priority 11 — Effects, frame and LUT — L135-151 (17 lines)
- docs/roadmap.md § Priority 12 — Quality, project page and release process — L152-188 (37 lines)
- docs/roadmap.md § Priority 13 — Screen recorder — L189-207 (19 lines)
- docs/roadmap.md § Accepted limits (explained, not planned) — L208-226 (19 lines)
- docs/roadmap.md § Not planned — L227-243 (17 lines)
- docs/workflow.md § Agentic workflow — L1-4 (4 lines)
- docs/workflow.md § Agents — L5-21 (17 lines)
- docs/workflow.md § Commands — L22-31 (10 lines)
- docs/workflow.md § `/add-feature` pipeline — L32-50 (19 lines)
- docs/workflow.md § `/release` pipeline — L51-69 (19 lines)
- docs/workflow.md § Rules (full text in `AGENTS.md`) — L70-85 (16 lines)
- docs/workflow.md § Token economy — L86-96 (11 lines)
- docs/workflow.md § Skills — L97-103 (7 lines)
- docs/workflow.md § CI — L104-106 (3 lines)
- docs/workflow.md § Scripts — L107-121 (15 lines)
- docs/workflow.md § Hooks and settings — L122-124 (3 lines)
- docs/workflow.md § Repository — L125-127 (3 lines)
- docs/workflow.md § Where things live — L128-143 (16 lines)
- docs/workflow.md § Cloning the workflow — L144-157 (14 lines)
- docs/workflow.md § Workflow history — L158-177 (20 lines)
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
- 3.2.0 open for the release hand-off: business R-T6 must match ADR-281 (Product Manager); release notes must say that 3.1.x refuses 3.2.0 projects (format 16) and that a project as media follows its source only on reopen; the speaker echo risk is in the roadmap (Priority 13).

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
- 3.1.0 business (2026-11-24): changelog entry 3.1.0; roadmap future work (REQ-333 test rewrite, Enlarge word and Typewriter in the subtitle track, ASS track, several audio tracks in one output, release notes items, manual checks at STATUS); docs-check ok; version set 3.1.0 (consistent). README unchanged (planned work only, no user-facing change delivered yet).
- 3.1.0 architecture (2026-11-24): changelog bullet (format 15 on every save, release note "projects saved by 3.1.0 open only in 3.1.0 or later"); roadmap: 3.1.0 measurements (Priority 1), lossless non-first audio track and Subtitles popup audio track memory (Priority 11), split of app.rs and ui/popups.rs (881 lines) merged into the existing split item; docs-check ok; version unchanged (3.1.0). notices.sh not run (not a documenter script; cargo about writes packaging/notices; needed by the Principal Engineer when dependencies change). README unchanged (no user-facing change yet).
- 3.1.0 code (2026-11-24): coverage badge 95.1 (script --badge, no test run; PE result: line 95.1%, feature 320/320 by script, 338/338 by hand); changelog 3.1.0 rewritten as Added/Changed/Fixed/Workflow (REQ-334 to REQ-340); README latest changes 3.1.0, 2.3.0, 2.2.0; README usage steps 5, 6, 13, 15, 16 and known limitations (subtitles, editing, project formats) updated; splash summary.txt and summary-player.txt rewritten; roadmap future work added (feature-coverage status format, pinned ffmpeg run, As-track preview, Effects scroll at 800 px, popup audio memory note, release notes), REQ-333 test rewrite item removed as delivered; docs-check ok; version 3.1.0 consistent. Index check: ai-docs/business.md ranges for docs/business.md and docs/business-verification.md match scripts/doc-index.sh (heading texts differ in the index, not in ranges).
- 3.1.0 close (2026-11-24): README checked (latest changes 3.1.0, 2.3.0, 2.2.0; poem 10 verses, unchanged; badges 3.1.0 and 95.1 match); docs-check ok; version check consistent. No files changed.
- 3.1.0 code fix (2026-11-24): flaky preview test a_still_is_rendered_again_after_a_superseded_request now waits for shown_key == K1 (REQ-144); changelog 3.1.0 gained a Tests bullet (not user-facing); roadmap Priority 12 item on tests waiting for state; index lines of roadmap and changelog refreshed; docs-check ok; version 3.1.0 consistent. README and poem unchanged.
- 3.1.2 business (2026-11-26): version set 3.1.2 (consistent; was 3.1.1, skipped by Andrea's choice); changelog 3.1.2 added (Added: plans for videolore-screen-spy REQ-341..352, 357 and player playlist REQ-355, 356, not built); roadmap has no 3.2.0 reference (nothing to rename); README unchanged (planned work only); docs-check ok. version.sh check warns notices out of date (packaging/notices, not run by the documenter).
- 3.1.2 architecture (2026-11-26): changelog 3.1.2 gained the architecture bullet (16 decisions ADR-255..270, docs/architecture-screen-spy.md) and the REQ-358 and amended-plan bullet; roadmap Priority 13 "Screen recorder" added (Wayland, region and window, pause, hotkeys, sound source or tracks, level control, hardware encoders); docs/architecture.md status line fixed (3.0.0 to 3.1.2 are in the changelog, not released); docs-check ok; version consistent. README unchanged (no user-facing change built).
- 3.1.2 code (2026-11-26): coverage badge 95.0 (script --badge, no test run; PE result: line 95.0%, feature 336/336); changelog 3.1.2 Added/Changed/Fixed rewritten as built (recorder, REQ-358, player playlist, editor refused-import fix); roadmap Priority 13 why line and seven items added (macOS system-audio permission wait, Start speed, Linux default-source check, Windows/macOS measurements, real-system checks 38-41 and macOS 11-14.5, automated capture test on Windows/macOS); README: badges display-info and jiff, installation line for the recorder, usage 15 playlist, usage 17 screen recorder, Known limitations group Screen recorder, latest changes 3.1.2, 3.1.0, 2.3.0 (2.2.0 dropped); poem unchanged; docs-check ok; version 3.1.2 consistent. Changed docs by others: none.
- 3.1.2 close (2026-11-26): version 3.1.2 consistent (no change); docs-check ok; README latest changes 3.1.2, 3.1.0, 2.3.0 checked (3.1.1 is workflow-only per changelog), recorder named videolore screen recorder as in the changelog; poem unchanged. Inconsistency left for the owner: assets/splash/summary.txt calls the program "videolore screen spy" while README, changelog and business docs say "videolore screen recorder" (videolore-screen-spy is the binary name). Changed docs by others: none.
- 3.1.2 close (coordinator correction): README and docs/changelog.md now say "videolore screen spy" (program name, as in the window title, About, installers and splash); "screen recorder" stays as the plain description; the splash inconsistency is resolved. docs-check run after the replace.
- 3.1.3 business (2026-11-26): version set 3.1.3 (consistent; was 3.1.2); changelog 3.1.3 added (Added: business plan of GitHub community standards REQ-359..366, planned, docs/business-community.md); docs/business.md changed by the PM (key decision line, Community index entry, 3.1.3 section; not mine). REQ-366 wording reworded via doc-replace (no forbidden term in docs/, per docs-check); roadmap unchanged (no future items passed); README unchanged (no user-facing change built); docs-check ok. version.sh set prints "notices are out of date: run scripts/notices.sh" (not run by the documenter). Business index ai-docs/business.md is the PM's file: not refreshed by the documenter.
- 3.1.3 architecture (2026-11-26): community files written (CODE_OF_CONDUCT.md from the Contributor Covenant 2.1 source, placeholder replaced by community@andrealorenzani.name, attribution kept; CONTRIBUTING.md, SECURITY.md, SUPPORT.md, .github/ISSUE_TEMPLATE/{bug_report.yml,feature_request.yml,config.yml}, .github/pull_request_template.md); README Community section before License; changelog 3.1.3 Added bullets REQ-359..365; README and changelog index lines refreshed. Covenant fetched from the upstream EthicalSource/contributor_covenant release branch (the contributor-covenant.org path returned 404), into a new directory under the scratchpad. Poem, LICENSE and version unchanged. Changed docs by others: none (docs/business-community.md is the PM's; docs/architecture-build.md is the architect's).
- 3.1.3 documenter code (2026-11-26): CONTRIBUTING.md fixed (local checks for the three programs with --all/--workspace, feature coverage row, build prerequisites from build.yml, README, docs/architecture-build.md and docs/code-testing.md, ffmpeg pin 9.0.2); changelog 3.1.3 gained the Fixed bullet (two flaky hardware timing tests) and the community tests bullet; roadmap Priority 12 gained two items (LUT timing test on a loaded machine, docs-check does not scan community files and .github templates); coverage badge 95.0 (script --badge, no test run; PE result: line 95.0%, feature 336/336); docs-check ok; version 3.1.3 consistent. README unchanged (no user-facing change in this part). Changed docs by others: none.
- 3.1.3 code correction (2026-11-26): CONTRIBUTING.md, .github/pull_request_template.md and README build prerequisites corrected (libxcb-randr0-dev, g++, no-default-features scope, ffmpeg check only with VIDEOLORE_REQUIRE_FFMPEG=1 for the editor, real_x11 only with VIDEOLORE_REQUIRE_CAPTURE=1, CI XDG_SESSION_TYPE=x11 and non-monitor mic); required strings kept; changelog 3.1.3 Fixed bullet added; docs-check ok; version 3.1.3 consistent; poem and latest changes unchanged. Changed docs by others: CONTRIBUTING.md and .github/pull_request_template.md (approved edit by the coordinator, not in docs/).
- 3.1.3 close (2026-11-26): roadmap Priority 12 gained the stricter pull request template test item (five commands); README From source Linux packages fixed to one apt command and the C++ compiler named once (README-only); workflow history line for 3.1.3 (clone manifest skips community templates); index lines of roadmap and workflow refreshed with doc-index.sh; coverage badge 95.0 already matches the PE result (line 95.0%, feature 344/344); Latest changes unchanged (community files and the prerequisites fix are not program changes); docs-check ok; version 3.1.3 consistent. Changed docs by others: none.
- 3.2.0 business (2026-11-28): version set 3.2.0 (consistent; was 3.1.3; README badge changed by the script); changelog 3.2.0 added (Added: REQ-367..377 planned, one bullet per topic; Changed: business index and key decisions); roadmap unchanged (the PM's future work is for the architect and the verification doc, the risks are not roadmap items); README unchanged (no user-facing change built); docs-check ok (index line ranges of ai-docs/business.md accepted by the check). version.sh set prints "notices are out of date: run scripts/notices.sh" (not run by the documenter, the principal-engineer's step). Changed docs by others: docs/business-timeline.md (new), docs/business-editing.md, docs/business-player.md, docs/business-screen-spy.md, docs/business.md (PM).
- 3.2.0 architecture (2026-11-28): changelog 3.2.0 gained one architecture bullet (ADR-276..284, intermediate, format 16, strips, playlist total, PNG, recorder filter); roadmap: Priority 4 item project as media follows its source on reopen, Priority 12 item measurements of the 3.2.0 features, Priority 13 items echo cancellation and manual Linux microphone check; docs-check ok; version 3.2.0 consistent. README unchanged (no user-facing change built). Changed docs by others: docs/architecture*.md (architect), ai-docs/architecture*.md (architect).
- 3.2.0 code (2026-11-28): coverage badge 95.0% (scripts/coverage.sh --badge); changelog 3.2.0 gained the code bullet (REQ-367..377, 355/355, 95.0%) and the ffmpeg re-pin bullet; docs/architecture-packaging.md line 72 now names the BtbN 2026-11-27 build 9.0.2-982-g836df4e858 (was 2026-11-13); README Latest changes now 3.2.0, 3.1.2, 3.1.0 (2.3.0 dropped), 3.2.0 usage notes in Usage steps 1, 2 and 15, format 16 limitation; docs/architecture.md and docs/architecture-timeline.md "planned" for 3.2.0 changed to "built" (architect's docs, edited by the documenter); roadmap: URGENT ffmpeg pin item, manual checks 42-50, release notes 3.2.0, project-as-media cleanup and limits items, playlist total, recorder 25 ms and music items, picture strips and fixed-copy not planned; index refreshed by hand from doc-index.sh output.
- 3.2.0 close (2026-11-28): version consistent (3.2.0), docs-check ok. README: latest changes 3.2.0, 3.1.2, 3.1.0 (a 3.1.3 entry was added and reverted: community files are not program changes, as decided at the 3.1.3 close), blank line before Older changes restored, index lines refreshed; poem unchanged. Leftovers fixed: ADR heading Built in 3.2.0 (ADR-276 to 284) in docs/architecture-decisions.md with anchors in docs/architecture.md and docs/architecture-timeline.md; status words for 3.2.0 changed from planned to built in docs/architecture.md, architecture-timeline.md, architecture-components.md, architecture-player.md, architecture-nfr.md, architecture-risks.md, architecture-security.md, architecture-screen-spy.md, architecture-projects.md (line counts unchanged, indexes valid). 3.1.0 planned wording not touched. Changed docs by others: docs/architecture*.md (architect's, status words only).
