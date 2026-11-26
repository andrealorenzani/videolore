# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 3.1.2 — 2026-11-26 19:03 +0000
### Added
- **videolore-screen-spy**, a new program that records one screen to a video file, with a live preview, the screen list, an output file with Choose…, a video format, and computer sound and microphone mixed into one track (REQ-341 to REQ-352, REQ-357; see `docs/business-screen-spy.md`). It has its own installers for Windows, macOS and Linux, and the editor's installer contains no recorder.
- Screen recorder: WAV recordings are saved as MKV files; Linux needs X11 or XWayland (native Wayland is refused with a message); computer sound needs macOS 14.6 or later (REQ-358). Start checks the system before recording (permissions, screen, file, sound sources) and **Cancel start** stops the checks; on Linux a monitor used as the default input refuses Start, and on macOS the microphone question is answered before recording.
- Player **playlist**: File > Open… chooses several videos at once and File > Add to playlist adds more at any time; the title shows the position as "name (n of m)" (REQ-355, REQ-356; see `docs/business-player.md`).
- Architecture of the screen recorder: a separate workspace program with its own installers and 16 decisions (ADR-255 to ADR-270), see `docs/architecture-screen-spy.md`.
### Changed
- Player File > Open… replaces the playlist with the chosen videos (REQ-356 amends REQ-254).
### Fixed
- Editor: a file the import refused could be reported as "Import finished"; the refusal is now reported.
### Workflow
- CI, `scripts/version.sh`, coverage, feature coverage and the code lookup scripts cover the screen recorder; the clone-workflow templates were reviewed and stay generic.

## 3.1.1 — 2026-11-26 00:20 +0000
### Workflow
- `/clone-workflow` is now a skill (`.claude/skills/clone-workflow/`) with one manifest, generic templates with placeholders, `clone.sh` and `check.sh`; it is copied into cloned projects too.
- `/change-workflow` keeps the manifest and templates up to date (rule 6b).

## 3.1.0 — 2026-11-24 19:37 +0000
### Added
- Subtitle style **As subtitle track**: the export writes the lines as a separate switchable subtitle track instead of drawing them on the picture (REQ-337).
- Player **Audio** menu: chooses which audio track of the video plays, the first one by default (REQ-338).
- **Audio track** choice in the **Volume and track** effect (renamed from Volume): the clip's sound is the chosen track, in the preview and in the export; every audio track of a file is recorded when it is analysed (REQ-340).
- **Audio track** choice in the **Subtitles** effect: recognition and word timing use the chosen track; the lines keep the text of a subtitle track of the clip when there is one (REQ-339).
### Changed
- **Enlarge word** grows from the base of the word: the words before it move left and the words after it move right, so no two words overlap and the line's centre stays where it is (REQ-334, replaces the placement rule of REQ-333). Projects with Enlarge word saved before 3.1.0 look different when opened.
- **Overlap media** has its own row in the Effects list, with the look of the other effect rows (REQ-336).
- Project format 15 on every save: 3.0.x and older refuse projects saved by 3.1.0 or later; formats 14 and older open as before (ADR-244 to ADR-254).
### Fixed
- **Fit empty with blurred video** shows the blurred fill on a clip's Change aspect in the preview, as in the export (REQ-335).
### Workflow
- New `scripts/doc-replace.sh` replaces a requirement, an ADR or a literal text (library version, README badge) without reading the old text; the test loop (run, fix, rerun) now runs once, at the end of all the code of a development.
### Tests
- A preview test that checked a returned frame could fail intermittently; it now waits for the shown frame's key, as the still-rendering rule requires (REQ-144).

## 3.0.0 — 2026-11-22 17:22 +0000
- Version 3.0.0 opened after release 2.3.0 "Multiple overlays and effect stack and order". New README poem. Project page regenerated for release 2.3.0.
