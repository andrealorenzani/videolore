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
- `docs/*.md` are for humans: brief, with every interesting detail. Each owner may split its doc into `docs/<area>-<topic>.md` sub-documents (and should past ~150 lines or with distinct topics); `docs/<area>.md` remains the index. Example: `docs/business.md` → `docs/business-editing.md`, `docs/business-export.md`, `docs/business-ui.md`.
- PM, Architect and Documenter have **PLAN** (read-only, returns plan + questions) and **UPDATE** (writes own docs) modes; the caller picks the mode.
- Agent colors available in Claude Code: red, blue, green, yellow, purple, orange, pink, cyan.

## Commands

| Command | File | What it does |
|---|---|---|
| `/add-feature <text>` | `.claude/commands/add-feature.md` | Full feature pipeline (below). |
| `/change-workflow <text>` | `.claude/commands/change-workflow.md` | Changes agents/commands/rules/skills/guardrails. Calls no project agent; may consult an advisor. Updates this file. |
| `/clone-workflow <dir>` | `.claude/commands/clone-workflow.md` | Exports the workflow to another directory using the manifest below. |
| `/release [version]` | `.claude/commands/release.md` | Major release: gates → `scripts/version.sh set <next major>` → Documenter (release entry, README, poem) → freeze `README.md`, `docs/`, `ai-docs/` into `release/<version>/` → reset changelog → commit `Release <version>` → tag `v<version>` on `main` → push. The tag makes CI create the GitHub release with executables. |

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
3. **Every change bumps the version, everywhere**: the user's version if named, otherwise last number +1 (0.1.1 → 0.1.2); `/release` bumps the major. Always through `scripts/version.sh set` (Cargo.toml, Cargo.lock, README badge), verified by `scripts/version.sh check` locally and in CI. `/add-feature` decides it in Phase 0 and the Documenter applies it; `/change-workflow` and `/release` apply it themselves.
4. Changelog in reverse chronological order, each entry `## <version> — <YYYY-MM-DD HH:MM +zone>`; on release it is frozen as `release/<version>/docs/changelog.md` (release log) and reset.
4b. `release/<version>/` snapshots (README, docs, ai-docs) are immutable: deny rule `Edit(release/**)` in `.claude/settings.json` + `scripts/release-guard.sh` in CI. Andrea deletes old ones manually.
5. README is for users: no reference to AI/agents/workflow; "Latest changes" by version, never by date; every README change ends with a new 10-verse poem for Andrea Lorenzani, verses 1-9 ending with `\` so GitHub shows one verse per line.
6. Workflow changes only via `/change-workflow`.
7. Code passes `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`; line coverage ≥ 85% and feature coverage 100% (`REQ-NNN (implemented)` in `docs/business.md` and `docs/business-*.md` ↔ `// covers: REQ-NNN` in tests); README coverage badge with one decimal. CI enforces all of it.
8. Every development ends with a commit with a single human-readable line (no prefixes, body or trailers). Push only on request.
9. `LICENSE` is the unmodified GPL-3.0 text; Andrea Lorenzani's copyright is in the README and in the header of each `.rs` file.

## Skills
None yet. Project skills go in `.claude/skills/<name>/SKILL.md`.

## CI
`.github/workflows/build.yml`: on push to `main`, PRs and `v*` tags. Job `check` (latest stable Rust) runs fmt, clippy (default, no and all features), tests, version consistency, coverage ≥ 85%, feature coverage, release guard; job `msrv` builds and tests with the `rust-version` read from `Cargo.toml`; job `build` runs only if both pass and makes release executables for Linux x86_64, macOS arm64, Windows x86_64 and uploads them as artifacts; on a `v*` tag the GitHub release is created with them and with `release/<version>/docs/changelog.md` as notes. Owned by the Principal Engineer.

## Scripts
| Script | Purpose |
|---|---|
| `scripts/version.sh` | `get`, `next-patch`, `next-major`, `set X.Y.Z` (all locations, all-or-nothing: restored if any step fails; no build needed), `check` |
| `scripts/coverage.sh` | line coverage with one decimal (`cargo llvm-cov`), `--update-readme`, `--fail-under N` |
| `scripts/feature-coverage.sh` | every implemented `REQ-NNN` (in `docs/business.md` and `docs/business-*.md`) has a `covers:` test |
| `scripts/collect-release.sh` | snapshot README, docs, ai-docs into `release/<version>/` |
| `scripts/release-guard.sh` | fail if a file under `release/` was modified/renamed |

Local coverage needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.

## Hooks and settings
`.claude/settings.json`: `permissions.deny: ["Edit(release/**)"]` — no agent can edit frozen release docs.

## Repository
- Remote `origin`: `git@github.com:andrealorenzani/videolore.git`, default branch `main`.

## Where things live
```
AGENTS.md                 rules for all agents (CLAUDE.md -> symlink to it)
.claude/agents/           agent definitions
.claude/commands/         slash commands
.claude/skills/           project skills
ai-docs/                  AI-only notes, one file per agent
docs/                     human docs (business, architecture, code, changelog, workflow) + their <area>-<topic>.md sub-documents
release/<version>/        frozen docs of each release (never edited)
scripts/                  version, coverage, release helpers
README.md, LICENSE        written by the Documenter
.github/workflows/        CI: checks + executables
```

## Workflow manifest
Used by `/clone-workflow`.

| Path | On clone |
|---|---|
| `.claude/agents/`, `.claude/commands/`, `.claude/skills/` | copy verbatim |
| `.claude/settings.json` | copy verbatim |
| `scripts/` | copy verbatim |
| `AGENTS.md` | copy, Project section replaced by a placeholder |
| `CLAUDE.md` | recreate as symlink to `AGENTS.md` |
| `docs/workflow.md` | copy, add a "cloned from" history line |
| `LICENSE` | copy verbatim |
| `.github/workflows/build.yml` | copy verbatim (binary name is read from `Cargo.toml`) |
| `ai-docs/business.md`, `ai-docs/architecture.md`, `ai-docs/code.md`, `ai-docs/notes.md` | create empty template |
| `docs/business.md`, `docs/architecture.md`, `docs/code.md`, `docs/changelog.md`, `release/.gitkeep` | create empty template |
| `docs/<area>-*.md` sub-documents, `src/`, `Cargo.*`, `target/`, `README.md`, `release/<version>/`, `.git/` (and the `origin` remote) | never copy |

## Workflow history
- 2026-11-13 — First draft: 4 agents (Product Manager, Architect, Principal Engineer, Documenter), 3 commands (`/add-feature`, `/change-workflow`, `/clone-workflow`), rules in `AGENTS.md`.
- 2026-11-13 — Documenter color set to yellow (gray is not supported); repo linked to GitHub `origin`.
- 2026-11-13 — v0.1.1: version bump on every change; changelog with version + date-time; README without AI/workflow references, versions instead of dates, poem line breaks; one-line commits; copyright headers; GitHub Actions build of executables.
- 2026-11-13 — v0.1.2: version changed everywhere via `scripts/version.sh`; `/release` command with frozen `release/<version>/` snapshots, tag and push; coverage ≥ 85% and feature coverage 100% for the Principal Engineer; README coverage badge.
- 2026-11-14 — v0.1.4: `scripts/version.sh set` all-or-nothing with rollback and no build; owners may split `docs/<area>.md` into `docs/<area>-<topic>.md` sub-documents with the main doc as index.
- 2026-11-14 — v0.1.5: CI on the latest stable Rust (1.100) and a separate MSRV job; GitHub Actions on current major versions; policy "follow the latest Rust and libraries" recorded in `docs/architecture.md`.
