// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of Apply LUT (1.6.0, REQ-135): the Effects row, the popup, the marker and
//! the icons of the menus, through egui's accessibility tree.

use super::ux_tests::{Harness, Node};
use super::*;
use crate::app::tests::{FakeDialogs, files, import, state};
use crate::app::{Instance, Popup};
use crate::timeline::FrameTarget;

/// Three clips (a, b, a) and a music clip, with a valid LUT the dialog will "choose".
fn setup() -> (tempfile::TempDir, AppState, String) {
    let dir = tempfile::tempdir().unwrap();
    let lut = dir.path().join("warm.cube");
    std::fs::write(&lut, crate::lut::tests::identity(2)).unwrap();
    let mut app = state(FakeDialogs {
        lut: Some(lut.clone()),
        ..Default::default()
    });
    import(&mut app, files(dir.path()));
    let now = Instant::now();
    for i in [0, 1, 0] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::PlaceMusic(Some(app.project.media[2].id)), now)
        .unwrap();
    (dir, app, lut.to_string_lossy().into_owned())
}

fn clip_nodes(h: &Harness) -> Vec<Node> {
    let mut v: Vec<Node> = h
        .nodes
        .iter()
        .filter(|n| {
            n.label
                .contains("Double-click for details, right-click for more")
        })
        .cloned()
        .collect();
    v.sort_by(|a, b| a.rect.min.x.partial_cmp(&b.rect.min.x).unwrap());
    v
}

// covers: REQ-135
// covers: REQ-142
#[test]
fn dragging_apply_lut_on_clip_2_opens_its_popup_and_choosing_a_file_sets_it() {
    let (_d, app, lut) = setup();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let clips = clip_nodes(&h);
    assert_eq!(clips.len(), 3);
    let ids: Vec<_> = h.app.project.timeline.clips.iter().map(|c| c.id).collect();
    let before = h.app.project.edit();
    let row = h.find("Apply LUT").unwrap();
    assert!(
        row.raw.ends_with(" Apply LUT") && row.raw != "Apply LUT",
        "icon on the row"
    );
    // dropped on the music track: nothing happens
    let music = h
        .nodes
        .iter()
        .find(|n| n.label.starts_with("song.mp3\nDrag to move"))
        .cloned()
        .unwrap();
    h.drag(row.rect.center(), music.rect.center());
    assert_eq!(h.app.project.edit(), before);
    assert!(!h.app.popups.is_open(Popup::Instance));
    // dropped on clip 2: the popup of that clip, nothing set until a file is chosen
    h.drag(row.rect.center(), clips[1].rect.center());
    assert_eq!(
        h.app.popups.instance,
        Some(Instance::Lut(FrameTarget::Clip(ids[1])))
    );
    assert_eq!(h.app.project.edit(), before);
    assert!(h.has("No LUT file chosen"), "{:#?}", h.labels());
    assert!(h.has("Choose file…") && h.has("Remove effect"));
    // Remove effect is greyed until a file is chosen
    assert!(h.find("Remove effect").unwrap().disabled);
    h.click_label("Choose file…");
    assert_eq!(
        h.app.project.timeline.lut_of(FrameTarget::Clip(ids[1])),
        Some(lut.as_str())
    );
    assert_eq!(
        h.app.project.timeline.lut_of(FrameTarget::Clip(ids[0])),
        None
    );
    assert_eq!(h.app.undo_text(), "Undo Apply LUT");
    h.idle(2);
    assert!(
        h.has("warm.cube"),
        "the file name is shown: {:#?}",
        h.labels()
    );
    // the clip has a marker at its start that opens the popup
    let markers = crate::ui::markers::layout(&h.app.project, 40.0, 0.0, (0.0, 10_000.0));
    let m = markers
        .iter()
        .find(|m| m.kind == crate::ui::markers::MarkerKind::Lut)
        .expect("a marker");
    assert_eq!(m.instance, Instance::Lut(FrameTarget::Clip(ids[1])));
    assert_eq!(
        crate::ui::markers::click_command_with(m, false),
        Command::OpenInstance(Instance::Lut(FrameTarget::Clip(ids[1])))
    );
    // a second Apply LUT on the same clip opens the existing popup (nothing new to undo)
    h.app.popups.close(Popup::Instance);
    let undo_len = h.app.history.undo_len();
    h.idle(2);
    let row = h.find("Apply LUT").unwrap();
    let clips = clip_nodes(&h);
    h.drag(row.rect.center(), clips[1].rect.center());
    assert_eq!(
        h.app.popups.instance,
        Some(Instance::Lut(FrameTarget::Clip(ids[1])))
    );
    assert_eq!(h.app.history.undo_len(), undo_len);
    // Remove effect gives the source colours back; one undo restores the LUT
    h.click_label("Remove effect");
    assert!(!h.app.project.timeline.any_lut());
    assert_eq!(h.app.undo_text(), "Undo Remove LUT");
    h.app.dispatch(Command::Undo, Instant::now()).unwrap();
    assert!(h.app.project.timeline.any_lut());
}

