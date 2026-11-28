# Code — module architecture

Back to the [index](code.md). How the modules of the editor (`src/`), of videolore player (`player/src/`) and of the screen recorder (`screen-spy/src/`) depend on each other and how data and threads flow between them. Every box is a real module; the arrows are real `use crate::…` dependencies (checked with `scripts/module-deps.sh`, which prints the full list, the users of a module with `-r`, or the whole graph with `--mermaid`). Where a feature is: [code-index.md](code-index.md); the patterns: [code-modules.md](code-modules.md).

## Layers
| Layer | Modules |
|---|---|
| Programs | `src/main.rs` → `lib.rs` (`real_app`, `native_options`, `prepare_desktop`); `player/src/main.rs` → `player/src/lib.rs` |
| Interface | `ui` (sub-modules per area: `actions`, `menu`, `toolbar`, `keymap`, `effects`, `popups`, `frame_popups`, `text_popup`, `lut_popup`, `applied_effects`, `timeline`, `overlap_lane`, `overlap_box`, `picture`, `boxes`, `placement`, `layout`, `place`, `theme`, `strings`, `splash_window`…), `dialogs`; the player's `ui`, `fullscreen` |
| Application | `app`: `AppState`, `Command` (`command`), the effect table (`effect_kind`), the dispatcher and its areas (`dispatch_project`, `dispatch_subtitles`, `dispatch_settings`, `dispatch_interface`, `dispatch_playback`, `dispatch_view`, `dispatch_effects`, `dispatch_objects`), `enabled`, `tick`, and the behaviour files (`effects`, `frame`, `lut`, `overlap`, `applied`, `texts`, `placing`, `sphere`, `stops`, `subs_create`, `subs_import`, `export_flow`, `recovery`); the player's `app`, `subs` |
| Engine (threads, processes) | `export` (+ `choice`), `outcheck`, `hwenc`, `hwdec`, `preview`, `proxy`, `stt`, `jobs`, `ffmpeg`, `scratch`, `autosave`, `instance` (one window), `desktop`, `siblings` |
| Rendering (pure) | `render` (`segments`, `inputs`, `graph`, `plan`, `args`, `overlay`, `steps`), `spatial`, `format`, `ass`, `word_styles`, `word_layout`, `camera`, `frame`, `keyframes`, `lut` (`parse`, `store`), `spherical`, `mp4tracks`, `picture` |
| Model (pure) | `project` (+ `v9`, `v13`, `order`), `timeline`, `overlap`, `effect_order`, `order_chain`, `music`, `subtitles`, `text` (+ `timing`, `layout`, `events`), `selection`, `history`, `media`, `srt`, `align`, `speech_regions`, `measure`, `placement` |
| Configuration and notices | `settings` (+ `interface`), `fonts`, `notices`, `packaging`, `splash`, `machine` |
| Shared helpers | `base` (`io`, `private`, `pos`, `names`, `language`, `shadow`, `font`, `testing`): uses no other module, so it is in no cycle; `time` |

The top-level modules form **no cycle** (since 2.0.1; the shared helpers moved to `base`). `tests/module_cycles.rs` parses `src/` with `syn` and fails on a cycle, on a `crate::` path in `base` and on `#[path]`; a new module is found without a list. Each item has one path: re-exports are only the `render`, `text` and `app` hubs over their private files.

## Module dependency graph
The main edges (leaves `base`, `time` and the uses of `ffmpeg` by every process starter are left out for readability):

```mermaid
flowchart TD
  main["main.rs"] --> lib["lib.rs"] --> ui
  pmain["player main.rs"] --> plib["player lib.rs"] --> pui["player ui"] --> papp["player app"]
  pui --> ui
  papp --> preview & proxy & media & render & project & stt & jobs & camera & srt & subtitles
  dialogs --> app
  ui --> app & placement & fonts & notices & splash & export & project & stt
  app --> export & preview & stt & proxy & hwenc & hwdec & jobs & outcheck
  app --> project & history & settings & render & placement & align & srt
  app --> overlap & effect_order & order_chain & picture & autosave & scratch & desktop & siblings
  export --> render & outcheck & hwenc & spherical & mp4tracks & spatial & fonts & ass & format & project
  outcheck --> render & hwenc
  preview --> render & export & hwdec & hwenc & jobs & spatial & lut & project
  app --> nested & strips
  ui --> strips
  nested --> export & project & media & jobs & timeline & lut & picture
  strips --> jobs & ffmpeg & timeline
  proxy --> jobs & settings & media & machine
  stt --> align & srt & jobs & settings & speech_regions & subtitles & media
  hwdec --> hwenc
  jobs --> ffmpeg
  ffmpeg --> machine
  desktop --> instance
  render --> project & timeline & music & subtitles & text & overlap & effect_order & picture
  render --> camera & frame & keyframes & lut & format & ass & hwenc & proxy & spatial & srt & fonts
  project --> timeline & music & subtitles & text & overlap & effect_order & order_chain & camera & frame & keyframes & lut & media
  timeline --> frame & keyframes & media & overlap & effect_order
  order_chain --> effect_order
  overlap --> picture
  text --> subtitles & ass & fonts & measure & picture
  ass --> measure & word_styles & word_layout & subtitles
  fonts --> ass
  placement --> project & subtitles & text & selection & ass
  subtitles --> timeline & media
  autosave --> project
  spatial --> camera & media
  media --> spherical
  settings --> hwenc & machine
  splash --> settings
```

