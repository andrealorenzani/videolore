---
description: Ship a major release - bump the major version everywhere, freeze all docs in release/<version>, reset the changelog, tag main and push to GitHub
argument-hint: "[version, default: next major e.g. 1.0.0]"
---

# /release

Requested version (optional): `$ARGUMENTS`

You are the orchestrator. Pushing to GitHub is part of this command: invoking `/release` is the user's authorisation to push `main` and the tag. Stop at the first failing step and report it; never force-push, never move or delete an existing tag.

## 0. Preconditions
1. `git status` is clean and the branch is `main`. Otherwise stop and tell the user.
2. `git fetch origin` and `main` is not behind `origin/main`. If it is, stop.
3. Quality gates must pass: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `scripts/coverage.sh --fail-under 85`, `scripts/feature-coverage.sh`, `scripts/version.sh check`. If one fails, stop and report (suggest `/add-feature` or a fix).

## 1. Bump the major version everywhere (FIRST change)
1. Version = `$ARGUMENTS` if given (must be `X.Y.Z` and greater than the current one), else `scripts/version.sh next-major` (0.4.2 → 1.0.0, 1.3.0 → 2.0.0).
2. `release/<version>/` must not exist; if it does, stop.
3. `scripts/version.sh set <version>` — updates `Cargo.toml`, `Cargo.lock` and the README version badge, then checks them.

## 2. Final docs for the release (`documenter`)
Call the `documenter` agent with: `RELEASE <version>`. It adds the `## <version> — <date-time>` entry at the top of `docs/changelog.md` (release summary of everything since the last release), updates the README (latest changes, coverage badge via `scripts/coverage.sh --update-readme`, new poem) and checks consistency. It must **not** reset the changelog nor touch `release/`.

## 3. Freeze all docs
`scripts/collect-release.sh <version>` — copies `README.md`, `docs/` and `ai-docs/` into `release/<version>/`. The `release/<version>/docs/changelog.md` copy is the **release log**. From now on nothing under `release/` is ever edited (rule in `AGENTS.md`, denied in `.claude/settings.json`, checked in CI by `scripts/release-guard.sh`). Only the user deletes old snapshots, manually.

## 4. New empty changelog
Rewrite `docs/changelog.md` as the empty template:
```
# Changelog

Changes since the last release, newest first. Each entry: version and date-time. On release this file is frozen in `release/<version>/docs/changelog.md` and reset.

No changes since <version>.
```

## 5. Commit, tag, push
1. `scripts/version.sh check`, then `git add -A` and `git commit -m "Release <version>"`.
2. Tag `main`: `git tag -a v<version> -m "Release <version>"`.
3. `git push origin main` then `git push origin v<version>`.

## 6. GitHub release
The tag push triggers `.github/workflows/build.yml`: it builds the executables (Linux, macOS, Windows) and **creates the GitHub release `v<version>`** with them attached and `release/<version>/docs/changelog.md` as release notes. GitHub adds the source code archives (zip/tar.gz) of the tag automatically.
If `gh` is installed and authenticated, check it with `gh run watch` / `gh release view v<version>`; otherwise give the user the Actions link (`https://github.com/<owner>/<repo>/actions`) and say: if the workflow fails, create the release manually from tag `v<version>` on GitHub.

## 7. Report
Version, snapshot directory, commit, tag, push result, link to the release/Actions page, anything not verified.
