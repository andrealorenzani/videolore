# Roadmap

Known limitations and improvements identified so far, to consider for future versions. Grouped by area; each item says where it came from. Delivered items are removed and mentioned in the [changelog](changelog.md).

## Export
- **Make Faster really faster on integrated graphics** — on an Intel Iris Plus G7 the hardware export plus its safety check ranged from 14% quicker to 41% slower than Best quality. Make the check cheaper (render the normal picture only around the three sample frames, or decode in parallel with the export) and/or set the speed target per kind of graphics. (source: 0.4.0, QG-05 measurement)
- **Verify hardware export on every system** — only Linux with Intel graphics has been tried on real hardware; Windows (NVIDIA, Intel, AMD), Linux (NVIDIA, AMD) and macOS (Apple) still need the QG-05 checklist in [code-hardware](code-hardware.md). (source: 0.4.0, known limitation)
- **Hardware export on Ubuntu 24.04 with the included ffmpeg** — Intel/AMD hardware is skipped there because the system video library (libva 2.20) is older than the included ffmpeg needs (2.21). Options: an ffmpeg build that works with older libva, or shipping a compatible libva. (source: 0.4.0, known limitation)
- **Intel HEVC hardware** — broken on the tested Ubuntu Intel driver even at constant quality; revisit with newer drivers. (source: 0.4.0, known limitation)
- **Constant-quality mode for Quick Sync** — only VA-API has the constant-quality fallback; Quick Sync drivers that refuse bitrate control fall back to the normal method. (source: 0.4.0, known limitation)
- **Choose the graphics card** — on computers with several graphics cards NVIDIA is always preferred and Quick Sync uses the default device; let the user pick the card. (source: 0.4.0, known limitation)
- **Hardware for more formats** — AV1 and VP9 encoding, and hardware decoding, are not used; preview copies and effects also stay on the normal method. (source: 0.4.0, scope decision)
- **Skip a hardware candidate that timed out twice** — a candidate that keeps running out of time (for example HEVC VA-API on the tested Intel driver: about 14 s, then it fails) still costs 5 s and a repeat on every check (about 6.4 s idle, up to 10 s under load on that laptop). Skip it after two time-outs, in the automatic repeat or for the session. (source: 0.4.2, release verification of 0.4.1)
- **Tell a busy computer from a hanging candidate** — use the output produced before the time-out and the CPU time or load, so that only slow starts get the retry and the repeat. (source: 0.4.2, release verification of 0.4.1)
- **Limit the background work of the hardware check** — a check costs about 13–20 s per request, takes one process slot and may wake the discrete graphics card on hybrid laptops; the repeat is not paused while an export or subtitle generation runs, so pause or delay it then. (source: 0.4.2, release verification of 0.4.1 and known limitation)
- **Show the repeat in Settings → Advanced** — say "check inconclusive, repeating" while the automatic repeat runs. (source: 0.4.2, release verification of 0.4.1)
- **Under extreme load the check can still find no hardware** — after its repeat, a very busy computer can still end with "no hardware", so Faster uses the normal method. (source: 0.4.2, known limitation)

## Effects
- **More effect types** — title or text overlays and per-clip speed. Each needs its own requirements. (source: 1.1.0, postponed from the 1.1.0 request)
- **Length of each fade** — every fade lasts 0.5 s (or half a shorter clip) and cannot be changed; let the user choose the length of each fade. (source: 1.1.0, postponed from the 1.1.0 request)
- **Volume over time** — volume that changes along the timeline (automation), instead of one level per clip and one for the music. (source: 1.1.0, postponed from the 1.1.0 request)
- **Effects on music clips** — per-clip music volume and fades on music; today only the video clips take effects. (source: 1.2.0, out of scope of the 1.2.0 request)
- **One-sided boundary fades** — a fade only in or only out at a clip boundary, instead of a fade in both directions. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Start and end fades follow the clip** — the fades from and to black stay at the edges of the video when clips move; let them follow the first or last clip. (source: 1.2.0, known limitation)

