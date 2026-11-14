# ai-notes: business

<!-- AI-only notes. Owner agent edits only. Dense format OK. -->

## State
- Target version 0.2.0. Next free ID: REQ-045. Never reuse IDs.
- impl in 0.2.0: ALL 001-044 (014-020 moved planned -> implemented). No planned reqs left.
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
- 014 (reworded) offline subtitle generation from VIDEO audio only (not music); background, progress, cancel. Model downloaded once after consent dialog (size + licence) OR local file from user; then fully offline. ONLY network access of product.
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
- 025 all ops reachable from command bar/panels; shortcuts split, cut, copy, paste, undo, redo, play/pause. 0.2.0 command bar adds Save, Save As, Clear proxy cache, Generate subtitles, About/notices (+ Quit Ctrl/Cmd+Q).
- 026 Save writes current file; Save As asks path; Save on never-saved = Save As. AC: Save As a.vlproj, edit, Save -> updated, no dialog.
- 027 title = project name + unsaved marker; save clears; undo back to saved state clears.
- 028 New/Open/close window/quit when dirty -> Save/Discard/Cancel. Save then continue (failed save or cancelled Save As stops); Discard continues; Cancel unchanged. No prompt when clean.
- 029 0.1.x projects open identically; newer-version files refused w/ clear msg, file untouched.
- 030 import/preview/export work right after install without system ffmpeg; "install ffmpeg" msg only if bundled copy missing/damaged; user override (setting/env) kept; H.264+HEVC export kept, same output formats as 0.1.3.
- 031 About/third-party notices (app + installer): every bundled component w/ version, licence, where to get exact source (link or written offer).
- 032 import never freezes window: item "analysing" -> name+duration or rejection; multiple imports while editing. AC 10 files no UI block.
- 033 <= 3 ffmpeg/ffprobe processes at once; extra queued; obsolete dropped. AC 20 imports+export+preview -> always <= 3, all complete.
- 034 still preview re-rendered only when picture at playhead changes; bursts coalesced. AC 0 renders if unchanged; 10 rapid edits -> <= 1.
- 035 video continues real time if audio stalls; audio catches up/skips. AC drift <= 100 ms after resume.
- 036 HEVC in MP4/MOV tagged hvc1 (copy + re-encode).
- 037 200 segments from one source export OK, frame-accurate.
- 038 proxies: source big for this machine -> lower-res preview copy, background, progress on media item; preview/timeline use it when ready; export always originals; cache outside project, reused, recreated if missing; size shown + clear cmd; on by default, settings switch. Rule: > 1920x1080 px, or > 1280x720 when < 8 logical cores or < 8 GB RAM. Deterministic; export identical on/off.
- 039 memory does not grow w/ source file size. AC peak(4 GB src) <= peak(100 MB src same res) + 200 MB. (CI uses 2-5 min clip; assumption that it extrapolates.)
- 040 (reworded after Architect Q) one double-click installer per OS, no terminal, ffmpeg+runtimes included. Win x86_64 wizard, per-user, no admin, Start menu, uninstall from Apps. macOS Apple Silicon disk image drag to Applications. Linux x86_64: AppImage (after one-time documented "allow executing" step) OR extracted .tar.gz, run by double-click on current Ubuntu LTS + Fedora; both self-add app-menu entry + .vlproj association. Uninstall never deletes projects.
- 041 (reworded after Architect Q) app icon; .vlproj associated; double-click opens it. If app already running: Win/Linux open a second window with that project (first window untouched, no prompt); macOS delivers to running app -> REQ-028 prompt if dirty.
- 042 installers for 3 OSes built automatically per release, published on release page; unsigned in 0.2.0; README per-OS download/install incl first-launch steps (Gatekeeper, SmartScreen, AppImage permission).
- 043 video edits do not move music; music past video end cut there w/ 0.5 s fade-out; silence where no music clip.
- 044 regenerating subtitles asks confirmation before replacing manual edits.

## Constraints / decisions
- C1 REVERSED in 0.2.0: ffmpeg bundled in every installer (was: user-installed >= 4.4). Reason: user-friendliness (Andrea). GPL builds (x264/x265) needed for H.264/HEVC; source offered per release (third-party source archive + written offer). Licence compliance not lawyer-reviewed (assumption).
- C2 offline: only network = consented one-time speech model download (~32 MB, MIT). No telemetry, no update checks.
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

## MINOR defaults (0.2.0 PLAN)
- 3-process cap = ffmpeg/ffprobe only; speech recognition limited separately (its audio extraction counts).
- Queue priority: preview > export > import probe > proxy/subtitle background. Proxy generation preemptible (restarts).
- Proxy rule tunable assumption.
- Music accepts audio files only.
- Windows per-user install.
- Undo to saved state clears marker.
- User ffmpeg override kept.
- About/notices from command bar.
- Older: undo depth 50 min; shortcuts conventional (exact bindings by engineering, assumption); letterbox black.

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
