// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Overlap media in the application (2.3.0): adding over the clip under the playhead, moving and
//! resizing, the ends, the edits of a clip with overlaps and their one warning, Change speed and
//! the Volume effect (REQ-280, REQ-281, REQ-325, REQ-327, REQ-329, REQ-331).

use super::tests::{FakeDialogs, files, import, state};
use super::*;
use crate::overlap::Kind;

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Three clips: a.mp4 (5 s), b.mp4 trimmed to 3 s, a.mp4 (5 s).
fn three(app: &mut AppState, dir: &Path) -> Vec<ClipId> {
    let f = files(dir);
    import(app, f);
    let now = Instant::now();
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    for m in [a, b, a] {
        app.dispatch(Command::Append(Some(m)), now).unwrap();
    }
    let c = app.project.timeline.clips[1].id;
    app.dispatch(
        Command::SetTrim {
            clip: c,
            src_in: s(1),
            src_out: s(4),
        },
        now,
    )
    .unwrap();
    app.project.timeline.clips.iter().map(|c| c.id).collect()
}

fn png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, crate::picture::tests::header(w, h)).unwrap();
    p
}

fn add(app: &mut AppState, file: &Path, at: Micros) -> Result<(), String> {
    app.dispatch(Command::Seek(at), Instant::now()).unwrap();
    app.dispatch(
        Command::AddOverlap(Some(file.to_path_buf())),
        Instant::now(),
    )
}

fn overlaps(app: &AppState, c: ClipId) -> Vec<crate::overlap::Overlap> {
    app.project.timeline.overlaps_of(c).to_vec()
}

// covers: REQ-325, REQ-331, REQ-316
#[test]
fn an_overlap_is_added_over_the_clip_under_the_playhead_for_three_seconds_cut_to_the_clip() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let logo = png(dir.path(), "logo.png", 400, 200);
    let n = app.history.undo_len();
    // over clip 3 (8 s to 13 s) at 10 s: 2 s into it, 3 s long
    add(&mut app, &logo, s(10)).unwrap();
    let o = &overlaps(&app, c[2])[0];
    assert_eq!((o.from, o.len), (s(2), s(3)));
    assert_eq!(o.kind, Kind::Still);
    assert_eq!((o.src_w, o.src_h), (400, 200));
    assert_eq!((o.place.x, o.place.y), (5000, 5000));
    assert!(o.width > 0 && o.height > 0);
    assert_eq!(app.selected_overlap, Some(o.id));
    assert_eq!(app.selected_clip, Some(c[2]));
    assert_eq!(app.history.undo_len(), n + 1);
    assert_eq!(app.undo_text(), "Undo Add overlap");
    assert!(app.project.timeline.overlaps_of(c[0]).is_empty());
    // near the end of the clip it is cut to the clip
    add(&mut app, &logo, s(12)).unwrap();
    let last = overlaps(&app, c[2]).pop().unwrap();
    assert_eq!((last.from, last.len), (s(4), s(1)));
    assert_ne!(last.id, o.id, "ids are unique in the project");
    // one undo takes it away, one redo brings it back
    app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert_eq!(overlaps(&app, c[2]).len(), 1);
    app.dispatch(Command::Redo, Instant::now()).unwrap();
    assert_eq!(overlaps(&app, c[2]).len(), 2);
    // the overlap is a row of the picture group, on top of the others
    let rows = app.project.order_of(c[2]);
    assert!(
        rows.contains(&crate::effect_order::Row::Overlap(o.id)),
        "{rows:?}"
    );
}

// covers: REQ-325
#[test]
fn with_no_clip_under_the_playhead_nothing_is_added_and_the_status_line_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let logo = png(dir.path(), "logo.png", 400, 200);
    let end = app.project.timeline.duration();
    let n = app.history.undo_len();
    let e = add(&mut app, &logo, end).unwrap_err();
    assert_eq!(e, overlap::NO_CLIP_UNDER_PLAYHEAD);
    assert_eq!(app.status, overlap::NO_CLIP_UNDER_PLAYHEAD);
    assert!(!app.project.timeline.any_overlap());
    assert_eq!(app.history.undo_len(), n);
    // the command is greyed out with that reason
    assert!(!app.is_enabled(&Command::AddOverlap(None)));
    assert_eq!(
        app.disabled_reason(&Command::AddOverlap(None)),
        Some(overlap::NO_CLIP_UNDER_PLAYHEAD)
    );
}

