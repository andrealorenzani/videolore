// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The virtual camera of 360° video: pure maths, no I/O.
//!
//! A camera is a direction (yaw, pitch) and a zoom (horizontal field of view). Values are
//! whole hundredths of a degree so they compare exactly (frame keys, golden tests). A
//! point of view (POV) is a camera at a time of the SOURCE file; between two POVs the
//! camera moves linearly in time, yaw the short way round.
//!
//! Conventions measured on ffmpeg's `v360` (spike of 1.2.6): positive yaw looks right,
//! positive pitch looks up, angles in degrees, `d_fov` (diagonal) with `w`/`h` gives the
//! rectilinear horizontal field of view we ask for. Runtime commands of `v360` ADD to the
//! rotation unless the filter has `reset_rot=1`: with it every command is absolute.

use crate::media::MediaId;
use crate::time::{Fps, Micros};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Most POVs one source can have.
pub const MAX_POVS: usize = 200;
pub const MIN_FOV_CD: i32 = 3000;
pub const MAX_FOV_CD: i32 = 11000;
pub const DEFAULT_FOV_CD: i32 = 9000;
const FULL_TURN_CD: i32 = 36000;
const HALF_TURN_CD: i32 = 18000;
const MAX_PITCH_CD: i32 = 9000;
/// Longest command file (lines); a longer path is written with a coarser step.
pub const MAX_COMMAND_LINES: i64 = 1_000_000;
/// Wheel sensitivity: zoom factor per scroll point.
const WHEEL_K: f64 = 0.0015;

/// A view direction and zoom, in hundredths of a degree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Camera {
    /// `(-18000, 18000]`; positive looks right.
    pub yaw_cd: i32,
    /// `[-9000, 9000]`; positive looks up.
    pub pitch_cd: i32,
    /// Horizontal field of view, `[3000, 11000]`.
    pub fov_cd: i32,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera::FRONT
    }
}

fn wrap_yaw(cd: i64) -> i32 {
    let m = cd.rem_euclid(FULL_TURN_CD as i64) as i32;
    if m > HALF_TURN_CD {
        m - FULL_TURN_CD
    } else if m == 0 {
        0
    } else {
        m
    }
}

impl Camera {
    /// Front, level, 90° of view.
    pub const FRONT: Camera = Camera {
        yaw_cd: 0,
        pitch_cd: 0,
        fov_cd: DEFAULT_FOV_CD,
    };

    /// Builds a camera from any values, bringing them into range (yaw wraps, the rest
    /// are clamped).
    pub fn new(yaw_cd: i64, pitch_cd: i64, fov_cd: i64) -> Camera {
        Camera {
            yaw_cd: wrap_yaw(yaw_cd),
            pitch_cd: pitch_cd.clamp(-(MAX_PITCH_CD as i64), MAX_PITCH_CD as i64) as i32,
            fov_cd: fov_cd.clamp(MIN_FOV_CD as i64, MAX_FOV_CD as i64) as i32,
        }
    }

    /// The same camera brought into range.
    pub fn clamped(self) -> Camera {
        Camera::new(self.yaw_cd as i64, self.pitch_cd as i64, self.fov_cd as i64)
    }

    pub fn is_in_range(self) -> bool {
        self == self.clamped()
    }

    pub fn yaw_deg(self) -> f64 {
        self.yaw_cd as f64 / 100.0
    }

    pub fn pitch_deg(self) -> f64 {
        self.pitch_cd as f64 / 100.0
    }

    pub fn fov_deg(self) -> f64 {
        self.fov_cd as f64 / 100.0
    }

    /// The camera after dragging the picture by `dx`/`dy` pixels (screen coordinates,
    /// y down) on a preview `view_w` pixels wide: `angle = pixels * fov / view_w`, the
    /// picture follows the pointer.
    pub fn dragged(self, dx: f32, dy: f32, view_w: f32) -> Camera {
        if !(view_w.is_finite() && view_w > 0.0 && dx.is_finite() && dy.is_finite()) {
            return self;
        }
        let k = self.fov_deg() / view_w as f64 * 100.0;
        Camera::new(
            self.yaw_cd as i64 - (dx as f64 * k).round() as i64,
            self.pitch_cd as i64 + (dy as f64 * k).round() as i64,
            self.fov_cd as i64,
        )
    }

