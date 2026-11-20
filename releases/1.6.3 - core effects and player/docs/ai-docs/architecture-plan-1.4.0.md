# ai-notes: architecture plan 1.4.0 (PLAN 2026-11-17, target 1.4.0)

Owner: architect. Index: ai-docs/architecture.md. Requirements: ai-docs/business.md "1.4.0 PLAN" + docs/business-{editing,export,music,picture,text,ui,subtitles}.md (REQ-114..123 new; 022, 023, 069, 075, 089, 097, 100, 103, 106, 111, 117 reworded/planned).

## Code facts (read at plan time)
- format.rs: all_identical, choose_output (lowest pixels, fps, bitrate), EncodeSettings{muxer,extension,vencoder,vextra,acodec,aencoder}; mp3 -> libmp3lame (l.72), fallback to aac when encoder missing (l.247).
- timeline.rs Clip{id,media,src_in,src_out,volume:u8,muted:bool} (Copy, Eq, serde defaults).
- text.rs TextItem{id,text,start,ttl,effects,colour,font,place:Pos,size:u16 (SIZE_MIN..MAX 250..4000),anchor:Option<Anchor360>}; font_px(h,size); estimate_box(item,w,h) = pure estimate (char 0.6 em, line 1.2 em, margins from ass::margin_lr) already used by export slide distance.
- render.rs: segments(), build_graph(segs,bed,GraphSpec), plan_export / plan_export_mode, export_args, preview_audio_args, music_bed, ExportMode (l.62, ADR-072).
- project.rs SCHEMA_VERSION = 5 (l.26); media entries carry treat_as_360:Option<bool> (1.2.6) and spherical:Option<bool>.
- ui/menu.rs Effects menu (l.~93) holds Fades, Subtitles, Volume, Apply ▸ x2, ToggleFadeAtPlayhead, GenerateSubtitles, AddSubtitleLine, AddText, AlignH, AlignV, (Reset). Target: Effects = effects + Add text + Apply ▸; item commands -> Edit menu.
- app.rs Modal enum (l.189) holds dialog modals (SubtitleSource, ConfirmRegenerate...). src/export/ holds only hw_tests.rs (worker is src/export.rs).
- ffmpeg 6.1.1 (local apt) has afftdn, anlmdn, arnndn (needs model file), agate, highpass as core filters: no external library. Bundled 9.x not checked (PE checks in smoke test).

