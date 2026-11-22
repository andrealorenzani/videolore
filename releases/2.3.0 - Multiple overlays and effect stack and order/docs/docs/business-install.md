# Business — Installation

Back to the [index](business.md).

## Features
- One double-click installer per operating system, with ffmpeg and everything else included; no terminal and no separate installs.
  - **Windows (x86_64)**: a wizard; installs for the current user without administrator rights; a Start-menu entry; uninstalled from Apps settings.
  - **macOS (Apple Silicon)**: a disk image; drag videolore into Applications.
  - **Linux (x86_64)**: an AppImage, or a `.tar.gz` archive to extract; both run by double-click on current Ubuntu LTS and Fedora and add themselves to the application menu.
- An application icon; `.vlproj` project files open in videolore with a double-click.
- An About / third-party notices screen listing every bundled component.
- Installers for all three systems are built automatically for every release and published on the release page.

## Requirements
- REQ-030 (implemented): Import, preview and export work right after installation, with no ffmpeg installed on the machine. A message asking to install ffmpeg appears only if the bundled copy is missing or damaged. A user-chosen ffmpeg (Settings > Advanced, or an environment variable) can still override the bundled one. H.264 and HEVC export remain available.
- REQ-031 (implemented): An About / third-party notices screen (also shown by the installer) lists every bundled component and every speech model that can be downloaded (ffmpeg, the three speech models and others) with its version, licence and where to get its exact source code (a link or a written offer).
- REQ-040 (implemented): There is one double-click installer per operating system, needing no terminal and no separate installs (ffmpeg and runtimes included). Windows x86_64: wizard, per-user, no administrator rights, Start-menu entry, uninstall from Apps settings. macOS Apple Silicon: disk image, drag to Applications. Linux x86_64: an AppImage, after a one-time documented permission step, or the extracted `.tar.gz`, run by double-click on current Ubuntu LTS and Fedora; both add an application-menu entry and the `.vlproj` association by themselves. The Linux packages keep running on current Ubuntu LTS and Fedora even though they are built on a pinned build machine. Installers do no performance tuning: videolore sets itself up at its first start (REQ-045). Uninstalling never deletes user projects.
- REQ-041 (implemented): videolore has an application icon and `.vlproj` files are associated with it: double-clicking one opens it. If videolore is already running, on Windows and Linux the project opens in the running window (REQ-230); on macOS the file goes to the running application, and the unsaved-changes prompt (REQ-028) applies if the open project has unsaved changes.
- REQ-042 (implemented): Installers for all three operating systems are built automatically for every release and published on the release page, on explicitly pinned build machines. They are not code-signed; the README has one section per operating system with download and installation steps, including the first-launch steps (macOS Gatekeeper "unidentified developer", Windows SmartScreen "unknown publisher", Linux AppImage "allow executing as program"). Acceptance: every release has the three installers; the README covers those steps for each system.

## Key decisions
- ffmpeg is bundled (this reverses the 0.1.x rule that users had to install it), because the product must be easy to install for non-experts. The bundled build includes H.264 and HEVC encoders, which are GPL-licensed; their exact source code is published with every release, together with a written offer.
- Installers are unsigned; code signing and notarisation are not planned (Andrea, 2.0.2).
- Linux gets both an AppImage and a `.tar.gz`, because some systems handle one better than the other.
- On Windows, installation is per-user, so no administrator rights are needed.
- Computer detection is done by videolore itself at its first start, not by the installers, so the behaviour is the same on every system.
- The third-party notices note that patent licensing for H.264 and HEVC varies by country.

## Known limitations
- Unsigned installers: macOS and Windows show a warning on first launch, and the AppImage needs the "allow executing" permission once; the README explains each step.
- Some Linux file managers do not start a program from the extracted `.tar.gz` on double-click; the archive has a launcher and a desktop entry (REQ-231), and the README gives the fallback.
- Assumption: the way source code is offered for bundled components has not been reviewed by a lawyer (see the [roadmap](roadmap.md)).

## Out of scope
- Code signing and notarisation (not planned).
- Intel Macs, ARM Windows and ARM Linux; app stores; Linux distribution packages other than the AppImage and `.tar.gz`; automatic updates.
