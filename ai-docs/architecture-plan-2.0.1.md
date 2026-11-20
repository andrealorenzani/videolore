# ai-notes: plan 2.0.1 — break the module cycles (owner: architect; index in ai-docs/architecture.md)

Scope: REQ-151..153 only (docs/business-performance.md § Code structure (2.0.1)). No behaviour, file format, setting, file name or output change. No migration, no notice, no format bump. Human version: docs/architecture-structure.md. Decisions: ADR-136..141 (ai-docs/architecture-adr.md).

## 1. Measured cycle facts (2026-11-20, version 2.0.0 tree, production code)
Method: every `src/**/*.rs` file, `//` comment lines removed, text from the first `#[cfg(test)]` cut (test blocks are at file end in this tree), `crate::<module>` references, top-level module grain (lib root excluded because nothing in production refers to crate-root items). Two false positives of a naive scan were removed by hand: `outcheck -> export` and `ffmpeg -> jobs` are doc-comment links only.
- Strongly connected group (16 modules, equal to docs/code-architecture.md § Cycles): align, ass, camera, frame, lut, media, music, project, proxy, scratch, settings, srt, stt, subtitles, text, timeline.
- Direct pairs (9 + 1 missing from the PE table):
  - project <-> settings: settings.rs:673,844,857 (`project::read_limited`, `write_atomic`); project.rs:639 (`settings::read_capped`).
  - stt <-> media: stt.rs:775,814,878 (`media::display_name`, `valid_language`; stt.rs:19 `MediaId`); media.rs:141 (`stt::language_name`).
  - stt <-> scratch: stt.rs:457-518 (`scratch::Work`, `private_dir`, `create_private_file`); scratch.rs:146 (`stt::builtin_models`, through `is_partial_name`).
  - scratch <-> proxy: scratch.rs:145 (`proxy::is_cache_file`); proxy.rs:328-339 (`scratch::private_dir`, `Work`).
  - lut <-> project: lut.rs:228 (`project::read_limited`); project.rs:503,508 and project/v9.rs:167 (`lut::path_ok`, `display_name`).
  - subtitles <-> timeline: timeline.rs:10,905,906 (`subtitles::Pos`, `POS_MAX`); subtitles.rs:12 (`timeline::Timeline`, `ClipId`).
  - subtitles <-> text: subtitles.rs:130,157 (`text::Anchor360`); text.rs:11 (`MAX_TEXT`, `SIZE_NORMAL`, `POS_MAX`, `Pos`).
  - ass <-> text: text.rs:10 (`ass::outline`, `ass::AssEvent` use); ass.rs:203 (`text::font_px`, `SIZE_NORMAL`), ass.rs:219 (`text::AssEvent`). Also `text::BASE_HEIGHT` (text.rs:45) used by font_px.
  - app <-> ui (MISSING in PE table): app/sphere.rs:11 `use crate::ui::boxes` for `boxes::align_h`, `align_v` (lines 324-333); ui -> app is the normal direction (about 20 ui files).
- Other edges inside the group that must go for the break (found by the simulation): camera -> text (camera.rs:354-368, Anchor360), frame -> subtitles (frame.rs:15, Pos/POS_MAX), scratch -> project (scratch.rs:192, read_limited), stt -> project (stt.rs:787, read_limited).
- Not cycles: export <-> outcheck (doc link only; outcheck uses render, hwenc, ffmpeg, time), ffmpeg <-> jobs (doc link only; jobs uses ffmpeg). docs/code-architecture.md is right on these two.
- File grain (folder modules split): one SCC, the same 16 modules; app/ui has no file-level cycle (only the top-level one).

## 2. Target layout
New module `base` = `src/base/` (one module, five files, no logic in mod.rs beyond docs and `pub mod`):
- `src/base/mod.rs`: `pub mod io; pub mod private; pub mod pos; pub mod names; pub mod language;`
- `src/base/io.rs`: capped and atomic file I/O (std only).
- `src/base/private.rs`: private folders and files, `Work`, file-name constants of the work folder.
- `src/base/pos.rs`: subtitle/text positions and 360 anchors, pure geometry helpers.
- `src/base/names.rs`: file-name rules of cache, partial files and speech models.
- `src/base/language.rs`: language code table and names.
Rules for `base` (checked by the test in section 4): no `crate::` path at all (no import of any other crate module); std and already-locked external crates only (serde, serde_json not needed by io/private/names/language; serde derives only in pos.rs for Pos and Anchor360, as today); no `unsafe`.