// covers: REQ-135
#[test]
fn a_double_click_applies_to_the_whole_video_and_has_no_marker() {
    let (_d, app, lut) = setup();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let row = h.find("Apply LUT").unwrap();
    h.double_click(row.rect.center());
    assert_eq!(
        h.app.popups.instance,
        Some(Instance::Lut(FrameTarget::Whole))
    );
    assert!(h.app.popups.is_open(Popup::Instance));
    h.click_label("Choose file…");
    assert_eq!(
        h.app.project.timeline.lut_of(FrameTarget::Whole),
        Some(lut.as_str())
    );
    assert!(
        crate::ui::markers::layout(&h.app.project, 40.0, 0.0, (0.0, 10_000.0))
            .iter()
            .all(|m| m.kind != crate::ui::markers::MarkerKind::Lut)
    );
    // the popup title says what it applies to, with an icon
    assert!(h.app.popup_title(Popup::Instance).contains("whole video"));
}

// covers: REQ-069
// covers: REQ-075
// covers: REQ-135
#[test]
fn the_effects_menu_and_the_clip_menu_offer_apply_lut() {
    use crate::ui::actions::ActionId;
    let (_d, mut app, _) = setup();
    assert!(crate::ui::menu::menus().iter().any(|(t, items)| {
        *t == "Effects" && items.contains(&crate::ui::menu::Item::Action(ActionId::ApplyLut))
    }));
    assert!(crate::ui::menu::APPLY_TO_CLIP.contains(&ActionId::ApplyClipLut));
    assert!(crate::ui::menu::APPLY_TO_VIDEO.contains(&ActionId::ApplyWholeLut));
    // the clip's right-click menu lists every effect except Change aspect
    let ids: Vec<_> = app.project.timeline.clips.iter().map(|c| c.id).collect();
    app.dispatch(
        Command::ApplyEffect {
            effect: crate::app::EffectKind::ApplyLut,
            target: crate::app::Target::Clip(ids[0]),
        },
        Instant::now(),
    )
    .unwrap();
    assert_eq!(
        app.popups.instance,
        Some(Instance::Lut(FrameTarget::Clip(ids[0])))
    );
    assert_eq!(app.popup_title(Popup::Instance), "Apply LUT — a.mp4");
    // on a project without clips the actions are greyed with a reason
    let empty = state(FakeDialogs::default());
    let cmd = ActionId::ApplyWholeLut.command(&empty);
    assert!(empty.disabled_reason(&cmd).is_some());
}

// covers: REQ-075
#[test]
fn change_aspect_cannot_be_dropped_on_a_clip_and_has_its_own_tooltip() {
    let (_d, app, _) = setup();
    let mut h = Harness::new(app, 1280.0, 800.0);
    let clips = clip_nodes(&h);
    let before = h.app.project.edit();
    let row = h.find("Change aspect").unwrap();
    h.drag(row.rect.center(), clips[1].rect.center());
    assert_eq!(h.app.project.edit(), before);
    assert!(!h.app.popups.is_open(Popup::Instance));
    assert_eq!(
        crate::ui::effects::ASPECT_TOOLTIP,
        "Sets the frame of the whole video"
    );
    assert_ne!(
        crate::ui::effects::ASPECT_TOOLTIP,
        crate::ui::effects::EFFECT_TOOLTIP
    );
}
