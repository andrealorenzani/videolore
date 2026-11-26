# Technical plan 3.1.3 (PLAN 2026-11-26; owner: architect; archived at release)

Scope: REQ-359..366 (docs/business-community.md): GitHub community files, community profile, no internal wording in public files. Target 3.1.3 (Cargo.toml already 3.1.3). Decisions ADR-271..275 below. No SEVERE question: the conduct address, the reporting channels and the Discussions setting were decided by Andrea (REFINE 2026-11-26).

## Verified facts (checked 2026-11-26)
- LICENSE is byte-identical to the FSF gpl-3.0.txt: 674 lines, sha256 3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986.
- Contributor Covenant 2.1 (upstream code_of_conduct.md): 85 lines, sha256 977d781349351fd7c1f076e4c7dc7de2a05b40e12c773542c3815dd4ce7f37ba on 2026-11-26 (the hash may change upstream; check the version line instead). One placeholder "[INSERT CONTACT METHOD]" (enforcement paragraph). Attribution section: "adapted from the Contributor Covenant, version 2.1, available at https://www.contributor-covenant.org/version/2/1/code_of_conduct.html".
- README and LICENSE contain no forbidden term (list in ADR-274). Bare "workflow" and "command" are NOT forbidden: README has the GitHub URL `actions/workflows/build.yml` and "command line".
- Crates: sha2 0.11.1 is a normal dependency (usable in tests). yaml-rust2 0.13.2: MIT OR Apache-2.0, rust-version 1.85, not in Cargo.lock yet.
- scripts/docs-check.sh checks docs/ and README only; the root community files and .github templates are not scanned by it.
- scripts/collect-release.sh copies README.md, docs/ and ai-docs/; sources.zip takes every tracked file, so the new files are in it. Installers (scripts/package.sh) do not include them.
- .claude/skills/clone-workflow/check.sh requires every file under `.github` to be listed in manifest.txt (copy, template, skip). The root text files are outside its scan.

## Files (writers per AGENTS.md owner default accepted at REFINE: documenter writes the text and templates, principal-engineer writes the tests)
| File | Format | Content | Writer |
|---|---|---|---|
| `CODE_OF_CONDUCT.md` | Markdown | Covenant 2.1 text copied from upstream, "[INSERT CONTACT METHOD]" replaced by `community@andrealorenzani.name`, attribution kept | documenter |
| `CONTRIBUTING.md` | Markdown | Report a problem (SUPPORT.md, issue forms; security to SECURITY.md); propose a change (issue first for large changes, pull request, no merge promise); local checks `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, coverage gate 85% (`scripts/coverage.sh --fail-under 85`); contributions under GPL-3.0 (inbound = outbound, no contributor agreement); the code of conduct applies | documenter |
| `SECURITY.md` | Markdown | Supported version: latest release only; private reporting via GitHub private vulnerability reporting (link `https://github.com/andrealorenzani/videolore/security/advisories/new`); "do not open a public issue"; no response time promised | documenter |
| `SUPPORT.md` | Markdown | Questions and bugs through GitHub issues (link `https://github.com/andrealorenzani/videolore/issues/new/choose`), what to include (version, operating system, steps), README link, security via SECURITY.md, no response time promised, Discussions off | documenter |
| `.github/ISSUE_TEMPLATE/bug_report.yml` | GitHub issue form (YAML) | name "Bug report"; labels `bug`; required fields: "videolore version" (input), "Operating system" (input), "Steps to reproduce", "Expected result", "Actual result" (textarea); optional "Logs or screenshots" | documenter |
| `.github/ISSUE_TEMPLATE/feature_request.yml` | GitHub issue form (YAML) | name "Feature request"; labels `enhancement`; required "Problem" and "Proposal" (textarea) | documenter |
| `.github/ISSUE_TEMPLATE/config.yml` | YAML | `blank_issues_enabled: false`; contact links "Support" (SUPPORT.md blob URL) and "Report a security vulnerability" (`security/advisories/new`) | documenter |
| `.github/pull_request_template.md` | Markdown | Four checkboxes `- [ ]`: local checks pass (cargo fmt, clippy, test, coverage gate); tests cover the changed behaviour; user-facing changes described in the pull request; new Rust files carry the licence header (`// SPDX-License-Identifier: GPL-3.0-only`, copyright line) | documenter |
| `README.md` | Markdown | New `## Community` section before `## License` with links to CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md, SUPPORT.md and the issue chooser. Poem and licence section unchanged | documenter |
| `tests/community.rs` | Rust integration test | Tests below, one `// covers: REQ-NNN` per test | principal-engineer |
| `Cargo.toml` | `[dev-dependencies]` | `yaml-rust2 = "0.13.2"` with a comment; Cargo.lock updated by cargo | principal-engineer |

Form details are fixed by this table (labels exactly as written; the test reads them). GitHub's default labels `bug` and `enhancement` are assumed to exist (assumption A-3.1.3-1).