## Components and changes (1.4.0)
| Module | Change |
|---|---|
| timeline | Clip.clean_audio:bool (#[serde(default)], always written). set_clean_audio(ids,on) 1 undo; split/copy/paste keep; merge needs equal flag: "The clips have different clean audio settings". |
| render | Segment.clean:bool; audio chain per segment: afftdn (const params) BEFORE clip volume and fades; plan reencode when any clean; GraphSpec::AudioOnly (no video branches, -vn); preview_audio_args uses same chain; golden argv unchanged when no clean clip. |
| format | VIDEO_PRESETS: const table of 12 (w,h,fps,codec H264|HEVC,container MP4|MOV) mapped to MediaFormat/EncodeSettings; audio choice -> acodec/aencoder; mp3 only if encoder probe lists libmp3lame. |
| export (pure, NEW src/export/choice.rs) | ExportChoice{mode:Full|AudioOnly|Srt, video:Same|Preset(u8), audio:Same|Aac|Mp3, sphere:Option<Sphere360|Flat>, folder, name}; file_name_ok(name) ; with_extension(name, ext) (replaces known ext only); target_path; clash_check(path, project_files, media paths); reasons (greyed text); plan(project, choice) -> ExportRequest{Video(plan,out)|AudioOnly(plan,out)|Srt(path)}. |
| ui (NEW ui/export_dialog.rs) | egui::Window as Modal::Export(ExportDialog) (keymap off while open, Esc closes); file field + "..." (rfd folder), mode, video/audio dropdowns with reset buttons, 360 choice when all video clips 360, Export/Cancel; overwrite confirm = Modal::ConfirmOverwrite(path). |
| app | Command::OpenExport, StartExport(ExportChoice), SetCleanAudio{target,on} (also ApplyEffect CleanAudio), ResizeTextWidth{id,width,dragged}, Align/Reset accept ObjRef::Text; Set360 REMOVED; AppState.export_memory:Option<ExportChoice> (session only, not in project/settings). Ctrl/Cmd+E and toolbar Export open dialog; while exporting still cancel. |
| text | TextItem.width:u16 permille 100..=1000 replaces size; box height fixed 200 permille of H; fit() pure best fit (ADR-087); Anchor360 {yaw,pitch,width_cd,height_cd} replaces size_cd/wrap_cd semantics (names kept in code only if PE prefers; schema names in v6 = width_cd,height_cd). |
| placement / ui boxes | Flat text + subtitle drag: centre clamped to [0,W]x[0,H] (ADR-089); text box from fit() estimate (not egui), so hit-test = export; width handle on right edge (resize_width). 360: limit = view (unchanged). |
| ui menu / actions | Effects menu = effects; Edit menu gets Generate subtitles, Add subtitle line, Add/remove fade, Align H/V, Reset, Reset subtitle position. Shared ActionId table unchanged except CleanAudio (+ shortcut none). Context group shown for text selection too (REQ-106). |
| settings | unchanged (export choices never stored). |
| project | SCHEMA_VERSION 6, written always; loads 1..6; TooNew >6. v<=5 keys `size`, `treat_as_360`, Anchor size_cd/wrap_cd read only in migration; strict serde must accept them for v<=5 only. |
| packaging / CI (PE) | smoke test: -filters has afftdn; -encoders has libmp3lame (ADR-013 list). |

## ADRs (1.4.0)
ADR-080 (1.4.0) Clean audio = FFmpeg afftdn per video clip, in the clip's own audio chain before clip volume and fades; constants in one function (nr, nf, tn; start values nr=12 nf=-30 tn=1 ASSUMPTION, tuned by PE against the REQ-114 gate, recorded in docs/code.md). REQ-114. TECHNICAL.
  alt rejected: anlmdn (non-local means, heavy CPU for preview), arnndn (needs a downloaded model file, not bundled, licence and size), agate/highpass (do not remove noise under speech). reason: core filter in every build, no model, cost measured once.
ADR-081 (1.4.0) clip flag clean_audio (schema v6). Whole video = set on every video clip (one undo). Music untouched (REQ-020). TECHNICAL.
ADR-082 (1.4.0) Export dialog: pure choice model (src/export/choice.rs) + egui modal (ui/export_dialog.rs); session memory only; overwrite confirm; refuses a target equal to any project media file or the project file (data-loss guard, new). REQ-118/119. TECHNICAL.
  alt rejected: export settings stored in project (Andrea M14: not stored), in Settings (M7).
ADR-083 (1.4.0) video presets = fixed table in format.rs, 12 entries (ASSUMPTION list, shown with technical names per Andrea's example): 1920x1080 25/30/60 fps libx264 .mp4; 1920x1080 25 fps libx264 .mov; 1920x1080 25/30 fps libx265 .mp4; 1280x720 25/30 fps libx264 .mp4; 1280x720 25 fps libx265 .mp4; 854x480 25 fps libx264 .mp4; 3840x2160 25 fps libx265 .mp4; 3840x2160 30 fps libx265 .mov. Always re-encoded; H.264/HEVC so hardware export applies (REQ-055). Pictures letterboxed, never stretched (REQ-022). REQ-120. TECHNICAL.
  alt rejected: free resolution/bitrate fields (Andrea/PM: fixed list; not asked).
ADR-084 (1.4.0) audio choice: Same as source = today's automatic audio (AAC; ASSUMPTION to verify on the code path), aac, mp3 (libmp3lame, only when probed). Only audio output: AAC -> .m4a, MP3 -> .mp3; Same -> AAC .m4a. Only audio = audio graph of the whole timeline (video sound, music, volumes, clean audio) with -vn: GraphSpec::AudioOnly, no video decode. REQ-119/121. TECHNICAL.
ADR-085 (1.4.0) 360 choice in dialog only when ALL video clips are 360 (mixed -> flat silently); default 360 video (ASSUMPTION); replaces the 1.3 question before each export (ADR-072 modal removed). REQ-122/103. TECHNICAL.
ADR-086 (1.4.0) 360 switch removed: no Set360 command, no per-media override. is_360 = spherical metadata only (spherical.unwrap_or(false)). Load drops treat_as_360 silently (no undo, not dirty). Consequence: a 360 file without spherical metadata stays flat (documented product risk, Andrea decided M12). REQ-100/123.
ADR-087 (1.4.0) text box and best fit: flat box width 100..=1000 permille of W (default 800), height fixed 200 permille of H. Best fit = pure text::fit(item, W, H): largest font size f with rows(f)*1.2*f <= box_h, rows = sum over lines of ceil(chars*0.6*f / wrap_w) (wrap_w = box_w - 2*margin_lr), f in [2% H, box_h/1.2], binary search 40 steps; below minimum -> 2% H (text runs past bottom). Preview and export call the same function with export geometry -> equal by construction, no stored size, no font bytes. Anchored text: box in equirect px of its 360 segment (width_cd 1000..=36000 = 10..360 deg, height_cd fixed 20% of view vertical at add time), same fit. REQ-111/116. TECHNICAL.
  alt rejected: egui measurement (preview only; export mismatch; ADR-057 path already measures subtitles with egui); ttf-parser glyph widths (new direct dependency, per-item font bytes in export; Future work if estimate is visibly off); stored size (derived state, load migration needs measurement).
  risk: estimate 0.6 em vs real Noto Sans Bold widths: text may fill 85-115% of the box; accepted, measured in QG.
ADR-088 (1.4.0) schema v6 migration (in memory, same result except measured text look, see Question Q1): flat text: width = smallest w in [100..1000] whose fit() >= old font px (old = font_px(H,size)), else 1000; anchor: width_cd = wrap_cd, height_cd = lines(old size, wrap) * size_cd * 1.2; clip clean_audio=false; treat_as_360 dropped. REQ-123. TECHNICAL (pending Q1).
ADR-089 (1.4.0) drag limit for flat text and subtitle lines: centre clamped to the picture (box may be partly outside; drawn clipped by libass and egui clip). Pos stays 0..=10000 -> no schema change for positions. 360: view limit (REQ-112 unchanged). REQ-089/117. TECHNICAL.
ADR-090 (1.4.0) menus: Effects menu = effects + Add text + Apply ▸; item commands in Edit menu (same ActionIds, placement only). REQ-069/075. TECHNICAL.
ADR-091 (1.4.0) Context Menu and Align/Reset apply to any selected text item (ObjRef::Text) through the same commands; Reset of a text = default place (lower third centre) and default width 800 permille. Width handle: right edge only, left edge fixed, width clamped 100..1000 permille, one drag one undo. REQ-106/115/116. TECHNICAL.

## Schema v6 (REQ-123)
- clips[]: + clean_audio (bool, always written).
- texts[]: - size; + width (u16); anchor: - size_cd, wrap_cd -> + width_cd (1000..=36000), height_cd (>0, <= 18000).
- media[]: - treat_as_360 (never written); spherical kept.
- Validation: width 100..=1000; anchor width/height ranges; clean_audio bool. Other rules as v5.
- Golden: v1..v5 fixtures (tests/fixtures) load; export argv for each identical to 1.3.0 EXCEPT text items (size migration) and 360 projects with a switch (manual). Non-text, non-clean, same-format projects: byte-identical argv.

## Export dialog flow
- Command::OpenExport (File > Export…, toolbar, Ctrl/Cmd+E): exporting -> CancelExport (unchanged). Else Modal::Export with memory or defaults: name = project name (project folder) first time; folder = project folder, then last export folder of session, then user documents folder (ASSUMPTION).
- Disabled with reason: empty/invalid name, folder missing/not a directory/readonly, clash with project media or .vlproj path, Only subtitles with no subtitles ("No subtitles on the timeline"), Only audio with no clips.
- Export: if target exists -> Modal::ConfirmOverwrite -> StartExport(choice) -> plan(project, choice) -> existing worker (video) / audio-only job (same partial+rename rule, same Scheduler slot) / SRT write (ExportSrt path).
- File name rules: no / \ : * ? " < > | or control chars or NUL; not . or ..; no trailing dot or space; not Windows reserved (CON PRN AUX NUL COM1-9 LPT1-9); <= 200 chars; extension replaced if it is one of mp4 mov m4a mp3 srt, else appended.

## NFR (1.4.0)
- Clean audio gate (REQ-114 a): noise-only parts >= 6 dB lower, speech parts within 3 dB; measured by PE on committed fixture (espeak-ng speech + generated wind/pink noise, <= 300 KB), ASSUMPTION thresholds.
- Preview with clean audio on a 1080p clip: no audio correction pad/drop events over 60 s playback on the reference laptop (ESTIMATE target; PE records measurement).
- Interface frame <= 200 ms with clean audio playback + export + subtitle generation (REQ-104 harness extended).
- Dialog opens <= 100 ms (unit/UI test); folder check is one metadata call.
- Best fit: 200 text items fit in < 10 ms (unit).
- No change to golden argv of projects without new features.
- Coverage >= 85 %; feature coverage 100 % for REQ-114..123 (implemented ones).

## Security (1.4.0)
- Dialog text = untrusted input: file name rules above; folder typed in the field must be an existing directory; target built with Path::join of a validated name only (no separators possible).
- Source/project clash refused (new data-loss guard); partial file in target folder then rename (existing); never overwrites without confirmation.
- ffmpeg args: presets from const table, clean audio constants, codec/fps from enums; no user string reaches ffmpeg (keeps ADR-002 rules).
- mp3 only when probed encoder list has it.
- Session memory is not stored; no network, no new crate.
- Text width/anchor sizes validated on load (no value reaches ASS unvalidated).

## Crates
- No new crate (runtime or dev). afftdn, libmp3lame, presets and fit use existing code and the bundled ffmpeg. Rejected for later: ttf-parser direct dependency (ADR-087 alternative).

## Ordered steps for the Principal Engineer
0. Baselines BEFORE change: golden argv fixtures v1..v5 (existing) + one 1.3.0 project with texts, clean-free clips and 360 switch; headless frame time; afftdn timing on a 60 s 1080p clip (for the NFR); bundled-ffmpeg smoke for afftdn/libmp3lame (report only).
1. Schema v6 + migration + validation (REQ-123, REQ-100 switch drop, REQ-113 regression): fixtures v1..v5 load; v6 round trip; TooNew >6; migrated text width search tests.
2. Timeline clean flag ops + history labels (REQ-114: split/copy/merge/one undo).
3. text::fit + Anchor width/height + estimate box use (REQ-111/116 pure: monotone in width, 2x lines smaller, 3x longer lines smaller, min 2% H, 200 items < 10 ms).
4. render: clean audio chain, plan reencode, GraphSpec::AudioOnly, preview audio chain (REQ-114 b-d; golden unchanged without clean).
5. ffmpeg integration: clean gate on fixture (REQ-114 a); audio-only length and music present (REQ-119 acceptance); presets argv (REQ-120: 1280x720 25fps libx264 .mp4 size/fps/codec; hvc1 for libx265 mp4).
6. format presets + audio mapping + encoder probe (REQ-120/121 pure tests).
7. export/choice.rs pure: names, extensions, clash, reasons, plan mapping, 360 rule (REQ-118, 119, 122).
8. app: OpenExport/StartExport/ConfirmOverwrite, memory, SetCleanAudio, text Align/Reset/width commands, drag clamp, remove Set360 + old 360 ask modal (REQ-115, 116, 117, 100, 118).
9. UI: export_dialog.rs; menu.rs Effects/Edit split; Clean audio row + popup + marker; text group + width handle; Clip details without 360 switch; headless tests (800x500 reachability, greyed reasons, Ctrl+E) (REQ-069, 075, 106, 114, 115, 116, 117, 118).
10. packaging smoke list (afftdn, libmp3lame).
11. Tests replaced where their meaning changes (text size, 360 switch, Effects menu placement, export 360 ask), each replacement named in the report; no test deleted to go green.
12. Quality suite (PE): fmt, clippy -D warnings, cargo test, coverage >= 85, feature coverage 100 (`// covers: REQ-114..123`).

## Tests per REQ (summary)
- REQ-114: timeline pure (flag ops, merge), render argv (afftdn before volume; none without clean), ffmpeg gate, UI popup remove.
- REQ-115/116: pure resize_width, align/reset text, group rule, fit monotone; UI greyed reasons.
- REQ-117: clamp centre; partly-outside box clipped in export (ffmpeg test: left 30% outside cut at edge, preview vs export within 2% H).
- REQ-118: name rules, extension replacement, clash, folder missing, overwrite confirm, memory across reopen (UI headless).
- REQ-119: audio-only length, music present, Srt same text as File > Export subtitles, greyed reasons.
- REQ-120/121: preset entries and argv; reset Same; mp3 only when probed.
- REQ-122: 360 choice only when all video clips 360; mixed exports flat.
- REQ-123: migration fixtures, v6 round trip, TooNew.
- REQ-069/075/106: menu placement table, context rule with text selection, Ctrl+E.

## Risks
1. afftdn muffles speech or leaves noise: gate measured; params tuned; risk recorded.
2. Preview CPU with clean audio on weak computers: may underrun; mitigation options (lighter params in preview only, Future work).
3. Best-fit estimate vs real glyph widths: fill ratio 85–115 %.
4. 1.3 text look after migration (Q1).
5. Dialog complexity in egui (headless tests required, 800x500).
6. Preset table: 4K and HEVC re-encode time; no speed target.
7. mp3 in MP4 players compatibility (ASSUMPTION; MOV/MP4 both offered).
8. 360 files without spherical metadata stay flat with no switch (Andrea M12; documented).
9. Export choices lost on restart (Andrea M7/M14).

## Questions
- Q1 CONFIRMED by Andrea 2026-11-17 = A (migrate by search, ADR-088). Options were: [SEVERE PRODUCT-CHANGING] text items made in 1.3.0 after opening in 1.4.0 (REQ-123 says "same result"; the size control is replaced by best fit). Options: A) migrate by search: same look for one-line and most texts, big multi-line texts shrink to fit the 20% box (default, implemented by PE unless Andrea chooses otherwise); B) keep the old size exactly by a legacy size and no best fit for these items (contradicts REQ-116 for them); C) reset 1.3 text items to the 1.4 default (80% width, best fit), simplest, look changes for all. Consequence: A/C change the look of existing text items; B keeps two kinds of text.
- Answer: A (confirmed 2026-11-17). Open questions: none.
- Technical decisions taken (no question): ADR-080..091.

## Future work
- Measured best fit (ttf-parser) if the estimate is visibly off (ADR-087).
- Lighter clean-audio parameters for preview if CPU requires.
- 360 files without metadata: per-import choice (roadmap).
- Export speed for 360 and 4K presets (roadmap).
- Compaction: milestones 0.2.0..1.2.6 removed from ai-docs/architecture.md in 1.4.0 PLAN; the older ADR-001..079 list there is still long: move it to ai-docs/architecture-decisions-history.md at the next PLAN (owner: architect).
- Docs-check: docs/architecture-export-dialog.md is new; the documenter adds it to docs/architecture.md (done by architect in this PLAN).
