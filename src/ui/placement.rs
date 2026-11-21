// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Where subtitle lines are on the preview picture, and moving them by dragging
//! (REQ-089, REQ-092). Works in output-frame pixels (the ASS `PlayRes` space, so the
//! preview and the export agree) and maps to the screen through the picture rectangle.
//! Text is measured by a function passed in (egui with the subtitle font in the app, a
//! fake in tests). Pure.

use crate::app::AppState;
use crate::ass;
use crate::base::pos::{POS_MAX, Pos};
use crate::project::Project;
use crate::selection::ObjRef;
use crate::subtitles::{LineId, Preset};
use crate::time::Micros;
use eframe::egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2, pos2, vec2};

/// Share of the picture of the action-safe and title-safe areas (REQ-199).
pub const ACTION_SAFE_SHARE: f64 = crate::placement::ACTION_SAFE;
pub const TITLE_SAFE_SHARE: f64 = crate::placement::TITLE_SAFE;
pub use crate::placement::Guides;

/// A subtitle being dragged in the preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Drag {
    /// The line under the pointer when the drag started.
    pub line: LineId,
    /// Every line that moves with it.
    pub ids: Vec<LineId>,
    /// The text items of a mixed selection that move with it (REQ-172).
    pub texts: Vec<crate::text::TextId>,
    /// Pointer minus box centre when the drag started (frame pixels).
    pub grab: Vec2,
    /// Size of the dragged line's box (frame pixels).
    pub size: Vec2,
    /// Output frame size.
    pub frame: (u32, u32),
    /// The whole box must stay inside the picture (a 360 view); else only its centre
    /// must (REQ-117).
    pub whole_box: bool,
}

pub use crate::placement::visible_text;

/// The box of a line of `text_size` (frame pixels, measured at the line's own size):
/// centred on its own position, or at the style's place. `size` is the line's own size
/// (thousandths of the preset's), which also scales the padding.
pub fn line_box(
    preset: Preset,
    pos: Option<Pos>,
    size: Option<u16>,
    frame: (u32, u32),
    text_size: Vec2,
) -> Rect {
    let text = (f64::from(text_size.x), f64::from(text_size.y));
    to_egui(crate::placement::padded_line_rect(
        preset, pos, size, frame, text,
    ))
}

/// A box of the pure placement rule as an egui rectangle (frame pixels).
fn to_egui(r: crate::placement::Rect) -> Rect {
    Rect::from_center_size(pos2(r.cx as f32, r.cy as f32), vec2(r.w as f32, r.h as f32))
}

/// The boxes of the lines on screen at `at`, in drawing order (the last one is on top).
/// `measure(text, font size, wrap width)` gives the size of laid-out text.
pub fn boxes(
    project: &Project,
    at: Micros,
    frame: (u32, u32),
    measure: &mut dyn FnMut(&str, f32, f32) -> Vec2,
) -> Vec<(LineId, Rect)> {
    let preset = project.subtitles.preset;
    let wrap = crate::placement::line_wrap(frame) as f32;
    project
        .subtitles
        .shown_at(&project.timeline, at)
        .into_iter()
        .filter(|l| !l.text.trim().is_empty())
        .map(|l| {
            let text = visible_text(&l, preset, at);
            let font = crate::placement::line_font(l.size, frame);
            let t = measure(&text, font as f32, wrap);
            (l.line, line_box(preset, l.position, l.size, frame, t))
        })
        .collect()
}

/// The box of a text item (frame pixels): the box of its width and the fixed height
/// (REQ-116), centred on its place. It is the box the export lays the text out in, so what
/// is grabbed is what is exported.
pub fn text_box(item: &crate::text::TextItem, frame: (u32, u32)) -> Rect {
    to_egui(crate::placement::text_rect(item, frame))
}