## 360 video
- **Export 360 video** — keep the 360 format in the export (today a 360 clip is exported only as a flat video that follows the camera path). (source: 1.2.6, postponed from the 1.2.6 request)
- **Stereo 3D, cube maps and 180° video** — side-by-side or top-and-bottom stereo, cube-map layouts, partial 180° pictures and cropped equirectangular pictures are not recognised; only one full equirectangular picture is supported, and cropped ones are shown as flat video. (source: 1.2.6, postponed from the 1.2.6 request)
- **Separate points of view per clip** — two copies of the same part of one source share their points of view; give each clip its own camera path. (source: 1.2.6, known limitation)
- **Instant look-around** — a camera still is estimated at 150–700 ms (targets 400 ms for 4K and 700 ms for 8K); if that feels slow while dragging, add a quick view computed inside the program, while the final picture still comes from `ffmpeg`. (source: 1.2.6, measurement to do)
- **Starting direction of 360 files** — the initial yaw, pitch and roll written in a file are ignored, so "front" is always the middle of the picture. (source: 1.2.6, known limitation)
- **Camera roll** — the 360 camera cannot tilt sideways; add roll to the camera path and to the POVs. (source: 1.2.6, postponed from the 1.2.6 request)
- **Easing between points of view** — the camera moves at a constant speed in a straight line between POVs; add easing curves or other speeds. (source: 1.2.6, postponed from the 1.2.6 request)
- **Keyboard control of the 360 camera** — turn, zoom and set POVs from the keyboard; today only the mouse and the wheel work. (source: 1.2.6, postponed from the 1.2.6 request)
- **Spatial sound that follows the camera** — rotate ambisonic audio with the camera; today the spatial sound is played as it is. (source: 1.2.6, postponed from the 1.2.6 request)
- **Other objects on the picture** — titles, stickers and overlays that can be placed and moved like subtitle lines; the boxes are built to take them. (source: 1.2.6, postponed from the 1.2.6 request)

## Subtitles
- **Meet the accuracy target of reused subtitles** — the word timing of a video's own subtitles is approximate: 82% of the words were matched and the start error averaged 0.48 s, against a target of 90% and 0.3 s. Improve the word timing (DTW timestamps, sound-onset snapping, or a larger model), then re-enable the strict test `alignment_matches_real_speech`. (source: 1.2.0, known limitation, REQ-095)
- **Import subtitle files** — import .srt and .vtt files and sidecar files next to the video. Optionally re-time the start and end of lines from the speech, and make matching faster by checking only the uncertain lines. (source: 1.2.0, out of scope of the 1.2.0 request)
- **WebVTT export** — write .vtt as well as .srt. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Switchable subtitle track in MP4 and MKV export** — keep the subtitles as a track the player can switch off, instead of only burning them in. (source: 1.2.0, out of scope of the 1.2.0 request)
- **ASS export with positions** — write the moved lines with their places; the .srt file has no positions. (source: 1.2.0, known limitation)
- **Text from picture-based tracks** — read DVD, Blu-ray (PGS) and DVB subtitle tracks with OCR; they are ignored today. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Keep moved lines inside the picture when the text grows** — a text edit after a drag can push a line past the edge in the export; keep it inside, or warn. (source: 1.2.0, known limitation)
- **Keep moved lines apart** — two lines shown at the same time at the same place overlap; keep them apart, or warn. (source: 1.2.0, known limitation)
- **Preview hit boxes for stacked lines** — lines without their own position that share a place can be hard to pick in the preview. (source: 1.2.0, known limitation)
- **Measuring font for the preview** — the font used to measure the box has no CJK glyphs, so the box widths of such text are approximate. (source: 1.2.0, known limitation)
- **Re-check the tracks of replaced files** — a file replaced on disk keeps its old subtitle-track list until it is imported again; only media with unknown tracks is checked when a project opens. (source: 1.2.0, known limitation)
- **Style per line or clip** — each line or clip with its own style; today there are three styles for the whole project. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Custom style editor** — change the colours, fonts and placement of the three styles. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Translation and more than one language per project** — one language per project today. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Other speech models** — only the three quality levels are offered. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Measure the larger speech levels** — speed and memory of Balanced and Most accurate are estimates; measure them and tune the recommendation thresholds. (source: 0.3.0, assumption)
- **Subtitle fonts for other scripts** — Chinese, Japanese, Arabic, Hindi and similar scripts use whatever fonts the system has, so they may look different per computer or show empty boxes, and the subtitle editor cannot show them. Bundle fonts for these scripts. (source: 0.2.0, known limitation)
- **Speech in pieces without lost words** — speech is recognised in pieces of about two minutes, and a word cut at a boundary can be lost; overlap the pieces or join them at word boundaries. (source: 0.3.0, known limitation)
- **Align horizontally on a line at the style's place** — the command does nothing for a line that is still at its style's place; say so, or make it useful there. (source: 1.2.6, known limitation)
- **Keep the centre when enlarged text wraps** — enlarging a line's text can make it wrap onto more lines and shift its centre slightly; keep the centre fixed, or warn. (source: 1.2.6, known limitation)

