// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Text items (REQ-108 to REQ-111): a text shown for a while at a place of the picture,
//! with fade in, fade out, blink and slide effects, a colour and a font. Items live on
//! the **timeline** (they do not move when the video is edited, like music). Everything
//! here is pure: the model, its validation (projects are untrusted input) and the ASS
//! events that burn the text in, used by the preview and the export alike.

use crate::ass::{self, AssEvent, font_px};
pub use crate::base::font::{DEFAULT_FONT, MAX_FONT, valid_font};
use crate::base::pos::{Anchor360, POS_MAX, Pos};
use crate::fonts;
use crate::measure::{self, Shaped};
use crate::subtitles::{MAX_TEXT, SIZE_NORMAL};
use crate::time::Micros;
use serde::{Deserialize, Serialize};

/// Shortest time to live.
pub const MIN_TTL: Micros = Micros(100_000);
/// Time to live of a new text.
pub const DEFAULT_TTL: Micros = Micros(3_000_000);
/// Shortest and longest setting of an effect.
pub const EFFECT_MIN: Micros = Micros(100_000);
pub const EFFECT_MAX: Micros = Micros(5_000_000);
/// Setting of an effect when it is switched on.
pub const EFFECT_DEFAULT: Micros = Micros(500_000);
/// Longest time to live a file may claim (a day): the video length is only a limit when
/// editing, because a video may be cut shorter after the text was placed.
pub const MAX_TTL: Micros = Micros(86_400_000_000);
/// Narrowest and widest box of a text, in thousandths of the picture width (REQ-116).
pub const WIDTH_MIN: u16 = 100;
pub const WIDTH_MAX: u16 = 1000;
/// Width of a new text (80% of the picture width).
pub const DEFAULT_WIDTH: u16 = 800;
/// Lowest, highest and default height of a text box, in thousandths of the picture height
/// (REQ-159: 5% to 100%; a new box is 20% high).
pub const HEIGHT_MIN: u16 = 50;
pub const HEIGHT_MAX: u16 = 1000;
pub const DEFAULT_HEIGHT: u16 = 200;
/// Smallest best-fit text height, in thousandths of the picture height (2%).
pub const MIN_FIT: u32 = measure::MIN_SIZE;
/// Largest text height, in thousandths of the picture height (REQ-157: 10%).
pub const MAX_FIT: u32 = measure::MAX_SIZE;
/// Limits of the ASS events of one item and of a project.
pub const MAX_EVENTS_ITEM: usize = 2000;
pub const MAX_EVENTS_TOTAL: usize = 20_000;
/// Most lines of a text and longest family name.
pub const MAX_LINES: usize = 200;
/// Text items of a project.
pub const MAX_ITEMS: usize = 5000;
/// Where a new text starts: centred, in the lower third.
pub const DEFAULT_PLACE: Pos = Pos { x: 5000, y: 8300 };

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct TextId(pub u64);

/// Where a sliding text comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlideDir {
    Left,
    Right,
    Top,
    Bottom,
}

impl SlideDir {
    pub const ALL: [SlideDir; 4] = [
        SlideDir::Left,
        SlideDir::Right,
        SlideDir::Top,
        SlideDir::Bottom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SlideDir::Left => "from left",
            SlideDir::Right => "from right",
            SlideDir::Top => "from top",
            SlideDir::Bottom => "from bottom",
        }
    }
}

/// The four effects of a text; each has its own time when on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TextEffects {
    #[serde(default)]
    pub fade_in: Option<Micros>,
    #[serde(default)]
    pub fade_out: Option<Micros>,
    #[serde(default)]
    pub blink: Option<Micros>,
    #[serde(default)]
    pub slide: Option<(SlideDir, Micros)>,
}

impl TextEffects {
    pub fn any(&self) -> bool {
        self.fade_in.is_some()
            || self.fade_out.is_some()
            || self.blink.is_some()
            || self.slide.is_some()
    }
}

/// What keeps a text readable on a busy picture (REQ-158).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Readability {
    /// Nothing around the letters (what a text of a project saved before 2.0.3 opens as).
    #[default]
    None,
    /// A black outline, 6% of the text size wide.
    Outline,
    /// A black box behind the lines, 60% opaque, padded by 10% of the text size.
    Box,
}

impl Readability {
    pub const ALL: [Readability; 3] = [Readability::None, Readability::Outline, Readability::Box];