### 2.1 Items that move (old path -> new path; body unchanged unless stated)
| Item | From | To | Callers (about) |
|---|---|---|---|
| `read_capped(r, limit)` | settings.rs:862 | base::io | settings, project |
| `read_limited(path, limit)` (body calls base::io::read_capped) | project.rs:616 | base::io | settings.rs:673, lut.rs:228, stt.rs:787, scratch (cleanup), project load, player/src/subs.rs:96 |
| `write_atomic(path, bytes) -> io::Result<()>` (errors as io::Error; same temp name `.<name>.tmp-<pid>`) | project.rs:253 | base::io | settings.rs:844,857; project save |
| `parent_dir(path)` (body of project_dir) | project.rs:218 | base::io (project_dir stays and calls it) | project |
| `private_dir`, `create_private_file`, `publish` | scratch.rs:39,51,~70 | base::private | export, proxy, stt, spherical, preview, app (scratch::publish 3x, create_private_file 6x, private_dir 5x) |
| `Work` (+ Drop, register, path, new) | scratch.rs:86-~130 | base::private | export, proxy, stt, preview |
| `LOCK_FILE`, `TARGETS_FILE` | scratch.rs:25,27 | base::private (pub) | scratch cleanup |
| `is_cache_file` | proxy.rs:139 | base::names | proxy, scratch |
| `is_partial_name` | scratch.rs ~140-152 | base::names (uses is_cache_file and SPEECH_MODEL_FILES) | scratch cleanup |
| `SPEECH_MODEL_FILES: [&str; 3]` = "ggml-tiny-q5_1.bin", "ggml-small-q5_1.bin", "ggml-large-v3-turbo-q5_0.bin" | stt.rs:61,67,73 (literals) | base::names; stt builtin_models uses them | stt, scratch |
| `LANGUAGE_TABLE`, `language_entry` | stt.rs:171-272, 274 | base::language | |
| `iso639_2_to_1`, `language_name` | stt.rs:288,293 | base::language | stt (track args), media.rs:141 |
| `POS_MAX`, `Pos` (+ from_fraction, fraction, is_valid) | subtitles.rs:54-88 | base::pos | ~35 files (use lines) |
| `Anchor360` (+ is_valid) | text.rs:110-131 | base::pos | text, subtitles, camera, project, app, tests |
| `align_h`, `align_v` | ui/boxes.rs:316-329 | base::pos | ui/boxes, app/sphere.rs |
Stays where it is: `scratch::default_root` (player and app call it), `scratch::cleanup_stale` (start-up), the registry size limit (`MAX_TARGETS_BYTES`) and `remove_targets` (now using base::io::read_limited); `project::write_atomic` keeps its signature as a wrapper (`io_err` mapping) because the project save path uses it; `subtitles::SIZE_*` and `MAX_TEXT` stay in subtitles (text keeps using them). `text::BASE_HEIGHT` moves to ass with font_px (section 2.2).

### 2.2 ASS items: stay out of base (decision ADR-138)
`text::AssEvent` (text.rs:422-431), `text::font_px` (text.rs:432-435) and `text::BASE_HEIGHT` (text.rs:45) move to `src/ass.rs` (pub). `text` keeps producing events (`ass::AssEvent`), calls `ass::font_px`. `ass::text_style` calls the local `font_px`. Rejected: putting them in base (base would hold rendering rules); keeping them in text and passing closures to ass (more code, same result).

### 2.3 Re-exports: none (ADR-141)
No `pub use` shims in the old modules: every caller gets the new path. Reason: a shim would hide the real edge from the check and keep two paths for one item. Exception to check: none. The player (same workspace, PE-owned `player/src/*.rs`) gets the same path changes (`videolore::base::pos::Pos`, `videolore::base::io::read_limited`); `videolore::scratch::default_root` unchanged.