## Editing
- **Keep undo history when a file is removed** — removing a file from the media list drops the undo history when an undo step refers to it; keep the steps that do not depend on the file. (source: 0.2.0, known limitation)
- **One window for double-clicked projects** — on Windows and Linux a double-clicked project opens a second window instead of reusing the open one (macOS keeps one window). (source: 0.2.0, known limitation)
- **Drag subtitle markers to re-time lines** — move the start and end of a subtitle line by dragging its marker on the timeline; today the times are typed in the line's settings. (source: 1.1.0, postponed from the 1.1.0 request)
- **Drag fade markers** — move a fade by dragging its marker; fades sit on clip boundaries today. (source: 1.1.0, postponed from the 1.1.0 request)
- **Group actions on selected lines** — delete or shift the timing of several subtitle lines selected together; today only moving and resetting their positions works on a selection. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Snap and safe-area guides** — snap moved subtitle lines to guides, and show title-safe areas in the preview. (source: 1.2.0, out of scope of the 1.2.0 request)
- **Drag clip edges to trim** — trim a clip by dragging its edge on the timeline; today trimming uses the Clip details window. (source: 1.1.0, postponed from the 1.1.0 request)

## Interface
- **Native macOS menu bar** — the menu bar is inside the window on macOS, and the standard Hide and Minimize items are switched off so that Cmd+Q asks about unsaved changes. Move the menu to the top of the screen and restore Hide and Minimize, keeping that prompt. (source: 0.2.0, known limitation; menu inside the window since 1.1.0)
- **Hide, collapse and dock sections** — sections can be resized since 1.1.0 but not yet moved, docked, hidden or collapsed. (source: 1.1.0, out of scope of the 1.1.0 request)
- **Saved workspaces** — keep several section layouts and switch between them. (source: 1.1.0, out of scope of the 1.1.0 request)
- **Customisable and icon toolbar** — choose which buttons the toolbar shows, and show them as icons. (source: 1.1.0, out of scope of the 1.1.0 request)
- **Rebindable shortcuts** — let the user change the keyboard shortcuts, which are fixed today and listed in Help > Keyboard shortcuts. (source: 1.1.0, out of scope of the 1.1.0 request)
- **Light theme and system theme** — add a light theme and follow the system theme. (source: 1.1.0, out of scope of the 1.1.0 request)
- **Interface text size** — Ctrl/Cmd+= and Ctrl/Cmd+- zoom the timeline since 1.1.0 and no longer change the text size; add a setting for the text size. (source: 1.1.0, known limitation)
- **Translated interface** — translate the interface into other languages. (source: 1.1.0, out of scope of the 1.1.0 request)
- **Screen-reader support** — the window does not yet describe its controls to screen readers, because the accessibility support of the windowing toolkit is switched off; enable it and test it with a screen reader. (source: 1.1.0, known limitation)
- **Keyboard navigation of timeline markers** — move from marker to marker with the keyboard; markers can be reached only with the mouse today. (source: 1.1.0, known limitation)
- **Remember window size, position and popup positions** — the window size and position are not remembered at all, and popups keep their place only until videolore is closed. Remember them across sessions. (source: 1.1.0, known limitation)
- **Better default popup positions** — popups open near the centre of the window and can cover the timeline; choose positions that leave the timeline visible. (source: 1.1.0, known limitation)
- **Ctrl/Cmd+V for clips** — the shortcut pastes clips only when the system clipboard holds text, because the windowing layer sends no paste event otherwise. The menu and toolbar Paste always work. (source: 1.1.0, known limitation)
- **Drag and drop on Linux Wayland** — dropping files onto the window does not work in Wayland sessions, because the windowing library does not report dropped files there. Enable it when the library supports it. File > Import media… and File > Open… work there. (source: 1.1.0, known limitation)
- **Discover Ctrl/Cmd+click and Shift+drag** — selecting several subtitle lines with these gestures is not shown anywhere on screen; add a hint or a menu entry. (source: 1.2.0, known limitation)
- **Sub-second ruler labels** — the timeline ruler shows whole seconds only; at frame zoom show fractions of a second. (source: 1.2.0, known limitation)
- **Autosave and crash recovery** — save the project automatically and offer to recover unsaved work after a crash. (source: 1.1.0, postponed from the 1.1.0 request)