    /// The camera after `scroll` points of wheel (positive zooms in) or a pinch.
    pub fn zoomed(self, scroll: f32) -> Camera {
        if !scroll.is_finite() {
            return self;
        }
        let f = (-WHEEL_K * scroll as f64).exp();
        Camera::new(
            self.yaw_cd as i64,
            self.pitch_cd as i64,
            (self.fov_cd as f64 * f).round() as i64,
        )
    }

    /// Camera `frac` (0..=1) of the way from `self` to `to`: yaw the short way round
    /// (a half turn goes the positive way), pitch and zoom directly.
    pub fn lerp(self, to: Camera, frac: f64) -> Camera {
        let mut d = (to.yaw_cd - self.yaw_cd).rem_euclid(FULL_TURN_CD);
        if d > HALF_TURN_CD {
            d -= FULL_TURN_CD;
        }
        let mix = |a: i32, b: i32| a as f64 + (b - a) as f64 * frac;
        Camera::new(
            (self.yaw_cd as f64 + d as f64 * frac).round() as i64,
            mix(self.pitch_cd, to.pitch_cd).round() as i64,
            mix(self.fov_cd, to.fov_cd).round() as i64,
        )
    }
}

/// A camera at a time of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pov {
    pub t: Micros,
    pub cam: Camera,
}

/// The camera of a path (sorted by time, unique times) at source time `t`: it stays on
/// the first POV before it and on the last after it. `None` for an empty path.
pub fn at(path: &[Pov], t: Micros) -> Option<Camera> {
    let first = path.first()?;
    let last = path.last()?;
    if t <= first.t {
        return Some(first.cam);
    }
    if t >= last.t {
        return Some(last.cam);
    }
    let i = path.partition_point(|p| p.t <= t);
    let (a, b) = (&path[i - 1], &path[i]);
    let frac = (t.0 - a.t.0) as f64 / (b.t.0 - a.t.0) as f64;
    Some(a.cam.lerp(b.cam, frac))
}

/// The POVs of every 360 source of a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CameraPaths {
    map: BTreeMap<MediaId, Vec<Pov>>,
}

#[derive(Serialize, Deserialize)]
struct PathEntry {
    media: MediaId,
    povs: Vec<Pov>,
}

impl Serialize for CameraPaths {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v: Vec<PathEntry> = self
            .map
            .iter()
            .filter(|(_, p)| !p.is_empty())
            .map(|(m, p)| PathEntry {
                media: *m,
                povs: p.clone(),
            })
            .collect();
        v.serialize(s)
    }
}

impl<'de> Deserialize<'de> for CameraPaths {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<CameraPaths, D::Error> {
        let v = Vec::<PathEntry>::deserialize(d)?;
        let mut map = BTreeMap::new();
        for e in v {
            if map.insert(e.media, e.povs).is_some() {
                return Err(serde::de::Error::custom("duplicate camera path"));
            }
        }
        Ok(CameraPaths { map })
    }
}

/// Why a POV could not be stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PovError {
    TooMany,
    BadTime,
}

impl CameraPaths {
    pub fn is_empty(&self) -> bool {
        self.map.values().all(Vec::is_empty)
    }

    /// The POVs of a source (sorted).
    pub fn path(&self, media: MediaId) -> &[Pov] {
        self.map.get(&media).map_or(&[], Vec::as_slice)
    }