// covers: REQ-325, REQ-316, REQ-002
#[test]
fn a_broken_file_or_a_picture_above_sixteen_megapixels_is_refused_with_its_message() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    three(&mut app, dir.path());
    let broken = dir.path().join("broken.mov");
    std::fs::write(&broken, b"not a video at all").unwrap();
    let e = add(&mut app, &broken, s(1)).unwrap_err();
    assert!(
        e.contains("broken.mov") && e.contains("not a picture or video"),
        "{e}"
    );
    let big = png(dir.path(), "big.png", 4001, 4000);
    let e = add(&mut app, &big, s(1)).unwrap_err();
    assert_eq!(
        e,
        "The picture is larger than 16 megapixels (4001 x 4000) and was not added."
    );
    let gone = dir.path().join("gone.png");
    assert!(add(&mut app, &gone, s(1)).is_err());
    assert!(!app.project.timeline.any_overlap());
    // 16 megapixels exactly is accepted, and a GIF plays (it is a motion overlap)
    let ok = png(dir.path(), "ok.png", 4000, 4000);
    add(&mut app, &ok, s(1)).unwrap();
    let gif = dir.path().join("a.gif");
    std::fs::write(&gif, crate::picture::tests::gif_header(300, 200)).unwrap();
    add(&mut app, &gif, s(2)).unwrap();
    let c0 = app.project.timeline.clips[0].id;
    let kinds: Vec<Kind> = overlaps(&app, c0).iter().map(|o| o.kind).collect();
    assert_eq!(kinds, [Kind::Still, Kind::Motion]);
    // a video is measured by ffprobe (the fake one answers for the files of these tests)
    let video = dir.path().join("a.mp4");
    add(&mut app, &video, s(3)).unwrap();
    let last = overlaps(&app, c0).pop().unwrap();
    assert_eq!(last.kind, Kind::Motion);
    assert_eq!((last.src_w, last.src_h), (1280, 720));
    assert_eq!(last.name(), "a.mp4");
    // a dialog that is closed adds nothing
    let n = app.history.undo_len();
    app.dispatch(Command::AddOverlap(None), Instant::now())
        .unwrap();
    assert_eq!(app.history.undo_len(), n);
}

// covers: REQ-325, REQ-327
#[test]
fn the_box_and_the_ends_are_changed_in_one_undo_step_each_and_kept_inside_the_clip() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    add(&mut app, &png(dir.path(), "l.png", 400, 200), s(1)).unwrap();
    let id = overlaps(&app, c[0])[0].id;
    let now = Instant::now();
    // a move: the same size, one step named Move
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetOverlapBox {
            id,
            place: Pos { x: 2000, y: 3000 },
            width: 300,
            height: 150,
            reshaped: false,
        },
        now,
    )
    .unwrap();
    let o = &overlaps(&app, c[0])[0];
    assert_eq!((o.place.x, o.place.y, o.reshaped), (2000, 3000, false));
    assert_eq!(app.history.undo_len(), n + 1);
    // the same value again is no step; a resize is named so
    app.dispatch(
        Command::SetOverlapBox {
            id,
            place: Pos { x: 2000, y: 3000 },
            width: 300,
            height: 150,
            reshaped: false,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    app.dispatch(
        Command::SetOverlapBox {
            id,
            place: Pos { x: 20_000, y: 3000 },
            width: 2000,
            height: 0,
            reshaped: true,
        },
        now,
    )
    .unwrap();
    let o = &overlaps(&app, c[0])[0];
    assert_eq!(
        (o.place.x, o.width, o.height, o.reshaped),
        (10_000, 1000, 1, true)
    );
    assert_eq!(app.undo_text(), "Undo Resize overlap");
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(overlaps(&app, c[0])[0].width, 300);
    // the ends: the end to 5 s (the clip's end), the start before the clip's start
    app.dispatch(
        Command::SetOverlapTimes {
            id,
            from: s(1),
            len: s(10),
        },
        now,
    )
    .unwrap();
    let o = &overlaps(&app, c[0])[0];
    assert_eq!((o.from, o.end()), (s(1), s(5)));
    app.dispatch(
        Command::SetOverlapTimes {
            id,
            from: Micros(-500_000),
            len: s(2),
        },
        now,
    )
    .unwrap();
    assert_eq!(overlaps(&app, c[0])[0].from, Micros::ZERO);
    // select and remove
    app.dispatch(Command::SelectOverlap(Some(id)), now).unwrap();
    assert_eq!(app.selected_overlap, Some(id));
    app.dispatch(Command::RemoveOverlap(id), now).unwrap();
    assert!(!app.project.timeline.any_overlap());
    assert_eq!(app.selected_overlap, None);
    assert!(app.dispatch(Command::RemoveOverlap(id), now).is_err());
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(overlaps(&app, c[0]).len(), 1);
}

// covers: REQ-329
#[test]
fn a_split_cuts_each_overlap_to_the_part_where_it_lies_with_no_warning() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let logo = png(dir.path(), "l.png", 400, 200);
    add(&mut app, &logo, s(1)).unwrap(); // 1 s to 4 s
    add(&mut app, &logo, s(4)).unwrap(); // 4 s to 5 s (cut to the clip)
    let first = overlaps(&app, c[0])[0].id;
    // the clip split at 2.5 s: the first overlap is in both parts, the second only in the second
    app.dispatch(Command::Seek(Micros(2_500_000)), Instant::now())
        .unwrap();
    app.dispatch(Command::Split, Instant::now()).unwrap();
    assert_eq!(app.modal, None, "no warning when a part is kept");
    let ids: Vec<ClipId> = app.project.timeline.clips.iter().map(|c| c.id).collect();
    let (left, right) = (overlaps(&app, ids[0]), overlaps(&app, ids[1]));
    assert_eq!(left.len(), 1);
    assert_eq!(
        (left[0].id, left[0].from, left[0].end()),
        (first, s(1), Micros(2_500_000))
    );
    assert_eq!(right.len(), 2);
    assert_eq!(
        (right[0].from, right[0].end()),
        (Micros::ZERO, Micros(1_500_000))
    );
    assert_eq!(
        (right[1].from, right[1].end()),
        (Micros(1_500_000), Micros(2_500_000))
    );
    let mut all = app.project.timeline.overlap_ids();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), 3, "every overlap has its own id");
    // 2 s to 6 s on an 8 s clip, split at 4 s: 2 s to 4 s in the first part
    let o = crate::overlap::Overlap {
        from: s(2),
        len: s(4),
        ..right[0].clone()
    };
    assert_eq!(
        o.kept_part(Micros::ZERO, s(4)).map(|p| (p.from, p.len)),
        Some((s(2), s(2)))
    );
}

