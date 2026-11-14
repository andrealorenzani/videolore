---
description: Add a new feature through the full agent pipeline (Product Manager -> Architect -> PM + Architect + Principal Engineer -> Documenter)
argument-hint: <feature description>
---

# /add-feature

Feature requested by the user:

<feature>
$ARGUMENTS
</feature>

If the feature text is empty, ask the user for it and stop.

## Phase 0 — Target version
If the user named a version for this change (e.g. "for 0.3.0", "version 1.0.0"), that is the **target version**. Otherwise it is `scripts/version.sh next-patch` (0.1.1 → 0.1.2). Pass the target version to every agent.

You are the orchestrator: you do not write project files yourself, the agents do. Run the phases in order.

## Phase 1 — Business plan (`product-manager`, PLAN mode)
1. Call the `product-manager` agent in **PLAN mode** with the feature text.
2. If its `## Ambiguities` contain any `[SEVERE]` item: ask them to the user with AskUserQuestion (use the options the agent proposed; up to 4 questions per call, repeat if needed). MINOR items are not asked.
3. If you asked questions: call `product-manager` again in **PLAN mode** with the feature, its previous plan and the user's answers, asking for the **final plan**.

## Phase 2 — Compact
Write a compact brief (keep every decision, drop repetition):
- `Request:` the original feature text.
- `Business plan:` the final PM plan (summary, requirements with acceptance criteria, out of scope, MINOR defaults).
- `User answers:` question -> answer.

## Phase 3 — Technical plan (`architect`, PLAN mode)
1. Call the `architect` agent in **PLAN mode** with the compact brief.
2. If its `## Questions` contain any `[SEVERE PRODUCT-CHANGING]` item: ask them to the user with AskUserQuestion, then call `architect` again in **PLAN mode** with the brief, its previous plan and the answers, asking for the **final plan**.
3. Append the final technical plan and the answers to the brief. This is the **final plan**.

## Phase 4 — Implementation (parallel)
In a **single message**, call in parallel:
- `product-manager` in **UPDATE mode** with the final plan.
- `architect` in **UPDATE mode** with the final plan.
- `principal-engineer` with the final plan: implement it, update `ai-docs/code.md` and `docs/code.md`, return the changes made.
Wait for all three. The Principal Engineer runs the tests itself after coding and loops (fix code, rerun) until they pass; its report contains the results, the line coverage and the feature coverage. You do not run any test. If the Principal Engineer reports checks it could not make pass, or deviations, tell the user and ask whether to continue to documentation or fix first.

## Phase 5 — Documentation (`documenter`)
Call the `documenter` agent with: the **target version**, the final plan, the Principal Engineer's result (including line coverage and feature coverage), and the lists of changes from PM and Architect. It updates the changelog, README (with its poem) and keeps `docs/` consistent. This step is mandatory (rule 1 of `AGENTS.md`).

## Commit
Do **not** run tests (only the Principal Engineer does). Run only the static checks `scripts/version.sh check && scripts/release-guard.sh`, then `git add -A` and commit with a **single human-readable line** describing the feature for a human (no prefixes, no body, no trailers, no mention of agents/AI). Do not push unless the user asks.

## Finish
Report to the user in a few lines: target version, what was built, files changed, test results with line and feature coverage as reported by the Principal Engineer, decisions taken with defaults, open points.