/// The box of a text or subtitle line fixed to the sphere, seen by the camera `cam` on a
/// picture of `frame` pixels (REQ-112): at the projected point, as big as the scene around
/// it. A text (`line_size` is `None`) fills its box of angles; a subtitle line (with its
/// own `line_size`, thousandths) is as big as its measured text. `None` while the point is
/// behind the camera or the box is outside the picture.
pub fn anchored_box(
    item: &crate::text::TextItem,
    line_size: Option<u16>,
    cam: crate::camera::Camera,
    frame: (u32, u32),
    measure: &mut dyn FnMut(&str, f32, f32) -> Vec2,
) -> Option<Rect> {
    let a = item.anchor?;
    let (x, y) = crate::camera::project(
        cam,
        frame.0,
        frame.1,
        f64::from(a.yaw_cd) / 100.0,
        f64::from(a.pitch_cd) / 100.0,
    )?;
    let focal = (f64::from(frame.0) / 2.0) / (cam.fov_deg().to_radians() / 2.0).tan();
    let per_deg = focal * std::f64::consts::PI / 180.0;
    let wrap = (f64::from(a.width_cd) / 100.0 * per_deg).max(8.0);
    let size = match line_size {
        Some(size) => {
            let font =
                (f64::from(a.height_cd) / 100.0 * per_deg * f64::from(size) / 1000.0).max(4.0);
            let text = measure(&item.text, font as f32, wrap as f32);
            let pad = ass::outline(font.round() as u32) as f32;
            text + vec2(2.0 * pad, 2.0 * pad)
        }
        None => vec2(
            wrap as f32,
            (f64::from(a.height_cd) / 100.0 * per_deg).max(4.0) as f32,
        ),
    };
    let rect = Rect::from_center_size(pos2(x as f32, y as f32), size);
    rect.intersects(Rect::from_min_size(
        pos2(0.0, 0.0),
        vec2(frame.0 as f32, frame.1 as f32),
    ))
    .then_some(rect)
}

/// The boxes of the text items on screen at `at` (a text that is hidden for a moment by
/// its blink keeps its box: it is still there to grab). With a 360 camera `cam`, a text
/// fixed to the sphere is where the camera sees it (and has no box when it is out of
/// view); otherwise every text is at its flat place.
pub fn text_boxes(
    project: &Project,
    at: Micros,
    frame: (u32, u32),
    cam: Option<crate::camera::Camera>,
    measure: &mut dyn FnMut(&str, f32, f32) -> Vec2,
) -> Vec<(crate::text::TextId, Rect)> {
    project
        .texts
        .shown_at(at)
        .into_iter()
        .filter_map(|t| match (t.anchor, cam) {
            (Some(_), Some(c)) => anchored_box(t, None, c, frame, measure).map(|r| (t.id, r)),
            _ => Some((t.id, text_box(t, frame))),
        })
        .collect()
}

/// The boxes of the lines fixed to the sphere that are on screen at `at`, as the camera
/// `cam` sees them (`None`: the point is out of view, so the line has no box there).
pub fn anchored_line_boxes(
    project: &Project,
    at: Micros,
    frame: (u32, u32),
    cam: crate::camera::Camera,
    measure: &mut dyn FnMut(&str, f32, f32) -> Vec2,
) -> Vec<(LineId, Option<Rect>)> {
    project
        .subtitles
        .shown_at(&project.timeline, at)
        .into_iter()
        .filter(|l| l.anchor.is_some() && !l.text.trim().is_empty())
        .map(|l| {
            let mut t = crate::text::TextItem::new(crate::text::TextId(0), l.text.clone(), l.start);
            t.anchor = l.anchor;
            let size = l.size.unwrap_or(crate::subtitles::SIZE_NORMAL);
            (l.line, anchored_box(&t, Some(size), cam, frame, measure))
        })
        .collect()
}

