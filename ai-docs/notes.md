# ai-notes: notes

- docs inventory: README.md LICENSE docs/{business,architecture,code,changelog,workflow}.md; no sub-documents yet; release/ (frozen, never touch, currently empty)
- version 0.1.5 set via scripts/version.sh (needs PKG_CONFIG_PATH to alsa.pc on this machine for coverage; `set` itself works without it)
- 0.1.5 dev: changelog entry 2026-11-14 01:03 +0000; coverage 96.0 (README badge updated), features 18/18; business.md music/subtitles now "release not yet assigned"
- last README poem: 2026-11-14 (0.1.5, 10 lines, lines 1-9 end with backslash)
- README rules: "workflows" appears only in GitHub Actions badge/link URLs (allowed)
- open: AGENTS.md (not mine) still says "MSRV 1.88" in Project section; fix via /change-workflow
- open: README has badges for eframe, cpal, rfd, serde, ffmpeg only; serde_json, thiserror, directories, tempfile have none (rule: one badge per main library)
- open: ai-docs/architecture.md and ai-docs/code.md are PM/Arch/PE notes, not mine
- open: docs/architecture.md says "Implemented (since 0.1.3)"; keep in sync when new REQs are implemented
- open: ffmpeg module is the only one starting processes (true today); keep in sync
- open: tempfile is a normal dependency (runtime), sha2 dev-only
