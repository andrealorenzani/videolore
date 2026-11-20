# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

## 2.0.1 — 2026-11-20 20:06 +0000
- Changed: business requirements for the code structure (REQ-151..153, delivered): the shared helpers move to one common module, with no change in behaviour, no module cycle and an automatic check that fails when a cycle is added.
- Changed: architecture for 2.0.1 (see [code structure](architecture-structure.md)): a new `base` module in `src/base/` for the shared helpers (file reading and writing, private work folders, positions, names, language table); the module cycles are broken; the automatic cycle check runs in `cargo test`; `syn` is a development dependency (already in the lock file, no new package).
- Changed: code: the shared helpers of `src/` moved into `src/base/` (`io`, `private`, `pos`, `names`, `language`) and every module now uses them from there; the module cycles are gone (the top-level modules form no cycle); `tests/module_cycles.rs` fails `cargo test` on a cycle and has its own fixture tests; `syn` 2 added as a development dependency only. No file format, setting, file name or output changes.

## 2.0.0 — 2026-11-20 19:50 +0000
- Version 2.0.0 opened after release 1.6.3 "core effects and player". New README poem. Project page regenerated for release 1.6.3.