### 2.4 How each pair is broken (after the moves)
| Pair | Edge removed | Mechanism |
|---|---|---|
| project <-> settings | both directions | base::io |
| stt <-> media | media -> stt | base::language |
| stt <-> scratch | stt -> scratch (and scratch -> stt) | base::private; base::names |
| scratch <-> proxy | scratch -> proxy and proxy -> scratch | base::names; base::private |
| lut <-> project | lut -> project | base::io |
| subtitles <-> timeline | timeline -> subtitles | base::pos |
| subtitles <-> text | subtitles -> text | base::pos (Anchor360) |
| ass <-> text | ass -> text | ass.rs owns AssEvent, font_px, BASE_HEIGHT |
| app <-> ui | app -> ui | base::pos (align_h, align_v) |
| (inside the group) | camera -> text, frame -> subtitles, scratch -> project, stt -> project | base::pos, base::io |
Simulation on the same scan with these edges removed: no strongly connected component left (top level and file grain). `base` has no outgoing edge, so it can be in no cycle.

## 3. Behaviour invariants (must not change; checked by existing tests unchanged in assertions)
- Errors: `read_limited` kinds InvalidData ("too large") and InvalidInput ("not a regular file") stay; project load maps them as today. `write_atomic` failures: `ProjectError::Io(e.to_string())` through the project wrapper; settings uses `e.to_string()` as today; the message "invalid file name" stays (io::Error with InvalidInput and that text).
- Names: temp file `.<name>.tmp-<pid>`; work folders `job-` prefix; lock file `lock`; targets file `targets`; `.videolore-partial` suffix; `<32 hex>.mkv(.partial)`; `ggml-*.bin` and `.part`; language table content identical.
- Serde: Pos, Anchor360 fields, derives and aliases (`size_cd`, `wrap_cd`) unchanged; no format bump.
- Private folder: 0700 folders, 0600 files, lock closed before folder removal (Work field order).
- Startup: `scratch::cleanup_stale` called at the same place; partial-name rule identical.
- Tests: assertions unchanged; only `use` lines and paths change. A test that needs a changed name is a bug in the move, not a reason to edit the assertion.

