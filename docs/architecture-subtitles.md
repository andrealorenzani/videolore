# Architecture: subtitles and speech-to-text

Part of [Architecture](architecture.md).

## Model
- Subtitle lines are stored against the **source media**, not the timeline: each line has the media it belongs to, start and end in that file, the text, optional word timings and an "edited by hand" flag.
- What is shown is computed from the timeline (REQ-018): for each video clip, the lines of its source that fall inside the clip's kept range are shown, shifted to the clip's position. So cutting a clip removes its subtitles, copying duplicates them, moving moves them, trimming hides those outside the range (extending the trim shows them again) and splitting keeps them.
- Editing (REQ-016, all undoable): add a line at the playhead (2 s, inside the clip under the playhead), delete, change text, change timing (converted through the clip it is shown in and kept inside it).
- A line used in two copies of the same clip is the same speech: changing its text changes both.
- Changing a line's text drops its word timings; the word-highlight style then spreads the highlight evenly over the words.
- One language and one style per project. Since 1.2.0 each line can also have its own position on screen (see [Position on screen](#position-on-screen-120)).

## Styles and burn-in
Three styles (REQ-017):
1. **Classic:** white text with a black outline, at the bottom.
2. **Box:** white text on a semi-transparent box, in the lower third.
3. **Word highlight:** bold white text, the current word in yellow, about 4 words at a time, centre-low.

- Subtitles are burned into the picture by `ffmpeg`'s `subtitles` filter (libass) from a subtitle file in ASS format written by `videolore`.
- Text size = the smaller of 5.5% of the video height and 9% of its width, so it reads well both in 9:16 and in landscape; margins scale the same way.
- The subtitle file and the bundled font are written to a private temporary folder, and `ffmpeg` runs inside that folder, so no file path has to be written into the filter (avoids escaping problems with Windows paths and quotes).
- Preview uses the same file at export resolution, scaled down, so preview and export match.
- Fonts: Noto Sans Bold, Latin/Greek/Cyrillic only; see the limitation in [ffmpeg](architecture-ffmpeg.md#subtitle-rendering-and-fonts).
- User text cannot inject styling: `{`, `}` and `\` are replaced with look-alike characters, and only `videolore` writes line breaks and highlight tags. Text is limited to 1000 characters per line.

## Speech-to-text
- **Engine:** whisper.cpp through the `whisper-rs` crate, running on the CPU, fully offline (REQ-014). Alternatives rejected: cloud services (privacy, cost), Vosk (quality), candle-whisper (speed and maturity).
- **Included in release builds** (build feature `stt`, on by default). Building it from source needs `cmake`, a C++ compiler and libclang. A build with `--no-default-features` leaves it out: "Generate subtitles" then says it is not available in this build, while editing, styles and burn-in still work.
- **What is transcribed:** see [Creating subtitles](#creating-subtitles). Only the video's own audio, never the music.
- **Steps:**
  1. The processor is checked (see [CPU requirements](#cpu-requirements-since-021)), and the model of the chosen [quality level](#subtitle-quality-levels-030) must be present (otherwise the consent dialog opens).
  2. `ffmpeg` extracts the audio as 16 kHz mono to a file in `videolore`'s private work folder (one background slot of the job queue).
  3. Transcription reads it in 120 s chunks (bounded memory; small artefacts at chunk boundaries are accepted).
  4. Word timings are grouped into lines at pauses longer than 0.6 s or at the style's word limit.
- **Language (REQ-015):** detected on the first chunk and saved in the project; the user can override it, which re-runs transcription in that language.
- **Background, progress, cancel:** progress is per chunk; cancel takes effect during transcription.
- **Threads:** set by "How hard videolore works" (since 0.3.0; based on physical cores, see [performance](architecture-performance.md#how-hard-videolore-works)); one transcription at a time. It does not count toward the `ffmpeg` process limit.
- **Speed (0.3.0):** flash attention was measured and is off at every level (it made subtitles slower); one decoding state is reused for the whole job instead of one per chunk; decoding stays greedy at every level (beam search would make "Most accurate" 2–3 times slower).
- Changing the quality level never regenerates by itself.

## Creating subtitles
Since 1.2.0 Subtitles is applied like the other effects (see [effects](architecture-effects.md#applying-an-effect)):
- **On a clip:** only the part of the source that the clip uses is processed (the audio is extracted with a start time and a length); lines of that source outside this part are left untouched. Lines of that source overlapping the part are replaced by the new ones, which are kept inside it.
- **On the whole video:** each source used on the timeline is processed in full, as before (so lengthening a trim later reveals more subtitles), and all lines are replaced.
- If a source involved contains a text subtitle track, the user chooses between its text and speech recognition; see [subtitle tracks](architecture-subtitle-tracks.md).
- If lines in the processed part were edited by hand, the user is asked first (REQ-044); lines elsewhere do not count.
- One subtitle job at a time; while it runs, starting another is greyed with "Subtitles are being created". Progress shows "Creating subtitles", or "Matching subtitles to the speech" when a track's text is used.
- The result is applied as **one undo step** only when the whole job has finished. Cancelling, at any point, changes nothing.

## Position on screen (1.2.0)
- Each line has an optional position: the centre of its text box, stored as a fraction of the picture (in 1/10000 steps), so it survives a change of output format. Without a position the line sits where its style puts it, exactly as in 1.1.
- The position belongs to the line, so copies of the same clip share it (they are the same speech).
- **Burn-in:** a positioned line gets a centre-anchor and position tag at the start of its ASS line, computed from the stored fraction at the export resolution. Only `videolore` writes these tags, from checked numbers. Preview and export use the same subtitle file, so the text appears in the same place (an automated test with `ffmpeg` checks the centre of the burned text is within 2% of the frame height of the requested place).
- **Dragging in the preview:** `videolore` computes the boxes of the lines visible at the playhead with egui's text layout and the same bundled font (Noto Sans Bold) at the same size and wrap width as the export. The box is kept fully inside the picture. While dragging, an outline of the text follows the pointer at once and the preview frame is re-rendered in the background (only the latest request); nothing is saved during the drag. Releasing is one undo step. "Reset position" returns to the style's place.
- **Selecting several lines:** Ctrl+click (Cmd+click on macOS) on a line marker or in the Subtitles popup list adds or removes a line; a plain click selects only that line. Shift+drag on the timeline selects every line whose start is in the dragged range (with Ctrl/Cmd too it adds to the selection), and moves the playhead to the first one. Clicking empty timeline clears the selection; Esc closes the topmost popup first, then clears it.
- **Moving a group:** dragging one of the selected lines moves them all to exactly the same position (their previous relative positions are not kept), as one undo step. Only the dragged line's box is kept inside the picture. Reset position on a selection resets all of them.
- The selection is part of the window state, not of the project or the undo history.
- **Known limits:** egui and the burn-in library may measure text a few pixels differently near the edges (the burned result is what counts); a line made longer after being placed near an edge may then cross it; positioned lines shown at the same time can overlap.

## CPU requirements (since 0.2.1)
- whisper.cpp is compiled for a **fixed instruction-set baseline**, not for the machine that builds it. This is set once in `.cargo/config.toml` (`GGML_NATIVE=OFF`), which the `whisper-rs` build passes on to whisper.cpp's build.
- **Why:** before 0.2.1 whisper.cpp was optimised for the CPU of the build machine. Release executables are built on GitHub's servers, whose CPUs can have instructions (such as AVX-512) that many users' CPUs lack; generating subtitles could then crash on those computers.
- **Minimum CPU for generating subtitles on x86-64 (Windows, Linux):** AVX2, FMA, F16C and BMI2, i.e. roughly Intel processors from 2013 (Haswell) on and AMD processors from 2015 (Excavator, all Ryzen) on. AVX-512 is never used.
- **macOS (Apple Silicon):** unchanged; every Apple Silicon Mac is supported.
- **Older x86-64 CPUs (since 0.3.0):** at start-up `videolore` checks for SSE4.2, AVX, AVX2, FMA, F16C and BMI2. If any is missing, "Generate subtitles" shows "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand.", no download is offered, the three quality levels are greyed out with the same reason, and whisper.cpp code is never run. Everything else works normally. (In 0.2.1 and 0.2.2 subtitle generation was expected to crash on these computers.)
- **No slower engine for older CPUs in 0.3.0.** The program contains a single copy of whisper.cpp, so a second copy built for older CPUs would clash with it, and picking the best code at run time would need whisper.cpp as separate loadable libraries, which the `whisper-rs` build does not support. The only workable route would be a separate helper program built for older CPUs (about 7 times slower). Andrea decided on 2026-11-14 not to build it: it is out of scope and would be reconsidered only on request.
- **Assumption:** whisper.cpp runs no newer-CPU instructions while the program starts, so the check is reached. An optional CI step running the tests under an emulated older CPU may confirm it.
- Building from source uses the same baseline. Someone building only for their own computer can set `GGML_NATIVE=ON` in the environment.
- CI's build cache key was changed so that whisper.cpp builds made the old way are not reused.

## Subtitle quality levels (0.3.0)
"Subtitle quality" in Settings picks one of three whisper models, all multilingual, quantised, MIT licence, from a fixed revision of `huggingface.co/ggerganov/whisper.cpp`:

| Level | File | Size | SHA-256 |
|---|---|---|---|
| Fast (the 0.2.x model) | `ggml-tiny-q5_1.bin` | 32,152,673 bytes (32 MB) | `818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7` |
| Balanced | `ggml-small-q5_1.bin` | 190,085,487 bytes (190 MB) | `ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb` |
| Most accurate | `ggml-large-v3-turbo-q5_0.bin` | 574,041,195 bytes (574 MB) | `394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2` |

- **Estimated cost (assumption, to be measured):** compared with Fast, Balanced is about 6–8 times slower and Most accurate about 20–30 times slower; with 8 threads on a modern x86 processor that is roughly 30x, 5x and 1–2x faster than real time. Extra memory while transcribing: about 0.5 GB (Balanced) and 1.3 GB (Most accurate).
- **Why these models:** "base" (60 MB) gains too little over "tiny"; "medium" (539 MB) is slower and less accurate than "large-v3-turbo"; 8-bit versions are about 40% bigger for little gain; unquantised ones are 3–6 times bigger; English-only versions lose other languages.
- Which level is recommended, and when a level is unavailable, is described in [performance](architecture-performance.md#recommended-settings).
- An automated test (run in CI with the three models cached) checks that Balanced and Most accurate make no more word errors than Fast on a synthetic reference sentence.

## Speech model download
- Models are downloaded only when subtitles are generated with a level whose model is missing, never in advance and never by "Use recommended settings".
- **Consent first:** before any network access a dialog shows the level, size, licence and source. Choices:
  - **Download**;
  - **Use Fast instead (no download)**, only if the Fast model is already present: for this run only, the setting is unchanged;
  - **Download the Fast one instead (32 MB)**, only if no model is present yet;
  - **Choose a file on this computer…**: it must have that level's exact size and SHA-256 and is copied into the data folder;
  - **Cancel**: nothing changes.
- Downloaded over HTTPS from the fixed address of that level only, into a partial file, limited to the expected size, checked for SHA-256 and then moved into the app's data folder (`…/videolore/models/`). An interrupted download, a wrong checksum or a full disk deletes the partial file ("Not enough free disk space" in the last case). After that, everything is offline.
- Before each use the file size is checked, and the SHA-256 once per session.
- The download uses the `ureq` HTTP client with `rustls` (no system OpenSSL), honours `HTTPS_PROXY`, has timeouts, follows at most 5 redirects and only to HTTPS. Non-HTTPS addresses are refused before connecting. No other part of the program uses the network.
- **Storage:** Settings shows the space used by downloaded speech files (the sum of the known model files present) and offers to remove them after confirmation. Only the known model files and their partial downloads are deleted; subtitles already in projects are not affected, and a model is downloaded again (with consent) when next needed.

## Tests
- Transcription and download are behind small interfaces with test doubles, so the whole flow (chunking, language, progress, cancel, consent per level, "Use Fast instead", checksum mismatch, interrupted download, non-HTTPS refusal, storage and removal, old-processor message, regenerate confirmation) is tested without the real models. Since 1.2.0 also: only the clip's part reaches `ffmpeg` and the recogniser, lines of other clips are unchanged, positions in the ASS file, selection and group moves.
- One integration test runs the real model on a short synthetic voice clip (made with `espeak-ng`) and checks that "hello" is recognised with increasing word times. It runs in CI with the model cached.

## Risks
- The C++ build of whisper.cpp on three platforms slows CI and can break on toolchain changes.
- Scripts other than Latin, Greek and Cyrillic depend on system fonts.
- On x86-64 computers without AVX2 (before about 2013) automatic subtitles are not available (message, no crash; see [CPU requirements](#cpu-requirements-since-021)).
- Many users will be asked to approve a 190 MB or 574 MB download on their first subtitle generation; the 32 MB Fast option is offered in the same dialog.
- Model speed and memory figures are estimates until measured.
- If the build environment ever sets `SOURCE_DATE_EPOCH`, whisper.cpp's build silently falls back to the slow SSE-only baseline; it must stay unset (or AVX2, FMA, F16C and BMI2 must be enabled explicitly).
