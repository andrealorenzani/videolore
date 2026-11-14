# ai-notes: notes

- docs inventory: README.md LICENSE docs/{business,architecture,code,changelog,workflow}.md; release/ (frozen, never touch)
- version 0.1.3 set via scripts/version.sh (needs PKG_CONFIG_PATH to alsa.pc on this machine; without it `set` fails at cargo check and leaves README badge stale)
- 0.1.3 dev: changelog entry added (2026-11-14 00:26 +0000) with Known limitations; coverage 96.4, features 18/18, fmt+clippy ok
- last README poem: 2026-11-14 (0.1.3, 10 lines, verses 1-9 end with backslash)
- README rules: "workflows" appears only in the GitHub Actions badge/link URLs (allowed)
- docs/code.md: Known limitations section (0.1.3) added
- docs/architecture.md: "UI never waits" and "file inspection in background" flagged with 0.1.3 exceptions; tempfile + sha2 rows added
- open: architecture says ffmpeg module is the only one starting processes (true today); keep in sync
- open: tempfile is a normal dependency (used at runtime for concat list/graph/preview scripts), sha2 dev-only
- open: MSRV 1.88 only verified locally; CI uses stable
