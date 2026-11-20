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

Hardware export on real graphics cards is not part of this checklist; it stays with QG-05 in [Settings, performance and safety](business-performance.md#quality-goals).
