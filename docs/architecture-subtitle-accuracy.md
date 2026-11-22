# Architecture: subtitle accuracy (REQ-177, 2.2.0, built: not met)

Part of [2.2.0 overview](architecture-2.2.0-overview.md). Requirement REQ-177 (amended 2.2.0; [subtitle accuracy](business-subtitle-accuracy.md)); the other timing rules REQ-178 and REQ-179 are built and unchanged.

## Why a new method
In 2.0.3 the Fast speech model matched 23 of 28 words on the reference clip, and the matched words started 0.48 s late on average. Measured and not shipped: dynamic time warping (every word 0.43 to 0.75 s late), an energy-onset snap of the recognised times (7 of 23 within 0.3 s), and a speech-region prototype (21 to 22 of 23 within 0.3 s for 4 of 120 settings, all on the one synthetic clip, whose words are separated by 0.15 s pauses).

## Method (speech regions)
1. **Energy frames:** 10 ms RMS of the recogniser's 16 kHz mono samples, in dB. No second decode.
2. **Speech regions:** the noise floor is the 10th percentile of the clip's frames (no tuned value). A frame is speech when its level is at least the floor plus T. Silences shorter than G are bridged; regions shorter than 0.10 s are dropped.
3. **Placement:** the words of each cue (in order; a cue never takes words used by an earlier cue) are placed over the regions in the cue's window (cue start minus 1 s to cue end plus 1 s). A dynamic programme gives each region a number of words; the cost of a region is the difference between its length and the predicted length of its words (at least 0.1 s per word, characters divided by R), plus a penalty lambda for each skipped word. A recognised word that falls in a region moves that word to the region's edge (the region-edge snap; it replaces the 2.0.3 energy-onset snap, which is removed).
4. **Parameters, fixed in advance:** T in {9, 12, 15} dB, G in {0.12, 0.20, 0.30} s, R in {8, 10, 12} characters per second, lambda in {0.5, 1, 2} s: 81 settings.

## Selection and check
- **Reference clip only:** the setting with the most matched words within 0.3 s; ties go to the first setting in the order T, G, R, lambda ascending; a setting below 75% within 0.3 s is not eligible. Recorded before the second clip is used; no setting changes after it.
- **Second clip:** about 30 s of real speech with its transcript, recorded and committed by Andrea (a fixture with a sha-256 check, as the reference clip). The build asks for it before it reaches this requirement.
- **Pass:** at least 90% of the matched words within 0.3 s on the second clip with the frozen setting (the reference clip has the same target). The matched share is reported per clip, not required.
- **Fail:** REQ-177 stays planned; the result is reported; no re-tuning on either clip without a new decision.

## Built result (2026-11-22): NOT MET
- The method is built and wired (`speech_regions.rs`, `align::retime_with`, `stt::recognise_with_level`). The grid of 81 settings on the reference clip gives at most 10 of 23 matched words within 0.3 s (43%); the target is 90%, and a setting needs 75% to be eligible. No setting is frozen (`FROZEN = None`): the recognised times stay the default path. The check on the second clip exists and is skipped until the clip is present. REQ-177 stays planned; a pace-aware method is needed.

## Tests and limits
- The strict test `alignment_matches_real_speech` runs without `ignore` in CI (VIDEOLORE_REQUIRE_STT=1, pinned ffmpeg, the committed fixture); the second clip has its own test.
- Known limitation: a word the speech model mishears is not matched and is not counted; continuous speech has long regions, which is why the second clip matters.
- No new crate; the module is pure (`src/speech_regions.rs`).
