// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Subtitles of a project. Lines are stored in **source** time (the media file they were
//! spoken in), so they follow the video edits automatically: [`Subtitles::project`] maps
//! them onto the timeline through the clips. Cutting a clip removes its lines, copying a
//! clip duplicates them, trimming hides the ones outside the kept range, splitting keeps
//! them. Edits (add, delete, re-text, re-time) are plain methods, undone by snapshots.

use crate::base::pos::{POS_MAX, Pos};
use crate::media::MediaId;
use crate::time::Micros;
use crate::timeline::{ClipId, Timeline};
use serde::{Deserialize, Serialize};

/// Longest text of one line (characters).
pub const MAX_TEXT: usize = 1000;
/// Most words stored in a project (all lines together).
pub const MAX_WORDS: usize = 200_000;
/// Length of a line added by hand.
pub const NEW_LINE_LEN: Micros = Micros(2_000_000);
/// Step of a dragged start or end marker, and the shortest a line can be made by dragging
/// (REQ-192).
pub const EDGE_STEP: Micros = Micros(100_000);
/// Step of "Shift lines earlier / later" (REQ-197).
pub const SHIFT_STEP: Micros = Micros(500_000);

/// Which end of a line a marker drag moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEdge {
    Start,
    End,
}

/// The `(start, end)` of a line that was shown from `start` to `end` when its `edge` marker
/// is dragged to time `to` (REQ-192): moved in whole steps of [`EDGE_STEP`] from where it
/// was, the start never past the end and each line at least one step long.
pub fn dragged_edge(start: Micros, end: Micros, edge: LineEdge, to: Micros) -> (Micros, Micros) {
    let snap = |from: Micros| {
        let steps = ((to.0 - from.0) as f64 / EDGE_STEP.0 as f64).round() as i64;
        from + Micros(steps.saturating_mul(EDGE_STEP.0))
    };
    match edge {
        LineEdge::Start => (
            snap(start).clamp(Micros::ZERO, (end - EDGE_STEP).max(Micros::ZERO)),
            end,
        ),
        LineEdge::End => (start, snap(end).max(start + EDGE_STEP)),
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct LineId(pub u64);

/// The look of the subtitles (one per project).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// White text with a black outline, at the bottom.
    #[default]
    Classic,
    /// White text on a semi-transparent box, in the lower third.
    Boxed,
    /// Bold white text; the word being spoken turns yellow; a few words at a time.
    WordHighlight,
    /// The word being spoken is drawn larger than the others (2.1.0, REQ-309).
    EnlargeWord,
    /// The word being spoken appears letter by letter (2.1.0, REQ-310).
    Typewriter,
}

impl Preset {
    pub const ALL: [Preset; 5] = [
        Preset::Classic,
        Preset::Boxed,
        Preset::WordHighlight,
        Preset::EnlargeWord,
        Preset::Typewriter,
    ];

    /// The styles that follow the spoken words one at a time.
    pub fn follows_words(self) -> bool {
        matches!(
            self,
            Preset::WordHighlight | Preset::EnlargeWord | Preset::Typewriter
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            Preset::Classic => "Classic",
            Preset::Boxed => "Box",
            Preset::WordHighlight => "Word highlight",
            Preset::EnlargeWord => "Enlarge word",
            Preset::Typewriter => "Typewriter",
        }
    }
}

/// Largest and smallest growth of the Enlarge word style, pixels at a picture height of 1080.
pub const ENLARGE_MIN: u8 = 1;
pub const ENLARGE_MAX: u8 = 40;

/// The settings of the project's one subtitle style that the presets do not fix (2.1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleSettings {
    /// Shadow distance and colour (REQ-307); 0 px is no shadow.
    #[serde(default)]
    pub shadow: crate::base::shadow::Shadow,
    /// The colour of the word being spoken (REQ-308); the 2.0.4 yellow by default.
    #[serde(default = "default_highlighted")]
    pub highlighted: [u8; 3],
    /// The colour of the other words (REQ-308); white by default.
    #[serde(default = "default_not_yet")]
    pub not_yet: [u8; 3],
    /// How much larger the spoken word is for Enlarge word (REQ-309).
    #[serde(default = "default_enlarge")]
    pub enlarge: u8,
}

fn default_highlighted() -> [u8; 3] {
    [255, 255, 0]
}
fn default_not_yet() -> [u8; 3] {
    [255, 255, 255]
}
fn default_enlarge() -> u8 {
    12
}

impl Default for StyleSettings {
    fn default() -> Self {
        StyleSettings {
            shadow: crate::base::shadow::Shadow::default(),
            highlighted: default_highlighted(),
            not_yet: default_not_yet(),
            enlarge: default_enlarge(),
        }
    }
}