/// An arrow at the edge of the picture that points to an item fixed to the sphere which is
/// outside the view (REQ-260).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffViewArrow {
    /// Where the arrow is, in frame pixels.
    pub at: Pos2,
    /// The direction it points to (unit vector, screen axes).
    pub dir: Vec2,
    /// The turn (yaw, pitch in hundredths of a degree) that centres the item.
    pub turn: (i32, i32),
}

/// Distance from the picture's edge to the arrow (frame pixels).
pub const ARROW_INSET: f32 = 18.0;

/// The arrow for the anchor `a` seen by `cam` on a `frame` picture; `None` when the point
/// is inside the view.
pub fn off_view_arrow(
    cam: crate::camera::Camera,
    frame: (u32, u32),
    a: &crate::base::pos::Anchor360,
) -> Option<OffViewArrow> {
    let (w, h) = (frame.0 as f64, frame.1 as f64);
    let (yaw, pitch) = (f64::from(a.yaw_cd) / 100.0, f64::from(a.pitch_cd) / 100.0);
    let wrap = |d: f64| {
        let d = (d + 180.0).rem_euclid(360.0) - 180.0;
        if d <= -180.0 { 180.0 } else { d }
    };
    let dyaw = wrap(yaw - cam.yaw_deg());
    let dpitch = pitch - cam.pitch_deg();
    let (dx, dy) = match crate::camera::project(cam, frame.0, frame.1, yaw, pitch) {
        Some((x, y)) if (0.0..=w).contains(&x) && (0.0..=h).contains(&y) => return None,
        Some((x, y)) => (x - w / 2.0, y - h / 2.0),
        // behind the camera: towards the side the shortest turn takes
        None => (dyaw.signum() * w / 2.0, -dpitch / 90.0 * h / 2.0),
    };
    let len = dx.hypot(dy).max(1e-9);
    let (ux, uy) = (dx / len, dy / len);
    let m = f64::from(ARROW_INSET);
    let t = ((w / 2.0 - m) / ux.abs().max(1e-9)).min((h / 2.0 - m) / uy.abs().max(1e-9));
    Some(OffViewArrow {
        at: pos2((w / 2.0 + ux * t) as f32, (h / 2.0 + uy * t) as f32),
        dir: vec2(ux as f32, uy as f32),
        turn: (
            (dyaw * 100.0).round() as i32,
            (dpitch * 100.0).round() as i32,
        ),
    })
}

/// The arrows of the lines and texts fixed to the sphere that are shown at `at` and out of
/// the view of `cam` (REQ-260).
pub fn off_view_arrows(
    project: &Project,
    at: Micros,
    frame: (u32, u32),
    cam: crate::camera::Camera,
) -> Vec<OffViewArrow> {
    let lines = project
        .subtitles
        .shown_at(&project.timeline, at)
        .into_iter()
        .filter_map(|l| l.anchor);
    let texts = project
        .texts
        .shown_at(at)
        .into_iter()
        .filter_map(|t| t.anchor);
    lines
        .chain(texts)
        .filter_map(|a| off_view_arrow(cam, frame, &a))
        .collect()
}

/// The topmost box under frame point `p`.
pub fn hit(boxes: &[(LineId, Rect)], p: Pos2) -> Option<(LineId, Rect)> {
    boxes.iter().rev().find(|(_, r)| r.contains(p)).copied()
}

/// Keeps a box centre so that the whole box stays inside the frame (centred when the box
/// is larger than the frame).
pub fn clamp_centre(c: Pos2, size: Vec2, frame: (u32, u32)) -> Pos2 {
    let (w, h) = (frame.0 as f32, frame.1 as f32);
    let axis = |v: f32, half: f32, max: f32| {
        if 2.0 * half >= max {
            max / 2.0
        } else if v.is_finite() {
            v.clamp(half, max - half)
        } else {
            max / 2.0
        }
    };
    pos2(axis(c.x, size.x / 2.0, w), axis(c.y, size.y / 2.0, h))
}

