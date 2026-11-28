# Code — videolore screen spy and the player's playlist (3.1.2)

Back to the [index](code.md). Requirements REQ-341 to 352, 357, 358 ([business-screen-spy.md](business-screen-spy.md)) and REQ-355, 356 ([business-player.md](business-player.md)). Design: [architecture-screen-spy.md](architecture-screen-spy.md).

## The program
`screen-spy/` is the third workspace member: package and binary `videolore-screen-spy`, a library plus a thin `main.rs`. It depends on `videolore` with **no features** (no sound output, no speech engine, no network code; CI checks the graph with `cargo tree --target all`) and uses its `ffmpeg::locate` and `Runner`, `base::io`, `ui::icons` and `notices::components`. Its own crates: `display-info` (screens), `jiff` (local date and time of the default name), and on Windows and macOS only `cpal` (the two sound sources); macOS also `objc2*` and `block2`.

## Modules of `screen-spy/src`
| File | What | Pure |
|---|---|---|
| `lib.rs` | `real_app`, `native_options`, ffmpeg lookup, save dialog, `local_stamp` | no |
| `app.rs` | `RecorderState`: `Command`, `Phase` (idle, previewing, starting, recording, stopping), `tick(now)`, start checks (`Checks` on their own thread), popups; everything injected through `Deps` | yes, with fakes |
| `plan.rs` | `Os`, `VideoFormat`, `AudioFormat`, `preview_args`, `record_args`, `filter_graph`, `preview_size`, file names | yes |
| `session.rs` | `Platform::detect`: Wayland and X display decision, the macOS 14.6 decision, the permission messages, `microphone_gate` | yes |
| `screens.rs` | "Screen N, WxH", `physical` (display-info's units to pixels), `SystemScreens` | mapping yes |
| `devices.rs` | readers of `ffmpeg -devices`, `-sources pulse`; what a recording needs; the avfoundation screen name | yes |
| `process.rs` | the only place that starts a capture: `Spawner`/`Child`, `q` to stop, `STOP_TIMEOUT` 15 s, `failure_cause` | no |
| `preview.rs` | `Feed`: a thread reads every raw picture and keeps the newest | yes |
| `audio.rs` | `Sound`: `PulseSound` (Linux), `DeviceSound` (cpal, Windows and macOS), `hold` (the thread that owns the streams), s16le stereo conversion, silence filler, `Feeder` (127.0.0.1 only, one connection) | all but the cpal streams |
| `macos.rs` | macOS version, screen recording permission, `Microphone` (status and question), `SystemTap` (the capture device of the computer's sound); the only `unsafe` code, each block with its SAFETY comment | no |
| `settings.rs` | `screen-spy.json` version 1; a damaged file is renamed `.bad` | yes |
| `ui.rs`, `about.rs` | the egui window and popups; the About text (own components and crate list) | thin |

## How it works
- **One capture at a time.** Before and between recordings a preview child runs (`-nostdin`, 5 fps). Start kills it, runs the checks and spawns the recording child, whose second output feeds the preview. It is the only ffmpeg that reads stdin (`q` closes the file); after 15 s it is killed.
- **Start checks, in order.** On the window's thread (quick; macOS wants screens and permission asked there): machine blocked (Wayland, no display, macOS below 14.6), file name and folder, "Replace the file?" (`-y` only then, otherwise `-n`), the chosen screen still there, the screen permission. Then on a thread of their own (`Phase::Starting`, polled by `tick`): ffmpeg devices and encoders (kept after a successful start), both sound sources. Meanwhile the choices are locked, no capture runs, the button reads **Cancel start**, and closing the window quits at once. A cancelled start throws the result away (an opened sound is closed) and the screen is checked again before the recording process starts.
- **Screens:** `x11grab` takes position and size in pixels, `gdigrab` offsets, `avfoundation` the device **name** `Capture screen N` (plain numbers count the cameras first). The list is read every 2 s; removal of the recorded screen stops the recording with that cause.
- **Sound:** Linux: two `-f pulse` inputs (`@DEFAULT_MONITOR@`, `@DEFAULT_SOURCE@`), each tried for 0.1 s before Start. When ffmpeg lists only monitors there is no microphone, and when the **default source is a monitor** Start is refused with a message (it would record the computer's sound twice); no source name is passed to ffmpeg. macOS: a microphone permission not asked yet is asked for and its answer awaited (up to 2 minutes, `microphone_gate`) before anything opens; the computer's sound is a tap of everything the computer plays (`initStereoGlobalTapButExcludeProcesses`) inside a private device, recorded by cpal as an ordinary input, because cpal's own loopback records the inputs of an output device that has some. Windows and macOS: each cpal stream is converted to 16 bit stereo and sent over its own TCP connection on 127.0.0.1; a source behind the clock by more than 200 ms is filled with silence. ffmpeg mixes with `amix` (`normalize=0`), reduces the steady noise of the mixed track (`afftdn`, `plan::NOISE_REDUCTION`), cuts off the filter's delay of 1200 samples (25 ms, `atrim`) so the sound stays with the picture, then limits (`alimiter`) (REQ-377, `plan::mix_filters`). On the synthetic fixture the quiet parts are 15 to 25 dB lower, speech-like bursts within 0.7 dB and a click moves by 1 sample (the same figures with ffmpeg 6.1.1 and the pinned 9.0.2); the end of a recording loses its last 25 ms of sound.
- **Picture and sound together:** the sound inputs come first and the screen last, so the file starts with the picture while the sound already runs. On Linux each `pulse` input has `-isync` to the screen (both stamp the wall clock): measured with a flash and a tone at the same moment, 17 to 39 ms apart (0.26 to 2.2 s without it). The `tcp` sources of Windows and macOS have no clock: their offset is the time the screen takes to open (not measured).
- **File:** `libx264 veryfast` CRF 23, a key frame every second, `-flush_packets 1`; AAC/MP3 in fragmented MP4, WAV in Matroska (`-cluster_time_limit 1000`): a killed recording keeps all but about the last second.
- **Older macOS:** `.cargo/config.toml` links CoreAudio weakly (`-weak_framework`), because cpal's loopback uses calls of macOS 14.2; CI checks with `nm -m` that no program imports them strongly. The recorder never calls them below 14.6.

## Tests
- `src/tests.rs` (argv of the three systems, tables, names, session and version decisions, the microphone question, the stream-owning thread, screens, devices, Pulse probing with scripted runs, conversion, feeder over a real socket, preview feed, failure causes, `cat` as a real child, settings corpus, About, packaging), `src/app_tests.rs` (state machine with fakes and an injected clock), `src/ui_tests.rs` (accessibility tree), a test in `audio.rs` (silence on a quiet or unplugged source).
- `tests/ffmpeg.rs`: the recorder's own command with real ffmpeg, the screen input replaced by `testsrc2`, two real feeders (AAC, MP3, WAV, killed recordings). `the_recorder_records_a_real_x11_screen_with_pulseaudio_sound` needs `VIDEOLORE_REQUIRE_CAPTURE=1`, `DISPLAY` and a PulseAudio server (CI step with Xvfb and two null sinks; **never** point it at a desktop's own sound server: it records the default microphone).

## Player: playlist (REQ-355, 356)
`Command::OpenVideos` (the answer of Open…, `pick_files`) replaces the list (`Playlist::replace`: once each, at most 10 000; projects left out) and plays the first video; `Command::AddToPlaylistPaths` appends without touching the playing one (a video opened by itself heads the list it starts). The title is `name (n of m) — videolore player` when the list has more than one video (`Playlist::place`). A dropped file or a file given on the command line still opens by itself.

## Known limits
- Not verified on real hardware: everything on Windows and macOS (gdigrab offsets and DPI scaling, avfoundation, WASAPI and CoreAudio loopback, permissions, launch on macOS 11 to 14.5, the installers).
- macOS: the system-audio permission is asked by macOS when the tap is first made; until it is given the computer's sound may be silent (not verified).
- Linux: when ffmpeg's list marks no default source, or cannot be read, the default source is only probed.
- Windows and macOS: the offset between picture and sound at the start is not measured. A system ffmpeg older than 6.0 (PATH fallback) does not know `-isync`; the bundled one does.
- Each PulseAudio source takes 0.2 to 2 s to open on the test server (5 s limit each), and the recording process opens the sources again: a few seconds pass between Start and the recording. A cancelled start waits for the source being tried.
- 1080p60 and 4K at `veryfast` may not keep real time (not measured).
