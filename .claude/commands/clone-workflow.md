---
description: Export the current agentic workflow (agents, commands, skills, rules, templates) into another directory
argument-hint: <target directory>
---

# /clone-workflow

Target directory: `$ARGUMENTS`

If the target is empty, ask the user for it and stop.

## Steps
1. Read `docs/workflow.md`, section **Workflow manifest**: it lists what is copied verbatim, what is created empty from a template, and what is never copied. Follow it; if it disagrees with the files on disk, tell the user and use what is on disk.
2. Resolve the target to an absolute path. If it does not exist, create it. If it already contains any of the files to write, list them and ask the user (AskUserQuestion): overwrite, skip existing, or abort.
3. **Copy verbatim** (keep the directory structure): `.claude/agents/`, `.claude/commands/`, `.claude/skills/`, `.claude/settings.json` (if present), `AGENTS.md`, `docs/workflow.md`, `LICENSE`.
4. Recreate `CLAUDE.md` as a **symlink** to `AGENTS.md` (`ln -s AGENTS.md CLAUDE.md`), never as a copy.
5. **Create empty templates** (project-specific content must not leak): `ai-docs/business.md`, `ai-docs/architecture.md`, `ai-docs/code.md`, `ai-docs/notes.md`, `docs/business.md`, `docs/architecture.md`, `docs/code.md`, `docs/changelog.md`, `docs/releases/.gitkeep` — each with just its title and a "Not written yet" line.
6. In the copied `AGENTS.md`, replace the **Project** section with a placeholder (`<project name> — to be defined`), and in the copied `docs/workflow.md` add a history line: `<date> — cloned from <source path>`.
7. Do not copy `src/`, `Cargo.*`, `target/`, `README.md`, `.git/`. The README is written by the `documenter` on the first development of the new project.
8. Report: files copied, templates created, symlink created, anything skipped.
