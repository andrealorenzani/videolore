# ai-notes: plan 2.0.4 — all remaining batches in one development (owner: architect; index in ai-docs/architecture.md)

Scope: ONE development 2.0.4 (D57): batches 5–15 and the continuous items; REQ-180..299 (120 REQs, all `(planned)`); QG-11..18 (no covers). Stages (ordered, one Principal Engineer call each): stages 0–5 in [architecture-plan-2.0.4-stages-1.md](architecture-plan-2.0.4-stages-1.md), stages 6–12 in [architecture-plan-2.0.4-stages-2.md](architecture-plan-2.0.4-stages-2.md). ADR-168..201 (in [architecture-adr.md](architecture-adr.md)). Format 11 (D58) is bumped in stage 1. Questions Q-1..Q-7 are closed by Andrea (D61) and recorded in § 6. Archived in the release snapshot after release.

## 0. Facts found by reading (2026-11-21, tree version 2.0.4, HEAD 86ea398 = 2.0.3 built)
- Schema: `SCHEMA_VERSION` 10 (src/project.rs:30); loads 1..10; newer refused (TooNew); `MAX_PROJECT_BYTES` 16 MiB (project.rs:33). `MediaEntry` {id, path_abs, path_rel, name, duration, format} (project.rs:169): no file size or modification time, so REQ-186 is new stored data.
- Camera paths are keyed by media: `CameraPaths { map: BTreeMap<MediaId, Vec<Pov>> }` (camera.rs:172); `Pov { t, cam }`; `Camera { yaw_cd, pitch_cd, fov_cd }` (no roll, no ease). REQ-268 needs per-clip paths.
- `Clip` (timeline.rs:33): id, media, src_in, src_out, volume u8, muted, clean_audio, fade_in/fade_out bool. `MusicClip` (music.rs:25) is `Copy`: id, media, src_in, src_out, start (no volume, fades or clean flag).
- Preview: no `hwaccel` anywhere in src/. Playback = one ffmpeg video process and one audio process per run (`spawn_pair`, preview.rs:393–424). Stills = one ffmpeg run per still (preview.rs:535, 651).
- Hardware: `Codec { H264, Hevc }` (hwenc.rs:32); `VIDEO_PRESETS` has 12 entries (format.rs:170); the check builds full check commands (outcheck.rs).
- Settings: `MAX_SETTINGS_BYTES` 64 KiB (settings.rs:26); recent list 10 entries of 1024 bytes.
- Fonts: assets/fonts holds only NotoSans-Bold.ttf (+ OFL.txt, SOURCE.txt). fonts.rs holds families and the background scan; face bytes are read by base/font.rs (2.0.3).
- GUI: eframe features glow, default_fonts, wayland, x11. The `accesskit` feature of eframe is OFF; egui's accesskit is on (cargo tree: accesskit 0.25.2 via egui default). `accesskit_winit` is not in Cargo.lock.
- image crate: feature `gif` only (no png in the tree).
- `unsafe`: forbidden crate-wide, allowed only in stt::whisper and desktop::macos (ADR-034).
- Sizes (lines, 2026-11-21): render.rs 3553, app.rs 3124, stt.rs 2159, project.rs 1866, timeline.rs 1711, text.rs 1501, preview.rs 1362, ffmpeg.rs 1185, ui/timeline.rs 1100, export.rs 1114. render/ holds only golden_tests.rs; app/ holds the submodules listed in components.
- Frame blur: `BLUR_DIVISOR` 8 (frame.rs:29). REQ-222 says "a quarter of the frame size": mismatch to measure in stage 6 (Future work).
- Business: REQ-180..299 `(planned)`; next free ID REQ-300; REQ-177 untouched; REQ-245 English only (D60); D57 one development; D58 format 11 one bump.
- Stale notes fixed by this plan: ai-docs/architecture.md said "2.0.3 PLAN" (2.0.3 is built, format 10 in code). Human docs keep "(planned)" wording for 2.0.2/2.0.3 items until the release clean-up.

