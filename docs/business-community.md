# Business — GitHub community standards

## Purpose
videolore is public on GitHub. The repository shows the files and settings that GitHub's community profile checks, so that users know where to get help, how to report a problem or a security issue, how to contribute and how the community is expected to behave. Users never need the internal development material to use or report on the program.

## Users
- **Users** who need help, or want to report a bug or a security problem.
- **Contributors** who propose a change to the code or the documents.
- **The maintainer** (Andrea Lorenzani), who receives the reports and decides on contributions.

## Features (implemented, 3.1.3)
- **Code of conduct**: the Contributor Covenant 2.1 text, with the conduct report address community@andrealorenzani.name.
- **Contributing guide**: how to report, how to propose a change, the local checks a change must pass, and the licence of contributions.
- **Security policy**: the supported version, and private reporting through GitHub's private vulnerability reporting.
- **Support**: where to ask questions; no response time is promised.
- **Issue forms**: a bug report and a feature request; blank issues are off; the chooser links support and private security reporting.
- **Pull request template**: a checklist for the change.
- **README**: a short "Community" section links these files.

## Requirements
- REQ-359 (implemented): The repository has a code of conduct (`CODE_OF_CONDUCT.md`, root): the Contributor Covenant 2.1 with its attribution, and the conduct report address community@andrealorenzani.name. Acceptance: the file exists and names only that address; a test finds the file and the covenant's version line.
- REQ-360 (implemented): The repository has a contributing guide (`CONTRIBUTING.md`): how to report a problem, how to propose a change, the local checks a change must pass for the three programs (formatting, clippy, tests, the coverage gate of 85%, feature coverage), the build prerequisites, and that contributions are under GPL-3.0. Acceptance: the file names each check command in its workspace form and lists the build prerequisites, and states the licence of contributions.
- REQ-361 (implemented): The repository has a security policy (`SECURITY.md`): the supported version, how to report a vulnerability privately (GitHub private vulnerability reporting), and a request not to open a public issue for it. Acceptance: the file names the private channel and the supported version, and no address that Andrea did not give.
- REQ-362 (implemented): The repository has support information (`SUPPORT.md`): where to ask questions and report bugs, what to include, and that no response time is promised. Acceptance: the file links the issue forms and the README.
- REQ-363 (implemented): New issues use forms under `.github/ISSUE_TEMPLATE`: a bug report and a feature request; blank issues are off; the chooser links support and private security reporting. Acceptance: the bug form asks for the version, the operating system, the steps, the expected and the actual result; the chooser file has both links.
- REQ-364 (implemented): Pull requests start from a template (`.github/pull_request_template.md`) with a checklist: the local checks pass, tests cover the changed behaviour, user-facing changes are noted, new Rust files carry the licence header. Acceptance: the template exists and lists these four items.
- REQ-365 (implemented): The community files of REQ-359 to REQ-364 exist and the README's Community section links them. Not tested here and still pending (manual, Andrea): GitHub's community profile shows every item complete (description, README, code of conduct, contributing guide, licence, security policy, support, issue templates, pull request template) once the settings of the manual list are set. Acceptance: a test finds the files of REQ-359 to REQ-364 and the README links; Insights > Community shows all items after the push (manual check, not yet done).
- REQ-366 (implemented): The community files and the README contain no development-tool or internal process wording, and the LICENSE holds the unmodified GPL-3.0 text. Acceptance: a test scans the community files and the README for the forbidden terms and compares the LICENSE with the GPL-3.0 text; the README licence section still names Andrea Lorenzani.

## Key decisions
- Conduct text: Contributor Covenant 2.1 (CC BY 4.0, attribution kept) [MINOR].
- Conduct reports: community@andrealorenzani.name (given by Andrea, 2026-11-26). Security reports: GitHub private vulnerability reporting only; the conduct address is not used for them (Andrea, 2026-11-26).
- Contributions: issues and pull requests are welcome; Andrea reviews them and promises no merge; no contributor agreement; contributions are under GPL-3.0 [MINOR].
- Support: GitHub issues only; Discussions stay off unless Andrea turns them on [MINOR].
- Supported version: the latest release only [MINOR].
- LICENSE and the README licence section stay as they are (GPL-3.0, Andrea Lorenzani).

## Repository settings to set by hand on github.com (Andrea)
- **Description**: "Offline desktop video editor for short social media clips, with a separate video player and screen recorder." (suggested).
- **Website**: the project page address, only if the page is published.
- **Topics** (suggested): video-editor, video, rust, egui, ffmpeg, subtitles, offline, desktop-app.
- **Private vulnerability reporting**: Settings > Advanced security (or Code security): turn it on.
- **Conduct mail**: make sure community@andrealorenzani.name receives mail (it is the conduct report address).
- **Reported content**: Settings > Moderation options: check the reported-content setting (GitHub may name it differently).
- **Community profile**: Insights > Community: every item green after the push.

## Out of scope
- A conduct committee, a response-time promise, a bug bounty, a contributor agreement, a public mailing list, paid support.

## Known limitations
- The community profile is checked on github.com only, after the push; the tests can check the files, not the profile.
- Conduct reports depend on the mailbox community@andrealorenzani.name receiving mail (a manual check).
- Security reports depend on private vulnerability reporting being turned on by hand.
