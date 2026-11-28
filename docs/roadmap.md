# Roadmap

Open items after version 3.2.0, most valuable and urgent first. Each band says why it comes there; each item says where it came from. Delivered items are removed (see the [changelog](changelog.md)). Suggested versions and batches: [roadmap tasks](roadmap_tasks.md).

## Priority 1 — Release checks and decisions
Why first: they need a real computer, player, ear or a decision by the maintainer.
- **URGENT: ffmpeg pin is a daily build** — the Linux and Windows pin of 3.2.0 (BtbN 2026-11-27, 9.0.2-982-g836df4e858) is likely removed around mid December 2026, and the package job then fails. Re-pin, or keep a project-held copy; alternatives are the month-end 2026-10-31 build (9.0.2-39) or moving to 9.1. (source: 3.2.0, ffmpeg pin)
- **Manual checks 42 to 50 of 3.2.0** — the import popup, the project as media, the ruler and strips, the playlist menu, the screenshot and the recorder noise reduction, by hand on a real system (verification document). (source: 3.2.0)
- **Manual checks of 2.0.4** — screen readers, the player under "Open with" on Windows, two-screen window places, clip speed by ear, picture items, 360 arrows, camera paths, player picture subtitles. Not a blocker. (source: 2.0.4)
- **Maintainer measurements for 2.0.3** — Balanced and Most accurate levels; installed font cost; font names on three systems (REQ-165). (source: 2.0.3)
- **Accuracy target of reused subtitles (REQ-177)** — decide: a larger speech model, or keep the target open. See Priority 3. (source: 2.0.3, decision)
- **Faster on integrated graphics (REQ-315)** — about 9% quicker against 20%; accept or keep the target. (source: 2.2.0, decision)
- **Second speech clip for REQ-177** — the maintainer records and commits `tests/fixtures/align/second-clip-30s.wav` with its transcript. (source: 2.2.0, user action)
- **Player builds on real systems** — Windows and macOS builds and installers run once. (source: 1.6.0)
- **Format 8 refusal by 1.5.x** — open a 1.6.0 project that uses a LUT with the real 1.5.0 build; it must be refused and left untouched. (source: 1.6.0)
- **Clean audio and MP3 by ear** — `afftdn` and `libmp3lame` on the bundled ffmpeg 9.x. (source: 1.4.0)
- **Windows and macOS test runs with the pinned ffmpeg** — the full suite ran on Linux only at 3.2.0; integration tests run on Linux in CI only. (source: 2.0.2, 3.2.0)
- **Moving-camera export cost on 9.0.2** — measure again on the bundled ffmpeg. (source: 2.0.2)
- **Manual checks of 3.1.0 at its status** — the checks listed in the verification document for the 3.1.0 requirements, by hand on a real system. (source: 3.1.0)
- **Measurements of 3.1.0 after the build** — time of the second-audio-track decode (T2) for a non-first track; proxy size per audio track; blurred fill pixels; Enlarge word baseline within 2 %. (source: 3.1.0 architecture)
- **Release notes of 3.1.0** — say that projects saved by 3.1.0 open only in 3.1.0 or later; that projects with Enlarge word look different after the upgrade (words move from the base); that the blurred fill is fixed; that "As subtitle track" differs between preview and export. (source: 3.1.0)
- **Release notes of 3.2.0** — say that 3.1.x and older cannot open projects saved by 3.2.0 (format 16); that a project imported as media follows its source only when it is reopened; that the playlist total shows dashes until every length is known; that the recorder loses the last 25 ms of sound. (source: 3.2.0)
- **Release process check** — run one release and review the consolidation. (source: 1.6.3)

## Priority 2 — Text and subtitles on the picture
Why: captions and titles are what social creators put on a video most often.
- **Migrated text sizes** — 1.3.0 texts take the best-fit size on opening; offer the old size only if asked. (source: 1.4.0)
- **Text and picture moves** — texts keep their place when the picture moves and may end past the edge; follow the picture or warn. (source: 1.5.0)
- **Text that follows video edits** — texts keep their time when the video is edited; decide. (source: 1.3.0)
- **Text in the .srt file** — decide whether text items are written to it. (source: 1.3.0)
- **Longer text times** — a time to live above 24 hours is refused on opening; raise the limit if needed. (source: 1.3.0)
- **Font scan repaint** — repaint when a font scan ends after a project opens. (source: 2.0.3)
- **CJK measuring font** — box widths of CJK text are approximate. (source: 1.2.0)
- **Subtitle font copy and padding on macOS and Windows** — check by hand with the font names (REQ-165). (source: 2.0.3)

