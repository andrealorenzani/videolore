# Code — module architecture

How the modules of the editor (`src/`) and of videolore player (`player/src/`) depend on each
other and how data and threads flow between them. Every box below is a real module; the arrows
are real `use crate::…` dependencies (the main ones; the full list per module is in the table at
the end). Where to find a feature is in [code-index.md](code-index.md); what each module does is
in [code-modules.md](code-modules.md).

## Layers

| Layer | Modules |
|---|---|
| Programs | `src/main.rs` → `lib.rs` (`real_app`, `native_options`, `prepare_desktop`); `player/src/main.rs` → `player/src/lib.rs` |
| Interface | `ui` (and its sub-modules, among them `scripts`: the interface fonts of the subtitle scripts), `dialogs`, `splash`; the player's `ui`, `fullscreen` |
| Application | `app` (`AppState`, `Command`, `dispatch`; sub-modules `command`, `dispatch_project`, `dispatch_subtitles`, `dispatch_settings`, `dispatch_playback`, `dispatch_view` (one area of the dispatcher each), `enabled` (which commands can run), `tick` (once per frame), `effects`, `export_flow`, `frame`, `lut`, `offer` (new clip), `placing`, `recovery` (autosave in the app), `sphere`, `stops` (the keys [ and ]), `subs_create`, `subs_import`, `texts`); the player's `app`, `subs` |
| Engine (threads, processes) | `export` (+ `export::choice`), `outcheck`, `hwenc`, `hwdec` (hardware decoders of the preview), `preview`, `proxy`, `stt`, `jobs`, `ffmpeg`, `scratch` (work-folder root and start-up cleanup), `autosave` (copies of the open project in the cache, written on a worker) |
| Rendering (pure) | `render` (filter graphs and argv; sub-modules `segments`, `inputs`, `graph`, `plan`, `args`), `format`, `ass`, `camera`, `frame`, `keyframes`, `lut`, `spherical`, `mp4tracks` (switches a subtitle track of an MP4 off by default) |
| Model (pure) | `project` (+ `project::v9`), `timeline`, `music`, `subtitles`, `text`, `selection`, `history`, `media`, `srt` (SRT and WebVTT), `align`, `measure` (text measured with the font's tables), `placement` (boxes of lines and texts, least move, overlaps) |
| Configuration and notices | `settings` (uses `base`, `hwenc`, `machine`), `fonts` (uses `ass`, `base`; keeps where the installed font files are and reads a face when a text uses it), `notices` (uses `ffmpeg`, `packaging`) |
| Shared helpers: `base` | `base::io` (capped reading, atomic writing), `base::private` (private folders and files, `Work`), `base::pos` (`Pos`, `POS_MAX`, `Anchor360`, `align_h`, `align_v`), `base::names` (cache, partial and speech-model file names), `base::language` (language table), `base::testing` (test-only: `best_of`, the best of several timing samples; std only). It uses no module of videolore, so it is in no cycle |
| Leaves (no dependency on other modules) | `time`, `machine`, `packaging`, `history`, `splash` |
| Start-up (Windows and Linux) | `instance` (one window: lock, heartbeat, inbox; uses `base`), used by `desktop` and `main.rs` |

The layers group the modules by role. `base` is the bottom: every module may use it, and it uses none. Since 2.0.1 the modules form no cycle (next section).

## Module dependency graph

```mermaid
flowchart TD
  main["main.rs"] --> lib["lib.rs"]
  pmain["player main.rs"] --> plib["player lib.rs"]
  lib --> ui
  plib --> pui["player ui"] --> papp["player app"]
  pui --> ui
  papp --> preview & proxy & media & render & stt & jobs & ffmpeg

  ui --> app
  ui --> splash
  dialogs --> app

  app --> export & preview & stt & proxy & hwenc & hwdec & jobs
  app --> project & history & settings & render

  export --> render & outcheck & hwenc & base & spherical & ass & format
  outcheck --> render & hwenc
  preview --> render & export & base & jobs
  proxy --> jobs & machine & settings & base
  stt --> align & srt & jobs & base & settings
  hwenc --> ffmpeg
  hwdec --> hwenc & ffmpeg
  preview --> hwdec & hwenc
  ffmpeg --> machine
  desktop --> instance
  instance --> base
  jobs --> ffmpeg

  render --> project & timeline & music & subtitles & text
  render --> camera & frame & lut & format & ass & hwenc & proxy
  project --> frame & timeline & music & subtitles & text & camera & keyframes & lut & media & base
  timeline --> frame & keyframes & media & base
  frame --> keyframes & base
  media --> ffmpeg & base
  settings --> hwenc & machine & base
  scratch --> base
  app --> scratch & base
  ui --> base
  subtitles --> timeline & base
  text --> subtitles & ass & base & fonts & measure
  fonts --> ass & base
  placement --> project & subtitles & text & selection & ass & base
  app --> placement & align & srt
  ui --> placement & fonts
  app --> autosave
  autosave --> project & base
  export --> mp4tracks
  render --> srt
  render --> fonts
  export --> fonts
```

### Cycles

Since 2.0.1 the top-level modules form **no cycle**. Before, nine pairs used each other
(`project`/`settings`, `stt`/`media`, `stt`/`scratch`, `scratch`/`proxy`, `lut`/`project`,
`subtitles`/`timeline`, `subtitles`/`text`, `ass`/`text` and `app`/`ui`), and 16 modules formed
one strongly connected group. The cycles were removed by moving the shared helpers into the
`base` module, which uses no other module of videolore:

| Pair (2.0.0) | Now |
|---|---|
| `project` ↔ `settings` | both use `base::io` (`read_limited`, `read_capped`, `write_atomic`); `project::write_atomic` is a wrapper that maps the error |
| `stt` ↔ `media` | `base::language` (`language_name`, `iso639_2_to_1`) |
| `stt` ↔ `scratch` | `base::private` (`Work`, `private_dir`, `create_private_file`, `publish`), `base::names` (`SPEECH_MODEL_FILES`, `is_partial_name`) |
| `scratch` ↔ `proxy` | `base::names::is_cache_file`, `base::private` |
| `lut` ↔ `project` | `lut` reads with `base::io::read_limited` |
| `subtitles` ↔ `timeline` | `base::pos::{Pos, POS_MAX}` |
| `subtitles` ↔ `text` | `base::pos::Anchor360` |
| `ass` ↔ `text` | `AssEvent`, `font_px` and `BASE_HEIGHT` live in `ass`; `text` uses `ass` |
| `app` ↔ `ui` | `app::sphere` uses `base::pos::{align_h, align_v}` (was `ui::boxes`); `ui` uses `app`, never the reverse |

Nothing is re-exported from the old modules: each item has one path. `export` runs `outcheck`,
which reuses only `render` and `hwenc`; `jobs::Limited` wraps any `ffmpeg::Runner`, and `ffmpeg`
does not know about the scheduler. `tests/module_cycles.rs` reads every file of `src/` with
`syn` (comments, `#[cfg(test)]` items and test files skipped) and fails `cargo test` on a cycle
between top-level modules, on any `crate::` path inside `base`, and on a `#[path]` attribute.
A new module is found without a list. Not seen by it: paths built inside macros.

## Import → timeline → preview → export

```mermaid
flowchart LR
  subgraph UIT["UI thread"]
    D["dialogs / drag and drop"] --> C["Command::Import"]
    C --> AS["AppState::dispatch"]
    AS --> ED["AppState::edit<br/>(copy of Edit, History)"]
    T["AppState::tick"]
  end
  AS -->|"ProbeJob"| PP["probe Pool<br/>media::probe (ffprobe)"]
  PP -->|"ProbeResult"| T
  T --> MP["AppState::maybe_proxy"]
  MP -->|"ProxyJob"| PX["proxy Pool<br/>ProxyWorker::generate"]
  PX -->|"ProxyEvent"| T
  ED --> TL["timeline / music / subtitles / texts"]
  TL --> PV["Preview::request_still / play<br/>render::frame_key, still_args,<br/>preview_video_args"]
  PV --> FF1["ffmpeg (still or playback)"]
  FF1 -->|"Frame (RGBA), PCM"| UI2["ui picture + AudioSink"]
  AS --> EF["app::export_flow::start_export_with"]
  EF --> EJ["ExportJob → export::start"]
  EJ --> PL["render::plan_export_choice → export_args"]
  PL --> FF2["ffmpeg → partial file"]
  FF2 --> OC["outcheck (hardware only)"]
  OC --> PUB["base::private::publish (rename)"]
  FF2 -->|"ExportEvent"| T
```

The same project value feeds the preview and the export: `render` builds both command lines
from it (one graph builder, `render::build_graph` / `compose_frame`), so what is seen is what is
exported. Project files go through `project::save` (atomic) and `project::load` (capped,
validated; `project::v9` turns a file of format 8 or older into format 9 in memory; the writer makes
format 11, which 2.0.3 and older refuse). A copy of the open project is written to the cache by
`autosave` every two minutes while it has unsaved changes (a clone, on a worker thread).

## Threads and channels

```mermaid
flowchart TD
  UI["UI thread (eframe)<br/>ui::frame_ui → AppState::dispatch / tick"]
  CHK["ffmpeg check thread<br/>ffmpeg::check"] -->|"mpsc: Result FfmpegInfo"| UI
  HWP["hardware check thread<br/>hwenc::probe_all, then hwdec::probe_all"] -->|"mpsc: HwStatus, hwdec::Working"| UI
  INB["inbox (files in the cache)<br/>instance::poll_current, every 250 ms in tick"] -->|"project path"| UI
  PRB["probe Pool (MAX_CAP workers)<br/>media::probe"] -->|"mpsc: ProbeResult"| UI
  PRX["proxy Pool (1 worker)<br/>ProxyWorker::generate"] -->|"mpsc: ProxyEvent"| UI
  EXP["export worker<br/>export::run"] -->|"mpsc: ExportEvent"| UI
  EXP --> SUP["stdout reader + reap threads<br/>(export::supervise, reap)"]
  STL["still thread<br/>preview::still_thread (mailbox: latest only; kept blurred background)"] -->|"mpsc: Frame"| UI
  PLY["playback thread<br/>preview::start_playback"] -->|"sync_channel FRAME_BUFFER: Frame"| UI
  PLY --> AUD["audio reader thread"] --> BUF["AudioBuffer (Arc)"] --> SINK["cpal callback (AudioSink)"]
  STT["speech thread<br/>stt::start"] -->|"mpsc: SttEvent"| UI
  MOD["model install thread<br/>stt::install_model"] -->|"mpsc: ModelEvent"| UI
  TRK["track reader thread<br/>app::subs_create"] -->|"mpsc"| UI
  FNT["font scan thread<br/>fonts::Fonts::start_with"] -->|"Arc Mutex list"| UI
  AUT["autosave worker<br/>autosave::start (a clone of the project)"] -->|"mpsc: Result"| UI
  SCH["jobs::Scheduler<br/>(cap 3–6 ffmpeg processes, priorities, preemption)"]
  PRB & PRX & EXP & STL & PLY & STT & HWP -.->|"Limited runner: a slot per process"| SCH
```

- Nothing slow runs on the UI thread: `AppState::tick` drains every channel once per frame.
- Cancellation is an `Arc<AtomicBool>` per job; `Limited::cancellable` kills the job's processes.
- A job of an old project is dropped by replacing its channel (New/Open); late results go nowhere.
- Every child process is started by a `ffmpeg::Runner` (`SystemRunner` in the programs, fakes
  in the tests) with an argument list, never a shell.

## videolore player

```mermaid
flowchart LR
  PUI["player ui::PlayerApp<br/>frame_ui"] --> PST["player app::PlayerState<br/>(Command, tick)"]
  PUI --> FS["fullscreen"]
  PST --> SUBS["player subs<br/>(tracks, .srt)"]
  PST --> preview2["preview::Preview"]
  PST --> media2["media::probe"]
  PST --> proxy2["proxy::ProxyWorker"]
  PST --> camera2["camera (360 view)"]
  SUBS --> srt2["srt"]
  PUI --> splash2["ui::splash_window (shared)"]
```

The player builds its own `Project` of one clip in memory (never saved) and reuses the editor's
engine; it is compiled without the `stt` feature, so it has no speech engine and no network code
(checked in CI with `cargo tree`).

## Dependencies of each module

`grep -oE "crate::[a-z_0-9]+"` over the code of each module, comments and tests excluded (`base` uses none; modules with no entry use none):

| Module | Uses |
|---|---|
| `align` | media, srt, subtitles, time |
| `app` | align, ass, autosave, base, camera, desktop, export, ffmpeg, fonts, format, frame, history, hwdec, hwenc, jobs, keyframes, lut, machine, media, music, outcheck, placement, preview, project, proxy, render, scratch, selection, settings, srt, stt, subtitles, text, time, timeline |
| `ass` | base, subtitles, time |
| `autosave` | base, project |
| `camera` | base, media, time |
| `dialogs` | app, project |
| `export` | ass, base, ffmpeg, fonts, format, hwenc, media, mp4tracks, outcheck, project, render, spherical, time |
| `ffmpeg` | machine, time |
| `fonts` | ass, base |
| `format` | hwenc, media, time |
| `frame` | base, keyframes, time |
| `hwdec` | ffmpeg, hwenc |
| `hwenc` | ffmpeg, time |
| `instance` | base |
| `jobs` | ffmpeg |
| `keyframes` | time |
| `lut` | base, media |
| `measure` | (none; uses the `skrifa` and `harfrust` crates) |
| `media` | base, ffmpeg, time |
| `mp4tracks` | (none) |
| `music` | media, time, timeline |
| `notices` | ffmpeg, packaging |
| `desktop` | instance |
| `outcheck` | ffmpeg, hwenc, render, time |
| `placement` | ass, base, measure, project, selection, subtitles, text, time |
| `preview` | base, export, ffmpeg, hwdec, hwenc, jobs, project, render, time |
| `project` | base, camera, frame, keyframes, lut, media, music, subtitles, text, time, timeline |
| `proxy` | base, ffmpeg, jobs, machine, media, settings, time |
| `render` | ass, base, camera, ffmpeg, fonts, format, frame, hwenc, lut, media, music, project, proxy, srt, subtitles, text, time, timeline |
| `scratch` | base |
| `selection` | subtitles, text |
| `settings` | base, hwenc, machine |
| `spherical` | base |
| `srt` | base, media, subtitles, time |
| `stt` | align, base, ffmpeg, jobs, media, settings, srt, subtitles, time |
| `subtitles` | base, media, time, timeline |
| `text` | ass, base, fonts, measure, subtitles, time |
| `timeline` | base, frame, keyframes, media, time |
| `ui` | app, ass, base, camera, export, ffmpeg, fonts, format, frame, hwenc, keyframes, lut, media, music, notices, placement, project, selection, settings, splash, stt, subtitles, text, time, timeline |

## 2.0.4 additions (batches 12 to 15)

```mermaid
flowchart LR
  ui_place["ui/place, theme, strings, view_menu"] --> settings_i["settings/interface"]
  ui_place --> app
  app --> dispatch_i["app/dispatch_interface"] --> settings_i
  app --> siblings
  media --> spherical
  camera --> keyframes
  frame --> keyframes
  project --> lut
  render --> camera & lut & music
  player_app["player app"] --> pset["player settings"] & plist["player playlist"] & siblings
```

New edges, all downwards (checked by `tests/module_cycles.rs`): `media` reads the initial view of a file with `spherical::read_pose`; the start camera function lives in `camera` (not in `media`, which would make a cycle); `keyframes` owns `Ease`, shared by `camera` and `frame`; `project` embeds LUT copies through `lut`; the player uses `videolore::siblings` to start the editor.