## Performance
- **Faster test suite** — tests got 25% faster in 0.3.0 against a 30% goal; most of the remaining time is the two real speech-recognition tests. (source: 0.3.0, QG-03 result)
- **Preview copies with a lighter preset** — `ultrafast` would decode a little cheaper, but every existing preview copy would be regenerated. (source: 0.3.0, performance review)
- **Regenerate cleared preview copies at once** — cleared preview copies are made again only after reopening the project or changing Preview quality. (source: 0.3.0, known limitation)
- **Preview copies for 720p sources under Automatic** — a 1280x720 source never gets a preview copy in Automatic; check whether slower computers need one. (source: 0.2.0, known limitation)
- **Limit the preview copy cache** — the cache is never cleaned automatically; "Clear preview copies" is the only way to empty it. Add an age or size limit. (source: 0.2.0, known limitation)
- **Playback and export without waiting** — while an export and playback run together, other ffmpeg work (analysis, proxies) waits. (source: 0.3.0, known limitation)
- **Headless frame-time baseline** — record the frame time of the preview and timeline without a window, to compare versions; not done in 1.2.0. (source: 1.2.0, skipped item)
- **Hardware decoding for preview and preview copies** — only the export uses the computer's video hardware today; decoding the preview and making preview copies with it could speed up large sources. (source: 1.2.6, speed review)
- **GPU or SIMD path for 360 reprojection** — if the 8K look-around target of REQ-104 is not met, move the 360 reprojection to the graphics card or to SIMD code. (source: 1.2.6, product risk)
- **Faster moving-camera export** — a 360 clip with a moving camera exports at about one third of real time; send fewer commands per second to `ffmpeg`, or compute the projection inside the program. (source: 1.2.6, known limitation)
- **Measured quality goals** — the numbers of the quality goals (detection time, regressions, speed) are assumptions to confirm with measurements. (source: 0.3.0, assumption)

## Quality
- **Check the installer configuration on every push** — install cargo-packager 0.11.8 in a Linux job, run `scripts/package.sh <target> --config-only` and build the deb and app formats from stub binaries, so packaging errors are not found only at release time. (source: 0.4.3, release of 0.4.2)
- **Split the hardware developer document** — [code-hardware](code-hardware.md) is 237 lines and mixes the check, the export, the tests and the QG-05 results; split it into smaller documents. (source: 0.4.2, documentation size)
- **Large-file memory test** — the memory acceptance test runs on a few minutes of video, not 4 GB; add a run on a large file. (source: 0.4.2, assumption in the product documentation)
- **Stable computer identity on virtual machines** — a virtual machine or container whose number of cores changes between starts is treated as a new computer; with custom settings this only shows a notice. (source: 0.3.0, known limitation)
- **Split the application module** — `src/app.rs` (about 2,900 lines) mixes the editing state with popups, recent projects, drops and the effect and subtitle commands; move them into their own submodules. (source: 1.1.0, size of the module; grown in 1.2.0)
- **Robust preview test under load** — the test `preview_delivers_rgba_frames_of_expected_size` failed once under heavy parallel load; make it independent of the load on the machine. (source: 1.1.0, test failure seen once)
- **Timing gates in interface tests** — some interface tests in `src/ui/tests.rs` compare timings with fixed limits; one run on a busy desktop measured 212 ms against the 200 ms limit of REQ-104. Make them independent of the load on the machine. (source: 1.2.6, test failure seen once)
- **Re-measure the 360 camera on each bundled ffmpeg 9.x** — the behaviour of the `v360` filter and its `reset_rot` option is measured on the bundled ffmpeg; measure it again after each ffmpeg update. (source: 1.2.6, measurement to do)
- **Gated alignment test on CI's versions** — the real-speech test of reused subtitles has been run only on the development machine; confirm it with the ffmpeg and espeak versions CI uses. (source: 1.2.0, not verified locally)
- **macOS and Windows builds of 1.2.0** — these were checked only by CI; confirm the builds and the subtitle export on real systems. (source: 1.2.0, not verified locally)
- **Interface tests with a test harness** — the interface tests use a small helper on the window's accessibility tree; move them to `egui_kittest` if the helper grows. (source: 1.1.0, planned fallback)

