# Architecture: subtitles and speech-to-text

Part of [Architecture](architecture.md).

## Model
- Subtitle lines are stored against the **source media**, not the timeline: each line has the media it belongs to, start and end in that file, the text, optional word timings and an "edited by hand" flag.
- What is shown is computed from the timeline (REQ-018): for each video clip, the lines of its source that fall inside the clip's kept range are shown, shifted to the clip's position. So cutting a clip removes its subtitles, copying duplicates them, moving moves them, trimming hides those outside the range (extending the trim shows them again) and splitting keeps them.
- Editing (REQ-016, all undoable): add a line at the playhead (2 s, inside the clip under the playhead), delete, change text, change timing (converted through the clip it is shown in and kept inside it).
- A line used in two copies of the same clip is the same speech: changing its text changes both.
- Changing a line's text drops its word timings; the word-highlight style then spreads the highlight evenly over the words.
- One language and one style per project.

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
- **What is transcribed:** for each source used on the timeline, its whole audio track (so lengthening a trim reveals more subtitles). Only the video's own audio, never the music.
- **Steps:**
  1. `ffmpeg` extracts the audio as 16 kHz mono to a private temporary file (one background slot of the job queue).
  2. Transcription reads it in 120 s chunks (bounded memory; small artefacts at chunk boundaries are accepted).
  3. Word timings are grouped into lines at pauses longer than 0.6 s or at the style's word limit.
- **Language (REQ-015):** detected on the first chunk and saved in the project; the user can override it, which re-runs transcription in that language.
- **Background, progress, cancel:** progress is per chunk; cancel takes effect during transcription.
- **Threads:** half the CPU cores, between 1 and 4; one transcription at a time. It does not count toward the 3-process `ffmpeg` limit.
- **Regenerating** replaces all lines; if any line was edited by hand, the user is asked first (REQ-044).

## Speech model
- Model: whisper "tiny" multilingual, quantised (`ggml-tiny-q5_1.bin`), 32 MB, MIT licence.
- Source: a fixed revision on Hugging Face (`huggingface.co/ggerganov/whisper.cpp`), with exact size (32,152,673 bytes) and SHA-256 `818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7`.
- **Consent first:** before any network access a dialog shows the size, licence, source and that this is the product's only network access. Without consent nothing is downloaded.
- Downloaded over HTTPS to a temporary file, checked for exact size and SHA-256, then moved into the app's data folder (`…/videolore/models/`). After that, everything is offline.
- Alternatively the user can pick a local copy of the same file; it must have the same size and SHA-256 and is copied into the data folder. Choosing a different model is out of scope.
- The download uses the `ureq` HTTP client with `rustls` (no system OpenSSL), honours `HTTPS_PROXY`, has timeouts, and follows redirects only to HTTPS. No other part of the program uses the network.

## Tests
- Transcription and download are behind small interfaces with test doubles, so the whole flow (chunking, language, progress, cancel, consent, checksum mismatch, regenerate confirmation) is tested without the real model.
- One integration test runs the real model on a short synthetic voice clip (made with `espeak-ng`) and checks that "hello" is recognised with increasing word times. It runs in CI with the model cached.

## Risks
- The C++ build of whisper.cpp on three platforms slows CI and can break on toolchain changes.
- Scripts other than Latin, Greek and Cyrillic depend on system fonts.
