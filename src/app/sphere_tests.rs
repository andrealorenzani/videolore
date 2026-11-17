// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of 1.2.6 in the app: the live 360 camera, points of view, the "Treat as 360°
//! video" switch, and the boxes' resize, align and reset commands.

use super::tests::{FakeDialogs, files, import, runner, settle, state, state_with};
use super::*;
use crate::camera::{Camera, Pov};
use crate::ffmpeg::fake::ok;
use crate::subtitles::{NewLine, SIZE_MAX, SIZE_MIN};

fn s(n: i64) -> Micros {
    Micros(n * 1_000_000)
}

/// Clip 1 is a 360 source (a.mp4, 5 s), clip 2 a normal one (b.mp4, 5 s).
fn sphere_app(dir: &Path) -> AppState {
    let mut app = state(FakeDialogs::default());
    import(&mut app, files(dir));
    let now = Instant::now();
    let (a, b) = (app.project.media[0].id, app.project.media[1].id);
    app.project.media[0].format.spherical = Some(true);
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    app.dispatch(Command::Append(Some(b)), now).unwrap();
    app.saved = app.project.doc();
    app.dirty = false;
    app
}

fn a(app: &AppState) -> MediaId {
    app.project.media[0].id
}

// covers: REQ-101
#[test]
fn dragging_and_zooming_turn_the_live_camera_without_touching_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let m = a(&app);
    assert_eq!(app.sphere_at_playhead().map(|x| x.0), Some(m));
    assert_eq!(app.camera_at_playhead(), Some(Camera::FRONT));
    let undo = app.history.undo_len();
    // 100 px on a 900 px wide view with 90 degrees: 10 degrees, the picture follows
    app.dispatch(
        Command::DragCamera {
            dx: 100.0,
            dy: 0.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.camera_at_playhead().unwrap().yaw_cd, -1000);
    // the same drag again from where it is
    app.dispatch(
        Command::DragCamera {
            dx: 0.0,
            dy: 100.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.camera_at_playhead().unwrap().pitch_cd, 1000);
    app.dispatch(Command::ZoomCamera(200.0), now).unwrap();
    assert!(app.camera_at_playhead().unwrap().fov_cd < 9000);
    // nothing saved, nothing to undo, the project is not changed
    assert_eq!(app.history.undo_len(), undo);
    assert!(app.project.camera.is_empty());
    app.refresh_dirty();
    assert!(!app.dirty);
    // the preview shows it as one fixed view, the project keeps no POV
    let shown = app.shown_project();
    assert_eq!(shown.camera.path(m).len(), 1);
    assert_eq!(shown.camera.path(m)[0].cam.yaw_cd, -1000);
    drop(shown);
    // a still camera is the same value: nothing happens again
    app.dispatch(Command::ZoomCamera(0.0), now).unwrap();
    // wheel on a normal clip: not available
    app.seek(s(6), now);
    assert!(app.sphere_at_playhead().is_none());
    assert!(!app.is_enabled(&Command::ZoomCamera(1.0)));
    assert_eq!(
        app.disabled_reason(&Command::ZoomCamera(1.0)),
        Some(sphere::NOT_A_360)
    );
    assert!(app.dispatch(Command::ZoomCamera(1.0), now).is_err());
    assert!(
        app.dispatch(
            Command::DragCamera {
                dx: 1.0,
                dy: 1.0,
                view_w: 1.0
            },
            now
        )
        .is_err()
    );
    assert!(app.camera_at_playhead().is_none());
}

// covers: REQ-102
#[test]
fn set_pov_stores_the_shown_view_and_is_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let m = a(&app);
    app.dispatch(
        Command::DragCamera {
            dx: -450.0,
            dy: 0.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    app.seek(s(1), now);
    assert!(app.is_enabled(&Command::SetPov));
    app.dispatch(Command::SetPov, now).unwrap();
    let p = app.project.camera.path(m);
    assert_eq!(p.len(), 1);
    assert_eq!((p[0].t, p[0].cam.yaw_cd), (s(1), 4500));
    assert_eq!(app.undo_text(), "Undo Set POV");
    assert!(app.dirty);
    // the same view at the same time again: nothing new
    let n = app.history.undo_len();
    app.dispatch(Command::SetPov, now).unwrap();
    assert_eq!(app.history.undo_len(), n);
    // another time, another view
    app.seek(s(3), now);
    app.dispatch(
        Command::DragCamera {
            dx: 0.0,
            dy: 450.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::SetPov, now).unwrap();
    assert_eq!(app.project.camera.path(m).len(), 2);
    // at 2 s the path is half way (from the first view to the second)
    app.seek(s(2), now);
    let c = app.camera_at_playhead().unwrap();
    assert_eq!((c.yaw_cd, c.pitch_cd), (4500, 2250));
    // undo removes the last POV
    app.dispatch(Command::Undo, now).unwrap();
    assert_eq!(app.project.camera.path(m).len(), 1);
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.camera.is_empty());
    app.dispatch(Command::Redo, now).unwrap();
    assert_eq!(app.project.camera.path(m).len(), 1);
    // Set POV on a normal clip: greyed with the reason
    app.seek(s(7), now);
    assert!(!app.is_enabled(&Command::SetPov));
    assert_eq!(
        app.disabled_reason(&Command::SetPov),
        Some(sphere::NOT_A_360)
    );
    assert!(app.dispatch(Command::SetPov, now).is_err());
}

// covers: REQ-101, REQ-102
#[test]
fn a_drag_overrides_the_path_until_seek_play_or_reset() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let m = a(&app);
    let pov = |t: i64, yaw: i32| Pov {
        t: s(t),
        cam: Camera::new(i64::from(yaw) * 100, 0, 9000),
    };
    app.project.camera.set(m, pov(0, 0)).unwrap();
    app.project.camera.set(m, pov(4, 80)).unwrap();
    app.seek(s(2), now);
    assert_eq!(app.camera_at_playhead().unwrap().yaw_cd, 4000);
    app.dispatch(
        Command::DragCamera {
            dx: -90.0,
            dy: 0.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    assert!(app.live[&m].overriding);
    assert_eq!(app.camera_at_playhead().unwrap().yaw_cd, 4900);
    // moving through the path: the dragged view stays, until a seek
    assert_eq!(app.effective_camera(m, s(3)).yaw_cd, 4900);
    assert!(app.is_enabled(&Command::ResetObjects));
    app.seek(s(2), now);
    assert!(!app.live[&m].overriding);
    assert_eq!(app.camera_at_playhead().unwrap().yaw_cd, 4000);
    // Reset gives the path back and keeps the POVs
    app.dispatch(
        Command::DragCamera {
            dx: -90.0,
            dy: 0.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    assert!(app.is_enabled(&Command::ResetObjects));
    app.dispatch(Command::ResetObjects, now).unwrap();
    assert_eq!(app.camera_at_playhead().unwrap().yaw_cd, 4000);
    assert_eq!(app.project.camera.path(m).len(), 2);
    assert!(!app.camera_is_moved() || app.live[&m].cam != Camera::FRONT);
}

// covers: REQ-099
#[test]
fn reset_on_a_360_clip_returns_the_live_view_to_the_front() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let undo = app.history.undo_len();
    // nothing moved: greyed with a reason
    assert!(!app.is_enabled(&Command::ResetObjects));
    assert_eq!(
        app.disabled_reason(&Command::ResetObjects),
        Some(sphere::CAMERA_AT_FRONT)
    );
    assert!(app.dispatch(Command::ResetObjects, now).is_err());
    app.dispatch(
        Command::DragCamera {
            dx: 100.0,
            dy: 100.0,
            view_w: 900.0,
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ZoomCamera(300.0), now).unwrap();
    assert!(app.camera_is_moved());
    app.dispatch(Command::ResetObjects, now).unwrap();
    assert_eq!(app.camera_at_playhead(), Some(Camera::FRONT));
    assert!(!app.is_enabled(&Command::ResetObjects));
    assert_eq!(app.history.undo_len(), undo, "the live view has no undo");
    // on a normal clip with nothing selected: select something first
    app.seek(s(7), now);
    assert_eq!(
        app.disabled_reason(&Command::ResetObjects),
        Some(sphere::SELECT_ON_PICTURE)
    );
    assert!(app.dispatch(Command::ResetObjects, now).is_err());
}

// covers: REQ-102
#[test]
fn povs_can_be_edited_removed_and_are_limited() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let m = a(&app);
    app.seek(s(1), now);
    app.dispatch(Command::SetPov, now).unwrap();
    let edit = Pov {
        t: s(2),
        cam: Camera::new(12_000, 500, 5000),
    };
    app.dispatch(
        Command::EditPov {
            media: m,
            old_t: s(1),
            pov: edit,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.camera.path(m), [edit]);
    assert_eq!(app.undo_text(), "Undo Edit POV");
    // no change: no undo step
    let n = app.history.undo_len();
    app.dispatch(
        Command::EditPov {
            media: m,
            old_t: s(2),
            pov: edit,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n);
    // values out of range are brought into range; a time outside the video is refused
    let wild = Pov {
        t: s(2),
        cam: Camera {
            yaw_cd: 99_999,
            pitch_cd: 99_999,
            fov_cd: 1,
        },
    };
    app.dispatch(
        Command::EditPov {
            media: m,
            old_t: s(2),
            pov: wild,
        },
        now,
    )
    .unwrap();
    assert!(app.project.camera.path(m)[0].cam.is_in_range());
    for t in [-1, 6_000_000] {
        let bad = Pov {
            t: Micros(t),
            cam: Camera::FRONT,
        };
        assert!(
            app.dispatch(
                Command::EditPov {
                    media: m,
                    old_t: s(2),
                    pov: bad
                },
                now
            )
            .is_err()
        );
    }
    assert!(
        app.dispatch(
            Command::EditPov {
                media: MediaId(99),
                old_t: s(2),
                pov: edit
            },
            now
        )
        .is_err()
    );
    // remove
    assert!(
        app.dispatch(Command::RemovePov { media: m, t: s(4) }, now)
            .is_err()
    );
    app.dispatch(Command::RemovePov { media: m, t: s(2) }, now)
        .unwrap();
    assert!(app.project.camera.is_empty());
    assert_eq!(app.undo_text(), "Undo Remove POV");
    // 200 at most
    for i in 0..200 {
        app.project
            .camera
            .set(
                m,
                Pov {
                    t: Micros(i * 10_000),
                    cam: Camera::FRONT,
                },
            )
            .unwrap();
    }
    app.seek(Micros(3_000_000), now);
    let e = app.dispatch(Command::SetPov, now).unwrap_err();
    assert!(e.contains("at most 200"), "{e}");
}

// covers: REQ-100, REQ-123
#[test]
fn only_the_file_makes_a_clip_360_and_an_old_switch_is_dropped_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let app = sphere_app(dir.path());
    let m = a(&app);
    let b = app.project.media[1].id;
    assert!(app.project.media(m).unwrap().format.is_360());
    assert!(!app.project.media(b).unwrap().format.is_360());
    // saved by 1.4.0: no switch is written
    let file = dir.path().join("p.vlproj");
    crate::project::save(&app.project, &file).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains("treat_as_360"));
    // a project of 1.3.0 with the switch on a flat file and off a 360 file: both ignored
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["schema_version"] = 5.into();
    v["media"][0]["format"]["treat_as_360"] = false.into();
    v["media"][1]["format"]["treat_as_360"] = true.into();
    let old = dir.path().join("old.vlproj");
    std::fs::write(&old, serde_json::to_string(&v).unwrap()).unwrap();
    let (q, _) = crate::project::load(&old).unwrap();
    assert!(q.media[0].format.is_360(), "the file says 360");
    assert!(!q.media[1].format.is_360(), "the file says flat");
}

// covers: REQ-100
#[test]
fn removing_a_source_drops_its_points_of_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let m = a(&app);
    app.dispatch(Command::SetPov, now).unwrap();
    app.dispatch(
        Command::DragCamera {
            dx: 1.0,
            dy: 1.0,
            view_w: 100.0,
        },
        now,
    )
    .unwrap();
    assert!(app.live.contains_key(&m));
    app.dispatch(Command::SelectLine(None), now).unwrap();
    // the clip is on the timeline: first remove it
    let c = app.project.timeline.clips[0].id;
    app.dispatch(Command::Delete, now).unwrap();
    let _ = c;
    app.history.clear();
    app.dispatch(Command::RemoveMedia(m), now).unwrap();
    assert!(app.project.camera.is_empty());
    assert!(!app.live.contains_key(&m));
}

/// Two lines (a at 0–4 s, b at 0–4 s) on the 360 clip's source and the first selected.
fn with_lines(app: &mut AppState) -> (LineId, LineId) {
    let m = app.project.media[0].id;
    let line = |text: &str| NewLine {
        media: m,
        start: Micros(0),
        end: s(4),
        text: text.into(),
        words: vec![],
    };
    app.project
        .subtitles
        .replace_all(vec![line("one"), line("two")], None);
    let ids: Vec<LineId> = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    (ids[0], ids[1])
}

// covers: REQ-098
#[test]
fn align_centres_the_selected_lines_on_one_axis_in_one_undo_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let (l1, l2) = with_lines(&mut app);
    // nothing selected: greyed out with the reason
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let c = Command::AlignSelected(axis);
        assert!(!app.is_enabled(&c));
        assert_eq!(app.disabled_reason(&c), Some(sphere::SELECT_ON_PICTURE));
        assert!(app.dispatch(c, now).is_err());
    }
    let low = Pos { x: 1000, y: 8000 };
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l1, l2],
            pos: Some(low),
        },
        now,
    )
    .unwrap();
    app.dispatch(Command::ToggleLine(l1), now).unwrap();
    app.dispatch(Command::ToggleLine(l2), now).unwrap();
    let n = app.history.undo_len();
    app.dispatch(Command::AlignSelected(Axis::Horizontal), now)
        .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    for l in &app.project.subtitles.lines {
        assert_eq!(l.position, Some(Pos { x: 5000, y: 8000 }));
    }
    assert_eq!(app.undo_text(), "Undo Align horizontally");
    // aligned already: no new undo step
    app.dispatch(Command::AlignSelected(Axis::Horizontal), now)
        .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    app.dispatch(Command::AlignSelected(Axis::Vertical), now)
        .unwrap();
    for l in &app.project.subtitles.lines {
        assert_eq!(l.position, Some(Pos { x: 5000, y: 5000 }));
    }
    assert_eq!(app.undo_text(), "Undo Align vertically");
    app.dispatch(Command::Undo, now).unwrap();
    app.dispatch(Command::Undo, now).unwrap();
    for l in &app.project.subtitles.lines {
        assert_eq!(l.position, Some(low));
    }
    // a line at the style's place is already centred horizontally; vertically it gets
    // the middle of the picture
    app.dispatch(Command::ResetObjects, now).unwrap();
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none())
    );
    let n = app.history.undo_len();
    app.dispatch(Command::AlignSelected(Axis::Horizontal), now)
        .unwrap();
    assert_eq!(app.history.undo_len(), n, "nothing to change");
    app.dispatch(Command::AlignSelected(Axis::Vertical), now)
        .unwrap();
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position == Some(Pos { x: 5000, y: 5000 }))
    );
}

