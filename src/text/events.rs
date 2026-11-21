// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The ASS events that burn texts and 360 lines in, for the preview and the export alike.

use super::*;

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
#[derive(Debug, Clone, PartialEq)]
struct Layout {
    /// Size of the picture (slide distances are measured against it).
    w: f64,
    h: f64,
    /// Centre of the text box.
    cx: f64,
    cy: f64,
    /// Font size.
    px: f64,
    /// The lines, already wrapped.
    lines: Vec<String>,
    /// Box of the text (slide distance).
    bw: f64,
    bh: f64,
    /// Left and right margin that fix the wrap width.
    margins: Option<(u32, u32)>,
}

/// `text` (words separated by spaces and `\N`) with a karaoke time before each word: the
/// n-th word lasts `cs[n]` hundredths of a second (the last value is kept for the rest).
fn karaoke_text(text: &str, cs: &[u32]) -> String {
    let mut n = 0;
    let mut out = String::new();
    for (i, line) in text.split("\\N").enumerate() {
        if i > 0 {
            out.push_str("\\N");
        }
        for (j, word) in line.split(' ').enumerate() {
            if j > 0 {
                out.push(' ');
            }
            if word.is_empty() {
                continue;
            }
            let d = cs.get(n).or(cs.last()).copied().unwrap_or(1).max(1);
            out.push_str(&format!("{{\\k{d}}}{word}"));
            n += 1;
        }
    }
    out
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
    let (w, h, cx, cy, px, bw, bh, margins) = (
        lay.w,
        lay.h,
        lay.cx,
        lay.cy,
        lay.px,
        lay.bw,
        lay.bh,
        lay.margins,
    );
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
    // an installed font is drawn as chosen; any other one is the bundled font, written out
    // so that the renderer never picks a silent fallback of its own (REQ-164)
    let installed = !item.font.is_empty() && fonts::resolve(&item.font).is_some();
    let (font, bold) = if installed {
        (item.font.as_str(), 0)
    } else {
        (DEFAULT_FONT, 1)
    };
    let mut base = format!(
        "\\an5\\q2\\fn{font}\\b{bold}\\fs{px:.1}\\bord{}\\1c&H{b:02X}{g:02X}{r:02X}&",
        item.readability.border(px)
    );
    if !item.karaoke.is_empty() {
        // the word being spoken turns yellow (the line's colour before it is spoken)
        base.push_str(&format!("\\2c&H{b:02X}{g:02X}{r:02X}&\\1c&H00FFFF&"));
    }
    let text = lay
        .lines
        .iter()
        .map(|l| {
            if installed {
                ass::escape(l)
            } else {
                ass::escape_scripts(l)
            }
        })
        .collect::<Vec<_>>()
        .join("\\N");
    let text = if item.karaoke.is_empty() {
        text
    } else {
        karaoke_text(&text, &item.karaoke)
    };
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
                boxed: item.readability == Readability::Box,
                shadow: item.shadow,
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
    let fit = layout(item, width, height);
    let lay = Layout {
        w,
        h,
        cx: f64::from(item.place.x.min(POS_MAX)) * w / f64::from(POS_MAX),
        cy: f64::from(item.place.y.min(POS_MAX)) * h / f64::from(POS_MAX),
        px: fit.px,
        lines: fit.lines,
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
    let fit = match px {
        Some(px) => layout_at(&item.text, &item.font, f64::from(px), wrap),
        None => {
            let box_h = f64::from(a.height_cd) / 100.0 * f64::from(h) / 180.0;
            layout_in(&item.text, &item.font, wrap, box_h, 4.0, f64::from(h) * 0.1)
        }
    };
    let margin = ((f64::from(w) - wrap) / 2.0).max(0.0).round() as u32;
    let pad = item.readability.border(fit.px);
    let (bw, bh) = (fit.width + 2.0 * pad, fit.height() + 2.0 * pad);
    let lay = Layout {
        w: f64::from(w),
        h: f64::from(h),
        cx,
        cy,
        px: fit.px,
        lines: fit.lines,
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
                ..lay.clone()
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
                ..lay.clone()
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
    project_events_in(texts, width, height, video_end, flat_windows, None)
}

/// [`project_events`] with the items also limited to `keep` (the parts of the timeline that
/// are not drawn by the overlay stage, REQ-304); `None`: everywhere.
pub fn project_events_in(
    texts: &Texts,
    width: u32,
    height: u32,
    video_end: Micros,
    flat_windows: Option<&[Window]>,
    keep: Option<&[Window]>,
) -> Vec<AssEvent> {
    texts
        .items
        .iter()
        .filter(|i| i.picture.is_none())
        .flat_map(|i| {
            let ws: Option<Vec<Window>> = match (i.anchor.is_some(), flat_windows, keep) {
                (true, Some(f), Some(k)) => Some(intersect(f, k)),
                (true, Some(f), None) => Some(f.to_vec()),
                (_, _, Some(k)) => Some(k.to_vec()),
                _ => None,
            };
            flat_events_in(
                i,
                width,
                height,
                estimate_box(i, width, height),
                video_end,
                ws.as_deref(),
            )
        })
        .collect()
}

/// The events of one item on the flat picture, limited to `windows` (`None`: everywhere).
pub fn item_events(
    item: &TextItem,
    width: u32,
    height: u32,
    video_end: Micros,
    windows: Option<&[Window]>,
) -> Vec<AssEvent> {
    flat_events_in(
        item,
        width,
        height,
        estimate_box(item, width, height),
        video_end,
        windows,
    )
}

/// The parts that two lists of windows have in common.
pub fn intersect(a: &[Window], b: &[Window]) -> Vec<Window> {
    let mut out = Vec::new();
    for (s1, e1) in a {
        for (s2, e2) in b {
            let (s, e) = ((*s1).max(*s2), (*e1).min(*e2));
            if s < e {
                out.push((s, e));
            }
        }
    }
    out
}
