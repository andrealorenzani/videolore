# Architecture — 2.0.4 development (built, not released)

Back to the [architecture index](architecture.md). This development builds the remaining batches of the [roadmap tasks](roadmap_tasks.md) as one version (2.0.4). The requirements are in the [business](business.md) documents (REQ-180 to REQ-299). The quality, security and verification parts are in [architecture-2.0.4-quality.md](architecture-2.0.4-quality.md). The module architecture as built is in [code-architecture](code-architecture.md) (written at release).

Status at the checkpoint of 2026-11-21: 108 requirements are built and tested. Not built yet: REQ-253 (picture subtitles in the player), REQ-257 to REQ-260 (360 subtitle presets, automatic anchors, re-anchor, off-view arrows), REQ-268 (per-clip camera paths), REQ-273 (spatial sound), REQ-280 (volume keys), REQ-281 (speed), REQ-286 to REQ-288 (per-clip aspect, picture items, aspect drop), and the quality items QG-14 to QG-18. Those stay planned.

## Stages
Each stage is one implementation step, ends with the full test suite, and can be stopped there. The order is the roadmap order, with one addition: stage 0 splits the two largest files before features change them.

| Stage | Batch | Requirements | Main change |
|---|---|---|---|
| 0 | preparation (no requirement) | none | `render` and `app` split by area, no behaviour change |
| 1 | 5 subtitle files and fonts | REQ-180..188 | VTT and ASS files, import, separate subtitle track, Arabic and Devanagari fonts (CJK from system fonts); project format 11 |
| 2 | 6 editing gestures and autosave | REQ-189..199 | autosave and recovery in the cache, drag trim and markers, [ and ] keys, guides and safe areas |
| 3 | 7 whole-video and keyframe clarity | REQ-200..205 | one row per clip in the whole-video popup, offer for new clips, keyframe copy and paste |
| 4 | 8 faster hardware export | REQ-206..211 | cheaper check, busy or hanging classes, check paused during jobs |
| 5 | 9 hardware reach | REQ-212..219 | AV1 and VP9 presets, hardware decoding in the preview, graphics card choice |
| 6 | 10 export and preview speed | REQ-220..224 | plain codec names, size warning, blurred background cache, job priorities |
| 7 | 11 platforms and installers | REQ-225..233 | CI path rules and advisories, pinned installer tools, one window, macOS menu, Linux archive |
| 8 | 12 interface | REQ-234..247 | remembered windows, sections, workspaces, toolbar and shortcut editors, theme, text size, accessibility |
| 9 | 13 video player | REQ-248..256 (253 not built) | player settings file, playlist, open in player and editor, Windows "Open with" template |
| 10 | 14 360 video | REQ-257..273 (257..260, 268, 273 not built) | camera roll, easing and 10 updates per second, half-size drag, speed-first export, 360 marker in MKV and WebM, initial view, keyboard camera |
| 11 | 15 effects and frame | REQ-274..294 (280, 281, 286..288 not built) | fade length and sides, music and Clean audio, crop, Enhance speech, blur levels, custom ratios, LUT strength, `.3dl` and `.csp`, LUT copies, favourites |
| 12 | continuous quality | REQ-295..299 built; QG-14..18 not built | splash option, preview cache limit, page image and policy |

Stage dependencies: 0 first; 1 before 2 and 10 (the project format); 3 before 10; 4 before 5; 5 before 6; 7 before 8 and 9; 6 and 10 before 11; 11 before 12.

## Components
- **Interface** (`ui`): remembered window and popup positions, sections that hide, collapse and dock, workspaces, a customisable toolbar, rebindable shortcuts, a text table (English only), themes, text size, accessibility names, a keyframe widget for Scale and Move, the subtitle list with fonts for more scripts.
- **Application** (`app`): autosave tick and recovery offer, whole-video copies by clip, the offer for new clips, keyframe copy and paste, the one-window open path, sibling-program start in the player.
- **Model** (pure): `timeline`, `music`, `keyframes`, `frame`, `history` (forget steps of a removed file), `camera` (roll, easing), `subtitles`, `srt` (VTT), `ass` (file export), `placement` (snap guides), `project` (format 11).
- **Engine** (threads and processes): `autosave` (worker writer), `instance` (one window with a lock file and an inbox), `hwenc` (AV1 and VP9 encoders, busy or hanging classes), `hwdec` (decode probe and arguments), `outcheck` (samples only), `jobs` (check pause, Background priority), `preview` (hardware decoding, background cache, half-size camera while dragging), `proxy` (audio kept, cache limit), `export` (subtitle mode, speed-first 360, separate track).
- **Rendering** (pure argument builders): `render` (fades, crop, LUT strength, blur levels, Clean audio strength, speech enhancement), `lut` (`.cube`, `.3dl` and `.csp` readers, copies, strength), `camera` and `spherical` (360 marker, initial view).
- **Player** (`player/`): its own settings file in the user's configuration folder, a session playlist, open-in and open-from, its own About list.
- **Configuration**: `settings` (program settings only, additive fields, VERSION stays 2), `fonts` (added faces, copies in the work folder).

