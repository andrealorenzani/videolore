// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! 2.1.0: the subtitle style settings and the text shadow as commands (one undo step each).

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::subtitles::{Preset, StyleSettings};

// covers: REQ-307
// covers: REQ-308
// covers: REQ-309
#[test]
fn the_style_settings_are_one_undo_step_and_values_are_brought_inside_their_range() {
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    let st = StyleSettings {
        highlighted: [255, 0, 0],
        not_yet: [0, 0, 255],
        shadow: crate::base::shadow::Shadow {
            px: 90,
            colour: [1, 2, 3],
        },
        enlarge: 0,
    };
    app.dispatch(Command::SetPreset(Preset::EnlargeWord), now)
        .unwrap();
    app.dispatch(Command::SetSubtitleStyle(st), now).unwrap();
    let got = app.project.subtitles.style;
    assert_eq!(got.shadow.px, crate::base::shadow::SHADOW_MAX);
    assert_eq!(
        got.enlarge,
        crate::subtitles::ENLARGE_MIN,
        "N=0 is refused: 1 is the least"
    );
    assert_eq!((got.highlighted, got.not_yet), ([255, 0, 0], [0, 0, 255]));
    // the same values again change nothing and make no undo step
    app.dispatch(Command::SetSubtitleStyle(got), now).unwrap();
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.subtitles.style.is_default());
    assert_eq!(app.project.subtitles.preset, Preset::EnlargeWord);
    app.dispatch(Command::Redo, now).unwrap();
    assert_eq!(app.project.subtitles.style, got);
}

// covers: REQ-306
#[test]
fn a_text_edited_with_a_shadow_keeps_it_and_undo_restores_the_old_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    let m = app.project.media[0].id;
    app.dispatch(Command::Append(Some(m)), now).unwrap();
    let item = crate::text::TextItem::new(crate::text::TextId(0), "Hi".into(), Micros::ZERO);
    app.dispatch(Command::AddText(item), now).unwrap();
    let mut t = app.project.texts.items[0].clone();
    assert!(t.shadow.is_none());
    t.shadow = crate::base::shadow::Shadow {
        px: 8,
        colour: [255, 0, 0],
    };
    app.dispatch(Command::EditText(t), now).unwrap();
    assert_eq!(app.project.texts.items[0].shadow.px, 8);
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.texts.items[0].shadow.is_none());
}