## Import → timeline → preview → export
```mermaid
flowchart LR
  subgraph UIT["UI thread"]
    D["dialogs / drag and drop"] --> C["Command::PickImport / DropFiles"]
    C --> AS["AppState::dispatch<br/>(DISPATCH_AREAS)"]
    AS --> ED["AppState::edit<br/>(copy of Edit, History,<br/>overlap removal check)"]
    T["AppState::tick"]
  end
  AS -->|"ProbeJob"| PP["probe Pool<br/>media::probe (ffprobe)"]
  PP -->|"ProbeResult"| T
  T --> MP["AppState::maybe_proxy"]
  MP -->|"ProxyJob"| PX["proxy Pool<br/>ProxyWorker::generate"]
  PX -->|"ProxyEvent"| T
  ED --> TL["timeline (clips, overlaps, orders)<br/>music / subtitles / texts"]
  TL --> SEG["render::segments<br/>+ overlay::attach / steps"]
  SEG --> PV["Preview::request_still / play<br/>render::frame_key, still_args,<br/>preview_video_args"]
  PV --> FF1["ffmpeg (still or playback)"]
  FF1 -->|"Frame (RGBA), PCM"| UI2["ui picture + AudioSink"]
  AS --> EF["app::export_flow::start_export_with"]
  EF --> EJ["ExportJob → export::start"]
  EJ --> PL["render::plan_export_choice → export_args"]
  SEG --> PL
  PL --> FF2["ffmpeg → partial file"]
  FF2 --> OC["outcheck (hardware only)"]
  OC --> PUB["base::private::publish (rename)"]
  FF2 -->|"ExportEvent"| T
```

The same project value feeds the preview and the export: `render` builds both command lines from it (one graph builder: `render::build_graph`, `compose_frame`, the overlay stage and the step builder), so what is seen is what is exported. Project files go through `project::save` (atomic) and `project::load` (capped, validated; `project::v9` and `project::v13` convert older formats in memory; the writer makes format 16). `autosave` writes a clone of an unsaved project to the cache every two minutes.

## A command, from the click to the undo step
```mermaid
flowchart LR
  W["widget / menu / shortcut<br/>(ui::actions::ACTIONS)"] -->|"Command"| D["AppState::dispatch"]
  D --> E{"is_enabled?"}
  E -->|"no"| R["disabled_reason → status line"]
  E -->|"yes"| A["dispatch_inner: each DispatchArea<br/>returns Done or Pass"]
  A --> B["behaviour file (app/effects.rs, app/frame.rs…)"]
  B --> ED["AppState::edit(label, closure)"]
  ED -->|"Err REMOVAL_ASK"| M["Modal::RemoveOverlaps + pending_removal"]
  ED -->|"Ok"| H["History push (undo name = Command::undo_name / EffectKind::undo_name)"]
```

