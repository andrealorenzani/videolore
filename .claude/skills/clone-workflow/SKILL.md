---
name: clone-workflow
description: Export the agentic workflow of this project (agents, commands, skills, rules, scripts, CI, generic templates) into another directory with a script. Use for /clone-workflow, or when asked to copy or clone the workflow into a new project.
argument-hint: <target directory>
---

# /clone-workflow

Target directory: `$ARGUMENTS`

If the target is empty, ask the user for it and stop.

Everything is in this directory (`.claude/skills/clone-workflow/`), and the whole directory is copied, so a cloned project can clone again:

| File | Content |
|---|---|
| `clone.sh` | does the copy |
| `check.sh` | verifies that the manifest and the templates cover the current workflow |
| `manifest.txt` | what is copied, templated, created empty or skipped (the only manifest) |
| `templates/` | generic versions of the files that are specific to this project (`dot-claude/` stands for `.claude/`) |
| `project-terms.txt` | words of this project that must never appear in a copied file or template |

## Placeholders
`{{PROJECT_NAME}}`, `{{AUTHOR}}`, `{{REPO_SLUG}}` (`owner/repository`). `clone.sh` fills the ones it is given; the others stay in the new files and are listed at the end. The files under `templates/` always keep them.

## Steps
1. Run `.claude/skills/clone-workflow/check.sh`. If it fails, show the problems and ask the user whether to clone anyway (fixing them is a workflow change: `/change-workflow`).
2. Take the project name, author and repository from the request. Ask once (AskUserQuestion) for the missing ones; "decide later" is a valid answer. Never fill them with this project's values on your own.
3. Run `.claude/skills/clone-workflow/clone.sh <target> [--name N] [--author A] [--repo owner/repository]`. Exit code 3 means files already exist: show the list, ask the user (overwrite, skip existing, abort) and rerun with `--on-conflict overwrite|skip`.
4. Do not edit the cloned files by hand: if something is wrong or specific, fix the template or the manifest here.
5. Report what the script printed: files copied and from templates, empty files, skipped, unfilled placeholders, and what the new project still needs.
