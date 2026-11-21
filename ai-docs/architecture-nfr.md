# ai-notes: NFR targets (owner: architect; index in ai-docs/architecture.md)

Gates are marked GATE; everything else is an estimate or target. Grouped by area; version of origin in brackets.

## Processes and scheduling
- GATE: ffmpeg/ffprobe processes never exceed the level's cap: 3 (Light, Balanced) or clamp(L/2,3,6) (Maximum; 3 if R<7 GiB); test 20 imports + export + preview at L=1,8,16, high-water <= cap, all complete. [0.2.0, 0.3.0 ADR-029]
- Scheduler cap floor 3: playback 2 + export 1. Track extraction and range audio extraction: 1 Background slot each, sequential (REQ-033). Whisper outside cap. [1.2.0]
- Hardware probe: 1 Probe slot; [0.4.2] 1 s first try + one 4 s retry per candidate, <= 10 s total, at most one automatic repeat when inconclusive (<= ~20 s bg per trigger). CI real probe bound = total + 1.5 s, two runs equal results, finds no hw.
- Hardware export: stall 30 s; post-kill wait <= 5 s then abandon; cancel < 1 s in every phase (poll <= 250 ms); one partial at a time; killed process keeps its Export slot until it truly exits (ADR-039 deviation). [0.4.0]
- Export progress >= 2/s; cancel < 1 s; no partial outputs; atomic project and settings saves. [0.2.0]
- The UI thread does no subprocess or blocking I/O except project save/load (<= 16 MiB), settings (<= 64 KiB), small Linux desktop writes at start-up. Rule: UI never waits on a lock held across worker I/O (try_lock / channels only). [0.2.0, 0.3.0]

## UI responsiveness
- GATE: every UI frame <= 200 ms during export + subtitle generation (harness ui::tests::stays_responsive_while_exporting_and_transcribing: FakeRunner export ~3 s + fake transcriber on available_parallelism threads (Maximum) + 30 headless frames with Space / timeline scroll / Settings click). [0.3.0; extended 1.1.0 (splitter drags, popups), 1.2.0 (effect drag, matching), 1.2.6 (camera, boxes, 360 playback on 4K x 2K and 8K x 4K), 1.3.0 (texts, 4K/8K look-around)]
- Importing 10 files: dispatch returns at once, each frame < 50 ms. [0.2.0]
- Interface 1.1.0: smallest window 800x500; every control visible or reachable by scrolling; section proportions within 1 px at 800x500, 1280x800, 2560x1440; all 12 toolbar commands usable at 800 px; target < 16 ms per frame at 1280x800 (ESTIMATE).
- Layout: whole-pixel borders; full coverage, no overlap; mins enforced; impossible sizes -> >= 0, no panic. Markers x = instance start ±1 px at min/mid/max zoom. Settings saved only on drag release / reset / recent change.
- Drag ghost < 16 ms; nothing saved during a drag. [1.2.0]
- Settings window and dialogs: Export dialog opens within 100 ms (ESTIMATE target). [1.4.0]
- Text: best fit of 200 items < 10 ms; event build <= 50 ms for 200 items (unit). [1.3.0, 1.4.0]

## Preview and playback
- Still frame: no render for edits that do not change the picture at the playhead; at most one render for a burst of 10 edits. Pause still = one render after 80 ms quiet (1.6.1: no extra render for an unchanged picture). [0.2.0, 1.6.1]
- Playback: video keeps real time while audio stalls; A/V drift after resume <= 40 ms (business requirement 100 ms). Playback start with a proxy < 300 ms (target, not gate). [0.2.0]
- Clean audio: noise-only parts >= 6 dB quieter, speech within 3 dB (ASSUMPTION, PE measures on fixture); preview with clean audio: no audio corrections over 60 s (ESTIMATE); frame <= 200 ms with clean audio + export + STT. [1.4.0]
- Camera (1.2.6): view at 3 sample times preview vs export within 1° (picture test with coloured bands), 45° (±1°) at mid 0->90; still 4K source < 400 ms, 8K proxy < 700 ms (ESTIMATES); sendcmd file for 10 min 30 fps < 100 ms; path lookup O(log n). Measured 0.1.x: latency ~125-155 ms on tiny clips. 360 export >= 0.5x realtime 4K->1080p was an ESTIMATE NOT met (~1/3 real time or worse at 1080p; accepted, roadmap). 360 video export has no speed target (user decision, 1.3.0).
- Frame effects (1.5.0): export 1080p -> 9:16 with Scale/Move <= 1.5x the same re-encode without effects (assumption); still frame with effects < 80 ms at preview size (estimate).
- LUT (1.6.0): preview playback within 20% of no-LUT at 1080p; export <= 1.5x re-encode (assumptions, to measure). LUT cube: committed normalised cube in Work folder.

