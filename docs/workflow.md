# Agentic workflow

How AI agents build this project: who does what, which commands exist, which rules apply, and where every piece lives. This file is the source of truth for `/change-workflow` and `/clone-workflow`.

## Agents

| Agent | File | Color | Model | Focus | Private notes | Human doc |
|---|---|---|---|---|---|---|
| Product Manager | `.claude/agents/product-manager.md` | blue | haiku | business, requirements, functionalities | `ai-docs/business.md` | `docs/business*.md` |
| Architect | `.claude/agents/architect.md` | green | haiku | architecture, NFRs, security, performance, technology choices | `ai-docs/architecture.md` | `docs/architecture*.md` |
| Principal Engineer | `.claude/agents/principal-engineer.md` | red | sonnet (opus when the session model is Opus; always opus in `/release` consolidation and page regeneration) | code, tests, code patterns, implementation security, the project page | `ai-docs/code.md` | `docs/code*.md` (+ code, + `github_page/`) |
| Documenter | `.claude/agents/documenter.md` | yellow | haiku | docs, README, changelog, roadmap, workflow doc, version, release poem | `ai-docs/notes.md` | any `docs/*.md`, `README.md`, `docs/changelog.md`, `docs/roadmap.md`, `docs/workflow*.md` (never the text of `LICENSE`) |
| Analyst | `.claude/agents/analyst.md` | purple | haiku | read-only answers about the project (used by `/analysis`); tools `Read`, `Glob`, `Grep` only, it can write nothing | none | none |

- `ai-docs/*.md` are notes for AI, short: an index of every paragraph of the human docs the owner manages (`- <file> § <heading> — L<start>-<end> (<n> lines)`), so an agent finds a paragraph with `grep` and reads only its lines. AI-only detail goes in `ai-docs/<area>-<topic>.md`, indexed in the main file. Only the owner edits them.
- `docs/*.md` are for humans: brief, with every interesting detail. An owner may split its doc into `docs/<area>-<topic>.md` sub-documents (past ~150 lines or with distinct topics); `docs/<area>.md` remains the index.
- The Documenter has no ownership guardrail on `docs/` and `README.md`; it never edits another agent's `ai-docs/`.
- Every agent's return to the command is short: `[SEVERE]` questions for the user, the list of changed files, and `## Future work` bullets.
- **Models** (`AGENTS.md` rule 13): Product Manager, Architect and Documenter always use `haiku`. The Principal Engineer uses `opus` if the selected session model is Opus, otherwise `sonnet`; the agent file says `sonnet` and `/add-feature` and `/release` pass `model: opus` in the Agent call when needed. In `/release` the consolidation (step 2) and the page regeneration (step 6) are **always** on `opus`. The Analyst always uses `haiku`.
- Agent colors available in Claude Code: red, blue, green, yellow, purple, orange, pink, cyan.

## Commands

| Command | File | What it does |
|---|---|---|
| `/add-feature <text>` | `.claude/commands/add-feature.md` | Feature pipeline (below). |
| `/change-workflow <text>` | `.claude/commands/change-workflow.md` | Changes agents/commands/rules/skills/guardrails. Calls no project agent; may consult an advisor. Updates this file. |
| `/clone-workflow <dir>` | `.claude/skills/clone-workflow/SKILL.md` | Exports the workflow to another directory (see **Cloning the workflow**). |
| `/analysis <question>` | `.claude/commands/analysis.md` | Asks the read-only Analyst (haiku; `Read`, `Glob`, `Grep` only) to answer a question about the project from all the documents and the code. Nothing is written by the agent or by the command. |
| `/release [name]` | `.claude/commands/release.md` | Releases the **current** version (below). |

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

### `/release` pipeline
```
0 preconditions (clean main, not behind, version check, no tag)   1 release name (asked if missing)
2 consolidation, each agent cleans and compacts what it owns, each appends its section to github_page/after release.md (git-ignored, temporary):
    documenter RELEASE-CLEAN  -> cleans docs/, cleans README Known limitations -> hand-off in ai-docs/notes.md
    product-manager RELEASE   -> prioritised docs/roadmap.md + docs/roadmap_tasks.md (versions and batches); cleans its docs
    principal-engineer RELEASE (always opus) -> optimises code, tests (faster, fewer, same feature coverage), libraries (+ estimates);
                                docs/code-index.md + docs/code-architecture.md (mermaid); cleans its docs; suite green
    architect RELEASE         -> verifies and stores the module architecture; cleans its docs
3 gate: principal-engineer VERIFY -> documenter RELEASE (release summary, README badge/latest) -> scripts/collect-release.sh (docs incl. plans, sources.zip, RELEASE_NOTES.md)
4 plans ai-docs/architecture-plan-*.md git rm'd (they live in the snapshot); changelog cleaned
5 commit "Release <V> <name>" -> tag v<V> -> push -> watch CI (artefacts exist)
6 scripts/version.sh set <next major> -> changelog entry -> documenter POEM (new poem) -> principal-engineer PAGE (always opus):
  regenerates github_page/ from github_page/after release.md, pointing to the last release and its artefacts, "Download" link right after the project name; deletes the temporary file
  -> commit "Start version <N>" -> push -> watch CI and Pages
```
- The code and test optimisation happens **before** the gate and `sources.zip`, so the tag, the snapshot and the VERIFY report describe the same code.
- If a step fails the consolidation stays uncommitted in the working tree; after the fix `/release` can be run again (every call is idempotent).

