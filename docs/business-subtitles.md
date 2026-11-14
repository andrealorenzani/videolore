# Business — Subtitles

Back to the [index](business.md).

## Features
- Automatic subtitles generated on this computer from the speech in the video (never from the music), in the background with progress and cancel.
- Three subtitle quality levels (Fast, Balanced, Most accurate), chosen in Settings; the recommended one depends on the computer (see [Settings, performance and safety](business-performance.md)).
- The speech model of the chosen level is downloaded once, after a dialog that shows its size (about 32 MB for Fast, 190 MB for Balanced, 574 MB for Most accurate) and licence and asks for consent; alternatively the user can provide the model file. After that, everything works offline.
- On processors too old for the speech engine, a clear message replaces automatic generation; lines can still be added by hand.
- Language detected automatically, with a manual override.
- Subtitle lines can be added, deleted, re-worded and re-timed; all changes can be undone.
- At least three social-media styles, including word-by-word highlight, visible in the preview and burned into the exported video.
- Subtitles stay attached to the speech they belong to when the timeline is edited.

## Requirements
- REQ-014 (implemented): Subtitles are generated offline from the video audio only (not the music), in the background with progress and cancel. The speech model of the chosen subtitle quality level (REQ-052) is downloaded once after a consent dialog showing its size and licence, or provided by the user as a local file; each model has its own consent; afterwards everything works offline. These downloads are the only network access of the product.
- REQ-015 (implemented): The subtitle language is detected automatically and can be overridden by the user; a project has one language.
- REQ-016 (implemented): Subtitle lines can be added, deleted, re-worded and re-timed, and all these actions can be undone.
- REQ-017 (implemented): At least 3 social-media style presets are available, including word-by-word highlight; a project uses one preset; subtitles are visible in the preview and burned into the export; text size scales with the video height so it is readable in both vertical (9:16) and horizontal video.
- REQ-018 (implemented): Subtitles are tied to the speech in the source: cutting a segment removes its subtitles, moving or copying a segment moves or duplicates them, trimming hides those outside the kept range, splitting keeps them.
- REQ-044 (implemented): Regenerating subtitles asks for confirmation before replacing lines the user has edited, also when regenerating at a different subtitle quality level.

Subtitle quality levels, model downloads and storage, and older processors are covered by REQ-052, REQ-053 and REQ-054 in [Settings, performance and safety](business-performance.md).

## Key decisions
- The speech model file must be exactly the expected one (checked before use), whether downloaded or provided by the user.
- The bundled subtitle font (Noto Sans Bold) covers Latin, Greek and Cyrillic scripts only.
- Changing the text of a line drops its per-word timing; the word-by-word highlight then spreads the words evenly over the line.
- If the same part of a source appears twice on the timeline (for example after copy and paste), editing the text of its subtitle changes both copies, because they are the same speech.
- Any subtitle line in the project means the export is re-encoded.

## Known limitations
- Scripts other than Latin, Greek and Cyrillic (for example Chinese, Japanese, Arabic, Hindi) use whatever fonts the system has: in the exported video they may look different on each computer or appear as empty boxes, and they cannot be read in the application's subtitle editor.
- Recognition is done in pieces of about 2 minutes; accuracy may be slightly lower right at the boundaries.
- x86-64 processors without AVX2 (from before about 2013) cannot generate subtitles automatically. Since 0.3.0 videolore checks this and shows a message instead of crashing (in 0.2.1 and 0.2.2 generation could crash on such processors).

## Out of scope
- Separate subtitle files (SRT/VTT) or switchable subtitles.
- More than one language per project, translation.
- Custom style editor, free positioning of subtitles.
- Speech models other than the three offered by the subtitle quality levels.