// covers: REQ-329
#[test]
fn deleting_a_clip_with_overlaps_asks_once_cancel_changes_nothing_and_ok_is_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let logo = png(dir.path(), "l.png", 400, 200);
    add(&mut app, &logo, s(9)).unwrap();
    add(&mut app, &logo, s(10)).unwrap();
    let now = Instant::now();
    app.dispatch(Command::SelectClip(Some(c[2])), now).unwrap();
    let before = app.project.edit();
    let n = app.history.undo_len();
    // one popup for both overlaps
    app.dispatch(Command::Delete, now).unwrap();
    assert_eq!(app.modal, Some(Modal::RemoveOverlaps(2)));
    assert!(crate::app::overlap::removal_text(2).contains("remove 2 overlaps"));
    assert!(crate::app::overlap::removal_text(1).contains("remove 1 overlap."));
    assert_eq!(app.project.edit(), before, "nothing changed yet");
    assert_eq!(app.history.undo_len(), n);
    // Cancel: the clip and both overlaps stay
    app.dispatch(Command::CancelRemoval, now).unwrap();
    assert_eq!(app.modal, None);
    assert_eq!(app.project.edit(), before);
    assert_eq!(app.history.undo_len(), n);
    // OK: the clip goes with its overlaps, in one undo step
    app.dispatch(Command::Delete, now).unwrap();
    assert_eq!(app.modal, Some(Modal::RemoveOverlaps(2)));
    app.dispatch(Command::ConfirmRemoval, now).unwrap();
    assert_eq!(app.modal, None);
    assert_eq!(app.project.timeline.clips.len(), 2);
    assert!(!app.project.timeline.any_overlap());
    assert_eq!(app.history.undo_len(), n + 1);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(
        app.project.edit(),
        before,
        "one undo restores the clip and both overlaps"
    );
    // the buttons are only there while an edit waits
    assert!(!app.is_enabled(&Command::ConfirmRemoval));
}