## Packaging
- **Run the installer job before each release** — a manual run, or on pull requests that touch `scripts/package.sh`, `packaging/` or `assets/icon/`; the 0.4.2 macOS and Windows failures were found only at release time. (source: 0.4.3, release of 0.4.2)
- **Pin the Windows installer toolchain** — the NSIS toolchain that cargo-packager downloads on Windows should get a fixed version and SHA-256 in `packaging/tools.lock`, like the other downloads. (source: 0.4.3)
- **Licence page encoding** — the NSIS licence page shows `third-party.txt`; non-ASCII text without a byte-order mark may look garbled (cosmetic, not verified). (source: 0.4.3)
- **macOS and Windows installers for 0.4.2** — the 0.4.2 GitHub release has only the Linux installers; provide the macOS and Windows installers with the next release. (source: 0.4.3, release of 0.4.2)
- **Code signing and notarisation** — installers are unsigned, so Windows and macOS warn on first launch. Postponed, not rejected. (source: 0.2.0, decision)
- **Linux file managers and the archive** — some file managers do not start the program from the extracted `.tar.gz` on double-click; ship a fix or a launcher that works in them. (source: 0.2.0, known limitation)
- **Legal review of source offers** — the way the source code of bundled components is offered has not been reviewed by a lawyer. (source: 0.2.0, assumption)
- **Detection in the installers** — the computer is set up at the first start of the app; detection could also run during installation. Open question. (source: 0.3.0, open question)
- **Old-processor check in CI** — run the tests under an emulated pre-AVX2 processor to prove the friendly message path. (source: 0.3.0, skipped item)
- **Weekly security-advisory check** — a scheduled CI run of the dependency advisory check, not only on pushes. (source: 0.3.0, skipped item)
- **Other systems** — builds exist for Windows and Linux on x86-64 and for macOS on Apple Silicon only; Intel Macs and ARM Windows or Linux are not built. (source: 0.4.0, scope decision)

## Workflow
- **Shorter internal documents** — the three long internal development documents (architecture about 145 KB, code about 61 KB, business about 51 KB) should become short indexes of the human documentation, one line per paragraph with its line range, with the detail moved to sub-documents. (source: 1.2.5, documentation size)

## Not planned
- **Rewriting H.264, AAC and other codecs in Rust** — considered and rejected on 2026-11-16: ffmpeg's x264, x265 and AAC encoders are tuned with SIMD code, a rewrite would likely be slower and less correct, and it adds licence, patent and maintenance risk. Speed comes from hardware decoding and encoding instead.
- **Slower speech engine for old processors** — dropped on 2026-11-14: processors without AVX2 get a clear message and subtitle lines can still be added by hand.
- **Merge the business and architecture planning into one step** — saves one fresh start per feature, but the notes still dominate the cost and one owner would hold two private notes files, against the ownership rule. Decided on 2026-11-15.
- **Keeping layout and recent projects in versions before 1.1.0** — versions before 1.1.0 do not know the saved layout and the recent-projects list, so they drop both when they save their settings. Accepted on 2026-11-15: downgrades are rare and the projects themselves are not affected.
