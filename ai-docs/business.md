# ai-notes: business

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

## State
- Target version 1.2.0. Next free ID: REQ-096. Never reuse IDs. Quality goals use QG-NN (next QG-06), no covers: tests.
- impl in 1.2.0: ALL 001-095. Nothing planned. 082-095 new; reworded 011/020/023/052/054/069/075/077/078/079 (+ impact sentences on 008/056/057). All marked implemented (PE implements in parallel w/ `// covers:`).
- 1.2.0 doc placement: 082, 093 + reworded 069/075/077/078/079 -> business-ui.md; 083, 094 (+011, 008 note) -> business-editing.md; 084 (+020) -> business-music.md (TITLE renamed "Music and volume", filename same); 085-092, 095 -> business-subtitles.md (subsections: Creating and editing / Subtitles already in the video / Saving as file / Position and selection); 023 -> business-export.md; 052/054/056/057 notes -> business-performance.md. Index: doc descriptions, 3 key decisions (1.2.0), limitations header -> 1.2.0 + 3 items.
- impl in 1.1.0: ALL 001-081. Nothing planned. 024+025 reworded, 066-081 new (UI/UX restructuring), all marked implemented (PE implements in parallel, must add `// covers:` per REQ).
- 1.1.0 docs: new sub-doc docs/business-ui.md holds 024, 025, 066-081 (+ features, key decisions, impact list, known limitations, OOS). 024/025 MOVED out of business-editing.md (left a pointer paragraph, no REQ pattern). Index lists business-ui.md.
- impl in 0.4.0: ALL 001-065. Nothing planned.
- 0.4.0 = hardware export (055 reworded planned->impl, 060-065 new, QG-05) + Andrea's confirmation of 0.3.0 defaults (2026-11-14). Docs: 055, 060-065 in business-export.md "### Faster export (0.4.0)" + "## Known limitations (0.4.0)"; QG-05 in business-performance.md Quality goals; perf doc REQ-055 line replaced by cross-ref (no "(planned)" pattern left).
- impl in 0.3.0: 001-054, 056-059. planned: 055 (hw export encoding).
- impl in 0.2.0: ALL 001-044 (014-020 moved planned -> implemented).
- 0.3.0 = compatibility (first-start auto setup + plain Settings), performance review, security, faster tests, CI warnings. Andrea away during PLAN/UPDATE -> SEVERE ambiguities DEFAULTED, must be asked at the end (A1-A4 + Architect's defaulted items, see 0.3.0 section).
- Sub-doc business-performance.md (0.3.0): 045-059 + QG-01..04.
- History: 0.1.3 impl 001-013, 021-025; 014-020 planned. 0.2.0 = "resolve all 0.1.3 known limitations" + projects save/open/dirty + bundled ffmpeg + perf/proxies + installers + music + subtitles.
- Human docs split (0.2.0): docs/business.md = index (purpose, users, constraints, global decisions, global out of scope, limitations summary, links). Sub-docs:
  - business-editing.md: 001-013, 026-029, 032-035, 038, 039 (media, projects, timeline, preview, responsiveness, proxies, memory). 024/025 moved to business-ui.md in 1.1.0.
  - business-ui.md (1.1.0): 024, 025, 066-081.
  - business-export.md: 021-023, 036, 037
  - business-music.md: 019, 020, 043
  - business-subtitles.md: 014-018, 044
  - business-install.md: 030, 031, 040-042
- feature-coverage.sh greps `REQ-NNN (implemented)` anywhere in those files -> never write that pattern in prose for a non-implemented req.

## Product
- videolore: desktop, offline, non-destructive video editor for creators of short social clips. Simple fixed UI, fast export, auto subtitles + music bed. Installs with one double-click per OS.
- Users: U1 social content creator (non-expert, wants speed, captions); U2 hobbyist editing personal clips; not pro editors (no multitrack video, no color grading).

## Requirements (short)
- 001 non-destructive: sources never modified; edits = project data.
- 002 (reworded 0.2.0) import media list (video + audio): name+duration; unsupported -> clear rejection. Audio-only accepted, placeable on music track (refused on video track).
- 003 save/reopen project; missing sources reported by file name (project still opens).
- 004 undo/redo >= 50 steps. (0.2.0: covers music, mix, subtitle edits too.)
- 005 append media to timeline end.
- 006 split at playhead.
- 007 cut/copy/paste segments, ripple (no gaps).
- 008 merge contiguous segments same source & contiguous source ranges; disabled otherwise.
- 009 trim segment edges within source bounds.
- 010 reorder segments, ripple.
- 011 fade-out + fade-in through black, video+audio, removable. Per side = min(0.5 s, clip_len/2). Never overlaps.
- 012 zoom slider: fit-all .. frame level.
- 013 preview: play/pause/seek, shows effects (0.2.0: + music, subtitles).
- 014 (reworded 0.2.0, again 0.3.0) offline subtitle generation from VIDEO audio only (not music); background, progress, cancel. Model OF CHOSEN LEVEL (052) downloaded once after consent dialog (size + licence) OR local file from user; each model own consent; then fully offline. Those downloads = ONLY network access of product.
- 015 (reworded) language auto-detected, overridable; one language per project.
- 016 (reworded) lines added, deleted, re-texted, re-timed; all undoable.
- 017 (reworded) >= 3 presets incl. word-by-word highlight; one preset per project; visible in preview + burned in export; text size scales with video height (readable 9:16 and landscape).
- 018 (reworded) subtitles tied to source speech: cut removes, move/copy moves/duplicates, trim hides those outside kept range, split keeps.
- 019 (reworded) one music track below video track; clips never overlap; audio files placed, moved, trimmed, split, deleted; undoable.
- 020 (reworded) music mixed with video audio in preview+export; separate volume 0-100% + mute for video audio and music; saved in project.
- 021 export w/ progress; cancel -> no partial file.
- 022 auto output format: all identical -> same; else lowest by pixels, then fps, then bitrate; letterbox/pillarbox, never stretch.
- 023 (reworded 0.2.0) stream copy iff all clips identical format AND no effects (fades, subtitles, music) AND every cut on keyframe; else single re-encode pass, frame-accurate. Architect: video volume != 100% or muted also forces re-encode.
- 024 (REWORDED 1.1.0) menu bar / toolbar / content / status bar; content default top row 2/3 h (Media 30 | Preview 70), bottom 1/3 (Timeline 70 | Effects 30); title headers; Timeline header = zoom slider + Fit; defaults at first start + Reset layout. AC no saved layout, 800x500/1280x800/2560x1440 ±1 px, no overlap, full coverage. (Was until 1.0: fixed 5-panel 60/30/10 with command bar.)
- 025 (REWORDED 1.1.0) every op reachable from menus; common ones also toolbar/sections/context menus; shortcuts per 071. (Was: command bar/panels + 7 shortcuts; test HELP.len()==10 changes.)
- 026 Save writes current file; Save As asks path; Save on never-saved = Save As. AC: Save As a.vlproj, edit, Save -> updated, no dialog.
- 027 title = project name + unsaved marker; save clears; undo back to saved state clears.
- 028 New/Open/close window/quit when dirty -> Save/Discard/Cancel. Save then continue (failed save or cancelled Save As stops); Discard continues; Cancel unchanged. No prompt when clean.
- 029 0.1.x projects open identically; newer-version files refused w/ clear msg, file untouched.
- 030 import/preview/export work right after install without system ffmpeg; "install ffmpeg" msg only if bundled copy missing/damaged; user override (setting/env) kept, 0.3.0 override UI moved to Settings -> Advanced (was effects panel); H.264+HEVC export kept, same output formats as 0.1.3.
- 031 About/third-party notices (app + installer): every bundled component w/ version, licence, where to get exact source (link or written offer). 0.3.0: + all 3 offered speech models (MIT) + any new bundled component.
- 032 import never freezes window: item "analysing" -> name+duration or rejection; multiple imports while editing. AC 10 files no UI block.
- 033 (reworded 0.3.0) ffmpeg/ffprobe processes at once <= cap of current effort level (050): Light/Balanced 3, Maximum clamp(L/2,3,6) (3 if RAM<7GiB); extra queued; obsolete dropped. AC 20 imports+export+preview at every level never exceed level cap, all complete.
- 034 still preview re-rendered only when picture at playhead changes; bursts coalesced. AC 0 renders if unchanged; 10 rapid edits -> <= 1.
- 035 video continues real time if audio stalls; audio catches up/skips. AC drift <= 100 ms after resume.
- 036 HEVC in MP4/MOV tagged hvc1 (copy + re-encode).
- 037 200 segments from one source export OK, frame-accurate.
- 038 proxies: source big for this machine -> lower-res preview copy, background, progress on media item; preview/timeline use it when ready; export always originals; cache outside project, reused, recreated if missing; size shown + clear cmd. 0.3.0: on/off switch REPLACED by Preview quality (051); Automatic = this rule. Rule: > 1920x1080 px, or > 1280x720 when < 8 logical cores or < 8 GB RAM. Deterministic; export identical whatever choice.
- 039 memory does not grow w/ source file size. AC peak(4 GB src) <= peak(100 MB src same res) + 200 MB. (CI uses 2-5 min clip; assumption that it extrapolates.)
- 040 (reworded after Architect Q) one double-click installer per OS, no terminal, ffmpeg+runtimes included. Win x86_64 wizard, per-user, no admin, Start menu, uninstall from Apps. macOS Apple Silicon disk image drag to Applications. Linux x86_64: AppImage (after one-time documented "allow executing" step) OR extracted .tar.gz, run by double-click on current Ubuntu LTS + Fedora; both self-add app-menu entry + .vlproj association. Uninstall never deletes projects. 0.3.0: Linux built on pinned runner (ubuntu-24.04, glibc 2.39), packages still run on current Ubuntu LTS + Fedora; installers do NO tuning (detection in app, 045).
- 042 note 0.3.0: build machines pinned explicitly (QG-04).
- 041 (reworded after Architect Q) app icon; .vlproj associated; double-click opens it. If app already running: Win/Linux open a second window with that project (first window untouched, no prompt); macOS delivers to running app -> REQ-028 prompt if dirty.
- 042 installers for 3 OSes built automatically per release, published on release page; unsigned in 0.2.0; README per-OS download/install incl first-launch steps (Gatekeeper, SmartScreen, AppImage permission).
- 043 video edits do not move music; music past video end cut there w/ 0.5 s fade-out; silence where no music clip.
- 044 regenerating subtitles asks confirmation before replacing manual edits (0.3.0: also when regenerating at another level).

### 0.3.0 (doc: business-performance.md)
- 045 first-start auto setup in APP (all OSes incl AppImage; not installer). No settings file -> detect, save recommended values, self-closing notice once "videolore has been set up for this computer. You can change this in Settings." AC: created+notice once; 2nd start no notice/no re-detect (unless 048); detect <=3 s, no benchmark, no network; deterministic.
- 046 Settings = floating window INSIDE app (egui window, not OS window), from command bar. Preview quality Automatic/Smoother/Original; Subtitle quality Fast/Balanced/Most accurate; How hard videolore works Light/Balanced/Maximum speed; 1-line explanations; status "Using recommended settings for this computer"/"Custom settings"; storage lines "Preview copies" + "Clear preview copies", "Speech recognition files" + "Remove downloaded speech files"; collapsed Advanced: ffmpeg override, bundled component paths/versions/origin, speech engine, processor supported. AC (a) main section none of thread, core, proxy, codec, encoder, ffmpeg, AVX, CPU, GPU, model, quantised (case-insens whole words); (b) no restart; (c) program settings never in project; (d) unsupported greyed + plain reason.
- 047 "Use recommended settings": re-detect, reset 3 choices, plain summary or "Already using the recommended settings". AC = exactly 045 rec; untouched: ffmpeg override, projects, preview-copy cache, models; never downloads.
- 048 machine fingerprint changed: recommended in use (choices == recommend(saved fp)) -> recompute silently + 045 notice; custom -> keep + non-blocking notice w/ "Use recommended settings". No stored "recommended" flag.
- 049 upgrade from 0.2.x = first start except proxies_enabled=false -> Original; ffmpeg override kept; model kept. AC 0.2.2 file w/ proxies off + override.
- 050 effort level controls ffmpeg process cap, export/proxy ffmpeg threads, STT threads. Light <= max(1,L/2) per job; Balanced = 0.2.2 behaviour; Maximum may use all; configured never > L. Mapping table in architecture (Architect). Next job only, running not restarted. AC mapping at L 1,2,4,8,16,32 x RAM 4/8/16 GiB.
- 051 Preview quality: Automatic = 038 rule; Smoother = proxy if > 1280x720; Original = never. Export identical whatever choice (D10).
- 052 subtitle levels: Fast = tiny-q5_1 32 MB (unchanged); Balanced = small-q5_1 ~190 MB; Most accurate = large-v3-turbo-q5_0 ~574 MB; all MIT. Rec: Fast if !speech_supported; MA if L>=12 && RAM>=15 GiB; Balanced if L>=4 && RAM>=7 GiB; else Fast. Greyed: MA RAM<7 GiB, Balanced RAM<3.5 GiB. Download only when generating w/ missing level, consent dialog (size, licence) options: Download / "Use Fast instead (no download)" if tiny present (THIS RUN ONLY, setting unchanged) / "Download the Fast one instead (32 MB)" if nothing installed / local file (must match level size+SHA) / Cancel. AC WER(higher) <= WER(Fast) on reference clip (gated integration test); decline changes nothing; level change never auto-regenerates.
- 053 (wording aligned w/ Architect: "Speech recognition files" / "Remove downloaded speech files") disk used by downloaded models + remove w/ confirmation; projects/subtitles untouched; re-download w/ consent. AC size == disk; freed; subtitles still show/export.
- 054 (REWORDED per Architect verdict) older CPUs never crash: !speech_supported (x86-64 lacking AVX2/FMA/F16C/BMI2/SSE4.2/AVX) -> Generate shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand"; no download offered; 3 levels greyed w/ same reason; ggml never reached. NO fallback engine in 0.3.0 (single static ggml; no GGML_BACKEND_DL in whisper-rs-sys). Fallback engine (SSE-only helper exe) DROPPED 0.4.0 (Andrea), out of scope. AC simulated CPU -> no crash, message, rec never unrunnable.
- 055 (0.3.0 planned, NOT BUILT: no GPU in CI, per-vendor tuning, exit-0 driver faults) -> 0.4.0 reworded + implemented, see 0.4.0 section.
- 056 responsive while export + STT run together at any level: play/pause, timeline scroll, open Settings react; harness: each frame <= 200 ms + effect visible next frame. 032 unchanged.
- 057 hostile/malformed project/settings/media: no crash, no program run, no writes outside user-chosen/own folders, sources untouched; bad project refused clearly; bad settings -> recommended + notice (bad file copied to settings.json.bad). Corpus: bad/truncated JSON, deep nesting, >16 MiB, huge numbers, ../ & absolute paths, control/bidi chars, FIFO project.
- 058 models only from fixed built-in https URLs, SHA-256 checked before use; partial/wrong deleted; no other network/telemetry/update check. AC wrong checksum, interrupted, http:// refused.
- 059 temp files user-only (0600/0700 Unix), removed on finish/cancel, crash leftovers cleaned next start. Moved from system temp to <cache>/videolore/work/. Finished export 0644.
- QG-01 perf review (each decision w/ reason, by Architect); no regression: export & Fast STT <= 0.2.2 +5%, start <= +0.5 s. PE reports.
- QG-02 vuln check in CI (cargo-deny advisories + yanked deny; documented ignores only); bundled ffmpeg refreshed (9.0.2 latest; BtbN re-pin).
- QG-03 cargo test wall time >= 30% lower than 0.2.2, nothing removed/weakened, cov >= 85, feature 100%. RESULT 0.3.0: 25% (target missed; recorded in perf doc, per caller 0.4.0 UPDATE).

### 0.4.0 (doc: business-export.md "Faster export (0.4.0)"; QG-05 in business-performance.md)
Andrea answers 2026-11-14: S1 Best quality default everywhere, Faster opt-in, upgrades load Best quality. S2 all vendors on, safety net, unverified combos = known limitations. S3 check = full decode + metadata/frame count + 3 sample frames vs normal render. S4 Faster <= 3 VMAF below BQ, size <= 1.5x BQ.
- 055 (reworded) Export=Faster + re-encode to H.264/HEVC -> working hw from 061: NVENC (Win/Linux), QSV (Win/Linux), AMF (Win only), VA-API (Linux; AMD+Intel), VideoToolbox (macOS). Missing/error exit/crash/30 s no-progress stall/063 fail -> silent sw redo, no error. AC simulated per vendor: each failure -> file same format as BQ, correct frame count, no error, no temp left (059); cancel in hw/check/redo -> no partial (021), no redo; BQ never uses hw. Redo fails too -> Failed as 0.3.0, nothing remembered.
- 060 Settings main section "Export: Faster / Best quality", texts: Faster "Uses your computer's video hardware when it can. Often quicker, but not on every computer; files may be a little larger." (changed post-QG-05 2026-11-14 from "Much quicker; ..." because iGPU measured slower; code must match) BQ "Uses the normal method. Slower, best picture for the file size." Rec = BQ always. Faster never greyed; no-hw note "This computer has no video hardware videolore can use, so both choices give the same result." (only when check done + neither format has hw). Next export only. AC 046(a) banned words; Faster -> Custom; 047 -> BQ; program settings not project (C3); 045 saves BQ.
- 061 (reworded 0.4.2) background check per codec: 1 s/candidate first try (from spawn); on time-out same candidate 1 retry <=4 s (order unchanged); hard bound 10 s whole check (slot waiting excluded); inconclusive (no hw for a format AND one of its candidates timed out) -> whole check repeated once automatically, first answer usable meanwhile; probe time-outs never recorded in 064 memory; cancellable. (Was <=1 s/candidate, <=3 s total in 0.4.0-0.4.1.) No window/startup/preview delay, no network, counts in 033 cap (1 Probe-priority slot). Export before check done -> sw. Order Win NVENC>QSV>AMF; Linux NVENC>QSV>VA-API (render nodes by number, max 4); macOS VT. Stops at first working per codec. Runs ONLY when needed (Architect, allowed by plan): startup if saved=Faster, switch to Faster, Use recommended, Try again, ffmpeg override change while Faster. BQ -> nothing runs; Advanced "not checked (only checked when Export is Faster)". Session-only results, no cache. Advanced: hw per format + last export method (session only). AC QG-01 startup, limits, deterministic, user ffmpeg (030) probed same.
- 062 hw output == BQ: container, codec, res, fps, 8-bit 4:2:0, audio, frames+timing, letterbox, fades, music, burned subs (022, 037). HEVC mp4/mov hvc1 (036). Choice never changes stream copy (023): byte-identical. Non-H.264/HEVC always sw. Hw targets same avg bitrate as sw (VBR, per-vendor factor 100-120%); unknown source bitrate -> sw.
- 063 output check before publish: full decode no errors; frame count/duration/res/fps/pix_fmt/profile/hvc1/audio match; 3 samples (10/50/90%, best of +-1 neighbours) PSNR >= 25 dB on Y,U,V vs unencoded sw render (Architect threshold, ASSUMPTION, may move after QG-05). Colour tags not compared. Progress includes check (hw 0-0.85, check 0.85-1). AC simulated empty/truncated/missing frames/wrong size/green/garbage detected -> correct sw file; good file passes.
- 064 failure memory per format+method on this computer; resets: fingerprint change, new version, Use recommended (+recheck), "Try video hardware again" (+recheck); also a different method winning the check = hw change -> retry (Architect). Recorded only if sw redo succeeded; cancel never. Advanced "Video hardware turned off after a problem". Settings file stays v2 with optional fields export + hardware_off (max 8, validated per entry).
- 065 0.3.x file w/o export -> BQ, no notice, rec/custom status unchanged, file not rewritten; 0.2.x (049) + damaged (057) -> BQ.
- QG-05 manual real-hw verification (no covers): Win NVIDIA/Intel/AMD; Linux NVIDIA prop/Intel/AMD; macOS AS. Ref project (mixed formats letterbox, 50+ cuts, fades, music, subs), H.264+HEVC, Faster vs BQ: a Advanced shows hw; b format+frames == BQ; c hvc1 plays QuickTime+iPhone; d no blocking/banding/colour shift/green; e <= 3 VMAF below; f size <= 1.5x; g time incl check >= 40% lower at 1080p (ASSUMPTION, measured target not gate; Architect estimate 20-45% -> AT RISK); h forced failure -> correct sw file. Results per release in developer docs (docs/code.md); unverified combos = known limitations.
- Known limitations 0.4.0 (+ post-impl: libva>=2.21 for bundled ffmpeg, Intel HEVC hw broken on measured driver, Faster can be slower on iGPU) (from Architect): NVIDIA driver >= 610 (SDK 13.1 headers; older -> check fails, sw, Advanced "graphics driver too old"); Linux VA-API/QSV need system libva + vendor driver; Fedora default Mesa no H.264/HEVC encode -> RPM Fusion freeworld VA drivers (mesa-va-drivers-freeworld) for AMD/Intel; QSV on Linux = default render node only (other Intel node via VA-API); multi-GPU always prefers NVIDIA; unverified vendor combos per QG-05; 3-sample check misses damage between samples (QG-05 d guard); whole-system driver hang not preventable.
- POST-IMPL real-hw findings (Andrea laptop Ubuntu 24.04, Intel Iris Plus G7; Andrea chose fix before release, PE fixed):
  - Ubuntu free Intel VA driver refuses bitrate control -> VA-API = constant-quality + size guard: hw file > 1.5x expected BQ size -> sw redo, NOT recorded as hw failure (064). Exception to D25.
  - Bundled ffmpeg needs system libva >= 2.21; older (24.04 = 2.20) -> VA-API + QSV on Linux skipped, no crash, Advanced "needs a newer system video library (libva 2.21 or newer)". System/user ffmpeg (030) can still use hw.
  - QG-05 Linux+Intel (system ffmpeg, VA-API H.264): e,f PASS on 3 projects; g FAIL: Faster 14% quicker .. 41% slower than BQ (hw encode + check not faster than sw on iGPU). HEVC hw broken in that driver -> sw (check catches).
  - Recommendation recorded: re-baseline g after dGPU measurement (maybe per-GPU-kind target), consider cheaper 063 check later (must still catch 063 fault list). OPEN.
  - Docs: business-export.md key decisions (VA-API CQ exception, Faster wording reason) + known limitations (libva 2.21, Intel HEVC, Faster not always faster; replaced old "speed still to be measured"); business-performance.md QG-05 results table; business.md limitations summary.
- MINOR defaults 0.4.0: H.264+HEVC only; fixed order; Faster not greyed; export before check -> sw; stall 30 s; progress may go back once on redo, no msg; "hardware"/"video hardware" allowed in main section, GPU/encoder still banned; Light still applies to sw filter work during hw export (050 unchanged).
- Impact applied in docs: 046 text note (4th choice, Advanced hw status), 047 text note (resets Export, clears memory, rechecks). 045/048/049/057 covered by 060/065 wording, texts not changed.
- QG-04 no Node 20 / ubuntu-latest warnings; runners pinned (ubuntu-24.04, macos-15, windows-2025); Linux packages run on Ubuntu LTS + Fedora.

### 1.1.0 UI/UX restructuring (doc: business-ui.md)
Request (Andrea): resizable sections by dragging edges; Effects section = vertical list, click -> config popup; each effect instance marked on timeline, click marker -> instance settings; menus for settings + common actions; everything visible, scrollbars where needed; PM free to add common UI elements.
User answer: drag&drop on Linux Wayland (winit 0.30 never emits DroppedFile) -> ship where supported (Win, macOS, Linux X11); Wayland = known limitation (README + roadmap), Import/Open remain. All PM ambiguities MINOR, defaults accepted.
- 066 (M) 3 splitters Media|Preview, Timeline|Effects, rows; resize cursor; live; only 2 neighbours change; mins Media/Effects w 160, Preview/Timeline w 240, rows h 100; window resize keeps proportions; bars fixed content height. AC ±100 px, clamp, ±1 px after window resize, ≤200 ms/frame dragging under export+STT (056).
- 067 (M) layout = program setting (C3), restored at start; View > Reset layout -> 024; saved layout not fitting -> mins win (applied at draw only, stored fractions unchanged); missing/damaged -> default, other settings kept, no notice; opening project never changes layout.
- 068 (M) 800x500 any section at min: every control/text visible or scroll-reachable (sections, effect popups, clip popup, Settings, dialogs, shortcuts window); scrollbars only on overflow; Preview keeps controls+seek, picture shrinks; Timeline vertical scroll; long names "…" + tooltip. AC UI audit 50 media, 200 lines, long name.
- 069 (M) menus File/Edit/Playback/Effects/View/Help, item list in human doc; shortcut text; greyed == is_enabled; long menus scroll.
- 070 (M) toolbar replaces command bar: 12 cmds (Import, Add to timeline, Add music, Split, Cut, Copy, Paste, Delete, Undo, Redo, Play/Pause, Export|Cancel); overflow "»" menu. AC 800 px all 12 triggerable.
- 071 (M) shortcuts: existing kept + New N, Open O, Save as Shift+S, Import I, Export E, Delete/Backspace, Left/Right frame, Home/End, Ctrl+=/-/0 zoom, F1 shortcuts window, Esc closes topmost popup. Never while typing or modal. Single table drives menus/tooltips/shortcuts window. AC per binding; window lists exactly bound set.
- 072 (S) tooltips on toolbar, markers, add-fade handle, borders, icon controls (what + shortcut); disabled reasons at least Split, Merge, Paste, Undo, Redo, Export, Generate subtitles, Add music.
- 073 (M) status bar: left status msg; middle jobs w/ progress (imports count, proxies, model download, STT + Cancel, export + Cancel); right "Output: …" (moved from effects panel). AC export progress + Cancel == 021.
- 074 (S) "Undo <action>"/"Redo <action>" in Edit menu + tooltips. AC after Split -> "Undo Split"; after undo -> "Redo Split".
- 075 (M) Effects section list: Fade through black, Subtitles, Volume; name, 1-line desc, count ("3 fades", "42 lines", Volume levels); click -> popup; scrolls. Clip details, music clip, export, shortcut list removed from section.
- 076 (M) effect popups: in-app floating, non-modal, one per type, movable, close + Esc; changes immediate, in preview, 1 undo step each. Fades popup (explanation, add nearest playhead, list -> instance, Remove all = 1 undo step); Subtitles popup (style, lang, Generate/Cancel + progress + model dl progress, add line at playhead, lines list -> instance; 014-018, 044, 052-054 unchanged); Volume popup (020 unchanged, 1 undo per slider release).
- 077 (M) markers per fade + subtitle line, differ by shape/colour; tooltip type/time/text start; click: select, seek start, highlight, open instance popup (fade: two clips, time, Remove; line: text, start/end ±0.1 s, Delete, Play this line). Boundary w/o fade -> "Add fade" handle. Fade marker click OPENS popup (no direct removal any more). Overlap -> chooser. Follow ripple + zoom. AC 5 items (±1 px at 3 zooms, remove+undo, line edit undo, cut removes markers, chooser).
- 078 (M) Clip details popup: double-click clip (video/music), Edit menu, context menu. Video: source, in/out/len, trim ±1 frame / at playhead, fade in/out, Merge. Music: start/end trim, Move to playhead, Split, Delete. Non-modal, Esc, follows selection. AC == 009/019 results.
- 079 (S) context menus: video clip (Split here, Cut, Copy, Paste after, Delete, Merge with next, Add/remove fade before/after, Clip details); music clip (Split, Delete, Move to playhead, Clip details); marker (Open settings, Remove/Delete); empty timeline (Paste, Add subtitle line here); media items unchanged; greyed per is_enabled.
- 080 (M) Open recent ≤10, most recent first, file name + path tooltip, Clear list; through 028 prompt; missing -> message + removed; program setting only. Architect: absolute UTF-8 paths ≤1024 bytes, no control chars, dedupe; non-UTF-8 never added. AC 12 opened -> 10 in order; damaged list -> reset, no crash (057).
- 081 (S) drag&drop Win/macOS/Linux X11 ONLY (Wayland not: winit 0.30). Media -> import (002, 032 rules); .vlproj -> open after 028 prompt; any project in drop -> first project wins, rest ignored + status msg; ignored while modal open (Architect). Empty media list "Import media… or drop files here". AC 3 videos + 1 unsupported -> 3 imported + 1 rejection; project w/ unsaved -> prompt.
- MINOR defaults accepted: 1 only existing effects; 2 Volume listed w/o marker (whole project); 3 clip settings in Clip details popup; 4 popups non-modal; 5 command bar -> toolbar + status bar, zoom in Timeline header; 6 Settings… in Edit, Ctrl/Cmd+, kept; 7 layout persisted + Reset; 8 mins per Architect (160/240/100); 9 fade marker click opens popup, separate Add-fade handle; 10 overlap chooser; 11 recent = 10; 12 recent projects brought INTO scope (was OOS since 0.2.0); 13 mixed drop first project wins; 14 new settings fields optional, 1.0 settings open without notice (settings VERSION stays 2, additive lenient); 15 version 1.1.0 (named by Andrea).
- Impact applied in docs: 011 (fade click opens popup; removal via popup/context menu) + 012 (zoom slider in Timeline header + View menu) notes in business-editing.md; 046 "opened from Edit menu", 056 (+ border drags, open popups), 057 (+ layout/recent corpus) notes in business-performance.md; perf key decision "five-panel layout unchanged" -> "does not change the window layout". 009/019/016/044/020/021/028/031 impacts listed only in business-ui.md "Impact on existing features" (their REQ texts are UI-location-free, unchanged). Project format unchanged (029).
- Known limitations 1.1.0: Wayland no drag&drop; popup positions session-only; window size/position not remembered; macOS menu inside window (D14). 1.0 builds drop layout/recent fields on save (accepted).
- Product risk: egui API churn; text fields vs global shortcuts (guarded by tests).

### 1.2.0 effects per clip, subtitle reuse/align, SRT, positions, ruler (docs: ui, editing, music, subtitles, export, performance)
Request (Andrea): reuse existing subtitle track vs STT; .srt export; effect list names only; per-line on-screen position, Ctrl+click multi-select, Shift+drag timeline range select, group move = same position; ruler s / mm:ss / hh:mm:ss (90 s / 90 min); every effect to clip (drag / right-click) or whole video (double-click).
Answers: S1=A (fade on clip = both edges; whole = start+end edge fades). S2=A (subtitles on clip = only clip range; whole = all). S3=B (reuse track text + STT align words; needs model/consent/CPU rules).
- 082 apply effect to video clip (drag from list / right-click Effects ▸) or whole video (double-click; = one clip start..end). Click still opens popup. One undo step per application + opens instance popup. VOLUME (Q2): undo recorded at value change (slider release/mute), not at apply; AC(a) for Volume = one undo restores changed level. Drop elsewhere (music track, empty) -> nothing; cursor shows drop possible/not.
- 083 fade on clip: both edges (boundary = 011; first clip -> start fade from black; last -> end fade to black). Whole: start + end fades. Q1: start/end fades ATTACHED TO VIDEO EDGES (not to clip): moving/cutting clip 1 leaves start fade at t=0. Length 011 rule. Remove from clip = both edges. Each start/end/boundary own marker, removable alone. Add-fade handle stays. AC a/b/c/d (1.1 project same fades).
- 084 clip volume 0-100% (D12 curve) + mute, x video level (020). Marker at clip start iff !=100 or muted. In Clip details. Split both halves, copy/paste, reorder keep; Merge needs equal (greyed "The clips have different volumes"). AC a gain equivalence, b muted silent, c split, d forces re-encode.
- 085 subtitles on clip = only source range of clip (recognise or reuse 087); elsewhere untouched; whole = all clips. Hand-edited in range -> 044 ask. One subtitle job at a time ("Subtitles are being created").
- 086 media analysis records TEXT subtitle tracks (SRT, ASS/SSA, mov_text, WebVTT) + lang + title; picture-based ignored; pre-1.2 media re-checked in bg on project open; tooltip "Contains subtitles (<language>)".
- 087 dialog "This video already contains subtitles. Use their text, or recognise the speech?" [Use the video's subtitles][Recognise speech][Cancel]; several tracks -> list. Use: text from track for range only, tags stripped, multi-line cue joined, cleaned (MAX_TEXT, no control), then 095 align; track text never replaced; language from track unless user chose (015); edited=false. Recognise = 1.1 path. No-track sources -> recognition. Cancel: nothing, no undo. Asked every time.
- 095 alignment: STT on range at Settings level (052), match ignoring case/punct, fuzzy, missing/extra words; line keeps cue start/end; matched words recognised timing clamped in cue; unmatched spread between matched neighbours. Model consent/local file (014/052/053/058); bg progress "Matching subtitles to the speech" popup + status bar; 050; 056. Fallbacks keep track text + cue timing, words even (D13): (i) cue <50% matched/no speech -> status "N of M lines could not be matched to the speech; their words are spread evenly over the line"; (ii) CPU unsupported (054) -> Use button wording "Use the video's subtitles (this computer cannot match words to the speech; they will be spread evenly)", no download (Q9: also builds without stt); (iii) download refused -> consent extra button "Use without matching words"; (iv) recognition fails -> whole range spread + msg. Cancel during matching/download -> all or nothing. AC a-f + gated real-speech test (Fast, >=90% matched, start within 0.3 s; Q10 committed fixture; ASSUMPTION 0.3 s may be borderline, PE reports measured).
- 088 File > Export subtitles (.srt)…: timeline times, numbered time order, HH:MM:SS,mmm, UTF-8 no BOM, one text line/cue; greyed "No subtitles on the timeline"; default name <project>.srt; no positions; atomic (no partial). Q7: empty-text lines skipped.
- 089 per-line position, default preset place; drag in preview, box clamped inside picture; stored as fraction (survives 022); export centre within 2% H of preview; 1 undo/drag; line popup "Reset position"; belongs to line (copies share, M6).
- 090 Ctrl/Cmd+click markers or popup list toggles; plain click = only that (077); highlight; empty timeline click clears; Esc clears when no popup (else closes topmost).
- 091 Shift+drag timeline range selects lines whose START inside; Shift+Ctrl/Cmd adds; playhead -> first selected start.
- 092 group drag: all selected get EXACT dragged line's final position (relative ignored); 1 undo; Reset on selection resets all. Q8: only dragged box clamped.
- 093 ruler by time at RIGHT edge of visible: <=90 s "45s"; <=90 min mm:ss (minutes >59 ok, "75:30"); beyond hh:mm:ss; round steps 1,2,5,10,15,30 s/min, 1 h+; no overlap. Q7 zero-padded.
- 094 compat: <=1.1 projects same result (boundary fades, no edge fades, vol 100, preset positions). 1.2 stores new fields; schema always bumped (Q5: v3 always written); older versions refuse ("newer version", untouched). Hostile new fields refused or clamped (Q6: vol>100, Pos>10000 refused; track metadata sanitised).
- Reworded: 011 (+start/end fades, per clip or whole, 083); 020 (+clip volume x video level); 023 (+edge fades, clip vol !=100/muted; doc also lists video level !=100/muted, was architect note); 052 (+level used for 095, same consent); 054 (+video's subtitles usable, words even); 069 (File + Export subtitles (.srt)…; Effects + Apply to selected clip ▸ (3), Apply to whole video ▸ (3), Reset subtitle position); 075 (rows name only + tooltip "Drag onto a clip to apply it to that clip; double-click to apply it to the whole video"; click popup, double-click whole, draggable; count test changes); 077 (+start/end fade markers, clip volume marker -> clip volume popup; Ctrl/Cmd+click line markers 090; line popup Reset position); 078 (+clip volume slider+mute); 079 (video clip + Effects ▸ 3; subtitle marker + Reset position).
- Impact notes written in docs: 008 (merge equal volume), 056 (effect drag + preview drag during export+matching), 057 (embedded subtitle tracks + new project fields), business-ui "Impact … Since 1.2.0" (071 gestures + Esc order, 072 new disabled reasons, 073 status jobs, 076 popup titles "— whole video"/"— <clip name>" + new popups, 068).
- MINOR defaults accepted M1-M19 (see plan): M1 ruler right edge; M2 drops video clips only; M3 click popup/double-click whole; M4 0-100 no boost; M5 vol follows split/copy/reorder; M6 pos belongs to line; M7 fraction clamped 2%; M8 multi-select markers+list, playhead to first, Esc order; M9 asked every time; M10 cue cleaning; M11 cue keeps times, >=50%; M12 Settings level; M13 fallback even spread + msg; M14 all or nothing; M15 SRT separate cmd; M16 schema bump; M17 bg re-check, run first if needed; M18 one job; M19 1.2.0.
- Known limitations 1.2.0: alignment quality depends on track text vs speech (translated/paraphrased fall back); positioned lines overlapping in time overlap on screen; text edit after drag can push box past edge in export (no re-clamp). Also in index: 1.2 projects not openable by <=1.1.
- Product risk: Ctrl/Cmd+click + Shift+drag discoverability (tooltips + shortcuts window); real-speech 0.3 s gate borderline (Architect risk 1).

## Constraints / decisions
- C1 REVERSED in 0.2.0: ffmpeg bundled in every installer (was: user-installed >= 4.4). Reason: user-friendliness (Andrea). GPL builds (x264/x265) needed for H.264/HEVC; source offered per release (third-party source archive + written offer). Licence compliance not lawyer-reviewed (assumption).
- C2 offline: only network = consented one-time download of the speech model(s) the user chooses (each w/ own consent; 32/190/574 MB, MIT; fixed https URLs, SHA-256 checked). No telemetry, no update checks.
- C3 (0.3.0) settings are program-level, never in project; no project schema change in 0.3.0.
- D1 fade fixed length.
- D2 output format automatic (no export settings dialog).
- D3 layout fixed. REVERSED 1.1.0 for RESIZING only (Andrea request): sections resizable by dragging, remembered, Reset layout; moving/docking/hiding/detaching still OOS.
- D29 (1.1.0) all commands in menus driven by one action table (menus, toolbar, tooltips, shortcuts window = actual bindings). Effects configured in non-modal popups from Effects list or timeline markers. Fade marker click opens popup (no accidental removal).
- D30 (1.1.0) layout + recent list = program settings (C3), never project; project format unchanged.
- D31 (1.1.0) drag&drop shipped where supported (Win, macOS, Linux X11); Wayland limitation documented (Andrea).
- D4 stream copy only when frame-accurate; correctness > speed.
- D5 (0.2.0) installers unsigned; signing deferred not rejected. README first-launch steps.
- D6 (0.2.0) Linux ships AppImage + tar.gz (Andrea).
- D7 (0.2.0) double-click .vlproj while running: second window on Win/Linux; macOS -> running app + prompt (Andrea).
- D8 (0.2.0) bundled subtitle font Noto Sans Bold only (Latin/Greek/Cyrillic) (Andrea). Other scripts -> system fonts in export, may render as boxes; not readable in the app's subtitle editor. Documented limitation.
- D9 music does not ripple with video edits (simple mental model: music bed independent).
- D10 proxies never used for export (output quality never depends on proxies).
- D11 project schema v2 saved by 0.2.0 not openable by 0.1.x (forward-incompatible; documented).
- D12 volume slider perceptual curve (Architect: gain=(p/100)^2, 50% ~ -12 dB).
- D13 text edit of a subtitle line drops word timings -> highlight spread evenly over words. Editing a line shown in two copies of the same source range changes both.
- D32 (1.2.0) effects applied per video clip (drag/right-click) or whole video (double-click); single click = popup only (no accidental change). Effects on music clips OOS.
- D33 (1.2.0) start/end fades attached to video edges (Q1), not clip; "video starts from black" invariant under edits.
- D34 (1.2.0) reuse track: track text wins, cue start/end kept, STT only times words; always degrades to even spread (never loses text), user told.
- D35 (1.2.0) one preset per project still; only position per line. Position = fraction of picture.
- D36 (1.2.0) project schema bumped (v3 always written); <=1.1 can't open 1.2 projects (like D11).
- D14 macOS: default app menu off so Cmd+Q passes through prompt; Hide/Minimize menu items lost (accepted).

- D15 (0.3.0) detection in app at first start, all OSes; installers dumb (A1 defaulted, STILL PENDING).
- D16 (0.3.0) single "How hard" level instead of thread/process numbers (A5). Light = per-job half cores (2 jobs together may use all cores); preview playback never limited; cap >= 3. Balanced == 0.2.2 (no regression). Maximum ffmpeg auto threading may exceed L internally (x264 ~1.5xL), documented. Exports at different levels may differ by bytes, same look/length. Light per-job CONFIRMED by Andrea 2026-11-14.
- D17 (0.3.0) Settings = in-app floating window (A7 + Architect); 5-panel layout unchanged. CONFIRMED 2026-11-14.
- D18 (0.3.0) custom settings win on machine change (A9); notice self-closing (8 s), no wizard (A8).
- D19 (0.3.0) speech levels tiny/small/turbo; greedy decoding; recommend bigger only on strong machines. CONFIRMED 2026-11-14 (levels + current thresholds).
- D20 (0.3.0) older CPUs: message only. CONFIRMED 2026-11-14; slow fallback engine (SSE-only helper) DROPPED, not postponed -> out of scope (reconsider only on request).
- D21 (0.3.0) hw export deferred -> SUPERSEDED 0.4.0 by D23.
- D22 (0.3.0) temp files in <cache>/videolore/work/ (private), not system temp.
- D23 (0.4.0) hw export built for all vendors w/ silent fallback + output check (S2); BQ default, Faster opt-in (S1); check = decode+metadata+3 samples (S3); quality bar 3 VMAF / 1.5x size (S4).
- D24 (0.4.0) probe only when needed (Faster selected or explicit action), not every start (Architect; saves work, doesn't wake dGPU).
- D25 (0.4.0) hw targets sw bitrate (VBR) not constant quality; unknown bitrate -> sw. EXCEPTION VA-API: constant quality + 1.5x size guard (driver refuses bitrate control), oversize -> sw redo, not remembered.
- D27 (0.4.0 post-QG-05) Faster wording honest: "often quicker, but not on every computer" (iGPU measured up to 41% slower).
- D26 (0.4.0) silent fallback; method visible only in Advanced (session only).
- D28 (0.4.2, pre-release "MVP") probe time limits relaxed (Andrea chose fix): release verification found VA-API test encode >1 s under CPU load -> "no hardware" -> Faster useless on busy computer. Fix by PE: 1 s + one 4 s retry per candidate, 10 s total cap, one automatic full recheck if inconclusive, time-outs never remembered (064). REQ-061 reworded, still implemented. AC added: slow-but-working hw found in order; inconclusive -> exactly 1 repeat; time-out never turns hw off. Needs PE tests `// covers: REQ-061` for the new ACs (assumed present in PE fix).

## 0.3.0 defaulted items — status after Andrea 2026-11-14
- RESOLVED: A2 levels (confirmed), A3 hw export (build now, all vendors -> 0.4.0), A4 older CPUs (message only, fallback dropped), Light per-job (confirmed), Settings in-app window (confirmed).
- STILL PENDING (not mentioned by Andrea): A1 detection location (app first start vs installer); A10 QG numbers adjustable after baseline; Architect note: allow single local cargo llvm-cov run (rule 7 change, needs /change-workflow).

## MINOR defaults (0.3.0 PLAN)
- A5 single effort level. A6 Automatic preview = 038 rule exactly. A7 Settings own window, layout unchanged. A8 notice self-closing. A9 custom wins. A10 QG numbers are assumptions. A11 QGs no REQ ids; human docs section "Quality goals".

## MINOR defaults (0.2.0 PLAN)
- 3-process cap = ffmpeg/ffprobe only; speech recognition limited separately (its audio extraction counts). (0.3.0: cap per effort level, 033.)
- Queue priority: preview > export > import probe > proxy/subtitle background. Proxy generation preemptible (restarts).
- Proxy rule tunable assumption.
- Music accepts audio files only.
- Windows per-user install.
- Undo to saved state clears marker.
- User ffmpeg override kept.
- About/notices from command bar.
- Older: undo depth 50 min; shortcuts conventional (exact bindings by engineering, assumption); letterbox black.

## Known limitations (0.3.0 additions/changes)
- REMOVED: "STT crashes on pre-AVX2 x86-64" (0.2.1, was only in README/arch/changelog, never in business docs) -> now friendly message (054). Still: pre-AVX2 cannot auto-generate subtitles.
- Model speed/memory + recommendation thresholds are estimates (ASSUMPTION) until PE measures.
- VMs/containers w/ variable cores may look like a new machine each start (custom -> notice only).
- Proxies cleared -> regenerated after reopening project or changing Preview quality (ASSUMPTION carried from 0.2.x toggle behaviour).

## Known limitations (0.2.0)
- Non-Latin/Greek/Cyrillic subtitle scripts (D8).
- Unsigned installers: first-launch warnings (Gatekeeper, SmartScreen); AppImage needs exec permission once; some Linux file managers do not run the tar.gz binary on double-click (fallback: right-click Run / run once from terminal).
- Second window on Win/Linux (no single-instance).
- 0.2.0 projects not openable by 0.1.x.
- macOS Hide/Minimize menu items missing (D14).
- Subtitle accuracy possibly lower at 2-min chunk boundaries (accepted).
- No automatic proxy cache eviction (manual clear).

## Product risks
- H.264/HEVC patent licensing varies by country -> note in third-party notices.
- GPL corresponding source adequacy unreviewed by lawyer.
- Installer/first-launch behaviour verifiable only by hand on each OS (manual checklist by PE).

## Out of scope
- Multiple video tracks/overlays, color grading, keyframe animation, transitions other than fade-through-black, cloud/account/sync, online services (except model download), mobile, collaboration, direct social upload, export settings UI.
- 0.2.0 additions: code signing/notarisation (deferred); recent-projects list, autosave, crash recovery, auto-updates; Intel Mac, ARM Windows/Linux, app stores, Linux packages beyond AppImage + tar.gz; music ducking, per-clip fades, volume curves/automation, > 1 music track; SRT/VTT export, soft subtitles, > 1 language per project, translation, custom style editor, free subtitle positioning, choosing the speech model.
- Removed from OOS in 0.2.0: bundled ffmpeg.
- Removed from OOS in 0.3.0: "choosing the speech model" (replaced by 3 quality levels; arbitrary/third-party models still OOS).
- 0.3.0 additions: detection/benchmark in installers; long benchmarks; online machine DBs; telemetry; arbitrary/third-party speech models; translation; GPU for preview/effects (export encoding in scope since 0.4.0); per-project performance settings; raw technical values in main Settings section. Still OOS: code signing, Intel Mac, ARM Win/Linux, auto-updates.
- 1.1.0: REMOVED from OOS: recent-projects list (now 080), resizable panels (066). Editing OOS now "Dockable, movable or hideable panels"; index OOS "Autosave, crash recovery, automatic updates". ADDED (business-ui.md): move/dock/hide/collapse/detach sections, saved layouts/workspaces, resizing bars; customisable toolbar, rebindable shortcuts; new effect types (titles/overlays, speed, colour, other transitions, fade from/to black at start/end), per-fade length (D1); dragging markers / clip edges; native macOS menu bar (D14); themes, text size, translation; autosave/crash recovery; project format change.
- 1.2.0: REMOVED from OOS: "fade from/to black at start/end" (now 083), "any change to project file format" (094) [business-ui.md]; "Separate subtitle files (SRT/VTT)" -> SRT export in scope (088), "free positioning" (089) [business-subtitles.md]. ADDED: effects on music clips (ui, music); one-sided boundary fades + per-fade length (ui, editing); sub-second ruler labels (ui); volume boost >100% (music); WebVTT export, switchable tracks, positions in SRT; importing external .srt/.vtt; picture-based tracks (OCR); re-timing reused cue start/end; style per line/clip; group actions other than move/reset; snap/safe-area guides (subtitles).
- 0.4.0 additions (in business-export.md): hw for proxies/preview/effects/subtitles/decoding; hw AV1/VP9; choosing card/vendor, per-vendor quality sliders, any technical export setting (D2); AMF on Linux; bundling/installing GPU drivers; visible fallback message; Faster as default; battery-aware behaviour. In business-performance.md: slow fallback speech engine for pre-AVX2 CPUs.
