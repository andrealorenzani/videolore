# ai-notes: notes

- docs inventory: README.md LICENSE; docs/ index+sub-docs: business{,-editing,-export,-install,-music,-subtitles}.md, architecture{,-ffmpeg,-media-pipeline,-packaging,-security,-subtitles,-dependencies}.md, code{,-modules,-testing,-packaging}.md, changelog.md, workflow.md; release/ frozen, never touch (empty)
- all sub-docs linked from their index; no dead links (checked 0.2.0); anchor architecture-ffmpeg.md#subtitle-rendering-and-fonts exists
- version 0.2.0 set via scripts/version.sh (Cargo.toml, Cargo.lock, README badge); check consistent
- 0.2.0 dev: changelog entry 2026-11-14 10:17 +0000; coverage 95.4 (README badge), features 44/44
- last README poem: 2026-11-14 (0.2.0, 10 lines, lines 1-9 end with backslash)
- README rules: "workflows" only in GitHub Actions badge/link URLs (allowed); no AI/agent/command/workflow words
- README badges: one per normal dependency (eframe, cpal, rfd, whisper-rs, ureq, serde, serde_json, thiserror, directories, tempfile, sha2, sysinfo) + ffmpeg 9.0.2 bundled (from packaging/ffmpeg.lock); macOS-only objc2/winit have no badge
- docs fixes 0.2.0: business-editing shortcut "S for split" -> Ctrl/Cmd+B (+Save/Quit); business-editing known limitations +3 (undo history on media removal, proxy regen, 720p); architecture-packaging link to code-packaging.md
- UI labels: command bar "Clear proxy cache", settings "Clear preview copies" (both real, docs use both correctly)
- open: AGENTS.md (not mine) Project section says MSRV 1.88 (stale, fix via /change-workflow)
- open: business.md "Known limitations (0.2.0)" does not list the 3 editing limitations (kept in business-editing.md)
- open: docs/workflow.md unchanged in 0.2.0 (.claude/ and AGENTS.md untouched in this development)
- open: docs/business-editing.md uses "freezes the window" (product wording, not WIP; left as is)
- open: ai-docs/architecture.md and ai-docs/business.md contain stale 0.1.x facts? not mine; PM/Arch to refresh if needed
