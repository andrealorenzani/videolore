// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of 2.0.4, batch 7, as amended in 2.2.0: the popup of a whole-video copy
//! (REQ-200), no offer in the status bar (REQ-201), one set of picture handles (REQ-202),
//! the keyframe marker, its typed time and the arrow keys (REQ-203) and Copy and Paste
//! keyframes (REQ-204).

use super::frame_popups::{KEY_MARKER, KEY_TOLERANCE, KeyBar};
use super::ux_tests::Harness;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{EffectKind, FrameDrag, FrameFx, FrameValue, Target};
use crate::frame::Scale;
use crate::timeline::FrameTarget;
use eframe::egui::{Key, Modifiers, Rect, pos2};

// the marker is 12 pixels across and a click within 8 pixels picks it (REQ-203)
const _: () = assert!(KEY_MARKER >= 12.0 && KEY_TOLERANCE >= 8.0);

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Two 5 s clips, the playhead at 1 s, a frame on screen.
fn loaded() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    (dir, app)
}

fn picture(h: &Harness) -> Rect {
    h.find("Drag a subtitle to move it")
        .unwrap_or_else(|| panic!("{:#?}", h.labels()))
        .rect
}

fn at(pic: Rect, fx: f32, fy: f32) -> eframe::egui::Pos2 {
    pos2(pic.min.x + fx * pic.width(), pic.min.y + fy * pic.height())
}

fn permille(app: &AppState, i: usize) -> u16 {
    let id = app.project.timeline.clips[i].id;
    app.project.timeline.scale_keys(FrameTarget::Clip(id))[0]
        .v
        .permille
}

// covers: REQ-200, REQ-148
#[test]
fn a_whole_video_scale_opens_the_popup_of_one_clip_without_a_list_of_copies() {
    let (_d, mut app) = loaded();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::WholeVideo,
        },
        Instant::now(),
    )
    .unwrap();
    let mut h = Harness::new(app, 1280.0, 900.0);
    h.idle(2);
    // the popup is the one of the clip under the playhead (clip 1): no row per clip
    assert!(!h.has("1. a.mp4 (1 keyframe)"), "{:#?}", h.labels());
    assert!(!h.has("2. b.mp4 (1 keyframe)"), "{:#?}", h.labels());
    assert!(!h.has("One copy per clip; select the one to edit:"));
    assert!(h.has("Remove effect"), "{:#?}", h.labels());
    let c2 = h.app.project.timeline.clips[1].id;
    h.app
        .dispatch(
            Command::SetPicture(FrameDrag {
                target: FrameTarget::Clip(c2),
                value: FrameValue::Scale(Scale {
                    permille: 1400,
                    ..Scale::NORMAL
                }),
            }),
            Instant::now(),
        )
        .unwrap();
    assert_eq!((permille(&h.app, 0), permille(&h.app, 1)), (1000, 1400));
    // Remove effect in the open popup removes the copy of its own clip (clip 1) only
    h.idle(2);
    h.click_label("Remove effect");
    let left: Vec<_> = h
        .app
        .project
        .timeline
        .clip_frames
        .iter()
        .map(|f| f.clip)
        .collect();
    assert_eq!(left, vec![c2]);
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!((permille(&h.app, 0), permille(&h.app, 1)), (1000, 1400));
}

// covers: REQ-201
#[test]
fn the_status_bar_has_no_offer_and_no_button_for_a_new_clip() {
    let (_d, mut app) = loaded();
    for i in 0..2 {
        let c = app.project.timeline.clips[i].id;
        app.dispatch(
            Command::SetClipVolume {
                clip: c,
                volume: 50,
                muted: false,
            },
            Instant::now(),
        )
        .unwrap();
    }
    let m = app.project.media[0].id;
    app.dispatch(Command::Append(Some(m)), Instant::now())
        .unwrap();
    let mut h = Harness::new(app, 1280.0, 900.0);
    h.idle(2);
    assert!(!h.has("The new clip has no Volume"), "{:#?}", h.labels());
    assert!(!h.has("Apply to new clips"), "{:#?}", h.labels());
    assert!(!h.app.project.timeline.clips[2].has_volume());
    assert_eq!(h.app.project.timeline.clips[0].volume, 50);
}

// covers: REQ-202
#[test]
fn one_set_of_handles_a_drag_changes_only_the_clip_under_the_playhead() {
    let (_d, mut app) = loaded();
    let now = Instant::now();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    let (c1, c2) = (
        app.project.timeline.clips[0].id,
        app.project.timeline.clips[1].id,
    );
    // the copy of clip 1 is half size, the playhead is in clip 2: one box, clip 2's
    app.dispatch(Command::SelectKey(Some(Micros::ZERO)), now)
        .unwrap();
    app.dispatch(
        Command::SetPicture(FrameDrag {
            target: FrameTarget::Clip(c1),
            value: FrameValue::Scale(Scale {
                permille: 500,
                ..Scale::NORMAL
            }),
        }),
        now,
    )
    .unwrap();
    app.dispatch(Command::Seek(s(6)), now).unwrap();
    assert_eq!(
        app.frame_target_at_playhead(FrameFx::Scale),
        Some(FrameTarget::Clip(c2))
    );
    // the popups open at the left, so that they do not cover the picture being dragged
    app.settings
        .interface
        .popups
        .insert("Instance".into(), (10.0, 60.0));
    let mut h = Harness::new(app, 1280.0, 900.0);
    let pic = picture(&h);
    let value = |h: &Harness, c| {
        h.app.project.timeline.scale_keys(FrameTarget::Clip(c))[0]
            .v
            .permille
    };
    assert_eq!((value(&h, c1), value(&h, c2)), (500, 1000));
    // the clip's own corner: clip 2 only; the other copy is not drawn and cannot be grabbed
    h.drag(at(pic, 1.0, 1.0), at(pic, 0.9, 0.9));
    assert_eq!(value(&h, c1), 500, "the other copy is unchanged");
    assert!(value(&h, c2) < 1000, "{}", value(&h, c2));
    h.drag(at(pic, 0.75, 0.75), at(pic, 0.6, 0.6));
    assert_eq!(value(&h, c1), 500, "no dashed handle of the other copy");
}

