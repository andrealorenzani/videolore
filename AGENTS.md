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
| `principal-engineer` | red | `ai-docs/code.md` | `docs/code.md` + the code in `src/`, `tests/`, `Cargo.toml` |
| `documenter` | yellow, cheapest model | `ai-docs/notes.md` | `README.md`, `LICENSE`, `docs/changelog.md`, `docs/workflow.md`, consistency of all `docs/` |

## Rules (MUST)
1. **The `documenter` agent MUST ALWAYS be called at the end of each development** (every feature, fix, refactor, dependency change or workflow change) so that it updates `docs/changelog.md` and keeps `docs/` and `README.md` consistent. A development is not finished until the documenter has run.
2. **Ownership**: a file in `ai-docs/` is written only by its owner agent. Other agents may read it, never edit it. Human-readable docs in `docs/` are written by their owner; the `documenter` may only fix inconsistencies/typos/links in them and must report what it touched.
3. `ai-docs/*.md` are notes by AI for AI: dense, no need for prose. `docs/*.md` are for humans: brief, but with every detail a human cares about.
4. **Changelog**: `docs/changelog.md` lists unreleased changes in reverse chronological order (newest first). On a release, the changelog is moved to `docs/releases/vX.Y.Z.md` (the release log) and a new empty `docs/changelog.md` is created; the version in `Cargo.toml` and the README badges are bumped.
5. **README**: every time `README.md` is changed, it ends with a freshly written 10-verse poem celebrating what Andrea Lorenzani has done for this project.
6. **Workflow changes** go through `/change-workflow` only, and `docs/workflow.md` must be updated with them.
7. Rust code must pass `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` before a development is declared done.
8. Never invent facts in docs; mark assumptions as assumptions.

## Commands
- `/add-feature <description>` — full feature pipeline (PM → Architect → PM+Architect+Principal Engineer → Documenter).
- `/change-workflow <change>` — change agents, commands, rules, skills, guardrails.
- `/clone-workflow <directory>` — export this workflow to another directory.