## 1. Approach
- Stage order = roadmap order (suggested order of the Product Manager). Stage 0 is an addition with no REQ: a no-behaviour split of render.rs and app.rs (QG-16 part) so that stages 3, 10 and 11 do not grow 3000-line files. Golden tests (argv byte-identical) and the module-cycle test guard it.
- Each stage ends with the full suite (fmt, clippy -D warnings, test, coverage ≥ 85%, feature coverage 100%), run by the PE only. Each new REQ has at least one `// covers: REQ-NNN` test; amended REQs change their tests with them (business-decisions risk list).
- Compatibility gates: a project without the new features exports with the same argv as 2.0.3 (golden); 2.0.3 projects load unchanged (format 10 → 11 in memory); format 11 is refused by 2.0.3 and older (D11).
- Architecture rules kept: no new process type (all ffmpeg/ffprobe work through `jobs`), no shell, no network except the consented speech model, UI thread does no I/O beyond the existing capped reads, untrusted input bounded before use, one undo step per user gesture, program settings never in projects (C3).
- Documentation per stage: the PE updates docs/code-index.md and docs/code-architecture.md (module list, layers, dependencies) for the modules it adds or moves; the architect verifies them at release (step 2.4 of /release). The architect's human docs for 2.0.4 are docs/architecture-2.0.4-overview.md and -quality.md; the documenter runs after each stage (rule 1).
- Manual checks (business-verification items 9–28) are a checklist, never a blocker; the PE records the automated part.

## 2. Components (new and changed, by stage)
| Module | Change | Stage |
|---|---|---|
| `render` (render.rs) | split by area without change (0); separate subtitle stream (1); blur choice, LUT strength, crop, speed, per-clip frame, picture overlay, volume keys, fade length and sides, clean strength, enhance (11); 360 speed-first and spatial sound pre-pass (10); blur background cache input (6) | 0, 1, 6, 10, 11 |
| `app` (app.rs, submodules) | dispatch split by area (0); autosave tick and recovery (2); keyframe copy and paste, apply-to-new-clips offer (3); single window poll (7); player-open helpers (9); camera per clip (10) | 0, 2, 3, 7, 9, 10 |
| NEW `autosave` | pure policy (due, newer than save, names) and writer on a worker (2) | 2 |
| `history` | forget snapshots that reference a removed media (2) | 2 |
| `timeline`, `music` | trim by edge, fade move, fade length and sides, speed, crop, per-clip frame, music volume, fades, clean (2, 11) | 2, 11 |
| `subtitles`, `srt`, `ass` | VTT read and write, import, ASS file export with place and size, re-time window (1); re-anchor (10) | 1, 2, 10 |
| `placement` | snap to guides and safe-area rectangles (2) | 2 |
| `keyframes` | copy and paste of keys, easing flag, volume track (3, 11) | 3, 11 |
| `media` | file size and modification time per media; re-analysis on open (1) | 1 |
| `project` (format 11 fields; a `project::v11` conversion module only if the 10 → 11 migration of stage 10 grows beyond defaults) | format 11 bump, migration in memory, MAX_PROJECT_BYTES 96 MiB (11) | 1, 10, 11 |
| `export`, `export/choice` | subtitle mode (burned or separate track), 360 choice greyed presets, speed-first 360, note (1, 10); codec names and warning (6) | 1, 6, 10 |
| `format` | AV1 and VP9 presets (5); plain names with encoder in tooltip (6) | 5, 6 |
| `hwenc` | Codec gains Av1 and Vp9; per-vendor candidates; busy or hanging classification; probe activity (4, 5) | 4, 5 |
| NEW `hwdec` | decode probe and decode arguments per vendor; preview decoder choice (5) | 5 |
| `outcheck` | samples-only normal-method render (4) | 4 |
| `jobs` | pause of the check while a job runs; Background priority for imports, proxies and subtitle preparation (4, 6) | 4, 6 |
| `preview` | hardware decode with Faster; restart at the playhead on decoder failure; blur background cache; camera half-resolution while dragging (5, 6, 10) | 5, 6, 10 |
| `proxy` | audio kept in preview copies (9); cache limit, protected open project, clear re-queues (12) | 9, 12 |
| `settings` | program settings additive fields (VERSION stays 2): window, popups, sections, workspaces, toolbar, shortcuts, theme, text size, graphics card, favourites, splash option, cache limit (4, 5, 8, 11, 12) | 4–12 |
| NEW `instance` | one window: lock file with std `File::try_lock`, inbox folder, heartbeat (7) | 7 |
| `desktop` | Linux desktop entry for the archive, macOS menu bar (7) | 7 |
| `base` (src/base/) | file names for autosave copies, the inbox and lock, player settings and cache copies (`base::names`); private folder helper for the inbox (`base::private`); `base::io`, `base::font`, `base::pos`, `base::language` unchanged; base uses no module of videolore (kept) | 1, 2, 7, 9, 12 |
| `notices`, `packaging` | BOM for third-party.txt, new fonts and crates in notices, player notices (7, 9, 11) | 7, 9, 11 |
| `fonts`, `base/font` | added face files (Arabic and Devanagari, bundled, OFL; CJK from system fonts only, Q-3 closed), coverage check of installed fonts per script, note when no font covers a script (1) | 1 |
| `camera`, `spherical` | per-clip paths, roll, ease, initial view, MKV and WebM marker, sendcmd at most 10 per second (10) | 10 |
| `lut` | `.3dl` and `.csp` readers, DOMAIN scaling, copies and strength (11) | 11 |
| `frame` | crop, custom ratio 1:4..4:1, blur level, per-clip frame size (11) | 11 |
| `pictures` NEW | picture items (PNG) model, limits, embedded copy (11) | 11 |
| `splash` | `should_show` decision by option and last version (12) | 12 |
| `ui` | layout (sections, docking), workspaces, toolbar and shortcut editors, theme (`ui/theme`), text table (`ui/strings`), keyframe timeline widget (`ui/keyframe_timeline`), subtitle list UI and fonts, export dialog labels and notes, clip details speed and crop, LUT popup strength and favourites, Clean audio popup, camera keys and arrows | 1–11 |
| `player` (player/src) | settings file (`settings`), playlist, open in player and editor, picture subtitles through the preview graph, About from its own notices | 9 |
| CI and packaging | path filters and `needs` for the installer job, weekly advisories workflow, named CPU-message test, NSIS pin, Linux archive launcher and desktop entry, Windows licence BOM, player associations, page image and CSP | 7, 12 |
| `github_page/` | og:image still, CSP meta, badge and download hosts (12) | 12 |

