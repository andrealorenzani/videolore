---
description: Change the agentic workflow (agents, commands, rules, guardrails, skills, hooks) and update docs/workflow.md
argument-hint: <the change to the workflow>
---

# /change-workflow

Requested change:

<change>
$ARGUMENTS
</change>

If the change text is empty, ask the user for it and stop.

## Rules
- Do **NOT** call any project agent (`product-manager`, `architect`, `principal-engineer`, `documenter`). Do the work yourself.
- You may consult the advisor (a second opinion from a stronger model / the advisor tool, if available) when the change is risky or ambiguous — e.g. it alters ownership, the rule that the documenter always runs, or the order of `/add-feature`.
- `docs/workflow.md` is the source of truth: read it first, and update it last.

## Steps
1. Read `docs/workflow.md`, `AGENTS.md` and the files in `.claude/` touched by the change.
2. If the change conflicts with an existing rule or is ambiguous in a way that changes the result, ask the user (AskUserQuestion) before editing.
3. Apply the change. Where things live:
   - agents: `.claude/agents/<name>.md` (frontmatter: name, description, tools, model, color)
   - commands: `.claude/commands/<name>.md`
   - skills: `.claude/skills/<name>/SKILL.md`
   - rules and guardrails: `AGENTS.md` (`CLAUDE.md` is a symlink, never edit it); enforceable guardrails may also become hooks/permissions in `.claude/settings.json`
   - a new agent also needs its `ai-docs/<file>.md` (private notes) and, if it has one, its `docs/<file>.md`.
4. Keep everything consistent: the agent table in `AGENTS.md`, the command list, and references in other commands/agents.
5. Update `docs/workflow.md`: the relevant section, the **Workflow manifest** (it drives `/clone-workflow`) and the **Workflow history** (date + one line).
6. Bump the version everywhere with `scripts/version.sh set <version>` (the user's version if named, otherwise `scripts/version.sh next-patch`), and add an entry at the top of `docs/changelog.md` with header `## <version> — <date '+%Y-%m-%d %H:%M %z'>` and `Workflow` bullets, since the documenter is not called here. If the change touches what the README shows, update it following the README rules in `AGENTS.md` (no AI/workflow references, versions not dates, leave the coverage badge as it is — you never run tests —, new poem with `\` line breaks). Never touch `release/`.
7. Commit with a single human-readable line (no prefixes, body or trailers). Push only if the user asks.
8. Report to the user what changed, file by file.