## Export and quality
- QG-01 (0.3.0): at recommended settings export <= 0.2.2 +5% (Balanced ffmpeg args == 0.2.2); Fast subtitles <= 0.2.2 +5%. start-up <= 0.2.2 + 0.5 s; window opens < 1 s; detection < 50 ms typical, <= 3 s guaranteed, no network, deterministic.
- Best quality (default, 0.4.0): export argv byte-identical to 0.3.0 (golden test); no probe, no check, start-up unchanged. Faster without working hw: no extra cost, same args.
- QG-05 (manual, real hardware; matrix win NVIDIA/Intel/AMD, linux NVIDIA prop/Intel/AMD, macOS AS; H.264+HEVC x Faster/Best): a hw used; b format + frames = Best; c hvc1 + QuickTime/iPhone; d visual; e <= 3 VMAF below Best; f <= 1.5x size; g >= 40% less time at 1080p incl check (ASSUMPTION, adjustable); h forced failure -> correct sw file. Results per release in docs/code.md.
  MEASURED 0.4.0 (Intel Iris Plus G7, Ubuntu 24.04, free intel-media-driver 24.1, system ffmpeg 6.1.1, VA-API H.264 CQ): e pass (VMAF Faster-Best -1.4/+1.4/+6.7), f pass (0.96x/1.32x/1.38x), g FAIL (time vs Best -14%, +41%, +6%). HEVC VA-API fails on driver even at CQ -> sw. With bundled ffmpeg: VA-API/QSV skipped (libva 2.20) -> no hw at all. Full row in docs/code-hardware.md.
  RECOMMEND (later version): re-baseline g per hardware class, or cheaper check (render graph only for short segments around the 3 samples; keep strict decode).
- Stream copy: never touched by hardware choice (REQ-023), byte-identical under both choices.
- Hardware choice may change export bytes (x264 thread-dependent) but not look or length; Preview quality never changes export (REQ-051).
- Export of 200 segments from one source: one decoder, frame-accurate (frame count = sum of segment frames; framemd5 spot checks). [0.2.0]
- 1.2.0: 10k subtitle cues: matching < 50 ms (unit; cost = recognition of same audio); SRT write 10k lines < 100 ms (atomic, UI thread); ruler O(visible ticks).
- Subtitle position: exported text centre within 2% of frame height of preview position (libass integration test 320x180). Text 1.3.0: flat text centre within 2% H preview vs export; 360 text within 1° (preview vs export and yaw/pitch math vs v360). Boxes and sizes within 2% of picture height (1.2.6).
- Alignment quality gate (gated, Fast model, committed fixture tests/fixtures/align/speech.mkv): >= 90% words matched, matched starts within 0.3 s (ASSUMPTION borderline, risk). [1.2.0]
- Accuracy test (gated): WER(small) <= WER(tiny), WER(turbo) <= WER(tiny) on espeak-ng sentence (0.3.0).
- 1.2.6 compatibility: golden argv for v1, v2 (1.1), v3 (1.2.5) fixtures byte-identical; projects without 360 clips export with the 1.2.5 commands; 1.3.0: argv byte-identical when no text/anchor (golden); 1.4.0: project without new features = same commands; 1.5.0: no frame effect = identical argv; 1.2.6 golden v1..v4 fixtures.
- Subtitle speed levels (0.3.0 estimates, PE measures): tiny >= 10x real time @4 thr (estimate); vs tiny small 6–8x slower, turbo 20–30x; 8 AVX2 threads: tiny ~30x RT, small ~5x, turbo ~1–2x; peak mem small ~0.5 GB, turbo ~1.3 GB; MostAccurate +~1.3 GB (estimate), greyed < 7 GiB.