    pub fn name(self) -> &'static str {
        match self {
            Readability::None => "None",
            Readability::Outline => "Outline",
            Readability::Box => "Background box",
        }
    }

    /// Width in pixels of the outline, or the padding of the box, at text size `px`.
    pub fn border(self, px: f64) -> f64 {
        match self {
            Readability::None => 0.0,
            Readability::Outline => (px * 0.06).round().max(1.0),
            Readability::Box => (px * 0.10).round().max(1.0),
        }
    }
}

fn default_height() -> u16 {
    DEFAULT_HEIGHT
}

/// One text on the timeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextItem {
    pub id: TextId,
    /// Lines separated by `\n`.
    pub text: String,
    /// Timeline time of the first picture.
    pub start: Micros,
    /// Time to live.
    pub ttl: Micros,
    #[serde(default)]
    pub effects: TextEffects,
    pub colour: [u8; 3],
    /// Family name; empty = the bundled font.
    #[serde(default)]
    pub font: String,
    /// Centre of the text box (flat picture).
    pub place: Pos,
    /// Width of the box in thousandths of the picture width, `WIDTH_MIN..=WIDTH_MAX`
    /// (1.4.0). The text size is the best fit of that box ([`layout`]).
    pub width: u16,
    /// Height of the box in thousandths of the picture height, `HEIGHT_MIN..=HEIGHT_MAX`
    /// (2.0.3, REQ-159); a project saved before has 20%.
    #[serde(default = "default_height")]
    pub height: u16,
    /// Outline or background box (2.0.3, REQ-158); a project saved before has none.
    #[serde(default)]
    pub readability: Readability,
    /// The shadow of the letters or of the box (2.1.0, REQ-306); a project saved before has none.
    #[serde(default, skip_serializing_if = "crate::base::shadow::Shadow::is_none")]
    pub shadow: crate::base::shadow::Shadow,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor360>,
    /// A picture item (2.0.4, REQ-287): the image is drawn in the box instead of text; the
    /// text is its file name, shown on the timeline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<Box<crate::picture::Picture>>,
    /// The duration of each word in hundredths of a second, for the word highlight of a
    /// subtitle line fixed to the sphere (REQ-257); never saved, empty for a text item.
    #[serde(skip)]
    pub karaoke: Vec<u32>,
}

/// The text with line breaks normalised and control characters removed; `None` when
/// nothing is left or it is too long.
pub fn clean_text(text: &str) -> Option<String> {
    let t = text.replace("\r\n", "\n").replace('\r', "\n");
    let t: String = t
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .collect();
    let t = t.trim_matches('\n').to_string();
    let lines = t.split('\n');
    if t.trim().is_empty()
        || lines.clone().count() > MAX_LINES
        || lines.into_iter().any(|l| l.chars().count() > MAX_TEXT)
    {
        return None;
    }
    Some(t)
}

impl TextItem {
    /// A new item at `start` with the defaults of REQ-108 (white, 3 s, lower third).
    pub fn new(id: TextId, text: String, start: Micros) -> TextItem {
        TextItem {
            id,
            text,
            start,
            ttl: DEFAULT_TTL,
            effects: TextEffects::default(),
            colour: [255, 255, 255],
            font: String::new(),
            place: DEFAULT_PLACE,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            readability: Readability::Outline,
            shadow: crate::base::shadow::Shadow::default(),
            anchor: None,
            picture: None,
            karaoke: Vec::new(),
        }
    }

    /// A new picture item at `start`: 3 s, a box with the picture's proportions in the
    /// middle of a `frame`-sized picture (REQ-287).
    pub fn new_picture(
        id: TextId,
        picture: crate::picture::Picture,
        start: Micros,
        frame: (u32, u32),
    ) -> TextItem {
        let (w, h) = picture.default_box(frame);
        let mut t = TextItem::new(id, picture.name(), start);
        t.text = clean_text(&t.text).unwrap_or_else(|| "picture".into());
        t.place = Pos { x: 5000, y: 5000 };
        t.width = w.clamp(WIDTH_MIN, WIDTH_MAX);
        t.height = h.clamp(HEIGHT_MIN, HEIGHT_MAX);
        t.picture = Some(Box::new(picture));
        t
    }

    pub fn end(&self) -> Micros {
        self.start + self.ttl
    }