## Rules (full text in `AGENTS.md`)
1. The Documenter runs at the end of **every part** of a development (after the business, architecture and code parts), and only updates docs.
2. An `ai-docs/` file is written only by its owner. The Documenter may write any `docs/` file and `README.md` (no owner guardrail) and reports what it touched.
3. **Every development bumps the version, everywhere**: the user's version if named, otherwise last number +1 (0.1.1 → 0.1.2); after `/release` the version moves to the next major. The target is decided once by the command and applied by the first documenter call; later calls leave it unchanged. Always through `scripts/version.sh set` (Cargo.toml, Cargo.lock, README badge), verified by `scripts/version.sh check`.
4. Changelog in reverse chronological order, one entry per development, `## <version> — <YYYY-MM-DD HH:MM +zone>`; on release its entries become `releases/<version> - <name>/RELEASE_NOTES.md` and it is cleaned.
4b. `releases/<version> - <name>/` snapshots are immutable: deny rules `Edit(releases/**)` and `Write(releases/**)` in `.claude/settings.json` + `scripts/release-guard.sh` in CI. Andrea deletes old ones manually.
4c. `docs/roadmap.md` (Documenter) keeps the known limitations and improvements for future versions, fed by every agent's `## Future work` and by features a plan postpones; on `/release` the Product Manager rewrites it prioritised and writes `docs/roadmap_tasks.md`.
4d. **Project page** (`github_page/`, Principal Engineer): every regeneration points to the last release and its artefacts, with a "Download" link right after the project name (`AGENTS.md` rule 4e).
4e. **Splash files**: fixed names (`splash.gif`, `splash-player.gif`, `summary.txt`, `summary-player.txt`), nothing copied per version (`AGENTS.md` rule 4f).
5. README is for users: no reference to AI/agents/workflow; "Latest changes" shows only the last 3 versions, by version, never by date. The **poem** at the end of the README is replaced by a new one only by `/release`, when the next major version is opened (Documenter, POEM mode); verses 1-9 end with `\` so GitHub shows one verse per line.
6. Workflow changes only via `/change-workflow`, which also keeps the clone skill complete (`check.sh` must pass).
7. **Only the Principal Engineer runs tests**, once, at the end of all the code of the development (not after each step), looping (fix code, rerun) until the suite passes; it reports line coverage and feature coverage, which the Documenter writes into the badge with `scripts/coverage.sh --badge`. No other agent or command runs tests, builds or coverage; `/release` asks the Principal Engineer for a VERIFY run. CI reruns everything independently. Code passes `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`; line coverage ≥ 85% and feature coverage 100% (`REQ-NNN (implemented)` in `docs/business*.md` ↔ `// covers: REQ-NNN` in tests).
8. Every development ends with a commit with one human-readable line (no prefixes or body) plus the `Co-Authored-By:` trailer as the only other line. Push only on request.
9. `LICENSE` is the unmodified GPL-3.0 text; Andrea Lorenzani's copyright is in the README and in the header of each `.rs` file.
10. **Token economy** (`AGENTS.md` rule 12): agents read their index first and grep only the paragraphs they need. The Documenter checks all of `docs/` with `scripts/docs-check.sh` and reads in full only the files a development changed; on `/release` (step 2) it compacts every `docs/` file (not `ai-docs/`, not the changelog) and every other agent cleans and compacts what it owns, removing only useless data.

## Token economy
The cost is not the number of agents but the notes and docs read in full again and again. Measures in place:

- **Indexed notes** (rule 3): each agent starts from a short index and reads paragraphs by line range, so a feature no longer reads whole notes.
- **Mechanical docs checks** (1.2.1): links, sub-document indexes, forbidden terms, changelog headers and the README's latest changes and poem are checked by `scripts/docs-check.sh`, which prints only the problems.
- **Short hand-offs**: agents write their work into files and return only questions, file lists and future-work bullets.
- **Compacted docs** (1.2.5, and on every release): repeated sentences, stale process notes and duplicates removed; every fact and every requirement kept.
- **Not merged**: the Product Manager and the Architect stay separate (one owner per note, and the cold start is cheap next to the notes); the Documenter stays separate because it is the cheap model.

Trade-off: a stale fact in an untouched page can survive until the next release.

## Skills
Project skills go in `.claude/skills/<name>/SKILL.md`.

| Skill | Purpose |
|---|---|
| `clone-workflow` | `/clone-workflow <dir>`: copies the workflow into another project |

## CI
`.github/workflows/build.yml`: on push to `main`, PRs and `v*` tags. Job `check` (latest stable Rust) runs fmt, clippy (default, no and all features), tests, version consistency, coverage ≥ 85%, feature coverage, release guard; job `msrv` builds all targets with the `rust-version` read from `Cargo.toml` (the tests run in `check`); job `build` runs only if both pass and makes release executables for Linux x86_64, macOS arm64, Windows x86_64 and uploads them as artifacts; on a `v*` tag job `package` builds the installers and creates the GitHub release with them and with `releases/<version> - <name>/RELEASE_NOTES.md` as notes. Owned by the Principal Engineer.

## Scripts
| Script | Purpose |
|---|---|
| `scripts/version.sh` | `get`, `next-patch`, `next-major`, `set X.Y.Z` (all locations, all-or-nothing: restored if any step fails; no build needed), `check` |
| `scripts/doc-replace.sh` | replace a requirement (`req`), an ADR (`adr`), any id line (`line`) or a literal text such as a library or badge version (`text [--all]`) without reading the old text; prints file and line range and `+N lines`; refuses `releases/` |
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
README.md                written by the Documenter; LICENSE is the unmodified GPL-3.0 text (never edited)
.github/workflows/        CI: checks + executables; pages.yml publishes github_page/
github_page/             the project page (one static index.html), published by pages.yml; github_page/after release.md is temporary and git-ignored (written during /release)
assets/splash/           splash.gif, splash-player.gif, summary.txt, summary-player.txt (fixed names)
```

## Cloning the workflow
Everything is in `.claude/skills/clone-workflow/`, which is copied too, so a clone can be cloned again.

| File | Content |
|---|---|
| `manifest.txt` | the only manifest: for every workflow file `copy`, `template` or `skip`, plus empty files, the `CLAUDE.md` symlink and `.gitignore` lines |
| `templates/` | generic versions of the project-specific files, with `{{PROJECT_NAME}}`, `{{AUTHOR}}`, `{{REPO_SLUG}}` placeholders |
| `clone.sh` | copies and fills the placeholders it is given; writes nothing if files exist, unless told to overwrite or skip; in the clone drops the `skip` lines, writes its own word list and records the template hashes |
| `check.sh` | fails on an unlisted workflow file, a missing template, a templated file changed since its template was reviewed, or a word of `project-terms.txt` in a copied file or template |

`/change-workflow` updates the manifest and templates and runs `check.sh`. Changes made elsewhere (for example CI or scripts edited during a development) are reported by the next `check.sh`.

Never cloned: sources, `Cargo.*`, `README.md`, release snapshots, sub-documents, `github_page/`, `.git/`.

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
- 2026-11-17 — v1.2.8: commit messages may carry the `Co-Authored-By:` trailer as their only second line (rule 9).
- 2026-11-20 — v1.6.3: `/release` reworked: consolidation by every agent before the gate and the snapshot (Documenter cleans docs and README limitations, Product Manager prioritises the roadmap and writes `docs/roadmap_tasks.md`, Principal Engineer on Opus optimises code, tests and libraries and writes the code index and the mermaid architecture, Architect verifies and stores it), architectural plans archived in the snapshot, a temporary git-ignored `github_page/after release.md`, a new README poem and a regenerated project page (last release, Download link) when the next major opens; project page rules (rule 4e); splash files with fixed names (rule 4f); new read-only `/analysis` command and `analyst` agent (haiku).
- 2026-11-24 — v3.1.0: `scripts/doc-replace.sh` replaces requirements, ADRs and versions without reading them (rule 13b, all owner agents); the Principal Engineer runs the quality suite once, at the end of all the code of a development, not while coding (rule 7).
- 2026-11-26 — cloning moved into the `clone-workflow` skill (manifest, generic templates, placeholders, `clone.sh`, `check.sh`), replacing the command; `/change-workflow` keeps it complete (rule 6b).