## Memory and resources
- GATE (REQ-039): memory does not grow with source size: <= 200 MB more peak RSS for a long (2–5 min CI lavfi) source than a short one of the same resolution; ASSUMPTION holds for 4 GB sources. Caps: preview frame channel 8 x 640x360 RGBA (~7 MB); audio buffer 1 s; ffmpeg stderr ring 64 KiB; run() stdout 16 MiB; STT chunk 120 s x 16 kHz f32 (~7.7 MB); keyframe output line by line; track text read <= 16 MiB, <= 100k cues, MAX_WORDS 200k project cap; 360 frames: no frame buffers in videolore; ffmpeg 8K frame ~100 MB x few; command file <= ~64 MB cap (step doubling). Hardware check (outcheck): no extra memory in videolore since the ADR-039 deviation (PSNR runs inside ffmpeg; the old Rust PSNR ~12 MB figure is obsolete).
- Font files: <= 64 MiB, worker thread, LRU 8. Font scan never delays start-up (QG-01 bound).
- Installers: 70–130 MB each (estimate, measured in CI, not gate). Binary growth from splash pictures <= 175 KB per program (estimate, not measured). Player binary includes its own ffmpeg copy (size).
- Test memory cap (1.6.2, ADR-135): build/test/coverage on the development machine (15 GB) under `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0`; CI not capped; 8 GB assumption to check.
- 1.6.2 splash: decode <= 100 ms per program (estimate); summary fit once per open and on scale change; each picture ~1 MB RGBA.

## Platforms and portability
- Platforms: Linux x86_64 (Ubuntu 24.04 LTS and 26.04 LTS, current Fedora; glibc 2.39 max, package.sh fails above), macOS Apple Silicon (aarch64), Windows x86_64.
- STT on x86_64 needs AVX2 + FMA + F16C + BMI2 (+SSE4.2, AVX): about 2013+ CPUs; without it a message, never a crash (ADR-031, ADR-027 baseline). aarch64: all features assumed.
- Hardware encoders: NVIDIA (NVENC; driver >= 610 for the pinned SDK 13.1), Intel QSV and AMD AMF on Windows; Linux NVIDIA, VA-API (Intel, AMD); macOS VideoToolbox. Multi-GPU prefers NVIDIA.
- MSRV 1.100 (= stable by policy, ADR-012); toolchain latest stable.
- Coverage >= 85% line (`scripts/coverage.sh --fail-under 85`, main.rs excluded), feature coverage 100% (every implemented REQ has `// covers:`). Coverage via cargo-llvm-cov single run in CI (ADR-035).
- Build: cargo test wall >= 30% below 0.2.x baseline (QG-03); CI check job <= 5 min typical (ESTIMATE, 1.6.1 ADR-126; measured 1.6.0 25.2 min, apt 20.0 min; estimate ~4.5 min, typical ~5.9 min).

## Page (1.4.1, ADR-092)
- Static HTML, inline CSS, no JS, no external scripts or fonts; GIF images kept (~341 KB total); images 317–512 px, 46–71 KB each, shown at <= 300 CSS px; deployed by pages.yml. Recommended items (not verified implemented): img width/height; lazy loading below the fold; keyboard-scrollable timeline region; reduced-motion stills; optional SEO tags and CSP.

## Verification (2.0.2 PLAN; QG-07..10, docs/business-performance.md)
- QG-07 bundled ffmpeg 9.x checks: suite green on the pinned build; CI never skips an ffmpeg test (VIDEOLORE_REQUIRE_FFMPEG=1); 360 camera re-measured on 9.0.2 (median of 3), no gate change; record in docs/architecture-360.md.
- QG-08 test ffmpeg version = packaging/ffmpeg.lock linux-x86_64 version (test asserts it).
- QG-09 no product limit changed (REQ-061 1 s/4 s/10 s, REQ-104 200 ms); wall-clock product gates pass on the best of 5 samples; suite passes under a load run (4 busy loops; PE records).
- QG-10 notices exit 0/1 recorded; cargo-about 0.9.3 asserted; package-config job on every push (Windows and macOS --config-only, Linux --stub AppImage).
- Cost (estimate; QG-06): pinned ffmpeg archive about 100 MB, cached; package-config 2 to 3 min. Measured in the first CI run.

## Quality targets 1.6.x
- 1.6.0: splash decode <= 100 ms (estimate); player has no network code (CI check: player graph has no ureq / whisper-rs).
- 1.6.2: summary text tested to fit (estimate ~340 of 442 px at 14 px); editor version rectangle 74..408 x 381..465; player 78..386 x 388..462 inside 487x505 (tested).

