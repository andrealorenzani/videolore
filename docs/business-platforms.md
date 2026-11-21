# Business — Platforms and installers (2.0.4)

Back to the [index](business.md). Batch 11 of the [roadmap tasks](roadmap_tasks.md), built in the 2.0.4 development. The installers stay unsigned (decided by Andrea on 2026-11-20). The parts that need a real system are in [verification](business-verification.md).

## CI checks
- REQ-225 (implemented): The installer configuration job of the automatic build (QG-10) also runs on every pull request that changes `scripts/package.sh`, `packaging/` or `assets/icon/`, and the executables job does not start unless it passed. Acceptance: the workflow's triggers list those paths; a broken installer configuration fails the check before the executables are built.
- REQ-226 (implemented): The dependency security-advisory check also runs on a weekly schedule, not only on pushes. Acceptance: the workflow has a schedule trigger for it; a known advisory fails the scheduled run.
- REQ-227 (implemented): On every push, the automatic build runs the processor-too-old message path of REQ-054 with the processor features forced off, so the friendly message is tested on every run. Acceptance: the job fails if the message path is removed or crashes; it is not a real old processor.
- REQ-228 (implemented): `packaging/tools.lock` records the version and SHA-256 of the NSIS toolchain that the Windows installer uses, like the other downloads; a different file fails the build. Acceptance: a test finds the entry in the lock file; a wrong hash fails the installer job.

## Installers and file managers
- REQ-229 (implemented): The licence page of the Windows installer shows `third-party.txt` correctly, with its non-ASCII text (the file is written with a UTF-8 byte-order mark). Acceptance: the generated file starts with the byte-order mark; the page is checked by eye on Windows (see [verification](business-verification.md)).
- REQ-231 (implemented): The Linux `.tar.gz` contains a desktop entry with the application icon and a launcher script, so file managers that do not start an extracted program on double-click can still start videolore from their menu. The README keeps its fallback steps. Acceptance: the archive lists the desktop entry and the launcher; the launcher is executable and starts the program; the file managers in the checklist start it.

## Windows and Linux: one window
- REQ-230 (implemented): When videolore is already running, a double-clicked project opens in the running window (after the unsaved-changes prompt of REQ-028) instead of a second window; if the running window cannot be reached, a new window opens as before. Amends REQ-041 and REQ-133 (the second window of 0.2.0 is replaced on Windows and Linux; macOS keeps the running application). Acceptance: with the editor running, a double-click on a project opens no second window; the running window opens the project after the prompt; with no running window a new one starts; a drop of several projects still opens the first one (REQ-081).

## macOS and Wayland
- REQ-232 (implemented): On macOS the menu bar is at the top of the screen, with the standard Hide, Hide Others and Minimize items. Cmd+Q still goes through the unsaved-changes prompt (REQ-028). Amends REQ-069 and the macOS limitation of 1.1.0. Acceptance: on a Mac the menu bar is at the top, Hide and Minimize work, Cmd+Q asks about unsaved changes (checked by hand, see [verification](business-verification.md)); the menu definition contains the three items (test).
- REQ-233 (implemented): Where the windowing library reports dropped files (Wayland sessions, when the library supports it), dropping media and projects works as on X11 (REQ-081). Otherwise File > Import media… and File > Open… remain the way in. Acceptance: on a Wayland session with support, a dropped video is imported; without support the known limitation stays and the File menu works.

## Key decisions (2.0.4)
- [MINOR] One window for double-clicked projects is built although single-instance detection was rejected earlier (rejected list, 2.0.3); the reason is this roadmap item. The data goes through the running window, not through a new process; the technique is chosen when it is built.
- [MINOR] Intel Macs, ARM Windows and ARM Linux stay out of scope in 2.0.4 (the build targets of REQ-040 are unchanged).
- Detection inside the installers stays out (open question on the roadmap; detection stays at first start, REQ-045).
- The legal review of the source offer is Andrea's check, outside the code.

## Out of scope (2.0.4)
- Intel Macs, ARM Windows and ARM Linux builds; app stores; Linux packages other than the AppImage and `.tar.gz`.
- Code signing and notarisation (not planned).