## 3. Project format 11 (one bump, stage 1; D58)
- 2.0.3 and older refuse it (TooNew, never written). Writer: format 11 from stage 1 on; the release is the only user-visible version.
- Stage 1 (bump): `MediaEntry.file_size`, `MediaEntry.modified_ms` (absent = unknown; a migrated project records the stats without re-analysis, so no analysis count changes on first open).
- Stage 10: `Clip.is_360` (explicit 360 mark, REQ-263); `Clip.camera` (per-clip POVs; migration copies each media path of format 10 into the clips that use it, REQ-268); `Pov.roll_cd` (default 0, REQ-270); `Pov.ease` (segment after this POV is eased, REQ-271); `Clip.spherical_projection` only if the metadata copy needs it (REQ-262; decided in the spike).
- Stage 11: `Clip.fade_in_len`, `Clip.fade_out_len` (Option<Micros>, absent = the 0.5 s rule, REQ-274); fade sides on existing flags (REQ-275; PE verifies the boundary mapping); `Clip.speed_permille` (500..=2000, default 1000, REQ-281); `Clip.crop` (four permille values 0..=450, REQ-283); `Clip.frame` (own shape, fit and empty space, REQ-286); `Clip.volume_keys` (Vec of Key<u8>, REQ-280); `Key.ease` (bool, REQ-282); `Clip.clean_strength` (u8, default 100) and `Clip.speech_enhance` (bool, REQ-278/279); `MusicClip.volume`, `muted`, `fade_in`, `fade_out`, `clean_audio` (REQ-276/277); `Timeline.lut_strength` and `Clip.lut_strength` (u8, default 100, REQ-289); blur level on the frame value (default Medium, REQ-284); `Timeline.pictures` (PictureItem list; PNG bytes embedded as base64 text, ≤ 8 MiB each, ≤ 20 items, REQ-287); `lut_copies` (map from sha-256 to the LUT text, REQ-293; ≤ 20 MB per LUT, ≤ 64 MiB total).
- Caps: project read 96 MiB (was 16 MiB; ADR-191); all other bounds per ADR-168..201.
- Fields added after stage 11's writer are additive with serde defaults; no second bump. Program settings (stages 4–12) never touch the project.

