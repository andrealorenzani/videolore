# Scripts that read the documents

Read-only helpers in `scripts/`. They print parts of the human documents, or facts read from the code, so that a reader (human or agent) loads only what is needed. None of them writes to `docs/`, `ai-docs/` or `README.md`. Each script prints its usage with a wrong call.

| Script | Prints | Example |
|---|---|---|
| `scripts/req.sh REQ-NNN...` | the status, the file and line of each requirement, its full paragraph and the test files that cover it (`// covers:`) | `scripts/req.sh REQ-177` |
| `scripts/reqs-status.sh [status] [file]` | one line per requirement: `REQ-NNN status file:line`; `--counts` gives the total per status | `scripts/reqs-status.sh planned` |
| `scripts/doc-section.sh <file> <heading>` | one section, from its heading to the next heading of the same or a higher level; the line range goes to stderr | `scripts/doc-section.sh docs/code-hardware.md "Hardware check"` |
| `scripts/doc-index.sh [file...]` | the index of the headings of the human documents, with line ranges and lengths (default: every `docs/*.md` except the changelog) | `scripts/doc-index.sh docs/business-text.md` |
| `scripts/code-map.sh [--tests] [word]` | the module map read from the sources: one line per `.rs` file of the editor and the player with its line count and the first line of its `//!` doc; a word filters by path or doc | `scripts/code-map.sh overlap` |
| `scripts/req-code.sh REQ-NNN...` | where a requirement is in the code: the source files that name it (not tests) and every covering test as `file:line test_name` | `scripts/req-code.sh REQ-325` |
| `scripts/symbol.sh [-u] Name` | where a Rust item (fn, struct, enum, trait, type, const, static, mod, macro) is defined; `-u` adds the number of uses per file | `scripts/symbol.sh -u EffectKind` |
| `scripts/module-deps.sh [module \| -r module \| --mermaid]` | the top-level modules each module uses (`crate::` paths, test modules left out), the users of a module, or the graph as mermaid lines; `tests/module_cycles.rs` is the exact check | `scripts/module-deps.sh -r overlap` |

Related checks that run without reading the text: `scripts/feature-coverage.sh` (every implemented requirement has a test), `scripts/docs-check.sh` (links, sub-document indexes, forbidden terms, changelog headers, README latest changes and poem).

## Requirements
Requirements are the lines that start with `- REQ-NNN (status):` in `docs/business.md` and `docs/business-*.md`. The status is what is in brackets: `implemented`, `planned` (possibly followed by a note), or a short note. Acceptance criteria are part of the same paragraph.

## Reading a long document
1. `scripts/doc-index.sh <file>` to find the section.
2. `scripts/doc-section.sh <file> "<heading>"` to print it.
