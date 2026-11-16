---
description: Add a new feature through the agent pipeline (product-manager -> architect -> principal-engineer, the documenter after each part)
argument-hint: <feature description>
---

# /add-feature

Feature requested by the user:

<feature>
$ARGUMENTS
</feature>

If the feature text is empty, ask the user for it and stop.

You are the orchestrator: you call the agents in order and relay **only** three things: the user's questions and answers, file paths, and short counts (coverage, implemented REQ ids). You never copy an agent's plan or report into another agent's prompt: agents read each other's work from the notes and docs. Keep your own text short.

## Phase 0 — Target version
If the user named a version (e.g. "for 0.3.0"), that is the **target version**. Otherwise it is `scripts/version.sh next-patch`. Pass it to every documenter call.

## Phase 1 — Business (`product-manager`)
1. Call `product-manager` in **PLAN** mode with the feature text.
2. If its `## Questions` is not `none`: ask them with AskUserQuestion (its options; up to 4 per call), then call `product-manager` in **REFINE** mode with the answers.
3. Call `documenter` with the target version, part `business`, and the PM's short return.

## Phase 2 — Architecture (`architect`)
1. Call `architect` in **PLAN** mode with the feature text and the path of the business plan (`ai-docs/business.md`, `docs/business*.md`).
2. If its `## Questions` is not `none`: ask them with AskUserQuestion, then call `architect` in **REFINE** mode with the answers.
3. Call `documenter` with the target version, part `architecture`, and the architect's short return.

## Phase 3 — Implementation (`principal-engineer`)
1. Call `principal-engineer` (implement mode) with the paths of the plans (`docs/business*.md` requirements, `ai-docs/architecture.md`). It implements, runs the quality suite and loops until green, and updates its notes and docs.
2. If its `## Deviations` is not `none`, or a check did not pass, tell the user and ask whether to continue or fix first.
3. Call `product-manager` in **STATUS** mode with the implemented `REQ-NNN` ids from the Principal Engineer's result.
4. Call `documenter` with the target version, part `code`, the line coverage and feature coverage from the Principal Engineer's result, and the `## Future work` items of all agents so far (business, architecture, code), which go to the roadmap.

## Phase 4 — Close (`documenter`)
Call `documenter` with the target version and part `close`. It checks the README (latest changes), the docs consistency and the version.

## Commit
Do **not** run tests (only the Principal Engineer does). Run only the static checks `scripts/version.sh check && scripts/release-guard.sh`, then `git add -A` and commit with a **single human-readable line** describing the feature for a human (no prefixes, no body, no trailers, no mention of agents/AI). Do not push unless the user asks.

## Finish
Report to the user in a few lines: target version, what was built, files changed, test results with line and feature coverage as reported by the Principal Engineer, decisions taken with defaults, open points, and what went into the roadmap.
