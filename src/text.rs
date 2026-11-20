// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Text items (REQ-108 to REQ-111): a text shown for a while at a place of the picture,
//! with fade in, fade out, blink and slide effects, a colour and a font. Items live on
//! the **timeline** (they do not move when the video is edited, like music). Everything
//! here is pure: the model, its validation (projects are untrusted input) and the ASS
//! events that burn the text in, used by the preview and the export alike.

use crate::ass::{self, AssEvent, font_px};
use crate::base::pos::{Anchor360, POS_MAX, Pos};
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
/// Height of every text box, in thousandths of the picture height (20%).
pub const BOX_HEIGHT: u32 = 200;
/// Smallest best-fit text height, in thousandths of the picture height (2%).
pub const MIN_FIT: u32 = 20;
/// Limits of the ASS events of one item and of a project.
pub const MAX_EVENTS_ITEM: usize = 2000;
pub const MAX_EVENTS_TOTAL: usize = 20_000;
/// Most lines of a text and longest family name.
pub const MAX_LINES: usize = 200;
pub const MAX_FONT: usize = 100;
/// Text items of a project.
pub const MAX_ITEMS: usize = 5000;
/// Where a new text starts: centred, in the lower third.
pub const DEFAULT_PLACE: Pos = Pos { x: 5000, y: 8300 };
/// The font used when none is chosen (the bundled subtitle font).
pub const DEFAULT_FONT: &str = "Noto Sans";

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
    /// (1.4.0). The text size is the best fit of that box ([`fit_px`]).
    pub width: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor360>,
}

