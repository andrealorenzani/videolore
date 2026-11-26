# Business — Release verification

Back to the [index](business.md).

This document holds the checks that confirm a release on real systems. The automated part runs in the test suite and in the automatic builds (quality goals QG-07 to QG-10 in [Settings, performance and safety](business-performance.md#quality-goals)). The part that needs a real computer, player or ear is a manual checklist for the maintainer. A manual check is not a blocker for a version: its result is recorded here at the next release, and a failed check becomes a roadmap item. Result of each item: pending until recorded.

## Automated (2.0.2 and later)
- The full test suite passes with the ffmpeg of the tests pinned to the bundled version (QG-07, QG-08).
- The splash summaries fit their popups (REQ-154); the paused preview shows the picture at a fade (REQ-144, REQ-155).
- The tests do not fail because the machine is busy (QG-09).
- The notices check, the cargo-about install and the installer configuration check run in the automatic builds (QG-10).

## Manual checklist (2.0.2)
1. **Player on real systems** (REQ-138 to REQ-141, REQ-150): build the player for Windows, macOS and Linux; run each installer; open an MP4; check Play/Pause, seek, volume, a 360 drag, subtitles from the video and from a `.srt` file, full screen, and the start screen. Result: pending.
2. **Older editor refuses newer projects** (REQ-029): with a real 1.5.0 build, open a project saved with 1.6.0 that uses a LUT. Expected: the newer-version message, and the file unchanged (compare its checksum). Result: pending.
3. **Clean audio and MP3 by ear** (REQ-114, REQ-121): export a clip with wind or chatter using Clean audio and listen; export Only audio as MP3 and play it. Result: pending.
4. **360 marker in a 360 player** (REQ-103): export a 360 video to MP4 and MOV and open each in a 360 player. Expected: shown as a sphere, not flat. Result: pending.
5. **Splash on both programs** (REQ-133, REQ-134, REQ-150, REQ-154): start the editor and the player with the longest summary; the text is readable and inside the popup. Result: pending.
6. **Release flow** (next release): review the consolidation: sizes, roadmap priorities, batch file, code index and diagrams, and the regenerated project page with its download links. Result: pending.
7. **Font names on three systems** (REQ-165): on Linux, macOS and Windows, five fonts from the font list are drawn as chosen. Result: pending.
8. **Cost of an installed font** (REQ-166): with a 20 MB font installed, the editor's window appears at most 0.5 s later, and the font list shows its names within 1 s. Result: pending.

Hardware export on real graphics cards stays with QG-05 (see [Settings, performance and safety](business-performance.md#quality-goals)) and item 15 below.

## Manual checks of 2.0.4 (not a blocker)
9. **Subtitles in five scripts** (REQ-187, REQ-188): Arabic and Devanagari (bundled fonts) look right on Linux, macOS and Windows; Chinese, Japanese and Korean look right with a CJK font installed, and show the REQ-187 note without one. The licence of each added font (OFL or MIT) and the installer size are checked.
10. **Separate subtitle track** (REQ-182): MP4, MOV and MKV with a separate track open in a player that can switch subtitles off.
11. **Gestures by hand** (REQ-191 to REQ-193, REQ-198): trim by dragging a clip edge, drag a subtitle marker and a fade marker with a mouse and a trackpad; the snap feels acceptable.
12. **Autosave after a crash** (REQ-189, REQ-190): on each system, kill videolore during editing and check the recovery offer at the next start.
13. **Keyframe popup in real use** (REQ-203): three users edit Scale and Move keyframes; if they struggle, the popup is reworked before the release.
14. **Faster on integrated graphics** (REQ-207, QG-05 g): on the Intel laptop, Faster and Best quality timed at 1080p on the reference project; the numbers set the integrated target.
15. **Hardware by system** (REQ-212 to REQ-219): Windows NVIDIA, Intel, AMD; Linux NVIDIA and AMD; macOS Apple Silicon. Each: AV1 and VP9 export and decoding, the 360 export with texts, the frame effects, a forced failure. Ubuntu 24.04 shows the libva message (REQ-217). Intel HEVC on a newer driver and Fedora with RPM Fusion drivers are recorded as known limitations if they fail. A two-card laptop: choosing the card (REQ-216).
16. **Mac menu bar** (REQ-232): menu bar at the top, Hide and Minimize, Cmd+Q asks about unsaved changes.
17. **Wayland drop** (REQ-233): a dropped video is imported on a Wayland session with support, or the limitation is confirmed.
18. **Linux file managers** (REQ-231): start videolore from the extracted `.tar.gz` in two file managers.
19. **One window** (REQ-230): on Windows and Linux a double-click on a project while videolore runs opens it in the running window.
20. **Installer job and toolchain** (REQ-225, REQ-228): the installer job runs on a pull request that touches packaging; the Windows toolchain hash is checked on a real runner.
21. **Licence page** (REQ-229): the Windows licence page shows the non-ASCII text correctly.
22. **File association of the player** (REQ-248): the player is offered under "Open with" for MP4 and MKV on each installer; after choosing it as default, a double-click opens the player.
23. **360 in players** (REQ-261, REQ-262, REQ-258): 360 export to MKV and WebM, and the re-encoded 360 video, open as a sphere in a 360 player; automatic lines stay in place when the camera turns.
24. **Spatial sound by ear** (REQ-273): a turned camera moves the sound of a first-order ambisonic clip to the front in the stereo output.
25. **Clean audio by ear** (REQ-277 to REQ-279): Clean audio at 0%, 50% and 100% on speech with wind, and Enhance speech.
26. **Screen readers** (REQ-246): NVDA on Windows, VoiceOver on macOS and Orca on Linux read the toolbar, the menus and a popup.
27. **Translated texts** (REQ-245): not applicable (English only, decided by Andrea); nothing to check.
28. **Player builds on real systems** (see item 1) and the moving-camera export on 9.0.2 (see the roadmap).
29. **Windows "Open with" for the player** (REQ-248): after installing the player, MP4 and MKV offer it under Open with, and the installer did not make it the default (the template `packaging/windows-player.nsi` has not been run on Windows).
30. **Window and popup places** (REQ-234, REQ-235): move and resize the window and a popup on two screens, close, start again; unplug the second screen and start: the window opens visible.
31. **Clip speed by ear and eye** (REQ-281): a clip at 50%, 150% and 200% in the preview and the export: the voice keeps its pitch, subtitles stay on their words, and the timeline length matches the exported file.
32. **Overlap media on a real video** (REQ-325, REQ-327 to REQ-331): add PNG and GIF overlaps (a logo with transparency, an animation) at several places and times; they appear and disappear at their times in the preview and in the export, keep their proportions, and a 360 export leaves them out.
33. **Off-view arrows** (REQ-260): in a 360 clip, an item outside the view shows an arrow at the nearest picture edge; a click turns the view to it.
34. **Per-clip camera paths** (REQ-268): split a 360 clip, give each part another camera path, undo, redo, save and reopen: each part keeps its own path; a project saved by 2.0.3 opens with the same paths.
35. **Picture subtitles in the player** (REQ-253): a real Blu-ray rip (PGS) and a DVD rip opened in the player: the track is listed marked "picture", its lines are drawn at their times and scale with the window; a line already on screen when playback or a jump starts appears at the next change (known limitation).

## Manual checks of 3.1.0 (not a blocker)
36. **Player audio menu on a real two-track file** (REQ-338): open an MP4 and an MKV that each have two audio tracks (for example two languages) in the player. Expected: the Audio menu lists both tracks, the first one plays by default; choosing the second changes the sound at once; opening another file starts again on the first track. Result: pending.
37. **Subtitle track switch-off in a real player** (REQ-337, REQ-182): export a project with the "As subtitle track" style to MP4, MOV and MKV; open each in a player that can switch subtitles off. Expected: the picture has no lines; the track lists the project language with its lines at their times; switching it off hides them. Result: pending.

## Manual checks of 3.1.2 (not a blocker)
38. **Screen recorder on each system** (REQ-341, REQ-344, REQ-349): on Windows, on macOS 14.6 or later and on Linux (X11 or XWayland), record a 10-second video of a second screen with the default choices, and check the file plays in the player; then record once with WAV sound and check the file is an .mkv that plays. Expected: the file is complete, and the screen permission prompt (macOS) appears once. Result: pending.
39. **Playlist from Open** (REQ-355, REQ-356): in the player, Open… three videos, then Add to playlist two more while the first plays. Expected: all five play in order; the title shows the place. Result: pending.
40. **Recorder messages** (REQ-357, REQ-358): on a Mac with macOS older than 14.6, and on a native Wayland session, press Start. Expected: a message appears before recording (the macOS version needed; X11 needed and Wayland later), and no file is created. Result: pending.
41. **Sound sources and Cancel start** (REQ-349, REQ-357): on Linux with the default input set to a monitor, press Start: a message appears and nothing is recorded. On macOS, the first Start asks for the microphone and records only after the answer; check that the computer's sound is in the file (it may be silent the first time until its permission is given). Press Cancel start during the checks: no file is created and the choices unlock. Expected: the messages and the files as described. Result: pending.
