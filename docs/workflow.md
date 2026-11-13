# Agentic workflow

How AI agents build this project: who does what, which commands exist, which rules apply, and where every piece lives. This file is the source of truth for `/change-workflow` and `/clone-workflow`.

## Agents

| Agent | File | Color | Model | Focus | Private notes | Human doc |
|---|---|---|---|---|---|---|
| Product Manager | `.claude/agents/product-manager.md` | blue | inherit | business, requirements, functionalities | `ai-docs/business.md` | `docs/business.md` |
| Architect | `.claude/agents/architect.md` | green | inherit | architecture, NFRs, security, performance, technology choices | `ai-docs/architecture.md` | `docs/architecture.md` |
| Principal Engineer | `.claude/agents/principal-engineer.md` | red | inherit | code, tests, code patterns, implementation security | `ai-docs/code.md` | `docs/code.md` (+ code) |
| Documenter | `.claude/agents/documenter.md` | yellow | haiku (cheapest) | docs consistency, README, changelog, workflow doc, licence | `ai-docs/notes.md` | `README.md`, `docs/changelog.md`, `docs/workflow.md`, `LICENSE` |

- `ai-docs/*.md` are notes written by AI for AI; only the owner edits them.
- `docs/*.md` are for humans: brief, with every interesting detail.
- PM, Architect and Documenter have **PLAN** (read-only, returns plan + questions) and **UPDATE** (writes own docs) modes; the caller picks the mode.
- Agent colors available in Claude Code: red, blue, green, yellow, purple, orange, pink, cyan.

## Commands

| Command | File | What it does |
|---|---|---|
| `/add-feature <text>` | `.claude/commands/add-feature.md` | Full feature pipeline (below). |
| `/change-workflow <text>` | `.claude/commands/change-workflow.md` | Changes agents/commands/rules/skills/guardrails. Calls no project agent; may consult an advisor. Updates this file. |
| `/clone-workflow <dir>` | `.claude/commands/clone-workflow.md` | Exports the workflow to another directory using the manifest below. |

### `/add-feature` pipeline
```
user feature
  -> product-manager (PLAN) -> [SEVERE ambiguities? ask user -> product-manager (PLAN, final)]
  -> compact brief (request + business plan + answers)
  -> architect (PLAN) -> [SEVERE PRODUCT-CHANGING questions? ask user -> architect (PLAN, final)]
  -> parallel: product-manager (UPDATE) | architect (UPDATE) | principal-engineer (implement + tests)
  -> documenter (changelog, README + poem, docs consistency)
  -> report to user
```
MINOR ambiguities and TECHNICAL questions are decided by the agents and recorded as assumptions/decisions.

## Rules (full text in `AGENTS.md`)
1. The Documenter is ALWAYS called at the end of each development.
2. Each agent edits only its own `ai-docs/` file; the Documenter may fix consistency in `docs/` and reports it.
3. **Every change bumps the version**: the user's version if named, otherwise last number +1 (0.1.1 → 0.1.2). `/add-feature` decides it in Phase 0 and the Documenter writes it to `Cargo.toml`; `/change-workflow` bumps it itself.
4. Changelog in reverse chronological order, each entry `## <version> — <YYYY-MM-DD HH:MM +zone>`; on release it becomes `docs/releases/vX.Y.Z.md` and a new empty changelog is created.
5. README is for users: no reference to AI/agents/workflow; "Latest changes" by version, never by date; every README change ends with a new 10-verse poem for Andrea Lorenzani, verses 1-9 ending with `\` so GitHub shows one verse per line.
6. Workflow changes only via `/change-workflow`.
7. Code passes `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`; CI enforces it.
8. Every development ends with a commit with a single human-readable line (no prefixes, body or trailers). Push only on request.
9. `LICENSE` is the unmodified GPL-3.0 text; Andrea Lorenzani's copyright is in the README and in the header of each `.rs` file.

## Skills
None yet. Project skills go in `.claude/skills/<name>/SKILL.md`.

## CI
`.github/workflows/build.yml`: on push to `main`, PRs and `v*` tags. Job `check` runs fmt/clippy/test; job `build` makes release executables for Linux x86_64, macOS arm64, Windows x86_64 and uploads them as artifacts; on a `v*` tag they are attached to the GitHub release. Owned by the Principal Engineer.

## Hooks and settings
None yet. Enforceable guardrails can go in `.claude/settings.json`.

## Repository
- Remote `origin`: `git@github.com:andrealorenzani/videolore.git`, default branch `main`.

## Where things live
```
AGENTS.md                 rules for all agents (CLAUDE.md -> symlink to it)
.claude/agents/           agent definitions
.claude/commands/         slash commands
.claude/skills/           project skills
ai-docs/                  AI-only notes, one file per agent
docs/                     human docs (business, architecture, code, changelog, workflow)
docs/releases/            release logs (old changelogs)
README.md, LICENSE        written by the Documenter
.github/workflows/        CI: checks + executables
```

## Workflow manifest
Used by `/clone-workflow`.

| Path | On clone |
|---|---|
| `.claude/agents/`, `.claude/commands/`, `.claude/skills/` | copy verbatim |
| `.claude/settings.json` (if present) | copy verbatim |
| `AGENTS.md` | copy, Project section replaced by a placeholder |
| `CLAUDE.md` | recreate as symlink to `AGENTS.md` |
| `docs/workflow.md` | copy, add a "cloned from" history line |
| `LICENSE` | copy verbatim |
| `.github/workflows/build.yml` | copy verbatim (binary name is read from `Cargo.toml`) |
| `ai-docs/business.md`, `ai-docs/architecture.md`, `ai-docs/code.md`, `ai-docs/notes.md` | create empty template |
| `docs/business.md`, `docs/architecture.md`, `docs/code.md`, `docs/changelog.md`, `docs/releases/` | create empty template |
| `src/`, `Cargo.*`, `target/`, `README.md`, `.git/` (and the `origin` remote) | never copy |

## Workflow history
- 2026-11-13 — First draft: 4 agents (Product Manager, Architect, Principal Engineer, Documenter), 3 commands (`/add-feature`, `/change-workflow`, `/clone-workflow`), rules in `AGENTS.md`.
- 2026-11-13 — Documenter color set to yellow (gray is not supported); repo linked to GitHub `origin`.
- 2026-11-13 — v0.1.1: version bump on every change; changelog with version + date-time; README without AI/workflow references, versions instead of dates, poem line breaks; one-line commits; copyright headers; GitHub Actions build of executables.
