# Code — effects on clips, the video's own subtitles, positions, SRT (1.2.0)

Part of [code.md](code.md). Everything here was added in 1.2.0.

## Applying an effect

`Command::ApplyEffect { effect, target }` is the single entry for the three effects
(`EffectKind::{Fade, Subtitles, Volume}`) and the three targets (`Target::Clip(id)`,
`Target::Selected` = the selected clip or the one under the playhead, `Target::WholeVideo`).
The Effects list (click = open the popup, double-click = whole video, drag onto a video clip =
that clip), the clip's right-click menu (Effects ▸) and the Effects menu all send it. The code is
in `src/app/effects.rs`.

| Effect | On a clip | On the whole video | Undo step |
|---|---|---|---|
| Fade through black | `Timeline::set_clip_fades`: the fades at both of its edges (boundary fades; the start fade if it is the first clip, the end fade if it is the last) | `Timeline::set_edge_fades`: `start_fade` and `end_fade` | one, "Fade through black" (none when the fades are already there) |
| Volume | opens the clip's volume popup (`Instance::ClipVolume`) | opens the Volume popup | when a value changes ("Clip volume" / "Volume"), not when the popup opens |
| Subtitles | creates subtitles for the clip's source range | for every file of the timeline | one, when the background job ends |

The popup opened by applying shows what it was applied to in its title ("Fade through black —
whole video", "Clip volume — b.mp4"; `AppState::popup_title`).

Dragging uses egui's drag-and-drop payload (`EffectKind`): only video clips read it
(`dnd_hover_payload` / `dnd_release_payload` in `ui/timeline.rs`), so a drop on the music track
or anywhere else does nothing; the pointer shows "copy" over a clip and "no drop" elsewhere on
the timeline.

## Fades at the video's edges

Since 1.6.1 the fade in from black and the fade out to black belong to **clips**: `Clip.fade_in`
(used when the clip is the first one) and `Clip.fade_out` (used when it is the last one), both
absent from the file when false. Cutting or moving the first clip takes its start fade with it.
`Timeline::fades_of(i)` reports them as the fade-in of the first clip and the fade-out of the
last one, so the renderer, the preview's frame key and the timeline drawing need no special
case; `start_fade()` / `end_fade()` read the first and last clip's flags and `set_edge_fades`
is the whole-video Fade. A split keeps the start fade on the left half and the end fade on the
right one. One exception in `render`: the picture of the end fade starts two frames early
(`Segment::to_black`), so that the very last frame is black even when a source starts a few
milliseconds late; the sound fade is unchanged. Each edge fade has its own marker and popup
(`Instance::FadeStart` / `FadeEnd`, `Command::SetEdgeFade`); "Remove all fades" removes them too.

## Whole-video effects are copies on the clips (1.6.1, REQ-147, REQ-148)

There is no whole-video state any more, except Change aspect. A double-click applies an effect
to every clip at that moment, with the same code as for one clip: Volume sets the level of each
clip (`Command::SetMix` writes the clips; the project keeps only the music level), Apply LUT
puts the path in every `clip_luts` entry (replacing a clip's own), Scale and Move add one
copy per clip with a key at 0 (a clip that has one is skipped, so there is no second copy),
the Fade sets the flags above. A clip added later has nothing; deleting a clip drops its entries.
`FrameTarget::Whole` stays in the API as a way to say "all clips": reading gives the first clip's
copy and a change reaches all copies (`Timeline::whole_edit`: the key time is clip time shifted
by the first clip's source start, and a copy shorter than that time is skipped). The picture
handles edit the clip under the playhead (`frame_target_at_playhead`); the whole-video popup
shows the first clip's keys on a timeline as long as the longest clip (`key_range`).

## Clip volume

`Clip.volume` (0–100, default 100) and `Clip.muted`. The gain is the slider curve
`(p/100)²` (`music::clip_gain`); the renderer adds `volume=<gain>` to that clip's audio chain
before its fades, **only when the gain is not 1**, so projects without clip volumes produce
exactly the ffmpeg arguments of 1.1. The project's video level is applied after the clips are
joined, so the two multiply. A changed clip volume forces a re-encode. Split, copy/paste and
reorder keep the values; `Timeline::can_merge` needs equal values
(`merge_block_reason` gives "The clips have different volumes").

## Subtitles for a clip or the whole video

`src/app/subs_create.rs`. A request (`CreateRequest`) waits in `AppState::pending_create` while
a dialog or a check is open; every answer calls `continue_create`, which goes as far as it can:

1. One creation at a time ("Subtitles are being created").
2. Media analysed before 1.2.0 have unknown subtitle tracks: they are checked in the background
   when the project opens (`start_track_check`, results patch the project **and** the saved
   copy, so the project does not become "unsaved"); a creation asked meanwhile waits for it.
3. Hand-edited lines in the range → the usual confirmation.
4. A source has text subtitle tracks → `Modal::SubtitleSource` (track labels, Use / Recognise /
   Cancel). On computers that cannot run speech recognition (old processor, or a build without
   `stt`) the Use button says the words will be spread evenly.
5. The speech file is missing → the consent dialog; when subtitles are being reused it also
   offers "Use without matching words" (no download).
6. `stt::start` with one `RangeJob` per source: `Mode::Recognise`, `Mode::Align { track }` or
   `Mode::Spread { track }`.
7. `SttEvent::Done` → `finish_create`: one undo step. A clip replaces only the lines of that
   source range (`Subtitles::replace_range`); the whole video replaces everything, as before.

Cancel at any point (dialogs, download, matching) leaves the project unchanged.

### Reading a track

`stt::track_args` asks ffmpeg to convert track N to SRT in the job's private work folder
(`-map 0:s:N -c:s srt`; N is videolore's own count of the subtitle streams, never a string from
the file). The file is read with a 16 MiB limit and parsed by `srt::parse` (at most 100 000
cues, 1000 characters each, damaged blocks skipped); `srt::strip_tags` removes `{...}` and
`<...>` tags and joins the lines. Cue times are relative to the start of the file.

### Matching words (`align`)

Recognition runs on the same audio range as "Recognise" (with the track's language, mapped by
`base::language::iso639_2_to_1`, unless the user chose one). `align::align` then works cue by cue, forward
only: the cue's words against the recognised words from 1 s before to 1 s after it, with a
Needleman–Wunsch alignment (exact word +2, close spelling +1 for words of 3+ letters,
substitution −1, gap −1; recognised words before and after the cue cost nothing). A cue is
matched when at least half of its words are found; found words take the recognised start
(kept inside the cue, never going back), the others are spread between their found neighbours.
An unmatched cue keeps no word times (they are spread evenly when shown). The line always keeps
the cue's text and times. If recognition fails, every cue falls back the same way and the user
is told.

**Known limitation (measured 2026-11-15):** with the Fast speech file, whisper.cpp's word
times are coarse. On the committed fixture (`tests/fixtures/align/`) 23 of 28 words were found
and their starts were 0.48 s off on average (1.10 s at worst); the target of the requirement
(90 % found, each within 0.3 s) is not reached. The test that checks that target
(`alignment_matches_real_speech`) is therefore marked `#[ignore]`.

## Subtitle positions

`Line.position: Option<Pos>`; `Pos { x, y }` is the centre of the text box in 1/10000 of the
output picture, so it survives a change of output format. `ass::document` writes
`{\an5\pos(X,Y)}` in front of the line (and of every word group of the word-highlight style);
without a position the document is exactly the one of 1.1. Only `ass` writes tags; the numbers
are validated (`0..=10000`) when a project is loaded.

In the preview (`ui/placement.rs`, `subtitle_drag` in `ui/mod.rs`) the boxes of the lines on
screen are computed in output-frame pixels: the text is measured with egui using the bundled
subtitle font (registered once as the family `videolore-subtitle`), padded by the outline, and
centred on the line's position or on the style's place. While dragging, the centre is kept so
that the whole box stays inside the picture, `Command::DragLines` shows the lines there at once
(`AppState::shown_project` gives the preview a copy of the project with the dragged position),
and the release sends `Command::SetLinePositions` (one undo step). When the dragged line is one
of several selected lines, all of them get exactly its position.

Selection (`AppState::selection`, not part of the project or of undo): Ctrl/Cmd+click on a
marker or a row of the Subtitles popup (`Command::ToggleLine`), Shift+drag on the timeline
(`Command::SelectRange`: lines whose start is in the range; the overlay exists only while Shift
is held, so it never takes input from the clips), Esc with no popup open clears it.

## SRT export

File > Export subtitles (.srt)… → `Command::PickExportSrt` → `srt::write` on
`Subtitles::project(&timeline)` (timeline times after every edit, numbered in time order,
`HH:MM:SS,mmm`, LF, UTF-8 without BOM, empty lines skipped) → `project::write_atomic`
(temporary file + rename: no partial file).

## Timeline ruler

`ui/ruler.rs` (pure): the format follows the time at the right edge of the visible part
(`≤ 90 s`: "45s"; `≤ 90 min`: "75:30"; beyond: "01:05:00"); ticks are multiples of the first
step of 1, 2, 5, 10, 15, 30 s, 1, 2, 5, 10, 15, 30 min, 1 h, 2 h, 5 h… whose spacing fits the
widest label, and only the visible ones are made.

## Project format 3

Written always since 1.2.0. New, all optional when reading: `volume` and `muted` of a clip,
`start_fade` / `end_fade` of the timeline (since 1.6.1 `fade_in` / `fade_out` of a clip), `position` of a subtitle line, `subtitle_tracks` of a
media format (absent = not checked yet). Files of 1.1 and earlier load with the defaults and
export exactly as before (`tests/fixtures/export-argv-1.1.0.txt`; since 1.6.1 the video's sound level of such a file is the level of each clip, so the volume is applied per clip: `export-argv-1.1.0-in-1.6.1.txt`). Videolore 1.1 refuses format
3 with its "newer version" message. A clip volume above 100 or a position outside `0..=10000`
is refused; subtitle track descriptions are made safe instead (language code checked, title
without control characters and at most 64 characters, at most 16 tracks).

## Project format 9 (1.6.1, REQ-149)

`src/project/v9.rs` converts a file of format 8 or older in memory when it opens (no undo step,
no change mark): the whole-video fields are read aside (`Legacy::take`; a value of the wrong
type is a damaged file) and `apply` writes them into the clips. Scale and Move keys (timeline
time) become clip time and are sampled at the clip's start and end, so the picture is the same;
the video's sound level is multiplied into each clip's level (rounded to 1%); `start_fade` /
`end_fade` become the first clip's `fade_in` and the last clip's `fade_out`; the whole-video LUT
becomes each clip's LUT. Exception (Q-F): a clip with its own Scale or Move and a whole-video
one gets ONE path (`merge_path`): times are 0, the clip length, its own key times and the
shifted whole-video key times inside the clip (at most 200, own keys first), scales multiplied,
moves added, exact at those times and a line between; a clip with its own LUT keeps it. One
status-line note says how many clips were combined or kept their LUT. Format 9 is refused by
1.6.0 ("newer version"); 1.6.1 writes no `whole_*` field.
