---
description: Release the current version - consolidate docs and code, snapshot docs, sources and release notes in releases/<version> - <name>, reset the changelog, tag and push, then open the next major version and regenerate the project page
argument-hint: "[release name, e.g. MVP]"
---

# /release

Release name (optional): `$ARGUMENTS`

You are the orchestrator. The version released is the **current** version (`scripts/version.sh get`, e.g. 0.4.1); afterwards the project moves to the next major (e.g. 1.0.0). Pushing to GitHub is part of this command: invoking `/release` is the user's authorisation to push `main` (twice) and the tag. Stop at the first failing step and report it; never force-push, never move or delete an existing tag.

Agents cannot start agents, so you make every call, one after the other. Agents hand work over through files, never through you: you pass only paths, the version, the release name and short counts. **Temporary page file**: `github_page/after release.md` (the name has a space: quote it) is created in step 2, collects what the project page needs (each agent appends its own `## <agent>` section), is git-ignored (it is never committed, never in `sources.zip`, never published) and is deleted in step 6.

## 0. Preconditions
1. `git status` is clean and the branch is `main`. Otherwise stop and tell the user.
2. `git fetch origin` and `main` is not behind `origin/main`. If it is, stop.
3. `scripts/version.sh check` passes (static check, no tests). Let `V = scripts/version.sh get`.
4. No `releases/<V> - */` directory and no tag `v<V>` exist (`git tag -l v<V>`, `git ls-remote --tags origin v<V>`). If one does, stop.

## 1. Release name
The snapshot directory is `releases/<V> - <name>` (e.g. `releases/0.4.1 - MVP`). Use `$ARGUMENTS` as the name if given. If it is empty or in doubt, ask the user with AskUserQuestion, proposing 2-3 short names derived from the changelog highlights (e.g. "MVP", "Faster startup"). The name: 1-60 characters, letters, digits, spaces, `.` `_` `+` `-` (checked by `scripts/collect-release.sh`).