## 4. The automatic cycle check (ADR-140)
- File: `tests/module_cycles.rs` (integration test; runs in `cargo test`, so in CI and in the coverage run; annotated `// covers: REQ-153`).
- Dev-dependency: `syn = { version = "2", features = ["full", "visit"] }` (the lock already has syn 2.0.119 through the derive crates; no new package; MIT OR Apache-2.0). Check `cargo tree -i syn` after the change: one version of 2.x.
- Algorithm: walk `CARGO_MANIFEST_DIR/src` recursively; for each file, module name = first path segment under src (`lib.rs` and `main.rs` excluded as sources; crate root references count as a node "lib" only if they appear, none today). Parse with `syn::parse_file`. Skip every item with a `#[cfg(test)]` attribute (and any `mod tests` / `*_tests` file: test files are skipped by name, as in `tests`). Collect edges from `use crate::X...` items (UseTree, including `use crate::{a, b}`) and from any `syn::Path` whose first segment is `crate` followed by `X`, where X is a module of `src/`. Comments are not in the AST, so doc links are never edges (this is why the naive text scan was not used).
- Graph: nodes = top-level modules; edges deduplicated; the test runs Tarjan (or DFS with colours) and fails with the cycle path (`a -> b -> a`) and the file of each edge.
- Second rule in the same test: module `base` has no outgoing edge (no `crate::` path at all in src/base/**), with the failure message naming the file.
- Self-tests (no src read; in-memory sources through the same function): (a) two modules using each other -> cycle reported; (b) a reference in a comment or a `#[cfg(test)]` block -> no cycle; (c) `use crate::{a::X, b::Y}` tree handled; (d) three-module cycle reported with its full path; (e) base with a reference -> failure.
- The real-tree test asserts no cycle and that base has no outgoing edge. A new module is found automatically (walk, not a list).
- Known limits (Future work): `#[path]` attributes and `include!` are not followed (none in the tree today; the test fails if a `#[path]` attribute appears in src, to avoid a silent gap); macro-generated paths are not seen; test code is not part of the rule (ADR-140).
- Rejected: a grep in `scripts/` (not run by `cargo test`; text scan gave false positives on doc links); an external graph tool (not in CI, extra tool to install); the check in the feature script (feature-coverage is a REQ check, not a structure check).

## 5. Ordered steps for the Principal Engineer (opus or sonnet, rule 13 applies)
1. Add `syn` dev-dependency; write `tests/module_cycles.rs` with the self-tests (a)-(e) and the real-tree test. Expect the real-tree test to fail until step 6 (it lists the pairs).
2. Create `src/base/` (mod, io, private, pos, names, language). Move the items of table 2.1 with bodies unchanged; `read_limited` calls `base::io::read_capped`; `project::project_dir` calls `base::io::parent_dir`.
3. Re-point callers (about 45 files): use lines and paths only; `settings` uses `base::io` directly; `scratch` keeps default_root and cleanup_stale, uses base::names and base::private.
4. Move `AssEvent`, `font_px`, `BASE_HEIGHT` into `src/ass.rs`; re-point text, ass, render, app, export, tests.
5. Move `align_h`, `align_v` into base::pos; re-point ui/boxes and app/sphere.rs. app/sphere.rs must have no `crate::ui` path.
6. Re-point `player/src/*.rs` (Pos, read_limited).
7. Annotate: `// covers: REQ-151` on the existing tests that prove the invariants (render golden and project fixtures from 2.0.0, settings load/save tests, the work-folder name test) and add one test that pins the names of section 3 (lock, targets, job-, temp file, partial suffixes, SPEECH_MODEL_FILES equal to builtin_models files). `// covers: REQ-152` on the existing REQ-057 and REQ-059 tests (keep their annotations). `// covers: REQ-153` on tests/module_cycles.rs.
8. Run the suite (fmt, clippy -D warnings, test, coverage --fail-under 85, feature-coverage). Report counts. Then update docs/code-architecture.md § Cycles and the layers table (PE's document: add the app <-> ui pair and the base layer; the 16-module group becomes none) and report the line coverage.
9. Commit with one line; no push unless asked.

## 6. Tests (mapping to REQ)
- REQ-151: golden/fixture tests of 2.0.0 files (project, settings, cache names), invariant names test, existing tests unchanged in assertions.
- REQ-152: existing REQ-057 (bad files) and REQ-059 (temp files) tests, unchanged.
- REQ-153: tests/module_cycles.rs (real tree plus self-tests).
- Coverage: base/* gets its tests through the callers (io tests for capped and atomic writes can stay in the moved modules; the moved test bodies follow their code).

## 7. NFR, security, performance
- Performance: no runtime change (same code moved; no extra allocation). Compile time: a smaller dependency graph inside the crate; not measured. Test time: syn parse of about 60k lines in debug: an estimate of 1 to 3 s, to measure in step 8 (CI time QG-06).
- Security: the capped read, regular-file and non-blocking open rules move with read_limited (REQ-057 unchanged). No network, no new runtime crate, no unsafe (`#![deny(unsafe_code)]` stays). Supply chain: syn is a dev-dependency already in the lock; cargo-deny licence list already allows MIT/Apache-2.0.
- Portability: unchanged (the cfg blocks move with their bodies: Unix mode bits, macOS/Linux O_NONBLOCK, Windows lock).

## 8. Risks
- Large churn (about 45 files) with mechanical edits: compile errors catch all wrong paths; a wrong literal in names.rs would change a file name: pinned by the test in step 7.
- The check is syntactic: a cycle through a macro or `#[path]` is not seen (section 4, known limits).
- ass/text split: AssEvent moves into ass; if text's tests depend on private helpers, they move with the code.
- Documentation drift: docs/code-architecture.md is the PE's; the PE updates it in step 8 (architect verifies in RELEASE).

## 9. Future work (not in 2.0.1)
- Batch 1 of the roadmap ("verify the release") moves to a later version (business decision).
- The layering table in docs/code-architecture.md calls settings, fonts, notices "Base": after 2.0.1 the real base is `base`; rename the layer in the PE document (PE, release consolidation).
- Large files app.rs (about 3000 lines), render.rs (about 3400 lines): debt, not split in 2.0.1.
- docs/architecture.md is about 160 lines: split its Components table into docs/architecture-components.md at the next release (architect).
- A `#[path]` or `include!` in src would need the check extended (test fails on them now).
