// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The selection of movable objects (subtitle lines and text items): one value shared by
//! the timeline, the preview and the lists (REQ-090 to REQ-092). Every transition is a
//! pure method, so the rules are tested without a window.

use crate::subtitles::LineId;
use crate::text::TextId;
use std::collections::BTreeSet;

/// One selectable object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjRef {
    Line(LineId),
    Text(TextId),
}

/// The selected objects and the one that was picked last (the one popups follow).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    lines: BTreeSet<LineId>,
    texts: BTreeSet<TextId>,
    primary: Option<ObjRef>,
}

impl FromIterator<LineId> for Selection {
    fn from_iter<I: IntoIterator<Item = LineId>>(iter: I) -> Self {
        let mut s = Selection::default();
        for id in iter {
            s.add(ObjRef::Line(id));
        }
        s
    }
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.texts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.lines.len() + self.texts.len()
    }

    pub fn contains(&self, id: &LineId) -> bool {
        self.lines.contains(id)
    }

    pub fn contains_text(&self, id: TextId) -> bool {
        self.texts.contains(&id)
    }

    pub fn has(&self, o: ObjRef) -> bool {
        match o {
            ObjRef::Line(l) => self.lines.contains(&l),
            ObjRef::Text(t) => self.texts.contains(&t),
        }
    }

    /// The selected subtitle lines.
    pub fn iter(&self) -> impl Iterator<Item = &LineId> {
        self.lines.iter()
    }

    pub fn lines(&self) -> Vec<LineId> {
        self.lines.iter().copied().collect()
    }

    pub fn texts(&self) -> Vec<TextId> {
        self.texts.iter().copied().collect()
    }

    pub fn has_lines(&self) -> bool {
        !self.lines.is_empty()
    }

    /// The object picked last.
    pub fn primary(&self) -> Option<ObjRef> {
        self.primary.filter(|o| self.has(*o))
    }

    /// The line picked last, if the last pick was a line.
    pub fn primary_line(&self) -> Option<LineId> {
        match self.primary() {
            Some(ObjRef::Line(l)) => Some(l),
            _ => None,
        }
    }

    pub fn primary_text(&self) -> Option<TextId> {
        match self.primary() {
            Some(ObjRef::Text(t)) => Some(t),
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        *self = Selection::default();
    }

    fn add(&mut self, o: ObjRef) {
        match o {
            ObjRef::Line(l) => self.lines.insert(l),
            ObjRef::Text(t) => self.texts.insert(t),
        };
        self.primary = Some(o);
    }

    fn remove(&mut self, o: ObjRef) {
        match o {
            ObjRef::Line(l) => self.lines.remove(&l),
            ObjRef::Text(t) => self.texts.remove(&t),
        };
        if self.primary == Some(o) {
            self.primary = self.lines.iter().next().map(|l| ObjRef::Line(*l)).or(self
                .texts
                .iter()
                .next()
                .map(|t| ObjRef::Text(*t)));
        }
    }

    /// Selects only `o`.
    pub fn select_only(&mut self, o: ObjRef) {
        self.clear();
        self.add(o);
    }

    /// A click without movement: with Ctrl, Cmd or Shift the object joins the selection or
    /// leaves it; a plain click selects only it.
    pub fn click(&mut self, o: ObjRef, toggle: bool) {
        if !toggle {
            self.select_only(o);
        } else if self.has(o) {
            self.remove(o);
        } else {
            self.add(o);
        }
    }

    /// Picks `objs` (the objects whose start is inside a time range); `add` keeps the
    /// current selection.
    pub fn pick(&mut self, objs: &[ObjRef], add: bool) {
        if !add {
            self.clear();
        }
        for o in objs {
            self.add(*o);
        }
    }

    /// The objects a drag of `o` moves: the whole selection when `o` is in a selection of
    /// several, else `o` alone.
    pub fn drag_targets(&self, o: ObjRef) -> Vec<ObjRef> {
        if self.has(o) && self.len() > 1 {
            self.lines
                .iter()
                .map(|l| ObjRef::Line(*l))
                .chain(self.texts.iter().map(|t| ObjRef::Text(*t)))
                .collect()
        } else {
            vec![o]
        }
    }

    /// Esc with no popup open clears the selection; returns whether it was used.
    pub fn esc(&mut self, popup_open: bool) -> bool {
        if popup_open || self.is_empty() {
            return false;
        }
        self.clear();
        true
    }

    /// Forgets the objects that no longer exist.
    pub fn prune(
        &mut self,
        line_exists: impl Fn(LineId) -> bool,
        text_exists: impl Fn(TextId) -> bool,
    ) {
        self.lines.retain(|l| line_exists(*l));
        self.texts.retain(|t| text_exists(*t));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(n: u64) -> ObjRef {
        ObjRef::Line(LineId(n))
    }
    fn t(n: u64) -> ObjRef {
        ObjRef::Text(TextId(n))
    }

    // covers: REQ-090
    #[test]
    fn toggle_clicks_add_and_remove_and_a_plain_click_leaves_one() {
        let mut s = Selection::default();
        for n in 1..=3 {
            s.click(l(n), true);
        }
        assert_eq!(s.len(), 3);
        s.click(l(2), true);
        assert_eq!(s.lines(), [LineId(1), LineId(3)]);
        s.click(t(1), true);
        assert_eq!(s.len(), 3);
        assert_eq!(s.primary_text(), Some(TextId(1)));
        s.click(l(3), false);
        assert_eq!(s.len(), 1);
        assert_eq!(s.primary_line(), Some(LineId(3)));
    }

    // covers: REQ-090
    #[test]
    fn removing_the_primary_object_falls_back_to_another() {
        let mut s = Selection::default();
        s.click(l(1), true);
        s.click(t(5), true);
        s.click(t(5), true);
        assert_eq!(s.primary(), Some(l(1)));
        s.click(l(1), true);
        assert!(s.is_empty());
        assert_eq!(s.primary(), None);
        s.click(t(2), true);
        s.click(t(3), true);
        s.click(t(3), true);
        assert_eq!(s.primary_text(), Some(TextId(2)));
        assert!(s.primary_line().is_none());
    }

    // covers: REQ-091
    #[test]
    fn a_range_pick_replaces_or_adds() {
        let mut s: Selection = [LineId(1)].into_iter().collect();
        s.pick(&[l(5), t(9)], true);
        assert_eq!(s.len(), 3);
        s.pick(&[l(5)], false);
        assert_eq!(s.lines(), [LineId(5)]);
        assert!(s.texts().is_empty());
        assert!(s.has_lines());
        assert!(s.contains(&LineId(5)));
        assert!(!s.contains_text(TextId(9)));
    }

    // covers: REQ-092
    #[test]
    fn dragging_a_selected_object_moves_the_group_else_only_itself() {
        let mut s = Selection::default();
        s.pick(&[l(1), l(2), t(3)], false);
        assert_eq!(s.drag_targets(l(2)), [l(1), l(2), t(3)]);
        assert_eq!(s.drag_targets(l(7)), [l(7)]);
        s.click(l(1), false);
        assert_eq!(s.drag_targets(l(1)), [l(1)]);
        // the selection itself is not touched by asking
        assert_eq!(s.len(), 1);
    }

    // covers: REQ-090
    #[test]
    fn esc_clears_only_without_a_popup_and_prune_forgets_gone_objects() {
        let mut s = Selection::default();
        s.pick(&[l(1), t(1)], false);
        assert!(!s.esc(true));
        assert_eq!(s.len(), 2);
        s.prune(|l| l.0 != 1, |_| true);
        assert_eq!(s.len(), 1);
        s.prune(|_| true, |_| false);
        assert!(s.is_empty());
        assert!(!s.esc(false));
        s.select_only(l(4));
        assert!(s.esc(false));
        assert!(s.is_empty());
    }
}
