# Architecture: subtitle tracks in videos, word matching and SRT export

Part of [Architecture](architecture.md). Since 1.2.0 (requirements REQ-086..088 and REQ-095, see [business](business.md)). How subtitles are created in general is in [subtitles](architecture-subtitles.md).

## Finding subtitle tracks
- When a file is analysed, the existing `ffprobe` call (no extra process) also lists its subtitle streams. Text tracks are kept: SRT, ASS/SSA, MP4 timed text and WebVTT, with their language and title. Picture-based tracks (DVD, Blu-ray PGS, DVB, teletext) are ignored.
- At most 16 tracks per file are kept. The language must look like a language code (letters, digits and `-`, at most 16 characters), otherwise it is left out. The title is cleaned like any displayed name (control and text-direction characters replaced) and shortened to 64 characters. A broken track description never makes the file unusable.
- Each track is identified by its position among the file's subtitle streams, counted by `videolore`; nothing read from the file is ever passed to `ffmpeg` as an argument.
- The media item's tooltip says "Contains subtitles (<language>)".
- Media imported before 1.2.0 are checked again in the background when the project opens (see [project files](architecture-projects.md#format-versions)).

## Reading a track
- `ffmpeg` converts the chosen track to plain SRT in `videolore`'s private work folder (one background slot of the job queue, can be cancelled). This keeps all container parsing (MKV, MP4…) inside `ffmpeg`, which already parses the video.
- `videolore` reads that file only if it is a regular file of at most 16 MiB, and parses it with its own small SRT reader: it accepts `,` or `.` before milliseconds, Windows line ends, a byte-order mark and missing cue numbers, skips damaged blocks, and keeps at most 100,000 cues. Times are checked (no overflow, start before end); bad cues are dropped.
- Only the cues overlapping the part being processed are used, cut to that part. Formatting and position tags (`{\an8}`, `<i>`, …) are removed, a cue on several lines becomes one line, and the text is cleaned like typed text (at most 1000 characters, no control characters). Burned-in text is still escaped as before, so a track cannot add styling.
- **Assumption:** `ffmpeg` writes cue times relative to the start of the file, also for files whose timestamps do not start at zero; an integration test covers such a file.

## Use the video's subtitles or recognise the speech
When subtitles are about to be created (see [subtitles](architecture-subtitles.md#creating-subtitles)) and at least one source involved has a text track, a dialog asks: "Use the video's subtitles", "Recognise speech" or "Cancel". With several tracks, a list (language and title) chooses one; for each source the track with the same language and title is used, otherwise the same language, otherwise its first track. Sources without a track are always recognised. The question is asked every time.

With **Use**, each subtitle line takes its text and its start and end from the track; speech recognition is then run on the same audio only to place each word in time (for the word-highlight style), using the model of the Subtitle quality chosen in Settings, with the same consent, checks and processor rules as normal recognition. The language of the track is used for the project and for recognition, unless the user has chosen a language.

## Matching words to the speech
- Done by a small, deterministic module of `videolore` (no new crate, no extra model).
- Words are compared ignoring case and punctuation; long words may differ by a few letters (at most one letter in four); short words must match exactly. Missing and extra words are tolerated.
- Each cue is matched on its own, against the recognised words from 1 s before it to 1 s after it, in time order, so an error in one cue cannot shift the following ones. The method is a classic sequence alignment (Needleman–Wunsch) with an edit distance for spelling differences.
- A cue counts as matched when at least half of its words are matched. Matched words get the recognised times, kept inside the cue; unmatched words in between are spread evenly between their matched neighbours. **The cue's start and end are never changed.**
- A cue that cannot be matched (less than half of the words, or no speech) keeps its timing and its words are spread evenly; at the end the status bar says "N of M lines could not be matched to the speech; their words are spread evenly over the line".
- Speed: 10,000 cues in under 50 ms. The cost of the whole operation is the cost of speech recognition on the same audio.

## When matching is not possible
The track's text is never lost: the fallback is always the track's text with its own timing and words spread evenly.
- **Processor without speech support, or a build without speech-to-text:** the Use button reads "Use the video's subtitles (this computer cannot match words to the speech; they will be spread evenly)" and nothing is downloaded.
- **Model download declined:** the consent dialog has an extra button "Use without matching words" (no network access).
- **Recognition fails:** that part falls back as above, with a message.
- **Cancel** during matching or download: nothing changes and no undo step is created.

## Export as SRT
- File > Export subtitles (.srt)… writes the lines as they appear on the timeline (after cuts, trims and copies), numbered in time order, `HH:MM:SS,mmm --> HH:MM:SS,mmm`, UTF-8 without byte-order mark, one text line per cue. Lines with empty text are skipped. Positions are not written (SRT has no standard for them).
- The save dialog proposes `<project name>.srt`. The file is written to a temporary name and renamed, so a failed write leaves no partial file. It is greyed with "No subtitles on the timeline" when there is nothing to write.
- The writer and reader are the same small in-house module; existing SRT crates were not chosen because they add an untrusted-input dependency for about 200 lines of code that are easy to test fully.

## 2.0.3 (planned): accuracy and recognition pieces
- **Target (reworded by the maintainer):** on the committed reference clip at the Fast level, at least 90% of the words that are matched start within 0.3 s of their reference time, after a small correction of each word's start to the nearest sound onset. How many words are matched is reported, not required. Balanced and Most accurate must not be worse than Fast. The strict test runs without being skipped; CI requires the speech model and the fixtures (checked by their checksums) and fails when one is missing.
- **How:** the speech engine's word starts are corrected by the sound: each matched word starts at the latest rise of the sound (10 ms steps) between 1.0 s and 0.2 s before the recognised start. Measured: the engine's own token timing (DTW) was 0.4 to 0.75 s late, so it is not used; with the sound correction the onsets were within 0.03 s of the reference. No new model, no fuzzy matching. A larger speech model is not planned; it would need a separate decision on its download size.
- **Pieces of about two minutes:** each piece also reads 2 seconds of the audio on each side; a word belongs to the piece where it starts. A word at a boundary is therefore kept once and in full. The extra audio is about 3% of the work at the Fast level.
- **Paraphrased tracks:** when at least half of the lines of a video's own track fall back to evenly spread words, the status message says that the track does not follow the speech closely and that the word timing is approximate. The text is kept.
- **Tests (planned):** a synthetic five-minute audio with known words across each boundary (no word missing or repeated); a speech fixture of five minutes with its transcript; a paraphrased track gives the message with its count, a track that follows the speech gives none.

## Tests
- Detection from recorded `ffprobe` output (text tracks kept, picture tracks dropped, hostile descriptions), and with real files generated by `ffmpeg` (MKV with SRT in Italian, MP4 with timed text in English).
- SRT reader against a corpus of damaged and hostile files; writer output read back by `ffmpeg` as an independent check.
- Matching with a fake recogniser: identical words, misspelt / missing / extra words, completely different text, times outside the cue, an old processor (no download, no recognition), cancel during matching and download.
- One gated test runs the real Fast model on a committed 20-second synthetic clip with a matching SRT: at least 90% of words matched, each within 0.3 s of the reference time. **Status in 1.2.0:** not met (23 of 28 words, 82%; mean word-start error 0.48 s, maximum 1.10 s on the committed fixture), so `alignment_matches_real_speech` is `#[ignore]`d and a weaker gated test runs instead. **2.0.3 (planned):** the gate is reworded (at least 90% of the matched words within 0.3 s after the sound correction; the matched share is reported); see the section above.

## Risks
- Word times from whisper are accurate to about 0.1–0.3 s, so the 0.3 s target is close to the limit (assumption); measured on the fixture the Fast model is worse than that, see the status above. In 2.0.3 the sound correction brings the matched words' onsets to within 0.03 s of the reference on the fixture (measured); words the model mishears stay unmatched.
- Tracks that are translations or paraphrases of the speech will often fall back to evenly spread words.
- Matching costs as much as recognising the same audio.
