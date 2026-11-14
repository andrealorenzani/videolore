# AGENTS.md — rules for every AI agent working on this repository

`CLAUDE.md` is a symlink to this file. Edit only `AGENTS.md`.
The full description of the workflow lives in `docs/workflow.md`; read it before changing anything in `.claude/`.

## Project
- `videolore` — a Rust project (edition 2024, MSRV 1.88). Version source of truth: `Cargo.toml` (`version`).
- License: GPL-3.0 (`LICENSE`). Author and owner: Andrea Lorenzani.

## Agents and the documents they own
| Agent | Color | Private notes (AI-only) | Human-readable doc |
|---|---|---|---|
| `product-manager` | blue | `ai-docs/business.md` | `docs/business.md` |
| `architect` | green | `ai-docs/architecture.md` | `docs/architecture.md` |
| `principal-engineer` | red | `ai-docs/code.md` | `docs/code.md` + the code in `src/`, `tests/`, `Cargo.toml` (except `version`), `.github/workflows/` |
| `documenter` | yellow, cheapest model | `ai-docs/notes.md` | `README.md`, `LICENSE`, `docs/changelog.md`, `docs/workflow.md`, the version (via `scripts/version.sh`), consistency of all `docs/` |

## Rules (MUST)
1. **The `documenter` agent MUST ALWAYS be called at the end of each development** (every feature, fix, refactor, dependency change or workflow change) so that it updates `docs/changelog.md` and keeps `docs/` and `README.md` consistent. A development is not finished until the documenter has run.
2. **Ownership**: a file in `ai-docs/` is written only by its owner agent. Other agents may read it, never edit it. Human-readable docs in `docs/` are written by their owner; the `documenter` may only fix inconsistencies/typos/links in them and must report what it touched.
3. `ai-docs/*.md` are notes by AI for AI: dense, no need for prose. `docs/*.md` are for humans: brief, but with every detail a human cares about.
3b. **Sub-documents**: an owner may split its human doc `docs/<area>.md` into `docs/<area>-<topic>.md` (e.g. `docs/business-ui.md`, `docs/business-export.md`) and should do it when the doc passes ~150 lines or mixes distinct topics. `docs/<area>.md` stays as the index linking every sub-document. Sub-documents have the same owner and rules as their index.
4. **Versioning — every change bumps the version, everywhere.** Each development (feature, fix, refactor, dependency or workflow change) gets a new version. If the user names the version, use it; otherwise increment the **last number by one** (0.1.0 → 0.1.1 → 0.1.2); a `/release` increments the **major** (→ 1.0.0, 2.0.0, …). The version is changed **only** with `scripts/version.sh set X.Y.Z`, which updates every location (`Cargo.toml`, `Cargo.lock`, README version badge) and verifies them; `scripts/version.sh check` must pass before every commit (CI enforces it). A new place that shows the version must be added to that script.
4b. **Changelog**: `docs/changelog.md` lists the changes since the last release in reverse chronological order (newest first); each entry header is `## <version> — <YYYY-MM-DD HH:MM +zone>` (from `date '+%Y-%m-%d %H:%M %z'`).
4c. **Releases** (`/release` only): bump the major version first, then freeze `README.md`, `docs/` and `ai-docs/` into `release/<version>/` (its `docs/changelog.md` is the release log), reset `docs/changelog.md`, commit, tag `v<version>` on `main`, push. **Files under `release/` are never modified by anyone** (denied in `.claude/settings.json`, checked by `scripts/release-guard.sh` in CI); only Andrea deletes old snapshots, manually.
5. **README**: it is for the users of the software. It must contain **no reference to AI, agents, commands or this workflow** (`ai-docs/`, `docs/workflow.md`, `AGENTS.md`, `.claude/` are never mentioned). "Latest changes" lists **versions, never dates** (`- **0.1.1** — ...`), and only changes that matter to users. Every time `README.md` is changed, it ends with a freshly written 10-verse poem celebrating what Andrea Lorenzani has done for this project; every verse except the last ends with a backslash `\` so GitHub renders one verse per line.
6. **Workflow changes** go through `/change-workflow` only, and `docs/workflow.md` must be updated with them.
7. Rust code must pass `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` before a development is declared done.
7b. **Coverage**: line coverage **at least 85%** (`scripts/coverage.sh --fail-under 85`, `src/main.rs` excluded: it only wires `src/lib.rs`), and **feature coverage 100%**: every requirement marked `REQ-NNN (implemented)` in `docs/business.md` has at least one test annotated `// covers: REQ-NNN` (`scripts/feature-coverage.sh`). The README coverage badge shows the current value with one decimal (e.g. `86.2%`), updated with `scripts/coverage.sh --update-readme` at every development.
8. Never invent facts in docs; mark assumptions as assumptions.
9. **Commits**: every development ends with a git commit whose message is a single human-readable line (e.g. `Add prebuilt executables for Linux, macOS and Windows`): no prefixes like `feat:`, no body, no trailers, no mention of agents or AI. Push only when the user asks.
10. **Licence notice**: `LICENSE` holds the unmodified GPL-3.0 text. The copyright holder (Andrea Lorenzani) appears in the README licence section and in a header at the top of every Rust source file: `// SPDX-License-Identifier: GPL-3.0-only` and `// Copyright (C) <year> Andrea Lorenzani`.
11. **CI**: `.github/workflows/build.yml` checks fmt/clippy/test, coverage ≥85%, feature coverage 100%, version consistency and frozen release docs, then builds release executables (Linux, macOS, Windows) on every push to `main`; pushing a `v*` tag creates the GitHub release with the executables and the release log. Keep it green.

## Commands
- `/add-feature <description>` — full feature pipeline (PM → Architect → PM+Architect+Principal Engineer → Documenter).
- `/change-workflow <change>` — change agents, commands, rules, skills, guardrails.
- `/clone-workflow <directory>` — export this workflow to another directory.
- `/release [version]` — major version bump, freeze docs in `release/<version>/`, reset changelog, tag and push.

## Scripts (`scripts/`)
`version.sh` (get/next-patch/next-major/set/check), `coverage.sh`, `feature-coverage.sh`, `collect-release.sh`, `release-guard.sh`.
