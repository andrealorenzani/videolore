# Business — Settings, performance and safety

Back to the [index](business.md).

Since 0.3.0.

## Features
- **Automatic setup**: the first time videolore starts on a computer (on every operating system, the Linux AppImage included), it looks at the computer and picks the recommended settings. A short notice says so and closes by itself. The user needs no technical knowledge.
- **Settings window**: Edit > Settings… (Ctrl/Cmd+,) opens (until 1.0 a "Settings" button in the command bar) a window inside videolore with plain choices:
  - **Preview quality**: Automatic / Smoother / Original.
  - **Subtitle quality**: Fast / Balanced / Most accurate.
  - **How hard videolore works**: Light / Balanced / Maximum speed.
  - **Export**: Faster / Best quality (since 0.4.0; Best quality is the default; see [Export](business-export.md#faster-export-040)).

  Each choice has a one-line explanation. A status line says "Using recommended settings for this computer" or "Custom settings". The window also shows how much disk space the preview copies and the speech recognition files use, with "Clear preview copies" and "Remove downloaded speech files". A collapsed "Advanced" section holds the technical details: the user-chosen ffmpeg, the paths, versions and origin of the bundled components, and (since 0.4.0) which video hardware works for export, which method the last export used, and "Try video hardware again".
- **Use recommended settings**: a button in Settings that checks the computer again and restores the recommended choices.
- **More accurate subtitles**: two larger speech recognition files (Balanced and Most accurate), downloaded only with the user's consent the first time they are needed.
- **Older processors**: on computers whose processor is too old for automatic subtitles, videolore says so in plain words instead of crashing; everything else works.
- **Safety**: damaged or hostile project, settings and media files cannot crash videolore, run other programs or write anywhere unexpected; downloads are checked; temporary files are private and cleaned up.

## Requirements

### Automatic setup
- REQ-045 (implemented): On first start (no settings file), on every operating system, videolore detects the computer and saves recommended values for every performance setting, then shows once a notice that closes by itself: "videolore has been set up for this computer. You can change this in Settings." Acceptance: the settings are created and the notice is shown once; on the second start there is no notice and no new detection (unless REQ-048 applies); detection takes at most 3 seconds, runs no long benchmark and uses no network; the same computer characteristics always give the same result.
- REQ-046 (implemented): Settings is a window inside videolore, opened from the Edit menu (from the command bar until 1.0), with three plain-language choices: Preview quality (Automatic / Smoother / Original), Subtitle quality (Fast / Balanced / Most accurate) and How hard videolore works (Light / Balanced / Maximum speed), each with a one-line explanation, and a status line "Using recommended settings for this computer" or "Custom settings". A collapsed "Advanced" section holds the user-chosen ffmpeg and the paths, versions and origin of the bundled components. Acceptance: (a) the main section contains none of the words thread, core, proxy, codec, encoder, ffmpeg, AVX, CPU, GPU, model, quantised; (b) changes apply without restarting; (c) settings belong to the program, never to a project file; (d) choices this computer cannot use are greyed out with a plain reason. Since 0.4.0 the main section also has the Export choice (REQ-060) and Advanced shows the video hardware status (REQ-061, REQ-064); acceptance (a) still applies.
- REQ-047 (implemented): A "Use recommended settings" button checks the computer again, resets the choices (since 0.4.0 also Export, back to Best quality, clearing the remembered hardware failures and running the hardware check again, REQ-064) and shows a plain summary of what changed, or "Already using the recommended settings". Acceptance: the result is exactly the first-start recommendation (REQ-045); the user-chosen ffmpeg, projects, preview copies and downloaded speech files are untouched; nothing is downloaded.
- REQ-048 (implemented): When the computer's characteristics differ from those saved: if the recommended settings were in use, they are recomputed silently and the setup notice of REQ-045 is shown; if the user had custom settings, they are kept and a notice offers "Use recommended settings" without blocking. Acceptance: both cases tested with simulated computer data.
- REQ-049 (implemented): Upgrading from 0.2.x behaves as a first start, except that: if preview copies had been switched off, Preview quality becomes Original; a user-chosen ffmpeg is kept; an already downloaded speech file is kept. Acceptance: a 0.2.2 settings file with preview copies off and a user-chosen ffmpeg loads as Original with the same ffmpeg; projects from 0.1.x and 0.2.x open identically (REQ-029).

### Performance
- REQ-050 (implemented): "How hard videolore works" controls all background work: how many ffmpeg processes run at once, and how many threads export, preview copies and speech recognition use. Light uses at most half of the computer's logical cores (at least 1) for each background job; Balanced is the usual recommendation and matches 0.2.2; Maximum speed may use all cores; videolore never configures more threads than the computer has. The exact mapping is documented in the [architecture](architecture.md). Acceptance: the mapping is tested at 1, 2, 4, 8, 16 and more cores and with less than 8 GB, 8 GB and 16 GB or more of memory; a change applies to the next job and running jobs are not restarted.
- REQ-051 (implemented): Preview quality replaces the 0.2.x on/off switch for preview copies: Automatic uses the existing rule (REQ-038); Smoother makes a preview copy of any source larger than 1280x720; Original never makes one. Acceptance: the decision is the same for the same inputs; the exported file is the same whatever the choice.
- REQ-052 (implemented): Subtitle quality has three levels: Fast uses the 0.2.x speech file (about 32 MB, never worse than 0.2.2); Balanced (about 190 MB) and Most accurate (about 574 MB) use larger, more accurate ones. The recommendation depends on the computer: Most accurate only where it runs reasonably fast, and levels needing more memory than the computer has are greyed out. A level's file is downloaded only when subtitles are generated with that level and the file is missing, after a consent dialog showing size and licence; the dialog offers "Use Fast instead (no download)" when the Fast file is present (for that run only; the setting does not change), or "Download the Fast one instead" when nothing is installed; a local file can still be chosen for each level. Afterwards everything works offline. Acceptance: on a reference clip the word error rate of each higher level is at most that of Fast; declining changes nothing; changing the level never regenerates subtitles by itself (REQ-044 applies when regenerating). Since 1.2.0 the chosen level is also used to match the words of a video's own subtitles to the speech (REQ-095), with the same download consent.
- REQ-053 (implemented): Settings shows the disk space used by downloaded speech recognition files and a "Remove downloaded speech files" button (with confirmation); projects and their subtitles are untouched, and a file is downloaded again, with consent, when next needed. Acceptance: the shown size matches the disk; after removal the space is freed and existing subtitles still show and export.
- REQ-054 (implemented): Subtitle generation never crashes on older processors. On a processor without the features the speech engine needs (for example x86-64 processors without AVX2, from before about 2013), "Generate subtitles" shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand", no download is offered, and the three subtitle levels are greyed out with the same reason. The rest of the program works normally. Since 1.2.0, on such processors the subtitles already contained in a video can still be used: their text and line timing are kept and the words of each line are spread evenly, without matching to the speech and without any download (REQ-095). Acceptance: with a simulated processor lacking the features, there is no crash and the message is shown; the recommendation never picks a level that cannot run.
- Faster export with the computer's video hardware (REQ-055, REQ-060 to REQ-065, since 0.4.0) is described in [Export](business-export.md#faster-export-040).
- REQ-056 (implemented): The window stays responsive while busy: with an export and subtitle generation running together at any "How hard" level, play/pause, timeline scrolling and opening Settings keep reacting. Acceptance: in a test harness every input is handled and visible within 200 ms. REQ-032 is unchanged. Since 1.1.0 this also covers dragging section borders and having effect popups open (REQ-066, REQ-076). Since 1.2.0 it also covers dragging an effect onto a clip and dragging a subtitle in the preview while an export and subtitle matching (REQ-095) run.

### Safety
- REQ-057 (implemented): Damaged or hostile project, settings or media files never crash videolore, never make it run another program, never make it write outside the folders chosen by the user or its own folders, and never modify sources. Damaged projects are refused with a clear message; damaged settings are replaced by the recommended settings, with a notice. Acceptance: a test set of bad files (invalid or truncated data, huge numbers, absolute or `..` paths, control characters in names, oversized files) passes all of the above. Since 1.1.0 the test set also covers damaged layout values and recent-projects lists in the settings (REQ-067, REQ-080): these fall back to the default layout or an empty list, keep the other settings and show no notice. Since 1.2.0 it also covers subtitle tracks inside media files (damaged or hostile text, names and languages) and the new project values (REQ-094).
- REQ-058 (implemented): Speech recognition files are downloaded only from fixed, built-in secure (HTTPS) addresses and checked against their known fingerprint (SHA-256) before use; partial or wrong files are deleted. There is no other network access, no telemetry and no update check. Acceptance: tests for a wrong fingerprint, an interrupted download and a refused non-HTTPS address.
- REQ-059 (implemented): Temporary files (extracted audio, partial exports, partial preview copies, partial downloads) can be read only by the user, are removed when the work finishes or is cancelled, and leftovers from a crash are cleaned up at the next start. Acceptance: none remain after a cancel or after a simulated crash and restart; on Linux and macOS they are readable only by the user.

## Quality goals
These are engineering targets (QG-01 to QG-04 since 0.3.0, QG-05 since 0.4.0); they are measured and reported for each development but have no feature tests. The numbers are assumptions that can be adjusted after the first measurements.
- **QG-01 Performance review**: every choice (number of processes and threads, speech engine and files, each library, preview and export settings) is reviewed and justified. No slowdown: at the recommended settings, export time and Fast subtitle time are at most 5% above 0.2.2 on the same computer, and start-up (without the first-start detection) is at most 0.5 s slower.
- **QG-02 Security upkeep**: the automatic build fails on any known vulnerability in a dependency, unless it is documented as not affecting videolore; the bundled ffmpeg is updated to the latest release.
- **QG-03 Faster tests**: the test run is at least 30% faster than in 0.2.2 on the same computer, without removing or weakening any test, keeping line coverage of at least 85% and every implemented requirement tested. **Result in 0.3.0: 25% faster**, short of the 30% target.
- **QG-04 Clean automatic builds**: no deprecation warnings in the automatic builds; the Linux build machine is pinned explicitly, and the Linux packages still run on current Ubuntu LTS and Fedora (REQ-040).
- **QG-05 Real-hardware check of Faster export** (0.4.0): the automatic build machines have no graphics hardware, so hardware export is checked by hand on: Windows with NVIDIA, Intel and AMD; Linux with NVIDIA (proprietary driver), Intel and AMD; macOS on Apple Silicon. On each, a reference project (mixed formats with letterboxing, 50 or more cuts, fades, music, subtitles) is exported in H.264 and HEVC with Faster and with Best quality, and:
  - a. Settings, Advanced shows that the video hardware was used;
  - b. format and number of frames are identical to Best quality;
  - c. HEVC carries the hvc1 tag and plays in QuickTime and on an iPhone;
  - d. full-screen viewing shows no blocking, banding, colour shift or green frames;
  - e. the quality score is at most 3 VMAF points below Best quality;
  - f. the file is at most 1.5 times the size of the Best quality file;
  - g. total time, including the output check, is at least 40% lower than Best quality at 1080p (assumption; a measured target, adjusted after measurement and not a release gate; the estimate is 20–45%, so it may not be met);
  - h. with the hardware forced to fail (driver disabled or unavailable), a correct file is still produced with the normal method.

  Results are recorded for each release in the developer documentation; combinations not verified are listed as known limitations.

  **Results in 0.4.0:**

  | Computer | Method | e. quality | f. size | g. time | Notes |
  |---|---|---|---|---|---|
  | Linux (Ubuntu 24.04), Intel Iris Plus G7, system ffmpeg | VA-API, H.264 | met (3 projects) | met (3 projects) | **not met**: from 14% quicker to 41% slower than Best quality | HEVC hardware is broken in this driver; the normal method makes HEVC files. With the bundled ffmpeg the hardware is skipped (system libva 2.20, 2.21 needed). |

  Other combinations are not yet measured. Goal g is not met on integrated Intel graphics: the hardware export plus the output check is not quicker than the normal method there. Recommendation: re-baseline g after measuring a dedicated graphics card (for example a per-kind-of-graphics target instead of one 40% figure), and consider a cheaper output check later, as long as it still catches the faults listed in REQ-063.

- **QG-06 Shorter automatic builds** (1.6.1): the automatic build and test run on GitHub should finish sooner. Only the system packages the build really needs are installed, and the slow steps are shortened. Target (assumption, measured and reported, not a release gate): the 1.6.1 run takes at least 30% less time than the 1.6.0 run on the same runner. Nothing is removed from the checks: formatting, lints, tests, line coverage of at least 85%, feature coverage of 100%, version and release checks and the executables still run, and the Linux packages still run on current Ubuntu LTS and Fedora (REQ-040).

## Key decisions
- Detection runs inside videolore at first start, on every operating system; installers do no tuning. One way of working everywhere, and it can be tested.
- One plain "How hard videolore works" choice instead of thread or process numbers; technical values never appear in the main part of Settings.
- Settings is a window inside videolore; it does not change the window layout (REQ-024).
- The setup notice closes by itself; there is no setup wizard.
- When the computer changes, custom settings win over the recommendation.
- "How hard" Balanced behaves exactly like 0.2.2, so nothing gets slower. Light limits each background job to half the cores; two jobs running together can still use all cores in total, and preview playback is never limited.
- Exports made at different "How hard" levels may differ by a few bytes but look the same and have the same length. Preview quality never changes the export.
- Speech recognition levels: Fast = the 0.2.x file (about 32 MB); Balanced = about 190 MB; Most accurate = about 574 MB. All are MIT-licensed. Most accurate is recommended only on computers with at least 12 logical cores and about 16 GB of memory, Balanced on at least 4 cores and about 8 GB. Many users will therefore be asked once to approve a larger download, with the 32 MB Fast file offered in the same dialog.
- Older processors get a clear message instead of a slower fallback engine, because a fallback engine cannot be included in the program as it is built today. Confirmed by Andrea for 0.4.0: the slower fallback engine is dropped, not postponed (it can be reconsidered on request).
- Hardware video encoding for export was not built in 0.3.0 (it cannot be verified on the automatic build machines, needs tuning per graphics vendor, and some driver faults produce bad video without an error). In 0.4.0 it is built for every vendor, with an automatic fallback and an output check that address those risks, and is verified by hand (QG-05); see [Export](business-export.md#faster-export-040).
- Confirmed by Andrea for 0.4.0: Settings is a window inside videolore; Light limits each background job rather than setting a total budget; the three speech recognition levels and their recommendation thresholds stay as described above.
- Temporary files live in videolore's own private cache folder instead of the system temporary folder.

## Known limitations
- Speed and memory use of the larger speech recognition files, and the thresholds for recommending them, are estimates to be confirmed by measurement (assumption).
- On virtual machines or containers whose number of cores changes between starts, videolore may treat each start as a new computer; with custom settings this only shows a notice.
- Processors too old for the speech engine (x86-64 without AVX2) cannot generate subtitles automatically; lines can still be added by hand. No slower fallback engine is planned.
- Known limitations of Faster export (graphics driver versions, Linux drivers, computers with several graphics cards) are listed in [Export](business-export.md#known-limitations-040).

## Out of scope
- Detection or benchmarks inside installers; long benchmarks; online databases of computers; telemetry.
- Speech recognition files other than the three offered; translation.
- Graphics-card acceleration for preview or effects (export can use it since 0.4.0, see [Export](business-export.md)).
- A slower fallback speech engine for processors without AVX2.
- Performance settings per project.
- Raw technical values (threads, cores, codecs) in the main part of Settings.
