// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 2.0.4, batch 7: the whole-video copies shown apart (REQ-200), the offer for a new
//! clip (REQ-201), keyframe time and copies (REQ-203, REQ-204) and the same-file LUT (REQ-205).

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::frame::Scale;
use crate::timeline::KeysClip;

fn now() -> Instant {
    Instant::now()
}

/// Three clips: a, b, a (5 s each).
fn three() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    for i in [0, 1, 0] {
        let m = app.project.media[i].id;
        app.dispatch(Command::Append(Some(m)), now()).unwrap();
    }
    (dir, app)
}

fn clip(app: &AppState, i: usize) -> ClipId {
    app.project.timeline.clips[i].id
}

fn apply_whole_scale(app: &mut AppState) {
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::WholeVideo,
        },
        now(),
    )
    .unwrap();
}

fn scale_of(app: &AppState, i: usize) -> Vec<u16> {
    app.project
        .timeline
        .scale_keys(FrameTarget::Clip(clip(app, i)))
        .iter()
        .map(|k| k.v.permille)
        .collect()
}

// covers: REQ-200
#[test]
fn the_whole_video_popup_edits_the_selected_copy_only_and_remove_effect_removes_them_all() {
    let (_d, mut app) = three();
    apply_whole_scale(&mut app);
    for i in 0..3 {
        assert_eq!(scale_of(&app, i), [1000], "one copy per clip");
    }
    // by default the first copy is shown; selecting clip 2 shows that one
    assert_eq!(app.selected_copy(FrameFx::Scale), Some(clip(&app, 0)));
    app.dispatch(Command::SelectCopy(clip(&app, 1)), now())
        .unwrap();
    assert_eq!(app.selected_copy(FrameFx::Scale), Some(clip(&app, 1)));
    assert_eq!(
        app.playhead,
        Micros(5_000_000),
        "the playhead goes to that clip"
    );
    // a change of the selected copy leaves clips 1 and 3 as they were
    let c2 = FrameTarget::Clip(clip(&app, 1));
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetPicture(FrameDrag {
            target: c2,
            value: FrameValue::Scale(Scale {
                permille: 1500,
                ..Scale::NORMAL
            }),
        }),
        now(),
    )
    .unwrap();
    assert_eq!(
        (scale_of(&app, 0), scale_of(&app, 1), scale_of(&app, 2)),
        (vec![1000], vec![1500], vec![1000])
    );
    assert_eq!(app.history.undo_len(), n + 1);
    // Add and Remove keyframe act on the selected copy only
    app.dispatch(Command::Seek(Micros(7_000_000)), now())
        .unwrap();
    app.dispatch(
        Command::AddKey {
            target: c2,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.project.timeline.scale_keys(c2).len(), 2);
    assert_eq!(scale_of(&app, 0).len(), 1);
    assert_eq!(scale_of(&app, 2).len(), 1);
    // a selected row that loses the effect falls back to the first copy
    // Remove effect removes the three copies, one undo restores them
    let before = app.history.undo_len();
    app.dispatch(
        Command::RemoveFrameEffect {
            target: FrameTarget::Whole,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert!((0..3).all(|i| scale_of(&app, i).is_empty()));
    assert_eq!(app.history.undo_len(), before + 1);
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(
        (
            scale_of(&app, 0),
            scale_of(&app, 1).len(),
            scale_of(&app, 2)
        ),
        (vec![1000], 2, vec![1000])
    );
    assert!(
        app.dispatch(Command::SelectCopy(ClipId(99)), now())
            .is_err()
    );
}

// covers: REQ-201
#[test]
fn an_appended_clip_without_the_whole_video_volume_is_offered_it_and_gets_it_only_by_the_button() {
    let (_d, mut app) = three();
    for i in 0..3 {
        let c = clip(&app, i);
        app.dispatch(
            Command::SetClipVolume {
                clip: c,
                volume: 50,
                muted: false,
            },
            now(),
        )
        .unwrap();
    }
    assert!(app.new_clip_offer_text().is_none(), "no clip was added yet");
    let m = app.project.media[0].id;
    app.dispatch(Command::Append(Some(m)), now()).unwrap();
    assert_eq!(
        app.new_clip_offer_text().as_deref(),
        Some("The new clip has no Volume")
    );
    assert!(app.is_enabled(&Command::ApplyWholeToNew));
    // without the button the new clip has no Volume
    let new = clip(&app, 3);
    assert!(!app.project.timeline.get(new).unwrap().has_volume());
    // the button sets it to 50 % and changes no other clip; one undo step
    let n = app.history.undo_len();
    app.dispatch(Command::ApplyWholeToNew, now()).unwrap();
    assert_eq!(app.project.timeline.get(new).unwrap().volume, 50);
    assert_eq!(app.history.undo_len(), n + 1);
    assert!((0..3).all(|i| app.project.timeline.clips[i].volume == 50));
    assert!(app.new_clip_offer_text().is_none());
    app.dispatch(Command::Undo, now()).unwrap();
    assert!(!app.project.timeline.get(new).unwrap().has_volume());
    assert!((0..3).all(|i| app.project.timeline.clips[i].volume == 50));
    // the offer is gone with an undone append
    app.dispatch(Command::Undo, now()).unwrap();
    assert!(app.new_clip_offer_text().is_none());
}

// covers: REQ-201
#[test]
fn pasted_and_split_off_clips_are_offered_too_and_every_shared_effect_is_named() {
    let (_d, mut app) = three();
    apply_whole_scale(&mut app);
    for i in 0..3 {
        let c = clip(&app, i);
        app.dispatch(
            Command::SetCleanAudio {
                target: Target::Clip(c),
                on: true,
            },
            now(),
        )
        .unwrap();
    }
    // copy a clip, take its effects off, paste: the pasted clip lacks both
    app.clipboard = Some(Clip::new(
        ClipId(99),
        app.project.media[0].id,
        Micros::ZERO,
        Micros(2_000_000),
    ));
    app.dispatch(Command::Seek(Micros(2_000_000)), now())
        .unwrap();
    app.dispatch(Command::Paste, now()).unwrap();
    let t = app.new_clip_offer_text().expect("an offer after the paste");
    assert!(
        t.starts_with("The new clip has no ") && t.contains("Clean audio") && t.contains("Scale"),
        "{t}"
    );
    app.dispatch(Command::ApplyWholeToNew, now()).unwrap();
    let new = app.new_clip_offer.is_none();
    assert!(new);
    let pasted = app
        .project
        .timeline
        .clips
        .iter()
        .find(|c| c.clean_audio && c.src_out == Micros(2_000_000))
        .copied()
        .unwrap();
    assert!(
        !app.project
            .timeline
            .scale_keys(FrameTarget::Clip(pasted.id))
            .is_empty()
    );
    // a split keeps the effects on both halves: nothing is offered
    app.dispatch(Command::Seek(Micros(1_000_000)), now())
        .unwrap();
    app.dispatch(Command::Split, now()).unwrap();
    assert!(app.new_clip_offer_text().is_none());
    // the effect the others do not all share is not offered
    app.dispatch(
        Command::RemoveFrameEffect {
            target: FrameTarget::Clip(clip(&app, 0)),
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    let m = app.project.media[1].id;
    app.dispatch(Command::Append(Some(m)), now()).unwrap();
    assert_eq!(
        app.new_clip_offer_text().as_deref(),
        Some("The new clip has no Clean audio")
    );
}

// covers: REQ-204
#[test]
fn copied_keyframes_replace_those_of_the_target_at_the_same_clip_times_and_nothing_else() {
    let (_d, mut app) = three();
    let (c1, c3) = (clip(&app, 0), clip(&app, 2));
    let t1 = FrameTarget::Clip(c1);
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c1),
        },
        now(),
    )
    .unwrap();
    app.dispatch(Command::Seek(Micros(2_000_000)), now())
        .unwrap();
    app.dispatch(
        Command::AddKey {
            target: t1,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::SetPicture(FrameDrag {
            target: t1,
            value: FrameValue::Scale(Scale {
                permille: 2000,
                ..Scale::NORMAL
            }),
        }),
        now(),
    )
    .unwrap();
    // clip 3 has its own Move and a different Scale
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Move,
            target: Target::Clip(c3),
        },
        now(),
    )
    .unwrap();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c3),
        },
        now(),
    )
    .unwrap();
    let move_before = app
        .project
        .timeline
        .move_keys(FrameTarget::Clip(c3))
        .to_vec();
    // nothing to paste yet
    assert!(!app.is_enabled(&Command::PasteKeys {
        clip: c3,
        effect: FrameFx::Scale
    }));
    app.dispatch(
        Command::CopyKeys {
            clip: c1,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert!(matches!(app.clipboard_keys, Some(KeysClip::Scale(_))));
    // the copied effect only
    assert!(
        app.dispatch(
            Command::PasteKeys {
                clip: c3,
                effect: FrameFx::Move
            },
            now()
        )
        .is_err()
    );
    let n = app.history.undo_len();
    app.dispatch(
        Command::PasteKeys {
            clip: c3,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    // clip 3 has the same keys at the same clip times (relative to each clip's start)
    fn rel(app: &AppState, id: ClipId) -> Vec<(i64, u16)> {
        let c = app.project.timeline.get(id).unwrap();
        app.project
            .timeline
            .scale_keys(FrameTarget::Clip(id))
            .iter()
            .map(|k| ((k.t - c.src_in).0, k.v.permille))
            .collect()
    }
    assert_eq!(rel(&app, c3), rel(&app, c1));
    assert_eq!(rel(&app, c3), [(0, 1000), (2_000_000, 2000)]);
    // its other effect is untouched; one undo restores the old Scale
    assert_eq!(
        app.project.timeline.move_keys(FrameTarget::Clip(c3)),
        &move_before[..]
    );
    app.dispatch(Command::Undo, now()).unwrap();
    assert_eq!(rel(&app, c3), [(0, 1000)]);
    // a clip without the effect gets it from the paste
    let c2 = clip(&app, 1);
    app.dispatch(
        Command::PasteKeys {
            clip: c2,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    assert_eq!(rel(&app, c2).len(), 2);
    assert!(
        app.dispatch(
            Command::CopyKeys {
                clip: c2,
                effect: FrameFx::Move
            },
            now()
        )
        .is_err()
    );
}

// covers: REQ-203
#[test]
fn a_keyframe_moves_in_time_by_a_typed_time_or_a_frame_and_never_onto_another() {
    let (_d, mut app) = three();
    let c = clip(&app, 0);
    let t = FrameTarget::Clip(c);
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::Scale,
            target: Target::Clip(c),
        },
        now(),
    )
    .unwrap();
    app.dispatch(Command::Seek(Micros(2_000_000)), now())
        .unwrap();
    app.dispatch(
        Command::AddKey {
            target: t,
            effect: FrameFx::Scale,
        },
        now(),
    )
    .unwrap();
    let key = app.popups.key.unwrap();
    assert_eq!(key, Micros(2_000_000));
    // a typed time: 3.5 s
    let n = app.history.undo_len();
    app.dispatch(
        Command::MoveKey {
            clip: c,
            effect: FrameFx::Scale,
            from: key,
            to: Micros(3_500_000),
        },
        now(),
    )
    .unwrap();
    assert_eq!(
        app.popups.key,
        Some(Micros(3_500_000)),
        "the key stays selected"
    );
    assert_eq!(app.history.undo_len(), n + 1);
    // one frame to the right (Right key)
    let frame = app.frame_len();
    app.dispatch(
        Command::MoveKey {
            clip: c,
            effect: FrameFx::Scale,
            from: Micros(3_500_000),
            to: Micros(3_500_000) + frame,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.popups.key, Some(Micros(3_500_000) + frame));
    // onto the first key: refused; past the clip: stops at its end
    assert!(
        app.dispatch(
            Command::MoveKey {
                clip: c,
                effect: FrameFx::Scale,
                from: app.popups.key.unwrap(),
                to: Micros::ZERO
            },
            now()
        )
        .is_err()
    );
    app.dispatch(
        Command::MoveKey {
            clip: c,
            effect: FrameFx::Scale,
            from: app.popups.key.unwrap(),
            to: Micros(99_000_000),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.popups.key, Some(Micros(5_000_000)));
    // no change, no step
    let n = app.history.undo_len();
    app.dispatch(
        Command::MoveKey {
            clip: c,
            effect: FrameFx::Scale,
            from: Micros(5_000_000),
            to: Micros(5_000_000),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    assert!(
        app.dispatch(
            Command::MoveKey {
                clip: c,
                effect: FrameFx::Scale,
                from: Micros(7),
                to: Micros(8)
            },
            now()
        )
        .is_err()
    );
}

// covers: REQ-205
#[test]
fn applying_the_lut_file_a_clip_already_has_changes_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let warm = dir.path().join("warm.cube");
    let cold = dir.path().join("cold.cube");
    for p in [&warm, &cold] {
        std::fs::write(p, crate::lut::tests::identity(2)).unwrap();
    }
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir.path()));
    for i in [0, 1] {
        let m = app.project.media[i].id;
        app.dispatch(Command::Append(Some(m)), now()).unwrap();
    }
    let c = FrameTarget::Clip(clip(&app, 1));
    app.dispatch(
        Command::SetLut {
            target: c,
            path: warm.clone(),
        },
        now(),
    )
    .unwrap();
    let n = app.history.undo_len();
    let before = app.project.timeline.clone();
    // the same file again: message, no undo step, nothing changed
    app.dispatch(
        Command::SetLut {
            target: c,
            path: warm.clone(),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.status, "The clip already uses warm.cube");
    assert_eq!(app.history.undo_len(), n);
    assert_eq!(app.project.timeline, before);
    // another file replaces it
    app.dispatch(
        Command::SetLut {
            target: c,
            path: cold.clone(),
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.project.timeline.lut_of(c), Some(cold.to_str().unwrap()));
    assert_eq!(app.history.undo_len(), n + 1);
    // the whole video: when every clip has it already, the same message; else it applies
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Whole,
            path: warm.clone(),
        },
        now(),
    )
    .unwrap();
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetLut {
            target: FrameTarget::Whole,
            path: warm,
        },
        now(),
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    assert!(app.status.starts_with("Every clip already uses"));
}
