// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! 360 video in the app (REQ-100..102): the live view you drag, Set POV and its edits, and
//! the boxes' resize, align and reset commands (lines and text items, REQ-115)
//! (REQ-097..099) which share the "select something on the picture" rules.

use super::*;
use crate::base::pos::{POS_MAX, align_h, align_v};
use crate::camera::{self, Camera, Pov, PovError};
use crate::subtitles::SIZE_NORMAL;

/// Disabled reason of the commands that need an object on the picture.
pub const SELECT_ON_PICTURE: &str = "Select something on the picture first";
/// Disabled reason of Set POV.
pub const NOT_A_360: &str = "Move the playhead over a 360° clip first";
/// Disabled reason of Reset when everything selected is already original.
/// What Align does: the lines and texts that move, with their new place.
pub(super) type AlignPlan = (Vec<(LineId, Option<Pos>)>, Vec<(crate::text::TextId, Pos)>);

pub const ALREADY_CENTRED: &str = "Already centred";
pub const ALREADY_ORIGINAL: &str = "Everything selected is already in its original place and size";
/// Disabled reason of Reset on a 360 clip whose camera has not moved.
pub const CAMERA_AT_FRONT: &str = "The camera is already at the front";
/// Quiet time after the last drag before playback restarts with the new camera.
pub const CAMERA_RESTART: Duration = Duration::from_millis(100);

/// Which way Align works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Centre on the picture's vertical middle line (moves along x).
    Horizontal,
    /// Centre on the picture's horizontal middle line (moves along y).
    Vertical,
}

/// The camera you are looking through now (window state: not saved, not in undo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveView {
    pub cam: Camera,
    /// A drag took over from the saved POVs, until the next seek, play or Reset.
    pub overriding: bool,
}

/// Lines being resized in the preview, shown at once and saved on release.
#[derive(Debug, Clone, PartialEq)]
pub struct ResizeDrag {
    pub ids: Vec<LineId>,
    /// The line whose corner is dragged: it gets the new centre.
    pub line: LineId,
    pub pos: Pos,
    pub size: u16,
}

fn pov_error(e: PovError) -> String {
    match e {
        PovError::TooMany => format!(
            "a clip can have at most {} points of view",
            camera::MAX_POVS
        ),
        PovError::BadTime => "that point of view is outside the video".into(),
    }
}

impl AppState {
    /// The 360 clip under the playhead and the source time shown there.
    pub fn sphere_at_playhead(&self) -> Option<(MediaId, Micros)> {
        let tl = &self.project.timeline;
        let (i, start) = tl.locate(self.playhead)?;
        let c = tl.clips[i];
        let m = self.project.media(c.media)?;
        (m.is_ready() && !m.missing && m.format.is_360())
            .then_some((c.media, c.src_in + (self.playhead - start)))
    }

    /// Where on the timeline the POV of `media` at source time `t` is: in the first clip
    /// of the source whose kept part contains it (a POV outside every clip is hidden).
    pub fn pov_position(&self, media: MediaId, t: Micros) -> Option<Micros> {
        if !self.project.camera.path(media).iter().any(|p| p.t == t) {
            return None;
        }
        let tl = &self.project.timeline;
        tl.clips
            .iter()
            .enumerate()
            .find(|(_, c)| c.media == media && c.src_in <= t && t < c.src_out)
            .map(|(i, c)| tl.start_of(i) + (t - c.src_in))
    }

    /// The camera of 360 source `media` at source time `t`: the live one when a drag took
    /// over, else the saved path, else the live view (or front, level, 90°).
    pub fn effective_camera(&self, media: MediaId, t: Micros) -> Camera {
        let live = self.live.get(&media);
        if let Some(l) = live.filter(|l| l.overriding) {
            return l.cam;
        }
        camera::at(self.project.camera.path(media), t)
            .or(live.map(|l| l.cam))
            .unwrap_or(Camera::FRONT)
    }

    /// Where the 360 clip at the playhead looks now (for the preview's readout).
    pub fn camera_at_playhead(&self) -> Option<Camera> {
        self.sphere_at_playhead()
            .map(|(m, t)| self.effective_camera(m, t))
    }

    /// The sources whose camera the preview must show as one fixed view: dragged over the
    /// path, or without any POV.
    pub(super) fn live_overlay(&self) -> Vec<(MediaId, Camera)> {
        self.live
            .iter()
            .filter(|(m, l)| l.overriding || self.project.camera.path(**m).is_empty())
            .map(|(m, l)| (*m, l.cam))
            .collect()
    }

