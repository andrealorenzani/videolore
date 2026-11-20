---
description: Ask a question about the project; a read-only Haiku analyst reads all the documents and the code and answers (it can write nothing)
argument-hint: "<question about the project>"
---

# /analysis

Question: `$ARGUMENTS`

If the question is empty, ask the user for it and stop.

Call the `analyst` agent once with `model: haiku` (the agent file says so too) and the question, exactly as the user wrote it, plus the instruction to answer from the project's documents and code. The analyst has only the read tools `Read`, `Glob` and `Grep`: it **cannot write, edit, delete or run anything**, and so you must not either. This command changes **nothing** in the repository, creates no file, no commit, no note and no memory, even if the answer suggests a change.

Show the analyst's answer to the user as it is (keep its `path:line` references). If the user then wants something changed, tell them which command does it (`/add-feature`, `/change-workflow`, `/release`); do not do it here. Do not call any other agent.