// covers: REQ-097
#[test]
fn resizing_gives_every_selected_line_the_size_and_the_dragged_one_its_centre() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let (l1, l2) = with_lines(&mut app);
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![l2],
            pos: Some(Pos { x: 7000, y: 3000 }),
        },
        now,
    )
    .unwrap();
    let n = app.history.undo_len();
    let drag = ResizeDrag {
        ids: vec![l1, l2],
        line: l1,
        pos: Pos { x: 4000, y: 9000 },
        size: 2000,
    };
    app.dispatch(Command::DragResize(Some(drag.clone())), now)
        .unwrap();
    // shown at once, not saved
    let shown = app.shown_project();
    assert!(shown.subtitles.lines.iter().all(|l| l.size == Some(2000)));
    assert_eq!(shown.subtitles.get(l1).unwrap().position, Some(drag.pos));
    assert_eq!(
        shown.subtitles.get(l2).unwrap().position,
        Some(Pos { x: 7000, y: 3000 })
    );
    drop(shown);
    assert!(app.project.subtitles.lines.iter().all(|l| l.size.is_none()));
    app.dispatch(
        Command::ResizeLines {
            ids: vec![l1, l2],
            line: l1,
            size: 2000,
            pos: drag.pos,
        },
        now,
    )
    .unwrap();
    assert!(app.resize_drag.is_none());
    assert_eq!(app.history.undo_len(), n + 1, "one drag is one undo step");
    let subs = &app.project.subtitles;
    assert_eq!(subs.get(l1).unwrap().size, Some(2000));
    assert_eq!(subs.get(l2).unwrap().size, Some(2000));
    assert_eq!(subs.get(l1).unwrap().position, Some(drag.pos));
    assert_eq!(
        subs.get(l2).unwrap().position,
        Some(Pos { x: 7000, y: 3000 }),
        "each keeps its own position"
    );
    assert_eq!(app.undo_text(), "Undo Resize subtitle");
    // the same again: nothing new
    app.dispatch(
        Command::ResizeLines {
            ids: vec![l1, l2],
            line: l1,
            size: 2000,
            pos: drag.pos,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.history.undo_len(), n + 1);
    // undo restores size and position in one step
    app.dispatch(Command::Undo, now).unwrap();
    assert!(app.project.subtitles.lines.iter().all(|l| l.size.is_none()));
    assert_eq!(app.project.subtitles.get(l1).unwrap().position, None);
    app.dispatch(Command::Redo, now).unwrap();
    // 100% is stored as no size; sizes are limited; hostile arguments are refused
    app.dispatch(
        Command::ResizeLines {
            ids: vec![l1],
            line: l1,
            size: 1000,
            pos: drag.pos,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap().size, None);
    app.dispatch(
        Command::ResizeLines {
            ids: vec![l1],
            line: l1,
            size: 60_000,
            pos: drag.pos,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap().size, Some(SIZE_MAX));
    app.dispatch(
        Command::ResizeLines {
            ids: vec![l1],
            line: l1,
            size: 1,
            pos: drag.pos,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.subtitles.get(l1).unwrap().size, Some(SIZE_MIN));
    for bad in [
        Command::ResizeLines {
            ids: vec![],
            line: l1,
            size: 2000,
            pos: drag.pos,
        },
        Command::ResizeLines {
            ids: vec![l2],
            line: l1,
            size: 2000,
            pos: drag.pos,
        },
        Command::ResizeLines {
            ids: vec![l1],
            line: l1,
            size: 2000,
            pos: Pos { x: 20_000, y: 0 },
        },
    ] {
        assert!(app.dispatch(bad, now).is_err());
    }
    // the drag feedback is limited too
    app.dispatch(
        Command::DragResize(Some(ResizeDrag {
            size: 1,
            pos: Pos { x: 50_000, y: 1 },
            ..drag
        })),
        now,
    )
    .unwrap();
    let r = app.resize_drag.clone().unwrap();
    assert_eq!((r.size, r.pos.x), (SIZE_MIN, 10_000));
    app.dispatch(Command::DragResize(None), now).unwrap();
    assert!(app.resize_drag.is_none());
}

// covers: REQ-099, REQ-089
#[test]
fn reset_gives_the_style_place_and_100_percent_back_to_every_selected_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let (l1, l2) = with_lines(&mut app);
    let third = {
        let m = app.project.media[0].id;
        app.project.subtitles.replace_all(
            app.project
                .subtitles
                .lines
                .iter()
                .map(|l| NewLine {
                    media: m,
                    start: l.start,
                    end: l.end,
                    text: l.text.clone(),
                    words: vec![],
                })
                .chain(std::iter::once(NewLine {
                    media: m,
                    start: Micros(0),
                    end: s(4),
                    text: "three".into(),
                    words: vec![],
                }))
                .collect(),
            None,
        );
        app.project.subtitles.lines[2].id
    };
    let _ = (l1, l2);
    let ids: Vec<LineId> = app.project.subtitles.lines.iter().map(|l| l.id).collect();
    // nothing selected, nothing moved
    assert!(!app.is_enabled(&Command::ResetObjects));
    app.dispatch(
        Command::SetLinePositions {
            ids: ids.clone(),
            pos: Some(Pos { x: 100, y: 200 }),
        },
        now,
    )
    .unwrap();
    app.dispatch(
        Command::ResizeLines {
            ids: ids.clone(),
            line: third,
            size: 3000,
            pos: Pos { x: 100, y: 200 },
        },
        now,
    )
    .unwrap();
    for id in &ids {
        app.dispatch(Command::ToggleLine(*id), now).unwrap();
    }
    assert!(app.is_enabled(&Command::ResetObjects));
    let n = app.history.undo_len();
    app.dispatch(Command::ResetObjects, now).unwrap();
    assert_eq!(app.history.undo_len(), n + 1, "three lines, one step");
    assert!(
        app.project
            .subtitles
            .lines
            .iter()
            .all(|l| l.position.is_none() && l.size.is_none())
    );
    assert_eq!(app.undo_text(), "Undo Reset");
    // everything is original now: greyed with the reason
    assert!(!app.is_enabled(&Command::ResetObjects));
    assert_eq!(
        app.disabled_reason(&Command::ResetObjects),
        Some(sphere::ALREADY_ORIGINAL)
    );
    assert!(!app.is_enabled(&Command::ResetPosition));
    assert_eq!(
        app.disabled_reason(&Command::ResetPosition),
        Some(sphere::ALREADY_ORIGINAL)
    );
    app.dispatch(Command::ResetPosition, now).unwrap();
    assert_eq!(app.history.undo_len(), n + 1, "nothing to do");
    // "Reset position" of 1.2 (one line, by id) also gives the size back
    app.dispatch(
        Command::ResizeLines {
            ids: vec![ids[0]],
            line: ids[0],
            size: 1500,
            pos: Pos { x: 5000, y: 5000 },
        },
        now,
    )
    .unwrap();
    app.dispatch(
        Command::SetLinePositions {
            ids: vec![ids[0]],
            pos: None,
        },
        now,
    )
    .unwrap();
    assert_eq!(app.project.subtitles.get(ids[0]).unwrap().size, None);
}

// covers: REQ-096, REQ-057
#[test]
fn show_boxes_is_a_saved_setting() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = sphere_app(dir.path());
    let now = Instant::now();
    let path = dir.path().join("settings.json");
    app.settings_path = Some(path.clone());
    assert!(app.settings.show_boxes);
    app.dispatch(Command::ToggleBoxes, now).unwrap();
    assert!(!app.settings.show_boxes);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"show_boxes\": false"), "{text}");
    app.dispatch(Command::ToggleBoxes, now).unwrap();
    assert!(app.settings.show_boxes);
    settle(&mut app);
}

