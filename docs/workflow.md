# Agentic workflow

How AI agents build this project: who does what, which commands exist, which rules apply, and where every piece lives. This file is the source of truth for `/change-workflow` and `/clone-workflow`.

## Agents

| Agent | File | Color | Model | Focus | Private notes | Human doc |
|---|---|---|---|---|---|---|
| Product Manager | `.claude/agents/product-manager.md` | blue | haiku | business, requirements, functionalities | `ai-docs/business.md` | `docs/business*.md` |
| Architect | `.claude/agents/architect.md` | green | haiku | architecture, NFRs, security, performance, technology choices | `ai-docs/architecture.md` | `docs/architecture*.md` |
| Principal Engineer | `.claude/agents/principal-engineer.md` | red | sonnet (opus when the session model is Opus) | code, tests, code patterns, implementation security | `ai-docs/code.md` | `docs/code*.md` (+ code) |
| Documenter | `.claude/agents/documenter.md` | yellow | haiku | docs, README, changelog, roadmap, workflow doc, version | `ai-docs/notes.md` | any `docs/*.md`, `README.md`, `docs/changelog.md`, `docs/roadmap.md`, `docs/workflow*.md` (never the text of `LICENSE`) |

- `ai-docs/*.md` are notes for AI, short: an index of every paragraph of the human docs the owner manages (`- <file> § <heading> — L<start>-<end> (<n> lines)`), so an agent finds a paragraph with `grep` and reads only its lines. AI-only detail goes in `ai-docs/<area>-<topic>.md`, indexed in the main file. Only the owner edits them.
- `docs/*.md` are for humans: brief, with every interesting detail. An owner may split its doc into `docs/<area>-<topic>.md` sub-documents (past ~150 lines or with distinct topics); `docs/<area>.md` remains the index.
- The Documenter has no ownership guardrail on `docs/` and `README.md`; it never edits another agent's `ai-docs/`.
- Every agent's return to the command is short: `[SEVERE]` questions for the user, the list of changed files, and `## Future work` bullets.
- **Models** (`AGENTS.md` rule 13): Product Manager, Architect and Documenter always use `haiku`. The Principal Engineer uses `opus` if the selected session model is Opus, otherwise `sonnet`; the agent file says `sonnet` and `/add-feature` and `/release` pass `model: opus` in the Agent call when needed.
- Agent colors available in Claude Code: red, blue, green, yellow, purple, orange, pink, cyan.

## Commands

| Command | File | What it does |
|---|---|---|
| `/add-feature <text>` | `.claude/commands/add-feature.md` | Feature pipeline (below). |
| `/change-workflow <text>` | `.claude/commands/change-workflow.md` | Changes agents/commands/rules/skills/guardrails. Calls no project agent; may consult an advisor. Updates this file. |
| `/clone-workflow <dir>` | `.claude/commands/clone-workflow.md` | Exports the workflow to another directory using the manifest below. |
| `/release [name]` | `.claude/commands/release.md` | Releases the **current** version: gates (Principal Engineer VERIFY) → release name (asked if missing) → Documenter (release summary, roadmap, README, docs compaction) → `scripts/collect-release.sh` creates `releases/<version> - <name>/` (all docs unchanged, `sources.zip`, `RELEASE_NOTES.md` from the changelog) → changelog cleaned → commit `Release <version> <name>` → tag `v<version>` → push → watch CI → `scripts/version.sh set <next major>` → commit `Start version <major>` → push. The tag makes CI create the GitHub release with executables, installers and the release notes. |