    /// Stops the manual camera from overriding the saved POVs (seek, play, Reset).
    pub(super) fn clear_overrides(&mut self) {
        for l in self.live.values_mut() {
            l.overriding = false;
        }
    }

    /// Dragging the picture of a 360 clip by `dx`/`dy` pixels on a preview `view_w` wide.
    pub(super) fn drag_camera(
        &mut self,
        dx: f32,
        dy: f32,
        view_w: f32,
        now: Instant,
    ) -> Result<(), String> {
        let (m, t) = self.sphere_at_playhead().ok_or(NOT_A_360)?;
        let cam = self.effective_camera(m, t).dragged(dx, dy, view_w);
        self.set_live(m, cam, now);
        Ok(())
    }

    /// Wheel or pinch zoom of a 360 clip.
    pub(super) fn zoom_camera(&mut self, scroll: f32, now: Instant) -> Result<(), String> {
        let (m, t) = self.sphere_at_playhead().ok_or(NOT_A_360)?;
        let cam = self.effective_camera(m, t).zoomed(scroll);
        self.set_live(m, cam, now);
        Ok(())
    }

    fn set_live(&mut self, media: MediaId, cam: Camera, now: Instant) {
        let has_path = !self.project.camera.path(media).is_empty();
        let l = LiveView {
            cam,
            overriding: has_path,
        };
        if self.live.get(&media) == Some(&l) {
            return;
        }
        self.live.insert(media, l);
        self.camera_changed(now);
    }

    /// The picture must follow a camera change: a new still, or (playing) a new stream
    /// after a short quiet time.
    fn camera_changed(&mut self, now: Instant) {
        if self.preview.is_playing() {
            self.camera_restart = Some(now + CAMERA_RESTART);
        } else {
            self.refresh_still(now);
        }
    }

    /// Restarts playback with the current camera once the drag has paused.
    pub(super) fn tick_camera(&mut self, now: Instant) {
        let Some(due) = self.camera_restart else {
            return;
        };
        if now < due {
            return;
        }
        self.camera_restart = None;
        if self.preview.is_playing() {
            let at = self.preview.position(now);
            self.playhead = at;
            let shown = self.shown_project().into_owned();
            if let Err(e) = self.preview.play(&shown, at, now) {
                self.status = e;
            }
        }
    }

    /// Set POV: the camera shown at the playhead becomes a point of view of the source.
    pub(super) fn set_pov(&mut self, now: Instant) -> Result<(), String> {
        let (media, t) = self.sphere_at_playhead().ok_or(NOT_A_360)?;
        let cam = self.effective_camera(media, t);
        let existing = self.project.camera.path(media).iter().find(|p| p.t == t);
        if existing.is_some_and(|p| p.cam == cam) {
            return Ok(());
        }
        self.live.insert(
            media,
            LiveView {
                cam,
                overriding: false,
            },
        );
        self.edit(now, "Set POV", |e| {
            e.camera.set(media, Pov { t, cam }).map_err(pov_error)
        })
    }

    /// Changes a POV (time and/or view) from its popup.
    pub(super) fn edit_pov(
        &mut self,
        media: MediaId,
        old_t: Micros,
        pov: Pov,
        now: Instant,
    ) -> Result<(), String> {
        let m = self.project.media(media).ok_or("no such media")?;
        if pov.t < Micros::ZERO || pov.t > m.duration {
            return Err(pov_error(PovError::BadTime));
        }
        let pov = Pov {
            t: pov.t,
            cam: pov.cam.clamped(),
        };
        if old_t == pov.t && self.project.camera.path(media).contains(&pov) {
            return Ok(());
        }
        self.edit(now, "Edit POV", |e| {
            e.camera.edit(media, old_t, pov).map_err(pov_error)
        })
    }

    pub(super) fn remove_pov(
        &mut self,
        media: MediaId,
        t: Micros,
        now: Instant,
    ) -> Result<(), String> {
        if !self.project.camera.path(media).iter().any(|p| p.t == t) {
            return Err(pov_error(PovError::BadTime));
        }
        self.edit(now, "Remove POV", |e| {
            e.camera.remove(media, t);
            Ok(())
        })
    }

