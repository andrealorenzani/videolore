// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Dispatch of the commands on the objects of the picture (texts, pictures, subtitle line
//! boxes, the selection, the 360 camera and its points of view, boxes, snapping, safe
//! areas): the arms of [`AppState::dispatch`] for this area. A command of another area is
//! handed back unchanged ([`Routed::Pass`]).

use super::*;

impl AppState {
    pub(super) fn dispatch_objects(
        &mut self,
        cmd: Command,
        now: Instant,
        _label: &'static str,
    ) -> Routed {
        let mut pass = None;
        let run = || -> Result<(), String> {
            match cmd {
                Command::ToggleLine(id) => self.toggle_line(id),
                Command::AddText(draft) => self.add_text(draft, now),
                Command::AddPicture(path) => self.add_picture(path, now),
                Command::EditText(item) => self.edit_text(item, now),
                Command::MoveText { id, start, ttl } => self.move_text(id, start, ttl, now),
                Command::DeleteTexts(ids) => self.delete_texts(ids, now),
                Command::SetTextBoxes { ids, place } => self.set_text_boxes(ids, place, now),
                Command::ResizeTextWidth { ids, width } => {
                    self.resize_text_box(ids, TextEdge::Right, width, now)
                }
                Command::ResizeTextBox { ids, edge, value } => {
                    self.resize_text_box(ids, edge, value, now)
                }
                Command::SetReadability { ids, readability } => {
                    self.set_readability(ids, readability, now)
                }
                Command::ResizeTextAnchorWidth { ids, width_cd } => {
                    self.resize_text_anchor_width(ids, width_cd, now)
                }
                Command::MoveTextAnchor {
                    ids,
                    yaw_cd,
                    pitch_cd,
                } => self.move_text_anchor(ids, yaw_cd, pitch_cd, now),
                Command::MoveLineAnchor {
                    ids,
                    yaw_cd,
                    pitch_cd,
                } => self.move_line_anchor(ids, yaw_cd, pitch_cd, now),
                Command::OpenText(id) => self.open_text(id, now),
                Command::OpenNewText => {
                    self.popups.text = None;
                    self.popups.show(Popup::Text);
                    Ok(())
                }
                Command::ToggleText(id) => self.toggle_text(id),
                Command::SelectText(id) => {
                    if self.project.texts.get(id).is_none() {
                        return Err("no such text".into());
                    }
                    self.selection.select_only(ObjRef::Text(id));
                    Ok(())
                }
                Command::SelectRange { from, to, add } => self.select_range(from, to, add, now),
                Command::ClearSelection => {
                    self.selection.clear();
                    Ok(())
                }
                Command::SetLinePositions { ids, pos } => self.set_line_positions(ids, pos, now),
                Command::ResetPosition => {
                    let ids = self.selected_lines();
                    self.reset_lines(ids, now)
                }
                Command::ResetObjects => self.reset_objects(now),
                Command::MoveSelection {
                    lines,
                    texts,
                    place,
                } => self.move_selection(lines, texts, place, now),
                Command::MoveInside => self.move_inside(now),
                Command::ReAnchor => self.re_anchor(now),
                Command::AlignSelected(axis) => self.align_selected(axis, now),
                Command::ResizeLines {
                    ids,
                    line,
                    size,
                    pos,
                } => self.resize_lines(ids, line, size, pos, now),
                Command::DragResize(d) => {
                    self.drag_resize(d, now);
                    Ok(())
                }
                Command::DragLines(d) => {
                    self.drag_lines(d, now);
                    Ok(())
                }
                Command::DragCamera { dx, dy, view_w } => self.drag_camera(dx, dy, view_w, now),
                Command::RollCamera { dx, view_w } => self.roll_camera(dx, view_w, now),
                Command::TurnCamera { yaw_cd, pitch_cd } => self.turn_camera(yaw_cd, pitch_cd, now),
                Command::ZoomCamera(scroll) => self.zoom_camera(scroll, now),
                Command::SetPov => self.set_pov(now),
                Command::EditPov { media, old_t, pov } => self.edit_pov(media, old_t, pov, now),
                Command::RemovePov { media, t } => self.remove_pov(media, t, now),
                Command::ToggleBoxes => self.toggle_boxes(),
                Command::ToggleSnap => self.toggle_snap(),
                Command::ToggleSafeAreas => self.toggle_safe_areas(),
                other => {
                    pass = Some(other);
                    Ok(())
                }
            }
        };
        let r = run();
        match pass {
            Some(c) => Routed::Pass(c),
            None => Routed::Done(r),
        }
    }
}