impl StyleSettings {
    pub fn is_default(&self) -> bool {
        *self == StyleSettings::default()
    }

    /// The settings with every value inside its range.
    pub fn clamped(self) -> StyleSettings {
        StyleSettings {
            shadow: self.shadow.clamped(),
            enlarge: self.enlarge.clamp(ENLARGE_MIN, ENLARGE_MAX),
            ..self
        }
    }
}

/// Smallest, normal and largest text size of a line, in thousandths of the preset's size
/// (25%, 100%, 400%).
pub const SIZE_MIN: u16 = 250;
pub const SIZE_NORMAL: u16 = 1000;
pub const SIZE_MAX: u16 = 4000;

/// One recognised word with its time (source time in [`Line`], timeline time in
/// [`ShownLine`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    pub start: Micros,
    pub end: Micros,
    pub text: String,
}

/// A subtitle line, in the source time of `media`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub id: LineId,
    pub media: MediaId,
    pub start: Micros,
    pub end: Micros,
    pub text: String,
    /// Word timings from speech recognition (empty after a manual text edit).
    #[serde(default)]
    pub words: Vec<Word>,
    /// Changed by hand since it was generated.
    #[serde(default)]
    pub edited: bool,
    /// Own place on screen (1.2.0); `None` = the place of the style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Pos>,
    /// Own text size in thousandths of the preset's size, `SIZE_MIN..=SIZE_MAX` (1.2.6);
    /// `None` = 100%.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u16>,
    /// The point of the 360° world the line is fixed to (1.3.0); `None` = fixed to the
    /// picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<crate::base::pos::Anchor360>,
}

/// A line as generated by speech recognition (before it gets an id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLine {
    pub media: MediaId,
    pub start: Micros,
    pub end: Micros,
    pub text: String,
    pub words: Vec<Word>,
}

/// A line as it appears on the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownLine {
    pub line: LineId,
    pub clip: ClipId,
    pub start: Micros,
    pub end: Micros,
    pub text: String,
    pub words: Vec<Word>,
    /// The line's own place on screen (`None`: the style's place).
    pub position: Option<Pos>,
    /// The line's own text size (thousandths of the preset's; `None`: 100%).
    pub size: Option<u16>,
    /// The point of the 360° sphere the line is fixed to (`None`: fixed to the picture).
    pub anchor: Option<crate::base::pos::Anchor360>,
}