    /// Drops the preview copy of a source and makes the one its kind needs now.
    pub(super) fn redo_proxy(&mut self, id: MediaId) {
        self.cancel_proxy(id);
        if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id) {
            m.proxy = ProxyState::None;
        }
        self.maybe_proxy(id);
    }

    /// View > Show boxes on the picture.
    pub(super) fn toggle_boxes(&mut self) -> Result<(), String> {
        self.settings.show_boxes = !self.settings.show_boxes;
        self.save_settings()
    }

    // ----- boxes: resize, align, reset -----

    /// A corner drag on the preview, saved on release: `ids` get `size`, the dragged
    /// `line` its new centre; one undo step.
    pub(super) fn resize_lines(
        &mut self,
        ids: Vec<LineId>,
        line: LineId,
        size: u16,
        pos: Pos,
        now: Instant,
    ) -> Result<(), String> {
        self.resize_drag = None;
        if ids.is_empty() || !ids.contains(&line) {
            return Err(SELECT_ON_PICTURE.into());
        }
        if !pos.is_valid() {
            return Err(format!("a position must be between 0 and {POS_MAX}"));
        }
        let size = (size != SIZE_NORMAL).then_some(size);
        let subs = &self.project.subtitles;
        let unchanged = ids.iter().all(|id| {
            subs.get(*id).is_some_and(|l| {
                l.size
                    == size.map(|s| s.clamp(crate::subtitles::SIZE_MIN, crate::subtitles::SIZE_MAX))
            })
        }) && subs.get(line).is_some_and(|l| l.position == Some(pos));
        if unchanged {
            self.refresh_still(now);
            return Ok(());
        }
        self.edit(now, "Resize subtitle", |e| {
            e.subtitles.set_boxes(&ids, None, Some(size)).map_err(err)?;
            e.subtitles
                .set_boxes(&[line], Some(Some(pos)), None)
                .map_err(err)
        })
    }

    /// Shows lines at their new size while a corner is dragged (nothing is saved).
    pub(super) fn drag_resize(&mut self, d: Option<ResizeDrag>, now: Instant) {
        self.resize_drag = d.map(|mut d| {
            d.pos = Pos {
                x: d.pos.x.min(POS_MAX),
                y: d.pos.y.min(POS_MAX),
            };
            d.size = d
                .size
                .clamp(crate::subtitles::SIZE_MIN, crate::subtitles::SIZE_MAX);
            d
        });
        self.refresh_still(now);
    }

    /// Align horizontally / vertically: the selected lines are centred on the picture on
    /// that axis, the other coordinate and the size stay. A line at the style's place is
    /// already centred horizontally; vertically it gets the middle of the picture.
    pub(super) fn align_selected(&mut self, axis: Axis, now: Instant) -> Result<(), String> {
        if self.selected_lines().is_empty() && self.selected_texts().is_empty() {
            return Err(SELECT_ON_PICTURE.into());
        }
        let (lines, texts) = self.align_plan(axis);
        if lines.is_empty() && texts.is_empty() {
            return Ok(());
        }
        let label = match axis {
            Axis::Horizontal => "Align horizontally",
            Axis::Vertical => "Align vertically",
        };
        self.edit(now, label, |e| {
            for (id, pos) in &lines {
                if let Some(l) = e.subtitles.lines.iter_mut().find(|l| l.id == *id) {
                    l.position = *pos;
                }
            }
            for (id, place) in &texts {
                if let Some(t) = e.texts.items.iter_mut().find(|t| t.id == *id) {
                    t.place = *place;
                }
            }
            Ok(())
        })
    }

    /// What Align does to the selection: the lines and texts that are not centred on the
    /// axis yet, with their new place (REQ-098, REQ-115, REQ-173). A line at the style's
    /// place gets its own place: the centre of the picture on that axis, the other
    /// coordinate where its box is now.
    pub(super) fn align_plan(&self, axis: Axis) -> AlignPlan {
        let frame = self.output_size();
        let subs = &self.project.subtitles;
        let preset = subs.preset;
        let mid = POS_MAX / 2;
        let lines = self
            .selected_lines()
            .into_iter()
            .filter_map(|id| {
                let l = subs.get(id)?;
                let new = match (axis, l.position, frame) {
                    (Axis::Horizontal, Some(p), _) => Some(align_h(p)),
                    (Axis::Vertical, Some(p), _) => Some(align_v(p)),
                    (Axis::Horizontal, None, Some(f)) => Some(Pos {
                        x: mid,
                        y: crate::placement::line_rect(preset, &l.text, None, l.size, f)
                            .pos(f)
                            .y,
                    }),
                    (Axis::Horizontal, None, None) => None,
                    (Axis::Vertical, None, _) => Some(Pos { x: mid, y: mid }),
                };
                (new != l.position).then_some((id, new))
            })
            .collect();
        let texts = self
            .selected_texts()
            .into_iter()
            .filter_map(|id| {
                let t = self.project.texts.get(id)?;
                let new = match axis {
                    Axis::Horizontal => align_h(t.place),
                    Axis::Vertical => align_v(t.place),
                };
                (new != t.place).then_some((id, new))
            })
            .collect();
        (lines, texts)
    }

    /// True when Reset would change something among the selected lines.
    pub fn selection_is_changed(&self) -> bool {
        self.project
            .subtitles
            .any_moved_or_sized(&self.selected_lines())
            || self.selected_texts_changed()
    }

    /// True when a selected text item is not at the default place and width (REQ-115).
    fn selected_texts_changed(&self) -> bool {
        self.selected_texts().iter().any(|id| {
            self.project.texts.get(*id).is_some_and(|t| {
                t.place != crate::text::DEFAULT_PLACE
                    || t.width != crate::text::DEFAULT_WIDTH
                    || t.height != crate::text::DEFAULT_HEIGHT
            })
        })
    }

    /// True when the 360 clip at the playhead is looked at differently from the default.
    pub fn camera_is_moved(&self) -> bool {
        let Some((m, _)) = self.sphere_at_playhead() else {
            return false;
        };
        self.live
            .get(&m)
            .is_some_and(|l| l.overriding || l.cam != Camera::FRONT)
    }

    /// Reset applies to the camera: no line is selected and the 360 clip at the playhead
    /// is looked at differently from the default.
    pub fn reset_applies_to_camera(&self) -> bool {
        self.selected_lines().is_empty()
            && self.selected_texts().is_empty()
            && self.camera_is_moved()
    }

    /// Reset: selected lines back to the style's place and 100%; with no line selected,
    /// the live camera of the 360 clip at the playhead back to front, level, 90°.
    pub(super) fn reset_objects(&mut self, now: Instant) -> Result<(), String> {
        let ids = self.selected_lines();
        let tids = self.selected_texts();
        if !ids.is_empty() || !tids.is_empty() {
            return self.reset_selection(ids, tids, now);
        }
        let Some((m, _)) = self.sphere_at_playhead() else {
            return Err(SELECT_ON_PICTURE.into());
        };
        if !self.camera_is_moved() {
            return Err(CAMERA_AT_FRONT.into());
        }
        self.live.insert(
            m,
            LiveView {
                cam: Camera::FRONT,
                overriding: false,
            },
        );
        self.camera_changed(now);
        Ok(())
    }

    /// Lines back to the style's place and size and texts to the default place and width,
    /// in one undo step (nothing to do when they already are).
    fn reset_selection(
        &mut self,
        ids: Vec<LineId>,
        tids: Vec<crate::text::TextId>,
        now: Instant,
    ) -> Result<(), String> {
        self.drag_pos = None;
        self.resize_drag = None;
        let lines = self.project.subtitles.any_moved_or_sized(&ids);
        if !lines && !self.selected_texts_changed() {
            self.refresh_still(now);
            return Ok(());
        }
        self.edit(now, "Reset", |e| {
            if lines {
                e.subtitles.reset(&ids).map_err(err)?;
            }
            for t in e.texts.items.iter_mut().filter(|t| tids.contains(&t.id)) {
                t.place = crate::text::DEFAULT_PLACE;
                t.width = crate::text::DEFAULT_WIDTH;
                t.height = crate::text::DEFAULT_HEIGHT;
            }
            Ok(())
        })
    }

    /// Lines back to the style's place and size (one undo step; nothing to do when they
    /// already are).
    pub(super) fn reset_lines(&mut self, ids: Vec<LineId>, now: Instant) -> Result<(), String> {
        self.drag_pos = None;
        self.resize_drag = None;
        if ids.is_empty() {
            return Err(SELECT_ON_PICTURE.into());
        }
        if !self.project.subtitles.any_moved_or_sized(&ids) {
            self.refresh_still(now);
            return Ok(());
        }
        self.edit(now, "Reset", |e| e.subtitles.reset(&ids).map_err(err))
    }
}