    /// Checks every value of an item (a project file is untrusted input).
    pub fn validate(&self) -> Result<(), String> {
        if clean_text(&self.text).as_deref() != Some(self.text.as_str()) {
            return Err("text item with an invalid text".into());
        }
        if self.start < Micros::ZERO || self.start > MAX_TTL {
            return Err("text item starting outside the timeline".into());
        }
        if self.ttl < MIN_TTL || self.ttl > MAX_TTL {
            return Err("text item with an invalid time to live".into());
        }
        let ok = |t: Micros| (EFFECT_MIN..=EFFECT_MAX).contains(&t);
        let e = &self.effects;
        if [e.fade_in, e.fade_out, e.blink, e.slide.map(|s| s.1)]
            .into_iter()
            .flatten()
            .any(|t| !ok(t))
        {
            return Err("text effect time outside 0.1 to 5 s".into());
        }
        if !valid_font(&self.font) {
            return Err("text item with an invalid font name".into());
        }
        if !self.place.is_valid()
            || !(WIDTH_MIN..=WIDTH_MAX).contains(&self.width)
            || !(HEIGHT_MIN..=HEIGHT_MAX).contains(&self.height)
        {
            return Err("text item with an invalid place or box".into());
        }
        if self.anchor.is_some_and(|a| !a.is_valid()) {
            return Err("text item with an invalid 360 anchor".into());
        }
        if self.picture.as_ref().is_some_and(|p| !p.is_valid()) {
            return Err("text item with an invalid picture".into());
        }
        if event_count(self) > MAX_EVENTS_ITEM {
            return Err("text item with too many blinks".into());
        }
        Ok(())
    }
}

/// The text items of a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Texts {
    pub items: Vec<TextItem>,
    pub next_id: u64,
}

/// Why an edit of the text items was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TextError {
    #[error("no such text")]
    Unknown,
    #[error("{0}")]
    Invalid(String),
}

