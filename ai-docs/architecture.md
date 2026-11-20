# ai-notes: architecture

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

target: 1.6.1 PLAN (2026-11-19; whole-video effects per clip + format 9, visible box edges, paused-preview fix, CI time; ADR-120..127; details ai-docs/architecture-plan-1.6.1.md; no open SEVERE question; Q-F closed 2026-11-19 (Scale/Move combined, clip LUT wins) | built 1.6.0 (splash screen, Apply LUT, videolore player as workspace member `player/` with its own installers, icons; ADR-105..119; details ai-docs/architecture-plan-1.6.0.md; questions: no open SEVERE question; Q-A, Q-B, Q-C closed 2026-11-19 (answers 1); Q-E origin of splash image open before release) | built 1.5.0 (frame: aspect, Scale, Move, keyframes; schema 7; ADR-093..104; ai-docs/architecture-plan-1.5.0.md) | 1.4.x (export dialog, clean audio, text width; ADR-080..092) | 1.3.0 (text items, 360 video; ADR-066..079) | MSRV 1.100 (ADR-012) | toolchain: latest stable | ed 2024 | GPL-3.0-only
scope 0.2.0..1.2.6: see history in the sub-documents (1.3.0 plan, 1.4.0 plan); decisions in the ADRs below.

## ai sub-documents (read only when a task needs them)
- ai-docs/architecture-plan-1.6.1.md § whole file — L1-117 (117 lines, PLAN 2026-11-19, Q-F decided at § Decided: Q-F L64-73): code facts, paused-preview cause and fix (ADR-120), CI measurements and plan (ADR-126), visible edges (ADR-121), per-clip whole-video effects and format 9 (ADR-122..125), splash 1.6.1 (ADR-127), Q-F closed (Decided Q-F), ordered steps for the Principal Engineer, tests per REQ, NFR, security, risks, future work.
- ai-docs/architecture-plan-1.6.0.md § whole file — L1-123 (123 lines, PLAN 2026-11-19, REFINE 2026-11-19): code facts (workspace not yet, lib features, package.sh, CI, GIF facts, crate verification), ADR-105..119, components, data flow, ordered steps for the Principal Engineer, tests per REQ, NFR, security, risks, questions Q-A..Q-C closed, Q-D decided, Q-E open pre-release, future work.
- ai-docs/architecture-plan-1.5.0.md § whole file — L1-103 (103 lines, PLAN 2026-11-19): code facts, ADR-093..104, components, ordered steps, tests per REQ, NFR, security, risks, Q-A (closed: 4096 px cap), future work.
- ai-docs/architecture-plan-1.4.0.md § Code facts — L5-14; § Components and changes (1.4.0) — L15-30; § ADRs (1.4.0) ADR-080..091 — L31-49; § Schema v6 (REQ-123) — L50-56; § Export dialog flow — L57-62; § NFR (1.4.0) — L63-71; § Security (1.4.0) — L72-79; § Crates — L80-82; § Ordered steps for the Principal Engineer — L83-97; § Tests per REQ (summary) — L98-108; § Risks — L109-119; § Questions — L120-124; § Future work — L125-131 (131 lines, PLAN 2026-11-17)
- ai-docs/architecture-plan-1.3.0.md § whole file — L1-136 (136 lines): code facts, crate decision, text model, effects to ASS events, flat render, selection, context menu, 360 anchors, 360 video export, project v5, rename, tests per REQ, ordered steps (done), security, risks, questions; § Built in 1.3.0 and deviations — L131-136 (DEV-1 time to live, DEV-2 spherical marker, DEV-3 anchored subtitle lines, DEV-4 selection).

## human docs index (generated from grep -n '^#' of docs/architecture*.md on 2026-11-19; refresh when a managed doc changes; read a range with sed -n 'A,Bp')
- docs/architecture-360.md § Title — L1-3 (3 lines)
- docs/architecture-360.md § Approach — L5-8 (4 lines)
- docs/architecture-360.md § Recognising a 360 video — L10-14 (5 lines)
- docs/architecture-360.md § Camera and POVs — L16-21 (6 lines)
- docs/architecture-360.md § Preview and export graph — L23-30 (8 lines)
- docs/architecture-360.md § Performance and limits — L32-36 (5 lines)
- docs/architecture-360.md § Tests — L38-41 (4 lines)
- docs/architecture-360.md § Risks — L43-47 (5 lines)
- docs/architecture-360.md § Anchored text and subtitles (1.3.0) — L49-58 (10 lines)
- docs/architecture-360.md § Export as 360 video (1.3.0) — L60-65 (6 lines)
- docs/architecture-360.md § Tests for 1.3.0 — L67-70 (4 lines)
- docs/architecture-boxes.md § Title — L1-4 (4 lines)
- docs/architecture-boxes.md § Boxes on the picture — L5-9 (5 lines)
- docs/architecture-boxes.md § Moving and resizing — L10-14 (5 lines)
- docs/architecture-boxes.md § Effect commands — L15-20 (6 lines)
- docs/architecture-boxes.md § Context Menu and text boxes (1.3.0) — L21-25 (5 lines)
- docs/architecture-boxes.md § Text items, width and dragging (1.4.0) — L26-32 (7 lines)
- docs/architecture-boxes.md § Visible edges (1.6.1, planned, REQ-145, REQ-146) — L33-37 (5 lines)
- docs/architecture-boxes.md § Tests — L38-44 (7 lines)
- docs/architecture-build.md § Title — L1-4 (4 lines)
- docs/architecture-build.md § Rust toolchain policy — L5-8 (4 lines)
- docs/architecture-build.md § Workspace (1.6.0, planned) — L9-14 (6 lines)
- docs/architecture-build.md § CI (planned changes for 1.6.0) — L15-23 (9 lines)
- docs/architecture-build.md § CI (as built) — L24-31 (8 lines)
- docs/architecture-build.md § CI time (planned for 1.6.1, QG-06) — L32-39 (8 lines)
- docs/architecture-decisions.md § Title — L1-33 (33 lines)
- docs/architecture-dependencies.md § Title — L1-5 (5 lines)
- docs/architecture-dependencies.md § Build features — L7-13 (7 lines)
- docs/architecture-dependencies.md § Rust crates — L15-47 (33 lines)
- docs/architecture-dependencies.md § Bundled with the installers (not Rust) — L49-57 (9 lines)
- docs/architecture-dependencies.md § Tools used only to build, test or package — L59-73 (15 lines)
- docs/architecture-dependencies.md § Platform build requirements (from source) — L75-80 (6 lines)
- docs/architecture-effects.md § Title — L1-4 (4 lines)
- docs/architecture-effects.md § Applying an effect — L5-14 (10 lines)
- docs/architecture-effects.md § Fade through black — L15-23 (9 lines)
- docs/architecture-effects.md § Clip volume — L24-30 (7 lines)
- docs/architecture-effects.md § Clean audio (1.4.0) — L31-39 (9 lines)
- docs/architecture-effects.md § Whole video per clip (1.6.1, planned) — L40-47 (8 lines)
- docs/architecture-effects.md § When the export must re-encode — L48-50 (3 lines)
- docs/architecture-effects.md § Tests — L51-56 (6 lines)
- docs/architecture-export-dialog.md § Title — L1-3 (3 lines)
- docs/architecture-export-dialog.md § Opening the dialog — L5-8 (4 lines)
- docs/architecture-export-dialog.md § Modes — L10-14 (5 lines)
- docs/architecture-export-dialog.md § Export file — L16-21 (6 lines)
- docs/architecture-export-dialog.md § Video format (Full video only) — L23-40 (18 lines)
- docs/architecture-export-dialog.md § Audio format (Full video and Only audio) — L42-46 (5 lines)
- docs/architecture-export-dialog.md § 360 choice — L48-50 (3 lines)
- docs/architecture-export-dialog.md § What the export does with the choices — L52-55 (4 lines)
- docs/architecture-export-dialog.md § Tests — L57-61 (5 lines)
- docs/architecture-export-dialog.md § Risks — L63-66 (4 lines)
- docs/architecture-ffmpeg.md § Title — L1-3 (3 lines)
- docs/architecture-ffmpeg.md § Why ffmpeg is bundled — L5-6 (2 lines)
- docs/architecture-ffmpeg.md § Which builds — L8-20 (13 lines)
- docs/architecture-ffmpeg.md § Pinning and download — L22-28 (7 lines)
- docs/architecture-ffmpeg.md § Where it is installed — L30-38 (9 lines)
- docs/architecture-ffmpeg.md § How it is found — L40-51 (12 lines)
- docs/architecture-ffmpeg.md § Subtitle rendering and fonts — L53-57 (5 lines)
- docs/architecture-ffmpeg.md § Licences and source code — L59-68 (10 lines)
- docs/architecture-ffmpeg.md § Size — L70-71 (2 lines)
- docs/architecture-ffmpeg.md § Tests — L73-75 (3 lines)
- docs/architecture-ffmpeg.md § Risks — L77-79 (3 lines)
- docs/architecture-frame.md § Title — L1-4 (4 lines)
- docs/architecture-frame.md § Overview — L5-11 (7 lines)
- docs/architecture-frame.md § Frame size (Change aspect) — L12-19 (8 lines)
- docs/architecture-frame.md § Picture composition — L20-29 (10 lines)
- docs/architecture-frame.md § Scale and Move keyframes — L30-38 (9 lines)
- docs/architecture-frame.md § Filter graph — L39-43 (5 lines)
- docs/architecture-frame.md § Export and 360 video — L44-47 (4 lines)
- docs/architecture-frame.md § Preview — L48-52 (5 lines)
- docs/architecture-frame.md § Project format — L53-57 (5 lines)
- docs/architecture-frame.md § Non-functional targets (1.5.0) — L58-64 (7 lines)
- docs/architecture-frame.md § Security — L65-69 (5 lines)
- docs/architecture-frame.md § Dependencies — L70-72 (3 lines)
- docs/architecture-frame.md § Visible edges (1.6.1, planned) — L73-77 (5 lines)
- docs/architecture-frame.md § Whole-video Scale and Move (1.6.1, planned; Q-F rule in the last bullet) — L78-82 (5 lines)
- docs/architecture-frame.md § Tests — L84-89 (6 lines)
- docs/architecture-frame.md § Risks — L90-96 (7 lines)
- docs/architecture-frame.md § Decided: size limit of the frame — L97-98 (2 lines)
- docs/architecture-hardware-export.md § Title — L1-3 (3 lines)
- docs/architecture-hardware-export.md § When the hardware is used — L5-14 (10 lines)
- docs/architecture-hardware-export.md § Supported hardware — L16-25 (10 lines)
- docs/architecture-hardware-export.md § Encoder settings — L27-45 (19 lines)
- docs/architecture-hardware-export.md § VA-API constant quality — L47-52 (6 lines)
- docs/architecture-hardware-export.md § Background hardware check — L54-73 (20 lines)
- docs/architecture-hardware-export.md § Export with hardware — L75-82 (8 lines)
- docs/architecture-hardware-export.md § Output check — L84-92 (9 lines)
- docs/architecture-hardware-export.md § Remembering failures — L94-97 (4 lines)
- docs/architecture-hardware-export.md § Settings file — L99-102 (4 lines)
- docs/architecture-hardware-export.md § Settings window — L104-106 (3 lines)
- docs/architecture-hardware-export.md § Performance — L108-112 (5 lines)
- docs/architecture-hardware-export.md § Testing — L114-118 (5 lines)
- docs/architecture-hardware-export.md § Security — L120-123 (4 lines)
- docs/architecture-hardware-export.md § Known limitations — L125-132 (8 lines)
- docs/architecture-hardware-export.md § Risks — L134-142 (9 lines)
- docs/architecture-icons.md § Title — L1-3 (3 lines)
- docs/architecture-icons.md § Choice of set — L5-7 (3 lines)
- docs/architecture-icons.md § How icons are drawn — L9-14 (6 lines)
- docs/architecture-icons.md § Tests (planned) — L16-20 (5 lines)
- docs/architecture-icons.md § Notices and packaging — L22-24 (3 lines)
- docs/architecture-icons.md § Risks — L26-28 (3 lines)
- docs/architecture-lut.md § Title — L1-3 (3 lines)
- docs/architecture-lut.md § Overview — L5-6 (2 lines)
- docs/architecture-lut.md § Reading the file (`src/lut.rs`, pure) — L8-16 (9 lines)
- docs/architecture-lut.md § Storage and project format 8 — L18-23 (6 lines)
- docs/architecture-lut.md § Missing or changed files — L25-28 (4 lines)
- docs/architecture-lut.md § Where it is applied (per segment) — L30-38 (9 lines)
- docs/architecture-lut.md § Export — L40-42 (3 lines)
- docs/architecture-lut.md § Non-functional targets (1.6.0) — L44-47 (4 lines)
- docs/architecture-lut.md § Tests (planned) — L49-52 (4 lines)
- docs/architecture-lut.md § Risks — L54-58 (5 lines)
- docs/architecture-media-pipeline.md § Title — L1-4 (4 lines)
- docs/architecture-media-pipeline.md § Job queue: a fixed process limit — L5-25 (21 lines)
- docs/architecture-media-pipeline.md § Preview — L26-32 (7 lines)
- docs/architecture-media-pipeline.md § Export — L33-43 (11 lines)
- docs/architecture-media-pipeline.md § Music mix — L44-49 (6 lines)
- docs/architecture-media-pipeline.md § Proxies — L50-59 (10 lines)
- docs/architecture-media-pipeline.md § Memory — L60-73 (14 lines)
- docs/architecture-media-pipeline.md § Risks — L74-76 (3 lines)
- docs/architecture-packaging.md § Title — L1-3 (3 lines)
- docs/architecture-packaging.md § What is produced — L5-22 (18 lines)
- docs/architecture-packaging.md § Windows — L24-29 (6 lines)
- docs/architecture-packaging.md § macOS — L31-35 (5 lines)
- docs/architecture-packaging.md § Linux — L37-47 (11 lines)
- docs/architecture-packaging.md § Opening a project by double-click — L49-51 (3 lines)
- docs/architecture-packaging.md § Icon — L53-57 (5 lines)
- docs/architecture-packaging.md § Release pipeline — L59-68 (10 lines)
- docs/architecture-packaging.md § CI runners and actions — L70-86 (17 lines)
- docs/architecture-packaging.md § Bundled ffmpeg refresh (0.3.0) — L88-91 (4 lines)
- docs/architecture-packaging.md § Two products (1.6.0, planned) — L93-98 (6 lines)
- docs/architecture-packaging.md § Risks — L100-103 (4 lines)
- docs/architecture-performance.md § Title — L1-3 (3 lines)
- docs/architecture-performance.md § Detecting the computer — L5-17 (13 lines)
- docs/architecture-performance.md § Start-up — L19-32 (14 lines)
- docs/architecture-performance.md § Recommended settings — L34-50 (17 lines)
- docs/architecture-performance.md § "How hard videolore works" — L52-81 (30 lines)
- docs/architecture-performance.md § Preview quality — L83-90 (8 lines)
- docs/architecture-performance.md § Responsiveness while busy — L92-93 (2 lines)
- docs/architecture-performance.md § 0.3.0 performance review — L95-108 (14 lines)
- docs/architecture-performance.md § Hardware video encoding (since 0.4.0) — L110-111 (2 lines)
- docs/architecture-performance.md § Risks — L113-116 (4 lines)
- docs/architecture-player.md § Title — L1-3 (3 lines)
- docs/architecture-player.md § Overview — L5-6 (2 lines)
- docs/architecture-player.md § Crate layout (workspace) — L8-12 (5 lines)
- docs/architecture-player.md § How it works — L14-20 (7 lines)
- docs/architecture-player.md § Opening files — L22-25 (4 lines)
- docs/architecture-player.md § Settings, files and privacy — L27-30 (4 lines)
- docs/architecture-player.md § Installers — L32-35 (4 lines)
- docs/architecture-player.md § Tests (planned) — L37-43 (7 lines)
- docs/architecture-player.md § Risks — L45-49 (5 lines)
- docs/architecture-projects.md § Title — L1-4 (4 lines)
- docs/architecture-projects.md § Format versions — L5-26 (22 lines)
- docs/architecture-projects.md § Loading and validation (format 8 -> 9 conversion, Q-F rule) — L27-37 (11 lines)
- docs/architecture-projects.md § Unsaved changes — L39-44 (6 lines)
- docs/architecture-projects.md § What is not in a project — L45-46 (2 lines)
- docs/architecture-security.md § Title — L1-4 (4 lines)
- docs/architecture-security.md § Threat model — L5-11 (7 lines)
- docs/architecture-security.md § Running ffmpeg — L12-21 (10 lines)
- docs/architecture-security.md § Project and settings files — L22-32 (11 lines)
- docs/architecture-security.md § Subtitles — L33-37 (5 lines)
- docs/architecture-security.md § Network — L38-44 (7 lines)
- docs/architecture-security.md § Temporary files (since 0.3.0) — L45-51 (7 lines)
- docs/architecture-security.md § Other files written outside projects — L52-60 (9 lines)
- docs/architecture-security.md § Text items and fonts (1.3.0) — L61-66 (6 lines)
- docs/architecture-security.md § Spherical marker (1.3.0) — L67-70 (4 lines)
- docs/architecture-security.md § Export dialog and clean audio (1.4.0) — L71-77 (7 lines)
- docs/architecture-security.md § Frame effects (1.5.0) — L78-82 (5 lines)
- docs/architecture-security.md § Apply LUT, player and assets (1.6.0, planned) — L83-87 (5 lines)
- docs/architecture-security.md § Changes planned for 1.6.1 — L88-92 (5 lines)
- docs/architecture-security.md § Unsafe code — L93-95 (3 lines)
- docs/architecture-security.md § Unsigned installers — L96-98 (3 lines)
- docs/architecture-security.md § Supply chain — L99-105 (7 lines)
- docs/architecture-splash.md § Title — L1-4 (4 lines)
- docs/architecture-splash.md § Overview — L5-7 (3 lines)
- docs/architecture-splash.md § Assets — L8-15 (8 lines)
- docs/architecture-splash.md § Layout — L16-21 (6 lines)
- docs/architecture-splash.md § Behaviour — L22-28 (7 lines)
- docs/architecture-splash.md § 1.6.1 assets (planned) — L29-33 (5 lines)
- docs/architecture-splash.md § Tests (planned) — L34-39 (6 lines)
- docs/architecture-splash.md § Risks — L40-43 (4 lines)
- docs/architecture-subtitle-tracks.md § Title — L1-3 (3 lines)
- docs/architecture-subtitle-tracks.md § Finding subtitle tracks — L5-10 (6 lines)
- docs/architecture-subtitle-tracks.md § Reading a track — L12-16 (5 lines)
- docs/architecture-subtitle-tracks.md § Use the video's subtitles or recognise the speech — L18-21 (4 lines)
- docs/architecture-subtitle-tracks.md § Matching words to the speech — L23-29 (7 lines)
- docs/architecture-subtitle-tracks.md § When matching is not possible — L31-36 (6 lines)
- docs/architecture-subtitle-tracks.md § Export as SRT — L38-41 (4 lines)
- docs/architecture-subtitle-tracks.md § Tests — L43-47 (5 lines)
- docs/architecture-subtitle-tracks.md § Risks — L49-52 (4 lines)
- docs/architecture-subtitles.md § Title — L1-3 (3 lines)
- docs/architecture-subtitles.md § Model — L5-11 (7 lines)
- docs/architecture-subtitles.md § Styles and burn-in — L13-24 (12 lines)
- docs/architecture-subtitles.md § Speech-to-text — L26-39 (14 lines)
- docs/architecture-subtitles.md § Creating subtitles — L41-48 (8 lines)
- docs/architecture-subtitles.md § Position on screen (1.2.0) — L50-60 (11 lines)
- docs/architecture-subtitles.md § CPU requirements (since 0.2.1) — L62-71 (10 lines)
- docs/architecture-subtitles.md § Subtitle quality levels (0.3.0) — L73-85 (13 lines)
- docs/architecture-subtitles.md § Speech model download — L87-98 (12 lines)
- docs/architecture-subtitles.md § Tests — L100-102 (3 lines)
- docs/architecture-subtitles.md § Risks — L104-110 (7 lines)
- docs/architecture-text.md § Title — L1-3 (3 lines)
- docs/architecture-text.md § Model — L5-8 (4 lines)
- docs/architecture-text.md § Rendering — L10-14 (5 lines)
- docs/architecture-text.md § Effects — L16-21 (6 lines)
- docs/architecture-text.md § Fonts and safety — L23-27 (5 lines)
- docs/architecture-text.md § Box, best fit and place (1.4.0) — L29-37 (9 lines)
- docs/architecture-text.md § Tests — L39-43 (5 lines)
- docs/architecture-text.md § Risks — L45-50 (6 lines)
- docs/architecture-text.md § Deviations (1.3.0) — L52-53 (2 lines)
- docs/architecture-ui.md § Title — L1-3 (3 lines)
- docs/architecture-ui.md § Window structure — L5-12 (8 lines)
- docs/architecture-ui.md § Resizable sections — L14-21 (8 lines)
- docs/architecture-ui.md § Everything reachable — L23-28 (6 lines)
- docs/architecture-ui.md § Menus, toolbar and shortcuts — L30-38 (9 lines)
- docs/architecture-ui.md § Status bar — L40-41 (2 lines)
- docs/architecture-ui.md § Effects and popups — L43-48 (6 lines)
- docs/architecture-ui.md § Timeline markers — L50-54 (5 lines)
- docs/architecture-ui.md § Time ruler (1.2.0) — L56-59 (4 lines)
- docs/architecture-ui.md § Recent projects — L61-62 (2 lines)
- docs/architecture-ui.md § Drag and drop — L64-66 (3 lines)
- docs/architecture-ui.md § Settings file — L68-69 (2 lines)
- docs/architecture-ui.md § Testing the interface — L71-72 (2 lines)
- docs/architecture-ui.md § Performance — L74-77 (4 lines)
- docs/architecture-ui.md § Selection (1.3.0) — L79-82 (4 lines)
- docs/architecture-ui.md § Add Text popup (1.3.0) — L84-86 (3 lines)
- docs/architecture-ui.md § Risks — L88-91 (4 lines)
- docs/architecture-ui.md § Icons, splash and the Apply LUT popup (1.6.0, planned) — L93-96 (4 lines)
- docs/architecture.md § Title — L1-3 (3 lines)
- docs/architecture.md § Overview — L5-8 (4 lines)
- docs/architecture.md § Sub-documents — L10-35 (26 lines)
- docs/architecture.md § Components — L37-78 (42 lines)
- docs/architecture.md § Data flow — L80-97 (18 lines)
- docs/architecture.md § Main technical decisions — L99-100 (2 lines)
- docs/architecture.md § Projects — L102-103 (2 lines)
- docs/architecture.md § Deviations from the 1.3.0 plan (as built) — L105-109 (5 lines)
- docs/architecture.md § Non-functional targets — L111-133 (23 lines)
- docs/architecture.md § Build, CI and versions — L135-136 (2 lines)
- docs/architecture.md § Known risks — L138-152 (15 lines)

## component map (src/ modules)
- time: µs/rational time, ts parse/format, no floats in cut math. pure.
- media: MediaInfo from ffprobe json; 0.2.0: MediaStatus{Analysing,Ready,Rejected(reason)}; audio-only accepted, kind() Video/Audio. probe on probe pool.
- timeline: video track; clips, cuts/splits, fades, invariants. pure. semantics unchanged.
- music (NEW): MusicTrack{clips: Vec<MusicClip{id,media,src_in,src_out,start}>, next_id}; sorted, non-overlap; place/move/trim/split/delete. pure. audio-only media only.
- subtitles (NEW): Subtitles{language:Option<String>, language_overridden, preset:PresetId, lines: Vec<Line{id,media,start,end,text,words:Vec<Word{start,end,text}>,edited}>, next_id}. times = SOURCE media time (REQ-018 free). project(&Timeline)->Vec<ShownLine> pure projection; edit ops; presets.
- ass (NEW): pure ASS writer: presets, size formula, \k karaoke, escape.
- history: snapshot Edit{timeline,music,subtitles,mix}; Mix{video_volume:u8,video_muted,music_volume:u8,music_muted}. cap 100.
- project: schema v2 (v3 since 1.2.0, ADR-051) + v1 migration + TooNew; ProjectDoc (persisted model) for dirty tracking; Save/Save As. pure+fs.
- format: output format/encoder table (unchanged).
- render: ExportPlan; 0.2.0: monotone-run input grouping, music graph, subtitle filter, -tag:v hvc1, proxy substitution (Source::Preview only), single-segment still graph, FrameKey. pure.
  0.4.0: export_args(plan,paths,version,threads,hw:Option<&HwEncode>): None = byte-identical 0.3.0 argv (golden test); Some = hw.global after head(), graph_tail appended (also script file), map out_label, sw video args (-c:v, vextra, -b:v/-maxrate/-bufsize, -pix_fmt) replaced by hw.vcodec; audio untouched. hvc1 rule = codec HEVC && muxer mp4|mov (was vencoder=="libx265"). expected_video_frames(&Plan) (sum per segment, same rounding as fps filter). reference_frame_args(project,at,&Geometry,version,script,out) (still graph Source::Export, full geometry, yuv420p, optional cap scale, 1 rawvideo frame).
- hwenc (NEW 0.4.0, ADR-037/038, mostly pure): Codec{H264,Hevc}, Vendor{Nvenc,Qsv,Amf,Vaapi,VideoToolbox}, Candidate{vendor,device:Option<PathBuf>}; candidates(os,codec,&encoders,&render_nodes) fixed order; render_nodes(dir) /dev/dri/renderD\d+ char devs sorted, <=4; encode_args(&Candidate,Codec,target_bps)->HwEncode{global,graph_tail,out_label,vcodec}; probe(runner,ffmpeg,&cands,ProbeLimits{per 1 s,retry 4 s,total 10 s},cancel)->HwStatus{h264,hevc:Option<Candidate>,notes:Vec<(Codec,Vendor,Reason)>} + HwStatus::inconclusive() (0.4.2); Reason{DriverTooOld,NotAvailable,TimedOut,Failed}.
- outcheck (NEW 0.4.0, ADR-039): builds check cmds, parses ffprobe JSON + decode progress + ffmpeg psnr filter log (PSNR computed by ffmpeg, not Rust, since follow-up); Result<(),CheckFailure{Decode,FrameCount{exp,got},Metadata(&'static str),Picture{idx,psnr}}>; CQ size rule (>1.5x usual -> fail, not remembered).
- ffmpeg: locate (bundled), version/filters/encoders probe, Runner trait spawn(+cwd)/run (stdout cap 16 MiB), progress parse, kill, stderr ring 64 KiB. only module spawning ffmpeg.
- jobs (NEW): Scheduler (cap 3; 0.3.0: set_cap(u8) per effort level, MAX_CAP 6, ADR-029), Permit, LimitedRunner, spawn_group, preemption, cancel. wraps Runner -> every ffmpeg/ffprobe spawn goes through it.
- export: worker w/ Export-priority runner; writes ASS+font into private work folder used as cwd (0.3.0: scratch::Work, ADR-033); partial file + atomic rename. 0.3.0: threads:Option<u32> from budget. 0.4.0: ExportJob{hw:Option<HwOffer{h264,hevc:Option<Candidate>}>, stall_timeout (30 s; tests ms), check_limits}; hw attempt + watchdog -> outcheck -> sw redo (ADR-039). Done{path, method:ExportMethod{StreamCopy,Software,Hardware(Vendor),SoftwareAfterProblem{codec,vendor,problem}}}; Problem{ExitedWithError,Stalled,CheckFailed(CheckFailure)} (crash/signal = ExitedWithError).
- preview: wall-clock master + audio drift correction; latest-only still mailbox + 80 ms debounce; music+subs in graph; proxies.
- proxy (NEW): rule, key, cache dir, bg generation + progress, clear, size.
- stt (NEW): Transcriber + ModelFetcher traits + fakes, extraction job, chunking, word ts, lang detect/override. WhisperTranscriber + UreqFetcher cfg(feature="stt").
- machine (NEW; reworked 0.3.0, ADR-028): Machine{logical,physical,ram_bytes,os,arch,cpu:CpuFeatures}, detect(), speech_supported(), fingerprint(), own_rss(). tests inject Machine.
- settings (NEW 0.3.0, ADR-028/029): Settings v2 (moved out of app.rs), LegacySettings, startup() pure, recommend(), budget(), availability(), summary(), main_texts(), load_file()/save (only I/O). 0.4.0 (ADR-040): export:ExportChoice{BestQuality(default),Faster}, hardware_off:Vec<HardwareOff{format,method,version}>; Choices 4 fields; recommend() export always BestQuality.
- scratch (NEW 0.3.0, ADR-033): private work folders <cache>/work/<rand>/ 0700 + lock + targets registry; cleanup_stale() at start.
- ui/settings (NEW 0.3.0): egui::Window Settings (main 3 choices + status + storage; collapsed Advanced). 0.4.0: main + Export choice + "no video hardware" note; Advanced + hw status per format, probe notes, last method, turned-off line, "Try video hardware again".
- notices (NEW): static table of bundled components + generated rust crate notices (include_str!). 0.3.0: all 3 speech models.
- desktop (NEW): linux self-integration (menu entry, icon, mime) + removal; macos odoc Apple Event handler.
- ui: thin egui panels. app: AppState::dispatch, tick() drains channels, settings. 0.4.0: HwState{probe: NotChecked|Checking(gen)|Done(HwStatus), last_method (session only)}; probe thread -> mpsc -> tick; generation counter drops stale results; SetExport saves (+ probe if Faster); Done{SoftwareAfterProblem} -> record hardware_off + save.
- packaging smoke test (0.4.0): bundled -encoders must list hw encoders per OS (ADR-037).
- 1.1.0 UI restructure (ADR-041..048), all logic pure + outside egui where possible:
  - ui/mod.rs frame_ui order: app.tick; dropped files -> DropFiles; keymap (consumes matched events); Panel::top menubar (egui::MenuBar) + Panel::top toolbar + Panel::bottom statusbar (resizable(false), content height); CentralPanel = content area split by ui/layout; sections + splitters; popups/windows; modals; dispatch. start-up: ctx.options_mut(zoom_with_keyboard=false) (ADR-046).
  - ui/layout.rs (rewritten, pure, ADR-041): Fractions{top,media,timeline} default {2/3,0.30,0.70}; Mins{media w160, effects w160, preview w240, timeline w240, row h100}; GAP 6 px (splitters in gap); sections(area,f,m)->SectionRects{media,preview,timeline,effects,splitters[3]} whole-pixel borders, exact cover; effective(f,area,m) mins at draw only; drag(f,Splitter,delta,area,m) clamp both neighbours; Splitter{MediaPreview,TimelineEffects,Rows}. section(ui,rect,title,header_extra,body): fixed header + ScrollArea VisibleWhenNeeded; registers container in UiCache.containers (id, viewport, content rect) for REQ-068 audit.
  - ui/actions.rs (NEW, ADR-042): enum ActionId (38: New Open Save SaveAs Import Export Quit Undo Redo Cut Copy Paste Delete Split Merge MoveLeft MoveRight AddToTimeline AddMusic ClipDetails Settings PlayPause GoStart GoEnd PrevFrame NextFrame Fades Subtitles Volume ToggleFadeAtPlayhead GenerateSubtitles AddSubtitleLine ZoomIn ZoomOut ZoomFit ResetLayout Shortcuts About); Action{id,label,shortcuts:&'static [KeyboardShortcut],tooltip}; const ACTIONS; ActionId::command(&AppState)->Command (Export->CancelExport while exporting; GoEnd->Seek(duration)). single source for menus, toolbar, tooltips, shortcuts window, keymap::command_for.
  - ui/menu.rs (NEW): File/Edit/Playback/Effects/View/Help per REQ-069; Button::shortcut_text(ctx.format_shortcut); greyed = !app.is_enabled(&cmd); long menus in ScrollArea::vertical().max_height(available). File > Open recent submenu (sanitised names, full path tooltip, Clear list).
  - ui/toolbar.rs (NEW): 12 actions; pure fit(widths,avail,overflow_w)->usize; rest in "»" menu (same greying/tooltips). widths from ctx.fonts layout_no_wrap + padding.
  - ui/status.rs (NEW): left status/ffmpeg-check msg ellipsized + tooltip; middle AppState::background_jobs()->Vec<Job{label,progress:Option<f32>,cancel:Option<Command>}> (imports analysing, preview copies, model download, subtitle gen + Cancel, export + Cancel); right "Output: …" (moved from effects panel).
  - ui/effects.rs (rewritten): list rows Fade through black / Subtitles / Volume with counts (tl.fades.len(), subtitles.project(tl).len(), "Video 80% · Music muted") -> OpenPopup. clip details/export/shortcut list moved out.
  - ui/popups.rs (NEW, ADR-043): egui::Window per Popup, fixed id, .open(&mut bool), vscroll(true), constrain(true), default size fits 800x500, non-modal. bodies Fades / Subtitles (lines via ScrollArea::show_rows) / Volume (mix_settings) / ClipDetails (moved clip_settings + music_clip_settings, follows selection) / Instance (fade: clips, time, Remove; line: text committed on focus loss = 1 undo, start/end ±0.1 s, Delete, Play this line) / Shortcuts (from ACTIONS) / Settings (existing ui/settings). chooser = egui::Popup at pointer.
  - ui/markers.rs (NEW, pure, ADR-048): layout(edit view, zoom, x0)->Vec<Marker{instance,kind,x,x_end,tooltip}>: fade diamond at boundary x (fade time = boundary time), subtitle flag at line start + thin bar to end, "+" Add-fade handle on boundaries w/o fade; culled to visible x; hits(&[Marker],x,tol 4 px)->Vec<Instance>.
  - ui/timeline.rs: rows ruler, marker lane 14 px, video track min 40, music track min 24; content h = max(avail, sum mins); ScrollArea::both; header zoom slider + Fit; double-click clip -> ClipDetails; context_menu per REQ-079 (video clip, music clip, marker, empty); clip names single-row galley w/ '…' + hover full. ZoomBy factor √2 w/ existing clamp.
  - ui/media.rs: section header, drop hint "Import media… or drop files here", hovered_files overlay "Drop to import", name truncation.
  - app: AppState.popups: Popups{open:Vec<Popup>, instance:Option<Instance>, chooser:Option<Vec<Instance>>}; Popup{Settings,Fades,Subtitles,Volume,ClipDetails,Instance,Shortcuts}; Instance{Fade(ClipId,ClipId),Line(LineId,ClipId)}. settings_open -> Popup::Settings (OpenSettings/CloseSettings kept). new Commands: OpenPopup, ClosePopup, OpenInstance (select + seek start + open), ChooseInstance(Vec), SetFade{left,right,on} (idempotent by clip-id pair), RemoveAllFades (1 undo), PlayLine(LineId) (seek+play; tick pauses at stop_at:Option<Micros>), ZoomBy(f32), ZoomFit(=SetZoom(0.0)), SetLayout(Fractions), ResetLayout, OpenRecent(PathBuf), ClearRecent, DropFiles(Vec<PathBuf>). AppState::disabled_reason(&Command)->Option<&'static str> next to is_enabled (REQ-072). AppState::edit(now,label,f). pure app::drop_plan(paths)->DropPlan{Open(first .vlproj, ignored_count)|Import(paths)}.
  - history: History<T> stores (T,&'static str); record(before,label), undo_label(), redo_label(); label travels with snapshot. labels from Command::undo_name() ("Split","Remove fade","Edit subtitle text"…). menu "Undo Split" / plain "Undo" when empty (REQ-074).
  - settings: + layout:Option<SavedLayout{top,media,timeline}> (f32), recent:Vec<PathBuf> (ADR-044); pure push_recent(list,path).
  - UiCache: + layout_drag:Option<Fractions> (live during drag), containers registry (tests + REQ-068 audit).
- 1.2.0 (ADR-049..058):
  - timeline: Clip + volume:u8 (default 100) + muted:bool, #[serde(default)], Clip stays Copy+Eq. Timeline + start_fade, end_fade: bool (#[serde(default)]) attached to VIDEO EDGES not clips (ADR-049). fades_of(i) adds edge fades (fade-in idx 0 if start_fade, fade-out last idx if end_fade), length ADR-008. new set_clip_fades(id,on), set_edge_fades(on), set_start_fade, set_end_fade, set_clip_volume(id,vol,muted). split/insert_at copy volume+mute. can_merge needs equal (volume,muted); pure merge_block_reason() -> "The clips have different volumes".
  - music: Mix unchanged; clip_gain(c)=gain(c.volume,c.muted); video-level gain multiplies it.
  - render: Segment.gain:f64; per-segment audio `volume={:.6}` before afade only if gain != 1.0 (defaults -> argv byte-identical 1.1, golden). plan_export "no effects" also needs !start_fade && !end_fade && all clips 100/unmuted. FrameKey.subtitles -> Vec<(String, Option<usize>, Option<Pos>)> + transient drag override.
  - subtitles: Line + position:Option<Pos> (#[serde(default, skip_serializing_if=None)]); Pos{x:u16,y:u16} centre of text box in 1/10000 of output frame (0..=10000), None = preset place; ShownLine carries it (copies share). set_position(&[LineId],Option<Pos>) 1 undo; replace_range(media,from,to,Vec<NewLine>,language) (ADR-055); replace_all kept for whole-video recognition.
  - ass: Some(pos) -> each Dialogue (+ each word-highlight chunk) prefixed `{\an5\pos(X,Y)}` X=round(x*W/10000) Y=round(y*H/10000) in PlayRes; None -> 1.1 output exactly (golden). pure default_centre(preset,W,H,box_h) (x=W/2, y=H-MarginV-box_h/2), box_padding(preset,size).
  - srt (NEW, pure, ADR-054): write(&[ShownLine])->String; parse(&str)->Vec<Cue{start,end,text}> tolerant + capped; strip_tags.
  - align (NEW, pure, ADR-056): align(cues, recognised words) -> lines w/ word timings + unmatched count.
  - media: MediaFormat + subtitle_tracks:Option<Vec<SubtitleTrack>> (#[serde(default)]; None = not checked (1.1 media), Some([]) = none). SubtitleTrack{ordinal:u8, codec:TextCodec{Srt,Ass,MovText,WebVtt,Text}, language:Option<String>, title:Option<String>} (ADR-053).
  - stt: SttJob.ranges: Vec<RangeJob{media,path,range:Option<(Micros,Micros)>,mode:Mode{Recognise, Align{track,language}, Spread{track}}}>; extraction_args(+range -ss/-t before -i); track_args(); SttEvent::Done + replace:Vec<(MediaId,Option<range>)>, unmatched:(usize,usize), fallback_message; iso639_2_to_1() static table.
  - project: SCHEMA_VERSION 3 written always; loads 1,2,3 (ADR-051).
  - app: submodules src/app/effects.rs + src/app/subs_create.rs (app.rs 2641 lines). state selection:BTreeSet<LineId> (UI only, pruned after edit/undo/redo), drag_pos:Option<(Vec<LineId>,Pos)> (transient, in FrameKey), pending_create:Option<CreateRequest>, Modal::SubtitleSource, consent reuse_path flag. Commands ApplyEffect{EffectKind{Fade,Subtitles,Volume}, Target{Clip(ClipId),WholeVideo}}, SetClipVolume, SetEdgeFade{start|end,on}, SelectLine{id,Replace|Toggle}, SelectRange{from,to,add}, ClearSelection, SetLinePositions{ids,pos}, ResetPosition, ExportSrt(PathBuf), ChooseSubtitleSource{Use(label)|Recognise|Cancel}, UseWithoutMatching. Instance + FadeStart, FadeEnd, ClipVolume(ClipId); popups titled "— whole video" / "— <clip name>".
  - ui/ruler.rs (NEW, pure, ADR-058); ui/effects.rs rows name only + drag source; ui/timeline.rs DnD target + Effects ▸ context submenu + Shift+drag overlay; ui/markers.rs + start/end fade diamonds, clip-volume marker, modifier click; ui/popups.rs clip volume / edge fade instances, Reset position, Clip details volume, list multi-select; ui/mod.rs preview hit-test + drag ghost, egui FontFamily::Name("subtitle") = bundled Noto Sans Bold (measure only); placement::boxes(project,at,geometry,measure) pure; menu/actions: Export subtitles (.srt)…, Apply to selected clip ▸, Apply to whole video ▸, Reset subtitle position, shortcuts rows Ctrl/Cmd+click + Shift+drag.
- 1.2.6 (ADR-059..065):
  - camera (NEW, pure): Camera{yaw_cd:i32 (-18000,18000], pitch_cd:i16 ±9000, fov_cd:u16 3000..=11000} (centi-degrees, Eq+Hash for FrameKey); Pov{t:Micros (SOURCE time), cam}; CameraPaths(BTreeMap<MediaId,Vec<Pov>>) sorted unique t, <=200/source; at(&[Pov],t)->Camera (hold edges, linear in t, yaw shortest arc, tie 180 -> +); set/remove/edit (replace at same t); clamp(); drag(cam,dx_px,dy_px,view_w_px) deg = px*fov/view_w, picture follows pointer (yaw -= dx*k, pitch += dy*k); wheel(cam,scroll)/pinch -> fov*exp(-k*scroll) clamped; default(); command_lines(path,src_in,src_out,fps,step_cap)->String for sendcmd (numbers only, {:.4}); vfov(h_fov,W,H)=2atan(tan(h/2)*H/W).
  - media: MediaFormat + spherical:Option<bool> (#[serde(default)], None=unchecked -> bg re-check as subtitle_tracks); parse_probe: stream side_data_list "Spherical Mapping" projection=="equirectangular" && no bound_* && no "Stereo 3D" non-2D -> Some(true). project media entry + treat_as_360:Option<bool> (user override); fn is_360(&media_entry)=override.unwrap_or(spherical.unwrap_or(false)); not in history, sets dirty (like import). tag "360°" in ui/media.rs.
  - project: SCHEMA_VERSION 4 always; Project/ProjectDoc + camera:CameraPaths (#[serde(default)]); history Edit + camera (cap 100 unchanged).
  - subtitles: Line + size:Option<u16> permille 250..=4000 (#[serde(default,skip_serializing_if=None)]); ShownLine carries it; set_boxes(&[LineId],pos:Option<Option<Pos>>,size:Option<Option<u16>>) 1 undo; reset(&[LineId]) clears both (replaces/extends set_position(None)).
  - ass: Some(size) -> prefix `{\an5\pos(X,Y)\fs<round(base*size/1000)>\bord<scaled>\shad<scaled>}` (also each karaoke chunk); None -> byte-identical 1.2 (golden). base = ass::font_size(W,H,preset).
  - render: Segment + sphere:Option<Sphere{media,src_in,path:Vec<Pov>,fixed:Option<Camera>,cmd_file:String}>; graph for sphere segment: trim,setpts,fps,sendcmd=f=<cmd_file>,v360@cN=input=e:output=flat:reset_rot=1:w=W:h=H:d_fov=..:yaw=..:pitch=..:interp=linear (commands: START-END intervals of +-half frame, changed values only),setsar=1 (NO scale/pad: fills output frame); cmd files written by export/preview into private Work folder (cwd; relative names; escape per sendcmd rules); effective_dims(media)= (1920,1080) when 360 (REQ-022 auto format only); plan_export stream-copy/no-effects false when any 360 clip; no 360 clip -> argv byte-identical 1.2.5 (golden). FrameKey + camera at playhead (quantised ints) + subtitles tuple gets size; transient drag override gets (pos,size).
  - proxy: needs_proxy_360(w,h): w>4096 -> target 4096x2048 (scale keeps 2:1), independent of 720p rule; preview gate too_large_360(w,h): w>16384||h>8192 -> message, no decode (export unaffected).
  - ui/boxes.rs (NEW, pure): BoxItem{id:ObjectId::Line(LineId),rect:RectF(fractions),min_scale,max_scale}; handle_at(rect,p,tol 8px)->Option<Corner>|Body; cursor(hit); resize(rect0,corner,p,bounds)->(rect,factor) proportional, opposite corner fixed, factor=dot(p-fixed,d)/dot(d,d) clamp [min/size, max/size] and picture bounds; align_h/align_v(pos)->Pos; placement::boxes reused as producer. selection stays BTreeSet<LineId>.
  - settings: show_boxes:bool (#[serde(default=true)], file stays v2, 0.4.0 precedent); View > Show boxes (ToggleBoxes -> save). hidden = no outline/handles/resize, body drag still works.
  - app (new submodules src/app/camera.rs, src/app/boxes.rs; app.rs already 2641 lines): AppState.live:HashMap<MediaId,LiveView{cam,overriding:bool}> (window state); effective_camera(media,t)= overriding? live : paths nonempty? at(path,t) : live.cam or default. Commands: DragCamera{dx,dy}, ZoomCamera(f32), SetPov (playhead in 360 clip; stores effective camera; clears override), EditPov{media,old_t,pov}, RemovePov{media,t}, Set360(MediaId,Option<bool>), AlignSelected(Axis), ResetObjects, ToggleBoxes, ResizeLines{ids,size,pos_dragged}. Reset: selection nonempty -> lines (greyed "Already in the original place" if all original); else 360 clip at playhead -> live=default, overriding=false (greyed if already). overriding cleared by Seek/Play/Reset; Pause keeps. disabled_reason: "Select something on the picture first" (Align; Reset w/o selection and w/o 360). SetPov disabled: "Move the playhead onto a 360° clip".
  - Instance + Pov(MediaId, Micros); ui/markers.rs POV marker (diamond variant) for POVs inside the clip's kept source range, timeline x = clip.start+(t-src_in); popup: time, yaw, pitch, zoom (DragValue, 1 undo on commit), Remove.
  - actions: + AlignHorizontal, AlignVertical, ResetObjects, SetPov, ToggleBoxes (-> 43); toolbar = existing 12 + separator group [AlignHorizontal, AlignVertical, ResetObjects] placed after the 12 (overflow into "»" first); toolbar::fit must treat the separator as zero-cost, group kept together in overflow. shortcuts: SetPov bare `P`; Align H/V Ctrl/Cmd+Shift+H / +Shift+J, Reset Ctrl/Cmd+Shift+R (NOT Ctrl+Alt: AltGr collisions); PE may adjust, test asserts uniqueness. menus: Effects (Align x2, Reset, Set POV), Playback (Set POV), View (Show boxes); preview + marker context menus.
  - ui/mod.rs preview: 360 clip at playhead & no subtitle box under pointer: drag -> DragCamera, scroll/pinch -> ZoomCamera, Set POV button overlay in preview controls; camera drag debounce 30 ms (still mailbox) ; during playback with override: restart stream from playhead (latest-only, 100 ms) with fixed camera.
- 1.5.0 PLAN (not built): keyframes.rs (NEW, pure Key<T>, at(), limits); frame.rs (NEW, pure FrameAspect, frame_size, 4096 cap); timeline.rs + frame fields & rules; render.rs stages A-D + expression generator + re-encode rule; project.rs v7 validation notes; app/frame.rs (NEW submodule); ui/frame_popups.rs + keytimeline layout (NEW); preview FrameKey frame state. Details ai-docs/architecture-plan-1.5.0.md § Components and changes.
- main.rs: run_native (+ macOS: event_loop_builder default menu off, odoc observer before run_native; argv[1] open).
data flow:
- UI ev -> AppState::dispatch -> mutate Edit -> history -> recompute dirty -> FrameKey@playhead -> if != shown_key post still req to mailbox after 80 ms debounce (injected now).
- import: Command::Import(paths) -> placeholder Analysing -> probe pool (3 thr, Probe prio) -> mpsc -> tick() Ready/Rejected -> proxy rule may enqueue proxy job.
- export: render::plan_export(project, Source::Export) (originals) -> export thread, Export runner (1 ffmpeg) -> progress -> rename.
- subtitles: per timeline source w/ audio: extract 16 kHz mono f32 temp file (Background, 1 slot, released after extraction) -> STT thread (own thread budget, outside cap) -> 120 s chunks -> Transcript -> lines in source time. edited lines -> confirm first (REQ-044).
- open by double-click: Win/Linux argv[1] -> Command::OpenPath; macOS odoc -> mpsc -> tick -> OpenPath. both through REQ-028 prompt.
- startup (0.3.0): Machine::detect -> scratch::cleanup_stale -> settings::load_file (regular file, <= 64 KiB+1 read) -> settings::startup(bytes,&m) pure -> {settings, notice, write} -> save if write -> budget(effort,&m) -> sched.set_cap -> preview mode from Preview quality -> notice.
- setting change (0.3.0): SetEffort -> budget + set_cap (running jobs keep args); SetPreviewQuality -> preview rule, queue newly needed proxies, keep existing; SetSubtitleQuality -> save only, never regenerate. all saves write_atomic.
- export (0.4.0, ADR-039): app gives hw=Some(offer) only if Export=Faster && probe Done && method not in hardware_off for that format && codec h264|hevc; worker uses hw only if plan Reencode && codec has candidate && video_bitrate Some; else exact 0.3.0 path. hw run -> .<name>.hw.videolore-partial (registered; matches cleanup pattern), progress 0–0.85, watchdog -> exit 0 -> outcheck (0.85–1.0) -> rename, Done Hardware(v). fail -> kill (wait <=5 s then abandon), delete hw partial, cancel? Cancelled : sw redo into .<name>.videolore-partial w/ 0.3.0 args (progress back to 0 once, no message) -> Done SoftwareAfterProblem; redo fail -> Failed as 0.3.0, nothing remembered.
- probe (0.4.0, ADR-038): triggers = start-up after ffmpeg caps check if saved Faster | switch to Faster | "Try video hardware again" | "Use recommended settings" | ffmpeg override change while Faster. Best quality -> nothing runs. per codec, candidates in order, stop at first working.

- 1.1.0 layout: splitter drag -> UiCache.layout_drag live (no save) -> drag_stopped -> Command::SetLayout(Fractions) -> settings.layout + 1 atomic save. View > Reset layout -> ResetLayout (default + save). window resize: stored fractions kept, effective() applies mins at draw only. opening a project never touches settings.layout.
- 1.1.0 actions: menu/toolbar/context menu/keymap -> ActionId::command(&app) -> Command -> AppState::dispatch. keymap skipped when egui_wants_keyboard_input() || modal_open; matched events consumed before widgets. Esc: egui::Popup::is_any_open -> egui; else topmost of our windows by ctx.memory areas().order() -> ClosePopup; fallback most recently opened.
- 1.1.0 markers: timeline view -> markers::layout (pure, culled) -> paint; click -> hits -> 1: OpenInstance | >1: ChooseInstance -> chooser popup -> OpenInstance. Add-fade handle -> SetFade{on:true} (1 undo).
- 1.1.0 recent: successful open / Save as / first Save -> push_recent (dedupe, front, max 10) -> settings save. OpenRecent(p): missing -> remove + save + status msg; else guarded(Open(Some(p))) (REQ-028).
- 1.1.0 drop: i.raw.dropped_files (DroppedFileHandle::path()) -> DropFiles -> modal open? ignore + status : drop_plan -> Open(first project) via REQ-028 guard + status "N other files ignored" | Import(paths) same path as File > Import (REQ-002/032). Wayland: winit 0.30 emits no DroppedFile -> nothing arrives (ADR-047).

- 1.2.0 apply effect: Effects row click -> OpenPopup | double-click -> ApplyEffect{WholeVideo} | drag_started -> egui DragAndDrop payload EffectKind -> timeline video clip dnd_hover (highlight + CursorIcon::Copy; elsewhere NoDrop) -> dnd_release -> ApplyEffect{Clip}. context menu / Effects menu same command. Fade/Subtitles: 1 undo at apply; Volume: popup opens, undo on value change (Q2). popup of created/changed instance opens.
- 1.2.0 create subtitles (ADR-055): ApplyEffect{Subtitles,t} -> job running? greyed "Subtitles are being created" -> ranges (clip: (media,src_in,src_out); whole: every distinct source in full) -> any source tracks==None? pending_create + status "Checking the video for subtitles…" + probe (M17), resume in tick -> hand-edited lines overlap? REQ-044 confirm -> any usable track? Modal::SubtitleSource (Use/Recognise/Cancel, label list) -> Align needs model? consent modal + "Use without matching words" (-> Spread) -> stt::start(job) 1 thread, per range: Align/Spread: track_args -> read_limited 16 MiB -> srt::parse -> cues overlapping range clamped -> strip_tags; Align: range audio -ss/-t -> chunked whisper (lang from track unless overridden) -> align::align; recognition fail -> Spread + message; Recognise: today's path on range -> Done -> 1 undo "Create subtitles": replace_range per range | replace_all (whole-video recognise) + status unmatched message. Cancel/Failed before Done -> nothing, no undo.
- 1.2.0 media re-check: project open -> media with subtitle_tracks None -> background ffprobe (Probe prio) -> patch current project AND saved ProjectDoc (dirty unchanged).
- 1.2.0 preview drag: pointer press on subtitle box (placement::boxes, topmost) -> drag_pos updated (clamped centre) -> ghost painted + FrameKey changes -> latest-only still -> release -> SetLinePositions{ids,pos} 1 undo, drag_pos None. ids = selection if dragged in it and >1 else that line.
- 1.2.0 selection: marker/list click (+command mod -> Toggle) -> SelectLine; Shift(+Ctrl/Cmd) drag on tracks -> SelectRange{from,to,add} -> seek first selected; Esc: topmost popup else ClearSelection; empty timeline click clears.
- 1.2.0 SRT export: File > Export subtitles (.srt)… -> rfd save `<project name>.srt` -> ExportSrt(p) -> srt::write(subtitles.project(&timeline)) -> project::write_atomic (UI thread, like save).

- 1.4.0 deltas (PLANNED; supersede older lines above where they differ):
  - timeline: Clip.clean_audio; merge rule. render: Segment.clean -> afftdn chain; GraphSpec::AudioOnly; VIDEO_PRESETS in format; preview_audio_args same chain.
  - export/choice.rs (NEW pure): ExportChoice, name/extension/clash rules, plan mapping. ui/export_dialog.rs (NEW egui modal). ui/menu.rs: Effects vs Edit split. Set360 + Clip details switch REMOVED.
  - text: width permille (size removed), fit() best fit, Anchor width_cd/height_cd; placement/ui boxes: estimate boxes for text, centre clamp for flat text and lines, width handle.
  - project: v6 (migration from v5 in memory; treat_as_360 dropped).
  - app: Modal::Export, Modal::ConfirmOverwrite, export_memory (session), SetCleanAudio, ResizeTextWidth; old 360 export question modal removed.

## ADRs
ADR-001 GUI. eframe/egui latest (0.37 at 0.1.5). alt iced/slint/gtk4-rs/tauri. reason: pure Rust, single binary, MIT/Apache. [MSRV reasoning superseded by ADR-012]
ADR-002 media engine. ffmpeg/ffprobe CLI subprocess. alt ffmpeg-next/sys (link pain, ABI churn, static licence linkage), gstreamer-rs, pure-Rust decoders. reason: zero build deps, robust.
  rules (kept): Command no shell; `file:` prefix; `-protocol_whitelist file,pipe`; `-nostdin -hide_banner`, -y only on our temp output.
  ["not bundled" + "any ffmpeg >= 4.4 required" SUPERSEDED by ADR-013 in 0.2.0]. user override kept.
ADR-003 preview audio cpal; null sink fallback. [clock part superseded by ADR-016]
ADR-004 concurrency std::thread + mpsc + Mutex/Condvar; no tokio. cancel AtomicBool + kill. [process cap: ADR-015]
ADR-005 support crates serde/serde_json/thiserror/directories/rfd/tempfile. MIT/Apache.
ADR-006 export tiers. T1 stream copy only if same src format AND compatible output AND no effects AND keyframe cuts. 0.2.0: "effects" = fades, music clips, subtitle lines, video volume != 100% or muted (REQ-023 reworded). else T2 single re-encode. temp in dest dir + rename; never overwrite source. [input layout: ADR-017]
ADR-007 encoder mapping, sw first, `-encoders` probe at startup; unavailable formats disabled. [0.4.0: sw stays the mapping; hw only as opt-in attempt for H.264/HEVC re-encode, ADR-037]
ADR-008 fades min(0.5 s, clip/2), video+audio aligned.
ADR-009 STT engine whisper.cpp via whisper-rs; model ggml tiny-q5_1 multilingual, not bundled, download after consent. ["stt off by default" SUPERSEDED by ADR-022]. alt cloud APIs (privacy), vosk (quality), candle-whisper (perf/maturity).
ADR-010 music via ffmpeg amix/volume/afade in T2; no new crate. detailed in ADR-019.
ADR-011 CI: check job + Linux build: apt ffmpeg libasound2-dev; VIDEOLORE_REQUIRE_FFMPEG=1 in check job. 0.2.0 adds apt cmake libclang-dev espeak-ng; STT model cache; cargo-deny; notices freshness; package job (ADR-014). CI files owned by PE.
ADR-012 toolchain policy: latest stable; rust-version = current stable (1.100); fix new lints, never pin old toolchain/crates. CI latest + MSRV job.

ADR-013 (0.2.0) bundled ffmpeg. ctx: REQ-030 works w/o ffmpeg installed; C1 reversed. supersedes ADR-002 "not bundled"/">=4.4".
  choice: GPL builds, same FFmpeg release line on all OS (9.0.x at plan time; confirm at pin):
    win x86_64: BtbN/FFmpeg-Builds win64-gpl-shared (release branch; exes share DLLs ~half size of 2 static).
    linux x86_64: BtbN linux64-gpl static (release branch).
    macOS aarch64: Martin Riedl build server (ffmpeg.martin-riedl.de) release static arm64 ffmpeg+ffprobe, published SHA-256; incl x264 x265 libass freetype fontconfig harfbuzz vpx opus lame svt-av1.
  alt rejected: LGPL builds (no x264/x265 -> REQ-030 H.264/HEVC lost); build in CI (30+ min/OS, maintenance); evermeet (Intel only); Homebrew (dylibs).
  pinning: packaging/ffmpeg.lock per target: url, sha256, ffmpeg version, provider, licence, build-script src URL+commit, ffmpeg.org src tarball URL+sha256. scripts/fetch-ffmpeg.sh <target> <dest> curl + sha256sum / shasum -a 256, extract only ffmpeg/ffprobe/DLLs. BtbN keeps month-end builds 2 y -> pin month-end. Riedl retention unknown -> CI fails loudly, re-pin.
  smoke test on packaged copy: -version; -encoders has libx264 libx265 libvpx-vp9 libopus; -filters has subtitles amix alimiter. 0.4.0 adds: linux h264/hevc_{nvenc,qsv,vaapi}; windows h264/hevc_{nvenc,qsv,amf}; macOS h264/hevc_videotoolbox.
  location: always next to main exe. win: install dir (+ *.dll), fonts/ notices/ beside. macOS: Contents/MacOS/ (bundle seal covers nested code), resources Contents/Resources/. AppImage: usr/bin/, usr/share/videolore/. tar.gz: top folder next to videolore, fonts/ notices/.
  lookup: settings > env VIDEOLORE_FFMPEG/FFPROBE > bundled_dirs(exe_dir)=[exe_dir, exe_dir/../Resources] > PATH. resources: exe_dir/{fonts,notices}, exe_dir/../Resources/..., exe_dir/../share/videolore/... "bundled damaged" (present but fails -version/caps) vs "not installed" messages distinct.
  new feature checks: subtitles amix alimiter volume split asplit.
  licence: app GPL-3.0-only, ffmpeg run as separate program (aggregation) but we distribute GPL binaries -> provide source. BtbN gpl = --enable-gpl --enable-version3 -> GPLv3+; x264/x265 GPLv2+; all compat. release asset videolore-<ver>-third-party-source.tar.xz: ffmpeg.org tarball+.asc, git archive of provider build scripts at pinned commit, Noto Sans source ref + OFL, README. notices: written offer 3 y (GPLv3 §6), H.264/HEVC patent note. ASSUMPTION: no lawyer review.
  fonts: libass in all 3 builds; bundle Noto Sans Bold (OFL-1.1, ~0.5 MB) at assets/fonts/ + OFL text + source url/sha. Latin/Greek/Cyrillic only (Andrea). others -> libass system fallback (may differ per OS/boxes); egui default fonts cannot show them in editor. documented limitation.
  size est 70–130 MB per installer (measure in CI, not gate).
  tests: CI keeps apt ffmpeg (Ubuntu build has x264/x265/libass) -> hvc1 + burn-in integration tests run; package job smoke-tests bundled copy per OS.

ADR-014 (0.2.0) installers/packaging. REQ-040/041/042. unsigned (Andrea; signing deferred).
  win x86_64: NSIS videolore-<ver>-windows-x86_64-setup.exe via cargo-packager 0.11.8 (Apache-2.0 OR MIT; last release 2025-11-27; CI-only tool, `cargo install --locked --version 0.11.8`). install_mode=currentUser (no admin, HKCU), start-menu shortcut, Apps uninstall entry, .vlproj assoc HKCU\Software\Classes via file_associations, licence page = notices. uninstaller removes install dir + our HKCU keys only, never projects.
  macOS aarch64: videolore-<ver>-macos-aarch64.dmg via cargo-packager (drag to Applications); file_associations -> CFBundleDocumentTypes + UTExportedTypeDeclarations; then `codesign --force --deep --sign -` ad-hoc reseal (nested ffmpeg consistent). no Gatekeeper signing.
  linux AppImage: videolore-<ver>-linux-x86_64.AppImage via appimagetool (AppImage/appimagetool release pinned URL+SHA-256 in packaging/tools.lock, PE picks current) on hand-built AppDir: AppRun, videolore.desktop (MimeType=application/x-videolore-project;), videolore.png, usr/bin/{videolore,ffmpeg,ffprobe}, usr/share/videolore/{fonts,notices}. static type2 runtime -> needs fusermount3 (current Ubuntu LTS + Fedora), not libfuse2. cargo-packager NOT used for AppImage (linuxdeploy, unspecified runtime).
  linux tar.gz (Andrea: also tar.gz): top folder videolore-<ver>-linux-x86_64/: videolore 0755, ffmpeg/ffprobe 0755, fonts/, notices/, videolore.png, README.txt (extract, double-click), marker .videolore-bundle. risk: some file managers don't run ELF on double-click -> README fallback (right-click Run / ./videolore once).
  linux self-integration (desktop::linux, startup, idempotent): only if $APPIMAGE set (abs existing file -> Exec target $APPIMAGE) else exe_dir/.videolore-bundle exists (target canonical current_exe()); else nothing (source/cargo run). writes ~/.local/share/applications/videolore.desktop (Exec="<target>" %f quoted per desktop-entry spec, MimeType), ~/.local/share/icons/hicolor/256x256/apps/videolore.png, ~/.local/share/mime/packages/videolore.xml (glob *.vlproj). best-effort update-mime-database + update-desktop-database via Command (abs args, no shell, errors ignored). rewrite when target moved. About (Linux): "Remove from application menu" deletes the 3 files.
  icon: original SVG assets/icon/videolore.svg; committed derived icon.png(1024) .ico .icns icon-256.rgba (include_bytes! -> ViewportBuilder::with_icon; no image crate). scripts/icons.sh dev-only. no Windows exe resource icon.
  double-click .vlproj (REQ-041 reworded): win/linux argv[1] -> OpenPath at startup; running app -> OS starts 2nd window (Andrea), no IPC, 1st untouched. macOS: Apple Event odoc (winit 0.30 doesn't expose) -> desktop::macos NSAppleEventManager kAEOpenDocuments handler via objc2 0.6.5 + objc2-foundation 0.3.2 (MIT, min features), installed from NSApplicationWillFinishLaunchingNotification observer registered in main before run_native -> mpsc -> tick -> OpenPath -> REQ-028 prompt.
  CI package job (PE): matrix 3 OS, needs [check, msrv], on v* tags + workflow_dispatch: fetch ffmpeg + smoke, cargo build --release --locked, scripts/package.sh <target>, upload, tag -> softprops/action-gh-release@v3. 5 assets: setup.exe, dmg, AppImage, tar.gz, third-party-source.tar.xz (from linux leg or small notices job). existing build job: bare exes on push/PR as artifacts, no longer attached to releases.
  alt: hand NSIS script + hdiutil (fallback if cargo-packager breaks); linuxdeploy; flatpak/snap/deb (out of scope).

ADR-015 (0.2.0) process scheduler. REQ-033. jobs::Scheduler{cap:3} Mutex<State>+Condvar. [0.3.0: cap per effort level via set_cap, 3..=6, ADR-029]
  Priority{Preview=0, Export=1, Probe=2, Background=3}. acquire(prio, slots:u8, cancel:&AtomicBool, preempt:Option<Arc<AtomicBool>>) -> Option<Permit>: grant when in_use+slots<=cap and no earlier-ranked waiter (strict prio, FIFO within level); None on cancel. Permit drop -> release + notify_all.
  preemption: Preview/Export waiter unsatisfiable -> set preempt flag of lowest-prio running preemptible permit (proxy only) -> kill ffmpeg, drop permit, requeue, restart from 0. probes + STT extraction short, not preempted. export never.
  structural: Scheduler::runner(prio)->Arc<dyn Runner> = LimitedRunner: run() holds 1 slot for call; spawn() -> Process wrapper keeps permit until wait/kill/Drop; spawn_group(&[(prog,args);2]) 2 slots atomically. all callers get such runner -> cap unbypassable.
  slots: playback 2 (video+audio together; single NUT-muxed proc rejected: demuxer complexity); still 1; export 1 (keyframe probes sequential at Export prio before); probe 1 each (pool 3 thr); proxy 1 (1 worker, preemptible); STT extraction 1 (Background). worst: export+playback=3.
  obsolete: still frames latest-only mailbox Mutex<Option<Req>>+generation; newer replaces pending, running older gen killed. playback restart kills old pair. media removal cancels its probe/proxy. New/Open/close cancels all project jobs.
  startup ffmpeg capability check on bg thread; UI "checking ffmpeg…", import/preview/export disabled until done.
  STT inference NOT in cap (only ffmpeg/ffprobe per PM default); own thread budget.

ADR-016 (0.2.0) preview. REQ-034/035. supersedes ADR-003 "audio clock master" + risk "audio drives clock".
  FrameKey (pure, render) at t: segment media + effective path (proxy|orig), source frame idx (fps-snapped), fade params at t, output geometry, visible subtitle cues (text, preset, highlighted word idx). render only if key != shown_key after 80 ms debounce (injected now). edit after playhead -> 0 renders; 10-edit burst -> <=1.
  still graph: single-segment at playhead (1 input).
  clock: wall clock master (Instant), anchored play/seek, pause excluded. audio callback tracks device_pos (counts silence written when starved). pure audio_correction(device_pos, wall_pos)->{None,Drop(n),Pad(n)} threshold 40 ms. decoder stall -> silence then drop backlog; device stall -> corrected on resume. video never waits for audio.
  music: preview audio proc uses same render::audio_graph as export, from `from` w/ +skip trick. subtitles: same ASS+font in preview private tempdir = cwd; subtitles filter after scale; ASS PlayResX/Y = export geometry -> same look. proxies via render::segments(project, from, Source::Preview{proxies_on}) proxy if Ready else orig.
  alt rejected: audio master (video stalls with audio, the 0.1.3 limitation).

ADR-017 (0.2.0) export inputs grouped in monotone runs. REQ-037. supersedes "one input per clip".
  consecutive segments same source with src_in(i+1) >= src_out(i) share one input: -ss <run_start> -t <run_len+1s> -i file:<p>; [k:v:0]split=n / [k:a:0]asplit=n; per seg trim/atrim, setpts, existing normalisation. new input only when order goes back in time -> ffmpeg memory bounded (no branch queues ahead). 200 in-order segs from 1 src = 1 input/decoder. graph via script file when > 8 KiB (existing). music clips grouped same way.
  alt rejected: one input per clip (200 decoders); one input per source w/ split for reordered clips (unbounded frame queues).
  hvc1 (REQ-036): -tag:v hvc1 whenever video codec HEVC and muxer mp4|mov, copy and re-encode.

ADR-018 (0.2.0) proxies. REQ-038/039.
  rule pure: needs_proxy(w,h,MachineInfo{cores,ram_bytes}) = w*h > 1920*1080 || (w*h > 1280*720 && (cores < 8 || ram < 8 GiB)). display size (rotation). audio-only never. cores=available_parallelism; RAM=sysinfo (system feature) once at startup.
  format: libx264 -preset veryfast -crf 23 -g round(fps/2), fit 960x540 even dims, -fps_mode passthrough, AAC 128k stereo, .mkv timestamps from 0. <key>.mkv.partial -> rename. -progress pipe:1 -> media item.
  cache: ProjectDirs::cache_dir()/proxies/<key>.mkv; key = first 32 hex of SHA-256(abs path, size, mtime ns, PROXY_PARAMS_VERSION) (sha2 -> runtime dep). reused across sessions, recreated if missing, size in settings. clear deletes only ^[0-9a-f]{32}\.mkv(\.partial)?$ not being generated. no auto eviction. on by default; switch in settings (not project). export never uses proxies: plan_export identical on/off. [0.3.0: switch SUPERSEDED by Preview quality Automatic/Smoother/Original, ADR-029; cache moves under Settings "Preview copies"]
  memory caps (REQ-039, nothing scales w/ file size): preview frame chan 8x640x360 RGBA (~7 MB); audio buf 1 s; stderr 64 KiB; run() stdout 16 MiB; STT chunk 120 s x 16 kHz f32 (~7.7 MB); keyframe output parsed line by line.

ADR-019 (0.2.0) music mix. details ADR-010. REQ-019/020/043.
  bed = concat of anullsrc gaps + music segs (atrim, asetpts, aresample=48000, aformat=fltp:stereo), atrim=duration=<video_len>. clip past video end -> afade=t=out:st=<len-0.5>:d=0.5. no music where no clips. [va]volume=gv, [mus]volume=gm, amix=inputs=2:duration=first:normalize=0, alimiter=limit=0.97. gain=(p/100)^2 (50% ≈ -12 dB), mute -> 0. music doesn't ripple with video edits.
  UI: music lane under video track inside existing timeline panel, proportions unchanged (REQ-024); sliders record undo on drag stop.

ADR-020 (0.2.0) subtitles model + burn-in. REQ-016/017/018.
  lines stored in source time; shown via projection: for each video clip in order, lines of that media intersecting [src_in,src_out) clipped and mapped clip_start+(s-src_in); words outside hidden. cut removes, copy dupes, split keeps. re-time converts through the shown clip, clamped. add-at-playhead uses clip under playhead (2 s default, clamped). re-text of line shown twice changes both. text edit clears words -> highlight preset spreads evenly by word count.
  burn-in: ass::write(shown, preset, geometry) -> subs.ass + NotoSans-Bold.ttf in fonts/ in private tempfile dir; ffmpeg cwd = that dir; filter `subtitles=f=subs.ass:fontsdir=fonts` after [outv]. relative names avoid filtergraph path escaping (win C:\, quotes).
  size = min(0.055*H, 0.09*W); margins scale; wrap per preset. presets: Classic (white, black outline, bottom); Box (white on semi-opaque box BorderStyle=3, lower third); Word highlight (bold white, current word yellow via {\kNN} SecondaryColour->PrimaryColour, centre-low, <= ~4 words/chunk).
  escape: { -> (, } -> ), \ -> U+FF3C; only our code emits \N \k; text cap 1000 chars.

ADR-021 (0.2.0) STT pipeline. REQ-014/015/044.
  whisper-rs 0.16.0 (Unlicense; whisper-rs-sys 0.15.0 rust-version 1.88; build deps cmake 0.1 + bindgen 0.72 -> cmake, C++ compiler, libclang). whisper.cpp MIT. CPU only, no metal/cuda.
  model ggml-tiny-q5_1.bin multilingual MIT: https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny-q5_1.bin size 32152673 sha256 818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7 (verified HF API 2026-11-14). stored ProjectDirs::data_dir()/models/; .part + exact size + sha -> rename. consent dialog (32 MB, MIT, source, "only network access") before any network call. local file option: same sha+size, copied into models dir. choosing another model out of scope.
  http: ureq 3.5.2 (MIT/Apache, rust-version 1.85) optional under stt, default-features=false features=[rustls] -> rustls 0.23 + ring 0.17 (Apache-2.0 AND ISC) + webpki-roots 1.0 (CDLA-Permissive-2.0). https only, redirects https only, connect+read timeouts, HTTPS_PROXY honoured. only downloader module uses network.
  traits: Transcriber{transcribe(&self, pcm:&[f32], offset:Micros, lang:Option<&str>, cancel:&AtomicBool)->Result<Transcript,SttError>}, ModelFetcher; fakes for orchestration tests; real glue (~100 lines) covered by integration test gated VIDEOLORE_STT_MODEL (CI: cached model by sha, espeak-ng "hello world, this is videolore", assert "hello" + increasing word ts).
  word ts: set_token_timestamps(true), set_split_on_word(true), set_max_len(1); group words into lines at pauses > 0.6 s or preset chunk limit.
  language: auto on first chunk (set_language(Some("auto")) -> detected id), stored in project; override re-runs. one per project.
  threads clamp(cores/2,1,4) [0.3.0 SUPERSEDED: budget.stt_threads ADR-029]; one STT job at a time; outside ffmpeg cap; extraction = 1 Background slot. chunks 120 s from extracted temp file (boundary artefacts accepted). cancel via abort callback AtomicBool; progress = chunk/total.
  scope: each timeline source transcribed in full (trim extend reveals subs); video audio only, never music; regenerate replaces all lines, confirm if any edited (REQ-044).
  alt rejected: cloud APIs; vosk; candle-whisper; reqwest (tokio, heavier) vs ureq; native-tls (OpenSSL on linux) vs rustls.

ADR-022 (0.2.0) cargo features: default = ["audio","stt"]; stt = ["dep:whisper-rs","dep:ureq"]. supersedes ADR-009 "stt off by default". release builds include STT. source builds need cmake + C++ + libclang; --no-default-features = minimal build: "Generate subtitles" shows "not available in this build"; edit/presets/burn-in still work. CI clippy/test default, --no-default-features, --all-features.
  CI per OS: ubuntu apt cmake libclang-dev espeak-ng (+ffmpeg libasound2-dev pkg-config); macOS Xcode clang + cmake; windows MSVC + cmake + LLVM from image, LIBCLANG_PATH=C:\Program Files\LLVM\bin if needed. rust-cache keeps whisper.cpp build.

ADR-023 (0.2.0) project schema v2 + dirty. REQ-026..029.
  v2 adds music{clips[{id,media,src_in,src_out,start}],next_id}, mix{video_volume 0..=100, video_muted, music_volume 0..=100, music_muted}, subtitles{language?, language_overridden, preset, lines[{id,media,start,end,text,edited,words[{start,end,text}]}], next_id}.
  load: v1 -> migrate in memory (empty music, mix 100/unmuted, no subs; export identical). >2 -> ProjectError::TooNew(v), clear message, never written on load. validate: music refs audio-only, no overlap, in range; sub lines ref existing media, start<end, text <= 1000 chars, words inside line, <= 200k words total; 16 MiB cap kept.
  save always v2 -> v1 file saved in 0.2.0 unreadable by 0.1.x (changelog note). [1.2.0: save always v3, loads v1/v2/v3, >3 TooNew; ADR-051]
  dirty: saved:Option<ProjectDoc>; ProjectDoc = persisted model (media w/o transient status/proxy, + Edit, abs paths). dirty = current_doc() != saved, recomputed after mutation/undo/redo/import/removal (not per frame). undo to saved clears. Analysing media excluded until Ready.
  title "<name> •" when dirty, "Untitled" before first save, e.g. "clip.vlproj • — videolore".
  Save = write project_path or Save As. PendingAction{New, Open(Option<PathBuf>), Close, Quit}; modal Save/Discard/Cancel only when dirty; failed/cancelled save stops action.
  close (eframe 0.37): ui(): close_requested && dirty && !allow_close -> ViewportCommand::CancelClose + modal(Close); after Discard/successful Save -> allow_close + ViewportCommand::Close.
  alt rejected: dirty counter/flag (undo back to saved wouldn't clear); hash compare (no gain over struct eq at this size).

ADR-024 (0.2.0) quit + macOS menu. Command::Quit from Cmd+Q (macOS) / Ctrl+Q. macOS: NativeOptions::event_loop_builder + EventLoopBuilderExtMacOS::with_default_menu(false) (winit via eframe re-export, no new direct dep) so Cmd+Q reaches keymap -> REQ-028 prompt. loss: standard Hide/Minimize menu items (accepted).

ADR-025 (0.2.0) supply chain. cargo-deny 0.20.2 in check job, deny.toml: licences allow MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unlicense, CDLA-Permissive-2.0, BSL-1.0, MPL-2.0; advisories; bans. cargo-about 0.9.3 -> packaging/notices/rust-crates.txt (plain-text template), committed, CI freshness = regenerate + diff. ffmpeg archives + appimagetool pinned SHA-256. Cargo.lock committed, --locked. closes ADR-011 "cargo-deny not done".

ADR-026 (0.2.0) notices/About (REQ-031). notices module: ffmpeg (version read at runtime), x264/x265/libass/other libs + licences, Noto Sans OFL-1.1, whisper.cpp MIT, model MIT, rust crates (generated), source links + written offer, patent note. About reachable from command bar; also installer licence page (win).

ADR-027 (0.2.1) whisper.cpp CPU baseline (decided by Andrea). ctx: whisper-rs-sys -> cmake ggml GGML_NATIVE default ON (not cross, no SOURCE_DATE_EPOCH) -> -march=native of GH runner (may have AVX-512) -> release exe SIGILL on user CPUs lacking runner ISA.
  choice: .cargo/config.toml [env] GGML_NATIVE = "OFF". whisper-rs-sys 0.15.0 build.rs forwards every GGML_*/WHISPER_*/CMAKE_* env var as cmake -D (verified source 2026-11-14). ggml CMakeLists (vendored): NATIVE OFF + NATIVE_DEFAULT ON -> INS_ENB ON -> SSE4.2, AVX, AVX2, BMI2, FMA, F16C ON (MSVC: /arch:AVX2, FMA/F16C implied); AVX512*, AVX-VNNI, AMX OFF (verified source).
  result: x86-64 STT baseline = AVX2+FMA+F16C+BMI2 (Intel Haswell 2013+, AMD Excavator 2015+/Zen; ~2013+). macOS aarch64: NEON/ARMv8 baseline as before (unaffected). applies to source builds too (config.toml in repo); user may override GGML_NATIVE=ON for a local native build.
  pre-AVX2 x86-64: ASSUMPTION (not verified on hw): STT path (whisper/ggml code) hits illegal instruction -> process crash (SIGILL / STATUS_ILLEGAL_INSTRUCTION) when generating subtitles; rest of app unaffected because ggml code only runs inside STT (ASSUMPTION: compiler does not hoist AVX2 into code run at startup; ggml objects only reached via whisper calls). no runtime CPU check in 0.2.1 (possible follow-up: is_x86_feature_detected!("avx2"/"fma"/"f16c") gate -> "not supported on this CPU" message instead of crash; or GGML_CPU_ALL_VARIANTS + GGML_BACKEND_DL dynamic dispatch).
  CI: rust-cache key bumped so cached native-built whisper-rs-sys artefacts are not reused (build.rs has no rerun-if-env-changed on GGML_NATIVE).
  alt rejected: native (SIGILL on users); SSE-only (GGML_AVX*/FMA/F16C OFF -> ~7x slower STT, per Andrea's measurement/estimate); runtime dispatch GGML_CPU_ALL_VARIANTS (needs GGML_BACKEND_DL = shipping loadable backend libs; not supported by whisper-rs-sys static build; more packaging) -> deferred.
  risk: setting SOURCE_DATE_EPOCH in build env flips NATIVE_DEFAULT OFF -> INS_ENB OFF -> silent SSE-only build (slow); keep it unset or set GGML_AVX2/FMA/F16C/BMI2 ON explicitly if ever needed. future whisper-rs-sys bump: re-verify env forwarding + ggml defaults.
  0.3.0: "no runtime CPU check" SUPERSEDED by ADR-031 (gate + message).

ADR-028 (0.3.0) machine detection + settings v2 + startup decision. REQ-045..049, 057. A1 defaulted (still pending). in-app window CONFIRMED by Andrea 2026-11-14. 0.4.0: settings v2 gains additive fields export + hardware_off (ADR-040).
  ctx: auto-setup on first start on every OS; zero technical knowledge; Settings plain words.
  detection (machine.rs): logical=std available_parallelism (cgroup/affinity aware); physical=sysinfo physical_core_count, unknown -> max(1,ceil(L/2)), clamp <= L; ram=sysinfo total_memory (0 unknown); os/arch std::env::consts; CpuFeatures{sse42,avx,avx2,fma,f16c,bmi2} via is_x86_feature_detected! (checks OS AVX support); aarch64 all true. speech_supported = aarch64 || all six (ADR-027 baseline). fingerprint{os,arch,logical,physical,ram_gib=(ram+GIB/2)/GIB nearest, speech_supported}. no timing, no network -> deterministic, ms (<= 3 s guaranteed; test detect()<3 s + equal fingerprints twice).
  where: app first start on every OS incl AppImage (A1); installers do no tuning. alt rejected: Windows installer runs detection; installer writes flag (two code paths, untestable).
  settings.json v2 (config dir): {version:2, preview_quality: automatic|smoother|original, subtitle_quality: fast|balanced|most_accurate, effort: light|balanced|maximum, ffmpeg, ffprobe (abs or null), machine:{fingerprint}}. serde rename_all snake_case, unknown fields allowed (fwd compat), any parse error = malformed. validate logical/physical 1..=4096, ram_gib <= 1048576. "recommended in use" = 3 choices == recommend(current machine); no stored flag.
  startup() pure table:
    no file -> recommend(m)+fp, notice Setup, write.
    legacy (no "version", has proxies_enabled/ffmpeg/ffprobe) -> recommended, but Preview=Original if proxies_enabled==false; override kept; models untouched; Setup; write. (REQ-049)
    v2 same fp -> as is, no notice, no write.
    v2 diff fp, choices == recommend(saved fp) -> recommend(new), new fp, Setup, write. (REQ-048)
    v2 diff fp, custom -> keep choices, new fp, MachineChanged notice w/ "Use recommended settings" action, write. (A9)
    malformed (bad JSON, unknown enum, out of range, > 64 KiB, not regular file) -> recommended, Malformed notice, write; bad file copied to settings.json.bad first (overwrite).
    version > 2 -> recommended in memory only, no notice, no write until user changes something.
    override not absolute / not existing regular file -> dropped + Malformed notice.
  recommend(m) pure: preview Automatic; effort Balanced always; subtitles: Fast if !speech_supported, else MostAccurate if L>=12 && R>=15 GiB, else Balanced if L>=4 && R>=7 GiB, else Fast. thresholds in reported GiB (8 GB box ~7.6 GiB). availability: all levels greyed if !speech_supported ("This computer's processor is too old for automatic subtitles."); MostAccurate greyed R<7 GiB, Balanced R<3.5 GiB ("Needs more memory than this computer has."); recommend never picks greyed.
  UI: in-app egui::Window (A7) from command bar "Settings"; main: Preview quality / Subtitle quality / How hard videolore works + one-line explanations, status line, storage lines ("Preview copies" + Clear, "Speech recognition files" + "Remove downloaded speech files"); collapsed Advanced: ffmpeg override (moved from effects panel), bundled component paths/versions/origin, speech engine, processor support. proxy switch removed from ui/effects. main strings only from settings::main_texts() -> banned-word test (thread core proxy codec encoder ffmpeg AVX CPU GPU model quantised; case-insensitive whole words).
  notices: Notice{text, action:Option<NoticeAction>, until:Option<Instant>}; Setup self-closes after 8 s (injected now); MachineChanged w/ action; Malformed.
  "Use recommended settings" (REQ-047): re-detect, set 3 choices = recommend(m), summary(old,new) -> "Subtitle quality: Fast → Balanced" lines or "Already using the recommended settings"; never touches override, projects, proxy cache, models; never fetches.
  alt rejected: toml/ron (serde_json already dep, same format); OS multi-viewport window (Wayland inconsistency, not headless-testable); raw-cpuid/cpufeatures (std suffices); num_cpus / raw /proc,sysctl,Win32 (sysinfo already dep).

ADR-029 (0.3.0) effort levels -> resource budget. REQ-050, REQ-033 reworded, QG-01. A5 + SEVERE defaulted semantics; Light = per-job limit CONFIRMED by Andrea 2026-11-14. 0.4.0: Light -threads still applies to sw filter work during hw export + to outcheck decode.
  budget(effort,&m) pure, deterministic. L logical, P physical (<= L), R reported RAM.
    max_processes: Light 3 | Balanced 3 | Maximum clamp(L/2,3,6), 3 if R<7 GiB.
    ffmpeg threads (export + proxy): Light Some(max(1,L/2)) | Balanced None | Maximum None (None = ffmpeg auto = 0.2.2).
    stt_threads: Light clamp(min(P,max(1,L/2)),1,8) | Balanced clamp(min(P,max(1,L-1)),1,8) | Maximum clamp(P,1,16).
    proxy workers 1 all levels; preview, ffprobe, audio extraction never limited (ffmpeg auto).
  cap floor 3: playback 2 + export 1 (ADR-015). MAX_CAP 6; probe pool 6 threads, scheduler limits.
  Some(t): export adds -threads t before each -i, -filter_complex_threads t, output -threads t; proxy -threads t -filter_threads t.
  set_cap: lowering never kills; new grants wait until in_use+slots<=cap. level change applies to next job; running jobs keep args.
  worked (P=L/2 w/ HT): L1P1 3/1/1|3/1|3/1; L2P1 3/1/1|3/1|3/1; L4P2 3/2/2|3/2|3/2; L8P4 3/4/4|3/4|4/4 (3 if R<7); L8P8 3/4/4|3/7|4/8; L16P8 3/8/8|3/8|6/8; L32P16 3/16/8|3/8|6/16. (Light cap/threads/stt | Balanced cap/stt | Maximum cap/stt) = expected test output.
  guarantees: every value >=1 and <=L; Light threads + stt <= max(1,L/2); Balanced stt >= 0.2.2 clamp(L/2,1,4) (no regression). Balanced/Maximum: ffmpeg internal auto threading (x264 ~1.5xL) outside guarantee, documented. Light = per job limit, two jobs together may use all cores (SEVERE defaulted).
  output: effort may change export bytes (x264 thread-dependent) but not look/length; Preview quality never changes export (REQ-051).
  preview quality (REQ-051): Automatic = needs_proxy unchanged (ADR-018, A6); Smoother = w*h > 1280*720; Original = never. plan_export identical for all.
  alt rejected: strict total thread budget split across jobs (slower Balanced exports -> fails QG-01, complex); OS niceness (unsafe setpriority/libc); cgroups (Linux only).

ADR-030 (0.3.0) subtitle quality levels = 3 whisper models. REQ-052/053, REQ-014 reworded. A2 SEVERE defaulted; models + recommendation thresholds CONFIRMED by Andrea 2026-11-14 (numbers still adjustable after measurement, A10). extends ADR-009/021 (single model superseded).
  all huggingface.co/ggerganov/whisper.cpp, MIT, URL https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/<file>; size+sha verified HF tree API 2026-11-14:
    Fast ggml-tiny-q5_1.bin 32152673 818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7 (unchanged)
    Balanced ggml-small-q5_1.bin 190085487 ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb
    MostAccurate ggml-large-v3-turbo-q5_0.bin 574041195 394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
  ESTIMATES (ASSUMPTION, PE measures): vs tiny small 6–8x slower, turbo 20–30x; 8 AVX2 thr tiny ~30x RT, small ~5x, turbo ~1–2x; peak mem small ~0.5 GB, turbo ~1.3 GB.
  rejected: base-q5_1 60 MB (small gain); medium-q5_0 539 MB (slower + less accurate than turbo); q8_0 (+~40% size, little gain); f16 (3–6x size); .en (no multilingual). greedy best_of 1 all levels (beam 2–3x slower).
  code: ModelSpec table replaces constants; ModelStore per spec, downloaded_bytes() (sum of known files present), remove_downloaded() (only known names + .part). flash attention on (WhisperContextParameters::flash_attn; no DTW) + one WhisperState per job (not per 120 s chunk); PE keeps flash attn for Fast only if measured tiny time not worse. stt threads from budget (ADR-029).
  consent modal per level: level name, size MB, MIT, source. options: Download | "Use Fast instead (no download)" only if tiny installed, this run only, setting unchanged | "Download the Fast one instead (32 MB)" only if nothing installed | "Choose a file on this computer…" must match level size+sha | Cancel (nothing changes). declining changes nothing; changing level never regenerates; REQ-044 confirm when regenerating at other level.
  integrity: full sha at install; before each load size check + sha once per session (memo by path,size,mtime). download limit spec.size+1. disk full -> delete .part, "Not enough free disk space".
  storage (REQ-053): sum of known files; remove after confirmation; subtitles in projects unaffected; re-download needs consent.
  accuracy test: higher_levels_are_at_least_as_accurate (integration, feature stt, gated VIDEOLORE_STT_MODELS_DIR; CI cache keyed by 3 sha): espeak-ng sentence w/o brand names, word-level Levenshtein WER after lowercase + strip punct; WER(small)<=WER(tiny), WER(turbo)<=WER(tiny).
  engine unchanged: whisper-rs 0.16.0 (latest; abort-callback workaround stays). alt candle-whisper, vosk, cloud rejected (ADR-021).

ADR-031 (0.3.0) older processors: CPU gate + message, no fallback engine. REQ-054. A4 SEVERE defaulted; CONFIRMED by Andrea 2026-11-14: message only, SSE-only helper DROPPED (out of scope, reconsider only on request; not a 0.4 candidate). supersedes ADR-027 "no runtime check".
  infeasible fallback in 0.3.0: whisper-rs-sys links one static ggml -> second ISA variant = symbol clash; GGML_CPU_ALL_VARIANTS needs GGML_BACKEND_DL (unsupported by whisper-rs-sys). only route = separate SSE-only helper exe + IPC + packaging (~7x slower) ["0.4 candidate" withdrawn: dropped 2026-11-14].
  0.3.0: !speech_supported -> Generate shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand."; no download offered; 3 levels greyed w/ same reason; ggml never reached. test: inject CpuFeatures{avx2:false,..} -> no transcriber call, message, recommend Fast, all unavailable.
  optional CI (M7): lib test under qemu-x86_64 -cpu Westmere on gate test (proves startup incl ggml static init doesn't SIGILL). ASSUMPTION qemu user-mode refuses AVX2 for that model; PE verifies with one-line AVX2 probe first, else skip + manual checklist.

ADR-032 (0.3.0) hardware export encoding deferred. REQ-055 stays (planned). A3 SEVERE defaulted. [SUPERSEDED 0.4.0 by ADR-037..040: Andrea 2026-11-14 build it now; fallback alone was insufficient -> added stall watchdog + output check + failure memory; probe result session-only, not stored in fingerprint; rate control = VBR at sw bitrate, not cq/global_quality]
  feasible in principle (BtbN gpl: nvenc, amf, qsv/libvpl, vaapi on linux; macOS VideoToolbox) but: no GPU on GH runners -> only fallback testable; per-vendor rate control tuning (nvenc -cq, qsv -global_quality, amf -qp_*, vaapi hwupload+device, vt -q:v); driver bugs -> bad frames w/ exit 0 not caught by fallback (risk REQ-022/023); shares 3 s detection budget; does not fit session.
  future design: bg probe (3-frame lavfi encode per candidate, 1 s timeout each, total <= 3 s, result stored in fingerprint), hw-first export, sw re-run on non-zero exit. "Export: Faster / Best quality" choice only then.
  alt: all vendors w/ fallback; VideoToolbox only (still unverifiable in CI).

ADR-033 (0.3.0) private work folder + crash cleanup. REQ-059.
  scratch::Work::new(kind) -> <cache_dir>/videolore/work/<rand>/ mode 0700 + `lock` file (std File::try_lock, stable 1.89, exclusive for job lifetime) + optional `targets` file listing partial paths outside the folder.
  contents: extracted audio, ASS, font copy, filter scripts. ffmpeg cwd = work folder.
  export partial: .<name>.videolore-partial in dest dir, create_new + 0600, registered in targets; ffmpeg -y truncates but keeps mode; success -> 0644 + rename; fail/cancel -> remove. proxy <key>.mkv.partial and model <file>.part registered same way.
  cleanup_stale() at start: each work folder whose lock try_lock succeeds (= stale) -> delete each target whose name matches ^\..+\.videolore-partial$ | ^[0-9a-f]{32}\.mkv\.partial$ | <known model>.part AND is regular file AND not symlink; then delete folder. held lock (other window) -> untouched. windows in-use file -> skip.
  cache + data dirs created 0700 on Unix.
  alt rejected: system temp (/tmp often RAM-backed, long audio fills RAM; shared); sysinfo PID liveness (PID reuse); age-based deletion (races w/ long jobs).

ADR-034 (0.3.0) security hardening. REQ-057/058, QG-02. findings from code review 2026-11-14.
  unsafe: only whisper abort callback (reviewed ok, 'f lifetime binding) + desktop::macos objc2. lib.rs #![deny(unsafe_code)], #[allow(unsafe_code)] on stt::whisper + desktop::macos only.
  BUG project::load: metadata().len() then read_to_string -> FIFO/device .vlproj blocks UI forever; growing file beats 16 MiB cap. fix: open, file.metadata()?.is_file(), read take(MAX+1), refuse over. same in settings::load_file (64 KiB). serde_json recursion limit 128 -> deep nesting = error (test > 200 levels).
  media paths from projects: read-only by ffprobe/ffmpeg; probe rejects non-regular; NEW 30 s probe timeout -> kill + "could not be analysed in time". relative `..` resolved vs project folder, still read-only: accepted, documented.
  BUG concat_quote escapes ' but not \n/\r -> name w/ newline injects ffconcat directives (file protocol, read-only). fix: any control char in a source path -> no stream copy, re-encode path (-i args safe).
  display: control chars + bidi overrides U+202A–202E, U+2066–2069 -> U+FFFD in displayed names only.
  settings: override must be abs existing regular file; settings file trusted at user-account level (writer can already run code as user) -> stated in threat model.
  downloads (REQ-058): URLs only from ModelSpec (test: all https://); fetch refuses non-https before any connection (offline testable); ureq https_only also redirects, max 5 redirects, timeouts kept; size limit; sha before rename; partial / wrong sha -> delete .part. test: ureq used only in stt::download (include_str! grep test, like packaging tests). no telemetry/update check.
  corpus tests (REQ-057): bad/truncated JSON, nesting > 200, > 16 MiB, 1e309, u64::MAX, negative times, ../../etc/passwd, abs paths to non-media, control + bidi in names, Unix FIFO project (return < 1 s). every case: no crash, clear refusal, nothing written, sources unchanged. settings corpus same idea.
  supply chain: cargo-deny already fails on advisories; add yanked = "deny", documented ignore = [] (future entries "RUSTSEC-…", # reason not affecting). optional weekly schedule run of cargo deny check advisories.
  CI: top-level permissions contents: read; contents: write only package job; third-party actions pinned by full SHA w/ tag comment (ADR-036).
  ffmpeg refresh: 9.0.2 still latest FFmpeg release; Riedl macOS 1789931890_9.0.2 already pinned. Linux/Windows re-pin BtbN autobuild-2026-11-13-12-43 (9.0 branch): linux64-gpl ffmpeg-n9.0.2-57-gd838f9f9ae-linux64-gpl-9.0.tar.xz sha256 01f1a5a6721de2d4e2e49dbadf6eb30bb750934429f685f6f5a389168ac27e23; win64-gpl-shared ffmpeg-n9.0.2-57-gd838f9f9ae-win64-gpl-shared-9.0.zip sha256 f4e068477787394da18a44b27d3e2cb19e05b21495b836a4fd5a31958b608932. source = full commit of gd838f9f9ae (PE resolves), build_scripts_commit = BtbN matching. supersedes ADR-013 "pin month-end" for this pin; risk daily build expires sooner -> re-pin 2026-11-30 month-end on failure (loud).

ADR-035 (0.3.0) test speed. QG-03.
  fixtures generated once per param set -> env!("CARGO_TARGET_TMPDIR")/fixtures/<sha256(params + ffmpeg version)>.mp4, written .partial + rename (parallel-safe, reused across runs).
  smaller media where size irrelevant: 640x360 -> 320x180 (except letterbox/geometry tests); 120 s memory source -> 160x90 @10 fps; proxy source 0.5 s.
  fixed sleeps -> channels/condvars where possible.
  Cargo.toml: [lib] doctest = false (no doctests); [profile.dev] debug = "line-tables-only"; try [profile.dev.package."*"] opt-level = 1, keep only if warm cargo test measurably faster. whisper.cpp already Release (no gain).
  target: cargo test wall >= 30% below M0 baseline (3 runs median). no test removed/weakened.
  runner: cargo test (rule 7). alt cargo-nextest rejected (breaks scripts/test-annotations.sh output parsing; rule names cargo test).
  CI check job: single instrumented run `cargo llvm-cov --locked --ignore-filename-regex 'src/main\.rs$' --fail-under-lines 85 --summary-only` in existing tee + test-annotations pattern (tests once, whisper.cpp built once). msrv job: cargo build --locked --all-targets only (MSRV == stable by policy; tests run in check; none removed).
  open (not blocking, needs /change-workflow): local rule-7 suite runs tests twice (cargo test + coverage.sh); single llvm-cov run locally would ~halve PE verification.

ADR-036 (0.3.0) CI runners + actions. QG-04, REQ-040/042 reworded.
  runners: ubuntu-24.04 (check, msrv, build, package; glibc 2.39 -> runs on Ubuntu 24.04 + 26.04 LTS + current Fedora; AppImage needs fusermount3, present on all), macos-15, windows-2025 (PE confirms labels). removes ubuntu-latest migration notice.
  scripts/package.sh linux: fail if highest GLIBC_ symbol version needed by videolore > 2.39.
  actions (all Node 24 or composite; verified releases 2026-11-14):
    actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0 (replaces @v4 -> Node 20 warning gone)
    actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
    actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
    Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3
    taiki-e/install-action@4a76afe23f9e08d2d9550fb56e556444e8611d21 # v2.90.4 (composite)
    softprops/action-gh-release@8a0cd651e5b2365bd640854a3fa5dfc70e4875f3 # v3.0.4
    dtolnay/rust-toolchain@683b588c487a69d01f6c6ddf40f036ff682cc2c8 (composite) w/ toolchain: stable | msrv
  speech-model cache keyed by 3 sha, VIDEOLORE_STT_MODELS_DIR. packaging/notices/rust-crates.txt regenerated if Cargo.lock changes.
  optional: qemu old-CPU step (ADR-031).

ADR-037 (0.4.0) hw encode arguments per vendor. REQ-055/062. supersedes ADR-032.
  ctx: opt-in Faster; output must equal Best quality format (REQ-062); S4: <=3 VMAF below, <=1.5x size.
  verified at pinned commits 2026-11-14: BtbN c9fe5114 linux64+win64 enable ffnvcodec (50-ffnvcodec.sh), amf (50-amf.sh), libvpl (50-onevpl.sh, not arm64), vaapi (50-vaapi/*, libva loaded from system at runtime, skipped on win), libvmaf (45-vmaf.sh). FFmpeg 9.0 vs nv-codec-headers f2053071 = Video Codec SDK 13.1.15 -> min NVIDIA driver 610.0 (linux+win). Riedl 6a611e19 plain ./configure, VideoToolbox not disabled -> h264/hevc_videotoolbox auto-built; libvmaf on. NOT confirmed: encoders actually present in binaries -> packaging smoke test (M7).
  T = source video_bitrate x factor; M = T*5/4 (cap == target makes QSV/VA-API switch to CBR); bufsize 2T; P = high (H.264) | main (HEVC); -g 250 all (x264/x265 default; ffmpeg default 12 bloats VT/VA-API); B-frames encoder default (setting them breaks older GPUs).
    nvenc: h264_nvenc|hevc_nvenc -preset p5 -tune hq -rc vbr -spatial_aq 1 -b:v T -maxrate M -bufsize 2T -g 250 -profile:v P -pix_fmt yuv420p; factor 100%.
    qsv: h264_qsv|hevc_qsv -preset medium -b:v T -maxrate M -bufsize 2T -g 250 -profile:v P -pix_fmt nv12; own session (d3d11 win, default render node linux); 100%.
    amf (win only): h264_amf|hevc_amf -usage transcoding -quality quality -rc vbr_peak -b:v T -maxrate M -bufsize 2T -g 250 -profile:v P -pix_fmt nv12; 120%.
    vaapi (linux): global -init_hw_device vaapi=va:/dev/dri/renderDNNN -filter_hw_device va; graph tail ;[outv]format=nv12,hwupload[outvhw], map [outvhw]; h264_vaapi|hevc_vaapi -rc_mode VBR -b:v T -maxrate M -bufsize 2T -g 250 -profile:v P; no -pix_fmt; 110%.
    videotoolbox (macOS): h264_videotoolbox|hevc_videotoolbox -allow_sw 0 -realtime 0 -b:v T -maxrate M -bufsize 2T -g 250 -profile:v P -pix_fmt yuv420p; 120%.
  common: -tag:v hvc1 HEVC in mp4/mov; filter graph unchanged (ends yuv420p) -> letterbox/fades/subs identical; Light -threads kept; audio args unchanged.
  ASSUMPTIONS: factors (AMD/VT/VA-API somewhat worse than x264 veryfast per bit; nvenc p5 / qsv medium ~equal) -> QG-05 e/f tunes table; expected size <= ~1.2x; VT may ignore -maxrate for HEVC (harmless). NV12 lossless vs yuv420p; check asserts ffprobe pix_fmt yuv420p.
  video_bitrate None (rare; ffprobe falls back to container bitrate) -> normal method; no untested cq path.
  candidates: win NVENC > QSV > AMF; linux NVENC > QSV > VA-API per render node in number order; macOS VT. skip free when <codec>_<vendor> missing from -encoders (works for user ffmpeg REQ-030). only H.264/HEVC; AV1/VP9 always sw.
  alt rejected: constant-quality modes (-cq, -global_quality, -qp_*, -q:v: no evidence of CRF mapping, bigger size risk, 2nd probe); AMF on linux (out of scope); explicit B-frames; per-vendor settings UI (D2).
  DEVIATION (0.4.0 follow-up, after real hw, Andrea chose fix before release): VA-API CQ variant for drivers refusing VBR (Ubuntu free intel-media-driver 24.1: `-rc_mode VBR` rejected).
    args: h264_vaapi|hevc_vaapi -rc_mode CQP -qp Q (no -b:v/-maxrate/-bufsize), same device setup + hwupload tail, -g 250, profile. Q start 23. VA-API only; QSV/others have no CQ variant.
    size watch: once 2 s of output (or 1/10 of expected duration, whichever first) written, rate = bytes/out_time; if > 1.4 x (normal method video + audio bitrate) -> kill, restart ONCE with Q' = Q + ceil(6*log2(ratio/1.3)), cap 40 (6 QP ~ halves size, aim 1.3x). no 2nd restart.
    final: outcheck refuses CQ file > 1.5 x usual size (normal bitrate x duration) -> sw redo; NOT recorded in hardware_off (content-dependent, not hw fault).
    calibration (Iris Plus G7, 5 sources, table in docs/code-hardware.md): no single QP nor bits-per-pixel rule meets both e (<=3 VMAF) and f (<=1.5x) on all content (cells = size vs Best, VMAF Best-CQ: 1024x576 0.9 Mb/s QP 30 still 1.46x; 1920x1080 4.4 Mb/s QP 27 already +5.5 at 0.28x). QP 23 = start that never exceeded e in calibration; watch+restart+1.5x check hold f. sign: VMAF Best-CQ <=3 required (negative = CQ better).
    HEVC VA-API broken on that driver even at CQ -> probe fails -> sw.
    supersedes the "no untested cq path" and "constant-quality rejected" points above for VA-API only.

ADR-038 (0.4.0) hw probe. REQ-061.
  cmd: ffmpeg -hide_banner -nostdin -v error [global] -f lavfi -i testsrc2=size=640x360:rate=30 -frames:v 8 [-filter_complex tail w/ hwupload for VA-API] <encode_args(cand,codec,2_000_000)> -f null -. same arg set as real export; 640x360 above every vendor minimum; synthetic input, no file access, no network.
  limits [SUPERSEDED 0.4.2, see DEVIATION below]: <=1 s per candidate from spawn (slot wait excluded), <=3 s summed (untried = not working); 1 slot, Probe prio, bg, never preempted; deterministic pick for same working set; one process per (codec,candidate) (combined H.264+HEVC can't attribute failure); stop at first working per codec (typ. 2–4 probes).
  Reason parse: nvenc stderr "Driver does not support the required nvenc API version" | "minimum required Nvidia driver" -> DriverTooOld (Advanced "NVIDIA: graphics driver too old (needs 610 or newer)").
  when: only on demand (triggers in data flow). Best quality default -> nothing runs, Advanced "not checked (only checked when Export is Faster)". result session-only, no persisted cache -> driver changes noticed. reason: avoid waking dGPU on hybrid laptops, slots + up to 3 s bg work for users who never use it; brief allows. export before probe Done -> sw.
  render_nodes: names ^renderD\d+$, char devices only, sorted, <=4; device string only from these, no shell.
  alt rejected: probe every start (cost, dGPU wake); probe every vendor (budget); persisted probe cache (stale on driver change).
  DEVIATION (0.4.0 follow-up):
    CQ retry: VA-API candidate whose plain (VBR) probe fails with a plain failure (not missing device/encoder) is re-probed with the CQ variant (ADR-037), counted inside the same 1 s/candidate + 3 s total budget. Advanced shows "VA-API (renderDNNN, constant quality)".
    old libva guard (linux): bundled BtbN ffmpeg dlopens system libva and core-dumps when libva lacks vaMapBuffer2 (libva >= 2.21; Ubuntu 24.04 = 2.20). before probing: read ffmpeg binary bytes for "vaMapBuffer2" and system libva.so.2 (LD_LIBRARY_PATH + usual lib dirs) for same symbol -- plain byte scan, never dlopen/exec. ffmpeg needs it && libva missing/older -> VA-API + QSV candidates skipped without spawn, Reason note -> Advanced "needs a newer system video library (libva 2.21 or newer)". system ffmpeg linked to system libva unaffected.
    scan cost ~3.1 s on 173 MB bundled binary, background thread, once per session (cached by path+size+mtime); NOT counted in the 3 s run budget (budget = process run time only).
    probe generation number: newer probe replaces older, late results ignored.
    hw offered only if ffmpeg has the `psnr` filter (outcheck dependency).
  DEVIATION (0.4.2, probe time-outs; supersedes limits 1 s/3 s above + "untried = not working"):
    context: MVP release verification: real probe flaky under CPU load (VA-API H.264 test encode > 1 s -> TimedOut -> "no hardware" although hw works). Andrea chose fix.
    ProbeLimits{per:1 s, retry:4 s, total:10 s} (hwenc::ProbeLimits::default). time from spawn, slot wait excluded (unchanged).
    retry: try that hits `per` -> ONE immediate retry of the same candidate with limit `retry` before next candidate (order unchanged -> determinism kept). still timed out -> Reason::TimedOut note, next candidate. try cut short by `total` -> not retried; untried after total -> TimedOut (not "not working").
    VA-API CQ variant (0.4.0 deviation) gets the same first try + retry, inside same total.
    inconclusive: HwStatus::inconclusive() = some codec has no candidate AND a TimedOut note for that codec. app (tick): inconclusive && !hw_rechecked -> launch whole probe once more (hw_rechecked=true; reset on every new trigger via start_hw_probe); first answer stays HwProbe::Done (in use + on screen) until repeat ends; repeat result replaces it. max one repeat per trigger -> bg work <= 2 x 10 s (~20 s) per trigger. generation rule unchanged (newer trigger cancels repeat).
    time-outs NEVER feed REQ-064 failure memory (hardware_off): only failed hw exports do (unchanged ADR-040).
    cost: slow broken candidate now costs 5 s (1+4) instead of 1 s. measured Andrea laptop (Iris Plus G7, Ubuntu 24.04, system ffmpeg): HEVC VA-API test encode ~14 s then fails -> each probe spends 5 s on it -> probe ~6.4 s idle, up to 10 s loaded, and ALWAYS repeated once (HEVC inconclusive) -> ~13–20 s bg per trigger. H.264 usable from first answer.
    alt rejected: single longer per-candidate limit (e.g. 5 s; same worst case, but no cheap 1 s path and no TimedOut signal for the repeat); measure load + scale limits (non-deterministic); persist probe result (stale on driver change, ADR-038); treat TimedOut as hw failure in REQ-064 (would turn hw off on a busy computer).
    tests: FakeRunner never-ending streams for per/retry/total, time-out once then works (also CQ), inconclusive -> single repeat, first answer kept; CI real probe bound = total + 1.5 s, two runs equal.
    NOTE: docs/business-export.md REQ-061 still says "at most 1 second each, at most 3 seconds in total" -> PM to reword (not mine).

ADR-039 (0.4.0) hw-first export, stall watchdog, output check, silent sw redo. REQ-055/063, REQ-021/059.
  hw run: partial .<name>.hw.videolore-partial (registered in work targets, matches ^\..+\.videolore-partial$); reader thread sends -progress lines over channel; main loop recv_timeout(<=250 ms) checks cancel + stall; stall = out_time_us not grown for stall_timeout (30 s) from spawn to first frame, then from last increase -> kill. kill -> wait <=5 s -> abandon (slot released). at most one partial at a time.
  outcheck (all sw tools on hw partial):
    (a) ffprobe -count_packets -show_streams -show_format JSON, run_within 30 s: codec as planned, profile High|Main, pix_fmt yuv420p, w/h/fps = plan, SAR 1:1|unset, codec_tag hvc1 for HEVC mp4/mov, audio present w/ planned codec/rate/channels, video packets = expected ±1, video duration within ±2 frames. colour tags NOT compared (metadata; would perma-disable a vendor writing bt709) -> QG-05 d manual.
    (b) one pass decode + samples: ffmpeg -nostdin -v error -xerror -i file:<partial> -filter_complex "[0:v:0]split[a][b];[b]select='eq(n,k1-1)+…+eq(n,k3+1)'[,scale cap],format=yuv420p[s]" -map [a] -f null - -map [s] -fps_mode passthrough -f rawvideo file:<work>/hw.yuv -progress pipe:1. k = 10/50/90% of expected frames ±1 -> 9 frames. pass = exit 0, nothing at error level, final frame= == packet count. same 30 s stall rule; Light -threads.
    (c) refs: 3x reference_frame_args at k/fps + half frame, run_within 30 s; same graph as sw path, unencoded.
    (d) PSNR per plane Y,U,V in Rust; sample passes if best of its ±1 neighbours >= 25 dB on all 3 planes. frames >1920x1080 scaled down both sides scale=…:flags=area. streamed, <=4 frames in memory (~12 MB @1080p).
    ASSUMPTIONS: good encodes at source bitrate 33–45 dB; green/garbage/wrong-size/swapped-chroma < 15–20 dB; QG-05 records real values, threshold may move. cost: decode ~600–1500 fps 1080p H.264 / ~300–700 fps HEVC @8 thr + 0.3–2 s refs -> ~2–5 s per 1-min 1080p export.
    why PSNR not ssim filter: deterministic, testable w/o ffmpeg, no 2nd graph; all planes catch NV12 chroma faults (likeliest hw failure) that luma SSIM misses.
    residual risk: corruption only between samples passes -> QG-05 d.
  failure (error/crash/signal exit, stall, check fail): kill, delete hw partial (best effort, stays registered), cancel -> Cancelled (no redo), else sw redo .<name>.videolore-partial with 0.3.0 args, progress back to 0 once, no message. redo ok -> Done SoftwareAfterProblem (+ memory, ADR-040); redo fail -> Failed as 0.3.0, nothing remembered (cause not hw).
  stream copy never touched (REQ-023): byte-identical under both choices.
  alt rejected: exit-code-only fallback (ADR-032 plan; misses exit-0 bad frames, S3); full VMAF per export (slow; libvmaf only for QG-05 tooling); parallel decode via tee to pipe (2 slots, complexity; later optimisation for QG-05 g).
  DEVIATION (0.4.0, during build + follow-up) -- replaces (b)(c)(d) above:
    compare by frame POSITION: one ffmpeg pass renders the normal method's video graph unencoded alongside the hw file; ffmpeg `psnr` filter (Y/U/V) on 3 samples at 10/50/90% of frames, each +-1 neighbour; pass = best neighbour >= 25 dB on all 3 planes. >1920x1080 scaled down same filter both sides.
    expected frame count + duration taken from that graph's output (file frames = graph frames +-1, duration within 2 frames) -- not computed from the timeline.
    dropped: Rust PSNR, reference stills (reference_frame_args), expected_video_frames. reason: timeline-based count/timestamps were wrong in 15/30 real cases (clips not whole frames long, many cuts, VFR sources); encoder receives exactly the graph's frames so position matching needs no timing model.
    strict decode (-xerror, one frame per packet) kept as separate pass. metadata (a) kept.
    killed process (stall/cancel/fail) keeps its Export slot in a background thread until it truly exits (not released at abandon) -> process cap holds even with hung drivers.
    hw offered only if ffmpeg has `psnr` filter.
    cost: check = strict decode + full graph render -> on iGPU check costs ~ as much as sw encode saving (QG-05 g fail, risk 24).

ADR-040 (0.4.0) Export choice, failure memory, settings fields. REQ-060/064/065, REQ-045/047/048/049/057.
  settings.json stays version 2, additive #[serde(default)] fields: export: "best_quality"|"faster"; hardware_off: [{format:"h264"|"hevc", method:"nvenc"|"qsv"|"amf"|"vaapi"|"videotoolbox", version:"<CARGO_PKG_VERSION>"}]. reason: 0.3.x ignores unknown fields -> downgrade-safe; v3 would be "too new" for 0.3.x. file w/o fields -> Best quality, no entries, not rewritten. 0.2.x + damaged -> Settings::with(recommend()) -> Best quality.
  validation: entries w/ unknown format/method or > 32 chars dropped individually (file not malformed); <= 8 entries.
  resets: fingerprint change at startup clears; entries w/ version != running dropped on load; "Use recommended settings" clears + reprobes; "Try video hardware again" clears + reprobes.
  matching: entry disables hw for format only while probe still picks the same method; different method winning = hw change -> try again. record only when sw redo succeeded; cancel never records.
  recommended/custom over 4 choices; recommend() export = BestQuality always (S1). Faster never greyed; "no video hardware" sentence only when probe Done and neither format has hw. last export method session-only. "hardware"/"video hardware" allowed in main section; GPU/encoder still banned (REQ-046a).
  alt rejected: settings v3 (downgrade breaks); persisted last method; per-vendor memory w/o method match (stale after GPU change).

ADR-041 (1.1.0) own proportional splitter layout. REQ-024 reworded, REQ-066/067/068. reverses D3 for resizing only (docking/moving/hiding stays out).
  ctx: 4 sections (Media|Preview top, Timeline|Effects bottom), 3 borders; must keep proportions on window resize, only the 2 neighbours change on drag, minimum sizes, persisted in program settings, reset.
  choice: pure ui/layout.rs (Fractions + Mins + GAP 6 px + sections/effective/drag) inside CentralPanel; splitter = ui.interact(gap_rect, Sense::drag()) w/ ResizeHorizontal/Vertical cursor, 1 px hover line, tooltip. bars (menu/toolbar/status) = egui Panel top/bottom resizable(false).
  mins: Media w160, Effects w160, Preview w240, Timeline w240, rows h100 (fit 800x500 min window: 160+6+240=406 <= 800; 100+6+100 <= content h). applied at draw only (stored fractions unchanged -> shrink + regrow restores). mins don't fit -> split in proportion to mins; never panic/negative.
  borders rounded to whole px; sections + gaps cover area exactly -> ±1 px proportions at 800x500, 1280x800, 2560x1440.
  save: once on drag_stopped (Command::SetLayout), not per frame.
  alt rejected: egui resizable SidePanel/TopBottomPanel (absolute widths, no proportional resize, neighbours rule broken, state in egui memory not our settings); egui_dock / egui_tiles (new crate, docking out of scope, own persistence format); eframe persistence (separate file, not our validation/REQ-057).
ADR-042 (1.1.0) single action table drives menus, toolbar, tooltips, shortcuts window, keymap. REQ-025 reworded, REQ-069..072.
  choice: ui/actions.rs const ACTIONS (ActionId, label, shortcuts, tooltip) + ActionId::command(&AppState). greying only from app.is_enabled(&cmd); disabled reason from app.disabled_reason(&cmd) via on_disabled_hover_text. shortcut text via ctx.format_shortcut (Cmd on macOS, Ctrl elsewhere = egui Modifiers::COMMAND).
  keymap: table matched first; egui Copy/Cut/Paste events kept as today; consume_shortcut/consume_key before widgets; off when egui_wants_keyboard_input() or modal_open; Ctrl/Cmd+= accepts Key::Equals and Key::Plus with/without Shift; Save vs Save as matched with exact shift. F1 Shortcuts; Esc topmost popup (see data flow).
  invariant tests: every binding -> command; no duplicate shortcut; shortcuts window list == bound set (replaces HELP.len()==10).
  menus: egui::MenuBar + menu_button/SubMenuButton inside the window; long menus in ScrollArea max_height(available). macOS native menu bar stays off (ADR-024, D14).
  toolbar: 12 text buttons; overflow "»" menu by pure fit() (alt rejected: wrapping (variable height breaks fixed bars), horizontal scroll (hidden actions, no affordance)).
  alt rejected: native menus via muda (new crate, per-OS behaviour, not headless-testable, conflicts w/ ADR-024 Cmd+Q path); separate per-surface command lists (drift between menu/shortcut/help).
ADR-043 (1.1.0) popup + instance state in AppState, driven by Commands. REQ-075..078.
  choice: AppState.popups (open list, current instance, chooser), Popup + Instance enums; panels push OpenPopup/ClosePopup/OpenInstance/ChooseInstance; each popup = egui::Window fixed id, non-modal, one per type, .open(&mut bool), vscroll + constrain; single Instance window shows current instance. position kept for session only (egui memory).
  reason: state in AppState = testable w/o egui (dispatch tests), same Command path as menus/shortcuts, Esc + "close after Remove" logic pure. non-modal so editing + playback continue (REQ-076).
  edits: SetFade{left,right,on} idempotent by clip-id pair; RemoveAllFades 1 undo; line text committed on focus loss = 1 undo; slider release = 1 undo (as ADR-019 UI). PlayLine sets stop_at -> tick pauses at line end.
  instance popup closes / chooser cleared when its instance no longer exists after an edit (cut, undo).
  alt rejected: egui::Modal (blocks editing, REQ-076); OS multi-viewport windows (Wayland inconsistency, not headless-testable; same reason as ADR-028); state only in egui memory (untestable, lost on id change).
ADR-044 (1.1.0) settings v2 additive fields layout + recent. REQ-067/080, REQ-057. extends ADR-028/040 pattern.
  fields: layout: Option<SavedLayout{top,media,timeline}> (plain f32 shares); recent: Vec<PathBuf>. both #[serde(default, deserialize_with = lenient_*)] (pattern of lenient_hardware_off). VERSION stays 2.
  load: 1.0 file (no fields) -> defaults, no notice, write=false. layout non-finite / outside (0,1) / wrong type -> None (others kept, no Malformed notice, no write). recent: keep only strings, absolute, <= 1024 bytes, no control chars/NUL; dedupe; max 10; anything else dropped per entry. non-UTF-8 paths never added (serde_json would fail every later save). worst case file ≈ 20 KiB < MAX_SETTINGS_BYTES 64 KiB (test).
  save: existing atomic save path, same newer-version rules (version > 2 -> in memory only until user changes). saves: drag stop, reset, recent change.
  downgrade: 1.0 builds ignore unknown fields and drop them on their next save (accepted; Future work).
  alt rejected: VERSION 3 (1.0 would treat as too new, lose all settings on downgrade); eframe persistence / separate layout file (second file, second validation path); storing layout/recent in project (C3: program settings never in project).
ADR-045 (1.1.0) UI acceptance tests via egui built-in AccessKit tree. REQ-068..079.
  choice: existing headless Context::run_ui harness + ctx.enable_accesskit() -> FullOutput platform_output accesskit_update -> test helper (ui/tests): node list (role, label, bounds, disabled), click_label, drag(from,to) via injected pointer events. bounds inside ScrollArea checked against own registry UiCache.containers (id, viewport, content rect) recorded by section/scroll_body helpers each frame.
  REQ-068 audit: 800x500, all sections at minimum, 50 media, 200 subtitle lines, 200-char file name; each popup + Settings opened in turn; every interactive node fully inside its container's visible rect inside the window, or inside a registered scroll area whose content contains it and whose viewport is visible; scrollbar nodes only on overflow.
  fallback: egui_kittest 0.37.0 (dev-dep, MIT/Apache) only if helper > ~150 lines. no new crate otherwise (egui's accesskit feature already used internally by egui; eframe `accesskit` feature for screen readers stays off -> Future work).
  alt rejected: egui_kittest now (extra dev-dep + wgpu/snapshot features not needed); pixel snapshot tests (fragile across fonts/OS); manual checklist only (no REQ-068 regression guard).
ADR-046 (1.1.0) egui keyboard zoom off. REQ-071.
  ctx: egui default options zoom_with_keyboard=true consume Ctrl/Cmd +/-/0 before app -> would scale whole UI instead of timeline zoom. choice: ctx.options_mut(|o| o.zoom_with_keyboard = false) at start-up; our ZoomIn/ZoomOut/ZoomFit bound there. test asserts option off + Ctrl+= changes timeline zoom only. loss: no UI scaling by keyboard (UI text size setting = Future work).
ADR-047 (1.1.0) drag and drop via eframe/winit; Linux Wayland not supported. REQ-081. Andrea chose option A (ship where supported).
  ctx: winit 0.30.13 (eframe 0.37) emits DroppedFile/HoveredFile on Windows, macOS, X11 (incl XWayland apps) but never on native Wayland.
  choice: frame_ui reads i.raw.dropped_files (DroppedFileHandle::path()) + hovered_files (overlay). pure drop_plan: any .vlproj -> first project opened after REQ-028 prompt, rest ignored w/ status msg; else Import(paths) through File > Import path (REQ-002/032, probe hardening). ignored while a modal is open (status msg). Wayland: File > Import…/Open… are the way; known limitation in README + roadmap; REQ-081 names platforms.
  alt rejected: force X11 backend on Linux (WINIT_UNIX_BACKEND / with_x11; loses native Wayland scaling/IME, regressions for all Wayland users); own Wayland data-device via smithay-client-toolkit (new crate + unsafe-adjacent surface glue, not eframe-integrated); wait for winit release (no date).
  re-check when eframe moves to a winit release with Wayland DnD.
ADR-048 (1.1.0) timeline instance markers derived, pure; undo labels + disabled reasons. REQ-072/074/077.
  markers: from projection (no stored marker data -> project format unchanged, follow ripple/zoom, vanish when clip cut). fade marker at boundary (fade time = boundary time, ADR-008 length unchanged). Volume has no marker (not time-based). hits tol 4 px; >1 -> chooser. culled to visible range; lines list via show_rows.
  history labels: (snapshot,&'static str) pairs, label moves with snapshot between stacks. no format change (history not persisted).
  disabled reasons: AppState::disabled_reason pure next to is_enabled; at least Split, Merge, Paste, Undo, Redo, Export, Generate subtitles, Add music.
  alt rejected: stored marker list in project (format change, sync bugs); fade marker at end of left clip (ambiguous w/ ADR-008 split length).
  [1.2.0: Volume gets a marker per clip when level != 100 or muted (ADR-050); start/end fade diamonds (ADR-049). still derived, nothing stored for markers.]

ADR-049 (1.2.0) start/end fades attached to video edges; fade effect on a clip = both edges. REQ-083, REQ-011 reworded, S1=A, Q1.
  ctx: fade through black now applies per clip (fades at both edges) or whole video (black at very start + very end, staying there whatever is edited later).
  choice: Timeline.start_fade/end_fade bools (edge-attached), boundary fades unchanged (BTreeSet<(ClipId,ClipId)>). set_clip_fades(id,on): both neighbour boundaries + start edge if first + end edge if last. set_edge_fades(on) = whole video (start+end). removing from a clip removes both its edges (incl edge fades if first/last). fades_of(i) yields edge fades -> render::segments, frame_key, markers reuse it; length ADR-008 min(0.5 s, len/2). each edge/boundary fade own marker + instance (FadeStart/FadeEnd), removable alone. "+" boundary handle stays.
  consequence (Q1): moving clip 1 into the middle leaves start fade at video start (REQ-083c needs it after cutting first clip). edge fades on empty timeline ignored.
  alt rejected: clip-attached start/end flags (fade would move with clip / vanish when clip 1 cut -> breaks REQ-083c); fade-in/out per clip (one-sided fades, out of scope D1).
  1.1 projects: no edge fades -> identical result (golden).
ADR-050 (1.2.0) per-clip volume in Clip; gain in segment chain. REQ-084, REQ-020/023/078 reworded, M4/M5, Q2.
  Clip{volume:u8 0..=100, muted} default 100/false. gain = (vol/100)^2 (D12/ADR-019) x video Mix gain; muted -> 0. no boost (>100 refused in project).
  render: Segment.gain; per-segment audio `volume={:.6}` before afade only when gain != 1.0 -> default argv byte-identical to 1.1 (golden test). stream copy forbidden when any clip !=100 or muted (REQ-023).
  edits: split/insert_at/copy/paste/reorder keep volume; merge needs equal (volume,muted) else greyed "The clips have different volumes".
  undo (Q2): applying Volume opens popup; undo step recorded on value change (slider release / mute toggle), not at apply -> apply w/o change = no empty undo step.
  marker at clip start only when !=100 or muted; click -> ClipVolume instance popup; also slider+mute in Clip details.
  alt rejected: volume as separate per-clip map in Timeline (sync on split/copy); per-clip `volume` filter always (argv changes for 1.1 projects); applying gain in amix stage (needs per-clip labels anyway).
ADR-051 (1.2.0) project schema v3, always written. REQ-094, REQ-029/057, M16, Q5/Q6.
  new fields: Clip.volume/muted, Timeline.start_fade/end_fade, Line.position, MediaFormat.subtitle_tracks; all #[serde(default)].
  choice Q5: SCHEMA_VERSION = 3 always written; loads 1,2,3. 1.1 refuses v3 (version > 2 -> TooNew, file untouched).
  rejected: write v2 when no new data used (refuse-only-when-new-data, M16 option): two writer paths + tests, 1.1 could open a file then silently drop later-added data on the same path mix; small gain.
  validation (Q6): clip volume > 100 -> refused ("volume above 100%"); Pos x|y > 10000 -> refused; unknown ids refused (existing); edge fades on empty timeline ignored; subtitle_tracks metadata sanitised/truncated, never refuses (it is cache data re-derivable by ffprobe). NaN/negatives impossible (integer types; serde error -> refused).
  re-check of 1.1 media (tracks None) patches both current + saved ProjectDoc -> no dirty marker from background work.
ADR-052 (1.2.0) one ApplyEffect command; effect drag via egui DragAndDrop. REQ-082, REQ-075 reworded, REQ-069/079 extended, M2/M3.
  Command::ApplyEffect{effect:EffectKind{Fade,Subtitles,Volume}, target:Target{Clip(ClipId),WholeVideo}} = single path for drop, context menu "Effects ▸", menu "Apply to selected clip ▸"/"Apply to whole video ▸", double-click row. each opens popup of instance created/changed; Fade/Subtitles 1 undo, Volume per ADR-050.
  DnD: row Label sense(click_and_drag); drag_started -> egui::DragAndDrop::set_payload(ctx, EffectKind); video clip dnd_hover_payload -> highlight + CursorIcon::Copy; dnd_release_payload -> ApplyEffect{Clip}; payload over non-target -> CursorIcon::NoDrop; release elsewhere -> nothing (music track, empty space). in-app DnD (not OS DnD) -> works on Wayland too (ADR-047 limitation is OS file drops only).
  rows: name only + tooltip; click = popup, double-click = whole video (egui double_clicked also fires clicked first -> popup opens then apply; accepted, apply opens same popup).
  disabled reasons: "Select a clip first", "Subtitles are being created", "No subtitles on the timeline", "The clips have different volumes".
  alt rejected: own pointer tracking for DnD (egui built-in suffices); separate commands per effect x target (6+ commands, drift).
ADR-053 (1.2.0) embedded text subtitle tracks: ffprobe detection + ffmpeg extraction to SRT. REQ-086/087, M9/M17, Q4.
  detection: existing ffprobe -show_streams JSON (no new process). codec_type=="subtitle" allow list subrip|srt|ass|ssa|mov_text|webvtt|text; others (hdmv_pgs_subtitle, dvd_subtitle, dvb_subtitle, xsub, dvb_teletext, …) ignored. <=16 tracks kept; ordinal = index among subtitle streams counted by us (u8) -> `-map 0:s:N` (no file string reaches argv). language must match [A-Za-z0-9-]{1,16} else None; title sanitize_for_display (control+bidi -> U+FFFD) <=64 chars. bad subtitle entry never rejects media.
  1.1 media (tracks None): background re-probe on project open (Probe prio); if subtitles requested first -> pending_create waits for it; failed probe = no tracks.
  extraction: track_args = `-hide_banner -nostdin -loglevel error -y -protocol_whitelist file,pipe -i file:<src> -map 0:s:<N> -c:s srt -f srt file:<work>/track.srt` (private work folder ADR-033, fixed name, Background slot 1, cancellable). read via project::read_limited (regular file, 16 MiB) -> srt::parse. ASSUMPTION: ffmpeg normalises cue times to file start (integration test MKV with -output_ts_offset).
  track pick per source (Q4): user picks a label "<language name or code> — <title>"; per source: same language+title, else same language, else first track. sources without tracks -> Recognise.
  alt rejected: Rust MKV/MP4 demuxers (matroska/mp4 crates: new deps, more untrusted parsing in our process, more formats to cover); ffmpeg -f ass (richer, but SRT is the common simplest text form and one parser suffices).
ADR-054 (1.2.0) in-house srt.rs (parse + write); SRT export. REQ-088, M15, Q7.
  write(&[ShownLine]): sort (start, line id), number from 1, `HH:MM:SS,mmm --> HH:MM:SS,mmm` (ms floored, end >= start+1 ms), LF, UTF-8 no BOM, one text line per cue, empty-text lines skipped (Q7). timeline times (projection: cuts/trims/copies applied). no positions.
  parse(&str): tolerant (`,`/`.` ms, CRLF, BOM, missing index, junk blocks skipped); caps 100,000 cues, MAX_TEXT per cue; checked arithmetic, hours <= 99,999, ms < 1000, times finite >= 0, end > start else cue dropped. total function (never panics).
  strip_tags: remove `{…}` and `</?[A-Za-z][^>]*>`, `\N` `\n` `\h` -> space, join lines, then clean_text (MAX_TEXT, no control chars).
  export: File > Export subtitles (.srt)… rfd save dialog default `<project name>.srt`; project::write_atomic (temp + rename) -> no partial file; UI thread (small, like project save; 10k lines < 100 ms). greyed "No subtitles on the timeline" when no shown line with text.
  alt rejected: srtparse / subparse / srtlib crates (small gain, extra untrusted-input dep, some unmaintained; ~200 lines in-house fully testable).
ADR-055 (1.2.0) subtitle creation per range with modes Recognise / Align / Spread; all-or-nothing. REQ-085/087/095, REQ-044/052/054/015, S2=A, S3=B, M9..M14, M18, Q3, Q9.
  ranges: clip -> (media, src_in, src_out); whole video -> each distinct source in full (REQ-014 unchanged). extraction with `-ss from -t len` before -i; transcriber offset = from + chunk offset.
  modes per range: Recognise (today's path on range); Align{track,language} (track text + whisper words aligned, ADR-056); Spread{track} (track text + cue timing, words spread evenly; no recognition).
  dialog (Modal::SubtitleSource) when >=1 involved source has usable track; asked every time (M9). Use wording depends on machine.speech_supported() && stt::available(): if false -> "(this computer cannot match words to the speech; they will be spread evenly)" -> Spread, never download (Q9: also builds without stt feature).
  consent: existing modal + extra "Use without matching words" when reuse_path (Align -> Spread); Cancel nothing. model checks unchanged (ADR-030).
  fallbacks: (i) cue < 50% matched or no speech -> even spread, counted in status "N of M lines could not be matched to the speech; their words are spread evenly over the line"; (ii) CPU/build cannot recognise -> Spread all, no message beyond wording; (iii) download refused -> Spread; (iv) recognition fails -> that range Spread + message. Align never Fails the job; only Recognise ranges can.
  result (Q3): Done -> 1 undo "Create subtitles"; replace_range(media,from,to,lines,language): removes lines of that media overlapping [from,to), adds new clamped to range, edited=false; whole-video recognition -> replace_all. language from track (639-2 -> 639-1 static table, ~99 whisper langs, B+T codes, und -> None = detect) unless user overrode (REQ-015).
  REQ-044 confirm only for hand-edited lines overlapping the ranges. one job at a time (M18).
  cancel during matching/download -> nothing changes, no undo (M14). progress label "Matching subtitles to the speech" if any Align range else "Creating subtitles".
  alt rejected: Spread only (S3=B wants accurate highlight); re-time cue start/end from speech (out of scope, Future work); per-range separate undo steps (partial results on cancel).
ADR-056 (1.2.0) word alignment: per-cue Needleman–Wunsch, in-house, deterministic. REQ-095, M11, Q10.
  tokens: track split_whitespace (spelling kept); normalised = Unicode lowercase, alphanumerics only; empty normalised never matched. whisper words (ADR-021 token timestamps) normalised same.
  per cue in time order with monotonic cursor; candidates = recognised words with start in [cue.start-1.0 s, cue.end+1.0 s] and index >= cursor. NW: exact +2, fuzzy +1 if lev(a,b) <= max(1, floor(len/4)) and both >= 3 chars (short words exact only), substitution -1, gap -1; traceback prefers diagonal; cursor after last matched candidate.
  cue matched iff matchable > 0 && matched/matchable >= 0.5; else words=[] (even spread via existing ShownLine::word_at / ass::words_of, D13).
  timings: matched start = recognised start clamped [cue.start, cue.end), non-decreasing; unmatched runs spread evenly between previous anchor end (or cue start) and next anchor start (or cue end); end = min(own end, next start), >= start+10 ms, clamped cue.end; final pass guarantees validate_subtitles invariants. line start/end = cue (never re-timed), words.len() = token count, edited=false.
  perf: 10k cues < 50 ms (per-cue DP bounded by cue tokens x window words).
  gated integration test (Q10): committed fixture tests/fixtures/align/ (speech.mkv ~20 s Opus + SRT w/ 1 misspelling, < 100 KB; reference.json word starts; make.sh dev-only espeak-ng per word/phrase concatenated at known offsets). Fast model: >= 90% words matched, each matched start within 0.3 s.
  alt rejected: global DP over whole track (O(n·m) memory on long videos, drift across cues); strsim crate (Levenshtein ~20 lines); whisper DTW timestamps (ADR-030 off; needs per-model alignment heads); forced aligner (wav2vec/MFA: new model download + runtime).
ADR-057 (1.2.0) per-line on-screen position via ASS \pos; preview drag; selection as UI state. REQ-089..092, REQ-017/018, M6..M8, Q8.
  Pos{x,y:u16} = centre of text box, 1/10000 of output frame -> survives output format change (REQ-022). None = preset place (1.1 output identical).
  ASS: `{\an5\pos(X,Y)}` from our code only, validated u16 -> no injection path (escape() unchanged). preview = export (same ass::document, PlayRes = export geometry).
  hit-test/clamp: placement::boxes measures with egui text layout using bundled Noto Sans Bold (ass::FONT) registered as FontFamily::Name("subtitle") at ass::font_size(W,H), wrap W-2·margin_lr + box_padding; word-highlight measures visible chunk; measure fn injected (fake in tests). centre clamped [w/2, W-w/2] x [h/2, H-h/2].
  drag: drag_pos transient (not history/project, nothing saved during drag), part of FrameKey -> latest-only still re-render; ghost (outline+text) painted at once; release -> SetLinePositions 1 undo. during playback: ghost, applied on release via normal edit path.
  group (Q8): every selected line gets exactly the dragged line's final position; only the dragged line's box clamped (others may cross edge if wider; accepted, libass still renders).
  selection: BTreeSet<LineId> in AppState, NOT in history/project; pruned after edit/undo/redo. Ctrl/Cmd+click (egui Modifiers::command) toggle on markers + popup list; plain click Replace + open instance; Shift+drag overlay ui.interact(tracks_rect, Sense::drag()) registered after clips, only when shift held at press; selects lines whose timeline START is in range; +Ctrl/Cmd adds; seek to first selected. Esc: topmost popup, else clear.
  Reset position: line popup / Effects menu / marker menu, on selection resets all.
  alt rejected: margins (MarginL/R/V per line: no free 2D placement); drawing subtitles as egui overlay in preview (preview != export); measuring with ab_glyph/fontdue (new crate; egui already lays out text); selection in history (undo would jump selection).
  known limits: egui vs libass width may differ by pixels at edge clamp (ASS authoritative); text edit after drag may make box cross edge (no re-clamp); positioned lines overlapping in time overlap on screen.
ADR-058 (1.2.0) ruler labels by visible range; round tick steps. REQ-093, M1, Q7.
  ui/ruler.rs pure: mode(t_right) Seconds if <= 90 s, MinSec if <= 5400 s, else HourMinSec; t_right = time at right edge of visible rect. label: "45s" | "01:40"/"75:30" (minutes unwrapped, zero-padded 2) | "01:05:00".
  STEPS s = [1,2,5,10,15,30,60,120,300,600,900,1800,3600] then 2h,5h,10h… ; ticks(t0,t1,z,char_w): width = chars(label(t1,mode))*char_w + 8 px; step = first with step*z >= width; only multiples in [t0,t1]. char_w = monospace 9 pt glyph width passed in. invariant consecutive dx >= label width -> no overlap. O(visible ticks).
  alt rejected: step ceil(60/zoom) (1.1, overlaps + odd steps); sub-second labels (out of scope).

ADR-059 (1.2.6) 360 projection engine: ffmpeg v360 for preview AND export. REQ-100..104, M-1.
  ctx: preview=export within 1°; subtitles/fades/hw/check already in ffmpeg graph. choice: graph `sendcmd` + `v360@cN` (input=e output=flat), built by the same render code for Source::Preview/Export. alt rejected: in-process CPU reprojection (bilinear 0.5 Mpx ~10 ms; instant drag, but 2nd implementation + subtitles burned after distortion); GPU glow shader (untestable headless, 2 impls); new crate (none needed). tradeoff: camera change = new latest-only still -> latency ~125-155 ms measured on tiny clips (4K/8K-copy targets 400/700 ms are ESTIMATES, not gates; only gate = UI frame <200 ms during export/STT); instant in-process quick view = Future work if poor.
  AS BUILT (spike, ffmpeg 6.1.1; CI/bundled 9.x not re-measured): (1) commands are ADDITIVE unless filter has reset_rot=1 -> always set, every command absolute. (2) sendcmd single instants miss frames -> each command has interval START-END of half a frame either side of its frame. (3) target named `v360@cN` (N=segment index) so several 360 clips are commanded separately. (4) each command re-inits the LUT ~75-100 ms at 1080p -> send ONLY changed values per frame; zoom via ONE command `d_fov` (diagonal fov, camera::diagonal_fov_deg from h fov + output aspect; equals h_fov to one grey level) instead of h_fov+v_fov. (5) yaw+ right, pitch+ up, degrees. Consequence: moving-camera export ~1/3 real time or worse at 1080p (the 0.5x estimate NOT met; yaw-only paths cheapest); USER DECIDED to accept it, speed work in roadmap (fewer commands/s, in-process projection). Preview copy rule: source wider than 4096 -> 4096x2048 copy (own cache key), up to 4096 original. First plan (h_fov+v_fov explicit, half-frame-early instants, 400/700 ms and 0.5x as targets) superseded.
ADR-060 (1.2.6) 360 detection + override. REQ-100.
  ffprobe -show_streams already prints side_data_list. recognised iff "Spherical Mapping" projection "equirectangular", no bound_left/right/top/bottom != 0, and no "Stereo 3D" type other than 2D. metadata yaw/pitch/roll ignored (front = centre; assumption). Only enumerated values read; no file string is shown or put in a command. MediaFormat.spherical Option<bool>; None -> bg re-check on project open patches media + saved ProjectDoc (dirty unchanged), exactly as subtitle_tracks (reuse path). user switch treat_as_360:Option<bool> per source in project (override wins); not undoable, marks dirty. Fixture: tests/fixtures/spherical-360.mp4 <100 KB built once (ffmpeg + inserted spherical-v1 uuid XML box or sv3d; make script dev-only), plus JSON unit fixtures of ffprobe output (equirect, cubemap, stereo, bounds, none). alt rejected: guess by 2:1 (M6 false distortion); read metadata in Rust (own mp4 parser; ffprobe does it).
ADR-061 (1.2.6) camera path model. REQ-102.
  per SOURCE media, SOURCE time (like subtitles, ADR 018): continuity after split/trim/cut free; outside kept range hidden not lost; media removal drops its path. consequence: two copies of the same range share POVs (accepted; Future work). ints centi-degrees (Eq/Hash, deterministic FrameKey/golden). interpolation f64 from ints at render time; yaw shortest arc (|d|<=180, 180 -> +), linear in t, pitch/zoom linear. no POV: live cam if any else default(0,0,90°) -> live view never exported unless Set POV (REQ-101). max 200/source; replace at same t. camera Edit in history snapshot (Set/Edit/Remove POV = 1 undo each). per-frame command file: lines only while camera moves (between POVs: every frame; hold spans: one line at span start); bounded (>2M lines -> step doubled). alt rejected: per clip POVs (copy/split bookkeeping, REQ says continuity); keyframes in timeline time (breaks on ripple); eased curves (OOS).
ADR-062 (1.2.6) live view, drag, zoom. REQ-101.
  live per source, window state, not saved/undo. angle = px*fov/view_w (equal in paused + playback; tests 100 px -> same angle). picture follows pointer => yaw decreases on drag right. wheel (no modifier) over preview of 360 clip and pinch: fov *= exp(-0.0015*scroll_points) clamp 30..110; pitch ±90 clamp; yaw wraps. subtitle box under pointer wins (placement::boxes hit first). path override (see app) until seek/play/Reset. no keyboard control (OOS). playback + drag with override: restart stream (latest-only).
ADR-063 (1.2.6) export geometry + preview size policy. REQ-103/104/022/023.
  360 clip fills the output frame at output aspect (no bars): zoom = horizontal fov, sent to v360 as d_fov (diagonal; v360 derives h/v from output aspect). REQ-022 auto format: 360 counts 1920x1080 at source fps (all-360 project -> 1920x1080); if output is e.g. 9:16 the camera still fills it (PM text says "usual letterboxing" applies to normal clips; note in report). never stream copy. hw export: v360 stays in sw graph before hwupload/encoder; outcheck samples from graph render (slower on 8K, 3 samples). proxies: 360 source w>4096 -> 4096x2048 copy (720p rule useless: 320 px across 90° FOV); w<=4096 original. too large (>16384x8192): message "This 360° video is too large to show here. It is still exported.", no decode; export still attempted (ffmpeg memory risk, accepted). alt rejected: letterbox camera view to 16:9 inside portrait output (ugly bars); 360 output format (OOS).
ADR-064 (1.2.6) boxes, per-line size, align/reset. REQ-096..099.
  generic box layer (BoxItem list) but only lines today (M2). size Option<u16> permille 250..4000 per line, survives format change; ASS `\fs`+scaled bord/shad via our own tags (numbers only); wrap width unchanged (text may wrap more); placement::boxes measures with size factor. resize: corner proportional, opposite corner fixed, limits 25/400 + picture; resulting centre -> Pos (saved with size, 1 undo via ResizeLines). group: all selected get dragged line's size, keep own Pos. Align = pos.x or pos.y =5000 per selected line; keeps other coord+size. Reset clears pos+size (Reset position same command). show_boxes program setting; hidden -> no handles. cursor ResizeNwSe/NeSw/Grab/Move. boxes never in export (ASS from model only). alt rejected: \fscx/\fscy (box measure diverges, no wrap change); font size via style per line (ASS style explosion); free-aspect resize (OOS).
ADR-065 (1.2.6) schema v4. REQ-105.
  SCHEMA_VERSION 4 written always (same reason as ADR-051: one writer). loads 1..4, TooNew >4. new fields: media.spherical, media.treat_as_360, camera:CameraPaths, line.size. validation: >200 POVs/source, unknown media, t<0 or duplicate t, size outside 250..4000 -> refuse (damaged); yaw/pitch/fov out of range -> clamp+normalise (view settings, not structure). golden: v1, v2(1.1), v3(1.2.5) fixtures exported before/after identical argv; 1.2.5 refuses v4 w/o writing. hostile corpus: NaN/huge numbers (ints -> serde rejects floats), 10k POVs, wrong types.

## NFR targets (0.2.0)
- <= 3 ffmpeg/ffprobe procs always; by construction (ADR-015); test atomic high-water mark: 20 imports + export + preview.
- UI thread: no subprocess/blocking I/O except project save/load (<= 16 MiB) + small linux desktop writes at startup.
- import 10 files: dispatch returns immediately, frame < 50 ms.
- still frame: 0 renders for non-changing edits; <= 1 per 10-edit burst.
- A/V: video advances during audio stall; drift after resume <= 40 ms (business 100 ms).
- export 200 segs/1 src: 1 decoder; frame-accurate (frame count = Σ seg frames; framemd5 spot checks).
- memory: <= 200 MB peak RSS growth short vs long same-res source (sysinfo own pid); CI long = 2–5 min lavfi; ASSUMPTION: no size-dependent structure -> holds for 4 GB; unit tests on buffer caps.
- playback start with proxy < 300 ms (target, not gate). STT tiny >= 10x real time @4 thr (estimate). installer 70–130 MB (estimate, measured). startup to window < 1 s (ffmpeg check bg).
- export progress >= 2/s, cancel < 1 s, no partial outputs, atomic project save (unchanged).
- platforms: Linux x86_64 (Ubuntu LTS, Fedora current), macOS aarch64, Windows x86_64. STT on x86_64 needs AVX2+FMA+F16C+BMI2 CPU (ADR-027); everything else no extra CPU req.
- coverage >= 85% line, feature coverage 100%; every REQ >= 1 `// covers: REQ-0xx` test.

## NFR targets (0.3.0 additions/changes)
- procs: "<= 3" -> <= budget.max_processes of current level (3/3/clamp(L/2,3,6)); test 20 imports + export + preview at every level, L=1,8,16: high-water <= cap, all complete.
- detection: < 50 ms typical (estimate), <= 3 s guaranteed, deterministic, no network.
- startup (QG-01): <= 0.2.2 + 0.5 s (detection + cleanup ms; Settings window built only when opened).
- QG-01 at recommended: export <= 0.2.2 +5% (Balanced ffmpeg args == 0.2.2); Fast subtitles <= 0.2.2 +5% (same model, >= threads, flash attn gated by measurement, 1 state/job).
- REQ-056 harness ui::tests::stays_responsive_while_exporting_and_transcribing: FakeRunner export streaming progress ~3 s + fake transcriber busy on available_parallelism threads (Maximum) + 30 headless frames each injecting Space / timeline scroll / Settings click; pass = each frame (tick + run_ui) <= 200 ms wall and input effect visible next frame. rule: UI thread never waits on lock held across worker I/O (try_lock / channels only; review point).
- memory REQ-039 unchanged; MostAccurate +~1.3 GB (estimate), greyed < 7 GiB.
- platforms: STT on x86_64 still AVX2+FMA+F16C+BMI2 (+SSE4.2, AVX); without -> gate message, never crash (ADR-031).
- QG-03: cargo test wall >= 30% lower than M0; CI time not growing.

## NFR targets (0.4.0 additions)
- QG-01: Best quality (default) export argv byte-identical to 0.3.0 (golden test from M0); no probe, no check, start-up unchanged. Faster w/o working hw: no extra cost, same args.
- probe: [0.4.2] 1 s first try + one 4 s retry on time-out per candidate, <=10 s total, at most one automatic repeat when inconclusive (<= ~20 s bg per trigger); 1 Probe slot, no network, deterministic order; CI real-ffmpeg probe < total + 1.5 s (x2 runs, equal results) and finds no hw. [was <=1 s/cand, <=3 s total]
- export: stall 30 s; post-kill wait <=5 s then abandon; cancel <1 s every phase (poll <=250 ms); all processes sequential, 1 Export slot each (REQ-033); one partial at a time; samples in private work folder (REQ-059); extra mem: none in our process since ADR-039 deviation (PSNR inside ffmpeg; old Rust-PSNR ~12 MB figure obsolete) (REQ-039).
- REQ-056 harness must pass with FakeRunner hw export running.
- QG-05 (manual, real hw; matrix win NVIDIA/Intel/AMD, linux NVIDIA prop/Intel/AMD, macOS AS; H.264+HEVC x Faster/Best): a hw used, b format+frames = Best, c hvc1 + QuickTime/iPhone, d visual, e <=3 VMAF below, f <=1.5x size, g >=40% less time @1080p incl check (ASSUMPTION, adjustable), h forced failure -> correct sw file. results per release in docs/code.md.
- QG-05 g RISK: hw removes only encode; decode+filter often bound @1080p; check adds ~15–35% of hw run. estimate (ASSUMPTION) 20–45% saving @1080p, more for HEVC. recommend measured target, not gate.
- QG-05 MEASURED 0.4.0 (Andrea laptop, Intel Iris Plus G7, Ubuntu 24.04, free intel-media-driver 24.1, system ffmpeg 6.1.1, VA-API H.264 CQ): e pass (VMAF Faster-Best -1.4/+1.4/+6.7, i.e. never >3 below), f pass (0.96x/1.32x/1.38x), g FAIL (time vs Best: -14%, +41%, +6%). HEVC VA-API fails on driver even at CQ -> sw. with bundled ffmpeg: VA-API/QSV skipped (libva 2.20) -> no hw at all. full row in docs/code-hardware.md.
- RECOMMEND (later version): re-baseline QG-05 g (per-hardware-class target, dGPU vs iGPU) or cheaper check: render graph only for short segments around the 3 samples (seek) instead of full graph; keep strict decode.
- platforms: win x86_64 (NVENC, QSV, AMF), linux x86_64 (NVENC, QSV or VA-API Intel, VA-API AMD), macOS AS (VT).

## NFR targets (1.1.0 additions)
- REQ-056 extended: every frame <= 200 ms during export + subtitle generation incl splitter drags and open popups (harness adds pointer-drag frames + each popup open). target < 16 ms/frame at 1280x800, 50 media, 200 lines (ESTIMATE, not gate). layout O(1); markers culled to visible range; lines show_rows; settings saved only on drag release / reset / recent change (never per frame).
- layout: whole-pixel borders, proportions ±1 px at 800x500, 1280x800, 2560x1440; full coverage, no overlap; mins enforced; impossible sizes -> >= 0, no panic.
- min window 800x500 unchanged; REQ-068: every section body + popup in ScrollArea; popups constrain + vscroll; preview controls allocated first, picture shrinks (min 0); timeline vertical scroll when tracks don't fit.
- toolbar: all 12 actions reachable at 800 px (overflow »).
- markers x = instance start ±1 px at min/mid/max zoom.
- REQ-057 settings corpus additions: layout NaN, ±inf, negative, >1, strings, nested arrays; recent 10k entries, relative paths, control chars, NUL, 5 KB paths, non-strings -> defaults/dropped, other settings kept, no crash, no Malformed notice.
- settings worst case with layout + 10 recent ≈ 20 KiB < 64 KiB cap.

## NFR targets (1.2.0 additions)
- alignment: 10k cues < 50 ms (unit test); recognition cost for Align = today's recognition on same range at Settings level; track extraction demux-only ~1–3 s/GB (ESTIMATE), 1 Background slot, cancellable.
- processes: track extraction + range audio extraction each 1 Background slot, sequential (REQ-033); whisper outside cap (ADR-015).
- SRT write 10k lines < 100 ms (UI thread, atomic). ruler O(visible ticks). drag ghost < 16 ms; nothing saved during drag.
- REQ-056 harness + preview subtitle drag + effect drag during export + matching: every frame <= 200 ms.
- memory: track text read <= 16 MiB, <= 100k cues, MAX_WORDS (200k) project cap; range audio 120 s chunks (REQ-039).
- compatibility (REQ-094): v1/v2 load with defaults (volume 100, unmuted, no edge fades, position None, tracks None); golden export argv of v1 + 1.1 (v2) fixtures identical before/after (captured in step 0 before any change); 1.1 refuses v3 without writing.
- position accuracy: export text centre within 2% of frame height of preview position (libass integration test 320x180).
- alignment quality gate (gated, Fast model, committed fixture): >= 90% words matched, matched starts within 0.3 s (ASSUMPTION borderline, risk 38).

ADR-066 (1.3.0) text items = own project list, rendered by the subtitle ASS/libass path (one engine for preview and export). REQ-108..111, 113.
  alt rejected: drawtext (no wrap box, weak fontconfig name lookup, no move/fade combos); per-item PNG + overlay expressions (sprites, slide/fade by expressions, layout drift vs egui); in-process rasteriser (second engine). reason: wrap, escape, fonts and burn-in already tested.
ADR-067 (1.3.0) text effects = ASS events from one pure function on the timeline: \fad/\fade (fades), \move (slide, times relative to event start), one event per visible blink phase. caps 2000 events/item, 20000/project, refused by validation. REQ-109.
  alt rejected: \t chains for blink (long, slow to parse); sendcmd on ASS (libass has no runtime change); per-frame overlay expressions. reason: testable without ffmpeg, bounded cost.
ADR-068 (1.3.0) installed fonts: fontdb 0.24.0 default features (MIT; pure Rust; keeps fontconfig dirs on Linux); background scan at start; family name stored, charset whitelist (letters digits space - . _ ( )); egui gets font bytes from a worker (cap 64 MiB, LRU 8); missing font kept, libass fallback, UI note. REQ-108/111. TECHNICAL (decided: user answer "installed fonts").
  alt rejected: font-kit (native code per OS), fontique 0.13 (larger tree), per-OS APIs (3 implementations). Only new crate of 1.3.0.
ADR-069 (1.3.0) 360 anchor stored as (yaw, pitch, size_cd, wrap_cd) in 1/100 deg; anchored text burned into the equirect frame of its 360 segment BEFORE v360 (static ASS per segment, local times; box across yaw ±180 written twice). REQ-112. ADDENDUM (built, DEV-3): anchored SUBTITLE lines are drawn as plain white TextItems (no preset box/highlight); automatic view-fixed lines unchanged.
  alt rejected: per-frame ASS repositioning (libass cannot change placement at run time); overlay after projection with per-frame sprites (second projection); GPU (rejected ADR-059). reason: exact placement from the same projection, no per-frame cost.
ADR-070 (1.3.0) projection in pure camera.rs: rectilinear project(cam,w,h,yaw,pitch)->Option<xy> and unproject; same convention and f=(w/2)/tan(hfov/2) as v360 (sign from the 1.2.6 spike; band test). box = projected tangent-plane corners. REQ-112/111. Tolerance 1 deg vs real ffmpeg.
ADR-071 (1.3.0) graph: events computed on the timeline, cut per segment; anchored pieces in 360 segments before v360; flat pieces in the flat ASS after concat only on flat ranges; no extra ffmpeg process (ADR-015 budget). 1.2.6 argv byte-identical when no text/anchor (golden).
  alt rejected: one ASS with segment time mapping (projection differs per segment); one ffmpeg run per segment (process budget).
ADR-072 (1.3.0) ExportMode {Automatic, Sphere360, FlatCamera} in the export plan; the UI asks each time for projects with 360 clips (not saved, user decision); Sphere360 = graph without v360, all segments 360 (refused with message otherwise); size = REQ-022 lowest pixel count. REQ-103.
  alt rejected: automatic choice (user decided ask); choice stored in project (user decided ask each time).
ADR-073 (1.3.0) ONE Selection value (BTreeSet<ObjRef{Line,Text}>) in window state, pure transitions in src/selection.rs. removes HashSet selection + selected_line (two states = cause of lost multi-selection). REQ-090..092.
  alt rejected: keep both and sync (the bug source).
ADR-074 (1.3.0) Context group = pure rule over selection + playhead-on-360; actions stay shared-table actions; greyed states removed from the group only. REQ-106.
ADR-075 (1.3.0) rename label only "Fade through black" -> "Fade IN/OUT" in the label table; internal EffectKind name and format unchanged. REQ-107.
  alt rejected: rename internal enum (churn, no value).
ADR-076 (1.3.0) schema v5: texts (#[serde(default)] Vec) + line.anchor (Option, skip if None); written always (ADR-051/065 reason); loads 1..5, TooNew >5; validation list in ai-docs/architecture-plan-1.3.0.md. REQ-113. ADDENDUM (built, DEV-1): on load ttl refused only if <100 ms or >24 h (text.rs MAX_TTL); ttl longer than video accepted on load (cut at video end); add/edit/move clamp to MIN_TTL..=video length (app/texts.rs clamp_ttl). Plan's "longer than video = refused" dropped.
ADR-077 (1.3.0) TECHNICAL: slide and fade of anchored text move in the plane of the sphere picture (direction = frame directions), not across the view (per-frame position would need a second renderer). Consequence: text may leave the view while sliding. REQ-109/112.
ADR-078 (1.3.0) 360 video output: anchored items at anchor; automatic (view-fixed) subtitle lines at their preset frame place (confirmed by Andrea, Q2=A). Spherical Video V1 on the output: spike M0 decides the way (native option, remux, or own box writer last). REQ-103. ADDENDUM (built, DEV-2): spike result ffmpeg 6.1.1 drops spherical side data on re-encode (measured) -> option 3 taken: src/spherical.rs rewrites moov of own MP4/MOV output, adds uuid ffcc8263-f855-4a93-8814-587a02521fdd Spherical V1 XML box; only moov read (size-capped), rest copied; MKV/WebM unmarked; bundled 9.x not measured.
ADR-079 (1.3.0) TECHNICAL: anchored text layout in equirect px: size = 5% of view vertical FOV at add time (deg) x factor, wrap = 60% of view horizontal FOV (assumption, to confirm with REQ-111 wording). REQ-111/112.

ADR-080..091 (1.4.0, PLANNED; full text in ai-docs/architecture-plan-1.4.0.md § ADRs):
- ADR-080 clean audio = ffmpeg afftdn per video clip before clip volume; anlmdn/arnndn/agate rejected. REQ-114.
- ADR-081 clip.clean_audio flag (v6); whole video = every video clip; merge needs equal. REQ-114.
- ADR-082 export dialog: pure src/export/choice.rs + egui modal; session memory only; overwrite confirm; refuses target = project media/.vlproj. REQ-118/119.
- ADR-083 video presets: fixed 12-entry table in format.rs, always re-encoded (hardware eligible). REQ-120.
- ADR-084 audio: Same (AAC) / aac / mp3 (probed); only-audio = audio graph with -vn (GraphSpec::AudioOnly). REQ-119/121.
- ADR-085 360 choice only when all video clips are 360; default 360 video (ASSUMPTION); ADR-072 ask modal removed. REQ-122/103.
- ADR-086 360 switch removed; is_360 = spherical metadata; load drops treat_as_360. REQ-100/123.
- ADR-087 text width permille 100..1000, height fixed 20% H; best fit = pure text::fit with 0.6 em estimate (same for preview and export, no stored size, no font bytes); anchored text uses width_cd/height_cd. REQ-111/116. alt egui measure / ttf-parser rejected for now.
- ADR-088 v6 migration: flat text width by search so fit ~ old size; anchor height from old lines; clean=false. REQ-123. Q1 confirmed = A (2026-11-17).
- ADR-089 drag limit for flat text and subtitles: centre on picture (grab rule); 360 unchanged. REQ-089/117.
- ADR-090 Effects menu = effects + Add text + Apply ▸; item commands in Edit menu (placement only). REQ-069/075.
- ADR-091 Context group + Align/Reset for selected text items; width handle right edge only; Reset = default place and width. REQ-106/115/116.

## 1.5.0 PLAN decisions (summary; full ADRs in ai-docs/architecture-plan-1.5.0.md)
- ADR-093 storage: frame_aspect, whole_scale/whole_move, clip_frames in Timeline (Clip stays Copy; no BTreeMap with ClipId keys in JSON).
- ADR-094 time base: clip keys in source time (never deleted by trim/split; hidden not lost); whole-video keys in timeline time.
- ADR-095 limits: Key<T>{t,v}, <=200 per effect, Scale permille 100..=4000, Move Pos 0..=10000, hold + linear; new keyframes.rs (camera.rs not refactored: debt).
- ADR-096 Scale pivot: Scale keys carry hidden centre offset so corner drags keep the opposite corner fixed; Move independent. TECHNICAL.
- ADR-097 frame size: pure frame.rs; base = format/preset size; fit width/height; even sides; longer side cap 4096 (Q-A closed 2026-11-19 = A).
- ADR-098 layers: picture fitted without pad when frame effect applies; background (colour or blur at 1/8 size); overlay with per-frame scale+position; fades after composition; no effect = identical argv.
- ADR-099 expressions from numbers only (nested if piecewise linear); script file when long; fallback sampled command file if bundled ffmpeg refuses t.
- ADR-100 colour #RRGGBB validated -> 0xRRGGBB; blur fixed strength.
- ADR-101 edit rules: split/copy keep keys, merge needs equal keys, remove drops; one undo per action.
- ADR-102 export: any frame effect re-encodes; Sphere360 ignores all three; flat camera applies after camera view; hw export applies.
- ADR-103 preview hit priority: subtitle/text boxes > Scale corners > Move target > picture (camera).
- ADR-104 format 7: always written; 1.4.x refuses; damaged values corrected with notes; keys dropped when outside the media (REQ-132 says clip: PM to align; flagged 2026-11-19).
- Crates: none new (ffmpeg core filters: color, split, scale, boxblur, overlay; egui colour picker).

## NFR targets (1.4.0 additions, full list in plan)
- clean audio gate: noise-only parts >= 6 dB lower, speech within 3 dB (ASSUMPTION thresholds, PE measures on committed fixture); preview: no audio correction events over 60 s (ESTIMATE); frame <= 200 ms with clean audio + export + STT.
- best fit: 200 items < 10 ms; dialog opens <= 100 ms; golden argv unchanged for projects without new features; coverage >= 85, feature coverage 100 for REQ-114..123.

## NFR targets (1.3.0 additions)
- every UI frame <= 200 ms with texts during export + STT + look-around on 4K x 2K and 8K x 4K (REQ-104 harness extended; gate).
- event build <= 50 ms for 200 items (unit); events <= 2000/item, <= 20000/project (validation).
- font scan on worker, never blocks start-up (QG-01 bound still 0.5 s over 0.2.2); picker "Loading fonts…" until done.
- flat text centre within 2% H preview vs export; 360 text within 1 deg (preview vs export and yaw/pitch math vs v360).
- 1.2.6 projects: export argv byte-identical (golden v1..v4 fixtures).
- 360 video export: no speed target (user decision).
- coverage >= 85 %, feature coverage 100 % for REQ-106..113 (`// covers:`).

## NFR targets (1.2.6 additions)
- every UI frame <= 200 ms (gate): camera drag/zoom + box drag/resize + playback of 360 during export + STT, on 4K x 2K and 8K x 4K generated 360 files (REQ-056 harness extended).
- ESTIMATES (not gates): camera still 4K src < 400 ms, 8K proxy < 700 ms; 360 export >= 0.5x realtime 4K->1080p; sendcmd file for 10 min 30 fps < 100 ms; path lookup O(log n).
- accuracy: view at 3 sample times preview vs export within 1° (picture test with coloured bands), 45° (±1°) at mid 0->90; text size/centre preview vs export within 2% H.
- memory: no frame buffers in videolore; ffmpeg 8K frame ~100 MB x few; command file <= ~64 MB cap (step doubling).
- compat: golden argv for v1/v2/v3 fixtures byte-identical; non-360 projects unchanged.

## security (1.4.0)
- export dialog text untrusted: name rules (no separators, control, reserved Windows names, <= 200 chars, extension from table); folder = existing directory; Path::join of validated name only; target equal to any project media or the .vlproj refused (data-loss guard); overwrite needs confirmation; partial file in target folder then rename.
- ffmpeg args: presets/codecs from enums, clean audio constants; no user string reaches ffmpeg. mp3 only when probed. session memory not stored; no network, no crate.
- v<=5 keys (size, treat_as_360) read only in migration; text width and anchor values validated on load.

## security (1.2.6)
- sendcmd file + filter args built only from validated ints formatted by us; no user/file string reaches ffmpeg or ASS; metadata read = enumerated ffprobe values; POV count/values bounded on load (ADR-065); file in private Work folder (0700, ADR-033), relative name, escaped; hostile 8K/huge dims -> too_large gate (ints u32, no overflow: use u64 products).

## security (1.3.0)
- text/font names untrusted: ASS escape (ADR-020 path), font charset whitelist <=100 chars, colour #RRGGBB, numbers formatted by us; caps 2000/20000 events; font files: regular, <=64 MiB, worker; system fonts parsed by OS/libass/FreeType (trusted, risk noted); anchors validated on load; no network, no unsafe.

## security
threat: malicious media names/contents, malicious project JSON, PATH hijack, tampered downloads (ffmpeg build, model), filtergraph/ASS injection, desktop entry injection.
- no shell; OsString args; file: prefix; protocol whitelist file,pipe.
- bundled ffmpeg = shipped untrusted-media parser: refresh pin every release; SHA-256 pinned at build; settings show path, version, source (bundled/override/PATH). PATH last.
- spawn cwd only our private temp dir.
- ASS escape (ADR-020), text cap.
- model: consent, https only, pinned commit URL, exact size + sha before use, .part->rename; user file must match sha+size before whisper.cpp (C++ parser) loads it. ureq only under stt + only downloader. no telemetry.
- proxy cache: hash names; clear only our pattern in our dir.
- linux desktop: writes only ~/.local/share; Exec quoted per spec; target abs existing file; helpers abs args no shell errors ignored.
- macOS odoc paths -> same open/validation path as user pick.
- project v2: strict serde, size cap, full invariant validation.
- supply chain ADR-025. secrets: none. unsigned installers (Andrea) -> README first-launch steps; signing deferred.
- 0.3.0 (ADR-033/034): settings file untrusted for parsing (malformed -> recommended), trusted at account level for override; FIFO/device project + settings refused (regular-file check, bounded read); probe timeout 30 s; concat control-char guard; display sanitising (control + bidi); deny(unsafe_code) crate-wide w/ 2 allows; downloads https-only pre-connection check + redirect cap 5 + model URL table test + ureq-only-in-downloader test; private work folder 0700 + partial exports 0600 + lock-based crash cleanup restricted to own name patterns, regular non-symlink files; cache/data dirs 0700; cargo-deny yanked=deny; CI least-privilege permissions + SHA-pinned actions.
- 0.4.0 (ADR-037..040): probe input synthetic lavfi (no file access); VA-API device string only from enumerated ^renderD\d+$ char devices, no shell; hw output = untrusted media, decoded only by sw decoder (import risk class); hardware_off fields validated (enum, <=32 chars, <=8 entries); no new crates, no network; GPU drivers = system components, REQ-031 bundled list unchanged; driver crash/hang contained in child (kill + abandon); full system hang inside driver cannot be prevented (documented). follow-up: libva guard reads ffmpeg + libva.so.2 as bytes only (no dlopen/exec of untrusted-by-version lib), bounded to known paths; prevents bundled-ffmpeg core dump on libva < 2.21.
- 1.1.0 (ADR-044/047): dropped paths = same class as File > Import (untrusted media; probe hardening, REQ-057) or File > Open (.vlproj via project::load: regular-file check, 16 MiB bounded read, strict validation, REQ-028 prompt). drops ignored while a modal is open. recent list = untrusted settings data: per-entry validation (string, absolute, <= 1024 bytes, no control/NUL, max 10), display sanitised (control + bidi -> U+FFFD, existing helper), file name shown + full path tooltip, opened only on explicit click through the same load path; missing -> message + removed. non-UTF-8 never stored. layout values finite (0,1) only. no network, no new crates, no unsafe; settings remain trusted at account level (ADR-034). privacy: recent list stores absolute project paths in the user's own config dir (same place as override paths); "Clear list" removes them.
- 1.2.0 (ADR-051/053/054/057): embedded subtitle data = untrusted media content: converted by ffmpeg (separate process) to SRT in private 0700 work folder under fixed name; read_limited 16 MiB regular file; srt::parse total (checked arithmetic, caps 100k cues, MAX_TEXT, finite times); strip_tags + clean_text (control chars) before use; ASS escape() still applied -> track text cannot inject ASS tags; \pos emitted only from validated u16. ffprobe subtitle metadata: codec allow list, language regex [A-Za-z0-9-]{1,16}, title sanitised (control+bidi -> U+FFFD) <= 64 chars, <= 16 tracks, ordinal u8 from our own count (no file-supplied string in argv). project v3: volume <= 100 and Pos <= 10000 else refused; unknown ids refused; subtitle_tracks sanitised (never refuses). SRT output: user-chosen path via save dialog, write_atomic, no shell; text = our cleaned text. no network change ("Use without matching words" offline; model download path unchanged ADR-030/034). no new crates, no new unsafe. REQ-057 corpus: NaN/negative/huge/strings in new fields, 10k tracks, 1 MB titles, SRT corpus (overflow times, 200k cues, binary junk, CRLF/BOM).

## crate shortlist (verified 2026-11-14; Cargo.toml/lock authoritative)
runtime existing: eframe/egui 0.37, cpal 0.18.2 (opt audio), rfd 0.18, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, directories 6.0.0, tempfile 3.27.0.
new runtime: sha2 0.11.1 (dev -> dep; proxy key + model check); sysinfo 0.40.3 (MIT, rust-version 1.95, default-features=false features=[system]); whisper-rs 0.16.0 (opt stt, Unlicense); ureq 3.5.2 (opt stt, rustls); macOS-only objc2 0.6.5 + objc2-foundation 0.3.2 (MIT, minimal features).
tools (CI only, not linked): cargo-packager 0.11.8, appimagetool (pinned tools.lock), cargo-deny 0.20.2, cargo-about 0.9.3, espeak-ng (test fixture).
bundled non-Rust: ffmpeg GPL builds (BtbN, Riedl), Noto Sans Bold OFL-1.1, model ggml-tiny-q5_1 MIT (downloaded, not bundled).
0.3.0 review (QG-01): no new crates, none dropped. all latest 2026-11-14: eframe 0.37 (glow; wgpu heavier, rejected), cpal 0.18.2, rfd 0.18, sysinfo 0.40.3 (dropping needs per-OS unsafe -> keep; now also physical cores), ureq 3.5.2, sha2 0.11.1 (CPU SHA ext), serde/serde_json, thiserror, directories 6, tempfile 3.27, whisper-rs 0.16.0, objc2/winit macOS. downloaded models now 3 (ADR-030), all MIT. CI tools: + cargo-llvm-cov (already used by coverage.sh), qemu-user optional.
0.4.0: no new crates. hw encoders + libvmaf come from bundled ffmpeg (already pinned); drivers/libva/libcuda/libvpl runtimes from system. PE checks linux bundled ffmpeg can load them (file/ldd; ASSUMPTION from BtbN libva runtime loading).
1.1.0: no new crates (runtime or dev). checked 2026-11-15: egui/eframe 0.37.0, winit 0.30.13 (via eframe; no Wayland DnD), egui_kittest 0.37.0 (fallback only, ADR-045). egui built-ins used: MenuBar, menu_button/SubMenuButton, Popup, Window, ScrollArea::show_rows, Modifiers::COMMAND + format_shortcut, AccessKit output for tests. rejected: egui_dock/egui_tiles, muda.
1.2.0: no new crates (runtime or dev). egui built-ins: DragAndDrop (set_payload / dnd_hover_payload / dnd_release_payload), CursorIcon::Copy/NoDrop, FontFamily::Name for bundled Noto Sans Bold (text measure). rejected: srtparse/subparse/srtlib (ADR-054), strsim (ADR-056), isolang (static 639 table), ab_glyph/fontdue (ADR-057), matroska/mp4 demux crates (ADR-053), forced aligners (ADR-056).

1.3.0: +fontdb 0.24.0 (MIT, rust-version 1.71, default features; transitive fontconfig-parser 0.5.8 MIT, memmap2 0.9 MIT/Apache, roxmltree MIT/Apache, ttf-parser 0.25 MIT/Apache) = the only new crate. rejected: font-kit 0.14.3, fontique 0.13.0, per-OS APIs. PE: cargo tree + cargo-deny after adding. checked 2026-11-17 via cargo info (read-only).
1.2.6: no new crates (runtime/dev). v360/sendcmd from bundled ffmpeg (pinned); egui CursorIcon resize + scroll/zoom_delta built in. checked plan time only, not re-verified.

1.4.0 (PLANNED): no new crate (runtime or dev). afftdn, libmp3lame from the bundled ffmpeg (smoke test lists them); best fit by in-house estimate (ttf-parser rejected for now, Future work). checked at plan time from ffmpeg 6.1.1 local filters.

## risks
1.4.0 (PLANNED, plan § Risks): afftdn speech muffling; preview CPU with clean audio; best-fit estimate fill 85–115 %; 1.3 text look after migration (Q1 SEVERE); dialog complexity (headless tests at 800x500); 360 files without metadata stay flat; export choices lost on restart (by design).
1. manual-only checks: macOS launch-time odoc timing, Gatekeeper on nested ffmpeg (fallback `xattr -dr com.apple.quarantine /Applications/videolore.app`), SmartScreen, linux file-manager behaviour. odoc fail -> in-app Open still works. manual checklist in docs/code.md (PE).
2. cargo-packager no release ~1 y -> fallback hand NSIS + hdiutil.
3. Riedl URL retention unknown -> CI fails loudly, re-pin.
4. whisper-rs-sys bindgen/libclang on 3 OS -> build time + CI fragility.
5. non-Latin subtitle scripts -> system fonts (documented limitation).
6. 1.3.0: spherical marker resolved by own MP4/MOV writer (ADR-078 addendum; MKV/WebM unmarked); libass vs fontdb family names (silent fallback in export only); flat text near poles (accepted, Q1=A); slide leaves the view (ADR-077); blink event cost (capped); egui vs libass wrap (as subtitles). detail: ai-docs/architecture-plan-1.3.0.md § Risks.
6. GPL corresponding-source adequacy not lawyer-reviewed.
7. some linux file managers won't run ELF from tar.gz -> README fallback.
8. scope largest so far, long CI -> milestones M0–M7 each green.
9. egui frequent breaking releases (ADR-012) -> thin ui. new stable lints -> fix promptly.
10. ffmpeg build variance for user override/PATH -> startup capability probe + fallback; bundled copy has known caps.
11. H.264/HEVC patent licensing varies by country -> note in notices.
12. (0.2.1) pre-AVX2 x86-64 users: STT crash (ASSUMPTION) -> README/system requirements state min CPU; runtime gate = follow-up candidate (ADR-027). 0.3.0: gate done (ADR-031); residual ASSUMPTION: static ggml init does not run AVX2 at startup (optional qemu check).
13. (0.3.0) model speed/memory + recommendation thresholds are estimates -> adjust after PE measurement (A10).
14. (0.3.0) fingerprint changes on VMs/containers w/ variable cores -> custom path only notifies; recommended path recomputes silently.
15. (0.3.0) flash attention on CPU may slow tiny -> measured, gated.
16. (0.3.0) BtbN daily build pin may expire before re-package -> re-pin month-end.
17. (0.3.0) Windows: in-use file in stale work folder not deletable -> skipped, retried next start.
18. (0.3.0) SEVERE defaults pending Andrea -> RESOLVED 2026-11-14: old-CPU message only (helper dropped), models + thresholds, Light per job, in-app window confirmed; hw encode now built (ADR-037..040). still pending: A1, A10, ADR-035 coverage-run note.
19. (0.4.0) no GPU in CI -> real hw only via QG-05 manual matrix; unverified combos = known limitations (S2).
20. (0.4.0) NVIDIA needs driver >= 610 (SDK 13.1 headers in pinned BtbN); older -> probe fails fast, sw, Advanced explains. likely common on linux LTS late 2026.
21. (0.4.0) linux VA-API/QSV need system libva + vendor driver (intel-media-driver / Mesa radeonsi); Fedora default Mesa lacks H.264/HEVC encode -> AMD needs RPM Fusion mesa-va-drivers-freeworld. QSV linux uses default render node only (other Intel node via VA-API). multi-GPU always prefers NVIDIA.
22. (0.4.0) exit-0 corruption only between sampled frames passes outcheck -> QG-05 d.
23. (0.4.0) PSNR 25 dB threshold + vendor bitrate factors are ASSUMPTIONS -> tune from QG-05.
24. (0.4.0) QG-05 g (>=40%) FAILED on Intel iGPU VA-API CQ (-14% / +41% / +6% vs Best quality): iGPU encode + check (strict decode + full graph render) not faster than x264 veryfast. known limitation for 0.4.0; recommend re-baseline g or segment-only check render (ADR-039).
25. (0.4.0) hw encoders presence in bundled binaries not verified from build scripts alone -> packaging smoke test.
26. (0.4.0) bundled linux ffmpeg 9.0.2 needs libva >= 2.21 (vaMapBuffer2); Ubuntu 24.04 ships 2.20 -> VA-API + QSV skipped -> with bundled ffmpeg on Ubuntu 24.04 Intel/AMD hw never used (only via system ffmpeg override). guard prevents core dump. revisit: pin a BtbN build vs older libva, or bundle libva (adds driver ABI coupling).
27. (0.4.0) VA-API CQ: content-dependent size/quality; QP 23 is near the e limit on some sources (1080p 4.4 Mb/s ~+2.5 interpolated), a restart with coarser QP can push e over 3 on unmeasured content; size held only by watch-restart + 1.5x check (oversize -> sw, not remembered). QSV has no CQ variant.
28. (0.4.0) HEVC VA-API broken on Ubuntu free intel-media-driver 24.1 even at CQ -> HEVC sw on such systems.
29. (0.4.0) libva byte scan ~3.1 s on 173 MB binary (bg, once/session) delays first probe result; not in the probe run budget (10 s since 0.4.2).
30. (0.4.2) probe retry: a slow-failing candidate (HEVC VA-API on Ubuntu free intel driver: ~14 s then fails) costs 5 s per probe and makes the answer inconclusive -> always repeated once -> ~13–20 s bg work + 1 Probe slot held, possible dGPU wake on hybrid laptops, per trigger. no distinction busy computer vs hanging candidate. 10 s total can still end "no hw" under extreme load (repeat mitigates, not eliminates).
31. (1.1.0) drag and drop does nothing on native Linux Wayland (winit 0.30) -> documented limitation; Import/Open menus remain. re-check on eframe/winit upgrade.
32. (1.1.0) egui API churn (MenuBar/Popup/Panel APIs changed in recent releases) -> keep UI thin, logic in pure layout/actions/markers/status modules.
33. (1.1.0) global shortcuts vs text fields (Delete/Backspace/Left/Right/Home/End) -> guarded by egui_wants_keyboard_input(), tested per key.
34. (1.1.0) AccessKit bounds inside ScrollArea are content-relative/clipped -> own UiCache.containers registry; if helper grows > ~150 lines switch to egui_kittest.
35. (1.1.0) in-window menu bar on macOS (D14, ADR-024) looks non-native; Hide/Minimize still missing.
36. (1.1.0) app.rs ~2120 lines and growing with popups/recent/drop -> split into src/app/ submodules later.
37. (1.1.0) 1.0 builds drop layout + recent fields on their next save (downgrade, accepted).
38. (1.2.0) whisper token timestamps ±0.1–0.3 s; 0.3 s alignment gate may be borderline with Fast model (ASSUMPTION) -> fixture w/ clear pauses; PE reports measured error.
39. (1.2.0) egui text width vs libass width may differ by pixels at edge clamp; ASS output authoritative.
40. (1.2.0) text edited after a drag can grow the box past the edge (no re-clamp in export) -> known limitation, Future work.
41. (1.2.0) positioned lines overlapping in time overlap on screen (no collision avoidance).
42. (1.2.0) track cue times assume ffmpeg normalises to file start (ASSUMPTION) -> integration test MKV with start_time != 0.
43. (1.2.0) app.rs growth (2641 lines) -> new flows in src/app/effects.rs + src/app/subs_create.rs; further split = Future work. supersedes risk 36 note.
44. (1.2.0) alignment quality depends on how closely track text matches speech (translated/paraphrased tracks fall back to even spread often); cost ≈ full recognition of the range.
45. (1.2.0) schema v3 always written -> 1.1 cannot open any project saved by 1.2.0 (stated in changelog; accepted, Q5).
46. (1.2.0) start/end fades edge-attached: moving clip 1 elsewhere does not move its start fade (Q1); if users expect it to follow -> Future work.
47. (1.2.6) RESOLVED by spike: yaw/pitch/d_fov commandable; reset_rot=1 needed; cost ~75-100 ms per command -> export ~1/3 real time (accepted, roadmap).
48. (1.2.6) camera latency via ffmpeg stills while dragging 8K; quick in-process view = Future work.
49. (1.2.6) two copies of same source range share POVs; schema v4 -> 1.2.5 cannot open.
50. (1.2.6) CI ffmpeg on Linux = distro apt (6.1): v360 command flags may differ from bundled; tests gated on spike result. BtbN 9.x reset_rot/command semantics not re-measured (assumed same).
51. (1.2.6) toolbar overflow with 15 actions at 800 px; shortcut collisions.

## Milestones 0.2.0..1.2.6 (compacted in 1.4.0 PLAN; history only, decisions are in the ADRs)
## 1.3.0 answers of Andrea (REFINE 2026-11-17; no open questions)
- Q1 = A: 360 text is a flat sticker on the sphere (exact place, one projection engine; narrows toward the poles like any flat picture in a 360 video). Per-view perspective text (B) is not planned.
- Q2 = A: automatic (view-fixed) subtitle lines in a 360 video export stay at their preset place in the frame (lower third, about 30 deg below the front).

## 1.4.1 PLAN (project page, 2026-11-18; no app code, no REQ)
- Scope: github_page/index.html + images (static, no build). docs/architecture*.md: no change (page not mentioned). Notes: no new section needed beyond this one.
- ADR-092 (1.4.1) static page: one HTML file, inline CSS only, no JS, no external scripts or fonts, GIF images kept (WebP/AV1 = Future work: browser support, second asset set). Deployed by pages.yml (paths github_page/**, pinned actions) = OK, no change.
- Verified: 1.4.1 card time = changelog header 2026-11-18 00:01 +0000 (OPEN item closed); crate versions in page = Cargo.lock; ffmpeg 9.0.2 = README badge; images 317-512 px, 46-71 KB each, shown at <=300 CSS px.
- TECH items for PE (github_page/index.html, recommended, no REQ):
  1. img width/height attributes = intrinsic size (sizes above) -> no layout shift.
  2. loading="lazy" decoding="async" on the 5 images below the fold (intro image stays eager).
  3. .timeline-wrap (overflow-x: scroll): tabindex="0", role="region", aria-label "Timeline of versions" -> keyboard scroll (WCAG 2.1.1).
  4. Reduced motion (WCAG 2.2.2): GIFs loop forever; <picture><source media="(prefers-reduced-motion: reduce)" srcset="images/<name>-still.png"> with first frame PNG, no JS. Needs 6 still assets (PE extracts with bundled tooling, not in tests).
  5. Optional SEO: <link rel="canonical">, og:title/og:description/og:url (og:image only with a static PNG, GIF not reliable); optional <link rel="icon" href="data:,"> to avoid favicon 404.
  6. Optional defence in depth: meta CSP `default-src 'none'; img-src 'self' https://github.com; style-src 'self' 'unsafe-inline'; base-uri 'none'; form-action 'none'`; verify the badge still loads (redirects) before shipping.
- Not changed: links have no target=_blank (no rel issue); no third-party scripts; alt texts descriptive.
- Future work: animated WebP/AV1 for GIF weight (~341 KB total today); page-level pause control if 4 is not done.

## 1.6.1 PLAN (2026-11-19; full text in ai-docs/architecture-plan-1.6.1.md)
- ADR-120 paused preview (REQ-144): cause = REQ-034 skip marks (`shown_key`, `posted_key`) not following the newest request (early return without generation bump, superseded/dropped results keep posted_key, play() keeps posted_key). Fix: a target change bumps generation and clears wanted/posted; equal to shown -> bump and cancel; play/seek reset all marks. REQ-034 kept. Rejected: removing the cache (renders per edit burst); render on every pause (fallback only).
- ADR-121 visible edges (REQ-145, 146): `Hit::Edge(Side)` on the visible part of the box edges (pure geometry in ui/frame_handles, ui/boxes); `scale_to_edge` proportional from the normal axis, opposite edge fixed, 10..400% / 25..400%. Rejected: free aspect edge drag (breaks REQ-127); separate widgets.
- ADR-122 whole-video effects (REQ-147, 082): Timeline methods write per-clip state; format 9 keeps no whole-video effect state except Change aspect (frame_aspect). Rejected: keep whole range (two rules forever); render-time virtual layer (no per-clip deletion).
- ADR-123 edge fades (REQ-083): `Clip.fade_in`/`fade_out` (serde default, skipped when false) replace Timeline `start_fade`/`end_fade`; fades_of reads the clip flags; they move and are removed with the clip. Rejected: video-level flags with first/last rule.
- ADR-124 format 9 (REQ-149, 029, 057): SCHEMA_VERSION 9; in-memory conversion of format 8 in new `src/project/v9.rs` (pure, no undo, no change mark); whole Scale/Move -> per-clip copies (keys shifted to clip time + edge samples, cap 200); whole LUT -> each clip; start/end fade -> clip flags; project video level -> each clip level (product, rounded 1%), level 100%. Conflicts with a clip's own value -> Q-F decided 2026-11-19: Scale/Move combined per clip (product of scales, sum of moves, at the key times of both, exact there; over 200 times the clip's own keys kept first); clip's own LUT kept, whole-video LUT not applied to it; one status-line note when a clip changed. 1.6.0 refuses 9 with its newer-version message.
- ADR-125 copies at application (REQ-148): one Scale/Move per clip with a key at 0; whole-video popup = first clip view, changes applied to all copies at the same clip time, one undo; deleting a clip deletes its copy; no stacking (REQ-130), so the composition has one clip track.
- ADR-126 CI time (QG-06): install only missing apt packages (`--no-install-recommends`, retries, 30 s timeout), `timeout-minutes`, cargo-about prebuilt via the pinned install action; optional speech fixture instead of espeak-ng. Measured: 1.6.0 check job 25.2 min (apt 20.0); estimate ~4.5 min (typical ~5.9 min). Rejected: apt cache action (supply chain); parallel executables (untested binaries); dropping checks.
- ADR-127 splash 1.6.1: `assets/splash/1.6.1.gif` = copy of 1.6.0.gif (Andrea 2026-11-19); `1.6.1.txt` = heading + 4 lines (draft in ai-docs/business.md § 1.6.1 REFINE).
- Q-F [SEVERE PRODUCT-CHANGING, closed 2026-11-19]: answer A for Scale and Move, clip's own LUT wins; rule in ai-docs/architecture-plan-1.6.1.md § Decided: Q-F.
- NFR 1.6.1: pause still = one render after 80 ms quiet (no extra render for unchanged picture); CI check job <= 5 min typical (estimate); no new crate; MSRV 1.100 unchanged.
- security 1.6.1: conversion reads untrusted numbers (checked, capped, refused when damaged); no new CI action; no network.
- risks 1.6.1: Q-F rule for rare projects (approximate between key times); CI target depends on the apt mirror (stall outside control, timeouts bound it); users expect clips added later to get the effect (rule documented, no message); edge handles vs picture drag (PE test).

## 1.6.0 PLAN (2026-11-19; full text in ai-docs/architecture-plan-1.6.0.md)
- ADR-105 products: Cargo workspace; root `videolore` (lib + editor) + member `player/` (`videolore-player`) using the lib with default-features=false + audio (no whisper-rs, no ureq). Rejected: second bin in root; `videolore-core` extraction now; second repo.
- ADR-106 player = in-memory one-clip Project + existing preview engine; no save/export/edit commands in its state.
- ADR-107 player: no settings file (recommended levels in memory), writes only cache and work folders.
- ADR-108 player installers: `scripts/package.sh <target> [editor|player]`; own bundle id; no file association; same pinned ffmpeg lock (second copy in installer); player icon = editor icon artwork (Q-A closed = 1, 2026-11-19).
- ADR-109 LUT reader `src/lut.rs`: .cube 3D (2..=65) and 1D (2..=4096, sampled to 3D 33), 16 MiB cap, finite, clamp 0..1, DOMAIN 0..1 only; no crate.
- ADR-110 LUT in graph: normalised cube in Work folder (fixed names), `format=gbrp,lut3d=...:interp=tetrahedral,format=yuv420p`; per segment: picture scale -> whole LUT -> clip LUT -> overlay on background -> fades; 360 after v360; any LUT re-encodes.
- ADR-111 storage: Timeline.whole_lut + clip_luts (serde default, Clip stays Copy); split copies, merge needs equal, format 8; paths UTF-8; damaged -> cleared with note.
- ADR-112 missing file: project opens, LUT inactive, export stops before output naming the file; changed content re-validated at choose and export; sha256 decides preview redraw.
- ADR-113 splash: `src/splash.rs` pure + `ui/splash.rs`; assets via include_bytes!/include_str! of CARGO_PKG_VERSION (missing = build error); image crate gif; Popup::Splash (in-app window, not an OS viewport); skipped with any path argument, double-click first start included (Q-C closed = 1, 2026-11-19; no single-instance code).
- ADR-114 icons: vendored Phosphor Regular TTF (MIT) as fallback of Proportional; constants in ui/icons.rs; ActionId.icon required; cmap test; no egui-phosphor (egui 0.36 pin), no SVG (resvg MPL), no Font Awesome (CC BY).
- ADR-115 notices: rust-crates.txt (editor) + rust-crates-player.txt (player), third-party.txt Phosphor entry, src/notices.rs table, origin of splash image to record (Q-E).
- ADR-116 versions: both Cargo.toml literal; version.sh writes and checks both lock entries and the badge.
- ADR-117 gates: --workspace in CI, coverage ignores both main.rs, feature coverage scans player/, CI check: player graph has no ureq/whisper-rs.
- ADR-118 player open rules: argv[1] video path, File > Open, drop; .vlproj refused by extension without reading; no file association, no odoc.
- ADR-119 player subtitles/camera: shared embedded-track extraction (moved out of app/subs_create.rs, no copy), .srt 16 MiB cap, positions session-only, camera::drag + clamp + live override.
- crate shortlist 1.6.0: + image 0.25.10 (gif feature, MIT/Apache-2.0; already in lock via eframe), + ttf-parser 0.25 as dev-dependency (already in tree via fontdb 0.24). Rejected: egui-phosphor 0.14.0 (egui 0.36), egui-pop-phosphor 0.15 (fork), egui_extras 0.37 (not needed), resvg (MPL-2.0), Font Awesome (CC BY 4.0).
- NFR 1.6.0: splash decode <= 100 ms (estimate); LUT preview playback within 20% of no-LUT at 1080p (assumption); LUT export <= 1.5x re-encode (assumption); player no network (CI); new logic in new modules (app.rs 3515 lines, render.rs 3227 lines: wiring only).
- security 1.6.0: LUT input untrusted (caps, strict parse, fixed names in Work folder, no user text in filters); player media untrusted (same ffmpeg model), never writes next to video, never reads .vlproj; assets compiled in; Phosphor font SHA-256 pinned.
- risks 1.6.0: lut3d options on bundled 9.x unverified (PE step 3); yuv round trip 1-2 levels (test tolerance); splash GIF has one frame, kept static by decision (Q-B closed = 1); splash summary missing = build error (intended); player duplicates ffmpeg (size); 1.5.x refusal of format 8 checked by hand only.
- open questions 1.6.0 (REFINE 2026-11-19): Q-A closed = 1 (player uses editor icon artwork); Q-B closed = 1 (splash GIF kept as supplied); Q-C closed = 1 (no splash with any path argument; PM flag: REQ-133 "at every start" needs this exception); Q-D [TECHNICAL] player carries own ffmpeg copy (decided); Q-E [TECHNICAL, open before release] origin/licence of assets/splash/1.6.0.gif.