## 4. Stage order and dependencies
| Stage | Batch | REQs | Depends on |
|---|---|---|---|
| 0 | QG-16 part (no REQ) | none, no behaviour change | — |
| 1 | 5 subtitle files and fonts; format 11 | 180–188 | 0 |
| 2 | 6 editing gestures and autosave | 189–199 | 0, 1 (writer of format 11) |
| 3 | 7 whole-video and keyframe clarity | 200–205 | 0 |
| 4 | 8 faster hardware export | 206–211 | — (independent of 1–3) |
| 5 | 9 hardware reach | 212–219 | 4 (check base), 0 |
| 6 | 10 export and preview speed | 220–224 | 5 (preview.rs shared) |
| 7 | 11 platforms and installers | 225–233 | — |
| 8 | 12 interface | 234–247 | 3 (popups), 7 (settings window geometry, single window) |
| 9 | 13 video player | 248–256 | 7 (installers, sibling programs), 1 (preview copy audio uses the same cache) |
| 10 | 14 360 video | 257–273 | 0, 1 (format 11), 3 (keyframe widget) |
| 11 | 15 effects and frame | 274–294 | 6 (blur cache keys), 10 (camera and keyframe model), 0 |
| 12 | continuous quality | 295–299, QG-11..18 | 11 (favourites in settings), 2 (cache), 1 (fonts measured) |

- Critical path: 0 → 1 → 3 → 10 → 11 → 12. The PE runs the stages in the table order; no two stages in parallel.
- Each stage is one PE call: implement, run the full suite, fix the code until green, report the counts and the feature coverage, then hand over. A stage may be split in two calls only if the PE reports why.

## 5. Cut-back to the roadmap if infeasible (in this order of preference)
1. REQ-273 spatial sound: built only if the rotation runs in real time in the preview (CPU of the audio path within 10% of one core, estimate) and the export pre-pass takes at most the clip's own duration (estimate); else roadmap (business decision, REQ-273 text).
2. REQ-261 MKV and WebM 360 marker: the in-house writer is the decided path (Q-7 closed); cut back only if the writer cannot pass the ffprobe check, and the PE reports.
3. REQ-287 picture items: if the overlay in the preview graph breaks the 200 ms frame gate (cut to roadmap; the model stays).
4. REQ-253 picture-based subtitles in the player: if the overlay in the player's graph is not done in stage 9 time.
5. REQ-216 device choice: keep Automatic only if two-card systems cannot be checked (manual item 15).
6. REQ-214 hardware decode in the preview: stays opt-in with Faster; if a decoder cannot be made to fall back without a black frame, cut to roadmap.
7. REQ-279 Enhance speech: the bundled model is used if its licence is permissive, otherwise speechnorm named "Normalise speech" (Q-6 closed); cut only if neither passes.
8. REQ-239 workspaces and REQ-242 rebinding: cut to roadmap if the interface stage runs out of time (both are program settings only).
9. REQ-187 script coverage: no CJK font is bundled (Q-3 closed), so the cut-back is the Arabic and Devanagari fonts only if their licence check fails; the CJK note of REQ-187/188 is not a cut-back.
Not cut: REQ-293 (LUT copy, data safety), REQ-297 (cache limit), REQ-230 (one window), format 11 itself.