## 2.0.3 PLAN (REQ-155..179; plan ai-docs/architecture-plan-2.0.3.md)
- GATE (REQ-155) paused picture at the last frame of a clip: picture (not black) at 1.98 s and 1.995 s of a 2 s 25 fps clip; export argv unchanged (golden).
- GATE (REQ-156) fill 90–100% of box height for ten test texts; export size = preview size within 2% of picture height (ffmpeg test, same events).
- GATE (REQ-157) "Hi" in the default box drawn at 10% of picture height (within 2%).
- GATE (REQ-166) a 20 MB installed font adds at most 0.5 s to the window's appearance (manual, maintainer's computer, business-verification); automated part: no face read at start.
- GATE (REQ-177, reworded D56): strict alignment test: at the Fast level, at least 90% of the MATCHED words start within 0.3 s of reference.json after the onset snap; the matched share is printed, not gated; pinned ffmpeg 9.0.2 and tiny q5_1 (sha-checked); Balanced and Most accurate not worse than Fast (measured, scratch note).
- GATE (REQ-178) 5-minute reference: no transcript word missing, no duplicate at a boundary.
- ESTIMATE measure: best fit of a 200-character text under 10 ms (cached per key; 1.4.0 target kept), per edit of the text; shaping once per (text, face).
- ESTIMATE UI frame <= 200 ms with 200 texts during export and recognition (existing gate, REQ-104 test on the new handles and warnings).
- QG-01 (kept): Fast subtitles at most +5% time: piece margins add 3.3% of audio work (2 s on each side of 120 s; ADR-165), measured on the reference clip.
- Memory: no DTW (D56); the 10 ms energy envelope adds about 7 MB per hour of audio (estimate); the "Most accurate" memory note (about 1.3 GB, estimate) is unchanged; the 200 MB growth gate of the export stays.
- Export of a project without texts or new features: argv byte-identical (GATE), except the blur fill of pictures smaller than 64 px wide (ADR-152).
- Format 10: loading old projects is at most as slow as format 9 (no extra pass beyond the REQ-167 count, estimate).

## 2.0.4 (built at the checkpoint 2026-11-21; REQ-180..299 except the 12 not built; plan ai-docs/architecture-plan-2.0.4.md)
- GATE golden: projects without the new features export with the 2.0.3 argv (render/golden_tests); blurred projects may change pixels only by the ADR-180 blur (divisor 8, built). Export argv of a plain project unchanged.
- GATE UI frame ≤ 200 ms with the new popups, keyframe widget, theme and autosave running during export and subtitle work (harness, extended per stage).
- GATE REQ-206: normal-method reference renders at most three ranges of one second in the check.
- GATE REQ-224: first playback frame ≤ 200 ms with ten imports queued (harness).
- GATE REQ-223: ten Scale changes at one playhead draw the blurred background once (counted).
- GATE REQ-267: half-size view while dragging; full size within 200 ms after release (estimate on 4K).
- GATE REQ-266: at most 10 camera updates per second of video (a 10 s moving camera = at most 100).
- GATE REQ-289 / REQ-290 / REQ-291: LUT strength, .3dl/.csp and DOMAIN fixtures within 2 colour levels of their references.
- ESTIMATE REQ-297: cache enforcement < 100 ms for 1000 copies (measured once).
- ESTIMATE REQ-187: added fonts are Arabic and Devanagari only, 1–2 MB each (to measure; the files are in assets/fonts); no CJK font is bundled (D61; CJK from system fonts, note when none); installer size reported by the configuration check; the 25 MB MINOR budget is not under pressure from CJK.
- ESTIMATE project load ≤ 1 s for a 96 MiB file (format 11 with LUT copies); cap 96 MiB (ADR-191); LUT copies ≤ 20 MB each, ≤ 64 MiB per project.
- ESTIMATE autosave: copy on the UI thread is O(clips); serialise and write on the worker; a 1 h project ≤ 10 MB JSON (estimate).
- ESTIMATE hardware check: total time per candidate unchanged (REQ-061: 1 s and 4 s, 10 s total; repeat ≤ 20 s); AV1 and VP9 candidates add to the probe, bounded by the same limits.
- QG-05 g per kind (integrated and dedicated) measured manually (REQ-207; manual item 14); the target is set after the measurement.
- QG-11 (not a gate): export of 4K and HEVC at 1080p and 4K, large frames (4096 px cap), blurred fill, Clean audio cost on a weak computer; recorded in the developer documentation.
- QG-12 (not a gate): 360 export on 8K x 4K; moving-camera export on bundled 9.0.2 measured; 4K and 8K frame times of preview and look-around against REQ-104 (200 ms).
- QG-13 (not a gate): Apply LUT with 33 and 65-point LUTs: preview within 20 % of playback without a LUT; export ≤ 1.5 times the re-encode without one (assumptions).
- QG-14..18: BUILT (stage 14, ADR-201); QG-11..13 measured, not gated.
- GATE REQ-280 (built): volume keyframes 0 % at 0 s, 100 % at 4 s, 50 % at 2 s = gain 0.25; preview and export agree.
- GATE REQ-281 (built, tests in tests/ffmpeg_integration/v204b.rs and render/tests.rs): 4 s at 200 % lasts 2 s; pitch within 1 %; a subtitle within 0.1 s of its word; a sped-up clip is never stream copied.
- MEASURED REQ-273 (built): spatial pre-pass 120 s of four-channel sound in 1.7 s, debug profile (69x real time); a one-minute preview clip waits about 1 s; CPU on weak hardware unmeasured; preview clips over 10 min keep recorded sound.
- GATE REQ-253 (built): picture subtitle golden argv: projects without a chosen picture subtitle export with the same argv; the overlay test checks the pixels of the box before, inside and after its time.
- ESTIMATE REQ-287 (built): PNG header read only; no count, file size or pixel limit in the code (ADR-192 limits OPEN).
- Measured 2.0.4 (not gated): blurred fill divisor 8 at 4K 2.9 to 3.1 s vs 3.3 to 3.7 s; cycle check 1.7 s; pinned ffmpeg 173 MB unpacked each.
- Settings file cap 64 KiB unchanged (worst case ~35 KB, ADR-195); project read cap 16 MiB → 96 MiB in format 11 (ADR-191; `MAX_PROJECT_BYTES`).
- Platforms: Linux x86_64 (glibc 2.39 max, unchanged), macOS aarch64 and Windows x86_64 (unchanged; Intel Mac, ARM Windows and ARM Linux out of scope, REQ-040 targets unchanged); the one-window channel is file based on Windows and Linux (ADR-182).
- Accessibility: AccessKit adapters on all three OS (D61, Q-4 closed); the UI frame gate is measured with the tree on.

