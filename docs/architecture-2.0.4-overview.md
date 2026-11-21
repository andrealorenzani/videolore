# Architecture — 2.0.4 development (built, not released)

Back to the [architecture index](architecture.md). This development builds the remaining batches of the [roadmap tasks](roadmap_tasks.md) as one version (2.0.4). The requirements are in the [business](business.md) documents (REQ-180 to REQ-299). The quality, security and verification parts are in [architecture-2.0.4-quality.md](architecture-2.0.4-quality.md). The module architecture as built is in [code-architecture](code-architecture.md) (written at release).

Status at the checkpoint of 2026-11-21: all 120 requirements REQ-180 to REQ-299 and the quality items QG-14 to QG-18 are built and have tests. Nothing of the plan is left to build; the business documents keep the `(planned)` marks until the Product Manager and the Documenter update them. The designs as built are in the sections below and in the sub-documents linked from them.

## Stages
Each stage was one implementation step that ended with the full test suite. The order is the roadmap order, with one addition: stage 0 splits the two largest files before features change them.

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
| 9 | 13 video player | REQ-248..256 | player settings file, playlist, open in player and editor, Windows "Open with" template, picture subtitles (REQ-253) |
| 10 | 14 360 video | REQ-261..267, 269..272 | camera roll, easing and 10 updates per second, half-size drag, speed-first export, 360 marker in MKV and WebM, initial view, keyboard camera |
| 11 | 15 effects and frame | REQ-274..280, 282..285, 288..294 | fade length and sides, music and Clean audio, crop, Enhance speech, blur levels, custom ratios, LUT strength, `.3dl` and `.csp`, LUT copies, favourites, volume keys |
| 12 | continuous quality | REQ-295..299 | splash option, preview cache limit, page image and policy |
| 13 | speed, pictures, 360 text and paths | REQ-281, 286, 287, 257..260, 268 | clip speed, clip Change aspect (drop on a clip), picture items, anchored 360 lines as text with re-anchor and off-view arrows, per-clip camera paths |
| 14 | spatial sound, picture subtitles, quality items | REQ-273, 253; QG-14..18 | spatial pre-pass, picture subtitle overlay, CI speech fixture instead of espeak-ng, pointer and UI tests, text split, one box rule, splash summary check (`build.rs`) |

Stage dependencies: 0 first; 1 before 2 and 10 (the project format); 3 before 10; 4 before 5; 5 before 6; 7 before 8 and 9; 6 and 10 before 11; 11 before 12. Stages 13 and 14 came last, after 11 and 12.

## Components
- **Interface** (`ui`): remembered window and popup positions, sections that hide, collapse and dock, workspaces, a customisable toolbar, rebindable shortcuts, a text table (`ui/strings.rs`, English only), themes (`ui/theme.rs`), text size, accessibility names, a keyframe widget for Scale and Move (in the existing UI modules, no separate file), the subtitle list with fonts for more scripts, the picture popup and the Add Picture row.
- **Application** (`app`): autosave tick and recovery offer, whole-video copies by clip, the offer for new clips, keyframe copy and paste, the one-window open path, sibling-program start in the player, re-anchor of lines and texts, the per-clip camera path copy on write, picture items (`app/texts.rs`).
- **Model** (pure): `timeline` (clip speed, `Clip::len()` = length on the timeline, `src_len()` = source length, clip Change aspect in `clip_aspects`), `music`, `keyframes`, `frame`, `history`, `camera` (roll, easing, per-clip paths, `start_camera`), `subtitles`, `srt` (VTT), `ass` (file export, word highlight), `placement` (one box rule `padded_line_rect`), `text` (split into `text/timing.rs`, `text/layout.rs`, `text/events.rs`; types and `Texts` stay in `text.rs`), `picture` (PNG header size and fit in the box), `spatial` (first-order ambisonic rotation and stereo pair, pre-pass), `project` (format 11).
- **Engine** (threads and processes): `autosave` (worker writer), `instance` (one window with a lock file and an inbox), `hwenc` (AV1 and VP9 encoders, busy or hanging classes), `hwdec` (decode probe and arguments), `outcheck` (samples only), `jobs` (check pause, Background priority), `preview` (hardware decoding, background cache, half-size camera while dragging, synchronous spatial pre-pass), `proxy` (audio kept, cache limit), `export` (subtitle mode, speed-first 360, separate track, spatial pre-pass).
- **Rendering** (pure argument builders): `render` (fades, crop, LUT strength, blur levels, Clean audio strength, speech enhancement, speed, picture overlays, picture subtitle overlay), `lut` (`.cube`, `.3dl` and `.csp` readers, copies, strength), `camera` and `spherical` (360 marker in MP4, MOV, MKV and WebM, initial view).
- **Player** (`player/`): its own settings file in the user's configuration folder, a session playlist, open-in and open-from, its own About list, picture subtitles chosen from the file's own tracks.
- **Configuration**: `settings` (program settings only, additive fields, VERSION stays 2), `fonts` (added faces, copies in the work folder), `siblings` (the other program found by its fixed name).
- **Build**: `build.rs` checks that the two splash summaries exist and are not empty (QG-17), with a message that says what to write.

