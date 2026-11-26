# Architecture: videolore screen recorder (3.1.2, built)

Part of [Architecture](architecture.md). Requirements: [screen recorder](business-screen-spy.md) (REQ-341 to 352, 357, 358). Decisions: [decisions](architecture-decisions.md#built-in-312-adr-255-to-270). Installers: [packaging](architecture-packaging.md#third-program-the-screen-recorder-312). Module map with diagram: [code-architecture](code-architecture.md#videolore-screen-spy). Status: built in 3.1.2; Linux verified on a local X11 session and CI, Windows and macOS not verified on hardware (see [Not verified](#not-verified)).

## Overview
- A third program, `videolore-screen-spy`, with its own executable and installers. It records one screen to a video file (MP4, or MKV for WAV sound) with the computer sound and the microphone mixed into one sound track. It has a live preview, no editing, no project file and no network.
- It is a separate workspace member (`screen-spy/`), built like the player. It uses the editor's library only for the ffmpeg lookup and runner, the bounded file helpers, the icons and the notices; the library is built **without** its features (no sound output, no speech, no network).

## How the recording is made
- **One ffmpeg child at a time.** The picture comes from ffmpeg's own screen input: `gdigrab` on Windows, `avfoundation` on macOS (input by screen name `Capture screen N`), `x11grab` on Linux (X11 sessions and XWayland). The screen's position and size come from the list of screens (display-info).
- **Preview and recording share one capture.** The child has two outputs: the recording file and a small raw preview on its standard output (5 frames per second, 640 pixels wide, a reader thread keeps only the newest frame). Before Start a preview-only child runs, and it runs again between recordings; Start replaces it with the recording child (about 1 second without picture).
- **Input order.** The sound inputs are opened first and the screen last, so the file starts with the picture while the sound already runs. On Linux every `pulse` input carries `-isync` to the screen (both stamp the wall clock).
- **Sound, Linux.** Two ffmpeg `pulse` inputs: computer sound `@DEFAULT_MONITOR@`, microphone `@DEFAULT_SOURCE@`. Each is tried for 0.1 s before Start. When ffmpeg lists only monitors there is no microphone. When the default source is itself a monitor, Start is refused with a message, because the computer's sound would be recorded twice. No source name is passed to ffmpeg.
- **Sound, Windows.** The computer sound is a cpal loopback of the default output (WASAPI); the microphone is the default input. No driver is installed.
- **Sound, macOS 14.6 or later.** The computer sound is the program's own CoreAudio tap of everything the computer plays, inside a private device, recorded by cpal as an ordinary input. cpal's own output loopback was not used: it records the inputs of an output device that has some. The microphone permission is asked first, and the answer is awaited (up to 2 minutes) before any source opens. Older macOS (below 14.6) refuses Start with a message; no recording starts.
- **Transport.** Each cpal source is converted to 16-bit stereo PCM and sent over its own TCP connection to 127.0.0.1 (ephemeral port, one connection, peer must be 127.0.0.1; nothing is stored). A source that falls more than 200 ms behind the clock is filled with silence. ffmpeg mixes the sources with `amix` (`normalize=0`) and `alimiter`.
- **File.** H.264 `libx264 veryfast`, CRF 23, a key frame every second, `-flush_packets 1`. AAC or MP3 go into a fragmented MP4 (a file cut short by a crash, a full disk or a removed screen still plays). WAV goes into a Matroska file (`-cluster_time_limit 1000`). Stop sends `q` and a newline to the recording child's stdin; after 15 seconds the child is killed. A killed recording keeps all but about the last second.
- **Formats.** Video: 1280x720 at 30 fps, 1920x1080 at 30 fps (default), 1920x1080 at 60 fps, 3840x2160 at 30 fps. Audio: AAC (default) or MP3 in an MP4 file, WAV in an MKV file (the extension follows the audio format, REQ-345).
- **Screens.** The list is read every 2 seconds. `x11grab` takes position and size in pixels; `gdigrab` takes offsets; display-info units are divided by the Xft scale on Linux, points on macOS, pixels on Windows (`screens::physical`). If the recorded screen disappears the recording stops and keeps its file, with the cause.

## Components (screen-spy/src)
| Module | Role | Pure |
|---|---|---|
| `app` | state (idle, previewing, starting, recording, stopping), commands, start checks, messages, popups; everything injected through `Deps` | yes (fake runner, sound, screens, clock) |
| `plan` | ffmpeg arguments for preview and recording per system; format table; audio-to-container rule; default file name (jiff local time) | yes |
| `session` | Linux session decision (native Wayland refused, REQ-358); macOS version decision (14.6 for computer sound, REQ-357); permission and microphone messages | yes |
| `screens` | screen list, numbering "Screen N, WxH", display-info units to pixels | mapping pure, display-info adapter thin |
| `devices` | readers of `ffmpeg -devices`, `-sources pulse` and `-encoders`; what a recording needs; the avfoundation screen name | yes |
| `process` | the only place that starts a capture: spawner, `q` to stop, 15 s stop timeout, failure cause | no (adapter) |
| `preview` | newest-frame feed, rgb24 640 px, 5 fps | yes |
| `audio` | `Sound`: Linux `pulse` inputs; Windows and macOS cpal streams, the thread that owns them (`hold`), 16-bit stereo conversion, silence filler, `Feeder` (127.0.0.1, one connection) | conversion and filler pure |
| `macos` | macOS version, screen recording permission, microphone status and question, the computer sound tap (`SystemTap`) | no; the only `unsafe` code of the program |
| `settings` | `screen-spy.json` version 1; a damaged file is renamed `.bad` | yes |
| `ui`, `about` | the window and its popups (Replace, Stop, close); About with the program's own components and crate list | thin |

The library (`videolore`, no features) supplies `ffmpeg::locate`, the `Runner` trait, `base::io`, `ui::icons` and `notices`.

## Start checks
- **On the window's thread (quick; macOS asks for the screen there):** machine blocked (native Wayland, no display, macOS below 14.6: Start is refused for every source, see Platforms), file name and folder, "Replace the file?" (`-y` only after Yes, otherwise `-n`), the chosen screen still present, the screen recording permission.
- **On a thread of their own (`Phase::Starting`, polled by `tick`):** ffmpeg devices and encoders (the result is kept after a successful start), then the sound sources (each opened once).
- Meanwhile the choices are locked, no capture runs, the button reads **Cancel start**, and closing the window quits at once. A cancelled start discards its result (an opened sound is closed) and the screen is checked again before the recording child starts.

## Platforms
| System | Picture | Computer sound | Microphone | Notes |
|---|---|---|---|---|
| Windows x86_64 | gdigrab, screen offsets | cpal loopback of the default output (WASAPI) | cpal default input | DRM-protected windows may show black (ffmpeg gdigrab) |
| macOS Apple Silicon | avfoundation, screen by name | own CoreAudio tap in a private device, macOS 14.6 or later | cpal default input, permission asked first | screen recording permission (asked at Start; macOS applies it after a restart); older Macs (below 14.6) get a message and no recording at Start |
| Linux x86_64 | x11grab (X11 sessions and XWayland) | ffmpeg pulse, monitor of the default output | ffmpeg pulse, default source | native Wayland session refused with a message before Start (REQ-358); a default source that is a monitor is refused |

The recorder's installer declares macOS 11.0 like the editor and the player, so older Macs can open it and read the message (REQ-357). The CoreAudio symbols are linked weakly (`.cargo/config.toml`, `-weak_framework CoreAudio`), so the program starts on older macOS; CI checks the built programs with `nm -m` that the process-tap symbols are weak imports.

## Permissions and messages
- Before Start: a native Wayland session is refused (REQ-358); the chosen screen exists; on macOS the screen recording permission is allowed (`CGPreflightScreenCaptureAccess`; when it is missing, `CGRequestScreenCaptureAccess` asks macOS to show its prompt, once); ffmpeg has the devices and encoders; on macOS 14.6 or later the microphone and the computer sound open. A failure names the item (for example "Computer sound needs macOS 14.6 or later", "Screen recording is not allowed for this program").
- A recording that stops by itself keeps the file and names the cause (disk full, screen removed, capture failed).
- Windows and Linux ask nothing in advance; macOS asks for screen recording and microphone, and for the system audio when the tap is first made.

## Security
- Arguments are numbers, closed lists, the chosen folder and file name, and the screen geometry; no text from a file or the system reaches ffmpeg. No shell. Protocols `file`, `pipe` and `tcp` only. The program spawns ffmpeg only in `process.rs`, at most one capture child at a time.
- The loopback TCP port accepts only 127.0.0.1 peers and one connection per source; PCM is never stored.
- The file is written to the path the user chose; an existing file is replaced only after the Replace popup.
- Settings: a regular file, at most 64 KiB, never next to a recording (corpus test as the player's).
- Unsafe code: the library denies it; the recorder's `lib.rs` denies it too and allows it only in `macos.rs` (the AVFoundation microphone calls, the CoreAudio tap and private device, and the release of their identifiers; each block has a SAFETY comment).
- No network code (CI `cargo tree` check), no telemetry. The recording contains what the screen showed and is kept only where the user saved it.

## Performance
- Targets in [NFR](architecture-nfr.md#screen-recorder-312). Not measured yet: real-time capture at 1080p30 on a four-core machine, 4K30 and 1080p60 frame drops, start to first frame, the audio and video offset on Windows and macOS.
- Built and gated by tests: one ffmpeg child at a time; the preview reader keeps only the newest frame and never blocks the recording; stop finishes within 15 seconds, otherwise the child is killed; a killed recording is playable (Linux capture test and `tests/ffmpeg.rs`).

## Tests
- Pure unit tests (`src/tests.rs`, `src/app_tests.rs`, `src/ui_tests.rs`) for all three systems on the Linux runner: argument lists, formats, container rule, file names, settings corpus, session and macOS version decisions, microphone question, screen labels, device parsing, PCM conversion, silence filler, feeder over a real socket, state machine with fakes and an injected clock, accessibility tree.
- `tests/ffmpeg.rs`: the recorder's own command run by real ffmpeg with the screen input replaced by `testsrc2`, real feeders (AAC, MP3, WAV, killed recordings). `the_recorder_records_a_real_x11_screen_with_pulseaudio_sound` runs in CI under Xvfb with a private PulseAudio server and needs `VIDEOLORE_REQUIRE_CAPTURE=1`; it must never use a desktop's own sound server.
- Windows and macOS adapters (gdigrab, avfoundation, cpal, CoreAudio tap, permissions) have no automated test; they are checked by hand (business verification items 38 and 39).

## Not verified
- **Windows:** nothing run on Windows. Unverified: gdigrab offsets and DPI scaling, WASAPI loopback, the cpal microphone, the NSIS installer, the bundled ffmpeg's devices.
- **macOS:** nothing run on macOS. Unverified: the avfoundation screen name, the screen recording and microphone prompts, the system audio permission and the tap (until it is given the computer's sound may be silent), the private device, the weak CoreAudio link at launch on macOS 11 to 14.5, the disk image, the Info.plist texts.
- **Both:** the audio and video offset at the start, the real-time figures and the installer sizes are not measured.
- **Linux:** verified on a local X11 session under Xvfb with a private PulseAudio server and in the CI step (the CI YAML was reproduced locally, not run on a runner). No native Wayland session is recorded (refused by design).

## Risks
- Audio and video clocks of Windows and macOS start at different moments: a few hundred milliseconds may differ (unmeasured).
- Preview switches to the recording child at Start (about 1 s without picture).
- Platform code outside the Linux runner is covered by manual checks only.
- Three products now bundle ffmpeg: the download grows by about 100 MB per system.
- Software encoding cost: 1080p60 and 4K may drop frames on weak computers (libx264 veryfast); hardware encoders are a later candidate.
- A link to a newer macOS symbol would stop all three programs on older Macs; the weak link and the CI `nm -m` check guard it, the launch test on macOS 11 to 14.5 is still to be done by hand.