// covers: REQ-329
#[test]
fn a_trim_removes_only_the_overlaps_with_nothing_left_and_asks_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let logo = png(dir.path(), "l.png", 400, 200);
    add(&mut app, &logo, s(1)).unwrap(); // clip 1: 1 s to 4 s
    add(&mut app, &logo, s(4)).unwrap(); // 4 s to 5 s
    let now = Instant::now();
    // the start of the clip cut by 2 s: the first overlap is cut (1 s left), the second moves
    app.dispatch(
        Command::SetTrim {
            clip: c[0],
            src_in: s(2),
            src_out: s(5),
        },
        now,
    )
    .unwrap();
    assert_eq!(app.modal, None);
    let o = overlaps(&app, c[0]);
    assert_eq!((o[0].from, o[0].len), (Micros::ZERO, s(2)));
    assert_eq!((o[1].from, o[1].len), (s(2), s(1)));
    // the clip cut at the end to 1 s: the second overlap has nothing left: asked first
    let before = app.project.edit();
    let cmd = Command::SetTrim {
        clip: c[0],
        src_in: s(2),
        src_out: s(3),
    };
    app.dispatch(cmd, now).unwrap();
    assert_eq!(app.modal, Some(Modal::RemoveOverlaps(1)));
    assert_eq!(app.project.edit(), before);
    app.dispatch(Command::ConfirmRemoval, now).unwrap();
    let o = overlaps(&app, c[0]);
    assert_eq!(o.len(), 1);
    assert_eq!((o[0].from, o[0].end()), (Micros::ZERO, s(1)));
    // the trim that gives the time back keeps the overlaps where they were
    app.dispatch(
        Command::SetTrim {
            clip: c[0],
            src_in: s(1),
            src_out: s(3),
        },
        now,
    )
    .unwrap();
    assert_eq!(overlaps(&app, c[0])[0].from, s(1));
}

// covers: REQ-329, REQ-281
#[test]
fn a_change_of_speed_rescales_the_overlaps_and_one_speed_for_all_clips_is_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    add(&mut app, &png(dir.path(), "l.png", 400, 200), s(1)).unwrap();
    let now = Instant::now();
    let n = app.history.undo_len();
    app.dispatch(
        Command::SetSpeed {
            clip: c[0],
            speed: 2000,
        },
        now,
    )
    .unwrap();
    let o = &overlaps(&app, c[0])[0];
    assert_eq!((o.from, o.len), (Micros(500_000), Micros(1_500_000)));
    assert_eq!(app.project.timeline.clips[0].len(), Micros(2_500_000));
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(overlaps(&app, c[0])[0].from, s(1));
    app.dispatch(Command::Redo, now).unwrap();
    // every clip at once: a copy on each, one undo step
    let n2 = app.history.undo_len();
    assert_eq!(n2, n + 1);
    app.dispatch(Command::SetSpeedAll { speed: 1500 }, now)
        .unwrap();
    assert!(app.project.timeline.clips.iter().all(|c| c.speed == 1500));
    assert_eq!(app.history.undo_len(), n2 + 1);
    assert_eq!(app.undo_text(), "Undo Change speed");
    // the same speed again changes nothing
    app.dispatch(Command::SetSpeedAll { speed: 1500 }, now)
        .unwrap();
    assert_eq!(app.history.undo_len(), n2 + 1);
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.timeline.clips[1].speed, 1000);
    assert_eq!(app.project.timeline.clips[0].speed, 2000);
}

// covers: REQ-281
#[test]
fn change_speed_is_an_effect_for_one_clip_or_all_clips_and_nothing_is_set_until_a_value_is() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let now = Instant::now();
    assert!(crate::app::EffectKind::ALL.contains(&crate::app::EffectKind::ChangeSpeed));
    assert_eq!(crate::app::EffectKind::ChangeSpeed.name(), "Change speed");
    let n = app.history.undo_len();
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::ChangeSpeed,
            target: Target::Clip(c[1]),
        },
        now,
    )
    .unwrap();
    assert_eq!(
        app.popups.instance,
        Some(Instance::Speed(FrameTarget::Clip(c[1])))
    );
    assert_eq!(app.popup_title(Popup::Instance), "Change speed — b.mp4");
    app.dispatch(
        Command::ApplyEffect {
            effect: EffectKind::ChangeSpeed,
            target: Target::WholeVideo,
        },
        now,
    )
    .unwrap();
    assert_eq!(
        app.popups.instance,
        Some(Instance::Speed(FrameTarget::Whole))
    );
    assert_eq!(
        app.popup_title(Popup::Instance),
        "Change speed — every clip"
    );
    assert_eq!(app.history.undo_len(), n, "no value, no change");
    // the speed row of the Applied effects widget opens the popup and is fixed
    app.dispatch(
        Command::SetSpeed {
            clip: c[1],
            speed: 500,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SelectClip(Some(c[1])), now).unwrap();
    let row = app
        .applied()
        .unwrap()
        .rows
        .into_iter()
        .find(|r| r.name.starts_with("Change speed"))
        .unwrap();
    assert_eq!(row.name, "Change speed: 50%");
    assert!(row.row.is_none());
    // out of range values are refused
    assert!(
        app.dispatch(
            Command::SetSpeed {
                clip: c[1],
                speed: 400
            },
            now
        )
        .is_err()
    );
    assert!(
        app.dispatch(Command::SetSpeedAll { speed: 2500 }, now)
            .is_err()
    );
}

