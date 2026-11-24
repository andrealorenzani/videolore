# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

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

## 3.0.0 — 2026-11-22 17:22 +0000
- Version 3.0.0 opened after release 2.3.0 "Multiple overlays and effect stack and order". New README poem. Project page regenerated for release 2.3.0.