## Priority 3 — Subtitle accuracy and subtitle features
Why: automatic subtitles are a core promise; the accuracy target is not met.
- **Meet the accuracy target (REQ-177)** — 82% of words matched and 0.48 s average start error, against 90% and 0.3 s; then re-enable the strict test. (source: 1.2.0, 2.2.0)
- **Pace-aware method for continuous speech** — the speech-region method reached 43%; judge a new method on the second clip. (source: 2.2.0)
- **Five-minute real-speech test (REQ-178)** — on a real five-minute clip. (source: 2.0.3)
- **Gated alignment test on CI's versions** — confirm with the ffmpeg and fixture CI uses. (source: 1.2.0)
- **Larger speech levels** — measure speed and memory of Balanced and Most accurate. (source: 0.3.0)
- **Replaced files** — re-check the track list of a file replaced on disk. (source: 1.2.0)
- **Editor preview of "As subtitle track"** — the preview draws the Classic look, not the chosen style; the track in the export has no styling. (source: 3.1.0)
- **Subtitle fonts for other scripts** — CJK and other scripts use installed fonts; bundled fonts need a licence check. (source: 0.2.0, 2.0.4)
- **Text from picture-based tracks** — OCR of DVD, Blu-ray and DVB subtitles. (source: 1.2.0)
- **Style per line or clip; custom style editor; translation; other speech models.** (source: 1.2.0)
- **Enlarge word and Typewriter in subtitle files** — decide whether a format can carry them, and draw them inside the "As subtitle track" track. (source: 2.1.0, 3.1.0)
- **Style-preserving ASS track for "As subtitle track"** — the exported track keeps the subtitle style instead of plain text. (source: 3.1.0, REQ-337 limit)
- **Typewriter caret and typing sound** — add if users ask. (source: 2.1.0)
- **Preview cache key for Typewriter** — the cache follows the word, not each letter. (source: 2.1.0)

## Priority 4 — Editing speed and safety
Why: these save time on every edit.
- **Double-click on a clip that already has the LUT** — keep its settings or ask. (source: 1.6.1)
- **Keyframe popup usability** — hard to pick keyframes in the small Scale and Move popup. (source: 1.5.0)
- **Time-based LUT progress** — the start progress is an estimate. (source: 2.2.0)
- **Undo steps lost when a media file is removed.** (source: 1.1.0)
- **Project as media follows its source on reopen** — a project imported as media is rendered once to a private intermediate file; later changes to the source project reach it only when the importing project is reopened. (source: 3.2.0, known limitation)
- **Cleanup and retry of project as media** — old videos in the imports folder are never deleted; a cancelled rebuild has no retry until the project is reopened; the source check runs on the window thread and every open re-reads the sources. (source: 3.2.0, known limitations)
- **Limits and cost of project as media** — 96 MiB per project file and 64 projects per import are fixed limits; the second encode uses about 84 MB of disk and 21 s per source minute, measured on a synthetic clip; decide the limits and the cost on real projects. (source: 3.2.0, measured)

## Priority 5 — Overlap media and picture items
Why: overlaps are new in 2.3.0; their limits show in real use.
- **Memory and time of large overlay files** — a 4000 x 4000 PNG peaks at about 830 MB; a still is decoded per frame. (source: 2.3.0)
- **Full-size preview of large overlays** — files above 1920 x 1080 are drawn at half size. (source: 2.3.0)
- **Alpha in hardware decoders** — the software path is used where alpha is dropped; check on real graphics. (source: 2.3.0, R1)
- **Drawn handles on the overlap box.** (source: 2.3.0)
- **Sound of MOV overlaps** — decide whether it plays with the clip. (source: 2.3.0)
- **Number of overlaps per clip** — decide whether a limit is needed. (source: 2.3.0)
- **Overlap lane zoom.** (source: 2.3.0)
- **Re-probe MOV overlays when used.** (source: 2.3.0)
- **Split keeps overlaps per half** — check that users expect it. (source: 2.3.0)
- **Overlay keyframes and rotation.** (source: 2.3.0)
- **Default place of new picture rows** — revisit with users. (source: 2.2.0, 2.3.0)
- **Picture item limits** — the other planned limits (20 items, 8 MiB) are not built; decide. (source: 2.0.4, 2.2.0)
- **Removal of old picture items** — remove the old kind in a later format change. (source: 2.3.0)
- **Overlap in 360 export** — not drawn; decide. (source: 2.3.0)

