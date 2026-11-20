# Architecture: code structure (2.0.1)

Part of [Architecture](architecture.md). The module dependency graph and the list of cycles as found in 2.0.0 are in [code architecture](code-architecture.md#cycles). This page describes the change made in 2.0.1 (REQ-151..153, see [performance](business-performance.md#code-structure-201)). It changes no behaviour, no file format, no setting, no file name and no output.

## Why
Nine pairs of modules used each other, and sixteen modules formed one group in which no module could be used or tested alone. The app also depended on the interface (one import of the window's box geometry). Rust allows this inside one program; the cost is that the shared helpers (capped file reading, atomic writing, private work folders, positions, language names, cache and speech-model file names) lived in the modules that used them, so every user depended on every other.

## The base module
One module `base` in `src/base/` holds the shared helpers. It imports no other module of videolore; it uses only the standard library and the external crates the program already has.

| File | Contents | Used by |
|---|---|---|
| `base/io.rs` | capped reading of a regular file (at most a given size; folders, pipes and devices refused, non-blocking open), capped reading of a stream, atomic writing (temporary file in the same folder, fsync, rename), the parent folder of a path | projects, settings, LUT files, speech-track files, the work-folder registry |
| `base/private.rs` | private folders (0700 on Unix), private files (0600), `Work` (a job's folder with a lock, removed on drop), publishing a finished file (0644), the names of the lock and registry files | export, preview copies, speech recognition, 360 marker |
| `base/pos.rs` | `Pos` and `POS_MAX` (position of a line or text on screen), `Anchor360` (point of the 360 sphere), the alignment helpers | subtitles, texts, timeline, camera, frame, interface, application |
| `base/names.rs` | the cache file rule (`<32 hex>.mkv`, `.partial`), the partial-file rule of the work folders, the three speech-model file names | proxy cache, cleanup at start-up, speech recognition |
| `base/language.rs` | the table of language codes, speech-model codes and English names | speech recognition, media (track names) |

The module list of 2.0.1 is in [code architecture](code-architecture.md). `scratch` keeps the work-folder root (`default_root`) and the cleanup at start-up; `ass` keeps the subtitle-rendering rules that texts use.

## Cycles broken
| Pair (2.0.0) | Import removed | Now |
|---|---|---|
| project and settings | both directions (file reading and atomic writing) | both use `base::io` |
| stt and media | media used the language table of stt | `base::language` |
| stt and scratch | stt used the work folders; scratch used the model names | `base::private`, `base::names` |
| scratch and proxy | scratch used the cache file rule; proxy used the work folders | `base::names`, `base::private` |
| lut and project | lut read its files through project | `base::io` |
| subtitles and timeline | timeline used the position type of subtitles | `base::pos` |
| subtitles and text | subtitles used the 360 anchor type of texts | `base::pos` |
| ass and text | ASS rules (`AssEvent`, font size, base height) lived in text | they move into `ass` (text uses ass) |
| app and ui | app used the box alignment helpers of the interface | `base::pos` |

Also removed inside the group: camera used the anchor type of text, frame used the position type of subtitles, scratch and speech recognition read small files through project. After the moves no cycle remains, and `base` depends on nothing.

Public paths: nothing is re-exported. Each moved item has one path in its new module; the player uses the new paths too. The folder of the default work root keeps its path (`scratch::default_root`).

## Automatic check
`cargo test` runs `tests/module_cycles.rs` (so CI and the coverage run run it):
- it reads every `.rs` file under `src/` with `syn` (a development dependency; it is already in the lock file through the derive crates, so no new package is added; MIT or Apache-2.0);
- it skips comments, `#[cfg(test)]` items and test files, so only the code that ships counts;
- it builds the graph of top-level modules from every `crate::` path and every `use crate::` line, and fails with the path of the cycle (`a -> b -> a`) and the files of its edges;
- it fails if `base` refers to any module, and if a `#[path]` attribute appears in `src/` (the check would not follow it);
- its own tests give it small fixtures: a two-module cycle, a three-module cycle, a reference in a comment or a test block (no cycle), a grouped `use crate::{...}`, and a reference from `base` (failure).
A new module is found without a list, and a new cycle fails the build.

## What does not change
- Files: project format 9, settings file (version 2), cache file names, work-folder names (`job-` prefix, `lock`, `targets`), temporary file names (`.<name>.tmp-<pid>`), speech-model names, partial suffixes. A 2.0.0 project, settings file and cache folder work in 2.0.1 with no migration and no notice.
- Errors: the same kinds and messages (for example "invalid file name", "file too large", "not a regular file"). Damaged files are still refused or replaced as in REQ-057, and temporary files are still private and cleaned up as in REQ-059.
- Tests: the existing tests keep their assertions; only their `use` lines change. The new tests pin the names above.

## Non-functional
- Speed and memory: the same code moved; no new work at run time.
- Compile time: not measured (expected: unchanged or slightly lower).
- Test time: the structure check parses the source once; estimate 1 to 3 s in a debug build (to be measured in CI, QG-06).
- Portability: the platform-specific code moves with its bodies (Unix permission bits, Linux and macOS non-blocking open, Windows lock behaviour); no new platform code.

## Security
No change to the rules. Capped reads, regular-file checks and atomic writes now live in one place (`base/io.rs`); the protections of REQ-057 are the same. No network, no unsafe code (`#![deny(unsafe_code)]` stays), no new runtime dependency.

## Tests for 2.0.1
- REQ-151: the 2.0.0 fixtures (projects, settings, cache names) keep working; a test pins the names listed above.
- REQ-152: the existing REQ-057 and REQ-059 tests, unchanged.
- REQ-153: `tests/module_cycles.rs` (the real tree and the fixtures).

## Risks and later work
- The check is syntactic: a cycle created through a macro is not seen. Known gap, accepted for 2.0.1.
- The large files `app.rs` and `render.rs` remain large (debt, not split in 2.0.1).
- Batch 1 of the roadmap ("verify the release") moves to a later version.