## 2. Consolidation (before the gate and the snapshot)
Every agent cleans what it owns: it removes what is no longer needed and compacts what is needed, making related data contiguous, keeping every fact, requirement and exact wording a `grep` may need. The four calls run in this order; each agent reads the files of the previous one by path. Nobody changes the version, nobody touches `releases/`, and **nobody deletes `ai-docs/architecture-plan-*.md`** (step 4 moves them to the snapshot).
1. `documenter` in **RELEASE-CLEAN** mode with `<V>` and `<name>`: cleans and compacts the documents it owns and all of `docs/` (`docs/changelog.md` excluded); analyses and cleans the `Known limitations` of the README and hands every one of them to the Product Manager through its notes (`ai-docs/notes.md`, section `Release hand-off`); creates `github_page/after release.md` with the Documenter section.
2. `product-manager` in **RELEASE** mode: writes a prioritised `docs/roadmap.md` (from everything it knows, the roadmap of the Documenter and the Documenter's `Known limitations`) and `docs/roadmap_tasks.md` (the roadmap split into versions, in the order and batches suggested for implementation); cleans and compacts its own documents; appends its section to `github_page/after release.md`.
3. `principal-engineer` in **RELEASE** mode, **always with `model: opus`** (whatever the session model is): optimisation of the code and of the tests, library updates (it reports what each update and each new library would cost), reorganisation of its documents, the code index (`docs/code-index.md`) and the module architecture with mermaid diagrams (`docs/code-architecture.md`); it runs the quality suite after every code change and loops until green; cleans its own documents; appends its section to `github_page/after release.md`.
4. `architect` in **RELEASE** mode: verifies and stores what the Principal Engineer wrote about the module architecture, cleans and compacts its own documents, appends its section to `github_page/after release.md`.
If an agent returns `[SEVERE]` questions or something fails, stop and tell the user. The working tree keeps the consolidation uncommitted; once the problem is solved (and committed), `/release` can be run again, because every call is idempotent.

## 3. Gate and snapshot
1. **Quality gate**: call the `principal-engineer` in **VERIFY mode** (the model is chosen by the session's selected model: `opus` if it is Opus, otherwise `sonnet` (the default, also when Fable, Haiku or anything else is selected). Pass it in the Agent call's `model` parameter; the agent file defaults to `sonnet`) — it runs the full test suite without changing anything and reports results, line coverage and feature coverage. You never run tests yourself. If anything fails, stop and report (suggest `/add-feature` or a fix).
2. Call the `documenter` with `RELEASE <V> "<name>"`, the line coverage from the VERIFY report and any future work collected (see `/add-feature`). It does **not** change the version, the poem or the roadmap. It adds a short `### Release <V> — <name>` summary at the top of the current `## <V>` entry of `docs/changelog.md` (highlights of everything since the previous release), updates the README (coverage badge, Latest changes = last 3 versions), runs `scripts/docs-check.sh` and checks consistency. It does not reset the changelog and does not touch `releases/`.
3. `scripts/collect-release.sh <V> "<name>"` creates `releases/<V> - <name>/` with:
   - `docs/` — `README.md`, `docs/` and `ai-docs/`, copied unchanged (so the snapshot holds the architectural plans);
   - `sources.zip` — every tracked and new (not ignored) file of the working tree, without `releases/`;
   - `RELEASE_NOTES.md` — the release notes, taken from `docs/changelog.md` (all entries since the previous release).

From now on nothing under `releases/` is ever edited (rule in `AGENTS.md`, denied in `.claude/settings.json`, checked in CI by `scripts/release-guard.sh`). Only the user deletes old snapshots, manually.

## 4. Clean the changelog and archive the plans
1. Check that `releases/<V> - <name>/docs/ai-docs/` holds every `architecture-plan-*.md` of `ai-docs/`; if one is missing, stop. Then `git rm` the `ai-docs/architecture-plan-*.md` files from the working tree: the plans now live only in the snapshot (the Architect's notes already say so).
2. Rewrite `docs/changelog.md` as:
```
# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

No changes since <V> (release "<name>", notes in `releases/<V> - <name>/RELEASE_NOTES.md`).
```

## 5. Commit, tag and push the release
1. `scripts/version.sh check && scripts/release-guard.sh`, then `git add -A` (the temporary page file is git-ignored) and `git commit -m "Release <V> <name>"` (one human-readable line, no prefixes or body, plus the `Co-Authored-By:` trailer as the only other line).
2. Tag: `git tag -a v<V> -m "Release <V> - <name>"`.
3. `git push origin main`, then `git push origin v<V>`.
4. The tag push triggers `.github/workflows/build.yml`: executables and the **GitHub release `v<V>`** with `releases/<V> - <name>/RELEASE_NOTES.md` as notes. Watch the run until it ends (`gh run watch` if available, otherwise poll the public API `https://api.github.com/repos/<owner>/<repo>/actions/runs?head_sha=<sha>` with a background wait loop) and report the result. If it fails, read the annotations, report them and stop before step 6 only if the user asks; the release commit and tag stay as they are. The project page of step 6 points to the artefacts of this release, so they must exist.

## 6. Open the next major version
1. `N = scripts/version.sh next-major` (0.4.1 → 1.0.0, 1.3.0 → 2.0.0); `scripts/version.sh set <N>` (Cargo.toml, Cargo.lock, README badge).
2. Add at the top of `docs/changelog.md` (below the introduction, replacing the "No changes since" line):
   ```
   ## <N> — <date '+%Y-%m-%d %H:%M %z'>
   - Version <N> opened after release <V> "<name>". New README poem. Project page regenerated for release <V>.
   ```
3. **New poem**: call the `documenter` in **POEM** mode: it writes a new 10-verse poem at the end of `README.md` (the rule of `AGENTS.md`: {{AUTHOR}}'s work on this project, verses 1-9 end with `\`, verse 10 does not), replacing the previous one, and runs `scripts/docs-check.sh`. Nothing else in the README changes except the version badge already set.
4. **Project page**: call the `principal-engineer` in **PAGE** mode, **always with `model: opus`**, with `<V>`, `<name>` and the repository path. It reads `github_page/after release.md`, regenerates `github_page/` following the page rules of `AGENTS.md` (it points to the last release `v<V>` and to its artefacts, with a download link right after the project name), and then **deletes `github_page/after release.md`**. Check afterwards that the file is gone.
5. `scripts/version.sh check && scripts/release-guard.sh`, `git add -A`, `git commit -m "Start version <N>"`, `git push origin main`, and watch that CI run too (the page is published by `.github/workflows/pages.yml` when `github_page/` changes).

## 7. Report
Released version and name, snapshot directory (with the size of `sources.zip`), release commit, tag, GitHub release / Actions result, new version and its commit, the new poem and the page (regenerated, points to `v<V>`), what the consolidation changed (sizes of the documents before and after, optimisation, libraries), anything not verified.
