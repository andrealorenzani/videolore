# Architecture: main technical decisions

Part of [Architecture](architecture.md). One line per decision: the choice and its reason, with the record id (ADR-NNN; the full records with context and alternatives are kept with the project's architecture notes). Details: the linked area documents.

## Platform, language, build
- **GUI eframe/egui 0.37** (ADR-001): pure Rust, one executable, three platforms (iced, slint, gtk4-rs, tauri considered).
- **Latest stable Rust, `rust-version` 1.100** (ADR-012; supersedes the MSRV reasoning of ADR-001).
- **Std threads and channels, no async runtime** (ADR-004); the UI thread drains every channel once per frame.
- **Support crates** serde, serde_json, thiserror, directories, rfd, tempfile (ADR-005); later crates in [dependencies](architecture-dependencies.md).
- **Features `audio` and `stt`, both default** (ADR-022); the minimal build has no sound and no speech.
- **Workspace: editor as root, player as member** (ADR-105): the player uses the library without speech, so its network-free build is a CI fact. Versions written and checked in both manifests (ADR-116).
- **CI on fixed runners with SHA-pinned actions** (ADR-036); CI jobs (ADR-011) and test speed (ADR-035).
- **Licence GPL-3.0-only**; supply chain: cargo-deny licence list, cargo-about notices, pinned archives (ADR-025, ADR-026).

## Media engine and export
- **ffmpeg and ffprobe as separate programs, never linked** (ADR-002); **bundled since 0.2.0** (ADR-013: GPL builds with x264 and x265; LGPL rejected, no H.264 or HEVC encoder).
- **Preview audio through cpal;** without a device the preview is silent and wall-clocked (ADR-003; clock part superseded by ADR-016).
- **Job queue with a process cap:** 3 for Light and Balanced, 3 to 6 for Maximum; priorities preview, export, import, background (ADR-015, ADR-029).
- **Preview: latest-only still mailbox, FrameKey, wall-clock master, audio correction** (ADR-016); one render per burst, 80 ms quiet (REQ-034).
- **Stream copy only when safe, otherwise one re-encode; temporary file then rename** (ADR-006); encoder lists with fallbacks, software first (ADR-007; hardware opt-in ADR-037).
- **Monotone input runs; hvc1 tag rule** (ADR-017; REQ-036, REQ-037).
- **Proxies by resolution, cores and memory; cache key by source and settings** (ADR-018); 360 copy 4096x2048 (1.2.6).
- **Frame graph:** layers fitted without pad, background, overlay, fades after composition (ADR-098); numbers only in expressions, script file when long (ADR-099); colour `#RRGGBB` to `0xRRGGBB` (ADR-100).
- **Music:** amix, volume, afade; gain (p/100)^2; limiter (ADR-010, ADR-019).
- **Export dialog as a window with session-only choices and a fixed format table** (ADR-082, ADR-083, ADR-084): every output uses the one export path.

## Hardware export (0.4.0 to 2.0.4)
- **Opt-in "Faster"; "Best quality" default everywhere** (REQ-055). H.264 and HEVC only; vendor arguments incl. the VA-API CQ deviation (ADR-037); hardware probe with 1 s, 4 s and 10 s limits (ADR-038).
- **Hardware first, output check, silent software redo** on any problem; 30 s stall watchdog (ADR-039); failure memory per format and method (ADR-040).
- **Output check:** file details, strict full decode, three position-matched samples at 25 dB PSNR per plane (ADR-039; 2.0.4 ADR-177: samples only, paused during jobs).
- **Codec presets AV1 and VP9; preview hardware decoding; graphics card Automatic or a validated device name** (ADR-178, ADR-179).

## Speech and subtitles
- **Local whisper.cpp through whisper-rs** (ADR-009, ADR-021; cloud and Vosk rejected; ADR-009 superseded in parts by ADR-022 and ADR-030).
- **Three quality levels = three models, each downloaded once after consent** (ADR-030).
- **CPU baseline GGML_NATIVE=OFF (AVX2, FMA, F16C)** (ADR-027), amended by ADR-031: processor check and message, no slower engine.
- **Lines against the source media, shown through the timeline** (ADR-020); creation per range, all-or-nothing, one undo (ADR-055).
- **Word alignment per cue by Needleman-Wunsch, in-house, deterministic** (ADR-056); sound onset correction (2.0.3).
- **Per-line ASS position and size** (ADR-057); ASS escaping and presets (ADR-020).
- **Embedded tracks through ffprobe and ffmpeg SRT extraction; in-house SRT** (ADR-053, ADR-054); **separate track mov_text or srt, never default** (ADR-171).
- **Accuracy (REQ-177): speech regions and a frozen setting** (ADR-220); built, NOT MET (43 % against 90 %).

## Project, settings and storage
- **Versioned JSON; newer formats refused and never written; strict validation** (ADR-023; format steps ADR-051 v3, ADR-076 v5, ADR-104 v7, ADR-169 v11, ADR-222 v13, ADR-230 v14).
- **The current version is always written** (1.2.0): no writers for old formats; older programs refuse newer files.
- **Unsaved changes by comparison with the saved edit** (ADR-023); quit path with macOS default menu off (ADR-024).
- **Markers computed from the edit, so the format does not change** (1.1.0; ADR-048).
- **Settings version 2 with additive optional fields** (ADR-044, ADR-195).
- **Machine detection at each start and recommended settings; no installer tuning** (ADR-028; levels ADR-029).
- **Private work folders (0700), lock file, crash clean-up** (ADR-033); autosave copies in the cache, never the project (ADR-173); one window by lock file and inbox (ADR-182); preview copy cache by hashed names, the open project never removed (ADR-198).
- **Per-clip data:** clip keys in source time, whole-video keys in timeline time (ADR-094); frame storage (ADR-093); edit rules: split and copy keep keys, merge needs equal keys, remove drops, one undo (ADR-101).
- **Whole-video effects as copies on the clips** (1.6.1, format 9; REQ-083, 147, 148); the video's Change aspect stays one setting.
- **Format 13 for the order of all rows** (ADR-222; 12 read with defaults); **format 14 for overlaps, 13 converted in memory** (ADR-230, ADR-241).

## Effects, frame and picture
- **Fades 0.5 s or half the clip; edge fades belong to the clip from 1.6.1** (ADR-008; ADR-049 superseded by ADR-123).
- **Clip volume per clip, multiplied by the video level; 100 % adds nothing to the command** (ADR-050).
- **Clean audio with ffmpeg `afftdn`, no model file** (1.4.0, ADR-080, ADR-081; neural and non-local filters rejected for cost and download). **Enhance speech with `speechnorm` named "Normalise speech"** (2.0.4); `arnndn` not used (licence not checked).
- **Frame size capped at 4096 px on the longer side** (ADR-097; decided 2026-11-19; 1920 px and no cap rejected).
- **Keyframes: at most 200 per effect; Scale 100 to 4000 permille; Move 0 to 10000; hold and linear** (ADR-095); Scale pivot as a hidden offset (ADR-096).
- **Blurred fill at 1/8 of each side** (ADR-180; measured; REQ-222 names 1/4, a product decision open); blur levels and custom ratios (ADR-193); the still's blurred background as one raw RGBA file (ADR-180).
- **Effects by drag onto a clip or double-click for the whole video, one command** (ADR-052, ADR-090).
- **Text items through the subtitle ASS path, one engine** (ADR-066); effects as ASS events with caps (ADR-067); installed fonts via fontdb 0.24 (ADR-068).
- **Best-fit text size: one pure estimate for preview and export** (1.4.0, ADR-087); by glyph widths from 2.0.3. Migration to the box: choice A (ADR-088).
- **LUT: pure `.cube` reader, the path stored, `lut3d` in the picture branch** (ADR-109, ADR-110); missing-file rule (ADR-111; ADR-112 superseded by the three-button modal of ADR-191).
- **LUT strength blended with the identity; LUT copies up to 20 MB and 64 MiB per project; read cap 96 MiB** (ADR-191).
- **LUT sizes 2 to 256, streamed, one Work copy per content** (2.1.0; ADR-202 to 213); **background preparation with percentage and the picture unprocessed until ready** (2.2.0, D63, route D, ADR-214).
- **Applied effects order:** legacy order for old projects; rows move inside their groups; the default order is the 2.1.0 build (ADR-223 to 225, ADR-229); new rows keep their default place (deviation from REQ-321).
- **Overlap media bound to its clip; warning before removal; once then transparent; 16 MP limit** (ADR-230 to 241; REQ-325 to 331).
- **Enlarge word drawn alone at its measured centre** (ADR-235; the REQ-333 fix of REQ-309); Typewriter one event per word (REQ-310).
- **Start screen option** (ADR-199; ADR-240; stored "Never" reads as "Once for each version").
- **Clip speed: one time mapping (`Clip::len`, `src_len`, `to_timeline`, `to_src`); `setpts` and `atempo` 0.5 to 2.0; never stream copied; different speeds do not merge** (2.0.4, REQ-281).
- **Picture items are text items with an optional picture** (2.0.4, REQ-287; a separate picture list rejected); the picture limit of 16 megapixels (ADR-219, REQ-316); the plan's count and size limits (ADR-192) are open.

## 360 video
- **360 through the `v360` filter for preview and export, one engine** (ADR-059); own reprojection and GPU shader rejected.
- **Detection from spherical metadata; the switch removed in 1.4.0** (ADR-060, ADR-086).
- **POVs per source in source time; the live view is not saved** (ADR-061, ADR-062).
- **Absolute sendcmd commands, only changed values, half-frame intervals** (ADR-059 as built; at most 10 updates per second, REQ-266).
- **360 export geometry: fill the frame, d_fov, the 4096 proxy rule, a too-large message** (ADR-063).
- **Anchored items burned into the sphere frame before v360; flat place on flat clips** (ADR-069, ADR-077, ADR-078). Anchored subtitle lines were plain white (DEV-3, ADR-069 superseded by ADR-188 in 2.0.4: preset outline or box and word highlight).
- **Spherical marker in-house for MP4 and MOV (ADR-078); in place for MKV and WebM (ADR-187, 2.0.4).**
- **Export choice 360 video or flat camera view** (ADR-085, 1.4.0 rule); 360 video ignores Change aspect, Scale and Move (ADR-102).
- **Per-clip camera paths by copy on write** (2.0.4, REQ-268), not a copy on load (rejected: copies diverge and the file grows).
- **Spatial sound: first-order ambisonics in Rust, a fixed cardioid pair, a 32-bit float WAV pre-pass; no time-varying ffmpeg pan** (ADR-189); channel order W, Y, Z, X assumed.

## User interface
- **Own proportional layout instead of egui panels** (ADR-041).
- **One action table for menus, toolbar, tooltips and shortcuts** (ADR-042).
- **Popup and instance state in AppState, driven by commands; non-modal popups** (ADR-043).
- **Acceptance tests through the AccessKit tree** (ADR-045); egui keyboard zoom off (ADR-046); drag and drop through winit, not on Linux Wayland (ADR-047).
- **One selection value; a context group rule over selection and playhead** (ADR-073, ADR-074).
- **Boxes and handles on the picture; the box layer takes any list of boxes** (ADR-064); hit priority (ADR-103).
- **"Fade IN/OUT" label, internal name unchanged** (ADR-075); the text width handle on the right edge only (ADR-091).
- **Icons from one vendored MIT font (Phosphor), not a crate** (ADR-114).
- **Splash as an in-app window with assets compiled in** (ADR-113; partly superseded by ADR-128 to 131).
- **Notices in About from the packaging notice files** (ADR-115, ADR-026).
- **Remembered places, sections, workspaces, themes, AccessKit names** (2.0.4, REQ-234 to 247, ADR-197).
- **Menus inside the window on every system** (1.1.0, macOS included).

## Packaging, platform and security
- **Installers unsigned; packages per OS (NSIS, dmg, AppImage, tar.gz)** (ADR-014; signing postponed by decision of Andrea).
- **Linux AppImage assembled by hand with a pinned appimagetool; no `.deb`** (decided 2026-11-20).
- **Player installers with their own identifiers; Windows "Open with" template only** (ADR-108, ADR-184; the default program stays the user's choice).
- **Installer check as a reusable workflow on packaging pull requests; weekly advisories read-only** (ADR-181).
- **Security hardening: no shell, `file:` prefix, protocol whitelist, bounded reads, FIFO refusal, checked downloads** (ADR-034); unsafe policy with three reviewed sites (ADR-183 amends).
- **Menu bar by one allow-listed objc2 call; no menu crate** (ADR-183).
- **Speech downloads with consent, HTTPS only, SHA-256** (ADR-021, ADR-030); no telemetry, no update check.

## Structure and testing
- **Shared helpers in one `base` module; no module cycles, checked by a test** (2.0.1; ADR-136 to 141, ADR-140 the test). Alternatives rejected: a separate crate; re-export shims.
- **Split before features: `render` and `app` by area, no behaviour change** (ADR-168, 2.0.4).
- **One format bump per development** (format 11 for 2.0.4 by D58, 12, 13, 14).
- **Golden argv tests: projects without new features produce the same command** (QG-15).
- **Tests use the bundled pinned ffmpeg; `VIDEOLORE_REQUIRE_FFMPEG` and `VIDEOLORE_REQUIRE_STT` make missing dependencies fail** (2.0.2, 2.0.3).
- **Timing tests use the best of up to five samples; product limits unchanged** (QG-09).
- **Speech fixture committed and checked by SHA-256; espeak-ng removed from CI** (2.0.4, QG-14).

## Planned for 3.1.0 (ADR-244 to 254)
- **Format 15 on every save, refused by older programs (ADR-244; Andrea chose this for Q-F1: every save writes 15, so 3.0.x cannot open 3.1.0 saves).**
- **An audio track is its ordinal among the file's audio streams; a clip stores it (ADR-245).**
- **A non-first audio track is an effect for export: re-encode, no stream copy (ADR-246).**
- **Proxies keep all audio streams; the preview restarts the sound on a track change (ADR-247).**
- **Enlarge word grows from its base, in a pure `word_layout` module (ADR-248; replaces REQ-333's centre rule).**
- **"As subtitle track" writes the lines as a switchable track and burns none; the editor preview still draws them (ADR-249).**
- **The blurred fill of a clip's Change aspect uses the same chain as the whole video (ADR-250).**
- **Overlap media has its own Effects row (ADR-251).**
- **The player's Audio menu sets the in-memory clip's track; nothing is saved (ADR-252).**
- **The Subtitles popup's Audio track is a command argument, not stored (ADR-253).**
- **Volume and track: the track is a clip field read by preview, export, merge and the 360 pre-pass (ADR-254).**

## Open choices
Listed in [risks](architecture-risks.md#open-product-decisions): the blur divisor, the picture item limits, the Faster target, REQ-177, the boxed shadow design, the default 360 choice.
