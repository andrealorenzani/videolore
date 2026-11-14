# AGENTS.md — rules for every AI agent working on this repository

`CLAUDE.md` is a symlink to this file. Edit only `AGENTS.md`.
The full description of the workflow lives in `docs/workflow.md`; read it before changing anything in `.claude/`.

## Project
- `videolore` — a Rust project (edition 2024, MSRV 1.100, always the latest stable Rust). Version source of truth: `Cargo.toml` (`version`).
- License: GPL-3.0 (`LICENSE`). Author and owner: Andrea Lorenzani.

## Agents and the documents they own
| Agent | Color | Private notes (AI-only) | Human-readable doc |
|---|---|---|---|
| `product-manager` | blue | `ai-docs/business.md` | `docs/business.md` |
| `architect` | green | `ai-docs/architecture.md` | `docs/architecture.md` |
| `principal-engineer` | red | `ai-docs/code.md` | `docs/code.md` + the code in `src/`, `tests/`, `Cargo.toml` (except `version`), `.github/workflows/` |
| `documenter` | yellow, cheapest model | `ai-docs/notes.md` | `README.md`, `LICENSE`, `docs/changelog.md`, `docs/roadmap.md`, `docs/workflow.md`, the version (via `scripts/version.sh`), consistency of all `docs/` |

## Rules (MUST)
1. **The `documenter` agent MUST ALWAYS be called at the end of each development** (every feature, fix, refactor, dependency change or workflow change) so that it updates `docs/changelog.md` and keeps `docs/` and `README.md` consistent. A development is not finished until the documenter has run.
2. **Ownership**: a file in `ai-docs/` is written only by its owner agent. Other agents may read it, never edit it. Human-readable docs in `docs/` are written by their owner; the `documenter` may only fix inconsistencies/typos/links in them and must report what it touched.
3. `ai-docs/*.md` are notes by AI for AI: dense, no need for prose. `docs/*.md` are for humans: brief, but with every detail a human cares about.
3b. **Sub-documents**: an owner may split its human doc `docs/<area>.md` into `docs/<area>-<topic>.md` (e.g. `docs/business-ui.md`, `docs/business-export.md`) and should do it when the doc passes ~150 lines or mixes distinct topics. `docs/<area>.md` stays as the index linking every sub-document. Sub-documents have the same owner and rules as their index.
4. **Versioning — every change bumps the version, everywhere.** Each development (feature, fix, refactor, dependency or workflow change) gets a new version. If the user names the version, use it; otherwise increment the **last number by one** (0.1.0 → 0.1.1 → 0.1.2); after a `/release` the version moves to the next **major** (0.4.1 released → 1.0.0, 2.0.0, …). The version is changed **only** with `scripts/version.sh set X.Y.Z`, which updates every location (`Cargo.toml`, `Cargo.lock`, README version badge) and verifies them; `scripts/version.sh check` must pass before every commit (CI enforces it). A new place that shows the version must be added to that script.
4b. **Changelog**: `docs/changelog.md` lists the changes since the last release in reverse chronological order (newest first); each entry header is `## <version> — <YYYY-MM-DD HH:MM +zone>` (from `date '+%Y-%m-%d %H:%M %z'`).
4c. **Releases** (`/release` only): the **current** version is released. The Documenter prepares the docs, then `scripts/collect-release.sh` creates `releases/<version> - <name>/` with a copy of all docs unchanged (`README.md`, `docs/`, `ai-docs/`), `sources.zip` and `RELEASE_NOTES.md` (taken from the changelog); the changelog is cleaned; commit, tag `v<version>`, push; then the version moves to the next major (`scripts/version.sh set`), commit, push. **Files under `releases/` are never modified by anyone** (denied in `.claude/settings.json`, checked by `scripts/release-guard.sh` in CI); only Andrea deletes old snapshots, manually.
4d. **Roadmap**: `docs/roadmap.md` (owned by the Documenter) lists the known limitations and improvements to do in future versions. Every agent returns a `## Future work` section with anything important for the future; the command collects these items (and the features a plan postpones from a request) and passes them to the Documenter, who keeps the roadmap current.
5. **README**: it is for the users of the software. It must contain **no reference to AI, agents, commands or this workflow** (`ai-docs/`, `docs/workflow.md`, `AGENTS.md`, `.claude/` are never mentioned). "Latest changes" lists **only the last 3 versions** with user-facing changes, by **version, never date** (`- **0.1.1** — ...`), with a link to the changelog and the release notes for older ones. Every time `README.md` is changed, it ends with a freshly written 10-verse poem celebrating what Andrea Lorenzani has done for this project; every verse except the last ends with a backslash `\` so GitHub renders one verse per line.
6. **Workflow changes** go through `/change-workflow` only, and `docs/workflow.md` must be updated with them.
7. **Tests are run only by the `principal-engineer`, after coding.** The quality suite is: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `scripts/coverage.sh --fail-under 85`, `scripts/feature-coverage.sh`. The Principal Engineer runs it once the code is written; if anything fails it fixes the **code** (never weakens or deletes a test to get green) and reruns the whole suite, until it passes. It then reports the results, the **line coverage** (one decimal) and the **feature coverage** (`implemented REQs with tests / implemented REQs`). No other agent, and no command orchestrator, runs tests, builds or coverage: they use the Principal Engineer's report. CI on GitHub runs the suite again independently.
7b. **Coverage**: line coverage **at least 85%** (`scripts/coverage.sh --fail-under 85`, `src/main.rs` excluded: it only wires `src/lib.rs`), and **feature coverage 100%**: every requirement marked `REQ-NNN (implemented)` in `docs/business.md` has at least one test annotated `// covers: REQ-NNN` (`scripts/feature-coverage.sh`). The README coverage badge shows the value reported by the Principal Engineer with one decimal (e.g. `86.2%`), written by the documenter with `scripts/coverage.sh --badge <value>` (no test run).
8. Never invent facts in docs; mark assumptions as assumptions.
9. **Commits**: every development ends with a git commit whose message is a single human-readable line (e.g. `Add prebuilt executables for Linux, macOS and Windows`): no prefixes like `feat:`, no body, no trailers, no mention of agents or AI. Push only when the user asks.
10. **Licence notice**: `LICENSE` holds the unmodified GPL-3.0 text. The copyright holder (Andrea Lorenzani) appears in the README licence section and in a header at the top of every Rust source file: `// SPDX-License-Identifier: GPL-3.0-only` and `// Copyright (C) <year> Andrea Lorenzani`.
11. **CI**: `.github/workflows/build.yml` checks fmt/clippy/test, coverage ≥85%, feature coverage 100%, version consistency and frozen release docs, then builds release executables (Linux, macOS, Windows) on every push to `main`; pushing a `v*` tag creates the GitHub release with the executables and the release log. Keep it green.

## Commands
- `/add-feature <description>` — full feature pipeline (PM → Architect → PM+Architect+Principal Engineer → Documenter).
- `/change-workflow <change>` — change agents, commands, rules, skills, guardrails.
- `/clone-workflow <directory>` — export this workflow to another directory.
- `/release [name]` — release the current version into `releases/<version> - <name>/` (docs, sources zip, release notes), clean the changelog, tag `v<version>` and push, then move to the next major version and push.

## Scripts (`scripts/`)
`version.sh` (get/next-patch/next-major/set/check), `coverage.sh`, `feature-coverage.sh`, `collect-release.sh` (release snapshot), `release-guard.sh`, `test-annotations.sh` (CI test failures as annotations), `notices.sh`, `icons.sh`, `fetch-ffmpeg.sh`, `package.sh` (installers).