// covers: REQ-203
#[test]
fn a_keyframe_marker_is_twelve_pixels_picked_within_eight_and_moved_by_the_arrow_keys() {
    // a click 6 pixels from a keyframe selects it; 9 pixels away does not
    let bar = KeyBar::new(Micros::ZERO, s(10), 400.0);
    let keys = [s(5)];
    let x = bar.x_of(s(5)).unwrap();
    assert_eq!(bar.key_at(x + 6.0, &keys), Some(s(5)));
    assert_eq!(bar.key_at(x - 8.0, &keys), Some(s(5)));
    assert_eq!(bar.key_at(x + 9.0, &keys), None);
    // in the window: select the key, type a time, then Right moves it by one frame
    let (_d, mut app) = loaded();
    let c1 = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c1),
        },
        Instant::now(),
    )
    .unwrap();
    app.dispatch(
        Command::AddKey {
            target: FrameTarget::Clip(c1),
            effect: FrameFx::Scale,
        },
        Instant::now(),
    )
    .unwrap();
    let key = app.popups.key.unwrap();
    assert_eq!(key, s(1));
    let mut h = Harness::new(app, 1280.0, 900.0);
    h.idle(3);
    assert!(
        h.has("Keyframe time"),
        "typed time and value: {:#?}",
        h.labels()
    );
    let frame = h.app.frame_len();
    // the bar is the only wide, short widget with a click: click on the keyframe in it
    let r = h
        .nodes
        .iter()
        .filter(|n| {
            n.interactive
                && n.rect.height() < 40.0
                && n.rect.width() > 200.0
                && n.rect.height() > 20.0
        })
        .map(|n| n.rect)
        .next()
        .expect("the keyframe bar");
    // the clip is 5 s long: the key at 1 s is a fifth of the way across
    h.click(pos2(r.left() + r.width() / 5.0, r.center().y));
    assert_eq!(
        h.app.popups.key,
        Some(key),
        "the click selected the keyframe"
    );
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(
        h.app.popups.key,
        Some(key + frame),
        "one frame to the right"
    );
    assert_eq!(h.app.playhead, s(1), "the playhead did not take the key");
    h.key(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.app.popups.key, Some(key), "and back");
    assert_eq!(h.app.playhead, s(1));
}

// covers: REQ-204
#[test]
fn copy_keyframes_and_paste_keyframes_to_are_in_the_popup() {
    let (_d, mut app) = loaded();
    let (c1, c2) = (
        app.project.timeline.clips[0].id,
        app.project.timeline.clips[1].id,
    );
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c1),
        },
        Instant::now(),
    )
    .unwrap();
    let mut h = Harness::new(app, 1280.0, 900.0);
    h.idle(3);
    assert!(h.has("Copy keyframes"), "{:#?}", h.labels());
    assert!(h.has("Paste keyframes to"));
    // pasting is not possible before copying
    assert!(h.find("Paste keyframes to").unwrap().disabled);
    h.click_label("Copy keyframes");
    h.idle(2);
    assert!(h.app.clipboard_keys.is_some());
    h.click_label("Paste keyframes to");
    h.idle(2);
    assert!(
        h.has("2. b.mp4"),
        "the other clips are listed: {:#?}",
        h.labels()
    );
    h.click_label("2. b.mp4");
    h.idle(2);
    assert_eq!(
        h.app
            .project
            .timeline
            .scale_keys(FrameTarget::Clip(c2))
            .len(),
        1
    );
    assert_eq!(h.app.history.undo_label(), Some("Paste keyframes"));
}

// covers: REQ-203
#[test]
fn a_click_on_an_empty_place_of_the_keyframe_bar_moves_the_playhead_and_deselects() {
    let (_d, mut app) = loaded();
    let c1 = app.project.timeline.clips[0].id;
    let now = Instant::now();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c1),
        },
        now,
    )
    .unwrap();
    app.dispatch(
        Command::AddKey {
            target: FrameTarget::Clip(c1),
            effect: FrameFx::Scale,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.popups.key, Some(s(1)));
    let mut h = Harness::new(app, 1280.0, 900.0);
    h.idle(3);
    let r = h
        .nodes
        .iter()
        .filter(|n| n.interactive && n.rect.height() < 40.0 && n.rect.width() > 200.0)
        .filter(|n| n.rect.height() > 20.0)
        .map(|n| n.rect)
        .next()
        .expect("the keyframe bar");
    // the clip is 5 s long: a click four fifths across is 4 s of the clip, far from the key
    h.click(pos2(r.left() + r.width() * 0.8, r.center().y));
    assert_eq!(h.app.popups.key, None, "the key is not selected any more");
    assert_eq!(h.app.playhead, s(4));
}
