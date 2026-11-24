# Architecture: subtitles, speech-to-text and subtitle styles

Part of [Architecture](architecture.md). Requirements: [subtitles](business-subtitles.md), [files](business-subtitle-files.md), [styles](business-subtitle-styles.md), [accuracy](business-subtitle-accuracy.md). Text items: [text](architecture-text.md).

## Model and editing
- Lines are stored against the **source media** (media, start and end in that file, text, optional word timings, "edited by hand" flag). The timeline shows, for each video clip, the lines of its source inside the kept range, shifted to the clip: cutting removes them, copying duplicates, moving moves, trimming hides (extending shows again), splitting keeps (REQ-018).
- Edits (REQ-016, undoable): add at the playhead (2 s, inside the clip under it), delete, change text (drops word timings), change timing (kept inside its clip). A line in two copies is the same speech. One language and one style per project.
- **Position:** the box centre as a fraction of the picture (1/10000), shared by copies; "Reset position" returns to the style's place (1.2.0). **Size:** 25 to 400 % (1.2.6; [UI](architecture-ui.md#boxes-on-the-picture-126-140-161)). Dragging keeps the centre on the picture (1.4.0); one drag, one undo step; a dragged selected line moves the group.
- **Placement (2.0.3):** overlaps and parts outside the picture are **warned, never moved** (dashed outline; status bar count); "Move inside the picture" (least move, one undo step); Align stores a line's place first; a grown line keeps its centre.