## Priority 6 — Export speed and hardware
Why: export time is what users wait for; hardware is measured on one system only.
- **Make Faster really faster on integrated graphics** — cheaper check or a target per kind of graphics. (source: 0.4.0, 2.2.0; REQ-206, REQ-315)
- **Decode and upload speed of hardware export** — about 7.5 s of an 11 s export on the reference project. (source: 2.2.0)
- **Busy computer and hardware check** — skip a candidate that times out twice; tell busy from hanging. (source: 0.4.2)
- **Verify hardware export on every system** — Windows (NVIDIA, Intel, AMD), Linux (NVIDIA, AMD), macOS (Apple). (source: 0.4.0)
- **Ubuntu 24.04 with the included ffmpeg** — libva 2.20 against 2.21; an ffmpeg build or compatible libva. Fedora needs RPM Fusion drivers. (source: 0.4.0)
- **Intel HEVC hardware** — broken on the tested driver; revisit. (source: 0.4.0)
- **Choose the graphics card** — NVIDIA is always preferred. (source: 0.4.0)
- **Hardware check of 360 export with texts, and of frame effects.** (source: 1.3.0, 1.5.0)
- **Export speed for 4K and HEVC presets.** (source: 1.4.0)
- **Export speed of large frames** — frames above 4096 px are slow; decide a limit. (source: 1.5.0)
- **Export time of blurred fill** — measure on real material. (source: 1.5.0)
- **Playback of a blurred fill and Scale in the preview** — render only the changed part. (source: 1.5.0, 2.2.0)
- **Clean audio cost in the preview** — measure on weak computers. (source: 1.4.0)
- **Cost of non-default orders** — such clips are built step by step; measure and tune. (source: 2.2.0)

