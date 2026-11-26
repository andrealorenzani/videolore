# Architecture: components and code structure

Part of [Architecture](architecture.md). The module graph, threads, channels and the import, preview, export and command flows: [code-architecture](code-architecture.md). Where a feature lives: [code-index](code-index.md).

## Layers and rules
- **Programs:** `src/main.rs` → `lib.rs` (`real_app`, `native_options`, `prepare_desktop`); `player/src/main.rs` → `player/src/lib.rs`. 3.1.2: `screen-spy/src/main.rs` → `lib.rs` (the screen recorder, its modules in [screen recorder](architecture-screen-spy.md#components-screen-spysrc)).
- **Interface (`ui`):** windows, panels, popups, Settings; kept thin; one action table (`ui::actions`) shared by menus, toolbar and shortcuts. Splash: [UI](architecture-ui.md#splash-160-162-204).
- **Application (`app`):** `AppState`, `Command`, the dispatch areas, behaviour files; wires the UI to the workers.
- **Engine (threads, processes):** `export`, `outcheck`, `hwenc`, `hwdec`, `preview`, `proxy`, `stt`, `jobs`, `ffmpeg`, `scratch`, `autosave`, `instance`, `desktop`, `siblings`.
- **Rendering (pure):** `render` (segments, inputs, graph, plan, args, overlay, steps), `spatial`, `format`, `ass`, `word_styles`, `camera`, `frame`, `keyframes`, `lut`, `spherical`, `mp4tracks`, `picture`.
- **Model (pure):** `project` (+ `v9`, `v13`, `order`), `timeline`, `overlap`, `effect_order`, `order_chain`, `music`, `subtitles`, `text`, `selection`, `history`, `media`, `srt`, `align`, `speech_regions`, `measure`, `placement`.
- **Configuration and notices:** `settings` (+ `interface`), `fonts`, `notices`, `packaging`, `splash`, `machine`. **Shared helpers:** `base` (no other module) and `time`.
- **Rules:** the top-level modules form **no cycle** (since 2.0.1). Pure modules import no `render`, `app` or `ui`. `tests/module_cycles.rs` parses `src/` with `syn` (development dependency) and fails on a cycle, on a `crate::` path in `base`, or on `#[path]`; a new module is found without a list; a cycle made through a macro is not seen (accepted). Re-exports only from the `render`, `text` and `app` hubs.

## Module roles
- `time`: exact time arithmetic and timestamps (no floating point in cut maths).
- `media`: facts of a file from ffprobe (analysing, ready, rejected; video or audio-only; text tracks 1.2.0; 360 flag 1.2.6; overlay-file facts; audio tracks 3.1.0 planned, ADR-245).
- `timeline`: clips, in and out points, order, cuts, fades (edge fades as clip flags, 1.6.1), volume, clean audio, speed (2.0.4), overlaps (`clip_overlaps`), order rows (`clip_order`), per-clip aspects; the validity rules.
- `export::choice` (1.4.0): pure model of the Export dialog (mode, formats, 360 choice, name and target rules, reasons, mapping to the plan).
- `music`: the music track; clips never overlap; place, move, trim, split, delete.
- `subtitles`: lines stored against the source speech, their timeline appearance, position (1.2.0), `StyleSettings` (2.1.0). `srt` (1.2.0): SRT reader and writer. `align` (1.2.0): matches track words to the speech (2.0.3 sound correction). `speech_regions` (2.2.0): the REQ-177 speech-region method (not frozen).
- `camera` (1.2.6): 360 camera maths (limits, POV path, drag and zoom, the `v360` command file with absolute commands and changed values); sphere projection (1.3.0); start view (2.0.4).
- `ass`: the ASS subtitle file for each style; shadows (2.1.0); `word_centres` (2.3.0). `word_styles` (2.1.0): pure word plans; does not import `ass`. `word_layout` (3.1.0 planned, ADR-248): pure positions of the words of a line for Enlarge word from its base; imports nothing (widths are given to it).
- `history`: undo and redo of the whole edit; each step named (1.1.0).
- `project`: versioned JSON, formats 1 to 15 (writer 15 from 3.1.0; planned), `v9` and `v13` conversions in memory, `order`; unsaved-changes tracking. [projects](architecture-projects.md).
- `overlap` (2.3.0): pure overlap model (type, clamp, cut, split, removal of an edit, speed rescale).
- `effect_order`, `order_chain` (2.1.0, 2.2.0): movable rows and their order; the sound group. [effects](architecture-effects.md).
- `format`: output formats, codecs, encoders with fallbacks.
- `render`: the export plan, the ffmpeg arguments and the preview graphs; submodules `args`, `graph`, `inputs`, `plan`, `segments`, `overlay` (2.1.0), `steps` (2.2.0); golden argv tests guard the default output.
- `ffmpeg`: finds ffmpeg and ffprobe (bundled first), checks features, runs them through a `Runner`, reads progress, cancels.
- `jobs`: the job queue (process cap 3 to 6 by level, priorities, preemption); every process goes through it.
- `export`: runs an export in the background and writes it safely; hardware first when asked, normal method on any problem (0.4.0); `prepare_spatial` (2.0.4). `hwenc` (0.4.0): hardware candidates and the background check. `outcheck` (0.4.0): checks a hardware export. `hwdec` (2.0.4): decode probe for the preview.
- `preview`: still and playback frames and audio; `still_thread` keeps the latest request; the preview copy cache. `proxy`: preview copies of large sources and their cache.
- `stt`: speech-to-text: extraction (a clip's part or a track), pieces, matching jobs, language detection, the three levels, download and storage.
- `machine`: cores, memory, operating system, processor features. `settings` (0.3.0): settings file, first start, recommended settings, level translation; export choice and failures (0.4.0); layout and recent projects (1.1.0); `interface` (2.x).
- `scratch` (0.3.0): private work folders and crash clean-up; the work-root default.
- `base` (2.0.1): shared helpers (below). `notices`: third-party notices in About. `desktop`: Linux menu entry and file association; macOS open-document events and the menu call (2.0.4).
- `instance` (2.0.4): one window (lock file, heartbeat, inbox). `siblings` (2.0.4): starts the sibling program by fixed name. `autosave` (2.0.4): policy (pure), worker, recovery offer.
- `text` (1.3.0): text items, their ASS events (fades, blink, slide, cuts), caps, font-name rule; readability, box height, best fit (2.0.3); picture items (2.0.4); split into `timing`, `layout`, `events` (2.0.4).
- `fonts` (1.3.0): installed fonts (background scan with `fontdb`), family list, name check, font bytes on demand (2.0.3).
- `measure` (2.0.3): glyph widths, line height, word wrap, best-fit size. `placement` (2.0.3): box rules (least move, centre kept, warnings only); `padded_line_rect` (2.0.4).
- `picture` (2.0.4, 2.3.0): PNG size from the header, limits, fit in the box; overlay reads and the 16 MP check.
- `spatial` (2.0.4): ambisonic rotation, the stereo cardioid pair, the WAV writer, the pre-pass runner.
- `selection` (1.3.0): the one selection of lines and text items and its pure transitions.
- `spherical` (1.3.0, 2.0.4): the Spherical Video V1 marker for MP4 and MOV; in place for MKV and WebM.
- `keyframes` (1.5.0): pure keyframe lists for Scale, Move and volume keys (2.0.4). `frame` (1.5.0): Change aspect shape, fit, fill, frame size.
- `lut` (1.6.0, 2.1.0): `lut/mod.rs` (API and limits), `parse` (streamed reader), `store` (cache, `Load`, `Prep`, Work copy).
- `splash` (1.6.0, 1.6.2): layout, text fit, summaries, bundled pictures (`EDITOR`, `PLAYER`). `ui::splash_window` (1.6.2): the window for either program. `ui::icons` (1.6.0): icon font and constants.
- `player/` (1.6.0): `videolore-player`: state and window on the engine without speech; its settings and playlist (2.0.4).
- `app`: application state and wiring; popups, recent projects, dropped files (1.1.0); effects, subtitle dialogs and the selection of lines (1.2.0).

## base (2.0.1)
`src/base/` holds helpers used by several modules; it imports no other module and uses only the standard library and existing crates.

| File | Contents | Used by |
|---|---|---|
| `io.rs` | capped reading of a regular file (folders, pipes, devices refused; non-blocking open), capped streams, atomic writing (temp file, fsync, rename) | projects, settings, LUT files, track files, the work registry |
| `private.rs` | private folders (0700), private files (0600), `Work` (job folder with a lock, removed on drop), publishing (0644), lock and registry names | export, previews, speech, 360 marker |
| `pos.rs` | `Pos`, `POS_MAX`, `Anchor360`, alignment helpers | subtitles, texts, timeline, camera, frame, interface, app |
| `names.rs` | cache file rule (`<32 hex>.mkv`, `.partial`), partial rule, the three speech-model names | proxy cache, start-up clean-up, speech |
| `language.rs` | language codes, speech-model codes, English names | speech, media |
| `shadow.rs` (2.1.0) | `Shadow { px, colour }`, 0 to 20 px at 1080 | text, subtitles, ass |
| `font.rs`, `testing.rs` | font helpers (2.0.x); test helpers | fonts, tests |

## Structure history
- **2.0.1:** nine module pairs used each other and sixteen modules formed one group. Broken by `base`: project and settings use `base::io`; stt and media use `base::language`; stt and scratch use `base::private` and `base::names`; scratch and proxy the same; lut and project use `base::io`; subtitles and timeline, subtitles and text use `base::pos`; ass and text (the ASS rules move into `ass`); app and ui use `base::pos`; camera, frame, scratch and speech no longer read small files through `project`. Nothing re-exported; each item has one path. Files, names, errors and settings unchanged: a 2.0.0 project, settings file and cache work with no migration. `app.rs` and `render.rs` stayed large (debt).
- **2.0.4 (stage 0, QG-16):** `render.rs` became `render/`; `app.rs` took its dispatch arms into `app/` by area, before any feature (ADR-168); `text.rs` became `text/`. New: `spatial`, `picture`, `autosave`, `instance`, `hwdec`, `siblings`, `ui/strings`, `ui/theme`, the player's `settings` and `playlist`; root `build.rs` (splash summaries). `render.rs` keeps 92 lines with its submodules; `app.rs` keeps the state and dispatch core (about 2,100 lines). No `ui/keyframe_timeline.rs`: the keyframe widget is in the existing UI modules (ADR-176).
- **2.1.0 to 2.3.0:** `lut/` split; `effect_order`, `word_styles`, `render/overlay`, `ui/applied_effects`, `overlap`, `project/v13`, `ui/overlap_lane`, `ui/overlap_box`. `overlap.rs` depends on no interface, application or render module.

## Testing the structure
- Tests keep their assertions; only `use` lines change when code moves. Pinned names: work-folder names (`job-` prefix, `lock`, `targets`), temporary names (`.<name>.tmp-<pid>`), speech-model names, partial suffixes, cache names.
- The cycle check (seven tests) runs in about 1.7 s (estimate 1 to 3 s in debug; measured in CI).
- Platform-specific code (Unix permission bits, Linux and macOS non-blocking open, Windows lock behaviour) moved with its bodies; no new platform code. `#![deny(unsafe_code)]` stays ([security](architecture-security.md#unsafe-code)).