## Project format 11
One bump for the whole development (D58). Projects of format 10 open unchanged; 2.0.3 and older refuse format 11 and never write it.
- Stage 1: each media entry records its file size and modification time, so changed media is analysed again on opening.
- Stage 10: a clip can be marked as a 360 video; a point of view stores its roll, and a segment can be eased. The camera paths are still kept per source (per-clip paths are not built).
- Stage 11: fade lengths and sides, crop, Clean audio strength and speech enhancement, music clip volume, mute, fades and Clean audio, easing per key, LUT strength, the blur level, copies of the LUT files used.
- The project read limit rises from 16 MiB to 96 MiB because LUT copies (up to 20 MB each, 64 MiB per project) are stored in the project. Program settings (window, popups, favourites, splash option, cache limit, graphics card) never go into a project.

## Main technical decisions
| Decision | Choice | Reason |
|---|---|---|
| Split before features | render and app split by area, no behaviour change | the two files would grow past review size |
| Format bump | one bump, in the subtitle stage | a single new format for users |
| Subtitle files | VTT and ASS from the same lines as SRT | the same text in every format |
| Separate subtitle track | SRT input, mov_text or srt, not default | burned-in stays the default |
| More script fonts | two committed open-licence faces (Arabic, Devanagari); CJK from the system's fonts | small installers; a note names a script that no installed font covers |
| Autosave | in the cache, on a worker, same format | never changes a project file |
| Gestures | one command per drag | one drag, one undo step |
| Hardware check | samples only, paused during jobs, busy or hanging | less wasted time on busy computers |
| AV1 and VP9 | presets and encoders, decoders probed | scope decided by Andrea |
| Graphics card | Automatic or a validated device name | never a raw argument |
| One window | lock file, inbox folder, no socket, no unsafe | same file-based method on both systems |
| macOS menu bar | one reviewed unsafe call (objc2 menu); no menu crate | keeps the menu without a new crate; ADR-034 amended |
| Player settings | own file in the user's configuration folder | never next to the video |
| Windows player association | "Open with" only, own NSIS template | the default stays the user's choice |
| 360 camera | roll, easing, 10 updates per second, half-size drag | speed first |
| 360 marker in MKV and WebM | written in place into the file the export made, own fields only | no second copy of the file; the padding ffmpeg leaves is reused |
| Blurred fill | an eighth of each side | 4K blur 2.9 to 3.1 s against 3.3 to 3.7 s for a quarter; the quarter named in REQ-222 is not used |
| Preview background cache | raw RGBA file for stills | ffmpeg reads files, not memory |
| Enhance speech | ffmpeg `speechnorm`, named "Normalise speech" | no model file to download or license-check |
| LUT strength | blended table: each entry mixed with the identity | any strength without an extra filter pass |
| LUT copies | raw text in the project, 96 MiB read cap | the file is kept when moved |
| Accessibility | eframe AccessKit feature | screen readers on each system |

The alternatives rejected for each decision are recorded in [architecture decisions](architecture-decisions.md). Andrea decided the open questions of the development (Q-1 to Q-7).

## Packaging and CI
- The installer configuration check is a reusable workflow (`installer-config.yml`): it runs on pull requests that touch packaging, and the executables job waits for it.
- A weekly security-advisory check runs with read-only permissions.
- The processor-too-old message is tested by name on every push, with the processor features switched off in the test.
- The Windows installer tools are pinned by version and SHA-256; the licence file carries a byte-order mark for the Windows licence page.
- The Windows player installer uses its own NSIS template (`packaging/windows-player.nsi`) with "Open with" entries only. It has not been run on Windows yet.
- Linux archive: a desktop entry, a launcher script and the executables; the README keeps its steps.
- The player gets file-type registrations for MP4, MOV, MKV, WebM, M4V and AVI (offered under "Open with"; the default is the user's own choice).
- Fonts add size to every installer: the Arabic and Devanagari faces only (a few MB); CJK is not bundled.

## Dependencies
New crates and packages (checked by cargo-deny):
- `accesskit_winit` (0.34.2) and the platform adapters, through eframe's `accesskit` feature, on Windows, macOS and Linux (REQ-246).
- `objc2-app-kit` 0.3.2, macOS only, minimal features: the menu bar (REQ-232).
- `std::fs::File::try_lock` for the one-window lock (no crate).
- Nothing new for the VTT and ASS writers, the LUT readers, crop, the autosave, or the MKV marker (written in-house for our own fields).
- Fonts: two committed open-licence faces (Arabic, Devanagari), listed in the third-party notices. CJK is not bundled: Chinese, Japanese and Korean use the system's fonts.
- Not used or not added: `muda` (the menu is built with objc2), IPC crates for the one window, font subsetting tools at build time, an ambisonic library, the `image` crate's `png` feature (picture items are not built), network model downloads for Enhance speech.