// covers: REQ-100, REQ-105
#[test]
fn media_of_older_projects_are_checked_for_spherical_information_without_becoming_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let probe = r#"{"streams":[
        {"codec_type":"video","codec_name":"h264","width":1280,"height":720,"avg_frame_rate":"30/1",
         "side_data_list":[{"side_data_type":"Spherical Mapping","projection":"equirectangular"}]},
        {"codec_type":"audio","codec_name":"aac"}],
        "format":{"format_name":"mov,mp4","duration":"5.0"}}"#;
    let mut r = runner();
    r.responders.insert(
        0,
        Box::new(move |p: &Path, a: &[String]| {
            (p.ends_with("ffprobe") && a.last()?.ends_with("a.mp4")).then(|| ok(probe))?
        }),
    );
    let mut app = state_with(FakeDialogs::default(), r);
    // the 1.2.5 fixture: its media were analysed before the check existed
    let p = crate::render::golden_tests::open_fixture(
        dir.path(),
        crate::render::golden_tests::V3_FIXTURE,
    );
    assert!(p.media.iter().all(|m| m.format.spherical.is_none()));
    let file = dir.path().join("fixture.vlproj");
    let now = Instant::now();
    app.dispatch(Command::OpenProject(file), now).unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(10);
    while app.track_check.is_some() && Instant::now() < deadline {
        app.tick(Instant::now());
    }
    let find = |app: &AppState, n: &str| {
        app.project
            .media
            .iter()
            .find(|m| m.name == n)
            .unwrap()
            .format
            .clone()
    };
    assert_eq!(find(&app, "a.mp4").spherical, Some(true));
    assert!(find(&app, "a.mp4").is_360());
    assert_eq!(find(&app, "b.mp4").spherical, Some(false));
    assert!(!app.dirty, "the check does not make the project unsaved");
    app.refresh_dirty();
    assert!(!app.dirty);
    settle(&mut app);
}