/// True for a font family name that is safe to put in an ASS file.
pub fn valid_font(name: &str) -> bool {
    name.chars().count() <= MAX_FONT
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '.' | '_' | '(' | ')'))
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
            anchor: None,
        }
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
        if !self.place.is_valid() || !(WIDTH_MIN..=WIDTH_MAX).contains(&self.width) {
            return Err("text item with an invalid place or width".into());
        }
        if self.anchor.is_some_and(|a| !a.is_valid()) {
            return Err("text item with an invalid 360 anchor".into());
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

/// Effective fade lengths: each is limited to half of the time to live (REQ-109 d).
pub fn fades(item: &TextItem) -> (Micros, Micros) {
    let half = Micros(item.ttl.0 / 2);
    (
        item.effects.fade_in.map_or(Micros::ZERO, |f| f.min(half)),
        item.effects.fade_out.map_or(Micros::ZERO, |f| f.min(half)),
    )
}

/// The shown phases of an item, relative to its start: one for a text that does not blink.
pub fn phases(item: &TextItem) -> Vec<(Micros, Micros)> {
    let Some(i) = item.effects.blink else {
        return vec![(Micros::ZERO, item.ttl)];
    };
    let i = i.0.max(1);
    let mut v = Vec::new();
    let mut s = 0i64;
    while s < item.ttl.0 && v.len() <= MAX_EVENTS_ITEM {
        v.push((Micros(s), Micros((s + i).min(item.ttl.0))));
        s += 2 * i;
    }
    v
}

/// How many ASS events an item needs (computed without building them).
pub fn event_count(item: &TextItem) -> usize {
    match item.effects.blink {
        None => 1,
        Some(i) => {
            let i = i.0.max(1);
            usize::try_from(
                item.ttl.0.max(0) / (2 * i) + i64::from(item.ttl.0.max(0) % (2 * i) != 0),
            )
            .unwrap_or(usize::MAX)
        }
    }
}

/// The fades alone: 0 at the start rising to 1 over the fade in, falling to 0 over the
/// fade out at the end of the time to live.
fn envelope(item: &TextItem, t: Micros) -> f64 {
    let (fin, fout) = fades(item);
    let mut a = 1.0;
    if fin > Micros::ZERO && t < fin {
        a = (t.0 as f64 / fin.0 as f64).max(0.0);
    }
    if fout > Micros::ZERO && t > item.ttl - fout {
        a = a.min(((item.ttl - t).0 as f64 / fout.0 as f64).max(0.0));
    }
    a
}

/// Visibility (0..=1) of the text at `t` after its start (the formula the ASS events
/// follow; used by tests and by the preview boxes).
pub fn alpha_at(item: &TextItem, t: Micros) -> f64 {
    if t < Micros::ZERO || t >= item.ttl {
        return 0.0;
    }
    if let Some(i) = item.effects.blink
        && (t.0 / i.0.max(1)) % 2 == 1
    {
        return 0.0;
    }
    envelope(item, t)
}

/// The box of a text on a flat `width` x `height` picture, in pixels: its width and the
/// fixed height (REQ-116).
pub fn box_px(item: &TextItem, width: u32, height: u32) -> (f64, f64) {
    (
        f64::from(width) * f64::from(item.width) / 1000.0,
        f64::from(height) * f64::from(BOX_HEIGHT) / 1000.0,
    )
}

/// The rows `text` takes at font height `px` when it wraps at `wrap` pixels: characters
/// are taken as 0.6 of the font height wide.
fn rows_at(text: &str, px: f64, wrap: f64) -> f64 {
    text.split('\n')
        .map(|l| {
            (l.chars().count() as f64 * px * 0.6 / wrap.max(1.0))
                .ceil()
                .max(1.0)
        })
        .sum()
}

/// The **best fit** (REQ-116): the largest font height in pixels at which `text`, wrapped
/// at `box_w`, fits inside `box_h` (rows of 1.2 times the font height), never more than one
/// line filling the box, never below `min_px`. Pure: preview and export call it with the
/// same geometry, so they agree.
pub fn fit_in(text: &str, box_w: f64, box_h: f64, min_px: f64) -> f64 {
    let fits = |px: f64| rows_at(text, px, box_w) * px * 1.2 <= box_h;
    let hi = box_h / 1.2;
    if hi <= min_px || fits(hi) {
        return hi.max(min_px);
    }
    if !fits(min_px) {
        return min_px;
    }
    let (mut lo, mut hi) = (min_px, hi);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Font size in pixels of a flat text on a `width` x `height` picture: its best fit.
pub fn fit_px(item: &TextItem, width: u32, height: u32) -> u32 {
    let (bw, bh) = box_px(item, width, height);
    let min = f64::from(height) * f64::from(MIN_FIT) / 1000.0;
    (fit_in(&item.text, bw, bh, min).floor() as u32).max(6)
}

/// The box width that makes a text made in 1.3.0 (with the size `old_size`, thousandths
/// of the default) look as before (ADR-088): the narrowest width whose best fit is at
/// least the old font height, else the widest. Measured on a 1920x1080 picture.
pub fn legacy_width(text: &str, old_size: u16) -> u16 {
    let want = font_px(1080, old_size);
    let mut item = TextItem::new(TextId(0), text.to_string(), Micros::ZERO);
    let (mut lo, mut hi) = (WIDTH_MIN, WIDTH_MAX);
    item.width = hi;
    if fit_px(&item, 1920, 1080) < want {
        return WIDTH_MAX;
    }
    while lo < hi {
        let mid = (lo + hi) / 2;
        item.width = mid;
        if fit_px(&item, 1920, 1080) >= want {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    hi
}

/// The box height (hundredths of a degree) of a text fixed to the sphere made in 1.3.0
/// with the angles `size_cd` (height of a line at 100%), `wrap_cd` and the size `old_size`:
/// the rows it took (ADR-088), clamped to the range of an anchor.
pub fn legacy_anchor_height(text: &str, size_cd: u32, wrap_cd: u32, old_size: u16) -> u32 {
    let (w, h) = (3840.0, 1920.0);
    let px =
        f64::from(size_cd) / 100.0 * (h / 180.0) * f64::from(old_size) / f64::from(SIZE_NORMAL);
    let wrap = (f64::from(wrap_cd) / 100.0 * w / 360.0).clamp(8.0, w);
    let rows = rows_at(text, px.max(4.0), wrap);
    let cd = rows * f64::from(size_cd) * f64::from(old_size) / f64::from(SIZE_NORMAL) * 1.2;
    (cd.round() as u32).clamp(1, 18_000)
}

/// Rough size (pixels) of the text of an item, used where nothing is measured (the slide
/// distance of the export): characters are taken as 0.6 of the font size wide, and long
/// lines wrap at the width of the box.
pub fn estimate_box(item: &TextItem, width: u32, height: u32) -> (f64, f64) {
    let px = f64::from(fit_px(item, width, height));
    let max_w = box_px(item, width, height).0;
    let mut w: f64 = 0.0;
    for l in item.text.split('\n') {
        w = w.max((l.chars().count() as f64 * px * 0.6).min(max_w));
    }
    let rows = rows_at(&item.text, px, max_w);
    let pad = ass::outline(px as u32);
    (w + 2.0 * pad, rows * px * 1.2 + 2.0 * pad)
}

fn alpha_hex(a: f64) -> String {
    format!(
        "&H{:02X}&",
        ((1.0 - a.clamp(0.0, 1.0)) * 255.0).round() as u32
    )
}

fn ms(t: Micros) -> i64 {
    (t.0 + 500) / 1000
}

/// Where and how big an item is drawn, in the pixels of the picture the ASS file is for.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Layout {
    /// Size of the picture (slide distances are measured against it).
    w: f64,
    h: f64,
    /// Centre of the text box.
    cx: f64,
    cy: f64,
    /// Font size.
    px: u32,
    /// Box of the text (slide distance).
    bw: f64,
    bh: f64,
    /// Left and right margin that fix the wrap width.
    margins: Option<(u32, u32)>,
}

/// A part of the timeline (`start`, `end`) an item may be drawn in.
pub type Window = (Micros, Micros);

fn events_with(
    item: &TextItem,
    lay: &Layout,
    video_end: Micros,
    windows: Option<&[Window]>,
) -> Vec<AssEvent> {
    let (fin, fout) = fades(item);
    let Layout {
        w,
        h,
        cx,
        cy,
        px,
        bw,
        bh,
        margins,
    } = *lay;
    let from = item.effects.slide.map(|(d, dur)| {
        let (x, y) = match d {
            SlideDir::Left => ((cx - bw).min(-bw / 2.0), cy),
            SlideDir::Right => ((cx + bw).max(w + bw / 2.0), cy),
            SlideDir::Top => (cx, (cy - bh).min(-bh / 2.0)),
            SlideDir::Bottom => (cx, (cy + bh).max(h + bh / 2.0)),
        };
        (x, y, dur)
    });
    let [r, g, b] = item.colour;
    let font = if item.font.is_empty() {
        DEFAULT_FONT
    } else {
        &item.font
    };
    let base = format!(
        "\\an5\\fn{font}\\fs{px}\\bord{}\\1c&H{b:02X}{g:02X}{r:02X}&",
        ass::outline(px)
    );
    let text = item
        .text
        .split('\n')
        .map(ass::escape)
        .collect::<Vec<_>>()
        .join("\\N");
    let mut out = Vec::new();
    for (ps0, pe0) in phases(item) {
        // the part of the phase that lies in the video and in the windows
        let pieces: Vec<(Micros, Micros)> = match windows {
            None => vec![(ps0, pe0)],
            Some(ws) => ws
                .iter()
                .map(|(a, b)| ((*a - item.start).max(ps0), (*b - item.start).min(pe0)))
                .collect(),
        };
        for (ps, pe) in pieces {
            let (s, e) = (item.start + ps, (item.start + pe).min(video_end));
            if e <= s {
                continue;
            }
            let pe = e - item.start;
            let full = ps == ps0 && pe == pe0;
            let mut tags = base.clone();
            // place: still, or moving until the slide has finished
            match from {
                Some((x0, y0, dur)) if ps < dur => {
                    let k = ps.0 as f64 / dur.0 as f64;
                    let (x1, y1) = (x0 + (cx - x0) * k, y0 + (cy - y0) * k);
                    tags.push_str(&format!(
                        "\\move({:.0},{:.0},{:.0},{:.0},0,{})",
                        x1,
                        y1,
                        cx,
                        cy,
                        ms(dur - ps)
                    ));
                }
                _ => tags.push_str(&format!("\\pos({cx:.0},{cy:.0})")),
            }
            // visibility: \fad for a plain fade, \t steps when the event is only a part
            if item.effects.blink.is_none() && full {
                if fin > Micros::ZERO || fout > Micros::ZERO {
                    tags.push_str(&format!("\\fad({},{})", ms(fin), ms(fout)));
                }
            } else {
                let init = envelope(item, ps);
                if init < 1.0 {
                    tags.push_str(&format!("\\alpha{}", alpha_hex(init)));
                }
                if fin > ps {
                    let upto = fin.min(pe);
                    tags.push_str(&format!(
                        "\\t(0,{},\\alpha{})",
                        ms(upto - ps),
                        alpha_hex(envelope(item, upto))
                    ));
                }
                let down = item.ttl - fout;
                if fout > Micros::ZERO && pe > down {
                    let from_t = down.max(ps);
                    let to = pe.min(item.ttl);
                    tags.push_str(&format!(
                        "\\t({},{},\\alpha{})",
                        ms(from_t - ps),
                        ms(to - ps),
                        alpha_hex(envelope(item, to))
                    ));
                }
            }
            out.push(AssEvent {
                start: s,
                end: e,
                tags,
                text: text.clone(),
                margins,
            });
        }
    }
    out
}

/// The ASS events of `item` for a flat picture of `width` x `height`, with the box of the
/// text estimated at `box_wh` (slide distance). Times are timeline times; events longer
/// than the video are cut at `video_end`.
pub fn events(
    item: &TextItem,
    width: u32,
    height: u32,
    box_wh: (f64, f64),
    video_end: Micros,
) -> Vec<AssEvent> {
    flat_events_in(item, width, height, box_wh, video_end, None)
}

/// [`events`] restricted to some parts of the timeline (`None`: everywhere).
pub fn flat_events_in(
    item: &TextItem,
    width: u32,
    height: u32,
    box_wh: (f64, f64),
    video_end: Micros,
    windows: Option<&[Window]>,
) -> Vec<AssEvent> {
    let (w, h) = (f64::from(width), f64::from(height));
    let lay = Layout {
        w,
        h,
        cx: f64::from(item.place.x.min(POS_MAX)) * w / f64::from(POS_MAX),
        cy: f64::from(item.place.y.min(POS_MAX)) * h / f64::from(POS_MAX),
        px: fit_px(item, width, height),
        bw: box_wh.0,
        bh: box_wh.1,
        margins: {
            let m = ((w - box_px(item, width, height).0) / 2.0).max(0.0).round() as u32;
            Some((m, m))
        },
    };
    events_with(item, &lay, video_end, windows)
}

/// Pixel of an anchor on an equirectangular picture of `w` x `h`: longitude 0 in the
/// middle, positive yaw to the right, positive pitch up.
pub fn equirect_point(a: &Anchor360, w: u32, h: u32) -> (f64, f64) {
    (
        (f64::from(a.yaw_cd) / 36_000.0 + 0.5) * f64::from(w),
        (0.5 - f64::from(a.pitch_cd) / 18_000.0) * f64::from(h),
    )
}

/// The events of an item fixed to a point of the sphere, drawn flat on the equirectangular
/// picture (`w` x `h`) at that point, so that a camera view or a 360 player shows it
/// there. A box that crosses the left or right edge of the picture is also drawn on the
/// other side. Times are timeline times; `windows` limits them to the 360 clips.
pub fn equirect_events(
    item: &TextItem,
    w: u32,
    h: u32,
    video_end: Micros,
    windows: Option<&[Window]>,
) -> Vec<AssEvent> {
    equirect_events_px(item, w, h, None, video_end, windows)
}

/// The font height of a subtitle line fixed to the sphere: its angle `height_cd` at 100%
/// times the line's own `size` (thousandths), on an equirectangular picture `h` high.
pub fn line_equirect_px(a: &Anchor360, h: u32, size: u16) -> u32 {
    let deg_px = f64::from(h) / 180.0;
    ((f64::from(a.height_cd) / 100.0 * deg_px * f64::from(size) / f64::from(SIZE_NORMAL)).round()
        as u32)
        .max(4)
}

/// [`equirect_events`] with an own font height `px` (subtitle lines); `None`: the best fit
/// of the item's box (REQ-116).
pub fn equirect_events_px(
    item: &TextItem,
    w: u32,
    h: u32,
    px: Option<u32>,
    video_end: Micros,
    windows: Option<&[Window]>,
) -> Vec<AssEvent> {
    let Some(a) = item.anchor else {
        return Vec::new();
    };
    let (cx, cy) = equirect_point(&a, w, h);
    let wrap = (f64::from(a.width_cd) / 100.0 * f64::from(w) / 360.0).clamp(8.0, f64::from(w));
    let px = px.unwrap_or_else(|| {
        let box_h = f64::from(a.height_cd) / 100.0 * f64::from(h) / 180.0;
        (fit_in(&item.text, wrap, box_h, 4.0).floor() as u32).max(4)
    });
    let margin = ((f64::from(w) - wrap) / 2.0).max(0.0).round() as u32;
    // the estimate of the box, with the equirect font and wrap
    let chars = item
        .text
        .split('\n')
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(1) as f64;
    let bw = (chars * f64::from(px) * 0.6).min(wrap);
    let bh = rows_at(&item.text, f64::from(px), wrap) * f64::from(px) * 1.2;
    let lay = Layout {
        w: f64::from(w),
        h: f64::from(h),
        cx,
        cy,
        px,
        bw,
        bh,
        margins: Some((margin, margin)),
    };
    let mut out = events_with(item, &lay, video_end, windows);
    if cx - bw / 2.0 < 0.0 {
        out.extend(events_with(
            item,
            &Layout {
                cx: cx + f64::from(w),
                ..lay
            },
            video_end,
            windows,
        ));
    }
    if cx + bw / 2.0 > f64::from(w) {
        out.extend(events_with(
            item,
            &Layout {
                cx: cx - f64::from(w),
                ..lay
            },
            video_end,
            windows,
        ));
    }
    out
}

/// The events of every item shown on the flat picture (`width` x `height`) of the video.
/// An item fixed to the sphere is shown there only where the video is flat: `windows`
/// are the timeline times of the flat clips (`None`: no 360 clip at all, every item is
/// shown at its flat place).
pub fn project_events(
    texts: &Texts,
    width: u32,
    height: u32,
    video_end: Micros,
    flat_windows: Option<&[Window]>,
) -> Vec<AssEvent> {
    texts
        .items
        .iter()
        .flat_map(|i| {
            let ws = if i.anchor.is_some() {
                flat_windows
            } else {
                None
            };
            flat_events_in(
                i,
                width,
                height,
                estimate_box(i, width, height),
                video_end,
                ws,
            )
        })
        .collect()
}

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
        i.font = "DejaVu Sans".into();
        i.width = 1000;
        let ev = &events(&i, 1920, 1080, estimate_box(&i, 1920, 1080), secs(100.0))[0];
        assert!(ev.tags.contains("\\1c&H332211&"));
        assert!(ev.tags.contains("\\fnDejaVu Sans"));
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
        let mut last = 0;
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
        // never above one line filling the box (20% of the height / 1.2)
        let mut short = item();
        short.text = "Hi".into();
        assert!(at(&short, 1000) <= (1080.0_f64 * 0.2 / 1.2) as u32);
        // never below 2% of the height: text that does not fit runs past the box
        let mut huge = item();
        huge.text = "word ".repeat(2000);
        assert_eq!(at(&huge, 100), (1080.0_f64 * 0.02) as u32);
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
        assert!(fit_px(&i, 1920, 1080) >= font_px(1080, SIZE_NORMAL));
        i.width = narrow - 1;
        assert!(narrow == WIDTH_MIN || fit_px(&i, 1920, 1080) < font_px(1080, SIZE_NORMAL));
        // a text too big for the box gets the widest box and shrinks to fit
        assert_eq!(legacy_width(&"word ".repeat(300), 4000), WIDTH_MAX);
        // a text fixed to the sphere: more rows make a taller box, in range
        let one = legacy_anchor_height("one line", 500, 6000, 1000);
        let many = legacy_anchor_height(&"words ".repeat(40), 500, 6000, 1000);
        assert!(many > one && (1..=18_000).contains(&many));
        assert_eq!(legacy_anchor_height("x", 1, 1, 250), 1);
    }
}