/// Keeps a box centre on the picture (REQ-117): the box may go partly past an edge, its
/// centre may not.
pub fn clamp_to_picture(c: Pos2, frame: (u32, u32)) -> Pos2 {
    let axis = |v: f32, max: u32| {
        if v.is_finite() {
            v.clamp(0.0, max as f32)
        } else {
            max as f32 / 2.0
        }
    };
    pos2(axis(c.x, frame.0), axis(c.y, frame.1))
}

/// A frame point as a stored position.
pub fn to_pos(c: Pos2, frame: (u32, u32)) -> Pos {
    let f = |v: f32, max: u32| {
        if max == 0 {
            return POS_MAX / 2;
        }
        let r = (f64::from(v) / f64::from(max) * f64::from(POS_MAX)).round();
        r.clamp(0.0, f64::from(POS_MAX)) as u16
    };
    Pos {
        x: f(c.x, frame.0),
        y: f(c.y, frame.1),
    }
}

/// Where the dragged line goes for a pointer at frame point `pointer`: the box keeps its
/// grab offset and stays inside the picture.
pub fn drag_to(d: &Drag, pointer: Pos2) -> Pos {
    let c = if d.whole_box {
        clamp_centre(pointer - d.grab, d.size, d.frame)
    } else {
        clamp_to_picture(pointer - d.grab, d.frame)
    };
    to_pos(c, d.frame)
}

/// A screen point as a frame point, for a picture drawn in `picture`.
pub fn to_frame(p: Pos2, picture: Rect, frame: (u32, u32)) -> Pos2 {
    let s = picture.width() / frame.0.max(1) as f32;
    if s <= 0.0 || !s.is_finite() {
        return pos2(0.0, 0.0);
    }
    pos2((p.x - picture.min.x) / s, (p.y - picture.min.y) / s)
}

/// A frame rectangle on the screen, for a picture drawn in `picture`.
pub fn to_screen(r: Rect, picture: Rect, frame: (u32, u32)) -> Rect {
    let s = picture.width() / frame.0.max(1) as f32;
    Rect::from_min_max(
        picture.min + r.min.to_vec2() * s,
        picture.min + r.max.to_vec2() * s,
    )
}

/// The centre `c` (frame pixels) of a box being dragged, snapped to the guides when the
/// View option is on (REQ-198): the centre to use and the guides it sits on. The other
/// boxes shown at the playhead (not the `dragged` ones) are guides too.
pub fn snapped(app: &AppState, c: Pos2, frame: (u32, u32), dragged: &[ObjRef]) -> (Pos2, Guides) {
    if !app.settings.snap_to_guides {
        return (c, Guides::default());
    }
    let others: Vec<(f64, f64)> = crate::placement::rects_at(&app.project, app.playhead, frame)
        .into_iter()
        .filter(|(o, _)| !dragged.contains(o))
        .map(|(_, r)| (r.cx, r.cy))
        .collect();
    let ((x, y), g) = crate::placement::snap((f64::from(c.x), f64::from(c.y)), frame, &others);
    (pos2(x as f32, y as f32), g)
}