## 6. Questions — all closed by Andrea (D61, 2026-11-21 REFINE)
No open SEVERE question. The decisions below are binding for the stages; the defaults of the first plan are replaced.
- Q-1 (REQ-248, player association) CLOSED (a): the installers add the player under "Open with" for MP4, MOV, MKV, WebM, M4V and AVI; the player becomes the default only by the user's own choice in the OS. REQ-248 acceptance: "after the user's choice". Stage 9 and packaging follow this; no silent default.
- Q-2 (REQ-273, spatial sound) CLOSED (a): first-order ambisonic (ACN/SN3D) rotated in Rust, reduced to stereo by a fixed cardioid pair, no extra data. Cut back to the roadmap only if real-time operation fails (cut-back item 1). Stage 10.
- Q-3 (REQ-187/188, CJK fonts) CLOSED (c, not the architect's default): NO bundled CJK font. Chinese, Japanese and Korean use the system's installed fonts; when no installed font covers the script, the subtitle list and export show a note naming the missing script (REQ-187/188). Arabic and Devanagari fonts stay bundled (OFL). Consequence: no CJK budget pressure on the installers; CJK glyphs depend on the system (empty boxes possible on a system without CJK fonts). Stage 1.
- Q-4 (REQ-246, screen readers) CLOSED (a): AccessKit enabled in eframe for Windows, macOS and Linux (AT-SPI on Linux). Size and crate count measured in stage 8 and recorded; no cut-back on size alone. Stage 8.
- Q-5 (REQ-266, 360 export speed) CLOSED (b): the fastest encoder preset whose file stays within 1.5 times the Best-quality size of the same settings, measured in stage 10 (the preset chosen is recorded in ADR-186). Stage 10.
- Q-6 (REQ-279, Enhance speech) CLOSED (a): ffmpeg `arnndn` with a bundled RNN speech model, if the model's licence is permissive (verify in stage 11; if not permissive, fall back to `speechnorm` named "Normalise speech" in the interface and REQ-279 wording amended by the Product Manager). Stage 11.
- Q-7 (REQ-261, 360 marker in MKV and WebM) CLOSED (a): in-house Matroska Projection writer, conditional on the stage 10 spike (the spike result is recorded; if the spike fails, the writer still goes ahead, and the PE reports). Stage 10.

## 7. Risks (summary; full list in ai-docs/architecture-risks.md § 2.0.4 PLAN)
- Size of the whole development: 120 REQs in 13 stages; regression risk on render and export graphs (golden argv gates, stage 0 split first).
- Hardware AV1 and VP9 unverified on every system; VP9 hardware encoders exist mostly on VA-API and few QuickSync generations (expect software). Output check protects files, not speed.
- Busy-or-hanging classification under load (stage 4): false "hanging" would skip a working candidate; tested with fake activity only.
- Preview hardware decode failure mid-playback (stage 5): the restart must keep the last frame.
- Per-clip frames and speed in the export graph (stage 11): the stream-copy rule must stay off for changed clips.
- Project file size: LUT copies up to 20 MB each (stage 11); the read cap rises to 96 MiB (bounded read, memory use at most the cap).
- Autosave on a worker thread: a copy of the project is made on the UI thread (cost O(clips)); measured in stage 2.
- Spatial sound in real time (stage 10): cut-back item 1.
- Single window (stage 7): the inbox is a file in the private folder; the same validation as File > Open applies; a crashed first window leaves a stale lock that the OS releases.
- macOS menu bar (stage 7): the unsafe policy (ADR-034) decides between a crate and an allow-listed call (ADR-183).
- Fonts (stage 1): Arabic and Devanagari faces raise every installer by a few MB (estimate, measured); CJK is not bundled, so CJK coverage depends on the system's installed fonts (a Linux system without CJK fonts shows the note, not boxes silently); the UI font is parsed on first open of the subtitle list.
- Accessibility (stage 8): AccessKit on three systems adds the Linux AT-SPI bridge (crates, binary size, measured and recorded).

## 8. Future work (for the Documenter and Product Manager)
- Stage 6 measurement only: REQ-222 says "quarter of the frame size", frame.rs uses BLUR_DIVISOR 8. Stage 6 measures the blurred fill at both sizes, keeps the one that meets REQ-222 and REQ-125 (1 px) and records the value in ADR-180. No other stage depends on it.
- Documentation wording for 2.0.2/2.0.3 "(planned)" items in docs/architecture*.md: cleaned at the 2.0.4 release (architect).
- Human docs for the 2.0.4 development: docs/architecture-2.0.4-overview.md and docs/architecture-2.0.4-quality.md (written now; the per-area sub-documents get the 2.0.4 facts at release).
- Roadmap items not in 2.0.4 stay on the roadmap (business.md § Open): Priority 2 text items, ruler labels below 1 s, multi-selection report, batch 4 leftovers (gated alignment test, five-minute speech test, larger speech levels), 360 frame effects applied (REQ-265 note instead).
- Measurements that need Andrea's hardware: manual checklist items 14–17 (business-verification § Manual checks of 2.0.4).