## Tests (tests/community.rs; the PE writes them with the code; suite run once at the end)
- REQ-359 `conduct_is_covenant_2_1`: CODE_OF_CONDUCT.md has "Contributor Covenant", "version 2.1" and the upstream URL; no "[INSERT"; the only email-like address in every community file and README is `community@andrealorenzani.name`.
- REQ-360 `contributing_names_checks_and_licence`: CONTRIBUTING.md contains `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `85%`, `GPL-3.0`.
- REQ-361 `security_policy_names_private_channel`: SECURITY.md contains "private vulnerability reporting" (case-insensitive), "latest release", "public issue", the advisories URL; the address rule above.
- REQ-362 `support_links_forms_and_readme`: SUPPORT.md contains `issues/new/choose`, `README.md`, "response time".
- REQ-363 `issue_forms_parse_and_require_fields`: the three YAML files parse (yaml-rust2); bug form has the five labelled fields, the four listed as required (`validations.required: true`); config has `blank_issues_enabled: false` and contact links to `security/advisories/new` and `SUPPORT.md`; feature form parses and has its two fields.
- REQ-364 `pull_request_template_lists_four_checks`: exactly four `- [ ]` items; keywords present: "cargo", "test", "user-facing", "SPDX-License-Identifier".
- REQ-365 `community_files_exist_and_readme_links_them`: REQ-359..364 files exist; README has "## Community" and links to the four root files.
- REQ-366 `public_files_have_no_internal_wording`: scan CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md, SUPPORT.md, .github/ISSUE_TEMPLATE/*, .github/pull_request_template.md and README.md (whole file, incl. poem) for the forbidden list (ADR-274); the test fails with file and line.
- REQ-366 `licence_is_unmodified_gpl3`: sha2 of LICENSE equals 3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986 and README "## License" section contains "Andrea Lorenzani".

## Ordered steps
0. Command: `/change-workflow` adds two manifest lines (ADR-275), because .claude/ is a workflow file (AGENTS rule 6). Must happen before the commit (check.sh must pass).
1. Documenter (after this plan): writes the files of the table, the README `## Community` section, the changelog entry 3.1.3 (bullet per REQ-359..366), runs docs-check.sh, refreshes its index. The Covenant text is fetched with curl into a new empty directory outside the project and copied, never retyped; if the fetch fails, stop and report.
2. Principal Engineer: adds the dev-dependency, writes tests/community.rs (SPDX header, one `// covers:` per test), updates Cargo.lock.
3. Principal Engineer, once at the end: fmt, clippy, test, coverage (≥ 85 %), feature coverage (REQ-359..366 must have tests; the product manager marks them implemented after the report), version check.
4. Documenter last: docs-check.sh, the summary bullets. Commit with the one-line message (no AI wording).
5. Manual (Andrea, on github.com, after the push): description, topics, private vulnerability reporting on, mailbox community@andrealorenzani.name receives mail, Community profile all green.

## Decisions
- ADR-271 community files are plain Markdown and YAML at the root and under `.github/`, English, hand-written, no generator. Alternatives: an organisation `.github` repository (rejected: this is one user's repository); a generated page (rejected: more to maintain). Reason: GitHub reads these paths by convention and the community profile checks them.
- ADR-272 issue forms in YAML (bug and feature), blank issues off, chooser links support and private security reporting. Alternative: Markdown templates (rejected: no required fields and no validation in the test). Reason: the bug report must ask version, operating system, steps, expected and actual result.
- ADR-273 conduct text: Contributor Covenant 2.1 copied from upstream with the contact placeholder replaced by community@andrealorenzani.name (Q-C1); attribution kept (CC BY 4.0). Alternative: own text (rejected: unvetted, more review). Reason: the standard text is recognised by GitHub and readers.
- ADR-274 tests in one integration file tests/community.rs. YAML is parsed with yaml-rust2 0.13.2 (dev-dependency, MIT OR Apache-2.0, rust-version 1.85, pure Rust, parse only). Alternatives: line-based grep for YAML (rejected: validity not checked); a Python check in CI (rejected: a second language for one check); serde_yaml (rejected: deprecated upstream). LICENSE is checked by sha2, already a dependency. Forbidden list in the test (case-sensitive whole words `AI`, `Claude`, `Anthropic`, `ChatGPT`, `Copilot`; case-insensitive whole word `agent`, `agents`; substrings `AGENTS`, `ai-docs`, `.claude`, `docs/workflow`, `workflow.md`, `change-workflow`, `add-feature`). `workflow` alone and `command` are not forbidden (GitHub URLs, "command line"). REQ-366 wording: "development-tool or internal process wording" is tested by this list; the product manager may add "(the terms of tests/community.rs)" to the requirement.
- ADR-275 no change to CI, packaging or the snapshot. CI: the check job runs `cargo test` (the new tests run there); build.yml, package.sh, collect-release.sh and release-guard.sh are unchanged; sources.zip includes the files; the installers do not. Clone-workflow: `skip .github/ISSUE_TEMPLATE/*` and `skip .github/pull_request_template.md` (project-specific names, address and URLs; no template); the root text files are not scanned and are not listed.
- Licence policy: cargo-deny already allows MIT and Apache-2.0; the dev-dependency ships in no binary, so the third-party notices are unchanged.

## Assumptions (checked by Andrea or at VERIFY)
- A-3.1.3-1 labels `bug` and `enhancement` exist on github.com (GitHub default labels).
- A-3.1.3-2 the paths listed above are the ones the community profile checks (GitHub documentation); the profile is checked by hand after the push.
- A-3.1.3-3 the advisories URL and the issues chooser URL keep their form on github.com.
- A-3.1.3-4 the covenant upstream is reachable by the documenter; the PE has no network during the build (the PE does not fetch anything).

## Risks
- R-GH1 public files show internal words (Q-C2 accepted: releases/ and docs/ are not in the scan; only the listed files are).
- R-GH2 the conduct mailbox receives no mail: manual check (Andrea).
- R-GH3 private vulnerability reporting off by default: reports can be lost until Andrea turns it on (manual).
- R-GH4 covenant text altered: the test checks the version line and the absence of the placeholder, not a full hash, because the placeholder replacement changes the file.
- R-GH5 the forbidden list grows or misses a term: docs-check.sh does not scan the root files (future work).

## Future work
- docs-check.sh to cover the root community files and .github templates (workflow change, /change-workflow).
- Manual checks above (profile, mailbox, reporting switch).