// covers: REQ-280, REQ-282
#[test]
fn volume_is_an_effect_with_keyframes_one_per_clip_and_remove_takes_the_keys_away() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let now = Instant::now();
    app.dispatch(
        Command::SetClipVolume {
            clip: c[0],
            volume: 40,
            muted: false,
        },
        now,
    )
    .unwrap();
    let apply = Command::ApplyEffect {
        effect: EffectKind::Volume,
        target: Target::Clip(c[0]),
    };
    app.dispatch(apply.clone(), now).unwrap();
    // one keyframe at the level of the slider
    let keys = app.project.timeline.volume_keys(c[0]).to_vec();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].v, 40);
    // applied again: still one effect
    app.dispatch(apply, now).unwrap();
    assert_eq!(app.project.timeline.volume_keys(c[0]).len(), 1);
    // add a key, ease it, change it
    app.dispatch(
        Command::SetVolumeKey {
            clip: c[0],
            at: s(2),
            percent: 100,
        },
        now,
    )
    .unwrap();
    app.dispatch(
        Command::SetVolumeKeyEase {
            clip: c[0],
            at: Micros::ZERO,
            ease: crate::keyframes::Ease::EaseInOut,
        },
        now,
    )
    .unwrap();
    let keys = app.project.timeline.volume_keys(c[0]);
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].ease, crate::keyframes::Ease::EaseInOut);
    assert_eq!(app.undo_text(), "Undo Volume keyframe ease");
    // the other clips have none
    assert!(app.project.timeline.volume_keys(c[1]).is_empty());
    // Remove effect: the keys go, the slider level stays
    app.dispatch(Command::RemoveVolume { clip: c[0] }, now)
        .unwrap();
    assert!(app.project.timeline.volume_keys(c[0]).is_empty());
    assert_eq!(app.project.timeline.clips[0].volume, 40);
    assert!(!app.is_enabled(&Command::RemoveVolume { clip: c[0] }));
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.timeline.volume_keys(c[0]).len(), 2);
}

// covers: REQ-332, REQ-330
#[test]
fn opening_a_project_of_format_13_says_what_the_conversion_did_once_with_no_change_mark() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = state(FakeDialogs::default());
    let c = three(&mut app, dir.path());
    let now = Instant::now();
    // a speed and volume keys are kept as they are; a picture item under clip 1 and a crop
    app.dispatch(
        Command::SetSpeed {
            clip: c[0],
            speed: 1500,
        },
        now,
    )
    .unwrap();
    app.dispatch(
        Command::SetVolumeKey {
            clip: c[0],
            at: Micros::ZERO,
            percent: 20,
        },
        now,
    )
    .unwrap();
    let pic = png(dir.path(), "logo.png", 400, 200);
    app.dispatch(Command::Seek(s(1)), now).unwrap();
    app.dispatch(Command::AddPicture(Some(pic)), now).unwrap();
    let file = dir.path().join("p.vlproj");
    app.dispatch(Command::SaveProjectAs(file.clone()), now)
        .unwrap();
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    v["schema_version"] = 13.into();
    v["timeline"]["clips"][0]["crop"] =
        serde_json::json!({"left": 10, "top": 0, "right": 0, "bottom": 0});
    std::fs::write(&file, v.to_string()).unwrap();
    let mut open = state(FakeDialogs::default());
    open.dispatch(Command::OpenProject(file.clone()), now)
        .unwrap();
    settle_projects(&mut open);
    assert_eq!(
        open.status,
        "Opening: 1 clip(s) changed by the conversion (1 crop(s) dropped, 1 picture(s) made overlaps)"
    );
    // kept as stored, no crop, the picture an overlap of clip 1; nothing to undo or save
    let first = open.project.timeline.clips[0];
    assert_eq!(first.speed, 1500);
    assert!(first.crop.is_none());
    assert_eq!(open.project.timeline.volume_keys(first.id).len(), 1);
    assert_eq!(open.project.timeline.overlaps_of(first.id).len(), 1);
    assert!(open.project.texts.items.iter().all(|t| t.picture.is_none()));
    assert!(!open.dirty);
    assert_eq!(open.history.undo_len(), 0);
    // saved again it is format 14 and opens with the usual line
    open.dispatch(Command::SaveProject, now).unwrap();
    let mut again = state(FakeDialogs::default());
    again.dispatch(Command::OpenProject(file), now).unwrap();
    assert_eq!(again.status, "Project opened");
}

fn settle_projects(app: &mut AppState) {
    super::tests::settle(app);
}