## Project format 11
One bump for the whole development (D58). Projects of format 10 open unchanged; 2.0.3 and older refuse format 11 and never write it. Values added after a stage are optional with defaults, so a file written by an earlier stage opens with the same result.
- Stage 1: each media entry records its file size and modification time, so changed media is analysed again on opening.
- Stage 10: a clip can be marked as a 360 video; a point of view stores its roll; a segment can be eased. The camera paths keep the source's path, and a clip that has changed its path keeps its own entry with a `clip` field (copy on write, see [360 video](architecture-360.md#per-clip-camera-paths-204)). Entries of clips that are not in the project are dropped on opening.
- Stage 11: fade lengths and sides per clip; crop of a video clip; easing per keyframe; Clean audio strength and speech enhancement; music clip volume, mute, fades and Clean audio; volume keys (in percent, per clip); LUT strength; the blur level; a copy of each LUT file used.
- Stage 13: clip speed (stored in thousandths, 500 to 2000, written only when not 1000); a clip's own Change aspect; picture items as text items (their picture file, size and place).
- Stage 14: no format change. Picture subtitles and spatial sound are program or session values, never saved.
- The read limit rises from 16 MiB to 96 MiB in this format, because LUT copies (at most 20 MB each, 64 MiB per project) are stored in the file. Every other limit stays. Program settings (window, popups, favourites, splash option, cache limit, graphics card) never go into a project.

## Main technical decisions
The alternatives are recorded in [architecture decisions](architecture-decisions.md) (decision records of 2.0.4, ADR-168..201).

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
| Clip speed | a clip's timeline length is its source length divided by the speed; keys stay in source time; `atempo` for the sound | one clock per clip; keyframes and subtitles follow the source; a sped-up clip is never stream copied and different speeds do not merge |
| Picture items | a text item with an optional picture: one lane, box, move and undo of the text item; the picture is a looped still overlay in the shared graph | one editing path for both kinds of item; PNG header read only, ffmpeg decodes |
| Per-clip camera paths | copy on write: a clip follows its source's path until its path is first changed | a change to one clip does not move the others; no copy of the path for clips that are not changed |
| Anchored 360 lines | drawn as text items with the preset's outline or box and the word highlight (karaoke) | one drawing path with the texts; supersedes the plain white of 1.3.0 |
| Spatial sound (REQ-273) | first-order ambisonics turned in Rust, fixed cardioid pair to stereo; export by a 32-bit float WAV pre-pass in the work folder; preview pre-pass before the audio starts | no time-varying ffmpeg filter exists for this; the work folder keeps the file private |
| Picture subtitles (REQ-253) | the file's own bitmap subtitle stream overlaid in the shared render graph, only when chosen | same graph for preview and export; the golden argv of projects without the choice is unchanged |
| Text module split and one box rule | `text/` submodules; `placement::padded_line_rect` shared by the measured box and the preview hit boxes | QG-16: one place for each rule |

## Packaging and CI
- The installer configuration check is a reusable workflow (`installer-config.yml`): it runs on pull requests that touch packaging, and the executables job waits for it.
- A weekly security-advisory check runs with read-only permissions.
- The processor-too-old message is tested by name on every push, with the processor features switched off in the test.
- The Windows installer tools are pinned by version and SHA-256; the licence file carries a byte-order mark for the Windows licence page.
- The Windows player installer uses its own NSIS template (`packaging/windows-player.nsi`) with "Open with" entries only. It has not been run on Windows yet.
- Linux archive: a desktop entry, a launcher script and the executables; the README keeps its steps.
- The player gets file-type registrations for MP4, MOV, MKV, WebM, M4V and AVI (offered under "Open with"; the default is the user's own choice).
- The speech test uses the committed fixture `tests/fixtures/align/speech.mkv`; CI installs no speech synthesiser (espeak-ng is gone from CI).
- Fonts add size to every installer: the Arabic and Devanagari faces only (a few MB); CJK is not bundled.

## Dependencies
New crates and packages (checked by cargo-deny):
- `accesskit_winit` (0.34.2) and the platform adapters, through eframe's `accesskit` feature, on Windows, macOS and Linux (REQ-246).
- `objc2-app-kit` 0.3.2, macOS only, minimal features: the menu bar (REQ-232); a new direct dependency.
- `std::fs::File::try_lock` for the one-window lock (no crate).
- Nothing new for the VTT and ASS writers, the LUT readers, crop, the autosave, the MKV marker, the speed, the spatial sound, the picture subtitles or the picture items (written in-house for our own fields; the PNG size is read from the header).
- `image` stays with its `gif` feature only: the `png` feature was not added.
- Fonts: two committed open-licence faces (Arabic, Devanagari), listed in the third-party notices. CJK is not bundled: Chinese, Japanese and Korean use the system's fonts. A test font is named in both notice files.
- Not used: `muda` (the menu is built with objc2), IPC crates for the one window, font subsetting tools at build time, an ambisonic library, network model downloads for Enhance speech.