## Priority 7 — Installation and platforms
Why: unsigned installers and platform gaps add friction.
- **Drag and drop on Linux Wayland** — enable when the library reports drops. (source: 1.1.0)
- **Quit item in the macOS menu.** (source: 2.0.4)
- **Idle heartbeat for the one-window lock.** (source: 2.0.4)
- **Detection during installation** — open question. (source: 0.3.0)
- **Builds of 1.2.0 on real systems** — macOS and Windows. (source: 1.2.0)
- **Other systems** — Intel Macs, ARM Windows and Linux are not built. (source: 0.4.0)
- **Legal review of source offers** — by a lawyer (Andrea's check). (source: 0.2.0)

## Priority 8 — Interface
Why: comfort and discoverability; none blocks a task.
- **Sub-second ruler labels.** (source: 1.2.0)
- **Multi-selection lost** — report a way to trigger it. (source: 1.3.0)
- **Playlist total shows dashes until every length is known** — the list total of the player's playlist is shown once all lengths are known; a partial total or an earlier total would help. (source: 3.2.0, known limitation)
- **Effects section scrolls at 800 px** — at 800 px of window height only 11 rows fit in the Effects section, so it scrolls. (source: 3.1.0)

## Priority 9 — Video player
Why: the player only watches video; none is needed for editing.
- **Picture subtitles on 360 clips.** (source: 2.0.4, REQ-253)
- **Picture subtitles at a start or a pause.** (source: 2.0.4, REQ-253)
- **Lighter ffmpeg build for the player** — measure the installers first. (source: 1.6.0)

## Priority 10 — 360 video
Why: a niche with known, bounded gaps.
- **Faster 360 and moving-camera export** — about a third of real time; measure on 8K x 4K. (source: 1.2.6, 1.3.0)
- **GPU or SIMD path for reprojection** — if the 8K target of REQ-104 is missed. (source: 1.2.6)
- **Instant look-around** — a quick view if dragging feels slow. (source: 1.2.6)
- **Roll in the spatial sound (REQ-273).** (source: 2.0.4)
- **Four-channel 360 sound order** — assumed ambisonic channel order; check. (source: 2.0.4, D61)
- **Spatial sound pre-pass in the background.** (source: 2.0.4)
- **Spherical information of 360 export** — check in real players for MP4, MOV, MKV and WebM. (source: 1.3.0)
- **Video format presets and frame effects in 360 export** — apply them or say so. (source: 1.3.0, 2.0.4)
- **Mixed 360 and flat clips** — exported as the flat view only. (source: 1.3.0)
- **Movable order of sphere-fixed items** — let them move and the export follow. (source: 2.1.0)
- **Enlarge word and Typewriter on 360 clips** — draw on the sphere. (source: 2.1.0)
- **Stereo 3D, cube maps and 180° video** — scope decision. (source: 1.2.6)

## Priority 11 — Effects, frame and LUT
Why: each new effect is new scope with its own requirements.
- **Clean audio on quiet speech** — loss of up to about 2.5 dB; decide the default. (source: 1.4.0)
- **Keyframes for text and other effects** — the text shadow is one fixed distance and colour. (source: 1.5.0, 2.1.0)
- **More effect types and stickers or shapes.** (source: 1.1.0, 1.2.6)
- **Remove several whole-video copies at once.** (source: 2.2.0)
- **Per-line order in the Applied effects widget.** (source: 2.1.0)
- **Copy the order of effects between clips.** (source: 2.2.0)
- **Audio fades after the sound group** — confirm with users. (source: 2.2.0)
- **Colour management for LUTs.** (source: 1.6.0)
- **LUT still loading when a project closes** — cancel the read on close. (source: 2.1.0)
- **Find the LUT file in the background.** (source: 2.1.0)
- **Several audio tracks mixed into one export output** — today one audio track is chosen per clip (REQ-340). (source: 3.1.0)
- **Lossless export of a non-first audio track** — copy the chosen track without re-encoding (stream copy). (source: 3.1.0 architecture)
- **Remember the Subtitles popup audio track per clip** — the popup asks again each time today; keeping it needs stored state in the project. (source: 3.1.0 architecture, 3.1.0 accepted deviation)
- **Exact conversion of combined Scale and Move** — only if a user reports a changed old project. (source: 1.6.1)

## Priority 12 — Quality, project page and release process
Why: keeps the code and tests healthy, replaces assumed numbers with measured ones, and keeps the page and release right; low urgency.
- **Measured quality goals** — detection time, regressions and speed are assumptions. (source: 0.3.0)
- **Apply LUT cost** — measure with 33- and 65-point LUTs. (source: 1.6.0)
- **4K and 8K frame times** — preview and 360 view against REQ-104. (source: 1.3.0)
- **Headless frame-time baseline.** (source: 1.2.0)
- **Feature coverage script and one status format** — `scripts/feature-coverage.sh` ignores statuses written "(implemented; ...)", so 320 of the 338 implemented requirements were counted; accept that form and settle one status format. (source: 3.1.0, feature coverage check)
- **Faster test suite and smaller 360 pictures in tests.** (source: 0.3.0, 1.6.3)
- **Tests wait for state, not for a returned frame** — the fake ffmpeg answers instantly, so a test that unwraps a frame returned by `poll_until` can see none; the preview tests that do this (`poll_until(...).unwrap()` in `src/preview.rs`) should wait for the state they check, such as the shown key. (source: 3.1.0, flaky preview test fixed)
- **CI time** — push check (QG-06), pinned ffmpeg download, installer job, cycle check. (source: 1.6.1, 2.0.1, 2.0.2)
- **Preview copies** — a lighter preset; copies for 720p sources under Automatic. (source: 0.3.0, 0.2.0)
- **Preview cache cleanup on a worker.** (source: 2.0.4)
- **Large-file memory test.** (source: 0.4.2)
- **Stable computer identity on virtual machines.** (source: 0.3.0)
- **Interface tests with a harness** — `egui_kittest` if the helper grows. (source: 1.1.0)
- **Split the application module** — `src/app.rs` (1,962 lines at 3.1.0), `src/render.rs`, `src/ui/popups.rs` (881 lines) and `dispatch_inner`, when next changed. (source: 1.1.0, 1.6.3, 3.1.0 architecture)
- **One keyframe module for the 360 camera and the picture.** (source: 1.5.0)
- **Cycle check limits** — macro-generated cycles and `#[path]` files are not followed. (source: 2.0.1)
- **Shared core crate** — only if a third program is planned. (source: 1.6.0)
- **Quality gates QG-22 to QG-27** — confirm numbering. (source: 2.2.0)
- **Animated splash picture** — decide REQ-134 wording. (source: 1.6.0)
- **Splash summary from the changelog** — take the two summaries from the changelog. (source: 1.6.0)
- **Missing splash picture or summary** — build without the splash or not. (source: 1.6.0)
- **Splash picture size in the programs.** (source: 1.6.2)
- **Smaller project page images** — about 341 KB of GIFs. (source: 1.4.1)
- **Content-Security-Policy hashes on the project page.** (source: 2.0.4)
- **Notices after every version bump.** (source: 2.0.2)
- **Player notice file version line** — decide whether to leave it out. (source: 2.0.1)
- **Release notes of 2.2.0 and 2.3.0** — project formats 13 and 14 and the removed whole-video popup and Crop. (source: 2.2.0, 2.3.0)
- **Test memory cap as a workflow rule (8 GB).** (source: 1.6.2)
- **Package list for the player on crates.io** — only if published. (source: 1.6.2)

- **Timing test of the editor on a loaded machine** — the test of the LUT preparation in `src/app` has a 200 ms limit and can fail when the machine is busy; measure it on a loaded machine and give it a limit that does not depend on the load, or run it alone (source: 3.1.3, flaky test noted during the development).
- **Documentation check for community files** — `scripts/docs-check.sh` does not scan the community files in the root (`CONTRIBUTING.md`, `SECURITY.md`, `SUPPORT.md`, `CODE_OF_CONDUCT.md`) and the `.github` templates for forbidden terms and broken links (source: 3.1.3, known limitation of the check).
- **Stricter pull request template test** — `tests/community.rs` checks that the pull request checklist names the five local checks; it could compare the list with the commands in `CONTRIBUTING.md` and `.github/workflows/build.yml` so that a change to one of them fails the test (source: 3.1.3, community tests).
- **Measure the 3.2.0 editor and player features** — import time of a project as media through the app, real footage, picture strips on a slow computer, the 200 ms frame target of the strips, the screenshot on Windows, macOS and Linux, and the recorder noise reduction delay and effect on music. (source: 3.2.0, measurements not made)

## Priority 13 — Screen recorder
Why: the options left out of the screen recorder of 3.1.2, and its checks on real systems; each needs its own design and measurements.
- **Native Wayland recording** — the first version needs X11 or XWayland and refuses native Wayland with a message; add it when a portal or capture route is chosen. (source: 3.1.2, Linux session decision)
- **Region and window capture** — the first version records a whole screen only. (source: 3.1.2, postponed from the screen recorder request)
- **Pause and resume** — stop and start a recording without losing the file. (source: 3.1.2, postponed from the screen recorder request)
- **Hotkeys** — start and stop the recording from the keyboard while another program has focus. (source: 3.1.2, postponed from the screen recorder request)
- **Choice of sound source or separate tracks** — pick the microphone and the computer sound, or record them as separate tracks. (source: 3.1.2, postponed from the screen recorder request)
- **Level control** — a volume control or meter for each sound source. (source: 3.1.2, postponed from the screen recorder request)
- **Hardware encoders for the recorder** — use the graphics or system encoders where available, with measurements against the software encoder. (source: 3.1.2, postponed from the screen recorder plan)
- **Wait for the macOS system-audio permission** — Start does not yet wait for macOS's answer to the computer-sound permission, as it does for the microphone question. (source: 3.1.2, macOS sound check)
- **Faster Start** — the sound sources are opened twice, once by the checks and again by the recording process, so a few seconds pass between Start and the recording. (source: 3.1.2, Start checks)
- **Linux default-source check when ffmpeg marks none** — when ffmpeg's list marks no default source, the monitor check only probes the default. (source: 3.1.2, Linux sound check)
- **Measure the recorder on Windows and macOS** — picture and sound offset at the start, performance at 1080p30, 4K30 and 1080p60, start time and installer sizes. (source: 3.1.2, measurements not made)
- **Real-system checks of the recorder** — manual checks 38 to 41 in the verification list, and the launch on macOS 11 to 14.5. (source: 3.1.2, not verified on real hardware)
- **Automated capture test for Windows and macOS** — the real-recording test runs on Linux only (X11 with PulseAudio). (source: 3.1.2, test coverage)
- **Echo cancellation for the recorder** — computer sound that the microphone picks up from the speakers is recorded twice, because the recorder has no echo cancellation. (source: 3.2.0, open risk)
- **Manual Linux microphone check of the noise reduction** — the 3.2.0 noise reduction was designed from the capture gain and has not been checked on a real Linux microphone; its effect on music is not measured, and a filter on the microphone alone is the fallback if computer sound is hurt. (source: 3.2.0, verification not made)
- **Last 25 ms of recorded sound lost** — the recorder loses the last 25 ms of sound when a recording stops. (source: 3.2.0, known limitation)

## Accepted limits (explained, not planned)
- **Subtitles need AVX2, FMA and F16C on Windows and Linux** — older processors get a message. (source: 0.3.0)
- **Text items use installed fonts** — a missing font falls back to the bundled font with a note. (source: 1.3.0)
- **Paraphrased subtitles** — matching works only as well as the text follows the speech. (source: 1.2.0)
- **Moved subtitle lines shown together can overlap** — a warning only. (source: 2.0.3, D53)
- **Text size cap** — 10% of picture height unless a maximum is set (2% to 50%). (source: 2.0.3, 2.2.0)
- **Clips added after a whole-video effect** — they do not get it; a note appears. (source: 1.6.1, REQ-147)
- **Keyframes are straight unless eased.** (source: 1.5.0)
- **Frame effects re-encode even at defaults; speed clips do not merge with other speeds** — by design. (source: 1.5.0, 2.3.0)
- **Copy and Cut put the marker text on the clipboard.** (source: 2.0.4)
- **LUT preparation of a 256-point table takes about 12 s** — D63. (source: 2.2.0)
- **.3dl and .csp files up to 64 MiB.** (source: 2.1.0)
- **A missing LUT file stops export until found, or export without the LUT.** (source: 1.6.0)
- **LUT below Change aspect does not grade the fill** — a user choice. (source: 2.2.0)
- **NVIDIA driver 610 or newer** — an older driver uses the normal method. (source: 0.4.x)
- **Preview of 1280 x 720 sources under Automatic gets no copy.** (source: 0.2.0)
- **No 360 switch; a video without spherical information is flat.** (source: 1.4.0)
- **Overlap MOV drawn opaque without alpha.** (source: 2.3.0)

## Not planned
- **Remembering export choices in the project or in Settings** — the dialog keeps them for the session. Decided 2026-11-17.
- **Rewriting H.264, AAC and other codecs in Rust** — ffmpeg's encoders are tuned with SIMD. Decided 2026-11-16.
- **Slower speech engine for old processors** — dropped 2026-11-14.
- **Keeping layout and recent projects in versions before 1.1.0** — accepted 2026-11-15.
- **Starting the executables job in parallel with the checks** — rejected 2026-11-19.
- **Code signing and notarisation** — installers stay unsigned. Decided 2026-11-20.
- **Custom resolutions, frame rates, bitrates and encoder options.** Decided 2026-11-20.
- **A separate vendor pipeline for hardware export.** Decided 2026-11-20.
- **Translated interface** — English only (REQ-245, D60).
- **Opening projects from newer versions in older versions** — refused by design.
- **Binary input for 256-point LUTs and a smaller preview table** — measured; the full `.cube` table stays (D63, REQ-312).
- **Manual crop of a clip** — removed in 2.3.0 (Andrea, 2026-11-22); Change aspect frames a clip.
- **Keeping the 360 switch of 1.3.0 projects** — dropped on opening.
- **Speech model for Enhance speech** — normalisation with no model file (D61, 2026-11-21).
- **Picture strips with effects** — strips show the clip without its effects; not planned unless asked. (source: 3.2.0)
- **Fixed-copy mode for project as media** — an imported project always follows its source; not planned unless asked. (source: 3.2.0)
