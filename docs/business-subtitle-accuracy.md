# Business — Subtitle accuracy

Back to the [index](business.md). Added in 2.0.3 (same release as readable text and subtitle placement). It extends the subtitle features of [Subtitles](business-subtitles.md) (REQ-014, REQ-087, REQ-095).

## Status of the accuracy target (REQ-177)
REQ-177 is **planned, not met** in 2.0.3. The target of REQ-095 (at least 90% of the words matched, each within 0.3 s) is not reached with the Fast level on the committed reference clip:
- 23 of 28 words are matched; the other 5 are misheard by the speech model.
- The matched words start 0.48 s late on average, 1.10 s at worst. The sound onsets of the clip match the reference within 0.03 s.
- Tried and removed: dynamic time warping (every matched word 0.43 to 0.75 s late); a one-sided onset snap (7 of the 23 matched words within 0.3 s); a speech-region method (21 to 22 of 23 words, but only with parameters tuned on the one synthetic clip, whose words are separated by 0.15 s pauses).
- The strict test `alignment_matches_real_speech` stays ignored, with these measurements.
- To close it, a timing method independent of pauses is needed (for example forced alignment with a different model), or the target must be restated. A larger speech model is excluded by decision D56.

## Requirements
- REQ-177 (planned): At the Fast level, on the committed reference clip, at least 90% of the words that are matched to the reference start within 0.3 s of the reference, after the onset snap. The share of matched words is reported, not required: a word the speech model mishears is not matched, and that stays a known limitation. The Balanced and Most accurate levels are measured on the same clip and are not worse (a measurement for the maintainer, not a gate). The speech model is not changed. Acceptance: the strict test passes locally and in CI with the pinned version of ffmpeg and the committed speech fixture (`tests/fixtures/align/speech.mkv`, no speech synthesiser is installed in CI), and the test reports the share of matched words.
- REQ-178 (implemented): Recognition in pieces of about 2 minutes does not lose or duplicate words at a piece boundary: consecutive pieces overlap, and a word heard in both is kept once. Acceptance: a reference clip of 5 minutes of speech with a known transcript: no word of the transcript is missing from the recognised words, and a word heard twice at a boundary appears once.
- REQ-179 (implemented): When a video's own subtitles are used (REQ-087) and at least half of the lines fall back to evenly spread words (REQ-095), the status message after matching adds that the video's subtitle text does not follow the speech closely, so the word timing is approximate; the text is kept as it is. Acceptance: a paraphrased track on a reference clip gives this message with the fallback count; a track that follows the speech gives no extra message.

## Key decisions
- (2.0.3, Andrea 2026-11-20, D56) The REQ-177 target is worded as a share of the matched words within 0.3 s at the Fast level, after the onset snap; the matched share is reported, not required. Reason: the speech model mishears 5 of 28 words on the reference clip, and no timing change can match a word the model did not hear. The speech model stays as it is: no larger model, no fuzzy matching.
- (2.0.3, [MINOR]) The gate is one reference clip at the Fast level; the other levels are measured, not gated.
- (2.0.3) The timing methods above did not reach the target without tuning on one clip. Whether to change the target or to accept a different timing method is open for Andrea (see the status).

## Known limitations
- REQ-177 is not met (see the status above): with the Fast level, reused subtitles have 23 of 28 words matched and the matched words start 0.48 s late on average, against a target of 90% within 0.3 s.
- Words the speech model mishears are not matched (5 of 28 on the reference clip) and keep evenly spread timing. Not fixed in 2.0.3 (D56): fixing it needs a larger model or fuzzy matching.

## Out of scope
- Importing subtitle files and re-timing lines from the speech (see the roadmap).
- Text from picture-based subtitle tracks (OCR).
- Speech models other than the three subtitle quality levels.
- A larger speech model, and fuzzy matching of misheard words, to raise the matched share (D56).
