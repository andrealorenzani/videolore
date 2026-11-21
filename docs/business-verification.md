# Business — Release verification

Back to the [index](business.md).

This document holds the checks that confirm a release on real systems. The automated part runs in the test suite and in the automatic builds (see the quality goals QG-07 to QG-10 in [Settings, performance and safety](business-performance.md#quality-goals)). The part that needs a real computer, a real player or a real ear is a manual checklist for the maintainer. A manual check is not a blocker for a version: its result is recorded here at the next release, and a failed check becomes a roadmap item.

Added with version 2.0.2 (the batch "Verify the release").

## Automated in 2.0.2
- The full test suite passes on the current code, with the ffmpeg of the tests pinned to the bundled version (QG-07, QG-08).
- The splash summaries fit their popups (REQ-154) and the paused preview shows the picture at a fade (REQ-144). The last ~20 ms of a clip showed no paused picture in 2.0.2 (defect against REQ-144); fixed in 2.0.3 (REQ-155, tested).
- The tests do not fail because the machine is busy (QG-09).
- The notices check, the cargo-about install and the installer configuration check run in the automatic builds (QG-10).

## Manual checklist (maintainer, by hand)
Result of each item: pending until the maintainer records it at the next release.

1. **Player on real systems** (REQ-138 to REQ-141, REQ-150): build the player for Windows, macOS and Linux from the release configuration; run each installer; open an MP4; check Play/Pause, seek, volume, a 360 drag, subtitles from the video and from a `.srt` file, full screen, and the start screen. Result: pending.
2. **Older editor refuses newer projects** (REQ-029): with a real 1.5.0 build (built from the 1.5.0 release), open a project saved with 1.6.0 that uses a LUT. Expected: the newer-version message, and the file is unchanged (compare its checksum before and after). Result: pending.
3. **Clean audio and MP3 by ear** (REQ-114, REQ-121): export a clip with wind or chatter using Clean audio, and listen to the speech and the noise; export Only audio as MP3 and play the file. Result: pending.
4. **360 marker in a 360 player** (REQ-103): export a 360 video to MP4 and to MOV and open each in a player that supports 360 video. Expected: shown as a sphere, not flat. Result: pending.
5. **Splash on both programs** (REQ-133, REQ-134, REQ-150, REQ-154): start the editor and the player with the longest summary of the version; check that the text is readable and inside the popup. Result: pending.
6. **Release flow** (next release): review the consolidation: the sizes after compaction, the roadmap priorities and the batch file, the code index and the diagrams, and the regenerated project page with its download links. Result: pending.
7. **Font names on three systems** (REQ-165, 2.0.3): on Linux, macOS and Windows, five fonts from the font list are drawn as chosen in the preview. Result: pending.
8. **Cost of an installed font** (REQ-166, 2.0.3): with a 20 MB font installed, the editor's window appears at most 0.5 s later than without it, and the font list shows its names within 1 s of opening. Result: pending.

Hardware export on real graphics cards is not part of this checklist; it stays with QG-05 in [Settings, performance and safety](business-performance.md#quality-goals) and with the 2.0.4 items below.

## Manual checks of 2.0.4 (not a blocker)
These parts need a real system, a real player, a real ear or a person. The automated part of each is a requirement in the 2.0.4 documents. Results are recorded here at the next release. Result of each item: pending.

9. **Subtitles in five scripts** (REQ-187, REQ-188): Arabic and Devanagari lines (bundled fonts) look right in the export on Linux, macOS and Windows; Chinese, Japanese and Korean lines look right on a system with a CJK font, and on a system without one the note of REQ-187 appears. The maintainer checks the licence of each added font (OFL or MIT) and the installer size reported by the configuration check.
10. **Separate subtitle track** (REQ-182): MP4, MOV and MKV with a separate track open in a player that can switch subtitles off.
11. **Gestures by hand** (REQ-191 to REQ-193, REQ-198): trim by dragging a clip edge, drag a subtitle marker and a fade marker with a mouse and with a trackpad; the feel of the snap is acceptable.
12. **Autosave after a crash** (REQ-189, REQ-190): on each system, kill videolore during editing and check the recovery offer at the next start.
13. **Keyframe popup in real use** (REQ-203): three users (or the maintainer on three projects) edit Scale and Move keyframes in the popup; if they struggle, the popup is reworked before the release.
14. **Faster on integrated graphics** (REQ-207, QG-05 g): on the Intel laptop, Faster and Best quality are timed at 1080p on the reference project; the numbers set the integrated target.
15. **Hardware by system** (REQ-212 to REQ-219): Windows NVIDIA, Intel and AMD; Linux NVIDIA and AMD; macOS Apple Silicon. Each: AV1 and VP9 hardware export and hardware decoding, the 360 export with texts, the frame effects, and a forced failure. Ubuntu 24.04 shows the libva message (REQ-217). Intel HEVC on a newer driver and Fedora with the RPM Fusion drivers are recorded as known limitations if they fail. A two-card laptop: choosing the card (REQ-216).
16. **Mac menu bar** (REQ-232): menu bar at the top, Hide and Minimize, Cmd+Q asks about unsaved changes.
17. **Wayland drop** (REQ-233): a dropped video is imported on a Wayland session with support, or the known limitation is confirmed.
18. **Linux file managers** (REQ-231): start videolore from the extracted `.tar.gz` in two file managers (for example Nautilus and Dolphin).
19. **One window** (REQ-230): on Windows and Linux, a double-click on a project while videolore runs opens it in the running window.
20. **Installer job and toolchain** (REQ-225, REQ-228): the installer job runs on a pull request that touches packaging; the Windows toolchain hash is checked on a real runner.
21. **Licence page** (REQ-229): the Windows licence page shows the non-ASCII text correctly.
22. **File association of the player** (REQ-248): the player is offered under "Open with" for an MP4 and an MKV on each installer; after the user chooses it as default, a double-click opens the player.
23. **360 in players** (REQ-261, REQ-262, REQ-258): 360 export to MKV and WebM, and the re-encoded 360 video, open as a sphere in a 360 player; automatic lines stay in place when the camera turns.
24. **Spatial sound by ear** (REQ-273): a turned camera moves the sound of a first-order ambisonic clip to the front in the stereo output.
25. **Clean audio by ear** (REQ-277 to REQ-279): Clean audio at 0%, 50% and 100% on speech with wind, music clean-up, and Enhance speech.
26. **Screen readers** (REQ-246): NVDA on Windows, VoiceOver on macOS and Orca on Linux read the toolbar, the menus and a popup.
27. **Translated texts** (REQ-245): not applicable in 2.0.4 (English only, decided by Andrea); nothing to check by hand.
28. **Player builds on real systems** (see item 1), and the moving-camera export on 9.0.2 (see the roadmap, Priority 1).
29. **Windows "Open with" for the player** (REQ-248): after installing videolore player, MP4/MKV offer it under Open with; check that it is listed under Open with and the installer did not make it the default (the installer template `packaging/windows-player.nsi` has not been run on Windows).
30. **Window and popup places** (REQ-234, REQ-235): move and resize the window and a popup on a two-screen set-up, close, start again; unplug the second screen and start: the window opens visible.

