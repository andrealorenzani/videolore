// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 1.5.0: Change aspect, Scale, Move and their keyframes (REQ-124..131).

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::frame::{Fill, Fit, Shape};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

fn now() -> Instant {
    Instant::now()
}

/// Two 5 s clips, a.mp4 then b.mp4.
fn two() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    for i in [0, 1] {
        let m = app.project.media[i].id;
        app.dispatch(Command::Append(Some(m)), now()).unwrap();
    }
    (dir, app)
}

fn apply(app: &mut AppState, effect: EffectKind, target: Target) -> Result<(), String> {
    app.dispatch(Command::ApplyEffect { effect, target }, now())
}

fn clip(app: &AppState, i: usize) -> ClipId {
    app.project.timeline.clips[i].id
}

// covers: REQ-124
#[test]
fn change_aspect_starts_16_9_fit_width_black_and_every_change_is_one_undo_step() {
    let (_d, mut app) = two();
    apply(&mut app, EffectKind::ChangeAspect, Target::WholeVideo).unwrap();
    let a = app.project.timeline.frame_aspect.unwrap();
    assert_eq!(
        (a.shape, a.fit, a.fill),
        (
            Shape::Wide16x9,
            Fit::Width,
            Fill::Colour(crate::frame::Rgb::BLACK)
        )
    );
    assert_eq!(app.popups.instance, Some(Instance::Aspect));
    assert!(app.popups.is_open(Popup::Instance));
    assert_eq!(app.undo_text(), "Undo Change aspect");
    // a change
    let b = FrameAspect {
        shape: Shape::Tall9x16,
        fit: Fit::Height,
        fill: Fill::Blur,
        blur: Default::default(),
    };
    app.dispatch(Command::SetAspect(b), now()).unwrap();
    assert_eq!(app.project.timeline.frame_aspect, Some(b));
    // the same value again is not a step
    let n = app.history.undo_len();
    app.dispatch(Command::SetAspect(b), now()).unwrap();
    assert_eq!(app.history.undo_len(), n);
    // one undo restores the previous choice
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.project.timeline.frame_aspect, Some(a));
    // the preview's frame has the new shape; Remove effect gives the source frame back
    app.dispatch(Command::SetAspect(b), now()).unwrap();
    let (w, h) = app.output_size().unwrap();
    assert!(h > w);
    app.dispatch(Command::RemoveAspect, now()).unwrap();
    assert_eq!(app.project.timeline.frame_aspect, None);
    let (w, h) = app.output_size().unwrap();
    assert!(w > h);
    assert!(!app.popups.is_open(Popup::Instance));
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.project.timeline.frame_aspect, Some(b));
}

// covers: REQ-130, REQ-286, REQ-288
#[test]
fn change_aspect_on_a_clip_gives_that_clip_its_own_aspect_and_not_the_whole_video() {
    let (_d, mut app) = two();
    let c = clip(&app, 0);
    let cmd = Command::ApplyEffect {
        effect: EffectKind::ChangeAspect,
        target: Target::Clip(c),
    };
    assert!(app.is_enabled(&cmd));
    let n = app.history.undo_len();
    app.dispatch(cmd, now()).unwrap();
    assert_eq!(app.history.undo_len(), n + 1, "one undo step");
    assert!(app.project.timeline.own_aspect(c).is_some());
    assert!(app.project.timeline.own_aspect(clip(&app, 1)).is_none());
    assert!(
        app.project.timeline.frame_aspect.is_none(),
        "not the whole video"
    );
    app.dispatch(Command::Undo, now()).unwrap();
    assert!(app.project.timeline.own_aspect(c).is_none());
}