## Threads and channels
```mermaid
flowchart TD
  UI["UI thread (eframe)<br/>ui::frame_ui → AppState::dispatch / tick"]
  CHK["ffmpeg check thread<br/>ffmpeg::check"] -->|"mpsc: Result FfmpegInfo"| UI
  HWP["hardware check thread<br/>hwenc::probe_all, then hwdec::probe_all"] -->|"mpsc: HwStatus, hwdec::Working"| UI
  INB["inbox files<br/>instance::poll_current (every 250 ms in tick)"] -->|"project path"| UI
  PRB["probe Pool<br/>media::probe"] -->|"mpsc: ProbeResult"| UI
  PRX["proxy Pool (1 worker)<br/>ProxyWorker::generate"] -->|"mpsc: ProxyEvent"| UI
  EXP["export worker<br/>export::run (+ spatial pre-pass, output check threads)"] -->|"mpsc: ExportEvent"| UI
  STL["still thread<br/>preview::still_thread (latest request only; kept background)"] -->|"mpsc: Frame"| UI
  PLY["playback thread<br/>preview::start_playback"] -->|"sync_channel: Frame"| UI
  PLY --> AUD["audio reader thread"] --> BUF["AudioBuffer (Arc)"] --> SINK["cpal callback (AudioSink)"]
  STT["speech thread<br/>stt::start"] -->|"mpsc: SttEvent"| UI
  MOD["model install thread<br/>stt::install_model"] -->|"mpsc: ModelEvent"| UI
  TRK["track reader thread<br/>app::subs_create"] -->|"mpsc"| UI
  LUT["LUT load and preparation threads<br/>lut::Load, lut::Prep"] -->|"mpsc: percent, table"| UI
  FNT["font scan thread<br/>fonts::Fonts::start_with"] -->|"Arc Mutex list"| UI
  AUT["autosave worker<br/>autosave::start"] -->|"mpsc: Result"| UI
  IMP["import thread, one at a time<br/>nested::start (+ its export worker)"] -->|"mpsc: nested::Event"| UI
  STR["snapshot thread, one at a time<br/>strips::Strips::tick"] -->|"mpsc: Thumb"| UI
  SCH["jobs::Scheduler<br/>(cap 3–6 ffmpeg processes, priorities, preemption)"]
  PRB & PRX & EXP & STL & PLY & STT & HWP & IMP & STR -.->|"Limited runner: a slot per process"| SCH
```

- Nothing slow runs on the UI thread: `AppState::tick` drains every channel once per frame.
- Cancellation is an `Arc<AtomicBool>` per job (dropping a `Load` or `Prep` stops it); `Limited::cancellable` kills the job's processes.
- A job of an old project is dropped by replacing its channel (New/Open); late results go nowhere.
- Every child process is started by a `ffmpeg::Runner` (`SystemRunner` in the programs, fakes in the tests) with an argument list, never a shell.

## videolore player
```mermaid
flowchart LR
  PUI["player ui::PlayerApp"] --> PST["player app::PlayerState<br/>(Command, tick)"]
  PUI --> FS["fullscreen"]
  PUI --> SPL["videolore::ui splash window, theme"]
  PST --> SUBS["player subs<br/>(text tracks, .srt, picture tracks)"]
  PST --> PRE["preview::Preview"]
  PST --> MED["media::probe"]
  PST --> PRX2["proxy::ProxyWorker"]
  PST --> CAM["camera (360 view)"]
  PST --> SIB["siblings (Open in editor)"]
  PST --> LST["player playlist<br/>(order, lengths)"]
  PUI --> SHOT["player screenshot<br/>(crop, PNG)"]
  PST --> SHOT
  SUBS --> SRT["srt"]
```

The player builds its own `Project` of one clip in memory (never saved) and reuses the editor's engine; it is built without the `stt` feature, so it has no speech engine and no network code (checked in CI with `cargo tree`).

## videolore screen spy
```mermaid
flowchart LR
  RUI["ui::RecorderApp"] --> APP["app::RecorderState<br/>(Command, tick)"]
  RUI --> ABT["about"] --> NOT["videolore::notices"]
  APP --> PLAN["plan (ffmpeg arguments)"] --> DEV["devices"]
  APP --> SES["session::Platform"]
  APP --> SCR["screens (display-info)"]
  APP --> SET["settings"]
  APP --> PRO["process::Spawner<br/>(one ffmpeg capture)"]
  PRO -->|"raw frames on stdout"| PRV["preview::Feed (thread, newest only)"] --> APP
  APP --> AUD["audio::Sound"]
  AUD -->|"Linux: -f pulse inputs"| PRO
  AUD -->|"Windows, macOS: cpal callback, channel, Feeder thread, TCP 127.0.0.1"| PRO
  APP -->|"start checks (thread, one-shot channel)"| RUN["videolore ffmpeg::Runner<br/>(-devices, -encoders, source checks)"]
  AUD -->|"macOS"| TAP["macos::SystemTap, microphone_gate"]
```

Threads: the UI thread; one thread per start for the checks (it calls `Sound::open`); per capture one preview reader and one stderr reader; on Windows and macOS one thread that owns the cpal streams (`audio::hold`), the system's audio thread and a feeder thread per source. The UI thread never waits for the checks or the capture: `tick` polls them, and Stop has a 15 s deadline.