impl Texts {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, id: TextId) -> Option<&TextItem> {
        self.items.iter().find(|t| t.id == id)
    }

    /// Reserves a new id.
    pub fn new_id(&mut self) -> TextId {
        self.next_id += 1;
        TextId(self.next_id)
    }

    /// Adds an item; refused when it, or the project, would carry too many events.
    pub fn add(&mut self, item: TextItem) -> Result<TextId, TextError> {
        item.validate().map_err(TextError::Invalid)?;
        if self.items.len() >= MAX_ITEMS
            || self.total_events() + event_count(&item) > MAX_EVENTS_TOTAL
        {
            return Err(TextError::Invalid(
                "too many text effects in this project".into(),
            ));
        }
        let id = item.id;
        self.next_id = self.next_id.max(id.0);
        self.items.push(item);
        Ok(id)
    }

    /// Replaces the item with the same id.
    pub fn replace(&mut self, item: TextItem) -> Result<(), TextError> {
        item.validate().map_err(TextError::Invalid)?;
        let i = self
            .items
            .iter()
            .position(|t| t.id == item.id)
            .ok_or(TextError::Unknown)?;
        let others: usize = self
            .items
            .iter()
            .filter(|t| t.id != item.id)
            .map(event_count)
            .sum();
        if others + event_count(&item) > MAX_EVENTS_TOTAL {
            return Err(TextError::Invalid(
                "too many text effects in this project".into(),
            ));
        }
        self.items[i] = item;
        Ok(())
    }

    pub fn remove(&mut self, id: TextId) -> Result<(), TextError> {
        let i = self
            .items
            .iter()
            .position(|t| t.id == id)
            .ok_or(TextError::Unknown)?;
        self.items.remove(i);
        Ok(())
    }

    pub fn total_events(&self) -> usize {
        self.items.iter().map(event_count).sum()
    }

    /// The items shown at timeline time `t`.
    pub fn shown_at(&self, t: Micros) -> Vec<&TextItem> {
        self.items
            .iter()
            .filter(|i| t >= i.start && t < i.end())
            .collect()
    }

    /// Checks a whole list read from a file.
    pub fn validate(&self) -> Result<(), String> {
        if self.items.len() > MAX_ITEMS {
            return Err("too many text items".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for i in &self.items {
            if !seen.insert(i.id) {
                return Err("duplicate text id".into());
            }
            i.validate()?;
        }
        if self.total_events() > MAX_EVENTS_TOTAL {
            return Err("too many text effects in the project".into());
        }
        Ok(())
    }
}

mod events;
mod layout;
mod timing;

pub use events::*;
pub use layout::*;
pub use timing::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> TextItem {
        TextItem::new(TextId(1), "Hello\nworld".into(), Micros(4_000_000))
    }

    fn secs(s: f64) -> Micros {
        Micros::from_secs_f64(s)
    }

    // covers: REQ-108
    #[test]
    fn a_new_item_has_the_defaults() {
        let i = item();
        assert_eq!(i.ttl, DEFAULT_TTL);
        assert_eq!(i.colour, [255, 255, 255]);
        assert_eq!(i.place, DEFAULT_PLACE);
        assert_eq!(i.end(), Micros(7_000_000));
        i.validate().unwrap();
        assert!(!i.effects.any());
        assert_eq!(SlideDir::Left.name(), "from left");
        assert_eq!(SlideDir::ALL.len(), 4);
    }

    // covers: REQ-108
    #[test]
    fn text_is_cleaned_and_limited() {
        assert_eq!(clean_text("a\r\nb\u{7}c"), Some("a\nbc".into()));
        assert_eq!(clean_text("\n\n x \n"), Some(" x ".into()));
        assert_eq!(clean_text("   "), None);
        assert_eq!(clean_text(""), None);
        assert_eq!(clean_text(&"x".repeat(MAX_TEXT + 1)), None);
        assert_eq!(clean_text(&"x\n".repeat(MAX_LINES + 1)), None);
        assert!(clean_text(&"x".repeat(MAX_TEXT)).is_some());
    }

    // covers: REQ-109
    #[test]
    fn a_plain_fade_is_one_event_with_fad_limited_to_half_the_ttl() {
        let mut i = item();
        i.effects.fade_in = Some(secs(1.0));
        i.effects.fade_out = Some(secs(5.0));
        let (fin, fout) = fades(&i);
        assert_eq!((fin, fout), (secs(1.0), secs(1.5)));
        let ev = events(&i, 1920, 1080, (400.0, 100.0), secs(100.0));
        assert_eq!(ev.len(), 1);
        assert_eq!((ev[0].start, ev[0].end), (secs(4.0), secs(7.0)));
        assert!(ev[0].tags.contains("\\fad(1000,1500)"), "{}", ev[0].tags);
        assert_eq!(ev[0].text, "Hello\\Nworld");
        // 50% at 0.5 s of a 1 s fade
        let mut j = item();
        j.effects.fade_in = Some(secs(1.0));
        assert!((alpha_at(&j, secs(0.5)) - 0.5).abs() < 0.05);
        assert_eq!(alpha_at(&j, secs(2.0)), 1.0);
        assert_eq!(alpha_at(&j, secs(3.0)), 0.0);
        assert_eq!(alpha_at(&j, Micros(-1)), 0.0);
    }

    // covers: REQ-109
    #[test]
    fn blink_makes_one_event_per_shown_phase() {
        let mut i = item();
        i.ttl = secs(3.0);
        i.effects.blink = Some(secs(0.5));
        let p = phases(&i);
        assert_eq!(
            p,
            [
                (secs(0.0), secs(0.5)),
                (secs(1.0), secs(1.5)),
                (secs(2.0), secs(2.5))
            ]
        );
        assert_eq!(event_count(&i), 3);
        // hidden between 0.5 s and 1.0 s
        assert_eq!(alpha_at(&i, secs(0.75)), 0.0);
        assert_eq!(alpha_at(&i, secs(0.25)), 1.0);
        assert_eq!(alpha_at(&i, secs(1.25)), 1.0);
        let ev = events(&i, 1280, 720, (100.0, 40.0), secs(100.0));
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[1].start, secs(5.0));
        assert!(!ev[1].tags.contains("\\fad"));
        // the last phase is cut by the end of the video
        let cut = events(&i, 1280, 720, (100.0, 40.0), secs(6.2));
        assert_eq!(cut.len(), 3);
        assert_eq!(cut[2].end, secs(6.2));
        let none = events(&i, 1280, 720, (100.0, 40.0), secs(4.5));
        assert_eq!(none.len(), 1);
    }

    // covers: REQ-109
    #[test]
    fn blink_with_fades_uses_alpha_steps_per_phase() {
        let mut i = item();
        i.ttl = secs(3.0);
        i.effects.blink = Some(secs(0.5));
        i.effects.fade_in = Some(secs(1.0));
        i.effects.fade_out = Some(secs(1.0));
        let ev = events(&i, 1280, 720, (100.0, 40.0), secs(100.0));
        // first phase [0,0.5): starts invisible, rises to 50%
        assert!(ev[0].tags.contains("\\alpha&HFF&"), "{}", ev[0].tags);
        assert!(
            ev[0].tags.contains("\\t(0,500,\\alpha&H80&)"),
            "{}",
            ev[0].tags
        );
        // second phase [1,1.5): after the fade in, before the fade out: no alpha
        assert!(!ev[1].tags.contains("\\alpha"), "{}", ev[1].tags);
        // last phase [2,2.5): fades out from full to 25%.. at 2.5 s
        assert!(ev[2].tags.contains("\\t(0,500,\\alpha&H"), "{}", ev[2].tags);
    }

    // covers: REQ-109
    #[test]
    fn slide_starts_one_box_outside_and_reaches_the_place() {
        let mut i = item();
        i.place = Pos { x: 5000, y: 5000 };
        i.effects.slide = Some((SlideDir::Left, secs(0.5)));
        let ev = events(&i, 1000, 500, (400.0, 100.0), secs(100.0));
        // the box (400 wide) starts 400 px left of its place and fully outside the picture
        assert!(
            ev[0].tags.contains("\\move(-200,250,500,250,0,500)"),
            "{}",
            ev[0].tags
        );
        i.effects.slide = Some((SlideDir::Right, secs(1.0)));
        let ev = events(&i, 1000, 500, (400.0, 100.0), secs(100.0));
        assert!(
            ev[0].tags.contains("\\move(1200,250,500,250,0,1000)"),
            "{}",
            ev[0].tags
        );
        i.effects.slide = Some((SlideDir::Top, secs(1.0)));
        let ev = events(&i, 1000, 500, (400.0, 100.0), secs(100.0));
        assert!(
            ev[0].tags.contains("\\move(500,-50,500,250,0,1000)"),
            "{}",
            ev[0].tags
        );
        i.effects.slide = Some((SlideDir::Bottom, secs(1.0)));
        let ev = events(&i, 1000, 500, (400.0, 100.0), secs(100.0));
        assert!(
            ev[0].tags.contains("\\move(500,550,500,250,0,1000)"),
            "{}",
            ev[0].tags
        );
        // with a blink, a phase after the slide is still and one inside is cut
        i.effects.slide = Some((SlideDir::Left, secs(1.0)));
        i.effects.blink = Some(secs(0.5));
        i.ttl = secs(3.0);
        let ev = events(&i, 1000, 500, (400.0, 100.0), secs(100.0));
        assert!(ev[0].tags.contains("\\move(-200,250,500,250,0,1000)"));
        assert!(ev[1].tags.contains("\\pos(500,250)"), "{}", ev[1].tags);
    }

    // covers: REQ-111
    #[test]
    fn colour_font_size_and_escaping_are_written_by_us() {
        let mut i = item();
        i.text = "{\\evil}".into();
        i.colour = [0x11, 0x22, 0x33];
        // a font that is installed (a copy of the bundled one under another name)
        crate::fonts::install_patched("Tag Sans1", 1124);
        i.font = "Tag Sans1".into();
        i.width = 1000;
        let ev = &events(&i, 1920, 1080, estimate_box(&i, 1920, 1080), secs(100.0))[0];
        assert!(ev.tags.contains("\\1c&H332211&"));
        assert!(ev.tags.contains("\\fnTag Sans1\\b0"), "{}", ev.tags);
        assert!(ev.tags.contains(&format!("\\fs{}", fit_px(&i, 1920, 1080))));
        assert!(!ev.text.contains('{') && !ev.text.contains('\\'));
        assert_eq!(font_px(1080, SIZE_NORMAL), 54);
        let (w, h) = estimate_box(&item(), 1920, 1080);
        assert!(w > 100.0 && h > 100.0);
    }

    // covers: REQ-113
    #[test]
    fn hostile_values_are_refused() {
        let ok = item();
        ok.validate().unwrap();
        type Case = Box<dyn Fn(&mut TextItem)>;
        let bad: Vec<Case> = vec![
            Box::new(|i| i.ttl = Micros(99_999)),
            Box::new(|i| i.ttl = Micros(MAX_TTL.0 + 1)),
            Box::new(|i| i.start = Micros(-1)),
            Box::new(|i| i.effects.fade_in = Some(Micros(99_999))),
            Box::new(|i| i.effects.blink = Some(Micros(5_000_001))),
            Box::new(|i| i.effects.slide = Some((SlideDir::Top, Micros(0)))),
            Box::new(|i| i.font = "a;b".into()),
            Box::new(|i| i.font = "x".repeat(101)),
            Box::new(|i| i.text = "".into()),
            Box::new(|i| i.text = "a\u{7}".into()),
            Box::new(|i| i.width = 99),
            Box::new(|i| i.width = 1001),
            Box::new(|i| i.place = Pos { x: 10_001, y: 0 }),
            Box::new(|i| {
                i.anchor = Some(Anchor360 {
                    yaw_cd: 18_001,
                    pitch_cd: 0,
                    height_cd: 100,
                    width_cd: 100,
                })
            }),
            Box::new(|i| {
                i.anchor = Some(Anchor360 {
                    yaw_cd: 0,
                    pitch_cd: 9_001,
                    height_cd: 100,
                    width_cd: 100,
                })
            }),
            Box::new(|i| {
                i.anchor = Some(Anchor360 {
                    yaw_cd: 0,
                    pitch_cd: 0,
                    height_cd: 0,
                    width_cd: 100,
                })
            }),
            Box::new(|i| {
                i.anchor = Some(Anchor360 {
                    yaw_cd: 0,
                    pitch_cd: 0,
                    height_cd: 18_001,
                    width_cd: 100,
                })
            }),
            Box::new(|i| {
                i.ttl = Micros(3_600_000_000);
                i.effects.blink = Some(EFFECT_MIN);
            }),
        ];
        for (n, f) in bad.iter().enumerate() {
            let mut i = ok.clone();
            f(&mut i);
            assert!(i.validate().is_err(), "case {n}");
        }
        assert!(valid_font("Noto Sans (Bold)-1.0_x"));
        assert!(!valid_font("a\nb"));
    }

    // covers: REQ-108, REQ-110
    #[test]
    fn the_list_adds_replaces_removes_and_caps() {
        let mut t = Texts::default();
        assert!(t.is_empty());
        let id = t.new_id();
        let id = t.add(TextItem::new(id, "a".into(), Micros::ZERO)).unwrap();
        assert!(t.get(id).is_some());
        assert_eq!(t.shown_at(secs(1.0)).len(), 1);
        assert!(t.shown_at(secs(3.0)).is_empty());
        let mut j = t.get(id).unwrap().clone();
        j.ttl = secs(6.0);
        t.replace(j.clone()).unwrap();
        assert_eq!(t.get(id).unwrap().ttl, secs(6.0));
        j.id = TextId(99);
        assert_eq!(t.replace(j), Err(TextError::Unknown));
        let mut bad = t.get(id).unwrap().clone();
        bad.ttl = Micros(1);
        assert!(matches!(t.replace(bad), Err(TextError::Invalid(_))));
        assert_eq!(t.remove(TextId(5)), Err(TextError::Unknown));
        t.validate().unwrap();
        // the total of events is capped
        let mut b = TextItem::new(TextId(50), "b".into(), Micros::ZERO);
        b.ttl = secs(400.0);
        b.effects.blink = Some(EFFECT_MIN);
        assert_eq!(event_count(&b), 2000);
        for n in 0..9 {
            b.id = TextId(50 + n);
            t.add(b.clone()).unwrap();
        }
        b.id = TextId(70);
        let e = t.add(b.clone()).unwrap_err();
        assert!(e.to_string().contains("too many"));
        assert_eq!(t.total_events(), 18_001);
        let mut big = t.get(TextId(50)).unwrap().clone();
        big.ttl = secs(300.0);
        t.replace(big).unwrap();
        t.items[0].ttl = secs(10_000.0);
        t.items[0].effects.blink = Some(EFFECT_MIN);
        assert!(t.validate().is_err());
        t.remove(id).unwrap();
        assert!(t.next_id >= 58);
    }

    // covers: REQ-113
    #[test]
    fn a_list_with_a_duplicate_id_is_refused() {
        let mut t = Texts::default();
        t.items.push(item());
        t.items.push(item());
        assert!(t.validate().unwrap_err().contains("duplicate"));
    }

    // covers: REQ-113
    #[test]
    fn items_round_trip_through_json() {
        let mut i = item();
        i.effects.slide = Some((SlideDir::Bottom, secs(0.5)));
        i.anchor = Some(Anchor360 {
            yaw_cd: 3000,
            pitch_cd: 1000,
            height_cd: 500,
            width_cd: 6000,
        });
        let j = serde_json::to_string(&i).unwrap();
        let back: TextItem = serde_json::from_str(&j).unwrap();
        assert_eq!(back, i);
    }

    // covers: REQ-116
    #[test]
    fn best_fit_follows_the_width_and_the_amount_of_text() {
        let mut i = item();
        i.text = "The quick brown fox jumps over the lazy dog and keeps running far away".into();
        let at = |i: &TextItem, w: u16| {
            let mut j = i.clone();
            j.width = w;
            fit_px(&j, 1920, 1080)
        };
        // monotone in the width: a wider box never gives a smaller text
        let mut last = 0.0;
        for w in (WIDTH_MIN..=WIDTH_MAX).step_by(50) {
            let f = at(&i, w);
            assert!(f >= last, "width {w}: {f} < {last}");
            last = f;
        }
        assert!(at(&i, 1000) > at(&i, 200), "widening makes the text larger");
        // lines three times longer give a smaller text in a box of the same width
        let mut long = i.clone();
        long.text = long.text.repeat(3);
        assert!(at(&long, 500) < at(&i, 500));
        // more lines make a smaller text
        let mut two = item();
        two.text = "one\ntwo".into();
        let mut four = item();
        four.text = "one\ntwo\nthree\nfour".into();
        assert!(at(&four, 800) < at(&two, 800));
        // never above 10% of the height (REQ-157)
        let mut short = item();
        short.text = "Hi".into();
        assert_eq!(at(&short, 1000), 108.0);
        // never below 2% of the height: text that does not fit runs past the box
        let mut huge = item();
        huge.text = "word ".repeat(2000);
        assert_eq!(at(&huge, 100), 1080.0 * 0.02);
        // the same item gives the same size in every call: preview and export agree
        assert_eq!(fit_px(&i, 1920, 1080), fit_px(&i, 1920, 1080));
        // 200 items fit quickly
        let t = std::time::Instant::now();
        for _ in 0..200 {
            std::hint::black_box(fit_px(&i, 1920, 1080));
        }
        assert!(t.elapsed() < std::time::Duration::from_millis(200));
    }

    // covers: REQ-111, REQ-116
    #[test]
    fn the_box_and_the_events_use_the_width() {
        let mut i = item();
        i.width = 500;
        let (bw, bh) = box_px(&i, 1920, 1080);
        assert_eq!((bw, bh), (960.0, 216.0));
        let ev = &events(&i, 1920, 1080, estimate_box(&i, 1920, 1080), secs(100.0))[0];
        // the text wraps at the box width: margins leave 960 px
        assert_eq!(ev.margins, Some((480, 480)));
        // an event of the 360 anchor uses the fit of its angle box
        i.anchor = Some(Anchor360 {
            yaw_cd: 0,
            pitch_cd: 0,
            height_cd: 2000,
            width_cd: 6000,
        });
        let ev = equirect_events(&i, 3840, 1920, secs(100.0), None);
        assert!(!ev.is_empty());
        assert!(ev[0].tags.contains("\\fs"));
    }

    // covers: REQ-123
    #[test]
    fn a_text_of_1_3_0_gets_the_width_that_keeps_its_look() {
        // one short line: the old size fits a narrow box
        let narrow = legacy_width("Hi", SIZE_NORMAL);
        assert!(narrow < 400, "{narrow}");
        let mut i = TextItem::new(TextId(1), "Hi".into(), Micros::ZERO);
        i.width = narrow;
        assert!(legacy_fit_px(&i, 1920, 1080) >= font_px(1080, SIZE_NORMAL));
        i.width = narrow - 1;
        assert!(narrow == WIDTH_MIN || legacy_fit_px(&i, 1920, 1080) < font_px(1080, SIZE_NORMAL));
        // a text too big for the box gets the widest box and shrinks to fit
        assert_eq!(legacy_width(&"word ".repeat(300), 4000), WIDTH_MAX);
        // a text fixed to the sphere: more rows make a taller box, in range
        let one = legacy_anchor_height("one line", 500, 6000, 1000);
        let many = legacy_anchor_height(&"words ".repeat(40), 500, 6000, 1000);
        assert!(many > one && (1..=18_000).contains(&many));
        assert_eq!(legacy_anchor_height("x", 1, 1, 250), 1);
    }

    fn install_test_font(family: &str, win_ascent: u16) {
        crate::fonts::install_patched(family, win_ascent);
    }

    // covers: REQ-158
    #[test]
    fn readability_is_written_as_none_outline_or_a_box() {
        let mut i = item();
        i.text = "Hi".into();
        i.width = 1000;
        let at = |r: Readability| {
            let mut j = i.clone();
            j.readability = r;
            events(&j, 1920, 1080, estimate_box(&j, 1920, 1080), secs(100.0))
        };
        // Hi in the default box: 10% of 1080
        let none = &at(Readability::None)[0];
        assert!(none.tags.contains("\\bord0\\"), "{}", none.tags);
        assert!(!none.boxed);
        let out = &at(Readability::Outline)[0];
        assert!(out.tags.contains("\\bord6\\"), "6% of 108: {}", out.tags);
        assert!(!out.boxed);
        let boxed = &at(Readability::Box)[0];
        assert!(
            boxed.tags.contains("\\bord11\\"),
            "10% of 108: {}",
            boxed.tags
        );
        assert!(boxed.boxed);
        // the readability is stored by name, and a text without it (before 2.0.3) is None
        let j = serde_json::to_string(&Readability::Box).unwrap();
        assert_eq!(j, "\"box\"");
        let old: TextItem = serde_json::from_str(
            r#"{"id":1,"text":"x","start":0,"ttl":1000000,"colour":[1,2,3],"place":{"x":1,"y":2},"width":500}"#,
        )
        .unwrap();
        assert_eq!(old.readability, Readability::None);
        assert_eq!(old.height, DEFAULT_HEIGHT);
        assert_eq!(Readability::None.border(100.0), 0.0);
        assert_eq!(Readability::ALL.len(), 3);
        assert_eq!(Readability::Box.name(), "Background box");
        // a new text starts with an outline
        assert_eq!(item().readability, Readability::Outline);
    }

    // covers: REQ-156
    #[test]
    fn the_lines_are_wrapped_by_us_and_the_renderer_never_wraps() {
        let mut i = item();
        i.text = "The quick brown fox jumps over the lazy dog and keeps running far away".into();
        i.width = 300;
        let ev = &events(&i, 1920, 1080, estimate_box(&i, 1920, 1080), secs(100.0))[0];
        assert!(ev.tags.contains("\\q2"), "{}", ev.tags);
        let lines: Vec<&str> = ev.text.split("\\N").collect();
        assert!(lines.len() > 1, "a narrow box wraps: {}", ev.text);
        // nothing is lost and no line is wider than the box
        assert_eq!(lines.join(" "), i.text);
        let l = layout(&i, 1920, 1080);
        assert!(l.width <= box_px(&i, 1920, 1080).0 + 1.0);
        assert_eq!(l.lines.len(), lines.len());
        // the same layout twice: preview and export agree
        assert_eq!(layout(&i, 1920, 1080), l);
    }

    // covers: REQ-163
    #[test]
    fn the_layout_uses_the_face_the_picture_is_drawn_with() {
        install_test_font("Wide Sans", 300);
        let mut a = item();
        a.text = "A text that is measured with a font and is long enough to fill the box".into();
        a.width = 500;
        let mut b = a.clone();
        b.font = "Wide Sans".into();
        let (la, lb) = (layout(&a, 1920, 1080), layout(&b, 1920, 1080));
        // the wider font gets a smaller size for the same box, and still fits it
        assert!(lb.px < la.px, "{} {}", lb.px, la.px);
        assert!(lb.fits && lb.width <= box_px(&b, 1920, 1080).0 + 1.0);
        // the same size in both fonts: the installed one is wider
        let wrap = |t: &TextItem| {
            shaped_for(&t.text, &t.font)
                .unwrap()
                .at_size(1e6, 50.0)
                .width
        };
        assert!(wrap(&b) > 1.5 * wrap(&a));
        let ev = &events(&b, 1920, 1080, estimate_box(&b, 1920, 1080), secs(100.0))[0];
        assert!(ev.tags.contains("\\fnWide Sans\\b0"), "{}", ev.tags);
    }

    // covers: REQ-164
    #[test]
    fn a_missing_font_is_drawn_in_the_bundled_font_and_its_name_is_kept() {
        let mut i = item();
        i.font = "Not Installed Anywhere".into();
        assert!(i.validate().is_ok());
        let ev = &events(&i, 1920, 1080, estimate_box(&i, 1920, 1080), secs(100.0))[0];
        assert!(
            ev.tags.contains("\\fnNoto Sans\\b1"),
            "written out, never the renderer's own fallback: {}",
            ev.tags
        );
        assert!(!ev.tags.contains("Not Installed"));
        // measured with the bundled font: the same layout as the default font
        let d = item();
        assert_eq!(layout(&i, 1920, 1080), layout(&d, 1920, 1080));
        // the project keeps the name
        let j: TextItem = serde_json::from_str(&serde_json::to_string(&i).unwrap()).unwrap();
        assert_eq!(j.font, "Not Installed Anywhere");
    }
}