    pub fn media_ids(&self) -> impl Iterator<Item = MediaId> + '_ {
        self.map.keys().copied()
    }

    /// Stores a POV; one at the same time is replaced.
    pub fn set(&mut self, media: MediaId, pov: Pov) -> Result<(), PovError> {
        if pov.t < Micros::ZERO {
            return Err(PovError::BadTime);
        }
        let v = self.map.entry(media).or_default();
        match v.binary_search_by_key(&pov.t, |p| p.t) {
            Ok(i) => v[i] = pov,
            Err(i) => {
                if v.len() >= MAX_POVS {
                    return Err(PovError::TooMany);
                }
                v.insert(i, pov);
            }
        }
        Ok(())
    }

    /// Removes the POV at exactly `t`.
    pub fn remove(&mut self, media: MediaId, t: Micros) -> bool {
        let Some(v) = self.map.get_mut(&media) else {
            return false;
        };
        let Ok(i) = v.binary_search_by_key(&t, |p| p.t) else {
            return false;
        };
        v.remove(i);
        if v.is_empty() {
            self.map.remove(&media);
        }
        true
    }

    /// Replaces the POV at `old_t` by `pov` (which may have another time).
    pub fn edit(&mut self, media: MediaId, old_t: Micros, pov: Pov) -> Result<(), PovError> {
        if !self.remove(media, old_t) {
            return Err(PovError::BadTime);
        }
        self.set(media, pov)
    }

    /// Forgets the path of a source.
    pub fn drop_media(&mut self, media: MediaId) {
        self.map.remove(&media);
    }

    /// Checks a loaded value: structure errors are returned, views are brought into
    /// range. `duration` gives the length of a source (`None`: unknown source).
    pub fn validate(
        &mut self,
        duration: impl Fn(MediaId) -> Option<Micros>,
    ) -> Result<(), &'static str> {
        for (m, v) in &mut self.map {
            let d = duration(*m).ok_or("camera path of an unknown media")?;
            if v.len() > MAX_POVS {
                return Err("too many points of view");
            }
            for p in v.iter_mut() {
                if p.t < Micros::ZERO || p.t > d {
                    return Err("point of view outside its source");
                }
                p.cam = p.cam.clamped();
            }
            v.sort_by_key(|p| p.t);
            if v.windows(2).any(|w| w[0].t == w[1].t) {
                return Err("two points of view at the same time");
            }
        }
        Ok(())
    }
}

