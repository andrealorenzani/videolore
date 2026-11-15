# Agentic workflow

How AI agents build this project: who does what, which commands exist, which rules apply, and where every piece lives. This file is the source of truth for `/change-workflow` and `/clone-workflow`.

## Agents

| Agent | File | Color | Model | Focus | Private notes | Human doc |
|---|---|---|---|---|---|---|
| Product Manager | `.claude/agents/product-manager.md` | blue | inherit | business, requirements, functionalities | `ai-docs/business.md` | `docs/business.md` |
| Architect | `.claude/agents/architect.md` | green | inherit | architecture, NFRs, security, performance, technology choices | `ai-docs/architecture.md` | `docs/architecture.md` |
| Principal Engineer | `.claude/agents/principal-engineer.md` | red | inherit | code, tests, code patterns, implementation security | `ai-docs/code.md` | `docs/code.md` (+ code) |
| Documenter | `.claude/agents/documenter.md` | yellow | haiku (cheapest) | docs consistency, README, changelog, roadmap, workflow doc, licence | `ai-docs/notes.md` | `README.md`, `docs/changelog.md`, `docs/roadmap.md`, `docs/workflow.md`, `LICENSE` |

- `ai-docs/*.md` are notes written by AI for AI; only the owner edits them.
- `docs/*.md` are for humans: brief, with every interesting detail. Each owner may split its doc into `docs/<area>-<topic>.md` sub-documents (and should past ~150 lines or with distinct topics); `docs/<area>.md` remains the index. Example: `docs/business.md` → `docs/business-editing.md`, `docs/business-export.md`, `docs/business-ui.md`.
- Every agent ends its report with a `## Future work` section (postponed features, limitations, improvements for later); the command collects them and the Documenter records them in `docs/roadmap.md`.
- PM, Architect and Documenter have **PLAN** (read-only, returns plan + questions) and **UPDATE** (writes own docs) modes; the caller picks the mode.
- Agent colors available in Claude Code: red, blue, green, yellow, purple, orange, pink, cyan.

## Commands

| Command | File | What it does |
|---|---|---|
| `/add-feature <text>` | `.claude/commands/add-feature.md` | Full feature pipeline (below). |
| `/change-workflow <text>` | `.claude/commands/change-workflow.md` | Changes agents/commands/rules/skills/guardrails. Calls no project agent; may consult an advisor. Updates this file. |
| `/clone-workflow <dir>` | `.claude/commands/clone-workflow.md` | Exports the workflow to another directory using the manifest below. |
| `/release [name]` | `.claude/commands/release.md` | Releases the **current** version: gates (Principal Engineer VERIFY) → release name (asked if missing) → Documenter (release summary, roadmap, README, poem) → `scripts/collect-release.sh` creates `releases/<version> - <name>/` (all docs unchanged, `sources.zip`, `RELEASE_NOTES.md` from the changelog) → changelog cleaned → commit `Release <version> <name>` → tag `v<version>` → push → watch CI → `scripts/version.sh set <next major>` → commit `Start version <major>` → push. The tag makes CI create the GitHub release with executables, installers and the release notes. |

### `/add-feature` pipeline
```
user feature
  -> product-manager (PLAN) -> [SEVERE ambiguities? ask user -> product-manager (PLAN, final)]
  -> compact brief (request + business plan + answers)
  -> architect (PLAN) -> [SEVERE PRODUCT-CHANGING questions? ask user -> architect (PLAN, final)]
  -> parallel: product-manager (UPDATE) | architect (UPDATE) | principal-engineer (implement, then test loop: run suite -> fix code -> rerun until green; reports line + feature coverage)
  -> documenter (changelog, roadmap from every agent's Future work + postponed features, README + poem, docs consistency)
  -> report to user
```
MINOR ambiguities and TECHNICAL questions are decided by the agents and recorded as assumptions/decisions.

