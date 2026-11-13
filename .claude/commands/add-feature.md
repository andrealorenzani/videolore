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
Wait for all three. If the Principal Engineer reports failing checks or deviations, tell the user and ask whether to continue to documentation or fix first.

## Phase 5 — Documentation (`documenter`)
Call the `documenter` agent with: the final plan, the Principal Engineer's result, and the lists of changes from PM and Architect. It updates the changelog, README (with its poem) and keeps `docs/` consistent. This step is mandatory (rule 1 of `AGENTS.md`).

## Finish
Report to the user in a few lines: what was built, files changed, test result, decisions taken with defaults, open points.
