---
description: Release the current version - snapshot docs, sources and release notes in releases/<version> - <name>, reset the changelog, tag and push, then open the next major version
argument-hint: "[release name, e.g. MVP]"
---

# /release

Release name (optional): `$ARGUMENTS`

You are the orchestrator. The version released is the **current** version (`scripts/version.sh get`, e.g. 0.4.1); afterwards the project moves to the next major (e.g. 1.0.0). Pushing to GitHub is part of this command: invoking `/release` is the user's authorisation to push `main` (twice) and the tag. Stop at the first failing step and report it; never force-push, never move or delete an existing tag.

## 0. Preconditions
1. `git status` is clean and the branch is `main`. Otherwise stop and tell the user.
2. `git fetch origin` and `main` is not behind `origin/main`. If it is, stop.
3. `scripts/version.sh check` passes (static check, no tests). Let `V = scripts/version.sh get`.
4. No `releases/<V> - */` directory and no tag `v<V>` exist (`git tag -l v<V>`, `git ls-remote --tags origin v<V>`). If one does, stop.
5. Quality gates: call the `principal-engineer` in **VERIFY mode** — it runs the full test suite without changing anything and reports results, line coverage and feature coverage. You never run tests yourself. If anything fails, stop and report (suggest `/add-feature` or a fix).

## 1. Release name
The snapshot directory is `releases/<V> - <name>` (e.g. `releases/0.4.1 - MVP`). Use `$ARGUMENTS` as the name if given. If it is empty or in doubt, ask the user with AskUserQuestion, proposing 2-3 short names derived from the changelog highlights (e.g. "MVP", "Hardware export"). The name: 1-60 characters, letters, digits, spaces, `.` `_` `+` `-` (checked by `scripts/collect-release.sh`).

## 2. Final docs for the release (`documenter`)
Call the `documenter` agent with `RELEASE <V> "<name>"`, the line coverage from the VERIFY report and any future work collected (see `/add-feature`). It does **not** change the version. It adds a short `### Release <V> — <name>` summary at the top of the current `## <V>` entry of `docs/changelog.md` (highlights of everything since the previous release), refreshes `docs/roadmap.md`, updates the README (coverage badge, Latest changes = last 3 versions; the poem is rewritten only if `<V>` is a major version `X.0.0`), compacts every `docs/` file (not `ai-docs/`, not the changelog) removing only useless data, and checks consistency. It does not reset the changelog and does not touch `releases/`.

## 3. Snapshot
`scripts/collect-release.sh <V> "<name>"` creates `releases/<V> - <name>/` with:
- `docs/` — `README.md`, `docs/` and `ai-docs/`, copied unchanged;
- `sources.zip` — every tracked and new (not ignored) file of the working tree, without `releases/`;
- `RELEASE_NOTES.md` — the release notes, taken from `docs/changelog.md` (all entries since the previous release).

From now on nothing under `releases/` is ever edited (rule in `AGENTS.md`, denied in `.claude/settings.json`, checked in CI by `scripts/release-guard.sh`). Only the user deletes old snapshots, manually.

## 4. Clean the changelog
Rewrite `docs/changelog.md` as:
```
# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release the entries become `releases/<version> - <name>/RELEASE_NOTES.md` and this file is cleaned.

No changes since <V> (release "<name>", notes in `releases/<V> - <name>/RELEASE_NOTES.md`).
```

## 5. Commit, tag and push the release
1. `scripts/version.sh check && scripts/release-guard.sh`, then `git add -A` and `git commit -m "Release <V> <name>"` (single line, no prefixes, body or trailers).
2. Tag: `git tag -a v<V> -m "Release <V> - <name>"`.
3. `git push origin main`, then `git push origin v<V>`.
4. The tag push triggers `.github/workflows/build.yml`: executables, installers and the **GitHub release `v<V>`** with `releases/<V> - <name>/RELEASE_NOTES.md` as notes. Watch the run until it ends (`gh run watch` if available, otherwise poll the public API `https://api.github.com/repos/<owner>/<repo>/actions/runs?head_sha=<sha>` with a background wait loop) and report the result. If it fails, read the annotations, report them and stop before step 6 only if the user asks; the release commit and tag stay as they are.

## 6. Open the next major version
1. `N = scripts/version.sh next-major` (0.4.1 → 1.0.0, 1.3.0 → 2.0.0); `scripts/version.sh set <N>` (Cargo.toml, Cargo.lock, README badge).
2. Add at the top of `docs/changelog.md` (below the introduction, replacing the "No changes since" line):
   ```
   ## <N> — <date '+%Y-%m-%d %H:%M %z'>
   - Version <N> opened after release <V> "<name>". No user-facing changes yet.
   ```
   The README "Latest changes" does not list it (no user-facing change); only the version badge changes, so the poem is not touched.
3. `scripts/version.sh check && scripts/release-guard.sh`, `git add -A`, `git commit -m "Start version <N>"`, `git push origin main`, and watch that CI run too.

## 7. Report
Released version and name, snapshot directory (with the size of `sources.zip`), release commit, tag, GitHub release / Actions result, new version and its commit, anything not verified.
