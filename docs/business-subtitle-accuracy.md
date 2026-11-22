# Business — Subtitle accuracy

Back to the [index](business.md). Added in 2.0.3 (same release as readable text and subtitle placement). It extends the subtitle features of [Subtitles](business-subtitles.md) (REQ-014, REQ-087, REQ-095). The 2.2.0 build of the target is planned in [Speed](business-speed.md).

## Status of the accuracy target (REQ-177)
REQ-177 is **planned, not met** in 2.0.3 and 2.1.0. The target of REQ-095 (at least 90% of the matched words within 0.3 s) is not reached with the Fast level on the committed reference clip:
- 23 of 28 words are matched; the other 5 are misheard by the speech model.
- The matched words start 0.48 s late on average, 1.10 s at worst. The sound onsets of the clip match the reference within 0.03 s.
- Tried in 2.0.3: dynamic time warping (every matched word 0.43 to 0.75 s late); a one-sided onset snap (7 of the 23 matched words within 0.3 s); a speech-region method (21 to 22 of 23 matched words within 0.3 s, but only with parameters tuned on the one synthetic clip, whose words are separated by 0.15 s pauses).
- The strict test `alignment_matches_real_speech` stays ignored until REQ-177 is built.
- Andrea chose (2.2.0) to build the speech-region method with its parameters checked on more than one clip; the target is kept.

## Requirements
- REQ-177 (planned, amended in 2.2.0; measured in 2.2.0: not met): the speech-region method is built and wired; on the reference clip 10 of 23 matched words (43%) start within 0.3 s, against 90%; no setting reaches the eligibility, so the old path stays the default and the strict test stays ignored. The second-clip check waits for `tests/fixtures/align/second-clip-30s.wav`, which the user records and commits. Reused subtitles are timed to the speech regions of the clip: the words of the video's own subtitles are placed in the regions where speech is detected, with one set of parameters, tuned once and not changed per clip. Acceptance: with that one set, at the Fast level, on the committed reference clip and on at least one further clip with a known transcript (the check set), at least 90% of the words that are matched to the reference start within 0.3 s of the reference, after the onset snap, on every clip of the check set. The share of matched words is reported per clip, not required: a word the speech model mishears is not matched, and that stays a known limitation. The parameters are tuned on the reference clip; the further clip is used only to check them. Precondition: the further clip is recorded and committed by the user (about 30 s of speech with its transcript); the build asks the user for it before it reaches REQ-177, and REQ-177 is not built without it. The speech model is not changed. The Balanced and Most accurate levels are measured on the same clips and are not worse (a measurement for the maintainer, not a gate). Acceptance: the strict test `v12::alignment_matches_real_speech` is enabled (no `ignore`) and passes locally and in CI with the pinned version of ffmpeg and the committed speech fixture (`tests/fixtures/align/speech.mkv`, no speech synthesiser is installed in CI); the test reports the share of matched words per clip. If a clip of the check set misses the target, the method is not shipped with those parameters: the result is reported and REQ-177 stays planned.
- REQ-178 (implemented): Recognition in pieces of about 2 minutes does not lose or duplicate words at a piece boundary: consecutive pieces overlap, and a word heard in both is kept once. Acceptance: a reference clip of 5 minutes of speech with a known transcript: no word of the transcript is missing from the recognised words, and a word heard twice at a boundary appears once.
- REQ-179 (implemented): When a video's own subtitles are used (REQ-087) and at least half of the lines fall back to evenly spread words (REQ-095), the status message after matching adds that the video's subtitle text does not follow the speech closely, so the word timing is approximate; the text is kept as it is. Acceptance: a paraphrased track on a reference clip gives this message with the fallback count; a track that follows the speech gives no extra message.

## Key decisions
- (2.0.3, Andrea 2026-11-20, D56) The REQ-177 target is worded as a share of the matched words within 0.3 s at the Fast level, after the onset snap; the matched share is reported, not required. Reason: the speech model mishears 5 of 28 words on the reference clip, and no timing change can match a word the model did not hear. The speech model stays as it is: no larger model, no fuzzy matching.
- (2.0.3, [MINOR]) The gate of 2.0.3 was one reference clip at the Fast level; the other levels are measured, not gated.
- (2.2.0, Andrea) Build the speech-region method with REQ-177's target, and check its parameters on more than one clip before shipping. The earlier tuning on one synthetic clip is not enough.
- (2.2.0, Andrea, REFINE) The second clip is recorded and committed by Andrea himself (about 30 s with its transcript). It is a precondition of REQ-177: the build asks for it before reaching that requirement.

## Known limitations
- Until REQ-177 is built, reused subtitles have 23 of 28 words matched on the reference clip, and the matched words start 0.48 s late on average, against a target of 90% within 0.3 s.
- Words the speech model mishears are not matched (5 of 28 on the reference clip) and keep evenly spread timing. Not fixed (D56): fixing it needs a larger model or fuzzy matching.

## Out of scope
- Importing subtitle files and re-timing lines from the speech (see the roadmap).
- Text from picture-based subtitle tracks (OCR).
- Speech models other than the three subtitle quality levels.
- A larger speech model, and fuzzy matching of misheard words, to raise the matched share (D56).