/// Draws the guide lines a snapped box sits on, over the picture.
pub fn draw_guides(ui: &Ui, g: &Guides, picture: Rect, frame: (u32, u32)) {
    let s = picture.width() / frame.0.max(1) as f32;
    let stroke = Stroke::new(1.0, Color32::from_rgb(255, 80, 200));
    let painter = ui.painter_at(picture);
    if let Some(x) = g.x {
        let sx = picture.min.x + x as f32 * s;
        painter.line_segment([pos2(sx, picture.min.y), pos2(sx, picture.max.y)], stroke);
    }
    if let Some(y) = g.y {
        let sy = picture.min.y + y as f32 * s;
        painter.line_segment([pos2(picture.min.x, sy), pos2(picture.max.x, sy)], stroke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subtitles::{NewLine, Word};
    use crate::timeline::tests::media;

    /// 10 px per character, one line of the font size.
    fn measure(t: &str, size: f32, _wrap: f32) -> Vec2 {
        vec2(t.chars().count() as f32 * 10.0, size)
    }

    fn project(preset: Preset) -> Project {
        let mut p = Project::default();
        let m = media(1, 10, true);
        p.add_media(m.clone());
        p.timeline.append(&m).unwrap();
        p.subtitles.preset = preset;
        p.subtitles.replace_all(
            vec![
                NewLine {
                    media: m.id,
                    start: Micros(0),
                    end: Micros(4_000_000),
                    text: "one two three four five six".into(),
                    words: vec![],
                },
                NewLine {
                    media: m.id,
                    start: Micros(1_000_000),
                    end: Micros(2_000_000),
                    text: "top".into(),
                    words: vec![Word {
                        start: Micros(1_000_000),
                        end: Micros(2_000_000),
                        text: "top".into(),
                    }],
                },
            ],
            None,
        );
        p
    }

    // covers: REQ-089
    #[test]
    fn boxes_sit_at_the_style_place_or_their_own_position() {
        let frame = (1280, 720);
        let mut p = project(Preset::Classic);
        let b = boxes(&p, Micros(1_500_000), frame, &mut measure);
        assert_eq!(b.len(), 2);
        let (_, r) = b[0];
        // centred horizontally, bottom at the style's margin
        assert!((r.center().x - 640.0).abs() < 0.5);
        let bottom = 720.0 - ass::margin_v(Preset::Classic, 720) as f32;
        assert!((r.max.y - bottom).abs() < 0.5, "{r:?}");
        // an own position: the centre is there
        let id = p.subtitles.lines[1].id;
        p.subtitles
            .set_position(&[id], Some(Pos { x: 2500, y: 2000 }))
            .unwrap();
        let b = boxes(&p, Micros(1_500_000), frame, &mut measure);
        assert_eq!(b[1].1.center(), pos2(320.0, 144.0));
        // the topmost box wins
        assert_eq!(hit(&b, pos2(320.0, 144.0)).unwrap().0, id);
        assert!(hit(&b, pos2(5.0, 5.0)).is_none());
        // word highlight measures only the words on screen
        let w = project(Preset::WordHighlight);
        let l = &w.subtitles.project(&w.timeline)[0];
        assert_eq!(
            visible_text(l, Preset::WordHighlight, Micros(100_000)),
            "one two three four"
        );
        assert_eq!(
            visible_text(l, Preset::WordHighlight, Micros(3_900_000)),
            "five six"
        );
        assert_eq!(visible_text(l, Preset::Classic, Micros(0)), l.text);
    }

    // covers: REQ-089, REQ-117
    #[test]
    fn a_dragged_box_may_go_partly_past_an_edge_but_its_centre_stays_on_the_picture() {
        let mut d = Drag {
            texts: Vec::new(),
            line: LineId(1),
            ids: vec![LineId(1)],
            grab: vec2(10.0, 0.0),
            size: vec2(200.0, 40.0),
            frame: (1000, 500),
            whole_box: false,
        };
        // the centre stops on the edge itself
        assert_eq!(drag_to(&d, pos2(-500.0, 250.0)), Pos { x: 0, y: 5000 });
        assert_eq!(drag_to(&d, pos2(5000.0, 250.0)), Pos { x: 10_000, y: 5000 });
        assert_eq!(drag_to(&d, pos2(500.0, -50.0)).y, 0);
        assert_eq!(drag_to(&d, pos2(500.0, 900.0)).y, 10_000);
        // a box 30 % of its width outside the left edge: allowed
        assert_eq!(drag_to(&d, pos2(40.0, 250.0)).x, 300);
        assert_eq!(
            clamp_to_picture(pos2(f32::NAN, 1.0), (1000, 500)),
            pos2(500.0, 1.0)
        );
        // on a 360 view the whole box stays in
        d.whole_box = true;
        assert_eq!(drag_to(&d, pos2(-500.0, 250.0)), Pos { x: 1000, y: 5000 });
    }

    // covers: REQ-089
    #[test]
    fn a_dragged_whole_box_stays_inside_the_picture() {
        let d = Drag {
            texts: Vec::new(),
            line: LineId(1),
            ids: vec![LineId(1)],
            grab: vec2(10.0, 0.0),
            size: vec2(200.0, 40.0),
            frame: (1000, 500),
            whole_box: true,
        };
        // past each edge: stops with the box touching it
        assert_eq!(drag_to(&d, pos2(-500.0, 250.0)), Pos { x: 1000, y: 5000 });
        assert_eq!(drag_to(&d, pos2(5000.0, 250.0)), Pos { x: 9000, y: 5000 });
        assert_eq!(drag_to(&d, pos2(500.0, -50.0)).y, 400);
        assert_eq!(drag_to(&d, pos2(500.0, 900.0)).y, 9600);
        // inside: follows the pointer minus the grab offset
        assert_eq!(drag_to(&d, pos2(260.0, 100.0)), Pos { x: 2500, y: 2000 });
        // a box larger than the picture is centred
        assert_eq!(
            clamp_centre(pos2(0.0, 0.0), vec2(2000.0, 10.0), (1000, 500)).x,
            500.0
        );
        assert_eq!(
            clamp_centre(pos2(f32::NAN, 1.0), vec2(1.0, 1.0), (10, 10)).x,
            5.0
        );
        assert_eq!(to_pos(pos2(1.0, 1.0), (0, 0)), Pos { x: 5000, y: 5000 });
        // screen <-> frame
        let picture = Rect::from_min_size(pos2(100.0, 50.0), vec2(500.0, 250.0));
        let f = to_frame(pos2(350.0, 175.0), picture, (1000, 500));
        assert_eq!(f, pos2(500.0, 250.0));
        let s = to_screen(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 500.0)),
            picture,
            (1000, 500),
        );
        assert_eq!(s, picture);
        assert_eq!(
            to_frame(pos2(1.0, 1.0), Rect::NOTHING, (0, 0)),
            pos2(0.0, 0.0)
        );
    }

    // covers: REQ-260
    #[test]
    fn an_item_out_of_view_has_an_arrow_at_the_nearest_edge_that_turns_to_it() {
        use crate::base::pos::Anchor360;
        use crate::camera::Camera;
        let cam = Camera::new(0, 0, 9000);
        let frame = (1600, 900);
        let at = |yaw_cd, pitch_cd| Anchor360 {
            yaw_cd,
            pitch_cd,
            height_cd: 500,
            width_cd: 5000,
        };
        // in view: no arrow
        assert!(off_view_arrow(cam, frame, &at(0, 0)).is_none());
        assert!(off_view_arrow(cam, frame, &at(1000, 500)).is_none());
        // yaw 90 while the view looks at 0: right edge, pointing right, turn of 90 degrees
        let a = off_view_arrow(cam, frame, &at(9000, 0)).unwrap();
        assert!(
            a.at.x > 1600.0 - 40.0 && (a.at.y - 450.0).abs() < 1.0,
            "{a:?}"
        );
        assert!(a.dir.x > 0.9);
        assert_eq!(a.turn, (9000, 0));
        // to the left, behind, above
        assert!(off_view_arrow(cam, frame, &at(-9000, 0)).unwrap().at.x < 40.0);
        let b = off_view_arrow(cam, frame, &at(18_000, 0)).unwrap();
        assert_eq!(b.turn.0.abs(), 18_000);
        let up = off_view_arrow(cam, frame, &at(0, 8000)).unwrap();
        assert!(up.at.y < 40.0 && up.turn == (0, 8000), "{up:?}");
        // turning by the arrow's turn centres the item
        let turned = cam.turned(a.turn.0, a.turn.1);
        assert!(off_view_arrow(turned, frame, &at(9000, 0)).is_none());
    }
}