## Styles and burn-in
- Three styles (REQ-017): **Classic** (white, black outline, bottom); **Box** (white on a semi-transparent box, lower third); **Word highlight** (bold white, current word yellow, about 4 words, centre-low).
- Burn-in: the `subtitles` filter (libass) reads an ASS file written by videolore; size = the smaller of 5.5 % of the video height and 9 % of its width. The ASS file and the bundled font are written to the private work folder and ffmpeg runs there, so no path enters a filter. Preview uses the same file at export resolution.
- Styling cannot be injected: `{`, `}`, `\` are replaced; only videolore writes tags; 1000 characters per line. A positioned line gets `\an5`/`\pos` from checked numbers; the burned centre is within 2 % of the requested place (test).
- **Shadow (2.1.0, REQ-306, 307):** 0 to 20 px at 1080 px height (default 0). The style's `Shadow` is `px * height / 1080` and `BackColour` the shadow colour, for every style including boxed. Boxed styles (`BorderStyle 3`): design A (box fill from `OutlineColour`, by spike) or design B (vector box behind the text). Shadow 0 writes the 2.0.4 values. Blinking hides the shadow with the text. `base/shadow.rs`, `subtitles::StyleSettings`.
- **Highlight colours (REQ-308):** Word highlight has **Not yet highlighted** (default white) and **Highlighted** (default yellow); stored in the project and the ASS export.
- **Enlarge word from its base (REQ-334, planned 3.1.0, ADR-248; replaces the centre rule of REQ-333):** the enlarged word keeps its baseline and grows upwards; its width grows on both sides, so its centre stays; the words before move left and the words after right by half its extra width. A pure module `word_layout` computes the positions from the measured widths; only a line with an enlarged word gets per-word events. The line's box grows upwards. Old projects with Enlarge word look different after the upgrade (release note).
- **Enlarge word (REQ-309; fix REQ-333, 2.3.0, ADR-235; layout replaced by REQ-334 in 3.1.0):** the spoken word N px larger at 1080 px (1 to 40, default 12). 2.2.0 built a hidden base event with an enlarged run on the line's `\an5 \pos`, so the whole line moved. 2.3.0: `measure::Shaped::place_words` gives each word's place, `ass::word_centres` its centre, `word_styles::enlarge` draws the word alone as `{\an5\pos(x,y)\fs..}`; the others keep their place; an unmeasurable line keeps the in-line form. Gate: centre within 2 % of the picture height; the in-house layout must match libass at 1080 px (spike).
- **Typewriter (REQ-310):** one event per word; letters appear with karaoke tags over a transparent secondary colour; spoken words stay in the Not-yet colour; a word under 0.1 s appears at once; no typing sound.
- **As subtitle track (REQ-337, planned 3.1.0, ADR-249):** style `Preset::AsTrack`. The export does not burn the lines: it writes them as one switchable track through the existing track writer (MP4 and MOV `mov_text`, MKV SRT) with the project language; the Subtitles choice of the export dialog is fixed to "Separate switchable track". The track has text and times only. The editor preview draws the lines in the Classic look so they can be placed (one exception to REQ-131 parity). 360 export writes the track too.
- Word styles add about two events per word (caps: [NFR](architecture-nfr.md#sizes-and-limits)). Lines without word times spread words evenly. `Command::SetSubtitleStyle`; `word_styles` plans; `Preset::{EnlargeWord, Typewriter}`.

## Creating subtitles
- Applied like the other effects ([effects](architecture-effects.md)). **On a clip:** only the source part the clip uses is processed; lines outside it are untouched; overlapping lines are replaced by the new ones, kept inside it. **On the whole video:** each source on the timeline is processed in full (a longer trim reveals more); all lines replaced.
- A text track in a source gives the choice of its text or recognition ([tracks](#subtitle-tracks-and-word-matching)). Hand-edited lines in the processed part trigger a question first (REQ-044).
- **Audio track (REQ-339, planned 3.1.0, ADR-253):** the popup's Audio track choice (the clip's audio tracks, track 1 by default) is an argument of the command, not stored. Extraction (step 2) maps `0:a:<k>`; the word timing matches the chosen track; a text track's lines keep their text (Use). With no text track, Recognise uses the chosen track.
- One job at a time ("Subtitles are being created" greys the entry); progress "Creating subtitles" or "Matching subtitles to the speech". The result is **one undo step** at the end; cancelling changes nothing.

## Speech-to-text
- **Engine:** whisper.cpp through `whisper-rs`, CPU, offline (REQ-014); cloud services, Vosk and candle-whisper rejected (ADR-009, ADR-021).
- **Build:** feature `stt` (default; needs cmake, a C++ compiler, libclang); without it "Generate subtitles" says it is not available; editing and burn-in still work. The player is built without it.
- **Steps:** (1) processor check and the model of the chosen level present (else the consent dialog); (2) ffmpeg extracts 16 kHz mono to the work folder (one background slot); (3) transcription in 120 s pieces, each also reading 2 s on each side; a word belongs to the piece where it starts; (4) word timings grouped into lines at pauses over 0.6 s or the style's word limit. Only the video's own audio is transcribed, never the music.
- **Language (REQ-015):** detected on the first piece and saved; the user may override (re-runs).
- **Threads:** from "How hard videolore works" ([performance](architecture-performance.md)); one transcription at a time; not counted in the ffmpeg cap. Speed (0.3.0): flash attention off; one decoding state per job; greedy decoding (beam search would make "Most accurate" 2 to 3 times slower). A level change never regenerates by itself.

## CPU requirements
- whisper.cpp is built for a **fixed baseline**: `.cargo/config.toml` sets `GGML_NATIVE=OFF` (0.2.1: release builds run on CI machines whose CPUs, e.g. with AVX-512, users may lack).
- Minimum on x86-64: AVX2, FMA, F16C, BMI2 (Intel Haswell 2013 and later; AMD 2015 and later, all Ryzen). AVX-512 never used. Apple Silicon supported.
- **Start-up check (0.3.0):** SSE4.2, AVX, AVX2, FMA, F16C, BMI2. If one is missing: "This computer's processor is too old for automatic subtitles; you can still add subtitle lines by hand.", no download, the three levels greyed, whisper code never run.
- No slower engine for old CPUs (decided 2026-11-14; a helper would be about 7 times slower; out of scope unless requested). Assumption: whisper.cpp runs no newer instruction while starting. Source builds use the same baseline (`GGML_NATIVE=ON` for one's own machine). `SOURCE_DATE_EPOCH` must stay unset (it selects the slow baseline silently).

## Quality levels and models (0.3.0)

| Level | File | Size | SHA-256 |
|---|---|---|---|
| Fast | `ggml-tiny-q5_1.bin` | 32,152,673 bytes | `818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7` |
| Balanced | `ggml-small-q5_1.bin` | 190,085,487 bytes | `ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb` |
| Most accurate | `ggml-large-v3-turbo-q5_0.bin` | 574,041,195 bytes | `394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2` |

- All multilingual, quantised, MIT, from a fixed revision of `huggingface.co/ggerganov/whisper.cpp`.
- Cost (assumption): Balanced 6 to 8 times slower than Fast, Most accurate 20 to 30 times; roughly 30x, 5x and 1 to 2x real time with 8 threads. Extra memory while transcribing: about 0.5 GB (Balanced), 1.3 GB (Most accurate; unavailable below about 8 GB RAM).
- Why these: "base" gains little over "tiny"; "medium" is slower and less accurate than large-v3-turbo; 8-bit about 40 % bigger; unquantised 3 to 6 times bigger; English-only loses languages. CI test: Balanced and Most accurate make no more word errors than Fast on a synthetic sentence.

## Speech model download
- Only when a level's model is missing and subtitles are generated; never in advance or by "Use recommended settings".
- **Consent:** the dialog shows level, size, licence and source. Choices: **Download**; **Use Fast instead (no download)** (only if Fast is present; this run only); **Download the Fast one instead (32 MB)** (only if no model); **Choose a file on this computer...** (exact size and SHA-256 of that level; copied into the data folder); **Cancel**.
- HTTPS only, the level's fixed address, into a partial file limited to the expected size; SHA-256 checked, then moved to `…/videolore/models/`. An interrupted download, a wrong checksum or a full disk deletes the partial ("Not enough free disk space"). Size checked before each use; SHA-256 once per session.
- `ureq` with `rustls` (no system OpenSSL), `HTTPS_PROXY` honoured, timeouts, at most 5 redirects, HTTPS only. Settings shows the space used and offers removal (known model files and partials only).

## Subtitle files and tracks
- **Import (2.0.4):** SRT and WebVTT, regular files of at most 16 MiB, UTF-8 (BOM accepted; other encodings refused), at most 100,000 cues, finite times; control and bidi characters removed; then the usual creation rules (one undo step).
- **Export:** SRT, WebVTT or ASS (`ass` module), through a temporary file and rename. File > Export subtitles (.srt): the timeline's lines after cuts, trims and copies; numbered; `HH:MM:SS,mmm --> HH:MM:SS,mmm`; UTF-8 without BOM; empty lines skipped; positions not written; greyed with "No subtitles on the timeline". One in-house reader and writer (no crate).
- **Separate track (REQ-182, ADR-171):** an SRT is one more input written as `mov_text` (MP4, MOV) or `srt` (MKV); never the default (MP4: the enabled bit is cleared).
- **Fonts (ADR-172):** Arabic and Devanagari faces committed (SIL OFL); Chinese, Japanese and Korean from system fonts.
- **Text tracks in videos (1.2.0):** ffprobe lists SRT, ASS/SSA, MP4 timed text and WebVTT tracks with language and title; picture tracks (PGS, DVB, DVD, teletext) ignored; at most 16 per file; language matches a code (letters, digits, `-`, at most 16 characters); title cleaned, at most 64 characters; identified by a position counted by videolore. Tooltip "Contains subtitles (<language>)". Media imported before 1.2.0 are checked when the project opens.
- **Reading a track:** ffmpeg converts it to SRT in the work folder (cancellable); videolore reads a regular file of at most 16 MiB with its own parser (`,` or `.` before milliseconds, CRLF, BOM, missing numbers; damaged blocks skipped; times checked). Only cues overlapping the processed part are used; tags such as `{\an8}` removed. Assumption: cue times relative to the file start (a test covers a file not starting at zero).

## Subtitle tracks and word matching
- **Choice:** "Use the video's subtitles", "Recognise speech" or "Cancel" (asked every time). Several tracks: a list; per source the track with the same language and title, else the same language, else the first. Sources without a track are recognised.
- **Use:** each line takes text, start and end from the track; recognition runs on the same audio only to place words (for word highlight), with the same consent and processor rules.
- **Matching (2.0.3):** deterministic, no crate or model. Words compared ignoring case and punctuation; long words may differ by one letter in four; short words must match exactly. Each cue is matched alone against recognised words from 1 s before to 1 s after it (Needleman-Wunsch with edit distance). A cue is matched when at least half its words match; matched words take the recognised times kept inside the cue; others spread evenly. **Cue start and end never change.** Unmatched cue: keeps its timing; status "N of M lines could not be matched to the speech; their words are spread evenly over the line". 10,000 cues under 50 ms.
- **Sound correction (2.0.3):** each matched word starts at the latest sound rise (10 ms steps) between 1.0 s and 0.2 s before the recognised start (the engine's DTW timing, 0.4 to 0.75 s late, is not used).
- **Paraphrased tracks:** when at least half a track's lines fall back, the status says the track does not follow the speech closely; the text is kept.
- **Fallbacks:** no speech support: "Use the video's subtitles (this computer cannot match words to the speech; they will be spread evenly)", nothing downloaded; download declined: "Use without matching words"; recognition fails: that part falls back with a message; Cancel: nothing changes.

## Accuracy target (REQ-177, amended 2.2.0): NOT MET
- **Target:** on the reference clip (committed 20 s synthetic clip, Fast) at least 90 % of matched words within 0.3 s; the matched share is reported; Balanced and Most accurate not worse than Fast. Test `alignment_matches_real_speech` in CI (VIDEOLORE_REQUIRE_STT=1).
- **Measured:** 2.0.3 Fast matched 23 of 28 words, mean word-start error 0.48 s, maximum 1.10 s (82 %).
- **2.2.0 method (`speech_regions.rs`, pure):** 10 ms RMS in dB; noise floor = 10th percentile; speech = floor plus T; silences under G bridged; regions under 0.10 s dropped; words of each cue placed over regions in the window (start − 1 s to end + 1 s) by dynamic programming (cost: length mismatch against at least 0.1 s per word and characters / R, plus lambda per skipped word); region-edge snap. Grid of 81 fixed settings: T {9, 12, 15} dB, G {0.12, 0.20, 0.30} s, R {8, 10, 12} characters/s, lambda {0.5, 1, 2} s.
- **Result (2026-11-22):** best setting 10 of 23 within 0.3 s (43 %); eligibility needs 75 %; `FROZEN = None`; the default path stays. Rejected spikes: dynamic time warping (0.43 to 0.75 s late), energy-onset snap (7 of 23).
- **Next step (if pursued):** a second clip (about 30 s of real speech with its transcript, sha-256 checked, Andrea's) with the frozen setting, pass at 90 % within 0.3 s, no re-tuning without a new decision; a pace-aware method is needed.

## Tests
- Transcription and download behind doubles: chunking, language, progress, cancel, consent per level, "Use Fast instead", checksum mismatch, interrupted download, non-HTTPS refusal, storage and removal, old processor, regenerate confirmation; the clip's part alone reaches ffmpeg; ASS positions; selection and group moves.
- One integration test runs the real model on a committed short voice clip ("hello", increasing word times) in CI with the model cached.
- ffprobe detection from recorded output and real MKV and MP4 files; SRT reader on damaged and hostile files; writer output read back by ffmpeg; matching with a fake recogniser.
- Word styles: Enlarge (2 %, others fixed, N = 0 refused, one undo); Typewriter (about two letters at 0.4 s of a 1 s five-letter word; export equals preview); shadows (8 px copy offset within 2 %; boxed shadow behind the box); highlight colours in preview, export and ASS.
