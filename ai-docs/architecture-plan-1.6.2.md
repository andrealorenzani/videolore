# Architecture plan 1.6.2 (splash for the editor and the player) — PLAN 2026-11-20

Owner: architect. Request: business.md § 1.6.2 PLAN (REQ-133, REQ-134 reworded, REQ-150 new; drafts of both summaries there). Target 1.6.2. Questions: none (all decisions [TECHNICAL], ADR-128..135 in ai-docs/architecture.md).

## Code facts (read 2026-11-20)
- `src/splash.rs` (pub mod of the lib, package `videolore`): `IMAGE` = include_bytes of `assets/splash/<CARGO_PKG_VERSION>.gif`, `SUMMARY` = include_str of `<version>.txt`; `TEXT_RECT` (74,381)-(408,465); caps `MAX_FRAMES` 64, `MAX_SIDE` 4096; tests assert 486x506 and 2..=6 summary lines.
- `src/ui/splash_ui.rs`: `load`, `show`, `fit_version`, `summary_panel`. Summary: no scroll area, labels wrap, text beyond the panel is clipped silently; no fit rule.
- `src/lib.rs` `real_app`: `show_splash()` only when no path is given (ADR-113 / Q-C 1.6.0).
- `Cargo.toml` already says 1.6.2, so the tree does not build: `assets/splash/1.6.2.gif` and `1.6.2.txt` do not exist. Fixed by PE step 1.
- Pictures: `1.6.0.gif` and `1.6.1.gif` are byte-identical (cmp, 87875 bytes), both tracked; `videolore-player.gif` 487x505, untracked (must be committed with the code). Player rectangle 78..386 x 388..462 lies inside 487x505 (386<=487, 462<=505). Editor rectangle 74..408 x 381..465 lies inside 486x506.
- `player/src/lib.rs` `real_app(file)`: opens `file` at once; no splash. The player has no popup enum: its only popup is the full-screen hint, with its own state in `player/src/fullscreen.rs`, drawn by `popups()` in `player/src/ui.rs`; Esc becomes `Command::Escape` and closes dialog, then the hint, then leaves full screen.
- `player/Cargo.toml`: depends on `videolore` (path, no speech, audio) and `eframe`, `rfd`. The lib already has `ui::icons` (player reuses the editor's icon artwork, 1.6.0 Q-A).
- `packaging/notices/third-party.txt` L71-73: splash entry names `<version>.gif`; must name the fixed pictures.
- Release sources: `scripts/collect-release.sh` takes `git ls-files --cached --others --exclude-standard`, so committed or untracked non-ignored files are included.
- CI (`.github/workflows/build.yml`): fmt, clippy (three variants), test, build of the whole workspace compile the lib, so a missing asset fails every job; no CI step names the splash.
- Machine: 15 GB RAM, cgroup v2 with the memory controller, `systemd-run` present (for the test cap, ADR-135).

## Components (changes)
- `videolore::splash` (lib, pure): `Program { image, summary, text_rect }`; constants `EDITOR` and `PLAYER` hold the four `include_*!` (ADR-129). Functions: `version_text`, `layout`, `fit_text` (version), `fit_summary` (new, ADR-130), `mean_colour`, `text_colour`, `decode` (plus total-bytes cap, ADR-133), `summary_lines(&Program)`.
- `videolore::ui::splash_window` (lib, new, egui): `show(ctx, slot: &mut Option<Loaded>, program: &Program) -> SplashOutcome { Open, Close }` and `SplashRects` (the caller turns `Close` into its own command). Draws image (animated if the GIF has frames), version text, summary, Close. Moved out of `splash_ui.rs`.
- `src/ui/splash_ui.rs` (editor): thin adapter: `Popup::Splash` glue, passes `EDITOR`, pushes `ClosePopup`. Behaviour unchanged (REQ-133).
- `player/`: a splash state of its own (the player has no popup enum yet), drawn in `popups()`; `real_app` opens it when `file` is None (ADR-132); passes `videolore::splash::PLAYER`; Esc order: splash, full-screen hint, full screen.
- `packaging/notices/third-party.txt`: splash entry rewritten (PE).

## Data flow
Start without a path -> `Popup::Splash` opened (non-blocking) -> per frame `splash_window::show` (decode once on first show, textures cached in the program's slot; summary size fitted once per open and on scale change, cached) -> Close / Esc (topmost popup first) / window close -> `ClosePopup`. A start with a path never opens it. No file is read at run time.

## Ordered steps for the Principal Engineer
1. Assets (tree builds again): `git mv assets/splash/1.6.1.gif assets/splash/videolore.gif`; `git rm assets/splash/1.6.0.gif` (identical to videolore.gif; git and releases/1.6.0 keep it); keep `1.6.0.txt`, `1.6.1.txt`; write `assets/splash/1.6.2.txt` and `assets/splash/1.6.2-player.txt` exactly from the drafts in business.md § 1.6.2 PLAN (UTF-8, LF, final newline, heading then lines); `git add` `videolore-player.gif` and the new files.
2. Pure lib `src/splash.rs`: `Program`, `EDITOR`, `PLAYER` (include macros of ADR-129; delete `IMAGE`/`SUMMARY`); `summary_lines(&Program)`; `fit_summary`; total-bytes cap. Tests (pure first): see the list below.
3. Window: `src/ui/splash_window.rs` (move from `splash_ui.rs`, generic over `Program`); summary fit (no clip, no scroll); `splash_ui.rs` becomes the adapter. Existing `splash_ux_tests` must pass unchanged in meaning (open/close/notice/path rules).
4. Player: splash state drawn in `player/src/ui.rs` `popups()` (the player has no popup enum; add a small one only if it is simpler); `real_app` opens it when `file` is None; Esc order: splash, other popup, full screen; F11 and Space keep working while it is open (non-blocking). Tests REQ-150.
5. Notices: `packaging/notices/third-party.txt` splash entry: the two pictures (`assets/splash/videolore.gif` editor, `assets/splash/videolore-player.gif` player), fixed names, origin and licence still to be confirmed. No Cargo change, so `scripts/notices.sh --check` must still pass.
6. Code docs (PE owns `docs/code*.md`): `docs/code-splash-icons.md`, `docs/code-player.md`, `docs/code-modules.md`, `docs/code-testing.md` (the test memory cap, ADR-135). Then the architect's human docs are already updated (this plan).
7. Quality suite, every command under the memory cap (ADR-135): `cargo fmt --all --check`; `cargo clippy --locked --workspace --all-targets -- -D warnings` (CI also runs `--no-default-features` and `--all-features`); `cargo test --locked --workspace`; `scripts/coverage.sh --fail-under 85`; `scripts/feature-coverage.sh`; `scripts/version.sh check`; `scripts/notices.sh --check`. Report the line coverage (one decimal) and feature coverage.
8. Measure (report, do not block): release build size of the editor before and after (both pictures in the lib; if the linker keeps the player picture in the editor, ADR-128 fallback: move `PLAYER` into the player crate).

## Tests per REQ
- REQ-133 (editor, unchanged): existing `splash_ux_tests` (open at start, three closing methods, Esc order, banner waits, path shows none, second start shows again). `// covers: REQ-133` kept.
- REQ-134 (reworded): `EDITOR` decodes 486x506, one frame allowed to be animated by the GIF; `TEXT_RECT` inside the picture; `version_text("1.6.2") == "Version 1.6"` and the egui-measured text fits the rectangle minus margin; popup 400 wide (+-1 px), as high as picture; summary: heading + 1..=5 lines, fits at the chosen size (no clip, measured with egui wrap); `fit_summary` unit tests (largest size, floor, never enlarged). `// covers: REQ-134`.
- REQ-150 (new, player): `PLAYER` decodes 487x505; rectangle 78..386 x 388..462 inside it; "Version 1.6" fits 308x74 minus margin; popup 400x505; summary `1.6.2-player` heading + 1..=5 lines fits; opens with no file, not with a file; each of the three closing methods removes it; the next start shows it again; Esc closes it before the other popup and before full screen; Space and F11 still work while it is open. `// covers: REQ-150` in `player/src/ui_tests.rs` or `player/src/tests.rs`.
- Shared: the version text and layout tests in the lib (`// covers: REQ-134`). Coverage: new code in the lib and player counted in `scripts/coverage.sh`.

## NFR (1.6.2)
- Start: decode of one picture <= 100 ms (estimate, unchanged), once per program per run; the summary fit is computed once per open and on scale change, not every frame.
- Memory: each picture is about 1 MB as RGBA (487x505x4 = 0.98 MB per frame); only the picture of the running program is decoded.
- Binary size: at most the two pictures (about 175 KB) more in each program; measured (PE step 8).
- Portability: no OS-specific code, same behaviour on Linux, macOS and Windows; MSRV 1.100 unchanged.
- Test time: the summary fit test measures text once per program, no extra full render loop.

## Security (1.6.2)
- Pictures and summaries are compiled in: no file is read at run time, no network (player graph check unchanged).
- The summary is plain text drawn as labels: no markup, no links, no user input.
- Decoder caps (ADR-133): 64 frames, side 4096, and a total of 64 MiB of RGBA per picture (new). The files are trusted build inputs; the caps guard against a bad replacement at build time.

## Crates (1.6.2)
No new crate. `image` 0.25.10 (gif feature, already in the lock) and `eframe` 0.37.0 (egui) are used by the lib; the player reuses them through `videolore`. `cargo-deny` and `notices.sh` unchanged.

## Risks (1.6.2)
- Build breaks after a version bump until both summaries exist (intended; the message is the compiler's "couldn't read" error). Two summaries per release are more work.
- Summary text too long for the popup: shrinks to the floor; a longer text fails the test instead of being clipped.
- Stale summary per program (a release without an updated text shows the old one).
- Both pictures are in the editor and the player libraries: binary size (measured, fallback in step 8).
- The pictures' origin and licence are still unconfirmed (business limitation, before release).
- The player has no file association, so a double-clicked video gets no splash (not affected: a path is given).
- The editor summary of 1.6.2 describes the splash itself (PM draft; PM may prefer user-facing items).

## Future work (architect)
- Origin and licence of `videolore.gif` and `videolore-player.gif` (before release).
- Remove `1.6.0.txt` and `1.6.1.txt` only if they stop being useful (kept as history; not read by the build).
- A clearer build message for a missing summary (build.rs) only if the compiler message proves unclear.
- If the player ever gets a crates.io or other package, `include_*!` paths outside the crate need packaging care (`include` list).
- The test memory cap belongs in AGENTS.md rule 7 and docs/workflow.md: via `/change-workflow` (architect cannot edit it).
- ai-docs/architecture.md is far above the short-note rule (compaction owed, owner work).