## 2.1.0 PLAN (REQ-300..311; plan ai-docs/architecture-plan-2.1.0.md § 2, § 9)
- LUT load (GATE QG-19 part, measured in S0a and S2): parse 144^3 <= 2 s and 256^3 <= 6 s wall on the development machine (ESTIMATES); best of 5 with base::testing at 144^3; the UI frame <= 200 ms while a LUT loads (GATE: worker job with progress and cancel, ADR-205).
- LUT memory: table u16 6 bytes per entry (256^3 = 100.7 MB, 144^3 = 17.9 MB, 160^3 = 30.7 MB); cache by bytes 192 MB (ADR-205, one 256 table plus a 144 table); load peak extra <= table + 2 MiB (no whole-file or whole-text buffer, ADR-203/206). Each ffmpeg process with a 256 LUT holds a float table of about 201 MB (ffmpeg, not measured yet: S0a).
- LUT Work copy (ADR-206): one file per distinct content, about 450 MB at 256^3 and about 80 MB at 144^3 (on disk, not in memory); written <= 3 s at 256^3 (ESTIMATE, S4); progress shown; removed with the Work folder.
- Preview with a 256^3 LUT: each preview start parses the text in ffmpeg; the start-up time is measured in S0a (risk, plan section 11). Playback rate within 20% of the same clip without a LUT (ESTIMATE from the 1.6.0 target, measured S4); export with a LUT <= 1.5x the re-encode without (existing assumption, measured S4).
- Overlay stage (ADR-208): only for clips with a changed order; graph size grows by one filter step per movable row; argv of other projects byte-identical (GATE QG-21).
- Event caps: ASS 2000 events per document and 20000 per file (ADR-067) kept; Typewriter and Enlarge use about two events per word; shadow copies only for boxed items.
- Limits (Q-1 = B): LUT edge 2..=256, file 512 MiB, line 256 bytes; shadow 0..=20 px at 1080; enlarge 1..=40 px at 1080; project format 12 read cap 96 MiB unchanged.
- Platforms: unchanged (Linux, macOS, Windows); no new platform code.