## Rules (full text in `AGENTS.md`)
1. The Documenter is ALWAYS called at the end of each development.
2. Each agent edits only its own `ai-docs/` file; the Documenter may fix consistency in `docs/` and reports it.
3. **Every change bumps the version, everywhere**: the user's version if named, otherwise last number +1 (0.1.1 → 0.1.2); after `/release` the version moves to the next major. Always through `scripts/version.sh set` (Cargo.toml, Cargo.lock, README badge), verified by `scripts/version.sh check` locally and in CI. `/add-feature` decides it in Phase 0 and the Documenter applies it; `/change-workflow` and `/release` apply it themselves.
4. Changelog in reverse chronological order, each entry `## <version> — <YYYY-MM-DD HH:MM +zone>`; on release its entries become `releases/<version> - <name>/RELEASE_NOTES.md` and it is cleaned.
4b. `releases/<version> - <name>/` snapshots (docs, sources zip, release notes) are immutable: deny rules `Edit(releases/**)` and `Write(releases/**)` in `.claude/settings.json` + `scripts/release-guard.sh` in CI. Andrea deletes old ones manually.
4c. `docs/roadmap.md` (Documenter) keeps the known limitations and improvements for future versions, fed by every agent's `## Future work` and by features a plan postpones.
5. README is for users: no reference to AI/agents/workflow; "Latest changes" shows only the last 3 versions, by version, never by date; every README change ends with a new 10-verse poem for Andrea Lorenzani, verses 1-9 ending with `\` so GitHub shows one verse per line.
6. Workflow changes only via `/change-workflow`.
7. **Only the Principal Engineer runs tests**, after coding, looping (fix code, rerun) until the suite passes; it reports line coverage and feature coverage, which everyone else uses (the Documenter writes the badge with `scripts/coverage.sh --badge`). No other agent or command runs tests, builds or coverage; `/release` asks the Principal Engineer for a VERIFY run. CI reruns everything independently. Code passes `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`; line coverage ≥ 85% and feature coverage 100% (`REQ-NNN (implemented)` in `docs/business.md` and `docs/business-*.md` ↔ `// covers: REQ-NNN` in tests); README coverage badge with one decimal. CI enforces all of it.
8. Every development ends with a commit with a single human-readable line (no prefixes, body or trailers). Push only on request.
9. `LICENSE` is the unmodified GPL-3.0 text; Andrea Lorenzani's copyright is in the README and in the header of each `.rs` file.
10. **Token economy** (`AGENTS.md` rule 12): agents read only what their task needs. The Documenter checks all of `docs/` with `scripts/docs-check.sh` and reads in full only the files a development changed; a full prose read happens on `/release`. The Principal Engineer runs the tests quietly and reads summaries and failures.

## Token economy
Measured at 1.2.0 (bytes ÷ 4 ≈ tokens). The main cost is not the number of agents but the notes and docs that get read in full again and again:

| File | Size | Read by |
|---|---|---|
| `ai-docs/architecture.md` | 131 KB | Architect, up to 3 calls per feature |
| `ai-docs/code.md` | 52 KB | Principal Engineer, each development |
| `ai-docs/business.md` | 48 KB | Product Manager, 2–3 calls per feature |
| `docs/` (all, 30 files) | 355 KB | Documenter, each development before 1.2.1 |
| `AGENTS.md` + agent definitions | about 30 KB | every agent, each call |

- **Aggregating agents: not worth it.** Merging the Product Manager and the Architect would save one cold start per feature, but the notes still get read, and one agent would own two notes files (against rule 2). The Documenter stays separate on purpose: it is the cheap model.
- **Compacting the notes: the real saving.** Halving the three notes (231 KB, about 58k tokens per full read of all three) cuts every later read. Each owner does it in its own development (recorded in `docs/roadmap.md`, Workflow). Human docs stay whole: they are the readable record.
- **Docs checks mechanised** (1.2.1): links, sub-document indexes, forbidden terms, changelog headers and the README's latest changes and poem are checked by `scripts/docs-check.sh`, which prints only the problems. Prose consistency is checked only for the files a development changed, and in full on release. Trade-off: a stale fact in an untouched page can survive until the next release.

## Skills
None yet. Project skills go in `.claude/skills/<name>/SKILL.md`.

## CI
`.github/workflows/build.yml`: on push to `main`, PRs and `v*` tags. Job `check` (latest stable Rust) runs fmt, clippy (default, no and all features), tests, version consistency, coverage ≥ 85%, feature coverage, release guard; job `msrv` builds all targets with the `rust-version` read from `Cargo.toml` (the tests run in `check`); job `build` runs only if both pass and makes release executables for Linux x86_64, macOS arm64, Windows x86_64 and uploads them as artifacts; on a `v*` tag job `package` builds the installers and creates the GitHub release with them and with `releases/<version> - <name>/RELEASE_NOTES.md` as notes. Owned by the Principal Engineer.