### `/add-feature` pipeline
```
target version (named by the user, else next-patch)
  -> product-manager PLAN   writes ai-docs/business.md + docs/business*.md (planned REQs); returns [SEVERE] questions only
       [SEVERE?] -> command asks the user -> product-manager REFINE (answers)
  -> documenter (part business)
  -> architect PLAN         reads the business notes by path; writes ai-docs/architecture.md + docs/architecture*.md; returns [SEVERE] questions only
       [SEVERE?] -> command asks the user -> architect REFINE
  -> documenter (part architecture)
  -> principal-engineer     implements, runs the suite, loops until green; writes ai-docs/code.md + docs/code*.md; returns results, coverage, implemented REQs
  -> product-manager STATUS  marks the implemented REQs
  -> documenter (part code: changelog, roadmap from all Future work, coverage badge)
  -> documenter (part close: README latest changes, consistency, version check)
  -> commit, report to the user
```
- Agents hand work over through files, not through the command: the command passes paths, the user's answers and short counts only.
- Claude Code subagents cannot start other subagents, so the command makes each call. The documenter is called right after each part, as the agent's last step, and only updates docs.
- MINOR ambiguities and TECHNICAL questions are decided by the agents and recorded as assumptions/decisions in their notes.

## Rules (full text in `AGENTS.md`)
1. The Documenter runs at the end of **every part** of a development (after the business, architecture and code parts), and only updates docs.
2. An `ai-docs/` file is written only by its owner. The Documenter may write any `docs/` file and `README.md` (no owner guardrail) and reports what it touched.
3. **Every development bumps the version, everywhere**: the user's version if named, otherwise last number +1 (0.1.1 → 0.1.2); after `/release` the version moves to the next major. The target is decided once by the command and applied by the first documenter call; later calls leave it unchanged. Always through `scripts/version.sh set` (Cargo.toml, Cargo.lock, README badge), verified by `scripts/version.sh check`.
4. Changelog in reverse chronological order, one entry per development, `## <version> — <YYYY-MM-DD HH:MM +zone>`; on release its entries become `releases/<version> - <name>/RELEASE_NOTES.md` and it is cleaned.
4b. `releases/<version> - <name>/` snapshots are immutable: deny rules `Edit(releases/**)` and `Write(releases/**)` in `.claude/settings.json` + `scripts/release-guard.sh` in CI. Andrea deletes old ones manually.
4c. `docs/roadmap.md` (Documenter) keeps the known limitations and improvements for future versions, fed by every agent's `## Future work` and by features a plan postpones.
5. README is for users: no reference to AI/agents/workflow; "Latest changes" shows only the last 3 versions, by version, never by date. The **poem** at the end of the README is rewritten only when a new major version is released (`X.0.0`); verses 1-9 end with `\` so GitHub shows one verse per line.
6. Workflow changes only via `/change-workflow`.
7. **Only the Principal Engineer runs tests**, after coding, looping (fix code, rerun) until the suite passes; it reports line coverage and feature coverage, which the Documenter writes into the badge with `scripts/coverage.sh --badge`. No other agent or command runs tests, builds or coverage; `/release` asks the Principal Engineer for a VERIFY run. CI reruns everything independently. Code passes `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`; line coverage ≥ 85% and feature coverage 100% (`REQ-NNN (implemented)` in `docs/business*.md` ↔ `// covers: REQ-NNN` in tests).
8. Every development ends with a commit with a single human-readable line (no prefixes, body or trailers). Push only on request.
9. `LICENSE` is the unmodified GPL-3.0 text; Andrea Lorenzani's copyright is in the README and in the header of each `.rs` file.
10. **Token economy** (`AGENTS.md` rule 12): agents read their index first and grep only the paragraphs they need. The Documenter checks all of `docs/` with `scripts/docs-check.sh` and reads in full only the files a development changed; on `/release` it compacts every `docs/` file (not `ai-docs/`, not the changelog), removing only useless data.

## Token economy
The cost is not the number of agents but the notes and docs read in full again and again. Measures in place:

- **Indexed notes** (rule 3): each agent starts from a short index and reads paragraphs by line range, so a feature no longer reads whole notes.
- **Mechanical docs checks** (1.2.1): links, sub-document indexes, forbidden terms, changelog headers and the README's latest changes and poem are checked by `scripts/docs-check.sh`, which prints only the problems.
- **Short hand-offs**: agents write their work into files and return only questions, file lists and future-work bullets.
- **Compacted docs** (1.2.5, and on every release): repeated sentences, stale process notes and duplicates removed; every fact and every requirement kept.
- **Not merged**: the Product Manager and the Architect stay separate (one owner per note, and the cold start is cheap next to the notes); the Documenter stays separate because it is the cheap model.

Trade-off: a stale fact in an untouched page can survive until the next release.

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
| `scripts/feature-coverage.sh` | every implemented `REQ-NNN` (in `docs/business*.md`) has a `covers:` test |
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
ai-docs/                  AI-only notes: index + optional <area>-<topic>.md sub-documents, one owner per file
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
| `docs/<area>-*.md` sub-documents, `ai-docs/<area>-*.md`, `github_page/`, `.github/workflows/pages.yml` (project page), `src/`, `Cargo.*`, `target/`, `README.md`, `releases/<version> - <name>/`, `.git/` (and the `origin` remote) | never copy |

## Workflow history
- 2026-11-13 — First draft: 4 agents (Product Manager, Architect, Principal Engineer, Documenter), 3 commands (`/add-feature`, `/change-workflow`, `/clone-workflow`), rules in `AGENTS.md`.
- 2026-11-13 — Documenter color set to yellow (gray is not supported); repo linked to GitHub `origin`.
- 2026-11-13 — v0.1.1: version bump on every change; changelog with version + date-time; README without AI/workflow references, versions instead of dates, poem line breaks; one-line commits; copyright headers; GitHub Actions build of executables.
- 2026-11-13 — v0.1.2: version changed everywhere via `scripts/version.sh`; `/release` command with frozen snapshots, tag and push; coverage ≥ 85% and feature coverage 100% for the Principal Engineer; README coverage badge.
- 2026-11-14 — v0.1.4: `scripts/version.sh set` all-or-nothing with rollback and no build; owners may split `docs/<area>.md` into `docs/<area>-<topic>.md` sub-documents with the main doc as index.
- 2026-11-14 — v0.1.5: CI on the latest stable Rust (1.100) and a separate MSRV job; GitHub Actions on current major versions; policy "follow the latest Rust and libraries" recorded in `docs/architecture.md`.
- 2026-11-14 — v0.1.6: tests run only by the Principal Engineer after coding, in a fix-and-rerun loop until green; it reports line and feature coverage; no other agent or command runs tests.
- 2026-11-14 — v0.4.1: `/release` releases the current version into `releases/<version> - <name>/`, cleans the changelog, tags and pushes, then moves to the next major and pushes; `docs/roadmap.md` fed by every agent's `## Future work`; README shows the last 3 versions.
- 2026-11-15 — v1.2.1: token economy: `scripts/docs-check.sh` replaces the Documenter's full read of `docs/` at each development; quiet test output for the Principal Engineer; rule 12 in `AGENTS.md`; agent merge rejected.
- 2026-11-15 — v1.2.2–1.2.3: project page in `github_page/` (static, no build) and its deployment by `.github/workflows/pages.yml` to GitHub Pages; the page is a project file, not part of the copied workflow.
- 2026-11-15 — v1.2.4: the Pages workflow no longer enables Pages itself (the default token cannot); enabling it is a one-time manual step.
- 2026-11-16 — v1.2.5: notes become indexes (`ai-docs/` index by paragraph and line range, optional `ai-docs/<area>-<topic>.md` sub-documents); `/add-feature` hands work over through files, the command relays only questions, paths and counts; the Documenter runs after each part, with no ownership guardrail on `docs/`; the README poem changes only on a major release; docs compacted on release (and once now).
- 2026-11-17 — v1.2.7: models per agent: Haiku for Product Manager, Architect and Documenter; Principal Engineer on Sonnet, or Opus when the session model is Opus.
