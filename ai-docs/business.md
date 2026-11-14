# ai-notes: business

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

## State
- Target version 0.3.0. Next free ID: REQ-060. Never reuse IDs. Quality goals use QG-NN (next QG-05), no covers: tests.
- impl in 0.3.0: 001-054, 056-059. planned: 055 (hw export encoding).
- impl in 0.2.0: ALL 001-044 (014-020 moved planned -> implemented).
- 0.3.0 = compatibility (first-start auto setup + plain Settings), performance review, security, faster tests, CI warnings. Andrea away during PLAN/UPDATE -> SEVERE ambiguities DEFAULTED, must be asked at the end (A1-A4 + Architect's defaulted items, see 0.3.0 section).
- Sub-doc business-performance.md (0.3.0): 045-059 + QG-01..04.
- History: 0.1.3 impl 001-013, 021-025; 014-020 planned. 0.2.0 = "resolve all 0.1.3 known limitations" + projects save/open/dirty + bundled ffmpeg + perf/proxies + installers + music + subtitles.
- Human docs split (0.2.0): docs/business.md = index (purpose, users, constraints, global decisions, global out of scope, limitations summary, links). Sub-docs:
  - business-editing.md: 001-013, 024-029, 032-035, 038, 039 (media, projects, timeline, preview, layout, responsiveness, proxies, memory)
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
- 024 fixed 5-panel layout: top 60% (media 30% | preview 70%); middle 30% (timeline 70% | effects 30%); bottom 10% command bar incl. zoom. 0.2.0: music lane + subtitle list fit inside existing panels, proportions unchanged.
- 025 all ops reachable from command bar/panels; shortcuts split, cut, copy, paste, undo, redo, play/pause. 0.2.0 command bar adds Save, Save As, Clear proxy cache, Generate subtitles, About/notices (+ Quit Ctrl/Cmd+Q). 0.3.0 adds Settings (Clear proxy cache kept in bar; also "Clear preview copies" in Settings).
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
- 054 (REWORDED per Architect verdict) older CPUs never crash: !speech_supported (x86-64 lacking AVX2/FMA/F16C/BMI2/SSE4.2/AVX) -> Generate shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand"; no download offered; 3 levels greyed w/ same reason; ggml never reached. NO fallback engine in 0.3.0 (single static ggml; no GGML_BACKEND_DL in whisper-rs-sys). Fallback engine (SSE-only helper exe) = planned later, no REQ id yet (candidate for 0.4). AC simulated CPU -> no crash, message, rec never unrunnable.
- 055 (planned) hw video encoding for export w/ silent sw fallback, keep 022/023/036, "Export: Faster / Best quality" only when built. NOT BUILT 0.3.0 (Architect): no GPU in CI, per-vendor tuning, driver faults exit 0 -> fallback can't catch. Planned design in architecture notes.
- 056 responsive while export + STT run together at any level: play/pause, timeline scroll, open Settings react; harness: each frame <= 200 ms + effect visible next frame. 032 unchanged.
- 057 hostile/malformed project/settings/media: no crash, no program run, no writes outside user-chosen/own folders, sources untouched; bad project refused clearly; bad settings -> recommended + notice (bad file copied to settings.json.bad). Corpus: bad/truncated JSON, deep nesting, >16 MiB, huge numbers, ../ & absolute paths, control/bidi chars, FIFO project.
- 058 models only from fixed built-in https URLs, SHA-256 checked before use; partial/wrong deleted; no other network/telemetry/update check. AC wrong checksum, interrupted, http:// refused.
- 059 temp files user-only (0600/0700 Unix), removed on finish/cancel, crash leftovers cleaned next start. Moved from system temp to <cache>/videolore/work/. Finished export 0644.
- QG-01 perf review (each decision w/ reason, by Architect); no regression: export & Fast STT <= 0.2.2 +5%, start <= +0.5 s. PE reports.
- QG-02 vuln check in CI (cargo-deny advisories + yanked deny; documented ignores only); bundled ffmpeg refreshed (9.0.2 latest; BtbN re-pin).
- QG-03 cargo test wall time >= 30% lower than 0.2.2, nothing removed/weakened, cov >= 85, feature 100%.
- QG-04 no Node 20 / ubuntu-latest warnings; runners pinned (ubuntu-24.04, macos-15, windows-2025); Linux packages run on Ubuntu LTS + Fedora.

## Constraints / decisions
- C1 REVERSED in 0.2.0: ffmpeg bundled in every installer (was: user-installed >= 4.4). Reason: user-friendliness (Andrea). GPL builds (x264/x265) needed for H.264/HEVC; source offered per release (third-party source archive + written offer). Licence compliance not lawyer-reviewed (assumption).
- C2 offline: only network = consented one-time download of the speech model(s) the user chooses (each w/ own consent; 32/190/574 MB, MIT; fixed https URLs, SHA-256 checked). No telemetry, no update checks.
- C3 (0.3.0) settings are program-level, never in project; no project schema change in 0.3.0.
- D1 fade fixed length.
- D2 output format automatic (no export settings dialog).
- D3 layout fixed.
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
- D14 macOS: default app menu off so Cmd+Q passes through prompt; Hide/Minimize menu items lost (accepted).

- D15 (0.3.0) detection in app at first start, all OSes; installers dumb (A1 defaulted).
- D16 (0.3.0) single "How hard" level instead of thread/process numbers (A5). Light = per-job half cores (2 jobs together may use all cores); preview playback never limited; cap >= 3. Balanced == 0.2.2 (no regression). Maximum ffmpeg auto threading may exceed L internally (x264 ~1.5xL), documented. Exports at different levels may differ by bytes, same look/length.
- D17 (0.3.0) Settings = in-app floating window (A7 + Architect); 5-panel layout unchanged.
- D18 (0.3.0) custom settings win on machine change (A9); notice self-closing (8 s), no wizard (A8).
- D19 (0.3.0) speech levels tiny/small/turbo; greedy decoding; recommend bigger only on strong machines (A2 defaulted).
- D20 (0.3.0) older CPUs: message only, fallback engine planned (A4 defaulted -> Architect infeasible).
- D21 (0.3.0) hw export deferred, 055 planned (A3 defaulted -> Architect not built).
- D22 (0.3.0) temp files in <cache>/videolore/work/ (private), not system temp.

## 0.3.0 defaulted items to ask Andrea at the end
- A1 detection location (chosen app first start; alt Windows installer too / installer flag only).
- A2 model levels (chosen 3 levels tiny/small/turbo, consent on first use; alt single model / base for Balanced / medium for MA / always recommend Fast).
- A3 hw export (Architect: not built, planned; alt all vendors w/ fallback / VideoToolbox only).
- A4 older CPUs (Architect: message only; alt for 0.4 SSE-only helper exe, ~7x slower).
- Light meaning per-job vs strict total budget (Architect defaulted per-job).
- Settings in-app window vs OS window.
- QG numbers (A10) adjustable after baseline.
- Architect non-blocking: allow single cargo llvm-cov run locally (rule 7 change via /change-workflow).

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
- 0.3.0 additions: detection/benchmark in installers; long benchmarks; online machine DBs; telemetry; arbitrary/third-party speech models; translation; GPU for preview/effects (export encoding only under 055, planned); per-project performance settings; raw technical values in main Settings section. Still OOS: code signing, Intel Mac, ARM Win/Linux, auto-updates.