## Scripts
| Script | Purpose |
|---|---|
| `scripts/version.sh` | `get`, `next-patch`, `next-major`, `set X.Y.Z` (all locations, all-or-nothing: restored if any step fails; no build needed), `check` |
| `scripts/docs-check.sh` | mechanical docs consistency (Documenter): dead links, sub-documents missing from their index, forbidden AI/agent terms in README and human docs, changelog headers, README latest changes (max 3) and poem (10 verses) |
| `scripts/coverage.sh` | line coverage with one decimal (`cargo llvm-cov`), `--update-readme`, `--fail-under N` (Principal Engineer and CI only); `--badge NN.N` writes the README badge without running tests (Documenter) |
| `scripts/feature-coverage.sh` | every implemented `REQ-NNN` (in `docs/business.md` and `docs/business-*.md`) has a `covers:` test |
| `scripts/collect-release.sh` | `<version> "<name>"`: create `releases/<version> - <name>/` with `docs/` (README, docs, ai-docs unchanged), `sources.zip` (tracked + new files, without `releases/`) and `RELEASE_NOTES.md` (changelog entries) |
| `scripts/release-guard.sh` | fail if a file under `releases/` was modified/renamed |
| `scripts/test-annotations.sh` | CI: turn failing tests into GitHub annotations |
| `scripts/notices.sh`, `scripts/icons.sh`, `scripts/fetch-ffmpeg.sh`, `scripts/package.sh` | project build helpers (third-party notices, icons, pinned ffmpeg, installers), owned by the Principal Engineer |

Local coverage needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.

## Hooks and settings
`.claude/settings.json`: `permissions.deny: ["Edit(releases/**)", "Write(releases/**)"]` — no agent can edit frozen release snapshots.

## Repository
- Remote `origin`: `git@github.com:andrealorenzani/videolore.git`, default branch `main`.

## Where things live
```
AGENTS.md                 rules for all agents (CLAUDE.md -> symlink to it)
.claude/agents/           agent definitions
.claude/commands/         slash commands
.claude/skills/           project skills
ai-docs/                  AI-only notes, one file per agent
docs/                     human docs (business, architecture, code, changelog, roadmap, workflow) + their <area>-<topic>.md sub-documents
releases/<version> - <name>/  release snapshots: docs/, sources.zip, RELEASE_NOTES.md (never edited)
scripts/                  version, coverage, release helpers
README.md, LICENSE        written by the Documenter
.github/workflows/        CI: checks + executables; pages.yml publishes github_page/
github_page/             the project page (one static index.html), published by pages.yml
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
| `docs/business.md`, `docs/architecture.md`, `docs/code.md`, `docs/changelog.md`, `docs/roadmap.md`, `releases/.gitkeep` | create empty template |
| `docs/<area>-*.md` sub-documents, `github_page/`, `.github/workflows/pages.yml` (project page), `src/`, `Cargo.*`, `target/`, `README.md`, `releases/<version> - <name>/`, `.git/` (and the `origin` remote) | never copy |

## Workflow history
- 2026-11-13 — First draft: 4 agents (Product Manager, Architect, Principal Engineer, Documenter), 3 commands (`/add-feature`, `/change-workflow`, `/clone-workflow`), rules in `AGENTS.md`.
- 2026-11-13 — Documenter color set to yellow (gray is not supported); repo linked to GitHub `origin`.
- 2026-11-13 — v0.1.1: version bump on every change; changelog with version + date-time; README without AI/workflow references, versions instead of dates, poem line breaks; one-line commits; copyright headers; GitHub Actions build of executables.
- 2026-11-13 — v0.1.2: version changed everywhere via `scripts/version.sh`; `/release` command with frozen `release/<version>/` snapshots, tag and push; coverage ≥ 85% and feature coverage 100% for the Principal Engineer; README coverage badge.
- 2026-11-14 — v0.1.4: `scripts/version.sh set` all-or-nothing with rollback and no build; owners may split `docs/<area>.md` into `docs/<area>-<topic>.md` sub-documents with the main doc as index.
- 2026-11-14 — v0.1.5: CI on the latest stable Rust (1.100) and a separate MSRV job; GitHub Actions on current major versions; policy "follow the latest Rust and libraries" recorded in `docs/architecture.md`.
- 2026-11-14 — v0.1.6: tests run only by the Principal Engineer after coding, in a fix-and-rerun loop until green; it reports line and feature coverage; no other agent or command runs tests (`/release` uses a Principal Engineer VERIFY run; badge via `scripts/coverage.sh --badge`).
- 2026-11-14 — v0.4.1: `/release` releases the current version into `releases/<version> - <name>/` (docs, `sources.zip`, `RELEASE_NOTES.md`), cleans the changelog, tags and pushes, then moves to the next major and pushes; `release/` renamed `releases/`; `docs/roadmap.md` fed by every agent's `## Future work`; README shows the last 3 versions.
- 2026-11-15 — v1.2.1: token economy: `scripts/docs-check.sh` replaces the Documenter's full read of `docs/` at each development (prose read of changed files, full read on release); quiet test output for the Principal Engineer; rule 12 in `AGENTS.md`; agent merge rejected and notes compaction recorded in the roadmap.
- 2026-11-15 — v1.2.2–1.2.3: project page in `github_page/` (static, no build) and its deployment by `.github/workflows/pages.yml` to GitHub Pages; the page is a project file, not part of the copied workflow.
