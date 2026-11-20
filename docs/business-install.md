# Business — Installation

Back to the [index](business.md).

## Features
- One double-click installer per operating system, with ffmpeg and everything else included; no terminal and no separate installs.
  - **Windows (x86_64)**: an installation wizard, installs for the current user without administrator rights, adds a Start-menu entry, uninstalled from Apps settings.
  - **macOS (Apple Silicon)**: a disk image; drag videolore into Applications.
  - **Linux (x86_64)**: an AppImage, or a `.tar.gz` archive to extract; both run by double-click on current Ubuntu LTS and Fedora and add themselves to the application menu.
- An application icon, and `.vlproj` project files open in videolore with a double-click.
- An About / third-party notices screen listing every bundled component.
- Installers for all three systems are built automatically for every release and published on the release page.

## Requirements
- REQ-030 (implemented): Import, preview and export work right after installation, with no ffmpeg installed on the machine. A message asking to install ffmpeg appears only if the bundled copy is missing or damaged. A user-chosen ffmpeg (set in Settings → Advanced since 0.3.0, or by environment variable) can still override the bundled one. H.264 and HEVC export remain available, with the same output formats as 0.1.3.
- REQ-031 (implemented): An About / third-party notices screen (also shown by the installer) lists every bundled component and every speech model that can be downloaded (ffmpeg, the three speech models and others) with its version, licence and where to get its exact source code (a link or a written offer).
- REQ-040 (implemented): There is one double-click installer per operating system, needing no terminal and no separate installs (ffmpeg and runtimes included). Windows x86_64: wizard, per-user, no administrator rights, Start-menu entry, uninstall from Apps settings. macOS Apple Silicon: disk image, drag to Applications. Linux x86_64: an AppImage, after a one-time documented permission step, or the extracted `.tar.gz`, run by double-click on current Ubuntu LTS and Fedora; both add an application-menu entry and the `.vlproj` association by themselves. The Linux packages keep running on current Ubuntu LTS and Fedora even though they are no longer built on a moving "latest" build machine. Installers do no performance tuning: videolore sets itself up at its first start (REQ-045). Uninstalling never deletes user projects.
- REQ-041 (implemented): videolore has an application icon and `.vlproj` files are associated with it: double-clicking one opens it. If videolore is already running, on Windows and Linux a second window opens with that project (the first window and its unsaved work are untouched, no prompt); on macOS the file goes to the running application, and the unsaved-changes prompt (REQ-028) applies if the open project has unsaved changes.
- REQ-042 (implemented): Installers for all three operating systems are built automatically for every release and published on the release page, on explicitly pinned build machines. In 0.2.0 they are not code-signed; the README has one section per operating system with download and installation steps, including the one-time first-launch steps (macOS Gatekeeper "unidentified developer", Windows SmartScreen "unknown publisher", Linux AppImage "allow executing as program"). Acceptance: every release has the three installers; the README covers those steps for each system.

## Key decisions
- ffmpeg is bundled (this reverses the 0.1.x rule that users had to install it), because the product must be easy to install for non-experts. The bundled build includes H.264 and HEVC encoders, which are GPL-licensed; their exact source code is published with every release, together with a written offer.
- Installers are unsigned. Andrea decided in 2.0.2 that they stay unsigned; code signing and notarisation are not planned.
- Linux gets both an AppImage and a `.tar.gz`, because some systems handle one better than the other.
- On Windows, installation is per-user, so no administrator rights are needed.
- On Windows and Linux, a project opened by double-click while videolore is running gets its own window, so nothing in the open window is ever lost.
- The third-party notices include a note that patent licensing for H.264 and HEVC varies by country.

- Since 0.3.0 the computer is detected by videolore itself at first start, not by the installers, so the behaviour is the same on every system, the AppImage and `.tar.gz` included.

## Known limitations
- Unsigned installers: macOS and Windows show a warning on first launch, and the AppImage needs the "allow executing" permission once; the README explains each step.
- Some Linux file managers do not start a program from the extracted `.tar.gz` on double-click; the README gives the fallback (right-click and run, or start it once from a terminal).
- On Windows and Linux, double-clicking a project while videolore is open starts a second window instead of reusing the first.
- Assumption: the way source code is offered for bundled components has not been reviewed by a lawyer.

## Out of scope
- Code signing and notarisation (not planned, decided 2.0.2).
- Intel Macs, ARM Windows and ARM Linux.
- App stores, Linux distribution packages other than the AppImage and `.tar.gz`.
- Automatic updates.