/// Unit vectors (right, up, forward) of a view looking at `yaw`, `pitch` (degrees; positive
/// yaw looks right, positive pitch up), in a frame where x is right, y up, z front.
fn basis(yaw: f64, pitch: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let (sy, cy) = yaw.to_radians().sin_cos();
    let (sp, cp) = pitch.to_radians().sin_cos();
    (
        [cy, 0.0, -sy],
        [-sp * sy, cp, -sp * cy],
        [cp * sy, sp, cp * cy],
    )
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Where the point of the sphere at (`yaw_deg`, `pitch_deg`) is on the picture of `w` x
/// `h` pixels a rectilinear camera `cam` makes (straight lines stay straight): pixel
/// coordinates from the top left, or `None` when the point is behind the camera.
pub fn project(cam: Camera, w: u32, h: u32, yaw_deg: f64, pitch_deg: f64) -> Option<(f64, f64)> {
    let (r, u, f) = basis(cam.yaw_deg(), cam.pitch_deg());
    let (_, _, v) = basis(yaw_deg, pitch_deg);
    let depth = dot(v, f);
    if depth <= 1e-6 {
        return None;
    }
    let focal = (f64::from(w) / 2.0) / (cam.fov_deg().to_radians() / 2.0).tan();
    Some((
        f64::from(w) / 2.0 + focal * dot(v, r) / depth,
        f64::from(h) / 2.0 - focal * dot(v, u) / depth,
    ))
}

/// The point of the sphere (`yaw`, `pitch` in degrees, yaw in `(-180, 180]`) that the
/// pixel (`x`, `y`) of the picture of `w` x `h` shows: the inverse of [`project`].
pub fn unproject(cam: Camera, w: u32, h: u32, x: f64, y: f64) -> (f64, f64) {
    let (r, u, f) = basis(cam.yaw_deg(), cam.pitch_deg());
    let focal = (f64::from(w) / 2.0) / (cam.fov_deg().to_radians() / 2.0).tan();
    let (dx, dy) = (x - f64::from(w) / 2.0, f64::from(h) / 2.0 - y);
    let v: Vec<f64> = (0..3)
        .map(|i| f[i] * focal + r[i] * dx + u[i] * dy)
        .collect();
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    let yaw = v[0].atan2(v[2]).to_degrees();
    let pitch = (v[1] / n).clamp(-1.0, 1.0).asin().to_degrees();
    (if yaw <= -180.0 { 180.0 } else { yaw }, pitch)
}

/// The anchor of a text added now with the camera `cam` showing a `w` x `h` picture: the
/// centre of the view; the text is 5% of the view's height tall at 100% and wraps at 60%
/// of its width (assumptions of REQ-112).
pub fn anchor_at(cam: Camera, w: u32, h: u32) -> crate::base::pos::Anchor360 {
    let vfov = vertical_fov_deg(cam.fov_deg(), w, h);
    crate::base::pos::Anchor360 {
        yaw_cd: cam.yaw_cd,
        pitch_cd: cam.pitch_cd,
        height_cd: ((vfov * 100.0 * 0.05).round() as u32).clamp(1, 17_000),
        width_cd: ((cam.fov_deg() * 100.0 * 0.6).round() as u32).clamp(1, 36_000),
    }
}

/// The anchor of a text item added now (REQ-111, REQ-116): like [`anchor_at`] but with a
/// box 20% of the view's height tall, as the flat box is 20% of the picture's height.
pub fn text_anchor_at(cam: Camera, w: u32, h: u32) -> crate::base::pos::Anchor360 {
    let vfov = vertical_fov_deg(cam.fov_deg(), w, h);
    crate::base::pos::Anchor360 {
        height_cd: ((vfov * 100.0 * 0.2).round() as u32).clamp(1, 18_000),
        ..anchor_at(cam, w, h)
    }
}

/// Vertical field of view of a rectilinear view `h_fov` wide on a `w` x `h` picture.
pub fn vertical_fov_deg(h_fov_deg: f64, w: u32, h: u32) -> f64 {
    let t = (h_fov_deg.to_radians() / 2.0).tan() * h as f64 / w.max(1) as f64;
    (2.0 * t.atan()).to_degrees()
}

/// Diagonal field of view (what `v360` is commanded with: it derives both others from
/// the output aspect), limited below 170°.
pub fn diagonal_fov_deg(h_fov_deg: f64, w: u32, h: u32) -> f64 {
    let diag = (w as f64).hypot(h as f64) / w.max(1) as f64;
    let t = (h_fov_deg.to_radians() / 2.0).tan() * diag;
    (2.0 * t.atan()).to_degrees().min(170.0)
}

/// The `v360` filter named `v360@<id>` (so a command file can address it alone) for a
/// camera, `w` x `h` output (initial values: those at the segment start).
pub fn filter(id: &str, cam: Camera, w: u32, h: u32) -> String {
    format!(
        "v360@{id}=input=e:output=flat:reset_rot=1:w={w}:h={h}:d_fov={:.4}:yaw={:.2}:pitch={:.2}:interp=linear",
        diagonal_fov_deg(cam.fov_deg(), w, h),
        cam.yaw_deg(),
        cam.pitch_deg()
    )
}

/// The `sendcmd` file of a segment showing source time `src_in..src_out` at `fps`: one
/// line for each frame at which the camera differs from the previous one, carrying only
/// the values that changed. Times are relative to the segment start; each command has
/// an interval of half a frame either side of its frame (a lone instant would need the
/// frame's rounded time to match exactly: measured with `sendcmd` of ffmpeg 6.1), so it
/// reaches that frame and no other. A still camera gives an empty file. Numbers only,
/// all formatted here.
pub fn command_lines(
    target: &str,
    path: &[Pov],
    src_in: Micros,
    src_out: Micros,
    fps: Fps,
    w: u32,
    h: u32,
) -> String {
    let mut out = String::new();
    if path.len() < 2 || src_out <= src_in {
        return out;
    }
    let frames = fps.frame_index(src_out - src_in).max(1);
    let step = ((frames + MAX_COMMAND_LINES - 1) / MAX_COMMAND_LINES).max(1);
    let half = fps.frame_duration().0 / 2;
    let mut prev = at(path, src_in).unwrap_or_default();
    let mut k = step;
    while k < frames {
        let cam = at(path, src_in + fps.frame_time(k)).unwrap_or_default();
        if cam != prev {
            let t = fps.frame_time(k).0;
            let _ = write!(
                out,
                "{}-{}",
                Micros((t - half).max(0)).ff(),
                Micros(t + half).ff()
            );
            let mut sep = " ";
            if cam.yaw_cd != prev.yaw_cd {
                let _ = write!(out, "{sep}{target} yaw {:.2}", cam.yaw_deg());
                sep = ", ";
            }
            if cam.pitch_cd != prev.pitch_cd {
                let _ = write!(out, "{sep}{target} pitch {:.2}", cam.pitch_deg());
                sep = ", ";
            }
            if cam.fov_cd != prev.fov_cd {
                let _ = write!(
                    out,
                    "{sep}{target} d_fov {:.4}",
                    diagonal_fov_deg(cam.fov_deg(), w, h)
                );
            }
            out.push_str(";\n");
            prev = cam;
        }
        k += step;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-112
    #[test]
    fn a_point_straight_ahead_is_at_the_centre_and_the_projection_inverts() {
        let cam = Camera::new(3000, 1000, 9000);
        let (x, y) = project(cam, 1600, 900, 30.0, 10.0).unwrap();
        assert!(
            (x - 800.0).abs() < 1e-6 && (y - 450.0).abs() < 1e-6,
            "{x} {y}"
        );
        // 45 degrees to the right on the horizon of a 90 degrees camera: the right edge
        let (x, y) = project(Camera::FRONT, 1600, 900, 45.0, 0.0).unwrap();
        assert!((x - 1600.0).abs() < 1e-6 && (y - 450.0).abs() < 1e-6);
        // up is up, left is left
        let (_, y) = project(Camera::FRONT, 1600, 900, 0.0, 10.0).unwrap();
        assert!(y < 450.0);
        let (x, _) = project(Camera::FRONT, 1600, 900, -20.0, 0.0).unwrap();
        assert!(x < 800.0);
        // behind the camera
        assert!(project(Camera::FRONT, 1600, 900, 180.0, 0.0).is_none());
        assert!(project(Camera::FRONT, 1600, 900, 90.0, 0.0).is_none());
        // inverse
        for (yaw, pitch, px, py) in [(10.0, 5.0, 100.0, 80.0), (-170.0, -30.0, 1500.0, 800.0)] {
            let cam = Camera::new((yaw * 100.0) as i64, (pitch * 100.0) as i64, 7000);
            let (a, b) = unproject(cam, 1600, 900, px, py);
            let (qx, qy) = project(cam, 1600, 900, a, b).unwrap();
            assert!(
                (qx - px).abs() < 1e-6 && (qy - py).abs() < 1e-6,
                "{qx} {qy}"
            );
        }
        let (yaw, pitch) = unproject(Camera::new(-18000, 0, 9000), 1600, 900, 800.0, 450.0);
        assert_eq!((yaw.round(), pitch.round()), (180.0, 0.0));
    }

    // covers: REQ-112
    #[test]
    fn a_fixed_point_moves_left_when_the_camera_turns_right_and_returns() {
        let (w, h) = (1600, 900);
        let at = Camera::new(3000, 1000, 9000);
        // the item added with the view at yaw 30, pitch 10
        let (yaw, pitch) = (30.0, 10.0);
        // after the camera turned 40 degrees to the right it is on the left of the centre,
        // 40 degrees away measured on the horizontal plane (the angle seen from the centre)
        let turned = Camera::new(7000, 1000, 9000);
        let (x, y) = project(turned, w, h, yaw, pitch).unwrap();
        assert!(x < 800.0);
        let (back_yaw, back_pitch) = unproject(turned, w, h, x, y);
        assert!((back_yaw - yaw).abs() < 0.01 && (back_pitch - pitch).abs() < 0.01);
        let c = unproject(turned, w, h, 800.0, 450.0);
        assert!((c.0 - 70.0).abs() < 0.01);
        assert!(
            ((yaw - c.0).abs() - 40.0).abs() <= 1.0,
            "{} deg apart",
            (yaw - c.0).abs()
        );
        // turning back puts it where it was
        let (x0, y0) = project(at, w, h, yaw, pitch).unwrap();
        assert!((x0 - 800.0).abs() < 1e-6 && (y0 - 450.0).abs() < 1e-6);
        // zooming in makes it farther from the centre (bigger, like the scene)
        let zoomed = Camera::new(7000, 1000, 5000);
        let (xz, _) = project(zoomed, w, h, yaw, pitch).unwrap();
        assert!(xz < x);
    }

    // covers: REQ-112
    #[test]
    fn the_anchor_of_a_new_item_is_the_view_centre_with_the_size_of_the_view() {
        let a = anchor_at(Camera::new(3000, 1000, 9000), 1600, 900);
        assert_eq!((a.yaw_cd, a.pitch_cd), (3000, 1000));
        // 90 degrees wide: 60% = 54 degrees wrap; the view is about 58.7 degrees high: 5%
        assert_eq!(a.width_cd, 5400);
        assert!((a.height_cd as i64 - 293).abs() <= 2, "{}", a.height_cd);
        assert!(a.is_valid());
    }
    fn pov(t_s: i64, yaw: i32) -> Pov {
        Pov {
            t: Micros(t_s * 1_000_000),
            cam: Camera::new(yaw as i64 * 100, 0, 9000),
        }
    }

    // covers: REQ-102
    #[test]
    fn midpoint_is_45_degrees() {
        let p = [pov(0, 0), pov(10, 90)];
        let c = at(&p, Micros(5_000_000)).unwrap();
        assert!((c.yaw_deg() - 45.0).abs() < 0.01);
    }

    // covers: REQ-102
    #[test]
    fn holds_before_and_after_and_single_pov() {
        let p = [pov(2, 30), pov(4, 60)];
        assert_eq!(at(&p, Micros(0)).unwrap().yaw_cd, 3000);
        assert_eq!(at(&p, Micros(9_000_000)).unwrap().yaw_cd, 6000);
        let one = [pov(3, -20)];
        assert_eq!(at(&one, Micros(0)).unwrap().yaw_cd, -2000);
        assert_eq!(at(&one, Micros(8_000_000)).unwrap().yaw_cd, -2000);
        assert_eq!(at(&[], Micros(0)), None);
    }

    // covers: REQ-102
    #[test]
    fn yaw_takes_the_short_way_round() {
        let p = [pov(0, 170), pov(10, -170)];
        let mid = at(&p, Micros(5_000_000)).unwrap();
        assert_eq!(mid.yaw_cd, 18000);
        let q = [pov(0, -10), pov(10, 10)];
        assert_eq!(at(&q, Micros(5_000_000)).unwrap().yaw_cd, 0);
        let r = [pov(0, 0), pov(10, 180)];
        assert!(at(&r, Micros(2_500_000)).unwrap().yaw_cd > 0);
    }

    #[test]
    fn pitch_and_zoom_interpolate_directly() {
        let a = Camera::new(0, -3000, 4000);
        let b = Camera::new(0, 3000, 10000);
        let m = a.lerp(b, 0.5);
        assert_eq!((m.pitch_cd, m.fov_cd), (0, 7000));
    }

    #[test]
    fn clamps_and_wraps() {
        let c = Camera::new(36100, 12000, 100);
        assert_eq!((c.yaw_cd, c.pitch_cd, c.fov_cd), (100, 9000, 3000));
        assert_eq!(Camera::new(-18000, -20000, 99999).yaw_cd, 18000);
        assert_eq!(Camera::new(0, 0, 99999).fov_cd, 11000);
        assert!(Camera::FRONT.is_in_range());
        assert!(
            !Camera {
                yaw_cd: 40000,
                ..Camera::FRONT
            }
            .is_in_range()
        );
        assert_eq!(Camera::default(), Camera::FRONT);
    }

    // covers: REQ-101
    #[test]
    fn drag_of_100_pixels_is_the_same_angle_everywhere() {
        let c = Camera::FRONT;
        // 90 degrees over 900 pixels: 100 px = 10 degrees, picture follows the pointer
        let d = c.dragged(100.0, 0.0, 900.0);
        assert_eq!(d.yaw_cd, -1000);
        let e = c.dragged(0.0, 100.0, 900.0);
        assert_eq!(e.pitch_cd, 1000);
        assert_eq!(c.dragged(f32::NAN, 0.0, 900.0), c);
        assert_eq!(c.dragged(1.0, 0.0, 0.0), c);
    }

    // covers: REQ-101
    #[test]
    fn pitch_stops_at_the_poles_and_zoom_stays_in_range() {
        let c = Camera::FRONT.dragged(0.0, 100_000.0, 900.0);
        assert_eq!(c.pitch_cd, 9000);
        let c = Camera::FRONT.dragged(0.0, -100_000.0, 900.0);
        assert_eq!(c.pitch_cd, -9000);
        assert_eq!(Camera::FRONT.zoomed(100_000.0).fov_cd, 3000);
        assert_eq!(Camera::FRONT.zoomed(-100_000.0).fov_cd, 11000);
        assert!(Camera::FRONT.zoomed(100.0).fov_cd < 9000);
        assert_eq!(Camera::FRONT.zoomed(f32::INFINITY), Camera::FRONT);
        // free turn around
        let w = Camera::FRONT.dragged(-100_000.0, 0.0, 900.0);
        assert!(w.yaw_cd > -18000 && w.yaw_cd <= 18000);
    }

    #[test]
    fn set_replace_remove_and_limit() {
        let mut p = CameraPaths::default();
        let m = MediaId(1);
        assert!(p.is_empty());
        p.set(m, pov(5, 10)).unwrap();
        p.set(m, pov(1, 20)).unwrap();
        p.set(m, pov(5, 30)).unwrap();
        assert_eq!(p.path(m).len(), 2);
        assert_eq!(p.path(m)[0].t, Micros(1_000_000));
        assert_eq!(p.path(m)[1].cam.yaw_cd, 3000);
        assert_eq!(
            p.set(
                m,
                Pov {
                    t: Micros(-1),
                    cam: Camera::FRONT
                }
            ),
            Err(PovError::BadTime)
        );
        assert!(p.remove(m, Micros(1_000_000)));
        assert!(!p.remove(m, Micros(1_000_000)));
        assert!(!p.remove(MediaId(9), Micros(0)));
        p.edit(m, Micros(5_000_000), pov(2, 1)).unwrap();
        assert_eq!(p.path(m)[0].t, Micros(2_000_000));
        assert_eq!(p.edit(m, Micros(7), pov(2, 1)), Err(PovError::BadTime));
        assert_eq!(p.media_ids().count(), 1);
        p.drop_media(m);
        assert!(p.is_empty());
        for i in 0..MAX_POVS as i64 {
            p.set(
                m,
                Pov {
                    t: Micros(i),
                    cam: Camera::FRONT,
                },
            )
            .unwrap();
        }
        assert_eq!(
            p.set(
                m,
                Pov {
                    t: Micros(100_000),
                    cam: Camera::FRONT
                }
            ),
            Err(PovError::TooMany)
        );
        // replacing at the limit still works
        p.set(
            m,
            Pov {
                t: Micros(3),
                cam: Camera::new(5, 0, 9000),
            },
        )
        .unwrap();
    }

    #[test]
    fn validation_refuses_structure_and_fixes_views() {
        let dur = |m: MediaId| (m.0 == 1).then_some(Micros(10_000_000));
        let ok = |p: &mut CameraPaths| p.validate(dur);
        let mut p = CameraPaths::default();
        p.map.insert(
            MediaId(1),
            vec![
                Pov {
                    t: Micros(2),
                    cam: Camera {
                        yaw_cd: 99999,
                        pitch_cd: 99999,
                        fov_cd: 1,
                    },
                },
                Pov {
                    t: Micros(1),
                    cam: Camera::FRONT,
                },
            ],
        );
        ok(&mut p).unwrap();
        let v = p.path(MediaId(1));
        assert_eq!(v[0].t, Micros(1));
        assert!(v[1].cam.is_in_range());
        let mut q = CameraPaths::default();
        q.map.insert(MediaId(2), vec![pov(1, 1)]);
        assert!(ok(&mut q).is_err());
        let mut q = CameraPaths::default();
        q.map.insert(MediaId(1), vec![pov(11, 1)]);
        assert!(ok(&mut q).is_err());
        let mut q = CameraPaths::default();
        q.map.insert(MediaId(1), vec![pov(1, 1), pov(1, 2)]);
        assert!(ok(&mut q).is_err());
        let mut q = CameraPaths::default();
        q.map.insert(
            MediaId(1),
            (0..201)
                .map(|i| Pov {
                    t: Micros(i),
                    cam: Camera::FRONT,
                })
                .collect(),
        );
        assert!(ok(&mut q).is_err());
    }

    #[test]
    fn json_round_trip_and_duplicates() {
        let mut p = CameraPaths::default();
        p.set(MediaId(3), pov(1, 5)).unwrap();
        let j = serde_json::to_string(&p).unwrap();
        let q: CameraPaths = serde_json::from_str(&j).unwrap();
        assert_eq!(p, q);
        let dup = r#"[{"media":1,"povs":[]},{"media":1,"povs":[]}]"#;
        assert!(serde_json::from_str::<CameraPaths>(dup).is_err());
        assert!(serde_json::from_str::<CameraPaths>("{\"a\":1}").is_err());
        assert!(
            serde_json::from_str::<CameraPaths>("[{\"media\":1,\"povs\":[{\"t\":1.5}]}]").is_err()
        );
    }

    #[test]
    fn field_of_view_maths() {
        let v = vertical_fov_deg(90.0, 1920, 1080);
        assert!((v - 58.7155).abs() < 0.01);
        let d = diagonal_fov_deg(90.0, 1920, 1080);
        assert!((d - 97.8508).abs() < 0.01);
        assert!(diagonal_fov_deg(110.0, 100, 10_000) <= 170.0);
    }

    #[test]
    fn filter_text_is_numbers_only() {
        let f = filter("c0", Camera::new(1000, -500, 9000), 1920, 1080);
        assert_eq!(
            f,
            "v360@c0=input=e:output=flat:reset_rot=1:w=1920:h=1080:d_fov=97.8508:yaw=10.00:pitch=-5.00:interp=linear"
        );
    }

    // covers: REQ-102
    #[test]
    fn command_lines_only_while_the_camera_moves() {
        let fps = Fps::new(10, 1).unwrap();
        let p = [pov(0, 0), pov(1, 10), pov(5, 10)];
        let s = command_lines(
            "v360@c0",
            &p,
            Micros::ZERO,
            Micros(2_000_000),
            fps,
            1920,
            1080,
        );
        let lines: Vec<&str> = s.lines().collect();
        // frames 1..=9 move, frame 10 onwards holds
        assert_eq!(lines.len(), 10);
        assert_eq!(lines[0], "0.050000-0.150000 v360@c0 yaw 1.00;");
        assert!(lines[9].starts_with("0.950000-1.050000 v360@c0 yaw 10.00"));
        // a still camera or one POV gives nothing
        assert_eq!(
            command_lines(
                "v360@c0",
                &p[..1],
                Micros::ZERO,
                Micros(2_000_000),
                fps,
                1920,
                1080
            ),
            ""
        );
        assert_eq!(
            command_lines(
                "v360@c0",
                &p,
                Micros(2_000_000),
                Micros(4_000_000),
                fps,
                1920,
                1080
            ),
            ""
        );
        assert_eq!(
            command_lines("v360@c0", &p, Micros(5), Micros(5), fps, 1, 1),
            ""
        );
    }

    #[test]
    fn command_lines_carry_changed_values_and_use_segment_time() {
        let fps = Fps::new(10, 1).unwrap();
        let a = Pov {
            t: Micros::ZERO,
            cam: Camera::new(0, 0, 9000),
        };
        let b = Pov {
            t: Micros(1_000_000),
            cam: Camera::new(0, 1000, 5000),
        };
        let s = command_lines(
            "v360@c0",
            &[a, b],
            Micros::ZERO,
            Micros(1_000_000),
            fps,
            1920,
            1080,
        );
        let first = s.lines().next().unwrap();
        assert!(first.contains("v360@c0 pitch ") && first.contains("v360@c0 d_fov "));
        assert!(!first.contains("yaw"));
        // starting inside the path: times restart at the segment start
        let s2 = command_lines(
            "v360@c0",
            &[a, b],
            Micros(500_000),
            Micros(1_000_000),
            fps,
            1920,
            1080,
        );
        assert!(s2.starts_with("0.050000-0.150000 "));
    }

    #[test]
    fn long_paths_get_a_coarser_step() {
        let fps = Fps::new(30, 1).unwrap();
        let p = [pov(0, 0), pov(40_000_000, 90)];
        // 40e6 s is far beyond the cap: step > 1 keeps the file bounded
        let s = command_lines(
            "v360@c0",
            &p,
            Micros::ZERO,
            Micros(40_000_000 * 1_000_000 / 40),
            fps,
            1920,
            1080,
        );
        assert!(s.lines().count() as i64 <= MAX_COMMAND_LINES);
    }
}