// covers: REQ-130
// covers: REQ-076
#[test]
fn scale_goes_on_one_clip_and_a_second_scale_opens_the_first_popup() {
    let (_d, mut app) = two();
    let (c0, c1) = (clip(&app, 0), clip(&app, 1));
    apply(&mut app, EffectKind::Scale, Target::Clip(c1)).unwrap();
    let tl = &app.project.timeline;
    assert!(tl.scale_keys(FrameTarget::Clip(c0)).is_empty());
    assert_eq!(tl.scale_keys(FrameTarget::Clip(c1)).len(), 1);
    assert_eq!(
        app.popups.instance,
        Some(Instance::Scale(FrameTarget::Clip(c1)))
    );
    assert_eq!(app.undo_text(), "Undo Scale");
    // a second one: no new step, the same popup
    let n = app.history.undo_len();
    app.popups.close(Popup::Instance);
    apply(&mut app, EffectKind::Scale, Target::Clip(c1)).unwrap();
    assert_eq!(app.history.undo_len(), n);
    assert_eq!(
        app.popups.instance,
        Some(Instance::Scale(FrameTarget::Clip(c1)))
    );
    // Move is independent; a whole-video Move is a copy on each clip that lacks one and
    // opens the popup of the clip under the playhead (clip 0)
    apply(&mut app, EffectKind::Move, Target::Clip(c1)).unwrap();
    apply(&mut app, EffectKind::Move, Target::WholeVideo).unwrap();
    assert_eq!(
        app.project.timeline.move_keys(FrameTarget::Clip(c0)).len(),
        1
    );
    assert_eq!(
        app.popups.instance,
        Some(Instance::Move(FrameTarget::Clip(c0)))
    );
    // the popup titles name the clip, never "whole video" (REQ-076)
    let t = app.popup_title(Popup::Instance);
    assert!(
        t.starts_with("Move — ") && !t.contains("whole video"),
        "{t}"
    );
    // a click on the Scale row opens the popup of the selected clip's own Scale
    app.dispatch(Command::SelectClip(Some(c1)), now()).unwrap();
    app.dispatch(Command::OpenFrameEffect(EffectKind::Scale), now())
        .unwrap();
    assert_eq!(
        app.popups.instance,
        Some(Instance::Scale(FrameTarget::Clip(c1)))
    );
    // a clip with no Scale: the click says so and opens nothing new
    app.dispatch(Command::SelectClip(Some(c0)), now()).unwrap();
    app.dispatch(Command::OpenFrameEffect(EffectKind::Scale), now())
        .unwrap();
    assert!(app.status.contains("has no Scale"), "{}", app.status);
    // Remove effect: one step, the popup closes
    app.dispatch(
        Command::RemoveFrameEffect {
            target: FrameTarget::Clip(c1),
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert!(
        app.project
            .timeline
            .scale_keys(FrameTarget::Clip(c1))
            .is_empty()
    );
    assert!(!app.popups.is_open(Popup::Instance));
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(
        app.project.timeline.scale_keys(FrameTarget::Clip(c1)).len(),
        1
    );
}

// covers: REQ-129
#[test]
fn keyframes_are_added_at_the_playhead_with_the_shown_value_and_removed_when_selected() {
    let (_d, mut app) = two();
    let c = FrameTarget::Clip(clip(&app, 0));
    let c0 = clip(&app, 0);
    apply(&mut app, EffectKind::Scale, Target::Clip(c0)).unwrap();
    // 100% at 0 s, 200% at 4 s
    app.dispatch(Command::Seek(s(4)), now()).unwrap();
    app.dispatch(
        Command::AddKey {
            target: c,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.popups.key, Some(s(4)));
    app.dispatch(
        Command::SetPicture(FrameDrag {
            target: c,
            value: FrameValue::Scale(Scale {
                permille: 2000,
                dx: 0,
                dy: 0,
            }),
        }),
        now(),
    )
    .unwrap();
    // changing the selected key does not make another one
    assert_eq!(app.project.timeline.scale_keys(c).len(), 2);
    app.dispatch(Command::Seek(s(2)), now()).unwrap();
    assert_eq!(app.scale_at(c).unwrap().permille, 1500);
    // Add at 2 s changes nothing at that moment
    app.dispatch(
        Command::AddKey {
            target: c,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.scale_at(c).unwrap().permille, 1500);
    assert_eq!(app.project.timeline.scale_keys(c).len(), 3);
    // the same time twice is refused
    assert!(
        app.dispatch(
            Command::AddKey {
                target: c,
                effect: FrameFx::Scale
            },
            now()
        )
        .is_err()
    );
    // Remove keyframe needs a selection
    app.dispatch(Command::SelectKey(None), now()).unwrap();
    let cmd = Command::RemoveKey {
        target: c,
        effect: FrameFx::Scale,
    };
    assert!(!app.is_enabled(&cmd));
    assert_eq!(app.disabled_reason(&cmd), Some("Select a keyframe first"));
    app.dispatch(Command::SelectKey(Some(s(2))), now()).unwrap();
    app.dispatch(cmd, now()).unwrap();
    assert_eq!(app.project.timeline.scale_keys(c).len(), 2);
    // removing the first of two makes the second one's value apply from the start
    app.dispatch(Command::SelectKey(Some(Micros::ZERO)), now())
        .unwrap();
    app.dispatch(
        Command::RemoveKey {
            target: c,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    app.dispatch(Command::Seek(Micros::ZERO), now()).unwrap();
    assert_eq!(app.scale_at(c).unwrap().permille, 2000);
    // the last keyframe cannot be removed
    app.dispatch(Command::SelectKey(Some(s(4))), now()).unwrap();
    assert!(
        app.dispatch(
            Command::RemoveKey {
                target: c,
                effect: FrameFx::Scale
            },
            now()
        )
        .is_err()
    );
    assert_eq!(app.history.undo_label(), Some("Remove keyframe"));
}

// covers: REQ-127
// covers: REQ-128
#[test]
fn a_change_on_the_picture_without_a_selected_key_makes_one_at_the_playhead() {
    let (_d, mut app) = two();
    apply(&mut app, EffectKind::Move, Target::WholeVideo).unwrap();
    app.dispatch(Command::SelectKey(None), now()).unwrap();
    app.dispatch(Command::Seek(s(3)), now()).unwrap();
    // the whole-video Move is a copy on each clip: the playhead is in the first one
    let w = FrameTarget::Clip(clip(&app, 0));
    // a drag is shown in the preview but is not an edit
    let drag = FrameDrag {
        target: w,
        value: FrameValue::Move(Pos { x: 6000, y: 5000 }),
    };
    let n = app.history.undo_len();
    app.dispatch(Command::DragPicture(Some(drag)), now())
        .unwrap();
    assert_eq!(app.history.undo_len(), n);
    assert_eq!(app.project.timeline.move_keys(w).len(), 1);
    let shown = app.shown_project();
    assert_eq!(shown.timeline.move_keys(w).len(), 2);
    drop(shown);
    // released: one step, one new key at the playhead
    app.dispatch(Command::SetPicture(drag), now()).unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    assert_eq!(app.project.timeline.move_keys(w).len(), 2);
    assert_eq!(app.move_at(w), Some(Pos { x: 6000, y: 5000 }));
    assert!(app.frame_drag.is_none());
    assert_eq!(app.undo_text(), "Undo Move");
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.project.timeline.move_keys(w).len(), 1);
    // the picture's rectangle follows (frame fractions), and the handles belong to the
    // clip under the playhead (its copy of the whole-video effect, REQ-148)
    app.dispatch(Command::Redo, now()).unwrap();
    let (r, _) = app.picture_rect().unwrap();
    assert!(((r.0 + r.2) / 2.0 - 0.6).abs() < 1e-6);
    let c0 = clip(&app, 0);
    assert_eq!(
        app.frame_target_at_playhead(FrameFx::Move),
        Some(FrameTarget::Clip(c0))
    );
    apply(&mut app, EffectKind::Move, Target::Clip(c0)).unwrap();
    assert_eq!(
        app.frame_target_at_playhead(FrameFx::Move),
        Some(FrameTarget::Clip(c0))
    );
    assert_eq!(app.frame_target_at_playhead(FrameFx::Scale), None);
}

// covers: REQ-130
#[test]
fn a_pasted_clip_keeps_its_keyframes_and_the_selected_clip_is_the_scaled_one() {
    let (_d, mut app) = two();
    let c0 = clip(&app, 0);
    apply(&mut app, EffectKind::Scale, Target::Clip(c0)).unwrap();
    assert_eq!(app.selected_clip, Some(c0));
    app.dispatch(Command::Copy, now()).unwrap();
    app.dispatch(Command::Seek(s(10)), now()).unwrap();
    app.dispatch(Command::Paste, now()).unwrap();
    let pasted = app.project.timeline.clips.last().unwrap().id;
    assert_ne!(pasted, c0);
    assert_eq!(
        app.project
            .timeline
            .scale_keys(FrameTarget::Clip(pasted))
            .len(),
        1
    );
    // cut and paste too
    app.dispatch(Command::SelectClip(Some(pasted)), now())
        .unwrap();
    app.dispatch(Command::Cut, now()).unwrap();
    assert!(app.project.timeline.frames_of(pasted).is_none());
    app.dispatch(Command::Paste, now()).unwrap();
    let again = app.project.timeline.clips.last().unwrap().id;
    assert_eq!(
        app.project
            .timeline
            .scale_keys(FrameTarget::Clip(again))
            .len(),
        1
    );
}

// covers: REQ-129
#[test]
fn markers_and_instances_find_their_clip() {
    let (_d, mut app) = two();
    let c1 = clip(&app, 1);
    apply(&mut app, EffectKind::Scale, Target::Clip(c1)).unwrap();
    apply(&mut app, EffectKind::Move, Target::WholeVideo).unwrap();
    let at = |i| app.instance_start(i);
    assert_eq!(at(Instance::Scale(FrameTarget::Clip(c1))), Some(s(5)));
    assert_eq!(at(Instance::Move(FrameTarget::Clip(c1))), Some(s(5)));
    assert_eq!(at(Instance::Aspect), Some(Micros::ZERO));
    // no whole-video Scale popup: a Whole target has no start
    assert_eq!(at(Instance::Scale(FrameTarget::Whole)), None);
    assert_eq!(
        Instance::Scale(FrameTarget::Clip(c1)).frame_fx(),
        Some(FrameFx::Scale)
    );
    assert_eq!(Instance::Aspect.frame_fx(), None);
}

// covers: REQ-130
#[test]
fn effect_commands_are_disabled_without_clips() {
    let mut app = state(FakeDialogs::default());
    for effect in [
        EffectKind::ChangeAspect,
        EffectKind::Scale,
        EffectKind::Move,
    ] {
        let cmd = Command::ApplyEffect {
            effect,
            target: Target::WholeVideo,
        };
        assert!(!app.is_enabled(&cmd), "{effect:?}");
        assert!(app.dispatch(cmd, now()).is_err());
        assert!(!app.is_enabled(&Command::OpenFrameEffect(effect)));
    }
    assert!(!app.is_enabled(&Command::SetAspect(FrameAspect::default())));
    assert!(app.picture_rect().is_none());
    assert!(!app.is_enabled(&Command::AddKey {
        target: FrameTarget::Whole,
        effect: FrameFx::Scale
    }));
    assert!(!app.is_enabled(&Command::RemoveFrameEffect {
        target: FrameTarget::Clip(ClipId(9)),
        effect: FrameFx::Move
    }));
}

// covers: REQ-282
#[test]
fn the_easing_of_a_keyframe_segment_reaches_the_preview_value_and_the_export_expression() {
    let (_d, mut app) = two();
    let c = FrameTarget::Clip(clip(&app, 0));
    let c0 = clip(&app, 0);
    apply(&mut app, EffectKind::Scale, Target::Clip(c0)).unwrap();
    app.dispatch(Command::Seek(s(4)), now()).unwrap();
    app.dispatch(
        Command::AddKey {
            target: c,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetPicture(FrameDrag {
            target: c,
            value: FrameValue::Scale(Scale {
                permille: 2000,
                dx: 0,
                dy: 0,
            }),
        }),
        now(),
    )
    .unwrap();
    // select the first key (at 0 s) and ease the segment after it
    app.dispatch(Command::SelectKey(Some(Micros::ZERO)), now())
        .unwrap();
    app.dispatch(Command::Seek(s(1)), now()).unwrap();
    let straight = app.scale_at(c).unwrap().permille;
    assert_eq!(straight, 1250);
    let undo = app.history.undo_len();
    app.dispatch(
        Command::SetKeyEase {
            target: c,
            effect: FrameFx::Scale,
            ease: crate::keyframes::Ease::EaseInOut,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo + 1);
    let eased = app.scale_at(c).unwrap().permille;
    assert!(
        eased < straight,
        "a quarter of the time: behind the straight line, {eased}"
    );
    app.dispatch(Command::Seek(s(2)), now()).unwrap();
    assert_eq!(
        app.scale_at(c).unwrap().permille,
        1500,
        "the middle is the same"
    );
    // the export expression is the same curve
    let stack = app.project.timeline.frame_stack(0);
    assert!(
        stack.scale_factor().contains("3-2*"),
        "{}",
        stack.scale_factor()
    );
    // the same choice again changes nothing; no key selected: refused
    assert_eq!(app.history.undo_len(), undo + 1);
    app.dispatch(Command::SelectKey(None), now()).unwrap();
    assert!(
        app.dispatch(
            Command::SetKeyEase {
                target: c,
                effect: FrameFx::Scale,
                ease: crate::keyframes::Ease::Straight
            },
            now()
        )
        .is_err()
    );
}

// covers: REQ-281
#[test]
fn speed_changes_the_clip_length_in_one_undo_step_and_the_popup_offers_it() {
    let (_d, mut app) = two();
    let c0 = clip(&app, 0);
    let before = app.project.timeline.duration();
    let undo = app.history.undo_len();
    app.dispatch(
        Command::SetSpeed {
            clip: c0,
            speed: 2000,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(app.project.timeline.clips[0].len(), Micros(2_500_000));
    assert_eq!(app.project.timeline.duration(), before - Micros(2_500_000));
    // the same value again is not a step; a value outside 50-200 % is refused
    app.dispatch(
        Command::SetSpeed {
            clip: c0,
            speed: 2000,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), undo + 1);
    assert!(
        app.dispatch(
            Command::SetSpeed {
                clip: c0,
                speed: 100
            },
            now()
        )
        .is_err()
    );
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(app.project.timeline.duration(), before);
}