impl ShownLine {
    /// Index of the word being spoken at timeline time `t` (words spread evenly over the
    /// line when there are no word timings).
    pub fn word_at(&self, t: Micros) -> Option<usize> {
        if t < self.start || t >= self.end {
            return None;
        }
        if !self.words.is_empty() {
            return self.words.iter().position(|w| w.start <= t && t < w.end);
        }
        let n = self.text.split_whitespace().count() as i64;
        if n == 0 {
            return None;
        }
        let len = (self.end - self.start).0.max(1);
        Some((((t - self.start).0 * n) / len).min(n - 1) as usize)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum SubtitleError {
    #[error("the playhead is not inside a clip")]
    NotInsideClip,
    #[error("no such subtitle line")]
    NoSuchLine,
    #[error("the subtitle is not shown in that clip")]
    NotInClip,
    #[error("a subtitle must last at least one frame")]
    TooShort,
}

/// All the subtitles of a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Subtitles {
    /// Spoken language (ISO 639-1 code, e.g. `en`), detected or chosen.
    #[serde(default)]
    pub language: Option<String>,
    /// The user chose the language (it is not re-detected).
    #[serde(default)]
    pub language_overridden: bool,
    #[serde(default)]
    pub preset: Preset,
    /// Shadow, word colours and the Enlarge word size (2.1.0, REQ-306 to REQ-310).
    #[serde(default, skip_serializing_if = "StyleSettings::is_default")]
    pub style: StyleSettings,
    #[serde(default)]
    pub lines: Vec<Line>,
    #[serde(default)]
    pub next_id: u64,
}

/// Cleans user text: one line, no control characters, at most [`MAX_TEXT`] characters.
pub fn clean_text(text: &str) -> String {
    let one_line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let words: Vec<&str> = one_line.split_whitespace().collect();
    words.join(" ").chars().take(MAX_TEXT).collect()
}

impl Subtitles {
    fn new_id(&mut self) -> LineId {
        self.next_id += 1;
        LineId(self.next_id)
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn get(&self, id: LineId) -> Option<&Line> {
        self.lines.iter().find(|l| l.id == id)
    }

    fn get_mut(&mut self, id: LineId) -> Result<&mut Line, SubtitleError> {
        self.lines
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or(SubtitleError::NoSuchLine)
    }

    /// True when any line was changed by hand (regenerating would lose it).
    pub fn has_edits(&self) -> bool {
        self.lines.iter().any(|l| l.edited)
    }

    /// Lines as they appear on the timeline, in timeline order.
    pub fn project(&self, tl: &Timeline) -> Vec<ShownLine> {
        let mut out = Vec::new();
        let mut clip_start = Micros::ZERO;
        for c in &tl.clips {
            let map = |t: Micros| clip_start + c.to_timeline(t - c.src_in);
            for l in self.lines.iter().filter(|l| l.media == c.media) {
                if l.end <= c.src_in || l.start >= c.src_out {
                    continue;
                }
                let s = l.start.max(c.src_in);
                let e = l.end.min(c.src_out);
                let words = l
                    .words
                    .iter()
                    .filter(|w| w.end > c.src_in && w.start < c.src_out)
                    .map(|w| Word {
                        start: map(w.start.max(c.src_in)),
                        end: map(w.end.min(c.src_out)),
                        text: w.text.clone(),
                    })
                    .collect();
                out.push(ShownLine {
                    line: l.id,
                    clip: c.id,
                    start: map(s),
                    end: map(e),
                    text: l.text.clone(),
                    words,
                    position: l.position,
                    size: l.size,
                    anchor: l.anchor,
                });
            }
            clip_start = clip_start + c.len();
        }
        out.sort_by_key(|l| (l.start, l.line));
        out
    }

    /// Lines on screen at timeline time `t`.
    pub fn shown_at(&self, tl: &Timeline, t: Micros) -> Vec<ShownLine> {
        self.project(tl)
            .into_iter()
            .filter(|l| l.start <= t && t < l.end)
            .collect()
    }

    /// Adds a line at the playhead, in the clip under it (2 s, cut at the clip's end).
    pub fn add_at(
        &mut self,
        tl: &Timeline,
        t: Micros,
        text: &str,
        min_len: Micros,
    ) -> Result<LineId, SubtitleError> {
        let (i, start) = tl.locate(t).ok_or(SubtitleError::NotInsideClip)?;
        let c = tl.clips[i];
        let s = c.src_at(t - start);
        let e = (s + c.to_src(NEW_LINE_LEN)).min(c.src_out);
        if c.to_timeline(e - s) < min_len.max(Micros(1)) {
            return Err(SubtitleError::TooShort);
        }
        let id = self.new_id();
        self.lines.push(Line {
            id,
            media: c.media,
            start: s,
            end: e,
            text: clean_text(text),
            words: Vec::new(),
            edited: true,
            position: None,
            size: None,
            anchor: None,
        });
        Ok(id)
    }

    pub fn delete(&mut self, id: LineId) -> Result<(), SubtitleError> {
        let before = self.lines.len();
        self.lines.retain(|l| l.id != id);
        if self.lines.len() == before {
            return Err(SubtitleError::NoSuchLine);
        }
        Ok(())
    }

    /// Replaces the text. Word timings are dropped (the highlight then spreads evenly).
    pub fn set_text(&mut self, id: LineId, text: &str) -> Result<(), SubtitleError> {
        let l = self.get_mut(id)?;
        l.text = clean_text(text);
        l.words.clear();
        l.edited = true;
        Ok(())
    }

    /// Re-times a line given timeline times, converted through the clip it is shown in and
    /// kept inside that clip.
    pub fn retime(
        &mut self,
        id: LineId,
        tl: &Timeline,
        clip: ClipId,
        start: Micros,
        end: Micros,
        min_len: Micros,
    ) -> Result<(), SubtitleError> {
        let i = tl.index_of(clip).ok_or(SubtitleError::NotInClip)?;
        let c = tl.clips[i];
        let clip_start = tl.start_of(i);
        let line_media = self.get(id).ok_or(SubtitleError::NoSuchLine)?.media;
        if line_media != c.media {
            return Err(SubtitleError::NotInClip);
        }
        let to_src = |t: Micros| c.src_at(t - clip_start).clamp(c.src_in, c.src_out);
        let (s, e) = (to_src(start), to_src(end));
        if c.to_timeline(e - s) < min_len.max(Micros(1)) {
            return Err(SubtitleError::TooShort);
        }
        let l = self.get_mut(id)?;
        l.start = s;
        l.end = e;
        l.words.retain(|w| w.start >= s && w.end <= e);
        l.edited = true;
        Ok(())
    }

    /// Deletes several lines in one step (REQ-197); unknown ids are an error and nothing
    /// changes.
    pub fn delete_many(&mut self, ids: &[LineId]) -> Result<(), SubtitleError> {
        if ids.is_empty() || ids.iter().any(|id| self.get(*id).is_none()) {
            return Err(SubtitleError::NoSuchLine);
        }
        self.lines.retain(|l| !ids.contains(&l.id));
        Ok(())
    }

    /// Moves lines by `delta` together, keeping their lengths and their distances (REQ-197).
    /// The move is cut short so that no line starts before 0 s or ends after its file
    /// (`limit(media)`); the shift actually made is returned.
    pub fn shift(
        &mut self,
        ids: &[LineId],
        delta: Micros,
        limit: impl Fn(MediaId) -> Micros,
    ) -> Result<Micros, SubtitleError> {
        if ids.is_empty() || ids.iter().any(|id| self.get(*id).is_none()) {
            return Err(SubtitleError::NoSuchLine);
        }
        let picked = || self.lines.iter().filter(|l| ids.contains(&l.id));
        let earliest = picked().map(|l| l.start).min().unwrap_or(Micros::ZERO);
        let room = picked()
            .map(|l| limit(l.media) - l.end)
            .min()
            .unwrap_or(Micros::ZERO)
            .max(Micros::ZERO);
        let made = delta.max(Micros::ZERO - earliest).min(room);
        for l in self.lines.iter_mut().filter(|l| ids.contains(&l.id)) {
            l.start = l.start + made;
            l.end = l.end + made;
            for w in &mut l.words {
                w.start = w.start + made;
                w.end = w.end + made;
            }
            l.edited = true;
        }
        Ok(made)
    }

    /// Replaces every line with freshly generated ones.
    pub fn replace_all(&mut self, lines: Vec<NewLine>, language: Option<String>) {
        self.lines.clear();
        self.push_new(lines);
        if !self.language_overridden {
            self.language = language;
        }
    }

    fn push_new(&mut self, lines: Vec<NewLine>) {
        for n in lines {
            let id = self.new_id();
            self.lines.push(Line {
                id,
                media: n.media,
                start: n.start,
                end: n.end,
                text: clean_text(&n.text),
                words: n.words,
                edited: false,
                position: None,
                size: None,
                anchor: None,
            });
        }
    }

    /// True when a line of `media` overlapping source range `[from, to)` was changed by
    /// hand (creating subtitles for that range would replace it).
    pub fn has_edits_in(&self, media: MediaId, from: Micros, to: Micros) -> bool {
        self.lines
            .iter()
            .any(|l| l.edited && l.media == media && l.end > from && l.start < to)
    }

    /// Replaces the lines of `media` overlapping source range `[from, to)` with `lines`
    /// (kept inside the range; lines of other files and elsewhere are untouched).
    pub fn replace_range(
        &mut self,
        media: MediaId,
        from: Micros,
        to: Micros,
        lines: Vec<NewLine>,
        language: Option<String>,
    ) {
        self.lines
            .retain(|l| !(l.media == media && l.end > from && l.start < to));
        let kept: Vec<NewLine> = lines
            .into_iter()
            .filter(|n| n.media == media && n.end > from && n.start < to)
            .filter_map(|mut n| {
                n.start = n.start.max(from);
                n.end = n.end.min(to);
                if n.end <= n.start {
                    return None;
                }
                let (s, e) = (n.start, n.end);
                n.words.retain(|w| w.end > s && w.start < e);
                for w in &mut n.words {
                    w.start = w.start.clamp(s, e);
                    w.end = w.end.clamp(w.start, e);
                }
                Some(n)
            })
            .collect();
        self.push_new(kept);
        if !self.language_overridden && language.is_some() {
            self.language = language;
        }
    }

    /// Gives `ids` their own place on screen (`None`: back to the style's place). Unknown
    /// ids are an error and nothing changes.
    pub fn set_position(&mut self, ids: &[LineId], pos: Option<Pos>) -> Result<(), SubtitleError> {
        if ids.iter().any(|id| self.get(*id).is_none()) {
            return Err(SubtitleError::NoSuchLine);
        }
        let pos = pos.map(|p| Pos {
            x: p.x.min(POS_MAX),
            y: p.y.min(POS_MAX),
        });
        for l in self.lines.iter_mut().filter(|l| ids.contains(&l.id)) {
            l.position = pos;
        }
        Ok(())
    }

    /// Sets position and/or size of `ids` in one go: `None` leaves a value alone,
    /// `Some(None)` clears it. A size outside the limits is clamped. Unknown ids are an
    /// error and nothing changes.
    pub fn set_boxes(
        &mut self,
        ids: &[LineId],
        pos: Option<Option<Pos>>,
        size: Option<Option<u16>>,
    ) -> Result<(), SubtitleError> {
        if ids.iter().any(|id| self.get(*id).is_none()) {
            return Err(SubtitleError::NoSuchLine);
        }
        for l in self.lines.iter_mut().filter(|l| ids.contains(&l.id)) {
            if let Some(p) = pos {
                l.position = p.map(|p| Pos {
                    x: p.x.min(POS_MAX),
                    y: p.y.min(POS_MAX),
                });
            }
            if let Some(sz) = size {
                l.size = sz.map(|v| v.clamp(SIZE_MIN, SIZE_MAX));
            }
        }
        Ok(())
    }

    /// Back to the style's place and 100% for `ids`.
    pub fn reset(&mut self, ids: &[LineId]) -> Result<(), SubtitleError> {
        self.set_boxes(ids, Some(None), Some(None))
    }

    /// True when any of `ids` has an own position or size.
    pub fn any_moved_or_sized(&self, ids: &[LineId]) -> bool {
        self.lines
            .iter()
            .any(|l| ids.contains(&l.id) && (l.position.is_some() || l.size.is_some()))
    }

    /// Words stored in all lines.
    pub fn word_count(&self) -> usize {
        self.lines.iter().map(|l| l.words.len()).sum()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::timeline::tests::media;

    fn s(n: i64) -> Micros {
        Micros(n * 1_000_000)
    }

    fn ms(n: i64) -> Micros {
        Micros(n * 1_000)
    }

    pub fn word(a: i64, b: i64, t: &str) -> Word {
        Word {
            start: ms(a),
            end: ms(b),
            text: t.into(),
        }
    }

    /// One 10 s clip of media 1 with two generated lines (1–3 s and 6–8 s).
    pub fn setup() -> (Timeline, Subtitles) {
        let mut tl = Timeline::default();
        tl.append(&media(1, 10, true)).unwrap();
        let mut subs = Subtitles::default();
        subs.replace_all(
            vec![
                NewLine {
                    media: MediaId(1),
                    start: s(1),
                    end: s(3),
                    text: "hello world".into(),
                    words: vec![word(1000, 1800, "hello"), word(2000, 3000, "world")],
                },
                NewLine {
                    media: MediaId(1),
                    start: s(6),
                    end: s(8),
                    text: "second line".into(),
                    words: vec![],
                },
            ],
            Some("en".into()),
        );
        (tl, subs)
    }

    const F: Micros = Micros(33_333);

    // covers: REQ-018
    #[test]
    fn lines_follow_cut_copy_trim_and_split() {
        let (mut tl, subs) = setup();
        assert_eq!(subs.project(&tl).len(), 2);
        // split keeps every line in place
        tl.split(s(5), F).unwrap();
        let shown = subs.project(&tl);
        assert_eq!(
            shown.iter().map(|l| (l.start, l.end)).collect::<Vec<_>>(),
            vec![(s(1), s(3)), (s(6), s(8))]
        );
        // move the second half first: its line moves with it
        let second = tl.clips[1].id;
        tl.move_to(second, 0).unwrap();
        let shown = subs.project(&tl);
        assert_eq!((shown[0].start, shown[0].end), (s(1), s(3)));
        assert_eq!(shown[0].text, "second line");
        assert_eq!((shown[1].start, shown[1].end), (s(6), s(8)));
        // copy (paste a duplicate of the first 5 s at the end): the line is duplicated
        let first = *tl.get(tl.clips[1].id).unwrap();
        tl.insert_at(&first, tl.duration(), F).unwrap();
        let shown = subs.project(&tl);
        assert_eq!(shown.iter().filter(|l| l.text == "hello world").count(), 2);
        assert_eq!(shown.last().unwrap().start, s(11));
        // cut removes them
        let dup = tl.clips[2].id;
        tl.remove(dup).unwrap();
        assert_eq!(subs.project(&tl).len(), 2);
        // trim hides what is outside the kept range and clips words
        let c = tl.clips[1].id;
        tl.trim(c, s(2), s(5), s(10), F).unwrap();
        let shown = subs.project(&tl);
        let hello = shown.iter().find(|l| l.text == "hello world").unwrap();
        assert_eq!((hello.start, hello.end), (s(5), s(6)));
        assert_eq!(hello.words.len(), 1);
        assert_eq!(hello.words[0].text, "world");
        tl.trim(c, s(4), s(5), s(10), F).unwrap();
        assert!(subs.project(&tl).iter().all(|l| l.text != "hello world"));
        // shown_at
        assert_eq!(subs.shown_at(&tl, s(1)).len(), 1);
        assert!(subs.shown_at(&tl, s(4)).is_empty());
    }

    // covers: REQ-016
    #[test]
    fn add_delete_retext_retime() {
        let (tl, mut subs) = setup();
        assert!(!subs.has_edits());
        let id = subs.add_at(&tl, s(9), "  new \n line ", F).unwrap();
        let l = subs.get(id).unwrap();
        assert_eq!((l.start, l.end), (s(9), s(10)), "cut at the clip end");
        assert_eq!(l.text, "new line");
        assert!(subs.has_edits());
        assert_eq!(
            subs.add_at(&tl, s(99), "x", F),
            Err(SubtitleError::NotInsideClip)
        );
        assert_eq!(
            subs.add_at(&tl, Micros(9_999_990), "x", F),
            Err(SubtitleError::TooShort)
        );
        // re-text drops word timings and marks the line as edited
        let first = subs.lines[0].id;
        subs.set_text(first, "hi there").unwrap();
        assert_eq!(subs.get(first).unwrap().text, "hi there");
        assert!(subs.get(first).unwrap().words.is_empty());
        assert_eq!(
            subs.set_text(LineId(99), "x"),
            Err(SubtitleError::NoSuchLine)
        );
        // re-time through the clip, clamped to it
        let clip = tl.clips[0].id;
        subs.retime(first, &tl, clip, s(-3), s(4), F).unwrap();
        let l = subs.get(first).unwrap();
        assert_eq!((l.start, l.end), (Micros::ZERO, s(4)));
        assert_eq!(
            subs.retime(first, &tl, clip, s(4), s(4), F),
            Err(SubtitleError::TooShort)
        );
        assert_eq!(
            subs.retime(first, &tl, ClipId(99), s(1), s(2), F),
            Err(SubtitleError::NotInClip)
        );
        assert_eq!(
            subs.retime(LineId(99), &tl, clip, s(1), s(2), F),
            Err(SubtitleError::NoSuchLine)
        );
        // delete
        subs.delete(id).unwrap();
        assert_eq!(subs.delete(id), Err(SubtitleError::NoSuchLine));
        assert_eq!(subs.lines.len(), 2);
        assert_eq!(subs.word_count(), 0);
        assert!(!subs.is_empty());
    }

    #[test]
    fn retime_refuses_a_clip_of_another_file() {
        let (mut tl, mut subs) = setup();
        tl.append(&media(2, 5, true)).unwrap();
        let other = tl.clips[1].id;
        let first = subs.lines[0].id;
        assert_eq!(
            subs.retime(first, &tl, other, s(11), s(12), F),
            Err(SubtitleError::NotInClip)
        );
    }

    #[test]
    fn word_highlight_index() {
        let (tl, subs) = setup();
        let shown = subs.project(&tl);
        assert_eq!(shown[0].word_at(ms(1500)), Some(0));
        assert_eq!(shown[0].word_at(ms(2500)), Some(1));
        assert_eq!(shown[0].word_at(ms(1900)), None);
        assert_eq!(shown[0].word_at(s(5)), None);
        // no timings: even spread
        assert_eq!(shown[1].word_at(ms(6100)), Some(0));
        assert_eq!(shown[1].word_at(ms(7900)), Some(1));
        let empty = ShownLine {
            text: String::new(),
            ..shown[1].clone()
        };
        assert_eq!(empty.word_at(ms(6100)), None);
        assert_eq!(clean_text(&"x".repeat(MAX_TEXT + 5)).len(), MAX_TEXT);
        assert_eq!(Preset::ALL.len(), 5);
        assert_eq!(Preset::WordHighlight.name(), "Word highlight");
        assert_eq!(Preset::default(), Preset::Classic);
    }

    #[test]
    fn regenerating_keeps_an_overridden_language() {
        let (_, mut subs) = setup();
        assert_eq!(subs.language.as_deref(), Some("en"));
        subs.language = Some("it".into());
        subs.language_overridden = true;
        subs.replace_all(vec![], Some("en".into()));
        assert_eq!(subs.language.as_deref(), Some("it"));
        assert!(subs.is_empty());
    }

    // covers: REQ-085, REQ-044
    #[test]
    fn creating_for_a_range_replaces_only_that_range() {
        let (_, mut subs) = setup();
        subs.lines.push(Line {
            id: LineId(50),
            media: MediaId(2),
            start: s(1),
            end: s(2),
            text: "other file".into(),
            words: vec![],
            edited: true,
            position: None,
            size: None,
            anchor: None,
        });
        assert!(subs.has_edits_in(MediaId(2), s(0), s(5)));
        assert!(!subs.has_edits_in(MediaId(1), s(0), s(10)));
        assert!(!subs.has_edits_in(MediaId(2), s(2), s(5)));
        // replace [0.5 s, 4 s) of media 1: the 1–3 s line goes, 6–8 s stays
        subs.replace_range(
            MediaId(1),
            ms(500),
            s(4),
            vec![
                NewLine {
                    media: MediaId(1),
                    start: Micros::ZERO,
                    end: s(2),
                    text: "new".into(),
                    words: vec![word(0, 900, "early"), word(1000, 1500, "new")],
                },
                NewLine {
                    media: MediaId(1),
                    start: s(5),
                    end: s(6),
                    text: "outside".into(),
                    words: vec![],
                },
                NewLine {
                    media: MediaId(9),
                    start: s(1),
                    end: s(2),
                    text: "wrong file".into(),
                    words: vec![],
                },
            ],
            Some("it".into()),
        );
        let texts: Vec<&str> = subs.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["second line", "other file", "new"]);
        let new = subs.lines.last().unwrap();
        assert_eq!(
            (new.start, new.end),
            (ms(500), s(2)),
            "kept inside the range"
        );
        assert_eq!(new.words[0].start, ms(500));
        assert!(!new.edited);
        assert_eq!(subs.language.as_deref(), Some("it"));
        // a language chosen by the user stays
        subs.language_overridden = true;
        subs.replace_range(MediaId(1), s(0), s(1), vec![], Some("de".into()));
        assert_eq!(subs.language.as_deref(), Some("it"));
    }

    // covers: REQ-089, REQ-092, REQ-018
    #[test]
    fn positions_belong_to_lines_and_follow_them() {
        let (mut tl, mut subs) = setup();
        let (a, b) = (subs.lines[0].id, subs.lines[1].id);
        let p = Pos { x: 5000, y: 2000 };
        subs.set_position(&[a, b], Some(p)).unwrap();
        assert!(subs.lines.iter().all(|l| l.position == Some(p)));
        assert_eq!(
            subs.set_position(&[a, LineId(99)], None),
            Err(SubtitleError::NoSuchLine)
        );
        assert_eq!(subs.get(a).unwrap().position, Some(p), "nothing changed");
        // values past the edge are kept on the picture
        subs.set_position(&[b], Some(Pos { x: 60_000, y: 7 }))
            .unwrap();
        assert_eq!(
            subs.get(b).unwrap().position,
            Some(Pos { x: POS_MAX, y: 7 })
        );
        // a copied clip shows the same line at the same place
        let first = tl.clips[0];
        tl.insert_at(&first, tl.duration(), F).unwrap();
        let shown = subs.project(&tl);
        assert_eq!(shown.iter().filter(|l| l.position == Some(p)).count(), 2);
        subs.set_position(&[a], None).unwrap();
        assert_eq!(subs.get(a).unwrap().position, None);
        assert_eq!(Pos::from_fraction(0.25, 2.0), Pos { x: 2500, y: 10_000 });
        assert_eq!(Pos::from_fraction(f64::NAN, 0.0), Pos { x: 5000, y: 0 });
        assert_eq!(Pos { x: 2500, y: 10_000 }.fraction(), (0.25, 1.0));
        assert!(!Pos { x: 10_001, y: 0 }.is_valid());
        // stored only when set: 1.1 files read with no position
        let json = serde_json::to_string(subs.get(a).unwrap()).unwrap();
        assert!(!json.contains("position"));
    }

    // covers: REQ-097, REQ-099
    #[test]
    fn sizes_are_per_line_clamped_and_reset_with_the_position() {
        let (_, mut subs) = setup();
        let (a, b) = (subs.lines[0].id, subs.lines[1].id);
        let p = Pos { x: 100, y: 200 };
        subs.set_boxes(&[a, b], Some(Some(p)), Some(Some(2000)))
            .unwrap();
        assert!(
            subs.lines
                .iter()
                .all(|l| l.size == Some(2000) && l.position == Some(p))
        );
        // only the size: the position stays
        subs.set_boxes(&[a], None, Some(Some(60_000))).unwrap();
        assert_eq!(subs.get(a).unwrap().size, Some(SIZE_MAX));
        assert_eq!(subs.get(a).unwrap().position, Some(p));
        subs.set_boxes(&[a], None, Some(Some(1))).unwrap();
        assert_eq!(subs.get(a).unwrap().size, Some(SIZE_MIN));
        assert_eq!(
            subs.set_boxes(&[a, LineId(99)], None, Some(None)),
            Err(SubtitleError::NoSuchLine)
        );
        assert_eq!(subs.get(a).unwrap().size, Some(SIZE_MIN), "nothing changed");
        assert!(subs.any_moved_or_sized(&[a]));
        subs.reset(&[a]).unwrap();
        let l = subs.get(a).unwrap();
        assert_eq!((l.size, l.position), (None, None));
        assert!(subs.any_moved_or_sized(&[a, b]));
        subs.reset(&[b]).unwrap();
        assert!(!subs.any_moved_or_sized(&[a, b]));
        // stored only when set; shown lines carry it
        let json = serde_json::to_string(subs.get(a).unwrap()).unwrap();
        assert!(!json.contains("size"));
        subs.set_boxes(&[a], None, Some(Some(1500))).unwrap();
        let tl = setup().0;
        assert!(subs.project(&tl).iter().any(|l| l.size == Some(1500)));
    }

    // covers: REQ-192
    #[test]
    fn a_dragged_marker_moves_in_tenths_of_a_second_and_never_crosses_the_other() {
        let (a, b) = (s(1), s(3));
        let ms = |n: i64| Micros(n * 1000);
        // the start 0.5 s later
        assert_eq!(dragged_edge(a, b, LineEdge::Start, ms(1500)), (ms(1500), b));
        // snapped to steps from where it was
        assert_eq!(dragged_edge(a, b, LineEdge::Start, ms(1530)), (ms(1500), b));
        assert_eq!(dragged_edge(a, b, LineEdge::Start, ms(1560)), (ms(1600), b));
        // past the end: stops 0.1 s before it
        assert_eq!(dragged_edge(a, b, LineEdge::Start, s(9)), (ms(2900), b));
        // before 0 s: stops at 0
        assert_eq!(
            dragged_edge(a, b, LineEdge::Start, Micros(-9_000_000)).0,
            Micros::ZERO
        );
        // the end marker: later, and never before the start plus a step
        assert_eq!(dragged_edge(a, b, LineEdge::End, ms(3700)), (a, ms(3700)));
        assert_eq!(
            dragged_edge(a, b, LineEdge::End, Micros::ZERO),
            (a, ms(1100))
        );
        // the drag moves the line in the project and one undo restores it
        let (tl, mut subs) = setup();
        let id = subs.lines[0].id;
        let clip = tl.clips[0].id;
        let (st, en) = dragged_edge(s(1), s(3), LineEdge::Start, ms(1500));
        subs.retime(id, &tl, clip, st, en, Micros(33_333)).unwrap();
        assert_eq!((subs.lines[0].start, subs.lines[0].end), (ms(1500), s(3)));
    }

    // covers: REQ-197
    #[test]
    fn lines_are_deleted_together_and_shifted_keeping_lengths_and_never_before_zero() {
        let (_, mut subs) = setup();
        let ids: Vec<LineId> = subs.lines.iter().map(|l| l.id).collect();
        let limit = |_: MediaId| s(10);
        // later by 0.5 s: both move, lengths kept
        let made = subs.shift(&ids, SHIFT_STEP, limit).unwrap();
        assert_eq!(made, SHIFT_STEP);
        assert_eq!(
            (subs.lines[0].start, subs.lines[0].end),
            (Micros(1_500_000), Micros(3_500_000))
        );
        assert_eq!(subs.lines[0].words[0].start, Micros(1_500_000));
        assert!(subs.lines.iter().all(|l| l.edited));
        // earlier: stops when the first line reaches 0 s (the group keeps its shape)
        let made = subs.shift(&ids, Micros(-9_000_000), limit).unwrap();
        assert_eq!(made, Micros(-1_500_000));
        assert_eq!(subs.lines[0].start, Micros::ZERO);
        assert_eq!(subs.lines[1].start, Micros(5_000_000));
        // later: stops when the last line reaches the end of its file
        let made = subs.shift(&ids, s(99), limit).unwrap();
        assert_eq!(subs.lines[1].end, s(10));
        assert_eq!(made, Micros(10_000_000 - 7_000_000));
        // unknown or no lines: an error, nothing changes
        let before = subs.clone();
        assert!(subs.shift(&[LineId(99)], SHIFT_STEP, limit).is_err());
        assert!(subs.shift(&[], SHIFT_STEP, limit).is_err());
        assert_eq!(subs, before);
        // Delete: all of them or none
        assert!(subs.delete_many(&[ids[0], LineId(99)]).is_err());
        assert_eq!(subs.lines.len(), 2);
        subs.delete_many(&ids).unwrap();
        assert!(subs.lines.is_empty());
        assert!(subs.delete_many(&[]).is_err());
    }
}
