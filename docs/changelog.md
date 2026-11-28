# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 3.2.0 — 2026-11-28 15:22 +0000
### Added
- Business definition of the timeline ruler marks by zoom, the vertical pointer line, the picture strip on clips and the setting to disable it (REQ-367 to REQ-370); see `docs/business-timeline.md`.
- Business definition of importing a project as media, with the reload warning popup and self-import refused (REQ-371 to REQ-373); see `docs/business-editing.md`.
- Business definition of the player Playlist menu (move up/down, remove), the list total as `[hh:mm:ss]` and Take screenshot to PNG (REQ-374 to REQ-376); see `docs/business-player.md`.
- Business definition of the screen spy noise reduction (REQ-377); see `docs/business-screen-spy.md`.
- Code of REQ-367 to REQ-377: ruler marks, pointer line, picture strips and their setting, import of a project as media with the reload popup, player Playlist menu, list total, Take screenshot to PNG and the recorder noise reduction; each requirement has a test (feature coverage 355/355, line coverage 95.0%).

### Changed
- Business documents: index, key decision and out-of-scope list updated for version 3.2.0; manual checks 42 to 50 added to the verification list.
- Bundled ffmpeg for Linux and Windows is re-pinned to the BtbN build of 2026-11-27 (ffmpeg 9.0.2-982-g836df4e858); the 2026-11-13 build is no longer published, so its links returned 404.
- Architecture for 3.2.0 (decisions ADR-276 to ADR-284, see `docs/architecture-decisions.md` and `docs/architecture-timeline.md`): the project as media is rendered once to a private intermediate and rebuilt on reopen, self and cyclic imports are refused, project format 16 is saved (programs before 3.2.0 refuse it), picture strips come from one background ffmpeg job with a memory cache, the playlist total is shown once all durations are known, screenshots are saved as PNG without a new crate, and the screen spy noise reduction is an `afftdn` filter on the mixed track (the recorder noise came from a high microphone capture gain with no filter).

## 3.1.3 — 2026-11-26 21:01 +0000
### Added
- Business definition of the GitHub community standards (REQ-359 to REQ-366): code of conduct, contributing guide, security policy, support, issue forms, pull request template and community profile checks; see `docs/business-community.md`.
- Code of conduct `CODE_OF_CONDUCT.md` (REQ-359): Contributor Covenant 2.1 with its attribution; conduct reports go to community@andrealorenzani.name.
- Contributing guide `CONTRIBUTING.md` (REQ-360): how to report, how to propose a change, the local checks of the three programs (formatting, clippy, tests, coverage gate of 85%, feature coverage), the build prerequisites and the GPL-3.0 licence of contributions.
- Security policy `SECURITY.md` (REQ-361): the latest release is supported; private reporting through GitHub private vulnerability reporting; no public issue for a vulnerability.
- Support `SUPPORT.md` (REQ-362): where to ask questions and report bugs, what to include, no response time promised.
- Issue forms `.github/ISSUE_TEMPLATE/` (REQ-363): bug report and feature request forms; blank issues off; the chooser links support and private security reporting.
- Pull request template `.github/pull_request_template.md` (REQ-364): checklist of the local checks, tests, user-facing notes and the licence header.
- README `Community` section (REQ-365) links the community files.
- Tests of the community files (`tests/community.rs`, REQ-359 to REQ-366): the Covenant text, the contributing checks, the private security channel, the support links, the issue forms, the pull request checklist, the README links, the absence of internal wording and the unmodified GPL-3.0 text.
### Fixed
- Two flaky hardware timing tests of the editor no longer fail on a loaded machine.
- Build and test prerequisites of the contributing guide and README: the Linux packages now include `libxcb-randr0-dev` and a C++ compiler; `cargo build --no-default-features` builds only the editor; the ffmpeg version check applies to the editor tests and only with `VIDEOLORE_REQUIRE_FFMPEG=1`; the real X11 capture test runs only with `VIDEOLORE_REQUIRE_CAPTURE=1`; the pull request checklist names the same five checks.

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
